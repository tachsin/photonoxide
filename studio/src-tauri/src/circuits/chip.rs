//! A chip's devices and its jobs: an instance that carries a device's design (inline, or a job
//! file of the workspace's `jobs/`), the library's chip of a chip file
//! (`photonoxide::job::ChipDesign`), and the jobs its Study and Optimize panels make of it
//! (`spectrum_from_chip`, `study_from_chip`, `optimize_from_chip`).

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use photonoxide::circuit::Component;
use photonoxide::device::Spec;
use photonoxide::job::{self, ChipDesign, ChipInstance, Corner, Study, SweepAxis};
use photonoxide::run::Job;
use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use super::{Chip, MEASURED, Placed, library};
use crate::devices::optimizing::OptimizeIn;
use crate::devices::{SpecIn, StudyIn};
use crate::studio::Studio;

/// An instance's device design, as a chip file holds it: a job's `[device]` table inline (its
/// kind, fidelity, stack and parameters), or `job = "jobs/<name>.yaml"`, a design of the
/// workspace, whose fidelity and parameters those given here change.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceRef {
    /// The job file holding the design, relative to the workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fidelity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<Value>,
    /// Expressions by name: the design's own, or those changing a job file's.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, String>,
}

impl DeviceRef {
    /// The design, its job file read from the workspace `root`.
    ///
    /// # Errors
    ///
    /// A message for a job file that can't be read or holds no design, a kind that isn't the
    /// job's, or what the library refuses of the table.
    pub fn spec(&self, root: &Path) -> Result<Spec, String> {
        let mut table = match &self.job {
            Some(file) => {
                let job = Job::load(&super::resolve(root, file)).map_err(|e| e.to_string())?;
                let mut t = job
                    .device()
                    .cloned()
                    .ok_or_else(|| format!("{file} holds no device's design"))?;
                if let Some(k) = &self.kind
                    && t.get("kind").and_then(Value::as_str) != Some(k)
                {
                    return Err(format!("{file} is a design of another kind than {k}"));
                }
                // its expressions as strings, as the instance's are
                if let Some(Value::Table(p)) = t.get_mut("parameters") {
                    for (_, v) in p.iter_mut() {
                        if !v.is_str() {
                            *v = Value::String(match &*v {
                                Value::Float(x) => format!("{x:?}"),
                                other => other.to_string(),
                            });
                        }
                    }
                }
                t
            }
            None => {
                let mut t = Table::new();
                t.insert(
                    "kind".into(),
                    Value::String(
                        self.kind
                            .clone()
                            .ok_or("a device's design needs its kind, or a job file")?,
                    ),
                );
                t
            }
        };
        if let Some(f) = &self.fidelity {
            table.insert("fidelity".into(), Value::String(f.clone()));
        }
        if let Some(s) = &self.stack {
            table.insert("stack".into(), s.clone());
        }
        if !self.parameters.is_empty() {
            let mut p = match table.remove("parameters") {
                Some(Value::Table(p)) => p,
                _ => Table::new(),
            };
            for (n, e) in &self.parameters {
                p.insert(n.clone(), Value::String(e.clone()));
            }
            table.insert("parameters".into(), Value::Table(p));
        }
        Spec::from_table(&table).map_err(|e| e.to_string())
    }

    /// `spec` inline.
    pub fn inline(spec: &Spec) -> DeviceRef {
        let t = spec.to_table();
        DeviceRef {
            job: None,
            kind: Some(spec.kind.clone()),
            fidelity: t.get("fidelity").and_then(Value::as_str).map(str::to_owned),
            stack: t.get("stack").cloned(),
            parameters: spec.parameters.iter().cloned().collect(),
        }
    }
}

/// The library's id of a device kind: the Components page's, and the chip's glyph's.
pub fn id_of(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "waveguide" => "waveguide",
        "bend" => "bend",
        "directional coupler" => "directional-coupler",
        "all-pass ring" => "ring-all-pass",
        "add-drop ring" => "ring-add-drop",
        "rib waveguide" => "rib-waveguide",
        "MMI 1x2" => "mmi-1x2",
        "MMI 2x2" => "mmi-2x2",
        "Y-branch" => "y-branch",
        "MZI" => "mzi-couplers",
        "Y-branch MZI" => "mzi-y-branches",
        "thermo-optic phase shifter" => "thermo-optic-shifter",
        "doped thermo-optic phase shifter" => "doped-thermo-optic-shifter",
        "in-guide thermo-optic phase shifter" => "in-guide-thermo-optic-shifter",
        "GST phase-change phase shifter" => "gst-phase-shifter",
        "Sb2S3 phase-change phase shifter" => "sb2s3-phase-shifter",
        "Sb2Se3 phase-change phase shifter" => "sb2se3-phase-shifter",
        _ => return None,
    })
}

