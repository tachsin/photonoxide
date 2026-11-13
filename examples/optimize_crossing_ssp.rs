//! Hammond et al.'s waveguide crossing by the subpixel-smoothed projection: topology optimization
//! to β = ∞, shape optimization at β = ∞ from a plain cross, and the standard projection's
//! vanishing gradient against the smoothed one's as β grows.
//!
//! A. M. Hammond, A. Oskooi, I. M. Hammond, M. Chen, S. E. Ralph, S. G. Johnson, "Unifying and
//! accelerating level-set and density-based topology optimization by subpixel-smoothed
//! projection", Opt. Express 33, 33620 (2025),
//! [doi:10.1364/OE.563512](https://doi.org/10.1364/OE.563512), Section 4.1, Figs. 2, 5 and 6.
//!
//! **The problem** (Section 4.1). A 3 × 3 µm design region where two silicon waveguides (ε = 12)
//! cross, E along z (2D), at 1.55 µm: the power into the east guide's fundamental mode from the
//! west guide's, maximized, the design held to the square's four mirror and rotation symmetries.
//! The paper doesn't state the guides' width, their cladding or the filter: here 0.5 µm guides in
//! oxide (n = 1.44), a conic filter of 100 nm, 2D FDFD on 20 nm cells (theirs: Meep at 33 nm, the
//! design at 17 nm), genoxide's MMA (theirs: NLopt's CCSA).
//!
//! **What is compared.** (i) Their topology optimization, β = 16, 32 and ∞ for 30 iterations
//! each from ρ = ½ (Fig. 6a), and (ii) their shape optimization at β = ∞ from a plain cross for 60
//! (Fig. 6b), each reaching their 99 % (Fig. 2's target): the transmission at β = ∞ at least
//! 0.99. (iii) Fig. 5(b): the gradient's norm at the plain cross against β, the standard
//! projection's falling towards 0 while the smoothed one's settles at its β = ∞ value: here the
//! standard one's at β = 1024 below a hundredth of the smoothed one's, and the smoothed one's at
//! β = 1024 within 5 % of its β = ∞ value. (iv) Fig. 2: from the plain cross, the evaluations to
//! reach 99 % at each β, the standard projection's growing with β (and never at β = ∞, where its
//! gradient is zero) while the smoothed one's stays bounded (`--full`): here the smoothed one
//! reaching it at every β and the standard one not within 60 at β = 1024.
//!
//! **What it found** (`--full`, 2026-11-12, 90 s on 12 threads here). The gradient's norm at the
//! plain cross: the smoothed projection's 1.34e-2, 8.13e-3, 8.10e-3 and 8.10e-3 at β = 16, 64,
//! 256 and 1024, its β = ∞ value 8.10e-3; the standard one's 1.30e-2, 1.29e-3, 2.3e-8 and
//! 8.6e-29. Topology optimization reaches T = 0.9980 at β = 16, 0.9993 at 32 and 0.9993 at ∞ (the
//! design binary, its one-pixel boundary rounded: 0.9988); shape optimization at β = ∞ from the
//! plain cross 0.9952, 99 % at evaluation 28. From the plain cross, the evaluations to 99 % at
//! β = 16, 64, 256, 1024 and ∞: smoothed 28, 25, 28, 23 and 28; standard 25, then none within
//! 60 (Fig. 2: theirs diverging with β). The design is recorded in
//! `examples/designs/optimize_crossing_ssp.hex`, binary, which the plain run solves; `--full`
//! optimizes again and checks it repeats it pixel for pixel, values and gradients rounded to 12
//! digits.
//!
//! ```sh
//! cargo run --release --example optimize_crossing_ssp                # the gradients, and the recorded design solved
//! cargo run --release --example optimize_crossing_ssp -- --full      # the optimizations
//! cargo run --release --example optimize_crossing_ssp -- --full --write  # and record the design
//! ```

mod common;

use photonoxide::portable;
use std::process::ExitCode;
use std::sync::Mutex;

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{Boundaries, Direction, Grid, Polarization, PortMode, Solver2d};
use photonoxide::topology::{Density, Edge, Placement, Projection, SmoothedProjection};
use photonoxide::units::Wavelength;

/// The cell, µm, the PML in cells, and the design region's pixels a side.
const H: f64 = 0.02;
const PML: usize = 20;
const SIDE: usize = 150;
/// The plane between the PMLs: from −2.5 to 2.5 µm both ways; the region from −1.5 to 1.5.
const HALF: f64 = 2.5;
const REGION: f64 = 1.5;
/// Silicon's (their ε = 12) and oxide's permittivities, the guide's half-width, the filter.
const SI: f64 = 12.0;
const OX: f64 = 1.44 * 1.44;
const GUIDE: f64 = 0.25;
const FILTER: f64 = 0.1;
/// Their target, and the schedule.
const TARGET: f64 = 0.99;
/// 1 − TARGET, as the checks print it.
const MISS: f64 = 0.01;
const TOPOLOGY: [f64; 3] = [16.0, 32.0, f64::INFINITY];
const PER_STAGE: u64 = 30;
const SHAPE: u64 = 60;

