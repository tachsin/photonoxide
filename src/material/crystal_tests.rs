//! A crystal's orientation: the rotation, the tensor it turns, and the slab modes it gives.

use super::*;
use crate::material::catalogue::{self, Conditions};
use crate::material::{Material, Model, Provenance};
use crate::mode::Polarization;
use crate::mode::slab::Slab;
use crate::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity, modes};
use crate::units::{Length, Wavelength};

fn c(x: f64) -> Complex64 {
    Complex64::new(x, 0.0)
}

/// The cuts, each crystal axis normal to the wafer with each of the others along x.
fn cuts() -> Vec<Orientation> {
    use CrystalAxis::{X, Y, Z};
    let mut all = Vec::new();
    for normal in [X, Y, Z] {
        for along in [X, Y, Z] {
            if along != normal {
                all.push(Orientation::cut(normal, Some(along)).unwrap());
            }
        }
    }
    all
}

/// Euler angles spread over the sphere, none of them special.
fn eulers() -> Vec<Orientation> {
    [
        (17.0, 33.0, 71.0),
        (-120.0, 95.0, 10.0),
        (300.0, -45.0, 200.0),
        (5.0, 179.0, -359.0),
    ]
    .iter()
    .map(|&(a, b, g)| Orientation::euler_deg(a, b, g).unwrap())
    .collect()
}

#[test]
fn every_orientation_is_a_proper_rotation_and_zero_angles_are_the_identity() {
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    // exactly: cos 0 and sin 0 are 1 and 0, and a z-cut with x along x is the crystal's frame
    assert_eq!(
        Orientation::euler_deg(0.0, 0.0, 0.0).unwrap().matrix(),
        identity
    );
    assert_eq!(
        Orientation::cut(CrystalAxis::Z, None).unwrap().matrix(),
        identity
    );
    for o in cuts().into_iter().chain(eulers()) {
        let m = o.matrix();
        // M Mᵀ = I and det M = +1: a rotation, never a reflection
        for i in 0..3 {
            for j in 0..3 {
                let dot: f64 = (0..3).map(|k| m[i][k] * m[j][k]).sum();
                assert!((dot - identity[i][j]).abs() < 1e-15, "{o}: {m:?}");
            }
        }
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        assert!((det - 1.0).abs() < 1e-15, "{o}: det {det}");
    }
}

#[test]
fn a_turned_tensors_eigenvalues_are_the_principal_ones() {
    // a biaxial tensor turned every way: its characteristic polynomial's coefficients (the
    // trace, the sum of the principal 2 × 2 minors, the determinant) are the principal values',
    // so its eigenvalues are n_x², n_y², n_z²
    let p = [c(1.74f64.powi(2)), c(1.75f64.powi(2)), c(1.84f64.powi(2))];
    let trace = p[0] + p[1] + p[2];
    let minors = p[0] * p[1] + p[1] * p[2] + p[2] * p[0];
    let det = p[0] * p[1] * p[2];
    for o in cuts().into_iter().chain(eulers()) {
        let t = o.rotate(p);
        let tr = t[0][0] + t[1][1] + t[2][2];
        let mi = t[0][0] * t[1][1] - t[0][1] * t[1][0] + t[1][1] * t[2][2] - t[1][2] * t[2][1]
            + t[0][0] * t[2][2]
            - t[0][2] * t[2][0];
        let de = t[0][0] * (t[1][1] * t[2][2] - t[1][2] * t[2][1])
            - t[0][1] * (t[1][0] * t[2][2] - t[1][2] * t[2][0])
            + t[0][2] * (t[1][0] * t[2][1] - t[1][1] * t[2][0]);
        assert!((tr - trace).norm() < 1e-13, "{o}");
        assert!((mi - minors).norm() < 1e-12, "{o}");
        assert!((de - det).norm() < 1e-12, "{o}");
        // and symmetric
        for (i, row) in t.iter().enumerate() {
            for (j, e) in row.iter().enumerate() {
                assert!((e - t[j][i]).norm() < 1e-15, "{o}");
            }
        }
    }
}

#[test]
fn euler_angles_and_the_cut_they_make_turn_a_tensor_alike() {
    // R_x(90°) R_z(90°) puts the crystal's x on the chip's z and its y on the chip's −x: an
    // x-cut with y along x (the sign changes nothing in ε)
    let p = [c(2.0), c(3.0), c(4.5)];
    let a = Orientation::euler_deg(0.0, 90.0, 90.0).unwrap().rotate(p);
    let b = Orientation::cut(CrystalAxis::X, None).unwrap().rotate(p);
    for i in 0..3 {
        for j in 0..3 {
            assert!((a[i][j] - b[i][j]).norm() < 1e-15, "{a:?} vs {b:?}");
        }
    }
    // the x-cut's c axis (crystal z) on the chip's y, its x up
    assert_eq!(b[1][1], c(4.5));
    assert_eq!(b[2][2], c(2.0));
}

