//! A heater array's crosstalk corrected: a job's `[task]` of kind `"optimize"` with a
//! `[task.crosstalk]` table and no `[device]` or `[circuit]`.
//!
//! ```yaml
//! name: crosstalk-correction
//! timeout_minutes: 1
//!
//! task:
//!   kind: optimize
//!   method: lbfgsb
//!   seed: 1
//!   evaluations: 200
//!   crosstalk:
//!     # the array's steady response, rad/W: shifter i's phase per W of heater j
//!     phase_per_watt: [[296.4, 49.6], [49.6, 296.4]]
//!     # or Gurses et al.'s row: count, each heater's own P_pi and its neighbours'
//!     # count: 9
//!     # p_pi_mw: 10.6
//!     # neighbour_p_pi_mw: [21.7]
//!     target_rad: [1.5708, 3.1416]
//!     max_power_mw: 30.0
//! ```
//!
//! **The variables** are the heaters' powers, `P1`, `P2`, … in mW, each from 0 to
//! `max_power_mw`, starting where each heater alone would set its own shifter
//! ([`crate::thermal::Correction::uncorrected`]). **The objective** is the phase error
//! Σ (1 − cos(φ_i − φ*_i)), minimized, with its exact gradient
//! ([`crate::thermal::Correction::objective`]); the record names `phase_error`, the total
//! `power_mw` and each shifter's error `error_k` (rad, into (−π, π]). The optimizer is
//! genoxide's, the run the `photonoxide` program's, as for a device.

use schemars::JsonSchema;
use serde::Deserialize;

use super::super::{Event, task_error};
use super::{Point, Problem, Quantity, Settings, Target, Variable, figures_of, settings};
use crate::device::objective::{Names, Source};
use crate::device::{Goal, Reduce};
use crate::expr::{Dual, Scope};
use crate::run::{Job, Run};
use crate::thermal::Correction;
use crate::{Error, Result};

/// A crosstalk correction's `"optimize"` task.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::job) struct CrosstalkTask {
    /// `"optimize"`.
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    /// genoxide's method: lbfgsb, adam, mma, gcmma, nelder-mead, cmaes, de, pso.
    method: String,
    /// The optimizer's seed.
    seed: Option<u64>,
    /// The budget: the optimizer's evaluations.
    evaluations: Option<usize>,
    /// A population method's size.
    population: Option<usize>,
    /// Adam's learning rate.
    learning_rate: Option<f64>,
    /// The heater array.
    crosstalk: CrosstalkSpec,
}

/// `[task.crosstalk]`: a heater array's steady response and the phases wanted of it.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CrosstalkSpec {
    /// The steady response, rad/W: `phase_per_watt[i][j]`, shifter i's phase per W of heater j
    /// (a static job's thermal model's residues summed, or its dn_eff/dP times 2π/λ).
    phase_per_watt: Option<Vec<Vec<f64>>>,
    /// Gurses et al.'s row: how many shifters.
    count: Option<usize>,
    /// Gurses et al.'s row: each heater's power for π on its own shifter, mW.
    p_pi_mw: Option<f64>,
    /// Gurses et al.'s row: the power for π on the n-th neighbour, mW, n = 1, 2, …
    #[serde(default)]
    neighbour_p_pi_mw: Vec<f64>,
    /// Each shifter's wanted phase, rad.
    target_rad: Vec<f64>,
    /// Each shifter's phase with no power, rad (0 by default).
    #[serde(default)]
    bias_rad: Vec<f64>,
    /// What each heater stands, mW: every power's upper bound.
    max_power_mw: f64,
}

/// A crosstalk correction's optimization: the correction, and the names its record uses.
#[derive(Debug)]
pub(in crate::job) struct CrosstalkTarget {
    correction: Correction,
}

/// Whether `job`'s task is a crosstalk correction's: an `"optimize"` task with a
/// `[task.crosstalk]`.
pub(in crate::job) fn corrects(job: &Job) -> bool {
    job.device().is_none() && job.circuit().is_none() && job.task().contains_key("crosstalk")
}

