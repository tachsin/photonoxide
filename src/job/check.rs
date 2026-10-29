//! Checking a job without running it.

use super::modes::{
    check_export, check_tensors, check_threshold, modes_guide, modes_pml, modes_task, modes_z,
};
use super::scene::device_scene;
use super::structure::{StructureTask, draw, tensor_stack, window_z};
use super::{chip, device, fdfd, fdtd, fit, optimize, params, statics, study};
use crate::run::Job;
use crate::stack::LayerStack;
use crate::units::Wavelength;
use crate::{Error, Result};

pub(super) fn task_error(reason: impl Into<String>) -> Error {
    Error::Parse {
        what: "task".into(),
        reason: reason.into(),
    }
}

/// A window along one axis, named as the job names it (`"x_um"`): finite, from a smaller to a
/// larger value.
pub(super) fn check_window(name: &str, w: [f64; 2]) -> Result<()> {
    if w[0].is_finite() && w[1].is_finite() && w[1] > w[0] {
        Ok(())
    } else {
        Err(task_error(format!(
            "{name} must go from a smaller to a larger number, got [{}, {}]",
            w[0], w[1]
        )))
    }
}

/// The grid's step: finite and positive.
pub(super) fn check_step(step_nm: f64) -> Result<()> {
    if step_nm.is_finite() && step_nm > 0.0 {
        Ok(())
    } else {
        Err(task_error(format!(
            "step_nm must be positive, got {step_nm}"
        )))
    }
}

/// The most cells a job's grid may have along one side: what a picture may have
/// ([`crate::raster`]), and far beyond what a solve can take.
const MOST_CELLS: f64 = 10_000.0;

/// The fewest cells a job's grid may have along one side: a cross-section needs 3 nodes, and a
/// picture of one cell shows nothing.
const FEWEST_CELLS: f64 = 2.0;

/// A window's cells at the job's step: at most [`MOST_CELLS`], so that a step mistyped a
/// thousand times too small is refused instead of filling the memory, and at least
/// [`FEWEST_CELLS`], so that one a thousand times too large is refused instead of solving a
/// window of one cell.
pub(super) fn check_cells(name: &str, window: [f64; 2], step_nm: f64) -> Result<()> {
    let exact = (window[1] - window[0]) / (step_nm / 1000.0);
    let cells = exact.round();
    if cells > MOST_CELLS {
        // a number, not a line of three hundred digits
        let shown = if cells < 1e12 {
            cells.to_string()
        } else {
            format!("{cells:.1e}")
        };
        return Err(task_error(format!(
            "{name} at a step of {step_nm} nm is {shown} cells, more than {MOST_CELLS}: take a \
             larger step_nm or a smaller window"
        )));
    }
    if exact < FEWEST_CELLS {
        // to the picometre, without the subtraction's rounding
        let width = window[1] - window[0];
        let shown = if width >= 1e-3 {
            ((width * 1e6).round() / 1e6).to_string()
        } else {
            format!("{width:.1e}")
        };
        return Err(task_error(format!(
            "{name} is {shown} um across, less than {FEWEST_CELLS} cells at a step of {step_nm} \
             nm: take a smaller step_nm or a larger window"
        )));
    }
    Ok(())
}

/// An optional number the run uses as a coordinate: finite when given.
pub(super) fn check_finite(name: &str, v: Option<f64>) -> Result<()> {
    match v {
        Some(v) if !v.is_finite() => Err(task_error(format!("{name} must be a number, got {v}"))),
        _ => Ok(()),
    }
}

/// Every material of `stack` (its substrate, its cladding, each layer's and what surrounds it)
/// must have data at each of `wavelengths_um`; for a sweep, its two ends, a material's validity
/// being one range.
pub(super) fn check_materials(stack: &LayerStack, wavelengths_um: &[f64]) -> Result<()> {
    let mut materials = vec![stack.substrate(), stack.cladding()];
    for layer in stack.layers() {
        materials.push(&layer.material);
        materials.push(&layer.background);
    }
    for &w in wavelengths_um {
        let w = Wavelength::um(w)?;
        for m in &materials {
            m.permittivity(w)?;
        }
    }
    Ok(())
}

/// A sweep's range: finite ends that differ, and at least 2 points.
pub(super) fn check_sweep(from: f64, to: f64, points: usize) -> Result<()> {
    if points < 2 || !(from.is_finite() && to.is_finite()) {
        return Err(task_error("a sweep needs from, to and at least 2 points"));
    }
    if from == to {
        return Err(task_error(format!(
            "a sweep's from and to must differ, got {from} for both"
        )));
    }
    Ok(())
}

