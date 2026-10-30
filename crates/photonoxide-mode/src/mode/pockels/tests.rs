use super::*;
use crate::material::CrystalAxis;
use crate::material::catalogue::entry;

/// Lithium niobate's bulk clamped tensor, in the crystal's axes.
fn linbo3() -> Pockels {
    let e = entry("linbo3").unwrap();
    let t = e
        .tensors
        .iter()
        .find(|t| t.label == "bulk, clamped, 633 nm")
        .unwrap();
    Pockels::from_tensor(t).unwrap()
}

#[test]
fn the_voigt_elements_round_trip_and_follow_3m() {
    let r = linbo3();
    let v = r.voigt();
    // r13 = r23, r12 = -r22, r42 = r51, r61 = -r22 (Nye's pattern for 3m)
    assert!((v[0][2] - 9.10).abs() < 1e-12 && (v[1][2] - 9.10).abs() < 1e-12);
    assert!((v[0][1] + 3.40).abs() < 1e-12 && (v[5][0] + 3.40).abs() < 1e-12);
    assert!((v[3][1] - 18.1).abs() < 1e-12);
    // symmetric in its first two indices
    for k in 0..3 {
        assert_eq!(r.element(0, 2, k), r.element(2, 0, k));
        assert_eq!(r.element(1, 2, k), r.element(2, 1, k));
    }
    assert_eq!(Pockels::from_voigt(v).unwrap(), r);
    assert!(Pockels::from_voigt([[f64::NAN; 3]; 6]).is_err());
}

#[test]
fn an_unknown_element_is_refused_until_completed() {
    let e = entry("linbo3").unwrap();
    let film = e
        .tensors
        .iter()
        .find(|t| t.label.starts_with("thin film"))
        .unwrap();
    let bulk = e
        .tensors
        .iter()
        .find(|t| t.label == "bulk, clamped, 633 nm")
        .unwrap();
    assert!(Pockels::from_tensor(film).is_err());
    let r = Pockels::completed(film, bulk).unwrap().voigt();
    assert_eq!(r[2][2], 26.9);
    assert_eq!(r[0][2], 9.10);
    let d = e
        .tensors
        .iter()
        .find(|t| t.kind == TensorKind::SecondOrder)
        .unwrap();
    assert!(Pockels::from_tensor(d).is_err());
}

#[test]
fn turning_a_crystal_moves_its_elements_with_its_axes() {
    let r = linbo3();
    // a z-cut is the crystal's own axes
    let z = Orientation::cut(CrystalAxis::Z, None).unwrap();
    assert_eq!(r.rotated(&z), r);
    // an x-cut with the c axis along the chip's x: r33 is r_xxx on the chip
    let x = Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Z)).unwrap();
    let t = r.rotated(&x);
    assert!((t.element(0, 0, 0) - 31.2e-12).abs() < 1e-24);
    // and across a guide along y, r_xxx again (x across, y up, z along)
    let s = t.in_section(Along::Y);
    assert!((s.element(0, 0, 0) - 31.2e-12).abs() < 1e-24);
    // a full turn about any axis changes nothing
    let full = Orientation::euler_deg(360.0, 0.0, 0.0).unwrap();
    let back = r.rotated(&full);
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                assert!((back.element(i, j, k) - r.element(i, j, k)).abs() < 1e-24);
            }
        }
    }
}

#[test]
fn a_field_along_c_gives_the_textbook_index_change() {
    let r = linbo3();
    let (no, ne) = (2.286, 2.203);
    let eps = [
        [
            c64::new(no * no, 0.0),
            c64::new(0.0, 0.0),
            c64::new(0.0, 0.0),
        ],
        [
            c64::new(0.0, 0.0),
            c64::new(no * no, 0.0),
            c64::new(0.0, 0.0),
        ],
        [
            c64::new(0.0, 0.0),
            c64::new(0.0, 0.0),
            c64::new(ne * ne, 0.0),
        ],
    ];
    let e = 1e6;
    let d = r.permittivity_change(eps, [0.0, 0.0, e]);
    // Δε = 2nΔn with Δn = -n³rE/2
    assert!((d[2][2].re - (-ne.powi(4) * 31.2e-12 * e)).abs() < 1e-15);
    assert!((d[0][0].re - (-no.powi(4) * 9.10e-12 * e)).abs() < 1e-15);
    assert_eq!(d[0][1], c64::new(0.0, 0.0));
}

#[test]
fn vpi_l_is_the_wavelength_over_twice_or_four_times_the_index_per_volt() {
    let w = Wavelength::um(1.55).unwrap();
    let single = half_wave_voltage_length(1e-5, w, Drive::SingleArm).unwrap();
    let pp = half_wave_voltage_length(-1e-5, w, Drive::PushPull).unwrap();
    assert!((single - 1.55e-6 / 2e-5).abs() < 1e-15);
    assert!((pp - single / 2.0).abs() < 1e-15);
    assert!(half_wave_voltage_length(0.0, w, Drive::PushPull).is_err());
    assert!(half_wave_voltage_length(f64::NAN, w, Drive::PushPull).is_err());
}

#[test]
fn a_guide_along_x_turns_tensors_a_quarter_turn() {
    let c = |v: f64| c64::new(v, 0.0);
    let t = [
        [c(1.0), c(0.0), c(0.0)],
        [c(0.0), c(2.0), c(0.0)],
        [c(0.0), c(0.0), c(3.0)],
    ];
    let y = Along::Y.tensor(t);
    assert_eq!((y[0][0], y[1][1], y[2][2]), (c(1.0), c(3.0), c(2.0)));
    let x = Along::X.tensor(t);
    assert_eq!((x[0][0], x[1][1], x[2][2]), (c(2.0), c(3.0), c(1.0)));
}
