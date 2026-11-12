//! A density design region optimized: a job's `[task]` of kind `"optimize"` with a
//! `[task.design]` and no `[device]` or `[circuit]` (docs/methods/topology.md, "The job").
//!
//! ```yaml
//! name: bend
//! timeout_minutes: 30
//!
//! task:
//!   kind: optimize
//!   x_um: [-1.9, 3.9]       # the window, its PMLs inside it
//!   y_um: [-0.9, 4.9]
//!   step_nm: 20             # the cell, and the design's pixel
//!   core_index: 3.4         # the guides and the design's solid
//!   cladding_index: 1.44    # around them, and the design's void
//!   guide:
//!     - { x_um: [-2, 0], y_um: [1.25, 1.75] }
//!     - { x_um: [1.25, 1.75], y_um: [3, 5] }
//!   port:
//!     - { name: in, side: west, at_um: -1.0, window_um: [0.5, 2.5] }
//!     - { name: out, side: north, at_um: 4.0, window_um: [0.5, 2.5] }
//!   source: in              # the port the light enters by (west or east), its mode source_mode
//!   objective:
//!     - { port: out, goal: maximize, from_um: 1.5, to_um: 1.6, points: 10 }
//!   design:
//!     x_um: [0, 3]
//!     y_um: [0, 3]
//!     filter_nm: 90         # the conic filter's radius
//!     beta: [8, 16, 32]     # the projection's β at each stage, at η = ½ (eta)
//!     evaluations: [70, 70, 150]
//!     linewidth_nm: 90      # Zhou et al.'s length constraints, from rules_from (the last stage)
//!     spacing_nm: 90
//!     island_um2: 0.08      # Hammond et al.'s minimum areas
//!     hole_um2: 0.2
//!     # robust_eta: 0.3     # the worst over the eroded, nominal and dilated designs
//!     # start: 0.5, asymmetry: 0.25: the start's density, and a term breaking a mirror symmetry
//! ```
//!
//! **The plane.** 2D FDFD with E along z ([`crate::fdfd::Solver2d`]) on cells of `step_nm`, the
//! cladding everywhere but the guides' rectangles of the core; the design region's pixels are the
//! grid's cells, each a density between the cladding (0) and the core (1)
//! ([`crate::topology::Density`]: a conic filter, then a tanh projection). The indices are fixed,
//! as the published problems state them. Every number a run reports is on this grid.
//!
//! **The objectives.** Each is a port's mode's power over the source's mode's, at each of its
//! wavelengths: the transmission into a mode, or the reflection when the port is the source's own
//! (its column measured going back). A west or east port is a column, its mode measured by
//! [`crate::fdfd::Solver2d::mode_power_gradient`]; a south or north one is two rows, its mode
//! that of the guide its row crosses, turned ([`crate::fdfd::Solver2d::row_mode_functional`]).
//! Each value at each wavelength (and each realization of a robust design) is one term,
//! 1 − P for a maximized one and P for a minimized one, and the run minimizes the worst term by
//! the epigraph ([`crate::topology::Epigraph`]): every gradient by the adjoint, its values and
//! gradients rounded to 12 significant digits so that the run repeats to the bit.
//!
//! **The stages.** The projection's β is raised from stage to stage; each stage is a new MMA
//! (genoxide's, run by the `photonoxide` program) from the last stage's design, for its
//! evaluations. MMA's iterates sit a little outside the epigraph's constraints, so each stage
//! keeps its own best design: the least worst term with every rule met, its start's when no
//! evaluation improves on it.
//!
//! **The rules**, from stage `rules_from` (the last by default): the linewidth and the spacing by
//! Zhou et al.'s geometric constraints with Arrieta et al.'s hyperparameters
//! ([`crate::topology::LengthConstraints::with_arrieta_decay`]), each held as g/ε − 1 ≤ 0 with ε
//! from [`crate::topology::geometric_threshold`] for a strip across the region; and the minimum
//! area and enclosed area by Hammond et al.'s Eqs. 21–22
//! ([`crate::topology::AreaConstraints`]), held to their G = 10⁻⁵ t and counted in pixels.
//!
//! **The record** ([`TopologyRecorder`]): [`Event::Started`], the start's permittivity,
//! [`Event::Topology`] (the problem); per stage an [`Event::Stage`]; per evaluation an
//! [`Event::TopologyEvaluation`]; the design ([`Event::Density`]) and the field
//! ([`Event::Field`]) every `record_every` evaluations and at each stage's end; and at the end the
//! binary design, its field and its spectrum, minimum length and areas
//! ([`Event::TopologyOptimum`]).

use std::sync::Mutex;

use num_complex::Complex64 as c64;
use rayon::prelude::*;
use schemars::{JsonSchema, SchemaGenerator};
use serde::{Deserialize, Serialize};

use super::super::check::{check_cells, check_step, check_window};
use super::super::{Event, task_error};
use super::method;
use crate::fdfd::{Boundaries, Direction, Field2d, Grid, Polarization, PortMode, Solver2d};
use crate::raster::Raster;
use crate::run::{Job, Run};
use crate::topology::ruler::{Pattern, minimum_length};
use crate::topology::{
    AreaConstraints, Density, Edge, Epigraph, LengthConstraints, Placement, Projection,
    geometric_threshold,
};
use crate::units::Wavelength;
use crate::{Error, Result};

/// Hammond et al.'s G = a t for the area rules (their a = 10⁻⁵).
const AREA_SLACK: f64 = 1e-5;
/// Arrieta et al.'s normalized decay c/R² (their Table 1).
const ARRIETA_DECAY: f64 = 64.0;

/// A guide: a rectangle of the core.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TopologyGuide {
    /// From and to along x, µm.
    x_um: [f64; 2],
    /// Along y, µm.
    y_um: [f64; 2],
}

/// A port: a line across a guide, where a mode is launched or measured.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TopologyPort {
    /// Its name, which `source` and the objectives use.
    name: String,
    /// `"west"` or `"east"` (a column, light along x), `"south"` or `"north"` (two rows, light
    /// along y): the side of the design the light leaves or enters by.
    side: String,
    /// Where its line is, µm: x for a west or east port, y for a south or north one.
    at_um: f64,
    /// Its window along the line, µm (y for a west or east port, x for a south or north one): the
    /// guide and its evanescent tails, clear of any other guide.
    window_um: [f64; 2],
}

/// An objective: a port's mode's power over the source's, at each of its wavelengths.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TopologyObjective {
    /// The port it is measured at: the source's own for a reflection.
    port: String,
    /// The port's mode, from 1: the fundamental (1) by default.
    mode: Option<usize>,
    /// `"maximize"` (the default) or `"minimize"`.
    goal: Option<String>,
    /// Its first wavelength, µm.
    from_um: f64,
    /// Its last, µm: `from_um` by default.
    to_um: Option<f64>,
    /// How many wavelengths from `from_um` to `to_um`, evenly spaced: 1 by default, at least 2
    /// when they differ.
    points: Option<usize>,
}

