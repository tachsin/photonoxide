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

    /// The n × n matrix whose row r is `rows[r]`, its columns sorted and distinct. The
    /// transpose is laid out by counting, without sorting.
    pub(crate) fn from_rows(rows: Vec<Vec<(usize, c64)>>) -> Sparse {
        let n = rows.len();
        let mut starts = Vec::with_capacity(n + 1);
        starts.push(0);
        let mut columns = Vec::with_capacity(rows.iter().map(Vec::len).sum());
        let mut values = Vec::with_capacity(columns.capacity());
        let mut t_starts = vec![0usize; n + 1];
        for row in &rows {
            for &(c, v) in row {
                columns.push(c);
                values.push(v);
                t_starts[c + 1] += 1;
            }
            starts.push(columns.len());
        }
        for r in 0..n {
            t_starts[r + 1] += t_starts[r];
        }
        // the transpose's rows filled in order of the original rows, so each is sorted
        let mut next = t_starts.clone();
        let mut t_columns = vec![0; columns.len()];
        let mut t_values = vec![c64::new(0.0, 0.0); columns.len()];
        for r in 0..n {
            for k in starts[r]..starts[r + 1] {
                let at = &mut next[columns[k]];
                t_columns[*at] = r;
                t_values[*at] = values[k];
                *at += 1;
            }
        }
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

    /// Row `r`'s entries, (column, value).
    pub(crate) fn row(&self, r: usize) -> impl Iterator<Item = (usize, c64)> + '_ {
        (self.starts[r]..self.starts[r + 1]).map(|k| (self.columns[k], self.values[k]))
    }

    /// A v.
    pub(crate) fn apply(&self, v: &[c64]) -> Vec<c64> {
        let mut out = vec![c64::new(0.0, 0.0); self.n];
        product(&self.starts, &self.columns, &self.values, v, &mut out);
        out
    }

    /// Aᵀ v (the transpose, not the conjugate transpose).
    pub(crate) fn apply_transpose(&self, v: &[c64]) -> Vec<c64> {
        let mut out = vec![c64::new(0.0, 0.0); self.n];
        product(&self.t_starts, &self.t_columns, &self.t_values, v, &mut out);
        out
    }
}

impl Sparse {
    /// A diagonal S whose similarity B = S A S⁻¹ is complex symmetric, and B, symmetric to the
    /// last bit; or none if there is no such S. S² = D with d_i a_ij = d_j a_ji for every entry
    /// to within 1e-12 of it: D is found by walking A's graph from each unknown not yet reached
    /// (d = 1 there, d_j = d_i a_ij / a_ji along each entry), scaled to a largest |d_i| of 1, and
    /// s_i = √d_i; then b_ij = a_ij s_i/s_j and b_ji = a_ji s_j/s_i agree, and their mean is
    /// kept. A similarity keeps A's eigenvalues, which D A (symmetric too) doesn't: QMR for
    /// symmetric matrices on D A didn't converge on the 3D guide where QMR on A takes 2 000
    /// iterations. A x = b is B (S x) = S b. The 3D curl-curl operator with PMLs is symmetric so,
    /// D its cells' stretch factors (measured: to 7e-15); Shin and Fan's operator, and a Bloch
    /// side's phases, aren't.
    pub(crate) fn symmetrized(&self) -> Option<(Vec<c64>, Sparse)> {
        let n = self.n;
        let entry = |r: usize, c: usize| -> Option<c64> {
            let row = &self.columns[self.starts[r]..self.starts[r + 1]];
            row.binary_search(&c)
                .ok()
                .map(|k| self.values[self.starts[r] + k])
        };
        let mut d: Vec<Option<c64>> = vec![None; n];
        for start in 0..n {
            if d[start].is_some() {
                continue;
            }
            d[start] = Some(c64::new(1.0, 0.0));
            let mut stack = vec![start];
            while let Some(i) = stack.pop() {
                for (j, aij) in self.row(i) {
                    if d[j].is_some() || i == j {
                        continue;
                    }
                    let aji = entry(j, i)?;
                    if aji.norm() == 0.0 || aij.norm() == 0.0 {
                        return None;
                    }
                    d[j] = Some(d[i]? * aij / aji);
                    stack.push(j);
                }
            }
        }
        let d: Vec<c64> = d.into_iter().collect::<Option<_>>()?;
        let largest = d.iter().map(|z| z.norm()).fold(0.0, f64::max);
        let d: Vec<c64> = d.iter().map(|z| z / largest).collect();
        let s: Vec<c64> = d.iter().map(|z| z.sqrt()).collect();
        let mut rows = Vec::with_capacity(n);
        for i in 0..n {
            let mut row = Vec::with_capacity(self.starts[i + 1] - self.starts[i]);
            for (j, aij) in self.row(i) {
                let aji = entry(j, i)?;
                if (d[i] * aij - d[j] * aji).norm()
                    > 1e-12 * (d[i] * aij).norm().max((d[j] * aji).norm())
                {
                    return None;
                }
                let (upper, lower) = (aij * s[i] / s[j], aji * s[j] / s[i]);
                row.push((j, (upper + lower) / 2.0));
            }
            rows.push(row);
        }
        Some((s, Sparse::from_rows(rows)))
    }
}

/// A matrix-vector product by rows into `out`, the rows shared among rayon's threads: each row
/// is summed in the same order whatever the threads, so the result is the same bit for bit.
fn product(starts: &[usize], columns: &[usize], values: &[c64], v: &[c64], out: &mut [c64]) {
    use rayon::prelude::*;
    let row = |r: usize| -> c64 {
        (starts[r]..starts[r + 1])
            .map(|k| values[k] * v[columns[k]])
            .sum()
    };
    out.par_iter_mut()
        .with_min_len(4096)
        .enumerate()
        .for_each(|(r, o)| *o = row(r));
}

/// A linear operator QMR can solve with: its products with vectors and its transpose's.
pub(crate) trait Operator {
    /// The number of rows (and columns).
    fn size(&self) -> usize;
    /// A v.
    fn apply(&self, v: &[c64]) -> Vec<c64>;
    /// Aᵀ v (the transpose, not the conjugate transpose).
    fn apply_transpose(&self, v: &[c64]) -> Vec<c64>;
    /// A v into `out`, which an operator that can writes without allocating.
    fn apply_into(&self, v: &[c64], out: &mut [c64]) {
        out.copy_from_slice(&self.apply(v));
    }
    /// Aᵀ v into `out`.
    fn apply_transpose_into(&self, v: &[c64], out: &mut [c64]) {
        out.copy_from_slice(&self.apply_transpose(v));
    }
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

