//! Devices: a component that knows its geometry.
//!
//! A [`Device`] joins four things: a layer stack, the geometry's parameters (a width, a gap, a
//! radius, a thickness), the models that compute it at each [`Fidelity`], and the
//! [`Component`] those models give. A circuit's components take their physical inputs once,
//! when they are built (a directional coupler's supermodes, a bend's radius, a ring's guide); a
//! device builds them from its parameters, so its geometry can be shown, changed, swept, run
//! and optimized.
//!
//! - [`Device::parameters`]: each with its name, unit, default and range. Their values are a
//!   [`Values`]; a [`Design`] gives them as expressions of named parameters
//!   ([`crate::expr::Parameters`]), `gap = "w/2.5"`, with their derivatives.
//! - [`Device::stack`] and [`Device::geometry`]: the layers, and the shapes drawn on them with
//!   the ports (position, direction, width, layer).
//! - [`Device::fidelities`]: the models it has, each a [`Method`]: what computes it, its
//!   write-up and papers, its stated error against the next fidelity up, and where it holds.
//! - [`Device::component`]: the component a fidelity gives at these values, built by the
//!   circuit's own builders ([`crate::circuit::components`]), with the values of its parameters
//!   and their derivatives ([`Built`]).
//! - [`Device::derived`]: its cheap figures, each with its fidelity and grid: n_eff and n_g, the
//!   cross-over length, the free spectral range, Q, the bend's loss.
//! - [`Device::objectives`] and [`Device::constraints`]: what an optimization of it makes
//!   small, large or near a target, and keeps within bounds, each an expression of its outputs
//!   with derivatives where they exist ([`objective`]).
//!
//! - [`compact`]: a compact model fitted to the device's own runs at a 2D or 3D fidelity (or its
//!   closed form) over some of its parameters ([`FitPlan`], [`Fitted`]), which gives the design
//!   one more fidelity, [`Fidelity::Compact`], at the source's accuracy, with exact derivatives
//!   in the parameters it covers.
//!
//! The first devices are a silicon strip's: the [`StripWaveguide`], the [`StripBend`], the
//! [`StripCoupler`] (a directional coupler) and the [`StripRing`] (all-pass and add-drop).
//! [`library`] has them at their defaults, which are the studio's library's components bit
//! for bit. The first active devices are the thermo-optic phase shifters ([`ThermoShifter`]): a
//! metal heater over a strip, doped heaters beside a rib and the guide heated itself (Harris et
//! al.'s in-guide heater), their phase following the heater's power at analytic, 2D and
//! measured fidelity, the measured one a measured device's file ([`Measurement`]).
//!
//! **Fidelities and grids.** Every figure names its model and its grid: an analytic figure
//! comes from a closed form, or a fit to the full-vector solver whose grid it states; a 2D
//! figure (a bent slab by the effective index method) says it is 2D, and is never a device's
//! performance; a 3D figure states the solver's grid. A solver-backed fidelity solves on its
//! device's [`Mesh`].
//!
//! **Saved as a job.** A [`Spec`] is a device's design as a job file's `[device]` table holds
//! it: the device's kind, its stack (a preset or layers of the materials catalogue's), its
//! parameters' expressions and its fidelity. [`crate::job::check`] checks it and
//! [`crate::job::preview`] draws it.
//!
//! docs/methods/devices.md describes the abstraction and each device's models, with their
//! papers and the validation of each.

mod active;
mod bend;
pub mod compact;
mod coupler;
mod fit;
mod guide;
mod mmi;
mod mzi;
pub mod objective;
mod rib;
mod ring;
mod section;
mod solve;
pub(crate) mod spec;
mod thermo;
mod thermo_closed;
mod waveguide;
mod ybranch;

pub(crate) mod checks;
pub(crate) mod splitter_checks;
#[cfg(test)]
mod splitter_tests;
#[cfg(test)]
mod tests;
pub(crate) mod thermo_checks;
#[cfg(test)]
mod thermo_tests;

