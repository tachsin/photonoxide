//! Dispersive media by auxiliary differential equations: Drude and Lorentz terms, each a
//! polarization current stepped with E.
//!
//! - **The medium:** ε(ω) = ε∞ + Σ χₚ(ω), each term χₚ = aₚ / (ω₀ₚ² − ω² − iΓₚω) in the
//!   e^(−iωt) convention: a Lorentz oscillator, aₚ = Δεₚ ω₀ₚ², or a Drude term, ω₀ₚ = 0 and
//!   aₚ = ωₚ². Ampère's law is ε∞ ∂E/∂t + σE + Σ Jₚ = ∇ × H̃ − J, each polarization current
//!   Jₚ = ∂Pₚ/∂t obeying (M. Okoniewski, M. Mrozowski, M. A. Stuchly, IEEE Microw. Guided Wave
//!   Lett. 7, 121 (1997), doi:10.1109/75.569723, Eq. 5, with ε₀ = 1 and their δₚ = Γₚ/2)
//!
//!   ∂²Jₚ/∂t² + Γₚ ∂Jₚ/∂t + ω₀ₚ² Jₚ = aₚ ∂E/∂t.
//!
//!   For a Drude term it is the time derivative of ∂Jₚ/∂t + ΓₚJₚ = ωₚ²E, the current a free
//!   electron gas carries, and is the same from rest.
//! - **The update** is their synchronized Lorentz scheme (LADES): Jₚ at the steps, E's update at
//!   the half step with Jₚ^(n+½) = (Jₚⁿ⁺¹ + Jₚⁿ)/2, and the currents' equation centred on step
//!   n with (∂E/∂t)ⁿ = (Eⁿ⁺¹ − Eⁿ⁻¹)/(2Δt):
//!
//!   Jₚⁿ⁺¹ = αₚJₚⁿ + ξₚJₚⁿ⁻¹ + γₚ(Eⁿ⁺¹ − Eⁿ⁻¹)/(2Δt),
//!   αₚ = (2 − ω₀ₚ²Δt²)/(1 + ΓₚΔt/2), ξₚ = (ΓₚΔt/2 − 1)/(1 + ΓₚΔt/2),
//!   γₚ = aₚΔt²/(1 + ΓₚΔt/2),
//!
//!   derived here from Eq. 5 by central differences (their Eq. 8 prints γₚ without the
//!   1/(1 + δₚΔt), which the centred damping term needs). Put into E's update, the new E is
//!   implicit only through γₚ, and solves to E ← p_e E + p_m (∇ × H̃ − J) + p_m [χ Eⁿ⁻¹ −
//!   Σ ((αₚ + 1)Jₚⁿ + ξₚJₚⁿ⁻¹)/2], with χ = Σ γₚ/(4Δt), ν = 1 + (Δt/ε∞)(σ/2 + χ),
//!   p_e = (1 − Δtσ/(2ε∞))/ν and p_m = (Δt/ε∞)/ν: their pseudo-code's LADES case. The grid's
//!   own update gives p_e E + p_m (∇ × H̃ − J) with p_e and p_m in place of its ca and cb;
//!   the rest is added after it, and the currents then step. 2P + 1 values per E value: Jₚⁿ,
//!   Jₚⁿ⁻¹ and Eⁿ⁻¹.
//! - **What the run solves:** for e^(−iωt) at the steps, the update is FDFD's equations at the
//!   leapfrog's ω̃ = (2/Δt) sin(ωΔt/2) with each term's χₚ replaced by
//!   (i cos(ωΔt/2)/ω̃) γₚ (z − 1/z) / (2Δt (z − αₚ − ξₚ/z)), z = e^(−iωΔt)
//!   ([`Dispersive::leapfrog_permittivity`]): χₚ to second order in Δt.
//! - **Stability:** a medium is refused unless its terms are passive (ε∞ > 0, Δε ≥ 0, Γ ≥ 0: no
//!   gain), and unless every plane wave the grid holds, (KΔt)² from 0 to 4C², steps with
//!   |z| ≤ 1: the roots of the update's characteristic polynomial,
//!   ε∞(z − 1)² + (σΔt/2)(z² − 1) + Σ (γₚ/4)(z² − 1)²/(z² − αₚz − ξₚ) + (KΔt)² z = 0,
//!   found by Aberth's iteration. In practice that is C ≤ √ε∞ and ω₀ₚΔt ≤ 2 for each term.
//! - **Fits** of a material's ε(λ) over a band ([`Dispersive::fit`]): ε∞ ≥ 1 and Lorentz terms
//!   of nonnegative strength at fixed resonances (and dampings, where the material absorbs)
//!   around the band, by nonnegative least squares (C. L. Lawson, R. J. Hanson, Solving Least
//!   Squares Problems, SIAM (1995), doi:10.1137/1.9781611971217, Ch. 23), so every fit is
//!   passive and stable; the largest error over the band is stated.

