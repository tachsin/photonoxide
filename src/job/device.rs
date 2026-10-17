//! Jobs that compute a device: a `[device]` table ([`crate::device::Spec`]) with a `[task]` that
//! says how, and [`from_device`], which writes one.
//!
//! - `kind = "spectrum"`: the device's own model at its design's fidelity, the component
//!   [`crate::device::Device::component`] gives, its S-parameters over a band of wavelengths and
//!   its figures. At a solver-backed fidelity that is a solve: of its cross-section's modes, or
//!   by 2D or 3D FDFD, on the device's own grid or one the task sets. The run records exactly
//!   what the device gives, to the last bit.
//! - `kind = "fdtd"`: the device's drawn structure stepped in time, as an `"fdtd"` job steps
//!   its shapes; its sources and monitors find the device's ports by name (`port = "o1"`), and
//!   each port's guide runs straight on through the window's edge.
//!
//! ```toml
//! name = "mmi-3d"
//!
//! [device]
//! kind = "MMI 1x2"
//! fidelity = "3D"
//!
//! [task]
//! kind = "spectrum"
//! from_um = 1.5          # the band, the device's own (its wavelength ± span/2) by default
//! to_um = 1.6
//! points = 101
//! step_nm = 80.0         # the grid: the cells in the chip's plane (an FDFD's, or a mode
//! step_z_nm = 40.0       # solve's mesh step); 3D FDFD's cells' height; the cladding kept
//! margin_um = 0.5        # beside the guides; the wavelengths an FDFD samples and its PML
//! samples = 3            # cells. Each left out is the device's own.
//! pml_cells = 12
//! # model_2d = "fdfd"    # an MMI's 2D model by FDFD in place of its guided modes
//! ```

use std::sync::mpsc;
use std::time::{Duration, Instant};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use super::{Event, ScenePort, reason, task_error};
use crate::circuit::Fidelity;
use crate::device::spec::Grids;
use crate::device::{Built, Design, Device, Spec};
use crate::generator;
use crate::geometry::{Point, Polygon, Shape};
use crate::run::{Job, Run, Stop};
use crate::stack::{Etch, Structure};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// How a job made by [`from_device`] computes its device.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Compute {
    /// FDTD of the device's drawn structure in place of its own model: `2` in the core layer's
    /// plane by the effective index method (a 2D estimate, not the device's performance), `3` the
    /// stack; `None`, its model at the design's fidelity.
    pub fdtd: Option<u8>,
    /// The band, µm: from, to and how many wavelengths; the device's own (its `wavelength` ±
    /// `span`/2, or 1.5 to 1.6 µm), at 101 points (21 for FDTD), when `None`.
    pub band: Option<(f64, f64, usize)>,
    /// The cells' size in the chip's plane, nm: a mode solve's mesh step, an FDFD's cells, an
    /// FDTD's (40 nm in 2D and 50 nm in 3D when `None`); the device's own otherwise.
    pub step_nm: Option<f64>,
    /// A 3D FDFD's cells' height, nm.
    pub step_z_nm: Option<f64>,
    /// The cladding kept beside the guides, µm.
    pub margin_um: Option<f64>,
    /// The wavelengths an FDFD samples across the device's span.
    pub samples: Option<usize>,
    /// An FDFD's PML cells on each side (an FDTD's: 20 when `None`).
    pub pml_cells: Option<usize>,
    /// An MMI's 2D model by 2D FDFD in place of its guided modes.
    pub fdfd_2d: bool,
    /// The run's hard limit, minutes: 30 when `None`.
    pub timeout_minutes: Option<f64>,
}

impl Compute {
    /// The device's own model at its design's fidelity, on its own grid.
    pub fn model() -> Compute {
        Compute::default()
    }

    /// FDTD of the device's structure: `dimensions` 2 (by the effective index method) or 3.
    pub fn fdtd(dimensions: u8) -> Compute {
        Compute {
            fdtd: Some(dimensions),
            ..Compute::default()
        }
    }
}

