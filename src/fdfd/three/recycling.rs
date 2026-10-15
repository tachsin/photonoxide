//! Recycling from one 3D solve to the next: a wavelength sweep, a forward solve and its adjoint,
//! an S-matrix's ports (docs/methods/recycling.md).

use std::collections::VecDeque;

use num_complex::Complex64 as c64;

use super::{IterativeSolver3d, Preconditioning};
use crate::Result;
use crate::fdfd::Direction;
use crate::fdfd::krylov::{Convergence, Space, Stopping, gcro_dr, least_residual};
use crate::fdfd::three::{Field3d, Port3d, PortMode3d};

/// What one solve leaves for the next, carried by the caller through
/// [`IterativeSolver3d::solve_system_recycled`], [`IterativeSolver3d::s_matrix_recycled`] and
/// [`IterativeSolver3d::mode_power_gradient_recycled`]: a wavelength sweep's solvers in turn, a
/// forward solve and its adjoint, an S-matrix's ports.
///
/// - **Earlier solutions** ([`Recycler::with_solutions`]): the last `count` fields solved,
///   whose combination of least residual starts the next solve (P. F. Fischer, Comput. Methods
///   Appl. Mech. Engrg. 163, 193 (1998), doi:10.1016/S0045-7825(98)00012-7, Method 1); on every
///   path, QMR's included. For a sweep, where a wavelength's field is near the last ones', or an
///   optimization's designs: keep two per port (or per forward and adjoint) and wavelength
///   or design to be extrapolated from; 8 took a two-port coupler's 16-wavelength sweep from
///   1 419 iterations to 435 (docs/methods/recycling.md).
/// - **A recycled Krylov space** ([`Recycler::with_krylov`]) for the GMRES path preconditioned by
///   the multigrid cycle ([`IterativeSolver3d::with_multigrid`]): GCRO-DR (M. L. Parks, E. de
///   Sturler, G. Mackey, D. D. Johnson, S. Maiti, SIAM J. Sci. Comput. 28, 1651 (2006),
///   doi:10.1137/040607277) keeping `vectors` harmonic Ritz vectors of A M⁻¹ from each cycle.
///   On the same solver (an adjoint, the next port) the space is used as it is; on a new one
///   (the next wavelength) it costs `vectors` cycles of the new multigrid to carry over. It
///   must be fewer than the multigrid's restart (40 by default), which is GCRO-DR's cycle. On
///   other paths it isn't used. Measured, it saves little there: the multigrid's operator has
///   many small eigenvalues, and an adjoint after its forward took 41 iterations against 42.
///
/// Neither changes what a solve is held to: each field to its own relative residual, as the
/// plain solve.
#[derive(Clone, Debug)]
pub struct Recycler {
    vectors: usize,
    solutions: usize,
    space: Space,
    earlier: VecDeque<Earlier>,
}

/// An earlier solution, and its product with the operator of the solver named, once taken.
#[derive(Clone, Debug)]
struct Earlier {
    values: Vec<c64>,
    product: Option<(u64, Vec<c64>)>,
}

impl Default for Recycler {
    fn default() -> Recycler {
        Recycler::new()
    }
}

impl Recycler {
    /// A recycler that recycles nothing yet: no earlier solutions, no Krylov space.
    pub fn new() -> Recycler {
        Recycler {
            vectors: 0,
            solutions: 0,
            space: Space::default(),
            earlier: VecDeque::new(),
        }
    }

    /// The same, each solve started from the combination of the last `count` solutions of
    /// least residual.
    pub fn with_solutions(mut self, count: usize) -> Recycler {
        self.solutions = count;
        self
    }

    /// The same, with GCRO-DR keeping `vectors` harmonic Ritz vectors on the multigrid's path
    /// (0: plain GMRES).
    pub fn with_krylov(mut self, vectors: usize) -> Recycler {
        self.vectors = vectors;
        self
    }

    /// The recycled Krylov space's dimension now.
    pub fn dimension(&self) -> usize {
        self.space.dimension()
    }

    /// The products with A M⁻¹ (the multigrid's cycles) spent carrying the space to new
    /// solvers, summed over the solves.
    pub fn products(&self) -> usize {
        self.space.products
    }

