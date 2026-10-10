//! Eigenvalues of large sparse matrices near a shift: shift-and-invert Arnoldi with
//! Krylov–Schur restarts.
//!
//! Y. Saad, *Numerical Methods for Large Eigenvalue Problems*, 2nd ed., SIAM (2011),
//! [doi:10.1137/1.9781611970739](https://doi.org/10.1137/1.9781611970739) (free from the
//! author's site): Arnoldi's method with modified Gram–Schmidt (Algorithm 6.2) and a second
//! orthogonalization when cancellation is severe (Section 6.2.2), applied to (A − σI)⁻¹ so that
//! the eigenvalues nearest σ converge first (Section 8.1.3).
//!
//! Restarted by G. W. Stewart's Krylov–Schur method (SIAM J. Matrix Anal. Appl. 23, 601 (2002),
//! online 2001, [doi:10.1137/S0895479800371529](https://doi.org/10.1137/S0895479800371529)):
//! the Krylov space grows to m vectors (Section 3, (3.1)), its Rayleigh quotient is brought to
//! Schur form (3.3), the wanted Ritz values (the largest of (A − σI)⁻¹) are moved to its
//! leading corner by exchanges of neighbours, and the decomposition is truncated to them
//! (3.4), which keeps a Krylov decomposition (Definition 2.1) to grow again. So the space
//! never holds more than m + 1 vectors however many restarts it takes, and every wanted
//! direction found so far is kept, not just their sum. A Ritz pair's residual comes from the
//! decomposition itself; when every wanted one is small, it is accepted only when its true
//! residual ‖Ax − λx‖ / (|λ| ‖x‖) is small too, computed with A itself.

pub(crate) mod checks;
pub(crate) mod contour;
mod growing;
mod schur;

pub(crate) use growing::nearest_growing;

use faer::linalg::matmul::matmul;
use faer::linalg::solvers::Solve;
use faer::sparse::{SparseColMat, Triplet};
use faer::{Accum, Mat, Par, c64};

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

/// A − σI factorized by faer's sparse LU, and A by rows.
pub(crate) struct Shifted {
    n: usize,
    rows: Vec<Vec<(usize, c64)>>,
    shift: c64,
    lu: faer::sparse::linalg::solvers::Lu<usize, c64>,
}

impl Shifted {
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the shifted matrix is singular.
    pub(crate) fn new(n: usize, entries: &[(usize, usize, c64)], shift: c64) -> Result<Shifted> {
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
        Ok(Shifted { n, rows, shift, lu })
    }

    /// (A − σI)⁻¹ v.
    pub(crate) fn solve(&self, v: &[c64]) -> Vec<c64> {
        let mut rhs = Mat::<c64>::from_fn(self.n, 1, |i, _| v[i]);
        self.lu.solve_in_place(rhs.as_mut());
        (0..self.n).map(|i| rhs[(i, 0)]).collect()
    }

    /// ‖A x − λ x‖.
    pub(crate) fn residual(&self, value: c64, x: &[c64]) -> f64 {
        apply(&self.rows, x)
            .iter()
            .zip(x)
            .map(|(a, b)| (a - value * b).norm_sqr())
            .sum::<f64>()
            .sqrt()
    }

    /// ‖(A − σI) v‖.
    fn shifted_norm(&self, v: &[c64]) -> f64 {
        self.residual(self.shift, v)
    }
}

/// A fixed start vector: results don't depend on chance.
fn start(n: usize) -> Vec<c64> {
    (0..n)
        .map(|i| {
            c64::new(
                1.0 + 0.5 * (i as f64 * 0.7).sin(),
                0.25 * (i as f64 * 1.3).cos(),
            )
        })
        .collect()
}

/// What a Krylov–Schur solve took.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Record {
    /// Restarts: truncations of the decomposition, one per growth but the last.
    pub restarts: usize,
    /// Solves with the factors of A − σI, one per step of the Arnoldi process.
    pub solves: usize,
    /// The most basis vectors of length n held at once.
    pub held: usize,
    /// The largest residual of a wanted Ritz pair as the decomposition gives it, over the
    /// tolerance, after each growth: below 1 when they have all converged.
    pub history: Vec<f64>,
    /// The wanted Ritz values, as eigenvalues of A, after each growth.
    pub values: Vec<Vec<c64>>,
    /// Their residuals ‖Ax − λx‖ / (max(|λ|, 1) ‖x‖) as the decomposition gives them.
    pub residuals: Vec<Vec<f64>>,
}

