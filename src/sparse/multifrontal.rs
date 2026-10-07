//! A multifrontal LU on the structure of A + Aᵀ with static pivoting: the factorization that
//! keeps PARDISO's and MUMPS's fill (docs/baselines.md), where faer's LU, pivoting by rows
//! anywhere in a column, works in AᵀA's structure.
//!
//! - **Static pivoting** (X. S. Li, J. W. Demmel, "SuperLU_DIST", ACM Trans. Math. Softw. 29, 110
//!   (2003), doi:10.1145/779359.779361, their Figure 1): large entries are first permuted onto
//!   the diagonal and the matrix scaled ([`super::matching`], Duff & Koster's MC64 option 5),
//!   so pivots can be taken from the diagonal blocks; a pivot still smaller than √ε ‖A‖₁ is set
//!   to that (a half-precision perturbation), and the solve is refined afterwards.
//! - **The fronts** (I. S. Duff, J. K. Reid, "The multifrontal solution of indefinite sparse
//!   symmetric linear equations", ACM Trans. Math. Softw. 9, 302 (1983),
//!   doi:10.1145/356044.356047): each supernode's front is a dense matrix, its rows and columns
//!   the supernode's and those its factor's columns reach; it sums the matrix's entries and its
//!   children's update matrices, eliminates the supernode's columns, and passes the Schur
//!   complement up to its parent. Here the unsymmetric version on a symmetric structure: the
//!   front's L and U parts share their index list. Pivoting stays within the supernode's own
//!   rows (partial pivoting inside the diagonal block), so the structure is the symbolic one.
//! - **The tree** (J. W. H. Liu, "The role of elimination trees in sparse factorization", SIAM
//!   J. Matrix Anal. Appl. 11, 134 (1990), doi:10.1137/0611010): a front needs only its
//!   children's updates, so independent subtrees are factorized on separate threads, each front
//!   the same computation whatever the threads (its children's updates summed in their order),
//!   so the factors are the same bits on any number of threads.
//!
//! The supernodes and their row structures are faer's symbolic supernodal Cholesky of the
//! structure of A + Aᵀ, in our order: nested dissection with one-step separators on a grid
//! (George's sets, [`super::nested_dissection`]), or AMD otherwise.

use std::sync::{Arc, OnceLock};

use faer::dyn_stack::{MemBuffer, MemStack};
use faer::linalg::matmul::matmul;
use faer::linalg::triangular_solve::{
    solve_lower_triangular_in_place, solve_unit_lower_triangular_in_place,
    solve_unit_upper_triangular_in_place, solve_upper_triangular_in_place,
};
use faer::perm::PermRef;
use faer::sparse::linalg::SupernodalThreshold;
use faer::sparse::linalg::cholesky::{
    CholeskySymbolicParams, SymbolicCholeskyRaw, SymmetricOrdering, factorize_symbolic_cholesky,
};
use faer::sparse::{SparseColMat, SparseColMatRef, Triplet};
use faer::{Accum, Mat, MatMut, Par, Side};
use num_complex::Complex64 as c64;
use rayon::prelude::*;

use super::matching::{Matching, matching};
use super::{adjacency, nested_dissection};
use crate::{Error, Result};

fn failed(what: &str, why: impl std::fmt::Display) -> Error {
    Error::invalid("the multifrontal LU", format!("{what}: {why}"))
}

/// One supernode: its columns `begin..end` (in the elimination order), the rows below them that
/// its factor's columns reach (ascending), its children.
#[derive(Clone, Debug)]
struct Front {
    begin: usize,
    end: usize,
    below: Vec<usize>,
    children: Vec<usize>,
    /// The front's work, to decide what is worth a thread: its subtree's flops, estimated.
    work: f64,
}

/// The analysis: the matching and scaling, the elimination order and the fronts. It depends on
/// the matrix's structure and, through the matching, on its values; a matrix of the same
/// structure and similar values (another wavelength) is factorized with it as it is.
#[derive(Debug)]
pub(crate) struct Analysis {
    n: usize,
    matching: Matching,
    /// The node (column, with its matched row) eliminated k-th.
    order: Vec<usize>,
    /// The inverse: the step at which node j is eliminated.
    step: Vec<usize>,
    fronts: Vec<Front>,
    roots: Vec<usize>,
    entries: usize,
    /// Factorized as complex symmetric, L D Lᵀ: the matrix is (B = Bᵀ, not Hermitian), the
    /// matching is the identity and the scaling the same on rows and columns.
    symmetric: bool,
}

/// `a`'s columns as (row, value) lists.
fn columns_of(a: SparseColMatRef<'_, usize, c64>) -> Vec<Vec<(usize, c64)>> {
    (0..a.ncols())
        .map(|j| {
            a.row_idx_of_col(j)
                .zip(a.val_of_col(j))
                .map(|(i, &v)| (i, v))
                .collect()
        })
        .collect()
}

impl Analysis {
    /// The analysis of `a` (square) for its LU. With `positions`, each unknown's place on its
    /// grid, the order is nested dissection with one-step separators; without, AMD.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if `a` isn't square, is structurally singular, or faer's
    /// analysis fails (out of memory).
    pub(crate) fn new(
        a: SparseColMatRef<'_, usize, c64>,
        positions: Option<&[[f64; 3]]>,
    ) -> Result<Analysis> {
        let n = a.ncols();
        if a.nrows() != n {
            return Err(failed(
                "the matrix",
                format!("{} × {n}, not square", a.nrows()),
            ));
        }
        let matching = matching(n, &columns_of(a))?;
        Self::with(a, positions, matching, false)
    }

