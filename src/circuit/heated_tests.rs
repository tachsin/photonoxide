use std::sync::Arc;

use super::components::{Coupler, PhaseShifter, mzi};
use super::{Component, Heated};
use crate::thermal::RcModel;
use crate::units::Wavelength;

fn heated() -> Heated {
    let coupler: Arc<dyn Component> = Arc::new(Coupler::new());
    let arm: Arc<dyn Component> = Arc::new(PhaseShifter::new());
    let circuit = mzi(coupler.clone(), coupler, arm.clone(), arm)
        .unwrap()
        .compile()
        .unwrap();
    // 100 rad/W in its own arm, 10 in the other, a fast and a slow pole
    let model = RcModel::new(
        vec![1e-5, 1e-4],
        vec![
            vec![vec![80.0, 2.0], vec![2.0, 80.0]],
            vec![vec![20.0, 8.0], vec![8.0, 20.0]],
        ],
    )
    .unwrap();
    Heated::new(circuit, &["upper", "lower"], model).unwrap()
}

fn w() -> Wavelength {
    Wavelength::um(1.55).unwrap()
}

#[test]
fn the_steady_phases_are_the_models() {
    let h = heated();
    let p = h.phases(&[0.01, 0.02]).unwrap();
    assert!((p[0] - (1.0 + 0.2)).abs() < 1e-12);
    assert!((p[1] - (0.1 + 2.0)).abs() < 1e-12);
    assert_eq!(h.provenance().fidelity, super::Fidelity::Compact);
}

#[test]
fn a_transient_ends_at_the_steady_state() {
    let h = heated();
    let times: Vec<f64> = (0..400).map(|k| f64::from(k) * 1e-5).collect();
    let on = vec![vec![0.015; 400], vec![0.0; 400]];
    let t = h.transient(w(), &times, &on).unwrap();
    let steady = h.s_matrix(w(), &[0.015, 0.0]).unwrap();
    assert!((t.s[399][(2, 0)] - steady[(2, 0)]).norm() < 1e-9);
    assert_eq!(
        t.s[0][(2, 0)],
        h.s_matrix(w(), &[0.0, 0.0]).unwrap()[(2, 0)]
    );
}

#[test]
fn the_modulation_at_0_hz_is_the_steady_derivative() {
    let h = heated();
    let at = [0.0157, 0.002];
    let m = h.modulation(w(), &at, (2, 0), 1, &[0.0]).unwrap();
    let d = 1e-7;
    let power = |p: f64| h.s_matrix(w(), &[at[0], p]).unwrap()[(2, 0)].norm_sqr();
    let fd = (power(at[1] + d) - power(at[1] - d)) / (2.0 * d);
    assert!(
        (m.per_watt[0].re - fd).abs() < 1e-6 * fd.abs(),
        "{} against {fd}",
        m.per_watt[0]
    );
    assert!(m.per_watt[0].im.abs() < 1e-12);
    assert!(h.modulation(w(), &at, (2, 0), 2, &[0.0]).is_err());
}
