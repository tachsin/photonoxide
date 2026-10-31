//! Kerr microresonators: the Lugiato–Lefever equation, its homogeneous steady states, their
//! modulation instability, and dissipative Kerr solitons, fed by a guide's dispersion.
//!
//! - L. A. Lugiato, R. Lefever, Phys. Rev. Lett. 58, 2209 (1987), doi:10.1103/PhysRevLett.58.2209
//!   (Eqs. (1), (2), (4)–(6)): the equation, its cubic steady state and the instability.
//! - S. Coen, H. G. Randle, T. Sylvestre, M. Erkintalo, Opt. Lett. 38, 37 (2012),
//!   doi:10.1364/OL.38.000037 (Eq. (3)): the mean-field equation of a ring from its waveguide's
//!   β_k and γ.
//! - T. Herr et al., Nat. Photonics 8, 145 (2013), doi:10.1038/nphoton.2013.343
//!   (Eqs. (1)–(2)): the soliton's sech and its width.
//! - T. J. Kippenberg, A. L. Gaeta, M. Lipson, M. L. Gorodetsky, Science 361, eaan8083 (2018),
//!   doi:10.1126/science.aan8083 (Eqs. (1)–(3)): the equation in the resonator's rates and its
//!   dimensionless form.
//!
//! # Conventions
//!
//! - **The equation**, dimensionless (Kippenberg et al.'s Eq. (3), rearranged):
//!   ∂ψ/∂τ = −(1 + iζ₀)ψ + i|ψ|²ψ + i ½ ∂²ψ/∂θ² + f, with higher orders of dispersion as each
//!   mode's own detuning (below). Time τ is in units of twice the photon lifetime (τ = κt/2), ζ₀
//!   = 2Δ/κ is the pump's detuning from the cold resonance in half linewidths (positive: the
//!   pump redder), f the drive, real and positive, and |ψ|² the power in units of the one whose
//!   Kerr shift is half a linewidth. It is Lugiato and Lefever's Eq. (1) with E = ψ, E_I = f,
//!   θ = ζ₀, η = +1 (self-focusing) and a = ½.
//! - **Modes.** ψ is sampled at n points (a power of two) around the ring, each spectral
//!   component e^{−iΩT} a comb line μ (Ω = μ·2π/t_R; μ > 0 bluer, as in [`super::pulse`]). In
//!   mode μ the dispersion is a detuning δ_μ: dψ_μ/dτ = −(1 + i(ζ₀ + δ_μ))ψ_μ + i[|ψ|²ψ]_μ +
//!   f δ_{μ0}, δ_μ = 2D_int(μ)/κ with D_int(μ) = D₂μ²/2 + D₃μ³/6 + … the resonances' departure
//!   from the comb (Kippenberg et al., Eq. (1)). With D₂ alone, δ_μ = d₂μ², d₂ = D₂/κ.
//! - **A ring from its waveguide** (Coen et al.'s Eq. (3)): round trip L (m) and time t_R,
//!   total loss α = α_i + θ/2 per round trip (of the field), coupling θ (of the power), detuning
//!   δ₀ (rad per round trip), the waveguide's γ and β_k, pump P_in. Then ζ₀ = δ₀/α,
//!   f² = θγLP_in/α³, |E|² = (α/γL)|ψ|², τ = αt/t_R, and δ_μ = −(L/α) Σ_{k≥2} β_kΩ_μ^k/k!.
//! - **Split-step.** exp(hL/2) exp(hN) exp(hL/2), the half steps merged between steps: the
//!   linear part L (loss, detuning, dispersion and the drive) solved exactly in each mode,
//!   ψ_μ(h) = e^{L_μh}ψ_μ + f(e^{L₀h} − 1)/L₀ in mode 0; the Kerr part exactly,
//!   ψ → ψ e^{i|ψ|²h}. Second order in h, its steady states too; the validation cases record
//!   the order. Keep the largest mode's δ_μh below π: above it the scheme's own instability
//!   grows the outermost modes (Coen et al.'s soliton on 1024 modes settles at h = 10⁻⁴, not at
//!   4 × 10⁻⁴). The transforms are this module's own radix-2 FFT, on one thread.

use std::f64::consts::PI;

use num_complex::Complex64 as c64;

