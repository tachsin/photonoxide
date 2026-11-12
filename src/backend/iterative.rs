//! Iterative backends: QMR, and GMRES with photonoxide's multigrid, run elsewhere (a GPU), on
//! the same recurrences as photonoxide's own.
//!
//! For GMRES with multigrid, photonoxide builds the cycle on the CPU (each level's operator,
//! smoother factors and transfers, and the coarsest level's factorization) and hands it over as
//! a [`MultigridCycle`]; the backend runs the restarted GMRES and the cycle. Without the finest
//! matrix, the finest level and GMRES's operator are given as their [`Stencil`] instead, if the
//! backend runs them ([`IterativeSolver::runs_multigrid_stencil`]). A backend may keep what it
//! made of a hierarchy between solves ([`IterativeSolver::prepare_multigrid`]): an S-matrix's
//! ports and a recycled sweep's solves on one solver then copy it up and analyse it once.
//!
//! For QMR, photonoxide keeps what surrounds a run: the restarts after a breakdown of the Lanczos
//! process, and the tightened tolerance of QMR on a symmetric similar matrix
//! ([`crate::fdfd::IterativeSolver3d`]). A backend does one run, from x₀ = 0 to the tolerance
//! or a breakdown, as `backend::krylov` does it: R. W. Freund, N. M. Nachtigal, Numer. Math. 60,
//! 315 (1991), doi:10.1007/BF01385726, without look-ahead; on a complex symmetric matrix, R. W.
//! Freund, SIAM J. Sci. Stat. Comput. 13, 425 (1992), doi:10.1137/0913023.

use std::sync::{Arc, OnceLock, RwLock};

use num_complex::Complex64 as c64;

use super::krylov::{Convergence, Stopping};
use super::{Capabilities, Choice, Listed, Matrix, OWN, check_name, invalid};
use crate::Result;

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

/// How one run of QMR ended.
#[derive(Clone, Debug, PartialEq)]
pub enum QmrRun {
    /// At the tolerance: the solution, and how it went (the true residual at the end).
    Done(Vec<c64>, Convergence),
    /// At a (near-)breakdown, |w_nᵀ v_n| below 1e-14 for unit vectors: the iterate so far, the
    /// updated residual's history, and |w_nᵀ v_n|. The caller restarts from the iterate.
    Broken(Vec<c64>, Vec<f64>, f64),
}

/// A backend that runs QMR.
pub trait IterativeSolver: Send + Sync {
    /// What it is and can do ([`Capabilities::symmetric`]: it runs the complex symmetric QMR).
    fn capabilities(&self) -> Capabilities;

