//! Two-photon interference with spectra: the Hong–Ou–Mandel dip (C. K. Hong, Z. Y. Ou,
//! L. Mandel, Phys. Rev. Lett. 59, 2044 (1987), doi:10.1103/PhysRevLett.59.2044) through a
//! frequency-dependent S(ω), from a circuit's own sweep.
//!
//! Photon 1, with spectral amplitude φ₁(ω), enters port i₁ delayed by τ; photon 2, φ₂(ω),
//! enters port i₂. The two-photon amplitude to find one at port o₁ with frequency ω and one at
//! o₂ with ω′ is
//!
//! f(ω, ω′) = A(ω) B(ω′) + D(ω) C(ω′),
//!
//! with A = φ₁ e^(iωτ) S_o₁i₁, B = φ₂ S_o₂i₂, C = φ₁ e^(iωτ) S_o₂i₁ and D = φ₂ S_o₁i₂, and
//! the coincidence probability ∬ |f|² dω dω′ separates into single integrals:
//!
//! P(τ) = ∫|A|² ∫|B|² + ∫|C|² ∫|D|² + 2 Re(∫A D̄ ∫B C̄),
//!
//! the last term the interference. Each is a trapezoidal sum over the sweep's frequencies,
//! which need not be evenly spaced; the photons' spectra are normalized on the same grid.

use std::f64::consts::TAU;

use num_complex::Complex64 as c64;

use crate::circuit::Spectrum;
use crate::units::{SPEED_OF_LIGHT, Wavelength};
use crate::{Error, Result};

/// A photon's spectral amplitude φ(ω).
#[derive(Clone, Debug, PartialEq)]
pub enum Wavepacket {
    /// A Gaussian: |φ(ω)|² ∝ exp(−(ω − ω₀)²/(2σ²)), flat phase (transform-limited).
    Gaussian {
        /// ω₀, rad/s.
        center: f64,
        /// σ, rad/s: the standard deviation of |φ(ω)|².
        width: f64,
    },
    /// Sampled amplitudes at angular frequencies (rad/s, increasing), linearly interpolated,
    /// zero outside.
    Sampled(Vec<(f64, c64)>),
}

impl Wavepacket {
    /// A transform-limited Gaussian photon centred at `center` with a full width at half
    /// maximum of `fwhm_nm` in its power spectrum, in wavelength.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a width that isn't positive and finite.
    pub fn gaussian(center: Wavelength, fwhm_nm: f64) -> Result<Wavepacket> {
        if !(fwhm_nm.is_finite() && fwhm_nm > 0.0) {
            return Err(Error::invalid(
                "wavepacket",
                format!("its width must be positive, got {fwhm_nm} nm"),
            ));
        }
        let lambda = center.to_um() * 1e-6;
        let omega = TAU * SPEED_OF_LIGHT / lambda;
        // Δω = ω Δλ/λ, and a Gaussian's FWHM is 2√(2 ln 2) σ
        let fwhm_omega = omega * fwhm_nm * 1e-9 / lambda;
        Ok(Wavepacket::Gaussian {
            center: omega,
            width: fwhm_omega / (8.0 * 2f64.ln()).sqrt(),
        })
    }

    /// φ(ω), unnormalized.
    pub fn amplitude(&self, omega: f64) -> c64 {
        match self {
            Wavepacket::Gaussian { center, width } => {
                let x = (omega - center) / width;
                c64::new((-x * x / 4.0).exp(), 0.0)
            }
            Wavepacket::Sampled(points) => {
                let k = points.partition_point(|&(w, _)| w <= omega);
                if k == 0 || k == points.len() {
                    return c64::new(0.0, 0.0);
                }
                let ((w0, a0), (w1, a1)) = (points[k - 1], points[k]);
                let s = (omega - w0) / (w1 - w0);
                a0 * (1.0 - s) + a1 * s
            }
        }
    }
}

/// A Hong–Ou–Mandel dip: the coincidence probability at each delay, and the probability for
/// distinguishable photons, which a long delay reaches.
#[derive(Clone, Debug, PartialEq)]
pub struct Dip {
    /// The delays τ of photon 1, s.
    pub delays: Vec<f64>,
    /// The coincidence probability at each delay.
    pub coincidences: Vec<f64>,
    /// The coincidence probability without interference: distinguishable photons.
    pub distinguishable: f64,
}

impl Dip {
    /// The visibility, 1 − min P(τ)/P_distinguishable, over the delays computed.
    pub fn visibility(&self) -> f64 {
        let low = self
            .coincidences
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        1.0 - low / self.distinguishable
    }
}

/// The sweep's angular frequencies and the photons' normalized amplitudes on them, sorted by
/// frequency, with each S-matrix's index.
struct Grid {
    omega: Vec<f64>,
    order: Vec<usize>,
    phi: [Vec<c64>; 2],
}

