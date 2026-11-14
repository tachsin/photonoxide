//! Device and circuit co-design (0.7, #761's I11): a ring's coupler designed in 2D FDFD inside
//! the ring's circuit, the circuit's sensitivity to the coupler's S-matrix carried to every cell
//! of the coupler by FDFD's adjoint, so the device is optimized for what the circuit does with it
//! (S. Mason et al., arXiv:2509.07233 (2025), co-optimized inverse design, the plan's target).
//!
//! **The chip** is an all-pass ring. Its coupler is a 2D FDFD problem (E along z, 20 nm cells):
//! two 0.3 µm guides of 2.85 (a 220 nm silicon slab's effective index) in 1.444, 1.5 µm apart at
//! the ports and brought to 0.3 µm apart over 1 µm by cosine S-bends 3 µm long; the gap there is
//! a design region of five bands along the guides, each band's cells at one density ρ between
//! the cladding's and the core's permittivity. The ring's loop is a loss of round-trip amplitude
//! a = 0.95 and a phase shifter.
//!
//! **The closed form** (A. Yariv, Electron. Lett. 36, 321 (2000)): with t and t′ the bus's and
//! the ring's through amplitudes and κ, κ′ the crossings, the through port carries
//! |(t − (tt′ − κκ′) a e^{iφ})/(1 − t′ a e^{iφ})|², which vanishes only at critical coupling,
//! |t| = a |tt′ − κκ′| (|t| = a for a lossless coupler), on resonance. L-BFGS-B moves the bands'
//! densities to minimize the through power, the loop's phase put on resonance for each design
//! (a golden-section search on the circuit alone; by the envelope theorem the gradient is the one
//! at that phase). The densities' gradient is one circuit adjoint (its sensitivity to the
//! coupler's S, `Circuit::instance_sensitivity`) and four FDFD adjoints
//! (`Solver2d::s_matrix_gradient`).
//!
//! Checked: the composed gradient against fourth-order differences of the whole chain at three
//! cells and the phase, within ten times what the differences resolve; the through power at the
//! optimum below 1e-6. The closed form's |t| and a|tt′ − κκ′| are printed beside, with the
//! coupler's reflections, which the 2 × 2 form leaves out and the circuit keeps.

mod common;

use std::f64::consts::PI;
use std::process::ExitCode;
use std::sync::Arc;

use genoxide::prelude::*;
use photonoxide::Complex64 as c64;
use photonoxide::circuit::components::PhaseShifter;
use photonoxide::circuit::{Circuit, Component, Fixed, Netlist, SMatrix, objective};
use photonoxide::fdfd::{Boundaries, Grid, Polarization, Port, Side, Solver2d};
use photonoxide::portable;
use photonoxide::units::Wavelength;

const OK: &str = "a valid example";
/// The loop's round-trip amplitude.
const LOOP: f64 = 0.95;
/// The cell, µm.
const H: f64 = 0.02;

fn lambda() -> Wavelength {
    Wavelength::um(1.55).expect(OK)
}

/// The coupler: its grid, cells, design region, ports (lb, lt, rt, rb) and solver.
struct Coupler {
    base: Vec<c64>,
    region: Vec<usize>,
    ports: Vec<Port>,
    contrast: f64,
    solver: Solver2d,
    /// Each region cell's pixel, 4 × 4 cells each.
    pixel_of: Vec<usize>,
    pixels: usize,
}

