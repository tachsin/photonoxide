//! Adjoint gradients: how an objective changes with every cell's permittivity, from one more
//! back-substitution.
//!
//! G. Veronis, R. W. Dutton, S. Fan, "Method for sensitivity analysis of photonic crystal
//! devices", Opt. Lett. 29, 2288 (2004), doi:10.1364/OL.29.002288, Eqs. 2–4: with A(ε) u = b and
//! b independent of ε, an objective F(u) changes as ∂F/∂ε_k = −2 Re(λᵀ (∂A/∂ε_k) u), where the
//! adjoint field λ solves Aᵀ λ = ∂F/∂u. One factorization serves both solves, so the gradient
//! for every cell costs one more back-substitution (T. W. Hughes et al., ACS Photonics 5, 4781
//! (2018), doi:10.1021/acsphotonics.8b01522, Eqs. 9–13, the same for FDFD).
//!
//! The objective here is the power a port mode carries one way, F = |a|², a = cᵀu the mode's
//! amplitude (linear in u: the projection on two columns, [`super::Field2d::mode_amplitudes`]).
//! Then ∂F/∂u = conj(a) c and ∂F/∂ε_k = −2 Re(λᵀ (∂A/∂ε_k) u) with Aᵀ λ = conj(a) c.
//!
//! **∂A/∂ε** for a problem given cell by cell ([`Solver2d::from_cells`]): with E along z, ε_k
//! sits on the diagonal as k₀² ε_k. With H along z, ε enters through the faces' couplings
//! 1/(ε_f s s Δ²), each face's ε_f the mean of its two cells, so a cell's derivative collects half
//! of each of its faces' (all of a face on a wall). The source b must not depend on ε: the
//! cells varied must be away from the ports' columns (their total-field/scattered-field rows),
//! and the ports' modes are those of the unvaried guides.

use num_complex::Complex64 as c64;

use super::{Direction, Edges, Field2d, Polarization, PortMode, Solver2d, stretch};
use crate::{Error, Result};

impl Solver2d {
    /// u of Aᵀ u = `rhs`, with one step of iterative refinement.
    fn solve_transposed(&self, rhs: &[c64]) -> Vec<c64> {
        let mut values = self.lu.solve_transpose(rhs);
        let mut residual = rhs.to_vec();
        for t in &self.entries {
            residual[t.col] -= t.val * values[t.row];
        }
        for (v, d) in values.iter_mut().zip(self.lu.solve_transpose(&residual)) {
            *v += d;
        }
        values
    }

    /// The power `mode` carries `direction` in `field` (a field of this problem), |a|² with a the
    /// mode's amplitude at its column, and its gradient with respect to each cell's relative
    /// permittivity (its real part), `[j * nx + i]` for cell (i, j). See the module's docs for
    /// what must hold for the gradient to be exact.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for H along z on a problem not given cell by cell
    /// ([`Solver2d::from_cells`]), or a field or mode of another problem.
    pub fn mode_power_gradient(
        &self,
        field: &Field2d,
        mode: &PortMode,
        direction: Direction,
    ) -> Result<(f64, Vec<f64>)> {
        let g = self.grid;
        let (nx, ny) = (g.nx, g.ny);
        if field.values.len() != nx * ny || mode.profile.len() != ny {
            return Err(Error::invalid(
                "fdfd gradient",
                "the field and the mode must belong to this problem",
            ));
        }
        if self.polarization == Polarization::Hz && !self.cellwise {
            return Err(Error::invalid(
                "fdfd gradient",
                "with H along z the gradient is for a problem given cell by cell (from_cells)",
            ));
        }
        // a = cᵀu: the projection on columns c and c + 1 (see mode_amplitudes)
        let step = (c64::new(0.0, 1.0) * mode.beta * mode.dx).exp();
        let d = step - 1.0 / step;
        let (on_c, on_next) = match direction {
            Direction::Forward => (-1.0 / (step * d), 1.0 / d),
            Direction::Backward => (1.0 + 1.0 / (step * d), -1.0 / d),
        };
        let mut c = vec![c64::new(0.0, 0.0); nx * ny];
        for j in 0..ny {
            let wp = mode.weights[j] * mode.profile[j];
            c[j * nx + mode.column] = wp * on_c;
            c[j * nx + mode.column + 1] = wp * on_next;
        }
        let a: c64 = c.iter().zip(&field.values).map(|(c, u)| c * u).sum();
        let adjoint_source: Vec<c64> = c.iter().map(|c| a.conj() * c).collect();
        let lambda = self.solve_transposed(&adjoint_source);
        let u = &field.values;
        let mut gradient = vec![0.0; nx * ny];
        match self.polarization {
            Polarization::Ez => {
                let k2 = self.k0 * self.k0;
                for (k, g) in gradient.iter_mut().enumerate() {
                    *g = -2.0 * (lambda[k] * k2 * u[k]).re;
                }
            }
            Polarization::Hz => self.hz_gradient(&lambda, u, &mut gradient),
        }
        Ok((a.norm_sqr(), gradient))
    }

