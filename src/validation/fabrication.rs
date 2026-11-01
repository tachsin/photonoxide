//! The rules (#544) of the fabrication engine's cases (#622): the round trip of the three first
//! designs, exact by construction; every capability's source; Deal and Grove's printed tables,
//! to their own rounding; the rules' choices on cases with textbook or published answers,
//! which are categorical; and the Smart Cut start: the proton ranges against Bruel's ∼8 nm/keV,
//! to its printed digit, and its stacks and round trip, exact by construction.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    let exact = |why: &'static str| Rule::new(Absolute, RoundOff, 0.0, Exact, why);
    vec![
        (
            "fabrication/round-trip-waveguide",
            exact(
                "an exact identity: the executed flow draws the design's own shapes and etches, so the structures are equal to the bit and no point may differ",
            ),
        ),
        (
            "fabrication/round-trip-tfln",
            exact(
                "an exact identity: the executed flow draws the design's own shapes and etches, so the structures are equal to the bit and no point may differ",
            ),
        ),
        (
            "fabrication/round-trip-heater",
            exact(
                "an exact identity: the executed flow draws the design's own shapes and etches, so the structures are equal to the bit and no point may differ",
            ),
        ),
        (
            "fabrication/capabilities-cited",
            exact("a count: every source must resolve, so no fault is allowed"),
        ),
        (
            "fabrication/deal-grove-tables",
            Rule::new(
                Absolute,
                Reference,
                1.0,
                Exact,
                "half a unit of the last printed digit of A, B and B/A, carried through B/A: a misfit over that allowance of at most 1",
            ),
        ),
        (
            "fabrication/etch-stop-or-timed",
            exact("a choice is categorical: no wrong answer is allowed"),
        ),
        (
            "fabrication/lithography-choice",
            exact("a choice is categorical: no wrong answer is allowed"),
        ),
        (
            "fabrication/thermal-order",
            exact("a choice is categorical: no wrong answer is allowed"),
        ),
        (
            "fabrication/reported-flows",
            exact("a sequence is contained or not: no reported sequence may be missing"),
        ),
        (
            "fabrication/smart-cut-range-bruel",
            Rule::new(
                Relative,
                Reference,
                0.0625,
                Exact,
                "half a unit of the last printed digit of Bruel's ∼8 nm/keV, 0.5 over 8; the ranges are PSTAR's printed values, read on its printed rows",
            ),
        ),
        (
            "fabrication/smart-cut-stack",
            exact(
                "an exact identity: the route's steps take off, to the bit, what the split film has beyond the design's, so no pair may differ",
            ),
        ),
        (
            "fabrication/smart-cut-round-trip",
            exact(
                "an exact identity: the executed flow draws the design's own shapes and etches, so the structures are equal to the bit and no point may differ",
            ),
        ),
    ]
}
