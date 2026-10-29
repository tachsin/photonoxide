//! The rules (#544) of the phase-change phase shifter devices' cases, by the case's id. The
//! analytic fidelity against the 2D one is a model: the stated error of the effective index
//! method, with the 2D model's own grid error beside it; the film's mixture and the figure of
//! merit are published formulas evaluated in round-off; the jobs equal the components to the
//! bit.

use super::{Basis::*, Measure::*, Numerical, Numerical::*, Rule};

/// The 2D model's own error in these figures: its cells against cells half the size, at the
/// defaults and with the 10 nm film, the coarser one's difference from the finer (the finer one's
/// error is a third of it at second order); device/pcm-2d-grid found 1.2 % at most.
const GRID: Numerical = Stated(
    1.2e-2,
    "the 2D cells against half the size, device/pcm-2d-grid (the 10 nm film the worst)",
);

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "device/pcm-analytic-index",
            Rule::new(
                Relative,
                Model,
                0.15,
                GRID,
                "the analytic fidelity's stated error: the field a product of a vertical and a lateral profile",
            ),
        ),
        (
            "device/pcm-analytic-loss",
            Rule::new(Relative, Model, 0.15, GRID, "as device/pcm-analytic-index"),
        ),
        (
            "device/pcm-wide-guide",
            Rule::new(
                Relative,
                Model,
                1e-2,
                Noted,
                "the effective index method's 1.2 % in n_eff for the 500 × 220 nm strip, at a guide four times wider",
            ),
        ),
        (
            "device/pcm-2d-grid",
            Rule::new(
                Relative,
                Discretisation,
                2e-2,
                Measured,
                "the mode solver's second order: halving the cells moves the index change and the loss under 2 % (1.2 % for a 10 nm film, 0.5 % for 25 nm)",
            ),
        ),
        (
            "device/pcm-frantz-mix",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "an exact mixture of the two permittivities: round-off",
            ),
        ),
        (
            "device/pcm-fom",
            Rule::new(
                Relative,
                RoundOff,
                1e-12,
                Exact,
                "Delaney et al.'s Eq. (1) evaluated by the device and by the catalogue: round-off",
            ),
        ),
        (
            "device/pcm-jobs",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same computation, to the bit",
            ),
        ),
    ]
}
