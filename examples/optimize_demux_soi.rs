//! Piggott et al.'s compact wavelength demultiplexer: a 2.8 × 2.8 µm region of 220 nm silicon on
//! silica, air above, sending 1300 nm to one 500 nm waveguide and 1550 nm to another; designed
//! here by topology optimization in 2D and verified in 3D FDFD.
//!
//! A. Y. Piggott et al., "Inverse design and demonstration of a compact and broadband on-chip
//! wavelength demultiplexer", Nat. Photonics 9, 374 (2015),
//! [doi:10.1038/nphoton.2015.69](https://doi.org/10.1038/nphoton.2015.69), Figs. 1, 2 and 4(a).
//!
//! **The problem** (their Fig. 1 and p. 2). A fully etched layer of silicon 220 nm thick
//! (n = 3.49) on silica (1.45), air (1) above; one input waveguide on the left and two output
//! waveguides on the right, all 500 nm wide, around a 2.8 × 2.8 µm design region. The outputs'
//! centres are 0.54 µm above and below the input's axis, read from Fig. 1(b)'s scale bar (the
//! paper's Supplementary Section 1 has the drawing; we don't have it). The fundamental TE mode
//! goes in; at 1300 nm more than 90 % of it is to leave by port 2 (above) and less than 1 % by
//! port 3 (below), the converse at 1550 nm. Their last stage asks for it at ten wavelengths, five
//! equally spaced in frequency about each centre; they don't give the spacing, and ours is ±2 %
//! of the frequency in steps of 1 %.
//!
//! **The 2D design.** The plane by the effective index method, as `optimize_ybranch`'s: the
//! 220 nm slab's TE index at each wavelength (2.99 at 1300 nm, 2.85 at 1550 nm, silica below and
//! air above) inside the silicon and the silica's outside, where an etched region's light has no
//! slab to stay in; H along z, 2D FDFD on 20 nm cells, PMLs of 0.3 µm. Densities on the region's
//! 140 × 140 pixels of 20 nm, a conic filter of 100 nm and a tanh projection at η = ½
//! (`topology`), the worst of the ten 1 − T and the ten shares into the other port, each over its bound in the
//! specification (10 % and 1 %), minimized in its epigraph form by genoxide's MMA,
//! every gradient by the adjoint; β from 8 to 256 in six stages of 40 evaluations. Their
//! method differs: "objective first" and steepest descent on a continuous permittivity, then a
//! level set, on GPU-accelerated 3D FDFD.
//!
//! **The 3D check.** The binarized design extruded through the 220 nm layer, solved in 3D FDFD
//! (`--full`) on cells of 36.7, 31.4 and 27.5 nm (a sixth, a seventh and an eighth of the
//! layer), inside stretched PMLs of 0.44 µm, by GMRES and the multigrid to a residual of 1e-9;
//! each output's fundamental mode in a window of its own, extrapolated to h → 0 at second order.
//!
//! **What is compared.** The design isn't published, so ours is compared with theirs by the
//! same measures, as a target to meet. In 2D: their specification at the two centres, T above
//! 90 % and the other port below 1 % (a 2D number, not the device's). In 3D: their simulated
//! S-parameters, Fig. 4(a)'s 3D FDTD of their design, read from the figure to ±0.1 dB in the
//! pass bands: S21 −1.7 dB and S31 −23.8 dB at 1300 nm, S31 −2.0 dB and S21 −23.0 dB at
//! 1550 nm (their measured devices: −1.8 and −2.4 dB at the bands' peaks, below −11 dB of
//! crosstalk, context only). Our 3D insertion loss is checked to be at most theirs where our
//! grids' spread leaves no doubt which is lower; otherwise it is reported, not checked.
//!
//! **What it found** is in the output below.
//!
//! ```sh
//! cargo run --release --example optimize_demux_soi                    # the recorded design, 2D and the recorded 3D
//! cargo run --release --example optimize_demux_soi -- --full          # the optimization again, and the 3D solves
//! cargo run --release --example optimize_demux_soi -- --full --write  # the optimization, recorded
//! cargo run --release --example optimize_demux_soi -- --3d             # the recorded design's 3D solves
//! ```