    /// One run of QMR on `matrix` from x₀ = 0 for `b`, to `stopping`'s relative residual
    /// (checked on the true residual) or a breakdown: the complex symmetric QMR, one product
    /// an iteration, for a [`super::Form::Symmetric`] matrix; the general one, with Aᵀ's
    /// products too, otherwise.
    ///
    /// # Errors
    ///
    /// The backend's, and [`crate::Error::InvalidValue`] if the tolerance isn't reached within
    /// the iterations allowed or the process ends short of it, as photonoxide's own QMR.
    fn qmr_run(&self, matrix: &Matrix<'_>, b: &[c64], stopping: Stopping) -> Result<QmrRun>;

    /// One run of the general QMR on A M⁻¹, M = LU from `ilu` (photonoxide's ILU(0) of
    /// `matrix`), from y₀ = 0 for `b`: the caller takes x = M⁻¹ y. Each iteration one product with
    /// A and one with Aᵀ, two triangular solves with M and two with Mᵀ.
    ///
    /// # Errors
    ///
    /// As [`IterativeSolver::qmr_run`]; by default, that the backend doesn't run it.
    fn qmr_run_ilu(
        &self,
        matrix: &Matrix<'_>,
        ilu: &IluFactors,
        b: &[c64],
        stopping: Stopping,
    ) -> Result<QmrRun> {
        let _ = (matrix, ilu, b, stopping);
        Err(invalid(format!(
            "{} doesn't run QMR with ILU(0)",
            self.capabilities().name
        )))
    }

    /// Whether it runs [`IterativeSolver::gmres_multigrid`]. By default, no.
    fn runs_multigrid(&self) -> bool {
        false
    }

    /// Restarted GMRES on A M⁻¹ from x₀ = 0 for `b`, M⁻¹ one `cycle` of photonoxide's
    /// multigrid, to `stopping`'s relative residual ‖b − A x‖ / ‖b‖: the solution x (= M⁻¹ u),
    /// and how it went. As photonoxide's own (Y. Saad, *Iterative Methods for Sparse Linear
    /// Systems*, 2nd ed., SIAM (2003), doi:10.1137/1.9780898718003, Algorithm 9.5 restarted as
    /// Algorithm 6.11): every [`MultigridCycle::restart`] steps, or at the tolerance, x is updated
    /// and the true residual b − A x computed afresh, to stop on or restart from; the Arnoldi
    /// basis by modified Gram–Schmidt, the least-squares problem by complex Givens rotations as
    /// each column comes, the history the residual's norm they give at each step.
    ///
    /// # Errors
    ///
    /// The backend's, and [`crate::Error::InvalidValue`] if the tolerance isn't reached within
    /// the iterations allowed; by default, that the backend doesn't run it.
    fn gmres_multigrid(
        &self,
        matrix: RowMatrix<'_>,
        cycle: &MultigridCycle<'_>,
        b: &[c64],
        stopping: Stopping,
    ) -> Result<(Vec<c64>, Convergence)> {
        let _ = (matrix, cycle, b, stopping);
        Err(invalid(format!(
            "{} doesn't run GMRES with multigrid",
            self.capabilities().name
        )))
    }

    /// Whether it also runs the multigrid without its finest matrix: a cycle whose finest level
    /// is a [`Stencil`] smoothed by damped Jacobi ([`MultigridCycle::stencil`]), restrictions
    /// weighted as [`MultigridLevel::scaling`] says (coarse levels discretized again), and GMRES
    /// on a stencil ([`Fine::Stencil`]); through [`IterativeSolver::prepare_multigrid`]. By
    /// default, no: such a cycle isn't given to it.
    fn runs_multigrid_stencil(&self) -> bool {
        false
    }

    /// What it makes of GMRES's operator `fine` and the `cycle` before a solve (on a GPU: their
    /// copies there and the triangular factors' analysis), kept for the solves that follow on
    /// the same hierarchy, as a [`super::DirectSolver`]'s analysis and factorization are: each
    /// solve then runs [`PreparedMultigrid::gmres`] as [`IterativeSolver::gmres_multigrid`] runs
    /// one. `None` (the default): it keeps nothing, and each solve calls
    /// [`IterativeSolver::gmres_multigrid`] with the matrix, which a stencil hasn't.
    ///
    /// # Errors
    ///
    /// The backend's: what it can't take, and what doesn't fit.
    fn prepare_multigrid(
        &self,
        fine: Fine<'_>,
        cycle: &MultigridCycle<'_>,
    ) -> Result<Option<Box<dyn PreparedMultigrid>>> {
        let _ = (fine, cycle);
        Ok(None)
    }
}

/// A hierarchy as a backend keeps it between solves ([`IterativeSolver::prepare_multigrid`]).
pub trait PreparedMultigrid: Send + Sync {
    /// Restarted GMRES from x₀ = 0 for `b`, as [`IterativeSolver::gmres_multigrid`], on what was
    /// prepared; `cycle` is the one it was prepared from, for its coarsest solves
    /// ([`MultigridCycle::coarsest_solve`]).
    ///
    /// # Errors
    ///
    /// As [`IterativeSolver::gmres_multigrid`], and for a cycle of another size than the one
    /// prepared.
    fn gmres(
        &self,
        cycle: &MultigridCycle<'_>,
        b: &[c64],
        stopping: Stopping,
    ) -> Result<(Vec<c64>, Convergence)>;
}

/// The operator GMRES multiplies by, as a backend is given it: the finest level's matrix, or,
/// without it, its stencil (the unshifted operator: [`Stencil::k0_squared`] is real).
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Fine<'a> {
    /// By rows.
    Matrix(RowMatrix<'a>),
    /// Without its matrix.
    Stencil(Stencil<'a>),
}

impl Fine<'_> {
    /// Its rows.
    pub fn rows(&self) -> usize {
        match self {
            Fine::Matrix(m) => m.rows(),
            Fine::Stencil(s) => s.unknowns(),
        }
    }

    /// A v, on one thread.
    pub fn apply(&self, v: &[c64]) -> Vec<c64> {
        match self {
            Fine::Matrix(m) => m.apply(v),
            Fine::Stencil(s) => s.apply(v),
        }
    }
}

/// Shin and Fan's operator on Yee's grid without its matrix, as a backend takes it: what
/// `fdfd::three`'s matrix-free product reads (docs/methods/multigrid-stencil.md), one value of
/// E per cell and component, laid out component by component (x, y, z), then k, j, i with i
/// fastest. [`Stencil::apply`] is the product, A v = k₀²(1 + iβ) ε v − ∇ × ∇ × v +
/// ∇(ε⁻¹ ∇ · (ε v)), two passes over the grid, E to H and to the nodes' potential, and back:
///
/// - **E, masked:** a value tangential to a wall at an axis' low end (E's component c at index 0
///   along an axis a ≠ c that has walls) is held at zero: its row is the identity's, and it
///   enters no difference.
/// - **H = ∇ × E** at the faces, each difference forward along its axis times
///   [`Stencil::at_halves`] there (1/(s h) halfway after the node); **φ = ε⁻¹ ∇ · (ε E)** at the
///   nodes, each difference backward times [`Stencil::at_nodes`] there, times
///   [`Stencil::node_inverse`].
/// - **A v = k₀² ε E − ∇ × H + ∇φ** on each value not held at zero: ∇ × H's differences backward
///   times [`Stencil::at_nodes`], ∇φ's forward times [`Stencil::at_halves`].
/// - **Beyond an axis' end:** zero with walls; with a Bloch-periodic axis, the value across the
///   high end times [`Stencil::phase`], across the low end times its inverse.
#[derive(Clone, Copy)]
pub struct Stencil<'a> {
    pub(crate) cells: [usize; 3],
    pub(crate) epsilon: &'a [c64],
    pub(crate) node_inverse: &'a [c64],
    pub(crate) k0_squared: c64,
    pub(crate) at_nodes: [&'a [c64]; 3],
    pub(crate) at_halves: [&'a [c64]; 3],
    pub(crate) phases: [Option<c64>; 3],
}

impl std::fmt::Debug for Stencil<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stencil")
            .field("cells", &self.cells)
            .field("k0_squared", &self.k0_squared)
            .field("phases", &self.phases)
            .finish_non_exhaustive()
    }
}

