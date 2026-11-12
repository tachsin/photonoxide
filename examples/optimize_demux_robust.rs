//! Christiansen & Sigmund's robust demultiplexer: a 2 µm silicon region in air sending 1300 nm
//! to one waveguide and 1550 nm to another, designed so that its eroded and dilated versions,
//! a near-uniform etch of ±8 nm, work as well as the nominal one.
//!
//! R. E. Christiansen, O. Sigmund, "Inverse design in photonics by topology optimization:
//! tutorial", J. Opt. Soc. Am. B 38, 496 (2021),
//! [doi:10.1364/JOSAB.406048](https://doi.org/10.1364/JOSAB.406048), Section 9, Fig. 7,
//! Tables 4–7 and Eq. 22; B. S. Lazarov, O. Sigmund, Int. J. Numer. Methods Eng. 86, 765
//! (published online 2010), [doi:10.1002/nme.3072](https://doi.org/10.1002/nme.3072), the
//! Helmholtz filter.
//!
//! **The problem** (Fig. 7 and Tables 4–5). A domain of 6 × 3 µm of air (n = 1) with a 2 × 2 µm
//! design region at its centre, silicon n = 3.48, E along z. An input waveguide 299 nm thick on
//! the axis runs 2 µm from the left edge to the region; two output waveguides run 2 µm from the
//! region to the right edge: one 299 nm thick 511 nm above the axis, for 1300 nm, and one 356 nm
//! thick 483 nm below it, for 1550 nm (Fig. 7 draws d_WG,1 and d_WG,2 from the axis to the
//! waveguides; taken here to their centres). 10 nm cells everywhere (theirs: 10 nm in the
//! design region), a cell a waveguide's edge crosses given its share of silicon.
//!
//! **The design** (Eq. 22 and Table 6). Densities on the region's 200 × 200 pixels, filtered by
//! a Helmholtz filter of 100 nm, projected at η = ½, filtered again by one of 50 nm and projected
//! at three thresholds, 0.7, 0.5 and 0.3: the eroded, nominal and dilated realizations, each solved
//! at both wavelengths, six problems (`topology::DoubleFiltered`). The second filter moves a
//! straight edge by r ln(1/2η) = 7.4 nm at η = 0.3: their ±8 nm (`topology/helmholtz-edge`).
//!
//! **What differs from the paper, and why.** They solve by finite elements with a PML on the
//! left and first-order absorbing boundaries on the other three sides, launch a Gaussian beam
//! into the input waveguide, and count the power flowing through each output's end; here 2D FDFD
//! with PMLs all round, the input waveguide's fundamental mode launched, and the power in each
//! output's fundamental mode, so the transmission is a share of the power the mode carried in.
//! They maximize a p-norm of the six transmissions (P = 2) with an artificial damping term,
//! iξ(1 − ξ), in the permittivity; here genoxide's MMA maximizes the worst of the six by its
//! epigraph form, with a real permittivity (the term vanishes on a binary design). β rises from
//! their (5, 2.5) to (160, 80) in six stages of 80 evaluations (theirs: 1000 iterations).
//!
//! **What is compared.** Their Table 7 gives each realization's transmission relative to the
//! power entering through the input waveguide (T_Rel): 0.86, 0.69 and 0.82 at 1300 nm and 0.83,
//! 0.85 and 0.81 at 1550 nm for the dilated, nominal and eroded designs, "≈" to two digits.
//! Ours, each realization binarized: T/(1 − R), the wanted output's mode over what wasn't
//! reflected, against each of theirs as a target to meet (the same device and etch, by other
//! means); beside it the power in the other output's mode (crosstalk) and the total flux through
//! the outputs' plane, which checks the modes' bookkeeping against the field's own power flow.
//!
//! **What it found** (`--full`, 2026-11-11, 23 minutes on 12 threads here). All six cases reach
//! 0.980 to 0.986 of the power that entered (T/(1 − R)), against Table 7's 0.69 to 0.86; the other
//! output gets at most 0.14 %, the flux through the outputs' plane (0.982 to 0.990) is the modes'
//! and a little more, and at 5 nm every value is within 0.002 of the 10 nm one. Ours meets Table 7
//! in every case, by margins the differences above can't separate from the method: PMLs and the
//! mode against their absorbing boundaries and Gaussian beam, the worst case against their p-norm.
//! An earlier run with 40 evaluations a stage ended at 0.61 to 0.89: the effort mattered. The three
//! realizations are recorded in `examples/designs/optimize_demux_robust.hex`, which the plain run
//! solves; `--full` optimizes again and checks it repeats them pixel for pixel, its values and
//! gradients rounded to 12 digits so that a system's last bits can't steer it, and solves them
//! again at 5 nm.
//!
//! ```sh
//! cargo run --release --example optimize_demux_robust                    # the recorded design, solved
//! cargo run --release --example optimize_demux_robust -- --full          # the optimization, and 5 nm
//! cargo run --release --example optimize_demux_robust -- --full --write  # and record the design
//! ```

