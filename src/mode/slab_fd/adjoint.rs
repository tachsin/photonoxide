//! A profile mode's effective index against every cell's permittivity and every interior node's
//! position, from the mode alone: the eigenproblem's own adjoint.
//!
//! The matrix M whose eigenvalues are β² (the module's equations) is W⁻¹H, with H symmetric
//! (unconjugated) and W diagonal: W_i = (w + e)/2 for TE and (w/ε_w + e/ε_e)/2 for TM, the
//! weights the equations are divided through by. So y = W x is a left eigenvector, Mᵀy = λy,
//! and a simple eigenvalue moves as dλ = yᵀ dM x / yᵀx. Without a PML or loss, H is real
//! symmetric and W positive, and this is the Hellmann–Feynman form
//! xᵀ(dH − λ dW)x / xᵀWx of a symmetric-definite pencil; with them, it is the same formula with
//! the unconjugated product, which is what makes y a left eigenvector. dM's entries are the rows'
//! own expressions evaluated on dual numbers ([`crate::mode::dual`]), so they are exact, and
//! n_eff = β/k gives dn_eff = dλ/(2k² n_eff). The cost is a pass over the rows a parameter
//! touches: two rows for a cell, three for a node.

use num_complex::Complex64 as c64;

use super::{Profile, ProfileMode, row, spacings};
use crate::mode::Polarization;
use crate::mode::dual::Dual;
use crate::units::Wavelength;
use crate::{Error, Result};

/// How a [`ProfileMode`]'s effective index changes with the profile ([`Profile::index_gradient`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ProfileGradient {
    /// dn_eff/dε for each cell, a real change of its relative permittivity.
    pub cells: Vec<c64>,
    /// dn_eff/dx for each interior node, node 1 first (µm⁻¹): moving node i moves the boundary
    /// between cells i − 1 and i, and inside a PML its complex coordinate with it (the layers'
    /// extents held). The two end nodes, which bound the profile and its PMLs, aren't
    /// parameters.
    pub nodes: Vec<c64>,
}

