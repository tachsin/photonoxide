//! The photonics inverse-design testbed's waveguide mode converter: its published designs
//! re-evaluated on photonoxide's 2D FDFD, and a design of our own by topology optimization with a
//! 50 nm minimum length.
//!
//! M. Chen et al., "Validation and characterization of algorithms and software for photonics
//! inverse design", J. Opt. Soc. Am. B 41, A161 (2024),
//! [doi:10.1364/JOSAB.506412](https://doi.org/10.1364/JOSAB.506412), Section 3.C and Fig. 5; the
//! designs and the numbers of NanoComp/photonics-opt-testbed (MIT licence, commit a06518872de6),
//! `waveguide_mode_converter/README.md`, which Ceviche's FDFD computed at 10 nm.
//!
//! **The problem.** A 1.6 × 1.6 µm design region of silicon (ε 12.25) and oxide (ε 2.25) between
//! two 400 nm silicon waveguides in oxide, E along z: the input waveguide's fundamental mode in,
//! the output's second mode out, worst over 1265, 1270, 1275, 1285, 1290 and 1295 nm (the
//! testbed's run.py). The domain is the testbed's model's (ceviche-challenges'
//! mode_converter_spec_12 with the testbed's sizes): 350 × 300 cells of 10 nm, 20 cells of PML,
//! the waveguides 75 cells long, each port a 1.9 µm window. The S-parameters are photonoxide's
//! power-normalized ones, |S₁₁|² the reflection into the fundamental mode and |S₂₁|² the
//! conversion into the second.
//!
//! **The published designs.** Eight of the testbed's circle-brush designs (Ceviche's conditional
//! generator, 6 to 20 pixel brushes; the data in `src/cases/topology/testbed/`, whose minimum
//! lengths `topology/ruler-testbed` reproduces bit for bit): their worst transmission is checked
//! against the README's printed digits. Their worst reflection, −18 to −34 dB, is printed beside
//! the README's, not checked: it is set by the grid in both codes. Ours moves by +0.04, +0.17 and
//! +0.54 dB from 10 to 5 nm on the first three designs (and the transmission by −0.004, −0.005
//! and +0.009 dB), as much as it differs from Ceviche's (0.02 to 0.62 dB); `--full` repeats that
//! grid study.
//!
//! **Our design.** Densities on the design region's 160 × 160 pixels, a conic filter of 50 nm
//! (5 pixels) and a tanh projection at η = ½ (`topology`), the worst of reflection and
//! 1 − transmission over the six wavelengths minimized in its epigraph form (twelve constraints)
//! by genoxide's MMA, every gradient by the adjoint. The start breaks the mirror symmetry across
//! the guides' axis (a symmetric device can't turn the even mode into the odd one, and its
//! gradient there is zero). β rises from 8 to 256 in six stages of 30 evaluations; then the
//! minimum linewidth and spacing of 50 nm are imposed for 60 evaluations more, as Arrieta et al.
//! recommend (unconstrained first, then the constraints): Zhou et al.'s geometric constraints with
//! Arrieta et al.'s hyperparameters for a conic filter, R = ℓ_t = 50 nm, c = 64 R², and ε their
//! Eq. 18 for a strip across the region, each held as g/ε − 1 ≤ 0. The design is then binarized
//! at ½ and measured: its minimum length by the testbed's own metric (`topology::ruler`), its
//! spectrum by the same solve as the published designs'.
//!
//! **What is compared.** Chen et al. find the measured length within 20 % of the imposed one for
//! filter-and-projection designs, and local optima of the same length spread by about 15 % in
//! reflection and 10 to 30 % in transmission; so our design is checked for its length within 20 %
//! of 50 nm and for a reflection and a transmission within the range of the testbed's published
//! designs whose measured length is 55 to 76 nm (thirteen: −25.88 to −41.95 dB and −0.04 to
//! −0.26 dB), not against one of them.
//!
//! **What it found** (`--full`, 2026-11-11, 6.5 minutes on 12 threads here). The unconstrained
//! stages end at −30.79 dB and −0.156 dB; the constraints bring the worst to −32.74 dB and
//! −0.154 dB with both met (g/ε 0.83 and 0.99). Binarized and measured as the testbed measures its
//! designs, on the model's whole domain: 43.6 nm, −32.89 dB and −0.154 dB. The length is 13 %
//! short of the 50 nm imposed, within Chen et al.'s 20 % but short: Arrieta et al.'s ε is the
//! constraint's value at a straight strip, and a design's corners and curves can come out narrower
//! with the constraint met (an earlier run without the rounding below reached 55.0 nm). The
//! testbed's 50 nm design by Meep's filter and projection measures 55.26 nm at −33.33 dB and
//! −0.07 dB. The design is recorded in `examples/designs/optimize_mode_converter_50nm.hex`, which
//! the plain run measures; `--full` optimizes it again and checks it is the same, its values and
//! gradients rounded to 12 digits each evaluation so that a system's last bits can't steer it.
//!
//! ```sh
//! cargo run --release --example optimize_mode_converter                    # the published designs and ours, measured
//! cargo run --release --example optimize_mode_converter -- --full          # and the optimization and the 5 nm grid
//! cargo run --release --example optimize_mode_converter -- --full --write  # and record the design
//! ```