pub use active::Measurement;
pub use bend::StripBend;
pub use compact::{FitAxis, FitPlan, Fitted, Sampling};
pub use coupler::StripCoupler;
pub use mmi::{Model2d, StripMmi};
pub use mzi::StripMzi;
pub use objective::{Constraint, Goal, Limit, Objective, Reduce};
pub use rib::RibWaveguide;
pub use ring::StripRing;
pub use section::{Cells, Mesh, Scheme, Symmetry};
pub use solve::FdfdGrid;
pub use spec::{LayerSpec, MaterialSpec, PRESETS, Spec, StackSpec, fidelity};
pub use thermo::ThermoShifter;
pub use waveguide::StripWaveguide;
pub use ybranch::StripYBranch;

use std::fmt;

use crate::circuit::{Component, Fidelity, Parameter, SMatrix};
use crate::expr::{self, Dim};
use crate::geometry::Point;
use crate::stack::{LayerStack, Structure};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// A component that knows its geometry: a stack, the parameters of the shapes drawn on it, the
/// models that compute it at each fidelity, and the component each gives.
///
/// Implementations list their parameters once (the slice doesn't change), and refuse values
/// that aren't theirs ([`Values::check`]).
pub trait Device: fmt::Debug + Send + Sync {
    /// What kind of device it is, e.g. `"waveguide"`: the kind of the component it gives.
    fn kind(&self) -> &str;

    /// What it is, in a sentence or two.
    fn about(&self) -> &str;

    /// Its parameters, in the order a [`Values`] lists them: name, unit, default and range.
    fn parameters(&self) -> &[Parameter];

    /// The layers at `values` (a thickness among them).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for values that aren't this device's.
    fn stack(&self, values: &Values) -> Result<LayerStack>;

    /// The shapes on the stack at `values`, with the ports.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for values that aren't this device's, or that make no shape.
    fn geometry(&self, values: &Values) -> Result<Geometry>;

    /// The models it has, cheapest first.
    fn fidelities(&self) -> &[Method];

    /// The component `fidelity` gives at `values`, with its parameters' values and figures.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for values that aren't this device's, a fidelity it doesn't
    /// have, or values outside the model's range (an analytic fit's); the solver's errors at a
    /// solver-backed fidelity.
    fn component(&self, values: &Values, fidelity: Fidelity) -> Result<Built>;

    /// Its cheap figures at `values` from `fidelity`'s model, each with its fidelity and grid.
    ///
    /// # Errors
    ///
    /// As [`Device::component`].
    fn derived(&self, values: &Values, fidelity: Fidelity) -> Result<Vec<Figure>> {
        Ok(self.component(values, fidelity)?.figures)
    }

    /// The objectives an optimization of it offers, each an expression of its outputs
    /// ([`objective`]) to make small, large or near a target: a coupler's split ratio and its
    /// flatness over a band, a ring's extinction and critical coupling. The library's devices
    /// declare theirs by kind; another device, none.
    fn objectives(&self) -> Vec<Objective> {
        objective::objectives_of(self.kind())
    }

    /// The constraints an optimization of it offers, each an expression of its outputs kept at
    /// most or at least a bound: a smallest gap, a footprint, a free spectral range. The
    /// library's devices declare theirs by kind; another device, none.
    fn constraints(&self) -> Vec<Constraint> {
        objective::constraints_of(self.kind())
    }
}

/// A device's component at a fidelity and its values: the component, the values of its own
/// parameters that the device's give, how those follow the device's parameters, and the
/// device's figures from the same model.
#[derive(Debug)]
pub struct Built {
    /// The component, as the circuit's builders make it.
    pub component: Box<dyn Component>,
    /// The values of its parameters ([`Component::parameters`]) at the device's values: a
    /// waveguide's length and loss, a ring's round trip and coupling.
    pub values: Vec<f64>,
    /// For each device parameter, in order, the derivatives of [`Built::values`] with respect
    /// to it, exact; `None` for a parameter the component is built from (a width, a gap), whose
    /// derivative needs another component.
    pub derivatives: Vec<Option<Vec<f64>>>,
    /// The figures of [`Device::derived`].
    pub figures: Vec<Figure>,
}

