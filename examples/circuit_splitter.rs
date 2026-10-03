//! A tunable splitter: the phase in one arm of a Mach–Zehnder interferometer, tuned by genoxide's
//! L-BFGS-B and Adam on the circuit adjoint's gradient until 30% of the light stays in the bar
//! port.
//!
//! W. R. Clements et al., Optica 3, 1460 (2016),
//! [doi:10.1364/OPTICA.3.001460](https://doi.org/10.1364/OPTICA.3.001460), Eq. 1 and Fig. 1(c):
//! two 50:50 couplers with a phase θ between them are a variable beam splitter. Straight through
//! both couplers, r e^(iθ) r + (iκ)(iκ) = (e^(iθ) − 1)/2, so the bar port carries sin²(θ/2) and
//! 30% needs θ = 2 arcsin √0.3 = 1.159279 rad, the closed form the result is checked against
//! (to 1e-6 rad).
//!
//! ```sh
//! cargo run --release --example circuit_splitter
//! ```

mod common;

use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use genoxide::prelude::*;
use photonoxide::circuit::{Circuit, Netlist, objective};
use photonoxide::units::Wavelength;

use common::circuit::{Coupler, PhaseShifter};

/// The bar power wanted.
const TARGET: f64 = 0.3;

/// An MZI of two 50:50 couplers, a phase shifter in each arm; ports in1, in2, out1, out2.
fn mzi() -> photonoxide::Result<Circuit> {
    let coupler = Arc::new(Coupler::new());
    let shifter = Arc::new(PhaseShifter::new());
    let mut n = Netlist::new();
    n.add("c1", coupler.clone())?;
    n.add("c2", coupler)?;
    n.add("upper", shifter.clone())?;
    n.add("lower", shifter)?;
    n.set("upper", "phase", 2.0)?;
    for (a, b) in [
        ("c1.b1", "upper.a"),
        ("upper.b", "c2.a1"),
        ("c1.b2", "lower.a"),
        ("lower.b", "c2.a2"),
    ] {
        n.connect(a, b)?;
    }
    for (name, port) in [
        ("in1", "c1.a1"),
        ("in2", "c1.a2"),
        ("out1", "c2.b1"),
        ("out2", "c2.b2"),
    ] {
        n.expose(name, port)?;
    }
    n.compile()
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let circuit = mzi()?;
    let wavelength = [Wavelength::um(1.55)?];
    // the one gene: the upper arm's phase, in (0, π) where the bar power rises with it
    let k = circuit.parameter("upper.phase").expect("a parameter");
    let start = circuit.values()[k];
    let real = Real::new([0.0..=std::f64::consts::PI]).expect("a valid range");
    // F = (|S_out1,in1|² − 0.3)² and dF/dθ from the circuit adjoint
    let fitness = |x: &Reals, gradient: &mut [f64]| {
        let mut values = circuit.values().to_vec();
        values[k] = x[0];
        match circuit.gradient(&wavelength, &values, |s| {
            objective::power_error(s, 2, 0, &[TARGET])
        }) {
            Ok((f, g)) => {
                gradient[0] = g[k];
                f
            }
            Err(_) => f64::NAN,
        }
    };
    let stop = || {
        Stop::target(1e-20)
            .or(Stop::evaluations(2_000))
            .or(Stop::time(Duration::from_secs(60)))
    };
    // the closed form, to the 9 decimals printed
    let exact = (2.0 * TARGET.sqrt().asin() * 1e9).round() / 1e9;
    println!(
        "An MZI of two 50:50 couplers, phase θ in one arm, at 1.55 µm (Clements et al. 2016, Eq. 1), exact: no grid"
    );
    println!("  from θ = {start} rad, to {TARGET} of the power in the bar port");
    let mut checks = common::Checks::default();

    let lbfgsb = Lbfgsb::builder(real.clone())
        .initial_genome(Reals::from(vec![start]))
        .gradient_tolerance(1e-12)
        .function_tolerance(0.0)
        .minimize()
        .build()
        .expect("a valid L-BFGS-B");
    let outcome = Engine::new(lbfgsb, Differentiable(&fitness))
        .stop_when(stop())
        .run()
        .expect("a run");
    let theta = outcome.best_genome()[0];
    println!(
        "  L-BFGS-B: {} evaluations, each the circuit solve and one adjoint solve",
        outcome.evaluations()
    );
    checks.compare("θ by L-BFGS-B (rad)", theta, exact, 1e-6);
    let mut values = circuit.values().to_vec();
    values[k] = theta;
    let power = circuit.s_matrix_with(wavelength[0], &values)?.power(2, 0);
    checks.compare("bar power", power, TARGET, 1e-9);

    let adam = FirstOrder::builder(real)
        .step(first_order::Step::adam(0.02))
        .initial_genome(Reals::from(vec![start]))
        .minimize()
        .build()
        .expect("a valid Adam");
    let outcome = Engine::new(adam, Differentiable(&fitness))
        .stop_when(Stop::target(1e-14).or(stop()))
        .run()
        .expect("a run");
    let theta = outcome.best_genome()[0];
    println!(
        "  Adam (learning rate 0.02): {} evaluations",
        outcome.evaluations()
    );
    checks.compare("θ by Adam (rad)", theta, exact, 1e-6);
    Ok(checks.finish())
}
