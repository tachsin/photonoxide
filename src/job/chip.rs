//! A chip of devices: a job's `[circuit]` table, its instances each a device's design at its
//! fidelity or one of the circuit's own components, computed, studied and optimized together.
//!
//! ```toml
//! name = "mzi-of-devices"
//! timeout_minutes = 5
//!
//! [circuit]
//! connections = [["split.o3", "upper.o1"], ["upper.o2", "combine.o2"],
//!                ["split.o4", "lower.o1"], ["lower.o2", "combine.o1"]]
//!
//! [[circuit.instance]]
//! name = "split"
//! device = { kind = "directional coupler", fidelity = "analytic" }   # a [device] table
//!
//! [[circuit.instance]]
//! name = "upper"
//! device = { kind = "waveguide", parameters = { length = "150 um" } }
//!
//! [[circuit.instance]]
//! name = "lower"
//! device = { kind = "waveguide", parameters = { length = "100 um" } }
//!
//! [[circuit.instance]]
//! name = "combine"
//! component = "coupler"          # the circuit's own: coupler, phase-shifter or terminator
//! values = { coupling = 0.5 }    # its parameters' values; a device's own component's too
//!
//! [[circuit.port]]
//! name = "in"
//! at = "split.o1"
//!
//! [[circuit.port]]
//! name = "out"
//! at = "combine.o4"
//!
//! [task]
//! kind = "spectrum"              # or "sweep", "corners", "optimize"
//! from_um = 1.5
//! to_um = 1.6
//! points = 501
//! ```
//!
//! **Instances.** A `device` is a [`Spec`]: its kind, stack, parameters' expressions and fidelity,
//! as a `[device]` table holds them. It is built at its fidelity ([`crate::device::Device::component`])
//! and placed with the values its parameters give its component; an instance's `values` then set
//! its component's parameters by name (a measured or library component's, or a device's own
//! component's, which no longer follows the design). A `component` is one of [`COMPONENTS`]:
//! the ideal coupler, the phase shifter and the terminator.
//!
//! **Parameters by name.** A study's axis, a corner's offset and an optimization's variable name
//! a parameter `instance.parameter`: one of a device's parameters or of the names its design's
//! expressions define (`ring.gap`, `ring.w`), or one of a component's (`split.coupling`). Each is
//! in its unit: µm for a length, degrees for an angle.
//!
//! **Tasks.** `"spectrum"` records [`Event::Chip`] (each instance's model, fidelity and grid)
//! and an [`Event::SParameters`] per wavelength between the external ports; `"sweep"` and
//! `"corners"` (offsets of instances' parameters) record [`Event::Chip`] for the nominal chip,
//! [`Event::ChipStudy`], and an [`Event::ChipPoint`] per point, each the bits of that point's own
//! `"spectrum"` job, side by side on rayon's threads; `"optimize"` is [`super::Problem`]'s, its
//! gradients the circuit adjoint ([`crate::circuit::Circuit::jacobian`]) through each device's
//! derivatives. The band is the task's `from_um`, `to_um` and `points`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use super::device::fidelity_name;
use super::study::{Named, set};
use super::{Corner, Event, SweepAxis, reason, task_error};
use crate::circuit::components::{Coupler, PhaseShifter};
use crate::circuit::{Circuit, Component, Fixed, Netlist, SMatrix};
use crate::device::{Built, Design, Device, Spec};
use crate::expr::Dim;
use crate::run::{Job, Run, Stop};
use crate::units::Wavelength;
use crate::{Error, Result};

/// The circuit's own components a chip's instance may be (`component = "coupler"`), by the ids
/// the studio's library gives them: the ideal 2 × 2 coupler, the ideal phase shifter, and a
/// perfect absorber of one port.
pub const COMPONENTS: [&str; 3] = ["coupler", "phase-shifter", "terminator"];

/// The most instances a chip takes.
const MAX_INSTANCES: usize = 4096;

/// A chip as a job's `[circuit]` table holds it: its instances, the connections between their
/// ports, and the external ports.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct ChipDesign {
    /// The instances, in the order the netlist adds them.
    pub instances: Vec<ChipInstance>,
    /// Connections, each two ports written `instance.port`.
    pub connections: Vec<[String; 2]>,
    /// The external ports, in the order of the chip's S-matrix.
    pub ports: Vec<ChipPort>,
}

/// An instance on a chip: a device's design or one of the circuit's components, and values of
/// its component's parameters.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ChipInstance {
    /// Its name, unique on the chip.
    pub name: String,
    /// What it is.
    pub model: InstanceModel,
    /// Its component's parameters' values by name, set after the device (if it is one) gives
    /// its own.
    pub values: Vec<(String, f64)>,
}

/// What an instance is.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)] // a few per chip, built once
pub enum InstanceModel {
    /// A device's design at its fidelity, as a `[device]` table holds it.
    Device(Spec),
    /// One of [`COMPONENTS`], by its id.
    Component(String),
}