use std::collections::HashMap;

use super::*;
use crate::material::{Material, Model};
use crate::units::Wavelength;

/// One term of a dispersive medium's susceptibility, in the e^(−iωt) convention, as
/// [`Model::Lorentz`] and [`Model::Drude`] write them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pole {
    /// A Lorentz oscillator, Δε ω₀² / (ω₀² − ω² − iγω).
    Lorentz {
        /// The strength Δε, ≥ 0.
        strength: f64,
        /// The resonance f₀ (ω₀ = 2πf₀).
        resonance: Frequency,
        /// The damping γ/2π, as a frequency in c/µm, ≥ 0.
        damping: f64,
    },
    /// A Drude term, −ω_p² / (ω² + iγω).
    Drude {
        /// The plasma frequency f_p (ω_p = 2πf_p).
        plasma: Frequency,
        /// The collision rate γ/2π, as a frequency in c/µm, ≥ 0.
        damping: f64,
    },
}

impl Pole {
    /// a, ω₀² and Γ of a / (ω₀² − ω² − iΓω), angular frequencies in rad/µm (c = 1).
    fn parts(&self) -> (f64, f64, f64) {
        match *self {
            Pole::Lorentz {
                strength,
                resonance,
                damping,
            } => {
                let w0 = resonance.angular();
                (strength * w0 * w0, w0 * w0, std::f64::consts::TAU * damping)
            }
            Pole::Drude { plasma, damping } => {
                let wp = plasma.angular();
                (wp * wp, 0.0, std::f64::consts::TAU * damping)
            }
        }
    }

    /// Its susceptibility at `frequency`.
    pub fn susceptibility(&self, frequency: Frequency) -> c64 {
        let w = frequency.angular();
        let (a, w0sq, gamma) = self.parts();
        a / c64::new(w0sq - w * w, -gamma * w)
    }

    /// The update's coefficients at the step `dt`: αₚ, ξₚ and γₚ.
    fn coefficients(&self, dt: f64) -> [f64; 3] {
        let (a, w0sq, gamma) = self.parts();
        let d = 1.0 + gamma * dt / 2.0;
        [
            (2.0 - w0sq * dt * dt) / d,
            (gamma * dt / 2.0 - 1.0) / d,
            a * dt * dt / d,
        ]
    }
}

/// A dispersive medium, ε(ω) = ε∞ + Σ terms.
#[derive(Clone, Debug, PartialEq)]
pub struct Dispersive {
    /// The permittivity at high frequency, ε∞ > 0.
    pub eps_inf: f64,
    /// The terms.
    pub poles: Vec<Pole>,
}

/// A medium fitted to a material over a band, by [`Dispersive::fit`].
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    /// The medium.
    pub medium: Dispersive,
    /// The band's shortest and longest vacuum wavelengths.
    pub band: (Wavelength, Wavelength),
    /// The largest |ε_fit − ε| over the band.
    pub error: f64,
    /// The largest |ε_fit − ε| / |ε| over the band.
    pub relative: f64,
}

impl Dispersive {
    /// Its relative permittivity at `frequency`.
    pub fn permittivity(&self, frequency: Frequency) -> c64 {
        self.poles
            .iter()
            .map(|p| p.susceptibility(frequency))
            .sum::<c64>()
            + self.eps_inf
    }

