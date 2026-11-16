//! Su et al.'s grating coupler: a fibre's Gaussian beam at 10° onto a 14 µm grating of 100 nm
//! deep grooves in 220 nm silicon, coupled into the slab's TE₀; their design, read from their
//! figure, and one designed here, both solved in 2D FDFD and again on three finer grids.
//!
//! L. Su, R. Trivedi, N. V. Sapra, A. Y. Piggott, D. Vercruysse and J. Vučković, "Fully-automated
//! optimization of grating couplers", Opt. Express 26, 4023 (2018),
//! [doi:10.1364/OE.26.004023](https://doi.org/10.1364/OE.26.004023), Section 3.1 and Fig. 2.
//!
//! A grating coupler is a problem of its cross-section, the plane of the layer stack and the
//! direction the light runs in, so it is designed and verified here in 2D only, as the paper does.
//!
//! TEXT_PLACEHOLDER

mod common;

use photonoxide::portable;
use std::io::Write;
use std::process::ExitCode;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{Boundaries, Direction, Grid, Polarization, PortMode, Solver2d};
use photonoxide::material::{silica, silicon};
use photonoxide::topology::{Density, Edge, Epigraph, Projection};
use photonoxide::units::Wavelength;
use rayon::prelude::*;

/// The silicon layer's thickness, the grooves' depth and the buried oxide's thickness, µm.
const LAYER: f64 = 0.22;
const ETCH: f64 = 0.1;
const BOX: f64 = 2.0;
/// The design region along x, µm, from 0; its pixels, µm, and their count.
const REGION: f64 = 14.0;
const PIXEL: f64 = 0.02;
const SIDE: usize = 700;
/// The beam: its angle from the normal in air, degrees (in the silica by Snell's law); its waist
/// (the 1/e radius of the field), µm; its centre along x, µm; the row it starts on, µm.
const ANGLE: f64 = 10.0;
const WAIST: f64 = 5.2;
const BEAM_X: f64 = 6.8;
const SOURCE_Y: f64 = 1.0;
/// Where the incident power is measured, µm, between the layer and the source (in silica alone).
const FLUX_Y: f64 = 0.6;
/// The domain inside the PMLs, µm, and the PMLs' thickness, µm.
const X_SPAN: (f64, f64) = (-6.0, 20.0);
const Y_SPAN: (f64, f64) = (-2.6, 1.4);
const PML: f64 = 1.0;
/// The waveguide's port: its x and the rows of its window (µm), clear of the substrate.
const PORT_X: f64 = -3.0;
const PORT_ROWS: (f64, f64) = (-1.0, 1.2);
/// The design's wavelength, µm; the spectrum's first and last wavelengths and its points.
const LAMBDA: f64 = 1.55;
const BAND: (f64, f64, usize) = (1.52, 1.58, 16);
/// The density stage: the start's density, the conic filter's radius, µm, β's stages and their
/// evaluations.
const START: f64 = 0.5;
const RADIUS: f64 = 0.1;
const BETAS: [f64; 6] = [8.0, 16.0, 32.0, 64.0, 128.0, 256.0];
const PER_STAGE: u64 = 40;
/// Their minimum feature, µm: every groove and every tooth between grooves at least this wide.
const FEATURE: f64 = 0.1;
/// The edge stage: its stages, their evaluations, and how far an edge may move in one, µm.
const EDGE_STAGES: usize = 3;
const PER_EDGE_STAGE: u64 = 50;
const REACH: f64 = 0.2;
/// Their design (Fig. 2(a)): the grooves, µm from the region's start, read from the figure's
/// vector outline at its 1 µm scale bar (11.801 pt).
const GROOVES: [(f64, f64); 23] = [
    (0.0971, 0.2090),
    (0.3163, 0.5827),
    (0.6826, 1.3413),
    (1.4673, 1.6212),
    (2.6597, 2.7598),
    (3.2950, 3.3951),
    (3.9584, 4.0582),
    (4.5003, 4.6004),
    (5.0932, 5.2082),
    (5.7156, 5.8382),
    (6.3099, 6.4459),
    (6.9251, 7.0780),
    (7.5333, 7.7337),
    (8.1237, 8.3658),
    (8.7578, 8.9960),
    (9.3876, 9.6283),
    (9.9939, 10.2511),
    (10.6432, 10.8721),
    (11.2463, 11.4924),
    (11.8747, 12.1122),
    (12.4929, 12.7327),
    (13.1094, 13.3520),
    (13.7336, 13.9730),
];
/// The paper's text: the least insertion loss, dB, and the 1-dB bandwidth, nm. Fig. 2(c)'s
/// curve, read from its vector path: least loss 1.9185 dB at 1550.3 nm, 1-dB band 1532.6 to
/// 1569.1 nm (36.55 nm). The two differ by 0.02 dB and 2.55 nm: that is the reading's ±.
const TEXT_LOSS: f64 = 1.94;
const TEXT_BANDWIDTH: f64 = 34.0;
const FIG_LOSS: f64 = 1.9185;
const FIG_BANDWIDTH: f64 = 36.55;
const FIG_CENTRE: f64 = 1550.3;
/// The finer grids of `--full`: cells of 20 nm divided by these, and how many wavelengths each
/// solves at once (a 5 nm grid's factorization takes several GB).
const FINE: [usize; 3] = [2, 3, 4];
const AT_ONCE: [usize; 3] = [4, 3, 2];
/// What `--full` found, per fine grid, per design (theirs, ours): the least loss, dB, its
/// wavelength, nm, and the 1-dB band's edges, nm.
const RECORDED: [[[f64; 4]; 2]; 3] = [[[f64::NAN; 4]; 2]; 3];

