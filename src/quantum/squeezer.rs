//! Squeezing from 0.6.1's parametric amplifiers: the quantum map of an amplifier read off its
//! classical one.
//!
//! An undepleted parametric amplifier is linear in its signal: the classical amplitudes leave as
//! A′ = μ A + ν A* (one mode, the degenerate amplifier) or A′_s = μ A_s + ν A_i* (two modes),
//! and the same Bogoliubov map, on the annihilation operators in photon units, is the quantum
//! amplifier: â′ = μ â + ν â†, |μ|² − |ν|² = 1, a squeezer of r = asinh|ν| (C. Weedbrook et al.,
//! Rev. Mod. Phys. 84, 621 (2012), doi:10.1103/RevModPhys.84.621, Sec. II.B). So μ and ν come
//! from two classical runs with a weak seed, ε and iε, through the coupled equations as 0.6.1
//! integrates them ([`crate::nonlinear::coupled`], [`crate::nonlinear::fwm`]); the seed is
//! small enough to leave at 10⁻⁶ of the pump's amplitude, so its own depletion of the pump, of
//! order 10⁻¹² of its power, is below the integration's error. Without loss only: a lossy amplifier adds noise that a Bogoliubov map
//! can't carry, so the loss is applied to the squeezed state after it ([`super::Gaussian::lossy`]).

use num_complex::Complex64 as c64;

use crate::nonlinear::coupled::{SecondHarmonic, ThreeWave};
use crate::nonlinear::fwm::{FourWave, Waves};
use crate::portable;
use crate::{Error, Result};

/// A squeezer: its Bogoliubov coefficients and what they amount to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Squeezer {
    /// μ: â′ = μ â + ν â† (or ν b̂† for two modes).
    pub mu: c64,
    /// ν.
    pub nu: c64,
}

impl Squeezer {
    /// The squeezing r = asinh|ν|.
    pub fn r(&self) -> f64 {
        portable::asinh(portable::complex::abs(self.nu))
    }

    /// The squeezing's phase φ, ξ = r e^{iφ} in [`super::Gaussian::squeezed`]'s convention:
    /// â′ = â cosh r − â† e^{iφ} sinh r up to the output's phase, so e^{iφ} = −ν μ*/|ν μ|.
    pub fn phase(&self) -> f64 {
        portable::complex::arg(-self.nu * self.mu.conj())
    }

    /// How far it is from a Bogoliubov map, ||μ|² − |ν|² − 1| / |μ|²: the integration's error,
    /// relative to the gain (at high gain |μ|² and |ν|² are large and nearly equal).
    pub fn bogoliubov_error(&self) -> f64 {
        (self.mu.norm_sqr() - self.nu.norm_sqr() - 1.0).abs() / self.mu.norm_sqr()
    }

    /// The phase-sensitive gain of the amplified quadrature, (|μ| + |ν|)² = e^{2r}, in dB.
    pub fn gain_db(&self) -> f64 {
        20.0 * portable::log10(portable::complex::abs(self.mu) + portable::complex::abs(self.nu))
    }
}

fn no_loss(loss: &[f64], what: &'static str) -> Result<()> {
    if loss.iter().any(|&a| a != 0.0) {
        return Err(Error::invalid(
            what,
            "squeezing is read off a lossless amplifier: apply the loss to the squeezed state after it",
        ));
    }
    Ok(())
}

/// The seed's largest amplitude after the amplifier, as a share of the pump's: kept at 10⁻⁶, so
/// the seed's own pull on the pump, 10⁻¹² of its power, is below the integration's error.
const SEED: f64 = 1e-6;

/// Runs `run` with a seed small enough that its largest output stays at [`SEED`] of `pump`: the
/// first try at that size, then scaled down by what the amplifier gained.
fn seeded<T>(
    pump: f64,
    what: &'static str,
    run: impl Fn(f64) -> Result<(T, f64)>,
) -> Result<(T, f64)> {
    let mut eps = SEED * pump;
    if eps == 0.0 {
        return Err(Error::invalid(what, "needs a pump"));
    }
    for _ in 0..4 {
        let (out, largest) = run(eps)?;
        if largest <= 1.01 * SEED * pump {
            return Ok((out, eps));
        }
        eps *= SEED * pump / largest;
    }
    Ok((run(eps)?.0, eps))
}

