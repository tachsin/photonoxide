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
//! - **An add-drop ring** ([`AddDropWave`]) has its drop coupler at L/2, a point (through r₂,
//!   across ik₂), the round trip's phase lumped at the input coupler as in the all-pass ring:
//!   F = ik₁/(1 − r₁r₂a e^{iφ}). Each driving field steps down by its r₂ there, so the pass's
//!   integral splits in two halves, the second weighted by the drivers' r₂ (r₂,ω² for SHG,
//!   r₂,p²r₂,s for FWM): G = S[r₂e^{−αL/2}I(0, L/2) + w e^{−αL/2}I(L/2, L)], I(u, v) =
//!   (e^{xv} − e^{xu})/x, α and r₂ the generated wave's. Its field before the input coupler is
//!   C = G/(1 − r₁r₂a e^{iφ}), the through port takes k₁²|C|² (= |F|²|G|²), and the drop port
//!   k₂²|r₁e^{iφ}e^{−αL/4}C + S e^{−αL/4}I(0, L/2)|², the field at the drop coupler
//!   ([`second_harmonic_add_drop`], [`four_wave_add_drop`]).
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
use crate::portable;
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
        -2.0 * portable::ln(portable::complex::abs(self.round_trip)) / self.length
    }

    /// The round trip's phase φ in (−π, π]: the detuning from the nearest resonance.
    pub fn detuning(&self) -> f64 {
        portable::complex::arg(self.round_trip)
    }

    fn check(&self) -> Result<()> {
        let a = portable::complex::abs(self.round_trip);
        if !(self.length.is_finite()
            && self.length > 0.0
            && a > 0.0
            && a <= 1.0
            && portable::complex::abs(self.enhancement).is_finite())
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
    let integral = if portable::complex::abs(x) * length < 1e-8 {
        c64::new(length, 0.0)
    } else {
        (portable::complex::exp(x * length) - 1.0) / x
    };
    let pp = pump.build_up() * power;
    Ok(idler.build_up()
        * signal.build_up()
        * portable::powi(g * pp, 2)
        * portable::exp(-ai * length)
        * integral.norm_sqr())
}

/// One wave in an add-drop ring: its input coupler's power coupling k₁², its drop coupler's
/// k₂² halfway round, and the ring as [`RingWave`] holds it, the round trip a e^{iφ} the guide's
/// and the fixed loss only, the field past the input coupler F = ik₁/(1 − r₁r₂a e^{iφ}) (the
/// field [`crate::circuit::components::AddDropRing`] has there).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AddDropWave {
    /// The ring: F with both couplers, the round trip without them.
    pub wave: RingWave,
    /// k₁², the input bus's coupler.
    pub coupling: f64,
    /// k₂², the drop bus's coupler, at L/2 from the input's.
    pub coupling_drop: f64,
}

/// The generated wave's power out of an add-drop ring's two ports, W (or over the signal's
/// power, for four-wave mixing).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ports {
    /// Out of the through port, past the input coupler.
    pub through: f64,
    /// Out of the drop port.
    pub drop: f64,
}

impl AddDropWave {
    /// A wave in an add-drop ring of couplings k₁² = `coupling` and k₂² = `coupling_drop`,
    /// round trip a e^{iφ} = `round_trip` (the couplers left out) and length `length` µm. With
    /// k₂² = 0 its wave is [`RingWave::all_pass`]'s.
    ///
    /// # Errors
    ///
    /// As [`RingWave::all_pass`], for either coupling.
    pub fn new(
        coupling: f64,
        coupling_drop: f64,
        round_trip: c64,
        length: f64,
    ) -> Result<AddDropWave> {
        if !(0.0..=1.0).contains(&coupling_drop) {
            return Err(Error::invalid(
                "ring",
                format!("a power coupling must be from 0 to 1, got {coupling_drop}"),
            ));
        }
        let r2 = (1.0 - coupling_drop).sqrt();
        // F from the round trip with the drop coupler in it; the round trip itself without
        let mut wave = RingWave::all_pass(coupling, r2 * round_trip, length)?;
        wave.round_trip = round_trip;
        wave.check()?;
        Ok(AddDropWave {
            wave,
            coupling,
            coupling_drop,
        })
    }

