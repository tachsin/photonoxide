//! Devices: the components that know their geometry ([`crate::device`]), as plain data:
//! [`devices`], [`stacks`], [`device_spectrum`] (and [`device_spectrum_until`], which a
//! [`Stop`] stops), [`device_derived`], [`device_geometry`] and [`device_job`].

use std::path::PathBuf;

use crate::Complex64 as c64;
use crate::device::{self, Design, Device, Spec, StackSpec};
use crate::geometry::{Point, Region, Tolerance};
use crate::run::Format;
use crate::units::Wavelength;
use crate::{Error, Result};

use super::{Spectrum, Stop};

/// A device of the library ([`crate::device::library`]): what it is, its parameters, its models
/// and its ports.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DeviceInfo {
    /// Its kind, what a [`DeviceDesign`] names it by, e.g. `"directional coupler"`.
    pub kind: String,
    /// What it is, in a sentence or two.
    pub about: String,
    /// Its parameters, in order.
    pub parameters: Vec<DeviceParameter>,
    /// Its models, cheapest first: the first is a design's when it names none.
    pub fidelities: Vec<DeviceModel>,
    /// Its ports' names, in the order of its S-matrices.
    pub ports: Vec<String>,
}

/// A parameter of a device: its name, unit, default and range.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DeviceParameter {
    /// Its name, e.g. `"gap"`.
    pub name: String,
    /// Its unit, e.g. `"µm"` or `"°"`, or `""` for a pure number.
    pub unit: String,
    /// Its default.
    pub default: f64,
    /// The smallest value it takes.
    pub min: f64,
    /// The largest.
    pub max: f64,
}

/// One of a device's models ([`crate::device::Method`]): its fidelity, what computes it, its
/// write-up and papers, its stated error against the next fidelity up, and where it holds.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DeviceModel {
    /// `"analytic"`, `"compact"`, `"2D"`, `"3D"` or `"measured"`.
    pub fidelity: String,
    /// What computes it, in a sentence.
    pub name: String,
    /// Its write-up in docs/methods, e.g. `"devices"` (docs/methods/devices.md).
    pub write_up: String,
    /// The papers it implements, each with its DOI.
    pub papers: Vec<String>,
    /// Its error against the next fidelity up over its validity; `None` for the highest.
    pub error: Option<StatedError>,
    /// Where it holds: a range for each parameter it limits, and `"wavelength"` (µm).
    pub validity: Vec<ValidityRange>,
}

/// A model's stated error: the largest difference in one quantity against another fidelity.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StatedError {
    /// The quantity, e.g. `"n_eff"`.
    pub quantity: String,
    /// The largest difference, in `unit`.
    pub bound: f64,
    /// Its unit: `""` for a pure number, `"%"` for a relative one.
    pub unit: String,
    /// The fidelity it is measured against.
    pub against: String,
}

/// A range a model holds over, for one parameter or the wavelength.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ValidityRange {
    /// The parameter's name, or `"wavelength"`.
    pub name: String,
    /// The smallest value.
    pub min: f64,
    /// The largest.
    pub max: f64,
}

/// A device's design as data: the device, its parameters' values, the fidelity it is computed
/// at, its stack, and the file of its compact model if it has one.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DeviceDesign {
    /// The device's kind ([`DeviceInfo::kind`]), e.g. `"directional coupler"`.
    pub kind: String,
    /// Values of its parameters, (name, value) in the parameters' units; those not given take
    /// their defaults.
    pub values: Vec<(String, f64)>,
    /// `"analytic"`, `"2D"`, `"3D"`, ... as the device has them; its cheapest when `None`.
    pub fidelity: Option<String>,
    /// Its stack, by the name of one of the catalogue's ([`stacks`]): `"soi_220"` (the default,
    /// when `None`), `"soi"` or `"nitride"`, the core as thick as the device's `thickness` and
    /// 2 µm of oxide under it.
    pub stack: Option<String>,
    /// The file of a compact model fitted to the device's runs (`compact.json`, which a run of a
    /// `"fit"` job writes in its folder, [`crate::job::fit_from_device`]): the design then has the
    /// compact fidelity too, over the ranges the model covers.
    pub compact: Option<PathBuf>,
}

impl DeviceDesign {
    /// The device `kind` at its defaults, its cheapest fidelity, on 220 nm SOI.
    pub fn new(kind: &str) -> DeviceDesign {
        DeviceDesign {
            kind: kind.to_owned(),
            values: Vec::new(),
            fidelity: None,
            stack: None,
            compact: None,
        }
    }

