//! The rules (#544) of the fabrication engine's further platforms (F6 of #622): the round trips,
//! exact by construction; the rules' choices and the papers' reported flows, categorical.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    let exact = |why: &'static str| Rule::new(Absolute, RoundOff, 0.0, Exact, why);
    let round_trip = || {
        exact(
            "an exact identity: the executed flow draws the design's own shapes and etches, so the structures are equal to the bit and no point may differ",
        )
    };
    vec![
        ("fabrication/nitride-round-trip", round_trip()),
        (
            "fabrication/nitride-rules",
            exact("a choice is categorical: no wrong answer is allowed"),
        ),
        (
            "fabrication/nitride-reported-flows",
            exact("a sequence is contained or not: no reported step may be missing"),
        ),
        ("fabrication/pcm-round-trip", round_trip()),
        (
            "fabrication/pcm-rules",
            exact("a choice is categorical: no wrong answer is allowed"),
        ),
        (
            "fabrication/pcm-reported-flows",
            exact("a sequence is contained or not: no reported step may be missing"),
        ),
        ("fabrication/aln-round-trip", round_trip()),
        ("fabrication/gaas-round-trip", round_trip()),
        (
            "fabrication/gaas-rules",
            exact("a choice is categorical: no wrong answer is allowed"),
        ),
        (
            "fabrication/aln-reported-flows",
            exact("a sequence is contained or not: no reported step may be missing"),
        ),
    ]
}
