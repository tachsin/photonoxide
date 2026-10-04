//! The 3D port setups that both the tests and the validation report run: port modes against
//! the exact slab and the full-vector mode solver, a straight strip, a step's reciprocity, a
//! closed guide's energy, and a structure invariant along z against the 2D solver's S-matrix.

use num_complex::Complex64 as c64;

use super::{Axis, Boundaries3d, Grid3d, Port3d, PortMode3d, Solver3d};
use crate::fdfd::{Boundaries, Edges, Grid, Polarization, Side, Solver2d};
use crate::mode::Polarization as Kind;
use crate::units::{Length, Wavelength};

/// Silicon and oxide at 1.55 µm, as everywhere in these checks.
pub(crate) const SILICON: f64 = 3.476;
/// Oxide.
pub(crate) const OXIDE: f64 = 1.444;

fn lam() -> Wavelength {
    Wavelength::um(1.55).unwrap()
}

fn index(n: f64) -> c64 {
    c64::new(n * n, 0.0)
}

/// A walled boundary on every side: a PML of no cells.
const WALL: Edges = Edges::Pml { low: 0, high: 0 };

/// The share of a mode's tangential field along `component`, from 0 to 1.
pub(crate) fn share(mode: &PortMode3d, grid: &Grid3d, component: Axis) -> f64 {
    let (b, c) = mode.axis().others();
    let total = |which: Axis| -> f64 {
        (0..grid.n(c))
            .flat_map(|v| (0..grid.n(b)).map(move |u| (u, v)))
            .map(|uv| mode.e(which, uv).norm_sqr())
            .sum()
    };
    total(component) / (total(b) + total(c))
}

/// The fundamental `kind` mode of 220 nm of silicon in oxide at 1.55 µm, the slab normal to z
/// and uniform along y (Bloch-periodic, one cell), on a port normal to x of an `h` grid, walls
/// 2 µm from the slab: its effective index, and the exact slab's.
pub(crate) fn slab_port_index(kind: Kind, h: f64) -> (f64, f64) {
    use crate::mode::slab::Slab;
    let exact = Slab::new(OXIDE, SILICON, OXIDE, Length::nm(220.0))
        .unwrap()
        .modes(kind, lam())[0]
        .effective_index();
    let nz = (4.0 / h).round() as usize;
    let grid = Grid3d {
        nx: 5,
        ny: 1,
        nz,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: 0.0,
        z0: -(nz as f64) * h / 2.0,
    };
    let boundaries = Boundaries3d {
        x: WALL,
        y: Edges::Bloch { k: 0.0 },
        z: WALL,
        reflection: 1e-8,
        order: 3.0,
        real_stretch: 0.0,
    };
    let slab = |_: f64, _: f64, z: f64| index(if z.abs() < 0.11 { SILICON } else { OXIDE });
    let (lattice, eps) = Solver3d::setup(grid, lam(), slab, boundaries).unwrap();
    let modes = lattice
        .port_modes(&eps, (Axis::X, 2), [0..1, 0..nz], 2)
        .unwrap();
    // TE has E along the slab (y), TM across it (z)
    let along = match kind {
        Kind::Te => Axis::Y,
        Kind::Tm => Axis::Z,
    };
    let mode = modes
        .iter()
        .find(|m| share(m, &grid, along) > 0.5)
        .expect("the slab's mode");
    (mode.effective_index().re, exact)
}

/// The strip of these checks: 0.5 × 0.22 µm of silicon in oxide, along x, centred on y = z = 0.
pub(crate) fn strip(_: f64, y: f64, z: f64) -> c64 {
    index(if y.abs() < 0.25 && z.abs() < 0.11 {
        SILICON
    } else {
        OXIDE
    })
}

/// The strip's window: walls 2.02 × 3.5 µm around it, far enough that neither mode feels them
/// (the TM-like one reaches furthest, up and down), with every interface on a node of grids
/// of 20 nm and finer.
const STRIP_WINDOW: (f64, f64) = (2.02, 3.5);

