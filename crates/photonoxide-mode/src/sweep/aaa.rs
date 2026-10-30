//! The AAA algorithm: a rational function in barycentric form, its support points chosen
//! greedily and its weights from the Loewner matrix's smallest singular vector.
//!
//! Y. Nakatsukasa, O. Sète, L. N. Trefethen, "The AAA algorithm for rational approximation",
//! SIAM J. Sci. Comput. 40, A1494 (2018),
//! [doi:10.1137/16M1106122](https://doi.org/10.1137/16M1106122): the barycentric form of their
//! Eq. 2.5, the core algorithm of their Section 3 (Eqs. 3.1–3.10, Fig. 4.1's code) and the poles
//! from the arrowhead pencil of their Eq. 3.11.

use faer::Mat;
use faer::prelude::Solve;
use num_complex::Complex64 as c64;

use crate::{Error, Result};

/// The relative tolerance AAA stops at on its samples by default: the paper's, 1e-13 relative
/// to the largest sample (p. A1498 and Fig. 4.1).
pub const DEFAULT_TOLERANCE: f64 = 1e-13;

/// A rational function of type (m − 1, m − 1) in barycentric form (the paper's Eq. 2.5),
///
/// r(x) = Σ w_j f_j/(x − z_j) / Σ w_j/(x − z_j),
///
/// with m support points z_j on the real line, the values f_j there and the weights w_j. Where
/// w_j ≠ 0 it interpolates, r(z_j) = f_j (their Theorem 2.1).
#[derive(Clone, Debug, PartialEq)]
pub struct Rational {
    support: Vec<f64>,
    values: Vec<c64>,
    weights: Vec<c64>,
}

impl Rational {
    /// The AAA approximation of the samples `f` at the distinct points `x` (the paper's core
    /// algorithm, Section 3): at step m the next support point is the sample where the last
    /// step's |f − r| is largest, and the weights w (‖w‖ = 1) minimize ‖A w‖ over the samples
    /// that are not support points, A the Loewner matrix (F_i − f_j)/(Z_i − z_j) of Eq. 3.6,
    /// built as S_F C − C S_f from the Cauchy matrix C (Eqs. 3.7–3.9): w is A's last right
    /// singular vector. It stops when max |f − r| over the samples is at most `tolerance` times
    /// the largest |f| (their Fig. 4.1; [`DEFAULT_TOLERANCE`] is theirs), or at m = M/2 for M
    /// samples, the largest m whose least-squares problem has at least as many rows as unknowns
    /// (their assumption below Eq. 3.4). Fewer than two samples give the constant through them.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no samples, `x` and `f` of different lengths, a point or a
    /// value that isn't finite, points that aren't distinct, or a tolerance that isn't a
    /// non-negative number.
    pub fn aaa(x: &[f64], f: &[c64], tolerance: f64) -> Result<Rational> {
        if x.is_empty() || x.len() != f.len() {
            return Err(Error::invalid(
                "AAA",
                format!(
                    "needs samples, and a value for each point: got {} points and {} values",
                    x.len(),
                    f.len()
                ),
            ));
        }
        if !(tolerance >= 0.0 && tolerance.is_finite()) {
            return Err(Error::invalid(
                "AAA",
                format!("the tolerance must be a non-negative number, got {tolerance}"),
            ));
        }
        if x.iter().any(|v| !v.is_finite()) || f.iter().any(|v| !v.is_finite()) {
            return Err(Error::invalid(
                "AAA",
                "the points and values must be finite",
            ));
        }
        let mut sorted = x.to_vec();
        sorted.sort_by(f64::total_cmp);
        if sorted.windows(2).any(|w| w[0] == w[1]) {
            return Err(Error::invalid("AAA", "the points must be distinct"));
        }
        let m_total = x.len();
        let scale = f.iter().map(|v| v.norm()).fold(0.0, f64::max);
        let mean = f.iter().sum::<c64>() / m_total as f64;
        let mut r = vec![mean; m_total];
        // J of Fig. 4.1: the samples not yet support points
        let mut rest: Vec<usize> = (0..m_total).collect();
        let mut chosen: Vec<usize> = Vec::new();
        let mut weights = vec![c64::new(1.0, 0.0)];
        let steps = (m_total / 2).max(1);
        for m in 1..=steps {
            // the greedy step: the next support point where the residual is largest
            let (at, _) = rest
                .iter()
                .enumerate()
                .map(|(k, &i)| {
                    // a zero of the denominator at a sample: the largest residual there is
                    let e = (f[i] - r[i]).norm();
                    (k, if e.is_nan() { f64::INFINITY } else { e })
                })
                .fold((0, -1.0), |best, c| if c.1 > best.1 { c } else { best });
            chosen.push(rest.remove(at));
            if rest.is_empty() {
                weights = vec![c64::new(1.0, 0.0)];
                break;
            }
            // the Loewner matrix over the rest (Eqs. 3.6 and 3.9)
            let a = Mat::<c64>::from_fn(rest.len(), m, |i, j| {
                let (zi, zj) = (rest[i], chosen[j]);
                (f[zi] - f[zj]) / (x[zi] - x[zj])
            });
            weights = smallest_right_singular_vector(&a);
            // N/D on the rest (Eq. 3.10), the samples themselves at the support points
            let fit = Rational {
                support: chosen.iter().map(|&i| x[i]).collect(),
                values: chosen.iter().map(|&i| f[i]).collect(),
                weights: weights.clone(),
            };
            for &i in &rest {
                r[i] = fit.eval(x[i]);
            }
            for &i in &chosen {
                r[i] = f[i];
            }
            let error = (0..m_total)
                .map(|i| (f[i] - r[i]).norm())
                .fold(
                    0.0,
                    |a: f64, b| if b.is_nan() { f64::INFINITY } else { a.max(b) },
                );
            if error <= tolerance * scale {
                break;
            }
        }
        Ok(Rational {
            support: chosen.iter().map(|&i| x[i]).collect(),
            values: chosen.iter().map(|&i| f[i]).collect(),
            weights,
        })
    }

