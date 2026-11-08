//! An `"eme"` job's expansion as plain data ([`crate::eme`]): [`eme`], what its run records,
//! computed without a folder.

use crate::Complex64 as c64;
use crate::convergence::Verdict;
use crate::job::Event;
use crate::run::{Run, Stop};
use crate::{Error, Result};

use super::JobSource;

/// An `"eme"` job's expansion as its run records it: the solver's settings, the sections, the
/// S-parameters at each wavelength, the convergence studies, the estimated errors and a
/// period's Bloch modes, each number to the bit what its event holds.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct EmeSolution {
    /// `"eme"` for a 2D job, `"eme 3d"` for one on a plane (`[task.plane]`).
    pub module: String,
    /// A 2D job's modes and its tapers' steps at its own settings; a 3D job's plane in cells,
    /// across and up.
    pub cells: [usize; 2],
    /// A 2D job's longest taper step, µm (0 without a taper); a 3D job's larger cell side.
    pub step_um: f64,
    /// The unknowns: a 2D job's modes times its cross-sections, a 3D job's plane's.
    pub unknowns: usize,
    /// The solver's settings in words, each `(name, value)`: the polarization or the plane, the
    /// window, the modes, the sections, the PML, the steps, a period, the interpolation.
    pub settings: Vec<(String, String)>,
    /// The sections in order along the structure.
    pub sections: Vec<EmeSectionInfo>,
    /// The S-parameters at each wavelength, in the order solved (or interpolated).
    pub points: Vec<EmePoint>,
    /// Each convergence study (`[task.convergence]`), in the order recorded.
    pub studies: Vec<EmeStudy>,
    /// Each estimated error: the adaptive steps' at a wavelength, the interpolation's over the
    /// sweep.
    pub errors: Vec<EmeError>,
    /// A `[task.period]`'s Bloch modes at each wavelength solved.
    pub bloch: Vec<EmeBloch>,
}

/// A section of the structure ([`crate::job::EmeSection`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct EmeSectionInfo {
    /// Its name: the job's own, or a cut plane's, `"cut 3"`.
    pub name: String,
    /// `"straight"`, `"taper"`, `"bend"` or `"cut"`.
    pub kind: String,
    /// Where it starts along the structure, µm.
    pub start_um: f64,
    /// Its length, µm; a bend's along the arc at the window's middle.
    pub length_um: f64,
    /// The cross-sections it is solved in at the job's own settings.
    pub steps: usize,
    /// A bend's radius to the window's middle, µm.
    pub radius_um: Option<f64>,
    /// A bend's angle, degrees.
    pub angle_deg: Option<f64>,
    /// How many times it is repeated: a period's count, 1 otherwise.
    pub repeats: usize,
}

/// The S-parameters at one wavelength, between the guided modes at the two ends.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct EmePoint {
    /// The wavelength, µm.
    pub wavelength_um: f64,
    /// The ports' modes, the first end's then the last's, e.g. `"1 TE0"`, `"2 m1"`.
    pub ports: Vec<String>,
    /// Each port mode's effective index (its real part).
    pub effective_index: Vec<f64>,
    /// The S-matrix, `(n, n)` in C order: S_qp, from port mode p into q, at `q * n + p`.
    pub s: Vec<c64>,
}

/// A convergence study of the expansion: its modes, its steps or a bend's layers refined until
/// the quantity changed by less than the tolerance ([`crate::convergence::ToTolerance`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct EmeStudy {
    /// What was studied, e.g. `"|S21|^2 of the fundamental modes at 1.55 um, the modes"`.
    pub label: String,
    /// The wavelength, µm.
    pub wavelength_um: f64,
    /// Each grid of the study, coarsest first, as recorded: a spacing, µm, or one over a count
    /// (1/modes for a study of the modes).
    pub grids: Vec<f64>,
    /// The quantity on each grid.
    pub values: Vec<f64>,
    /// The value reported: the finest grid's, or Richardson's limit where the order is known.
    pub value: f64,
    /// The tolerance it was driven to.
    pub tolerance: Option<f64>,
    /// Its estimated error; `None` where the grids don't show it.
    pub error: Option<f64>,
    /// The observed order, from three grids converging monotonically.
    pub order: Option<f64>,
    /// Whether it reached the tolerance.
    pub converged: bool,
    /// The verdict in words.
    pub reason: String,
}

