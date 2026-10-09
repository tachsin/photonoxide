//! The modes of any planar profile by finite differences: graded, lossy and leaky stacks.
//!
//! A profile is a permittivity per cell between nodes x₀ < x₁ < … < x_N (µm, spacing free),
//! and the field ψ — E_y for TE (E along the layers), H_y for TM — lives on the nodes, zero one
//! spacing beyond the ends unless a perfectly matched layer absorbs it there. At node i, with
//! the cell to its left (ε_w, width w) and to its right (ε_e, width e):
//!
//! - TE, ψ'' + (k²ε − β²)ψ = 0:
//!   2/(w + e) [(ψ_{i+1} − ψ_i)/e − (ψ_i − ψ_{i−1})/w] + k² ε̄ ψ_i = β² ψ_i,
//!   ε̄ = (w ε_w + e ε_e)/(w + e);
//! - TM, ε (ψ'/ε)' + (k²ε − β²)ψ = 0, which keeps ψ and ψ'/ε continuous at interfaces:
//!   2/(w + e) [(ψ_{i+1} − ψ_i)/(e ε_e) − (ψ_i − ψ_{i−1})/(w ε_w)] + k² ψ_i = β² ⟨1/ε⟩ ψ_i,
//!   ⟨1/ε⟩ = (w/ε_w + e/ε_e)/(w + e), divided through by ⟨1/ε⟩.
//!
//! These are the full-vector scheme's equations (A. B. Fallahkhair, K. S. Li, T. E. Murphy,
//! J. Lightwave Technol. 26, 1423 (2008), [doi:10.1109/JLT.2008.923643](https://doi.org/10.1109/JLT.2008.923643))
//! in the limit of a structure uniform in one direction, and like them second order with
//! interfaces on nodes. A PML stretches the spacings into the complex plane exactly as in
//! [`crate::mode::vector::Pml`] (W. C. Chew, J. M. Jin, E. Michielssen, Microw. Opt. Technol.
//! Lett. 15, 363 (1997)). The modes nearest an effective index come from shift-and-invert
//! Arnoldi.

use num_complex::Complex64 as c64;

use super::Polarization;
use crate::units::Wavelength;
use crate::{Error, Result};

/// A planar profile on a grid of nodes, with optional perfectly matched layers at its ends.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    nodes: Vec<f64>,
    cells: Vec<c64>,
    pml: (f64, f64),
    strength: f64,
}

/// A mode of a [`Profile`]: its effective index and its field on the nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileMode {
    /// n_eff = β/k: complex for a leaky or lossy mode (Im > 0, decaying).
    pub effective_index: c64,
    /// ψ at the nodes, the largest 1.
    pub field: Vec<c64>,
}

impl Profile {
    /// Nodes (µm, at least 3, strictly increasing) and the relative permittivity of each cell
    /// between them.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for fewer than 3 nodes, nodes that aren't increasing, the wrong
    /// number of cells, or a permittivity that isn't finite.
    pub fn new(nodes: Vec<f64>, cells: Vec<c64>) -> Result<Profile> {
        if nodes.len() < 3
            || nodes.iter().any(|x| !x.is_finite())
            || nodes.windows(2).any(|w| w[1] <= w[0])
        {
            return Err(Error::invalid(
                "profile",
                "needs at least 3 finite, strictly increasing nodes",
            ));
        }
        if cells.len() != nodes.len() - 1
            || cells
                .iter()
                .any(|e| !(e.re.is_finite() && e.im.is_finite()))
        {
            return Err(Error::invalid(
                "profile",
                format!(
                    "needs {} finite cells, got {}",
                    nodes.len() - 1,
                    cells.len()
                ),
            ));
        }
        Ok(Profile {
            nodes,
            cells,
            pml: (0.0, 0.0),
            strength: 0.0,
        })
    }

