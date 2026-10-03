//! A ring tuned to critical coupling: an all-pass ring's coupling and radius, tuned by genoxide's
//! L-BFGS-B on the circuit adjoint's gradient until its through port is dark at 1.55 µm.
//!
//! W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012),
//! [doi:10.1002/lpor.201100017](https://doi.org/10.1002/lpor.201100017), Eqs. 2, 3 and 12: the
//! through power vanishes on a resonance, n_eff L = m λ (Eq. 3), at critical coupling, when the
//! self-coupling r equals the round trip's amplitude a, r = a (Eqs. 2 and 12). With a = 10^(−αL/20)
//! for a loss α, the closed forms are R = m λ / (2π n_eff) and κ² = 1 − a², which the tuned ring is
//! checked against.
//!
//! ```sh
//! cargo run --release --example circuit_ring_critical
//! ```

mod common;

use std::f64::consts::TAU;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use genoxide::prelude::*;
use photonoxide::circuit::{Circuit, Netlist, objective};
use photonoxide::units::Wavelength;

use common::circuit::{Coupler, Waveguide};

/// The wavelength the ring is tuned to, µm.
const LAMBDA: f64 = 1.55;
/// The ring's loss, dB/cm.
const LOSS: f64 = 3.0;
/// The silicon wire's effective and group indices at 1.55 µm.
const INDEX: f64 = 2.4;
const GROUP_INDEX: f64 = 4.2;

/// An all-pass ring (Bogaerts et al., Fig. 2A): a coupler whose output b2 returns to its input a2
/// through the ring; ports in and through.
fn ring(kappa2: f64, radius: f64) -> photonoxide::Result<Circuit> {
    let mut n = Netlist::new();
    n.add("coupler", Arc::new(Coupler::new()))?;
    n.add("ring", Arc::new(Waveguide::new(INDEX, GROUP_INDEX, LAMBDA)))?;
    n.set("coupler", "kappa2", kappa2)?;
    n.set("ring", "length", TAU * radius)?;
    n.set("ring", "loss", LOSS)?;
    n.connect("coupler.b2", "ring.a")?;
    n.connect("ring.b", "coupler.a2")?;
    n.expose("in", "coupler.a1")?;
    n.expose("through", "coupler.b1")?;
    n.compile()
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let (kappa2, radius) = (0.05, 9.97);
    let circuit = ring(kappa2, radius)?;
    let wavelength = [Wavelength::um(LAMBDA)?];
    let k = circuit.parameter("coupler.kappa2").expect("a parameter");
    let l = circuit.parameter("ring.length").expect("a parameter");
    // the genes: κ² and the radius R, the ring's length 2πR
    let real = Real::new([1e-4..=0.5, 9.9..=10.1]).expect("valid ranges");
    let fitness = |x: &Reals, gradient: &mut [f64]| {
        let mut values = circuit.values().to_vec();
        values[k] = x[0];
        values[l] = TAU * x[1];
        match circuit.gradient(&wavelength, &values, |s| objective::power(s, 1, 0)) {
            Ok((f, g)) => {
                gradient[0] = g[k];
                gradient[1] = TAU * g[l];
                f
            }
            Err(_) => f64::NAN,
        }
    };
    let lbfgsb = Lbfgsb::builder(real)
        .initial_genome(Reals::from(vec![kappa2, radius]))
        .gradient_tolerance(0.0)
        .function_tolerance(0.0)
        .minimize()
        .build()
        .expect("a valid L-BFGS-B");
    let outcome = Engine::new(lbfgsb, Differentiable(&fitness))
        .stop_when(
            Stop::target(1e-20)
                .or(Stop::evaluations(500))
                .or(Stop::time(Duration::from_secs(60))),
        )
        .run()
        .expect("a run");
    let best = outcome.best_genome();
    let (kappa2_found, radius_found) = (best[0], best[1]);
    let through = outcome.best_fitness().score().unwrap_or(f64::NAN);

    // the closed forms, for the resonance nearest the start
    let m = (INDEX * TAU * radius / LAMBDA).round();
    let radius_exact = m * LAMBDA / (TAU * INDEX);
    let a = 10f64.powf(-LOSS * 1e-4 * TAU * radius_exact / 20.0);
    let kappa2_exact = 1.0 - a * a;

    println!(
        "An all-pass ring, n_eff {INDEX}, n_g {GROUP_INDEX}, {LOSS} dB/cm, tuned at {LAMBDA} µm (Bogaerts et al. 2012), exact: no grid"
    );
    println!(
        "  from κ² = {kappa2}, R = {radius} µm: L-BFGS-B, {} evaluations, each the circuit solve and one adjoint solve",
        outcome.evaluations()
    );
    println!(
        "  resonance order m = {m}: R = m λ/(2π n_eff), a = {a:.6}, critical κ² = 1 − a² (Eqs. 3 and 2)"
    );
    let mut checks = common::Checks::default();
    checks.compare("radius (µm)", radius_found, round(radius_exact, 9), 1e-9);
    checks.compare(
        "κ² (× 1e-3)",
        kappa2_found * 1e3,
        round(kappa2_exact * 1e3, 9),
        1e-6,
    );
    let r_over_a =
        (1.0 - kappa2_found).sqrt() / 10f64.powf(-LOSS * 1e-4 * TAU * radius_found / 20.0);
    checks.compare("r / a (Eq. 2: 1)", r_over_a, 1.0, 1e-9);
    checks.compare("through power (Eq. 12: 0)", through, 0.0, 1e-12);
    Ok(checks.finish())
}

/// `x` to `decimals` decimals, as printed.
fn round(x: f64, decimals: i32) -> f64 {
    let scale = 10f64.powi(decimals);
    (x * scale).round() / scale
}