const RECORDED: &str = include_str!("designs/optimize_crossing_ssp.hex");
const RECORDED_FILE: &str = "examples/designs/optimize_crossing_ssp.hex";

fn grid() -> Grid {
    let n = (2.0 * HALF / H).round() as usize + 2 * PML;
    Grid {
        nx: n,
        ny: n,
        dx: H,
        dy: H,
        x0: -HALF - PML as f64 * H,
        y0: -HALF - PML as f64 * H,
    }
}

fn index(origin: f64, v: f64) -> usize {
    ((v - origin) / H).round() as usize
}

/// The plane without the design: the four arms, the region the cladding's.
fn background(g: &Grid) -> Vec<c64> {
    (0..g.nx * g.ny)
        .map(|p| {
            let (x, y) = (g.x(p % g.nx), g.y(p / g.nx));
            let inside = x.abs() < REGION && y.abs() < REGION;
            let arm = x.abs() < GUIDE || y.abs() < GUIDE;
            c64::new(if arm && !inside { SI } else { OX }, 0.0)
        })
        .collect()
}

/// The densities' average over the square's eight symmetries: a symmetric projector, its own
/// transpose.
fn symmetric(rho: &[f64]) -> Vec<f64> {
    let n = SIDE;
    let m = n - 1;
    (0..n * n)
        .map(|p| {
            let (i, j) = (p % n, p / n);
            [
                (i, j),
                (m - i, j),
                (i, m - j),
                (m - i, m - j),
                (j, i),
                (m - j, i),
                (j, m - i),
                (m - j, m - i),
            ]
            .iter()
            .map(|&(a, b)| rho[b * n + a])
            .sum::<f64>()
                / 8.0
        })
        .collect()
}

/// A plain cross of the guides' width through the region.
fn cross() -> Vec<f64> {
    (0..SIDE * SIDE)
        .map(|p| {
            let x = -REGION + H * ((p % SIDE) as f64 + 0.5);
            let y = -REGION + H * ((p / SIDE) as f64 + 0.5);
            f64::from(u8::from(x.abs() < GUIDE || y.abs() < GUIDE))
        })
        .collect()
}

/// The projection at β: smoothed (theirs) or the standard threshold projection.
#[derive(Clone, Copy)]
enum Kind {
    Smoothed,
    Standard,
}

struct Crossing {
    background: Vec<c64>,
    placement: Placement,
    solver: Solver2d,
    input: PortMode,
    output: PortMode,
    ratio: f64,
}

impl Crossing {
    fn new() -> photonoxide::Result<Crossing> {
        let g = grid();
        let lam = Wavelength::um(1.55)?;
        let background = background(&g);
        let solver =
            Solver2d::from_cells(g, Polarization::Ez, lam, &background, Boundaries::pml(PML))?;
        let rows = index(g.y0, -1.0)..index(g.y0, 1.0);
        let input = solver
            .port_modes_within(index(g.x0, -2.0), rows.clone(), 1)?
            .remove(0);
        let output = solver
            .port_modes_within(index(g.x0, 2.0), rows, 1)?
            .remove(0);
        let ratio = output.power(H) / input.power(H);
        Ok(Crossing {
            background,
            placement: Placement {
                origin: [index(g.x0, -REGION), index(g.y0, -REGION), 0],
                shape: [SIDE, SIDE, 1],
                grid: [g.nx, g.ny, 1],
            },
            solver,
            input,
            output,
            ratio,
        })
    }

    fn density(kind: Kind, beta: f64) -> photonoxide::Result<Density> {
        let d =
            Density::new([SIDE, SIDE, 1], H, [OX, SI])?.with_filter(FILTER, Edge::Renormalized)?;
        Ok(match kind {
            Kind::Smoothed => d.with_smoothed_projection(SmoothedProjection::new(beta, 0.5)?),
            Kind::Standard => d.with_projection(Projection::new(beta, 0.5)?),
        })
    }

