//! The photonics inverse-design testbed's 3D metagrating, the deflector of its "3d mode
//! converters": a grating of stripes designed by topology optimization in 2D, and the testbed's
//! published designs and ours verified in 3D FDFD.
//!
//! M. Chen et al., "Validation and characterization of algorithms and software for photonics
//! inverse design", J. Opt. Soc. Am. B 41, A161 (2024),
//! [doi:10.1364/JOSAB.506412](https://doi.org/10.1364/JOSAB.506412), Section 3.F and Fig. 8, and
//! Supplement 1, Section 2.5 and Table 1; the designs and the scripts of
//! NanoComp/photonics-opt-testbed (MIT licence, commit 683c9249f21b), `Metagrating3D/`.
//!
//! **The problem** (Section 3.F; the testbed's `metagrating_meep.py`). A 325 nm silicon pattern
//! (n = 3.45) on a silica substrate (n = 1.45) under air, periodic along x (Px = λ/sin 50° =
//! 1.3707 µm) and y (Py = λ/2 = 0.525 µm), lit at normal incidence from the substrate at
//! λ = 1.05 µm with E along x (p, the testbed's TM); the figure of merit is the power into the
//! (+1, 0) order in air, 50° from the normal, over the power incident in the substrate. The paper
//! gives the indices as 3.452 and 1.452, the testbed's README and scripts as 3.45 and 1.45, by
//! which its designs were computed; we take the testbed's.
//!
//! **How it is computed.** In 2D, the x–z plane as our plane (its y our z) with H along the plane's
//! normal (`Polarization::Hz`), Bloch-periodic along x at k = 0, PMLs above and below; in 3D, the
//! same with y Bloch-periodic too, E on Yee's grid (`Solver3d`). A sheet of current in the
//! substrate (M_z in 2D, J_x in 3D) launches the wave; the incident power is the flux of the same
//! sheet in silica alone (as the testbed's script normalizes), and the order's power is the
//! scheme's own flux through a plane in air of the field's Fourier component at k_x = 2π/Px,
//! k_y = 0 (each component's sum taken at its own Yee positions, so the orders' powers add up to
//! the plane's flux). A stripe pattern, uniform along y, is a 2D problem exactly: the 2D number
//! is the device's, which the 3D run confirms.
//!
//! **Explore in 2D: a 1d pattern.** Chen et al.'s Fig. 8(e), Meep's design of stripes
//! (`device5.csv`, 119 values on the nodes of 118 intervals), reached 84.3 % in FDTD and 83.0 %
//! in RCWA (the testbed's README: 84.1 %); "80 % for 1d design patterns" is their typical optimum.
//! We design our own on 118 pixels of 11.6 nm: densities, a conic filter of 60 nm made periodic
//! (the pattern wrapped by the filter's reach), a tanh projection at η = ½ with β from 4 to 256
//! in seven stages of 40 evaluations (Meep's 4 to 256 and ∞, 80 each, at 86 pixels a µm),
//! genoxide's MMA on the efficiency's adjoint gradient (`Solver2d::functional_power_gradient` of
//! the order's amplitude), on 2D cells of 5.8 nm (two a pixel). Meep starts from uniform density ½; there the
//! order's amplitude is zero and so is its gradient, so we start from ½ plus a quarter of a sine
//! across the period. Values and gradients are rounded to 12 digits each evaluation, so that a
//! system's last bits can't steer the search. The design is binarized at ½ and measured on 2D
//! grids of 11.6, 5.8 and 2.9 nm, extrapolated as the Y-branch's 3D study is (second order, the
//! uncertainty spanning first order and the order the grids show).
//!
//! **Verify in 3D** (`--full`): the testbed's five designs (`device1.csv` to `device5.csv`; Fig.
//! 8(b), (c) and (d) are devices 2, 3 and 4, by their printed FDTD numbers) and ours, in 3D FDFD
//! on three grids of 23.2, 18.5 and 14.6 nm, extrapolated the same way. Each published design is
//! compared with the paper's Table 1 (the testbed's README for device 1, which the paper doesn't
//! show): within the two codes' own disagreement of the nearer one, the paper's cross-check
//! (|RCWA − FDTD|), our grids' spread beside it.
//!
//! **What it found.** The plain run's 2D numbers are its own. `--full` has not been run yet.
//!
//! ```sh
//! cargo run --release --example optimize_metagrating             # the 2D design and the stripes' 2D check
//! cargo run --release --example optimize_metagrating -- --full   # and the search again, and the 3D verification
//! cargo run --release --example optimize_metagrating -- --full --write   # the search alone, its design recorded
//! ```

mod common;

use photonoxide::portable;
use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{
    Axis, Boundaries, Boundaries3d, Edges, Field2d, Field3d, Grid, Grid3d, Polarization,
    Solver2d, Solver3d,
};
use photonoxide::topology::{Density, Edge, Projection, SmoothedProjection};
use photonoxide::units::Wavelength;

