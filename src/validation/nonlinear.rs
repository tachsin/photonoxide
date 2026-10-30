//! The rules (#544) of phase matching's cases (0.6.1, [`crate::nonlinear::phase`]): two exact
//! identities, Fejer et al.'s Table I to half a unit of each value's last printed digit (each
//! case's error is the largest over its rows, in those halves, so its tolerance is 1), and Wang
//! et al.'s duty cycle to its stated uncertainty.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

const TABLE_1: &str = "half a unit of each value's last printed digit, the error measured in those halves: Fejer et al. state no other uncertainty, and computed the table with the indices used here";

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    let table = |numerical| Rule::new(Absolute, Reference, 1.0, numerical, TABLE_1);
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
    ]
}
