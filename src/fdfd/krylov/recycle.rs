//! Recycling Krylov subspaces from one solve to the next: GCRO-DR, and the minimum residual over
//! earlier solutions as a starting guess.
//!
//! M. L. Parks, E. de Sturler, G. Mackey, D. D. Johnson, S. Maiti, "Recycling Krylov subspaces
//! for sequences of linear systems", SIAM J. Sci. Comput. 28, 1651 (2006),
//! doi:10.1137/040607277: GCRO with deflated restarting, their Section 2.4 and Appendix, on the
//! right-preconditioned operator 𝒜 = A M⁻¹.
//!
//! - **The recycled space** (their Eqs. 2.5–2.6): U and C with 𝒜 U = C and Cᴴ C = I, k columns
//!   each. For a new operator (the next wavelength) the U of the last system is carried over and
//!   C rebuilt by the reduced QR factorization of 𝒜 U (Appendix lines 3–5, Eq. 2.7): k products
//!   with the new 𝒜. For the same operator (an adjoint, the next port) C is still 𝒜 U, and
//!   nothing is spent.
//! - **A cycle** (lines 6–7 and 22–29): the residual's part in range(C) is solved by U at once,
//!   x += U Cᴴ r, r −= C Cᴴ r; then m − k Arnoldi steps on (I − C Cᴴ) 𝒜 from that residual, which
//!   give 𝒜 [U V] = [C V⁺] G with G = [[D, B], [0, H]] (Eqs. 2.9–2.11; D scales U's columns to
//!   unit length, B = Cᴴ 𝒜 V). The residual is minimized over range([U V]) (Eqs. 2.12–2.15):
//!   with r ⊥ C its top block is solved exactly, so its norm is GMRES's on H, which Givens
//!   rotations give at every step (Saad 2003, Section 6.5.3), and the cycle stops at the
//!   tolerance.
//! - **What is kept** (lines 30–34, Eq. 2.16): the k harmonic Ritz vectors of 𝒜 over
//!   range([U V]) whose harmonic Ritz values θ are smallest in magnitude, Gᴴ G z = θ Gᴴ Wᴴ V̂ z
//!   with W = [C V⁺] and V̂ = [U D, V]; Y = V̂ P, [Q, R] = qr(G P), C = W Q, U = Y R⁻¹. The first
//!   cycle of the first system has no U: it is GMRES(m), and the same formula with k = 0 gives
//!   GMRES-DR's harmonic Ritz vectors (line 14). After the last cycle U is kept for the next
//!   system (line 36).
//!
//! The earlier solutions as a starting guess ([`least_residual`]) are P. F. Fischer's
//! projection (Comput. Methods Appl. Mech. Engrg. 163, 193 (1998),
//! doi:10.1016/S0045-7825(98)00012-7, Method 1), which is Parks et al.'s first step with the
//! earlier solutions as the space; it serves any solver, QMR's included.
//!
//! Every vector operation is a pass over fixed chunks, each chunk's sums in order and the chunks'
//! in order: the same bits on any number of threads. The small eigenproblem and the small QR
//! factorizations are dense and sequential.

use faer::Mat;
use num_complex::Complex64 as c64;

use super::{CHUNK, Convergence, Operator, Preconditioner, Stopping, combine, dot_conj, norm};
use crate::{Error, Result};

/// The recycled space U, C with 𝒜 U = C and Cᴴ C = I, in the right-preconditioned variable
/// (A M⁻¹ u = b, x = M⁻¹ u), and the operator C belongs to.
#[derive(Clone, Debug, Default)]
pub(crate) struct Space {
    pub(crate) u: Vec<Vec<c64>>,
    pub(crate) c: Vec<Vec<c64>>,
    /// Which operator C = 𝒜 U was taken with ([`gcro_dr`]'s `operator`), if any.
    pub(crate) operator: Option<u64>,
    /// The products with 𝒜 spent rebuilding C for a new operator, summed over the solves.
    pub(crate) products: usize,
}

impl Space {
    /// The recycled space's dimension k.
    pub(crate) fn dimension(&self) -> usize {
        self.u.len()
    }
}

/// The identity as a preconditioner: GCRO-DR on A itself.
pub(crate) struct Identity;

