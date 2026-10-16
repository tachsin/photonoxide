//! A device's studies: a job's `[device]` computed at many designs, each a point of the study.
//!
//! - `kind = "sweep"`: one or two of the design's parameters (the device's own, or one its
//!   expressions name) stepped evenly between two values, the second fastest, and the band of
//!   wavelengths at each point.
//! - `kind = "corners"`: the nominal design and named corners of it, each its parameters moved
//!   by offsets (a width bias, a thickness) and its materials at a temperature.
//!
//! Each point is the design whose parameters are those values, written as plain numbers in
//! place of their expressions, computed as a `"spectrum"` task computes its design: the device's
//! own model at the design's fidelity, on the task's grids. A point is the run of that design's
//! own job to the last bit ([`Study::designs`] gives them), whatever the threads or a farm's
//! workers. The run records the scene and ports of the nominal design, then
//! [`Event::DeviceStudy`] (what the study steps through), then an [`Event::DevicePoint`] per
//! point in order: the device's values there, its figures with their grid, its ports' effective
//! indices and its S-parameters over the band. A point the model refuses (a fit outside its
//! range, a solve that fails) records why, and the study goes on.
//!
//! ```toml
//! name = "coupler-gap"
//!
//! [device]
//! kind = "directional coupler"
//! fidelity = "analytic"
//!
//! [task]
//! kind = "sweep"
//! from_um = 1.5            # the band at each point: the device's own (its wavelength ±
//! to_um = 1.6              # span/2) at 21 points by default
//! points = 21
//! # step_nm, step_z_nm, margin_um, samples, pml_cells, model_2d: the grids, as a spectrum's
//!
//! [[task.axis]]            # one or two
//! parameter = "gap"        # in its unit: µm for a length, degrees for an angle
//! from = 0.1
//! to = 0.4
//! points = 7
//! ```
//!
//! ```toml
//! [task]
//! kind = "corners"         # the nominal design first, then each corner
//!
//! [[task.corner]]
//! name = "wide, thick"
//! offsets = { width = 0.01, thickness = 0.005 }   # added to the nominal values, in their units
//!
//! [[task.corner]]
//! name = "hot"
//! temperature_k = 340.0    # every material of the stack whose model takes a temperature
//! ```

use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use super::device::{
    Compute, DeviceFigure, band_of, fidelity_name, grids_of, own_band, scene_ports, spec_of,
};
use super::{Event, task_error};
use crate::circuit::Fidelity;
use crate::device::spec::Grids;
use crate::device::{Design, Device, MaterialSpec, Spec, StackSpec};
use crate::expr::Dim;
use crate::material::catalogue;
use crate::run::{Job, Run, Stop};
use crate::units::Wavelength;
use crate::{Error, Result};

/// The most points a study takes.
const MAX_POINTS: usize = 10_000;

/// The most points an axis of a sweep takes.
const MAX_AXIS: usize = 1001;

/// The most corners a study takes.
const MAX_CORNERS: usize = 256;

/// The band's points at each point of a study when its task doesn't say.
const BAND_POINTS: usize = 21;

/// An axis of a sweep: a parameter of the design stepped evenly from one value to another.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SweepAxis {
    /// The parameter: one of the device's ([`Device::parameters`]), or one the design's
    /// expressions name.
    pub parameter: String,
    /// The first value, in the parameter's unit: µm for a length, degrees for an angle, the
    /// number otherwise.
    pub from: f64,
    /// The last.
    pub to: f64,
    /// How many values, evenly spaced from `from` to `to`.
    pub points: usize,
}

impl SweepAxis {
    /// `parameter` from `from` to `to` in `points` even steps.
    pub fn new(parameter: &str, from: f64, to: f64, points: usize) -> SweepAxis {
        SweepAxis {
            parameter: parameter.to_owned(),
            from,
            to,
            points,
        }
    }

    /// Its values: `from + (to − from) k/(points − 1)`.
    pub fn values(&self) -> Vec<f64> {
        let (a, b, n) = (self.from, self.to, self.points);
        (0..n)
            .map(|k| a + (b - a) * k as f64 / (n.max(2) - 1) as f64)
            .collect()
    }
}

