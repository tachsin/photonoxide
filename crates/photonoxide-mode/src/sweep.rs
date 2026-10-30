//! Adaptive sweeps: a response over wavelength or frequency from as few solves as it needs,
//! as a rational function with an estimate of its error.
//!
//! A sweep asks a solver for a quantity (a mode's effective index, an S-parameter) at the
//! points it chooses in a range, fits the samples with the AAA algorithm, and adds samples where
//! its error estimate is largest until that estimate is below the tolerance asked for. What
//! comes back ([`Sweep`]) is the fit, which can be evaluated anywhere in the range, its samples
//! and its estimated error.
//!
//! ```
//! use photonoxide::Complex64;
//! use photonoxide::sweep::{Options, Sweep};
//!
//! // a resonance 4 nm wide at 1.55 µm; in practice a solver's n_eff or S-parameter at λ (µm)
//! let response = |l: f64| Ok(Complex64::new(1.0 / (1.0 + ((l - 1.55) / 0.002).powi(2)), 0.0));
//! let sweep = Sweep::run(response, 1.5, 1.6, &Options::new(1e-8))?;
//! assert!(sweep.converged && sweep.relative_error() <= 1e-8);
//! let exact = 1.0 / (1.0 + (0.0012f64 / 0.002).powi(2));
//! assert!((sweep.eval(1.5512).re - exact).abs() <= 1e-8);
//! # Ok::<(), photonoxide::Error>(())
//! ```
//!
//! - **The fit** is AAA's ([`Rational::aaa`]; Y. Nakatsukasa, O. Sète, L. N. Trefethen, SIAM J.
//!   Sci. Comput. 40, A1494 (2018), [doi:10.1137/16M1106122](https://doi.org/10.1137/16M1106122)):
//!   a rational function in barycentric form (their Eq. 2.5), its support points chosen
//!   greedily among the samples and its weights from the Loewner matrix's smallest singular
//!   vector (Eqs. 3.4–3.9), stopped at 1e-13 relative to the largest sample (p. A1498, Fig. 4.1).
//! - **The error estimate.** The paper measures the error only on the samples, which a fit
//!   through them makes small however wrong it is between them; it gives no estimate between
//!   samples. Here the samples, in order along the range, are split into two interleaved
//!   halves, every other one, and each half is fitted alone. The estimate is twice the largest
//!   difference between the fit to all samples, r, and either half's, r_h, over a grid of
//!   [`Options::points`] across the range and at the real part of every pole of the three fits
//!   that lies in the range (where a fit's spike would be; [`Rational::poles`], their Eq. 3.11);
//!   and at least 1e-13 of the largest sample, AAA's own tolerance.
//! - **Why it bounds the error.** With e = f − r and e_h = f − r_h, the difference
//!   d_h = r − r_h is e_h − e. Wherever r is at least 1.5 times as accurate as one of the halves,
//!   |e| ≤ ⅔|e_h| ≤ ⅔(|e| + |d_h|), so |e| ≤ 2|d_h|: the estimate. A rational fit's error falls
//!   fast with its number of samples (geometrically for a function analytic around the range),
//!   so with twice the samples r is usually far more accurate than either half, not 1.5 times.
//!   That is an argument, not a proof, so it was measured: 200 test functions with known values
//!   (resonances, ring and Fabry–Perot responses, a delay line's phase, poles near the range,
//!   branch points, steps, smooth dispersion), each swept at four tolerances, 800 sweeps
//!   (`sweep/estimate-bounds-error`): the true error was at most 0.51 of the estimate, its median
//!   6e-5 of it. The difference of two successive fits (before and after the newest sample), the
//!   obvious alternative, was tried first and did not bound the error: it fell below it at points
//!   neither fit had sampled.
//! - **Where samples go.** The next sample is the point of the grid or the pole where the
//!   difference is largest; a pole's real part is sampled, which takes a spurious pole out of
//!   the fit (it can't have one at a support point).
//! - **What it can't see.** A feature that no fit has seen: one much narrower than the grid's
//!   spacing, between its points. On Lorentzian resonances over [−1, 1] (grid spacing 0.002)
//!   the estimate held down to half-widths of 0.001; at 0.0003 the true error was 1.11 times it.
//!   A grid that resolves the response (the default 1001 points) is the condition of the
//!   estimate.
//!
//! A sweep of a mode follows one mode ([`Branch`]): each new wavelength's mode is the one,
//! among the solver's candidates, whose fields overlap most with the mode found at the nearest
//! wavelength sampled so far ([`crate::mode::vector::VectorMode::overlap`], as
//! [`crate::mode::dispersion::track`] does along a list of wavelengths), so the fitted branch
//! is the same mode through a crossing, not the n-th by index.

