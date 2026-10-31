//! Cross-phase modulation between two pulses at different wavelengths along a Kerr guide: the
//! coupled nonlinear Schrödinger equations, each pulse with its own dispersion, γ and loss, and
//! their group velocities' mismatch.
//!
//! G. P. Agrawal, *Nonlinear Fiber Optics*, 6th ed., Academic Press (2019),
//! doi:10.1016/C2018-0-01168-8, Section 7.4.
//!
//! # Conventions
//!
//! Those of [`super::pulse`], and:
//!
//! - **The equations** are Agrawal's Eqs. (7.4.1)–(7.4.2) with each pulse's loss and higher
//!   orders of dispersion: ∂A₁/∂z = D₁A₁ + iγ₁(|A₁|² + c|A₂|²)A₁ and
//!   ∂A₂/∂z + d∂A₂/∂T = D₂A₂ + iγ₂(|A₂|² + c|A₁|²)A₂, T in the frame moving with the first pulse,
//!   d = 1/v_g2 − 1/v_g1 (Eq. (7.4.3)) the walk-off in ps/m, positive when the second pulse is the
//!   slower. The cross-phase factor c is 2 for co-polarized pulses (Eq. (7.4.1)); 2/3 for
//!   orthogonal linear polarizations at one wavelength (Eq. (6.1.11)). Each γ_j is that
//!   pulse's own, n₂ω_j/(cA_eff), the overlaps f₁₁, f₂₂ and f₁₂ of Eq. (7.1.10) taken as one
//!   area, as Agrawal's Eqs. (7.1.11)–(7.1.13) take them; the four-wave terms between the
//!   pulses, which Eq. (7.1.9) doesn't carry, are left out.
//! - **Each pulse's spectrum** is about its own carrier ω₁ or ω₂, on one time grid: the same
//!   points and step for both.
//! - **Split-step.** The symmetrized split-step of [`super::pulse`]: each pulse's linear step in
//!   its own spectrum, the second's with e^{idΩh}, which moves it to later T by dh; the Kerr step
//!   exact, both moduli unchanged under it. Second order in the step.

use num_complex::Complex64 as c64;

use super::fft::Fft;
use super::pulse::{Guide, Pulse};
use crate::{Error, Result};

/// Two pulses at different wavelengths in one Kerr guide: each one's [`Guide`] at its own
/// carrier, the walk-off between them and the cross-phase factor.
#[derive(Clone, Debug, PartialEq)]
pub struct TwoPulses {
    /// The first pulse's guide: its dispersion about ω₁, γ₁ and loss. The frame moves with it.
    pub first: Guide,
    /// The second pulse's: about ω₂, γ₂ and its loss.
    pub second: Guide,
    /// d = 1/v_g2 − 1/v_g1, ps/m: `second.dispersion.beta(1) − first.dispersion.beta(1)` for
    /// dispersions from the modes ([`super::pulse::Dispersion::of_mode`]).
    pub walk_off: f64,
    /// The cross-phase factor c: 2 for co-polarized pulses.
    pub cross: f64,
}

impl TwoPulses {
    /// Co-polarized pulses (c = 2) in `first` and `second`, `walk_off` ps/m apart per metre.
    pub fn new(first: Guide, second: Guide, walk_off: f64) -> TwoPulses {
        TwoPulses {
            first,
            second,
            walk_off,
            cross: 2.0,
        }
    }

