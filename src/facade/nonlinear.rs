//! A `"nonlinear"` job's process as plain data ([`crate::nonlinear`]): [`nonlinear`], what its run
//! records, computed without a folder.

use crate::run::{Run, Stop};
use crate::{Error, Result};

use super::JobSource;

/// A nonlinear process as a `"nonlinear"` job's run records it, each part as JSON exactly as its
/// event holds it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct NonlinearProcess {
    /// The setup ([`crate::job::NonlinearSetup`]): the waves with their wavelengths and
    /// indices, the mismatch and the coherence length, the grating, the residual mismatch and
    /// the response, the efficiency and where it comes from, the acceptance bandwidths, the
    /// phase-matched wavelength and the notes.
    pub setup: String,
    /// The phase-matching map ([`crate::job::PhaseMap`]), if the job asks for one.
    pub phase_map: Option<String>,
    /// The process along its length ([`crate::job::EfficiencyCurve`]), if it has a coupling.
    pub efficiency: Option<String>,
    /// The pairs' joint spectrum ([`crate::job::PairSpectrum`]), for SPDC and FWM.
    pub pair_spectrum: Option<String>,
}

/// What the `"nonlinear"` job `job`'s run records, computed without a folder: its setup, its
/// phase-matching map, its efficiency along the length and its pairs, each to the bit what its
/// event holds. It stops at `stop`'s deadline or the job's own `timeout_minutes`, whichever
/// comes first, and returns `None` when it did.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a job of another kind, and the errors of the job's check and run
/// ([`crate::job::check`], [`crate::job::execute`]).
pub fn nonlinear(job: &JobSource, stop: &Stop) -> Result<Option<NonlinearProcess>> {
    let job = job.load()?;
    if job.device().is_some()
        || job.circuit().is_some()
        || job.task().get("kind").and_then(|k| k.as_str()) != Some("nonlinear")
    {
        return Err(Error::invalid(
            "job",
            "its task must be of kind \"nonlinear\"",
        ));
    }
    let stop = stop.within(job.timeout());
    let mut run = Run::in_memory();
    crate::job::execute(&job, &mut run, &stop)?;
    if stop.reason().is_some() {
        return Ok(None);
    }
    let mut out = NonlinearProcess {
        setup: String::new(),
        phase_map: None,
        efficiency: None,
        pair_spectrum: None,
    };
    for line in run.lines() {
        let mut event: serde_json::Value =
            serde_json::from_str(line).map_err(|e| Error::Parse {
                what: "event".into(),
                reason: e.to_string(),
            })?;
        let part = |key: &str, event: &mut serde_json::Value| event[key].take().to_string();
        match event["type"].as_str() {
            Some("nonlinear_setup") => out.setup = part("setup", &mut event),
            Some("phase_map") => out.phase_map = Some(part("map", &mut event)),
            Some("nonlinear_efficiency") => out.efficiency = Some(part("curve", &mut event)),
            Some("pair_spectrum") => out.pair_spectrum = Some(part("spectrum", &mut event)),
            _ => {}
        }
    }
    Ok(Some(out))
}
