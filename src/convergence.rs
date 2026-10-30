//! Grid convergence: the gate a number computed on a grid passes before it is reported.
//!
//! ROADMAP's pitfalls: "a result quoted on one grid changed by 0.07 on the next". The rule is
//! that grid convergence gates every reported number: a number from a grid solve is reported
//! with its grid and a check of that grid, the same quantity solved on other grids, and is
//! flagged (or refused) when the check fails or wasn't made.
//!
//! A [`Study`] holds one quantity on grids each finer than the last; the reported number is the
//! finest grid's. [`Study::check`] estimates that number's error and compares it with the
//! tolerance given, into a [`Convergence`]:
//!
//! - **one grid:** [`Verdict::Unchecked`], with no estimate;
//! - **two grids,** spacings h and h/r: the first-order estimate |Δ|/(r − 1), Δ the last
//!   change. Every discretization here converges at least at first order (each has its
//!   convergence-order test), so for r = 2 the estimate is the change itself, and an upper bound
//!   once the grids are in the asymptotic range;
//! - **three grids or more,** the last three in one ratio r: the changes' ratio q = Δ₁/Δ₂.
//!   Above 1 the grids converge at the observed order p = ln q / ln r, and the estimate is
//!   Richardson's, |v − v∞| = |Δ₂|/(q − 1) ([`richardson`], whose limit
//!   v∞ doesn't depend on r). Below 0 the values oscillate, and the estimate is the larger of
//!   the last two changes. Between 0 and 1 the changes don't shrink: the grids
//!   don't show convergence, and the verdict is [`Verdict::NotConverged`] whatever the
//!   tolerance. Three grids in different ratios are taken as the last two.
//!
//! The verdict is [`Verdict::Converged`] when the estimate is at most the tolerance.
//! [`Convergence::gated`] gives the number only then. docs/methods/convergence.md has where the
//! gate applies (a modes job's `[task.convergence]`, `results.json`, the examples' checks) and
//! what it doesn't cover yet.
//!
//! [`ToTolerance`] drives a solve to a tolerance instead of checking the grids it was given: it
//! halves the grid's spacing (or doubles a count, such as an expansion's modes or steps) until
//! the quantity changes by less than the tolerance, with Richardson's extrapolation where the
//! scheme's order is known, and records every grid and its value in a [`Refined`]. It refuses,
//! with an error naming the quantity, its last two values and the tolerance, when the budget of
//! grids runs out first. [`ToTolerance::eme`] does it for eigenmode expansion in its two
//! parameters, the number of modes and the number of steps, separately and then together.
//!
//! L. F. Richardson, Philos. Trans. R. Soc. A 210, 307 (1911), doi:10.1098/rsta.1911.0009 (the
//! extrapolation); P. J. Roache, Annu. Rev. Fluid Mech. 29, 123 (1997),
//! doi:10.1146/annurev.fluid.29.1.123 (grid convergence studies and their estimates).

use num_complex::Complex64;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub(crate) mod checks;

/// Richardson extrapolation from results on three grids, each half the spacing of the last:
/// the convergence order p is fitted from the real parts, (v₀ − v₁)/(v₁ − v₂) = 2^p, and the
/// limit is v₂ − (v₁ − v₂)/(2^p − 1), for real and imaginary parts alike. Returns the limit and p.
/// Meaningful once the three are in the asymptotic range (the ratio steady).
pub fn richardson(values: [Complex64; 3]) -> (Complex64, f64) {
    let ratio = (values[0].re - values[1].re) / (values[1].re - values[2].re);
    let order = ratio.log2();
    (values[2] - (values[1] - values[2]) / (ratio - 1.0), order)
}

/// A quantity computed on grids each finer than the last: the grid study behind a number.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Study {
    /// What it is, e.g. `"n_eff of mode 1 of 2"`.
    pub quantity: String,
    /// Each grid's spacing, µm, coarsest first.
    pub steps_um: Vec<f64>,
    /// The quantity on each grid.
    pub values: Vec<f64>,
}

/// Whether a number's grid has converged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Verdict {
    /// Its estimated error is within the tolerance.
    Converged,
    /// Its estimated error is above the tolerance, or the grids don't show convergence.
    NotConverged,
    /// It was computed on one grid: no check was made.
    Unchecked,
}

impl Verdict {
    /// Its name, as a record writes it: `"converged"`, `"not_converged"` or `"unchecked"`.
    pub fn name(self) -> &'static str {
        match self {
            Verdict::Converged => "converged",
            Verdict::NotConverged => "not_converged",
            Verdict::Unchecked => "unchecked",
        }
    }
}

