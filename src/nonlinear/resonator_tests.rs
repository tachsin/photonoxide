use super::*;
use crate::nonlinear::kerr_checks::coen_2012_ring;

#[test]
fn the_homogeneous_states_solve_the_cubic() {
    for (z, f2) in [(1.0, 2.0), (3.0, 4.0), (3.0, 6.0), (5.0, 0.3), (-1.0, 1.0)] {
        let r = Resonator::quadratic(z, f64::sqrt(f2), 0.02, 8).unwrap();
        let roots = r.homogeneous();
        assert!(!roots.is_empty());
        for &x in &roots {
            assert!(
                (x * (1.0 + (x - z).powi(2)) / f2 - 1.0).abs() < 1e-13,
                "{z} {f2} {x}"
            );
            // and its field is stationary: −(1 + iζ₀)ψ + i|ψ|²ψ + f = 0
            let psi = r.homogeneous_field(x);
            assert!((psi.norm_sqr() - x).abs() < 1e-12);
            let rate = -c64::new(1.0, z) * psi + c64::new(0.0, x) * psi + f2.sqrt();
            assert!(rate.norm() < 1e-12);
        }
    }
    // three states between the turning points' drives, at ζ₀ = 3
    let (lo, hi) = bistable_range(3.0).unwrap();
    let (f_hi, f_lo) = (drive_for(3.0, lo), drive_for(3.0, hi));
    assert!(f_lo < f_hi);
    let mid = 0.5 * (f_lo + f_hi);
    assert_eq!(homogeneous_powers(3.0, mid).len(), 3);
    assert_eq!(homogeneous_powers(3.0, 1.1 * f_hi).len(), 1);
    // the turning points are where df²/dX = 1 + (X − ζ₀)(3X − ζ₀) = 0 (Lugiato and Lefever, p. 2210)
    for x in [lo, hi] {
        assert!((1.0 + (x - 3.0) * (3.0 * x - 3.0)).abs() < 1e-13);
    }
    // monostable at or below √3
    assert!(bistable_range(3f64.sqrt()).is_none());
}

#[test]
fn the_instability_follows_lugiato_and_lefever() {
    // below |E_s|² = 1 the homogeneous state is stable in every mode (their Eq. (6))
    for a in [0.0, 0.5, 1.0, 2.0] {
        assert!(instability_rate(1.0, 0.9, a) < 0.0);
    }
    // above it, unstable between a⁻ and a⁺ = η(2X − θ) ∓ (X² − 1)^½ (their Eq. (5))
    let (z, x): (f64, f64) = (1.0, 1.5);
    let (am, ap) = (
        2.0 * x - z - (x * x - 1.0).sqrt(),
        2.0 * x - z + (x * x - 1.0).sqrt(),
    );
    assert!(instability_rate(z, x, 0.5 * (am + ap)) > 0.0);
    assert!(instability_rate(z, x, 0.99 * am) < 0.0);
    assert!(instability_rate(z, x, 1.01 * ap) < 0.0);
    assert!(instability_rate(z, x, am).abs() < 1e-12);
}

#[test]
fn the_split_step_keeps_a_settled_state_and_takes_the_drive_exactly() {
    // without the Kerr term a homogeneous field relaxes to f/(1 + iζ₀), exactly in the linear
    // step: start there, with a power so small the Kerr term is round-off
    let r = Resonator::quadratic(2.0, 1e-8, 0.01, 8).unwrap();
    let psi = r.homogeneous_field(0.0);
    let end = r.evolve(&[psi; 8], 3.0, 7).unwrap();
    for v in end {
        assert!((v - psi).norm() < 1e-20);
    }
    assert!(r.evolve(&[psi; 4], 1.0, 1).is_err());
    assert!(Resonator::quadratic(1.0, 1.0, 0.1, 12).is_err());
    assert!(Resonator::quadratic(1.0, -1.0, 0.1, 16).is_err());
}

