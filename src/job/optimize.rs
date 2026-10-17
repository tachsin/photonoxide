//! A device optimized: a job's `[device]` with a `[task]` of kind `"optimize"`.
//!
//! The task names the **variables** (the design's parameters, each between two bounds), one
//! **objective** or two (a Pareto front), any **constraints**, the band and corners each is
//! taken over, and the **method**: one of genoxide's ([`METHODS`]), with its seed and budget.
//! The job's `timeout_minutes` is required: an optimization always has a hard limit (AGENTS.md).
//!
//! ```toml
//! name = "coupler-3db"
//! timeout_minutes = 2
//!
//! [device]
//! kind = "directional coupler"
//! fidelity = "analytic"
//!
//! [task]
//! kind = "optimize"
//! method = "lbfgsb"        # genoxide's: lbfgsb, adam, mma, gcmma, nelder-mead, cmaes, de,
//! seed = 1                 #   pso, nsga2, sms-emoa
//! evaluations = 200        # the budget: the optimizer's evaluations
//! from_um = 1.53           # the band each quantity is taken over: the design's own wavelength
//! to_um = 1.57             #   alone when left out
//! points = 5
//! reduce = "worst"         # over the band and the corners: "worst" (the default) or "average"
//! # population, learning_rate: a population method's size, Adam's rate
//! # step_nm, step_z_nm, margin_um, samples, pml_cells, model_2d: the grids, as a spectrum's
//!
//! [[task.variable]]
//! parameter = "length"     # the device's, or one its design names; in its unit
//! from = 2.0
//! to = 20.0
//!
//! [[task.objective]]
//! name = "split"           # one the device declares (Device::objectives), or
//! target = 0.5             # expression = "T_o3_o1", goal = "maximize"
//!
//! [[task.constraint]]
//! name = "excess_loss"     # one the device declares (Device::constraints), or
//! at_most = 0.2            # expression = "gap", at_least = 0.12
//!
//! [[task.corner]]          # as a corners study's: each quantity is taken at the nominal
//! name = "wide"            #   design and at every corner
//! offsets = { width = 0.01 }
//! ```
//!
//! **The problem.** [`Problem`] reads and checks the task, and evaluates the objectives' costs
//! and the constraints' values at any point of the variables, with their derivatives
//! ([`Problem::evaluate`]): exact where the device's component gives them for every quantity
//! ([`crate::device::objective`]), checked against fourth-order central differences before the
//! optimizer uses them ([`Problem::check_gradients`]); central differences elsewhere. Each
//! objective's cost is the worst (largest) or the average over the band and the corners of
//! its value (minimized), its negative (maximized) or its squared distance from a target; a
//! spread (a flatness) is the largest value less the smallest. A constraint g ≤ 0 is its worst
//! value's distance past its bound.
//!
//! **The optimizer is genoxide's** (AGENTS.md: optimizers come from genoxide), and the library
//! has no genoxide dependency (#244's decision 1): the `photonoxide` program runs an
//! `"optimize"` job, live in the studio's window, through a [`Recorder`]. [`super::execute`]
//! refuses one and says so.
//!
//! **The record.** [`Recorder::begin`] records the design's scene and ports and
//! [`Event::Optimization`] (the problem, and the derivatives' checks); [`Recorder::record`] an
//! [`Event::Evaluation`] per evaluation, in the optimizer's order: the variables, each
//! objective's cost and value, the constraints' values and violation, the gradient's norm and
//! the best so far; [`Recorder::front`] an [`Event::Front`] after each generation of a run
//! with two objectives (the non-dominated evaluations and their hypervolume); and
//! [`Recorder::finish`] the [`Event::Optimum`]. The same seed gives the same record, to the
//! bit, on any number of threads.

use rayon::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use super::device::{Compute, band_of, fidelity_name, grids_of, scene_ports, spec_of};
use super::study::{CornerTask, Named, check_corners, corners_of, set};
use super::{Corner, Event, Study, task_error};
use crate::circuit::SMatrix;
use crate::device::objective::{Names, Source};
use crate::device::spec::Grids;
use crate::device::{Built, Design, Goal, Limit, Reduce, Spec};
use crate::expr::{Dim, Dual, Expr};
use crate::run::{Job, Run};
use crate::units::Wavelength;
use crate::{Error, Result};

/// One of the optimizers an `"optimize"` task names: genoxide's methods, as the `photonoxide`
/// program runs them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Method {
    /// Its name in a job, e.g. `"lbfgsb"`.
    pub name: &'static str,
    /// Its name for people, e.g. `"L-BFGS-B"`.
    pub title: &'static str,
    /// Whether it uses the objective's gradient.
    pub gradient: bool,
    /// Whether it takes constraints: by their values and Jacobian (MMA), or by Deb's
    /// feasibility rules (the others that do).
    pub constraints: bool,
    /// How many objectives it takes: 1, or 2 for a Pareto front.
    pub objectives: usize,
    /// A population method's size when the job gives none; `None` for a method of one point.
    pub population: Option<usize>,
    /// Whether it draws random numbers (its seed matters).
    pub random: bool,
    /// What it is, in a sentence.
    pub about: &'static str,
    /// The paper it implements, with its DOI.
    pub paper: &'static str,
}

/// genoxide's methods an `"optimize"` task may name.
pub const METHODS: &[Method] = &[
    Method {
        name: "lbfgsb",
        title: "L-BFGS-B",
        gradient: true,
        constraints: false,
        objectives: 1,
        population: None,
        random: false,
        about: "a quasi-Newton method within the variables' bounds, on the gradient",
        paper: "R. H. Byrd, P. Lu, J. Nocedal, C. Zhu, SIAM J. Sci. Comput. 16, 1190 (1995), \
                doi:10.1137/0916069",
    },
    Method {
        name: "adam",
        title: "Adam",
        gradient: true,
        constraints: false,
        objectives: 1,
        population: None,
        random: false,
        about: "first-order steps scaled by running moments of the gradient",
        paper: "D. P. Kingma, J. Ba, ICLR 2015, arXiv:1412.6980",
    },
    Method {
        name: "mma",
        title: "MMA",
        gradient: true,
        constraints: true,
        objectives: 1,
        population: None,
        random: false,
        about: "the method of moving asymptotes, on the gradient and the constraints' Jacobian: \
                fast, without a proof of convergence from any start",
        paper: "K. Svanberg, Int. J. Numer. Methods Eng. 24, 359 (1987), \
                doi:10.1002/nme.1620240207",
    },
    Method {
        name: "gcmma",
        title: "GCMMA",
        gradient: true,
        constraints: true,
        objectives: 1,
        population: None,
        random: false,
        about: "the globally convergent method of moving asymptotes: a point is taken only once \
                its approximations are conservative, so each is better than the last",
        paper: "K. Svanberg, SIAM J. Optim. 12, 555 (2002), doi:10.1137/S1052623499362822",
    },
    Method {
        name: "nelder-mead",
        title: "Nelder-Mead",
        gradient: false,
        constraints: true,
        objectives: 1,
        population: None,
        random: false,
        about: "a simplex of points, reflected, expanded and shrunk on the values alone",
        paper: "J. A. Nelder, R. Mead, Comput. J. 7, 308 (1965), doi:10.1093/comjnl/7.4.308",
    },
    Method {
        name: "cmaes",
        title: "CMA-ES",
        gradient: false,
        constraints: true,
        objectives: 1,
        population: Some(0),
        random: true,
        about: "an evolution strategy that adapts its search distribution's covariance",
        paper: "N. Hansen, A. Ostermeier, Evol. Comput. 9, 159 (2001), \
                doi:10.1162/106365601750190398",
    },
    Method {
        name: "de",
        title: "differential evolution",
        gradient: false,
        constraints: true,
        objectives: 1,
        population: Some(20),
        random: true,
        about: "a population moved by the differences between its members",
        paper: "R. Storn, K. Price, J. Glob. Optim. 11, 341 (1997), doi:10.1023/A:1008202821328",
    },
    Method {
        name: "pso",
        title: "particle swarm",
        gradient: false,
        constraints: true,
        objectives: 1,
        population: Some(20),
        random: true,
        about: "a swarm drawn to each particle's best and the swarm's",
        paper: "J. Kennedy, R. Eberhart, Proc. ICNN'95 4, 1942 (1995), \
                doi:10.1109/ICNN.1995.488968",
    },
    Method {
        name: "nsga2",
        title: "NSGA-II",
        gradient: false,
        constraints: true,
        objectives: 2,
        population: Some(40),
        random: true,
        about: "a genetic algorithm sorted into non-dominated fronts: a Pareto front of two \
                objectives",
        paper: "K. Deb, A. Pratap, S. Agarwal, T. Meyarivan, IEEE Trans. Evol. Comput. 6, 182 \
                (2002), doi:10.1109/4235.996017",
    },
    Method {
        name: "sms-emoa",
        title: "SMS-EMOA",
        gradient: false,
        constraints: true,
        objectives: 2,
        population: Some(40),
        random: true,
        about: "a steady-state search that keeps the front's largest hypervolume",
        paper: "N. Beume, B. Naujoks, M. Emmerich, Eur. J. Oper. Res. 181, 1653 (2007), \
                doi:10.1016/j.ejor.2006.08.008",
    },
];

