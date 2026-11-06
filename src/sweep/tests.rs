use faer::Mat;
use num_complex::Complex64 as c64;

use super::aaa::jacobi_for_tests;
use super::checks;
use super::*;
use rayon::prelude::*;

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
    noisy_seeded(x, eta, 0)
}

/// [`noisy`] with another realisation of its noise for each `seed` (0 is [`noisy`]'s own), each
/// uniform in ±η: the point's bits and the seed's mixed by SplitMix64's output function.
fn noisy_seeded(x: f64, eta: f64, seed: u64) -> c64 {
    let bits = if seed == 0 {
        x.to_bits()
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407)
    } else {
        splitmix(x.to_bits() ^ splitmix(seed))
    };
    let unit = (bits >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0;
    re((2.0 + 0.3 * (1.5 * x).sin() + 0.1 * x * x) * (1.0 + eta * unit))
}

/// SplitMix64's output function (Steele, Lea and Flood, OOPSLA 2014): 64 bits mixed into 64.
fn splitmix(z: u64) -> u64 {
    let z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
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
                // the fits interpolating to 1e-13 throughout, without the stopping rule
                &Options::new(tolerance).with_max_samples(60).with_stall(0),
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

/// One realisation of [`noisy`]'s noise: its seed ([`noisy_seeded`]), and every sample then
/// multiplied by 1 + `ulps` ε, a change in its last bits as another system's `sin` or its
/// rounding could make.
#[derive(Clone, Copy, Debug)]
struct Realisation {
    seed: u64,
    ulps: u32,
}

/// What a sweep of [`noisy`] gives at `tolerance`: the sweep, and its fit's true error against
/// the noiseless function relative to the largest |value|.
fn noisy_sweep(eta: f64, tolerance: f64, max_samples: usize) -> (Sweep, f64) {
    noisy_sweep_of(
        eta,
        Realisation { seed: 0, ulps: 0 },
        tolerance,
        max_samples,
    )
}

/// [`noisy_sweep`] of one realisation of the noise.
fn noisy_sweep_of(
    eta: f64,
    noise: Realisation,
    tolerance: f64,
    max_samples: usize,
) -> (Sweep, f64) {
    let moved = 1.0 + f64::from(noise.ulps) * f64::EPSILON;
    let sweep = Sweep::run(
        |x| Ok(noisy_seeded(x, eta, noise.seed) * moved),
        -1.0,
        1.0,
        &Options::new(tolerance)
            .with_max_samples(max_samples)
            .with_stall(10),
    )
    .unwrap();
    let truth = (0..=2000)
        .map(|k| -1.0 + 0.001 * f64::from(k))
        .map(|x| (sweep.eval(x) - noisy(x, 0.0)).norm())
        .fold(0.0, f64::max)
        / sweep.scale;
    (sweep, truth)
}

/// The noise levels of `the_stopping_rule_on_noisy_samples_study`'s ensemble.
const NOISE_LEVELS: [f64; 9] = [1e-12, 1e-11, 1e-10, 1e-9, 1e-8, 1e-7, 1e-6, 1e-5, 1e-4];

/// The levels `a_sweep_of_noisy_samples_stops_at_their_noise` sweeps.
const UNIT_LEVELS: [f64; 3] = [1e-12, 1e-10, 1e-8];

#[test]
fn a_sweep_of_noisy_samples_stops_at_their_noise() {
    // a relative noise η, uniform in ±η: its root mean square is η/√3 of each value, about
    // 0.5 η of the largest; a tolerance of 1e-12 can't be met, and the sweep stops at the noise
    // before its limit of 200, whatever the realisation. The promise is the ensemble's
    // (the_stopping_rule_on_noisy_samples_study): at these levels 120 sweeps of 120
    // stopped (30 seeds each, and seed 0 at 1e-10 moved by 0 to 29 ulps), each estimate at least
    // its fit's true error. Here nine: seed 0, seed 0 moved by 4 ulps (on which the rule as #714
    // left it, deciding at one step, ran to its limit at 1e-10) and seed 1, at each level.
    // Measures and bases: the count of stops, at least 8 of 9 (with no miss in 120 the rate
    // is at most 3 in 120 at 95 %, and two misses of nine then have a chance of 2 %); the
    // median roughness over the stops, relative to η, within the ensemble's 5 % to 95 % points
    // for one sweep at these levels, 0.54 to 41.47 (a median of eight or nine leaves them with
    // a chance below 1e-4); and the estimate against the true error, every sweep
    let realisations = [
        Realisation { seed: 0, ulps: 0 },
        Realisation { seed: 0, ulps: 4 },
        Realisation { seed: 1, ulps: 0 },
    ];
    let cases: Vec<(f64, Realisation)> = UNIT_LEVELS
        .into_iter()
        .flat_map(|eta| realisations.map(|r| (eta, r)))
        .collect();
    let sweeps: Vec<(Sweep, f64)> = cases
        .par_iter()
        .map(|&(eta, r)| noisy_sweep_of(eta, r, 1e-12, 200))
        .collect();
    let mut rough = Vec::new();
    for ((eta, r), (s, truth)) in cases.iter().zip(&sweeps) {
        assert!(
            *truth <= s.relative_error(),
            "{eta:e} {r:?}: true {truth:e}, estimate {:e}",
            s.relative_error()
        );
        if s.stalled && s.calls() < 200 {
            assert!(!s.converged, "{eta:e} {r:?}");
            rough.push(s.noise.unwrap() / eta);
        }
    }
    let calls: Vec<usize> = sweeps.iter().map(|s| s.0.calls()).collect();
    assert!(rough.len() >= 8, "{} of 9 stopped: {calls:?}", rough.len());
    rough.sort_by(f64::total_cmp);
    let median = rough[rough.len() / 2];
    assert!(
        (0.54..=41.47).contains(&median),
        "the median roughness {median} η: {rough:?}"
    );
    // a tolerance above the noise is met by the fits above it
    let (s, truth) = noisy_sweep(1e-8, 1e-6, 200);
    assert!(s.converged && s.noise.is_some() && truth <= s.relative_error());
    // without noise the rule never acts
    let (s, _) = noisy_sweep(0.0, 1e-12, 200);
    assert!(s.converged && !s.stalled && s.noise.is_none());
    // and without the rule the sweep runs to its limit
    let s = Sweep::run(
        |x| Ok(noisy(x, 1e-6)),
        -1.0,
        1.0,
        &Options::new(1e-12).with_max_samples(60).with_stall(0),
    )
    .unwrap();
    assert!(!s.stalled && s.calls() == 60);
}

#[test]
#[ignore = "the stopping rule on the 800 noiseless sweeps of sweep/estimate-bounds-error: in release, prints"]
fn the_stopping_rule_on_noiseless_sweeps_study() {
    // each test function at each tolerance with the rule and without: how many sweeps it
    // watches, how many it samples differently, how many it stops, and the largest ratio of
    // the true error to the estimate with it
    let functions = checks::test_functions(50);
    let (mut watched, mut changed, mut stopped, mut worst) = (0, 0, 0, 0.0f64);
    for (name, f) in &functions {
        for &t in &checks::RATIO_TOLERANCES {
            let run = |stall: usize| {
                Sweep::run(|x| Ok(f(x)), -1.0, 1.0, &Options::new(t).with_stall(stall)).unwrap()
            };
            let (with, without) = (run(10), run(0));
            if with.noise.is_none() {
                continue;
            }
            watched += 1;
            let truth = (0..=20000)
                .map(|k| -1.0 + 2.0 * f64::from(k) / 20000.0)
                .map(|x| (with.eval(x) - f(x)).norm())
                .fold(0.0, f64::max)
                / with.scale;
            worst = worst.max(truth / with.relative_error());
            let same = with.samples == without.samples && with.fit == without.fit;
            changed += usize::from(!same);
            stopped += usize::from(with.stalled);
            if !same || with.stalled {
                eprintln!(
                    "{name} {t:e}: {} samples against {}, estimate {:.1e} against {:.1e}, true {truth:.1e}, stalled {}",
                    with.calls(),
                    without.calls(),
                    with.relative_error(),
                    without.relative_error(),
                    with.stalled
                );
            }
        }
    }
    eprintln!(
        "{} sweeps: {watched} watched, {changed} sampled otherwise, {stopped} stopped; the largest true error over the estimate {worst:.2}",
        functions.len() * checks::RATIO_TOLERANCES.len()
    );
}

#[test]
#[ignore = "the noise-aware stopping rule on noisy samples (src/sweep.rs): in release, prints"]
fn the_stopping_rule_on_noisy_samples_study() {
    // asked for 1e-12: 30 seeds at each of nine levels (the table in docs/methods/sweep.md), and
    // seed 0 at 1e-10 with every sample moved by 0 to 29 ulps, as another system's last bits
    let mut jobs: Vec<(f64, Realisation)> = NOISE_LEVELS
        .into_iter()
        .flat_map(|eta| (1..=30).map(move |seed| (eta, Realisation { seed, ulps: 0 })))
        .collect();
    jobs.extend((0..30).map(|ulps| (1e-10, Realisation { seed: 0, ulps })));
    let started = std::time::Instant::now();
    let done = std::sync::atomic::AtomicUsize::new(0);
    let sweeps: Vec<(Sweep, f64)> = jobs
        .par_iter()
        .map(|&(eta, r)| {
            let out = noisy_sweep_of(eta, r, 1e-12, 200);
            let k = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            let elapsed = started.elapsed().as_secs_f64();
            eprintln!(
                "  {k}/{} ({:.0} %), {elapsed:.0} s, about {:.0} s to go",
                jobs.len(),
                100.0 * k as f64 / jobs.len() as f64,
                elapsed / k as f64 * (jobs.len() - k) as f64
            );
            out
        })
        .collect();
    let family = |name: String, keep: &dyn Fn(f64, Realisation) -> bool| {
        let rows: Vec<(f64, &Sweep, f64)> = jobs
            .iter()
            .zip(&sweeps)
            .filter(|((eta, r), _)| keep(*eta, *r))
            .map(|((eta, _), (s, truth))| (*eta, s, *truth))
            .collect();
        // the 5 %, 50 % and 95 % points of a quantity over the rows, and its largest
        let spread = |v: &dyn Fn(&(f64, &Sweep, f64)) -> f64| {
            let mut x: Vec<f64> = rows.iter().map(v).collect();
            x.sort_by(f64::total_cmp);
            let at = |q: f64| x[((x.len() - 1) as f64 * q).round() as usize];
            (at(0.05), at(0.5), at(0.95), x[x.len() - 1])
        };
        let calls = spread(&|r| r.1.calls() as f64);
        let rough = spread(&|r| r.1.noise.unwrap_or(f64::NAN) / r.0);
        let estimate = spread(&|r| r.1.relative_error() / r.0);
        let truth = spread(&|r| r.2 / r.0);
        let bound = spread(&|r| r.2 / r.1.relative_error());
        eprintln!(
            "{name}: stopped {} of {}; 5 %, 50 %, 95 % and most: samples {:.0} {:.0} {:.0} {:.0}, roughness {:.2} {:.2} {:.2} {:.2} η, estimate {:.1} {:.1} {:.2e} {:.2e} η, true error {:.1} {:.1} {:.1} {:.1} η, true error over estimate at most {:.2}",
            rows.iter()
                .filter(|r| r.1.stalled && r.1.calls() < 200)
                .count(),
            rows.len(),
            calls.0,
            calls.1,
            calls.2,
            calls.3,
            rough.0,
            rough.1,
            rough.2,
            rough.3,
            estimate.0,
            estimate.1,
            estimate.2,
            estimate.3,
            truth.0,
            truth.1,
            truth.2,
            truth.3,
            bound.3
        );
    };
    for eta in NOISE_LEVELS {
        family(format!("noise {eta:.0e}, 30 seeds"), &|e, r| {
            e == eta && r.seed > 0
        });
    }
    family("all levels and seeds".into(), &|_, r| r.seed > 0);
    // what a_sweep_of_noisy_samples_stops_at_their_noise's bands come from
    family(
        "noise 1e-12, 1e-10 and 1e-8, 30 seeds each, and seed 0 moved".into(),
        &|e, _| UNIT_LEVELS.contains(&e),
    );
    family("noise 1e-10, seed 0, 0 to 29 ulps".into(), &|_, r| {
        r.seed == 0
    });
    family("everything".into(), &|_, _| true);
    // a tolerance above the noise, met by the fits above it
    let (s, truth) = noisy_sweep(1e-10, 1e-8, 200);
    eprintln!(
        "noise 1e-10, seed 0, asked for 1e-8: converged {} in {} samples, estimate {:.1e}, true error {truth:.1e}",
        s.converged,
        s.calls(),
        s.relative_error()
    );
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
fn a_shift_almost_on_an_eigenvalue_is_moved_off_it() {
    // the solve a branch's sweep stopped at (#669): with 1.40 above the strip, at 1.4904658 µm,
    // the shift from the mode found nearest, 2.5e-5 from an eigenvalue; the third pair's
    // residual stayed at 1.8e-9 against 1e-9 for 200 restarts. Now the three nearest the shift,
    // nearest first, each within the solver's tolerance, the values the stall held
    use crate::eigen::{Shifted, krylov_schur};
    let cs = clad_strip(1.4).unwrap();
    let (entries, _) = crate::mode::vector::eigenproblem(&cs, um(1.4904658302821023), None);
    let n = cs.unknowns();
    let shift = c64::new(53.73827469444786, 0.0);
    let (pairs, record) = krylov_schur(n, &entries, shift, 3, 1e-9, None, &|| false)
        .unwrap()
        .unwrap();
    let held = [53.73830001319977, 51.95092559199361, 31.29088405755595];
    let a = Shifted::new(n, &entries, c64::new(0.0, 0.0)).unwrap();
    for (p, h) in pairs.iter().zip(held) {
        assert!((p.value.re - h).abs() <= 1e-9 * h, "{} {h}", p.value);
        assert!(a.residual(p.value, &p.vector) <= 1e-9 * p.value.norm());
    }
    assert_eq!(pairs.len(), 3);
    assert!(record.restarts < 60, "{}", record.restarts);
}

#[test]
fn a_refined_branch_follows_the_mode_by_index_through_an_anticrossing() {
    // the same 50 nm step into the anticrossing, refined where the margin is below 0.5: the
    // halves are clear choices, and the branch keeps the rank by index it started with, as
    // steps of 2.5 nm do; unrefined, it changes rank
    use crate::mode::vector;
    let near = te1_near(1.4, 1.35);
    let rank = |l: f64, n: f64| {
        vector::modes(&clad_strip(1.4).unwrap(), um(l), 4, None)
            .unwrap()
            .iter()
            .position(|m| (m.effective_index().re - n).abs() < 1e-9)
            .unwrap()
    };
    let start = rank(1.35, near);
    let mut refined = Branch::new(|_| clad_strip(1.4), um(1.35), Some(near), 3)
        .unwrap()
        .with_refinement(0.5, 5)
        .unwrap();
    refined.mode(um(1.4)).unwrap();
    let m = refined.mode(um(1.45)).unwrap();
    let (l, s) = refined.least_margin().unwrap();
    eprintln!(
        "refined: {} steps, least margin {:.3} ({:.3} against {:.3}) at {l}",
        refined.steps().len(),
        s.margin(),
        s.overlap,
        s.runner_up.unwrap_or(0.0)
    );
    assert!(s.margin() >= 0.5, "{l}: {s:?}");
    assert!(refined.steps().len() > 2, "{}", refined.steps().len());
    assert_eq!(rank(1.45, m.effective_index().re), start);
    let mut coarse = Branch::new(|_| clad_strip(1.4), um(1.35), Some(near), 3).unwrap();
    coarse.mode(um(1.4)).unwrap();
    let c = coarse.mode(um(1.45)).unwrap();
    assert_ne!(rank(1.45, c.effective_index().re), start);
    // a margin is from 0 to 1
    let b = Branch::new(|_| clad_strip(1.4), um(1.35), Some(near), 3).unwrap();
    assert!(b.with_refinement(1.5, 1).is_err());
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