    /// The two pulses after `length` m on `steps` split steps.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for pulses on different grids, a guide with Raman or
    /// self-steepening (the coupled equations here are Kerr only), a length that isn't finite and
    /// non-negative, no steps, or a γ, loss, walk-off or cross factor that isn't finite.
    pub fn propagate(
        &self,
        first: &Pulse,
        second: &Pulse,
        length: f64,
        steps: usize,
    ) -> Result<(Pulse, Pulse)> {
        let n = first.field.len();
        if second.field.len() != n || first.step != second.step {
            return Err(Error::invalid(
                "cross-phase modulation",
                "the two pulses must be on one time grid: the same points and step",
            ));
        }
        let guides = [&self.first, &self.second];
        if guides
            .iter()
            .any(|g| g.raman.is_some() || g.self_steepening)
        {
            return Err(Error::invalid(
                "cross-phase modulation",
                "the coupled equations take the Kerr term alone: a guide without Raman or self-steepening",
            ));
        }
        let finite = guides
            .iter()
            .all(|g| g.gamma.is_finite() && g.loss.is_finite())
            && self.walk_off.is_finite()
            && self.cross.is_finite();
        if !(length.is_finite() && length >= 0.0) || steps == 0 || !finite {
            return Err(Error::invalid(
                "cross-phase modulation",
                format!(
                    "needs a finite length ≥ 0, steps, and finite γ, losses, walk-off and cross factor, got {length} m and {steps} steps"
                ),
            ));
        }
        let fft = Fft::new(n)?;
        let h = length / steps as f64;
        let omegas = first.frequencies();
        let linear = |g: &Guide, walk: f64, dz: f64| -> Vec<c64> {
            omegas
                .iter()
                .map(|&w| {
                    c64::new(-0.5 * g.loss * dz, (g.dispersion.phase(w) + walk * w) * dz).exp()
                })
                .collect()
        };
        let half = [
            linear(&self.first, 0.0, 0.5 * h),
            linear(&self.second, self.walk_off, 0.5 * h),
        ];
        let full = [
            linear(&self.first, 0.0, h),
            linear(&self.second, self.walk_off, h),
        ];
        let mut a = [first.field.clone(), second.field.clone()];
        let apply = |a: &mut [Vec<c64>; 2], f: &[Vec<c64>; 2]| {
            for (field, factor) in a.iter_mut().zip(f) {
                fft.inverse(field);
                for (v, x) in field.iter_mut().zip(factor) {
                    *v *= x;
                }
                fft.forward(field);
            }
        };
        let (g1, g2, c) = (self.first.gamma, self.second.gamma, self.cross);
        apply(&mut a, &half);
        for s in 0..steps {
            let [a1, a2] = &mut a;
            for (u, v) in a1.iter_mut().zip(a2.iter_mut()) {
                let (p1, p2) = (u.norm_sqr(), v.norm_sqr());
                *u *= c64::new(0.0, g1 * (p1 + c * p2) * h).exp();
                *v *= c64::new(0.0, g2 * (p2 + c * p1) * h).exp();
            }
            apply(&mut a, if s + 1 == steps { &half } else { &full });
        }
        let [f1, f2] = a;
        Ok((
            Pulse {
                step: first.step,
                field: f1,
            },
            Pulse {
                step: first.step,
                field: f2,
            },
        ))
    }
}

/// The first pulse's nonlinear phase at T (ps) after `length` m without dispersion or loss, two
/// unchirped Gaussians √P_j exp(−(T − T_j)²/2T₀²) of one width T₀ (ps), the second delayed by
/// `delay` ps, `walk_off` ps/m apart:
/// φ₁ = γ₁L[P₁e^{−τ²} + (cP₂/2)(√π/δ)(erf(τ − τ_d) − erf(τ − τ_d − δ))], τ = T/T₀, τ_d = T_d/T₀,
/// δ = dL/T₀ (Agrawal Eq. (7.4.9), there with c = 2; at δ = 0 the cross term is cP₂e^{−(τ−τ_d)²}).
#[allow(clippy::too_many_arguments)]
pub fn gaussian_phase(
    gamma: f64,
    peaks: [f64; 2],
    t0: f64,
    delay: f64,
    walk_off: f64,
    cross: f64,
    length: f64,
    t: f64,
) -> f64 {
    let (tau, td, delta) = (t / t0, delay / t0, walk_off * length / t0);
    let spm = peaks[0] * (-tau * tau).exp();
    let xpm = if delta.abs() < 1e-8 {
        cross * peaks[1] * (-(tau - td).powi(2)).exp()
    } else {
        0.5 * cross * peaks[1] * std::f64::consts::PI.sqrt() / delta
            * (libm::erf(tau - td) - libm::erf(tau - td - delta))
    };
    gamma * length * (spm + xpm)
}

#[cfg(test)]
#[path = "xpm_tests.rs"]
mod tests;
