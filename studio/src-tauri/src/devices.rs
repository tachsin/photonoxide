//! A component's page, for the components that know their geometry: the library's devices
//! (`photonoxide::device`). The Components page shows each as an Overview and a Design:
//!
//! - [`device_library`]: each device with its parameters, its models (fidelity, method, papers,
//!   stated error, validity), its ports, its Academy lesson and the published results on it;
//! - [`device_geometry`]: a design (a [`Spec`]: the stack, the parameters' expressions, the
//!   fidelity) evaluated as the form shows it: each field's value, the error in it with its
//!   column, what it uses and what uses it; the stack, the shapes and ports from above, and
//!   the 3D scene (`photonoxide::job::preview` of the design's job);
//! - [`device_derived`]: the device's figures at a fidelity, each with its fidelity and grid; an
//!   analytic fit outside its tables refuses, and the page offers the 3D model instead;
//! - [`device_spectrum`]: the S-parameters over wavelength at a fidelity;
//! - [`save_device`] and [`read_device`]: a design as a job file with a `[device]` table in the
//!   workspace's `jobs/`, and back.
//!
//! A solve (2D or 3D) takes seconds: its component is kept, so the spectrum after the figures
//! doesn't solve again. Kinds without a device keep `circuits::component_spectrum`.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use photonoxide::circuit::{Fidelity, Spectrum};
use photonoxide::device::{self, Built, Device, LayerSpec, MaterialSpec, Spec, StackSpec};
use photonoxide::expr::{self, Dim, Expr};
use photonoxide::job::{self, Event};
use photonoxide::run::{Format, Job};
use serde::{Deserialize, Serialize};

use crate::circuits::{PortInfo, SpectrumData, Sweep, ports_of, write_touchstone};
use crate::studio::Studio;

/// Each device: the component library's id it stands for, its Academy lesson, and the
/// published results (examples/) on it.
pub const DEVICES: [(&str, &str, &str, &[&str]); 17] = [
    (
        "waveguide",
        "waveguide",
        "strip-waveguide",
        &[
            "strip_waveguide",
            "group_index",
            "hadley_corners",
            "marcatili",
            "effective_index_method",
        ],
    ),
    ("bend", "bend", "leaky-and-bent-guides", &["bend_loss"]),
    (
        "directional coupler",
        "directional-coupler",
        "directional-coupler",
        &["directional_coupler", "coupler_liu_poon"],
    ),
    (
        "all-pass ring",
        "ring-all-pass",
        "ring-resonator",
        &[
            "circuit_ring_critical",
            "ring_q_factor",
            "circuit_fit",
            "ring_liu_poon",
        ],
    ),
    (
        "add-drop ring",
        "ring-add-drop",
        "ring-resonator",
        &["ring_q_factor"],
    ),
    (
        "rib waveguide",
        "rib-waveguide",
        "strip-waveguide",
        &["rib_hadley"],
    ),
    ("MMI 1x2", "mmi-1x2", "mmi", &["mmi_soldano"]),
    (
        "MMI 2x2",
        "mmi-2x2",
        "mmi",
        &["mmi_soldano", "mmi_liu_poon"],
    ),
    (
        "Y-branch",
        "y-branch",
        "directional-coupler",
        &["y_junction_zhang"],
    ),
    (
        "MZI",
        "mzi-couplers",
        "mach-zehnder",
        &["mzi_dwivedi", "circuit_splitter"],
    ),
    (
        "Y-branch MZI",
        "mzi-y-branches",
        "mach-zehnder",
        &["mzi_dwivedi"],
    ),
    (
        "thermo-optic phase shifter",
        "thermo-optic-shifter",
        "modulators",
        &["thermo_jacques"],
    ),
    (
        "doped thermo-optic phase shifter",
        "doped-thermo-optic-shifter",
        "modulators",
        &["thermo_jacques"],
    ),
    (
        "in-guide thermo-optic phase shifter",
        "in-guide-thermo-optic-shifter",
        "modulators",
        &["thermo_harris"],
    ),
    (
        "GST phase-change phase shifter",
        "gst-phase-shifter",
        "modulators",
        &["pcm_delaney"],
    ),
    (
        "Sb2S3 phase-change phase shifter",
        "sb2s3-phase-shifter",
        "modulators",
        &["pcm_delaney"],
    ),
    (
        "Sb2Se3 phase-change phase shifter",
        "sb2se3-phase-shifter",
        "modulators",
        &["pcm_delaney"],
    ),
];

pub(crate) fn text(e: photonoxide::Error) -> String {
    e.to_string()
}

fn fidelity_name(f: Fidelity) -> &'static str {
    match f {
        Fidelity::Analytic => "analytic",
        Fidelity::Compact => "compact",
        Fidelity::TwoD => "2D",
        Fidelity::ThreeD => "3D",
        Fidelity::Measured => "measured",
    }
}

// ---- the library ----

/// A device as the Components page lists it.
#[derive(Serialize)]
pub struct DeviceInfo {
    /// The component library's id it stands for, e.g. `"directional-coupler"`.
    id: &'static str,
    /// Its kind, as a design names it, e.g. `"directional coupler"`.
    kind: String,
    about: String,
    parameters: Vec<ParameterInfo>,
    /// Its models, cheapest first.
    fidelities: Vec<MethodInfo>,
    /// Its ports' names, in order.
    ports: Vec<String>,
    /// Its Academy lesson's id.
    lesson: &'static str,
    /// The examples that reproduce a paper on it.
    examples: Vec<&'static str>,
    /// Its 2D model by FDFD, where its 2D fidelity has a cheaper one in its place (an MMI's
    /// guided modes): a job's `model_2d = "fdfd"`.
    fdfd_2d: Option<MethodInfo>,
}

#[derive(Serialize)]
struct ParameterInfo {
    name: String,
    unit: String,
    default: f64,
    min: f64,
    max: f64,
}

#[derive(Serialize)]
pub struct MethodInfo {
    fidelity: &'static str,
    name: String,
    write_up: String,
    papers: Vec<String>,
    error: Option<StatedInfo>,
    validity: Vec<RangeInfo>,
}

#[derive(Serialize)]
struct StatedInfo {
    quantity: String,
    bound: f64,
    unit: String,
    against: &'static str,
}

#[derive(Serialize)]
struct RangeInfo {
    name: String,
    min: f64,
    max: f64,
}