mod aaa;
mod branch;
pub(crate) mod checks;
#[cfg(test)]
mod tests;

pub use aaa::{DEFAULT_TOLERANCE, Rational};
pub use branch::Branch;

use num_complex::Complex64 as c64;

use crate::{Error, Result};

/// How a [`Sweep`] samples and when it stops.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Options {
    /// The error asked for, relative to the largest |value| sampled: the sweep stops when its
    /// estimate is at most this. At least 1e-13, the estimate's floor.
    pub tolerance: f64,
    /// The grid across the range on which the estimate is taken and new samples are chosen,
    /// ends included: it must resolve the response. 1001 by default.
    pub points: usize,
    /// The samples to start with, evenly spaced, ends included, evaluated in order along the
    /// range. 5 by default, at least 3.
    pub initial: usize,
    /// The most samples (solver calls) to take; a sweep that reaches it stops unconverged.
    /// 200 by default.
    pub max_samples: usize,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            tolerance: 1e-6,
            points: 1001,
            initial: 5,
            max_samples: 200,
        }
    }
}

impl Options {
    /// The defaults with the relative `tolerance` asked for.
    pub fn new(tolerance: f64) -> Options {
        Options {
            tolerance,
            ..Options::default()
        }
    }

    /// With a grid of `points` for the estimate and the next samples.
    pub fn with_points(self, points: usize) -> Options {
        Options { points, ..self }
    }

    /// Starting from `initial` evenly spaced samples.
    pub fn with_initial(self, initial: usize) -> Options {
        Options { initial, ..self }
    }

    /// Taking at most `max_samples` samples.
    pub fn with_max_samples(self, max_samples: usize) -> Options {
        Options {
            max_samples,
            ..self
        }
    }

    fn check(&self) -> Result<()> {
        if !(self.tolerance >= FLOOR && self.tolerance.is_finite()) {
            return Err(Error::invalid(
                "sweep",
                format!(
                    "the tolerance must be at least {FLOOR:e}, the estimate's floor, got {}",
                    self.tolerance
                ),
            ));
        }
        if self.initial < 3 || self.points < self.initial || self.max_samples < self.initial {
            return Err(Error::invalid(
                "sweep",
                format!(
                    "needs at least 3 initial samples, and as many grid points and samples at most: got {} initial, {} points, {} samples at most",
                    self.initial, self.points, self.max_samples
                ),
            ));
        }
        Ok(())
    }
}

/// The estimate's floor, relative to the largest sample: AAA's own tolerance (the paper's
/// default, p. A1498), below which the fits' differences are round-off.
const FLOOR: f64 = DEFAULT_TOLERANCE;

/// A response sampled adaptively and fitted by AAA, with its estimated error (the module's
/// documentation).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Sweep {
    /// The range's ends.
    pub range: (f64, f64),
    /// The samples (point, value), in order along the range: one solver call each.
    pub samples: Vec<(f64, c64)>,
    /// The AAA fit to all of them.
    pub fit: Rational,
    /// The estimated error of [`Sweep::fit`], in the value's unit: the largest over the range.
    pub error: f64,
    /// The largest |value| sampled, which the tolerance is relative to.
    pub scale: f64,
    /// The relative tolerance asked for.
    pub tolerance: f64,
    /// Whether the estimate reached the tolerance (not: the sample limit was reached first).
    pub converged: bool,
    /// The relative estimate after each sample from the initial ones on: how it fell.
    pub history: Vec<f64>,
}