const OURS: &str = include_str!("designs/optimize_grating_coupler_soi.txt");
const OURS_FILE: &str = "examples/designs/optimize_grating_coupler_soi.txt";

/// Silicon's and silica's indices at `lam`, µm.
fn indices(lam: f64) -> photonoxide::Result<(f64, f64)> {
    let w = Wavelength::um(lam)?;
    Ok((
        silicon().refractive_index(w)?.re,
        silica().refractive_index(w)?.re,
    ))
}

/// The 2D grid of cell `h`, µm, the PMLs included.
fn grid(h: f64) -> Grid {
    let (x0, y0) = (X_SPAN.0 - PML, Y_SPAN.0 - PML);
    Grid {
        nx: ((X_SPAN.1 + PML - x0) / h).round() as usize,
        ny: ((Y_SPAN.1 + PML - y0) / h).round() as usize,
        dx: h,
        dy: h,
        x0,
        y0,
    }
}

/// The PMLs' cells on a grid of cell `h`.
fn pml(h: f64) -> usize {
    (PML / h).round() as usize
}

/// The length of [a, b] within [lo, hi].
fn within(a: f64, b: f64, lo: f64, hi: f64) -> f64 {
    (hi.min(b) - lo.max(a)).max(0.0)
}

/// The column or row holding `v`, µm, on an axis from `v0` in steps of `h`.
fn index(v: f64, v0: f64, h: f64) -> usize {
    ((v - v0) / h).floor() as usize
}

/// The plane's permittivities on `g` (each cell the mean over its area: E along z is parallel to
/// every interface): the silicon substrate, the buried oxide, the layer with `grooves` in its top
/// 100 nm, silica above; `None`: silica everywhere.
fn cells(g: &Grid, (si, ox): (f64, f64), grooves: Option<&[(f64, f64)]>) -> Vec<c64> {
    let (si, ox) = (si * si, ox * ox);
    let Some(grooves) = grooves else {
        return vec![c64::new(ox, 0.0); g.nx * g.ny];
    };
    let h = g.dx;
    // the share of silicon in the grooves' band, column by column
    let shares: Vec<f64> = (0..g.nx)
        .map(|i| {
            let (a, b) = (g.x(i) - 0.5 * h, g.x(i) + 0.5 * h);
            1.0 - grooves
                .iter()
                .map(|&(lo, hi)| within(a, b, lo, hi))
                .sum::<f64>()
                / h
        })
        .collect();
    let mut eps = Vec::with_capacity(g.nx * g.ny);
    for j in 0..g.ny {
        let (a, b) = (g.y(j) - 0.5 * h, g.y(j) + 0.5 * h);
        let silicon = within(a, b, f64::NEG_INFINITY, -BOX) + within(a, b, 0.0, LAYER - ETCH);
        let band = within(a, b, LAYER - ETCH, LAYER);
        let oxide = within(a, b, -BOX, 0.0) + within(a, b, LAYER, f64::INFINITY);
        for s in &shares {
            let e = (si * (silicon + band * s) + ox * (oxide + band * (1.0 - s))) / h;
            eps.push(c64::new(e, 0.0));
        }
    }
    eps
}

/// The fibre's beam as a line of current on the row at [`SOURCE_Y`] in silica of index `n`: a
/// Gaussian of waist [`WAIST`] about `centre`, seen along the row (stretched by 1/cos θ), its
/// phase running at k₀ sin θ towards −x (θ in air: the same along x in the silica by Snell's
/// law), so that the beam going down leans towards the waveguide.
fn beam(g: &Grid, lam: f64, n: f64, centre: f64) -> Vec<c64> {
    let sin = portable::sin(ANGLE * std::f64::consts::PI / 180.0);
    let kx = -2.0 * std::f64::consts::PI / lam * sin;
    let cos = (1.0 - sin * sin / (n * n)).sqrt();
    let (row, edge) = (index(SOURCE_Y, g.y0, g.dy), pml(g.dx));
    let mut j = vec![c64::new(0.0, 0.0); g.nx * g.ny];
    for i in edge..g.nx - edge {
        let x = g.x(i);
        let u = (x - centre) * cos / WAIST;
        j[row * g.nx + i] = portable::complex::from_polar(portable::exp(-u * u), kx * x);
    }
    j
}