    /// The analysis of `a`, complex symmetric (a = aᵀ to the last bit), for its L D Lᵀ: the
    /// structure's half and the factors' half of an LU's. `None` if `a` isn't symmetric, or if
    /// its maximum-product matching moves a row (its diagonal then isn't the place for its
    /// pivots, and the LU of [`Analysis::new`] is the factorization to use). The scaling is the
    /// matching's made symmetric, √(r_i c_i) for row and column i (as Duff and Pralet's
    /// symmetrized MC64), a power of two.
    ///
    /// # Errors
    ///
    /// As [`Analysis::new`].
    pub(crate) fn new_symmetric(
        a: SparseColMatRef<'_, usize, c64>,
        positions: Option<&[[f64; 3]]>,
    ) -> Result<Option<Analysis>> {
        let n = a.ncols();
        if a.nrows() != n {
            return Err(failed(
                "the matrix",
                format!("{} × {n}, not square", a.nrows()),
            ));
        }
        let columns = columns_of(a);
        // symmetric: (i, j) and (j, i) the same value
        for (j, column) in columns.iter().enumerate() {
            for &(i, v) in column {
                let mirror = columns[i]
                    .binary_search_by_key(&j, |&(r, _)| r)
                    .ok()
                    .map(|k| columns[i][k].1);
                if mirror != Some(v) {
                    return Ok(None);
                }
            }
        }
        let mut matching = matching(n, &columns)?;
        if matching.rows.iter().enumerate().any(|(j, &r)| j != r) {
            return Ok(None);
        }
        let scale: Vec<f64> = (0..n)
            .map(|i| {
                let s = (matching.row_scale[i] * matching.column_scale[i]).sqrt();
                2f64.powi(s.log2().round() as i32)
            })
            .collect();
        matching.row_scale.clone_from(&scale);
        matching.column_scale = scale;
        Self::with(a, positions, matching, true).map(Some)
    }

    fn with(
        a: SparseColMatRef<'_, usize, c64>,
        positions: Option<&[[f64; 3]]>,
        matching: Matching,
        symmetric: bool,
    ) -> Result<Analysis> {
        let n = a.ncols();
        // the matched matrix's structure: old row rows[j] becomes row j
        let mut new_row = vec![0usize; n];
        for (j, &r) in matching.rows.iter().enumerate() {
            new_row[r] = j;
        }
        let pairs: Vec<(usize, usize)> = (0..n)
            .flat_map(|j| a.row_idx_of_col(j).map(move |i| (i, j)))
            .map(|(i, j)| (new_row[i], j))
            .collect();
        // A + Aᵀ's structure, with its diagonal, for faer's analysis
        let pattern = {
            let mut t: Vec<Triplet<usize, usize, f64>> = Vec::with_capacity(2 * pairs.len() + n);
            for &(i, j) in &pairs {
                t.push(Triplet::new(i, j, 1.0));
                t.push(Triplet::new(j, i, 1.0));
            }
            for j in 0..n {
                t.push(Triplet::new(j, j, 1.0));
            }
            SparseColMat::<usize, f64>::try_new_from_triplets(n, n, &t)
                .map_err(|e| failed("the structure", format!("{e:?}")))?
        };
        let params = CholeskySymbolicParams {
            supernodal_flop_ratio_threshold: SupernodalThreshold::FORCE_SUPERNODAL,
            ..Default::default()
        };
        let symbolic = match positions {
            Some(positions) => {
                let (starts, neighbours) = adjacency(n, &pairs);
                let order = nested_dissection(&starts, &neighbours, positions);
                let mut inverse = vec![0usize; n];
                for (k, &j) in order.iter().enumerate() {
                    inverse[j] = k;
                }
                factorize_symbolic_cholesky(
                    pattern.symbolic(),
                    Side::Upper,
                    SymmetricOrdering::Custom(PermRef::new_checked(&order, &inverse, n)),
                    params,
                )
            }
            None => factorize_symbolic_cholesky(
                pattern.symbolic(),
                Side::Upper,
                SymmetricOrdering::Amd,
                params,
            ),
        }
        .map_err(|e| failed("the analysis", format!("{e:?}")))?;
        let (order, step) = match symbolic.perm() {
            Some(p) => {
                let (f, i) = p.arrays();
                (f.to_vec(), i.to_vec())
            }
            None => ((0..n).collect(), (0..n).collect()),
        };
        let SymbolicCholeskyRaw::Supernodal(supernodal) = symbolic.raw() else {
            return Err(failed("the analysis", "faer chose a simplicial structure"));
        };
        let count = supernodal.n_supernodes();
        let mut fronts: Vec<Front> = (0..count)
            .map(|s| {
                let begin = supernodal.supernode_begin()[s];
                let end = supernodal.supernode_end()[s];
                let below = supernodal.supernode(s).pattern().to_vec();
                Front {
                    begin,
                    end,
                    below,
                    children: Vec::new(),
                    work: 0.0,
                }
            })
            .collect();
        // each column's supernode, then each supernode's parent: the one holding the first row
        // its factor reaches
        let mut owner = vec![0usize; n];
        for (s, f) in fronts.iter().enumerate() {
            owner[f.begin..f.end].fill(s);
        }
        let mut roots = Vec::new();
        for s in 0..count {
            match fronts[s].below.first() {
                Some(&r) => {
                    let parent = owner[r];
                    fronts[parent].children.push(s);
                }
                None => roots.push(s),
            }
        }
        // children come before their parents: each subtree's work, bottom up
        for s in 0..count {
            let k = (fronts[s].end - fronts[s].begin) as f64;
            let m = k + fronts[s].below.len() as f64;
            let own = 8.0 * k * m * m;
            let below: f64 = fronts[s].children.iter().map(|&c| fronts[c].work).sum();
            fronts[s].work = own + below;
        }
        // L and U: each front's diagonal block and its two panels; L D Lᵀ: the block's lower
        // half and one panel
        let entries = fronts
            .iter()
            .map(|f| {
                let k = f.end - f.begin;
                if symmetric {
                    k * (k + 1) / 2 + k * f.below.len()
                } else {
                    k * k + 2 * k * f.below.len()
                }
            })
            .sum();
        Ok(Analysis {
            n,
            matching,
            order,
            step,
            fronts,
            roots,
            entries,
            symmetric,
        })
    }

    /// The unknowns of the analysed matrix.
    pub(crate) fn n(&self) -> usize {
        self.n
    }

    /// Whether the factorization is L D Lᵀ ([`Analysis::new_symmetric`]).
    pub(crate) fn symmetric(&self) -> bool {
        self.symmetric
    }

    /// The factors' entries: L and U, each front's diagonal block and its L and U panels; or
    /// L D Lᵀ, the block's lower half and its L panel.
    pub(crate) fn factor_entries(&self) -> usize {
        self.entries
    }
}