/// The most growths of the Krylov space before giving up.
const GROWTHS: usize = 200;

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
    // (never asked to stop, it always has an answer or an error)
    Ok(nearest_until(n, entries, shift, count, tolerance, &|| false)?.unwrap_or_default())
}

/// As [`nearest`], giving up when `stop` says so: `None` then. It is asked before each step of
/// the Arnoldi process (one solve with the factors each); the factorization of the shifted
/// matrix, before the first step, and the small dense work of a restart can't be interrupted.
///
/// # Errors
///
/// As [`nearest`].
pub(crate) fn nearest_until(
    n: usize,
    entries: &[(usize, usize, c64)],
    shift: c64,
    count: usize,
    tolerance: f64,
    stop: &dyn Fn() -> bool,
) -> Result<Option<Vec<Pair>>> {
    Ok(krylov_schur(n, entries, shift, count, tolerance, None, stop)?.map(|(pairs, _)| pairs))
}

/// The Krylov space's size for `count` eigenvalues of an n × n matrix, and how many vectors a
/// restart keeps: m = max(2 count + 20, 40), as the growing restarts' first run had, and half
/// the unwanted ones besides the wanted, so each growth takes (m − count)/2 steps.
pub(crate) fn sizes(n: usize, count: usize) -> (usize, usize) {
    sizes_of(n, count, (2 * count + 20).max(40))
}

/// As [`sizes`], for a space of `m` vectors (at least `count` + 1).
fn sizes_of(n: usize, count: usize, m: usize) -> (usize, usize) {
    let m = n.min(m.max(count + 1));
    let keep = (count + (m - count) / 2).min(m.saturating_sub(1)).max(1);
    (m, keep)
}

