use super::*;
use crate::portable;

#[test]
fn on_resonance_the_build_up_is_the_closed_form() {
    // r = 0.9, a = 0.98: |F|² = (1 − r²)/(1 − ra)²
    let w = RingWave::all_pass(0.19, c64::new(0.98, 0.0), 100.0).unwrap();
    let r: f64 = 0.9;
    assert!((w.build_up() - (1.0 - r * r) / portable::powi(1.0 - r * 0.98, 2)).abs() < 1e-12);
    assert!((w.loss() - (-2.0 * portable::ln(0.98f64) / 100.0)).abs() < 1e-15);
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
    let helt = portable::powi(gamma * p * 200e-6, 2) * f2 * f2 * f2 * f2;
    assert!((eta / helt - 1.0).abs() < 1e-12, "{eta} {helt}");
    let other = RingWave::all_pass(0.05, c64::new(1.0, 0.0), 300.0).unwrap();
    assert!(four_wave(&w, &w, &other, gamma, 0.0, p).is_err());
}

#[test]
fn an_add_drop_ring_without_its_drop_coupler_is_the_all_pass_ring() {
    let round = portable::complex::from_polar(0.97, 0.3);
    let all = RingWave::all_pass(0.05, round, 120.0).unwrap();
    let none = AddDropWave::new(0.05, 0.0, round, 120.0).unwrap();
    assert_eq!(all, none.wave);
    // F has both couplers in it, the round trip neither
    let both = AddDropWave::new(0.05, 0.04, round, 120.0).unwrap();
    assert_eq!(both.wave.round_trip, round);
    let (r1, r2) = (0.95f64.sqrt(), 0.96f64.sqrt());
    let f2 = 0.05 / (1.0 - r1 * r2 * round).norm_sqr();
    assert!((both.wave.build_up() / f2 - 1.0).abs() < 1e-12);
    assert!(AddDropWave::new(0.05, 1.5, round, 120.0).is_err());
    // with no drop couplers, the through ports are the all-pass ring's and nothing drops
    let h = AddDropWave::new(0.08, 0.0, portable::complex::from_polar(0.95, -0.2), 120.0).unwrap();
    let shg = second_harmonic_add_drop(&none, &h, 1e-4, 0.003, 1e-3).unwrap();
    let one = second_harmonic(&all, &h.wave, 1e-4, 0.003, 1e-3).unwrap();
    assert!((shg.through / one - 1.0).abs() < 1e-12, "{shg:?} {one}");
    assert_eq!(shg.drop, 0.0);
    let fwm = four_wave_add_drop(&none, &none, &h, 80.0, 2.0, 1e-3).unwrap();
    let one = four_wave(&all, &all, &h.wave, 80.0, 2.0, 1e-3).unwrap();
    assert!((fwm.through / one - 1.0).abs() < 1e-12, "{fwm:?} {one}");
}

#[test]
fn lossless_second_harmonic_is_the_build_ups_times_the_single_pass() {
    let w = RingWave::all_pass(0.02, c64::new(1.0, 0.0), 150.0).unwrap();
    let h = RingWave::all_pass(0.04, c64::new(1.0, 0.0), 150.0).unwrap();
    let k = 1e-4;
    let p2 = second_harmonic(&w, &h, k, 0.0, 1e-3).unwrap();
    let expected = h.build_up() * portable::powi(k * w.build_up() * 1e-3 * 150.0, 2);
    assert!((p2 / expected - 1.0).abs() < 1e-12);
}
