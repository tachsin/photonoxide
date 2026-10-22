//! The media of a job's stack, for the design view's sidebar (#435): each layer's material and
//! background, the substrate and the cladding, with the catalogue's index at the job's
//! wavelength, the range of wavelengths it holds for, its temperature and where its data comes
//! from. A stack of the task's own names its media in the text (`[task.stack]`), and each is
//! found there by its path, for the sidebar to write an orientation into; a preset's are the
//! library's own.

use photonoxide::material::Material;
use photonoxide::material::catalogue::{self, Axis, Conditions};
use photonoxide::run::{Format, Job};
use photonoxide::stack::LayerStack;
use photonoxide::units::{Length, Wavelength};
use serde::Serialize;
use toml::{Table, Value};

use crate::drawn::Key;

/// One index of a medium at the job's wavelength.
#[derive(Debug, Serialize)]
pub struct AxisIndex {
    pub axis: Axis,
    pub n: f64,
    pub k: f64,
}

/// A medium of a job's stack.
#[derive(Debug, Serialize)]
pub struct Medium {
    /// The layer it is in, by name; none for the substrate and the cladding.
    pub layer: Option<String>,
    /// "material", "background", "substrate" or "cladding".
    pub role: &'static str,
    /// Where the text names it (`["task", "stack", "layer", 1, "material"]`); none for a
    /// preset's.
    pub path: Option<Vec<Key>>,
    /// The catalogue's id, when the text names one.
    pub id: Option<String>,
    /// Its name, e.g. "LiNbO₃ (congruent)", or "n = 3.5" for a fixed index.
    pub name: String,
    /// The index model's name; empty for a fixed index or a preset's medium.
    pub model: String,
    /// Whether it is a uniaxial or biaxial crystal, which may be oriented.
    pub anisotropic: bool,
    /// The wavelength the indices are at, µm.
    pub wavelength_um: f64,
    /// Its indices there, one per axis (n_o and n_e for a uniaxial crystal); none outside its
    /// range.
    pub index: Vec<AxisIndex>,
    /// The wavelengths it holds for, µm; none for a fixed index.
    pub range_um: Option<[f64; 2]>,
    /// The temperature, K: the job's, the model's default, or the data's.
    pub temperature_k: Option<f64>,
    /// Where its data comes from: the publication, its DOI, the data's own source and licence,
    /// and anything else to know.
    pub reference: String,
    pub doi: String,
    pub data: String,
    pub notes: String,
}

/// The media of the job in `text`: a stack of its own's in the text's order (the substrate,
/// each layer's material and background, the cladding), or a preset's. Empty for a device's
/// job.
///
/// # Errors
///
/// A message when the text isn't a job.
pub fn media(text: &str, format: Format) -> Result<Vec<Medium>, String> {
    let job = Job::parse_as(text, format).map_err(|e| e.to_string())?;
    if job.device().is_some() {
        return Ok(Vec::new());
    }
    let task = job.task();
    let wavelength = number(task.get("wavelength_um")).unwrap_or(1.55);
    let mut out = Vec::new();
    match task.get("stack") {
        Some(Value::Table(t)) if !t.contains_key("preset") => {
            let path = |rest: &[Key]| {
                let mut p = vec![Key::Name("task".into()), Key::Name("stack".into())];
                p.extend_from_slice(rest);
                p
            };
            if let Some(v) = t.get("substrate") {
                out.push(written(
                    v,
                    None,
                    "substrate",
                    path(&[Key::Name("substrate".into())]),
                    wavelength,
                ));
            }
            for (k, layer) in t
                .get("layer")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .enumerate()
            {
                let Value::Table(l) = layer else { continue };
                let name = l.get("name").and_then(Value::as_str).map(str::to_owned);
                for role in ["material", "background"] {
                    if let Some(v) = l.get(role) {
                        let at = path(&[
                            Key::Name("layer".into()),
                            Key::Index(k),
                            Key::Name(role.into()),
                        ]);
                        out.push(written(v, name.clone(), role, at, wavelength));
                    }
                }
            }
            if let Some(v) = t.get("cladding") {
                out.push(written(
                    v,
                    None,
                    "cladding",
                    path(&[Key::Name("cladding".into())]),
                    wavelength,
                ));
            }
        }
        stack => {
            let (name, oxide) = match stack {
                Some(Value::Table(t)) => (
                    t.get("preset").and_then(Value::as_str),
                    number(t.get("bottom_oxide_um")),
                ),
                Some(Value::String(s)) => (Some(s.as_str()), None),
                _ => (None, None),
            };
            let oxide = Length::um(oxide.or(number(task.get("bottom_oxide_um"))).unwrap_or(2.0));
            let core = number(task.get("core_nm"));
            let stack = match name.unwrap_or("soi_220") {
                "soi" => LayerStack::soi(Length::nm(core.unwrap_or(220.0)), oxide).ok(),
                "nitride" => LayerStack::nitride(Length::nm(core.unwrap_or(400.0)), oxide).ok(),
                "soi_220" => Some(LayerStack::soi_220()),
                _ => None,
            };
            if let Some(s) = stack {
                out.push(preset(s.substrate(), None, "substrate", wavelength));
                for l in s.layers() {
                    out.push(preset(
                        &l.material,
                        Some(l.name.clone()),
                        "material",
                        wavelength,
                    ));
                    out.push(preset(
                        &l.background,
                        Some(l.name.clone()),
                        "background",
                        wavelength,
                    ));
                }
                out.push(preset(s.cladding(), None, "cladding", wavelength));
            }
        }
    }
    Ok(out)
}

