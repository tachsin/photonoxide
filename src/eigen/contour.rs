//! Every eigenvalue of a sparse pencil A x = λ B x inside a closed curve, by contour integrals.
//!
//! - **The projector.** For a curve Γ around some of the eigenvalues,
//!   P = (1/2πi) ∮_Γ (zB − A)⁻¹ B dz is the spectral projector onto their eigenvectors
//!   (J. Kestyn, E. Polizzi, P. T. P. Tang, "FEAST eigensolver for non-Hermitian problems", SIAM
//!   J. Sci. Comput. 38, S772 (2016), [doi:10.1137/15M1026572](https://doi.org/10.1137/15M1026572),
//!   Eq. 2.4), and P Y spans them for almost any block Y of at least as many columns.
//! - **The quadrature.** The integral is taken by the N-point trapezoidal rule in the curve's
//!   parameter (T. Sakurai, H. Sugiura, "A projection method for generalized eigenvalue
//!   problems using numerical integration", J. Comput. Appl. Math. 159, 119 (2003),
//!   [doi:10.1016/S0377-0427(03)00565-X](https://doi.org/10.1016/S0377-0427(03)00565-X),
//!   Section 3), so P becomes the rational filter Σ w_j (z_j B − A)⁻¹ B, one shifted solve per
//!   point (Kestyn Eqs. 1.4–1.6), whose error falls exponentially with N: an eigenvalue outside
//!   the curve is damped by about (its distance in units of the curve's size)^(−N).
//! - **The iteration.** FEAST's subspace iteration with Rayleigh–Ritz (E. Polizzi,
//!   "Density-matrix-based algorithm for solving eigenvalue problems", Phys. Rev. B 79, 115112
//!   (2009), [doi:10.1103/PhysRevB.79.115112](https://doi.org/10.1103/PhysRevB.79.115112),
//!   Fig. 2): filter the block, orthonormalize it, solve the small projected pencil, and filter
//!   the subspace again until every Ritz pair inside the curve has a small true residual
//!   (Kestyn Eq. 2.11), each checked with A and B themselves. The subspace is resized to its
//!   numerical rank (Kestyn Section 2.5), here by Gram–Schmidt twice, which keeps a real
//!   subspace real. The Rayleigh–Ritz step is one-sided, on the right subspace only, which
//!   Kestyn et al. note is possible; it gives no left eigenvectors.
//! - **The count.** The trace of P is the number of eigenvalues inside, and the first filtered
//!   block of random ±1 columns estimates it for free: (1/m) Σ y_cᴴ (P y)_c, whose mean is
//!   tr P. A subspace no larger than the number of Ritz values it puts inside the curve can't
//!   hold them all with room to converge (Kestyn Section 2.3: it must not be smaller than the
//!   count): that is an error, or, when no size was given, a larger subspace.
//!
//! The points are independent: each factorizes its z_j B − A with photonoxide's multifrontal
//! LU ([`crate::sparse::Multifrontal`], one analysis of the structure shared), refines its
//! solves, and they run side by side on rayon's threads, their contributions summed in the
//! points' order, so the result is the same bits on any number of threads. A real pencil on a
//! curve symmetric about the real axis needs only the upper half's points: the lower half's
//! solves are their conjugates (Kestyn Section 2.4, Table 1).

use std::sync::Arc;

use faer::sparse::{SparseColMat, Triplet};
use faer::{Mat, c64};
use rayon::prelude::*;

use super::Pair;
use crate::sparse::{Analysis, Multifrontal};
use crate::{Error, Result};

fn failed(why: impl Into<String>) -> Error {
    Error::invalid("contour eigensolver", why.into())
}

/// A pencil A x = λ B x of n × n sparse matrices, as (row, column, value) entries (repeated
/// entries are summed); B the identity when `None`.
pub(crate) struct Pencil<'a> {
    pub n: usize,
    pub a: &'a [(usize, usize, c64)],
    pub b: Option<&'a [(usize, usize, c64)]>,
    /// Each unknown's place on its grid, for a nested-dissection order; AMD without.
    pub positions: Option<&'a [[f64; 3]]>,
}

