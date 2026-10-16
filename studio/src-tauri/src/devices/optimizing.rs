//! A device's Optimize tab: what its optimization offers (the device's objectives and
//! constraints, the outputs an expression may name, genoxide's methods), the job its form makes
//! (`photonoxide::job::optimize_from_device`), a job of it read back into the form, and the design
//! of any evaluation, which Apply takes into Design.

use photonoxide::device::{Goal, Limit, Reduce, Values};
use photonoxide::job::{self, Aim, Bound, Optimization, Variable};
use photonoxide::run::{Format, Job};
use serde::{Deserialize, Serialize};

use super::{ComputeIn, SpecIn, text};

/// One of a device's objectives, as the tab lists it.
#[derive(Debug, Serialize)]
pub struct ObjectiveInfo {
    name: String,
    symbol: String,
    unit: String,
    expression: String,
    /// `"minimize"`, `"maximize"` or `"target"`.
    goal: String,
    target: Option<f64>,
    /// The reduction it always takes (a flatness's spread), if any.
    reduce: Option<String>,
    about: String,
}

/// One of a device's constraints, as the tab lists it.
#[derive(Debug, Serialize)]
pub struct ConstraintInfo {
    name: String,
    symbol: String,
    unit: String,
    expression: String,
    /// `"at_most"` or `"at_least"`.
    limit: String,
    bound: f64,
    about: String,
}

/// An output an expression may name.
#[derive(Debug, Serialize)]
pub struct OutputInfo {
    name: String,
    unit: String,
    about: String,
    per_wavelength: bool,
}

/// One of genoxide's methods, as the tab offers it.
#[derive(Debug, Serialize)]
pub struct MethodInfo {
    name: &'static str,
    title: &'static str,
    gradient: bool,
    constraints: bool,
    objectives: usize,
    population: Option<usize>,
    random: bool,
    about: &'static str,
    paper: &'static str,
}

/// What a device's optimization offers at a design.
#[derive(Debug, Serialize)]
pub struct OptimizeInfo {
    objectives: Vec<ObjectiveInfo>,
    constraints: Vec<ConstraintInfo>,
    outputs: Vec<OutputInfo>,
    methods: Vec<MethodInfo>,
    /// The parameters whose derivatives the cheapest model gives exactly (the others are
    /// differenced).
    exact: Vec<String>,
}

impl OutputInfo {
    /// An output `name` in `unit`.
    pub fn new(name: &str, unit: &str, about: &str, per_wavelength: bool) -> OutputInfo {
        OutputInfo {
            name: name.to_owned(),
            unit: unit.to_owned(),
            about: about.to_owned(),
            per_wavelength,
        }
    }
}

impl OptimizeInfo {
    /// The device's objectives and constraints.
    pub fn offers(self) -> (Vec<ObjectiveInfo>, Vec<ConstraintInfo>) {
        (self.objectives, self.constraints)
    }
}

/// genoxide's methods, as a form offers them.
pub fn methods() -> Vec<MethodInfo> {
    job::optimize::METHODS
        .iter()
        .map(|m| MethodInfo {
            name: m.name,
            title: m.title,
            gradient: m.gradient,
            constraints: m.constraints,
            objectives: m.objectives,
            population: m.population,
            random: m.random,
            about: m.about,
            paper: m.paper,
        })
        .collect()
}

fn goal(g: Goal) -> String {
    g.name().to_owned()
}

fn limit(l: Limit) -> String {
    l.name().to_owned()
}

fn reduce(r: Reduce) -> String {
    r.name().to_owned()
}

/// What `spec`'s device offers an optimization: its objectives and constraints, the outputs of
/// its cheapest model at the design, genoxide's methods, and the parameters with exact
/// derivatives.
///
/// # Errors
///
/// What the library refuses of the design.
pub fn optimize_info(spec: &SpecIn) -> Result<OptimizeInfo, String> {
    let s = spec.spec()?;
    let (device, design) = s.build().map_err(text)?;
    let values: &Values = design.values();
    let cheapest = device.fidelities()[0].fidelity;
    let outputs =
        photonoxide::device::objective::outputs(device.as_ref(), values, cheapest).map_err(text)?;
    let exact = device
        .component(values, cheapest)
        .map(|b| {
            device
                .parameters()
                .iter()
                .zip(&b.derivatives)
                .filter(|(_, d)| d.is_some())
                .map(|(p, _)| p.name.clone())
                .collect()
        })
        .unwrap_or_default();
    Ok(OptimizeInfo {
        objectives: device
            .objectives()
            .into_iter()
            .map(|o| ObjectiveInfo {
                name: o.name,
                symbol: o.symbol,
                unit: o.unit,
                expression: o.expression,
                goal: goal(o.goal),
                target: o.target,
                reduce: o.reduce.map(reduce),
                about: o.about,
            })
            .collect(),
        constraints: device
            .constraints()
            .into_iter()
            .map(|c| ConstraintInfo {
                name: c.name,
                symbol: c.symbol,
                unit: c.unit,
                expression: c.expression,
                limit: limit(c.limit),
                bound: c.bound,
                about: c.about,
            })
            .collect(),
        outputs: outputs
            .into_iter()
            .map(|o| OutputInfo {
                name: o.name,
                unit: o.unit,
                about: o.about,
                per_wavelength: o.per_wavelength,
            })
            .collect(),
        methods: methods(),
        exact,
    })
}

#[tauri::command(async)]
pub fn device_optimize_info(spec: SpecIn) -> Result<OptimizeInfo, String> {
    optimize_info(&spec)
}

