//! A 2D dielectric ring's resonances, exactly: the complex frequencies at which a ring of
//! permittivity ε between radii a and b, in vacuum, holds a field with no incoming wave.
//!
//! With E along the ring's axis (E_z, the field everywhere tangential to the ring's faces),
//! E_z = u(r) e^(imφ) e^(−iωt) solves Bessel's equation of order m in each region, with k = ω
//! outside (c = 1) and nk inside the ring, n = √ε:
//!
//! - inside the hole, u = A J_m(kr), finite at the centre;
//! - in the ring, u = B J_m(nkr) + C Y_m(nkr);
//! - outside, u = D H⁽¹⁾_m(kr), outgoing for e^(−iωt).
//!
//! E_z and ∂E_z/∂r are continuous at r = a and r = b (H_φ is). Eliminating A and D leaves two
//! equations in B and C, whose determinant
//!
//! [n J′_m(nka) J_m(ka) − J′_m(ka) J_m(nka)] [n Y′_m(nkb) H_m(kb) − H′_m(kb) Y_m(nkb)]
//! − [n Y′_m(nka) J_m(ka) − J′_m(ka) Y_m(nka)] [n J′_m(nkb) H_m(kb) − H′_m(kb) J_m(nkb)]
//!
//! vanishes at a resonance (each row multiplied through by the outer function, so that it has
//! no poles): ω = ω_r − iγ with γ > 0, the field decaying in time as e^(−γt), and Q = ω_r/2γ.
//! The roots are found by the secant method from a guess.
//!
//! **The functions.** J_m and Y_m of integer order and complex argument by their power series
//! (M. Abramowitz, I. A. Stegun, *Handbook of Mathematical Functions*, NBS (1964), Eqs. 9.1.10
//! and 9.1.11), the digamma function at integers ψ(k + 1) = −γ_E + Σ 1/j; the derivatives from
//! J′_m = J_(m−1) − (m/z) J_m (Eq. 9.1.27), J′_0 = −J_1. The series lose about e^|z|/(2π|z|)
//! of their precision to cancellation, so arguments are kept to |z| ≤ 15, where it is below
//! about 1e-10; the Wronskian J_m Y′_m − J′_m Y_m = 2/(πz) (Eq. 9.1.16) checks them.

use num_complex::Complex64 as c64;

use crate::{Error, Result};

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("fdtd::ring", reason)
}

/// Euler's constant γ_E.
const EULER: f64 = std::f64::consts::EULER_GAMMA;

/// The largest |z| the series are summed at.
const LARGEST: f64 = 15.0;

/// J_m(z) and Y_m(z) for integer m ≥ 0, by their power series.
pub(crate) fn bessel(m: u32, z: c64) -> (c64, c64) {
    let half = z / 2.0;
    let quarter = -half * half;
    // J_m: Σ (−z²/4)^k (z/2)^m / (k! (m + k)!), with the digamma-weighted sum for Y_m alongside
    let lead = (0..m).fold(c64::new(1.0, 0.0), |p, j| p * half / f64::from(j + 1));
    let (mut j, mut weighted) = (c64::new(0.0, 0.0), c64::new(0.0, 0.0));
    let mut term = lead;
    // ψ(k + 1) and ψ(m + k + 1)
    let mut psi_k = -EULER;
    let mut psi_mk = -EULER + (1..=m).map(|i| 1.0 / f64::from(i)).sum::<f64>();
    let mut k = 0u32;
    loop {
        j += term;
        weighted += term * (psi_k + psi_mk);
        k += 1;
        term *= quarter / (f64::from(k) * f64::from(m + k));
        psi_k += 1.0 / f64::from(k);
        psi_mk += 1.0 / f64::from(m + k);
        if term.norm() <= 1e-18 * j.norm().max(lead.norm()) && f64::from(k) > half.norm() {
            break;
        }
    }
    // Y_m = −(1/π) Σ_{k<m} ((m−k−1)!/k!) (z/2)^(2k−m) + (2/π) ln(z/2) J_m − (1/π) Σ weighted
    let mut finite = c64::new(0.0, 0.0);
    if m > 0 {
        // the k-th term is ((m − k − 1)!/k!) (z²/4)^k (z/2)^(−m)
        // (m − 1)! (z/2)^(−m), with lead = (z/2)^m / m!
        let mut t = 1.0 / (f64::from(m) * lead);
        for k in 0..m {
            finite += t;
            if k + 1 < m {
                t *= half * half / (f64::from(k + 1) * f64::from(m - k - 1));
            }
        }
    }
    let pi = std::f64::consts::PI;
    let y = -finite / pi + 2.0 / pi * half.ln() * j - weighted / pi;
    (j, y)
}

