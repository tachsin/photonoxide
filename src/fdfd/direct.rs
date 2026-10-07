//! The 2D and 3D solvers' direct solves, by a backend of [`crate::backend`] (photonoxide's own
//! multifrontal factorization unless another is chosen): as L D Lᵀ of the system's complex
//! symmetric similarity when it has one and the backend takes it, as LU otherwise.
//!
//! The curl-curl operator with PMLs, and the 2D operators, are similar to a complex symmetric
//! matrix: B = S A S⁻¹ with S² the diagonal D for which D A is symmetric (the cells' stretch
//! factors; [`super::krylov::Sparse::symmetrized`]). Then A x = b is B (S x) = S b, and B's
//! L D Lᵀ stores half of A's LU and takes about half its work (docs/baselines.md). A Bloch
//! side's phases break the symmetry, and a system with them is factorized by LU; so is one
//! whose L D Lᵀ needed a pivot perturbed, as the LU's pivoting inside its blocks is the safer.

use std::sync::Arc;

use faer::sparse::Triplet;
use num_complex::Complex64 as c64;

use super::krylov::Sparse;
use crate::backend::{Analysis, Choice, Columns, DirectSolver, Factorization, Form};
use crate::{Error, Result};

/// The backend a solver factorizes with, and the analysis of its matrix's structure once there
/// is one: what a solver hands to the next of a sweep.
#[derive(Clone)]
pub(crate) struct Plan {
    solver: Arc<dyn DirectSolver>,
    analysis: Option<Arc<dyn Analysis>>,
}

impl Plan {
    /// The plan of a choice of backend, nothing analysed yet.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the backend isn't registered or isn't available
    /// ([`crate::backend::direct`]).
    pub(crate) fn new(choice: &Choice) -> Result<Plan> {
        Ok(Plan {
            solver: crate::backend::direct(choice)?,
            analysis: None,
        })
    }
}

/// A system's factors, with the similarity they were taken through.
pub(crate) struct Direct {
    solver: Arc<dyn DirectSolver>,
    analysis: Arc<dyn Analysis>,
    factors: Box<dyn Factorization>,
    /// S, if the factors are B's: B = S A S⁻¹.
    similarity: Option<Vec<c64>>,
}

impl Direct {
    /// The factors of the n × n matrix of `entries`, by `plan`'s backend, ordered on the
    /// unknowns' `positions`; with `plan`'s analysis if it has one (that of a matrix of the
    /// same structure, which also says whether to factorize the similarity).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the matrix can't be assembled or factorized.
    pub(crate) fn new(
        entries: &[Triplet<usize, usize, c64>],
        positions: &[[f64; 3]],
        plan: Plan,
    ) -> Result<Direct> {
        let n = positions.len();
        let Plan { solver, analysis } = plan;
        let symmetric_wanted = solver.capabilities().symmetric
            && analysis
                .as_ref()
                .is_none_or(|a| a.form() == Form::Symmetric);
        if symmetric_wanted {
            let a = Sparse::new(n, entries.iter().map(|t| (t.row, t.col, t.val)));
            if let Some((s, b)) = a.symmetrized() {
                let triplets: Vec<Triplet<usize, usize, c64>> = (0..n)
                    .flat_map(|r| b.row(r).map(move |(c, v)| Triplet::new(r, c, v)))
                    .collect();
                let b = Columns::new(n, &triplets)?;
                let b = b.matrix(Form::Symmetric, Some(positions))?;
                let analysis = match &analysis {
                    Some(a) => Some(a.clone()),
                    None => solver.analyse(&b)?,
                };
                if let Some(analysis) = analysis {
                    let factors = analysis.factorize(&b)?;
                    if factors.report().perturbed_pivots == 0 {
                        return Ok(Direct {
                            solver,
                            analysis,
                            factors,
                            similarity: Some(s),
                        });
                    }
                }
            }
        }
        Self::general(
            entries,
            positions,
            solver,
            analysis.filter(|a| a.form() == Form::General),
        )
    }

    /// The LU of the n × n matrix of `entries` by the backend of these factors, whether or not
    /// the matrix has a symmetric similarity.
    ///
    /// # Errors
    ///
    /// As [`Direct::new`].
    pub(crate) fn lu(
        &self,
        entries: &[Triplet<usize, usize, c64>],
        positions: &[[f64; 3]],
    ) -> Result<Direct> {
        Self::general(entries, positions, self.solver.clone(), None)
    }

    fn general(
        entries: &[Triplet<usize, usize, c64>],
        positions: &[[f64; 3]],
        solver: Arc<dyn DirectSolver>,
        analysis: Option<Arc<dyn Analysis>>,
    ) -> Result<Direct> {
        let a = Columns::new(positions.len(), entries)?;
        let a = a.matrix(Form::General, Some(positions))?;
        let analysis = match analysis {
            Some(a) => a,
            None => solver.analyse(&a)?.ok_or_else(|| {
                Error::invalid(
                    "fdfd",
                    format!(
                        "the direct solver {} can't factorize the system's matrix",
                        solver.capabilities().name
                    ),
                )
            })?,
        };
        Ok(Direct {
            factors: analysis.factorize(&a)?,
            solver,
            analysis,
            similarity: None,
        })
    }

    /// Whether the factors are L D Lᵀ of the similarity.
    pub(crate) fn symmetric(&self) -> bool {
        self.similarity.is_some()
    }

    /// The backend and the analysis, for a matrix of the same structure.
    pub(crate) fn plan(&self) -> Plan {
        Plan {
            solver: self.solver.clone(),
            analysis: Some(self.analysis.clone()),
        }
    }

    /// What the factorization took, as the backend reports it.
    pub(crate) fn report(&self) -> crate::backend::Report {
        self.factors.report()
    }

    /// The backend that factorized: its name and version.
    pub(crate) fn backend(&self) -> String {
        let c = self.solver.capabilities();
        format!("{} {}", c.name, c.version)
    }

    /// x with A x = `b`.
    ///
    /// # Errors
    ///
    /// The backend's.
    pub(crate) fn solve(&self, b: &[c64]) -> Result<Vec<c64>> {
        match &self.similarity {
            None => self.factors.solve(b),
            // B (S x) = S b
            Some(s) => {
                let sb: Vec<c64> = b.iter().zip(s).map(|(b, s)| b * s).collect();
                let y = self.factors.solve(&sb)?;
                Ok(y.iter().zip(s).map(|(y, s)| y / s).collect())
            }
        }
    }

    /// x with Aᵀ x = `b`.
    ///
    /// # Errors
    ///
    /// The backend's.
    pub(crate) fn solve_transpose(&self, b: &[c64]) -> Result<Vec<c64>> {
        match &self.similarity {
            None => self.factors.solve_transpose(b),
            // Aᵀ = S B S⁻¹: B (S⁻¹ x) = S⁻¹ b
            Some(s) => {
                let sb: Vec<c64> = b.iter().zip(s).map(|(b, s)| b / s).collect();
                let y = self.factors.solve(&sb)?;
                Ok(y.iter().zip(s).map(|(y, s)| y * s).collect())
            }
        }
    }
}