/// The job that computes `spec`'s device as `compute` says, named `name`: a `[device]` table
/// holding the design, and a `[task]` of kind `"spectrum"` (the device's own model at the
/// design's fidelity: its cross-section's modes, 2D or 3D FDFD, or a closed form) or `"fdtd"`
/// (its structure stepped in time, a mode source at its first port and a mode monitor at each
/// other). It is checked by [`super::check`], and its run is recorded and replayed as any job's;
/// a `"spectrum"` run records what [`Device::component`] gives at the design's fidelity, to the
/// last bit.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a name that isn't a job's, a design [`Spec::build`] refuses, and
/// the errors of [`super::check`] for the job it makes (a grid the device doesn't take, a 3D
/// solve larger than the direct solver holds here).
pub fn from_device(name: &str, spec: &Spec, compute: &Compute) -> Result<Job> {
    let (device, design) = spec.build()?;
    let values = design.values();
    let band = match compute.band {
        Some(b) => b,
        None => {
            let (from, to) = own_band(values);
            (from, to, if compute.fdtd.is_some() { 21 } else { 101 })
        }
    };
    let mut task = Table::new();
    let float = Value::Float;
    match compute.fdtd {
        None => {
            task.insert("kind".into(), "spectrum".into());
            task.insert("from_um".into(), float(band.0));
            task.insert("to_um".into(), float(band.1));
            task.insert("points".into(), Value::Integer(band.2 as i64));
            let optional = [
                ("step_nm", compute.step_nm),
                ("step_z_nm", compute.step_z_nm),
                ("margin_um", compute.margin_um),
            ];
            for (key, v) in optional {
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
        }
        Some(dimensions) => {
            fdtd_task(
                &mut task,
                device.as_ref(),
                &design,
                dimensions,
                band,
                compute,
            )?;
        }
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

/// The band a device states: its `wavelength` ± `span`/2, or 1.5 to 1.6 µm.
pub(super) fn own_band(values: &crate::device::Values) -> (f64, f64) {
    match (values.get("wavelength"), values.get("span")) {
        (Some(l), Some(s)) if s > 0.0 => (l - s / 2.0, l + s / 2.0),
        (Some(l), _) => (l - 0.05, l + 0.05),
        _ => (1.5, 1.6),
    }
}

/// An `"fdtd"` task for `device`: its window around the drawing with room for the PMLs, the
/// band as its spectrum, a mode source at the first port and a mode monitor at each port.
fn fdtd_task(
    task: &mut Table,
    device: &dyn Device,
    design: &Design,
    dimensions: u8,
    band: (f64, f64, usize),
    compute: &Compute,
) -> Result<()> {
    let values = design.values();
    let geometry = device.geometry(values)?;
    let core = geometry
        .ports
        .first()
        .map(|p| p.layer.clone())
        .ok_or_else(|| task_error(format!("a {} has no ports to drive", device.kind())))?;
    let step = compute
        .step_nm
        .unwrap_or(if dimensions == 3 { 50.0 } else { 40.0 });
    let pml = compute.pml_cells.unwrap_or(20);
    // the PMLs inside the window, and 0.5 µm between them and the device
    let room = 0.5 + pml as f64 * step / 1e3 + compute.margin_um.unwrap_or(0.0);
    let (x, y) = extent(&geometry);
    let r = |v: f64| (v * 1e3).round() / 1e3;
    let float = Value::Float;
    let pair = |a: f64, b: f64| Value::Array(vec![float(r(a)), float(r(b))]);
    task.insert("kind".into(), "fdtd".into());
    task.insert(
        "wavelength_um".into(),
        float(values.get("wavelength").unwrap_or(0.5 * (band.0 + band.1))),
    );
    task.insert("layer".into(), Value::String(core));
    task.insert("dimensions".into(), Value::Integer(i64::from(dimensions)));
    task.insert("x_um".into(), pair(x[0] - room, x[1] + room));
    task.insert("y_um".into(), pair(y[0] - room, y[1] + room));
    task.insert("step_nm".into(), float(step));
    task.insert("pml_cells".into(), Value::Integer(pml as i64));
    let mut spectrum = Table::new();
    spectrum.insert("from_um".into(), float(band.0));
    spectrum.insert("to_um".into(), float(band.1));
    spectrum.insert("points".into(), Value::Integer(band.2 as i64));
    task.insert("spectrum".into(), Value::Table(spectrum));
    let item = |port: &str| {
        let mut t = Table::new();
        t.insert("type".into(), "mode".into());
        t.insert("port".into(), Value::String(port.to_owned()));
        Value::Table(t)
    };
    let first = geometry.ports[0].name.clone();
    task.insert("source".into(), Value::Array(vec![item(&first)]));
    task.insert(
        "monitor".into(),
        Value::Array(geometry.ports.iter().map(|p| item(&p.name)).collect()),
    );
    Ok(())
}

/// The drawing's extent with its ports, µm: x and y.
fn extent(geometry: &crate::device::Geometry) -> ([f64; 2], [f64; 2]) {
    let s = &geometry.structure;
    let (mut x, mut y) = (
        [f64::INFINITY, f64::NEG_INFINITY],
        [f64::INFINITY, f64::NEG_INFINITY],
    );
    let mut take = |p: Point| {
        let (px, py) = (p.x.to_um(), p.y.to_um());
        x = [x[0].min(px), x[1].max(px)];
        y = [y[0].min(py), y[1].max(py)];
    };
    for layer in s.stack().layers() {
        for shape in s.shapes(&layer.name) {
            let b = shape.bounds();
            take(b.min);
            take(b.max);
        }
    }
    for p in &geometry.ports {
        take(p.position);
    }
    if !x[0].is_finite() {
        return ([0.0, 0.0], [0.0, 0.0]);
    }
    (x, y)
}

// ---- a job's device, for the tasks that compute it ----

/// The job's design: its spec, read from its `[device]` table, its compact model's file found
/// from the job's folder ([`Job::dir`]).
pub(super) fn spec_of(job: &Job) -> Result<Spec> {
    let table = job
        .device()
        .ok_or_else(|| task_error("the job has no [device] table"))?;
    let spec = Spec::from_table(table)?;
    Ok(match job.dir() {
        Some(dir) => spec.resolved_in(dir),
        None => spec,
    })
}

/// The kind of the task beside a job's `[device]`: `None` for a design alone.
pub(super) fn task_kind(job: &Job) -> Result<Option<String>> {
    if job.task().is_empty() {
        return Ok(None);
    }
    match job.task().get("kind").and_then(Value::as_str) {
        Some(k @ ("spectrum" | "fdtd" | "sweep" | "corners" | "optimize" | "fit")) => {
            Ok(Some(k.to_owned()))
        }
        Some(other) => Err(task_error(format!(
            "a job with a [device] table computes it by a \"spectrum\" task (its own model), \
             studies it by a \"sweep\" or \"corners\" one, optimizes it by an \"optimize\" one, \
             fits a compact model to it by a \"fit\" one, or steps it in time by an \"fdtd\" \
             one, not \"{other}\": the device is the structure"
        ))),
        None => Err(task_error(
            "a device's task needs a kind: \"spectrum\", \"sweep\", \"corners\", \"optimize\", \
             \"fit\" or \"fdtd\"",
        )),
    }
}

/// The ports of a job's device, named as the device names them (`"o1"`), for its sources and
/// monitors to find; none for a job without a device.
///
/// # Errors
///
/// What [`Spec::build`] and the device's geometry refuse.
pub(super) fn named_ports(job: &Job) -> Result<Vec<(String, generator::Port)>> {
    if job.device().is_none() {
        return Ok(Vec::new());
    }
    let (device, design) = spec_of(job)?.build()?;
    Ok(device
        .geometry(design.values())?
        .ports
        .into_iter()
        .map(|p| {
            let port = generator::Port::new(&p.name, p.position, p.direction, p.width, &p.layer);
            (p.name, port)
        })
        .collect())
}

/// The ports of a job's device as a run records them ([`Event::Ports`]).
pub(super) fn scene_ports(job: &Job) -> Result<Vec<ScenePort>> {
    Ok(named_ports(job)?
        .into_iter()
        .map(|(name, p)| ScenePort {
            name,
            layer: p.layer.clone(),
            position_um: [p.position.x.to_um(), p.position.y.to_um()],
            direction_deg: p.direction,
            width_um: p.width.to_um(),
        })
        .collect())
}

/// The structure of a job's device at its design's values, each port's guide led straight on
/// out of the window `x` × `y` (µm), cut as the core's shapes are, so that the guides run
/// through the window's edges and its PMLs; `None` for a job without a device.
///
/// # Errors
///
/// What [`Spec::build`] and the device's geometry refuse.
pub(super) fn structure(job: &Job, x: [f64; 2], y: [f64; 2]) -> Result<Option<Structure>> {
    if job.device().is_none() {
        return Ok(None);
    }
    let (device, design) = spec_of(job)?.build()?;
    let g = device.geometry(design.values())?;
    let mut s = g.structure.clone();
    // past the window from anywhere inside it
    let length = 2.0 * ((x[1] - x[0]) + (y[1] - y[0])) + 10.0;
    for p in &g.ports {
        let etch = s.etches(&p.layer).first().copied().unwrap_or(Etch::FULL);
        s.draw_etched(&p.layer, lead(p, length)?, etch)?;
    }
    Ok(Some(s))
}

/// A port's guide led straight on `length` µm from it, the way light leaves by.
fn lead(p: &crate::device::Port, length: f64) -> Result<Shape> {
    let (c, s) = (
        p.direction.to_radians().cos(),
        p.direction.to_radians().sin(),
    );
    let (x, y, w) = (p.position.x.to_um(), p.position.y.to_um(), p.width.to_um());
    let centre = Point::um(x + c * length / 2.0, y + s * length / 2.0);
    let quarter = p.direction.rem_euclid(90.0);
    if quarter < 1e-9 || quarter > 90.0 - 1e-9 {
        // along x or y: a rectangle
        let axis_x = c.abs() > 0.5;
        let (wx, wy) = if axis_x { (length, w) } else { (w, length) };
        let centre = if axis_x {
            Point::um(x + c.signum() * length / 2.0, y)
        } else {
            Point::um(x, y + s.signum() * length / 2.0)
        };
        return Shape::rect(centre, Length::um(wx), Length::um(wy));
    }
    let (ux, uy) = (c * length / 2.0, s * length / 2.0);
    let (nx, ny) = (-s * w / 2.0, c * w / 2.0);
    let (cx, cy) = (centre.x.to_um(), centre.y.to_um());
    Ok(Shape::Polygon(Polygon::new(vec![
        Point::um(cx - ux - nx, cy - uy - ny),
        Point::um(cx + ux - nx, cy + uy - ny),
        Point::um(cx + ux + nx, cy + uy + ny),
        Point::um(cx - ux + nx, cy - uy + ny),
    ])?))
}

/// Refuses an `"fdtd"` task beside a device that draws shapes or takes a stack of its own: the
/// device is the structure.
pub(super) fn check_fdtd_task(job: &Job) -> Result<()> {
    if job.device().is_none() {
        return Ok(());
    }
    for key in [
        "stack",
        "core_nm",
        "bottom_oxide_um",
        "rect",
        "circle",
        "ring",
        "path",
        "shape",
        "component",
        "body",
        "cell",
        "parameters",
    ] {
        if job.task().contains_key(key) {
            return Err(task_error(format!(
                "a device's fdtd task takes no {key}: the [device] table is the structure, on \
                 its stack"
            )));
        }
    }
    Ok(())
}

/// Copies the compact model a job's design names into the run's folder, under its file's name,
/// so that the job the run keeps runs again from there.
fn keep_compact(job: &Job, run: &Run) -> Result<()> {
    let Some(name) = job
        .device()
        .and_then(|d| d.get("compact"))
        .and_then(Value::as_str)
    else {
        return Ok(());
    };
    if run.dir().as_os_str().is_empty() {
        return Ok(());
    }
    let Some(from) = spec_of(job)?.compact else {
        return Ok(());
    };
    let Some(file) = std::path::Path::new(name).file_name() else {
        return Ok(());
    };
    let to = run.dir().join(file);
    std::fs::copy(&from, &to).map_err(|e| Error::Io {
        path: to.display().to_string(),
        reason: e.to_string(),
    })?;
    Ok(())
}

// ---- the spectrum task ----

/// A `"spectrum"` task: the band and the grids.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct SpectrumTask {
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    from_um: Option<f64>,
    to_um: Option<f64>,
    points: Option<usize>,
    step_nm: Option<f64>,
    step_z_nm: Option<f64>,
    margin_um: Option<f64>,
    samples: Option<usize>,
    pml_cells: Option<usize>,
    model_2d: Option<String>,
}

/// A `"spectrum"` job read and checked: the design, the device on its grids, the wavelengths.
struct Prepared {
    spec: Spec,
    device: Box<dyn Device>,
    design: Design,
    wavelengths: Vec<f64>,
}

/// Reads and checks a `"spectrum"` job, without solving: the design, the grids the device takes,
/// the band, and a 3D FDFD's size.
fn prepare(job: &Job) -> Result<Prepared> {
    let spec = spec_of(job)?;
    let (task, grids) = spectrum_task(job)?;
    let (_, design) = spec.build()?;
    let device = spec.device_with(&grids)?;
    // a 3D FDFD larger than the direct solver holds here is refused before it starts
    spec.fdfd_size(&grids)?;
    let wavelengths = band_of(task.from_um, task.to_um, task.points, 101, &design)?;
    Ok(Prepared {
        spec,
        device,
        design,
        wavelengths,
    })
}

/// The size of the FDFD solve a job's device takes, for a `"spectrum"` task whose model is one
/// (an MMI's or a Y-branch's 2D and 3D, an MMI's 2D by FDFD only): the cells along x and y and
/// the unknowns of a solve; `None` for another model, a design alone or an `"fdtd"` task. What a
/// window says of a 3D run before it starts.
///
/// # Errors
///
/// What [`super::check`] refuses of the device and its grids, and a 3D solve larger than the
/// direct solver holds here (1.2 million unknowns), the error saying how large.
pub fn fdfd_size(job: &Job) -> Result<Option<(usize, usize, f64)>> {
    if job.device().is_none() || task_kind(job)?.as_deref() != Some("spectrum") {
        return Ok(None);
    }
    let (_, grids) = spectrum_task(job)?;
    spec_of(job)?.fdfd_size(&grids)
}

/// The wavelengths of a band a task gives as `from_um`, `to_um` and `points` (each optional):
/// the design's own band (its `wavelength` ± `span`/2) when the ends aren't given, at
/// `default_points` when the count isn't, evenly spaced.
///
/// # Errors
///
/// [`Error::Parse`] for one end without the other, ends that aren't a band, or a count that
/// isn't from 1 to 20001 (at distinct ends unless it is 1).
pub(super) fn band_of(
    from_um: Option<f64>,
    to_um: Option<f64>,
    points: Option<usize>,
    default_points: usize,
    design: &Design,
) -> Result<Vec<f64>> {
    let (from, to) = match (from_um, to_um) {
        (Some(a), Some(b)) => (a, b),
        (None, None) => own_band(design.values()),
        _ => return Err(task_error("give both from_um and to_um, or neither")),
    };
    let points = points.unwrap_or(default_points);
    if !(from.is_finite() && to.is_finite() && from > 0.0 && to >= from) {
        return Err(task_error(format!(
            "the band must go from a positive wavelength to one no smaller, got {from} to {to} um"
        )));
    }
    if points == 0 || points > 20_001 || (points > 1 && to == from) {
        return Err(task_error(format!(
            "the band takes 1 to 20001 points, at distinct ends unless it has one, got {points}"
        )));
    }
    let wavelengths: Vec<f64> = if points == 1 {
        vec![from]
    } else {
        (0..points)
            .map(|k| from + (to - from) * k as f64 / (points - 1) as f64)
            .collect()
    };
    for &w in &wavelengths {
        Wavelength::um(w)?;
    }
    Ok(wavelengths)
}

/// The grids a task sets: its `step_nm`, `step_z_nm`, `margin_um`, `samples`, `pml_cells` and
/// `model_2d`, each the device's own when left out.
///
/// # Errors
///
/// [`Error::Parse`] for a `model_2d` that isn't `"guided"` or `"fdfd"`.
pub(super) fn grids_of(
    step_nm: Option<f64>,
    step_z_nm: Option<f64>,
    margin_um: Option<f64>,
    samples: Option<usize>,
    pml_cells: Option<usize>,
    model_2d: Option<&str>,
) -> Result<Grids> {
    let fdfd_2d = match model_2d {
        None | Some("guided") => false,
        Some("fdfd") => true,
        Some(other) => {
            return Err(task_error(format!(
                "model_2d \"{other}\": \"guided\" (an MMI's guided modes) or \"fdfd\""
            )));
        }
    };
    Ok(Grids {
        step: step_nm.map(|v| v / 1e3),
        step_z: step_z_nm.map(|v| v / 1e3),
        margin: margin_um,
        samples,
        pml: pml_cells,
        fdfd_2d,
    })
}

/// A `"spectrum"` task read, and the grids it sets.
fn spectrum_task(job: &Job) -> Result<(SpectrumTask, Grids)> {
    let task: SpectrumTask = job
        .task()
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(e.to_string()))?;
    let grids = grids_of(
        task.step_nm,
        task.step_z_nm,
        task.margin_um,
        task.samples,
        task.pml_cells,
        task.model_2d.as_deref(),
    )?;
    Ok((task, grids))
}

/// [`super::check`] for a `"spectrum"` job.
pub(super) fn check_spectrum(job: &Job) -> Result<()> {
    prepare(job).map(|_| ())
}

/// One of a device's figures, as a run records it ([`Event::DeviceModel`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DeviceFigure {
    /// What it is, e.g. `"effective index"`.
    pub name: String,
    /// Its symbol, e.g. `"n_eff"`.
    pub symbol: String,
    /// Its value, in `unit`.
    pub value: f64,
    /// Its unit, or `""`.
    pub unit: String,
    /// The fidelity it comes from: `"analytic"`, `"2D"` or `"3D"`.
    pub fidelity: String,
    /// The grid it was computed on, or `"exact: no grid"`.
    pub grid: String,
}

