//! Iterative refinement in double precision of a factorization in single: GMRES-based
//! iterative refinement, GMRES-IR (E. Carson, N. J. Higham, "Accelerating the solution of linear
//! systems by iterative refinement in three precisions", SIAM J. Sci. Comput. 40, A817 (2018),
//! doi:10.1137/17M1140819, Algorithm 8.1; P. Amestoy, A. Buttari, N. J. Higham,
//! J.-Y. L'Excellent, T. Mary, B. Vieublé, "Combining sparse approximate factorizations with
//! mixed-precision iterative refinement", ACM Trans. Math. Softw. 49, 4 (2023),
//! doi:10.1145/3582493, Algorithm 2).
//!
//! The factors M ≈ A are single precision (u_f = 2⁻²⁴); everything else is double (u = 2⁻⁵³):
//! the first solve x₀ = M⁻¹ b, each residual rᵢ = b − A xᵢ, and the correction's GMRES on
//! M⁻¹ A dᵢ = M⁻¹ rᵢ, whose products read the single-precision factors in double arithmetic.
//! In Amestoy et al.'s five precisions that is (u_f, u, u_r, u_g, u_p) = (s, d, d, d, d), one of
//! the variants they run on MUMPS (their Section 5.2). The factors stay single in memory: the
//! solves widen each entry as they read it, the cast "on the fly" they describe (Section 5.1).
//!
//! **When it converges.** With a stable factorization, refinement by the factors alone (LU-IR)
//! is guaranteed only for κ(A) u_f ≪ 1; GMRES-IR's condition (their Eq. 4.2,
//! (u_g + u_p κ(A))(κ(A)² u_f² + 1) ≪ 1) is weaker, so it solves systems the factors alone
//! can't. Static pivoting's perturbed pivots and the factorization's growth enter as Amestoy et
//! al.'s Theorem 4.1 says, through (u_f ρ_n + ε). Past those conditions it fails, and the
//! failure is the point of the checks below: N. J. Higham and T. Mary, "Mixed precision
//! algorithms in numerical linear algebra", Acta Numerica 31, 347 (2022),
//! doi:10.1017/S0962492922000022, Section 7, set out where low-precision factors stop being
//! enough.
//!
//! **When it stops.** Each iterate's normwise backward error,
//! η(x) = ‖b − A x‖∞ / (‖A‖∞ ‖x‖∞ + ‖b‖∞) (Carson and Higham's Section 2), is computed in
//! double. The refinement has converged when η ≤ p u, p the most nonzeros in a row of [A b]:
//! the rounding of the residual itself, below which a computed η says nothing more (the p of
//! Higham and Mary's limiting accuracy, their Eq. 6.4). A step that doesn't halve η, or
//! [`STEPS`] steps without converging, is a failure, returned with every η and GMRES's
//! iterations in every step: never an iterate that only looks like an answer.
//!
//! GMRES itself (Y. Saad, *Iterative Methods for Sparse Linear Systems*, 2nd ed., SIAM (2003),
//! doi:10.1137/1.9780898718003, Algorithm 9.4: left preconditioned, modified Gram–Schmidt; its
//! small least-squares problem by plane rotations as the basis grows) runs until it has
//! reduced the preconditioned residual by [`INNER_TOLERANCE`],
//! Amestoy et al.'s τ_g, or for [`INNER_ITERATIONS`]; its last iterate is the correction either
//! way, and the next step's η says whether it helped.

use num_complex::Complex64 as c64;

/// GMRES's stopping test inside each step: the preconditioned residual reduced by this, as
/// Amestoy et al. set τ_g in all their experiments (Section 5.2).
pub(crate) const INNER_TOLERANCE: f64 = 1e-6;

/// GMRES's iterations in a step at most, its Krylov basis this many vectors of the system's
/// size: well beyond the "few dozen" Amestoy et al. rarely saw exceeded (Section 5.2), and a
/// small fraction of the factors' memory on the 3D problems the single-precision factors are
/// for.
pub(crate) const INNER_ITERATIONS: usize = 60;

