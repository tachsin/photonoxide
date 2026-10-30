//! Mie's series: a plane wave scattered by a sphere, exactly (G. Mie, Ann. Phys. 330, 377
//! (1908), doi:10.1002/andp.19083300302).
//!
//! A sphere of radius ρ in a lossless medium of index n₀, lit by a plane wave of vacuum
//! wavelength λ, is described by Mie's size parameter α = 2π n₀ ρ / λ and the sphere's index
//! relative to the medium, m′ (his §13). The field outside is the incident wave plus a series of
//! outgoing partial waves, the electric ones weighted by Mie's a_ν and the magnetic ones by his
//! p_ν (his Eq. 55). The power they carry away and the power they take from the incident wave,
//! per unit incident intensity, are his §26's parts III and II:
//!
//! - scattering: C_sca = (λ′²/2π) Σ (|a_ν|² + |p_ν|²)/(2ν + 1);
//! - extinction: C_ext = (λ′²/2π) Im Σ (−1)^ν (a_ν − p_ν) (his Eq. 98);
//!
//! and absorption is their difference, λ′ = λ/n₀ the wavelength in the medium.
//!
//! **In photonoxide's convention.** Mie writes fields as e^(+iωt) times a complex amplitude, so
//! his metals have m′ = n − iκ, and he uses functions of his own: his I_ν(x) (Eq. 25) is the
//! Riccati–Bessel function ψ_ν(x) = x j_ν(x), and his outgoing K_ν(−x) (Eq. 19) is
//! (−i)^(ν+1) x h_ν⁽²⁾(x). Here, with e^(−iωt) and m = n + iκ, the outgoing function is
//! ξ_ν(x) = x h_ν⁽¹⁾(x) = ψ_ν(x) − iχ_ν(x), χ_ν(x) = −x y_ν(x), and his Eq. 55, divided through by
//! ψ_ν(mα), becomes
//!
//! a_ν = [(D_ν(mα)/m + ν/α) ψ_ν(α) − ψ_(ν−1)(α)] / [(D_ν(mα)/m + ν/α) ξ_ν(α) − ξ_(ν−1)(α)],
//!
//! b_ν = [(m D_ν(mα) + ν/α) ψ_ν(α) − ψ_(ν−1)(α)] / [(m D_ν(mα) + ν/α) ξ_ν(α) − ξ_(ν−1)(α)],
//!
//! with D_ν = ψ_ν′/ψ_ν. Mie's own coefficients are a_ν = −(2ν + 1)(−1)^ν i ā_ν and
//! p_ν = (2ν + 1)(−1)^ν i b̄_ν, the bar the complex conjugate with m conjugated. His sums become
//!
//! Q_sca = (2/α²) Σ (2ν + 1)(|a_ν|² + |b_ν|²),   Q_ext = (2/α²) Σ (2ν + 1) Re(a_ν + b_ν),
//!
//! the cross-sections divided by the sphere's shadow, πρ². A perfect conductor is the limit
//! m → ∞ (his §17): a_ν = (ν/α ψ_ν − ψ_(ν−1))/(ν/α ξ_ν − ξ_(ν−1)), b_ν = ψ_ν/ξ_ν.
//!
//! **The functions.** All of them follow from Mie's recurrence (26),
//! (2ν + 1) f_ν/x = f_(ν−1) + f_(ν+1), and its derivative. The outgoing ξ_ν, which grows with ν
//! past ν ≈ α, is stepped up from ξ₀ = sin α − i cos α and ξ₁. ψ_ν, which falls past ν ≈ α and
//! can't be stepped up, comes from its logarithmic derivative, stepped down,
//! D_(ν−1) = ν/z − 1/(D_ν + ν/z), from zero well above the last term (where any start is
//! forgotten), and then ψ_ν = ψ_(ν−1)/(D_ν + ν/z), up from ψ₀ = sin α. The sphere's own ψ_ν(mα)
//! never appears, only D_ν(mα), so a metal's exponentially large ψ_ν(mα) can't overflow.
//!
//! **The number of terms.** The terms fall faster than exponentially once ν passes α. The series
//! is summed until a term adds less than 10⁻¹⁷ of the sum, past ν = α; [`Mie::with_terms`]
//! keeps a given number, to test that convergence.

use num_complex::Complex64 as c64;