/// The wavelength, µm, the deflection angle, degrees, and the indices of silicon and silica.
const LAMBDA: f64 = 1.05;
const ANGLE: f64 = 50.0;
const N_SI: f64 = 3.45;
const N_SIO2: f64 = 1.45;
/// The period along y and the pattern's thickness, µm.
const PY: f64 = 0.525;
const THICKNESS: f64 = 0.325;
/// Our design's pixels across the period, the filter's radius, µm, and the β schedule.
const PIXELS: usize = 118;
const RADIUS: f64 = 0.06;
/// Meep's: tanh up to 32, the smoothed projection above (Meep's subpixel averaging there).
const BETAS: [f64; 9] = [
    4.0,
    8.0,
    16.0,
    32.0,
    64.0,
    96.0,
    128.0,
    256.0,
    f64::INFINITY,
];
const PER_STAGE: u64 = 80;
/// Meep's damping up to β = 32: Im ε = 0.05 ρ̂(1 − ρ̂) Re ε.
const DAMPING: f64 = 0.05;
/// The search's plane: cells a pixel along x (5.8 nm).
const OPT_SCALE: usize = 2;
/// The 2D grids: cells a pixel along x (2.9, 1.45 and 0.97 nm), whole cells to a node of the
/// stripes' 472 intervals.
const SCALES: [usize; 3] = [4, 8, 12];
/// The 3D grids: cells along x and y across the periods, and along z through the pattern
/// (23.2, 18.5 and 14.6 nm).
const GRIDS_3D: [(usize, usize, usize); 3] = [(59, 23, 14), (74, 28, 18), (94, 36, 22)];

/// The testbed's designs as its validation reads them, interpolated: device1 to device4 472 × 180
/// pixels (rows along x, each row's 180 values as 45 hex digits), device5 473 values along x on
/// the nodes of 472 intervals.
const DEVICES: [&str; 5] = [
    include_str!("../src/cases/topology/testbed/metagrating_device1_interpolated.hex"),
    include_str!("../src/cases/topology/testbed/metagrating_device2_interpolated.hex"),
    include_str!("../src/cases/topology/testbed/metagrating_device3_interpolated.hex"),
    include_str!("../src/cases/topology/testbed/metagrating_device4_interpolated.hex"),
    include_str!("../src/cases/topology/testbed/metagrating_device5_interpolated.csv"),
];
/// What each published design is in the paper, and its efficiencies there by RCWA and FDTD, %
/// (Fig. 8 and Table 1; device 1 from the testbed's README, which the paper doesn't show).
const PUBLISHED: [(&str, f64, f64); 5] = [
    ("device 1 (the testbed's README)", 95.7, 95.5),
    ("device 2, Fig. 8(b)", 93.2, 93.8),
    ("device 3, Fig. 8(c)", 96.8, 95.0),
    ("device 4, Fig. 8(d)", 93.7, 92.5),
    ("device 5, Fig. 8(e), stripes", 83.0, 84.3),
];
/// Chen et al.'s typical optimum for a 1d pattern, %.
const TYPICAL_1D: f64 = 80.0;
/// Our recorded design (`--full --write`): 118 pixels of 16 parts each, silicon 1 and air 0, as
/// hex digits, the first value the first digit's highest bit.
const OURS: &str = include_str!("designs/optimize_metagrating.hex");
const OURS_FILE: &str = "examples/designs/optimize_metagrating.hex";
const SUBPIXELS: usize = 16;

fn recorded() -> Vec<bool> {
    OURS.trim()
        .chars()
        .flat_map(|c| {
            let nibble = c.to_digit(16).expect("a hex digit");
            (0..4).map(move |q| nibble & (8 >> q) != 0)
        })
        .collect()
}

fn to_hex(design: &[bool]) -> String {
    design
        .chunks(4)
        .map(|c| {
            let n = c.iter().fold(0u32, |n, &b| n * 2 + u32::from(b));
            char::from_digit(n, 16).expect("a digit")
        })
        .collect()
}

/// What `--full` found in 3D: each design's efficiency on each grid (devices 1 to 5, then ours).
const RECORDED_3D: [[f64; 3]; 6] = [[f64::NAN; 3]; 6];

fn period() -> f64 {
    LAMBDA / portable::sin(ANGLE * std::f64::consts::PI / 180.0)
}

fn k0() -> f64 {
    2.0 * std::f64::consts::PI / LAMBDA
}

/// The diffraction order's wavenumber along x, rad/µm.
fn kx() -> f64 {
    2.0 * std::f64::consts::PI / period()
}

/// e^{−iθ}.
fn phase(theta: f64) -> c64 {
    c64::new(portable::cos(theta), -portable::sin(theta))
}

/// A design: silicon or air at (x, y) in the unit cell, µm.
enum Pattern {
    /// Pixels across both periods, `values[i * ny + j]` (rows along x).
    Pixels {
        nx: usize,
        ny: usize,
        values: Vec<bool>,
    },
    /// Stripes: values on the nodes of `values.len() − 1` intervals across Px, the first and the
    /// last the same point.
    Nodes(Vec<bool>),
    /// Stripes: pixels across Px.
    Stripes(Vec<bool>),
}