impl Built {
    /// The component's S-matrix at `wavelength` and the device's values.
    ///
    /// # Errors
    ///
    /// The component's.
    pub fn s_matrix(&self, wavelength: Wavelength) -> Result<SMatrix> {
        self.component.s_matrix(wavelength, &self.values)
    }

    /// The figure named `symbol`, if the model gives it.
    pub fn figure(&self, symbol: &str) -> Option<&Figure> {
        self.figures.iter().find(|f| f.symbol == symbol)
    }
}

/// One of a device's figures: a number its model gives cheaply, with the model and the grid
/// it comes from.
#[derive(Clone, Debug, PartialEq)]
pub struct Figure {
    /// What it is, e.g. `"effective index"`.
    pub name: String,
    /// Its symbol, unique among a device's figures, e.g. `"n_eff"`.
    pub symbol: String,
    /// Its value, in `unit`.
    pub value: f64,
    /// Its unit, e.g. `"µm"`, or `""` for a pure number.
    pub unit: String,
    /// The model that gives it.
    pub fidelity: Fidelity,
    /// The grid it was computed on, or `"exact: no grid"` for a closed form; an analytic fit
    /// names the grid of the solves it was fitted to.
    pub grid: String,
}

/// One of a device's models: its fidelity, what computes it, where it is written up, its stated
/// error against the next fidelity up, and where it holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Method {
    /// The fidelity it is.
    pub fidelity: Fidelity,
    /// What computes it, in a sentence.
    pub name: String,
    /// Its write-up in docs/methods, e.g. `"devices"` (docs/methods/devices.md).
    pub write_up: String,
    /// The papers it implements, each with its DOI.
    pub papers: Vec<String>,
    /// Its error against the next fidelity up over [`Method::validity`], measured by the
    /// validation report; `None` for the highest.
    pub error: Option<Stated>,
    /// Where it holds: a range for each parameter it limits, and `"wavelength"`.
    pub validity: Vec<Range>,
}

/// A model's stated error: the largest difference in one quantity against another fidelity.
#[derive(Clone, Debug, PartialEq)]
pub struct Stated {
    /// The quantity, e.g. `"n_eff"`.
    pub quantity: String,
    /// The largest difference, in `unit`.
    pub bound: f64,
    /// Its unit: `""` for a pure number, `"%"` for a relative one.
    pub unit: String,
    /// The fidelity it is measured against.
    pub against: Fidelity,
}

/// A range a model holds over, for one parameter or the wavelength.
#[derive(Clone, Debug, PartialEq)]
pub struct Range {
    /// The parameter's name, or `"wavelength"`.
    pub name: String,
    /// The smallest value.
    pub min: f64,
    /// The largest value.
    pub max: f64,
}

impl Range {
    fn new(name: &str, min: f64, max: f64) -> Range {
        Range {
            name: name.into(),
            min,
            max,
        }
    }
}

/// The shapes a device draws on its stack, and its ports.
#[derive(Clone, Debug, PartialEq)]
pub struct Geometry {
    /// The stack and the shapes on its layers.
    pub structure: Structure,
    /// The ports, in the order of its component's ports.
    pub ports: Vec<Port>,
}

/// Where a port crosses a device's edge: its position, the direction light leaves by, the
/// guide's width there, and its layer.
#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    /// The name of the component's port, e.g. `"o1"`.
    pub name: String,
    /// Its centre, seen from above.
    pub position: Point,
    /// The direction light leaves the device by, degrees counterclockwise from +x.
    pub direction: f64,
    /// The guide's width there.
    pub width: Length,
    /// The layer the guide is drawn on.
    pub layer: String,
}

