//! Block QMR: the quasi-minimal residual method for several right-hand sides of one system at
//! once, A X = B.
//!
//! R. W. Freund, M. Malhotra, "A block QMR algorithm for non-Hermitian linear systems with
//! multiple right-hand sides", Linear Algebra Appl. 254, 119 (1997),
//! doi:10.1016/S0024-3795(96)00529-0:
//!
//! - **Lanczos vectors for many starting vectors** (their Algorithm 3.1, without look-ahead):
//!   right vectors v₁, v₂, … spanning the block Krylov space of A and B, left ones w₁, w₂, …
//!   of Aᵀ and L, biorthogonal, wⱼᵀvₖ = 0 for j ≠ k and δₖ = wₖᵀvₖ ≠ 0 (Eq. 3.2). They are built
//!   **a vector at a time**: each candidate (a column of B first, then A vₙ₋ₘ) is
//!   biorthogonalized against the earlier vectors its history index says it may meet (Eqs.
//!   3.3–3.6), then normalized (Eq. 3.7). L = B, as single QMR takes w₁ = v₁.
//! - **Deflation** (their Section 3, steps 1d and 2d): a candidate whose biorthogonalized part is
//!   at most `DEFLATION` of what it was is dropped, so the block shrinks; one that leaves
//!   something behind is kept in the sets 𝓘 against which later candidates of the other
//!   sequence are biorthogonalized. A column of B dropped so is a system whose right-hand side
//!   is a combination of the others': it leaves the block and is recovered from their
//!   solutions (Eq. 4.18). A product dropped so leaves fewer quasi-residuals than systems: the
//!   combination of systems with none is solved, and the system that weighs most in it leaves
//!   the block, to be recovered at the end (Eqs. 4.16–4.20). A system whose true residual has
//!   reached its tolerance is frozen and leaves the block too.
//! - **Block quasi-minimal residuals** (their Algorithm 4.2): X = V Z with Z minimizing
//!   ‖[ρ; 0] − T Z‖ (Eq. 4.4), T the band of recurrence coefficients (Eq. 3.14), by Givens
//!   rotations as each of T's columns comes (Eqs. 5.1–5.6), and X updated by one rank-one step an
//!   iteration through directions p with short recurrences (Eqs. 5.9–5.10).
//! - **The complex symmetric form** (their Section 6 with J = I): for A = Aᵀ and L = B the left
//!   vectors are the right ones, so they and Aᵀ's products aren't taken: one product with A an
//!   iteration, as [`super::qmr_symmetric`] has for a single right-hand side.
//!
//! The residuals are watched through the quasi-residuals (the last rows of the rotated
//! right-hand side, Eq. 4.9 without its √n), and a system's true residual is computed when its
//! quasi-residual reaches its tolerance; a system stops only on its true residual. A breakdown
//! (|δₙ| below 1e-14, look-ahead not implemented) restarts the block from the iterates it has
//! reached, as [`super::qmr`] does, and so does any system whose recovered solution falls short.
//!
//! Every vector operation is a pass over fixed chunks of the vectors, each chunk's sums taken in
//! order and the chunks' added in order: the same bits on any number of threads.

use std::collections::VecDeque;

use num_complex::Complex64 as c64;

use super::{
    CHUNK, Convergence, Operator, Preconditioner, RightPreconditioned, Stopping, norm, rotate,
};
use crate::{Error, Result};

/// A candidate Lanczos vector is deflated when its biorthogonalized part is at most this
/// fraction of its norm before: their dtol (Remark 3.1), 1e-6 as in their examples, taken
/// relative to the candidate since A's scale is k₀² ε, not 1. Much smaller misses dependence
/// that rounding blurs (a candidate already in the span can keep 1e-7 to 1e-6 of itself once
/// the vectors' biorthogonality has slipped), and a vector kept from that noise stalls the
/// iteration, as their Figs. 1 and 4 show without deflation. What a deflation leaves out of
/// the residual is checked on the true residual, and started again from if it matters.
const DEFLATION: f64 = 1e-6;

/// A breakdown of the Lanczos process: |wₙᵀvₙ| below this for unit vectors.
const BREAKDOWN: f64 = 1e-14;

/// The most times [`block_qmr`] restarts.
const RESTARTS: usize = 10;

/// How a block QMR solve went.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockConvergence {
    /// The block iterations, each one product with A (and one with Aᵀ unless A is complex
    /// symmetric): what one QMR solve per right-hand side would sum its iterations to.
    pub iterations: usize,
    /// All products with A: the iterations', and those of the true residuals checked.
    pub products: usize,
    /// The Lanczos vectors deflated as (almost) dependent, right and left.
    pub deflations: usize,
    /// The systems that left the block as combinations of the others, and were recovered from
    /// their solutions.
    pub dropped: usize,
    /// The times the block was started again from its iterates.
    pub restarts: usize,
    /// Each right-hand side's: the block iteration at which its true residual was reached, that
    /// residual, and its quasi-residual after each iteration, relative to its right-hand side.
    pub columns: Vec<Convergence>,
}

/// Solves A X = B, B's columns `b`, by block QMR from X₀ = 0, each column to the relative
/// residual `stopping` asks, ‖W(bⱼ − A xⱼ)‖ ≤ tol ‖W bⱼ‖ with W the diagonal `weights` (or 1).
/// `symmetric` takes A as complex symmetric (A = Aᵀ, unchecked) and runs the simplified block
/// QMR of Freund and Malhotra's Section 6: one product with A an iteration.
///
/// # Errors
///
/// [`Error::InvalidValue`] if a column doesn't match A, if the tolerance isn't positive, if
/// the tolerance isn't reached within the iterations allowed, or after 10 restarts.
pub(crate) fn block_qmr(
    a: &impl Operator,
    b: &[Vec<c64>],
    stopping: Stopping,
    symmetric: bool,
    weights: Option<&[c64]>,
) -> Result<(Vec<Vec<c64>>, BlockConvergence)> {
    block_qmr_left(a, b, None, stopping, symmetric, weights)
}

