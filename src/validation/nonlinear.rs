//! The rules (#544) of phase matching's cases (0.6.1, [`crate::nonlinear::phase`]): two exact
//! identities, Fejer et al.'s Table I to half a unit of each value's last printed digit (each
//! case's error is the largest over its rows, in those halves, so its tolerance is 1), and Wang
//! et al.'s duty cycle to its stated uncertainty.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

const TABLE_1: &str = "half a unit of each value's last printed digit, the error measured in those halves: Fejer et al. state no other uncertainty, and computed the table with the indices used here";

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    let table = |numerical| Rule::new(Absolute, Reference, 1.0, numerical, TABLE_1);
    let grid =
        |tolerance, why| Rule::new(Relative, Discretisation, tolerance, Measured, why).varying();
    vec![
        (
            "nonlinear/sinc2-half-maximum",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "sin x / x at the root, 1.3915573782515103 to the last bit, is 1/√2 to round-off",
            ),
        ),
        (
            "nonlinear/sinc2-length-squared",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the grating cancels the mismatch to round-off, so the powers are L² to round-off",
            ),
        ),
        // the coherence length has no finite differences: our error is round-off
        ("nonlinear/fejer-table-1-coherence", table(Exact)),
        ("nonlinear/fejer-table-1-wavelength", table(Noted)),
        ("nonlinear/fejer-table-1-temperature", table(Noted)),
        ("nonlinear/fejer-table-1-tilt-about-z", table(Noted)),
        ("nonlinear/fejer-table-1-tilt-toward-z", table(Noted)),
        (
            "nonlinear/wang-2018-duty-cycle",
            Rule::new(
                Absolute,
                Reference,
                3.6,
                Exact,
                "the paper's ±3 % on the 39 % duty cycle: at 36 % the cut is 3.61 points larger, at 42 % 2.77 smaller; the larger, rounded down",
            ),
        ),
        (
            "nonlinear/coupled-undepleted",
            grid(2e-13, "the error on 500 steps, 1.77e-13, rounded up: on 1000 it is 1.2e-14, the fourth order nonlinear/coupled-rk4-order checks"),
        ),
        (
            "nonlinear/coupled-lossy",
            grid(2e-13, "the error on 500 steps, 1.72e-13, rounded up: on 1000 it is 1.2e-14"),
        ),
        (
            "nonlinear/coupled-shg-tanh",
            grid(1e-12, "the error on 500 steps, 9.3e-13, rounded up: on 1000 it is 5.7e-14"),
        ),
        (
            "nonlinear/coupled-sfg-sech",
            grid(1e-12, "the error on 500 steps, 9.3e-13, rounded up: on 1000 it is 5.7e-14"),
        ),
        (
            "nonlinear/coupled-opa-cosh",
            grid(9e-12, "the error on 500 steps, 8.2e-12, rounded up: on 1000 it is 5.1e-13"),
        ),
        (
            "nonlinear/coupled-manley-rowe",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Measured,
                "the invariants are exact for the equations; RK4's drift from them, 9.2e-14 on 250 steps and 2.4e-15 on 500, is at round-off on 1000",
            )
            .varying(),
        ),
        (
            "nonlinear/pairs-gaussian-purity",
            grid(1e-11, "on 201 points a side the grid's error is below the SVD's round-off: 3.4e-13, and 4.4e-13 and 2.3e-12 on 301 and 401 (on 101 it is 1.3e-3)"),
        ),
        (
            "nonlinear/coupled-rk4-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "fourth order within 0.1: the halvings from 125 steps give 4.06, 4.03 and 4.04, before round-off",
            ),
        ),
    ]
}
