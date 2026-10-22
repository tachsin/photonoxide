//! The frames a job's pictures are in (#435). The chip's, seen from above: x and y in the
//! plane, z up. A cross-section's: the axis across the guide and z up, the light along the
//! third. Which chip axis is across depends on the job:
//!
//! - a modes job with `propagation = "x"` is cut at `cut_x_um`, across y;
//! - one with `"y"`, or none (an older job), is cut at `cut_y_um`, across x;
//! - a static job is always cut at `cut_x_um`, across y.
//!
//! The design view's frame switch relabels its axes by this; no number in the job changes.
//!
//! A crystal's frame is the third: the catalogue's uniaxial materials name their optic axis in
//! the crystal's own axes ("z (the c axis)"), and an x-cut or z-cut film lays those on the chip's
//! axes differently. No job says a crystal's orientation yet (Euler angles in a job come with
//! #432), so [`crystals`] only finds the anisotropic materials a job uses and where, for the
//! design view to say which ones have a frame it can't draw until then.

use photonoxide::material::catalogue::{self, OpticalClass};
use photonoxide::run::Job;
use serde::Serialize;
use toml::{Table, Value};

/// A job's cross-section, in the chip's frame.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Section {
    /// The job's kind, "modes" or "static": a modes job's scene ends at its cut, a static
    /// job's reaches as far on either side.
    pub kind: &'static str,
    /// The chip's axis the light travels along, normal to the cut: "x" or "y".
    pub along: &'static str,
    /// The chip's axis across the guide, the cross-section's horizontal one: "x" or "y".
    pub across: &'static str,
    /// What the chip's x, y and z are called in the cross-section's frame.
    pub names: [&'static str; 3],
    /// Where the cut is along `along`, µm: `cut_x_um` or `cut_y_um`, 0 when the job doesn't say.
    pub at_um: f64,
}

impl Section {
    fn along(kind: &'static str, along: &'static str, task: &Table) -> Section {
        let across = if along == "x" { "y" } else { "x" };
        let name = |axis: &str| if axis == along { "along" } else { "across" };
        Section {
            kind,
            along,
            across,
            names: [name("x"), name("y"), "z"],
            at_um: task
                .get(if along == "x" { "cut_x_um" } else { "cut_y_um" })
                .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
                .unwrap_or(0.0),
        }
    }
}

/// The cross-section `job` is cut on: a modes or static job's; none for the others, which are
/// in the chip's frame only.
pub fn section(job: &Job) -> Option<Section> {
    if job.device().is_some() {
        return None;
    }
    let task = job.task();
    match task.get("kind").and_then(|k| k.as_str())? {
        "modes" => Some(Section::along(
            "modes",
            match task.get("propagation").and_then(|p| p.as_str()) {
                Some("x") => "x",
                _ => "y",
            },
            task,
        )),
        "static" => Some(Section::along("static", "x", task)),
        _ => None,
    }
}

/// The cross-section of the job in `text`, for the design view's frame switch: none for a job
/// that isn't cut.
#[tauri::command]
pub fn job_section(text: String) -> Result<Option<Section>, String> {
    let job = Job::parse(&text).map_err(|e| e.to_string())?;
    Ok(section(&job))
}

/// An anisotropic material a job uses: a crystal, whose axes are a frame of their own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Crystal {
    /// The catalogue's id, e.g. "linbo3".
    pub id: String,
    /// Its formula, e.g. "LiNbO₃".
    pub formula: String,
    /// Which crystal axis its optic axis is, as the catalogue says it: "z (the c axis)".
    pub optic_axis: String,
    /// Positive (n_e > n_o) or negative uniaxial.
    pub positive: bool,
    /// Where the job uses it, e.g. `stack.layer "film"` or `body #2`.
    pub places: Vec<String>,
}

/// The keys a job names a medium by: a catalogue id, or a table with its `id`.
const MEDIA: [&str; 4] = ["material", "background", "substrate", "cladding"];

/// The catalogue's anisotropic materials `job` uses, in its task or its device, by id, each
/// with the places it is used at in order.
pub fn crystals(job: &Job) -> Vec<Crystal> {
    let mut found = Vec::new();
    walk(job.task(), "", &mut found);
    if let Some(device) = job.device() {
        walk(device, "device", &mut found);
    }
    found.sort_by(|a, b| a.id.cmp(&b.id));
    for c in &mut found {
        c.places.sort();
    }
    found
}

/// `a.b`, or `b` at the top.
fn join(a: &str, b: &str) -> String {
    if a.is_empty() {
        b.to_owned()
    } else {
        format!("{a}.{b}")
    }
}

