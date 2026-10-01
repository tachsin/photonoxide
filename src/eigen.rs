//! Eigenvalues of large sparse matrices near a shift: shift-and-invert Arnoldi.
//!
//! Y. Saad, *Numerical Methods for Large Eigenvalue Problems*, 2nd ed., SIAM (2011),
//! [doi:10.1137/1.9781611970739](https://doi.org/10.1137/1.9781611970739) (free from the
//! author's site): Arnoldi's method with modified Gram–Schmidt (Algorithm 6.2) and a second
//! orthogonalization when cancellation is severe (Section 6.2.2), applied to (A − σI)⁻¹ so that
//! the eigenvalues nearest σ converge first (Section 8.1.3), restarted from the wanted Ritz
//! vectors until they converge (Algorithm 6.3, with several vectors combined). A Ritz pair is
//! accepted only when its true residual ‖Ax − λx‖ / (|λ| ‖x‖) is small, computed with A itself.

use faer::linalg::solvers::Solve;
use faer::sparse::{SparseColMat, Triplet};
use faer::{Mat, c64};

use crate::{Error, Result};

/// An eigenpair of A.
pub(crate) struct Pair {
    pub value: c64,
    pub vector: Vec<c64>,
}

fn dot(a: &[c64], b: &[c64]) -> c64 {
    // (a, b) = Σ conj(b_i) a_i, Saad's inner product
    a.iter().zip(b).map(|(x, y)| x * y.conj()).sum()
}

fn norm(a: &[c64]) -> f64 {
    a.iter().map(|x| x.norm_sqr()).sum::<f64>().sqrt()
}

/// y = A x for A given as triplets grouped by row.
fn apply(rows: &[Vec<(usize, c64)>], x: &[c64]) -> Vec<c64> {
    rows.iter()
        .map(|row| row.iter().map(|&(j, v)| v * x[j]).sum())
        .collect()
}