use super::fft::Fft;
use super::pulse::{C_M_PER_PS, Dispersion};
use crate::{Error, Result};

/// A Kerr resonator in the dimensionless Lugiato–Lefever equation.
#[derive(Clone, Debug, PartialEq)]
pub struct Resonator {
    detuning: f64,
    drive: f64,
    /// δ_μ per spectral bin, in [`Resonator::modes`]' order
    dispersion: Vec<f64>,
}

impl Resonator {
    /// The resonator with detuning ζ₀, drive f ≥ 0 and each mode's detuning δ_μ =
    /// `dispersion(μ)`, on `points` (a power of two of at least 4) modes μ = −n/2 … n/2 − 1.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a point count that isn't a power of two of at least 4, a
    /// detuning or drive that isn't finite, a negative drive, or a δ_μ that isn't finite.
    pub fn new(
        detuning: f64,
        drive: f64,
        dispersion: impl Fn(i64) -> f64,
        points: usize,
    ) -> Result<Resonator> {
        if points < 4 || !points.is_power_of_two() {
            return Err(Error::invalid(
                "resonator",
                format!("needs a power of two of at least 4 modes, got {points}"),
            ));
        }
        if !(detuning.is_finite() && drive.is_finite() && drive >= 0.0) {
            return Err(Error::invalid(
                "resonator",
                format!("needs a finite detuning and a finite drive ≥ 0, got {detuning}, {drive}"),
            ));
        }
        let dispersion: Vec<f64> = mode_numbers(points).map(dispersion).collect();
        if dispersion.iter().any(|d| !d.is_finite()) {
            return Err(Error::invalid("resonator", "a mode's δ_μ isn't finite"));
        }
        Ok(Resonator {
            detuning,
            drive,
            dispersion,
        })
    }

    /// With D₂ alone: δ_μ = d₂μ², d₂ = D₂/κ (> 0 for anomalous dispersion).
    ///
    /// # Errors
    ///
    /// As [`Resonator::new`].
    pub fn quadratic(detuning: f64, drive: f64, d2: f64, points: usize) -> Result<Resonator> {
        Resonator::new(detuning, drive, |m| d2 * (m * m) as f64, points)
    }

    /// ζ₀.
    pub fn detuning(&self) -> f64 {
        self.detuning
    }

    /// f.
    pub fn drive(&self) -> f64 {
        self.drive
    }

    /// The number of modes n.
    pub fn points(&self) -> usize {
        self.dispersion.len()
    }

    /// The mode numbers μ of the spectral bins: 0 … n/2 − 1, then −n/2 … −1.
    pub fn modes(&self) -> Vec<i64> {
        mode_numbers(self.points()).collect()
    }

    /// The same resonator at another detuning.
    pub fn with_detuning(&self, detuning: f64) -> Resonator {
        Resonator {
            detuning,
            ..self.clone()
        }
    }

    /// The same resonator with another drive.
    pub fn with_drive(&self, drive: f64) -> Resonator {
        Resonator {
            drive,
            ..self.clone()
        }
    }

    /// The field after `time` (in τ) from `field` (n points around the ring), on `steps` split
    /// steps.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a field of other than n points, a time that isn't finite and
    /// non-negative, or no steps.
    pub fn evolve(&self, field: &[c64], time: f64, steps: usize) -> Result<Vec<c64>> {
        let n = self.points();
        if field.len() != n || !(time.is_finite() && time >= 0.0) || steps == 0 {
            return Err(Error::invalid(
                "resonator",
                format!(
                    "needs a field of {n} points, a finite time ≥ 0 and steps, got {} points, {time}, {steps}",
                    field.len()
                ),
            ));
        }
        let fft = Fft::new(n)?;
        let h = time / steps as f64;
        let (half, full) = (self.linear(0.5 * h), self.linear(h));
        let mut psi = field.to_vec();
        apply(&fft, &mut psi, &half);
        for s in 0..steps {
            for v in psi.iter_mut() {
                *v *= c64::new(0.0, v.norm_sqr() * h).exp();
            }
            apply(&fft, &mut psi, if s + 1 == steps { &half } else { &full });
        }
        Ok(psi)
    }