#![allow(clippy::disallowed_methods)] // until #822 converts it to `photonoxide::portable`

mod common;

use std::process::ExitCode;

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{
    Boundaries, Direction, Grid, Polarization, Port, PortMode, Side, Solver2d,
};
use photonoxide::topology::ruler::{Pattern, minimum_length};
use photonoxide::topology::{
    Density, Edge, Epigraph, LengthConstraints, Placement, Projection, geometric_threshold,
};
use photonoxide::units::Wavelength;

/// The testbed's wavelengths, nm.
const NM: [f64; 6] = [1265.0, 1270.0, 1275.0, 1285.0, 1290.0, 1295.0];
/// The domain in 10 nm cells, and the design region's first cell and size.
const DOMAIN: [usize; 2] = [350, 300];
const ORIGIN: [usize; 2] = [95, 70];
const SIDE: usize = 160;
/// ε of the oxide and the silicon.
const EPS: [f64; 2] = [2.25, 12.25];
/// The imposed minimum length, nm, and the filter's radius in pixels (Arrieta et al.: R = ℓ_t).
const LENGTH_NM: f64 = 50.0;
const RADIUS_PX: f64 = 5.0;
/// The continuation's β and evaluations per stage, and the constrained stage's evaluations.
const BETAS: [f64; 6] = [8.0, 16.0, 32.0, 64.0, 128.0, 256.0];
const PER_STAGE: usize = 30;
const CONSTRAINED: usize = 60;

/// The published designs: their file, their data, and the README's worst reflection and
/// transmission, dB.
const PUBLISHED: [(&str, &str, f64, f64); 8] = [
    (
        "circle_10_x47530832_w11_s483",
        include_str!(
            "../src/cases/topology/testbed/converter_generator_circle_10_x47530832_w11_s483.hex"
        ),
        -22.50,
        -0.19,
    ),
    (
        "circle_10_x47530832_w27_s681",
        include_str!(
            "../src/cases/topology/testbed/converter_generator_circle_10_x47530832_w27_s681.hex"
        ),
        -27.17,
        -0.19,
    ),
    (
        "circle_12_x47530832_w12_s248",
        include_str!(
            "../src/cases/topology/testbed/converter_generator_circle_12_x47530832_w12_s248.hex"
        ),
        -28.49,
        -0.35,
    ),
    (
        "circle_14_x47530832_w13_s793",
        include_str!(
            "../src/cases/topology/testbed/converter_generator_circle_14_x47530832_w13_s793.hex"
        ),
        -33.62,
        -0.19,
    ),
    (
        "circle_16_x47530832_w14_s150",
        include_str!(
            "../src/cases/topology/testbed/converter_generator_circle_16_x47530832_w14_s150.hex"
        ),
        -21.23,
        -0.81,
    ),
    (
        "circle_18_x47530832_w15_s879",
        include_str!(
            "../src/cases/topology/testbed/converter_generator_circle_18_x47530832_w15_s879.hex"
        ),
        -23.66,
        -0.93,
    ),
    (
        "circle_20_x47530832_w16_s416",
        include_str!(
            "../src/cases/topology/testbed/converter_generator_circle_20_x47530832_w16_s416.hex"
        ),
        -18.43,
        -1.23,
    ),
    (
        "circle_20_x47530832_w40_s988",
        include_str!(
            "../src/cases/topology/testbed/converter_generator_circle_20_x47530832_w40_s988.hex"
        ),
        -18.16,
        -1.34,
    ),
];

