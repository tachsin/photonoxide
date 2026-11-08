//! The rules (#544) of rigorous coupled-wave analysis's cases (#743): uniform layers and lossless
//! gratings at round-off, and the orders in which the grating approaches its static limit and
//! TM converges in the harmonics.
use super::{Basis::*, Measure::*, Numerical::*, Rule};
pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "mode/rcwa-uniform-multilayer",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "a uniform layer's modes are its plane waves, so the scattering matrices are the transfer matrices' algebra rearranged: round-off",
            ),
        ),
        (
            "mode/rcwa-energy-lossless",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "the truncated system of a lossless grating conserves the flux exactly, its matrices Hermitian: round-off of 41 × 41 eigenproblems and products",
            ),
        ),
        (
            "mode/rcwa-effective-medium-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "TE's second order and TM's first, within 0.1",
            ),
        ),
        (
            "mode/rcwa-tm-convergence",
            Rule::new(
                Absolute,
                Order,
                0.05,
                Measured,
                "Laurent's rule at the walls: first order in 1/M, within 0.05",
            ),
        ),
    ]
}
