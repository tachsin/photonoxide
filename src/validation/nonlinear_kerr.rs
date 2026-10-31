//! The rules (#544) of the cases of third-order processes (0.6.1,
//! [`crate::nonlinear::kerr_checks`]): the nonlinear Schrödinger equation. The closed forms
//! hold to each scheme's own error, the tolerance that error on half the steps (or the coarser
//! grid) rounded up; the order to 0.1.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    pulse_rules()
}

fn grid(measure: super::Measure, tolerance: f64, why: &'static str) -> Rule {
    Rule::new(measure, Discretisation, tolerance, Measured, why).varying()
}

fn order(why: &'static str) -> Rule {
    Rule::new(Absolute, Order, 0.1, Measured, why)
}

/// The NLSE and the dispersion it takes.
fn pulse_rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "nonlinear/nlse-dispersion-silica-beta2",
            grid(
                Relative,
                2e-8,
                "the fit's own estimate, 4.9e-9 (the difference of its two best-agreeing interpolants), times three, rounded up: the samples' round-off magnified in a second derivative (the error is 5.6e-9)",
            ),
        ),
        (
            "nonlinear/nlse-dispersion-silica-beta4",
            grid(
                Relative,
                1e-5,
                "five times the fit's own estimate, 1.9e-6, rounded up: the error, 2.1e-6, is the samples' round-off magnified in a fourth derivative, which moves between systems",
            ),
        ),
        (
            "nonlinear/nlse-fundamental-soliton",
            grid(
                Absolute,
                2e-5,
                "the error on 2000 steps, 1.11e-5, rounded up: second order (1.8e-4, 4.4e-5, 1.1e-5, 2.8e-6 and 7.0e-7 on 500 to 8000), so 4000's is a quarter of it",
            ),
        ),
        (
            "nonlinear/nlse-second-order-soliton",
            grid(
                Absolute,
                2e-5,
                "the error on 2000 steps, 1.65e-5, rounded up: second order (2.6e-4, 6.6e-5, 1.6e-5, 4.1e-6 and 1.0e-6 on 500 to 8000)",
            ),
        ),
        (
            "nonlinear/nlse-third-order-period",
            grid(
                Absolute,
                6e-5,
                "the error on 4000 steps, 5.02e-5, rounded up: second order (8.1e-4, 2.0e-4, 5.0e-5 and 1.3e-5 on 1000 to 8000), the same on 2048 points as on 1024",
            ),
        ),
        (
            "nonlinear/nlse-split-step-order",
            order(
                "second order within 0.1: the halvings from 500 steps give 2.00, 2.00 and 2.00 on the second-order soliton",
            ),
        ),
        (
            "nonlinear/nlse-spm-phase",
            grid(
                Relative,
                2e-6,
                "the error on 200 steps, 1.99e-6, rounded up: the midpoint rule of the loss in the lumped nonlinear step, second order (7.9e-6, 2.0e-6, 5.0e-7, 1.2e-7 on 100 to 800)",
            ),
        ),
        (
            "nonlinear/nlse-spm-broadening",
            grid(
                Relative,
                1e-12,
                "without dispersion the step is exact; on 1024 points the time grid's error is at round-off (1.8e-8 on 512, 2.2e-16 on 1024 and 2048)",
            ),
        ),
        (
            "nonlinear/nlse-photon-number",
            Rule::new(
                Absolute,
                Discretisation,
                2e-9,
                Measured,
                "the drift on 1000 steps, 1.36e-9, rounded up: the RK4 nonlinear step's (2.0e-8, 1.4e-9, 8.7e-11 and 5.8e-12 on 500 to 4000)",
            )
            .varying(),
        ),
    ]
}
