//! The rules (#544) of the cases of third-order processes (0.6.1,
//! [`crate::nonlinear::kerr_checks`]): the nonlinear Schrödinger equation, four-wave mixing
//! and the Lugiato–Lefever equation. The closed forms hold to each scheme's own error, the
//! tolerance that error on half the steps (or the coarser grid) rounded up; the orders to 0.1;
//! Coen et al.'s soliton to half a unit of its one printed digit.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    let mut rules = pulse_rules();
    rules.extend(fwm_rules());
    rules.extend(lle_rules());
    rules
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

/// Four-wave mixing.
fn fwm_rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "nonlinear/fwm-gain-matched",
            grid(
                Relative,
                2e-9,
                "the error on 400 steps, 1.89e-9, rounded up: RK4's, fourth order (4.7e-7, 3.0e-8, 1.9e-9, 1.4e-10 on 100 to 800); the undepleted closed form's own error is the signal's 1e-12 of the pump's power",
            ),
        ),
        (
            "nonlinear/fwm-gain-mismatched",
            grid(
                Relative,
                4e-11,
                "the error on 400 steps, 3.3e-11, rounded up: RK4's (9.1e-9, 5.3e-10, 3.3e-11, 3.4e-12 on 100 to 800)",
            ),
        ),
        (
            "nonlinear/fwm-invariants",
            grid(
                Absolute,
                5e-10,
                "the drift on 400 steps, 4.0e-10, rounded up: the invariants are exact for the equations, RK4 keeps them to its own error (1.0e-7, 6.4e-9, 4.0e-10, 2.5e-11 on 100 to 800)",
            ),
        ),
        (
            "nonlinear/fwm-nlse-gain",
            grid(
                Relative,
                3e-6,
                "the error on 1000 steps, 2.57e-6, rounded up: the split-step's, second order (1.0e-5, 2.6e-6, 6.4e-7, 1.6e-7 on 500 to 4000)",
            ),
        ),
        (
            "nonlinear/fwm-sfwm-purity",
            grid(
                Relative,
                1e-11,
                "on 201 points a side the grid's error is below the SVD's round-off: 1.0e-13, and 5.0e-14 on 301 (on 101 it is 2.7e-8), as for the down-converted pairs",
            ),
        ),
    ]
}

/// The Lugiato–Lefever equation.
fn lle_rules() -> Vec<(&'static str, Rule)> {
    let homogeneous = "the upper state's error on 6400 steps, 2.22e-5, rounded up: the split-step's steady state is second order in the step (upper 3.6e-4, 8.9e-5, 2.2e-5, 5.6e-6 and lower 6.1e-5, 1.5e-5, 3.8e-6, 9.5e-7 on 1600 to 12 800)";
    vec![
        (
            "nonlinear/lle-homogeneous-lower",
            grid(Relative, 3e-5, homogeneous),
        ),
        (
            "nonlinear/lle-homogeneous-upper",
            grid(Relative, 3e-5, homogeneous),
        ),
        (
            "nonlinear/lle-instability-rate",
            grid(
                Relative,
                6e-6,
                "the error on 3200 steps, 5.32e-6, rounded up: second order (8.5e-5, 2.1e-5, 5.3e-6 and 1.4e-6 on 800 to 6400)",
            ),
        ),
        (
            "nonlinear/lle-split-step-order",
            order(
                "second order within 0.1: the halvings from 800 steps give 2.00, 2.00 and 1.96 on the growth rate",
            ),
        ),
        (
            "nonlinear/lle-coen-2012-duration",
            Rule::new(
                Absolute,
                Reference,
                50.0,
                Noted,
                "half a unit of the one significant digit of the printed '∼400 fs': the paper states no other uncertainty for the soliton its Fig. 2(a)'s inset shows",
            ),
        ),
    ]
}