use crate::units::Frequency;
use crate::{Error, Result};

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("fdtd::mie", reason)
}

/// The most terms [`Mie::new`] sums: 2α + 60 of them, for α up to about 5·10⁴.
const MOST_TERMS: usize = 100_000;

/// Mie's coefficients for one sphere at one wavelength, and the efficiencies they give.
#[derive(Clone, Debug, PartialEq)]
pub struct Mie {
    /// The size parameter α = 2π n₀ ρ / λ.
    pub size: f64,
    /// The electric coefficients a₁, a₂, …, in photonoxide's convention (see the module docs).
    pub electric: Vec<c64>,
    /// The magnetic coefficients b₁, b₂, ….
    pub magnetic: Vec<c64>,
}

impl Mie {
    /// The coefficients for size parameter `size` = α and relative index `index` = m (`None`: a
    /// perfect conductor), as many as converge.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a size parameter that isn't positive and finite or is above
    /// about 5·10⁴, or an index that isn't a passive medium's (n ≥ 0 and κ ≥ 0, not both zero:
    /// a lossless metal is m = iκ).
    pub fn new(size: f64, index: Option<c64>) -> Result<Mie> {
        Mie::series(size, index, None)
    }

    /// The first `terms` coefficients only.
    ///
    /// # Errors
    ///
    /// As [`Mie::new`], and for no terms.
    pub fn with_terms(size: f64, index: Option<c64>, terms: usize) -> Result<Mie> {
        if terms == 0 {
            return Err(invalid("a series needs a term"));
        }
        Mie::series(size, index, Some(terms))
    }

    fn series(size: f64, index: Option<c64>, terms: Option<usize>) -> Result<Mie> {
        if !(size.is_finite() && size > 0.0) {
            return Err(invalid(format!(
                "the size parameter is {size}, not positive"
            )));
        }
        if let Some(m) = index
            && !(m.re.is_finite()
                && m.im.is_finite()
                && m.re >= 0.0
                && m.im >= 0.0
                && m != c64::new(0.0, 0.0))
        {
            return Err(invalid(format!(
                "the relative index {m} isn't a passive medium's (n ≥ 0, κ ≥ 0, not both 0)"
            )));
        }
        // the most terms this call can need: given, or past α where the terms collapse
        let most = match terms {
            Some(n) => n,
            None => {
                let most = (2.0 * size + 60.0).ceil() as usize;
                if most > MOST_TERMS {
                    return Err(invalid(format!("a size parameter of {size} is too large")));
                }
                most
            }
        };
        let x = c64::new(size, 0.0);
        let outside = log_derivatives(x, most);
        let inside = index.map(|m| (m, log_derivatives(m * x, most)));
        let mut electric = Vec::with_capacity(most);
        let mut magnetic = Vec::with_capacity(most);
        // ψ and ξ at orders ν − 1 and ν, from ν = 1
        let (sin, cos) = size.sin_cos();
        let mut psi_before = sin;
        let mut xi_before = c64::new(sin, -cos);
        let mut xi = c64::new(sin / size - cos, -(cos / size + sin));
        let mut sum = 0.0;
        let mut converged = false;
        for n in 1..=most {
            let nu = n as f64;
            let psi = psi_before / (outside[n].re + nu / size);
            let (a, b) = match &inside {
                Some((m, d)) => {
                    let ea = d[n] / *m + nu / size;
                    let eb = *m * d[n] + nu / size;
                    (
                        (ea * psi - psi_before) / (ea * xi - xi_before),
                        (eb * psi - psi_before) / (eb * xi - xi_before),
                    )
                }
                None => (
                    (nu / size * psi - psi_before) / (nu / size * xi - xi_before),
                    psi / xi,
                ),
            };
            electric.push(a);
            magnetic.push(b);
            let term = (2.0 * nu + 1.0) * (a.norm() + b.norm());
            sum += term;
            if terms.is_none() && nu > size && term <= 1e-17 * sum {
                converged = true;
                break;
            }
            let next = (2.0 * nu + 1.0) / size * xi - xi_before;
            psi_before = psi;
            xi_before = xi;
            xi = next;
        }
        if terms.is_none() && !converged {
            return Err(invalid(format!(
                "the series at α = {size} didn't converge in {most} terms"
            )));
        }
        Ok(Mie {
            size,
            electric,
            magnetic,
        })
    }

