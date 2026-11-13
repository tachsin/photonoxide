//! Photon-pair sources as quantum states: from a joint spectral amplitude (0.6.1's
//! [`crate::nonlinear::pairs`]) to the squeezing of its Schmidt modes, the number of pairs, the
//! heralding through a circuit, and the interference of photons from two sources.
//!
//! # Conventions
//!
//! - **The state** of a spontaneous source whose pump doesn't deplete is a two-mode squeezed
//!   vacuum in each pair of Schmidt modes (W. P. Grice, I. A. Walmsley, Phys. Rev. A 56, 1627
//!   (1997), doi:10.1103/PhysRevA.56.1627; L. G. Helt et al., Opt. Lett. 35, 3006 (2010),
//!   doi:10.1364/OL.35.003006): the joint spectrum's Schmidt decomposition f = Σ_k √λ_k
//!   u_k(ν_s) v_k(ν_i), Σλ_k = 1, and a gain g, give the squeezing r_k = g √λ_k of the pair
//!   u_k, v_k. Valid where the gain doesn't reshape the modes themselves (low gain: g small, or
//!   one mode).
//! - **Pairs:** mode k holds n pairs with probability (1 − t_k) t_k^n, t_k = tanh² r_k, so the
//!   number of pairs is the sum of independent geometric numbers, its mean Σ sinh² r_k. The
//!   gain of a source is set by its mean number of pairs per pulse, μ.
//! - **Unheralded g⁽²⁾** of either arm, 1 + Σ n̄_k² / (Σ n̄_k)², n̄_k = sinh² r_k: 2 for one mode,
//!   1 + P at low gain, P the purity: each mode a thermal state, whose g⁽²⁾ is 2.
//! - **Two sources' heralded photons** interfere on a balanced coupler with the visibility
//!   Tr(ρ_A ρ_B), ρ the heralded signal's spectral state, the idler traced out: the purity P for
//!   two identical sources (at low gain, one pair).

use num_complex::Complex64 as c64;

use super::Gaussian;
use crate::circuit::{Component, transmission};
use crate::nonlinear::pairs::{Heralding, JointSpectrum};
use crate::portable;
use crate::units::Wavelength;
use crate::{Error, Result};

fn invalid(what: impl Into<String>) -> Error {
    Error::invalid("pair source", what)
}

/// A pair source's state: the squeezing of each pair of Schmidt modes.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    weights: Vec<f64>,
    squeezing: Vec<f64>,
}

impl Source {
    /// The source of Schmidt weights `weights` (each ≥ 0, normalized here) with `mean_pairs`
    /// pairs per pulse: the gain g found by bisection so that Σ sinh²(g √λ_k) = μ. Weights below
    /// 1e-14 of the largest are left out.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no positive weight, a weight that isn't finite and ≥ 0, or a
    /// mean that isn't finite and > 0.
    pub fn new(weights: &[f64], mean_pairs: f64) -> Result<Source> {
        if weights.iter().any(|w| !(w.is_finite() && *w >= 0.0)) {
            return Err(invalid("Schmidt weights must be finite and ≥ 0"));
        }
        if !(mean_pairs.is_finite() && mean_pairs > 0.0) {
            return Err(invalid(format!(
                "the mean number of pairs must be finite and positive, got {mean_pairs}"
            )));
        }
        let largest = weights.iter().copied().fold(0.0, f64::max);
        if largest <= 0.0 {
            return Err(invalid("no Schmidt weight is positive"));
        }
        let kept: Vec<f64> = weights
            .iter()
            .copied()
            .filter(|&w| w > 1e-14 * largest)
            .collect();
        let total: f64 = kept.iter().sum();
        let weights: Vec<f64> = kept.iter().map(|w| w / total).collect();
        let mean = |g: f64| -> f64 {
            weights
                .iter()
                .map(|l| portable::powi(portable::sinh(g * l.sqrt()), 2))
                .sum()
        };
        let (mut low, mut high) = (0.0, 1.0);
        while mean(high) < mean_pairs {
            high *= 2.0;
        }
        for _ in 0..200 {
            let mid = 0.5 * (low + high);
            if mean(mid) < mean_pairs {
                low = mid;
            } else {
                high = mid;
            }
        }
        let g = 0.5 * (low + high);
        Ok(Source {
            squeezing: weights.iter().map(|l| g * l.sqrt()).collect(),
            weights,
        })
    }

    /// The source of a joint spectrum ([`JointSpectrum::schmidt`]) at `mean_pairs` pairs per
    /// pulse.
    ///
    /// # Errors
    ///
    /// The decomposition's, and as [`Source::new`].
    pub fn of(spectrum: &JointSpectrum, mean_pairs: f64) -> Result<Source> {
        Source::new(&spectrum.schmidt()?.weights, mean_pairs)
    }

