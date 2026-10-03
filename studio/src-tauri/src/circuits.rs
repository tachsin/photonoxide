//! The Components and Chip pages' commands: the component library, a component's spectrum, and
//! circuits: chip files read and written, checked step by step and simulated.
//!
//! A **chip file** (TOML, in the workspace's `circuits/`; the format is in studio/README.md) is a
//! netlist with the chip view's drawing: each instance's kind (a library id), its parameters'
//! values and its place on the canvas; the connections; the external ports with their names and
//! places; and the wavelengths to simulate. [`Chip::netlist`] builds the library's
//! [`Netlist`] from it, and [`Chip::from_netlist`] a chip from a netlist, so the two round-trip.
//!
//! The library does the physics: the studio only builds netlists and hands them to
//! `Netlist::compile`, and shows what comes back.

pub mod library;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use photonoxide::Error;
use photonoxide::circuit::{Component, Fidelity, Netlist, NetlistError, SMatrix, Spectrum};
use photonoxide::compact::touchstone::{Convention, Precision, Touchstone};
use photonoxide::units::Wavelength;
use serde::{Deserialize, Serialize};

use crate::studio::Studio;
use library::{Symbol, symbol};

/// The format chip files are written in; files of a later format are refused.
pub const FORMAT: u32 = 1;

/// What a model is built from: a kind, and a measured component's file and time convention.
type Key<'a> = (&'a str, Option<&'a str>, Option<TimeConvention>);

/// A circuit as the chip view draws it: a netlist, and where everything sits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chip {
    #[serde(default = "format")]
    pub format: u32,
    /// Its name, and the file's: `circuits/<name>.toml`.
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub about: String,
    /// The wavelengths it is simulated at.
    #[serde(default)]
    pub sweep: Sweep,
    /// Connections, each two ports written `instance.port`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connections: Vec<[String; 2]>,
    #[serde(default, rename = "instance")]
    pub instances: Vec<Placed>,
    #[serde(default, rename = "port")]
    pub ports: Vec<External>,
}

fn format() -> u32 {
    FORMAT
}

/// Wavelengths evenly spaced from `from_um` to `to_um`, `points` of them.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sweep {
    pub from_um: f64,
    pub to_um: f64,
    pub points: usize,
}

impl Default for Sweep {
    fn default() -> Sweep {
        Sweep {
            from_um: 1.5,
            to_um: 1.6,
            points: 501,
        }
    }
}

impl Sweep {
    /// The most points a sweep may take.
    pub const MOST: usize = 20_001;

    /// Its wavelengths.
    ///
    /// # Errors
    ///
    /// A message for a range that runs backwards, a wavelength that isn't positive, or fewer than
    /// two or more than [`Sweep::MOST`] points.
    pub fn wavelengths(&self) -> Result<Vec<Wavelength>, String> {
        if !(self.from_um.is_finite() && self.to_um.is_finite() && self.to_um > self.from_um) {
            return Err(format!(
                "the wavelengths must run upwards: from {} to {} µm",
                self.from_um, self.to_um
            ));
        }
        if !(2..=Sweep::MOST).contains(&self.points) {
            return Err(format!(
                "a sweep takes 2 to {} points, not {}",
                Sweep::MOST,
                self.points
            ));
        }
        let step = (self.to_um - self.from_um) / (self.points - 1) as f64;
        (0..self.points)
            .map(|k| Wavelength::um(self.from_um + step * k as f64).map_err(|e| e.to_string()))
            .collect()
    }
}

/// An instance on the chip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placed {
    pub name: String,
    /// Its kind: a library id (`library::entries`).
    pub kind: String,
    /// Its symbol's centre on the canvas.
    pub x: f64,
    pub y: f64,
    /// Turned clockwise on the canvas by this many degrees: 0, 90, 180 or 270.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation: u16,
    /// Mirrored top to bottom before it is turned.
    #[serde(default, skip_serializing_if = "is_false")]
    pub mirror: bool,
    /// Its parameters' values by name; a parameter not given takes its default.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, f64>,
    /// A measured component's Touchstone file (kind [`MEASURED`]): relative to the workspace,
    /// or absolute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// The time convention of that file's values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convention: Option<TimeConvention>,
}

