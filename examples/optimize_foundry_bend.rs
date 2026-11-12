//! Hammond et al.'s broadband 90° bend under a foundry's design rules: minimum linewidth and
//! spacing of 90 nm, a minimum area of 0.08 µm² and a minimum enclosed area of 0.2 µm², designed by
//! topology optimization over ten wavelengths.
//!
//! A. M. Hammond, A. Oskooi, S. G. Johnson, S. E. Ralph, "Photonic topology optimization with
//! semiconductor-foundry design-rule constraints", Opt. Express 29, 23916 (2021),
//! [doi:10.1364/OE.431188](https://doi.org/10.1364/OE.431188), Sections 3.1, 4, 6 and 6.2,
//! Eqs. 10–15, 21–22, 33 and 35, Figs. 8 and 9.
//!
//! **The problem** (Section 6 and 6.2). A 90° bend in a silicon waveguide (n = 3.4) in oxide
//! (n = 1.44) through a 3 × 3 µm design region, the worst over ten wavelengths from 1.50 to
//! 1.60 µm of 1 − |α₁⁺|², the power left in the output's fundamental mode (their Eq. 35), by the
//! epigraph (their Eq. 33). The paper doesn't state the waveguide's width, the polarization or
//! where the guides meet the region: here a 0.5 µm guide, E along z (2D), the input entering the
//! region's west side at its middle and the output leaving its north side at its middle; 2D FDFD
//! on 20 nm cells (theirs: Meep at 33 nm, the design at 17 nm). The output runs along y, so its
//! mode is measured on two rows (`Solver2d::row_mode_functional`, `fdfd/adjoint-gradient-row`).
//!
//! **The rules and the schedule** (Sections 3.1, 4 and 6; Fig. 8's rulebook). A conic filter of
//! 135 nm, 1.5 times the rules' 90 nm, and a projection at η = ½; 70 MMA evaluations at β = 8 and
//! 16 and 150 at 32, the rules imposed only at β = 32, as theirs (theirs: 70 at each). The linewidth
//! and spacing by Zhou et al.'s geometric constraints with Arrieta et al.'s hyperparameters
//! (c = 64 R², ε of their Eq. 18 for a strip across the region, R the filter's radius), each held as g/ε − 1 ≤ 0, where Hammond et al. hold
//! theirs to G = 10⁻⁵ t; the minimum area and enclosed area by their Eqs. 21–22
//! (`topology::AreaConstraints`) held to their G = 10⁻⁵ t, shrinking with the objective t.
//!
//! **What is compared.** Their Fig. 9's nine bends, one per rulebook, all have an average
//! transmission above 89 % (Section 6.2); the figure prints each one's, not this rulebook's alone.
//! Ours: the band-average transmission at least 0.89; the measured minimum length (the testbed's
//! metric, `topology::ruler`) at least 90 nm, less Chen et al.'s 20 % for the metric (the rule is
//! a minimum: a longer one keeps it); no island or hole left below its minimum.
//!
//! **What it found** (`--full`, 2026-11-12, 8 minutes on 12 threads here). The unconstrained
//! stages reach a worst transmission of 0.998 at β = 16; with the rules, 0.999 at β = 32. The
//! design, binary: 0.998 to 0.999 over the band (mean 0.998), a measured minimum length of 110 nm,
//! and no island or hole below its minimum, against every rulebook's 89 % in Hammond et al.'s
//! Fig. 9 (their most constrained, 98.2 %, is another rulebook's). The 0.5 µm guide carries three
//! modes in 2D (E along z), and the transmission is into the fundamental alone. What the run
//! needed: each stage keeps its own best design, its start included, ranked as the topology job
//! ranks them (one that meets every rule first, then the least worst objective, then the least
//! violation), since MMA's iterates sit just outside the epigraph's constraints and genoxide's
//! best feasible genome stayed the start; the area constraints counted in pixels for MMA's
//! scaling (g n ≤ a t n, the same rule); and a filter wider than the rules: with a 90 nm filter
//! the unconstrained stages' design has 58 nm features, and no evaluation of the ruled stage met
//! every rule in 300. The design is recorded in `examples/designs/optimize_foundry_bend.hex`,
//! which the plain run solves; `--full` optimizes it again and checks it repeats it pixel for
//! pixel, its values and gradients rounded to 12 digits.
//!
//! ```sh
//! cargo run --release --example optimize_foundry_bend                    # the recorded design, solved
//! cargo run --release --example optimize_foundry_bend -- --full          # the optimization
//! cargo run --release --example optimize_foundry_bend -- --full --write  # and record the design
//! ```