/// [`block_qmr`] with the left starting block L given (its general form), not B: as Freund and
/// Malhotra's examples take it, random (their Section 7).
///
/// # Errors
///
/// As [`block_qmr`], and [`Error::InvalidValue`] if L doesn't match B.
pub(crate) fn block_qmr_left(
    a: &impl Operator,
    b: &[Vec<c64>],
    left_block: Option<&[Vec<c64>]>,
    stopping: Stopping,
    symmetric: bool,
    weights: Option<&[c64]>,
) -> Result<(Vec<Vec<c64>>, BlockConvergence)> {
    let n = a.size();
    if left_block.is_some_and(|l| l.len() != b.len() || l.iter().any(|c| c.len() != n)) {
        return Err(Error::invalid(
            "block qmr",
            "the left block needs a column of the size of A for each right-hand side",
        ));
    }
    if let Some(bad) = b.iter().find(|c| c.len() != n) {
        return Err(Error::invalid(
            "block qmr",
            format!("a right-hand side needs {n} values, got {}", bad.len()),
        ));
    }
    if weights.is_some_and(|w| w.len() != n) {
        return Err(Error::invalid(
            "block qmr",
            "the weights need one per unknown",
        ));
    }
    if stopping.tolerance.is_nan() || stopping.tolerance <= 0.0 {
        return Err(Error::invalid(
            "block qmr",
            "the tolerance must be positive",
        ));
    }
    let m = b.len();
    let zero = c64::new(0.0, 0.0);
    let scales: Vec<f64> = b.iter().map(|c| weighted(c, weights)).collect();
    let plain: Vec<f64> = b.iter().map(|c| norm(c)).collect();
    let mut x = vec![vec![zero; n]; m];
    let mut residuals: Vec<Vec<c64>> = b.to_vec();
    let mut reached: Vec<f64> = scales
        .iter()
        .map(|&s| if s == 0.0 { 0.0 } else { 1.0 })
        .collect();
    let mut histories: Vec<Vec<f64>> = vec![Vec::new(); m];
    let mut done_at = vec![0usize; m];
    let mut summary = BlockConvergence {
        iterations: 0,
        products: 0,
        deflations: 0,
        dropped: 0,
        restarts: 0,
        columns: Vec::new(),
    };
    for round in 0..=RESTARTS {
        let open: Vec<usize> = (0..m)
            .filter(|&c| reached[c] > stopping.tolerance)
            .collect();
        if open.is_empty() {
            summary.columns = (0..m)
                .map(|c| Convergence {
                    iterations: done_at[c],
                    residual: reached[c],
                    history: std::mem::take(&mut histories[c]),
                })
                .collect();
            return Ok((x, summary));
        }
        let left = stopping.max_iterations.saturating_sub(summary.iterations);
        if left == 0 {
            return Err(no_convergence(stopping, &reached));
        }
        summary.restarts += usize::from(round > 0);
        let starts: Vec<&[c64]> = open.iter().map(|&c| residuals[c].as_slice()).collect();
        let tolerances: Vec<f64> = open
            .iter()
            .map(|&c| stopping.tolerance * scales[c] / weighted(&residuals[c], weights))
            .collect();
        let lefts: Option<Vec<&[c64]>> =
            left_block.map(|l| open.iter().map(|&c| l[c].as_slice()).collect());
        let run = run(
            a,
            &starts,
            lefts.as_deref(),
            &tolerances,
            symmetric,
            weights,
            left,
        );
        for (i, &c) in open.iter().enumerate() {
            let scale = norm(&residuals[c]) / plain[c];
            add_to(&mut x[c], &run.x[i]);
            histories[c].extend(run.history[i].iter().map(|h| h * scale));
            // a system frozen in the run, or else one recovered or left at the run's end
            let at = if run.done_at[i] > 0 {
                run.done_at[i]
            } else {
                run.iterations
            };
            done_at[c] = summary.iterations + at;
        }
        summary.iterations += run.iterations;
        summary.products += run.products;
        summary.deflations += run.deflations;
        summary.dropped += run.dropped;
        // the true residuals, to stop on or to start again from
        for &c in &open {
            residuals[c] = residual(a, &b[c], &x[c]);
            summary.products += 1;
            reached[c] = weighted(&residuals[c], weights) / scales[c];
        }
        if run.ending == Ending::Limit && open.iter().any(|&c| reached[c] > stopping.tolerance) {
            return Err(no_convergence(stopping, &reached));
        }
    }
    Err(Error::invalid(
        "block qmr",
        format!(
            "started again {RESTARTS} times without every system reaching {:e}",
            stopping.tolerance
        ),
    ))
}

/// [`block_qmr`] on the right-preconditioned system A M⁻¹ Y = B, X = M⁻¹ Y, as
/// [`super::qmr_preconditioned`] for one right-hand side: each iteration one product with A and
/// Aᵀ, one M⁻¹ and one M⁻ᵀ, and the residual each system stops on its own.
///
/// # Errors
///
/// As [`block_qmr`].
pub(crate) fn block_qmr_preconditioned(
    a: &impl Operator,
    m: &impl Preconditioner,
    b: &[Vec<c64>],
    stopping: Stopping,
) -> Result<(Vec<Vec<c64>>, BlockConvergence)> {
    let (y, how) = block_qmr(&RightPreconditioned { a, m }, b, stopping, false, None)?;
    Ok((y.iter().map(|y| m.solve(y)).collect(), how))
}

