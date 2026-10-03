//! The quasi-minimal residual method (QMR) for a sparse, non-Hermitian system A x = b.
//!
//! R. W. Freund, N. M. Nachtigal, "QMR: a quasi-minimal residual method for non-Hermitian
//! linear systems", Numer. Math. 60, 315 (1991), doi:10.1007/BF01385726: their Algorithm 3.1,
//! with the Lanczos vectors from their Algorithm 2.1 and the implementation of their Section 4.
//!
//! - **Lanczos** (Algorithm 2.1, regular steps only, Eq. 2.7): two sequences of unit vectors,
//!   v_n spanning K_n(r₀, A) and w_n spanning K_n(w₁, Aᵀ), biorthogonal under the unconjugated
//!   form wᵀv (Eqs. 2.1–2.2, 2.10), from three-term recurrences; w₁ = v₁ = r₀/‖r₀‖ (p. 322).
//!   They satisfy A V⁽ⁿ⁾ = V⁽ⁿ⁺¹⁾ H_e⁽ⁿ⁾ with H_e⁽ⁿ⁾ tridiagonal (Eqs. 2.13–2.14, 3.5).
//! - **Quasi-minimal residual** (Eqs. 3.6–3.9, weights ω_j = 1, Eq. 3.10): x_n = x₀ + V⁽ⁿ⁾z
//!   with z minimizing ‖ρ₀e₁ − H_e⁽ⁿ⁾z‖, by Givens rotations (Eqs. 4.1–4.7), and the iterate
//!   updated by short recurrences through p_n = (v_n − ε_n p_{n−1} − θ_n p_{n−2})/δ_n
//!   (Eqs. 4.8–4.9).
//! - **Stopping** (Eq. 4.10): the residual is updated at the cost of one more vector sum per
//!   iteration (Proposition 4.1b, Eq. 4.12); once it is below the tolerance, the true residual
//!   ‖b − A x_n‖ is computed and checked. (The authors check the bound of Eq. 4.11 first,
//!   √(n+1) times the quasi-residual |τ̃_{n+1}|; it is about 10× pessimistic after 100 steps,
//!   which would add iterations.)
//!
//! The look-ahead steps of Algorithm 2.1 (its inner vectors, from the authors' refs. 6–7), which
//! step over a breakdown of the Lanczos process (wᵀv = 0 for nonzero v and w), are **not**
//! implemented: a breakdown, or a near-breakdown, is reported as an error.

use num_complex::Complex64 as c64;

use crate::{Error, Result};

/// A sparse matrix by rows, with its transpose by rows, for the products QMR takes.
pub(crate) struct Sparse {
    n: usize,
    starts: Vec<usize>,
    columns: Vec<usize>,
    values: Vec<c64>,
    /// The transpose, by rows.
    t_starts: Vec<usize>,
    t_columns: Vec<usize>,
    t_values: Vec<c64>,
}

impl Sparse {
    /// The n × n matrix with these entries, (row, column, value); repeated entries are summed.
    pub(crate) fn new(n: usize, entries: impl IntoIterator<Item = (usize, usize, c64)>) -> Sparse {
        let entries: Vec<(usize, usize, c64)> = entries.into_iter().collect();
        let by_rows = |transposed: bool| {
            let mut sorted: Vec<(usize, usize, c64)> = entries
                .iter()
                .map(|&(r, c, v)| if transposed { (c, r, v) } else { (r, c, v) })
                .collect();
            sorted.sort_by_key(|&(r, c, _)| (r, c));
            let mut starts = vec![0; n + 1];
            let (mut columns, mut values): (Vec<usize>, Vec<c64>) = (Vec::new(), Vec::new());
            let mut last: Option<(usize, usize)> = None;
            for (r, c, v) in sorted {
                if last == Some((r, c)) {
                    *values.last_mut().unwrap() += v;
                } else {
                    columns.push(c);
                    values.push(v);
                    starts[r + 1] += 1;
                    last = Some((r, c));
                }
            }
            for r in 0..n {
                starts[r + 1] += starts[r];
            }
            (starts, columns, values)
        };
        let (starts, columns, values) = by_rows(false);
        let (t_starts, t_columns, t_values) = by_rows(true);
        Sparse {
            n,
            starts,
            columns,
            values,
            t_starts,
            t_columns,
            t_values,
        }
    }

    /// The number of stored entries.
    pub(crate) fn nonzeros(&self) -> usize {
        self.values.len()
    }

    /// A v.
    pub(crate) fn apply(&self, v: &[c64]) -> Vec<c64> {
        product(&self.starts, &self.columns, &self.values, v)
    }

