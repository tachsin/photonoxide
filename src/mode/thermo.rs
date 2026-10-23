//! A change of the cross-section in a full-vector mode: its effective index's change by
//! first-order perturbation, and the change of permittivity a temperature field makes.
//!
//! **Perturbation.** A change Δε of the relative permittivity, away from moving boundaries,
//! shifts a mode's frequency at fixed β by Δω/ω = −½ ∫ E*·Δε·E dA / ∫ ε|E|² dA, to first order
//! (S. G. Johnson et al., "Perturbation theory for Maxwell's equations with shifting material
//! boundaries", Phys. Rev. E 65, 066611 (2002),
//! [doi:10.1103/PhysRevE.65.066611](https://doi.org/10.1103/PhysRevE.65.066611), their
//! starting point, before the moving boundaries). At fixed ω the shift of β is Δω over the group
//! velocity, which for a lossless guide is the energy's, v_g = ½ Re ∫ (E × H*)·ẑ dA over
//! ¼ ∫ (ε₀ε|E|² + μ₀|H|²) dA = ½ ε₀ ∫ ε|E|² dA. With E in units of Z₀ times H (as
//! [`crate::mode::fields`] gives it) the two give
//!
//! Δn_eff = ∫ E*·Δε·E dA / (2 Re ∫ (E × H*)·ẑ dA),
//!
//! the overlap of |E|² with Δε over twice the power. The material changes uniformly inside each
//! cell, so no boundary moves and the unperturbed field is the one to use. A re-solve of the
//! perturbed cross-section ([`index_change_resolved`]) checks it: the two differ at second order
//! in Δε, and by the field's own discretization error, which falls with the grid.
//!
//! **Temperature.** A material's index moves with its temperature by its thermo-optic
//! coefficient dn/dT ([`crate::material::catalogue::IndexModel::thermo_optic`]). Given the
//! cross-section at a temperature and the same one a step warmer, each cell's
//! dε/dT = (ε_warm − ε_cold)/step, mixing the materials as the cross-section's cells do;
//! [`permittivity_change`] scales it by a temperature rise at each cell's centre: a uniform one,
//! or a heat solve's field ([`crate::thermal`]).

use faer::c64;

use crate::mode::vector::{self, CrossSection, Permittivity, VectorMode};
use crate::units::Wavelength;
use crate::{Error, Result};

/// The change of `mode`'s effective index when the permittivity of `cs`'s cells changes by
/// `delta` (one per cell, in the order of [`CrossSection::cells`]), to first order:
/// ∫ E*·Δε·E dA / (2 Re ∫ (E × H*)·ẑ dA). On a grid cut by mirror walls both integrals cover the
/// part solved, and the ratio is the whole cross-section's.
///
/// # Errors
///
/// [`Error::InvalidValue`] unless `mode` was found on `cs`'s grid and `delta` has one change per
/// cell.
pub fn index_change(mode: &VectorMode, cs: &CrossSection, delta: &[Permittivity]) -> Result<c64> {
    if delta.len() != cs.cells().len() {
        return Err(Error::invalid(
            "delta",
            format!("{} changes for {} cells", delta.len(), cs.cells().len()),
        ));
    }
    let f = mode.fields_sided(cs, true)?;
    let (nx, ny) = (f.x().len(), f.y().len());
    let mut overlap = c64::new(0.0, 0.0);
    for i in 0..nx {
        let dx = cs.x()[i + 1] - cs.x()[i];
        for j in 0..ny {
            let dy = cs.y()[j + 1] - cs.y()[j];
            let d = delta[i * ny + j];
            let e = f.e(i, j);
            let de = [
                d.xx * e[0] + d.xy * e[1],
                d.yx * e[0] + d.yy * e[1],
                d.zz * e[2],
            ];
            overlap += (e[0].conj() * de[0] + e[1].conj() * de[1] + e[2].conj() * de[2]) * dx * dy;
        }
    }
    Ok(overlap / (4.0 * f.power()))
}

/// [`index_change`] cell by cell: each cell's share of the change, the real part of its term
/// E*·Δε·E dA over 4 times the power, in the order of [`CrossSection::cells`]; their sum is
/// [`index_change`]'s real part, to round-off. With `delta` a change per kelvin, the shares are
/// how much a rise of 1 K in each cell alone moves the index: the weights a temperature field
/// is summed with ([`crate::thermal::Guide`]).
///
/// # Errors
///
/// As [`index_change`].
pub fn index_change_density(
    mode: &VectorMode,
    cs: &CrossSection,
    delta: &[Permittivity],
) -> Result<Vec<f64>> {
    if delta.len() != cs.cells().len() {
        return Err(Error::invalid(
            "delta",
            format!("{} changes for {} cells", delta.len(), cs.cells().len()),
        ));
    }
    let f = mode.fields_sided(cs, true)?;
    let (nx, ny) = (f.x().len(), f.y().len());
    let scale = 1.0 / (4.0 * f.power());
    let mut out = vec![0.0; nx * ny];
    for i in 0..nx {
        let dx = cs.x()[i + 1] - cs.x()[i];
        for j in 0..ny {
            let dy = cs.y()[j + 1] - cs.y()[j];
            let d = delta[i * ny + j];
            let e = f.e(i, j);
            let de = [
                d.xx * e[0] + d.xy * e[1],
                d.yx * e[0] + d.yy * e[1],
                d.zz * e[2],
            ];
            let term = e[0].conj() * de[0] + e[1].conj() * de[1] + e[2].conj() * de[2];
            out[i * ny + j] = term.re * dx * dy * scale;
        }
    }
    Ok(out)
}

