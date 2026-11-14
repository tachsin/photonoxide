//! A Kerr medium in 2D FDFD, E along z, and its adjoint (T. W. Hughes, M. Minkov,
//! I. A. D. Williamson, S. Fan, "Adjoint method and inverse design for nonlinear nanophotonic
//! devices", ACS Photonics 5, 4781 (2018), doi:10.1021/acsphotonics.8b01522).
//!
//! **The problem.** An intensity-dependent permittivity ε̃ = ε + χ|u|² cell by cell (their
//! Eq. 14, with χ = 3χ⁽³⁾ for a field u in V/m; here u is in the solver's own units, so χ is the
//! change of ε per unit |u|² of the field the source drives) turns A(ε)u = b into
//! f(u, ū) = A(ε + χ|u|²) u − b = 0 (their Eq. 15). With E along z, ε enters A only on its
//! diagonal, k₀²ε, so f = A(ε)u + k₀² χ|u|² u − b.
//!
//! **Newton's method**, with u and ū independent (their Eqs. 4–5): ∂f/∂u = J = A(ε + 2χ|u|²)
//! and ∂f/∂ū = K = k₀² diag(χu²) (their Eqs. 16–17), so a step δ solves J δ + K δ̄ = −f. That
//! system is linear over the reals but not over the complex numbers; it is solved by GMRES on
//! the real vector space of δ (the real part of the inner product), preconditioned by J's own
//! factors: δ + J⁻¹(K δ̄) = −J⁻¹f, each iteration one solve with them. The first u is the linear
//! problem's field, or a given one (to follow a branch as the power grows: a Kerr device can
//! have several solutions, and Newton finds the one its start leads to). A step that doesn't
//! lower ‖f‖ is halved, at most twelve times. The solve stops when ‖f‖ ≤ 10⁻¹² ‖b‖.
//!
//! **The adjoint** of an objective F(u) real, here F = |cᵀu|² for any linear functional c (a
//! port mode's amplitude): with g = ∂F/∂u = conj(cᵀu) c, the adjoint field λ solves
//! Jᵀλ + K̄ λ̄ = g, linear over the reals again (their Eq. 6, transposed as Eq. 7 has it), and
//! then dF/dp = −2 Re(λᵀ ∂f/∂p) for any real parameter p (their Eq. 8). For a cell's
//! permittivity ε_k, ∂f/∂ε_k = k₀²(1 + χ′_k|u_k|²) u_k e_k, with χ′ = ∂χ/∂ε where the
//! nonlinearity follows the material (their Eq. 18 makes χ proportional to the density
//! (ε − 1)/(ε_m − 1), so χ′ = χ_m/(ε_m − 1)): [`Kerr2d::with_chi_slope`]. The adjoint is solved as
//! the step is, by GMRES with Jᵀ's factors, at the converged field's J. With χ = 0 everything
//! is the linear adjoint ([`Solver2d::functional_power_gradient`]).
//!
//! The source and the ports must lie where χ = 0 (their total-field/scattered-field rows are
//! the linear operator's), and the cells varied away from them, as for the linear adjoint.

use num_complex::Complex64 as c64;

use super::{Boundaries, Direction, Field2d, Grid, Polarization, PortMode, Solver2d};
use crate::units::Wavelength;
use crate::{Error, Result};

/// The relative residual ‖f‖/‖b‖ a Kerr solve stops at.
const TOLERANCE: f64 = 1e-12;
/// The most Newton steps.
const STEPS: usize = 60;

/// A 2D FDFD problem with E along z in a Kerr medium ([`Kerr2d::new`]); the module's docs have
/// the method.
pub struct Kerr2d {
    linear: Solver2d,
    wavelength: Wavelength,
    eps: Vec<c64>,
    chi: Vec<f64>,
    slope: Vec<f64>,
}

/// A solution of a [`Kerr2d`] problem: the field, how the solve went, and the factors of the
/// Newton step's matrix at it, which the adjoint uses.
pub struct KerrField {
    field: Field2d,
    jacobian: Solver2d,
    steps: usize,
    residual: f64,
}

impl KerrField {
    /// The field, E along z at the cells' centres.
    pub fn field(&self) -> &Field2d {
        &self.field
    }

    /// The Newton steps it took.
    pub fn steps(&self) -> usize {
        self.steps
    }

    /// ‖f(u)‖/‖b‖ at the field: below 10⁻¹².
    pub fn residual(&self) -> f64 {
        self.residual
    }
}