/// An external port: a name, and the instance's port it exposes.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ChipPort {
    /// Its name, e.g. `"in"`: an expression's `T_out_in` names it.
    pub name: String,
    /// The port it exposes, `instance.port`.
    pub at: String,
}

/// One of a chip's instances as a run records it ([`Event::Chip`]): what it is, and the model
/// that computes it with its fidelity and grid.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChipPart {
    /// The instance's name.
    pub name: String,
    /// The device's kind, e.g. `"add-drop ring"`, or the component's id, e.g. `"coupler"`.
    pub kind: String,
    /// Whether it is a device's design (else one of the circuit's components).
    pub device: bool,
    /// Its fidelity: `"analytic"`, `"compact"`, `"2D"`, `"3D"` or `"measured"`.
    pub fidelity: String,
    /// What computes it, in a sentence: the device's method, or the component's provenance.
    pub method: String,
    /// The grid its numbers come from, or `"exact: no grid"`.
    pub grid: String,
    /// Its component's parameters' names and values in the circuit, in order.
    pub parameters: Vec<String>,
    /// Their values.
    pub values: Vec<f64>,
}

impl ChipInstance {
    /// An instance `name` of the device design `spec`.
    pub fn device(name: &str, spec: Spec) -> ChipInstance {
        ChipInstance {
            name: name.to_owned(),
            model: InstanceModel::Device(spec),
            values: Vec::new(),
        }
    }

    /// An instance `name` of the circuit's component `id` (one of [`COMPONENTS`]).
    pub fn component(name: &str, id: &str) -> ChipInstance {
        ChipInstance {
            name: name.to_owned(),
            model: InstanceModel::Component(id.to_owned()),
            values: Vec::new(),
        }
    }

    /// The same instance with its component's parameter `parameter` at `value`.
    #[must_use]
    pub fn with_value(mut self, parameter: &str, value: f64) -> ChipInstance {
        match self.values.iter_mut().find(|(n, _)| n == parameter) {
            Some(v) => v.1 = value,
            None => self.values.push((parameter.to_owned(), value)),
        }
        self
    }
}

impl ChipDesign {
    /// A chip with nothing on it.
    pub fn new() -> ChipDesign {
        ChipDesign::default()
    }

    /// The same chip with `instance` added.
    #[must_use]
    pub fn instance(mut self, instance: ChipInstance) -> ChipDesign {
        self.instances.push(instance);
        self
    }

    /// The same chip with ports `a` and `b` (`instance.port`) connected.
    #[must_use]
    pub fn connect(mut self, a: &str, b: &str) -> ChipDesign {
        self.connections.push([a.to_owned(), b.to_owned()]);
        self
    }

    /// The same chip with the external port `name` exposing `at` (`instance.port`).
    #[must_use]
    pub fn port(mut self, name: &str, at: &str) -> ChipDesign {
        self.ports.push(ChipPort {
            name: name.to_owned(),
            at: at.to_owned(),
        });
        self
    }

