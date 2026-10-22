//! What each entry of a job's text draws, for the Custom device's click-to-edit: a shape picked
//! in the 3D view or from above is found among these by its outline, and the text jumps to the
//! entry's place in it.
//!
//! The library lists a scene's shapes layer by layer, not by the entry that drew them, so each
//! entry's are found by drawing the task's entries one more at a time, as a structure job, and
//! taking what each adds. A component placed by `connect` meets an earlier one, so every prefix
//! draws. An entry's place in the text is the file's own: a shape on a line (`shapes:`, #467) is
//! `task.shapes[j]`, though the library reads it into `task.rect` after the rectangles written
//! as tables.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;
use serde::de::IgnoredAny;
use toml::{Table, Value};

use photonoxide::job::{Event, SceneShape};
use photonoxide::run::{Format, Job};

/// A step of a path into a job's text: a table's key or a list's index (from 0).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Key {
    Name(String),
    Index(usize),
}

/// An entry of the task that draws shapes.
#[derive(Debug, Serialize)]
pub struct Drawn {
    /// Where it is in the text, as the file has it: `["task", "rect", 0]`, or
    /// `["task", "shapes", 3]` for a shape on a line.
    pub path: Vec<Key>,
    /// The list the library reads it into (`rect`, `circle`, `ring`, `component`, `path`,
    /// `shape`, `body`) and its place there, from 0: the form's shapes are known by these.
    pub list: String,
    pub index: usize,
    /// Its shapes, as the scene has them.
    pub shapes: Vec<SceneShape>,
}

/// Every entry of a job's task that draws shapes, in the order they draw, and the stack's layers.
#[derive(Debug, Default, Serialize)]
pub struct Drawing {
    pub entries: Vec<Drawn>,
    /// The layers' names, bottom to top.
    pub layers: Vec<String>,
}

/// The task's lists that draw shapes.
const DRAWN: [&str; 7] = [
    "rect",
    "circle",
    "ring",
    "component",
    "path",
    "shape",
    "body",
];

/// The kinds a shape on a line is read into a list of its own; any other is a `shape` of that
/// type (`photonoxide::job::short`).
const LINES: [&str; 4] = ["rect", "circle", "ring", "path"];

/// The task's keys a structure job takes that place or draw shapes, besides the lists.
const KEPT: [&str; 7] = [
    "stack",
    "core_nm",
    "bottom_oxide_um",
    "wavelength_um",
    "layer",
    "parameters",
    "cell",
];

/// What each entry of the job in `text` draws. Empty for a device's or a chip's job; the entries
/// up to the first that can't be drawn when one can't (the job isn't valid).
///
/// # Errors
///
/// A message when the text isn't a job.
pub fn drawing(text: &str, format: Format) -> Result<Drawing, String> {
    let job = Job::parse_as(text, format).map_err(|e| e.to_string())?;
    if job.device().is_some() || job.circuit().is_some() {
        return Ok(Drawing::default());
    }
    let task = job.task();
    let places = places(text, format);
    let mut prefix = structure(task);
    let mut out = Drawing::default();
    let mut before = match scene(&prefix) {
        Ok((shapes, layers)) => {
            out.layers = layers;
            shapes
        }
        Err(_) => return Ok(out),
    };
    for list in DRAWN {
        let Some(Value::Array(items)) = task.get(list) else {
            continue;
        };
        for (index, item) in items.iter().enumerate() {
            match prefix.get_mut(list) {
                Some(Value::Array(a)) => a.push(item.clone()),
                _ => {
                    prefix.insert(list.into(), Value::Array(vec![item.clone()]));
                }
            }
            let Ok((now, _)) = scene(&prefix) else {
                return Ok(out);
            };
            let shapes = added(&before, &now);
            before = now;
            let path = places
                .get(&(list, index))
                .cloned()
                .unwrap_or_else(|| place(list, index));
            out.entries.push(Drawn {
                path,
                list: list.into(),
                index,
                shapes,
            });
        }
    }
    Ok(out)
}

/// [`drawing`], for the Custom device's click-to-edit.
#[tauri::command]
pub fn job_drawing(text: String, format: Format) -> Result<Drawing, String> {
    drawing(&text, format)
}

/// `task.<list>[index]`.
fn place(list: &str, index: usize) -> Vec<Key> {
    vec![
        Key::Name("task".into()),
        Key::Name(list.into()),
        Key::Index(index),
    ]
}

