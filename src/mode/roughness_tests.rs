use super::*;
use crate::mode::vector::{self, Boundaries, Permittivity};

fn lam(um: f64) -> Wavelength {
    Wavelength::um(um).unwrap()
}

#[test]
fn gauss_legendre_integrates_polynomials_exactly() {
    for n in [1, 2, 5, 20, 48] {
        let (x, w) = gauss_legendre(n);
        // ∫ x^m over [−1, 1] for every m up to 2n − 1
        for m in 0..2 * n {
            let sum: f64 = x.iter().zip(&w).map(|(&x, &w)| w * x.powi(m as i32)).sum();
            let exact = if m % 2 == 1 {
                0.0
            } else {
                2.0 / (m as f64 + 1.0)
            };
            assert!(
                (sum - exact).abs() < 1e-13,
                "n {n}, x^{m}: {sum} vs {exact}"
            );
        }
    }
}

/// Each shape's |F(k)|² against ∫ Υ(x) e^(−ikx) dx by quadrature, also at the cosine's and the
/// sine's removable poles (|k|a = π, 2π); the cosine's and the sine's against Barwicz and
/// Haus's Eqs. 10 and 11 as printed away from them.
#[test]
fn the_shapes_array_factors_are_their_transforms() {
    let a: f64 = 0.2;
    for shape in [Shape::Square, Shape::Cosine, Shape::Sine] {
        let profile = |x: f64| match shape {
            Shape::Square => 1.0 / a.sqrt(),
            Shape::Cosine => (2.0 / a).sqrt() * (PI * x / a).cos(),
            Shape::Sine => (2.0 / a).sqrt() * (2.0 * PI * x / a).sin(),
        };
        for k in [
            0.0,
            3.0,
            PI / a,
            -PI / a,
            2.0 * PI / a,
            2.0 * PI / a + 1e-9,
            77.0,
        ] {
            let re = integrate(-a / 2.0, a / 2.0, 8, &|x| profile(x) * (k * x).cos());
            let im = integrate(-a / 2.0, a / 2.0, 8, &|x| profile(x) * (k * x).sin());
            let numeric = re * re + im * im;
            let ours = shape.power(a, k);
            assert!(
                (ours - numeric).abs() < 1e-12 * numeric + 1e-15,
                "{shape:?} at k = {k}: {ours} vs {numeric}"
            );
        }
        // the printed forms, u = ka/2
        let k = 9.0;
        let u = k * a / 2.0;
        let printed = match shape {
            Shape::Square => continue,
            Shape::Cosine => 8.0 * a * PI * PI * u.cos().powi(2) / (PI * PI - 4.0 * u * u).powi(2),
            Shape::Sine => {
                32.0 * a * PI * PI * u.sin().powi(2) / (4.0 * PI * PI - 4.0 * u * u).powi(2)
            }
        };
        assert!((shape.power(a, k) - printed).abs() < 1e-13 * printed);
    }
}

use crate::mode::roughness_checks::{SLABS, rectangle};

fn nm(v: f64) -> Length {
    Length::nm(v)
}

/// Payne and Lacey's Eq. 9 against our exact slab's field normalized to ∫ φ² = 1: twice it, as
/// Barwicz and Haus say.
#[test]
fn payne_and_lacey_eq9_is_twice_the_normalized_field() {
    let r = Roughness::exponential(nm(1.0), nm(50.0)).unwrap();
    for (n1, n2, t) in SLABS {
        let ours = slab_loss(n1, n2, Length::um(t), lam(1.55), &r).unwrap();
        let pl = payne_lacey(n1, n2, Length::um(t), lam(1.55), &r).unwrap();
        let ratio = pl.eq9 / ours.field_at_wall;
        assert!((ratio - 2.0).abs() < 1e-11, "{n1} in {n2}: {ratio}");
    }
}

