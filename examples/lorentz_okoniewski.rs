//! A plane wave's reflection from a two-term Lorentz medium, by FDTD with auxiliary
//! differential equations.
//!
//! M. Okoniewski, M. Mrozowski, M. A. Stuchly, "Simple treatment of multi-term dispersion in
//! FDTD", IEEE Microw. Guided Wave Lett. 7, 121 (1997),
//! [doi:10.1109/75.569723](https://doi.org/10.1109/75.569723), Section III and Fig. 1:
//!
//! - **The problem:** a TEM wave from vacuum onto a half-space of a Lorentz medium, P = 2,
//!   ε_s = 3, ε∞ = 1.5, ω₁ = 40π Grad/s and ω₂ = 100π Grad/s (20 and 50 GHz), δ₁ = 0.1ω₁ and
//!   δ₂ = 0.1ω₂, A₁ = 0.4 and A₂ = 0.6, on cells of Δz = 37.5 µm: ε(ω) = ε∞ +
//!   Σ (ε_s − ε∞)A_p ω_p² / (ω_p² + 2jωδ_p − ω²) (their Eq. 1, e^(+jωt)).
//! - **The measurement:** the absolute errors of the reflection coefficient's magnitude and
//!   phase against the analytic one, 0 to 60 GHz, for their three schemes; this is their
//!   synchronized Lorentz scheme (LADES), the dashed curves. Their time step and the length of
//!   their run aren't given.
//!
//! Read off the figure (at 600 dpi: 98.4 pixels to 10⁻³ in magnitude and 45.9 to 10⁻³ rad in
//! phase, the frames' corners and ticks consistent to a pixel), LADES's magnitude error is
//! about +0.5 × 10⁻³ with ripples of ±0.3 × 10⁻³ up to 45 GHz, then rises: 0.70 × 10⁻³ at
//! 45 GHz, 1.15 × 10⁻³ at 52, 1.68 × 10⁻³ at 58 and 1.84 × 10⁻³ at 59.5. Its phase error is
//! within ±1 × 10⁻³ rad (its ripples) up to 50 GHz, then rises: 0.77 × 10⁻³ at 53 GHz,
//! 1.6 × 10⁻³ at 56 and 2.1 × 10⁻³ at 57. The paper doesn't explain the ripples (its Debye
//! curve has none); with its time step and run unknown, the curves are read as bounds.
//!
//! Here, with the paper's grid and medium: a line of Δz = 37.5 µm cells (one cell across and
//! periodic across, a CPML of 40 cells at each end), the medium's face halfway between two
//! values of E, a differentiated Gaussian from a current sheet 100 cells before it, and the
//! reflection from the run with the medium and the run without, its phase referred to the face
//! with the grid's own wavenumber. At a Courant number of 1 and of 0.5, and at Δz/2 to see the
//! grid's error.
//! Every |Δr| and phase error must be within the paper's curve (its smooth part plus its
//! ripple, 0.8 × 10⁻³, below 45 GHz; the values read above beyond).
//!
//! ```sh
//! cargo run --release --example lorentz_okoniewski
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::Complex64 as c64;
use photonoxide::fdfd::{Axis, Edges, Grid3d};
use photonoxide::fdtd::{Boundaries, Cpml, Dispersive, Field, Pole, Simulation, Source, Waveform};
use photonoxide::units::Frequency;

/// The paper's cell, µm.
const DZ: f64 = 37.5;

/// A frequency in GHz.
fn ghz(f: f64) -> Frequency {
    Frequency::thz(f / 1000.0).unwrap()
}

/// The paper's Lorentz medium, in photonoxide's e^(−iωt): Δε_p = (ε_s − ε∞)A_p, damping
/// γ_p = 2δ_p.
fn medium() -> Dispersive {
    let term = |a: f64, f0: f64| Pole::Lorentz {
        strength: (3.0 - 1.5) * a,
        resonance: ghz(f0),
        damping: 2.0 * 0.1 * ghz(f0).to_natural(),
    };
    Dispersive {
        eps_inf: 1.5,
        poles: vec![term(0.4, 20.0), term(0.6, 50.0)],
    }
}