/// A corner of a process: the design with some parameters moved by offsets, and its materials at
/// a temperature.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Corner {
    /// Its name, e.g. `"wide, thick"`.
    pub name: String,
    /// (parameter, offset): each added to the parameter's nominal value, in its unit (µm for a
    /// length, degrees for an angle); by name.
    pub offsets: Vec<(String, f64)>,
    /// The temperature, K, of every material of the stack whose model takes one; the materials'
    /// own when `None`.
    pub temperature_k: Option<f64>,
}

impl Corner {
    /// The corner `name` with `offsets`, (parameter, offset) pairs.
    pub fn new(name: &str, offsets: &[(&str, f64)]) -> Corner {
        let mut offsets: Vec<(String, f64)> =
            offsets.iter().map(|&(n, v)| (n.to_owned(), v)).collect();
        offsets.sort_by(|a, b| a.0.cmp(&b.0));
        Corner {
            name: name.to_owned(),
            offsets,
            temperature_k: None,
        }
    }

    /// The same corner with its materials at `kelvin`.
    #[must_use]
    pub fn at_temperature(self, kelvin: f64) -> Corner {
        Corner {
            temperature_k: Some(kelvin),
            ..self
        }
    }
}

/// What a study of a device steps through.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Study {
    /// One or two axes, the second's values fastest.
    Sweep(Vec<SweepAxis>),
    /// The nominal design, then each corner.
    Corners(Vec<Corner>),
}

impl Study {
    /// Its task's kind: `"sweep"` or `"corners"`.
    pub fn kind(&self) -> &'static str {
        match self {
            Study::Sweep(_) => "sweep",
            Study::Corners(_) => "corners",
        }
    }

    /// The designs it computes, in its points' order: `spec` with each point's parameters
    /// written as plain numbers in place of their expressions (a length's in µm), and a
    /// corner's stack at its temperature (a preset as its layers). The job
    /// [`super::from_device`] makes of each computes what the study records at that point, to
    /// the bit.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for a study that isn't one (no axis or more than two, a parameter swept
    /// twice, an axis without distinct ends or with fewer than 2 or more than 1001 points, more
    /// than 10 000 points, no corner, more than 256, a corner named twice or `"nominal"`, a
    /// corner that moves nothing); the errors of [`Spec::build`] for the nominal design; and
    /// [`Error::InvalidValue`] for a parameter neither the device nor the design has, a
    /// temperature at the analytic fidelity, or one no material of the stack takes.
    pub fn designs(&self, spec: &Spec) -> Result<Vec<Spec>> {
        let (device, design) = spec.build()?;
        let named = Named {
            device: device.as_ref(),
            design: &design,
            spec,
        };
        match self {
            Study::Sweep(axes) => {
                check_axes(axes)?;
                for a in axes {
                    named.check(&a.parameter)?;
                }
                let first = axes[0].values();
                let second = axes
                    .get(1)
                    .map_or_else(|| vec![f64::NAN], SweepAxis::values);
                let mut out = Vec::with_capacity(first.len() * second.len());
                for &u in &first {
                    for &v in &second {
                        let mut s = spec.clone();
                        set(
                            &mut s,
                            &axes[0].parameter,
                            named.literal(&axes[0].parameter, u),
                        );
                        if let Some(b) = axes.get(1) {
                            set(&mut s, &b.parameter, named.literal(&b.parameter, v));
                        }
                        out.push(s);
                    }
                }
                Ok(out)
            }
            Study::Corners(corners) => {
                check_corners(corners)?;
                let mut out = vec![spec.clone()];
                for c in corners {
                    let mut s = spec.clone();
                    for (name, offset) in &c.offsets {
                        named.check(name)?;
                        let nominal = named.nominal(name);
                        set(&mut s, name, named.literal(name, nominal + offset));
                    }
                    if let Some(t) = c.temperature_k {
                        if spec.fidelity == Fidelity::Analytic {
                            return Err(Error::invalid(
                                "study",
                                format!(
                                    "the corner \"{}\" sets a temperature, which changes the \
                                     stack's materials: the analytic models are fits and closed \
                                     forms that don't solve them; study the corners at 2D or 3D",
                                    c.name
                                ),
                            ));
                        }
                        s.stack = at_temperature(&spec.stack, t)?;
                    }
                    out.push(s);
                }
                Ok(out)
            }
        }
    }
}

