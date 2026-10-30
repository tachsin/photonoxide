//! A device's compact model fitted from its own runs: a job's `[device]` at a source fidelity
//! and a `[task]` of kind `"fit"`.
//!
//! The samples are the design with the covered parameters set to each sample's values (written
//! as plain numbers in place of their expressions), each computed at the design's fidelity as a
//! `"spectrum"` task computes it, side by side on rayon's threads or a farm's workers: the
//! samples the model is fitted to first, then the held-out ones. The run records the nominal
//! design's scene and ports, [`Event::CompactPlan`], an [`Event::DevicePoint`] per sample in
//! order, then the model fitted to them with its errors, stability and passivity
//! ([`Event::CompactFit`]); a run with a folder also writes the model's file there,
//! `compact.json` ([`crate::device::Fitted`]), which a design names to have the compact
//! fidelity.
//!
//! ```yaml
//! name: ring-compact
//!
//! device:
//!   kind: all-pass ring
//!   fidelity: analytic        # the source: the runs the model is fitted to
//!
//! task:
//!   kind: fit
//!   from_um: 1.54             # the band, the device's own at 101 points by default
//!   to_um: 1.56
//!   points: 81
//!   sampling: grid            # or latin hypercube, with count and seed
//!   held_out: 8               # samples kept out of the fit, on a Latin hypercube of their own
//!   poles: 12
//!   degree: 2                 # a polynomial in the parameters; or interpolation: piecewise linear
//!   # step_nm, step_z_nm, margin_um, samples, pml_cells, model_2d: the grids, as a spectrum's
//!
//!   axis:                     # one to four of the device's parameters
//!     - parameter: gap
//!       from: 0.15
//!       to: 0.25
//!       points: 5             # a grid's; a Latin hypercube takes count
//! ```

use schemars::{JsonSchema, SchemaGenerator};
use serde::Deserialize;
use toml::{Table, Value};

use super::device::{
    Compute, DEVICE_EXPORTS, band_of, computed, farmed, fidelity_name, grids_of, own_band,
    scene_ports, spec_of,
};
use super::kind::TaskKind;
use super::study::{design_point, set};
use super::{Event, task_error};
use crate::circuit::{Fidelity, SMatrix, Spectrum};
use crate::compact::{Interpolation, Sample};
use crate::device::spec::Grids;
use crate::device::{FitAxis, FitPlan, Fitted, Sampling, Spec};
use crate::run::{Job, Run, Stop};
use crate::units::Wavelength;
use crate::{Complex64, Error, Result};

/// The band's points at each sample when the task doesn't say.
const BAND_POINTS: usize = 101;

/// A `[[task.axis]]` of a fit.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AxisTask {
    parameter: String,
    from: f64,
    to: f64,
    points: Option<usize>,
}

/// A `"fit"` task.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct FitTask {
    #[allow(dead_code)] // looked up by the registry (job::kind); kept for serde and the schema
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
    sampling: Option<String>,
    count: Option<usize>,
    seed: Option<u64>,
    held_out: Option<usize>,
    poles: Option<usize>,
    degree: Option<usize>,
    interpolation: Option<String>,
    #[serde(default)]
    axis: Vec<AxisTask>,
    /// What the run writes beside its record: its tables as CSV ([`crate::export`]).
    #[allow(dead_code)] // read by super::export_of, checked by serde
    export: Option<super::ExportSpec>,
}