    /// The Schmidt weights λ_k, largest first, summing to 1.
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }

    /// The squeezing r_k of each pair of Schmidt modes.
    pub fn squeezing(&self) -> &[f64] {
        &self.squeezing
    }

    /// The mean number of pairs, Σ sinh² r_k.
    pub fn mean_pairs(&self) -> f64 {
        self.squeezing
            .iter()
            .map(|r| portable::powi(portable::sinh(*r), 2))
            .sum()
    }

    /// The probabilities of 0, 1, … `most` pairs in a pulse: the geometric numbers of the
    /// modes convolved.
    pub fn pairs(&self, most: usize) -> Vec<f64> {
        let mut p = vec![0.0; most + 1];
        p[0] = 1.0;
        for r in &self.squeezing {
            let t = portable::powi(portable::tanh(*r), 2);
            let mode: Vec<f64> = (0..=most)
                .map(|n| (1.0 - t) * portable::powi(t, n as i32))
                .collect();
            let mut next = vec![0.0; most + 1];
            for (i, a) in p.iter().enumerate() {
                for (j, b) in mode.iter().enumerate().take(most + 1 - i) {
                    next[i + j] += a * b;
                }
            }
            p = next;
        }
        p
    }

    /// The share of pulses with any pair that hold more than one: P(n ≥ 2) / P(n ≥ 1), the
    /// multi-pair noise a photon-number-resolving herald would see.
    pub fn multi_pair(&self) -> f64 {
        let p = self.pairs(1);
        let any = 1.0 - p[0];
        if any <= 0.0 {
            return 0.0;
        }
        (any - p[1]) / any
    }

    /// Either arm's unheralded g⁽²⁾(0), 1 + Σ n̄_k² / (Σ n̄_k)².
    pub fn unheralded_g2(&self) -> f64 {
        let n: Vec<f64> = self
            .squeezing
            .iter()
            .map(|r| portable::powi(portable::sinh(*r), 2))
            .collect();
        let total: f64 = n.iter().sum();
        1.0 + n.iter().map(|x| x * x).sum::<f64>() / (total * total)
    }

    /// The source as a Gaussian state of its first `modes` pairs of Schmidt modes (all of them
    /// when fewer): mode k the signal's k-th Schmidt mode, mode `modes + k` the idler's.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no modes.
    pub fn gaussian(&self, modes: usize) -> Result<Gaussian> {
        let k = modes.min(self.squeezing.len());
        if k == 0 {
            return Err(invalid("a Gaussian state needs at least one pair of modes"));
        }
        let mut g = Gaussian::vacuum(2 * k);
        for (j, &r) in self.squeezing.iter().take(k).enumerate() {
            g = g.two_mode_squeezed(j, k + j, r, 0.0)?;
        }
        Ok(g)
    }
}

/// The visibility of the Hong–Ou–Mandel interference between the heralded signals of two
/// sources, Tr(ρ_A ρ_B) with ρ(ν, ν′) = Σ_i f(ν, ν_i) f*(ν′, ν_i) normalized: ‖F_A† F_B‖² /
/// (‖F_A‖² ‖F_B‖²), F the amplitudes as signal × idler matrices. Their signals must share a
/// grid; their idlers are traced out each on its own. The purity for a source with itself.
///
/// # Errors
///
/// [`Error::InvalidValue`] for signal grids that differ, or a spectrum that is zero.
pub fn hom_visibility(a: &JointSpectrum, b: &JointSpectrum) -> Result<f64> {
    if a.signal.len() != b.signal.len()
        || a.signal
            .iter()
            .zip(&b.signal)
            .any(|(x, y)| (x - y).abs() > 1e-12 * x.abs())
    {
        return Err(invalid(
            "two sources interfere on a common signal grid: their signal frequencies differ",
        ));
    }
    let (ns, na, nb) = (a.signal.len(), a.idler.len(), b.idler.len());
    let fa = |s: usize, i: usize| a.amplitude[s * na + i];
    let fb = |s: usize, j: usize| b.amplitude[s * nb + j];
    let norm_a: f64 = a.amplitude.iter().map(c64::norm_sqr).sum();
    let norm_b: f64 = b.amplitude.iter().map(c64::norm_sqr).sum();
    if !(norm_a > 0.0 && norm_b > 0.0) {
        return Err(invalid("a joint spectrum is zero"));
    }
    let mut overlap = 0.0;
    for i in 0..na {
        for j in 0..nb {
            let m: c64 = (0..ns).map(|s| fa(s, i).conj() * fb(s, j)).sum();
            overlap += m.norm_sqr();
        }
    }
    Ok(overlap / (norm_a * norm_b))
}

/// The amplitude transmissions of `component` from port `from` to port `to` at a grid's
/// frequencies (1/µm, as [`JointSpectrum::signal`] holds them): what the photon of that arm
/// passes on its way to its detector, filters, couplers and loss.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a frequency that isn't a wavelength, and the component's own
/// ([`transmission`]).
pub fn path(
    component: &dyn Component,
    values: &[f64],
    from: &str,
    to: &str,
    frequencies: &[f64],
) -> Result<Vec<c64>> {
    let wavelengths: Vec<Wavelength> = frequencies
        .iter()
        .map(|nu| Wavelength::um(1.0 / nu))
        .collect::<Result<_>>()?;
    transmission(component, from, to, values, &wavelengths)
}

/// The heralding efficiencies of a joint spectrum with each photon's path through a component
/// (its filters, couplers and loss): [`path`] at the signal's and the idler's frequencies, then
/// [`JointSpectrum::heralding`].
///
/// # Errors
///
/// As [`path`] and [`JointSpectrum::heralding`].
pub fn heralding_through(
    spectrum: &JointSpectrum,
    component: &dyn Component,
    values: &[f64],
    signal: (&str, &str),
    idler: (&str, &str),
) -> Result<Heralding> {
    let ts = path(component, values, signal.0, signal.1, &spectrum.signal)?;
    let ti = path(component, values, idler.0, idler.1, &spectrum.idler)?;
    spectrum.heralding(&ts, &ti)
}
