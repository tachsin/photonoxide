//! Many photons with spectra through a frequency-dependent S(ω): the output patterns' probabilities
//! with detectors that count photons without telling their frequencies apart.
//!
//! Photon a, with spectral amplitude φ_a(ω) and delay τ_a, enters port t_a. Through the circuit's
//! swept S(ω) it leaves port q with amplitude ψ_aq(ω) = φ_a(ω) e^(iωτ_a) S_q,t_a(ω) (and into the
//! loss modes with the rest). A pattern R, photons at ports r_1 … r_n (port r listed m_r times),
//! has probability
//!
//! P(R) = (1/(N ∏_r m_r!)) Σ_σ Σ_ρ ∏_j G_j(σ(j), ρ(j)),   G_j(a, b) = ∫ ψ_a,r_j ψ̄_b,r_j dω,
//!
//! over permutations σ and ρ of the photons, N = Σ_σ ∏_a [t_σ(a) = t_a] ⟨φ_a|φ_σ(a)⟩ the input
//! state's norm (1 when every photon has a port of its own). The sum over ρ for each σ is the
//! permanent of the matrix M^σ_jb = G_j(σ(j), b), so P(R) costs n! permanents of n × n. With a
//! frequency-independent S it is Q1's partly distinguishable probability ([`super::probability`]
//! with [`super::Photons::Overlaps`]); with two photons, the Hong–Ou–Mandel coincidence
//! ([`super::hom_dip`]). docs/methods/quantum.md derives it.
//!
//! The integrals are trapezoidal sums over the sweep's frequencies (in any order, sorted here), as
//! [`super::hom_dip`] takes them, the photons' spectra normalized on the same grid.

use std::f64::consts::TAU;

use num_complex::Complex64 as c64;

use super::fock::{Distribution, Fock, patterns, permutations};
use super::hom::{Wavepacket, trapezoid};
use super::matrix::Matrix;
use super::permanent::permanent;
use crate::circuit::Spectrum;
use crate::units::SPEED_OF_LIGHT;
use crate::{Error, Result};

/// The most photons the sum over permutations takes: 8! permanents of 8 × 8.
const MOST: usize = 8;

/// A photon on a port: its spectrum and its delay.
#[derive(Clone, Debug, PartialEq)]
pub struct SpectralPhoton {
    /// The port it enters by.
    pub port: String,
    /// Its spectral amplitude.
    pub wavepacket: Wavepacket,
    /// Its delay τ, s: its amplitude carries e^(iωτ).
    pub delay: f64,
}

impl SpectralPhoton {
    /// A photon with `wavepacket` entering `port`, not delayed.
    pub fn new(port: &str, wavepacket: Wavepacket) -> SpectralPhoton {
        SpectralPhoton {
            port: port.to_string(),
            wavepacket,
            delay: 0.0,
        }
    }

    /// The same photon delayed by `delay` s.
    #[must_use]
    pub fn delayed(mut self, delay: f64) -> SpectralPhoton {
        self.delay = delay;
        self
    }
}

/// Photons with spectra through a swept S(ω): each photon's amplitude at each port and
/// frequency, ready for any output pattern.
#[derive(Clone, Debug)]
pub struct SpectralInput {
    ports: Vec<String>,
    /// The sweep's angular frequencies, increasing, rad/s.
    omega: Vec<f64>,
    /// ψ[a][q][i]: photon a at port q at frequency i.
    psi: Vec<Vec<Vec<c64>>>,
    /// The input state's norm N.
    norm: f64,
}

