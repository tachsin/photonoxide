//! Pulses along a waveguide: the nonlinear Schrödinger equation, from the guide's modes.
//!
//! G. P. Agrawal, *Nonlinear Fiber Optics*, 6th ed., Academic Press (2019),
//! doi:10.1016/C2018-0-01168-8 (Chapters 2–5). The guide's dispersion β₂, β₃, β₄, … comes from
//! its effective index over wavelength ([`Dispersion::of_mode`], from the mode solver through
//! [`super::guide::ModeIndex`]) and its γ = 2πn₂/(λA_eff) from its full-vector fields and the
//! catalogue's n₂ ([`crate::mode::fields::Fields::kerr_coefficient`]).
//!
//! # Conventions
//!
//! - **Units.** Distance z in m; the time T in ps, in the frame moving at the group velocity
//!   (T = t − β₁z, Agrawal Eq. (2.3.44)); angular frequencies in rad/ps; β_k = d^kβ/dω^k in
//!   ps^k/m; γ in 1/(W·m); the power loss α in 1/m; the amplitude A in √W, |A|² the power.
//!   Wavelengths in µm, as in the rest of [`super`].
//! - **The carrier** is e^{−iω₀t} (Agrawal Eq. (2.3.2)), so a spectral component e^{−iΩT} sits
//!   at ω₀ + Ω: Ω > 0 is bluer. The spectrum is Ã(Ω) = ∫ A(T) e^{iΩT} dT (Eq. (2.4.12)), and
//!   ∂/∂T becomes −iΩ.
//! - **The equation** is Agrawal's Eq. (2.3.36) with α₁ = 0 and γ₁ = γ/ω₀ (p. 44):
//!   ∂A/∂z = −(α/2)A + i Σ_{k≥2} (i^k β_k/k!) ∂^kA/∂T^k +
//!   iγ (1 + (i/ω₀) ∂/∂T) [A ∫₀^∞ R(T′)|A(T − T′)|² dT′], with
//!   R(T) = (1 − f_R)δ(T) + f_R h_R(T) (Eq. (2.3.33)). Without the Raman and self-steepening
//!   terms (both optional, off by default) it is the NLSE, Eq. (2.3.46) with β₃, β₄, … kept.
//! - **The dispersion operator** in the spectrum is D(Ω) = i Σ_{k≥2} β_kΩ^k/k! − α/2.
//! - **Split-step.** The symmetrized split-step Fourier method (Eq. (2.4.8)), its half steps of
//!   dispersion merged between steps (Eq. (2.4.10)): exp(hD/2) [exp(hN) exp(hD)]^{M−1} exp(hN)
//!   exp(hD/2). With the Kerr term alone the nonlinear step is exact (|A| doesn't change under it,
//!   Eq. (4.1.3)); with Raman or self-steepening it is the classical fourth-order Runge–Kutta
//!   method over the step. The scheme is second order in h (its leading error the double
//!   commutator, p. 47); the validation cases record the order.
//! - **The time grid** is a power of two of points, T_j = (j − n/2)·δT: periodic, as the FFT
//!   makes it (p. 49). The transforms are this crate's own radix-2 FFT on one thread, so a run
//!   is the same bit for bit whatever the number of threads.

use std::f64::consts::{PI, TAU};

use num_complex::Complex64 as c64;

use super::fft::Fft;
use super::guide::{ModeIndex, chebyshev_coefficients, chebyshev_derivative, clenshaw};
use crate::{Error, Result};

/// The speed of light in m/ps.
pub(crate) const C_M_PER_PS: f64 = crate::units::SPEED_OF_LIGHT * 1e-12;

/// The angular frequency (rad/ps) of a vacuum wavelength (µm).
pub fn angular_frequency(wavelength_um: f64) -> f64 {
    TAU * C_M_PER_PS * 1e6 / wavelength_um
}