/// A sparse matrix as (row, column, value) entries.
pub(crate) type Entries = Vec<(usize, usize, c64)>;

/// A matrix by rows.
struct Rows(Vec<Vec<(usize, c64)>>);

impl Rows {
    fn new(n: usize, entries: &[(usize, usize, c64)]) -> Rows {
        let mut rows = vec![Vec::new(); n];
        for &(i, j, v) in entries {
            rows[i].push((j, v));
        }
        Rows(rows)
    }

    fn apply(&self, x: &[c64]) -> Vec<c64> {
        self.0
            .iter()
            .map(|row| row.iter().map(|&(j, v)| v * x[j]).sum())
            .collect()
    }
}

/// The pencil by rows: A, and B unless it is the identity.
struct Operators {
    a: Rows,
    b: Option<Rows>,
}

impl Operators {
    fn b(&self, x: &[c64]) -> Vec<c64> {
        match &self.b {
            Some(b) => b.apply(x),
            None => x.to_vec(),
        }
    }

    /// (zB − A) x.
    fn shifted(&self, z: c64, x: &[c64]) -> Vec<c64> {
        let ax = self.a.apply(x);
        let bx = self.b(x);
        bx.iter().zip(&ax).map(|(b, a)| z * b - a).collect()
    }
}

/// Quadrature on a closed curve: nodes z_j and weights w_j with
/// Σ w_j f(z_j) ≈ (1/2πi) ∮ f(z) dz for f analytic near the curve.
#[derive(Clone, Debug)]
pub(crate) struct Quadrature {
    nodes: Vec<c64>,
    weights: Vec<c64>,
    /// The second half's nodes and weights are the first half's conjugates, exactly.
    symmetric: bool,
}

impl Quadrature {
    /// The N-point trapezoidal rule in the parameter θ ∈ [0, 2π) of a closed, positively
    /// oriented curve, `curve(θ)` giving the point z(θ) and its derivative z'(θ): nodes at
    /// θ_j = 2π(j + ½)/N, off the real axis for a curve symmetric about it, and weights
    /// z'(θ_j)/(iN). With `symmetric` (the curve its own mirror image in the real axis, N even),
    /// the second half is the first's conjugates, bit for bit.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for fewer than 2 points, an odd count with `symmetric`, or a
    /// curve that isn't finite.
    pub(crate) fn trapezoid(
        points: usize,
        symmetric: bool,
        curve: impl Fn(f64) -> (c64, c64),
    ) -> Result<Quadrature> {
        if points < 2 || (symmetric && !points.is_multiple_of(2)) {
            return Err(failed(format!(
                "the quadrature takes at least 2 points, an even number on a curve symmetric about the real axis; got {points}"
            )));
        }
        let at = |j: usize| {
            let theta = std::f64::consts::TAU * (j as f64 + 0.5) / points as f64;
            let (z, dz) = curve(theta);
            (z, dz / c64::new(0.0, points as f64))
        };
        let (mut nodes, mut weights): (Vec<c64>, Vec<c64>) = if symmetric {
            (0..points / 2).map(at).unzip()
        } else {
            (0..points).map(at).unzip()
        };
        if symmetric {
            for j in (0..points / 2).rev() {
                nodes.push(nodes[j].conj());
                weights.push(weights[j].conj());
            }
        }
        if nodes.iter().chain(&weights).any(|z| !z.is_finite()) {
            return Err(failed("the curve must be finite"));
        }
        Ok(Quadrature {
            nodes,
            weights,
            symmetric,
        })
    }

    /// The nodes z_j.
    #[cfg(test)]
    pub(crate) fn nodes(&self) -> &[c64] {
        &self.nodes
    }

    /// The rational filter Σ w_j / (z_j − λ): about 1 inside the curve and 0 outside.
    #[cfg(test)]
    pub(crate) fn filter(&self, lambda: c64) -> c64 {
        self.nodes
            .iter()
            .zip(&self.weights)
            .map(|(z, w)| w / (z - lambda))
            .sum()
    }
}

