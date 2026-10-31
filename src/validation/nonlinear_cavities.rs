//! The rules (#544) of 0.6.1's last physics cases: two-photon and free-carrier absorption and
//! cross-phase modulation ([`crate::nonlinear::absorption_checks`]), and the resonant processes,
//! heralding and parametric oscillators ([`crate::nonlinear::cavity_checks`]). The closed forms
//! hold to each scheme's own error, the tolerance that error on half the steps rounded up; the
//! orders to 0.1.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    let mut rules = absorption_rules();
    rules.extend(cavity_rules());
    rules
}

fn grid(measure: super::Measure, tolerance: f64, why: &'static str) -> Rule {
    Rule::new(measure, Discretisation, tolerance, Measured, why).varying()
}

fn order(why: &'static str) -> Rule {
    Rule::new(Absolute, Order, 0.1, Measured, why)
}

/// Two-photon and free-carrier absorption, and cross-phase modulation.
fn absorption_rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "nonlinear/nlse-tpa-power",
            grid(
                Relative,
                8e-7,
                "the error on 200 steps, 7.19e-7, rounded up: the loss's midpoint rule in the split-step, second order (2.9e-6, 7.2e-7, 1.8e-7 and 4.5e-8 on 100 to 800); the nonlinear step is exact",
            ),
        ),
        (
            "nonlinear/nlse-tpa-phase",
            grid(
                Relative,
                5e-7,
                "the error on 200 steps, 4.28e-7, rounded up: the loss's midpoint rule, second order (1.7e-6, 4.3e-7, 1.1e-7 and 2.7e-8 on 100 to 800)",
            ),
        ),
        (
            "nonlinear/nlse-tpa-order",
            order(
                "second order within 0.1: the halvings from 100 steps give 2.00, 2.00 and 2.00 on the peak power",
            ),
        ),
        (
            "nonlinear/nlse-carriers-gaussian",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Measured,
                "the trapezoidal rule on a Gaussian sampled 16 times to T₀ is exact to round-off: 2.2e-16 to 6.7e-16 on 256 to 2048 points",
            ),
        ),
        (
            "nonlinear/nlse-fca-cw-power",
            grid(
                Relative,
                3e-7,
                "the error on 50 steps, 2.77e-7, rounded up: RK4's, fourth order (4.4e-6, 2.8e-7, 1.7e-8 and 1.1e-9 on 25 to 200); the carriers at the window's end are steady to e⁻⁴⁰",
            ),
        ),
        (
            "nonlinear/nlse-fca-cw-phase",
            grid(
                Relative,
                4e-6,
                "the error on 50 steps, 3.50e-6, rounded up: RK4's, fourth order (5.7e-5, 3.5e-6, 2.2e-7 and 1.3e-8 on 25 to 200)",
            ),
        ),
        (
            "nonlinear/xpm-walk-off-phase",
            grid(
                Absolute,
                8e-6,
                "the error on 1000 steps, 7.14e-6, rounded up: the walk-off between Kerr steps, second order (2.9e-5, 7.1e-6 and 1.8e-6 on 500 to 2000; 1.3e-6 on 4000, where another error of about 1e-6 is reached)",
            ),
        ),
        (
            "nonlinear/xpm-split-step-order",
            order("second order within 0.1: the halvings from 500 steps give 2.00 and 2.00"),
        ),
    ]
}

/// Rings, heralding and parametric oscillators.
fn cavity_rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "nonlinear/ring-shg-enhancement",
            grid(
                Relative,
                2e-10,
                "the error on 10 steps a pass, 1.03e-10, rounded up: RK4's (1.8e-9 on 5), then a floor near 1e-11 (1.0e-12 on 20, 7.7e-12 on 40): the round trips stopped at a change of 1e-14, and the depletion the closed form leaves out",
            ),
        ),
        (
            "nonlinear/ring-fwm-enhancement",
            grid(
                Relative,
                1e-12,
                "twice the largest error on 5 to 40 steps a pass, 4.4e-13, rounded up: every step count is at the iteration's floor (4.4e-13, 4.0e-13, 2.9e-13 and 1.3e-13), the pass's integrand smooth and RK4's error below it",
            ),
        ),
        (
            "nonlinear/pairs-heralding-gaussian",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Measured,
                "the sums of a Gaussian sampled finely are its integral to round-off: 3.3e-15 to 2.2e-14 on 101 to 301 points a side, growing with the sums' length",
            )
            .varying(),
        ),
        (
            "nonlinear/pairs-filtered-purity",
            Rule::new(
                Relative,
                RoundOff,
                1e-11,
                Measured,
                "the singular value decomposition's round-off, as for the down-converted pairs' purity: 1.3e-15 to 4.4e-15 on 101 to 301 points a side",
            )
            .varying(),
        ),
        (
            "nonlinear/opo-threshold-doubly",
            Rule::new(
                Relative,
                Discretisation,
                1e-13,
                Measured,
                "the bisection's stop, 1e-13 relative: RK4's error is below it on 50 to 400 steps (the error 2.6e-14 to 5.0e-14)",
            )
            .varying(),
        ),
        (
            "nonlinear/opo-threshold-singly",
            grid(
                Relative,
                8e-12,
                "the error on 100 steps, 7.89e-12, rounded up: RK4's, fourth order (1.3e-10, 7.9e-12, 4.9e-13 and 6.5e-14 on 50 to 400)",
            ),
        ),
        (
            "nonlinear/opo-steady-photons",
            grid(
                Absolute,
                9e-10,
                "the imbalance on 100 steps a pass, 8.06e-10, rounded up: RK4's (8.1e-10, 5.0e-11 and 2.3e-12 on 100 to 400); the round trips settled to 1e-13",
            ),
        ),
        (
            "nonlinear/opo-uniform-limit-order",
            order(
                "first order within 0.1: the gap at 4, 2, 1 and 0.5 % loss per round trip is 1.56e-3, 7.75e-4, 3.86e-4 and 1.93e-4, halvings of 1.01, 1.00 and 1.00",
            ),
        ),
    ]
}
