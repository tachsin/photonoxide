use faer::Mat;
use num_complex::Complex64 as c64;

use super::aaa::jacobi_for_tests;
use super::checks;
use super::*;

fn re(v: f64) -> c64 {
    c64::new(v, 0.0)
}

#[test]
fn jacobi_matches_the_svd() {
    let a = Mat::<c64>::from_fn(7, 4, |i, j| {
        c64::new(
            (i as f64 + 1.0).sin() * (j as f64 + 2.0),
            (i * j) as f64 / 7.0 - 0.3,
        )
    });
    let (s, v) = jacobi_for_tests(&a);
    let svd = a.thin_svd().expect("an SVD");
    let mut ours = s.clone();
    ours.sort_by(|a, b| b.total_cmp(a));
    for (k, &value) in ours.iter().enumerate() {
        let theirs = svd.S().column_vector()[k].re;
        assert!((value - theirs).abs() < 1e-12 * ours[0], "{value} {theirs}");
    }
    // ‖A v_k‖ = s_k for each of Jacobi's right singular vectors
    for k in 0..4 {
        let norm = (0..7)
            .map(|i| {
                (0..4)
                    .map(|j| a[(i, j)] * v[(j, k)])
                    .sum::<c64>()
                    .norm_sqr()
            })
            .sum::<f64>()
            .sqrt();
        assert!((norm - s[k]).abs() < 1e-12 * ours[0]);
    }
}

#[test]
fn aaa_interpolates_at_its_support_points() {
    let x: Vec<f64> = (0..40).map(|k| f64::from(k) / 39.0).collect();
    let f: Vec<c64> = x.iter().map(|&x| re((3.0 * x).exp())).collect();
    let r = Rational::aaa(&x, &f, DEFAULT_TOLERANCE).unwrap();
    for (&z, k) in r.support().iter().zip(0..) {
        let i = x.iter().position(|&v| v == z).unwrap();
        assert_eq!(r.eval(z), f[i], "support point {k}");
    }
    // e^(3x) on [0, 1] to near round-off, with few terms
    let worst = (0..1000)
        .map(|k| {
            let x = (f64::from(k) + 0.5) / 1000.0;
            (r.eval(x) - re((3.0 * x).exp())).norm()
        })
        .fold(0.0, f64::max);
    assert!(worst < 1e-11, "{worst}");
    assert!(r.len() < 15, "{}", r.len());
    let norm: f64 = r.weights().iter().map(|w| w.norm_sqr()).sum();
    assert!((norm - 1.0).abs() < 1e-12);
}

#[test]
fn aaa_finds_a_rationals_poles() {
    let (value, pole, terms) = checks::known_rational();
    assert!(value < 1e-12, "{value}");
    assert!(pole < 1e-10, "{pole}");
    assert_eq!(terms, 5);
}

#[test]
fn aaa_refuses_bad_samples() {
    let one = [re(1.0)];
    assert!(Rational::aaa(&[], &[], 1e-13).is_err());
    assert!(Rational::aaa(&[0.0, 1.0], &one, 1e-13).is_err());
    assert!(Rational::aaa(&[0.0, 0.0], &[re(1.0), re(2.0)], 1e-13).is_err());
    assert!(Rational::aaa(&[f64::NAN], &one, 1e-13).is_err());
    assert!(Rational::aaa(&[0.0], &[re(f64::INFINITY)], 1e-13).is_err());
    assert!(Rational::aaa(&[0.0], &one, -1.0).is_err());
    // one sample: the constant
    let r = Rational::aaa(&[0.5], &one, 1e-13).unwrap();
    assert_eq!(r.eval(3.0), re(1.0));
    assert!(r.poles().is_empty());
}

#[test]
fn a_sweep_refuses_bad_options() {
    let f = |x: f64| Ok(re(x));
    assert!(Sweep::run(f, 1.0, 0.0, &Options::default()).is_err());
    assert!(Sweep::run(f, 0.0, f64::INFINITY, &Options::default()).is_err());
    assert!(Sweep::run(f, 0.0, 1.0, &Options::new(1e-14)).is_err());
    assert!(Sweep::run(f, 0.0, 1.0, &Options::default().with_initial(2)).is_err());
    assert!(Sweep::run(f, 0.0, 1.0, &Options::default().with_points(4)).is_err());
    assert!(Sweep::run(f, 0.0, 1.0, &Options::default().with_max_samples(4)).is_err());
    let nan = |_: f64| Ok(re(f64::NAN));
    assert!(Sweep::run(nan, 0.0, 1.0, &Options::default()).is_err());
}

