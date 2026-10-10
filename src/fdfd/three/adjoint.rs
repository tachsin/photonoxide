//! Adjoint gradients in 3D: how a port mode's power changes with the permittivity of every value
//! of E, from one more solve with the same factors.
//!
//! As in 2D (G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004),
//! doi:10.1364/OL.29.002288, Eqs. 2–4): with A(ε) u = b and b independent of ε, an objective
//! F(u) changes as ∂F/∂ε_r = −2 Re(λᵀ (∂A/∂ε_r) u), where λ solves Aᵀ λ = ∂F/∂u. Here
//! A = −∇ × ∇ × + k₀² ε, so ∂A/∂ε_r = k₀² at the diagonal of value r alone.
//!
//! **Aᵀ from A's own factors.** With V the product of the PMLs' stretches at each value of E,
//! V A is symmetric (see the ports' docs), so Aᵀ = V A V⁻¹ and λ = V A⁻¹ (V⁻¹ ∂F/∂u): a solve
//! with A, its right-hand side and its solution scaled by V. Outside the PMLs V = 1.
//!
//! **The objective** is the power a port mode carries one way, F = |a|², its amplitude
//! a = Σ ωᵣ uᵣ linear in the field (the Lorentz form's weights,
//! [`super::ports::mode_amplitude_weights_of`]), so ∂F/∂u = ā ω, the mode's complex profile and
//! the form's phase whole.
//!
//! The iterative solver's adjoint ([`super::IterativeSolver3d::mode_power_gradient`]) is the
//! same solve with A by QMR or GMRES, and can recycle the forward solve's Krylov space
//! ([`super::Recycler`]): A is the forward's own operator.

use num_complex::Complex64 as c64;

use super::{Field3d, Lattice, PortMode3d, Solver3d};
use crate::fdfd::Direction;
use crate::{Error, Result};

impl Lattice {
    /// The amplitude a = Σ ωᵣ uᵣ of `mode` going `direction` in `field` (a field of this
    /// lattice), and the adjoint's right-hand side for |a|², V⁻¹ ā ω: its solve with A, scaled
    /// by V, is λ.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a field or a mode of another problem.
    pub(super) fn power_adjoint_source(
        &self,
        field: &Field3d,
        mode: &PortMode3d,
        direction: Direction,
    ) -> Result<(c64, Vec<c64>)> {
        let n = self.grid.unknowns();
        if field.values.len() != n || field.lattice != *self {
            return Err(Error::invalid(
                "fdfd gradient",
                "the field must belong to this problem",
            ));
        }
        let (size, step) = mode.shape();
        let (b, c) = mode.axis().others();
        if size != [self.grid.n(b), self.grid.n(c)]
            || step != self.grid.step(mode.axis())
            || mode.k0() != self.k0
            || mode.plane() + 1 >= self.grid.n(mode.axis())
        {
            return Err(Error::invalid(
                "fdfd gradient",
                "the mode must come from this problem's port_modes",
            ));
        }
        let weights = self.mode_amplitude_weights(mode, direction);
        let u = &field.values;
        let a: c64 = weights.iter().map(|&(r, w)| w * u[r]).sum();
        // λ = V A⁻¹ (V⁻¹ ā ω)
        let mut rhs = vec![c64::new(0.0, 0.0); n];
        for &(r, w) in &weights {
            rhs[r] = a.conj() * w / self.volume(r);
        }
        Ok((a, rhs))
    }

    /// ∂|a|²/∂ε at each value of E, −2 Re(λ k₀² u), from the field u and `solved`, the solve of
    /// A s = [`Lattice::power_adjoint_source`]'s right-hand side (λ = V s); zero on a wall.
    pub(super) fn power_gradient(&self, u: &[c64], solved: &[c64]) -> Vec<f64> {
        let k2 = self.k0 * self.k0;
        (0..self.grid.unknowns())
            .map(|r| {
                if self.fixed(r) {
                    0.0
                } else {
                    let lambda = self.volume(r) * solved[r];
                    -2.0 * (lambda * k2 * u[r]).re
                }
            })
            .collect()
    }
}

