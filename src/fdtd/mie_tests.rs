use num_complex::Complex64 as c64;

use super::mie::{Mie, Sphere, log_derivatives};
use super::mie_checks::{TABLE_ONE_OFF, lossless_balance, table_one_entries, truncation_error};
use crate::units::Wavelength;

fn c(re: f64, im: f64) -> c64 {
    c64::new(re, im)
}

/// Q_sca of a small sphere over Rayleigh's limit, (8/3) α⁴ |(m² − 1)/(m² + 2)|², less one: the
/// series' approach to its first term (Mie's §15).
fn rayleigh_ratio(size: f64, index: c64) -> f64 {
    let mie = Mie::new(size, Some(index)).unwrap();
    let m2 = index * index;
    let limit = 8.0 / 3.0 * size.powi(4) * ((m2 - 1.0) / (m2 + 2.0)).norm_sqr();
    mie.scattering() / limit - 1.0
}

/// ψ₁ and ψ₁′ in closed form: Mie's Eq. 25 summed, sin z/z − cos z.
fn psi1(z: c64) -> (c64, c64) {
    let psi = z.sin() / z - z.cos();
    (psi, z.sin() - psi / z)
}

/// ξ₁ = ψ₁ − iχ₁ and ξ₁′, χ₁ = cos x/x + sin x.
fn xi1(x: f64) -> (c64, c64) {
    let xi0 = c(x.sin(), -x.cos());
    let xi = c(x.sin() / x - x.cos(), -(x.cos() / x + x.sin()));
    (xi, xi0 - xi / x)
}

#[test]
fn log_derivatives_match_closed_forms() {
    for z in [
        c(0.3, 0.0),
        c(2.0, 0.0),
        c(7.5, 0.0),
        c(1.5, 0.4),
        c(0.5, 3.5),
        c(4.0, 12.0),
    ] {
        let d = log_derivatives(z, 4);
        assert!(
            (d[0] - z.cos() / z.sin()).norm() < 1e-13 * d[0].norm().max(1.0),
            "{z}: D0"
        );
        let (psi, dpsi) = psi1(z);
        assert!(
            (d[1] - dpsi / psi).norm() < 1e-12 * d[1].norm().max(1.0),
            "{z}: D1"
        );
        let psi2 = (3.0 / (z * z) - 1.0) * z.sin() - 3.0 * z.cos() / z;
        let dpsi2 = psi - 2.0 * psi2 / z;
        assert!(
            (d[2] - dpsi2 / psi2).norm() < 1e-11 * d[2].norm().max(1.0),
            "{z}: D2"
        );
    }
}

#[test]
fn first_coefficients_match_closed_forms() {
    // including the four Table I entries where Mie's arithmetic differs (see mie_checks)
    let cases = [
        (0.5, c(1.5, 0.0)),
        (2.0, c(3.5, 0.01)),
        (6.0, c(1.33, 0.0)),
        (0.4f64.sqrt(), c64::new(-3.20, 1.57).sqrt()),
        (0.8f64.sqrt(), c64::new(-4.84, 1.26).sqrt()),
        (0.2f64.sqrt(), c64::new(-6.97, 1.63).sqrt()),
    ];
    for (x, m) in cases {
        let mie = Mie::new(x, Some(m)).unwrap();
        let (p, dp) = psi1(c(x, 0.0));
        let (pm, dpm) = psi1(m * x);
        let (xi, dxi) = xi1(x);
        let a = (m * pm * dp - p * dpm) / (m * pm * dxi - xi * dpm);
        let b = (pm * dp - m * p * dpm) / (pm * dxi - m * xi * dpm);
        assert!((mie.electric[0] - a).norm() < 1e-13, "a1 at {x}, {m}");
        assert!((mie.magnetic[0] - b).norm() < 1e-13, "b1 at {x}, {m}");
    }
}

#[test]
fn reproduces_mies_table_one() {
    let entries = table_one_entries();
    assert_eq!(entries.len(), 66);
    let mut deviations: Vec<f64> = entries.iter().map(|e| (e.series - e.mie).norm()).collect();
    deviations.sort_by(f64::total_cmp);
    let median = deviations[deviations.len() / 2];
    assert!(median < 0.004, "median deviation {median}");
    let off: Vec<_> = entries
        .iter()
        .filter(|e| (e.series - e.mie).norm() > 0.016)
        .map(|e| (e.wavelength, e.size_squared))
        .collect();
    assert_eq!(
        off, TABLE_ONE_OFF,
        "the entries off by more than Mie's rounding"
    );
}

#[test]
fn lossless_spheres_scatter_what_they_take() {
    let worst = lossless_balance();
    assert!(worst < 1e-12, "{worst}");
}

#[test]
fn small_spheres_scatter_as_rayleigh() {
    // the next term is of order α²
    for index in [c(1.5, 0.0), c(3.5, 0.0), c(0.5, 2.0)] {
        let coarse = rayleigh_ratio(1e-2, index).abs();
        let fine = rayleigh_ratio(1e-3, index).abs();
        assert!(coarse < 1e-3, "{index}: {coarse}");
        assert!(
            (coarse / fine - 100.0).abs() < 5.0,
            "{index}: {coarse} then {fine}"
        );
    }
    // a perfect conductor: Q_sca → (10/3) α⁴, the magnetic dipole adding a quarter
    let size: f64 = 1e-3;
    let mie = Mie::new(size, None).unwrap();
    let ratio = mie.scattering() / (10.0 / 3.0 * size.powi(4));
    assert!((ratio - 1.0).abs() < 1e-5, "{ratio}");
}

