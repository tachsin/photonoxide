//! A device's design as a job file holds it: the `[device]` table.
//!
//! ```toml
//! name = "narrow-coupler"
//!
//! [device]
//! kind = "directional coupler"   # a device of device::library(), by its kind
//! fidelity = "analytic"          # analytic (the default), 2D or 3D, as the device has them
//! stack = "soi_220"              # soi_220 (the default), soi or nitride; or a table, below
//!
//! [device.parameters]            # expressions (crate::expr); any left out take their defaults
//! w = "450 nm"
//! width = "w"
//! gap = "w/2.5"
//! ```
//!
//! A stack is a preset, `stack = "soi"` (2 µm of oxide under the core) or
//! `stack = { preset = "nitride", bottom_oxide_um = 3 }`, or layers of the materials
//! catalogue's ([`crate::material::catalogue`]), bottom to top, the guides drawn on `core`:
//!
//! ```toml
//! [device.stack]
//! core = "Si"
//! substrate = "si"               # a catalogue id: its default model at its default conditions
//! cladding = "sio2"
//!
//! [[device.stack.layer]]
//! name = "BOX"
//! thickness_um = 2.0
//! material = "sio2"
//! background = "sio2"
//!
//! [[device.stack.layer]]
//! name = "Si"
//! thickness_um = 0.22           # the device's `thickness` parameter sets the core's
//! material = { id = "si", model = "li-1980" }   # a model, temperature (K) and composition
//! background = "sio2"
//! ```

use toml::{Table, Value};

use crate::circuit::Fidelity;
use crate::material::Material;
use crate::material::catalogue::{self, Conditions};
use crate::stack::{Layer, LayerStack};
use crate::units::Length;
use crate::{Error, Result};

use super::{Design, Device, StripBend, StripCoupler, StripRing, StripWaveguide};

/// A device's design as a job's `[device]` table holds it: which device, on which stack, its
/// parameters as expressions, and the fidelity its figures and S-parameters come from.
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    /// The device, by its kind ([`Device::kind`]): `"waveguide"`, `"bend"`,
    /// `"directional coupler"`, `"all-pass ring"` or `"add-drop ring"`.
    pub kind: String,
    /// The fidelity it is computed at.
    pub fidelity: Fidelity,
    /// Its stack.
    pub stack: StackSpec,
    /// Its named parameters, (name, expression): the device's own and any others they use, as
    /// [`Design::new`] takes them. A file lists them by name.
    pub parameters: Vec<(String, String)>,
}

/// A device's stack: a preset, or layers of the catalogue's materials.
#[derive(Clone, Debug, PartialEq)]
pub enum StackSpec {
    /// `"soi_220"` ([`LayerStack::soi_220`]), `"soi"` ([`LayerStack::soi`]) or `"nitride"`
    /// ([`LayerStack::nitride`]), the oxide under the core `bottom_oxide_um` thick (2 µm for
    /// soi_220); the core is the device's thickness.
    Preset {
        /// The preset's name.
        name: String,
        /// The oxide under the core, µm.
        bottom_oxide_um: f64,
    },
    /// Layers, bottom to top, between a substrate and a cladding, the guides drawn on `core`.
    Layers {
        /// Below the layers.
        substrate: MaterialSpec,
        /// The layers, bottom to top.
        layers: Vec<LayerSpec>,
        /// Above them.
        cladding: MaterialSpec,
        /// The layer the guides are drawn on; the device's thickness is its thickness.
        core: String,
    },
}

/// A layer of a [`StackSpec::Layers`].
#[derive(Clone, Debug, PartialEq)]
pub struct LayerSpec {
    /// Its name, e.g. `"Si"`.
    pub name: String,
    /// Its thickness, µm.
    pub thickness_um: f64,
    /// What shapes on it are made of.
    pub material: MaterialSpec,
    /// What fills it elsewhere.
    pub background: MaterialSpec,
}

/// A material of the catalogue: an entry, its index model (the default when `None`), and the
/// conditions the model takes (each at its default when `None`).
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialSpec {
    /// The entry's id, e.g. `"si"`.
    pub id: String,
    /// The index model's id, e.g. `"li-1980"`.
    pub model: Option<String>,
    /// The temperature, K, for a model that takes one.
    pub temperature: Option<f64>,
    /// The composition, for a model that takes one.
    pub composition: Option<f64>,
}