mod common;

use std::process::ExitCode;

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{Boundaries, Direction, Grid, Polarization, PortMode, Solver2d};
use photonoxide::topology::{DoubleFiltered, Epigraph, Placement, Projection};
use photonoxide::units::Wavelength;

/// Ω in nm, and the cells of PML around it at 10 nm.
const OMEGA_NM: [f64; 2] = [6000.0, 3000.0];
const PML: usize = 20;
/// Silicon's permittivity, n = 3.48; air's is 1.
const SI: f64 = 3.48 * 3.48;
/// The design region's pixels a side.
const SIDE: usize = 200;
/// The two wavelengths, nm, and which output each goes to (1: above, 2: below).
const BANDS: [(f64, usize); 2] = [(1300.0, 1), (1550.0, 2)];
/// The continuation's (β₁, β₂) and the evaluations per stage.
const BETAS: [[f64; 2]; 6] = [
    [5.0, 2.5],
    [10.0, 5.0],
    [20.0, 10.0],
    [40.0, 20.0],
    [80.0, 40.0],
    [160.0, 80.0],
];
const PER_STAGE: u64 = 80;
/// Table 7's T_Rel: (dilated, nominal, eroded) at 1300 and at 1550 nm.
const TABLE_7: [[f64; 3]; 2] = [[0.86, 0.69, 0.82], [0.83, 0.85, 0.81]];
const NAMES: [&str; 3] = ["eroded", "nominal", "dilated"];

const RECORDED: &str = include_str!("designs/optimize_demux_robust.hex");
const RECORDED_FILE: &str = "examples/designs/optimize_demux_robust.hex";

/// The grid at `scale` times 10 nm, Ω and its PML.
fn grid(scale: usize) -> Grid {
    let h = 0.01 / scale as f64;
    let pml = (PML * scale) as f64 * h;
    Grid {
        nx: (OMEGA_NM[0] / 10.0) as usize * scale + 2 * PML * scale,
        ny: (OMEGA_NM[1] / 10.0) as usize * scale + 2 * PML * scale,
        dx: h,
        dy: h,
        x0: -OMEGA_NM[0] * 5e-4 - pml,
        y0: -OMEGA_NM[1] * 5e-4 - pml,
    }
}

/// The share of a cell from `y` to `y + h` (µm) inside [lo, hi] (nm).
fn share(y: f64, h: f64, lo: f64, hi: f64) -> f64 {
    let (lo, hi) = (lo * 1e-3, hi * 1e-3);
    ((hi.min(y + h) - lo.max(y)) / h).clamp(0.0, 1.0)
}