    /// Aᵀ v (the transpose, not the conjugate transpose).
    pub(crate) fn apply_transpose(&self, v: &[c64]) -> Vec<c64> {
        product(&self.t_starts, &self.t_columns, &self.t_values, v)
    }
}

/// A matrix-vector product by rows, the rows shared among the machine's threads: each row is
/// summed in the same order whatever the threads, so the result is the same bit for bit.
fn product(starts: &[usize], columns: &[usize], values: &[c64], v: &[c64]) -> Vec<c64> {
    let n = starts.len() - 1;
    let mut out = vec![c64::new(0.0, 0.0); n];
    let row = |r: usize| -> c64 {
        (starts[r]..starts[r + 1])
            .map(|k| values[k] * v[columns[k]])
            .sum()
    };
    let threads = std::thread::available_parallelism().map_or(1, |t| t.get());
    // below some 10^4 rows a thread costs more than it saves
    if threads == 1 || n < 16_384 {
        for (r, o) in out.iter_mut().enumerate() {
            *o = row(r);
        }
        return out;
    }
    let chunk = n.div_ceil(threads);
    std::thread::scope(|scope| {
        for (c, block) in out.chunks_mut(chunk).enumerate() {
            let row = &row;
            scope.spawn(move || {
                for (k, o) in block.iter_mut().enumerate() {
                    *o = row(c * chunk + k);
                }
            });
        }
    });
    out
}

/// A linear operator QMR can solve with: its products with vectors and its transpose's.
pub(crate) trait Operator {
    /// The number of rows (and columns).
    fn size(&self) -> usize;
    /// A v.
    fn apply(&self, v: &[c64]) -> Vec<c64>;
    /// Aᵀ v (the transpose, not the conjugate transpose).
    fn apply_transpose(&self, v: &[c64]) -> Vec<c64>;
}

impl Operator for Sparse {
    fn size(&self) -> usize {
        self.n
    }

    fn apply(&self, v: &[c64]) -> Vec<c64> {
        Sparse::apply(self, v)
    }

    fn apply_transpose(&self, v: &[c64]) -> Vec<c64> {
        Sparse::apply_transpose(self, v)
    }
}

/// An approximate inverse of a matrix M, for preconditioning: M⁻¹ v and M⁻ᵀ v.
pub(crate) trait Preconditioner {
    /// M⁻¹ v.
    fn solve(&self, v: &[c64]) -> Vec<c64>;
    /// M⁻ᵀ v (the transpose, not the conjugate transpose).
    fn solve_transpose(&self, v: &[c64]) -> Vec<c64>;
}

/// A M⁻¹, the operator of a right-preconditioned system: A M⁻¹ y = b, x = M⁻¹ y. Its residual
/// b − A M⁻¹ y is the original system's, b − A x.
struct RightPreconditioned<'a, A, M> {
    a: &'a A,
    m: &'a M,
}

impl<A: Operator, M: Preconditioner> Operator for RightPreconditioned<'_, A, M> {
    fn size(&self) -> usize {
        self.a.size()
    }

    fn apply(&self, v: &[c64]) -> Vec<c64> {
        self.a.apply(&self.m.solve(v))
    }

    fn apply_transpose(&self, v: &[c64]) -> Vec<c64> {
        self.m.solve_transpose(&self.a.apply_transpose(v))
    }
}

/// Solves A x = b by QMR on the right-preconditioned system A M⁻¹ y = b, x = M⁻¹ y: each
/// iteration takes one product with A, one with Aᵀ, one M⁻¹ and one M⁻ᵀ. The residual it stops
/// on is A x = b's own.
///
/// # Errors
///
/// As [`qmr`].
pub(crate) fn qmr_preconditioned(
    a: &impl Operator,
    m: &impl Preconditioner,
    b: &[c64],
    stopping: Stopping,
) -> Result<(Vec<c64>, Convergence)> {
    let (y, convergence) = qmr(&RightPreconditioned { a, m }, b, stopping)?;
    Ok((m.solve(&y), convergence))
}

/// When QMR stops.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stopping {
    /// The relative residual ‖b − A x‖ / ‖b‖ to reach (Freund and Nachtigal's Eq. 4.10, with
    /// x₀ = 0); 1e-6 is what they and Shin and Fan use.
    pub tolerance: f64,
    /// The most iterations to take (each one product with A and one with Aᵀ).
    pub max_iterations: usize,
}

impl Default for Stopping {
    /// A relative residual of 1e-6 within 100 000 iterations.
    fn default() -> Stopping {
        Stopping {
            tolerance: 1e-6,
            max_iterations: 100_000,
        }
    }
}