/// The design region and its schedule.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TopologyDesign {
    /// From and to along x, µm: on the grid's lines, inside the PMLs.
    x_um: [f64; 2],
    /// Along y, µm.
    y_um: [f64; 2],
    /// The conic filter's radius, nm.
    filter_nm: f64,
    /// The projection's threshold η: ½ by default.
    eta: Option<f64>,
    /// The projection's β at each stage.
    beta: Vec<f64>,
    /// Each stage's MMA evaluations, one per stage.
    evaluations: Vec<u64>,
    /// The start's density: ½ by default.
    start: Option<f64>,
    /// An amplitude a: the start gains a sin(π(v − ½)) sin(π u), u and v the pixel's place across
    /// the region along x and y, odd across its middle along y, to break a mirror symmetry the
    /// ports have (0 by default).
    asymmetry: Option<f64>,
    /// The robust realizations' threshold η in (0, ½): the worst over the design projected at
    /// 1 − η (eroded), ½ and η (dilated); none by default.
    robust_eta: Option<f64>,
    /// The minimum linewidth, nm, with `spacing_nm`.
    linewidth_nm: Option<f64>,
    /// The minimum spacing, nm, with `linewidth_nm`.
    spacing_nm: Option<f64>,
    /// The minimum area of an island, µm², with `hole_um2`.
    island_um2: Option<f64>,
    /// The minimum enclosed area of a hole, µm², with `island_um2`.
    hole_um2: Option<f64>,
    /// The first stage (from 0) the rules hold at: the last by default.
    rules_from: Option<usize>,
}

/// A density design region's `"optimize"` task.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::job) struct TopologyTask {
    /// `"optimize"`.
    #[allow(dead_code)] // looked up by the registry (job::kind); kept for serde and the schema
    kind: String,
    /// genoxide's method: `"mma"` (the default and the one), which takes the epigraph's many
    /// constraints.
    method: Option<String>,
    /// The window along x, µm, its PMLs inside it.
    x_um: [f64; 2],
    /// Along y, µm.
    y_um: [f64; 2],
    /// The cell, nm: the design's pixel too.
    step_nm: f64,
    /// PML cells on each side, inside the window: 20 by default.
    pml_cells: Option<usize>,
    /// The core's refractive index: the guides' and the design's solid.
    core_index: f64,
    /// The cladding's: around them, and the design's void.
    cladding_index: f64,
    /// The guides, rectangles of the core.
    #[serde(default)]
    guide: Vec<TopologyGuide>,
    /// The ports.
    port: Vec<TopologyPort>,
    /// The port the light enters by: a west or east one.
    source: String,
    /// Its mode, from 1: the fundamental (1) by default.
    source_mode: Option<usize>,
    /// What is optimized, one or more.
    objective: Vec<TopologyObjective>,
    /// The design region and its schedule.
    design: TopologyDesign,
    /// The design and the field recorded every this many evaluations (10 by default), and at
    /// each stage's end.
    record_every: Option<usize>,
}

/// The schema of a design region's task, as `docs/job.schema.json` holds it.
pub(in crate::job) fn schema(g: &mut SchemaGenerator) -> serde_json::Value {
    super::super::schema::task::<TopologyTask>(g)
}

/// Whether `job`'s task is a design region's: an `"optimize"` task with a `[task.design]`.
pub(in crate::job) fn designs(job: &Job) -> bool {
    job.device().is_none() && job.circuit().is_none() && job.task().contains_key("design")
}

/// Checks a design region's job without solving: what [`Topology::of`] reads and refuses, but
/// for the modes.
pub(in crate::job) fn check(job: &Job) -> Result<()> {
    Plan::of(job).map(|_| ())
}

/// The start's permittivity, as [`Event::Permittivity`]: what the studio draws of the job before
/// it runs.
pub(in crate::job) fn preview(job: &Job) -> Result<Event> {
    let plan = Plan::of(job)?;
    let density = plan.density(plan.design.beta[0])?;
    let mut cells = plan.background.clone();
    plan.placement
        .embed(&density.permittivity(&plan.start())?, &mut cells)?;
    Ok(Event::Permittivity {
        view: "the window, the design at its start".into(),
        axes: ["x".into(), "y".into()],
        wavelength_um: plan.wavelengths[0],
        raster: plan.raster(&cells),
    })
}

/// A stage's rules: the length constraints with their ε, and the area constraints.
type Rules = (Option<(LengthConstraints, f64)>, Option<AreaConstraints>);

/// The goal of an objective.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Goal {
    Maximize,
    Minimize,
}

/// The side of a port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    West,
    East,
    South,
    North,
}

impl Side {
    fn of(name: &str) -> Result<Side> {
        match name {
            "west" => Ok(Side::West),
            "east" => Ok(Side::East),
            "south" => Ok(Side::South),
            "north" => Ok(Side::North),
            other => Err(task_error(format!(
                "a port's side is \"west\", \"east\", \"south\" or \"north\", got \"{other}\""
            ))),
        }
    }

    /// Whether the port is a column (light along x).
    fn column(self) -> bool {
        matches!(self, Side::West | Side::East)
    }

    /// Which way light leaving the design by this side goes.
    fn out(self) -> Direction {
        match self {
            Side::West | Side::South => Direction::Backward,
            Side::East | Side::North => Direction::Forward,
        }
    }
}

/// A port on the grid: its line and its window, in cells.
#[derive(Clone, Debug)]
struct PortPlace {
    name: String,
    side: Side,
    /// The column (west, east) or the first of its two rows (south, north).
    line: usize,
    /// The window along the line: rows (west, east) or columns (south, north).
    window: std::ops::Range<usize>,
}

/// An objective, read.
#[derive(Clone, Debug)]
struct ObjectivePlan {
    port: usize,
    mode: usize,
    goal: Goal,
    wavelengths: Vec<f64>,
}

/// The job read and checked, without the modes: the grid, the cells, the ports, the design.
struct Plan {
    timeout_minutes: f64,
    grid: Grid,
    pml: usize,
    background: Vec<c64>,
    eps: [f64; 2],
    ports: Vec<PortPlace>,
    source: usize,
    source_mode: usize,
    objectives: Vec<ObjectivePlan>,
    /// Every wavelength any objective takes, in the order they first appear, µm.
    wavelengths: Vec<f64>,
    placement: Placement,
    shape: [usize; 2],
    pixel_um: f64,
    region: [[f64; 2]; 2],
    design: TopologyDesign,
    eta: f64,
    rules_from: usize,
    record_every: usize,
}

/// `v`'s cell index along an axis from `origin` on cells of `h`, refused unless it is on a line
/// of the grid.
fn on_line(what: &str, v: f64, origin: f64, h: f64) -> Result<usize> {
    let k = ((v - origin) / h).round();
    if k < 0.0 || ((v - origin) - k * h).abs() > 1e-6 * h {
        return Err(task_error(format!(
            "{what} ({v} µm) must be on a line of the grid: the window's edge plus a whole number \
             of cells"
        )));
    }
    Ok(k as usize)
}

/// The cell whose centre is nearest `v` along an axis.
fn nearest(v: f64, origin: f64, h: f64) -> isize {
    ((v - origin) / h - 0.5).round() as isize
}

