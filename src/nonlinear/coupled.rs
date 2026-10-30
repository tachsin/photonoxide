//! The coupled-amplitude equations of second-order processes along a guide: second-harmonic
//! generation, and sum- and difference-frequency generation (with parametric amplification),
//! depleted or not, with a mismatch and propagation loss.
//!
//! # Conventions
//!
//! Those of [`super::phase`], and:
//!
//! - **Amplitudes in √W.** Each wave's complex amplitude A_j has |A_j|² = its power. The fields
//!   are A_j e^{i(k_j z − ω_j t)}, so a mismatch Δk = k₃ − k₁ − k₂ − K_m (the module's sign)
//!   appears as e^{∓iΔkz} on the right-hand sides.
//! - **The coupling κ**, in 1/(√W·µm), is set by the normalized efficiency η = κ² of the
//!   undepleted process: P_2ω = η P_ω² L² sinc²(ΔkL/2) for SHG, P₃ = η P₁ P₂ L² sinc²(ΔkL/2)
//!   for sum-frequency generation. A guide's η in %/(W·cm²) converts with
//!   [`coupling_from_efficiency`]. Its overlap with the mode solver's fields comes next.
//! - **Loss** α_j is the power's attenuation, 1/µm (each amplitude decays as e^{−α_j z/2});
//!   [`loss_from_db_per_cm`] converts.
//! - **SHG**, with the fundamental's amplitude A₁ and the harmonic's A₂:
//!   dA₁/dz = −iκ A₁* A₂ e^{iΔkz} − α₁A₁/2, dA₂/dz = −iκ A₁² e^{−iΔkz} − α₂A₂/2. Without loss
//!   P₁ + P₂ is conserved; at Δk = 0, P₂ = P₀ tanh²(κ√P₀ z) (Armstrong et al. 1962, Sec. VI:
//!   u₃ = u₁(0) tanh(z/l)).
//! - **Three waves**, ω₃ = ω₁ + ω₂ with frequencies ν_j = 1/λ_j:
//!   dA₁/dz = −iκ (ν₁/ν₃) A₂* A₃ e^{iΔkz}, dA₂/dz = −iκ (ν₂/ν₃) A₁* A₃ e^{iΔkz},
//!   dA₃/dz = −iκ A₁A₂ e^{−iΔkz}, each with its −α_jA_j/2. Without loss the Manley–Rowe
//!   relations hold (Armstrong et al., Eq. (6.6)): P₁/ν₁ + P₃/ν₃ and P₂/ν₂ + P₃/ν₃ (photon
//!   fluxes) are conserved, and so is P₁ + P₂ + P₃.
//! - **Integration** is the classical fourth-order Runge–Kutta method on `steps` equal steps; its
//!   error falls as the step's fourth power (a validation case checks it).
//!
//! J. A. Armstrong, N. Bloembergen, J. Ducuing, P. S. Pershan, Phys. Rev. 127, 1918 (1962),
//! doi:10.1103/PhysRev.127.1918.

use num_complex::Complex64 as c64;

use crate::{Error, Result};

/// κ (1/(√W·µm)) from a normalized efficiency η in %/(W·cm²): η = κ², and
/// 1 %/(W·cm²) = 10⁻¹⁰ /(W·µm²).
pub fn coupling_from_efficiency(percent_per_watt_cm2: f64) -> f64 {
    (percent_per_watt_cm2 * 1e-10).sqrt()
}

/// A power loss α (1/µm) from dB/cm: α = (ln 10/10)·dB/cm ÷ 10⁴.
pub fn loss_from_db_per_cm(db_per_cm: f64) -> f64 {
    db_per_cm * std::f64::consts::LN_10 / 10.0 / 1e4
}

/// Second-harmonic generation along a guide: its coupling, mismatch and losses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SecondHarmonic {
    /// κ, 1/(√W·µm): η = κ².
    pub coupling: f64,
    /// Δk = k_2ω − 2k_ω − K_m, rad/µm.
    pub mismatch: f64,
    /// The power losses of the fundamental and the harmonic, 1/µm.
    pub loss: [f64; 2],
}

impl SecondHarmonic {
    /// The amplitudes (√W) of the fundamental and the harmonic after `length` µm from
    /// `start`, by `steps` steps of RK4.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a length that isn't positive and finite, or no steps.
    pub fn propagate(&self, start: [c64; 2], length: f64, steps: usize) -> Result<[c64; 2]> {
        let (k, dk, [a1, a2]) = (self.coupling, self.mismatch, self.loss);
        let i = c64::new(0.0, 1.0);
        let f = |z: f64, a: &[c64; 2]| {
            let e = c64::from_polar(1.0, dk * z);
            [
                -i * k * a[0].conj() * a[1] * e - 0.5 * a1 * a[0],
                -i * k * a[0] * a[0] * e.conj() - 0.5 * a2 * a[1],
            ]
        };
        rk4(f, start, length, steps)
    }
}