/// A fidelity as a design writes it.
pub(super) fn fidelity_name(f: Fidelity) -> &'static str {
    match f {
        Fidelity::Analytic => "analytic",
        Fidelity::Compact => "compact",
        Fidelity::TwoD => "2D",
        Fidelity::ThreeD => "3D",
        Fidelity::Measured => "measured",
    }
}

/// The device's component at its values and fidelity, built on a thread of its own so that a
/// stop ends the run at once (the solve already started is let go, its result discarded);
/// `None` when the stop came first.
fn built(
    device: Box<dyn Device>,
    design: &Design,
    f: Fidelity,
    stop: &Stop,
) -> Result<Option<Built>> {
    if stop.reason().is_some() {
        return Ok(None);
    }
    let values = design.values().clone();
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("photonoxide device solve".into())
        .spawn(move || {
            let _ = tx.send(device.component(&values, f));
        })
        .map_err(|e| Error::invalid("device", format!("can't start the solve: {e}")))?;
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(result) => return result.map(Some),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if stop.reason().is_some() {
                    return Ok(None);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(Error::invalid("device", "the solve ended without a result"));
            }
        }
    }
}

/// Runs a `"spectrum"` job: the scene and ports, then the device's model at its fidelity (its
/// method, source, figures and the time the solve took), then its S-parameters at each
/// wavelength of the band.
pub(super) fn run_spectrum(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let p = prepare(job)?;
    run.record(&super::device_scene(job)?)?;
    run.record(&Event::Ports {
        ports: scene_ports(job)?,
    })?;
    let f = p.spec.fidelity;
    let method = p
        .device
        .fidelities()
        .iter()
        .find(|m| m.fidelity == f)
        .cloned()
        .ok_or_else(|| task_error(format!("a {} has no {f} model", p.spec.kind)))?;
    let kind = p.device.kind().to_owned();
    let started = Instant::now();
    let Some(built) = built(p.device, &p.design, f, stop)? else {
        return Ok(());
    };
    let seconds = started.elapsed().as_secs_f64();
    let provenance = built.component.provenance();
    run.record(&Event::DeviceModel {
        device: kind,
        fidelity: fidelity_name(f).to_owned(),
        method: method.name,
        write_up: method.write_up,
        papers: method.papers,
        source: provenance.source,
        grid: built
            .figures
            .first()
            .map(|g| g.grid.clone())
            .unwrap_or_default(),
        seconds,
        figures: built
            .figures
            .iter()
            .map(|g| DeviceFigure {
                name: g.name.clone(),
                symbol: g.symbol.clone(),
                value: g.value,
                unit: g.unit.clone(),
                fidelity: fidelity_name(g.fidelity).to_owned(),
                grid: g.grid.clone(),
            })
            .collect(),
    })?;
    let ports: Vec<String> = built
        .component
        .ports()
        .iter()
        .map(|q| q.name.clone())
        .collect();
    let indices: Vec<f64> = built
        .component
        .ports()
        .iter()
        .map(|q| q.mode.as_ref().map_or(0.0, |m| m.effective_index))
        .collect();
    for &w in &p.wavelengths {
        if stop.reason().is_some() {
            return Ok(());
        }
        let s = built.s_matrix(Wavelength::um(w)?)?;
        let n = ports.len();
        run.record(&Event::SParameters {
            wavelength_um: w,
            ports: ports.clone(),
            effective_indices: indices.clone(),
            s: (0..n)
                .map(|q| {
                    (0..n)
                        .map(|k| {
                            let v = s[(q, k)];
                            [v.re, v.im]
                        })
                        .collect()
                })
                .collect(),
        })?;
    }
    Ok(())
}