/// The library's devices, in its order.
#[tauri::command]
pub fn device_library() -> Result<Vec<DeviceInfo>, String> {
    device::library()
        .iter()
        .map(|d| {
            let (_, id, lesson, examples) = DEVICES
                .iter()
                .find(|e| e.0 == d.kind())
                .ok_or_else(|| format!("the studio has no page for the {}", d.kind()))?;
            let values = device::Values::defaults(d.as_ref());
            let ports = d
                .geometry(&values)
                .map_err(text)?
                .ports
                .into_iter()
                .map(|p| p.name)
                .collect();
            Ok(DeviceInfo {
                id,
                kind: d.kind().to_owned(),
                about: d.about().to_owned(),
                parameters: d
                    .parameters()
                    .iter()
                    .map(|p| ParameterInfo {
                        name: p.name.clone(),
                        unit: p.unit.clone(),
                        default: p.default,
                        min: p.min,
                        max: p.max,
                    })
                    .collect(),
                fidelities: d.fidelities().iter().map(method_info).collect(),
                ports,
                lesson,
                examples: examples.to_vec(),
                fdfd_2d: fdfd_2d(d.kind()),
            })
        })
        .collect()
}

/// A model as the page shows it.
pub(crate) fn method_info(m: &device::Method) -> MethodInfo {
    MethodInfo {
        fidelity: fidelity_name(m.fidelity),
        name: m.name.clone(),
        write_up: m.write_up.clone(),
        papers: m.papers.clone(),
        error: m.error.as_ref().map(|s| StatedInfo {
            quantity: s.quantity.clone(),
            bound: s.bound,
            unit: s.unit.clone(),
            against: fidelity_name(s.against),
        }),
        validity: m
            .validity
            .iter()
            .map(|r| RangeInfo {
                name: r.name.clone(),
                min: r.min,
                max: r.max,
            })
            .collect(),
    }
}

/// The 2D model by FDFD of a device whose 2D fidelity is another by default: an MMI's.
fn fdfd_2d(kind: &str) -> Option<MethodInfo> {
    let mmi = match kind {
        "MMI 1x2" => device::StripMmi::one_by_two(),
        "MMI 2x2" => device::StripMmi::two_by_two(),
        _ => return None,
    };
    mmi.with_model_2d(device::Model2d::Fdfd)
        .fidelities()
        .iter()
        .find(|m| m.fidelity == Fidelity::TwoD)
        .map(method_info)
}

// ---- a design, as the window sends it ----

/// A design as the window holds it: [`Spec`] in JSON.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecIn {
    pub kind: String,
    pub fidelity: String,
    pub stack: StackIn,
    /// (name, expression), in the form's order.
    pub parameters: Vec<(String, String)>,
    /// The file of its compact model, if it has one: the path a saved design names (next to
    /// it), or the absolute path of one not saved yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compact: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StackIn {
    Preset {
        name: String,
        bottom_oxide_um: f64,
    },
    Layers {
        substrate: MaterialIn,
        layers: Vec<LayerIn>,
        cladding: MaterialIn,
        core: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayerIn {
    pub name: String,
    pub thickness_um: f64,
    pub material: MaterialIn,
    pub background: MaterialIn,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialIn {
    pub id: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub composition: Option<f64>,
}

impl From<&MaterialIn> for MaterialSpec {
    fn from(m: &MaterialIn) -> MaterialSpec {
        MaterialSpec {
            id: m.id.clone(),
            model: m.model.clone(),
            temperature: m.temperature,
            composition: m.composition,
        }
    }
}

impl From<&MaterialSpec> for MaterialIn {
    fn from(m: &MaterialSpec) -> MaterialIn {
        MaterialIn {
            id: m.id.clone(),
            model: m.model.clone(),
            temperature: m.temperature,
            composition: m.composition,
        }
    }
}

impl SpecIn {
    /// The library's design.
    ///
    /// # Errors
    ///
    /// A message for an unknown fidelity.
    pub fn spec(&self) -> Result<Spec, String> {
        let stack = match &self.stack {
            StackIn::Preset {
                name,
                bottom_oxide_um,
            } => StackSpec::Preset {
                name: name.clone(),
                bottom_oxide_um: *bottom_oxide_um,
            },
            StackIn::Layers {
                substrate,
                layers,
                cladding,
                core,
            } => StackSpec::Layers {
                substrate: substrate.into(),
                layers: layers
                    .iter()
                    .map(|l| LayerSpec {
                        name: l.name.clone(),
                        thickness_um: l.thickness_um,
                        material: (&l.material).into(),
                        background: (&l.background).into(),
                    })
                    .collect(),
                cladding: cladding.into(),
                core: core.clone(),
            },
        };
        Ok(Spec {
            kind: self.kind.clone(),
            fidelity: device::fidelity(&self.fidelity).map_err(text)?,
            stack,
            parameters: self.parameters.clone(),
            compact: self.compact.clone(),
        })
    }

    /// The design the library's `spec` is.
    pub fn of(spec: &Spec) -> SpecIn {
        let stack = match &spec.stack {
            StackSpec::Preset {
                name,
                bottom_oxide_um,
            } => StackIn::Preset {
                name: name.clone(),
                bottom_oxide_um: *bottom_oxide_um,
            },
            StackSpec::Layers {
                substrate,
                layers,
                cladding,
                core,
            } => StackIn::Layers {
                substrate: substrate.into(),
                layers: layers
                    .iter()
                    .map(|l| LayerIn {
                        name: l.name.clone(),
                        thickness_um: l.thickness_um,
                        material: (&l.material).into(),
                        background: (&l.background).into(),
                    })
                    .collect(),
                cladding: cladding.into(),
                core: core.clone(),
            },
        };
        SpecIn {
            kind: spec.kind.clone(),
            fidelity: fidelity_name(spec.fidelity).to_owned(),
            stack,
            parameters: spec.parameters.clone(),
            compact: spec.compact.clone(),
        }
    }
}

/// The text of the job file that holds `spec` under `name`, with `about` as its opening
/// comment.
fn job_text(spec: &Spec, name: &str, about: &str) -> Result<String, String> {
    let mut file = toml::Table::new();
    file.insert("name".into(), toml::Value::String(name.to_owned()));
    file.insert("device".into(), toml::Value::Table(spec.to_table()));
    // written in YAML, the job's every float to the last bit; a design computes nothing, and
    // has no task
    let body = Job::parse(&toml::to_string(&file).map_err(|e| e.to_string())?)
        .and_then(|j| j.to_text(Format::Yaml))
        .map_err(|e| e.to_string())?
        .replace("\ntask: {}\n", "\n");
    let comment: String = about
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| format!("# {l}\n"))
        .collect();
    Ok(if comment.is_empty() {
        body
    } else {
        format!("{comment}\n{body}")
    })
}

// ---- the form and the geometry ----

/// A field of the form: a parameter's expression and what it gives.
#[derive(Debug, PartialEq, Serialize)]
pub struct Field {
    name: String,
    expression: String,
    /// Its value in its unit (a device parameter's, or µm, radians or none for another).
    value: Option<f64>,
    unit: String,
    /// Whether it is one of the device's parameters, rather than a name the others use.
    device: bool,
    /// The parameters its expression uses, and those whose expressions use it.
    uses: Vec<String>,
    used_by: Vec<String>,
    error: Option<FieldError>,
}

/// What is wrong with a field: the message, and the column (from 1) it points at, if it does.
#[derive(Debug, PartialEq, Serialize)]
struct FieldError {
    message: String,
    column: Option<usize>,
}

/// A port seen from above.
#[derive(Debug, PartialEq, Serialize)]
struct PortView {
    name: String,
    x: f64,
    y: f64,
    /// The direction light leaves by, degrees from +x.
    direction: f64,
    width: f64,
    layer: String,
}

/// A design evaluated: the form's fields, and if they hold a design, its values, ports and
/// scene (the stack, and every shape's outline), and where the fidelity's stated error doesn't
/// hold.
#[derive(Serialize)]
pub struct GeometryView {
    fields: Vec<Field>,
    /// The device's values in its parameters' order, when the design is whole.
    values: Option<Vec<f64>>,
    /// What keeps it from being a design: a field's error, or the stack's.
    problem: Option<String>,
    /// The stack's own problem, apart from the fields'.
    stack_problem: Option<String>,
    ports: Vec<PortView>,
    scene: Option<Event>,
    /// Values outside the fidelity's validity ranges.
    outside: Vec<String>,
}

/// The column an expression's error names ("column 3: …"), and the message without it.
fn column(message: &str) -> (Option<usize>, String) {
    let Some(rest) = message.strip_prefix("column ") else {
        return (None, message.to_owned());
    };
    let Some((n, msg)) = rest.split_once(": ") else {
        return (None, message.to_owned());
    };
    match n.parse() {
        Ok(c) => (Some(c), msg.to_owned()),
        Err(_) => (None, message.to_owned()),
    }
}

/// The unit of an expression of dimension `dim`, as a field shows it.
fn dim_unit(dim: Option<Dim>) -> &'static str {
    match dim {
        Some(d) if d == Dim::LENGTH => "µm",
        Some(d) if d == Dim::ANGLE => "rad",
        Some(d) if d == Dim::NONE => "",
        None => "",
        Some(_) => "?",
    }
}