#[test]
fn a_sech_has_its_width() {
    let n = 256;
    let w = 0.1;
    let field: Vec<c64> = (0..n)
        .map(|j| {
            let phi = 2.0 * PI * (j as f64 - 100.3) / n as f64;
            c64::new(1.0 / (phi / w).cosh(), 0.0)
        })
        .collect();
    let fwhm = 2.0 * w * (2f64.sqrt()).acosh();
    assert!((peak_width(&field).unwrap() / fwhm - 1.0).abs() < 1e-4);
}

#[test]
fn the_approximate_soliton_and_its_limit() {
    let r = Resonator::quadratic(6.0, 2.5, 0.002, 64).unwrap();
    let s = r.soliton(0.002).unwrap();
    assert!((s.amplitude - 12f64.sqrt()).abs() < 1e-14);
    assert!((s.width - (0.002f64 / 6.0).sqrt()).abs() < 1e-15);
    assert!((s.cos_phase - 48f64.sqrt() / (PI * 2.5)).abs() < 1e-15);
    // beyond π²f²/8 there is none
    assert!(
        r.with_detuning(1.01 * max_detuning(2.5))
            .soliton(0.002)
            .is_none()
    );
    let field = r.soliton_field(&s);
    assert!(
        (field[32]
            - s.background
            - c64::new(s.cos_phase, (1.0 - s.cos_phase.powi(2)).sqrt()) * s.amplitude)
            .norm()
            < 1e-12
    );
    // Herr et al.'s Eq. (2) and the approximation's shortest width at critical coupling
    let ring = Ring {
        coupling: 1e-3,
        loss: 1e-3,
        ..coen_2012_ring().unwrap()
    };
    let herr = herr_minimum_width(
        ring.dispersion.beta(2),
        ring.gamma,
        ring.finesse(),
        ring.pump,
    );
    let ours = ring.minimum_soliton_width();
    assert!((ours / herr - 2.0 * (2f64.sqrt()).acosh() / PI.sqrt()).abs() < 1e-12);
}

#[test]
fn coens_ring_normalizes_as_the_paper_has_it() {
    let ring = coen_2012_ring().unwrap();
    assert!((ring.normalized_detuning() - 0.0012 / 1.75e-5).abs() < 1e-12);
    // f² = θγLP_in/α³
    let f2 = 1.75e-5 * 0.032e-3 * 0.0119 * 0.0556 / 1.75e-5f64.powi(3);
    assert!((ring.normalized_drive().powi(2) / f2 - 1.0).abs() < 1e-14);
    // its loaded Q, ω t_R/2α, is the caption's 1.90 × 10⁹ at 1.55 µm
    let q = angular_frequency(1.55) * ring.roundtrip / (2.0 * ring.loss);
    assert!((q / 1.9e9 - 1.0).abs() < 0.005, "{q}");
    // δ_μ = d₂μ² for β₂ alone
    let r = ring.resonator(16).unwrap();
    let d2 = ring.d2();
    for (m, d) in r.modes().iter().zip(&r.dispersion) {
        assert!((d - d2 * (m * m) as f64).abs() < 1e-12 * d2 * (m * m).max(1) as f64);
    }
}

#[test]
fn coens_soliton_holds_on_twice_the_modes() {
    // the case's 512 modes against 1024: the FWHM (fs) moves by much less than its rule's
    // ±50 fs (the steps' own part, 0.006 fs, the case notes)
    let ring = coen_2012_ring().unwrap();
    let width = |points: usize, step: f64| {
        let r = ring.resonator(points).unwrap();
        let s = r.soliton(ring.d2()).unwrap();
        let (end, _) = r
            .settle(&r.soliton_field(&s), step, 0.5, 1e-10, 200.0)
            .unwrap();
        ring.time_of(peak_width(&end).unwrap()) * 1e3
    };
    // on 1024 modes the largest δ_μ is four times larger, and the step a quarter, to keep δ_μh
    // below π (the split-step's own instability above it)
    let (a, b) = (width(512, 4e-4), width(1024, 1e-4));
    assert!((a - b).abs() < 0.01, "{a} {b}");
}

use super::super::pulse::angular_frequency;
