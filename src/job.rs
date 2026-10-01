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
    /// The run ended.
    Finished {
        /// Why it ended early, if it did: `"timeout"` or `"requested"`.
        stopped: Option<String>,
        /// Its duration, seconds.
        seconds: f64,
    },
}

#[derive(Deserialize)]
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

impl StructureTask {
    fn stack(&self) -> Result<LayerStack> {
        let thicknesses = || match (self.core_nm, self.bottom_oxide_um) {
            (Some(core), Some(oxide)) => Ok((Length::nm(core), Length::um(oxide))),
            _ => Err(task_error(format!(
                "stack \"{}\" needs core_nm and bottom_oxide_um",
                self.stack
            ))),
        };
        match self.stack.as_str() {
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

    fn structure(&self) -> Result<Structure> {
        let mut s = Structure::new(self.stack()?);
        for r in &self.rect {
            let shape = Shape::rect(
                Point::um(r.center_um[0], r.center_um[1]),
                Length::um(r.size_um[0]),
                Length::um(r.size_um[1]),
            )?;
            s.draw(&r.layer, shape)?;
        }
        for c in &self.circle {
            let shape = Shape::circle(
                Point::um(c.center_um[0], c.center_um[1]),
                Length::um(c.radius_um),
            )?;
            s.draw(&c.layer, shape)?;
        }
        Ok(s)
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

    #[test]
    fn bad_tasks_are_errors() {
        let root = temp("bad");
        for (text, says) in [
            (
                JOB.replace("kind = \"structure\"", "kind = \"modes\""),
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