    /// T into the east guide's mode, and its gradient in the free densities.
    fn evaluate(&self, density: &Density, rho: &[f64]) -> photonoxide::Result<(f64, Vec<f64>)> {
        let sym = symmetric(rho);
        let mut cells = self.background.clone();
        self.placement
            .embed(&density.permittivity(&sym)?, &mut cells)?;
        let s = self.solver.reuse_cells(Wavelength::um(1.55)?, &cells)?;
        let field = s.solve_system(&s.mode_source(&self.input, Direction::Forward))?;
        let (p, dp) = s.mode_power_gradient(&field, &self.output, Direction::Forward)?;
        let g = density.gradient(&sym, &self.placement.restrict(&dp)?)?;
        let g: Vec<f64> = symmetric(&g)
            .iter()
            .map(|v| round12(v * self.ratio))
            .collect();
        Ok((round12(p * self.ratio), g))
    }

    /// T of a binary design.
    fn binary(&self, design: &[bool]) -> photonoxide::Result<f64> {
        let eps: Vec<f64> = design.iter().map(|&s| if s { SI } else { OX }).collect();
        let mut cells = self.background.clone();
        self.placement.embed(&eps, &mut cells)?;
        let s = self.solver.reuse_cells(Wavelength::um(1.55)?, &cells)?;
        let field = s.solve_system(&s.mode_source(&self.input, Direction::Forward))?;
        Ok(
            s.mode_power_gradient(&field, &self.output, Direction::Forward)?
                .0
                * self.ratio,
        )
    }

    /// MMA on 1 − T from `rho` for `evaluations`: the best densities and each evaluation's T.
    fn optimize(
        &self,
        density: &Density,
        rho: &[f64],
        evaluations: u64,
    ) -> photonoxide::Result<(Vec<f64>, Vec<f64>)> {
        use genoxide::prelude::*;
        let seen = Mutex::new((Vec::new(), (f64::INFINITY, rho.to_vec())));
        let fitness = |x: &Reals, gradient: &mut [f64]| -> f64 {
            let Ok((t, g)) = self.evaluate(density, x) else {
                return f64::NAN;
            };
            for (a, b) in gradient.iter_mut().zip(&g) {
                *a = -b;
            }
            let mut s = seen.lock().expect("the record");
            s.0.push(t);
            if 1.0 - t < s.1.0 {
                s.1 = (1.0 - t, x.to_vec());
            }
            1.0 - t
        };
        let mma = Mma::builder(Real::new(vec![0.0..=1.0; rho.len()]).expect("bounds"))
            .initial_genome(Reals::from(rho.to_vec()))
            .minimize()
            .build()
            .expect("an MMA");
        Engine::new(mma, Differentiable(&fitness))
            .stop_when(Stop::evaluations(evaluations))
            .run()
            .expect("a run");
        let (record, (_, best)) = seen.into_inner().expect("the record");
        Ok((best, record))
    }
}

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

fn norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
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