impl Plan {
    fn of(job: &Job) -> Result<Plan> {
        let task: TopologyTask = job
            .task()
            .clone()
            .try_into()
            .map_err(|e: toml::de::Error| task_error(e.message().to_owned()))?;
        let Some(timeout_minutes) = job.timeout().map(|d| d.as_secs_f64() / 60.0) else {
            return Err(task_error(
                "an optimization needs timeout_minutes, its hard limit (AGENTS.md)",
            ));
        };
        if let Some(m) = &task.method
            && m != "mma"
        {
            return Err(task_error(format!(
                "a design region's method is \"mma\", which takes the epigraph's constraints, got \
                 \"{m}\""
            )));
        }
        check_window("x_um", task.x_um)?;
        check_window("y_um", task.y_um)?;
        check_step(task.step_nm)?;
        check_cells("x_um", task.x_um, task.step_nm)?;
        check_cells("y_um", task.y_um, task.step_nm)?;
        let h = task.step_nm * 1e-3;
        let pml = task.pml_cells.unwrap_or(20);
        let nx = ((task.x_um[1] - task.x_um[0]) / h).round() as usize;
        let ny = ((task.y_um[1] - task.y_um[0]) / h).round() as usize;
        if nx < 2 * pml + 8 || ny < 2 * pml + 8 {
            return Err(task_error(format!(
                "the window is {nx} × {ny} cells, too few for PMLs of {pml} cells on each side and \
                 a plane between them"
            )));
        }
        let grid = Grid {
            nx,
            ny,
            dx: h,
            dy: h,
            x0: task.x_um[0],
            y0: task.y_um[0],
        };
        for (name, n) in [
            ("core_index", task.core_index),
            ("cladding_index", task.cladding_index),
        ] {
            if !(n.is_finite() && n >= 1.0) {
                return Err(task_error(format!("{name} must be at least 1, got {n}")));
            }
        }
        let eps = [
            task.cladding_index * task.cladding_index,
            task.core_index * task.core_index,
        ];
        for g in &task.guide {
            check_window("a guide's x_um", g.x_um)?;
            check_window("a guide's y_um", g.y_um)?;
        }
        let background: Vec<c64> = (0..nx * ny)
            .map(|p| {
                let (x, y) = (grid.x(p % nx), grid.y(p / nx));
                let core = task
                    .guide
                    .iter()
                    .any(|g| x > g.x_um[0] && x < g.x_um[1] && y > g.y_um[0] && y < g.y_um[1]);
                c64::new(eps[usize::from(core)], 0.0)
            })
            .collect();

        // the ports
        if task.port.is_empty() {
            return Err(task_error("a design region's task needs a [[task.port]]"));
        }
        let mut ports = Vec::new();
        for p in &task.port {
            if ports.iter().any(|q: &PortPlace| q.name == p.name) {
                return Err(task_error(format!("two ports are named \"{}\"", p.name)));
            }
            let side = Side::of(&p.side)?;
            check_window("a port's window_um", p.window_um)?;
            let (along, across, n_along, n_across) = if side.column() {
                (grid.x0, grid.y0, nx, ny)
            } else {
                (grid.y0, grid.x0, ny, nx)
            };
            let line = nearest(p.at_um, along, h);
            // a row port reads rows line and line + 1
            let last = if side.column() { line } else { line + 1 };
            if line < (pml + 2) as isize || last + 2 >= (n_along - pml) as isize {
                return Err(task_error(format!(
                    "port \"{}\" at {} µm must be two cells clear of the PMLs, inside the window",
                    p.name, p.at_um
                )));
            }
            // the cells whose centres are inside the window, the same on either side of a mirror
            let a = (((p.window_um[0] - across) / h - 0.5 - 1e-9).ceil() as isize).max(pml as isize)
                as usize;
            let b = (((p.window_um[1] - across) / h - 0.5 + 1e-9).floor() as isize + 1)
                .min((n_across - pml) as isize);
            if b <= a as isize + 2 {
                return Err(task_error(format!(
                    "port \"{}\"'s window has fewer than three cells between the PMLs",
                    p.name
                )));
            }
            ports.push(PortPlace {
                name: p.name.clone(),
                side,
                line: line as usize,
                window: a..b as usize,
            });
        }
        let port_named = |name: &str| {
            ports.iter().position(|p| p.name == name).ok_or_else(|| {
                task_error(format!(
                    "no port is named \"{name}\"; the ports are {}",
                    ports
                        .iter()
                        .map(|p| format!("\"{}\"", p.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
        };
        let source = port_named(&task.source)?;
        if !ports[source].side.column() {
            return Err(task_error(
                "the source is launched on a column: a west or east port",
            ));
        }
        let mode_of = |m: Option<usize>| match m {
            Some(0) => Err(task_error("a mode is counted from 1, the fundamental")),
            Some(m) => Ok(m - 1),
            None => Ok(0),
        };
        let source_mode = mode_of(task.source_mode)?;

        // the objectives and their wavelengths
        if task.objective.is_empty() {
            return Err(task_error(
                "a design region's task needs an [[task.objective]]",
            ));
        }
        let mut objectives = Vec::new();
        let mut wavelengths: Vec<f64> = Vec::new();
        for o in &task.objective {
            let port = port_named(&o.port)?;
            let goal = match o.goal.as_deref() {
                None | Some("maximize") => Goal::Maximize,
                Some("minimize") => Goal::Minimize,
                Some(other) => {
                    return Err(task_error(format!(
                        "an objective's goal is \"maximize\" or \"minimize\", got \"{other}\""
                    )));
                }
            };
            let to = o.to_um.unwrap_or(o.from_um);
            let points = o.points.unwrap_or(1);
            if !(o.from_um.is_finite() && o.from_um > 0.0 && to.is_finite() && to >= o.from_um) {
                return Err(task_error(format!(
                    "an objective's wavelengths go from a positive from_um to a to_um at least as \
                     large, got {} to {to}",
                    o.from_um
                )));
            }
            if points == 0 || (to > o.from_um && points < 2) || points > 64 {
                return Err(task_error(format!(
                    "an objective takes 1 to 64 wavelengths, at least 2 between two ends, got \
                     {points}"
                )));
            }
            let at: Vec<f64> = (0..points)
                .map(|k| {
                    if points == 1 {
                        o.from_um
                    } else {
                        o.from_um + (to - o.from_um) * k as f64 / (points - 1) as f64
                    }
                })
                .collect();
            for &w in &at {
                Wavelength::um(w)?;
                if !wavelengths.iter().any(|v| v.to_bits() == w.to_bits()) {
                    wavelengths.push(w);
                }
            }
            objectives.push(ObjectivePlan {
                port,
                mode: mode_of(o.mode)?,
                goal,
                wavelengths: at,
            });
        }

        // the design region
        let d = task.design.clone();
        check_window("design.x_um", d.x_um)?;
        check_window("design.y_um", d.y_um)?;
        let i0 = on_line("design.x_um's start", d.x_um[0], grid.x0, h)?;
        let i1 = on_line("design.x_um's end", d.x_um[1], grid.x0, h)?;
        let j0 = on_line("design.y_um's start", d.y_um[0], grid.y0, h)?;
        let j1 = on_line("design.y_um's end", d.y_um[1], grid.y0, h)?;
        if i0 < pml || j0 < pml || i1 > nx - pml || j1 > ny - pml {
            return Err(task_error(
                "the design region must lie between the PMLs, inside the window",
            ));
        }
        let shape = [i1 - i0, j1 - j0];
        for p in &ports {
            let (lo, hi) = if p.side.column() { (i0, i1) } else { (j0, j1) };
            let last = if p.side.column() { p.line } else { p.line + 1 };
            if p.line < hi && last >= lo {
                let (w_lo, w_hi) = if p.side.column() { (j0, j1) } else { (i0, i1) };
                if p.window.start < w_hi && p.window.end > w_lo {
                    return Err(task_error(format!(
                        "port \"{}\" crosses the design region: put it on a guide outside it",
                        p.name
                    )));
                }
            }
        }
        if !(d.filter_nm.is_finite() && d.filter_nm > 0.0) {
            return Err(task_error(format!(
                "design.filter_nm must be positive, got {}",
                d.filter_nm
            )));
        }
        let eta = d.eta.unwrap_or(0.5);
        if d.beta.is_empty() || d.beta.len() != d.evaluations.len() {
            return Err(task_error(format!(
                "design.beta and design.evaluations give one value per stage, at least one: got \
                 {} and {}",
                d.beta.len(),
                d.evaluations.len()
            )));
        }
        for &b in &d.beta {
            Projection::new(b, eta).map_err(|e| task_error(format!("design: {e}")))?;
        }
        if d.evaluations.contains(&0) {
            return Err(task_error("each stage needs at least one evaluation"));
        }
        let start = d.start.unwrap_or(0.5);
        let asymmetry = d.asymmetry.unwrap_or(0.0);
        if !(start.is_finite()
            && asymmetry.is_finite()
            && start - asymmetry.abs() >= 0.0
            && start + asymmetry.abs() <= 1.0)
        {
            return Err(task_error(format!(
                "the start's density, design.start ± design.asymmetry, must stay in [0, 1], got \
                 {start} ± {asymmetry}"
            )));
        }
        if let Some(r) = d.robust_eta
            && !(r > 0.0 && r < 0.5)
        {
            return Err(task_error(format!(
                "design.robust_eta must be in (0, ½), the dilated design's threshold, got {r}"
            )));
        }
        match (d.linewidth_nm, d.spacing_nm) {
            (None, None) => {}
            (Some(l), Some(s)) if l.is_finite() && l > 0.0 && s.is_finite() && s > 0.0 => {}
            _ => {
                return Err(task_error(
                    "design.linewidth_nm and design.spacing_nm come together, both positive",
                ));
            }
        }
        match (d.island_um2, d.hole_um2) {
            (None, None) => {}
            (Some(a), Some(b)) if a.is_finite() && a > 0.0 && b.is_finite() && b > 0.0 => {}
            _ => {
                return Err(task_error(
                    "design.island_um2 and design.hole_um2 come together, both positive",
                ));
            }
        }
        let rules_from = d.rules_from.unwrap_or(d.beta.len() - 1);
        if rules_from >= d.beta.len() {
            return Err(task_error(format!(
                "design.rules_from is a stage from 0 to {}, got {rules_from}",
                d.beta.len() - 1
            )));
        }
        let record_every = task.record_every.unwrap_or(10);
        if record_every == 0 {
            return Err(task_error("record_every must be at least 1"));
        }
        let plan = Plan {
            timeout_minutes,
            grid,
            pml,
            background,
            eps,
            ports,
            source,
            source_mode,
            objectives,
            wavelengths,
            placement: Placement {
                origin: [i0, j0, 0],
                shape: [shape[0], shape[1], 1],
                grid: [nx, ny, 1],
            },
            shape,
            pixel_um: h,
            region: [d.x_um, d.y_um],
            design: d,
            eta,
            rules_from,
            record_every,
        };
        // the filter and the rules built once, so that what they refuse is refused here
        let density = plan.density(plan.design.beta[0])?;
        plan.rules(&density)?;
        Ok(plan)
    }

    /// The design's density at `beta`.
    fn density(&self, beta: f64) -> Result<Density> {
        Ok(
            Density::new([self.shape[0], self.shape[1], 1], self.pixel_um, self.eps)?
                .with_filter(self.design.filter_nm * 1e-3, Edge::Renormalized)?
                .with_projection(Projection::new(beta, self.eta)?),
        )
    }

    /// The rules on `density`: the length constraints and their ε, and the area constraints.
    fn rules(&self, density: &Density) -> Result<Rules> {
        let lengths = match (self.design.linewidth_nm, self.design.spacing_nm) {
            (Some(l), Some(s)) => {
                let c = LengthConstraints::new(density, l * 1e-3, s * 1e-3)?.with_arrieta_decay();
                // a strip across the region along its longer side: R over its shorter side
                let side = self.shape[0].min(self.shape[1]) as f64 * self.pixel_um;
                let eps = geometric_threshold(ARRIETA_DECAY, self.design.filter_nm * 1e-3 / side)?;
                Some((c, eps))
            }
            _ => None,
        };
        let areas = match (self.design.island_um2, self.design.hole_um2) {
            (Some(a), Some(b)) => Some(AreaConstraints::new(self.shape, self.pixel_um, a, b)?),
            _ => None,
        };
        Ok((lengths, areas))
    }

    /// The rules' labels, in their order, from the rules' stage.
    fn rule_labels(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.design.linewidth_nm.is_some() {
            out.push("linewidth".into());
            out.push("spacing".into());
        }
        if self.design.island_um2.is_some() {
            out.push("island area".into());
            out.push("hole area".into());
        }
        out
    }

    /// The start's densities: `start`, and the asymmetry's term.
    fn start(&self) -> Vec<f64> {
        let start = self.design.start.unwrap_or(0.5);
        let a = self.design.asymmetry.unwrap_or(0.0);
        let [nx, ny] = self.shape;
        (0..nx * ny)
            .map(|p| {
                let u = (p % nx) as f64 / nx as f64;
                let v = (p / nx) as f64 / ny as f64;
                let pi = std::f64::consts::PI;
                start + a * (pi * (v - 0.5)).sin() * (pi * u).sin()
            })
            .collect()
    }

    /// The window's cells as a picture, inside the PMLs.
    fn raster(&self, cells: &[c64]) -> Raster {
        self.picture(|i, j| cells[j * self.grid.nx + i].re)
    }

    /// A picture of `value(i, j)` over the plane between the PMLs.
    fn picture(&self, value: impl Fn(usize, usize) -> f64) -> Raster {
        let g = self.grid;
        let p = self.pml;
        let (nx, ny) = (g.nx - 2 * p, g.ny - 2 * p);
        Raster {
            nx,
            ny,
            x0: g.x0 + p as f64 * g.dx,
            x1: g.x0 + (p + nx) as f64 * g.dx,
            y0: g.y0 + p as f64 * g.dy,
            y1: g.y0 + (p + ny) as f64 * g.dy,
            values: (0..ny)
                .flat_map(|j| (0..nx).map(move |i| (i, j)))
                .map(|(i, j)| value(i + p, j + p) as f32)
                .collect(),
        }
    }
}

/// How a port's mode is measured.
enum Measure {
    /// A column's mode, travelling one way.
    Column(PortMode, Direction),
    /// A mode on two rows, as its functional's coefficients.
    Row(Vec<c64>),
}

/// One measurement at a wavelength: an objective's term.
struct Term {
    objective: usize,
    measure: Measure,
    /// The measured mode's power over the source's.
    ratio: f64,
    goal: Goal,
}

/// A wavelength's solver, source and measurements.
struct Band {
    wavelength: Wavelength,
    solver: Solver2d,
    source: PortMode,
    direction: Direction,
    terms: Vec<Term>,
}

/// A design region's optimization, read from its job, with each wavelength's ports' modes solved:
/// what the `photonoxide` program runs genoxide's MMA on, a stage at a time ([`Topology::stage`]).
pub struct Topology {
    plan: Plan,
    bands: Vec<Band>,
    /// Each term's label, in the order of the bands' terms: e.g. `"T(out, mode 1) at 1.500 µm"`.
    labels: Vec<String>,
}

/// One stage of a [`Topology`]: its β, its realizations and its rules, and its best design.
pub struct TopologyStage<'t> {
    topology: &'t Topology,
    index: usize,
    beta: f64,
    /// The nominal density.
    density: Density,
    /// The densities each term is taken over: the nominal alone, or eroded, nominal, dilated.
    realizations: Vec<Density>,
    lengths: Option<(LengthConstraints, f64)>,
    areas: Option<AreaConstraints>,
    epigraph: Epigraph,
    /// The stage's best evaluation and its densities.
    best: Mutex<(Rank, Vec<f64>)>,
}

/// How an evaluation ranks for a stage's best: one that meets every rule beats one that doesn't;
/// of those that do, the least worst term; of those that don't, the least violation Σ max(0, g).
#[derive(Clone, Copy, Debug)]
struct Rank {
    feasible: bool,
    worst: f64,
    violation: f64,
}

impl Rank {
    /// Beaten by any evaluation.
    const NONE: Rank = Rank {
        feasible: false,
        worst: f64::INFINITY,
        violation: f64::INFINITY,
    };

    fn of(worst: f64, rules: &[f64]) -> Rank {
        Rank {
            feasible: rules.iter().all(|v| *v <= 0.0),
            worst,
            violation: rules.iter().map(|v| v.max(0.0)).sum(),
        }
    }

    fn beats(&self, other: &Rank) -> bool {
        match (self.feasible, other.feasible) {
            (true, false) => true,
            (false, true) => false,
            (true, true) => self.worst < other.worst,
            (false, false) => self.violation < other.violation,
        }
    }
}

/// An evaluation of a stage at y = (ρ, t).
#[derive(Clone, Debug)]
pub struct TopologyPoint {
    /// The epigraph's objective, t.
    pub t: f64,
    /// Each term: 1 − P for a maximized objective, P for a minimized one, the realizations'
    /// in turn.
    pub values: Vec<f64>,
    /// The worst term.
    pub worst: f64,
    /// Each rule's g (≤ 0 when met), none before the rules' stage.
    pub rules: Vec<f64>,
    /// Whether every rule is met.
    pub feasible: bool,
    /// Whether it is the stage's best so far.
    pub improved: bool,
    /// The field of the nominal design at the first wavelength.
    field: Option<Field2d>,
}

fn round12(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let scale = 10f64.powi(11 - x.abs().log10().floor() as i32);
    if !scale.is_finite() {
        // below about 1e-297 the scale overflows: such a value is kept as it is
        return x;
    }
    (x * scale).round() / scale
}

impl Topology {
    /// Whether `job` optimizes a design region: an `"optimize"` task with a `[task.design]` and
    /// no `[device]` or `[circuit]`, which the `photonoxide` program runs.
    pub fn designs(job: &Job) -> bool {
        designs(job) && job.task().get("kind").and_then(toml::Value::as_str) == Some("optimize")
    }

    /// `job`'s design region read and checked, and each wavelength's source and ports' modes
    /// solved on the plane without the design (the guides alone, the region the cladding's).
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for a task that isn't one (each field's rule is its doc's), and what the
    /// mode solves refuse: a port whose window guides fewer modes than it asks for.
    pub fn of(job: &Job) -> Result<Topology> {
        let plan = Plan::of(job)?;
        let built: Vec<Result<(Band, Vec<String>)>> = plan
            .wavelengths
            .par_iter()
            .map(|&um| band(&plan, um))
            .collect();
        let mut bands = Vec::new();
        let mut labels = Vec::new();
        for b in built {
            let (b, l) = b?;
            bands.push(b);
            labels.extend(l);
        }
        let labels = match plan.design.robust_eta {
            None => labels,
            Some(_) => ["eroded", "nominal", "dilated"]
                .iter()
                .flat_map(|r| labels.iter().map(move |l| format!("{l}, {r}")))
                .collect(),
        };
        Ok(Topology {
            plan,
            bands,
            labels,
        })
    }

    /// The design region's pixels.
    pub fn len(&self) -> usize {
        self.plan.shape[0] * self.plan.shape[1]
    }

    /// Whether it has none (never: a region is at least a pixel).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The design region's pixels along x and y.
    pub fn shape(&self) -> [usize; 2] {
        self.plan.shape
    }

    /// The stages.
    pub fn stages(&self) -> usize {
        self.plan.design.beta.len()
    }

    /// The start's densities.
    pub fn start(&self) -> Vec<f64> {
        self.plan.start()
    }

    /// The hard limit, minutes.
    pub fn timeout_minutes(&self) -> f64 {
        self.plan.timeout_minutes
    }

    /// Each term's label, the realizations' in turn.
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// Stage `index` (from 0): its density at its β, its realizations and rules.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a stage past the last.
    pub fn stage(&self, index: usize) -> Result<TopologyStage<'_>> {
        let Some(&beta) = self.plan.design.beta.get(index) else {
            return Err(Error::invalid(
                "topology stage",
                format!("there are {} stages, got stage {index}", self.stages()),
            ));
        };
        let density = self.plan.density(beta)?;
        let realizations = match self.plan.design.robust_eta {
            None => vec![density.clone()],
            Some(eta) => {
                let r = density.robust(eta)?;
                vec![r.eroded, r.nominal, r.dilated]
            }
        };
        let (lengths, areas) = if index >= self.plan.rules_from {
            self.plan.rules(&density)?
        } else {
            (None, None)
        };
        let epigraph = Epigraph::new(self.len(), self.labels.len())?;
        Ok(TopologyStage {
            topology: self,
            index,
            beta,
            density,
            realizations,
            lengths,
            areas,
            epigraph,
            best: Mutex::new((Rank::NONE, Vec::new())),
        })
    }

    /// Each term's P (the measured mode's power over the source's) of a binary design, solved
    /// with its pixels the core's (`true`) or the cladding's, and the field at the first
    /// wavelength.
    fn binary(&self, design: &[bool]) -> Result<(Vec<f64>, Field2d)> {
        let eps: Vec<f64> = design
            .iter()
            .map(|&s| self.plan.eps[usize::from(s)])
            .collect();
        let mut cells = self.plan.background.clone();
        self.plan.placement.embed(&eps, &mut cells)?;
        let solved: Vec<Result<(Vec<f64>, Field2d)>> = self
            .bands
            .par_iter()
            .map(|b| {
                let s = b.solver.reuse_cells(b.wavelength, &cells)?;
                let field = s.solve_system(&s.mode_source(&b.source, b.direction))?;
                let mut p = Vec::new();
                for t in &b.terms {
                    p.push(round12(power(&s, &field, &t.measure)?.0 * t.ratio));
                }
                Ok((p, field))
            })
            .collect();
        let mut values = Vec::new();
        let mut first = None;
        for r in solved {
            let (p, field) = r?;
            values.extend(p);
            first.get_or_insert(field);
        }
        Ok((values, first.expect("at least one wavelength")))
    }
}

/// A measured mode's power and its gradient in the cells' permittivities.
fn power(s: &Solver2d, field: &Field2d, measure: &Measure) -> Result<(f64, Vec<f64>)> {
    match measure {
        Measure::Column(mode, direction) => s.mode_power_gradient(field, mode, *direction),
        Measure::Row(c) => s.functional_power_gradient(field, c),
    }
}

/// The band at `um`: its solver on the plane without the design, the source's mode, and each
/// objective's measurement there, with their labels.
fn band(plan: &Plan, um: f64) -> Result<(Band, Vec<String>)> {
    let lam = Wavelength::um(um)?;
    let g = plan.grid;
    let s = Solver2d::from_cells(
        g,
        Polarization::Ez,
        lam,
        &plan.background,
        Boundaries::pml(plan.pml),
    )?;
    let modes = |p: &PortPlace, m: usize| -> Result<(PortMode, Option<Solver2d>)> {
        if p.side.column() {
            let mut found = s.port_modes_within(p.line, p.window.clone(), m + 1)?;
            if found.len() <= m {
                return Err(task_error(format!(
                    "port \"{}\" guides {} mode(s) at {um} µm, not mode {}",
                    p.name,
                    found.len(),
                    m + 1
                )));
            }
            Ok((found.swap_remove(m), None))
        } else {
            // the guide the rows cross, turned: a straight guide whose cells along y are the
            // plane's along x on the port's row
            let aux_grid = Grid {
                nx: 2 * plan.pml + 10,
                ny: g.nx,
                dx: g.dy,
                dy: g.dx,
                x0: 0.0,
                y0: g.x0,
            };
            let aux_cells: Vec<c64> = (0..aux_grid.nx * aux_grid.ny)
                .map(|q| plan.background[p.line * g.nx + q / aux_grid.nx])
                .collect();
            let aux = Solver2d::from_cells(
                aux_grid,
                Polarization::Ez,
                lam,
                &aux_cells,
                Boundaries::pml(plan.pml),
            )?;
            let mut found = aux.port_modes_within(plan.pml + 4, p.window.clone(), m + 1)?;
            if found.len() <= m {
                return Err(task_error(format!(
                    "port \"{}\" guides {} mode(s) at {um} µm, not mode {}",
                    p.name,
                    found.len(),
                    m + 1
                )));
            }
            Ok((found.swap_remove(m), Some(aux)))
        }
    };
    let src_port = &plan.ports[plan.source];
    let (source, _) = modes(src_port, plan.source_mode)?;
    let direction = match src_port.side {
        Side::West => Direction::Forward,
        _ => Direction::Backward,
    };
    let reference = source.power(g.dy);
    let mut terms = Vec::new();
    let mut labels = Vec::new();
    for (k, o) in plan.objectives.iter().enumerate() {
        if !o.wavelengths.iter().any(|w| w.to_bits() == um.to_bits()) {
            continue;
        }
        let p = &plan.ports[o.port];
        let (mode, _) = modes(p, o.mode)?;
        // a row mode's profile runs along x
        let across = if p.side.column() { g.dy } else { g.dx };
        let ratio = mode.power(across) / reference;
        // the source's own port measures what comes back
        let measure = if p.side.column() {
            let direction = if o.port == plan.source {
                match direction {
                    Direction::Forward => Direction::Backward,
                    _ => Direction::Forward,
                }
            } else {
                p.side.out()
            };
            Measure::Column(mode, direction)
        } else {
            Measure::Row(s.row_mode_functional(&mode, p.line, 0, p.side.out())?)
        };
        let name = if o.port == plan.source { "R" } else { "T" };
        labels.push(format!(
            "{}{name}({}, mode {}) at {um:.3} µm",
            if o.goal == Goal::Maximize {
                "1 − "
            } else {
                ""
            },
            p.name,
            o.mode + 1
        ));
        terms.push(Term {
            objective: k,
            measure,
            ratio,
            goal: o.goal,
        });
    }
    Ok((
        Band {
            wavelength: lam,
            solver: s,
            source,
            direction,
            terms,
        },
        labels,
    ))
}

/// Each term's value and its gradient in the densities, and the field of the first band of the
/// realization `keep` when asked.
type Terms = (Vec<f64>, Vec<Vec<f64>>, Option<Field2d>);

impl TopologyStage<'_> {
    /// The stage, from 0.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Its β.
    pub fn beta(&self) -> f64 {
        self.beta
    }

    /// Its MMA evaluations.
    pub fn evaluations(&self) -> u64 {
        self.topology.plan.design.evaluations[self.index]
    }

    /// MMA's variables: the densities and t.
    pub fn variables(&self) -> usize {
        self.topology.len() + 1
    }

    /// Its constraints: the epigraph's, one per term, then the rules'.
    pub fn constraints(&self) -> usize {
        self.epigraph.constraints() + self.rule_count()
    }

    fn rule_count(&self) -> usize {
        2 * usize::from(self.lengths.is_some()) + 2 * usize::from(self.areas.is_some())
    }

    /// The rules' labels, in their order.
    pub fn rule_labels(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.lengths.is_some() {
            out.push("linewidth".into());
            out.push("spacing".into());
        }
        if self.areas.is_some() {
            out.push("island area".into());
            out.push("hole area".into());
        }
        out
    }

    /// The physical density (nominal) of `rho`.
    ///
    /// # Errors
    ///
    /// What [`Density::physical`] refuses.
    pub fn physical(&self, rho: &[f64]) -> Result<Vec<f64>> {
        self.density.physical(rho)
    }

    /// Each term and its gradient at `rho`, rounded to 12 digits; the nominal design's field at
    /// the first wavelength.
    fn terms(&self, rho: &[f64]) -> Result<Terms> {
        let plan = &self.topology.plan;
        let cells: Vec<Vec<c64>> = self
            .realizations
            .iter()
            .map(|d| {
                let mut cells = plan.background.clone();
                plan.placement.embed(&d.permittivity(rho)?, &mut cells)?;
                Ok(cells)
            })
            .collect::<Result<_>>()?;
        let nominal = self.realizations.len() / 2;
        let jobs: Vec<(usize, usize)> = (0..self.realizations.len())
            .flat_map(|r| (0..self.topology.bands.len()).map(move |b| (r, b)))
            .collect();
        type Solved = (Vec<f64>, Vec<Vec<f64>>, Option<Field2d>);
        let solved: Vec<Result<Solved>> = jobs
            .par_iter()
            .map(|&(r, b)| {
                let band = &self.topology.bands[b];
                let density = &self.realizations[r];
                let s = band.solver.reuse_cells(band.wavelength, &cells[r])?;
                let field = s.solve_system(&s.mode_source(&band.source, band.direction))?;
                let (mut values, mut gradients) = (Vec::new(), Vec::new());
                for t in &band.terms {
                    let (p, dp) = power(&s, &field, &t.measure)?;
                    let d = density.gradient(rho, &plan.placement.restrict(&dp)?)?;
                    let (value, sign) = match t.goal {
                        Goal::Maximize => (1.0 - p * t.ratio, -t.ratio),
                        Goal::Minimize => (p * t.ratio, t.ratio),
                    };
                    values.push(round12(value));
                    gradients.push(d.iter().map(|&v| round12(v * sign)).collect());
                }
                let keep = r == nominal && b == 0;
                Ok((values, gradients, keep.then_some(field)))
            })
            .collect();
        let (mut values, mut gradients, mut field) = (Vec::new(), Vec::new(), None);
        for s in solved {
            let (v, g, f) = s?;
            values.extend(v);
            gradients.extend(g);
            if f.is_some() {
                field = f;
            }
        }
        Ok((values, gradients, field))
    }

    /// The stage's start: y = (ρ, t) with t the worst term at `rho`, which becomes the stage's
    /// best; and its evaluation, as [`TopologyStage::evaluate`] gives it.
    ///
    /// # Errors
    ///
    /// What the solves refuse.
    pub fn start(&self, rho: &[f64]) -> Result<(Vec<f64>, TopologyPoint)> {
        let (values, _, field) = self.terms(rho)?;
        let worst = Epigraph::start(&values);
        let mut y = rho.to_vec();
        y.push(worst);
        let rules = self.rule_values(&y)?.0;
        *self.best.lock().unwrap_or_else(|e| e.into_inner()) =
            (Rank::of(worst, &rules), rho.to_vec());
        Ok((
            y,
            TopologyPoint {
                t: worst,
                worst,
                feasible: rules.iter().all(|v| *v <= 0.0),
                rules,
                values,
                improved: true,
                field,
            },
        ))
    }

    /// The rules' values at y = (ρ, t) and their Jacobian's rows (each `variables()` wide).
    fn rule_values(&self, y: &[f64]) -> Result<(Vec<f64>, Vec<f64>)> {
        let n = self.topology.len();
        let w = n + 1;
        let rho = &y[..n];
        let mut g = Vec::new();
        let mut jacobian = Vec::new();
        if let Some((lengths, eps)) = &self.lengths {
            let l = lengths.evaluate(rho)?;
            for s in 0..2 {
                g.push(round12(l.values[s] / eps - 1.0));
                let mut row = vec![0.0; w];
                for (r, d) in row[..n].iter_mut().zip(&l.gradients[s]) {
                    *r = round12(d / eps);
                }
                jacobian.extend(row);
            }
        }
        if let Some(areas) = &self.areas {
            let a = areas.evaluate(&self.density.physical(rho)?)?;
            // g ≤ a t as g − a t ≤ 0, both in pixels: a few pixels' violation reads as a few,
            // which MMA's scaling needs
            let pixels = n as f64;
            for s in 0..2 {
                let d = self.density.pullback(rho, &a.gradients[s])?;
                g.push(round12((a.values[s] - AREA_SLACK * y[n]) * pixels));
                let mut row = vec![0.0; w];
                for (r, d) in row[..n].iter_mut().zip(&d) {
                    *r = round12(d * pixels);
                }
                row[n] = -AREA_SLACK * pixels;
                jacobian.extend(row);
            }
        }
        Ok((g, jacobian))
    }

    /// MMA's evaluation at y = (ρ, t): t, with its gradient in `gradient`, and every constraint's
    /// value in `g` and Jacobian row in `jacobian` (`variables()` wide): the epigraph's, then the
    /// rules'. The stage's best is updated.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for slices of the wrong lengths, and what the solves refuse.
    pub fn evaluate(
        &self,
        y: &[f64],
        gradient: &mut [f64],
        g: &mut [f64],
        jacobian: &mut [f64],
    ) -> Result<TopologyPoint> {
        let n = self.topology.len();
        let w = n + 1;
        let k = self.epigraph.constraints();
        if y.len() != w || g.len() != self.constraints() || jacobian.len() != self.constraints() * w
        {
            return Err(Error::invalid(
                "topology evaluation",
                format!(
                    "y has {w} values, g {} and the Jacobian {} × {w}; got {}, {} and {}",
                    self.constraints(),
                    self.constraints(),
                    y.len(),
                    g.len(),
                    jacobian.len()
                ),
            ));
        }
        let (values, gradients, field) = self.terms(&y[..n])?;
        let flat: Vec<f64> = gradients.concat();
        let t = self.epigraph.evaluate(
            y,
            &values,
            &flat,
            gradient,
            &mut g[..k],
            &mut jacobian[..k * w],
        )?;
        let (rules, rows) = self.rule_values(y)?;
        g[k..].copy_from_slice(&rules);
        jacobian[k * w..].copy_from_slice(&rows);
        let worst = Epigraph::start(&values);
        let feasible = rules.iter().all(|v| *v <= 0.0);
        let rank = Rank::of(worst, &rules);
        let mut best = self.best.lock().unwrap_or_else(|e| e.into_inner());
        let improved = rank.beats(&best.0);
        if improved {
            *best = (rank, y[..n].to_vec());
        }
        drop(best);
        Ok(TopologyPoint {
            t,
            values,
            worst,
            rules,
            feasible,
            improved,
            field,
        })
    }

    /// The stage's best design, its start's included: the least worst term among those that meet
    /// every rule, or the least violation (Σ max(0, g)) when none does.
    pub fn best(&self) -> Vec<f64> {
        self.best
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .1
            .clone()
    }
}

/// A term of a design region's optimization, as its record names it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologyTerm {
    /// What it is, e.g. `"1 − T(out, mode 1) at 1.500 µm"`.
    pub label: String,
    /// Its objective, from 0, in the job's order.
    pub objective: usize,
    /// The port it is measured at.
    pub port: String,
    /// The port's mode, from 1.
    pub mode: usize,
    /// Whether it is a reflection: measured at the source's own port.
    pub reflection: bool,
    /// Whether its objective is maximized (the term is 1 − P) or minimized (P).
    pub maximize: bool,
    /// Its wavelength, µm.
    pub wavelength_um: f64,
    /// The robust design's realization it is taken over: `"eroded"`, `"nominal"` or
    /// `"dilated"`; none for a design that isn't robust.
    pub realization: Option<String>,
}

/// The record of a design region's run: what it has recorded, and the best evaluation of the
/// stage under way.
pub struct TopologyRecorder {
    every: usize,
    count: usize,
    stage_count: usize,
    best: Option<usize>,
}

impl TopologyRecorder {
    /// Starts the record: [`Event::Started`], the start's permittivity, and
    /// [`Event::Topology`].
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the record can't be written.
    pub fn begin(topology: &Topology, job: &Job, run: &mut Run) -> Result<TopologyRecorder> {
        let plan = &topology.plan;
        run.record(&Event::Started {
            job: job.name().to_owned(),
            kind: "optimize".into(),
        })?;
        run.record(&preview(job)?)?;
        let mma = method("mma")?;
        let terms = topology
            .bands
            .iter()
            .flat_map(|b| b.terms.iter().map(|t| (t.objective, b.wavelength.to_um())))
            .collect::<Vec<_>>();
        let realizations: Vec<Option<String>> = match plan.design.robust_eta {
            None => vec![None],
            Some(_) => ["eroded", "nominal", "dilated"]
                .iter()
                .map(|r| Some((*r).to_owned()))
                .collect(),
        };
        run.record(&Event::Topology {
            method: mma.name.into(),
            optimizer: mma.title.into(),
            paper: mma.paper.into(),
            timeout_minutes: plan.timeout_minutes,
            pixels: plan.shape,
            pixel_nm: plan.pixel_um * 1e3,
            region_um: plan.region,
            filter_nm: plan.design.filter_nm,
            eta: plan.eta,
            beta: plan.design.beta.clone(),
            evaluations: plan.design.evaluations.clone(),
            robust_eta: plan.design.robust_eta,
            terms: realizations
                .iter()
                .flat_map(|r| terms.iter().map(move |t| (r, t)))
                .zip(&topology.labels)
                .map(|((realization, &(objective, wavelength_um)), label)| {
                    let o = &plan.objectives[objective];
                    TopologyTerm {
                        label: label.clone(),
                        objective,
                        port: plan.ports[o.port].name.clone(),
                        mode: o.mode + 1,
                        reflection: o.port == plan.source,
                        maximize: o.goal == Goal::Maximize,
                        wavelength_um,
                        realization: realization.clone(),
                    }
                })
                .collect(),
            rules: plan.rule_labels(),
            rules_from: plan.rules_from,
            record_every: plan.record_every,
        })?;
        Ok(TopologyRecorder {
            every: plan.record_every,
            count: 0,
            stage_count: 0,
            best: None,
        })
    }

    /// A stage begins: [`Event::Stage`].
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the record can't be written.
    pub fn stage(&mut self, stage: &TopologyStage, run: &mut Run) -> Result<()> {
        self.best = None;
        self.stage_count = 0;
        run.record(&Event::Stage {
            stage: stage.index,
            stages: stage.topology.stages(),
            parameter: "beta".into(),
            value: stage.beta,
            keep: "point".into(),
            evaluations: self.count,
        })
    }

    /// An evaluation: [`Event::TopologyEvaluation`], and the design and its field every
    /// `record_every` evaluations of the stage (its first among them).
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the record can't be written.
    pub fn record(
        &mut self,
        stage: &TopologyStage,
        run: &mut Run,
        rho: &[f64],
        point: &TopologyPoint,
    ) -> Result<()> {
        self.count += 1;
        if point.improved {
            self.best = Some(self.count);
        }
        run.record(&Event::TopologyEvaluation {
            index: self.count,
            stage: stage.index,
            beta: stage.beta,
            t: Some(point.t),
            worst: Some(point.worst),
            values: point.values.clone(),
            rules: point.rules.clone(),
            feasible: point.feasible,
            best: self.best,
            error: None,
        })?;
        if self.stage_count.is_multiple_of(self.every) {
            self.design(stage, run, rho, point.field.as_ref(), "the design")?;
        }
        self.stage_count += 1;
        Ok(())
    }

    /// An evaluation that failed: recorded with its error.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the record can't be written.
    pub fn failed(&mut self, stage: &TopologyStage, run: &mut Run, error: &str) -> Result<()> {
        self.count += 1;
        self.stage_count += 1;
        run.record(&Event::TopologyEvaluation {
            index: self.count,
            stage: stage.index,
            beta: stage.beta,
            t: None,
            worst: None,
            values: Vec::new(),
            rules: Vec::new(),
            feasible: false,
            best: self.best,
            error: Some(error.to_owned()),
        })
    }

    /// A stage's end: its best design and that design's field.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the record can't be written, and what the solves refuse.
    pub fn stage_end(&mut self, stage: &TopologyStage, run: &mut Run) -> Result<()> {
        let rho = stage.best();
        let (_, _, field) = stage.terms(&rho)?;
        self.design(stage, run, &rho, field.as_ref(), "the stage's best design")
    }

    fn design(
        &self,
        stage: &TopologyStage,
        run: &mut Run,
        rho: &[f64],
        field: Option<&Field2d>,
        what: &str,
    ) -> Result<()> {
        let plan = &stage.topology.plan;
        let physical = stage.physical(rho)?;
        run.record(&Event::Density {
            index: self.count,
            stage: stage.index,
            label: format!("{what} at β = {}", stage.beta),
            raster: region_raster(plan, &physical),
        })?;
        if let Some(f) = field {
            run.record(&field_event(
                plan,
                f,
                format!("|E_z|² of {what}, evaluation {}", self.count),
            ))?;
        }
        Ok(())
    }

    /// The run's end: the last stage's best design, binary (its physical density above ½),
    /// solved, its field, and [`Event::TopologyOptimum`] with its terms' P, its minimum length
    /// and its small islands and holes.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the record can't be written, and what the solves refuse.
    pub fn finish(&mut self, stage: &TopologyStage, run: &mut Run, stop: &str) -> Result<()> {
        let topology = stage.topology;
        let plan = &topology.plan;
        let rho = stage.best();
        let physical = stage.physical(&rho)?;
        let design: Vec<bool> = physical.iter().map(|&v| v > 0.5).collect();
        let (powers, field) = topology.binary(&design)?;
        let as_density: Vec<f64> = design.iter().map(|&s| f64::from(u8::from(s))).collect();
        run.record(&Event::Density {
            index: self.count,
            stage: stage.index,
            label: "the design, binary".into(),
            raster: region_raster(plan, &as_density),
        })?;
        run.record(&field_event(
            plan,
            &field,
            "|E_z|² of the design, binary".into(),
        ))?;
        let pixel_nm = plan.pixel_um * 1e3;
        let length_nm = minimum_length(&Pattern::new(plan.shape, design)?, [pixel_nm, pixel_nm])?;
        let (islands, holes) = match (plan.design.island_um2, plan.design.hole_um2) {
            (Some(a), Some(b)) => {
                let v = AreaConstraints::new(plan.shape, plan.pixel_um, a, b)?
                    .evaluate(&as_density)?
                    .violations;
                (Some(v[0]), Some(v[1]))
            }
            _ => (None, None),
        };
        let wavelengths: Vec<f64> = topology
            .bands
            .iter()
            .flat_map(|b| b.terms.iter().map(|_| b.wavelength.to_um()))
            .collect();
        let objectives: Vec<usize> = topology
            .bands
            .iter()
            .flat_map(|b| b.terms.iter().map(|t| t.objective))
            .collect();
        run.record(&Event::TopologyOptimum {
            index: self.best,
            stage: stage.index,
            powers,
            wavelengths_um: wavelengths,
            objectives,
            length_nm,
            islands,
            holes,
            stop: stop.to_owned(),
            evaluations: self.count,
        })
    }
}

/// A picture of the design region's values, in µm.
fn region_raster(plan: &Plan, values: &[f64]) -> Raster {
    Raster {
        nx: plan.shape[0],
        ny: plan.shape[1],
        x0: plan.region[0][0],
        x1: plan.region[0][1],
        y0: plan.region[1][0],
        y1: plan.region[1][1],
        values: values.iter().map(|&v| v as f32).collect(),
    }
}

/// |E_z|² over the plane between the PMLs, its largest value 1.
fn field_event(plan: &Plan, field: &Field2d, label: String) -> Event {
    let raster = plan.picture(|i, j| field.at(i, j).norm_sqr());
    let peak = raster
        .values
        .iter()
        .copied()
        .fold(0.0, f32::max)
        .max(f32::MIN_POSITIVE);
    Event::Field {
        label,
        wavelength_um: plan.wavelengths[0],
        z_um: 0.0,
        intensity: Raster {
            values: raster.values.iter().map(|v| v / peak).collect(),
            ..raster
        },
    }
}

#[cfg(test)]
#[path = "topology_tests.rs"]
mod tests;