/// The volume current on our exact slab mode (both walls, power, by quadrature) against Payne
/// and Lacey: twice their Eq. 1 with the exact φ²(d), and so their Eq. 10 as printed, each with
/// Eq. 1's 1/n₁ read as k₀/β; their S (Eq. 8) against our quadrature; and Eq. 10 the same as Eq.
/// 1 with Eq. 9's φ²(d), the identity their algebra gives (so Eqs. 10–12 are read right).
#[test]
fn the_volume_current_is_twice_payne_and_lacey_eq1_and_their_eq10() {
    for lc in [5.0, 50.0, 500.0] {
        let r = Roughness::exponential(nm(1.0), nm(lc)).unwrap();
        for (n1, n2, t) in SLABS {
            let ours = slab_loss(n1, n2, Length::um(t), lam(1.55), &r).unwrap();
            let pl = payne_lacey(n1, n2, Length::um(t), lam(1.55), &r).unwrap();
            let weak = n1 / ours.effective_index;
            let eq1 = pl.eq1(ours.field_at_wall, n1, n2, lam(1.55));
            let close = |a: f64, b: f64| (a - b).abs() < 1e-11 * b;
            assert!(close(ours.spectrum_integral, pl.eq8), "{lc} {n1}: S");
            assert!(close(ours.alpha, 2.0 * eq1 * weak), "{lc} {n1}: Eq. 1");
            assert!(close(ours.alpha, pl.eq10 * weak), "{lc} {n1}: Eq. 10");
            assert!(
                close(pl.eq1(pl.eq9, n1, n2, lam(1.55)), pl.eq10),
                "{lc} {n1}"
            );
        }
    }
}

/// Beside nothing (the core's index the cladding's), the far field of a current is its part
/// across the direction, as in the cladding alone.
#[test]
fn a_slab_of_the_cladding_is_no_slab() {
    let base = Radiation {
        k0: lam(1.55).wavenumber(),
        cladding: 1.45,
        eps_core: 1.45 * 1.45,
        eps_cladding: 1.45 * 1.45,
        width: Some(0.4),
        height: 0.2,
        beta: 2.0 * lam(1.55).wavenumber(),
        roughness: Roughness::exponential(nm(1.0), nm(50.0)).unwrap(),
        printed: false,
    };
    let free = Radiation {
        width: None,
        ..base
    };
    for r in [
        [0.3f64, 0.4, 0.866],
        [-0.6, 0.0, 0.8],
        [0.1, 0.6, -0.8],
        [-0.2, -0.9, 0.387],
    ] {
        let n = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
        let r = r.map(|v| v / n);
        for j in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] {
            assert!(
                (base.far(j, r) - free.far(j, r)).abs() < 1e-14,
                "{r:?} {j:?}"
            );
        }
    }
}

/// Barwicz and Haus's 3D analysis in the cladding alone (their Section IV), a rod of uniform
/// current (the square shape) the field of our exact slab's TE₀ mode carries: as the rod grows
/// tall, its loss per height tends to the 2D volume current's, which a line source's Green's
/// function gives, from below, its shortfall falling as 1/a (their Fig. 4(a), P3D/P2D → 1).
#[test]
fn a_tall_rods_loss_tends_to_the_slabs() {
    let r = Roughness::exponential(nm(1.0), nm(50.0)).unwrap();
    let (n1, n2, t) = SLABS[2];
    let slab = slab_loss(n1, n2, Length::um(t), lam(1.55), &r).unwrap();
    let k0 = lam(1.55).wavenumber();
    let beta = slab.effective_index * k0;
    let shortfall = |a: f64| {
        let rod = Radiation {
            k0,
            cladding: n2,
            eps_core: n1 * n1,
            eps_cladding: n2 * n2,
            width: None,
            height: a,
            beta,
            roughness: r,
            printed: false,
        };
        // v²/P = E²/(power per height) = 2k₀φ²(d)/β, with Z₀ = 1
        let field = 2.0 * k0 * slab.field_at_wall / beta;
        let alpha =
            rod.prefactor(n1 * n1 - n2 * n2, field) * rod.integral([0.0, 1.0, 0.0], Shape::Square);
        1.0 - alpha / slab.alpha
    };
    let (s10, s40) = (shortfall(10.0), shortfall(40.0));
    assert!(s10 > 0.0 && s40 > 0.0, "{s10} {s40}");
    assert!((s10 / s40 - 4.0).abs() < 0.2, "1/a: {s10} {s40}");
    assert!(s40 < 0.01, "{s40}");
}

/// The sphere's quadrature has converged: twice the points in each direction move the integral
/// by under 1e-9 relative, beside a silicon core for both currents and shapes.
#[test]
fn the_spheres_quadrature_has_converged() {
    let k0 = lam(1.55).wavenumber();
    let rod = Radiation {
        k0,
        cladding: 1.45,
        eps_core: 3.5 * 3.5,
        eps_cladding: 1.45 * 1.45,
        width: Some(0.4),
        height: 0.2,
        beta: 2.4 * k0,
        roughness: Roughness::exponential(nm(1.0), nm(50.0)).unwrap(),
        printed: false,
    };
    for (j, shape) in [
        ([1.0, 0.0, 0.0], Shape::Cosine),
        ([0.0, 1.0, 0.0], Shape::Cosine),
        ([0.0, 0.0, 1.0], Shape::Sine),
    ] {
        let coarse = rod.integral(j, shape);
        let fine = rod.integral_with(j, shape, 48, 96);
        assert!(
            (coarse - fine).abs() < 1e-9 * fine,
            "{j:?}: {coarse} vs {fine}"
        );
    }
}