/// [`media`], for the sidebar.
#[tauri::command]
pub fn job_media(text: String, format: Format) -> Result<Vec<Medium>, String> {
    media(&text, format)
}

#[allow(clippy::cast_precision_loss)]
fn number(v: Option<&Value>) -> Option<f64> {
    v.and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
}

/// A medium as `materials` (one per axis of `axes`) have it.
fn of(materials: &[Material], axes: &[Axis], wavelength: f64) -> Medium {
    let lam = Wavelength::um(wavelength).ok();
    let index = axes
        .iter()
        .zip(materials)
        .filter_map(|(&axis, m)| {
            let n = m.refractive_index(lam?).ok()?;
            Some(AxisIndex {
                axis,
                n: n.re,
                k: n.im,
            })
        })
        .collect();
    let first = &materials[0];
    let (lo, hi) = first.range();
    let p = first.provenance();
    Medium {
        layer: None,
        role: "material",
        path: None,
        id: None,
        name: first.name().to_owned(),
        model: String::new(),
        anisotropic: false,
        wavelength_um: wavelength,
        index,
        // a fixed index holds everywhere
        range_um: (lo.to_um() > 1e-6 && hi.to_um() < 1e6).then(|| [lo.to_um(), hi.to_um()]),
        temperature_k: p.temperature,
        reference: p.reference.clone(),
        doi: p.doi.clone(),
        data: p.data.clone(),
        notes: p.notes.clone(),
    }
}

/// A preset's medium.
fn preset(m: &Material, layer: Option<String>, role: &'static str, wavelength: f64) -> Medium {
    Medium {
        layer,
        role,
        ..of(std::slice::from_ref(m), &[Axis::Isotropic], wavelength)
    }
}

