//! A device's design as a job file holds it: the `[device]` table.
//!
//! ```yaml
//! name: narrow-coupler
//!
//! device:
//!   kind: directional coupler      # a device of device::library(), by its kind
//!   fidelity: analytic             # analytic, 2D or 3D, as the device has them; its cheapest by default
//!   stack: soi_220                 # soi_220 (the default), soi or nitride; or a table, below
//!
//!   parameters:                    # expressions (crate::expr); any left out take their defaults
//!     w: 450 nm
//!     width: w
//!     gap: w/2.5
//! ```
//!
//! A design with a compact model fitted to its runs ([`super::compact`]) names its file, next to
//! the job's (a relative path is from the job file's folder, [`crate::run::Job::dir`]), and
//! may then be computed at the compact fidelity:
//!
//! ```yaml
//! device:
//!   kind: all-pass ring
//!   fidelity: compact
//!   compact: ring.compact.json
//! ```
//!
//! A stack is a preset, `stack: soi` (2 µm of oxide under the core) or
//! `stack: { preset: nitride, bottom_oxide_um: 3 }`, or layers of the materials
//! catalogue's ([`crate::material::catalogue`]), bottom to top, the guides drawn on `core`:
//!
//! ```yaml
//! device:
//!   stack:
//!     core: Si
//!     substrate: si                # a catalogue id: its default model at its default conditions
//!     cladding: sio2
//!     layer:
//!       - name: BOX
//!         thickness_um: 2.0
//!         material: sio2
//!         background: sio2
//!
//!       - name: Si
//!         thickness_um: 0.22       # the device's `thickness` parameter sets the core's
//!         material: { id: si, model: li-1980 }   # a model, temperature (K) and composition
//!         background: sio2
//! ```

use toml::{Table, Value};

use crate::circuit::Fidelity;
use crate::material::Material;
use crate::material::catalogue::{self, Conditions};
use crate::stack::{Layer, LayerStack};
use crate::units::Length;
use crate::{Error, Result};

use super::{
    Cells, Design, Device, FdfdGrid, Mesh, Model2d, PcmShifter, RibWaveguide, StripBend,
    StripCoupler, StripMmi, StripMzi, StripRing, StripWaveguide, StripYBranch, TflnModulator,
    ThermoShifter,
};
use crate::circuit::components::MmiKind;

