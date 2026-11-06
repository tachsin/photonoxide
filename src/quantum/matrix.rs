//! A small dense complex matrix, any shape, row by row: a transfer matrix, a unitary, a
//! permanent's argument. With the two factorizations the module needs, a Hermitian matrix's
//! eigenvectors by Jacobi's method and a square matrix's QR by Gram–Schmidt, written here so
//! that they give the same bits on every system.

use std::ops::{Index, IndexMut};

use num_complex::Complex64 as c64;

use crate::circuit::SMatrix;
use crate::{Error, Result};

/// A dense complex matrix: `a[(i, j)]` is row i, column j.
#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    /// Row by row: `values[i * cols + j]`.
    values: Vec<c64>,
}

impl Matrix {
    /// The rows × cols zero matrix.
    pub fn zeros(rows: usize, cols: usize) -> Matrix {
        Matrix {
            rows,
            cols,
            values: vec![c64::new(0.0, 0.0); rows * cols],
        }
    }

    /// The n × n identity.
    pub fn identity(n: usize) -> Matrix {
        Matrix::from_fn(n, n, |i, j| c64::new(if i == j { 1.0 } else { 0.0 }, 0.0))
    }

    /// The matrix with entry `f(i, j)` at row i, column j.
    pub fn from_fn(rows: usize, cols: usize, f: impl Fn(usize, usize) -> c64) -> Matrix {
        Matrix {
            rows,
            cols,
            values: (0..rows * cols).map(|k| f(k / cols, k % cols)).collect(),
        }
    }

    /// The matrix from its rows.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the rows differ in length or hold a value that isn't finite.
    pub fn from_rows(rows: Vec<Vec<c64>>) -> Result<Matrix> {
        let cols = rows.first().map_or(0, Vec::len);
        if rows.iter().any(|r| r.len() != cols) {
            return Err(Error::invalid(
                "matrix",
                "every row must have the same number of values",
            ));
        }
        if rows
            .iter()
            .flatten()
            .any(|v| !v.re.is_finite() || !v.im.is_finite())
        {
            return Err(Error::invalid("matrix", "every value must be finite"));
        }
        Ok(Matrix {
            rows: rows.len(),
            cols,
            values: rows.into_iter().flatten().collect(),
        })
    }

    /// The number of rows.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// The number of columns.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Whether it is square.
    pub fn is_square(&self) -> bool {
        self.rows == self.cols
    }

    /// The matrix of the given rows and columns, in that order, each as often as it is listed:
    /// `a.select(&[0, 0], &[1, 2])` repeats row 0. The submatrix a pattern of photons picks.
    ///
    /// # Panics
    ///
    /// If an index is out of range.
    pub fn select(&self, rows: &[usize], cols: &[usize]) -> Matrix {
        Matrix::from_fn(rows.len(), cols.len(), |i, j| self[(rows[i], cols[j])])
    }

    /// The conjugate transpose, A†.
    pub fn adjoint(&self) -> Matrix {
        Matrix::from_fn(self.cols, self.rows, |i, j| self[(j, i)].conj())
    }

    /// The product A B.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless A has as many columns as B has rows.
    pub fn product(&self, other: &Matrix) -> Result<Matrix> {
        if self.cols != other.rows {
            return Err(Error::invalid(
                "matrix",
                format!(
                    "can't multiply {} columns by {} rows",
                    self.cols, other.rows
                ),
            ));
        }
        Ok(Matrix::from_fn(self.rows, other.cols, |i, j| {
            (0..self.cols).map(|k| self[(i, k)] * other[(k, j)]).sum()
        }))
    }

    /// The trace, the sum of the diagonal.
    pub fn trace(&self) -> c64 {
        (0..self.rows.min(self.cols)).map(|i| self[(i, i)]).sum()
    }

    /// The largest |A_ij − B_ij|, or infinity when the shapes differ.
    pub fn max_difference(&self, other: &Matrix) -> f64 {
        if self.rows != other.rows || self.cols != other.cols {
            return f64::INFINITY;
        }
        self.values
            .iter()
            .zip(&other.values)
            .map(|(a, b)| (a - b).norm())
            .fold(0.0, f64::max)
    }

    /// How far its columns are from orthonormal: the largest entry of |A†A − I|. Zero for a
    /// unitary, and for an isometry (a lossy circuit with its loss modes).
    pub fn isometry_error(&self) -> f64 {
        let mut worst: f64 = 0.0;
        for i in 0..self.cols {
            for j in 0..self.cols {
                let dot: c64 = (0..self.rows)
                    .map(|k| self[(k, i)].conj() * self[(k, j)])
                    .sum();
                let one = if i == j { 1.0 } else { 0.0 };
                worst = worst.max((dot - one).norm());
            }
        }
        worst
    }

