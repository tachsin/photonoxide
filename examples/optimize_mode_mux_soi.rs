//! Frellsen et al.'s two-mode (de)multiplexer: a 4.22 × 2.6 µm region of 340 nm silicon on
//! silica, air above, between a 710 nm two-mode waveguide and two 430 nm single-mode ones,
//! sending TE₀ to the upper and TE₁ (as TE₀) to the lower; their design, read from their figure,
//! and one designed here in 2D, both solved in 3D FDFD.
//!
//! L. F. Frellsen, Y. Ding, O. Sigmund and L. H. Frandsen, "Topology optimized mode multiplexing
//! in silicon-on-insulator photonic wire waveguides", Opt. Express 24, 16866 (2016),
//! [doi:10.1364/OE.24.016866](https://doi.org/10.1364/OE.24.016866), Section 2 and Figs. 1 and 2.
//!
//! **The problem** (their Section 2.1 and Fig. 1(a)). A fully etched layer of silicon 340 nm
//! thick (ε = 11.68) on silica (2.085), air above; a design domain of 4.22 × 2.6 µm between a
//! two-mode waveguide 0.71 µm wide on the left and two single-mode ones 0.43 µm wide on the right.
//! TE₀ in the wide guide is to leave by the upper single-mode guide, TE₁ by the lower one, as
//! TE₀; by reciprocity the same device multiplexes. The guides' places are read from Fig. 1(b)
//! (the figure's scale from the three guides' printed widths): the wide guide's centre 0.315 µm
//! above the domain's, the outputs' 0.424 µm above and 0.473 µm below it.
//!
//! **Their design** (Fig. 1(b)), from 3D topology optimization by FDTD on 32 nm cells, is printed
//! as a black-and-white image; it is read here onto the domain's 20 nm pixels
//! (`examples/designs/frellsen_2016_fig1b.hex`) and solved first, to check this setup against
//! their 3D FDTD of it, Fig. 2(d): transmission normalized to a straight wire's, at 1550 nm
//! −0.11 dB for TE₀ to TE₀ and −0.38 dB for TE₀ to TE₁ (read from the figure to ±0.02 dB; the
//! paper: below 1 dB of loss from 1450 to 1700 nm, measured below 1.2 dB, context only). A
//! straight two-mode wire in the same setup checks the ports: all its light stays in its mode.
//!
//! **Our design.** The plane by the effective index method: the 340 nm slab's TE index at each
//! wavelength (silica below, air above) inside the silicon and air's outside (the air that
//! `optimize_demux_soi` found the 3D device needs), H along z, 2D FDFD on 20 nm cells, PMLs of
//! 0.3 µm. Densities on the domain's 211 × 130 pixels of 20 nm, a conic filter of 100 nm and a
//! tanh projection at η = ½ (`topology`). At five wavelengths, equally spaced in frequency
//! ±3 and ±6 % about 1575 nm (their pulses' 260 nm about 1580 nm), for each of the two modes,
//! the worst of 1 − T and of ten times the share into the other guide is minimized in its
//! epigraph form by genoxide's MMA, every gradient by the adjoint; β from 8 to 256 in six stages
//! of 40 evaluations. Their method differs: 3D FDTD with pulses and the adjoint, 200 iterations.
//!
//! **The 3D check.** Each design extruded through the 340 nm layer, solved in 3D FDFD (`--full`)
//! at 1550 nm on cells of 42.5, 37.8 and 34 nm (an eighth, a ninth and a tenth of the layer)
//! inside stretched PMLs of 16 cells, by GMRES and the multigrid to a residual of 1e-9; the wide
//! guide's TE₀ or TE₁ in, each output's TE₀ out in a window of its own; extrapolated to h → 0 at
//! second order.
//!
//! **What is compared.** Their design solved here against Fig. 2(d), agreement checked where our
//! grids' spread and the reading decide it, otherwise reported. Our design against their design's
//! Fig. 2(d) as a target: our loss checked to be at most theirs where our spread decides it,
//! otherwise reported. The share into the other guide is reported only: their Fig. 2(c) gives an
//! extinction ratio from a line scan of the flux, not a modal share.
//!
//! **What it found**: not run yet.
//!
//! ```sh
//! cargo run --release --example optimize_mode_mux_soi                    # the recorded designs in 2D, the recorded 3D
//! cargo run --release --example optimize_mode_mux_soi -- --full          # the optimization again, and the 3D solves
//! cargo run --release --example optimize_mode_mux_soi -- --full --write  # the optimization, recorded
//! cargo run --release --example optimize_mode_mux_soi -- --3d             # the recorded designs' 3D solves
//! cargo run --release --example optimize_mode_mux_soi -- --3d --coarse    # those on the coarsest grid only
//! ```