impl Pattern {
    /// A design of [`DEVICES`]: stripes from a column of values, pixels from rows of hex digits.
    fn published(text: &str) -> Pattern {
        if text.contains(',') || text.lines().all(|l| l.trim().len() <= 1) {
            return Pattern::from_csv(text);
        }
        let rows: Vec<Vec<bool>> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|l| {
                l.chars()
                    .flat_map(|c| {
                        let nibble = c.to_digit(16).expect("a hex digit");
                        (0..4).map(move |q| nibble & (8 >> q) != 0)
                    })
                    .collect()
            })
            .collect();
        Pattern::Pixels {
            nx: rows.len(),
            ny: rows[0].len(),
            values: rows.concat(),
        }
    }

    fn from_csv(text: &str) -> Pattern {
        let rows: Vec<Vec<bool>> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|l| {
                l.split(',')
                    .map(|v| v.trim().parse::<f64>().expect("a number") > 0.5)
                    .collect()
            })
            .collect();
        if rows.iter().all(|r| r.len() == 1) {
            Pattern::Nodes(rows.into_iter().map(|r| r[0]).collect())
        } else {
            Pattern::Pixels {
                nx: rows.len(),
                ny: rows[0].len(),
                values: rows.concat(),
            }
        }
    }

    fn silicon(&self, x: f64, y: f64) -> bool {
        let (px, u) = (period(), |x: f64, p: f64| (x / p).rem_euclid(1.0));
        match self {
            Pattern::Pixels { nx, ny, values } => {
                let i = ((u(x, px) * *nx as f64) as usize).min(nx - 1);
                let j = ((u(y, PY) * *ny as f64) as usize).min(ny - 1);
                values[i * ny + j]
            }
            // node i's value from x_i on, a rigid shift of half an interval, which a periodic
            // grating doesn't see; whole cells hold one node each
            Pattern::Nodes(v) => {
                let n = v.len() - 1;
                v[((u(x, px) * n as f64) as usize).min(n - 1)]
            }
            Pattern::Stripes(v) => {
                let i = ((u(x, px) * v.len() as f64) as usize).min(v.len() - 1);
                v[i]
            }
        }
    }
}

/// x rounded to 12 significant digits.
fn round12(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let scale = portable::powi(10f64, 11 - portable::log10(x.abs()).floor() as i32);
    if !scale.is_finite() {
        return x;
    }
    (x * scale).round() / scale
}

/// A 2D problem's rows: the PMLs, the substrate, the pattern and the air; the source's row, the
/// reference's and the monitor's.
struct Plane {
    grid: Grid,
    pml: usize,
    pattern: std::ops::Range<usize>,
    source: usize,
    reference: usize,
    monitor: usize,
}

impl Plane {
    /// The plane with `s` cells a pixel along x, the pattern 28 s rows.
    fn new(s: usize) -> Plane {
        let (nx, dy) = (PIXELS * s, THICKNESS / (28 * s) as f64);
        let (pml, below, rows, above) = (30 * s, 26 * s, 28 * s, 34 * s);
        Plane {
            grid: Grid {
                nx,
                ny: 2 * pml + below + rows + above,
                dx: period() / nx as f64,
                dy,
                x0: 0.0,
                y0: 0.0,
            },
            pml,
            pattern: pml + below..pml + below + rows,
            source: pml + 8 * s,
            reference: pml + 16 * s,
            monitor: pml + below + rows + 13 * s,
        }
    }

    fn boundaries(&self) -> Boundaries {
        Boundaries {
            x: Edges::Bloch { k: 0.0 },
            y: Edges::Pml {
                low: self.pml,
                high: self.pml,
            },
            ..Boundaries::pml(self.pml)
        }
    }

    /// The cells with `columns` (ε per column) in the pattern's rows; silica alone if `None`.
    fn cells(&self, columns: Option<&[f64]>) -> Vec<c64> {
        let g = self.grid;
        (0..g.nx * g.ny)
            .map(|p| {
                let (i, j) = (p % g.nx, p / g.nx);
                let eps = match columns {
                    None => N_SIO2 * N_SIO2,
                    Some(c) if self.pattern.contains(&j) => c[i],
                    Some(_) if j < self.pattern.start => N_SIO2 * N_SIO2,
                    Some(_) => 1.0,
                };
                c64::new(eps, 0.0)
            })
            .collect()
    }

    fn solver(&self, cells: &[c64]) -> photonoxide::Result<Solver2d> {
        Solver2d::from_cells(
            self.grid,
            Polarization::Hz,
            Wavelength::um(LAMBDA)?,
            cells,
            self.boundaries(),
        )
    }

