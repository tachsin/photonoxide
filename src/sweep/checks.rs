//! The adaptive sweeps' validation cases, shared by the tests and the validation report.

use std::f64::consts::PI;

use num_complex::Complex64 as c64;

use super::{Branch, Options, Rational, Sweep};
use crate::mode::Polarization;
use crate::mode::multilayer::Multilayer;
use crate::mode::vector::{self, CrossSection, Permittivity};
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier, note_numerical};

fn re(v: f64) -> c64 {
    c64::new(v, 0.0)
}

fn um(v: f64) -> Wavelength {
    Wavelength::from_um_unchecked(v)
}

/// The known rational function of type (3, 4) the first case fits: poles at 0.3 ± 0.2i and
/// −0.5 ± 0.1i.
fn known(x: f64) -> c64 {
    let x = re(x);
    (x * x * x - 0.5 * x + 0.2) / (((x - 0.3) * (x - 0.3) + 0.04) * ((x + 0.5) * (x + 0.5) + 0.01))
}

/// The poles of [`known`].
const KNOWN_POLES: [(f64, f64); 4] = [(0.3, 0.2), (0.3, -0.2), (-0.5, 0.1), (-0.5, -0.1)];

/// AAA on 200 evenly spaced samples of [`known`] over [−1, 1]: the fit's largest error on
/// 20001 points, relative to the function's largest sample, and its poles' largest distance
/// from the known ones (relative to their size); with the fit's number of support points.
pub(crate) fn known_rational() -> (f64, f64, usize) {
    let x: Vec<f64> = (0..200)
        .map(|k| -1.0 + 2.0 * f64::from(k) / 199.0)
        .collect();
    let f: Vec<c64> = x.iter().map(|&x| known(x)).collect();
    let fit = Rational::aaa(&x, &f, super::DEFAULT_TOLERANCE).expect("a valid fit");
    let scale = f.iter().map(|v| v.norm()).fold(0.0, f64::max);
    let value = (0..=20000)
        .map(|k| {
            let x = -1.0 + 2.0 * f64::from(k) / 20000.0;
            (fit.eval(x) - known(x)).norm()
        })
        .fold(0.0, f64::max)
        / scale;
    let poles = fit.poles();
    let pole = KNOWN_POLES
        .iter()
        .map(|&(a, b)| {
            let p = c64::new(a, b);
            poles
                .iter()
                .map(|q| (q - p).norm() / p.norm())
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max);
    (value, pole, fit.len())
}

/// A small deterministic generator for the test functions' parameters (Knuth's MMIX LCG).
struct Lcg(u64);

impl Lcg {
    /// Uniform on [a, b).
    fn uniform(&mut self, a: f64, b: f64) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        a + (b - a) * ((self.0 >> 11) as f64 / (1u64 << 53) as f64)
    }

    /// 10^u, u uniform on [a, b).
    fn log(&mut self, a: f64, b: f64) -> f64 {
        10f64.powf(self.uniform(a, b))
    }
}

