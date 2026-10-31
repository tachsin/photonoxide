//! Optical parametric oscillators: a χ⁽²⁾ guide in a cavity that feeds the signal back (singly
//! resonant) or the signal and the idler (doubly resonant), the pump through it once. Their
//! threshold and their steady state above it, on top of the parametric amplification of
//! [`super::coupled::ThreeWave`].
//!
//! # Conventions
//!
//! Those of [`super::coupled`] (lengths in µm, amplitudes in √W, κ in 1/(√W·µm)), and:
//!
//! - **The waves** are [`super::coupled::ThreeWave`]'s: 1 the signal, 2 the idler, 3 the pump,
//!   ω₃ = ω₁ + ω₂, the guide's mismatch and losses its own.
//! - **The cavity** returns a share R_j of each resonant wave's power at the guide's end to its
//!   start, with a round-trip phase φ_j from the nearest resonance: A_j(0) ← √R_j e^{iφ_j} A_j(L).
//!   R_j holds the mirrors' or coupler's reflectance and the cavity's other losses; T_j, the
//!   share of the power at the guide's end that leaves as output, is at most 1 − R_j. A wave
//!   that isn't resonant starts each pass at zero; the pump is fresh each pass.
//! - **Threshold** is the pump power at which the round trip, with the pump undepleted, first
//!   returns the signal and idler (A₁, A₂*) as large as they started: the spectral radius of
//!   its 2 × 2 matrix reaches 1. With Δk = 0, no loss in the guide and φ_j = 0 this is
//!   cosh(gL) = 1 + l₁l₂/(2 − l₁ − l₂), l_j = 1 − √R_j (R. W. Boyd, *Nonlinear Optics*, 3rd ed.,
//!   Academic Press (2008), Eq. (2.9.8)), with g² = κ²ν₁ν₂|A₃|²/ν₃²; singly resonant, l₂ = 1,
//!   cosh(gL) = 1/√R₁.
//! - **The steady state** is found by running the round trips, the coupled equations depleted,
//!   from a weak signal until the signal's power at the guide's end changes by less than a
//!   tolerance between round trips.

use num_complex::Complex64 as c64;

use super::coupled::ThreeWave;
use crate::{Error, Result};

/// What the cavity does to one resonant wave each round trip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Feedback {
    /// R: the share of the power at the guide's end returned to its start.
    pub returned: f64,
    /// T: the share of the power at the guide's end that leaves as output (T ≤ 1 − R).
    pub output: f64,
    /// φ: the round trip's phase from the nearest resonance, rad.
    pub phase: f64,
}

impl Feedback {
    /// A cavity on resonance (φ = 0) returning `returned` of the power and coupling out the rest
    /// (T = 1 − R): a lossless output coupler.
    pub fn resonant(returned: f64) -> Feedback {
        Feedback {
            returned,
            output: 1.0 - returned,
            phase: 0.0,
        }
    }

    fn factor(&self) -> c64 {
        c64::from_polar(self.returned.sqrt(), self.phase)
    }

    fn check(&self) -> Result<()> {
        if !((0.0..=1.0).contains(&self.returned)
            && self.output >= 0.0
            && self.output <= 1.0 - self.returned + 1e-15
            && self.phase.is_finite())
        {
            return Err(Error::invalid(
                "parametric oscillator",
                format!(
                    "the feedback needs 0 ≤ R ≤ 1, 0 ≤ T ≤ 1 − R and a finite phase, got R = {}, T = {}",
                    self.returned, self.output
                ),
            ));
        }
        Ok(())
    }
}

/// An optical parametric oscillator: the guide's process and length, and the cavity's feedback
/// of the signal and, if doubly resonant, of the idler.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oscillator {
    /// The guide: wave 1 the signal, 2 the idler, 3 the pump.
    pub process: ThreeWave,
    /// The guide's length, µm.
    pub length: f64,
    /// The signal's feedback.
    pub signal: Feedback,
    /// The idler's, or none: singly resonant.
    pub idler: Option<Feedback>,
}

/// An oscillator's steady state at one pump power.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SteadyState {
    /// The signal's output power, W: T₁ times its power at the guide's end.
    pub signal: f64,
    /// The idler's output power, W: T₂ times its power at the guide's end, or all of it when the
    /// idler isn't resonant.
    pub idler: f64,
    /// The pump's power left at the guide's end, W.
    pub pump: f64,
    /// The amplitudes at the guide's end: signal, idler and pump.
    pub amplitudes: [c64; 3],
    /// The round trips run.
    pub round_trips: usize,
    /// The last round trip's relative change of the signal's power: below the tolerance when
    /// settled.
    pub change: f64,
}

impl Oscillator {
    fn check(&self, steps: usize) -> Result<()> {
        self.signal.check()?;
        if let Some(f) = &self.idler {
            f.check()?;
        }
        if !(self.length.is_finite() && self.length > 0.0) || steps == 0 {
            return Err(Error::invalid(
                "parametric oscillator",
                format!(
                    "needs a positive length and steps, got {} um and {steps}",
                    self.length
                ),
            ));
        }
        Ok(())
    }