/// The domain's permittivities on `g`, the design region `design` (one value per design pixel,
/// on 10 nm pixels; each cell takes its pixel's) or air.
fn cells(g: &Grid, design: Option<&[f64]>) -> Vec<c64> {
    let scale = (0.01 / g.dx).round() as usize;
    (0..g.nx * g.ny)
        .map(|p| {
            let (i, j) = (p % g.nx, p / g.nx);
            let (x, y) = (g.x0 + i as f64 * g.dx, g.y0 + j as f64 * g.dy);
            let s = if x < -1.0 - 1e-9 {
                share(y, g.dy, -149.5, 149.5)
            } else if x >= 1.0 - 1e-9 {
                share(y, g.dy, 511.0 - 149.5, 511.0 + 149.5)
                    + share(y, g.dy, -483.0 - 178.0, -483.0 + 178.0)
            } else if (-1.0 - 1e-9..1.0 - 1e-9).contains(&y) {
                let (a, b) = (
                    ((x + 1.0) / g.dx).round() as usize / scale,
                    ((y + 1.0) / g.dy).round() as usize / scale,
                );
                design.map_or(0.0, |d| d[b.min(SIDE - 1) * SIDE + a.min(SIDE - 1)])
            } else {
                0.0
            };
            c64::new(1.0 + s * (SI - 1.0), 0.0)
        })
        .collect()
}

/// The row of `y_nm` on `g`.
fn row(g: &Grid, y_nm: f64) -> usize {
    ((y_nm * 1e-3 - g.y0) / g.dy).round() as usize
}

/// The column of `x_nm` on `g`.
fn column(g: &Grid, x_nm: f64) -> usize {
    ((x_nm * 1e-3 - g.x0) / g.dx).round() as usize
}

/// The ports at a wavelength: the input's fundamental mode at x = −2.5 µm, and each output's at
/// x = +2.5 µm, each in a window around its waveguide.
fn ports(s: &Solver2d, g: &Grid) -> photonoxide::Result<[PortMode; 3]> {
    let input = s
        .port_modes_within(column(g, -2500.0), row(g, -700.0)..row(g, 700.0), 1)?
        .remove(0);
    let up = s
        .port_modes_within(column(g, 2500.0), row(g, 60.0)..row(g, 1100.0), 1)?
        .remove(0);
    let down = s
        .port_modes_within(column(g, 2500.0), row(g, -1100.0)..row(g, -5.0), 1)?
        .remove(0);
    Ok([input, up, down])
}

/// x rounded to 12 significant digits.
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

/// A binary realization solved on a grid `scale` times finer: per wavelength, the wanted
/// output's share, the reflection, the other output's share, and the total flux through the
/// outputs' plane, each over the input mode's power.
fn solve(design: &[bool], scale: usize) -> photonoxide::Result<Vec<[f64; 4]>> {
    let g = grid(scale);
    let d: Vec<f64> = design.iter().map(|&b| f64::from(u8::from(b))).collect();
    let eps = cells(&g, Some(&d));
    let mut out = Vec::new();
    for (nm, wanted) in BANDS {
        let lam = Wavelength::um(nm * 1e-3)?;
        let s = Solver2d::from_cells(g, Polarization::Ez, lam, &eps, Boundaries::pml(PML * scale))?;
        let [input, up, down] = ports(&s, &g)?;
        let field = s.solve_system(&s.mode_source(&input, Direction::Forward))?;
        let p_in = input.power(g.dy);
        let share = |m: &PortMode| field.mode_amplitudes(m).0.norm_sqr() * m.power(g.dy) / p_in;
        let (t, other) = if wanted == 1 {
            (share(&up), share(&down))
        } else {
            (share(&down), share(&up))
        };
        let r = field.mode_amplitudes(&input).1.norm_sqr();
        let flux = field.flux_x(column(&g, 2500.0)) / p_in;
        out.push([t, r, other, flux]);
    }
    Ok(out)
}

/// The optimization's problem at one pair of β.
struct Demux {
    pipeline: DoubleFiltered,
    placement: Placement,
    background: Vec<c64>,
    /// per wavelength: the solver at the background, the input's mode, the wanted output's, and
    /// the latter's power over the former's
    bands: Vec<(Wavelength, Solver2d, PortMode, PortMode, f64)>,
}

