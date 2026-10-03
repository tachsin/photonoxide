//! Objectives for [`Circuit::gradient`](super::Circuit::gradient): real functions of a circuit's
//! S-matrices at several wavelengths, each with its sensitivities.
//!
//! An objective returns F and, for each wavelength, the matrix G of its Wirtinger derivatives
//! G_qp = ∂F/∂S_qp = ½(∂F/∂Re S_qp − i ∂F/∂Im S_qp), so that dF = 2 Re Σ G_qp dS_qp. Any closure
//! of the same form is an objective; sums of objectives add their values and their
//! sensitivities.
//!
//! An objective (the unit test `circuit::objective::tests::the_module_example` runs this):
//!
//! ```ignore
//! use photonoxide::circuit::{SMatrix, objective};
//! use photonoxide::Complex64 as c64;
//!
//! // a 2-port that transmits 0.6 + 0.3i
//! let s = SMatrix::from_fn(2, |q, p| if q == p { c64::new(0.0, 0.0) } else { c64::new(0.6, 0.3) });
//! let (power, g) = objective::power(&[s.clone()], 1, 0);
//! assert!((power - 0.45).abs() < 1e-15);
//! assert_eq!(g[0][(1, 0)], c64::new(0.6, -0.3)); // conj(S₂₁)
//! let (error, _) = objective::power_error(&[s], 1, 0, &[0.5]);
//! assert!((error - 0.05f64.powi(2)).abs() < 1e-15);
//! ```

use num_complex::Complex64 as c64;

use super::SMatrix;

/// Σ_λ |S_qp(λ)|²: the power from port p to port q, summed over the wavelengths. Its sensitivity
/// is conj(S_qp) at (q, p).
///
/// # Panics
///
/// If q or p isn't a port.
pub fn power(s: &[SMatrix], q: usize, p: usize) -> (f64, Vec<SMatrix>) {
    let value = s.iter().map(|m| m.power(q, p)).sum();
    let sensitivities = s
        .iter()
        .map(|m| only(m.size(), q, p, m[(q, p)].conj()))
        .collect();
    (value, sensitivities)
}

/// Σ_λ (|S_qp(λ)|² − T_λ)²: how far the power from port p to port q is from `targets`, one per
/// wavelength, e.g. a splitting ratio or a filter's passband and stopband. Its sensitivity is
/// 2 (|S_qp|² − T) conj(S_qp) at (q, p).
///
/// # Panics
///
/// If q or p isn't a port, or there isn't one target per S-matrix.
pub fn power_error(s: &[SMatrix], q: usize, p: usize, targets: &[f64]) -> (f64, Vec<SMatrix>) {
    assert_eq!(s.len(), targets.len(), "one target per wavelength");
    let mut value = 0.0;
    let sensitivities = s
        .iter()
        .zip(targets)
        .map(|(m, &t)| {
            let miss = m.power(q, p) - t;
            value += miss * miss;
            only(m.size(), q, p, 2.0 * miss * m[(q, p)].conj())
        })
        .collect();
    (value, sensitivities)
}

/// Σ_λ Σ_qp |S_qp(λ) − T_qp(λ)|²: how far S is from `targets`, amplitudes and phases, one per
/// wavelength, e.g. a unitary a mesh is to implement. Its sensitivity is conj(S − T).
///
/// # Panics
///
/// If there isn't one target per S-matrix, each of its size.
pub fn matrix_error(s: &[SMatrix], targets: &[SMatrix]) -> (f64, Vec<SMatrix>) {
    assert_eq!(s.len(), targets.len(), "one target per wavelength");
    let mut value = 0.0;
    let sensitivities = s
        .iter()
        .zip(targets)
        .map(|(m, t)| {
            assert_eq!(m.size(), t.size(), "a target of the S-matrix's size");
            let g = SMatrix::from_fn(m.size(), |q, p| (m[(q, p)] - t[(q, p)]).conj());
            value += g.rows().iter().flatten().map(c64::norm_sqr).sum::<f64>();
            g
        })
        .collect();
    (value, sensitivities)
}

/// The n × n matrix with `v` at (q, p) and 0 elsewhere.
fn only(n: usize, q: usize, p: usize, v: c64) -> SMatrix {
    let mut m = SMatrix::zeros(n);
    m[(q, p)] = v;
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_module_example() {
        // a 2-port that transmits 0.6 + 0.3i
        let s = SMatrix::from_fn(2, |q, p| {
            if q == p {
                c64::new(0.0, 0.0)
            } else {
                c64::new(0.6, 0.3)
            }
        });
        let (power, g) = power(std::slice::from_ref(&s), 1, 0);
        assert!((power - 0.45).abs() < 1e-15);
        assert_eq!(g[0][(1, 0)], c64::new(0.6, -0.3)); // conj(S₂₁)
        let (error, _) = power_error(&[s], 1, 0, &[0.5]);
        assert!((error - 0.05f64.powi(2)).abs() < 1e-15);
    }

    #[test]
    fn sensitivities_are_the_wirtinger_derivatives() {
        // F(S + h e_qp) − F(S − h e_qp) ≈ 2 Re(G_qp) 2h, and with i h, −2 Im(G_qp) 2h
        let s = SMatrix::from_fn(3, |q, p| {
            c64::new(0.1 * q as f64 - 0.2, 0.3 + 0.05 * p as f64)
        });
        let t = SMatrix::from_fn(3, |q, p| c64::new(0.2, -0.1 * (q + p) as f64));
        type Objective<'a> = &'a dyn Fn(&[SMatrix]) -> (f64, Vec<SMatrix>);
        let objectives: [Objective; 3] = [
            &|m| power(m, 2, 1),
            &|m| power_error(m, 2, 1, &[0.3]),
            &|m| matrix_error(m, std::slice::from_ref(&t)),
        ];
        for f in objectives {
            let g = &f(std::slice::from_ref(&s)).1[0];
            for (q, p) in [(2, 1), (0, 2)] {
                for step in [c64::new(1e-6, 0.0), c64::new(0.0, 1e-6)] {
                    let mut plus = s.clone();
                    plus[(q, p)] += step;
                    let mut minus = s.clone();
                    minus[(q, p)] -= step;
                    let fd = (f(&[plus]).0 - f(&[minus]).0) / 2.0;
                    let predicted = 2.0 * (g[(q, p)] * step).re;
                    assert!((fd - predicted).abs() < 1e-14, "{fd} {predicted}");
                }
            }
        }
    }
}