/// The form's definitions: the device's parameters in its order, each as the design gives it or
/// at its default as [`device::Design::new`] fills it in, then the design's other named
/// parameters in its order (a file lists them by name; the form keeps the device's order).
fn definitions(spec: &Spec, device: &dyn Device) -> Vec<(String, String)> {
    let given = |name: &str| spec.parameters.iter().find(|(n, _)| n == name);
    let mut all: Vec<(String, String)> = device
        .parameters()
        .iter()
        .map(|p| {
            given(&p.name)
                .cloned()
                .unwrap_or_else(|| (p.name.clone(), format!("{:?}", p.default)))
        })
        .collect();
    all.extend(
        spec.parameters
            .iter()
            .filter(|(n, _)| !device.parameters().iter().any(|p| p.name == *n))
            .cloned(),
    );
    all
}

/// Evaluates `spec` as the form shows it.
pub fn geometry(spec_in: &SpecIn) -> GeometryView {
    let mut view = GeometryView {
        fields: Vec::new(),
        values: None,
        problem: None,
        stack_problem: None,
        ports: Vec::new(),
        scene: None,
        outside: Vec::new(),
    };
    let spec = match spec_in.spec() {
        Ok(s) => s,
        Err(e) => {
            view.problem = Some(e);
            return view;
        }
    };
    // the device's own parameters, whatever its stack: a stack that can't be built still has
    // its fields shown
    let Some(template) = device::library()
        .into_iter()
        .find(|d| d.kind() == spec.kind)
    else {
        view.problem = Some(format!("no device \"{}\"", spec.kind));
        return view;
    };
    let parameters = template.parameters().to_vec();
    let all = definitions(&spec, template.as_ref());
    let is_device = |n: &str| parameters.iter().any(|p| p.name == n);
    let mut fields: Vec<Field> = all
        .iter()
        .map(|(name, expression)| Field {
            name: name.clone(),
            expression: expression.clone(),
            value: None,
            unit: parameters
                .iter()
                .find(|p| p.name == *name)
                .map_or_else(String::new, |p| p.unit.clone()),
            device: is_device(name),
            uses: Vec::new(),
            used_by: Vec::new(),
            error: None,
        })
        .collect();
    // the expressions: a field's own error, with its column
    match expr::Parameters::new(&all) {
        Err(e) => {
            let (col, message) = column(&e.message);
            match fields.iter_mut().find(|f| f.name == e.name) {
                Some(f) => {
                    f.error = Some(FieldError {
                        message: message.clone(),
                        column: col,
                    });
                    view.problem = Some(format!("{}: {message}", e.name));
                }
                None => view.problem = Some(e.to_string()),
            }
            // what each uses, as far as names go
            uses(&mut fields, None);
            view.fields = fields;
            return view;
        }
        Ok(p) => {
            uses(&mut fields, Some(&p));
            for f in &mut fields {
                let dim = p.dim(&f.name).flatten();
                let v = p.value(&f.name);
                if f.device {
                    // an angle in radians given to a parameter in degrees is turned into degrees
                    let factor = match dim {
                        Some(d) if d.angle != 0 && f.unit == "°" => 180.0 / std::f64::consts::PI,
                        _ => 1.0,
                    };
                    f.value = v.map(|v| v * factor);
                } else {
                    f.value = v;
                    f.unit = dim_unit(dim).to_owned();
                }
            }
        }
    }
    // the device on its stack, and the design: units and ranges
    let device = match spec.device() {
        Ok(d) => d,
        Err(e) => {
            view.stack_problem = Some(text(e));
            view.problem.clone_from(&view.stack_problem);
            view.fields = fields;
            return view;
        }
    };
    let (_, design) = match spec.build() {
        Ok(b) => b,
        Err(e) => {
            let message = text(e);
            let at = parameters
                .iter()
                .find(|p| message.contains(&format!("'s {} ", p.name)))
                .map(|p| p.name.clone());
            match at.and_then(|n| fields.iter_mut().find(|f| f.name == n)) {
                Some(f) => {
                    f.error = Some(FieldError {
                        message: message.clone(),
                        column: None,
                    });
                }
                None => view.stack_problem = Some(message.clone()),
            }
            view.problem = Some(message);
            view.fields = fields;
            return view;
        }
    };
    let values = design.values();
    view.values = Some(values.as_slice().to_vec());
    // where the fidelity's stated error holds
    if let Some(m) = device
        .fidelities()
        .iter()
        .find(|m| m.fidelity == spec.fidelity)
    {
        for r in &m.validity {
            if let Some(v) = values.get(&r.name)
                && !(r.min..=r.max).contains(&v)
            {
                view.outside.push(r.name.clone());
            }
        }
    }
    match device.geometry(values) {
        Ok(g) => {
            view.ports = g
                .ports
                .iter()
                .map(|p| PortView {
                    name: p.name.clone(),
                    x: p.position.x.to_um(),
                    y: p.position.y.to_um(),
                    direction: p.direction,
                    width: p.width.to_um(),
                    layer: p.layer.clone(),
                })
                .collect();
        }
        Err(e) => view.problem = Some(text(e)),
    }
    match job_text(&spec, "design", "")
        .and_then(|t| Job::parse_as(&t, Format::Yaml).map_err(text))
        .and_then(|j| job::preview(&j).map_err(text))
    {
        Ok(scene) => view.scene = Some(scene),
        Err(e) => view.problem = view.problem.take().or(Some(e)),
    }
    view.fields = fields;
    view
}

