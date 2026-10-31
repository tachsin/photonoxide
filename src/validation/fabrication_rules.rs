//! The rules (#544) of the fabrication rule engine's cases (F4 of #622): a textbook's table and
//! worked example to their printing's rounding; the library's minimum features against the
//! Rayleigh limit and the rules' choices, which are categorical; the accuracy read-out's
//! combination and the round trip over the library's devices, exact by construction.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    let exact = |why: &'static str| Rule::new(Absolute, RoundOff, 0.0, Exact, why);
    vec![
        (
            "fabrication/rayleigh-franssila",
            Rule::new(
                Absolute,
                Reference,
                1.0,
                Exact,
                "half a unit of the last printed digit of each linewidth: a misfit over that allowance of at most 1",
            ),
        ),
        (
            "fabrication/deal-grove-campbell",
            Rule::new(
                Absolute,
                Reference,
                0.005,
                Exact,
                "half a unit of the last printed digit of 0.48 µm",
            ),
        ),
        (
            "fabrication/min-features-above-limit",
            exact(
                "a comparison is categorical: no minimum feature may ask for a k₁ below the limit",
            ),
        ),
        (
            "fabrication/rule-choices",
            exact("a choice is categorical: no wrong answer is allowed"),
        ),
        (
            "fabrication/accuracy-combined",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "an exact identity: the read-out's root-sum-square of the same two numbers, to the arithmetic's round-off",
            ),
        ),
        (
            "fabrication/round-trip-library",
            exact(
                "an exact identity: the executed flow draws the design's own shapes and etches, so the structures are equal to the bit and no point may differ",
            ),
        ),
    ]
}