/// The kind of a measured component: S-parameters read from a Touchstone file.
pub const MEASURED: &str = "touchstone";

/// The time convention a Touchstone file's complex values are in: the specification doesn't
/// say, so the file's reader must.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeConvention {
    /// Fields as e^(−iωt): photonoxide's, and what its Touchstone files are written in.
    Physics,
    /// Fields as e^(+jωt): microwave engineering's, and most RF tools'.
    Engineering,
}

impl From<TimeConvention> for Convention {
    fn from(c: TimeConvention) -> Convention {
        match c {
            TimeConvention::Physics => Convention::Physics,
            TimeConvention::Engineering => Convention::Engineering,
        }
    }
}

/// `file` resolved against the workspace `root` when it is relative.
fn resolve(root: &Path, file: &str) -> PathBuf {
    let path = Path::new(file);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

/// `path` relative to the workspace `root` (with forward slashes) when it is inside it, so a
/// chip file moves with its workspace; else as it is.
fn relative(root: &Path, path: &Path) -> String {
    let inside = root
        .canonicalize()
        .ok()
        .zip(path.canonicalize().ok())
        .and_then(|(r, p)| p.strip_prefix(r).ok().map(Path::to_path_buf));
    match inside {
        Some(rel) => rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
        None => path.display().to_string(),
    }
}

/// The component an instance of `kind` is: the library's entry, or for [`MEASURED`] the
/// Touchstone `file` (resolved against `root`) read in `convention`.
fn build(
    kind: &str,
    file: Option<&str>,
    convention: Option<TimeConvention>,
    root: &Path,
) -> Result<Arc<dyn Component>, String> {
    if kind == MEASURED {
        let file = file.ok_or("a measured component needs its Touchstone file")?;
        let convention =
            convention.ok_or("a measured component needs its file's time convention")?;
        return library::measured(&resolve(root, file), convention.into());
    }
    let entry =
        library::entry(kind).ok_or_else(|| format!("no component {kind} in the library"))?;
    (entry.build)()
}

/// An external port: a port of the circuit, exposing one of an instance's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct External {
    pub name: String,
    /// The port it exposes, `instance.port`; empty while it isn't wired.
    #[serde(default)]
    pub at: String,
    pub x: f64,
    pub y: f64,
    /// The direction its tag points, as an instance's rotation.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation: u16,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde's skip_serializing_if passes a reference
fn is_zero(v: &u16) -> bool {
    *v == 0
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(v: &bool) -> bool {
    !*v
}

/// Something that keeps a chip from being simulated, with what it points at so the chip view
/// can mark it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Problem {
    pub message: String,
    /// The instance it is about.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    /// The port it is about, `instance.port`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    /// The connection it is about, by its position in the chip's list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<usize>,
    /// The external port it is about, by its position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<usize>,
    /// The parameter it is about.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter: Option<String>,
    /// Whether it is a port left dangling: what an unfinished chip has, rather than a mistake.
    pub dangling: bool,
}

impl Problem {
    fn of(e: &Error) -> Problem {
        let mut p = Problem::message(e.to_string());
        if let Error::Netlist(n) = e {
            p.message = n.to_string();
            match n {
                NetlistError::UnknownPort(r)
                | NetlistError::SelfConnection(r)
                | NetlistError::PortUsedTwice { port: r, .. } => {
                    p.instance = Some(r.instance.clone());
                    p.port = Some(r.to_string());
                }
                NetlistError::Dangling(r) => {
                    p.instance = Some(r.instance.clone());
                    p.port = Some(r.to_string());
                    p.dangling = true;
                }
                NetlistError::ModeMismatch { a, .. } => p.port = Some(a.to_string()),
                NetlistError::UnknownParameter {
                    instance,
                    parameter,
                }
                | NetlistError::ValueOutOfRange {
                    instance,
                    parameter,
                    ..
                } => {
                    p.instance = Some(instance.clone());
                    p.parameter = Some(parameter.clone());
                }
                NetlistError::InvalidComponent { instance, .. }
                | NetlistError::DuplicateInstance(instance)
                | NetlistError::UnknownInstance(instance)
                | NetlistError::SizeMismatch { instance, .. } => {
                    p.instance = Some(instance.clone());
                }
                _ => {}
            }
        }
        p
    }

    fn message(message: String) -> Problem {
        Problem {
            message,
            instance: None,
            port: None,
            connection: None,
            external: None,
            parameter: None,
            dangling: false,
        }
    }
}

impl Chip {
    /// The chip in a chip file's text.
    ///
    /// # Errors
    ///
    /// A message for TOML that isn't a chip, or a chip of a later format.
    pub fn parse(text: &str) -> Result<Chip, String> {
        let chip: Chip = toml::from_str(text).map_err(|e| e.to_string())?;
        if chip.format > FORMAT {
            return Err(format!(
                "a chip of format {}, newer than this program's ({FORMAT}): update photonoxide",
                chip.format
            ));
        }
        Ok(chip)
    }

    /// The chip as a chip file's text.
    ///
    /// # Errors
    ///
    /// A message if a value can't be written in TOML (a value that isn't finite).
    pub fn to_text(&self) -> Result<String, String> {
        // the connections one to a line, before the tables, rather than all on one line
        let rest = Chip {
            connections: Vec::new(),
            ..self.clone()
        };
        let body = toml::to_string(&rest).map_err(|e| e.to_string())?;
        if self.connections.is_empty() {
            return Ok(body);
        }
        let quote = |s: &str| toml::Value::String(s.into()).to_string();
        let mut block = String::from(
            "connections = [
",
        );
        for [a, b] in &self.connections {
            block += &format!(
                "    [{}, {}],
",
                quote(a),
                quote(b)
            );
        }
        block += "]
";
        Ok(
            match body.find(
                "
[",
            ) {
                Some(k) => format!(
                    "{}

{block}
{}",
                    body[..k].trim_end(),
                    &body[k + 1..]
                ),
                None => format!(
                    "{body}
{block}"
                ),
            },
        )
    }

    /// The library's netlist of this chip, built a step at a time, and everything that went
    /// wrong on the way, each step's error with what it points at, then every port left
    /// dangling. A step that fails is left out, and the rest go on, so every problem shows at
    /// once. Measured components' files are resolved against the workspace `root`.
    pub fn netlist(&self, root: &Path) -> (Netlist, Vec<Problem>) {
        let mut netlist = Netlist::new();
        let mut problems = Vec::new();
        // one model per kind (and file), shared by its instances
        let mut models: HashMap<Key, Arc<dyn Component>> = HashMap::new();
        let mut missing = HashSet::new();
        for inst in &self.instances {
            let key = (inst.kind.as_str(), inst.file.as_deref(), inst.convention);
            let model = match models.get(&key) {
                Some(m) => Ok(m.clone()),
                None => build(key.0, key.1, key.2, root),
            };
            let model = match model {
                Ok(m) => m,
                Err(message) => {
                    problems.push(Problem {
                        instance: Some(inst.name.clone()),
                        ..Problem::message(format!("{}: {message}", inst.name))
                    });
                    missing.insert(inst.name.as_str());
                    continue;
                }
            };
            models.insert(key, model.clone());
            if let Err(e) = netlist.add(&inst.name, model) {
                let mut p = Problem::of(&e);
                p.instance = Some(inst.name.clone());
                problems.push(p);
                missing.insert(inst.name.as_str());
                continue;
            }
            for (parameter, &value) in &inst.values {
                if let Err(e) = netlist.set(&inst.name, parameter, value) {
                    problems.push(Problem::of(&e));
                }
            }
        }
        // a step that names an instance that couldn't be added has already been reported
        let gone = |r: &str| r.split_once('.').is_some_and(|(i, _)| missing.contains(i));
        for (k, [a, b]) in self.connections.iter().enumerate() {
            if gone(a) || gone(b) {
                continue;
            }
            if let Err(e) = netlist.connect(a, b) {
                problems.push(Problem {
                    connection: Some(k),
                    ..Problem::of(&e)
                });
            }
        }
        for (k, port) in self.ports.iter().enumerate() {
            if port.at.is_empty() {
                problems.push(Problem {
                    external: Some(k),
                    ..Problem::message(format!(
                        "external port {} isn't wired to anything",
                        port.name
                    ))
                });
                continue;
            }
            if gone(&port.at) {
                continue;
            }
            if let Err(e) = netlist.expose(&port.name, &port.at) {
                problems.push(Problem {
                    external: Some(k),
                    ..Problem::of(&e)
                });
            }
        }
        problems.extend(
            netlist
                .problems()
                .iter()
                .map(|e| Problem::of(&e.clone().into())),
        );
        (netlist, problems)
    }
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the format's other direction, from a netlist, which only the tests take yet"
    )
)]
impl Chip {
    /// A chip with nothing on it.
    pub fn new(name: &str) -> Chip {
        Chip {
            format: FORMAT,
            name: name.into(),
            about: String::new(),
            sweep: Sweep::default(),
            connections: Vec::new(),
            instances: Vec::new(),
            ports: Vec::new(),
        }
    }

