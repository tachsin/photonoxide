//! A crystal's orientation: the rotation, the tensor it turns, and the slab modes it gives.

use super::checks::{cut_slab_error, cut_slab_exact, cuts, eulers, turned_tensor_error};
use super::*;
use crate::material::Material;
use crate::material::catalogue::{self, Conditions};
use crate::mode::Polarization;
use crate::mode::slab::Slab;
use crate::units::{Length, Wavelength};

fn c(x: f64) -> Complex64 {
    Complex64::new(x, 0.0)
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
    // so its eigenvalues are n_x², n_y², n_z²; and it stays symmetric
    let e = turned_tensor_error();
    assert!(e < 1e-13, "{e}");
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

#[test]
fn x_cut_and_z_cut_slabs_have_their_modes_exactly_at_second_order() {
    // a 600 nm slab of a negative uniaxial crystal (n_o 2.21, n_e 2.14, about lithium
    // niobate's at 1550 nm) in 1.444, the guide along the chip's x: the cross-section's x is
    // the chip's y, its y the chip's z, its z the chip's x. An x-cut puts the c axis across
    // (TE sees n_e, TM n_o), a z-cut up (TE sees n_o, TM n_e normal to the slab and n_o along
    // it): the swap, exact against the slab's closed forms. Exact n_eff: x-cut TE 1.970741, TM
    // 1.956465; z-cut TE 2.039088, TM 1.901142; the errors at 5 nm -9.8e-6, -6.4e-6, -1.1e-5
    // and -5.6e-6, each a quarter of 10 nm's (order 2.00)
    let (_, ne, clad, d, lam) = super::checks::SLAB;
    let iso_tm = Slab::new(clad, ne, clad, Length::um(d))
        .unwrap()
        .modes(Polarization::Tm, Wavelength::um(lam).unwrap())[0]
        .effective_index();
    // the swap: the x-cut's TE is the n_e slab's, below the z-cut's TE (n_o); the z-cut's TM is
    // between the isotropic slabs of n_e and n_o
    let x_te = cut_slab_exact(CrystalAxis::X, Polarization::Te);
    let z_te = cut_slab_exact(CrystalAxis::Z, Polarization::Te);
    let z_tm = cut_slab_exact(CrystalAxis::Z, Polarization::Tm);
    let x_tm = cut_slab_exact(CrystalAxis::X, Polarization::Tm);
    assert!(
        x_te < z_te && z_tm > iso_tm && z_tm < x_tm,
        "{x_te} {z_te} {z_tm} {x_tm}"
    );
    for cut in [CrystalAxis::X, CrystalAxis::Z] {
        for pol in [Polarization::Te, Polarization::Tm] {
            let errors: Vec<f64> = [0.01, 0.005]
                .iter()
                .map(|&h| cut_slab_error(cut, pol, h))
                .collect();
            let order = (errors[0] / errors[1]).abs().log2();
            assert!(
                (order - 2.0).abs() < 0.15,
                "{cut:?} {pol:?}: order {order}, {errors:?}"
            );
            assert!(errors[1].abs() < 1e-4, "{cut:?} {pol:?}: {errors:?}");
        }
    }
}