impl Profile {
    /// The derivatives of `mode`'s effective index (a mode of this profile at `wavelength` and
    /// `polarization`, from [`Profile::modes`] or its kin) with respect to every cell's
    /// permittivity and every interior node's position, exact for the discrete eigenproblem
    /// (the module's docs). A degenerate eigenvalue has no derivative: the result is then
    /// meaningless.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a mode whose field isn't one value per node, or one whose
    /// unconjugated norm yᵀx vanishes (not an eigenvector of this profile's matrix).
    pub fn index_gradient(
        &self,
        polarization: Polarization,
        wavelength: Wavelength,
        mode: &ProfileMode,
    ) -> Result<ProfileGradient> {
        let n = self.nodes.len();
        let x = &mode.field;
        if x.len() != n {
            return Err(Error::invalid(
                "profile gradient",
                format!("the mode has {} values for {n} nodes", x.len()),
            ));
        }
        let k = wavelength.wavenumber();
        let k2 = k * k;
        let coords = self.coordinates();
        let weight = |i: usize| {
            let (w, e) = spacings(&coords, i);
            let (ew, ee) = self.neighbours(i);
            match polarization {
                Polarization::Te => (w + e) * 0.5,
                Polarization::Tm => (w / ew + e / ee) * 0.5,
            }
        };
        let y: Vec<c64> = (0..n).map(|i| weight(i) * x[i]).collect();
        let norm: c64 = y.iter().zip(x).map(|(a, b)| a * b).sum();
        let neff = mode.effective_index;
        if norm.norm_sqr() == 0.0 || neff.norm_sqr() == 0.0 {
            return Err(Error::invalid(
                "profile gradient",
                "the mode's unconjugated norm vanishes: not an eigenvector of this profile",
            ));
        }
        let scale = 1.0 / (norm * 2.0 * k2 * neff);
        // yᵢ (dM x)ᵢ for row i, its entries' derivatives carried by the duals
        let row_change = |i: usize, w: Dual, e: Dual, ew: Dual, ee: Dual| -> c64 {
            let (diag, cw, ce) = row(polarization, k2, w, e, ew, ee);
            let mut s = diag.d * x[i];
            if i > 0 {
                s += cw.d * x[i - 1];
            }
            if i < n - 1 {
                s += ce.d * x[i + 1];
            }
            y[i] * s
        };
        let cells = (0..self.cells.len())
            .map(|c| {
                // the rows whose cells include c: c (east of node c) and c + 1 (west of it),
                // the ends' continued cells among them
                let mut change = c64::new(0.0, 0.0);
                for i in [c, c + 1] {
                    let (iw, ie) = self.neighbour_cells(i);
                    let (w, e) = spacings(&coords, i);
                    let (ew, ee) = self.neighbours(i);
                    let seed = |cell: usize, v: c64| {
                        if cell == c {
                            Dual::variable(v)
                        } else {
                            Dual::constant(v)
                        }
                    };
                    change += row_change(i, w.into(), e.into(), seed(iw, ew), seed(ie, ee));
                }
                change * scale
            })
            .collect();
        let nodes = (1..n - 1)
            .map(|m| {
                let moved = c64::new(1.0, self.stretch(self.nodes[m]).1);
                let at = |j: usize| {
                    Dual::new(
                        coords[j],
                        if j == m { moved } else { c64::new(0.0, 0.0) },
                    )
                };
                let mut change = c64::new(0.0, 0.0);
                for i in m - 1..=m + 1 {
                    // the spacings as `spacings` takes them, on duals
                    let w = if i > 0 { at(i) - at(i - 1) } else { at(1) - at(0) };
                    let e = if i < n - 1 {
                        at(i + 1) - at(i)
                    } else {
                        at(n - 1) - at(n - 2)
                    };
                    let (ew, ee) = self.neighbours(i);
                    change += row_change(i, w, e, ew.into(), ee.into());
                }
                change * scale
            })
            .collect();
        Ok(ProfileGradient { cells, nodes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portable;

    fn lam() -> Wavelength {
        Wavelength::um(1.55).unwrap()
    }

    /// A leaky, lossy, graded profile: an asymmetric core on nodes spaced unevenly, with PMLs.
    fn graded() -> Profile {
        let nodes: Vec<f64> = (0..=160)
            .map(|i| {
                let t = i as f64 / 160.0;
                -2.0 + 4.0 * t + 0.15 * portable::sin(std::f64::consts::PI * t)
            })
            .collect();
        let cells = nodes
            .windows(2)
            .map(|w| {
                let x = 0.5 * (w[0] + w[1]);
                if (-0.2..0.15).contains(&x) {
                    c64::new(11.0 + 2.0 * x, 0.01)
                } else if x > 0.15 {
                    c64::new(2.1, 0.0)
                } else {
                    c64::new(4.0, 0.0)
                }
            })
            .collect();
        Profile::new(nodes, cells)
            .unwrap()
            .with_pml(0.4, 0.4, 2.0)
            .unwrap()
    }

    /// The largest of a set of derivatives: what a difference's error is measured against.
    fn top(g: &[c64]) -> f64 {
        g.iter()
            .map(|v| portable::complex::abs(*v))
            .fold(0.0, f64::max)
    }

    fn index(p: &Profile, pol: Polarization) -> c64 {
        p.modes(pol, lam(), 1, Some(3.2)).unwrap()[0].effective_index
    }

    #[test]
    fn the_gradient_is_the_differences_of_the_index() {
        let p = graded();
        for pol in [Polarization::Te, Polarization::Tm] {
            let mode = &p.modes(pol, lam(), 1, Some(3.2)).unwrap()[0];
            let g = p.index_gradient(pol, lam(), mode).unwrap();
            let delta = 1e-3;
            for c in [0, 45, 80, 100, 159] {
                let f = |d: f64| {
                    let mut q = p.clone();
                    q.cells[c] += d;
                    index(&q, pol)
                };
                let fd = (8.0 * (f(delta) - f(-delta)) - (f(2.0 * delta) - f(-2.0 * delta)))
                    / (12.0 * delta);
                let err = portable::complex::abs(fd - g.cells[c]);
                assert!(
                    err < 1e-6 * top(&g.cells),
                    "{pol:?} cell {c}: {} against {fd}",
                    g.cells[c]
                );
            }
            let delta = 1e-4;
            for m in [1, 40, 75, 120, 159] {
                let f = |d: f64| {
                    let mut q = p.clone();
                    q.nodes[m] += d;
                    index(&q, pol)
                };
                let fd = (8.0 * (f(delta) - f(-delta)) - (f(2.0 * delta) - f(-2.0 * delta)))
                    / (12.0 * delta);
                let err = portable::complex::abs(fd - g.nodes[m - 1]);
                assert!(
                    err < 1e-6 * top(&g.nodes),
                    "{pol:?} node {m}: {} against {fd}",
                    g.nodes[m - 1]
                );
            }
        }
    }

    #[test]
    fn a_mode_of_another_profile_is_refused() {
        let p = graded();
        let mode = ProfileMode {
            effective_index: c64::new(3.0, 0.0),
            field: vec![c64::new(1.0, 0.0); 3],
        };
        assert!(p.index_gradient(Polarization::Te, lam(), &mode).is_err());
    }
}