impl Demux {
    fn new(betas: [f64; 2]) -> photonoxide::Result<Demux> {
        let g = grid(1);
        let background = cells(&g, None);
        let pipeline = DoubleFiltered::new(
            [SIDE, SIDE, 1],
            0.01,
            [1.0, SI],
            [0.1, 0.05],
            [
                Projection::new(betas[0], 0.5)?,
                Projection::new(betas[1], 0.5)?,
            ],
        )?;
        let mut bands = Vec::new();
        for (nm, wanted) in BANDS {
            let lam = Wavelength::um(nm * 1e-3)?;
            let s =
                Solver2d::from_cells(g, Polarization::Ez, lam, &background, Boundaries::pml(PML))?;
            let [input, up, down] = ports(&s, &g)?;
            let output = if wanted == 1 { up } else { down };
            let ratio = output.power(g.dy) / input.power(g.dy);
            bands.push((lam, s, input, output, ratio));
        }
        Ok(Demux {
            pipeline,
            placement: Placement {
                origin: [column(&g, -1000.0), row(&g, -1000.0), 0],
                shape: [SIDE, SIDE, 1],
                grid: [g.nx, g.ny, 1],
            },
            background,
            bands,
        })
    }

    /// The six objectives, 1 − T for each realization (eroded, nominal, dilated) and wavelength,
    /// and their gradients, rounded to 12 digits.
    fn evaluate(&self, rho: &[f64]) -> photonoxide::Result<(Vec<f64>, Vec<f64>)> {
        let (mut values, mut gradients) = (Vec::new(), Vec::new());
        for p in self.pipeline.robust(0.3)? {
            let mut cells = self.background.clone();
            self.placement.embed(&p.permittivity(rho)?, &mut cells)?;
            for (lam, base, input, output, ratio) in &self.bands {
                let s = base.reuse_cells(*lam, &cells)?;
                let field = s.solve_system(&s.mode_source(input, Direction::Forward))?;
                let (t, dt) = s.mode_power_gradient(&field, output, Direction::Forward)?;
                let g = p.gradient(rho, &self.placement.restrict(&dt)?)?;
                values.push(round12(1.0 - t * ratio));
                gradients.extend(g.iter().map(|&v| round12(-v * ratio)));
            }
        }
        Ok((values, gradients))
    }
}

