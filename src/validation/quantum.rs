//! The rules (#544) of the linear-optical statistics' cases ([`crate::quantum::checks`]). The
//! identities hold to round-off; the HOM dips' trapezoidal sums of Gaussians sampled to ±8σ are
//! exact to round-off too (their truncation is e^(−32)); Clements et al.'s Fig. 3 is read off a
//! raster figure, so the reading and the two means' sampling set those tolerances; the orders
//! as the device's.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

fn exact(tolerance: f64, why: &'static str) -> Rule {
    Rule::new(Absolute, RoundOff, tolerance, Exact, why)
}

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "quantum/permanent-ones",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "sums of integers below 2⁵³ in Gray-code order: exact to the last bit up to n = 14",
            ),
        ),
        (
            "quantum/permanent-derangements",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "sums of integers below 2⁵³, as above",
            ),
        ),
        (
            "quantum/permanent-formulas",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "three sums of the same terms in different orders, with Kahan's compensation: a few units of round-off (1e-15 at n = 8)",
            ),
        ),
        (
            "quantum/permanent-threads",
            exact(0.0, "the same bits on any number of threads"),
        ),
        (
            "quantum/hom-visibility",
            exact(1e-14, "a 2 × 2 permanent: a few units of round-off"),
        ),
        (
            "quantum/hom-balanced",
            exact(1e-15, "t² + (iκ)² = 1/2 − 1/2: round-off of √(1/2) squared"),
        ),
        (
            "quantum/patterns-sum",
            exact(
                1e-12,
                "a sum of 126 probabilities, each from a permanent: round-off",
            ),
        ),
        (
            "quantum/distinguishable-classical",
            exact(1e-14, "sums of products of |U|²: round-off"),
        ),
        (
            "quantum/partly-distinguishable",
            exact(
                1e-13,
                "two sums of the same terms, 48 and 6 permanents of six-mode matrices: round-off",
            ),
        ),
        (
            "quantum/full-bunching",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "a 6 × 6 permanent of equal rows against a product: round-off",
            ),
        ),
        (
            "quantum/dilation",
            exact(
                1e-13,
                "Jacobi's eigenvectors are orthonormal to round-off (about 1e-15 per entry)",
            ),
        ),
        (
            "quantum/hom-dip-gaussian",
            exact(
                1e-12,
                "the trapezoidal rule on a Gaussian sampled 25 times per σ over ±8σ: its error is e^(−32), below round-off",
            ),
        ),
        (
            "quantum/hom-delay-line",
            exact(1e-12, "as above, ±8σ on 601 wavelengths"),
        ),
        (
            "quantum/mesh-reconstruction",
            exact(
                1e-12,
                "66 2 × 2 rotations of a 12 × 12 unitary and back: round-off (about 1e-14)",
            ),
        ),
        ("quantum/mesh-depth", exact(0.0, "counts of cells, exact")),
        (
            "quantum/clements-fig3-size",
            Rule::new(
                Absolute,
                Reference,
                1e-3,
                Noted,
                "the figure read off its raster, each triangle at its centroid: one pixel (0.00034), and three standard errors of the difference of two means over 500 unitaries (3√2 × 0.00015 at N = 50), 0.00098 rounded up",
            ),
        ),
        (
            "quantum/clements-fig3-loss",
            Rule::new(
                Absolute,
                Reference,
                4e-3,
                Noted,
                "the figure read off its raster, each marker at its centroid: one pixel (0.00091; the circles at the lowest loss read 1.0007 for a fidelity of 1), and three standard errors of the difference of two means over 500 unitaries (3√2 × 0.00073 for Reck's at 1 dB), 0.0040",
            ),
        ),
        (
            "quantum/peruzzo-correlations",
            exact(
                1e-15,
                "a 2 × 2 permanent against the same products written out",
            ),
        ),
        (
            "quantum/peruzzo-classical-bound",
            Rule::new(
                Absolute,
                Reference,
                0.0,
                Exact,
                "the paper's statement: no pair violates the bound with distinguishable photons",
            ),
        ),
        (
            "quantum/sampler-convergence",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "the square-root law within 0.1, shown to two decimals: eight seeds at each count keep the slope's own scatter to a few hundredths",
            ),
        ),
        (
            "quantum/coupler-hom-order",
            Rule::new(
                Absolute,
                Order,
                0.5,
                Measured,
                "the standard scheme's first order at convex corners, as device/coupler-3d-order, shown to two decimals",
            ),
        ),
    ]
}