    /// The design as a job's `[device]` table holds it: each value written as the shortest
    /// decimal that reads back to it, so the design's values are these to the bit.
    fn spec(&self) -> Result<Spec> {
        let mut spec = Spec::new(&self.kind);
        let template = spec.device()?;
        let names: Vec<&str> = template
            .parameters()
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        for (name, value) in &self.values {
            if !names.contains(&name.as_str()) {
                return Err(Error::invalid(
                    "device",
                    format!(
                        "a {} has no parameter {name}; its parameters are {}",
                        self.kind,
                        names.join(", ")
                    ),
                ));
            }
            if !value.is_finite() {
                return Err(Error::invalid(
                    "device",
                    format!("a {}'s {name} must be finite, got {value}", self.kind),
                ));
            }
            spec.parameters.push((name.clone(), format!("{value:?}")));
        }
        if let Some(f) = &self.fidelity {
            spec.fidelity = device::fidelity(f).map_err(|_| {
                Error::invalid(
                    "fidelity",
                    format!(
                        "is \"analytic\", \"compact\", \"2D\", \"3D\" or \"measured\", not {f:?}"
                    ),
                )
            })?;
        }
        if let Some(stack) = &self.stack {
            spec.stack = StackSpec::preset(stack);
        }
        spec.compact = self.compact.as_ref().map(|p| p.display().to_string());
        Ok(spec)
    }

    /// The device on its stack and its design, checked.
    fn build(&self) -> Result<(Spec, Box<dyn Device>, Design)> {
        let spec = self.spec()?;
        let (device, design) = spec.build()?;
        Ok((spec, device, design))
    }
}

/// Every device of the library ([`crate::device::library`]), in its order: the strip waveguide,
/// bend, directional coupler, all-pass and add-drop rings, rib waveguide, 1 × 2 and 2 × 2 MMIs,
/// Y-branch, and the MZIs of directional couplers and of Y-branches, each with its parameters,
/// its models and its ports.
///
/// # Errors
///
/// None is expected: a device's geometry at its defaults, which gives its ports.
pub fn devices() -> Result<Vec<DeviceInfo>> {
    device::library()
        .iter()
        .map(|d| {
            let ports = d
                .geometry(&device::Values::defaults(d.as_ref()))?
                .ports
                .into_iter()
                .map(|p| p.name)
                .collect();
            Ok(DeviceInfo {
                kind: d.kind().to_owned(),
                about: d.about().to_owned(),
                parameters: d
                    .parameters()
                    .iter()
                    .map(|p| DeviceParameter {
                        name: p.name.clone(),
                        unit: p.unit.clone(),
                        default: p.default,
                        min: p.min,
                        max: p.max,
                    })
                    .collect(),
                fidelities: d.fidelities().iter().map(model).collect(),
                ports,
            })
        })
        .collect()
}

fn model(m: &device::Method) -> DeviceModel {
    DeviceModel {
        fidelity: m.fidelity.to_string(),
        name: m.name.clone(),
        write_up: m.write_up.clone(),
        papers: m.papers.clone(),
        error: m.error.as_ref().map(|s| StatedError {
            quantity: s.quantity.clone(),
            bound: s.bound,
            unit: s.unit.clone(),
            against: s.against.to_string(),
        }),
        validity: m
            .validity
            .iter()
            .map(|r| ValidityRange {
                name: r.name.clone(),
                min: r.min,
                max: r.max,
            })
            .collect(),
    }
}

/// A stack of the catalogue, by the name a [`DeviceDesign`] takes: its layers of the materials
/// catalogue's, bottom to top, between a substrate and a cladding.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StackInfo {
    /// Its name, what [`DeviceDesign::stack`] takes, e.g. `"soi_220"`.
    pub name: String,
    /// What it is, in a sentence or two.
    pub about: String,
    /// The material below the layers: the catalogue's id ([`super::materials`]), e.g. `"si"`.
    pub substrate: String,
    /// The layers, bottom to top.
    pub layers: Vec<StackLayer>,
    /// The material above them, the catalogue's id.
    pub cladding: String,
    /// The layer a device draws its guides on, e.g. `"Si"`: its thickness is the device's
    /// `thickness` parameter.
    pub core: String,
}