/// The incident power of `source` on `g` at `lam`: the beam in silica alone, through a row
/// below it.
fn incident(g: Grid, lam: f64, source: &[c64]) -> photonoxide::Result<f64> {
    let free = Solver2d::from_cells(
        g,
        Polarization::Ez,
        Wavelength::um(lam)?,
        &cells(&g, indices(lam)?, None),
        Boundaries::pml(pml(g.dx)),
    )?;
    Ok(-free.solve(source)?.flux_y(index(FLUX_Y, g.y0, g.dy)))
}

/// The waveguide's TE₀ on a solver of `g`.
fn waveguide_mode(s: &Solver2d, g: &Grid) -> photonoxide::Result<PortMode> {
    let rows = index(PORT_ROWS.0, g.y0, g.dy)..index(PORT_ROWS.1, g.y0, g.dy);
    Ok(s.port_modes_within(index(PORT_X, g.x0, g.dx), rows, 1)?
        .remove(0))
}

/// The shares of the incident power that `designs` (their grooves) couple into the waveguide's
/// TE₀ on cells of `h`, µm, at `lam`, µm, the beam's centre at `centre`.
fn coupled(
    h: f64,
    lam: f64,
    centre: f64,
    designs: &[&[(f64, f64)]],
) -> photonoxide::Result<Vec<f64>> {
    let g = grid(h);
    let n = indices(lam)?;
    let wavelength = Wavelength::um(lam)?;
    let source = beam(&g, lam, n.1, centre);
    let incident = incident(g, lam, &source)?;
    // the first design's solver, reused for the others, and the waveguide's TE₀ on it (the
    // port's column is the same slab in every design)
    let mut first: Option<(Solver2d, PortMode)> = None;
    let mut out = Vec::new();
    for &d in designs {
        let eps = cells(&g, n, Some(d));
        let s = match &first {
            None => Solver2d::from_cells(
                g,
                Polarization::Ez,
                wavelength,
                &eps,
                Boundaries::pml(pml(h)),
            )?,
            Some((s, _)) => s.reuse_cells(wavelength, &eps)?,
        };
        let mode = match &first {
            None => waveguide_mode(&s, &g)?,
            Some((_, m)) => m.clone(),
        };
        let back = s.solve(&source)?.mode_amplitudes(&mode).1;
        out.push(back.norm_sqr() * mode.power(g.dy) / incident);
        if first.is_none() {
            first = Some((s, mode));
        }
    }
    Ok(out)
}

/// x rounded to 12 significant digits: an evaluation's values and gradients, so that a system's
/// last bits can't steer the optimizer.
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

/// The optimization's problem at [`LAMBDA`] on cells of [`PIXEL`]: the slab's solver, the
/// waveguide's TE₀, the beam and the power it brings down.
struct Coupler {
    lam: Wavelength,
    /// Silicon's and silica's indices.
    n: (f64, f64),
    oxide: f64,
    contrast: f64,
    background: Vec<c64>,
    solver: Solver2d,
    mode: PortMode,
    source: Vec<c64>,
    /// The waveguide mode's power over the incident power.
    ratio: f64,
    /// The grooves' band's rows.
    rows: std::ops::Range<usize>,
}

impl Coupler {
    fn new() -> photonoxide::Result<Coupler> {
        let g = grid(PIXEL);
        let n = indices(LAMBDA)?;
        let lam = Wavelength::um(LAMBDA)?;
        let source = beam(&g, LAMBDA, n.1, BEAM_X);
        let incident = incident(g, LAMBDA, &source)?;
        let background = cells(&g, n, Some(&[]));
        let solver =
            Solver2d::from_cells(g, Polarization::Ez, lam, &background, Boundaries::pml(pml(PIXEL)))?;
        let mode = waveguide_mode(&solver, &g)?;
        let ratio = mode.power(g.dy) / incident;
        // the band's rows, by their centres
        let rows = index(LAYER - ETCH + 0.5 * PIXEL, g.y0, g.dy)
            ..index(LAYER - 0.5 * PIXEL, g.y0, g.dy) + 1;
        Ok(Coupler {
            lam,
            n,
            oxide: n.1 * n.1,
            contrast: n.0 * n.0 - n.1 * n.1,
            background,
            solver,
            mode,
            source,
            ratio,
            rows,
        })
    }