fn names(count: usize) -> Names {
    let mut names = Names {
        scope: Scope::new(),
        sources: Vec::new(),
        outputs: Vec::new(),
    };
    names.add(
        "phase_error",
        Source::Figure("phase_error".into()),
        "",
        "Σ (1 − cos(φ − φ*)) over the shifters",
        false,
    );
    names.add(
        "power_mw",
        Source::Figure("power_mw".into()),
        "mW",
        "the heaters' total power",
        false,
    );
    for k in 1..=count {
        names.add(
            &format!("error_{k}"),
            Source::Figure(format!("error_{k}")),
            "rad",
            &format!("shifter {k}'s phase less its target, into (−π, π]"),
            false,
        );
    }
    names
}

/// [`Problem::of`] for a crosstalk correction.
pub(super) fn of(job: &Job) -> Result<Problem> {
    let task: CrosstalkTask = job
        .task()
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(e.to_string()))?;
    let Settings {
        method,
        timeout_minutes,
        seed,
        evaluations,
        ..
    } = settings(
        job,
        &task.method,
        task.seed,
        task.evaluations,
        task.population,
        task.learning_rate,
        None,
    )?;
    if method.objectives != 1 {
        return Err(task_error(format!(
            "{} finds a Pareto front of two objectives: a correction has one, its phase error",
            method.title
        )));
    }
    let c = &task.crosstalk;
    let bad = |e: Error| task_error(format!("[task.crosstalk]: {e}"));
    let mut correction = match (&c.phase_per_watt, c.count, c.p_pi_mw) {
        (Some(m), None, None) if c.neighbour_p_pi_mw.is_empty() => {
            Correction::new(m.clone(), c.target_rad.clone()).map_err(bad)?
        }
        (None, Some(n), Some(p)) => Correction::row(
            n,
            p * 1e-3,
            &c.neighbour_p_pi_mw
                .iter()
                .map(|v| v * 1e-3)
                .collect::<Vec<_>>(),
            c.target_rad.clone(),
        )
        .map_err(bad)?,
        _ => {
            return Err(task_error(
                "[task.crosstalk] takes phase_per_watt, or Gurses et al.'s row: count, p_pi_mw \
                 and neighbour_p_pi_mw; one of them",
            ));
        }
    };
    if !c.bias_rad.is_empty() {
        correction = correction.with_bias(c.bias_rad.clone()).map_err(bad)?;
    }
    if !(c.max_power_mw.is_finite() && c.max_power_mw > 0.0) {
        return Err(task_error(format!(
            "[task.crosstalk] max_power_mw must be positive, not {}",
            c.max_power_mw
        )));
    }
    let n = correction.heaters();
    if n > 64 {
        return Err(task_error(format!(
            "[task.crosstalk] has {n} heaters: an optimization takes at most 64 variables"
        )));
    }
    let start: Vec<f64> = match correction.uncorrected() {
        Ok(p) => p
            .iter()
            .map(|w| (w * 1e3).clamp(0.0, c.max_power_mw))
            .collect(),
        Err(_) => vec![0.0; n],
    };
    let variables: Vec<Variable> = (1..=n)
        .map(|k| Variable::new(&format!("P{k}"), 0.0, c.max_power_mw))
        .collect();
    let names = names(correction.shifters());
    let text = "phase_error".to_owned();
    let expression = names.parse("the objective", &text)?;
    let objective = Quantity {
        figures: figures_of(&names, &expression),
        name: "phase error".into(),
        symbol: "phase_error".into(),
        unit: String::new(),
        text,
        expression,
        goal: Some((Goal::Minimize, 0.0)),
        limit: None,
        reduce: Reduce::Worst,
        instance: None,
    };
    Ok(Problem {
        kind: "crosstalk correction".into(),
        wavelengths: Vec::new(),
        corners: Vec::new(),
        units: vec!["mW".into(); n],
        angle: vec![false; n],
        start,
        variables,
        method,
        seed,
        evaluations,
        population: task.population.or(method.population.filter(|&p| p > 0)),
        learning_rate: task.learning_rate,
        names,
        objectives: vec![objective],
        constraints: Vec::new(),
        exact: vec![false; n],
        timeout_minutes,
        continuation: None,
        stage: Default::default(),
        target: Target::Crosstalk(Box::new(CrosstalkTarget { correction })),
    })
}