/// What each field's expression uses and is used by: from the compiled expressions when they
/// all parse, else from the names that appear in them.
fn uses(fields: &mut [Field], parameters: Option<&expr::Parameters>) {
    let names: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
    let used: Vec<Vec<String>> = fields
        .iter()
        .map(|f| {
            let mut u: Vec<String> = match parameters
                .and_then(|p| Expr::parse(&f.expression, p.scope()).ok().map(|e| (p, e)))
            {
                Some((p, e)) => e
                    .slots()
                    .iter()
                    .map(|&s| p.scope().name(s).to_owned())
                    .filter(|n| names.contains(n))
                    .collect(),
                None => names
                    .iter()
                    .filter(|n| words(&f.expression).any(|w| w == n.as_str()))
                    .cloned()
                    .collect(),
            };
            u.retain(|n| *n != f.name);
            u.dedup();
            u
        })
        .collect();
    for (k, f) in fields.iter_mut().enumerate() {
        f.uses.clone_from(&used[k]);
        f.used_by = names
            .iter()
            .zip(&used)
            .filter(|(_, u)| u.contains(&names[k]))
            .map(|(n, _)| n.clone())
            .collect();
    }
}

/// The identifiers in an expression's text.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| {
            w.chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_')
        })
}

/// The form's fields, the design's values, ports and scene.
#[tauri::command]
pub fn device_geometry(spec: SpecIn) -> GeometryView {
    geometry(&spec)
}

// ---- figures and spectra ----

/// The components built lately, by design and fidelity: a solve's is kept for the spectrum
/// that follows it.
static BUILT: Mutex<Vec<(String, Arc<Built>)>> = Mutex::new(Vec::new());

/// How many components [`BUILT`] keeps.
const KEPT: usize = 8;

/// The device's component at `spec`'s values and `fidelity`, built or kept.
fn built(spec: &Spec, fidelity: Fidelity) -> Result<Arc<Built>, String> {
    let (device, design) = spec.build().map_err(text)?;
    let key = format!(
        "{:?}|{:?}|{}|{:?}",
        spec.kind,
        spec.stack,
        fidelity_name(fidelity),
        design.values().as_slice()
    );
    let lock = || {
        BUILT
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    };
    if let Some((_, b)) = lock().iter().find(|(k, _)| *k == key) {
        return Ok(b.clone());
    }
    let b = Arc::new(device.component(design.values(), fidelity).map_err(text)?);
    let mut kept = lock();
    kept.retain(|(k, _)| *k != key);
    kept.push((key, b.clone()));
    if kept.len() > KEPT {
        kept.remove(0);
    }
    Ok(b)
}

/// A figure, as the page shows it.
#[derive(Debug, PartialEq, Serialize)]
pub struct FigureView {
    name: String,
    symbol: String,
    value: f64,
    unit: String,
    fidelity: &'static str,
    grid: String,
}

/// The device's figures at a fidelity, and how long they took.
#[derive(Serialize)]
pub struct Derived {
    figures: Vec<FigureView>,
    fidelity: &'static str,
    seconds: f64,
}

/// The device's figures at `fidelity` (a solve's in the background: seconds).
///
/// # Errors
///
/// The library's message: an analytic fit outside its tables says so, and to solve.
pub fn derived(spec: &SpecIn, fidelity: &str) -> Result<Derived, String> {
    let started = Instant::now();
    let f = device::fidelity(fidelity).map_err(text)?;
    let b = built(&spec.spec()?, f)?;
    Ok(Derived {
        figures: b
            .figures
            .iter()
            .map(|g| FigureView {
                name: g.name.clone(),
                symbol: g.symbol.clone(),
                value: g.value,
                unit: g.unit.clone(),
                fidelity: fidelity_name(g.fidelity),
                grid: g.grid.clone(),
            })
            .collect(),
        fidelity: fidelity_name(f),
        seconds: started.elapsed().as_secs_f64(),
    })
}

#[tauri::command(async)]
pub fn device_derived(spec: SpecIn, fidelity: String) -> Result<Derived, String> {
    derived(&spec, &fidelity)
}

/// A device's S-parameters over wavelength, with its ports' modes and where they come from.
#[derive(Serialize)]
pub struct DeviceSpectrum {
    data: SpectrumData,
    ports: Vec<PortInfo>,
    fidelity: &'static str,
    /// The grid its figures name.
    grid: String,
    /// The component's provenance: what it reproduces, and on which grid.
    source: String,
    /// The wavelengths it holds for, µm, if limited.
    validity: Option<(f64, f64)>,
}

fn spectrum_of(
    spec: &SpecIn,
    fidelity: &str,
    sweep: Sweep,
) -> Result<(Spectrum, Arc<Built>), String> {
    let f = device::fidelity(fidelity).map_err(text)?;
    let b = built(&spec.spec()?, f)?;
    let spectrum =
        Spectrum::of(b.component.as_ref(), &sweep.wavelengths()?, &b.values).map_err(text)?;
    Ok((spectrum, b))
}