/// A front's factors: the diagonal block's L (unit, strictly below) and U (on and above) with
/// L's panel below them, U's panel beside them, and the row order partial pivoting chose
/// inside the block. L D Lᵀ: the block's L (unit, strictly below) and D (on the diagonal) with
/// L's panel below, and no U.
#[derive(Debug)]
struct Factors {
    /// m × k: [L₁₁\U₁₁; L₂₁], or [L₁₁\D; L₂₁].
    lower: Mat<c64>,
    /// k × (m − k): U₁₂ (empty for L D Lᵀ).
    upper: Mat<c64>,
    /// Row i of the pivoted block is the front's row `rows[i]` (0..k); empty for L D Lᵀ.
    rows: Vec<usize>,
}

/// The LU factorization of [`Analysis`]'s matrix: B = P Pr Dr A Dc Pᵀ = L U, P the elimination
/// order, Pr the matching, Dr and Dc its scaling.
#[derive(Debug)]
pub(crate) struct Multifrontal {
    analysis: Arc<Analysis>,
    factors: Vec<Factors>,
    /// Pivots perturbed to √ε ‖B‖₁: 0 on a matrix the matching made safe.
    perturbed: usize,
}

/// A front's Schur complement, passed to its parent: rows and columns `below`.
struct Update {
    values: Mat<c64>,
}

/// Fronts with more work than this have their children factorized side by side, and their own
/// dense work threaded.
const PARALLEL_WORK: f64 = 2e7;

impl Multifrontal {
    /// Factorizes `a`, whose structure (and roughly whose values) `analysis` analysed.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if `a` isn't the analysed size or holds an entry outside the
    /// analysed structure.
    pub(crate) fn new(
        analysis: Arc<Analysis>,
        a: SparseColMatRef<'_, usize, c64>,
    ) -> Result<Multifrontal> {
        let n = analysis.n;
        if a.nrows() != n || a.ncols() != n {
            return Err(failed(
                "the matrix",
                format!("{} × {}, analysed {n} × {n}", a.nrows(), a.ncols()),
            ));
        }
        // B's entries, by B's columns and by B's rows
        let m = &analysis.matching;
        let mut new_row = vec![0usize; n];
        for (j, &r) in m.rows.iter().enumerate() {
            new_row[r] = j;
        }
        let mut by_column: Vec<Vec<(usize, c64)>> = vec![Vec::new(); n];
        let mut by_row: Vec<Vec<(usize, c64)>> = vec![Vec::new(); n];
        let mut norm = vec![0.0f64; n];
        for j in 0..n {
            for (i, &v) in a.row_idx_of_col(j).zip(a.val_of_col(j)) {
                let value = v * (m.row_scale[i] * m.column_scale[j]);
                let (r, c) = (analysis.step[new_row[i]], analysis.step[j]);
                norm[c] += value.norm();
                if r >= c {
                    by_column[c].push((r, value));
                } else if !analysis.symmetric {
                    // L D Lᵀ reads the lower half only: this entry's mirror is in it
                    by_row[r].push((c, value));
                }
            }
        }
        let tiny = f64::EPSILON.sqrt() * norm.iter().copied().fold(0.0, f64::max);
        let slots: Vec<OnceLock<(Factors, usize)>> = (0..analysis.fronts.len())
            .map(|_| OnceLock::new())
            .collect();
        let context = Context {
            analysis: &analysis,
            by_column: &by_column,
            by_row: &by_row,
            tiny,
            slots: &slots,
        };
        let roots = &analysis.roots;
        let run = || -> Result<()> {
            roots
                .par_iter()
                .map(|&s| context.subtree(s).map(|_| ()))
                .collect::<Result<Vec<()>>>()
                .map(|_| ())
        };
        run()?;
        let mut perturbed = 0;
        let factors = slots
            .into_iter()
            .map(|slot| {
                let (f, p) = slot.into_inner().expect("every front factorized");
                perturbed += p;
                f
            })
            .collect();
        Ok(Multifrontal {
            analysis,
            factors,
            perturbed,
        })
    }

    /// The analysis, for another matrix of the same structure.
    pub(crate) fn analysis(&self) -> &Arc<Analysis> {
        &self.analysis
    }

    /// The pivots set to √ε ‖B‖₁.
    pub(crate) fn perturbed(&self) -> usize {
        self.perturbed
    }

    /// x with A x = `b`.
    pub(crate) fn solve(&self, b: &[c64]) -> Vec<c64> {
        if self.analysis.symmetric {
            return self.solve_symmetric(b);
        }
        let a = &self.analysis;
        let m = &a.matching;
        // B (P Dc⁻¹ x) = P Pr Dr b
        let mut w: Vec<c64> = a
            .order
            .iter()
            .map(|&j| {
                let r = m.rows[j];
                b[r] * m.row_scale[r]
            })
            .collect();
        for (f, s) in a.fronts.iter().zip(&self.factors) {
            let k = f.end - f.begin;
            let mut x = Mat::<c64>::from_fn(k, 1, |i, _| w[f.begin + s.rows[i]]);
            solve_unit_lower_triangular_in_place(
                s.lower.as_ref().subrows(0, k),
                x.as_mut(),
                Par::Seq,
            );
            for (t, &r) in f.below.iter().enumerate() {
                let row = s.lower.as_ref().row(k + t);
                let mut sum = c64::new(0.0, 0.0);
                for i in 0..k {
                    sum += row[i] * x[(i, 0)];
                }
                w[r] -= sum;
            }
            for i in 0..k {
                w[f.begin + i] = x[(i, 0)];
            }
        }
        for (f, s) in a.fronts.iter().zip(&self.factors).rev() {
            let k = f.end - f.begin;
            let mut x = Mat::<c64>::from_fn(k, 1, |i, _| w[f.begin + i]);
            for i in 0..k {
                let row = s.upper.as_ref().row(i);
                let mut sum = c64::new(0.0, 0.0);
                for (t, &r) in f.below.iter().enumerate() {
                    sum += row[t] * w[r];
                }
                x[(i, 0)] -= sum;
            }
            solve_upper_triangular_in_place(s.lower.as_ref().subrows(0, k), x.as_mut(), Par::Seq);
            for i in 0..k {
                w[f.begin + i] = x[(i, 0)];
            }
        }
        let mut x = vec![c64::new(0.0, 0.0); a.n];
        for (k, &j) in a.order.iter().enumerate() {
            x[j] = w[k] * m.column_scale[j];
        }
        x
    }