    /// `n` uniform cells from `x0` to `x1`, each with the permittivity `eps` gives at its centre.
    ///
    /// # Errors
    ///
    /// As [`Profile::new`].
    pub fn uniform(x0: f64, x1: f64, n: usize, eps: impl Fn(f64) -> c64) -> Result<Profile> {
        let nodes: Vec<f64> = (0..=n)
            .map(|i| x0 + (x1 - x0) * i as f64 / n as f64)
            .collect();
        let cells = nodes.windows(2).map(|w| eps(0.5 * (w[0] + w[1]))).collect();
        Profile::new(nodes, cells)
    }

    /// The same profile with perfectly matched layers in its last `low` µm at the small-x end
    /// and `high` µm at the other, of strength α (see [`crate::mode::vector::Pml`]).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for thicknesses or a strength that are negative or not finite,
    /// or layers thicker together than the profile.
    pub fn with_pml(mut self, low: f64, high: f64, strength: f64) -> Result<Profile> {
        let ok = |v: f64| v.is_finite() && v >= 0.0;
        if !(ok(low) && ok(high) && ok(strength)) {
            return Err(Error::invalid(
                "PML",
                "thicknesses and strength must be finite and not negative",
            ));
        }
        if low + high >= self.nodes[self.nodes.len() - 1] - self.nodes[0] {
            return Err(Error::invalid(
                "PML",
                "the layers are thicker than the profile",
            ));
        }
        self.pml = (low, high);
        self.strength = strength;
        Ok(self)
    }

    /// The nodes, µm.
    pub fn nodes(&self) -> &[f64] {
        &self.nodes
    }

    /// The `count` modes with effective indices nearest `near` (the highest index in the
    /// profile when `None`), nearest first.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a count of 0, a `near` that isn't finite, or if the
    /// eigenproblem doesn't converge.
    pub fn modes(
        &self,
        polarization: Polarization,
        wavelength: Wavelength,
        count: usize,
        near: Option<f64>,
    ) -> Result<Vec<ProfileMode>> {
        if count == 0 {
            return Err(Error::invalid("mode count", "must be at least 1"));
        }
        crate::mode::vector::check_near(near)?;
        let k = wavelength.wavenumber();
        let k2 = k * k;
        let n = self.nodes.len();
        let entries = self.entries(polarization, k2);
        let n_max =
            near.unwrap_or_else(|| self.cells.iter().map(|e| e.re.sqrt()).fold(1.0, f64::max));
        let shift = c64::new(k2 * n_max * n_max, 0.0);
        let pairs = crate::eigen::nearest(n, &entries, shift, count, 1e-10)?;
        Ok(pairs
            .into_iter()
            .map(|p| ProfileMode::new(k, p.value, &p.vector))
            .collect())
    }

    /// Every mode whose effective index lies in `region`, highest Re n_eff first, with no count
    /// and no guess: by contour integrals ([`crate::mode::region`]), the same bits on any number
    /// of threads.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if a shifted matrix can't be factorized, a subspace given in
    /// `search` is too small for the modes in the region, or they don't converge within its
    /// iterations.
    pub fn modes_in(
        &self,
        polarization: Polarization,
        wavelength: Wavelength,
        region: &crate::mode::region::Region,
        search: &crate::mode::region::Search,
    ) -> Result<crate::mode::region::Found<ProfileMode>> {
        let k = wavelength.wavenumber();
        let entries = self.entries(polarization, k * k);
        let positions: Vec<[f64; 3]> = self.nodes.iter().map(|&x| [x, 0.0, 0.0]).collect();
        let found =
            crate::mode::region::search(self.nodes.len(), &entries, &positions, k, region, search)?;
        Ok(crate::mode::region::Found {
            modes: found
                .pairs
                .into_iter()
                .map(|p| ProfileMode::new(k, p.value, &p.vector))
                .collect(),
            residuals: found.residuals,
            estimate: found.estimate,
            subspace: found.subspace,
            iterations: found.iterations,
        })
    }