    /// The permittivity a run with time step `dt` sees at `frequency`: a source oscillating
    /// as e^(−iωt) settles to FDFD's field at ω̃ = (2/Δt) sin(ωΔt/2) with this permittivity (and
    /// a conductivity σ adding iσ cos(ωΔt/2)/ω̃, as without the medium). It is
    /// [`Dispersive::permittivity`] to second order in Δt.
    pub fn leapfrog_permittivity(&self, frequency: Frequency, dt: f64) -> c64 {
        let w = frequency.angular();
        let z = c64::from_polar(1.0, -w * dt);
        let tilde = 2.0 / dt * (w * dt / 2.0).sin();
        let factor = c64::new(0.0, (w * dt / 2.0).cos() / tilde);
        let mut eps = c64::new(self.eps_inf, 0.0);
        for p in &self.poles {
            let [alpha, xi, gamma] = p.coefficients(dt);
            eps += factor * gamma * (z - 1.0 / z) / (2.0 * dt * (z - alpha - xi / z));
        }
        eps
    }

    /// The medium of a model whose terms are Lorentz or Drude terms, exactly: a real
    /// [`Model::Constant`], [`Model::Drude`], [`Model::Lorentz`] and [`Model::Sellmeier`] (each
    /// term B λ²/(λ² − C²) is Δε = B at f₀ = 1/C, undamped; C = 0 adds B to ε∞).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for another model (fit it over a band with
    /// [`Dispersive::fit`]), a complex constant, a Sellmeier term of negative strength, or a
    /// medium [`Dispersive::check`] refuses.
    pub fn from_model(model: &Model) -> Result<Dispersive> {
        let medium = match model {
            Model::Constant(n) if n.im == 0.0 => Dispersive {
                eps_inf: n.re * n.re,
                poles: Vec::new(),
            },
            Model::Drude {
                eps_inf,
                plasma,
                damping,
            } => Dispersive {
                eps_inf: *eps_inf,
                poles: vec![Pole::Drude {
                    plasma: *plasma,
                    damping: *damping,
                }],
            },
            Model::Lorentz { eps_inf, poles } => Dispersive {
                eps_inf: *eps_inf,
                poles: poles
                    .iter()
                    .map(|p| Pole::Lorentz {
                        strength: p.strength,
                        resonance: p.resonance,
                        damping: p.damping,
                    })
                    .collect(),
            },
            Model::Sellmeier { a, terms } => {
                let mut medium = Dispersive {
                    eps_inf: *a,
                    poles: Vec::new(),
                };
                for t in terms {
                    if t.c == 0.0 {
                        medium.eps_inf += t.b;
                    } else {
                        medium.poles.push(Pole::Lorentz {
                            strength: t.b,
                            resonance: Frequency::natural(1.0 / t.c.abs())?,
                            damping: 0.0,
                        });
                    }
                }
                medium
            }
            other => {
                return Err(invalid(format!(
                    "a {} model has no exact Lorentz or Drude terms: fit it over a band with \
                     Dispersive::fit",
                    model_name(other)
                )));
            }
        };
        medium.check()?;
        Ok(medium)
    }

    /// Checks that the medium is passive: ε∞ positive and finite, every term's strength and
    /// damping finite and nonnegative. A term of negative strength or damping is a gain.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] otherwise.
    pub fn check(&self) -> Result<()> {
        if !(self.eps_inf.is_finite() && self.eps_inf > 0.0) {
            return Err(invalid(format!(
                "a dispersive medium's eps_inf must be positive and finite, got {}",
                self.eps_inf
            )));
        }
        for p in &self.poles {
            let (strength, damping) = match *p {
                Pole::Lorentz {
                    strength, damping, ..
                } => (strength, damping),
                Pole::Drude { damping, .. } => (1.0, damping),
            };
            if !(strength.is_finite() && strength >= 0.0 && damping.is_finite() && damping >= 0.0) {
                return Err(invalid(format!(
                    "a term of negative strength or damping is a gain, which FDTD can't step: \
                     {p:?}"
                )));
            }
        }
        Ok(())
    }

