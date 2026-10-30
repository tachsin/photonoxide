//! Every mode in a region of the complex effective-index plane, with no count and no guess: by
//! contour integrals (docs/methods/contour.md).
//!
//! A [`Region`] is a disc or an ellipse of effective indices n_eff = β/k₀. The mode solvers'
//! eigenvalue is β² = k₀² n_eff², so the region's boundary is carried onto the β² plane by that
//! map, and the integral taken there by the trapezoidal rule in the boundary's own parameter
//! (T. Sakurai, H. Sugiura, J. Comput. Appl. Math. 159, 119 (2003),
//! [doi:10.1016/S0377-0427(03)00565-X](https://doi.org/10.1016/S0377-0427(03)00565-X)); the
//! eigenvalues inside are found by FEAST's subspace iteration (E. Polizzi, Phys. Rev. B 79,
//! 115112 (2009), [doi:10.1103/PhysRevB.79.115112](https://doi.org/10.1103/PhysRevB.79.115112);
//! for leaky and lossy modes, J. Kestyn, E. Polizzi, P. T. P. Tang, SIAM J. Sci. Comput. 38,
//! S772 (2016), [doi:10.1137/15M1026572](https://doi.org/10.1137/15M1026572)). The map is one
//! to one on a region that doesn't hold n_eff = 0, where it would fold n and −n together, and
//! a region must lie in Re n_eff ≥ 0, the forward modes' half.
//!
//! [`crate::mode::vector::modes_in`] and [`crate::mode::slab_fd::Profile::modes_in`] search a
//! region; [`Search`] says how, and [`Found`] holds the modes with the count's estimate and each
//! mode's residual.

use num_complex::Complex64 as c64;

use crate::eigen::contour::{self, Quadrature};

pub(crate) mod checks;
use crate::{Error, Result};

/// A region of effective indices: the inside of an ellipse with its axes along the real and
/// imaginary directions (a disc when they are equal).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    centre: c64,
    real: f64,
    imaginary: f64,
}

impl Region {
    /// The disc of effective indices within `radius` of `centre`.
    ///
    /// # Errors
    ///
    /// As [`Region::ellipse`].
    pub fn disc(centre: c64, radius: f64) -> Result<Region> {
        Region::ellipse(centre, radius, radius)
    }

    /// The ellipse of effective indices about `centre` with semi-axes `real` along Re n_eff and
    /// `imaginary` along Im n_eff.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the centre is finite and the semi-axes positive and
    /// finite, and the region lies in Re n_eff ≥ 0 without holding n_eff = 0.
    pub fn ellipse(centre: c64, real: f64, imaginary: f64) -> Result<Region> {
        if !centre.is_finite()
            || !(real > 0.0 && real.is_finite())
            || !(imaginary > 0.0 && imaginary.is_finite())
        {
            return Err(Error::invalid(
                "region",
                format!(
                    "the centre must be finite and the semi-axes positive and finite, got {centre}, {real} and {imaginary}"
                ),
            ));
        }
        let region = Region {
            centre,
            real,
            imaginary,
        };
        if centre.re - real < 0.0 || region.measure(c64::new(0.0, 0.0)) <= 1.0 {
            return Err(Error::invalid(
                "region",
                format!(
                    "must lie in Re n_eff ≥ 0 and not hold n_eff = 0, where β² = k₀² n_eff² folds n and −n together; got centre {centre} with semi-axes {real} and {imaginary}"
                ),
            ));
        }
        Ok(region)
    }

    /// The guided modes' region from `low` to `high` (`low` < `high`, both positive): an
    /// ellipse on the real axis through them, half as tall as it is wide.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless 0 < `low` < `high`, both finite.
    pub fn between(low: f64, high: f64) -> Result<Region> {
        if !(low > 0.0 && high > low && high.is_finite()) {
            return Err(Error::invalid(
                "region",
                format!("the indices must satisfy 0 < low < high, got {low} and {high}"),
            ));
        }
        let half = 0.5 * (high - low);
        Region::ellipse(c64::new(low + half, 0.0), half, 0.5 * half)
    }

    /// Every guided mode's region above `low`, for a guide whose highest index is `highest`:
    /// [`Region::between`] `low` and a tenth of the way beyond `highest`, so the boundary
    /// passes clear of the modes at the top. With `low` the cladding's index it holds every
    /// guided mode.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless 0 < `low` < `highest`, both finite.
    pub fn above(low: f64, highest: f64) -> Result<Region> {
        if !(low > 0.0 && highest > low && highest.is_finite()) {
            return Err(Error::invalid(
                "region",
                format!(
                    "no guided mode lies above {low}: the threshold must be positive and below the highest index, {highest}"
                ),
            ));
        }
        Region::between(low, highest + 0.1 * (highest - low))
    }