/// The presets, by name: soi_220, soi and nitride.
pub const PRESETS: [&str; 3] = ["soi_220", "soi", "nitride"];

impl Spec {
    /// The device `kind` at its defaults: on 220 nm SOI, analytic, no parameter given.
    pub fn new(kind: &str) -> Spec {
        Spec {
            kind: kind.to_owned(),
            fidelity: Fidelity::Analytic,
            stack: StackSpec::default(),
            parameters: Vec::new(),
        }
    }

    /// The design a job's `[device]` table holds.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for a table that isn't one: a missing or unknown key, a value of the
    /// wrong type, an unknown fidelity.
    pub fn from_table(table: &Table) -> Result<Spec> {
        known(
            table,
            "[device]",
            &["kind", "fidelity", "stack", "parameters"],
        )?;
        let kind = string(table, "[device]", "kind")?
            .ok_or_else(|| parse("[device] needs a kind, e.g. kind = \"waveguide\""))?;
        let fidelity = match string(table, "[device]", "fidelity")? {
            None => Fidelity::Analytic,
            Some(f) => fidelity(&f)?,
        };
        let stack = match table.get("stack") {
            None => StackSpec::default(),
            Some(v) => StackSpec::from_value(v)?,
        };
        let parameters = match table.get("parameters") {
            None => Vec::new(),
            Some(Value::Table(t)) => t
                .iter()
                .map(|(name, v)| match v {
                    Value::String(s) => Ok((name.clone(), s.clone())),
                    Value::Integer(i) => Ok((name.clone(), i.to_string())),
                    Value::Float(x) => Ok((name.clone(), format!("{x:?}"))),
                    _ => Err(parse(format!(
                        "[device.parameters] {name}: must be an expression in a string, e.g. \
                         \"500 nm\", or a number"
                    ))),
                })
                .collect::<Result<_>>()?,
            Some(_) => {
                return Err(parse(
                    "[device.parameters] must be a table: name = \"expression\"",
                ));
            }
        };
        Ok(Spec {
            kind,
            fidelity,
            stack,
            parameters,
        })
    }

    /// The design as a `[device]` table, which [`Spec::from_table`] reads back to the same
    /// design (its parameters by name).
    pub fn to_table(&self) -> Table {
        let mut t = Table::new();
        t.insert("kind".into(), Value::String(self.kind.clone()));
        t.insert(
            "fidelity".into(),
            Value::String(fidelity_name(self.fidelity).into()),
        );
        t.insert("stack".into(), self.stack.to_value());
        if !self.parameters.is_empty() {
            let p: Table = self
                .parameters
                .iter()
                .map(|(n, e)| (n.clone(), Value::String(e.clone())))
                .collect();
            t.insert("parameters".into(), Value::Table(p));
        }
        t
    }

    /// The device: the library's of this kind, drawn on this stack.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an unknown kind or a stack that can't be built: an unknown
    /// preset or material, a material's conditions outside its model's range, a core that
    /// isn't one of the layers.
    pub fn device(&self) -> Result<Box<dyn Device>> {
        let (stack, core) = self.stack.build()?;
        Ok(match self.kind.as_str() {
            "waveguide" => Box::new(StripWaveguide::on(stack, &core)?),
            "bend" => Box::new(StripBend::on(stack, &core)?),
            "directional coupler" => Box::new(StripCoupler::on(stack, &core)?),
            "all-pass ring" => Box::new(StripRing::on(stack, &core, false)?),
            "add-drop ring" => Box::new(StripRing::on(stack, &core, true)?),
            other => {
                return Err(Error::invalid(
                    "device",
                    format!(
                        "no device \"{other}\": waveguide, bend, directional coupler, all-pass \
                         ring or add-drop ring"
                    ),
                ));
            }
        })
    }