    /// The field after integrating from `field` until it settles: in chunks of `chunk` (in τ)
    /// on steps of `step`, until the largest change of |ψ|² over a chunk, per unit τ, is below
    /// `tolerance` times the peak of |ψ|², or `limit` (in τ) is reached. Returns the field and
    /// the time taken.
    ///
    /// # Errors
    ///
    /// As [`Resonator::evolve`]; and [`Error::InvalidValue`] when it doesn't settle by `limit`.
    pub fn settle(
        &self,
        field: &[c64],
        step: f64,
        chunk: f64,
        tolerance: f64,
        limit: f64,
    ) -> Result<(Vec<c64>, f64)> {
        let steps = ((chunk / step).round() as usize).max(1);
        let mut psi = field.to_vec();
        let mut t = 0.0;
        while t < limit {
            let next = self.evolve(&psi, chunk, steps)?;
            t += chunk;
            let peak = next.iter().map(|v| v.norm_sqr()).fold(0.0, f64::max);
            let change = next
                .iter()
                .zip(&psi)
                .map(|(a, b)| (a.norm_sqr() - b.norm_sqr()).abs())
                .fold(0.0, f64::max)
                / chunk;
            psi = next;
            if change <= tolerance * peak.max(f64::MIN_POSITIVE) {
                return Ok((psi, t));
            }
        }
        Err(Error::invalid(
            "resonator",
            format!("the field didn't settle to {tolerance:e} within τ = {limit}"),
        ))
    }

    /// exp(L_μ h) per mode, and the drive's term in mode 0.
    fn linear(&self, h: f64) -> Linear {
        let factors = self
            .dispersion
            .iter()
            .map(|d| (c64::new(-1.0, -(self.detuning + d)) * h).exp())
            .collect::<Vec<c64>>();
        let l0 = c64::new(-1.0, -(self.detuning + self.dispersion[0]));
        let source = (factors[0] - 1.0) / l0 * self.drive;
        Linear { factors, source }
    }

    /// The homogeneous steady states' intracavity powers X = |ψ|², the real positive roots of
    /// f² = X(1 + (X − ζ₀)²) (Lugiato and Lefever's Eq. (2)), ascending: one, or three where
    /// the response is bistable (two of them coinciding at a turning point).
    pub fn homogeneous(&self) -> Vec<f64> {
        homogeneous_powers(self.detuning, self.drive)
    }

    /// The homogeneous steady field of power X: ψ_s = f/(1 + i(ζ₀ − X)) (∂ψ/∂τ = 0 with ψ
    /// uniform).
    pub fn homogeneous_field(&self, power: f64) -> c64 {
        c64::new(self.drive, 0.0) / c64::new(1.0, self.detuning - power)
    }

    /// The growth rate (in 1/τ) of a perturbation of the homogeneous state of power X in the
    /// modes ±μ: the larger root of λ² + 2λ + ζ = 0, λ = −1 + (1 − ζ)^½ (its real part), with
    /// Lugiato and Lefever's Eq. (4), ζ = 1 + (X − ζ₀)(3X − ζ₀) + a(a − 2(2X − ζ₀)), a the
    /// mode's detuning δ_μ (their a(n), η = 1). Positive where the state is unstable (their Eqs.
    /// (5)–(6)). The pair ±μ must have the same δ (even dispersion).
    pub fn instability_rate(&self, power: f64, mode: i64) -> f64 {
        let a = self.dispersion[bin(mode, self.points())];
        instability_rate(self.detuning, power, a)
    }
}

/// exp(L_μh) per mode and the drive's term.
struct Linear {
    factors: Vec<c64>,
    source: c64,
}

/// Steps the linear part: to the modes, each multiplied, the drive added to mode 0, and back.
fn apply(fft: &Fft, psi: &mut [c64], linear: &Linear) {
    // components e^{−iΩT}, as in super::pulse: the inverse transform's coefficients, 1/n
    // included, so bin 0 is the mean of ψ
    fft.inverse(psi);
    for (v, f) in psi.iter_mut().zip(&linear.factors) {
        *v *= f;
    }
    psi[0] += linear.source;
    fft.forward(psi);
}

fn mode_numbers(n: usize) -> impl Iterator<Item = i64> {
    let half = (n / 2) as i64;
    (0..n as i64).map(move |k| if k < half { k } else { k - n as i64 })
}

fn bin(mode: i64, n: usize) -> usize {
    mode.rem_euclid(n as i64) as usize
}

