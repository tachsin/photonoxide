//! The 3D problem solved iteratively: QMR on Shin and Fan's operator, without factorizing.

use std::sync::Arc;

use num_complex::Complex64 as c64;

use super::multigrid::{Hierarchy, Multigrid, shifted};
use super::{Axis, Boundaries3d, Field3d, Grid3d, Lattice, Port3d, PortMode3d, Solver3d};
use crate::backend::{self, Choice, Form, IterativeSolver};
use crate::fdfd::Direction;
use crate::fdfd::krylov::{
    Convergence, Ilu0, Sparse, Stopping, gmres_preconditioned, qmr, qmr_preconditioned,
    qmr_preconditioned_by, qmr_similar, qmr_similar_by, restarted_by,
};
use crate::units::Wavelength;
use crate::{Error, Result};

/// The operator QMR iterates on. Both have the same solution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Formulation {
    /// The curl-curl equation as it is, A = −∇ × ∇ × + k₀² ε: Shin and Fan's Eq. 7 with s = 0.
    CurlCurl,
    /// Shin and Fan's Eq. 7 with s = −1: the continuity equation, ∇ · (ε E) = ∇ · J / (i k₀)
    /// in our units, added as A + ∇(ε⁻¹ ∇ · (ε ·)) with ε⁻¹ at the nodes, and the right-hand
    /// side to match. In a uniform medium this turns −∇ × ∇ × into the vector Laplacian, which
    /// removes the near-zero eigenvalues of the curl-curl operator's null space (their Section 2).
    /// It converges in fewer iterations to its own residual, but not to a more accurate field:
    /// see docs/methods/fdfd-3d.md.
    ShinFan,
}

impl Formulation {
    /// Shin and Fan's s.
    fn s(self) -> f64 {
        match self {
            Formulation::CurlCurl => 0.0,
            Formulation::ShinFan => -1.0,
        }
    }
}

/// A 3D FDFD problem for an iterative solver: the matrix assembled, nothing factorized. Memory
/// grows as the unknowns, not as the factorization's fill.
///
/// W. Shin, S. Fan, "Accelerated solution of the frequency-domain Maxwell's equations by
/// engineering the eigenvalue distribution of the operator", Opt. Express 21, 22578 (2013),
/// doi:10.1364/OE.21.022578: their Eq. 7, solved by QMR (R. W. Freund, N. M. Nachtigal, Numer.
/// Math. 60, 315 (1991), doi:10.1007/BF01385726) as they do for 3D problems (their Section 4).
pub struct IterativeSolver3d {
    lattice: Lattice,
    eps: Vec<c64>,
    formulation: Formulation,
    /// The matrix QMR multiplies by: A, or, when `similarity` is S, B = S A S⁻¹.
    matrix: Sparse,
    /// The curl-curl operator with PMLs or periodic sides is similar to a complex symmetric
    /// matrix by a diagonal S ([`Sparse::symmetrized`]): then `matrix` is that matrix, and QMR
    /// for complex symmetric matrices (Freund 1992) solves with one product an iteration.
    similarity: Option<Vec<c64>>,
    /// What preconditions QMR, if anything.
    preconditioner: Preconditioning,
    /// The backend that runs QMR or GMRES, if not photonoxide's own
    /// ([`IterativeSolver3d::with_iterative_backend`]).
    backend: Option<Arc<dyn IterativeSolver>>,
}

/// A preconditioner for QMR.
enum Preconditioning {
    None,
    Ilu(Box<Ilu0>),
    Multigrid(Box<Hierarchy>),
}