/// The first evaluation (from 1) at or above the target, if any.
fn reached(record: &[f64]) -> Option<usize> {
    record.iter().position(|&t| t >= TARGET).map(|k| k + 1)
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let args: Vec<String> = std::env::args().collect();
    let full = args.iter().any(|a| a == "--full");
    let write = args.iter().any(|a| a == "--write");
    let mut checks = common::Checks::default();
    let mut setup = common::Setup::new();
    setup.item(
        "domain",
        "5 × 5 µm of oxide between PMLs of 20 cells, 20 nm cells; a 3 × 3 µm design region at the centre",
    );
    setup.medium("silicon", &common::eps(SI));
    setup.medium("oxide", &common::eps(OX));
    setup.item(
        "guides",
        "0.5 µm wide, crossing at the centre; E along z; 1.55 µm; the west guide's fundamental mode in, the east guide's out",
    );
    setup.item(
        "design",
        "150 × 150 densities averaged over the square's eight symmetries, a conic filter of 100 nm, the subpixel-smoothed projection at η = ½ (R̂ = 0.55 cells); genoxide's MMA on 1 − T",
    );
    setup.item(
        "tolerance",
        "their 99 % at β = ∞ (Figs. 2 and 6); the standard projection's gradient at β = 1024 below 1 % of the smoothed one's, and the smoothed one's within 5 % of its β = ∞ value (Fig. 5b)",
    );
    setup.print();
    let c = Crossing::new()?;
    let start = std::time::Instant::now();

    // Fig. 5(b): the gradient's norm at the plain cross against β
    let cross = cross();
    println!("The gradient's norm at the plain cross (Fig. 5b):");
    let at = |kind: Kind, beta: f64| -> photonoxide::Result<f64> {
        Ok(norm(
            &c.evaluate(&Crossing::density(kind, beta)?, &cross)?.1,
        ))
    };
    let infinite = at(Kind::Smoothed, f64::INFINITY)?;
    let mut last = (0.0, 0.0);
    for beta in [16.0, 64.0, 256.0, 1024.0] {
        let (smoothed, standard) = (at(Kind::Smoothed, beta)?, at(Kind::Standard, beta)?);
        println!("    β {beta:>4}: smoothed {smoothed:.4e}, standard {standard:.4e}");
        last = (smoothed, standard);
    }
    println!("    β    ∞: smoothed {infinite:.4e}, standard 0 (a step)");
    checks.at_most(
        "standard over smoothed gradient at β = 1024",
        last.1 / last.0,
        0.01,
    );
    checks.at_most(
        "smoothed gradient at β = 1024 against β = ∞",
        (last.0 / infinite - 1.0).abs(),
        0.05,
    );

    let design = if full {
        // (i) topology optimization: β 16, 32, ∞ from ρ = ½
        println!("Topology optimization (Fig. 6a):");
        let mut rho = vec![0.5; SIDE * SIDE];
        let mut final_density = None;
        for beta in TOPOLOGY {
            let d = Crossing::density(Kind::Smoothed, beta)?;
            let (best, record) = c.optimize(&d, &rho, PER_STAGE)?;
            rho = best;
            println!(
                "    β {beta:>3}: T {:.4} ({:.0} s)",
                record.iter().copied().fold(0.0, f64::max),
                start.elapsed().as_secs_f64()
            );
            final_density = Some(d);
        }
        let d = final_density.expect("a stage");
        let (t, _) = c.evaluate(&d, &rho)?;
        checks.at_most("1 − T, topology optimization at β = ∞", 1.0 - t, MISS);
        let physical = d.physical(&symmetric(&rho))?;
        let binary: Vec<bool> = physical.iter().map(|&v| v > 0.5).collect();

        // (ii) shape optimization at β = ∞ from the plain cross
        let d = Crossing::density(Kind::Smoothed, f64::INFINITY)?;
        let (best, record) = c.optimize(&d, &cross, SHAPE)?;
        let (t, _) = c.evaluate(&d, &best)?;
        println!(
            "Shape optimization at β = ∞ from the plain cross (Fig. 6b): T {:.4} after {} evaluations, {} reached at evaluation {} ({:.0} s)",
            t,
            record.len(),
            TARGET,
            reached(&record).map_or("none".into(), |k| k.to_string()),
            start.elapsed().as_secs_f64()
        );
        checks.at_most("1 − T, shape optimization at β = ∞", 1.0 - t, MISS);

        // (iv) Fig. 2: evaluations to reach 99 % from the plain cross against β
        println!("Evaluations to reach {TARGET} from the plain cross (Fig. 2), at most {SHAPE}:");
        let mut standard_last = None;
        let mut misses = usize::from(reached(&record).is_none());
        for beta in [16.0, 64.0, 256.0, 1024.0] {
            let mut counts = Vec::new();
            for kind in [Kind::Smoothed, Kind::Standard] {
                let (_, record) = c.optimize(&Crossing::density(kind, beta)?, &cross, SHAPE)?;
                counts.push(reached(&record));
            }
            println!(
                "    β {beta:>4}: smoothed {}, standard {}",
                counts[0].map_or("none".into(), |k| k.to_string()),
                counts[1].map_or("none".into(), |k| k.to_string())
            );
            misses += usize::from(counts[0].is_none());
            standard_last = Some(counts);
        }
        println!(
            "    β    ∞: smoothed {}, standard none (its gradient is zero) ({:.0} s)",
            reached(&record).map_or("none".into(), |k| k.to_string()),
            start.elapsed().as_secs_f64()
        );
        checks.count(
            "β where the smoothed projection misses 99 % in 60",
            misses,
            0,
        );
        if let Some(counts) = standard_last {
            let smoothed = counts[0].unwrap_or(usize::MAX);
            let standard = counts[1].unwrap_or(usize::MAX);
            checks.count(
                "β = 1024: the standard projection needs more than the smoothed",
                usize::from(standard <= smoothed),
                0,
            );
        }
        if write {
            std::fs::write(RECORDED_FILE, to_hex(&binary)).expect("the design written");
            println!("  written to {RECORDED_FILE}");
        } else {
            let differing = binary
                .iter()
                .zip(from_hex(RECORDED))
                .filter(|(a, b)| **a != *b)
                .count();
            checks.count("pixels unlike the recorded design", differing, 0);
        }
        binary
    } else {
        from_hex(RECORDED)
    };
    let t = c.binary(&design)?;
    println!(
        "The topology-optimized design, binary (its smoothed boundary rounded to the pixels): T {t:.4}"
    );
    checks.at_most("1 − T, the recorded design binary", 1.0 - t, MISS);
    Ok(checks.finish())
}