/// The real positive roots X of f² = X(1 + (X − ζ₀)²), ascending (Lugiato and Lefever's Eq.
/// (2)).
pub fn homogeneous_powers(detuning: f64, drive: f64) -> Vec<f64> {
    // X³ − 2ζX² + (1 + ζ²)X − f² = 0
    let (b, c, d) = (-2.0 * detuning, 1.0 + detuning * detuning, -drive * drive);
    let mut roots = real_cubic_roots(b, c, d);
    // polish each by Newton's method on the cubic
    for x in roots.iter_mut() {
        for _ in 0..3 {
            let p = ((*x + b) * *x + c) * *x + d;
            let dp = (3.0 * *x + 2.0 * b) * *x + c;
            if dp != 0.0 {
                *x -= p / dp;
            }
        }
    }
    roots.retain(|x| *x >= 0.0);
    roots.sort_by(f64::total_cmp);
    roots
}

/// The real roots of x³ + bx² + cx + d.
fn real_cubic_roots(b: f64, c: f64, d: f64) -> Vec<f64> {
    let shift = b / 3.0;
    let p = c - b * b / 3.0;
    let q = 2.0 * b * b * b / 27.0 - b * c / 3.0 + d;
    let disc = (q / 2.0).powi(2) + (p / 3.0).powi(3);
    if disc > 0.0 {
        let s = disc.sqrt();
        vec![(-q / 2.0 + s).cbrt() + (-q / 2.0 - s).cbrt() - shift]
    } else {
        let r = (-p / 3.0).sqrt();
        let phi = if r == 0.0 {
            0.0
        } else {
            (-q / (2.0 * r * r * r)).clamp(-1.0, 1.0).acos()
        };
        (0..3)
            .map(|k| 2.0 * r * ((phi + 2.0 * PI * k as f64) / 3.0).cos() - shift)
            .collect()
    }
}

/// The powers X at the turning points of the homogeneous response, where df²/dX = 0:
/// X = (2ζ₀ ∓ (ζ₀² − 3)^½)/3, which exist for ζ₀ > √3 (Lugiato and Lefever, p. 2209: the
/// response is S shaped, bistable, above θ_c = √3).
pub fn bistable_range(detuning: f64) -> Option<(f64, f64)> {
    let s = detuning * detuning - 3.0;
    (detuning > 0.0 && s > 0.0).then(|| {
        let r = s.sqrt();
        ((2.0 * detuning - r) / 3.0, (2.0 * detuning + r) / 3.0)
    })
}

/// The drive f whose homogeneous state has the power X: f² = X(1 + (X − ζ₀)²).
pub fn drive_for(detuning: f64, power: f64) -> f64 {
    (power * (1.0 + (power - detuning).powi(2))).sqrt()
}

/// λ = −1 + Re(1 − ζ)^½ with Lugiato and Lefever's Eq. (4) for a mode of detuning `a`.
pub fn instability_rate(detuning: f64, power: f64, a: f64) -> f64 {
    let (x, t) = (power, detuning);
    let zeta = 1.0 + (x - t) * (3.0 * x - t) + a * (a - 2.0 * (2.0 * x - t));
    let s = 1.0 - zeta;
    -1.0 + if s > 0.0 { s.sqrt() } else { 0.0 }
}

/// The approximate single soliton of a resonator with D₂ alone (δ_μ = d₂μ²), on its
/// homogeneous background: ψ ≈ ψ_b + B sech(θ/W) e^{iφ₀}.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Soliton {
    /// The peak amplitude B = (2ζ₀)^½ above the background; |ψ|² peaks near 2ζ₀.
    pub amplitude: f64,
    /// The width W in the angle around the ring, rad: W = (d₂/ζ₀)^½ (Herr et al.'s Eq. (1),
    /// sech((2(ω₀ − ω_p)/D₂)^½ φ)).
    pub width: f64,
    /// cos φ₀ = (8ζ₀)^½/(πf), from the balance of loss and drive over the soliton.
    pub cos_phase: f64,
    /// The background: the lower homogeneous state's field.
    pub background: c64,
}

