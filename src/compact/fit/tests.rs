use std::f64::consts::TAU;

use super::*;
use crate::compact::checks::*;

/// Every pole of a model is stable.
fn stable(model: &Rational) -> bool {
    model.poles.iter().all(|a| a.re < 0.0)
}

// Gustavsen and Semlyen, Section 4.2 and Table 3
#[test]
fn the_papers_response_is_recovered_in_one_relocation() {
    let model = paper_fit();
    let (poles, residues, surplus) = paper_recovery(&model);
    // the paper: an RMS error of 3.8e-12, pole errors to 4e-10 Hz, the two surplus poles'
    // residues below 4.3e-7 Hz (2.7e-6 in s)
    assert!(model.error.rms < 1e-10, "{:?}", model.error);
    assert!(poles < 1e-10, "{poles}");
    assert!(residues < 1e-9, "{residues}");
    assert!(surplus < 1e-4, "{surplus}");
    assert_eq!(model.iterations, 1);
    assert!(stable(&model));
    // a real model: real poles and conjugate pairs, real d and h
    for a in &model.poles {
        assert!(a.im == 0.0 || model.poles.contains(&a.conj()));
    }
    assert!((model.constant[0] - 0.2).norm() < 1e-9);
    assert!((model.proportional[0] - 2e-5).norm() < 1e-14);
}

// Section 4.3 and Table 4: real starting poles fail at first, and iterating recovers
#[test]
fn real_starting_poles_converge_by_iteration() {
    let (s, f) = paper_samples();
    let start: Vec<c64> = (0..20)
        .map(|k| c64::new(-TAU * (1.0 + (1e5 - 1.0) * k as f64 / 19.0), 0.0))
        .collect();
    let options = |iterations| Options {
        iterations,
        tolerance: 0.0,
        symmetry: Symmetry::Real,
        proportional: true,
        starting_poles: Some(start.clone()),
        ..Options::new(20)
    };
    // the paper: 7.1, then 1.0e-11 and 4.2e-13; here 20, then 6e-12 and 2e-12
    let first = vector_fit(&s, std::slice::from_ref(&f), &options(1)).unwrap();
    assert!(first.error.rms > 1.0, "{:?}", first.error);
    let third = vector_fit(&s, &[f], &options(3)).unwrap();
    assert!(third.error.rms < 1e-10, "{:?}", third.error);
    assert!(stable(&third));
}

#[test]
fn a_complex_model_fits_the_papers_response_too() {
    let (s, f) = paper_samples();
    let model = vector_fit(
        &s,
        &[f],
        &Options {
            proportional: true,
            ..Options::new(20)
        },
    )
    .unwrap();
    assert!(model.error.rms < 1e-9, "{:?}", model.error);
    assert!(stable(&model));
}

#[test]
fn unstable_poles_are_flipped() {
    let zeros = vec![c64::new(0.5, 3.0), c64::new(-0.2, -1.0)];
    let poles: Vec<c64> = stable_poles(zeros, Symmetry::Complex)
        .iter()
        .map(|p| p.value())
        .collect();
    assert_eq!(poles, vec![c64::new(-0.5, 3.0), c64::new(-0.2, -1.0)]);
    let real = stable_poles(
        vec![c64::new(2.0, 0.0), c64::new(1.0, 4.0), c64::new(1.0, -4.0)],
        Symmetry::Real,
    );
    assert_eq!(
        real,
        vec![Pole::Real(-2.0), Pole::Pair(c64::new(-1.0, 4.0))]
    );
    // a response with a pole in the right half plane can't be fitted by stable poles, but the
    // fit stays stable
    let s: Vec<c64> = (0..60)
        .map(|k| c64::new(0.0, 0.1 * k as f64 + 0.1))
        .collect();
    let f: Vec<c64> = s.iter().map(|&z| 1.0 / (z - c64::new(0.3, 2.0))).collect();
    let model = vector_fit(&s, &[f], &Options::new(4)).unwrap();
    assert!(stable(&model));
}

#[test]
fn a_known_delay_is_taken_out_exactly() {
    // f = e^(−sτ) (r/(s − a) + d) on an optical band, s = −iω
    let (a, r, d, tau) = (
        c64::new(-0.002, -4.05),
        c64::new(-0.003, 0.001),
        c64::new(0.3, 0.1),
        12.5,
    );
    let s: Vec<c64> = (0..201)
        .map(|k| c64::new(0.0, -(3.95 + 0.2 * k as f64 / 200.0)))
        .collect();
    let f: Vec<c64> = s
        .iter()
        .map(|&z| (r / (z - a) + d) * (-z * tau).exp())
        .collect();
    let fixed = vector_fit(
        &s,
        std::slice::from_ref(&f),
        &Options {
            delay: Delay::Fixed(tau),
            ..Options::new(2)
        },
    )
    .unwrap();
    assert!(fixed.error.max < 1e-12, "{:?}", fixed.error);
    let near = fixed
        .poles
        .iter()
        .map(|p| (p - a).norm())
        .fold(f64::INFINITY, f64::min);
    assert!(near < 1e-12 * a.norm(), "{near}");
    // the phase slope of a pure delay is the delay
    let pure: Vec<c64> = s.iter().map(|&z| (-z * tau).exp() * 0.7).collect();
    assert!((estimate_delay(&s, &pure) - tau).abs() < 1e-9 * tau);
}