/// A number with its grid and the check of that grid ([`Study::check`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct Convergence {
    /// What it is.
    pub quantity: String,
    /// The number: the finest grid's value.
    pub value: f64,
    /// The finest grid's spacing, µm.
    pub step_um: f64,
    /// Each grid's spacing, coarsest first.
    pub steps_um: Vec<f64>,
    /// The quantity on each grid.
    pub values: Vec<f64>,
    /// The tolerance on its error; `None` unchecked.
    pub tolerance: Option<f64>,
    /// Its estimated error; `None` on one grid, or where the grids don't show convergence.
    pub error: Option<f64>,
    /// The observed order of convergence, from three grids converging monotonically.
    pub order: Option<f64>,
    /// Richardson's extrapolated limit, from the same three grids.
    pub limit: Option<f64>,
    /// The verdict.
    pub verdict: Verdict,
    /// The verdict in words.
    pub reason: String,
}

impl Study {
    /// A study of `quantity`, with no grid yet.
    pub fn new(quantity: &str) -> Study {
        Study {
            quantity: quantity.to_owned(),
            steps_um: Vec::new(),
            values: Vec::new(),
        }
    }

    /// The study with `value` on a grid of spacing `step_um`, finer than the last.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a spacing that isn't positive or isn't finer than the last
    /// one, and a value that isn't a number.
    pub fn on(mut self, step_um: f64, value: f64) -> Result<Study> {
        if !(step_um.is_finite() && step_um > 0.0) {
            return Err(Error::invalid(
                "grid study",
                format!("a grid's spacing is above 0, got {step_um}"),
            ));
        }
        if let Some(&last) = self.steps_um.last()
            && step_um >= last
        {
            return Err(Error::invalid(
                "grid study",
                format!("each grid is finer than the last: {step_um} um after {last} um"),
            ));
        }
        if !value.is_finite() {
            return Err(Error::invalid(
                "grid study",
                format!("{} is not a number on the {step_um} um grid", self.quantity),
            ));
        }
        self.steps_um.push(step_um);
        self.values.push(value);
        Ok(self)
    }

    /// The finest grid's value checked against `tolerance` on its error, as the module says.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a study with no grid, and a tolerance that isn't above 0.
    pub fn check(&self, tolerance: f64) -> Result<Convergence> {
        if !(tolerance.is_finite() && tolerance > 0.0) {
            return Err(Error::invalid(
                "grid study",
                format!("a tolerance is above 0, got {tolerance}"),
            ));
        }
        let n = self.values.len();
        let (Some(&value), Some(&step)) = (self.values.last(), self.steps_um.last()) else {
            return Err(Error::invalid(
                "grid study",
                format!("{} has no grid", self.quantity),
            ));
        };
        let mut c = Convergence {
            quantity: self.quantity.clone(),
            value,
            step_um: step,
            steps_um: self.steps_um.clone(),
            values: self.values.clone(),
            tolerance: Some(tolerance),
            error: None,
            order: None,
            limit: None,
            verdict: Verdict::Unchecked,
            reason: format!("one grid ({step} um): no check"),
        };
        if n < 2 {
            return Ok(c);
        }
        let (v, h) = (&self.values, &self.steps_um);
        let r = h[n - 2] / h[n - 1];
        let d2 = v[n - 1] - v[n - 2];
        let error;
        if n >= 3 && same_ratio(h[n - 3] / h[n - 2], r) && d2 != 0.0 {
            let d1 = v[n - 2] - v[n - 3];
            let q = d1 / d2;
            if q > 1.0 {
                let three = [v[n - 3], v[n - 2], v[n - 1]].map(|x| Complex64::new(x, 0.0));
                let (limit, _) = richardson(three);
                c.limit = Some(limit.re);
                c.order = Some(q.ln() / r.ln());
                error = (value - limit.re).abs();
                c.reason = format!(
                    "three grids converging at order {:.2}: Richardson's estimate {error:.3e}",
                    q.ln() / r.ln()
                );
            } else if q < 0.0 {
                error = d1.abs().max(d2.abs());
                c.reason = format!("three grids oscillating: the larger change {error:.3e}");
            } else {
                c.verdict = Verdict::NotConverged;
                c.reason = format!(
                    "the grids don't show convergence: the last change {:.3e} after {:.3e}",
                    d2.abs(),
                    d1.abs()
                );
                return Ok(c);
            }
        } else {
            error = d2.abs() / (r - 1.0);
            c.reason = format!(
                "two grids, at least first order: {:.3e} changed by refining {r:.3}-fold",
                d2.abs()
            );
        }
        c.error = Some(error);
        if error <= tolerance {
            c.verdict = Verdict::Converged;
            c.reason = format!("converged: {}, within {tolerance:e}", c.reason);
        } else {
            c.verdict = Verdict::NotConverged;
            c.reason = format!("not converged: {}, above {tolerance:e}", c.reason);
        }
        Ok(c)
    }
}

