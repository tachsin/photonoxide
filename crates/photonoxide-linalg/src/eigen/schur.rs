//! The complex Schur form of a small dense matrix, and the reordering of its diagonal: what a
//! Krylov–Schur restart does to its Rayleigh quotient.
//!
//! A = Q T Qᴴ with Q unitary and T upper triangular, by Householder reduction to Hessenberg form
//! and the shifted QR algorithm with Wilkinson's shift, one shift at a time, applied by Givens
//! rotations (G. H. Golub, C. F. Van Loan, *Matrix Computations*, 4th ed., Johns Hopkins
//! University Press (2013, issued 2012),
//! [doi:10.56021/9781421407944](https://doi.org/10.56021/9781421407944), Sections 7.4 and 7.5,
//! here in complex arithmetic, where one shift needs no pairing with its conjugate). Two
//! neighbouring eigenvalues on T's diagonal are exchanged by one rotation, as LAPACK's
//! `ztrexc` does (Z. Bai, J. W. Demmel, Linear Algebra Appl. 186, 73 (1993),
//! [doi:10.1016/0024-3795(93)90286-W](https://doi.org/10.1016/0024-3795(93)90286-W), the
//! method G. W. Stewart's Krylov–Schur restart relies on). Every loop runs in a fixed order on
//! one thread, so the result is the same bits however many threads the caller has.

use faer::{Mat, c64};

use crate::{Error, Result};

/// The unitary rotation G = [[u₁, −ū₂], [u₂, ū₁]] whose first column is (f, g)/‖(f, g)‖, so
/// Gᴴ (f, g)ᵀ = (‖(f, g)‖, 0)ᵀ; the identity for (0, 0).
fn rotation(f: c64, g: c64) -> (c64, c64) {
    let r = f.norm().hypot(g.norm());
    if r == 0.0 {
        (c64::new(1.0, 0.0), c64::new(0.0, 0.0))
    } else {
        (f / r, g / r)
    }
}

/// Rows p and p + 1 of `a`, columns `columns`, multiplied by Gᴴ from the left.
fn rows(a: &mut Mat<c64>, p: usize, columns: std::ops::Range<usize>, (u1, u2): (c64, c64)) {
    for j in columns {
        let (x, y) = (a[(p, j)], a[(p + 1, j)]);
        a[(p, j)] = u1.conj() * x + u2.conj() * y;
        a[(p + 1, j)] = -u2 * x + u1 * y;
    }
}

/// Columns p and p + 1 of `a`, rows `rows`, multiplied by G from the right.
fn columns(a: &mut Mat<c64>, p: usize, rows: std::ops::Range<usize>, (u1, u2): (c64, c64)) {
    for i in rows {
        let (x, y) = (a[(i, p)], a[(i, p + 1)]);
        a[(i, p)] = x * u1 + y * u2;
        a[(i, p + 1)] = -x * u2.conj() + y * u1.conj();
    }
}

/// Reduces `a` to upper Hessenberg form in place by Householder reflections, accumulating them
/// into `q`'s columns (Golub and Van Loan, Algorithm 7.4.2). A column already zero below its
/// subdiagonal is left as it is, so a block already triangular stays exactly so.
fn hessenberg(a: &mut Mat<c64>, q: &mut Mat<c64>) {
    let n = a.nrows();
    for c in 0..n.saturating_sub(2) {
        let below: f64 = (c + 2..n).map(|i| a[(i, c)].norm_sqr()).sum();
        if below == 0.0 {
            continue;
        }
        let x0 = a[(c + 1, c)];
        let norm = (below + x0.norm_sqr()).sqrt();
        // v = x − α e₁ with α = −e^{i arg x₀} ‖x‖: no cancellation in v's first entry
        let phase = if x0.norm() == 0.0 {
            c64::new(1.0, 0.0)
        } else {
            x0 / x0.norm()
        };
        let mut v: Vec<c64> = (c + 1..n).map(|i| a[(i, c)]).collect();
        v[0] += phase * norm;
        let vv: f64 = v.iter().map(|z| z.norm_sqr()).sum();
        let scale = 2.0 / vv;
        // a ← (I − 2 v vᴴ / vᴴv) a on rows c + 1 …, then a ← a (I − 2 v vᴴ / vᴴv) on those columns
        for j in c..n {
            let s: c64 = v
                .iter()
                .enumerate()
                .map(|(k, vk)| vk.conj() * a[(c + 1 + k, j)])
                .sum();
            let s = s * scale;
            for (k, vk) in v.iter().enumerate() {
                a[(c + 1 + k, j)] -= vk * s;
            }
        }
        for m in [&mut *a, &mut *q] {
            for i in 0..n {
                let s: c64 = v
                    .iter()
                    .enumerate()
                    .map(|(k, vk)| m[(i, c + 1 + k)] * vk)
                    .sum();
                let s = s * scale;
                for (k, vk) in v.iter().enumerate() {
                    m[(i, c + 1 + k)] -= s * vk.conj();
                }
            }
        }
        for i in c + 2..n {
            a[(i, c)] = c64::new(0.0, 0.0);
        }
    }
}