    /// The matrix whose eigenvalues are β², as (row, column, value) entries.
    pub(crate) fn entries(&self, polarization: Polarization, k2: f64) -> Vec<(usize, usize, c64)> {
        let (start, end) = (self.nodes[0], self.nodes[self.nodes.len() - 1]);
        // complex coordinates inside the PML (Chew et al., Eq. 45)
        let x: Vec<c64> = self
            .nodes
            .iter()
            .map(|&p| {
                let mut im = 0.0;
                if self.pml.0 > 0.0 && p < start + self.pml.0 {
                    im -= self.strength * (start + self.pml.0 - p).powi(3)
                        / (3.0 * self.pml.0 * self.pml.0);
                }
                if self.pml.1 > 0.0 && p > end - self.pml.1 {
                    im += self.strength * (p - (end - self.pml.1)).powi(3)
                        / (3.0 * self.pml.1 * self.pml.1);
                }
                c64::new(p, im)
            })
            .collect();
        let n = x.len();
        let last = self.cells.len() - 1;
        let mut entries = Vec::with_capacity(3 * n);
        for i in 0..n {
            // beyond an end the spacing and the cell continue, the field being zero
            let w = if i > 0 { x[i] - x[i - 1] } else { x[1] - x[0] };
            let e = if i < n - 1 {
                x[i + 1] - x[i]
            } else {
                x[n - 1] - x[n - 2]
            };
            let (ew, ee) = (
                self.cells[i.saturating_sub(1).min(last)],
                self.cells[i.min(last)],
            );
            let s = 2.0 / (w + e);
            match polarization {
                Polarization::Te => {
                    let (cw, ce) = (s / w, s / e);
                    let diag = -(cw + ce) + k2 * (w * ew + e * ee) / (w + e);
                    entries.push((i, i, diag));
                    if i > 0 {
                        entries.push((i, i - 1, cw));
                    }
                    if i < n - 1 {
                        entries.push((i, i + 1, ce));
                    }
                }
                Polarization::Tm => {
                    let inv = (w / ew + e / ee) / (w + e);
                    let (cw, ce) = (s / (w * ew) / inv, s / (e * ee) / inv);
                    let diag = -(cw + ce) + k2 / inv;
                    entries.push((i, i, diag));
                    if i > 0 {
                        entries.push((i, i - 1, cw));
                    }
                    if i < n - 1 {
                        entries.push((i, i + 1, ce));
                    }
                }
            }
        }
        entries
    }
}

impl ProfileMode {
    /// The mode of eigenvalue β² and field `vector` at wavenumber k, the field's peak 1.
    fn new(k: f64, beta2: c64, vector: &[c64]) -> ProfileMode {
        let peak = vector
            .iter()
            .copied()
            .max_by(|a, b| a.norm().total_cmp(&b.norm()))
            .unwrap_or(c64::new(1.0, 0.0));
        ProfileMode {
            effective_index: beta2.sqrt() / k,
            field: vector.iter().map(|v| v / peak).collect(),
        }
    }
}

