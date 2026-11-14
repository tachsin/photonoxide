//! A full-vector mode's effective index against every cell's permittivity: the eigenproblem's
//! own adjoint.
//!
//! The matrix A of Eq. (8), whose eigenvalues are β², isn't symmetric (Fallahkhair et al.'s
//! scheme couples H_x and H_y through unequal stencils, and a PML makes it complex), so a simple
//! eigenvalue moves as dλ = yᵀ dA x / yᵀx with x the mode (the right eigenvector) and y the left
//! one, Aᵀy = λy: the first-order perturbation of a non-normal matrix's eigenvalue, which for a
//! Hermitian matrix is Hellmann–Feynman's xᴴ dA x / xᴴx. y comes from inverse iteration with
//! Aᵀ shifted just off λ, started from x. dA for a cell is the derivative of the entries of the
//! four nodes whose regions include it, from the same expressions that assemble A
//! ([`super::node_entries`], Eqs. (21)–(36)) evaluated on dual numbers, so it is exact for the
//! discrete problem; the cost is one solve with Aᵀ's factors per iteration, then a pass over
//! the nodes. n_eff = β/k gives dn_eff = dλ/(2k² n_eff).
//!
//! The change of a cell is isotropic, δε·I (ε_xx, ε_yy and ε_zz together): what a density or a
//! material changes. [`crate::mode::thermo::index_change`] is the same derivative from the
//! continuous perturbation formula (Johnson et al. 2002) on the mode's fields: the two agree to
//! the field's discretization error, this one exactly with the solver.

use faer::c64;

use super::{CrossSection, Numbering, VectorMode, assemble, node_entries, node_spacings};
use crate::eigen::Shifted;
use crate::mode::dual::Dual;
use crate::portable;
use crate::units::Wavelength;
use crate::{Error, Result};

impl VectorMode {
    /// dn_eff/dε for every cell of `cs` (the cross-section this mode was solved on, at
    /// `wavelength`), in the order of [`CrossSection::cells`]: the change of the effective index
    /// as each cell's relative permittivity grows isotropically, δε·I. Exact for the discrete
    /// eigenproblem (the module's docs); a degenerate eigenvalue has no derivative.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if `cs` isn't on the mode's grid or `wavelength` isn't the
    /// mode's, or if the left eigenvector doesn't converge.
    pub fn index_gradient(&self, cs: &CrossSection, wavelength: Wavelength) -> Result<Vec<c64>> {
        if cs.x != self.x || cs.y != self.y {
            return Err(Error::invalid(
                "mode gradient",
                "the cross-section must be the one the mode was solved on",
            ));
        }
        let k = wavelength.wavenumber();
        if (k - self.k).abs() > 1e-12 * k {
            return Err(Error::invalid(
                "mode gradient",
                "the wavelength must be the one the mode was solved at",
            ));
        }
        let k2 = k * k;
        let numbering = Numbering::new(cs);
        let x = self.to_unknowns(&numbering.unknowns);
        let entries = assemble(cs, k2);
        let y = left_vector(x.len(), &entries, self.beta2, &x)?;
        let norm: c64 = y.iter().zip(&x).map(|(a, b)| a * b).sum();
        let neff = self.effective_index();
        if norm.norm_sqr() == 0.0 {
            return Err(Error::invalid(
                "mode gradient",
                "the left and right eigenvectors are orthogonal: the eigenvalue is defective",
            ));
        }
        let (xs, ys) = cs.stretched_nodes();
        let mut gradient = vec![c64::new(0.0, 0.0); cs.cells.len()];
        for i in 0..cs.x.len() {
            for j in 0..cs.y.len() {
                let cells = cs.node_regions(i, j);
                let spacings = node_spacings(&xs, &ys, i, j);
                for (r, &c) in cells.iter().enumerate() {
                    // each cell once per node, however many of its regions it fills
                    if cells[..r].contains(&c) {
                        continue;
                    }
                    let regions = cells.map(|cell| {
                        let e = cs.cells[cell];
                        let iso = |v: c64| {
                            if cell == c {
                                Dual::variable(v)
                            } else {
                                Dual::constant(v)
                            }
                        };
                        [
                            iso(e.xx),
                            Dual::constant(e.xy),
                            Dual::constant(e.yx),
                            iso(e.yy),
                            iso(e.zz),
                        ]
                    });
                    let mut change = c64::new(0.0, 0.0);
                    node_entries(
                        &numbering,
                        (i, j),
                        regions,
                        spacings,
                        k2,
                        |row, q, v: Dual| {
                            change += y[row] * v.d * x[q];
                        },
                    );
                    gradient[c] += change;
                }
            }
        }
        let scale = 1.0 / (norm * 2.0 * k2 * neff);
        Ok(gradient.into_iter().map(|g| g * scale).collect())
    }
}