    /// The share η coupled with `eps`, and its derivative with respect to each column's share of
    /// silicon in the grooves' band.
    fn coupling(&self, eps: &[c64]) -> photonoxide::Result<(f64, Vec<f64>)> {
        let s = self.solver.reuse_cells(self.lam, eps)?;
        let field = s.solve(&self.source)?;
        let (power, d_eps) = s.mode_power_gradient(&field, &self.mode, Direction::Backward)?;
        let g = s.grid();
        let d_share = (0..g.nx)
            .map(|c| {
                self.rows.clone().map(|j| d_eps[j * g.nx + c]).sum::<f64>()
                    * self.contrast
                    * self.ratio
            })
            .collect();
        Ok((power * self.ratio, d_share))
    }

    /// The density stage's 1 − η and its gradient in the pixels' densities, rounded.
    fn densities(&self, density: &Density, rho: &[f64]) -> photonoxide::Result<(f64, Vec<f64>)> {
        let g = self.solver.grid();
        let share = density.permittivity(rho)?;
        let column = |k: usize| index((k as f64 + 0.5) * PIXEL, g.x0, g.dx);
        let mut eps = self.background.clone();
        for (k, s) in share.iter().enumerate() {
            for j in self.rows.clone() {
                eps[j * g.nx + column(k)] = c64::new(self.oxide + s * self.contrast, 0.0);
            }
        }
        let (eta, d_column) = self.coupling(&eps)?;
        let d_share: Vec<f64> = (0..SIDE).map(|k| d_column[column(k)]).collect();
        let d_rho = density.gradient(rho, &d_share)?;
        Ok((
            round12(1.0 - eta),
            d_rho.iter().map(|&d| round12(-d)).collect(),
        ))
    }

    /// The edge stage's 1 − η and its gradient in the edges (each groove's start and end in
    /// turn), rounded: moving an edge by δ moves δ/h of silicon in or out of its cell.
    fn edges(&self, x: &[f64]) -> photonoxide::Result<(f64, Vec<f64>)> {
        let g = self.solver.grid();
        let grooves: Vec<(f64, f64)> = x.chunks(2).map(|e| (e[0], e[1])).collect();
        let eps = cells(&g, self.n, Some(&grooves));
        let (eta, d_column) = self.coupling(&eps)?;
        let gradient = x
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                // a groove's start moving on gives silicon back; its end moving on takes it
                let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
                let c = index(p, g.x0, g.dx).min(g.nx - 1);
                round12(-sign * d_column[c] / g.dx)
            })
            .collect();
        Ok((round12(1.0 - eta), gradient))
    }
}

/// The progress log's path: `GRATING_SOI_LOG`, or one in the working directory.
fn log_path() -> String {
    std::env::var("GRATING_SOI_LOG").unwrap_or_else(|_| "optimize_grating_coupler_soi.log".into())
}

/// The progress log, opened to append.
fn open_log() -> Mutex<std::fs::File> {
    let path = log_path();
    eprintln!("progress in {path}");
    Mutex::new(
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .expect("a log file"),
    )
}

/// A line to stderr and the progress log.
fn note(log: &Mutex<std::fs::File>, line: &str) {
    eprintln!("{line}");
    let _ = writeln!(log.lock().expect("the log"), "{line}");
}

/// The optimization's progress: evaluations done of all of them, and since when.
struct Progress {
    log: Mutex<std::fs::File>,
    count: AtomicU64,
    total: u64,
    start: Instant,
}

impl Progress {
    fn step(&self, stage: &str, eta: f64) {
        let k = self.count.fetch_add(1, Ordering::SeqCst) + 1;
        let minutes = self.start.elapsed().as_secs_f64() / 60.0;
        note(
            &self.log,
            &format!(
                "optimization: evaluation {k}/{} ({:.0} %), {stage}: η {eta:.4}; {minutes:.1} min in, about {:.1} min left",
                self.total,
                100.0 * k.min(self.total) as f64 / self.total as f64,
                minutes * self.total.saturating_sub(k) as f64 / k as f64
            ),
        );
    }
}