    /// The chip a job's `[circuit]` table holds.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for a table that isn't one: an unknown key, an instance without a name or
    /// with both or neither of `device` and `component`, a value that isn't a number, a
    /// connection that isn't two ports, a port without its name or `at`; and what
    /// [`Spec::from_table`] refuses of a device.
    pub fn from_table(table: &Table) -> Result<ChipDesign> {
        for key in table.keys() {
            if !["instance", "connections", "port"].contains(&key.as_str()) {
                return Err(task_error(format!(
                    "[circuit] has no key {key}: it takes instance, connections and port"
                )));
            }
        }
        let mut chip = ChipDesign::new();
        let array = |key: &str| -> Result<Vec<Value>> {
            match table.get(key) {
                None => Ok(Vec::new()),
                Some(Value::Array(a)) => Ok(a.clone()),
                Some(_) => Err(task_error(format!("[circuit] {key} must be an array"))),
            }
        };
        for (k, v) in array("instance")?.into_iter().enumerate() {
            let Value::Table(t) = v else {
                return Err(task_error(format!(
                    "[[circuit.instance]] #{} must be a table",
                    k + 1
                )));
            };
            chip.instances.push(instance_of(&t, k)?);
        }
        for (k, v) in array("connections")?.into_iter().enumerate() {
            let pair = v.as_array().and_then(|a| match a.as_slice() {
                [Value::String(a), Value::String(b)] => Some([a.clone(), b.clone()]),
                _ => None,
            });
            chip.connections.push(pair.ok_or_else(|| {
                task_error(format!(
                    "[circuit] connection {}: two ports, [\"a.o1\", \"b.o2\"]",
                    k + 1
                ))
            })?);
        }
        for (k, v) in array("port")?.into_iter().enumerate() {
            let t = v.as_table().cloned().unwrap_or_default();
            for key in t.keys() {
                if key != "name" && key != "at" {
                    return Err(task_error(format!(
                        "[[circuit.port]] #{} has no key {key}: it takes name and at",
                        k + 1
                    )));
                }
            }
            let get = |key: &str| {
                t.get(key)
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| {
                        task_error(format!("[[circuit.port]] #{} needs {key}, a string", k + 1))
                    })
            };
            chip.ports.push(ChipPort {
                name: get("name")?,
                at: get("at")?,
            });
        }
        Ok(chip)
    }

    /// The chip as a `[circuit]` table, which [`ChipDesign::from_table`] reads back to the same
    /// chip.
    pub fn to_table(&self) -> Table {
        let mut t = Table::new();
        if !self.connections.is_empty() {
            t.insert(
                "connections".into(),
                Value::Array(
                    self.connections
                        .iter()
                        .map(|[a, b]| {
                            Value::Array(vec![Value::String(a.clone()), Value::String(b.clone())])
                        })
                        .collect(),
                ),
            );
        }
        let instances = self
            .instances
            .iter()
            .map(|i| {
                let mut u = Table::new();
                u.insert("name".into(), Value::String(i.name.clone()));
                match &i.model {
                    InstanceModel::Device(s) => {
                        u.insert("device".into(), Value::Table(s.to_table()));
                    }
                    InstanceModel::Component(c) => {
                        u.insert("component".into(), Value::String(c.clone()));
                    }
                }
                if !i.values.is_empty() {
                    let v: Table = i
                        .values
                        .iter()
                        .map(|(n, x)| (n.clone(), Value::Float(*x)))
                        .collect();
                    u.insert("values".into(), Value::Table(v));
                }
                Value::Table(u)
            })
            .collect();
        t.insert("instance".into(), Value::Array(instances));
        if !self.ports.is_empty() {
            t.insert(
                "port".into(),
                Value::Array(
                    self.ports
                        .iter()
                        .map(|p| {
                            let mut u = Table::new();
                            u.insert("name".into(), Value::String(p.name.clone()));
                            u.insert("at".into(), Value::String(p.at.clone()));
                            Value::Table(u)
                        })
                        .collect(),
                ),
            );
        }
        t
    }

    /// The chip's circuit at its design: each device built at its fidelity, the netlist
    /// compiled.
    ///
    /// # Errors
    ///
    /// What a device's design and model refuse, and the netlist's errors: an unknown component,
    /// a port connected twice or left dangling, a value outside its parameter's range.
    pub fn circuit(&self) -> Result<Circuit> {
        Ok(self.assemble()?.circuit)
    }

    /// The external ports' names, in the S-matrix's order.
    pub fn port_names(&self) -> Vec<String> {
        self.ports.iter().map(|p| p.name.clone()).collect()
    }

    /// The index of the instance `name`.
    fn index(&self, name: &str) -> Option<usize> {
        self.instances.iter().position(|i| i.name == name)
    }

    /// What `name` (`instance.parameter`) is: an instance's design parameter or component
    /// value, with its unit, nominal value and range.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a name that isn't `instance.parameter`, an unknown instance,
    /// or a parameter it hasn't.
    pub(super) fn resolve(&self, name: &str) -> Result<Resolved> {
        let (inst, parameter) = name.split_once('.').ok_or_else(|| {
            Error::invalid(
                "chip",
                format!("\"{name}\" names a parameter as instance.parameter, e.g. ring.gap"),
            )
        })?;
        let k = self.index(inst).ok_or_else(|| {
            Error::invalid(
                "chip",
                format!(
                    "{name}: the chip has no instance {inst}; it has {}",
                    self.instances
                        .iter()
                        .map(|i| i.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        })?;
        let i = &self.instances[k];
        let component_parameter = |c: &dyn Component| -> Result<Resolved> {
            let p = c
                .parameters()
                .iter()
                .find(|p| p.name == parameter)
                .ok_or_else(|| {
                    Error::invalid(
                        "chip",
                        format!(
                            "{name}: a {} has no parameter {parameter}; it has {}",
                            c.kind(),
                            c.parameters()
                                .iter()
                                .map(|p| p.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    )
                })?;
            let nominal = i
                .values
                .iter()
                .find(|(n, _)| n == parameter)
                .map_or(p.default, |(_, v)| *v);
            Ok(Resolved {
                instance: k,
                parameter: parameter.to_owned(),
                design: false,
                unit: p.unit.clone(),
                angle: false,
                nominal,
                range: Some((p.min, p.max)),
            })
        };
        match &i.model {
            InstanceModel::Component(id) => component_parameter(component(id)?.as_ref()),
            InstanceModel::Device(spec) => {
                // a component value the instance sets stays its own
                if i.values.iter().any(|(n, _)| n == parameter) {
                    let (device, design) = spec.build()?;
                    let built = device.component(design.values(), spec.fidelity)?;
                    return component_parameter(built.component.as_ref());
                }
                let (device, design) = spec.build()?;
                let named = Named {
                    device: device.as_ref(),
                    design: &design,
                    spec,
                };
                named.check(parameter).map_err(|e| {
                    Error::invalid("chip", format!("{name}: {}", strip(&e.to_string())))
                })?;
                let (unit, angle, range) =
                    match device.parameters().iter().find(|p| p.name == parameter) {
                        Some(p) => (p.unit.clone(), false, Some((p.min, p.max))),
                        None => match named.dim(parameter) {
                            Some(d) if d == Dim::LENGTH => ("µm".to_owned(), false, None),
                            Some(d) if d == Dim::ANGLE => ("°".to_owned(), true, None),
                            _ => (String::new(), false, None),
                        },
                    };
                Ok(Resolved {
                    instance: k,
                    parameter: parameter.to_owned(),
                    design: true,
                    unit,
                    angle,
                    nominal: named.nominal(parameter),
                    range,
                })
            }
        }
    }

    /// The same chip with the parameter `r` at `value`, in its unit: a design parameter written
    /// as a plain number in place of its expression (with its unit for a name the design
    /// defines), a component's value set.
    pub(super) fn with(&self, r: &Resolved, value: f64) -> Result<ChipDesign> {
        let mut chip = self.clone();
        let i = &mut chip.instances[r.instance];
        if r.design {
            let InstanceModel::Device(spec) = &mut i.model else {
                unreachable!("a design parameter is a device's")
            };
            let (device, design) = spec.build()?;
            let literal = Named {
                device: device.as_ref(),
                design: &design,
                spec,
            }
            .literal(&r.parameter, value);
            set(spec, &r.parameter, literal);
        } else {
            *i = i.clone().with_value(&r.parameter, value);
        }
        Ok(chip)
    }

    /// The chip built: each instance's component, its parameters' slots in the circuit, and the
    /// circuit compiled.
    pub(super) fn assemble(&self) -> Result<Assembled> {
        if self.instances.is_empty() {
            return Err(Error::invalid("chip", "a chip needs at least one instance"));
        }
        if self.instances.len() > MAX_INSTANCES {
            return Err(Error::invalid(
                "chip",
                format!(
                    "a chip takes at most {MAX_INSTANCES} instances, got {}",
                    self.instances.len()
                ),
            ));
        }
        if self.ports.is_empty() {
            return Err(Error::invalid(
                "chip",
                "a chip needs an external port ([[circuit.port]]) to have a response",
            ));
        }
        let mut netlist = Netlist::new();
        let mut parts = Vec::with_capacity(self.instances.len());
        // the circuit's own components, built once and shared by their instances
        let mut shared: Vec<(String, std::sync::Arc<dyn Component>)> = Vec::new();
        for i in &self.instances {
            let at =
                |e: Error| Error::invalid("chip", format!("{}: {}", i.name, strip(&e.to_string())));
            let (arc, device) = match &i.model {
                InstanceModel::Component(id) => {
                    let c = match shared.iter().find(|(s, _)| s == id) {
                        Some((_, c)) => c.clone(),
                        None => {
                            let c = component(id).map_err(at)?;
                            shared.push((id.clone(), c.clone()));
                            c
                        }
                    };
                    (c, None)
                }
                InstanceModel::Device(spec) => {
                    let (device, design) = spec.build().map_err(at)?;
                    let built = device
                        .component(design.values(), spec.fidelity)
                        .map_err(at)?;
                    let Built {
                        component,
                        values,
                        derivatives,
                        figures,
                    } = built;
                    let arc: std::sync::Arc<dyn Component> = std::sync::Arc::from(component);
                    (
                        arc,
                        Some(DevicePart {
                            device,
                            design,
                            fidelity: spec.fidelity,
                            values,
                            derivatives,
                            figures,
                        }),
                    )
                }
            };
            netlist.add(&i.name, arc.clone())?;
            if let Some(d) = &device {
                for (p, &v) in arc.parameters().iter().zip(&d.values) {
                    netlist.set(&i.name, &p.name, v)?;
                }
            }
            for (p, v) in &i.values {
                netlist.set(&i.name, p, *v)?;
            }
            let overridden = arc
                .parameters()
                .iter()
                .map(|p| i.values.iter().any(|(n, _)| *n == p.name))
                .collect();
            parts.push(Part {
                component: arc,
                device,
                overridden,
                offset: 0,
            });
        }
        for [a, b] in &self.connections {
            netlist.connect(a, b)?;
        }
        for p in &self.ports {
            netlist.expose(&p.name, &p.at)?;
        }
        let circuit = netlist.compile()?;
        for (part, i) in parts.iter_mut().zip(&self.instances) {
            if let Some(first) = part.component.parameters().first() {
                part.offset = circuit
                    .parameter(&format!("{}.{}", i.name, first.name))
                    .unwrap_or_else(|| {
                        unreachable!("every instance's parameters are the circuit's")
                    });
            }
        }
        let values = circuit.values().to_vec();
        Ok(Assembled {
            circuit,
            values,
            parts,
        })
    }

    /// Each instance as a run records it, at the chip's design.
    pub(super) fn parts(&self, assembled: &Assembled) -> Vec<ChipPart> {
        self.instances
            .iter()
            .zip(&assembled.parts)
            .map(|(i, p)| {
                let parameters: Vec<String> = p
                    .component
                    .parameters()
                    .iter()
                    .map(|q| q.name.clone())
                    .collect();
                let values = assembled.values[p.offset..p.offset + parameters.len()].to_vec();
                let provenance = p.component.provenance();
                match (&i.model, &p.device) {
                    (InstanceModel::Device(spec), Some(d)) => {
                        let method = d
                            .device
                            .fidelities()
                            .iter()
                            .find(|m| m.fidelity == spec.fidelity)
                            .map_or_else(String::new, |m| m.name.clone());
                        ChipPart {
                            name: i.name.clone(),
                            kind: spec.kind.clone(),
                            device: true,
                            fidelity: fidelity_name(spec.fidelity).to_owned(),
                            method,
                            grid: d
                                .figures
                                .first()
                                .map_or_else(|| "exact: no grid".to_owned(), |f| f.grid.clone()),
                            parameters,
                            values,
                        }
                    }
                    _ => ChipPart {
                        name: i.name.clone(),
                        kind: match &i.model {
                            InstanceModel::Component(c) => c.clone(),
                            InstanceModel::Device(s) => s.kind.clone(),
                        },
                        device: false,
                        fidelity: fidelity_name(provenance.fidelity).to_owned(),
                        method: provenance.source,
                        grid: "exact: no grid".into(),
                        parameters,
                        values,
                    },
                }
            })
            .collect()
    }
}

/// An error's message without its "invalid ...:" head, to be given another.
fn strip(message: &str) -> String {
    match message.split_once(": ") {
        Some((head, rest)) if head.starts_with("invalid") => rest.to_owned(),
        _ => message.to_owned(),
    }
}

/// An instance from its `[[circuit.instance]]` table, the `k`-th.
fn instance_of(t: &Table, k: usize) -> Result<ChipInstance> {
    let what = format!("[[circuit.instance]] #{}", k + 1);
    for key in t.keys() {
        if !["name", "device", "component", "values"].contains(&key.as_str()) {
            return Err(task_error(format!(
                "{what} has no key {key}: it takes name, device or component, and values"
            )));
        }
    }
    let name = t
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| task_error(format!("{what} needs a name, a string")))?
        .to_owned();
    let what = format!("instance {name}");
    let model = match (t.get("device"), t.get("component")) {
        (Some(Value::Table(d)), None) => InstanceModel::Device(
            Spec::from_table(d)
                .map_err(|e| task_error(format!("{what}: {}", strip(&e.to_string()))))?,
        ),
        (None, Some(Value::String(c))) => InstanceModel::Component(c.clone()),
        _ => {
            return Err(task_error(format!(
                "{what} needs a device (a [device] table) or a component (one of {}), one of them",
                COMPONENTS.join(", ")
            )));
        }
    };
    let values = match t.get("values") {
        None => Vec::new(),
        Some(Value::Table(v)) => v
            .iter()
            .map(|(n, x)| {
                x.as_float()
                    .or_else(|| x.as_integer().map(|i| i as f64))
                    .map(|x| (n.clone(), x))
                    .ok_or_else(|| task_error(format!("{what}: values.{n} must be a number")))
            })
            .collect::<Result<_>>()?,
        Some(_) => {
            return Err(task_error(format!(
                "{what}: values must be a table, name = number"
            )));
        }
    };
    Ok(ChipInstance {
        name,
        model,
        values,
    })
}

/// The circuit's component `id`.
fn component(id: &str) -> Result<std::sync::Arc<dyn Component>> {
    Ok(match id {
        "coupler" => std::sync::Arc::new(Coupler::new()),
        "phase-shifter" => std::sync::Arc::new(PhaseShifter::new()),
        "terminator" => std::sync::Arc::new(Fixed::new("terminator", &["o1"], SMatrix::zeros(1))?),
        other => {
            return Err(Error::invalid(
                "chip",
                format!(
                    "no component \"{other}\": a chip's instance is a device's design (device = \
                     {{ kind = ... }}) or one of {}",
                    COMPONENTS.join(", ")
                ),
            ));
        }
    })
}

/// What a parameter's name on a chip resolves to.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Resolved {
    /// The instance's index.
    pub(super) instance: usize,
    /// The parameter's name on it.
    pub(super) parameter: String,
    /// Whether it is a device's design parameter (else a component's value).
    pub(super) design: bool,
    /// Its unit.
    pub(super) unit: String,
    /// Whether it is an angle the design takes in radians (the name in degrees).
    pub(super) angle: bool,
    /// Its value on the chip.
    pub(super) nominal: f64,
    /// Its parameter's range, where it has one.
    pub(super) range: Option<(f64, f64)>,
}

/// A device instance built: the device, its design, and what its model gave.
pub(super) struct DevicePart {
    pub(super) device: Box<dyn Device>,
    pub(super) design: Design,
    pub(super) fidelity: crate::circuit::Fidelity,
    /// Its component's values, as the device gives them.
    pub(super) values: Vec<f64>,
    /// [`Built::derivatives`].
    pub(super) derivatives: Vec<Option<Vec<f64>>>,
    /// [`Built::figures`].
    pub(super) figures: Vec<crate::device::Figure>,
}

/// An instance in the circuit.
pub(super) struct Part {
    pub(super) component: std::sync::Arc<dyn Component>,
    pub(super) device: Option<DevicePart>,
    /// Per component parameter: whether the instance's `values` set it.
    pub(super) overridden: Vec<bool>,
    /// Its first parameter's slot among the circuit's values.
    pub(super) offset: usize,
}

/// A chip built.
pub(super) struct Assembled {
    pub(super) circuit: Circuit,
    pub(super) values: Vec<f64>,
    pub(super) parts: Vec<Part>,
}

impl Assembled {
    /// The chip's S-matrix at `wavelength` (µm).
    pub(super) fn s_matrix(&self, wavelength: f64) -> Result<SMatrix> {
        self.circuit
            .s_matrix_with(Wavelength::um(wavelength)?, &self.values)
    }
}

/// An S-matrix as a run records it: `s[q][p]`, real and imaginary parts.
pub(super) fn recorded(s: &SMatrix) -> Vec<Vec<[f64; 2]>> {
    let n = s.size();
    (0..n)
        .map(|q| (0..n).map(|p| [s[(q, p)].re, s[(q, p)].im]).collect())
        .collect()
}

// ---- the tasks ----

/// The job's chip: its `[circuit]` table read.
pub(super) fn chip_of(job: &Job) -> Result<ChipDesign> {
    let table = job
        .circuit()
        .ok_or_else(|| task_error("the job has no [circuit] table"))?;
    ChipDesign::from_table(table)
}

/// A chip's `"spectrum"`, `"sweep"` or `"corners"` task.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ChipTask {
    kind: String,
    from_um: f64,
    to_um: f64,
    points: Option<usize>,
    #[serde(default)]
    axis: Vec<SweepAxis>,
    #[serde(default)]
    corner: Vec<super::study::CornerTask>,
}

/// The wavelengths `from_um` to `to_um`, `points` of them (`default` when not given).
pub(super) fn band(
    from_um: f64,
    to_um: f64,
    points: Option<usize>,
    default: usize,
) -> Result<Vec<f64>> {
    let points = points.unwrap_or(if to_um == from_um { 1 } else { default });
    if !(from_um.is_finite() && to_um.is_finite() && from_um > 0.0 && to_um >= from_um) {
        return Err(task_error(format!(
            "the band must go from a positive wavelength to one no smaller, got {from_um} to \
             {to_um} um"
        )));
    }
    if points == 0 || points > 20_001 || (points > 1 && to_um == from_um) {
        return Err(task_error(format!(
            "the band takes 1 to 20001 points, at distinct ends unless it has one, got {points}"
        )));
    }
    let w: Vec<f64> = if points == 1 {
        vec![from_um]
    } else {
        (0..points)
            .map(|k| from_um + (to_um - from_um) * k as f64 / (points - 1) as f64)
            .collect()
    };
    for &x in &w {
        Wavelength::um(x)?;
    }
    Ok(w)
}

/// What a chip's study steps through.
#[derive(Clone, Debug, PartialEq)]
enum ChipStudy {
    Sweep(Vec<SweepAxis>),
    Corners(Vec<Corner>),
}

impl ChipStudy {
    fn kind(&self) -> &'static str {
        match self {
            ChipStudy::Sweep(_) => "sweep",
            ChipStudy::Corners(_) => "corners",
        }
    }
}