/// J_m, Y_m and their derivatives at z.
fn with_derivatives(m: u32, z: c64) -> [c64; 4] {
    let (j, y) = bessel(m, z);
    let (dj, dy) = if m == 0 {
        let (j1, y1) = bessel(1, z);
        (-j1, -y1)
    } else {
        let (jm, ym) = bessel(m - 1, z);
        let order = f64::from(m) / z;
        (jm - order * j, ym - order * y)
    };
    [j, dj, y, dy]
}

/// A dielectric ring in vacuum, uniform along its axis, for E along that axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Ring {
    /// The inner radius, µm.
    pub(crate) inner: f64,
    /// The outer radius, µm.
    pub(crate) outer: f64,
    /// The ring's relative permittivity.
    pub(crate) eps: f64,
}

impl Ring {
    /// A ring between radii `inner` and `outer` (µm) of permittivity `eps`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless 0 < inner < outer and the permittivity is finite and
    /// positive.
    pub(crate) fn new(inner: f64, outer: f64, eps: f64) -> Result<Ring> {
        if !(inner > 0.0 && outer > inner && outer.is_finite() && eps.is_finite() && eps > 0.0) {
            return Err(invalid(format!(
                "a ring needs 0 < inner < outer and a positive permittivity, got {inner}, \
                 {outer} and {eps}"
            )));
        }
        Ok(Ring { inner, outer, eps })
    }

    /// The determinant whose zeros are the resonances of order `m`, at the complex angular
    /// frequency `omega` (rad/µm, with c = 1).
    fn determinant(&self, m: u32, omega: c64) -> c64 {
        let n = self.eps.sqrt();
        let (a, b) = (self.inner, self.outer);
        let [ja, dja, _, _] = with_derivatives(m, omega * a);
        let [jna, djna, yna, dyna] = with_derivatives(m, n * omega * a);
        let [jnb, djnb, ynb, dynb] = with_derivatives(m, n * omega * b);
        let [jb, djb, yb, dyb] = with_derivatives(m, omega * b);
        let (h, dh) = (jb + c64::i() * yb, djb + c64::i() * dyb);
        let first = [n * djna * ja - dja * jna, n * dyna * ja - dja * yna];
        let second = [n * djnb * h - dh * jnb, n * dynb * h - dh * ynb];
        first[0] * second[1] - first[1] * second[0]
    }

    /// The resonance of azimuthal order `m` nearest `guess`, a complex frequency (c/µm): Re the
    /// frequency, −Im the decay rate over 2π. Returns ω/2π = f − iγ/2π.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the guess's arguments are beyond the series' range
    /// (n|ω|b ≤ 15) or the secant iteration doesn't converge.
    pub(crate) fn resonance(&self, m: u32, guess: c64) -> Result<c64> {
        let tau = std::f64::consts::TAU;
        let reach = |omega: c64| self.eps.sqrt() * omega.norm() * self.outer;
        let mut x0 = guess * tau;
        let mut x1 = x0 * (1.0 + 1e-4);
        let mut f0 = self.determinant(m, x0);
        for _ in 0..100 {
            if reach(x1) > LARGEST {
                return Err(invalid(format!(
                    "a resonance at {} c/µm is beyond the series' range",
                    x1 / tau
                )));
            }
            let f1 = self.determinant(m, x1);
            if f1 == f0 {
                break;
            }
            let x2 = x1 - f1 * (x1 - x0) / (f1 - f0);
            (x0, f0, x1) = (x1, f1, x2);
            if (x1 - x0).norm() <= 1e-14 * x1.norm() {
                return Ok(x1 / tau);
            }
        }
        Err(invalid(format!(
            "no resonance of order {m} converged near {guess} c/µm"
        )))
    }
}