mod common;

use std::process::ExitCode;

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{Boundaries, Direction, Grid, Polarization, PortMode, Solver2d};
use photonoxide::topology::ruler::{Pattern, minimum_length};
use photonoxide::topology::{
    AreaConstraints, Density, Edge, Epigraph, LengthConstraints, Placement, Projection,
    geometric_threshold,
};
use photonoxide::units::Wavelength;

/// The cell, µm, the PML in cells, and the design region's pixels a side.
const H: f64 = 0.02;
const PML: usize = 20;
const SIDE: usize = 150;
/// The interior: x from −1.5 to 3.5 µm, y from −0.5 to 4.5 µm; the region from (0, 0) to (3, 3).
const X0: f64 = -1.5;
const Y0: f64 = -0.5;
const SPAN: f64 = 5.0;
/// Silicon's and oxide's permittivities.
const SI: f64 = 3.4 * 3.4;
const OX: f64 = 1.44 * 1.44;
/// The guide's half-width and centre, µm.
const HALF: f64 = 0.25;
const CENTRE: f64 = 1.5;
/// The rules: linewidth and spacing (µm), minimum area and enclosed area (µm²).
const LINE: f64 = 0.09;
/// The conic filter's radius, µm: 1.5 times the rules' length, so that the unconstrained stages'
/// features are near the rules before they are imposed.
const FILTER: f64 = 0.135;
const AREA: f64 = 0.08;
const HOLE: f64 = 0.2;
/// The schedule.
const BETAS: [f64; 3] = [8.0, 16.0, 32.0];
const PER_STAGE: u64 = 70;
/// The stage with the rules: more evaluations, for islands to erode from their edges.
const RULED: u64 = 150;
/// Hammond et al.'s G = a t for the area rules.
const A_K: f64 = 1e-5;
/// Their floor: every rulebook's average transmission above 89 %.
const FLOOR: f64 = 0.89;

const RECORDED: &str = include_str!("designs/optimize_foundry_bend.hex");
const RECORDED_FILE: &str = "examples/designs/optimize_foundry_bend.hex";

fn wavelengths() -> [f64; 10] {
    std::array::from_fn(|k| 1.50 + 0.1 * k as f64 / 9.0)
}

fn grid() -> Grid {
    let n = (SPAN / H).round() as usize + 2 * PML;
    Grid {
        nx: n,
        ny: n,
        dx: H,
        dy: H,
        x0: X0 - PML as f64 * H,
        y0: Y0 - PML as f64 * H,
    }
}

fn index(origin: f64, v: f64) -> usize {
    ((v - origin) / H).round() as usize
}

/// The permittivities: the two guides, and `design` (one per region pixel) or oxide in the region.
fn cells(g: &Grid, design: Option<&[f64]>) -> Vec<c64> {
    let guide = |v: f64| (v - CENTRE).abs() < HALF;
    (0..g.nx * g.ny)
        .map(|p| {
            let (i, j) = (p % g.nx, p / g.nx);
            let (x, y) = (g.x(i), g.y(j));
            let e = if (0.0..3.0).contains(&x) && (0.0..3.0).contains(&y) {
                design.map_or(OX, |d| {
                    let (a, b) = (index(0.0, x - H / 2.0), index(0.0, y - H / 2.0));
                    d[b.min(SIDE - 1) * SIDE + a.min(SIDE - 1)]
                })
            } else if (x < 0.0 && guide(y)) || (y >= 3.0 && guide(x)) {
                SI
            } else {
                OX
            };
            c64::new(e, 0.0)
        })
        .collect()
}