impl IterativeSolver3d {
    /// Assembles the problem at `wavelength` on `grid`, with relative permittivity
    /// `eps(x, y, z)` (µm), inside `boundaries`, for QMR on `formulation`.
    ///
    /// # Errors
    ///
    /// As [`Solver3d::new`], but for the factorization, which there isn't.
    pub fn new(
        grid: Grid3d,
        wavelength: Wavelength,
        eps: impl Fn(f64, f64, f64) -> c64,
        boundaries: Boundaries3d,
        formulation: Formulation,
    ) -> Result<IterativeSolver3d> {
        let (lattice, eps) = Solver3d::setup(grid, wavelength, eps, boundaries)?;
        let mut matrix = Sparse::new(
            grid.unknowns(),
            lattice
                .assemble_with(&eps, formulation.s())
                .into_iter()
                .map(|t| (t.row, t.col, t.val)),
        );
        // the curl-curl operator, unpreconditioned (neither preconditioner takes it), by QMR
        // for symmetric matrices where it can be: twice as fast on the 40³ guide
        // (docs/methods/fdfd-3d.md); Shin and Fan's operator, and a Bloch side's, can't
        let mut similarity = None;
        if formulation == Formulation::CurlCurl
            && let Some((s, b)) = matrix.symmetrized()
        {
            matrix = b;
            similarity = Some(s);
        }
        Ok(IterativeSolver3d {
            lattice,
            eps,
            formulation,
            matrix,
            similarity,
            preconditioner: Preconditioning::None,
            backend: None,
        })
    }

    /// The same problem, QMR preconditioned by the incomplete LU factorization of its matrix with
    /// no fill, ILU(0) (as in Y. Saad, Iterative Methods for Sparse Linear Systems, 2nd ed., SIAM
    /// (2003), doi:10.1137/1.9780898718003), from the right: each iteration then
    /// takes two triangular solves with the factors and two with their transposes. For Shin and
    /// Fan's operator with PMLs whose stretch stays within 45° of the real axis
    /// ([`Boundaries3d::stretched_pml`]): there it cuts QMR's iterations by 10 to 20 times
    /// (docs/methods/fdfd-3d.md).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for the curl-curl operator, on which ILU(0) fails (QMR doesn't
    /// converge: its near-null space of gradients makes the incomplete factors useless), or for a
    /// zero pivot.
    pub fn with_ilu(mut self) -> Result<IterativeSolver3d> {
        if self.formulation == Formulation::CurlCurl {
            return Err(Error::invalid(
                "fdfd preconditioner",
                "ILU(0) needs Shin and Fan's operator (Formulation::ShinFan): on the curl-curl \n                 one QMR doesn't converge",
            ));
        }
        self.preconditioner = Preconditioning::Ilu(Box::new(Ilu0::new(&self.matrix)?));
        Ok(self)
    }

    /// As [`IterativeSolver3d::with_ilu`], each of the factors' triangular solves taken as
    /// `sweeps` Jacobi sweeps, in parallel, where the exact ones are sequential.
    ///
    /// # Errors
    ///
    /// As [`IterativeSolver3d::with_ilu`].
    #[cfg(test)]
    pub(crate) fn with_ilu_sweeps(self, sweeps: usize) -> Result<IterativeSolver3d> {
        let mut solver = self.with_ilu()?;
        if let Preconditioning::Ilu(ilu) = solver.preconditioner {
            solver.preconditioner = Preconditioning::Ilu(Box::new(ilu.with_sweeps(sweeps)));
        }
        Ok(solver)
    }

    /// The same problem, solved by GMRES preconditioned from the right by a multigrid cycle on
    /// Shin and Fan's operator ([`Multigrid`]; GMRES, restarted, as Y. Saad, *Iterative Methods
    /// for Sparse Linear Systems*, 2nd ed., SIAM (2003), doi:10.1137/1.9780898718003, Algorithms
    /// 9.5 and 6.11: one cycle an iteration, where QMR would take the cycle and its transpose): Galerkin coarse operators, ILU(0) smoothing on every level, and a
    /// direct solve on the coarsest (B. Reps, W. Vanroose, H. bin Zubair, J. Comput. Phys. 229,
    /// 8384 (2010), doi:10.1016/j.jcp.2010.07.022, Section 6.1), with the complex shift of
    /// Y. A. Erlangga, C. W. Oosterlee, C. Vuik, SIAM J. Sci. Comput. 27, 1471 (2006),
    /// doi:10.1137/040615195, Eq. 8, if asked. For PMLs whose stretch stays within 45° of the
    /// real axis ([`Boundaries3d::stretched_pml`]); see docs/methods/fdfd-3d.md.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for the curl-curl operator, whose near-null space of gradients no
    /// point smoother reduces, for options without smoothing, or if a level's ILU(0) or the
    /// coarsest factorization fails.
    pub fn with_multigrid(self, options: Multigrid) -> Result<IterativeSolver3d> {
        self.with_multigrid_on(options, &crate::backend::Choice::Auto)
    }