impl<'a> Stencil<'a> {
    /// The cells along x, y and z.
    pub fn cells(&self) -> [usize; 3] {
        self.cells
    }

    /// The unknowns, three a cell.
    pub fn unknowns(&self) -> usize {
        3 * self.cells.iter().product::<usize>()
    }

    /// ε at each value of E.
    pub fn epsilon(&self) -> &'a [c64] {
        self.epsilon
    }

    /// ε⁻¹ at each node (a cell's low corner, indexed as E's x component), the mean of the ε the
    /// node's divergence takes; zero on a wall, whose potential the gradient leaves out.
    pub fn node_inverse(&self) -> &'a [c64] {
        self.node_inverse
    }

    /// k₀²(1 + iβ): β the multigrid's complex shift on its finest level, 0 for the operator GMRES
    /// multiplies by.
    pub fn k0_squared(&self) -> c64 {
        self.k0_squared
    }

    /// 1/(s h) at each node along `axis` (0, 1, 2 for x, y, z): s the PML's stretch there, h the
    /// step.
    pub fn at_nodes(&self, axis: usize) -> &'a [c64] {
        self.at_nodes[axis]
    }

    /// 1/(s h) halfway after each node along `axis`.
    pub fn at_halves(&self, axis: usize) -> &'a [c64] {
        self.at_halves[axis]
    }

    /// The phase a field takes across `axis`' high end if it is Bloch-periodic; `None` for walls.
    pub fn phase(&self, axis: usize) -> Option<c64> {
        self.phases[axis]
    }

    /// Whether E's `component` at (i, j, k) is held at zero, tangential to a wall.
    pub fn fixed(&self, component: usize, at: [usize; 3]) -> bool {
        (0..3).any(|a| a != component && at[a] == 0 && self.phases[a].is_none())
    }

    /// A v, on one thread, each value's sum in the order the type's docs give it: the product a
    /// backend reproduces (to rounding: a GPU may fuse its multiplications and additions).
    pub fn apply(&self, v: &[c64]) -> Vec<c64> {
        let [nx, ny, nz] = self.cells;
        let cells = nx * ny * nz;
        let zero = c64::new(0.0, 0.0);
        let index = |c: usize, i: usize, j: usize, k: usize| c * cells + (k * ny + j) * nx + i;
        let n = [nx, ny, nz];
        // the neighbour one step up or down axis a, with its phase: None beyond a wall
        let step = |a: usize, m: usize, forward: bool| -> Option<(usize, c64)> {
            let one = c64::new(1.0, 0.0);
            if forward {
                if m + 1 < n[a] {
                    Some((m + 1, one))
                } else {
                    self.phases[a].map(|p| (0, p))
                }
            } else if m > 0 {
                Some((m - 1, one))
            } else {
                self.phases[a].map(|p| (n[a] - 1, 1.0 / p))
            }
        };
        let e: Vec<c64> = (0..3 * cells)
            .map(|r| {
                let (c, rest) = (r / cells, r % cells);
                let at = [rest % nx, (rest / nx) % ny, rest / (nx * ny)];
                if self.fixed(c, at) { zero } else { v[r] }
            })
            .collect();
        // a field's value at a neighbour along axis a of (i, j, k), zero beyond a wall
        let moved = |f: &[c64], c: usize, at: [usize; 3], a: usize, forward: bool| -> c64 {
            match step(a, at[a], forward) {
                Some((m, phase)) => {
                    let mut to = at;
                    to[a] = m;
                    f[index(c, to[0], to[1], to[2])] * phase
                }
                None => zero,
            }
        };
        let mut h = vec![zero; 3 * cells];
        let mut phi = vec![zero; cells];
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let at = [i, j, k];
                    let here = |c: usize| e[index(c, i, j, k)];
                    let (wx, wy, wz) = (
                        self.at_halves[0][i],
                        self.at_halves[1][j],
                        self.at_halves[2][k],
                    );
                    h[index(0, i, j, k)] = (moved(&e, 2, at, 1, true) - here(2)) * wy
                        - (moved(&e, 1, at, 2, true) - here(1)) * wz;
                    h[index(1, i, j, k)] = (moved(&e, 0, at, 2, true) - here(0)) * wz
                        - (moved(&e, 2, at, 0, true) - here(2)) * wx;
                    h[index(2, i, j, k)] = (moved(&e, 1, at, 0, true) - here(1)) * wx
                        - (moved(&e, 0, at, 1, true) - here(0)) * wy;
                    // ε E's divergence at the node, each difference backward
                    let u = |c: usize, to: Option<(usize, c64)>, a: usize| -> c64 {
                        match to {
                            Some((m, phase)) => {
                                let mut p = at;
                                p[a] = m;
                                let r = index(c, p[0], p[1], p[2]);
                                e[r] * self.epsilon[r] * phase
                            }
                            None => zero,
                        }
                    };
                    let mine = |c: usize| {
                        let r = index(c, i, j, k);
                        e[r] * self.epsilon[r]
                    };
                    let dx = (mine(0) - u(0, step(0, i, false), 0)) * self.at_nodes[0][i];
                    let dy = (mine(1) - u(1, step(1, j, false), 1)) * self.at_nodes[1][j];
                    let dz = (mine(2) - u(2, step(2, k, false), 2)) * self.at_nodes[2][k];
                    phi[index(0, i, j, k)] = self.node_inverse[index(0, i, j, k)] * (dx + dy + dz);
                }
            }
        }
        let mut out = vec![zero; 3 * cells];
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let at = [i, j, k];
                    let here = |c: usize| h[index(c, i, j, k)];
                    let (wx, wy, wz) = (
                        self.at_nodes[0][i],
                        self.at_nodes[1][j],
                        self.at_nodes[2][k],
                    );
                    let curl = [
                        (here(2) - moved(&h, 2, at, 1, false)) * wy
                            - (here(1) - moved(&h, 1, at, 2, false)) * wz,
                        (here(0) - moved(&h, 0, at, 2, false)) * wz
                            - (here(2) - moved(&h, 2, at, 0, false)) * wx,
                        (here(1) - moved(&h, 1, at, 0, false)) * wx
                            - (here(0) - moved(&h, 0, at, 1, false)) * wy,
                    ];
                    let node = phi[index(0, i, j, k)];
                    let gradient = [
                        (moved(&phi, 0, at, 0, true) - node) * self.at_halves[0][i],
                        (moved(&phi, 0, at, 1, true) - node) * self.at_halves[1][j],
                        (moved(&phi, 0, at, 2, true) - node) * self.at_halves[2][k],
                    ];
                    for c in 0..3 {
                        let r = index(c, i, j, k);
                        out[r] = if self.fixed(c, at) {
                            v[r]
                        } else {
                            self.k0_squared * self.epsilon[r] * e[r] - curl[c] + gradient[c]
                        };
                    }
                }
            }
        }
        out
    }
}