/// How a QMR solve went.
#[derive(Clone, Debug, PartialEq)]
pub struct Convergence {
    /// The iterations taken.
    pub iterations: usize,
    /// The true relative residual ‖b − A x‖ / ‖b‖ of the system solved, at the end.
    pub residual: f64,
    /// The relative residual after each iteration, as Freund and Nachtigal's Eq. 4.12 updates
    /// it: the convergence history.
    pub history: Vec<f64>,
}

fn dot(a: &[c64], b: &[c64]) -> c64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn norm(a: &[c64]) -> f64 {
    a.iter().map(|x| x.norm_sqr()).sum::<f64>().sqrt()
}

/// A Givens rotation (c, s) of Freund and Nachtigal's Eq. 4.1, G = [[c, s], [−s̄, c]], applied
/// to (a, b).
fn rotate((c, s): (f64, c64), a: c64, b: c64) -> (c64, c64) {
    (c * a + s * b, -s.conj() * a + c * b)
}

/// Solves A x = b by QMR from x₀ = 0.
///
/// # Errors
///
/// [`Error::InvalidValue`] if `b` doesn't match A, if the tolerance isn't positive, if the
/// Lanczos process breaks down (|w_nᵀ v_n| below 1e-14 for unit vectors: the look-ahead steps
/// that would step over it aren't implemented), or if the tolerance isn't reached within the
/// iterations allowed.
pub(crate) fn qmr(
    a: &impl Operator,
    b: &[c64],
    stopping: Stopping,
) -> Result<(Vec<c64>, Convergence)> {
    let n = a.size();
    if b.len() != n {
        return Err(Error::invalid(
            "qmr",
            format!("the right-hand side needs {n} values, got {}", b.len()),
        ));
    }
    if stopping.tolerance.is_nan() || stopping.tolerance <= 0.0 {
        return Err(Error::invalid("qmr", "the tolerance must be positive"));
    }
    let zero = c64::new(0.0, 0.0);
    let mut x = vec![zero; n];
    let rho0 = norm(b);
    if rho0 == 0.0 {
        return Ok((
            x,
            Convergence {
                iterations: 0,
                residual: 0.0,
                history: Vec::new(),
            },
        ));
    }
    // Algorithm 3.1, step 0: r0 = b, v1 = w1 = r0 / rho0
    let mut v: Vec<c64> = b.iter().map(|z| z / rho0).collect();
    let mut w = v.clone();
    let mut v_old = vec![zero; n];
    let mut w_old = vec![zero; n];
    let mut d = dot(&w, &v);
    let mut d_old = c64::new(1.0, 0.0);
    // rho_n and xi_n, the norms that made v_n and w_n unit vectors (rho_1 = xi_1 = 1)
    let (mut rho, mut xi) = (1.0, 1.0);
    // the two previous rotations and directions p, and tau-tilde (Eq. 4.17: rho0 for omega = 1)
    let mut rotations: [Option<(f64, c64)>; 2] = [None, None];
    let mut p_old = vec![zero; n];
    let mut p_older = vec![zero; n];
    let mut tau = c64::new(rho0, 0.0);
    // the residual r_n, updated by Eq. 4.12 from r0 = b
    let mut r = b.to_vec();
    let mut history = Vec::new();
    for iteration in 1..=stopping.max_iterations {
        if d.norm() < 1e-14 {
            return Err(Error::invalid(
                "qmr",
                format!(
                    "the Lanczos process broke down at step {iteration} (|w^T v| = {:e}); the \
                     look-ahead steps that step over this aren't implemented",
                    d.norm()
                ),
            ));
        }
        // Algorithm 2.1, a regular step (Eq. 2.7) with blocks of one vector: the coefficient of
        // v_{n-1} is w_{n-1}^T A v_n / d_{n-1} = xi_n d_n / d_{n-1}, and of w_{n-1}, rho_n d_n /
        // d_{n-1}, by biorthogonality
        let av = a.apply(&v);
        let atw = a.apply_transpose(&w);
        let alpha = dot(&w, &av) / d;
        let beta_v = xi * d / d_old;
        let beta_w = rho * d / d_old;
        let mut v_next: Vec<c64> = (0..n)
            .map(|k| av[k] - alpha * v[k] - beta_v * v_old[k])
            .collect();
        let mut w_next: Vec<c64> = (0..n)
            .map(|k| atw[k] - alpha * w[k] - beta_w * w_old[k])
            .collect();
        let (rho_next, xi_next) = (norm(&v_next), norm(&w_next));
        // the new column of H_e (Eq. 2.14): beta_v in row n-1, alpha in row n, rho_{n+1} in
        // row n+1; the previous rotations bring it to (theta_n, epsilon_n, mu), Eq. 4.4
        let (mut theta, mut epsilon, mut mu) =
            (zero, if iteration > 1 { beta_v } else { zero }, alpha);
        if let Some(g) = rotations[0] {
            (theta, epsilon) = rotate(g, zero, epsilon);
        }
        if let Some(g) = rotations[1] {
            (epsilon, mu) = rotate(g, epsilon, mu);
        }
        // the new rotation zeroes rho_{n+1} under mu (Eq. 4.5)
        let nu = c64::new(rho_next, 0.0);
        let (c, s) = if mu.norm() > 0.0 {
            let c = mu.norm() / (mu.norm_sqr() + rho_next * rho_next).sqrt();
            (c, (c * nu / mu).conj())
        } else {
            (0.0, c64::new(1.0, 0.0))
        };
        let delta = c * mu + s * nu;
        rotations = [rotations[1], Some((c, s))];
        // Eq. 4.7: tau_n = c tau-tilde_n, tau-tilde_{n+1} = -conj(s) tau-tilde_n
        let tau_n = c * tau;
        tau = -s.conj() * tau;
        // Eqs. 4.8-4.9: p_n = (v_n - epsilon_n p_{n-1} - theta_n p_{n-2}) / delta_n, and
        // x_n = x_{n-1} + tau_n p_n
        let p: Vec<c64> = (0..n)
            .map(|k| (v[k] - epsilon * p_old[k] - theta * p_older[k]) / delta)
            .collect();
        for k in 0..n {
            x[k] += tau_n * p[k];
        }
        p_older = std::mem::replace(&mut p_old, p);
        // Eq. 2.9: the next unit vectors
        let ended = rho_next == 0.0 || xi_next == 0.0;
        if !ended {
            for k in 0..n {
                v_next[k] /= rho_next;
                w_next[k] /= xi_next;
            }
            // Eq. 4.12 with omega = 1: r_n = |s_n|^2 r_{n-1} + c_n tau-tilde_{n+1} v_{n+1}
            let (keep, add) = (s.norm_sqr(), c * tau);
            for k in 0..n {
                r[k] = keep * r[k] + add * v_next[k];
            }
        }
        let updated = norm(&r) / rho0;
        history.push(updated);
        if updated <= stopping.tolerance || ended {
            // Eq. 4.10, on the true residual
            let ax = a.apply(&x);
            let true_residual: Vec<c64> = (0..n).map(|k| b[k] - ax[k]).collect();
            let residual = norm(&true_residual) / rho0;
            if residual <= stopping.tolerance {
                return Ok((
                    x,
                    Convergence {
                        iterations: iteration,
                        residual,
                        history,
                    },
                ));
            }
            if ended {
                return Err(Error::invalid(
                    "qmr",
                    format!(
                        "the Lanczos process ended at step {iteration} with the residual at                          {residual:e}"
                    ),
                ));
            }
            // the updated residual has drifted from the true one: carry on from the true one
            r = true_residual;
        }
        v_old = std::mem::replace(&mut v, v_next);
        w_old = std::mem::replace(&mut w, w_next);
        (rho, xi) = (rho_next, xi_next);
        d_old = d;
        d = dot(&w, &v);
    }
    Err(Error::invalid(
        "qmr",
        format!(
            "no convergence to {:e} in {} iterations (the residual is at {:e})",
            stopping.tolerance,
            stopping.max_iterations,
            history.last().copied().unwrap_or(f64::NAN)
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A non-Hermitian, non-symmetric tridiagonal matrix: −1 − 0.3, 4 + i, −1 + 0.3 (a
    /// convection–diffusion operator with a complex shift), n × n.
    fn tridiagonal(n: usize) -> Sparse {
        let mut e = Vec::new();
        for r in 0..n {
            e.push((r, r, c64::new(4.0, 1.0)));
            if r > 0 {
                e.push((r, r - 1, c64::new(-1.3, 0.0)));
            }
            if r + 1 < n {
                e.push((r, r + 1, c64::new(-0.7, 0.0)));
            }
        }
        Sparse::new(n, e)
    }

    fn dense_solve(a: &Sparse, b: &[c64]) -> Vec<c64> {
        use faer::linalg::solvers::Solve;
        let n = a.size();
        let mut m = faer::Mat::<c64>::zeros(n, n);
        for r in 0..n {
            for k in a.starts[r]..a.starts[r + 1] {
                m[(r, a.columns[k])] += a.values[k];
            }
        }
        let rhs = faer::Mat::<c64>::from_fn(n, 1, |r, _| b[r]);
        let x = m.full_piv_lu().solve(&rhs);
        (0..n).map(|r| x[(r, 0)]).collect()
    }

    #[test]
    fn qmr_solves_a_non_hermitian_system_as_lu_does() {
        let a = tridiagonal(200);
        let b: Vec<c64> = (0..200)
            .map(|k| c64::new((k as f64 * 0.37).sin(), (k as f64 * 0.11).cos()))
            .collect();
        let stopping = Stopping {
            tolerance: 1e-12,
            max_iterations: 1000,
        };
        let (x, how) = qmr(&a, &b, stopping).unwrap();
        let exact = dense_solve(&a, &b);
        let error = x
            .iter()
            .zip(&exact)
            .map(|(p, q)| (p - q).norm())
            .fold(0.0, f64::max)
            / exact.iter().map(|q| q.norm()).fold(0.0, f64::max);
        assert!(how.residual <= 1e-12, "{how:?}");
        assert!(error < 1e-10, "{error}");
        // the history is one value per iteration, and the updated residual the true one
        assert_eq!(how.history.len(), how.iterations);
        let updated = how.history.last().unwrap();
        assert!((how.residual - updated).abs() < 1e-3 * updated, "{how:?}");
    }

    #[test]
    fn the_threaded_product_is_the_sequential_one_bit_for_bit() {
        // past the threshold for threads: the rows are shared out, each summed in order
        let n = 40_000;
        let a = Sparse::new(
            n,
            (0..n).flat_map(|r| {
                [0usize, 7, 911].map(move |d| {
                    let c = (r + d) % n;
                    (
                        r,
                        c,
                        c64::new((r as f64 * 0.1).sin(), (c as f64 * 0.3).cos()),
                    )
                })
            }),
        );
        let v: Vec<c64> = (0..n)
            .map(|k| c64::new((k as f64).sqrt(), -(k as f64 * 0.7).sin()))
            .collect();
        let sequential: Vec<c64> = (0..n)
            .map(|r| {
                (a.starts[r]..a.starts[r + 1])
                    .map(|k| a.values[k] * v[a.columns[k]])
                    .sum()
            })
            .collect();
        assert!(a.apply(&v) == sequential);
    }

    #[test]
    fn the_transpose_is_the_transpose() {
        let a = Sparse::new(
            3,
            [
                (0, 1, c64::new(2.0, 1.0)),
                (2, 0, c64::new(-1.0, 0.5)),
                (2, 0, c64::new(1.0, 0.0)),
                (1, 1, c64::new(0.0, 3.0)),
            ],
        );
        assert_eq!(a.nonzeros(), 3);
        let v = [c64::new(1.0, 0.0), c64::new(0.0, 1.0), c64::new(2.0, -1.0)];
        let (av, atv) = (a.apply(&v), a.apply_transpose(&v));
        assert_eq!(av[0], c64::new(2.0, 1.0) * v[1]);
        assert_eq!(av[2], c64::new(0.0, 0.5) * v[0]);
        assert_eq!(atv[0], c64::new(0.0, 0.5) * v[2]);
        assert_eq!(
            atv[1],
            c64::new(2.0, 1.0) * v[0] + c64::new(0.0, 3.0) * v[1]
        );
    }

    #[test]
    fn a_breakdown_and_bad_input_are_errors() {
        // b = (1, i): w1^T v1 = (1 + i^2) / 2 = 0, a breakdown at the first step
        let identity = Sparse::new(2, [(0, 0, c64::new(1.0, 0.0)), (1, 1, c64::new(2.0, 0.0))]);
        let b = [c64::new(1.0, 0.0), c64::new(0.0, 1.0)];
        assert!(qmr(&identity, &b, Stopping::default()).is_err());
        assert!(qmr(&identity, &b[..1], Stopping::default()).is_err());
        let bad = Stopping {
            tolerance: 0.0,
            ..Stopping::default()
        };
        assert!(qmr(&identity, &[c64::new(1.0, 0.0); 2], bad).is_err());
        // too few iterations
        let a = tridiagonal(100);
        let few = Stopping {
            tolerance: 1e-12,
            max_iterations: 3,
        };
        assert!(qmr(&a, &[c64::new(1.0, 0.0); 100], few).is_err());
        // a zero right-hand side is solved by zero
        let (x, how) = qmr(&a, &[c64::new(0.0, 0.0); 100], Stopping::default()).unwrap();
        assert!(x.iter().all(|v| v.norm() == 0.0) && how.iterations == 0);
    }
}
