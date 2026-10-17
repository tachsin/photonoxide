//! The `"static"` job: heat conduction, electrostatics or current continuity on a cross-section
//! of a stack and its shapes ([`crate::thermal`], [`crate::electro`]).
//!
//! The section is the plane x = `cut_x_um` (light travels along x, as in every kind of job):
//! across it y, up it z, over the window `y_um` × `z_um`. The grid is fitted to the stack's
//! interfaces, the shapes' edges along the cut and every rectangle the task draws in the
//! section, its cells `fine_nm` at their corners (by default `step_nm`) growing by `growth` (1.2
//! by default) to `step_nm`.
//!
//! - `physics = "heat"`: `[task.thermal_conductivity]` gives each medium's k in W/(m·K), by its
//!   name (a catalogue material's, as `Si` and `SiO2` in the presets, or a region's); a number,
//!   or `[across, up]`. `[[task.heater]]` rectangles give W/m, `[[task.sink]]` rectangles hold a
//!   temperature.
//! - `physics = "electrostatic"`: `[task.permittivity]` the relative static permittivities;
//!   `[[task.conductor]]` rectangles held at a potential (V), whose capacitance matrix (F/m) the
//!   run records.
//! - `physics = "current"`: `[task.electrical_conductivity]` in S/m, the `[[task.conductor]]`
//!   rectangles its contacts; with a `[task.thermal_conductivity]` too, the Joule heat is then
//!   conducted as a heat problem on the same grid.
//!
//! `[task.boundary]` holds each side (`left`, `right`, `bottom`, `top`): `"insulated"` (the
//! default: no flux, or a mirror), `{ fixed = 0.0 }`, `{ flux = 0.0 }` (W/m² or C/m² into the
//! window) or `{ convective = 10.0, ambient = 0.0 }` (W/(m²·K)). `[[task.region]]` rectangles
//! draw a named `medium` over the cut (a heater's metal); `[[task.probe]]` rectangles name the
//! means the run records.
//!
//! ```toml
//! name = "heater"
//! timeout_minutes = 5
//!
//! [task]
//! kind = "static"
//! physics = "heat"
//! stack = "soi_220"
//! y_um = [-20.0, 20.0]
//! z_um = [-10.0, 6.0]
//! step_nm = 1000.0
//! fine_nm = 20.0
//!
//! [[task.rect]]
//! layer = "Si"
//! center_um = [0.0, 0.0]
//! size_um = [10.0, 0.5]
//!
//! [task.thermal_conductivity]
//! Si = 148.0
//! SiO2 = 1.4
//! TiN = 28.0
//!
//! [task.boundary]
//! bottom = { fixed = 0.0 }
//! top = { convective = 10.0, ambient = 0.0 }
//!
//! [[task.region]]
//! y_um = [-1.0, 1.0]
//! z_um = [4.22, 4.34]
//! medium = "TiN"
//!
//! [[task.heater]]
//! y_um = [-1.0, 1.0]
//! z_um = [4.22, 4.34]
//! power_w_per_m = 10.0
//!
//! [[task.probe]]
//! name = "guide"
//! y_um = [-0.25, 0.25]
//! z_um = [2.0, 2.22]
//! ```
//!
//! The run records an [`Event::StaticField`] per field (the temperature or the potential, and a
//! current's Joule heat), each in its unit on pixels of about the finest step, and an
//! [`Event::StaticResult`] with the grid and the numbers: the probes' means, the largest value,
//! the heat or charge of each held rectangle, what leaves through each side, the balance's
//! imbalance, and a capacitance matrix or the contacts' currents and Joule power.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    CircleSpec, Event, PathSpec, RectSpec, RingSpec, StackField, draw, draw_paths, named_stack,
    task_error,
};
use crate::Result;
use crate::electro::{Conductor, Current, Electrostatic};
use crate::geometry::{Point, Shape};
use crate::run::{Job, Run, Stop};
use crate::statics::{Boundary, Cut, Features, Field, Grading, Grid, Section, Side, Tensor};
use crate::thermal::Heat;
use crate::units::Length;

/// The most cells a static job's grid may have.
const MOST_CELLS: usize = 4_000_000;

/// The most pixels a recorded field has along a side.
const PIXELS: usize = 400;

/// A number of a static job's result, with its unit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticValue {
    /// What it is, e.g. `"mean over guide"`.
    pub name: String,
    /// Its value.
    pub value: f64,
    /// Its unit, e.g. `"K"`, `"W/m"`, `"F/m"`.
    pub unit: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(untagged)]