/// A test function with known values on [−1, 1], and what kind it is.
pub(crate) type TestFunction = (&'static str, Box<dyn Fn(f64) -> c64 + Send + Sync>);

/// The test functions of `sweep/estimate-bounds-error`, `each` of every kind, on [−1, 1]:
/// Lorentzian resonances, two overlapping ones, a ring's all-pass response, a Fabry–Perot
/// transmission, a delay line's phase, poles near the range, a branch point near its end,
/// steps (tanh), exp(sin), and a smooth effective index; their widths, positions and strengths
/// spread over decades.
pub(crate) fn test_functions(each: usize) -> Vec<TestFunction> {
    let mut g = Lcg(2018);
    let mut out: Vec<TestFunction> = Vec::new();
    for _ in 0..each {
        let (c, w) = (g.uniform(-1.0, 1.0), g.log(-1.7, -0.5));
        out.push((
            "Lorentzian",
            Box::new(move |x| re(1.0 / (1.0 + ((x - c) / w).powi(2)) + 0.3 * x)),
        ));
        let (c, d, w) = (
            g.uniform(-0.5, 0.5),
            g.uniform(0.05, 0.3),
            g.log(-1.5, -0.7),
        );
        out.push((
            "two Lorentzians",
            Box::new(move |x| {
                re(1.0 / (1.0 + ((x - c) / w).powi(2)) + 0.7 / (1.0 + ((x - c - d) / w).powi(2)))
            }),
        ));
        let (r, a, l, p) = (
            g.uniform(0.8, 0.99),
            g.uniform(0.8, 0.99),
            g.uniform(3.0, 15.0),
            g.uniform(0.0, 2.0 * PI),
        );
        out.push((
            "ring",
            Box::new(move |x| {
                let e = c64::from_polar(a, p + l * x);
                (r - e) / (1.0 - r * e)
            }),
        ));
        let (f, k, p) = (g.log(0.0, 1.7), g.uniform(2.0, 8.0), g.uniform(0.0, 3.0));
        out.push((
            "Fabry-Perot",
            Box::new(move |x| re(1.0 / (1.0 + f * (k * x + p).sin().powi(2)))),
        ));
        let k = g.uniform(5.0, 30.0);
        out.push(("delay", Box::new(move |x| c64::from_polar(1.0, k * x))));
        let p = c64::new(g.uniform(-1.0, 1.0), g.log(-1.7, -0.3));
        out.push(("pole", Box::new(move |x| 1.0 / (re(x) - p))));
        let d = 1.0 + g.log(-2.0, 0.0);
        out.push(("branch point", Box::new(move |x| re((x + d).sqrt()))));
        let (s, c) = (g.log(0.0, 1.3), g.uniform(-0.8, 0.8));
        out.push(("step", Box::new(move |x| re((s * (x - c)).tanh()))));
        let (k, p) = (g.uniform(1.0, 6.0), g.uniform(0.0, 6.0));
        out.push(("exp(sin)", Box::new(move |x| re((k * x + p).sin().exp()))));
        let (a, b) = (g.uniform(0.05, 0.2), g.uniform(0.01, 0.05));
        out.push((
            "effective index",
            Box::new(move |x| re(2.8 - a * x - b * x * x + 0.01 * (-x).exp())),
        ));
    }
    out
}

/// The tolerances asked of the sweeps of `sweep/estimate-bounds-error`.
pub(crate) const RATIO_TOLERANCES: [f64; 4] = [1e-4, 1e-6, 1e-8, 1e-10];

/// One sweep of a test function over [−1, 1] at `tolerance`: its true error (on 20001 points
/// and at the real parts of its fit's poles in the range, relative to its largest sample), its
/// estimate, and its number of samples.
pub(crate) fn true_and_estimated(
    f: &(dyn Fn(f64) -> c64 + Sync),
    tolerance: f64,
) -> (f64, f64, usize) {
    let sweep = Sweep::run(|x| Ok(f(x)), -1.0, 1.0, &Options::new(tolerance))
        .expect("a sweep of a finite function");
    let mut points: Vec<f64> = (0..=20000)
        .map(|k| -1.0 + 2.0 * f64::from(k) / 20000.0)
        .collect();
    points.extend(
        sweep
            .fit
            .poles()
            .iter()
            .map(|p| p.re)
            .filter(|x| x.abs() <= 1.0),
    );
    let true_error = points
        .iter()
        .map(|&x| (sweep.eval(x) - f(x)).norm())
        .fold(0.0, f64::max)
        / sweep.scale;
    (true_error, sweep.relative_error(), sweep.calls())
}

/// Every test function at every tolerance of [`RATIO_TOLERANCES`]: the largest ratio of the
/// true error to the estimate, the median ratio, and the mean number of samples.
pub(crate) fn estimate_ratios(each: usize) -> (f64, f64, f64) {
    let functions = test_functions(each);
    let jobs: Vec<(usize, f64)> = (0..functions.len())
        .flat_map(|i| RATIO_TOLERANCES.iter().map(move |&t| (i, t)))
        .collect();
    let results =
        crate::parallel::map_in_order(&jobs, |&(i, t)| true_and_estimated(&*functions[i].1, t));
    let mut ratios: Vec<f64> = results.iter().map(|&(t, e, _)| t / e).collect();
    ratios.sort_by(f64::total_cmp);
    let calls = results.iter().map(|r| r.2 as f64).sum::<f64>() / results.len() as f64;
    (
        ratios.last().copied().unwrap_or(f64::NAN),
        ratios[ratios.len() / 2],
        calls,
    )
}

/// The slab of `sweep/slab-dispersion`: 220 nm of silicon (3.473) in oxide (1.444).
fn slab() -> Multilayer {
    Multilayer::new(re(1.444), &[(re(3.473), Length::nm(220.0))], re(1.444)).expect("a valid slab")
}

/// The slab's TE0 effective index at `l` µm, exactly (its bound modes' largest root).
pub(crate) fn slab_te0(l: f64) -> f64 {
    slab()
        .bound_modes(Polarization::Te, um(l))
        .expect("a lossless slab")[0]
        .effective_index()
        .re
}

/// The range of `sweep/slab-dispersion`, µm.
pub(crate) const SLAB_RANGE: (f64, f64) = (1.2, 2.0);

/// The tolerance of `sweep/slab-dispersion` and `sweep/mode-crossing`, relative; their sweeps
/// are asked for a third of it.
pub(crate) const SLAB_TOLERANCE: f64 = 1e-10;

/// The slab's TE0 n_eff swept over [`SLAB_RANGE`] at a third of [`SLAB_TOLERANCE`]: the sweep,
/// and its true error against the exact roots on 4001 wavelengths, relative to the largest
/// sample.
pub(crate) fn slab_sweep() -> (Sweep, f64) {
    let (a, b) = SLAB_RANGE;
    let sweep = Sweep::run(
        |l| Ok(re(slab_te0(l))),
        a,
        b,
        &Options::new(SLAB_TOLERANCE / 3.0),
    )
    .expect("a sweep of the slab");
    let error = (0..=4000)
        .map(|k| {
            let l = a + (b - a) * f64::from(k) / 4000.0;
            (sweep.eval(l).re - slab_te0(l)).abs()
        })
        .fold(0.0, f64::max)
        / sweep.scale;
    (sweep, error)
}

/// How a uniform grid's samples are interpolated, for the comparison with a sweep.
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum Uniform {
    /// Straight lines between samples.
    Linear,
    /// Lagrange's cubic through the four nearest samples (the end intervals' from the first or
    /// last four), fourth order.
    Cubic,
    /// AAA on the uniform samples.
    Aaa,
}

/// The largest error over 4001 points of `range` of `f` from `n` evenly spaced samples
/// interpolated by `how`, relative to the largest |f| there.
#[cfg(test)]
pub(crate) fn uniform_error(
    f: &dyn Fn(f64) -> c64,
    range: (f64, f64),
    n: usize,
    how: Uniform,
) -> f64 {
    let (a, b) = range;
    let h = (b - a) / (n - 1) as f64;
    let x: Vec<f64> = (0..n).map(|k| a + h * k as f64).collect();
    let v: Vec<c64> = x.iter().map(|&l| f(l)).collect();
    let fit = matches!(how, Uniform::Aaa)
        .then(|| Rational::aaa(&x, &v, super::DEFAULT_TOLERANCE).expect("a valid fit"));
    let at = |l: f64| -> c64 {
        let k = (((l - a) / h).floor() as usize).min(n - 2);
        match how {
            Uniform::Linear => {
                let t = (l - x[k]) / h;
                v[k] * (1.0 - t) + v[k + 1] * t
            }
            Uniform::Cubic => {
                let s = k.saturating_sub(1).min(n.saturating_sub(4));
                (s..s + 4)
                    .map(|i| {
                        (s..s + 4)
                            .filter(|&j| j != i)
                            .map(|j| (l - x[j]) / (x[i] - x[j]))
                            .product::<f64>()
                            * v[i]
                    })
                    .sum()
            }
            Uniform::Aaa => fit.as_ref().map_or(re(f64::NAN), |r| r.eval(l)),
        }
    };
    let mut scale = 0.0f64;
    let mut worst = 0.0f64;
    for k in 0..=4000 {
        let l = a + (b - a) * f64::from(k) / 4000.0;
        let exact = f(l);
        scale = scale.max(exact.norm());
        let e = (at(l) - exact).norm();
        worst = worst.max(if e.is_nan() { f64::INFINITY } else { e });
    }
    worst / scale
}

/// The fewest evenly spaced samples (at most `limit`) whose interpolant by `how` reaches
/// `tolerance` ([`uniform_error`]), by doubling and then bisection (the error falls with the
/// number); `None` past the limit.
#[cfg(test)]
pub(crate) fn uniform_calls(
    f: &dyn Fn(f64) -> c64,
    range: (f64, f64),
    tolerance: f64,
    how: Uniform,
    limit: usize,
) -> Option<usize> {
    let mut hi = 4;
    while uniform_error(f, range, hi, how) > tolerance {
        if hi >= limit {
            return None;
        }
        hi = (2 * hi).min(limit);
    }
    // (the cubic needs 4 samples)
    let mut lo = (hi / 2).max(3);
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if uniform_error(f, range, mid, how) > tolerance {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(hi)
}
/// The strip of `sweep/mode-crossing`: 600 × 220 nm of silicon (3.473) in oxide (1.444), on a
/// 20 nm grid over 2.4 × 1.6 µm. Symmetric above and below and left and right, so its TE1-like
/// mode (odd across the width) and TM0-like mode (even) can't couple and cross near 1.4 µm.
pub(crate) fn crossing_strip(_: Wavelength) -> crate::Result<CrossSection> {
    CrossSection::uniform((-1.2, 1.2, 120), (-0.8, 0.8, 80), |x, y| {
        let n: f64 = if x.abs() < 0.3 && y.abs() < 0.11 {
            3.473
        } else {
            1.444
        };
        Permittivity::isotropic(re(n * n))
    })
}

/// The range of `sweep/mode-crossing`, µm.
pub(crate) const CROSSING_RANGE: (f64, f64) = (1.3, 1.6);

/// The strip's TE1-like and TM0-like modes at `l` µm from the solver, told apart by their
/// polarization, not by overlap: of the four modes nearest the top, the second with H mostly
/// along y (TE-like) and the first with H mostly along x.
pub(crate) fn te1_tm0(l: f64) -> (f64, f64) {
    let modes = vector::modes(
        &crossing_strip(um(l)).expect("a valid strip"),
        um(l),
        4,
        None,
    )
    .expect("the strip's modes");
    let te: Vec<f64> = modes
        .iter()
        .filter(|m| m.te_fraction() > 0.5)
        .map(|m| m.effective_index().re)
        .collect();
    let tm = modes
        .iter()
        .find(|m| m.te_fraction() < 0.5)
        .map(|m| m.effective_index().re);
    (te[1], tm.expect("a TM-like mode"))
}

/// The TE1-like branch of [`crossing_strip`] swept over [`CROSSING_RANGE`] at a third of
/// [`SLAB_TOLERANCE`], followed by overlap from the mode nearest 2.08 at the range's start,
/// among 3 candidates: the sweep, and the held-out wavelengths' (µm) values from the solver,
/// (TE1, TM0) told apart by polarization.
pub(crate) fn crossing_sweep() -> (Sweep, Vec<(f64, f64, f64)>) {
    let (a, b) = CROSSING_RANGE;
    let mut branch = Branch::new(crossing_strip, um(a), Some(2.08), 3).expect("the first mode");
    let sweep = Sweep::run(
        |l| Ok(re(branch.effective_index(um(l))?.re)),
        a,
        b,
        &Options::new(SLAB_TOLERANCE / 3.0),
    )
    .expect("a sweep of the strip");
    let held: Vec<(f64, f64, f64)> = (0..12)
        .map(|k| {
            let l = a + (b - a) * (f64::from(k) + 0.5) / 12.0;
            let (te1, tm0) = te1_tm0(l);
            (l, te1, tm0)
        })
        .collect();
    (sweep, held)
}

/// `v` rounded up to one significant digit: a sweep's estimate, whose last digits are the
/// platform's (its fits' singular vectors), shown as a bound that reads the same everywhere.
fn up(v: f64) -> f64 {
    if v <= 0.0 || !v.is_finite() {
        return v;
    }
    let p = 10f64.powf(v.log10().floor());
    (v / p).ceil() * p
}

fn zero(measured: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected: 0.0,
        tolerance,
        error: measured,
    }
}

/// Every adaptive-sweep case, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "sweep/aaa-rational",
            title: r"AAA on 200 samples over $[-1, 1]$ of a known rational function of type (3, 4), poles at $0.3 \pm 0.2i$ and $-0.5 \pm 0.1i$: the fit on 20001 points and its poles (larger relative error shown)",
            tier: Tier::Analytic,
            source: r"the function itself, $(x^3 - x/2 + 1/5) / \bigl(((x - 0.3)^2 + 0.04)((x + 0.5)^2 + 0.01)\bigr)$: AAA's type $(m - 1, m - 1)$ holds it exactly from $m = 5$ (Nakatsukasa, Sète, Trefethen 2018, doi:10.1137/16M1106122, Eqs. 2.5, 3.4–3.11)",
            run: || {
                let (value, pole, _) = known_rational();
                zero(value.max(pole), 1e-10)
            },
        },
        Case {
            id: "sweep/estimate-bounds-error",
            title: r"The sweep's error estimate against its true error: 200 test functions with known values (resonances, ring and Fabry–Perot responses, a delay line's phase, poles near the range, branch points, steps, a smooth effective index) at tolerances $10^{-4}$ to $10^{-10}$, 800 sweeps (largest ratio of the true error to the estimate shown)",
            tier: Tier::Analytic,
            source: r"each function's own values on 20001 points and at its fit's poles; the estimate bounds the error where the fit to all samples is at least 1.5 times as accurate as a fit to every other one",
            run: || {
                let (worst, _, _) = estimate_ratios(20);
                // to two decimals, rounded up: the ratio's last digits are the platform's
                zero((worst * 100.0).ceil() / 100.0, 1.0)
            },
        },
        Case {
            id: "sweep/slab-dispersion",
            title: r"An adaptive sweep of a slab's TE0 $n_\text{eff}$ from 1.2 to 2.0 µm (220 nm of 3.473 in 1.444), asked for $3.3 \times 10^{-11}$: its fit against the exact roots on 4001 wavelengths (largest error relative to the largest $n_\text{eff}$ shown)",
            tier: Tier::Analytic,
            source: r"the slab's exact characteristic equation (mode::multilayer; J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Eq. 26), its roots to the last bit",
            run: || {
                let (sweep, error) = slab_sweep();
                note_numerical(up(sweep.relative_error()));
                zero(error, SLAB_TOLERANCE)
            },
        },
        Case {
            id: "sweep/mode-crossing",
            title: r"A 600 × 220 nm silicon strip in oxide (20 nm grid), its TE1-like mode swept from 1.3 to 1.6 µm through its crossing with the TM0-like mode near 1.4 µm, followed by overlap, asked for $3.3 \times 10^{-11}$: the fit against the solver's TE1-like mode, told apart by polarization, at 12 wavelengths between samples (largest error relative to the largest $n_\text{eff}$ shown)",
            tier: Tier::Analytic,
            source: r"the strip's two mirror symmetries: TE1 (odd across the width) and TM0 (even) can't couple, so the branch is the solver's second TE-like mode at every wavelength, the same grid",
            run: || {
                let (sweep, held) = crossing_sweep();
                note_numerical(up(sweep.relative_error()));
                let error = held
                    .iter()
                    .map(|&(l, te1, _)| (sweep.eval(l).re - te1).abs())
                    .fold(0.0, f64::max)
                    / sweep.scale;
                zero(error, SLAB_TOLERANCE)
            },
        },
    ]
}