/// [`block_qmr`] for an A whose similarity B = S A S⁻¹ is complex symmetric, given B and the
/// diagonal S, as [`super::qmr_similar`] for one right-hand side: the symmetric form on
/// B Y = S B, X = S⁻¹ Y, each system stopping on the residual of A X = B itself,
/// ‖S⁻¹(S b − B y)‖ ≤ tol ‖b‖, the weights W = S⁻¹ of [`block_qmr`]. Its histories are B's
/// quasi-residuals, relative to S b.
///
/// # Errors
///
/// As [`block_qmr`].
pub(crate) fn block_qmr_similar(
    b_matrix: &impl Operator,
    s: &[c64],
    b: &[Vec<c64>],
    stopping: Stopping,
) -> Result<(Vec<Vec<c64>>, BlockConvergence)> {
    if s.len() != b_matrix.size() {
        return Err(Error::invalid("block qmr", "S needs one value per unknown"));
    }
    let c: Vec<Vec<c64>> = b
        .iter()
        .map(|column| column.iter().zip(s).map(|(x, y)| x * y).collect())
        .collect();
    let inverse: Vec<c64> = s.iter().map(|z| 1.0 / z).collect();
    let (y, how) = block_qmr(b_matrix, &c, stopping, true, Some(&inverse))?;
    let x = y
        .iter()
        .map(|y| y.iter().zip(&inverse).map(|(p, q)| p * q).collect())
        .collect();
    Ok((x, how))
}

fn no_convergence(stopping: Stopping, reached: &[f64]) -> Error {
    Error::invalid(
        "block qmr",
        format!(
            "no convergence to {:e} in {} iterations (the worst residual is at {:e})",
            stopping.tolerance,
            stopping.max_iterations,
            reached.iter().copied().fold(0.0, f64::max)
        ),
    )
}

/// How one run of the block Lanczos process ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ending {
    /// Every system reached its tolerance or left the block.
    Converged,
    /// The right block Krylov space is exhausted: no candidate is left.
    Exhausted,
    /// A breakdown, wₙᵀvₙ = 0, or the left space exhausted before the right.
    Broken,
    /// The iterations allowed.
    Limit,
}

/// What one run gives: each start's solution, quasi-residual history and the iteration at
/// which it was frozen (0 if it never was).
struct Outcome {
    x: Vec<Vec<c64>>,
    history: Vec<Vec<f64>>,
    done_at: Vec<usize>,
    iterations: usize,
    products: usize,
    deflations: usize,
    dropped: usize,
    ending: Ending,
}

/// Where a candidate Lanczos vector comes from.
#[derive(Clone, Copy)]
enum Source {
    /// A starting vector, a column of B (for the left vectors too: L = B).
    Start(usize),
    /// The product of A (or Aᵀ) with the Lanczos vector of this index.
    Product(usize),
}

/// A system that left the block: its solution is `base` + Σ c xₖ over `terms`.
struct Dropped {
    system: usize,
    base: Option<Vec<c64>>,
    terms: Vec<(usize, c64)>,
}

/// The run's state.
struct Run<'a, A> {
    a: &'a A,
    symmetric: bool,
    weights: Option<&'a [c64]>,
    starts: &'a [&'a [c64]],
    /// The left starting vectors L, if not B's columns.
    lefts: Option<&'a [&'a [c64]]>,
    start_norms: Vec<f64>,
    tolerances: Vec<f64>,
    /// Each start's norm in the weights' metric.
    scales: Vec<f64>,
    thresholds: Vec<f64>,
    max_iterations: usize,
    /// The right and left Lanczos vectors, `None` once no later step needs them; the left
    /// ones empty for a symmetric A, whose left vectors are the right ones.
    v: Vec<Option<Vec<c64>>>,
    w: Vec<Option<Vec<c64>>>,
    delta: Vec<c64>,
    /// Each vector's source: the index of the vector whose product it came from, or none for
    /// a starting vector; a candidate from A vⱼ meets the left vectors from wₖ on, k this
    /// source of wⱼ (their history index φⱼ), and one from Aᵀ wⱼ the right ones from vⱼ's.
    source_v: Vec<Option<usize>>,
    source_w: Vec<Option<usize>>,
    /// Their sets 𝓘ᵥ and 𝓘𝓌: the vectors whose products were deflated with something left.
    deflated_v: Vec<usize>,
    deflated_w: Vec<usize>,
    next_start_v: usize,
    next_start_w: usize,
    mu: usize,
    phi: usize,
    /// The vectors below this index have been let go, but for the sets'.
    released: usize,
    /// Products taken ahead for the next steps' candidates (their Section 8: once vₙ is
    /// built, the products the next m steps take, A v_μ … A vₙ, are all known): A vⱼ from
    /// j = `mu` on, and Aᵀ wⱼ from `phi` on, a block at a time.
    ahead_v: VecDeque<Vec<c64>>,
    ahead_w: VecDeque<Vec<c64>>,
    /// The systems still in the block, by start, and the rows of the rotated right-hand side
    /// Q[ρ; 0] from row `top` on, one value per system in the block.
    active: Vec<usize>,
    window: VecDeque<Vec<c64>>,
    top: usize,
    /// Each kept start's coefficients ρ on the vectors up to its own.
    start_rows: Vec<Vec<c64>>,
    /// Each column of T's rotations: column j's k-th turns rows j and j + k.
    rotations: Vec<Vec<(f64, c64)>>,
    /// The latest directions p, by column.
    directions: VecDeque<(usize, Vec<c64>)>,
    /// The widest block, m: T's lower and upper bandwidths are at most m, U's 2m.
    width: usize,
    x: Vec<Vec<c64>>,
    history: Vec<Vec<f64>>,
    done_at: Vec<usize>,
    dropped: Vec<Dropped>,
    products: usize,
    deflations: usize,
}