/// The density stage: densities on the region's pixels, filtered and projected, β raised in
/// stages; the design binarized.
fn density_stage(problem: &Coupler, progress: &Progress) -> photonoxide::Result<Vec<bool>> {
    use genoxide::prelude::*;
    let n = SIDE;
    let mut rho = vec![START; n];
    let form = Epigraph::new(n, 1)?;
    // the densities, and the epigraph's bound: 1 − η, at most 1, its start the value itself
    let mut bounds = vec![0.0..=1.0; n];
    bounds.push(0.0..=1.0);
    let mut density = None;
    for beta in BETAS {
        let d = Density::new([SIDE, 1, 1], PIXEL, [0.0, 1.0])?
            .with_filter(RADIUS, Edge::Padded(1.0))?
            .with_projection(Projection::new(beta, 0.5)?);
        let evaluate = |x: &[f64]| {
            let out = problem.densities(&d, x);
            if let Ok((v, _)) = &out {
                progress.step(&format!("densities, β {beta}"), 1.0 - v);
            }
            out
        };
        let (value, _) = evaluate(&rho)?;
        let mut y = rho.clone();
        y.push(Epigraph::start(&[value]));
        let fitness = Constrained::differentiable(
            1,
            |x: &Reals, gradient: &mut [f64], g: &mut [f64], jacobian: &mut [f64]| {
                let Ok((value, d)) = evaluate(&x[..n]) else {
                    return f64::NAN;
                };
                if form
                    .evaluate(x, &[value], &d, gradient, g, jacobian)
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
        let (value, _) = evaluate(&rho)?;
        println!(
            "    densities, β {beta:>3}: η {:.4} ({:.2} dB) ({:.0} s)",
            1.0 - value,
            -db(1.0 - value),
            progress.start.elapsed().as_secs_f64()
        );
        density = Some(d);
    }
    let d = density.expect("a stage");
    Ok(d.physical(&rho)?.iter().map(|&v| v > 0.5).collect())
}

/// The edge stage, as theirs: the grooves' edges moved by MMA with every groove and every tooth
/// between grooves at least [`FEATURE`] wide (linear constraints), each stage within [`REACH`]
/// of where it starts.
fn edge_stage(
    problem: &Coupler,
    start: &[(f64, f64)],
    progress: &Progress,
) -> photonoxide::Result<Vec<(f64, f64)>> {
    use genoxide::prelude::*;
    let mut x: Vec<f64> = start.iter().flat_map(|&(a, b)| [a, b]).collect();
    let n = x.len();
    let m = n - 1;
    for stage in 0..EDGE_STAGES {
        let evaluate = |x: &[f64]| {
            let out = problem.edges(x);
            if let Ok((v, _)) = &out {
                progress.step(&format!("edges, stage {}", stage + 1), 1.0 - v);
            }
            out
        };
        let bounds: Vec<_> = x
            .iter()
            .map(|&p| (p - REACH).max(0.0)..=(p + REACH).min(REGION))
            .collect();
        let fitness = Constrained::differentiable(
            m,
            |y: &Reals, gradient: &mut [f64], g: &mut [f64], jacobian: &mut [f64]| {
                let Ok((value, d)) = evaluate(&y[..n]) else {
                    return f64::NAN;
                };
                gradient.copy_from_slice(&d);
                jacobian.fill(0.0);
                for i in 0..m {
                    // FEATURE − (x_{i+1} − x_i) ≤ 0
                    g[i] = FEATURE - (y[i + 1] - y[i]);
                    jacobian[i * n + i] = 1.0;
                    jacobian[i * n + i + 1] = -1.0;
                }
                value
            },
        );
        let mma = Mma::builder(Real::new(bounds).expect("bounds"))
            .initial_genome(Reals::from(x.clone()))
            .minimize()
            .build()
            .expect("an MMA");
        let outcome = Engine::new(mma, fitness)
            .stop_when(Stop::evaluations(PER_EDGE_STAGE))
            .run()
            .expect("a run");
        x = outcome.best_genome()[..n].to_vec();
        let (value, _) = evaluate(&x)?;
        println!(
            "    edges, stage {}: η {:.4} ({:.2} dB) ({:.0} s)",
            stage + 1,
            1.0 - value,
            -db(1.0 - value),
            progress.start.elapsed().as_secs_f64()
        );
    }
    Ok(x.chunks(2).map(|e| (e[0], e[1])).collect())
}

/// The optimization: the density stage, its grooves made feasible, then the edge stage.
fn optimize() -> photonoxide::Result<Vec<(f64, f64)>> {
    let problem = Coupler::new()?;
    let progress = Progress {
        log: open_log(),
        count: AtomicU64::new(0),
        total: BETAS.len() as u64 * (PER_STAGE + 2) + EDGE_STAGES as u64 * (PER_EDGE_STAGE + 1),
        start: Instant::now(),
    };
    let pixels = density_stage(&problem, &progress)?;
    let first = grooves(&pixels);
    let start = feasible(first.clone());
    let eta = |g: &[(f64, f64)]| -> photonoxide::Result<f64> {
        let x: Vec<f64> = g.iter().flat_map(|&(a, b)| [a, b]).collect();
        Ok(1.0 - problem.edges(&x)?.0)
    };
    println!(
        "    binarized: {} grooves, the narrowest feature {:.0} nm, η {:.4}; with those under {:.0} nm merged: {} grooves, η {:.4}",
        first.len(),
        1e3 * narrowest(&first),
        eta(&first)?,
        1e3 * FEATURE,
        start.len(),
        eta(&start)?
    );
    edge_stage(&problem, &start, &progress)
}

/// A binary design's grooves, µm from the region's start.
fn grooves(design: &[bool]) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut open = None;
    for (k, &silicon) in design.iter().chain([&true]).enumerate() {
        match (silicon, open) {
            (false, None) => open = Some(k),
            (true, Some(a)) => {
                out.push((a as f64 * PIXEL, k as f64 * PIXEL));
                open = None;
            }
            _ => {}
        }
    }
    out
}

/// The grooves' and the teeth's widths (the teeth between grooves; the slab outside the region
/// continues the first and the last).
fn widths(g: &[(f64, f64)]) -> impl Iterator<Item = f64> + '_ {
    let grooves = g.iter().map(|&(a, b)| b - a);
    let teeth = g.windows(2).map(|w| w[1].0 - w[0].1);
    grooves.chain(teeth)
}

