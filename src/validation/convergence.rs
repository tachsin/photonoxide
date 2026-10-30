//! The rules (#544) of convergence to a tolerance's cases (#432): a slab's index driven to a
//! stated tolerance against its exact root, the estimate against the true error, the order the
//! recorded grids show, a stepped taper driven in modes and steps against FDFD driven the same
//! way, and the refusal when the budget runs out.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "convergence/slab-tolerance",
            Rule::new(
                Absolute,
                Discretisation,
                1e-5,
                Measured,
                "the tolerance the driver is asked for, 1e-5 on n_eff (its fifth decimal): the case checks that it delivers it",
            ),
        ),
        (
            "convergence/slab-estimate",
            Rule::new(
                Absolute,
                Discretisation,
                1.0,
                Measured,
                "the true error over the estimate: at most 1 when the estimate bounds the error",
            ),
        ),
        (
            "convergence/slab-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "the scheme's second order (its faces on nodes), within 0.1 as eme/taper-steps-order's",
            ),
        ),
        (
            "convergence/eme-taper-fdfd",
            Rule::new(
                Relative,
                Discretisation,
                6e-4,
                Noted,
                "three times the tolerance both are driven to (2e-4 relative), which bounds each one's estimate, so that our own error is at most a third",
            ),
        ),
        (
            "convergence/refusal",
            Rule::new(
                Absolute,
                RoundOff,
                0.5,
                Exact,
                "a count: the refusal's missing parts (no error, the quantity, either of the last two values, the tolerance), none, within half a part",
            ),
        ),
    ]
}
