//! The circuit adjoint: how a circuit's response changes with every parameter of every instance,
//! from one more back-substitution with the transposed system.
//!
//! The circuit solve is M b = S_b E x with M = I − S_b Γ (see [`Circuit`]), and S = Eᵀ B for the
//! outgoing waves B, a column per external input. Differentiating M B = S_b E with respect to a
//! real parameter θ, M ∂B = ∂S_b E + ∂S_b Γ B, so
//!
//! ∂S/∂θ = Eᵀ M⁻¹ (∂S_b/∂θ) A,  A = Γ B + E,
//!
//! A the waves going into every port. A real objective F of S (of S at several wavelengths,
//! summed) changes as dF = 2 Re Σ G_qp dS_qp, with G_qp = ∂F/∂S_qp its Wirtinger derivative
//! (½(∂/∂Re S_qp − i ∂/∂Im S_qp); for F = |S_qp|², G_qp = conj(S_qp)). Then
//!
//! ∂F/∂θ = 2 Re tr(Λᵀ (∂S_b/∂θ) A),  Mᵀ Λ = E G,
//!
//! the adjoint variable method of G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004),
//! doi:10.1364/OL.29.002288, Eqs. 2–4, for a system whose right-hand side depends on θ as well:
//! their −2 Re(λᵀ ∂A/∂θ u) becomes 2 Re(λᵀ (∂r/∂θ − ∂M/∂θ b)), and with r = S_b E x and
//! ∂M/∂θ = −∂S_b/∂θ Γ that is 2 Re(λᵀ ∂S_b/∂θ a). S depends on S_b holomorphically, so the
//! adjoint system is Mᵀ, transposed and not conjugated, and solves with the factorization of M
//! the circuit solve made. ∂S_b/∂θ is one instance's block, so each parameter costs a product of
//! its component's size, whatever the circuit's.
//!
//! **∂S_b/∂θ** is each component's [`Component::derivatives`], or, for a component that gives
//! none, fourth-order central differences of its [`Component::s_matrix`], with a step
//! h = ε^(1/3) max(|θ|, 1) ([`STEP`]), at most an eighth of the parameter's range, and
//! fourth-order one-sided differences inwards at a bound. A nested [`Circuit`] gives its own
//! derivatives exactly, by [`Circuit::jacobian`].

pub(crate) mod checks;
#[cfg(test)]
mod tests;

use faer::Mat;
use faer::linalg::solvers::Solve;
use num_complex::Complex64 as c64;

use super::solve::Solved;
use super::{Circuit, Component, SMatrix};
use crate::units::Wavelength;
use crate::{Error, Result};

/// The relative step of the finite differences a component without
/// [`Component::derivatives`] is differentiated by: ε^(1/3) = 2^(−52/3) ≈ 6.06e-6, ε being
/// `f64::EPSILON`; the step is h = `STEP` · max(|θ|, 1), rounded so that θ + h is exact.
pub const STEP: f64 = 6.055_454_452_393_339_5e-6;

impl Circuit {
    /// The position of the parameter named `name` (`instance.parameter`) in
    /// [`Component::parameters`] and in the values.
    pub fn parameter(&self, name: &str) -> Option<usize> {
        self.parameters().iter().position(|p| p.name == name)
    }

    /// The circuit's S-matrix at `wavelength`, with its parameters at `values`, and ∂S/∂θ_k for
    /// every parameter, in the order of [`Component::parameters`], by the circuit adjoint (see
    /// [the circuit adjoint](crate::circuit::adjoint)): one solve with the transposed system, a right-hand side per external
    /// port, and a product of each component's size per parameter.
    ///
    /// # Errors
    ///
    /// As [`Circuit::s_matrix_with`], a component's own from its
    /// [`Component::derivatives`], and [`Error::InvalidValue`] for derivatives of the wrong
    /// number or size.
    pub fn jacobian(
        &self,
        wavelength: Wavelength,
        values: &[f64],
    ) -> Result<(SMatrix, Vec<SMatrix>)> {
        let k = self.external.len();
        let count = self.parameters().len();
        if k == 0 {
            return Ok((SMatrix::zeros(0), vec![SMatrix::zeros(0); count]));
        }
        let solved = self.solved(wavelength, values)?;
        let incoming = self.incoming(&solved.b);
        // Z = M⁻ᵀ E, so that Eᵀ M⁻¹ = Zᵀ
        let mut rhs = Mat::<c64>::zeros(self.partner.len(), k);
        for (c, &g) in self.external.iter().enumerate() {
            rhs[(g, c)] = c64::new(1.0, 0.0);
        }
        let z = transposed(&solved, &rhs);
        let mut jacobian = vec![SMatrix::zeros(k); count];
        self.each_derivative(wavelength, values, |index, start, ds| {
            let size = ds.size();
            jacobian[index] = SMatrix::from_fn(k, |q, p| {
                let mut sum = c64::new(0.0, 0.0);
                for a in 0..size {
                    let za = z[(start + a, q)];
                    for b in 0..size {
                        sum += za * ds[(a, b)] * incoming[(start + b, p)];
                    }
                }
                sum
            });
        })?;
        Ok((solved.s, jacobian))
    }