#[test]
fn a_sweep_passes_the_solvers_error_on() {
    let failing = |x: f64| {
        if x > 0.5 {
            Err(crate::Error::invalid("test", "no mode"))
        } else {
            Ok(re(x))
        }
    };
    assert!(Sweep::run(failing, 0.0, 1.0, &Options::default()).is_err());
}

#[test]
fn a_sweep_of_a_rational_function_stops_at_the_floor() {
    // 1/(x − p) is of type (0, 1): a handful of samples hold it
    let p = c64::new(0.2, 0.3);
    let sweep = Sweep::run(|x| Ok(1.0 / (re(x) - p)), -1.0, 1.0, &Options::new(1e-12)).unwrap();
    assert!(sweep.converged);
    assert!(sweep.calls() <= 8, "{}", sweep.calls());
    assert!(sweep.relative_error() <= 1e-12);
    // the samples are in order and the history ends at the estimate
    assert!(sweep.samples.windows(2).all(|w| w[0].0 < w[1].0));
    assert_eq!(*sweep.history.last().unwrap(), sweep.relative_error());
    assert_eq!(sweep.range, (-1.0, 1.0));
}

#[test]
fn a_sweep_stops_unconverged_at_its_limit() {
    let sweep = Sweep::run(
        |x| Ok(c64::from_polar(1.0, 40.0 * x)),
        -1.0,
        1.0,
        &Options::new(1e-10).with_max_samples(8),
    )
    .unwrap();
    assert!(!sweep.converged);
    assert_eq!(sweep.calls(), 8);
    assert!(sweep.relative_error() > 1e-10);
}

#[test]
fn the_estimate_bounds_the_error_on_a_few_test_functions() {
    // the validation case's functions, one of each kind, at two tolerances
    for (name, f) in checks::test_functions(1) {
        for tolerance in [1e-5, 1e-9] {
            let (true_error, estimate, _) = checks::true_and_estimated(&*f, tolerance);
            assert!(estimate <= tolerance, "{name}: {estimate:e}");
            assert!(
                true_error <= estimate,
                "{name}: {true_error:e} > {estimate:e}"
            );
        }
    }
}

#[test]
fn a_zero_function_is_converged_at_once() {
    let sweep = Sweep::run(|_| Ok(re(0.0)), 0.0, 1.0, &Options::default()).unwrap();
    assert!(sweep.converged);
    assert_eq!(sweep.calls(), 5);
    assert_eq!(sweep.eval(0.3), re(0.0));
}

#[test]
fn a_sweep_of_a_ring_needs_far_fewer_solves_than_a_uniform_cubic_grid() {
    // docs/methods/sweep.md's ring: 36 samples where a piecewise cubic needs 2660
    let ring = |x: f64| {
        let e = c64::from_polar(0.97, 0.7 + 12.0 * x);
        (0.95 - e) / (1.0 - 0.95 * e)
    };
    let sweep = Sweep::run(|x| Ok(ring(x)), -1.0, 1.0, &Options::new(1e-4)).unwrap();
    assert!(sweep.converged);
    let cubic =
        checks::uniform_calls(&ring, (-1.0, 1.0), 1e-4, checks::Uniform::Cubic, 20000).unwrap();
    assert!(10 * sweep.calls() < cubic, "{} {cubic}", sweep.calls());
    let linear = checks::uniform_error(&ring, (-1.0, 1.0), sweep.calls(), checks::Uniform::Linear);
    assert!(linear > 1e-2, "{linear}");
    // AAA on a uniform grid gets there with fewer (20), but with no estimate to say so
    let aaa = checks::uniform_calls(&ring, (-1.0, 1.0), 1e-4, checks::Uniform::Aaa, 400).unwrap();
    assert!(aaa < sweep.calls(), "{aaa}");
}
