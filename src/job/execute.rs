//! Running a job into a run's record.

use super::check::{check, check_direct};
use super::event::Event;
use super::kind::{Holder, find, named, unknown};
use super::modes::check_export;
use super::scene::device_scene;
use super::{chip, device, results};
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
    let name = named(job)?;
    match find(Holder::Solver, name) {
        Some(kind) => kind.execute(job, run, stop),
        // refused where a kind's run starts: the direct solver checked, the start recorded
        None => solver_run(job, run, stop, name, |_, _, _| Err(unknown(name))),
    }
}

/// Whether `job` has a sweep whose points a farm's workers can solve ([`crate::farm`]): a
/// `"modes"` or `"fdfd"` job's `[task.sweep]`, or a device's study (a `[device]` with a
/// `"sweep"` or `"corners"` task).
pub fn farms(job: &Job) -> bool {
    // a [device] first, as the preview takes it: a job holding a [circuit] too is refused by
    // check, and farms as its device's task would
    let kind = if job.device().is_some() {
        device::task_kind(job).ok().flatten()
    } else {
        job.task()
            .get("kind")
            .and_then(toml::Value::as_str)
            .and_then(|name| find(Holder::of(job), name))
    };
    kind.map_or_else(|| job.task().contains_key("sweep"), |k| k.farms(job))
}

/// Runs a task alone of kind `kind` into `run`: its direct solver checked, then
/// [`Event::Started`], `body`, and [`Event::Finished`] if `body` ends well.
pub(super) fn solver_run(
    job: &Job,
    run: &mut Run,
    stop: &Stop,
    kind: &str,
    body: impl FnOnce(&Job, &mut Run, &Stop) -> Result<()>,
) -> Result<()> {
    check_direct(job, kind)?;
    run.record(&Event::Started {
        job: job.name().to_owned(),
        kind: kind.to_owned(),
    })?;
    body(job, run, stop)?;
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
    // a device's design drawn as it is edited, unchecked; its fdtd task checked first
    if job.device().is_some() {
        return match device::task_kind(job)? {
            None => device_scene(job),
            Some(kind) => kind.preview(job),
        };
    }
    // a task alone's or a chip's kind checks the job first, as check does
    let name = job.task().get("kind").and_then(|k| k.as_str());
    if let Some(kind) = name.and_then(|n| find(Holder::of(job), n)) {
        return kind.preview(job);
    }
    // a kind the registry hasn't, or none: refused as check refuses it
    check(job)?;
    Err(unknown(name.unwrap_or_default()))
}