/// The parameters a study of a design can move: the device's, and the others its expressions
/// name.
pub(super) struct Named<'a> {
    pub(super) device: &'a dyn Device,
    pub(super) design: &'a Design,
    pub(super) spec: &'a Spec,
}

impl Named<'_> {
    pub(super) fn is_device(&self, name: &str) -> bool {
        self.device.parameters().iter().any(|p| p.name == name)
    }

    pub(super) fn check(&self, name: &str) -> Result<()> {
        if self.is_device(name) || self.spec.parameters.iter().any(|(n, _)| n == name) {
            return Ok(());
        }
        let mut have: Vec<&str> = self
            .device
            .parameters()
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        have.extend(self.spec.parameters.iter().map(|(n, _)| n.as_str()));
        have.dedup();
        Err(Error::invalid(
            "study",
            format!(
                "a {} has no parameter {name}, and the design names none: {}",
                self.device.kind(),
                have.join(", ")
            ),
        ))
    }

    /// The dimension of a parameter the design names (not one of the device's).
    pub(super) fn dim(&self, name: &str) -> Option<Dim> {
        self.design.expressions().dim(name).flatten()
    }

    /// Its nominal value, in its unit: µm for a length, degrees for an angle.
    pub(super) fn nominal(&self, name: &str) -> f64 {
        if let Some(v) = self.design.values().get(name)
            && self.is_device(name)
        {
            return v;
        }
        let v = self.design.expressions().value(name).unwrap_or(f64::NAN);
        match self.dim(name) {
            Some(d) if d == Dim::ANGLE => v.to_degrees(),
            _ => v,
        }
    }

    /// `value` as the expression that gives it: a device parameter's a plain number in its
    /// unit, read back exactly; another's with its unit, so the expressions that use it keep
    /// theirs.
    pub(super) fn literal(&self, name: &str, value: f64) -> String {
        if self.is_device(name) {
            return format!("{value:?}");
        }
        match self.dim(name) {
            Some(d) if d == Dim::LENGTH => format!("{value:?} um"),
            Some(d) if d == Dim::ANGLE => format!("{value:?} deg"),
            _ => format!("{value:?}"),
        }
    }
}

/// Sets the parameter `name` of `spec` to `expression`, in place of its own or added.
pub(super) fn set(spec: &mut Spec, name: &str, expression: String) {
    match spec.parameters.iter_mut().find(|(n, _)| n == name) {
        Some(p) => p.1 = expression,
        None => spec.parameters.push((name.to_owned(), expression)),
    }
}

pub(super) fn check_axes(axes: &[SweepAxis]) -> Result<()> {
    if axes.is_empty() || axes.len() > 2 {
        return Err(task_error(format!(
            "a sweep steps one or two parameters ([[task.axis]]), got {}",
            axes.len()
        )));
    }
    if axes.len() == 2 && axes[0].parameter == axes[1].parameter {
        return Err(task_error(format!(
            "a sweep steps {} on one axis only",
            axes[0].parameter
        )));
    }
    for a in axes {
        if !(a.from.is_finite() && a.to.is_finite()) || a.from == a.to {
            return Err(task_error(format!(
                "the axis of {} needs distinct ends, got {} to {}",
                a.parameter, a.from, a.to
            )));
        }
        if a.points < 2 || a.points > MAX_AXIS {
            return Err(task_error(format!(
                "the axis of {} takes 2 to {MAX_AXIS} points, got {}",
                a.parameter, a.points
            )));
        }
    }
    let n: usize = axes.iter().map(|a| a.points).product();
    if n > MAX_POINTS {
        return Err(task_error(format!(
            "a study takes at most {MAX_POINTS} points, got {n}"
        )));
    }
    Ok(())
}

