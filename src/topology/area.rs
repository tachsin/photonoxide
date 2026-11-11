//! Minimum area and minimum enclosed area: a foundry's rules on islands too small to print and
//! holes too small to open, as constraints an optimizer keeps.
//!
//! A. M. Hammond, A. Oskooi, S. G. Johnson, S. E. Ralph, "Photonic topology optimization with
//! semiconductor-foundry design-rule constraints", Opt. Express 29, 23916 (2021),
//! doi:10.1364/OE.431188, Section 4, Eqs. 21 and 22: g_A = ∫ ρ̄ I_A and g_EA = ∫ (1 − ρ̄) I_EA,
//! where the indicators mark the islands whose area is below the minimum (their solid density
//! summed) and the holes whose enclosed area is (their void density summed), each region's
//! pixels dilated by one; the indicators are held fixed in the gradient, which then erodes a
//! small island (∂g_A/∂ρ̄ = I_A) and fills a small hole (∂g_EA/∂ρ̄ = −I_EA).
//!
//! Hammond et al. find the regions by marching squares at a threshold of 0.6; here they are the
//! 4-connected components of the pixels above 0.6 (solid) and below 0.4 (void, 1 − ρ̄ above
//! 0.6), the same regions on a pixel grid. A region touching the design region's edge isn't
//! counted: it continues outside (a waveguide, the cladding). Both values are integrals over the
//! region, in µm², divided by its area.

use crate::{Error, Result};

/// The two area constraints on a 2D region.
#[derive(Clone, Debug, PartialEq)]
pub struct AreaConstraints {
    shape: [usize; 2],
    pixel_um: f64,
    min_area_um2: f64,
    min_hole_um2: f64,
}

/// The constraints' values and their gradients in the physical densities.
#[derive(Clone, Debug, PartialEq)]
pub struct Areas {
    /// g_A (the islands') and g_EA (the holes'), as shares of the region's area.
    pub values: [f64; 2],
    /// ∂g_A/∂ρ̄ and ∂g_EA/∂ρ̄, one value per pixel each.
    pub gradients: [Vec<f64>; 2],
    /// How many islands and holes are below their minimum.
    pub violations: [usize; 2],
}

impl AreaConstraints {
    /// The constraints on a region of `shape` pixels `pixel_um` wide, for a minimum island area
    /// and a minimum hole area, µm².
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an empty region or a size that isn't positive and finite.
    pub fn new(
        shape: [usize; 2],
        pixel_um: f64,
        min_area_um2: f64,
        min_hole_um2: f64,
    ) -> Result<AreaConstraints> {
        let ok = |v: f64| v.is_finite() && v > 0.0;
        if shape.contains(&0) || !ok(pixel_um) || !ok(min_area_um2) || !ok(min_hole_um2) {
            return Err(Error::invalid(
                "area constraints",
                format!(
                    "a region of {shape:?} pixels of {pixel_um} µm, areas {min_area_um2} and {min_hole_um2} µm²"
                ),
            ));
        }
        Ok(AreaConstraints {
            shape,
            pixel_um,
            min_area_um2,
            min_hole_um2,
        })
    }

    /// The 4-connected components of `inside` that don't touch the region's edge, each its
    /// pixels.
    fn components(&self, inside: &[bool]) -> Vec<Vec<usize>> {
        let [ni, nj] = self.shape;
        let mut seen = vec![false; ni * nj];
        let mut out = Vec::new();
        for start in 0..ni * nj {
            if !inside[start] || seen[start] {
                continue;
            }
            let (mut stack, mut pixels, mut edge) = (vec![start], Vec::new(), false);
            seen[start] = true;
            while let Some(p) = stack.pop() {
                pixels.push(p);
                let (i, j) = (p % ni, p / ni);
                edge |= i == 0 || j == 0 || i == ni - 1 || j == nj - 1;
                let mut visit = |q: usize| {
                    if inside[q] && !seen[q] {
                        seen[q] = true;
                        stack.push(q);
                    }
                };
                if i > 0 {
                    visit(p - 1);
                }
                if i + 1 < ni {
                    visit(p + 1);
                }
                if j > 0 {
                    visit(p - ni);
                }
                if j + 1 < nj {
                    visit(p + ni);
                }
            }
            if !edge {
                out.push(pixels);
            }
        }
        out
    }

    /// The indicator of the regions of `inside` whose `weight` summed (µm²) is below `minimum`:
    /// their pixels and their 4-neighbours; and how many there are.
    fn indicator(&self, inside: &[bool], weight: &[f64], minimum: f64) -> (Vec<f64>, usize) {
        let [ni, nj] = self.shape;
        let cell = self.pixel_um * self.pixel_um;
        let mut mark = vec![0.0; ni * nj];
        let mut count = 0;
        for region in self.components(inside) {
            let area: f64 = region.iter().map(|&p| weight[p]).sum::<f64>() * cell;
            if area >= minimum {
                continue;
            }
            count += 1;
            for &p in &region {
                let (i, j) = (p % ni, p / ni);
                mark[p] = 1.0;
                if i > 0 {
                    mark[p - 1] = 1.0;
                }
                if i + 1 < ni {
                    mark[p + 1] = 1.0;
                }
                if j > 0 {
                    mark[p - ni] = 1.0;
                }
                if j + 1 < nj {
                    mark[p + ni] = 1.0;
                }
            }
        }
        (mark, count)
    }

    /// g_A and g_EA at the physical densities `physical` (ρ̄, after the projection), with their
    /// gradients in ρ̄ (the indicators held fixed): pull them back to the densities with
    /// [`super::Density::pullback`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for densities that aren't one per pixel.
    pub fn evaluate(&self, physical: &[f64]) -> Result<Areas> {
        let n = self.shape[0] * self.shape[1];
        if physical.len() != n {
            return Err(Error::invalid(
                "area constraints",
                format!("expected {n} densities, got {}", physical.len()),
            ));
        }
        let solid: Vec<bool> = physical.iter().map(|&x| x > 0.6).collect();
        let void: Vec<bool> = physical.iter().map(|&x| x < 0.4).collect();
        let empty: Vec<f64> = physical.iter().map(|&x| 1.0 - x).collect();
        let (islands, ni) = self.indicator(&solid, physical, self.min_area_um2);
        let (holes, nh) = self.indicator(&void, &empty, self.min_hole_um2);
        let cell = self.pixel_um * self.pixel_um;
        let share = cell / (n as f64 * cell);
        let g_a: f64 = physical
            .iter()
            .zip(&islands)
            .map(|(x, m)| x * m)
            .sum::<f64>()
            * share;
        let g_ea: f64 = empty.iter().zip(&holes).map(|(x, m)| x * m).sum::<f64>() * share;
        Ok(Areas {
            values: [g_a, g_ea],
            gradients: [
                islands.iter().map(|m| m * share).collect(),
                holes.iter().map(|m| -m * share).collect(),
            ],
            violations: [ni, nh],
        })
    }
}