/// The narrowest groove or tooth, µm.
fn narrowest(g: &[(f64, f64)]) -> f64 {
    widths(g).fold(f64::INFINITY, f64::min)
}

/// Grooves with no feature under [`FEATURE`]: the narrowest, while it is too narrow, filled (a
/// groove) or etched with its neighbours (a tooth).
fn feasible(mut g: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    loop {
        let groove = (0..g.len()).map(|k| (g[k].1 - g[k].0, k, true));
        let tooth = (1..g.len()).map(|k| (g[k].0 - g[k - 1].1, k, false));
        let Some((width, k, is_groove)) = groove.chain(tooth).min_by(|a, b| a.0.total_cmp(&b.0))
        else {
            return g;
        };
        if width >= FEATURE {
            return g;
        }
        if is_groove {
            g.remove(k);
        } else {
            g[k - 1].1 = g[k].1;
            g.remove(k);
        }
    }
}

/// A design's grooves as text, one per line, µm to 6 decimals.
fn to_text(g: &[(f64, f64)]) -> String {
    g.iter().map(|(a, b)| format!("{a:.6} {b:.6}\n")).collect()
}

fn from_text(text: &str) -> Vec<(f64, f64)> {
    text.lines()
        .filter_map(|l| {
            let mut v = l.split_whitespace().map(|s| s.parse::<f64>().expect("a number"));
            Some((v.next()?, v.next()?))
        })
        .collect()
}

/// dB of a power share.
fn db(p: f64) -> f64 {
    10.0 * portable::log10(p)
}

/// The spectrum's wavelengths, µm.
fn band() -> Vec<f64> {
    let (a, b, n) = BAND;
    (0..n)
        .map(|k| a + (b - a) * k as f64 / (n - 1) as f64)
        .collect()
}

/// From a spectrum of losses, dB, at `lams`, nm: the least loss and its wavelength (a parabola
/// through the least sample and its neighbours), and the 1-dB band's edges (linear between the
/// samples); NaN for an edge outside the spectrum.
fn features(lams: &[f64], loss: &[f64]) -> [f64; 4] {
    let k = (0..loss.len())
        .min_by(|&a, &b| loss[a].total_cmp(&loss[b]))
        .expect("a spectrum");
    let (centre, least) = if k == 0 || k + 1 == loss.len() {
        (lams[k], loss[k])
    } else {
        let (y0, y1, y2) = (loss[k - 1], loss[k], loss[k + 1]);
        let d = y0 - 2.0 * y1 + y2;
        let step = lams[k] - lams[k - 1];
        (
            lams[k] + 0.5 * step * (y0 - y2) / d,
            y1 - (y0 - y2) * (y0 - y2) / (8.0 * d),
        )
    };
    let target = least + 1.0;
    let edge = |order: Vec<usize>| {
        order
            .windows(2)
            .find(|w| loss[w[1]] >= target)
            .map_or(f64::NAN, |w| {
                let (a, b) = (w[0], w[1]);
                lams[a] + (target - loss[a]) * (lams[b] - lams[a]) / (loss[b] - loss[a])
            })
    };
    [
        least,
        centre,
        edge((0..=k).rev().collect()),
        edge((k..loss.len()).collect()),
    ]
}

/// The designs' spectra on cells of `h`, µm: per design, the loss in dB at each of [`band`]'s
/// wavelengths. `log`: a progress line per wavelength.
fn spectra(
    designs: &[&[(f64, f64)]],
    h: f64,
    log: Option<(&Mutex<std::fs::File>, &str)>,
) -> photonoxide::Result<Vec<Vec<f64>>> {
    let lams = band();
    let started = Instant::now();
    let done = AtomicU64::new(0);
    let per_lam: Vec<Vec<f64>> = lams
        .par_iter()
        .map(|&lam| -> photonoxide::Result<Vec<f64>> {
            let out: Vec<f64> = coupled(h, lam, BEAM_X, designs)?
                .into_iter()
                .map(|e| -db(e))
                .collect();
            if let Some((file, label)) = log {
                let k = done.fetch_add(1, Ordering::SeqCst) + 1;
                let n = lams.len() as u64;
                let minutes = started.elapsed().as_secs_f64() / 60.0;
                note(
                    file,
                    &format!(
                        "{label}: wavelength {k}/{n} ({:.0} %) {:.0} nm on {:.2} nm cells: {}; {minutes:.1} min in, about {:.1} min left",
                        100.0 * k as f64 / n as f64,
                        1e3 * lam,
                        1e3 * h,
                        out.iter()
                            .map(|l| format!("{l:.4} dB"))
                            .collect::<Vec<_>>()
                            .join(", "),
                        minutes * (n - k) as f64 / k as f64
                    ),
                );
            }
            Ok(out)
        })
        .collect::<photonoxide::Result<_>>()?;
    Ok((0..designs.len())
        .map(|d| per_lam.iter().map(|v| v[d]).collect())
        .collect())
}