/// A device's parameters' values, in the order of [`Device::parameters`] and in their units.
#[derive(Clone, Debug, PartialEq)]
pub struct Values {
    names: Vec<String>,
    values: Vec<f64>,
}

impl Values {
    /// Every parameter of `device` at its default.
    pub fn defaults(device: &dyn Device) -> Values {
        Values {
            names: device.parameters().iter().map(|p| p.name.clone()).collect(),
            values: device.parameters().iter().map(|p| p.default).collect(),
        }
    }

    /// `device`'s defaults with `changes`, (name, value) pairs, in the parameters' units.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a name that isn't one of its parameters or a value outside
    /// the parameter's range.
    pub fn new(device: &dyn Device, changes: &[(&str, f64)]) -> Result<Values> {
        let mut values = Values::defaults(device);
        for &(name, value) in changes {
            values = values.with(device, name, value)?;
        }
        Ok(values)
    }

    /// The same values with `name` at `value`.
    ///
    /// # Errors
    ///
    /// As [`Values::new`].
    pub fn with(&self, device: &dyn Device, name: &str, value: f64) -> Result<Values> {
        let k = position(device, name)?;
        let p = &device.parameters()[k];
        if !p.allows(value) {
            return Err(Error::invalid(
                "device",
                format!(
                    "a {}'s {name} must be from {} to {} {}, got {value}",
                    device.kind(),
                    p.min,
                    p.max,
                    p.unit
                ),
            ));
        }
        let mut v = self.clone();
        v.values[k] = value;
        Ok(v)
    }

    /// The value of the parameter `name`.
    pub fn get(&self, name: &str) -> Option<f64> {
        self.names
            .iter()
            .position(|n| n == name)
            .map(|k| self.values[k])
    }

    /// The values, in the parameters' order.
    pub fn as_slice(&self) -> &[f64] {
        &self.values
    }

    /// Refuses values that aren't `device`'s: other names, or a value outside its range.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] then.
    pub fn check(&self, device: &dyn Device) -> Result<()> {
        let parameters = device.parameters();
        if self.names.len() != parameters.len()
            || self.names.iter().zip(parameters).any(|(n, p)| *n != p.name)
        {
            return Err(Error::invalid(
                "device",
                format!(
                    "these values are for the parameters {}, not a {}'s",
                    self.names.join(", "),
                    device.kind()
                ),
            ));
        }
        for (p, &v) in parameters.iter().zip(&self.values) {
            if !p.allows(v) {
                return Err(Error::invalid(
                    "device",
                    format!(
                        "a {}'s {} must be from {} to {} {}, got {v}",
                        device.kind(),
                        p.name,
                        p.min,
                        p.max,
                        p.unit
                    ),
                ));
            }
        }
        Ok(())
    }

    /// The value of the parameter `name`, which the device has.
    fn at(&self, name: &str) -> f64 {
        self.get(name)
            .unwrap_or_else(|| unreachable!("a device asks only for its own parameters: {name}"))
    }
}

/// The position of `name` among `device`'s parameters.
fn position(device: &dyn Device, name: &str) -> Result<usize> {
    device
        .parameters()
        .iter()
        .position(|p| p.name == name)
        .ok_or_else(|| {
            Error::invalid(
                "device",
                format!("a {} has no parameter {name}", device.kind()),
            )
        })
}

/// A device's parameters given as expressions ([`crate::expr`]): each of its parameters, and
/// any others named to define them, e.g. `width = "450 nm"`, `w = "0.45"`, `gap = "w/2.5"`.
/// Parameters left out take their defaults.
///
/// An expression's value is in its parameter's unit: `0.45` for a width is 0.45 µm, as is
/// `450 nm` (lengths are kept in µm); an angle parameter's `90` is 90°, as is `pi/2 rad`
/// (angles in radians are turned into degrees). A length can't be given to an angle or a
/// number, nor an angle to a length.
#[derive(Clone, Debug, PartialEq)]
pub struct Design {
    parameters: Vec<Parameter>,
    kind: String,
    expressions: expr::Parameters,
    values: Values,
}

