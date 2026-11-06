//! Several responses on one set of samples: rational functions that share their support points
//! and weights, so one denominator (the set-valued AAA of P. Lietaert, K. Meerbergen, J. Pérez,
//! B. Vandereycken, "Automatic rational approximation and linearization of nonlinear eigenvalue
//! problems", IMA J. Numer. Anal. 42, 1087 (2021),
//! [doi:10.1093/imanum/draa098](https://doi.org/10.1093/imanum/draa098), their Section 2.2),
//! and a sweep that places one sample for all of them where the worst one needs it.

use faer::Mat;
use num_complex::Complex64 as c64;

use super::aaa::{poles, smallest_right_singular_vector};
use super::{DEFAULT_TOLERANCE, FLOOR, Options, relative_to};
use crate::{Error, Result};

/// K rational functions of type (m − 1, m − 1) that share their m support points z_j and their
/// weights w_j, so their denominator:
///
/// r_k(x) = Σ w_j f_jk/(x − z_j) / Σ w_j/(x − z_j),
///
/// f_jk the k-th function's value at z_j (Lietaert et al.'s Eq. 2.4). Each interpolates its own
/// values at the support points where w_j ≠ 0, as [`super::Rational`] does, and a linear relation
/// the values all keep, such as f_jk = f_jl, the functions keep everywhere.
#[derive(Clone, Debug, PartialEq)]
pub struct RationalSet {
    support: Vec<f64>,
    /// The values at each support point, the functions' in order.
    values: Vec<Vec<c64>>,
    weights: Vec<c64>,
}

impl RationalSet {
    /// The set-valued AAA approximation of the samples `f` at the distinct points `x`, `f[i]`
    /// every function's value at `x[i]` (Lietaert et al.'s Section 2.2): at step m the next support
    /// point is the sample where the largest of the functions' residuals |f_k − r_k| is largest,
    /// and the weights (‖w‖ = 1) minimize the stacked Loewner matrices' ‖A w‖ (their Eq. 2.5),
    /// the sum of every function's squared linearized residuals. They scale each function by its
    /// largest value first; here the functions keep their own sizes, so that the fit is made, and
    /// stops, at one scale for all of them (an S-matrix's entries, all at most 1, where a small
    /// reflection scaled up would weigh as much as the transmission in the denominator). It stops
    /// when every residual on the samples is at most `tolerance` times the largest |f| of all, or
    /// at m = M/2 for M samples, as [`super::Rational::aaa`]. With one function it is that.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no samples, `x` and `f` of different lengths, samples without
    /// a value or with different numbers of them, a point or a value that isn't finite, points
    /// that aren't distinct, or a tolerance that isn't a non-negative number.
    pub fn aaa(x: &[f64], f: &[Vec<c64>], tolerance: f64) -> Result<RationalSet> {
        if x.is_empty() || x.len() != f.len() {
            return Err(Error::invalid(
                "set-valued AAA",
                format!(
                    "needs samples, and values for each point: got {} points and {} samples",
                    x.len(),
                    f.len()
                ),
            ));
        }
        let count = f[0].len();
        if count == 0 || f.iter().any(|v| v.len() != count) {
            return Err(Error::invalid(
                "set-valued AAA",
                "every sample needs the same number of values, at least one",
            ));
        }
        if !(tolerance >= 0.0 && tolerance.is_finite()) {
            return Err(Error::invalid(
                "set-valued AAA",
                format!("the tolerance must be a non-negative number, got {tolerance}"),
            ));
        }
        if x.iter().any(|v| !v.is_finite()) || f.iter().flatten().any(|v| !v.is_finite()) {
            return Err(Error::invalid(
                "set-valued AAA",
                "the points and values must be finite",
            ));
        }
        let mut sorted = x.to_vec();
        sorted.sort_by(f64::total_cmp);
        if sorted.windows(2).any(|w| w[0] == w[1]) {
            return Err(Error::invalid(
                "set-valued AAA",
                "the points must be distinct",
            ));
        }
        let m_total = x.len();
        let scale = f.iter().flatten().map(|v| v.norm()).fold(0.0, f64::max);
        let mean: Vec<c64> = (0..count)
            .map(|k| f.iter().map(|v| v[k]).sum::<c64>() / m_total as f64)
            .collect();
        let mut r = vec![mean; m_total];
        // the largest residual over the functions at sample i; a zero of the denominator there
        // is the largest residual
        let residual = |i: usize, r: &[Vec<c64>]| {
            (0..count)
                .map(|k| (f[i][k] - r[i][k]).norm())
                .fold(
                    0.0,
                    |a: f64, b| if b.is_nan() { f64::INFINITY } else { a.max(b) },
                )
        };
        let mut rest: Vec<usize> = (0..m_total).collect();
        let mut chosen: Vec<usize> = Vec::new();
        let mut weights = vec![c64::new(1.0, 0.0)];
        let steps = (m_total / 2).max(1);
        for m in 1..=steps {
            let (at, _) = rest
                .iter()
                .enumerate()
                .map(|(k, &i)| (k, residual(i, &r)))
                .fold((0, -1.0), |best, c| if c.1 > best.1 { c } else { best });
            chosen.push(rest.remove(at));
            if rest.is_empty() {
                weights = vec![c64::new(1.0, 0.0)];
                break;
            }
            // the Loewner matrices of the functions over the rest, stacked (their Eq. 2.5)
            let rows = rest.len();
            let a = Mat::<c64>::from_fn(rows * count, m, |row, j| {
                let (k, i) = (row / rows, rest[row % rows]);
                let zj = chosen[j];
                (f[i][k] - f[zj][k]) / (x[i] - x[zj])
            });
            weights = smallest_right_singular_vector(&a);
            let fit = RationalSet {
                support: chosen.iter().map(|&i| x[i]).collect(),
                values: chosen.iter().map(|&i| f[i].clone()).collect(),
                weights: weights.clone(),
            };
            for &i in &rest {
                r[i] = fit.eval(x[i]);
            }
            for &i in &chosen {
                r[i].clone_from(&f[i]);
            }
            let error = (0..m_total).map(|i| residual(i, &r)).fold(0.0, f64::max);
            if error <= tolerance * scale {
                break;
            }
        }
        Ok(RationalSet {
            support: chosen.iter().map(|&i| x[i]).collect(),
            values: chosen.iter().map(|&i| f[i].clone()).collect(),
            weights,
        })
    }