/// The vacuum wavelength (µm) of an angular frequency (rad/ps).
pub fn wavelength_of(omega: f64) -> f64 {
    TAU * C_M_PER_PS * 1e6 / omega
}

/// A power loss α in 1/m from dB/cm: α = (ln 10/10)·dB/cm × 100.
pub fn loss_per_m(db_per_cm: f64) -> f64 {
    db_per_cm * std::f64::consts::LN_10 / 10.0 * 100.0
}

// ---------------------------------------------------------------------------------------------
// Dispersion
// ---------------------------------------------------------------------------------------------

/// The points of the Chebyshev interpolants [`Dispersion::of_index`] compares.
const FIT_POINTS: [usize; 4] = [9, 17, 33, 65];

/// A guide's dispersion at one wavelength: β₁, β₂, … β_K, the Taylor coefficients of β(ω)
/// about ω₀ (Agrawal Eqs. (2.3.23)–(2.3.24)), each with its estimated error.
#[derive(Clone, Debug, PartialEq)]
pub struct Dispersion {
    wavelength: f64,
    /// β_k at index k − 1, ps^k/m
    betas: Vec<f64>,
    /// each β_k's estimated error, the same units
    errors: Vec<f64>,
}

impl Dispersion {
    /// Given coefficients: β₂, β₃, … (ps^k/m, from index 0) at `wavelength_um`; β₁ is taken as 0,
    /// which the frame moving at the group velocity doesn't need, and the errors as 0.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a wavelength that isn't positive or a coefficient that isn't
    /// finite.
    pub fn new(wavelength_um: f64, from_beta2: &[f64]) -> Result<Dispersion> {
        if !(wavelength_um.is_finite() && wavelength_um > 0.0)
            || from_beta2.iter().any(|b| !b.is_finite())
        {
            return Err(Error::invalid(
                "dispersion",
                "needs a positive wavelength and finite coefficients",
            ));
        }
        let mut betas = vec![0.0];
        betas.extend_from_slice(from_beta2);
        Ok(Dispersion {
            wavelength: wavelength_um,
            errors: vec![0.0; betas.len()],
            betas,
        })
    }

    /// From an effective (or bulk) index n(λ), `index` taking λ in µm: β_1 … β_`order` at
    /// `wavelength_um` from Chebyshev interpolants of n(ω) on 9, 17, 33 and 65 points across
    /// `from` to `to` µm (Chebyshev points in ω), with β = ωn/c, so β_k = (ω₀ n^(k) + k n^(k−1))/c.
    /// Each β_k is taken from the pair of successive interpolants that agree best on it, the
    /// finer one's, and its error is their difference: more points shrink the interpolation's
    /// error but magnify the samples' round-off in a high derivative. A wider range holds the
    /// higher orders better.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a range that doesn't hold the wavelength, an order below 2 or
    /// above 8, or an index that isn't finite; and `index`'s errors.
    pub fn of_index(
        index: impl Fn(f64) -> Result<f64>,
        wavelength_um: f64,
        (from, to): (f64, f64),
        order: usize,
    ) -> Result<Dispersion> {
        if !(from.is_finite() && to.is_finite() && 0.0 < from && from < to)
            || !(from..=to).contains(&wavelength_um)
            || !(2..=8).contains(&order)
        {
            return Err(Error::invalid(
                "dispersion",
                format!(
                    "needs 0 < from < to around the wavelength and an order of 2 to 8, got {from} to {to} um at {wavelength_um}, order {order}"
                ),
            ));
        }
        let fits = FIT_POINTS
            .iter()
            .map(|&n| betas_from_chebyshev(&index, wavelength_um, (from, to), n, order))
            .collect::<Result<Vec<_>>>()?;
        // each β_k from the pair of successive interpolants that agree best: more points
        // shrink the interpolation's error but magnify the samples' round-off in a k-th
        // derivative, roughly as the points' number to the 2k-th power
        let mut betas = Vec::with_capacity(order);
        let mut errors = Vec::with_capacity(order);
        for k in 0..order {
            let (b, e) = fits
                .windows(2)
                .map(|w| (w[1][k], (w[1][k] - w[0][k]).abs()))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap_or((fits[fits.len() - 1][k], f64::INFINITY));
            betas.push(b);
            errors.push(e);
        }
        Ok(Dispersion {
            wavelength: wavelength_um,
            betas,
            errors,
        })
    }

