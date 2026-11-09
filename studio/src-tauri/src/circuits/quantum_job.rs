//! The quantum view's computations as a job: the Chip page's Quantum tab, its forms as they
//! stand, written as a `"quantum"` task beside the chip's `[circuit]`
//! (`photonoxide::job::quantum_from_chip`), which `photonoxide run` runs from the command line
//! and records as the window computes it.

use serde::Deserialize;

use super::Chip;
use crate::studio::Studio;

/// The job asked for: its name, its hard limit and its task table (without its kind).
#[derive(Clone, Debug, Deserialize)]
pub struct QuantumJobIn {
    pub name: String,
    pub timeout_minutes: f64,
    /// `wavelength_um`, `photons`, `sample`, `dip`, `budget`, `squeezed`, as
    /// `photonoxide::job`'s `"quantum"` task takes them.
    pub task: serde_json::Value,
}

/// The text of the job that computes photons through `chip` as `request` says, checked.
#[tauri::command(async)]
pub fn chip_quantum_job(
    state: tauri::State<'_, Studio>,
    chip: Chip,
    request: QuantumJobIn,
) -> Result<String, String> {
    quantum_job_text(&chip, &request, &state.workspace())
}

/// [`chip_quantum_job`]'s text.
pub(crate) fn quantum_job_text(
    chip: &Chip,
    request: &QuantumJobIn,
    root: &std::path::Path,
) -> Result<String, String> {
    let design = super::chip::design(chip, root)?;
    let task = match toml::Value::try_from(&request.task).map_err(|e| e.to_string())? {
        toml::Value::Table(t) => t,
        _ => return Err("a quantum job's task is a table".into()),
    };
    photonoxide::job::quantum_from_chip(&request.name, &design, &task, request.timeout_minutes)
        .map(|job| job.text().to_owned())
        .map_err(|e| e.to_string())
}