    /// Every function at `x` by the barycentric formula, in order; their values themselves at a
    /// support point.
    pub fn eval(&self, x: f64) -> Vec<c64> {
        let count = self.functions();
        let (mut n, mut d) = (vec![c64::new(0.0, 0.0); count], c64::new(0.0, 0.0));
        for ((&z, f), &w) in self.support.iter().zip(&self.values).zip(&self.weights) {
            if x == z {
                return f.clone();
            }
            let c = w / (x - z);
            for (n, &f) in n.iter_mut().zip(f) {
                *n += c * f;
            }
            d += c;
        }
        n.into_iter().map(|n| n / d).collect()
    }

    /// The number of functions K.
    pub fn functions(&self) -> usize {
        self.values.first().map_or(0, Vec::len)
    }

    /// The support points z_j, which the functions share.
    pub fn support(&self) -> &[f64] {
        &self.support
    }

    /// The weights w_j, of unit 2-norm, which the functions share.
    pub fn weights(&self) -> &[c64] {
        &self.weights
    }

    /// The number of support points m: each function is of type (m − 1, m − 1).
    pub fn len(&self) -> usize {
        self.support.len()
    }

    /// Whether there are no support points (never, for one [`RationalSet::aaa`] made).
    pub fn is_empty(&self) -> bool {
        self.support.is_empty()
    }

    /// The poles the functions share: the zeros of their denominator, as
    /// [`super::Rational::poles`] finds them (Nakatsukasa et al.'s Eq. 3.11). Any order.
    pub fn poles(&self) -> Vec<c64> {
        poles(&self.support, &self.weights)
    }
}

