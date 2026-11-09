//! The source designer's squeezing at high gain: a guide's or a crystal's source cut into slices
//! along its length (`quantum::Broadband::sliced`), each slice coupling the signal's frequencies
//! to the idler's by the pump's amplitude times e^{2ixt}, t its place in the length and x = ΔkL/2
//! the device's own phase mismatch (`Device::phase_at_frequencies`), beside the Schmidt model's
//! single exponential of the whole joint spectrum (`Broadband::exponential`), whose squeezing is
//! g√λ_k on the low-gain Schmidt modes.
//!
//! Where the phase matching changes with the frequencies, the slices don't commute: the squeezing
//! and the squeezed modes then change with the gain, which the Schmidt model leaves out. The view
//! draws both: each pair of modes' squeezing, the signal's mean photon number, the effective
//! number of modes, and the first signal modes' shapes with how far the gain has moved them.
//! The first squeezing's change from half the slices states the slicing's error.
//!
//! A ring and a guide pumped in two modes aren't sliced: a ring's pairs are its resonances',
//! with no phase mismatch along a length, and two pumps' overlap changes along the guide, which
//! `Broadband::sliced` doesn't take. Lossless and undepleted, as `Broadband` is; four-wave
//! mixing's self- and cross-phase modulation of the pump are left out.

use std::f64::consts::LN_10;
use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::nonlinear::pairs::{self, Matching};
use photonoxide::quantum::Broadband;
use serde::{Deserialize, Serialize};

use super::{Design, EDGE, FEWEST_POINTS, Medium, Shape, checked, err, guide_source, positive};

/// The largest gain g: the Schmidt model's first mode is then squeezed by up to g, about 43 dB,
/// with sinh² g ≈ 5500 photons in it, far into the high gain an undepleted pump allows.
pub const MOST_GAIN: f64 = 5.0;
/// The fewest slices: two, so that half of them is still a source.
pub const FEWEST_SLICES: usize = 2;
/// The most slices: each is a decomposition and a product of (signal + idler)² maps, and with
/// half of them for the error, 200 on 101 points take a few seconds.
pub const MOST_SLICES: usize = 200;
/// The most points across each axis: the map is (2 × points)², one product per slice.
pub const MOST_GAIN_POINTS: usize = 101;
/// The most pairs of modes whose squeezing is returned, largest first.
pub const SHOWN_SQUEEZING: usize = 12;
/// The signal modes whose shapes are returned, the first ones.
pub const SHOWN_MODES: usize = 2;
/// Above this share of the first squeezing, the change from half the slices gets a note.
pub const SLICE_NOTE: f64 = 0.01;

/// The gain and the slicing to compute at.
#[derive(Clone, Debug, Deserialize)]
pub struct Gain {
    /// g: the Schmidt model squeezes mode k by g√λ_k, so that Σ r_k² = g².
    pub gain: f64,
    /// N, even: the change from N/2 slices is the slicing's error.
    pub slices: usize,
    /// Points across each axis of the grid, the design's half width.
    pub points: usize,
}

/// One signal mode, sliced and in the Schmidt model.
#[derive(Debug, Serialize)]
pub struct ModeShape {
    /// |u_k|² at each signal frequency (each mode's sum is 1): the sliced source's, then the
    /// Schmidt model's.
    sliced: Vec<f64>,
    schmidt: Vec<f64>,
    /// 1 − |⟨u_k|u_k⁽⁰⁾⟩|²: how far the gain has moved the mode from the Schmidt mode.
    shift: f64,
}

/// A source's squeezing at high gain as the view draws it: each quantity sliced, then in the
/// Schmidt model.
#[derive(Debug, Serialize)]
pub struct Squeezed {
    gain: f64,
    slices: usize,
    points: usize,
    /// r_k, largest first, at most [`SHOWN_SQUEEZING`]: sliced, then the Schmidt model's g√λ_k.
    squeezing: [Vec<f64>; 2],
    /// The same in dB, 10 log₁₀ e^{2r}: the squeezed quadrature's variance below the vacuum's.
    squeezing_db: [Vec<f64>; 2],
    /// The signal's mean photon number, Σ sinh² r_k.
    mean_photons: [f64; 2],
    /// (Σ sinh² r_k)² / Σ sinh⁴ r_k: the Schmidt number at low gain.
    mode_number: [f64; 2],
    /// The signal's wavelengths, nm.
    signal_axis: Vec<f64>,
    /// The first [`SHOWN_MODES`] signal modes.
    modes: Vec<ModeShape>,
    /// r₁ on N slices less r₁ on N/2.
    slice_change: f64,
    /// The largest entry of M Σ M† − Σ over M's largest squared: zero for a map that keeps the
    /// commutators.
    symplectic_error: f64,
    notes: Vec<String>,
    seconds: f64,
}

/// The squeezing at high gain of the source `design` (a guide or a crystal), at `gain`.
#[tauri::command(async)]
pub fn pair_squeezing(design: Design, gain: Gain) -> Result<Squeezed, String> {
    squeezed(&design, &gain)
}