/// The device kind a library id stands for, if a device's.
pub fn kind_of(id: &str) -> Option<&'static str> {
    [
        "waveguide",
        "bend",
        "directional coupler",
        "all-pass ring",
        "add-drop ring",
        "rib waveguide",
        "MMI 1x2",
        "MMI 2x2",
        "Y-branch",
        "MZI",
        "Y-branch MZI",
        "thermo-optic phase shifter",
        "doped thermo-optic phase shifter",
        "in-guide thermo-optic phase shifter",
        "GST phase-change phase shifter",
        "Sb2S3 phase-change phase shifter",
        "Sb2Se3 phase-change phase shifter",
    ]
    .into_iter()
    .find(|k| id_of(k) == Some(id))
}

/// A device instance's component: its design built at its fidelity, and the values its
/// parameters give the component.
///
/// # Errors
///
/// What the library refuses of the design.
pub fn component(
    device: &DeviceRef,
    root: &Path,
) -> Result<(Arc<dyn Component>, Vec<f64>), String> {
    let spec = device.spec(root)?;
    let (d, design) = spec.build().map_err(|e| e.to_string())?;
    let built = d
        .component(design.values(), spec.fidelity)
        .map_err(|e| e.to_string())?;
    Ok((Arc::from(built.component), built.values))
}

/// An instance as the library's chip has it.
fn instance_of(inst: &Placed, root: &Path) -> Result<ChipInstance, String> {
    let at = |e: String| format!("{}: {e}", inst.name);
    let mut out = if let Some(d) = &inst.device {
        ChipInstance::device(&inst.name, d.spec(root).map_err(at)?)
    } else if job::COMPONENTS.contains(&inst.kind.as_str()) {
        ChipInstance::component(&inst.name, &inst.kind)
    } else if let Some(kind) = kind_of(&inst.kind) {
        // the library's kind is its device at its defaults; the instance's values, every one,
        // are its component's
        let mut c = ChipInstance::device(&inst.name, Spec::new(kind));
        let entry = library::entry(&inst.kind).ok_or_else(|| at("not in the library".into()))?;
        let model = (entry.build)().map_err(at)?;
        for p in model.parameters() {
            let v = inst.values.get(&p.name).copied().unwrap_or(p.default);
            c = c.with_value(&p.name, v);
        }
        return Ok(c);
    } else if inst.kind == MEASURED {
        return Err(at(
            "a measured component can't be in a chip's job yet: its file isn't part of the job"
                .into(),
        ));
    } else {
        return Err(at(format!(
            "the library's {} isn't a device's design: place a device (its page's Place on chip) \
             to study or optimize the chip",
            inst.kind
        )));
    };
    for (p, v) in &inst.values {
        out = out.with_value(p, *v);
    }
    Ok(out)
}

/// The library's chip of `chip`: its devices' designs resolved, its library kinds as their
/// devices at their defaults with the instance's values.
///
/// # Errors
///
/// A message naming the instance the library can't take.
pub fn design(chip: &Chip, root: &Path) -> Result<ChipDesign, String> {
    let mut d = ChipDesign::new();
    for inst in &chip.instances {
        d = d.instance(instance_of(inst, root)?);
    }
    for [a, b] in &chip.connections {
        d = d.connect(a, b);
    }
    for p in &chip.ports {
        if p.at.is_empty() {
            return Err(format!("external port {} isn't wired to anything", p.name));
        }
        d = d.port(&p.name, &p.at);
    }
    Ok(d)
}

/// An instance's design resolved, as the Chip page shows it and opens it.
#[derive(Debug, Serialize)]
pub struct InstanceDevice {
    spec: SpecIn,
    /// The device's id on the Components page.
    id: &'static str,
    job: Option<String>,
}

/// The design `device` holds, resolved in the workspace.
#[tauri::command]
pub fn chip_device(
    state: tauri::State<'_, Studio>,
    device: DeviceRef,
) -> Result<InstanceDevice, String> {
    instance_device(&device, &state.workspace())
}

fn instance_device(device: &DeviceRef, root: &Path) -> Result<InstanceDevice, String> {
    let spec = device.spec(root)?;
    Ok(InstanceDevice {
        id: id_of(&spec.kind)
            .ok_or_else(|| format!("no device {} on the Components page", spec.kind))?,
        spec: SpecIn::of(&spec),
        job: device.job.clone(),
    })
}

/// `spec` inline, as an instance carries it.
#[tauri::command]
pub fn chip_device_ref(spec: SpecIn) -> Result<DeviceRef, String> {
    Ok(DeviceRef::inline(&spec.spec()?))
}

