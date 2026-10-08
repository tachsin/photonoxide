//! The catalogue of benchmark problems: families of problems, each at many sizes, each saying
//! what it asks of the linear algebra and how its answer is checked.
//!
//! An [`Entry`] is a record, cheap to list: its id, its family and size, its grid, its unknowns
//! and nonzeros, its [`Task`], the memory it is expected to take (so that a runner can skip
//! what won't fit), the [`Tier`] it belongs to and what its accuracy is checked against.
//! [`Entry::run`] builds the problem, solves it by the direct solver of a
//! [`crate::backend::Choice`] where it has a direct solve, and returns a
//! [`super::Measurement`] whose accuracy is the check's error.
//!
//! The families:
//!
//! - **2D FDFD** ([`Family::Fdfd2d`]): a slab, a bend, an MMI and a ring on square grids of
//!   10⁴ to 2 × 10⁶ cells of 20 nm and on long ones, E along z and H along z, with PMLs all
//!   round (the complex symmetric similarity, L D Lᵀ) or a Bloch-periodic x (LU only).
//! - **3D FDFD** ([`Family::Fdfd3d`]): the strip in cubes of 16³ cells of 40 nm upward, and a
//!   strip, a bend, an MMI and a directional coupler in boxes shaped like devices, with plain
//!   and stretched PMLs, and a grating with a Bloch-periodic x.
//! - **3D iterative** ([`Family::Iterative3d`]): the guide, Diel and the strip with ports by
//!   QMR and by GMRES with multigrid, as `photonoxide bench` has run them.
//! - **Modes** ([`Family::Modes`]): a strip's full-vector cross-section from 10⁴ to 3 × 10⁵
//!   unknowns, one mode and eight.
//! - **Circuits** ([`Family::Circuit`]): a Mach–Zehnder's netlist over a sweep, a system so
//!   small that overhead is all there is to time.
//! - **Dense kernels** ([`Family::Dense`]): the LU and the product of complex n × n matrices,
//!   n from 32 to 4096.
//! - **FDTD** ([`Family::Fdtd`]): the kernel stepping a box of vacuum and a silicon guide, in f64
//!   and f32, as `photonoxide bench` runs them: memory traffic, no system to solve.
//!
//! The systems of [`super::export`] and the problems of [`super::problems`] are entries too,
//! under the ids they have always had.
//!
//! Every direct problem's right-hand side is fixed, the same on every machine, and its check
//! is the residual of one unrefined solve, ‖b − A x‖ / ‖b‖: what a backend's factors are worth
//! as they are. No number here is a device's performance: the 2D problems are 2D, and the
//! grids are chosen for their size, not for convergence.

use std::sync::Arc;
use std::time::Instant;

use faer::sparse::Triplet;
use num_complex::Complex64 as c64;

use super::{Accuracy, Measurement, Phase};
use crate::backend::Choice;
use crate::fdfd::direct::{Direct, Plan};
use crate::fdfd::{Boundaries, Boundaries3d, Edges, Grid, Grid3d, Polarization, Solver3d};
use crate::units::Wavelength;
use crate::{Error, Result};

/// A family of problems.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Family {
    /// 2D FDFD systems, solved directly.
    Fdfd2d,
    /// 3D FDFD systems, solved directly.
    Fdfd3d,
    /// 3D FDFD problems solved by QMR or GMRES.
    Iterative3d,
    /// The mode solvers' eigenproblems.
    Modes,
    /// Circuits: tiny dense systems.
    Circuit,
    /// Dense kernels.
    Dense,
    /// A built-in job, run whole.
    Job,
    /// FDTD's kernel stepping a box: no linear algebra, only memory.
    Fdtd,
}

impl Family {
    /// Every family, in the catalogue's order (a slice, so a new family isn't a new type).
    pub const ALL: &'static [Family] = &[
        Family::Fdfd2d,
        Family::Fdfd3d,
        Family::Iterative3d,
        Family::Modes,
        Family::Circuit,
        Family::Dense,
        Family::Job,
        Family::Fdtd,
    ];

    /// Its name, as an entry's id begins.
    pub fn name(self) -> &'static str {
        match self {
            Family::Fdfd2d => "fdfd2d",
            Family::Fdfd3d => "fdfd3d",
            Family::Iterative3d => "fdfd3d-iterative",
            Family::Modes => "modes",
            Family::Circuit => "circuit",
            Family::Dense => "dense",
            Family::Job => "job",
            Family::Fdtd => "fdtd",
        }
    }
}

/// What a problem asks of the linear algebra.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Task {
    /// A sparse direct solve of a general matrix: an LU.
    DirectGeneral,
    /// A sparse direct solve of a matrix with a complex symmetric similarity: an L D Lᵀ where
    /// the backend takes one, an LU otherwise.
    DirectSymmetric,
    /// An iterative solve to this relative residual.
    Iterative {
        /// ‖b − A x‖ / ‖b‖ to reach.
        tolerance: f64,
    },
    /// The eigenpairs nearest a shift.
    Eigen {
        /// How many.
        modes: usize,
    },
    /// A dense kernel.
    Dense,
    /// Several of those, as a whole run does them.
    Mixed,
    /// Steps of an explicit time-domain scheme: no system to solve.
    TimeSteps {
        /// How many.
        steps: usize,
    },
}

/// How long a set of problems takes: each tier holds the one before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// About a minute in all: the smallest of each family.
    Quick,
    /// Tens of minutes.
    Standard,
    /// Hours, up to the machine's memory.
    Full,
}

impl Tier {
    /// Every tier, the shortest first.
    pub const ALL: [Tier; 3] = [Tier::Quick, Tier::Standard, Tier::Full];

    /// Its name: `quick`, `standard` or `full`.
    pub fn name(self) -> &'static str {
        match self {
            Tier::Quick => "quick",
            Tier::Standard => "standard",
            Tier::Full => "full",
        }
    }

    /// The tier of a name.
    pub fn parse(name: &str) -> Option<Tier> {
        Tier::ALL.into_iter().find(|t| t.name() == name)
    }
}

/// One problem's record.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Entry {
    /// Its id, stable: `fdfd2d/ring-ez-400`, `strip-24`, `fdfd3d/guide-ilu`.
    pub id: String,
    /// Its family.
    pub family: Family,
    /// What is solved, in a sentence.
    pub title: String,
    /// The size its family is scaled by: cells along the shorter side of a grid, a matrix's
    /// order, a sweep's points.
    pub size: usize,
    /// The grid, e.g. `"400 × 400 cells of 20 nm, PMLs of 20, E along z"`: no number without
    /// its grid.
    pub grid: String,
    /// The unknowns of the system solved.
    pub unknowns: usize,
    /// The matrix's nonzeros: counted for the 2D systems, the stencil's 13 an unknown for the
    /// 3D ones (walls and PMLs take a few away), n² for a dense matrix.
    pub nonzeros: usize,
    /// What it asks of the linear algebra.
    pub task: Task,
    /// The memory it is expected to take, in bytes: the factors and the matrix of a direct
    /// solve by photonoxide's own solver, the matrix and the vectors of an iterative one.
    pub memory_bytes: u64,
    /// The shortest tier it is part of.
    pub tier: Tier,
    /// What its accuracy is checked against.
    pub check: String,
    kind: Kind,
}

/// A 2D structure, scaled with its window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flat {
    Slab,
    Bend,
    Mmi,
    Ring,
}

