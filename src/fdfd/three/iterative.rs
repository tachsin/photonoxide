//! The 3D problem solved iteratively: QMR on Shin and Fan's operator, without factorizing.

use num_complex::Complex64 as c64;

use super::{Axis, Boundaries3d, Field3d, Grid3d, Lattice, Port3d, PortMode3d, Solver3d};
use crate::fdfd::Direction;
use crate::fdfd::krylov::{Convergence, Ilu0, Sparse, Stopping, qmr, qmr_preconditioned};
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
    matrix: Sparse,
    /// The matrix's ILU(0), when QMR is preconditioned.
    ilu: Option<Ilu0>,
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
        let matrix = Sparse::new(
            grid.unknowns(),
            lattice
                .assemble_with(&eps, formulation.s())
                .into_iter()
                .map(|t| (t.row, t.col, t.val)),
        );
        Ok(IterativeSolver3d {
            lattice,
            eps,
            formulation,
            matrix,
            ilu: None,
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
        self.ilu = Some(Ilu0::new(&self.matrix)?);
        Ok(self)
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
        let (values, convergence) = match &self.ilu {
            Some(ilu) => qmr_preconditioned(&self.matrix, ilu, &b, stopping)?,
            None => qmr(&self.matrix, &b, stopping)?,
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