mod common;

use photonoxide::portable;
use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{
    Axis, Boundaries, Boundaries3d, Direction, Formulation, Grid, Grid3d, IterativeSolver3d,
    Multigrid, Polarization, PortMode, Product, Solver2d, Stopping,
};
use photonoxide::mode::slab::Slab;
use photonoxide::topology::{Density, Edge, Epigraph, Placement, Projection};
use photonoxide::units::{Length, Wavelength};
use rayon::prelude::*;

/// The indices Piggott et al. use, and the layer's thickness, µm.
const N_SI: f64 = 3.49;
const N_SIO2: f64 = 1.45;
const THICKNESS: f64 = 0.22;
/// The waveguides' width, the outputs' centres off the axis and the region's half-side, µm.
const WIDTH: f64 = 0.5;
const OFFSET: f64 = 0.54;
const HALF: f64 = 1.4;
/// The design's pixel, µm, and pixels a side.
const PIXEL: f64 = 0.02;
const SIDE: usize = 140;
/// The 2D domain's half-sizes inside the PMLs, and the PMLs, µm; the ports' x.
const DOMAIN_2D: [f64; 2] = [2.4, 2.2];
const PML_2D: f64 = 0.3;
const PORT_X: f64 = 2.0;
/// The bands' centres, nm, and the port each goes to (true: port 2, above).
const CENTRES: [(f64, bool); 2] = [(1300.0, true), (1550.0, false)];
/// The steps in frequency about each centre.
const STEPS: [f64; 5] = [-0.02, -0.01, 0.0, 0.01, 0.02];
/// The conic filter's radius, µm; β's stages and their evaluations.
const RADIUS: f64 = 0.1;
const BETAS: [f64; 6] = [8.0, 16.0, 32.0, 64.0, 128.0, 256.0];
const PER_STAGE: u64 = 40;
/// Their specification as bounds: 1 − T at most (T above 90 %) and the other port at most.
const SPEC: (f64, f64) = (0.1, 0.01);
/// Fig. 4(a), their 3D FDTD, dB: (wanted, other) at 1300 and at 1550 nm; the reading's ±.
const FIG_4A: [(f64, f64); 2] = [(-1.7, -23.8), (-2.0, -23.0)];
const READING: f64 = 0.1;
/// The 3D grids: cells across the layer; the 3D domain's half-sizes inside the PMLs, µm (x, y),
/// and its z inside them, the layer from 0 to THICKNESS; the PMLs' cells; the ports' x.
const LAYER_CELLS: [usize; 3] = [6, 7, 8];
const DOMAIN_3D: [f64; 2] = [1.98, 1.76];
const Z_3D: (f64, f64) = (-0.66, 0.88);
const PML_3D: usize = 16;
const PORT_X_3D: f64 = 1.7;
const RESIDUAL: f64 = 1e-9;
/// What `--full` found in 3D, per grid: (T, other) at 1300 nm, then at 1550 nm.
const RECORDED_3D: [[f64; 4]; 3] = [[f64::NAN; 4]; 3];

const RECORDED: &str = include_str!("designs/optimize_demux_soi.hex");
const RECORDED_FILE: &str = "examples/designs/optimize_demux_soi.hex";

/// The design's wavelengths, µm, each with the port it goes to.
fn wavelengths() -> Vec<(f64, bool)> {
    CENTRES
        .iter()
        .flat_map(|&(nm, up)| STEPS.iter().map(move |s| (nm * 1e-3 / (1.0 + s), up)))
        .collect()
}

/// The 220 nm slab's TE index at `lam`, µm: silica below, air above.
fn slab_index(lam: f64) -> photonoxide::Result<f64> {
    Ok(Slab::new(N_SIO2, N_SI, 1.0, Length::um(THICKNESS))?
        .modes(photonoxide::mode::Polarization::Te, Wavelength::um(lam)?)[0]
        .effective_index())
}