impl Coupler {
    fn new() -> photonoxide::Result<Coupler> {
        let pml = 15;
        let (length, half) = (9.0, 1.6);
        let grid = Grid {
            nx: (length / H).round() as usize + 2 * pml,
            ny: (2.0 * half / H).round() as usize + 2 * pml,
            dx: H,
            dy: H,
            x0: -(pml as f64) * H,
            y0: -half - pml as f64 * H,
        };
        let (core, clad) = (2.85f64 * 2.85, 1.444f64 * 1.444);
        let mut base = vec![c64::new(clad, 0.0); grid.nx * grid.ny];
        let mut region = Vec::new();
        let mut at = Vec::new();
        for j in 0..grid.ny {
            for i in 0..grid.nx {
                let (x, y) = (grid.x(i), grid.y(j));
                let k = j * grid.nx + i;
                let centre = guide_centre(x);
                if (y.abs() - centre).abs() < 0.15 {
                    base[k] = c64::new(core, 0.0);
                } else if y.abs() < centre - 0.15 && (4.0..5.0).contains(&x) {
                    region.push(k);
                    at.push((i, j));
                }
            }
        }
        let (i0, _) = (
            at.iter().map(|a| a.0).min().unwrap_or(0),
            at.iter().map(|a| a.1).min().unwrap_or(0),
        );
        let pixel_of: Vec<usize> = at.iter().map(|&(i, _)| (i - i0) / 10).collect();
        let pixels = pixel_of.iter().max().map_or(0, |m| m + 1);
        let solver = Solver2d::from_cells(
            grid,
            Polarization::Ez,
            lambda(),
            &base,
            Boundaries::pml(pml),
        )?;
        let column = |x: f64| ((x - grid.x0) / H) as usize;
        let mid = grid.ny / 2;
        let mut ports = Vec::new();
        for (x, top, side) in [
            (0.4, false, Side::Left),
            (0.4, true, Side::Left),
            (length - 0.4, true, Side::Right),
            (length - 0.4, false, Side::Right),
        ] {
            let rows = if top { mid..grid.ny - pml } else { pml..mid };
            let mode = solver.port_modes_within(column(x), rows, 1)?.remove(0);
            ports.push(Port { mode, side });
        }
        Ok(Coupler {
            base,
            region,
            ports,
            contrast: core - clad,
            solver,
            pixel_of,
            pixels,
        })
    }

    /// The solver at densities `rho`.
    fn at(&self, rho: &[f64]) -> photonoxide::Result<Solver2d> {
        let mut eps = self.base.clone();
        for (&k, &r) in self.region.iter().zip(rho) {
            eps[k] += r * self.contrast;
        }
        self.solver.reuse_cells(lambda(), &eps)
    }
}

/// The ring around a coupler of S-matrix `s`: the circuit, its in and out ports and the
/// phase's slot.
fn ring(s: &[Vec<c64>]) -> photonoxide::Result<(Circuit, usize, usize, usize)> {
    let c = SMatrix::from_fn(4, |q, p| s[q][p]);
    let loss = SMatrix::from_fn(2, |q, p| {
        if q == p {
            c64::new(0.0, 0.0)
        } else {
            c64::new(LOOP, 0.0)
        }
    });
    let mut net = Netlist::new();
    net.add(
        "c",
        Arc::new(Fixed::new("fdfd coupler", &["lb", "lt", "rt", "rb"], c)?),
    )?;
    net.add("loss", Arc::new(Fixed::new("loss", &["o1", "o2"], loss)?))?;
    net.add("phi", Arc::new(PhaseShifter::new()))?;
    net.connect("c.rt", "loss.o1")?;
    net.connect("loss.o2", "phi.o1")?;
    net.connect("phi.o2", "c.lt")?;
    net.expose("in", "c.lb")?;
    net.expose("out", "c.rb")?;
    let circuit = net.compile()?;
    let port = |name: &str| {
        circuit
            .ports()
            .iter()
            .position(|p| p.name == name)
            .expect("a port")
    };
    let slot = circuit.parameter("phi.phase").expect("the phase");
    let (i, o) = (port("in"), port("out"));
    Ok((circuit, i, o, slot))
}