/// The strip's fundamental TE-like and TM-like modes on a 3D port's plane of an `h` grid (E_t
/// on the Yee plane), inside walls (`STRIP_WINDOW`): their effective indices.
pub(crate) fn strip_port_indices(h: f64) -> [f64; 2] {
    let (ny, nz) = (
        (STRIP_WINDOW.0 / h).round() as usize,
        (STRIP_WINDOW.1 / h).round() as usize,
    );
    let grid = Grid3d {
        nx: 5,
        ny,
        nz,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: -STRIP_WINDOW.0 / 2.0,
        z0: -STRIP_WINDOW.1 / 2.0,
    };
    let boundaries = Boundaries3d {
        x: WALL,
        y: WALL,
        z: WALL,
        reflection: 1e-8,
        order: 3.0,
        real_stretch: 0.0,
    };
    let (lattice, eps) = Solver3d::setup(grid, lam(), strip, boundaries).unwrap();
    let ports = lattice
        .port_modes(&eps, (Axis::X, 2), [0..ny, 0..nz], 2)
        .unwrap();
    let pick = |along: Axis| {
        ports
            .iter()
            .find(|m| share(m, &grid, along) > 0.5)
            .expect("the strip's mode")
            .effective_index()
            .re
    };
    [pick(Axis::Y), pick(Axis::Z)]
}

/// The same modes by the full-vector mode solver (H_t at the nodes, Fallahkhair et al.) and, with
/// `hadley`, by Hadley's high-accuracy equations as well (else NaN), on the same grid and window:
/// TE-like and TM-like by each, in that order.
#[cfg(test)]
pub(crate) fn strip_solver_indices(h: f64, hadley: bool) -> [f64; 4] {
    use crate::mode::vector::{CrossSection, Permittivity, VectorMode, modes};
    let (w, t) = (STRIP_WINDOW.0 / 2.0, STRIP_WINDOW.1 / 2.0);
    let cs = CrossSection::uniform(
        (-w, w, (2.0 * w / h).round() as usize),
        (-t, t, (2.0 * t / h).round() as usize),
        |y, z| Permittivity::isotropic(strip(0.0, y, z)),
    )
    .unwrap();
    let vector = modes(&cs, lam(), 2, None).unwrap();
    let accurate = if hadley {
        crate::mode::hadley::modes(&cs, lam(), 2, None).unwrap()
    } else {
        Vec::new()
    };
    // the mode solvers' x and y are the port's y and z: TE-like has H along their y
    let by = |modes: &[VectorMode], te: bool| {
        modes
            .iter()
            .find(|m| (m.te_fraction() > 0.5) == te)
            .map_or(f64::NAN, |m| m.effective_index().re)
    };
    [
        by(&vector, true),
        by(&vector, false),
        by(&accurate, true),
        by(&accurate, false),
    ]
}

/// A strip 0.22 µm tall of silicon in oxide along x, `width(x)` wide and centred on
/// y = `offset`, on a 50 nm grid of `nx` × 20 × 16 cells (y and z centred on the strip's
/// axis), with PMLs of 5 cells along x and 4 across: small enough for the direct solver in
/// a second, with the PMLs close enough to the strip that its modes reach into them.
fn strip_solver(nx: usize, width: impl Fn(f64) -> f64, offset: f64) -> Solver3d {
    let h = 0.05;
    let (ny, nz) = (20, 16);
    let grid = Grid3d {
        nx,
        ny,
        nz,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: -(ny as f64) * h / 2.0,
        z0: -(nz as f64) * h / 2.0,
    };
    let eps = move |x: f64, y: f64, z: f64| {
        index(if (y - offset).abs() < width(x) / 2.0 && z.abs() < 0.11 {
            SILICON
        } else {
            OXIDE
        })
    };
    let boundaries = Boundaries3d {
        x: Edges::Pml { low: 5, high: 5 },
        ..Boundaries3d::pml(4)
    };
    Solver3d::new(grid, lam(), eps, boundaries).unwrap()
}

/// The fundamental modes at planes `left` and `right` along x, as the device's two ports.
fn two_ports(solver: &Solver3d, left: usize, right: usize) -> Vec<Port3d> {
    let mode = |plane: usize| solver.port_modes(Axis::X, plane, 1).unwrap().remove(0);
    vec![
        Port3d {
            mode: mode(left),
            side: Side::Left,
        },
        Port3d {
            mode: mode(right),
            side: Side::Right,
        },
    ]
}