    /// The entries' squared moduli, |A_ij|²: the matrix of distinguishable particles.
    pub fn moduli_squared(&self) -> Matrix {
        Matrix::from_fn(self.rows, self.cols, |i, j| {
            c64::new(self[(i, j)].norm_sqr(), 0.0)
        })
    }
}

impl From<&SMatrix> for Matrix {
    fn from(s: &SMatrix) -> Matrix {
        let n = s.size();
        Matrix::from_fn(n, n, |q, p| s[(q, p)])
    }
}

impl Index<(usize, usize)> for Matrix {
    type Output = c64;

    fn index(&self, (i, j): (usize, usize)) -> &c64 {
        assert!(i < self.rows && j < self.cols, "({i}, {j}) is out of range");
        &self.values[i * self.cols + j]
    }
}

impl IndexMut<(usize, usize)> for Matrix {
    fn index_mut(&mut self, (i, j): (usize, usize)) -> &mut c64 {
        assert!(i < self.rows && j < self.cols, "({i}, {j}) is out of range");
        &mut self.values[i * self.cols + j]
    }
}

/// A Hermitian matrix's eigenvalues and eigenvectors (the columns of V, H = V diag(λ) V†) by
/// the cyclic Jacobi method: each off-diagonal pair zeroed in turn by a phase and a rotation,
/// sweeps repeated until the off-diagonal part is round-off. Only the lower triangle's
/// Hermitian part is used. Slow for large matrices, exact to round-off for the small ones here.
pub(crate) fn hermitian_eigen(h: &Matrix) -> (Vec<f64>, Matrix) {
    let n = h.rows;
    let mut a = Matrix::from_fn(n, n, |i, j| 0.5 * (h[(i, j)] + h[(j, i)].conj()));
    let mut v = Matrix::identity(n);
    let scale = a.values.iter().map(|x| x.norm_sqr()).sum::<f64>().sqrt();
    for _sweep in 0..100 {
        let off: f64 = (0..n)
            .flat_map(|i| (0..n).filter(move |&j| j != i).map(move |j| (i, j)))
            .map(|(i, j)| a[(i, j)].norm_sqr())
            .sum::<f64>()
            .sqrt();
        if off <= 1e-15 * scale || off == 0.0 {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                let g = a[(p, q)].norm();
                if g == 0.0 {
                    continue;
                }
                // a_pq = g e^(iφ): the phase makes the pair real, the rotation zeroes it
                let phase = a[(p, q)] / g;
                let tau = (a[(q, q)].re - a[(p, p)].re) / (2.0 * g);
                let t = if tau >= 0.0 {
                    1.0 / (tau + (1.0 + tau * tau).sqrt())
                } else {
                    -1.0 / (-tau + (1.0 + tau * tau).sqrt())
                };
                let c = 1.0 / (1.0 + t * t).sqrt();
                let s = t * c;
                let e = phase.conj(); // e^(−iφ)
                // A ← A G with G's columns p = (c, −s e^(−iφ)) and q = (s, c e^(−iφ))
                for k in 0..n {
                    let (akp, akq) = (a[(k, p)], a[(k, q)]);
                    a[(k, p)] = akp * c - akq * s * e;
                    a[(k, q)] = akp * s + akq * c * e;
                    let (vkp, vkq) = (v[(k, p)], v[(k, q)]);
                    v[(k, p)] = vkp * c - vkq * s * e;
                    v[(k, q)] = vkp * s + vkq * c * e;
                }
                // A ← G† A
                for k in 0..n {
                    let (apk, aqk) = (a[(p, k)], a[(q, k)]);
                    a[(p, k)] = apk * c - aqk * s * phase;
                    a[(q, k)] = apk * s + aqk * c * phase;
                }
                a[(p, q)] = c64::new(0.0, 0.0);
                a[(q, p)] = c64::new(0.0, 0.0);
            }
        }
    }
    ((0..n).map(|i| a[(i, i)].re).collect(), v)
}

/// The QR factorization of a square matrix by modified Gram–Schmidt, applied twice for
/// orthogonality to round-off: Q unitary, R upper triangular. Assumes full rank, which a
/// Gaussian random matrix has with probability 1.
pub(crate) fn qr(a: &Matrix) -> (Matrix, Matrix) {
    let n = a.rows;
    let mut q = a.clone();
    let mut r = Matrix::zeros(n, n);
    for j in 0..n {
        for _pass in 0..2 {
            for i in 0..j {
                let dot: c64 = (0..n).map(|k| q[(k, i)].conj() * q[(k, j)]).sum();
                r[(i, j)] += dot;
                for k in 0..n {
                    let qki = q[(k, i)];
                    q[(k, j)] -= dot * qki;
                }
            }
        }
        let norm = (0..n).map(|k| q[(k, j)].norm_sqr()).sum::<f64>().sqrt();
        r[(j, j)] = c64::new(norm, 0.0);
        for k in 0..n {
            q[(k, j)] /= norm;
        }
    }
    (q, r)
}