/// The through power at densities `rho` and loop phase `phase`, its gradient (the densities',
/// then the phase's) and the coupler's S-matrix.
fn evaluate(
    c: &Coupler,
    rho: &[f64],
    phase: f64,
) -> photonoxide::Result<(f64, Vec<f64>, Vec<Vec<c64>>)> {
    let solver = c.at(rho)?;
    let s = solver.s_matrix(&c.ports)?;
    let (circuit, i, o, slot) = ring(&s)?;
    let mut values = circuit.values().to_vec();
    values[slot] = phase;
    let (f, h) =
        circuit.instance_sensitivity(&[lambda()], &values, "c", |m| objective::power(m, o, i))?;
    let h: Vec<Vec<c64>> = (0..4)
        .map(|q| (0..4).map(|p| h[0][(q, p)]).collect())
        .collect();
    let (_, by_cell) = solver.s_matrix_gradient(&c.ports, &h)?;
    let mut gradient: Vec<f64> = c.region.iter().map(|&k| by_cell[k] * c.contrast).collect();
    let (_, by_parameter) =
        circuit.gradient(&[lambda()], &values, |m| objective::power(m, o, i))?;
    gradient.push(by_parameter[slot]);
    Ok((f, gradient, s))
}
pub fn main() -> photonoxide::Result<ExitCode> {
    let mut setup = common::Setup::new();
    setup.item(
        "device",
        "a ring's coupler: two 0.3 µm guides (2.85 in 1.444) brought from 1.5 to 0.3 µm apart over 1 µm by 3 µm S-bends, the gap there a design region of five bands; the loop a loss of amplitude 0.95 and a phase shifter",
    );
    setup.item(
        "grid",
        "2D FDFD, E along z, 20 nm cells, 15 cells of PML, at 1.55 µm",
    );
    setup.item(
        "solver",
        "genoxide's L-BFGS-B on the five bands' densities (in [0, 1]) from 0, the loop on resonance for each (golden-section search), the gradient by the circuit adjoint and FDFD's adjoint",
    );
    setup.item(
        "tolerance",
        "the composed gradient within what fourth-order differences resolve; the through power below 1e-6 at the optimum",
    );
    setup.print();
    let mut checks = common::Checks::default();
    let c = Coupler::new()?;
    let n = c.region.len();
    // a weak, asymmetric start
    let start: Vec<f64> = (0..n)
        .map(|k| 0.05 + 0.05 * ((0.6180339887 * (k + 1) as f64) % 1.0))
        .chain([0.0])
        .collect();
    let (f0, g0, s0) = evaluate(&c, &start[..n], start[n])?;

    // the composed gradient against differences of the whole chain
    let probe = |k: usize, d: f64| -> f64 {
        let mut x = start.clone();
        x[k] += d;
        evaluate(&c, &x[..n], x[n]).expect(OK).0
    };
    let diff = |k: usize, h: f64| {
        (8.0 * (probe(k, h) - probe(k, -h)) - (probe(k, 2.0 * h) - probe(k, -2.0 * h))) / (12.0 * h)
    };
    // a probe agrees when it is within ten times what the differences resolve (the gap between
    // steps h and 2h), as the cases hold it; the count, not the digits, is what is printed, since
    // the digits are round-off's and differ between systems
    let top = g0.iter().fold(0.0, |m: f64, g| m.max(g.abs()));
    let probes = [0, n / 2 + 7, n - 1, n];
    let agreeing = probes
        .iter()
        .filter(|&&k| {
            let (one, two) = (diff(k, 1e-3), diff(k, 2e-3));
            (one - g0[k]).abs() / top <= 10.0 * (one - two).abs() / top
        })
        .count();
    println!(
        "\nThe start: through power {f0:.4}, the coupler's |t| = {:.4}; the composed gradient within ten times what fourth-order differences resolve at {agreeing} of {} probes (three cells and the loop's phase)",
        portable_abs(s0[3][0]),
        probes.len()
    );
    checks.count(
        "probes where the gradient meets the differences",
        agreeing,
        probes.len(),
    );

    // the design: pixels of 4 × 4 cells (80 nm), the loop's phase put on resonance for each
    // design (a golden-section search on the circuit alone), so that F(ρ) = min over φ and, by
    // the envelope theorem, its gradient is the one at that phase
    let pixels = c.pixels;
    let at_resonance = |rho: &[f64]| -> (f64, Vec<f64>, Vec<Vec<c64>>, f64) {
        let cells: Vec<f64> = c.pixel_of.iter().map(|&p| rho[p]).collect();
        let s = c.at(&cells).expect(OK).s_matrix(&c.ports).expect(OK);
        let phase = resonance(&s);
        let (f, g, s) = evaluate(&c, &cells, phase).expect(OK);
        let mut by_pixel = vec![0.0; pixels];
        for (r, &p) in c.pixel_of.iter().enumerate() {
            by_pixel[p] += g[r];
        }
        (f, by_pixel, s, phase)
    };
    let fitness = |x: &Reals, gradient: &mut [f64]| -> f64 {
        let (f, g, _, _) = at_resonance(x);
        gradient.copy_from_slice(&g);
        f
    };
    let run = Lbfgsb::builder(Real::new(vec![0.0..=1.0; pixels]).expect("bounds"))
        .initial_genome(Reals::from(vec![0.0; pixels]))
        .gradient_tolerance(0.0)
        .function_tolerance(0.0)
        .minimize()
        .build()
        .expect("a valid L-BFGS-B");
    let outcome = Engine::new(run, Differentiable(&fitness))
        .stop_when(Stop::target(1e-9).or(Stop::evaluations(200)))
        .run()
        .expect("a run");
    let best = outcome.best_genome().to_vec();
    let (_, _, _, phase) = at_resonance(&best);
    let cells: Vec<f64> = c.pixel_of.iter().map(|&p| best[p]).collect();
    let (f, _, s) = evaluate(&c, &cells, phase)?;
    let t = portable_abs(s[3][0]);
    let kept: f64 = (0..4).map(|q| s[q][0].norm_sqr()).sum();
    println!(
        "Optimized in {} evaluations: through power {}; the coupler's |t| = {t:.4} against the loop's a = {LOOP}, the coupler keeping {kept:.4} of the power (its radiation and reflection the rest)",
        outcome.evaluations(),
        if f < 1e-12 {
            "below 1e-12".to_string()
        } else {
            format!("{f:.2e}")
        }
    );
    // the closed form for a coupler with loss: with t and t' the bus's and the ring's through
    // amplitudes and κ, κ' the crossings, the through port vanishes on resonance only where
    // |t| = a |t t' − κ κ'|; the coupler's reflections, which the 2 × 2 form leaves out, set how
    // closely
    let det = s[3][0] * s[2][1] - s[2][0] * s[3][1];
    let reflection = (0..4)
        .map(|q| portable_abs(s[q][0]).max(portable_abs(s[q][1])))
        .take(2)
        .fold(0.0, f64::max);
    println!(
        "  critical coupling: |t| = {t:.4} and a |t t' − κκ'| = {:.4}, apart by {:.1e}; the coupler's largest reflection amplitude {reflection:.3}",
        LOOP * portable_abs(det),
        (t - LOOP * portable_abs(det)).abs()
    );
    checks.at_most("the through power at the optimum", f, 1e-6);
    Ok(checks.finish())
}

