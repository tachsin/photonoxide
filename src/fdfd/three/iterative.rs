//! The 3D problem solved iteratively: QMR on Shin and Fan's operator, without factorizing.

use num_complex::Complex64 as c64;

use super::{Boundaries3d, Field3d, Grid3d, Lattice, Solver3d};
use crate::fdfd::krylov::{Convergence, Sparse, Stopping, qmr};
use crate::units::Wavelength;
use crate::{Error, Result};

/// The operator QMR iterates on. Both have the same solution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Formulation {
    /// The curl-curl equation as it is, A = −∇ × ∇ × + k₀² ε: Shin and Fan's Eq. 7 with s = 0.
    CurlCurl,
    /// Shin and Fan's Eq. 7 with s = −1: the continuity equation, ∇ · (ε E) = ∇ · J / (i k₀)
    /// in our units, added as A + ε⁻¹ ∇(∇ · (ε ·)), with the right-hand side to match. In a
    /// uniform medium this turns −∇ × ∇ × into the vector Laplacian, which removes the near-zero
    /// eigenvalues of the curl-curl operator's null space (their Section 2).
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
        })
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
        let (values, convergence) = qmr(&self.matrix, &b, stopping)?;
        Ok((
            Field3d {
                lattice: self.lattice.clone(),
                values,
            },
            convergence,
        ))
    }
}