    /// From a mode's effective index over wavelength ([`ModeIndex`], the mode solver's): as
    /// [`Dispersion::of_index`] on the interpolant across its whole range, each error the larger
    /// of that fit's and the change from the interpolant on half the mode's points
    /// ([`ModeIndex::coarse`]), which bounds what the solver's samples leave in each β_k.
    ///
    /// # Errors
    ///
    /// As [`Dispersion::of_index`]; and [`Error::InvalidValue`] for an interpolant on fewer than
    /// 5 points.
    pub fn of_mode(mode: &ModeIndex, wavelength_um: f64, order: usize) -> Result<Dispersion> {
        let range = mode.range();
        let fine = Dispersion::of_index(|l| Ok(mode.n_eff(l)), wavelength_um, range, order)?;
        let coarse_mode = mode.coarse().ok_or_else(|| {
            Error::invalid(
                "dispersion",
                "the mode's interpolant has fewer than 5 points",
            )
        })?;
        let coarse =
            Dispersion::of_index(|l| Ok(coarse_mode.n_eff(l)), wavelength_um, range, order)?;
        let errors = fine
            .betas
            .iter()
            .zip(&coarse.betas)
            .zip(&fine.errors)
            .map(|((a, b), e)| (a - b).abs().max(*e))
            .collect();
        Ok(Dispersion { errors, ..fine })
    }

    /// The wavelength, µm.
    pub fn wavelength(&self) -> f64 {
        self.wavelength
    }

    /// ω₀, rad/ps.
    pub fn omega(&self) -> f64 {
        angular_frequency(self.wavelength)
    }

    /// The highest order held.
    pub fn order(&self) -> usize {
        self.betas.len()
    }

    /// β_k (ps^k/m; β₁ in ps/m), 0 above the order held.
    pub fn beta(&self, k: usize) -> f64 {
        if k == 0 {
            return f64::NAN;
        }
        self.betas.get(k - 1).copied().unwrap_or(0.0)
    }

    /// β_k's estimated error, the same units.
    pub fn error(&self, k: usize) -> f64 {
        if k == 0 {
            return f64::NAN;
        }
        self.errors.get(k - 1).copied().unwrap_or(0.0)
    }

    /// The group index c·β₁.
    pub fn group_index(&self) -> f64 {
        self.beta(1) * C_M_PER_PS
    }

    /// Σ_{k≥2} β_k Ω^k/k!, rad/m: the dispersive part of β(ω₀ + Ω) − β₀ − β₁Ω.
    pub fn phase(&self, omega: f64) -> f64 {
        let mut sum = 0.0;
        let mut term = omega; // Ω^k/k!
        for (k, b) in self.betas.iter().enumerate().skip(1) {
            term *= omega / (k + 1) as f64;
            sum += b * term;
        }
        sum
    }

    /// The linear mismatch of degenerate four-wave mixing with signal and idler at ω₀ ± Ω,
    /// Δk = β(ω₀+Ω) + β(ω₀−Ω) − 2β(ω₀) = 2 Σ_m β_{2m}Ω^{2m}/(2m)! (Agrawal Eq. (10.4.12),
    /// whose sign this is; Eq. (10.3.6) is its first two terms), rad/m.
    pub fn four_wave_mismatch(&self, omega: f64) -> f64 {
        self.phase(omega) + self.phase(-omega)
    }
}