/// The loop's phase in [−π, π) that minimizes the through power around a coupler of S-matrix
/// `s`: golden-section search on the circuit's solve (a ring's through power has one minimum a
/// period, at its resonance), from the coarse scan's best of 64 phases.
fn resonance(s: &[Vec<c64>]) -> f64 {
    let (circuit, i, o, slot) = ring(s).expect(OK);
    let base = circuit.values().to_vec();
    let power = |phase: f64| {
        let mut values = base.clone();
        values[slot] = phase;
        circuit.s_matrix_with(lambda(), &values).expect(OK)[(o, i)].norm_sqr()
    };
    let step = 2.0 * PI / 64.0;
    let coarse = (0..64)
        .map(|k| -PI + step * k as f64)
        .min_by(|a, b| power(*a).total_cmp(&power(*b)))
        .expect("a phase");
    let golden = 0.5 * (5f64.sqrt() - 1.0);
    let (mut lo, mut hi) = (coarse - step, coarse + step);
    for _ in 0..80 {
        let (x1, x2) = (hi - golden * (hi - lo), lo + golden * (hi - lo));
        if power(x1) < power(x2) {
            hi = x2;
        } else {
            lo = x1;
        }
    }
    0.5 * (lo + hi)
}

/// |z| by photonoxide's portable maths.
fn portable_abs(z: c64) -> f64 {
    photonoxide::portable::complex::abs(z)
}

/// The guides' centres' distance from the axis at x (µm): 0.75 at the ports, 0.3 along the
/// coupling length from 4 to 5 µm, and cosine S-bends 3 µm long between.
fn guide_centre(x: f64) -> f64 {
    let bend = |t: f64| 0.5 * (1.0 + portable::cos(std::f64::consts::PI * t.clamp(0.0, 1.0)));
    let away = if x < 4.0 {
        bend((x - 1.0) / 3.0)
    } else if x > 5.0 {
        1.0 - bend((x - 5.0) / 3.0)
    } else {
        0.0
    };
    0.3 + 0.45 * away
}