fn run(
    a: &impl Operator,
    starts: &[&[c64]],
    lefts: Option<&[&[c64]]>,
    tolerances: &[f64],
    symmetric: bool,
    weights: Option<&[c64]>,
    max_iterations: usize,
) -> Outcome {
    let n = a.size();
    let m = starts.len();
    let zero = c64::new(0.0, 0.0);
    let mut state = Run {
        a,
        symmetric,
        weights,
        starts,
        lefts,
        start_norms: starts.iter().map(|s| norm(s)).collect(),
        tolerances: tolerances.to_vec(),
        scales: starts.iter().map(|s| weighted(s, weights)).collect(),
        thresholds: tolerances.to_vec(),
        max_iterations,
        v: Vec::new(),
        w: Vec::new(),
        delta: Vec::new(),
        source_v: Vec::new(),
        source_w: Vec::new(),
        deflated_v: Vec::new(),
        deflated_w: Vec::new(),
        next_start_v: 0,
        next_start_w: 0,
        mu: 0,
        phi: 0,
        released: 0,
        ahead_v: VecDeque::new(),
        ahead_w: VecDeque::new(),
        active: Vec::new(),
        window: VecDeque::new(),
        top: 0,
        start_rows: Vec::new(),
        rotations: Vec::new(),
        directions: VecDeque::new(),
        width: m,
        x: vec![vec![zero; n]; m],
        history: vec![Vec::new(); m],
        done_at: vec![0; m],
        dropped: Vec::new(),
        products: 0,
        deflations: 0,
    };
    let ending = state.go();
    state.recover();
    Outcome {
        iterations: state.rotations.len(),
        x: state.x,
        history: state.history,
        done_at: state.done_at,
        products: state.products,
        deflations: state.deflations,
        dropped: state.dropped.len(),
        ending,
    }
}