/// The chips a study computes, in its points' order.
fn designs(chip: &ChipDesign, study: &ChipStudy) -> Result<Vec<ChipDesign>> {
    match study {
        ChipStudy::Sweep(axes) => {
            super::study::check_axes(axes)?;
            let resolved = axes
                .iter()
                .map(|a| chip.resolve(&a.parameter))
                .collect::<Result<Vec<_>>>()?;
            let first = axes[0].values();
            let second = axes
                .get(1)
                .map_or_else(|| vec![f64::NAN], SweepAxis::values);
            let mut out = Vec::with_capacity(first.len() * second.len());
            for &u in &first {
                for &v in &second {
                    let mut c = chip.with(&resolved[0], u)?;
                    if let Some(r) = resolved.get(1) {
                        c = c.with(r, v)?;
                    }
                    out.push(c);
                }
            }
            Ok(out)
        }
        ChipStudy::Corners(corners) => {
            super::study::check_corners(corners)?;
            let mut out = vec![chip.clone()];
            for c in corners {
                out.push(corner(chip, c)?);
            }
            Ok(out)
        }
    }
}

/// `chip` at the corner `c`: each offset added to its parameter's nominal value.
pub(super) fn corner(chip: &ChipDesign, c: &Corner) -> Result<ChipDesign> {
    if c.temperature_k.is_some() {
        return Err(Error::invalid(
            "chip",
            format!(
                "the corner \"{}\" sets a temperature: a chip's corners move its instances' \
                 parameters (a device's own Study tab takes its materials' temperature)",
                c.name
            ),
        ));
    }
    let mut out = chip.clone();
    for (name, offset) in &c.offsets {
        let r = chip.resolve(name)?;
        out = out.with(&r, r.nominal + offset)?;
    }
    Ok(out)
}

