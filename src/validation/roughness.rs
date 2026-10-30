//! The rules (#544) of sidewall-roughness loss's cases (#432): Payne and Lacey's slab and their
//! Eq. 9 against the volume current on the exact slab mode, at round-off, and the 3D analysis's
//! convergence on the grid against the corners' singularity.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "roughness/payne-lacey-eq9",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "the exact slab's roots to the last bit and its field's integrals by Gauss–Legendre sums on smooth exponentials and cosines, against a closed form: round-off",
            ),
        ),
        (
            "roughness/payne-lacey-slab",
            Rule::new(
                Relative,
                RoundOff,
                1e-10,
                Exact,
                "the same integrals, the far field's over the angle and the mode's power, by Gauss–Legendre sums on smooth integrands, against Eqs. 8–12 in closed form: round-off",
            ),
        ),
        (
            "roughness/silicon-grid-order",
            Rule::new(
                Absolute,
                Order,
                0.1,
                Measured,
                "the corners' 2τ − 1 = 0.54, within 0.1: the observed order falls from 0.66 (20, 10 and 5 nm) to 0.61 (10, 5 and 2.5 nm) towards it",
            ),
        ),
    ]
}