#[test]
fn the_starting_poles_follow_the_papers_recipe() {
    // Eqs. 9-10 over 1 Hz to 100 kHz: Table 2
    let (s, _) = paper_samples();
    let start = starting_poles(&s, 20, Symmetry::Real);
    assert_eq!(start.len(), 20);
    // β from |Im s| at 1 Hz to 100 kHz, α = β/100 (here capped at half the spacing, which is
    // larger than β/100 over this band)
    assert!((start[0] - c64::new(-TAU / 100.0, TAU)).norm() < 1e-9);
    assert_eq!(start[1], start[0].conj());
    assert!((start[18] - c64::new(-TAU * 1e3, TAU * 1e5)).norm() < 1e-6);
    // far from zero frequency the real parts are half the spacing at most
    let s: Vec<c64> = (0..100)
        .map(|k| c64::new(0.0, -(4.0 + 0.001 * k as f64)))
        .collect();
    let start = starting_poles(&s, 10, Symmetry::Complex);
    assert!(
        start
            .iter()
            .all(|a| a.re < 0.0 && -a.re <= 0.5 * 0.099 / 9.0 + 1e-15 && a.im <= -4.0 + 1e-12)
    );
}

#[test]
fn bad_inputs_are_errors() {
    let s: Vec<c64> = (1..=10).map(|k| c64::new(0.0, k as f64)).collect();
    let f = vec![c64::new(1.0, 0.0); 10];
    let fit = |s: &[c64], f: &[Vec<c64>], o: &Options| {
        vector_fit(s, f, o).map(|_| ()).unwrap_err().to_string()
    };
    assert!(fit(&s, &[], &Options::new(2)).contains("at least one response"));
    assert!(fit(&s, &[f[..9].to_vec()], &Options::new(2)).contains("10 samples"));
    assert!(fit(&s, std::slice::from_ref(&f), &Options::new(0)).contains("one pole"));
    let real = Options {
        symmetry: Symmetry::Real,
        ..Options::new(3)
    };
    assert!(fit(&s, std::slice::from_ref(&f), &real).contains("pairs"));
    assert!(fit(&s, std::slice::from_ref(&f), &Options::new(12)).contains("more samples"));
    // every sample at one point: ten samples, and one thing known
    let same = vec![c64::new(0.0, 2.0); 10];
    let said = fit(&same, std::slice::from_ref(&f), &Options::new(2));
    assert!(
        said.contains("10 samples at 1 distinct points can't determine"),
        "{said}"
    );
    // three points, each sampled several times, are three: not enough for 4 poles, enough for 1
    let thrice: Vec<c64> = (0..9)
        .map(|k| c64::new(0.0, 1.0 + (k % 3) as f64))
        .collect();
    let g = vec![c64::new(1.0, 0.0); 9];
    let said = fit(&thrice, std::slice::from_ref(&g), &Options::new(4));
    assert!(said.contains("9 samples at 3 distinct points"), "{said}");
    assert!(vector_fit(&thrice, std::slice::from_ref(&g), &Options::new(1)).is_ok());
    let mut nan = f.clone();
    nan[3] = c64::new(f64::NAN, 0.0);
    assert!(fit(&s, &[nan], &Options::new(2)).contains("finite"));
    let wrong = Options {
        starting_poles: Some(vec![c64::new(-1.0, 1.0)]),
        ..Options::new(2)
    };
    assert!(fit(&s, std::slice::from_ref(&f), &wrong).contains("starting poles"));
    let unpaired = Options {
        symmetry: Symmetry::Real,
        starting_poles: Some(vec![c64::new(-1.0, 1.0), c64::new(-1.0, 2.0)]),
        ..Options::new(2)
    };
    assert!(fit(&s, &[f], &unpaired).contains("conjugate"));
}

// Deschrijver et al. 2008, Eqs. 8, 10 and 11: the same least squares, reduced
#[test]
fn eliminating_each_responses_unknowns_is_the_full_least_squares() {
    let d = crate::compact::checks::fast_vf_difference();

    assert!(d < 1e-8, "{d}");
}
