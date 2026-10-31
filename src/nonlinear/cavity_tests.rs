use super::*;

#[test]
fn on_resonance_the_build_up_is_the_closed_form() {
    // r = 0.9, a = 0.98: |F|² = (1 − r²)/(1 − ra)²
    let w = RingWave::all_pass(0.19, c64::new(0.98, 0.0), 100.0).unwrap();
    let r: f64 = 0.9;
    assert!((w.build_up() - (1.0 - r * r) / (1.0 - r * 0.98).powi(2)).abs() < 1e-12);
    assert!((w.loss() - (-2.0 * 0.98f64.ln() / 100.0)).abs() < 1e-15);
    assert_eq!(w.detuning(), 0.0);
    assert!(RingWave::all_pass(1.5, c64::new(0.98, 0.0), 100.0).is_err());
    assert!(RingWave::all_pass(0.1, c64::new(1.1, 0.0), 100.0).is_err());
}

#[test]
fn lossless_and_on_resonance_four_wave_mixing_is_helts_equation_5() {
    let w = RingWave::all_pass(0.05, c64::new(1.0, 0.0), 200.0).unwrap();
    let (gamma, p) = (1.5, 0.01);
    let eta = four_wave(&w, &w, &w, gamma, 0.0, p).unwrap();
    let f2 = w.build_up();
    let helt = (gamma * p * 200e-6).powi(2) * f2 * f2 * f2 * f2;
    assert!((eta / helt - 1.0).abs() < 1e-12, "{eta} {helt}");
    let other = RingWave::all_pass(0.05, c64::new(1.0, 0.0), 300.0).unwrap();
    assert!(four_wave(&w, &w, &other, gamma, 0.0, p).is_err());
}

#[test]
fn lossless_second_harmonic_is_the_build_ups_times_the_single_pass() {
    let w = RingWave::all_pass(0.02, c64::new(1.0, 0.0), 150.0).unwrap();
    let h = RingWave::all_pass(0.04, c64::new(1.0, 0.0), 150.0).unwrap();
    let k = 1e-4;
    let p2 = second_harmonic(&w, &h, k, 0.0, 1e-3).unwrap();
    let expected = h.build_up() * (k * w.build_up() * 1e-3 * 150.0).powi(2);
    assert!((p2 / expected - 1.0).abs() < 1e-12);
}