    /// The largest |z| over the modes of a plane wave stepped in this medium with conductivity
    /// `sigma`, at time step `dt`, for (KΔt)² = `u`: the roots of the update's characteristic
    /// polynomial (see the module's docs).
    pub(super) fn amplification(&self, sigma: f64, dt: f64, u: f64) -> f64 {
        // ε∞(z − 1)² + (σΔt/2)(z² − 1) + u z, times every term's z² − αz − ξ, plus each term's
        // (γ/4)(z² − 1)² times the others'
        let quadratics: Vec<[f64; 3]> = self
            .poles
            .iter()
            .map(|p| {
                let [alpha, xi, _] = p.coefficients(dt);
                [-xi, -alpha, 1.0]
            })
            .collect();
        let product = |skip: Option<usize>| {
            quadratics
                .iter()
                .enumerate()
                .filter(|(i, _)| Some(*i) != skip)
                .fold(vec![1.0], |acc, (_, q)| multiply(&acc, q))
        };
        let e = self.eps_inf;
        let half = sigma * dt / 2.0;
        let base = [e - half, u - 2.0 * e, e + half];
        let mut poly = multiply(&base, &product(None));
        for (i, p) in self.poles.iter().enumerate() {
            let [_, _, gamma] = p.coefficients(dt);
            let term = multiply(
                &[gamma / 4.0, 0.0, -gamma / 2.0, 0.0, gamma / 4.0],
                &product(Some(i)),
            );
            for (c, t) in poly.iter_mut().zip(term) {
                *c += t;
            }
        }
        roots(&poly).iter().fold(0.0f64, |m, z| m.max(z.norm()))
    }

    /// Whether the medium steps stably with conductivity `sigma` at time step `dt` on a grid
    /// whose curl's largest eigenvalue is `top` (4 Σ wᵢ/Δᵢ²): every (KΔt)² up to `top` Δt²,
    /// in 200 steps. K = 0 is left out: there z = 1 is a double root, a static field, which
    /// the iteration finds only to the cube root of round-off.
    pub(super) fn stable(&self, sigma: f64, dt: f64, top: f64) -> bool {
        let points = 200;
        let most = top * dt * dt;
        (1..=points).all(|i| {
            let u = most * i as f64 / points as f64;
            self.amplification(sigma, dt, u) <= 1.0 + 1e-6
        })
    }