/// The device's S-parameters over `sweep` at `fidelity`.
pub fn spectrum(spec: &SpecIn, fidelity: &str, sweep: Sweep) -> Result<DeviceSpectrum, String> {
    let (s, b) = spectrum_of(spec, fidelity, sweep)?;
    let p = b.component.provenance();
    Ok(DeviceSpectrum {
        data: SpectrumData::of(&s),
        ports: ports_of(b.component.as_ref()),
        fidelity: fidelity_name(p.fidelity),
        grid: b
            .figures
            .first()
            .map(|f| f.grid.clone())
            .unwrap_or_default(),
        source: p.source,
        validity: p.validity,
    })
}

#[tauri::command(async)]
pub fn device_spectrum(
    spec: SpecIn,
    fidelity: String,
    sweep: Sweep,
) -> Result<DeviceSpectrum, String> {
    spectrum(&spec, &fidelity, sweep)
}

/// Writes the device's S-parameters over `sweep` at `fidelity` to `path`, as a Touchstone file.
#[tauri::command(async)]
pub fn device_touchstone(
    spec: SpecIn,
    fidelity: String,
    sweep: Sweep,
    path: String,
) -> Result<(), String> {
    let (s, _) = spectrum_of(&spec, &fidelity, sweep)?;
    write_touchstone(Path::new(&path), &s)
}

// ---- the jobs that compute a design (the Simulate tab) ----

/// How a job computes its design, as the Simulate tab's form holds it: [`job::Compute`].
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComputeIn {
    /// FDTD of the drawn structure, 2 or 3 dimensions; `None`, the device's own model.
    #[serde(default)]
    pub fdtd: Option<u8>,
    /// The band, µm: from, to, how many wavelengths.
    #[serde(default)]
    pub band: Option<(f64, f64, usize)>,
    #[serde(default)]
    pub step_nm: Option<f64>,
    #[serde(default)]
    pub step_z_nm: Option<f64>,
    #[serde(default)]
    pub margin_um: Option<f64>,
    #[serde(default)]
    pub samples: Option<usize>,
    #[serde(default)]
    pub pml_cells: Option<usize>,
    #[serde(default)]
    pub fdfd_2d: bool,
    #[serde(default)]
    pub timeout_minutes: Option<f64>,
}

impl ComputeIn {
    fn compute(&self) -> job::Compute {
        let mut c = match self.fdtd {
            Some(d) => job::Compute::fdtd(d),
            None => job::Compute::model(),
        };
        c.band = self.band;
        c.step_nm = self.step_nm;
        c.step_z_nm = self.step_z_nm;
        c.margin_um = self.margin_um;
        c.samples = self.samples;
        c.pml_cells = self.pml_cells;
        c.fdfd_2d = self.fdfd_2d;
        c.timeout_minutes = self.timeout_minutes;
        c
    }

    /// What the form holds of a device job's task (`None` for a design alone) and its timeout.
    fn of(task: &toml::Table, timeout: Option<f64>) -> Option<ComputeIn> {
        let num = |t: &toml::Table, k: &str| {
            t.get(k)
                .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
        };
        let count = |t: &toml::Table, k: &str| {
            t.get(k)
                .and_then(toml::Value::as_integer)
                .and_then(|i| usize::try_from(i).ok())
        };
        let band = |t: &toml::Table| match (num(t, "from_um"), num(t, "to_um")) {
            (Some(a), Some(b)) => Some((a, b, count(t, "points").unwrap_or(101))),
            _ => None,
        };
        let mut c = ComputeIn {
            step_nm: num(task, "step_nm"),
            pml_cells: count(task, "pml_cells"),
            timeout_minutes: timeout,
            ..ComputeIn::default()
        };
        match task.get("kind").and_then(toml::Value::as_str)? {
            "fdtd" => {
                c.fdtd = Some(count(task, "dimensions").map_or(2, |d| d as u8));
                c.band = task
                    .get("spectrum")
                    .and_then(toml::Value::as_table)
                    .and_then(band);
            }
            _ => {
                c.band = band(task);
                c.step_z_nm = num(task, "step_z_nm");
                c.margin_um = num(task, "margin_um");
                c.samples = count(task, "samples");
                c.fdfd_2d = task.get("model_2d").and_then(toml::Value::as_str) == Some("fdfd");
            }
        }
        Some(c)
    }
}

/// The text of the job that computes `spec` as `compute` says, named `name`
/// ([`job::from_device`]), with an opening comment saying what it computes and how to run it.
///
/// # Errors
///
/// What the library refuses: a grid the device doesn't take, a 3D solve too large, a band that
/// isn't one.
pub fn job_text_of(spec: &SpecIn, compute: &ComputeIn, name: &str) -> Result<String, String> {
    let s = spec.spec()?;
    let j = job::from_device(name, &s, &compute.compute()).map_err(text)?;
    let how = match compute.fdtd {
        Some(2) => "by 2D FDTD of its drawn structure (the effective index method's plane: a 2D \
                    estimate, not the device's performance)"
            .to_owned(),
        Some(_) => "by 3D FDTD of its drawn structure".to_owned(),
        None if compute.fdfd_2d => "by its 2D model, 2D FDFD (labelled 2D)".to_owned(),
        None => format!(
            "by its own {} model",
            if spec.fidelity == "analytic" {
                "analytic"
            } else {
                spec.fidelity.as_str()
            }
        ),
    };
    let body = j.to_text(Format::Yaml).map_err(text)?;
    Ok(format!(
        "# The {} {how}.\n#   photonoxide run jobs/{name}.yaml\n{body}",
        spec.kind
    ))
}

/// The job that computes `spec` as `compute` says, as its text.
#[tauri::command(async)]
pub fn device_job(spec: SpecIn, compute: ComputeIn, name: String) -> Result<String, String> {
    job_text_of(&spec, &compute, &name)
}

// ---- the studies of a design (the Study tab) ----

/// An axis of a sweep, as the Study tab's form holds it: [`job::SweepAxis`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AxisIn {
    pub parameter: String,
    pub from: f64,
    pub to: f64,
    pub points: usize,
}

/// A corner, as the Study tab's form holds it: [`job::Corner`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CornerIn {
    pub name: String,
    #[serde(default)]
    pub offsets: Vec<(String, f64)>,
    #[serde(default)]
    pub temperature_k: Option<f64>,
}

/// A study, as the Study tab's form holds it: [`job::Study`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StudyIn {
    /// `"sweep"` or `"corners"`.
    pub kind: String,
    #[serde(default)]
    pub axes: Vec<AxisIn>,
    #[serde(default)]
    pub corners: Vec<CornerIn>,
}