/// Whether two refinement ratios are the same.
fn same_ratio(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * b
}

impl Convergence {
    /// A number on one grid, with no check: [`Verdict::Unchecked`].
    pub fn unchecked(quantity: &str, step_um: f64, value: f64) -> Convergence {
        Convergence {
            quantity: quantity.to_owned(),
            value,
            step_um,
            steps_um: vec![step_um],
            values: vec![value],
            tolerance: None,
            error: None,
            order: None,
            limit: None,
            verdict: Verdict::Unchecked,
            reason: format!("one grid ({step_um} um): no check"),
        }
    }

    /// Whether its grid has converged.
    pub fn converged(&self) -> bool {
        self.verdict == Verdict::Converged
    }

    /// The number, once its grid has converged.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a number whose grid hasn't converged or wasn't checked,
    /// saying why.
    pub fn gated(&self) -> Result<f64> {
        if self.converged() {
            return Ok(self.value);
        }
        Err(Error::invalid(
            "grid convergence",
            format!(
                "{} = {} on a {} um grid is not reported: {}",
                self.quantity, self.value, self.step_um, self.reason
            ),
        ))
    }
}

/// A tolerance on a quantity's change from one grid to the next, and on its estimated error.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Tolerance {
    /// In the quantity's own unit.
    Absolute(f64),
    /// Relative to the quantity's value.
    Relative(f64),
}

impl Tolerance {
    /// In the quantity's own unit, at `value`.
    fn at(self, value: f64) -> f64 {
        match self {
            Tolerance::Absolute(t) => t,
            Tolerance::Relative(t) => t * value.abs(),
        }
    }

    fn size(self) -> f64 {
        match self {
            Tolerance::Absolute(t) | Tolerance::Relative(t) => t,
        }
    }
}

impl std::fmt::Display for Tolerance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tolerance::Absolute(t) => write!(f, "{t:e}"),
            Tolerance::Relative(t) => write!(f, "{t:e} relative"),
        }
    }
}

/// A quantity driven to a tolerance ([`ToTolerance`]): its value, the estimate of its remaining
/// error, and every grid it was solved on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Refined {
    /// What it is.
    pub quantity: String,
    /// The number: Richardson's limit of the last two grids where the order is known, the
    /// finest grid's value where it isn't.
    pub value: f64,
    /// The estimate of `value`'s remaining error, in the quantity's unit: with the order known,
    /// the change of Richardson's limit from the previous pair of grids (on two grids, the
    /// finest grid's own estimate, |Δ|/(2^p − 1)); without it, the gate's ([`Study::check`]).
    /// `None` where the grids don't show convergence.
    pub error: Option<f64>,
    /// The tolerance it was driven to, in the quantity's unit (at its value, for a relative
    /// one).
    pub tolerance: f64,
    /// The scheme's order, where it is known and was used for Richardson's extrapolation.
    pub order: Option<f64>,
    /// The order the last three grids show, ln(Δ₁/Δ₂)/ln 2, where their changes shrink
    /// without changing sign.
    pub observed_order: Option<f64>,
    /// Each grid's spacing, coarsest first: µm for a grid, 1/N for a count N (an expansion's
    /// modes or steps).
    pub grids: Vec<f64>,
    /// The quantity on each grid.
    pub values: Vec<f64>,
    /// Whether it reached the tolerance: the last change and the estimate both within it.
    pub reached: bool,
    /// How it ended, in words.
    pub reason: String,
}

impl Refined {
    /// The last change between grids, Δ.
    pub fn last_change(&self) -> Option<f64> {
        let n = self.values.len();
        (n >= 2).then(|| self.values[n - 1] - self.values[n - 2])
    }