    /// A medium fitted to `material`'s permittivity from `shortest` to `longest` (vacuum
    /// wavelengths): ε∞ ≥ 1 and Lorentz terms of nonnegative strength at fixed resonances, by
    /// nonnegative least squares over 64 frequencies evenly spaced in the band, with the
    /// largest error over 256.
    ///
    /// Where the material is lossless over the band (Im ε below 1e-9 of |ε|), the terms are
    /// undamped, at 1.25 to 4 times the band's highest frequency and 0.8 to 0.25 times its
    /// lowest; the fit is then a Sellmeier formula. Where it absorbs, the resonances run from
    /// half the lowest frequency to twice the highest, each at dampings of 0.05, 0.15 and 0.4
    /// times itself, with undamped ones above.
    ///
    /// # Errors
    ///
    /// [`Error::OutsideValidity`] if the band reaches beyond the material's range, as the
    /// material refuses those wavelengths; [`Error::InvalidValue`] for an empty band.
    pub fn fit(material: &Material, shortest: Wavelength, longest: Wavelength) -> Result<Fit> {
        if shortest >= longest {
            return Err(invalid(format!(
                "a fit's band needs shortest < longest, got {shortest} to {longest}"
            )));
        }
        let (low, high) = (1.0 / longest.to_um(), 1.0 / shortest.to_um());
        // the band's frequencies, its ends at the given wavelengths exactly
        let sample = |count: usize| -> Result<Vec<(f64, c64)>> {
            (0..count)
                .map(|i| {
                    let lam = if i == 0 {
                        longest
                    } else if i + 1 == count {
                        shortest
                    } else {
                        let f = low + (high - low) * i as f64 / (count - 1) as f64;
                        Wavelength::um(1.0 / f)?
                    };
                    Ok((1.0 / lam.to_um(), material.permittivity(lam)?))
                })
                .collect()
        };
        let data = sample(64)?;
        let largest = data.iter().fold(0.0f64, |m, (_, e)| m.max(e.norm()));
        let lossy = data.iter().any(|(_, e)| e.im.abs() > 1e-9 * largest);
        // the candidates' resonances and dampings, c/µm
        let mut terms: Vec<(f64, f64)> = Vec::new();
        if lossy {
            let count = 16;
            for i in 0..count {
                let f = 0.5 * low * (4.0 * high / low).powf(i as f64 / (count - 1) as f64);
                terms.extend([0.05, 0.15, 0.4].map(|d| (f, d * f)));
            }
            terms.extend([2.5, 3.2, 4.0].map(|r| (r * high, 0.0)));
        } else {
            terms.extend([1.25, 1.6, 2.0, 2.5, 3.2, 4.0].map(|r| (r * high, 0.0)));
            terms.extend([0.8, 0.63, 0.5, 0.4, 0.32, 0.25].map(|r| (r * low, 0.0)));
        }
        let candidates = terms
            .into_iter()
            .map(|(f, damping)| {
                Ok(Pole::Lorentz {
                    strength: 1.0,
                    resonance: Frequency::natural(f)?,
                    damping,
                })
            })
            .collect::<Result<Vec<Pole>>>()?;
        // the columns: ε∞ − 1, then each candidate at unit strength; rows: Re, and Im if lossy
        let rows = data.len() * if lossy { 2 } else { 1 };
        let columns = 1 + candidates.len();
        let mut a = vec![0.0; rows * columns];
        let mut b = vec![0.0; rows];
        for (i, &(f, eps)) in data.iter().enumerate() {
            let frequency = Frequency::natural(f)?;
            a[i * columns] = 1.0;
            b[i] = eps.re - 1.0;
            if lossy {
                let j = data.len() + i;
                b[j] = eps.im;
            }
            for (k, p) in candidates.iter().enumerate() {
                let chi = p.susceptibility(frequency);
                a[i * columns + 1 + k] = chi.re;
                if lossy {
                    a[(data.len() + i) * columns + 1 + k] = chi.im;
                }
            }
        }
        let x = nonnegative_least_squares(&a, &b, rows, columns);
        let medium = Dispersive {
            eps_inf: 1.0 + x[0],
            poles: candidates
                .iter()
                .zip(&x[1..])
                .filter(|(_, s)| **s > 0.0)
                .map(|(p, &s)| match *p {
                    Pole::Lorentz {
                        resonance, damping, ..
                    } => Pole::Lorentz {
                        strength: s,
                        resonance,
                        damping,
                    },
                    drude => drude,
                })
                .collect(),
        };
        medium.check()?;
        let (mut error, mut relative) = (0.0f64, 0.0f64);
        for (f, eps) in sample(256)? {
            let d = (medium.permittivity(Frequency::natural(f)?) - eps).norm();
            error = error.max(d);
            relative = relative.max(d / eps.norm());
        }
        Ok(Fit {
            medium,
            band: (shortest, longest),
            error,
            relative,
        })
    }
}

fn model_name(model: &Model) -> &'static str {
    match model {
        Model::Constant(_) => "complex constant",
        Model::Cauchy(_) => "Cauchy",
        Model::Tabulated(_) => "tabulated",
        Model::Formula { .. } => "formula",
        Model::WithExtinction { .. } => "with-extinction",
        Model::Pikhtin { .. } => "Pikhtin",
        Model::Afromowitz { .. } => "Afromowitz",
        _ => "this",
    }
}

/// The product of two polynomials, coefficients from the constant term up.
fn multiply(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] += x * y;
        }
    }
    out
}