    fn apply_into(&self, v: &[c64], out: &mut [c64]) {
        product(&self.starts, &self.columns, &self.values, v, out);
    }

    fn apply_transpose_into(&self, v: &[c64], out: &mut [c64]) {
        product(&self.t_starts, &self.t_columns, &self.t_values, v, out);
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

/// The values a sum is taken over at once, in [`dot_conj`] and [`combine`]: fixed, so that the
/// sums come out the same bit for bit on any number of threads.
const CHUNK: usize = 16_384;

/// (u, v) = Σ conj(u_k) v_k, the inner product of Cⁿ: chunk by chunk on rayon's threads, each
/// chunk summed in order and the chunks' sums added in order.
fn dot_conj(u: &[c64], v: &[c64]) -> c64 {
    use rayon::prelude::*;
    let parts: Vec<c64> = u
        .par_chunks(CHUNK)
        .zip(v.par_chunks(CHUNK))
        .map(|(p, q)| p.iter().zip(q).map(|(x, y)| x.conj() * y).sum())
        .collect();
    parts.into_iter().sum()
}

/// w += Σ_j c_j v_j, value by value on rayon's threads, each value's terms in order.
fn combine(w: &mut [c64], coefficients: &[c64], vectors: &[Vec<c64>]) {
    use rayon::prelude::*;
    w.par_iter_mut()
        .with_min_len(CHUNK)
        .enumerate()
        .for_each(|(k, wk)| {
            for (c, v) in coefficients.iter().zip(vectors) {
                *wk += c * v[k];
            }
        });
}

/// Solves A x = b from x₀ = 0 by restarted GMRES on the right-preconditioned system
/// A M⁻¹ u = b, x = M⁻¹ u: Y. Saad, *Iterative Methods for Sparse Linear Systems*, 2nd ed., SIAM
/// (2003), doi:10.1137/1.9780898718003, Algorithm 9.5 (GMRES with right preconditioning),
/// restarted every `restart` steps (Algorithm 6.11), its Arnoldi basis by modified Gram–Schmidt
/// (Algorithm 6.2) and its least-squares problem by Givens rotations as each column comes
/// (Section 6.5.3; complex ones, Section 6.5.9, Eqs. 6.80–6.81), which gives the residual's norm
/// at every step without forming x.
///
/// Each iteration takes one product with A and one M⁻¹, where QMR takes two of each (it needs
/// the transposes too), and keeps one more vector: `restart` + 1 of them in all. The residual
/// it stops on is A x = b's own, as right preconditioning leaves it, and is computed afresh at
/// each restart and at the end.
///
/// # Errors
///
/// [`Error::InvalidValue`] if `b` doesn't match A, if the tolerance isn't positive, if
/// `restart` is 0, or if the tolerance isn't reached within the iterations allowed.
pub(crate) fn gmres_preconditioned(
    a: &impl Operator,
    m: &impl Preconditioner,
    b: &[c64],
    stopping: Stopping,
    restart: usize,
) -> Result<(Vec<c64>, Convergence)> {
    let n = a.size();
    if b.len() != n {
        return Err(Error::invalid(
            "gmres",
            format!("the right-hand side needs {n} values, got {}", b.len()),
        ));
    }
    if stopping.tolerance.is_nan() || stopping.tolerance <= 0.0 || restart == 0 {
        return Err(Error::invalid(
            "gmres",
            "needs a positive tolerance and a restart of at least 1",
        ));
    }
    let zero = c64::new(0.0, 0.0);
    let rho0 = norm(b);
    let mut x = vec![zero; n];
    let mut history = Vec::new();
    if rho0 == 0.0 {
        return Ok((
            x,
            Convergence {
                iterations: 0,
                residual: 0.0,
                history,
            },
        ));
    }
    let target = stopping.tolerance * rho0;
    let mut residual = b.to_vec();
    loop {
        // line 1: r₀ = b − A x₀, β = ‖r₀‖, v₁ = r₀ / β
        let beta = norm(&residual);
        if beta <= target || history.len() >= stopping.max_iterations {
            break;
        }
        let mut v: Vec<Vec<c64>> = vec![residual.iter().map(|z| z / beta).collect()];
        // the Hessenberg matrix's columns as the rotations leave them (upper triangular), the
        // rotations (c_i, s_i), and β e₁ as they turn it
        let mut columns: Vec<Vec<c64>> = Vec::new();
        let mut rotations: Vec<(c64, f64)> = Vec::new();
        let mut g = vec![c64::new(beta, 0.0)];
        while columns.len() < restart && history.len() < stopping.max_iterations {
            let j = columns.len();
            // lines 3–8: w = A M⁻¹ v_j, orthogonalized against the basis
            let mut w = a.apply(&m.solve(&v[j]));
            let mut h = Vec::with_capacity(j + 2);
            for vi in &v {
                let hij = dot_conj(vi, &w);
                combine(&mut w, &[-hij], std::slice::from_ref(vi));
                h.push(hij);
            }
            let next = norm(&w);
            // the earlier rotations on the new column, then the one that zeroes h_{j+1,j}
            for (i, &(c, s)) in rotations.iter().enumerate() {
                let (p, q) = (h[i], h[i + 1]);
                h[i] = c.conj() * p + s * q;
                h[i + 1] = -s * p + c * q;
            }
            let size = (h[j].norm_sqr() + next * next).sqrt();
            if size == 0.0 {
                // A M⁻¹ v_j lies in the basis and adds nothing: the residual can't fall further
                break;
            }
            let (c, s) = (h[j] / size, next / size);
            h[j] = c64::new(size, 0.0);
            g.push(-s * g[j]);
            g[j] = c.conj() * g[j];
            rotations.push((c, s));
            h.truncate(j + 1);
            columns.push(h);
            history.push(g[j + 1].norm() / rho0);
            if g[j + 1].norm() <= target || next == 0.0 {
                break;
            }
            v.push(w.iter().map(|z| z / next).collect());
        }
        // line 11: y from the triangle, x = x₀ + M⁻¹ V y
        let steps = columns.len();
        if steps == 0 {
            break;
        }
        let mut y = vec![zero; steps];
        for i in (0..steps).rev() {
            let mut sum = g[i];
            for (jj, column) in columns.iter().enumerate().skip(i + 1) {
                sum -= column[i] * y[jj];
            }
            y[i] = sum / columns[i][i];
        }
        let mut u = vec![zero; n];
        combine(&mut u, &y, &v[..steps]);
        let dx = m.solve(&u);
        for (xk, d) in x.iter_mut().zip(&dx) {
            *xk += d;
        }
        // line 12: the true residual, to stop on or to restart from
        let ax = a.apply(&x);
        residual = b.iter().zip(&ax).map(|(p, q)| p - q).collect();
    }
    let reached = norm(&residual) / rho0;
    if reached > stopping.tolerance {
        return Err(Error::invalid(
            "gmres",
            format!(
                "no convergence to {:e} in {} iterations (the residual is at {reached:e})",
                stopping.tolerance, stopping.max_iterations
            ),
        ));
    }
    Ok((
        x,
        Convergence {
            iterations: history.len(),
            residual: reached,
            history,
        },
    ))
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

/// QMR's vector work, fused into passes on rayon's threads chunk by chunk ([`CHUNK`] values),
/// each chunk's sums taken in order and the chunks' added in order: the same bits on any number
/// of threads. Holds the chunks' partial sums, so that an iteration allocates nothing.
#[derive(Default)]
struct Sums {
    complex: Vec<c64>,
    pairs: Vec<(f64, f64)>,
    step: Vec<(f64, c64)>,
}

/// The vectors QMR's step writes; `w_next` none for a complex symmetric A, whose w is v.
struct Step<'a> {
    p: &'a mut [c64],
    x: &'a mut [c64],
    v_next: &'a mut [c64],
    w_next: Option<&'a mut [c64]>,
    r: &'a mut [c64],
}

/// The left Lanczos vectors' part of [`Sums::next_lanczos`]: w_{n+1} written from Aᵀ w_n, w_n,
/// w_{n−1} and β_w.
struct Left<'a> {
    w_next: &'a mut [c64],
    atw: &'a [c64],
    w: &'a [c64],
    w_old: &'a [c64],
    beta_w: c64,
}

impl Sums {
    /// Σ a_k b_k, unconjugated (Freund and Nachtigal's bilinear form).
    fn dot(&mut self, a: &[c64], b: &[c64]) -> c64 {
        use rayon::prelude::*;
        a.par_chunks(CHUNK)
            .zip(b.par_chunks(CHUNK))
            .map(|(p, q)| p.iter().zip(q).map(|(x, y)| x * y).sum::<c64>())
            .collect_into_vec(&mut self.complex);
        self.complex.iter().sum()
    }