    /// x with Aᵀ x = `b`.
    pub(crate) fn solve_transpose(&self, b: &[c64]) -> Vec<c64> {
        if self.analysis.symmetric {
            return self.solve_symmetric(b);
        }
        let a = &self.analysis;
        let m = &a.matching;
        // Bᵀ (P Pr Dr⁻¹ x) = P Dc b
        let mut w: Vec<c64> = a.order.iter().map(|&j| b[j] * m.column_scale[j]).collect();
        for (f, s) in a.fronts.iter().zip(&self.factors) {
            let k = f.end - f.begin;
            let mut x = Mat::<c64>::from_fn(k, 1, |i, _| w[f.begin + i]);
            solve_lower_triangular_in_place(
                s.lower.as_ref().subrows(0, k).transpose(),
                x.as_mut(),
                Par::Seq,
            );
            for (t, &r) in f.below.iter().enumerate() {
                let mut sum = c64::new(0.0, 0.0);
                for i in 0..k {
                    sum += s.upper[(i, t)] * x[(i, 0)];
                }
                w[r] -= sum;
            }
            for i in 0..k {
                w[f.begin + i] = x[(i, 0)];
            }
        }
        for (f, s) in a.fronts.iter().zip(&self.factors).rev() {
            let k = f.end - f.begin;
            let mut x = Mat::<c64>::from_fn(k, 1, |i, _| w[f.begin + i]);
            for i in 0..k {
                let mut sum = c64::new(0.0, 0.0);
                for (t, &r) in f.below.iter().enumerate() {
                    sum += s.lower[(k + t, i)] * w[r];
                }
                x[(i, 0)] -= sum;
            }
            solve_unit_upper_triangular_in_place(
                s.lower.as_ref().subrows(0, k).transpose(),
                x.as_mut(),
                Par::Seq,
            );
            for i in 0..k {
                w[f.begin + s.rows[i]] = x[(i, 0)];
            }
        }
        let mut x = vec![c64::new(0.0, 0.0); a.n];
        for (k, &j) in a.order.iter().enumerate() {
            let r = m.rows[j];
            x[r] = w[k] * m.row_scale[r];
        }
        x
    }
}

impl Multifrontal {
    /// x with A x = `b` (= Aᵀ x) for L D Lᵀ, A symmetric: B (P D⁻¹ x) = P D b, B = P D A D Pᵀ.
    fn solve_symmetric(&self, b: &[c64]) -> Vec<c64> {
        let a = &self.analysis;
        let scale = &a.matching.row_scale;
        let mut w: Vec<c64> = a.order.iter().map(|&j| b[j] * scale[j]).collect();
        // L and D: each front's block, then its panel's rows below
        for (f, s) in a.fronts.iter().zip(&self.factors) {
            let k = f.end - f.begin;
            let mut x = Mat::<c64>::from_fn(k, 1, |i, _| w[f.begin + i]);
            solve_unit_lower_triangular_in_place(
                s.lower.as_ref().subrows(0, k),
                x.as_mut(),
                Par::Seq,
            );
            for (t, &r) in f.below.iter().enumerate() {
                let mut sum = c64::new(0.0, 0.0);
                for i in 0..k {
                    sum += s.lower[(k + t, i)] * x[(i, 0)];
                }
                w[r] -= sum;
            }
            for i in 0..k {
                w[f.begin + i] = x[(i, 0)] / s.lower[(i, i)];
            }
        }
        // Lᵀ, the other way round
        for (f, s) in a.fronts.iter().zip(&self.factors).rev() {
            let k = f.end - f.begin;
            let mut x = Mat::<c64>::from_fn(k, 1, |i, _| w[f.begin + i]);
            for i in 0..k {
                let mut sum = c64::new(0.0, 0.0);
                for (t, &r) in f.below.iter().enumerate() {
                    sum += s.lower[(k + t, i)] * w[r];
                }
                x[(i, 0)] -= sum;
            }
            solve_unit_upper_triangular_in_place(
                s.lower.as_ref().subrows(0, k).transpose(),
                x.as_mut(),
                Par::Seq,
            );
            for i in 0..k {
                w[f.begin + i] = x[(i, 0)];
            }
        }
        let mut x = vec![c64::new(0.0, 0.0); a.n];
        for (k, &j) in a.order.iter().enumerate() {
            x[j] = w[k] * scale[j];
        }
        x
    }
}

/// What every front's factorization reads.
struct Context<'a> {
    analysis: &'a Analysis,
    by_column: &'a [Vec<(usize, c64)>],
    by_row: &'a [Vec<(usize, c64)>],
    tiny: f64,
    slots: &'a [OnceLock<(Factors, usize)>],
}