/// The roots of a real polynomial (coefficients from the constant term up, the leading one
/// nonzero), by Aberth's simultaneous iteration (O. Aberth, Math. Comput. 27, 339 (1973),
/// doi:10.1090/S0025-5718-1973-0329236-7).
pub(super) fn roots(coefficients: &[f64]) -> Vec<c64> {
    let mut p: Vec<f64> = coefficients.to_vec();
    while p.len() > 1 && p[p.len() - 1] == 0.0 {
        p.pop();
    }
    let n = p.len() - 1;
    if n == 0 {
        return Vec::new();
    }
    let lead = p[n];
    let p: Vec<f64> = p.iter().map(|c| c / lead).collect();
    // start on a circle of the roots' geometric mean modulus, turned off the real axis
    let radius = p[0].abs().powf(1.0 / n as f64).max(1e-3);
    let mut z: Vec<c64> = (0..n)
        .map(|k| c64::from_polar(radius, std::f64::consts::TAU * (k as f64 + 0.25) / n as f64))
        .collect();
    let eval = |x: c64| -> (c64, c64) {
        let (mut v, mut d) = (c64::new(0.0, 0.0), c64::new(0.0, 0.0));
        for &c in p.iter().rev() {
            d = d * x + v;
            v = v * x + c;
        }
        (v, d)
    };
    for _ in 0..500 {
        let mut largest = 0.0f64;
        for k in 0..n {
            let (v, d) = eval(z[k]);
            if v == c64::new(0.0, 0.0) {
                continue;
            }
            let ratio = v / d;
            let repulsion: c64 = (0..n)
                .filter(|&j| j != k)
                .map(|j| 1.0 / (z[k] - z[j]))
                .sum();
            let step = ratio / (1.0 - ratio * repulsion);
            if step.re.is_finite() && step.im.is_finite() {
                z[k] -= step;
                largest = largest.max(step.norm() / z[k].norm().max(1e-300));
            }
        }
        if largest < 1e-15 {
            break;
        }
    }
    z
}

/// min ‖A x − b‖ over x ≥ 0, A `rows` × `columns` by rows: Lawson and Hanson's active set
/// (Ch. 23, Algorithm NNLS), each least squares on the free columns by
/// [`crate::compact::fit::least_squares`].
fn nonnegative_least_squares(a: &[f64], b: &[f64], rows: usize, columns: usize) -> Vec<f64> {
    use faer::Mat;
    // columns scaled to unit norm, so that the gradient's entries compare
    let scale: Vec<f64> = (0..columns)
        .map(|j| {
            let norm = (0..rows)
                .map(|i| a[i * columns + j].powi(2))
                .sum::<f64>()
                .sqrt();
            if norm > 0.0 { 1.0 / norm } else { 0.0 }
        })
        .collect();
    let at = |i: usize, j: usize| a[i * columns + j] * scale[j];
    let residual = |x: &[f64]| -> Vec<f64> {
        (0..rows)
            .map(|i| b[i] - (0..columns).map(|j| at(i, j) * x[j]).sum::<f64>())
            .collect()
    };
    let gradient = |r: &[f64]| -> Vec<f64> {
        (0..columns)
            .map(|j| (0..rows).map(|i| at(i, j) * r[i]).sum::<f64>())
            .collect()
    };
    let solve = |free: &[usize]| -> Vec<f64> {
        let m = Mat::from_fn(rows, free.len(), |i, k| at(i, free[k]));
        let rhs = Mat::from_fn(rows, 1, |i, _| b[i]);
        crate::compact::fit::least_squares(m, &rhs)
    };
    let tolerance = 1e-12 * b.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-300);
    let mut x = vec![0.0; columns];
    let mut free: Vec<usize> = Vec::new();
    for _ in 0..3 * columns {
        let w = gradient(&residual(&x));
        let next = (0..columns)
            .filter(|j| !free.contains(j))
            .max_by(|&p, &q| w[p].total_cmp(&w[q]));
        let Some(t) = next.filter(|&t| w[t] > tolerance) else {
            break;
        };
        free.push(t);
        loop {
            let s = solve(&free);
            if s.iter().all(|&v| v > 0.0) {
                for (k, &j) in free.iter().enumerate() {
                    x[j] = s[k];
                }
                break;
            }
            // back along the segment to the first variable that reaches zero
            let mut alpha = 1.0f64;
            for (k, &j) in free.iter().enumerate() {
                if s[k] <= 0.0 {
                    alpha = alpha.min(x[j] / (x[j] - s[k]));
                }
            }
            for (k, &j) in free.iter().enumerate() {
                x[j] += alpha * (s[k] - x[j]);
            }
            free.retain(|&j| x[j] > 1e-15 * x.iter().fold(0.0f64, |m, v| m.max(*v)));
            for (j, v) in x.iter_mut().enumerate() {
                if !free.contains(&j) {
                    *v = 0.0;
                }
            }
            if free.is_empty() {
                break;
            }
        }
    }
    x.iter().zip(&scale).map(|(v, s)| v * s).collect()
}

