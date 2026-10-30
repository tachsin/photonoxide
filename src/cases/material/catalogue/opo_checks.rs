//! The catalogue's lithium niobate in a PPLN optical parametric oscillator: the idler its
//! indices predict, through phase matching ([`crate::nonlinear::phase`]), against the papers'
//! figures. Here, not beside the catalogue, because phase matching is in the layer above it
//! (docs/architecture.md).

use super::models;
use crate::material::catalogue::Axis;
use crate::nonlinear::phase::{Device, Expansion, Grating, Index, Process, State, Wave};
use crate::units::Wavelength;
use crate::validation::Outcome;

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// The idler (µm) of an OPO pumped at `pump` (µm) in a periodically poled crystal whose grating
/// has the period `period` (µm, at the expansion's reference temperature), every wave of the
/// index `index` (extraordinary, d₃₃) at `state`: the first-order quasi-phase-matched
/// difference-frequency process, Δk = k_p − k_s − k_i − 2π/Λ(T) = 0, its idler tuned between
/// degeneracy and `longest` with the pump held ([`Device::phase_matching_wavelength`]).
pub(crate) fn opo_idler(
    index: &Index,
    state: State,
    expansion: Expansion,
    pump: f64,
    period: f64,
    longest: f64,
) -> f64 {
    let run = || -> crate::Result<f64> {
        // any signal to start from: the tuning moves it with the idler
        let (p, s) = (Wavelength::um(pump)?, Wavelength::um(1.5 * pump)?);
        let process = Process::difference_frequency(
            Wave::new(p, index.clone()),
            Wave::new(s, index.clone()),
            index.clone(),
        )?
        // the idler tuned, the signal following, the pump held
        .with_tuning(vec![-1.0, 1.0, 0.0], 1)?;
        let device = Device::new(process, 1.0, state)?
            .with_expansion(expansion)
            .with_grating(Grating::new(period, 1, 0.5)?);
        device.phase_matching_wavelength(2.0 * pump + 1e-9, longest)
    };
    run().unwrap_or(f64::NAN)
}

/// Zelmon et al.'s Fig. 3: the predicted idler of Myers et al.'s 1.064 µm-pumped PPLN OPO at a
/// 30 µm grating, read off the plot as 3.35 µm (±0.03).
pub(crate) fn zelmon_opo() -> Outcome {
    let (e, _) = models::ZELMON_CONGRUENT;
    let idler = models::zelmon("LiNbO3 (e)", e, "Table 1").map_or(f64::NAN, |m| {
        opo_idler(
            &Index::material(m),
            State::default(),
            Expansion::NONE,
            1.064,
            30.0,
            5.0,
        )
    });
    outcome(idler, 3.35, 0.03)
}

/// Jundt's Fig. 1: the idler of a 1.064 µm-pumped PPLN OPO at 250 °C and a 25.5 µm grating
/// (at 25 °C, expanded by Jundt's Eq. (3), [`Expansion::LINBO3_JUNDT`]), read off the plot as
/// 4.755 µm (±0.02).
pub(crate) fn jundt_opo() -> Outcome {
    let idler = Index::catalogue("linbo3", Some("jundt-1997"), Axis::Extraordinary).map_or(
        f64::NAN,
        |index| {
            opo_idler(
                &index,
                State::celsius(250.0),
                Expansion::LINBO3_JUNDT,
                1.064,
                25.5,
                5.0,
            )
        },
    );
    outcome(idler, 4.755, 0.02)
}