    /// A chip of `netlist`, named `name`: each instance's entry found in the library
    /// (`library::entry_of`), and everything laid out in a row, left to right, the external
    /// ports beside the ports they expose.
    ///
    /// # Errors
    ///
    /// A message for an instance whose component isn't in the library.
    pub fn from_netlist(name: &str, netlist: &Netlist) -> Result<Chip, String> {
        let mut chip = Chip::new(name);
        let mut places = HashMap::new();
        for (k, inst) in netlist.instances().iter().enumerate() {
            let entry = library::entry_of(&inst.component).ok_or_else(|| {
                format!(
                    "{}: no component of kind {} in the library",
                    inst.name,
                    inst.component.kind()
                )
            })?;
            let (x, y) = (140.0 * k as f64, 0.0);
            places.insert(inst.name.as_str(), (x, y, entry.glyph));
            chip.instances.push(Placed {
                name: inst.name.clone(),
                kind: entry.id.into(),
                x,
                y,
                rotation: 0,
                mirror: false,
                file: None,
                convention: None,
                values: inst
                    .component
                    .parameters()
                    .iter()
                    .zip(&inst.values)
                    .map(|(p, &v)| (p.name.clone(), v))
                    .collect(),
            });
        }
        chip.connections = netlist
            .connections()
            .iter()
            .map(|(a, b)| [a.to_string(), b.to_string()])
            .collect();
        for (name, r) in netlist.external() {
            let (x, y, glyph) = places[r.instance.as_str()];
            let inst = netlist
                .instance(&r.instance)
                .expect("an exposed port's instance");
            let pin = symbol(glyph, inst.component.ports())
                .pins
                .into_iter()
                .find(|p| p.port == r.port)
                .expect("a pin per port");
            let (dx, dy) = (pin.angle.to_radians().cos(), pin.angle.to_radians().sin());
            chip.ports.push(External {
                name: name.clone(),
                at: r.to_string(),
                x: x + pin.x + 40.0 * dx,
                y: y + pin.y + 40.0 * dy,
                rotation: if dx < -0.5 { 0 } else { 180 },
            });
        }
        Ok(chip)
    }
}

