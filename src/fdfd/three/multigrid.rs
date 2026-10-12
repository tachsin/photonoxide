//! A multigrid preconditioner for the 3D system: cycles on Shin and Fan's operator, with Galerkin
//! coarse operators, ILU(0) smoothing and a direct solve on the coarsest grid.
//!
//! B. Reps, W. Vanroose, H. bin Zubair, J. Comput. Phys. 229, 8384 (2010),
//! doi:10.1016/j.jcp.2010.07.022, Section 6.1 and Fig. 14: for Helmholtz problems with
//! complex-stretched absorbing layers, ILU(0) smoothing with Galerkin coarse operators, full
//! weighting and (bi)linear prolongation. Y. A. Erlangga, C. W. Oosterlee, C. Vuik, SIAM J. Sci.
//! Comput. 27, 1471 (2006), doi:10.1137/040615195: the complex shift of the preconditioner's
//! operator (their Eq. 8, (β₁, β₂)) and F-cycles (Section 4.3).
//!
//! On Yee's grid each component of E is interpolated from a coarser grid over the same box on
//! its own staggered points, trilinearly (P), restricted by Pᵀ, and each coarse operator is
//! Pᵀ A P.
//!
//! **Without the finest matrix** ([`Hierarchy::from_stencil_on`], #333): the finest level's
//! operator is Shin and Fan's from its stencil ([`MatrixFree::shin_fan`]), smoothed by damped
//! Jacobi with the diagonal the stencil gives, Erlangga, Oosterlee and Vuik's smoother for the
//! shifted operator (their Section 4.3: ω-Jacobi; ω = 0.8 here, measured). The first
//! coarse operator is still Pᵀ A P, its fine rows made from the stencil one at a time, to the
//! same bits as the stored matrix's, or, [`Coarse::Rediscretized`], every coarse operator is
//! Shin and Fan's discretized again on the coarser grid, restricted by W⁻¹ Pᵀ D_f
//! (docs/methods/multigrid-stencil.md).

use std::sync::Arc;

use num_complex::Complex64 as c64;

use super::matrix_free::MatrixFree;
use super::{Axis, Grid3d, Lattice};
use crate::backend::{Columns, DirectSolver, Factorization, Form};
use crate::fdfd::Edges;
use crate::fdfd::krylov::{BlockIlu0, Operator, Preconditioner, Sparse};
use crate::{Error, Result};

/// The shape of a multigrid cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CycleShape {
    /// One coarse correction per level.
    V,
    /// An F-cycle: an F-cycle then a V-cycle on the coarser level.
    F,
    /// Two coarse corrections per level.
    W,
}

/// How the multigrid preconditioner is built and cycled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Multigrid {
    /// The complex shift β of the preconditioner's operator: k₀²(1 + iβ)ε on its diagonal
    /// (Erlangga et al.'s (β₁, β₂) = (1, β), on the side of the medium's loss). 0 for none.
    pub shift: f64,
    /// The cycle.
    pub shape: CycleShape,
    /// ILU(0) smoothing steps before each coarse correction.
    pub pre: usize,
    /// And after it.
    pub post: usize,
    /// The coarsest grid's largest number of unknowns, solved directly.
    pub coarsest: usize,
    /// The steps of GMRES between restarts: it keeps one vector of the grid's size per step,
    /// so this bounds its memory (16 bytes per unknown and step).
    pub restart: usize,
    /// How the coarse operators are made.
    pub coarse: Coarse,
    /// The finest level's smoothing when it has no matrix (a solver built with
    /// [`super::Product::MatrixFree`]); the coarser levels take ILU(0) as `pre` and `post` say.
    pub jacobi: Jacobi,
}

/// How the cycle's coarse operators are made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coarse {
    /// Pᵀ A P, Reps et al.'s Galerkin operators: from the finest matrix, or, without it, from
    /// the stencil's rows one at a time, to the same bits.
    Galerkin,
    /// Shin and Fan's operator discretized again on each coarser grid, each cell and dual cell
    /// as long as the finer ones it merges in the PMLs' stretched coordinate, the permittivity
    /// averaged onto it, and the residual restricted by W⁻¹ Pᵀ D_f (D_f the volume of each fine
    /// value of E's cell, in that coordinate, and W = Pᵀ D_f 1, so that its rows sum to one): 15
    /// entries a row where Pᵀ A P has about 60. More iterations than Galerkin's, and V-cycles
    /// only: with F-cycles it fails (docs/methods/multigrid-stencil.md).
    Rediscretized,
}

/// Damped Jacobi on the finest level without its matrix: x ← x + ω D⁻¹ (b − A x), D the
/// operator's diagonal from its stencil.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Jacobi {
    /// ω.
    pub omega: f64,
    /// Sweeps before each coarse correction.
    pub pre: usize,
    /// And after it.
    pub post: usize,
}

impl Default for Multigrid {
    /// Reps et al.'s V(0, 1) with a shift of 0.5, the coarsest grid at most 2 000 unknowns, and
    /// GMRES restarted every 40 steps: what was measured (docs/methods/fdfd-3d.md). Without the
    /// shift neither Diel nor the strip with ports converged in 20 minutes; a shift of 1 takes
    /// as many iterations to a less accurate field; a coarsest grid of 20 000 saves one or two
    /// iterations and takes four times as long to build.
    fn default() -> Multigrid {
        Multigrid {
            shift: 0.5,
            shape: CycleShape::V,
            pre: 0,
            post: 1,
            coarsest: 2_000,
            restart: 40,
            coarse: Coarse::Galerkin,
            jacobi: Jacobi {
                omega: 0.8,
                pre: 1,
                post: 1,
            },
        }
    }
}

/// A rectangular sparse matrix by rows, for the transfers between levels: its products with
/// vectors on rayon's threads, each row summed in order, the same bit for bit on any number.
struct Transfer {
    starts: Vec<usize>,
    columns: Vec<usize>,
    values: Vec<c64>,
}

impl Transfer {
    fn from_rows(rows: &[Vec<(usize, c64)>]) -> Transfer {
        let mut starts = Vec::with_capacity(rows.len() + 1);
        starts.push(0);
        let (mut columns, mut values) = (Vec::new(), Vec::new());
        for row in rows {
            for &(c, v) in row {
                columns.push(c);
                values.push(v);
            }
            starts.push(columns.len());
        }
        Transfer {
            starts,
            columns,
            values,
        }
    }

