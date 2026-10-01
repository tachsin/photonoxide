//! Marcatili's approximation for the modes of a rectangular dielectric waveguide.
//!
//! E. A. J. Marcatili, "Dielectric rectangular waveguide and directional coupler for integrated
//! optics", Bell Syst. Tech. J. 48, 2071 (1969),
//! [doi:10.1002/j.1538-7305.1969.tb01166.x](https://doi.org/10.1002/j.1538-7305.1969.tb01166.x).
//! A core of index n₁, `width` a by `height` b, with claddings n₂ above, n₄ below, n₃ left and
//! n₅ right; the four corner regions are ignored. The field separates, and
//! k_z² = k₁² − k_x² − k_y² (Eq. 3), with k_x and k_y from two slab-like equations:
//!
//! - E^y_pq modes (E mostly along y): k_x a = pπ − tan⁻¹(k_x ξ₃) − tan⁻¹(k_x ξ₅) (Eq. 6) and
//!   k_y b = qπ − tan⁻¹((n₂²/n₁²) k_y η₂) − tan⁻¹((n₄²/n₁²) k_y η₄) (Eq. 7);
//! - E^x_pq modes: the n² ratios move to the x equation (Eqs. 20–21);
//!
//! with ξ_j = [(k₁² − k_j²) − k_x²]^(−½) and η_j likewise in k_y (Eqs. 8–10). [`Rectangle::mode`]
//! solves the transcendental equations exactly (the paper's solid curves);
//! [`Rectangle::closed_form`] uses its closed-form approximations, Eqs. (12)–(13) and (22)–(23)
//! (the dashed ones). Marcatili states the closed form is within a few percent of the exact
//! solution for (k_z² − k₄²)/(k₁² − k₄²) ≥ 0.5; near cutoff, where the field reaches into the
//! corners, the approximation itself fails. Its error against photonoxide's full-vector solver
//! is measured in the tests and the validation report.

use std::f64::consts::PI;

use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// Which transverse electric field dominates: E^x (along the width) or E^y (along the height).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// E^x_pq: E mostly along x, the width.
    Ex,
    /// E^y_pq: E mostly along y, the height.
    Ey,
}

/// A rectangular core in four claddings (Marcatili's Fig. 4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    core: f64,
    /// Above (n₂), left (n₃), below (n₄), right (n₅).
    above: f64,
    left: f64,
    below: f64,
    right: f64,
    width: f64,
    height: f64,
}

impl Rectangle {
    /// A core of index `core`, `width` × `height`, in one cladding.
    ///
    /// # Errors
    ///
    /// As [`Rectangle::new`].
    pub fn uniform(core: f64, cladding: f64, width: Length, height: Length) -> Result<Rectangle> {
        Rectangle::new(core, [cladding; 4], width, height)
    }

    /// A core of index `core`, `width` × `height`, with claddings `[above, left, below, right]`
    /// (Marcatili's n₂, n₃, n₄, n₅).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for indices below 1 or not below the core's, or dimensions that
    /// aren't positive.
    pub fn new(core: f64, claddings: [f64; 4], width: Length, height: Length) -> Result<Rectangle> {
        if !(core.is_finite()
            && claddings
                .iter()
                .all(|&n| n.is_finite() && n >= 1.0 && n < core))
        {
            return Err(Error::invalid(
                "rectangle",
                format!(
                    "the claddings {claddings:?} must be at least 1 and below the core's {core}"
                ),
            ));
        }
        let (a, b) = (width.to_um(), height.to_um());
        if !(a.is_finite() && a > 0.0 && b.is_finite() && b > 0.0) {
            return Err(Error::invalid(
                "rectangle",
                format!("the size must be positive, got {width} by {height}"),
            ));
        }
        let [above, left, below, right] = claddings;
        Ok(Rectangle {
            core,
            above,
            left,
            below,
            right,
            width: a,
            height: b,
        })
    }

    /// The x and y equations' parameters for one family: (cladding index, n² weight) on each
    /// side, x first.
    fn sides(&self, family: Family) -> [(f64, f64); 4] {
        let n1 = self.core * self.core;
        let w = |n: f64| n * n / n1;
        match family {
            Family::Ey => [
                (self.left, 1.0),
                (self.right, 1.0),
                (self.above, w(self.above)),
                (self.below, w(self.below)),
            ],
            Family::Ex => [
                (self.left, w(self.left)),
                (self.right, w(self.right)),
                (self.above, 1.0),
                (self.below, 1.0),
            ],
        }
    }