/// The reflection coefficient at `frequencies` (GHz) on cells of `dz` µm at the Courant number
/// `courant`.
fn reflection(dz: f64, courant: f64, frequencies: &[f64]) -> Vec<c64> {
    let cells = |x: f64| (x / dz).round() as usize;
    let (cpml, before, after) = (cells(40.0 * DZ), cells(200.0 * DZ), cells(400.0 * DZ));
    let n = 2 * cpml + before + after;
    let grid = Grid3d {
        nx: n,
        ny: 1,
        nz: 1,
        dx: dz,
        dy: dz,
        dz,
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
    };
    let boundaries = Boundaries {
        x: Edges::Pml {
            low: cpml,
            high: cpml,
        },
        y: Edges::Bloch { k: 0.0 },
        z: Edges::Bloch { k: 0.0 },
        cpml: Cpml::default(),
    };
    let face = grid.node(Axis::X, cpml + before) - dz / 2.0;
    let (source, probe) = (
        cpml + before - cells(100.0 * DZ),
        cpml + before - cells(50.0 * DZ),
    );
    // no DC; its spectrum peaks at 25 GHz and is 1e-3 of that by 120 GHz
    let width = std::f64::consts::SQRT_2 / ghz(25.0).angular();
    let pulse = Waveform::DifferentiatedGaussian {
        width,
        delay: 6.0 * width,
    };
    let lorentz = medium();
    let run = |with_medium: bool| -> (Vec<f64>, f64) {
        let mut s = Simulation::new(grid, |_, _, _| 1.0, boundaries, courant).unwrap();
        if with_medium {
            s = s.with_medium(&lorentz, |x, _, _| x > face).unwrap();
        }
        s.add_source(Source {
            field: Field::E,
            component: Axis::Z,
            at: (source, 0, 0),
            waveform: pulse,
        })
        .unwrap();
        let p = s.add_probe(Field::E, Axis::Z, (probe, 0, 0)).unwrap();
        // 40 times the slower resonance's decay time, 1/δ₁ = 80 ps
        s.run_until(40.0 / (0.1 * ghz(20.0).angular()));
        (s.probe(p).to_vec(), s.dt())
    };
    let ((full, dt), (empty, _)) = std::thread::scope(|s| {
        let full = s.spawn(|| run(true));
        (full.join().unwrap(), run(false))
    });
    let distance = face - grid.node(Axis::X, probe);
    frequencies
        .iter()
        .map(|&f| {
            let w = ghz(f).angular();
            let dft = |series: &[f64]| -> c64 {
                series
                    .iter()
                    .enumerate()
                    .map(|(n, v)| v * c64::new(0.0, w * (n + 1) as f64 * dt).exp())
                    .sum()
            };
            let scattered: Vec<f64> = full.iter().zip(&empty).map(|(a, b)| a - b).collect();
            // the grid's wavenumber in vacuum: sin(kΔz/2)/Δz = sin(ωΔt/2)/Δt
            let k = 2.0 / dz * ((w * dt / 2.0).sin() * dz / dt).asin();
            dft(&scattered) / dft(&empty) * c64::new(0.0, -2.0 * k * distance).exp()
        })
        .collect()
}

/// The analytic reflection coefficient, (1 − n)/(1 + n).
fn analytic(f: f64) -> c64 {
    let n = medium().permittivity(ghz(f)).sqrt();
    (1.0 - n) / (1.0 + n)
}