impl Context<'_> {
    /// Factorizes the subtree under front `s`, its factors into their slots, and returns its
    /// update.
    fn subtree(&self, s: usize) -> Result<Update> {
        let front = &self.analysis.fronts[s];
        let parallel = front.work > PARALLEL_WORK && rayon::current_num_threads() > 1;
        let updates: Vec<Update> = if parallel {
            front
                .children
                .par_iter()
                .map(|&c| self.subtree(c))
                .collect::<Result<_>>()?
        } else {
            // a sequential subtree, depth first without recursion: deep trees don't overflow
            let mut done: Vec<Update> = Vec::with_capacity(front.children.len());
            for &c in &front.children {
                done.push(self.sequential(c)?);
            }
            done
        };
        self.factorize(s, updates, parallel)
    }

    /// The subtree under `s` on this thread, in postorder with an explicit stack.
    fn sequential(&self, s: usize) -> Result<Update> {
        let fronts = &self.analysis.fronts;
        // (front, its children's updates so far)
        let mut stack: Vec<(usize, Vec<Update>)> = vec![(s, Vec::new())];
        loop {
            let (top, got) = {
                let (t, u) = stack
                    .last()
                    .expect("the stack holds the root until it returns");
                (*t, u.len())
            };
            if got < fronts[top].children.len() {
                let child = fronts[top].children[got];
                stack.push((child, Vec::new()));
                continue;
            }
            let (top, updates) = stack.pop().expect("just looked at it");
            let update = self.factorize(top, updates, false)?;
            match stack.last_mut() {
                Some((_, siblings)) => siblings.push(update),
                None => return Ok(update),
            }
        }
    }

    /// Front `s`: assembled from B's entries and its children's updates, its columns
    /// eliminated, its factors stored, its update returned.
    fn factorize(&self, s: usize, updates: Vec<Update>, parallel: bool) -> Result<Update> {
        let front = &self.analysis.fronts[s];
        let fronts = &self.analysis.fronts;
        let (begin, end) = (front.begin, front.end);
        let k = end - begin;
        let p = front.below.len();
        let m = k + p;
        let par = if parallel { Par::rayon(0) } else { Par::Seq };
        let local = |r: usize| -> usize {
            if r < end {
                r - begin
            } else {
                k + front
                    .below
                    .binary_search(&r)
                    .expect("B's structure is inside the analysed one")
            }
        };
        let mut f = Mat::<c64>::zeros(m, m);
        for c in begin..end {
            for &(r, v) in &self.by_column[c] {
                f[(local(r), c - begin)] += v;
            }
            for &(col, v) in &self.by_row[c] {
                f[(c - begin, local(col))] += v;
            }
        }
        // the children's updates, in their order: their rows are among this front's
        for (&child, update) in front.children.iter().zip(updates) {
            let rows = &fronts[child].below;
            let mut at = Vec::with_capacity(rows.len());
            let mut t = 0;
            for &r in rows {
                if r < end {
                    at.push(r - begin);
                } else {
                    while front.below[t] < r {
                        t += 1;
                    }
                    at.push(k + t);
                }
            }
            // L D Lᵀ's updates are their lower halves
            let symmetric = self.analysis.symmetric;
            for (cj, &j) in at.iter().enumerate() {
                let from = if symmetric { cj } else { 0 };
                for (ci, &i) in at.iter().enumerate().skip(from) {
                    f[(i, j)] += update.values[(ci, cj)];
                }
            }
        }
        if self.analysis.symmetric {
            let perturbed = eliminate_symmetric(f.as_mut(), k, self.tiny, par);
            let lower = f.as_ref().submatrix(0, 0, m, k).to_owned();
            let values = f.as_ref().submatrix(k, k, p, p).to_owned();
            self.slots[s]
                .set((
                    Factors {
                        lower,
                        upper: Mat::new(),
                        rows: Vec::new(),
                    },
                    perturbed,
                ))
                .map_err(|_| failed("a front", format!("{s} factorized twice")))?;
            return Ok(Update { values });
        }
        // the diagonal block, pivoting among its own rows, small pivots perturbed
        let (rows, perturbed) = pivot_block(f.as_mut().submatrix_mut(0, 0, k, k), self.tiny, par);
        let permuted = Mat::<c64>::from_fn(k, p, |i, j| f[(rows[i], k + j)]);
        f.as_mut().submatrix_mut(0, k, k, p).copy_from(&permuted);
        let (top, bottom) = f.as_mut().split_at_row_mut(k);
        let (l11u11, mut u12) = top.split_at_col_mut(k);
        let (mut l21, mut f22) = bottom.split_at_col_mut(k);
        // U₁₂ = L₁₁⁻¹ F₁₂, L₂₁ = F₂₁ U₁₁⁻¹, S = F₂₂ − L₂₁ U₁₂
        solve_unit_lower_triangular_in_place(l11u11.as_ref(), u12.as_mut(), par);
        solve_lower_triangular_in_place(
            l11u11.as_ref().transpose(),
            l21.as_mut().transpose_mut(),
            par,
        );
        matmul(
            f22.as_mut(),
            Accum::Add,
            l21.as_ref(),
            u12.as_ref(),
            c64::new(-1.0, 0.0),
            par,
        );
        let lower = f.as_ref().submatrix(0, 0, m, k).to_owned();
        let upper = f.as_ref().submatrix(0, k, k, p).to_owned();
        let values = f.as_ref().submatrix(k, k, p, p).to_owned();
        self.slots[s]
            .set((Factors { lower, upper, rows }, perturbed))
            .map_err(|_| failed("a front", format!("{s} factorized twice")))?;
        Ok(Update { values })
    }
}

/// The columns of a panel of L D Lᵀ factorized together: the diagonal block one column after
/// another, the rows below by a triangular solve, the rest by products.
const PANEL: usize = 64;

/// L D Lᵀ of the front `f` (m × m, complex symmetric, its lower half read and written) without
/// pivoting, its first k columns eliminated: L and D in place of the first k columns (D on the
/// diagonal, L unit and below it), the Schur complement's lower half in place of the rest. A
/// pivot smaller than `tiny` is set to it (with its own phase, or 1's); returns how many were.
fn eliminate_symmetric(mut f: MatMut<'_, c64>, k: usize, tiny: f64, par: Par) -> usize {
    use faer::linalg::matmul::triangular::{BlockStructure, matmul as triangular};
    let m = f.nrows();
    let minus = c64::new(-1.0, 0.0);
    let mut perturbed = 0;
    let mut j0 = 0;
    while j0 < k {
        let j1 = (j0 + PANEL).min(k);
        // the panel's diagonal block, one column after another
        for j in j0..j1 {
            let mut d = f[(j, j)];
            if d.norm() < tiny {
                d = if d.norm() > 0.0 {
                    d / d.norm() * tiny
                } else {
                    c64::new(tiny, 0.0)
                };
                f[(j, j)] = d;
                perturbed += 1;
            }
            for r in j + 1..j1 {
                f[(r, j)] /= d;
            }
            for c in j + 1..j1 {
                let w = d * f[(c, j)];
                for r in c..j1 {
                    let l = f[(r, j)];
                    f[(r, c)] -= l * w;
                }
            }
        }
        // the panel's rows below: F = L D Lᵀ there, so L D = F L⁻ᵀ, then L = (L D) D⁻¹
        let rows = m - j1;
        let width = j1 - j0;
        let (diagonal, mut below) = f
            .as_mut()
            .submatrix_mut(j0, j0, m - j0, width)
            .split_at_row_mut(width);
        solve_unit_lower_triangular_in_place(
            diagonal.as_ref(),
            below.as_mut().transpose_mut(),
            par,
        );
        let ld = below.as_ref().to_owned();
        for c in 0..width {
            let d = diagonal[(c, c)];
            for r in 0..rows {
                below[(r, c)] /= d;
            }
        }
        // the block's remaining columns (rows j1..m), minus the panel's part: L D Lᵀ
        if j1 < k {
            let (left, mut right) = f.as_mut().split_at_col_mut(j1);
            let l = left.as_ref().submatrix(j1, j0, k - j1, width);
            triangular(
                right.as_mut().submatrix_mut(j1, 0, k - j1, k - j1),
                BlockStructure::TriangularLower,
                Accum::Add,
                ld.as_ref().subrows(0, k - j1),
                BlockStructure::Rectangular,
                l.transpose(),
                BlockStructure::Rectangular,
                minus,
                par,
            );
            matmul(
                right.as_mut().submatrix_mut(k, 0, m - k, k - j1),
                Accum::Add,
                ld.as_ref().subrows(k - j1, m - k),
                l.transpose(),
                minus,
                par,
            );
        }
        j0 = j1;
    }
    // the Schur complement's lower half: F₂₂ − L₂₁ D L₂₁ᵀ
    let p = m - k;
    if p > 0 {
        let ld = Mat::<c64>::from_fn(p, k, |r, c| f[(k + r, c)] * f[(c, c)]);
        let (left, mut right) = f.as_mut().split_at_col_mut(k);
        triangular(
            right.as_mut().submatrix_mut(k, 0, p, p),
            BlockStructure::TriangularLower,
            Accum::Add,
            ld.as_ref(),
            BlockStructure::Rectangular,
            left.as_ref().submatrix(k, 0, p, k).transpose(),
            BlockStructure::Rectangular,
            minus,
            par,
        );
    }
    perturbed
}