pub(super) fn check_corners(corners: &[Corner]) -> Result<()> {
    if corners.is_empty() || corners.len() > MAX_CORNERS {
        return Err(task_error(format!(
            "a corners study takes 1 to {MAX_CORNERS} corners ([[task.corner]]), got {}",
            corners.len()
        )));
    }
    for (k, c) in corners.iter().enumerate() {
        if c.name.trim().is_empty() || c.name == "nominal" {
            return Err(task_error(format!(
                "corner {} needs a name of its own (the first point is \"nominal\")",
                k + 1
            )));
        }
        if corners[..k].iter().any(|d| d.name == c.name) {
            return Err(task_error(format!("two corners are named \"{}\"", c.name)));
        }
        if c.offsets.is_empty() && c.temperature_k.is_none() {
            return Err(task_error(format!(
                "the corner \"{}\" moves nothing: give it offsets or a temperature",
                c.name
            )));
        }
        for (k, (name, v)) in c.offsets.iter().enumerate() {
            if !v.is_finite() {
                return Err(task_error(format!(
                    "the corner \"{}\"'s offset of {name} must be a number, got {v}",
                    c.name
                )));
            }
            if c.offsets[..k].iter().any(|(n, _)| n == name) {
                return Err(task_error(format!(
                    "the corner \"{}\" moves {name} twice",
                    c.name
                )));
            }
        }
        if let Some(t) = c.temperature_k
            && !(t.is_finite() && t > 0.0)
        {
            return Err(task_error(format!(
                "the corner \"{}\"'s temperature must be above 0 K, got {t}",
                c.name
            )));
        }
    }
    Ok(())
}

/// Whether `m`'s index model takes a temperature.
fn takes_temperature(m: &MaterialSpec) -> bool {
    let Some(entry) = catalogue::entry(&m.id) else {
        return false;
    };
    let model = match &m.model {
        None => entry.default_model().cloned(),
        Some(id) => entry.index.iter().find(|i| i.id == *id).cloned(),
    };
    model.is_some_and(|i| i.temperature.is_some())
}

/// `stack` (a preset as its layers) with every material whose model takes a temperature at
/// `kelvin`.
fn at_temperature(stack: &StackSpec, kelvin: f64) -> Result<StackSpec> {
    let mut s = stack.layers();
    let StackSpec::Layers {
        substrate,
        layers,
        cladding,
        ..
    } = &mut s
    else {
        unreachable!("a stack's layers are layers")
    };
    let mut names = Vec::new();
    let mut any = false;
    let mut warm = |m: &mut MaterialSpec| {
        if takes_temperature(m) {
            m.temperature = Some(kelvin);
            any = true;
        }
        let name = match &m.model {
            Some(model) => format!("{} ({model})", m.id),
            None => m.id.clone(),
        };
        if !names.contains(&name) {
            names.push(name);
        }
    };
    warm(substrate);
    for l in layers.iter_mut() {
        warm(&mut l.material);
        warm(&mut l.background);
    }
    warm(cladding);
    if !any {
        return Err(Error::invalid(
            "study",
            format!(
                "no material of the stack ({}) has an index model that takes a temperature, so a \
                 corner's temperature would change nothing: choose a model that does (the \
                 materials catalogue lists each model's temperatures)",
                names.join(", ")
            ),
        ));
    }
    Ok(s)
}