/// What to search with.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Options {
    /// The subspace's size; when `None`, chosen from the count's estimate and grown as needed.
    pub subspace: Option<usize>,
    /// The largest relative residual ‖A x − λ B x‖ / (α ‖B x‖) accepted, α the largest |z_j|.
    pub tolerance: f64,
    /// The most filter passes, all subspaces together.
    pub iterations: usize,
}

/// The eigenpairs inside the curve, and how they were found.
pub(crate) struct Found {
    pub pairs: Vec<Pair>,
    /// Each pair's relative residual, as in [`Options::tolerance`].
    pub residuals: Vec<f64>,
    /// The count's estimate from the first filtered block: the trace of the filter.
    pub estimate: f64,
    /// The size of the last subspace.
    pub subspace: usize,
    /// Filter passes taken, all subspaces together.
    pub iterations: usize,
}

/// A fixed sequence of 64-bit words from `seed`: splitmix64.
fn bits(seed: u64) -> impl FnMut() -> u64 {
    let mut state = seed;
    move || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

/// A fixed block of ±1 entries: the start block, the same on every run.
fn rademacher(n: usize, columns: usize, seed: u64) -> Vec<Vec<c64>> {
    let mut next = bits(seed);
    (0..columns)
        .map(|_| {
            (0..n)
                .map(|_| c64::new(if next() >> 63 == 0 { 1.0 } else { -1.0 }, 0.0))
                .collect()
        })
        .collect()
}

fn dot(a: &[c64], b: &[c64]) -> c64 {
    // bᴴ a
    a.iter().zip(b).map(|(x, y)| x * y.conj()).sum()
}

fn norm(a: &[c64]) -> f64 {
    a.iter().map(|x| x.norm_sqr()).sum::<f64>().sqrt()
}

/// An orthonormal basis of the columns' span, by modified Gram–Schmidt twice; a column whose
/// part outside the basis so far is below 1e-10 of itself, or 1e-14 of the largest column, is
/// dependent and dropped (the subspace resized to its numerical rank). Real columns stay real.
fn orthonormalize(columns: Vec<Vec<c64>>) -> Vec<Vec<c64>> {
    let largest = columns.iter().map(|c| norm(c)).fold(0.0, f64::max);
    let mut basis: Vec<Vec<c64>> = Vec::with_capacity(columns.len());
    for mut q in columns {
        let before = norm(&q);
        for _ in 0..2 {
            for u in &basis {
                let h = dot(&q, u);
                for (qi, ui) in q.iter_mut().zip(u) {
                    *qi -= h * ui;
                }
            }
        }
        let after = norm(&q);
        if after > 1e-10 * before && after > 1e-14 * largest && after > 0.0 {
            basis.push(q.iter().map(|x| x / after).collect());
        }
    }
    basis
}

/// The Ritz values and the small problem's eigenvectors on the orthonormal basis `u`:
/// Uᴴ A U w = θ w when B is the identity, Uᴴ A U w = θ Uᴴ B U w (by QZ) otherwise; an infinite
/// eigenvalue of a singular B is +∞.
fn ritz(operators: &Operators, u: &[Vec<c64>]) -> Result<(Vec<c64>, Mat<c64>)> {
    let r = u.len();
    let au: Vec<Vec<c64>> = u.iter().map(|c| operators.a.apply(c)).collect();
    let ar = Mat::<c64>::from_fn(r, r, |i, j| dot(&au[j], &u[i]));
    let broken = || failed("the projected eigenproblem's solve failed (not a number)");
    match &operators.b {
        None => {
            let eig = ar
                .eigen()
                .map_err(|e| failed(format!("the projected eigenproblem: {e:?}")))?;
            let thetas: Vec<c64> = (0..r).map(|k| eig.S()[k]).collect();
            if thetas.iter().any(|t| t.is_nan()) {
                return Err(broken());
            }
            Ok((thetas, eig.U().to_owned()))
        }
        Some(b) => {
            let bu: Vec<Vec<c64>> = u.iter().map(|c| b.apply(c)).collect();
            let br = Mat::<c64>::from_fn(r, r, |i, j| dot(&bu[j], &u[i]));
            let eig = ar
                .generalized_eigen(&br)
                .map_err(|e| failed(format!("the projected pencil: {e:?}")))?;
            let (alpha, beta) = (eig.S_a(), eig.S_b());
            if (0..r).any(|k| alpha[k].is_nan() || beta[k].is_nan()) {
                return Err(broken());
            }
            let thetas = (0..r)
                .map(|k| {
                    if beta[k] == c64::new(0.0, 0.0) {
                        c64::new(f64::INFINITY, 0.0)
                    } else {
                        alpha[k] / beta[k]
                    }
                })
                .collect();
            Ok((thetas, eig.U().to_owned()))
        }
    }
}

/// The factors at each point, kept between passes while they fit this many entries in all
/// (16 bytes each: 2 GiB), refactorized each pass beyond it; the result is the same either way.
const KEPT_ENTRIES: usize = 1 << 27;

/// Filter passes in one subspace without convergence before a chosen subspace is doubled.
const STALL: usize = 6;

/// A shifted matrix's factors: photonoxide's multifrontal LU, or faer's sparse LU for a
/// structure the multifrontal analysis declines.
enum Factors {
    Own(Multifrontal),
    Faer(Box<faer::sparse::linalg::solvers::Lu<usize, c64>>),
}

impl Factors {
    fn solve(&self, b: &[c64]) -> Vec<c64> {
        match self {
            Factors::Own(f) => f.solve(b),
            Factors::Faer(lu) => {
                use faer::linalg::solvers::Solve;
                let mut rhs = Mat::<c64>::from_fn(b.len(), 1, |i, _| b[i]);
                lu.solve_in_place(rhs.as_mut());
                (0..b.len()).map(|i| rhs[(i, 0)]).collect()
            }
        }
    }
}

/// The shifted matrices z_j B − A, their factors and refined solves.
struct Shifts<'a> {
    pencil: &'a Pencil<'a>,
    operators: &'a Operators,
    analysis: Option<Arc<Analysis>>,
    keep: bool,
}