/// Along the wall's normal the s and p waves are one wave: a current along the wall radiates
/// the same there whichever way it points. Our reflected p-wave, (1 − R̂_TM), keeps it to
/// round-off beside silicon; Barwicz and Haus's G₁,₁ as printed, (1 + R̂_TM), doesn't.
#[test]
fn a_current_along_the_wall_radiates_alike_along_its_normal() {
    let k0 = lam(1.55).wavenumber();
    let mut rod = Radiation {
        k0,
        cladding: 1.45,
        eps_core: 3.5 * 3.5,
        eps_cladding: 1.45 * 1.45,
        width: Some(0.4),
        height: 0.2,
        beta: 2.2 * k0,
        roughness: Roughness::exponential(nm(1.0), nm(50.0)).unwrap(),
        printed: false,
    };
    for normal in [[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]] {
        let (s, p) = (
            rod.far([0.0, 1.0, 0.0], normal),
            rod.far([0.0, 0.0, 1.0], normal),
        );
        assert!((s - p).abs() < 1e-14 * s, "{normal:?}: {s} vs {p}");
    }
    rod.printed = true;
    let normal = [1.0, 0.0, 0.0];
    let (s, p) = (
        rod.far([0.0, 1.0, 0.0], normal),
        rod.far([0.0, 0.0, 1.0], normal),
    );
    assert!(p > 2.0 * s, "{s} vs {p}");
}

/// The corner's exponent: 1 without contrast, 2/3 as the contrast grows without bound, and
/// the root of its equation in between.
#[test]
fn the_corners_exponent_solves_its_equation() {
    assert_eq!(corner_exponent(2.0, 2.0), 1.0);
    assert!((corner_exponent(1e12, 1.0) - 2.0 / 3.0).abs() < 1e-5);
    let (e1, e2) = (3.5f64 * 3.5, 1.45f64 * 1.45);
    let tau = corner_exponent(e1, e2);
    let t = tau * PI / 4.0;
    assert!((e1 * t.tan() + e2 * (3.0 * t).tan()).abs() < 1e-9, "{tau}");
    assert!(tau > 2.0 / 3.0 && tau < 1.0);
}

/// The quarter behind mirror walls and the whole cross-section give the same loss: the
/// mirrors' factors on the power and on the wall's field are right (to 2e-5 here, the whole\n/// cross-section's mode found to the eigensolver's tolerance, not by its symmetry).
#[test]
fn a_quarter_and_the_whole_guide_lose_alike() {
    let r = Roughness::exponential(nm(1.0), nm(50.0)).unwrap();
    let loss = |full: bool| {
        let cs = rectangle((0.8, 0.4), (2.0, 1.45), 0.02, 0.8, true, full);
        let mode = crate::mode::hadley::modes(&cs, lam(1.55), 1, None).unwrap();
        assert!(mode[0].te_fraction() > 0.5);
        sidewall_loss(&cs, &mode[0], lam(1.55), &r).unwrap()
    };
    let (quarter, whole) = (loss(false), loss(true));
    assert!(quarter.te_like && whole.te_like);
    assert!(
        (quarter.size.0 - whole.size.0).abs() < 1e-12
            && (quarter.size.1 - whole.size.1).abs() < 1e-12
    );
    assert!(
        (quarter.alpha - whole.alpha).abs() < 1e-4 * whole.alpha,
        "{} vs {}",
        quarter.alpha,
        whole.alpha
    );
}