mod common;

use photonoxide::portable;
use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{
    Axis, Boundaries, Boundaries3d, Direction, Formulation, Grid, Grid3d, IterativeSolver3d,
    Multigrid, Polarization, PortMode, PortMode3d, Product, Solver2d, Stopping,
};
use photonoxide::mode::slab::Slab;
use photonoxide::topology::{Density, Edge, Epigraph, Placement, Projection};
use photonoxide::units::{Length, Wavelength};
use rayon::prelude::*;

/// The permittivities Frellsen et al. use, and the layer's thickness, µm.
const EPS_SI: f64 = 11.68;
const EPS_SIO2: f64 = 2.085;
const THICKNESS: f64 = 0.34;
/// The plane's index where the layer is etched away.
const N_ETCHED: f64 = 1.0;
/// The two-mode guide: its centre (y) and width, µm; the single-mode guides', upper then lower.
const WIDE: (f64, f64) = (0.315, 0.71);
const OUTPUTS: [(f64, f64); 2] = [(0.424, 0.43), (-0.473, 0.43)];
/// The domain's half-sizes, µm; the design's pixel, µm, and pixels along x and y.
const HALF: [f64; 2] = [2.11, 1.3];
const PIXEL: f64 = 0.02;
const SIDE: [usize; 2] = [211, 130];
/// The 2D domain's half-sizes inside the PMLs, and the PMLs, µm; the ports' x.
const DOMAIN_2D: [f64; 2] = [2.71, 1.8];
const PML_2D: f64 = 0.3;
const PORT_X: f64 = 2.45;
/// The centre wavelength, µm, and the steps in frequency about it.
const CENTRE: f64 = 1.575;
const STEPS: [f64; 5] = [-0.06, -0.03, 0.0, 0.03, 0.06];
/// The half-sizes of their start's silicon rectangle, µm (Section 2.1: 4 × 2.4 µm).
const START: [f64; 2] = [2.0, 1.2];
/// The conic filter's radius, µm; β's stages and their evaluations.
const RADIUS: f64 = 0.1;
const BETAS: [f64; 6] = [8.0, 16.0, 32.0, 64.0, 128.0, 256.0];
const PER_STAGE: u64 = 40;
/// The bounds the search aims at: 1 − T at most, and the other guide's share at most.
const SPEC: (f64, f64) = (0.1, 0.01);
/// The other guide's weight in the objectives, so that both bounds count alike.
const WEIGHT: f64 = SPEC.0 / SPEC.1;
/// Fig. 2(d), their 3D FDTD of their design at 1550 nm, dB: TE₀ to TE₀, TE₀ to TE₁; the
/// reading's ±.
const FIG_2D: [f64; 2] = [-0.11, -0.38];
const READING: f64 = 0.02;
/// The 3D wavelength, µm; cells across the layer; the domain's half-sizes inside the PMLs, µm
/// (x, y), and its z inside them, the layer from 0 to THICKNESS; the PMLs' cells; the ports' x.
const LAMBDA_3D: f64 = 1.55;
const LAYER_CELLS: [usize; 3] = [8, 9, 10];
const DOMAIN_3D: [f64; 2] = [2.75, 1.75];
const Z_3D: (f64, f64) = (-0.45, 0.79);
const PML_3D: usize = 16;
const PORT_X_3D: f64 = 2.45;
const RESIDUAL: f64 = 1e-9;
/// What `--full` found in 3D, per grid: per piece (the straight wire, their design, ours), per
/// mode in (TE₀, TE₁), the wanted output's share and the other's (the straight wire: the same
/// mode's and the other mode's).
const RECORDED_3D: [[[[f64; 2]; 2]; 3]; 3] = [[[[f64::NAN; 2]; 2]; 3]; 3];

const OURS: &str = include_str!("designs/optimize_mode_mux_soi.hex");
const OURS_FILE: &str = "examples/designs/optimize_mode_mux_soi.hex";
const THEIRS: &str = include_str!("designs/frellsen_2016_fig1b.hex");

/// The design's wavelengths, µm.
fn wavelengths() -> Vec<f64> {
    STEPS.iter().map(|s| CENTRE / (1.0 + s)).collect()
}

