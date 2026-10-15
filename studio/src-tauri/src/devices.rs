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
use photonoxide::run::Job;
use serde::{Deserialize, Serialize};

use crate::circuits::{PortInfo, SpectrumData, Sweep, ports_of, write_touchstone};
use crate::studio::Studio;

/// Each device: the component library's id it stands for, its Academy lesson, and the
/// published results (examples/) on it.
pub const DEVICES: [(&str, &str, &str, &[&str]); 11] = [
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
];

fn text(e: photonoxide::Error) -> String {
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
struct MethodInfo {
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
                fidelities: d
                    .fidelities()
                    .iter()
                    .map(|m| MethodInfo {
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
                    })
                    .collect(),
                ports,
                lesson,
                examples: examples.to_vec(),
            })
        })
        .collect()
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
        }
    }
}

/// The text of the job file that holds `spec` under `name`, with `about` as its opening
/// comment.
fn job_text(spec: &Spec, name: &str, about: &str) -> Result<String, String> {
    let mut file = toml::Table::new();
    file.insert("name".into(), toml::Value::String(name.to_owned()));
    file.insert("device".into(), toml::Value::Table(spec.to_table()));
    let body = toml::to_string(&file).map_err(|e| e.to_string())?;
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
        .and_then(|t| Job::parse(&t).map_err(text))
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

// ---- saved designs ----

/// A design saved as a job file.
#[derive(Debug, PartialEq, Serialize)]
pub struct Saved {
    spec: SpecIn,
    name: String,
    path: String,
    about: String,
}

/// The job file's text for `spec` under `name`, checked as the library checks a job.
///
/// # Errors
///
/// A message for a name that isn't a job's, or a design the library refuses.
pub fn saved_text(spec: &SpecIn, name: &str, about: &str) -> Result<String, String> {
    let text = job_text(&spec.spec()?, name, about)?;
    let job = Job::parse(&text).map_err(text_of)?;
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
    let job = Job::parse(text).map_err(text_of)?;
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

/// Saves `spec` as `jobs/<name>.toml` in the workspace; refuses to replace another file
/// unless `replace`.
#[tauri::command]
pub fn save_device(
    state: tauri::State<'_, Studio>,
    spec: SpecIn,
    name: String,
    about: String,
    replace: bool,
) -> Result<String, String> {
    let text = saved_text(&spec, &name, &about)?;
    let dir = state.workspace().join("jobs");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{name}.toml"));
    if path.exists() && !replace {
        return Err(format!("exists: {}", path.display()));
    }
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path.display().to_string())
}

/// The design saved in the job file at `path`.
#[tauri::command]
pub fn read_device(path: String) -> Result<Saved, String> {
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("{path}: {e}"))?
        .replace("\r\n", "\n");
    let (spec, name, about) = read_text(&text)?;
    Ok(Saved {
        spec,
        name,
        path,
        about,
    })
}

#[cfg(test)]
mod tests;