impl Kerr2d {
    /// The problem on `grid` at `wavelength`, with the linear relative permittivity `eps` and the
    /// Kerr coefficient `chi` cell by cell (`[j * nx + i]`: χ is the change of ε per unit |u|²
    /// of the solver's field, real; 0 for a linear cell), inside `boundaries`.
    ///
    /// # Errors
    ///
    /// As [`Solver2d::from_cells`], and [`Error::InvalidValue`] for a χ that isn't one finite
    /// value per cell.
    pub fn new(
        grid: Grid,
        wavelength: Wavelength,
        eps: &[c64],
        chi: &[f64],
        boundaries: Boundaries,
    ) -> Result<Kerr2d> {
        if chi.len() != grid.nx * grid.ny || chi.iter().any(|c| !c.is_finite()) {
            return Err(Error::invalid(
                "kerr",
                format!(
                    "needs {} finite values of chi, one per cell, got {}",
                    grid.nx * grid.ny,
                    chi.len()
                ),
            ));
        }
        let linear = Solver2d::from_cells(grid, Polarization::Ez, wavelength, eps, boundaries)?;
        Ok(Kerr2d {
            linear,
            wavelength,
            eps: eps.to_vec(),
            chi: chi.to_vec(),
            slope: vec![0.0; chi.len()],
        })
    }

    /// The same problem with χ′ = ∂χ/∂ε cell by cell, where the nonlinearity follows the
    /// material as ε changes (the module's docs): the adjoint's gradient then includes it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a slope that isn't one finite value per cell.
    pub fn with_chi_slope(mut self, slope: &[f64]) -> Result<Kerr2d> {
        if slope.len() != self.chi.len() || slope.iter().any(|c| !c.is_finite()) {
            return Err(Error::invalid(
                "kerr",
                "needs one finite slope of chi per cell",
            ));
        }
        self.slope = slope.to_vec();
        Ok(self)
    }

    /// The linear problem, ε alone: its ports, modes and sources are the Kerr problem's.
    pub fn linear(&self) -> &Solver2d {
        &self.linear
    }

    /// f(u) = A(ε)u + k₀²χ|u|²u − b.
    fn residual_of(&self, u: &[c64], rhs: &[c64]) -> Vec<c64> {
        let k2 = self.linear.k0 * self.linear.k0;
        let au = self.linear.apply(u);
        au.iter()
            .zip(u)
            .zip(&self.chi)
            .zip(rhs)
            .map(|(((a, u), chi), b)| a + u * (k2 * chi * u.norm_sqr()) - b)
            .collect()
    }

    /// The factors of J = A(ε + 2χ|u|²) at `u`.
    fn jacobian(&self, u: &[c64]) -> Result<Solver2d> {
        let eps: Vec<c64> = self
            .eps
            .iter()
            .zip(&self.chi)
            .zip(u)
            .map(|((e, chi), u)| e + 2.0 * chi * u.norm_sqr())
            .collect();
        self.linear.reuse_cells(self.wavelength, &eps)
    }

    /// The field for the right-hand side `rhs` (in A's form, as [`Solver2d::mode_source`] gives
    /// it), by Newton's method from the linear field or from `start`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a right-hand side or a start that isn't one finite value per
    /// cell of this problem, or a solve that doesn't converge within sixty steps (a power past
    /// where the branch it follows ends: start nearer, from a lower power's field).
    pub fn solve(&self, rhs: &[c64], start: Option<&Field2d>) -> Result<KerrField> {
        let linear = self.linear.solve_system(rhs)?;
        let mut u = match start {
            Some(f) if f.values.len() == rhs.len() => f.values.clone(),
            Some(_) => {
                return Err(Error::invalid(
                    "kerr",
                    "the start must be a field of this problem",
                ));
            }
            None => linear.values.clone(),
        };
        let size = norm(rhs).max(f64::MIN_POSITIVE);
        let k2 = self.linear.k0 * self.linear.k0;
        let mut f = self.residual_of(&u, rhs);
        let mut r = norm(&f) / size;
        let mut steps = 0;
        while r > TOLERANCE {
            if steps == STEPS {
                return Err(Error::invalid(
                    "kerr",
                    format!(
                        "Newton's method didn't converge in {STEPS} steps (residual {r:.2e}): \
                         start from a nearer field"
                    ),
                ));
            }
            steps += 1;
            let jacobian = self.jacobian(&u)?;
            let coupling: Vec<c64> = u
                .iter()
                .zip(&self.chi)
                .map(|(u, chi)| u * u * (k2 * chi))
                .collect();
            let minus_f: Vec<c64> = f.iter().map(|v| -v).collect();
            let delta = real_linear(
                &|v| jacobian.solve_system(v).map(|s| s.values),
                &coupling,
                &minus_f,
            )?;
            let mut t = 1.0;
            let mut accepted = false;
            for _ in 0..13 {
                let trial: Vec<c64> = u.iter().zip(&delta).map(|(u, d)| u + d * t).collect();
                let g = self.residual_of(&trial, rhs);
                let rg = norm(&g) / size;
                if rg < r {
                    u = trial;
                    f = g;
                    r = rg;
                    accepted = true;
                    break;
                }
                t *= 0.5;
            }
            if !accepted {
                return Err(Error::invalid(
                    "kerr",
                    format!("no Newton step lowers the residual {r:.2e}: start from a nearer field"),
                ));
            }
        }
        let jacobian = self.jacobian(&u)?;
        let mut field = linear;
        field.values = u;
        Ok(KerrField {
            field,
            jacobian,
            steps,
            residual: r,
        })
    }