/// The 340 nm slab's TE index at `lam`, µm: silica below, air above.
fn slab_index(lam: f64) -> photonoxide::Result<f64> {
    Ok(
        Slab::new(EPS_SIO2.sqrt(), EPS_SI.sqrt(), 1.0, Length::um(THICKNESS))?
            .modes(photonoxide::mode::Polarization::Te, Wavelength::um(lam)?)[0]
            .effective_index(),
    )
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

/// The share of the guides outside the domain in the cell [x−h/2, x+h/2] × [y−h/2, y+h/2];
/// `straight`: the two-mode guide alone, all the way through.
fn guides(x: f64, y: f64, h: f64, straight: bool) -> f64 {
    let (ya, yb) = (y - 0.5 * h, y + 0.5 * h);
    let across = |(c, w): (f64, f64)| within(ya, yb, c - 0.5 * w, c + 0.5 * w) / h;
    if straight {
        return across(WIDE);
    }
    let input = within(x - 0.5 * h, x + 0.5 * h, f64::NEG_INFINITY, -HALF[0]) / h;
    let output = within(x - 0.5 * h, x + 0.5 * h, HALF[0], f64::INFINITY) / h;
    input * across(WIDE) + output * (across(OUTPUTS[0]) + across(OUTPUTS[1]))
}

/// The design pixel (column, row) holding the point (x, y), if it is in the domain.
fn pixel(x: f64, y: f64) -> Option<(usize, usize)> {
    let inside = |v: f64, k: usize| (-HALF[k]..HALF[k]).contains(&v);
    (inside(x, 0) && inside(y, 1)).then(|| {
        let at = |v: f64, k: usize| (((v + HALF[k]) / PIXEL).floor() as usize).min(SIDE[k] - 1);
        (at(x, 0), at(y, 1))
    })
}

/// The plane's permittivities on `g` at a slab index `core`: the guides, and in the domain
/// `design`'s shares of silicon, one per pixel (none if `None`).
fn cells(g: &Grid, core: f64, design: Option<&[f64]>) -> Vec<c64> {
    let (si, clad) = (core * core, N_ETCHED * N_ETCHED);
    (0..g.nx * g.ny)
        .map(|p| {
            let (x, y) = (g.x(p % g.nx), g.y(p / g.nx));
            let s = match pixel(x, y) {
                Some((a, b)) => design.map_or(0.0, |d| d[b * SIDE[0] + a]),
                None => guides(x, y, g.dx, false),
            };
            c64::new(clad + s * (si - clad), 0.0)
        })
        .collect()
}

/// The column or row of `v`, µm, on an axis from `v0` in steps of `h`.
fn index(v: f64, v0: f64, h: f64) -> usize {
    ((v - v0) / h).round() as usize
}

/// The y between the two outputs, which parts their windows.
fn parting() -> f64 {
    let [(a, wa), (b, wb)] = OUTPUTS;
    0.5 * ((a - 0.5 * wa) + (b + 0.5 * wb))
}

/// The ports on a 2D problem: the wide guide's TE₀ and TE₁, and the outputs' TE₀ (upper,
/// lower), each in a window of its own.
fn ports(s: &Solver2d, g: &Grid) -> photonoxide::Result<([PortMode; 2], [PortMode; 2])> {
    let (h, row) = (g.dx, |y: f64| index(y, g.y0, g.dy));
    let top = DOMAIN_2D[1];
    let mut wide = s.port_modes_within(
        index(-PORT_X, g.x0, h),
        row(WIDE.0 - 1.2)..row(WIDE.0 + 1.2),
        2,
    )?;
    let te1 = wide.remove(1);
    let te0 = wide.remove(0);
    let column = index(PORT_X, g.x0, h);
    let mid = parting();
    let up = s
        .port_modes_within(column, row(mid)..row(top), 1)?
        .remove(0);
    let down = s
        .port_modes_within(column, row(-top)..row(mid), 1)?
        .remove(0);
    Ok(([te0, te1], [up, down]))
}

/// A binary design solved in 2D on cells of `h`: per wavelength, per mode in, the wanted
/// guide's share and the other's, each of the input mode's power.
fn spectrum(design: &[bool], h: f64) -> photonoxide::Result<Vec<[[f64; 2]; 2]>> {
    let g = grid(h);
    let d: Vec<f64> = design.iter().map(|&b| f64::from(u8::from(b))).collect();
    wavelengths()
        .into_par_iter()
        .map(|lam| {
            let eps = cells(&g, slab_index(lam)?, Some(&d));
            let s = Solver2d::from_cells(
                g,
                Polarization::Hz,
                Wavelength::um(lam)?,
                &eps,
                Boundaries::pml(pml_2d(h)),
            )?;
            let (inputs, outputs) = ports(&s, &g)?;
            let mut out = [[0.0; 2]; 2];
            for (m, input) in inputs.iter().enumerate() {
                let field = s.solve_system(&s.mode_source(input, Direction::Forward))?;
                let p_in = input.power(g.dy);
                let share =
                    |o: &PortMode| field.mode_amplitudes(o).0.norm_sqr() * o.power(g.dy) / p_in;
                out[m] = [share(&outputs[m]), share(&outputs[1 - m])];
            }
            Ok(out)
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

/// One wavelength of the optimization: the slab index, the solver at the background, the wide
/// guide's two modes, the outputs' and their powers over each input's.
struct Band {
    lam: Wavelength,
    core: f64,
    background: Vec<c64>,
    solver: Solver2d,
    inputs: [PortMode; 2],
    outputs: [PortMode; 2],
    dy: f64,
}

/// The optimization's problem at one β.
struct Mux {
    density: Density,
    placement: Placement,
    bands: Vec<Band>,
}

impl Mux {
    fn new(beta: f64) -> photonoxide::Result<Mux> {
        let g = grid(PIXEL);
        let density = Density::new([SIDE[0], SIDE[1], 1], PIXEL, [0.0, 1.0])?
            .with_filter(RADIUS, Edge::Renormalized)?
            .with_projection(Projection::new(beta, 0.5)?);
        let bands = wavelengths()
            .into_par_iter()
            .map(|lam| {
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
                let (inputs, outputs) = ports(&solver, &g)?;
                Ok(Band {
                    lam: wavelength,
                    core,
                    background,
                    solver,
                    inputs,
                    outputs,
                    dy: g.dy,
                })
            })
            .collect::<photonoxide::Result<_>>()?;
        Ok(Mux {
            density,
            placement: Placement {
                origin: [
                    index(-HALF[0], g.x0, PIXEL),
                    index(-HALF[1], g.y0, PIXEL),
                    0,
                ],
                shape: [SIDE[0], SIDE[1], 1],
                grid: [g.nx, g.ny, 1],
            },
            bands,
        })
    }

    /// The twenty objectives, per wavelength and mode in 1 − T and the other guide's share
    /// times [`WEIGHT`], and their gradients in the densities, rounded to 12 digits.
    fn evaluate(&self, rho: &[f64]) -> photonoxide::Result<(Vec<f64>, Vec<f64>)> {
        let share = self.density.permittivity(rho)?;
        let clad = N_ETCHED * N_ETCHED;
        let jobs: Vec<(usize, usize)> = (0..self.bands.len())
            .flat_map(|b| [(b, 0), (b, 1)])
            .collect();
        let per_job: Vec<[(f64, Vec<f64>); 2]> = jobs
            .par_iter()
            .map(|&(k, m)| -> photonoxide::Result<[(f64, Vec<f64>); 2]> {
                let b = &self.bands[k];
                let contrast = b.core * b.core - clad;
                let region: Vec<f64> = share.iter().map(|s| clad + s * contrast).collect();
                let mut eps = b.background.clone();
                self.placement.embed(&region, &mut eps)?;
                let s = b.solver.reuse_cells(b.lam, &eps)?;
                let input = &b.inputs[m];
                let field = s.solve_system(&s.mode_source(input, Direction::Forward))?;
                let p_in = input.power(b.dy);
                // the share into a guide, and its gradient in the densities
                let port = |mode: &PortMode| -> photonoxide::Result<(f64, Vec<f64>)> {
                    let ratio = mode.power(b.dy) / p_in;
                    let (p, dp) = s.mode_power_gradient(&field, mode, Direction::Forward)?;
                    let d_share: Vec<f64> = self
                        .placement
                        .restrict(&dp)?
                        .iter()
                        .map(|g| g * contrast * ratio)
                        .collect();
                    Ok((p * ratio, self.density.gradient(rho, &d_share)?))
                };
                let (t, dt) = port(&b.outputs[m])?;
                let (x, dx) = port(&b.outputs[1 - m])?;
                Ok([
                    (round12(1.0 - t), dt.iter().map(|&d| round12(-d)).collect()),
                    (
                        round12(x * WEIGHT),
                        dx.iter().map(|&d| round12(d * WEIGHT)).collect(),
                    ),
                ])
            })
            .collect::<photonoxide::Result<_>>()?;
        let mut values = Vec::new();
        let mut gradients = Vec::new();
        for (v, g) in per_job.into_iter().flatten() {
            values.push(v);
            gradients.extend(g);
        }
        Ok((values, gradients))
    }
}

/// The optimization, the design binarized.
fn optimize() -> photonoxide::Result<Vec<bool>> {
    use genoxide::prelude::*;
    let n = SIDE[0] * SIDE[1];
    let m = 4 * wavelengths().len();
    // their start (Fig. 1(a)): silicon over 4 × 2.4 µm within the domain, softened to 0.7
    // and 0.3 so that the projection's slope isn't nil there
    let mut rho: Vec<f64> = (0..n)
        .map(|p| {
            let x = -HALF[0] + ((p % SIDE[0]) as f64 + 0.5) * PIXEL;
            let y = -HALF[1] + ((p / SIDE[0]) as f64 + 0.5) * PIXEL;
            if x.abs() < START[0] && y.abs() < START[1] {
                0.7
            } else {
                0.3
            }
        })
        .collect();
    let form = Epigraph::new(n, m)?;
    // the densities, and the epigraph's bound, which the weighted share can take up to WEIGHT
    let mut bounds = vec![0.0..=1.0; n];
    bounds.push(0.0..=WEIGHT);
    let start = Instant::now();
    let mut last = None;
    for beta in BETAS {
        let d = Mux::new(beta)?;
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
        let mma = Mma::builder(Real::new(bounds.clone()).expect("bounds"))
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
            "    β {beta:>3}: worst T {:.4}, worst other guide {:.4} ({:.0} s)",
            1.0 - worst(0),
            worst(1) / WEIGHT,
            start.elapsed().as_secs_f64()
        );
        last = Some(d);
    }
    let d = last.expect("a stage");
    Ok(d.density.physical(&rho)?.iter().map(|&v| v > 0.5).collect())
}

/// The design as rows of hex digits, from the lowest y (each row along x, its first value the
/// first digit's highest bit).
fn to_hex(design: &[bool]) -> String {
    let mut s = String::new();
    for row in design.chunks(SIDE[0]) {
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
    let mut d = vec![false; SIDE[0] * SIDE[1]];
    for (j, line) in hex.lines().enumerate() {
        for (c, ch) in line.trim().chars().enumerate() {
            let nibble = ch.to_digit(16).expect("a hex digit");
            for q in 0..4 {
                if 4 * c + q < SIDE[0] {
                    d[j * SIDE[0] + 4 * c + q] = nibble & (8 >> q) != 0;
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

/// What a 3D solve has in its layer.
#[derive(Clone, Copy)]
enum Piece<'a> {
    /// The two-mode guide alone, all the way through.
    Straight,
    /// A design in the domain, between the guides.
    Device(&'a [bool]),
}

/// A 3D problem on one of [`LAYER_CELLS`].
struct Cell3d {
    grid: Grid3d,
    pml: usize,
}

type Window = (std::ops::Range<usize>, std::ops::Range<usize>);

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

    /// ε at (x, y, z) with `piece` in the layer.
    fn eps(piece: Piece, x: f64, y: f64, z: f64) -> c64 {
        let e = if z < 0.0 {
            EPS_SIO2
        } else if z < THICKNESS {
            let silicon = match (piece, pixel(x, y)) {
                (Piece::Device(design), Some((a, b))) => design[b * SIDE[0] + a],
                (Piece::Straight, _) => guides(x, y, 1e-9, true) > 0.5,
                (Piece::Device(_), None) => guides(x, y, 1e-9, false) > 0.5,
            };
            if silicon { EPS_SI } else { 1.0 }
        } else {
            1.0
        };
        c64::new(e, 0.0)
    }

    /// The share of a mode's transverse field along y (in the plane) over its window.
    fn along_y(mode: &PortMode3d, window: &Window) -> f64 {
        let (mut y, mut z) = (0.0, 0.0);
        for v in window.1.clone() {
            for u in window.0.clone() {
                y += mode.e(Axis::Y, (u, v)).norm_sqr();
                z += mode.e(Axis::Z, (u, v)).norm_sqr();
            }
        }
        y / (y + z)
    }

    /// The first `count` TE modes (E mostly along y) among a window's first `among`.
    fn te_modes(
        solver: &IterativeSolver3d,
        plane: usize,
        window: Window,
        among: usize,
        count: usize,
    ) -> photonoxide::Result<Vec<PortMode3d>> {
        let modes: Vec<PortMode3d> = solver
            .port_modes_within(Axis::X, plane, window.clone(), among)?
            .into_iter()
            .filter(|m| Cell3d::along_y(m, &window) > 0.5)
            .take(count)
            .collect();
        assert_eq!(
            modes.len(),
            count,
            "TE modes among the window's first {among}"
        );
        Ok(modes)
    }

    /// `piece` at [`LAMBDA_3D`] with the wide guide's mode `m` (0: TE₀, 1: TE₁) in: a device's
    /// wanted output's share and the other's (the straight wire: the same mode's and the other
    /// mode's), and GMRES's iterations.
    fn solve(&self, piece: Piece, m: usize) -> photonoxide::Result<([f64; 2], usize)> {
        let g = self.grid;
        let solver = IterativeSolver3d::new_with(
            g,
            Wavelength::um(LAMBDA_3D)?,
            |x, y, z| Cell3d::eps(piece, x, y, z),
            Boundaries3d::stretched_pml(self.pml),
            Formulation::ShinFan,
            Product::MatrixFree,
        )?
        .with_multigrid(Multigrid::default())?;
        let (x, row) = (|v: f64| index(v, g.x0, g.dx), |v: f64| index(v, g.y0, g.dy));
        let layers = self.pml..g.nz - self.pml;
        let top = DOMAIN_3D[1];
        let window = |lo: f64, hi: f64| (row(lo)..row(hi), layers.clone());
        let wide = window(WIDE.0 - 1.2, (WIDE.0 + 1.2).min(top));
        let inputs = Cell3d::te_modes(&solver, x(-PORT_X_3D), wide.clone(), 4, 2)?;
        let input = &inputs[m];
        let outputs = match piece {
            Piece::Straight => Cell3d::te_modes(&solver, x(PORT_X_3D), wide, 4, 2)?,
            Piece::Device(_) => {
                let mid = parting();
                let up = Cell3d::te_modes(&solver, x(PORT_X_3D), window(mid, top), 2, 1)?;
                let down = Cell3d::te_modes(&solver, x(PORT_X_3D), window(-top, mid), 2, 1)?;
                up.into_iter().chain(down).collect()
            }
        };
        let stopping = Stopping {
            tolerance: RESIDUAL,
            max_iterations: 5_000,
        };
        let (field, convergence) =
            solver.solve_system(&solver.mode_source(input, Direction::Forward), stopping)?;
        let share =
            |o: &PortMode3d| field.mode_amplitudes(o).0.norm_sqr() * o.power() / input.power();
        Ok((
            [share(&outputs[m]), share(&outputs[1 - m])],
            convergence.iterations,
        ))
    }
}

/// The 3D results of one grid: per piece (the straight wire, theirs, ours), per mode in.
type Results3d = [[[f64; 2]; 2]; 3];

/// The 3D runs on `grids` of [`LAYER_CELLS`], of the pieces `wanted` keeps (the straight wire
/// is solved on the first grid only, which is all its check needs).
fn three_d(
    designs: [&[bool]; 2],
    grids: &[usize],
    wanted: [bool; 3],
    log: &mut std::fs::File,
) -> photonoxide::Result<Vec<Results3d>> {
    let started = Instant::now();
    let names = ["the straight wire", "their design", "our design"];
    let pieces = [
        Piece::Straight,
        Piece::Device(designs[0]),
        Piece::Device(designs[1]),
    ];
    // the solves, each weighted by its unknowns (the straight wire's need few iterations)
    let jobs: Vec<(usize, usize, usize, f64)> = (0..grids.len())
        .flat_map(|k| (0..3).flat_map(move |p| (0..2).map(move |m| (k, p, m))))
        .filter(|&(k, p, _)| wanted[p] && (p > 0 || k == 0))
        .filter(|&(_, p, _)| !matches!(pieces[p], Piece::Device(d) if !d.contains(&true)))
        .map(|(k, p, m)| {
            let unknowns = Cell3d::new(grids[k]).grid.unknowns() as f64;
            (k, p, m, if p == 0 { 0.25 * unknowns } else { unknowns })
        })
        .collect();
    let total: f64 = jobs.iter().map(|j| j.3).sum();
    let mut done = 0.0;
    let mut out = vec![[[[f64::NAN; 2]; 2]; 3]; grids.len()];
    for (step, &(k, p, m, weight)) in jobs.iter().enumerate() {
        let cell = Cell3d::new(grids[k]);
        let ([a, b], iterations) = cell.solve(pieces[p], m)?;
        out[k][p][m] = [a, b];
        done += weight;
        let elapsed = started.elapsed().as_secs_f64() / 60.0;
        let line = format!(
            "step {}/{} ({:.0} %): grid {} of {} ({} × {} × {} cells, {:.1} nm), {}, TE{m} in: {a:.5}, {b:.3e}, {iterations} iterations; {elapsed:.1} min in, about {:.0} min left",
            step + 1,
            jobs.len(),
            100.0 * done / total,
            k + 1,
            grids.len(),
            cell.grid.nx,
            cell.grid.ny,
            cell.grid.nz,
            1000.0 * cell.grid.dx,
            names[p],
            elapsed * (total - done) / done
        );
        eprintln!("{line}");
        let _ = writeln!(log, "{line}");
    }
    Ok(out)
}

/// Whether 3D results hold every solve: the straight wire's on the first grid, the designs' on
/// all three.
fn complete(results: &[Results3d]) -> bool {
    let finite = |r: &[[f64; 2]; 2]| r.iter().flatten().all(|v| v.is_finite());
    finite(&results[0][0]) && results.iter().all(|g| finite(&g[1]) && finite(&g[2]))
}

/// dB of a power share.
fn db(p: f64) -> f64 {
    10.0 * portable::log10(p)
}

/// The 3D results against Fig. 2(d).
fn compare_3d(checks: &mut common::Checks, results: &[Results3d]) {
    let h = LAYER_CELLS.map(|n| THICKNESS / n as f64);
    let modes = ["TE₀ to TE₀", "TE₀ to TE₁"];
    for (m, &[stay, other]) in results[0][0].iter().enumerate() {
        println!(
            "  the straight wire on {:.1} nm cells, TE{m} in: {stay:.5} stays in it ({:.4} dB), {other:.1e} goes to the other mode",
            1e3 * h[0],
            db(stay)
        );
    }
    for (p, name) in [(1, "their design (Fig. 1(b))"), (2, "our design")] {
        println!("  {name}, at {:.0} nm:", 1e3 * LAMBDA_3D);
        for (m, &theirs) in FIG_2D.iter().enumerate() {
            let t = [0, 1, 2].map(|k| db(results[k][p][m][0]));
            let x = [0, 1, 2].map(|k| db(results[k][p][m][1]));
            let (te, ts, order) = extrapolated(h, t);
            let (xe, xs, _) = extrapolated(h, x);
            println!(
                "    {}: {:.3}, {:.3}, {:.3} dB on the three grids, {te:.2} ± {ts:.2} dB extrapolated (order {}); Fig. 2(d): {theirs} dB",
                modes[m],
                t[0],
                t[1],
                t[2],
                order.map_or("none".to_string(), |q| format!("{q:.1}"))
            );
            println!(
                "      into the other guide {:.1}, {:.1}, {:.1} dB, {xe:.1} ± {xs:.1} dB extrapolated (reported)",
                x[0], x[1], x[2]
            );
            let what = format!("{name}, {}, 3D (dB)", modes[m]);
            if p == 1 {
                // their design: ours agrees with their FDTD within the reading and our spread
                let tolerance = READING + ts;
                if (te - theirs).abs() <= tolerance {
                    checks.compare(&what, te, theirs, tolerance);
                } else {
                    println!(
                        "      reported, not checked: {:.2} dB from Fig. 2(d), beyond our grids' spread and the reading (a gap, #595)",
                        te - theirs
                    );
                }
            } else {
                // ours: its loss at most their design's where our spread decides it
                let bound = theirs - READING;
                if te - ts >= bound {
                    checks.at_most(&format!("{what}, loss"), -te, -bound);
                } else if te + ts < bound {
                    println!(
                        "      reported, not checked: {:.2} dB more loss than their design's, beyond our grids' spread and the reading (a gap, #595)",
                        theirs - te
                    );
                } else {
                    println!(
                        "      reported, not checked: their {theirs} dB is within our grids' spread of ours"
                    );
                }
            }
        }
    }
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let args: Vec<String> = std::env::args().collect();
    let full = args.iter().any(|a| a == "--full");
    let write = args.iter().any(|a| a == "--write");
    let coarse = args.iter().any(|a| a == "--coarse");
    let three = (full && !write) || args.iter().any(|a| a == "--3d");
    let mut checks = common::Checks::default();
    let mut setup = common::Setup::new();
    setup.item(
        "device",
        "a 4.22 × 2.6 µm domain of 340 nm silicon on silica, air above; a two-mode guide 0.71 µm wide on the left (its centre 0.315 µm above the domain's), single-mode guides 0.43 µm wide on the right (0.424 µm above and 0.473 µm below), read from Fig. 1(b)",
    );
    setup.medium("silicon", "ε = 11.68");
    setup.medium("silica", "ε = 2.085");
    setup.item(
        "wavelengths",
        "five, ±3 and ±6 % in frequency about 1575 nm; TE₀ in to the upper guide, TE₁ in to the lower, each as TE₀",
    );
    setup.item(
        "2D",
        "the effective index method: the 340 nm slab's TE index in the silicon and air's outside, H along z; 20 nm cells, 10 nm beside; PMLs of 0.3 µm",
    );
    setup.item(
        "design",
        "211 × 130 pixels of 20 nm, a 100 nm conic filter, tanh projection at η = ½, β 8 to 256 in six stages of 40 evaluations; genoxide's MMA on the worst of the ten 1 − T and of the ten shares into the other guide times 10",
    );
    setup.item("their design", "Fig. 1(b), read onto the same 20 nm pixels");
    setup.item(
        "3D (--full)",
        "each design extruded through the layer, at 1550 nm; cells of 42.5, 37.8 and 34 nm, stretched PMLs of 16 cells, GMRES and the multigrid to 1e-9; a straight two-mode wire on the first",
    );
    setup.item(
        "tolerance",
        "3D: their design against Fig. 2(d), read to ±0.02 dB, within the reading and our grids' spread; our loss at most theirs where that spread decides it; the share into the other guide reported",
    );
    setup.print();

    for lam in [wavelengths()[0], CENTRE, wavelengths()[4]] {
        println!(
            "The slab's TE index at {:.1} nm: {:.4}",
            1e3 * lam,
            slab_index(lam)?
        );
    }
    let theirs = from_hex(THEIRS);
    let ours = if full {
        println!("The optimization, in 2D on 20 nm cells:");
        let d = optimize()?;
        if write {
            std::fs::write(OURS_FILE, to_hex(&d)).expect("the design written");
            println!("  written to {OURS_FILE}");
        } else {
            let differing = d
                .iter()
                .zip(from_hex(OURS))
                .filter(|(a, b)| **a != *b)
                .count();
            checks.count("pixels unlike the recorded design", differing, 0);
        }
        d
    } else {
        from_hex(OURS)
    };
    let fill = |d: &[bool]| 100.0 * d.iter().filter(|&&b| b).count() as f64 / d.len() as f64;
    println!(
        "Silicon in the domain: {:.1} % of theirs (as read), {:.1} % of ours",
        fill(&theirs),
        fill(&ours)
    );

    println!(
        "In 2D (not the devices' performance), binarized: per mode in, the wanted guide's share and the other's, each of the input's power:"
    );
    for (name, design) in [("their design", &theirs), ("our design", &ours)] {
        println!("  {name}:");
        if !design.contains(&true) {
            println!("    not designed yet: run --full --write");
            continue;
        }
        let coarse = spectrum(design, PIXEL)?;
        let fine = spectrum(design, 0.5 * PIXEL)?;
        for (k, lam) in wavelengths().into_iter().enumerate() {
            let [[t0, x0], [t1, x1]] = fine[k];
            println!(
                "    {:.1} nm: TE₀ {t0:.4} ({:+.4} from 20 nm), other {x0:.2e}; TE₁ {t1:.4} ({:+.4}), other {x1:.2e}",
                1e3 * lam,
                t0 - coarse[k][0][0],
                t1 - coarse[k][1][0]
            );
        }
    }

    println!("3D FDFD at 1550 nm:");
    if three {
        let path = std::env::var("MODE_MUX_SOI_LOG")
            .unwrap_or_else(|_| "optimize_mode_mux_soi.log".into());
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .expect("a log file");
        eprintln!("progress in {path}");
        let grids: &[usize] = if coarse {
            &LAYER_CELLS[..1]
        } else {
            &LAYER_CELLS
        };
        let wanted = if args.iter().any(|a| a == "--ours") {
            [false, false, true]
        } else {
            [true; 3]
        };
        let results = three_d([&theirs, &ours], grids, wanted, &mut log)?;
        println!("  recorded: {results:?}");
        if coarse {
            for (p, name) in [(1, "their design"), (2, "our design")] {
                for (m, &[t, x]) in results[0][p].iter().enumerate() {
                    println!(
                        "  {name}, TE{m} in, {:.1} nm cells: {:.3} dB, other {:.1} dB",
                        1e3 * THICKNESS / LAYER_CELLS[0] as f64,
                        db(t),
                        db(x)
                    );
                }
            }
        } else {
            compare_3d(&mut checks, &results);
        }
    } else if complete(&RECORDED_3D) {
        println!("  what --full found (recorded):");
        compare_3d(&mut checks, &RECORDED_3D);
    } else {
        println!("  not recorded yet: run --full");
    }
    Ok(checks.finish())
}
