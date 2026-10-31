//! Four-wave mixing in a Kerr guide with one pump: parametric gain and wavelength conversion,
//! the phase mismatch from the guide's modes.
//!
//! G. P. Agrawal, *Nonlinear Fiber Optics*, 6th ed., Academic Press (2019),
//! doi:10.1016/C2018-0-01168-8, Chapter 10.
//!
//! # Conventions
//!
//! Those of [`super::pulse`] (z in m, γ in 1/(W·m), powers in W, amplitudes in √W), and:
//!
//! - **The process** is 2ω_p = ω_s + ω_i, signal and idler at ω_p ± Ω.
//! - **The linear mismatch** is Δk = β_s + β_i − 2β_p, rad/m: Agrawal's Eq. (10.1.8) with ω₁ = ω₂
//!   the pump, and [`super::phase`]'s sign for four waves. From a guide's dispersion it is
//!   [`super::pulse::Dispersion::four_wave_mismatch`], 2 Σ β_{2m}Ω^{2m}/(2m)! (Eq. (10.4.12)).
//!   Foster et al. (Nature 441, 960 (2006)) write Δk_L = 2k_p − k_s − k_i, the opposite sign.
//! - **The effective mismatch** κ = Δk + 2γP (Eq. (10.2.21)), the pump's self- and cross-phase
//!   modulation included; the gain g = ((γP)² − (κ/2)²)^½ (Eq. (10.2.19) with r = 1). Where
//!   |κ/2| > γP, g is imaginary and the signal oscillates instead of growing; the closed forms
//!   below take sinh(gL)/g → sin(|g|L)/|g| there, and L at g = 0.
//! - **The coupled equations** (Eqs. (10.2.1)–(10.2.4) with one pump, all overlaps 1/A_eff,
//!   Eq. (10.2.6), and one γ, Eq. (10.2.7)):
//!   dA_p/dz = iγ[(|A_p|² + 2|A_s|² + 2|A_i|²)A_p + 2A_p*A_sA_i e^{iΔkz}],
//!   dA_s/dz = iγ[(|A_s|² + 2|A_p|² + 2|A_i|²)A_s + A_p²A_i* e^{−iΔkz}], and the idler's with s
//!   and i swapped (the undepleted limit is Eqs. (10.2.24)–(10.2.25)). They conserve
//!   |A_p|² + |A_s|² + |A_i|² and |A_s|² − |A_i|², which the integration, by the classical
//!   fourth-order Runge–Kutta method, keeps to its own error.

use num_complex::Complex64 as c64;

use crate::{Error, Result};

/// The effective mismatch κ = Δk + 2γP (Agrawal Eq. (10.2.21)), rad/m.
pub fn effective_mismatch(gamma: f64, pump: f64, mismatch: f64) -> f64 {
    mismatch + 2.0 * gamma * pump
}

/// g² = (γP)² − (κ/2)² (Eq. (10.2.19)), 1/m²: the gain where positive.
pub fn gain_squared(gamma: f64, pump: f64, mismatch: f64) -> f64 {
    let k = 0.5 * effective_mismatch(gamma, pump, mismatch);
    (gamma * pump).powi(2) - k * k
}

/// The parametric gain g (1/m), 0 where the process doesn't amplify (|κ/2| ≥ γP).
pub fn parametric_gain(gamma: f64, pump: f64, mismatch: f64) -> f64 {
    gain_squared(gamma, pump, mismatch).max(0.0).sqrt()
}

/// sinh(gL)/g with g² given, continued to sin(|g|L)/|g| for g² < 0 and L at g = 0.
fn sinhc(g2: f64, length: f64) -> f64 {
    if g2 > 0.0 {
        let g = g2.sqrt();
        (g * length).sinh() / g
    } else if g2 < 0.0 {
        let g = (-g2).sqrt();
        (g * length).sin() / g
    } else {
        length
    }
}

/// The idler's power at `length` m per unit of signal power in, with no idler in:
/// P_i(L)/P_s(0) = (γP sinh(gL)/g)² (Eq. (10.4.6), whose 1 + κ²/4g² is (γP/g)²). The
/// conversion efficiency.
pub fn conversion(gamma: f64, pump: f64, mismatch: f64, length: f64) -> f64 {
    (gamma * pump * sinhc(gain_squared(gamma, pump, mismatch), length)).powi(2)
}