/// Sum- and difference-frequency generation ω₃ = ω₁ + ω₂ along a guide (also parametric
/// amplification): the frequencies, coupling, mismatch and losses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThreeWave {
    /// ν₁ and ν₂ (1/µm, the reciprocal vacuum wavelengths); ν₃ is their sum.
    pub frequencies: [f64; 2],
    /// κ, 1/(√W·µm): the undepleted sum's η = κ².
    pub coupling: f64,
    /// Δk = k₃ − k₁ − k₂ − K_m, rad/µm.
    pub mismatch: f64,
    /// The power losses of waves 1, 2 and 3, 1/µm.
    pub loss: [f64; 3],
}

impl ThreeWave {
    /// The amplitudes (√W) of waves 1, 2 and 3 after `length` µm from `start`, by `steps`
    /// steps of RK4.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a length that isn't positive and finite, no steps, or
    /// frequencies that aren't positive.
    pub fn propagate(&self, start: [c64; 3], length: f64, steps: usize) -> Result<[c64; 3]> {
        let [n1, n2] = self.frequencies;
        if !(n1 > 0.0 && n2 > 0.0) {
            return Err(Error::invalid(
                "process",
                "the frequencies must be positive",
            ));
        }
        let n3 = n1 + n2;
        let (k, dk, [a1, a2, a3]) = (self.coupling, self.mismatch, self.loss);
        let i = c64::new(0.0, 1.0);
        let f = |z: f64, a: &[c64; 3]| {
            let e = c64::from_polar(1.0, dk * z);
            [
                -i * k * (n1 / n3) * a[1].conj() * a[2] * e - 0.5 * a1 * a[0],
                -i * k * (n2 / n3) * a[0].conj() * a[2] * e - 0.5 * a2 * a[1],
                -i * k * a[0] * a[1] * e.conj() - 0.5 * a3 * a[2],
            ]
        };
        rk4(f, start, length, steps)
    }

    /// The Manley–Rowe photon fluxes P₁/ν₁ + P₃/ν₃ and P₂/ν₂ + P₃/ν₃ (W·µm) of the amplitudes
    /// `a`: constant along a lossless guide.
    pub fn manley_rowe(&self, a: &[c64; 3]) -> [f64; 2] {
        let [n1, n2] = self.frequencies;
        let n3 = n1 + n2;
        let p = a.map(|x| x.norm_sqr());
        [p[0] / n1 + p[2] / n3, p[1] / n2 + p[2] / n3]
    }
}

/// The classical fourth-order Runge–Kutta method for y′ = f(z, y) from z = 0 to `length`.
fn rk4<const N: usize>(
    f: impl Fn(f64, &[c64; N]) -> [c64; N],
    start: [c64; N],
    length: f64,
    steps: usize,
) -> Result<[c64; N]> {
    if !(length.is_finite() && length > 0.0) || steps == 0 {
        return Err(Error::invalid(
            "propagation",
            format!("needs a positive length and steps, got {length} um and {steps}"),
        ));
    }
    let h = length / steps as f64;
    let mut y = start;
    let add = |y: &[c64; N], k: &[c64; N], s: f64| -> [c64; N] {
        std::array::from_fn(|j| y[j] + k[j] * s)
    };
    for n in 0..steps {
        let z = n as f64 * h;
        let k1 = f(z, &y);
        let k2 = f(z + h / 2.0, &add(&y, &k1, h / 2.0));
        let k3 = f(z + h / 2.0, &add(&y, &k2, h / 2.0));
        let k4 = f(z + h, &add(&y, &k3, h));
        y = std::array::from_fn(|j| y[j] + (k1[j] + 2.0 * k2[j] + 2.0 * k3[j] + k4[j]) * (h / 6.0));
    }
    Ok(y)
}

/// The harmonic's power of lossy undepleted SHG in closed form, W: the field generated at z
/// from the fundamental's e^{−α₁z/2}², carried to L with e^{−α₂(L−z)/2},
/// P₂ = η P₀² |e^{−α₂L/2} (e^{γL} − 1)/γ|², γ = −α₁ + α₂/2 − iΔk (L² sinc²(ΔkL/2) when
/// lossless).
pub fn undepleted_lossy_shg(shg: &SecondHarmonic, p0: f64, length: f64) -> f64 {
    let [a1, a2] = shg.loss;
    let g = c64::new(-a1 + a2 / 2.0, -shg.mismatch);
    let integral = if g.norm() * length < 1e-8 {
        c64::new(length, 0.0)
    } else {
        ((g * length).exp() - 1.0) / g
    };
    shg.coupling * shg.coupling * p0 * p0 * (-a2 * length).exp() * integral.norm_sqr()
}

#[cfg(test)]
#[path = "coupled_tests.rs"]
mod tests;