    /// The centre.
    pub fn centre(&self) -> c64 {
        self.centre
    }

    /// The semi-axes, along Re n_eff and Im n_eff.
    pub fn semi_axes(&self) -> (f64, f64) {
        (self.real, self.imaginary)
    }

    /// ((Re(n − c))/a)² + ((Im(n − c))/b)²: below 1 inside.
    fn measure(&self, n: c64) -> f64 {
        let d = n - self.centre;
        (d.re / self.real).powi(2) + (d.im / self.imaginary).powi(2)
    }

    /// Whether `n` (an effective index) is inside.
    pub fn contains(&self, n: c64) -> bool {
        self.measure(n) < 1.0
    }

    /// The trapezoidal rule on the boundary carried to β² = k² n²: z(θ) = k² n(θ)²,
    /// z'(θ) = 2 k² n(θ) n'(θ), n(θ) = c + a cos θ + i b sin θ.
    pub(crate) fn quadrature(&self, k2: f64, points: usize) -> Result<Quadrature> {
        let (c, a, b) = (self.centre, self.real, self.imaginary);
        let symmetric = c.im == 0.0 && points.is_multiple_of(2);
        Quadrature::trapezoid(points, symmetric, |t| {
            let n = c + c64::new(a * t.cos(), b * t.sin());
            let dn = c64::new(-a * t.sin(), b * t.cos());
            (k2 * n * n, 2.0 * k2 * n * dn)
        })
    }
}

/// How to search a region.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Search {
    /// Quadrature points on the region's boundary: one sparse factorization each (half of
    /// them for a lossless guide in a region centred on the real axis). 16 by default.
    pub points: usize,
    /// The subspace's size, at least the number of modes in the region; `None` (the default)
    /// chooses it from the count's estimate and grows it as needed. A given size too small for
    /// the region is an error.
    pub subspace: Option<usize>,
    /// The largest relative residual ‖A h − β² h‖ / (α ‖h‖) of a mode, α the largest |β²| on
    /// the boundary. 1e-10 by default.
    pub tolerance: f64,
    /// The most filter passes, each one solve per point. 40 by default.
    pub iterations: usize,
}

impl Default for Search {
    fn default() -> Search {
        Search {
            points: 16,
            subspace: None,
            tolerance: 1e-10,
            iterations: 40,
        }
    }
}

/// The modes found in a region.
#[derive(Clone, Debug, PartialEq)]
pub struct Found<M> {
    /// Every mode in the region, highest Re n_eff first.
    pub modes: Vec<M>,
    /// Each mode's relative residual, as [`Search::tolerance`] measures it.
    pub residuals: Vec<f64>,
    /// The count's estimate before the search: the trace of the first filtered block of ±1
    /// vectors, whose mean is the number of eigenvalues in the region.
    pub estimate: f64,
    /// The size of the subspace that found them.
    pub subspace: usize,
    /// Filter passes taken.
    pub iterations: usize,
}