/// Several responses sampled adaptively on one set of points and fitted by set-valued AAA, with
/// each one's estimated error ([`SweepSet::run`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SweepSet {
    /// The range's ends.
    pub range: (f64, f64),
    /// The samples (point, every response's value there), in order along the range: one solver
    /// call each.
    pub samples: Vec<(f64, Vec<c64>)>,
    /// The set-valued AAA fit to all of them.
    pub fit: RationalSet,
    /// The estimated error of the worst response, in the values' unit: the largest of
    /// [`SweepSet::errors`].
    pub error: f64,
    /// Each response's estimated error, in order: the largest over the range.
    pub errors: Vec<f64>,
    /// The largest |value| of any response sampled, which the tolerance is relative to.
    pub scale: f64,
    /// The relative tolerance asked for.
    pub tolerance: f64,
    /// Whether the worst response's estimate reached the tolerance (not: the sample limit was
    /// reached first).
    pub converged: bool,
    /// The relative estimate of the worst response after each sample from the initial ones on.
    pub history: Vec<f64>,
}

impl SweepSet {
    /// Samples `f`, which gives K responses at a point, over `from` to `to` until the estimated
    /// error of every response's fit is at most `options.tolerance` times the largest |value| of
    /// any response sampled, or `options.max_samples` samples are taken. It is [`super::Sweep`]
    /// with one set of samples for all the responses: the fits are set-valued AAA's, sharing a
    /// denominator ([`RationalSet::aaa`]), the error estimate is each response's twice the
    /// largest difference between the fit to all samples and either interleaved half's, over
    /// the grid and the real parts of the three fits' poles in the range, as [`super::Sweep`]'s
    /// (the module's documentation), and the next sample is where the largest of those
    /// differences over the responses is largest. The stopping rule on noisy samples
    /// ([`Options::stall`]) is a scalar sweep's only.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a range that isn't finite and increasing, options outside
    /// their limits ([`Options`]) or with the stopping rule, and values from `f` that aren't
    /// finite, are none, or change in number; and `f`'s own errors.
    pub fn run(
        mut f: impl FnMut(f64) -> Result<Vec<c64>>,
        from: f64,
        to: f64,
        options: &Options,
    ) -> Result<SweepSet> {
        if !(from.is_finite() && to.is_finite() && from < to) {
            return Err(Error::invalid(
                "sweep",
                format!("the range must be finite and increasing, got {from} to {to}"),
            ));
        }
        options.check()?;
        if options.stall > 0 {
            return Err(Error::invalid(
                "sweep",
                "the stopping rule (stall) is a scalar sweep's: a set of responses sweeps without it",
            ));
        }
        let width = to - from;
        let at = |k: usize, n: usize| {
            if k + 1 == n {
                to
            } else {
                from + width * k as f64 / (n - 1) as f64
            }
        };
        let grid: Vec<f64> = (0..options.points).map(|k| at(k, options.points)).collect();
        let mut samples: Vec<(f64, Vec<c64>)> = Vec::with_capacity(options.max_samples);
        let mut count: Option<usize> = None;
        let mut sample = |x: f64, samples: &mut Vec<(f64, Vec<c64>)>| -> Result<()> {
            let v = f(x)?;
            if v.is_empty() || count.is_some_and(|c| c != v.len()) {
                return Err(Error::invalid(
                    "sweep",
                    format!(
                        "the function gave {} values at {x}, where it gave {} before: one for \
                         each response, the same everywhere",
                        v.len(),
                        count.unwrap_or(0)
                    ),
                ));
            }
            count = Some(v.len());
            if let Some(bad) = v.iter().find(|v| !v.is_finite()) {
                return Err(Error::invalid(
                    "sweep",
                    format!("a value of the function at {x} isn't finite: {bad}"),
                ));
            }
            let i = samples.partition_point(|s| s.0 < x);
            samples.insert(i, (x, v));
            Ok(())
        };
        for k in 0..options.initial {
            sample(at(k, options.initial), &mut samples)?;
        }
        let mut history = Vec::new();
        loop {
            let step = estimate(&samples, &grid, from, to)?;
            history.push(relative_to(step.error, step.scale));
            let converged = step.error <= options.tolerance * step.scale;
            match step.next.filter(|_| !converged) {
                Some(x) if samples.len() < options.max_samples => sample(x, &mut samples)?,
                _ => {
                    return Ok(SweepSet {
                        range: (from, to),
                        converged,
                        fit: step.fit,
                        error: step.error,
                        errors: step.errors,
                        scale: step.scale,
                        samples,
                        tolerance: options.tolerance,
                        history,
                    });
                }
            }
        }
    }