    /// [`IterativeSolver3d::with_multigrid`], its coarsest level factorized by the direct solver
    /// of `direct` ([`crate::backend`]).
    ///
    /// # Errors
    ///
    /// As [`IterativeSolver3d::with_multigrid`], and [`Error::InvalidValue`] for a backend that
    /// isn't registered or isn't available.
    pub fn with_multigrid_on(
        mut self,
        options: Multigrid,
        direct: &crate::backend::Choice,
    ) -> Result<IterativeSolver3d> {
        let solver = crate::backend::direct(direct)?;
        if self.formulation == Formulation::CurlCurl {
            return Err(Error::invalid(
                "fdfd preconditioner",
                "multigrid needs Shin and Fan's operator (Formulation::ShinFan)",
            ));
        }
        let operator = shifted(&self.lattice, &self.eps, &self.matrix, options.shift);
        let hierarchy = Hierarchy::new_on(&self.lattice, &self.eps, operator, options, &solver)?;
        self.preconditioner = Preconditioning::Multigrid(Box::new(hierarchy));
        Ok(self)
    }

    /// The Krylov solver run by the iterative backend `choice` names ([`backend::iterative`]): a
    /// GPU's, say. photonoxide's own for `auto` and `photonoxide`. A backend runs plain QMR, QMR
    /// with ILU(0) on factors photonoxide computes ([`backend::IluFactors`]), and, if it says so
    /// ([`IterativeSolver::runs_multigrid`]), GMRES with the multigrid cycle photonoxide builds
    /// ([`backend::MultigridCycle`]). photonoxide keeps the restarts and the similar matrix's
    /// tolerance around QMR's runs, and checks GMRES's solution on its own residual, so the
    /// solution is held to the same residual as photonoxide's own.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a backend that isn't registered or available, and for a
    /// solver preconditioned by multigrid with a backend that doesn't run it.
    pub fn with_iterative_backend(mut self, choice: &Choice) -> Result<IterativeSolver3d> {
        self.backend = backend::iterative(choice)?;
        self.check_backend()?;
        Ok(self)
    }

    fn check_backend(&self) -> Result<()> {
        if let Some(solver) = &self.backend
            && matches!(self.preconditioner, Preconditioning::Multigrid(_))
            && !solver.runs_multigrid()
        {
            return Err(Error::invalid(
                "iterative backend",
                format!(
                    "{} runs QMR plain or with ILU(0), not GMRES with multigrid",
                    solver.capabilities().name
                ),
            ));
        }
        Ok(())
    }

    /// The number of the multigrid's levels, the coarsest included; 0 without one.
    pub fn multigrid_levels(&self) -> usize {
        self.multigrid_grids().len()
    }

    /// Each of the multigrid's levels, finest first, the coarsest included: its cells along x,
    /// y and z. A PML's cells are merged later than the others ([`Multigrid`]), so a grid with
    /// PMLs coarsens by less than two a level along their axes. Empty without a multigrid.
    pub fn multigrid_grids(&self) -> Vec<[usize; 3]> {
        match &self.preconditioner {
            Preconditioning::Multigrid(h) => h.shapes().to_vec(),
            _ => Vec::new(),
        }
    }