/// The method named `name`.
///
/// # Errors
///
/// [`Error::Parse`] for a name that isn't one of [`METHODS`].
pub fn method(name: &str) -> Result<&'static Method> {
    METHODS.iter().find(|m| m.name == name).ok_or_else(|| {
        task_error(format!(
            "method \"{name}\": genoxide's {}",
            METHODS
                .iter()
                .map(|m| m.name)
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })
}

/// The most evaluations a task's budget takes.
const MAX_EVALUATIONS: usize = 1_000_000;

/// The most variables.
const MAX_VARIABLES: usize = 64;

/// The relative step of the central differences an optimizer's gradient takes where it isn't
/// exact: ε^(1/3), as the circuit adjoint's ([`crate::circuit::adjoint::STEP`]).
const STEP: f64 = crate::circuit::adjoint::STEP;

/// The relative step of the fourth-order differences an exact derivative is checked against
/// first (then 10 and 100 times smaller where those disagree: a ring's resonance can be sharp on
/// this step's scale, and the differences' error falls as its fourth power):
/// small enough to resolve a guide's phase over a long arm (an MZI's 150 µm, whose power's
/// period in its length is 0.4 µm), large enough that round-off stays near 1e-10.
const CHECK_STEP: f64 = 1e-5;

/// How closely an exact derivative and the fourth-order differences agree for it to be used:
/// relative, with a floor of 1e-10 of the quantity's size.
const CHECK_TOLERANCE: f64 = 1e-6;

// ---- the task as data ----

/// A variable of an optimization: a parameter of the design between two bounds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Variable {
    /// The parameter: one of the device's, or one the design's expressions name.
    pub parameter: String,
    /// The smallest value, in the parameter's unit (µm for a length, degrees for an angle).
    pub from: f64,
    /// The largest.
    pub to: f64,
}

impl Variable {
    /// `parameter` between `from` and `to`.
    pub fn new(parameter: &str, from: f64, to: f64) -> Variable {
        Variable {
            parameter: parameter.to_owned(),
            from,
            to,
        }
    }
}

/// An objective as a task gives it: one the device declares, by its symbol, or an expression
/// of its outputs, with what to do with it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Aim {
    /// A declared objective's symbol ([`crate::device::Device::objectives`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Or an expression of the outputs ([`crate::device::objective`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    /// `"minimize"`, `"maximize"` or `"target"`: an expression's (minimize by default), or in
    /// place of a declared objective's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
    /// The target, for a goal of `"target"`; a declared objective's own when left out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<f64>,
    /// `"worst"`, `"average"` or `"spread"` over the band and corners; the task's when left
    /// out (a declared spread keeps its own).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce: Option<String>,
    /// On a chip, the device instance whose own objective (or an expression of whose own
    /// outputs) this is; `None` for an expression of the chip's outputs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl Aim {
    /// The device's objective `symbol`, as it declares it.
    pub fn declared(symbol: &str) -> Aim {
        Aim {
            name: Some(symbol.to_owned()),
            ..Aim::default()
        }
    }

    /// The expression `expression`, its goal `goal`.
    pub fn expression(expression: &str, goal: Goal) -> Aim {
        Aim {
            expression: Some(expression.to_owned()),
            goal: Some(goal.name().to_owned()),
            ..Aim::default()
        }
    }

    /// The same, with the target `target`.
    #[must_use]
    pub fn with_target(self, target: f64) -> Aim {
        Aim {
            target: Some(target),
            ..self
        }
    }

    /// The same, reduced over the band and corners by `reduce`.
    #[must_use]
    pub fn with_reduce(self, reduce: Reduce) -> Aim {
        Aim {
            reduce: Some(reduce.name().to_owned()),
            ..self
        }
    }

    /// The same, of the chip's device instance `instance`: its own objective or outputs.
    #[must_use]
    pub fn of_instance(self, instance: &str) -> Aim {
        Aim {
            instance: Some(instance.to_owned()),
            ..self
        }
    }
}

/// A constraint as a task gives it: one the device declares, by its symbol, or an expression of
/// its outputs, with its bound.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Bound {
    /// A declared constraint's symbol ([`crate::device::Device::constraints`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Or an expression of the outputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    /// The largest value allowed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_most: Option<f64>,
    /// The smallest value allowed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_least: Option<f64>,
    /// `"worst"` (the default), `"average"` or `"spread"` over the band and corners.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce: Option<String>,
    /// On a chip, the device instance whose own constraint (or an expression of whose own
    /// outputs) this is: a smallest gap, a footprint; `None` for an expression of the chip's
    /// outputs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl Bound {
    /// The device's constraint `symbol`, at its declared bound.
    pub fn declared(symbol: &str) -> Bound {
        Bound {
            name: Some(symbol.to_owned()),
            ..Bound::default()
        }
    }

    /// The expression `expression` kept on the `limit` side of `bound`.
    pub fn expression(expression: &str, limit: Limit, bound: f64) -> Bound {
        Bound {
            expression: Some(expression.to_owned()),
            ..Bound::default()
        }
        .limited(limit, bound)
    }

    /// The same, kept on the `limit` side of `bound`.
    #[must_use]
    pub fn limited(self, limit: Limit, bound: f64) -> Bound {
        match limit {
            Limit::AtMost => Bound {
                at_most: Some(bound),
                at_least: None,
                ..self
            },
            Limit::AtLeast => Bound {
                at_least: Some(bound),
                at_most: None,
                ..self
            },
        }
    }

    /// The same, of the chip's device instance `instance`: its own constraint or outputs.
    #[must_use]
    pub fn of_instance(self, instance: &str) -> Bound {
        Bound {
            instance: Some(instance.to_owned()),
            ..self
        }
    }
}

/// An optimization of a device, as [`optimize_from_device`] writes it into a job.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Optimization {
    /// The method: one of [`METHODS`], by name.
    pub method: String,
    /// The seed of its random numbers.
    pub seed: u64,
    /// The budget: the optimizer's evaluations.
    pub evaluations: usize,
    /// A population method's size; the method's own when `None`.
    pub population: Option<usize>,
    /// Adam's learning rate, in the variables scaled to their ranges; 0.05 when `None`.
    pub learning_rate: Option<f64>,
    /// The variables.
    pub variables: Vec<Variable>,
    /// One objective, or two.
    pub objectives: Vec<Aim>,
    /// The constraints.
    pub constraints: Vec<Bound>,
    /// Corners each quantity is also taken at, beside the nominal design.
    pub corners: Vec<Corner>,
    /// `"worst"` or `"average"` over the band and corners; worst when `None`.
    pub reduce: Option<Reduce>,
}

impl Optimization {
    /// `method` with `seed` and a budget of `evaluations`, without variables or objectives yet.
    pub fn new(method: &str, seed: u64, evaluations: usize) -> Optimization {
        Optimization {
            method: method.to_owned(),
            seed,
            evaluations,
            population: None,
            learning_rate: None,
            variables: Vec::new(),
            objectives: Vec::new(),
            constraints: Vec::new(),
            corners: Vec::new(),
            reduce: None,
        }
    }

    /// The same, with `variable` added.
    #[must_use]
    pub fn variable(mut self, variable: Variable) -> Optimization {
        self.variables.push(variable);
        self
    }

    /// The same, with `objective` added.
    #[must_use]
    pub fn objective(mut self, objective: Aim) -> Optimization {
        self.objectives.push(objective);
        self
    }

    /// The same, with `constraint` added.
    #[must_use]
    pub fn constraint(mut self, constraint: Bound) -> Optimization {
        self.constraints.push(constraint);
        self
    }

    /// The same, with the corners `corners`.
    #[must_use]
    pub fn corners(mut self, corners: Vec<Corner>) -> Optimization {
        self.corners = corners;
        self
    }

    /// The same, a population method's size `size`.
    #[must_use]
    pub fn population(mut self, size: usize) -> Optimization {
        self.population = Some(size);
        self
    }

    /// The same, reduced over the band and corners by `reduce`.
    #[must_use]
    pub fn reduce(mut self, reduce: Reduce) -> Optimization {
        self.reduce = Some(reduce);
        self
    }
}

