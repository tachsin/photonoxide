use super::*;

#[test]
fn the_closed_forms_meet_their_limits() {
    let (gamma, pump, length): (f64, f64, f64) = (1.5, 2.0, 0.7);
    // κ = 0: g = γP, the largest gain (Agrawal, p. 407)
    let dk = -2.0 * gamma * pump;
    assert_eq!(effective_mismatch(gamma, pump, dk), 0.0);
    assert!((parametric_gain(gamma, pump, dk) - gamma * pump).abs() < 1e-15);
    let g = gamma * pump * length;
    assert!((signal_gain(gamma, pump, dk, length) - (1.0 + g.sinh().powi(2))).abs() < 1e-12);
    // g = 0 exactly (κ = ±2γP): the gain is 1 + (γPL)²
    let edge = 0.0;
    assert!((gain_squared(gamma, pump, edge)).abs() < 1e-15);
    assert!((signal_gain(gamma, pump, edge, length) - (1.0 + g * g)).abs() < 1e-12);
    // far from phase matching, Eq. (10.4.8): G ≈ 1 + (γPL)² sinc²(κL/2)
    let far = 400.0;
    let k = effective_mismatch(gamma, pump, far);
    let approx = 1.0 + g * g * ((0.5 * k * length).sin() / (0.5 * k * length)).powi(2);
    assert!((signal_gain(gamma, pump, far, length) / approx - 1.0).abs() < 1e-3);
    assert!(
        (conversion(gamma, pump, dk, length) + 1.0 - signal_gain(gamma, pump, dk, length)).abs()
            < 1e-12
    );
    assert_eq!(parametric_gain(gamma, pump, far), 0.0);
    let w = peak_detuning(1.0, 1.0, -0.05).unwrap();
    assert!((-0.05 * w * w + 2.0).abs() < 1e-12);
    assert!(peak_detuning(1.0, 1.0, 0.05).is_none());
}

#[test]
fn the_coupled_equations_give_the_closed_form_and_keep_their_invariants() {
    let fw = FourWave {
        gamma: 1.0,
        mismatch: -2.0,
    };
    let start = Waves {
        pump: c64::new(1.0, 0.0),
        signal: c64::new(1e-6, 0.0),
        idler: c64::new(0.0, 0.0),
    };
    let end = fw.propagate(start, 2.0, 400).unwrap();
    let g = end.signal.norm_sqr() / start.signal.norm_sqr();
    assert!(
        (g / signal_gain(1.0, 1.0, -2.0, 2.0) - 1.0).abs() < 3e-9,
        "{g}"
    );
    // the idler is the signal's gain less one, Eq. (10.4.6)
    let i = end.idler.norm_sqr() / start.signal.norm_sqr();
    assert!(
        (i / conversion(1.0, 1.0, -2.0, 2.0) - 1.0).abs() < 3e-9,
        "{i}"
    );
    // depleted: the invariants
    let strong = Waves {
        signal: c64::new(0.5, 0.0),
        ..start
    };
    let end = fw.propagate(strong, 2.0, 800).unwrap();
    assert!((end.power() - strong.power()).abs() < 1e-10);
    assert!(
        (end.signal.norm_sqr() - end.idler.norm_sqr() - strong.signal.norm_sqr()).abs() < 1e-10
    );
    // the pump gave power away
    assert!(end.pump.norm_sqr() < 0.9);
    assert!(fw.propagate(start, -1.0, 10).is_err());
}
