//! The rule (#544) of the TFLN Mach–Zehnder modulator's stated error in its analytic bandwidths
//! (`device::tfln_checks`, device/tfln-analytic-bandwidth). The analytic fidelity against the 2D
//! one is a model, as its Vπ·L's and its line's are (`modulator.rs`): the tolerance is the
//! analytic fidelity's stated error in the 3 and 6 dB bandwidths,
//! `TflnModulator::ANALYTIC_BANDWIDTH_ERROR`, with the 2D model's own grid error beside it.
//!
//! The analytic bandwidths are wrong by a factor, not by a few per cent: its conformal-mapping
//! line's RF index runs 8 to 9 % high, which about doubles the walk-off from the light that sets
//! them, so they come out at 0.32 to 0.41 of the 2D model's at 3 dB and 0.23 to 0.30 at 6 dB over
//! the analytic fidelity's validity (2026-10-30). The stated error is that measured spread, 77 %
//! at worst, rounded up to the next 5 % as the analytic Vπ·L's (12 %, stated 15) and line's (9 %,
//! stated 10) are. It isn't tuned to pass, and no factor corrects the analytic bandwidths.

use super::{Basis::*, Measure::*, Numerical, Numerical::*, Rule};
use crate::device::TflnModulator;

/// The 2D model's own grid at Zhang et al.'s design, its 3 dB bandwidth among the figures
/// (device/tfln-2d-grid: 4.8e-3, 2026-10-29).
const GRID: Numerical = Stated(
    4.8e-3,
    "the 2D grids against twice as fine, device/tfln-2d-grid",
);

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![(
        "device/tfln-analytic-bandwidth",
        Rule::new(
            Relative,
            Model,
            TflnModulator::ANALYTIC_BANDWIDTH_ERROR / 100.0,
            GRID,
            "the analytic fidelity's stated error in its bandwidths: the measured spread over its validity, low by a factor of 2.4 to 4.3",
        ),
    )]
}