/// The same change by a re-solve: the mode of `cs` perturbed by `delta` nearest to `mode`'s
/// effective index moved by [`index_change`], less `mode`'s, at `wavelength` (the one `mode` was
/// found at).
///
/// # Errors
///
/// As [`index_change`] and [`vector::modes`].
pub fn index_change_resolved(
    mode: &VectorMode,
    cs: &CrossSection,
    delta: &[Permittivity],
    wavelength: Wavelength,
) -> Result<c64> {
    let first = index_change(mode, cs, delta)?;
    let n = mode.effective_index();
    let moved = vector::modes(&cs.perturbed(delta)?, wavelength, 1, Some((n + first).re))?;
    Ok(moved[0].effective_index() - n)
}

/// Each cell's change of permittivity for a temperature rise `rise(x, y)` (K, at the cell's
/// centre, µm), to first order in it: `cold` is the cross-section with its materials at the
/// reference temperature, `warm` the same one with them `step_k` warmer, and a cell's
/// Δε = (ε_warm − ε_cold) / step × rise.
///
/// # Errors
///
/// [`Error::InvalidValue`] unless the two are on the same grid, for a step that isn't positive
/// and finite, or a rise that isn't finite.
pub fn permittivity_change(
    cold: &CrossSection,
    warm: &CrossSection,
    step_k: f64,
    rise: impl Fn(f64, f64) -> f64,
) -> Result<Vec<Permittivity>> {
    if cold.x() != warm.x() || cold.y() != warm.y() {
        return Err(Error::invalid(
            "warm",
            "the two cross-sections are on different grids",
        ));
    }
    if !(step_k.is_finite() && step_k > 0.0) {
        return Err(Error::invalid(
            "step",
            "the temperature step must be positive and finite",
        ));
    }
    let (x, y) = (cold.x(), cold.y());
    let ny = y.len() - 1;
    cold.cells()
        .iter()
        .zip(warm.cells())
        .enumerate()
        .map(|(n, (a, b))| {
            let (i, j) = (n / ny, n % ny);
            let r = rise(0.5 * (x[i] + x[i + 1]), 0.5 * (y[j] + y[j + 1]));
            if !r.is_finite() {
                return Err(Error::invalid(
                    "rise",
                    format!("the temperature rise at cell ({i}, {j}) isn't finite"),
                ));
            }
            let s = r / step_k;
            Ok(Permittivity {
                xx: (b.xx - a.xx) * s,
                xy: (b.xy - a.xy) * s,
                yx: (b.yx - a.yx) * s,
                yy: (b.yy - a.yy) * s,
                zz: (b.zz - a.zz) * s,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A square of ε = 12 in ε = 2, `n` cells a side.
    fn square(n: usize, eps: f64) -> CrossSection {
        CrossSection::uniform((-1.0, 1.0, n), (-1.0, 1.0, n), |x, y| {
            let e = if x.abs() < 0.3 && y.abs() < 0.2 {
                eps
            } else {
                2.0
            };
            Permittivity::isotropic(c64::new(e, 0.0))
        })
        .unwrap()
    }

    #[test]
    fn a_small_change_moves_the_index_as_a_re_solve_does() {
        let (cs, w) = (square(40, 12.0), Wavelength::um(1.55).unwrap());
        let mode = vector::modes(&cs, w, 1, None).unwrap().remove(0);
        let delta = permittivity_change(&cs, &square(40, 12.01), 1.0, |_, _| 1.0).unwrap();
        let first = index_change(&mode, &cs, &delta).unwrap();
        let resolved = index_change_resolved(&mode, &cs, &delta, w).unwrap();
        assert!(first.re > 0.0 && first.im.abs() < 1e-12, "{first}");
        // a coarse grid: the corners' fields in the overlap, a few percent
        assert!(
            (first.re / resolved.re - 1.0).abs() < 0.05,
            "{first} {resolved}"
        );
        // nothing changed, nothing moves
        let none = permittivity_change(&cs, &cs, 1.0, |_, _| 1.0).unwrap();
        assert_eq!(index_change(&mode, &cs, &none).unwrap(), c64::new(0.0, 0.0));
    }

    #[test]
    fn mismatched_inputs_are_refused() {
        let (cs, w) = (square(20, 12.0), Wavelength::um(1.55).unwrap());
        let mode = vector::modes(&cs, w, 1, None).unwrap().remove(0);
        let other = square(21, 12.0);
        assert!(index_change(&mode, &cs, &[]).is_err());
        assert!(cs.perturbed(&[]).is_err());
        assert!(permittivity_change(&cs, &other, 1.0, |_, _| 1.0).is_err());
        assert!(permittivity_change(&cs, &cs, 0.0, |_, _| 1.0).is_err());
        assert!(permittivity_change(&cs, &cs, 1.0, |_, _| f64::NAN).is_err());
        let delta = permittivity_change(&cs, &cs, 1.0, |_, _| 1.0).unwrap();
        assert!(index_change(&mode, &other, &delta).is_err());
    }
}