impl<A: Operator> Run<'_, A> {
    /// The right vector `i`.
    fn right(&self, i: usize) -> &[c64] {
        self.v[i]
            .as_deref()
            .expect("a right Lanczos vector still kept")
    }

    /// The left vector `i`: the right one for a symmetric A.
    fn left(&self, i: usize) -> &[c64] {
        if self.symmetric {
            self.right(i)
        } else {
            self.w[i]
                .as_deref()
                .expect("a left Lanczos vector still kept")
        }
    }

    /// The Lanczos process and the iterates, to the end of the run.
    fn go(&mut self) -> Ending {
        let m = self.starts.len();
        loop {
            // (1) the next right vector (their Algorithm 3.1, step 1): each candidate in turn
            // until one isn't deflated, a product's column of T taken by QMR either way
            loop {
                let source = if self.next_start_v < m {
                    Source::Start(self.next_start_v)
                } else if self.mu < self.v.len() {
                    Source::Product(self.mu)
                } else {
                    return Ending::Exhausted;
                };
                let lo = match source {
                    Source::Start(_) => 0,
                    Source::Product(j) => if self.symmetric {
                        self.source_v[j]
                    } else {
                        self.source_w[j]
                    }
                    .unwrap_or(0),
                };
                let extra = if self.symmetric {
                    &self.deflated_v
                } else {
                    &self.deflated_w
                };
                let rows: Vec<usize> = extra
                    .iter()
                    .copied()
                    .filter(|&i| i < lo)
                    .chain(lo..self.v.len())
                    .collect();
                let mut candidate = match source {
                    Source::Start(c) => self.starts[c].to_vec(),
                    Source::Product(j) => self.product(j, true),
                };
                let (t, before, after) = self.biorthogonalize(&mut candidate, &rows, true);
                let deflate = after <= DEFLATION * before;
                // the column's entries in T's band, rows `lo` on (their T^b, Eq. 3.14)
                let band: Vec<(usize, c64)> = rows
                    .iter()
                    .zip(&t)
                    .filter(|(i, _)| **i >= lo)
                    .map(|(&i, &c)| (i, c))
                    .collect();
                match source {
                    Source::Start(c) => {
                        self.next_start_v += 1;
                        if deflate {
                            self.deflations += 1;
                            self.drop_start(c, &t);
                            continue;
                        }
                        self.push_right(candidate, after, None);
                        self.add_start(c, &t, after);
                    }
                    Source::Product(j) => {
                        self.mu += 1;
                        let last = if deflate {
                            self.deflations += 1;
                            if after > 0.0 {
                                self.deflated_v.push(j);
                            }
                            self.v.len() - 1
                        } else {
                            self.push_right(candidate, after, Some(j));
                            self.window
                                .push_back(vec![c64::new(0.0, 0.0); self.active.len()]);
                            self.v.len() - 1
                        };
                        let mut column = band;
                        if !deflate {
                            column.push((last, c64::new(after, 0.0)));
                        }
                        if let Some(ending) = self.column(j, &column, last, deflate) {
                            return ending;
                        }
                        if deflate {
                            continue;
                        }
                    }
                }
                break;
            }
            let k = self.v.len() - 1;
            // (2) the next left vector, unless A is symmetric; then δₖ = wₖᵀvₖ (step 4)
            if !self.symmetric {
                loop {
                    let source = if self.next_start_w < m {
                        Source::Start(self.next_start_w)
                    } else if self.phi < self.w.len() {
                        Source::Product(self.phi)
                    } else {
                        return Ending::Broken;
                    };
                    let lo = match source {
                        Source::Start(_) => 0,
                        Source::Product(j) => self.source_v[j].unwrap_or(0),
                    };
                    let rows: Vec<usize> = self
                        .deflated_v
                        .iter()
                        .copied()
                        .filter(|&i| i < lo)
                        .chain(lo..k)
                        .collect();
                    let mut candidate = match source {
                        Source::Start(c) => self.lefts.map_or(self.starts[c], |l| l[c]).to_vec(),
                        Source::Product(j) => self.product(j, false),
                    };
                    let (_, before, after) = self.biorthogonalize(&mut candidate, &rows, false);
                    let deflate = after <= DEFLATION * before;
                    match source {
                        Source::Start(_) => self.next_start_w += 1,
                        Source::Product(j) => {
                            self.phi += 1;
                            if deflate && after > 0.0 {
                                self.deflated_w.push(j);
                            }
                        }
                    }
                    if deflate {
                        self.deflations += 1;
                        continue;
                    }
                    let delta = scale(&mut candidate, 1.0 / after, Some(self.right(k)));
                    self.w.push(Some(candidate));
                    self.source_w.push(match source {
                        Source::Start(_) => None,
                        Source::Product(j) => Some(j),
                    });
                    self.delta.push(delta);
                    break;
                }
            }
            if self.delta[k].norm() < BREAKDOWN {
                return Ending::Broken;
            }
            self.release();
        }
    }

    /// A vⱼ (`right`) or Aᵀ wⱼ, j the next product's: from the products taken ahead, which
    /// are taken for every vector from j on when there are none left.
    fn product(&mut self, j: usize, right: bool) -> Vec<c64> {
        let ahead = if right { &self.ahead_v } else { &self.ahead_w };
        if ahead.is_empty() {
            let products = if right {
                let vs: Vec<&[c64]> = (j..self.v.len()).map(|i| self.right(i)).collect();
                self.a.apply_block(&vs)
            } else {
                let ws: Vec<&[c64]> = (j..self.w.len()).map(|i| self.left(i)).collect();
                self.a.apply_transpose_block(&ws)
            };
            if right {
                self.products += products.len();
                self.ahead_v.extend(products);
            } else {
                self.ahead_w.extend(products);
            }
        }
        let ahead = if right {
            &mut self.ahead_v
        } else {
            &mut self.ahead_w
        };
        ahead.pop_front().expect("a product taken ahead")
    }

    /// Biorthogonalizes `candidate` against the vectors `rows` (their Eqs. 3.3–3.4 or 3.5–3.6):
    /// the coefficients tᵢ = (left ᵢ)ᵀ candidate / δᵢ, found together on the candidate as it
    /// came, then subtracted together. `right` for a right candidate (against the left
    /// vectors, minus the right ones), else a left one. Returns the coefficients and the
    /// candidate's norm before and after.
    fn biorthogonalize(
        &self,
        candidate: &mut [c64],
        rows: &[usize],
        right: bool,
    ) -> (Vec<c64>, f64, f64) {
        let against: Vec<&[c64]> = rows
            .iter()
            .map(|&i| if right { self.left(i) } else { self.right(i) })
            .collect();
        let along: Vec<&[c64]> = rows
            .iter()
            .map(|&i| if right { self.right(i) } else { self.left(i) })
            .collect();
        // in the order of modified Gram–Schmidt (their Remark 3.2): each coefficient from the
        // candidate as the earlier ones left it, each pass subtracting one term and finding the
        // next's coefficient; found together instead (classical Gram–Schmidt), the vectors lose
        // their biorthogonality to their neighbours exponentially, 1e-16 to 1e-5 in 30 steps on
        // a nonsymmetric test matrix
        let mut t: Vec<c64> = Vec::with_capacity(rows.len());
        let mut before = 0.0;
        let mut size = 0.0;
        for k in 0..=rows.len() {
            let remove = k.checked_sub(1).map(|p| (t[p], along[p]));
            let (d, s) = step(candidate, remove, against.get(k).copied());
            if k == 0 {
                before = s;
            }
            size = s;
            if k < rows.len() {
                t.push(d / self.delta[rows[k]]);
            }
        }
        (t, before.sqrt(), size.sqrt())
    }

    /// A new right vector, `candidate` of norm `size` normalized (their Eq. 3.7); for a
    /// symmetric A, its δ = vᵀv too.
    fn push_right(&mut self, mut candidate: Vec<c64>, size: f64, source: Option<usize>) {
        let delta = scale(&mut candidate, 1.0 / size, None);
        self.v.push(Some(candidate));
        self.source_v.push(source);
        if self.symmetric {
            self.delta.push(delta);
        }
    }

    /// A kept start, system `c`: a new column of ρ, its coefficients `t` on the vectors before
    /// its own and `size` on its own (their Eqs. 3.10–3.11).
    fn add_start(&mut self, c: usize, t: &[c64], size: f64) {
        debug_assert_eq!(self.top, 0);
        let mut coefficients = t.to_vec();
        coefficients.push(c64::new(size, 0.0));
        for (row, &value) in self.window.iter_mut().zip(t) {
            row.push(value);
        }
        let mut row = vec![c64::new(0.0, 0.0); self.active.len()];
        row.push(c64::new(size, 0.0));
        self.window.push_back(row);
        self.active.push(c);
        self.start_rows.push(coefficients);
    }

    /// A start deflated: system `c`'s right-hand side is Σ γₖ of the kept ones' to within the
    /// deflation tolerance, ρ_K γ = t with ρ_K the kept starts' triangle, so xc = Σ γₖ xₖ
    /// (their Eq. 4.18 with X₀ = 0).
    fn drop_start(&mut self, c: usize, t: &[c64]) {
        let kept = self.start_rows.len();
        let mut gamma = vec![c64::new(0.0, 0.0); kept];
        for k in (0..kept).rev() {
            let mut s = t[k];
            for (kk, g) in gamma.iter().enumerate().skip(k + 1) {
                s -= self.start_rows[kk][k] * g;
            }
            gamma[k] = s / self.start_rows[k][k];
        }
        self.drop(Dropped {
            system: c,
            base: None,
            terms: self.active.iter().copied().zip(gamma).collect(),
        });
    }

    /// A system leaves the block, to be recovered from the others: their tolerances tightened
    /// so that its residual, Σ cₖ rₖ, meets its own, Σ |cₖ| tolₖ ‖bₖ‖ ≤ tol ‖b‖.
    fn drop(&mut self, d: Dropped) {
        let spread: f64 = d
            .terms
            .iter()
            .map(|&(k, c)| c.norm() * self.scales[k])
            .sum();
        if spread > 0.0 {
            let bound = self.tolerances[d.system] * self.scales[d.system] / spread;
            for &(k, _) in &d.terms {
                if bound < self.tolerances[k] {
                    self.thresholds[k] *= bound / self.tolerances[k];
                    self.tolerances[k] = bound;
                }
            }
        }
        self.dropped.push(d);
    }

    /// QMR on T's column `j`, the coefficients `entries` (row, value) of A vⱼ, rows up to
    /// `last` (their Algorithm 4.2, step 2, and Section 5): the earlier rotations and new ones
    /// that zero it below row j, the rotated right-hand side's row j as the step yⱼ, the
    /// direction pⱼ and the iterates; a system dropped if `deflated` left the block more systems
    /// than quasi-residuals; then the systems whose quasi-residual reached their threshold
    /// checked on their true residual. An ending, if the run ends here.
    fn column(
        &mut self,
        j: usize,
        entries: &[(usize, c64)],
        last: usize,
        deflated: bool,
    ) -> Option<Ending> {
        let zero = c64::new(0.0, 0.0);
        debug_assert_eq!(self.rotations.len(), j);
        debug_assert_eq!(self.top, j);
        let lowest = entries.first().map_or(j, |e| e.0).min(j);
        let r0 = lowest.saturating_sub(self.width);
        let mut col = vec![zero; last + 1 - r0];
        for &(i, t) in entries {
            col[i - r0] = t;
        }
        // the earlier columns' rotations, in order (their Eq. 5.6): column jj's turn rows jj
        // and jj + k; those before r0 meet only zeros
        for jj in r0..j {
            for (k, &g) in self.rotations[jj].iter().enumerate() {
                let (p, q) = (jj - r0, jj + k + 1 - r0);
                (col[p], col[q]) = rotate(g, col[p], col[q]);
            }
        }
        // new rotations zeroing rows j + 1 … last against row j (their Eqs. 5.2–5.3)
        let mut turns = Vec::with_capacity(last - j);
        for k in 1..=last - j {
            let (p, q) = (j - r0, j + k - r0);
            let g = givens(col[p], col[q]);
            (col[p], col[q]) = rotate(g, col[p], col[q]);
            turns.push(g);
        }
        let diagonal = col[j - r0];
        if diagonal.norm() == 0.0 {
            return Some(Ending::Broken);
        }
        // the same rotations on the right-hand side's rows j … last (their Eq. 5.7)
        for (k, &g) in turns.iter().enumerate() {
            for c in 0..self.active.len() {
                let (p, q) = (self.window[0][c], self.window[k + 1][c]);
                let (p, q) = rotate(g, p, q);
                self.window[0][c] = p;
                self.window[k + 1][c] = q;
            }
        }
        self.rotations.push(turns);
        let step = self.window.pop_front().expect("the window's row j");
        self.top += 1;
        // pⱼ = (vⱼ − Σ pᵢ uᵢⱼ) / uⱼⱼ (Eq. 5.10) and xc += pⱼ yⱼc (Eq. 4.21), in one pass
        let previous: Vec<(&[c64], c64)> = self
            .directions
            .iter()
            .filter(|(i, _)| *i >= r0)
            .map(|(i, p)| (p.as_slice(), col[i - r0]))
            .filter(|(_, u)| u.norm() != 0.0)
            .collect();
        debug_assert!(
            (r0..j).all(|i| col[i - r0].norm() == 0.0 || self.directions.iter().any(|d| d.0 == i)),
            "a direction needed has been let go"
        );
        let mut targets: Vec<(&mut [c64], c64)> = Vec::with_capacity(self.active.len());
        {
            let mut by_system: Vec<Option<&mut Vec<c64>>> = self.x.iter_mut().map(Some).collect();
            for (pos, &c) in self.active.iter().enumerate() {
                if let Some(xc) = by_system[c].take() {
                    targets.push((xc.as_mut_slice(), step[pos]));
                }
            }
        }
        let p = direction(
            self.v[j].as_deref().expect("vⱼ kept"),
            &previous,
            diagonal,
            targets,
        );
        self.directions.push_back((j, p));
        while self
            .directions
            .front()
            .is_some_and(|d| d.0 + 2 * self.width < j + 1)
        {
            self.directions.pop_front();
        }
        let iteration = j + 1;
        // the quasi-residuals: the right-hand side's rows below j (Eq. 4.9)
        let quasi: Vec<f64> = (0..self.active.len())
            .map(|pos| {
                let c = self.active[pos];
                let sum: f64 = self.window.iter().map(|row| row[pos].norm_sqr()).sum();
                sum.sqrt() / self.start_norms[c]
            })
            .collect();
        for (pos, &c) in self.active.iter().enumerate() {
            self.history[c].push(quasi[pos]);
        }
        // a deflated product: fewer quasi-residuals than systems, so a combination of the
        // systems has none; the one weighing most in it leaves (their Eqs. 4.16–4.20)
        let mut quasi = quasi;
        if deflated && self.active.len() > self.window.len() {
            let rows: Vec<Vec<c64>> = self.window.iter().cloned().collect();
            let gamma = null_vector(&rows, self.active.len());
            let (pos, _) = gamma.iter().enumerate().fold((0, -1.0), |best, (i, g)| {
                if g.norm() > best.1 {
                    (i, g.norm())
                } else {
                    best
                }
            });
            let system = self.active[pos];
            let terms: Vec<(&[c64], c64)> = self
                .active
                .iter()
                .zip(&gamma)
                .map(|(&c, &g)| (self.x[c].as_slice(), g))
                .collect();
            let mut base = vec![zero; self.a.size()];
            combine_into(&mut base, &terms);
            let g = gamma[pos];
            base.iter_mut().for_each(|z| *z /= g);
            let terms = self
                .active
                .iter()
                .zip(&gamma)
                .filter(|(c, _)| **c != system)
                .map(|(&c, &gc)| (c, -gc / g))
                .collect();
            self.drop(Dropped {
                system,
                base: Some(base),
                terms,
            });
            self.remove(pos);
            quasi.remove(pos);
        }
        // the systems whose quasi-residual reached their threshold, on their true residuals
        let mut pos = 0;
        while pos < self.active.len() {
            let c = self.active[pos];
            if quasi[pos] <= self.thresholds[c] {
                let r = residual(self.a, self.starts[c], &self.x[c]);
                self.products += 1;
                let reached = weighted(&r, self.weights) / self.scales[c];
                if reached <= self.tolerances[c] {
                    self.done_at[c] = iteration;
                    self.remove(pos);
                    quasi.remove(pos);
                    continue;
                }
                // the true residual lags the quasi-residual by as much again
                self.thresholds[c] = quasi[pos] * 0.5 * self.tolerances[c] / reached;
            }
            pos += 1;
        }
        if self.active.is_empty() {
            return Some(Ending::Converged);
        }
        if iteration >= self.max_iterations {
            return Some(Ending::Limit);
        }
        None
    }

    /// The system at `pos` in the block leaves it.
    fn remove(&mut self, pos: usize) {
        self.active.remove(pos);
        for row in &mut self.window {
            row.remove(pos);
        }
    }

    /// Lets go of the vectors no later step needs: the next candidates meet the vectors from
    /// their history indices on, which only grow, and the sets' are kept.
    fn release(&mut self) {
        let m = self.starts.len();
        let from_right = if self.next_start_v < m || self.mu >= self.v.len() {
            0
        } else if self.symmetric {
            self.source_v[self.mu].unwrap_or(0)
        } else {
            self.source_w[self.mu].unwrap_or(0)
        };
        let from_left = if self.symmetric {
            from_right
        } else if self.next_start_w < m || self.phi >= self.w.len() {
            0
        } else {
            self.source_v[self.phi].unwrap_or(0)
        };
        // and the products' own vectors, vμ and w_φ on
        let keep = if self.symmetric {
            from_right.min(self.mu)
        } else {
            from_right.min(from_left).min(self.mu).min(self.phi)
        };
        for i in self.released..keep {
            if self.deflated_v.contains(&i) || self.deflated_w.contains(&i) {
                continue;
            }
            self.v[i] = None;
            if !self.symmetric {
                self.w[i] = None;
            }
        }
        self.released = self.released.max(keep);
    }

    /// The dropped systems' solutions from the others', the last dropped first.
    fn recover(&mut self) {
        let dropped = std::mem::take(&mut self.dropped);
        for d in dropped.iter().rev() {
            let mut x = d
                .base
                .clone()
                .unwrap_or_else(|| vec![c64::new(0.0, 0.0); self.a.size()]);
            let terms: Vec<(&[c64], c64)> = d
                .terms
                .iter()
                .map(|&(c, g)| (self.x[c].as_slice(), g))
                .collect();
            combine_into(&mut x, &terms);
            self.x[d.system] = x;
        }
        self.dropped = dropped;
    }
}