/// A parameter a chip's study or optimization may move.
#[derive(Debug, Serialize)]
pub struct ChipParameter {
    /// `instance.parameter`.
    name: String,
    instance: String,
    parameter: String,
    unit: String,
    value: f64,
    min: Option<f64>,
    max: Option<f64>,
    design: bool,
}

/// What a chip's optimization offers.
#[derive(Debug, Serialize)]
pub struct ChipOptimizeInfo {
    outputs: Vec<crate::devices::optimizing::OutputInfo>,
    methods: Vec<crate::devices::optimizing::MethodInfo>,
    instances: Vec<InstanceOffer>,
    parameters: Vec<ChipParameter>,
}

/// A device instance's own objectives and constraints.
#[derive(Debug, Serialize)]
pub struct InstanceOffer {
    name: String,
    kind: String,
    objectives: Vec<crate::devices::optimizing::ObjectiveInfo>,
    constraints: Vec<crate::devices::optimizing::ConstraintInfo>,
}

/// The chip's parameters, its outputs, genoxide's methods, and its device instances' own
/// objectives and constraints.
#[tauri::command(async)]
pub fn chip_optimize_info(
    state: tauri::State<'_, Studio>,
    chip: Chip,
) -> Result<ChipOptimizeInfo, String> {
    optimize_info(&chip, &state.workspace())
}

fn optimize_info(chip: &Chip, root: &Path) -> Result<ChipOptimizeInfo, String> {
    let ports: Vec<&str> = chip.ports.iter().map(|p| p.name.as_str()).collect();
    let mut outputs = Vec::new();
    for a in &ports {
        for b in &ports {
            outputs.push(crate::devices::optimizing::OutputInfo::new(
                &format!("T_{a}_{b}"),
                "",
                &format!("power from {b} into {a}"),
                true,
            ));
        }
    }
    outputs.push(crate::devices::optimizing::OutputInfo::new(
        "lambda",
        "µm",
        "the wavelength",
        true,
    ));
    let mut instances = Vec::new();
    let mut parameters = Vec::new();
    for inst in &chip.instances {
        if let Some(d) = &inst.device {
            let spec = d.spec(root).map_err(|e| format!("{}: {e}", inst.name))?;
            let info = crate::devices::optimizing::optimize_info(&SpecIn::of(&spec))?;
            let (device, design) = spec.build().map_err(|e| e.to_string())?;
            for p in device.parameters() {
                parameters.push(ChipParameter {
                    name: format!("{}.{}", inst.name, p.name),
                    instance: inst.name.clone(),
                    parameter: p.name.clone(),
                    unit: p.unit.clone(),
                    value: design.values().get(&p.name).unwrap_or(p.default),
                    min: Some(p.min),
                    max: Some(p.max),
                    design: true,
                });
            }
            let (objectives, constraints) = info.offers();
            instances.push(InstanceOffer {
                name: inst.name.clone(),
                kind: spec.kind.clone(),
                objectives,
                constraints,
            });
        } else if let Some(entry) = library::entry(&inst.kind) {
            let model = (entry.build)()?;
            for p in model.parameters() {
                parameters.push(ChipParameter {
                    name: format!("{}.{}", inst.name, p.name),
                    instance: inst.name.clone(),
                    parameter: p.name.clone(),
                    unit: p.unit.clone(),
                    value: inst.values.get(&p.name).copied().unwrap_or(p.default),
                    min: Some(p.min),
                    max: Some(p.max),
                    design: false,
                });
            }
        }
    }
    Ok(ChipOptimizeInfo {
        outputs,
        methods: crate::devices::optimizing::methods(),
        instances,
        parameters,
    })
}

/// A chip's job, as its Study and Optimize panels make it.
#[derive(Clone, Debug, Deserialize)]
pub struct ChipTaskIn {
    name: String,
    /// From, to (µm) and how many wavelengths.
    band: (f64, f64, usize),
    timeout_minutes: f64,
    #[serde(default)]
    study: Option<StudyIn>,
    #[serde(default)]
    optimize: Option<OptimizeIn>,
}

/// The job that computes, studies or optimizes `chip` as `task` says.
#[tauri::command(async)]
pub fn chip_job(
    state: tauri::State<'_, Studio>,
    chip: Chip,
    task: ChipTaskIn,
) -> Result<String, String> {
    job_text(&chip, &task, &state.workspace())
}