/// Every eigenvalue β² of the pencil (A, I) of `n` unknowns whose n_eff = √β²/k lies in
/// `region`, highest Re n_eff first: the pairs, their residuals and the search's record.
pub(crate) fn search(
    n: usize,
    entries: &[(usize, usize, c64)],
    positions: &[[f64; 3]],
    k: f64,
    region: &Region,
    search: &Search,
    stop: &dyn Fn() -> bool,
) -> Result<Option<contour::Found>> {
    let quadrature = region.quadrature(k * k, search.points)?;
    let pencil = contour::Pencil {
        n,
        a: entries,
        b: None,
        positions: Some(positions),
    };
    let inside = |beta2: c64| region.contains(beta2.sqrt() / k);
    let Some(mut found) = contour::solve_until(
        &pencil,
        &quadrature,
        &inside,
        &contour::Options {
            subspace: search.subspace,
            tolerance: search.tolerance,
            iterations: search.iterations,
        },
        stop,
    )?
    else {
        return Ok(None);
    };
    // highest Re n_eff first: the order of Re β (both roots principal)
    let mut order: Vec<usize> = (0..found.pairs.len()).collect();
    let key = |i: usize| found.pairs[i].value.sqrt();
    order.sort_by(|&a, &b| {
        key(b)
            .re
            .total_cmp(&key(a).re)
            .then(key(b).im.total_cmp(&key(a).im))
    });
    let mut pairs: Vec<Option<crate::eigen::Pair>> = found.pairs.drain(..).map(Some).collect();
    found.pairs = order
        .iter()
        .map(|&i| pairs[i].take().expect("each once"))
        .collect();
    found.residuals = order.iter().map(|&i| found.residuals[i]).collect();
    Ok(Some(found))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_that_fold_or_reach_backward_modes_are_errors() {
        assert!(Region::disc(c64::new(0.5, 0.0), 0.6).is_err());
        assert!(Region::disc(c64::new(0.5, 0.0), 0.5).is_err());
        assert!(Region::ellipse(c64::new(0.2, 0.5), 0.3, 0.1).is_err());
        assert!(Region::ellipse(c64::new(0.2, 0.5), 0.1, 0.6).is_ok());
        assert!(Region::disc(c64::new(f64::NAN, 0.0), 0.1).is_err());
        assert!(Region::disc(c64::new(1.5, 0.0), 0.0).is_err());
        assert!(Region::between(1.6, 1.5).is_err());
        let r = Region::between(1.5, 1.7).unwrap();
        assert!(r.contains(c64::new(1.51, 0.0)) && r.contains(c64::new(1.69, 0.01)));
        assert!(!r.contains(c64::new(1.49, 0.0)) && !r.contains(c64::new(1.6, 0.06)));
    }

    #[test]
    fn the_rule_integrates_one_over_beta2_minus_a_point_inside() {
        // Σ w_j / (z_j − λ) is 1 for λ = k² n² with n inside, 0 outside
        let r = Region::ellipse(c64::new(2.0, 0.02), 0.4, 0.3).unwrap();
        let k2 = 16.0;
        let q = r.quadrature(k2, 64).unwrap();
        let inside = k2 * c64::new(2.1, 0.03).powi(2);
        let outside = k2 * c64::new(2.6, 0.0).powi(2);
        assert!((q.filter(inside) - 1.0).norm() < 1e-12);
        assert!(q.filter(outside).norm() < 1e-12);
        // a flat ellipse converges more slowly: its boundary passes nearer the points inside
        let flat = Region::ellipse(c64::new(2.0, 0.02), 0.4, 0.1).unwrap();
        let q = flat.quadrature(k2, 64).unwrap();
        assert!((q.filter(inside) - 1.0).norm() < 1e-6);
    }

    #[test]
    fn a_uniform_box_has_every_exact_mode_in_the_region_each_twice() {
        let (worst, found, want) = checks::uniform_box();
        assert_eq!(found, want);
        assert!(worst < 1e-12, "{worst}");
    }

    #[test]
    fn the_four_layer_guide_has_chilwell_and_hodgkinsons_eight_bound_modes() {
        let e = checks::chilwell_bound();
        assert!(e < 2e-6, "{e}");
    }

    #[test]
    fn its_leaky_waves_are_chilwell_and_hodgkinsons() {
        let e = checks::chilwell_leaky();
        assert!(e < 2e-5, "{e}");
    }

    #[test]
    fn a_leaky_region_holds_what_a_dense_solve_finds_there() {
        let e = checks::dense_leaky();
        assert!(e < 2e-9, "{e}");
    }

    #[test]
    fn the_strips_modes_are_shift_and_inverts_on_any_number_of_threads() {
        let e = checks::strip_against_shift_invert();
        assert!(e < 1e-10, "{e}");
        assert_eq!(checks::thread_differences(), 0.0);
    }

    #[test]
    fn the_error_falls_exponentially_with_the_points() {
        let r = checks::exponential_rate();
        assert!((r - 1.0).abs() < 0.2, "{r}");
    }

    #[test]
    fn too_small_a_given_subspace_is_an_error() {
        use crate::mode::Polarization;
        let profile = crate::mode::slab_fd::chilwell_profile(0.004);
        let w = crate::units::Wavelength::from_um_unchecked(0.6328);
        let search = Search {
            subspace: Some(2),
            ..Search::default()
        };
        let e = profile
            .modes_in(Polarization::Te, w, &checks::bound_region(), &search)
            .unwrap_err()
            .to_string();
        assert!(e.contains("subspace"), "{e}");
    }
}