/// The eigenvalue of [[a, b], [c, d]] nearer d: Wilkinson's shift.
fn wilkinson(a: c64, b: c64, c: c64, d: c64) -> c64 {
    let half = (a + d) * 0.5;
    let s = ((a - d) * (a - d) * 0.25 + b * c).sqrt();
    let (l1, l2) = (half + s, half - s);
    if (l1 - d).norm() <= (l2 - d).norm() {
        l1
    } else {
        l2
    }
}

/// The Schur form of the square matrix `a`: (T, Q) with a = Q T Qᴴ, Q unitary and T upper
/// triangular, the entries below T's diagonal exactly zero.
///
/// # Errors
///
/// [`Error::InvalidValue`] if the QR algorithm doesn't converge within 100 sweeps per
/// eigenvalue, or `a` holds a NaN or an infinity.
pub(crate) fn schur(mut a: Mat<c64>) -> Result<(Mat<c64>, Mat<c64>)> {
    let n = a.nrows();
    debug_assert_eq!(n, a.ncols());
    if (0..n).any(|j| (0..n).any(|i| !a[(i, j)].is_finite())) {
        return Err(Error::invalid(
            "eigenproblem",
            "the Rayleigh quotient isn't finite",
        ));
    }
    let mut q = Mat::<c64>::identity(n, n);
    hessenberg(&mut a, &mut q);
    let largest = (0..n)
        .flat_map(|j| (0..n).map(move |i| (i, j)))
        .map(|(i, j)| a[(i, j)].norm())
        .fold(0.0, f64::max);
    let eps = f64::EPSILON;
    // the active block is lo..hi; below hi, T is done
    let mut hi = n;
    let mut since = 0usize;
    let mut sweeps = 0usize;
    while hi > 1 {
        // the lowest negligible subdiagonal entry splits the active block
        let mut lo = hi - 1;
        while lo > 0 {
            let mut scale = a[(lo - 1, lo - 1)].norm() + a[(lo, lo)].norm();
            if scale == 0.0 {
                scale = largest;
            }
            if a[(lo, lo - 1)].norm() <= eps * scale {
                a[(lo, lo - 1)] = c64::new(0.0, 0.0);
                break;
            }
            lo -= 1;
        }
        if lo == hi - 1 {
            // a 1 × 1 block: converged
            hi -= 1;
            since = 0;
            continue;
        }
        since += 1;
        sweeps += 1;
        if sweeps > 100 * n {
            return Err(Error::invalid(
                "eigenproblem",
                format!("the QR algorithm didn't converge on the {n} x {n} Rayleigh quotient"),
            ));
        }
        let mu = if since.is_multiple_of(10) {
            // an exceptional shift, to break a cycle
            a[(hi - 1, hi - 1)] + c64::new(0.75 * a[(hi - 1, hi - 2)].norm(), 0.0)
        } else {
            wilkinson(
                a[(hi - 2, hi - 2)],
                a[(hi - 2, hi - 1)],
                a[(hi - 1, hi - 2)],
                a[(hi - 1, hi - 1)],
            )
        };
        // one implicit QR sweep: the first rotation from the shifted first column, then the
        // bulge chased down the subdiagonal
        for p in lo..hi - 1 {
            let (f, g) = if p == lo {
                (a[(lo, lo)] - mu, a[(lo + 1, lo)])
            } else {
                (a[(p, p - 1)], a[(p + 1, p - 1)])
            };
            let g_ = rotation(f, g);
            let first = if p == lo { lo } else { p - 1 };
            rows(&mut a, p, first..n, g_);
            if p > lo {
                a[(p + 1, p - 1)] = c64::new(0.0, 0.0);
            }
            columns(&mut a, p, 0..(p + 3).min(hi), g_);
            columns(&mut q, p, 0..n, g_);
        }
    }
    for j in 0..n {
        for i in j + 1..n {
            a[(i, j)] = c64::new(0.0, 0.0);
        }
    }
    Ok((a, q))
}