/// The values of one dispersive medium and their currents.
#[derive(Clone, Debug)]
struct Group {
    /// Each value's component and index.
    values: Vec<(usize, usize)>,
    /// p_m at each value: the scale of the curl in E's update there.
    scale: Vec<f64>,
    /// χ = Σ γₚ/(4Δt).
    chi: f64,
    /// αₚ, ξₚ, γₚ per term.
    poles: Vec<[f64; 3]>,
    /// Eⁿ⁻¹ and Eⁿ at each value.
    before: Vec<f64>,
    now: Vec<f64>,
    /// Jₚⁿ and Jₚⁿ⁻¹, the terms of a value together.
    current: Vec<f64>,
    previous: Vec<f64>,
}

/// The dispersive media of a run.
#[derive(Clone, Debug, Default)]
pub(super) struct Media {
    groups: Vec<Group>,
    /// Each dispersive value's group, by component and index.
    members: HashMap<(usize, usize), usize>,
}

impl Media {
    pub(super) fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Whether E's value `r` of component `c` is in a dispersive medium.
    pub(super) fn contains(&self, c: usize, r: usize) -> bool {
        self.members.contains_key(&(c, r))
    }

    /// χ at a value: 0 outside the media.
    fn chi(&self, c: usize, r: usize) -> f64 {
        self.members
            .get(&(c, r))
            .map_or(0.0, |&g| self.groups[g].chi)
    }

    /// Takes a value out of its medium (before the run starts).
    pub(super) fn remove(&mut self, c: usize, r: usize) {
        let Some(g) = self.members.remove(&(c, r)) else {
            return;
        };
        let group = &mut self.groups[g];
        let keep: Vec<bool> = group.values.iter().map(|&v| v != (c, r)).collect();
        let mut k = keep.iter();
        group.values.retain(|_| *k.next().unwrap());
        let mut k = keep.iter();
        group.scale.retain(|_| *k.next().unwrap());
        group.resize();
    }

    /// Keeps Eⁿ before E's update.
    pub(super) fn save(&mut self, e: &[Vec<f64>; 3]) {
        for g in &mut self.groups {
            for (now, &(c, r)) in g.now.iter_mut().zip(&g.values) {
                *now = e[c][r];
            }
        }
    }

    /// After E's update: the rest of E's update in the media, then the currents' step.
    pub(super) fn update(&mut self, e: &mut [Vec<f64>; 3], dt: f64) {
        for g in &mut self.groups {
            let p = g.poles.len();
            let (poles, chi) = (&g.poles, g.chi);
            let fields = &*e;
            let updated: Vec<f64> = g
                .current
                .par_chunks_mut(p)
                .zip(g.previous.par_chunks_mut(p))
                .zip(g.before.par_iter_mut())
                .zip(g.now.par_iter())
                .zip(g.scale.par_iter())
                .zip(g.values.par_iter())
                .map(
                    |(((((current, previous), before), &now), &scale), &(c, r))| {
                        let mut rest = chi * *before;
                        for (q, &[alpha, xi, _]) in poles.iter().enumerate() {
                            rest -= 0.5 * ((alpha + 1.0) * current[q] + xi * previous[q]);
                        }
                        let next = fields[c][r] + scale * rest;
                        let slope = (next - *before) / (2.0 * dt);
                        for (q, &[alpha, xi, gamma]) in poles.iter().enumerate() {
                            let j = alpha * current[q] + xi * previous[q] + gamma * slope;
                            previous[q] = current[q];
                            current[q] = j;
                        }
                        *before = now;
                        next
                    },
                )
                .collect();
            for (&(c, r), v) in g.values.iter().zip(updated) {
                e[c][r] = v;
            }
        }
    }
}