/// The optimization; the three realizations binarized.
fn optimize() -> photonoxide::Result<[Vec<bool>; 3]> {
    use genoxide::prelude::*;
    let n = SIDE * SIDE;
    // a start that breaks the mirror symmetry across the axis, which a demultiplexer must
    let mut rho: Vec<f64> = (0..n)
        .map(|p| {
            let (i, j) = (
                (p % SIDE) as f64 / SIDE as f64,
                (p / SIDE) as f64 / SIDE as f64,
            );
            0.5 + 0.25 * (std::f64::consts::PI * (j - 0.5)).sin() * (std::f64::consts::PI * i).sin()
        })
        .collect();
    let form = Epigraph::new(n, 6)?;
    let start = std::time::Instant::now();
    let mut last = None;
    for betas in BETAS {
        let d = Demux::new(betas)?;
        let (values, _) = d.evaluate(&rho)?;
        let mut y = rho.clone();
        y.push(Epigraph::start(&values));
        let fitness = Constrained::differentiable(
            6,
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
        let worst = values.iter().copied().fold(0.0, f64::max);
        println!(
            "    β ({}, {}): worst T {:.3} ({:.0} s)",
            betas[0],
            betas[1],
            1.0 - worst,
            start.elapsed().as_secs_f64()
        );
        last = Some(d);
    }
    let d = last.expect("a stage");
    let r = d.pipeline.robust(0.3)?;
    let binary = |p: &DoubleFiltered| -> photonoxide::Result<Vec<bool>> {
        Ok(p.physical(&rho)?.iter().map(|&v| v > 0.5).collect())
    };
    Ok([binary(&r[0])?, binary(&r[1])?, binary(&r[2])?])
}

/// The three realizations as hex rows (each row along x, 50 digits), blocks apart by a line.
fn to_hex(designs: &[Vec<bool>; 3]) -> String {
    let mut s = String::new();
    for (k, d) in designs.iter().enumerate() {
        if k > 0 {
            s.push('\n');
        }
        for j in 0..SIDE {
            for c in 0..SIDE / 4 {
                let mut nibble = 0u32;
                for q in 0..4 {
                    nibble = nibble * 2 + u32::from(d[j * SIDE + 4 * c + q]);
                }
                s.push(char::from_digit(nibble, 16).expect("a digit"));
            }
            s.push('\n');
        }
    }
    s
}

fn from_hex(hex: &str) -> Vec<Vec<bool>> {
    hex.split("\n\n")
        .filter(|b| !b.trim().is_empty())
        .map(|block| {
            let mut d = vec![false; SIDE * SIDE];
            for (j, line) in block.lines().enumerate() {
                for (c, ch) in line.trim().chars().enumerate() {
                    let nibble = ch.to_digit(16).expect("a hex digit");
                    for q in 0..4 {
                        d[j * SIDE + 4 * c + q] = nibble & (8 >> q) != 0;
                    }
                }
            }
            d
        })
        .collect()
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let args: Vec<String> = std::env::args().collect();
    let full = args.iter().any(|a| a == "--full");
    let write = args.iter().any(|a| a == "--write");
    let mut checks = common::Checks::default();
    let mut setup = common::Setup::new();
    setup.item(
        "domain",
        "6 × 3 µm of air, 10 nm cells, PMLs of 20 cells all round; a 2 × 2 µm design region at the centre",
    );
    setup.medium("silicon", &common::eps(SI));
    setup.medium("air", &common::eps(1.0));
    setup.item(
        "waveguides",
        "input 299 nm thick on the axis; outputs 299 nm thick 511 nm above it (1300 nm) and 356 nm thick 483 nm below (1550 nm), centres; a cell an edge crosses has its share of silicon",
    );
    setup.item("wavelengths", "1300 and 1550 nm; E along z");
    setup.item(
        "ports",
        "the input's fundamental mode launched at x = −2.5 µm; each output's fundamental mode at x = 2.5 µm",
    );
    setup.item(
        "design",
        "200 × 200 densities, Helmholtz filters of 100 and 50 nm, projections at η = ½ and at 0.7, 0.5 and 0.3 (eroded, nominal, dilated); genoxide's MMA on the worst of the six, β (5, 2.5) to (160, 80), six stages of 80",
    );
    setup.item(
        "tolerance",
        "each realization's T/(1 − R) at least Table 7's T_Rel for it",
    );
    setup.print();

    let designs: Vec<Vec<bool>> = if full {
        println!("The optimization:");
        let d = optimize()?;
        let recorded = from_hex(RECORDED);
        if write {
            std::fs::write(RECORDED_FILE, to_hex(&d)).expect("the design written");
            println!("  written to {RECORDED_FILE}");
        } else {
            let differing: usize = d
                .iter()
                .zip(&recorded)
                .map(|(a, b)| a.iter().zip(b).filter(|(x, y)| x != y).count())
                .sum();
            checks.count("pixels unlike the recorded design", differing, 0);
        }
        d.to_vec()
    } else {
        from_hex(RECORDED)
    };
    println!(
        "Each realization binarized, at 10 nm: the wanted output's mode over the input's (T), its reflection (R), T/(1 − R) against Table 7, the other output's mode, and the flux through the outputs' plane:"
    );
    for (k, (name, d)) in NAMES.iter().zip(&designs).enumerate() {
        for (b, [t, r, other, flux]) in solve(d, 1)?.into_iter().enumerate() {
            let theirs = TABLE_7[b][2 - k];
            let rel = t / (1.0 - r);
            println!(
                "  {name} at {} nm: T {t:.3}, R {r:.4}, T/(1 − R) {rel:.3} (Table 7 ≈ {theirs}), other output {other:.4}, flux {flux:.3}",
                BANDS[b].0
            );
            checks.at_most(
                &format!("{name}, {} nm: 1 − T_Rel", BANDS[b].0),
                1.0 - rel,
                ((1.0 - theirs) * 100.0).round() / 100.0,
            );
        }
    }
    if full {
        println!("At 5 nm:");
        for (name, d) in NAMES.iter().zip(&designs) {
            for (b, [t, r, _, _]) in solve(d, 2)?.into_iter().enumerate() {
                println!(
                    "  {name} at {} nm: T/(1 − R) {:.3}",
                    BANDS[b].0,
                    t / (1.0 - r)
                );
            }
        }
    }
    Ok(checks.finish())
}