impl Shifts<'_> {
    fn matrix(pencil: &Pencil, z: c64) -> Result<SparseColMat<usize, c64>> {
        let n = pencil.n;
        let mut triplets: Vec<Triplet<usize, usize, c64>> =
            Vec::with_capacity(pencil.a.len() + pencil.b.map_or(n, <[_]>::len) + n);
        for &(i, j, v) in pencil.a {
            triplets.push(Triplet::new(i, j, -v));
        }
        match pencil.b {
            Some(b) => {
                for &(i, j, v) in b {
                    triplets.push(Triplet::new(i, j, z * v));
                }
                // every diagonal entry present, so every point has the same structure
                for i in 0..n {
                    triplets.push(Triplet::new(i, i, c64::new(0.0, 0.0)));
                }
            }
            None => {
                for i in 0..n {
                    triplets.push(Triplet::new(i, i, z));
                }
            }
        }
        SparseColMat::<usize, c64>::try_new_from_triplets(n, n, &triplets)
            .map_err(|e| failed(format!("the shifted matrix: {e:?}")))
    }

    /// The analysis of the matrices' structure, from the matrix at `z`: `None` where the
    /// multifrontal LU declines it (a structure with nothing to eliminate together, such as a
    /// diagonal), whose points faer's sparse LU then factorizes.
    fn analysis(pencil: &Pencil, z: c64) -> Result<Option<Arc<Analysis>>> {
        let m = Self::matrix(pencil, z)?;
        match Analysis::new(m.as_ref(), pencil.positions) {
            Ok(a) => Ok(Some(Arc::new(a))),
            Err(e) if e.to_string().contains("simplicial") => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// The factors of z B − A: with the shared analysis, or with its own where that one left a
    /// pivot perturbed (the matching chose for another point's values).
    fn factor(&self, z: c64) -> Result<Factors> {
        let m = Self::matrix(self.pencil, z)?;
        let Some(analysis) = &self.analysis else {
            return m
                .sp_lu()
                .map(|lu| Factors::Faer(Box::new(lu)))
                .map_err(|e| failed(format!("the shifted matrix at {z} is singular: {e:?}")));
        };
        let factors = Multifrontal::new(analysis.clone(), m.as_ref())?;
        if factors.perturbed() == 0 {
            return Ok(Factors::Own(factors));
        }
        let own = Arc::new(Analysis::new(m.as_ref(), self.pencil.positions)?);
        Multifrontal::new(own, m.as_ref()).map(Factors::Own)
    }

    /// x with (zB − A) x = b, refined against the matrix itself (the factors' static pivoting
    /// wants it).
    fn solve(&self, factors: &Factors, z: c64, b: &[c64]) -> Vec<c64> {
        let mut x = factors.solve(b);
        let size = norm(b);
        for _ in 0..3 {
            let mx = self.operators.shifted(z, &x);
            let r: Vec<c64> = b.iter().zip(&mx).map(|(b, m)| b - m).collect();
            if norm(&r) <= 1e-15 * size {
                break;
            }
            let dx = factors.solve(&r);
            for (xi, di) in x.iter_mut().zip(&dx) {
                *xi += di;
            }
        }
        x
    }
}

/// Q = Σ w_j (z_j B − A)⁻¹ B Y over the points, in their order: on a real pencil with a
/// symmetric rule and real Y, the upper half's points only, each with its conjugate's share,
/// 2 Re(w_j X_j). `None` when `stop` says so, asked before each batch of points.
fn filter(
    shifts: &Shifts,
    quadrature: &Quadrature,
    real: bool,
    cache: &mut [Option<Factors>],
    y: &[Vec<c64>],
    stop: &dyn Fn() -> bool,
) -> Result<Option<Vec<Vec<c64>>>> {
    let n = shifts.pencil.n;
    let by: Vec<Vec<c64>> = y.iter().map(|c| shifts.operators.b(c)).collect();
    let mut q = vec![vec![c64::new(0.0, 0.0); n]; y.len()];
    // a batch of points at a time, side by side; the sum is taken in the points' order whatever
    // the batch, so the bits don't depend on the threads
    let batch = rayon::current_num_threads().max(1);
    let points = cache.len();
    let mut start = 0;
    while start < points {
        if stop() {
            return Ok(None);
        }
        let end = (start + batch).min(points);
        let solved: Vec<Result<Vec<Vec<c64>>>> = cache[start..end]
            .par_iter_mut()
            .enumerate()
            .map(|(k, kept)| {
                let j = start + k;
                let z = quadrature.nodes[j];
                let factors = match kept.take() {
                    Some(f) => f,
                    None => shifts.factor(z)?,
                };
                let x: Vec<Vec<c64>> = by.iter().map(|b| shifts.solve(&factors, z, b)).collect();
                if shifts.keep {
                    *kept = Some(factors);
                }
                Ok(x)
            })
            .collect();
        for (k, x) in solved.into_iter().enumerate() {
            let w = quadrature.weights[start + k];
            for (qc, xc) in q.iter_mut().zip(x?) {
                for (qi, xi) in qc.iter_mut().zip(xc) {
                    let t = w * xi;
                    *qi += if real { c64::new(2.0 * t.re, 0.0) } else { t };
                }
            }
        }
        start = end;
    }
    Ok(Some(q))
}

/// The eigenpairs of `pencil` with eigenvalues inside the curve of `quadrature` (`inside` says
/// which those are), each with its relative residual below `options.tolerance`.
///
/// # Errors
///
/// [`Error::InvalidValue`] if a shifted matrix can't be factorized (an eigenvalue on a node),
/// the given subspace is too small for the eigenvalues inside, or the pairs inside don't
/// converge within the iterations.
#[cfg(test)]
pub(crate) fn solve(
    pencil: &Pencil,
    quadrature: &Quadrature,
    inside: &(dyn Fn(c64) -> bool + Sync),
    options: &Options,
) -> Result<Found> {
    // (never asked to stop, it always has its pairs or an error)
    Ok(solve_until(pencil, quadrature, inside, options, &|| false)?.expect("not stopped"))
}

/// As [`solve`], giving up when `stop` says so: `None` then. It is asked before each batch of
/// quadrature points (a factorization and solves each, side by side).
///
/// # Errors
///
/// As [`solve`].
pub(crate) fn solve_until(
    pencil: &Pencil,
    quadrature: &Quadrature,
    inside: &(dyn Fn(c64) -> bool + Sync),
    options: &Options,
    stop: &dyn Fn() -> bool,
) -> Result<Option<Found>> {
    let n = pencil.n;
    if n == 0 {
        return Ok(Some(Found {
            pairs: Vec::new(),
            residuals: Vec::new(),
            estimate: 0.0,
            subspace: 0,
            iterations: 0,
        }));
    }
    if options.tolerance.is_nan() || options.tolerance <= 0.0 || options.iterations == 0 {
        return Err(failed(
            "the tolerance must be positive and the iterations at least 1",
        ));
    }
    if options.subspace == Some(0) {
        return Err(failed("the subspace must hold at least 1 vector"));
    }
    let operators = Operators {
        a: Rows::new(n, pencil.a),
        b: pencil.b.map(|b| Rows::new(n, b)),
    };
    let real = quadrature.symmetric
        && pencil
            .a
            .iter()
            .chain(pencil.b.unwrap_or(&[]))
            .all(|e| e.2.im == 0.0);
    let points = if real {
        quadrature.nodes.len() / 2
    } else {
        quadrature.nodes.len()
    };
    let analysis = Shifts::analysis(pencil, quadrature.nodes[0])?;
    let entries = analysis
        .as_ref()
        .map_or(pencil.a.len() + n, |a| a.factor_entries());
    let keep = entries.saturating_mul(points) <= KEPT_ENTRIES;
    let shifts = Shifts {
        pencil,
        operators: &operators,
        analysis,
        keep,
    };
    let mut cache: Vec<Option<Factors>> = (0..points).map(|_| None).collect();
    let scale = quadrature
        .nodes
        .iter()
        .map(|z| z.norm())
        .fold(0.0, f64::max);

    let cap = n.min(4096);
    let mut size = options.subspace.unwrap_or(8).min(n);
    let mut estimate = None;
    let mut passes = 0;
    'subspace: loop {
        let mut y = rademacher(n, size, 0x5eed_c0de ^ size as u64);
        let mut here = 0;
        loop {
            if passes == options.iterations {
                return Err(failed(format!(
                    "the eigenpairs inside the region didn't converge in {} filter passes (subspace {size}); more quadrature points or a larger subspace may help",
                    options.iterations
                )));
            }
            passes += 1;
            here += 1;
            let Some(q) = filter(&shifts, quadrature, real, &mut cache, &y, stop)? else {
                return Ok(None);
            };
            if estimate.is_none() {
                // the trace of the filter, by the ±1 columns: (1/m) Σ y_cᴴ q_c
                let e = q.iter().zip(&y).map(|(qc, yc)| dot(qc, yc).re).sum::<f64>() / size as f64;
                estimate = Some(e);
                // the subspace should hold the count with room to spare
                let wanted = (1.5 * e.max(0.0)).ceil() as usize + 4;
                // an eigenvalue near a point spoils the estimate (the filter is large there)
                let plausible = e.is_finite() && e > -0.5 && e < n as f64;
                match options.subspace {
                    Some(m) if plausible && e > m as f64 => {
                        return Err(failed(format!(
                            "the region holds about {e:.1} eigenvalues, more than the subspace of {m}: give a larger subspace or a smaller region"
                        )));
                    }
                    None if plausible && wanted > size && size < cap => {
                        size = (2 * e.max(0.0).ceil() as usize + 8).min(cap);
                        continue 'subspace;
                    }
                    _ => {}
                }
            }
            let u = orthonormalize(q);
            let r = u.len();
            if r == 0 {
                return Ok(Some(Found {
                    pairs: Vec::new(),
                    residuals: Vec::new(),
                    estimate: estimate.unwrap_or(0.0),
                    subspace: 0,
                    iterations: passes,
                }));
            }
            // Rayleigh–Ritz: Uᴴ A U w = θ Uᴴ B U w
            let (thetas, w) = ritz(&operators, &u)?;
            let mut found = Vec::new();
            for (k, &theta) in thetas.iter().enumerate() {
                if !theta.is_finite() || !inside(theta) {
                    continue;
                }
                let mut x = vec![c64::new(0.0, 0.0); n];
                for (i, ui) in u.iter().enumerate() {
                    let c = w[(i, k)];
                    for (xj, uj) in x.iter_mut().zip(ui) {
                        *xj += c * uj;
                    }
                }
                let xn = norm(&x);
                let x: Vec<c64> = x.iter().map(|v| v / xn).collect();
                let ax = operators.a.apply(&x);
                let bx = operators.b(&x);
                let res = norm(
                    &ax.iter()
                        .zip(&bx)
                        .map(|(a, b)| a - theta * b)
                        .collect::<Vec<_>>(),
                ) / (scale * norm(&bx));
                found.push((theta, x, res));
            }
            if found.len() >= r {
                // no room: the region may hold more eigenvalues than the subspace
                if options.subspace.is_none() && r == size && size < cap {
                    size = (2 * size).min(cap);
                    continue 'subspace;
                }
                return Err(failed(format!(
                    "the subspace of {size} vectors (rank {r}) puts {} Ritz values inside the region, which may hold more: give a larger subspace or a smaller region (about {:.1} estimated)",
                    found.len(),
                    estimate.unwrap_or(f64::NAN)
                )));
            }
            if found.iter().all(|f| f.2 <= options.tolerance) {
                let residuals = found.iter().map(|f| f.2).collect();
                let pairs = found
                    .into_iter()
                    .map(|(value, vector, _)| Pair { value, vector })
                    .collect();
                return Ok(Some(Found {
                    pairs,
                    residuals,
                    estimate: estimate.unwrap_or(0.0),
                    subspace: r,
                    iterations: passes,
                }));
            }
            // no convergence after several passes in this subspace: one too small converges
            // slowly (Kestyn Section 2.3), so a larger one, unless the size was given. But a
            // filtered block whose rank is below its size spans more than 14 decades: a few
            // eigenvalues near the boundary, or of enormous condition (a PML's own continuum),
            // swamp the rest, and no subspace helps
            if here >= STALL && r < size {
                let worst = found.iter().map(|f| f.2).fold(0.0, f64::max);
                return Err(failed(format!(
                    "the residuals stall at {worst:.1e}: the filtered subspace has rank {r} of {size}, swamped by eigenvalues near the region's boundary or of enormous condition (a PML's continuum); move the boundary away from them, or take more quadrature points"
                )));
            }
            if options.subspace.is_none() && here >= STALL && size < cap {
                size = (2 * size).min(cap);
                continue 'subspace;
            }
            // filter the subspace again (its orthonormal basis spans the Ritz vectors)
            y = u;
        }
    }
}

