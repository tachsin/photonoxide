//! Photon pairs by spontaneous parametric down-conversion and spontaneous four-wave mixing:
//! the joint spectral amplitude of a signal and an idler, and its Schmidt decomposition (the
//! pairs' spectral purity).
//!
//! # Conventions
//!
//! Those of [`super::phase`], and:
//!
//! - **Frequencies** are ν = 1/λ in 1/µm (vacuum), so energy conservation is ν_p = ν_s + ν_i.
//! - **The joint spectral amplitude** is f(ν_s, ν_i) = α(ν_s + ν_i) Φ(ν_s, ν_i) (Grice and
//!   Walmsley 1997, Eqs. (7)–(9)): the pump's spectral amplitude α, a Gaussian whose intensity
//!   |α|² has the FWHM Δν, times the phase matching of a device of length L, sinc(ΔkL/2) as
//!   [`super::phase`] has it (the integral of e^{−iΔkz} over the length), or its Gaussian
//!   stand-in exp(−γ(ΔkL/2)²) with γ = ln 2 / (2·1.39156²) = 0.17897, whose square has the
//!   sinc²'s FWHM. A Gaussian pump and Gaussian phase matching make f a two-dimensional
//!   Gaussian, whose purity is known in closed form ([`gaussian_purity`]).
//! - **The Schmidt decomposition** is the singular value decomposition of f sampled on a grid,
//!   times the cell: f = Σ_k √λ_k u_k(ν_s) v_k(ν_i), Σλ_k = 1. The purity is P = Σλ_k², the
//!   Schmidt number K = 1/P; a separable state has P = 1.
//! - **Filters** multiply the amplitude by their amplitude transmissions t_s(ν_s) t_i(ν_i), from
//!   any component's S-matrix ([`crate::circuit::transmission`]); the **heralding efficiency** of each photon is
//!   the share of its partner's counts it comes with ([`Heralding`]): its spectral part, which
//!   the detectors' efficiencies and the paths' losses multiply.
//!
//! W. P. Grice, I. A. Walmsley, Phys. Rev. A 56, 1627 (1997), doi:10.1103/PhysRevA.56.1627.

use num_complex::Complex64 as c64;

use super::phase::{Device, HALF_MAXIMUM};
use crate::{Error, Result};

/// The γ of the Gaussian stand-in for sinc(x), exp(−γx²): its square has the FWHM of sinc²(x),
/// ln 2 / (2·1.39156²).
pub fn gaussian_gamma() -> f64 {
    std::f64::consts::LN_2 / (2.0 * HALF_MAXIMUM * HALF_MAXIMUM)
}

/// The phase-matching function of the joint spectrum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Matching {
    /// sinc(ΔkL/2), the device's own.
    Sinc,
    /// exp(−γ(ΔkL/2)²), γ = [`gaussian_gamma`].
    Gaussian,
}

/// A pump's spectrum: its centre ν₀ (1/µm) and the FWHM Δν of its intensity (1/µm).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pump {
    /// ν₀, 1/µm.
    pub center: f64,
    /// Δν, the FWHM of |α|², 1/µm.
    pub bandwidth: f64,
}

impl Pump {
    /// α(ν) = exp(−2 ln 2 (ν − ν₀)²/Δν²): |α|² is ½ at ν₀ ± Δν/2.
    pub fn amplitude(&self, nu: f64) -> f64 {
        let d = (nu - self.center) / self.bandwidth;
        (-2.0 * std::f64::consts::LN_2 * d * d).exp()
    }
}

/// A joint spectral amplitude on a grid.
#[derive(Clone, Debug)]
pub struct JointSpectrum {
    /// The signal's frequencies, 1/µm.
    pub signal: Vec<f64>,
    /// The idler's frequencies, 1/µm.
    pub idler: Vec<f64>,
    /// f at the i-th signal and j-th idler frequency, at index i × (idler's length) + j.
    pub amplitude: Vec<c64>,
}

impl Pump {
    /// The spectrum of two photons drawn from this pump, as spontaneous four-wave mixing takes
    /// them: the pump's amplitude convolved with itself, α ⊗ α, a Gaussian centred on 2ν₀ whose
    /// intensity has the FWHM √2 Δν (the convolution doubles the amplitude's variance).
    pub fn two_photon(&self) -> Pump {
        Pump {
            center: 2.0 * self.center,
            bandwidth: std::f64::consts::SQRT_2 * self.bandwidth,
        }
    }
}