#[test]
fn lithium_niobates_cuts_put_n_e_where_the_c_axis_lies() {
    let entry = catalogue::entry("linbo3").unwrap();
    let w = Wavelength::um(1.55).unwrap();
    let [o, _, e] = entry.principal(None, Conditions::default()).unwrap();
    let (eo, ee) = (o.permittivity(w).unwrap(), e.permittivity(w).unwrap());
    let x_cut = entry
        .oriented(
            None,
            Conditions::default(),
            Orientation::cut(CrystalAxis::X, None).unwrap(),
        )
        .unwrap();
    let z_cut = entry
        .oriented(
            None,
            Conditions::default(),
            Orientation::cut(CrystalAxis::Z, None).unwrap(),
        )
        .unwrap();
    let (tx, tz) = (x_cut.tensor(w).unwrap(), z_cut.tensor(w).unwrap());
    // x-cut: the c axis along the chip's y; z-cut: up
    assert_eq!([tx[0][0], tx[1][1], tx[2][2]], [eo, ee, eo]);
    assert_eq!([tz[0][0], tz[1][1], tz[2][2]], [eo, eo, ee]);
    assert!(x_cut.is_anisotropic() && x_cut.orientation().is_some());
    // one number for pictures: a third of the trace, the same in every orientation
    let mean = (eo + eo + ee) / 3.0;
    assert!((x_cut.permittivity(w).unwrap() - mean).norm() < 1e-15);
    assert!((z_cut.permittivity(w).unwrap() - mean).norm() < 1e-15);
    // an isotropic entry has no axes to turn, and a principal material must be isotropic
    let si = catalogue::entry("si").unwrap();
    assert!(
        si.oriented(
            None,
            Conditions::default(),
            Orientation::cut(CrystalAxis::X, None).unwrap()
        )
        .is_err()
    );
    assert!(
        Material::oriented(
            "twice",
            [x_cut.clone(), o.clone(), e],
            Orientation::cut(CrystalAxis::Z, None).unwrap()
        )
        .is_err()
    );
    // an isotropic material's tensor is ε times the identity, to the bit
    let t = o.tensor(w).unwrap();
    assert_eq!(
        [t[0][0], t[1][1], t[2][2], t[0][1], t[1][2]],
        [eo, eo, eo, c(0.0), c(0.0)]
    );
}

#[test]
fn an_orientation_is_read_from_a_job_and_bad_ones_refused() {
    let read = |s: &str| {
        let v: toml::Table = toml::from_str(&format!("o = {s}")).unwrap();
        Orientation::from_value(&v["o"], "orientation")
    };
    assert_eq!(
        read("\"x-cut\"").unwrap(),
        Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Y)).unwrap()
    );
    assert_eq!(
        read("{ cut = \"y\", along_x = \"z\" }").unwrap(),
        Orientation::cut(CrystalAxis::Y, Some(CrystalAxis::Z)).unwrap()
    );
    assert_eq!(
        read("{ euler_deg = [10, 20.5, -30] }").unwrap(),
        Orientation::euler_deg(10.0, 20.5, -30.0).unwrap()
    );
    for (bad, says) in [
        ("\"w-cut\"", "unknown orientation"),
        ("{ cut = \"x\", along_x = \"x\" }", "can't lie along"),
        ("{ cut = \"q\" }", "unknown cut"),
        ("{ euler_deg = [1, 2] }", "three numbers"),
        ("{ euler_deg = [1, 2, 400] }", "from -360 to 360"),
        ("{ euler_deg = [1, nan, 3] }", "from -360 to 360"),
        ("{ euler_deg = [1, 2, 3], cut = \"x\" }", "not both"),
        (
            "{ euler_deg = [1, 2, 3], along_x = \"y\" }",
            "without along_x",
        ),
        ("{ cut = \"x\", angle = 3 }", "unknown key"),
        ("{}", "needs a cut"),
        ("3", "must be"),
    ] {
        let e = read(bad).unwrap_err().to_string();
        assert!(e.contains(says), "{bad}: {e}");
    }
}

/// A material of one fixed index, for the slabs.
fn constant(n: f64) -> Material {
    Material::new(
        format!("n = {n}"),
        Model::Constant(c(n)),
        Wavelength::um(0.4).unwrap(),
        Wavelength::um(5.0).unwrap(),
        Provenance {
            reference: "a test".into(),
            doi: String::new(),
            data: String::new(),
            temperature: None,
            notes: String::new(),
        },
    )
    .unwrap()
}