/// An optimization as the tab's form holds it: [`Optimization`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OptimizeIn {
    pub method: String,
    pub seed: u64,
    pub evaluations: usize,
    #[serde(default)]
    pub population: Option<usize>,
    #[serde(default)]
    pub learning_rate: Option<f64>,
    /// `"worst"` or `"average"`; worst when `None`.
    #[serde(default)]
    pub reduce: Option<String>,
    pub variables: Vec<Variable>,
    pub objectives: Vec<Aim>,
    #[serde(default)]
    pub constraints: Vec<Bound>,
}

impl OptimizeIn {
    pub(crate) fn optimization(&self) -> Result<Optimization, String> {
        let mut o = Optimization::new(&self.method, self.seed, self.evaluations);
        o.population = self.population;
        o.learning_rate = self.learning_rate;
        o.reduce = self
            .reduce
            .as_deref()
            .map(Reduce::named)
            .transpose()
            .map_err(text)?;
        o.variables = self.variables.clone();
        o.objectives = self.objectives.clone();
        o.constraints = self.constraints.clone();
        Ok(o)
    }

    /// The optimization an `"optimize"` task holds, as the form holds it.
    pub fn of(task: &toml::Table) -> Option<OptimizeIn> {
        if task.get("kind").and_then(toml::Value::as_str) != Some("optimize") {
            return None;
        }
        let list = |key: &str| -> Vec<toml::Value> {
            task.get(key)
                .and_then(toml::Value::as_array)
                .cloned()
                .unwrap_or_default()
        };
        let count = |key: &str| {
            task.get(key)
                .and_then(toml::Value::as_integer)
                .and_then(|i| usize::try_from(i).ok())
        };
        Some(OptimizeIn {
            method: task.get("method")?.as_str()?.to_owned(),
            seed: task
                .get("seed")
                .and_then(toml::Value::as_integer)
                .and_then(|i| u64::try_from(i).ok())?,
            evaluations: count("evaluations")?,
            population: count("population"),
            learning_rate: task.get("learning_rate").and_then(toml::Value::as_float),
            reduce: task
                .get("reduce")
                .and_then(toml::Value::as_str)
                .map(str::to_owned),
            variables: list("variable")
                .into_iter()
                .filter_map(|v| v.try_into().ok())
                .collect(),
            objectives: list("objective")
                .into_iter()
                .filter_map(|v| v.try_into().ok())
                .collect(),
            constraints: list("constraint")
                .into_iter()
                .filter_map(|v| v.try_into().ok())
                .collect(),
        })
    }
}

/// The text of the job that optimizes `spec` as `optimize` says, each design computed as
/// `compute` says, named `name`, with an opening comment saying what it optimizes and how to run
/// it.
///
/// # Errors
///
/// What the library refuses: a variable, objective or constraint the device hasn't, bounds that
/// aren't, a method that doesn't take what is asked of it.
pub fn optimize_text_of(
    spec: &SpecIn,
    compute: &ComputeIn,
    optimize: &OptimizeIn,
    name: &str,
) -> Result<String, String> {
    let s = spec.spec()?;
    let mut c = compute.compute();
    c.fdtd = None;
    let j = job::optimize_from_device(name, &s, &c, &optimize.optimization()?).map_err(text)?;
    let what = optimize
        .objectives
        .iter()
        .map(|a| {
            a.name
                .clone()
                .or_else(|| a.expression.clone())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" against ");
    let over = optimize
        .variables
        .iter()
        .map(|v| v.parameter.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let body = j.to_text(Format::Toml).map_err(text)?;
    Ok(format!(
        "# The {} optimized for {what} over {over}, by its {} model and genoxide's {}.\n#   \
         photonoxide run jobs/{name}.toml   (live in the studio's window; --headless for batches)\n{body}",
        spec.kind, spec.fidelity, optimize.method
    ))
}

/// The job the form makes of `o` for `spec`, as [`super::read_job_form`] compares a file's with.
pub(super) fn optimize_job_of(
    name: &str,
    spec: &photonoxide::device::Spec,
    compute: &ComputeIn,
    o: &OptimizeIn,
) -> Result<Job, String> {
    let mut c = compute.compute();
    c.fdtd = None;
    job::optimize_from_device(name, spec, &c, &o.optimization()?).map_err(text)
}

#[tauri::command(async)]
pub fn device_optimize_job(
    spec: SpecIn,
    compute: ComputeIn,
    optimize: OptimizeIn,
    name: String,
) -> Result<String, String> {
    optimize_text_of(&spec, &compute, &optimize, &name)
}

/// The design of the optimization run in `dir` at the point `x` (an evaluation's variables), as
/// Apply takes it into Design: the run's job's design with the variables at `x`.
///
/// # Errors
///
/// A run without its job, what the library refuses of the job, or a point that isn't one of its
/// variables'.
pub fn design_at(dir: &std::path::Path, x: &[f64]) -> Result<SpecIn, String> {
    let j = ["toml", "json", "yaml"]
        .iter()
        .find_map(|ext| Job::load(&dir.join(format!("job.{ext}"))).ok())
        .ok_or_else(|| format!("{} holds no job", dir.display()))?;
    let spec = job::design_of(&j, x).map_err(text)?;
    Ok(SpecIn::of(&spec))
}

#[tauri::command(async)]
pub fn optimize_design(dir: String, x: Vec<f64>) -> Result<SpecIn, String> {
    design_at(std::path::Path::new(&dir), &x)
}