/// The finest level of a [`MultigridCycle`] without its matrix: its operator as a [`Stencil`]
/// (shifted: [`Stencil::k0_squared`] is k₀²(1 + iβ)), smoothed by damped Jacobi, and its
/// transfers to the next level.
#[derive(Clone, Copy, Debug)]
pub struct StencilLevel<'a> {
    pub(crate) operator: Stencil<'a>,
    pub(crate) jacobi: &'a [c64],
    pub(crate) pre: usize,
    pub(crate) post: usize,
    pub(crate) restriction: RowMatrix<'a>,
    pub(crate) prolongation: RowMatrix<'a>,
    pub(crate) scaling: Option<(&'a [c64], &'a [c64])>,
}

impl<'a> StencilLevel<'a> {
    /// The level's operator.
    pub fn operator(&self) -> Stencil<'a> {
        self.operator
    }

    /// ω D⁻¹ at each value, D the operator's diagonal from its stencil: a sweep is
    /// x ← x + ω D⁻¹ (b − A x), each value's product `r * jacobi[k]`.
    pub fn jacobi(&self) -> &'a [c64] {
        self.jacobi
    }

    /// Jacobi's sweeps before the level's coarse corrections ([`MultigridCycle::pre`] is the
    /// coarser levels' ILU(0) steps).
    pub fn pre(&self) -> usize {
        self.pre
    }

    /// And after them.
    pub fn post(&self) -> usize {
        self.post
    }

    /// Pᵀ, as [`MultigridLevel::restriction`].
    pub fn restriction(&self) -> RowMatrix<'a> {
        self.restriction
    }

    /// P, as [`MultigridLevel::prolongation`].
    pub fn prolongation(&self) -> RowMatrix<'a> {
        self.prolongation
    }

    /// As [`MultigridLevel::scaling`].
    pub fn scaling(&self) -> Option<(&'a [c64], &'a [c64])> {
        self.scaling
    }
}

/// A sparse complex matrix by rows, square or not, as a backend is given the multigrid's
/// operators and transfers: row i's entries are `indices[starts[i]..starts[i + 1]]`, distinct and
/// below the columns, with their `values`. An operator's columns are ascending in each row; a
/// transfer's come in the order photonoxide sums them in.
#[derive(Clone, Copy, Debug)]
pub struct RowMatrix<'a> {
    columns: usize,
    starts: &'a [usize],
    indices: &'a [usize],
    values: &'a [c64],
}

impl<'a> RowMatrix<'a> {
    /// The matrix of `starts.len() − 1` rows and `columns` columns with these entries.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless the starts run from 0 to the number of entries
    /// without decreasing, there is a value per index, and each index is below `columns`.
    pub fn new(
        columns: usize,
        starts: &'a [usize],
        indices: &'a [usize],
        values: &'a [c64],
    ) -> Result<RowMatrix<'a>> {
        if starts.first() != Some(&0)
            || starts.last() != Some(&indices.len())
            || indices.len() != values.len()
            || starts.windows(2).any(|p| p[0] > p[1])
        {
            return Err(invalid(format!(
                "a matrix by rows needs starts from 0 up to its {} entries, and a value for each",
                indices.len()
            )));
        }
        if indices.iter().any(|&c| c >= columns) {
            return Err(invalid(format!("a column index is not below {columns}")));
        }
        Ok(RowMatrix {
            columns,
            starts,
            indices,
            values,
        })
    }

    /// The matrix of these entries, laid out as [`RowMatrix::new`] asks, unchecked: for
    /// photonoxide's own, which are.
    pub(crate) fn trusted(
        columns: usize,
        starts: &'a [usize],
        indices: &'a [usize],
        values: &'a [c64],
    ) -> RowMatrix<'a> {
        debug_assert!(RowMatrix::new(columns, starts, indices, values).is_ok());
        RowMatrix {
            columns,
            starts,
            indices,
            values,
        }
    }

    /// The rows.
    pub fn rows(&self) -> usize {
        self.starts.len() - 1
    }

    /// The columns.
    pub fn columns(&self) -> usize {
        self.columns
    }

    /// The row starts, one more than the rows.
    pub fn starts(&self) -> &'a [usize] {
        self.starts
    }

    /// Each entry's column.
    pub fn indices(&self) -> &'a [usize] {
        self.indices
    }

    /// Each entry's value.
    pub fn values(&self) -> &'a [c64] {
        self.values
    }

    /// M v, each row summed in its entries' order, on one thread.
    pub fn apply(&self, v: &[c64]) -> Vec<c64> {
        (0..self.rows())
            .map(|r| {
                (self.starts[r]..self.starts[r + 1])
                    .map(|k| self.values[k] * v[self.indices[k]])
                    .sum()
            })
            .collect()
    }
}