    /// Itself once it reached its tolerance; the refusal otherwise: an error naming the
    /// quantity, its last two values and grids, the tolerance, and every grid.
    fn required(self, budget: usize) -> Result<Refined> {
        if self.reached {
            return Ok(self);
        }
        let n = self.values.len();
        let last_two = if n >= 2 {
            format!(
                "its last two values {} (grid {}) and {} (grid {}) differ by {:.3e}",
                self.values[n - 2],
                self.grids[n - 2],
                self.values[n - 1],
                self.grids[n - 1],
                (self.values[n - 1] - self.values[n - 2]).abs()
            )
        } else {
            format!("it has one value, {}", self.values[0])
        };
        let grids: Vec<String> = self
            .grids
            .iter()
            .zip(&self.values)
            .map(|(h, v)| format!("{h}: {v}"))
            .collect();
        Err(Error::invalid(
            "convergence to a tolerance",
            format!(
                "{} did not reach the tolerance {:e} within {budget} grids: {last_two}; {} \
                 (grids: {})",
                self.quantity,
                self.tolerance,
                self.reason,
                grids.join(", ")
            ),
        ))
    }

    /// Its estimate and verdict from the grids so far, at `tolerance`.
    fn assess(&mut self, tolerance: Tolerance) {
        let n = self.values.len();
        let (v, h) = (&self.values, &self.grids);
        let d2 = v[n - 1] - v[n - 2];
        self.observed_order = (n >= 3)
            .then(|| (v[n - 2] - v[n - 3]) / d2)
            .filter(|q| q.is_finite() && *q > 1.0)
            .map(|q| q.ln() / (h[n - 2] / h[n - 1]).ln());
        match self.order {
            Some(p) => {
                let f = (h[n - 2] / h[n - 1]).powf(p) - 1.0;
                let limit = |k: usize| v[k] + (v[k] - v[k - 1]) / f;
                self.value = limit(n - 1);
                self.error = Some(if n >= 3 {
                    (limit(n - 1) - limit(n - 2)).abs()
                } else {
                    d2.abs() / f
                });
            }
            None => {
                self.value = v[n - 1];
                let study = Study {
                    quantity: self.quantity.clone(),
                    steps_um: h.clone(),
                    values: v.clone(),
                };
                // the gate's estimate; its tolerance only names the verdict, unused here
                self.error = study.check(f64::MAX).ok().and_then(|c| c.error);
            }
        }
        self.tolerance = tolerance.at(self.value);
        let error = self.error.unwrap_or(f64::INFINITY);
        self.reached = d2.abs() <= self.tolerance && error <= self.tolerance;
        let order = match (self.order, self.observed_order) {
            (Some(p), Some(o)) => format!("Richardson at order {p} (observed {o:.2})"),
            (Some(p), None) => format!("Richardson at order {p}"),
            (None, Some(o)) => format!("the gate's estimate (observed order {o:.2})"),
            (None, None) => "the gate's estimate".to_owned(),
        };
        let estimate = self
            .error
            .map_or("no estimate: the changes don't shrink".to_owned(), |e| {
                format!("estimated error {e:.3e}")
            });
        self.reason = format!(
            "{} on {n} grids: the last change {:.3e}, {order}, {estimate}, against {:e}",
            if self.reached {
                "reached"
            } else {
                "not reached"
            },
            d2.abs(),
            self.tolerance
        );
    }
}

/// A quantity driven to a tolerance by eigenmode expansion ([`ToTolerance::eme`]): in its
/// number of modes, its number of steps, and both together.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmeRefined {
    /// What it is.
    pub quantity: String,
    /// The number: the together study's value at its modes and steps, plus the steps study's
    /// Richardson correction.
    pub value: f64,
    /// The estimate of its remaining error: the steps study's plus the together study's.
    pub error: f64,
    /// The tolerance it was driven to, in the quantity's unit.
    pub tolerance: f64,
    /// The modes that the together study ended with.
    pub modes: usize,
    /// The steps that the steps study ended with.
    pub steps: usize,
    /// The number of modes doubled at the first number of steps.
    pub mode_study: Refined,
    /// The number of steps doubled at the mode study's last number of modes, at second order
    /// (each step the cross-section at its middle: the midpoint rule).
    pub step_study: Refined,
    /// The number of modes doubled again at the steps study's last number of steps, from half
    /// the mode study's last, to what the steps' error leaves of the tolerance.
    pub together: Refined,
}

