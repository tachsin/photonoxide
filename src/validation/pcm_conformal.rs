//! The rules (#544) of the phase-change phase shifter's cases for a film that follows the guide
//! (on its walls and over its slab) and for the TM-like mode, by the case's id. The film's
//! drawing is checked in round-off (the cells of its zero-step limit, its area against a closed
//! form); the 2D model's grid by halving the cells; the effective index method, exact as the
//! guide widens, against the 2D model at a wide guide, as device/pcm-wide-guide does for the
//! film on the top in TE.

use super::{Basis::*, Measure::*, Numerical::*, Rule};

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "device/pcm-conformal-step-limit",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same cells drawn two ways, and the same solve: to the bit",
            ),
        ),
        (
            "device/pcm-conformal-area",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "a sum of the film's cells' areas against the closed form of rectangles whose faces are nodes: round-off",
            ),
        ),
        (
            "device/pcm-conformal-grid",
            Rule::new(
                Relative,
                Discretisation,
                2e-2,
                Measured,
                "as device/pcm-2d-grid: halving the cells moves the device's 2D figures by less than 2 %",
            ),
        ),
        (
            "device/pcm-wide-guide-tm-conformal",
            Rule::new(
                Relative,
                Model,
                1e-2,
                Noted,
                "as device/pcm-wide-guide: the effective index method's 1.2 % in n_eff for the 500 × 220 nm strip, at a guide four times wider",
            ),
        ),
    ]
}