/// The largest relative error of the Wronskian J_m Y′_m − J′_m Y_m = 2/(πz) for orders 0 to
/// 11 at complex arguments across |z| ≤ 15.
pub(crate) fn wronskian_error() -> f64 {
    let mut worst = 0.0f64;
    for m in 0..12 {
        for &(re, im) in &[
            (0.3, 0.0),
            (2.0, -0.1),
            (7.5, -0.4),
            (13.0, -0.05),
            (14.8, 0.2),
        ] {
            let z = c64::new(re, im);
            let [j, dj, y, dy] = with_derivatives(m, z);
            let exact = 2.0 / (std::f64::consts::PI * z);
            worst = worst.max(((j * dy - dj * y - exact) / exact).norm());
        }
    }
    worst
}

/// A resonance's Q from its complex frequency f − iγ/2π: ω_r/2γ.
pub(crate) fn ring_q(frequency: c64) -> f64 {
    -frequency.re / (2.0 * frequency.im)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values of J and Y at 1 and 2 (Abramowitz and Stegun's Table 9.1, to its 15 digits).
    #[test]
    fn bessel_functions_match_tabulated_values() {
        let cases = [
            (0, 1.0, 0.765_197_686_557_967, 0.088_256_964_215_677),
            (1, 1.0, 0.440_050_585_744_934, -0.781_212_821_300_289),
            (0, 2.0, 0.223_890_779_141_236, 0.510_375_672_649_745),
        ];
        for (m, x, j, y) in cases {
            let (jm, ym) = bessel(m, c64::new(x, 0.0));
            assert!(
                (jm.re - j).abs() < 1e-14 && jm.im.abs() < 1e-15,
                "J{m}({x}) {jm}"
            );
            assert!(
                (ym.re - y).abs() < 1e-14 && ym.im.abs() < 1e-15,
                "Y{m}({x}) {ym}"
            );
        }
    }

    /// The three-term recurrence f_(m+1) = (2m/z) f_m − f_(m−1) (Eq. 9.1.27), for J and Y, up
    /// to order 12 at complex arguments.
    #[test]
    fn the_recurrence_holds() {
        for &(re, im) in &[(0.7, 0.0), (5.0, -0.3), (11.0, -0.02), (14.5, 0.1)] {
            let z = c64::new(re, im);
            for m in 1..12 {
                let (j0, y0) = bessel(m - 1, z);
                let (j1, y1) = bessel(m, z);
                let (j2, y2) = bessel(m + 1, z);
                let scale = f64::from(2 * m) / z;
                let tolerance = 1e-9 * (j0.norm() + j2.norm() + y0.norm() + y2.norm());
                assert!(
                    (scale * j1 - j0 - j2).norm() < tolerance,
                    "J at {z}, order {m}"
                );
                assert!(
                    (scale * y1 - y0 - y2).norm() < tolerance,
                    "Y at {z}, order {m}"
                );
            }
        }
    }

    /// J_m Y′_m − J′_m Y_m = 2/(πz) at complex arguments up to |z| = 15.
    #[test]
    fn the_wronskian_holds() {
        let worst = wronskian_error();
        assert!(worst < 1e-9, "{worst}");
    }

    /// A resonance found from a real guess decays: it is below the real axis, at the Q of an
    /// outgoing wave.
    #[test]
    fn resonances_are_outgoing() {
        let ring = Ring::new(1.0, 2.0, 11.56).unwrap();
        let f = ring.resonance(4, c64::new(0.147, 0.0)).unwrap();
        // E_z's m = 4 mode, decaying (Im < 0) with Q about 344
        assert!(f.im < 0.0 && (ring_q(f) - 344.0).abs() < 1.0, "{f}");
        assert!(Ring::new(2.0, 1.0, 4.0).is_err());
    }
}