/// The single-mode squeezer of a degenerate parametric amplifier: second-harmonic generation's
/// equations run backwards from a harmonic pump of amplitude `pump` (√W) over `length` µm in
/// `steps` RK4 steps ([`SecondHarmonic::propagate`]), the fundamental's map read off.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a lossy amplifier or a pump of zero amplitude, and the
/// integration's own.
pub fn degenerate(
    amplifier: &SecondHarmonic,
    pump: c64,
    length: f64,
    steps: usize,
) -> Result<Squeezer> {
    no_loss(&amplifier.loss, "degenerate amplifier")?;
    let run = |eps: f64| -> Result<([c64; 2], f64)> {
        let a = amplifier.propagate([c64::new(eps, 0.0), pump], length, steps)?[0];
        let b = amplifier.propagate([c64::new(0.0, eps), pump], length, steps)?[0];
        Ok((
            [a, b],
            portable::complex::abs(a).max(portable::complex::abs(b)),
        ))
    };
    let ([a, b], eps) = seeded(portable::complex::abs(pump), "degenerate amplifier", run)?;
    // a = (μ + ν) ε, b = i (μ − ν) ε
    let plus = a / eps;
    let minus = b / (c64::new(0.0, 1.0) * eps);
    Ok(Squeezer {
        mu: 0.5 * (plus + minus),
        nu: 0.5 * (plus - minus),
    })
}

/// The two-mode squeezer of a non-degenerate parametric amplifier (difference-frequency
/// generation, waves 1 and 2 the signal and idler, 3 the pump of amplitude `pump` √W): the
/// signal seeded alone, μ and ν from the signal's and the idler's amplitudes in photon units
/// (each amplitude over √ν_j, ν_j its frequency).
///
/// # Errors
///
/// As [`degenerate`].
pub fn non_degenerate(
    amplifier: &ThreeWave,
    pump: c64,
    length: f64,
    steps: usize,
) -> Result<Squeezer> {
    no_loss(&amplifier.loss, "parametric amplifier")?;
    let [n1, n2] = amplifier.frequencies;
    let run = |eps: f64| -> Result<([c64; 3], f64)> {
        let out = amplifier.propagate(
            [c64::new(eps, 0.0), c64::new(0.0, 0.0), pump],
            length,
            steps,
        )?;
        Ok((
            out,
            portable::complex::abs(out[0]).max(portable::complex::abs(out[1])),
        ))
    };
    let (out, eps) = seeded(portable::complex::abs(pump), "parametric amplifier", run)?;
    // photon units: a_j = A_j / √ν_j; the seed's ε/√ν₁ in, a₂′ = ν a₁*
    let seed = eps / n1.sqrt();
    Ok(Squeezer {
        mu: out[0] / n1.sqrt() / seed,
        nu: out[1] / n2.sqrt() / seed,
    })
}

/// The two-mode squeezer of four-wave mixing with one pump (spontaneous four-wave mixing's
/// classical twin), with the pump's self- and cross-phase modulation: the signal seeded alone
/// through [`FourWave::propagate`] over `length` m with `pump` √W.
///
/// # Errors
///
/// As [`degenerate`].
pub fn four_wave(amplifier: &FourWave, pump: c64, length: f64, steps: usize) -> Result<Squeezer> {
    let run = |eps: f64| -> Result<(Waves, f64)> {
        let out = amplifier.propagate(
            Waves {
                pump,
                signal: c64::new(eps, 0.0),
                idler: c64::new(0.0, 0.0),
            },
            length,
            steps,
        )?;
        Ok((
            out,
            portable::complex::abs(out.signal).max(portable::complex::abs(out.idler)),
        ))
    };
    let (out, eps) = seeded(portable::complex::abs(pump), "four-wave mixing", run)?;
    Ok(Squeezer {
        mu: out.signal / eps,
        nu: out.idler / eps,
    })
}