/// The left eigenvector of the matrix of `entries` (n × n) for its eigenvalue `lambda`, whose
/// right one is `x`: inverse iteration with Aᵀ shifted by 10⁻⁷|λ| off λ, from x. Each step
/// shrinks the other eigenvectors' share by about 10⁻⁷|λ| over their distance from λ.
fn left_vector(
    n: usize,
    entries: &[(usize, usize, c64)],
    lambda: c64,
    x: &[c64],
) -> Result<Vec<c64>> {
    let transposed: Vec<(usize, usize, c64)> = entries.iter().map(|&(i, j, v)| (j, i, v)).collect();
    let size = portable::complex::abs(lambda).max(1.0);
    let shifted = Shifted::new(n, &transposed, lambda + c64::new(1e-7 * size, 0.0))?;
    let mut y = x.to_vec();
    for _ in 0..4 {
        y = shifted.solve(&y);
        let peak = y
            .iter()
            .map(|v| portable::complex::abs(*v))
            .fold(0.0, f64::max);
        if !(peak.is_finite() && peak > 0.0) {
            return Err(Error::invalid(
                "mode gradient",
                "the left eigenvector's iteration failed",
            ));
        }
        for v in &mut y {
            *v /= peak;
        }
    }
    let length = y.iter().map(|v| v.norm_sqr()).sum::<f64>().sqrt();
    let residual = shifted.residual(lambda, &y) / (size * length);
    if residual > 1e-7 {
        return Err(Error::invalid(
            "mode gradient",
            format!("the left eigenvector didn't converge (residual {residual:.1e})"),
        ));
    }
    Ok(y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::vector::{Boundaries, Boundary, Permittivity, Pml, modes};

    fn lam() -> Wavelength {
        Wavelength::um(1.55).unwrap()
    }

    /// A small, asymmetric, lossy ridge with a PML on one side and a wall on another: nothing
    /// in it is symmetric.
    fn ridge() -> CrossSection {
        let x: Vec<f64> = (0..=28).map(|i| -1.0 + 2.0 * f64::from(i) / 28.0).collect();
        let y: Vec<f64> = (0..=22).map(|j| -0.8 + 1.6 * f64::from(j) / 22.0).collect();
        let mut cells = Vec::new();
        for i in 0..28 {
            for j in 0..22 {
                let (xc, yc) = (0.5 * (x[i] + x[i + 1]), 0.5 * (y[j] + y[j + 1]));
                let eps = if (-0.25..0.3).contains(&xc) && (-0.11..0.11).contains(&yc) {
                    c64::new(12.0, 0.02)
                } else if yc < -0.11 {
                    c64::new(2.085, 0.0)
                } else {
                    c64::new(1.0, 0.0)
                };
                cells.push(Permittivity::isotropic(eps));
            }
        }
        CrossSection::new(x, y, cells)
            .unwrap()
            .with_boundaries(Boundaries {
                west: Boundary::ElectricWall,
                east: Boundary::Zero,
                south: Boundary::Zero,
                north: Boundary::Zero,
            })
            .unwrap()
            .with_pml(Pml {
                west: 0.0,
                east: 0.3,
                south: 0.0,
                north: 0.25,
                strength: 2.0,
            })
            .unwrap()
    }

    #[test]
    fn the_gradient_is_the_differences_of_the_index() {
        let cs = ridge();
        let mode = &modes(&cs, lam(), 1, Some(2.6)).unwrap()[0];
        let g = mode.index_gradient(&cs, lam()).unwrap();
        let top = g
            .iter()
            .map(|v| portable::complex::abs(*v))
            .fold(0.0, f64::max);
        let delta = 1e-3;
        // in the core, at its edge, in the cladding and in the PML
        for c in [
            14 * 22 + 11,
            10 * 22 + 11,
            14 * 22 + 8,
            14 * 22 + 20,
            26 * 22 + 11,
        ] {
            let f = |d: f64| {
                let mut delta_cells = vec![Permittivity::isotropic(c64::new(0.0, 0.0)); g.len()];
                delta_cells[c] = Permittivity::isotropic(c64::new(d, 0.0));
                let q = cs.perturbed(&delta_cells).unwrap();
                modes(&q, lam(), 1, Some(2.6)).unwrap()[0].effective_index()
            };
            let fd = (8.0 * (f(delta) - f(-delta)) - (f(2.0 * delta) - f(-2.0 * delta)))
                / (12.0 * delta);
            let err = portable::complex::abs(fd - g[c]);
            assert!(err < 1e-6 * top, "cell {c}: {} against {fd}", g[c]);
        }
    }
}
