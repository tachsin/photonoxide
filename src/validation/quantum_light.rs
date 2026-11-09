//! The rules (#544) of the quantum light's cases ([`crate::quantum::light_checks`]): the
//! hafnians' and Gaussian states' identities hold to round-off; the squeezers' to the RK4
//! integration's error, its order checked by its own case; the published numbers to half a unit
//! of their last printed digit, or the uncertainty the paper states.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

fn exact(tolerance: f64, why: &'static str) -> Rule {
    Rule::new(Absolute, RoundOff, tolerance, Exact, why)
}

fn printed(tolerance: f64, numerical: super::Numerical, why: &'static str) -> Rule {
    Rule::new(Absolute, Reference, tolerance, numerical, why)
}

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "quantum/hafnian-ones",
            Rule::new(
                Relative,
                RoundOff,
                1e-8,
                Exact,
                "an inclusion–exclusion over 2^12 subsets whose terms reach 10^4 times the result: round-off times that cancellation, under 1e-8",
            ),
        ),
        (
            "quantum/hafnian-definition",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "two sums of the same matchings, one by power traces from a characteristic polynomial: a few hundred units of round-off at 12 rows",
            ),
        ),
        (
            "quantum/hafnian-threads",
            exact(0.0, "the same bits on any number of threads"),
        ),
        (
            "quantum/squeezed-vacuum",
            exact(
                1e-12,
                "hafnians of all-equal matrices against a closed form, and a sum of 16 terms whose tail beyond 30 photons is below 1e-13: round-off",
            ),
        ),
        (
            "quantum/two-mode-thermal",
            exact(
                1e-13,
                "hafnians, a 4 × 4 inverse and determinant, and a torontonian of 4 terms: round-off",
            ),
        ),
        (
            "quantum/squeezed-loss",
            exact(
                1e-13,
                "a 4 × 4 real map of variances up to e^2.4: round-off",
            ),
        ),
        (
            "quantum/displaced-states",
            exact(
                1e-11,
                "loop hafnians up to 30 photons summed into moments of size 10: round-off of the largest terms",
            ),
        ),
        (
            "quantum/clicks-coherent",
            exact(
                1e-13,
                "16 alternating sums of at most 16 vacuum probabilities against products of exponentials: round-off",
            ),
        ),
        (
            "quantum/clicks-displaced",
            exact(
                1e-13,
                "the loop form's four terms against 256 loop hafnians summed, their tail beyond 15 photons a mode below 1e-14 (checked by the case): round-off",
            ),
        ),
        (
            "quantum/source-pairs",
            exact(
                1e-14,
                "a convolution of geometric distributions against a closed form: round-off",
            ),
        ),
        (
            "quantum/source-hom-gaussian",
            exact(
                1e-12,
                "sums of Gaussians sampled 9 widths out at 18 points per width: their error, e^(−40) and below, is under round-off",
            ),
        ),
        (
            "quantum/heralding-coupler",
            exact(1e-14, "ratios of sums of the same terms: round-off"),
        ),
        (
            "quantum/ring-helt-short",
            printed(
                0.005,
                Noted,
                "K printed to two decimals: half a unit of the second; ours from the grid's span, 20 against 10 linewidths",
            ),
        ),
        (
            "quantum/ring-helt-long",
            Rule::new(
                Relative,
                Reference,
                0.005 / 0.74,
                Noted,
                "the linewidth they state, 2π(0.74 GHz), to two figures: half a unit of the second, 0.68 %, which K follows in proportion with a pulse many times the dwelling time; ours from the grid's span, 10 against 5 linewidths",
            ),
        ),
        (
            "quantum/ring-vernon-equal",
            printed(
                0.005,
                Noted,
                "K printed to two decimals: half a unit of the second; ours from the grid's span, 20 against 10 linewidths",
            ),
        ),
        (
            "quantum/squeezer-degenerate",
            Rule::new(
                Absolute,
                Discretisation,
                1e-9,
                Noted,
                "RK4 on 2000 steps of a gain of 10: its error falls as the step's fourth power (quantum/squeezer-rk4-order); ours against 1000 steps, over 2⁴ − 1 (Richardson)",
            ),
        ),
        (
            "quantum/squeezer-four-wave",
            Rule::new(
                Relative,
                Discretisation,
                1e-9,
                Noted,
                "RK4 on 2000 steps: its error falls as the step's fourth power; ours against 1000 steps, over 2⁴ − 1 (Richardson)",
            ),
        ),
        (
            "quantum/squeezer-rk4-order",
            Rule::new(
                Absolute,
                Order,
                0.2,
                Measured,
                "RK4's fourth order within 0.2, shown to two decimals",
            ),
        ),
        (
            "quantum/vaidya-number-difference",
            printed(
                0.5,
                Exact,
                "\"~7 dB\": to the dB, half a unit (the ratio is 1 − η at four gains to round-off, or the case fails)",
            ),
        ),
        (
            "quantum/vaidya-outcoupling",
            printed(
                0.5,
                Exact,
                "\"about 5 dB\" and \"nearly 10 dB\": to the dB, half a unit",
            ),
        ),
        (
            "quantum/nehra-inferred",
            printed(0.87, Exact, "their stated ±0.87 dB"),
        ),
        (
            "quantum/nehra-all-optical",
            printed(0.5, Exact, "\"~11 dB\": to the dB, half a unit"),
        ),
        (
            "quantum/nehra-homodyne-bound",
            printed(
                0.5,
                Exact,
                "their bound, under 1 dB: the interval from 0 to 1 dB, its middle and half its width",
            ),
        ),
        (
            "quantum/broadband-symplectic",
            exact(
                1e-12,
                "40 products of 96 × 96 maps whose entries reach cosh² r ≈ 10³, relative to the largest squared: round-off",
            ),
        ),
        (
            "quantum/broadband-exponential",
            exact(
                1e-12,
                "two singular-value decompositions of the same 48 × 48 spectrum, squeezing up to 2: round-off",
            ),
        ),
        (
            "quantum/broadband-low-gain",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "the relative difference's order in g, 2 within 0.1",
            ),
        ),
        (
            "quantum/broadband-single-mode",
            exact(
                1e-13,
                "ten commuting 1 × 1 maps multiplied, values up to sinh² 2 ≈ 13: round-off",
            ),
        ),
        (
            "quantum/broadband-mismatched",
            Rule::new(
                Relative,
                Discretisation,
                1e-5,
                Noted,
                "the exponential midpoint rule on 1000 slices: its error falls as the slice's square (quantum/broadband-mismatched-order); ours against 500 slices, over 2² − 1 (Richardson)",
            ),
        ),
        (
            "quantum/broadband-mismatched-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "the midpoint rule's second order within 0.1",
            ),
        ),
        (
            "quantum/broadband-photons",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "sums of 48 squares of numbers up to sinh² r ≈ 10³, and a 96-mode Gaussian state's photons: round-off",
            ),
        ),
    ]
}