/// β_1 … β_order from the Chebyshev interpolant of n(ω) on `points` points.
fn betas_from_chebyshev(
    index: &impl Fn(f64) -> Result<f64>,
    wavelength_um: f64,
    (from, to): (f64, f64),
    points: usize,
    order: usize,
) -> Result<Vec<f64>> {
    let (lo, hi) = (angular_frequency(to), angular_frequency(from));
    let (mid, half) = (0.5 * (lo + hi), 0.5 * (hi - lo));
    let n = points - 1;
    let samples = (0..=n)
        .map(|j| {
            let w = mid + half * (PI * j as f64 / n as f64).cos();
            let v = index(wavelength_of(w))?;
            if v.is_finite() {
                Ok(v)
            } else {
                Err(Error::invalid(
                    "dispersion",
                    format!("the index at {} um isn't finite", wavelength_of(w)),
                ))
            }
        })
        .collect::<Result<Vec<f64>>>()?;
    let w0 = angular_frequency(wavelength_um);
    let x = (w0 - mid) / half;
    // n^(k)(ω₀), k = 0 … order
    let mut derivatives = Vec::with_capacity(order + 1);
    let mut c = chebyshev_coefficients(&samples);
    for k in 0..=order {
        derivatives.push(clenshaw(&c, x) / half.powi(k as i32));
        c = chebyshev_derivative(&c);
    }
    Ok((1..=order)
        .map(|k| (w0 * derivatives[k] + k as f64 * derivatives[k - 1]) / C_M_PER_PS)
        .collect())
}

// ---------------------------------------------------------------------------------------------
// The guide and its pulses
// ---------------------------------------------------------------------------------------------

/// The delayed Raman response h_R(T) = (τ₁⁻² + τ₂⁻²) τ₁ e^{−T/τ₂} sin(T/τ₁), a fraction f_R of
/// the Kerr response (Agrawal Eqs. (2.3.33) and (2.3.39)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Raman {
    /// f_R.
    pub fraction: f64,
    /// τ₁, ps.
    pub tau1: f64,
    /// τ₂, ps.
    pub tau2: f64,
}

impl Raman {
    /// Silica's: τ₁ = 12.2 fs, τ₂ = 32 fs, f_R = 0.18 (Agrawal, p. 42).
    pub const SILICA: Raman = Raman {
        fraction: 0.18,
        tau1: 0.0122,
        tau2: 0.032,
    };

    /// h̃_R(Ω) = ∫ h_R(T) e^{iΩT} dT = (τ₁⁻² + τ₂⁻²) / ((τ₂⁻¹ − iΩ)² + τ₁⁻²); h̃_R(0) = 1.
    pub fn response(&self, omega: f64) -> c64 {
        let (a, b) = (1.0 / self.tau1, 1.0 / self.tau2);
        let d = c64::new(b, -omega);
        c64::new(a * a + b * b, 0.0) / (d * d + a * a)
    }

    /// T_R = f_R ∫ T h_R(T) dT, ps (Agrawal Eq. (2.3.42)): the integral of this h_R is
    /// 2τ₁²τ₂/(τ₁² + τ₂²), so T_R = 1.46 fs for silica. Agrawal's Eq. (2.3.45) prints
    /// f_Rτ₁τ₂²/(τ₁² + τ₂²), 1.92 fs, for the same integral; his "about 2 fs" is near both.
    /// Only the response's transform, [`Raman::response`], enters the propagation.
    pub fn first_moment(&self) -> f64 {
        let (t1, t2) = (self.tau1, self.tau2);
        self.fraction * 2.0 * t1 * t1 * t2 / (t1 * t1 + t2 * t2)
    }
}

