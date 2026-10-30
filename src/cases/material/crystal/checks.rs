//! The orientation's checks, for its tests and the validation report.

use num_complex::Complex64;

use super::{CrystalAxis, Orientation};
use crate::mode::Polarization;
use crate::mode::slab::Slab;
use crate::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity, modes};
use crate::units::{Length, Wavelength};

fn c(x: f64) -> Complex64 {
    Complex64::new(x, 0.0)
}

/// The cuts, each crystal axis normal to the wafer with each of the others along x.
pub(crate) fn cuts() -> Vec<Orientation> {
    use CrystalAxis::{X, Y, Z};
    let mut all = Vec::new();
    for normal in [X, Y, Z] {
        for along in [X, Y, Z] {
            if along != normal {
                all.extend(Orientation::cut(normal, Some(along)));
            }
        }
    }
    all
}

/// Euler angles spread over the sphere, none of them special.
pub(crate) fn eulers() -> Vec<Orientation> {
    [
        (17.0, 33.0, 71.0),
        (-120.0, 95.0, 10.0),
        (300.0, -45.0, 200.0),
        (5.0, 179.0, -359.0),
    ]
    .iter()
    .filter_map(|&(a, b, g)| Orientation::euler_deg(a, b, g).ok())
    .collect()
}

/// A biaxial tensor (n 1.74, 1.75, 1.84) turned by every cut and by [`eulers`]: the largest
/// relative change of its characteristic polynomial's coefficients (the trace, the sum of the
/// principal 2 × 2 minors, the determinant), which are the principal values' when, and only
/// when, its eigenvalues are n_x², n_y², n_z²; and the largest asymmetry.
pub(crate) fn turned_tensor_error() -> f64 {
    let p = [c(1.74f64.powi(2)), c(1.75f64.powi(2)), c(1.84f64.powi(2))];
    let trace = p[0] + p[1] + p[2];
    let minors = p[0] * p[1] + p[1] * p[2] + p[2] * p[0];
    let det = p[0] * p[1] * p[2];
    let mut worst = 0.0f64;
    for o in cuts().into_iter().chain(eulers()) {
        let t = o.rotate(p);
        let tr = t[0][0] + t[1][1] + t[2][2];
        let mi = t[0][0] * t[1][1] - t[0][1] * t[1][0] + t[1][1] * t[2][2] - t[1][2] * t[2][1]
            + t[0][0] * t[2][2]
            - t[0][2] * t[2][0];
        let de = t[0][0] * (t[1][1] * t[2][2] - t[1][2] * t[2][1])
            - t[0][1] * (t[1][0] * t[2][2] - t[1][2] * t[2][0])
            + t[0][2] * (t[1][0] * t[2][1] - t[1][1] * t[2][0]);
        worst = worst
            .max((tr - trace).norm() / trace.norm())
            .max((mi - minors).norm() / minors.norm())
            .max((de - det).norm() / det.norm());
        for (i, row) in t.iter().enumerate() {
            for (j, e) in row.iter().enumerate() {
                worst = worst.max((e - t[j][i]).norm());
            }
        }
    }
    worst
}