/// The signal's gain G = P_s(L)/P_s(0) = 1 + (γP sinh(gL)/g)² (Eq. (10.4.7) with r = 1), with
/// no idler in.
pub fn signal_gain(gamma: f64, pump: f64, mismatch: f64, length: f64) -> f64 {
    1.0 + conversion(gamma, pump, mismatch, length)
}

/// The detuning Ω (rad/ps) of the gain's peak, where κ = 0 with Δk ≈ β₂Ω²:
/// Ω = (2γP/|β₂|)^½ for β₂ < 0 (Agrawal, p. 423); None for β₂ ≥ 0.
pub fn peak_detuning(gamma: f64, pump: f64, beta2: f64) -> Option<f64> {
    (beta2 < 0.0).then(|| (2.0 * gamma * pump / beta2.abs()).sqrt())
}

/// Degenerate four-wave mixing along a guide, depleted and with all the self- and cross-phase
/// terms: the coupled equations of the [module's conventions](self).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FourWave {
    /// γ, 1/(W·m).
    pub gamma: f64,
    /// Δk = β_s + β_i − 2β_p, rad/m.
    pub mismatch: f64,
}

/// The pump's, signal's and idler's amplitudes, √W.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Waves {
    /// A_p.
    pub pump: c64,
    /// A_s.
    pub signal: c64,
    /// A_i.
    pub idler: c64,
}

impl Waves {
    /// The total power, W.
    pub fn power(&self) -> f64 {
        self.pump.norm_sqr() + self.signal.norm_sqr() + self.idler.norm_sqr()
    }
}

impl FourWave {
    fn rate(&self, z: f64, w: &Waves) -> Waves {
        let (p, s, i) = (w.pump, w.signal, w.idler);
        let (pp, ps, pi) = (p.norm_sqr(), s.norm_sqr(), i.norm_sqr());
        let e = c64::new(0.0, self.mismatch * z).exp();
        let ig = c64::new(0.0, self.gamma);
        Waves {
            pump: ig * ((pp + 2.0 * ps + 2.0 * pi) * p + 2.0 * p.conj() * s * i * e),
            signal: ig * ((ps + 2.0 * pp + 2.0 * pi) * s + p * p * i.conj() * e.conj()),
            idler: ig * ((pi + 2.0 * pp + 2.0 * ps) * i + p * p * s.conj() * e.conj()),
        }
    }

    /// The waves after `length` m, from `start`, on `steps` RK4 steps.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a length that isn't finite and non-negative or no steps.
    pub fn propagate(&self, start: Waves, length: f64, steps: usize) -> Result<Waves> {
        if !(length.is_finite() && length >= 0.0) || steps == 0 {
            return Err(Error::invalid(
                "four-wave mixing",
                format!("needs a finite length ≥ 0 and steps, got {length} m, {steps}"),
            ));
        }
        let h = length / steps as f64;
        let add = |a: &Waves, k: &Waves, s: f64| Waves {
            pump: a.pump + k.pump * s,
            signal: a.signal + k.signal * s,
            idler: a.idler + k.idler * s,
        };
        let mut w = start;
        for n in 0..steps {
            let z = n as f64 * h;
            let k1 = self.rate(z, &w);
            let k2 = self.rate(z + 0.5 * h, &add(&w, &k1, 0.5 * h));
            let k3 = self.rate(z + 0.5 * h, &add(&w, &k2, 0.5 * h));
            let k4 = self.rate(z + h, &add(&w, &k3, h));
            w = Waves {
                pump: w.pump + (k1.pump + 2.0 * k2.pump + 2.0 * k3.pump + k4.pump) * (h / 6.0),
                signal: w.signal
                    + (k1.signal + 2.0 * k2.signal + 2.0 * k3.signal + k4.signal) * (h / 6.0),
                idler: w.idler
                    + (k1.idler + 2.0 * k2.idler + 2.0 * k3.idler + k4.idler) * (h / 6.0),
            };
        }
        Ok(w)
    }
}

#[cfg(test)]
#[path = "fwm_tests.rs"]
mod tests;