// ---- the library ----

/// A kind of component, as the Components page and the chip view's parts show it.
#[derive(Serialize)]
pub struct KindInfo {
    id: String,
    title: String,
    variant: String,
    category: String,
    about: String,
    equation: String,
    /// A measured component's file, as a chip file names it, and its time convention.
    file: Option<String>,
    convention: Option<TimeConvention>,
    /// What the component calls itself (`Component::kind`).
    kind: String,
    ports: Vec<PortInfo>,
    parameters: Vec<ParameterInfo>,
    provenance: ProvenanceInfo,
    reciprocal: bool,
    symbol: Symbol,
}

#[derive(Serialize)]
struct PortInfo {
    name: String,
    /// "TE0 at 1.55 µm, n_eff 2.44, n_g 4.18", when the component states it.
    mode: Option<ModeInfo>,
}

#[derive(Serialize)]
struct ModeInfo {
    polarization: String,
    order: usize,
    effective_index: f64,
    group_index: Option<f64>,
    wavelength_um: f64,
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
struct ProvenanceInfo {
    fidelity: &'static str,
    source: String,
    error: Option<f64>,
    validity: Option<(f64, f64)>,
}

fn fidelity(f: Fidelity) -> &'static str {
    match f {
        Fidelity::Analytic => "analytic",
        Fidelity::Compact => "compact",
        Fidelity::TwoD => "2D",
        Fidelity::ThreeD => "3D",
        Fidelity::Measured => "measured",
    }
}