impl FitTask {
    /// The plan the task gives.
    fn plan(&self) -> Result<FitPlan> {
        let sampling = match self.sampling.as_deref() {
            None | Some("grid") => {
                if self.count.is_some() || self.seed.is_some() {
                    return Err(task_error(
                        "a grid's samples are its axes' points: count and seed are a Latin \
                         hypercube's (sampling = \"latin hypercube\")",
                    ));
                }
                Sampling::Grid
            }
            Some("latin hypercube") => Sampling::LatinHypercube {
                points: self.count.ok_or_else(|| {
                    task_error("a Latin hypercube needs its count of samples, e.g. count = 25")
                })?,
                seed: self.seed.unwrap_or(1),
            },
            Some(other) => {
                return Err(task_error(format!(
                    "sampling \"{other}\": \"grid\" or \"latin hypercube\""
                )));
            }
        };
        let axes = self
            .axis
            .iter()
            .map(|a| {
                let points = match (sampling, a.points) {
                    (Sampling::Grid, Some(n)) => n,
                    (Sampling::Grid, None) => {
                        return Err(task_error(format!(
                            "a grid's axis of {} needs its points",
                            a.parameter
                        )));
                    }
                    (_, Some(_)) => {
                        return Err(task_error(format!(
                            "a Latin hypercube's samples are its count: the axis of {} takes no \
                             points",
                            a.parameter
                        )));
                    }
                    (_, None) => 0,
                };
                Ok(FitAxis::new(&a.parameter, a.from, a.to, points))
            })
            .collect::<Result<Vec<_>>>()?;
        let interpolation = match (self.interpolation.as_deref(), self.degree) {
            (None | Some("polynomial"), d) => Interpolation::Polynomial {
                degree: d.unwrap_or(2),
            },
            (Some("piecewise linear"), None) => Interpolation::PiecewiseLinear,
            (Some("piecewise linear"), Some(_)) => {
                return Err(task_error(
                    "a piecewise-linear model has no degree: give one or the other",
                ));
            }
            (Some(other), _) => {
                return Err(task_error(format!(
                    "interpolation \"{other}\": \"polynomial\" (with its degree) or \"piecewise \
                     linear\""
                )));
            }
        };
        let mut plan = FitPlan::new(axes)
            .sampled(sampling)
            .interpolated(interpolation);
        if let Some(h) = self.held_out {
            plan = plan.holding_out(h);
        }
        if let Some(p) = self.poles {
            plan = plan.with_poles(p);
        }
        Ok(plan)
    }
}

/// A fit read and checked, without solving: its plan, its samples' designs, the grids and the
/// band.
struct Prepared {
    spec: Spec,
    plan: FitPlan,
    /// The samples' values, then the held-out ones'.
    at: Vec<Vec<f64>>,
    designs: Vec<Spec>,
    grids: Grids,
    wavelengths: Vec<f64>,
    bytes: f64,
}