    /// The undepleted round trip's spectral radius at `pump` W: its 2 × 2 matrix on (A₁, A₂*)
    /// from the guide's transfer, taken on `steps` RK4 steps with seeds 10⁻⁹ of the pump's
    /// amplitude, times the cavity's feedback.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a feedback that isn't [`Feedback`]'s, a length that isn't
    /// positive, no steps, or a pump power that isn't positive and finite.
    pub fn round_trip_gain(&self, pump: f64, steps: usize) -> Result<f64> {
        self.check(steps)?;
        if !(pump.is_finite() && pump > 0.0) {
            return Err(Error::invalid(
                "parametric oscillator",
                format!("needs a positive pump power, got {pump} W"),
            ));
        }
        let ap = pump.sqrt();
        let eps = 1e-9 * ap;
        let zero = c64::new(0.0, 0.0);
        let column = |s: c64, i: c64| -> Result<[c64; 2]> {
            let out = self
                .process
                .propagate([s, i, c64::new(ap, 0.0)], self.length, steps)?;
            Ok([out[0] / eps, out[1].conj() / eps])
        };
        // the columns for A₁(0) = ε and for A₂*(0) = ε
        let t1 = column(c64::new(eps, 0.0), zero)?;
        let t2 = column(zero, c64::new(eps, 0.0))?;
        let fs = self.signal.factor();
        let fi = self.idler.map_or(zero, |f| f.factor().conj());
        let m = [[fs * t1[0], fs * t2[0]], [fi * t1[1], fi * t2[1]]];
        // the eigenvalues of a 2 × 2 matrix
        let tr = m[0][0] + m[1][1];
        let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
        let root = (tr * tr - 4.0 * det).sqrt();
        let radius = ((tr + root) * 0.5).norm().max(((tr - root) * 0.5).norm());
        if !radius.is_finite() {
            return Err(Error::invalid(
                "parametric oscillator",
                format!("the round trip isn't finite at {pump} W on {steps} steps"),
            ));
        }
        Ok(radius)
    }

    /// The threshold pump power, W, where [`Oscillator::round_trip_gain`] reaches 1: by
    /// bisection to a relative 10⁻¹³, from a bracket doubled up from 1 µW.
    ///
    /// # Errors
    ///
    /// As [`Oscillator::round_trip_gain`], and [`Error::InvalidValue`] if no pump below 10¹² W
    /// reaches threshold (no feedback, or no coupling).
    pub fn threshold(&self, steps: usize) -> Result<f64> {
        let excess = |p: f64| self.round_trip_gain(p, steps).map(|g| g - 1.0);
        let (mut lo, mut hi) = (1e-6, 1e-6);
        if excess(lo)? >= 0.0 {
            while excess(lo)? >= 0.0 {
                lo *= 0.5;
                if lo < 1e-30 {
                    return Err(Error::invalid(
                        "parametric oscillator",
                        "it oscillates at every pump power: the feedback has gain",
                    ));
                }
            }
            hi = 2.0 * lo;
        } else {
            while excess(hi)? < 0.0 {
                hi *= 2.0;
                if hi > 1e12 {
                    return Err(Error::invalid(
                        "parametric oscillator",
                        "no pump below 1e12 W reaches threshold",
                    ));
                }
            }
            lo = 0.5 * hi;
        }
        while hi - lo > 1e-13 * hi {
            let mid = 0.5 * (lo + hi);
            if excess(mid)? < 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        Ok(0.5 * (lo + hi))
    }

    /// The steady state at `pump` W: the round trips run with the depleted coupled equations on
    /// `steps` RK4 steps a pass, from a signal of 10⁻⁶ of the pump's power, until the signal's
    /// power at the guide's end changes by less than `tolerance` (relative) between round trips,
    /// or `round_trips` have run. Below threshold the signal decays to nothing.
    ///
    /// # Errors
    ///
    /// As [`Oscillator::round_trip_gain`].
    pub fn steady_state(
        &self,
        pump: f64,
        steps: usize,
        tolerance: f64,
        round_trips: usize,
    ) -> Result<SteadyState> {
        self.check(steps)?;
        if !(pump.is_finite() && pump > 0.0) {
            return Err(Error::invalid(
                "parametric oscillator",
                format!("needs a positive pump power, got {pump} W"),
            ));
        }
        let ap = c64::new(pump.sqrt(), 0.0);
        let zero = c64::new(0.0, 0.0);
        let (mut s, mut i) = (1e-3 * ap, zero);
        let mut last = f64::NAN;
        let mut change = f64::INFINITY;
        let mut out = [zero; 3];
        let mut n = 0;
        while n < round_trips {
            out = self.process.propagate([s, i, ap], self.length, steps)?;
            n += 1;
            let p = out[0].norm_sqr();
            change = ((p - last) / p).abs();
            last = p;
            s = self.signal.factor() * out[0];
            i = self.idler.map_or(zero, |f| f.factor() * out[1]);
            if change < tolerance || p < 1e-30 * pump {
                break;
            }
        }
        Ok(SteadyState {
            signal: self.signal.output * out[0].norm_sqr(),
            idler: self.idler.map_or(1.0, |f| f.output) * out[1].norm_sqr(),
            pump: out[2].norm_sqr(),
            amplitudes: out,
            round_trips: n,
            change,
        })
    }
}

/// The pump's depletion D = 1 − P₃(L)/P₃(0) of a singly resonant oscillator whose signal is the
/// same all along the guide (the limit of little loss of the signal per round trip), at
/// N = P₃/P_th times threshold, phase matched and lossless: D = sin²x, where x²/sin²x = N
/// (the coupled equations with A₁ held: A₃ ∝ cos(gz), the signal's gain per pass
/// ∝ sin²(gL)/g², equal to its threshold's L²); 1 at N = π²/4, and less above it as the pump
/// comes back. 0 below threshold.
pub fn uniform_depletion(times_threshold: f64) -> f64 {
    if times_threshold <= 1.0 {
        return 0.0;
    }
    // x/sin x = √N on 0 < x < π, increasing: bisection
    let target = times_threshold.sqrt();
    let (mut lo, mut hi) = (1e-9, std::f64::consts::PI * (1.0 - 1e-12));
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if mid / mid.sin() < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (0.5 * (lo + hi)).sin().powi(2)
}

#[cfg(test)]
#[path = "opo_tests.rs"]
mod tests;