/// The job that studies `spec`'s device as `study` says, computed as `compute` says (its own
/// model at the design's fidelity, on the grids it sets; the band at each point the device's own
/// at 21 points when `compute` gives none), named `name`. It is checked by [`super::check`].
///
/// # Errors
///
/// [`Error::InvalidValue`] for a `compute` by FDTD (a study computes its designs by their own
/// model) or a name that isn't a job's; the errors of [`Study::designs`]; and those of
/// [`super::check`] for the job it makes (a grid the device doesn't take, a 3D solve larger than
/// the direct solver holds here at any point).
pub fn study_from_device(name: &str, spec: &Spec, compute: &Compute, study: &Study) -> Result<Job> {
    if compute.fdtd.is_some() {
        return Err(Error::invalid(
            "study",
            "a study computes each design by the device's own model at its fidelity; FDTD runs \
             one design at a time (a job of kind \"fdtd\")",
        ));
    }
    study.designs(spec)?;
    let (_, design) = spec.build()?;
    let band = compute.band.unwrap_or_else(|| {
        let (from, to) = own_band(design.values());
        (from, to, BAND_POINTS)
    });
    let float = Value::Float;
    let mut task = Table::new();
    task.insert("kind".into(), study.kind().into());
    task.insert("from_um".into(), float(band.0));
    task.insert("to_um".into(), float(band.1));
    task.insert("points".into(), Value::Integer(band.2 as i64));
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
    match study {
        Study::Sweep(axes) => {
            let axes = axes
                .iter()
                .map(|a| {
                    let mut t = Table::new();
                    t.insert("parameter".into(), Value::String(a.parameter.clone()));
                    t.insert("from".into(), float(a.from));
                    t.insert("to".into(), float(a.to));
                    t.insert("points".into(), Value::Integer(a.points as i64));
                    Value::Table(t)
                })
                .collect();
            task.insert("axis".into(), Value::Array(axes));
        }
        Study::Corners(corners) => {
            let corners = corners
                .iter()
                .map(|c| {
                    let mut t = Table::new();
                    t.insert("name".into(), Value::String(c.name.clone()));
                    if !c.offsets.is_empty() {
                        let o: Table = c
                            .offsets
                            .iter()
                            .map(|(n, v)| (n.clone(), float(*v)))
                            .collect();
                        t.insert("offsets".into(), Value::Table(o));
                    }
                    if let Some(k) = c.temperature_k {
                        t.insert("temperature_k".into(), float(k));
                    }
                    Value::Table(t)
                })
                .collect();
            task.insert("corner".into(), Value::Array(corners));
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

// ---- the task ----

/// A `"sweep"` or `"corners"` task: the band, the grids, and the axes or corners.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StudyTask {
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
    #[serde(default)]
    axis: Vec<SweepAxis>,
    #[serde(default)]
    corner: Vec<CornerTask>,
}

/// A `[[task.corner]]`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CornerTask {
    name: String,
    #[serde(default)]
    offsets: Table,
    temperature_k: Option<f64>,
}

impl StudyTask {
    fn study(&self) -> Result<Study> {
        match self.kind.as_str() {
            "sweep" => {
                if !self.corner.is_empty() {
                    return Err(task_error(
                        "a sweep takes axes ([[task.axis]]), not corners",
                    ));
                }
                Ok(Study::Sweep(self.axis.clone()))
            }
            _ => {
                if !self.axis.is_empty() {
                    return Err(task_error(
                        "a corners study takes corners ([[task.corner]]), not axes",
                    ));
                }
                Ok(Study::Corners(corners_of(&self.corner)?))
            }
        }
    }
}

/// The corners `[[task.corner]]` tables give.
///
/// # Errors
///
/// [`Error::Parse`] for an offset that isn't a number.
pub(super) fn corners_of(tables: &[CornerTask]) -> Result<Vec<Corner>> {
    tables
        .iter()
        .map(|c| {
            let offsets = c
                .offsets
                .iter()
                .map(|(n, v)| {
                    let x = v
                        .as_float()
                        .or_else(|| v.as_integer().map(|i| i as f64))
                        .ok_or_else(|| {
                            task_error(format!(
                                "the corner \"{}\"'s offset of {n} must be a number",
                                c.name
                            ))
                        })?;
                    Ok((n.as_str(), x))
                })
                .collect::<Result<Vec<_>>>()?;
            let corner = Corner::new(&c.name, &offsets);
            Ok(match c.temperature_k {
                Some(t) => corner.at_temperature(t),
                None => corner,
            })
        })
        .collect()
}

/// A study read and checked, without solving: its designs, their grids and the band.
struct Prepared {
    spec: Spec,
    study: Study,
    designs: Vec<Spec>,
    grids: Grids,
    wavelengths: Vec<f64>,
    /// The memory a point's solve may take, bytes, by estimate.
    bytes: f64,
}

/// Reads and checks a study: every point's design builds, takes the grids, and a 3D FDFD at
/// each is one the direct solver holds here.
fn prepare(job: &Job) -> Result<Prepared> {
    let spec = spec_of(job)?;
    let task: StudyTask = job
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
    let (_, design) = spec.build()?;
    spec.device_with(&grids)?;
    let size = spec.fdfd_size(&grids)?;
    let wavelengths = band_of(task.from_um, task.to_um, task.points, BAND_POINTS, &design)?;
    let study = task.study()?;
    let designs = study.designs(&spec)?;
    for (k, d) in designs.iter().enumerate() {
        let at = |e: Error| {
            task_error(format!(
                "the study's point {} ({}): {e}",
                k + 1,
                describe(&study, &designs, k)
            ))
        };
        d.build().map_err(at)?;
        if size.is_some() {
            d.fdfd_size(&grids).map_err(at)?;
        }
    }
    let bytes = match size {
        Some((_, _, unknowns)) => unknowns * 5e3,
        None if spec.fidelity == Fidelity::Analytic => 1e6,
        // a cross-section's mode solves
        None => 2e9,
    };
    Ok(Prepared {
        spec,
        study,
        designs,
        grids,
        wavelengths,
        bytes,
    })
}

/// Point `k` of `study`, in words: its corner's name, or its parameters' expressions.
fn describe(study: &Study, designs: &[Spec], k: usize) -> String {
    match study {
        Study::Corners(c) => {
            if k == 0 {
                "nominal".into()
            } else {
                c[k - 1].name.clone()
            }
        }
        Study::Sweep(axes) => axes
            .iter()
            .map(|a| {
                let e = designs[k]
                    .parameters
                    .iter()
                    .find(|(n, _)| *n == a.parameter)
                    .map_or("", |(_, e)| e.as_str());
                format!("{} = {e}", a.parameter)
            })
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// [`super::check`] for a `"sweep"` or `"corners"` job.
pub(super) fn check(job: &Job) -> Result<()> {
    prepare(job).map(|_| ())
}

/// The ports' names of a study's device, in its component's order.
fn ports(job: &Job) -> Result<Vec<String>> {
    Ok(scene_ports(job)?.into_iter().map(|p| p.name).collect())
}

/// Runs a study: the nominal design's scene and ports, [`Event::DeviceStudy`], then each point
/// side by side on rayon's threads (or a farm's workers), recorded in order.
pub(super) fn run(job: &Job, run: &mut Run, stop: &Stop, scene: Event) -> Result<()> {
    let p = prepare(job)?;
    run.record(&scene)?;
    run.record(&Event::Ports {
        ports: scene_ports(job)?,
    })?;
    let (device, design) = p.spec.build()?;
    let method = device
        .fidelities()
        .iter()
        .find(|m| m.fidelity == p.spec.fidelity)
        .cloned()
        .ok_or_else(|| {
            task_error(format!(
                "a {} has no {} model",
                p.spec.kind, p.spec.fidelity
            ))
        })?;
    let named = Named {
        device: device.as_ref(),
        design: &design,
        spec: &p.spec,
    };
    let unit = |name: &str| -> String {
        if let Some(q) = device.parameters().iter().find(|q| q.name == name) {
            return q.unit.clone();
        }
        match named.dim(name) {
            Some(d) if d == Dim::LENGTH => "µm".into(),
            Some(d) if d == Dim::ANGLE => "°".into(),
            _ => String::new(),
        }
    };
    let (axes, corners) = match &p.study {
        Study::Sweep(a) => (a.clone(), Vec::new()),
        Study::Corners(c) => (Vec::new(), c.clone()),
    };
    run.record(&Event::DeviceStudy {
        study: p.study.kind().to_owned(),
        device: device.kind().to_owned(),
        fidelity: fidelity_name(p.spec.fidelity).to_owned(),
        method: method.name,
        write_up: method.write_up,
        papers: method.papers,
        parameters: device.parameters().iter().map(|q| q.name.clone()).collect(),
        units: device.parameters().iter().map(|q| q.unit.clone()).collect(),
        axis_units: axes.iter().map(|a| unit(&a.parameter)).collect(),
        axes,
        corners,
        points: p.designs.len(),
        wavelengths_um: p.wavelengths.clone(),
        ports: ports(job)?,
    })?;
    let batch = super::in_flight(p.bytes);
    super::sweep_points(run, stop, 0..p.designs.len(), batch, |k| point(&p, k, stop))
}

/// Point `k` of a study: its design computed, or why it wasn't; `None` when the stop came first.
fn point(p: &Prepared, k: usize, stop: &Stop) -> Result<Option<Vec<Event>>> {
    let (at, corner) = match &p.study {
        Study::Sweep(axes) => {
            let n2 = axes.get(1).map_or(1, |a| a.points);
            let mut at = vec![axes[0].values()[k / n2]];
            if let Some(b) = axes.get(1) {
                at.push(b.values()[k % n2]);
            }
            (at, None)
        }
        Study::Corners(c) => (
            Vec::new(),
            Some(if k == 0 {
                "nominal".to_owned()
            } else {
                c[k - 1].name.clone()
            }),
        ),
    };
    design_point(&p.designs[k], &p.grids, &p.wavelengths, k, at, corner, stop)
}

/// Point `k` of a study of a device's designs, `spec` computed at its fidelity on `grids` over
/// `wavelengths` (µm), recorded with `at` and `corner`: its design's values, figures and
/// S-parameters, or why the model gave nothing; `None` when the stop came first.
pub(super) fn design_point(
    spec: &Spec,
    grids: &Grids,
    wavelengths: &[f64],
    k: usize,
    at: Vec<f64>,
    corner: Option<String>,
    stop: &Stop,
) -> Result<Option<Vec<Event>>> {
    let mut event = Event::DevicePoint {
        point: k,
        at,
        corner,
        values: Vec::new(),
        error: None,
        source: String::new(),
        grid: String::new(),
        figures: Vec::new(),
        effective_indices: Vec::new(),
        s: Vec::new(),
    };
    if stop.reason().is_some() {
        return Ok(None);
    }
    let solved = (|| -> Result<Option<_>> {
        let (_, design) = spec.build()?;
        let device = spec.device_with(grids)?;
        // solved here, on this point's rayon task, so that the solve's own parallel work runs
        // on the pool too: a point waiting on a thread of its own would hold a worker the solve
        // needs, and with as many points in flight as workers none would finish. A stop ends
        // the study at its next point; a solve already started finishes.
        let b = device.component(design.values(), spec.fidelity)?;
        let s = wavelengths
            .iter()
            .map(|&w| {
                let m = b.s_matrix(Wavelength::um(w)?)?;
                let n = b.component.ports().len();
                Ok((0..n)
                    .map(|q| {
                        (0..n)
                            .map(|j| {
                                let v = m[(q, j)];
                                [v.re, v.im]
                            })
                            .collect()
                    })
                    .collect())
            })
            .collect::<Result<Vec<Vec<Vec<[f64; 2]>>>>>()?;
        Ok(Some((design, b, s)))
    })();
    let Event::DevicePoint {
        values,
        error,
        source,
        grid,
        figures,
        effective_indices,
        s,
        ..
    } = &mut event
    else {
        unreachable!("the event is a point")
    };
    match solved {
        Ok(None) => return Ok(None),
        Err(e) => {
            if let Ok((_, design)) = spec.build() {
                *values = design.values().as_slice().to_vec();
            }
            *error = Some(e.to_string());
        }
        Ok(Some((design, b, matrices))) => {
            *values = design.values().as_slice().to_vec();
            *source = b.component.provenance().source;
            *grid = b
                .figures
                .first()
                .map(|g| g.grid.clone())
                .unwrap_or_default();
            *figures = b
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
                .collect();
            *effective_indices = b
                .component
                .ports()
                .iter()
                .map(|q| q.mode.as_ref().map_or(0.0, |m| m.effective_index))
                .collect();
            *s = matrices;
        }
    }
    Ok(Some(vec![event]))
}