    /// r(x) by the barycentric formula (Eq. 2.5); f_j itself at a support point z_j.
    pub fn eval(&self, x: f64) -> c64 {
        let (mut n, mut d) = (c64::new(0.0, 0.0), c64::new(0.0, 0.0));
        for ((&z, &f), &w) in self.support.iter().zip(&self.values).zip(&self.weights) {
            if x == z {
                return f;
            }
            let c = w / (x - z);
            n += c * f;
            d += c;
        }
        n / d
    }

    /// The support points z_j.
    pub fn support(&self) -> &[f64] {
        &self.support
    }

    /// The weights w_j, of unit 2-norm.
    pub fn weights(&self) -> &[c64] {
        &self.weights
    }

    /// The number of support points m: r is of type (m − 1, m − 1).
    pub fn len(&self) -> usize {
        self.support.len()
    }

    /// Whether r has no support points (never, for one [`Rational::aaa`] made).
    pub fn is_empty(&self) -> bool {
        self.support.is_empty()
    }

    /// The poles of r: the finite eigenvalues of the (m + 1) × (m + 1) arrowhead pencil
    /// E − λB of the paper's Eq. 3.11, E = [[0, wᵀ], [1, diag(z)]] and B = diag(0, 1, …, 1),
    /// whose eigenvalues are the zeros of the denominator Σ w_j/(λ − z_j) and two infinite ones.
    /// The pencil is solved as the standard eigenproblem of (E − sB)⁻¹B, whose eigenvalues are
    /// μ = 1/(λ − s), with the shift s off the real line by the support's width: an infinite λ
    /// is μ = 0. Poles more than 1e8 times that width from it are left out with the infinite
    /// ones. Any order.
    pub fn poles(&self) -> Vec<c64> {
        let m = self.support.len();
        if m < 2 {
            return Vec::new();
        }
        let (lo, hi) = self
            .support
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &z| {
                (a.min(z), b.max(z))
            });
        let width = (hi - lo).max(f64::MIN_POSITIVE);
        let s = c64::new(0.5 * (lo + hi), width);
        let shifted = Mat::<c64>::from_fn(m + 1, m + 1, |i, j| match (i, j) {
            (0, 0) => c64::new(0.0, 0.0),
            (0, j) => self.weights[j - 1],
            (_, 0) => c64::new(1.0, 0.0),
            (i, j) if i == j => self.support[i - 1] - s,
            _ => c64::new(0.0, 0.0),
        });
        let b = Mat::<c64>::from_fn(m + 1, m + 1, |i, j| {
            if i == j && i > 0 {
                c64::new(1.0, 0.0)
            } else {
                c64::new(0.0, 0.0)
            }
        });
        let k = shifted.partial_piv_lu().solve(&b);
        let Ok(mu) = k.eigenvalues() else {
            return Vec::new();
        };
        mu.into_iter()
            .filter(|mu| mu.norm() * width > 1e-8)
            .map(|mu| s + 1.0 / mu)
            .collect()
    }
}

