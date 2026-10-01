//! What follows from a mode's effective index: group index, dispersion and loss, and following
//! one mode across wavelength.
//!
//! L. Chrostowski, M. Hochberg, *Silicon Photonics Design*, Cambridge University Press (2015),
//! [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168), Section 3.2.9:
//!
//! - the group index n_g = n_eff − λ dn_eff/dλ (Eq. 3.5), the speed c/n_g at which a pulse
//!   travels, which sets the free spectral range of rings and interferometers;
//! - the dispersion parameter D = −(λ/c) d²n_eff/dλ² (Eq. 3.6), here in ps/(nm·km).
//!
//! Both include the materials' dispersion when the effective indices do: build each
//! wavelength's cross-section from the materials at that wavelength. The derivatives are the
//! second-order three-point differences on the samples, which may be unevenly spaced.

use faer::c64;

use crate::mode::vector::{self, CrossSection, VectorMode};
use crate::units::{SPEED_OF_LIGHT, Wavelength};
use crate::{Error, Result};

/// The first and second derivatives of `f` at each of the points `x` (strictly increasing, at
/// least 3), from the parabola through each point and its neighbours (at the ends, the first
/// or last three points).
fn derivatives(x: &[f64], f: &[f64]) -> Vec<(f64, f64)> {
    (0..x.len())
        .map(|i| {
            let c = i.clamp(1, x.len() - 2);
            let (x0, x1, x2) = (x[c - 1], x[c], x[c + 1]);
            let (f0, f1, f2) = (f[c - 1], f[c], f[c + 1]);
            // Lagrange's parabola through the three points, differentiated at x[i]
            let t = x[i];
            let d0 = (2.0 * t - x1 - x2) / ((x0 - x1) * (x0 - x2));
            let d1 = (2.0 * t - x0 - x2) / ((x1 - x0) * (x1 - x2));
            let d2 = (2.0 * t - x0 - x1) / ((x2 - x0) * (x2 - x1));
            let second = 2.0
                * (f0 / ((x0 - x1) * (x0 - x2))
                    + f1 / ((x1 - x0) * (x1 - x2))
                    + f2 / ((x2 - x0) * (x2 - x1)));
            (f0 * d0 + f1 * d1 + f2 * d2, second)
        })
        .collect()
}

/// The wavelengths in micrometres, checked: as many as the indices, at least 3, increasing.
fn checked(wavelengths: &[Wavelength], n_eff: &[f64]) -> Result<Vec<f64>> {
    if wavelengths.len() != n_eff.len() || wavelengths.len() < 3 {
        return Err(Error::invalid(
            "dispersion",
            format!(
                "needs at least 3 wavelengths and an index for each, got {} and {}",
                wavelengths.len(),
                n_eff.len()
            ),
        ));
    }
    let um: Vec<f64> = wavelengths.iter().map(|w| w.to_um()).collect();
    if um.windows(2).any(|w| w[1] <= w[0]) {
        return Err(Error::invalid(
            "dispersion",
            "the wavelengths must be strictly increasing",
        ));
    }
    Ok(um)
}

/// The group index n_g = n_eff − λ dn_eff/dλ at each wavelength (Chrostowski and Hochberg,
/// Eq. 3.5), from the real effective indices of one mode at those wavelengths.
///
/// # Errors
///
/// [`Error::InvalidValue`] for fewer than 3 wavelengths, a different number of indices, or
/// wavelengths that don't increase.
pub fn group_index(wavelengths: &[Wavelength], n_eff: &[f64]) -> Result<Vec<f64>> {
    let um = checked(wavelengths, n_eff)?;
    Ok(derivatives(&um, n_eff)
        .iter()
        .zip(um.iter().zip(n_eff))
        .map(|(&(d1, _), (&l, &n))| n - l * d1)
        .collect())
}

/// The dispersion parameter D = −(λ/c) d²n_eff/dλ² at each wavelength (Chrostowski and
/// Hochberg, Eq. 3.6), in ps/(nm·km); positive is anomalous dispersion.
///
/// # Errors
///
/// As [`group_index`].
pub fn dispersion(wavelengths: &[Wavelength], n_eff: &[f64]) -> Result<Vec<f64>> {
    let um = checked(wavelengths, n_eff)?;
    // λ (µm) · d²n/dλ² (1/µm²) / c (m/s) is in s/(m·µm), which is 1e12 ps/(nm·km)
    Ok(derivatives(&um, n_eff)
        .iter()
        .zip(&um)
        .map(|(&(_, d2), &l)| -l * d2 / SPEED_OF_LIGHT * 1e12)
        .collect())
}