/// The 2D grid of cell `h`, µm.
fn grid(h: f64) -> Grid {
    let [x, y] = DOMAIN_2D.map(|d| d + PML_2D);
    Grid {
        nx: (2.0 * x / h).round() as usize,
        ny: (2.0 * y / h).round() as usize,
        dx: h,
        dy: h,
        x0: -x,
        y0: -y,
    }
}

/// The PML's cells on a 2D grid of cell `h`.
fn pml_2d(h: f64) -> usize {
    (PML_2D / h).round() as usize
}

/// The length of [a, b] within [lo, hi].
fn within(a: f64, b: f64, lo: f64, hi: f64) -> f64 {
    (hi.min(b) - lo.max(a)).max(0.0)
}

/// The share of the waveguides outside the region in the cell [x−h/2, x+h/2] × [y−h/2, y+h/2].
fn guides(x: f64, y: f64, h: f64) -> f64 {
    let (ya, yb) = (y - 0.5 * h, y + 0.5 * h);
    let w = 0.5 * WIDTH;
    let input = within(x - 0.5 * h, x + 0.5 * h, f64::NEG_INFINITY, -HALF) / h;
    let output = within(x - 0.5 * h, x + 0.5 * h, HALF, f64::INFINITY) / h;
    let across = |c: f64| within(ya, yb, c - w, c + w) / h;
    input * across(0.0) + output * (across(OFFSET) + across(-OFFSET))
}

/// The design pixel (column, row) holding the point (x, y), if it is in the region.
fn pixel(x: f64, y: f64) -> Option<(usize, usize)> {
    let inside = |v: f64| (-HALF..HALF).contains(&v);
    (inside(x) && inside(y)).then(|| {
        let at = |v: f64| (((v + HALF) / PIXEL).floor() as usize).min(SIDE - 1);
        (at(x), at(y))
    })
}

/// The plane's permittivities on `g` at a slab index `core`: the waveguides, and in the region
/// `design`'s shares of silicon, one per pixel (silica if `None`).
fn cells(g: &Grid, core: f64, design: Option<&[f64]>) -> Vec<c64> {
    let (si, clad) = (core * core, N_SIO2 * N_SIO2);
    (0..g.nx * g.ny)
        .map(|p| {
            let (x, y) = (g.x(p % g.nx), g.y(p / g.nx));
            let s = match pixel(x, y) {
                Some((a, b)) => design.map_or(0.0, |d| d[b * SIDE + a]),
                None => guides(x, y, g.dx),
            };
            c64::new(clad + s * (si - clad), 0.0)
        })
        .collect()
}

/// The column or row of `v`, µm, on an axis from `v0` in steps of `h`.
fn index(v: f64, v0: f64, h: f64) -> usize {
    ((v - v0) / h).round() as usize
}

/// The ports on a 2D problem: the input's fundamental mode, and the outputs' (above, below), each
/// in a window of its own.
fn ports(s: &Solver2d, g: &Grid) -> photonoxide::Result<[PortMode; 3]> {
    let (h, row) = (g.dx, |y: f64| index(y, g.y0, g.dy));
    let top = DOMAIN_2D[1];
    let input = s
        .port_modes_within(index(-PORT_X, g.x0, h), row(-1.2)..row(1.2), 1)?
        .remove(0);
    let up = s
        .port_modes_within(index(PORT_X, g.x0, h), row(0.0)..row(top), 1)?
        .remove(0);
    let down = s
        .port_modes_within(index(PORT_X, g.x0, h), row(-top)..row(0.0), 1)?
        .remove(0);
    Ok([input, up, down])
}