    /// The next Lanczos vectors before their scaling (Eq. 2.7), v = A v_n − α v_n − β_v v_{n−1}
    /// into `v_next` and, unless A is complex symmetric (`left` none, w = v), w = Aᵀ w_n −
    /// α w_n − β_w w_{n−1}; and their squared norms (v's twice for a symmetric A).
    fn next_lanczos(
        &mut self,
        v_next: &mut [c64],
        [av, v, v_old]: [&[c64]; 3],
        alpha: c64,
        beta_v: c64,
        left: Option<Left<'_>>,
    ) -> (f64, f64) {
        use rayon::prelude::*;
        let right = |k: usize| av[k] - alpha * v[k] - beta_v * v_old[k];
        match left {
            Some(Left {
                w_next,
                atw,
                w,
                w_old,
                beta_w,
            }) => {
                v_next
                    .par_chunks_mut(CHUNK)
                    .zip(w_next.par_chunks_mut(CHUNK))
                    .enumerate()
                    .map(|(chunk, (vs, ws))| {
                        let start = chunk * CHUNK;
                        let mut sums = (0.0, 0.0);
                        for (i, (vk, wk)) in vs.iter_mut().zip(ws.iter_mut()).enumerate() {
                            let k = start + i;
                            *vk = right(k);
                            *wk = atw[k] - alpha * w[k] - beta_w * w_old[k];
                            sums.0 += vk.norm_sqr();
                            sums.1 += wk.norm_sqr();
                        }
                        sums
                    })
                    .collect_into_vec(&mut self.pairs);
            }
            None => {
                v_next
                    .par_chunks_mut(CHUNK)
                    .enumerate()
                    .map(|(chunk, vs)| {
                        let start = chunk * CHUNK;
                        let mut sum = 0.0;
                        for (i, vk) in vs.iter_mut().enumerate() {
                            *vk = right(start + i);
                            sum += vk.norm_sqr();
                        }
                        (sum, sum)
                    })
                    .collect_into_vec(&mut self.pairs);
            }
        }
        (self.pairs.iter()).fold((0.0, 0.0), |s, p| (s.0 + p.0, s.1 + p.1))
    }