/// A device's design as a job's `[device]` table holds it: which device, on which stack, its
/// parameters as expressions, and the fidelity its figures and S-parameters come from.
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    /// The device, by its kind ([`Device::kind`]): `"waveguide"`, `"bend"`,
    /// `"directional coupler"`, `"all-pass ring"`, `"add-drop ring"`, `"rib waveguide"`,
    /// `"MMI 1x2"`, `"MMI 2x2"`, `"Y-branch"`, `"MZI"`, `"Y-branch MZI"`,
    /// `"thermo-optic phase shifter"`, `"doped thermo-optic phase shifter"`,
    /// `"in-guide thermo-optic phase shifter"`, `"GST phase-change phase shifter"`,
    /// `"Sb2S3 phase-change phase shifter"`, `"Sb2Se3 phase-change phase shifter"` or
    /// `"TFLN Mach-Zehnder modulator"` (on its own
    /// lithium niobate stack: the default stack, left out).
    pub kind: String,
    /// The fidelity it is computed at.
    pub fidelity: Fidelity,
    /// Its stack.
    pub stack: StackSpec,
    /// Its named parameters, (name, expression): the device's own and any others they use, as
    /// [`Design::new`] takes them. A file lists them by name.
    pub parameters: Vec<(String, String)>,
    /// The file of its compact model ([`super::compact::Fitted`]), if it has one: a path, from
    /// the job file's folder when relative ([`Spec::resolved_in`]). With it the device has the
    /// compact fidelity too.
    pub compact: Option<String>,
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
    /// The device `kind` at its defaults: on 220 nm SOI, at its cheapest fidelity (analytic;
    /// 2D for the rib and the MMIs, which have no analytic model), no parameter given.
    pub fn new(kind: &str) -> Spec {
        Spec {
            kind: kind.to_owned(),
            fidelity: cheapest(kind),
            stack: StackSpec::default(),
            parameters: Vec::new(),
            compact: None,
        }
    }

    /// The design a job's `[device]` table holds.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for a table that isn't one: a missing or unknown key, a value of the
    /// wrong type, an unknown fidelity.
    pub fn from_table(table: &Table) -> Result<Spec> {
        known(table, "[device]", &DEVICE_KEYS)?;
        let kind = string(table, "[device]", "kind")?
            .ok_or_else(|| parse("[device] needs a kind, e.g. kind = \"waveguide\""))?;
        let fidelity = match string(table, "[device]", "fidelity")? {
            None => cheapest(&kind),
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
        let compact = string(table, "[device]", "compact")?;
        if compact.as_deref().is_some_and(|c| c.trim().is_empty()) {
            return Err(parse("[device] compact must name a file"));
        }
        Ok(Spec {
            kind,
            fidelity,
            stack,
            parameters,
            compact,
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
        if let Some(c) = &self.compact {
            t.insert("compact".into(), Value::String(c.clone()));
        }
        t
    }

    /// The same design, its compact model's file found from `dir` (a job file's folder) when
    /// its path is relative.
    #[must_use]
    pub fn resolved_in(&self, dir: &std::path::Path) -> Spec {
        let mut s = self.clone();
        if let Some(c) = &self.compact
            && std::path::Path::new(c).is_relative()
        {
            s.compact = Some(dir.join(c).display().to_string());
        }
        s
    }

    /// `device` with this design's compact model, if it names one.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] and [`Error::Parse`] for a file that can't be read as a compact model, and
    /// [`Error::InvalidValue`] for a model of another device.
    fn with_compact(&self, device: Box<dyn Device>) -> Result<Box<dyn Device>> {
        let Some(path) = &self.compact else {
            return Ok(device);
        };
        let fitted = super::compact::load(std::path::Path::new(path))?;
        Ok(Box::new(super::compact::Compacted::new(
            device,
            fitted,
            &self.stack.to_value(),
        )?))
    }

    /// The device: the library's of this kind, drawn on this stack.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an unknown kind or a stack that can't be built: an unknown
    /// preset or material, a material's conditions outside its model's range, a core that
    /// isn't one of the layers.
    pub fn device(&self) -> Result<Box<dyn Device>> {
        let device = self.own_device()?;
        self.with_compact(device)
    }

    /// The library's device of this kind on this stack, without a compact model.
    fn own_device(&self) -> Result<Box<dyn Device>> {
        let (stack, core) = self.stack.build()?;
        Ok(match self.kind.as_str() {
            "waveguide" => Box::new(StripWaveguide::on(stack, &core)?),
            "bend" => Box::new(StripBend::on(stack, &core)?),
            "directional coupler" => Box::new(StripCoupler::on(stack, &core)?),
            "all-pass ring" => Box::new(StripRing::on(stack, &core, false)?),
            "add-drop ring" => Box::new(StripRing::on(stack, &core, true)?),
            "rib waveguide" => Box::new(RibWaveguide::on(stack, &core)?),
            "MMI 1x2" => Box::new(StripMmi::on(MmiKind::OneByTwo, stack, &core)?),
            "MMI 2x2" => Box::new(StripMmi::on(MmiKind::TwoByTwo, stack, &core)?),
            "Y-branch" => Box::new(StripYBranch::on(stack, &core)?),
            "MZI" => Box::new(StripMzi::on(stack, &core, false)?),
            "Y-branch MZI" => Box::new(StripMzi::on(stack, &core, true)?),
            "thermo-optic phase shifter" => Box::new(ThermoShifter::on(stack, &core, false)?),
            "doped thermo-optic phase shifter" => Box::new(ThermoShifter::on(stack, &core, true)?),
            "in-guide thermo-optic phase shifter" => {
                Box::new(ThermoShifter::in_guide_on(stack, &core)?)
            }
            "GST phase-change phase shifter" => Box::new(PcmShifter::on("gst", stack, &core)?),
            "Sb2S3 phase-change phase shifter" => Box::new(PcmShifter::on("sb2s3", stack, &core)?),
            "Sb2Se3 phase-change phase shifter" => {
                Box::new(PcmShifter::on("sb2se3", stack, &core)?)
            }
            super::tfln::KIND => {
                if self.stack != StackSpec::default() {
                    return Err(Error::invalid(
                        "device",
                        format!(
                            "a {} is on its own stack (a lithium niobate film on oxide over \
                             silicon, its thicknesses its parameters): leave the stack out",
                            super::tfln::KIND
                        ),
                    ));
                }
                Box::new(TflnModulator::new())
            }
            other => {
                return Err(Error::invalid(
                    "device",
                    format!(
                        "no device \"{other}\": waveguide, bend, directional coupler, all-pass \
                         ring, add-drop ring, rib waveguide, MMI 1x2, MMI 2x2, Y-branch, MZI, \
                         Y-branch MZI, thermo-optic phase shifter, doped thermo-optic phase \
                         shifter, in-guide thermo-optic phase shifter, GST, Sb2S3 or Sb2Se3 \
                         phase-change phase shifter or TFLN Mach-Zehnder modulator"
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
        if self.fidelity == Fidelity::Compact && self.compact.is_none() {
            return Err(Error::invalid(
                "device",
                format!(
                    "a {}'s compact model is fitted to its runs (its Model tab, or a job of kind \"fit\"); name its file in the design: compact = \"<file>.json\"",
                    device.kind()
                ),
            ));
        }
        super::method(device.kind(), device.fidelities(), self.fidelity)?;
        Ok((device, design))
    }
}

/// The grids a job sets on a device's solves (a `"spectrum"` task, [`crate::job`]): the cells'
/// size in the chip's plane, µm (a cross-section's mesh step for a mode solve, the cells of an
/// FDFD), the FDFD cells' height in 3D, the cladding kept beside the guides, µm, the
/// wavelengths an FDFD samples, the FDFD's PML cells, and an MMI's 2D model by FDFD in place of
/// its guided modes. Each left out is the device's own.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Grids {
    pub(crate) step: Option<f64>,
    pub(crate) step_z: Option<f64>,
    pub(crate) margin: Option<f64>,
    pub(crate) samples: Option<usize>,
    pub(crate) pml: Option<usize>,
    pub(crate) fdfd_2d: bool,
}

impl Grids {
    /// `mesh` with the step and margin set, when it is fitted to the core.
    fn mesh(&self, mesh: Mesh) -> Mesh {
        match mesh.cells {
            Cells::Fitted { step, margin } => Mesh {
                cells: Cells::Fitted {
                    step: self.step.unwrap_or(step),
                    margin: self.margin.unwrap_or(margin),
                },
                ..mesh
            },
            Cells::Window { .. } => mesh,
        }
    }

    /// `grid` with what is set; its averaging the device's own.
    fn fdfd(&self, grid: FdfdGrid) -> FdfdGrid {
        FdfdGrid {
            step: self.step.unwrap_or(grid.step),
            step_z: self.step_z.unwrap_or(grid.step_z),
            margin: self.margin.unwrap_or(grid.margin),
            samples: self.samples.unwrap_or(grid.samples),
            pml: self.pml.unwrap_or(grid.pml),
            averaging: grid.averaging,
        }
    }

    /// Refuses what a device whose solves are `solves` doesn't take: `"modes"` (a cross-section's
    /// mesh: a step and a margin), `"fdfd"` (all of them but the MMI's model, `"mmi"` all) or
    /// `"none"` (an MZI's, whose parts solve on their own grids).
    fn check(&self, kind: &str, solves: &str) -> Result<()> {
        let refuse = |what: &str, why: &str| {
            Err(Error::invalid(
                "device",
                format!("a {kind} takes no {what}: {why}"),
            ))
        };
        if solves == "none" && *self != Grids::default() {
            return refuse("grid", "its parts solve on their own grids");
        }
        if solves == "static" && *self != Grids::default() {
            return refuse(
                "grid",
                "its static and mode solves are on its own graded grids, which a validation \
                 case refines",
            );
        }
        if solves == "modes" {
            if self.step_z.is_some() {
                return refuse(
                    "step_z_nm",
                    "its solves are its cross-section's, on one mesh",
                );
            }
            if self.samples.is_some() {
                return refuse(
                    "samples",
                    "its mode solves sample five wavelengths across its span",
                );
            }
            if self.pml.is_some() {
                return refuse(
                    "pml_cells",
                    "its solves are its cross-section's modes, on a mesh of its own",
                );
            }
        }
        if self.fdfd_2d && solves != "mmi" {
            return refuse("model_2d", "its 2D model is its only one");
        }
        let ok = |v: Option<f64>| v.is_none_or(|v| v.is_finite() && v > 0.0);
        if !(ok(self.step) && ok(self.step_z) && ok(self.margin))
            || self.samples == Some(0)
            || self.pml.is_some_and(|n| n < 4)
        {
            return Err(Error::invalid(
                "device",
                format!(
                    "a grid needs positive steps, margin and samples, and 4 PML cells or more, \
                     got {self:?}"
                ),
            ));
        }
        Ok(())
    }
}

impl Spec {
    /// The device as [`Spec::device`] builds it, its solves on `grids`.
    ///
    /// # Errors
    ///
    /// Those of [`Spec::device`], and [`Error::InvalidValue`] for a grid the device doesn't
    /// take (a 3D FDFD's cells' height for a mode solve, any for an MZI).
    pub(crate) fn device_with(&self, grids: &Grids) -> Result<Box<dyn Device>> {
        if *grids == Grids::default() {
            return self.device();
        }
        let device = self.own_device_with(grids)?;
        self.with_compact(device)
    }

    /// [`Spec::device_with`] without a compact model.
    fn own_device_with(&self, grids: &Grids) -> Result<Box<dyn Device>> {
        let (stack, core) = self.stack.build()?;
        let kind = self.kind.as_str();
        let solves = match kind {
            "MMI 1x2" | "MMI 2x2" => "mmi",
            "Y-branch" => "fdfd",
            "MZI" | "Y-branch MZI" => "none",
            "thermo-optic phase shifter"
            | "doped thermo-optic phase shifter"
            | "in-guide thermo-optic phase shifter"
            | super::tfln::KIND => "static",
            _ => "modes",
        };
        grids.check(kind, solves)?;
        let g = grids;
        Ok(match kind {
            "waveguide" => {
                let d = StripWaveguide::on(stack, &core)?;
                let m = g.mesh(d.mesh());
                Box::new(d.with_mesh(m))
            }
            "bend" => {
                let d = StripBend::on(stack, &core)?;
                let (bent, straight) = d.meshes();
                Box::new(
                    d.with_mesh(g.mesh(bent))
                        .with_straight_mesh(g.mesh(straight)),
                )
            }
            "directional coupler" => {
                let d = StripCoupler::on(stack, &core)?;
                let m = g.mesh(d.mesh());
                Box::new(d.with_mesh(m))
            }
            "all-pass ring" | "add-drop ring" => {
                let d = StripRing::on(stack, &core, kind == "add-drop ring")?;
                let m = g.mesh(d.mesh());
                Box::new(d.with_mesh(m))
            }
            "rib waveguide" => {
                let d = RibWaveguide::on(stack, &core)?;
                let m = g.mesh(d.mesh());
                Box::new(d.with_mesh(m))
            }
            "MMI 1x2" | "MMI 2x2" => Box::new(self.mmi(stack, &core, g)?),
            "Y-branch" => {
                let d = StripYBranch::on(stack, &core)?;
                let (two, three) = d.grids();
                Box::new(d.with_grid_2d(g.fdfd(two)).with_grid_3d(g.fdfd(three)))
            }
            _ => self.own_device()?,
        })
    }

    /// The MMI this design is, on `grids`.
    fn mmi(&self, stack: LayerStack, core: &str, g: &Grids) -> Result<StripMmi> {
        let kind = if self.kind == "MMI 1x2" {
            MmiKind::OneByTwo
        } else {
            MmiKind::TwoByTwo
        };
        let mut d = StripMmi::on(kind, stack, core)?;
        if g.fdfd_2d {
            d = d.with_model_2d(Model2d::Fdfd);
        }
        let (two, three) = d.grids();
        Ok(d.with_grid_2d(g.fdfd(two)).with_grid_3d(g.fdfd(three)))
    }

    /// The size of the FDFD solve this design takes at its fidelity on `grids`, when its model is
    /// one (an MMI's or a Y-branch's 2D and 3D, an MMI's 2D by FDFD only): the cells along x
    /// and y and the unknowns of a solve; and refused, as the solve would refuse it, when a 3D
    /// one is more than a 3D solve holds here.
    ///
    /// # Errors
    ///
    /// Those of [`Spec::build`] and [`Spec::device_with`], and [`Error::InvalidValue`] for a 3D
    /// solve too large.
    pub(crate) fn fdfd_size(&self, grids: &Grids) -> Result<Option<(usize, usize, f64)>> {
        let (_, design) = self.build()?;
        if self.fidelity == Fidelity::Compact {
            // the grids checked against the device all the same, as at any fidelity
            self.device_with(grids)?;
            return Ok(None);
        }
        let values = design.values();
        // the grids checked against the device as its solve would take them
        self.device_with(grids)?;
        let (stack, core) = self.stack.build()?;
        let size = match self.kind.as_str() {
            "MMI 1x2" | "MMI 2x2" => self
                .mmi(stack, &core, grids)?
                .fdfd_size(values, self.fidelity)?,
            "Y-branch" => {
                let d = StripYBranch::on(stack, &core)?;
                let (two, three) = d.grids();
                d.with_grid_2d(grids.fdfd(two))
                    .with_grid_3d(grids.fdfd(three))
                    .fdfd_size(values, self.fidelity)?
            }
            _ => None,
        };
        if let Some((_, _, unknowns)) = size
            && self.fidelity == Fidelity::ThreeD
        {
            super::solve::check_size(unknowns)?;
        }
        Ok(size)
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
                let stack = layered(substrate, layers, cladding)?;
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
                        .map(|(k, l)| {
                            LayerSpec::from_value(l, &format!("[[device.stack.layer]] {}", k + 1))
                        })
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

    pub(crate) fn to_value(&self) -> Value {
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

/// The stack of `layers`, bottom to top, between `substrate` and `cladding`.
///
/// # Errors
///
/// [`Error::InvalidValue`] for an unknown material or one [`MaterialSpec::material`] refuses,
/// or layers [`LayerStack::new`] refuses.
pub(crate) fn layered(
    substrate: &MaterialSpec,
    layers: &[LayerSpec],
    cladding: &MaterialSpec,
) -> Result<LayerStack> {
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
    LayerStack::new(substrate.material()?, layers, cladding.material()?)
}

/// A job task's `[task.stack]` table: a substrate, a cladding and `[[task.stack.layer]]`s, as a
/// device's `[device.stack]` has them but with no core (the task names its own layer).
///
/// # Errors
///
/// [`Error::Parse`] for a table that isn't one, and the errors of [`layered`].
pub(crate) fn task_stack(t: &Table) -> Result<LayerStack> {
    let at = "[task.stack]";
    known(t, at, &["substrate", "cladding", "layer"])?;
    let medium = |key: &str| -> Result<Material> {
        task_medium(
            t.get(key)
                .ok_or_else(|| parse(format!("{at} needs a {key}, e.g. {key} = \"sio2\"")))?,
            &format!("{at} {key}"),
        )
    };
    let layers = match t.get("layer") {
        Some(Value::Array(a)) if !a.is_empty() => a
            .iter()
            .enumerate()
            .map(|(k, l)| task_layer(l, &format!("[[task.stack.layer]] {}", k + 1)))
            .collect::<Result<Vec<_>>>()?,
        _ => {
            return Err(parse(format!(
                "{at} needs its layers, bottom to top: [[task.stack.layer]]"
            )));
        }
    };
    LayerStack::new(medium("substrate")?, layers, medium("cladding")?)
}

/// A layer of a task's `[task.stack]`, as [`LayerSpec::from_value`] reads a device's, its media
/// as [`task_medium`] reads them.
fn task_layer(v: &Value, at: &str) -> Result<Layer> {
    let Value::Table(t) = v else {
        return Err(parse(format!("{at} must be a table")));
    };
    known(t, at, &["name", "thickness_um", "material", "background"])?;
    let medium = |key: &str| -> Result<Material> {
        task_medium(
            t.get(key)
                .ok_or_else(|| parse(format!("{at} needs a {key}")))?,
            &format!("{at} {key}"),
        )
    };
    Ok(Layer {
        name: string(t, at, "name")?.ok_or_else(|| parse(format!("{at} needs a name")))?,
        thickness: Length::um(
            number(t, at, "thickness_um")?
                .ok_or_else(|| parse(format!("{at} needs a thickness_um")))?,
        ),
        material: medium("material")?,
        background: medium("background")?,
    })
}

/// A medium of a task's `[task.stack]`: a catalogue entry, as [`MaterialSpec::from_value`] reads
/// it, or a number, a fixed real refractive index of at least 1, the same at every wavelength:
/// a benchmark's medium, such as Bienstman et al.'s silicon of 3.5, or air.
///
/// A catalogue crystal's table may give its `orientation` ([`crate::material::Orientation`]:
/// `"x-cut"`, a `cut` and `along_x`, or `euler_deg`): the medium is then the crystal turned onto
/// the chip, its tensor in the chip's frame ([`Material::oriented`]).
fn task_medium(v: &Value, at: &str) -> Result<Material> {
    #[allow(clippy::cast_precision_loss)]
    let index = match v {
        Value::Float(x) => *x,
        Value::Integer(i) => *i as f64,
        Value::Table(t) if t.contains_key("orientation") => {
            let orientation = crate::material::Orientation::from_value(
                &t["orientation"],
                &format!("{at}, orientation"),
            )?;
            let mut rest = t.clone();
            rest.remove("orientation");
            let spec = MaterialSpec::from_value(&Value::Table(rest), at)?;
            let entry = catalogue::entry(&spec.id).ok_or_else(|| {
                Error::invalid(
                    "material",
                    format!("no material \"{}\" in the catalogue", spec.id),
                )
            })?;
            return entry.oriented(
                spec.model.as_deref(),
                Conditions {
                    temperature: spec.temperature,
                    composition: spec.composition,
                },
                orientation,
            );
        }
        other => return MaterialSpec::from_value(other, at)?.material(),
    };
    if !(index.is_finite() && index >= 1.0) {
        return Err(parse(format!(
            "{at}: a fixed refractive index must be a number of at least 1, got {index}"
        )));
    }
    Material::new(
        format!("n = {index}"),
        crate::material::Model::Constant(num_complex::Complex64::new(index, 0.0)),
        crate::units::Wavelength::um(f64::MIN_POSITIVE)?,
        crate::units::Wavelength::um(f64::MAX)?,
        crate::material::Provenance {
            reference: "given by the job".into(),
            doi: String::new(),
            data: String::new(),
            temperature: None,
            notes: "a fixed refractive index, the same at every wavelength".into(),
        },
    )
}

impl LayerSpec {
    /// A layer's table, named `at` in errors (`[[device.stack.layer]] 2`).
    pub(crate) fn from_value(v: &Value, at: &str) -> Result<LayerSpec> {
        let at = at.to_owned();
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
                    "{}'s {} is anisotropic: a device's layers and a body take isotropic \
                     materials; a modes or structure job's [task.stack] takes it with its \
                     orientation, e.g. {{ id = \"{}\", orientation = \"x-cut\" }}",
                    self.id, model.id, self.id
                ),
            ));
        }
        Ok(materials.remove(0))
    }

    pub(crate) fn from_value(v: &Value, at: &str) -> Result<MaterialSpec> {
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

/// The keys a `[device]` table takes.
pub(crate) const DEVICE_KEYS: [&str; 5] = ["kind", "fidelity", "stack", "parameters", "compact"];

/// The names [`fidelity`] reads, as a job writes them.
pub(crate) const FIDELITIES: [&str; 7] =
    ["analytic", "compact", "2D", "3D", "measured", "2d", "3d"];

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

/// The cheapest fidelity of the device `kind`, a `[device]` table's when it names none: 2D for
/// the rib and the MMIs, which have no analytic model; analytic for the others (and an unknown
/// kind, which building refuses).
fn cheapest(kind: &str) -> Fidelity {
    match kind {
        "rib waveguide" | "MMI 1x2" | "MMI 2x2" => Fidelity::TwoD,
        _ => Fidelity::Analytic,
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