/// The fundamental TM mode of a symmetric slab `d` µm thick whose core's permittivity is
/// `up` normal to it and `along` along the guide (the field's E_y and E_z see them), in a
/// cladding of `clad`: H_x'' = ε_along (β²/ε_up − k₀²) H_x in the core, with H_x and H_x'/ε
/// continuous, so the even mode has (κ/ε_along) tan(κd/2) = γ/ε_clad with
/// κ² = ε_along (k₀² − β²/ε_up) and γ² = β² − ε_clad k₀². Solved by bisection in κd/2.
fn uniaxial_tm(up: f64, along: f64, clad: f64, d: f64, lam: f64) -> f64 {
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

#[test]
fn x_cut_and_z_cut_slabs_have_their_modes_exactly_at_second_order() {
    // a 600 nm slab of a negative uniaxial crystal (n_o 2.21, n_e 2.14, about lithium
    // niobate's at 1550 nm) in 1.444, the guide along the chip's x: the cross-section's x is
    // the chip's y, its y the chip's z, its z the chip's x. An x-cut puts the c axis across
    // (TE sees n_e, TM n_o), a z-cut up (TE sees n_o, TM n_e normal to the slab and n_o along
    // it): the swap, exact against the slab's closed forms. Exact n_eff: x-cut TE 1.970741, TM
    // 1.956465; z-cut TE 2.039088, TM 1.901142; the errors at 5 nm -9.8e-6, -6.4e-6, -1.1e-5
    // and -5.6e-6, each a quarter of 10 nm's (order 2.00)
    let (no, ne, clad, d, lam) = (2.21, 2.14, 1.444, 0.6, 1.55);
    let w = Wavelength::um(lam).unwrap();
    let principal = || [constant(no), constant(no), constant(ne)];
    let iso = |n: f64, pol| {
        Slab::new(clad, n, clad, Length::um(d))
            .unwrap()
            .modes(pol, w)[0]
            .effective_index()
    };
    let cases = [
        (CrystalAxis::X, Polarization::Te, iso(ne, Polarization::Te)),
        (CrystalAxis::X, Polarization::Tm, iso(no, Polarization::Tm)),
        (CrystalAxis::Z, Polarization::Te, iso(no, Polarization::Te)),
        (
            CrystalAxis::Z,
            Polarization::Tm,
            uniaxial_tm(ne * ne, no * no, clad * clad, d, lam),
        ),
    ];
    // the z-cut's TM is between the isotropic slabs of n_o and n_e
    assert!(cases[3].2 > iso(ne, Polarization::Tm) && cases[3].2 < iso(no, Polarization::Tm));
    for (cut, pol, exact) in cases {
        let crystal =
            Material::oriented("crystal", principal(), Orientation::cut(cut, None).unwrap())
                .unwrap();
        let t = crystal.tensor(w).unwrap();
        // chip (y, z, x) as the cross-section's (x, y, z)
        let axes = [1, 2, 0];
        let core = Permittivity::from_tensor(std::array::from_fn(|a| {
            std::array::from_fn(|b| t[axes[a]][axes[b]])
        }))
        .unwrap();
        let cladding = Permittivity::from_tensor(std::array::from_fn(|a| {
            std::array::from_fn(|b| if a == b { c(clad * clad) } else { c(0.0) })
        }))
        .unwrap();
        // TE (E along the walls' plane, across): H tangential to the walls; TM: H normal
        let wall = match pol {
            Polarization::Te => Boundary::ElectricWall,
            Polarization::Tm => Boundary::MagneticWall,
        };
        let errors: Vec<f64> = [0.01f64, 0.005]
            .iter()
            .map(|&h| {
                let n = (4.02 / h).round() as usize;
                let cs = CrossSection::uniform((0.0, 0.08, 4), (-2.01, 2.01, n), |_, y| {
                    if y.abs() < d / 2.0 { core } else { cladding }
                })
                .unwrap()
                .with_boundaries(Boundaries {
                    west: wall,
                    east: wall,
                    ..Default::default()
                })
                .unwrap();
                modes(&cs, w, 1, Some(exact)).unwrap()[0]
                    .effective_index()
                    .re
                    - exact
            })
            .collect();
        let order = (errors[0] / errors[1]).abs().log2();
        assert!(
            (order - 2.0).abs() < 0.15,
            "{cut:?} {pol:?}: order {order}, {errors:?}"
        );
        assert!(errors[1].abs() < 1e-4, "{cut:?} {pol:?}: {errors:?}");
    }
}