/// Exchanges T's diagonal entries k and k + 1 by one rotation, keeping T upper triangular and
/// Q T Qᴴ the same matrix: G's first column is the eigenvector of T's 2 × 2 block for its
/// second eigenvalue.
pub(crate) fn swap(t: &mut Mat<c64>, q: &mut Mat<c64>, k: usize) {
    let n = t.nrows();
    let (a, b, c) = (t[(k, k)], t[(k + 1, k + 1)], t[(k, k + 1)]);
    if a == b {
        return;
    }
    let g = rotation(c, b - a);
    rows(t, k, k..n, g);
    columns(t, k, 0..k + 2, g);
    columns(q, k, 0..q.nrows(), g);
    // what the rotation makes them, to rounding: exactly
    t[(k + 1, k)] = c64::new(0.0, 0.0);
    t[(k, k)] = b;
    t[(k + 1, k + 1)] = a;
}

/// Reorders T's diagonal so its first `count` entries are those of largest `key`, largest
/// first (the first of equals first), exchanging neighbours; positions before `from` stay.
pub(crate) fn order(
    t: &mut Mat<c64>,
    q: &mut Mat<c64>,
    from: usize,
    count: usize,
    key: impl Fn(c64) -> f64,
) {
    let n = t.nrows();
    for p in from..count.min(n) {
        let mut best = p;
        for i in p + 1..n {
            if key(t[(i, i)]) > key(t[(best, best)]) {
                best = i;
            }
        }
        for k in (p..best).rev() {
            swap(t, q, k);
        }
    }
}