/// The job that optimizes `spec`'s device as `optimization` says, each design computed as
/// `compute` says (its own model at the design's fidelity, on the grids it sets; the quantities
/// taken over its band, or at the design's own wavelength when it gives none), named `name`,
/// with a hard limit of `compute`'s timeout (30 minutes when it gives none). It is checked by
/// [`super::check`].
///
/// # Errors
///
/// [`Error::InvalidValue`] for a `compute` by FDTD; the errors of [`super::check`] for the job it
/// makes: a variable, objective or constraint the device hasn't, bounds that aren't, a method
/// that doesn't take what is asked of it.
pub fn optimize_from_device(
    name: &str,
    spec: &Spec,
    compute: &Compute,
    optimization: &Optimization,
) -> Result<Job> {
    if compute.fdtd.is_some() {
        return Err(Error::invalid(
            "optimize",
            "an optimization computes each design by the device's own model at its fidelity; \
             FDTD runs one design at a time (a job of kind \"fdtd\")",
        ));
    }
    let float = Value::Float;
    let mut task = Table::new();
    task.insert("kind".into(), "optimize".into());
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
        task.insert("learning_rate".into(), float(r));
    }
    if let Some(r) = optimization.reduce {
        task.insert("reduce".into(), r.name().into());
    }
    if let Some((from, to, points)) = compute.band {
        task.insert("from_um".into(), float(from));
        task.insert("to_um".into(), float(to));
        task.insert("points".into(), Value::Integer(points as i64));
    }
    for (key, v) in [
        ("step_nm", compute.step_nm),
        ("step_z_nm", compute.step_z_nm),
        ("margin_um", compute.margin_um),
    ] {
        if let Some(v) = v {
            task.insert(key.into(), float(v));
        }
    }
    for (key, v) in [
        ("samples", compute.samples),
        ("pml_cells", compute.pml_cells),
    ] {
        if let Some(n) = v {
            task.insert(key.into(), Value::Integer(n as i64));
        }
    }
    if compute.fdfd_2d {
        task.insert("model_2d".into(), "fdfd".into());
    }
    let variables = optimization
        .variables
        .iter()
        .map(table)
        .collect::<Result<Vec<_>>>()?;
    task.insert("variable".into(), Value::Array(variables));
    let objectives = optimization
        .objectives
        .iter()
        .map(table)
        .collect::<Result<Vec<_>>>()?;
    task.insert("objective".into(), Value::Array(objectives));
    if !optimization.constraints.is_empty() {
        let constraints = optimization
            .constraints
            .iter()
            .map(table)
            .collect::<Result<Vec<_>>>()?;
        task.insert("constraint".into(), Value::Array(constraints));
    }
    if !optimization.corners.is_empty() {
        task.insert(
            "corner".into(),
            Value::Array(optimization.corners.iter().map(corner_table).collect()),
        );
    }
    let mut file = Table::new();
    file.insert("name".into(), name.into());
    file.insert(
        "timeout_minutes".into(),
        float(compute.timeout_minutes.unwrap_or(30.0)),
    );
    file.insert("device".into(), Value::Table(spec.to_table()));
    file.insert("task".into(), Value::Table(task));
    let text = toml::to_string(&file).map_err(|e| task_error(e.to_string()))?;
    let job = Job::parse(&text)?;
    super::check(&job)?;
    Ok(job)
}

/// A table of a task, as TOML.
fn table<T: Serialize>(v: &T) -> Result<Value> {
    Value::try_from(v).map_err(|e| task_error(e.to_string()))
}

/// A corner as a `[[task.corner]]` table.
pub(super) fn corner_table(c: &Corner) -> Value {
    let mut t = Table::new();
    t.insert("name".into(), Value::String(c.name.clone()));
    if !c.offsets.is_empty() {
        let o: Table = c
            .offsets
            .iter()
            .map(|(n, v)| (n.clone(), Value::Float(*v)))
            .collect();
        t.insert("offsets".into(), Value::Table(o));
    }
    if let Some(k) = c.temperature_k {
        t.insert("temperature_k".into(), Value::Float(k));
    }
    Value::Table(t)
}

// ---- the task read ----

/// An `"optimize"` task.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct OptimizeTask {
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    method: String,
    seed: Option<u64>,
    evaluations: Option<usize>,
    population: Option<usize>,
    learning_rate: Option<f64>,
    reduce: Option<String>,
    from_um: Option<f64>,
    to_um: Option<f64>,
    points: Option<usize>,
    step_nm: Option<f64>,
    step_z_nm: Option<f64>,
    margin_um: Option<f64>,
    samples: Option<usize>,
    pml_cells: Option<usize>,
    model_2d: Option<String>,
    #[serde(default)]
    variable: Vec<VariableTask>,
    #[serde(default)]
    objective: Vec<Aim>,
    #[serde(default)]
    constraint: Vec<Bound>,
    #[serde(default)]
    corner: Vec<CornerTask>,
}

/// A `[[task.variable]]`.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct VariableTask {
    parameter: String,
    from: f64,
    to: f64,
}

/// A quantity of the problem: an objective or a constraint, compiled.
#[derive(Clone, Debug)]
struct Quantity {
    name: String,
    symbol: String,
    unit: String,
    text: String,
    expression: Expr,
    /// An objective's goal and target, or `None` for a constraint.
    goal: Option<(Goal, f64)>,
    /// A constraint's limit and bound.
    limit: Option<(Limit, f64)>,
    reduce: Reduce,
    /// The figures it names, by symbol.
    figures: Vec<String>,
    /// On a chip, the instance whose own outputs it is of; `None` for the chip's (and a
    /// device's).
    instance: Option<usize>,
}

impl Quantity {
    /// Its goal or limit, as the record names it.
    fn aim(&self) -> &'static str {
        match (self.goal, self.limit) {
            (Some((g, _)), _) => g.name(),
            (_, Some((l, _))) => l.name(),
            _ => unreachable!("a quantity is an objective or a constraint"),
        }
    }

    fn event(&self) -> OptimizeQuantity {
        OptimizeQuantity {
            name: self.name.clone(),
            symbol: self.symbol.clone(),
            unit: self.unit.clone(),
            expression: self.text.clone(),
            aim: self.aim().to_owned(),
            target: match (self.goal, self.limit) {
                (Some((Goal::Target, t)), _) => Some(t),
                (_, Some((_, b))) => Some(b),
                _ => None,
            },
            reduce: self.reduce.name().to_owned(),
        }
    }
}

/// An `"optimize"` job read and checked: the design, its variables and quantities, the method
/// and its settings, ready to evaluate any point of the variables ([`Problem::evaluate`]).
#[derive(Debug)]
pub struct Problem {
    target: Target,
    kind: String,
    wavelengths: Vec<f64>,
    corners: Vec<Corner>,
    variables: Vec<Variable>,
    units: Vec<String>,
    /// Per variable: a named parameter that is an angle (degrees here, radians to the design).
    angle: Vec<bool>,
    start: Vec<f64>,
    method: &'static Method,
    seed: u64,
    evaluations: usize,
    population: Option<usize>,
    learning_rate: Option<f64>,
    names: Names,
    objectives: Vec<Quantity>,
    constraints: Vec<Quantity>,
    /// Per variable: its derivatives exact, once checked ([`Problem::check_gradients`]).
    exact: Vec<bool>,
    timeout_minutes: f64,
}

/// What an optimization designs: a device, or a chip of devices.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // one per optimization
enum Target {
    /// A job's `[device]`, computed on the task's grids.
    Device { spec: Spec, grids: Grids },
    /// A job's `[circuit]`.
    Chip(Box<chip::ChipTarget>),
}

/// What a point of the variables gives: each objective's cost and value, each constraint's
/// value g (g ≤ 0 is met), and, when asked, their gradients in the variables.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Evaluation {
    /// Each objective's cost, minimized: its value, its negative, or its squared distance from
    /// its target, reduced over the band and corners. NaN where the model gave nothing.
    pub costs: Vec<f64>,
    /// Each objective's value: at the worst sample, the average, or the spread.
    pub values: Vec<f64>,
    /// Each constraint's g: its reduced value's distance past its bound (≤ 0 when met).
    pub constraints: Vec<f64>,
    /// Σ max(0, g): 0 when every constraint is met.
    pub violation: f64,
    /// Each objective's ∂cost/∂x, in the variables' units, when asked; empty otherwise.
    pub gradients: Vec<Vec<f64>>,
    /// Each constraint's ∂g/∂x, when asked.
    pub jacobian: Vec<Vec<f64>>,
    /// The device's models built for it: one per corner, and more for differences.
    pub solves: usize,
    /// Why the model gave nothing there (a fit outside its range, a solve that failed).
    pub error: Option<String>,
}

impl Evaluation {
    fn failed(problem: &Problem, error: &Error, solves: usize) -> Evaluation {
        Evaluation {
            costs: vec![f64::NAN; problem.objectives.len()],
            values: vec![f64::NAN; problem.objectives.len()],
            constraints: vec![f64::NAN; problem.constraints.len()],
            violation: f64::NAN,
            gradients: Vec::new(),
            jacobian: Vec::new(),
            solves,
            error: Some(error.to_string()),
        }
    }

    /// Whether every constraint is met.
    pub fn feasible(&self) -> bool {
        self.violation == 0.0
    }
}

/// An exact derivative checked against fourth-order central differences, as the record keeps
/// it ([`Event::Optimization`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GradientCheck {
    /// The variable.
    pub variable: String,
    /// The objective's or constraint's symbol.
    pub quantity: String,
    /// The exact derivative.
    pub exact: f64,
    /// The fourth-order central differences'.
    pub differences: f64,
    /// Whether they agree to 1e-6, relative (with a floor of 1e-10 of the
    /// quantity): an exact derivative is used only where every quantity's does.
    pub agrees: bool,
}

/// A variable as the record keeps it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OptimizeVariable {
    /// The parameter.
    pub parameter: String,
    /// Its unit: `"µm"`, `"°"`, or `""`.
    pub unit: String,
    /// The smallest value.
    pub from: f64,
    /// The largest.
    pub to: f64,
    /// Its value in the design the run starts from.
    pub start: f64,
    /// How its derivatives are taken: `"exact"`, `"differences"` (central), or `"none"` (the
    /// method uses values only).
    pub derivatives: String,
}