    fn through(&self) -> f64 {
        (1.0 - self.coupling_drop).sqrt()
    }

    /// The generated wave's two ports, from the source amplitude `source` (√W/µm) times e^{xz}
    /// along the first half and `second` times it along the second (the drivers' drop
    /// couplers), as the conventions' add-drop ring.
    fn ports(&self, source: c64, second: c64, x: c64) -> Ports {
        let (l, alpha) = (self.wave.length, self.wave.loss());
        let integral = |from: f64, to: f64| -> c64 {
            if portable::complex::abs(x) * l < 1e-8 {
                c64::new(to - from, 0.0)
            } else {
                (portable::complex::exp(x * to) - portable::complex::exp(x * from)) / x
            }
        };
        let quarter = portable::exp(-alpha * l / 4.0);
        // generated over each half, at its end
        let g1 = source * quarter * integral(0.0, l / 2.0);
        let g2 = source * second * quarter * quarter * integral(l / 2.0, l);
        let r1 = (1.0 - self.coupling).sqrt();
        let r2 = self.through();
        let end = (r2 * quarter * g1 + g2) / (1.0 - r1 * r2 * self.wave.round_trip);
        let mid =
            r1 * portable::complex::from_polar(1.0, self.wave.detuning()) * quarter * end + g1;
        Ports {
            through: self.coupling * end.norm_sqr(),
            drop: self.coupling_drop * mid.norm_sqr(),
        }
    }
}

/// The second harmonic's power (W) out of both ports of a doubly resonant add-drop ring: as
/// [`second_harmonic`], with each wave's drop coupler at L/2 (the conventions' add-drop ring).
/// With no drop couplers its through port is [`second_harmonic`]'s.
///
/// # Errors
///
/// As [`second_harmonic`].
pub fn second_harmonic_add_drop(
    fundamental: &AddDropWave,
    harmonic: &AddDropWave,
    coupling: f64,
    mismatch: f64,
    power: f64,
) -> Result<Ports> {
    let (f, h) = (&fundamental.wave, &harmonic.wave);
    f.check()?;
    h.check()?;
    same_ring(&[f, h])?;
    if !(coupling.is_finite() && mismatch.is_finite() && power.is_finite() && power >= 0.0) {
        return Err(Error::invalid(
            "ring",
            "needs a finite coupling and mismatch and a power ≥ 0",
        ));
    }
    let x = c64::new(-f.loss() + h.loss() / 2.0, -mismatch);
    let source = c64::new(coupling * f.build_up() * power, 0.0);
    let r2 = fundamental.through();
    Ok(harmonic.ports(source, c64::new(r2 * r2, 0.0), x))
}

/// The idler's power out of both ports of an add-drop ring resonant at all three waves, over the
/// signal's power in: as [`four_wave`], with each wave's drop coupler at L/2 (the conventions'
/// add-drop ring). With no drop couplers its through port is [`four_wave`]'s.
///
/// # Errors
///
/// As [`four_wave`].
pub fn four_wave_add_drop(
    pump: &AddDropWave,
    signal: &AddDropWave,
    idler: &AddDropWave,
    gamma: f64,
    mismatch: f64,
    power: f64,
) -> Result<Ports> {
    let (p, s, i) = (&pump.wave, &signal.wave, &idler.wave);
    for w in [p, s, i] {
        w.check()?;
    }
    same_ring(&[p, s, i])?;
    if !(gamma.is_finite() && mismatch.is_finite() && power.is_finite() && power >= 0.0) {
        return Err(Error::invalid(
            "ring",
            "needs a finite γ and mismatch and a pump power ≥ 0",
        ));
    }
    // per µm
    let x = c64::new(
        -(2.0 * p.loss() + s.loss() - i.loss()) / 2.0,
        -mismatch * 1e-6,
    );
    let source = c64::new(
        gamma * 1e-6 * p.build_up() * power * s.build_up().sqrt(),
        0.0,
    );
    let (rp, rs) = (pump.through(), signal.through());
    Ok(idler.ports(source, c64::new(rp * rp * rs, 0.0), x))
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