impl Sweep {
    /// Samples `f` over `from` to `to` (any variable: a wavelength in µm, a frequency) until the
    /// estimated error of the AAA fit to its samples is at most `options.tolerance` times the
    /// largest |value| sampled, or `options.max_samples` samples are taken.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a range that isn't finite and increasing, options outside
    /// their limits ([`Options`]), or a value from `f` that isn't finite; and `f`'s own errors.
    pub fn run(
        mut f: impl FnMut(f64) -> Result<c64>,
        from: f64,
        to: f64,
        options: &Options,
    ) -> Result<Sweep> {
        if !(from.is_finite() && to.is_finite() && from < to) {
            return Err(Error::invalid(
                "sweep",
                format!("the range must be finite and increasing, got {from} to {to}"),
            ));
        }
        options.check()?;
        let width = to - from;
        let at = |k: usize, n: usize| {
            if k + 1 == n {
                to
            } else {
                from + width * k as f64 / (n - 1) as f64
            }
        };
        let grid: Vec<f64> = (0..options.points).map(|k| at(k, options.points)).collect();
        let mut samples: Vec<(f64, c64)> = Vec::with_capacity(options.max_samples);
        let mut sample = |x: f64, samples: &mut Vec<(f64, c64)>| -> Result<()> {
            let v = f(x)?;
            if !v.is_finite() {
                return Err(Error::invalid(
                    "sweep",
                    format!("the function's value at {x} isn't finite: {v}"),
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
            let relative = if step.scale > 0.0 {
                step.error / step.scale
            } else {
                0.0
            };
            history.push(relative);
            let converged = step.error <= options.tolerance * step.scale;
            let next = step.next.filter(|_| !converged);
            match next {
                Some(x) if samples.len() < options.max_samples => sample(x, &mut samples)?,
                _ => {
                    return Ok(Sweep {
                        range: (from, to),
                        samples,
                        fit: step.fit,
                        error: step.error,
                        scale: step.scale,
                        tolerance: options.tolerance,
                        converged,
                        history,
                    });
                }
            }
        }
    }

    /// The fit at `x`.
    pub fn eval(&self, x: f64) -> c64 {
        self.fit.eval(x)
    }

    /// The estimated error relative to the largest |value| sampled.
    pub fn relative_error(&self) -> f64 {
        if self.scale > 0.0 {
            self.error / self.scale
        } else {
            0.0
        }
    }

    /// The number of samples: the solver calls the sweep made.
    pub fn calls(&self) -> usize {
        self.samples.len()
    }
}

/// One step of a sweep: the fit to all samples, its estimated error, the largest |value|, and
/// where the difference between the fits is largest among the points not yet sampled.
struct Step {
    fit: Rational,
    error: f64,
    scale: f64,
    next: Option<f64>,
}

/// The fit to `samples`, and its error estimate over `grid` and the poles in [from, to] (the
/// module's documentation).
fn estimate(samples: &[(f64, c64)], grid: &[f64], from: f64, to: f64) -> Result<Step> {
    let fit_of = |part: &[&(f64, c64)]| {
        let x: Vec<f64> = part.iter().map(|s| s.0).collect();
        let f: Vec<c64> = part.iter().map(|s| s.1).collect();
        Rational::aaa(&x, &f, DEFAULT_TOLERANCE)
    };
    let all: Vec<&(f64, c64)> = samples.iter().collect();
    let fit = fit_of(&all)?;
    let even: Vec<&(f64, c64)> = samples.iter().step_by(2).collect();
    let odd: Vec<&(f64, c64)> = samples.iter().skip(1).step_by(2).collect();
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
    let scale = samples.iter().map(|s| s.1.norm()).fold(0.0, f64::max);
    let separation = 1e-9 * (to - from);
    let mut largest = 0.0f64;
    let mut next: Option<(f64, f64)> = None;
    for &x in &points {
        let r = fit.eval(x);
        let d = halves
            .iter()
            .map(|h| (r - h.eval(x)).norm())
            .fold(
                0.0,
                |a: f64, b| if b.is_nan() { f64::INFINITY } else { a.max(b) },
            );
        let d = if d.is_finite() { d } else { f64::INFINITY };
        largest = largest.max(d);
        let i = samples.partition_point(|s| s.0 < x);
        let sampled = [i.wrapping_sub(1), i]
            .iter()
            .filter_map(|&j| samples.get(j))
            .any(|s| (s.0 - x).abs() <= separation);
        if !sampled && next.is_none_or(|(_, b)| d > b) {
            next = Some((x, d));
        }
    }
    Ok(Step {
        fit,
        error: (2.0 * largest).max(FLOOR * scale),
        scale,
        next: next.map(|(x, _)| x),
    })
}