/// An objective or constraint as the record keeps it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OptimizeQuantity {
    /// What it is.
    pub name: String,
    /// Its symbol (a declared one's), or `"objective 1"` for an expression.
    pub symbol: String,
    /// Its unit.
    pub unit: String,
    /// The expression of the outputs.
    pub expression: String,
    /// An objective's goal, `"minimize"`, `"maximize"` or `"target"`; a constraint's limit,
    /// `"at_most"` or `"at_least"`.
    pub aim: String,
    /// The target, or the constraint's bound.
    pub target: Option<f64>,
    /// `"worst"`, `"average"` or `"spread"` over the band and corners.
    pub reduce: String,
}

/// The settings every optimization's task gives: its method, hard limit, seed, budget and
/// reduction.
struct Settings {
    method: &'static Method,
    timeout_minutes: f64,
    seed: u64,
    evaluations: usize,
    reduce: Reduce,
}

/// A task's settings read and checked: a known method, a hard limit, a seed, a budget, a
/// population for a population method only, a learning rate for Adam only, and a reduction that
/// isn't a spread.
#[allow(clippy::too_many_arguments)]
fn settings(
    job: &Job,
    method_name: &str,
    seed: Option<u64>,
    evaluations: Option<usize>,
    population: Option<usize>,
    learning_rate: Option<f64>,
    reduce: Option<&str>,
) -> Result<Settings> {
    let method = method(method_name)?;
    let timeout_minutes = job
        .timeout()
        .map(|t| t.as_secs_f64() / 60.0)
        .ok_or_else(|| {
            task_error(
                "an optimization needs a hard limit: timeout_minutes (AGENTS.md: a hard \
                 timeout is always set)",
            )
        })?;
    let seed = seed
        .ok_or_else(|| task_error("an optimization needs its seed: seed = 1 (any whole number)"))?;
    let evaluations = evaluations.ok_or_else(|| {
        task_error("an optimization needs its budget: evaluations = 200 (the optimizer's)")
    })?;
    if evaluations == 0 || evaluations > MAX_EVALUATIONS {
        return Err(task_error(format!(
            "the budget takes 1 to {MAX_EVALUATIONS} evaluations, got {evaluations}"
        )));
    }
    if let Some(p) = population {
        if method.population.is_none() {
            return Err(task_error(format!(
                "{} isn't a population method: it takes no population",
                method.title
            )));
        }
        if !(4..=10_000).contains(&p) {
            return Err(task_error(format!(
                "a population takes 4 to 10000 members, got {p}"
            )));
        }
    }
    if let Some(r) = learning_rate {
        if method.name != "adam" {
            return Err(task_error(format!(
                "{} takes no learning rate: Adam's",
                method.title
            )));
        }
        if !(r.is_finite() && r > 0.0 && r <= 1.0) {
            return Err(task_error(format!(
                "Adam's learning rate, in the variables scaled to their ranges, is above 0 \
                 and at most 1, got {r}"
            )));
        }
    }
    let reduce = match reduce {
        None => Reduce::Worst,
        Some(r) => match Reduce::named(r)? {
            Reduce::Spread => {
                return Err(task_error(
                    "the task's reduction is \"worst\" or \"average\"; a spread is an \
                     objective's own",
                ));
            }
            r => r,
        },
    };
    Ok(Settings {
        method,
        timeout_minutes,
        seed,
        evaluations,
        reduce,
    })
}

/// The variables' derivatives' names, as the record keeps them.
fn derivative_name(method: &Method, exact: bool) -> &'static str {
    match (method.gradient, exact) {
        (false, _) => "none",
        (true, true) => "exact",
        (true, false) => "differences",
    }
}

/// Whether `job` optimizes its device (a `[device]` with a task of kind `"optimize"`): a job
/// the `photonoxide` program runs, its optimizer genoxide's.
pub fn optimizes(job: &Job) -> bool {
    (job.device().is_some() || job.circuit().is_some())
        && job.task().get("kind").and_then(Value::as_str) == Some("optimize")
}