/// Refinement steps at most, after the first solve.
pub(crate) const STEPS: usize = 20;

/// Double precision's unit roundoff, 2⁻⁵³.
const UNIT_ROUNDOFF: f64 = f64::EPSILON / 2.0;

/// How a refinement went.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Refinement {
    /// The normwise backward error of each iterate: the first solve's, then each step's.
    pub(crate) backward_errors: Vec<f64>,
    /// GMRES's iterations in each step.
    pub(crate) inner: Vec<usize>,
    /// The backward error it had to reach, p u.
    pub(crate) target: f64,
    /// Whether it reached it.
    pub(crate) converged: bool,
}

impl Refinement {
    /// The history in words, for an error: every backward error and GMRES's iterations.
    pub(crate) fn history(&self) -> String {
        let errors: Vec<String> = self
            .backward_errors
            .iter()
            .map(|e| format!("{e:.1e}"))
            .collect();
        let inner: Vec<String> = self.inner.iter().map(usize::to_string).collect();
        format!(
            "backward errors {} (the target {:.1e}) after {} refinement steps, GMRES taking {} \
             iterations",
            errors.join(", "),
            self.target,
            self.inner.len(),
            if inner.is_empty() {
                "no".into()
            } else {
                inner.join(", ")
            }
        )
    }
}

/// A square system to refine the solution of: its products in double precision, its ∞-norm,
/// the most nonzeros in a row, and the low-precision factors' solve M⁻¹ (in double arithmetic).
pub(crate) struct System<'a> {
    /// v ↦ A v.
    pub(crate) product: &'a dyn Fn(&[c64]) -> Vec<c64>,
    /// v ↦ M⁻¹ v.
    pub(crate) inverse: &'a dyn Fn(&[c64]) -> Vec<c64>,
    /// ‖A‖∞.
    pub(crate) norm: f64,
    /// The most nonzeros in a row of A.
    pub(crate) row_entries: usize,
}

fn norm_inf(v: &[c64]) -> f64 {
    v.iter().map(|z| z.norm()).fold(0.0, f64::max)
}

fn norm2(v: &[c64]) -> f64 {
    v.iter().map(c64::norm_sqr).sum::<f64>().sqrt()
}

/// Solves A x = `b` by GMRES-IR: Ok with the solution and its history when the backward error
/// reached p u, Err with the history when it didn't.
pub(crate) fn refine(
    system: &System<'_>,
    b: &[c64],
) -> std::result::Result<(Vec<c64>, Refinement), Refinement> {
    refine_observed(system, b, &mut |_| {})
}

/// [`refine`], each iterate shown to `observe` as it is made: the first solve's, then each
/// step's.
pub(crate) fn refine_observed(
    system: &System<'_>,
    b: &[c64],
    observe: &mut dyn FnMut(&[c64]),
) -> std::result::Result<(Vec<c64>, Refinement), Refinement> {
    let target = (system.row_entries + 1) as f64 * UNIT_ROUNDOFF;
    let mut history = Refinement {
        backward_errors: Vec::new(),
        inner: Vec::new(),
        target,
        converged: false,
    };
    let b_norm = norm_inf(b);
    if b_norm == 0.0 {
        history.backward_errors.push(0.0);
        history.converged = true;
        return Ok((vec![c64::new(0.0, 0.0); b.len()], history));
    }
    let mut x = (system.inverse)(b);
    loop {
        observe(&x);
        let applied = (system.product)(&x);
        let r: Vec<c64> = b.iter().zip(&applied).map(|(b, a)| b - a).collect();
        let eta = norm_inf(&r) / (system.norm * norm_inf(&x) + b_norm);
        let before = history.backward_errors.last().copied();
        history.backward_errors.push(eta);
        if eta <= target {
            history.converged = true;
            return Ok((x, history));
        }
        let stalled = before.is_some_and(|before| eta > 0.5 * before);
        if !eta.is_finite() || stalled || history.inner.len() == STEPS {
            return Err(history);
        }
        let (d, iterations) = gmres(system, &r);
        history.inner.push(iterations);
        for (x, d) in x.iter_mut().zip(&d) {
            *x += d;
        }
    }
}

