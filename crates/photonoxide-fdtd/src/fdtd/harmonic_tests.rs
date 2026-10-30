use num_complex::Complex64 as c64;

use super::harmonic::element;
use super::{Resonance, harmonic_inversion};
use crate::units::Frequency;

fn f(value: f64) -> Frequency {
    Frequency::natural(value).unwrap()
}

/// Σₖ dₖ e^(−iωₖ nτ), ωₖ = 2πfₖ − iγₖ.
fn signal(terms: &[(f64, f64, c64)], tau: f64, n: usize) -> Vec<c64> {
    (0..n)
        .map(|s| {
            let t = s as f64 * tau;
            terms
                .iter()
                .map(|&(freq, decay, d)| {
                    d * c64::new(-decay * t, -std::f64::consts::TAU * freq * t).exp()
                })
                .sum()
        })
        .collect()
}

#[test]
fn the_matrices_closed_form_is_the_double_sum() {
    let c: Vec<c64> = (0..40)
        .map(|n| c64::new((0.37 * n as f64).sin(), (0.11 * n as f64 * n as f64).cos()))
        .collect();
    let m = 12;
    let points = [
        c64::from_polar(1.0, -0.3),
        c64::from_polar(1.0, -0.31),
        c64::from_polar(0.97, -0.5),
        c64::from_polar(1.0, 2.0),
    ];
    for p in 0..3 {
        for &z in &points {
            for &w in &points {
                let direct: c64 = (0..=m)
                    .flat_map(|a| (0..=m).map(move |b| (a, b)))
                    .map(|(a, b)| c[a + b + p] * z.powi(-(a as i32)) * w.powi(-(b as i32)))
                    .sum();
                let closed = element(&c, m, p, z, w);
                assert!(
                    (closed - direct).norm() < 1e-11 * direct.norm().max(1.0),
                    "{p} {z} {w}: {closed} {direct}"
                );
            }
        }
    }
}

#[test]
fn two_terms_closer_than_the_fourier_resolution_are_recovered_to_round_off() {
    // 400 samples of 0.1: the transform's resolution 1/T = 0.025 c/µm, the two terms 0.01 apart,
    // and a third outside the window
    let (tau, n) = (0.1, 400);
    let terms = [
        (1.0, 0.01, c64::new(1.0, 0.5)),
        (1.01, 0.03, c64::new(-0.4, 0.8)),
        (1.6, 0.002, c64::new(2.0, 0.0)),
    ];
    let found: Vec<Resonance> = harmonic_inversion(&signal(&terms, tau, n), tau, f(0.9), f(1.1))
        .unwrap()
        .into_iter()
        .filter(|r| r.error < 1e-6)
        .collect();
    assert_eq!(found.len(), 2, "{found:?}");
    for (r, &(freq, decay, d)) in found.iter().zip(&terms) {
        assert!((r.frequency - freq).abs() < 1e-11, "{r:?}");
        assert!((r.decay - decay).abs() < 1e-10, "{r:?}");
        assert!((r.amplitude - d).norm() < 1e-9 * d.norm(), "{r:?}");
        assert!(r.error < 1e-9, "{r:?}");
    }
}

#[test]
fn a_real_signals_terms_are_half_its_amplitude_and_q_is_omega_over_twice_the_decay() {
    // A e^(−γt) cos(ωt − φ) = ½A e^(iφ) e^(−iωt) e^(−γt) + its conjugate
    let (tau, n) = (0.05, 600);
    let (amplitude, freq, decay, phase) = (3.0, 0.8, 0.004, 0.7);
    let real: Vec<c64> = (0..n)
        .map(|s| {
            let t = s as f64 * tau;
            let v =
                amplitude * (-decay * t).exp() * (std::f64::consts::TAU * freq * t - phase).cos();
            c64::new(v, 0.0)
        })
        .collect();
    let found: Vec<Resonance> = harmonic_inversion(&real, tau, f(0.5), f(1.2))
        .unwrap()
        .into_iter()
        .filter(|r| r.error < 1e-6)
        .collect();
    assert_eq!(found.len(), 1, "{found:?}");
    let r = found[0];
    assert!((r.frequency - freq).abs() < 1e-11, "{r:?}");
    assert!((r.decay - decay).abs() < 1e-11, "{r:?}");
    let expected = 0.5 * amplitude * c64::from_polar(1.0, phase);
    assert!((r.amplitude - expected).norm() < 1e-9, "{r:?}");
    let q = std::f64::consts::TAU * freq / (2.0 * decay);
    assert!((r.q() - q).abs() < 1e-7 * q, "{} {q}", r.q());
}

#[test]
fn harmonic_inversion_refuses_a_window_beyond_nyquist_and_a_short_signal() {
    let s = vec![c64::new(1.0, 0.0); 100];
    assert!(harmonic_inversion(&s, 0.1, f(1.0), f(5.0)).is_err());
    assert!(harmonic_inversion(&s, 0.1, f(2.0), f(1.0)).is_err());
    assert!(harmonic_inversion(&s[..5], 0.1, f(1.0), f(2.0)).is_err());
    assert!(harmonic_inversion(&s, 0.0, f(1.0), f(2.0)).is_err());
}