/// A 3D structure, scaled with its box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Solid {
    Strip,
    Bend,
    Mmi,
    Coupler,
    Grating,
}

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Flat {
        shape: Flat,
        polarization: Polarization,
        cells: [usize; 2],
        bloch: bool,
    },
    Solid {
        shape: Solid,
        cells: [usize; 3],
        stretched: bool,
    },
    /// One of [`super::export::IDS`].
    Exported(&'static str),
    /// One of [`super::problems`].
    Fixed(&'static str),
    /// The iterative solvers' guide through a cube of 2 `core` cells inside PMLs of `pml`.
    Guide {
        core: usize,
        pml: usize,
        solve: super::Solve,
    },
    Modes {
        step_nm: usize,
        count: usize,
    },
    Circuit {
        points: usize,
    },
    DenseLu {
        n: usize,
    },
    DenseProduct {
        n: usize,
    },
}

/// The 2D grids' step, µm, and the 3D grids'.
const H2: f64 = 0.02;
const H3: f64 = 0.04;
/// The 2D grids' PMLs, in cells.
const PML2: usize = 20;

/// The 3D grids' PMLs, in cells: 6 as the exported strips', 4 in the smallest boxes.
fn pml3(cells: [usize; 3]) -> usize {
    if cells.into_iter().min().unwrap_or(0) >= 24 {
        6
    } else {
        4
    }
}

/// A matrix's factors' entries by photonoxide's own solver in nested dissection order,
/// estimated: George's n log₂ n on a 2D grid of n unknowns and n^(4/3) on a 3D one, with the
/// constants measured on these problems. L D Lᵀ: 3.0 to 3.2 n log₂ n from 10⁴ to 6 × 10⁵
/// unknowns in 2D, 6.7 to 8.1 n^(4/3) from 1.2 × 10⁴ to 2.5 × 10⁵ in 3D. An LU (a Bloch side)
/// stored 2.1 to 2.2 times that in 2D and 2.5 in 3D. The catalogue's tests hold the estimate
/// within a factor of 2 of the factors a run reports.
fn factor_entries(unknowns: usize, three: bool, symmetric: bool) -> f64 {
    let n = unknowns as f64;
    let (half, lu) = if three {
        (7.5 * n.powf(4.0 / 3.0), 2.5)
    } else {
        (3.1 * n * n.log2(), 2.15)
    };
    if symmetric { half } else { lu * half }
}

/// The bytes of a direct solve: the factors' entries and the matrix's (a value and an index).
fn direct_memory(unknowns: usize, nonzeros: usize, three: bool, symmetric: bool) -> u64 {
    (16.0 * factor_entries(unknowns, three, symmetric) + 24.0 * nonzeros as f64) as u64
}

/// The nonzeros of the 2D operator on nx × ny cells: five a cell, less the neighbours beyond a
/// wall (none along a periodic axis).
fn nonzeros_2d([nx, ny]: [usize; 2], bloch: bool) -> usize {
    5 * nx * ny - 2 * nx - if bloch { 0 } else { 2 * ny }
}

fn tier_of_2d(cells: usize) -> Tier {
    match cells {
        ..=100 => Tier::Quick,
        101..=500 => Tier::Standard,
        _ => Tier::Full,
    }
}

fn tier_of_3d(cells: usize) -> Tier {
    match cells {
        ..=16 => Tier::Quick,
        17..=32 => Tier::Standard,
        _ => Tier::Full,
    }
}

fn flat(shape: Flat, polarization: Polarization, cells: [usize; 2], bloch: bool) -> Entry {
    let (name, what) = match shape {
        Flat::Slab => ("slab", "a 220 nm silicon slab in oxide"),
        Flat::Bend => ("bend", "a 500 nm silicon guide bent through a quarter turn"),
        Flat::Mmi => ("mmi", "a 1 × 2 multimode interferometer"),
        Flat::Ring => ("ring", "a ring resonator beside its bus"),
    };
    let (p, field) = match polarization {
        Polarization::Ez => ("ez", "E along z"),
        Polarization::Hz => ("hz", "H along z"),
    };
    let [nx, ny] = cells;
    let size = nx.min(ny);
    let long = if nx == ny {
        String::new()
    } else {
        format!("-{nx}x{ny}")
    };
    let sides = if bloch { "-bloch" } else { "" };
    let unknowns = nx * ny;
    let nonzeros = nonzeros_2d(cells, bloch);
    Entry {
        id: if long.is_empty() {
            format!("fdfd2d/{name}-{p}{sides}-{size}")
        } else {
            format!("fdfd2d/{name}-{p}{sides}{long}")
        },
        family: Family::Fdfd2d,
        title: format!(
            "2D FDFD, {field}: {what}, {}",
            if bloch {
                "Bloch-periodic along x, PMLs along y: its LU"
            } else {
                "PMLs all round: L D Lᵀ of its complex symmetric similarity"
            }
        ),
        size,
        grid: format!(
            "{nx} × {ny} cells of 20 nm, {}, {field}",
            if bloch {
                format!("Bloch-periodic along x, PMLs of {PML2} along y")
            } else {
                format!("PMLs of {PML2}")
            }
        ),
        unknowns,
        nonzeros,
        task: if bloch {
            Task::DirectGeneral
        } else {
            Task::DirectSymmetric
        },
        memory_bytes: direct_memory(unknowns, nonzeros, false, !bloch),
        tier: tier_of_2d(size),
        check: RESIDUAL.into(),
        kind: Kind::Flat {
            shape,
            polarization,
            cells,
            bloch,
        },
    }
}

const RESIDUAL: &str = "‖b − A x‖ / ‖b‖ of one unrefined solve";

fn solid(shape: Solid, cells: [usize; 3], stretched: bool) -> Entry {
    let (name, what) = match shape {
        Solid::Strip => ("strip", "a 500 × 220 nm silicon strip in oxide"),
        Solid::Bend => ("bend", "the strip bent through a quarter turn"),
        Solid::Mmi => ("mmi", "a multimode slab between the strip's ends"),
        Solid::Coupler => ("coupler", "two strips 200 nm apart"),
        Solid::Grating => ("grating", "the strip with teeth of 320 nm period"),
    };
    let bloch = shape == Solid::Grating;
    let [nx, ny, nz] = cells;
    let size = nx.min(ny).min(nz);
    let pml = pml3(cells);
    let unknowns = 3 * nx * ny * nz;
    let nonzeros = 13 * unknowns;
    let cube = nx == ny && ny == nz;
    Entry {
        id: format!(
            "fdfd3d/{name}{}-{}",
            if stretched { "-stretched" } else { "" },
            if cube {
                size.to_string()
            } else {
                format!("{nx}x{ny}x{nz}")
            }
        ),
        family: Family::Fdfd3d,
        title: format!(
            "3D FDFD, the curl-curl operator: {what}, {}",
            if bloch {
                "Bloch-periodic along x: its LU"
            } else if stretched {
                "stretched PMLs: L D Lᵀ of its complex symmetric similarity"
            } else {
                "PMLs all round: L D Lᵀ of its complex symmetric similarity"
            }
        ),
        size,
        grid: format!(
            "{nx} × {ny} × {nz} cells of 40 nm, {}",
            if bloch {
                format!("Bloch-periodic along x, PMLs of {pml} along y and z")
            } else if stretched {
                format!("stretched PMLs of {pml}")
            } else {
                format!("PMLs of {pml}")
            }
        ),
        unknowns,
        nonzeros,
        task: if bloch {
            Task::DirectGeneral
        } else {
            Task::DirectSymmetric
        },
        memory_bytes: direct_memory(unknowns, nonzeros, true, !bloch),
        tier: tier_of_3d(size),
        check: RESIDUAL.into(),
        kind: Kind::Solid {
            shape,
            cells,
            stretched,
        },
    }
}

fn exported(id: &'static str) -> Entry {
    let (family, cells, grid, unknowns, nonzeros, three): (_, _, String, _, _, _) =
        match id.strip_prefix("strip-").and_then(|c| c.parse().ok()) {
            Some(c) => {
                let n: usize = 3 * c * c * c;
                (
                    Family::Fdfd3d,
                    c,
                    format!("{c} × {c} × {c} cells of 40 nm, PMLs of 6"),
                    n,
                    13 * n,
                    true,
                )
            }
            None => (
                Family::Fdfd2d,
                340,
                "440 × 340 cells of 10 nm, PMLs of 20, E along z".into(),
                440 * 340,
                nonzeros_2d([440, 340], false),
                false,
            ),
        };
    Entry {
        id: id.into(),
        family,
        title: format!(
            "the system `photonoxide bench --export` writes as {id} for PARDISO and MUMPS \
             (docs/baselines.md)"
        ),
        size: cells,
        grid,
        unknowns,
        nonzeros,
        task: Task::DirectSymmetric,
        memory_bytes: direct_memory(unknowns, nonzeros, three, true),
        tier: if three {
            tier_of_3d(cells)
        } else {
            Tier::Standard
        },
        check: RESIDUAL.into(),
        kind: Kind::Exported(id),
    }
}

/// The fixed problems of [`super::problems`], by id: the family, the size, the grid, the
/// unknowns and the task of each, as its one run reports them.
fn fixed(id: &'static str, title: &str, heavy: bool) -> Option<Entry> {
    let guide = "40 × 40 × 40 cells of 10 nm, PMLs of 10";
    let diel = "40 × 90 × 80 cells of 10 nm, PMLs of 10";
    let ports = "72 × 102 × 82 cells of 20 nm, PMLs of 16";
    let (family, size, grid, unknowns, three, task) = match id {
        "fdfd2d/slab-lu" => (
            Family::Fdfd2d,
            340,
            "440 × 340 cells of 10 nm, PMLs of 20",
            440 * 340,
            false,
            Task::DirectSymmetric,
        ),
        "fdfd3d/guide-direct" => (
            Family::Fdfd3d,
            40,
            guide,
            192_000,
            true,
            Task::DirectSymmetric,
        ),
        "fdfd3d/guide-qmr" => (
            Family::Iterative3d,
            40,
            guide,
            192_000,
            true,
            Task::Iterative { tolerance: 1e-6 },
        ),
        "fdfd3d/guide-ilu" | "fdfd3d/guide-multigrid" => (
            Family::Iterative3d,
            40,
            guide,
            192_000,
            true,
            Task::Iterative { tolerance: 1e-8 },
        ),
        "fdfd3d/diel-multigrid" | "fdfd3d/diel-ilu" => (
            Family::Iterative3d,
            40,
            diel,
            864_000,
            true,
            Task::Iterative { tolerance: 1e-8 },
        ),
        "fdfd3d/strip-ports-multigrid" | "fdfd3d/strip-ports-ilu" => (
            Family::Iterative3d,
            72,
            ports,
            1_806_624,
            true,
            Task::Iterative { tolerance: 1e-8 },
        ),
        "job/strip-modes" => (
            Family::Job,
            11,
            "121 × 111 cells of 20 nm, 11 wavelengths",
            27_328,
            false,
            Task::Eigen { modes: 2 },
        ),
        "job/mmi-fdfd" => (
            Family::Job,
            11,
            "725 × 300 cells of 20 nm, 11 wavelengths",
            725 * 300,
            false,
            Task::Mixed,
        ),
        "job/ring-fdfd" => (
            Family::Job,
            201,
            "360 × 256 cells of 25 nm, 201 wavelengths",
            360 * 256,
            false,
            Task::Mixed,
        ),
        "fdtd3d/box-f64" | "fdtd3d/box-f32" => (
            Family::Fdtd,
            48,
            "48 × 48 × 48 cells of 50 nm, CPMLs of 8, vacuum",
            48 * 48 * 48,
            true,
            Task::TimeSteps { steps: 400 },
        ),
        "fdtd3d/guide-f64" | "fdtd3d/guide-f32" => (
            Family::Fdtd,
            160,
            "160 × 160 × 160 cells of 20 nm, CPMLs of 8, a 500 × 220 nm silicon guide",
            160 * 160 * 160,
            true,
            Task::TimeSteps { steps: 60 },
        ),
        _ => return None,
    };
    let nonzeros = match task {
        Task::TimeSteps { .. } => 0,
        _ if three => 13 * unknowns,
        _ => 5 * unknowns,
    };
    let memory_bytes = match task {
        // E, H̃, their coefficients and the CPMLs' ψ, for the run, the kernel alone and its check
        Task::TimeSteps { .. } => (3 * 13 * 8 * unknowns) as u64,
        Task::DirectSymmetric | Task::Mixed => direct_memory(unknowns, nonzeros, three, true),
        // the matrix, and GMRES's 40 vectors or QMR's dozen
        _ => (24 * nonzeros + 16 * 40 * unknowns) as u64,
    };
    Some(Entry {
        id: id.into(),
        family,
        title: title.into(),
        size,
        grid: grid.into(),
        unknowns,
        nonzeros,
        task,
        memory_bytes,
        tier: if heavy {
            Tier::Full
        } else if unknowns > 150_000 || family == Family::Job {
            Tier::Standard
        } else {
            Tier::Quick
        },
        check: if family == Family::Fdtd {
            "f64: the plain loops' bits; f32: f64".into()
        } else {
            "its own: an exact S-matrix or a direct solve's field, where it has one".into()
        },
        kind: Kind::Fixed(id),
    })
}

/// The iterative solvers' guide (a square silicon guide in vacuum, a dipole beside it) through
/// a cube smaller than `photonoxide bench`'s 40³.
fn guide(core: usize, pml: usize, solve: super::Solve) -> Entry {
    let cells = 2 * core + 2 * pml;
    let unknowns = 3 * cells * cells * cells;
    let nonzeros = 13 * unknowns;
    let (name, how, tolerance) = match solve {
        super::Solve::Qmr => ("qmr", "QMR on the curl-curl operator, plain PMLs", 1e-6),
        super::Solve::Ilu => (
            "ilu",
            "QMR + ILU(0) on Shin and Fan's operator, stretched PMLs",
            1e-8,
        ),
        super::Solve::Multigrid => (
            "multigrid",
            "GMRES + multigrid on Shin and Fan's operator, stretched PMLs",
            1e-8,
        ),
    };
    Entry {
        id: format!("fdfd3d-iterative/guide-{name}-{cells}"),
        family: Family::Iterative3d,
        title: format!(
            "3D FDFD by {how}: a {} nm silicon guide through the cube, a dipole beside it, to              a residual of {tolerance:e}",
            core * 10
        ),
        size: cells,
        grid: format!(
            "{cells} × {cells} × {cells} cells of 10 nm, {}PMLs of {pml}",
            if solve == super::Solve::Qmr {
                ""
            } else {
                "stretched "
            }
        ),
        unknowns,
        nonzeros,
        task: Task::Iterative { tolerance },
        // the matrix, and GMRES's 40 vectors or QMR's dozen
        memory_bytes: (24 * nonzeros + 16 * 40 * unknowns) as u64,
        tier: if cells <= 20 {
            Tier::Quick
        } else if cells <= 50 {
            Tier::Standard
        } else {
            Tier::Full
        },
        check: "the direct solver's field".into(),
        kind: Kind::Guide { core, pml, solve },
    }
}

/// The cells of the mode solvers' strip at a step: a 3 × 2 µm window.
fn mode_cells(step_nm: usize) -> [usize; 2] {
    [3000 / step_nm, 2000 / step_nm]
}

fn modes(step_nm: usize, count: usize) -> Entry {
    let [nx, ny] = mode_cells(step_nm);
    // H_x and H_y at each node
    let unknowns = 2 * (nx + 1) * (ny + 1);
    let nonzeros = 18 * unknowns;
    Entry {
        id: format!("modes/strip-{step_nm}nm-{count}"),
        family: Family::Modes,
        title: format!(
            "the full-vector mode solver: {count} mode{} of a 500 × 220 nm silicon strip in \
             oxide, by shift-and-invert Arnoldi",
            if count == 1 { "" } else { "s" }
        ),
        size: nx.min(ny),
        grid: format!("{nx} × {ny} cells of {step_nm} nm, a 3 × 2 µm window"),
        unknowns,
        nonzeros,
        task: Task::Eigen { modes: count },
        // the shifted matrix's LU in faer's ordering, and the Krylov vectors
        memory_bytes: (16.0 * 2.0 * factor_entries(unknowns, false, false)
            + 24.0 * nonzeros as f64
            + 16.0 * (2 * count + 20).max(40) as f64 * unknowns as f64)
            as u64,
        tier: match unknowns {
            ..=20_000 => Tier::Quick,
            20_001..=120_000 => Tier::Standard,
            _ => Tier::Full,
        },
        check: "each mode's eigen-residual".into(),
        kind: Kind::Modes { step_nm, count },
    }
}

fn circuit(points: usize) -> Entry {
    Entry {
        id: format!("circuit/mzi-{points}"),
        family: Family::Circuit,
        title: "a Mach–Zehnder interferometer's netlist (two couplers, two arms), its S-matrix \
                at each wavelength"
            .into(),
        size: points,
        grid: format!("no grid: exact; {points} wavelengths from 1.5 to 1.6 µm"),
        // the waves at the four components' ports
        unknowns: 12,
        nonzeros: 4 * 4 + 4 * 4 + 2 * 2 + 2 * 2,
        task: Task::Dense,
        memory_bytes: 1 << 20,
        tier: if points > 1000 {
            Tier::Standard
        } else {
            Tier::Quick
        },
        check: "the closed form of the interferometer".into(),
        kind: Kind::Circuit { points },
    }
}

fn dense(n: usize, product: bool) -> Entry {
    Entry {
        id: format!("dense/{}-{n}", if product { "product" } else { "lu" }),
        family: Family::Dense,
        title: if product {
            "the product of two complex n × n matrices, by faer's kernel on rayon's threads".into()
        } else {
            "the LU with partial pivoting of a complex n × n matrix, by faer's kernel on \
             rayon's threads"
                .into()
        },
        size: n,
        grid: format!("no grid: a dense {n} × {n} complex matrix"),
        unknowns: n,
        nonzeros: n * n,
        task: Task::Dense,
        memory_bytes: (16 * n * n * if product { 3 } else { 2 }) as u64,
        tier: match n {
            ..=512 => Tier::Quick,
            513..=2048 => Tier::Standard,
            _ => Tier::Full,
        },
        check: if product {
            "(A B) v against A (B v)".into()
        } else {
            "‖b − A x‖ / ‖b‖ of a solve with the factors".into()
        },
        kind: if product {
            Kind::DenseProduct { n }
        } else {
            Kind::DenseLu { n }
        },
    }
}

/// Every problem, family by family, each family from its smallest size.
pub fn catalogue() -> Vec<Entry> {
    let mut all = Vec::new();
    // 2D: squares from 10⁴ to 2 × 10⁶ cells, and long grids
    let squares = [100, 200, 400, 700, 1000, 1400];
    for shape in [Flat::Slab, Flat::Bend, Flat::Mmi, Flat::Ring] {
        for c in squares {
            all.push(flat(shape, Polarization::Ez, [c, c], false));
        }
    }
    for c in squares {
        all.push(flat(Flat::Slab, Polarization::Hz, [c, c], false));
        all.push(flat(Flat::Ring, Polarization::Hz, [c, c], false));
        all.push(flat(Flat::Slab, Polarization::Ez, [c, c], true));
        all.push(flat(Flat::Slab, Polarization::Hz, [c, c], true));
    }
    for c in [100, 200, 400] {
        all.push(flat(Flat::Slab, Polarization::Ez, [4 * c, c], false));
        all.push(flat(Flat::Mmi, Polarization::Ez, [4 * c, c], false));
    }
    all.push(exported("slab-2d"));
    // 3D: the strip in cubes, devices in boxes, stretched PMLs, a Bloch grating
    for c in [16, 20, 28, 36, 48, 56] {
        all.push(solid(Solid::Strip, [c, c, c], false));
    }
    for id in ["strip-24", "strip-32", "strip-40"] {
        all.push(exported(id));
    }
    for c in [16, 24, 32] {
        all.push(solid(Solid::Strip, [3 * c, 2 * c, c], false));
        all.push(solid(Solid::Bend, [2 * c, 2 * c, c], false));
        all.push(solid(Solid::Mmi, [3 * c, 2 * c, c], false));
        all.push(solid(Solid::Coupler, [3 * c, 2 * c, c], false));
        all.push(solid(Solid::Strip, [c, c, c], true));
        all.push(solid(Solid::Grating, [c, c, c], false));
    }
    // the iterative solvers' guide, smaller than the fixed problems'
    for (core, pml) in [(5, 5), (8, 7)] {
        for solve in [
            super::Solve::Qmr,
            super::Solve::Ilu,
            super::Solve::Multigrid,
        ] {
            all.push(guide(core, pml, solve));
        }
    }
    // QMR, plain and with ILU(0), and GMRES with multigrid, larger, where a GPU's bandwidth can
    // tell (#190)
    for (core, pml) in [(12, 10), (20, 14)] {
        all.push(guide(core, pml, super::Solve::Qmr));
        all.push(guide(core, pml, super::Solve::Ilu));
        all.push(guide(core, pml, super::Solve::Multigrid));
    }
    // the fixed problems, under their ids
    let fixed_problems = super::problems();
    for family in [Family::Fdfd2d, Family::Fdfd3d, Family::Iterative3d] {
        all.extend(
            fixed_problems
                .iter()
                .filter_map(|p| fixed(p.id, p.title, p.heavy))
                .filter(|e| e.family == family),
        );
    }
    for step_nm in [30, 20, 10, 6] {
        for count in [1, 8] {
            all.push(modes(step_nm, count));
        }
    }
    for points in [101, 10_001] {
        all.push(circuit(points));
    }
    for n in [32, 64, 128, 256, 512, 1024, 2048, 4096] {
        all.push(dense(n, false));
        all.push(dense(n, true));
    }
    all.extend(
        fixed_problems
            .iter()
            .filter_map(|p| fixed(p.id, p.title, p.heavy))
            .filter(|e| matches!(e.family, Family::Job | Family::Fdtd)),
    );
    all
}

/// The problems of a tier: its own and every shorter tier's.
pub fn tier(tier: Tier) -> Vec<Entry> {
    catalogue().into_iter().filter(|e| e.tier <= tier).collect()
}

/// The problem of an id.
pub fn entry(id: &str) -> Option<Entry> {
    catalogue().into_iter().find(|e| e.id == id)
}

/// A sparse system as its solver assembles it, with its unknowns' grid positions.
struct Assembled {
    triplets: Vec<Triplet<usize, usize, c64>>,
    positions: Vec<[f64; 3]>,
    rhs: Vec<c64>,
}

/// The right-hand side every direct problem takes: fixed, the same on every machine, zero where
/// `fixed` says the solver holds a value at zero.
fn rhs(n: usize, fixed: impl Fn(usize) -> bool) -> Vec<c64> {
    (0..n)
        .map(|i| {
            if fixed(i) {
                c64::new(0.0, 0.0)
            } else {
                c64::new(((i * 7919) % 13) as f64, 1.0)
            }
        })
        .collect()
}

const SILICON: f64 = 3.476;
const OXIDE: f64 = 1.444;

fn index(inside: bool) -> c64 {
    let n = if inside { SILICON } else { OXIDE };
    c64::new(n * n, 0.0)
}

/// Whether (x, y), from the window's centre, is in the 2D structure: the window is `lx` by `ly`.
fn in_flat(shape: Flat, (x, y): (f64, f64), (lx, ly): (f64, f64)) -> bool {
    match shape {
        Flat::Slab => y.abs() < 0.11,
        Flat::Bend => {
            // a quarter turn about the window's lower left corner
            let r = (x + lx / 2.0).hypot(y + ly / 2.0);
            (r - ly / 2.0).abs() < 0.25
        }
        Flat::Mmi => {
            let body = x.abs() < lx / 4.0 && y.abs() < ly / 6.0;
            let input = x <= -lx / 4.0 && y.abs() < 0.25;
            let outputs = x >= lx / 4.0 && (y.abs() - ly / 12.0).abs() < 0.25;
            body || input || outputs
        }
        Flat::Ring => {
            let radius = ly / 4.0;
            let ring = (x.hypot(y) - radius).abs() < 0.2;
            let bus = (y + radius + 0.45).abs() < 0.2;
            ring || bus
        }
    }
}

fn assemble_flat(
    shape: Flat,
    polarization: Polarization,
    [nx, ny]: [usize; 2],
    bloch: bool,
) -> Result<Assembled> {
    let (lx, ly) = (nx as f64 * H2, ny as f64 * H2);
    let g = Grid {
        nx,
        ny,
        dx: H2,
        dy: H2,
        x0: -lx / 2.0,
        y0: -ly / 2.0,
    };
    let boundaries = if bloch {
        Boundaries {
            x: Edges::Bloch { k: 1.0 },
            ..Boundaries::pml(PML2)
        }
    } else {
        Boundaries::pml(PML2)
    };
    crate::fdfd::check(g, &boundaries)?;
    let at = |x: f64, y: f64| index(in_flat(shape, (x, y), (lx, ly)));
    // E along z sees each cell's permittivity; H along z, the faces' between cells, sampled
    // at their centres
    let (mut eps_z, mut eps_y, mut eps_x) = (Vec::new(), Vec::new(), Vec::new());
    match polarization {
        Polarization::Ez => {
            for j in 0..ny {
                for i in 0..nx {
                    eps_z.push(at(g.x(i), g.y(j)));
                }
            }
        }
        Polarization::Hz => {
            for j in 0..ny {
                for i in 0..=nx {
                    eps_y.push(at(g.x0 + i as f64 * H2, g.y(j)));
                }
            }
            for j in 0..=ny {
                for i in 0..nx {
                    eps_x.push(at(g.x(i), g.y0 + j as f64 * H2));
                }
            }
        }
    }
    let k0 = std::f64::consts::TAU / Wavelength::um(1.55)?.to_um();
    let triplets = crate::fdfd::assemble(g, polarization, k0, &eps_z, &eps_y, &eps_x, &boundaries);
    let n = nx * ny;
    Ok(Assembled {
        triplets,
        positions: (0..n)
            .map(|k| [(k % nx) as f64, (k / nx) as f64, 0.0])
            .collect(),
        rhs: rhs(n, |_| false),
    })
}

/// Whether (x, y, z), from the box's centre, is in the 3D structure: the box is `lx` by `ly`
/// across.
fn in_solid(shape: Solid, (x, y, z): (f64, f64, f64), (lx, ly): (f64, f64)) -> bool {
    let core = z.abs() < 0.11;
    match shape {
        Solid::Strip => core && y.abs() < 0.25,
        Solid::Bend => {
            let r = (x + lx / 2.0).hypot(y + ly / 2.0);
            core && (r - ly / 2.0).abs() < 0.25
        }
        Solid::Mmi => core && ((x.abs() < lx / 4.0 && y.abs() < ly / 5.0) || y.abs() < 0.25),
        Solid::Coupler => core && (y.abs() - 0.35).abs() < 0.25,
        Solid::Grating => {
            // teeth 150 nm deep, every other 160 nm
            let tooth = (x / 0.16).floor().rem_euclid(2.0) < 1.0;
            y.abs() < 0.25 && z < 0.11 && z > if tooth { -0.11 } else { 0.04 }
        }
    }
}

fn assemble_solid(shape: Solid, cells: [usize; 3], stretched: bool) -> Result<Assembled> {
    let [nx, ny, nz] = cells;
    let (lx, ly, lz) = (nx as f64 * H3, ny as f64 * H3, nz as f64 * H3);
    let grid = Grid3d {
        nx,
        ny,
        nz,
        dx: H3,
        dy: H3,
        dz: H3,
        x0: -lx / 2.0,
        y0: -ly / 2.0,
        z0: -lz / 2.0,
    };
    let pml = pml3(cells);
    let boundaries = if shape == Solid::Grating {
        Boundaries3d {
            x: Edges::Bloch { k: 1.0 },
            ..Boundaries3d::pml(pml)
        }
    } else if stretched {
        Boundaries3d::stretched_pml(pml)
    } else {
        Boundaries3d::pml(pml)
    };
    let eps = move |x: f64, y: f64, z: f64| index(in_solid(shape, (x, y, z), (lx, ly)));
    let (lattice, eps) = Solver3d::setup(grid, Wavelength::um(1.55)?, eps, boundaries)?;
    let n = grid.unknowns();
    Ok(Assembled {
        triplets: lattice.assemble(&eps),
        positions: crate::fdfd::positions(&grid),
        rhs: rhs(n, |r| lattice.fixed(r)),
    })
}

fn assemble_exported(id: &str) -> Result<Assembled> {
    let system = super::export::system(id)?;
    let positions = match id
        .strip_prefix("strip-")
        .and_then(|c| c.parse::<usize>().ok())
    {
        Some(c) => crate::fdfd::positions(&Grid3d {
            nx: c,
            ny: c,
            nz: c,
            dx: H3,
            dy: H3,
            dz: H3,
            x0: 0.0,
            y0: 0.0,
            z0: 0.0,
        }),
        None => (0..system.n)
            .map(|k| [(k % 440) as f64, (k / 440) as f64, 0.0])
            .collect(),
    };
    Ok(Assembled {
        triplets: system
            .entries
            .iter()
            .map(|&(r, c, v)| Triplet::new(r, c, v))
            .collect(),
        positions,
        rhs: system.rhs,
    })
}

fn phase(name: &str, seconds: f64) -> Phase {
    Phase {
        name: name.into(),
        seconds,
        iterations: None,
        bytes: None,
    }
}

fn norm(v: &[c64]) -> f64 {
    v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt()
}

/// What a direct solve of an assembled system took, and its factors (which the tests hold the
/// records against).
#[cfg_attr(not(test), allow(dead_code))]
struct Solved {
    measurement: Measurement,
    /// Whether the factors are L D Lᵀ of the symmetric similarity.
    symmetric: bool,
    /// The matrix's nonzeros, repeated entries summed.
    nonzeros: usize,
}

fn solve_direct(entry: &Entry, direct: &Choice) -> Result<Solved> {
    let clock = Instant::now();
    let system = match &entry.kind {
        Kind::Flat {
            shape,
            polarization,
            cells,
            bloch,
        } => assemble_flat(*shape, *polarization, *cells, *bloch)?,
        Kind::Solid {
            shape,
            cells,
            stretched,
        } => assemble_solid(*shape, *cells, *stretched)?,
        Kind::Exported(id) => assemble_exported(id)?,
        _ => return Err(Error::invalid("bench", "not a direct problem")),
    };
    let mut phases = vec![phase("assembly", clock.elapsed().as_secs_f64())];
    let n = system.positions.len();
    let mut seen: Vec<(usize, usize)> = system.triplets.iter().map(|t| (t.col, t.row)).collect();
    seen.sort_unstable();
    seen.dedup();
    let clock = Instant::now();
    let factors = Direct::new(&system.triplets, &system.positions, Plan::new(direct)?)?;
    phases.push(phase(
        "analysis and factorization",
        clock.elapsed().as_secs_f64(),
    ));
    let clock = Instant::now();
    let x = factors.solve(&system.rhs)?;
    phases.push(phase("one solve", clock.elapsed().as_secs_f64()));
    let mut r = system.rhs.clone();
    for t in &system.triplets {
        r[t.row] -= t.val * x[t.col];
    }
    Ok(Solved {
        measurement: Measurement {
            grid: entry.grid.clone(),
            unknowns: n,
            phases,
            accuracy: Some(Accuracy {
                error: norm(&r) / norm(&system.rhs),
                against: entry.check.clone(),
            }),
            factor_entries: factors.report().factor_entries.map(|e| e as u64),
        },
        symmetric: factors.symmetric(),
        nonzeros: seen.len(),
    })
}

fn solve_modes(entry: &Entry, step_nm: usize, count: usize) -> Result<Measurement> {
    use crate::mode::vector::{CrossSection, Permittivity, modes};
    let [nx, ny] = mode_cells(step_nm);
    let clock = Instant::now();
    let cs = CrossSection::uniform((-1.5, 1.5, nx), (-1.0, 1.0, ny), |x, y| {
        Permittivity::isotropic(index(x.abs() < 0.25 && y.abs() < 0.11))
    })?;
    let found = modes(&cs, Wavelength::um(1.55)?, count, None)?;
    let seconds = clock.elapsed().as_secs_f64();
    let mut worst: f64 = 0.0;
    for mode in &found {
        worst = worst.max(mode.residual(&cs)?);
    }
    if found.len() != count {
        return Err(Error::invalid(
            "bench",
            format!("{} modes found of {count}", found.len()),
        ));
    }
    Ok(Measurement {
        grid: entry.grid.clone(),
        unknowns: cs.unknowns(),
        phases: vec![phase("the modes", seconds)],
        accuracy: Some(Accuracy {
            error: worst,
            against: entry.check.clone(),
        }),
        factor_entries: None,
    })
}

fn solve_circuit(entry: &Entry, points: usize) -> Result<Measurement> {
    use crate::circuit::Component;
    use crate::circuit::components::{Coupler, Dispersion, Waveguide, mzi, mzi_closed_form};
    let guide = Waveguide::new(Dispersion::new(Wavelength::um(1.55)?, 2.4, 4.2));
    let coupler: Arc<dyn Component> = Arc::new(Coupler::new());
    let arm: Arc<dyn Component> = Arc::new(guide.clone());
    let mut netlist = mzi(coupler.clone(), coupler, arm.clone(), arm)?;
    netlist.set("splitter", "coupling", 0.5)?;
    netlist.set("combiner", "coupling", 0.3)?;
    netlist.set("upper", "length", 150.0)?;
    netlist.set("lower", "length", 50.0)?;
    let wavelengths: Vec<Wavelength> = (0..points)
        .map(|k| Wavelength::um(1.5 + 0.1 * k as f64 / (points - 1).max(1) as f64))
        .collect::<Result<_>>()?;
    let clock = Instant::now();
    let circuit = netlist.compile()?;
    let mut matrices = Vec::with_capacity(points);
    for &w in &wavelengths {
        matrices.push(circuit.s_matrix(w)?);
    }
    let seconds = clock.elapsed().as_secs_f64();
    let split = |k: f64| (c64::new((1.0 - k).sqrt(), 0.0), c64::new(0.0, k.sqrt()));
    let mut worst: f64 = 0.0;
    for (&w, s) in wavelengths.iter().zip(&matrices) {
        let through =
            |length: f64| -> Result<c64> { Ok(guide.s_matrix(w, &[length, 0.0])?[(1, 0)]) };
        let exact = mzi_closed_form(split(0.5), split(0.3), through(150.0)?, through(50.0)?);
        for (q, out) in [(0, 3), (1, 2)] {
            for p in 0..2 {
                worst = worst.max((s[(out, p)] - exact[q][p]).norm());
            }
        }
    }
    Ok(Measurement {
        grid: entry.grid.clone(),
        unknowns: entry.unknowns,
        phases: vec![phase("the netlist compiled and its sweep", seconds)],
        accuracy: Some(Accuracy {
            error: worst,
            against: entry.check.clone(),
        }),
        factor_entries: None,
    })
}

/// A dense matrix of fixed entries in (−½, ½)², the same on every machine.
fn dense_matrix(n: usize, seed: u64) -> faer::Mat<c64> {
    let mut s = seed;
    let mut next = move || {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((s >> 11) as f64) / ((1u64 << 53) as f64) - 0.5
    };
    faer::Mat::from_fn(n, n, |_, _| c64::new(next(), next()))
}

fn times(a: &faer::Mat<c64>, v: &[c64]) -> Vec<c64> {
    let n = v.len();
    (0..a.nrows())
        .map(|i| (0..n).map(|j| a[(i, j)] * v[j]).sum())
        .collect()
}

fn solve_dense(entry: &Entry, n: usize, product: bool) -> Result<Measurement> {
    use faer::dyn_stack::{MemBuffer, MemStack};
    use faer::linalg::lu::partial_pivoting::factor::{lu_in_place, lu_in_place_scratch};
    use faer::linalg::matmul::matmul;
    use faer::{Accum, Par};
    let par = Par::rayon(0);
    let a = dense_matrix(n, 1);
    let v: Vec<c64> = rhs(n, |_| false);
    let (name, seconds, error) = if product {
        let b = dense_matrix(n, 2);
        let mut c = faer::Mat::<c64>::zeros(n, n);
        let clock = Instant::now();
        matmul(
            c.as_mut(),
            Accum::Replace,
            a.as_ref(),
            b.as_ref(),
            c64::new(1.0, 0.0),
            par,
        );
        let seconds = clock.elapsed().as_secs_f64();
        let (got, want) = (times(&c, &v), times(&a, &times(&b, &v)));
        let off: Vec<c64> = got.iter().zip(&want).map(|(g, w)| g - w).collect();
        ("the product", seconds, norm(&off) / norm(&want))
    } else {
        let mut lu = a.clone();
        let mut forward = vec![0usize; n];
        let mut inverse = vec![0usize; n];
        let mut stack = MemBuffer::new(lu_in_place_scratch::<usize, c64>(
            n,
            n,
            par,
            Default::default(),
        ));
        let clock = Instant::now();
        lu_in_place(
            lu.as_mut(),
            &mut forward,
            &mut inverse,
            par,
            MemStack::new(&mut stack),
            Default::default(),
        );
        let seconds = clock.elapsed().as_secs_f64();
        // P A = L U: x = U⁻¹ L⁻¹ P b
        let mut x: Vec<c64> = forward.iter().map(|&r| v[r]).collect();
        for i in 0..n {
            let mut sum = x[i];
            for j in 0..i {
                sum -= lu[(i, j)] * x[j];
            }
            x[i] = sum;
        }
        for i in (0..n).rev() {
            let mut sum = x[i];
            for j in i + 1..n {
                sum -= lu[(i, j)] * x[j];
            }
            x[i] = sum / lu[(i, i)];
        }
        let ax = times(&a, &x);
        let r: Vec<c64> = v.iter().zip(&ax).map(|(b, a)| b - a).collect();
        ("the factorization", seconds, norm(&r) / norm(&v))
    };
    Ok(Measurement {
        grid: entry.grid.clone(),
        unknowns: n,
        phases: vec![phase(name, seconds)],
        accuracy: Some(Accuracy {
            error,
            against: entry.check.clone(),
        }),
        factor_entries: None,
    })
}

impl Entry {
    /// Whether the problem has a sparse direct solve that a backend can take: the 2D and 3D
    /// systems. The others run on photonoxide's own code whatever the choice.
    pub fn takes_a_backend(&self) -> bool {
        matches!(
            self.kind,
            Kind::Flat { .. } | Kind::Solid { .. } | Kind::Exported(_)
        )
    }

    /// Whether its solve is QMR, plain or with ILU(0), or GMRES with multigrid, which an
    /// iterative backend can run ([`crate::backend::iterative`]): `fdfd3d-iterative/guide-*`.
    pub fn takes_an_iterative_backend(&self) -> bool {
        matches!(self.kind, Kind::Guide { .. })
    }

    /// The error its check allows: a solve that leaves more hasn't solved the problem.
    pub fn tolerance(&self) -> f64 {
        match self.kind {
            Kind::Flat { .. } | Kind::Solid { .. } | Kind::Exported(_) => 1e-9,
            Kind::Modes { .. } => 1e-9,
            Kind::Circuit { .. } => 1e-12,
            Kind::DenseLu { .. } | Kind::DenseProduct { .. } => 1e-9,
            // a field converged to 1e-6 or 1e-8 in its residual, against the direct solver's
            Kind::Guide { .. } => 1e-5,
            // their own checks differ: an S-matrix against its exact one, a field against a
            // direct solve's
            Kind::Fixed(_) => f64::INFINITY,
        }
    }

    /// Builds the problem and solves it, timing each phase: a direct problem by the direct
    /// solver `backend` names ([`Entry::takes_a_backend`]), plain QMR by the iterative one it
    /// names ([`Entry::takes_an_iterative_backend`]), the others as photonoxide solves them.
    ///
    /// # Errors
    ///
    /// The assembly's and the solver's, and [`Error::InvalidValue`] for a backend that isn't
    /// registered or isn't available.
    pub fn run(&self, backend: &Choice) -> Result<Measurement> {
        let direct = backend;
        match &self.kind {
            Kind::Flat { .. } | Kind::Solid { .. } | Kind::Exported(_) => {
                Ok(solve_direct(self, direct)?.measurement)
            }
            Kind::Fixed(id) => super::problems()
                .into_iter()
                .find(|p| p.id == *id)
                .ok_or_else(|| Error::invalid("bench", format!("no problem {id}")))?
                .run(),
            Kind::Guide { core, pml, solve } => {
                let tolerance = match self.task {
                    Task::Iterative { tolerance } => tolerance,
                    _ => 1e-8,
                };
                // the iterative backend of the choice runs the Krylov solver; the others are
                // photonoxide's
                let iterative = if self.takes_an_iterative_backend() {
                    direct
                } else {
                    &Choice::Auto
                };
                super::guide_3d_with(
                    super::Guide::Cube { core: *core },
                    *pml,
                    *solve,
                    tolerance,
                    iterative,
                    &mut || {},
                )
            }
            Kind::Modes { step_nm, count } => solve_modes(self, *step_nm, *count),
            Kind::Circuit { points } => solve_circuit(self, *points),
            Kind::DenseLu { n } => solve_dense(self, *n, false),
            Kind::DenseProduct { n } => solve_dense(self, *n, true),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_id_is_its_own_and_the_old_ones_are_kept() {
        let all = catalogue();
        let ids: HashSet<&str> = all.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids.len(), all.len());
        // the exported systems and `photonoxide bench`'s problems, as they were
        for id in super::super::export::IDS {
            assert!(ids.contains(id), "{id}");
        }
        for problem in super::super::problems() {
            assert!(ids.contains(problem.id), "{}", problem.id);
        }
        // every family is there, each entry's id begins with its family's name or is an older
        // one, and no record is without its grid
        for &family in Family::ALL {
            assert!(all.iter().any(|e| e.family == family), "{family:?}");
        }
        for e in &all {
            let old = matches!(e.kind, Kind::Exported(_) | Kind::Fixed(_));
            assert!(
                old || e.id.starts_with(&format!("{}/", e.family.name())),
                "{}",
                e.id
            );
            assert!(!e.grid.is_empty() && !e.title.is_empty() && !e.check.is_empty());
            assert!(e.memory_bytes > 0, "{}", e.id);
            assert_eq!(entry(&e.id).as_ref(), Some(e));
        }
        assert!(entry("fdfd2d/no-such").is_none());
        assert!(all.len() > 100, "{}", all.len());
    }

    #[test]
    fn each_tier_holds_the_one_before_it_and_every_family() {
        let (quick, standard, full) = (tier(Tier::Quick), tier(Tier::Standard), tier(Tier::Full));
        assert!(quick.len() < standard.len() && standard.len() < full.len());
        assert_eq!(full, catalogue());
        assert!(quick.iter().all(|e| standard.contains(e)));
        // the quick tier has the smallest of each family but the whole jobs
        for &family in Family::ALL {
            let there = quick.iter().any(|e| e.family == family);
            assert_eq!(there, family != Family::Job, "{family:?}");
        }
        // and nothing in it is expected to take more than a gigabyte
        for e in &quick {
            assert!(e.memory_bytes < 1 << 30, "{}: {}", e.id, e.memory_bytes);
        }
        for t in Tier::ALL {
            assert_eq!(Tier::parse(t.name()), Some(t));
        }
        assert_eq!(Tier::parse("everything"), None);
    }

    /// The smallest entries of each direct shape: the quick tier's.
    fn quick_direct() -> Vec<Entry> {
        tier(Tier::Quick)
            .into_iter()
            .filter(Entry::takes_a_backend)
            .collect()
    }

    #[test]
    fn the_smallest_direct_problems_are_solved_to_their_check_as_their_records_say() {
        let direct = quick_direct();
        // every 2D shape, both polarizations, Bloch sides, a long grid; the 3D cube, each
        // device's box, stretched PMLs and the grating
        assert!(direct.len() >= 16, "{}", direct.len());
        for e in &direct {
            let solved = solve_direct(e, &Choice::Auto).unwrap_or_else(|x| panic!("{}: {x}", e.id));
            let m = &solved.measurement;
            let error = m.accuracy.as_ref().unwrap().error;
            assert!(error < e.tolerance(), "{}: {error}", e.id);
            assert_eq!(m.unknowns, e.unknowns, "{}", e.id);
            assert_eq!(m.grid, e.grid);
            // the task is what the solver did: L D Lᵀ where the record says symmetric
            assert_eq!(
                solved.symmetric,
                e.task == Task::DirectSymmetric,
                "{}",
                e.id
            );
            // the nonzeros: counted in 2D, the stencil's 13 an unknown in 3D, less what the
            // walls and the PMLs' edges take
            match e.family {
                Family::Fdfd2d => assert_eq!(solved.nonzeros, e.nonzeros, "{}", e.id),
                _ => {
                    let ratio = solved.nonzeros as f64 / e.nonzeros as f64;
                    assert!((0.6..=1.0).contains(&ratio), "{}: {ratio}", e.id);
                }
            }
            // the memory, within a factor of 2 of the factors and the matrix
            let factors = m.factor_entries.unwrap() as usize;
            let measured = 16 * factors + 24 * solved.nonzeros;
            let ratio = e.memory_bytes as f64 / measured as f64;
            assert!((0.5..=2.0).contains(&ratio), "{}: {ratio}", e.id);
        }
    }

    #[test]
    fn a_direct_problem_runs_on_the_backend_it_is_given() {
        use crate::backend::tests::Recording;
        let recording = Recording::register("recording-catalogue");
        let choice = Choice::parse("recording-catalogue").unwrap();
        let e = entry("fdfd2d/slab-ez-bloch-100").unwrap();
        assert!(e.takes_a_backend());
        let own = e.run(&Choice::Auto).unwrap();
        let routed = e.run(&choice).unwrap();
        assert_eq!(recording.counts(), [1, 1, 1, 0]);
        // the same factors: the same residual to the bit, and the entries the backend reports
        assert_eq!(own.accuracy, routed.accuracy);
        let reported = recording.reports.lock().unwrap()[0].factor_entries;
        assert!(reported.is_some_and(|e| e > 0));
        assert_eq!(routed.factor_entries, reported.map(|e| e as u64));
        assert_eq!(own.factor_entries, routed.factor_entries);
        // faer's LU solves it too, to the check
        let faer = e.run(&Choice::parse("faer").unwrap()).unwrap();
        assert!(faer.accuracy.unwrap().error < e.tolerance());
        // a backend that isn't there is an error
        assert!(
            e.run(&Choice::parse("not-registered-catalogue").unwrap())
                .is_err()
        );
        // and the others take no backend
        assert!(!entry("dense/lu-32").unwrap().takes_a_backend());
    }

    #[test]
    fn the_smallest_of_the_other_families_are_solved_to_their_checks() {
        for id in [
            "fdfd3d-iterative/guide-qmr-20",
            "fdfd3d-iterative/guide-ilu-20",
            "fdfd3d-iterative/guide-multigrid-20",
            "modes/strip-30nm-1",
            "modes/strip-30nm-8",
            "circuit/mzi-101",
            "dense/lu-32",
            "dense/product-32",
            "dense/lu-256",
            "dense/product-256",
        ] {
            let e = entry(id).unwrap();
            assert_eq!(e.tier, Tier::Quick, "{id}");
            let m = e.run(&Choice::Auto).unwrap_or_else(|x| panic!("{id}: {x}"));
            let error = m.accuracy.as_ref().unwrap().error;
            assert!(error < e.tolerance(), "{id}: {error}");
            assert_eq!(m.unknowns, e.unknowns, "{id}");
            assert_eq!(m.grid, e.grid, "{id}");
            assert!(m.seconds() > 0.0);
        }
    }

    #[test]
    fn sizes_scale_as_declared() {
        // unknowns as the cells: a square of twice the side has four times as many, a cube
        // eight
        let unknowns = |id: &str| entry(id).unwrap().unknowns;
        assert_eq!(
            unknowns("fdfd2d/slab-ez-200"),
            4 * unknowns("fdfd2d/slab-ez-100")
        );
        assert_eq!(
            unknowns("fdfd2d/ring-hz-400"),
            16 * unknowns("fdfd2d/ring-hz-100")
        );
        assert_eq!(unknowns("fdfd3d/strip-56"), 8 * unknowns("fdfd3d/strip-28"));
        assert_eq!(unknowns("dense/lu-4096"), 4096);
        // the memory grows faster than the unknowns and slower than their square
        for (small, large) in [
            ("fdfd2d/slab-ez-100", "fdfd2d/slab-ez-1400"),
            ("fdfd3d/strip-16", "fdfd3d/strip-56"),
            ("modes/strip-30nm-1", "modes/strip-6nm-1"),
        ] {
            let (a, b) = (entry(small).unwrap(), entry(large).unwrap());
            let n = b.unknowns as f64 / a.unknowns as f64;
            let m = b.memory_bytes as f64 / a.memory_bytes as f64;
            assert!(m > n && m < n * n, "{small} to {large}: {m} for {n}");
        }
        // the families span the sizes the issue asks for
        let span = |family: Family| {
            let of: Vec<usize> = catalogue()
                .iter()
                .filter(|e| e.family == family && !matches!(e.kind, Kind::Fixed(_)))
                .map(|e| e.unknowns)
                .collect();
            (*of.iter().min().unwrap(), *of.iter().max().unwrap())
        };
        let (least, most) = span(Family::Fdfd2d);
        assert!(least <= 10_000 && most >= 1_900_000, "{least} {most}");
        let (least, most) = span(Family::Fdfd3d);
        assert!(
            least == 3 * 16 * 16 * 16 && most >= 500_000,
            "{least} {most}"
        );
        let (least, most) = span(Family::Modes);
        assert!(least <= 15_000 && most >= 300_000, "{least} {most}");
        assert_eq!(span(Family::Dense), (32, 4096));
    }

    #[test]
    #[ignore = "the second size of each direct family, to hold the memory estimate there too: cargo test --release -- --ignored a_larger"]
    fn a_larger_size_of_each_direct_family_keeps_its_record() {
        for id in [
            "fdfd2d/slab-ez-400",
            "fdfd2d/ring-hz-400",
            "fdfd2d/slab-ez-bloch-400",
            "fdfd2d/mmi-ez-1600x400",
            "fdfd3d/strip-28",
            "fdfd3d/coupler-72x48x24",
            "fdfd3d/grating-24",
            "strip-24",
            "slab-2d",
        ] {
            let e = entry(id).unwrap_or_else(|| panic!("{id}"));
            let solved = solve_direct(&e, &Choice::Auto).unwrap();
            let error = solved.measurement.accuracy.as_ref().unwrap().error;
            assert!(error < e.tolerance(), "{id}: {error}");
            assert_eq!(solved.measurement.unknowns, e.unknowns, "{id}");
            let factors = solved.measurement.factor_entries.unwrap() as usize;
            let measured = 16 * factors + 24 * solved.nonzeros;
            let ratio = e.memory_bytes as f64 / measured as f64;
            println!(
                "{id}: {} unknowns, {} nonzeros (record {}), factors {} entries, memory record/measured {ratio:.2}, symmetric {}",
                e.unknowns, solved.nonzeros, e.nonzeros, factors, solved.symmetric
            );
            assert!((0.5..=2.0).contains(&ratio), "{id}: {ratio}");
        }
    }
}
