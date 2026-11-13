//! What a run's folder can export: the tables its record holds and the result files beside it,
//! for the studio's Export dialog and `photonoxide export --list`.

use std::path::Path;

use serde::Serialize;

/// A table a run can export ([`photonoxide::export::tables`]), for the Export dialog.
#[derive(Clone, Debug, Serialize)]
pub struct ExportTable {
    /// Its name, e.g. `"s_parameters"`.
    pub name: String,
    /// Its file, e.g. `"s_parameters.csv"`.
    pub file: String,
    /// Its rows.
    pub rows: usize,
    /// Its file's size, bytes.
    pub bytes: usize,
}

/// A file of a run's results beside its record: a table (`.csv`) or fields (`.npz`).
#[derive(Clone, Debug, Serialize)]
pub struct ExportFile {
    /// Its name in the run's folder.
    pub name: String,
    /// Its size, bytes.
    pub bytes: u64,
}

/// What the run in a folder can export, and what it has written.
#[derive(Clone, Debug, Serialize)]
pub struct RunExports {
    /// The tables its record holds.
    pub tables: Vec<ExportTable>,
    /// The `.csv` and `.npz` files in its folder, by name.
    pub files: Vec<ExportFile>,
}

/// The tables the run in `dir` holds and the result files in its folder.
pub fn run_exports(dir: &Path) -> Result<RunExports, String> {
    let tables = photonoxide::export::tables::of_run(dir)
        .map_err(|e| e.to_string())?
        .iter()
        .map(|t| ExportTable {
            name: t.table.name().to_owned(),
            file: t.table.file(),
            rows: t.rows.len(),
            bytes: t.text().len(),
        })
        .collect();
    let mut files: Vec<ExportFile> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let kept = name.ends_with(".csv") || name.ends_with(".npz");
            kept.then(|| ExportFile {
                name,
                bytes: e.metadata().map(|m| m.len()).unwrap_or(0),
            })
        })
        .collect();
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(RunExports { tables, files })
}