/// A straight strip 0.5 µm wide between ports 0.25 µm apart (`strip_solver`'s grid): the
/// largest of |S11|, |S22|, |S21 − e^(iβL)| and |S12 − e^(iβL)|, and the mode's effective index.
pub(crate) fn straight_strip() -> (f64, c64) {
    let solver = strip_solver(20, |_| 0.5, 0.0);
    let ports = two_ports(&solver, 7, 12);
    let beta = ports[0].mode.beta();
    let expected = (c64::new(0.0, 1.0) * beta * 5.0 * 0.05).exp();
    let s = solver.s_matrix(&ports).unwrap();
    let worst = [
        s[0][0].norm(),
        s[1][1].norm(),
        (s[1][0] - expected).norm(),
        (s[0][1] - expected).norm(),
    ]
    .into_iter()
    .fold(0.0, f64::max);
    (worst, ports[0].mode.effective_index())
}

/// A strip stepping from 0.4 to 0.6 µm wide at x = 0.6 µm, 50 nm off the grid's axis so that
/// nothing is symmetric (`strip_solver`'s grid, 24 cells along x): its S-matrix between the two
/// guides' fundamental modes.
pub(crate) fn width_step() -> Vec<Vec<c64>> {
    let solver = strip_solver(24, |x| if x < 0.6 { 0.4 } else { 0.6 }, 0.05);
    solver.s_matrix(&two_ports(&solver, 7, 16)).unwrap()
}

/// A strip in a closed metal box, 0.6 × 0.4 µm of oxide on a 50 nm grid, stepping from 0.3 to
/// 0.4 µm wide (0.2 µm tall, 50 nm off the box's axis), the ports `gap` cells from the step on
/// either side and PMLs of 6 cells along x: the S-matrix between all the propagating modes on
/// both sides, and how many there are on the left. Lossless and closed, it loses no power: S is
/// unitary, but for the power that the evanescent modes, decaying from the step and growing
/// back from the PMLs, carry across the ports' planes.
pub(crate) fn closed_step(gap: usize) -> (Vec<Vec<c64>>, usize) {
    let h = 0.05;
    let (ny, nz) = (12, 8);
    let nx = 2 * (6 + gap + 3);
    let grid = Grid3d {
        nx,
        ny,
        nz,
        dx: h,
        dy: h,
        dz: h,
        x0: -(nx as f64) * h / 2.0,
        y0: -(ny as f64) * h / 2.0,
        z0: -(nz as f64) * h / 2.0,
    };
    let eps = |x: f64, y: f64, z: f64| {
        let half = if x < 0.0 { 0.15 } else { 0.2 };
        index(if (y - 0.05).abs() < half && z.abs() < 0.1 {
            SILICON
        } else {
            OXIDE
        })
    };
    let boundaries = Boundaries3d {
        x: Edges::Pml { low: 6, high: 6 },
        y: WALL,
        z: WALL,
        reflection: 1e-8,
        order: 3.0,
        real_stretch: 0.0,
    };
    let solver = Solver3d::new(grid, lam(), eps, boundaries).unwrap();
    let propagating = |plane: usize| -> Vec<PortMode3d> {
        solver
            .port_modes(Axis::X, plane, 8)
            .unwrap()
            .into_iter()
            .filter(|m| m.beta().im.abs() < 1e-9 * m.beta().norm())
            .collect()
    };
    let (left, right) = (propagating(8), propagating(nx - 9));
    let count = left.len();
    let ports: Vec<Port3d> = left
        .into_iter()
        .map(|mode| Port3d {
            mode,
            side: Side::Left,
        })
        .chain(right.into_iter().map(|mode| Port3d {
            mode,
            side: Side::Right,
        }))
        .collect();
    (solver.s_matrix(&ports).unwrap(), count)
}

/// The largest |Σ_k S_kq* S_kp − δ_qp|: how far S is from unitary.
pub(crate) fn unitarity(s: &[Vec<c64>]) -> f64 {
    let n = s.len();
    let mut worst: f64 = 0.0;
    for q in 0..n {
        for p in 0..n {
            let sum: c64 = (0..n).map(|k| s[k][q].conj() * s[k][p]).sum();
            let delta = if p == q { 1.0 } else { 0.0 };
            worst = worst.max((sum - delta).norm());
        }
    }
    worst
}

