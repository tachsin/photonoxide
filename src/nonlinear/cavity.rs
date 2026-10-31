//! Resonantly enhanced second-harmonic generation and four-wave mixing in a ring, the
//! enhancement from the 0.4 ring components
//! ([`crate::circuit::components::AllPassRing::ring_wave`]).
//!
//! # Conventions
//!
//! Those of [`super::coupled`] for SHG (lengths in µm, κ in 1/(√W·µm), Δk in rad/µm) and of
//! [`super::fwm`] for four-wave mixing (γ in 1/(W·m), Δk in rad/m), and:
//!
//! - **Each wave's ring** is an all-pass ring at that wave's wavelength, with its own coupling:
//!   a round trip multiplies the field by a e^{iφ}, and the field just past the coupler is
//!   F = ik/(1 − r a e^{iφ}) times the bus's ([`RingWave::all_pass`]; Helt et al. 2010's
//!   field-enhancement factor, with the loss). Every loss of the round trip,
//!   the guide's and the fixed loss per round trip, is spread along the ring, α = −2 ln a/L.
//! - **One pass** carries the generated wave as the undepleted coupled equations do, each
//!   driving field decaying as e^{−α_jz/2} from its value past the coupler; the generated field G
//!   at the pass's end builds up over the round trips, and leaves through the coupler as F e^{iφ}G:
//!   in the steady state, P_out = |F|²|G|². So
//!   P_2ω = |F_2ω|² κ²|F_ω|⁴P_ω²|I|² for SHG, with I = e^{−α₂L/2}(e^{xL} − 1)/x,
//!   x = −α₁ + α₂/2 − iΔk ([`super::coupled::undepleted_lossy_shg`] with |F_ω|²P_ω), and
//!   P_i/P_s = |F_i|²|F_s|²|F_p|⁴γ²P_p²|I|² for four-wave mixing, I = e^{−α_iL/2}(e^{xL} − 1)/x,
//!   x = −(2α_p + α_s − α_i)/2 − iΔk: without loss and on resonance, Helt et al.'s Eq. (5),
//!   (γP_pL)²|F_s|²|F_i|²|F_p|⁴.
//! - **Phase matching in a ring**: Δk is the waves' mismatch along the guide. Waves that are all
//!   on resonance (φ_j a whole number of turns) have ΔkL a whole number of turns too, and only
//!   ΔkL = 0 converts (the azimuthal orders conserved); a quasi-phase-matched or detuned ring
//!   gives its own Δk.
//! - **Valid** while the generated waves don't deplete the pump and the pump's own Kerr phase
//!   doesn't detune the ring: γP_p|F_p|²L well under the resonance's width 1 − ra.
//!
//! L. G. Helt, Z. Yang, M. Liscidini, J. E. Sipe, Opt. Lett. 35, 3006 (2010),
//! doi:10.1364/OL.35.003006.

use num_complex::Complex64 as c64;

use super::coupled::{SecondHarmonic, undepleted_lossy_shg};
use crate::{Error, Result};

/// One wave in its ring: the field enhancement F, the round trip's factor a e^{iφ} and the
/// ring's length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingWave {
    /// F = ik/(1 − r a e^{iφ}).
    pub enhancement: c64,
    /// a e^{iφ}.
    pub round_trip: c64,
    /// L, µm.
    pub length: f64,
}

impl RingWave {
    /// A wave in an all-pass ring of power coupling k² = `coupling`, round trip a e^{iφ} =
    /// `round_trip` and length `length` µm: F = ik/(1 − r a e^{iφ}), r = √(1 − k²). A ring
    /// component gives it at its wavelength and values
    /// ([`crate::circuit::components::AllPassRing::ring_wave`]).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a coupling outside 0 to 1, a round trip that isn't
    /// 0 < a ≤ 1, a length that isn't positive, or an enhancement that isn't finite.
    pub fn all_pass(coupling: f64, round_trip: c64, length: f64) -> Result<RingWave> {
        if !(0.0..=1.0).contains(&coupling) {
            return Err(Error::invalid(
                "ring",
                format!("a power coupling must be from 0 to 1, got {coupling}"),
            ));
        }
        let r = (1.0 - coupling).sqrt();
        let wave = RingWave {
            enhancement: c64::new(0.0, coupling.sqrt()) / (1.0 - r * round_trip),
            round_trip,
            length,
        };
        wave.check()?;
        Ok(wave)
    }