/// The eigenvector of the upper triangular T for its i-th diagonal entry, by back
/// substitution, of unit norm: nonzero only in its first i + 1 entries, which it returns. A
/// divisor smaller than ε‖T‖ is replaced by that, as LAPACK's `ztrevc` does, so a repeated
/// eigenvalue gives a vector, not an overflow.
pub(crate) fn eigenvector(t: &Mat<c64>, i: usize) -> Vec<c64> {
    let n = t.nrows();
    let size = (0..n)
        .flat_map(|j| (0..=j).map(move |r| (r, j)))
        .map(|(r, j)| t[(r, j)].norm())
        .fold(0.0, f64::max);
    let smallest = (f64::EPSILON * size).max(f64::MIN_POSITIVE);
    let lambda = t[(i, i)];
    let mut y = vec![c64::new(0.0, 0.0); i + 1];
    y[i] = c64::new(1.0, 0.0);
    for l in (0..i).rev() {
        let s: c64 = (l + 1..=i).map(|p| t[(l, p)] * y[p]).sum();
        let mut d = t[(l, l)] - lambda;
        if d.norm() < smallest {
            d = c64::new(smallest, 0.0);
        }
        y[l] = -s / d;
    }
    let norm = y.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
    y.iter().map(|z| z / norm).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(n: usize, seed: u64) -> Mat<c64> {
        // a fixed pseudo-random complex matrix
        let mut s = seed;
        let mut next = || {
            s = s
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (s >> 11) as f64 / (1u64 << 53) as f64 - 0.5
        };
        Mat::from_fn(n, n, |_, _| c64::new(next(), next()))
    }

    fn worst(a: &Mat<c64>, t: &Mat<c64>, q: &Mat<c64>) -> (f64, f64) {
        let n = a.nrows();
        let back = q * t * q.adjoint();
        let reconstruct = (0..n)
            .flat_map(|j| (0..n).map(move |i| (i, j)))
            .map(|(i, j)| (back[(i, j)] - a[(i, j)]).norm())
            .fold(0.0, f64::max);
        let qq = q.adjoint() * q;
        let unitary = (0..n)
            .flat_map(|j| (0..n).map(move |i| (i, j)))
            .map(|(i, j)| (qq[(i, j)] - if i == j { 1.0 } else { 0.0 }).norm())
            .fold(0.0, f64::max);
        (reconstruct, unitary)
    }

    #[test]
    fn the_schur_form_reconstructs_the_matrix_and_has_its_eigenvalues() {
        for (n, seed) in [(1, 1), (2, 2), (7, 3), (40, 4), (120, 5)] {
            let a = matrix(n, seed);
            let (t, q) = schur(a.clone()).unwrap();
            let (r, u) = worst(&a, &t, &q);
            assert!(r < 1e-12 * n as f64 && u < 1e-12 * n as f64, "{n}: {r} {u}");
            for j in 0..n {
                for i in j + 1..n {
                    assert_eq!(t[(i, j)], c64::new(0.0, 0.0));
                }
            }
            // the diagonal holds faer's eigenvalues
            let mut want: Vec<c64> = a.eigenvalues().unwrap();
            for i in 0..n {
                let d = t[(i, i)];
                let (k, e) = want
                    .iter()
                    .enumerate()
                    .map(|(k, w)| (k, (w - d).norm()))
                    .min_by(|x, y| x.1.total_cmp(&y.1))
                    .unwrap();
                assert!(e < 1e-10, "{n}: {d} {e}");
                want.remove(k);
            }
        }
    }

    #[test]
    fn reordering_keeps_the_form_and_puts_the_largest_first() {
        let a = matrix(30, 9);
        let (mut t, mut q) = schur(a.clone()).unwrap();
        order(&mut t, &mut q, 0, 10, |z| z.norm());
        let (r, u) = worst(&a, &t, &q);
        assert!(r < 1e-12 && u < 1e-12, "{r} {u}");
        let mut all: Vec<f64> = (0..30).map(|i| t[(i, i)].norm()).collect();
        let first: Vec<f64> = all[..10].to_vec();
        all.sort_by(|x, y| y.total_cmp(x));
        assert_eq!(first, all[..10]);
    }

    #[test]
    fn a_triangular_eigenvector_solves_its_eigenproblem() {
        let a = matrix(25, 11);
        let (t, _) = schur(a).unwrap();
        for i in [0, 5, 24] {
            let y = eigenvector(&t, i);
            let worst = (0..=i)
                .map(|r| {
                    let ty: c64 = (r..=i).map(|c| t[(r, c)] * y[c]).sum();
                    (ty - t[(i, i)] * y[r]).norm()
                })
                .fold(0.0, f64::max);
            assert!(worst < 1e-13, "{i}: {worst}");
        }
    }

    #[test]
    fn a_repeated_eigenvalue_gives_independent_schur_vectors() {
        // diag(1, 1, 2, 3) under a unitary similarity: both 1s found, the subspace exact
        let n = 4;
        let (_, u) = schur(matrix(n, 21)).unwrap();
        let d = Mat::from_fn(n, n, |i, j| {
            if i == j {
                c64::new([1.0, 3.0, 1.0, 2.0][i], 0.0)
            } else {
                c64::new(0.0, 0.0)
            }
        });
        let a = &u * &d * u.adjoint();
        let (mut t, mut q) = schur(a.clone()).unwrap();
        order(&mut t, &mut q, 0, 4, |z| -z.norm());
        let diag: Vec<f64> = (0..n).map(|i| t[(i, i)].re).collect();
        for (x, y) in diag.iter().zip([1.0, 1.0, 2.0, 3.0]) {
            assert!((x - y).abs() < 1e-13, "{diag:?}");
        }
        let (r, _) = worst(&a, &t, &q);
        assert!(r < 1e-13);
    }
}