/// Checks `job` without running it: its kind is known, its task fits that kind, the stack is
/// known, the shapes are valid, the layer is in the stack, the wavelength is valid, the windows
/// run from smaller to larger numbers, the step is positive, a sweep has distinct ends, at least
/// 2 points and a parameter the kind sweeps, the coordinates are numbers, and every material of
/// the stack has data at the wavelength and at both ends of a wavelength sweep; for an
/// `"fdfd"` job, the polarization, that the PMLs leave room, and each port's place: a known
/// side, a column inside the window and clear of the PMLs, a window of rows on the grid. No
/// window, the default height (1 µm below and above the layer) included, may be more than
/// 10 000 cells of the step across or fewer than 2; a modes job asks for at least one mode, and
/// a structure job's side view is cut inside its window. What only a solve finds (a port
/// whose column guides no mode, modes that don't converge) is left to the run.
///
/// # Errors
///
/// The errors [`execute`](crate::job::execute) would return for those, before recording anything.
pub fn check(job: &Job) -> Result<()> {
    check_inner(job)?;
    check_export(job)
}

/// The job's direct solver: one that is registered and available, and asked of a job that has
/// a direct solve to give it. What [`check`] and the run both refuse, in the same words.
pub(super) fn check_direct(job: &Job, kind: &str) -> Result<()> {
    crate::backend::direct(job.direct())?;
    if kind != "fdfd" && *job.direct() != crate::backend::Choice::Auto {
        return Err(task_error(format!(
            "[solver] direct = \"{}\" is for fdfd jobs: a {kind} job has no direct solve to \
             give a backend",
            job.direct()
        )));
    }
    Ok(())
}

fn check_inner(job: &Job) -> Result<()> {
    if job.circuit().is_some() {
        check_direct(job, "chip")?;
        return chip::check(job);
    }
    if job.device().is_some() {
        return match device::task_kind(job)?.as_deref() {
            None => device_scene(job).map(|_| ()),
            Some("fdtd") => {
                check_direct(job, "fdtd")?;
                fdtd::check(job)
            }
            Some(kind @ ("sweep" | "corners")) => {
                check_direct(job, kind)?;
                device_scene(job)?;
                study::check(job)
            }
            Some("fit") => {
                check_direct(job, "fit")?;
                device_scene(job)?;
                fit::check(job)
            }
            Some("optimize") => {
                check_direct(job, "optimize")?;
                device_scene(job)?;
                optimize::check(job)
            }
            Some(_) => {
                check_direct(job, "spectrum")?;
                device_scene(job)?;
                device::check_spectrum(job)
            }
        };
    }
    let kind = job
        .task()
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| task_error("needs a kind, e.g. kind = \"structure\""))?;
    check_direct(job, kind)?;
    let mut wavelengths = Vec::new();
    let (s, layer, wavelength_um) = match kind {
        "structure" => {
            let task: StructureTask = params::task(job)?;
            task.validate()?;
            let s = task.structure()?;
            window_z(task.z_um, &task.layer, &s, task.step_nm)?;
            (s, task.layer, task.wavelength_um)
        }
        "modes" => {
            let task = modes_task(job, None)?;
            task.validate()?;
            let stack = tensor_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?;
            let s = draw(stack, &task.rect, &task.circle, &task.ring)?;
            let z = modes_z(&task, &s)?;
            modes_guide(&task, &s, z)?;
            modes_pml(&task, &s, z)?;
            if let Some(sw) = task.sweep.iter().find(|sw| sw.parameter == "wavelength") {
                wavelengths.extend([sw.from, sw.to]);
            }
            if task.modes_above.is_some() {
                // (each material's data at these wavelengths is checked first, as below)
                check_materials(s.stack(), &[task.wavelength_um])?;
                check_materials(s.stack(), &wavelengths)?;
            }
            let mut at = vec![task.wavelength_um];
            at.extend(&wavelengths);
            check_threshold(&task, &s, z, &at)?;
            for &w in &at {
                check_tensors(s.stack(), task.along()?, w)?;
            }
            // a parameter's sweep: its shapes at both ends must be shapes
            if let Some(sw) = task
                .sweep
                .iter()
                .find(|sw| task.parameters.contains(&sw.parameter))
            {
                for end in [sw.from, sw.to] {
                    let at = modes_task(job, Some((&sw.parameter, end)))?;
                    let stack = tensor_stack(&at.stack, at.core_nm, at.bottom_oxide_um)?;
                    draw(stack, &at.rect, &at.circle, &at.ring)
                        .and_then(|swept| modes_pml(&task, &swept, z))
                        .map_err(|e| {
                            task_error(format!("at the sweep's {} = {end}: {e}", sw.parameter))
                        })?;
                }
            }
            (s, task.layer, task.wavelength_um)
        }
        "fdfd" => return fdfd::check(job),
        "fdtd" => return fdtd::check(job),
        "static" => return statics::check(job),
        "optimize" if optimize::crosstalk::corrects(job) => return optimize::check(job),
        "optimize" => {
            return Err(task_error(
                "an \"optimize\" task optimizes a [device], a [circuit], or a heater array's \
                 [task.crosstalk]",
            ));
        }
        other => return Err(task_error(format!("unknown kind \"{other}\""))),
    };
    s.stack()
        .layer(&layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {layer}")))?;
    wavelengths.insert(0, wavelength_um);
    check_materials(s.stack(), &wavelengths)
}
