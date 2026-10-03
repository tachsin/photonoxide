//! Fitting a ring to its spectra, with and without the gradient: an add-drop ring's two couplings,
//! its loss and its radius, fitted to through and drop spectra by genoxide's L-BFGS-B on the
//! circuit adjoint's gradient and by its CMA-ES on the circuit's response alone, to the same
//! target, counting evaluations.
//!
//! W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012),
//! [doi:10.1002/lpor.201100017](https://doi.org/10.1002/lpor.201100017), Eqs. 5 and 6 (the
//! through and drop intensities of an add-drop ring) make the "measured" spectra, from r₁, r₂, a
//! and φ; their Section 3.3 extracts coupling and loss from measured spectra the same way. Both
//! fits must return the parameters the spectra were made with.
//!
//! ```sh
//! cargo run --release --example circuit_fit
//! ```

mod common;

use std::f64::consts::{PI, TAU};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use genoxide::prelude::*;
use photonoxide::circuit::{Circuit, Netlist, SMatrix, objective};
use photonoxide::units::Wavelength;

use common::circuit::{Coupler, Waveguide};

/// The silicon wire: n_eff and n_g at λ₀ (µm).
const INDEX: f64 = 2.4;
const GROUP_INDEX: f64 = 4.2;
const LAMBDA: f64 = 1.55;

/// The ring the spectra are made from: κ₁², κ₂², the loss (dB/cm) and the radius (µm).
const TRUTH: [f64; 4] = [0.08, 0.05, 20.0, 10.0];
/// How close each fit must come: F below 1e-18 leaves about 1e-9 in the couplings, 1e-7 dB/cm
/// in the loss and 1e-12 µm in the radius.
const TOLERANCE: [f64; 4] = [1e-8, 1e-8, 1e-5, 1e-9];
/// Where both fits start.
const START: [f64; 4] = [0.12, 0.03, 10.0, 10.002];

/// An add-drop ring (Bogaerts et al., Fig. 2B): couplers c1 and c2 joined by two halves of the
/// ring; ports in, through, add, drop.
fn ring() -> photonoxide::Result<Circuit> {
    let coupler = Arc::new(Coupler::new());
    let half = Arc::new(Waveguide::new(INDEX, GROUP_INDEX, LAMBDA));
    let mut n = Netlist::new();
    n.add("c1", coupler.clone())?;
    n.add("c2", coupler)?;
    n.add("top", half.clone())?;
    n.add("bottom", half)?;
    for (a, b) in [
        ("c1.b2", "top.a"),
        ("top.b", "c2.a2"),
        ("c2.b2", "bottom.a"),
        ("bottom.b", "c1.a2"),
    ] {
        n.connect(a, b)?;
    }
    for (name, port) in [
        ("in", "c1.a1"),
        ("through", "c1.b1"),
        ("add", "c2.a1"),
        ("drop", "c2.b1"),
    ] {
        n.expose(name, port)?;
    }
    n.compile()
}