/// A layer of a [`StackInfo`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StackLayer {
    /// Its name, e.g. `"BOX"`.
    pub name: String,
    /// Its thickness, µm; the core's is the stack's own until a device sets it to its
    /// `thickness`.
    pub thickness_um: f64,
    /// What the shapes drawn on it are made of, the catalogue's id, e.g. `"si"`.
    pub material: String,
    /// What fills it elsewhere, the catalogue's id.
    pub background: String,
}

/// What a stack of the catalogue is.
fn stack_about(name: &str) -> &'static str {
    match name {
        "soi_220" => {
            "220 nm of silicon on 2 µm of buried oxide, clad in oxide: the standard silicon \
             photonics platform (L. Chrostowski, M. Hochberg, Silicon Photonics Design, \
             doi:10.1017/CBO9781316084168)."
        }
        "soi" => {
            "Silicon on 2 µm of buried oxide, clad in oxide: silicon-on-insulator with the \
             silicon as thick as the device's thickness."
        }
        "nitride" => {
            "Silicon nitride on 2 µm of oxide over a silicon substrate, clad in oxide: the \
             nitride as thick as the device's thickness."
        }
        _ => "",
    }
}

/// The stacks of the catalogue ([`crate::device::PRESETS`]): `"soi_220"`, `"soi"` and
/// `"nitride"`, each by the name a [`DeviceDesign`] takes, with its layers of the materials
/// catalogue's (Li's silicon, Malitson's silica, Luke's nitride), which are the stack the
/// devices are drawn on, to the bit.
pub fn stacks() -> Vec<StackInfo> {
    crate::device::PRESETS
        .iter()
        .filter_map(|&name| match StackSpec::preset(name).layers() {
            StackSpec::Layers {
                substrate,
                layers,
                cladding,
                core,
            } => Some(StackInfo {
                name: name.to_owned(),
                about: stack_about(name).to_owned(),
                substrate: substrate.id,
                layers: layers
                    .into_iter()
                    .map(|l| StackLayer {
                        name: l.name,
                        thickness_um: l.thickness_um,
                        material: l.material.id,
                        background: l.background.id,
                    })
                    .collect(),
                cladding: cladding.id,
                core,
            }),
            StackSpec::Preset { .. } => None,
        })
        .collect()
}

/// A device's S-parameters over wavelength, with the model they come from.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DeviceSpectrum {
    /// The device's ports' names, in the matrices' order.
    pub ports: Vec<String>,
    /// The vacuum wavelengths, µm.
    pub wavelength_um: Vec<f64>,
    /// The S-matrices, `(nλ, n, n)`, S_qp from port p into port q, in photonoxide's e^(−iωt)
    /// convention.
    pub s: Vec<c64>,
    /// The fidelity of the model: `"analytic"`, `"2D"`, `"3D"`, ...
    pub fidelity: String,
    /// The grid its figures name, or `"exact: no grid"` for a closed form; a fit names the grid
    /// of the solves it was fitted to.
    pub grid: String,
    /// What the model reproduces: a paper with its DOI and equation, or a solver with its grid.
    pub source: String,
    /// The wavelengths it holds for, µm, if limited.
    pub validity_um: Option<(f64, f64)>,
}

/// The S-parameters of `design` at `wavelength_um`: its component at its fidelity
/// ([`crate::device::Device::component`]), solved once (a mode solve, or 2D or 3D FDFD, at a
/// solver-backed fidelity: seconds to minutes) and evaluated at each wavelength. The numbers are
/// those of a component's page in the studio for the same design, to the bit. A 2D result is
/// an estimate, not the device's performance.
///
/// # Errors
///
/// [`Error::InvalidValue`] for an unknown kind, parameter, fidelity or stack, a value outside
/// its parameter's range, a fidelity the device hasn't, values outside an analytic fit's tables,
/// or a wavelength that isn't positive and finite; the solver's errors at a solver-backed
/// fidelity.
pub fn device_spectrum(design: &DeviceDesign, wavelength_um: &[f64]) -> Result<DeviceSpectrum> {
    let wavelengths = wavelengths(wavelength_um)?;
    let (spec, device, d) = design.build()?;
    let built = device.component(d.values(), spec.fidelity)?;
    spectrum_of(&built, &wavelengths)
}