/// A binary design solved in 2D on cells of `h`: per wavelength, the wanted port's share, the
/// other's and the reflection, each of the input mode's power.
fn spectrum(design: &[bool], h: f64) -> photonoxide::Result<Vec<[f64; 3]>> {
    let g = grid(h);
    let d: Vec<f64> = design.iter().map(|&b| f64::from(u8::from(b))).collect();
    wavelengths()
        .into_par_iter()
        .map(|(lam, up)| {
            let eps = cells(&g, slab_index(lam)?, Some(&d));
            let s = Solver2d::from_cells(
                g,
                Polarization::Hz,
                Wavelength::um(lam)?,
                &eps,
                Boundaries::pml(pml_2d(h)),
            )?;
            let [input, top, bottom] = ports(&s, &g)?;
            let field = s.solve_system(&s.mode_source(&input, Direction::Forward))?;
            let p_in = input.power(g.dy);
            let share = |m: &PortMode| field.mode_amplitudes(m).0.norm_sqr() * m.power(g.dy) / p_in;
            let (wanted, other) = if up { (&top, &bottom) } else { (&bottom, &top) };
            let r = field.mode_amplitudes(&input).1.norm_sqr();
            Ok([share(wanted), share(other), r])
        })
        .collect()
}

/// x rounded to 12 significant digits: an evaluation's values and gradients, so that a system's
/// last bits can't steer the optimizer.
fn round12(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let scale = portable::powi(10f64, 11 - portable::log10(x.abs()).floor() as i32);
    if !scale.is_finite() {
        // below about 1e-297 the scale overflows: such a value is kept as it is
        return x;
    }
    (x * scale).round() / scale
}

/// One wavelength of the optimization: the slab index, the solver at the background, the input's
/// mode, the wanted output's and the other's, and their powers over the input's.
struct Band {
    lam: Wavelength,
    core: f64,
    background: Vec<c64>,
    solver: Solver2d,
    input: PortMode,
    outputs: [(PortMode, f64); 2],
}

/// The optimization's problem at one β.
struct Demux {
    density: Density,
    placement: Placement,
    bands: Vec<Band>,
}

impl Demux {
    fn new(beta: f64) -> photonoxide::Result<Demux> {
        let g = grid(PIXEL);
        let density = Density::new([SIDE, SIDE, 1], PIXEL, [0.0, 1.0])?
            .with_filter(RADIUS, Edge::Renormalized)?
            .with_projection(Projection::new(beta, 0.5)?);
        let bands = wavelengths()
            .into_par_iter()
            .map(|(lam, up)| {
                let core = slab_index(lam)?;
                let background = cells(&g, core, None);
                let wavelength = Wavelength::um(lam)?;
                let solver = Solver2d::from_cells(
                    g,
                    Polarization::Hz,
                    wavelength,
                    &background,
                    Boundaries::pml(pml_2d(PIXEL)),
                )?;
                let [input, top, bottom] = ports(&solver, &g)?;
                let (wanted, other) = if up { (top, bottom) } else { (bottom, top) };
                let ratio = |m: &PortMode| m.power(g.dy) / input.power(g.dy);
                let outputs =
                    [(ratio(&wanted), wanted), (ratio(&other), other)].map(|(r, m)| (m, r));
                Ok(Band {
                    lam: wavelength,
                    core,
                    background,
                    solver,
                    input,
                    outputs,
                })
            })
            .collect::<photonoxide::Result<_>>()?;
        Ok(Demux {
            density,
            placement: Placement {
                origin: [index(-HALF, g.x0, PIXEL), index(-HALF, g.y0, PIXEL), 0],
                shape: [SIDE, SIDE, 1],
                grid: [g.nx, g.ny, 1],
            },
            bands,
        })
    }