/// At a wavelength: the solver at the background, the input's mode, the output's coefficients
/// on its rows and the output mode's power over the input's.
fn band(
    g: &Grid,
    background: &[c64],
    lam: Wavelength,
) -> photonoxide::Result<(Solver2d, PortMode, Vec<c64>, f64)> {
    let s = Solver2d::from_cells(*g, Polarization::Ez, lam, background, Boundaries::pml(PML))?;
    let rows = index(g.y0, CENTRE - 1.0)..index(g.y0, CENTRE + 1.0);
    let input = s
        .port_modes_within(index(g.x0, X0 + 0.5), rows, 1)?
        .remove(0);
    // the output guide turned: a straight guide whose cells along y are the bend's along x
    let aux_grid = Grid {
        nx: 2 * PML + 10,
        ny: g.nx,
        dx: H,
        dy: H,
        x0: 0.0,
        y0: g.x0,
    };
    let aux_cells: Vec<c64> = (0..aux_grid.nx * aux_grid.ny)
        .map(|p| {
            let x = aux_grid.y(p / aux_grid.nx);
            c64::new(if (x - CENTRE).abs() < HALF { SI } else { OX }, 0.0)
        })
        .collect();
    let aux = Solver2d::from_cells(
        aux_grid,
        Polarization::Ez,
        lam,
        &aux_cells,
        Boundaries::pml(PML),
    )?;
    // the turned guide's rows are the bend's columns: its window along x
    let columns = index(g.x0, CENTRE - 1.0)..index(g.x0, CENTRE + 1.0);
    let output = aux.port_modes_within(PML + 4, columns, 1)?.remove(0);
    let c = s.row_mode_functional(&output, index(g.y0, 4.0), 0, Direction::Forward)?;
    let ratio = output.power(H) / input.power(H);
    Ok((s, input, c, ratio))
}

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

/// The transmission into the output's mode at each wavelength of a binary design.
fn spectrum(design: &[bool]) -> photonoxide::Result<Vec<f64>> {
    let g = grid();
    let d: Vec<f64> = design.iter().map(|&b| if b { SI } else { OX }).collect();
    let eps = cells(&g, Some(&d));
    let background = cells(&g, None);
    let mut t = Vec::new();
    for um in wavelengths() {
        let lam = Wavelength::um(um)?;
        let (base, input, c, ratio) = band(&g, &background, lam)?;
        let s = base.reuse_cells(lam, &eps)?;
        let field = s.solve_system(&s.mode_source(&input, Direction::Forward))?;
        let (power, _) = s.functional_power_gradient(&field, &c)?;
        t.push(power * ratio);
    }
    Ok(t)
}

struct Bend {
    density: Density,
    placement: Placement,
    background: Vec<c64>,
    bands: Vec<(Wavelength, Solver2d, PortMode, Vec<c64>, f64)>,
}

impl Bend {
    fn new(beta: f64) -> photonoxide::Result<Bend> {
        let g = grid();
        let background = cells(&g, None);
        let density = Density::new([SIDE, SIDE, 1], H, [OX, SI])?
            .with_filter(FILTER, Edge::Renormalized)?
            .with_projection(Projection::new(beta, 0.5)?);
        let mut bands = Vec::new();
        for um in wavelengths() {
            let lam = Wavelength::um(um)?;
            let (s, input, c, ratio) = band(&g, &background, lam)?;
            bands.push((lam, s, input, c, ratio));
        }
        Ok(Bend {
            density,
            placement: Placement {
                origin: [index(g.x0, 0.0), index(g.y0, 0.0), 0],
                shape: [SIDE, SIDE, 1],
                grid: [g.nx, g.ny, 1],
            },
            background,
            bands,
        })
    }

    /// The ten objectives 1 − T and their gradients, rounded to 12 digits.
    fn evaluate(&self, rho: &[f64]) -> photonoxide::Result<(Vec<f64>, Vec<f64>)> {
        let mut cells = self.background.clone();
        self.placement
            .embed(&self.density.permittivity(rho)?, &mut cells)?;
        let (mut values, mut gradients) = (Vec::new(), Vec::new());
        for (lam, base, input, c, ratio) in &self.bands {
            let s = base.reuse_cells(*lam, &cells)?;
            let field = s.solve_system(&s.mode_source(input, Direction::Forward))?;
            let (t, dt) = s.functional_power_gradient(&field, c)?;
            let g = self.density.gradient(rho, &self.placement.restrict(&dt)?)?;
            values.push(round12(1.0 - t * ratio));
            gradients.extend(g.iter().map(|&v| round12(-v * ratio)));
        }
        Ok((values, gradients))
    }
}

/// More constraints beside the epigraph's: at y = (ρ, t), their values and Jacobian rows (n + 1
/// wide).
type Extra<'a> = &'a (dyn Fn(&[f64], &mut [f64], &mut [f64]) + Sync);