    /// The source: a sheet of M_z on the source's row.
    fn source(&self) -> Vec<c64> {
        let g = self.grid;
        let mut m = vec![c64::new(0.0, 0.0); g.nx * g.ny];
        for i in 0..g.nx {
            m[self.source * g.nx + i] = c64::new(1.0 / g.dy, 0.0);
        }
        m
    }

    /// The power the sheet sends up through silica alone.
    fn incident(&self) -> photonoxide::Result<f64> {
        let s = self.solver(&self.cells(None))?;
        Ok(s.solve(&self.source())?.flux_y(self.reference))
    }

    /// The +1 order's coefficients on the monitor's row: its amplitude a = cᵀu.
    fn order(&self) -> Vec<c64> {
        let g = self.grid;
        let mut c = vec![c64::new(0.0, 0.0); g.nx * g.ny];
        for i in 0..g.nx {
            c[self.monitor * g.nx + i] = phase(kx() * g.x(i)) / g.nx as f64;
        }
        c
    }

    /// The +1 order's amplitude on the monitor's row and its power through the face above it:
    /// `Field2d::flux_y`'s scheme on the order's Fourier components of the two rows.
    fn order_power(&self, field: &Field2d) -> (c64, f64) {
        self.order_power_at(field, 1.0, self.monitor)
    }

    /// Order `m`'s amplitude on row `j` and its power through the face above it.
    fn order_power_at(&self, field: &Field2d, m: f64, j: usize) -> (c64, f64) {
        let monitor = j;
        let g = self.grid;
        let amplitude = |j: usize| -> c64 {
            (0..g.nx)
                .map(|i| field.at(i, j) * phase(m * kx() * g.x(i)))
                .sum::<c64>()
                / g.nx as f64
        };
        let (a, b) = (amplitude(monitor), amplitude(monitor + 1));
        let ex = -(b - a) / (g.dy * c64::new(0.0, k0()));
        let power = -0.5 * (ex * (0.5 * (a + b)).conj()).re * g.nx as f64 * g.dx;
        (a, power)
    }

    /// The efficiency of a stripe pattern on this plane.
    fn efficiency(&self, pattern: &Pattern, incident: f64) -> photonoxide::Result<f64> {
        let g = self.grid;
        // each column's ε the mean of 16 points across it
        let columns: Vec<f64> = (0..g.nx)
            .map(|i| {
                (0..16)
                    .map(|q| {
                        let x = g.x0 + (i as f64 + (q as f64 + 0.5) / 16.0) * g.dx;
                        if pattern.silicon(x, 0.0) {
                            N_SI * N_SI
                        } else {
                            1.0
                        }
                    })
                    .sum::<f64>()
                    / 16.0
            })
            .collect();
        let s = self.solver(&self.cells(Some(&columns)))?;
        let field = s.solve(&self.source())?;
        Ok(self.order_power(&field).1 / incident)
    }
}

/// The 2D search on the plane of one cell a pixel.
struct Search {
    plane: Plane,
    solver: Solver2d,
    incident: f64,
    source: Vec<c64>,
    c: Vec<c64>,
    /// The filter's reach in pixels: the pattern is wrapped by as many on each side.
    reach: usize,
}

impl Search {
    fn new() -> photonoxide::Result<Search> {
        let plane = Plane::new(OPT_SCALE);
        let solver = plane.solver(&plane.cells(Some(&[1.0; PIXELS * OPT_SCALE])))?;
        let incident = plane.incident()?;
        let (source, c) = (plane.source(), plane.order());
        let reach = (RADIUS * PIXELS as f64 / period()).ceil() as usize;
        Ok(Search {
            plane,
            solver,
            incident,
            source,
            c,
            reach,
        })
    }

    fn density(&self, beta: f64) -> photonoxide::Result<Density> {
        let pixel = period() / PIXELS as f64;
        let d = Density::new([PIXELS + 2 * self.reach, 1, 1], pixel, [1.0, N_SI * N_SI])?
            .with_filter(RADIUS, Edge::Renormalized)?;
        Ok(if beta <= 32.0 {
            d.with_projection(Projection::new(beta, 0.5)?)
        } else {
            d.with_smoothed_projection(SmoothedProjection::new(beta, 0.5)?)
        })
    }

    /// The densities wrapped by the filter's reach on each side.
    fn wrapped(&self, rho: &[f64]) -> Vec<f64> {
        (0..PIXELS + 2 * self.reach)
            .map(|e| rho[(e + PIXELS - self.reach) % PIXELS])
            .collect()
    }

