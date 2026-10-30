use super::*;

fn real(x: f64) -> c64 {
    c64::new(x, 0.0)
}

#[test]
fn the_conversions_are_the_units() {
    // 100 %/(W·cm²) is 1 /(W·cm²) = 1e-8 /(W·µm²)
    assert!((coupling_from_efficiency(100.0).powi(2) - 1e-8).abs() < 1e-22);
    // 10 dB/cm is a factor 10 in power over 1 cm
    assert!(((-loss_from_db_per_cm(10.0) * 1e4).exp() - 0.1).abs() < 1e-15);
}

#[test]
fn undepleted_shg_is_the_sinc2_of_phase_matching() {
    let length = 4000.0;
    let shg = SecondHarmonic {
        coupling: coupling_from_efficiency(4500.0),
        mismatch: 2.0 / length,
        loss: [0.0; 2],
    };
    let p0: f64 = 1e-12;
    let [_, a2] = shg
        .propagate([real(p0.sqrt()), real(0.0)], length, 2000)
        .unwrap();
    let expected =
        shg.coupling.powi(2) * p0 * p0 * crate::nonlinear::phase::undepleted(shg.mismatch, length);
    assert!((a2.norm_sqr() / expected - 1.0).abs() < 1e-9);
    assert!((undepleted_lossy_shg(&shg, p0, length) / expected - 1.0).abs() < 1e-12);
}

#[test]
fn depleted_shg_is_armstrongs_tanh() {
    let shg = SecondHarmonic {
        coupling: 2e-3,
        mismatch: 0.0,
        loss: [0.0; 2],
    };
    // κ√P₀L = 2
    let (p0, length) = (1.0, 1000.0);
    let [a1, a2] = shg.propagate([real(1.0), real(0.0)], length, 2000).unwrap();
    assert!((a2.norm_sqr() - 2.0_f64.tanh().powi(2)).abs() < 1e-10);
    assert!((a1.norm_sqr() + a2.norm_sqr() - p0).abs() < 1e-12);
}

#[test]
fn parametric_gain_is_a_cosh() {
    // a strong pump at ν₃ and a weak signal at ν₁: P₁ grows as cosh²(ΓL), Γ = κ√(ν₁ν₂P₃)/ν₃
    let tw = ThreeWave {
        frequencies: [1.0 / 1.55, 1.0 / 1.6],
        coupling: 1e-3,
        mismatch: 0.0,
        loss: [0.0; 3],
    };
    let [n1, n2] = tw.frequencies;
    let gamma = tw.coupling * (n1 * n2).sqrt() / (n1 + n2);
    let length = 2.0 / gamma;
    let [a1, ..] = tw
        .propagate([real(1e-6), real(0.0), real(1.0)], length, 4000)
        .unwrap();
    assert!((a1.norm_sqr() / 1e-12 / 2.0_f64.cosh().powi(2) - 1.0).abs() < 1e-8);
}

#[test]
fn manley_rowe_holds_without_loss_and_not_with_it() {
    let mut tw = ThreeWave {
        frequencies: [1.0 / 1.55, 1.0 / 1.3],
        coupling: 2e-3,
        mismatch: 3e-3,
        loss: [0.0; 3],
    };
    let start = [real(0.7), c64::new(0.2, 0.3), real(0.1)];
    let before = tw.manley_rowe(&start);
    let after = tw.manley_rowe(&tw.propagate(start, 2000.0, 4000).unwrap());
    for k in 0..2 {
        assert!(
            (after[k] / before[k] - 1.0).abs() < 1e-10,
            "{before:?} {after:?}"
        );
    }
    tw.loss = [1e-4; 3];
    let lossy = tw.manley_rowe(&tw.propagate(start, 2000.0, 4000).unwrap());
    assert!(lossy[0] < 0.9 * before[0]);
}

#[test]
fn a_bad_propagation_is_refused() {
    let shg = SecondHarmonic {
        coupling: 1e-3,
        mismatch: 0.0,
        loss: [0.0; 2],
    };
    assert!(shg.propagate([real(1.0), real(0.0)], 0.0, 10).is_err());
    assert!(shg.propagate([real(1.0), real(0.0)], 1.0, 0).is_err());
    let tw = ThreeWave {
        frequencies: [0.0, 1.0],
        coupling: 1e-3,
        mismatch: 0.0,
        loss: [0.0; 3],
    };
    assert!(tw.propagate([real(1.0); 3], 1.0, 1).is_err());
}

#[test]
#[ignore]
fn explore_errors() {
    for n in [125, 250, 500, 1000, 2000, 4000] {
        println!("{n}: {:?}", crate::nonlinear::checks::coupled_errors(n));
    }
}