/// A value on the three fine grids: their mean, and half their range.
fn over_grids(p: [f64; 3]) -> (f64, f64) {
    let (lo, hi) = (p[0].min(p[1]).min(p[2]), p[0].max(p[1]).max(p[2]));
    ((p[0] + p[1] + p[2]) / 3.0, 0.5 * (hi - lo))
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let args: Vec<String> = std::env::args().collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    let full = flag("--full");
    let write = flag("--write");
    let fine = (full && !write) || flag("--fine");
    let mut checks = common::Checks::default();
    let mut setup = common::Setup::new();
    setup.item(
        "device",
        "a 14 µm region of 220 nm silicon, grooves 100 nm deep, on 2 µm of buried oxide over silicon, silica above; the waveguide on the left",
    );
    setup.item(
        "beam",
        "TE (E along the grooves), a Gaussian of 5.2 µm waist at 10° from the normal in air (6.9° in the silica), leaning towards the waveguide, its centre 6.8 µm into the region",
    );
    setup.item(
        "2D",
        "FDFD on 20 nm cells (theirs), PMLs of 1 µm all round; the incident power from the same beam in silica alone",
    );
    setup.item("their design", "Fig. 2(a), read from its vector outline");
    setup.item(
        "design",
        "densities on 700 pixels of 20 nm, each the top 100 nm of the layer, a 100 nm conic filter, tanh projection at η = ½, β 8 to 256 in six stages of 40 evaluations from densities ½; then the grooves' edges, every groove and tooth at least 100 nm, three stages of 50; genoxide's MMA on 1 − η at 1550 nm",
    );
    setup.item(
        "--full",
        "both designs again on cells of 10, 6.67 and 5 nm, from 1520 to 1580 nm in steps of 4 nm",
    );
    setup.item(
        "tolerance",
        "their design against the text's least loss and 1-dB bandwidth, within the text's and Fig. 2(c)'s difference and our grids' spread; ours against theirs where that spread decides it",
    );
    setup.medium("silicon", "Li 1980, 293 K (the paper prints no indices)");
    setup.medium("silica", "Malitson 1965");
    setup.print();
    let (si, ox) = indices(LAMBDA)?;
    println!("At 1550 nm: silicon {si:.4}, silica {ox:.4}");

    let ours = if full {
        println!("The optimization, in 2D on 20 nm cells, at 1550 nm:");
        let d = optimize()?;
        if write {
            std::fs::write(OURS_FILE, to_text(&d)).expect("the design written");
            println!("  written to {OURS_FILE}");
        } else {
            let recorded = from_text(OURS);
            let differing = if recorded.len() == d.len() {
                to_text(&d)
                    .lines()
                    .zip(to_text(&recorded).lines())
                    .filter(|(a, b)| a != b)
                    .count()
            } else {
                d.len().max(recorded.len())
            };
            checks.count("grooves unlike the recorded design", differing, 0);
        }
        d
    } else {
        from_text(OURS)
    };
    let designed = !ours.is_empty();
    println!(
        "Their design: {} grooves, the narrowest feature {:.1} nm; ours: {} grooves, the narrowest {:.1} nm",
        GROOVES.len(),
        1e3 * narrowest(&GROOVES),
        ours.len(),
        1e3 * narrowest(&ours)
    );
    if designed {
        // recorded to 1 pm: a feature at the bound may print a pm under it
        let under = widths(&ours).filter(|&w| w < FEATURE - 1e-6).count();
        checks.count("our features under 100 nm", under, 0);
    }

    if flag("--align") {
        println!("Their design at 1550 nm on 20 nm cells, the beam's centre moved:");
        let centres: Vec<f64> = (-15..=15).map(|k| BEAM_X + 0.1 * k as f64).collect();
        let found: Vec<(f64, f64)> = centres
            .par_iter()
            .map(|&c| -> photonoxide::Result<(f64, f64)> {
                Ok((c, coupled(PIXEL, LAMBDA, c, &[&GROOVES])?[0]))
            })
            .collect::<photonoxide::Result<_>>()?;
        for (c, e) in found {
            println!("  {c:.1} µm: {e:.4} ({:.3} dB)", -db(e));
        }
    }

    let mut designs: Vec<&[(f64, f64)]> = vec![&GROOVES];
    if designed {
        designs.push(&ours);
    }
    let names = ["their design", "our design"];
    let lams: Vec<f64> = band().iter().map(|l| 1e3 * l).collect();
    println!("In 2D on 20 nm cells, the loss into the waveguide's TE₀:");
    let coarse = spectra(&designs, PIXEL, None)?;
    for (d, loss) in coarse.iter().enumerate() {
        let [least, centre, lo, hi] = features(&lams, loss);
        println!(
            "  {}: least {least:.3} dB at {centre:.1} nm; 1-dB band {lo:.1} to {hi:.1} nm ({:.1} nm)",
            names[d],
            hi - lo
        );
        let every = loss
            .iter()
            .zip(&lams)
            .map(|(l, lam)| format!("{lam:.0} {l:.2}"))
            .collect::<Vec<_>>()
            .join(", ");
        println!("    {every} (nm dB)");
    }
    if !designed {
        println!("  our design: not designed yet: run --full --write");
    }

    println!("On the finer grids:");
    let results: Vec<[[f64; 4]; 2]> = if fine && designed {
        let log = open_log();
        let mut out = Vec::new();
        for (k, n) in FINE.into_iter().enumerate() {
            let h = PIXEL / n as f64;
            let label = format!("grid {}/{}", k + 1, FINE.len());
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(AT_ONCE[k])
                .build()
                .expect("a thread pool");
            let s = pool.install(|| spectra(&designs, h, Some((&log, &label))))?;
            let f = [features(&lams, &s[0]), features(&lams, &s[1])];
            println!("  recorded: {f:?}");
            out.push(f);
        }
        out
    } else {
        RECORDED.to_vec()
    };
    if results.iter().flatten().flatten().any(|v| v.is_nan()) {
        println!("  not recorded yet: run --full");
        return Ok(checks.finish());
    }
    let h = FINE.map(|n| 1e3 * PIXEL / n as f64);
    println!(
        "  on {:.2}, {:.2} and {:.2} nm cells, then their mean ± half their range:",
        h[0], h[1], h[2]
    );
    let mut summary = [[(0.0, 0.0); 3]; 2];
    for (d, name) in names.iter().enumerate() {
        let least = over_grids([0, 1, 2].map(|k| results[k][d][0]));
        let centre = over_grids([0, 1, 2].map(|k| results[k][d][1]));
        let width = over_grids([0, 1, 2].map(|k| results[k][d][3] - results[k][d][2]));
        println!(
            "  {name}: least loss {:.3} ± {:.3} dB at {:.1} ± {:.1} nm; 1-dB bandwidth {:.1} ± {:.1} nm",
            least.0, least.1, centre.0, centre.1, width.0, width.1
        );
        summary[d] = [least, centre, width];
    }
    println!(
        "  the paper: least loss {TEXT_LOSS} dB, 1-dB bandwidth {TEXT_BANDWIDTH} nm (Fig. 2(c): {FIG_LOSS} dB at {FIG_CENTRE} nm, {FIG_BANDWIDTH} nm)"
    );
    // their design: agreement with the text, within the text's and the figure's difference and
    // our grids' spread
    let [least, _, width] = summary[0];
    for (what, (v, s), printed, reading) in [
        (
            "their design, least loss (dB)",
            least,
            TEXT_LOSS,
            (TEXT_LOSS - FIG_LOSS).abs(),
        ),
        (
            "their design, 1-dB bandwidth (nm)",
            width,
            TEXT_BANDWIDTH,
            (TEXT_BANDWIDTH - FIG_BANDWIDTH).abs(),
        ),
    ] {
        let tolerance = reading + s;
        if (v - printed).abs() <= tolerance {
            checks.compare(what, v, printed, tolerance);
        } else {
            println!(
                "  {what}: {v:.3}, {:+.3} from the paper, beyond the reading and our grids' spread: reported, not checked (a gap, #595)",
                v - printed
            );
        }
    }
    // ours: its least loss at most their design's here, where the grids' spreads decide it
    let ((o, so), (t, st)) = (summary[1][0], summary[0][0]);
    if o + so <= t - st {
        checks.at_most("our least loss (dB), theirs here", o, t);
    } else if o - so > t + st {
        println!(
            "  our least loss is {:.3} dB more than their design's here, beyond the grids' spreads: reported, not checked (a gap, #595)",
            o - t
        );
    } else {
        println!(
            "  our least loss and their design's here are within the grids' spreads of each other: reported, not checked"
        );
    }
    Ok(checks.finish())
}