/// The Givens rotation (c, s) of [`rotate`] that zeroes b against a.
fn givens(a: c64, b: c64) -> (f64, c64) {
    if b.norm() == 0.0 {
        (1.0, c64::new(0.0, 0.0))
    } else if a.norm() == 0.0 {
        (0.0, c64::new(1.0, 0.0))
    } else {
        let c = a.norm() / a.norm().hypot(b.norm());
        (c, (c * b / a).conj())
    }
}

/// A unit vector γ with R γ = 0 for the k × `columns` matrix R of `rows`, k < `columns`: the
/// unit vector eᵢ with the most left outside the span of R's conjugated rows, projected out of
/// it (Gram–Schmidt twice).
fn null_vector(rows: &[Vec<c64>], columns: usize) -> Vec<c64> {
    let zero = c64::new(0.0, 0.0);
    let inner = |u: &[c64], v: &[c64]| -> c64 { u.iter().zip(v).map(|(a, b)| a.conj() * b).sum() };
    let length = |u: &[c64]| u.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
    let project_out = |basis: &[Vec<c64>], u: &mut Vec<c64>| {
        for _ in 0..2 {
            for q in basis {
                let d = inner(q, u);
                for (x, y) in u.iter_mut().zip(q) {
                    *x -= d * y;
                }
            }
        }
    };
    let mut basis: Vec<Vec<c64>> = Vec::new();
    for row in rows {
        let mut u: Vec<c64> = row.iter().map(|z| z.conj()).collect();
        project_out(&basis, &mut u);
        let size = length(&u);
        if size > 0.0 {
            basis.push(u.iter().map(|z| z / size).collect());
        }
    }
    let mut best = vec![zero; columns];
    let mut best_size = -1.0;
    for i in 0..columns {
        let mut u = vec![zero; columns];
        u[i] = c64::new(1.0, 0.0);
        project_out(&basis, &mut u);
        let size = length(&u);
        if size > best_size {
            best_size = size;
            best = u;
        }
    }
    best.iter().map(|z| z / best_size).collect()
}