/// The fundamental TM mode of a symmetric slab `d` µm thick whose core's permittivity is
/// `up` normal to it and `along` along the guide (the field's E_y and E_z see them), in a
/// cladding of `clad`: H_x'' = ε_along (β²/ε_up − k₀²) H_x in the core, with H_x and H_x'/ε
/// continuous, so the even mode has (κ/ε_along) tan(κd/2) = γ/ε_clad with
/// κ² = ε_along (k₀² − β²/ε_up) and γ² = β² − ε_clad k₀². Solved by bisection in κd/2.
pub(crate) fn uniaxial_tm(up: f64, along: f64, clad: f64, d: f64, lam: f64) -> f64 {
    let k0 = std::f64::consts::TAU / lam;
    let beta2 = |u: f64| {
        let kappa = 2.0 * u / d;
        up * (k0 * k0 - kappa * kappa / along)
    };
    let f = |u: f64| {
        let kappa = 2.0 * u / d;
        let gamma = (beta2(u) - clad * k0 * k0).max(0.0).sqrt();
        kappa / along * u.tan() - gamma / clad
    };
    let (mut lo, mut hi) = (1e-12, std::f64::consts::FRAC_PI_2 - 1e-12);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if f(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    beta2(0.5 * (lo + hi)).sqrt() / k0
}

/// The slab's numbers: n_o, n_e, the cladding, the thickness (µm) and the wavelength (µm).
pub(crate) const SLAB: (f64, f64, f64, f64, f64) = (2.21, 2.14, 1.444, 0.6, 1.55);

/// The exact fundamental mode of [`SLAB`], a negative uniaxial crystal (about lithium
/// niobate's at 1550 nm) in silica, guided along the chip's x and cut `cut` (with the default
/// axis along x), in `polarization`: an x-cut puts the c axis across (TE sees n_e, TM n_o), a
/// z-cut up (TE sees n_o, TM n_e normal to the slab and n_o along it).
pub(crate) fn cut_slab_exact(cut: CrystalAxis, polarization: Polarization) -> f64 {
    let (no, ne, clad, d, lam) = SLAB;
    let iso = |n: f64| {
        Slab::new(clad, n, clad, Length::um(d))
            .ok()
            .and_then(|s| {
                Wavelength::um(lam)
                    .ok()
                    .and_then(|w| s.modes(polarization, w).first().copied())
            })
            .map_or(f64::NAN, |m| m.effective_index())
    };
    match (cut, polarization) {
        (CrystalAxis::Z, Polarization::Tm) => uniaxial_tm(ne * ne, no * no, clad * clad, d, lam),
        (CrystalAxis::X, Polarization::Te) => iso(ne),
        (CrystalAxis::Z, Polarization::Te) | (CrystalAxis::X, Polarization::Tm) => iso(no),
        _ => f64::NAN,
    }
}

/// The full-vector solver's error on [`SLAB`] cut `cut`, its tensor turned into the
/// cross-section (the chip's y, z and x as its x, y and z), uniform along x, at spacing `h`
/// (µm) in a 4 µm window, against [`cut_slab_exact`].
pub(crate) fn cut_slab_error(cut: CrystalAxis, polarization: Polarization, h: f64) -> f64 {
    let (no, ne, clad, d, lam) = SLAB;
    let exact = cut_slab_exact(cut, polarization);
    let Ok(orientation) = Orientation::cut(cut, None) else {
        return f64::NAN;
    };
    let t = orientation.rotate([c(no * no), c(no * no), c(ne * ne)]);
    let axes = [1, 2, 0];
    let (Ok(core), Ok(cladding)) = (
        Permittivity::from_tensor(std::array::from_fn(|a| {
            std::array::from_fn(|b| t[axes[a]][axes[b]])
        })),
        Permittivity::from_tensor(std::array::from_fn(|a| {
            std::array::from_fn(|b| if a == b { c(clad * clad) } else { c(0.0) })
        })),
    ) else {
        return f64::NAN;
    };
    // TE (E across, along the walls' plane): H tangential to the walls; TM: H normal
    let wall = match polarization {
        Polarization::Te => Boundary::ElectricWall,
        Polarization::Tm => Boundary::MagneticWall,
    };
    let n = (4.02 / h).round() as usize;
    CrossSection::uniform((0.0, 0.08, 4), (-2.01, 2.01, n), |_, y| {
        if y.abs() < d / 2.0 { core } else { cladding }
    })
    .and_then(|cs| {
        cs.with_boundaries(Boundaries {
            west: wall,
            east: wall,
            ..Default::default()
        })
    })
    .ok()
    .zip(Wavelength::um(lam).ok())
    .and_then(|(cs, w)| modes(&cs, w, 1, Some(exact)).ok())
    .and_then(|m| m.first().map(|m| m.effective_index().re))
    .map_or(f64::NAN, |n| n - exact)
}