fn optimize() -> photonoxide::Result<Vec<bool>> {
    use genoxide::prelude::*;
    let n = SIDE * SIDE;
    let w = n + 1;
    // a grey start; the bend isn't symmetric under any mirror that maps its ports to themselves
    let mut rho = vec![0.5; n];
    let start = std::time::Instant::now();
    let form = Epigraph::new(n, 10)?;
    // MMA's iterates sit a little outside the epigraph's constraints (t just below the worst
    // objective), so genoxide's best feasible genome can stay the start: the stage keeps its own
    // best, its start included: of the designs that meet every rule the least worst objective, of
    // those that don't (when none does) the least violation; a key that sorts them so
    let rank = |worst: f64, rules: &[f64]| -> (bool, f64) {
        let violation: f64 = rules.iter().map(|v| v.max(0.0)).sum();
        if violation > 0.0 {
            (true, violation)
        } else {
            (false, worst)
        }
    };
    let run = |b: &Bend,
               rho: &[f64],
               extra: usize,
               evaluations: u64,
               more: Extra|
     -> photonoxide::Result<Vec<f64>> {
        let (values, _) = b.evaluate(rho)?;
        let mut y = rho.to_vec();
        y.push(Epigraph::start(&values));
        // the start ranks as any evaluation does: its rules too
        let mut rules = vec![0.0; extra];
        more(&y, &mut rules, &mut vec![0.0; extra * w]);
        let best = std::sync::Mutex::new((rank(Epigraph::start(&values), &rules), rho.to_vec()));
        let fitness = Constrained::differentiable(
            10 + extra,
            |x: &Reals, gradient: &mut [f64], g: &mut [f64], jacobian: &mut [f64]| {
                let Ok((values, gradients)) = b.evaluate(&x[..n]) else {
                    return f64::NAN;
                };
                if form
                    .evaluate(
                        x,
                        &values,
                        &gradients,
                        gradient,
                        &mut g[..10],
                        &mut jacobian[..10 * w],
                    )
                    .is_err()
                {
                    return f64::NAN;
                }
                more(x, &mut g[10..], &mut jacobian[10 * w..]);
                let worst = values.iter().copied().fold(f64::MIN, f64::max);
                let r = rank(worst, &g[10..]);
                let mut kept = best.lock().expect("the best");
                if r < kept.0 {
                    *kept = (r, x[..n].to_vec());
                }
                x[n]
            },
        );
        let mma = Mma::builder(Real::new(vec![0.0..=1.0; n + 1]).expect("bounds"))
            .initial_genome(Reals::from(y))
            .minimize()
            .build()
            .expect("an MMA");
        Engine::new(mma, fitness)
            .stop_when(Stop::evaluations(evaluations))
            .run()
            .expect("a run");
        Ok(best.into_inner().expect("the best").1)
    };
    let report = |b: &Bend, rho: &[f64], beta: f64| -> photonoxide::Result<()> {
        let (values, _) = b.evaluate(rho)?;
        let t: Vec<f64> = values.iter().map(|v| 1.0 - v).collect();
        println!(
            "    β {beta:>2}: worst T {:.3}, mean T {:.3} ({:.0} s)",
            t.iter().copied().fold(1.0, f64::min),
            t.iter().sum::<f64>() / t.len() as f64,
            start.elapsed().as_secs_f64()
        );
        Ok(())
    };
    for &beta in &BETAS[..2] {
        let b = Bend::new(beta)?;
        rho = run(&b, &rho, 0, PER_STAGE, &|_, _, _| {})?;
        report(&b, &rho, beta)?;
    }
    let b = Bend::new(BETAS[2])?;
    let lengths = LengthConstraints::new(&b.density, LINE, LINE)?.with_arrieta_decay();
    let eps = geometric_threshold(64.0, FILTER / 3.0)?;
    let areas = AreaConstraints::new([SIDE, SIDE], H, AREA, HOLE)?;
    let more = |x: &[f64], g: &mut [f64], jacobian: &mut [f64]| {
        let rho = &x[..n];
        let fail = |g: &mut [f64]| g.fill(f64::NAN);
        let (Ok(l), Ok(phys)) = (lengths.evaluate(rho), b.density.physical(rho)) else {
            return fail(g);
        };
        let Ok(a) = areas.evaluate(&phys) else {
            return fail(g);
        };
        for s in 0..2 {
            g[s] = round12(l.values[s] / eps - 1.0);
            for (r, d) in jacobian[s * w..s * w + n].iter_mut().zip(&l.gradients[s]) {
                *r = round12(d / eps);
            }
            // g ≤ a t as g − a t ≤ 0, both in pixels (the values are shares of the region's n
            // pixels): a few pixels' violation reads as a few, which MMA's scaling needs
            let Ok(d) = b.density.pullback(rho, &a.gradients[s]) else {
                return fail(g);
            };
            let pixels = n as f64;
            g[2 + s] = round12((a.values[s] - A_K * x[n]) * pixels);
            let row = &mut jacobian[(2 + s) * w..(3 + s) * w];
            for (r, d) in row[..n].iter_mut().zip(&d) {
                *r = round12(d * pixels);
            }
            row[n] = -A_K * pixels;
        }
    };
    rho = run(&b, &rho, 4, RULED, &more)?;
    report(&b, &rho, BETAS[2])?;
    Ok(b.density.physical(&rho)?.iter().map(|&v| v > 0.5).collect())
}

