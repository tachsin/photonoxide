//! The rules (#544) of the densities' cases (0.7, #761): the projection's and the filter's exact
//! identities at round-off, and the chain's gradient against differences within what they
//! resolve; and of robust designs: the filter across an edge and Eq. 29's length at the pixels'
//! own second-order error, measured by `topology::tests::the_robust_study`, and the epigraph's
//! Jacobian within what the differences resolve.
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
            )
            .varying(),
        ),
        (
            "topology/robust-edge-1d",
            Rule::new(
                Absolute,
                RoundOff,
                1e-14,
                Exact,
                "an exact identity of whole-pixel sums: at most 160 weights below 80 over their total, 6400: a few ulps of 1, held to 1e-14",
            ),
        ),
        (
            "topology/robust-edge-2d",
            Rule::new(
                Absolute,
                Discretisation,
                1.0 / (8.0 * 40.0 * 40.0),
                Measured,
                "the cone's integral summed over pixels, second order in Δ/R: 0.105, 0.084, 0.067, 0.056 and 0.062 (Δ/R)² at R/Δ = 5, 10, 20, 40 and 80 (the_robust_study); held to (Δ/R)²/8, the 1D hat's exact pixel offset (topology/robust-edge-1d), 7.8e-5 at R/Δ = 40",
            ),
        ),
        (
            "topology/robust-length",
            Rule::new(
                Absolute,
                Discretisation,
                4e-4,
                Measured,
                "bars and their crossings in whole pixels, second order in Δ/R: 2.9, 1.8, 0.93 and 2.4 (Δ/R)² at R/Δ = 25, 50, 100 and 200 (the_robust_study); held to 4 (Δ/R)², 4e-4 at R/Δ = 100",
            ),
        ),
        (
            "topology/robust-epigraph",
            Rule::new(
                Relative,
                Discretisation,
                3e-7,
                Noted,
                "fourth-order differences of each realization's power: at δ = 1e-3, 2e-3, 4e-3 and 8e-3 every error (3.9e-8, 2.5e-8, 8.1e-9, 1.4e-8) is below their resolution (2.7e-7, 1.4e-7, 6.7e-8, 3.4e-8), coarser than topology/fdfd-gradient's as the realizations' smallest gradients are smaller (the_robust_study); at δ = 4e-3 the resolution, 6.7e-8, is noted; held to 3e-7, above the report's three times it",
            )
            .varying(),
        ),
        (
            "topology/ruler-shapes",
            Rule::new(
                Absolute,
                Discretisation,
                1.0,
                Measured,
                "a disc drawn on round(d/Δ) pixels still fits a feature w pixels wide up to d near w + ½, and the bisection stops within a quarter pixel of the first violation: within a pixel (0.63, 0.34 and 0.19 for the bar, the gap and the ring)",
            ),
        ),
        (
            "topology/ruler-testbed",
            Rule::new(
                Absolute,
                Reference,
                5e-11,
                Exact,
                "the README prints each length to ten decimals of a nanometre: half a unit of the last, 5e-11 nm; the bisection's lengths are dyadic numbers of nanometres, reproduced bit for bit",
            ),
        ),
        (
            "topology/constraint-thresholds",
            Rule::new(
                Absolute,
                RoundOff,
                1e-14,
                Exact,
                "an exact inverse: a square root and its square, a few ulps of 1, held to 1e-14",
            ),
        ),
        (
            "topology/constraint-gradient",
            Rule::new(
                Relative,
                Discretisation,
                3e-9,
                Noted,
                "fourth-order differences of the densities: at δ = 1e-5, 1e-4, 1e-3 and 1e-2 the errors are 5.2e-8, 1.1e-9, 3.9e-11 and 7.7e-8 against a resolution of 7.0e-8, 7.0e-9, 7.0e-10 and 7.0e-11 (relative to the smallest gradient probed, 1.9e-5; the_constraint_study): below it up to 1e-3, the truncation above; at δ = 1e-3 the resolution is noted; held to 3e-9, above the report's three times it",
            ),
        ),
        (
            "topology/constraint-arrieta",
            Rule::new(
                Relative,
                Discretisation,
                100.0 / (200.0 * 200.0),
                Measured,
                "the pixels' sum against the integral, second order in Δ/R: 80, 83, 85 and 85 (Δ/R)² at R/Δ = 25, 50, 100 and 200 (the_arrieta_study; Richardson from the last two gives Eq. 18 to 1.4e-5); held to 100 (Δ/R)², 2.5e-3 at R/Δ = 200",
            ),
        ),
        (
            "topology/helmholtz-residual",
            Rule::new(
                Absolute,
                RoundOff,
                1e-12,
                Exact,
                "a direct solve by two orthogonal transforms of up to 48 terms a sum: their round-off, about 1e-14 of densities below 1, held to 1e-12",
            ),
        ),
        (
            "topology/helmholtz-edge",
            Rule::new(
                Absolute,
                Discretisation,
                0.1 * (1.25 / 14.433_756_729_740_644)
                    * (1.25 / 14.433_756_729_740_644)
                    * 14.433_756_729_740_644,
                Measured,
                "the pixels' second difference and the crossing's interpolation, second order in h/r: 0.026, 0.076, 0.019 and 0.016 (h/r)² r at h = 10, 5, 2.5 and 1.25 nm (the_helmholtz_study; 7.19 nm at the paper's 10 nm, against their ±8 nm); held to 0.1 (h/r)² r, 0.011 nm at 1.25 nm",
            ),
        ),
        (
            "topology/double-filter-gradient",
            Rule::new(
                Relative,
                Discretisation,
                1e-8,
                Measured,
                "fourth-order differences of a function of the densities alone: 1.0e-8, 4.0e-10 and 1.7e-9 at δ = 1e-4, 1e-3 and 1e-2 (the_helmholtz_study), round-off below and truncation above; at δ = 1e-3 held to 1e-8",
            )
            .varying(),
        ),
        (
            "topology/area-shapes",
            Rule::new(
                Absolute,
                RoundOff,
                1e-14,
                Exact,
                "sums of at most 1600 ones over 10⁴, exact in binary but for their last bits: held to 1e-14",
            ),
        ),
        (
            "topology/ssp-plane",
            Rule::new(
                Absolute,
                Discretisation,
                1e-9,
                Measured,
                "Simpson's rule on 20 000 intervals once the arc's square root is taken out: 1.0e-11 (the_ssp_study), the projection of a linear field exact; held to 1e-9",
            ),
        ),
        (
            "topology/ssp-disc",
            Rule::new(
                Absolute,
                Order,
                0.1 * 0.55 * 5.0 / 250.0,
                Measured,
                "the planar fill on a curve, first order in R̂/a: 0.078 R̂/a at 20, 10, 5 and 2.5 nm pixels (the_ssp_study); held to 0.1 R̂/a, 1.1e-3 at 5 nm",
            ),
        ),
        (
            "topology/ssp-gradient",
            Rule::new(
                Relative,
                Discretisation,
                1e-8,
                Measured,
                "fourth-order differences: at β = ∞, 1.5e-10, 1.6e-10 and 1.6e-6 at δ = 1e-5, 1e-4 and 1e-3, and at β = 64, 4.8e-10, 4.3e-11 and 1.1e-9 (the_ssp_study), round-off below and truncation above; at δ = 1e-4 held to 1e-8",
            ),
        ),
        (
            "topology/level-set-plane",
            Rule::new(
                Absolute,
                RoundOff,
                1e-14,
                Exact,
                "an exact identity for a linear φ: the triangles' shares against the clipped cell's area, 1.1e-16 (the_level_set_study); held to 1e-14",
            ),
        ),
        (
            "topology/level-set-circle",
            Rule::new(
                Relative,
                Order,
                0.5 * (5.0 / 500.0) * (5.0 / 500.0),
                Measured,
                "the chords' error, second order in h/R, its sign with how the circle meets the cells: within 0.3 (h/R)² at 40, 20, 10 and 5 nm (the_level_set_study); held to 0.5 (h/R)², 5e-5 at 5 nm",
            ),
        ),
        (
            "topology/level-set-gradient",
            Rule::new(
                Relative,
                Discretisation,
                2e-6,
                Measured,
                "fourth-order differences of the power: at δ = 1e-4 within what they resolve (5.8e-7 at a node of a small derivative, 2.6e-9 at a large one; the_level_set_study), truncation at 1e-3 (1.9e-6); held to 2e-6",
            )
            .varying(),
        ),
        (
            "topology/penalty-gap",
            Rule::new(
                Absolute,
                Order,
                1.0 / 1600.0,
                Measured,
                "the second difference's shortfall, second order in h/d: the switch at D = d (1 − c (h/d)²) with c = 0.87, 0.86, 0.86 and 0.80 at d/h = 10, 20, 40 and 80 (the_penalty_study); held to (h/d)², 6.3e-4 at d/h = 40",
            ),
        ),
        (
            "topology/penalty-curvature",
            Rule::new(
                Absolute,
                Order,
                0.05 / 40.0,
                Measured,
                "the nodes nearest the boundary: 4.7e-3, 5.0e-5 and 7e-6 at d/h = 10, 20 and 40 (the_penalty_study), below 0.05 h/d; held to 0.05 h/d, 1.25e-3 at d/h = 40",
            ),
        ),
        (
            "topology/penalty-gradient",
            Rule::new(
                Relative,
                RoundOff,
                1e-8,
                Measured,
                "the penalty is smooth but at its ramps' and absolute values' kinks, which differences wider than 1e-7 cross at some node (1.8e-3 at 1e-6); at 1e-7, 8.3e-11 (the_penalty_study); held to 1e-8",
            ),
        ),
        (
            "topology/litho-blur",
            Rule::new(
                Absolute,
                RoundOff,
                1e-14,
                Exact,
                "each pixel's weight is the Gaussian's integral over it, so the blur of a pixelated shape is its exact convolution: sums of differences of erfc below 1, 7.8e-16 (the_litho_study); held to 1e-14",
            ),
        ),
        (
            "topology/litho-transpose",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "the two sides are the same sum of 10³ to 10⁴ products in two orders: 1.8e-15 (the_litho_study); held to 1e-12",
            ),
        ),
        (
            "topology/litho-print-edge",
            Rule::new(
                Absolute,
                Discretisation,
                crate::topology::checks::SSP_ROW_ERROR + 0.1 / 16.0,
                Measured,
                "the subpixel-smoothed threshold fills a pixel by F₂ of its centre's distance, not by its area: a row's area is off by up to 0.1217 px across a straight edge (the supremum over the edge's place in a pixel), and the blur's curvature across a pixel adds 0.053, 0.019, 0.0035 and 0.0005 px at σ = 1, 2, 4 and 8 px, second order in Δ/σ (the_litho_study: worst 0.174, 0.141, 0.125 and 0.122 px); held to 0.1217 + 0.1 (Δ/σ)² px, 0.128 px at σ = 4 px",
            ),
        ),
        (
            "topology/litho-gradient",
            Rule::new(
                Relative,
                Discretisation,
                5e-8,
                Noted,
                "fourth-order differences of the mask: at δ = 1e-5, 1e-4 and 1e-3 every error (at most 1.4e-7, 2.2e-8 and 9.2e-10) is below what they resolve (1.5e-6, 1.5e-7 and 1.5e-8), and at 1e-2 the truncation shows (8.3e-5; the_litho_study); at δ = 1e-3 the resolution, 1.5e-8, is noted; held to 5e-8, above three times it",
            )
            .varying(),
        ),
    ]
}
