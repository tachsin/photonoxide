//! The jobs `photonoxide run` knows, and the events they record.
//!
//! A job file's `[task]` table names its kind. The kinds so far:
//!
//! - `"structure"`: a layer stack with shapes drawn on it, recorded as pictures of its
//!   permittivity, seen from above and on a vertical cut.
//!
//! ```toml
//! name = "strip"
//! timeout_minutes = 5
//!
//! [task]
//! kind = "structure"
//! stack = "soi_220"          # or "soi" / "nitride" with core_nm and bottom_oxide_um
//! wavelength_um = 1.55
//! layer = "Si"               # the layer seen from above
//! x_um = [-3.0, 3.0]
//! y_um = [-2.0, 2.0]
//! step_nm = 20.0
//!
//! [[task.rect]]
//! layer = "Si"
//! center_um = [0.0, 0.0]
//! size_um = [6.0, 0.5]
//!
//! [[task.circle]]
//! layer = "Si"
//! center_um = [1.5, 1.2]
//! radius_um = 0.4
//! ```
//!
//! - `"modes"`: the guided modes of a waveguide's cross-section, the same stack and shapes cut at
//!   y = `cut_y_um` (the modes travel along y), by the full-vector solver
//!   ([`crate::mode::vector`]) on a uniform grid of `step_nm`; recorded as a picture of the
//!   cross-section and one of each mode's |E|², and optionally swept over the wavelength or a
//!   rectangle's width, each point recorded as it is solved.
//!
//! ```toml
//! name = "strip-modes"
//! timeout_minutes = 10
//!
//! [task]
//! kind = "modes"
//! stack = "soi_220"
//! wavelength_um = 1.55
//! layer = "Si"               # the window's height defaults to 1 µm around it
//! x_um = [-1.2, 1.2]
//! step_nm = 20.0
//! modes = 2                  # how many, from the highest effective index
//!
//! [[task.rect]]
//! layer = "Si"
//! center_um = [0.0, 0.0]
//! size_um = [0.5, 10.0]      # 500 nm wide, along y
//!
//! [task.sweep]
//! parameter = "wavelength"   # or "width": rect `rect`'s size along x
//! from = 1.5
//! to = 1.6
//! points = 11
//! ```

use serde::{Deserialize, Serialize};