/// ‖W r‖ for the diagonal W of `weights`, or ‖r‖: chunk by chunk, in order.
fn weighted(r: &[c64], weights: Option<&[c64]>) -> f64 {
    use rayon::prelude::*;
    let parts: Vec<f64> = match weights {
        Some(w) => r
            .par_chunks(CHUNK)
            .zip(w.par_chunks(CHUNK))
            .map(|(p, q)| p.iter().zip(q).map(|(a, b)| (a * b).norm_sqr()).sum())
            .collect(),
        None => r
            .par_chunks(CHUNK)
            .map(|p| p.iter().map(|a| a.norm_sqr()).sum())
            .collect(),
    };
    parts.iter().sum::<f64>().sqrt()
}

/// b − A x.
fn residual(a: &impl Operator, b: &[c64], x: &[c64]) -> Vec<c64> {
    use rayon::prelude::*;
    let mut ax = vec![c64::new(0.0, 0.0); b.len()];
    a.apply_into(x, &mut ax);
    ax.par_iter_mut()
        .with_min_len(CHUNK)
        .zip(b)
        .for_each(|(q, p)| *q = p - *q);
    ax
}

/// x += d.
fn add_to(x: &mut [c64], d: &[c64]) {
    use rayon::prelude::*;
    x.par_iter_mut()
        .with_min_len(CHUNK)
        .zip(d)
        .for_each(|(a, b)| *a += b);
}