/// A chip's study task read and checked: the study, its chips and the band.
struct Prepared {
    chip: ChipDesign,
    study: Option<ChipStudy>,
    designs: Vec<ChipDesign>,
    wavelengths: Vec<f64>,
}

fn prepare(job: &Job) -> Result<Prepared> {
    let chip = chip_of(job)?;
    let task: ChipTask = job
        .task()
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(e.to_string()))?;
    let study = match task.kind.as_str() {
        "spectrum" => {
            if !task.axis.is_empty() || !task.corner.is_empty() {
                return Err(task_error("a chip's spectrum takes no axes or corners"));
            }
            None
        }
        "sweep" => {
            if !task.corner.is_empty() {
                return Err(task_error(
                    "a sweep takes axes ([[task.axis]]), not corners",
                ));
            }
            Some(ChipStudy::Sweep(task.axis.clone()))
        }
        "corners" => {
            if !task.axis.is_empty() {
                return Err(task_error(
                    "a corners study takes corners ([[task.corner]]), not axes",
                ));
            }
            Some(ChipStudy::Corners(super::study::corners_of(&task.corner)?))
        }
        other => {
            return Err(task_error(format!(
                "a job with a [circuit] table computes it by a \"spectrum\" task, studies it by a \
                 \"sweep\" or \"corners\" one, or optimizes it by an \"optimize\" one, not \
                 \"{other}\""
            )));
        }
    };
    let wavelengths = band(
        task.from_um,
        task.to_um,
        task.points,
        if study.is_some() { 101 } else { 501 },
    )?;
    chip.assemble()?;
    let designs = match &study {
        None => vec![chip.clone()],
        Some(s) => {
            let d = designs(&chip, s)?;
            for (k, c) in d.iter().enumerate() {
                c.assemble()
                    .map_err(|e| task_error(format!("the study's point {}: {e}", k + 1)))?;
            }
            d
        }
    };
    Ok(Prepared {
        chip,
        study,
        designs,
        wavelengths,
    })
}