    /// The device and its design, checked: the parameters' expressions evaluate, fit their
    /// parameters' units and ranges, and the device has the fidelity.
    ///
    /// # Errors
    ///
    /// Those of [`Spec::device`] and [`Design::new`], and [`Error::InvalidValue`] for a
    /// fidelity the device hasn't.
    pub fn build(&self) -> Result<(Box<dyn Device>, Design)> {
        let device = self.device()?;
        let definitions: Vec<(&str, &str)> = self
            .parameters
            .iter()
            .map(|(n, e)| (n.as_str(), e.as_str()))
            .collect();
        let design = Design::new(device.as_ref(), &definitions)?;
        super::method(device.kind(), device.fidelities(), self.fidelity)?;
        Ok((device, design))
    }
}

impl Default for StackSpec {
    fn default() -> StackSpec {
        StackSpec::preset("soi_220")
    }
}

impl StackSpec {
    /// The preset `name` with 2 µm of oxide under its core.
    pub fn preset(name: &str) -> StackSpec {
        StackSpec::Preset {
            name: name.to_owned(),
            bottom_oxide_um: 2.0,
        }
    }

    /// The stack and its core layer's name.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an unknown preset or material, an anisotropic one, a
    /// material's conditions outside its model's range, layers [`LayerStack::new`] refuses, or
    /// a core that isn't a layer.
    pub fn build(&self) -> Result<(LayerStack, String)> {
        match self {
            StackSpec::Preset {
                name,
                bottom_oxide_um,
            } => {
                let oxide = Length::um(*bottom_oxide_um);
                // the core's thickness is the device's: 220 nm until it is set
                let core = Length::nm(220.0);
                match name.as_str() {
                    "soi_220" if *bottom_oxide_um == 2.0 => {
                        Ok((LayerStack::soi_220(), "Si".into()))
                    }
                    "soi_220" => Err(Error::invalid(
                        "stack",
                        "soi_220 has 2 µm of oxide: take soi for another",
                    )),
                    "soi" => Ok((LayerStack::soi(core, oxide)?, "Si".into())),
                    "nitride" => Ok((LayerStack::nitride(core, oxide)?, "SiN".into())),
                    other => Err(Error::invalid(
                        "stack",
                        format!("no preset \"{other}\": soi_220, soi or nitride"),
                    )),
                }
            }
            StackSpec::Layers {
                substrate,
                layers,
                cladding,
                core,
            } => {
                let layers = layers
                    .iter()
                    .map(|l| {
                        Ok(Layer {
                            name: l.name.clone(),
                            thickness: Length::um(l.thickness_um),
                            material: l.material.material()?,
                            background: l.background.material()?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let stack = LayerStack::new(substrate.material()?, layers, cladding.material()?)?;
                if stack.layer(core).is_none() {
                    return Err(Error::invalid(
                        "stack",
                        format!("the core, {core}, isn't one of its layers"),
                    ));
                }
                Ok((stack, core.clone()))
            }
        }
    }

    /// The same stack as layers of the catalogue's materials: a preset's own (Li's silicon,
    /// Malitson's silica, Luke's nitride, which the presets are), to edit.
    pub fn layers(&self) -> StackSpec {
        let StackSpec::Preset {
            name,
            bottom_oxide_um,
        } = self
        else {
            return self.clone();
        };
        let m = MaterialSpec::id;
        let (core, material) = if name == "nitride" {
            ("SiN", "si3n4")
        } else {
            ("Si", "si")
        };
        StackSpec::Layers {
            substrate: m("si"),
            layers: vec![
                LayerSpec {
                    name: "BOX".into(),
                    thickness_um: *bottom_oxide_um,
                    material: m("sio2"),
                    background: m("sio2"),
                },
                LayerSpec {
                    name: core.into(),
                    thickness_um: 0.22,
                    material: m(material),
                    background: m("sio2"),
                },
            ],
            cladding: m("sio2"),
            core: core.into(),
        }
    }

    fn from_value(v: &Value) -> Result<StackSpec> {
        match v {
            Value::String(name) => Ok(StackSpec::preset(name)),
            Value::Table(t) if t.contains_key("preset") => {
                known(t, "[device.stack]", &["preset", "bottom_oxide_um"])?;
                let name = string(t, "[device.stack]", "preset")?.unwrap_or_default();
                let bottom_oxide_um =
                    number(t, "[device.stack]", "bottom_oxide_um")?.unwrap_or(2.0);
                Ok(StackSpec::Preset {
                    name,
                    bottom_oxide_um,
                })
            }
            Value::Table(t) => {
                let at = "[device.stack]";
                known(t, at, &["core", "substrate", "cladding", "layer"])?;
                let medium = |key: &str| -> Result<MaterialSpec> {
                    MaterialSpec::from_value(
                        t.get(key)
                            .ok_or_else(|| parse(format!("{at} needs a {key}")))?,
                        &format!("{at} {key}"),
                    )
                };
                let layers = match t.get("layer") {
                    Some(Value::Array(a)) => a
                        .iter()
                        .enumerate()
                        .map(|(k, l)| LayerSpec::from_value(l, k))
                        .collect::<Result<Vec<_>>>()?,
                    _ => {
                        return Err(parse(format!(
                            "{at} needs its layers: [[device.stack.layer]]"
                        )));
                    }
                };
                Ok(StackSpec::Layers {
                    substrate: medium("substrate")?,
                    cladding: medium("cladding")?,
                    core: string(t, at, "core")?.ok_or_else(|| {
                        parse(format!("{at} needs its core layer: core = \"Si\""))
                    })?,
                    layers,
                })
            }
            _ => Err(parse(
                "[device] stack must be a preset's name (soi_220, soi or nitride) or a table",
            )),
        }
    }

    fn to_value(&self) -> Value {
        match self {
            StackSpec::Preset {
                name,
                bottom_oxide_um,
            } if *bottom_oxide_um == 2.0 => Value::String(name.clone()),
            StackSpec::Preset {
                name,
                bottom_oxide_um,
            } => {
                let mut t = Table::new();
                t.insert("preset".into(), Value::String(name.clone()));
                t.insert("bottom_oxide_um".into(), Value::Float(*bottom_oxide_um));
                Value::Table(t)
            }
            StackSpec::Layers {
                substrate,
                layers,
                cladding,
                core,
            } => {
                let mut t = Table::new();
                t.insert("core".into(), Value::String(core.clone()));
                t.insert("substrate".into(), substrate.to_value());
                t.insert("cladding".into(), cladding.to_value());
                t.insert(
                    "layer".into(),
                    Value::Array(layers.iter().map(LayerSpec::to_value).collect()),
                );
                Value::Table(t)
            }
        }
    }
}

impl LayerSpec {
    fn from_value(v: &Value, k: usize) -> Result<LayerSpec> {
        let at = format!("[[device.stack.layer]] {}", k + 1);
        let Value::Table(t) = v else {
            return Err(parse(format!("{at} must be a table")));
        };
        known(t, &at, &["name", "thickness_um", "material", "background"])?;
        let medium = |key: &str| -> Result<MaterialSpec> {
            MaterialSpec::from_value(
                t.get(key)
                    .ok_or_else(|| parse(format!("{at} needs a {key}")))?,
                &format!("{at} {key}"),
            )
        };
        Ok(LayerSpec {
            name: string(t, &at, "name")?.ok_or_else(|| parse(format!("{at} needs a name")))?,
            thickness_um: number(t, &at, "thickness_um")?
                .ok_or_else(|| parse(format!("{at} needs a thickness_um")))?,
            material: medium("material")?,
            background: medium("background")?,
        })
    }

    fn to_value(&self) -> Value {
        let mut t = Table::new();
        t.insert("name".into(), Value::String(self.name.clone()));
        t.insert("thickness_um".into(), Value::Float(self.thickness_um));
        t.insert("material".into(), self.material.to_value());
        t.insert("background".into(), self.background.to_value());
        Value::Table(t)
    }
}

impl MaterialSpec {
    /// The catalogue's entry `id`, its default model at its default conditions.
    pub fn id(id: &str) -> MaterialSpec {
        MaterialSpec {
            id: id.to_owned(),
            model: None,
            temperature: None,
            composition: None,
        }
    }

    /// The material.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an unknown entry or model, an anisotropic model, or
    /// conditions outside its range.
    pub fn material(&self) -> Result<Material> {
        let entry = catalogue::entry(&self.id).ok_or_else(|| {
            Error::invalid(
                "material",
                format!("no material \"{}\" in the catalogue", self.id),
            )
        })?;
        let model = match &self.model {
            None => entry.default_model().cloned(),
            Some(m) => entry.index.iter().find(|i| i.id == *m).cloned(),
        }
        .ok_or_else(|| {
            Error::invalid(
                "material",
                format!(
                    "{} has no index model {}",
                    self.id,
                    self.model.as_deref().unwrap_or("")
                ),
            )
        })?;
        let mut materials = model.materials(Conditions {
            temperature: self.temperature,
            composition: self.composition,
        })?;
        if materials.len() != 1 {
            return Err(Error::invalid(
                "material",
                format!(
                    "{}'s {} is anisotropic: a device's layers take isotropic materials",
                    self.id, model.id
                ),
            ));
        }
        Ok(materials.remove(0))
    }

    fn from_value(v: &Value, at: &str) -> Result<MaterialSpec> {
        match v {
            Value::String(id) => Ok(MaterialSpec::id(id)),
            Value::Table(t) => {
                known(t, at, &["id", "model", "temperature", "composition"])?;
                Ok(MaterialSpec {
                    id: string(t, at, "id")?
                        .ok_or_else(|| parse(format!("{at} needs an id, e.g. id = \"si\"")))?,
                    model: string(t, at, "model")?,
                    temperature: number(t, at, "temperature")?,
                    composition: number(t, at, "composition")?,
                })
            }
            _ => Err(parse(format!(
                "{at} must be a catalogue id, e.g. \"si\", or a table with one"
            ))),
        }
    }

    fn to_value(&self) -> Value {
        if self.model.is_none() && self.temperature.is_none() && self.composition.is_none() {
            return Value::String(self.id.clone());
        }
        let mut t = Table::new();
        t.insert("id".into(), Value::String(self.id.clone()));
        if let Some(m) = &self.model {
            t.insert("model".into(), Value::String(m.clone()));
        }
        if let Some(x) = self.temperature {
            t.insert("temperature".into(), Value::Float(x));
        }
        if let Some(x) = self.composition {
            t.insert("composition".into(), Value::Float(x));
        }
        Value::Table(t)
    }
}

/// A fidelity by the name it is written with: analytic, compact, 2D, 3D or measured.
///
/// # Errors
///
/// [`Error::Parse`] for another name.
pub fn fidelity(name: &str) -> Result<Fidelity> {
    match name {
        "analytic" => Ok(Fidelity::Analytic),
        "compact" => Ok(Fidelity::Compact),
        "2D" | "2d" => Ok(Fidelity::TwoD),
        "3D" | "3d" => Ok(Fidelity::ThreeD),
        "measured" => Ok(Fidelity::Measured),
        other => Err(parse(format!(
            "[device] fidelity \"{other}\": analytic, compact, 2D, 3D or measured"
        ))),
    }
}

/// The name a fidelity is written with.
fn fidelity_name(f: Fidelity) -> &'static str {
    match f {
        Fidelity::Analytic => "analytic",
        Fidelity::Compact => "compact",
        Fidelity::TwoD => "2D",
        Fidelity::ThreeD => "3D",
        Fidelity::Measured => "measured",
    }
}

fn parse(reason: impl Into<String>) -> Error {
    Error::Parse {
        what: "job".into(),
        reason: reason.into(),
    }
}

/// Refuses a key of `t` that isn't one of `keys`.
fn known(t: &Table, at: &str, keys: &[&str]) -> Result<()> {
    match t.keys().find(|k| !keys.contains(&k.as_str())) {
        Some(k) => Err(parse(format!(
            "{at}: unknown key {k}; it takes {}",
            keys.join(", ")
        ))),
        None => Ok(()),
    }
}

fn string(t: &Table, at: &str, key: &str) -> Result<Option<String>> {
    match t.get(key) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(parse(format!("{at} {key} must be a string"))),
    }
}

fn number(t: &Table, at: &str, key: &str) -> Result<Option<f64>> {
    match t.get(key) {
        None => Ok(None),
        Some(Value::Float(x)) => Ok(Some(*x)),
        #[allow(clippy::cast_precision_loss)]
        Some(Value::Integer(i)) => Ok(Some(*i as f64)),
        Some(_) => Err(parse(format!("{at} {key} must be a number"))),
    }
}