impl Preconditioner for Identity {
    fn solve(&self, v: &[c64]) -> Vec<c64> {
        v.to_vec()
    }

    fn solve_transpose(&self, v: &[c64]) -> Vec<c64> {
        v.to_vec()
    }
}

/// The values [`combine_many`] takes at once: small, so that a chunk of every vector it reads
/// stays in cache while each output is summed.
const PASS: usize = 1024;

/// outs[o] += Σ_j coefficients[o][j] vectors[j] for every o in one pass over the vectors, chunk
/// by chunk on rayon's threads, each value's terms in order of j.
fn combine_many(outs: &mut [Vec<c64>], coefficients: &[Vec<c64>], vectors: &[&[c64]]) {
    use rayon::prelude::*;
    if outs.is_empty() {
        return;
    }
    let n = outs[0].len();
    crate::traffic::add(crate::traffic::vectors(
        n,
        vectors.len() + outs.len(),
        outs.len(),
    ));
    super::by_chunks(outs, PASS)
        .into_par_iter()
        .enumerate()
        .for_each(|(chunk, mut parts)| {
            let first = chunk * PASS;
            for (part, cs) in parts.iter_mut().zip(coefficients) {
                for (i, value) in part.iter_mut().enumerate() {
                    let at = first + i;
                    for (c, v) in cs.iter().zip(vectors) {
                        *value += c * v[at];
                    }
                }
            }
        });
}

/// (x_i, y_j) = Σ conj(x_i) y_j for every pair, chunk by chunk as [`dot_conj`] takes them, each
/// chunk's sums in order and the chunks' in order: each the same bits as [`dot_conj`]'s.
/// Indexed [i][j].
fn dots(xs: &[&[c64]], ys: &[&[c64]]) -> Vec<Vec<c64>> {
    use rayon::prelude::*;
    let zero = c64::new(0.0, 0.0);
    let Some(n) = xs.first().map(|x| x.len()) else {
        return Vec::new();
    };
    crate::traffic::add(crate::traffic::vectors(n, xs.len() + ys.len(), 0));
    let chunks = n.div_ceil(CHUNK);
    let parts: Vec<Vec<c64>> = (0..chunks)
        .into_par_iter()
        .map(|chunk| {
            let range = chunk * CHUNK..((chunk + 1) * CHUNK).min(n);
            let mut out = Vec::with_capacity(xs.len() * ys.len());
            for x in xs {
                for y in ys {
                    out.push(
                        x[range.clone()]
                            .iter()
                            .zip(&y[range.clone()])
                            .map(|(p, q)| p.conj() * q)
                            .sum(),
                    );
                }
            }
            out
        })
        .collect();
    let mut sums = vec![zero; xs.len() * ys.len()];
    for part in parts {
        for (s, p) in sums.iter_mut().zip(part) {
            *s += p;
        }
    }
    sums.chunks(ys.len().max(1)).map(<[c64]>::to_vec).collect()
}

/// The reduced QR factorization of the columns `a`, by modified Gram–Schmidt twice (each
/// column orthogonalized against the earlier ones a second time, which keeps Q orthonormal to
/// rounding), a column whose orthogonal part is at most 1e-12 of its norm dropped as dependent.
/// Q's columns, R's columns (each as long as the columns kept before it, and its diagonal), and
/// the indices of the columns kept.
fn qr(a: Vec<Vec<c64>>) -> (Vec<Vec<c64>>, Vec<Vec<c64>>, Vec<usize>) {
    let mut q: Vec<Vec<c64>> = Vec::new();
    let mut r: Vec<Vec<c64>> = Vec::new();
    let mut kept = Vec::new();
    for (index, mut column) in a.into_iter().enumerate() {
        let before = norm(&column);
        let mut coefficients = vec![c64::new(0.0, 0.0); q.len() + 1];
        for _ in 0..2 {
            for (i, qi) in q.iter().enumerate() {
                let h = dot_conj(qi, &column);
                combine(&mut column, &[-h], std::slice::from_ref(qi));
                coefficients[i] += h;
            }
        }
        let after = norm(&column);
        if before == 0.0 || after <= 1e-12 * before {
            continue;
        }
        coefficients[q.len()] = c64::new(after, 0.0);
        q.push(column.iter().map(|z| z / after).collect());
        r.push(coefficients);
        kept.push(index);
    }
    (q, r, kept)
}