/// A medium the text names at `path`: a catalogue id, a table with its id (and model,
/// temperature, composition, orientation), or a fixed index.
fn written(
    v: &Value,
    layer: Option<String>,
    role: &'static str,
    path: Vec<Key>,
    wavelength: f64,
) -> Medium {
    let blank = |name: String, notes: String| Medium {
        layer: layer.clone(),
        role,
        path: Some(path.clone()),
        id: None,
        name,
        model: String::new(),
        anisotropic: false,
        wavelength_um: wavelength,
        index: Vec::new(),
        range_um: None,
        temperature_k: None,
        reference: String::new(),
        doi: String::new(),
        data: String::new(),
        notes,
    };
    if let Some(n) = number(Some(v)) {
        return Medium {
            name: format!("n = {n}"),
            index: vec![AxisIndex {
                axis: Axis::Isotropic,
                n,
                k: 0.0,
            }],
            reference: "a fixed index, the same at every wavelength".into(),
            ..blank(String::new(), String::new())
        };
    }
    let empty = Table::new();
    let (id, t) = match v {
        Value::String(id) => (id.as_str(), &empty),
        Value::Table(t) => match t.get("id").and_then(Value::as_str) {
            Some(id) => (id, t),
            None => return blank("a medium without an id".into(), String::new()),
        },
        _ => return blank("an unknown medium".into(), String::new()),
    };
    let Some(entry) = catalogue::entry(id) else {
        return blank(
            id.to_owned(),
            format!("no material \"{id}\" in the catalogue"),
        );
    };
    let model = match t.get("model").and_then(Value::as_str) {
        Some(m) => entry.index.iter().find(|x| x.id == m),
        None => entry.default_model(),
    };
    let Some(model) = model else {
        return blank(entry.name.clone(), format!("{id} has no such index model"));
    };
    let conditions = Conditions {
        temperature: number(t.get("temperature")),
        composition: number(t.get("composition")),
    };
    let materials = match model.materials(conditions) {
        Ok(m) if !m.is_empty() => m,
        Ok(_) => return blank(entry.name.clone(), String::new()),
        Err(e) => return blank(entry.name.clone(), e.to_string()),
    };
    let base = of(&materials, &model.axes, wavelength);
    Medium {
        layer,
        role,
        path: Some(path),
        id: Some(entry.id.clone()),
        name: entry.name.clone(),
        model: model.name.clone(),
        anisotropic: model.axes.len() > 1,
        temperature_k: conditions
            .temperature
            .or(model.temperature.as_ref().map(|p| p.default))
            .or(base.temperature_k),
        ..base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of_task(task: &str) -> Vec<Medium> {
        media(&format!("name = \"media\"\n\n[task]\n{task}"), Format::Toml).unwrap()
    }

    #[test]
    fn a_preset_has_its_media_with_their_indices_and_sources() {
        let m = of_task(
            "kind = \"modes\"\nstack = \"soi_220\"\nlayer = \"Si\"\nwavelength_um = 1.55\nx_um = [-1, 1]\ny_um = [0, 0]\nstep_nm = 50",
        );
        let si = m
            .iter()
            .find(|m| m.layer.as_deref() == Some("Si") && m.role == "material")
            .unwrap();
        assert!(si.path.is_none());
        assert!((si.index[0].n - 3.48).abs() < 0.01, "{}", si.index[0].n);
        let [lo, hi] = si.range_um.unwrap();
        assert!(lo < 1.55 && 1.55 < hi);
        assert!(!si.reference.is_empty());
        assert_eq!(m.first().unwrap().role, "substrate");
        assert_eq!(m.last().unwrap().role, "cladding");
    }

    #[test]
    fn a_stack_of_its_own_names_each_medium_where_the_text_has_it() {
        let m = of_task(
            "kind = \"structure\"\nlayer = \"film\"\nwavelength_um = 1.55\nx_um = [-1, 1]\ny_um = [-1, 1]\nstep_nm = 50\n\
             [task.stack]\nsubstrate = \"sio2\"\ncladding = 1.0\n\
             [[task.stack.layer]]\nname = \"film\"\nthickness_um = 0.3\n\
             material = { id = \"linbo3\", orientation = \"x-cut\" }\nbackground = \"sio2\"\n",
        );
        let film = m.iter().find(|m| m.role == "material").unwrap();
        assert_eq!(film.layer.as_deref(), Some("film"));
        assert_eq!(film.id.as_deref(), Some("linbo3"));
        assert!(film.anisotropic);
        assert_eq!(film.index.len(), 2, "n_o and n_e");
        assert!(film.index.iter().all(|i| i.n > 2.1 && i.n < 2.3));
        assert!(film.range_um.is_some() && !film.doi.is_empty());
        let path = serde_json::to_string(&film.path).unwrap();
        assert_eq!(path, r#"["task","stack","layer",0,"material"]"#);
        let cladding = m.iter().find(|m| m.role == "cladding").unwrap();
        assert_eq!(
            (cladding.name.as_str(), cladding.index[0].n),
            ("n = 1", 1.0)
        );
        assert!(cladding.range_um.is_none());
        assert!(
            !m.iter()
                .find(|m| m.role == "substrate")
                .unwrap()
                .anisotropic
        );
    }

    #[test]
    fn a_wavelength_outside_the_range_has_no_index() {
        let m = of_task(
            "kind = \"structure\"\nlayer = \"film\"\nwavelength_um = 100.0\nx_um = [-1, 1]\ny_um = [-1, 1]\nstep_nm = 50\n\
             [task.stack]\nsubstrate = \"sio2\"\ncladding = \"sio2\"\n\
             [[task.stack.layer]]\nname = \"film\"\nthickness_um = 0.3\nmaterial = \"si\"\nbackground = \"sio2\"\n",
        );
        let si = m.iter().find(|m| m.role == "material").unwrap();
        let [_, hi] = si.range_um.unwrap();
        assert!(hi < 100.0 && si.index.is_empty());
    }
}