fn info(entry: &library::Entry) -> Result<KindInfo, String> {
    let c = (entry.build)().map_err(|e| format!("{}: {e}", entry.id))?;
    let mut info = describe(c.as_ref(), entry.glyph);
    info.id = entry.id.into();
    info.title = entry.title.into();
    info.variant = entry.variant.into();
    info.category = entry.category.into();
    info.about = entry.about.into();
    info.equation = entry.equation.into();
    Ok(info)
}

/// What the library says of component `c`, drawn as `glyph`; the studio's words left empty.
fn describe(c: &dyn Component, glyph: library::Glyph) -> KindInfo {
    let p = c.provenance();
    KindInfo {
        id: String::new(),
        title: String::new(),
        variant: String::new(),
        category: String::new(),
        about: String::new(),
        equation: String::new(),
        file: None,
        convention: None,
        kind: c.kind().into(),
        ports: c
            .ports()
            .iter()
            .map(|p| PortInfo {
                name: p.name.clone(),
                mode: p.mode.as_ref().map(|m| ModeInfo {
                    polarization: format!("{:?}", m.polarization),
                    order: m.order,
                    effective_index: m.effective_index,
                    group_index: m.group_index,
                    wavelength_um: m.wavelength.to_um(),
                }),
            })
            .collect(),
        parameters: c
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
        provenance: ProvenanceInfo {
            fidelity: fidelity(p.fidelity),
            source: p.source,
            error: p.error,
            validity: p.validity,
        },
        reciprocal: c.reciprocal(),
        symbol: symbol(glyph, c.ports()),
    }
}

/// The measured component in the Touchstone file at `path` (relative to the workspace, or
/// absolute), its values in `convention`, as the library and the chip view show it.
#[tauri::command(async)]
pub fn measured_component(
    state: tauri::State<'_, Studio>,
    path: String,
    convention: TimeConvention,
) -> Result<KindInfo, String> {
    measured_in(&state.workspace(), Path::new(&path), convention)
}

fn measured_in(root: &Path, path: &Path, convention: TimeConvention) -> Result<KindInfo, String> {
    let path = &resolve(root, &path.to_string_lossy());
    let file = relative(root, path);
    let c = build(MEASURED, Some(&file), Some(convention), root)?;
    let mut info = describe(c.as_ref(), library::Glyph::Measured);
    info.id = MEASURED.into();
    info.title = path
        .file_name()
        .map_or_else(|| file.clone(), |n| n.to_string_lossy().into_owned());
    info.category = "Measured".into();
    info.about = "S-parameters measured, or simulated by another program, read from a Touchstone file and interpolated in wavelength.".into();
    info.file = Some(file);
    info.convention = Some(convention);
    Ok(info)
}

/// Every kind of component in the library.
#[tauri::command(async)]
pub fn component_library() -> Result<Vec<KindInfo>, String> {
    library::entries().iter().map(info).collect()
}

/// S-parameters over wavelength: `re[q][p][k]` and `im[q][p][k]` are S_qp at the k-th
/// wavelength.
#[derive(Debug, Serialize)]
pub struct SpectrumData {
    ports: Vec<String>,
    wavelength_um: Vec<f64>,
    re: Vec<Vec<Vec<f64>>>,
    im: Vec<Vec<Vec<f64>>>,
}