/// Bogaerts et al.'s Eqs. 5 and 6 for the ring `x` at `wavelength_um`: the through and drop
/// intensities.
fn bogaerts(guide: &Waveguide, x: &[f64; 4], wavelength_um: f64) -> (f64, f64) {
    let (r1, r2) = ((1.0 - x[0]).sqrt(), (1.0 - x[1]).sqrt());
    let length = TAU * x[3];
    let a = 10f64.powf(-x[2] * 1e-4 * length / 20.0);
    let phi = TAU * guide.effective_index(wavelength_um) * length / wavelength_um;
    let denominator = 1.0 - 2.0 * r1 * r2 * a * phi.cos() + (r1 * r2 * a).powi(2);
    let through = (r2 * r2 * a * a - 2.0 * r1 * r2 * a * phi.cos() + r1 * r1) / denominator;
    let drop = (1.0 - r1 * r1) * (1.0 - r2 * r2) * a / denominator;
    (through, drop)
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let circuit = ring()?;
    let guide = Waveguide::new(INDEX, GROUP_INDEX, LAMBDA);
    // 1.548 to 1.556 µm in steps of 0.1 nm, across one resonance (1.5522 µm)
    let wavelengths: Vec<Wavelength> = (0..81)
        .map(|i| Wavelength::um(1.548 + 1e-4 * f64::from(i)))
        .collect::<photonoxide::Result<_>>()?;
    let (through, drop): (Vec<f64>, Vec<f64>) = wavelengths
        .iter()
        .map(|w| bogaerts(&guide, &TRUTH, w.to_um()))
        .unzip();
    let at = |name: &str| circuit.parameter(name).expect("a parameter");
    let (c1, c2) = (at("c1.kappa2"), at("c2.kappa2"));
    let (top, bottom) = (at("top.length"), at("bottom.length"));
    let (top_loss, bottom_loss) = (at("top.loss"), at("bottom.loss"));
    // the genes: κ₁², κ₂², the loss, the radius, each half of the ring πR long
    let values = |x: &[f64]| {
        let mut v = circuit.values().to_vec();
        v[c1] = x[0];
        v[c2] = x[1];
        v[top_loss] = x[2];
        v[bottom_loss] = x[2];
        v[top] = PI * x[3];
        v[bottom] = PI * x[3];
        v
    };
    // F = Σ_λ (|S_through|² − T)² + (|S_drop|² − D)²: the two objectives' values and
    // sensitivities add
    let misfit = |s: &[SMatrix]| {
        let (f, mut g) = objective::power_error(s, 1, 0, &through);
        let (f_drop, g_drop) = objective::power_error(s, 3, 0, &drop);
        for (g, d) in g.iter_mut().zip(&g_drop) {
            g[(3, 0)] = d[(3, 0)];
        }
        (f + f_drop, g)
    };
    let gradient_fitness = |x: &Reals, gradient: &mut [f64]| match circuit.gradient(
        &wavelengths,
        &values(x),
        misfit,
    ) {
        Ok((f, g)) => {
            gradient[0] = g[c1];
            gradient[1] = g[c2];
            gradient[2] = g[top_loss] + g[bottom_loss];
            gradient[3] = PI * (g[top] + g[bottom]);
            f
        }
        Err(_) => f64::NAN,
    };
    let value_fitness = |x: &Reals| {
        let v = values(x);
        let s = wavelengths
            .iter()
            .map(|&w| circuit.s_matrix_with(w, &v))
            .collect::<photonoxide::Result<Vec<_>>>();
        s.map_or(f64::NAN, |s| misfit(&s).0)
    };
    let real = Real::new([0.01..=0.3, 0.01..=0.3, 0.0..=50.0, 9.99..=10.01]).expect("valid ranges");
    let stop = || {
        Stop::target(1e-18)
            .or(Stop::evaluations(20_000))
            .or(Stop::time(Duration::from_secs(120)))
    };

    println!(
        "An add-drop ring's through and drop spectra, 81 wavelengths from 1.548 to 1.556 µm, made by Bogaerts et al.'s Eqs. 5 and 6, exact: no grid"
    );
    println!(
        "  made with κ₁² = {}, κ₂² = {}, {} dB/cm, R = {} µm; both fits from κ₁² = {}, κ₂² = {}, {} dB/cm, R = {} µm, to F below 1e-18",
        TRUTH[0], TRUTH[1], TRUTH[2], TRUTH[3], START[0], START[1], START[2], START[3]
    );
    let mut checks = common::Checks::default();
    let mut report = |name: &str, outcome: &genoxide::engine::Outcome<Reals>| {
        let x = outcome.best_genome();
        for (i, label) in ["κ₁²", "κ₂²", "loss (dB/cm)", "radius (µm)"]
            .iter()
            .enumerate()
        {
            checks.compare(&format!("{name}: {label}"), x[i], TRUTH[i], TOLERANCE[i]);
        }
    };

    let lbfgsb = Lbfgsb::builder(real.clone())
        .initial_genome(Reals::from(START.to_vec()))
        .gradient_tolerance(0.0)
        .function_tolerance(0.0)
        .minimize()
        .build()
        .expect("a valid L-BFGS-B");
    let outcome = Engine::new(lbfgsb, Differentiable(&gradient_fitness))
        .stop_when(stop())
        .run()
        .expect("a run");
    println!(
        "  L-BFGS-B with the adjoint gradient: {} evaluations, each 81 circuit solves and 81 adjoint solves",
        outcome.evaluations()
    );
    let lbfgsb_evaluations = outcome.evaluations();
    report("L-BFGS-B", &outcome);

    let cmaes = Cmaes::builder(real)
        .initial_mean(Reals::from(START.to_vec()))
        .initial_step(0.1)
        .minimize()
        .seed(1)
        .build()
        .expect("a valid CMA-ES");
    let outcome = Engine::new(cmaes, value_fitness)
        .stop_when(stop())
        .run()
        .expect("a run");
    println!(
        "  CMA-ES (seed 1), the response alone: {} evaluations, each 81 circuit solves",
        outcome.evaluations()
    );
    report("CMA-ES", &outcome);
    println!(
        "  CMA-ES took {:.0} times as many evaluations",
        outcome.evaluations() as f64 / lbfgsb_evaluations as f64
    );
    Ok(checks.finish())
}