    /// A real objective F of the circuit's S-matrices at `wavelengths`, its parameters at
    /// `values`, and ∂F/∂θ for every parameter, in the order of [`Component::parameters`], by
    /// the circuit adjoint (see [the circuit adjoint](crate::circuit::adjoint)).
    ///
    /// `objective` takes the S-matrices, one per wavelength, and returns F and its sensitivity
    /// to each: the matrix G with G_qp = ∂F/∂S_qp, the Wirtinger derivative, so that
    /// dF = 2 Re Σ G_qp dS_qp; for F = |S_qp|², G_qp = conj(S_qp) and every other entry 0.
    /// [`super::objective`] has the common ones. A sum over wavelengths is the sum of each
    /// wavelength's own terms.
    ///
    /// Each wavelength takes the circuit solve and one solve with the transposed system, a
    /// right-hand side per input the objective depends on (a column of G with an entry not 0).
    ///
    /// # Errors
    ///
    /// As [`Circuit::jacobian`]; [`Error::InvalidValue`] for a circuit without external ports,
    /// or an objective that doesn't return one sensitivity per wavelength, each the size of S.
    pub fn gradient<F>(
        &self,
        wavelengths: &[Wavelength],
        values: &[f64],
        objective: F,
    ) -> Result<(f64, Vec<f64>)>
    where
        F: FnOnce(&[SMatrix]) -> (f64, Vec<SMatrix>),
    {
        let k = self.external.len();
        if k == 0 {
            return Err(Error::invalid(
                "circuit gradient",
                "a circuit without external ports has no response",
            ));
        }
        let solved = wavelengths
            .iter()
            .map(|&w| self.solved(w, values))
            .collect::<Result<Vec<_>>>()?;
        let s: Vec<SMatrix> = solved.iter().map(|x| x.s.clone()).collect();
        let (value, sensitivities) = objective(&s);
        if sensitivities.len() != wavelengths.len() || sensitivities.iter().any(|g| g.size() != k) {
            return Err(Error::invalid(
                "circuit gradient",
                format!(
                    "the objective must give one {k} × {k} sensitivity per wavelength: {} wavelengths, {} sensitivities",
                    wavelengths.len(),
                    sensitivities.len()
                ),
            ));
        }
        let mut gradient = vec![0.0; self.parameters().len()];
        for ((solved, g), &wavelength) in solved.iter().zip(&sensitivities).zip(wavelengths) {
            // the inputs F depends on: the columns of G with an entry not 0
            let inputs: Vec<usize> = (0..k)
                .filter(|&p| (0..k).any(|q| g[(q, p)] != c64::new(0.0, 0.0)))
                .collect();
            if inputs.is_empty() {
                continue;
            }
            let mut rhs = Mat::<c64>::zeros(self.partner.len(), inputs.len());
            for (c, &p) in inputs.iter().enumerate() {
                for (q, &e) in self.external.iter().enumerate() {
                    rhs[(e, c)] = g[(q, p)];
                }
            }
            let lambda = transposed(solved, &rhs);
            let incoming = self.incoming(&solved.b);
            self.each_derivative(wavelength, values, |index, start, ds| {
                let size = ds.size();
                let mut sum = c64::new(0.0, 0.0);
                for (c, &p) in inputs.iter().enumerate() {
                    for a in 0..size {
                        let la = lambda[(start + a, c)];
                        for b in 0..size {
                            sum += la * ds[(a, b)] * incoming[(start + b, p)];
                        }
                    }
                }
                gradient[index] += 2.0 * sum.re;
            })?;
        }
        Ok((value, gradient))
    }

    /// A = Γ B + E: the waves going into every port, a column per external input.
    fn incoming(&self, b: &Mat<c64>) -> Mat<c64> {
        let (n, k) = (b.nrows(), b.ncols());
        let mut a = Mat::<c64>::zeros(n, k);
        for (i, partner) in self.partner.iter().enumerate() {
            if let Some(j) = *partner {
                for p in 0..k {
                    a[(i, p)] = b[(j, p)];
                }
            }
        }
        for (p, &g) in self.external.iter().enumerate() {
            a[(g, p)] += c64::new(1.0, 0.0);
        }
        a
    }