    /// The power's build-up |F|².
    pub fn build_up(&self) -> f64 {
        self.enhancement.norm_sqr()
    }

    /// The power loss α = −2 ln a/L, 1/µm: all the round trip's loss spread along the ring.
    pub fn loss(&self) -> f64 {
        -2.0 * self.round_trip.norm().ln() / self.length
    }

    /// The round trip's phase φ in (−π, π]: the detuning from the nearest resonance.
    pub fn detuning(&self) -> f64 {
        self.round_trip.arg()
    }

    fn check(&self) -> Result<()> {
        let a = self.round_trip.norm();
        if !(self.length.is_finite()
            && self.length > 0.0
            && a > 0.0
            && a <= 1.0
            && self.enhancement.norm().is_finite())
        {
            return Err(Error::invalid(
                "ring",
                format!(
                    "a wave in a ring needs a positive length, a round trip 0 < a ≤ 1 and a finite enhancement, got {} um and a = {a}",
                    self.length
                ),
            ));
        }
        Ok(())
    }
}

/// The second harmonic's power (W) out of a doubly resonant ring: `fundamental` and `harmonic`
/// in their rings, κ = `coupling` (1/(√W·µm), η = κ²), Δk = `mismatch` (k_2ω − 2k_ω − K_m,
/// rad/µm), `power` W of the fundamental in the bus. Undepleted: the conventions' P_2ω.
///
/// # Errors
///
/// [`Error::InvalidValue`] for waves of different lengths or that aren't a ring's, and a power,
/// coupling or mismatch that isn't finite.
pub fn second_harmonic(
    fundamental: &RingWave,
    harmonic: &RingWave,
    coupling: f64,
    mismatch: f64,
    power: f64,
) -> Result<f64> {
    fundamental.check()?;
    harmonic.check()?;
    same_ring(&[fundamental, harmonic])?;
    if !(coupling.is_finite() && mismatch.is_finite() && power.is_finite() && power >= 0.0) {
        return Err(Error::invalid(
            "ring",
            "needs a finite coupling and mismatch and a power ≥ 0",
        ));
    }
    let shg = SecondHarmonic {
        coupling,
        mismatch,
        loss: [fundamental.loss(), harmonic.loss()],
    };
    Ok(harmonic.build_up()
        * undepleted_lossy_shg(&shg, fundamental.build_up() * power, fundamental.length))
}

/// The idler's power out over the signal's power in (the conversion efficiency) of four-wave
/// mixing in a ring resonant at all three waves: γ = `gamma` (1/(W·m)), Δk = `mismatch`
/// (β_s + β_i − 2β_p, rad/m), `pump` W in the bus. Undepleted: the conventions' P_i/P_s.
///
/// # Errors
///
/// As [`second_harmonic`].
pub fn four_wave(
    pump: &RingWave,
    signal: &RingWave,
    idler: &RingWave,
    gamma: f64,
    mismatch: f64,
    power: f64,
) -> Result<f64> {
    for w in [pump, signal, idler] {
        w.check()?;
    }
    same_ring(&[pump, signal, idler])?;
    if !(gamma.is_finite() && mismatch.is_finite() && power.is_finite() && power >= 0.0) {
        return Err(Error::invalid(
            "ring",
            "needs a finite γ and mismatch and a pump power ≥ 0",
        ));
    }
    let length = pump.length;
    // per µm
    let (g, dk) = (gamma * 1e-6, mismatch * 1e-6);
    let (ap, as_, ai) = (pump.loss(), signal.loss(), idler.loss());
    let x = c64::new(-(2.0 * ap + as_ - ai) / 2.0, -dk);
    let integral = if x.norm() * length < 1e-8 {
        c64::new(length, 0.0)
    } else {
        ((x * length).exp() - 1.0) / x
    };
    let pp = pump.build_up() * power;
    Ok(idler.build_up()
        * signal.build_up()
        * (g * pp).powi(2)
        * (-ai * length).exp()
        * integral.norm_sqr())
}

fn same_ring(waves: &[&RingWave]) -> Result<()> {
    let l = waves[0].length;
    if waves.iter().any(|w| (w.length - l).abs() > 1e-12 * l) {
        return Err(Error::invalid(
            "ring",
            "the waves must share one ring: the same length",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "cavity_tests.rs"]
mod tests;
