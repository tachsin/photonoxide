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

/// A smooth dispersion, 2 + 0.3 sin 1.5x + 0.1x², with noise of relative size `eta`: each
/// sample's own, from its point's bits, so a sweep's samples don't depend on their order.
fn noisy(x: f64, eta: f64) -> c64 {
    let bits = x
        .to_bits()
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let unit = (bits >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0;
    re((2.0 + 0.3 * (1.5 * x).sin() + 0.1 * x * x) * (1.0 + eta * unit))
}

#[test]
#[ignore = "the sweep's floor on noisy samples (src/sweep.rs, What it can't see): in release, prints"]
fn the_sweeps_floor_on_noisy_samples_study() {
    // the noiseless function reaches 1e-10 in 28 samples; with noise above AAA's own 1e-13 the
    // estimate stops far above the noise and grows past it
    for eta in [0.0, 1e-12, 1e-10, 1e-8, 1e-6, 1e-4] {
        let mut line = Vec::new();
        let mut converged = Vec::new();
        for tolerance in [1e-2, 1e-3, 1e-4, 1e-6, 1e-8, 1e-10] {
            let sweep = Sweep::run(
                |x| Ok(noisy(x, eta)),
                -1.0,
                1.0,
                &Options::new(tolerance).with_max_samples(60),
            )
            .unwrap();
            let least = sweep.history.iter().copied().fold(f64::INFINITY, f64::min);
            line.push(format!(
                "{tolerance:.0e}: {} in {} (least {least:.0e})",
                if sweep.converged { "yes" } else { "no" },
                sweep.calls()
            ));
            converged.push(sweep.converged);
        }
        eprintln!("noise {eta:.0e}: {}", line.join(", "));
        if eta == 0.0 {
            assert!(converged.iter().all(|&c| c));
        } else {
            // the estimate stops at 7e-6, 1e-4, 9e-3, 6e-5 and 3e-3: 30 to 7e6 times the noise
            assert!(!converged[5], "noise {eta:e}");
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

fn um(l: f64) -> crate::units::Wavelength {
    crate::units::Wavelength::um(l).unwrap()
}

/// `sweep/mode-crossing`'s strip on oxide (1.444) with `top` above and beside it: unless `top`
/// is 1.444 the strip is no longer symmetric above and below, its TE1-like and TM0-like modes
/// couple, and they anticross (docs/methods/sweep.md, Anticrossings).
fn clad_strip(top: f64) -> crate::Result<crate::mode::vector::CrossSection> {
    use crate::mode::vector::{CrossSection, Permittivity};
    CrossSection::uniform((-1.2, 1.2, 120), (-0.8, 0.8, 80), move |x, y| {
        let n: f64 = if x.abs() < 0.3 && y.abs() < 0.11 {
            3.473
        } else if y < -0.11 {
            1.444
        } else {
            top
        };
        Permittivity::isotropic(re(n * n))
    })
}

/// The effective index of the strip's second TE-like mode (TE1) at `l` µm, among its four
/// highest, told apart by polarization.
fn te1_near(top: f64, l: f64) -> f64 {
    let modes = crate::mode::vector::modes(&clad_strip(top).unwrap(), um(l), 4, None).unwrap();
    let te1 = modes
        .iter()
        .filter(|m| m.te_fraction() > 0.5)
        .nth(1)
        .unwrap();
    te1.effective_index().re
}

#[test]
fn a_branch_records_how_it_chose_each_mode() {
    // the symmetric strip of sweep/mode-crossing: TE1 and TM0 cross without coupling, so every
    // step is a clear choice
    let mut branch = Branch::new(checks::crossing_strip, um(1.3), Some(2.08), 3).unwrap();
    assert!(branch.steps().is_empty() && branch.least_margin().is_none());
    for l in [1.6, 1.4, 1.5] {
        branch.mode(um(l)).unwrap();
    }
    // found before: no new step
    branch.mode(um(1.4)).unwrap();
    let steps = branch.steps();
    let at: Vec<f64> = steps.iter().map(|s| s.0).collect();
    assert_eq!(at, [1.4, 1.5, 1.6]);
    // 1.6 from 1.3, the only one found then; 1.4 from 1.3, nearer than 1.6; 1.5 from 1.4 or 1.6
    assert_eq!(steps[2].1.from, 1.3);
    assert_eq!(steps[0].1.from, 1.3);
    assert!(steps[1].1.from == 1.4 || steps[1].1.from == 1.6);
    for (l, s) in steps {
        assert!(s.overlap > 0.9 && s.margin() > 0.8, "{l}: {s:?}");
        assert!(
            s.margin() <= s.overlap && s.runner_up.is_some(),
            "{l}: {s:?}"
        );
    }
    let one = BranchStep {
        from: 1.3,
        overlap: 0.9,
        runner_up: None,
    };
    assert_eq!(one.margin(), 0.9);
}

#[test]
fn a_branch_shows_an_anticrossing_in_its_margins() {
    // with 1.40 above the strip, TE1 and TM0 anticross near 1.44 µm (docs/methods/sweep.md):
    // a step of 50 nm into it finds two modes almost equally alike (0.761 and 0.653, measured),
    // a step of 2.5 nm before it one clear choice (0.999 or more, measured)
    let near = te1_near(1.4, 1.35);
    let mut coarse = Branch::new(|_| clad_strip(1.4), um(1.35), Some(near), 3).unwrap();
    coarse.mode(um(1.4)).unwrap();
    coarse.mode(um(1.45)).unwrap();
    let (l, s) = coarse.least_margin().unwrap();
    assert!(l == 1.45 && s.from == 1.4, "{l}: {s:?}");
    assert!(s.margin() < 0.2 && s.overlap < 0.8, "{s:?}");
    let mut fine = Branch::new(|_| clad_strip(1.4), um(1.35), Some(near), 3).unwrap();
    fine.mode(um(1.3525)).unwrap();
    let (_, s) = fine.least_margin().unwrap();
    assert!(s.margin() > 0.9, "{s:?}");
}

#[test]
#[ignore = "a branch at an anticrossing (docs/methods/sweep.md, Anticrossings): in release, prints"]
fn a_branch_at_an_anticrossing_study() {
    use crate::mode::vector::{self, VectorMode};
    let show = |m: &VectorMode| {
        format!(
            "n_eff {:.5}, TE fraction {:.2}",
            m.effective_index().re,
            m.te_fraction()
        )
    };
    let (a, b) = (1.3, 1.7);
    for top in [1.4, 1.0] {
        // the anticrossing's narrowest gap between the second and third modes by index
        let gap = (0..=28)
            .map(|k| {
                let l = 1.40 + 0.0025 * f64::from(k);
                let modes = vector::modes(&clad_strip(top).unwrap(), um(l), 3, None).unwrap();
                (
                    modes[1].effective_index().re - modes[2].effective_index().re,
                    l,
                )
            })
            .min_by(|x, y| x.0.total_cmp(&y.0))
            .unwrap();
        let end = vector::modes(&clad_strip(top).unwrap(), um(b), 3, None).unwrap();
        eprintln!(
            "top {top}: least gap {:.4} at {:.4} µm (of 1.40 to 1.47); at {b} µm the second mode by index {}, the third {}",
            gap.0,
            gap.1,
            show(&end[1]),
            show(&end[2])
        );
        let near = te1_near(top, a);
        for step in [0.0025, 0.05, 0.1, 0.15] {
            let mut branch = Branch::new(|_| clad_strip(top), um(a), Some(near), 3).unwrap();
            let n = ((b - a) / step).round() as u32;
            for k in 1..=n {
                branch
                    .mode(um(a + (b - a) * f64::from(k) / f64::from(n)))
                    .unwrap();
            }
            let (l, s) = branch.least_margin().unwrap();
            eprintln!(
                "  steps of {step} µm: at {b} µm {}; least margin {:.3} at {l:.4} from {:.4} ({:.3} against {:.3})",
                show(&branch.found().last().unwrap().1),
                s.margin(),
                s.from,
                s.overlap,
                s.runner_up.unwrap_or(0.0)
            );
        }
        let tolerance = 1e-8;
        {
            let mut branch = Branch::new(|_| clad_strip(top), um(a), Some(near), 3).unwrap();
            let mut failed = None;
            let sweep = Sweep::run(
                |l| {
                    let n = branch.effective_index(um(l));
                    if n.is_err() {
                        failed = Some(l);
                    }
                    Ok(re(n?.re))
                },
                a,
                b,
                &Options::new(tolerance),
            );
            let (l, s) = branch.least_margin().unwrap();
            let last = branch
                .found()
                .iter()
                .find(|f| f.0 == b)
                .map_or("none".to_string(), |f| show(&f.1));
            let near_gap = branch
                .found()
                .iter()
                .filter(|f| (f.0 - gap.1).abs() < 0.025)
                .count();
            // past the anticrossing, the mode followed: TE-like (across) or TM-like (along)
            let past: Vec<f64> = branch
                .found()
                .iter()
                .filter(|f| f.0 > gap.1 + 0.05)
                .map(|f| f.1.te_fraction())
                .collect();
            let te_like = past.iter().filter(|&&t| t > 0.5).count();
            eprintln!(
                "  past {:.3} µm: {te_like} samples TE-like, {} TM-like",
                gap.1 + 0.05,
                past.len() - te_like
            );
            match sweep {
                Ok(sweep) => eprintln!(
                    "  sweep at {tolerance:e}: converged {} in {} solves, estimate {:.1e}; {near_gap} of them within 25 nm of the gap",
                    sweep.converged,
                    sweep.calls(),
                    sweep.relative_error()
                ),
                Err(e) => eprintln!(
                    "  sweep at {tolerance:e}: failed at {failed:?} µm after {} solves: {e}",
                    branch.found().len()
                ),
            }
            eprintln!(
                "    at {b} µm {last}; least margin {:.3} at {l:.4} from {:.4} ({:.3} against {:.3})",
                s.margin(),
                s.from,
                s.overlap,
                s.runner_up.unwrap_or(0.0)
            );
        }
    }
}
