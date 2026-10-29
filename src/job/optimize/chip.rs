//! A chip optimized: a job's `[circuit]` with a `[task]` of kind `"optimize"`.
//!
//! ```yaml
//! task:
//!   kind: optimize
//!   method: lbfgsb
//!   seed: 1
//!   evaluations: 200
//!   from_um: 1.5499           # the band each quantity is taken over
//!   to_um: 1.5501
//!   points: 21
//!
//!   variable:
//!     - parameter: ring1.radius   # instance.parameter: a design's, or a component's value
//!       from: 9.9
//!       to: 10.1
//!
//!   objective:
//!     - expression: "-10*log10(T_drop_in)"   # of the chip's outputs: T_q_p between its ports, lambda
//!       goal: minimize
//!
//!   constraint:
//!     - instance: c1          # a device instance's own constraint, on its own outputs
//!       name: gap
//!       at_least: 0.15
//! ```
//!
//! **The chip's outputs** are `T_q_p`, |S_qp|² between its external ports q and p, and `lambda`;
//! an instance's quantity (`instance = ...`) is its device's own: a declared objective or
//! constraint, or an expression of its own outputs ([`crate::device::objective`]).
//!
//! **Derivatives.** At each wavelength the circuit adjoint gives ∂S/∂θ for every parameter θ of
//! every instance ([`crate::circuit::Circuit::jacobian`]); a component's value is its own θ, and a
//! device's parameter moves its component's θ by the device's exact derivatives
//! ([`crate::device::Built::derivatives`]) through its design's expressions
//! ([`crate::device::Design::sensitivities`]). A parameter whose device has none (a gap, a bend's
//! radius) is differenced. Every exact derivative is checked against fourth-order differences
//! before it is used ([`Problem::check_gradients`]).

use std::f64::consts::PI;

use schemars::SchemaGenerator;
use serde::Deserialize;
use toml::Value;

use super::super::chip::{
    Assembled, CHIP_EXPORTS, ChipDesign, InstanceModel, Resolved, band, chip_job, chip_of, corner,
};
use super::super::kind::TaskKind;
use super::super::study::{CornerTask, check_corners, corners_of};
use super::super::{Event, task_error};
use super::{
    Aim, Bound, Optimization, Point, Problem, Quantity, Settings, Target, Variable, VariableTask,
    figures_of, settings, table,
};
use crate::circuit::SMatrix;
use crate::device::objective::{Names, Source};
use crate::device::{Goal, Limit, Reduce};
use crate::expr::{Dual, Scope};
use crate::run::{Job, Run, Stop};
use crate::units::Wavelength;
use crate::{Error, Result};

/// A chip's optimization: the chip, its variables resolved, and each instance's outputs where a
/// quantity names them.
#[derive(Debug)]
pub(in crate::job) struct ChipTarget {
    chip: ChipDesign,
    resolved: Vec<Resolved>,
    instance_names: Vec<Option<Names>>,
}

/// A chip's `"optimize"` task.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChipOptimizeTask {
    #[allow(dead_code)] // looked up by the registry (job::kind); kept for serde and the schema
    kind: String,
    method: String,
    seed: Option<u64>,
    evaluations: Option<usize>,
    population: Option<usize>,
    learning_rate: Option<f64>,
    reduce: Option<String>,
    from_um: f64,
    to_um: f64,
    points: Option<usize>,
    #[serde(default)]
    variable: Vec<VariableTask>,
    #[serde(default)]
    objective: Vec<Aim>,
    #[serde(default)]
    constraint: Vec<Bound>,
    #[serde(default)]
    corner: Vec<CornerTask>,
}

/// The `"optimize"` kind beside a `[circuit]`: the chip's instances' parameters optimized, by
/// genoxide in the `photonoxide` program.
pub(in crate::job) struct ChipOptimizeKind;

impl TaskKind for ChipOptimizeKind {
    fn name(&self) -> &'static str {
        "optimize"
    }

    /// A device's optimization's table, which the schema has stood for a chip's too.
    fn schema(&self, g: &mut SchemaGenerator) -> serde_json::Value {
        super::super::schema::task::<super::OptimizeTask>(g)
    }

    fn check(&self, job: &Job) -> Result<()> {
        super::check(job)
    }

    fn preview(&self, job: &Job) -> Result<Event> {
        super::super::check(job)?;
        // no scene of its own: as before, what reading the task as an fdfd job's refuses
        super::super::fdfd::preview(job)
    }

    fn execute(&self, job: &Job, _: &mut Run, _: &Stop) -> Result<()> {
        self.check(job)?;
        Err(super::runs_elsewhere())
    }

    fn exports(&self, _: &Job) -> (&'static str, &'static [&'static str]) {
        CHIP_EXPORTS
    }
}