    /// The power of a linear functional of the field, |cᵀu|² (c one coefficient per cell, as
    /// [`Solver2d::functional_power_gradient`] takes it), and its gradient with respect to each
    /// cell's linear relative permittivity, through the nonlinear problem (the module's docs).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for coefficients or a field that aren't one per cell of this
    /// problem, or an adjoint that doesn't converge.
    pub fn functional_power_gradient(
        &self,
        field: &KerrField,
        c: &[c64],
    ) -> Result<(f64, Vec<f64>)> {
        let n = self.chi.len();
        let u = &field.field.values;
        if c.len() != n || u.len() != n {
            return Err(Error::invalid(
                "kerr gradient",
                "the field and the coefficients must be one per cell of this problem",
            ));
        }
        let k2 = self.linear.k0 * self.linear.k0;
        let a: c64 = c.iter().zip(u).map(|(c, u)| c * u).sum();
        let g: Vec<c64> = c.iter().map(|c| a.conj() * c).collect();
        // Jᵀλ + K̄λ̄ = g
        let coupling: Vec<c64> = u
            .iter()
            .zip(&self.chi)
            .map(|(u, chi)| (u * u * (k2 * chi)).conj())
            .collect();
        let jacobian = &field.jacobian;
        let lambda = real_linear(&|v| jacobian.solve_transposed(v), &coupling, &g)?;
        let gradient = (0..n)
            .map(|k| {
                let df = u[k] * (k2 * (1.0 + self.slope[k] * u[k].norm_sqr()));
                -2.0 * (lambda[k] * df).re
            })
            .collect();
        Ok((a.norm_sqr(), gradient))
    }

    /// The power `mode` carries `direction` in `field` and its gradient, as
    /// [`Kerr2d::functional_power_gradient`] with the mode's coefficients at its column.
    ///
    /// # Errors
    ///
    /// As [`Kerr2d::functional_power_gradient`], and for a mode of another problem.
    pub fn mode_power_gradient(
        &self,
        field: &KerrField,
        mode: &PortMode,
        direction: Direction,
    ) -> Result<(f64, Vec<f64>)> {
        let c = self
            .linear
            .mode_coefficients(&field.field, mode, direction)?;
        self.functional_power_gradient(field, &c)
    }
}

/// ‖v‖.
fn norm(v: &[c64]) -> f64 {
    v.iter().map(|x| x.norm_sqr()).sum::<f64>().sqrt()
}