/// The propagation loss of a mode of effective index `n_eff` at `wavelength`, in dB/cm: the
/// power decays as e^(−2 k₀ Im(n_eff) z), with k₀ = 2π/λ.
pub fn loss_db_per_cm(n_eff: c64, wavelength: Wavelength) -> f64 {
    // per micrometre, times 1e4 µm/cm
    10.0 * std::f64::consts::LOG10_E * 2.0 * wavelength.wavenumber() * n_eff.im * 1e4
}

/// The radiation loss of a quarter turn, in dB, of a bend of `radius` whose mode has effective
/// index `n_eff` along the arc at that radius ([`crate::mode::vector::CrossSection::bent`],
/// [`crate::mode::bend::SlabBend`]): the power decays as e^(−2 k₀ Im(n_eff) s) over the arc
/// length s = πR/2.
pub fn bend_loss_db(n_eff: c64, wavelength: Wavelength, radius: crate::units::Length) -> f64 {
    10.0 * std::f64::consts::LOG10_E
        * 2.0
        * wavelength.wavenumber()
        * n_eff.im
        * std::f64::consts::FRAC_PI_2
        * radius.to_um()
}

/// One mode followed across `wavelengths`: the cross-section at each wavelength from
/// `cross_section` (on the same grid at every wavelength), and the mode with effective index
/// nearest `near` at the first (the fundamental when `None`). At each next wavelength, of the
/// `candidates` modes nearest the last effective index, the one whose fields overlap the last
/// mode's most ([`VectorMode::overlap`]); it carries on through crossings with other modes.
///
/// # Errors
///
/// [`Error::InvalidValue`] for no wavelengths, no candidates, or grids that change between
/// wavelengths; and the errors of `cross_section` and of [`vector::modes`].
pub fn track(
    mut cross_section: impl FnMut(Wavelength) -> Result<CrossSection>,
    wavelengths: &[Wavelength],
    near: Option<f64>,
    candidates: usize,
) -> Result<Vec<VectorMode>> {
    let Some((&first, rest)) = wavelengths.split_first() else {
        return Err(Error::invalid("track", "needs at least one wavelength"));
    };
    if candidates == 0 {
        return Err(Error::invalid("track", "needs at least one candidate mode"));
    }
    let not_found = || Error::invalid("track", "the solver returned no mode");
    let mut last = vector::modes(&cross_section(first)?, first, 1, near)?
        .into_iter()
        .next()
        .ok_or_else(not_found)?;
    let mut tracked = Vec::with_capacity(wavelengths.len());
    for &wavelength in rest {
        let found = vector::modes(
            &cross_section(wavelength)?,
            wavelength,
            candidates,
            Some(last.effective_index().re),
        )?;
        let mut best: Option<(f64, VectorMode)> = None;
        for mode in found {
            let o = last.overlap(&mode).ok_or_else(|| {
                Error::invalid("track", "the grid must be the same at every wavelength")
            })?;
            if best.as_ref().is_none_or(|(b, _)| o > *b) {
                best = Some((o, mode));
            }
        }
        let next = best.ok_or_else(not_found)?.1;
        tracked.push(std::mem::replace(&mut last, next));
    }
    tracked.push(last);
    Ok(tracked)
}