#[test]
fn converges_in_the_number_of_terms() {
    for (size, index) in [
        (5.0, c(1.5, 0.0)),
        (50.0, c(1.33, 0.0)),
        (200.0, c(1.5, 0.05)),
    ] {
        let short = truncation_error(size, index, (size / 2.0) as usize);
        assert!(
            short > 1e-2,
            "α = {size}: half the terms already converge, {short}"
        );
        // past α the error falls faster than exponentially
        let mut previous = f64::INFINITY;
        let start = size.ceil() as usize;
        for terms in (start..start + 40).step_by(4) {
            let error = truncation_error(size, index, terms);
            assert!(
                error <= previous,
                "α = {size}: not falling at {terms} terms"
            );
            previous = error;
        }
        let enough = (size + 4.0 * size.cbrt() + 10.0) as usize;
        let error = truncation_error(size, index, enough);
        assert!(error < 1e-12, "α = {size}: {error} at {enough} terms");
    }
}

#[test]
fn large_spheres_take_twice_their_shadow() {
    let mie = Mie::new(5000.0, Some(c(1.5, 0.01))).unwrap();
    assert!(
        (mie.extinction() - 2.0).abs() < 0.01,
        "{}",
        mie.extinction()
    );
    assert!(mie.absorption() > 0.0);
}

#[test]
fn a_matched_sphere_scatters_nothing() {
    let mie = Mie::new(3.0, Some(c(1.0, 0.0))).unwrap();
    assert!(mie.extinction().abs() < 1e-15 && mie.scattering() < 1e-30);
    // and weakly, as |m − 1|²
    let a = Mie::new(3.0, Some(c(1.001, 0.0))).unwrap().scattering();
    let b = Mie::new(3.0, Some(c(1.0001, 0.0))).unwrap().scattering();
    assert!((a / b - 100.0).abs() < 0.5, "{}", a / b);
}

#[test]
fn a_good_conductor_tends_to_a_perfect_one() {
    let perfect = Mie::new(2.0, None).unwrap();
    let mut previous = f64::INFINITY;
    for k in [1e2, 1e3, 1e4] {
        let metal = Mie::new(2.0, Some(c(k, k))).unwrap();
        let gap = (metal.extinction() - perfect.extinction()).abs();
        assert!(gap < previous, "{k}: {gap}");
        previous = gap;
    }
    assert!(previous < 1e-3, "{previous}");
}

#[test]
fn absorbing_spheres_absorb() {
    for x in [0.3, 1.0, 4.0] {
        let mie = Mie::new(x, Some(c(0.4, 3.0))).unwrap();
        assert!(mie.absorption() > 0.0 && mie.scattering() > 0.0, "{x}");
    }
}

#[test]
fn sphere_cross_sections_are_in_square_micrometres() {
    // a silicon-like sphere of radius 0.2 µm in glass, at 1.55 µm
    let sphere = Sphere {
        radius: 0.2,
        permittivity: Some(c(12.0, 0.0)),
        background: 2.25,
    };
    let f = Wavelength::um(1.55).unwrap().frequency();
    let sections = sphere.cross_sections(f).unwrap();
    let mie = Mie::new(
        1.5 * 0.2 * std::f64::consts::TAU / 1.55,
        Some(c(12f64.sqrt() / 1.5, 0.0)),
    )
    .unwrap();
    let shadow = std::f64::consts::PI * 0.04;
    assert!((sections.scattering / (shadow * mie.scattering()) - 1.0).abs() < 1e-12);
    assert!(sections.absorption.abs() < 1e-15);
}

#[test]
fn rejects_what_isnt_a_sphere() {
    assert!(Mie::new(0.0, None).is_err());
    assert!(Mie::new(f64::NAN, None).is_err());
    assert!(Mie::new(1.0, Some(c(1.5, -0.1))).is_err());
    assert!(Mie::new(1.0, Some(c(0.0, 0.0))).is_err());
    assert!(Mie::new(1.0, Some(c(-0.1, 1.0))).is_err());
    assert!(Mie::with_terms(1.0, None, 0).is_err());
    assert!(Mie::new(1e6, None).is_err());
    let f = Wavelength::um(1.0).unwrap().frequency();
    let sphere = |radius, background| Sphere {
        radius,
        permittivity: None,
        background,
    };
    assert!(sphere(-1.0, 1.0).mie(f).is_err());
    assert!(sphere(1.0, 0.0).mie(f).is_err());
}

#[test]
fn an_fdtd_sphere_scatters_as_mies() {
    // coarse, 4 cells a radius, and short: the run's machinery, not its accuracy
    use super::mie_checks::{Material, damped_drude, sphere_run};
    use crate::fdtd::Average;
    let run = sphere_run(4, &Material::Dielectric(4.0, Average::Subpixel), 60.0);
    assert!(run.mean_error(false) < 0.15, "{}", run.mean_error(false));
    assert!(
        run.spurious_absorption() < 0.02,
        "{}",
        run.spurious_absorption()
    );
    let run = sphere_run(4, &Material::Dispersive(damped_drude()), 60.0);
    assert!(run.mean_error(true) < 0.08, "{}", run.mean_error(true));
}

#[test]
fn a_lossless_metal_absorbs_nothing() {
    // ε = −4, m = 2i: the field inside decays, and the sphere scatters what it takes
    for x in [0.3, 1.0, 5.0] {
        let mie = Mie::new(x, Some(c(0.0, 2.0))).unwrap();
        assert!(mie.absorption().abs() < 1e-13 * mie.scattering(), "{x}");
    }
}