/// x of B x + D x̄ = r (D diagonal, `d`), with `solve` applying B⁻¹: GMRES over the reals on
/// x + B⁻¹(D x̄) = B⁻¹r, restarted every 80 iterations, to 10⁻¹³ of the right side.
fn real_linear(
    solve: &dyn Fn(&[c64]) -> Result<Vec<c64>>,
    d: &[c64],
    r: &[c64],
) -> Result<Vec<c64>> {
    let apply = |x: &[c64]| -> Result<Vec<c64>> {
        let dx: Vec<c64> = x.iter().zip(d).map(|(x, d)| d * x.conj()).collect();
        let b = solve(&dx)?;
        Ok(x.iter().zip(b).map(|(x, b)| x + b).collect())
    };
    let dot = |a: &[c64], b: &[c64]| -> f64 { a.iter().zip(b).map(|(a, b)| (a.conj() * b).re).sum() };
    let rhs = solve(r)?;
    let size = norm(&rhs);
    let mut x = vec![c64::new(0.0, 0.0); r.len()];
    if size == 0.0 {
        return Ok(x);
    }
    let goal = 1e-13 * size;
    let m = 80;
    for _ in 0..10 {
        let ax = apply(&x)?;
        let residual: Vec<c64> = rhs.iter().zip(&ax).map(|(b, a)| b - a).collect();
        let beta = norm(&residual);
        if beta <= goal {
            return Ok(x);
        }
        let mut basis: Vec<Vec<c64>> = vec![residual.iter().map(|v| v / beta).collect()];
        let mut h: Vec<Vec<f64>> = Vec::new();
        let (mut cs, mut sn): (Vec<f64>, Vec<f64>) = (Vec::new(), Vec::new());
        let mut g = vec![beta];
        let mut done = false;
        for j in 0..m {
            let mut w = apply(&basis[j])?;
            let mut column = vec![0.0; j + 2];
            for (i, v) in basis.iter().enumerate() {
                let hij = dot(v, &w);
                column[i] = hij;
                for (w, v) in w.iter_mut().zip(v) {
                    *w -= v * hij;
                }
            }
            let next = norm(&w);
            column[j + 1] = next;
            for i in 0..j {
                let (a, b) = (column[i], column[i + 1]);
                column[i] = cs[i] * a + sn[i] * b;
                column[i + 1] = -sn[i] * a + cs[i] * b;
            }
            let (a, b) = (column[j], column[j + 1]);
            let rho = (a * a + b * b).sqrt();
            let (c, s) = if rho == 0.0 { (1.0, 0.0) } else { (a / rho, b / rho) };
            cs.push(c);
            sn.push(s);
            column[j] = rho;
            column[j + 1] = 0.0;
            g.push(-s * g[j]);
            g[j] *= c;
            h.push(column);
            let converged = g[j + 1].abs() <= goal || next == 0.0;
            if !converged {
                basis.push(w.iter().map(|v| v / next).collect());
            }
            if converged || j + 1 == m {
                // back-substitution in the triangle, then x += V y
                let k = j + 1;
                let mut y = vec![0.0; k];
                for i in (0..k).rev() {
                    let mut s = g[i];
                    for (l, yl) in y.iter().enumerate().take(k).skip(i + 1) {
                        s -= h[l][i] * yl;
                    }
                    y[i] = s / h[i][i];
                }
                for (yi, v) in y.iter().zip(&basis) {
                    for (x, v) in x.iter_mut().zip(v) {
                        *x += v * yi;
                    }
                }
                done = converged;
                break;
            }
        }
        if done {
            return Ok(x);
        }
    }
    let ax = apply(&x)?;
    let left = norm(&rhs.iter().zip(&ax).map(|(b, a)| b - a).collect::<Vec<_>>());
    if left <= 1e-9 * size {
        Ok(x)
    } else {
        Err(Error::invalid(
            "kerr",
            format!("GMRES didn't converge (relative residual {:.1e})", left / size),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fdfd::Edges;
    use crate::portable;

    fn lam() -> Wavelength {
        Wavelength::um(2.0).unwrap()
    }

    #[test]
    fn a_real_linear_system_is_solved() {
        // B = diag(2 + i k), D = diag(0.7 e^{ik}): check B x + D x̄ = r
        let n = 30;
        let b: Vec<c64> = (0..n).map(|k| c64::new(2.0, 0.1 * k as f64)).collect();
        let d: Vec<c64> = (0..n)
            .map(|k| portable::complex::from_polar(0.7, k as f64))
            .collect();
        let r: Vec<c64> = (0..n)
            .map(|k| c64::new(portable::sin(k as f64), 1.0))
            .collect();
        let solve = |v: &[c64]| -> Result<Vec<c64>> { Ok(v.iter().zip(&b).map(|(v, b)| v / b).collect()) };
        let x = real_linear(&solve, &d, &r).unwrap();
        for k in 0..n {
            let lhs = b[k] * x[k] + d[k] * x[k].conj();
            assert!(portable::complex::abs(lhs - r[k]) < 1e-12, "{k}");
        }
    }

    /// A uniform Kerr medium, periodic both ways, driven by a uniform source: the field is
    /// uniform and its |U|² = s solves s |k₀²(ε + χ s)|² = |B|².
    #[test]
    fn a_uniform_medium_has_the_scalar_equations_root() {
        let grid = Grid {
            nx: 6,
            ny: 5,
            dx: 0.05,
            dy: 0.05,
            x0: 0.0,
            y0: 0.0,
        };
        let boundaries = Boundaries {
            x: Edges::Bloch { k: 0.0 },
            y: Edges::Bloch { k: 0.0 },
            reflection: 1e-8,
            order: 3.0,
        };
        let eps = c64::new(2.0, 0.3);
        let chi = 0.8;
        let b = c64::new(25.0, 10.0);
        let n = grid.nx * grid.ny;
        let kerr = Kerr2d::new(grid, lam(), &vec![eps; n], &vec![chi; n], boundaries).unwrap();
        let solved = kerr.solve(&vec![b; n], None).unwrap();
        let k2 = kerr.linear.k0 * kerr.linear.k0;
        let left = |s: f64| s * (k2 * (eps + chi * s)).norm_sqr() - b.norm_sqr();
        let (mut lo, mut hi) = (0.0, 10.0);
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if left(mid) > 0.0 {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        let s = 0.5 * (lo + hi);
        let u = solved.field().at(2, 3);
        assert!((u.norm_sqr() - s).abs() < 1e-12 * s, "{} against {s}", u.norm_sqr());
        // and the nonlinearity matters: chi s is not small against eps
        assert!(chi * s > 0.25 * eps.re, "{}", chi * s);
        assert!(solved.steps() > 2 && solved.residual() < 1e-12);
    }
}
