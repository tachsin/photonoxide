//! The rules (#544) of the real components' cases ([`crate::quantum::real_checks`]): the
//! statistics' and budgets' identities hold to round-off; the published thresholds and tests to
//! half a unit of their last printed digit, or the uncertainty the paper states; the emitters'
//! and detectors' closed forms to their grids' errors, the order of each grid checked by its own
//! case.

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
            "quantum/spectral-flat",
            exact(
                1e-13,
                "permanents of 3 × 3 summed over 6 permutations, against Q1's from the same overlaps: round-off",
            ),
        ),
        (
            "quantum/spectral-hom",
            exact(
                1e-13,
                "the same trapezoidal sums of the same products, grouped two ways: round-off",
            ),
        ),
        (
            "quantum/spectral-sum",
            exact(
                1e-13,
                "S(ω) unitary at every frequency, the sums over patterns and frequencies exchanged: round-off",
            ),
        ),
        (
            "quantum/touchstone-statistics",
            exact(
                1e-14,
                "the file's numbers are the shortest decimals that read back to the same doubles; the frequencies' conversion to wavelengths and back moves the last bits",
            ),
        ),
        (
            "quantum/budget-path",
            Rule::new(
                Relative,
                RoundOff,
                1e-13,
                Exact,
                "a product of four powers against the circuit's solve of the same S-matrices: round-off",
            ),
        ),
        (
            "quantum/varnava-threshold",
            exact(
                1e-14,
                "their 2/3 is exact algebra of their loss model; ours by bisection to the last bit",
            ),
        ),
        (
            "quantum/bartolucci-fusion",
            printed(0.0005, Exact, "\"10.4%\": half a unit of its last digit"),
        ),
        (
            "quantum/qi-simulability",
            printed(
                0.005,
                Exact,
                "\"error 2%\": half a unit of its one digit (the case fails unless Zhong et al.'s ε* is above 1, as their \"no solution for any ε ∈ [0, 1]\")",
            ),
        ),
        (
            "quantum/silverstone-fringe",
            printed(
                0.004,
                Exact,
                "their stated ±0.4%; ours from the guided-mode model on the exact slab modes, no grid",
            ),
        ),
        (
            "quantum/emitter-homogeneous",
            Rule::new(
                Absolute,
                Discretisation,
                0.006,
                Measured,
                "the closed form against our 20 nm grid: its error, second order (0.0045, a quarter of the 40 nm grid's 0.018)",
            ),
        ),
        (
            "quantum/emitter-order",
            Rule::new(
                Absolute,
                Order,
                0.2,
                Measured,
                "Yee's second order within 0.2, shown to two decimals",
            ),
        ),
        (
            "quantum/emitter-mirror",
            Rule::new(
                Absolute,
                Discretisation,
                0.001,
                Measured,
                "the closed form against our 10 nm grid: its error, second order (6.4e-4; quantum/emitter-order)",
            ),
        ),
        (
            "quantum/emitter-slab",
            Rule::new(
                Relative,
                Discretisation,
                0.01,
                Measured,
                "the slab's exact mode against our 10 nm grid's: its error, a third of the 20 nm grid's (0.54%)",
            ),
        ),
        (
            "quantum/detector-stack-joule",
            exact(
                1e-13,
                "the flux between a film's faces against its Joule heat by Simpson's rule on 2000 intervals of the exact field: round-off",
            ),
        ),
        (
            "quantum/detector-film-slab",
            Rule::new(
                Relative,
                Discretisation,
                1e-5,
                Measured,
                "the exact multilayer against our 2 nm nodes: their error, second order (4.7e-6; quantum/detector-film-order)",
            ),
        ),
        (
            "quantum/detector-film-order",
            Rule::new(
                Absolute,
                Order,
                0.2,
                Measured,
                "the scheme's second order within 0.2, shown to two decimals",
            ),
        ),
    ]
}