/// The group index of a TE slab two ways: from [`group_index`] on the exact effective indices
/// at 1.548 to 1.552 µm, and exactly. Without material dispersion, Hellmann–Feynman applied to
/// E'' + (k²ε − β²)E = 0 gives dβ²/d(k²) = ∫ε E² / ∫E², so n_g = dβ/dk = ⟨ε⟩/n_eff, with ⟨ε⟩
/// from the exact field. The slab: 220 nm of 3.473 between 1.444 and air, at 1.55 µm.
pub(crate) fn te_slab_group_index() -> (f64, f64) {
    use crate::mode::Polarization;
    use crate::mode::slab::Slab;
    use crate::units::Length;
    let slab = Slab::new(1.444, 3.473, 1.0, Length::nm(220.0)).expect("a valid slab");
    let te = |l: f64| slab.modes(Polarization::Te, Wavelength::from_um_unchecked(l))[0];
    let wavelengths: Vec<Wavelength> = (-2..=2)
        .map(|k| Wavelength::from_um_unchecked(1.55 + 0.001 * f64::from(k)))
        .collect();
    let n: Vec<f64> = wavelengths
        .iter()
        .map(|w| te(w.to_um()).effective_index())
        .collect();
    let differences = group_index(&wavelengths, &n).expect("five wavelengths")[2];
    // ⟨ε⟩ by the midpoint rule over 6 µm around the core, where the tails are below 1e-12
    let mode = te(1.55);
    let (mut num, mut den) = (0.0, 0.0);
    let steps = 600_000;
    for s in 0..steps {
        let x = -3.0 + 6.0 * (f64::from(s) + 0.5) / f64::from(steps);
        let e = mode.field(Length::um(x)).0;
        let n = if x < 0.0 {
            1.444f64
        } else if x <= 0.22 {
            3.473
        } else {
            1.0
        };
        num += n * n * e * e;
        den += e * e;
    }
    (differences, num / den / mode.effective_index())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lam(um: f64) -> Wavelength {
        Wavelength::um(um).unwrap()
    }

    #[test]
    fn the_differences_are_exact_on_a_parabola() {
        // uneven spacing: the three-point formulas differentiate a parabola exactly
        let x = [1.0, 1.3, 1.45, 2.0, 2.2];
        let f: Vec<f64> = x.iter().map(|t| 3.0 - 2.0 * t + 0.5 * t * t).collect();
        for (&t, (d1, d2)) in x.iter().zip(derivatives(&x, &f)) {
            assert!((d1 - (-2.0 + t)).abs() < 1e-12, "{t}: {d1}");
            assert!((d2 - 1.0).abs() < 1e-12, "{t}: {d2}");
        }
    }

    #[test]
    fn a_te_slabs_group_index_is_its_mean_permittivity_over_its_index() {
        let (differences, exact) = te_slab_group_index();
        assert!(
            (differences - exact).abs() < 1e-6,
            "{differences} vs {exact}"
        );
        assert!(differences > 3.5 && differences < 4.5, "{differences}");
    }

    #[test]
    fn dispersion_has_its_units() {
        // n = a − b λ² gives d²n/dλ² = −2b: D = 2bλ/c, in ps/(nm·km)
        let wavelengths: Vec<Wavelength> = [1.5, 1.55, 1.6].iter().map(|&l| lam(l)).collect();
        let n: Vec<f64> = wavelengths
            .iter()
            .map(|w| 2.0 - 0.01 * w.to_um().powi(2))
            .collect();
        let d = dispersion(&wavelengths, &n).unwrap();
        let expected = 2.0 * 0.01 * 1.55 / SPEED_OF_LIGHT * 1e12;
        assert!(
            (d[1] - expected).abs() < 1e-9 * expected,
            "{} vs {expected}",
            d[1]
        );
    }

    #[test]
    fn loss_follows_the_imaginary_index() {
        // κ = λ / (4π · 1 cm) loses 1/e of the power per centimetre: 10 log10(e) dB
        let w = lam(1.55);
        let kappa = 1.55 / (4.0 * std::f64::consts::PI * 1e4);
        let db = loss_db_per_cm(c64::new(2.4, kappa), w);
        assert!(
            (db - 10.0 * std::f64::consts::LOG10_E).abs() < 1e-12,
            "{db}"
        );
        assert_eq!(loss_db_per_cm(c64::new(2.4, 0.0), w), 0.0);
    }

    #[test]
    fn a_quarter_turns_loss_follows_the_imaginary_index() {
        // 1 dB over the arc when 2 k Im(n) πR/2 log10(e) = 0.1
        let (w, r) = (lam(1.55), crate::units::Length::um(5.0));
        let im = 0.1
            / (std::f64::consts::LOG10_E
                * 2.0
                * w.wavenumber()
                * std::f64::consts::FRAC_PI_2
                * 5.0);
        assert!((bend_loss_db(c64::new(2.5, im), w, r) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn bad_samples_are_errors() {
        let w: Vec<Wavelength> = [1.5, 1.55, 1.6].iter().map(|&l| lam(l)).collect();
        assert!(group_index(&w[..2], &[2.0, 2.1]).is_err());
        assert!(group_index(&w, &[2.0, 2.1]).is_err());
        let backwards = [w[2], w[1], w[0]];
        assert!(dispersion(&backwards, &[2.0, 2.1, 2.2]).is_err());
    }

    #[test]
    fn tracking_follows_one_mode_and_gives_the_strips_group_index() {
        // a 500 x 220 nm strip of 3.473 in 1.444 (no material dispersion) at 20 nm: TE-like at
        // every wavelength, n_eff falling, and n_g near the book's ~4.2 less the silicon's share
        let strip = |_: Wavelength| Ok(vector::strip(0.02));
        let wavelengths: Vec<Wavelength> = [1.53, 1.55, 1.57].iter().map(|&l| lam(l)).collect();
        let modes = track(strip, &wavelengths, None, 3).unwrap();
        assert!(modes.iter().all(|m| m.te_fraction() > 0.9));
        let n: Vec<f64> = modes.iter().map(|m| m.effective_index().re).collect();
        assert!(n.windows(2).all(|w| w[1] < w[0]), "{n:?}");
        let ng = group_index(&wavelengths, &n).unwrap()[1];
        assert!(ng > 3.9 && ng < 4.4, "{ng}");
        // a mode overlaps itself fully, and a changed grid is refused
        assert!((modes[0].overlap(&modes[0]).unwrap() - 1.0).abs() < 1e-12);
        let other = &vector::modes(&vector::strip(0.025), lam(1.55), 1, None).unwrap()[0];
        assert!(modes[0].overlap(other).is_none());
    }
}