/// The joint spectral amplitude of `device`, pumped by `pump`: on an `n` × `n` grid of signal
/// and idler frequencies `half` (1/µm) either side of the device's own, at the device's
/// operating point. The device is either
///
/// - a difference-frequency process, spontaneous down-conversion, whose waves are (signal,
///   idler, pump) as [`super::phase::Process::difference_frequency`] orders them, `pump` the
///   pump's spectrum; or
/// - a four-wave process with one pump, spontaneous four-wave mixing 2ω_p = ω_s + ω_i, whose
///   waves are (pump, pump, signal, idler) as [`super::phase::Process::four_wave`] orders them,
///   both pumps at one wavelength. Each pump photon is taken at (ν_s + ν_i)/2 in the phase
///   matching, and `pump` is then the two photons' spectrum, [`Pump::two_photon`] of the
///   pump's.
///
/// # Errors
///
/// [`Error::InvalidValue`] for any other process, fewer than two points, a half width that
/// isn't positive, and the errors of the waves' indices.
pub fn joint_spectrum(
    device: &Device,
    pump: Pump,
    matching: Matching,
    half: f64,
    n: usize,
) -> Result<JointSpectrum> {
    let waves = device.process().waves();
    let four = waves.len() == 4;
    let degenerate = four && waves[0].wavelength.to_um() == waves[1].wavelength.to_um();
    if !(waves.len() == 3 || degenerate) {
        return Err(Error::invalid(
            "joint spectrum",
            "needs a three-wave process (signal, idler, pump) or a four-wave one with one pump (pump, pump, signal, idler)",
        ));
    }
    let (si, ii) = if four { (2, 3) } else { (0, 1) };
    if n < 2 || !(half.is_finite() && half > 0.0) {
        return Err(Error::invalid(
            "joint spectrum",
            "needs two or more points and a positive half width",
        ));
    }
    let grid = |nu0: f64| -> Vec<f64> {
        (0..n)
            .map(|k| nu0 - half + 2.0 * half * k as f64 / (n - 1) as f64)
            .collect()
    };
    let signal = grid(1.0 / waves[si].wavelength.to_um());
    let idler = grid(1.0 / waves[ii].wavelength.to_um());
    let state = *device.state();
    let gamma = gaussian_gamma();
    let mut amplitude = Vec::with_capacity(n * n);
    for &s in &signal {
        for &i in &idler {
            let x = if four {
                let p = 0.5 * (s + i);
                device.phase_at_frequencies(&[p, p, s, i], &state)?
            } else {
                device.phase_at_frequencies(&[s, i, s + i], &state)?
            };
            let m = match matching {
                Matching::Sinc => {
                    if x.abs() < 1e-4 {
                        1.0 - x * x / 6.0
                    } else {
                        x.sin() / x
                    }
                }
                Matching::Gaussian => (-gamma * x * x).exp(),
            };
            amplitude.push(c64::new(pump.amplitude(s + i) * m, 0.0));
        }
    }
    Ok(JointSpectrum {
        signal,
        idler,
        amplitude,
    })
}

/// A joint spectrum's Schmidt decomposition: its weights.
#[derive(Clone, Debug, PartialEq)]
pub struct Schmidt {
    /// The weights λ_k, largest first, summing to 1.
    pub weights: Vec<f64>,
}

impl Schmidt {
    /// The purity P = Σλ_k² of either photon's reduced state.
    pub fn purity(&self) -> f64 {
        self.weights.iter().map(|l| l * l).sum()
    }

    /// The Schmidt number K = 1/P: the effective number of modes.
    pub fn number(&self) -> f64 {
        1.0 / self.purity()
    }
}

impl JointSpectrum {
    /// The Schmidt decomposition of the sampled amplitude, by its singular values.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when the decomposition fails (non-finite samples) or the
    /// amplitude is zero.
    pub fn schmidt(&self) -> Result<Schmidt> {
        let (ns, ni) = (self.signal.len(), self.idler.len());
        let m = faer::Mat::<c64>::from_fn(ns, ni, |i, j| self.amplitude[i * ni + j]);
        let mut s = m
            .singular_values()
            .map_err(|e| Error::invalid("joint spectrum", format!("its singular values: {e:?}")))?;
        let total: f64 = s.iter().map(|v| v * v).sum();
        if total <= 0.0 || total.is_nan() {
            return Err(Error::invalid("joint spectrum", "the amplitude is zero"));
        }
        s.sort_by(|a, b| b.total_cmp(a));
        Ok(Schmidt {
            weights: s.iter().map(|v| v * v / total).collect(),
        })
    }
}

/// The purity of a two-dimensional Gaussian amplitude f = exp(−½ zᵀMz), M = [[a, c], [c, b]]:
/// P = √(1 − c²/(ab)), from Tr ρ² of the reduced state in closed form.
pub fn gaussian_purity(a: f64, b: f64, c: f64) -> f64 {
    (1.0 - c * c / (a * b)).sqrt()
}

// ---------------------------------------------------------------------------------------------
// Filters and heralding
// ---------------------------------------------------------------------------------------------

