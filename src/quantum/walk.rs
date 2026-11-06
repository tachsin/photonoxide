//! An array of evanescently coupled waveguides by coupled-mode theory: the continuous-time
//! quantum walk of A. Peruzzo et al., Science 329, 1500 (2010), doi:10.1126/science.1193515.

use num_complex::Complex64 as c64;

use super::matrix::{Matrix, hermitian_eigen};
use crate::{Error, Result};

/// The transfer matrix of N coupled waveguides of length `length` (µm), nearest neighbours
/// coupled by `couplings` (N − 1 values, per µm), each guide's propagation constant left out
/// (a common one is a global phase): U = exp(iHz) with H tridiagonal, H_j,j±1 = C, the
/// coupled-mode equations da_j/dz = i Σ_k H_jk a_k in the e^(−iωt) convention (Peruzzo et al.'s
/// Eq. 1 and 2, whose Hamiltonian has the opposite sign: U and its complex conjugate give the
/// same probabilities).
///
/// # Errors
///
/// [`Error::InvalidValue`] for no couplings, or a coupling or length that isn't finite.
pub fn coupled_array(couplings: &[f64], length: f64) -> Result<Matrix> {
    if couplings.is_empty() || !couplings.iter().chain([&length]).all(|v| v.is_finite()) {
        return Err(Error::invalid(
            "coupled waveguide array",
            "needs at least one finite coupling and a finite length",
        ));
    }
    let n = couplings.len() + 1;
    let h = Matrix::from_fn(n, n, |i, j| {
        if i + 1 == j {
            c64::new(couplings[i], 0.0)
        } else if j + 1 == i {
            c64::new(couplings[j], 0.0)
        } else {
            c64::new(0.0, 0.0)
        }
    });
    let (values, v) = hermitian_eigen(&h);
    Ok(Matrix::from_fn(n, n, |i, j| {
        (0..n)
            .map(|k| v[(i, k)] * c64::from_polar(1.0, values[k] * length) * v[(j, k)].conj())
            .sum()
    }))
}