/// [`device_spectrum`], stopped by `stop`: its deadline, or a request (which a binding makes on
/// Ctrl+C). `None` when it stopped the call. The solve runs on a thread of its own and the call
/// waits for it while watching `stop`; a stop ends the wait at once, and the solve already
/// started is let go, its result discarded, since the solvers take no stop of their own (a job's
/// `"spectrum"` task stops so too). The numbers are [`device_spectrum`]'s, to the bit.
///
/// # Errors
///
/// As [`device_spectrum`].
pub fn device_spectrum_until(
    design: &DeviceDesign,
    wavelength_um: &[f64],
    stop: &Stop,
) -> Result<Option<DeviceSpectrum>> {
    let wavelengths = wavelengths(wavelength_um)?;
    let (spec, device, d) = design.build()?;
    let Some(built) = device::component_until(device, d.values(), spec.fidelity, stop)? else {
        return Ok(None);
    };
    spectrum_of(&built, &wavelengths).map(Some)
}

fn wavelengths(wavelength_um: &[f64]) -> Result<Vec<Wavelength>> {
    wavelength_um.iter().map(|&w| Wavelength::um(w)).collect()
}

/// A built component's S-parameters at `wavelengths`, with its model's provenance.
fn spectrum_of(built: &device::Built, wavelengths: &[Wavelength]) -> Result<DeviceSpectrum> {
    let s = crate::circuit::Spectrum::of(built.component.as_ref(), wavelengths, &built.values)?;
    let p = built.component.provenance();
    let Spectrum {
        ports,
        wavelength_um,
        s,
    } = Spectrum::of(&s);
    Ok(DeviceSpectrum {
        ports,
        wavelength_um,
        s,
        fidelity: p.fidelity.to_string(),
        grid: built
            .figures
            .first()
            .map(|f| f.grid.clone())
            .unwrap_or_default(),
        source: p.source,
        validity_um: p.validity,
    })
}

/// One of a device's figures: a number its model gives, with its fidelity and grid.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DeviceFigure {
    /// What it is, e.g. `"effective index"`.
    pub name: String,
    /// Its symbol, unique among a device's figures, e.g. `"n_eff"`.
    pub symbol: String,
    /// Its value, in `unit`.
    pub value: f64,
    /// Its unit, e.g. `"µm"`, or `""` for a pure number.
    pub unit: String,
    /// The fidelity of the model that gives it.
    pub fidelity: String,
    /// The grid it was computed on, or `"exact: no grid"` for a closed form.
    pub grid: String,
}

/// The figures of `design` at its fidelity ([`crate::device::Device::derived`]): n_eff and n_g,
/// the cross-over length, the free spectral range, Q, the bend's loss, each with its fidelity
/// and grid, as a component's page shows them.
///
/// # Errors
///
/// As [`device_spectrum`].
pub fn device_derived(design: &DeviceDesign) -> Result<Vec<DeviceFigure>> {
    let (spec, device, d) = design.build()?;
    Ok(device
        .derived(d.values(), spec.fidelity)?
        .into_iter()
        .map(|f| DeviceFigure {
            name: f.name,
            symbol: f.symbol,
            value: f.value,
            unit: f.unit,
            fidelity: f.fidelity.to_string(),
            grid: f.grid,
        })
        .collect())
}

/// A device's geometry: its stack, the polygons drawn on each layer, and its ports.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DeviceGeometry {
    /// The stack's layers, bottom to top, each with the polygons drawn on it.
    pub layers: Vec<GeometryLayer>,
    /// The material below the layers.
    pub substrate: String,
    /// The material above them.
    pub cladding: String,
    /// The ports, in the order of the device's S-matrices.
    pub ports: Vec<DevicePort>,
}

/// A layer of a device's stack, and what is drawn on it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GeometryLayer {
    /// Its name, e.g. `"Si"`.
    pub name: String,
    /// Its bottom and top, µm.
    pub z_um: (f64, f64),
    /// What the polygons are made of, e.g. `"Si"`.
    pub material: String,
    /// What fills the layer elsewhere.
    pub background: String,
    /// The polygons drawn on it, each shape's in the order drawn.
    pub polygons: Vec<DevicePolygon>,
}

/// A polygon with holes, seen from above, and how its edges were etched.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DevicePolygon {
    /// Its outer boundary, (x, y) in µm, counterclockwise, the first vertex not repeated.
    pub outer: Vec<(f64, f64)>,
    /// Its holes, each clockwise.
    pub holes: Vec<Vec<(f64, f64)>>,
    /// How deep the etch around it went from the layer's top, µm: `None` through the layer (a
    /// strip), less than the layer's thickness for a rib, whose layer is left below it.
    pub etch_depth_um: Option<f64>,
    /// Its walls' angle from the layer's plane, degrees: 90 vertical, less for walls that lean
    /// out going down (the polygon is the top face).
    pub sidewall_deg: f64,
}