impl Resonator {
    /// The approximate single soliton (D₂ alone, `d2` = D₂/κ): B sech(φ/W) e^{iφ₀} on the lower
    /// homogeneous state. B and W make it the stationary soliton of the lossless, undriven
    /// equation, i ∂ψ/∂τ + ½ψ_θθ + |ψ|²ψ = ζ₀ψ (Kippenberg et al.'s Eq. (3) with its right side
    /// but ζ₀ψ dropped, whose soliton is Agrawal's Eq. (5.2.15) with η² = 2ζ₀), W the width
    /// Herr et al.'s Eq. (1) prints. φ₀ is ours: the drive must make up the loss of the
    /// soliton's energy, 2∫|ψ|² = 2Re ∫ f ψ* to first order, which gives 2πf cos φ₀ = 4(2ζ₀)^½
    /// in θ. None when cos φ₀ > 1, beyond [`max_detuning`], or where the response has no lower
    /// state.
    pub fn soliton(&self, d2: f64) -> Option<Soliton> {
        let z = self.detuning;
        if !(z > 0.0 && d2 > 0.0 && self.drive > 0.0) {
            return None;
        }
        let cos_phase = (8.0 * z).sqrt() / (PI * self.drive);
        let lower = *self.homogeneous().first()?;
        (cos_phase <= 1.0).then(|| Soliton {
            amplitude: (2.0 * z).sqrt(),
            width: (d2 / z).sqrt(),
            cos_phase,
            background: self.homogeneous_field(lower),
        })
    }

    /// The approximate soliton sampled around the ring, centred on the middle point: the
    /// background plus B sech(φ/W) e^{iφ₀}, φ₀ the root of cos φ₀ with sin φ₀ ≥ 0.
    pub fn soliton_field(&self, soliton: &Soliton) -> Vec<c64> {
        let n = self.points();
        let phase = c64::new(
            soliton.cos_phase,
            (1.0 - soliton.cos_phase.powi(2)).max(0.0).sqrt(),
        );
        (0..n)
            .map(|j| {
                let phi = 2.0 * PI * (j as f64 - (n / 2) as f64) / n as f64;
                soliton.background + phase * (soliton.amplitude / (phi / soliton.width).cosh())
            })
            .collect()
    }
}

/// The FWHM of the highest peak of |ψ|² around the ring, in rad of its angle, from the field's
/// band-limited (trigonometric) interpolation on 16 times its points, the half-maximum points
/// found linearly between those.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a field whose length isn't a power of two of at least 4, or
/// whose peak doesn't fall to half within half the ring either side.
pub fn peak_width(field: &[c64]) -> Result<f64> {
    let n = field.len();
    let fft = Fft::new(n)?;
    let m = 16 * n;
    let fine_fft = Fft::new(m)?;
    let mut modes = field.to_vec();
    fft.inverse(&mut modes);
    let mut fine = vec![c64::new(0.0, 0.0); m];
    for (k, v) in modes.iter().enumerate() {
        // bin k is mode k below n/2, k − n above; the Nyquist bin split between ±n/2
        if k < n / 2 {
            fine[k] = *v;
        } else if k == n / 2 {
            fine[k] = 0.5 * v;
            fine[m - n / 2] = 0.5 * v;
        } else {
            fine[m - (n - k)] = *v;
        }
    }
    fine_fft.forward(&mut fine);
    let p: Vec<f64> = fine.iter().map(|v| v.norm_sqr()).collect();
    let (j, peak) =
        p.iter().enumerate().fold(
            (0, f64::NEG_INFINITY),
            |a, (i, &v)| if v > a.1 { (i, v) } else { a },
        );
    let half = 0.5 * peak;
    let side = |dir: i64| -> Option<f64> {
        for k in 1..m / 2 {
            let at = |s: usize| p[(j as i64 + dir * s as i64).rem_euclid(m as i64) as usize];
            let (a, b) = (at(k), at(k - 1));
            if a < half {
                return Some(k as f64 - (half - a) / (b - a));
            }
        }
        None
    };
    match (side(-1), side(1)) {
        (Some(l), Some(r)) => Ok((l + r) * 2.0 * PI / m as f64),
        _ => Err(Error::invalid(
            "peak width",
            "the peak doesn't fall to half its height within half the ring",
        )),
    }
}

/// The largest detuning at which the approximate soliton exists, ζ₀ = π²f²/8, where cos φ₀
/// reaches 1 ([`Resonator::soliton`]).
pub fn max_detuning(drive: f64) -> f64 {
    PI * PI * drive * drive / 8.0
}

