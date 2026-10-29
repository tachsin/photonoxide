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
//! L. F. Richardson, Philos. Trans. R. Soc. A 210, 307 (1911), doi:10.1098/rsta.1911.0009 (the
//! extrapolation); P. J. Roache, Annu. Rev. Fluid Mech. 29, 123 (1997),
//! doi:10.1146/annurev.fluid.29.1.123 (grid convergence studies and their estimates).

use num_complex::Complex64;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

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