/// A driver that refines a solve until its quantity changes by less than a tolerance.
///
/// Each grid halves the last one's spacing (or doubles a count). Once there are two grids, the
/// driver stops when the last change |Δ| and the estimate of the remaining error are both
/// within the tolerance. With the scheme's order p known ([`ToTolerance::with_order`]), the
/// value reported is Richardson's limit from the last two grids, v + Δ/(2^p − 1), and its
/// estimated error is how much that limit moved from the previous pair's; without it, the
/// finest grid's value with the gate's estimate ([`Study::check`]). Every grid is recorded.
/// When the budget of grids runs out first, it refuses with an error naming the quantity, its
/// last two values and the tolerance.
#[derive(Clone, Debug, PartialEq)]
pub struct ToTolerance {
    quantity: String,
    tolerance: Tolerance,
    order: Option<f64>,
    budget: usize,
}

impl ToTolerance {
    /// A driver of `quantity` to `tolerance`, the order unknown, with a budget of 8 grids.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a tolerance that isn't above 0.
    pub fn new(quantity: &str, tolerance: Tolerance) -> Result<ToTolerance> {
        let t = tolerance.size();
        if !(t.is_finite() && t > 0.0) {
            return Err(Error::invalid(
                "convergence to a tolerance",
                format!("a tolerance is above 0, got {t}"),
            ));
        }
        Ok(ToTolerance {
            quantity: quantity.to_owned(),
            tolerance,
            order: None,
            budget: 8,
        })
    }

    /// With the scheme's order known, `order`: Richardson's extrapolation at that order.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an order that isn't above 0.
    pub fn with_order(mut self, order: f64) -> Result<ToTolerance> {
        if !(order.is_finite() && order > 0.0) {
            return Err(Error::invalid(
                "convergence to a tolerance",
                format!("an order is above 0, got {order}"),
            ));
        }
        self.order = Some(order);
        Ok(self)
    }

    /// With at most `grids` grids (in each study, for [`ToTolerance::eme`]).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for fewer than 2 or more than 30.
    pub fn with_budget(mut self, grids: usize) -> Result<ToTolerance> {
        if !(2..=30).contains(&grids) {
            return Err(Error::invalid(
                "convergence to a tolerance",
                format!("a budget is from 2 to 30 grids, got {grids}"),
            ));
        }
        self.budget = grids;
        Ok(self)
    }

    /// The quantity `solve(h)` on grids of spacing `start`, `start`/2, `start`/4, … (µm) until
    /// it reaches the tolerance.
    ///
    /// # Errors
    ///
    /// The refusal ([`Error::InvalidValue`]) when the budget runs out first, naming the
    /// quantity, its last two values and the tolerance; a spacing that isn't above 0; a value
    /// that isn't a number; and `solve`'s own errors.
    pub fn grids(&self, start: f64, mut solve: impl FnMut(f64) -> Result<f64>) -> Result<Refined> {
        if !(start.is_finite() && start > 0.0) {
            return Err(Error::invalid(
                "convergence to a tolerance",
                format!("a grid's spacing is above 0, got {start}"),
            ));
        }
        let h = |k: usize| start / f64::from(1u32 << k);
        self.drive(self.order, self.tolerance, h, |k| solve(h(k)))?
            .required(self.budget)
    }

    /// The quantity `solve(n)` for the counts n = `start`, 2`start`, 4`start`, … (a number of
    /// modes or steps; recorded as the spacing 1/n) until it reaches the tolerance.
    ///
    /// # Errors
    ///
    /// As [`ToTolerance::grids`], and for a count of 0.
    pub fn counts(
        &self,
        start: usize,
        mut solve: impl FnMut(usize) -> Result<f64>,
    ) -> Result<Refined> {
        if start == 0 {
            return Err(Error::invalid(
                "convergence to a tolerance",
                "a count is at least 1",
            ));
        }
        let n = |k: usize| start << k;
        self.drive(
            self.order,
            self.tolerance,
            |k| 1.0 / n(k) as f64,
            |k| solve(n(k)),
        )?
        .required(self.budget)
    }