/// A Kerr waveguide for pulses: its dispersion, γ and loss, and the optional higher-order
/// nonlinear terms.
#[derive(Clone, Debug, PartialEq)]
pub struct Guide {
    /// β₂, β₃, … at the carrier.
    pub dispersion: Dispersion,
    /// γ, 1/(W·m).
    pub gamma: f64,
    /// α, the power loss, 1/m.
    pub loss: f64,
    /// Self-steepening, γ₁ = γ/ω₀ (Agrawal, p. 44).
    pub self_steepening: bool,
    /// The delayed Raman response.
    pub raman: Option<Raman>,
}

impl Guide {
    /// The NLSE's guide: `dispersion`, γ (1/(W·m)) and the power loss α (1/m), without Raman or
    /// self-steepening.
    pub fn new(dispersion: Dispersion, gamma: f64, loss: f64) -> Guide {
        Guide {
            dispersion,
            gamma,
            loss,
            self_steepening: false,
            raman: None,
        }
    }

    /// The same guide with self-steepening.
    pub fn with_self_steepening(self) -> Guide {
        Guide {
            self_steepening: true,
            ..self
        }
    }

    /// The same guide with a delayed Raman response.
    pub fn with_raman(self, raman: Raman) -> Guide {
        Guide {
            raman: Some(raman),
            ..self
        }
    }

    /// The pulse after `length` m on `steps` split steps.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a length that isn't finite and non-negative, no steps, or a γ
    /// or loss that isn't finite.
    pub fn propagate(&self, pulse: &Pulse, length: f64, steps: usize) -> Result<Pulse> {
        Ok(self
            .evolution(pulse, length, steps, 1)?
            .pop()
            .unwrap_or_else(|| pulse.clone()))
    }

    /// The pulse at `snapshots` points equally spaced to `length` (the last at `length`), on
    /// `steps` split steps, a multiple of `snapshots`.
    ///
    /// # Errors
    ///
    /// As [`Guide::propagate`], and [`Error::InvalidValue`] for steps that aren't a multiple
    /// of the snapshots.
    pub fn evolution(
        &self,
        pulse: &Pulse,
        length: f64,
        steps: usize,
        snapshots: usize,
    ) -> Result<Vec<Pulse>> {
        if !(length.is_finite() && length >= 0.0)
            || steps == 0
            || snapshots == 0
            || !steps.is_multiple_of(snapshots)
            || !(self.gamma.is_finite() && self.loss.is_finite())
        {
            return Err(Error::invalid(
                "propagation",
                format!(
                    "needs a finite length ≥ 0, finite γ and loss, and steps a multiple of the snapshots, got {length} m, {steps} steps, {snapshots} snapshots"
                ),
            ));
        }
        let n = pulse.field.len();
        let fft = Fft::new(n)?;
        let h = length / steps as f64;
        let omegas = pulse.frequencies();
        let linear = |dz: f64| -> Vec<c64> {
            omegas
                .iter()
                .map(|&w| c64::new(-0.5 * self.loss * dz, self.dispersion.phase(w) * dz).exp())
                .collect()
        };
        let (half, full) = (linear(0.5 * h), linear(h));
        let nonlinear = Nonlinear::new(self, &omegas, &fft);
        let mut a = pulse.field.clone();
        let mut out = Vec::with_capacity(snapshots);
        let per = steps / snapshots;
        apply_linear(&fft, &mut a, &half);
        for s in 0..steps {
            nonlinear.step(&mut a, h);
            let last = (s + 1) % per == 0;
            apply_linear(&fft, &mut a, if last { &half } else { &full });
            if last {
                out.push(Pulse {
                    step: pulse.step,
                    field: a.clone(),
                });
                if s + 1 < steps {
                    apply_linear(&fft, &mut a, &half);
                }
            }
        }
        Ok(out)
    }
}

/// Multiplies the spectrum of `a` by `factor`, Ω in [`Pulse::frequencies`]'s order.
fn apply_linear(fft: &Fft, a: &mut [c64], factor: &[c64]) {
    // components e^{−iΩT}: the inverse transform's sum carries e^{+2πijk/n}
    fft.inverse(a);
    for (v, f) in a.iter_mut().zip(factor) {
        *v *= f;
    }
    fft.forward(a);
}