impl StudyIn {
    pub(crate) fn study(&self) -> Result<job::Study, String> {
        match self.kind.as_str() {
            "sweep" => Ok(job::Study::Sweep(
                self.axes
                    .iter()
                    .map(|a| job::SweepAxis::new(&a.parameter, a.from, a.to, a.points))
                    .collect(),
            )),
            "corners" => Ok(job::Study::Corners(
                self.corners
                    .iter()
                    .map(|c| {
                        let offsets: Vec<(&str, f64)> =
                            c.offsets.iter().map(|(n, v)| (n.as_str(), *v)).collect();
                        let corner = job::Corner::new(&c.name, &offsets);
                        match c.temperature_k {
                            Some(t) => corner.at_temperature(t),
                            None => corner,
                        }
                    })
                    .collect(),
            )),
            other => Err(format!("a study is a sweep or corners, not \"{other}\"")),
        }
    }

    /// The study a device job's `"sweep"` or `"corners"` task holds, as the form holds it.
    fn of(task: &toml::Table) -> Option<StudyIn> {
        let kind = task.get("kind").and_then(toml::Value::as_str)?;
        if kind != "sweep" && kind != "corners" {
            return None;
        }
        let num = |v: &toml::Value| v.as_float().or_else(|| v.as_integer().map(|i| i as f64));
        let tables = |key: &str| -> Vec<toml::Table> {
            task.get(key)
                .and_then(toml::Value::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_table().cloned()).collect())
                .unwrap_or_default()
        };
        let axes = tables("axis")
            .iter()
            .filter_map(|t| {
                Some(AxisIn {
                    parameter: t.get("parameter")?.as_str()?.to_owned(),
                    from: num(t.get("from")?)?,
                    to: num(t.get("to")?)?,
                    points: usize::try_from(t.get("points")?.as_integer()?).ok()?,
                })
            })
            .collect();
        let corners = tables("corner")
            .iter()
            .filter_map(|t| {
                Some(CornerIn {
                    name: t.get("name")?.as_str()?.to_owned(),
                    offsets: t
                        .get("offsets")
                        .and_then(toml::Value::as_table)
                        .map(|o| {
                            o.iter()
                                .filter_map(|(n, v)| Some((n.clone(), num(v)?)))
                                .collect()
                        })
                        .unwrap_or_default(),
                    temperature_k: t.get("temperature_k").and_then(num),
                })
            })
            .collect();
        Some(StudyIn {
            kind: kind.to_owned(),
            axes,
            corners,
        })
    }
}

/// The text of the job that studies `spec` as `study` says, each point computed as `compute`
/// says ([`job::study_from_device`]), named `name`, with an opening comment saying what it
/// studies and how to run it, farmed or not.
///
/// # Errors
///
/// What the library refuses: a parameter the design hasn't, a point out of a parameter's range,
/// a temperature at the analytic fidelity or on materials that take none, a 3D solve too large
/// at a point.
pub fn study_text_of(
    spec: &SpecIn,
    compute: &ComputeIn,
    study: &StudyIn,
    name: &str,
) -> Result<String, String> {
    let s = spec.spec()?;
    let mut c = compute.compute();
    c.fdtd = None;
    let j = job::study_from_device(name, &s, &c, &study.study()?).map_err(text)?;
    let what = match study.kind.as_str() {
        "sweep" => format!(
            "swept in {}",
            study
                .axes
                .iter()
                .map(|a| a.parameter.as_str())
                .collect::<Vec<_>>()
                .join(" and ")
        ),
        _ => format!("at {} corners and its nominal design", study.corners.len()),
    };
    let body = j.to_text(Format::Yaml).map_err(text)?;
    Ok(format!(
        "# The {} {what}, by its {} model at each point.\n#   photonoxide run jobs/{name}.yaml \
         [--headless] [--workers 4]\n{body}",
        spec.kind, spec.fidelity
    ))
}

/// The job that studies `spec`, as its text.
#[tauri::command(async)]
pub fn device_study_job(
    spec: SpecIn,
    compute: ComputeIn,
    study: StudyIn,
    name: String,
) -> Result<String, String> {
    study_text_of(&spec, &compute, &study, &name)
}

/// A device job's text, read back into the Simulate tab's form: its design and how it is
/// computed, and what of it the form can't hold.
#[derive(Debug, PartialEq, Serialize)]
pub struct JobForm {
    name: String,
    /// The design, when the text holds one.
    spec: Option<SpecIn>,
    /// How it is computed; `None` for a design alone.
    compute: Option<ComputeIn>,
    /// What the text holds that the form can't, as `task.<key>` (or a table's name): the job
    /// runs as its text says, and the form says so.
    beyond: Vec<String>,
    /// Why it isn't a job the library runs, if it isn't.
    error: Option<String>,
    /// A model that is an FDFD: its cells along x and y and the unknowns of a solve.
    size: Option<(usize, usize, f64)>,
    /// A study's sweep or corners, for a job whose task studies the design.
    study: Option<StudyIn>,
    /// An optimization, for a job whose task optimizes the design.
    optimize: Option<OptimizeIn>,
}