/// A device's port: where it crosses the device's edge, the direction light leaves by, the
/// guide's width there, and its layer.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DevicePort {
    /// Its name, e.g. `"o1"`.
    pub name: String,
    /// Its centre seen from above, µm.
    pub position_um: (f64, f64),
    /// The direction light leaves the device by, degrees counterclockwise from +x.
    pub direction_deg: f64,
    /// The guide's width there, µm.
    pub width_um: f64,
    /// The layer the guide is drawn on.
    pub layer: String,
}

fn xy(p: &Point) -> (f64, f64) {
    (p.x.to_um(), p.y.to_um())
}

/// The geometry of `design` ([`crate::device::Device::geometry`]): its stack at its values, the
/// shapes drawn on each layer as polygons with holes, every curve within 1 nm and every vertex
/// on a 1 nm grid (GDSII's usual database unit), and its ports. Its fidelity is not used.
///
/// # Errors
///
/// [`Error::InvalidValue`] for an unknown kind, parameter or stack, or a value outside its
/// parameter's range.
pub fn device_geometry(design: &DeviceDesign) -> Result<DeviceGeometry> {
    let spec = design.spec()?;
    let device = spec.device()?;
    let definitions: Vec<(&str, &str)> = spec
        .parameters
        .iter()
        .map(|(n, e)| (n.as_str(), e.as_str()))
        .collect();
    let d = Design::new(device.as_ref(), &definitions)?;
    let g = device.geometry(d.values())?;
    let s = &g.structure;
    let stack = s.stack();
    let layers = stack
        .layers()
        .iter()
        .map(|layer| {
            let z = stack
                .layer(&layer.name)
                .map_or((0.0, 0.0), |(_, bottom, top)| (bottom.to_um(), top.to_um()));
            let polygons = s
                .shapes(&layer.name)
                .iter()
                .zip(s.etches(&layer.name))
                .flat_map(|(shape, etch)| {
                    shape
                        .polygons(Tolerance::default())
                        .contours
                        .into_iter()
                        .map(move |c| DevicePolygon {
                            outer: c.outer.iter().map(xy).collect(),
                            holes: c.holes.iter().map(|h| h.iter().map(xy).collect()).collect(),
                            etch_depth_um: etch.depth.map(|d| d.to_um()),
                            sidewall_deg: if etch.sidewall == std::f64::consts::FRAC_PI_2 {
                                90.0
                            } else {
                                etch.sidewall.to_degrees()
                            },
                        })
                })
                .collect();
            GeometryLayer {
                name: layer.name.clone(),
                z_um: z,
                material: layer.material.name().to_owned(),
                background: layer.background.name().to_owned(),
                polygons,
            }
        })
        .collect();
    Ok(DeviceGeometry {
        layers,
        substrate: stack.substrate().name().to_owned(),
        cladding: stack.cladding().name().to_owned(),
        ports: g
            .ports
            .iter()
            .map(|p| DevicePort {
                name: p.name.clone(),
                position_um: xy(&p.position),
                direction_deg: p.direction,
                width_um: p.width.to_um(),
                layer: p.layer.clone(),
            })
            .collect(),
    })
}

/// The job file that computes `design`, named `name`, as text in `format` (`"toml"`, `"json"`
/// or `"yaml"`): a `[device]` table holding the design and a `"spectrum"` task over the
/// device's own band at its fidelity ([`crate::job::from_device`]), as a component's Simulate
/// tab writes it. [`super::run_job`] runs it; its run records what [`device_spectrum`] gives,
/// to the bit.
///
/// # Errors
///
/// [`Error::InvalidValue`] for an unknown format, a name that isn't a job's, and the errors of
/// [`device_spectrum`]'s design and of [`super::check_job`] for the job it makes (a 3D solve
/// larger than the direct solver holds here).
pub fn device_job(design: &DeviceDesign, name: &str, format: &str) -> Result<String> {
    let format = match format.to_ascii_lowercase().as_str() {
        "toml" => Format::Toml,
        "json" => Format::Json,
        "yaml" | "yml" => Format::Yaml,
        _ => {
            return Err(Error::invalid(
                "job format",
                format!("is \"toml\", \"json\" or \"yaml\", not {format:?}"),
            ));
        }
    };
    let spec = design.spec()?;
    crate::job::from_device(name, &spec, &crate::job::Compute::model())?.to_text(format)
}