/// [`super::check`] for a job with a `[circuit]` table.
pub(super) fn check(job: &Job) -> Result<()> {
    if job.device().is_some() {
        return Err(task_error(
            "a job holds a [device] or a [circuit], not both: a chip's devices are its instances",
        ));
    }
    if job.task().get("kind").and_then(Value::as_str) == Some("optimize") {
        return super::optimize::check(job);
    }
    prepare(job).map(|_| ())
}

/// Runs a job with a `[circuit]` table: its spectrum, or its study.
pub(super) fn execute(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    check(job)?;
    if job.task().get("kind").and_then(Value::as_str) == Some("optimize") {
        return Err(task_error(
            "an \"optimize\" job's optimizer is genoxide's, which the photonoxide program runs: \
             photonoxide run <job> (the studio window shows it live); the library supplies its \
             objectives and records it (photonoxide::job::Problem, Recorder)",
        ));
    }
    let p = prepare(job)?;
    run.record(&Event::Started {
        job: job.name().to_owned(),
        kind: "chip".into(),
    })?;
    let nominal = p.chip.assemble()?;
    let ports = p.chip.port_names();
    run.record(&Event::Chip {
        instances: p.chip.parts(&nominal),
        ports: ports.clone(),
    })?;
    match &p.study {
        None => {
            for &w in &p.wavelengths {
                if stop.reason().is_some() {
                    break;
                }
                let s = nominal.s_matrix(w)?;
                run.record(&Event::SParameters {
                    wavelength_um: w,
                    ports: ports.clone(),
                    effective_indices: vec![0.0; ports.len()],
                    s: recorded(&s),
                })?;
            }
        }
        Some(study) => {
            let (axes, corners) = match study {
                ChipStudy::Sweep(a) => (a.clone(), Vec::new()),
                ChipStudy::Corners(c) => (Vec::new(), c.clone()),
            };
            let units = axes
                .iter()
                .map(|a| p.chip.resolve(&a.parameter).map(|r| r.unit))
                .collect::<Result<Vec<_>>>()?;
            run.record(&Event::ChipStudy {
                study: study.kind().to_owned(),
                axis_units: units,
                axes: axes.clone(),
                corners: corners.clone(),
                points: p.designs.len(),
                wavelengths_um: p.wavelengths.clone(),
                ports: ports.clone(),
            })?;
            let batch = rayon::current_num_threads().max(1);
            super::sweep_points(run, stop, 0..p.designs.len(), batch, |k| point(&p, k, stop))?;
        }
    }
    let seconds = run.elapsed().as_secs_f64();
    run.record(&Event::Finished {
        stopped: reason(stop),
        seconds,
    })
}