/// A ring resonator from its waveguide, in Coen et al.'s Eq. (3).
#[derive(Clone, Debug, PartialEq)]
pub struct Ring {
    /// The round trip L, m.
    pub length: f64,
    /// The round-trip time t_R, ps.
    pub roundtrip: f64,
    /// α = α_i + θ/2, the field's loss per round trip.
    pub loss: f64,
    /// θ, the coupler's power transmission.
    pub coupling: f64,
    /// δ₀, the detuning per round trip, rad (positive: the pump redder than the resonance).
    pub detuning: f64,
    /// The waveguide's γ, 1/(W·m).
    pub gamma: f64,
    /// The waveguide's β_k at the pump.
    pub dispersion: Dispersion,
    /// P_in, W.
    pub pump: f64,
}

impl Ring {
    /// ζ₀ = δ₀/α.
    pub fn normalized_detuning(&self) -> f64 {
        self.detuning / self.loss
    }

    /// f = (θγLP_in/α³)^½.
    pub fn normalized_drive(&self) -> f64 {
        (self.coupling * self.gamma * self.length * self.pump / self.loss.powi(3)).sqrt()
    }

    /// The power (W) of |ψ|² = 1: α/(γL).
    pub fn power_scale(&self) -> f64 {
        self.loss / (self.gamma * self.length)
    }

    /// The comb line spacing Ω₁ = 2π/t_R, rad/ps.
    pub fn line_spacing(&self) -> f64 {
        2.0 * PI / self.roundtrip
    }

    /// The finesse F = π/α.
    pub fn finesse(&self) -> f64 {
        PI / self.loss
    }

    /// The resonator on `points` modes, δ_μ = −(L/α)Σβ_kΩ_μ^k/k!.
    ///
    /// # Errors
    ///
    /// As [`Resonator::new`].
    pub fn resonator(&self, points: usize) -> Result<Resonator> {
        let w1 = self.line_spacing();
        Resonator::new(
            self.normalized_detuning(),
            self.normalized_drive(),
            |m| -(self.length / self.loss) * self.dispersion.phase(m as f64 * w1),
            points,
        )
    }

    /// d₂ = L|β₂|Ω₁²/(2α) (the δ_μ = d₂μ² of β₂ alone), with β₂ < 0 for anomalous dispersion.
    pub fn d2(&self) -> f64 {
        -self.length * self.dispersion.beta(2) * self.line_spacing().powi(2) / (2.0 * self.loss)
    }

    /// The time (ps) of an angle φ around the ring: t_R φ/2π.
    pub fn time_of(&self, angle: f64) -> f64 {
        self.roundtrip * angle / (2.0 * PI)
    }

    /// The round-trip time from a length and group index: t_R = n_gL/c, ps.
    pub fn roundtrip_of(length: f64, group_index: f64) -> f64 {
        group_index * length / C_M_PER_PS
    }

    /// The approximate soliton's shortest FWHM (of its power), at [`max_detuning`], ps: its
    /// T₀ = (L|β₂|/(2αζ₀))^½ there is (2/π)α(|β₂|/(θγP_in))^½, and the FWHM 2 acosh(√2)·T₀. At
    /// critical coupling (θ = α, F = π/α) it is 1.989 (|β₂|/(γFP_in))^½: Herr et al.'s Eq. (2),
    /// whose prefactor is ≈ 2.
    pub fn minimum_soliton_width(&self) -> f64 {
        let fwhm = 2.0 * (2f64.sqrt()).acosh();
        fwhm * (2.0 / PI)
            * self.loss
            * (self.dispersion.beta(2).abs() / (self.coupling * self.gamma * self.pump)).sqrt()
    }
}

/// Herr et al.'s Eq. (2): the shortest soliton FWHM in a critically coupled resonator,
/// Δt_min ≈ 2(−β₂/(γFP_in))^½ (ps for β₂ in ps²/m, γ in 1/(W·m), P_in in W).
pub fn herr_minimum_width(beta2: f64, gamma: f64, finesse: f64, pump: f64) -> f64 {
    2.0 * (-beta2 / (gamma * finesse * pump)).sqrt()
}

#[cfg(test)]
#[path = "resonator_tests.rs"]
mod tests;