    /// The rest of an iteration (Eqs. 4.8–4.9, 2.9, 4.12): p = (v_n − ε p_{n−1} − θ p_{n−2}) / δ
    /// and x += τ_n p; then, unless the process ended (`next` none), v_{n+1} and w_{n+1} scaled
    /// by 1/ρ_{n+1} and 1/ξ_{n+1} and r = keep r + add v_{n+1}. Returns ‖r‖² and w_{n+1}ᵀ v_{n+1}
    /// (v_{n+1}ᵀ v_{n+1} for a complex symmetric A, whose w is v).
    fn step(
        &mut self,
        out: Step<'_>,
        [v, p_old, p_older]: [&[c64]; 3],
        [epsilon, theta, delta, tau_n]: [c64; 4],
        next: Option<([f64; 2], f64, c64)>,
    ) -> (f64, c64) {
        use rayon::prelude::*;
        let Step {
            p,
            x,
            v_next,
            w_next,
            r,
        } = out;
        // the value k's update, but for w; its share of ‖r‖², and v_{n+1}'s value
        let update = |k: usize, pk: &mut c64, xk: &mut c64, vk: &mut c64, rk: &mut c64| {
            *pk = (v[k] - epsilon * p_old[k] - theta * p_older[k]) / delta;
            *xk += tau_n * *pk;
            if let Some(([rho_next, _], keep, add)) = next {
                *vk /= rho_next;
                *rk = keep * *rk + add * *vk;
            }
            rk.norm_sqr()
        };
        let chunks = p
            .par_chunks_mut(CHUNK)
            .zip(x.par_chunks_mut(CHUNK))
            .zip(v_next.par_chunks_mut(CHUNK))
            .zip(r.par_chunks_mut(CHUNK))
            .enumerate();
        match w_next {
            Some(w_next) => chunks
                .zip(w_next.par_chunks_mut(CHUNK))
                .map(|((chunk, (((ps, xs), vs), rs)), ws)| {
                    let start = chunk * CHUNK;
                    let (mut r2, mut d) = (0.0, c64::new(0.0, 0.0));
                    for i in 0..ps.len() {
                        r2 += update(start + i, &mut ps[i], &mut xs[i], &mut vs[i], &mut rs[i]);
                        if let Some(([_, xi_next], _, _)) = next {
                            ws[i] /= xi_next;
                            d += ws[i] * vs[i];
                        }
                    }
                    (r2, d)
                })
                .collect_into_vec(&mut self.step),
            None => chunks
                .map(|(chunk, (((ps, xs), vs), rs))| {
                    let start = chunk * CHUNK;
                    let (mut r2, mut d) = (0.0, c64::new(0.0, 0.0));
                    for i in 0..ps.len() {
                        r2 += update(start + i, &mut ps[i], &mut xs[i], &mut vs[i], &mut rs[i]);
                        if next.is_some() {
                            d += vs[i] * vs[i];
                        }
                    }
                    (r2, d)
                })
                .collect_into_vec(&mut self.step),
        }
        (self.step.iter()).fold((0.0, c64::new(0.0, 0.0)), |s, p| (s.0 + p.0, s.1 + p.1))
    }
}

fn norm(a: &[c64]) -> f64 {
    a.iter().map(|x| x.norm_sqr()).sum::<f64>().sqrt()
}

/// A Givens rotation (c, s) of Freund and Nachtigal's Eq. 4.1, G = [[c, s], [−s̄, c]], applied
/// to (a, b).
fn rotate((c, s): (f64, c64), a: c64, b: c64) -> (c64, c64) {
    (c * a + s * b, -s.conj() * a + c * b)
}

/// How one run of the Lanczos process ended.
enum Run {
    /// At the tolerance: the solution and how it went.
    Done(Vec<c64>, Convergence),
    /// At a (near-)breakdown, |w_nᵀ v_n| below 1e-14 for unit vectors: the iterate so far, the
    /// residual's history, and |w_nᵀ v_n|.
    Broken(Vec<c64>, Vec<f64>, f64),
}

/// The most times [`qmr`] restarts after a breakdown.
const RESTARTS: usize = 10;

/// Solves A x = b by QMR from x₀ = 0.
///
/// The look-ahead steps that would step over a breakdown of the Lanczos process aren't
/// implemented. Instead, at a breakdown or near-breakdown (|w_nᵀ v_n| below 1e-14 for unit
/// vectors), QMR restarts from the iterate it has reached, on its true residual (up to 10 times):
/// the iterate is good, only the Lanczos vectors can't go on. Seen once, on a 1.8 M-unknown
/// silicon strip with a port's mode source, at step 609.
///
/// # Errors
///
/// [`Error::InvalidValue`] if `b` doesn't match A, if the tolerance isn't positive, if the
/// Lanczos process breaks down at its first step, or more than 10 times, or if the tolerance
/// isn't reached within the iterations allowed.
pub(crate) fn qmr(
    a: &impl Operator,
    b: &[c64],
    stopping: Stopping,
) -> Result<(Vec<c64>, Convergence)> {
    restarted(a, b, stopping, false)
}

/// Solves A x = b by QMR from x₀ = 0 for a complex symmetric A (A = Aᵀ, not Hermitian):
/// R. W. Freund, "Conjugate gradient-type methods for linear systems with complex symmetric
/// coefficient matrices", SIAM J. Sci. Stat. Comput. 13, 425 (1992), doi:10.1137/0913023, his
/// Algorithm 3.2 on the complex symmetric Lanczos process (his Algorithm 2.1). It is [`qmr`]
/// with its left Lanczos vectors equal to its right ones: started with w₁ = v₁, they stay equal
/// when A = Aᵀ, so neither they nor Aᵀ's products are taken. One product with A an iteration,
/// where [`qmr`] takes one with A and one with Aᵀ; Freund's weights ω_j = ‖v_j‖ (his Eq. 3.11)
/// are the unit vectors [`qmr`] already keeps. A must be symmetric: nothing checks it.
///
/// # Errors
///
/// As [`qmr`].
pub(crate) fn qmr_symmetric(
    a: &impl Operator,
    b: &[c64],
    stopping: Stopping,
) -> Result<(Vec<c64>, Convergence)> {
    restarted(a, b, stopping, true)
}

/// Solves A x = b for an A whose similarity B = S A S⁻¹ is complex symmetric, given B and the
/// diagonal S ([`Sparse::symmetrized`]): [`qmr_symmetric`] on B y = S b, x = S⁻¹ y, to the
/// relative residual of A x = b itself that `stopping` asks, b − A x = S⁻¹ (S b − B y). S
/// weights B's residual unevenly (by up to 300 times on a 3D guide's PMLs), so where B's
/// residual is at the tolerance and A's isn't yet, QMR carries on from y to a tolerance tightened
/// by as much, up to 5 times. Its convergence history is B's residual, relative to ‖S b‖.
///
/// # Errors
///
/// As [`qmr`], and [`Error::InvalidValue`] if A's residual is still above the tolerance after
/// 5 rounds.
pub(crate) fn qmr_similar(
    b_matrix: &Sparse,
    s: &[c64],
    b: &[c64],
    stopping: Stopping,
) -> Result<(Vec<c64>, Convergence)> {
    let n = b.len();
    if s.len() != n || b_matrix.n != n {
        return Err(Error::invalid(
            "qmr",
            format!("the right-hand side needs {} values, got {n}", b_matrix.n),
        ));
    }
    let rho0 = norm(b);
    let c: Vec<c64> = b.iter().zip(s).map(|(x, y)| x * y).collect();
    let scale = norm(&c);
    let mut y = vec![c64::new(0.0, 0.0); n];
    let mut history = Vec::new();
    let mut tolerance = stopping.tolerance;
    for _ in 0..5 {
        let by = b_matrix.apply(&y);
        let rc: Vec<c64> = c.iter().zip(&by).map(|(p, q)| p - q).collect();
        let ra: Vec<c64> = rc.iter().zip(s).map(|(r, w)| r / w).collect();
        let residual = if rho0 == 0.0 { 0.0 } else { norm(&ra) / rho0 };
        if residual <= stopping.tolerance {
            let x = y.iter().zip(s).map(|(p, w)| p / w).collect();
            return Ok((
                x,
                Convergence {
                    iterations: history.len(),
                    residual,
                    history,
                },
            ));
        }
        if !history.is_empty() {
            // B's residual reached its tolerance before A's: tighten it by the shortfall
            tolerance *= 0.5 * stopping.tolerance / residual;
        }
        let left = norm(&rc);
        let (dy, how) = qmr_symmetric(
            b_matrix,
            &rc,
            Stopping {
                tolerance: tolerance * scale / left,
                max_iterations: stopping.max_iterations.saturating_sub(history.len()),
            },
        )?;
        for (yk, d) in y.iter_mut().zip(&dy) {
            *yk += d;
        }
        history.extend(how.history.iter().map(|h| h * left / scale));
    }
    Err(Error::invalid(
        "qmr",
        format!(
            "QMR on the symmetric similar matrix reached its tolerance 5 times without \
             A's residual reaching {:e}",
            stopping.tolerance
        ),
    ))
}

/// [`qmr`], restarted from its iterate at a breakdown, its left vectors kept unless `symmetric`.
fn restarted(
    a: &impl Operator,
    b: &[c64],
    stopping: Stopping,
    symmetric: bool,
) -> Result<(Vec<c64>, Convergence)> {
    let rho0 = norm(b);
    let mut x: Vec<c64> = Vec::new();
    let mut history = Vec::new();
    let mut residual = b.to_vec();
    for _ in 0..=RESTARTS {
        let scale = norm(&residual);
        let used = history.len();
        let left = Stopping {
            tolerance: if rho0 == 0.0 {
                stopping.tolerance
            } else {
                stopping.tolerance * rho0 / scale
            },
            max_iterations: stopping.max_iterations.saturating_sub(used),
        };
        let run = qmr_run(a, &residual, left, symmetric)?;
        let (dx, steps) = match run {
            Run::Done(dx, how) => {
                if x.is_empty() {
                    return Ok((dx, how));
                }
                (dx, Ok(how))
            }
            Run::Broken(dx, steps, d) => (dx, Err((steps, d))),
        };
        if x.is_empty() {
            x = dx;
        } else {
            for (xk, d) in x.iter_mut().zip(&dx) {
                *xk += d;
            }
        }
        match steps {
            Ok(how) => {
                history.extend(how.history.iter().map(|h| h * scale / rho0));
                return Ok((
                    x,
                    Convergence {
                        iterations: history.len(),
                        residual: how.residual * scale / rho0,
                        history,
                    },
                ));
            }
            Err((steps, d)) => {
                if steps.is_empty() {
                    return Err(Error::invalid(
                        "qmr",
                        format!(
                            "the Lanczos process broke down at its first step (|w^T v| = {d:e}); \
                             the look-ahead steps that step over this aren't implemented"
                        ),
                    ));
                }
                history.extend(steps.iter().map(|h| h * scale / rho0));
                let ax = a.apply(&x);
                residual = b.iter().zip(&ax).map(|(p, q)| p - q).collect();
            }
        }
    }
    Err(Error::invalid(
        "qmr",
        format!(
            "the Lanczos process broke down {} times, at step {}",
            RESTARTS + 1,
            history.len()
        ),
    ))
}

/// One run of QMR from x₀ = 0, to a breakdown or the tolerance (see [`qmr`]).
///
/// # Errors
///
/// [`Error::InvalidValue`] if `b` doesn't match A, if the tolerance isn't positive, or if the
/// tolerance isn't reached within the iterations allowed.
fn qmr_run(a: &impl Operator, b: &[c64], stopping: Stopping, symmetric: bool) -> Result<Run> {
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
        return Ok(Run::Done(
            x,
            Convergence {
                iterations: 0,
                residual: 0.0,
                history: Vec::new(),
            },
        ));
    }
    // Algorithm 3.1, step 0: r0 = b, v1 = w1 = r0 / rho0. For a complex symmetric A the left
    // vectors are the right ones (Freund 1992), and aren't kept
    let mut v: Vec<c64> = b.iter().map(|z| z / rho0).collect();
    let (mut w, mut w_old) = if symmetric {
        (Vec::new(), Vec::new())
    } else {
        (v.clone(), vec![zero; n])
    };
    let mut v_old = vec![zero; n];
    let mut d = dot(if symmetric { &v } else { &w }, &v);
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
    // the work vectors, made once: an iteration allocates nothing
    let left_size = if symmetric { 0 } else { n };
    let (mut av, mut atw) = (vec![zero; n], vec![zero; left_size]);
    let (mut v_next, mut w_next, mut p) = (vec![zero; n], vec![zero; left_size], vec![zero; n]);
    let mut sums = Sums::default();
    for iteration in 1..=stopping.max_iterations {
        if d.norm() < 1e-14 {
            // the iterate so far is good: its caller restarts from it
            return Ok(Run::Broken(x, history, d.norm()));
        }
        // Algorithm 2.1, a regular step (Eq. 2.7) with blocks of one vector: the coefficient of
        // v_{n-1} is w_{n-1}^T A v_n / d_{n-1} = xi_n d_n / d_{n-1}, and of w_{n-1}, rho_n d_n /
        // d_{n-1}, by biorthogonality
        a.apply_into(&v, &mut av);
        if !symmetric {
            a.apply_transpose_into(&w, &mut atw);
        }
        let alpha = sums.dot(if symmetric { &v } else { &w }, &av) / d;
        let beta_v = xi * d / d_old;
        let beta_w = rho * d / d_old;
        // v_{n+1} and w_{n+1} before their scaling, with their norms, in one pass
        let left = (!symmetric).then_some(Left {
            w_next: &mut w_next,
            atw: &atw,
            w: &w,
            w_old: &w_old,
            beta_w,
        });
        let (rho2, xi2) = sums.next_lanczos(&mut v_next, [&av, &v, &v_old], alpha, beta_v, left);
        let (rho_next, xi_next) = (rho2.sqrt(), xi2.sqrt());
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
        // x_n = x_{n-1} + tau_n p_n; Eq. 2.9, the next unit vectors; Eq. 4.12 with omega = 1,
        // r_n = |s_n|^2 r_{n-1} + c_n tau-tilde_{n+1} v_{n+1}: in one pass, with ‖r_n‖² and the
        // next w^T v
        let ended = rho_next == 0.0 || xi_next == 0.0;
        let (r2, d_next) = sums.step(
            Step {
                p: &mut p,
                x: &mut x,
                v_next: &mut v_next,
                w_next: (!symmetric).then_some(&mut w_next),
                r: &mut r,
            },
            [&v, &p_old, &p_older],
            [epsilon, theta, delta, tau_n],
            (!ended).then_some(([rho_next, xi_next], s.norm_sqr(), c * tau)),
        );
        std::mem::swap(&mut p_older, &mut p_old);
        std::mem::swap(&mut p_old, &mut p);
        let updated = r2.sqrt() / rho0;
        history.push(updated);
        if updated <= stopping.tolerance || ended {
            // Eq. 4.10, on the true residual
            let ax = a.apply(&x);
            let true_residual: Vec<c64> = (0..n).map(|k| b[k] - ax[k]).collect();
            let residual = norm(&true_residual) / rho0;
            if residual <= stopping.tolerance {
                return Ok(Run::Done(
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
        std::mem::swap(&mut v_old, &mut v);
        std::mem::swap(&mut v, &mut v_next);
        if !symmetric {
            std::mem::swap(&mut w_old, &mut w);
            std::mem::swap(&mut w, &mut w_next);
        }
        (rho, xi) = (rho_next, xi_next);
        d_old = d;
        d = d_next;
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

/// A triangular factor by rows, solved row by row: from the first row to the last (a lower
/// triangle) or from the last to the first (an upper one). Sequential: solving by wavefronts of
/// independent rows (level scheduling) was measured slower on every thread count, here and at
/// 864 000 unknowns (docs/methods/fdfd-3d.md).
struct Triangle {
    starts: Vec<usize>,
    columns: Vec<usize>,
    values: Vec<c64>,
    /// The diagonal's inverse, or `None` for a unit diagonal.
    inverse_diagonal: Option<Vec<c64>>,
    forward: bool,
}

impl Triangle {
    /// The factor whose row i is `rows(i)` (off the diagonal).
    fn new(
        n: usize,
        rows: impl Fn(usize) -> Vec<(usize, c64)>,
        inverse_diagonal: Option<Vec<c64>>,
        forward: bool,
    ) -> Triangle {
        let mut starts = Vec::with_capacity(n + 1);
        let (mut columns, mut values) = (Vec::new(), Vec::new());
        starts.push(0);
        for i in 0..n {
            for (j, v) in rows(i) {
                columns.push(j);
                values.push(v);
            }
            starts.push(columns.len());
        }
        Triangle {
            starts,
            columns,
            values,
            inverse_diagonal,
            forward,
        }
    }

    /// Solves in place: `x` holds the right-hand side on entry.
    fn solve(&self, x: &mut [c64]) {
        let n = x.len();
        let rows: Box<dyn Iterator<Item = usize>> = if self.forward {
            Box::new(0..n)
        } else {
            Box::new((0..n).rev())
        };
        for i in rows {
            let mut s = x[i];
            for k in self.starts[i]..self.starts[i + 1] {
                s -= self.values[k] * x[self.columns[k]];
            }
            x[i] = match &self.inverse_diagonal {
                Some(d) => s * d[i],
                None => s,
            };
        }
    }
}

/// An incomplete LU factorization with no fill, ILU(0) (as in Y. Saad, Iterative Methods for Sparse
/// Linear Systems, 2nd ed., SIAM (2003), doi:10.1137/1.9780898718003): L and U
/// on A's own sparsity, so that (LU)_ij = a_ij wherever a_ij ≠ 0. The triangular solves are
/// sequential.
pub(crate) struct Ilu0 {
    /// L y = v (unit diagonal), U x = y; and for the transpose, Uᵀ y = v, Lᵀ x = y.
    l: Triangle,
    u: Triangle,
    ut: Triangle,
    lt: Triangle,
}

impl Ilu0 {
    /// The factorization of `a` (its rows' columns sorted, as [`Sparse::new`] stores them).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a row without a diagonal entry or a zero pivot.
    pub(crate) fn new(a: &Sparse) -> Result<Ilu0> {
        let n = a.n;
        let (starts, columns) = (&a.starts, &a.columns);
        let mut values = a.values.clone();
        let mut diagonal = vec![usize::MAX; n];
        let mut at = vec![usize::MAX; n];
        for i in 0..n {
            for k in starts[i]..starts[i + 1] {
                at[columns[k]] = k;
                if columns[k] == i {
                    diagonal[i] = k;
                }
            }
            if diagonal[i] == usize::MAX {
                return Err(Error::invalid("ilu", format!("row {i} has no diagonal")));
            }
            for kk in starts[i]..diagonal[i] {
                let k = columns[kk];
                let pivot = values[diagonal[k]];
                if pivot.norm() == 0.0 {
                    return Err(Error::invalid("ilu", format!("a zero pivot at row {k}")));
                }
                let factor = values[kk] / pivot;
                values[kk] = factor;
                for jj in diagonal[k] + 1..starts[k + 1] {
                    let target = at[columns[jj]];
                    if target != usize::MAX {
                        let update = factor * values[jj];
                        values[target] -= update;
                    }
                }
            }
            for k in starts[i]..starts[i + 1] {
                at[columns[k]] = usize::MAX;
            }
        }
        let inverse: Vec<c64> = (0..n).map(|i| 1.0 / values[diagonal[i]]).collect();
        // the factors' transposes, by rows: the entries of each column
        let mut lower_t: Vec<Vec<(usize, c64)>> = vec![Vec::new(); n];
        let mut upper_t: Vec<Vec<(usize, c64)>> = vec![Vec::new(); n];
        for i in 0..n {
            for k in starts[i]..diagonal[i] {
                lower_t[columns[k]].push((i, values[k]));
            }
            for k in diagonal[i] + 1..starts[i + 1] {
                upper_t[columns[k]].push((i, values[k]));
            }
        }
        let l = Triangle::new(
            n,
            |i| {
                (starts[i]..diagonal[i])
                    .map(|k| (columns[k], values[k]))
                    .collect()
            },
            None,
            true,
        );
        let u = Triangle::new(
            n,
            |i| {
                (diagonal[i] + 1..starts[i + 1])
                    .map(|k| (columns[k], values[k]))
                    .collect()
            },
            Some(inverse.clone()),
            false,
        );
        let ut = Triangle::new(n, |i| upper_t[i].clone(), Some(inverse), true);
        let lt = Triangle::new(n, |i| lower_t[i].clone(), None, false);
        Ok(Ilu0 { l, u, ut, lt })
    }
}

impl Preconditioner for Ilu0 {
    fn solve(&self, v: &[c64]) -> Vec<c64> {
        let mut x = v.to_vec();
        self.l.solve(&mut x);
        self.u.solve(&mut x);
        x
    }

    fn solve_transpose(&self, v: &[c64]) -> Vec<c64> {
        let mut x = v.to_vec();
        self.ut.solve(&mut x);
        self.lt.solve(&mut x);
        x
    }
}

/// ILU(0) of a matrix's diagonal blocks, each block a set of rows and the same columns: a
/// block-Jacobi preconditioner whose blocks are solved by their incomplete factors, all at once
/// on rayon's threads. The blocks are fixed by the caller, not by the threads, so the result is
/// the same bit for bit on any number of them.
pub(crate) struct BlockIlu0 {
    n: usize,
    /// Each block's rows, ascending, and its factors.
    blocks: Vec<(Vec<usize>, Ilu0)>,
}

impl BlockIlu0 {
    /// The factors of `a`'s diagonal blocks `blocks`, which share out its rows.
    ///
    /// # Errors
    ///
    /// As [`Ilu0::new`], for any block.
    pub(crate) fn new(a: &Sparse, blocks: Vec<Vec<usize>>) -> Result<BlockIlu0> {
        use rayon::prelude::*;
        let n = a.n;
        let factors: Vec<Result<Ilu0>> = blocks
            .par_iter()
            .map(|rows| {
                // the block's own numbering: a row's place among the block's rows, which are
                // ascending, so a row of `a` keeps its columns' order and needs no sorting
                let sub = Sparse::from_rows(
                    rows.iter()
                        .map(|&r| {
                            a.row(r)
                                .filter_map(|(c, v)| rows.binary_search(&c).ok().map(|l| (l, v)))
                                .collect::<Vec<_>>()
                        })
                        .collect(),
                );
                Ilu0::new(&sub)
            })
            .collect();
        let mut out = Vec::with_capacity(blocks.len());
        for (rows, ilu) in blocks.into_iter().zip(factors) {
            out.push((rows, ilu?));
        }
        Ok(BlockIlu0 { n, blocks: out })
    }

    fn solve_with(&self, v: &[c64], transpose: bool) -> Vec<c64> {
        use rayon::prelude::*;
        let parts: Vec<Vec<c64>> = self
            .blocks
            .par_iter()
            .map(|(rows, ilu)| {
                let local: Vec<c64> = rows.iter().map(|&r| v[r]).collect();
                if transpose {
                    ilu.solve_transpose(&local)
                } else {
                    ilu.solve(&local)
                }
            })
            .collect();
        let mut out = vec![c64::new(0.0, 0.0); self.n];
        for ((rows, _), x) in self.blocks.iter().zip(parts) {
            for (&r, xr) in rows.iter().zip(x) {
                out[r] = xr;
            }
        }
        out
    }
}

impl Preconditioner for BlockIlu0 {
    fn solve(&self, v: &[c64]) -> Vec<c64> {
        self.solve_with(v, false)
    }

    fn solve_transpose(&self, v: &[c64]) -> Vec<c64> {
        self.solve_with(v, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A complex, nonsymmetric, diagonally dominant matrix of n rows with a few entries off the
    /// diagonal, and a right-hand side: the same on every run.
    fn gmres_case(n: usize) -> (Sparse, Vec<c64>) {
        let mut entries = Vec::new();
        for r in 0..n {
            entries.push((r, r, c64::new(4.0 + 0.1 * (r % 7) as f64, 1.0)));
            entries.push((r, (r + 1) % n, c64::new(-1.0, 0.3)));
            entries.push((r, (r + n - 1) % n, c64::new(-0.7, -0.2)));
            entries.push((r, (r * 7 + 3) % n, c64::new(0.4, 0.25)));
        }
        let b = (0..n)
            .map(|r| c64::new((r % 5) as f64 - 2.0, (r % 3) as f64))
            .collect();
        (Sparse::new(n, entries), b)
    }

    #[test]
    fn gmres_solves_a_system_to_its_tolerance_restarted_or_not() {
        let (a, b) = gmres_case(400);
        let ilu = Ilu0::new(&a).unwrap();
        let stop = Stopping {
            tolerance: 1e-11,
            max_iterations: 500,
        };
        let residual = |x: &[c64]| {
            let ax = a.apply(x);
            let r: Vec<c64> = b.iter().zip(&ax).map(|(p, q)| p - q).collect();
            norm(&r) / norm(&b)
        };
        // in one run of the Arnoldi process, and restarted every 3 steps: the same solution
        let (whole, how) = gmres_preconditioned(&a, &ilu, &b, stop, 200).unwrap();
        assert!(residual(&whole) <= 1e-11 && how.residual <= 1e-11);
        assert!(how.iterations < 40, "{}", how.iterations);
        // the residual it reports at each step is the true one's estimate: it falls, and ends
        // where the true one is
        assert!(how.history.windows(2).all(|w| w[1] <= w[0] * (1.0 + 1e-12)));
        assert!((how.history[how.iterations - 1] - how.residual).abs() < 1e-12);
        let (restarted, more) = gmres_preconditioned(&a, &ilu, &b, stop, 3).unwrap();
        assert!(residual(&restarted) <= 1e-11);
        assert!(more.iterations >= how.iterations);
        let d: Vec<c64> = whole.iter().zip(&restarted).map(|(p, q)| p - q).collect();
        assert!(norm(&d) < 1e-9 * norm(&whole));
        // and QMR's, with the same preconditioner
        let (by_qmr, _) = qmr_preconditioned(&a, &ilu, &b, stop).unwrap();
        let d: Vec<c64> = whole.iter().zip(&by_qmr).map(|(p, q)| p - q).collect();
        assert!(norm(&d) < 1e-9 * norm(&whole));
    }

    #[test]
    fn gmres_refuses_what_it_cant_solve_and_says_when_it_doesnt_converge() {
        let (a, b) = gmres_case(50);
        let ilu = Ilu0::new(&a).unwrap();
        let stop = |tolerance: f64, max_iterations: usize| Stopping {
            tolerance,
            max_iterations,
        };
        assert!(gmres_preconditioned(&a, &ilu, &b[1..], stop(1e-8, 100), 10).is_err());
        assert!(gmres_preconditioned(&a, &ilu, &b, stop(0.0, 100), 10).is_err());
        assert!(gmres_preconditioned(&a, &ilu, &b, stop(1e-8, 100), 0).is_err());
        let e = gmres_preconditioned(&a, &ilu, &b, stop(1e-12, 2), 10).unwrap_err();
        assert!(
            e.to_string()
                .contains("no convergence to 1e-12 in 2 iterations"),
            "{e}"
        );
        // nothing to solve: zero, in no iterations
        let zero = vec![c64::new(0.0, 0.0); 50];
        let (x, how) = gmres_preconditioned(&a, &ilu, &zero, stop(1e-8, 100), 10).unwrap();
        assert!(how.iterations == 0 && x == zero);
    }

    #[test]
    fn gmres_is_the_same_bit_for_bit_on_any_number_of_threads() {
        // more values than one chunk of its sums, so the chunks are in play
        let (a, b) = gmres_case(3 * CHUNK + 17);
        let ilu = Ilu0::new(&a).unwrap();
        let stop = Stopping {
            tolerance: 1e-10,
            max_iterations: 200,
        };
        let run = |threads: usize| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| gmres_preconditioned(&a, &ilu, &b, stop, 20).unwrap().0)
        };
        let one = run(1);
        for threads in [2, 5] {
            let many = run(threads);
            let same = one
                .iter()
                .zip(&many)
                .all(|(p, q)| p.re.to_bits() == q.re.to_bits() && p.im.to_bits() == q.im.to_bits());
            assert!(same, "{threads} threads differ from one");
        }
    }

    /// A complex symmetric matrix (not Hermitian): a shifted Laplacian on a ring with a complex
    /// diagonal and a few long-range symmetric couplings, n × n.
    fn complex_symmetric(n: usize) -> Vec<(usize, usize, c64)> {
        let mut entries = Vec::new();
        for r in 0..n {
            entries.push((r, r, c64::new(3.0 + 0.2 * (r % 5) as f64, 0.4)));
            let c = (r + 1) % n;
            let v = c64::new(-1.0, 0.15 * (r % 3) as f64);
            entries.push((r, c, v));
            entries.push((c, r, v));
            if r % 4 == 0 {
                let c = (r * 13 + 7) % n;
                if c != r {
                    let v = c64::new(0.3, -0.2);
                    entries.push((r, c, v));
                    entries.push((c, r, v));
                }
            }
        }
        entries
    }

    #[test]
    fn symmetric_qmr_is_qmr_without_its_left_vectors() {
        // on an exactly symmetric A, QMR's left vectors are its right ones bit for bit, so
        // dropping them and Aᵀ changes nothing: the same solution and history, to the last bit
        let n = 2 * CHUNK + 5;
        let a = Sparse::new(n, complex_symmetric(n));
        let b: Vec<c64> = (0..n)
            .map(|r| c64::new((r % 5) as f64 - 2.0, (r % 3) as f64))
            .collect();
        let stop = Stopping {
            tolerance: 1e-10,
            max_iterations: 400,
        };
        let bits = |v: &[c64]| -> Vec<(u64, u64)> {
            v.iter().map(|z| (z.re.to_bits(), z.im.to_bits())).collect()
        };
        let (x, how) = qmr(&a, &b, stop).unwrap();
        let (y, how_y) = qmr_symmetric(&a, &b, stop).unwrap();
        assert!(how.iterations > 10);
        assert_eq!(bits(&x), bits(&y));
        assert_eq!(how.history, how_y.history);
    }

    #[test]
    fn a_scaled_symmetric_matrix_is_found_symmetric_and_solved() {
        // A = D⁻¹ S₀, S₀ complex symmetric and D a complex diagonal: S² = D is found again (up
        // to a factor), and QMR for symmetric matrices on B = S A S⁻¹, B (S x) = S b, solves
        // A x = b
        let n = 300;
        let s0 = complex_symmetric(n);
        let scale = |r: usize| c64::new(1.0 + (r % 7) as f64, 0.5 * (r % 3) as f64);
        let a = Sparse::new(n, s0.iter().map(|&(r, c, v)| (r, c, v / scale(r))));
        let (s, b_matrix) = a.symmetrized().expect("A is similar to a symmetric matrix");
        let ratio = s[0] * s[0] / scale(0);
        for (r, sr) in s.iter().enumerate() {
            let off = (sr * sr / scale(r) - ratio).norm() / ratio.norm();
            assert!(off < 1e-12, "{r}: {off}");
        }
        let b: Vec<c64> = (0..n).map(|r| c64::new(1.0, (r % 4) as f64)).collect();
        let sb: Vec<c64> = b.iter().zip(&s).map(|(x, y)| x * y).collect();
        let stop = Stopping {
            tolerance: 1e-12,
            max_iterations: 1000,
        };
        let (y, _) = qmr_symmetric(&b_matrix, &sb, stop).unwrap();
        let x: Vec<c64> = y.iter().zip(&s).map(|(p, q)| p / q).collect();
        let ax = a.apply(&x);
        let r: Vec<c64> = b.iter().zip(&ax).map(|(p, q)| p - q).collect();
        assert!(norm(&r) < 1e-10 * norm(&b), "{}", norm(&r) / norm(&b));
        // a cycle whose ratios don't close (a Bloch side's phase does this) has no such D
        let mut odd = complex_symmetric(n);
        odd.push((0, 1, c64::new(0.0, 0.5)));
        assert!(Sparse::new(n, odd).symmetrized().is_none());
    }

    #[test]
    fn qmr_is_the_same_bit_for_bit_on_any_number_of_threads() {
        // plain and preconditioned, on more values than one chunk of its sums, with its history
        // as well as its solution
        let (a, b) = gmres_case(3 * CHUNK + 17);
        let ilu = Ilu0::new(&a).unwrap();
        let stop = Stopping {
            tolerance: 1e-10,
            max_iterations: 400,
        };
        let bits = |v: &[c64]| -> Vec<(u64, u64)> {
            v.iter().map(|z| (z.re.to_bits(), z.im.to_bits())).collect()
        };
        let run = |threads: usize| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| {
                    let (x, how) = qmr(&a, &b, stop).unwrap();
                    let (y, how_y) = qmr_preconditioned(&a, &ilu, &b, stop).unwrap();
                    (bits(&x), how.history, bits(&y), how_y.history)
                })
        };
        let one = run(1);
        assert!(!one.1.is_empty() && !one.3.is_empty());
        for threads in [2, 5] {
            assert!(run(threads) == one, "{threads} threads differ from one");
        }
    }

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
    fn ilu0_of_a_tridiagonal_matrix_is_its_lu() {
        // no fill: ILU(0) is the exact factorization, and its inverse the matrix's
        let a = tridiagonal(300);
        let ilu = Ilu0::new(&a).unwrap();
        let b: Vec<c64> = (0..300)
            .map(|k| c64::new((k as f64 * 0.3).cos(), (k as f64 * 0.7).sin()))
            .collect();
        let exact = dense_solve(&a, &b);
        let x = ilu.solve(&b);
        let worst = x
            .iter()
            .zip(&exact)
            .map(|(p, q)| (p - q).norm())
            .fold(0.0, f64::max);
        assert!(worst < 1e-13, "{worst}");
        // and its transpose: Aᵀ x = b
        let xt = ilu.solve_transpose(&b);
        let back = a.apply_transpose(&xt);
        let worst = back
            .iter()
            .zip(&b)
            .map(|(p, q)| (p - q).norm())
            .fold(0.0, f64::max);
        assert!(worst < 1e-13, "{worst}");
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