use crate::geometry::{Point, Shape};
use crate::raster::Raster;
use crate::run::{Job, Run, Stop, StopReason};
use crate::stack::{LayerStack, Structure};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// An event of a run's record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Event {
    /// The run started.
    Started {
        /// The job's name.
        job: String,
        /// The task's kind, e.g. `"structure"`.
        kind: String,
    },
    /// A picture of the permittivity (real part).
    Permittivity {
        /// What the picture shows, e.g. `"top view of layer Si"`.
        view: String,
        /// The axis labels, e.g. `["x", "y"]`.
        axes: [String; 2],
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The picture.
        raster: Raster,
    },
    /// A guided mode of a cross-section.
    Mode {
        /// What it is, e.g. `"mode 1 of 2"`.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The effective index, real and imaginary parts.
        effective_index: [f64; 2],
        /// The share of the transverse magnetic field in H_y (see
        /// [`crate::mode::vector::VectorMode::te_fraction`]): near 1 for a TE-like mode.
        te_fraction: f64,
        /// |E|² at the cells' centres, its largest value 1; x across, z up.
        intensity: Raster,
    },
    /// A point of a sweep: the modes' effective indices at one value of the parameter.
    SweepPoint {
        /// `"wavelength"` (µm) or `"width"` (µm).
        parameter: String,
        /// The parameter's value.
        value: f64,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The modes' effective indices, highest first, real and imaginary parts.
        effective_indices: Vec<[f64; 2]>,
        /// Their TE fractions, in the same order.
        te_fractions: Vec<f64>,
    },
    /// The run ended.
    Finished {
        /// Why it ended early, if it did: `"timeout"` or `"requested"`.
        stopped: Option<String>,
        /// Its duration, seconds.
        seconds: f64,
    },
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RectSpec {
    layer: String,
    center_um: [f64; 2],
    size_um: [f64; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CircleSpec {
    layer: String,
    center_um: [f64; 2],
    radius_um: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StructureTask {
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    stack: String,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    wavelength_um: f64,
    layer: String,
    x_um: [f64; 2],
    y_um: [f64; 2],
    z_um: Option<[f64; 2]>,
    side_y_um: Option<f64>,
    step_nm: f64,
    #[serde(default)]
    rect: Vec<RectSpec>,
    #[serde(default)]
    circle: Vec<CircleSpec>,
}

fn task_error(reason: impl Into<String>) -> Error {
    Error::Parse {
        what: "task".into(),
        reason: reason.into(),
    }
}

/// A named stack: `soi_220`, or `soi` / `nitride` with their core and bottom oxide.
fn named_stack(
    stack: &str,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
) -> Result<LayerStack> {
    let thicknesses = || match (core_nm, bottom_oxide_um) {
        (Some(core), Some(oxide)) => Ok((Length::nm(core), Length::um(oxide))),
        _ => Err(task_error(format!(
            "stack \"{stack}\" needs core_nm and bottom_oxide_um"
        ))),
    };
    match stack {
        "soi_220" => Ok(LayerStack::soi_220()),
        "soi" => {
            let (core, oxide) = thicknesses()?;
            LayerStack::soi(core, oxide)
        }
        "nitride" => {
            let (core, oxide) = thicknesses()?;
            LayerStack::nitride(core, oxide)
        }
        other => Err(task_error(format!(
            "unknown stack \"{other}\": soi_220, soi or nitride"
        ))),
    }
}

/// The stack with the shapes drawn on it.
fn draw(stack: LayerStack, rects: &[RectSpec], circles: &[CircleSpec]) -> Result<Structure> {
    let mut s = Structure::new(stack);
    for r in rects {
        let shape = Shape::rect(
            Point::um(r.center_um[0], r.center_um[1]),
            Length::um(r.size_um[0]),
            Length::um(r.size_um[1]),
        )?;
        s.draw(&r.layer, shape)?;
    }
    for c in circles {
        let shape = Shape::circle(
            Point::um(c.center_um[0], c.center_um[1]),
            Length::um(c.radius_um),
        )?;
        s.draw(&c.layer, shape)?;
    }
    Ok(s)
}

impl StructureTask {
    fn structure(&self) -> Result<Structure> {
        draw(
            named_stack(&self.stack, self.core_nm, self.bottom_oxide_um)?,
            &self.rect,
            &self.circle,
        )
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SweepSpec {
    parameter: String,
    from: f64,
    to: f64,
    points: usize,
    /// For a width sweep: which rectangle, in the order given (0 by default).
    rect: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModesTask {
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    stack: String,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    wavelength_um: f64,
    layer: String,
    x_um: [f64; 2],
    z_um: Option<[f64; 2]>,
    cut_y_um: Option<f64>,
    step_nm: f64,
    modes: Option<usize>,
    #[serde(default)]
    rect: Vec<RectSpec>,
    #[serde(default)]
    circle: Vec<CircleSpec>,
    sweep: Option<SweepSpec>,
}

/// The cross-section of `s` cut at y over `x` × `z` (µm) on a grid of about `step`, with every
/// shape's edge on the cut and every layer interface on a node, so a sweep over a width varies
/// it smoothly; each cell the permittivity at its centre at `wavelength`.
#[allow(clippy::too_many_arguments)]
fn cross_section(
    s: &Structure,
    rects: &[RectSpec],
    circles: &[CircleSpec],
    cut_y: f64,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
    wavelength: Wavelength,
) -> Result<crate::mode::vector::CrossSection> {
    // every material the cut can meet must know this wavelength
    let stack = s.stack();
    let mut materials = vec![stack.substrate(), stack.cladding()];
    for layer in stack.layers() {
        materials.push(&layer.material);
        materials.push(&layer.background);
    }
    for m in materials {
        m.permittivity(wavelength)?;
    }
    // the edges the cut crosses, and the layers' interfaces
    let mut x_edges = Vec::new();
    for r in rects {
        if (cut_y - r.center_um[1]).abs() < r.size_um[1] / 2.0 {
            x_edges.extend([
                r.center_um[0] - r.size_um[0] / 2.0,
                r.center_um[0] + r.size_um[0] / 2.0,
            ]);
        }
    }
    for c in circles {
        let d = cut_y - c.center_um[1];
        if d.abs() < c.radius_um {
            let half = (c.radius_um * c.radius_um - d * d).sqrt();
            x_edges.extend([c.center_um[0] - half, c.center_um[0] + half]);
        }
    }
    let mut z_edges = vec![0.0];
    let mut top = 0.0;
    for layer in stack.layers() {
        top += layer.thickness.to_um();
        z_edges.push(top);
    }
    let grid = |a: f64, b: f64, fixed: &[f64]| {
        crate::mode::vector::graded_nodes(a, b, (a, b), step, 0.0, step, fixed)
    };
    let (xs, zs) = (grid(x[0], x[1], &x_edges), grid(z[0], z[1], &z_edges));
    let mut cells = Vec::with_capacity((xs.len() - 1) * (zs.len() - 1));
    for i in 0..xs.len() - 1 {
        let xc = 0.5 * (xs[i] + xs[i + 1]);
        for j in 0..zs.len() - 1 {
            let zc = 0.5 * (zs[j] + zs[j + 1]);
            let eps = s
                .material_at(Point::um(xc, cut_y), Length::um(zc))
                .permittivity(wavelength)
                .unwrap_or(num_complex::Complex64::new(f64::NAN, 0.0));
            cells.push(crate::mode::vector::Permittivity::isotropic(eps));
        }
    }
    crate::mode::vector::CrossSection::new(xs, zs, cells)
}

/// A mode's |E|² as a picture of about `step` per pixel over `x` × `z`, each pixel the cell it
/// falls in (the grid needn't be uniform), its largest value 1.
fn intensity(
    fields: &crate::mode::fields::Fields,
    cs: &crate::mode::vector::CrossSection,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
) -> Raster {
    let (nx, ny) = (
        ((x[1] - x[0]) / step).round().max(1.0) as usize,
        ((z[1] - z[0]) / step).round().max(1.0) as usize,
    );
    // the cell holding a coordinate, along one axis
    let cell =
        |nodes: &[f64], v: f64| nodes.partition_point(|&n| n <= v).clamp(1, nodes.len() - 1) - 1;
    let mut values = vec![0.0f64; nx * ny];
    for j in 0..ny {
        let zv = z[0] + (j as f64 + 0.5) * (z[1] - z[0]) / ny as f64;
        let cj = cell(cs.y(), zv);
        for i in 0..nx {
            let xv = x[0] + (i as f64 + 0.5) * (x[1] - x[0]) / nx as f64;
            values[j * nx + i] = fields
                .e(cell(cs.x(), xv), cj)
                .iter()
                .map(|c| c.norm_sqr())
                .sum();
        }
    }
    let peak = values
        .iter()
        .copied()
        .fold(0.0, f64::max)
        .max(f64::MIN_POSITIVE);
    Raster {
        nx,
        ny,
        x0: x[0],
        x1: x[1],
        y0: z[0],
        y1: z[1],
        values: values.iter().map(|v| (v / peak) as f32).collect(),
    }
}

fn reason(stop: &Stop) -> Option<String> {
    stop.reason().map(|r| match r {
        StopReason::Timeout => "timeout".to_owned(),
        StopReason::Requested => "requested".to_owned(),
    })
}

/// Runs `job` into `run`, recording its [`Event`]s, and stops early when `stop` says so.
///
/// # Errors
///
/// [`Error::Parse`] for a task that isn't a known kind or doesn't fit it, the errors of the
/// task itself (an invalid shape, a wavelength a material has no data for), and
/// [`Error::Io`] if the record can't be written.
pub fn execute(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let kind = job
        .task()
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| task_error("needs a kind, e.g. kind = \"structure\""))?
        .to_owned();
    run.record(&Event::Started {
        job: job.name().to_owned(),
        kind: kind.clone(),
    })?;
    match kind.as_str() {
        "structure" => structure(job, run, stop)?,
        "modes" => modes(job, run, stop)?,
        other => return Err(task_error(format!("unknown kind \"{other}\""))),
    }
    let seconds = run.elapsed().as_secs_f64();
    run.record(&Event::Finished {
        stopped: reason(stop),
        seconds,
    })
}

fn structure(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let task: StructureTask = job
        .task()
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(e.to_string()))?;
    let s = task.structure()?;
    let lam = Wavelength::um(task.wavelength_um)?;
    let step = Length::nm(task.step_nm);
    let x = (Length::um(task.x_um[0]), Length::um(task.x_um[1]));
    let y = (Length::um(task.y_um[0]), Length::um(task.y_um[1]));
    let top = s.top_view(&task.layer, x, y, step, lam)?;
    run.record(&Event::Permittivity {
        view: format!("top view of layer {}", task.layer),
        axes: ["x".into(), "y".into()],
        wavelength_um: lam.to_um(),
        raster: top,
    })?;
    if stop.reason().is_some() {
        return Ok(());
    }
    let (_, bottom, top_z) = s
        .stack()
        .layer(&task.layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {}", task.layer)))?;
    let z = match task.z_um {
        Some([a, b]) => (Length::um(a), Length::um(b)),
        None => (bottom - Length::um(1.0), top_z + Length::um(1.0)),
    };
    let side_y = Length::um(task.side_y_um.unwrap_or(0.0));
    let side = s.side_view(side_y, x, z, step, lam)?;
    run.record(&Event::Permittivity {
        view: format!("side view at y = {} um", side_y.to_um()),
        axes: ["x".into(), "z".into()],
        wavelength_um: lam.to_um(),
        raster: side,
    })
}

fn modes(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let task: ModesTask = job
        .task()
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(e.to_string()))?;
    let stack = || named_stack(&task.stack, task.core_nm, task.bottom_oxide_um);
    let count = task.modes.unwrap_or(2).max(1);
    let lam = Wavelength::um(task.wavelength_um)?;
    let step = task.step_nm / 1000.0;
    let cut_y = task.cut_y_um.unwrap_or(0.0);
    let s = draw(stack()?, &task.rect, &task.circle)?;
    let z = match task.z_um {
        Some(z) => z,
        None => {
            let (_, bottom, top) = s
                .stack()
                .layer(&task.layer)
                .ok_or_else(|| task_error(format!("the stack has no layer {}", task.layer)))?;
            [bottom.to_um() - 1.0, top.to_um() + 1.0]
        }
    };
    // the cross-section, as the structure job shows a side view
    run.record(&Event::Permittivity {
        view: format!("cross-section at y = {cut_y} um"),
        axes: ["x".into(), "z".into()],
        wavelength_um: lam.to_um(),
        raster: s.side_view(
            Length::um(cut_y),
            (Length::um(task.x_um[0]), Length::um(task.x_um[1])),
            (Length::um(z[0]), Length::um(z[1])),
            Length::um(step),
            lam,
        )?,
    })?;
    let cs = cross_section(&s, &task.rect, &task.circle, cut_y, task.x_um, z, step, lam)?;
    let found = crate::mode::vector::modes(&cs, lam, count, None)?;
    let mut sorted: Vec<_> = found.iter().collect();
    sorted.sort_by(|a, b| b.effective_index().re.total_cmp(&a.effective_index().re));
    for (k, m) in sorted.iter().enumerate() {
        if stop.reason().is_some() {
            return Ok(());
        }
        let n = m.effective_index();
        run.record(&Event::Mode {
            label: format!("mode {} of {}", k + 1, sorted.len()),
            wavelength_um: lam.to_um(),
            effective_index: [n.re, n.im],
            te_fraction: m.te_fraction(),
            intensity: intensity(&m.fields(&cs)?, &cs, task.x_um, z, step),
        })?;
    }
    let Some(sweep) = &task.sweep else {
        return Ok(());
    };
    if sweep.points < 2 || !(sweep.from.is_finite() && sweep.to.is_finite()) {
        return Err(task_error("a sweep needs from, to and at least 2 points"));
    }
    for k in 0..sweep.points {
        if stop.reason().is_some() {
            return Ok(());
        }
        let value = sweep.from + (sweep.to - sweep.from) * k as f64 / (sweep.points - 1) as f64;
        let (cs, w) = match sweep.parameter.as_str() {
            "wavelength" => {
                let w = Wavelength::um(value)?;
                (
                    cross_section(&s, &task.rect, &task.circle, cut_y, task.x_um, z, step, w)?,
                    w,
                )
            }
            "width" => {
                let index = sweep.rect.unwrap_or(0);
                let mut rects: Vec<RectSpec> = task.rect.iter().map(RectSpec::clone).collect();
                let r = rects.get_mut(index).ok_or_else(|| {
                    task_error(format!("the width sweep's rect {index} isn't there"))
                })?;
                r.size_um[0] = value;
                let swept = draw(stack()?, &rects, &task.circle)?;
                (
                    cross_section(&swept, &rects, &task.circle, cut_y, task.x_um, z, step, lam)?,
                    lam,
                )
            }
            other => {
                return Err(task_error(format!(
                    "unknown sweep parameter \"{other}\": wavelength or width"
                )));
            }
        };
        let mut found = crate::mode::vector::modes(&cs, w, count, None)?;
        found.sort_by(|a, b| b.effective_index().re.total_cmp(&a.effective_index().re));
        run.record(&Event::SweepPoint {
            parameter: sweep.parameter.clone(),
            value,
            wavelength_um: w.to_um(),
            effective_indices: found
                .iter()
                .map(|m| [m.effective_index().re, m.effective_index().im])
                .collect(),
            te_fractions: found
                .iter()
                .map(crate::mode::vector::VectorMode::te_fraction)
                .collect(),
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::replay;
    use std::path::PathBuf;

    const JOB: &str = r#"
name = "strip"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 1.0]
y_um = [-1.0, 1.0]
step_nm = 50.0

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [2.0, 0.5]
"#;

    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn temp(tag: &str) -> TempDir {
        let dir =
            std::env::temp_dir().join(format!("photonoxide-job-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        TempDir(dir)
    }

    #[test]
    fn a_structure_job_records_a_top_and_a_side_view() {
        let root = temp("views");
        let job = Job::parse(JOB).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        assert_eq!(events.len(), 4);
        assert!(matches!(&events[0], Event::Started { kind, .. } if kind == "structure"));
        let Event::Permittivity { raster, axes, .. } = &events[1] else {
            panic!("{:?}", events[1])
        };
        assert_eq!(axes, &["x".to_owned(), "y".to_owned()]);
        assert_eq!((raster.nx, raster.ny), (40, 40));
        let Event::Permittivity { raster, axes, .. } = &events[2] else {
            panic!("{:?}", events[2])
        };
        assert_eq!(axes[1], "z");
        // the default cut spans 1 um below and above the 220 nm layer
        assert!((raster.y1 - raster.y0 - 2.22).abs() < 1e-9);
        assert!(matches!(&events[3], Event::Finished { stopped: None, .. }));
    }

    #[test]
    fn a_stopped_job_says_why() {
        let root = temp("stopped");
        let job = Job::parse(JOB).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        let stop = Stop::new(None);
        stop.request();
        execute(&job, &mut run, &stop).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // the top view, then it stops before the side view
        assert_eq!(events.len(), 3);
        assert!(matches!(
            &events[2],
            Event::Finished { stopped: Some(r), .. } if r == "requested"
        ));
    }

    const MODES: &str = r#"
name = "strip-modes"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 1.0]
step_nm = 25.0
modes = 2

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [0.5, 10.0]
"#;

    #[test]
    fn a_modes_job_records_the_strips_modes() {
        let root = temp("modes");
        let job = Job::parse(MODES).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // started, the cross-section, two modes, finished
        assert_eq!(events.len(), 5, "{events:?}");
        let Event::Mode {
            effective_index,
            te_fraction,
            intensity,
            ..
        } = &events[2]
        else {
            panic!("{:?}", events[2])
        };
        // the 500 x 220 nm strip's TE-like mode, first; its field peaks in the core
        assert!(
            effective_index[0] > 2.3 && effective_index[0] < 2.6,
            "{effective_index:?}"
        );
        assert!(*te_fraction > 0.9);
        let (lo, hi) = intensity.range();
        assert!(lo >= 0.0 && (hi - 1.0).abs() < 1e-6);
        let Event::Mode {
            effective_index: second,
            te_fraction,
            ..
        } = &events[3]
        else {
            panic!("{:?}", events[3])
        };
        assert!(second[0] < effective_index[0] && *te_fraction < 0.1);
    }

    #[test]
    fn a_width_sweep_records_rising_indices() {
        let root = temp("sweep");
        let text = format!(
            "{MODES}\n[task.sweep]\nparameter = \"width\"\nfrom = 0.4\nto = 0.6\npoints = 3\n"
        );
        let job = Job::parse(&text.replace("modes = 2", "modes = 1")).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        let points: Vec<f64> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepPoint {
                    effective_indices, ..
                } => Some(effective_indices[0][0]),
                _ => None,
            })
            .collect();
        assert_eq!(points.len(), 3);
        assert!(points.windows(2).all(|w| w[1] > w[0]), "{points:?}");
    }

    #[test]
    fn bad_tasks_are_errors() {
        let root = temp("bad");
        for (text, says) in [
            (
                JOB.replace("kind = \"structure\"", "kind = \"fdtd\""),
                "unknown kind",
            ),
            (
                JOB.replace("stack = \"soi_220\"", "stack = \"glass\""),
                "unknown stack",
            ),
            (
                JOB.replace("stack = \"soi_220\"", "stack = \"soi\""),
                "core_nm",
            ),
            (JOB.replace("step_nm", "step"), "step"),
            (
                JOB.replace("layer = \"Si\"\ncenter", "layer = \"M1\"\ncenter"),
                "M1",
            ),
        ] {
            let job = Job::parse(&text).unwrap();
            let mut run = Run::create(&root.0, &job).unwrap();
            let e = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert!(e.to_string().contains(says), "{says}: {e}");
        }
    }
}
