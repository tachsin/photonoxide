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
            "mode/rcwa-effective-medium-faces",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "the bulk's second order (a mirror-symmetric cell) and the faces' first, within 0.1",
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
        (
            "mode/rcwa-tangent-multilayer",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "both derivatives are exact, the transfer matrices' product rule and RCWA's forward mode through plane waves: round-off of a few products, as mode/rcwa-uniform-multilayer",
            ),
        ),
        (
            "mode/rcwa-tangent-gradient",
            Rule::new(
                Absolute,
                Discretisation,
                5e-7,
                Noted,
                "fourth-order differences of efficiencies at δ = 1e-4: what they resolve, the difference between δ and 2δ, is 2.8e-8 (noted; mostly 2δ's truncation, sixteen times δ's), and forward mode is within 1.9e-9 of them; held to ten times what they resolve, rounded up, 5e-7",
            )
            .varying(),
        ),
        (
            "mode/rcwa-tangent-born",
            Rule::new(
                Relative,
                Order,
                crate::mode::adjoint_checks::BORN_ORDER * 2.5e-3,
                Measured,
                "the Born approximation's next term, first order in Δ: the gap is 0.239 to 0.240 Δ at Δ = 0.02, 0.01, 0.005 and 0.0025 (the_mode_adjoint_study), halving with Δ; held to 0.3 Δ, 7.5e-4 at Δ = 2.5e-3",
            ),
        ),    ]
}