    /// Forgets everything: the next solve starts from nothing.
    pub fn clear(&mut self) {
        self.space = Space::default();
        self.earlier.clear();
    }
}

impl IterativeSolver3d {
    /// A E = b with A·x for this formulation's matrix (or operator), in E's variables: through
    /// the similarity S where the solver iterates on B = S A S⁻¹.
    fn system_product(&self, x: &[c64]) -> Vec<c64> {
        use crate::fdfd::krylov::Operator;
        let similar = |b: &dyn Fn(&[c64]) -> Vec<c64>, s: &[c64]| {
            let sx: Vec<c64> = x.iter().zip(s).map(|(p, q)| p * q).collect();
            b(&sx).iter().zip(s).map(|(p, q)| p / q).collect()
        };
        match (&self.free, &self.similarity) {
            (Some(operator), Some(s)) => similar(&|v| operator.apply(v), s),
            (Some(operator), None) => operator.apply(x),
            (None, Some(s)) => similar(&|v| self.matrix.apply(v), s),
            (None, None) => self.matrix.apply(x),
        }
    }

    /// As [`IterativeSolver3d::solve_system`], recycling what `recycler` holds from earlier
    /// solves and leaving it what this one learned ([`Recycler`]). The field is held to the same
    /// relative residual; its iterations are this solve's Arnoldi or Lanczos steps (the
    /// products spent carrying the space over are [`Recycler::products`]).
    ///
    /// # Errors
    ///
    /// As [`IterativeSolver3d::solve_system`], and [`crate::Error::InvalidValue`] if the
    /// recycler keeps as many vectors as the multigrid's restart, or more.
    pub fn solve_system_recycled(
        &self,
        rhs: &[c64],
        stopping: Stopping,
        recycler: &mut Recycler,
    ) -> Result<(Field3d, Convergence)> {
        let b = self.prepared(rhs)?;
        let (values, convergence) = self.solve_recycled(&b, stopping, recycler)?;
        Ok((
            Field3d {
                lattice: self.lattice.clone(),
                values,
            },
            convergence,
        ))
    }

    /// [`IterativeSolver3d::solve_system_recycled`] for a prepared right-hand side.
    fn solve_recycled(
        &self,
        b: &[c64],
        stopping: Stopping,
        recycler: &mut Recycler,
    ) -> Result<(Vec<c64>, Convergence)> {
        let n = b.len();
        let zero = c64::new(0.0, 0.0);
        recycler.earlier.retain(|e| e.values.len() == n);
        let norm = |v: &[c64]| v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
        let rho0 = norm(b);
        // the earlier solutions' combination of least residual, each one's product with this
        // operator taken once
        let (x0, r0) = if recycler.solutions > 0 && !recycler.earlier.is_empty() && rho0 > 0.0 {
            for e in &mut recycler.earlier {
                if e.product.as_ref().map(|p| p.0) != Some(self.id) {
                    e.product = Some((self.id, self.system_product(&e.values)));
                }
            }
            let xs: Vec<&[c64]> = recycler.earlier.iter().map(|e| &e.values[..]).collect();
            let axs: Vec<&[c64]> = recycler
                .earlier
                .iter()
                .filter_map(|e| e.product.as_ref().map(|p| &p.1[..]))
                .collect();
            let x0 = least_residual(b, &xs, &axs);
            // the true residual, which the solve continues from: the combination's own may
            // differ from it by the cancellation among nearly dependent solutions
            let ax0 = self.system_product(&x0);
            let r0 = b.iter().zip(&ax0).map(|(p, q)| p - q).collect();
            (x0, r0)
        } else {
            (vec![zero; n], b.to_vec())
        };
        let left = norm(&r0);
        let (dx, how) = if rho0 == 0.0 || left <= stopping.tolerance * rho0 {
            (
                vec![zero; n],
                Convergence {
                    iterations: 0,
                    residual: 0.0,
                    history: Vec::new(),
                },
            )
        } else {
            let inner = Stopping {
                tolerance: stopping.tolerance * rho0 / left,
                max_iterations: stopping.max_iterations,
            };
            match (&self.preconditioner, &self.backend) {
                (Preconditioning::Multigrid(h), None) if recycler.vectors > 0 => gcro_dr(
                    &self.matrix,
                    h.as_ref(),
                    &r0,
                    inner,
                    h.restart(),
                    recycler.vectors,
                    &mut recycler.space,
                    self.id,
                )?,
                _ => self.solve_prepared(&r0, inner)?,
            }
        };
        let x: Vec<c64> = x0.iter().zip(&dx).map(|(p, q)| p + q).collect();
        let scale = if rho0 == 0.0 { 0.0 } else { left / rho0 };
        let convergence = Convergence {
            iterations: how.iterations,
            residual: how.residual * scale,
            history: how.history.iter().map(|h| h * scale).collect(),
        };
        if recycler.solutions > 0 {
            if recycler.earlier.len() == recycler.solutions {
                recycler.earlier.pop_front();
            }
            recycler.earlier.push_back(Earlier {
                values: x.clone(),
                product: None,
            });
        }
        Ok((x, convergence))
    }