    /// Eigenmode expansion's quantity `solve(modes, steps)` driven to the tolerance in both of
    /// its parameters: the modes doubled from `modes` at `steps` steps, the order unknown (a
    /// basis truncated, not a grid); then the steps doubled from `steps` at the modes that
    /// study ended with, at second order (each step is the cross-section at its middle, the
    /// midpoint rule); then together, the modes doubled again from half that number at the
    /// steps the second study ended with, to what the steps' error leaves of the tolerance. The
    /// value is the last study's, with the steps' Richardson correction, and its error the two
    /// estimates' sum. A value already solved isn't solved again. The order set on the driver
    /// isn't used.
    ///
    /// # Errors
    ///
    /// As [`ToTolerance::counts`], for each study.
    pub fn eme(
        &self,
        modes: usize,
        steps: usize,
        mut solve: impl FnMut(usize, usize) -> Result<f64>,
    ) -> Result<EmeRefined> {
        if modes == 0 || steps == 0 {
            return Err(Error::invalid(
                "convergence to a tolerance",
                "an expansion has at least 1 mode and 1 step",
            ));
        }
        let mut solved = std::collections::HashMap::<(usize, usize), f64>::new();
        let mut value = |m: usize, s: usize| -> Result<f64> {
            if let Some(&v) = solved.get(&(m, s)) {
                return Ok(v);
            }
            let v = solve(m, s)?;
            solved.insert((m, s), v);
            Ok(v)
        };
        let named = |what: &str| ToTolerance {
            quantity: format!("{} ({what})", self.quantity),
            ..self.clone()
        };
        let count = |r: &Refined| (1.0 / r.grids[r.grids.len() - 1]).round() as usize;
        let mode_study = named(&format!("in modes, at {steps} steps"))
            .drive(
                None,
                self.tolerance,
                |k| 1.0 / (modes << k) as f64,
                |k| value(modes << k, steps),
            )?
            .required(self.budget)?;
        let m1 = count(&mode_study);
        let step_study = named(&format!("in steps, at {m1} modes"))
            .drive(
                Some(2.0),
                self.tolerance,
                |k| 1.0 / (steps << k) as f64,
                |k| value(m1, steps << k),
            )?
            .required(self.budget)?;
        let s1 = count(&step_study);
        let correction = step_study.value - step_study.values[step_study.values.len() - 1];
        let steps_error = step_study.error.unwrap_or(f64::INFINITY);
        let left = step_study.tolerance - steps_error;
        let from = (m1 / 2).max(modes);
        if left.is_nan() || left <= 0.0 {
            return Err(Error::invalid(
                "convergence to a tolerance",
                format!(
                    "{} did not reach the tolerance {:e}: the steps' estimated error {:e} leaves \
                     none of it for the modes",
                    self.quantity, step_study.tolerance, steps_error
                ),
            ));
        }
        let together = named(&format!("in modes, at {s1} steps"))
            .drive(
                None,
                Tolerance::Absolute(left),
                |k| 1.0 / (from << k) as f64,
                |k| value(from << k, s1),
            )?
            .required(self.budget)?;
        let error = steps_error + together.error.unwrap_or(f64::INFINITY);
        Ok(EmeRefined {
            quantity: self.quantity.clone(),
            value: together.value + correction,
            error,
            tolerance: step_study.tolerance,
            modes: count(&together),
            steps: s1,
            mode_study,
            step_study,
            together,
        })
    }