/// A slab 220 nm of silicon in oxide stepping to 300 nm at x = 0.6 µm, uniform along z, on a
/// 20 nm grid with PMLs of 10 cells along x and y: the S-matrix between the two guides'
/// fundamental modes by the 2D solver (`polarization`) and by the 3D one (one cell along z,
/// periodic), the 3D one's reference planes moved to the 2D one's: the largest difference.
///
/// The 3D grid is placed as in [`super::checks::two_d_difference`], so that the two discrete
/// systems are the same equations. For H along z, the 3D port planes (nodes, where E_y lies)
/// are half a cell before the 2D columns (where H_z lies), and S's reference planes move by
/// e^(iβ_q δ_q + iβ_p δ_p), δ the outward shift.
/// Both solvers take a mode's backward twin with the same tangential E, so S's reflections agree
/// in sign as well as in size.
pub(crate) fn two_d_s_difference(polarization: Polarization) -> f64 {
    let (h, pml) = (0.02, 10);
    let (nx, ny) = (60 + 2 * pml, 100 + 2 * pml);
    let x0 = -(pml as f64) * h;
    let y0 = -1.0 - pml as f64 * h;
    let grid = Grid {
        nx,
        ny,
        dx: h,
        dy: h,
        x0,
        y0,
    };
    let slab = |x: f64, y: f64| {
        let half = if x < 0.6 { 0.11 } else { 0.15 };
        index(if y.abs() < half { SILICON } else { OXIDE })
    };
    let flat = Solver2d::new(grid, polarization, lam(), slab, Boundaries::pml(pml)).unwrap();
    let (c1, c2) = (pml + 10, pml + 50);
    let mode2 = |c: usize| flat.port_modes(c, 1).unwrap().remove(0);
    let ports2 = [
        crate::fdfd::Port {
            mode: mode2(c1),
            side: Side::Left,
        },
        crate::fdfd::Port {
            mode: mode2(c2),
            side: Side::Right,
        },
    ];
    let s2 = flat.s_matrix(&ports2).unwrap();
    let shift = match polarization {
        Polarization::Ez => 0.5 * h,
        Polarization::Hz => 0.0,
    };
    let grid3 = Grid3d {
        nx,
        ny,
        nz: 1,
        dx: h,
        dy: h,
        dz: h,
        x0: x0 + shift,
        y0: y0 + shift,
        z0: 0.0,
    };
    let boundaries3 = Boundaries3d {
        x: Edges::Pml {
            low: pml,
            high: pml,
        },
        y: Edges::Pml {
            low: pml,
            high: pml,
        },
        z: Edges::Bloch { k: 0.0 },
        reflection: 1e-8,
        order: 3.0,
        real_stretch: 0.0,
    };
    let deep = Solver3d::new(grid3, lam(), |x, y, _| slab(x, y), boundaries3).unwrap();
    // E along z: the mode with E_z; H along z: the one with E_y
    let along = match polarization {
        Polarization::Ez => Axis::Z,
        Polarization::Hz => Axis::Y,
    };
    let mode3 = |c: usize| {
        deep.port_modes(Axis::X, c, 2)
            .unwrap()
            .into_iter()
            .find(|m| share(m, &grid3, along) > 0.5)
            .unwrap()
    };
    let ports3 = [
        Port3d {
            mode: mode3(c1),
            side: Side::Left,
        },
        Port3d {
            mode: mode3(c2),
            side: Side::Right,
        },
    ];
    let s3 = deep.s_matrix(&ports3).unwrap();
    // the 3D planes' outward shift from the 2D columns, for H along z: half a cell out on the
    // left, half a cell in on the right
    let delta = match polarization {
        Polarization::Ez => [0.0, 0.0],
        Polarization::Hz => [0.5 * h, -0.5 * h],
    };
    let beta = [ports3[0].mode.beta(), ports3[1].mode.beta()];
    let mut worst: f64 = 0.0;
    for q in 0..2 {
        for p in 0..2 {
            let turn = c64::new(0.0, -1.0) * (beta[q] * delta[q] + beta[p] * delta[p]);
            // both solvers' backward modes have the forward ones' tangential E, so the reflections
            // agree in sign too
            worst = worst.max((s3[q][p] * turn.exp() - s2[q][p]).norm());
        }
    }
    worst
}