    /// A field of this problem from E's `values`, laid out as [`Grid3d::index`] says: another
    /// solver's field (FDTD's at the leapfrog's frequency) read with this one's fluxes and
    /// mode projections, with no factorization to build.
    pub(crate) fn field(&self, values: Vec<c64>) -> Field3d {
        Field3d {
            lattice: self.lattice.clone(),
            values,
        }
    }

    /// The grid.
    pub fn grid(&self) -> Grid3d {
        self.lattice.grid
    }

    /// The nonzeros of the matrix QMR multiplies by.
    pub fn nonzeros(&self) -> usize {
        self.matrix.nonzeros()
    }

    /// The field for an electric current J = η₀ J on the edges, as [`Solver3d::solve`] takes
    /// it, by QMR to the relative residual `stopping` asks; and how the iteration went.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the source isn't one value per value of E, if the Lanczos
    /// process inside QMR breaks down, or if QMR doesn't reach the tolerance in the iterations
    /// allowed.
    pub fn solve(&self, current: &[c64], stopping: Stopping) -> Result<(Field3d, Convergence)> {
        let i_k0 = c64::new(0.0, -self.lattice.k0);
        let rhs: Vec<c64> = current.iter().map(|j| i_k0 * j).collect();
        self.solve_system(&rhs, stopping)
    }

    /// The field E of A E = `rhs`, A = −∇ × ∇ × + k₀² ε as [`Solver3d::solve_system`] has it,
    /// by QMR on this problem's formulation (the right-hand side transformed with it).
    ///
    /// # Errors
    ///
    /// As [`IterativeSolver3d::solve`].
    pub fn solve_system(&self, rhs: &[c64], stopping: Stopping) -> Result<(Field3d, Convergence)> {
        let n = self.lattice.grid.unknowns();
        if rhs.len() != n {
            return Err(Error::invalid(
                "fdfd source",
                format!("needs {n} values, one per value of E, got {}", rhs.len()),
            ));
        }
        let b: Vec<c64> = (0..n)
            .map(|r| {
                if self.lattice.fixed(r) {
                    c64::new(0.0, 0.0)
                } else {
                    rhs[r]
                }
            })
            .collect();
        let b = self
            .lattice
            .transformed_rhs(&self.eps, &b, self.formulation.s());
        self.check_backend()?;
        let (values, convergence) = match &self.preconditioner {
            Preconditioning::Ilu(ilu) => match &self.backend {
                Some(solver) => {
                    let matrix = self.matrix.as_matrix(Form::General)?;
                    let factors = ilu.factors().ok_or_else(|| {
                        Error::invalid("iterative backend", "ILU(0) by Jacobi sweeps")
                    })?;
                    qmr_preconditioned_by(&self.matrix, ilu.as_ref(), &b, stopping, &|rhs, st| {
                        solver.qmr_run_ilu(&matrix, &factors, rhs, st)
                    })?
                }
                None => qmr_preconditioned(&self.matrix, ilu.as_ref(), &b, stopping)?,
            },
            // a cycle is worth many products with A, and GMRES takes one per iteration where
            // QMR takes two (the cycle and its transpose)
            Preconditioning::Multigrid(h) => match &self.backend {
                Some(solver) => {
                    let cycle = h.cycle_for_backend()?;
                    let (x, how) =
                        solver.gmres_multigrid(self.matrix.rows(), &cycle, &b, stopping)?;
                    checked(&self.matrix, &b, x, how, stopping)?
                }
                None => gmres_preconditioned(&self.matrix, h.as_ref(), &b, stopping, h.restart())?,
            },
            Preconditioning::None => match (&self.backend, &self.similarity) {
                (Some(solver), Some(s)) => {
                    let matrix = self.matrix.as_matrix(Form::Symmetric)?;
                    qmr_similar_by(&self.matrix, s, &b, stopping, &|rhs, st| {
                        solver.qmr_run(&matrix, rhs, st)
                    })?
                }
                (Some(solver), None) => {
                    let matrix = self.matrix.as_matrix(Form::General)?;
                    restarted_by(&self.matrix, &b, stopping, &|rhs, st| {
                        solver.qmr_run(&matrix, rhs, st)
                    })?
                }
                (None, Some(s)) => qmr_similar(&self.matrix, s, &b, stopping)?,
                (None, None) => qmr(&self.matrix, &b, stopping)?,
            },
        };
        Ok((
            Field3d {
                lattice: self.lattice.clone(),
                values,
            },
            convergence,
        ))
    }
}

