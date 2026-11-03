//! Components as job files (#668): every component is a design, a job with a `[device]` table
//! and no task, the same job in YAML, TOML and JSON. The built-in ones are the files of
//! `jobs/components/`, one for each device of the library, each what the device's page builds at
//! its defaults; the user's are in the workspace's `jobs/`, made on the Components rail's
//! Design page or saved from a device's page.
//!
//! - [`design_files`]: the built-in components' files and the workspace's, for the Design page;
//! - [`design_text`] and [`design_read`]: a design as its job's text in a format, and back, with
//!   what of the text isn't a design and the library's check;
//! - [`save_component`]: a design saved under its name in the workspace's `jobs/`, refused when
//!   the name is a built-in component's or another file's.

use std::path::PathBuf;

use super::*;

/// The folder of the built-in components' files, in the repository.
pub const BUILT_IN: &str = "jobs/components";

/// The device a built-in component's name stands for: its kind, when `name` is one (letters
/// in either case: on Windows, `Waveguide.yaml` is `waveguide.yaml`).
pub fn built_in_kind(name: &str) -> Option<&'static str> {
    DEVICES
        .iter()
        .find(|(_, id, ..)| id.eq_ignore_ascii_case(name))
        .map(|(kind, ..)| *kind)
}

/// A component's job file, as the Design page lists it.
#[derive(Debug, Serialize)]
pub struct DesignFile {
    /// The job's name: the component's.
    name: String,
    /// The device it is a design of.
    kind: String,
    about: String,
    /// `jobs/components/<name>.yaml` for a built-in one; its path for one of the workspace.
    path: String,
    /// One of the library's, read only: editing it saves a copy under another name.
    built_in: bool,
    format: Format,
    text: String,
}

/// The built-in components' files and the workspace's.
#[derive(Debug, Serialize)]
pub struct DesignFiles {
    built_in: Vec<DesignFile>,
    mine: Vec<DesignFile>,
}

/// The design `text` holds, in `format`, as the Design page reads it.
#[derive(Debug, PartialEq, Serialize)]
pub struct DesignRead {
    name: String,
    /// Its `description`.
    about: String,
    /// The design, when the text holds one.
    spec: Option<SpecIn>,
    /// What the text holds besides a design: a task, a circuit, a timeout. A component is a
    /// design alone; a job that computes it is saved as a job.
    beyond: Vec<String>,
    /// Why the library refuses it, if it does: its parse or its check.
    error: Option<String>,
}

/// Reads a design's text in `format`.
pub fn read_design(text: &str, format: Format) -> DesignRead {
    let mut out = DesignRead {
        name: String::new(),
        about: String::new(),
        spec: None,
        beyond: Vec::new(),
        error: None,
    };
    let job = match Job::parse_as(text, format) {
        Ok(j) => j,
        Err(e) => {
            out.error = Some(text_of(e));
            return out;
        }
    };
    out.name = job.name().to_owned();
    out.about = job.description().unwrap_or_default().to_owned();
    let top: toml::Table = match format {
        Format::Toml => toml::from_str(text).unwrap_or_default(),
        Format::Json | Format::Yaml => serde_saphyr::from_str(text).unwrap_or_default(),
    };
    for key in top.keys() {
        if !["name", "description", "device", "$schema"].contains(&key.as_str())
            && !(key == "task" && job.task().is_empty())
        {
            out.beyond.push(key.clone());
        }
    }
    let Some(table) = job.device() else {
        out.error = Some(format!(
            "{} isn't a component: it has no device table",
            job.name()
        ));
        return out;
    };
    match Spec::from_table(table) {
        Ok(spec) => out.spec = Some(SpecIn::of(&spec)),
        Err(e) => {
            out.error = Some(text_of(e));
            return out;
        }
    }
    if let Err(e) = job::check(&job) {
        out.error = Some(text_of(e));
    }
    out
}