/// Point `k` of a chip's study: its chip's S-matrices over the band, or why it has none; `None`
/// when the stop came first.
fn point(p: &Prepared, k: usize, stop: &Stop) -> Result<Option<Vec<Event>>> {
    if stop.reason().is_some() {
        return Ok(None);
    }
    let (at, corner) = match p.study.as_ref() {
        Some(ChipStudy::Sweep(axes)) => {
            let n2 = axes.get(1).map_or(1, |a| a.points);
            let mut at = vec![axes[0].values()[k / n2]];
            if let Some(b) = axes.get(1) {
                at.push(b.values()[k % n2]);
            }
            (at, None)
        }
        Some(ChipStudy::Corners(c)) => (
            Vec::new(),
            Some(if k == 0 {
                "nominal".to_owned()
            } else {
                c[k - 1].name.clone()
            }),
        ),
        None => (Vec::new(), None),
    };
    let solved = spectrum(&p.designs[k], &p.wavelengths);
    let (s, error) = match solved {
        Ok(s) => (s, None),
        Err(e) => (Vec::new(), Some(e.to_string())),
    };
    Ok(Some(vec![Event::ChipPoint {
        point: k,
        at,
        corner,
        error,
        s,
    }]))
}

/// A chip's S-matrices at `wavelengths`, as a run records them: the bits its `"spectrum"` job
/// records.
pub(super) fn spectrum(chip: &ChipDesign, wavelengths: &[f64]) -> Result<Vec<Vec<Vec<[f64; 2]>>>> {
    let a = chip.assemble()?;
    wavelengths
        .iter()
        .map(|&w| a.s_matrix(w).map(|s| recorded(&s)))
        .collect()
}