/// An estimated error of the expansion, in words and as a number.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct EmeError {
    /// The wavelength it is at, µm (an interpolation's: the job's own).
    pub wavelength_um: f64,
    /// What it measures.
    pub measure: String,
    /// The error.
    pub error: f64,
}

/// A period's Bloch modes at one wavelength: the least decaying first, at most 4.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct EmeBloch {
    /// The wavelength, µm.
    pub wavelength_um: f64,
    /// Each mode's K/k₀ in the reduced zone, Im K/k₀ its decay.
    pub effective_index: Vec<c64>,
}

/// What the `"eme"` job `job`'s run records, computed without a folder: the solver's settings,
/// the sections, the S-parameters between the guided modes at the two ends at each wavelength,
/// the convergence studies, the estimated errors and a period's Bloch modes, each to the bit
/// what its event holds. Every form of the job runs: 2D sections, 3D strips on a plane, a
/// `[device]`'s or the job's own drawing cut into planes, bends, a `[task.period]`, a sweep and
/// `[task.interpolate]`. It stops at `stop`'s deadline or the job's own `timeout_minutes`,
/// whichever comes first, and returns `None` when it did.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a job of another kind, and the errors of the job's check and run
/// ([`crate::job::check`], [`crate::job::execute`]).
pub fn eme(job: &JobSource, stop: &Stop) -> Result<Option<EmeSolution>> {
    let job = job.load()?;
    if job.circuit().is_some() || job.task().get("kind").and_then(|k| k.as_str()) != Some("eme") {
        return Err(Error::invalid("job", "its task must be of kind \"eme\""));
    }
    let stop = stop.within(job.timeout());
    let mut run = Run::in_memory();
    crate::job::execute(&job, &mut run, &stop)?;
    if stop.reason().is_some() {
        return Ok(None);
    }
    let mut out = EmeSolution {
        module: String::new(),
        cells: [0, 0],
        step_um: 0.0,
        unknowns: 0,
        settings: Vec::new(),
        sections: Vec::new(),
        points: Vec::new(),
        studies: Vec::new(),
        errors: Vec::new(),
        bloch: Vec::new(),
    };
    for line in run.lines() {
        let event: Event = serde_json::from_str(line).map_err(|e| Error::Parse {
            what: "event".into(),
            reason: e.to_string(),
        })?;
        match event {
            Event::Solver {
                module,
                cells,
                step_um,
                unknowns,
                details,
            } => {
                out.module = module;
                out.cells = cells;
                out.step_um = step_um;
                out.unknowns = unknowns;
                out.settings = details.into_iter().map(|[k, v]| (k, v)).collect();
            }
            Event::EmeSections { sections } => {
                out.sections = sections
                    .into_iter()
                    .map(|s| EmeSectionInfo {
                        name: s.name,
                        kind: s.kind,
                        start_um: s.start_um,
                        length_um: s.length_um,
                        steps: s.steps,
                        radius_um: s.radius_um,
                        angle_deg: s.angle_deg,
                        repeats: s.repeats,
                    })
                    .collect();
            }
            Event::SParameters {
                wavelength_um,
                ports,
                effective_indices,
                s,
            } => out.points.push(EmePoint {
                wavelength_um,
                ports,
                effective_index: effective_indices,
                s: s.iter()
                    .flatten()
                    .map(|&[re, im]| c64::new(re, im))
                    .collect(),
            }),
            Event::GridConvergence {
                label,
                wavelength_um,
                check,
            } => out.studies.push(EmeStudy {
                label,
                wavelength_um,
                grids: check.steps_um,
                values: check.values,
                value: check.value,
                tolerance: check.tolerance,
                error: check.error,
                order: check.order,
                converged: check.verdict == Verdict::Converged,
                reason: check.reason,
            }),
            Event::SolveError {
                value,
                measure,
                error,
                ..
            } => out.errors.push(EmeError {
                wavelength_um: value,
                measure,
                error,
            }),
            Event::SweepPoint {
                wavelength_um,
                effective_indices,
                ..
            } => out.bloch.push(EmeBloch {
                wavelength_um,
                effective_index: effective_indices
                    .iter()
                    .map(|&[re, im]| c64::new(re, im))
                    .collect(),
            }),
            _ => {}
        }
    }
    Ok(Some(out))
}