    /// As [`IterativeSolver3d::s_matrix`], each port's run by
    /// [`IterativeSolver3d::solve_system_recycled`] in turn: the space the last solve left serves
    /// the next port, and the last port's the next caller.
    ///
    /// # Errors
    ///
    /// As [`IterativeSolver3d::s_matrix`] and [`IterativeSolver3d::solve_system_recycled`].
    pub fn s_matrix_recycled(
        &self,
        ports: &[Port3d],
        stopping: Stopping,
        recycler: &mut Recycler,
    ) -> Result<Vec<Vec<c64>>> {
        Ok(self
            .s_matrix_recycled_with_convergence(ports, stopping, recycler)?
            .0)
    }

    /// As [`IterativeSolver3d::s_matrix_recycled`], with how each port's run went.
    pub(crate) fn s_matrix_recycled_with_convergence(
        &self,
        ports: &[Port3d],
        stopping: Stopping,
        recycler: &mut Recycler,
    ) -> Result<(Vec<Vec<c64>>, Vec<Convergence>)> {
        let mut runs = Vec::with_capacity(ports.len());
        let s = self.lattice.s_matrix(ports, |rhs| {
            let (field, convergence) = self.solve_system_recycled(rhs, stopping, recycler)?;
            runs.push(convergence);
            Ok(field)
        })?;
        Ok((s, runs))
    }

    /// As [`super::Solver3d::mode_power_gradient`], the adjoint solved by this solver to the
    /// relative residual `stopping` asks: the power `mode` carries `direction` in `field`, and
    /// its gradient with respect to the permittivity at each value of E. The adjoint is a solve
    /// with A itself (A's transpose is V A V⁻¹, V the PMLs' stretches), so on Shin and Fan's
    /// operator too it is the forward's operator; how it went is the third value.
    ///
    /// # Errors
    ///
    /// As [`super::Solver3d::mode_power_gradient`] and [`IterativeSolver3d::solve_system`].
    pub fn mode_power_gradient(
        &self,
        field: &Field3d,
        mode: &PortMode3d,
        direction: Direction,
        stopping: Stopping,
    ) -> Result<(f64, Vec<f64>, Convergence)> {
        let (a, rhs) = self.lattice.power_adjoint_source(field, mode, direction)?;
        let (solved, how) = self.solve_system(&rhs, stopping)?;
        Ok((
            a.norm_sqr(),
            self.lattice.power_gradient(&field.values, &solved.values),
            how,
        ))
    }

    /// [`IterativeSolver3d::mode_power_gradient`], its adjoint solved by
    /// [`IterativeSolver3d::solve_system_recycled`]: after the forward solve with the same
    /// recycler, the forward's Krylov space serves the adjoint at no cost.
    ///
    /// # Errors
    ///
    /// As [`IterativeSolver3d::mode_power_gradient`].
    pub fn mode_power_gradient_recycled(
        &self,
        field: &Field3d,
        mode: &PortMode3d,
        direction: Direction,
        stopping: Stopping,
        recycler: &mut Recycler,
    ) -> Result<(f64, Vec<f64>, Convergence)> {
        let (a, rhs) = self.lattice.power_adjoint_source(field, mode, direction)?;
        let (solved, how) = self.solve_system_recycled(&rhs, stopping, recycler)?;
        Ok((
            a.norm_sqr(),
            self.lattice.power_gradient(&field.values, &solved.values),
            how,
        ))
    }
}