/// A core that isn't a rectangle, a cladding that isn't uniform, and a wall on the open edge
/// are refused.
#[test]
fn what_the_analysis_doesnt_cover_is_refused() {
    let r = Roughness::exponential(nm(1.0), nm(50.0)).unwrap();
    let cs = rectangle((0.8, 0.4), (2.0, 1.45), 0.05, 0.5, true, false);
    let mode = vector::modes(&cs, lam(1.55), 1, None).unwrap();
    assert!(sidewall_loss(&cs, &mode[0], lam(1.55), &r).is_ok());
    let mut cells = cs.cells().to_vec();
    let ny = cs.y().len() - 1;
    // a cell of a third medium in the cladding
    cells[(cs.x().len() - 2) * ny + ny - 1] = Permittivity::isotropic(c64::new(1.0, 0.0));
    let bad = CrossSection::new(cs.x().to_vec(), cs.y().to_vec(), cells)
        .unwrap()
        .with_boundaries(cs.boundaries())
        .unwrap();
    let err = sidewall_loss(&bad, &mode[0], lam(1.55), &r).unwrap_err();
    assert!(err.to_string().contains("uniform"), "{err}");
    let walled = cs
        .clone()
        .with_boundaries(Boundaries {
            east: Boundary::MagneticWall,
            ..cs.boundaries()
        })
        .unwrap();
    assert!(sidewall_loss(&walled, &mode[0], lam(1.55), &r).is_err());
    assert!(Roughness::exponential(nm(-1.0), nm(50.0)).is_err());
    assert!(Roughness::exponential(nm(1.0), nm(0.0)).is_err());
}

/// The slab cases pass their rules.
#[test]
fn the_slab_cases_hold() {
    assert!((crate::mode::roughness_checks::eq9_ratio() - 2.0).abs() < 2e-10);
    assert!(crate::mode::roughness_checks::slab_error() < 1e-10);
}

/// The grid case passes its rule, and its study converges.
#[test]
fn the_silicon_grid_case_holds() {
    let s = crate::mode::roughness_checks::silicon_study();
    let c = s.check(0.05).unwrap();
    eprintln!("{:?} {:?} {}", s.values, c.limit, c.reason);
    let v = &s.values;
    let order = ((v[1] - v[0]) / (v[2] - v[1])).log2();
    assert!(
        (order - crate::mode::roughness_checks::corner_order()).abs() < 0.1,
        "{order}"
    );
}

/// The strip waveguide device with σ above 0: at 3D its loss is the typed one plus the mode's
/// roughness loss, given as the figure `alpha_rough` on the device's grid, and that is the
/// module's on the same cross-section; the analytic model refuses it; σ = 0 changes nothing.
#[test]
fn the_waveguide_device_adds_its_roughness_loss() {
    use crate::circuit::Fidelity;
    use crate::device::{Device, Mesh, Scheme, StripWaveguide, Values};
    let d = StripWaveguide::new().with_mesh(Mesh::fitted(0.02, 1.0, Scheme::Hadley));
    let plain = Values::defaults(&d);
    let rough = Values::new(&d, &[("roughness", 0.002), ("correlation", 0.05)]).unwrap();
    let b0 = d.component(&plain, Fidelity::ThreeD).unwrap();
    let b = d.component(&rough, Fidelity::ThreeD).unwrap();
    assert!(b0.figure("alpha_rough").is_none());
    assert_eq!(b0.values[1], 2.7);
    let fig = b.figure("alpha_rough").unwrap().clone();
    assert_eq!(fig.unit, "dB/cm");
    assert!((b.values[1] - (2.7 + fig.value)).abs() < 1e-12);
    // the module's on the device's cross-section, by Hadley's equations
    let cs = d.cross_section(&rough, lam(1.55)).unwrap();
    let mode = crate::mode::hadley::modes(&cs, lam(1.55), 1, None).unwrap();
    let r = Roughness::exponential(nm(2.0), nm(50.0)).unwrap();
    let ours = sidewall_loss(&cs, &mode[0], lam(1.55), &r).unwrap();
    assert!(ours.te_like);
    assert!((ours.db_per_cm() - fig.value).abs() < 1e-12 * fig.value);
    let err = d.component(&rough, Fidelity::Analytic).unwrap_err();
    assert!(err.to_string().contains("solve the modes (3D)"), "{err}");
    let rib = Values::new(&d, &[("roughness", 0.002), ("slab", 0.09)]).unwrap();
    assert!(d.component(&rib, Fidelity::ThreeD).is_err());
    // a job's design gives them as lengths
    let design =
        crate::device::Design::new(&d, &[("roughness", "2 nm"), ("correlation", "0.1 um")])
            .unwrap();
    assert!((design.values().get("roughness").unwrap() - 0.002).abs() < 1e-15);
    assert!((design.values().get("correlation").unwrap() - 0.1).abs() < 1e-15);
    assert!(crate::device::Design::new(&d, &[("roughness", "60 nm")]).is_err());
}