    /// Calls `f(parameter, first port, ∂S_i/∂θ)` for every parameter of every instance i, with
    /// the circuit's parameters at `values`.
    fn each_derivative(
        &self,
        wavelength: Wavelength,
        values: &[f64],
        mut f: impl FnMut(usize, usize, &SMatrix),
    ) -> Result<()> {
        for (i, inst) in self.netlist().instances().iter().enumerate() {
            let count = inst.component.parameters().len();
            if count == 0 {
                continue;
            }
            let first = self.parameter_offsets[i];
            let own = &values[first..first + count];
            let derivatives = derivatives(inst.component.as_ref(), wavelength, own, &inst.name)?;
            for (t, ds) in derivatives.iter().enumerate() {
                f(first + t, self.offsets[i], ds);
            }
        }
        Ok(())
    }
}

/// X of Mᵀ X = `rhs`, with the circuit solve's factorization of M and one step of iterative
/// refinement.
fn transposed(solved: &Solved, rhs: &Mat<c64>) -> Mat<c64> {
    let mut x = solved.lu.solve_transpose(rhs);
    let mut residual = rhs.clone();
    for t in &solved.triplets {
        for c in 0..rhs.ncols() {
            residual[(t.col, c)] -= t.val * x[(t.row, c)];
        }
    }
    let correction = solved.lu.solve_transpose(&residual);
    x += &correction;
    x
}

/// ∂S/∂θ_k of `component` for each of its parameters, at `values`: its own derivatives, checked,
/// or finite differences.
fn derivatives(
    component: &dyn Component,
    wavelength: Wavelength,
    values: &[f64],
    instance: &str,
) -> Result<Vec<SMatrix>> {
    let ports = component.ports().len();
    match component.derivatives(wavelength, values)? {
        Some(d) => {
            if d.len() != values.len() || d.iter().any(|m| m.size() != ports) {
                return Err(Error::invalid(
                    "circuit gradient",
                    format!(
                        "instance {instance} gives {} derivatives for {} parameters, each must be {ports} × {ports}",
                        d.len(),
                        values.len()
                    ),
                ));
            }
            Ok(d)
        }
        None => (0..values.len())
            .map(|k| finite_difference(component, wavelength, values, k))
            .collect(),
    }
}

/// ∂S/∂θ_k of `component` at `values` by fourth-order finite differences of its S-matrix (see
/// the module's docs): central where θ ± 2h is in the parameter's range, one-sided inwards from a
/// bound. Zero for a parameter whose range is a single value.
///
/// # Errors
///
/// The component's own.
pub(crate) fn finite_difference(
    component: &dyn Component,
    wavelength: Wavelength,
    values: &[f64],
    k: usize,
) -> Result<SMatrix> {
    let parameter = &component.parameters()[k];
    let size = component.ports().len();
    let x = values[k];
    let span = parameter.max - parameter.min;
    if span.is_nan() || span <= 0.0 {
        return Ok(SMatrix::zeros(size));
    }
    let wanted = (STEP * x.abs().max(1.0)).min(span / 8.0);
    // the step as the points have it
    let h = (x + wanted) - x;
    let at = |steps: f64| -> Result<SMatrix> {
        let mut moved = values.to_vec();
        moved[k] = (x + steps * h).clamp(parameter.min, parameter.max);
        component.s_matrix(wavelength, &moved)
    };
    let combine = |weights: &[(f64, f64)], scale: f64| -> Result<SMatrix> {
        let mut sum = SMatrix::zeros(size);
        for &(steps, w) in weights {
            let s = at(steps)?;
            for q in 0..size {
                for p in 0..size {
                    sum[(q, p)] += w * s[(q, p)];
                }
            }
        }
        Ok(SMatrix::from_fn(size, |q, p| sum[(q, p)] / scale))
    };
    if x - 2.0 * h >= parameter.min && x + 2.0 * h <= parameter.max {
        combine(
            &[(-2.0, 1.0), (-1.0, -8.0), (1.0, 8.0), (2.0, -1.0)],
            12.0 * h,
        )
    } else {
        // one-sided, fourth order: (−25 f₀ + 48 f₁ − 36 f₂ + 16 f₃ − 3 f₄) / 12h, forwards from
        // the lower bound, backwards (h → −h) from the upper
        let direction = if x - 2.0 * h < parameter.min {
            1.0
        } else {
            -1.0
        };
        combine(
            &[
                (0.0, -25.0),
                (direction, 48.0),
                (2.0 * direction, -36.0),
                (3.0 * direction, 16.0),
                (4.0 * direction, -3.0),
            ],
            12.0 * h * direction,
        )
    }
}