    /// The order-`order` root of k d = order·π − tan⁻¹(w₁ k ξ₁) − tan⁻¹(w₂ k ξ₂), with
    /// ξ_j = [(k₁² − k_j²) − k²]^(−½); `None` if it doesn't exist.
    fn transverse(
        k0: f64,
        core: f64,
        d: f64,
        order: usize,
        (n_a, w_a): (f64, f64),
        (n_b, w_b): (f64, f64),
    ) -> Option<f64> {
        let k1 = k0 * core;
        let limit = (k1 * k1 - (k0 * n_a).powi(2))
            .min(k1 * k1 - (k0 * n_b).powi(2))
            .sqrt();
        let f = |k: f64| {
            let xi = |n: f64| 1.0 / (k1 * k1 - (k0 * n).powi(2) - k * k).max(0.0).sqrt();
            k * d + (w_a * k * xi(n_a)).atan() + (w_b * k * xi(n_b)).atan() - order as f64 * PI
        };
        // f rises from −order·π at k = 0
        let (mut lo, mut hi) = (0.0, limit * (1.0 - 1e-15));
        if f(hi) < 0.0 {
            return None;
        }
        while hi - lo > 1e-15 * limit {
            let mid = 0.5 * (lo + hi);
            if mid <= lo || mid >= hi {
                break;
            }
            if f(mid) < 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        Some(0.5 * (lo + hi))
    }

    /// The effective index k_z/k of mode E^family_pq (p, q ≥ 1) from Marcatili's transcendental
    /// equations, solved exactly; `None` if a transverse equation has no root or k_z isn't above
    /// every cladding's wavenumber.
    pub fn mode(&self, family: Family, p: usize, q: usize, wavelength: Wavelength) -> Option<f64> {
        let k0 = wavelength.wavenumber();
        let [left, right, above, below] = self.sides(family);
        let kx = Rectangle::transverse(k0, self.core, self.width, p, left, right)?;
        let ky = Rectangle::transverse(k0, self.core, self.height, q, above, below)?;
        self.effective(k0, kx, ky)
    }

    /// The effective index of mode E^family_pq from Marcatili's closed-form approximations,
    /// Eqs. (12)–(13) for E^y and (22)–(23) for E^x.
    pub fn closed_form(
        &self,
        family: Family,
        p: usize,
        q: usize,
        wavelength: Wavelength,
    ) -> Option<f64> {
        let k0 = wavelength.wavenumber();
        // A_j = π / (k₁² − k_j²)^½ = λ / (2 (n₁² − n_j²)^½), Eq. (10)
        let big_a = |n: f64| PI / (k0 * (self.core * self.core - n * n).sqrt());
        let [(l, wl), (r, wr), (u, wu), (d, wd)] = self.sides(family);
        let kx = p as f64 * PI
            / self.width
            / (1.0 + (wl * big_a(l) + wr * big_a(r)) / (PI * self.width));
        let ky = q as f64 * PI
            / self.height
            / (1.0 + (wu * big_a(u) + wd * big_a(d)) / (PI * self.height));
        self.effective(k0, kx, ky)
    }

    fn effective(&self, k0: f64, kx: f64, ky: f64) -> Option<f64> {
        let k1 = k0 * self.core;
        let kz2 = k1 * k1 - kx * kx - ky * ky;
        let cut = [self.above, self.left, self.below, self.right]
            .into_iter()
            .fold(0.0, f64::max)
            * k0;
        (kz2 > cut * cut).then(|| kz2.sqrt() / k0)
    }
}

/// Marcatili's normalized propagation constant (k_z² − k₄²)/(k₁² − k₄²), the ordinate of his
/// Fig. 6: 0 at cutoff, 1 when all the field is in the core.
pub fn normalized(n_eff: f64, core: f64, cladding: f64) -> f64 {
    (n_eff * n_eff - cladding * cladding) / (core * core - cladding * cladding)
}

/// The largest relative difference in the normalized constant between Marcatili's closed form
/// and his exact transcendental solution, for E^x_11 and E^y_11 of a guide a = 2b where the
/// latter is at least 0.5, over 2b/λ (n₁² − n₄²)^½ from 0.8 to 4 (his Fig. 6b), at λ = 1 µm.
pub(crate) fn closed_form_deviation(core: f64, cladding: f64) -> f64 {
    let lam = Wavelength::from_um_unchecked(1.0);
    let mut worst: f64 = 0.0;
    for k in 0..=32 {
        let big_b = 0.8 + 0.1 * f64::from(k);
        let b = big_b / (2.0 * (core * core - cladding * cladding).sqrt());
        let r = Rectangle::uniform(core, cladding, Length::um(2.0 * b), Length::um(b))
            .expect("a valid rectangle");
        for family in [Family::Ex, Family::Ey] {
            let (Some(exact), Some(closed)) =
                (r.mode(family, 1, 1, lam), r.closed_form(family, 1, 1, lam))
            else {
                continue;
            };
            let (e, c) = (
                normalized(exact, core, cladding),
                normalized(closed, core, cladding),
            );
            if e >= 0.5 {
                worst = worst.max((c - e).abs() / e);
            }
        }
    }
    worst
}

/// Marcatili's guide a = 2b, n₁ = 1.5 in n₁/1.05 (his Fig. 6b), at normalized height
/// B = 2b/λ (n₁² − n₄²)^½ and λ = 1 µm: the fundamental mode of `family` by photonoxide's
/// full-vector solver (a quarter, behind the mode's walls; b/40 in the core) and by Marcatili's
/// transcendental equations, as normalized constants.
pub(crate) fn against_vector(big_b: f64, family: Family) -> (f64, f64) {
    use crate::mode::vector::{
        self, Boundaries, Boundary, CrossSection, Permittivity, graded_nodes,
    };
    use num_complex::Complex64 as c64;
    let lam = Wavelength::from_um_unchecked(1.0);
    let (core, clad) = (1.5f64, 1.5f64 / 1.05);
    let b = big_b / (2.0 * (core * core - clad * clad).sqrt());
    let a = 2.0 * b;
    let h = b / 40.0;
    let x = graded_nodes(
        0.0,
        a / 2.0 + 4.0,
        (0.0, a / 2.0 + 0.2),
        h,
        0.05,
        0.1,
        &[a / 2.0],
    );
    let y = graded_nodes(
        0.0,
        b / 2.0 + 4.0,
        (0.0, b / 2.0 + 0.2),
        h,
        0.05,
        0.1,
        &[b / 2.0],
    );
    let mut cells = Vec::new();
    for i in 0..x.len() - 1 {
        for j in 0..y.len() - 1 {
            let inside = 0.5 * (x[i] + x[i + 1]) < a / 2.0 && 0.5 * (y[j] + y[j + 1]) < b / 2.0;
            let n = if inside { core } else { clad };
            cells.push(Permittivity::isotropic(c64::new(n * n, 0.0)));
        }
    }
    // E along x is even about x = 0, normal to it: the plane x = 0 is an electric wall for H
    let (west, south) = match family {
        Family::Ex => (Boundary::ElectricWall, Boundary::MagneticWall),
        Family::Ey => (Boundary::MagneticWall, Boundary::ElectricWall),
    };
    let cs = CrossSection::new(x, y, cells)
        .and_then(|cs| {
            cs.with_boundaries(Boundaries {
                west,
                south,
                ..Boundaries::default()
            })
        })
        .expect("a valid guide");
    let found = vector::modes(&cs, lam, 1, None).map_or(f64::NAN, |m| m[0].effective_index().re);
    let marcatili = Rectangle::uniform(core, clad, Length::um(a), Length::um(b))
        .ok()
        .and_then(|r| r.mode(family, 1, 1, lam))
        .unwrap_or(f64::NAN);
    (
        normalized(found, core, clad),
        normalized(marcatili, core, clad),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::Polarization;
    use crate::mode::slab::Slab;

    fn lam() -> Wavelength {
        Wavelength::um(1.0).unwrap()
    }

    #[test]
    fn a_very_wide_rectangle_is_the_slab() {
        // a → ∞: k_x → 0, and E^y (E across the slab's layers) is the slab's TM mode, E^x its TE
        let (core, clad, b) = (1.5, 1.45, 1.0);
        let r = Rectangle::uniform(core, clad, Length::um(2000.0), Length::um(b)).unwrap();
        let slab = Slab::new(clad, core, clad, Length::um(b)).unwrap();
        for (family, pol) in [
            (Family::Ey, Polarization::Tm),
            (Family::Ex, Polarization::Te),
        ] {
            let want = slab.modes(pol, lam())[0].effective_index();
            let got = r.mode(family, 1, 1, lam()).unwrap();
            assert!((got - want).abs() < 1e-6, "{family:?}: {got} vs {want}");
        }
    }

    #[test]
    fn the_closed_form_is_within_a_few_percent_where_marcatili_says() {
        // "for a guide and mode for which (k_z² − k₄²)/(k₁² − k₄²) ≥ 0.5, the closed form
        // approximation is within a few percent of the exact value" (p. 2083): his Fig. 6b guide
        let (core, clad) = (1.5, 1.5 / 1.05);
        let worst = closed_form_deviation(core, clad);
        // 4.1 % at most here
        assert!(worst < 0.05, "{worst}");
    }

    #[test]
    fn marcatili_meets_the_vector_solver_far_from_cutoff_and_not_near_it() {
        // normalized constants: 1e-4 apart at B = 3, 9e-3 at B = 1 near cutoff, where the field
        // reaches into the corners Marcatili ignores
        for family in [Family::Ex, Family::Ey] {
            let (v3, m3) = against_vector(3.0, family);
            let (v1, m1) = against_vector(1.0, family);
            assert!((v3 - m3).abs() < 5e-4, "{family:?} B 3: {v3} vs {m3}");
            assert!(
                (v1 - m1).abs() > 5e-3 && (v1 - m1).abs() < 2e-2,
                "{family:?} B 1: {v1} vs {m1}"
            );
        }
    }

    #[test]
    fn bad_rectangles_are_errors() {
        let um = Length::um;
        assert!(Rectangle::uniform(1.5, 1.6, um(1.0), um(1.0)).is_err());
        assert!(Rectangle::uniform(1.5, 1.4, um(0.0), um(1.0)).is_err());
        assert!(Rectangle::new(1.5, [1.4, 1.4, 0.5, 1.4], um(1.0), um(1.0)).is_err());
    }
}