/// One level of a [`MultigridCycle`] above the coarsest.
#[derive(Clone, Copy, Debug)]
pub struct MultigridLevel<'a> {
    pub(crate) operator: RowMatrix<'a>,
    pub(crate) smoother: &'a IluFactors,
    pub(crate) restriction: RowMatrix<'a>,
    pub(crate) prolongation: RowMatrix<'a>,
    pub(crate) scaling: Option<(&'a [c64], &'a [c64])>,
}

impl<'a> MultigridLevel<'a> {
    /// The level's operator Aₗ (square): the fine one is the operator the cycle is built on
    /// (Shin and Fan's, shifted), each coarser one Pᵀ Aₗ P.
    pub fn operator(&self) -> RowMatrix<'a> {
        self.operator
    }

    /// The smoother M = LU: ILU(0) of Aₗ's diagonal blocks (slabs of planes, all three
    /// components in each), as one pair of factors in the level's numbering; the entries
    /// coupling two blocks are left out, so the factors are block diagonal too.
    pub fn smoother(&self) -> &'a IluFactors {
        self.smoother
    }

    /// The restriction Pᵀ: the next level's rows, this level's columns.
    pub fn restriction(&self) -> RowMatrix<'a> {
        self.restriction
    }

    /// The prolongation P: this level's rows, the next level's columns.
    pub fn prolongation(&self) -> RowMatrix<'a> {
        self.prolongation
    }

    /// For a next level discretized again (only given to a backend that
    /// [`IterativeSolver::runs_multigrid_stencil`]): (D_f, W⁻¹), the residual restricted as
    /// W⁻¹ ∘ Pᵀ(D_f ∘ r), D_f a value per value here (its cell's volume), W⁻¹ one per value of
    /// the next level. `None`: restricted by Pᵀ alone.
    pub fn scaling(&self) -> Option<(&'a [c64], &'a [c64])> {
        self.scaling
    }
}

/// The residual `r` restricted by `restriction`, weighted by `scaling` if it has one.
fn restrict(
    restriction: RowMatrix<'_>,
    scaling: Option<(&[c64], &[c64])>,
    r: &[c64],
) -> Vec<c64> {
    match scaling {
        Some((fine, coarse)) => {
            let scaled: Vec<c64> = r.iter().zip(fine).map(|(p, q)| p * q).collect();
            let mut out = restriction.apply(&scaled);
            out.iter_mut().zip(coarse).for_each(|(p, q)| *p *= q);
            out
        }
        None => restriction.apply(r),
    }
}

/// photonoxide's multigrid cycle as a backend takes it, built by photonoxide on the CPU: each
/// level's operator, smoother and transfers, and the coarsest level's factorization, which
/// photonoxide solves with ([`MultigridCycle::coarsest_solve`]).
///
/// One cycle from level l for bₗ, from xₗ = 0 ([`MultigridCycle::cycle`] runs it on the CPU):
///
/// 1. [`MultigridCycle::pre`] smoothing steps, each xₗ ← xₗ + M⁻¹(bₗ − Aₗ xₗ) (the first,
///    from xₗ = 0, M⁻¹ bₗ);
/// 2. the coarse corrections the [`CycleShape`] asks for (V: one, a V-cycle
///    below; W: two W-cycles; F: an F-cycle then a V-cycle), each
///    xₗ ← xₗ + P cycle(l + 1, Pᵀ(bₗ − Aₗ xₗ)); on the coarsest level, the cycle is the direct
///    solve;
/// 3. [`MultigridCycle::post`] smoothing steps, as in 1.
///
/// Without the finest matrix ([`MultigridCycle::stencil`], given only to a backend that
/// [`IterativeSolver::runs_multigrid_stencil`]), level 0 is the stencil's: its smoothing steps
/// are [`StencilLevel::pre`] and [`StencilLevel::post`] sweeps of damped Jacobi,
/// xₗ ← xₗ + ω D⁻¹(bₗ − Aₗ xₗ) (the first from xₗ = 0 ω D⁻¹ bₗ), and [`MultigridCycle::levels`]
/// are the levels below it. A level's residual is restricted as its scaling says
/// ([`MultigridLevel::scaling`]).
pub struct MultigridCycle<'a> {
    pub(crate) shape: CycleShape,
    pub(crate) pre: usize,
    pub(crate) post: usize,
    pub(crate) restart: usize,
    pub(crate) stencil: Option<StencilLevel<'a>>,
    pub(crate) levels: Vec<MultigridLevel<'a>>,
    pub(crate) coarsest: RowMatrix<'a>,
    pub(crate) lu: &'a dyn super::Factorization,
    /// The thread the coarsest solves run on: their sums then come in one order whatever the
    /// machine.
    pub(crate) one: &'a rayon::ThreadPool,
}