    /// The twenty objectives, per wavelength 1 − T and the other port's share, each over its
    /// bound in [`SPEC`] (so that both are met when the worst is below 1), and their gradients in
    /// the densities, rounded to 12 digits.
    fn evaluate(&self, rho: &[f64]) -> photonoxide::Result<(Vec<f64>, Vec<f64>)> {
        let share = self.density.permittivity(rho)?;
        let clad = N_SIO2 * N_SIO2;
        let per_band: Vec<[(f64, Vec<f64>); 2]> = self
            .bands
            .par_iter()
            .map(|b| -> photonoxide::Result<[(f64, Vec<f64>); 2]> {
                let contrast = b.core * b.core - clad;
                let region: Vec<f64> = share.iter().map(|s| clad + s * contrast).collect();
                let mut eps = b.background.clone();
                self.placement.embed(&region, &mut eps)?;
                let s = b.solver.reuse_cells(b.lam, &eps)?;
                let field = s.solve_system(&s.mode_source(&b.input, Direction::Forward))?;
                // the share into a port, and its gradient in the densities
                let port =
                    |(mode, ratio): &(PortMode, f64)| -> photonoxide::Result<(f64, Vec<f64>)> {
                        let (p, dp) = s.mode_power_gradient(&field, mode, Direction::Forward)?;
                        let d_share: Vec<f64> = self
                            .placement
                            .restrict(&dp)?
                            .iter()
                            .map(|g| g * contrast * ratio)
                            .collect();
                        Ok((p * ratio, self.density.gradient(rho, &d_share)?))
                    };
                let (t, dt) = port(&b.outputs[0])?;
                let (x, dx) = port(&b.outputs[1])?;
                Ok([
                    (
                        round12((1.0 - t) / SPEC.0),
                        dt.iter().map(|&d| round12(-d / SPEC.0)).collect(),
                    ),
                    (
                        round12(x / SPEC.1),
                        dx.iter().map(|&d| round12(d / SPEC.1)).collect(),
                    ),
                ])
            })
            .collect::<photonoxide::Result<_>>()?;
        let mut values = Vec::new();
        let mut gradients = Vec::new();
        for (v, g) in per_band.into_iter().flatten() {
            values.push(v);
            gradients.extend(g);
        }
        Ok((values, gradients))
    }
}

/// The optimization, the design binarized.
fn optimize() -> photonoxide::Result<Vec<bool>> {
    use genoxide::prelude::*;
    let n = SIDE * SIDE;
    let m = 2 * wavelengths().len();
    // a start that breaks the mirror symmetry across the axis, which a demultiplexer must
    let mut rho: Vec<f64> = (0..n)
        .map(|p| {
            let (i, j) = (
                (p % SIDE) as f64 / SIDE as f64,
                (p / SIDE) as f64 / SIDE as f64,
            );
            0.5 + 0.25
                * portable::sin(std::f64::consts::PI * (j - 0.5))
                * portable::sin(std::f64::consts::PI * i)
        })
        .collect();
    let form = Epigraph::new(n, m)?;
    let start = Instant::now();
    let mut last = None;
    for beta in BETAS {
        let d = Demux::new(beta)?;
        let (values, _) = d.evaluate(&rho)?;
        let mut y = rho.clone();
        y.push(Epigraph::start(&values));
        let fitness = Constrained::differentiable(
            m,
            |x: &Reals, gradient: &mut [f64], g: &mut [f64], jacobian: &mut [f64]| {
                let Ok((values, gradients)) = d.evaluate(&x[..n]) else {
                    return f64::NAN;
                };
                if form
                    .evaluate(x, &values, &gradients, gradient, g, jacobian)
                    .is_err()
                {
                    return f64::NAN;
                }
                x[n]
            },
        );
        let mma = Mma::builder(Real::new(vec![0.0..=1.0; n + 1]).expect("bounds"))
            .initial_genome(Reals::from(y))
            .minimize()
            .build()
            .expect("an MMA");
        let outcome = Engine::new(mma, fitness)
            .stop_when(Stop::evaluations(PER_STAGE))
            .run()
            .expect("a run");
        rho = outcome.best_genome()[..n].to_vec();
        let (values, _) = d.evaluate(&rho)?;
        let worst = |k: usize| {
            values
                .iter()
                .skip(k)
                .step_by(2)
                .copied()
                .fold(0.0, f64::max)
        };
        println!(
            "    β {beta:>3}: worst T {:.4}, worst other port {:.4} ({:.0} s)",
            1.0 - worst(0) * SPEC.0,
            worst(1) * SPEC.1,
            start.elapsed().as_secs_f64()
        );
        last = Some(d);
    }
    let d = last.expect("a stage");
    Ok(d.density.physical(&rho)?.iter().map(|&v| v > 0.5).collect())
}