    /// The efficiency and its gradient in the densities, rounded to 12 digits; below β = 32 the
    /// intermediate densities are lossy, Im ε = 0.05 ρ̂(1 − ρ̂) Re ε (Meep's damping there).
    fn evaluate(
        &self,
        density: &Density,
        beta: f64,
        rho: &[f64],
    ) -> photonoxide::Result<(f64, Vec<f64>)> {
        let wrapped = self.wrapped(rho);
        let physical = density.physical(&wrapped)?;
        let damping = if beta <= 32.0 { DAMPING } else { 0.0 };
        let (e0, de) = (1.0, N_SI * N_SI - 1.0);
        let eps = |u: f64| {
            let re = e0 + u * de;
            c64::new(re, damping * u * (1.0 - u) * re)
        };
        let nx = self.plane.grid.nx;
        let mut cells = self.plane.cells(Some(&vec![1.0; nx]));
        for j in self.plane.pattern.clone() {
            for i in 0..nx {
                cells[j * nx + i] = eps(physical[self.reach + i / OPT_SCALE]);
            }
        }
        let s = self.solver.reuse_cells(Wavelength::um(LAMBDA)?, &cells)?;
        let field = s.solve(&self.source)?;
        let (a, power) = self.plane.order_power(&field);
        let (_, da) = s.functional_gradient(&field, &self.c)?;
        // the order's power is κ|a|², κ fixed by the air and the grid; |a|²'s derivatives in
        // Re ε and Im ε are 2 Re(a* da) and 2 Re(a* i da)
        let scale = power / a.norm_sqr() / self.incident;
        let mut d_physical = vec![0.0; PIXELS + 2 * self.reach];
        for j in self.plane.pattern.clone() {
            for i in 0..nx {
                let p = self.reach + i / OPT_SCALE;
                let u = physical[p];
                let d = da[j * nx + i];
                let d_re = 2.0 * (a.conj() * d).re;
                let d_im = 2.0 * (a.conj() * c64::new(0.0, 1.0) * d).re;
                let re = e0 + u * de;
                let d_im_du = damping * ((1.0 - 2.0 * u) * re + u * (1.0 - u) * de);
                d_physical[p] += scale * (d_re * de + d_im * d_im_du);
            }
        }
        // the pullback takes dF/dε of the real interpolation: dF/dρ̂ over its slope
        let d_eps: Vec<f64> = d_physical.iter().map(|g| g / de).collect();
        let wrapped_gradient = density.gradient(&wrapped, &d_eps)?;
        let mut gradient = vec![0.0; PIXELS];
        for (e, g) in wrapped_gradient.iter().enumerate() {
            gradient[(e + PIXELS - self.reach) % PIXELS] += g;
        }
        Ok((
            round12(power / self.incident),
            gradient.into_iter().map(round12).collect(),
        ))
    }

    /// β from 4 to 256, MMA maximizing the efficiency: the design, binarized at ½.
    fn optimize(&self) -> photonoxide::Result<Vec<bool>> {
        use genoxide::prelude::*;
        let mut rho: Vec<f64> = (0..PIXELS)
            .map(|i| {
                0.5 + 0.25
                    * portable::sin(2.0 * std::f64::consts::PI * (i as f64 + 0.5) / PIXELS as f64)
            })
            .collect();
        let start = Instant::now();
        for beta in BETAS {
            let density = self.density(beta)?;
            let fitness = |x: &Reals, gradient: &mut [f64]| -> f64 {
                let Ok((eta, g)) = self.evaluate(&density, beta, x) else {
                    return f64::NAN;
                };
                for (a, b) in gradient.iter_mut().zip(&g) {
                    *a = -b;
                }
                -eta
            };
            let mma = Mma::builder(Real::new(vec![0.0..=1.0; PIXELS]).expect("bounds"))
                .initial_genome(Reals::from(rho.clone()))
                .minimize()
                .build()
                .expect("an MMA");
            let outcome = Engine::new(mma, Differentiable(&fitness))
                .stop_when(Stop::evaluations(PER_STAGE))
                .run()
                .expect("a run");
            rho = outcome.best_genome().to_vec();
            let (eta, _) = self.evaluate(&density, beta, &rho)?;
            println!(
                "    β {beta:>3}: {:.2} % ({:.0} s)",
                100.0 * eta,
                start.elapsed().as_secs_f64()
            );
        }
        // at β = ∞ a pixel's fill is its edge's place within it (Hammond et al. 2025): the
        // pattern kept at a 16th of a pixel, each part-filled pixel filled from its more solid
        // neighbour's side
        let density = self.density(BETAS[BETAS.len() - 1])?;
        let physical = density.physical(&self.wrapped(&rho))?;
        let u: Vec<f64> = physical[self.reach..self.reach + PIXELS].to_vec();
        let mut fine = Vec::with_capacity(PIXELS * SUBPIXELS);
        for (i, &f) in u.iter().enumerate() {
            let (left, right) = (u[(i + PIXELS - 1) % PIXELS], u[(i + 1) % PIXELS]);
            let filled = (f * SUBPIXELS as f64).round() as usize;
            for q in 0..SUBPIXELS {
                fine.push(if left >= right {
                    q < filled
                } else {
                    q >= SUBPIXELS - filled
                });
            }
        }
        Ok(fine)
    }
}