impl SpectrumData {
    fn of(spectrum: &Spectrum) -> SpectrumData {
        let n = spectrum.ports().len();
        let parts = |f: fn(photonoxide::Complex64) -> f64| -> Vec<Vec<Vec<f64>>> {
            (0..n)
                .map(|q| {
                    (0..n)
                        .map(|p| spectrum.element(q, p).into_iter().map(f).collect())
                        .collect()
                })
                .collect()
        };
        SpectrumData {
            ports: spectrum.ports().to_vec(),
            wavelength_um: spectrum.wavelengths().iter().map(|w| w.to_um()).collect(),
            re: parts(|z| z.re),
            im: parts(|z| z.im),
        }
    }
}

/// A component's spectrum over `sweep`, its parameters at `values` (one per parameter, in
/// order).
#[tauri::command(async)]
pub fn component_spectrum(
    state: tauri::State<'_, Studio>,
    part: Part,
    values: Vec<f64>,
    sweep: Sweep,
) -> Result<SpectrumData, String> {
    let spectrum = component(&part, &values, sweep, &state.workspace())?;
    Ok(SpectrumData::of(&spectrum))
}

/// What a component is built from: a library id, or for [`MEASURED`] a file and its time
/// convention.
#[derive(Clone, Debug, Deserialize)]
pub struct Part {
    kind: String,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    convention: Option<TimeConvention>,
}

/// A component's spectrum over `sweep`, its parameters at `values`.
fn component(part: &Part, values: &[f64], sweep: Sweep, root: &Path) -> Result<Spectrum, String> {
    let kind = &part.kind;
    let c = build(kind, part.file.as_deref(), part.convention, root)?;
    let parameters = c.parameters();
    if values.len() != parameters.len() {
        return Err(format!(
            "{kind} takes {} values, not {}",
            parameters.len(),
            values.len()
        ));
    }
    if let Some((p, v)) = parameters.iter().zip(values).find(|(p, v)| !p.allows(**v)) {
        return Err(format!(
            "{} must be from {} to {}, not {v}",
            p.name, p.min, p.max
        ));
    }
    let wavelengths = sweep.wavelengths()?;
    let matrices = sweep_matrices(c.as_ref(), &wavelengths, values)?;
    let ports = c.ports().iter().map(|p| p.name.clone()).collect();
    Spectrum::new(ports, wavelengths, matrices).map_err(|e| e.to_string())
}

/// `c`'s S-matrices at every wavelength, its parameters at `values`, the wavelengths shared out
/// among the machine's cores: each is independent of the others, and a model that solves modes
/// at each (an MMI's) takes milliseconds apiece.
fn sweep_matrices(
    c: &dyn Component,
    wavelengths: &[Wavelength],
    values: &[f64],
) -> Result<Vec<SMatrix>, String> {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let chunk = wavelengths.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let parts: Vec<_> = wavelengths
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|&w| c.s_matrix(w, values))
                        .collect::<photonoxide::Result<Vec<_>>>()
                })
            })
            .collect();
        let mut matrices = Vec::with_capacity(wavelengths.len());
        for part in parts {
            let part = part
                .join()
                .map_err(|_| "a wavelength's solve panicked".to_owned())?;
            matrices.extend(part.map_err(|e| e.to_string())?);
        }
        Ok(matrices)
    })
}

// ---- circuits ----

/// Everything that keeps `chip` from being simulated: its steps' errors, then its dangling
/// ports.
#[tauri::command(async)]
pub fn circuit_check(state: tauri::State<'_, Studio>, chip: Chip) -> Vec<Problem> {
    chip.netlist(&state.workspace()).1
}

/// How a circuit's S-matrices measure up, at their worst over the sweep.
#[derive(Debug, Serialize)]
pub struct Checks {
    /// The largest |S_qp − S_pq|: zero for a reciprocal circuit.
    reciprocity: f64,
    /// The largest singular value of S: at most 1 for a passive circuit.
    largest_singular_value: f64,
    /// The largest entry of |SᴴS − I|: zero for a lossless circuit.
    unitarity: f64,
    /// Whether every instance is reciprocal, so the reciprocity error should be round-off.
    reciprocal: bool,
}