impl Design {
    /// `device`'s parameters from `definitions`, (name, expression) pairs.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an expression that doesn't parse or whose units don't fit
    /// its parameter, a cycle, or a value outside a parameter's range.
    pub fn new(device: &dyn Device, definitions: &[(&str, &str)]) -> Result<Design> {
        let mut all: Vec<(String, String)> = definitions
            .iter()
            .map(|&(n, e)| (n.to_owned(), e.to_owned()))
            .collect();
        for p in device.parameters() {
            if !all.iter().any(|(n, _)| *n == p.name) {
                // the default as a plain number: in the parameter's unit, read back exactly
                all.push((p.name.clone(), format!("{:?}", p.default)));
            }
        }
        let expressions =
            expr::Parameters::new(&all).map_err(|e| Error::invalid("device", format!("{e}")))?;
        let parameters = device.parameters().to_vec();
        let kind = device.kind().to_owned();
        let values = resolve(&parameters, &kind, &expressions)?;
        Ok(Design {
            parameters,
            kind,
            expressions,
            values,
        })
    }

    /// The device's values.
    pub fn values(&self) -> &Values {
        &self.values
    }

    /// The named parameters, the device's and the others.
    pub fn expressions(&self) -> &expr::Parameters {
        &self.expressions
    }

    /// The same design with the named parameter `name` set to `value` (in its unit as its
    /// expression gives it: µm for a length, radians for an angle in degrees or radians, the
    /// number otherwise) and those that use it evaluated again.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an unknown name, or values out of range.
    pub fn with(&self, name: &str, value: f64) -> Result<Design> {
        let expressions = self
            .expressions
            .with(name, value)
            .map_err(|e| Error::invalid("device", format!("{e}")))?;
        let values = resolve(&self.parameters, &self.kind, &expressions)?;
        Ok(Design {
            expressions,
            values,
            ..self.clone()
        })
    }

    /// The derivatives of the device's values with respect to the named parameter `name`, in
    /// the order of its parameters, by forward mode through the expressions
    /// ([`crate::expr::Parameters::derivatives`]); each in its parameter's unit per unit of
    /// `name` as [`Design::with`] takes it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an unknown name.
    pub fn sensitivities(&self, name: &str) -> Result<Vec<f64>> {
        let duals = self
            .expressions
            .derivatives(name)
            .map_err(|e| Error::invalid("device", format!("{e}")))?;
        Ok(self
            .parameters
            .iter()
            .map(|p| {
                let slot = self
                    .expressions
                    .slot(&p.name)
                    .unwrap_or_else(|| unreachable!("every device parameter is defined"));
                duals[slot].derivative * unit_factor(p, self.expressions.dim(&p.name))
            })
            .collect())
    }
}

/// The factor taking an expression's value in its dimension to `p`'s unit: radians to degrees
/// for an angle given to a parameter in degrees, 1 otherwise.
fn unit_factor(p: &Parameter, dim: Option<Option<Dim>>) -> f64 {
    match dim.flatten() {
        Some(d) if d.angle != 0 && p.unit == "°" => 180.0 / std::f64::consts::PI,
        _ => 1.0,
    }
}