/// A backend's solution of A x = b, held to `stopping`'s tolerance on the residual computed
/// here, which its convergence then reports.
fn checked(
    a: &Sparse,
    b: &[c64],
    x: Vec<c64>,
    mut how: Convergence,
    stopping: Stopping,
) -> Result<(Vec<c64>, Convergence)> {
    let norm = |v: &[c64]| v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
    if x.len() != b.len() {
        return Err(Error::invalid(
            "iterative backend",
            format!("{} values for {} unknowns", x.len(), b.len()),
        ));
    }
    let ax = a.apply(&x);
    let r: Vec<c64> = b.iter().zip(&ax).map(|(p, q)| p - q).collect();
    let rho0 = norm(b);
    how.residual = if rho0 == 0.0 {
        norm(&r)
    } else {
        norm(&r) / rho0
    };
    // NaN fails too
    if how.residual.is_nan() || how.residual > stopping.tolerance {
        return Err(Error::invalid(
            "iterative backend",
            format!(
                "its solution's residual is {:e}, above the tolerance {:e}",
                how.residual, stopping.tolerance
            ),
        ));
    }
    Ok((x, how))
}

impl IterativeSolver3d {
    /// As [`Solver3d::port_modes`].
    ///
    /// # Errors
    ///
    /// As [`Solver3d::port_modes`].
    pub fn port_modes(&self, axis: Axis, plane: usize, count: usize) -> Result<Vec<PortMode3d>> {
        let (b, c) = axis.others();
        let g = self.lattice.grid;
        self.port_modes_within(axis, plane, (0..g.n(b), 0..g.n(c)), count)
    }

    /// As [`Solver3d::port_modes_within`].
    ///
    /// # Errors
    ///
    /// As [`Solver3d::port_modes_within`].
    pub fn port_modes_within(
        &self,
        axis: Axis,
        plane: usize,
        window: (std::ops::Range<usize>, std::ops::Range<usize>),
        count: usize,
    ) -> Result<Vec<PortMode3d>> {
        self.lattice
            .port_modes(&self.eps, (axis, plane), [window.0, window.1], count)
    }

    /// As [`Solver3d::mode_source`]: the right-hand side for
    /// [`IterativeSolver3d::solve_system`].
    pub fn mode_source(&self, mode: &PortMode3d, direction: Direction) -> Vec<c64> {
        self.lattice.mode_source(mode, direction)
    }

    /// As [`Solver3d::s_matrix`], each run solved by QMR as `stopping` asks: S is as accurate
    /// as the runs' fields.
    ///
    /// # Errors
    ///
    /// As [`Solver3d::s_matrix`], and as [`IterativeSolver3d::solve`] for each run.
    pub fn s_matrix(&self, ports: &[Port3d], stopping: Stopping) -> Result<Vec<Vec<c64>>> {
        Ok(self.s_matrix_with_convergence(ports, stopping)?.0)
    }

    /// As [`IterativeSolver3d::s_matrix`], with how each run's QMR went, in the ports' order.
    pub(crate) fn s_matrix_with_convergence(
        &self,
        ports: &[Port3d],
        stopping: Stopping,
    ) -> Result<(Vec<Vec<c64>>, Vec<Convergence>)> {
        let mut runs = Vec::with_capacity(ports.len());
        let s = self.lattice.s_matrix(ports, |rhs| {
            let (field, convergence) = self.solve_system(rhs, stopping)?;
            runs.push(convergence);
            Ok(field)
        })?;
        Ok((s, runs))
    }
}