    /// The transpose, `columns` rows of it, each row's entries in order of the original rows.
    fn transpose(&self, columns: usize) -> Transfer {
        let mut starts = vec![0usize; columns + 1];
        for &c in &self.columns {
            starts[c + 1] += 1;
        }
        for c in 0..columns {
            starts[c + 1] += starts[c];
        }
        let mut next = starts.clone();
        let mut cols = vec![0; self.columns.len()];
        let mut values = vec![c64::new(0.0, 0.0); self.columns.len()];
        for r in 0..self.starts.len() - 1 {
            for k in self.starts[r]..self.starts[r + 1] {
                let at = &mut next[self.columns[k]];
                cols[*at] = r;
                values[*at] = self.values[k];
                *at += 1;
            }
        }
        Transfer {
            starts,
            columns: cols,
            values,
        }
    }

    fn row(&self, r: usize) -> impl Iterator<Item = (usize, c64)> + '_ {
        (self.starts[r]..self.starts[r + 1]).map(|k| (self.columns[k], self.values[k]))
    }

    /// As a backend takes it, with `columns` columns.
    fn rows(&self, columns: usize) -> crate::backend::RowMatrix<'_> {
        crate::backend::RowMatrix::trusted(columns, &self.starts, &self.columns, &self.values)
    }

    fn apply(&self, v: &[c64]) -> Vec<c64> {
        use rayon::prelude::*;
        let rows = self.starts.len() - 1;
        crate::traffic::add(crate::traffic::product(rows, v.len(), self.values.len()));
        let mut out = vec![c64::new(0.0, 0.0); rows];
        out.par_iter_mut()
            .with_min_len(4096)
            .enumerate()
            .for_each(|(r, o)| *o = self.row(r).map(|(c, w)| w * v[c]).sum());
        out
    }
}

/// x += d, on rayon's threads.
fn add_to(x: &mut [c64], d: &[c64]) {
    use rayon::prelude::*;
    crate::traffic::add(crate::traffic::vectors(x.len(), 2, 1));
    x.par_iter_mut()
        .with_min_len(16_384)
        .zip(d)
        .for_each(|(xk, dk)| *xk += dk);
}

/// A level's operator and smoother.
enum Stage {
    /// The operator's matrix, smoothed by ILU(0) in slabs.
    Stored { matrix: Sparse, smoother: BlockIlu0 },
    /// Without its matrix: Shin and Fan's operator from its stencil, smoothed by damped Jacobi
    /// (ω D⁻¹ here).
    Stencil {
        operator: MatrixFree,
        jacobi: Vec<c64>,
        pre: usize,
        post: usize,
    },
}

impl Stage {
    /// A x, or Aᵀ x.
    fn apply(&self, x: &[c64], transpose: bool) -> Vec<c64> {
        use crate::fdfd::krylov::Operator;
        match (self, transpose) {
            (Stage::Stored { matrix, .. }, false) => matrix.apply(x),
            (Stage::Stored { matrix, .. }, true) => matrix.apply_transpose(x),
            (Stage::Stencil { operator, .. }, false) => operator.apply(x),
            (Stage::Stencil { operator, .. }, true) => operator.apply_transpose(x),
        }
    }

    /// One smoothing step's correction for the residual `r`: M⁻¹ r, or M⁻ᵀ r.
    fn smooth(&self, r: &[c64], transpose: bool) -> Vec<c64> {
        use rayon::prelude::*;
        match self {
            Stage::Stored { smoother, .. } if transpose => smoother.solve_transpose(r),
            Stage::Stored { smoother, .. } => smoother.solve(r),
            // a diagonal is its own transpose
            Stage::Stencil { jacobi, .. } => {
                crate::traffic::add(crate::traffic::vectors(r.len(), 2, 1));
                r.par_iter()
                    .with_min_len(16_384)
                    .zip(jacobi)
                    .map(|(rk, dk)| rk * dk)
                    .collect()
            }
        }
    }

    /// The smoothing steps before and after the coarse corrections.
    fn steps(&self, options: &Multigrid) -> (usize, usize) {
        match self {
            Stage::Stored { .. } => (options.pre, options.post),
            Stage::Stencil { pre, post, .. } => (*pre, *post),
        }
    }

    /// The operator's matrix, if it has one.
    #[cfg(test)]
    fn matrix(&self) -> Option<&Sparse> {
        match self {
            Stage::Stored { matrix, .. } => Some(matrix),
            Stage::Stencil { .. } => None,
        }
    }
}

/// One level: its operator, its smoother, and the transfers to and from the next.
struct Level {
    stage: Stage,
    /// P by rows: for each value here, the next level's values and their weights.
    prolongation: Transfer,
    /// Pᵀ by rows.
    restriction: Transfer,
    /// For a rediscretized next level, the restriction W⁻¹ Pᵀ D_f: D_f here and W⁻¹ there.
    scaling: Option<(Vec<c64>, Vec<c64>)>,
}

impl Level {
    /// The residual `r` restricted to the next level: R r, or, for the transposed cycle, Pᵀ r
    /// (the transpose of the prolongation R's transpose is).
    fn restrict(&self, r: &[c64], transpose: bool) -> Vec<c64> {
        match &self.scaling {
            Some((fine, coarse)) if !transpose => {
                let scaled: Vec<c64> = r.iter().zip(fine).map(|(p, q)| p * q).collect();
                let mut out = self.restriction.apply(&scaled);
                out.iter_mut().zip(coarse).for_each(|(p, q)| *p *= q);
                out
            }
            _ => self.restriction.apply(r),
        }
    }

    /// The next level's correction `c` brought here: P c, or, for the transposed cycle, Rᵀ c.
    fn prolong(&self, c: &[c64], transpose: bool) -> Vec<c64> {
        match &self.scaling {
            Some((fine, coarse)) if transpose => {
                let scaled: Vec<c64> = c.iter().zip(coarse).map(|(p, q)| p * q).collect();
                let mut out = self.prolongation.apply(&scaled);
                out.iter_mut().zip(fine).for_each(|(p, q)| *p *= q);
                out
            }
            _ => self.prolongation.apply(c),
        }
    }
}

/// The cycle's levels, finest first, and the coarsest's factorization.
pub(crate) struct Hierarchy {
    levels: Vec<Level>,
    /// Each level's cells along x, y and z, the coarsest's included.
    shapes: Vec<[usize; 3]>,
    lu: Box<dyn Factorization>,
    /// The coarsest level's operator, which `lu` factorizes.
    coarsest: Sparse,
    /// Each level's smoother as one pair of factors, for a backend: made the first time one
    /// asks ([`Hierarchy::cycle_for_backend`]).
    smoothers: std::sync::OnceLock<Option<Vec<crate::backend::IluFactors>>>,
    /// One thread, for the solves with the coarsest factors: their sums then come in one
    /// order whatever the machine. A pool isn't `RefUnwindSafe`, but one that a panic left stays
    /// usable, so this keeps `IterativeSolver3d` unwind-safe, as it was before the multigrid.
    one: std::panic::AssertUnwindSafe<rayon::ThreadPool>,
    options: Multigrid,
}