/// The chip's outputs: the powers between its external ports, and the wavelength.
fn chip_names(ports: &[String]) -> Names {
    let mut names = Names {
        scope: Scope::new(),
        sources: Vec::new(),
        outputs: Vec::new(),
    };
    for (q, a) in ports.iter().enumerate() {
        for (p, b) in ports.iter().enumerate() {
            names.add(
                &format!("T_{a}_{b}"),
                Source::Power(q, p),
                "",
                &format!("power from {b} into {a}"),
                true,
            );
        }
    }
    names.add("lambda", Source::Lambda, "µm", "the wavelength", true);
    names
}

/// [`Problem::of`] for a job with a `[circuit]` table.
pub(super) fn of(job: &Job) -> Result<Problem> {
    let chip = chip_of(job)?;
    let task: ChipOptimizeTask = job
        .task()
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(e.to_string()))?;
    let Settings {
        method,
        timeout_minutes,
        seed,
        evaluations,
        reduce,
    } = settings(
        job,
        &task.method,
        task.seed,
        task.evaluations,
        task.population,
        task.learning_rate,
        task.reduce.as_deref(),
    )?;
    let wavelengths = band(task.from_um, task.to_um, task.points, 5)?;
    if wavelengths.len() > 1001 {
        return Err(task_error(format!(
            "an optimization takes each quantity at most at 1001 wavelengths, got {}",
            wavelengths.len()
        )));
    }
    let nominal = chip.assemble()?;
    let corners = corners_of(&task.corner)?;
    if !corners.is_empty() {
        check_corners(&corners)?;
        for c in &corners {
            corner(&chip, c)?.assemble()?;
        }
    }
    if task.variable.is_empty() || task.variable.len() > super::MAX_VARIABLES {
        return Err(task_error(format!(
            "an optimization takes 1 to {} variables ([[task.variable]]), got {}",
            super::MAX_VARIABLES,
            task.variable.len()
        )));
    }
    let mut variables = Vec::new();
    let mut resolved = Vec::new();
    for (k, v) in task.variable.iter().enumerate() {
        let r = chip.resolve(&v.parameter)?;
        if task.variable[..k]
            .iter()
            .any(|w| w.parameter == v.parameter)
        {
            return Err(task_error(format!(
                "the variable {} is given twice",
                v.parameter
            )));
        }
        if !(v.from.is_finite() && v.to.is_finite() && v.from < v.to) {
            return Err(task_error(format!(
                "the variable {} needs bounds from a smaller to a larger value, got {} to {}",
                v.parameter, v.from, v.to
            )));
        }
        if let Some((lo, hi)) = r.range
            && (v.from < lo || v.to > hi)
        {
            return Err(task_error(format!(
                "the variable {} takes {lo} to {hi} {}: its parameter's range, not {} to {}",
                v.parameter, r.unit, v.from, v.to
            )));
        }
        if !(v.from..=v.to).contains(&r.nominal) {
            return Err(task_error(format!(
                "the chip's {} is {}, outside its variable's bounds {} to {}: an optimization \
                 starts from the chip",
                v.parameter, r.nominal, v.from, v.to
            )));
        }
        variables.push(Variable::new(&v.parameter, v.from, v.to));
        resolved.push(r);
    }
    let ports = chip.port_names();
    let names = chip_names(&ports);
    let mut instance_names: Vec<Option<Names>> = (0..chip.instances.len()).map(|_| None).collect();
    if task.objective.is_empty() || task.objective.len() > method.objectives {
        return Err(task_error(format!(
            "{} takes {} objective{} ([[task.objective]]), got {}",
            method.title,
            method.objectives,
            if method.objectives == 1 { "" } else { "s" },
            task.objective.len()
        )));
    }
    if method.objectives == 2 && task.objective.len() != 2 {
        return Err(task_error(format!(
            "{} traces a Pareto front of two objectives, got {}",
            method.title,
            task.objective.len()
        )));
    }
    // (name, symbol, unit, expression, a declared reduction or bound) of a quantity
    type Picked = (
        String,
        String,
        String,
        String,
        Option<Reduce>,
        Option<(Limit, f64)>,
    );
    let mut objectives = Vec::new();
    for (k, a) in task.objective.iter().enumerate() {
        let label = format!("objective {}", k + 1);
        let at = match &a.instance {
            Some(i) => Some(instance(&chip, &nominal, &mut instance_names, i, &label)?),
            None => None,
        };
        let mut declared_goal = None;
        let mut declared_target = None;
        let (name, symbol, unit, text, fixed, _): Picked = match (&a.name, &a.expression, at) {
            (Some(n), None, Some(i)) => {
                let d = nominal.parts[i]
                    .device
                    .as_ref()
                    .expect("an instance's device");
                let declared = d.device.objectives();
                let o = declared.iter().find(|o| o.symbol == *n).ok_or_else(|| {
                    Error::invalid(
                        "objective",
                        format!(
                            "{label}: a {} has no objective \"{n}\": it has {}",
                            d.device.kind(),
                            declared
                                .iter()
                                .map(|o| o.symbol.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    )
                })?;
                declared_goal = Some(o.goal);
                declared_target = o.target;
                (
                    o.name.clone(),
                    format!("{}.{}", chip.instances[i].name, o.symbol),
                    o.unit.clone(),
                    o.expression.clone(),
                    o.reduce,
                    None,
                )
            }
            (Some(_), None, None) => {
                return Err(task_error(format!(
                    "{label}: a chip declares no objectives; give an expression of its outputs, \
                     or name an instance whose device's objective it is"
                )));
            }
            (None, Some(e), _) => (
                e.clone(),
                label.clone(),
                String::new(),
                e.clone(),
                None,
                None,
            ),
            _ => {
                return Err(task_error(format!(
                    "{label} needs a name (an instance's device's) or an expression, one of them"
                )));
            }
        };
        let goal = match (&a.goal, declared_goal) {
            (Some(g), _) => Goal::named(g)?,
            (None, Some(g)) => g,
            (None, None) => Goal::Minimize,
        };
        let target = match goal {
            Goal::Target => Some(a.target.or(declared_target).ok_or_else(|| {
                task_error(format!("{label} ({symbol}) has a target goal: give target"))
            })?),
            _ => {
                if a.target.is_some() {
                    return Err(task_error(format!(
                        "{label} ({symbol}) is to {}: it takes no target",
                        goal.name()
                    )));
                }
                None
            }
        };
        if let Some(t) = target
            && !t.is_finite()
        {
            return Err(task_error(format!("{label}'s target must be a number")));
        }
        let reduce = match (&a.reduce, fixed) {
            (Some(r), _) => Reduce::named(r)?,
            (None, Some(r)) => r,
            (None, None) => reduce,
        };
        let scope = at.map_or(&names, |i| instance_names[i].as_ref().expect("its names"));
        let expression = scope.parse(&label, &text)?;
        objectives.push(Quantity {
            figures: figures_of(scope, &expression),
            name,
            symbol,
            unit,
            text,
            expression,
            goal: Some((goal, target.unwrap_or(0.0))),
            limit: None,
            reduce,
            instance: at,
        });
    }
    if !task.constraint.is_empty() && !method.constraints {
        return Err(task_error(format!(
            "{} takes no constraints (it ignores them): constrain by mma, or a value-only \
             method (nelder-mead, cmaes, de, pso), or narrow the variables' bounds",
            method.title
        )));
    }
    let mut constraints = Vec::new();
    for (k, b) in task.constraint.iter().enumerate() {
        let label = format!("constraint {}", k + 1);
        let at = match &b.instance {
            Some(i) => Some(instance(&chip, &nominal, &mut instance_names, i, &label)?),
            None => None,
        };
        let (name, symbol, unit, text, _, declared): Picked = match (&b.name, &b.expression, at) {
            (Some(n), None, Some(i)) => {
                let d = nominal.parts[i]
                    .device
                    .as_ref()
                    .expect("an instance's device");
                let declared = d.device.constraints();
                let c = declared.iter().find(|c| c.symbol == *n).ok_or_else(|| {
                    Error::invalid(
                        "constraint",
                        format!(
                            "{label}: a {} has no constraint \"{n}\": it has {}",
                            d.device.kind(),
                            declared
                                .iter()
                                .map(|c| c.symbol.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    )
                })?;
                (
                    c.name.clone(),
                    format!("{}.{}", chip.instances[i].name, c.symbol),
                    c.unit.clone(),
                    c.expression.clone(),
                    None,
                    Some((c.limit, c.bound)),
                )
            }
            (Some(_), None, None) => {
                return Err(task_error(format!(
                    "{label}: a chip declares no constraints; give an expression of its outputs, \
                     or name an instance whose device's constraint it is"
                )));
            }
            (None, Some(e), _) => (
                e.clone(),
                label.clone(),
                String::new(),
                e.clone(),
                None,
                None,
            ),
            _ => {
                return Err(task_error(format!(
                    "{label} needs a name (an instance's device's) or an expression, one of them"
                )));
            }
        };
        let limit = match (b.at_most, b.at_least, declared) {
            (Some(v), None, _) => (Limit::AtMost, v),
            (None, Some(v), _) => (Limit::AtLeast, v),
            (None, None, Some(d)) => d,
            (None, None, None) => {
                return Err(task_error(format!(
                    "{label} ({symbol}) needs its bound: at_most or at_least"
                )));
            }
            (Some(_), Some(_), _) => {
                return Err(task_error(format!(
                    "{label} ({symbol}) takes at_most or at_least, not both: two constraints \
                     keep a quantity between two bounds"
                )));
            }
        };
        if !limit.1.is_finite() {
            return Err(task_error(format!("{label}'s bound must be a number")));
        }
        let reduce = match &b.reduce {
            Some(r) => Reduce::named(r)?,
            None => Reduce::Worst,
        };
        let scope = at.map_or(&names, |i| instance_names[i].as_ref().expect("its names"));
        let expression = scope.parse(&label, &text)?;
        constraints.push(Quantity {
            figures: figures_of(scope, &expression),
            name,
            symbol,
            unit,
            text,
            expression,
            goal: None,
            limit: Some(limit),
            reduce,
            instance: at,
        });
    }
    let n = variables.len();
    Ok(Problem {
        kind: "chip".into(),
        wavelengths,
        corners,
        units: resolved.iter().map(|r| r.unit.clone()).collect(),
        angle: resolved.iter().map(|r| r.angle).collect(),
        start: resolved.iter().map(|r| r.nominal).collect(),
        variables,
        method,
        seed,
        evaluations,
        population: task.population.or(method.population.filter(|&p| p > 0)),
        learning_rate: task.learning_rate,
        names,
        objectives,
        constraints,
        exact: vec![false; n],
        timeout_minutes,
        continuation: None,
        stage: Default::default(),
        target: Target::Chip(Box::new(ChipTarget {
            chip,
            resolved,
            instance_names,
        })),
    })
}

/// The index of the device instance `name` a quantity names, its outputs' names made the first
/// time.
fn instance(
    chip: &ChipDesign,
    nominal: &Assembled,
    names: &mut [Option<Names>],
    name: &str,
    what: &str,
) -> Result<usize> {
    let k = chip
        .instances
        .iter()
        .position(|i| i.name == name)
        .ok_or_else(|| {
            Error::invalid(
                "chip",
                format!("{what} names {name}, which isn't an instance"),
            )
        })?;
    let Some(d) = &nominal.parts[k].device else {
        return Err(Error::invalid(
            "chip",
            format!(
                "{what} names {name}, one of the circuit's components: an instance's own \
                 objectives and constraints are a device's"
            ),
        ));
    };
    if names[k].is_none() {
        let built = d.device.component(d.design.values(), d.fidelity).ok();
        names[k] = Some(Names::of(
            d.device.as_ref(),
            d.design.values(),
            built.as_ref(),
        )?);
    }
    Ok(k)
}

/// The chip at `x`.
pub(super) fn chip_at(t: &ChipTarget, x: &[f64]) -> Result<ChipDesign> {
    if x.len() != t.resolved.len() {
        return Err(Error::invalid(
            "optimize",
            format!("a point of {} variables, not {}", t.resolved.len(), x.len()),
        ));
    }
    let mut chip = t.chip.clone();
    for (r, &v) in t.resolved.iter().zip(x) {
        chip = chip.with(r, v)?;
    }
    Ok(chip)
}

/// The variables' expressions at `x`, `instance.parameter`.
pub(super) fn parameters_at(
    p: &Problem,
    t: &ChipTarget,
    x: &[f64],
) -> Result<Vec<(String, String)>> {
    let chip = chip_at(t, x)?;
    Ok(t.resolved
        .iter()
        .zip(&p.variables)
        .zip(x)
        .map(|((r, v), &value)| {
            let i = &chip.instances[r.instance];
            let expression = match (&i.model, r.design) {
                (InstanceModel::Device(spec), true) => spec
                    .parameters
                    .iter()
                    .find(|(n, _)| *n == r.parameter)
                    .map_or_else(|| format!("{value:?}"), |(_, e)| e.clone()),
                _ => format!("{value:?}"),
            };
            (v.parameter.clone(), expression)
        })
        .collect())
}

/// Records a chip's optimization's start: [`Event::Started`] and [`Event::Chip`]; the record's
/// fidelity, model and parameters' names.
pub(super) fn begin(
    p: &Problem,
    t: &ChipTarget,
    job: &Job,
    run: &mut Run,
) -> Result<(String, String, Vec<String>)> {
    run.record(&Event::Started {
        job: job.name().to_owned(),
        kind: "chip".into(),
    })?;
    let a = t.chip.assemble()?;
    let parts = t.chip.parts(&a);
    let mut fidelities: Vec<String> = Vec::new();
    for part in &parts {
        if !fidelities.contains(&part.fidelity) {
            fidelities.push(part.fidelity.clone());
        }
    }
    let devices = parts.iter().filter(|q| q.device).count();
    run.record(&Event::Chip {
        instances: parts,
        ports: t.chip.port_names(),
    })?;
    Ok((
        fidelities.join(" and "),
        format!(
            "the circuit solve of its {} instances ({devices} devices), each by its own model",
            t.chip.instances.len()
        ),
        p.variables.iter().map(|v| v.parameter.clone()).collect(),
    ))
}

/// How the circuit's values move with a variable: (slot, coefficient) pairs, or `None` where a
/// device has no exact derivative in it.
fn coefficients(a: &Assembled, r: &Resolved) -> Result<Option<Vec<(usize, f64)>>> {
    let part = &a.parts[r.instance];
    let parameters = part.component.parameters();
    if !r.design {
        let k = parameters
            .iter()
            .position(|q| q.name == r.parameter)
            .unwrap_or_else(|| unreachable!("a component's value is one of its parameters"));
        return Ok(Some(vec![(part.offset + k, 1.0)]));
    }
    let d = part
        .device
        .as_ref()
        .unwrap_or_else(|| unreachable!("a design parameter is a device's"));
    let mut sens = d.design.sensitivities(&r.parameter)?;
    if r.angle {
        for v in &mut sens {
            *v *= PI / 180.0;
        }
    }
    let mut out = Vec::new();
    for m in 0..parameters.len() {
        if part.overridden[m] {
            continue;
        }
        let mut total = 0.0;
        for (k, &dp) in sens.iter().enumerate() {
            if dp == 0.0 {
                continue;
            }
            let Some(along) = &d.derivatives[k] else {
                return Ok(None);
            };
            if along[m] != 0.0 {
                total += along[m] * dp;
            }
        }
        if total != 0.0 {
            out.push((part.offset + m, total));
        }
    }
    Ok(Some(out))
}

/// [`Problem::point`] for a chip: each quantity at each sample (the corners, then the
/// wavelengths), and their exact derivatives in the variables of `wanted`.
pub(super) fn point(p: &Problem, t: &ChipTarget, x: &[f64], wanted: &[usize]) -> Result<Point> {
    let nq = p.objectives.len() + p.constraints.len();
    let mut samples = vec![Vec::new(); nq];
    let mut derivatives: Vec<Option<Vec<Vec<f64>>>> =
        wanted.iter().map(|_| Some(vec![Vec::new(); nq])).collect();
    let at_x = chip_at(t, x)?;
    let mut chips = vec![at_x.clone()];
    for c in &p.corners {
        chips.push(corner(&at_x, c)?);
    }
    let quantities: Vec<&Quantity> = p.quantities().collect();
    let mut solves = 0;
    for chip in &chips {
        let a = chip.assemble()?;
        solves += 1;
        for q in &quantities {
            if let Some(i) = q.instance {
                let d = a.parts[i].device.as_ref().expect("an instance's device");
                for f in &q.figures {
                    if !d.figures.iter().any(|g| g.symbol == *f) {
                        return Err(Error::invalid(
                            "optimize",
                            format!(
                                "the model of {} gives no figure {f}, which {} names",
                                chip.instances[i].name, q.symbol
                            ),
                        ));
                    }
                }
            }
        }
        let chains = wanted
            .iter()
            .map(|&j| coefficients(&a, &t.resolved[j]))
            .collect::<Result<Vec<_>>>()?;
        // each design variable's sensitivities, for an instance's parameters' outputs
        let sens = wanted
            .iter()
            .map(|&j| {
                let r = &t.resolved[j];
                if !r.design {
                    return Ok(Vec::new());
                }
                let d = a.parts[r.instance].device.as_ref().expect("a device");
                let mut s = d.design.sensitivities(&r.parameter)?;
                if r.angle {
                    for v in &mut s {
                        *v *= PI / 180.0;
                    }
                }
                Ok(s)
            })
            .collect::<Result<Vec<_>>>()?;
        let jacobian = chains.iter().any(Option::is_some);
        for &w in &p.wavelengths {
            let at = Wavelength::um(w)?;
            let (s, ds) = if jacobian {
                a.circuit.jacobian(at, &a.values)?
            } else {
                (a.circuit.s_matrix_with(at, &a.values)?, Vec::new())
            };
            // the instances' own S-matrices and derivatives, where a quantity is of them
            let mut own: Vec<Option<(SMatrix, Option<Vec<SMatrix>>)>> =
                (0..a.parts.len()).map(|_| None).collect();
            for q in &quantities {
                if let Some(i) = q.instance
                    && own[i].is_none()
                {
                    let part = &a.parts[i];
                    let n = part.component.parameters().len();
                    let values = &a.values[part.offset..part.offset + n];
                    let si = part.component.s_matrix(at, values)?;
                    let di = if jacobian {
                        part.component.derivatives(at, values)?
                    } else {
                        None
                    };
                    own[i] = Some((si, di));
                }
            }
            let slots_of = |q: &Quantity| -> Vec<f64> {
                match q.instance {
                    None => {
                        let mut v = vec![0.0; p.names.scope.len()];
                        for (slot, source) in &p.names.sources {
                            v[*slot] = match source {
                                Source::Power(qq, pp) => s.power(*qq, *pp),
                                Source::Lambda => w,
                                _ => f64::NAN,
                            };
                        }
                        v
                    }
                    Some(i) => {
                        let names = t.instance_names[i].as_ref().expect("its names");
                        let d = a.parts[i].device.as_ref().expect("a device");
                        let si = &own[i].as_ref().expect("its S-matrix").0;
                        let mut v = vec![0.0; names.scope.len()];
                        for (slot, source) in &names.sources {
                            v[*slot] = match source {
                                Source::Power(qq, pp) => si.power(*qq, *pp),
                                Source::Figure(f) => d
                                    .figures
                                    .iter()
                                    .find(|g| g.symbol == *f)
                                    .map_or(f64::NAN, |g| g.value),
                                Source::Parameter(k) => d.design.values().as_slice()[*k],
                                Source::Lambda => w,
                                // a chip's task takes no continuation
                                Source::Stage => f64::NAN,
                            };
                        }
                        v
                    }
                }
            };
            let slots: Vec<Vec<f64>> = quantities.iter().map(|q| slots_of(q)).collect();
            for (k, q) in quantities.iter().enumerate() {
                samples[k].push(q.expression.eval(&slots[k]));
            }
            for (i, &j) in wanted.iter().enumerate() {
                let Some(dq) = derivatives[i].as_mut() else {
                    continue;
                };
                let Some(chain) = &chains[i] else {
                    derivatives[i] = None;
                    continue;
                };
                let r = &t.resolved[j];
                let n = s.size();
                let mut d_s = SMatrix::zeros(n);
                for &(slot, c) in chain {
                    for qq in 0..n {
                        for pp in 0..n {
                            d_s[(qq, pp)] += ds[slot][(qq, pp)] * c;
                        }
                    }
                }
                let mut known = true;
                for (k, q) in quantities.iter().enumerate() {
                    let mut duals: Vec<Dual> =
                        slots[k].iter().map(|&v| Dual::constant(v)).collect();
                    match q.instance {
                        None => {
                            for (slot, source) in &p.names.sources {
                                if let Source::Power(qq, pp) = source {
                                    duals[*slot].derivative =
                                        2.0 * (s[(*qq, *pp)].conj() * d_s[(*qq, *pp)]).re;
                                }
                            }
                        }
                        Some(inst) if inst == r.instance => {
                            let part = &a.parts[inst];
                            let (si, di) = own[inst].as_ref().expect("its S-matrix");
                            let m = si.size();
                            let local: Vec<(usize, f64)> = chain
                                .iter()
                                .filter(|(slot, _)| {
                                    (part.offset..part.offset + part.component.parameters().len())
                                        .contains(slot)
                                })
                                .map(|&(slot, c)| (slot - part.offset, c))
                                .collect();
                            let d_own = match di {
                                Some(di) => {
                                    let mut d = SMatrix::zeros(m);
                                    for &(t, c) in &local {
                                        for qq in 0..m {
                                            for pp in 0..m {
                                                d[(qq, pp)] += di[t][(qq, pp)] * c;
                                            }
                                        }
                                    }
                                    Some(d)
                                }
                                None if local.is_empty() => Some(SMatrix::zeros(m)),
                                None => None,
                            };
                            let names = t.instance_names[inst].as_ref().expect("its names");
                            for (slot, source) in &names.sources {
                                duals[*slot].derivative = match source {
                                    Source::Power(qq, pp) => d_own.as_ref().map_or(f64::NAN, |d| {
                                        2.0 * (si[(*qq, *pp)].conj() * d[(*qq, *pp)]).re
                                    }),
                                    Source::Figure(_) => {
                                        if r.design {
                                            f64::NAN
                                        } else {
                                            0.0
                                        }
                                    }
                                    Source::Parameter(kk) => {
                                        sens[i].get(*kk).copied().unwrap_or(0.0)
                                    }
                                    Source::Lambda | Source::Stage => 0.0,
                                };
                            }
                        }
                        // another instance's variable leaves this one's outputs as they are
                        Some(_) => {}
                    }
                    let v = q.expression.eval_dual(&duals).derivative;
                    known &= v.is_finite();
                    dq[k].push(v);
                }
                if !known {
                    derivatives[i] = None;
                }
            }
        }
    }
    Ok(Point {
        samples,
        derivatives,
        solves,
    })
}

/// The job that optimizes `chip` as `optimization` says (its variables, objectives and
/// constraints naming instances' parameters `instance.parameter`; its corners' offsets too),
/// each quantity taken over `band`, named `name`, with a hard limit of `timeout_minutes`. It is
/// checked by [`super::super::check`].
///
/// # Errors
///
/// The errors of [`super::super::check`] for the job it makes: a variable, objective or
/// constraint the chip hasn't, bounds that aren't, a method that doesn't take what is asked of
/// it.
pub fn optimize_from_chip(
    name: &str,
    chip: &ChipDesign,
    band: (f64, f64, usize),
    optimization: &Optimization,
    timeout_minutes: f64,
) -> Result<Job> {
    let mut task = super::super::chip::band_task("optimize", band);
    task.insert("method".into(), Value::String(optimization.method.clone()));
    task.insert("seed".into(), Value::Integer(optimization.seed as i64));
    task.insert(
        "evaluations".into(),
        Value::Integer(optimization.evaluations as i64),
    );
    if let Some(p) = optimization.population {
        task.insert("population".into(), Value::Integer(p as i64));
    }
    if let Some(r) = optimization.learning_rate {
        task.insert("learning_rate".into(), Value::Float(r));
    }
    if let Some(r) = optimization.reduce {
        task.insert("reduce".into(), r.name().into());
    }
    let list = |values: Result<Vec<Value>>| values.map(Value::Array);
    task.insert(
        "variable".into(),
        list(optimization.variables.iter().map(table).collect())?,
    );
    task.insert(
        "objective".into(),
        list(optimization.objectives.iter().map(table).collect())?,
    );
    if !optimization.constraints.is_empty() {
        task.insert(
            "constraint".into(),
            list(optimization.constraints.iter().map(table).collect())?,
        );
    }
    if !optimization.corners.is_empty() {
        task.insert(
            "corner".into(),
            Value::Array(
                optimization
                    .corners
                    .iter()
                    .map(super::corner_table)
                    .collect(),
            ),
        );
    }
    chip_job(name, chip, task, timeout_minutes)
}