/// x₀ = Σ αᵢ xᵢ minimizing ‖b − A x₀‖ over the span of `xs`, given their products `axs` = A xᵢ:
/// P. F. Fischer, "Projection techniques for iterative solution of Ax = b with successive
/// right-hand sides", Comput. Methods Appl. Mech. Engrg. 163, 193 (1998),
/// doi:10.1016/S0045-7825(98)00012-7, his Method 1 (b projected onto the orthonormalized
/// A xᵢ, Eq. 4), which is Parks et al.'s line 6, x₁ = x₀ + U Cᴴ r₀, with U the earlier
/// solutions. An A xᵢ dependent on the earlier ones (to 1e-12) is dropped with its xᵢ.
pub(crate) fn least_residual(b: &[c64], xs: &[&[c64]], axs: &[&[c64]]) -> Vec<c64> {
    let n = b.len();
    let zero = c64::new(0.0, 0.0);
    let (q, r, kept) = qr(axs.iter().map(|v| v.to_vec()).collect());
    if q.is_empty() {
        return vec![zero; n];
    }
    // R α = Qᴴ b, by back substitution
    let qb: Vec<c64> = q.iter().map(|qi| dot_conj(qi, b)).collect();
    let k = q.len();
    let mut alpha = vec![zero; k];
    for i in (0..k).rev() {
        let mut sum = qb[i];
        for (j, column) in r.iter().enumerate().skip(i + 1) {
            sum -= column[i] * alpha[j];
        }
        alpha[i] = sum / r[i][i];
    }
    let chosen: Vec<&[c64]> = kept.iter().map(|&i| xs[i]).collect();
    let mut x = vec![zero; n];
    combine_many(std::slice::from_mut(&mut x), &[alpha], &chosen);
    x
}

/// A Givens rotation (c, s), c complex and s real, that zeroes b in (a, b) as the GMRES of
/// [`super::gmres_preconditioned`] takes it: (c̄ a + s b, −s a + c b).
fn turn((c, s): (c64, f64), a: c64, b: c64) -> (c64, c64) {
    (c.conj() * a + s * b, -s * a + c * b)
}