    /// The study on grids `spacing(k)`, k = 0, 1, …, of values `solve(k)`, until the tolerance
    /// or the budget: the record either way.
    fn drive(
        &self,
        order: Option<f64>,
        tolerance: Tolerance,
        spacing: impl Fn(usize) -> f64,
        mut solve: impl FnMut(usize) -> Result<f64>,
    ) -> Result<Refined> {
        let mut r = Refined {
            quantity: self.quantity.clone(),
            value: f64::NAN,
            error: None,
            tolerance: f64::NAN,
            order,
            observed_order: None,
            grids: Vec::new(),
            values: Vec::new(),
            reached: false,
            reason: String::new(),
        };
        for k in 0..self.budget {
            let h = spacing(k);
            let v = solve(k)?;
            if !v.is_finite() {
                return Err(Error::invalid(
                    "convergence to a tolerance",
                    format!("{} is not a number on grid {h}", self.quantity),
                ));
            }
            r.grids.push(h);
            r.values.push(v);
            if k == 0 {
                r.value = v;
                r.tolerance = tolerance.at(v);
                r.reason = "one grid: no check".to_owned();
                continue;
            }
            r.assess(tolerance);
            if r.reached {
                break;
            }
        }
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn study(steps: &[f64], values: &[f64]) -> Study {
        steps
            .iter()
            .zip(values)
            .try_fold(Study::new("n"), |s, (&h, &v)| s.on(h, v))
            .unwrap()
    }

    /// ROADMAP's pitfall: a number quoted on one grid that changes by 0.07 on the next is
    /// caught, on two grids and on three, and is not reported.
    #[test]
    fn a_number_on_an_unconverged_grid_is_caught() {
        let c = study(&[0.02, 0.01], &[2.40, 2.47]).check(1e-3).unwrap();
        assert_eq!(c.verdict, Verdict::NotConverged, "{}", c.reason);
        assert!((c.error.unwrap() - 0.07).abs() < 1e-12);
        assert!(c.gated().unwrap_err().to_string().contains("not reported"));
        // the third grid shows first order: the coarse change stays an estimate
        let c = study(&[0.04, 0.02, 0.01], &[2.33, 2.40, 2.435])
            .check(1e-3)
            .unwrap();
        assert_eq!(c.verdict, Verdict::NotConverged, "{}", c.reason);
        assert!((c.order.unwrap() - 1.0).abs() < 1e-9);
        assert!((c.limit.unwrap() - 2.47).abs() < 1e-12);
        // changes that don't shrink aren't converging, however small
        let c = study(&[0.04, 0.02, 0.01], &[1.0, 1.0 + 1e-6, 1.0 + 2e-6])
            .check(1.0)
            .unwrap();
        assert_eq!(c.verdict, Verdict::NotConverged, "{}", c.reason);
        assert_eq!(c.error, None);
        // one grid is no check
        let c = study(&[0.01], &[2.47]).check(1.0).unwrap();
        assert_eq!(c.verdict, Verdict::Unchecked);
        assert!(c.gated().is_err());
        assert!(Convergence::unchecked("n", 0.01, 2.47).gated().is_err());
    }

    /// A second-order quantity on three halving grids: Richardson's order, limit and estimate,
    /// the estimate within a hair of the true error, and the number reported.
    #[test]
    fn a_converged_number_is_reported_with_richardsons_estimate() {
        let f = |h: f64| 1.5 + 3.0 * h * h;
        let steps = [0.04, 0.02, 0.01];
        let c = study(&steps, &steps.map(f)).check(1e-3).unwrap();
        assert_eq!(c.verdict, Verdict::Converged, "{}", c.reason);
        assert!((c.order.unwrap() - 2.0).abs() < 1e-6);
        assert!((c.limit.unwrap() - 1.5).abs() < 1e-12);
        assert!((c.error.unwrap() - 3e-4).abs() < 1e-12);
        assert_eq!(c.gated().unwrap(), f(0.01));
        // two grids in a ratio of 3: first order's |Δ|/(r − 1)
        let c = study(&[0.03, 0.01], &[1.0, 1.001]).check(1e-3).unwrap();
        assert!((c.error.unwrap() - 0.0005).abs() < 1e-12);
        assert!(c.converged());
        // oscillating values: the larger change
        let c = study(&steps, &[1.0, 1.0004, 0.9998]).check(1e-3).unwrap();
        assert!((c.error.unwrap() - 6e-4).abs() < 1e-12);
        assert!(c.converged());
    }

    /// A second-order quantity driven to a tolerance: it stops on the first grid where the
    /// change and the estimate are both within it, Richardson's limit exact for a pure h², every
    /// grid recorded.
    #[test]
    fn a_second_order_quantity_is_driven_to_its_tolerance() {
        let f = |h: f64| 1.5 + 3.0 * h * h + 40.0 * h.powi(4);
        let d = ToTolerance::new("n", Tolerance::Absolute(1e-4))
            .unwrap()
            .with_order(2.0)
            .unwrap();
        let r = d.grids(0.04, |h| Ok(f(h))).unwrap();
        assert!(r.reached, "{}", r.reason);
        // changes 3.6e-3, 9.1e-4, 2.3e-4, 5.6e-5: the fifth grid
        assert_eq!(r.grids, [0.04, 0.02, 0.01, 0.005, 0.0025]);
        assert_eq!(r.values, r.grids.iter().map(|&h| f(h)).collect::<Vec<_>>());
        assert!(r.last_change().unwrap().abs() <= 1e-4);
        let (error, estimate) = ((r.value - 1.5).abs(), r.error.unwrap());
        assert!(
            error <= estimate && estimate <= 1e-4,
            "{error:e} {estimate:e}"
        );
        // the h⁴ term: Richardson leaves 4·40h⁴, the estimate is 15 times that
        assert!(
            (estimate / error - 15.0).abs() < 1e-3,
            "{}",
            estimate / error
        );
        assert!((r.observed_order.unwrap() - 2.0).abs() < 0.01);
        // without the order: the finest grid's value, the gate's estimate
        let r = ToTolerance::new("n", Tolerance::Absolute(1e-4))
            .unwrap()
            .grids(0.04, |h| Ok(f(h)))
            .unwrap();
        assert_eq!(r.order, None);
        assert_eq!(r.value, f(0.0025));
        // Richardson at the observed order: within 0.2 % of the finest grid's true error here,
        // a hair below it (the h⁴ term), so not a bound, as the order known makes it
        assert!((r.error.unwrap() / (r.value - 1.5).abs() - 1.0).abs() < 2e-3);
        // a relative tolerance is taken at the value
        let r = ToTolerance::new("n", Tolerance::Relative(1e-4))
            .unwrap()
            .grids(0.04, |h| Ok(f(h)))
            .unwrap();
        assert!((r.tolerance - 1.5e-4).abs() < 1e-8);
    }

    /// A quantity that doesn't reach its tolerance within the budget is refused, the error
    /// naming the quantity, its last two values and the tolerance; one whose changes don't
    /// shrink is refused whatever the tolerance.
    #[test]
    fn a_quantity_short_of_its_tolerance_is_refused() {
        let d = ToTolerance::new("Re n_eff", Tolerance::Absolute(1e-9))
            .unwrap()
            .with_budget(3)
            .unwrap();
        let message = d.grids(0.04, |h| Ok(2.0 + h)).unwrap_err().to_string();
        for part in [
            "Re n_eff",
            "1e-9",
            "within 3 grids",
            "2.02",
            "2.01",
            "0.01: 2.01",
        ] {
            assert!(message.contains(part), "{part}: {message}");
        }
        // changes that don't shrink: no estimate
        let message = ToTolerance::new("x", Tolerance::Absolute(2e-3))
            .unwrap()
            .with_budget(3)
            .unwrap()
            .grids(0.04, |h| Ok(1e-3 * (0.04 / h).powi(2)))
            .unwrap_err()
            .to_string();
        assert!(message.contains("don't shrink"), "{message}");
        // bad settings and values
        assert!(ToTolerance::new("x", Tolerance::Absolute(0.0)).is_err());
        assert!(d.clone().with_order(-1.0).is_err());
        assert!(d.clone().with_budget(1).is_err());
        assert!(d.grids(0.0, |_| Ok(1.0)).is_err());
        assert!(d.grids(0.01, |_| Ok(f64::NAN)).is_err());
        assert!(d.counts(0, |_| Ok(1.0)).is_err());
    }

    /// An expansion's quantity driven in its modes and steps: the modes' error ~1/N³, the
    /// steps' ~1/S² (the midpoint rule's), each driven separately and then the modes again at
    /// the steps' count, every value solved once.
    #[test]
    fn an_expansion_is_driven_in_modes_and_steps() {
        let exact = 0.9;
        let f = |m: usize, s: usize| exact + 2.0 / (m as f64).powi(3) - 0.5 / (s as f64).powi(2);
        let mut solved = Vec::new();
        let r = ToTolerance::new("|S21|^2", Tolerance::Absolute(1e-5))
            .unwrap()
            .eme(4, 10, |m, s| {
                solved.push((m, s));
                Ok(f(m, s))
            })
            .unwrap();
        // the modes from 4 to 128 at 10 steps, the steps from 10 to 640 at 128 modes, then the
        // modes 64 and 128 at 640 steps
        assert_eq!(r.mode_study.grids.len(), 6);
        assert_eq!(r.step_study.grids.first(), Some(&0.1));
        assert_eq!((r.modes, r.steps), (128, 640));
        assert_eq!(r.together.grids, [1.0 / 64.0, 1.0 / 128.0]);
        let n = solved.len();
        solved.sort_unstable();
        solved.dedup();
        assert_eq!(solved.len(), n, "a value solved twice");
        assert!(r.error <= 1e-5, "{r:?}");
        assert!((r.value - exact).abs() <= r.error, "{r:?}");
        // a counted driver records 1/N
        let c = ToTolerance::new("x", Tolerance::Absolute(1e-4))
            .unwrap()
            .counts(10, |n| Ok(1.0 / (n * n) as f64))
            .unwrap();
        assert_eq!(c.grids[0], 0.1);
    }

    #[test]
    fn a_study_refuses_grids_out_of_order_and_bad_tolerances() {
        assert!(
            Study::new("n")
                .on(0.01, 1.0)
                .unwrap()
                .on(0.02, 1.0)
                .is_err()
        );
        assert!(Study::new("n").on(0.0, 1.0).is_err());
        assert!(Study::new("n").on(0.01, f64::NAN).is_err());
        assert!(Study::new("n").check(1e-3).is_err());
        assert!(study(&[0.01], &[1.0]).check(0.0).is_err());
    }
}
