//! The rules (#544) of the adaptive sweeps' cases (#432): AAA on a known rational function at
//! round-off, the error estimate against the true error over many test functions, and two
//! sweeps (a slab's exact dispersion, and a strip's mode through a crossing) against what they
//! sample, to the tolerance they were asked for.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "sweep/aaa-rational",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "a function of type (3, 4) is AAA's type (4, 4) exactly: what is left is round-off, AAA stopping at 1e-13 on its samples and the poles from a well-separated 6 × 6 pencil",
            )
            .varying(),
        ),
        (
            "sweep/estimate-bounds-error",
            Rule::new(
                Absolute,
                Discretisation,
                1.0,
                Measured,
                "the estimate's claim: the sweep's true error at most its estimate, a ratio of at most 1; the case measures that ratio itself",
            )
            .varying(),
        ),
        (
            "sweep/slab-dispersion",
            Rule::new(
                Relative,
                Discretisation,
                1e-10,
                Noted,
                "the sampling is the only error (the exact roots are found to the last bit): the tolerance asked of a sweep, 1e-10, the sweep run at a third of it so that its own estimate, noted, is under a third",
            )
            .varying(),
        ),
        (
            "sweep/mode-crossing",
            Rule::new(
                Relative,
                Discretisation,
                1e-10,
                Noted,
                "as sweep/slab-dispersion: the reference is the same solver on the same grid, so the sampling and the branch followed are the only errors; the sweep's estimate is noted",
            )
            .varying(),
        ),
    ]
}