/// The job that computes `chip`'s spectrum over `band`, named `name`, with a hard limit of
/// `timeout_minutes`. It is checked by [`super::check`].
///
/// # Errors
///
/// The errors of [`super::check`] for the job it makes: an instance that doesn't build, a
/// netlist that doesn't compile, a band that isn't one.
pub fn spectrum_from_chip(
    name: &str,
    chip: &ChipDesign,
    band: (f64, f64, usize),
    timeout_minutes: f64,
) -> Result<Job> {
    let task = band_task("spectrum", band);
    chip_job(name, chip, task, timeout_minutes)
}

/// The job that studies `chip` as `axes` (a sweep of one or two instances' parameters, when
/// `corners` is empty) or `corners` (offsets of them) say, its spectrum over `band` at each
/// point, named `name`. It is checked by [`super::check`].
///
/// # Errors
///
/// [`Error::InvalidValue`] for both axes and corners or neither, and the errors of
/// [`super::check`] for the job it makes.
pub fn study_from_chip(
    name: &str,
    chip: &ChipDesign,
    band: (f64, f64, usize),
    axes: &[SweepAxis],
    corners: &[Corner],
    timeout_minutes: f64,
) -> Result<Job> {
    let mut task = match (axes.is_empty(), corners.is_empty()) {
        (false, true) => band_task("sweep", band),
        (true, false) => band_task("corners", band),
        _ => {
            return Err(Error::invalid(
                "chip",
                "a chip's study is a sweep (its axes) or corners, one of them",
            ));
        }
    };
    if !axes.is_empty() {
        task.insert(
            "axis".into(),
            Value::Array(
                axes.iter()
                    .map(|a| {
                        let mut t = Table::new();
                        t.insert("parameter".into(), Value::String(a.parameter.clone()));
                        t.insert("from".into(), Value::Float(a.from));
                        t.insert("to".into(), Value::Float(a.to));
                        t.insert("points".into(), Value::Integer(a.points as i64));
                        Value::Table(t)
                    })
                    .collect(),
            ),
        );
    }
    if !corners.is_empty() {
        task.insert(
            "corner".into(),
            Value::Array(corners.iter().map(super::optimize::corner_table).collect()),
        );
    }
    chip_job(name, chip, task, timeout_minutes)
}

/// A task of `kind` over `band`.
pub(super) fn band_task(kind: &str, band: (f64, f64, usize)) -> Table {
    let mut task = Table::new();
    task.insert("kind".into(), kind.into());
    task.insert("from_um".into(), Value::Float(band.0));
    task.insert("to_um".into(), Value::Float(band.1));
    task.insert("points".into(), Value::Integer(band.2 as i64));
    task
}

/// A job of `chip` and `task`, checked.
pub(super) fn chip_job(
    name: &str,
    chip: &ChipDesign,
    task: Table,
    timeout_minutes: f64,
) -> Result<Job> {
    let mut file = Table::new();
    file.insert("name".into(), name.into());
    file.insert("timeout_minutes".into(), Value::Float(timeout_minutes));
    file.insert("circuit".into(), Value::Table(chip.to_table()));
    file.insert("task".into(), Value::Table(task));
    let text = toml::to_string(&file).map_err(|e| task_error(e.to_string()))?;
    let job = Job::parse(&text)?;
    super::check(&job)?;
    Ok(job)
}

#[cfg(test)]
#[path = "chip_tests.rs"]
mod tests;
