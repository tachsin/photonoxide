//! A small dense complex matrix, any shape, row by row: a transfer matrix, a unitary, a
//! permanent's argument. With the two factorizations the module needs, a Hermitian matrix's
//! eigenvectors by Jacobi's method and a square matrix's QR by Gram–Schmidt, written here so
//! that they give the same bits on every system.

use std::ops::{Index, IndexMut};

use num_complex::Complex64 as c64;

use crate::circuit::SMatrix;
use crate::portable;
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
            .map(|(a, b)| portable::complex::abs(a - b))
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
                worst = worst.max(portable::complex::abs(dot - one));
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
                let g = portable::complex::abs(a[(p, q)]);
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

/// The LU factorization of a square matrix with partial pivoting, in place: the factors and the
/// permutation's sign, or `None` when a pivot is exactly zero (a singular matrix).
fn lu(a: &Matrix) -> (Matrix, f64, bool) {
    let n = a.rows;
    let mut m = a.clone();
    let mut sign = 1.0;
    for k in 0..n {
        let p = (k..n)
            .max_by(|&i, &j| {
                portable::complex::abs(m[(i, k)]).total_cmp(&portable::complex::abs(m[(j, k)]))
            })
            .unwrap_or(k);
        if portable::complex::abs(m[(p, k)]) == 0.0 {
            return (m, sign, false);
        }
        if p != k {
            for j in 0..n {
                let t = m[(k, j)];
                m[(k, j)] = m[(p, j)];
                m[(p, j)] = t;
            }
            sign = -sign;
        }
        let pivot = m[(k, k)];
        for i in k + 1..n {
            let f = m[(i, k)] / pivot;
            m[(i, k)] = f;
            for j in k + 1..n {
                let mkj = m[(k, j)];
                m[(i, j)] -= f * mkj;
            }
        }
    }
    (m, sign, true)
}

/// The determinant of a square matrix, by LU with partial pivoting; 1 for the 0 × 0 matrix.
pub(crate) fn determinant(a: &Matrix) -> c64 {
    let (m, sign, regular) = lu(a);
    if !regular {
        return c64::new(0.0, 0.0);
    }
    (0..a.rows).map(|i| m[(i, i)]).product::<c64>() * sign
}

/// The inverse of a square matrix by Gauss–Jordan elimination with partial pivoting, or `None`
/// when it is singular.
pub(crate) fn inverse(a: &Matrix) -> Option<Matrix> {
    let n = a.rows;
    let mut m = a.clone();
    let mut inv = Matrix::identity(n);
    for k in 0..n {
        let p = (k..n).max_by(|&i, &j| {
            portable::complex::abs(m[(i, k)]).total_cmp(&portable::complex::abs(m[(j, k)]))
        })?;
        if portable::complex::abs(m[(p, k)]) == 0.0 {
            return None;
        }
        for j in 0..n {
            let (t, u) = (m[(k, j)], inv[(k, j)]);
            m[(k, j)] = m[(p, j)];
            m[(p, j)] = t;
            inv[(k, j)] = inv[(p, j)];
            inv[(p, j)] = u;
        }
        let pivot = m[(k, k)];
        for j in 0..n {
            m[(k, j)] /= pivot;
            inv[(k, j)] /= pivot;
        }
        for i in 0..n {
            if i == k {
                continue;
            }
            let f = m[(i, k)];
            if portable::complex::abs(f) == 0.0 {
                continue;
            }
            for j in 0..n {
                let (mkj, ikj) = (m[(k, j)], inv[(k, j)]);
                m[(i, j)] -= f * mkj;
                inv[(i, j)] -= f * ikj;
            }
        }
    }
    Some(inv)
}

/// The coefficients of det(I − λB) = Σ_j h_j λ^j, j = 0 … n, of a square matrix B: its
/// characteristic polynomial det(xI − B) = Σ_j h_j x^(n−j) read backwards. B is brought to upper
/// Hessenberg form by Householder reflections (similar, so the same polynomial), whose
/// polynomial follows from the recursion over its leading blocks (R. C. La Budde's method; G. H.
/// Golub, C. F. Van Loan, *Matrix Computations*): O(n³), with additions and multiplications only
/// after the reduction, and the same bits on every system.
pub(crate) fn det_one_minus(b: &Matrix) -> Vec<c64> {
    let n = b.rows;
    let mut h = b.clone();
    // Householder reduction to upper Hessenberg form
    for k in 0..n.saturating_sub(2) {
        let norm = (k + 1..n).map(|i| h[(i, k)].norm_sqr()).sum::<f64>().sqrt();
        if norm == 0.0 {
            continue;
        }
        let x0 = h[(k + 1, k)];
        let phase = if portable::complex::abs(x0) == 0.0 {
            c64::new(1.0, 0.0)
        } else {
            x0 / portable::complex::abs(x0)
        };
        // v = x + e^{i arg x0} ‖x‖ e₁, H ← (I − 2vv†/v†v) H (I − 2vv†/v†v)
        let mut v: Vec<c64> = (k + 1..n).map(|i| h[(i, k)]).collect();
        v[0] += phase * norm;
        let vv: f64 = v.iter().map(c64::norm_sqr).sum();
        if vv == 0.0 {
            continue;
        }
        for j in 0..n {
            let dot: c64 = v
                .iter()
                .enumerate()
                .map(|(a, va)| va.conj() * h[(k + 1 + a, j)])
                .sum::<c64>()
                * (2.0 / vv);
            for (a, va) in v.iter().enumerate() {
                h[(k + 1 + a, j)] -= va * dot;
            }
        }
        for i in 0..n {
            let dot: c64 = v
                .iter()
                .enumerate()
                .map(|(a, va)| h[(i, k + 1 + a)] * va)
                .sum::<c64>()
                * (2.0 / vv);
            for (a, va) in v.iter().enumerate() {
                h[(i, k + 1 + a)] -= dot * va.conj();
            }
        }
        for i in k + 2..n {
            h[(i, k)] = c64::new(0.0, 0.0);
        }
    }
    // p_i(x) = det(xI − H_i) of the leading i × i block, coefficients by ascending power:
    // p_i = (x − h_ii) p_{i−1} − Σ_{j=1}^{i−1} h_{i−j,i} (∏_{l=i−j+1}^{i} h_{l,l−1}) p_{i−j−1}
    // (one-based indices)
    let mut p: Vec<Vec<c64>> = vec![vec![c64::new(1.0, 0.0)]];
    for i in 1..=n {
        let hii = h[(i - 1, i - 1)];
        let prev = &p[i - 1];
        let mut next = vec![c64::new(0.0, 0.0); i + 1];
        for (d, c) in prev.iter().enumerate() {
            next[d + 1] += c;
            next[d] -= hii * c;
        }
        let mut product = c64::new(1.0, 0.0);
        for j in 1..i {
            // ∏_{l=i−j+1}^{i} h_{l,l−1}, one-based: grows by h_{i−j+1, i−j} each step
            product *= h[(i - j, i - j - 1)];
            let coefficient = h[(i - j - 1, i - 1)] * product;
            for (d, c) in p[i - j - 1].iter().enumerate() {
                next[d] -= coefficient * c;
            }
        }
        p.push(next);
    }
    // det(I − λB) = λⁿ p(1/λ): h_j is the coefficient of x^(n−j)
    let last = &p[n];
    (0..=n).map(|j| last[n - j]).collect()
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
