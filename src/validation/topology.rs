//! The rules (#544) of the densities' cases (0.7, #761): the projection's and the filter's exact
//! identities at round-off, and the chain's gradient against differences within what they
//! resolve.
use super::{Basis::*, Measure::*, Numerical::*, Rule};
pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "topology/projection-derivative",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the complex step takes no difference, so it and the closed form agree to the round-off of a few tanh and cosh: about 1e-15 of the largest derivative, held to 1e-12",
            ),
        ),
        (
            "topology/projection-identities",
            Rule::new(
                Absolute,
                RoundOff,
                1e-14,
                Exact,
                "exact identities of Eq. 9, each a quotient of sums of two tanh: a few ulps of 1, held to 1e-14",
            ),
        ),
        (
            "topology/filter-transpose",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the two sides are the same sum of 10⁴ to 10⁵ products in two orders: their round-off, √n ε ≈ 1e-14, held to 1e-12",
            ),
        ),
        (
            "topology/filter-preserves-linear",
            Rule::new(
                Absolute,
                RoundOff,
                1e-14,
                Exact,
                "averages of at most 300 weights of values below 1: a few ulps, held to 1e-14",
            ),
        ),
        (
            "topology/fdfd-gradient",
            Rule::new(
                Relative,
                Discretisation,
                1e-8,
                Noted,
                "fourth-order differences of the densities, as fdfd/adjoint-gradient-ez: the error they resolve, the powers' precision through their weights, falls as 1/δ (5.6e-12/δ at the corner pixel) and their truncation rises as δ⁴ (1.6 δ⁴ there, ×15.9 from δ = 1.6e-2 to 3.2e-2); the two meet near δ = 5e-3, and at δ = 2e-3 the truncation is 1 % of the resolution, 2.8e-9 at most, which is noted; held to 1e-8, a few times it",
            ),
        ),
    ]
}