impl Group {
    /// The state's length for its values, all zero.
    fn resize(&mut self) {
        let n = self.values.len();
        let p = self.poles.len();
        self.before = vec![0.0; n];
        self.now = vec![0.0; n];
        self.current = vec![0.0; n * p];
        self.previous = vec![0.0; n * p];
    }
}

impl Simulation {
    /// Puts the dispersive `medium` wherever `inside(x, y, z)` holds at a value of E, sampled
    /// there (not averaged over the cell, as the permittivity is): its ε∞ replaces the
    /// permittivity there, and its terms step there. A conductivity given before stays; a
    /// medium put later over this one replaces it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a medium [`Dispersive::check`] refuses, a medium the time
    /// step can't step stably (see the module docs: in practice ω₀Δt above 2 for a term, or a
    /// Courant number above √ε∞), or after the run has started.
    pub fn with_medium(
        self,
        medium: &Dispersive,
        inside: impl Fn(f64, f64, f64) -> bool,
    ) -> Result<Simulation> {
        self.medium(medium, &inside)
    }

    fn medium(
        mut self,
        medium: &Dispersive,
        inside: &dyn Fn(f64, f64, f64) -> bool,
    ) -> Result<Simulation> {
        medium.check()?;
        if self.steps > 0 {
            return Err(invalid("a dispersive medium goes in before the run starts"));
        }
        if let Some(mut b) = self.bloch.take() {
            b.twin = b.twin.medium(medium, inside)?;
            self.bloch = Some(b);
        }
        let dt = self.dt;
        let top = 4.0 * bloch::curl_bound(&self.grid, &self.boundaries);
        let poles: Vec<[f64; 3]> = medium.poles.iter().map(|p| p.coefficients(dt)).collect();
        let chi: f64 = poles.iter().map(|p| p[2]).sum::<f64>() / (4.0 * dt);
        let grid = self.grid;
        let mut values = Vec::new();
        let mut sigmas = Vec::new();
        for component in Axis::ALL {
            let c = component.index();
            for r in 0..grid.cells() {
                if self.cb[c][r] == 0.0 {
                    continue;
                }
                let at = (
                    r % grid.nx,
                    (r / grid.nx) % grid.ny,
                    r / (grid.nx * grid.ny),
                );
                let [x, y, z] = grid.e_position(component, at);
                if inside(x, y, z) {
                    // σ = (1 − ca)/cb − χ, whatever the medium there was
                    let sigma = (1.0 - self.ca[c][r]) / self.cb[c][r] - self.media.chi(c, r);
                    values.push((c, r));
                    sigmas.push(sigma.max(0.0));
                }
            }
        }
        let largest = sigmas.iter().fold(0.0f64, |m, s| m.max(*s));
        for sigma in [0.0, largest] {
            if !medium.stable(sigma, dt, top) {
                return Err(invalid(format!(
                    "the medium {medium:?} isn't stable at the time step {dt} um/c (a term's \
                     w0 dt above 2, or a Courant number above sqrt(eps_inf)): take a smaller \
                     step"
                )));
            }
        }
        let e = medium.eps_inf;
        let mut scale = Vec::with_capacity(values.len());
        for (&(c, r), &sigma) in values.iter().zip(&sigmas) {
            self.media.remove(c, r);
            let nu = 1.0 + dt / e * (sigma / 2.0 + chi);
            self.ca[c][r] = (1.0 - dt * sigma / (2.0 * e)) / nu;
            self.cb[c][r] = dt / e / nu;
            scale.push(self.cb[c][r]);
        }
        if !poles.is_empty() && !values.is_empty() {
            let g = self.media.groups.len();
            for &v in &values {
                self.media.members.insert(v, g);
            }
            let mut group = Group {
                values,
                scale,
                chi,
                poles,
                before: Vec::new(),
                now: Vec::new(),
                current: Vec::new(),
                previous: Vec::new(),
            };
            group.resize();
            self.media.groups.push(group);
        }
        Ok(self)
    }
}