    /// −2 Re(λᵀ (∂A/∂ε_cell) u) with H along z, face by face, as `assemble` couples them.
    fn hz_gradient(&self, lambda: &[c64], u: &[c64], gradient: &mut [f64]) {
        let g = self.grid;
        let (nx, ny) = (g.nx, g.ny);
        let b = &self.boundaries;
        let span_x = (g.x0, g.x0 + nx as f64 * g.dx);
        let span_y = (g.y0, g.y0 + ny as f64 * g.dy);
        let sx = |x: f64| stretch(x, span_x, b.x.pml(), g.dx, self.k0, b);
        let sy = |y: f64| stretch(y, span_y, b.y.pml(), g.dy, self.k0, b);
        // across the end of an axis: the neighbour's index along it and its phase, if any
        let beyond = |n: i64, len: usize, edges: Edges, step: f64| -> Option<(usize, c64)> {
            if (0..len as i64).contains(&n) {
                return Some((n as usize, c64::new(1.0, 0.0)));
            }
            match edges {
                Edges::Bloch { k } => {
                    let turns = if n < 0 { -1.0 } else { 1.0 };
                    Some((
                        n.rem_euclid(len as i64) as usize,
                        c64::from_polar(1.0, turns * k * len as f64 * step),
                    ))
                }
                Edges::Pml { .. } => None,
            }
        };
        // the cells a face's ε is the mean of, with their shares: face `n` before cell n along
        // an axis of `len` cells (see face_mean)
        let owners = |n: usize, len: usize, bloch: bool| -> [(usize, f64); 2] {
            if n == 0 || n == len {
                if bloch {
                    [(len - 1, 0.5), (0, 0.5)]
                } else {
                    let only = n.min(len - 1);
                    [(only, 0.5), (only, 0.5)]
                }
            } else {
                [(n - 1, 0.5), (n, 0.5)]
            }
        };
        let bloch = |e: Edges| matches!(e, Edges::Bloch { .. });
        for j in 0..ny {
            for i in 0..nx {
                let k = j * nx + i;
                let (x, y) = (g.x(i), g.y(j));
                // (face index along its axis, axis, the face's ε, its stretch, the step,
                // the neighbour's offset)
                let faces = [
                    (
                        i,
                        0,
                        self.eps_y[j * (nx + 1) + i],
                        sx(x) * sx(x - g.dx / 2.0),
                        g.dx,
                        -1i64,
                    ),
                    (
                        i + 1,
                        0,
                        self.eps_y[j * (nx + 1) + i + 1],
                        sx(x) * sx(x + g.dx / 2.0),
                        g.dx,
                        1,
                    ),
                    (
                        j,
                        1,
                        self.eps_x[j * nx + i],
                        sy(y) * sy(y - g.dy / 2.0),
                        g.dy,
                        -1,
                    ),
                    (
                        j + 1,
                        1,
                        self.eps_x[(j + 1) * nx + i],
                        sy(y) * sy(y + g.dy / 2.0),
                        g.dy,
                        1,
                    ),
                ];
                for (face, axis, eps, s, h, offset) in faces {
                    let coupling = 1.0 / (eps * s * h * h);
                    let neighbour = if axis == 0 {
                        beyond(i as i64 + offset, nx, b.x, g.dx).map(|(n, p)| u[j * nx + n] * p)
                    } else {
                        beyond(j as i64 + offset, ny, b.y, g.dy).map(|(n, p)| u[n * nx + i] * p)
                    };
                    let difference = neighbour.unwrap_or(c64::new(0.0, 0.0)) - u[k];
                    // ∂(coupling)/∂ε_face = −coupling/ε_face, on both its entries in row k
                    let term = -2.0 * (lambda[k] * (-coupling / eps) * difference).re;
                    let shares = if axis == 0 {
                        owners(face, nx, bloch(b.x)).map(|(n, w)| (j * nx + n, w))
                    } else {
                        owners(face, ny, bloch(b.y)).map(|(n, w)| (n * nx + i, w))
                    };
                    for (cell, w) in shares {
                        gradient[cell] += w * term;
                    }
                }
            }
        }
    }
}