/// The `count` eigenvalues of the n × n matrix given by `entries` (row, column, value) nearest
/// `shift`, with their eigenvectors, nearest first.
///
/// # Errors
///
/// [`Error::InvalidValue`] if the shifted matrix is singular or the eigenpairs don't converge
/// within the restarts.
pub(crate) fn nearest(
    n: usize,
    entries: &[(usize, usize, c64)],
    shift: c64,
    count: usize,
    tolerance: f64,
) -> Result<Vec<Pair>> {
    if count == 0 || n == 0 {
        return Ok(Vec::new());
    }
    let count = count.min(n);
    let mut rows = vec![Vec::new(); n];
    let mut shifted = Vec::with_capacity(entries.len() + n);
    for &(i, j, v) in entries {
        rows[i].push((j, v));
        shifted.push(Triplet::new(i, j, v));
    }
    for i in 0..n {
        shifted.push(Triplet::new(i, i, -shift));
    }
    let matrix = SparseColMat::<usize, c64>::try_new_from_triplets(n, n, &shifted)
        .map_err(|e| Error::invalid("eigenproblem", format!("{e:?}")))?;
    let lu = matrix.sp_lu().map_err(|e| {
        Error::invalid(
            "eigenproblem",
            format!("the shifted matrix is singular: {e:?}"),
        )
    })?;
    let solve = |v: &[c64]| -> Vec<c64> {
        let mut rhs = Mat::<c64>::from_fn(n, 1, |i, _| v[i]);
        lu.solve_in_place(rhs.as_mut());
        (0..n).map(|i| rhs[(i, 0)]).collect()
    };

    let m = n.min((2 * count + 20).max(40));
    // a fixed start vector: results don't depend on chance
    let mut start: Vec<c64> = (0..n)
        .map(|i| {
            c64::new(
                1.0 + 0.5 * (i as f64 * 0.7).sin(),
                0.25 * (i as f64 * 1.3).cos(),
            )
        })
        .collect();
    let mut best: Vec<Pair> = Vec::new();
    for _restart in 0..20 {
        let s = norm(&start);
        let mut v: Vec<Vec<c64>> = vec![start.iter().map(|x| x / s).collect()];
        let mut h = Mat::<c64>::zeros(m + 1, m);
        let mut steps = m;
        for j in 0..m {
            let mut w = solve(&v[j]);
            let before = norm(&w);
            for pass in 0..2 {
                for (i, vi) in v.iter().enumerate() {
                    let hij = dot(&w, vi);
                    h[(i, j)] += hij;
                    for (wk, vk) in w.iter_mut().zip(vi) {
                        *wk -= hij * vk;
                    }
                }
                // Saad 6.2.2: orthogonalize again only after severe cancellation
                if pass == 0 && norm(&w) > 0.7 * before {
                    break;
                }
            }
            let hn = norm(&w);
            h[(j + 1, j)] = c64::new(hn, 0.0);
            if hn <= 1e-14 * before.max(f64::MIN_POSITIVE) {
                steps = j + 1; // an invariant subspace: the Ritz values are exact
                break;
            }
            v.push(w.iter().map(|x| x / hn).collect());
        }
        let hm = Mat::<c64>::from_fn(steps, steps, |i, j| h[(i, j)]);
        let eig = hm
            .eigen()
            .map_err(|e| Error::invalid("eigenproblem", format!("{e:?}")))?;
        let theta = eig.S();
        let y = eig.U();
        // the largest |θ| of (A − σ)⁻¹ are the eigenvalues of A nearest σ
        let mut order: Vec<usize> = (0..steps).collect();
        order.sort_by(|&a, &b| theta[b].norm().total_cmp(&theta[a].norm()));
        let mut pairs = Vec::with_capacity(count);
        let mut all = true;
        for &k in order.iter().take(count) {
            let value = shift + c64::new(1.0, 0.0) / theta[k];
            let mut x = vec![c64::new(0.0, 0.0); n];
            for (i, vi) in v.iter().take(steps).enumerate() {
                let c = y[(i, k)];
                for (xj, vj) in x.iter_mut().zip(vi) {
                    *xj += c * vj;
                }
            }
            let xn = norm(&x);
            let x: Vec<c64> = x.iter().map(|z| z / xn).collect();
            let ax = apply(&rows, &x);
            let r: f64 = ax
                .iter()
                .zip(&x)
                .map(|(a, b)| (a - value * b).norm_sqr())
                .sum::<f64>()
                .sqrt();
            if r > tolerance * value.norm().max(1.0) {
                all = false;
            }
            pairs.push(Pair { value, vector: x });
        }
        if all {
            return Ok(pairs);
        }
        // restart from the wanted Ritz vectors, combined
        start = vec![c64::new(0.0, 0.0); n];
        for p in &pairs {
            for (s, x) in start.iter_mut().zip(&p.vector) {
                *s += x;
            }
        }
        best = pairs;
    }
    let worst = best.len();
    Err(Error::invalid(
        "eigenproblem",
        format!("{worst} eigenpairs near {shift} didn't converge in 20 restarts"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_eigenvalues_nearest_the_shift() {
        // a diagonal matrix with eigenvalues 1..=200, plus a symmetric coupling that keeps
        // them distinct: the nearest to 50.3 are 50, 51, 49
        let n = 200;
        let mut entries = Vec::new();
        for i in 0..n {
            entries.push((i, i, c64::new(i as f64 + 1.0, 0.0)));
        }
        let pairs = nearest(n, &entries, c64::new(50.3, 0.0), 3, 1e-10).unwrap();
        let values: Vec<f64> = pairs.iter().map(|p| p.value.re).collect();
        assert!(
            (values[0] - 50.0).abs() < 1e-9 && (values[1] - 51.0).abs() < 1e-9,
            "{values:?}"
        );
        assert!((values[2] - 49.0).abs() < 1e-9, "{values:?}");
    }

    #[test]
    fn handles_a_non_hermitian_complex_matrix() {
        // the 1D Laplacian with a complex potential: compare with a dense eigensolve
        let n = 60;
        let mut entries = Vec::new();
        for i in 0..n {
            entries.push((
                i,
                i,
                c64::new(2.0 + 0.01 * i as f64, 0.05 * (i as f64).sin()),
            ));
            if i + 1 < n {
                entries.push((i, i + 1, c64::new(-1.0, 0.0)));
                entries.push((i + 1, i, c64::new(-1.0, 0.1)));
            }
        }
        let dense = Mat::<c64>::from_fn(n, n, |i, j| {
            entries
                .iter()
                .filter(|e| e.0 == i && e.1 == j)
                .map(|e| e.2)
                .sum()
        });
        let all = dense.eigenvalues().unwrap();
        let shift = c64::new(0.5, 0.0);
        let mut want: Vec<c64> = all.clone();
        want.sort_by(|a, b| (a - shift).norm().total_cmp(&(b - shift).norm()));
        let pairs = nearest(n, &entries, shift, 4, 1e-10).unwrap();
        for (p, w) in pairs.iter().zip(&want) {
            assert!((p.value - w).norm() < 1e-9, "{} vs {}", p.value, w);
        }
    }
}