/// LU with partial pivoting of the k × k block in place, rows swapped only among its own: the
/// block's rows in their new order, and how many pivots were smaller than `tiny` and set to it
/// (with their own phase, or 1's).
fn pivot_block(mut a: MatMut<'_, c64>, tiny: f64, par: Par) -> (Vec<usize>, usize) {
    let k = a.nrows();
    let backup = a.to_owned();
    let mut forward = vec![0usize; k];
    let mut inverse = vec![0usize; k];
    let mut stack = MemBuffer::new(
        faer::linalg::lu::partial_pivoting::factor::lu_in_place_scratch::<usize, c64>(
            k,
            k,
            par,
            Default::default(),
        ),
    );
    faer::linalg::lu::partial_pivoting::factor::lu_in_place(
        a.as_mut(),
        &mut forward,
        &mut inverse,
        par,
        MemStack::new(&mut stack),
        Default::default(),
    );
    if (0..k).all(|i| a[(i, i)].norm() >= tiny) {
        return (forward, 0);
    }
    // a small pivot: again, unblocked, perturbing the small ones
    a.copy_from(&backup);
    let mut rows: Vec<usize> = (0..k).collect();
    let mut perturbed = 0;
    for j in 0..k {
        let (best, size) = (j..k)
            .map(|i| (i, a[(i, j)].norm()))
            .fold((j, -1.0), |b, x| if x.1 > b.1 { x } else { b });
        if best != j {
            for c in 0..k {
                let t = a[(j, c)];
                a[(j, c)] = a[(best, c)];
                a[(best, c)] = t;
            }
            rows.swap(j, best);
        }
        if size < tiny {
            let phase = if size > 0.0 {
                a[(j, j)] / size
            } else {
                c64::new(1.0, 0.0)
            };
            a[(j, j)] = phase * tiny;
            perturbed += 1;
        }
        let pivot = a[(j, j)];
        for i in j + 1..k {
            let l = a[(i, j)] / pivot;
            a[(i, j)] = l;
            if l != c64::new(0.0, 0.0) {
                for c in j + 1..k {
                    let u = a[(j, c)];
                    a[(i, c)] -= l * u;
                }
            }
        }
    }
    (rows, perturbed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_threaded_product_of_a_fronts_update_is_the_plain_one() {
        // S = F₂₂ − L₂₁ U₁₂ in the shapes of a 3D solve's last fronts, by faer's threaded
        // product as the fronts take it, against the sum written out. faer's x86 kernel
        // (private-gemm-x86 0.1.20) got 140 of 369² entries wrong, by as much as the entries
        // themselves, on GitHub's Windows runners with an AMD EPYC 7763, and only threaded:
        // 3D direct solves there were wrong (a residual of 0.02). 0.1.22 is right, and
        // Cargo.toml asks for it
        let random = |m: usize, n: usize, seed: u64| {
            let mut s = seed;
            let mut next = move || {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((s >> 11) as f64) / ((1u64 << 53) as f64) - 0.5
            };
            Mat::from_fn(m, n, |_, _| c64::new(next(), next()))
        };
        for (k, p) in [(145, 369), (140, 417), (37, 150)] {
            let f0 = random(k + p, k + p, (1000 * k + p) as u64);
            for par in [Par::Seq, Par::rayon(0), Par::rayon(2), Par::rayon(4)] {
                let mut f = f0.clone();
                let (top, bottom) = f.as_mut().split_at_row_mut(k);
                let (_, u12) = top.split_at_col_mut(k);
                let (l21, mut f22) = bottom.split_at_col_mut(k);
                matmul(
                    f22.as_mut(),
                    Accum::Add,
                    l21.as_ref(),
                    u12.as_ref(),
                    c64::new(-1.0, 0.0),
                    par,
                );
                let mut worst: f64 = 0.0;
                for j in 0..p {
                    for i in 0..p {
                        let sum: c64 = (0..k).map(|t| f0[(k + i, t)] * f0[(t, k + j)]).sum();
                        let want = f0[(k + i, k + j)] - sum;
                        worst = worst.max((f[(k + i, k + j)] - want).norm());
                    }
                }
                assert!(worst < 1e-12, "k {k}, p {p}, {par:?}: {worst}");
            }
        }
    }

    fn residual(a: SparseColMatRef<'_, usize, c64>, x: &[c64], b: &[c64], transpose: bool) -> f64 {
        let n = b.len();
        let mut r = b.to_vec();
        for j in 0..n {
            for (i, &v) in a.row_idx_of_col(j).zip(a.val_of_col(j)) {
                if transpose {
                    r[j] -= v * x[i];
                } else {
                    r[i] -= v * x[j];
                }
            }
        }
        let norm = |v: &[c64]| v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
        norm(&r) / norm(b)
    }

    /// A shifted 5-point Laplacian on an nx × ny grid, complex, unsymmetric in its values, with
    /// its unknowns' positions.
    fn laplacian(nx: usize, ny: usize) -> (SparseColMat<usize, c64>, Vec<[f64; 3]>) {
        let n = nx * ny;
        let mut t = Vec::new();
        let mut positions = vec![[0.0; 3]; n];
        for j in 0..ny {
            for i in 0..nx {
                let k = j * nx + i;
                positions[k] = [i as f64, j as f64, 0.0];
                t.push(Triplet::new(k, k, c64::new(-3.6, 0.2)));
                if i + 1 < nx {
                    t.push(Triplet::new(k, k + 1, c64::new(1.0, 0.1)));
                    t.push(Triplet::new(k + 1, k, c64::new(1.0, -0.2)));
                }
                if j + 1 < ny {
                    t.push(Triplet::new(k, k + nx, c64::new(1.0, 0.0)));
                    t.push(Triplet::new(k + nx, k, c64::new(1.0, 0.05)));
                }
            }
        }
        (
            SparseColMat::try_new_from_triplets(n, n, &t).unwrap(),
            positions,
        )
    }

    #[test]
    fn it_solves_and_transposes_by_nested_dissection_or_amd() {
        let (a, positions) = laplacian(37, 23);
        let n = a.ncols();
        let b: Vec<c64> = (0..n)
            .map(|i| c64::new((i % 7) as f64 - 3.0, 1.0))
            .collect();
        for positions in [Some(&positions[..]), None] {
            let analysis = Arc::new(Analysis::new(a.as_ref(), positions).unwrap());
            let lu = Multifrontal::new(analysis, a.as_ref()).unwrap();
            assert_eq!(lu.perturbed(), 0);
            let x = lu.solve(&b);
            assert!(residual(a.as_ref(), &x, &b, false) < 1e-13);
            let y = lu.solve_transpose(&b);
            assert!(residual(a.as_ref(), &y, &b, true) < 1e-13);
        }
    }

    #[test]
    fn rows_permuted_away_from_the_diagonal_are_matched_back() {
        // the Laplacian with its rows reversed in pairs: zeros on the diagonal, which the
        // matching undoes
        let (a, positions) = laplacian(20, 15);
        let n = a.ncols();
        let swap = |i: usize| {
            if i.is_multiple_of(2) && i + 1 < n {
                i + 1
            } else if i % 2 == 1 {
                i - 1
            } else {
                i
            }
        };
        let t: Vec<Triplet<usize, usize, c64>> = (0..n)
            .flat_map(|j| {
                a.row_idx_of_col(j)
                    .zip(a.val_of_col(j))
                    .map(move |(i, &v)| Triplet::new(swap(i), j, v))
                    .collect::<Vec<_>>()
            })
            .collect();
        let a = SparseColMat::<usize, c64>::try_new_from_triplets(n, n, &t).unwrap();
        let analysis = Arc::new(Analysis::new(a.as_ref(), Some(&positions)).unwrap());
        let lu = Multifrontal::new(analysis, a.as_ref()).unwrap();
        let b: Vec<c64> = (0..n).map(|i| c64::new(1.0, (i % 3) as f64)).collect();
        assert!(residual(a.as_ref(), &lu.solve(&b), &b, false) < 1e-13);
        assert!(residual(a.as_ref(), &lu.solve_transpose(&b), &b, true) < 1e-13);
    }

    #[test]
    fn the_factors_are_the_same_bits_on_any_number_of_threads() {
        let (a, positions) = laplacian(120, 90);
        let n = a.ncols();
        let b: Vec<c64> = (0..n).map(|i| c64::new((i % 5) as f64, -1.0)).collect();
        let analysis = Arc::new(Analysis::new(a.as_ref(), Some(&positions)).unwrap());
        let run = |threads: usize| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| {
                    let lu = Multifrontal::new(analysis.clone(), a.as_ref()).unwrap();
                    lu.solve(&b)
                        .iter()
                        .map(|z| (z.re.to_bits(), z.im.to_bits()))
                        .collect::<Vec<_>>()
                })
        };
        let one = run(1);
        for threads in [2, 4, 5, 20] {
            assert!(run(threads) == one, "{threads} threads differ from one");
        }
    }

    #[test]
    fn a_zero_pivot_is_perturbed_and_counted() {
        // a singular 2 × 2 block, [[1, 1], [1, 1]]: no permutation of its rows helps, and its
        // second pivot is zero
        let t = vec![
            Triplet::new(0, 0, c64::new(1.0, 0.0)),
            Triplet::new(0, 1, c64::new(1.0, 0.0)),
            Triplet::new(1, 0, c64::new(1.0, 0.0)),
            Triplet::new(1, 1, c64::new(1.0, 0.0)),
            Triplet::new(2, 2, c64::new(2.0, 0.0)),
        ];
        let a = SparseColMat::<usize, c64>::try_new_from_triplets(3, 3, &t).unwrap();
        let analysis = Arc::new(Analysis::new(a.as_ref(), None).unwrap());
        let lu = Multifrontal::new(analysis, a.as_ref()).unwrap();
        assert_eq!(lu.perturbed(), 1);
        assert!(
            lu.solve(&[c64::new(0.0, 0.0); 3])
                .iter()
                .all(|z| z.is_finite())
        );
    }

    /// The shifted Laplacian with symmetric complex values: B = Bᵀ, not Hermitian.
    fn symmetric_laplacian(nx: usize, ny: usize) -> (SparseColMat<usize, c64>, Vec<[f64; 3]>) {
        let (a, positions) = laplacian(nx, ny);
        let n = a.ncols();
        let mut t = Vec::new();
        for j in 0..n {
            for (i, &v) in a.row_idx_of_col(j).zip(a.val_of_col(j)) {
                if i >= j {
                    let v = if i == j {
                        v
                    } else {
                        c64::new(1.0, 0.1 * ((i + j) % 3) as f64)
                    };
                    t.push(Triplet::new(i, j, v));
                    if i != j {
                        t.push(Triplet::new(j, i, v));
                    }
                }
            }
        }
        (
            SparseColMat::try_new_from_triplets(n, n, &t).unwrap(),
            positions,
        )
    }

    #[test]
    fn a_complex_symmetric_matrix_is_factorized_as_l_d_lt_in_half_the_entries() {
        let (a, positions) = symmetric_laplacian(37, 23);
        let n = a.ncols();
        let b: Vec<c64> = (0..n)
            .map(|i| c64::new((i % 7) as f64 - 3.0, 1.0))
            .collect();
        let general = Analysis::new(a.as_ref(), Some(&positions)).unwrap();
        let symmetric = Analysis::new_symmetric(a.as_ref(), Some(&positions))
            .unwrap()
            .expect("symmetric, its diagonal matched to itself");
        assert!(symmetric.symmetric() && !general.symmetric());
        // the same order and fronts: the block's lower half and one panel, against both
        let (s, g) = (symmetric.factor_entries(), general.factor_entries());
        assert!(2 * s > g && 2 * s < g + g / 10, "{s} {g}");
        let lu = Multifrontal::new(Arc::new(symmetric), a.as_ref()).unwrap();
        assert_eq!(lu.perturbed(), 0);
        assert!(residual(a.as_ref(), &lu.solve(&b), &b, false) < 1e-13);
        assert!(residual(a.as_ref(), &lu.solve_transpose(&b), &b, true) < 1e-13);
        // an unsymmetric matrix is left to the LU
        let (unsymmetric, _) = laplacian(10, 10);
        assert!(
            Analysis::new_symmetric(unsymmetric.as_ref(), None)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn nested_dissection_stores_k2_log_k_on_a_k_by_k_grid_as_george_proved() {
        // George 1973, Section 4: on a k × k grid nested dissection's factors take O(k² log k)
        // storage, against O(k³) numbered row by row (the band). Doubling k then multiplies the
        // entries by 4 log(2k) / log k, about 4.7 here, and the band's by 8.
        let entries = |k: usize| {
            let (a, positions) = laplacian(k, k);
            Analysis::new(a.as_ref(), Some(&positions))
                .unwrap()
                .factor_entries() as f64
        };
        let (e32, e64, e128) = (entries(32), entries(64), entries(128));
        for ratio in [e64 / e32, e128 / e64] {
            assert!((4.0..5.5).contains(&ratio), "{e32} {e64} {e128}");
        }
        // numbered row by row instead (every position the same, so nothing is cut): the band,
        // which grows about 8 times
        let band = |k: usize| {
            let (a, _) = laplacian(k, k);
            let positions: Vec<[f64; 3]> = vec![[0.0; 3]; k * k];
            Analysis::new(a.as_ref(), Some(&positions))
                .unwrap()
                .factor_entries() as f64
        };
        assert!(band(64) / band(32) > 7.0, "{} {}", band(32), band(64));
    }

    #[test]
    fn l_d_lt_is_the_same_bits_on_any_number_of_threads() {
        let (a, positions) = symmetric_laplacian(120, 90);
        let n = a.ncols();
        let b: Vec<c64> = (0..n).map(|i| c64::new((i % 5) as f64, -1.0)).collect();
        let analysis = Arc::new(
            Analysis::new_symmetric(a.as_ref(), Some(&positions))
                .unwrap()
                .unwrap(),
        );
        let run = |threads: usize| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| {
                    let lu = Multifrontal::new(analysis.clone(), a.as_ref()).unwrap();
                    lu.solve(&b)
                        .iter()
                        .map(|z| (z.re.to_bits(), z.im.to_bits()))
                        .collect::<Vec<_>>()
                })
        };
        let one = run(1);
        for threads in [2, 4, 5, 20] {
            assert!(run(threads) == one, "{threads} threads differ from one");
        }
    }
}