/// The smoother's blocks on a level of `cells` along x, y and z: slabs of [`PLANES`] planes
/// across z, each with all three components' values in them (the last slab takes what is left).
/// Fixed by the grid, not by the machine's threads.
fn slabs(cells: [usize; 3]) -> Vec<Vec<usize>> {
    #[cfg(test)]
    let planes = std::env::var("MG_PLANES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(PLANES);
    #[cfg(not(test))]
    let planes = PLANES;
    let [nx, ny, nz] = cells;
    let plane = nx * ny;
    let count = (nz / planes).max(1);
    (0..count)
        .map(|b| {
            let first = b * planes;
            let last = if b + 1 == count { nz } else { first + planes };
            (0..3)
                .flat_map(|c| {
                    let start = c * plane * nz;
                    (start + first * plane)..(start + last * plane)
                })
                .collect()
        })
        .collect()
}

/// The planes across z in each of the smoother's blocks.
const PLANES: usize = 8;

/// The operator the cycle is built on: `matrix` with k₀² i β ε added on the diagonal of every
/// value not fixed at zero, Erlangga, Oosterlee and Vuik's shift (their Eq. 8 with
/// (β₁, β₂) = (1, β), in our e^(−iωt) on the side of the medium's loss and the PMLs').
pub(crate) fn shifted(lattice: &Lattice, eps: &[c64], matrix: &Sparse, shift: f64) -> Sparse {
    use rayon::prelude::*;
    let k2 = lattice.k0 * lattice.k0;
    let add = c64::new(0.0, shift * k2);
    // row by row, each row's columns in the order they are in (sorted), so nothing is sorted
    // again: the shift goes on the diagonal entry, or in its place if the row has none
    let rows: Vec<Vec<(usize, c64)>> = (0..matrix.size())
        .into_par_iter()
        .with_min_len(4096)
        .map(|r| {
            let mut row: Vec<(usize, c64)> = matrix.row(r).collect();
            if shift != 0.0 && !lattice.fixed(r) {
                let extra = add * eps[r];
                match row.binary_search_by_key(&r, |&(c, _)| c) {
                    Ok(k) => row[k].1 += extra,
                    Err(k) => row.insert(k, (r, extra)),
                }
            }
            row
        })
        .collect();
    Sparse::from_rows(rows)
}

/// A level's nodes along one axis: where each lies, in half-steps of the finest grid, the far
/// end (a wall, or the next period's first node) included. Its values of E lie at the nodes
/// (the components across the axis) or halfway between them (the component along it).
#[derive(Clone, Debug)]
struct Line {
    ends: Vec<usize>,
}

impl Line {
    /// The finest grid's `n` cells.
    fn finest(n: usize) -> Line {
        Line {
            ends: (0..=n).map(|m| 2 * m).collect(),
        }
    }

    /// The cells.
    fn cells(&self) -> usize {
        self.ends.len() - 1
    }

    /// The next level's: neighbouring cells merged in pairs where the pair is no longer, in
    /// the stretched coordinate `x` (in steps of the finest grid), than `target`: the next
    /// level's cells outside the PMLs. Inside a PML a cell is |s| times longer in that
    /// coordinate, so it waits for the levels around it to catch up. A line of fewer than 4
    /// cells isn't coarsened.
    fn coarser(&self, x: &[c64], target: f64) -> Line {
        let n = self.cells();
        if n < 4 {
            return self.clone();
        }
        let mut ends = vec![self.ends[0]];
        let mut m = 0;
        while m < n {
            if m + 2 <= n && (x[self.ends[m + 2]] - x[self.ends[m]]).norm() <= target {
                m += 2;
            } else {
                m += 1;
            }
            ends.push(self.ends[m]);
        }
        Line { ends }
    }

    /// For each of `coarse`'s nodes, the far end included, the index of the same node here.
    fn nodes_of(&self, coarse: &Line) -> Vec<usize> {
        coarse
            .ends
            .iter()
            .map(|e| self.ends.partition_point(|f| f < e))
            .collect()
    }

    /// Where value `m` lies: node m, or halfway between nodes m and m + 1 along the axis.
    fn place(&self, m: usize, along: bool) -> usize {
        if along {
            (self.ends[m] + self.ends[m + 1]) / 2
        } else {
            self.ends[m]
        }
    }
}

/// How much longer, in the stretched coordinate, a coarse cell may be than the level's cells
/// outside the PMLs: a PML's cell is |s| times longer than outside, and its neighbours couple
/// |s|² times more weakly along its axis than across it, an anisotropy no point smoother
/// reduces (U. Trottenberg, C. W. Oosterlee, A. Schüller, Multigrid, Academic Press (2001),
/// Section 5.1: semicoarsening). So a PML is coarsened along its axis only once the cells
/// across it are as long.
const ANISOTROPY: f64 = 1.5;

/// The complex coordinate x̃ = ∫ s dx of each axis on the finest grid, at each of its
/// half-steps, in units of the step: the coordinate in which the PML's waves are as smooth as
/// outside it, and the cycle interpolates. Two of them: the stretch of the cells (s at their
/// centres), which the components across the axis see between neighbours, and the stretch of
/// the nodes (s at the nodes), which the component along it sees between neighbours.
struct Stretched {
    across: [Vec<c64>; 3],
    along: [Vec<c64>; 3],
}

impl Stretched {
    fn new(lattice: &Lattice) -> Stretched {
        let map = |a: usize, along: bool| -> Vec<c64> {
            let n = lattice.grid.n(Axis::ALL[a]);
            let mut x = vec![c64::new(0.0, 0.0); 2 * n + 1];
            for p in 0..2 * n {
                let s = if along {
                    lattice.nodes[a][p.div_ceil(2).min(n - 1)]
                } else {
                    lattice.halves[a][p / 2]
                };
                x[p + 1] = x[p] + 0.5 * s;
            }
            x
        };
        Stretched {
            across: [0, 1, 2].map(|a| map(a, false)),
            along: [0, 1, 2].map(|a| map(a, true)),
        }
    }
}

/// Linear interpolation along one axis from the coarse line's values to the fine one's, in the
/// stretched coordinate `x`: for each fine value, the coarse values and their weights. `along`
/// for the component along the axis (between the nodes), else across it (at the nodes);
/// `phase` for a Bloch-periodic axis, e^(ikL) one period on, `None` for walls.
fn interpolate_line(
    fine: &Line,
    coarse: &Line,
    x: &[c64],
    along: bool,
    phase: Option<c64>,
) -> Vec<Vec<(usize, c64)>> {
    let one = c64::new(1.0, 0.0);
    let nf = fine.cells();
    let nc = coarse.cells();
    let period = *coarse.ends.last().unwrap_or(&0);
    // the coarse values' places, with the one before the first and after the last: across a
    // period's end (Bloch) or beyond a wall
    let place = |m: i64| -> i64 {
        if m < 0 {
            coarse.place(nc - 1, along) as i64 - period as i64
        } else if m as usize >= nc {
            coarse.place(0, along) as i64 + period as i64
        } else {
            coarse.place(m as usize, along) as i64
        }
    };
    // the stretched coordinate at a place (a Bloch axis has no stretch: there it is the place)
    let at = |p: i64| -> c64 {
        if phase.is_some() || p < 0 || p as usize >= x.len() {
            c64::new(p as f64, 0.0)
        } else {
            x[p as usize]
        }
    };
    (0..nf)
        .map(|i| {
            let p = fine.place(i, along) as i64;
            // the coarse value at or before p: m in −1..nc
            let mut m = -1i64;
            while m + 1 < nc as i64 && place(m + 1) <= p {
                m += 1;
            }
            if m >= 0 && place(m) == p {
                return vec![(m as usize, one)];
            }
            let (low, high) = (m, m + 1);
            // past the first or last value: across a period's end, or beyond a wall, where
            // the tangential E is zero (odd about it) and the normal E's derivative is
            // (∇ · E = 0 with the tangential E zero), so it extends evenly
            if phase.is_none() && along && (low < 0 || high >= nc as i64) {
                return vec![(low.clamp(0, nc as i64 - 1) as usize, one)];
            }
            let index = |m: i64| -> Option<(usize, c64)> {
                if (0..nc as i64).contains(&m) {
                    return Some((m as usize, one));
                }
                match phase {
                    Some(ph) if m < 0 => Some((nc - 1, ph.inv())),
                    Some(ph) => Some((0, ph)),
                    None => None,
                }
            };
            let t = (at(p) - at(place(low))) / (at(place(high)) - at(place(low)));
            let mut out = Vec::with_capacity(2);
            if let Some((c, ph)) = index(low) {
                out.push((c, (1.0 - t) * ph));
            }
            if let Some((c, ph)) = index(high) {
                out.push((c, t * ph));
            }
            out
        })
        .collect()
}

/// The permittivity each coarse value of E stands for: along its component, the harmonic
/// mean of the fine values in its coarse cell (as each value's own average is harmonic along
/// it), across it the fine values at the same nodes. `nodes` says which fine node each coarse
/// node is, axis by axis.
fn coarse_eps(gf: &Grid3d, gc: &Grid3d, eps: &[c64], nodes: &[Vec<usize>; 3]) -> Vec<c64> {
    (0..gc.unknowns())
        .map(|r| {
            let (component, at) = gc.at(r);
            let a = component.index();
            let mut fine = [0; 3];
            for (b, f) in fine.iter_mut().enumerate() {
                *f = nodes[b][at[b]];
            }
            let cells = nodes[a][at[a]]..nodes[a][at[a] + 1];
            let count = cells.len() as f64;
            let inverse: c64 = cells
                .map(|m| {
                    fine[a] = m;
                    1.0 / eps[gf.index(component, (fine[0], fine[1], fine[2]))]
                })
                .sum();
            count / inverse
        })
        .collect()
}

/// One axis's 1D interpolation weights: for each fine point, its coarse points and weights.
type Weights = Vec<Vec<(usize, c64)>>;

/// The prolongation from the coarse lattice to the fine one, by rows: each component of E
/// interpolated on its own staggered points, linearly along each axis in the stretched
/// coordinate (trilinearly), values fixed at zero left out. Along its own axis a component is
/// interpolated as ε E, the flux density, whose normal component is continuous across an
/// interface where E's jumps; across it, as E, whose tangential components are continuous.
fn prolongation(
    fine: (&Lattice, &[c64]),
    coarse: (&Lattice, &[c64]),
    lines: (&[Line; 3], &[Line; 3]),
    stretched: &Stretched,
) -> Vec<Vec<(usize, c64)>> {
    let ((fine, eps_f), (coarse, eps_c)) = (fine, coarse);
    let (gf, gc) = (fine.grid, coarse.grid);
    let nodes = [0, 1, 2].map(|a| lines.0[a].nodes_of(&lines.1[a]));
    // for each component and axis, the 1D weights
    let tables: Vec<[Weights; 3]> = Axis::ALL
        .iter()
        .map(|&component| {
            Axis::ALL.map(|axis| {
                let a = axis.index();
                let phase = match fine.boundaries.edges(axis) {
                    Edges::Bloch { k } => {
                        Some(c64::from_polar(1.0, k * gf.n(axis) as f64 * gf.step(axis)))
                    }
                    Edges::Pml { .. } => None,
                };
                let along = axis == component;
                let x = if along {
                    &stretched.along[a]
                } else {
                    &stretched.across[a]
                };
                interpolate_line(&lines.0[a], &lines.1[a], x, along, phase)
            })
        })
        .collect();
    #[cfg(test)]
    let flux = std::env::var("MG_PLAIN_INTERP").is_err();
    #[cfg(not(test))]
    let flux = true;
    use rayon::prelude::*;
    (0..gf.unknowns())
        .into_par_iter()
        .with_min_len(4096)
        .map(|r| {
            if fine.fixed(r) {
                return Vec::new();
            }
            let (component, at) = gf.at(r);
            let a = component.index();
            let t = &tables[a];
            let mut out = Vec::with_capacity(8);
            for &(i, wx) in &t[0][at[0]] {
                for &(j, wy) in &t[1][at[1]] {
                    for &(k, wz) in &t[2][at[2]] {
                        let col = gc.index(component, (i, j, k));
                        if coarse.fixed(col) {
                            continue;
                        }
                        let mut w = wx * wy * wz;
                        if flux {
                            // ε E interpolated along the component, on the coarse value's line
                            let c = [i, j, k];
                            let mut f = [0; 3];
                            for b in 0..3 {
                                f[b] = if b == a { at[a] } else { nodes[b][c[b]] };
                            }
                            w *= eps_c[col] / eps_f[gf.index(component, (f[0], f[1], f[2]))];
                        }
                        out.push((col, w));
                    }
                }
            }
            out
        })
        .collect()
}

/// A fine row of A by its number, into the second vector (the first to work in).
type Rows<'a> = dyn Fn(usize, &mut Vec<(usize, c64)>, &mut Vec<(usize, c64)>) + Sync + 'a;

/// Pᵀ A P, coarse row by coarse row on rayon's threads, each row accumulated in one order
/// whatever the threads. A coarse value no fine one reaches gets an identity row. A's rows come
/// from `a`, the stored matrix's or the stencil's.
fn galerkin(a: &Rows<'_>, p: &Transfer, pt: &Transfer) -> Sparse {
    use rayon::prelude::*;
    let coarse = pt.starts.len() - 1;
    let zero = c64::new(0.0, 0.0);
    let rows: Vec<Vec<(usize, c64)>> = (0..coarse)
        .into_par_iter()
        // a few dozen jobs, each with its own accumulator
        .with_min_len((coarse / 64).max(256))
        .map_init(
            || {
                (
                    vec![zero; coarse],
                    vec![usize::MAX; coarse],
                    Vec::new(),
                    (Vec::new(), Vec::new()),
                )
            },
            |(sums, seen, touched, (scratch, row)), c| {
                touched.clear();
                for (r, w) in pt.row(c) {
                    a(r, scratch, row);
                    for &(s, v) in row.iter() {
                        let wv = w * v;
                        for (cc, u) in p.row(s) {
                            if seen[cc] != c {
                                seen[cc] = c;
                                sums[cc] = zero;
                                touched.push(cc);
                            }
                            sums[cc] += wv * u;
                        }
                    }
                }
                if seen[c] != c {
                    seen[c] = c;
                    sums[c] = c64::new(1.0, 0.0);
                    touched.push(c);
                }
                touched.sort_unstable();
                touched.iter().map(|&cc| (cc, sums[cc])).collect()
            },
        )
        .collect();
    Sparse::from_rows(rows)
}

/// Row `r` of [`shifted`]'s matrix for Shin and Fan's operator on `lattice`, from its stencil:
/// [`Lattice::row_into`]'s entries summed by column as the assembled matrix sums them, and the
/// shift added on the diagonal as [`shifted`] adds it, so the same bits. Into `out`; `scratch`
/// to work in.
fn stencil_row(
    lattice: &Lattice,
    eps: &[c64],
    shift: f64,
    r: usize,
    scratch: &mut Vec<(usize, c64)>,
    out: &mut Vec<(usize, c64)>,
) {
    lattice.row_into(eps, -1.0, r, scratch);
    scratch.sort_unstable_by_key(|&(col, _)| col);
    out.clear();
    for &(col, v) in scratch.iter() {
        match out.last_mut() {
            Some((c, acc)) if *c == col => *acc += v,
            _ => out.push((col, v)),
        }
    }
    if shift != 0.0 && !lattice.fixed(r) {
        let k2 = lattice.k0 * lattice.k0;
        let extra = c64::new(0.0, shift * k2) * eps[r];
        match out.binary_search_by_key(&r, |&(c, _)| c) {
            Ok(k) => out[k].1 += extra,
            Err(k) => out.insert(k, (r, extra)),
        }
    }
}

/// [`shifted`]'s matrix for Shin and Fan's operator on `lattice`, assembled from its stencil.
fn assembled(lattice: &Lattice, eps: &[c64], shift: f64) -> Sparse {
    use rayon::prelude::*;
    Sparse::from_rows(
        (0..eps.len())
            .into_par_iter()
            .with_min_len(4096)
            .map_init(
                || (Vec::new(), Vec::new()),
                |(scratch, row), r| {
                    stencil_row(lattice, eps, shift, r, scratch, row);
                    row.clone()
                },
            )
            .collect(),
    )
}

/// The volume of each value of E's cell on `lattice`: its edge's length times its dual face's
/// area, each in the PMLs' stretched coordinate (µm³, complex in a PML): what the finite
/// differences' rows stand for, and what a residual is weighed by when it is restricted to a
/// grid discretized again.
fn volumes(lattice: &Lattice) -> Vec<c64> {
    let g = lattice.grid;
    (0..g.unknowns())
        .map(|r| {
            let (component, at) = g.at(r);
            let a = component.index();
            (0..3)
                .map(|b| {
                    let s = if b == a {
                        lattice.halves[b][at[b]]
                    } else {
                        lattice.nodes[b][at[b]]
                    };
                    s * g.step(Axis::ALL[b])
                })
                .product()
        })
        .collect()
}

/// The lattice of the coarser `lines` discretized again: each cell as long, and each node's dual
/// cell as wide, as the finest cells they take in, in the PMLs' stretched coordinate. Each axis'
/// step is its period over its cells (what a Bloch side's phase takes), and the stretches carry
/// the cells' lengths over it.
fn rediscretized_lattice(finest: &Lattice, lines: &[Line; 3], stretched: &Stretched) -> Lattice {
    let g = finest.grid;
    let mut grid = g;
    let mut nodes: [Vec<c64>; 3] = Default::default();
    let mut halves: [Vec<c64>; 3] = Default::default();
    for (a, axis) in Axis::ALL.into_iter().enumerate() {
        let line = &lines[a];
        let n = line.cells();
        let step = g.step(axis) * g.n(axis) as f64 / n as f64;
        // a length in the finest steps, over this level's step
        let unit = g.step(axis) / step;
        let bloch = matches!(finest.boundaries.edges(axis), Edges::Bloch { .. });
        let (across, along) = (&stretched.across[a], &stretched.along[a]);
        halves[a] = (0..n)
            .map(|m| (across[line.ends[m + 1]] - across[line.ends[m]]) * unit)
            .collect();
        let centre = |m: usize| line.place(m, true);
        nodes[a] = (0..n)
            .map(|m| {
                let length = if m > 0 {
                    along[centre(m)] - along[centre(m - 1)]
                } else if bloch {
                    // across the period's end, where nothing is stretched
                    c64::new((centre(0) + line.ends[n] - centre(n - 1)) as f64 / 2.0, 0.0)
                } else {
                    // the wall's node: its dual cell's half inside, twice
                    2.0 * (along[centre(0)] - along[0])
                };
                length * unit
            })
            .collect();
        match axis {
            Axis::X => (grid.nx, grid.dx) = (n, step),
            Axis::Y => (grid.ny, grid.dy) = (n, step),
            Axis::Z => (grid.nz, grid.dz) = (n, step),
        }
    }
    Lattice {
        grid,
        boundaries: finest.boundaries,
        k0: finest.k0,
        nodes,
        halves,
    }
}

/// The permittivity of each coarse value of E for an operator discretized again on `coarse`'s
/// lines: harmonic along its component over the fine cells its cell takes (as each value's own
/// average is), and across it arithmetic over the fine nodes of its dual cell, each weighed by
/// the coarse hat function there (1 at the coarse node, falling to 0 at the next).
fn averaged_eps(
    gf: &Grid3d,
    gc: &Grid3d,
    eps: &[c64],
    lines: (&[Line; 3], &[Line; 3]),
    bloch: [bool; 3],
) -> Vec<c64> {
    use rayon::prelude::*;
    let (fine, coarse) = lines;
    let nodes = [0, 1, 2].map(|a| fine[a].nodes_of(&coarse[a]));
    // across axis b, around coarse node m: each fine node and its weight
    let around = |b: usize, m: usize| -> Vec<(usize, f64)> {
        let (f, c) = (&fine[b], &coarse[b]);
        let (nf, nc) = (f.cells(), c.cells());
        let period = c.ends[nc] as f64;
        let at = c.ends[m] as f64;
        let mut out = Vec::new();
        // below: back to the coarse node before, across the period's end if it wraps
        if m > 0 || bloch[b] {
            let (from, low) = if m > 0 {
                (nodes[b][m - 1], c.ends[m - 1] as f64)
            } else {
                (nodes[b][nc - 1], c.ends[nc - 1] as f64 - period)
            };
            let width = at - low;
            let mut q = from + 1;
            loop {
                let index = q % nf;
                let place = if m == 0 && q < nf {
                    f.ends[q] as f64 - period
                } else {
                    f.ends[index] as f64
                };
                if place >= at {
                    break;
                }
                out.push((index, 1.0 - (at - place) / width));
                q += 1;
            }
        }
        out.push((nodes[b][m], 1.0));
        // above: up to the coarse node after (or the wall, or the period's end)
        let high = c.ends[m + 1] as f64;
        let width = high - at;
        for q in nodes[b][m] + 1..nodes[b][m + 1].min(nf) {
            out.push((q, 1.0 - (f.ends[q] as f64 - at) / width));
        }
        out
    };
    (0..gc.unknowns())
        .into_par_iter()
        .with_min_len(4096)
        .map(|r| {
            let (component, at) = gc.at(r);
            let a = component.index();
            let (b, c) = match a {
                0 => (1, 2),
                1 => (2, 0),
                _ => (0, 1),
            };
            let cells = nodes[a][at[a]]..nodes[a][at[a] + 1];
            let count = cells.len() as f64;
            let (mut sum, mut weights) = (c64::new(0.0, 0.0), 0.0);
            for (fb, wb) in around(b, at[b]) {
                for (fc, wc) in around(c, at[c]) {
                    let mut fine = [0; 3];
                    fine[b] = fb;
                    fine[c] = fc;
                    let inverse: c64 = cells
                        .clone()
                        .map(|m| {
                            fine[a] = m;
                            1.0 / eps[gf.index(component, (fine[0], fine[1], fine[2]))]
                        })
                        .sum();
                    sum += wb * wc * count / inverse;
                    weights += wb * wc;
                }
            }
            sum / weights
        })
        .collect()
}

/// One sweep of the finest level's damped Jacobi without its matrix, as it acts on an error `e`
/// of A x = b: e − ω D⁻¹ A e, A Shin and Fan's operator on `lattice` shifted as `options` says,
/// D its diagonal from the stencil. For the checks of the smoother against its Fourier symbol.
pub(crate) fn jacobi_on_error(
    lattice: &Lattice,
    eps: &[c64],
    options: &Multigrid,
) -> impl Fn(&[c64]) -> Vec<c64> {
    let stage = Hierarchy::stencil_stage(lattice, eps, options);
    move |e| {
        let r = stage.apply(e, false);
        let d = stage.smooth(&r, false);
        e.iter().zip(&d).map(|(p, q)| p - q).collect()
    }
}

impl Hierarchy {
    /// The hierarchy for the fine operator `matrix` on `lattice`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if no steps of smoothing are asked for, or if a level's ILU(0) or
    /// the coarsest factorization fails.
    #[cfg(test)]
    pub(crate) fn new(
        lattice: &Lattice,
        eps: &[c64],
        matrix: Sparse,
        options: Multigrid,
    ) -> Result<Hierarchy> {
        let solver = crate::backend::direct(&crate::backend::Choice::Auto)?;
        Self::new_on(lattice, eps, matrix, options, &solver)
    }

    /// [`Hierarchy::new`], its coarsest level factorized by `solver`.
    ///
    /// # Errors
    ///
    /// As [`Hierarchy::new`].
    pub(crate) fn new_on(
        lattice: &Lattice,
        eps: &[c64],
        matrix: Sparse,
        options: Multigrid,
        solver: &Arc<dyn DirectSolver>,
    ) -> Result<Hierarchy> {
        Self::build(lattice, eps, Some(matrix), options, solver)
    }

    /// The hierarchy for Shin and Fan's operator on `lattice`, shifted as `options` says,
    /// without its matrix: the finest level from the stencil, smoothed by damped Jacobi
    /// (`options.jacobi`), the next level's operator made as `options.coarse` says, and the
    /// coarsest factorized by `solver`.
    ///
    /// # Errors
    ///
    /// As [`Hierarchy::new`], and [`Error::InvalidValue`] for a Jacobi without sweeps or with an
    /// ω that isn't positive and finite.
    pub(crate) fn from_stencil_on(
        lattice: &Lattice,
        eps: &[c64],
        options: Multigrid,
        solver: &Arc<dyn DirectSolver>,
    ) -> Result<Hierarchy> {
        let j = options.jacobi;
        if j.pre + j.post == 0 || !(j.omega.is_finite() && j.omega > 0.0) {
            return Err(Error::invalid(
                "multigrid",
                format!(
                    "the finest level's Jacobi needs sweeps and an ω > 0, got {} and {} sweeps, ω = {}",
                    j.pre, j.post, j.omega
                ),
            ));
        }
        Self::build(lattice, eps, None, options, solver)
    }

    /// The hierarchy, its finest level by `matrix` or, without one, by the stencil.
    fn build(
        lattice: &Lattice,
        eps: &[c64],
        matrix: Option<Sparse>,
        options: Multigrid,
        solver: &Arc<dyn DirectSolver>,
    ) -> Result<Hierarchy> {
        if options.pre + options.post == 0 {
            return Err(Error::invalid("multigrid", "needs some smoothing"));
        }
        if options.restart == 0 {
            return Err(Error::invalid("multigrid", "needs a restart of at least 1"));
        }
        let mut levels = Vec::new();
        let mut fine = lattice.clone();
        let g = lattice.grid;
        let mut lines = [Line::finest(g.nx), Line::finest(g.ny), Line::finest(g.nz)];
        let stretched = Stretched::new(lattice);
        // the level's matrix; None for the finest without one
        let mut a = matrix;
        // the permittivity the transfers take, and, rediscretized, the operators
        let mut eps = eps.to_vec();
        let mut eps_operator = eps.clone();
        let mut shapes = Vec::new();
        let bloch =
            Axis::ALL.map(|axis| matches!(lattice.boundaries.edges(axis), Edges::Bloch { .. }));
        loop {
            let n = fine.grid.unknowns();
            #[cfg(test)]
            let anisotropy = std::env::var("MG_ANISOTROPY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(ANISOTROPY);
            #[cfg(not(test))]
            let anisotropy = ANISOTROPY;
            let target = (1 << (levels.len() + 1)) as f64 * anisotropy;
            let next_lines = [0, 1, 2].map(|a| lines[a].coarser(&stretched.across[a], target));
            let cells = |l: &[Line; 3]| l.iter().map(Line::cells).product::<usize>();
            if n <= options.coarsest || cells(&next_lines) == cells(&lines) {
                let a = match a {
                    Some(a) => a,
                    None => assembled(&fine, &eps_operator, options.shift),
                };
                let entries: Vec<_> = (0..n)
                    .flat_map(|r| {
                        a.row(r)
                            .map(move |(c, v)| faer::sparse::Triplet::new(r, c, v))
                            .collect::<Vec<_>>()
                    })
                    .collect();
                let one = rayon::ThreadPoolBuilder::new()
                    .num_threads(1)
                    .build()
                    .map_err(|e| Error::invalid("multigrid", e.to_string()))?;
                #[cfg(test)]
                let clock = std::time::Instant::now();
                let lu = one.install(|| -> Result<Box<dyn Factorization>> {
                    let columns = Columns::new(n, &entries)?;
                    let matrix = columns.matrix(Form::General, None)?;
                    let analysis = solver.analyse(&matrix)?.ok_or_else(|| {
                        Error::invalid(
                            "multigrid",
                            format!(
                                "the direct solver {} can't factorize the coarsest level",
                                solver.capabilities().name
                            ),
                        )
                    })?;
                    analysis.factorize(&matrix)
                })?;
                #[cfg(test)]
                if std::env::var("MG_TIMES").is_ok() {
                    println!(
                        "MG times: coarsest, {n} rows, {} entries: factorized in {:?}",
                        a.nonzeros(),
                        clock.elapsed()
                    );
                }
                shapes.push([0, 1, 2].map(|a| lines[a].cells()));
                return Ok(Hierarchy {
                    levels,
                    shapes,
                    lu,
                    coarsest: a,
                    smoothers: std::sync::OnceLock::new(),
                    one: std::panic::AssertUnwindSafe(one),
                    options,
                });
            }
            let gc = Grid3d {
                nx: next_lines[0].cells(),
                ny: next_lines[1].cells(),
                nz: next_lines[2].cells(),
                ..fine.grid
            };
            #[cfg(test)]
            let clock = std::time::Instant::now();
            let rediscretized = options.coarse == Coarse::Rediscretized;
            // the next level's lattice: the same one over coarser cells for Galerkin's operators,
            // whose stretches nothing reads, only its walls and, for the transfers' phases, its
            // period (each axis' step its period over its cells: with the finest step, a Bloch
            // side's phase two levels down was e^(ik n h) for n cells fewer than the finest's);
            // discretized again
            let coarse = if rediscretized {
                rediscretized_lattice(lattice, &next_lines, &stretched)
            } else {
                let mut gc = gc;
                gc.dx = g.dx * g.nx as f64 / gc.nx as f64;
                gc.dy = g.dy * g.ny as f64 / gc.ny as f64;
                gc.dz = g.dz * g.nz as f64 / gc.nz as f64;
                Lattice::new(gc, fine.boundaries, fine.k0)
            };
            let gc = coarse.grid;
            let nodes = [0, 1, 2].map(|a| lines[a].nodes_of(&next_lines[a]));
            let eps_c = coarse_eps(&fine.grid, &gc, &eps, &nodes);
            let prolongation = prolongation(
                (&fine, &eps),
                (&coarse, &eps_c),
                (&lines, &next_lines),
                &stretched,
            );
            let prolongation = Transfer::from_rows(&prolongation);
            let restriction = prolongation.transpose(gc.unknowns());
            #[cfg(test)]
            let t_transfer = clock.elapsed();
            let (next, eps_next, scaling) = if rediscretized {
                let eps_next =
                    averaged_eps(&fine.grid, &gc, &eps_operator, (&lines, &next_lines), bloch);
                let next = assembled(&coarse, &eps_next, options.shift);
                // each coarse value's share of the fine volumes, Pᵀ D_f 1, so that R's rows sum
                // to one: on a grid coarsened unevenly (the PMLs' cells merge later) a cell-
                // centred hat's integral isn't its cell's length
                let fine_volumes = volumes(&fine);
                let shares = restriction.apply(&fine_volumes);
                let inverse: Vec<c64> = shares
                    .iter()
                    .map(|v| {
                        if v.norm() > 0.0 {
                            1.0 / v
                        } else {
                            c64::new(1.0, 0.0)
                        }
                    })
                    .collect();
                (next, eps_next, Some((fine_volumes, inverse)))
            } else {
                let next = match &a {
                    Some(a) => galerkin(
                        &|r, _, row: &mut Vec<(usize, c64)>| {
                            row.clear();
                            row.extend(a.row(r));
                        },
                        &prolongation,
                        &restriction,
                    ),
                    None => {
                        let (fine, eps) = (&fine, &eps_operator);
                        galerkin(
                            &|r, scratch, row| {
                                stencil_row(fine, eps, options.shift, r, scratch, row)
                            },
                            &prolongation,
                            &restriction,
                        )
                    }
                };
                (next, eps_c.clone(), None)
            };
            #[cfg(test)]
            let t_galerkin = clock.elapsed();
            let shape = [0, 1, 2].map(|a| lines[a].cells());
            let stage = match a {
                Some(a) => {
                    let smoother = BlockIlu0::new(&a, slabs(shape))?;
                    Stage::Stored {
                        matrix: a,
                        smoother,
                    }
                }
                None => Self::stencil_stage(&fine, &eps_operator, &options),
            };
            #[cfg(test)]
            if std::env::var("MG_TIMES").is_ok() {
                println!(
                    "MG times: level {shape:?}, {} rows, {} entries: transfer {:?}, coarse operator {:?}, smoother {:?}",
                    n,
                    stage.matrix().map_or(0, Sparse::nonzeros),
                    t_transfer,
                    t_galerkin - t_transfer,
                    clock.elapsed() - t_galerkin
                );
            }
            shapes.push(shape);
            levels.push(Level {
                stage,
                prolongation,
                restriction,
                scaling,
            });
            a = Some(next);
            fine = coarse;
            lines = next_lines;
            eps = eps_c;
            eps_operator = eps_next;
        }
    }

    /// The finest level without its matrix: Shin and Fan's shifted operator from its stencil,
    /// and ω over its diagonal, the diagonal from the same rows the stored matrix would have.
    fn stencil_stage(lattice: &Lattice, eps: &[c64], options: &Multigrid) -> Stage {
        use rayon::prelude::*;
        let j = options.jacobi;
        #[cfg(test)]
        let j = Jacobi {
            omega: std::env::var("MG_OMEGA")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(j.omega),
            pre: std::env::var("MG_JPRE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(j.pre),
            post: std::env::var("MG_JPOST")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(j.post),
        };
        let jacobi = (0..eps.len())
            .into_par_iter()
            .with_min_len(4096)
            .map_init(
                || (Vec::new(), Vec::new()),
                |(scratch, row), r| {
                    stencil_row(lattice, eps, options.shift, r, scratch, row);
                    let d = row
                        .iter()
                        .find(|&&(c, _)| c == r)
                        .map_or(c64::new(1.0, 0.0), |&(_, v)| v);
                    j.omega / d
                },
            )
            .collect();
        Stage::Stencil {
            operator: MatrixFree::shin_fan(lattice, eps, options.shift),
            jacobi,
            pre: j.pre,
            post: j.post,
        }
    }

    /// Each level's cells along x, y and z, finest first.
    pub(crate) fn shapes(&self) -> &[[usize; 3]] {
        &self.shapes
    }

    /// The steps of GMRES between restarts.
    pub(crate) fn restart(&self) -> usize {
        self.options.restart
    }

    /// The cycle as a backend takes it ([`crate::backend::MultigridCycle`]): each level's
    /// smoother factorized as one pair of factors the first time (as much memory again as the
    /// smoothers), the rest borrowed.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a cycle a backend can't take: a level without its matrix, a
    /// rediscretized one (its restriction isn't Pᵀ), or ILU(0) by Jacobi sweeps.
    pub(crate) fn cycle_for_backend(&self) -> Result<crate::backend::MultigridCycle<'_>> {
        if self.levels.iter().any(|l| l.scaling.is_some()) {
            return Err(Error::invalid(
                "multigrid",
                "a backend takes Galerkin's coarse operators, restricted by Pᵀ",
            ));
        }
        let stored: Vec<(&Sparse, &BlockIlu0)> = self
            .levels
            .iter()
            .map(|l| match &l.stage {
                Stage::Stored { matrix, smoother } => Ok((matrix, smoother)),
                Stage::Stencil { .. } => Err(Error::invalid(
                    "multigrid",
                    "a backend takes each level's matrix: the finest has none",
                )),
            })
            .collect::<Result<_>>()?;
        let smoothers = self
            .smoothers
            .get_or_init(|| stored.iter().map(|(_, s)| s.factors()).collect())
            .as_ref()
            .ok_or_else(|| Error::invalid("multigrid", "a smoother by Jacobi sweeps"))?;
        let levels = self
            .levels
            .iter()
            .zip(&stored)
            .zip(smoothers)
            .map(
                |((l, (matrix, _)), smoother)| crate::backend::MultigridLevel {
                    operator: matrix.rows(),
                    smoother,
                    restriction: l.restriction.rows(l.prolongation.starts.len() - 1),
                    prolongation: l.prolongation.rows(l.restriction.starts.len() - 1),
                },
            )
            .collect();
        Ok(crate::backend::MultigridCycle {
            shape: self.options.shape,
            pre: self.options.pre,
            post: self.options.post,
            restart: self.options.restart,
            levels,
            coarsest: self.coarsest.rows(),
            lu: self.lu.as_ref(),
            one: &self.one,
        })
    }

    /// One cycle from `level` down, for A x = b (or Aᵀ x = b, the transposed cycle), from
    /// x = 0.
    fn cycle(&self, level: usize, b: &[c64], transpose: bool, shape: CycleShape) -> Vec<c64> {
        if level == self.levels.len() {
            // a backend's solve that fails gives NaNs, which the Krylov solver reports as a
            // solve that didn't converge
            return self.one.install(|| {
                if transpose {
                    self.lu.solve_transpose(b)
                } else {
                    self.lu.solve(b)
                }
                .unwrap_or_else(|_| super::nans(b.len()))
            });
        }
        let l = &self.levels[level];
        // b − A x, or b itself while x is still zero
        let residual = |x: Option<&[c64]>| -> Vec<c64> {
            match x {
                None => b.to_vec(),
                Some(x) => {
                    let mut r = l.stage.apply(x, transpose);
                    use rayon::prelude::*;
                    r.par_iter_mut()
                        .with_min_len(16_384)
                        .zip(b)
                        .for_each(|(rk, bk)| *rk = bk - *rk);
                    r
                }
            }
        };
        let smooth = |x: &mut Option<Vec<c64>>| {
            let r = residual(x.as_deref());
            let d = l.stage.smooth(&r, transpose);
            match x {
                None => *x = Some(d),
                Some(x) => add_to(x, &d),
            }
        };
        // the transposed cycle is the cycle's transpose: its steps in reverse order, each
        // transposed (post-smoothing first, the coarse corrections reversed)
        let (pre, post) = l.stage.steps(&self.options);
        let (pre, post) = if transpose { (post, pre) } else { (pre, post) };
        let mut x: Option<Vec<c64>> = None;
        for _ in 0..pre {
            smooth(&mut x);
        }
        let mut corrections = match shape {
            CycleShape::V => vec![CycleShape::V],
            CycleShape::W => vec![CycleShape::W, CycleShape::W],
            CycleShape::F => vec![CycleShape::F, CycleShape::V],
        };
        if transpose {
            corrections.reverse();
        }
        for inner in corrections {
            let r = residual(x.as_deref());
            let coarse = l.restrict(&r, transpose);
            let correction = self.cycle(level + 1, &coarse, transpose, inner);
            let fine = l.prolong(&correction, transpose);
            match &mut x {
                None => x = Some(fine),
                Some(x) => add_to(x, &fine),
            }
        }
        for _ in 0..post {
            smooth(&mut x);
        }
        x.unwrap_or_else(|| vec![c64::new(0.0, 0.0); b.len()])
    }
}

impl Preconditioner for Hierarchy {
    fn solve(&self, v: &[c64]) -> Vec<c64> {
        self.cycle(0, v, false, self.options.shape)
    }

    fn solve_transpose(&self, v: &[c64]) -> Vec<c64> {
        self.cycle(0, v, true, self.options.shape)
    }
}

#[cfg(test)]
#[path = "multigrid_tests.rs"]
mod tests;
