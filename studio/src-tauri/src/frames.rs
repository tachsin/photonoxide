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
//! the crystal's own axes ("z (the c axis)"), and the orientation a job gives a crystal in its
//! `[task.stack]` (x-, y- or z-cut, or z-x-z Euler angles: `photonoxide::material::Orientation`)
//! lays those on the chip's. [`crystals`] finds the anisotropic materials a job uses, where, and
//! how each is oriented there, for the design view to name the chip's axes by the crystal's,
//! draw the crystal's axes on the shapes it fills, and look along its c axis.

use photonoxide::material::Orientation;
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
    let job = crate::studio::parse_job(&text).map_err(|e| e.to_string())?;
    Ok(section(&job))
}

/// An anisotropic material a job uses: a crystal, whose axes are a frame of their own.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Crystal {
    /// The catalogue's id, e.g. "linbo3".
    pub id: String,
    /// Its formula, e.g. "LiNbO₃".
    pub formula: String,
    /// "uniaxial" or "biaxial".
    pub optical: &'static str,
    /// Which crystal axis a uniaxial crystal's optic axis is, as the catalogue says it: "z (the
    /// c axis)"; how a biaxial crystal's principal axes lie on a, b and c.
    pub optic_axis: String,
    /// Positive (n_e > n_o) or negative uniaxial; false for a biaxial crystal.
    pub positive: bool,
    /// Where the job uses it, e.g. `stack.layer "film"` or `body #2`.
    pub places: Vec<String>,
    /// The places the job orients it at, and how, in the order of `places`.
    pub oriented: Vec<Oriented>,
}

/// One place a job orients a crystal at (`material = { id, orientation }` in its
/// `[task.stack]`): the crystal's axes laid on the chip's.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Oriented {
    /// Where, as in [`Crystal::places`].
    pub place: String,
    /// The stack's layer it is in, by name; none for the substrate or the cladding.
    pub layer: Option<String>,
    /// What it is there: "material", "background", "substrate" or "cladding".
    pub role: String,
    /// The orientation in words: "x-cut, its y along the chip's x", or its Euler angles.
    pub label: String,
    /// The crystal's x, y and z as unit vectors in the chip's frame: the orientation matrix's
    /// columns (`photonoxide::material::Orientation::matrix`).
    pub axes: [[f64; 3]; 3],
    /// The crystal axis along the chip's x, y and z ("y", "-z"), or "" where none is (a tilt):
    /// the matrix's rows.
    pub along_chip: [String; 3],
}

impl Oriented {
    fn new(place: String, layer: Option<String>, role: &str, orientation: Orientation) -> Self {
        let m = orientation.matrix();
        let along = |row: [f64; 3]| {
            row.iter()
                .position(|v| (v.abs() - 1.0).abs() < 1e-9)
                .map_or_else(String::new, |k| {
                    let sign = if row[k] < 0.0 { "-" } else { "" };
                    format!("{sign}{}", ["x", "y", "z"][k])
                })
        };
        Oriented {
            place,
            layer,
            role: role.to_owned(),
            label: orientation.to_string(),
            axes: std::array::from_fn(|k| std::array::from_fn(|i| m[i][k])),
            along_chip: m.map(along),
        }
    }
}

/// The keys a job names a medium by: a catalogue id, or a table with its `id`.
const MEDIA: [&str; 4] = ["material", "background", "substrate", "cladding"];