/// A circuit simulated: its spectrum between its external ports, its checks, and how long the
/// solve took.
#[derive(Debug, Serialize)]
pub struct Simulation {
    spectrum: SpectrumData,
    checks: Checks,
    seconds: f64,
    instances: usize,
}

/// The spectrum of `chip` over its sweep, its netlist, and how long the solve took.
fn solve(chip: &Chip, root: &Path) -> Result<(Netlist, Spectrum, f64), String> {
    let wavelengths = chip.sweep.wavelengths()?;
    let (netlist, problems) = chip.netlist(root);
    if let Some(p) = problems.first() {
        return Err(if problems.len() > 1 {
            format!("{} (and {} more problems)", p.message, problems.len() - 1)
        } else {
            p.message.clone()
        });
    }
    if netlist.external().is_empty() {
        return Err("the circuit has no external ports: place one and wire it".into());
    }
    let started = Instant::now();
    let circuit = netlist.compile().map_err(|e| e.to_string())?;
    // Circuit::spectrum, one sparse solve per wavelength, the wavelengths on every core
    let matrices = sweep_matrices(&circuit, &wavelengths, circuit.values())?;
    let ports = circuit.ports().iter().map(|p| p.name.clone()).collect();
    let spectrum = Spectrum::new(ports, wavelengths, matrices).map_err(|e| e.to_string())?;
    Ok((netlist, spectrum, started.elapsed().as_secs_f64()))
}

/// Simulates `chip` over its sweep: the library's sparse solve of the whole netlist at each
/// wavelength.
#[tauri::command(async)]
pub fn circuit_simulate(state: tauri::State<'_, Studio>, chip: Chip) -> Result<Simulation, String> {
    simulate(&chip, &state.workspace())
}

fn simulate(chip: &Chip, root: &Path) -> Result<Simulation, String> {
    let (netlist, spectrum, seconds) = solve(chip, root)?;
    let mut checks = Checks {
        reciprocity: 0.0,
        largest_singular_value: 0.0,
        unitarity: 0.0,
        reciprocal: netlist.instances().iter().all(|i| i.component.reciprocal()),
    };
    for s in spectrum.matrices() {
        checks.reciprocity = checks.reciprocity.max(s.reciprocity_error());
        checks.largest_singular_value = checks
            .largest_singular_value
            .max(s.largest_singular_value().map_err(|e| e.to_string())?);
        checks.unitarity = checks.unitarity.max(s.unitarity_error());
    }
    Ok(Simulation {
        spectrum: SpectrumData::of(&spectrum),
        checks,
        seconds,
        instances: netlist.instances().len(),
    })
}

/// Writes `spectrum` to `path` as a Touchstone file (Version 2.0, its ports in order, in
/// photonoxide's e^(−iωt) convention).
fn write_touchstone(path: &Path, spectrum: &Spectrum) -> Result<(), String> {
    Touchstone::from_spectrum(spectrum, Convention::Physics)
        .and_then(|file| file.write_file(path, Precision::RoundTrip))
        .map_err(|e| e.to_string())
}

/// Simulates `chip` and writes its spectrum to `path` as a Touchstone file.
#[tauri::command(async)]
pub fn circuit_touchstone(
    state: tauri::State<'_, Studio>,
    chip: Chip,
    path: String,
) -> Result<(), String> {
    let (_, spectrum, _) = solve(&chip, &state.workspace())?;
    write_touchstone(Path::new(&path), &spectrum)
}

/// Writes component `kind`'s spectrum over `sweep`, its parameters at `values`, to `path` as a
/// Touchstone file.
#[tauri::command(async)]
pub fn component_touchstone(
    state: tauri::State<'_, Studio>,
    part: Part,
    values: Vec<f64>,
    sweep: Sweep,
    path: String,
) -> Result<(), String> {
    let spectrum = component(&part, &values, sweep, &state.workspace())?;
    write_touchstone(Path::new(&path), &spectrum)
}