/// Chilwell and Hodgkinson's four-layer guide (their Fig. 2) as a profile at spacing `h`, from
/// the substrate (x < −2 µm, 1.50; 6 µm of it, for TE₃'s slow tail) through the films (1.66, 1.60, 1.53, 1.66 upwards)
/// to the cover (x > 0, 1.0; 1 µm of it).
pub(crate) fn chilwell_profile(h: f64) -> Profile {
    let index = |x: f64| match x {
        x if x > 0.0 => 1.0,
        x if x > -0.5 => 1.66,
        x if x > -1.0 => 1.53,
        x if x > -1.5 => 1.60,
        x if x > -2.0 => 1.66,
        _ => 1.5,
    };
    Profile::uniform(-8.0, 1.0, (9.0 / h).round() as usize, |x| {
        c64::new(index(x) * index(x), 0.0)
    })
    .expect("a valid profile")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::multilayer::chilwell_four_layer;
    use crate::mode::slab::Slab;
    use crate::units::Length;

    fn lam() -> Wavelength {
        Wavelength::um(1.55).unwrap()
    }

    /// The book's slab, 220 nm of 3.473 in 1.444, in a 4.02 µm window at spacing `h` (nodes on
    /// the interfaces).
    fn book(h: f64) -> Profile {
        Profile::uniform(-2.01, 2.01, (4.02 / h).round() as usize, |x| {
            let n: f64 = if x.abs() < 0.11 { 3.473 } else { 1.444 };
            c64::new(n * n, 0.0)
        })
        .unwrap()
    }

    #[test]
    fn the_slab_converges_at_second_order_to_the_exact_one() {
        let slab = Slab::new(1.444, 3.473, 1.444, Length::nm(220.0)).unwrap();
        for pol in [Polarization::Te, Polarization::Tm] {
            let exact = slab.modes(pol, lam())[0].effective_index();
            let errors: Vec<f64> = [0.01, 0.005, 0.0025]
                .iter()
                .map(|&h| {
                    book(h).modes(pol, lam(), 1, None).unwrap()[0]
                        .effective_index
                        .re
                        - exact
                })
                .collect();
            for w in errors.windows(2) {
                let order = (w[0] / w[1]).abs().log2();
                assert!((order - 2.0).abs() < 0.1, "{pol:?}: {errors:?}");
            }
        }
    }

    #[test]
    fn the_four_layer_guide_has_chilwell_and_hodgkinsons_modes() {
        // all 8 bound modes within 2e-6 at 1 nm (Table 3 is printed to 6 decimals)
        let (stack, w) = chilwell_four_layer();
        let profile = chilwell_profile(0.001);
        for pol in [Polarization::Te, Polarization::Tm] {
            let exact = stack.bound_modes(pol, w).unwrap();
            let found = profile.modes(pol, w, 4, Some(1.63)).unwrap();
            let mut got: Vec<f64> = found.iter().map(|m| m.effective_index.re).collect();
            got.sort_by(|a, b| b.total_cmp(a));
            for (g, e) in got.iter().zip(&exact) {
                assert!(
                    (g - e.effective_index().re).abs() < 2e-6,
                    "{pol:?}: {g} vs {}",
                    e.effective_index().re
                );
            }
        }
    }

    #[test]
    fn a_pml_gives_the_leaky_waves() {
        // Chilwell and Hodgkinson's Table 2, m = 4 … 7, from the profile with a PML below
        let profile = chilwell_profile(0.001).with_pml(2.0, 0.0, 5.0).unwrap();
        let w = Wavelength::nm(632.8).unwrap();
        for (re, im) in [
            (1.46186, 0.00716),
            (1.38250, 0.01817),
            (1.28136, 0.03588),
            (1.14231, 0.05288),
        ] {
            let target = c64::new(re, im);
            let best = profile
                .modes(Polarization::Te, w, 3, Some(re))
                .unwrap()
                .into_iter()
                .map(|m| m.effective_index)
                .min_by(|a, b| (a - target).norm().total_cmp(&(b - target).norm()))
                .unwrap();
            assert!((best - target).norm() < 2e-5, "{best} vs {target}");
        }
    }

    #[test]
    fn bad_profiles_are_errors() {
        assert!(Profile::new(vec![0.0, 1.0], vec![c64::new(1.0, 0.0)]).is_err());
        assert!(Profile::new(vec![0.0, 1.0, 1.0], vec![c64::new(1.0, 0.0); 2]).is_err());
        assert!(Profile::new(vec![0.0, 1.0, 2.0], vec![c64::new(1.0, 0.0)]).is_err());
        assert!(book(0.01).with_pml(3.0, 2.0, 3.0).is_err());
        let w = Wavelength::um(1.55).unwrap();
        let e = book(0.01)
            .modes(Polarization::Te, w, 1, Some(f64::NAN))
            .unwrap_err()
            .to_string();
        assert!(e.contains("must be finite"), "{e}");
    }
}