/// The nonlinear step: exact for the Kerr term alone, RK4 otherwise.
struct Nonlinear<'a> {
    gamma: f64,
    fft: &'a Fft,
    /// 1 + Ω/ω₀ with self-steepening, else None
    shock: Option<Vec<f64>>,
    /// f_R and h̃_R(Ω), with Raman
    raman: Option<(f64, Vec<c64>)>,
}

impl<'a> Nonlinear<'a> {
    fn new(guide: &Guide, omegas: &[f64], fft: &'a Fft) -> Nonlinear<'a> {
        let w0 = guide.dispersion.omega();
        Nonlinear {
            gamma: guide.gamma,
            fft,
            shock: guide
                .self_steepening
                .then(|| omegas.iter().map(|w| 1.0 + w / w0).collect()),
            raman: guide
                .raman
                .map(|r| (r.fraction, omegas.iter().map(|&w| r.response(w)).collect())),
        }
    }

    fn step(&self, a: &mut [c64], h: f64) {
        if self.shock.is_none() && self.raman.is_none() {
            for v in a.iter_mut() {
                *v *= c64::new(0.0, self.gamma * v.norm_sqr() * h).exp();
            }
            return;
        }
        // classical RK4 on dA/dz = N(A)
        let k1 = self.rate(a);
        let at = |k: &[c64], s: f64| -> Vec<c64> {
            a.iter().zip(k).map(|(v, d)| v + d * (s * h)).collect()
        };
        let k2 = self.rate(&at(&k1, 0.5));
        let k3 = self.rate(&at(&k2, 0.5));
        let k4 = self.rate(&at(&k3, 1.0));
        for (j, v) in a.iter_mut().enumerate() {
            *v += (k1[j] + 2.0 * k2[j] + 2.0 * k3[j] + k4[j]) * (h / 6.0);
        }
    }

    /// N(A) = iγ (1 + (i/ω₀)∂_T)[A ((1 − f_R)|A|² + f_R h_R ⊗ |A|²)].
    fn rate(&self, a: &[c64]) -> Vec<c64> {
        let intensity: Vec<f64> = a.iter().map(|v| v.norm_sqr()).collect();
        let response: Vec<f64> = match &self.raman {
            None => intensity,
            Some((fr, h)) => {
                let mut s: Vec<c64> = intensity.iter().map(|&p| c64::new(p, 0.0)).collect();
                self.fft.inverse(&mut s);
                for (v, hv) in s.iter_mut().zip(h) {
                    *v *= hv;
                }
                self.fft.forward(&mut s);
                intensity
                    .iter()
                    .zip(&s)
                    .map(|(p, conv)| (1.0 - fr) * p + fr * conv.re)
                    .collect()
            }
        };
        let mut b: Vec<c64> = a.iter().zip(&response).map(|(v, r)| v * *r).collect();
        if let Some(shock) = &self.shock {
            self.fft.inverse(&mut b);
            for (v, s) in b.iter_mut().zip(shock) {
                *v *= s;
            }
            self.fft.forward(&mut b);
        }
        let i_gamma = c64::new(0.0, self.gamma);
        b.iter().map(|v| v * i_gamma).collect()
    }
}

/// A pulse on its time grid: T_j = (j − n/2)·δT, n a power of two.
#[derive(Clone, Debug, PartialEq)]
pub struct Pulse {
    /// δT, ps.
    pub step: f64,
    /// A(T_j), √W.
    pub field: Vec<c64>,
}

impl Pulse {
    /// A(T) sampled on `points` (a power of two) across `window` ps, centred on T = 0.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a point count that isn't a power of two of at least 4 or a
    /// window that isn't positive.
    pub fn sampled(points: usize, window: f64, amplitude: impl Fn(f64) -> c64) -> Result<Pulse> {
        if points < 4 || !points.is_power_of_two() || !(window.is_finite() && window > 0.0) {
            return Err(Error::invalid(
                "pulse",
                format!(
                    "needs a power of two of at least 4 points and a positive window, got {points}, {window} ps"
                ),
            ));
        }
        let step = window / points as f64;
        let field = (0..points)
            .map(|j| amplitude((j as f64 - (points / 2) as f64) * step))
            .collect();
        Ok(Pulse { step, field })
    }