/// `chip` as a chip file's text.
#[tauri::command]
pub fn circuit_text(chip: Chip) -> Result<String, String> {
    chip.to_text()
}

/// The chip in a chip file's `text`.
#[tauri::command]
pub fn circuit_parse(text: String) -> Result<Chip, String> {
    Chip::parse(&text)
}

/// A circuit built into the program.
#[derive(Serialize)]
pub struct CircuitExample {
    file: &'static str,
    chip: Chip,
}

/// The circuits built into the program: circuits/ in the repository.
pub fn example_texts() -> [(&'static str, &'static str); 4] {
    [
        ("mzi.toml", include_str!("../../../circuits/mzi.toml")),
        (
            "ring-all-pass.toml",
            include_str!("../../../circuits/ring-all-pass.toml"),
        ),
        (
            "ring-add-drop.toml",
            include_str!("../../../circuits/ring-add-drop.toml"),
        ),
        (
            "splitter-1x4.toml",
            include_str!("../../../circuits/splitter-1x4.toml"),
        ),
    ]
}

/// The circuits built into the program.
#[tauri::command]
pub fn circuit_examples() -> Result<Vec<CircuitExample>, String> {
    example_texts()
        .into_iter()
        .map(|(file, text)| {
            Ok(CircuitExample {
                file,
                chip: Chip::parse(text).map_err(|e| format!("{file}: {e}"))?,
            })
        })
        .collect()
}

/// A chip file in the workspace.
#[derive(Serialize)]
pub struct CircuitItem {
    path: String,
    name: String,
    about: String,
    instances: usize,
    modified: f64,
}

fn circuits_dir(state: &Studio) -> PathBuf {
    state.workspace().join("circuits")
}

/// The chip files in the workspace's `circuits/`.
#[tauri::command]
pub fn circuits(state: tauri::State<'_, Studio>) -> Vec<CircuitItem> {
    circuits_in(&circuits_dir(&state))
}

fn circuits_in(dir: &Path) -> Vec<CircuitItem> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let chip = Chip::parse(&std::fs::read_to_string(&path).ok()?).ok()?;
            let modified = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0.0, |d| d.as_secs_f64());
            Some(CircuitItem {
                path: path.display().to_string(),
                name: chip.name,
                about: chip.about,
                instances: chip.instances.len(),
                modified,
            })
        })
        .collect()
}

/// Checks a chip's name as a file name: letters, digits, `-` and `_`.
fn file_name(name: &str) -> Result<String, String> {
    if name.is_empty()
        || name.len() > 80
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "\"{name}\" can't name a file: use letters, digits, - and _"
        ));
    }
    Ok(format!("{name}.toml"))
}

/// Saves `chip` as `circuits/<its name>.toml` in the workspace; refuses to replace another file
/// unless `replace`. Returns the path.
#[tauri::command]
pub fn save_circuit(
    state: tauri::State<'_, Studio>,
    chip: Chip,
    replace: bool,
) -> Result<String, String> {
    save_in(&circuits_dir(&state), &chip, replace).map(|p| p.display().to_string())
}

fn save_in(dir: &Path, chip: &Chip, replace: bool) -> Result<PathBuf, String> {
    let file = file_name(&chip.name)?;
    let text = chip.to_text()?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(file);
    if path.exists() && !replace {
        return Err(format!("exists: {}", path.display()));
    }
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// The chip in the file at `path`.
#[tauri::command]
pub fn read_circuit(path: String) -> Result<Chip, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    Chip::parse(&text).map_err(|e| format!("{path}: {e}"))
}

/// Deletes a chip file of the workspace.
#[tauri::command]
pub fn delete_circuit(state: tauri::State<'_, Studio>, path: String) -> Result<(), String> {
    let path = crate::studio::inside(&circuits_dir(&state), Path::new(&path))?;
    if path.extension().is_none_or(|x| x != "toml") {
        return Err(format!("{} isn't a chip file", path.display()));
    }
    std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))
}
