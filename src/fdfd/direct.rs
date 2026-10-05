//! The 2D and 3D solvers' direct solves: the multifrontal factorization of [`crate::sparse`],
//! as L D Lᵀ of the system's complex symmetric similarity when it has one, as LU otherwise.
//!
//! The curl-curl operator with PMLs, and the 2D operators, are similar to a complex symmetric
//! matrix: B = S A S⁻¹ with S² the diagonal D for which D A is symmetric (the cells' stretch
//! factors; [`super::krylov::Sparse::symmetrized`]). Then A x = b is B (S x) = S b, and B's
//! L D Lᵀ stores half of A's LU and takes about half its work (docs/baselines.md). A Bloch
//! side's phases break the symmetry, and a system with them is factorized by LU; so is one
//! whose L D Lᵀ needed a pivot perturbed, as the LU's pivoting inside its blocks is the safer.

use std::sync::Arc;

use faer::sparse::{SparseColMat, Triplet};
use num_complex::Complex64 as c64;

use super::krylov::Sparse;
use crate::sparse::{Analysis, Multifrontal};
use crate::{Error, Result};

/// A system's factors, with the similarity they were taken through.
pub(crate) struct Direct {
    factors: Multifrontal,
    /// S, if the factors are B's: B = S A S⁻¹.
    similarity: Option<Vec<c64>>,
}

impl Direct {
    /// The factors of the n × n matrix of `entries`, ordered by nested dissection on the
    /// unknowns' `positions`; with `analysis` if given (that of a matrix of the same structure,
    /// which also says whether to factorize the similarity).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the matrix can't be assembled or factorized.
    pub(crate) fn new(
        entries: &[Triplet<usize, usize, c64>],
        positions: &[[f64; 3]],
        analysis: Option<Arc<Analysis>>,
    ) -> Result<Direct> {
        let n = positions.len();
        let assemble = |t: &[Triplet<usize, usize, c64>]| {
            SparseColMat::<usize, c64>::try_new_from_triplets(n, n, t)
                .map_err(|e| Error::invalid("fdfd", format!("can't assemble the matrix: {e:?}")))
        };
        let symmetric_wanted = analysis.as_ref().is_none_or(|a| a.symmetric());
        if symmetric_wanted {
            let a = Sparse::new(n, entries.iter().map(|t| (t.row, t.col, t.val)));
            if let Some((s, b)) = a.symmetrized() {
                let triplets: Vec<Triplet<usize, usize, c64>> = (0..n)
                    .flat_map(|r| b.row(r).map(move |(c, v)| Triplet::new(r, c, v)))
                    .collect();
                let b = assemble(&triplets)?;
                let analysis = match &analysis {
                    Some(a) => Some(a.clone()),
                    None => Analysis::new_symmetric(b.as_ref(), Some(positions))?.map(Arc::new),
                };
                if let Some(analysis) = analysis {
                    let factors = Multifrontal::new(analysis, b.as_ref())?;
                    if factors.perturbed() == 0 {
                        return Ok(Direct {
                            factors,
                            similarity: Some(s),
                        });
                    }
                }
            }
        }
        let a = assemble(entries)?;
        let analysis = match analysis.filter(|a| !a.symmetric()) {
            Some(a) => a,
            None => Arc::new(Analysis::new(a.as_ref(), Some(positions))?),
        };
        Ok(Direct {
            factors: Multifrontal::new(analysis, a.as_ref())?,
            similarity: None,
        })
    }

    /// The LU of the n × n matrix of `entries`, whether or not it has a symmetric similarity.
    ///
    /// # Errors
    ///
    /// As [`Direct::new`].
    pub(crate) fn lu(
        entries: &[Triplet<usize, usize, c64>],
        positions: &[[f64; 3]],
    ) -> Result<Direct> {
        let n = positions.len();
        let a = SparseColMat::<usize, c64>::try_new_from_triplets(n, n, entries)
            .map_err(|e| Error::invalid("fdfd", format!("can't assemble the matrix: {e:?}")))?;
        let analysis = Arc::new(Analysis::new(a.as_ref(), Some(positions))?);
        Ok(Direct {
            factors: Multifrontal::new(analysis, a.as_ref())?,
            similarity: None,
        })
    }

    /// Whether the factors are L D Lᵀ of the similarity.
    pub(crate) fn symmetric(&self) -> bool {
        self.similarity.is_some()
    }

    /// The analysis, for a matrix of the same structure.
    pub(crate) fn analysis(&self) -> &Arc<Analysis> {
        self.factors.analysis()
    }

    /// x with A x = `b`.
    pub(crate) fn solve(&self, b: &[c64]) -> Vec<c64> {
        match &self.similarity {
            None => self.factors.solve(b),
            // B (S x) = S b
            Some(s) => {
                let sb: Vec<c64> = b.iter().zip(s).map(|(b, s)| b * s).collect();
                let y = self.factors.solve(&sb);
                y.iter().zip(s).map(|(y, s)| y / s).collect()
            }
        }
    }

    /// x with Aᵀ x = `b`.
    pub(crate) fn solve_transpose(&self, b: &[c64]) -> Vec<c64> {
        match &self.similarity {
            None => self.factors.solve_transpose(b),
            // Aᵀ = S B S⁻¹: B (S⁻¹ x) = S⁻¹ b
            Some(s) => {
                let sb: Vec<c64> = b.iter().zip(s).map(|(b, s)| b / s).collect();
                let y = self.factors.solve(&sb);
                y.iter().zip(s).map(|(y, s)| y * s).collect()
            }
        }
    }
}