/// Our recorded design (`--full --write`).
const OURS: &str = include_str!("designs/optimize_mode_converter_50nm.hex");
const OURS_FILE: &str = "examples/designs/optimize_mode_converter_50nm.hex";

/// The published designs of measured length 55 to 76 nm: the range of their worst reflection
/// and transmission, dB (the README's thirteen).
const RANGE_R: f64 = -25.88;
const RANGE_T: f64 = -0.26;

/// A 160 × 160 design from rows of hex digits (each row along x, its first value the first
/// digit's highest bit).
fn from_hex(hex: &str) -> Vec<bool> {
    let mut out = vec![false; SIDE * SIDE];
    for (row, line) in hex.lines().enumerate() {
        for (k, c) in line.trim().chars().enumerate() {
            let nibble = c.to_digit(16).expect("a hex digit");
            for q in 0..4 {
                out[(4 * k + q) * SIDE + row] = nibble & (8 >> q) != 0;
            }
        }
    }
    out
}

/// A design's rows of hex digits, as [`from_hex`] reads them.
fn to_hex(design: &[bool]) -> String {
    let mut s = String::new();
    for row in 0..SIDE {
        for k in 0..SIDE / 4 {
            let mut nibble = 0u32;
            for q in 0..4 {
                nibble = nibble * 2 + u32::from(design[(4 * k + q) * SIDE + row]);
            }
            s.push(char::from_digit(nibble, 16).expect("a digit"));
        }
        s.push('\n');
    }
    s
}

/// The domain's permittivities with `design` (ε per design pixel) in its region, on a grid
/// `scale` times finer.
fn cells(design: &[f64], scale: usize) -> Vec<c64> {
    let [ni, nj] = DOMAIN.map(|n| n * scale);
    (0..ni * nj)
        .map(|p| {
            let (i, j) = ((p % ni) / scale, (p / ni) / scale);
            let inside = (ORIGIN[0]..ORIGIN[0] + SIDE).contains(&i)
                && (ORIGIN[1]..ORIGIN[1] + SIDE).contains(&j);
            let eps = if inside {
                design[(j - ORIGIN[1]) * SIDE + i - ORIGIN[0]]
            } else if (130..170).contains(&j) && (i < 95 || i >= DOMAIN[0] - 96) {
                EPS[1]
            } else {
                EPS[0]
            };
            c64::new(eps, 0.0)
        })
        .collect()
}

fn grid(scale: usize) -> Grid {
    let h = 0.01 / scale as f64;
    Grid {
        nx: DOMAIN[0] * scale,
        ny: DOMAIN[1] * scale,
        dx: h,
        dy: h,
        x0: 0.0,
        y0: 0.0,
    }
}

/// The two ports' modes at a wavelength, the fundamental on the left and the second on the right.
fn ports(solver: &Solver2d, scale: usize) -> photonoxide::Result<(PortMode, PortMode)> {
    let window = 55 * scale..245 * scale;
    let left = solver
        .port_modes_within(25 * scale, window.clone(), 1)?
        .remove(0);
    let right = solver.port_modes_within(325 * scale, window, 2)?.remove(1);
    Ok((left, right))
}

/// A binary design's worst reflection and transmission over the six wavelengths, dB, on a grid
/// `scale` times finer than 10 nm.
fn spectrum(design: &[bool], scale: usize) -> photonoxide::Result<(f64, f64)> {
    let eps: Vec<f64> = design.iter().map(|&s| EPS[usize::from(s)]).collect();
    let cells = cells(&eps, scale);
    let (mut r, mut t) = (f64::NEG_INFINITY, f64::INFINITY);
    for nm in NM {
        let lam = Wavelength::um(nm * 1e-3)?;
        let s = Solver2d::from_cells(
            grid(scale),
            Polarization::Ez,
            lam,
            &cells,
            Boundaries::pml(20 * scale),
        )?;
        let (left, right) = ports(&s, scale)?;
        let m = s.s_matrix(&[
            Port {
                mode: left,
                side: Side::Left,
            },
            Port {
                mode: right,
                side: Side::Right,
            },
        ])?;
        r = r.max(20.0 * m[0][0].norm().log10());
        t = t.min(20.0 * m[1][0].norm().log10());
    }
    Ok((r, t))
}