fn db(r: &[f64]) -> Vec<f64> {
    r.iter().map(|r| 20.0 * r / LN_10).collect()
}

pub(crate) fn squeezed(d: &Design, at: &Gain) -> Result<Squeezed, String> {
    let start = Instant::now();
    if !matches!(d.medium, Medium::Guide { .. } | Medium::Crystal { .. }) {
        return Err(
            "squeezing at high gain slices a guide or a crystal along its length: a ring's \
             pairs are its resonances', with no phase mismatch along a length, and two pumps' \
             overlap changes along the guide, which the slices don't take"
                .into(),
        );
    }
    if !(FEWEST_POINTS..=MOST_GAIN_POINTS).contains(&at.points) {
        return Err(format!(
            "squeezing at high gain takes {FEWEST_POINTS} to {MOST_GAIN_POINTS} points across \
             each axis, not {}: its map has (2 × points)² entries, multiplied once per slice",
            at.points
        ));
    }
    if !(FEWEST_SLICES..=MOST_SLICES).contains(&at.slices) || at.slices % 2 == 1 {
        return Err(format!(
            "the slices must be an even number from {FEWEST_SLICES} to {MOST_SLICES}, not {}: \
             half of them give the slicing's error",
            at.slices
        ));
    }
    let gain = positive(at.gain, MOST_GAIN, "the gain g", "")?;
    let design = Design {
        points: at.points,
        ..d.clone()
    };
    let bandwidth = checked(&design)?;
    let mut source = guide_source(&design, bandwidth)?;
    let mut notes = std::mem::take(&mut source.notes);
    if source.matching == Shape::Gaussian {
        notes.push(
            "the slices are the device's own phase matching: they add up to its sinc, not to the \
             Gaussian stand-in the maps above draw"
                .into(),
        );
    }
    if source.four {
        notes.push(
            "four-wave mixing's self- and cross-phase modulation of the pump are left out".into(),
        );
    }
    let spectrum = pairs::joint_spectrum(
        &source.device,
        source.pump,
        Matching::Sinc,
        source.half,
        at.points,
    )
    .map_err(err)?;
    let sliced = |n: usize| {
        Broadband::sliced(
            &spectrum.signal,
            &spectrum.idler,
            |nu| c64::new(source.pump.amplitude(nu), 0.0),
            |s, i| source.mismatch(s, i),
            n,
            gain,
        )
        .map_err(err)
    };
    let full = sliced(at.slices)?;
    let halved = sliced(at.slices / 2)?;
    let single = Broadband::exponential(&spectrum, gain).map_err(err)?;

    let first = |b: &Broadband| b.squeezing().first().copied().unwrap_or(0.0);
    let slice_change = first(&full) - first(&halved);
    if slice_change.abs() > SLICE_NOTE * first(&full) {
        notes.push(format!(
            "the first squeezing moves by {:.2}% from {} to {} slices: add slices",
            100.0 * slice_change / first(&full),
            at.slices / 2,
            at.slices
        ));
    }
    let shown = |b: &Broadband| -> Vec<f64> {
        b.squeezing()
            .iter()
            .copied()
            .take(SHOWN_SQUEEZING)
            .collect()
    };
    let squeezing = [shown(&full), shown(&single)];
    let mut modes = Vec::new();
    for k in 0..SHOWN_MODES {
        let (Some(u), Some(v)) = (full.signal_mode(k), single.signal_mode(k)) else {
            break;
        };
        let overlap: c64 = u.iter().zip(v).map(|(a, b)| a.conj() * b).sum();
        modes.push(ModeShape {
            sliced: u.iter().map(|z| z.norm_sqr()).collect(),
            schmidt: v.iter().map(|z| z.norm_sqr()).collect(),
            shift: 1.0 - overlap.norm_sqr(),
        });
    }
    if let Some(m) = modes.first() {
        let peak = m.sliced.iter().copied().fold(0.0, f64::max);
        // as the maps' note: the amplitude |u|, not |u|², against its peak
        let edge = (m.sliced[0].max(m.sliced[m.sliced.len() - 1]) / peak).sqrt();
        if peak > 0.0 && edge > EDGE {
            notes.push(format!(
                "the first squeezed mode's amplitude is still {:.1}% of its peak at the grid's \
                 edge: widen the grid",
                100.0 * edge
            ));
        }
    }
    Ok(Squeezed {
        gain,
        slices: at.slices,
        points: at.points,
        squeezing_db: [db(&squeezing[0]), db(&squeezing[1])],
        squeezing,
        mean_photons: [full.mean_photons(), single.mean_photons()],
        mode_number: [full.mode_number(), single.mode_number()],
        signal_axis: spectrum.signal.iter().map(|v| 1e3 / v).collect(),
        modes,
        slice_change,
        symplectic_error: full.symplectic_error(),
        notes,
        seconds: start.elapsed().as_secs_f64(),
    })
}

#[cfg(test)]
#[path = "squeezing_tests.rs"]
mod tests;