/// The device's values from evaluated expressions, each checked against its unit and range.
fn resolve(parameters: &[Parameter], kind: &str, expressions: &expr::Parameters) -> Result<Values> {
    let mut values = Vec::with_capacity(parameters.len());
    for p in parameters {
        let value = expressions
            .value(&p.name)
            .unwrap_or_else(|| unreachable!("every device parameter is defined"));
        let dim = expressions.dim(&p.name).flatten();
        let fits = match dim {
            None => true,
            Some(d) if d == Dim::NONE => true,
            Some(d) if d == Dim::LENGTH => p.unit == "µm",
            Some(d) if d == Dim::ANGLE => p.unit == "°",
            Some(_) => false,
        };
        if !fits {
            return Err(Error::invalid(
                "device",
                format!(
                    "a {kind}'s {} is in {}, but its expression is {}",
                    p.name,
                    if p.unit.is_empty() {
                        "no unit"
                    } else {
                        &p.unit
                    },
                    dim.map_or_else(String::new, Dim::name)
                ),
            ));
        }
        let value = value * unit_factor(p, Some(dim));
        if !p.allows(value) {
            return Err(Error::invalid(
                "device",
                format!(
                    "a {kind}'s {} must be from {} to {} {}, got {value}",
                    p.name, p.min, p.max, p.unit
                ),
            ));
        }
        values.push(value);
    }
    Ok(Values {
        names: parameters.iter().map(|p| p.name.clone()).collect(),
        values,
    })
}

/// The device's S-matrix at `wavelength`: its component at `fidelity` and `values`.
///
/// # Errors
///
/// As [`Device::component`] and [`Component::s_matrix`].
pub fn s_matrix(
    device: &dyn Device,
    values: &Values,
    fidelity: Fidelity,
    wavelength: Wavelength,
) -> Result<SMatrix> {
    device.component(values, fidelity)?.s_matrix(wavelength)
}

/// dS/dθ at `wavelength` for the named parameter θ of `design` (a device parameter, or one the
/// expressions name), exact: the component's derivatives ([`Component::derivatives`]) through
/// [`Built::derivatives`] and the expressions' ([`Design::sensitivities`]). `None` when θ moves
/// a parameter the component is built from (a width, a gap: a new component, whose derivative
/// is a finite difference), or the component has no exact derivatives there.
///
/// # Errors
///
/// As [`Device::component`] and [`Component::derivatives`], and [`Error::InvalidValue`] for an
/// unknown name.
pub fn derivative(
    device: &dyn Device,
    design: &Design,
    fidelity: Fidelity,
    wavelength: Wavelength,
    name: &str,
) -> Result<Option<SMatrix>> {
    let along = design.sensitivities(name)?;
    let built = device.component(design.values(), fidelity)?;
    let Some(ds) = built.component.derivatives(wavelength, &built.values)? else {
        return Ok(None);
    };
    let n = built.component.ports().len();
    let mut total = SMatrix::zeros(n);
    for (k, &dp) in along.iter().enumerate() {
        if dp == 0.0 {
            continue;
        }
        let Some(dtheta) = &built.derivatives[k] else {
            return Ok(None);
        };
        for (j, &dt) in dtheta.iter().enumerate() {
            if dt == 0.0 {
                continue;
            }
            for q in 0..n {
                for p in 0..n {
                    total[(q, p)] += ds[j][(q, p)] * (dt * dp);
                }
            }
        }
    }
    Ok(Some(total))
}

/// The devices at their defaults, on 220 nm SOI: the strip waveguide, bend, directional
/// coupler, the all-pass and add-drop rings, the rib waveguide, the 1 × 2 and 2 × 2 MMIs, the
/// Y-branch, the MZIs of directional couplers and of Y-branches, and the thermo-optic phase
/// shifters of a metal heater, of doped heaters and of the guide heated itself.
pub fn library() -> Vec<Box<dyn Device>> {
    vec![
        Box::new(StripWaveguide::new()),
        Box::new(StripBend::new()),
        Box::new(StripCoupler::new()),
        Box::new(StripRing::all_pass()),
        Box::new(StripRing::add_drop()),
        Box::new(RibWaveguide::new()),
        Box::new(StripMmi::one_by_two()),
        Box::new(StripMmi::two_by_two()),
        Box::new(StripYBranch::new()),
        Box::new(StripMzi::couplers()),
        Box::new(StripMzi::y_branches()),
        Box::new(ThermoShifter::metal()),
        Box::new(ThermoShifter::doped()),
        Box::new(ThermoShifter::in_guide()),
    ]
}