/// The shortest run of silicon and of air across the period, in pixels (the pattern periodic).
fn shortest_runs(v: &[bool]) -> (usize, usize) {
    let n = v.len();
    let Some(first) = (0..n).find(|&i| v[i] != v[(i + n - 1) % n]) else {
        return (n, n);
    };
    let (mut best, mut run) = ([usize::MAX; 2], 0);
    for k in 0..n {
        let i = (first + k) % n;
        run += 1;
        if v[(i + 1) % n] != v[i] {
            let s = usize::from(v[i]);
            best[s] = best[s].min(run);
            run = 0;
        }
    }
    (best[1], best[0])
}

/// Values on three grids of step `h` extrapolated at second order, the uncertainty spanning the
/// extrapolations at first order and at the order the grids show (if they show one).
fn extrapolated(h: [f64; 3], p: [f64; 3]) -> (f64, f64, Option<f64>) {
    let pow = portable::powf;
    let at_order = |q: f64| p[2] + (p[2] - p[1]) / (pow(h[1] / h[2], q) - 1.0);
    let second = at_order(2.0);
    let ratio = (p[1] - p[0]) / (p[2] - p[1]);
    let f = |q: f64| (pow(h[0], q) - pow(h[1], q)) / (pow(h[1], q) - pow(h[2], q)) - ratio;
    let (mut lo, mut hi) = (0.1, 10.0);
    let order = (f(lo) * f(hi) < 0.0).then(|| {
        for _ in 0..100 {
            let mid = 0.5 * (lo + hi);
            if f(lo) * f(mid) <= 0.0 {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        0.5 * (lo + hi)
    });
    let mut spread = (at_order(1.0) - second).abs();
    if let Some(q) = order {
        spread = spread.max((at_order(q) - second).abs());
    }
    (second, spread, order)
}

/// A 3D problem on one of [`GRIDS_3D`].
struct Cell3d {
    grid: Grid3d,
    pml: usize,
    /// The pattern's lowest z, µm, and the source's, the reference's and the monitor's planes.
    bottom: f64,
    source: usize,
    reference: usize,
    monitor: usize,
}

impl Cell3d {
    fn new((nx, ny, rows): (usize, usize, usize)) -> Cell3d {
        let dz = THICKNESS / rows as f64;
        let cells = |um: f64| (um / dz).round() as usize;
        let (pml, below, above) = (cells(0.28), cells(0.3), cells(0.4));
        Cell3d {
            grid: Grid3d {
                nx,
                ny,
                nz: 2 * pml + below + rows + above,
                dx: period() / nx as f64,
                dy: PY / ny as f64,
                dz,
                x0: 0.0,
                y0: 0.0,
                z0: 0.0,
            },
            pml,
            bottom: (pml + below) as f64 * dz,
            source: pml + cells(0.1),
            reference: pml + cells(0.2),
            monitor: pml + below + rows + cells(0.15),
        }
    }

    fn boundaries(&self) -> Boundaries3d {
        Boundaries3d {
            x: Edges::Bloch { k: 0.0 },
            y: Edges::Bloch { k: 0.0 },
            ..Boundaries3d::pml(self.pml)
        }
    }

    /// ε at (x, y, z) with `pattern` in the pattern's layer; silica alone if `None`.
    fn eps(&self, pattern: Option<&Pattern>, x: f64, y: f64, z: f64) -> c64 {
        let e = match pattern {
            None => N_SIO2 * N_SIO2,
            Some(_) if z < self.bottom => N_SIO2 * N_SIO2,
            Some(p) if z < self.bottom + THICKNESS && p.silicon(x, y) => N_SI * N_SI,
            Some(_) => 1.0,
        };
        c64::new(e, 0.0)
    }

    /// The source: a sheet of J_x on the source's plane.
    fn source(&self) -> Vec<c64> {
        let g = self.grid;
        let mut j = vec![c64::new(0.0, 0.0); g.unknowns()];
        for jy in 0..g.ny {
            for ix in 0..g.nx {
                j[g.index(Axis::X, (ix, jy, self.source))] = c64::new(1.0 / g.dz, 0.0);
            }
        }
        j
    }

    /// The (+1, 0) order's power through the plane between the monitor's nodes and the next:
    /// [`Field3d::flux`]'s scheme on the order's Fourier components, each component's sum at its
    /// own positions.
    fn order_power(&self, field: &Field3d) -> f64 {
        let (g, p) = (self.grid, self.monitor);
        let n = (g.nx * g.ny) as f64;
        let fourier = |e: bool, axis: Axis| -> c64 {
            let mut sum = c64::new(0.0, 0.0);
            for j in 0..g.ny {
                for i in 0..g.nx {
                    let (value, x) = if e {
                        (
                            0.5 * (field.e(axis, (i, j, p)) + field.e(axis, (i, j, p + 1))),
                            g.e_position(axis, (i, j, p))[0],
                        )
                    } else {
                        (field.h(axis, (i, j, p)), g.h_position(axis, (i, j, p))[0])
                    };
                    sum += value * phase(kx() * x);
                }
            }
            sum / n
        };
        let (ex, ey) = (fourier(true, Axis::X), fourier(true, Axis::Y));
        let (hx, hy) = (fourier(false, Axis::X), fourier(false, Axis::Y));
        0.5 * (ex * hy.conj() - ey * hx.conj()).re * n * g.dx * g.dy
    }
}

/// The 3D runs: every published design and ours on each grid, the efficiencies by design then grid.
fn three_d(ours: &Pattern, log: &mut std::fs::File) -> photonoxide::Result<Vec<[f64; 3]>> {
    let patterns: Vec<Pattern> = DEVICES
        .iter()
        .map(|t| Pattern::published(t))
        .chain(std::iter::once(match ours {
            Pattern::Stripes(v) => Pattern::Stripes(v.clone()),
            _ => unreachable!("our design is stripes"),
        }))
        .collect();
    let started = Instant::now();
    // each step a factorization, weighted by its grid's unknowns^1.5 for the time left
    let weight = |g: &(usize, usize, usize)| {
        let c = Cell3d::new(*g);
        let u = c.grid.unknowns() as f64;
        u * portable::powf(u, 0.5)
    };
    let per_grid = 1 + patterns.len();
    let total: f64 = GRIDS_3D.iter().map(|g| weight(g) * per_grid as f64).sum();
    let steps = GRIDS_3D.len() * per_grid;
    let (mut done, mut step) = (0.0, 0);
    let mut note = |line: String| {
        eprintln!("{line}");
        let _ = writeln!(log, "{line}");
    };
    let mut out = vec![[f64::NAN; 3]; patterns.len()];
    for (k, spec) in GRIDS_3D.iter().enumerate() {
        let cell = Cell3d::new(*spec);
        let lam = Wavelength::um(LAMBDA)?;
        let source = cell.source();
        let mut progress = |what: &str, note: &mut dyn FnMut(String)| {
            step += 1;
            done += weight(spec);
            let elapsed = started.elapsed().as_secs_f64() / 60.0;
            note(format!(
                "step {step}/{steps} ({:.0} %): grid {} of {} ({} × {} × {} cells, {:.1} nm), {what}; {elapsed:.1} min in, about {:.0} min left",
                100.0 * done / total,
                k + 1,
                GRIDS_3D.len(),
                cell.grid.nx,
                cell.grid.ny,
                cell.grid.nz,
                1000.0 * cell.grid.dx,
                elapsed * (total - done) / done
            ));
        };
        let reference = Solver3d::new(
            cell.grid,
            lam,
            |x, y, z| cell.eps(None, x, y, z),
            cell.boundaries(),
        )?;
        let incident = reference.solve(&source)?.flux(Axis::Z, cell.reference);
        progress(&format!("incident {incident:.6e}"), &mut note);
        for (d, pattern) in patterns.iter().enumerate() {
            let s = reference.reuse(lam, |x, y, z| cell.eps(Some(pattern), x, y, z))?;
            let eta = cell.order_power(&s.solve(&source)?) / incident;
            out[d][k] = eta;
            progress(
                &format!("design {} of {}: {:.4} %", d + 1, patterns.len(), 100.0 * eta),
                &mut note,
            );
        }
    }
    Ok(out)
}

/// The 3D results against the paper's.
fn compare_3d(checks: &mut common::Checks, results: &[[f64; 3]]) {
    let h = GRIDS_3D.map(|g| period() / g.0 as f64);
    let names = PUBLISHED
        .iter()
        .map(|p| p.0)
        .chain(std::iter::once("ours, stripes (2D design)"));
    for (d, (name, p)) in names.zip(results).enumerate() {
        let (eta, spread, order) = extrapolated(h, p.map(|v| 100.0 * v));
        let order = order.map_or("none".to_string(), |q| format!("{q:.1}"));
        println!(
            "  {name}: {:.2}, {:.2}, {:.2} % on the three grids, {eta:.2} ± {spread:.2} % extrapolated (order {order})",
            100.0 * p[0],
            100.0 * p[1],
            100.0 * p[2]
        );
        if let Some(&(_, rcwa, fdtd)) = PUBLISHED.get(d) {
            let d = (rcwa - fdtd).abs();
            let (lo, hi) = (rcwa.min(fdtd), rcwa.max(fdtd));
            println!(
                "    paper: {rcwa} % (RCWA), {fdtd} % (FDTD); within {d:.1} of the nearer: {:.1} to {:.1} %",
                lo - d,
                hi + d
            );
            checks.against(
                &format!("{name}, 3D (%)"),
                eta,
                "the codes' range, centre",
                0.5 * (lo + hi),
                0.5 * (hi - lo) + d,
            );
        }
    }
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let full = std::env::args().any(|a| a == "--full");
    let write = std::env::args().any(|a| a == "--write");
    let mut checks = common::Checks::default();
    let mut setup = common::Setup::new();
    setup.item(
        "unit cell",
        &format!(
            "{:.4} × {PY} µm (λ/sin 50° by λ/2), a {THICKNESS} µm pattern of silicon and air on silica, air above",
            period()
        ),
    );
    setup.medium("silicon", &common::n(N_SI));
    setup.medium("silica", &common::n(N_SIO2));
    setup.item(
        "light",
        "1.05 µm at normal incidence from the substrate, E along x (p); the (+1, 0) order in air, 50° from the normal",
    );
    setup.item(
        "2D",
        "the x–z plane, H along y (Hz), Bloch-periodic along x, PMLs of 0.35 µm above and below; cells of 11.6, 5.8 and 2.9 nm",
    );
    setup.item(
        "our design",
        "118 pixels of stripes, a periodic conic filter of 60 nm, tanh projection at η = ½, β 4 to 256 in seven stages of 40 evaluations, genoxide's MMA on the adjoint gradient",
    );
    setup.item(
        "3D (--full)",
        "Bloch-periodic along x and y, PMLs of 0.28 µm along z; cells of 23.2, 18.5 and 14.6 nm",
    );
    setup.item(
        "tolerance",
        "each published design within the two codes' disagreement (|RCWA − FDTD|) of the nearer; ours reported beside Chen et al.'s 80 % for stripes",
    );
    setup.print();



    let h = SCALES.map(|s| period() / (PIXELS * s) as f64);
    let planes: Vec<Plane> = SCALES.iter().map(|&s| Plane::new(s)).collect();
    let incident: Vec<f64> = planes
        .iter()
        .map(Plane::incident)
        .collect::<photonoxide::Result<_>>()?;

    println!("Fig. 8(e), Meep's stripes (device5.csv), in 2D, which is the device exactly:");
    let stripes = Pattern::published(DEVICES[4]);
    let mut p = [0.0; 3];
    for (k, plane) in planes.iter().enumerate() {
        p[k] = 100.0 * plane.efficiency(&stripes, incident[k])?;
    }
    let (eta, spread, order) = extrapolated(h, p);
    println!(
        "  {:.3}, {:.3}, {:.3} % on the three grids, {eta:.2} ± {spread:.2} % extrapolated (order {})",
        p[0],
        p[1],
        p[2],
        order.map_or("none".to_string(), |q| format!("{q:.1}"))
    );
    let (_, rcwa, fdtd) = PUBLISHED[4];
    let d = (rcwa - fdtd).abs();
    checks.against(
        "Fig. 8(e), 2D (%)",
        eta,
        "the codes' range, centre",
        0.5 * (rcwa + fdtd),
        0.5 * d + d,
    );

    let ours = if full {
        println!("Our stripes, optimized in 2D on cells of 5.8 nm:");
        let design = Search::new()?.optimize()?;
        if write {
            std::fs::write(OURS_FILE, to_hex(&design) + "
").expect("the design written");
            println!("  written to {OURS_FILE}");
        } else {
            let differing = design.iter().zip(recorded()).filter(|(a, b)| **a != *b).count();
            checks.count("pixels unlike the recorded design", differing, 0);
        }
        design
    } else {
        recorded()
    };
    let (si, air) = shortest_runs(&ours);
    let part_nm = 1000.0 * period() / ours.len() as f64;
    println!(
        "Our stripes (recorded, edges to a 16th of a pixel): the shortest silicon {:.1} nm and air {:.1} nm",
        si as f64 * part_nm,
        air as f64 * part_nm
    );
    let ours = Pattern::Stripes(ours);
    for (k, plane) in planes.iter().enumerate() {
        p[k] = 100.0 * plane.efficiency(&ours, incident[k])?;
    }
    let (eta, spread, order) = extrapolated(h, p);
    println!(
        "  {:.3}, {:.3}, {:.3} % on the three grids, {eta:.2} ± {spread:.2} % extrapolated (order {})",
        p[0],
        p[1],
        p[2],
        order.map_or("none".to_string(), |q| format!("{q:.1}"))
    );
    // a gap kept open (#595): reported, not checked
    println!(
        "  against Chen et al.'s typical {TYPICAL_1D} % for stripes and Meep's {} %: {:+.1} and {:+.1} points, a gap listed in #595",
        PUBLISHED[4].2,
        eta - TYPICAL_1D,
        eta - PUBLISHED[4].2
    );

    println!("3D FDFD, the published designs and ours:");
    if full && !write {
        let path =
            std::env::var("METAGRATING_LOG").unwrap_or_else(|_| "optimize_metagrating.log".into());
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .expect("a log file");
        eprintln!("progress in {path}");
        let results = three_d(&ours, &mut log)?;
        println!("  recorded: {results:?}");
        compare_3d(&mut checks, &results);
    } else if RECORDED_3D.iter().flatten().all(|v| v.is_finite()) {
        println!("  what --full found (recorded):");
        compare_3d(&mut checks, &RECORDED_3D);
    } else {
        println!("  not recorded yet: run --full");
    }
    Ok(checks.finish())
}