/// A design's minimum length, nm, measured as the testbed measures its designs: on the model's
/// whole domain, the waveguides and the cladding around the region (its README's lengths).
fn length(design: &[bool]) -> photonoxide::Result<f64> {
    let eps: Vec<f64> = design.iter().map(|&s| EPS[usize::from(s)]).collect();
    let solid = cells(&eps, 1)
        .iter()
        .map(|c| c.re > 0.5 * (EPS[0] + EPS[1]))
        .collect();
    minimum_length(&Pattern::new(DOMAIN, solid)?, [10.0, 10.0])
}

/// x rounded to 12 significant digits: an evaluation's values and gradients, so that a system's
/// last bits can't steer the optimizer.
fn round12(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let scale = 10f64.powi(11 - x.abs().log10().floor() as i32);
    if !scale.is_finite() {
        // below about 1e-297 the scale overflows: such a value is kept as it is
        return x;
    }
    (x * scale).round() / scale
}

/// The optimization's problem at one β: the density and, per wavelength, the solver at the
/// background, the ports' modes and the second mode's power over the first's.
struct Converter {
    density: Density,
    placement: Placement,
    background: Vec<c64>,
    bands: Vec<(Wavelength, Solver2d, PortMode, PortMode, f64)>,
}

impl Converter {
    fn new(beta: f64) -> photonoxide::Result<Converter> {
        let background = cells(&vec![EPS[0]; SIDE * SIDE], 1);
        let density = Density::new([SIDE, SIDE, 1], 0.01, EPS)?
            .with_filter(RADIUS_PX * 0.01, Edge::Renormalized)?
            .with_projection(Projection::new(beta, 0.5)?);
        let mut bands = Vec::new();
        for nm in NM {
            let lam = Wavelength::um(nm * 1e-3)?;
            let s = Solver2d::from_cells(
                grid(1),
                Polarization::Ez,
                lam,
                &background,
                Boundaries::pml(20),
            )?;
            let (left, right) = ports(&s, 1)?;
            let ratio = right.power(0.01) / left.power(0.01);
            bands.push((lam, s, left, right, ratio));
        }
        Ok(Converter {
            density,
            placement: Placement {
                origin: [ORIGIN[0], ORIGIN[1], 0],
                shape: [SIDE, SIDE, 1],
                grid: [DOMAIN[0], DOMAIN[1], 1],
            },
            background,
            bands,
        })
    }

    /// The twelve objectives (R and 1 − T per wavelength) and their gradients in the densities,
    /// rounded to 12 digits.
    fn evaluate(&self, rho: &[f64]) -> photonoxide::Result<(Vec<f64>, Vec<f64>)> {
        let mut cells = self.background.clone();
        self.placement
            .embed(&self.density.permittivity(rho)?, &mut cells)?;
        let (mut values, mut gradients) = (Vec::new(), Vec::new());
        for (lam, base, left, right, ratio) in &self.bands {
            let s = base.reuse_cells(*lam, &cells)?;
            let field = s.solve_system(&s.mode_source(left, Direction::Forward))?;
            let (r, dr) = s.mode_power_gradient(&field, left, Direction::Backward)?;
            let (t, dt) = s.mode_power_gradient(&field, right, Direction::Forward)?;
            let dr = self.density.gradient(rho, &self.placement.restrict(&dr)?)?;
            let dt = self.density.gradient(rho, &self.placement.restrict(&dt)?)?;
            values.push(round12(r));
            gradients.extend(dr.iter().map(|&d| round12(d)));
            values.push(round12(1.0 - t * ratio));
            gradients.extend(dt.iter().map(|&d| round12(-d * ratio)));
        }
        Ok((values, gradients))
    }
}