    /// Every response's fit at `x`, in order.
    pub fn eval(&self, x: f64) -> Vec<c64> {
        self.fit.eval(x)
    }

    /// The worst response's estimated error relative to the largest |value| sampled.
    pub fn relative_error(&self) -> f64 {
        relative_to(self.error, self.scale)
    }

    /// The number of samples: the solver calls the sweep made.
    pub fn calls(&self) -> usize {
        self.samples.len()
    }
}

/// One step of a set's sweep: the fit to all samples, each response's estimated error and the
/// largest, the largest |value|, and where the largest difference between the fits is largest
/// among the points not yet sampled.
struct Step {
    fit: RationalSet,
    errors: Vec<f64>,
    error: f64,
    scale: f64,
    next: Option<f64>,
}

/// The fit to `samples` and its estimate over `grid` and the three fits' poles in [from, to]
/// ([`SweepSet::run`]).
fn estimate(samples: &[(f64, Vec<c64>)], grid: &[f64], from: f64, to: f64) -> Result<Step> {
    let fit_of = |part: &[&(f64, Vec<c64>)]| {
        let x: Vec<f64> = part.iter().map(|s| s.0).collect();
        let f: Vec<Vec<c64>> = part.iter().map(|s| s.1.clone()).collect();
        RationalSet::aaa(&x, &f, DEFAULT_TOLERANCE)
    };
    let all: Vec<&(f64, Vec<c64>)> = samples.iter().collect();
    let fit = fit_of(&all)?;
    let even: Vec<&(f64, Vec<c64>)> = samples.iter().step_by(2).collect();
    let odd: Vec<&(f64, Vec<c64>)> = samples.iter().skip(1).step_by(2).collect();
    let halves = [fit_of(&even)?, fit_of(&odd)?];
    let mut points = grid.to_vec();
    for r in std::iter::once(&fit).chain(&halves) {
        points.extend(
            r.poles()
                .into_iter()
                .map(|p| p.re)
                .filter(|&x| x >= from && x <= to),
        );
    }
    let count = fit.functions();
    let scale = samples
        .iter()
        .flat_map(|s| s.1.iter())
        .map(|v| v.norm())
        .fold(0.0, f64::max);
    let separation = 1e-9 * (to - from);
    let mut largest = vec![0.0f64; count];
    let mut next: Option<(f64, f64)> = None;
    let finite = |d: f64| if d.is_finite() { d } else { f64::INFINITY };
    for &x in &points {
        let r = fit.eval(x);
        let h = [halves[0].eval(x), halves[1].eval(x)];
        let mut worst = 0.0f64;
        for k in 0..count {
            let d = finite((r[k] - h[0][k]).norm()).max(finite((r[k] - h[1][k]).norm()));
            largest[k] = largest[k].max(d);
            worst = worst.max(d);
        }
        let i = samples.partition_point(|s| s.0 < x);
        let sampled = [i.wrapping_sub(1), i]
            .iter()
            .filter_map(|&j| samples.get(j))
            .any(|s| (s.0 - x).abs() <= separation);
        if !sampled && next.is_none_or(|(_, b)| worst > b) {
            next = Some((x, worst));
        }
    }
    let errors: Vec<f64> = largest
        .into_iter()
        .map(|d| (2.0 * d).max(FLOOR * scale))
        .collect();
    let error = errors.iter().copied().fold(0.0, f64::max);
    Ok(Step {
        fit,
        errors,
        error,
        scale,
        next: next.map(|(x, _)| x),
    })
}