/// Whether two values are the same, a number the same whether written as an integer or a float.
fn same(a: &toml::Value, b: &toml::Value) -> bool {
    use toml::Value as V;
    match (a, b) {
        (V::Integer(x), V::Float(y)) | (V::Float(y), V::Integer(x)) => *x as f64 == *y,
        (V::Float(x), V::Float(y)) => x.to_bits() == y.to_bits(),
        (V::Array(x), V::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (V::Table(x), V::Table(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

/// Reads a device job's text into the Simulate tab's form.
pub fn read_job_form(text: &str) -> JobForm {
    let mut form = JobForm {
        name: String::new(),
        spec: None,
        compute: None,
        beyond: Vec::new(),
        error: None,
        size: None,
        study: None,
        optimize: None,
    };
    let job = match crate::studio::parse_job(text) {
        Ok(j) => j,
        Err(e) => {
            form.error = Some(text_of(e));
            return form;
        }
    };
    form.name = job.name().to_owned();
    let Some(table) = job.device() else {
        form.error = Some(format!(
            "{} isn't a device's job: it has no [device] table",
            job.name()
        ));
        return form;
    };
    let spec = match Spec::from_table(table) {
        Ok(s) => s,
        Err(e) => {
            form.error = Some(text_of(e));
            return form;
        }
    };
    let file = crate::studio::job_table(text);
    let timeout = file
        .get("timeout_minutes")
        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)));
    form.spec = Some(SpecIn::of(&spec));
    form.compute = ComputeIn::of(job.task(), timeout);
    for key in file.keys() {
        if !["name", "timeout_minutes", "device", "task"].contains(&key.as_str()) {
            form.beyond.push(key.clone());
        }
    }
    if let Err(e) = job::check(&job) {
        form.error = Some(text_of(e));
        return form;
    }
    form.size = job::fdfd_size(&job).ok().flatten();
    form.study = StudyIn::of(job.task());
    form.optimize = OptimizeIn::of(job.task());
    // what the form would write: any key of the task it writes otherwise is beyond it
    if let Some(compute) = &form.compute {
        let again = match (&form.study, &form.optimize) {
            (Some(study), _) => study.study().and_then(|s| {
                job::study_from_device(job.name(), &spec, &compute.compute(), &s).map_err(text_of)
            }),
            (None, Some(o)) => optimizing::optimize_job_of(job.name(), &spec, compute, o),
            (None, None) => {
                job::from_device(job.name(), &spec, &compute.compute()).map_err(text_of)
            }
        };
        match again {
            Ok(again) => {
                let mine = again.task();
                for (k, v) in job.task() {
                    if mine.get(k).is_none_or(|w| !same(v, w)) {
                        form.beyond.push(format!("task.{k}"));
                    }
                }
            }
            Err(e) => form.error = Some(e),
        }
    }
    form
}

/// A device job's text read into the Simulate tab's form.
#[tauri::command(async)]
pub fn device_job_read(text: String) -> JobForm {
    read_job_form(&text)
}

// ---- the design as the solver sees it ----

/// A design's permittivity on the cells the model chosen solves on: from above at the core's
/// middle, and on the cut; what the page's "As solved" view shows.
#[derive(Debug, Serialize)]
pub struct Solved {
    /// From above, at the core's middle; `None` for a model that solves a cross-section only.
    top: Option<photonoxide::raster::Raster>,
    /// On the cut, across it and up.
    cut: Option<photonoxide::raster::Raster>,
    /// The cells' size in the plane and up, nm, as the solver takes them.
    step_nm: f64,
    step_z_nm: f64,
    /// How many times coarser than the solver's the pictures' cells are (1: its own).
    coarser: f64,
    /// What the solver sees, in a sentence.
    note: String,
}

/// The most cells a picture of [`Solved`] has: past it, its cells are coarser than the solver's,
/// and it says so.
const MOST_PIXELS: f64 = 4e5;

/// `spec` as the model `compute` chooses solves it: its cells, from above and on the cut at
/// `at` µm along `normal` (`"x"` or `"y"`).
///
/// # Errors
///
/// The design's, and a cut that isn't one.
pub fn solved(spec: &SpecIn, compute: &ComputeIn, normal: &str, at: f64) -> Result<Solved, String> {
    use photonoxide::geometry::Point;
    use photonoxide::raster::Raster;
    use photonoxide::units::{Length, Wavelength};
    let s = spec.spec()?;
    let (device, design) = s.build().map_err(text)?;
    let values = design.values();
    let g = device.geometry(values).map_err(text)?;
    let core = g
        .ports
        .first()
        .map_or_else(|| "Si".to_owned(), |p| p.layer.clone());
    let (_, bottom, top) = g
        .structure
        .stack()
        .layer(&core)
        .ok_or_else(|| format!("the stack has no layer {core}"))?;
    let (bottom, top) = (bottom.to_um(), top.to_um());
    let wavelength = Wavelength::um(values.get("wavelength").unwrap_or(1.55)).map_err(text)?;
    // the window: the shapes and ports with 1 µm around them, 1 µm below and above the core
    let (mut x, mut y) = (
        [f64::INFINITY, f64::NEG_INFINITY],
        [f64::INFINITY, f64::NEG_INFINITY],
    );
    let mut take = |p: Point| {
        x = [x[0].min(p.x.to_um()), x[1].max(p.x.to_um())];
        y = [y[0].min(p.y.to_um()), y[1].max(p.y.to_um())];
    };
    for l in g.structure.stack().layers() {
        for shape in g.structure.shapes(&l.name) {
            let b = shape.bounds();
            take(b.min);
            take(b.max);
        }
    }
    for p in &g.ports {
        take(p.position);
    }
    if !x[0].is_finite() {
        return Err("the design draws nothing".into());
    }
    let (x, y, z) = (
        [x[0] - 1.0, x[1] + 1.0],
        [y[0] - 1.0, y[1] + 1.0],
        [bottom - 1.0, top + 1.0],
    );
    // the cells: the model's
    let fdfd = matches!(s.kind.as_str(), "MMI 1x2" | "MMI 2x2" | "Y-branch")
        && s.fidelity != Fidelity::Analytic
        && !(s.fidelity == Fidelity::TwoD && s.kind.starts_with("MMI") && !compute.fdfd_2d);
    let three = s.fidelity == Fidelity::ThreeD;
    let (step, step_z, plane, note) = if let Some(d) = compute.fdtd {
        let h = compute.step_nm.unwrap_or(if d == 3 { 50.0 } else { 40.0 }) / 1e3;
        let note = if d == 3 {
            "3D FDTD's Yee cells on the stack".to_owned()
        } else {
            "2D FDTD's cells in the core's plane: each holds its slab's effective index squared, \
             not the permittivity drawn here (the effective index method)"
                .to_owned()
        };
        (h, h, true, note)
    } else if fdfd {
        let h = compute
            .step_nm
            .map_or(if three { 0.04 } else { 0.02 }, |v| v / 1e3);
        let hz = compute
            .step_z_nm
            .map_or(h / if three { 2.0 } else { 4.0 }, |v| v / 1e3);
        let note = if three {
            "3D FDFD's cells on the stack, each its 8 × 8 samples' average".to_owned()
        } else {
            "2D FDFD's cells in the core's plane: each holds its slab's effective index squared, \
             not the permittivity drawn here (the effective index method)"
                .to_owned()
        };
        (h, hz, true, note)
    } else if s.fidelity == Fidelity::Analytic {
        return Ok(Solved {
            top: None,
            cut: None,
            step_nm: 0.0,
            step_z_nm: 0.0,
            coarser: 1.0,
            note: "a closed form, or a fit to the 3D solver: this model solves no grid".into(),
        });
    } else {
        let h = compute
            .step_nm
            .map_or(if s.kind == "bend" { 0.02 } else { 0.01 }, |v| v / 1e3);
        (
            h,
            h,
            false,
            "the mode solver's cross-section: cells fitted to the core about this far apart, its \
             faces on nodes"
                .into(),
        )
    };
    let picture =
        |a: [f64; 2], b: [f64; 2], ha: f64, hb: f64, at: &dyn Fn(f64, f64) -> (Point, f64)| {
            let mut ha = ha;
            let mut hb = hb;
            let cells = ((a[1] - a[0]) / ha) * ((b[1] - b[0]) / hb);
            let coarser = (cells / MOST_PIXELS).sqrt().max(1.0).ceil();
            ha *= coarser;
            hb *= coarser;
            let na = (((a[1] - a[0]) / ha).round() as usize).max(1);
            let nb = (((b[1] - b[0]) / hb).round() as usize).max(1);
            let mut v = Vec::with_capacity(na * nb);
            for j in 0..nb {
                for i in 0..na {
                    let (p, zz) = at(
                        a[0] + (i as f64 + 0.5) * (a[1] - a[0]) / na as f64,
                        b[0] + (j as f64 + 0.5) * (b[1] - b[0]) / nb as f64,
                    );
                    let m = g.structure.material_at(p, Length::um(zz));
                    v.push(m.permittivity(wavelength).map_err(text)?.re as f32);
                }
            }
            Ok::<_, String>((
                Raster {
                    nx: na,
                    ny: nb,
                    x0: a[0],
                    x1: a[1],
                    y0: b[0],
                    y1: b[1],
                    values: v,
                },
                coarser,
            ))
        };
    let mid = 0.5 * (bottom + top);
    let top_view = if plane {
        Some(picture(x, y, step, step, &|a, b| (Point::um(a, b), mid))?)
    } else {
        None
    };
    if !at.is_finite() {
        return Err(format!("the cut must be at a finite {normal}"));
    }
    // a 2D model (by the effective index method) solves no cut: its plane is all it sees
    let flat = compute.fdtd == Some(2) || (fdfd && !three);
    let cut = if flat {
        None
    } else if normal == "x" {
        Some(picture(y, z, step, step_z, &|a, b| (Point::um(at, a), b))?)
    } else {
        Some(picture(x, z, step, step_z, &|a, b| (Point::um(a, at), b))?)
    };
    let coarser = [top_view.as_ref().map(|t| t.1), cut.as_ref().map(|c| c.1)]
        .into_iter()
        .flatten()
        .fold(1.0, f64::max);
    Ok(Solved {
        top: top_view.map(|t| t.0),
        cut: cut.map(|c| c.0),
        step_nm: step * 1e3,
        step_z_nm: if flat { step * 1e3 } else { step_z * 1e3 },
        coarser,
        note,
    })
}

/// The design as the solver sees it.
#[tauri::command(async)]
pub fn device_solved(
    spec: SpecIn,
    compute: ComputeIn,
    normal: String,
    at: f64,
) -> Result<Solved, String> {
    solved(&spec, &compute, &normal, at)
}

// ---- saved designs ----

/// A design saved as a job file.
#[derive(Debug, PartialEq, Serialize)]
pub struct Saved {
    spec: SpecIn,
    name: String,
    path: String,
    about: String,
    /// The whole text, when the job computes the design (it has a task): the page's Simulate
    /// tab opens on it.
    job: Option<String>,
}

/// The job file's text for `spec` under `name`, checked as the library checks a job.
///
/// # Errors
///
/// A message for a name that isn't a job's, or a design the library refuses.
pub fn saved_text(spec: &SpecIn, name: &str, about: &str) -> Result<String, String> {
    saved_text_in(spec, name, about, None)
}

/// [`saved_text`] for a design to be saved in `dir`, whose compact model's file it names
/// relative to there.
fn saved_text_in(
    spec: &SpecIn,
    name: &str,
    about: &str,
    dir: Option<&std::path::Path>,
) -> Result<String, String> {
    let text = job_text(&spec.spec()?, name, about)?;
    let mut job = Job::parse_as(&text, Format::Yaml).map_err(text_of)?;
    if let Some(dir) = dir {
        job = job.in_dir(dir);
    }
    job::check(&job).map_err(text_of)?;
    Ok(text)
}

fn text_of(e: photonoxide::Error) -> String {
    e.to_string()
}

/// The design a job file's text holds.
///
/// # Errors
///
/// A message for a job that isn't a device's.
pub fn read_text(text: &str) -> Result<(SpecIn, String, String), String> {
    let job = crate::studio::parse_job(text).map_err(text_of)?;
    let table = job
        .device()
        .ok_or_else(|| format!("{} isn't a device's design", job.name()))?;
    let spec = Spec::from_table(table).map_err(text_of)?;
    let about = text
        .lines()
        .map_while(|l| l.strip_prefix('#'))
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n");
    Ok((SpecIn::of(&spec), job.name().to_owned(), about))
}

/// Saves `spec` as `jobs/<name>.yaml` in the workspace; refuses to replace another file
/// unless `replace`. A design saved before in TOML, `jobs/<name>.toml`, is that other file: on
/// `replace` the YAML takes its place.
#[tauri::command]
pub fn save_device(
    state: tauri::State<'_, Studio>,
    spec: SpecIn,
    name: String,
    about: String,
    replace: bool,
) -> Result<String, String> {
    // checked as it is, before anything is written
    saved_text(&spec, &name, &about)?;
    let dir = state.workspace().join("jobs");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{name}.yaml"));
    let old = dir.join(format!("{name}.toml"));
    match [&path, &old].into_iter().find(|p| p.exists()) {
        Some(there) if !replace => return Err(format!("exists: {}", there.display())),
        _ => {}
    }
    // its compact model kept next to it, as <name>.compact.json
    let kept = compact::kept_beside(&spec, &dir, &name)?;
    let text = saved_text_in(&kept, &name, &about, Some(&dir))?;
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    if old.exists() {
        std::fs::remove_file(&old).map_err(|e| format!("{}: {e}", old.display()))?;
    }
    Ok(path.display().to_string())
}

/// The design saved in the job file at `path`.
#[tauri::command]
pub fn read_device(path: String) -> Result<Saved, String> {
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("{path}: {e}"))?
        .replace("\r\n", "\n");
    let (spec, name, about) = read_text(&text)?;
    let spec = match std::path::Path::new(&path).parent() {
        Some(dir) => compact::found_from(spec, dir),
        None => spec,
    };
    let computes = crate::studio::parse_job(&text).is_ok_and(|j| !j.task().is_empty());
    Ok(Saved {
        spec,
        name,
        path,
        about,
        job: computes.then_some(text),
    })
}

pub mod compact;
pub mod optimizing;
pub use optimizing::OptimizeIn;

#[cfg(test)]
mod tests;