/// The catalogue's anisotropic materials `job` uses, in its task or its device, by id, each
/// with the places it is used at in order, and how it is oriented there.
pub fn crystals(job: &Job) -> Vec<Crystal> {
    let mut found = Vec::new();
    walk(job.task(), "", None, &mut found);
    if let Some(device) = job.device() {
        walk(device, "device", None, &mut found);
    }
    found.sort_by(|a, b| a.id.cmp(&b.id));
    for c in &mut found {
        c.places.sort();
        c.oriented.sort_by(|a, b| a.place.cmp(&b.place));
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

/// Looks through `table`, at `path` in the job (in the stack's layer `layer`, if any), for
/// media that are anisotropic crystals.
fn walk(table: &Table, path: &str, layer: Option<&str>, found: &mut Vec<Crystal>) {
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
                    add(id, place.clone(), found);
                    // a bad orientation is job::check's to refuse; the frame leaves it out
                    let orientation = t
                        .get("orientation")
                        .and_then(|o| Orientation::from_value(o, &place).ok());
                    if let (Some(o), Some(c)) = (orientation, found.iter_mut().find(|c| c.id == id))
                    {
                        // a stack's substrate and cladding are in no layer
                        let layer = if matches!(key.as_str(), "substrate" | "cladding") {
                            None
                        } else {
                            layer
                        };
                        c.oriented
                            .push(Oriented::new(place, layer.map(str::to_owned), key, o));
                    }
                }
            }
            Value::Table(t) => walk(t, &place, layer, found),
            Value::Array(items) => {
                for (k, item) in items.iter().enumerate() {
                    if let Value::Table(t) = item {
                        let name = t.get("name").and_then(Value::as_str);
                        let named = match name {
                            Some(name) => format!("{key} \"{name}\""),
                            None => format!("{key} #{}", k + 1),
                        };
                        let layer = if key == "layer" { name } else { layer };
                        walk(t, &join(path, &named), layer, found);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Adds catalogue material `id`, used at `place`, when it is uniaxial or biaxial.
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
    let (optical, optic_axis, positive) = match entry.crystal.optical {
        OpticalClass::Uniaxial {
            positive,
            optic_axis,
        } => ("uniaxial", optic_axis, positive),
        OpticalClass::Biaxial { axes } => ("biaxial", axes, false),
        _ => return,
    };
    found.push(Crystal {
        id: entry.id,
        formula: entry.formula,
        optical,
        optic_axis,
        positive,
        places: vec![place],
        oriented: Vec::new(),
    });
}

/// The anisotropic materials the job in `text` uses, for the design view's crystal frame.
#[tauri::command]
pub fn job_crystals(text: String) -> Result<Vec<Crystal>, String> {
    let job = crate::studio::parse_job(&text).map_err(|e| e.to_string())?;
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
        let text = include_str!("../../../jobs/strip-modes.yaml");
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
        let text = include_str!("../../../jobs/strip-modes.yaml");
        assert!(job_crystals(text.to_owned()).unwrap().is_empty());
        // none of these is oriented
        assert!(found.iter().all(|c| c.oriented.is_empty()));
    }

    #[test]
    fn a_job_orients_its_crystals_and_the_frame_names_the_axes_along_the_chips() {
        let job = |stack: &str| {
            Job::parse(&format!(
                "name = \"oriented\"\n\n[task]\nkind = \"modes\"\n[task.stack]\n{stack}"
            ))
            .unwrap()
        };
        let found = crystals(&job(r#"substrate = "sio2"
cladding = { id = "linbo3", orientation = "z-cut" }
[[task.stack.layer]]
name = "film"
thickness_um = 0.6
material = { id = "linbo3", orientation = "x-cut" }
background = { id = "linbo3", orientation = { euler_deg = [30, 0, 0] } }
"#));
        assert_eq!(found.len(), 1);
        let c = &found[0];
        assert_eq!(c.optical, "uniaxial");
        let places: Vec<&str> = c.oriented.iter().map(|o| o.place.as_str()).collect();
        assert_eq!(
            places,
            [
                "stack.cladding",
                "stack.layer \"film\"",
                "stack.layer \"film\".background"
            ]
        );
        let [cladding, film, background] = [&c.oriented[0], &c.oriented[1], &c.oriented[2]];
        // a z-cut is the crystal's own frame; the cladding is in no layer
        assert_eq!(
            (cladding.layer.as_deref(), cladding.role.as_str()),
            (None, "cladding")
        );
        assert_eq!(cladding.along_chip, ["x", "y", "z"]);
        // an x-cut: the crystal's y along the chip's x, its c axis (z) across, its x up
        assert_eq!(
            (film.layer.as_deref(), film.role.as_str()),
            (Some("film"), "material")
        );
        assert_eq!(film.along_chip, ["y", "z", "x"]);
        assert_eq!(
            film.axes,
            [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
        );
        assert!(film.label.starts_with("x-cut"), "{}", film.label);
        // turned 30° about z: only the crystal's z lies along a chip axis
        assert_eq!(background.role, "background");
        assert_eq!(background.along_chip, ["", "", "z"]);
        assert_eq!(background.axes[2], [0.0, 0.0, 1.0]);
        // a bad orientation is left out (job::check refuses it), the crystal still found
        let bad = crystals(&job(r#"[[task.stack.layer]]
name = "film"
thickness_um = 0.6
material = { id = "linbo3", orientation = "w-cut" }
"#));
        assert_eq!(bad.len(), 1);
        assert!(bad[0].oriented.is_empty());
    }
}