    /// √P₀ sech(T/T₀): `peak` W, `t0` ps.
    ///
    /// # Errors
    ///
    /// As [`Pulse::sampled`].
    pub fn sech(peak: f64, t0: f64, points: usize, window: f64) -> Result<Pulse> {
        let a = peak.sqrt();
        Pulse::sampled(points, window, |t| c64::new(a / (t / t0).cosh(), 0.0))
    }

    /// √P₀ exp(−T²/2T₀²) (Agrawal Eq. (3.2.4)): `peak` W, `t0` ps.
    ///
    /// # Errors
    ///
    /// As [`Pulse::sampled`].
    pub fn gaussian(peak: f64, t0: f64, points: usize, window: f64) -> Result<Pulse> {
        let a = peak.sqrt();
        Pulse::sampled(points, window, |t| {
            c64::new(a * (-0.5 * (t / t0).powi(2)).exp(), 0.0)
        })
    }

    /// The times T_j, ps.
    pub fn times(&self) -> Vec<f64> {
        let n = self.field.len();
        (0..n)
            .map(|j| (j as f64 - (n / 2) as f64) * self.step)
            .collect()
    }

    /// The angular frequencies Ω_k (rad/ps) of [`Pulse::spectrum`]: 2πk/(nδT), k = 0 … n/2 − 1,
    /// then −n/2 … −1.
    pub fn frequencies(&self) -> Vec<f64> {
        let n = self.field.len();
        let d = TAU / (n as f64 * self.step);
        (0..n)
            .map(|k| {
                let s = if k < n / 2 {
                    k as f64
                } else {
                    k as f64 - n as f64
                };
                s * d
            })
            .collect()
    }

    /// Ã(Ω_k) = δT Σ_j A(T_j) e^{iΩ_kT_j} (Agrawal Eq. (2.4.12)), √W·ps.
    pub fn spectrum(&self) -> Vec<c64> {
        let n = self.field.len();
        let fft = match Fft::new(n) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };
        let mut s = self.field.clone();
        fft.inverse(&mut s);
        // T_j = (j − n/2)δT: e^{iΩ_k T_j} = e^{2πijk/n}·(−1)^k
        s.iter()
            .enumerate()
            .map(|(k, v)| {
                let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
                v * (sign * self.step * n as f64)
            })
            .collect()
    }

    /// |A(T_j)|², W.
    pub fn power(&self) -> Vec<f64> {
        self.field.iter().map(|v| v.norm_sqr()).collect()
    }

    /// The peak power, W.
    pub fn peak_power(&self) -> f64 {
        self.power().into_iter().fold(0.0, f64::max)
    }

    /// The energy ∫|A|² dT, pJ (W·ps).
    pub fn energy(&self) -> f64 {
        self.power().iter().sum::<f64>() * self.step
    }

    /// The RMS spectral width Δω_rms = (⟨Ω²⟩ − ⟨Ω⟩²)^½ over |Ã(Ω)|², rad/ps (Agrawal Eqs.
    /// (4.1.14)–(4.1.15)).
    pub fn rms_bandwidth(&self) -> f64 {
        let s = self.spectrum();
        let w = self.frequencies();
        let weights: Vec<f64> = s.iter().map(|v| v.norm_sqr()).collect();
        let total: f64 = weights.iter().sum();
        let m1: f64 = weights.iter().zip(&w).map(|(p, x)| p * x).sum::<f64>() / total;
        let m2: f64 = weights.iter().zip(&w).map(|(p, x)| p * x * x).sum::<f64>() / total;
        (m2 - m1 * m1).max(0.0).sqrt()
    }

