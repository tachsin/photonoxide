//! The Custom device's form: a job of the four kinds (`structure`, `modes`, `fdfd`, `fdtd`) as
//! the form edits it, and back to the job file's text. What the form holds is its fields; what
//! it doesn't (a `[task.stack]` of layers, components, paths, shapes defined by functions,
//! bodies, a port found by name, a `[solver]` table, a value the form can't show) is kept as the
//! file has it and written back unchanged, named in [`Form::kept`] so the page can say so. A
//! task's `[task.parameters]` and a sweep of any of them are the form's own: a shape whose
//! places and sizes are expressions of them is kept, and follows them.
//!
//! Every job in `jobs/` comes back through the form as the same job (the tests).

use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use photonoxide::job::{Event, SceneShape};
use photonoxide::run::{Format, Job};

/// A number or an expression's text (`"w/2 + gap"`, `"150 nm"`), as a field holds it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Num {
    Number(f64),
    Text(String),
}

impl Num {
    fn of(v: &Value) -> Option<Num> {
        match v {
            Value::Float(x) => Some(Num::Number(*x)),
            Value::Integer(i) => Some(Num::Number(*i as f64)),
            Value::String(s) => Some(Num::Text(s.clone())),
            _ => None,
        }
    }

    fn write(&self) -> String {
        match self {
            Num::Number(x) => float(*x),
            Num::Text(s) => quoted(s),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub layer: String,
    pub center_um: [f64; 2],
    pub size_um: [f64; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Circle {
    pub layer: String,
    pub center_um: [f64; 2],
    pub radius_um: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ring {
    pub layer: String,
    pub center_um: [f64; 2],
    pub radius_um: f64,
    pub width_um: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Port {
    pub x_um: f64,
    pub side: String,
    pub y_um: Option<[f64; 2]>,
}

/// A sweep of the wavelength, a rectangle's width, or any of the task's parameters, its ends
/// numbers or expressions with units.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sweep {
    pub parameter: String,
    pub from: Num,
    pub to: Num,
    pub points: i64,
    pub rect: Option<i64>,
}

/// An fdtd job's source or monitor, with the fields its type reads.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Item {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: Option<String>,
    pub normal: Option<String>,
    pub at_um: Option<f64>,
    pub direction: Option<String>,
    pub x_um: Option<[f64; 2]>,
    pub y_um: Option<[f64; 2]>,
    pub z_um: Option<[f64; 2]>,
    pub position_um: Option<Vec<f64>>,
    pub component: Option<String>,
    pub polarization: Option<String>,
    pub center_um: Option<Vec<f64>>,
    pub waist_um: Option<f64>,
    pub focus_um: Option<f64>,
    pub angle_deg: Option<f64>,
    pub wavelengths_um: Option<Vec<f64>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Boundaries {
    pub x: Option<String>,
    pub y: Option<String>,
    pub z: Option<String>,
    pub bloch: Option<[f64; 3]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spectrum {
    pub from_um: f64,
    pub to_um: f64,
    pub points: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stop {
    pub until: String,
    pub fraction: Option<f64>,
    pub time_um: Option<f64>,
    pub limit_um: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub field: Option<String>,
    pub z_um: Option<f64>,
    pub every_um: Option<f64>,
}

/// A job as the Custom device's form holds it (the window's `JobModel`, lib/job.ts).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Form {
    /// The opening comment: what the job is.
    pub about: String,
    pub name: String,
    pub timeout_minutes: Option<f64>,
    pub kind: String,
    /// A preset's name; `""` when the task has a `[task.stack]` of its own, which is kept.
    pub stack: String,
    pub core_nm: Option<f64>,
    pub bottom_oxide_um: Option<f64>,
    pub wavelength_um: f64,
    pub layer: String,
    pub propagation: Option<String>,
    pub x_um: [f64; 2],
    pub y_um: [f64; 2],
    pub z_um: Option<[f64; 2]>,
    pub side_y_um: Option<f64>,
    pub cut_x_um: Option<f64>,
    pub cut_y_um: Option<f64>,
    pub step_nm: f64,
    pub modes: i64,
    pub modes_above: Option<f64>,
    pub polarization: String,
    pub pml_cells: Option<i64>,
    pub field_um: Option<f64>,
    pub rect: Vec<Rect>,
    pub circle: Vec<Circle>,
    pub ring: Vec<Ring>,
    pub port: Vec<Port>,
    pub sweep: Option<Sweep>,
    pub dimensions: i64,
    pub courant: Option<f64>,
    pub boundaries: Boundaries,
    pub spectrum: Option<Spectrum>,
    pub source: Vec<Item>,
    pub monitor: Vec<Item>,
    pub stop: Stop,
    pub view: View,
    /// The task's named parameters, `[task.parameters]`, in the file's order (as TOML reads
    /// it): each a number or an expression.
    pub parameters: Vec<(String, Num)>,
    /// What the form doesn't hold, by name (`"task.component"`, `"solver"`): kept as the file
    /// has it, and written back unchanged.
    pub kept: Vec<String>,
    /// Those, as TOML: a table of the top-level ones and, under `task`, the task's.
    pub kept_toml: String,
}

const KINDS: [&str; 4] = ["structure", "modes", "fdfd", "fdtd"];
const PRESETS: [&str; 3] = ["soi_220", "soi", "nitride"];

/// A float as TOML writes it, to the last bit: always with a point or an exponent.
fn float(x: f64) -> String {
    if !x.is_finite() {
        return "0.0".into();
    }
    format!("{x:?}")
}

fn quoted(s: &str) -> String {
    Value::String(s.to_owned()).to_string()
}

fn pair(p: [f64; 2]) -> String {
    format!("[{}, {}]", float(p[0]), float(p[1]))
}

fn number(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Float(x) => Some(*x),
        Value::Integer(i) => Some(*i as f64),
        _ => None,
    }
}

fn integer(v: Option<&Value>) -> Option<i64> {
    match v? {
        Value::Integer(i) => Some(*i),
        _ => None,
    }
}

fn string(v: Option<&Value>) -> Option<String> {
    v?.as_str().map(str::to_owned)
}

fn pair_of(v: Option<&Value>) -> Option<[f64; 2]> {
    let a = v?.as_array()?;
    match a.as_slice() {
        [x, y] => Some([number(Some(x))?, number(Some(y))?]),
        _ => None,
    }
}

fn numbers(v: Option<&Value>) -> Option<Vec<f64>> {
    v?.as_array()?.iter().map(|x| number(Some(x))).collect()
}

/// Whether `t`'s keys are all among `keys`.
fn only(t: &Table, keys: &[&str]) -> bool {
    t.keys().all(|k| keys.contains(&k.as_str()))
}

/// A list of tables read by `read`, each entry only of the keys it knows: `None` (the list
/// kept) when any isn't.
fn list<T>(v: Option<&Value>, read: impl Fn(&Table) -> Option<T>) -> Option<Vec<T>> {
    match v {
        None => Some(Vec::new()),
        Some(Value::Array(a)) => a.iter().map(|e| e.as_table().and_then(&read)).collect(),
        _ => None,
    }
}

/// The opening comment of a TOML or YAML job's text, as one paragraph, without its usage lines.
fn about_of(text: &str) -> String {
    text.lines()
        .map_while(|l| l.strip_prefix('#'))
        .map(str::trim)
        .take_while(|l| !l.is_empty() && !l.starts_with("photonoxide "))
        .collect::<Vec<_>>()
        .join(" ")
}

impl Form {
    /// The form of the job in `text`, written in `format`.
    ///
    /// # Errors
    ///
    /// A message when the text isn't a job file, or is a device's (it opens on its device's
    /// page).
    pub fn of(text: &str, format: Format) -> Result<Form, String> {
        let job = Job::parse_as(text, format).map_err(|e| e.to_string())?;
        if job.device().is_some() {
            return Err(format!(
                "{} is a device's job: it opens on its device's page",
                job.name()
            ));
        }
        if job.circuit().is_some() {
            return Err(format!(
                "{} is a chip's job (a [circuit] table of devices): photonoxide run runs it",
                job.name()
            ));
        }
        // the file as TOML reads it, whatever it was written in
        let toml_text = job.to_text(Format::Toml).map_err(|e| e.to_string())?;
        let file: Table = toml::from_str(&toml_text).map_err(|e| e.to_string())?;
        let task = file
            .get("task")
            .and_then(Value::as_table)
            .cloned()
            .unwrap_or_default();
        let kind = text_kind(&task);
        let mut kept_task = Table::new();
        let mut keep = |k: &str| {
            if let Some(v) = task.get(k) {
                kept_task.insert(k.to_owned(), v.clone());
            }
        };
        // each field the form holds, when it is what the form shows; else kept
        let mut held = |k: &str, ok: bool| {
            if !ok && task.contains_key(k) {
                keep(k);
            }
            ok
        };
        let t = |k: &str| task.get(k);
        let stack = match t("stack") {
            Some(Value::String(s)) if PRESETS.contains(&s.as_str()) => s.clone(),
            None => "soi_220".into(),
            Some(_) => {
                held("stack", false);
                String::new()
            }
        };
        let num = |k: &str, held: &mut dyn FnMut(&str, bool) -> bool| -> Option<f64> {
            let v = number(t(k));
            held(k, v.is_some() || t(k).is_none());
            v
        };
        let core_nm = num("core_nm", &mut held);
        let bottom_oxide_um = num("bottom_oxide_um", &mut held);
        let wavelength_um = num("wavelength_um", &mut held).unwrap_or(1.55);
        let layer = string(t("layer"));
        held("layer", layer.is_some() || t("layer").is_none());
        let propagation = string(t("propagation")).filter(|p| p == "x" || p == "y");
        held(
            "propagation",
            propagation.is_some() || t("propagation").is_none(),
        );
        let window = |k: &str, held: &mut dyn FnMut(&str, bool) -> bool| -> Option<[f64; 2]> {
            let v = pair_of(t(k));
            held(k, v.is_some() || t(k).is_none());
            v
        };
        let x_um = window("x_um", &mut held);
        let y_um = window("y_um", &mut held);
        let z_um = window("z_um", &mut held);
        let side_y_um = num("side_y_um", &mut held);
        let cut_x_um = num("cut_x_um", &mut held);
        let cut_y_um = num("cut_y_um", &mut held);
        let step_nm = num("step_nm", &mut held).unwrap_or(20.0);
        let modes = integer(t("modes"));
        held("modes", modes.is_some() || t("modes").is_none());
        let modes_above = num("modes_above", &mut held);
        let polarization = string(t("polarization")).filter(|p| p == "te" || p == "tm");
        held(
            "polarization",
            polarization.is_some() || t("polarization").is_none(),
        );
        let pml_cells = integer(t("pml_cells"));
        held("pml_cells", pml_cells.is_some() || t("pml_cells").is_none());
        let field_um = num("field_um", &mut held);
        let dimensions = integer(t("dimensions"));
        held(
            "dimensions",
            dimensions.is_some_and(|d| d == 2 || d == 3) || t("dimensions").is_none(),
        );
        let courant = num("courant", &mut held);
        let rect = list(t("rect"), |r| {
            only(r, &["layer", "center_um", "size_um"]).then_some(())?;
            Some(Rect {
                layer: string(r.get("layer"))?,
                center_um: pair_of(r.get("center_um"))?,
                size_um: pair_of(r.get("size_um"))?,
            })
        });
        held("rect", rect.is_some());
        let circle = list(t("circle"), |c| {
            only(c, &["layer", "center_um", "radius_um"]).then_some(())?;
            Some(Circle {
                layer: string(c.get("layer"))?,
                center_um: pair_of(c.get("center_um"))?,
                radius_um: number(c.get("radius_um"))?,
            })
        });
        held("circle", circle.is_some());
        let ring = list(t("ring"), |r| {
            only(r, &["layer", "center_um", "radius_um", "width_um"]).then_some(())?;
            Some(Ring {
                layer: string(r.get("layer"))?,
                center_um: pair_of(r.get("center_um"))?,
                radius_um: number(r.get("radius_um"))?,
                width_um: number(r.get("width_um"))?,
            })
        });
        held("ring", ring.is_some());
        let port = list(t("port"), |p| {
            only(p, &["x_um", "side", "y_um"]).then_some(())?;
            Some(Port {
                x_um: number(p.get("x_um"))?,
                side: string(p.get("side")).filter(|s| s == "left" || s == "right")?,
                y_um: match p.get("y_um") {
                    None => None,
                    v => Some(pair_of(v)?),
                },
            })
        });
        held("port", port.is_some());
        let item = |e: &Table| -> Option<Item> {
            const KEYS: [&str; 16] = [
                "type",
                "name",
                "normal",
                "at_um",
                "direction",
                "x_um",
                "y_um",
                "z_um",
                "position_um",
                "component",
                "polarization",
                "center_um",
                "waist_um",
                "focus_um",
                "angle_deg",
                "wavelengths_um",
            ];
            only(e, &KEYS).then_some(())?;
            let opt_text = |k: &str| match e.get(k) {
                None => Some(None),
                Some(Value::String(s)) => Some(Some(s.clone())),
                Some(_) => None,
            };
            let opt_num = |k: &str| match e.get(k) {
                None => Some(None),
                v => number(v).map(Some),
            };
            let opt_pair = |k: &str| match e.get(k) {
                None => Some(None),
                v => pair_of(v).map(Some),
            };
            let opt_nums = |k: &str| match e.get(k) {
                None => Some(None),
                v => numbers(v).map(Some),
            };
            Some(Item {
                kind: string(e.get("type"))?,
                name: opt_text("name")?,
                normal: opt_text("normal")?,
                at_um: opt_num("at_um")?,
                direction: opt_text("direction")?,
                x_um: opt_pair("x_um")?,
                y_um: opt_pair("y_um")?,
                z_um: opt_pair("z_um")?,
                position_um: opt_nums("position_um")?,
                component: opt_text("component")?,
                polarization: opt_text("polarization")?,
                center_um: opt_nums("center_um")?,
                waist_um: opt_num("waist_um")?,
                focus_um: opt_num("focus_um")?,
                angle_deg: opt_num("angle_deg")?,
                wavelengths_um: opt_nums("wavelengths_um")?,
            })
        };
        let source = list(t("source"), item);
        held("source", source.is_some());
        let monitor = list(t("monitor"), item);
        held("monitor", monitor.is_some());
        let boundaries = match t("boundaries") {
            None => Some(Boundaries::default()),
            Some(Value::Table(b)) if only(b, &["x", "y", "z", "bloch"]) => (|| {
                let side = |k: &str| match b.get(k) {
                    None => Some(None),
                    Some(Value::String(s)) => Some(Some(s.clone())),
                    Some(_) => None,
                };
                let bloch = match b.get("bloch") {
                    None => None,
                    v => {
                        let n = numbers(v)?;
                        (n.len() == 3).then(|| [n[0], n[1], n[2]])
                    }
                };
                if b.contains_key("bloch") && bloch.is_none() {
                    return None;
                }
                Some(Boundaries {
                    x: side("x")?,
                    y: side("y")?,
                    z: side("z")?,
                    bloch,
                })
            })(),
            Some(_) => None,
        };
        held("boundaries", boundaries.is_some());
        let spectrum = match t("spectrum") {
            None => Some(None),
            Some(Value::Table(s)) if only(s, &["from_um", "to_um", "points"]) => (|| {
                Some(Some(Spectrum {
                    from_um: number(s.get("from_um"))?,
                    to_um: number(s.get("to_um"))?,
                    points: integer(s.get("points"))?,
                }))
            })(),
            Some(_) => None,
        };
        held("spectrum", spectrum.is_some());
        let stop = match t("stop") {
            None => Some(Stop {
                until: "decay".into(),
                fraction: None,
                time_um: None,
                limit_um: None,
            }),
            Some(Value::Table(s)) if only(s, &["until", "fraction", "time_um", "limit_um"]) => {
                let until = string(s.get("until")).unwrap_or_else(|| "decay".into());
                let opt = |k: &str| match s.get(k) {
                    None => Some(None),
                    v => number(v).map(Some),
                };
                (|| {
                    Some(Stop {
                        until: until.clone(),
                        fraction: opt("fraction")?,
                        time_um: opt("time_um")?,
                        limit_um: opt("limit_um")?,
                    })
                })()
                .filter(|s| s.until == "decay" || s.until == "time")
            }
            Some(_) => None,
        };
        held("stop", stop.is_some());
        let view = match t("view") {
            None => Some(View::default()),
            Some(Value::Table(v)) if only(v, &["field", "z_um", "every_um"]) => (|| {
                Some(View {
                    field: match v.get("field") {
                        None => None,
                        f => Some(string(f)?),
                    },
                    z_um: match v.get("z_um") {
                        None => None,
                        z => Some(number(z)?),
                    },
                    every_um: match v.get("every_um") {
                        None => None,
                        e => Some(number(e)?),
                    },
                })
            })(),
            Some(_) => None,
        };
        held("view", view.is_some());
        let sweep = match t("sweep") {
            None => Some(None),
            Some(Value::Table(s)) if only(s, &["parameter", "from", "to", "points", "rect"]) => {
                (|| {
                    Some(Some(Sweep {
                        parameter: string(s.get("parameter"))?,
                        from: Num::of(s.get("from")?)?,
                        to: Num::of(s.get("to")?)?,
                        points: integer(s.get("points"))?,
                        rect: match s.get("rect") {
                            None => None,
                            r => Some(integer(r)?),
                        },
                    }))
                })()
            }
            Some(_) => None,
        };
        held("sweep", sweep.is_some());
        let parameters = match t("parameters") {
            None => Some(Vec::new()),
            Some(Value::Table(p)) => p
                .iter()
                .map(|(n, v)| Num::of(v).map(|v| (n.clone(), v)))
                .collect(),
            Some(_) => None,
        };
        held("parameters", parameters.is_some());
        // the task's keys the form has no field for
        const FIELDS: [&str; 33] = [
            "kind",
            "stack",
            "core_nm",
            "bottom_oxide_um",
            "wavelength_um",
            "layer",
            "propagation",
            "x_um",
            "y_um",
            "z_um",
            "side_y_um",
            "cut_x_um",
            "cut_y_um",
            "step_nm",
            "modes",
            "modes_above",
            "polarization",
            "pml_cells",
            "field_um",
            "dimensions",
            "courant",
            "rect",
            "circle",
            "ring",
            "port",
            "source",
            "monitor",
            "boundaries",
            "spectrum",
            "stop",
            "view",
            "sweep",
            "parameters",
        ];
        for k in task.keys() {
            if !FIELDS.contains(&k.as_str()) {
                kept_task.insert(k.clone(), task[k].clone());
            }
        }
        if !KINDS.contains(&kind.as_str()) {
            kept_task.insert(
                "kind".into(),
                task.get("kind")
                    .cloned()
                    .unwrap_or_else(|| Value::String(String::new())),
            );
        }
        let mut kept_file = Table::new();
        for (k, v) in &file {
            if !["name", "timeout_minutes", "task"].contains(&k.as_str()) {
                kept_file.insert(k.clone(), v.clone());
            }
        }
        let mut kept: Vec<String> = kept_file.keys().cloned().collect();
        kept.extend(kept_task.keys().map(|k| format!("task.{k}")));
        if !kept_task.is_empty() {
            kept_file.insert("task".into(), Value::Table(kept_task));
        }
        let kept_toml = if kept_file.is_empty() {
            String::new()
        } else {
            toml::to_string(&kept_file).map_err(|e| e.to_string())?
        };
        Ok(Form {
            about: if format == Format::Json {
                String::new()
            } else {
                about_of(text)
            },
            name: job.name().to_owned(),
            timeout_minutes: number(file.get("timeout_minutes")),
            kind: if KINDS.contains(&kind.as_str()) {
                kind.clone()
            } else {
                "structure".into()
            },
            stack,
            core_nm,
            bottom_oxide_um,
            wavelength_um,
            layer: layer.unwrap_or_else(|| "Si".into()),
            // a modes file that doesn't say is an older one, along y; the others' light goes
            // along x
            propagation: propagation.or_else(|| (kind != "modes").then(|| "x".to_owned())),
            x_um: x_um.unwrap_or([-2.0, 2.0]),
            y_um: y_um.unwrap_or([-2.0, 2.0]),
            z_um,
            side_y_um,
            cut_x_um,
            cut_y_um,
            step_nm,
            modes: modes.unwrap_or(2),
            modes_above,
            polarization: polarization.unwrap_or_else(|| "te".into()),
            pml_cells,
            field_um,
            rect: rect.unwrap_or_default(),
            circle: circle.unwrap_or_default(),
            ring: ring.unwrap_or_default(),
            port: port.unwrap_or_default(),
            sweep: sweep.flatten(),
            dimensions: dimensions.filter(|d| *d == 2 || *d == 3).unwrap_or(2),
            courant,
            boundaries: boundaries.unwrap_or_default(),
            spectrum: spectrum.flatten(),
            source: source.unwrap_or_default(),
            monitor: monitor.unwrap_or_default(),
            stop: stop.unwrap_or(Stop {
                until: "decay".into(),
                fraction: None,
                time_um: None,
                limit_um: None,
            }),
            view: view.unwrap_or_default(),
            parameters: parameters.unwrap_or_default(),
            kept,
            kept_toml,
        })
    }

    /// The job file's text, in TOML: the form's fields in the order the form shows them, then
    /// what it kept, unchanged.
    ///
    /// # Errors
    ///
    /// A message when what is kept isn't TOML (it always is, coming from [`Form::of`]).
    pub fn text(&self) -> Result<String, String> {
        let kept: Table = if self.kept_toml.trim().is_empty() {
            Table::new()
        } else {
            toml::from_str(&self.kept_toml).map_err(|e| e.to_string())?
        };
        let kept_task = kept
            .get("task")
            .and_then(Value::as_table)
            .cloned()
            .unwrap_or_default();
        let has = |k: &str| kept_task.contains_key(k);
        let mut out: Vec<String> = Vec::new();
        if !self.about.trim().is_empty() {
            out.extend(comment(&self.about));
        }
        out.push(format!("#   photonoxide run jobs/{}.toml", self.name));
        out.push(format!("name = {}", quoted(&self.name)));
        if let Some(m) = self.timeout_minutes {
            out.push(format!("timeout_minutes = {}", float(m)));
        }
        out.push(String::new());
        out.push("[task]".into());
        let mut line = |k: &str, v: String| {
            if !has(k) {
                out.push(format!("{k} = {v}"));
            }
        };
        if !has("kind") {
            line("kind", quoted(&self.kind));
        }
        if !self.stack.is_empty() {
            line("stack", quoted(&self.stack));
        }
        if self.stack != "soi_220" && !self.stack.is_empty() {
            line("core_nm", float(self.core_nm.unwrap_or(220.0)));
            line(
                "bottom_oxide_um",
                float(self.bottom_oxide_um.unwrap_or(2.0)),
            );
        }
        line("wavelength_um", float(self.wavelength_um));
        line("layer", quoted(&self.layer));
        let kind = self.kind.as_str();
        if kind == "fdfd" {
            line("polarization", quoted(&self.polarization));
        }
        if kind == "fdtd" {
            if self.dimensions == 3 {
                line("dimensions", "3".into());
            } else {
                line("polarization", quoted(&self.polarization));
            }
        }
        if kind == "modes" {
            if let Some(p) = &self.propagation {
                line("propagation", quoted(p));
            }
            if self.propagation.as_deref() == Some("x") {
                line("y_um", pair(self.y_um));
                if let Some(c) = self.cut_x_um {
                    line("cut_x_um", float(c));
                }
            } else {
                line("x_um", pair(self.x_um));
                if let Some(c) = self.cut_y_um {
                    line("cut_y_um", float(c));
                }
            }
        } else {
            line("x_um", pair(self.x_um));
            line("y_um", pair(self.y_um));
        }
        if let Some(z) = self.z_um
            && (kind == "structure" || kind == "modes" || (kind == "fdtd" && self.dimensions == 3))
        {
            line("z_um", pair(z));
        }
        if kind == "structure"
            && let Some(s) = self.side_y_um
        {
            line("side_y_um", float(s));
        }
        line("step_nm", float(self.step_nm));
        if kind == "modes" {
            match self.modes_above {
                Some(m) => line("modes_above", float(m)),
                None => line("modes", self.modes.max(1).to_string()),
            }
        }
        if (kind == "fdfd" || kind == "fdtd")
            && let Some(p) = self.pml_cells
        {
            line("pml_cells", p.to_string());
        }
        if kind == "fdtd"
            && let Some(c) = self.courant
        {
            line("courant", float(c));
        }
        if kind == "fdfd"
            && let Some(f) = self.field_um
        {
            line("field_um", float(f));
        }
        // the task's plain values the form doesn't hold, before its tables
        for (k, v) in &kept_task {
            if !matches!(v, Value::Table(_)) && !is_tables(v) {
                out.push(format!("{k} = {v}"));
            }
        }
        if !self.parameters.is_empty() && !has("parameters") {
            out.push(String::new());
            out.push("[task.parameters]".into());
            for (n, v) in &self.parameters {
                out.push(format!("{} = {}", key(n), v.write()));
            }
        }
        if kind == "fdtd" {
            let b = &self.boundaries;
            let sides: Vec<(&str, &String)> = [("x", &b.x), ("y", &b.y), ("z", &b.z)]
                .into_iter()
                .filter_map(|(a, s)| s.as_ref().map(|s| (a, s)))
                .filter(|(a, _)| *a != "z" || self.dimensions == 3)
                .collect();
            if (!sides.is_empty() || b.bloch.is_some()) && !has("boundaries") {
                out.push(String::new());
                out.push("[task.boundaries]".into());
                for (a, s) in sides {
                    out.push(format!("{a} = {}", quoted(s)));
                }
                if let Some(bl) = b.bloch {
                    out.push(format!(
                        "bloch = [{}, {}, {}]",
                        float(bl[0]),
                        float(bl[1]),
                        float(bl[2])
                    ));
                }
            }
            if let Some(s) = &self.spectrum
                && !has("spectrum")
            {
                out.push(String::new());
                out.push("[task.spectrum]".into());
                out.push(format!("from_um = {}", float(s.from_um)));
                out.push(format!("to_um = {}", float(s.to_um)));
                out.push(format!("points = {}", s.points.max(1)));
            }
        }
        if !has("rect") {
            for r in &self.rect {
                out.push(String::new());
                out.push("[[task.rect]]".into());
                out.push(format!("layer = {}", quoted(&r.layer)));
                out.push(format!("center_um = {}", pair(r.center_um)));
                out.push(format!("size_um = {}", pair(r.size_um)));
            }
        }
        if !has("circle") {
            for c in &self.circle {
                out.push(String::new());
                out.push("[[task.circle]]".into());
                out.push(format!("layer = {}", quoted(&c.layer)));
                out.push(format!("center_um = {}", pair(c.center_um)));
                out.push(format!("radius_um = {}", float(c.radius_um)));
            }
        }
        if !has("ring") {
            for r in &self.ring {
                out.push(String::new());
                out.push("[[task.ring]]".into());
                out.push(format!("layer = {}", quoted(&r.layer)));
                out.push(format!("center_um = {}", pair(r.center_um)));
                out.push(format!("radius_um = {}", float(r.radius_um)));
                out.push(format!("width_um = {}", float(r.width_um)));
            }
        }
        if kind == "fdfd" && !has("port") {
            for p in &self.port {
                out.push(String::new());
                out.push("[[task.port]]".into());
                out.push(format!("x_um = {}", float(p.x_um)));
                out.push(format!("side = {}", quoted(&p.side)));
                if let Some(y) = p.y_um {
                    out.push(format!("y_um = {}", pair(y)));
                }
            }
        }
        if kind == "fdtd" {
            for (table, items) in [("source", &self.source), ("monitor", &self.monitor)] {
                if has(table) {
                    continue;
                }
                for it in items {
                    out.push(String::new());
                    out.push(format!("[[task.{table}]]"));
                    out.extend(item_lines(it));
                }
            }
            let s = &self.stop;
            let mut stop = Vec::new();
            if s.until == "time" {
                stop.push("until = \"time\"".to_owned());
                stop.push(format!("time_um = {}", float(s.time_um.unwrap_or(500.0))));
            } else {
                if let Some(f) = s.fraction {
                    stop.push(format!("fraction = {}", float(f)));
                }
                if let Some(l) = s.limit_um {
                    stop.push(format!("limit_um = {}", float(l)));
                }
                if !stop.is_empty() {
                    stop.insert(0, "until = \"decay\"".into());
                }
            }
            if !stop.is_empty() && !has("stop") {
                out.push(String::new());
                out.push("[task.stop]".into());
                out.extend(stop);
            }
            let v = &self.view;
            let mut view = Vec::new();
            if let Some(f) = &v.field {
                view.push(format!("field = {}", quoted(f)));
            }
            if let Some(z) = v.z_um
                && self.dimensions == 3
            {
                view.push(format!("z_um = {}", float(z)));
            }
            if let Some(e) = v.every_um {
                view.push(format!("every_um = {}", float(e)));
            }
            if !view.is_empty() && !has("view") {
                out.push(String::new());
                out.push("[task.view]".into());
                out.extend(view);
            }
        }
        if let Some(s) = &self.sweep
            && kind != "structure"
            && kind != "fdtd"
            && !has("sweep")
        {
            out.push(String::new());
            out.push("[task.sweep]".into());
            let parameter = if kind == "fdfd" {
                "wavelength"
            } else {
                s.parameter.as_str()
            };
            out.push(format!("parameter = {}", quoted(parameter)));
            out.push(format!("from = {}", s.from.write()));
            out.push(format!("to = {}", s.to.write()));
            out.push(format!("points = {}", s.points.max(1)));
            if parameter == "width"
                && let Some(r) = s.rect
            {
                out.push(format!("rect = {r}"));
            }
        }
        // the task's tables the form doesn't hold, and the file's own (a [solver] table)
        let tables: Table = kept_task
            .iter()
            .filter(|(_, v)| matches!(v, Value::Table(_)) || is_tables(v))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let mut tail = Table::new();
        if !tables.is_empty() {
            tail.insert("task".into(), Value::Table(tables));
        }
        for (k, v) in &kept {
            if k != "task" {
                tail.insert(k.clone(), v.clone());
            }
        }
        let mut text = out.join("\n") + "\n";
        if !tail.is_empty() {
            let rest = toml::to_string(&tail).map_err(|e| e.to_string())?;
            // the task's own header is already written
            let rest = rest.strip_prefix("[task]\n").unwrap_or(&rest);
            text.push('\n');
            text.push_str(rest);
        }
        Ok(text)
    }
}

/// Whether `v` is an array of tables, written as `[[...]]`.
fn is_tables(v: &Value) -> bool {
    matches!(v, Value::Array(a) if !a.is_empty() && a.iter().all(|e| matches!(e, Value::Table(_))))
}

/// A key as TOML writes it: bare when it can be.
fn key(k: &str) -> String {
    if !k.is_empty()
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        k.to_owned()
    } else {
        quoted(k)
    }
}

/// An fdtd item's lines, its type first and then the fields it has, in the form's order.
fn item_lines(it: &Item) -> Vec<String> {
    let mut l = vec![format!("type = {}", quoted(&it.kind))];
    let s = |l: &mut Vec<String>, k: &str, v: &Option<String>| {
        if let Some(v) = v {
            l.push(format!("{k} = {}", quoted(v)));
        }
    };
    let n = |l: &mut Vec<String>, k: &str, v: Option<f64>| {
        if let Some(v) = v {
            l.push(format!("{k} = {}", float(v)));
        }
    };
    let a = |l: &mut Vec<String>, k: &str, v: &Option<Vec<f64>>| {
        if let Some(v) = v {
            let items: Vec<String> = v.iter().map(|x| float(*x)).collect();
            l.push(format!("{k} = [{}]", items.join(", ")));
        }
    };
    let p = |l: &mut Vec<String>, k: &str, v: Option<[f64; 2]>| {
        if let Some(v) = v {
            l.push(format!("{k} = {}", pair(v)));
        }
    };
    s(&mut l, "name", &it.name);
    s(&mut l, "normal", &it.normal);
    n(&mut l, "at_um", it.at_um);
    s(&mut l, "direction", &it.direction);
    p(&mut l, "x_um", it.x_um);
    p(&mut l, "y_um", it.y_um);
    p(&mut l, "z_um", it.z_um);
    a(&mut l, "position_um", &it.position_um);
    s(&mut l, "component", &it.component);
    s(&mut l, "polarization", &it.polarization);
    a(&mut l, "center_um", &it.center_um);
    n(&mut l, "waist_um", it.waist_um);
    n(&mut l, "focus_um", it.focus_um);
    n(&mut l, "angle_deg", it.angle_deg);
    a(&mut l, "wavelengths_um", &it.wavelengths_um);
    l
}

/// `text` wrapped to 95 characters, as comment lines.
fn comment(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::from("#");
    for word in text.split_whitespace() {
        if line.len() + 1 + word.len() > 95 && line != "#" {
            lines.push(std::mem::replace(&mut line, String::from("#")));
        }
        line.push(' ');
        line.push_str(word);
    }
    if line != "#" {
        lines.push(line);
    }
    lines
}

/// The task's kind, as its table names it.
fn text_kind(task: &Table) -> String {
    task.get("kind")
        .and_then(Value::as_str)
        .unwrap_or("structure")
        .to_owned()
}

/// The form of a job's text (a device's job is refused: it opens on its page).
#[tauri::command]
pub fn job_form(text: String, format: Format) -> Result<Form, String> {
    Form::of(&text, format)
}

/// The job file's text the form holds, in TOML.
#[tauri::command]
pub fn form_text(form: Form) -> Result<String, String> {
    form.text()
}

/// The shapes of the job in `text` the form doesn't hold, seen from above: rectangles whose
/// places or sizes are expressions, components, paths, shapes defined by functions, bodies.
/// Each as the library draws it for the 3D view (`photonoxide::job::preview`, its parameters'
/// expressions evaluated there), so the top view draws them where the 3D view does. The
/// rectangles, disks and rings the form holds are left out: the top view draws those itself,
/// to be picked and edited. None when the form holds everything the job draws.
///
/// # Errors
///
/// A message when the text isn't a job the form reads, or the job without the form's shapes
/// can't be drawn.
pub fn kept_shapes(text: &str, format: Format) -> Result<Vec<SceneShape>, String> {
    let form = Form::of(text, format)?;
    let kept = |k: &str| form.kept.iter().any(|x| x == &format!("task.{k}"));
    let job = Job::parse_as(text, format).map_err(|e| e.to_string())?;
    let mut file: Table = toml::from_str(&job.to_text(Format::Toml).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let Some(task) = file.get_mut("task").and_then(Value::as_table_mut) else {
        return Ok(Vec::new());
    };
    // the shapes the form holds, and a sweep (which may name one of them), are left out
    for k in ["rect", "circle", "ring"] {
        if !kept(k) {
            task.remove(k);
        }
    }
    task.remove("sweep");
    if !task.keys().any(|k| DRAWN.contains(&k.as_str())) {
        return Ok(Vec::new());
    }
    let rest = Job::parse(&toml::to_string(&file).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    match photonoxide::job::preview(&rest).map_err(|e| e.to_string())? {
        Event::Scene { shapes, .. } => Ok(shapes),
        _ => Ok(Vec::new()),
    }
}

/// What a job's task draws shapes with.
const DRAWN: [&str; 7] = [
    "rect",
    "circle",
    "ring",
    "component",
    "path",
    "shape",
    "body",
];

/// [`kept_shapes`], for the Custom device's top view.
#[tauri::command]
pub fn form_kept_shapes(text: String, format: Format) -> Result<Vec<SceneShape>, String> {
    kept_shapes(&text, format)
}

#[cfg(test)]
mod tests;