/// The quantities at `x` (mW) and their derivatives in the variables `wanted`: exact, the
/// objective's gradient per W made per mW.
pub(super) fn point(
    p: &Problem,
    t: &CrosstalkTarget,
    x: &[f64],
    wanted: &[usize],
) -> Result<Point> {
    let watts: Vec<f64> = x.iter().map(|v| v * 1e-3).collect();
    let (f, g) = t.correction.objective(&watts)?;
    let errors = t.correction.errors(&watts)?;
    let response = t.correction.response();
    let values = |symbol: &str| -> (f64, Vec<f64>) {
        match symbol {
            "phase_error" => (f, wanted.iter().map(|&j| g[j] * 1e-3).collect()),
            "power_mw" => (x.iter().sum(), vec![1.0; wanted.len()]),
            s => {
                let k: usize = s
                    .strip_prefix("error_")
                    .and_then(|k| k.parse().ok())
                    .unwrap_or(0);
                match k
                    .checked_sub(1)
                    .and_then(|i| errors.get(i).map(|e| (i, *e)))
                {
                    Some((i, e)) => (e, wanted.iter().map(|&j| response[i][j] * 1e-3).collect()),
                    None => (f64::NAN, vec![f64::NAN; wanted.len()]),
                }
            }
        }
    };
    let quantities: Vec<&Quantity> = p.quantities().collect();
    let mut samples = Vec::with_capacity(quantities.len());
    let mut derivatives: Vec<Option<Vec<Vec<f64>>>> = wanted
        .iter()
        .map(|_| Some(vec![Vec::new(); quantities.len()]))
        .collect();
    for (k, q) in quantities.iter().enumerate() {
        let at: Vec<(usize, f64, Vec<f64>)> = p
            .names
            .sources
            .iter()
            .map(|(slot, source)| {
                let (v, d) = match source {
                    Source::Figure(s) => values(s),
                    _ => (f64::NAN, vec![f64::NAN; wanted.len()]),
                };
                (*slot, v, d)
            })
            .collect();
        let mut slots = vec![0.0; p.names.scope.len()];
        for (slot, v, _) in &at {
            slots[*slot] = *v;
        }
        samples.push(vec![q.expression.eval(&slots)]);
        for (i, d) in derivatives.iter_mut().enumerate() {
            let mut duals: Vec<Dual> = slots.iter().map(|&v| Dual::constant(v)).collect();
            for (slot, _, dv) in &at {
                duals[*slot].derivative = dv[i];
            }
            if let Some(d) = d {
                d[k].push(q.expression.eval_dual(&duals).derivative);
            }
        }
    }
    Ok(Point {
        samples,
        derivatives,
        solves: 0,
    })
}

/// Records a correction's start: [`Event::Started`]; the record's fidelity, model and
/// parameters' names.
pub(super) fn begin(
    p: &Problem,
    t: &CrosstalkTarget,
    job: &Job,
    run: &mut Run,
) -> Result<(String, String, Vec<String>)> {
    run.record(&Event::Started {
        job: job.name().to_owned(),
        kind: "crosstalk correction".into(),
    })?;
    Ok((
        "compact".into(),
        format!(
            "the steady response of {} heaters on {} shifters, φ = φ₀ + C P (Gurses et al. 2022, \
             doi:10.1109/JSTQE.2022.3189965)",
            t.correction.heaters(),
            t.correction.shifters()
        ),
        p.variables.iter().map(|v| v.parameter.clone()).collect(),
    ))
}

/// The variables' values at `x`, as text.
pub(super) fn parameters_at(p: &Problem, x: &[f64]) -> Vec<(String, String)> {
    p.variables
        .iter()
        .zip(x)
        .map(|(v, value)| (v.parameter.clone(), format!("{value:?}")))
        .collect()
}