impl<'a> MultigridCycle<'a> {
    /// The cycle's shape.
    pub fn shape(&self) -> CycleShape {
        self.shape
    }

    /// The smoothing steps before each level's coarse corrections.
    pub fn pre(&self) -> usize {
        self.pre
    }

    /// And after them.
    pub fn post(&self) -> usize {
        self.post
    }

    /// The steps of GMRES between restarts.
    pub fn restart(&self) -> usize {
        self.restart
    }

    /// The levels above the coarsest with a matrix, finest first: below the stencil's, if the
    /// cycle has one.
    pub fn levels(&self) -> &[MultigridLevel<'a>] {
        &self.levels
    }

    /// The finest level without its matrix, if it has none: then [`MultigridCycle::levels`]
    /// start at the next.
    pub fn stencil(&self) -> Option<&StencilLevel<'a>> {
        self.stencil.as_ref()
    }

    /// The finest level's unknowns.
    pub fn unknowns(&self) -> usize {
        match (&self.stencil, self.levels.first()) {
            (Some(s), _) => s.operator.unknowns(),
            (None, Some(l)) => l.operator.rows(),
            (None, None) => self.coarsest.rows(),
        }
    }

    /// The coarsest level's operator, which photonoxide has factorized: a backend may solve
    /// with it its own way, or with [`MultigridCycle::coarsest_solve`].
    pub fn coarsest(&self) -> RowMatrix<'a> {
        self.coarsest
    }

    /// x with A_c x = `b` on the coarsest level, by photonoxide's factorization of it, on one
    /// thread: the bits of photonoxide's own cycle.
    ///
    /// # Errors
    ///
    /// The direct solver's.
    pub fn coarsest_solve(&self, b: &[c64]) -> Result<Vec<c64>> {
        self.one.install(|| self.lu.solve(b))
    }

    /// One cycle for `b` on the finest level, on the CPU, one thread, each sum in order: what a
    /// backend reproduces (to rounding: photonoxide's own cycle solves the smoother block by
    /// block and multiplies by the inverse of U's diagonal where this divides by it). A failed
    /// coarsest solve gives NaNs.
    pub fn cycle(&self, b: &[c64]) -> Vec<c64> {
        self.cycle_from(0, b, self.shape)
    }

    /// The cycle's levels above the coarsest, the stencil's first if it has one.
    fn depth(&self) -> usize {
        self.levels.len() + usize::from(self.stencil.is_some())
    }

    fn cycle_from(&self, level: usize, b: &[c64], shape: CycleShape) -> Vec<c64> {
        if level == self.depth() {
            return self
                .coarsest_solve(b)
                .unwrap_or_else(|_| vec![c64::new(f64::NAN, f64::NAN); b.len()]);
        }
        // the level's operator, smoother, steps and transfers
        type Apply<'f> = Box<dyn Fn(&[c64]) -> Vec<c64> + 'f>;
        let (operator, smoother, (pre, post), restriction, prolongation, scaling): (
            Apply<'_>,
            Apply<'_>,
            _,
            _,
            _,
            _,
        ) = match (&self.stencil, level) {
            (Some(s), 0) => (
                Box::new(move |x| s.operator.apply(x)),
                Box::new(move |r: &[c64]| r.iter().zip(s.jacobi).map(|(p, q)| p * q).collect()),
                (s.pre, s.post),
                s.restriction,
                s.prolongation,
                s.scaling,
            ),
            _ => {
                let l = &self.levels[level - usize::from(self.stencil.is_some())];
                (
                    Box::new(move |x| l.operator.apply(x)),
                    Box::new(move |r| l.smoother.solve(r)),
                    (self.pre, self.post),
                    l.restriction,
                    l.prolongation,
                    l.scaling,
                )
            }
        };
        let residual = |x: &Option<Vec<c64>>| -> Vec<c64> {
            match x {
                None => b.to_vec(),
                Some(x) => {
                    let ax = operator(x);
                    b.iter().zip(&ax).map(|(p, q)| p - q).collect()
                }
            }
        };
        let add = |x: &mut Option<Vec<c64>>, d: Vec<c64>| match x {
            None => *x = Some(d),
            Some(x) => x.iter_mut().zip(&d).for_each(|(p, q)| *p += q),
        };
        let mut x: Option<Vec<c64>> = None;
        for _ in 0..pre {
            let d = smoother(&residual(&x));
            add(&mut x, d);
        }
        let corrections = match shape {
            CycleShape::V => vec![CycleShape::V],
            CycleShape::W => vec![CycleShape::W, CycleShape::W],
            CycleShape::F => vec![CycleShape::F, CycleShape::V],
        };
        for inner in corrections {
            let coarse = restrict(restriction, scaling, &residual(&x));
            let correction = self.cycle_from(level + 1, &coarse, inner);
            add(&mut x, prolongation.apply(&correction));
        }
        for _ in 0..post {
            let d = smoother(&residual(&x));
            add(&mut x, d);
        }
        x.unwrap_or_else(|| vec![c64::new(0.0, 0.0); b.len()])
    }
}

/// ILU(0)'s factors as a backend takes them: L, unit lower triangular, without its diagonal,
/// and U, upper triangular, its diagonal included; each by rows, its columns ascending. M = LU
/// preconditions QMR from the right: QMR runs on A M⁻¹, as photonoxide's own does.
#[derive(Clone, Debug, PartialEq)]
pub struct IluFactors {
    pub(crate) n: usize,
    pub(crate) lower: (Vec<usize>, Vec<usize>, Vec<c64>),
    pub(crate) upper: (Vec<usize>, Vec<usize>, Vec<c64>),
}