/// Looks through `table`, at `path` in the job, for media that are uniaxial crystals.
fn walk(table: &Table, path: &str, found: &mut Vec<Crystal>) {
    for (key, value) in table {
        let medium = MEDIA.contains(&key.as_str());
        // a shape's or layer's `material` is the shape's or layer's; the others are named
        let place = if key == "material" && !path.is_empty() {
            path.to_owned()
        } else {
            join(path, key)
        };
        match value {
            Value::String(id) if medium => add(id, place, found),
            Value::Table(t) if medium => {
                if let Some(id) = t.get("id").and_then(Value::as_str) {
                    add(id, place, found);
                }
            }
            Value::Table(t) => walk(t, &place, found),
            Value::Array(items) => {
                for (k, item) in items.iter().enumerate() {
                    if let Value::Table(t) = item {
                        let named = match t.get("name").and_then(Value::as_str) {
                            Some(name) => format!("{key} \"{name}\""),
                            None => format!("{key} #{}", k + 1),
                        };
                        walk(t, &join(path, &named), found);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Adds catalogue material `id`, used at `place`, when it is uniaxial.
fn add(id: &str, place: String, found: &mut Vec<Crystal>) {
    if let Some(c) = found.iter_mut().find(|c| c.id == id) {
        if !c.places.contains(&place) {
            c.places.push(place);
        }
        return;
    }
    let Some(entry) = catalogue::entry(id) else {
        return;
    };
    if let OpticalClass::Uniaxial {
        positive,
        optic_axis,
    } = entry.crystal.optical
    {
        found.push(Crystal {
            id: entry.id,
            formula: entry.formula,
            optic_axis,
            positive,
            places: vec![place],
        });
    }
}

/// The anisotropic materials the job in `text` uses, for the design view's crystal frame.
#[tauri::command]
pub fn job_crystals(text: String) -> Result<Vec<Crystal>, String> {
    let job = Job::parse(&text).map_err(|e| e.to_string())?;
    Ok(crystals(&job))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(task: &str) -> Option<Section> {
        section(&Job::parse(&format!("name = \"frames\"\n\n[task]\n{task}")).unwrap())
    }

    #[test]
    fn a_modes_job_along_x_is_across_y() {
        let s = of("kind = \"modes\"\npropagation = \"x\"\ncut_x_um = 1.0").unwrap();
        assert_eq!((s.along, s.across, s.at_um), ("x", "y", 1.0));
        assert_eq!(s.names, ["along", "across", "z"]);
    }

    #[test]
    fn a_modes_job_along_y_or_an_older_one_is_across_x() {
        for task in ["kind = \"modes\"\npropagation = \"y\"", "kind = \"modes\""] {
            let s = of(task).unwrap();
            assert_eq!((s.along, s.across, s.at_um), ("y", "x", 0.0), "{task}");
            assert_eq!(s.names, ["across", "along", "z"]);
        }
        let s = of("kind = \"modes\"\ncut_x_um = 5.0\ncut_y_um = -2").unwrap();
        assert_eq!(s.at_um, -2.0);
    }

    #[test]
    fn a_static_job_is_across_y_and_the_others_are_not_cut() {
        let s = of("kind = \"static\"").unwrap();
        assert_eq!(
            (s.kind, s.along, s.across, s.at_um),
            ("static", "x", "y", 0.0)
        );
        assert_eq!(s.names, ["along", "across", "z"]);
        assert_eq!(of("kind = \"static\"\ncut_x_um = 3").unwrap().at_um, 3.0);
        for kind in ["structure", "fdfd", "fdtd"] {
            assert_eq!(of(&format!("kind = \"{kind}\"")), None, "{kind}");
        }
    }

    #[test]
    fn the_built_in_modes_jobs_name_their_propagation() {
        let text = include_str!("../../../jobs/strip-modes.toml");
        let s = job_section(text.to_owned()).unwrap().unwrap();
        assert_eq!(s.across, "y");
    }

    #[test]
    fn a_job_names_its_crystals_and_where_it_uses_them() {
        let job =
            |task: &str| Job::parse(&format!("name = \"crystals\"\n\n[task]\n{task}")).unwrap();
        let film = job(r#"kind = "modes"

[task.stack]
substrate = "sio2"
cladding = "sio2"

[[task.stack.layer]]
name = "film"
thickness_um = 0.6
material = "linbo3"
background = { id = "linbo3", model = "zelmon-1997-congruent" }

[[task.body]]
type = "sphere"
material = "aln"

[[task.body]]
type = "box"
material = "si"
"#);
        let found = crystals(&film);
        let ids: Vec<&str> = found.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["aln", "linbo3"]);
        assert_eq!(found[0].places, ["body #1"]);
        assert_eq!(found[1].optic_axis, "z (the c axis)");
        assert!(!found[1].positive);
        assert_eq!(
            found[1].places,
            ["stack.layer \"film\"", "stack.layer \"film\".background"]
        );
        // silicon and silica are isotropic, and so is every preset
        assert!(crystals(&job("kind = \"modes\"\nstack = \"soi_220\"")).is_empty());
        let text = include_str!("../../../jobs/strip-modes.toml");
        assert!(job_crystals(text.to_owned()).unwrap().is_empty());
    }
}