/// The text of `task`'s job of `chip`, checked.
pub fn job_text(chip: &Chip, task: &ChipTaskIn, root: &Path) -> Result<String, String> {
    let d = design(chip, root)?;
    let text = |e: photonoxide::Error| e.to_string();
    let job = match (&task.study, &task.optimize) {
        (None, None) => job::spectrum_from_chip(&task.name, &d, task.band, task.timeout_minutes),
        (Some(s), None) => {
            let study = s.study()?;
            let (axes, corners): (Vec<SweepAxis>, Vec<Corner>) = match study {
                Study::Sweep(a) => (a, Vec::new()),
                Study::Corners(c) => (Vec::new(), c),
                _ => return Err("a chip's study is a sweep or corners".into()),
            };
            job::study_from_chip(
                &task.name,
                &d,
                task.band,
                &axes,
                &corners,
                task.timeout_minutes,
            )
        }
        (None, Some(o)) => job::optimize_from_chip(
            &task.name,
            &d,
            task.band,
            &o.optimization()?,
            task.timeout_minutes,
        ),
        (Some(_), Some(_)) => return Err("a chip's job studies or optimizes it, not both".into()),
    }
    .map_err(text)?;
    Ok(job.text().to_owned())
}

/// The Chip page's chip of a chip job's text (a `[circuit]` table): its instances laid out in a
/// row as [`Chip::from_netlist`] lays a netlist out, each device instance carrying its design
/// inline, the sweep the task's band where it gives one.
///
/// # Errors
///
/// A message for a job that isn't a chip's, or a chip the library can't build.
pub fn chip_of_job(text: &str) -> Result<Chip, String> {
    let job = crate::studio::parse_job(text).map_err(|e| e.to_string())?;
    let table = job.circuit().ok_or_else(|| {
        format!(
            "{} isn't a chip's job: it has no [circuit] table",
            job.name()
        )
    })?;
    let d = ChipDesign::from_table(table).map_err(|e| e.to_string())?;
    let circuit = d.circuit().map_err(|e| e.to_string())?;
    let name: String = job
        .name()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let mut chip = Chip::from_netlist(&name, circuit.netlist())?;
    // in rows of four rather than one long row, each external port moved with its instance
    let moves: Vec<(String, f64, f64)> = chip
        .instances
        .iter_mut()
        .enumerate()
        .map(|(k, inst)| {
            let (x, y) = (160.0 * (k % 4) as f64, 160.0 * (k / 4) as f64);
            let d = (inst.name.clone(), x - inst.x, y - inst.y);
            (inst.x, inst.y) = (x, y);
            d
        })
        .collect();
    for p in &mut chip.ports {
        let owner = p.at.split_once('.').map_or("", |(i, _)| i);
        if let Some((_, dx, dy)) = moves.iter().find(|(n, _, _)| n == owner) {
            p.x += dx;
            p.y += dy;
        }
    }
    chip.about = text
        .lines()
        .map_while(|l| l.strip_prefix('#'))
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ");
    for (placed, inst) in chip.instances.iter_mut().zip(&d.instances) {
        placed.values = inst.values.iter().cloned().collect();
        if let job::InstanceModel::Device(spec) = &inst.model {
            placed.kind = id_of(&spec.kind)
                .ok_or_else(|| {
                    format!(
                        "{}: no device {} on the Components page",
                        inst.name, spec.kind
                    )
                })?
                .to_owned();
            placed.device = Some(DeviceRef::inline(spec));
        }
    }
    let task = job.task();
    let number = |k: &str| {
        task.get(k)
            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
    };
    if let (Some(from), Some(to)) = (number("from_um"), number("to_um"))
        && to > from
    {
        chip.sweep = super::Sweep {
            from_um: from,
            to_um: to,
            points: number("points").map_or(501, |p| (p as usize).max(2)),
        };
    }
    Ok(chip)
}

/// The chip a chip job's text holds, for the Chip page.
#[tauri::command]
pub fn chip_of_job_text(text: String) -> Result<Chip, String> {
    chip_of_job(&text)
}

/// `chip` with an optimization's design taken in: each `instance.parameter` at its expression,
/// a device's design parameter as written, a component's value as its number.
#[tauri::command]
pub fn chip_apply(chip: Chip, parameters: Vec<(String, String)>) -> Result<Chip, String> {
    apply(chip, &parameters)
}

pub fn apply(mut chip: Chip, parameters: &[(String, String)]) -> Result<Chip, String> {
    for (name, expression) in parameters {
        let (instance, parameter) = name
            .split_once('.')
            .ok_or_else(|| format!("{name} isn't instance.parameter"))?;
        let inst = chip
            .instances
            .iter_mut()
            .find(|i| i.name == instance)
            .ok_or_else(|| format!("no instance {instance}"))?;
        match (&mut inst.device, inst.values.contains_key(parameter)) {
            (Some(d), false) => {
                d.parameters
                    .insert(parameter.to_owned(), expression.clone());
            }
            _ => {
                let v: f64 = expression
                    .parse()
                    .map_err(|_| format!("{name}: {expression} isn't a number"))?;
                inst.values.insert(parameter.to_owned(), v);
            }
        }
    }
    Ok(chip)
}