/// More constraints beside the epigraph's: at the densities, their values and Jacobian rows.
type Extra<'a> = &'a (dyn Fn(&[f64], &mut [f64], &mut [f64]) + Sync);

/// The optimization: β from 8 to 256, then the length constraints. Returns the design, binarized,
/// and what each stage reached.
fn optimize() -> photonoxide::Result<Vec<bool>> {
    use genoxide::prelude::*;
    let n = SIDE * SIDE;
    // a start that breaks the mirror symmetry across the guides' axis
    let mut rho: Vec<f64> = (0..n)
        .map(|p| {
            let (i, j) = (
                (p % SIDE) as f64 / SIDE as f64,
                (p / SIDE) as f64 / SIDE as f64,
            );
            0.5 + 0.25 * (std::f64::consts::PI * (j - 0.5)).sin() * (std::f64::consts::PI * i).sin()
        })
        .collect();
    let form = Epigraph::new(n, 12)?;
    let start = std::time::Instant::now();
    let worst = |c: &Converter, rho: &[f64]| -> photonoxide::Result<(f64, f64)> {
        let (v, _) = c.evaluate(rho)?;
        let r = v.iter().step_by(2).copied().fold(0.0, f64::max);
        let t = v
            .iter()
            .skip(1)
            .step_by(2)
            .map(|x| 1.0 - x)
            .fold(1.0, f64::min);
        Ok((10.0 * r.log10(), 10.0 * t.log10()))
    };
    let run = |c: &Converter,
               rho: &[f64],
               extra: usize,
               evaluations: usize,
               more: Extra|
     -> photonoxide::Result<Vec<f64>> {
        let (values, _) = c.evaluate(rho)?;
        let mut y = rho.to_vec();
        y.push(Epigraph::start(&values));
        let m = 12 + extra;
        let fitness = Constrained::differentiable(
            m,
            |x: &Reals, gradient: &mut [f64], g: &mut [f64], jacobian: &mut [f64]| {
                let Ok((values, gradients)) = c.evaluate(&x[..n]) else {
                    return f64::NAN;
                };
                let w = n + 1;
                if form
                    .evaluate(
                        x,
                        &values,
                        &gradients,
                        gradient,
                        &mut g[..12],
                        &mut jacobian[..12 * w],
                    )
                    .is_err()
                {
                    return f64::NAN;
                }
                more(&x[..n], &mut g[12..], &mut jacobian[12 * w..]);
                x[n]
            },
        );
        let mma = Mma::builder(Real::new(vec![0.0..=1.0; n + 1]).expect("bounds"))
            .initial_genome(Reals::from(y))
            .minimize()
            .build()
            .expect("an MMA");
        let outcome = Engine::new(mma, fitness)
            .stop_when(Stop::evaluations(evaluations as u64))
            .run()
            .expect("a run");
        Ok(outcome.best_genome()[..n].to_vec())
    };
    for beta in BETAS {
        let c = Converter::new(beta)?;
        rho = run(&c, &rho, 0, PER_STAGE, &|_, _, _| {})?;
        let (r, t) = worst(&c, &rho)?;
        println!(
            "    β {beta:>3}: worst R {r:.2} dB, worst T {t:.3} dB ({:.0} s)",
            start.elapsed().as_secs_f64()
        );
    }
    let c = Converter::new(BETAS[BETAS.len() - 1])?;
    let constraints = LengthConstraints::new(&c.density, LENGTH_NM * 1e-3, LENGTH_NM * 1e-3)?
        .with_arrieta_decay();
    let eps = geometric_threshold(64.0, LENGTH_NM / (SIDE as f64 * 10.0))?;
    let more = |x: &[f64], g: &mut [f64], jacobian: &mut [f64]| {
        let Ok(l) = constraints.evaluate(x) else {
            g.fill(f64::NAN);
            return;
        };
        for s in 0..2 {
            g[s] = round12(l.values[s] / eps - 1.0);
            let row = &mut jacobian[s * (n + 1)..(s + 1) * (n + 1)];
            for (r, d) in row[..n].iter_mut().zip(&l.gradients[s]) {
                *r = round12(d / eps);
            }
        }
    };
    rho = run(&c, &rho, 2, CONSTRAINED, &more)?;
    let (r, t) = worst(&c, &rho)?;
    let l = constraints.evaluate(&rho)?;
    println!(
        "    50 nm constraints: worst R {r:.2} dB, worst T {t:.3} dB, g_LW/ε {:.2}, g_LS/ε {:.2} ({:.0} s)",
        l.values[0] / eps,
        l.values[1] / eps,
        start.elapsed().as_secs_f64()
    );
    Ok(c.density.physical(&rho)?.iter().map(|&v| v > 0.5).collect())
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let args: Vec<String> = std::env::args().collect();
    let full = args.iter().any(|a| a == "--full");
    let write = args.iter().any(|a| a == "--write");
    let mut checks = common::Checks::default();
    let mut setup = common::Setup::new();
    setup.item(
        "domain",
        "350 × 300 cells of 10 nm (the testbed's model), 20 cells of PML; a 1.6 × 1.6 µm design region from (0.95, 0.70) µm between two 400 nm waveguides 0.75 µm long",
    );
    setup.medium("silicon", &common::eps(EPS[1]));
    setup.medium("oxide", &common::eps(EPS[0]));
    setup.item("wavelengths", "1265, 1270, 1275, 1285, 1290 and 1295 nm");
    setup.item(
        "ports",
        "1.9 µm windows at x = 0.25 and 3.25 µm: the fundamental mode in on the left, the second mode out on the right; E along z",
    );
    setup.item(
        "our design",
        "160 × 160 densities, a conic filter of 50 nm, tanh projection at η = ½, β 8 to 256 in six stages of 30 evaluations, then 50 nm linewidth and spacing (Zhou et al., Arrieta et al.'s c = 64 R² and ε) for 60; genoxide's MMA on the epigraph of the worst R and 1 − T",
    );
    setup.item(
        "tolerance",
        "the published transmissions to the README's printed digits (0.005 dB); our length within 20 % of 50 nm (Chen et al.); our R and T within the published range at 55 to 76 nm",
    );
    setup.print();

    println!(
        "The testbed's published designs, worst over the six wavelengths, ours at 10 nm against the README's (Ceviche, 10 nm):"
    );
    for (name, hex, r_printed, t_printed) in PUBLISHED {
        let design = from_hex(hex);
        let (r, t) = spectrum(&design, 1)?;
        println!("  {name}: R {r:.2} dB (README {r_printed:.2}), T {t:.3} dB");
        checks.compare(&format!("T, {name} (dB)"), t, t_printed, 0.005);
    }
    if full {
        println!("  10 to 5 nm, the first three (each pixel 2 × 2 cells):");
        for (name, hex, _, _) in PUBLISHED.iter().take(3) {
            let design = from_hex(hex);
            let (r1, t1) = spectrum(&design, 1)?;
            let (r2, t2) = spectrum(&design, 2)?;
            println!("    {name}: R {:+.2} dB, T {:+.3} dB", r2 - r1, t2 - t1);
        }
    } else {
        println!(
            "  10 to 5 nm, the first three (recorded by --full): R +0.04, +0.17, +0.54 dB; T −0.004, −0.005, +0.009 dB"
        );
    }

    if full {
        println!("Our design, optimized:");
        let design = optimize()?;
        let differing = design
            .iter()
            .zip(from_hex(OURS))
            .filter(|(a, b)| **a != *b)
            .count();
        if write {
            std::fs::write(OURS_FILE, to_hex(&design)).expect("the design written");
            println!("  written to {OURS_FILE}");
        } else {
            checks.count("pixels unlike the recorded design", differing, 0);
        }
    }
    let ours = from_hex(OURS);
    let len = length(&ours)?;
    let (r, t) = spectrum(&ours, 1)?;
    println!(
        "Our design (recorded), binarized: minimum length {len:.1} nm, worst R {r:.2} dB, worst T {t:.3} dB"
    );
    checks.compare("minimum length (nm)", len, LENGTH_NM, 0.2 * LENGTH_NM);
    checks.at_most("worst R (dB)", r, RANGE_R);
    checks.at_most("worst insertion loss, −T (dB)", -t, -RANGE_T);
    Ok(checks.finish())
}
