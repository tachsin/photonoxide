use super::rc::{Foster, RcFit, RcModel};

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1e-300)
}

/// A 2 × 2 model on three poles, each residue matrix symmetric positive definite, and its step
/// responses at `times`.
fn known() -> RcModel {
    RcModel::new(
        vec![2e-6, 2e-5, 3e-4],
        vec![
            vec![vec![1.0, 0.1], vec![0.1, 0.8]],
            vec![vec![2.0, 0.5], vec![0.5, 2.5]],
            vec![vec![0.6, 0.4], vec![0.4, 0.7]],
        ],
    )
    .unwrap()
}

fn steps(m: &RcModel, times: &[f64]) -> Vec<Vec<Vec<f64>>> {
    (0..m.inputs())
        .map(|j| {
            (0..m.outputs())
                .map(|i| times.iter().map(|&t| m.step(i, j, t)).collect())
                .collect()
        })
        .collect()
}

#[test]
fn a_known_sum_of_exponentials_is_returned() {
    let m = known();
    let times: Vec<f64> = (0..=600).map(|k| k as f64 * 2e-6).collect();
    let fit = RcModel::fit(
        &times,
        &steps(&m, &times),
        Some(&m.steady()),
        &RcFit::new(3).passive(true),
    )
    .unwrap();
    for (a, b) in fit
        .model
        .time_constants_s()
        .iter()
        .zip(m.time_constants_s())
    {
        assert!(close(*a, *b, 1e-6), "{a} against {b}");
    }
    for (a, b) in fit.model.residues().iter().zip(m.residues()) {
        for (x, y) in a.iter().flatten().zip(b.iter().flatten()) {
            assert!(close(*x, *y, 1e-5), "{x} against {y}");
        }
    }
    assert!(fit.error < 1e-8, "{}", fit.error);
    assert!(fit.model.is_passive());
}

#[test]
fn the_error_falls_as_poles_are_added() {
    let m = known();
    let times: Vec<f64> = (1..=600).map(|k| k as f64 * 2e-6).collect();
    let fits = RcModel::fits(&times, &steps(&m, &times), None, &RcFit::new(3)).unwrap();
    assert!(fits[0].error > fits[1].error && fits[1].error > fits[2].error);
    assert!(fits[2].error < 1e-7, "{}", fits[2].error);
}

#[test]
fn a_simulated_step_is_the_step_response() {
    let m = known();
    let times: Vec<f64> = (0..50).map(|k| k as f64 * 1e-5).collect();
    let on = vec![vec![1.0; times.len()], vec![0.0; times.len()]];
    let y = m.simulate(&times, &on).unwrap();
    for (k, &t) in times.iter().enumerate() {
        assert!((y[1][k] - m.step(1, 0, t)).abs() < 1e-12);
    }
    // on, then off: the turn-off is the step's complement
    let half: Vec<f64> = (0..50).map(|k| f64::from(u8::from(k < 25))).collect();
    let y = m.simulate(&times, &[half, vec![0.0; 50]]).unwrap();
    let t0 = times[25];
    for (k, &t) in times.iter().enumerate().skip(25) {
        let expected = m.step(0, 0, t) - m.step(0, 0, t - t0);
        assert!((y[0][k] - expected).abs() < 1e-12);
    }
}

#[test]
fn a_single_pole_cuts_off_at_its_frequency() {
    let m = RcModel::new(vec![1e-5], vec![vec![vec![3.0]]]).unwrap();
    let f = m.cutoff(0, 0).unwrap();
    assert!(close(f, 1.0 / (2.0 * std::f64::consts::PI * 1e-5), 1e-8));
}

#[test]
fn a_cauer_ladder_has_its_foster_networks_impedance() {
    let f = Foster::new(
        vec![0.3, 2.0, 2.3, 0.2],
        vec![2.5e-6, 2.6e-5, 5.4e-5, 1.2e-4],
    )
    .unwrap();
    let c = f.cauer().unwrap();
    for hz in [0.0, 1e3, 1e4, 3e4, 1e5, 1e6] {
        let (a, b) = (f.impedance(hz), c.impedance(hz));
        assert!((a - b).norm() < 1e-9 * a.norm(), "{hz} Hz: {a} against {b}");
    }
    let total: f64 = c.resistances().iter().sum();
    assert!(close(total, 4.8, 1e-12));
}

#[test]
fn coenen_et_al_s_two_pole_foster_network_has_their_ladder() {
    // Table I, no UCUT: Foster 0.651, 0.872 with 4.880, 14.92; Cauer 1.061, 0.459 with 3.686,
    // 23.01 (the printed values' digits)
    let c = Foster::of_rc(vec![0.651, 0.872], vec![4.880, 14.92])
        .unwrap()
        .cauer()
        .unwrap();
    for (a, b) in c.resistances().iter().zip([1.061, 0.459]) {
        assert!(close(*a, b, 6e-3), "{a} against {b}");
    }
    for (a, b) in c.capacitances().iter().zip([3.686, 23.01]) {
        assert!(close(*a, b, 6e-3), "{a} against {b}");
    }
}

#[test]
fn a_non_square_or_asymmetric_model_is_not_passive() {
    let m = RcModel::new(vec![1e-5], vec![vec![vec![1.0, 0.2], vec![0.1, 1.0]]]).unwrap();
    assert!(!m.is_passive());
    let m = RcModel::new(vec![1e-5], vec![vec![vec![1.0, 2.0], vec![2.0, 1.0]]]).unwrap();
    assert!(!m.is_passive(), "an indefinite residue");
    assert!(RcModel::new(vec![-1.0], vec![vec![vec![1.0]]]).is_err());
}