/// A design's job text in `format`: its name, description and design, as the page writes it
/// and as it is saved; not checked (the editor shows what the library says of it).
///
/// # Errors
///
/// A message for an unknown fidelity, or a name the format can't hold.
pub fn text_in(spec: &SpecIn, name: &str, about: &str, format: Format) -> Result<String, String> {
    let yaml = job_text(&spec.spec()?, name, about)?;
    crate::studio::convert_job(yaml, Format::Yaml, format)
}

/// A design as its job's text in `format`.
#[tauri::command(async)]
pub fn design_text(
    spec: SpecIn,
    name: String,
    about: String,
    format: Format,
) -> Result<String, String> {
    text_in(&spec, &name, &about, format)
}

/// A design's text in `format`, read.
#[tauri::command(async)]
pub fn design_read(text: String, format: Format) -> DesignRead {
    read_design(&text, format)
}

fn listed(path: String, text: String, format: Format, built_in: bool) -> Option<DesignFile> {
    let read = read_design(&text, format);
    let spec = read.spec?;
    if !read.beyond.is_empty() {
        return None;
    }
    Some(DesignFile {
        name: read.name,
        kind: spec.kind,
        about: read.about,
        path,
        built_in,
        format,
        text,
    })
}

/// The components of the workspace's `jobs/` (`dir`): its designs alone, in any format, by name.
fn mine_in(dir: &std::path::Path) -> Vec<DesignFile> {
    let mut out: Vec<DesignFile> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter_map(|p| {
            let format = Format::from_path(&p)?;
            let text = std::fs::read_to_string(&p).ok()?.replace("\r\n", "\n");
            listed(p.display().to_string(), text, format, false)
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// The built-in components' files, in the library's order, and the workspace's.
#[tauri::command(async)]
pub fn design_files(state: tauri::State<'_, Studio>) -> DesignFiles {
    DesignFiles {
        built_in: crate::examples::components()
            .into_iter()
            .filter_map(|c| listed(format!("jobs/{}", c.file), c.text, Format::Yaml, true))
            .collect(),
        mine: mine_in(&state.workspace().join("jobs")),
    }
}

/// Saves the design `text` (in `format`) as `<its name>.<the format's extension>` in `dir`.
///
/// # Errors
///
/// A message when the text isn't a design alone the library takes, when its name is a built-in
/// component's, or, unless `replace`, when a file of that name exists in any format
/// (`"exists: <path>"`); on `replace`, the file in the other format goes.
pub fn save_component_in(
    dir: &std::path::Path,
    text: &str,
    format: Format,
    replace: bool,
) -> Result<PathBuf, String> {
    let read = read_design(text, format);
    if let Some(kind) = built_in_kind(&read.name) {
        return Err(format!(
            "\"{}\" is the built-in {kind}'s name ({BUILT_IN}/{}.yaml): give your component \
             another",
            read.name,
            read.name.to_ascii_lowercase()
        ));
    }
    if let Some(e) = read.error {
        return Err(e);
    }
    if !read.beyond.is_empty() {
        return Err(format!(
            "a component is a design alone; {} belong to a job that computes it: save it as \
             a job",
            read.beyond.join(", ")
        ));
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{}.{}", read.name, format.extension()));
    let others: Vec<PathBuf> = ["yaml", "yml", "toml", "json"]
        .iter()
        .map(|ext| dir.join(format!("{}.{ext}", read.name)))
        .filter(|p| p.exists())
        .collect();
    if let Some(there) = others.first().filter(|_| !replace) {
        return Err(format!("exists: {}", there.display()));
    }
    let mut text = text.replace("\r\n", "\n");
    if !text.ends_with('\n') {
        text.push('\n');
    }
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    for other in others.iter().filter(|p| **p != path) {
        std::fs::remove_file(other).map_err(|e| format!("{}: {e}", other.display()))?;
    }
    Ok(path)
}

/// Saves a component designed on the Design page in the workspace's `jobs/`.
#[tauri::command]
pub fn save_component(
    state: tauri::State<'_, Studio>,
    text: String,
    format: Format,
    replace: bool,
) -> Result<String, String> {
    save_component_in(&state.workspace().join("jobs"), &text, format, replace)
        .map(|p| p.display().to_string())
}

#[cfg(test)]
#[path = "design_tests.rs"]
mod tests;
