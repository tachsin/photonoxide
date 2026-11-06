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
//! - **The floor on samples that aren't smooth to their last digits.** The fits pass through
//!   the samples to AAA's 1e-13, roughness and all, and the halves' fits part where the
//!   roughness shows: the estimate stops, far above the roughness, and then grows, more samples
//!   not bringing it down (the sweep ends unconverged at [`Options::max_samples`], its
//!   [`Sweep::history`] showing the turn; #630 found spurious poles in the samples' last
//!   digits). Measured (`the_sweeps_floor_on_noisy_samples_study`) on 2 + 0.3 sin 1.5x + 0.1x²
//!   over [−1, 1], which reaches 1e-10 in 28 samples without noise: with a relative noise of
//!   1e-12, 1e-10, 1e-8, 1e-6 and 1e-4 the estimate stopped at 7e-6, 1e-4, 9e-3, 6e-5 and 3e-3,
//!   30 to 7e6 times the noise, and not in its order. On a solver's data
//!   (`the_sweep_on_the_slabs_solver_data_study`): a slab's effective index (0.4 µm of 2.2 in
//!   1.45) from the full-vector solver over 1.50 to 1.60 µm, whose Chebyshev coefficients level
//!   off at 5.5e-16 of it on 10 nm cells and at 9e-15 (3.2e-14 at most) on 2.5 nm cells, reaches
//!   2.5e-11 on the first and stops at 6.2e-9 on the second, growing past it when asked for
//!   1e-10; in #630, on another slab's fine grids, it stopped near 1e-6. A sweep's tolerance
//!   must sit above that floor, which the samples' roughness doesn't predict. For a quantity
//!   that is analytic over the range but known only to a solver's last digits, a polynomial
//!   interpolant at Chebyshev points keeps the round-off as round-off
//!   ([`crate::nonlinear::guide::ModeIndex`]). (These studies run without the stopping rule
//!   below, which is off by default.)
//! - **The stopping rule** (off by default; [`Options::with_stall`]). Once the estimate has gone
//!   [`Options::stall`] samples, and at least half as many again as the sweep had at the time,
//!   without falling below half its least, each step also measures the samples' roughness: for
//!   AAA tolerances from 1e-13 to 1e-1, two a decade, each interleaved half is fitted at the
//!   tolerance and evaluated at the other half's samples, and the root mean square of those
//!   misses, over √2, is the roughness at it. Too large a tolerance misses the response, too
//!   small a one follows the noise between samples; the least is the step's roughness, and its
//!   gain how many times as much the interpolating fits miss. Where the gain is at least 2 and
//!   the roughness at most a tenth of those fits' estimate and of the least estimate so far, that
//!   least at most 10 %, the step's three fits are taken at that tolerance if their estimate is
//!   the smaller, the estimate at least the roughness. At the next stall the steps since the
//!   estimate last made progress decide, not one step (whose gain and roughness jump from one
//!   sample to the next, so that a decision of one step's turned on the samples' last bits):
//!   where a fifth of them have a gain of 2, the least estimate is at least 2.6 times their
//!   median roughness ([`Sweep::noise`]) and at most 10 %, the stall stops the sweep
//!   ([`Sweep::stalled`]) with the fit of its least estimate; else a response not yet resolved,
//!   whose fits miss its samples by about as much as its estimate, is sampled on. On
//!   2 + 0.3 sin 1.5x + 0.1x² with a uniform relative noise of η (its root mean square about
//!   0.5 η of the largest value), asked for 1e-12, 254 of 270 sweeps (30 seeds at each of nine
//!   levels from 1e-12 to 1e-4) stop before their limit of 200, and all 30 of one seed's with its
//!   samples moved by 0 to 29 ulps (`the_stopping_rule_on_noisy_samples_study`; the table in
//!   `docs/methods/sweep.md`): the median roughness is 0.79 η, and the estimate, a median of
//!   106 η, bounds the fit's true error in every case. Without noise it stops none of 2000
//!   sweeps of the test functions, but samples 117 of them otherwise, so it is off unless asked
//!   (`the_stopping_rule_on_noiseless_sweeps_study`). On the slab's solver data on 2.5 nm cells
//!   asked for 1e-10 the roughness is 1.0e-13, far below the estimate's 6.2e-9: that floor is
//!   not the samples' noise, but a quarter of the steps show their roughness, and the sweep stops
//!   at 49 samples with an estimate of 6.2e-9 (7.8 without the rule).
//!
//! A sweep of a mode follows one mode ([`Branch`]): each new wavelength's mode is the one,
//! among the solver's candidates, whose fields overlap most with the mode found at the nearest
//! wavelength sampled so far ([`crate::mode::vector::VectorMode::overlap`], as
//! [`crate::mode::dispersion::track`] does along a list of wavelengths), so the fitted branch
//! is the same mode through a crossing, not the n-th by index. Through an anticrossing of two
//! modes of one symmetry it may follow either, depending on where the samples fall; its
//! [`Branch::steps`] show where it had to choose ([`Branch`], Anticrossings).

