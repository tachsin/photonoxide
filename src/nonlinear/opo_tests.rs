use super::*;

fn process() -> ThreeWave {
    // a 1.5 µm signal and a 3.0 µm idler from a 1.0 µm pump, 1e-4 /(√W·µm), matched, lossless
    ThreeWave {
        frequencies: [1.0 / 1.5, 1.0 / 3.0],
        coupling: 1e-4,
        mismatch: 0.0,
        loss: [0.0; 3],
    }
}

#[test]
fn the_singly_resonant_threshold_is_cosh_gl_one_over_r() {
    let osc = Oscillator {
        process: process(),
        length: 1e4,
        signal: Feedback::resonant(0.81),
        idler: None,
    };
    let p = osc.threshold(400).unwrap();
    let (n1, n2) = (1.0 / 1.5, 1.0 / 3.0);
    let n3 = n1 + n2;
    let gl = (1.0f64 / 0.9).acosh();
    let expected = (gl / 1e4).powi(2) / (1e-8 * n1 * n2 / (n3 * n3));
    assert!((p / expected - 1.0).abs() < 1e-8, "{p} {expected}");
}

#[test]
fn below_threshold_the_signal_dies_and_above_it_the_pump_is_depleted() {
    let osc = Oscillator {
        process: process(),
        length: 1e4,
        signal: Feedback::resonant(0.9),
        idler: None,
    };
    let p = osc.threshold(200).unwrap();
    let below = osc.steady_state(0.5 * p, 100, 1e-12, 20_000).unwrap();
    assert!(below.signal < 1e-20 * p);
    let above = osc.steady_state(2.0 * p, 100, 1e-12, 20_000).unwrap();
    assert!(above.change < 1e-12);
    assert!(above.pump < 2.0 * p);
    assert!(above.signal > 0.0 && above.idler > 0.0);
}

#[test]
fn the_uniform_limit_is_full_depletion_at_pi_squared_over_four() {
    assert_eq!(uniform_depletion(0.9), 0.0);
    let full = uniform_depletion(std::f64::consts::PI.powi(2) / 4.0);
    assert!((full - 1.0).abs() < 1e-12);
    // x²/sin²x = 2 at x = 1.39156 (sinc's half maximum), so D = x²/2
    let d = uniform_depletion(2.0);
    assert!((d - 1.391_557_4f64.powi(2) / 2.0).abs() < 1e-7, "{d}");
    assert!(uniform_depletion(4.0) < full);
}

#[test]
fn it_refuses_a_cavity_that_cannot_be() {
    let bad = Oscillator {
        process: process(),
        length: 1e4,
        signal: Feedback {
            returned: 0.9,
            output: 0.5,
            phase: 0.0,
        },
        idler: None,
    };
    assert!(bad.threshold(10).is_err());
    let none = Oscillator {
        signal: Feedback::resonant(0.0),
        ..bad
    };
    assert!(none.threshold(10).is_err());
}