/// The task as a structure job with none of its shapes: its stack, layer, parameters and cells,
/// in a small window (a scene's outlines don't depend on it).
fn structure(task: &Table) -> Table {
    let mut t: Table = KEPT
        .iter()
        .filter_map(|k| task.get(*k).map(|v| ((*k).to_owned(), v.clone())))
        .collect();
    t.insert("kind".into(), "structure".into());
    t.entry("wavelength_um").or_insert(Value::Float(1.55));
    if !t.contains_key("layer") {
        // a job that names no layer of its own (an fdtd job's): any its shapes are on
        let layer = DRAWN
            .iter()
            .filter_map(|l| task.get(*l).and_then(Value::as_array))
            .flatten()
            .find_map(|s| s.get("layer").and_then(Value::as_str))
            .unwrap_or("Si");
        t.insert("layer".into(), layer.into());
    }
    let window = Value::Array(vec![Value::Float(-1.0), Value::Float(1.0)]);
    t.insert("x_um".into(), window.clone());
    t.insert("y_um".into(), window);
    t.insert("step_nm".into(), Value::Float(100.0));
    t
}

/// The scene of a structure job's `task`: its shapes and its layers' names.
fn scene(task: &Table) -> Result<(Vec<SceneShape>, Vec<String>), String> {
    let mut file = Table::new();
    file.insert("name".into(), "drawn".into());
    file.insert("task".into(), Value::Table(task.clone()));
    let job = Job::parse(&toml::to_string(&file).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    match photonoxide::job::preview(&job).map_err(|e| e.to_string())? {
        Event::Scene { shapes, layers, .. } => {
            Ok((shapes, layers.into_iter().map(|l| l.name).collect()))
        }
        _ => Err("no scene".into()),
    }
}

/// The shapes of `now` that aren't in `before`, each of `before` matched once.
fn added(before: &[SceneShape], now: &[SceneShape]) -> Vec<SceneShape> {
    let mut used = vec![false; before.len()];
    now.iter()
        .filter(|s| {
            let found = before
                .iter()
                .enumerate()
                .find(|(k, b)| !used[*k] && b == s)
                .map(|(k, _)| k);
            match found {
                Some(k) => {
                    used[k] = true;
                    false
                }
                None => true,
            }
        })
        .cloned()
        .collect()
}

/// A job file's task as the file has it: the lists that draw, and the shapes on a line.
#[derive(Deserialize, Default)]
struct RawFile {
    #[serde(default)]
    task: RawTask,
}

#[derive(Deserialize, Default)]
struct RawTask {
    #[serde(default)]
    rect: Vec<IgnoredAny>,
    #[serde(default)]
    circle: Vec<IgnoredAny>,
    #[serde(default)]
    ring: Vec<IgnoredAny>,
    #[serde(default)]
    component: Vec<IgnoredAny>,
    #[serde(default)]
    path: Vec<IgnoredAny>,
    #[serde(default)]
    shape: Vec<IgnoredAny>,
    #[serde(default)]
    body: Vec<IgnoredAny>,
    #[serde(default)]
    shapes: Vec<BTreeMap<String, IgnoredAny>>,
}

/// Where each entry the library reads is in the text: `(list, index)` to its path there, for the
/// shapes on a line, which the library puts after each list's tables in the order they come.
fn places(text: &str, format: Format) -> BTreeMap<(&'static str, usize), Vec<Key>> {
    let raw: Option<RawFile> = match format {
        Format::Toml => toml::from_str(text).ok(),
        Format::Json => serde_json::from_str(text).ok(),
        Format::Yaml => serde_saphyr::from_str(text).ok(),
    };
    let mut out = BTreeMap::new();
    let Some(raw) = raw else {
        return out;
    };
    let t = &raw.task;
    let mut counts: BTreeMap<&'static str, usize> = [
        ("rect", t.rect.len()),
        ("circle", t.circle.len()),
        ("ring", t.ring.len()),
        ("component", t.component.len()),
        ("path", t.path.len()),
        ("shape", t.shape.len()),
        ("body", t.body.len()),
    ]
    .into_iter()
    .collect();
    for (j, item) in t.shapes.iter().enumerate() {
        let Some(kind) = item.keys().next().filter(|_| item.len() == 1) else {
            continue;
        };
        let list = LINES
            .iter()
            .find(|l| *l == kind)
            .copied()
            .unwrap_or("shape");
        let n = counts.entry(list).or_insert(0);
        out.insert(
            (list, *n),
            vec![
                Key::Name("task".into()),
                Key::Name("shapes".into()),
                Key::Index(j),
            ],
        );
        *n += 1;
    }
    out
}

#[cfg(test)]
mod tests;
