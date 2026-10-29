//! Running a job into a run's record.

use super::check::{check, check_direct, task_error};
use super::event::Event;
use super::modes::{check_export, modes, modes_scene, modes_task};
use super::scene::device_scene;
use super::structure::{StructureTask, draw, structure, structure_scene, tensor_stack};
use super::{chip, device, fdfd, fdtd, params, results, statics};
use crate::Result;
use crate::run::{Job, Run, Stop, StopReason};

pub(super) fn reason(stop: &Stop) -> Option<String> {
    stop.reason().map(|r| match r {
        StopReason::Timeout => "timeout".to_owned(),
        StopReason::Requested => "requested".to_owned(),
    })
}

/// Runs `job` into `run`, recording its [`Event`]s, and stops early when `stop` says so. A run
/// that fails records an [`Event::Failed`] with the error before returning it, and a run with a
/// directory gets its `results.json` ([`results`]) however it ends.
///
/// # Errors
///
/// [`Error::Parse`](crate::Error::Parse) for a task that isn't a known kind or doesn't fit it, the errors of the
/// task itself (an invalid shape, a wavelength a material has no data for), and
/// [`Error::Io`](crate::Error::Io) if the record can't be written.
pub fn execute(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let outcome = execute_job(job, run, stop);
    conclude(run, outcome)
}

/// Ends `run`'s record however its task ended: an [`Event::Failed`] with the error if
/// `outcome` is one, then the run's `results.json` ([`results`]) if it has a directory. For
/// whatever runs a task without [`execute`] (the program's optimizations).
///
/// # Errors
///
/// `outcome`'s error, else [`Error::Io`](crate::Error::Io) if the results can't be written.
pub fn conclude(run: &mut Run, outcome: Result<()>) -> Result<()> {
    if let Err(e) = &outcome {
        let seconds = run.elapsed().as_secs_f64();
        // (a record that can't be written can't take this either: the error is the task's)
        let _ = run.record(&Event::Failed {
            error: e.to_string(),
            seconds,
        });
    }
    let written = results::write_for(run);
    outcome.and(written)
}

/// The task of `job`, whichever it is: a chip's, a device's, or a kind's.
fn execute_job(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    check_export(job)?;
    if job.circuit().is_some() {
        return chip::execute(job, run, stop);
    }
    if job.device().is_some() {
        return device::execute(job, run, stop);
    }
    let kind = job
        .task()
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| task_error("needs a kind, e.g. kind = \"structure\""))?
        .to_owned();
    execute_task(job, run, stop, &kind)
}

/// Whether `job` has a sweep whose points a farm's workers can solve ([`crate::farm`]): a
/// `"modes"` or `"fdfd"` job's `[task.sweep]`, or a device's study (a `[device]` with a
/// `"sweep"` or `"corners"` task).
pub fn farms(job: &Job) -> bool {
    // a design with a compact model computes in microseconds, and names a file a worker on
    // another machine hasn't: its studies run here
    let compact = job.device().is_some_and(|d| d.contains_key("compact"));
    job.task().get("sweep").is_some()
        || (job.device().is_some()
            && !compact
            && matches!(
                job.task().get("kind").and_then(toml::Value::as_str),
                Some("sweep" | "corners" | "fit")
            ))
}

/// Runs `job`'s task of `kind` into `run`, from its start to its end.
pub(super) fn execute_task(job: &Job, run: &mut Run, stop: &Stop, kind: &str) -> Result<()> {
    let kind = kind.to_owned();
    check_direct(job, &kind)?;
    run.record(&Event::Started {
        job: job.name().to_owned(),
        kind: kind.clone(),
    })?;
    match kind.as_str() {
        "optimize" => {
            check(job)?;
            return Err(task_error(
                "an \"optimize\" job's optimizer is genoxide's, which the photonoxide program \
                 runs: photonoxide run <job> (the studio window shows it live); the library \
                 supplies its objectives and records it (photonoxide::job::Problem, Recorder)",
            ));
        }
        "structure" => structure(job, run, stop)?,
        "modes" => modes(job, run, stop)?,
        "fdfd" => fdfd::run(job, run, stop)?,
        "fdtd" => fdtd::run(job, run, stop)?,
        "static" => statics::run(job, run, stop)?,
        other => return Err(task_error(format!("unknown kind \"{other}\""))),
    }
    let seconds = run.elapsed().as_secs_f64();
    run.record(&Event::Finished {
        stopped: reason(stop),
        seconds,
    })
}

/// The scene `job`'s run records first, its structure over its window, without running it: what
/// the studio draws of a job before it runs. Checked as [`check`] does.
///
/// # Errors
///
/// The errors of [`check`].
pub fn preview(job: &Job) -> Result<Event> {
    if job.device().is_some() {
        if device::task_kind(job)?.as_deref() == Some("fdtd") {
            check(job)?;
            return fdtd::preview(job);
        }
        return device_scene(job);
    }
    check(job)?;
    match job.task().get("kind").and_then(|k| k.as_str()) {
        Some("structure") => {
            let task: StructureTask = params::task(job)?;
            structure_scene(&task, &task.structure()?)
        }
        Some("modes") => {
            let task = modes_task(job, None)?;
            let stack = tensor_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?;
            modes_scene(&task, &draw(stack, &task.rect, &task.circle, &task.ring)?)
        }
        Some("fdtd") => fdtd::preview(job),
        Some("static") => statics::preview(job),
        _ => fdfd::preview(job),
    }
}