impl Solver3d {
    /// The power `mode` carries `direction` in `field` (a field of this problem), |a|² with a
    /// its amplitude as [`Field3d::mode_amplitudes`] measures it, and its gradient with respect
    /// to the permittivity (its real part) each value of E sees, one per value as
    /// [`super::Grid3d::index`] numbers them (zero for values fixed at zero on a wall).
    ///
    /// The gradient is exact when the source doesn't depend on the permittivity: the values
    /// varied must be away from the source's plane, and the mode is held fixed.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a field or a mode of another problem, or if the solve fails.
    pub fn mode_power_gradient(
        &self,
        field: &Field3d,
        mode: &PortMode3d,
        direction: Direction,
    ) -> Result<(f64, Vec<f64>)> {
        let (a, rhs) = self.lattice.power_adjoint_source(field, mode, direction)?;
        let solved = self.solve_system(&rhs)?;
        Ok((
            a.norm_sqr(),
            self.lattice.power_gradient(&field.values, &solved.values),
        ))
    }
}

/// A rectangular guide along z (ε = 12, 0.4 × 0.3 µm, in 2.1) with a block of ε = 6 beside it
/// (0.1 × 0.1 × 0.15 µm), on 14 × 12 × 24 cells of 50 nm with PMLs of 4, at 1.55 µm: the guide's
/// mode launched forward on plane 7, its power ahead on plane 16. The adjoint gradient of that
/// power against fourth-order central differences (δ = 1e-3) at a value of each component in
/// the block, the core and the cladding between the planes: the largest difference relative to
/// the largest of those gradients, and the gradients by both.
pub(crate) fn gradient_against_differences() -> (f64, Vec<(f64, f64)>) {
    use super::{Axis, Boundaries3d, Grid3d};
    use crate::units::Wavelength;
    let (nx, ny, nz, h) = (14usize, 12usize, 24usize, 0.05);
    let grid = Grid3d {
        nx,
        ny,
        nz,
        dx: h,
        dy: h,
        dz: h,
        x0: -(nx as f64) * h / 2.0,
        y0: -(ny as f64) * h / 2.0,
        z0: -(nz as f64) * h / 2.0,
    };
    let eps = |x: f64, y: f64, z: f64| {
        let v = if x.abs() < 0.2 && y.abs() < 0.15 {
            12.0
        } else if (0.2..0.3).contains(&x) && (0.0..0.1).contains(&y) && (0.0..0.15).contains(&z) {
            6.0
        } else {
            2.1
        };
        c64::new(v, 0.0)
    };
    let solver = Solver3d::new(
        grid,
        Wavelength::um(1.55).unwrap(),
        eps,
        Boundaries3d::pml(4),
    )
    .unwrap();
    let source = solver.port_modes(Axis::Z, 7, 1).unwrap().remove(0);
    let ahead = solver.port_modes(Axis::Z, 16, 1).unwrap().remove(0);
    let rhs = solver.mode_source(&source, Direction::Forward);
    let power = |s: &Solver3d| {
        let field = s.solve_system(&rhs).unwrap();
        field.mode_amplitudes(&ahead).0.norm_sqr()
    };
    let field = solver.solve_system(&rhs).unwrap();
    let (_, gradient) = solver
        .mode_power_gradient(&field, &ahead, Direction::Forward)
        .unwrap();
    // in the block, the core and the cladding, between the planes
    let values = [
        grid.index(Axis::X, (11, 6, 12)),
        grid.index(Axis::Y, (12, 6, 12)),
        grid.index(Axis::Z, (11, 7, 11)),
        grid.index(Axis::X, (7, 6, 12)),
        grid.index(Axis::Y, (6, 5, 10)),
        grid.index(Axis::Z, (6, 6, 13)),
        grid.index(Axis::X, (2, 2, 12)),
        grid.index(Axis::Z, (7, 10, 9)),
    ];
    let delta = 1e-3;
    let pairs: Vec<(f64, f64)> = values
        .iter()
        .map(|&r| {
            let at = |d: f64| {
                let mut eps = solver.values().to_vec();
                eps[r] += d;
                power(&solver.with_values(eps).unwrap())
            };
            let difference = (8.0 * (at(delta) - at(-delta))
                - (at(2.0 * delta) - at(-2.0 * delta)))
                / (12.0 * delta);
            (gradient[r], difference)
        })
        .collect();
    let largest = pairs.iter().fold(0.0f64, |m, p| m.max(p.1.abs()));
    let worst = pairs.iter().fold(0.0f64, |m, (a, d)| m.max((a - d).abs())) / largest;
    (worst, pairs)
}