/// Sakurai and Sugiura's method as their Section 3 gives it: the m eigenvalues inside the
/// circle of centre γ and radius ρ from the moments μ_k of f(z) = uᴴ (zB − A)⁻¹ v by the
/// N-point trapezoidal rule at ω_j = γ + ρ e^(2πij/N) (Eq. 8), as the eigenvalues of the
/// Hankel pencil H<_m − ζ H_m, plus γ. The moments are taken in units of ρ, ((ω_j − γ)/ρ)^k,
/// and ζ scaled back, which is the same pencil better scaled.
///
/// # Errors
///
/// [`Error::InvalidValue`] if a shifted matrix can't be factorized or the Hankel pencil's
/// eigenvalues can't be found.
pub(crate) fn hankel(
    pencil: &Pencil,
    centre: c64,
    radius: f64,
    points: usize,
    m: usize,
    u: &[c64],
    v: &[c64],
) -> Result<Vec<c64>> {
    let n = pencil.n;
    let operators = Operators {
        a: Rows::new(n, pencil.a),
        b: pencil.b.map(|b| Rows::new(n, b)),
    };
    let omega: Vec<c64> = (0..points)
        .map(|j| {
            let theta = std::f64::consts::TAU * j as f64 / points as f64;
            centre + radius * c64::from_polar(1.0, theta)
        })
        .collect();
    let shifts = Shifts {
        pencil,
        operators: &operators,
        analysis: Shifts::analysis(pencil, omega[0])?,
        keep: false,
    };
    // f(ω_j) = uᴴ y_j, y_j = (ω_j B − A)⁻¹ v, each point on its own
    let f: Vec<c64> = omega
        .par_iter()
        .map(|&z| -> Result<c64> {
            let factors = shifts.factor(z)?;
            Ok(dot(&shifts.solve(&factors, z, v), u))
        })
        .collect::<Result<Vec<c64>>>()?;
    // μ_k = (1/N) Σ ((ω_j − γ)/ρ)^(k+1) f(ω_j), k = 0 … 2m − 1
    let mu: Vec<c64> = (0..2 * m)
        .map(|k| {
            omega
                .iter()
                .zip(&f)
                .map(|(&z, &fj)| ((z - centre) / radius).powi(k as i32 + 1) * fj)
                .sum::<c64>()
                / points as f64
        })
        .collect();
    let h = Mat::<c64>::from_fn(m, m, |i, j| mu[i + j]);
    let shifted = Mat::<c64>::from_fn(m, m, |i, j| mu[i + j + 1]);
    let eig = shifted
        .generalized_eigen(&h)
        .map_err(|e| failed(format!("the Hankel pencil: {e:?}")))?;
    let mut values: Vec<c64> = (0..m)
        .map(|k| centre + radius * eig.S_a()[k] / eig.S_b()[k])
        .collect();
    values.sort_by(|a, b| a.re.total_cmp(&b.re).then(a.im.total_cmp(&b.im)));
    Ok(values)
}