/// A pair source's heralding efficiencies through its filters: of the pairs whose idler passed
/// its filter, the share whose signal passed its own, and the other way round,
///
/// η_s = Σ|f|²|t_s|²|t_i|² / Σ|f|²|t_i|², η_i = Σ|f|²|t_s|²|t_i|² / Σ|f|²|t_s|²,
///
/// the efficiency's spectral part (a detector's efficiency and the paths' loss multiply it), as
/// the photons' joint spectrum sets it: a filter narrower than the photon's own marginal
/// spectrum cuts pairs whose partner was counted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Heralding {
    /// η_s: the signal's, heralded by the idler.
    pub signal: f64,
    /// η_i: the idler's, heralded by the signal.
    pub idler: f64,
}

impl JointSpectrum {
    /// Refuses filters that aren't one finite amplitude per frequency of the grid.
    fn check_filters(&self, signal: &[c64], idler: &[c64]) -> Result<()> {
        let finite = |t: &[c64]| t.iter().all(|v| v.re.is_finite() && v.im.is_finite());
        if signal.len() != self.signal.len()
            || idler.len() != self.idler.len()
            || !finite(signal)
            || !finite(idler)
        {
            return Err(Error::invalid(
                "joint spectrum",
                format!(
                    "a filter takes one finite amplitude per frequency: {} for the signal and {} for the idler, got {} and {}",
                    self.signal.len(),
                    self.idler.len(),
                    signal.len(),
                    idler.len()
                ),
            ));
        }
        Ok(())
    }

    /// The amplitude after filters: f(ν_s, ν_i) t_s(ν_s) t_i(ν_i), `signal` and `idler` the
    /// filters' amplitude transmissions at the grid's frequencies ([`JointSpectrum::signal`],
    /// [`JointSpectrum::idler`]), e.g. from a component's S-matrix by [`crate::circuit::transmission`]. Its
    /// [`JointSpectrum::schmidt`] is the filtered pairs' purity.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless each filter has one finite amplitude per frequency.
    pub fn filtered(&self, signal: &[c64], idler: &[c64]) -> Result<JointSpectrum> {
        self.check_filters(signal, idler)?;
        let ni = self.idler.len();
        Ok(JointSpectrum {
            signal: self.signal.clone(),
            idler: self.idler.clone(),
            amplitude: self
                .amplitude
                .iter()
                .enumerate()
                .map(|(k, f)| f * signal[k / ni] * idler[k % ni])
                .collect(),
        })
    }

    /// The heralding efficiencies through filters of amplitude transmissions `signal` and
    /// `idler` at the grid's frequencies ([`Heralding`]).
    ///
    /// # Errors
    ///
    /// As [`JointSpectrum::filtered`], and [`Error::InvalidValue`] when no pair passes a filter.
    pub fn heralding(&self, signal: &[c64], idler: &[c64]) -> Result<Heralding> {
        self.check_filters(signal, idler)?;
        let ni = self.idler.len();
        let (mut both, mut by_idler, mut by_signal) = (0.0, 0.0, 0.0);
        for (k, f) in self.amplitude.iter().enumerate() {
            let p = f.norm_sqr();
            let (ts, ti) = (signal[k / ni].norm_sqr(), idler[k % ni].norm_sqr());
            both += p * ts * ti;
            by_idler += p * ti;
            by_signal += p * ts;
        }
        if !(by_idler > 0.0 && by_signal > 0.0) {
            return Err(Error::invalid(
                "joint spectrum",
                "no pair passes one of the filters",
            ));
        }
        Ok(Heralding {
            signal: both / by_idler,
            idler: both / by_signal,
        })
    }
}

/// The heralding efficiencies of a two-dimensional Gaussian |f|² = exp(−zᵀMz),
/// M = [[a, c], [c, b]] (z the detunings from the filters' centres), through Gaussian filters
/// |t_s|² = exp(−ν_s²/w_s²) and |t_i|² = exp(−ν_i²/w_i²): each sum a Gaussian integral,
/// π/√det, so η_s = √(det(M + D_i)/det(M + D_s + D_i)), D_s = diag(1/w_s², 0),
/// D_i = diag(0, 1/w_i²), and η_i likewise.
pub fn gaussian_heralding(a: f64, b: f64, c: f64, widths: [f64; 2]) -> Heralding {
    let (ds, di) = (1.0 / (widths[0] * widths[0]), 1.0 / (widths[1] * widths[1]));
    let det = |x: f64, y: f64| x * y - c * c;
    let both = det(a + ds, b + di);
    Heralding {
        signal: (det(a, b + di) / both).sqrt(),
        idler: (det(a + ds, b) / both).sqrt(),
    }
}

#[cfg(test)]
#[path = "pairs_tests.rs"]
mod tests;