/// Solves A x = b from x₀ = 0 by GCRO-DR(m, k) on A M⁻¹ from the right (see the module's docs),
/// m = `cycle` the largest subspace a cycle minimizes over and k = `keep` the dimension of the
/// space recycled; `space` is the recycled space, carried in and out. `operator` names A M⁻¹: a
/// space taken with another is rebuilt for this one (k products), one taken with this one is
/// used as it is. The residual it stops on is A x = b's own, computed afresh after each cycle;
/// its iterations are the Arnoldi steps, the products spent rebuilding C are added to
/// `space.products`.
///
/// # Errors
///
/// [`Error::InvalidValue`] if `b` doesn't match A, if the tolerance isn't positive, if
/// `cycle` isn't larger than `keep`, if the small eigenproblem fails, or if the tolerance isn't
/// reached within the iterations allowed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn gcro_dr(
    a: &impl Operator,
    m: &impl Preconditioner,
    b: &[c64],
    stopping: Stopping,
    cycle: usize,
    keep: usize,
    space: &mut Space,
    operator: u64,
) -> Result<(Vec<c64>, Convergence)> {
    let n = a.size();
    if b.len() != n {
        return Err(Error::invalid(
            "gcro-dr",
            format!("the right-hand side needs {n} values, got {}", b.len()),
        ));
    }
    if stopping.tolerance.is_nan() || stopping.tolerance <= 0.0 || cycle <= keep {
        return Err(Error::invalid(
            "gcro-dr",
            "needs a positive tolerance and a cycle longer than the space it keeps",
        ));
    }
    if space.u.iter().any(|u| u.len() != n) {
        space.u.clear();
        space.c.clear();
        space.operator = None;
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
    // lines 3–5: C = Q, U = Y R⁻¹ from 𝒜 Y = Q R, for a space taken with another operator
    if !space.u.is_empty() && space.operator != Some(operator) {
        rebuild(a, m, space);
    }
    space.operator = Some(operator);
    if keep == 0 {
        space.u.clear();
        space.c.clear();
    }
    let target = stopping.tolerance * rho0;
    let mut residual = b.to_vec();
    let mut stalled = 0;
    loop {
        if norm(&residual) <= target || history.len() >= stopping.max_iterations {
            break;
        }
        let k = space.c.len();
        // lines 6–7: the residual's part in range(C), solved by U
        let crefs: Vec<&[c64]> = space.c.iter().map(Vec::as_slice).collect();
        let d: Vec<c64> = dots(&crefs, &[&residual])
            .into_iter()
            .map(|row| row[0])
            .collect();
        let mut r = residual.clone();
        let minus: Vec<c64> = d.iter().map(|z| -z).collect();
        combine_many(std::slice::from_mut(&mut r), &[minus], &crefs);
        let beta = norm(&r);
        // line 22: Arnoldi on (I − C Cᴴ) 𝒜, from r
        let mut v: Vec<Vec<c64>> = Vec::new();
        // H's columns as they came (for G) and as the rotations leave them, B's columns
        let mut raw: Vec<Vec<c64>> = Vec::new();
        let mut columns: Vec<Vec<c64>> = Vec::new();
        let mut bs: Vec<Vec<c64>> = Vec::new();
        let mut rotations: Vec<(c64, f64)> = Vec::new();
        let mut g = vec![c64::new(beta, 0.0)];
        if beta > target {
            v.push(r.iter().map(|z| z / beta).collect());
            let steps = cycle - k;
            while columns.len() < steps && history.len() < stopping.max_iterations {
                let j = columns.len();
                let mut w = a.apply(&m.solve(&v[j]));
                // against C first (B's column), then against V (H's), by modified Gram–Schmidt
                let mut bj = Vec::with_capacity(k);
                for c in &space.c {
                    let h = dot_conj(c, &w);
                    combine(&mut w, &[-h], std::slice::from_ref(c));
                    bj.push(h);
                }
                let mut h = Vec::with_capacity(j + 2);
                for vi in &v {
                    let hij = dot_conj(vi, &w);
                    combine(&mut w, &[-hij], std::slice::from_ref(vi));
                    h.push(hij);
                }
                let next = norm(&w);
                let mut column = h.clone();
                column.push(c64::new(next, 0.0));
                raw.push(column);
                bs.push(bj);
                for (i, &rot) in rotations.iter().enumerate() {
                    (h[i], h[i + 1]) = turn(rot, h[i], h[i + 1]);
                }
                let size = (h[j].norm_sqr() + next * next).sqrt();
                if size == 0.0 {
                    raw.pop();
                    bs.pop();
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
                if next > 0.0 {
                    v.push(w.iter().map(|z| z / next).collect());
                }
                if g[j + 1].norm() <= target || next == 0.0 {
                    break;
                }
            }
        }
        let steps = columns.len();
        // lines 27–28: y for V from the triangle, U's part solved exactly (r ⊥ C):
        // the correction U (d − B y) + V y
        let mut y = vec![zero; steps];
        for i in (0..steps).rev() {
            let mut sum = g[i];
            for (jj, column) in columns.iter().enumerate().skip(i + 1) {
                sum -= column[i] * y[jj];
            }
            y[i] = sum / columns[i][i];
        }
        let mut weights: Vec<c64> = d.clone();
        for (i, w) in weights.iter_mut().enumerate() {
            for (bj, yj) in bs.iter().zip(&y) {
                *w -= bj[i] * yj;
            }
        }
        if steps == 0 && d.iter().all(|z| z.norm() == 0.0) {
            stalled += 1;
            if stalled > 1 {
                break;
            }
        }
        let mut correction = vec![zero; n];
        let mut coefficients = weights.clone();
        coefficients.extend_from_slice(&y);
        let mut sources: Vec<&[c64]> = space.u.iter().map(Vec::as_slice).collect();
        sources.extend(v.iter().take(steps).map(Vec::as_slice));
        combine_many(
            std::slice::from_mut(&mut correction),
            &[coefficients],
            &sources,
        );
        let dx = m.solve(&correction);
        for (xk, dk) in x.iter_mut().zip(&dx) {
            *xk += dk;
        }
        let ax = a.apply(&x);
        residual = b.iter().zip(&ax).map(|(p, q)| p - q).collect();
        // lines 30–34: the space kept, from this cycle's
        if keep > 0 && steps > 0 {
            deflate(space, &v, &raw, &bs, keep)?;
        }
    }
    let reached = norm(&residual) / rho0;
    if reached > stopping.tolerance {
        return Err(Error::invalid(
            "gcro-dr",
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

/// C and U for a new operator from the space's U (Parks et al.'s lines 3–5): [Q, R] = qr(𝒜 U),
/// C = Q, U = U R⁻¹; a column of 𝒜 U dependent on the earlier ones is dropped with its U.
fn rebuild(a: &impl Operator, m: &impl Preconditioner, space: &mut Space) {
    let refs: Vec<&[c64]> = space.u.iter().map(Vec::as_slice).collect();
    let solved = m.solve_block(&refs);
    let solved_refs: Vec<&[c64]> = solved.iter().map(Vec::as_slice).collect();
    let products = a.apply_block(&solved_refs);
    space.products += products.len();
    let (q, r, kept) = qr(products);
    // U R⁻¹ column by column: u_i = (y_i − Σ_{j<i} r_ji u_j) / r_ii
    let mut u: Vec<Vec<c64>> = Vec::with_capacity(kept.len());
    for (i, &index) in kept.iter().enumerate() {
        let mut column = space.u[index].clone();
        let coefficients: Vec<c64> = (0..i).map(|j| -r[i][j]).collect();
        combine(&mut column, &coefficients, &u);
        let pivot = r[i][i];
        u.push(column.iter().map(|z| z / pivot).collect());
    }
    space.u = u;
    space.c = q;
}

/// The space kept from a cycle (Parks et al.'s lines 23–26 and 30–34): G = [[D, B], [0, H]] from
/// the cycle's C, U, Arnoldi vectors `v` (one more than H's columns, or as many after a lucky
/// breakdown), H's columns `raw` as they came and B's columns `bs`; the harmonic Ritz vectors
/// of the `keep` smallest harmonic Ritz values; Y = V̂ P, C = W Q, U = Y R⁻¹ from qr(G P).
fn deflate(
    space: &mut Space,
    v: &[Vec<c64>],
    raw: &[Vec<c64>],
    bs: &[Vec<c64>],
    keep: usize,
) -> Result<()> {
    let zero = c64::new(0.0, 0.0);
    let k = space.u.len();
    let j = raw.len();
    let size = k + j;
    // W = [C V⁺]: V⁺ the Arnoldi vectors, one more than the steps unless the last broke down
    let plus = v.len().min(j + 1);
    let rows = k + plus;
    // D: U's columns scaled to unit length
    let scales: Vec<f64> = space.u.iter().map(|u| 1.0 / norm(u)).collect();
    // G, (rows × size)
    let mut gm = Mat::<c64>::zeros(rows, size);
    for (i, s) in scales.iter().enumerate() {
        gm[(i, i)] = c64::new(*s, 0.0);
    }
    for (col, (h, bj)) in raw.iter().zip(bs).enumerate() {
        for (i, value) in bj.iter().enumerate() {
            gm[(i, k + col)] = *value;
        }
        for (i, value) in h.iter().enumerate().take(plus) {
            gm[(k + i, k + col)] = *value;
        }
    }
    // Wᴴ V̂, V̂ = [U D, V]: [[Cᴴ U D, 0], [V⁺ᴴ U D, I]]
    let mut wv = Mat::<c64>::zeros(rows, size);
    if k > 0 {
        let urefs: Vec<&[c64]> = space.u.iter().map(Vec::as_slice).collect();
        let mut wrefs: Vec<&[c64]> = space.c.iter().map(Vec::as_slice).collect();
        wrefs.extend(v.iter().take(plus).map(Vec::as_slice));
        let products = dots(&wrefs, &urefs);
        for (i, row) in products.iter().enumerate() {
            for (l, value) in row.iter().enumerate() {
                wv[(i, l)] = value * scales[l];
            }
        }
    }
    for col in 0..j {
        wv[(k + col, k + col)] = c64::new(1.0, 0.0);
    }
    // Gᴴ G z = θ Gᴴ Wᴴ V̂ z
    let gh = gm.adjoint();
    let left = gh * &gm;
    let right = gh * &wv;
    let eig = left
        .generalized_eigen(&right)
        .map_err(|e| Error::invalid("gcro-dr", format!("the harmonic Ritz values: {e:?}")))?;
    let (alpha, beta) = (eig.S_a(), eig.S_b());
    let magnitude = |i: usize| {
        if beta[i] == zero {
            f64::INFINITY
        } else {
            (alpha[i] / beta[i]).norm()
        }
    };
    let mut order: Vec<usize> = (0..size)
        .filter(|&i| !alpha[i].is_nan() && !beta[i].is_nan())
        .collect();
    order.sort_by(|&p, &q| magnitude(p).total_cmp(&magnitude(q)).then(p.cmp(&q)));
    order.truncate(keep.min(size));
    let vectors = eig.U();
    // G P, and its QR factorization
    let gp: Vec<Vec<c64>> = order
        .iter()
        .map(|&e| {
            (0..rows)
                .map(|i| (0..size).map(|l| gm[(i, l)] * vectors[(l, e)]).sum())
                .collect()
        })
        .collect();
    let (q, r, kept) = small_qr(gp);
    if kept.is_empty() {
        space.u.clear();
        space.c.clear();
        return Ok(());
    }
    // T = P R⁻¹ (size × kept), column by column: t_i = (p_i − Σ_{j<i} r_ji t_j) / r_ii
    let mut t: Vec<Vec<c64>> = Vec::with_capacity(kept.len());
    for (i, &index) in kept.iter().enumerate() {
        let e = order[index];
        let mut column: Vec<c64> = (0..size).map(|l| vectors[(l, e)]).collect();
        for (jj, tj) in t.iter().enumerate() {
            for (c, value) in column.iter_mut().zip(tj) {
                *c -= r[i][jj] * value;
            }
        }
        let pivot = r[i][i];
        t.push(column.iter().map(|z| z / pivot).collect());
    }
    // U = V̂ T, with V̂'s first k columns U D; C = W Q
    let u_coefficients: Vec<Vec<c64>> = t
        .iter()
        .map(|column| {
            column
                .iter()
                .enumerate()
                .map(|(l, z)| if l < k { z * scales[l] } else { *z })
                .collect()
        })
        .collect();
    let n = v[0].len();
    let mut sources: Vec<&[c64]> = space.u.iter().map(Vec::as_slice).collect();
    sources.extend(v.iter().take(j).map(Vec::as_slice));
    let mut u = vec![vec![zero; n]; kept.len()];
    combine_many(&mut u, &u_coefficients, &sources);
    let mut wsources: Vec<&[c64]> = space.c.iter().map(Vec::as_slice).collect();
    wsources.extend(v.iter().take(plus).map(Vec::as_slice));
    let mut c = vec![vec![zero; n]; kept.len()];
    combine_many(&mut c, &q, &wsources);
    space.u = u;
    space.c = c;
    Ok(())
}

/// [`qr`] for short columns, sequentially: the small matrix G P.
fn small_qr(a: Vec<Vec<c64>>) -> (Vec<Vec<c64>>, Vec<Vec<c64>>, Vec<usize>) {
    let dot = |p: &[c64], q: &[c64]| -> c64 { p.iter().zip(q).map(|(x, y)| x.conj() * y).sum() };
    let length = |p: &[c64]| p.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
    let mut q: Vec<Vec<c64>> = Vec::new();
    let mut r: Vec<Vec<c64>> = Vec::new();
    let mut kept = Vec::new();
    for (index, mut column) in a.into_iter().enumerate() {
        let before = length(&column);
        let mut coefficients = vec![c64::new(0.0, 0.0); q.len() + 1];
        for _ in 0..2 {
            for (i, qi) in q.iter().enumerate() {
                let h = dot(qi, &column);
                for (c, value) in column.iter_mut().zip(qi) {
                    *c -= h * value;
                }
                coefficients[i] += h;
            }
        }
        let after = length(&column);
        if before == 0.0 || after <= 1e-12 * before {
            continue;
        }
        coefficients[q.len()] = c64::new(after, 0.0);
        q.push(column.iter().map(|z| z / after).collect());
        r.push(coefficients);
        kept.push(index);
    }
    (q, r, kept)
}

#[cfg(test)]
#[path = "recycle_tests.rs"]
mod tests;

#[path = "recycle_example.rs"]
pub(crate) mod example;