    /// The extinction efficiency, the extinction cross-section over πρ².
    pub fn extinction(&self) -> f64 {
        let sum: f64 = self.terms().map(|(order, a, b)| order * (a + b).re).sum();
        2.0 * sum / (self.size * self.size)
    }

    /// The scattering efficiency.
    pub fn scattering(&self) -> f64 {
        let sum: f64 = self
            .terms()
            .map(|(order, a, b)| order * (a.norm_sqr() + b.norm_sqr()))
            .sum();
        2.0 * sum / (self.size * self.size)
    }

    /// The absorption efficiency: extinction less scattering.
    pub fn absorption(&self) -> f64 {
        self.extinction() - self.scattering()
    }

    /// Mie's own coefficients a_ν and p_ν (his Eq. 55, in his convention e^(+iωt)), for
    /// comparison with his tables.
    pub fn mie_coefficients(&self) -> Vec<(c64, c64)> {
        self.electric
            .iter()
            .zip(&self.magnetic)
            .enumerate()
            .map(|(i, (a, b))| {
                let nu = (i + 1) as f64;
                let sign = if (i + 1) % 2 == 0 { 1.0 } else { -1.0 };
                let factor = c64::new(0.0, (2.0 * nu + 1.0) * sign);
                (factor * a.conj(), -factor * b.conj())
            })
            .collect()
    }

    /// (2ν + 1, a_ν, b_ν), in order.
    fn terms(&self) -> impl Iterator<Item = (f64, c64, c64)> + '_ {
        self.electric
            .iter()
            .zip(&self.magnetic)
            .enumerate()
            .map(|(i, (&a, &b))| (2.0 * (i + 1) as f64 + 1.0, a, b))
    }
}

/// D_ν(z) = ψ_ν′(z)/ψ_ν(z) for ν = 0, …, `most`, stepped down from zero well above both `most`
/// and |z|.
pub(super) fn log_derivatives(z: c64, most: usize) -> Vec<c64> {
    let start = most.max(z.norm().ceil() as usize) + 32;
    let mut d = vec![c64::new(0.0, 0.0); most + 1];
    let mut current = c64::new(0.0, 0.0);
    for n in (1..=start).rev() {
        let ratio = n as f64 / z;
        current = ratio - 1.0 / (current + ratio);
        if n - 1 <= most {
            d[n - 1] = current;
        }
    }
    d
}

/// The cross-sections of a sphere, in µm².
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossSections {
    /// The power taken from the incident wave over its intensity.
    pub extinction: f64,
    /// The power scattered.
    pub scattering: f64,
    /// The power absorbed.
    pub absorption: f64,
}

/// A sphere in a lossless medium, for Mie's series.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    /// The radius, µm.
    pub radius: f64,
    /// The sphere's relative permittivity (`None`: a perfect conductor).
    pub permittivity: Option<c64>,
    /// The medium's relative permittivity, real and positive.
    pub background: f64,
}

impl Sphere {
    /// Mie's coefficients at `frequency`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a radius or a medium's permittivity that isn't positive and
    /// finite, and as [`Mie::new`].
    pub fn mie(&self, frequency: Frequency) -> Result<Mie> {
        if !(self.background.is_finite() && self.background > 0.0) {
            return Err(invalid(format!(
                "the medium's permittivity is {}, not positive",
                self.background
            )));
        }
        if !(self.radius.is_finite() && self.radius > 0.0) {
            return Err(invalid(format!(
                "the radius is {}, not positive",
                self.radius
            )));
        }
        let medium = self.background.sqrt();
        let size = medium * self.radius * frequency.angular();
        let index = self
            .permittivity
            .map(|eps| crate::units::refractive_index(eps) / medium);
        Mie::new(size, index)
    }

    /// The extinction, scattering and absorption cross-sections at `frequency`, in µm².
    ///
    /// # Errors
    ///
    /// As [`Sphere::mie`].
    pub fn cross_sections(&self, frequency: Frequency) -> Result<CrossSections> {
        let mie = self.mie(frequency)?;
        let shadow = std::f64::consts::PI * self.radius * self.radius;
        Ok(CrossSections {
            extinction: shadow * mie.extinction(),
            scattering: shadow * mie.scattering(),
            absorption: shadow * mie.absorption(),
        })
    }
}