/// The paper's LADES curve, read as a bound on |error| at `f` GHz: magnitude, phase.
fn bound(f: f64) -> (f64, f64) {
    // below 45 GHz the smooth part and its ripple; beyond, the values read, interpolated
    let magnitude = [
        (45.0, 0.70e-3),
        (52.0, 1.15e-3),
        (58.0, 1.68e-3),
        (59.5, 1.84e-3),
    ];
    let phase = [
        (50.0, 1.0e-3),
        (53.0, 0.77e-3),
        (56.0, 1.6e-3),
        (57.0, 2.1e-3),
    ];
    let at = |table: &[(f64, f64)], floor: f64| {
        if f <= table[0].0 {
            return floor;
        }
        for w in table.windows(2) {
            if f <= w[1].0 {
                let s = (f - w[0].0) / (w[1].0 - w[0].0);
                return (w[0].1 + s * (w[1].1 - w[0].1)).max(floor);
            }
        }
        table[table.len() - 1].1.max(floor)
    };
    (at(&magnitude, 0.8e-3), at(&phase, 1.0e-3))
}

pub fn main() -> ExitCode {
    let frequencies: Vec<f64> = (1..=24).map(|i| 2.5 * i as f64).collect();
    let frequencies = &frequencies;
    println!("Okoniewski, Mrozowski and Stuchly's two-term Lorentz half-space, normal incidence:");
    println!("the reflection coefficient against the analytic one (e^(-iwt): the phase error's");
    println!("sign is the opposite of the paper's, whose e^(+jwt) conjugates r)");
    let runs = [
        (DZ, 1.0, "dz = 37.5 um, C = 1"),
        (DZ, 0.5, "dz = 37.5 um, C = 0.5"),
        (DZ / 2.0, 1.0, "dz = 18.75 um, C = 1"),
    ];
    let results: Vec<Vec<c64>> = std::thread::scope(|s| {
        let handles: Vec<_> = runs
            .iter()
            .map(|&(dz, courant, _)| s.spawn(move || reflection(dz, courant, frequencies)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    // per run: the largest |Δ|r|| and |Δ phase|, and the largest of each over the paper's curve
    let mut worst = Vec::new();
    for (&(_, _, label), r) in runs.iter().zip(&results) {
        println!("{label}");
        println!("  f (GHz)   |r|        analytic   d|r|        d phase (rad)");
        let (mut m, mut p, mut over_m, mut over_p) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        for (&f, r) in frequencies.iter().zip(r) {
            let exact = analytic(f);
            let dm = r.norm() - exact.norm();
            let dp = (r / exact).arg();
            let (bm, bp) = bound(f);
            m = m.max(dm.abs());
            p = p.max(dp.abs());
            over_m = over_m.max(dm.abs() / bm);
            over_p = over_p.max(dp.abs() / bp);
            if f % 10.0 == 0.0 || f == 2.5 || f == 55.0 {
                println!(
                    "  {f:5.1}     {:.6}   {:.6}   {dm:+.2e}   {dp:+.2e}",
                    r.norm(),
                    exact.norm()
                );
            }
        }
        println!(
            "  largest |d|r|| {m:.2e} ({:.2} of the paper's curve), |d phase| {p:.2e} rad ({:.2})",
            over_m, over_p
        );
        worst.push((m, p, over_m, over_p));
    }
    let mut checks = common::Checks::default();
    for (i, which) in [(0, "C = 1"), (1, "C = 0.5")] {
        let (_, _, over_m, over_p) = worst[i];
        checks.against(
            &format!("{which}: |d|r|| over the curve"),
            over_m,
            "within it",
            0.0,
            1.0,
        );
        checks.against(
            &format!("{which}: |d phase| over the curve"),
            over_p,
            "within it",
            0.0,
            1.0,
        );
    }
    // halving the cell at C = 1 quarters the error: second order
    checks.against(
        "|d|r||, dz/2 over dz",
        worst[2].0 / worst[0].0,
        "second order",
        0.25,
        0.05,
    );
    checks.against(
        "|d phase|, dz/2 over dz",
        worst[2].1 / worst[0].1,
        "second order",
        0.25,
        0.05,
    );
    checks.finish()
}