/// The right singular vector of `a` (rows ≥ columns) for its smallest singular value: faer's
/// thin SVD, or, where that doesn't converge (it can, rarely: #398), one-sided Jacobi.
fn smallest_right_singular_vector(a: &Mat<c64>) -> Vec<c64> {
    let n = a.ncols();
    if let Ok(svd) = a.thin_svd() {
        // faer keeps a complex matrix's singular values as complex numbers, real and ≥ 0
        let s: Vec<f64> = svd.S().column_vector().iter().map(|v| v.re).collect();
        let k = (0..n).min_by(|&i, &j| s[i].total_cmp(&s[j])).unwrap_or(0);
        return (0..n).map(|i| svd.V()[(i, k)]).collect();
    }
    let (s, v) = jacobi(a);
    let k = (0..n).min_by(|&i, &j| s[i].total_cmp(&s[j])).unwrap_or(0);
    (0..n).map(|i| v[(i, k)]).collect()
}

/// One-sided Jacobi for a complex matrix (Hestenes 1958, as [`crate::lstsq`]'s real one): each
/// pair of columns made orthogonal by a phase and a rotation, V accumulating them; the columns'
/// norms are the singular values, in the columns' order.
fn jacobi(a: &Mat<c64>) -> (Vec<f64>, Mat<c64>) {
    let (rows, n) = (a.nrows(), a.ncols());
    let mut u = a.to_owned();
    let mut v = Mat::<c64>::from_fn(n, n, |i, j| c64::new(f64::from(u8::from(i == j)), 0.0));
    for _ in 0..100 {
        let mut rotated = false;
        for p in 0..n {
            for q in p + 1..n {
                let (mut alpha, mut beta, mut gamma) = (0.0, 0.0, c64::new(0.0, 0.0));
                for i in 0..rows {
                    alpha += u[(i, p)].norm_sqr();
                    beta += u[(i, q)].norm_sqr();
                    gamma += u[(i, p)].conj() * u[(i, q)];
                }
                let g = gamma.norm();
                if g == 0.0 || g <= f64::EPSILON * (alpha * beta).sqrt() {
                    continue;
                }
                rotated = true;
                // column q times e^(−iφ) makes the pair's inner product real, |γ|
                let phase = (gamma / g).conj();
                let zeta = (beta - alpha) / (2.0 * g);
                let t = zeta.signum() / (zeta.abs() + (1.0 + zeta * zeta).sqrt());
                let c = 1.0 / (1.0 + t * t).sqrt();
                let s = c * t;
                for i in 0..rows {
                    let (x, y) = (u[(i, p)], u[(i, q)] * phase);
                    u[(i, p)] = x * c - y * s;
                    u[(i, q)] = x * s + y * c;
                }
                for i in 0..n {
                    let (x, y) = (v[(i, p)], v[(i, q)] * phase);
                    v[(i, p)] = x * c - y * s;
                    v[(i, q)] = x * s + y * c;
                }
            }
        }
        if !rotated {
            break;
        }
    }
    let s = (0..n)
        .map(|k| (0..rows).map(|i| u[(i, k)].norm_sqr()).sum::<f64>().sqrt())
        .collect();
    (s, v)
}

#[cfg(test)]
pub(super) fn jacobi_for_tests(a: &Mat<c64>) -> (Vec<f64>, Mat<c64>) {
    jacobi(a)
}