/// A circle of centre c and radius r as a quadrature's curve.
#[cfg(test)]
pub(crate) fn circle(c: c64, r: f64) -> impl Fn(f64) -> (c64, c64) {
    move |t: f64| {
        let e = c64::from_polar(1.0, t);
        (c + r * e, c64::new(0.0, r) * e)
    }
}

/// Sakurai and Sugiura's Example 5 (their Eq. 11): A = diag(99, 98, …, 1, 0)/100 and
/// B = diag(0, …, 0, I₂₀), 100 × 100, whose finite eigenvalues are (j − 1)/100, j = 1 … 20.
pub(crate) fn sakurai_sugiura_example_5() -> (Entries, Entries) {
    let n = 100;
    let a = (0..n)
        .map(|i| (i, i, c64::new((99 - i) as f64 / 100.0, 0.0)))
        .collect();
    let b = (80..n).map(|i| (i, i, c64::new(1.0, 0.0))).collect();
    (a, b)
}

/// The largest error of the four eigenvalues in Sakurai and Sugiura's circle of Example 5
/// (centre 0.015, radius 0.02: 0, 0.01, 0.02, 0.03) by their Hankel method with m = 4 at
/// `points` points, u and v vectors uniform in (0, 1) as theirs from Matlab's `rand`, the
/// `draw`-th of a fixed sequence of them.
pub(crate) fn sakurai_sugiura_error(points: usize, draw: u64) -> f64 {
    let (a, b) = sakurai_sugiura_example_5();
    let pencil = Pencil {
        n: 100,
        a: &a,
        b: Some(&b),
        positions: None,
    };
    let uniform = |seed: u64| -> Vec<c64> {
        let mut next = bits(seed);
        (0..100)
            .map(|_| c64::new((next() >> 11) as f64 / (1u64 << 53) as f64, 0.0))
            .collect()
    };
    let (u, v) = (uniform(2 * draw + 1), uniform(2 * draw + 2));
    match hankel(&pencil, c64::new(0.015, 0.0), 0.02, points, 4, &u, &v) {
        Ok(values) => values
            .iter()
            .zip([0.0, 0.01, 0.02, 0.03])
            .map(|(l, e)| (l - e).norm())
            .fold(0.0, f64::max),
        Err(_) => f64::NAN,
    }
}

#[cfg(test)]
mod tests;