/// One pass of modified Gram–Schmidt on x: x −= c u for `remove` (c, u), then uᵀx for
/// `dot` u (unconjugated, their bilinear form) and ‖x‖², on the values as updated.
fn step(x: &mut [c64], remove: Option<(c64, &[c64])>, dot: Option<&[c64]>) -> (c64, f64) {
    use rayon::prelude::*;
    let read = 1 + usize::from(remove.is_some()) + usize::from(dot.is_some());
    crate::traffic::add(crate::traffic::vectors(
        x.len(),
        read,
        usize::from(remove.is_some()),
    ));
    let zero = c64::new(0.0, 0.0);
    let parts: Vec<(c64, f64)> = x
        .par_chunks_mut(CHUNK)
        .enumerate()
        .map(|(chunk, xs)| {
            let start = chunk * CHUNK;
            let (mut d, mut size) = (zero, 0.0);
            for (i, xi) in xs.iter_mut().enumerate() {
                let k = start + i;
                if let Some((c, u)) = remove {
                    *xi -= c * u[k];
                }
                if let Some(u) = dot {
                    d += u[k] * *xi;
                }
                size += xi.norm_sqr();
            }
            (d, size)
        })
        .collect();
    parts
        .iter()
        .fold((zero, 0.0), |s, p| (s.0 + p.0, s.1 + p.1))
}

/// x *= `by`, and Σ xₖ yₖ for y = `other`, or x itself.
fn scale(x: &mut [c64], by: f64, other: Option<&[c64]>) -> c64 {
    use rayon::prelude::*;
    crate::traffic::add(crate::traffic::vectors(
        x.len(),
        1 + usize::from(other.is_some()),
        1,
    ));
    let parts: Vec<c64> = x
        .par_chunks_mut(CHUNK)
        .enumerate()
        .map(|(chunk, xs)| {
            let start = chunk * CHUNK;
            let mut sum = c64::new(0.0, 0.0);
            for (i, xi) in xs.iter_mut().enumerate() {
                *xi *= by;
                sum += *xi * other.map_or(*xi, |y| y[start + i]);
            }
            sum
        })
        .collect();
    parts.iter().sum()
}

/// out += Σ cᵢ uᵢ, each value's terms in order.
fn combine_into(out: &mut [c64], terms: &[(&[c64], c64)]) {
    use rayon::prelude::*;
    crate::traffic::add(crate::traffic::vectors(out.len(), terms.len() + 1, 1));
    out.par_iter_mut()
        .with_min_len(CHUNK)
        .enumerate()
        .for_each(|(k, o)| {
            for (u, c) in terms {
                *o += c * u[k];
            }
        });
}

/// The direction p = (v − Σ pᵢ uᵢ) / `diagonal` over `previous` (pᵢ, uᵢ), and each target
/// x += y p, in one pass.
fn direction(
    v: &[c64],
    previous: &[(&[c64], c64)],
    diagonal: c64,
    targets: Vec<(&mut [c64], c64)>,
) -> Vec<c64> {
    use rayon::prelude::*;
    let n = v.len();
    crate::traffic::add(crate::traffic::vectors(
        n,
        1 + previous.len() + targets.len(),
        1 + targets.len(),
    ));
    let chunks = n.div_ceil(CHUNK);
    let mut by_chunk: Vec<Vec<(&mut [c64], c64)>> = (0..chunks).map(|_| Vec::new()).collect();
    for (x, y) in targets {
        for (i, part) in x.chunks_mut(CHUNK).enumerate() {
            by_chunk[i].push((part, y));
        }
    }
    let inverse = 1.0 / diagonal;
    let mut p = vec![c64::new(0.0, 0.0); n];
    p.par_chunks_mut(CHUNK)
        .zip(by_chunk.into_par_iter())
        .enumerate()
        .for_each(|(chunk, (ps, mut xs))| {
            let start = chunk * CHUNK;
            for (i, pk) in ps.iter_mut().enumerate() {
                let k = start + i;
                let mut s = v[k];
                for (q, u) in previous {
                    s -= u * q[k];
                }
                let value = s * inverse;
                *pk = value;
                for (x, y) in xs.iter_mut() {
                    x[i] += *y * value;
                }
            }
        });
    p
}

#[path = "example.rs"]
pub(crate) mod example;

#[cfg(test)]
#[path = "block_tests.rs"]
mod tests;