enum CoefficientSpec {
    Isotropic(f64),
    Diagonal([f64; 2]),
}

impl CoefficientSpec {
    fn tensor(&self) -> Tensor {
        match *self {
            CoefficientSpec::Isotropic(v) => Tensor::isotropic(v),
            CoefficientSpec::Diagonal([a, b]) => Tensor::diagonal(a, b),
        }
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(untagged)]
enum SideSpec {
    Named(String),
    Fixed { fixed: f64 },
    Flux { flux: f64 },
    Convective { convective: f64, ambient: f64 },
}

impl SideSpec {
    fn boundary(&self, side: &str) -> Result<Boundary> {
        Ok(match self {
            SideSpec::Named(n) if n == "insulated" => Boundary::INSULATED,
            SideSpec::Named(n) => {
                return Err(task_error(format!(
                    "[task.boundary] {side} = \"{n}\": a side is \"insulated\", {{ fixed = … }}, \
                     {{ flux = … }} or {{ convective = …, ambient = … }}"
                )));
            }
            SideSpec::Fixed { fixed } => Boundary::Fixed(*fixed),
            SideSpec::Flux { flux } => Boundary::Flux(*flux),
            SideSpec::Convective {
                convective,
                ambient,
            } => Boundary::Convective {
                coefficient: *convective,
                ambient: *ambient,
            },
        })
    }
}

#[derive(Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "StaticBoundaries")]
struct BoundarySpec {
    left: Option<SideSpec>,
    right: Option<SideSpec>,
    bottom: Option<SideSpec>,
    top: Option<SideSpec>,
}

/// A rectangle in the section's plane: y across, z up, µm.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Area {
    y_um: [f64; 2],
    z_um: [f64; 2],
    medium: Option<String>,
    power_w_per_m: Option<f64>,
    temperature: Option<f64>,
    potential_v: Option<f64>,
    name: Option<String>,
}

impl Area {
    fn shape(&self, what: &str, k: usize) -> Result<Shape> {
        let ([y0, y1], [z0, z1]) = (self.y_um, self.z_um);
        if !(y0.is_finite()
            && y1.is_finite()
            && z0.is_finite()
            && z1.is_finite()
            && y0 < y1
            && z0 < z1)
        {
            return Err(task_error(format!(
                "[[task.{what}]] #{}: y_um and z_um must go from a smaller to a larger number",
                k + 1
            )));
        }
        Shape::rect(
            Point::um(0.5 * (y0 + y1), 0.5 * (z0 + z1)),
            Length::um(y1 - y0),
            Length::um(z1 - z0),
        )
    }