/// As [`nearest_until`], with what the solve took; in a Krylov space of `space` vectors if
/// given, else of [`sizes`]'.
///
/// # Errors
///
/// As [`nearest`].
pub(crate) fn krylov_schur(
    n: usize,
    entries: &[(usize, usize, c64)],
    shift: c64,
    count: usize,
    tolerance: f64,
    space: Option<usize>,
    stop: &dyn Fn() -> bool,
) -> Result<Option<(Vec<Pair>, Record)>> {
    let mut record = Record::default();
    if count == 0 || n == 0 {
        return Ok(Some((Vec::new(), record)));
    }
    if stop() {
        return Ok(None);
    }
    let count = count.min(n);
    let op = Shifted::new(n, entries, shift)?;
    let (m, keep) = match space {
        Some(m) => sizes_of(n, count, m),
        None => sizes(n, count),
    };
    let s = start(n);
    let sn = norm(&s);
    let mut basis: Vec<Vec<c64>> = Vec::with_capacity(m + 1);
    basis.push(s.iter().map(|x| x / sn).collect());
    // (A − σI)⁻¹ U = U S + u bᴴ held as its (m + 1) × m array: S above, bᴴ in the row below
    // the decomposition's order k
    let mut h = Mat::<c64>::zeros(m + 1, m);
    let mut k = 0;
    for _ in 0..GROWTHS {
        // grow the space from k to m vectors (Stewart's (3.1))
        let mut steps = m;
        for j in k..m {
            if stop() {
                return Ok(None);
            }
            let mut w = op.solve(&basis[j]);
            record.solves += 1;
            let before = norm(&w);
            for pass in 0..2 {
                for (i, vi) in basis.iter().enumerate() {
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
            if hn <= 1e-14 * before.max(f64::MIN_POSITIVE) {
                // an invariant subspace: go on from a new direction, or stop if there is none
                h[(j + 1, j)] = c64::new(0.0, 0.0);
                if let Some(v) = fresh(&basis, j) {
                    basis.push(v);
                } else {
                    basis.push(vec![c64::new(0.0, 0.0); n]);
                    steps = j + 1;
                    break;
                }
            } else {
                h[(j + 1, j)] = c64::new(hn, 0.0);
                basis.push(w.iter().map(|x| x / hn).collect());
            }
            record.held = record.held.max(basis.len());
        }
        record.held = record.held.max(basis.len());
        // the Rayleigh quotient in Schur form, the wanted Ritz values first (3.3)
        let quotient = Mat::<c64>::from_fn(steps, steps, |i, j| h[(i, j)]);
        let (mut t, mut q) = schur::schur(quotient)?;
        let want = count.min(steps);
        let kept = keep.min(steps).max(want);
        schur::order(&mut t, &mut q, 0, kept, |z| z.norm());
        // bᴴ Q, the residual row in the Schur basis
        let b: Vec<c64> = (0..steps)
            .map(|c| (0..steps).map(|i| h[(steps, i)] * q[(i, c)]).sum())
            .collect();
        // a Ritz pair (θ, U Q y) of (A − σI)⁻¹ has residual (bᴴQ y) u, so as an eigenpair of A,
        // λ = σ + 1/θ, its residual is −(bᴴQ y / θ)(A − σI) u
        let size = op.shifted_norm(&basis[steps]);
        let mut ritz = Vec::with_capacity(want);
        let mut residuals = Vec::with_capacity(want);
        let mut worst = 0.0f64;
        for i in 0..want {
            let y = schur::eigenvector(&t, i);
            let rho: c64 = y.iter().zip(&b).map(|(y, b)| y * b).sum();
            let theta = t[(i, i)];
            let value = shift + c64::new(1.0, 0.0) / theta;
            let estimate = rho.norm() * size / theta.norm();
            let relative = estimate / value.norm().max(1.0);
            worst = worst.max(relative / tolerance);
            residuals.push(relative);
            ritz.push((value, y));
        }
        record.history.push(worst);
        record.values.push(ritz.iter().map(|r| r.0).collect());
        record.residuals.push(residuals);
        // truncate to the kept Schur vectors (3.4): U ← U Q[:, ..kept], u kept
        combine(&mut basis, steps, &q, kept);
        h.fill(c64::new(0.0, 0.0));
        for j in 0..kept {
            for i in 0..=j {
                h[(i, j)] = t[(i, j)];
            }
            h[(kept, j)] = b[j];
        }
        k = kept;
        if worst <= 1.0 {
            // each wanted pair's true residual, with A itself
            let mut pairs = Vec::with_capacity(want);
            for (value, y) in ritz {
                let mut x = vec![c64::new(0.0, 0.0); n];
                for (c, yc) in y.iter().enumerate() {
                    for (xj, vj) in x.iter_mut().zip(&basis[c]) {
                        *xj += yc * vj;
                    }
                }
                let xn = norm(&x);
                x.iter_mut().for_each(|z| *z /= xn);
                if op.residual(value, &x) > tolerance * value.norm().max(1.0) {
                    break;
                }
                pairs.push(Pair { value, vector: x });
            }
            if pairs.len() == want {
                return Ok(Some((pairs, record)));
            }
        }
        record.restarts += 1;
    }
    Err(Error::invalid(
        "eigenproblem",
        format!(
            "{count} eigenpairs near {shift} didn't converge in {GROWTHS} restarts of a Krylov \
             space of {m}"
        ),
    ))
}

/// A unit vector orthogonal to `basis`, to go on from an invariant subspace: a fixed one, the
/// `j`-th of its kind; `None` if the basis already spans the space.
fn fresh(basis: &[Vec<c64>], j: usize) -> Option<Vec<c64>> {
    let n = basis[0].len();
    let mut w: Vec<c64> = (0..n)
        .map(|i| {
            let (a, b) = (i as f64 + 1.0, j as f64 + 2.0);
            c64::new((0.37 * a * b).sin(), (0.61 * a + 1.7 * b).cos())
        })
        .collect();
    let before = norm(&w);
    for _ in 0..2 {
        for vi in basis {
            let c = dot(&w, vi);
            for (wk, vk) in w.iter_mut().zip(vi) {
                *wk -= c * vk;
            }
        }
    }
    let after = norm(&w);
    (after > 1e-8 * before).then(|| w.iter().map(|x| x / after).collect())
}

/// basis[..kept] ← basis[..steps] Q[:, ..kept], and basis[kept] ← basis[steps], the rest
/// dropped: a block of rows at a time, by faer's product on one thread.
fn combine(basis: &mut Vec<Vec<c64>>, steps: usize, q: &Mat<c64>, kept: usize) {
    const BLOCK: usize = 512;
    let n = basis[0].len();
    let qk = q.as_ref().subcols(0, kept);
    let mut from = Mat::<c64>::zeros(BLOCK.min(n), steps);
    let mut to = Mat::<c64>::zeros(BLOCK.min(n), kept);
    for first in (0..n).step_by(BLOCK) {
        let r = BLOCK.min(n - first);
        for (c, v) in basis.iter().take(steps).enumerate() {
            for i in 0..r {
                from[(i, c)] = v[first + i];
            }
        }
        matmul(
            to.as_mut().subrows_mut(0, r),
            Accum::Replace,
            from.as_ref().subrows(0, r),
            qk,
            c64::new(1.0, 0.0),
            Par::Seq,
        );
        for (c, v) in basis.iter_mut().take(kept).enumerate() {
            for i in 0..r {
                v[first + i] = to[(i, c)];
            }
        }
    }
    basis.swap(kept, steps);
    basis.truncate(kept + 1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_many_eigenvalues_among_close_ones() {
        // the 2D Laplacian on a 24 x 20 grid, whose eigenvalues are known and lie close
        // together: 6, 12 and 30 of them, each the exact one. (The case that needed the
        // restarts' Krylov space to grow is a waveguide's cross-section: see
        // job's a_modes_job_finds_eight_modes.)
        let (nx, ny) = (24, 20);
        let n = nx * ny;
        let mut entries = Vec::new();
        for j in 0..ny {
            for i in 0..nx {
                let r = j * nx + i;
                entries.push((r, r, c64::new(4.0, 0.0)));
                if i > 0 {
                    entries.push((r, r - 1, c64::new(-1.0, 0.0)));
                }
                if i + 1 < nx {
                    entries.push((r, r + 1, c64::new(-1.0, 0.0)));
                }
                if j > 0 {
                    entries.push((r, r - nx, c64::new(-1.0, 0.0)));
                }
                if j + 1 < ny {
                    entries.push((r, r + nx, c64::new(-1.0, 0.0)));
                }
            }
        }
        // exactly: 4 − 2cos(pπ/(nx+1)) − 2cos(qπ/(ny+1))
        let mut exact: Vec<f64> = (1..=nx)
            .flat_map(|p| (1..=ny).map(move |q| (p, q)))
            .map(|(p, q)| {
                let pi = std::f64::consts::PI;
                4.0 - 2.0 * (p as f64 * pi / (nx + 1) as f64).cos()
                    - 2.0 * (q as f64 * pi / (ny + 1) as f64).cos()
            })
            .collect();
        exact.sort_by(f64::total_cmp);
        for count in [6, 12, 30] {
            let pairs = nearest(n, &entries, c64::new(0.0, 0.0), count, 1e-10)
                .unwrap_or_else(|e| panic!("{count}: {e}"));
            let mut found: Vec<f64> = pairs.iter().map(|p| p.value.re).collect();
            found.sort_by(f64::total_cmp);
            for (f, e) in found.iter().zip(&exact) {
                assert!((f - e).abs() < 1e-8, "{count}: {f} vs {e}");
            }
        }
    }

    #[test]
    fn gives_up_when_asked_and_is_the_same_when_not() {
        let n = 200;
        let entries: Vec<(usize, usize, c64)> = (0..n)
            .map(|i| (i, i, c64::new(i as f64 + 1.0, 0.0)))
            .collect();
        let shift = c64::new(50.3, 0.0);
        // asked before the factorization: nothing is done
        assert!(
            nearest_until(n, &entries, shift, 3, 1e-10, &|| true)
                .unwrap()
                .is_none()
        );
        // asked to stop at the fourth question: it stops there, three steps in, not after the
        // forty or so a run of the process takes
        let asked = std::cell::Cell::new(0);
        let stop = || {
            asked.set(asked.get() + 1);
            asked.get() > 4
        };
        assert!(
            nearest_until(n, &entries, shift, 3, 1e-10, &stop)
                .unwrap()
                .is_none()
        );
        assert_eq!(asked.get(), 5);
        // never asked to stop: the pairs `nearest` gives
        let whole = nearest(n, &entries, shift, 3, 1e-10).unwrap();
        let until = nearest_until(n, &entries, shift, 3, 1e-10, &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(whole.len(), until.len());
        for (a, b) in whole.iter().zip(&until) {
            assert_eq!((a.value, &a.vector), (b.value, &b.vector));
        }
    }

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