impl IluFactors {
    /// The unknowns.
    pub fn n(&self) -> usize {
        self.n
    }

    /// L's rows below the diagonal: n + 1 row starts, each entry's column and value.
    pub fn lower(&self) -> (&[usize], &[usize], &[c64]) {
        (&self.lower.0, &self.lower.1, &self.lower.2)
    }

    /// U's rows, the diagonal first in each: n + 1 row starts, each entry's column and value.
    pub fn upper(&self) -> (&[usize], &[usize], &[c64]) {
        (&self.upper.0, &self.upper.1, &self.upper.2)
    }

    /// M⁻¹ v = U⁻¹ L⁻¹ v, row by row on one thread, each row's sum in its entries' order.
    pub fn solve(&self, v: &[c64]) -> Vec<c64> {
        let mut x = v.to_vec();
        let (starts, columns, values) = self.lower();
        for i in 0..self.n {
            let mut s = x[i];
            for k in starts[i]..starts[i + 1] {
                s -= values[k] * x[columns[k]];
            }
            x[i] = s;
        }
        let (starts, columns, values) = self.upper();
        for i in (0..self.n).rev() {
            let mut s = x[i];
            for k in starts[i] + 1..starts[i + 1] {
                s -= values[k] * x[columns[k]];
            }
            x[i] = s / values[starts[i]];
        }
        x
    }
}

#[derive(Clone)]
enum Entry {
    Available(Arc<dyn IterativeSolver>),
    Unavailable { name: String, reason: String },
}

impl Entry {
    fn name(&self) -> String {
        match self {
            Entry::Available(solver) => solver.capabilities().name,
            Entry::Unavailable { name, .. } => name.clone(),
        }
    }
}

fn registry() -> &'static RwLock<Vec<Entry>> {
    static REGISTRY: OnceLock<RwLock<Vec<Entry>>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(Vec::new()))
}

fn poisoned<T>(_: T) -> crate::Error {
    invalid("the registry was left locked by a thread that panicked")
}

fn place(name: &str, entry: Entry) -> Result<()> {
    check_name(name)?;
    if matches!(name, "auto" | OWN) {
        return Err(invalid(format!("{name:?} is photonoxide's own name")));
    }
    let mut entries = registry().write().map_err(poisoned)?;
    match entries.iter_mut().find(|e| e.name() == name) {
        Some(existing) => *existing = entry,
        None => entries.push(entry),
    }
    Ok(())
}

/// Registers an iterative solver under its capabilities' name, in place of one already
/// registered under it.
///
/// # Errors
///
/// [`crate::Error::InvalidValue`] for a name that isn't lower-case letters, digits, `-` and
/// `_`, for `auto` and `photonoxide`, and for a solver that isn't complex.
pub fn register_iterative(solver: Arc<dyn IterativeSolver>) -> Result<()> {
    let capabilities = solver.capabilities();
    if !capabilities.complex {
        return Err(invalid(format!(
            "{} doesn't solve complex systems, which photonoxide's are",
            capabilities.name
        )));
    }
    place(&capabilities.name, Entry::Available(solver))
}

/// Registers an iterative backend that can't be had, with the reason.
///
/// # Errors
///
/// As [`register_iterative`], for the name.
pub fn register_iterative_unavailable(name: &str, reason: impl Into<String>) -> Result<()> {
    place(
        name,
        Entry::Unavailable {
            name: name.to_owned(),
            reason: reason.into(),
        },
    )
}

/// Every iterative backend registered, the unavailable ones among them. photonoxide's own QMR
/// isn't one: it is what `auto` and `photonoxide` choose.
///
/// # Errors
///
/// [`crate::Error::InvalidValue`] if a thread panicked while registering.
pub fn iterative_solvers() -> Result<Vec<Listed>> {
    let entries = registry().read().map_err(poisoned)?;
    Ok(entries
        .iter()
        .map(|e| match e {
            Entry::Available(solver) => {
                let capabilities = solver.capabilities();
                Listed {
                    name: capabilities.name.clone(),
                    capabilities: Some(capabilities),
                    unavailable: None,
                }
            }
            Entry::Unavailable { name, reason } => Listed {
                name: name.clone(),
                capabilities: None,
                unavailable: Some(reason.clone()),
            },
        })
        .collect())
}

fn preference() -> &'static RwLock<Option<Arc<dyn IterativeSolver>>> {
    static PREFERRED: OnceLock<RwLock<Option<Arc<dyn IterativeSolver>>>> = OnceLock::new();
    PREFERRED.get_or_init(|| RwLock::new(None))
}

/// Makes `auto` stand for the iterative backend `choice` names, in this process, where it runs
/// a solve as photonoxide's own would ([`crate::fdfd::IterativeSolver3d::with_iterative_backend`]);
/// `auto` and `photonoxide` make it photonoxide's own again, as it is until this is called. For
/// running photonoxide's own checks on a backend (the validation cases, as
/// `photonoxide-native`'s `validate` example does): nothing in photonoxide calls it.
///
/// # Errors
///
/// As [`iterative`], and nothing changes.
pub fn prefer_iterative(choice: &Choice) -> Result<()> {
    let solver = iterative(choice)?;
    *preference().write().map_err(poisoned)? = solver;
    Ok(())
}