impl SpectralInput {
    /// The photons `photons` through the S(ω) of `spectrum`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a port the spectrum doesn't have, no photon or more than 8, a
    /// sweep of fewer than two wavelengths, or a photon whose spectrum is zero on the sweep.
    pub fn new(spectrum: &Spectrum, photons: &[SpectralPhoton]) -> Result<SpectralInput> {
        let n = photons.len();
        if n == 0 || n > MOST {
            return Err(invalid(format!(
                "takes 1 to {MOST} photons (a sum over n! permutations), got {n}"
            )));
        }
        let count = spectrum.wavelengths().len();
        if count < 2 {
            return Err(invalid("needs a sweep of at least two wavelengths"));
        }
        let ports = spectrum.ports().to_vec();
        let inputs = photons
            .iter()
            .map(|p| {
                spectrum.port(&p.port).ok_or_else(|| {
                    invalid(format!(
                        "no port {}; the ports are {}",
                        p.port,
                        ports.join(", ")
                    ))
                })
            })
            .collect::<Result<Vec<usize>>>()?;
        let omega_of = |k: usize| TAU * SPEED_OF_LIGHT / (spectrum.wavelengths()[k].to_um() * 1e-6);
        let mut order: Vec<usize> = (0..count).collect();
        order.sort_by(|&a, &b| omega_of(a).total_cmp(&omega_of(b)));
        let omega: Vec<f64> = order.iter().map(|&k| omega_of(k)).collect();
        // each photon's normalized amplitude with its delay's phase
        let mut phi = Vec::with_capacity(n);
        for p in photons {
            let raw: Vec<c64> = omega.iter().map(|&w| p.wavepacket.amplitude(w)).collect();
            let norm = trapezoid(&omega, |i| c64::new(raw[i].norm_sqr(), 0.0)).re;
            if !(norm.is_finite() && norm > 0.0) {
                return Err(invalid("a photon's spectrum is zero over the sweep"));
            }
            let scale = 1.0 / norm.sqrt();
            phi.push(
                raw.iter()
                    .zip(&omega)
                    .map(|(a, &w)| a * scale * c64::from_polar(1.0, w * p.delay))
                    .collect::<Vec<c64>>(),
            );
        }
        let matrices = spectrum.matrices();
        let psi = (0..n)
            .map(|a| {
                (0..ports.len())
                    .map(|q| {
                        (0..count)
                            .map(|i| phi[a][i] * matrices[order[i]][(q, inputs[a])])
                            .collect()
                    })
                    .collect()
            })
            .collect();
        // the input's norm: the permutations that keep every photon on its port
        let mut norm = 0.0;
        for sigma in permutations(n) {
            if (0..n).all(|a| inputs[sigma[a]] == inputs[a]) {
                let product: c64 = (0..n)
                    .map(|a| trapezoid(&omega, |i| phi[a][i] * phi[sigma[a]][i].conj()))
                    .product();
                norm += product.re;
            }
        }
        Ok(SpectralInput {
            ports,
            omega,
            psi,
            norm,
        })
    }

    /// The number of photons.
    pub fn photons(&self) -> usize {
        self.psi.len()
    }

    /// The ports, in the spectrum's order.
    pub fn ports(&self) -> &[String] {
        &self.ports
    }

    /// G_q(a, b) = ∫ ψ_aq ψ̄_bq dω for every pair of photons at port q.
    fn gram(&self, q: usize) -> Matrix {
        let n = self.photons();
        Matrix::from_fn(n, n, |a, b| {
            trapezoid(&self.omega, |i| {
                self.psi[a][q][i] * self.psi[b][q][i].conj()
            })
        })
    }

    /// The probability that every photon is detected and the ports count `output`
    /// (one occupation per port).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `output` isn't over the ports or doesn't hold every photon.
    pub fn probability(&self, output: &Fock) -> Result<f64> {
        let n = self.photons();
        if output.modes() != self.ports.len() || output.photons() != n {
            return Err(invalid(format!(
                "a pattern of {n} photons over {} ports, got {} over {}",
                self.ports.len(),
                output.photons(),
                output.modes()
            )));
        }
        let slots = output.mode_list();
        let grams: Vec<Matrix> = (0..self.ports.len()).map(|q| self.gram(q)).collect();
        let mut sum = c64::new(0.0, 0.0);
        for sigma in permutations(n) {
            let m = Matrix::from_fn(n, n, |j, b| grams[slots[j]][(sigma[j], b)]);
            sum += permanent(&m)?;
        }
        Ok(sum.re / (self.norm * output.factorials()))
    }

    /// Every pattern with all the photons detected at the ports, each with its probability, on
    /// rayon's threads collected in order. Its total is the chance that none was lost: 1 for a
    /// lossless circuit.
    ///
    /// # Errors
    ///
    /// As [`SpectralInput::probability`].
    pub fn distribution(&self) -> Result<Distribution> {
        let all = patterns(self.ports.len(), self.photons());
        let probabilities = crate::parallel::map_in_order(&all, |p| self.probability(p))
            .into_iter()
            .collect::<Result<Vec<f64>>>()?;
        Ok(Distribution::from_parts(all, probabilities))
    }
}

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("photons with spectra", reason)
}