fn grid(spectrum: &Spectrum, photons: [&Wavepacket; 2]) -> Result<Grid> {
    let n = spectrum.wavelengths().len();
    if n < 2 {
        return Err(Error::invalid(
            "two-photon interference",
            "needs a sweep of at least two wavelengths",
        ));
    }
    let mut order: Vec<usize> = (0..n).collect();
    let omega_of = |k: usize| TAU * SPEED_OF_LIGHT / (spectrum.wavelengths()[k].to_um() * 1e-6);
    order.sort_by(|&a, &b| omega_of(a).total_cmp(&omega_of(b)));
    let omega: Vec<f64> = order.iter().map(|&k| omega_of(k)).collect();
    let phi = photons.map(|p| {
        let raw: Vec<c64> = omega.iter().map(|&w| p.amplitude(w)).collect();
        let norm = trapezoid(&omega, |i| c64::new(raw[i].norm_sqr(), 0.0)).re;
        raw.iter().map(|a| a / norm.sqrt()).collect::<Vec<c64>>()
    });
    if phi.iter().flatten().any(|a| !a.re.is_finite()) {
        return Err(Error::invalid(
            "two-photon interference",
            "a photon's spectrum is zero over the sweep",
        ));
    }
    Ok(Grid { omega, order, phi })
}

/// ∫ f dω by the trapezoidal rule on the grid ω.
fn trapezoid(omega: &[f64], f: impl Fn(usize) -> c64) -> c64 {
    (1..omega.len())
        .map(|i| (f(i - 1) + f(i)) * (0.5 * (omega[i] - omega[i - 1])))
        .sum()
}

/// The coincidence probability between ports `outputs` for photon 1 in `inputs[0]` delayed by
/// `delay` (s) and photon 2 in `inputs[1]`, through the circuit's swept S(ω).
///
/// # Errors
///
/// [`Error::InvalidValue`] for a port name the spectrum doesn't have, equal output ports, a
/// sweep of fewer than two wavelengths, or a photon whose spectrum is zero on the sweep.
pub fn coincidence(
    spectrum: &Spectrum,
    inputs: [&str; 2],
    outputs: [&str; 2],
    photons: [&Wavepacket; 2],
    delay: f64,
) -> Result<f64> {
    Ok(hom_dip(spectrum, inputs, outputs, photons, &[delay])?.coincidences[0])
}

/// The coincidence probability at each of `delays`, as [`coincidence`], and without
/// interference.
///
/// # Errors
///
/// As [`coincidence`].
pub fn hom_dip(
    spectrum: &Spectrum,
    inputs: [&str; 2],
    outputs: [&str; 2],
    photons: [&Wavepacket; 2],
    delays: &[f64],
) -> Result<Dip> {
    let port = |name: &str| {
        spectrum.port(name).ok_or_else(|| {
            Error::invalid(
                "two-photon interference",
                format!(
                    "no port {name}; the ports are {}",
                    spectrum.ports().join(", ")
                ),
            )
        })
    };
    let (i1, i2, o1, o2) = (
        port(inputs[0])?,
        port(inputs[1])?,
        port(outputs[0])?,
        port(outputs[1])?,
    );
    if o1 == o2 {
        return Err(Error::invalid(
            "two-photon interference",
            "a coincidence needs two different output ports",
        ));
    }
    let g = grid(spectrum, photons)?;
    let s = |q: usize, p: usize, i: usize| spectrum.matrices()[g.order[i]][(q, p)];
    let w = &g.omega;
    // without the delay: photon 1's terms carry e^(iωτ), which cancels in |A|², |C|²
    let a = |i: usize| g.phi[0][i] * s(o1, i1, i);
    let b = |i: usize| g.phi[1][i] * s(o2, i2, i);
    let c = |i: usize| g.phi[0][i] * s(o2, i1, i);
    let d = |i: usize| g.phi[1][i] * s(o1, i2, i);
    let norm2 = |f: &dyn Fn(usize) -> c64| trapezoid(w, |i| c64::new(f(i).norm_sqr(), 0.0)).re;
    let distinguishable = norm2(&a) * norm2(&b) + norm2(&c) * norm2(&d);
    let coincidences = delays
        .iter()
        .map(|&tau| {
            let phase = |i: usize| c64::from_polar(1.0, w[i] * tau);
            let ad = trapezoid(w, |i| a(i) * phase(i) * d(i).conj());
            let bc = trapezoid(w, |i| b(i) * (c(i) * phase(i)).conj());
            distinguishable + 2.0 * (ad * bc).re
        })
        .collect();
    Ok(Dip {
        delays: delays.to_vec(),
        coincidences,
        distinguishable,
    })
}