mod aaa;
mod branch;
pub(crate) mod checks;
#[cfg(test)]
mod tests;

pub use aaa::{DEFAULT_TOLERANCE, Rational};
pub use branch::{Branch, BranchStep};

use num_complex::Complex64 as c64;

use crate::{Error, Result};

/// How a [`Sweep`] samples and when it stops.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Options {
    /// The error asked for, relative to the largest |value| sampled: the sweep stops when its
    /// estimate is at most this. At least 1e-13, the estimate's floor on samples smooth to
    /// their last digits; samples that aren't raise the floor (the module's documentation:
    /// 6.2e-9 on a slab's effective index on 2.5 nm cells).
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
    /// The stopping rule's patience: after this many samples (and at least half as many again
    /// as the sweep had then) without its estimate falling below half its least, the sweep
    /// measures its samples' roughness at every step, fits above it where it shows noise, and
    /// stops when the estimate stalls again with noise shown ([`Sweep::stalled`], the module's
    /// documentation). 0, the default, turns the rule off: the fits interpolate to 1e-13
    /// throughout, up to `max_samples`. 10 is what the measured cases use; for samples a solver
    /// gives with noise in their last digits.
    pub stall: usize,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            tolerance: 1e-6,
            points: 1001,
            initial: 5,
            max_samples: 200,
            stall: 0,
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

    /// With the stopping rule, after `stall` samples without progress ([`Options::stall`]; 0,
    /// the default, for none).
    pub fn with_stall(self, stall: usize) -> Options {
        Options { stall, ..self }
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
/// default, p. A1498), below which the fits' differences are round-off. It holds for samples
/// smooth to their last digits; noisier ones raise it (the module's documentation).
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
    /// Whether the sweep stopped at its samples' noise ([`Options::stall`]): its estimate had
    /// stopped falling with the noise shown, so more samples wouldn't have brought it to the
    /// tolerance. The fit is then the one of its least estimate.
    pub stalled: bool,
    /// Once the estimate first stalled, the samples' roughness, relative to the largest |value|:
    /// at each step the root mean square by which each interleaved half's fit misses the other
    /// half's samples, at the AAA tolerance where that is least, over √2 (each miss holds two
    /// samples' noise), and this the median over the steps since the estimate last made
    /// progress (one step's varies by a factor of four). Where it showed noise the estimate is at
    /// least a step's roughness, and a tolerance below it can't be met. `None` for a sweep that
    /// never stalled.
    pub noise: Option<f64>,
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
        // once the estimate has stalled: whether the fits above the roughness showed noise at a
        // step, the watched steps since the estimate last made progress (each one's roughness
        // and gain), and their median roughness (the stopping rule, the module's documentation)
        let (mut watching, mut seen) = (false, false);
        let mut window: Vec<(f64, f64)> = Vec::new();
        let mut noise: Option<f64> = None;
        // the least estimate and its step, and the samples when the estimate last fell below
        // half of what it had been
        let mut least: Option<(f64, Step)> = None;
        let mut progress = samples.len();
        loop {
            let mut step = estimate(&samples, &grid, from, to, DEFAULT_TOLERANCE)?;
            if watching {
                let (rough, gain, refit) = refit_above_roughness(&samples, &grid, from, to)?;
                window.push((rough, gain));
                noise = Some(median_roughness(&window));
                // this step's fits above the interpolating ones are taken where they miss the
                // held-out samples at most half as much, by a tenth at most of their estimate
                // and of the least estimate so far, that least at most 10 %, and their
                // estimate is the smaller
                let least_so_far = least.as_ref().map_or(f64::INFINITY, |l| l.0);
                let shown = gain >= GAIN
                    && least_so_far <= RESOLVED
                    && 10.0 * rough <= relative_to(refit.error, refit.scale).min(least_so_far);
                if shown && refit.error < step.error {
                    step = refit;
                }
                seen |= shown;
            }
            let relative = relative_to(step.error, step.scale);
            history.push(relative);
            if least.as_ref().is_none_or(|(l, _)| relative < 0.5 * *l) {
                progress = samples.len();
                window.clear();
            }
            if least.as_ref().is_none_or(|(l, _)| relative < *l) {
                least = Some((relative, step.clone()));
            }
            let converged = step.error <= options.tolerance * step.scale;
            let next = step.next.filter(|_| !converged);
            let stuck = options.stall > 0
                && next.is_some()
                && samples.len() - progress >= options.stall.max(progress / 2);
            if stuck && !watching {
                // these samples taken again with the fits above their roughness
                watching = true;
                progress = samples.len();
                history.pop();
                continue;
            }
            // a stall stops the sweep where its steps since the last progress showed noise;
            // else a response not yet resolved, sampled on, the steps counted afresh
            let stalled =
                stuck && noise_shown(&window, least.as_ref().map_or(f64::INFINITY, |l| l.0));
            if stuck && !stalled {
                progress = samples.len();
                window.clear();
            }
            seen |= stalled;
            match next {
                Some(x) if samples.len() < options.max_samples && !stalled => {
                    sample(x, &mut samples)?;
                }
                _ => {
                    // once noise was shown, the least estimate's fit: the later ones are no
                    // better
                    let best = match least.take() {
                        Some((_, l)) if seen => l,
                        _ => step,
                    };
                    return Ok(Sweep {
                        range: (from, to),
                        converged: best.error <= options.tolerance * best.scale,
                        fit: best.fit,
                        error: best.error,
                        scale: best.scale,
                        samples,
                        tolerance: options.tolerance,
                        history,
                        stalled,
                        noise,
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

fn relative_to(error: f64, scale: f64) -> f64 {
    if scale > 0.0 { error / scale } else { 0.0 }
}

/// A step shows noise where the interpolating fits miss the held-out samples at least this many
/// times as much as the best fits above them ([`refit_above_roughness`]'s gain).
const GAIN: f64 = 2.0;

/// The least estimate, relative, a sweep must have reached before its stall can be noise: a
/// sweep that hasn't come within 10 % of its response hasn't resolved it, whatever its fits miss.
const RESOLVED: f64 = 0.1;

/// The least share of a stalled sweep's steps since its last progress that must show noise
/// (a gain of at least [`GAIN`]) for it to stop: 0.2, the geometric mean of the largest share
/// at the stalls of 2000 noiseless sweeps, 0.09, and the median at the stalls of 150 noisy ones
/// at their noise, 0.45 (the module's documentation).
const NOISE_SHARE: f64 = 0.2;

/// How many times the median roughness of those steps the least estimate must be for a stall to
/// be noise: 2.6, the geometric mean of the largest ratio at a noiseless sweep's stall, 1.79 (a
/// response not yet resolved misses its held-out samples by about as much as its estimate), and
/// the least at the noisy sweeps' stalls at their noise, 3.87 (the module's documentation).
const NOISE_RATIO: f64 = 2.6;

/// The median roughness of a stalled sweep's watched steps (`(roughness, gain)` each): a step's
/// roughness varies by a factor of four from one sample to the next, its median far less.
fn median_roughness(window: &[(f64, f64)]) -> f64 {
    let mut r: Vec<f64> = window.iter().map(|w| w.0).collect();
    r.sort_by(f64::total_cmp);
    r.get(r.len() / 2).copied().unwrap_or(f64::NAN)
}

/// Whether a stalled sweep's watched steps since its last progress show noise: at least
/// [`NOISE_SHARE`] of them a gain of [`GAIN`], their median roughness at most 1/[`NOISE_RATIO`]
/// of the `least` estimate, and that at most [`RESOLVED`]. Each step's gain and roughness jump
/// from one sample to the next (the gain between 1 and 96, the roughness by a factor of four,
/// on one sweep): the decision is the window's, not one step's, so that it doesn't turn on the
/// samples' last bits.
fn noise_shown(window: &[(f64, f64)], least: f64) -> bool {
    if window.is_empty() {
        return false;
    }
    let showing = window.iter().filter(|w| w.1 >= GAIN).count();
    least <= RESOLVED
        && NOISE_RATIO * median_roughness(window) <= least
        && showing as f64 >= NOISE_SHARE * window.len() as f64
}

/// The AAA tolerances a stalled sweep tries for its fits: 1e-13 to 1e-1, two a decade.
fn tolerances() -> impl Iterator<Item = f64> {
    (0..=24).map(|k| DEFAULT_TOLERANCE * 10f64.powf(f64::from(k) / 2.0))
}

/// The samples' roughness and the sweep's fit above it, for a sweep that has stalled: for each
/// AAA tolerance of [`tolerances`], each interleaved half is fitted at it and evaluated at the
/// other half's samples; the root mean square of those misses, over √2, is the roughness at
/// that tolerance. At a tolerance too large the fits miss the response itself, at one too small
/// they follow the noise between samples; the least is the samples' roughness (relative to the
/// largest |value|). The sweep's fit and estimate are then taken again at that tolerance, the
/// estimate at least the roughness.
fn refit_above_roughness(
    samples: &[(f64, c64)],
    grid: &[f64],
    from: f64,
    to: f64,
) -> Result<(f64, f64, Step)> {
    let scale = samples.iter().map(|s| s.1.norm()).fold(0.0, f64::max);
    let halves: [Vec<&(f64, c64)>; 2] = [
        samples.iter().step_by(2).collect(),
        samples.iter().skip(1).step_by(2).collect(),
    ];
    let mut best: Option<(f64, f64)> = None;
    let mut interpolating = f64::INFINITY;
    for tolerance in tolerances() {
        let mut sq = 0.0;
        for (k, fitted) in halves.iter().enumerate() {
            let x: Vec<f64> = fitted.iter().map(|s| s.0).collect();
            let f: Vec<c64> = fitted.iter().map(|s| s.1).collect();
            let r = Rational::aaa(&x, &f, tolerance)?;
            sq += halves[1 - k]
                .iter()
                .map(|s| (s.1 - r.eval(s.0)).norm_sqr())
                .sum::<f64>();
        }
        let miss = (sq / samples.len() as f64).sqrt();
        let miss = if miss.is_finite() {
            miss
        } else {
            f64::INFINITY
        };
        if tolerance == DEFAULT_TOLERANCE {
            interpolating = miss;
        }
        if best.is_none_or(|(_, b)| miss < b) {
            best = Some((tolerance, miss));
        }
    }
    let (tolerance, miss) = best.expect("a tolerance was tried");
    let noise = relative_to(miss / std::f64::consts::SQRT_2, scale);
    let gain = if miss > 0.0 {
        interpolating / miss
    } else {
        1.0
    };
    let mut step = estimate(samples, grid, from, to, tolerance)?;
    if gain >= GAIN {
        // noise shown: the fits pass through noisy samples, so their error is at least it
        step.error = step.error.max(noise * scale);
    }
    Ok((noise, gain, step))
}

/// One step of a sweep: the fit to all samples, its estimated error, the largest |value|, and
/// where the difference between the fits is largest among the points not yet sampled.
#[derive(Clone)]
struct Step {
    fit: Rational,
    error: f64,
    scale: f64,
    next: Option<f64>,
}

/// The fit to `samples` at the AAA `tolerance`, and its error estimate over `grid` and the
/// poles in [from, to] (the module's documentation).
fn estimate(
    samples: &[(f64, c64)],
    grid: &[f64],
    from: f64,
    to: f64,
    tolerance: f64,
) -> Result<Step> {
    let fit_of = |part: &[&(f64, c64)]| {
        let x: Vec<f64> = part.iter().map(|s| s.0).collect();
        let f: Vec<c64> = part.iter().map(|s| s.1).collect();
        Rational::aaa(&x, &f, tolerance)
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