/// Reads and checks a fit: its plan is one for the device at its fidelity, every sample's
/// design builds and takes the grids (a 3D FDFD at each one a 3D solve holds here), and
/// the parameters the model doesn't cover keep their values at every sample.
fn prepare(job: &Job) -> Result<Prepared> {
    let spec = spec_of(job)?;
    let task: FitTask = job
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
    let (device, design) = spec.build()?;
    let plan = task.plan()?;
    plan.check(device.as_ref(), spec.fidelity)?;
    spec.device_with(&grids)?;
    let size = spec.fdfd_size(&grids)?;
    let wavelengths = band_of(task.from_um, task.to_um, task.points, BAND_POINTS, &design)?;
    if wavelengths.len() < 2 {
        return Err(task_error(
            "a compact model is fitted over a band: give it two wavelengths or more",
        ));
    }
    let mut at = plan.samples();
    at.extend(plan.held_out_samples());
    let mut designs = Vec::with_capacity(at.len());
    let covered: Vec<&str> = plan.axes.iter().map(|a| a.parameter.as_str()).collect();
    for (k, values) in at.iter().enumerate() {
        let mut s = spec.clone();
        for (name, v) in covered.iter().zip(values) {
            set(&mut s, name, format!("{v:?}"));
        }
        let fail = |e: Error| {
            task_error(format!(
                "the fit's sample {} ({}): {e}",
                k + 1,
                covered
                    .iter()
                    .zip(values)
                    .map(|(n, v)| format!("{n} = {v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        };
        let (_, d) = s.build().map_err(fail)?;
        for p in device.parameters() {
            if covered.contains(&p.name.as_str()) {
                continue;
            }
            let (now, then) = (d.values().get(&p.name), design.values().get(&p.name));
            if now != then {
                return Err(fail(Error::invalid(
                    "compact model",
                    format!(
                        "{} follows the covered parameters through its expression: the model \
                         would hold only along that line. Cover {} too, or give it a number",
                        p.name, p.name
                    ),
                )));
            }
        }
        if size.is_some() {
            s.fdfd_size(&grids).map_err(fail)?;
        }
        designs.push(s);
    }
    let bytes = match size {
        Some((_, _, unknowns)) => unknowns * 5e3,
        None if spec.fidelity == Fidelity::Analytic => 1e6,
        None => 2e9,
    };
    Ok(Prepared {
        spec,
        plan,
        at,
        designs,
        grids,
        wavelengths,
        bytes,
    })
}

/// [`super::check`] for a `"fit"` job.
pub(super) fn check(job: &Job) -> Result<()> {
    prepare(job).map(|_| ())
}

/// The `"fit"` kind beside a `[device]`: a compact model fitted to the device's own runs.
pub(super) struct FitKind;

impl TaskKind for FitKind {
    fn name(&self) -> &'static str {
        "fit"
    }

    fn schema(&self, g: &mut SchemaGenerator) -> serde_json::Value {
        super::schema::task::<FitTask>(g)
    }

    fn check(&self, job: &Job) -> Result<()> {
        super::device_scene(job)?;
        check(job)
    }

    fn preview(&self, job: &Job) -> Result<Event> {
        super::device_scene(job)
    }

    fn execute(&self, job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
        computed(job, run, stop, self::run)
    }

    fn farms(&self, job: &Job) -> bool {
        farmed(job)
    }

    fn exports(&self, _: &Job) -> (&'static str, &'static [&'static str]) {
        DEVICE_EXPORTS
    }
}

/// The job that fits a compact model of `spec`'s device at its fidelity as `plan` says,
/// computed as `compute` says (its grids; the band the device's own at 101 points when it gives
/// none), named `name`. It is checked by [`super::check`].
///
/// # Errors
///
/// [`Error::InvalidValue`] for a `compute` by FDTD, the errors of [`FitPlan::check`], and those
/// of [`super::check`] for the job it makes.
pub fn fit_from_device(name: &str, spec: &Spec, compute: &Compute, plan: &FitPlan) -> Result<Job> {
    if compute.fdtd.is_some() {
        return Err(Error::invalid(
            "compact model",
            "a compact model is fitted to the device's own model at its fidelity, not to FDTD",
        ));
    }
    let (device, design) = spec.build()?;
    plan.check(device.as_ref(), spec.fidelity)?;
    let band = compute.band.unwrap_or_else(|| {
        let (from, to) = own_band(design.values());
        (from, to, BAND_POINTS)
    });
    let float = Value::Float;
    let int = |n: usize| Value::Integer(n as i64);
    let mut task = Table::new();
    task.insert("kind".into(), "fit".into());
    task.insert("from_um".into(), float(band.0));
    task.insert("to_um".into(), float(band.1));
    task.insert("points".into(), int(band.2));
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
            task.insert(key.into(), int(n));
        }
    }
    if compute.fdfd_2d {
        task.insert("model_2d".into(), "fdfd".into());
    }
    match plan.sampling {
        Sampling::Grid => {
            task.insert("sampling".into(), "grid".into());
        }
        Sampling::LatinHypercube { points, seed } => {
            task.insert("sampling".into(), "latin hypercube".into());
            task.insert("count".into(), int(points));
            task.insert(
                "seed".into(),
                Value::Integer(i64::try_from(seed).map_err(|_| {
                    Error::invalid("compact model", "a seed is at most 2^63 − 1 in a job file")
                })?),
            );
        }
    }
    task.insert("held_out".into(), int(plan.held_out));
    task.insert("poles".into(), int(plan.poles));
    match plan.interpolation {
        Interpolation::Polynomial { degree } => {
            task.insert("degree".into(), int(degree));
        }
        Interpolation::PiecewiseLinear => {
            task.insert("interpolation".into(), "piecewise linear".into());
        }
    }
    let axes = plan
        .axes
        .iter()
        .map(|a| {
            let mut t = Table::new();
            t.insert("parameter".into(), Value::String(a.parameter.clone()));
            t.insert("from".into(), float(a.from));
            t.insert("to".into(), float(a.to));
            if plan.sampling == Sampling::Grid {
                t.insert("points".into(), int(a.points));
            }
            Value::Table(t)
        })
        .collect();
    task.insert("axis".into(), Value::Array(axes));
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

/// Runs a fit: the nominal design's scene and ports, [`Event::CompactPlan`], each sample side by
/// side (recorded in order), then the model fitted to them, [`Event::CompactFit`], also written
/// to the run's folder as `compact.json`.
pub(super) fn run(job: &Job, run: &mut Run, stop: &Stop, scene: Event) -> Result<()> {
    let p = prepare(job)?;
    run.record(&scene)?;
    let ports = scene_ports(job)?;
    run.record(&Event::Ports {
        ports: ports.clone(),
    })?;
    let (device, _) = p.spec.build()?;
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
    let unit = |name: &str| {
        device
            .parameters()
            .iter()
            .find(|q| q.name == name)
            .map_or_else(String::new, |q| q.unit.clone())
    };
    let samples = p.plan.samples().len();
    run.record(&Event::CompactPlan {
        device: device.kind().to_owned(),
        fidelity: fidelity_name(p.spec.fidelity).to_owned(),
        method: method.name,
        write_up: method.write_up,
        papers: method.papers,
        parameters: device.parameters().iter().map(|q| q.name.clone()).collect(),
        units: device.parameters().iter().map(|q| q.unit.clone()).collect(),
        axes: p.plan.axes.iter().map(|a| a.parameter.clone()).collect(),
        axis_units: p.plan.axes.iter().map(|a| unit(&a.parameter)).collect(),
        ranges: p.plan.axes.iter().map(|a| [a.from, a.to]).collect(),
        sampling: p.plan.describe_sampling(),
        samples,
        held_out: p.plan.held_out,
        poles: p.plan.poles,
        interpolation: match p.plan.interpolation {
            Interpolation::Polynomial { degree } => format!("polynomial of degree {degree}"),
            Interpolation::PiecewiseLinear => "piecewise linear".into(),
        },
        wavelengths_um: p.wavelengths.clone(),
        ports: ports.iter().map(|q| q.name.clone()).collect(),
    })?;
    run.start_tap();
    let batch = super::in_flight(p.bytes);
    let swept = super::sweep_points(run, stop, 0..p.designs.len(), batch, |k| {
        design_point(
            &p.designs[k],
            &p.grids,
            &p.wavelengths,
            k,
            p.at[k].clone(),
            None,
            stop,
        )
    });
    let lines = run.take_tap();
    swept?;
    if stop.reason().is_some() {
        return Ok(());
    }
    // the samples as recorded, whoever solved them: the same numbers to the bit
    let wavelengths = p
        .wavelengths
        .iter()
        .map(|&w| Wavelength::um(w))
        .collect::<Result<Vec<_>>>()?;
    let names: Vec<String> = ports.iter().map(|q| q.name.clone()).collect();
    let mut points = Vec::with_capacity(p.designs.len());
    let mut grid = String::new();
    for line in &lines {
        let Ok(Event::DevicePoint {
            point,
            error,
            grid: g,
            s,
            ..
        }) = serde_json::from_str::<Event>(line)
        else {
            continue;
        };
        if let Some(e) = error {
            return Err(task_error(format!(
                "the fit's sample {} ({}) gave nothing: {e}",
                point + 1,
                p.plan
                    .axes
                    .iter()
                    .zip(&p.at[point])
                    .map(|(a, v)| format!("{} = {v}", a.parameter))
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        if grid.is_empty() {
            grid = g;
        }
        let matrices = s
            .iter()
            .map(|m| SMatrix::from_fn(m.len(), |q, j| Complex64::new(m[q][j][0], m[q][j][1])))
            .collect();
        points.push(Sample {
            values: p.at[point].clone(),
            spectrum: Spectrum::new(names.clone(), wavelengths.clone(), matrices)?,
        });
    }
    if points.len() != p.designs.len() {
        return Err(task_error(format!(
            "the fit has {} of its {} samples",
            points.len(),
            p.designs.len()
        )));
    }
    let held = points.split_off(samples);
    let fitted = Fitted::fit(&p.spec, &p.plan, &grid, &points, &held)?;
    if !run.dir().as_os_str().is_empty() {
        fitted.write(&run.dir().join("compact.json"))?;
    }
    run.record(&Event::CompactFit {
        fitted: Box::new(fitted),
    })
}