fn to_hex(d: &[bool]) -> String {
    let mut s = String::new();
    for j in 0..SIDE {
        for c in 0..SIDE.div_ceil(4) {
            let mut nibble = 0u32;
            for q in 0..4 {
                let i = 4 * c + q;
                nibble = nibble * 2 + u32::from(i < SIDE && d[j * SIDE + i]);
            }
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
                let i = 4 * c + q;
                if i < SIDE {
                    d[j * SIDE + i] = nibble & (8 >> q) != 0;
                }
            }
        }
    }
    d
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let args: Vec<String> = std::env::args().collect();
    let full = args.iter().any(|a| a == "--full");
    let write = args.iter().any(|a| a == "--write");
    let mut checks = common::Checks::default();
    let mut setup = common::Setup::new();
    setup.item(
        "domain",
        "x from −1.5 to 3.5 µm and y from −0.5 to 4.5 µm of oxide, 20 nm cells, PMLs of 20 cells; a 3 × 3 µm design region from (0, 0)",
    );
    setup.medium("silicon", &common::eps(SI));
    setup.medium("oxide", &common::eps(OX));
    setup.item(
        "guides",
        "0.5 µm wide: the input along x into the region's west side at y = 1.5 µm, the output along y out of its north side at x = 1.5 µm; E along z",
    );
    setup.item("wavelengths", "ten from 1.50 to 1.60 µm");
    setup.item(
        "rules",
        "linewidth and spacing 90 nm, minimum area 0.08 µm², minimum enclosed area 0.2 µm² (Fig. 8's rulebook)",
    );
    setup.item(
        "design",
        "150 × 150 densities, a conic filter of 135 nm, projection at η = ½, β 8 and 16 with 70 MMA evaluations each and 32 with 150, the rules from β = 32; the epigraph of the worst 1 − T",
    );
    setup.item(
        "tolerance",
        "the band-average transmission at least 0.89 (Section 6.2); the measured minimum length at least 90 nm, less Chen et al.'s 20 % for the metric; no island or hole below its minimum",
    );
    setup.print();

    let design = if full {
        println!("The optimization:");
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
    let t = spectrum(&design)?;
    let mean = t.iter().sum::<f64>() / t.len() as f64;
    let worst = t.iter().copied().fold(1.0, f64::min);
    let as_density: Vec<f64> = design.iter().map(|&b| f64::from(u8::from(b))).collect();
    let areas = AreaConstraints::new([SIDE, SIDE], H, AREA, HOLE)?.evaluate(&as_density)?;
    let length = minimum_length(
        &Pattern::new([SIDE, SIDE], design.clone())?,
        [H * 1e3, H * 1e3],
    )?;
    println!(
        "The design, binary: transmission over 1.50–1.60 µm {} (mean {mean:.3}, worst {worst:.3}); minimum length {length:.0} nm; islands and holes below their minimum: {} and {}",
        t.iter()
            .map(|v| format!("{v:.3}"))
            .collect::<Vec<_>>()
            .join(", "),
        areas.violations[0],
        areas.violations[1]
    );
    checks.at_most(
        "1 − mean transmission",
        1.0 - mean,
        ((1.0 - FLOOR) * 100.0).round() / 100.0,
    );
    // the rule is a minimum: a longer minimum length keeps it; the metric's 20 % below it
    checks.at_most(
        "minimum length's shortfall below the rule (nm)",
        (LINE * 1e3 - length).max(0.0),
        0.2 * LINE * 1e3,
    );
    checks.count("islands below 0.08 µm²", areas.violations[0], 0);
    checks.count("holes below 0.2 µm²", areas.violations[1], 0);
    Ok(checks.finish())
}