/// Runs a job with a `[device]` table: the design alone records its scene; a `"spectrum"` task
/// its model's S-parameters; a `"sweep"` or `"corners"` task its model at each of the study's
/// designs ([`super::study`]); an `"fdtd"` task its structure in time.
pub(super) fn execute(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let kind = task_kind(job)?;
    if kind.as_deref() == Some("fdtd") {
        // as any fdtd job, its structure the device's
        return super::execute_task(job, run, stop, "fdtd");
    }
    if kind.as_deref() == Some("optimize") {
        super::check(job)?;
        return Err(task_error(
            "an \"optimize\" job's optimizer is genoxide's, which the photonoxide program runs: \
             photonoxide run <job> (the studio window shows it live); the library supplies its \
             objectives and records it (photonoxide::job::Problem, Recorder)",
        ));
    }
    // checked before anything is recorded
    super::check(job)?;
    let scene = super::device_scene(job)?;
    keep_compact(job, run)?;
    run.record(&Event::Started {
        job: job.name().to_owned(),
        kind: "device".into(),
    })?;
    match kind.as_deref() {
        Some("sweep" | "corners") => super::study::run(job, run, stop, scene)?,
        Some("fit") => super::fit::run(job, run, stop, scene)?,
        Some(_) => run_spectrum(job, run, stop)?,
        None => run.record(&scene)?,
    }
    let seconds = run.elapsed().as_secs_f64();
    run.record(&Event::Finished {
        stopped: reason(stop),
        seconds,
    })
}