impl Problem {
    /// `job`'s optimization, read and checked without solving (but for the cheapest model at
    /// the start, which names the figures its expressions may use).
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for a task that isn't one: an unknown method, no seed, budget or hard
    /// limit, no variable or more than 64, a variable twice or with bounds that aren't (or
    /// outside its parameter's range, or that the design's value isn't between), no objective,
    /// more than the method takes, a constraint asked of a method that takes none; and
    /// [`Error::InvalidValue`] for an objective or constraint the device hasn't, an expression
    /// that doesn't parse, and what [`Spec::build`] and the corners refuse.
    pub fn of(job: &Job) -> Result<Problem> {
        if job.circuit().is_some() {
            return chip::of(job);
        }
        let spec = spec_of(job)?;
        let task: OptimizeTask = job
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
        if task.objective.iter().any(|a| a.instance.is_some())
            || task.constraint.iter().any(|b| b.instance.is_some())
        {
            return Err(task_error(
                "a device's objectives and constraints name no instance: that is a chip's",
            ));
        }
        let grids = grids_of(
            task.step_nm,
            task.step_z_nm,
            task.margin_um,
            task.samples,
            task.pml_cells,
            task.model_2d.as_deref(),
        )?;
        let (device, design) = spec.build()?;
        spec.device_with(&grids)?;
        spec.fdfd_size(&grids)?;
        // the band: the task's, or the design's own wavelength
        let wavelengths = match (task.from_um, task.to_um) {
            (None, None) => {
                if task.points.is_some() {
                    return Err(task_error(
                        "points needs from_um and to_um: the band is the design's own \
                         wavelength alone without them",
                    ));
                }
                let at = design.values().get("wavelength").unwrap_or(1.55);
                vec![Wavelength::um(at)?.to_um()]
            }
            _ => band_of(task.from_um, task.to_um, task.points, 5, &design)?,
        };
        if wavelengths.len() > 1001 {
            return Err(task_error(format!(
                "an optimization takes each quantity at most at 1001 wavelengths, got {}",
                wavelengths.len()
            )));
        }
        let corners = corners_of(&task.corner)?;
        if !corners.is_empty() {
            check_corners(&corners)?;
            Study::Corners(corners.clone()).designs(&spec)?;
        }
        // the variables
        let named = Named {
            device: device.as_ref(),
            design: &design,
            spec: &spec,
        };
        if task.variable.is_empty() || task.variable.len() > MAX_VARIABLES {
            return Err(task_error(format!(
                "an optimization takes 1 to {MAX_VARIABLES} variables ([[task.variable]]), got {}",
                task.variable.len()
            )));
        }
        let mut variables = Vec::new();
        let mut units = Vec::new();
        let mut angle = Vec::new();
        let mut start = Vec::new();
        for (k, v) in task.variable.iter().enumerate() {
            named.check(&v.parameter)?;
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
            let (unit, is_angle) = match device.parameters().iter().find(|p| p.name == v.parameter)
            {
                Some(p) => {
                    if v.from < p.min || v.to > p.max {
                        return Err(task_error(format!(
                            "the variable {} takes {} to {} {}: a {}'s range, not {} to {}",
                            v.parameter,
                            p.min,
                            p.max,
                            p.unit,
                            device.kind(),
                            v.from,
                            v.to
                        )));
                    }
                    (p.unit.clone(), false)
                }
                None => match named.dim(&v.parameter) {
                    Some(d) if d == Dim::LENGTH => ("µm".to_owned(), false),
                    Some(d) if d == Dim::ANGLE => ("°".to_owned(), true),
                    _ => (String::new(), false),
                },
            };
            let x0 = named.nominal(&v.parameter);
            if !(v.from..=v.to).contains(&x0) {
                return Err(task_error(format!(
                    "the design's {} is {x0}, outside its variable's bounds {} to {}: an \
                     optimization starts from the design",
                    v.parameter, v.from, v.to
                )));
            }
            variables.push(Variable::new(&v.parameter, v.from, v.to));
            units.push(unit);
            angle.push(is_angle);
            start.push(x0);
        }
        // the quantities, on the cheapest model's outputs
        let cheapest = device.fidelities()[0].fidelity;
        let built = device.component(design.values(), cheapest).ok();
        let names = Names::of(device.as_ref(), design.values(), built.as_ref())?;
        let declared = device.objectives();
        let declared_constraints = device.constraints();
        if task.objective.is_empty() || task.objective.len() > method.objectives {
            return Err(task_error(format!(
                "{} takes {} objective{} ([[task.objective]]), got {}{}",
                method.title,
                method.objectives,
                if method.objectives == 1 { "" } else { "s" },
                task.objective.len(),
                if task.objective.len() == 2 {
                    ": two objectives are a Pareto front, by nsga2 or sms-emoa"
                } else {
                    ""
                }
            )));
        }
        if method.objectives == 2 && task.objective.len() != 2 {
            return Err(task_error(format!(
                "{} traces a Pareto front of two objectives, got {}",
                method.title,
                task.objective.len()
            )));
        }
        let mut objectives = Vec::new();
        for (k, a) in task.objective.iter().enumerate() {
            let label = format!("objective {}", k + 1);
            let (name, symbol, unit, text, goal, target, fixed) = match (&a.name, &a.expression) {
                (Some(n), None) => {
                    let o = declared.iter().find(|o| o.symbol == *n).ok_or_else(|| {
                        Error::invalid(
                            "objective",
                            format!(
                                "a {} has no objective \"{n}\": it has {}; or give an \
                                     expression of its outputs",
                                device.kind(),
                                declared
                                    .iter()
                                    .map(|o| o.symbol.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                        )
                    })?;
                    let goal = match &a.goal {
                        Some(g) => Goal::named(g)?,
                        None => o.goal,
                    };
                    (
                        o.name.clone(),
                        o.symbol.clone(),
                        o.unit.clone(),
                        o.expression.clone(),
                        goal,
                        a.target.or(o.target),
                        o.reduce,
                    )
                }
                (None, Some(e)) => {
                    let goal = Goal::named(a.goal.as_deref().unwrap_or("minimize"))?;
                    (
                        e.clone(),
                        label.clone(),
                        String::new(),
                        e.clone(),
                        goal,
                        a.target,
                        None,
                    )
                }
                _ => {
                    return Err(task_error(format!(
                        "{label} needs a name (one the device declares) or an \
                             expression, one of them"
                    )));
                }
            };
            let target = match goal {
                Goal::Target => Some(target.ok_or_else(|| {
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
            let expression = names.parse(&label, &text)?;
            objectives.push(Quantity {
                figures: figures_of(&names, &expression),
                name,
                symbol,
                unit,
                text,
                expression,
                goal: Some((goal, target.unwrap_or(0.0))),
                limit: None,
                reduce,
                instance: None,
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
            let (name, symbol, unit, text, declared_bound) = match (&b.name, &b.expression) {
                (Some(n), None) => {
                    let c = declared_constraints
                        .iter()
                        .find(|c| c.symbol == *n)
                        .ok_or_else(|| {
                            Error::invalid(
                                "constraint",
                                format!(
                                    "a {} has no constraint \"{n}\": it has {}; or give an \
                                     expression of its outputs",
                                    device.kind(),
                                    declared_constraints
                                        .iter()
                                        .map(|c| c.symbol.as_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                            )
                        })?;
                    (
                        c.name.clone(),
                        c.symbol.clone(),
                        c.unit.clone(),
                        c.expression.clone(),
                        Some((c.limit, c.bound)),
                    )
                }
                (None, Some(e)) => (e.clone(), label.clone(), String::new(), e.clone(), None),
                _ => {
                    return Err(task_error(format!(
                        "{label} needs a name (one the device declares) or an expression, one \
                         of them"
                    )));
                }
            };
            let limit = match (b.at_most, b.at_least, declared_bound) {
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
                        "{label} ({symbol}) takes at_most or at_least, not both: two \
                         constraints keep a quantity between two bounds"
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
            let expression = names.parse(&label, &text)?;
            constraints.push(Quantity {
                figures: figures_of(&names, &expression),
                name,
                symbol,
                unit,
                text,
                expression,
                goal: None,
                limit: Some(limit),
                reduce,
                instance: None,
            });
        }
        let n = variables.len();
        Ok(Problem {
            kind: device.kind().to_owned(),
            target: Target::Device { spec, grids },
            wavelengths,
            corners,
            variables,
            units,
            angle,
            start,
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
        })
    }

    /// The variables, in the order a point lists them.
    pub fn variables(&self) -> &[Variable] {
        &self.variables
    }

    /// The design's values of the variables: where the run starts.
    pub fn start(&self) -> &[f64] {
        &self.start
    }

    /// The method.
    pub fn method(&self) -> &'static Method {
        self.method
    }

    /// The seed.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The budget: the optimizer's evaluations.
    pub fn evaluations(&self) -> usize {
        self.evaluations
    }

    /// A population method's size: the task's, or the method's own (`None` for CMA-ES's own
    /// default, and for a method of one point).
    pub fn population(&self) -> Option<usize> {
        self.population
    }

    /// Adam's learning rate, in the variables scaled to their ranges: the task's, or 0.05.
    pub fn learning_rate(&self) -> f64 {
        self.learning_rate.unwrap_or(0.05)
    }

    /// How many objectives.
    pub fn objectives(&self) -> usize {
        self.objectives.len()
    }

    /// How many constraints.
    pub fn constraints(&self) -> usize {
        self.constraints.len()
    }

    /// The hard limit, minutes.
    pub fn timeout_minutes(&self) -> f64 {
        self.timeout_minutes
    }

    /// The design at `x`: the job's, the variables written as plain numbers in place of their
    /// expressions (a length's in µm, an angle's in degrees) — what Apply takes into the design.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a point that isn't one of the variables'.
    pub fn design_at(&self, x: &[f64]) -> Result<Spec> {
        if x.len() != self.variables.len() {
            return Err(Error::invalid(
                "optimize",
                format!(
                    "a point of {} variables, not {}",
                    self.variables.len(),
                    x.len()
                ),
            ));
        }
        let Target::Device { spec: own, .. } = &self.target else {
            return Err(Error::invalid(
                "optimize",
                "a chip's optimization designs a chip (Problem::chip_at), not a device",
            ));
        };
        let (device, design) = own.build()?;
        let named = Named {
            device: device.as_ref(),
            design: &design,
            spec: own,
        };
        let mut spec = own.clone();
        for (v, &value) in self.variables.iter().zip(x) {
            set(&mut spec, &v.parameter, named.literal(&v.parameter, value));
        }
        Ok(spec)
    }

    /// The parameters' expressions of the design at `x`, as Apply takes them: a device's every
    /// parameter ([`Problem::design_at`]); a chip's variables, `instance.parameter`, each a
    /// number in its unit (a name the design defines with its unit).
    ///
    /// # Errors
    ///
    /// As [`Problem::design_at`].
    pub fn parameters_at(&self, x: &[f64]) -> Result<Vec<(String, String)>> {
        match &self.target {
            Target::Device { .. } => Ok(self.design_at(x)?.parameters),
            Target::Chip(t) => chip::parameters_at(self, t, x),
        }
    }

    /// The chip at `x`, for a chip's optimization: its instances with the variables at `x`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a device's optimization, or a point that isn't one of the
    /// variables'.
    pub fn chip_at(&self, x: &[f64]) -> Result<super::ChipDesign> {
        match &self.target {
            Target::Chip(t) => chip::chip_at(t, x),
            Target::Device { .. } => Err(Error::invalid(
                "optimize",
                "a device's optimization designs a device (Problem::design_at), not a chip",
            )),
        }
    }

    /// The objectives and constraints at `x`, with their gradients when `gradient` is set (and
    /// the method uses them): exact for the variables whose derivatives are
    /// ([`Problem::check_gradients`]), central differences for the others. A point where the
    /// model gives nothing (a fit outside its range) has NaN costs and says why.
    pub fn evaluate(&self, x: &[f64], gradient: bool) -> Evaluation {
        let n = self.variables.len();
        let wanted: Vec<usize> = if gradient {
            (0..n).filter(|&j| self.exact[j]).collect()
        } else {
            Vec::new()
        };
        let point = match self.point(x, &wanted) {
            Ok(p) => p,
            Err(e) => return Evaluation::failed(self, &e, 0),
        };
        let reduced: Vec<Reduced> = self
            .quantities()
            .zip(&point.samples)
            .map(|(q, s)| reduce(q, s))
            .collect();
        let no = self.objectives.len();
        let mut e = Evaluation {
            costs: reduced[..no].iter().map(|r| r.result).collect(),
            values: reduced[..no].iter().map(|r| r.value).collect(),
            constraints: reduced[no..].iter().map(|r| r.result).collect(),
            violation: 0.0,
            gradients: Vec::new(),
            jacobian: Vec::new(),
            solves: point.solves,
            error: None,
        };
        // (+ 0.0: an empty sum is −0.0, which a record would keep)
        e.violation = e.constraints.iter().map(|g| g.max(0.0)).sum::<f64>() + 0.0;
        if let Some(bad) = e
            .costs
            .iter()
            .chain(&e.constraints)
            .position(|v| !v.is_finite())
        {
            let q = self.quantities().nth(bad).map_or("", |q| q.symbol.as_str());
            e.error = Some(format!(
                "{q} isn't a number there (a power of 0 in dB, a division by 0)"
            ));
            return e;
        }
        if !gradient {
            return e;
        }
        // the gradient, variable by variable: exact where it is, differences elsewhere
        let nq = reduced.len();
        let mut columns: Vec<Option<Vec<f64>>> = vec![None; n];
        for (i, &j) in wanted.iter().enumerate() {
            if let Some(d) = &point.derivatives[i] {
                columns[j] = Some(
                    (0..nq)
                        .map(|k| {
                            reduced[k]
                                .weights
                                .iter()
                                .zip(&d[k])
                                .map(|(w, v)| if *w == 0.0 { 0.0 } else { w * v })
                                .sum()
                        })
                        .collect(),
                );
            }
        }
        let base: Vec<f64> = e.costs.iter().chain(&e.constraints).copied().collect();
        let differenced: Vec<(usize, Differenced)> = (0..n)
            .filter(|&j| columns[j].is_none())
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|j| (j, self.difference(x, j, &base)))
            .collect();
        for (j, d) in differenced {
            match d {
                Ok((column, solves)) => {
                    e.solves += solves;
                    columns[j] = Some(column);
                }
                Err(err) => {
                    let mut failed = Evaluation::failed(self, &err, e.solves);
                    failed.error = Some(format!(
                        "the differences in {}: {err}",
                        self.variables[j].parameter
                    ));
                    return failed;
                }
            }
        }
        let column = |j: usize, k: usize| columns[j].as_ref().map_or(f64::NAN, |c| c[k]);
        e.gradients = (0..no)
            .map(|k| (0..n).map(|j| column(j, k)).collect())
            .collect();
        e.jacobian = (no..nq)
            .map(|k| (0..n).map(|j| column(j, k)).collect())
            .collect();
        e
    }

    /// Checks each variable's exact derivatives at the start against fourth-order central
    /// differences, every objective's cost and every constraint's value, and uses them from then
    /// on where they all agree; a variable whose don't (or that has none: a figure, a width
    /// that builds another component) is differenced. A method on values alone checks nothing.
    ///
    /// # Errors
    ///
    /// The model's at the start, or at a point of the differences.
    pub fn check_gradients(&mut self) -> Result<Vec<GradientCheck>> {
        self.exact = vec![false; self.variables.len()];
        if !self.method.gradient {
            return Ok(Vec::new());
        }
        let n = self.variables.len();
        let all: Vec<usize> = (0..n).collect();
        let x = self.start.clone();
        let point = self.point(&x, &all)?;
        let symbols: Vec<String> = self.quantities().map(|q| q.symbol.clone()).collect();
        let mut checks = Vec::new();
        let mut exact = vec![false; n];
        for (j, derivatives) in point.derivatives.iter().enumerate() {
            let Some(d) = derivatives else {
                continue;
            };
            // each quantity at each sample, before the reduction (whose worst case has kinks
            // where the worst sample changes); where the differences' own error is larger than
            // the tolerance (a resonance sharp on the step's scale), at steps 10 and 100 times
            // smaller, their error falling as the step's fourth power
            let compare = |fourth: &[Vec<f64>]| -> Vec<(bool, f64, f64)> {
                d.iter()
                    .zip(fourth)
                    .zip(&point.samples)
                    .map(|((dk, fk), values)| {
                        let mut worst: Option<(f64, f64, f64)> = None;
                        let mut agrees = true;
                        for ((&ex, &fd), &value) in dk.iter().zip(fk).zip(values) {
                            let miss = (ex - fd).abs();
                            agrees &= miss
                                <= CHECK_TOLERANCE * (ex.abs() + fd.abs())
                                    + 1e-10 * (1.0 + value.abs());
                            if worst.is_none_or(|w| miss > w.2) {
                                worst = Some((ex, fd, miss));
                            }
                        }
                        let (ex, fd, _) = worst.unwrap_or((0.0, 0.0, 0.0));
                        (agrees, ex, fd)
                    })
                    .collect()
            };
            let mut found = compare(&self.fourth_order(&x, j, CHECK_STEP)?);
            for finer in [CHECK_STEP / 10.0, CHECK_STEP / 100.0] {
                if found.iter().all(|f| f.0) {
                    break;
                }
                let again = compare(&self.fourth_order(&x, j, finer)?);
                for (f, a) in found.iter_mut().zip(again) {
                    if !f.0 {
                        *f = a;
                    }
                }
            }
            let mut all_agree = true;
            for (k, &(agrees, ex, fd)) in found.iter().enumerate() {
                all_agree &= agrees;
                checks.push(GradientCheck {
                    variable: self.variables[j].parameter.clone(),
                    quantity: symbols[k].clone(),
                    exact: ex,
                    differences: fd,
                    agrees,
                });
            }
            exact[j] = all_agree;
        }
        self.exact = exact;
        Ok(checks)
    }

    /// Per variable, how its derivatives are taken: `"exact"`, `"differences"`, or `"none"`.
    pub fn derivatives(&self) -> Vec<&'static str> {
        self.exact
            .iter()
            .map(|&e| derivative_name(self.method, e))
            .collect()
    }

    fn quantities(&self) -> impl Iterator<Item = &Quantity> {
        self.objectives.iter().chain(&self.constraints)
    }

    /// The values of the costs and constraints at `x`, without derivatives, and the models
    /// built.
    fn scalars(&self, x: &[f64]) -> Result<(Vec<f64>, usize)> {
        let p = self.point(x, &[])?;
        let values = self
            .quantities()
            .zip(&p.samples)
            .map(|(q, s)| reduce(q, s).result)
            .collect();
        Ok((values, p.solves))
    }

    /// The step of the differences in variable `j` at `x`, `relative` of max(|x|, 1).
    fn step(&self, x: &[f64], j: usize, relative: f64) -> f64 {
        let v = &self.variables[j];
        let h = relative * x[j].abs().max(1.0);
        // never more than an eighth of the bounds' span
        let h = h.min((v.to - v.from) / 8.0);
        // a step that is exact in floating point
        (x[j] + h) - x[j]
    }

    /// x with variable `j` moved by `by`.
    fn moved(x: &[f64], j: usize, by: f64) -> Vec<f64> {
        let mut y = x.to_vec();
        y[j] += by;
        y
    }

    /// Central differences of every cost and constraint in variable `j` (one-sided, second
    /// order, at a bound), and the models built.
    fn difference(&self, x: &[f64], j: usize, base: &[f64]) -> Result<(Vec<f64>, usize)> {
        let v = &self.variables[j];
        let h = self.step(x, j, STEP);
        let (column, solves) = if x[j] + h <= v.to && x[j] - h >= v.from {
            let (plus, a) = self.scalars(&Self::moved(x, j, h))?;
            let (minus, b) = self.scalars(&Self::moved(x, j, -h))?;
            (
                plus.iter()
                    .zip(&minus)
                    .map(|(p, m)| (p - m) / (2.0 * h))
                    .collect(),
                a + b,
            )
        } else {
            // inwards from the bound: (−3 f₀ + 4 f₁ − f₂)/(2h), h signed
            let s = if x[j] + 2.0 * h <= v.to { h } else { -h };
            let (one, a) = self.scalars(&Self::moved(x, j, s))?;
            let (two, b) = self.scalars(&Self::moved(x, j, 2.0 * s))?;
            (
                base.iter()
                    .zip(one.iter().zip(&two))
                    .map(|(f0, (f1, f2))| (-3.0 * f0 + 4.0 * f1 - f2) / (2.0 * s))
                    .collect(),
                a + b,
            )
        };
        Ok((column, solves))
    }

    /// Fourth-order differences of every quantity at every sample in variable `j`: central, or
    /// one-sided inwards at a bound.
    fn fourth_order(&self, x: &[f64], j: usize, relative: f64) -> Result<Vec<Vec<f64>>> {
        let v = &self.variables[j];
        let h = self.step(x, j, relative) / 2.0;
        let f = |k: f64| {
            self.point(&Self::moved(x, j, k * h), &[])
                .map(|p| p.samples)
        };
        let combine = |fs: &[Vec<Vec<f64>>], c: &[f64], over: f64| -> Vec<Vec<f64>> {
            (0..fs[0].len())
                .map(|q| {
                    (0..fs[0][q].len())
                        .map(|s| c.iter().zip(fs).map(|(c, f)| c * f[q][s]).sum::<f64>() / over)
                        .collect()
                })
                .collect()
        };
        if x[j] + 2.0 * h <= v.to && x[j] - 2.0 * h >= v.from {
            let fs = [f(-2.0)?, f(-1.0)?, f(1.0)?, f(2.0)?];
            return Ok(combine(&fs, &[1.0, -8.0, 8.0, -1.0], 12.0 * h));
        }
        // (−25 f₀ + 48 f₁ − 36 f₂ + 16 f₃ − 3 f₄)/(12 s)
        let s = if x[j] + 4.0 * h <= v.to { 1.0 } else { -1.0 };
        let fs: Vec<Vec<Vec<f64>>> = (0..5).map(|k| f(s * k as f64)).collect::<Result<_>>()?;
        Ok(combine(
            &fs,
            &[-25.0, 48.0, -36.0, 16.0, -3.0],
            12.0 * s * h,
        ))
    }

    /// The design at `x`, and at each corner of it.
    fn designs(&self, x: &[f64]) -> Result<Vec<Spec>> {
        let spec = self.design_at(x)?;
        if self.corners.is_empty() {
            return Ok(vec![spec]);
        }
        Study::Corners(self.corners.clone()).designs(&spec)
    }

    /// d(device values)/dx_j at `design`: per unit of the variable as the task gives it.
    fn sensitivities(&self, design: &Design, j: usize) -> Result<Vec<f64>> {
        let mut s = design.sensitivities(&self.variables[j].parameter)?;
        if self.angle[j] {
            // the design takes a named angle in radians; the variable is in degrees
            for v in &mut s {
                *v *= std::f64::consts::PI / 180.0;
            }
        }
        Ok(s)
    }

    /// Every quantity's value at every sample (the corners, then the wavelengths, fastest), and
    /// for each variable of `wanted` their exact derivatives where every quantity has them.
    fn point(&self, x: &[f64], wanted: &[usize]) -> Result<Point> {
        for (v, &value) in self.variables.iter().zip(x) {
            if !(v.from..=v.to).contains(&value) {
                return Err(Error::invalid(
                    "optimize",
                    format!(
                        "{} = {value} is outside its bounds, {} to {}",
                        v.parameter, v.from, v.to
                    ),
                ));
            }
        }
        let nq = self.objectives.len() + self.constraints.len();
        let mut samples = vec![Vec::new(); nq];
        let mut derivatives: Vec<Option<Vec<Vec<f64>>>> =
            wanted.iter().map(|_| Some(vec![Vec::new(); nq])).collect();
        let (fidelity, grids) = match &self.target {
            Target::Device { spec, grids } => (spec.fidelity, grids),
            Target::Chip(t) => return chip::point(self, t, x, wanted),
        };
        let designs = self.designs(x)?;
        let mut solves = 0;
        for d in &designs {
            let (_, design) = d.build()?;
            let device = d.device_with(grids)?;
            let built = device.component(design.values(), fidelity)?;
            solves += 1;
            for q in self.quantities() {
                for f in &q.figures {
                    if built.figure(f).is_none() {
                        return Err(Error::invalid(
                            "optimize",
                            format!(
                                "the {} model of a {} gives no figure {f}, which {} names",
                                fidelity_name(fidelity),
                                self.kind,
                                q.symbol
                            ),
                        ));
                    }
                }
            }
            let values = design.values().as_slice();
            let sens: Vec<Vec<f64>> = wanted
                .iter()
                .map(|&j| self.sensitivities(&design, j))
                .collect::<Result<_>>()?;
            for &w in &self.wavelengths {
                let at = Wavelength::um(w)?;
                let s = built.s_matrix(at)?;
                let slots = self.slot_values(&built, &s, values, w);
                for (k, q) in self.quantities().enumerate() {
                    samples[k].push(q.expression.eval(&slots));
                }
                if wanted.is_empty() {
                    continue;
                }
                let ds = built.component.derivatives(at, &built.values)?;
                for (i, sensitivities) in sens.iter().enumerate() {
                    let Some(dq) = derivatives[i].as_mut() else {
                        continue;
                    };
                    let d_s = chain(&built, ds.as_deref(), sensitivities, s.size());
                    let duals = self.duals(&slots, &s, d_s.as_ref(), sensitivities);
                    let mut known = true;
                    for (k, q) in self.quantities().enumerate() {
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

    /// The outputs' values at a sample, slot by slot.
    fn slot_values(&self, built: &Built, s: &SMatrix, values: &[f64], lambda: f64) -> Vec<f64> {
        let mut slots = vec![0.0; self.names.scope.len()];
        for (slot, source) in &self.names.sources {
            slots[*slot] = match source {
                Source::Power(q, p) => s.power(*q, *p),
                Source::Figure(f) => built.figure(f).map_or(f64::NAN, |g| g.value),
                Source::Parameter(k) => values[*k],
                Source::Lambda => lambda,
            };
        }
        slots
    }

    /// The outputs with their derivatives in one variable: a power's from dS/dx (NaN when it
    /// isn't known), a parameter's from the expressions, a figure's unknown (NaN).
    fn duals(
        &self,
        slots: &[f64],
        s: &SMatrix,
        d_s: Option<&SMatrix>,
        sensitivities: &[f64],
    ) -> Vec<Dual> {
        let mut duals: Vec<Dual> = slots.iter().map(|&v| Dual::constant(v)).collect();
        for (slot, source) in &self.names.sources {
            duals[*slot].derivative = match source {
                Source::Power(q, p) => {
                    d_s.map_or(f64::NAN, |d| 2.0 * (s[(*q, *p)].conj() * d[(*q, *p)]).re)
                }
                Source::Figure(_) => f64::NAN,
                Source::Parameter(k) => sensitivities[*k],
                Source::Lambda => 0.0,
            };
        }
        duals
    }
}

/// dS/dx for a variable whose device values move by `sensitivities`: the component's
/// derivatives through [`Built::derivatives`]; `None` when one moved isn't known (a parameter the
/// component is built from) or the component gives none.
fn chain(
    built: &Built,
    ds: Option<&[SMatrix]>,
    sensitivities: &[f64],
    n: usize,
) -> Option<SMatrix> {
    let mut total = SMatrix::zeros(n);
    for (k, &dp) in sensitivities.iter().enumerate() {
        if dp == 0.0 {
            continue;
        }
        let ds = ds?;
        let along = built.derivatives[k].as_ref()?;
        for (m, &dt) in along.iter().enumerate() {
            if dt == 0.0 {
                continue;
            }
            for q in 0..n {
                for p in 0..n {
                    total[(q, p)] += ds[m][(q, p)] * (dt * dp);
                }
            }
        }
    }
    Some(total)
}

/// The figures an expression names.
fn figures_of(names: &Names, e: &Expr) -> Vec<String> {
    names
        .sources
        .iter()
        .filter(|(slot, _)| e.slots().contains(slot))
        .filter_map(|(_, s)| match s {
            Source::Figure(f) => Some(f.clone()),
            _ => None,
        })
        .collect()
}

/// A variable's column of differences, and the models built for it.
type Differenced = Result<(Vec<f64>, usize)>;

/// A point's samples: each quantity's value per sample, and per variable asked, their exact
/// derivatives (`None` where one isn't known).
struct Point {
    samples: Vec<Vec<f64>>,
    derivatives: Vec<Option<Vec<Vec<f64>>>>,
    solves: usize,
}

/// A quantity reduced over its samples: the cost (or the constraint's g), the value reported,
/// and d(result)/d(sample) for each sample.
struct Reduced {
    result: f64,
    value: f64,
    weights: Vec<f64>,
}

/// `q` over its samples `f`.
fn reduce(q: &Quantity, f: &[f64]) -> Reduced {
    let n = f.len();
    let mut weights = vec![0.0; n];
    if f.iter().any(|v| !v.is_finite()) {
        return Reduced {
            result: f64::NAN,
            value: f64::NAN,
            weights,
        };
    }
    // the cost of a value and its derivative, for an objective; the identity for a constraint
    let cost = |v: f64| -> (f64, f64) {
        match q.goal {
            Some((Goal::Minimize, _)) | None => (v, 1.0),
            Some((Goal::Maximize, _)) => (-v, -1.0),
            Some((Goal::Target, t)) => ((v - t) * (v - t), 2.0 * (v - t)),
        }
    };
    // the first largest and smallest, in the samples' order
    let largest = |g: &dyn Fn(f64) -> f64| {
        (0..n).fold(0, |best, k| if g(f[k]) > g(f[best]) { k } else { best })
    };
    match q.reduce {
        Reduce::Spread => {
            let (hi, lo) = (largest(&|v| v), largest(&|v| -v));
            let spread = f[hi] - f[lo];
            let (c, dc) = match q.limit {
                Some((Limit::AtLeast, b)) => (b - spread, -1.0),
                Some((Limit::AtMost, b)) => (spread - b, 1.0),
                None => cost(spread),
            };
            if hi != lo {
                weights[hi] += dc;
                weights[lo] -= dc;
            }
            Reduced {
                result: c,
                value: spread,
                weights,
            }
        }
        Reduce::Average => {
            let mean = f.iter().sum::<f64>() / n as f64;
            match q.limit {
                Some((limit, b)) => {
                    let sign = if limit == Limit::AtMost { 1.0 } else { -1.0 };
                    weights.fill(sign / n as f64);
                    Reduced {
                        result: sign * (mean - b),
                        value: mean,
                        weights,
                    }
                }
                None => {
                    let mut total = 0.0;
                    for (w, &v) in weights.iter_mut().zip(f) {
                        let (c, dc) = cost(v);
                        total += c;
                        *w = dc / n as f64;
                    }
                    Reduced {
                        result: total / n as f64,
                        value: mean,
                        weights,
                    }
                }
            }
        }
        Reduce::Worst => match q.limit {
            Some((Limit::AtMost, b)) => {
                let k = largest(&|v| v);
                weights[k] = 1.0;
                Reduced {
                    result: f[k] - b,
                    value: f[k],
                    weights,
                }
            }
            Some((Limit::AtLeast, b)) => {
                let k = largest(&|v| -v);
                weights[k] = -1.0;
                Reduced {
                    result: b - f[k],
                    value: f[k],
                    weights,
                }
            }
            None => {
                let k = largest(&|v| cost(v).0);
                let (c, dc) = cost(f[k]);
                weights[k] = dc;
                Reduced {
                    result: c,
                    value: f[k],
                    weights,
                }
            }
        },
    }
}

// ---- the record ----

/// One of the points of a Pareto front ([`Event::Front`]): an evaluation's index and its costs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FrontPoint {
    /// The evaluation, from 1.
    pub index: usize,
    /// Its costs.
    pub costs: Vec<f64>,
    /// Its objectives' values.
    pub values: Vec<f64>,
}

/// The hypervolume a front of two costs (minimized) dominates up to `reference`: the area of
/// the union of the rectangles from each point to the reference, by a sweep in the first cost.
/// Points not better than the reference in both costs add nothing.
pub fn hypervolume(front: &[[f64; 2]], reference: [f64; 2]) -> f64 {
    let mut points: Vec<[f64; 2]> = front
        .iter()
        .copied()
        .filter(|p| p[0] < reference[0] && p[1] < reference[1])
        .collect();
    points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    let mut area = 0.0;
    let mut ceiling = reference[1];
    for p in points {
        if p[1] < ceiling {
            area += (reference[0] - p[0]) * (ceiling - p[1]);
            ceiling = p[1];
        }
    }
    area
}

/// Whether costs `a` dominate `b`: no worse in any, better in one.
fn dominates(a: &[f64], b: &[f64]) -> bool {
    a.iter().zip(b).all(|(x, y)| x <= y) && a.iter().zip(b).any(|(x, y)| x < y)
}

/// Records an optimization's run: its problem, each evaluation in the optimizer's order, its
/// fronts and its optimum ([`Event::Optimization`], [`Event::Evaluation`], [`Event::Front`],
/// [`Event::Optimum`]).
#[derive(Debug)]
pub struct Recorder {
    count: usize,
    solves: usize,
    /// The best so far (one objective): its index, costs and violation, and its point.
    best: Option<(usize, f64, f64, Vec<f64>)>,
    /// The front so far (two objectives): the non-dominated feasible evaluations.
    front: Vec<FrontPoint>,
    /// The feasible costs' smallest and largest so far.
    range: Option<([f64; 2], [f64; 2])>,
    /// Whether the front changed since the last [`Event::Front`].
    changed: bool,
    /// The front's reference point, fixed at the first front.
    reference: Option<[f64; 2]>,
    /// The ranges of the variables, for the gradient's norm in them.
    spans: Vec<f64>,
    objectives: usize,
}

impl Recorder {
    /// Starts the record of `problem`'s run of `job`: [`Event::Started`], the design's scene and
    /// ports, then [`Event::Optimization`] with the derivatives' `checks`.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the record can't be written, and what drawing the design refuses.
    pub fn begin(
        problem: &Problem,
        job: &Job,
        run: &mut Run,
        checks: Vec<GradientCheck>,
    ) -> Result<Recorder> {
        let (fidelity, model, parameters) = match &problem.target {
            Target::Device { spec, .. } => {
                run.record(&Event::Started {
                    job: job.name().to_owned(),
                    kind: "device".into(),
                })?;
                run.record(&super::device_scene(job)?)?;
                run.record(&Event::Ports {
                    ports: scene_ports(job)?,
                })?;
                let (device, _) = spec.build()?;
                let model = device
                    .fidelities()
                    .iter()
                    .find(|m| m.fidelity == spec.fidelity)
                    .cloned()
                    .ok_or_else(|| task_error(format!("a {} has no such model", problem.kind)))?;
                let parameters = device.parameters().iter().map(|p| p.name.clone()).collect();
                (
                    fidelity_name(spec.fidelity).to_owned(),
                    model.name,
                    parameters,
                )
            }
            Target::Chip(t) => chip::begin(problem, t, job, run)?,
        };
        let derivatives = problem.derivatives();
        run.record(&Event::Optimization {
            device: problem.kind.clone(),
            fidelity,
            model,
            method: problem.method.name.to_owned(),
            optimizer: problem.method.title.to_owned(),
            paper: problem.method.paper.to_owned(),
            seed: problem.seed,
            evaluations: problem.evaluations,
            population: problem.population,
            timeout_minutes: problem.timeout_minutes,
            variables: problem
                .variables
                .iter()
                .zip(&problem.units)
                .zip(&problem.start)
                .zip(&derivatives)
                .map(|(((v, unit), &start), d)| OptimizeVariable {
                    parameter: v.parameter.clone(),
                    unit: unit.clone(),
                    from: v.from,
                    to: v.to,
                    start,
                    derivatives: (*d).to_owned(),
                })
                .collect(),
            objectives: problem.objectives.iter().map(Quantity::event).collect(),
            constraints: problem.constraints.iter().map(Quantity::event).collect(),
            wavelengths_um: problem.wavelengths.clone(),
            corners: problem.corners.clone(),
            parameters,
            checks,
        })?;
        Ok(Recorder {
            count: 0,
            solves: 0,
            best: None,
            front: Vec::new(),
            range: None,
            changed: false,
            reference: None,
            spans: problem.variables.iter().map(|v| v.to - v.from).collect(),
            objectives: problem.objectives.len(),
        })
    }

    /// How many evaluations are recorded.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Records the evaluation `e` at `x`: [`Event::Evaluation`], with the best so far (one
    /// objective: feasible before infeasible, then the smaller cost, then the earlier).
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the record can't be written.
    pub fn record(&mut self, run: &mut Run, x: &[f64], e: &Evaluation) -> Result<()> {
        self.count += 1;
        self.solves += e.solves;
        let index = self.count;
        // a point the model gave nothing for is recorded without numbers (JSON has no NaN)
        let valid = e.error.is_none()
            && e.costs.iter().chain(&e.constraints).all(|c| c.is_finite())
            && e.violation.is_finite();
        if valid && self.objectives == 1 {
            let better = match &self.best {
                None => true,
                Some((_, cost, violation, _)) => {
                    if e.violation == 0.0 && *violation == 0.0 {
                        e.costs[0] < *cost
                    } else {
                        e.violation < *violation
                    }
                }
            };
            if better {
                self.best = Some((index, e.costs[0], e.violation, x.to_vec()));
            }
        }
        if valid && self.objectives == 2 && e.violation == 0.0 {
            let c = [e.costs[0], e.costs[1]];
            self.range = Some(match self.range {
                None => (c, c),
                Some((low, high)) => (
                    [low[0].min(c[0]), low[1].min(c[1])],
                    [high[0].max(c[0]), high[1].max(c[1])],
                ),
            });
            // kept unless a point of the front is as good in both costs
            let covered = self
                .front
                .iter()
                .any(|p| p.costs.iter().zip(&e.costs).all(|(a, b)| a <= b));
            if !covered {
                self.front.retain(|p| !dominates(&e.costs, &p.costs));
                self.front.push(FrontPoint {
                    index,
                    costs: e.costs.clone(),
                    values: e.values.clone(),
                });
                self.changed = true;
            }
        }
        let gradient_norm = e.gradients.first().map(|g| {
            g.iter()
                .zip(&self.spans)
                .map(|(d, s)| (d * s) * (d * s))
                .sum::<f64>()
                .sqrt()
        });
        run.record(&Event::Evaluation {
            index,
            x: x.to_vec(),
            costs: if valid { e.costs.clone() } else { Vec::new() },
            values: if valid { e.values.clone() } else { Vec::new() },
            constraints: if valid {
                e.constraints.clone()
            } else {
                Vec::new()
            },
            violation: valid.then_some(e.violation),
            gradient_norm,
            best: self.best.as_ref().map(|b| b.0),
            solves: self.solves,
            error: e.error.clone(),
        })
    }

    /// Records [`Event::Front`] when the front changed since the last: the non-dominated
    /// feasible evaluations so far (the first of equal ones), in their order, and their
    /// hypervolume, its reference point the largest costs of the feasible evaluations before the
    /// first front, a tenth of their range beyond. Nothing for a run of one objective.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the record can't be written.
    pub fn front(&mut self, run: &mut Run) -> Result<()> {
        if self.objectives != 2 || !self.changed {
            return Ok(());
        }
        self.changed = false;
        if self.reference.is_none()
            && let Some((low, high)) = self.range
        {
            let mut r = [0.0; 2];
            for (k, slot) in r.iter_mut().enumerate() {
                let margin = 0.1 * (high[k] - low[k]);
                *slot = high[k]
                    + if margin > 0.0 {
                        margin
                    } else {
                        high[k].abs().max(1.0) * 0.1
                    };
            }
            self.reference = Some(r);
        }
        let points = self.front.clone();
        let reference = self.reference.unwrap_or([f64::NAN; 2]);
        let hypervolume = if self.reference.is_some() {
            let costs: Vec<[f64; 2]> = points.iter().map(|p| [p.costs[0], p.costs[1]]).collect();
            hypervolume(&costs, reference)
        } else {
            0.0
        };
        run.record(&Event::Front {
            evaluations: self.count,
            points,
            hypervolume,
            reference: self.reference.map(|r| r.to_vec()).unwrap_or_default(),
        })
    }

    /// Records [`Event::Optimum`]: the best evaluation and its design's parameters (one
    /// objective), or the front (two); why it stopped, `stop` (the optimizer's reason, or the
    /// run's: `"timeout"`, `"requested"`).
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the record can't be written.
    pub fn finish(&mut self, problem: &Problem, run: &mut Run, stop: &str) -> Result<()> {
        let (index, parameters) = match &self.best {
            Some((index, _, _, x)) => (Some(*index), problem.parameters_at(x)?),
            None => (None, Vec::new()),
        };
        let front = if self.objectives == 2 {
            self.front.iter().map(|p| p.index).collect()
        } else {
            Vec::new()
        };
        run.record(&Event::Optimum {
            index,
            x: self.best.as_ref().map(|b| b.3.clone()).unwrap_or_default(),
            parameters,
            feasible: self.best.as_ref().is_some_and(|b| b.2 == 0.0),
            stop: stop.to_owned(),
            evaluations: self.count,
            solves: self.solves,
            front,
        })
    }
}

/// [`super::check`] for an `"optimize"` job.
pub(super) fn check(job: &Job) -> Result<()> {
    Problem::of(job).map(|_| ())
}

/// The design of evaluation `x` of the optimization `job`, as Apply takes it: the job's
/// `[device]` with the variables at `x`.
///
/// # Errors
///
/// What [`Problem::of`] refuses, and a point that isn't one of the variables'.
pub fn design_of(job: &Job, x: &[f64]) -> Result<Spec> {
    Problem::of(job)?.design_at(x)
}

mod chip;
pub use chip::optimize_from_chip;

#[cfg(test)]
mod tests;