/// The device's component at `values` and `fidelity` ([`Device::component`]), built on a thread
/// of its own so that `stop` ends the wait at once: the solve already started is let go, its
/// result discarded, as the solvers take no stop of their own. `None` when the stop came first.
/// The component is the one [`Device::component`] gives, to the bit.
///
/// # Errors
///
/// [`Device::component`]'s, and [`Error::InvalidValue`] if the thread can't be started.
pub(crate) fn component_until(
    device: Box<dyn Device>,
    values: &Values,
    fidelity: Fidelity,
    stop: &crate::run::Stop,
) -> Result<Option<Built>> {
    use std::sync::mpsc;
    use std::time::Duration;
    if stop.reason().is_some() {
        return Ok(None);
    }
    let values = values.clone();
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("photonoxide device solve".into())
        .spawn(move || {
            let _ = tx.send(device.component(&values, fidelity));
        })
        .map_err(|e| Error::invalid("device", format!("can't start the solve: {e}")))?;
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(result) => return result.map(Some),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if stop.reason().is_some() {
                    return Ok(None);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(Error::invalid("device", "the solve ended without a result"));
            }
        }
    }
}

/// A device's geometry: its generator's `drawing` ([`crate::generator`]) drawn on `structure`,
/// and its ports.
///
/// # Errors
///
/// As [`crate::generator::Drawing::draw_on`].
fn drawn(mut structure: Structure, drawing: &crate::generator::Drawing) -> Result<Geometry> {
    drawing.draw_on(&mut structure)?;
    let ports = drawing
        .ports()
        .iter()
        .map(|p| Port {
            name: p.name.clone(),
            position: p.position,
            direction: p.direction,
            width: p.width,
            layer: p.layer.clone(),
        })
        .collect();
    Ok(Geometry { structure, ports })
}

/// `drawing` with every shape cut by `etch` instead of its own: a generator draws strips, and
/// a device whose core is etched part of the way (a rib) or with sloped walls cuts them so.
///
/// # Errors
///
/// [`Error::InvalidValue`] for ports a drawing can't hold (never: they were its own).
fn etched(
    drawing: &crate::generator::Drawing,
    etch: crate::stack::Etch,
) -> Result<crate::generator::Drawing> {
    let mut d = crate::generator::Drawing::new(drawing.kind());
    for item in drawing.items() {
        d.add_etched(&item.layer, item.shape.clone(), etch);
    }
    for p in drawing.ports() {
        d.add_port(p.clone())?;
    }
    Ok(d)
}

/// The [`Method`] for the fidelity `fidelity` among `methods`.
fn method(kind: &str, methods: &[Method], fidelity: Fidelity) -> Result<Method> {
    methods
        .iter()
        .find(|m| m.fidelity == fidelity)
        .cloned()
        .ok_or_else(|| {
            let have: Vec<String> = methods.iter().map(|m| m.fidelity.to_string()).collect();
            Error::invalid(
                "device",
                format!(
                    "a {kind} has no {fidelity} model; it has {}",
                    have.join(" and ")
                ),
            )
        })
}

/// A figure.
fn figure(
    name: &str,
    symbol: &str,
    value: f64,
    unit: &str,
    fidelity: Fidelity,
    grid: &str,
) -> Figure {
    Figure {
        name: name.into(),
        symbol: symbol.into(),
        value,
        unit: unit.into(),
        fidelity,
        grid: grid.into(),
    }
}

/// The grid a closed form states.
const EXACT: &str = "exact: no grid";

/// A parameter.
fn parameter(name: &str, unit: &str, default: f64, min: f64, max: f64) -> Parameter {
    Parameter::new(name, unit, default, min, max)
}