/// The plane rotation that zeroes b against a: (c, s) with c real, [c s; −s̄ c] [a; b] = [ρ; 0].
fn rotation(a: c64, b: c64) -> (f64, c64) {
    let size = a.norm().hypot(b.norm());
    if size == 0.0 {
        return (1.0, c64::new(0.0, 0.0));
    }
    if a.norm() == 0.0 {
        return (0.0, b.conj() / b.norm());
    }
    let phase = a / a.norm();
    (a.norm() / size, phase * b.conj() / size)
}

/// GMRES on M⁻¹ A d = M⁻¹ `r` from d = 0: the correction and its iterations.
fn gmres(system: &System<'_>, r: &[c64]) -> (Vec<c64>, usize) {
    let n = r.len();
    let z = (system.inverse)(r);
    let beta = norm2(&z);
    if beta == 0.0 || !beta.is_finite() {
        return (vec![c64::new(0.0, 0.0); n], 0);
    }
    let m = INNER_ITERATIONS;
    let mut basis: Vec<Vec<c64>> = Vec::with_capacity(m + 1);
    basis.push(z.iter().map(|v| v / beta).collect());
    // the Hessenberg matrix by columns, rotated to upper triangular as it grows
    let mut h: Vec<Vec<c64>> = Vec::with_capacity(m);
    let mut rotations: Vec<(f64, c64)> = Vec::with_capacity(m);
    let mut g = vec![c64::new(beta, 0.0)];
    let mut done = 0;
    for j in 0..m {
        let mut w = (system.inverse)(&(system.product)(&basis[j]));
        let mut column = vec![c64::new(0.0, 0.0); j + 2];
        for (i, v) in basis.iter().enumerate() {
            let dot: c64 = v.iter().zip(&w).map(|(v, w)| v.conj() * w).sum();
            column[i] = dot;
            for (w, v) in w.iter_mut().zip(v) {
                *w -= dot * v;
            }
        }
        let next = norm2(&w);
        column[j + 1] = c64::new(next, 0.0);
        for (i, &(c, s)) in rotations.iter().enumerate() {
            let (a, b) = (column[i], column[i + 1]);
            column[i] = c * a + s * b;
            column[i + 1] = -s.conj() * a + c * b;
        }
        let (c, s) = rotation(column[j], column[j + 1]);
        column[j] = c * column[j] + s * column[j + 1];
        column[j + 1] = c64::new(0.0, 0.0);
        rotations.push((c, s));
        let gj = g[j];
        g[j] = c * gj;
        g.push(-s.conj() * gj);
        h.push(column);
        done = j + 1;
        let breakdown = next == 0.0 || !next.is_finite();
        if breakdown || g[j + 1].norm() <= INNER_TOLERANCE * beta {
            break;
        }
        basis.push(w.iter().map(|v| v / next).collect());
    }
    // H y = g, H upper triangular, then d = V y
    let mut y = vec![c64::new(0.0, 0.0); done];
    for i in (0..done).rev() {
        let mut sum = g[i];
        for (k, yk) in y.iter().enumerate().take(done).skip(i + 1) {
            sum -= h[k][i] * yk;
        }
        y[i] = sum / h[i][i];
    }
    let mut d = vec![c64::new(0.0, 0.0); n];
    for (v, yi) in basis.iter().zip(&y) {
        for (d, v) in d.iter_mut().zip(v) {
            *d += yi * v;
        }
    }
    if d.iter().all(|v| v.is_finite()) {
        (d, done)
    } else {
        (vec![c64::new(0.0, 0.0); n], done)
    }
}

#[cfg(test)]
#[path = "../cases/sparse/refine_tests.rs"]
mod tests;