    fn need(&self, what: &str, k: usize, field: &str, value: Option<f64>) -> Result<f64> {
        value
            .filter(|v| v.is_finite())
            .ok_or_else(|| task_error(format!("[[task.{what}]] #{} needs a finite {field}", k + 1)))
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct StaticTask {
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    physics: String,
    stack: StackField,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    cut_x_um: Option<f64>,
    y_um: [f64; 2],
    z_um: [f64; 2],
    step_nm: f64,
    fine_nm: Option<f64>,
    growth: Option<f64>,
    #[serde(default)]
    rect: Vec<RectSpec>,
    #[serde(default)]
    circle: Vec<CircleSpec>,
    #[serde(default)]
    ring: Vec<RingSpec>,
    #[serde(default)]
    path: Vec<PathSpec>,
    #[serde(default)]
    shape: Vec<super::shapes::ShapeSpec>,
    #[serde(default)]
    #[schemars(with = "Vec<crate::job::schema::Component>")]
    component: Vec<toml::Table>,
    #[serde(default)]
    #[schemars(with = "Vec<crate::job::schema::Body>")]
    body: Vec<toml::Table>,
    #[serde(default)]
    thermal_conductivity: BTreeMap<String, CoefficientSpec>,
    #[serde(default)]
    electrical_conductivity: BTreeMap<String, CoefficientSpec>,
    #[serde(default)]
    permittivity: BTreeMap<String, CoefficientSpec>,
    #[serde(default)]
    boundary: BoundarySpec,
    #[serde(default)]
    region: Vec<Area>,
    #[serde(default)]
    heater: Vec<Area>,
    #[serde(default)]
    sink: Vec<Area>,
    #[serde(default)]
    conductor: Vec<Area>,
    #[serde(default)]
    probe: Vec<Area>,
}

/// What a static job sets up: the section, its grid and the physics' problems.
struct Setup {
    physics: String,
    structure: crate::stack::Structure,
    cut: f64,
    grid: Grid,
    window: ([f64; 2], [f64; 2]),
    pixels: (usize, usize),
    heat: Option<Heat>,
    electrostatic: Option<Electrostatic>,
    current: Option<Current>,
    probes: Vec<(String, Shape)>,
    held: Vec<String>,
}

fn sides(spec: &BoundarySpec) -> Result<Vec<(Side, Boundary)>> {
    let mut out = Vec::new();
    for (side, s) in [
        (Side::Left, &spec.left),
        (Side::Right, &spec.right),
        (Side::Bottom, &spec.bottom),
        (Side::Top, &spec.top),
    ] {
        if let Some(s) = s {
            out.push((side, s.boundary(side.name())?));
        }
    }
    Ok(out)
}

fn coefficients(
    section: &Section,
    table: &BTreeMap<String, CoefficientSpec>,
    what: &str,
) -> Result<Vec<(String, Tensor)>> {
    let mut out = Vec::new();
    for m in section.media() {
        let t = table.get(m).ok_or_else(|| {
            task_error(format!(
                "[task.{what}] has no value for the medium {m} (the section holds {})",
                section.media().join(", ")
            ))
        })?;
        let t = t.tensor();
        if !(t.xx.is_finite() && t.yy.is_finite() && t.xx > 0.0 && t.yy > 0.0) {
            return Err(task_error(format!(
                "[task.{what}] {m} must be positive and finite"
            )));
        }
        out.push((m.clone(), t));
    }
    Ok(out)
}

fn setup(job: &Job) -> Result<Setup> {
    let task: StaticTask = super::params::task(job)?;
    super::check_window("y_um", task.y_um)?;
    super::check_window("z_um", task.z_um)?;
    super::check_step(task.step_nm)?;
    let fine = task.fine_nm.unwrap_or(task.step_nm);
    if !(fine.is_finite() && fine > 0.0 && fine <= task.step_nm) {
        return Err(task_error(format!(
            "fine_nm must be positive and at most step_nm, got {fine}"
        )));
    }
    let growth = task.growth.unwrap_or(1.2);
    let grading = Grading::new(task.step_nm * 1e-3, fine * 1e-3, growth)
        .map_err(|e| task_error(e.to_string()))?;
    let stack = named_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?;
    let structure = draw_paths(
        draw(stack, &task.rect, &task.circle, &task.ring)?,
        &task.path,
        &task.shape,
        &task.component,
    )
    .and_then(|s| super::bodies::place(s, &task.body))?;
    let cut = task.cut_x_um.unwrap_or(0.0);
    if !cut.is_finite() {
        return Err(task_error("cut_x_um must be a number"));
    }
    let mut section =
        Section::cut(&structure, Cut::X(cut)).map_err(|e| task_error(e.to_string()))?;
    let mut features = Features::new();
    features.extend(section.features());
    for (k, r) in task.region.iter().enumerate() {
        let shape = r.shape("region", k)?;
        let medium = r
            .medium
            .as_deref()
            .ok_or_else(|| task_error(format!("[[task.region]] #{} needs a medium", k + 1)))?;
        section.draw(shape, medium);
    }
    features.extend(section.features());
    for (what, list) in [
        ("heater", &task.heater),
        ("sink", &task.sink),
        ("conductor", &task.conductor),
        ("probe", &task.probe),
    ] {
        for (k, a) in list.iter().enumerate() {
            features.shape(&a.shape(what, k)?);
        }
    }
    let window = (task.y_um, task.z_um);
    let grid = Grid::fitted(task.y_um, task.z_um, &grading, &features)
        .map_err(|e| task_error(e.to_string()))?;
    if grid.cells() > MOST_CELLS {
        return Err(task_error(format!(
            "the grid would have {} cells, more than {MOST_CELLS}: take a larger fine_nm, \
             step_nm or growth, or a smaller window",
            grid.cells()
        )));
    }
    let (ny, nz) = (
        (((task.y_um[1] - task.y_um[0]) / (fine * 1e-3)).ceil() as usize).clamp(1, PIXELS),
        (((task.z_um[1] - task.z_um[0]) / (fine * 1e-3)).ceil() as usize).clamp(1, PIXELS),
    );
    let boundaries = sides(&task.boundary)?;
    let mut probes = Vec::new();
    for (k, p) in task.probe.iter().enumerate() {
        let name = p.name.clone().unwrap_or_else(|| format!("probe {}", k + 1));
        probes.push((name, p.shape("probe", k)?));
    }
    let mut setup = Setup {
        physics: task.physics.clone(),
        structure: structure.clone(),
        cut,
        grid: grid.clone(),
        window,
        pixels: (ny, nz),
        heat: None,
        electrostatic: None,
        current: None,
        probes,
        held: Vec::new(),
    };
    let heat = |section: &Section| -> Result<Heat> {
        let mut heat = Heat::new(section.clone(), grid.clone());
        for (m, t) in coefficients(section, &task.thermal_conductivity, "thermal_conductivity")? {
            heat.conductivity(&m, t);
        }
        for &(side, b) in &boundaries {
            heat.boundary(side, b);
        }
        for (k, h) in task.heater.iter().enumerate() {
            let p = h.need("heater", k, "power_w_per_m", h.power_w_per_m)?;
            heat.heater(h.shape("heater", k)?, p);
        }
        for (k, s) in task.sink.iter().enumerate() {
            let t = s.need("sink", k, "temperature", s.temperature)?;
            heat.sink(s.shape("sink", k)?, t);
        }
        Ok(heat)
    };
    // something must hold the solution's level: a fixed or convective side, or a held rectangle
    let held_side = boundaries
        .iter()
        .any(|(_, b)| !matches!(b, Boundary::Flux(_)));
    let held = match task.physics.as_str() {
        "heat" => held_side || !task.sink.is_empty(),
        _ => held_side || !task.conductor.is_empty(),
    };
    if !held {
        return Err(task_error(
            "nothing holds the solution's level: give [task.boundary] a fixed or convective side, \
             or the task a [[task.sink]] or [[task.conductor]]",
        ));
    }
    match task.physics.as_str() {
        "heat" => {
            setup.held = (1..=task.sink.len()).map(|k| format!("sink {k}")).collect();
            setup.heat = Some(heat(&section)?);
        }
        "electrostatic" => {
            let mut e = Electrostatic::new(section.clone(), grid.clone());
            for (m, t) in coefficients(&section, &task.permittivity, "permittivity")? {
                e.permittivity(&m, t);
            }
            for &(side, b) in &boundaries {
                e.boundary(side, b);
            }
            for (k, c) in task.conductor.iter().enumerate() {
                let v = c.need("conductor", k, "potential_v", c.potential_v)?;
                e.conductor(Conductor::shape(c.shape("conductor", k)?, v));
            }
            setup.held = (1..=task.conductor.len())
                .map(|k| format!("conductor {k}"))
                .collect();
            setup.electrostatic = Some(e);
        }
        "current" => {
            let mut c = Current::new(section.clone(), grid.clone());
            for (m, t) in coefficients(
                &section,
                &task.electrical_conductivity,
                "electrical_conductivity",
            )? {
                c.conductivity(&m, t);
            }
            for &(side, b) in &boundaries {
                if matches!(b, Boundary::Fixed(_) | Boundary::Flux(_)) {
                    c.boundary(side, b);
                }
            }
            for (k, a) in task.conductor.iter().enumerate() {
                let v = a.need("conductor", k, "potential_v", a.potential_v)?;
                c.contact(Conductor::shape(a.shape("conductor", k)?, v));
            }
            if task.conductor.is_empty() {
                return Err(task_error(
                    "a current job needs [[task.conductor]] contacts at potentials",
                ));
            }
            setup.held = (1..=task.conductor.len())
                .map(|k| format!("contact {k}"))
                .collect();
            setup.current = Some(c);
            if !task.thermal_conductivity.is_empty() {
                setup.heat = Some(heat(&section)?);
            }
        }
        other => {
            return Err(task_error(format!(
                "physics = \"{other}\": a static job's physics is \"heat\", \"electrostatic\" or \
                 \"current\""
            )));
        }
    }
    Ok(setup)
}

/// Checks a static job without solving it: its task, the stack and shapes, the grid's size,
/// every medium's coefficient and every boundary.
pub(super) fn check(job: &Job) -> Result<()> {
    setup(job).map(|_| ())
}

fn value(name: impl Into<String>, value: f64, unit: &str) -> StaticValue {
    StaticValue {
        name: name.into(),
        value,
        unit: unit.to_owned(),
    }
}

impl Setup {
    fn picture(&self, field: &Field) -> crate::raster::Raster {
        field.raster(self.window.0, self.window.1, self.pixels.0, self.pixels.1)
    }

    /// The structure in 3D for the studio's view: as long along x as the window is wide,
    /// about the cut, its media's permittivities at 1.55 µm (`None` if a material has no data
    /// there: the view is the optics', the problem doesn't need it).
    fn scene(&self) -> Option<Event> {
        let (y, z) = self.window;
        let half = 0.5 * (y[1] - y[0]);
        let wavelength = crate::units::Wavelength::um(1.55).ok()?;
        super::scene(
            &self.structure,
            [self.cut - half, self.cut + half],
            y,
            z,
            wavelength,
        )
        .ok()
    }
}

/// A static job's scene, for the job builder's preview.
pub(super) fn preview(job: &Job) -> Result<Event> {
    let setup = setup(job)?;
    setup.scene().ok_or_else(|| {
        task_error("the stack's materials have no permittivity at 1.55 um to draw it with")
    })
}

/// A field's numbers: the probes' means, its largest value and where, what each held rectangle
/// (`held`, by name) gives, what leaves through each side, and the imbalance.
fn summary(
    field: &Field,
    probes: &[(String, Shape)],
    held: &[String],
    (unit, flow): (&str, &str),
    out: &mut Vec<StaticValue>,
) -> Result<()> {
    for (name, shape) in probes {
        out.push(value(format!("mean over {name}"), field.mean(shape)?, unit));
    }
    let (max, at) = field.max();
    out.push(value("largest", max, unit));
    out.push(value("largest at y", at[0], "um"));
    out.push(value("largest at z", at[1], "um"));
    for (name, q) in held.iter().zip(field.held()) {
        out.push(value(format!("from {name}"), *q, flow));
    }
    for side in Side::ALL {
        out.push(value(
            format!("out through {}", side.name()),
            field.outflow(side),
            flow,
        ));
    }
    out.push(value("imbalance", field.imbalance(), ""));
    Ok(())
}

/// Runs a static job into `run`.
pub(super) fn run(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let setup = setup(job)?;
    if let Some(scene) = setup.scene() {
        run.record(&scene)?;
    }
    let (smallest_x, largest_x) = setup.grid.x().steps();
    let (smallest_y, largest_y) = setup.grid.y().steps();
    let mut values = Vec::new();
    let axes = ["y".to_owned(), "z".to_owned()];
    if let Some(e) = &setup.electrostatic {
        let field = e.solve()?;
        run.record(&Event::StaticField {
            label: "potential".into(),
            unit: "V".into(),
            axes: axes.clone(),
            raster: setup.picture(&field),
        })?;
        summary(
            &field,
            &setup.probes,
            &setup.held,
            ("V", "C/m"),
            &mut values,
        )?;
        if !e.conductors().is_empty() && e.conductors().len() <= 8 && stop.reason().is_none() {
            let c = e.capacitance()?;
            for (i, row) in c.iter().enumerate() {
                for (j, v) in row.iter().enumerate() {
                    values.push(value(format!("C[{}][{}]", i + 1, j + 1), *v, "F/m"));
                }
            }
        }
    }
    let mut joule = None;
    if let Some(c) = &setup.current {
        let flow = c.solve()?;
        run.record(&Event::StaticField {
            label: "potential".into(),
            unit: "V".into(),
            axes: axes.clone(),
            raster: setup.picture(flow.field()),
        })?;
        summary(
            flow.field(),
            &setup.probes,
            &setup.held,
            ("V", "A/m"),
            &mut values,
        )?;
        values.push(value("Joule power", flow.power(), "W/m"));
        joule = Some(flow);
    }
    if let Some(h) = &setup.heat
        && stop.reason().is_none()
    {
        let mut h = h.clone();
        if let Some(flow) = &joule {
            h.joule(flow)?;
        }
        let field = h.solve()?;
        run.record(&Event::StaticField {
            label: "temperature".into(),
            unit: "K".into(),
            axes,
            raster: setup.picture(&field),
        })?;
        // a current's contacts hold the potential, not the temperature
        let held: &[String] = if joule.is_some() { &[] } else { &setup.held };
        let mut heat_values = Vec::new();
        summary(&field, &setup.probes, held, ("K", "W/m"), &mut heat_values)?;
        if joule.is_some() {
            for v in &mut heat_values {
                v.name = format!("heat: {}", v.name);
            }
        }
        values.extend(heat_values);
    }
    run.record(&Event::StaticResult {
        physics: setup.physics.clone(),
        cells: [setup.grid.x().cells(), setup.grid.y().cells()],
        step_um: [smallest_x.min(smallest_y), largest_x.max(largest_y)],
        values,
    })
}