#[cfg(test)]
mod measure {
    use super::*;

    #[test]
    #[ignore = "a measurement: SYMMETRIC=1 (the symmetric forms), AMD=1 (AMD, not nested dissection), RAYON_NUM_THREADS; cargo test --release sparse::multifrontal::measure -- --ignored --nocapture"]
    fn on_the_exported_systems() {
        for id in ["slab-2d", "strip-24", "strip-32"] {
            let system = crate::bench::export::system(id).unwrap();
            // SYMMETRIC=1: its complex symmetric form, B = S A S⁻¹, by L D Lᵀ
            let symmetric = std::env::var("SYMMETRIC").is_ok();
            let system = if symmetric {
                let zero = vec![c64::new(0.0, 0.0); system.n];
                crate::bench::export::symmetric(&system, &zero).unwrap().0
            } else {
                system
            };
            let n = system.n;
            let t: Vec<Triplet<usize, usize, c64>> = system
                .entries
                .iter()
                .map(|&(r, c, v)| Triplet::new(r, c, v))
                .collect();
            let a = SparseColMat::<usize, c64>::try_new_from_triplets(n, n, &t).unwrap();
            let positions = if std::env::var("AMD").is_ok() {
                None
            } else {
                crate::bench::export::positions(&system)
            };
            let t0 = std::time::Instant::now();
            let analysis = Arc::new(if symmetric {
                Analysis::new_symmetric(a.as_ref(), positions.as_deref())
                    .unwrap()
                    .unwrap()
            } else {
                Analysis::new(a.as_ref(), positions.as_deref()).unwrap()
            });
            let t1 = std::time::Instant::now();
            let lu = Multifrontal::new(analysis.clone(), a.as_ref()).unwrap();
            let t2 = std::time::Instant::now();
            let x = lu.solve(&system.rhs);
            let t3 = std::time::Instant::now();
            let mut r = system.rhs.clone();
            for &(i, j, v) in &system.entries {
                r[i] -= v * x[j];
            }
            let norm = |v: &[c64]| v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
            let flops: f64 = analysis
                .fronts
                .iter()
                .map(|f| {
                    let k = (f.end - f.begin) as f64;
                    let p = f.below.len() as f64;
                    8.0 * (k * k * k / 3.0 + k * k * p + k * p * p)
                })
                .sum();
            eprintln!(
                "{id}: {:.1} Gflop, {:.1} Gflop/s",
                flops / 1e9,
                flops / 1e9 / (t2 - t1).as_secs_f64()
            );
            let identity = analysis
                .matching
                .rows
                .iter()
                .enumerate()
                .all(|(j, &r)| j == r);
            eprintln!(
                "{id}: analysis {:.3} s, factorization {:.3} s, solve {:.3} s, entries {:.1} M, fronts {}, largest {}, perturbed {}, matching identity {identity}, residual {:.1e}",
                (t1 - t0).as_secs_f64(),
                (t2 - t1).as_secs_f64(),
                (t3 - t2).as_secs_f64(),
                analysis.factor_entries() as f64 / 1e6,
                analysis.fronts.len(),
                analysis
                    .fronts
                    .iter()
                    .map(|f| f.end - f.begin + f.below.len())
                    .max()
                    .unwrap_or(0),
                lu.perturbed(),
                norm(&r) / norm(&system.rhs)
            );
        }
    }
}