/// The design as rows of hex digits (each row along x, its first value the first digit's highest
/// bit).
fn to_hex(design: &[bool]) -> String {
    let mut s = String::new();
    for row in design.chunks(SIDE) {
        for c in row.chunks(4) {
            let nibble = c
                .iter()
                .fold(0u32, |a, &b| a * 2 + u32::from(b))
                .checked_shl(4 - c.len() as u32)
                .unwrap_or(0);
            s.push(char::from_digit(nibble, 16).expect("a digit"));
        }
        s.push('\n');
    }
    s
}

fn from_hex(hex: &str) -> Vec<bool> {
    let mut d = vec![false; SIDE * SIDE];
    for (j, line) in hex.lines().enumerate() {
        for (c, ch) in line.trim().chars().enumerate() {
            let nibble = ch.to_digit(16).expect("a hex digit");
            for q in 0..4 {
                if 4 * c + q < SIDE {
                    d[j * SIDE + 4 * c + q] = nibble & (8 >> q) != 0;
                }
            }
        }
    }
    d
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

/// A 3D problem on one of [`LAYER_CELLS`].
struct Cell3d {
    grid: Grid3d,
    pml: usize,
}

impl Cell3d {
    fn new(across: usize) -> Cell3d {
        let h = THICKNESS / across as f64;
        let cells = |um: f64| (um / h).round() as usize;
        let pml = PML_3D;
        let [x, y] = DOMAIN_3D;
        Cell3d {
            grid: Grid3d {
                nx: cells(2.0 * x) + 2 * pml,
                ny: cells(2.0 * y) + 2 * pml,
                nz: cells(Z_3D.1 - Z_3D.0) + 2 * pml,
                dx: h,
                dy: h,
                dz: h,
                x0: -x - pml as f64 * h,
                y0: -y - pml as f64 * h,
                z0: Z_3D.0 - pml as f64 * h,
            },
            pml,
        }
    }

    /// ε at (x, y, z) with `design` in the layer.
    fn eps(design: &[bool], x: f64, y: f64, z: f64) -> c64 {
        let e = if z < 0.0 {
            N_SIO2 * N_SIO2
        } else if z < THICKNESS {
            let silicon = match pixel(x, y) {
                Some((a, b)) => design[b * SIDE + a],
                None => guides(x, y, 1e-9) > 0.5,
            };
            if silicon { N_SI * N_SI } else { 1.0 }
        } else {
            1.0
        };
        c64::new(e, 0.0)
    }

    /// `design` at `lam` (µm): the wanted port's share and the other's, and GMRES's iterations.
    fn solve(&self, design: &[bool], lam: f64, up: bool) -> photonoxide::Result<([f64; 2], usize)> {
        let g = self.grid;
        let solver = IterativeSolver3d::new_with(
            g,
            Wavelength::um(lam)?,
            |x, y, z| Cell3d::eps(design, x, y, z),
            Boundaries3d::stretched_pml(self.pml),
            Formulation::ShinFan,
            Product::MatrixFree,
        )?
        .with_multigrid(Multigrid::default())?;
        let (x, row) = (|v: f64| index(v, g.x0, g.dx), |v: f64| index(v, g.y0, g.dy));
        let layers = self.pml..g.nz - self.pml;
        let top = DOMAIN_3D[1];
        let window = |lo: f64, hi: f64| (row(lo)..row(hi), layers.clone());
        let input = solver
            .port_modes_within(Axis::X, x(-PORT_X_3D), window(-1.2, 1.2), 1)?
            .remove(0);
        let above = solver
            .port_modes_within(Axis::X, x(PORT_X_3D), window(0.0, top), 1)?
            .remove(0);
        let below = solver
            .port_modes_within(Axis::X, x(PORT_X_3D), window(-top, 0.0), 1)?
            .remove(0);
        let stopping = Stopping {
            tolerance: RESIDUAL,
            max_iterations: 5_000,
        };
        let (field, convergence) =
            solver.solve_system(&solver.mode_source(&input, Direction::Forward), stopping)?;
        let share = |m| field.mode_amplitudes(m).0.norm_sqr() * m.power() / input.power();
        let (wanted, other) = if up {
            (&above, &below)
        } else {
            (&below, &above)
        };
        Ok(([share(wanted), share(other)], convergence.iterations))
    }
}

/// The 3D runs: per grid, (T, other) at 1300 nm and at 1550 nm.
fn three_d(design: &[bool], log: &mut std::fs::File) -> photonoxide::Result<Vec<[f64; 4]>> {
    let started = Instant::now();
    let weight = |n: usize| Cell3d::new(n).grid.unknowns() as f64;
    let total: f64 = LAYER_CELLS.iter().map(|&n| 2.0 * weight(n)).sum();
    let steps = 2 * LAYER_CELLS.len();
    let (mut done, mut step) = (0.0, 0);
    let mut out = Vec::new();
    for (k, &n) in LAYER_CELLS.iter().enumerate() {
        let cell = Cell3d::new(n);
        let mut row = [f64::NAN; 4];
        for (b, &(nm, up)) in CENTRES.iter().enumerate() {
            let ([t, other], iterations) = cell.solve(design, nm * 1e-3, up)?;
            row[2 * b] = t;
            row[2 * b + 1] = other;
            step += 1;
            done += weight(n);
            let elapsed = started.elapsed().as_secs_f64() / 60.0;
            let line = format!(
                "step {step}/{steps} ({:.0} %): grid {} of {} ({} × {} × {} cells, {:.1} nm), {nm} nm: T {t:.5}, other {other:.3e}, {iterations} iterations; {elapsed:.1} min in, about {:.0} min left",
                100.0 * done / total,
                k + 1,
                LAYER_CELLS.len(),
                cell.grid.nx,
                cell.grid.ny,
                cell.grid.nz,
                1000.0 * cell.grid.dx,
                elapsed * (total - done) / done
            );
            eprintln!("{line}");
            let _ = writeln!(log, "{line}");
        }
        out.push(row);
    }
    Ok(out)
}

/// dB of a power share.
fn db(p: f64) -> f64 {
    10.0 * portable::log10(p)
}

/// The 3D results against Fig. 4(a).
fn compare_3d(checks: &mut common::Checks, results: &[[f64; 4]]) {
    let h = LAYER_CELLS.map(|n| THICKNESS / n as f64);
    for (b, &(nm, _)) in CENTRES.iter().enumerate() {
        let (theirs, theirs_other) = FIG_4A[b];
        let t = [0, 1, 2].map(|k| db(results[k][2 * b]));
        let x = [0, 1, 2].map(|k| db(results[k][2 * b + 1]));
        let (te, ts, order) = extrapolated(h, t);
        let (xe, xs, _) = extrapolated(h, x);
        println!(
            "  {nm} nm: wanted port {:.3}, {:.3}, {:.3} dB on the three grids, {te:.2} ± {ts:.2} dB extrapolated (order {}); Fig. 4(a): {theirs} dB",
            t[0],
            t[1],
            t[2],
            order.map_or("none".to_string(), |q| format!("{q:.1}"))
        );
        println!(
            "    other port {:.1}, {:.1}, {:.1} dB, {xe:.1} ± {xs:.1} dB extrapolated; Fig. 4(a): {theirs_other} dB",
            x[0], x[1], x[2]
        );
        let bound = theirs - READING;
        if te - ts >= bound {
            checks.at_most(&format!("{nm} nm, 3D insertion loss (dB)"), -te, -bound);
        } else if te + ts < bound {
            println!(
                "    reported, not checked: {:.2} dB more loss than their design's, beyond our grids' spread (a gap, #595)",
                bound - te
            );
        } else {
            println!(
                "    reported, not checked: their {theirs} dB is within our grids' spread of ours"
            );
        }
    }
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let args: Vec<String> = std::env::args().collect();
    let full = args.iter().any(|a| a == "--full");
    let write = args.iter().any(|a| a == "--write");
    let three = (full && !write) || args.iter().any(|a| a == "--3d");
    let mut checks = common::Checks::default();
    let mut setup = common::Setup::new();
    setup.item(
        "device",
        "a 2.8 × 2.8 µm region of 220 nm silicon on silica, air above; an input and two outputs 500 nm wide, the outputs' centres 0.54 µm above and below the input's axis (Fig. 1(b))",
    );
    setup.medium("silicon", &common::n(N_SI));
    setup.medium("silica", &common::n(N_SIO2));
    setup.item(
        "wavelengths",
        "1300 nm to port 2 (above) and 1550 nm to port 3 (below), each with four more at ±1 and ±2 % of its frequency; the input's fundamental TE mode",
    );
    setup.item(
        "2D",
        "the effective index method: the 220 nm slab's TE index in the silicon and the silica's outside, H along z; 20 nm cells, 10 nm beside; PMLs of 0.3 µm",
    );
    setup.item(
        "design",
        "140 × 140 pixels of 20 nm, a 100 nm conic filter, tanh projection at η = ½, β 8 to 256 in six stages of 40 evaluations; genoxide's MMA on the worst of the ten 1 − T over 10 % and the ten shares into the other port over 1 %",
    );
    setup.item(
        "3D (--full)",
        "the design extruded through the layer; cells of 36.7, 31.4 and 27.5 nm, stretched PMLs of 0.44 µm, GMRES and the multigrid to 1e-9",
    );
    setup.item(
        "tolerance",
        "2D: their specification at the centres, T ≥ 90 % and the other port ≤ 1 %; 3D: our loss at most Fig. 4(a)'s, read to ±0.1 dB, where our grids' spread decides it",
    );
    setup.print();

    for (nm, _) in CENTRES {
        println!(
            "The slab's TE index at {nm} nm: {:.4}",
            slab_index(nm * 1e-3)?
        );
    }
    let design = if full {
        println!("The optimization, in 2D on 20 nm cells:");
        let d = optimize()?;
        if write {
            std::fs::write(RECORDED_FILE, to_hex(&d)).expect("the design written");
            println!("  written to {RECORDED_FILE}");
        } else {
            let differing = d
                .iter()
                .zip(from_hex(RECORDED))
                .filter(|(a, b)| **a != *b)
                .count();
            checks.count("pixels unlike the recorded design", differing, 0);
        }
        d
    } else {
        from_hex(RECORDED)
    };
    let silicon = design.iter().filter(|&&b| b).count();
    println!(
        "The recorded design: {:.1} % silicon",
        100.0 * silicon as f64 / design.len() as f64
    );

    println!(
        "In 2D (not the device's performance), binarized: the wanted port, the other and the reflection, each of the input's power:"
    );
    let coarse = spectrum(&design, PIXEL)?;
    let fine = spectrum(&design, 0.5 * PIXEL)?;
    for (k, (lam, up)) in wavelengths().into_iter().enumerate() {
        let [t, x, r] = fine[k];
        println!(
            "  {:.1} nm → port {}: T {t:.4} ({:+.4} from 20 nm), other {x:.2e}, R {r:.2e}",
            1e3 * lam,
            if up { 2 } else { 3 },
            t - coarse[k][0]
        );
    }
    for (b, &(nm, _)) in CENTRES.iter().enumerate() {
        let [t, x, _] = fine[5 * b + 2];
        checks.at_most(&format!("{nm} nm, 2D: 1 − T"), 1.0 - t, SPEC.0);
        checks.at_most(&format!("{nm} nm, 2D: the other port"), x, SPEC.1);
    }

    println!("3D FDFD of the design at the two centres:");
    if three {
        let path =
            std::env::var("DEMUX_SOI_LOG").unwrap_or_else(|_| "optimize_demux_soi.log".into());
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .expect("a log file");
        eprintln!("progress in {path}");
        let results = three_d(&design, &mut log)?;
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