    /// Σ_k |Ã(Ω_k)|²/(ω₀ + Ω_k), proportional to the number of photons (Agrawal, p. 43), for a
    /// carrier at ω₀ (rad/ps).
    pub fn photons(&self, omega0: f64) -> f64 {
        self.spectrum()
            .iter()
            .zip(self.frequencies())
            .map(|(v, w)| v.norm_sqr() / (omega0 + w))
            .sum()
    }
}

// ---------------------------------------------------------------------------------------------
// Closed forms
// ---------------------------------------------------------------------------------------------

/// The dispersion length L_D = T₀²/|β₂|, m (Agrawal Eq. (3.1.5)).
pub fn dispersion_length(t0: f64, beta2: f64) -> f64 {
    t0 * t0 / beta2.abs()
}

/// The nonlinear length L_NL = 1/(γP₀), m (Eq. (3.1.5)).
pub fn nonlinear_length(gamma: f64, peak: f64) -> f64 {
    1.0 / (gamma * peak)
}

/// The soliton order N = (γP₀T₀²/|β₂|)^½ (Eq. (5.2.3)).
pub fn soliton_order(gamma: f64, peak: f64, t0: f64, beta2: f64) -> f64 {
    (gamma * peak * t0 * t0 / beta2.abs()).sqrt()
}

/// The fundamental soliton's peak power P₀ = |β₂|/(γT₀²), W (Eq. (5.2.20)).
pub fn soliton_power(gamma: f64, t0: f64, beta2: f64) -> f64 {
    beta2.abs() / (gamma * t0 * t0)
}

/// The soliton period z₀ = (π/2) L_D, m (Eq. (5.2.24)).
pub fn soliton_period(t0: f64, beta2: f64) -> f64 {
    0.5 * PI * dispersion_length(t0, beta2)
}

/// The effective length L_eff = (1 − e^{−αL})/α, m (Eq. (4.1.5)); L without loss.
pub fn effective_length(loss: f64, length: f64) -> f64 {
    if loss == 0.0 {
        length
    } else {
        -(-loss * length).exp_m1() / loss
    }
}

/// The largest self-phase-modulation phase φ_max = γP₀L_eff, rad (Eq. (4.1.6)).
pub fn spm_peak_phase(gamma: f64, peak: f64, loss: f64, length: f64) -> f64 {
    gamma * peak * effective_length(loss, length)
}

/// A Gaussian pulse's RMS spectral broadening by SPM alone,
/// Δω_rms/Δω₀ = (1 + (4/(3√3)) φ_max²)^½ (Eq. (4.1.16)).
pub fn spm_rms_broadening(phi_max: f64) -> f64 {
    (1.0 + 4.0 / (3.0 * 3f64.sqrt()) * phi_max * phi_max).sqrt()
}

/// The second-order soliton in normalized units (ξ = z/L_D, τ = T/T₀, u = N U, Eq. (5.2.5)):
/// u = 4[cosh 3τ + 3 e^{4iξ} cosh τ] e^{iξ/2} / [cosh 4τ + 4 cosh 2τ + 3 cos 4ξ] (Eq. (5.2.23)).
pub fn second_order_soliton(xi: f64, tau: f64) -> c64 {
    let num =
        c64::new((3.0 * tau).cosh(), 0.0) + c64::new(0.0, 4.0 * xi).exp() * (3.0 * tau.cosh());
    let den = (4.0 * tau).cosh() + 4.0 * (2.0 * tau).cosh() + 3.0 * (4.0 * xi).cos();
    num * c64::new(0.0, 0.5 * xi).exp() * (4.0 / den)
}

#[cfg(test)]
#[path = "pulse_tests.rs"]
mod tests;