/// The iterative backend `auto` stands for ([`prefer_iterative`]): `None`, photonoxide's own,
/// unless one was preferred.
pub fn preferred_iterative() -> Option<Arc<dyn IterativeSolver>> {
    preference().read().ok().and_then(|p| p.clone())
}

/// The iterative backend of a choice: `None` for `auto` and `photonoxide`, whose QMR is
/// photonoxide's own.
///
/// # Errors
///
/// [`crate::Error::InvalidValue`] for a name nothing is registered under, and for a backend
/// registered as unavailable, with its reason. Never another backend in its place.
pub fn iterative(choice: &Choice) -> Result<Option<Arc<dyn IterativeSolver>>> {
    let name = match choice {
        Choice::Auto | Choice::Photonoxide => return Ok(None),
        Choice::Named(name) => name,
    };
    let entries = registry().read().map_err(poisoned)?;
    match entries.iter().find(|e| e.name() == *name) {
        Some(Entry::Available(solver)) => Ok(Some(solver.clone())),
        Some(Entry::Unavailable { reason, .. }) => Err(invalid(format!(
            "the iterative solver {name:?} isn't available: {reason}"
        ))),
        None => {
            let known: Vec<String> = entries.iter().map(Entry::name).collect();
            Err(invalid(format!(
                "no iterative solver is registered as {name:?}: there are auto, photonoxide{}",
                known.iter().map(|k| format!(", {k}")).collect::<String>()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::Form;

    /// A backend that answers every run with zeros, for the registry's tests.
    struct Zeros;

    impl IterativeSolver for Zeros {
        fn capabilities(&self) -> Capabilities {
            Capabilities::new("zeros-for-tests", "1", "MIT OR Apache-2.0")
        }

        fn qmr_run(&self, matrix: &Matrix<'_>, _: &[c64], _: Stopping) -> Result<QmrRun> {
            Ok(QmrRun::Broken(
                vec![c64::new(0.0, 0.0); matrix.n()],
                Vec::new(),
                0.0,
            ))
        }
    }

    #[test]
    fn auto_and_photonoxide_are_photonoxides_own_qmr() {
        assert!(iterative(&Choice::Auto).unwrap().is_none());
        assert!(iterative(&Choice::Photonoxide).unwrap().is_none());
    }

    #[test]
    fn a_backend_is_found_by_name_and_an_unavailable_one_says_why() {
        register_iterative(Arc::new(Zeros)).unwrap();
        let solver = iterative(&Choice::Named("zeros-for-tests".into()))
            .unwrap()
            .unwrap();
        let (starts, rows, values) = (vec![0, 1], vec![0], vec![c64::new(1.0, 0.0)]);
        let m = Matrix::new(1, &starts, &rows, &values, Form::General).unwrap();
        let run = solver.qmr_run(&m, &[c64::new(1.0, 0.0)], Stopping::default());
        assert!(matches!(run, Ok(QmrRun::Broken(..))));
        assert!(
            iterative_solvers()
                .unwrap()
                .iter()
                .any(|l| l.name == "zeros-for-tests")
        );

        register_iterative_unavailable("missing-for-tests", "not installed").unwrap();
        let e = iterative(&Choice::Named("missing-for-tests".into()))
            .err()
            .unwrap();
        assert!(e.to_string().contains("not installed"), "{e}");
        let e = iterative(&Choice::Named("nothing-by-this-name".into()))
            .err()
            .unwrap();
        assert!(e.to_string().contains("no iterative solver"), "{e}");
    }

    #[test]
    fn photonoxides_own_names_are_refused() {
        assert!(register_iterative_unavailable("photonoxide", "x").is_err());
        assert!(register_iterative_unavailable("auto", "x").is_err());
    }

    #[test]
    fn a_matrix_by_rows_is_checked_and_multiplies() {
        let one = c64::new(1.0, 0.0);
        let i = c64::new(0.0, 1.0);
        // 2 × 3: [[1, 0, i], [0, 2, 0]]
        let (starts, indices, values) = ([0, 2, 3], [0, 2, 1], [one, i, 2.0 * one]);
        let m = RowMatrix::new(3, &starts, &indices, &values).unwrap();
        assert_eq!((m.rows(), m.columns()), (2, 3));
        assert_eq!(m.apply(&[one, one, one]), vec![one + i, 2.0 * one]);
        // a column out of range, starts that don't end at the entries, a value missing
        assert!(RowMatrix::new(2, &starts, &indices, &values).is_err());
        assert!(RowMatrix::new(3, &[0, 2, 2], &indices, &values).is_err());
        assert!(RowMatrix::new(3, &starts, &indices, &values[..2]).is_err());
        assert!(RowMatrix::new(3, &[0, 3, 2], &indices, &values).is_err());
    }

    #[test]
    fn ilu0s_factors_solve_with_lu() {
        // L = [[1, 0], [2, 1]], U = [[2, 1], [0, 4]]: LU = [[2, 1], [4, 6]]
        let c = |x: f64| c64::new(x, 0.0);
        let factors = IluFactors {
            n: 2,
            lower: (vec![0, 0, 1], vec![0], vec![c(2.0)]),
            upper: (vec![0, 2, 3], vec![0, 1, 1], vec![c(2.0), c(1.0), c(4.0)]),
        };
        // LU (1, 1) = (3, 10)
        assert_eq!(factors.solve(&[c(3.0), c(10.0)]), vec![c(1.0), c(1.0)]);
    }
}
