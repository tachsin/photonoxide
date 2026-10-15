//! Generators of photonic components: plain functions of a component's parameters that draw its
//! shapes on layers and name its ports.
//!
//! A generator is a parameter struct ([`Straight`], [`Ring`], [`Mmi`], …) whose
//! [`Generator::draw`] gives a [`Drawing`]: the shapes, each on a layer (exact shapes of
//! [`crate::geometry`]: sectors, paths, regions between two curves, polygons), and the
//! [`Port`]s, each with its name, position, the direction light leaves by, the guide's width
//! and its layer. A drawing can be moved, turned and mirrored ([`Drawing::transformed`]), put
//! with one of its ports on another's ([`Drawing::connected`]), and drawn on a
//! [`Structure`]. Its ports are found by name: a job's FDFD port or FDTD source placed at
//! `ring.through`, a circuit's `instance.port`.
//!
//! The first set:
//!
//! - waveguides: [`Straight`]; [`Taper`] (linear, exponential, parabolic, or a width w(x) of
//!   one's own); [`Bend`] (an arc, or a partial Euler bend); [`SBend`] (cosine, sine, Euler or
//!   Bézier); [`Spiral`];
//! - resonators with their buses: [`Ring`] (a ring or a racetrack, all-pass or add-drop) and
//!   [`Disk`];
//! - splitters and couplers: [`DirectionalCoupler`] (straight, with S-bend arms if asked),
//!   [`BentCoupler`], [`Mmi`] (N × M), [`YBranch`] (Bézier arms on a linear or custom stem),
//!   [`Crossing`].
//!
//! Each component is drawn in its own frame, its first port at or left of the origin. Lengths
//! are [`Length`]s; angles are degrees, as ports' directions are (and as the devices take them),
//! counterclockwise from +x; a port's direction is in [0, 360). A port's position, direction and
//! width come from closed forms, not from the drawn outline, so they are exact.
//!
//! The devices draw their geometry with these generators: [`crate::device::StripWaveguide`]
//! with [`Straight`], [`crate::device::StripBend`] with [`Bend`],
//! [`crate::device::StripCoupler`] with [`DirectionalCoupler`] and
//! [`crate::device::StripRing`] with [`Ring`], each to the bit as it drew them before.
//!
//! See docs/methods/generators.md for each generator's definition and its validation.

use std::f64::consts::FRAC_PI_2;
use std::fmt;

use crate::geometry::{Bounds, Point, Region, Shape, Transform};
use crate::stack::{Etch, LayerStack, Structure};
use crate::units::Length;
use crate::{Error, Result};

mod couplers;
mod resonators;
mod waveguides;

pub(crate) mod checks;
#[cfg(test)]
mod tests;

pub use couplers::{BentCoupler, Crossing, DirectionalCoupler, Mmi, SelfImaging, YBranch};
pub use resonators::{Disk, Ring};
pub use waveguides::{Bend, SBend, SBendKind, Spiral, Straight, Taper, TaperProfile};

/// A component's generator: its parameters, and the drawing they make.
pub trait Generator: fmt::Debug + Send + Sync {
    /// What it draws, e.g. `"ring"`.
    fn kind(&self) -> &str;

    /// Its shapes on `layer` and its ports.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for parameters that make no such component (a width that isn't
    /// positive, a gap that makes guides overlap), naming the parameter.
    fn draw(&self, layer: &str) -> Result<Drawing>;
}

/// Where a guide crosses a component's edge: its name, its centre, the direction light leaves
/// by, the guide's width and its layer, and how many modes it carries where that is known.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Port {
    /// The name, unique in its component and without dots (a job and a netlist name it as
    /// `component.port`).
    pub name: String,
    /// The centre of the guide's end.
    pub position: Point,
    /// The direction light leaves by, degrees counterclockwise from +x, in [0, 360).
    pub direction: f64,
    /// The guide's width there.
    pub width: Length,
    /// The layer the guide is drawn on.
    pub layer: String,
    /// How many modes the guide carries there, where that is known: a generator draws on a
    /// layer without knowing the stack, so it leaves this `None`; whoever knows the stack (a
    /// device, a mode solve) sets it with [`Port::with_modes`].
    pub modes: Option<usize>,
}

impl Port {
    /// The port `name` at `position` facing `direction` (degrees, taken into [0, 360)).
    pub fn new(name: &str, position: Point, direction: f64, width: Length, layer: &str) -> Port {
        Port {
            name: name.to_owned(),
            position,
            direction: degrees(direction),
            width,
            layer: layer.to_owned(),
            modes: None,
        }
    }

    /// The same port, carrying `modes` modes.
    #[must_use]
    pub fn with_modes(mut self, modes: usize) -> Port {
        self.modes = Some(modes);
        self
    }

    /// The unit vector light leaves by.
    pub fn outward(&self) -> [f64; 2] {
        let (s, c) = sin_cos_deg(self.direction);
        [c, s]
    }

    /// The port's image under `t`: moved, its direction turned (and reflected by a mirror), its
    /// width scaled. Exact for quarter turns and mirrors in x or y.
    pub fn transformed(&self, t: &Transform) -> Port {
        let (s, c) = sin_cos_deg(self.direction);
        Port {
            position: t.apply(self.position),
            direction: angle_of(t.linear([c, s])),
            width: self.width * t.factor(),
            ..self.clone()
        }
    }

    /// Whether `other` meets this port: at the same place, facing it, as wide, on the same
    /// layer, within `tolerance` of position and width and 1e-9 degrees of direction.
    pub fn meets(&self, other: &Port, tolerance: Length) -> bool {
        let tol = tolerance.to_um();
        let d = (self.position.x - other.position.x)
            .to_um()
            .hypot((self.position.y - other.position.y).to_um());
        let turn = degrees(other.direction - self.direction - 180.0);
        d <= tol
            && (self.width - other.width).to_um().abs() <= tol
            && (turn.min(360.0 - turn)) <= 1e-9
            && self.layer == other.layer
    }
}

/// `d` degrees taken into [0, 360), without a negative zero.
pub(crate) fn degrees(d: f64) -> f64 {
    let r = d.rem_euclid(360.0) + 0.0;
    if r >= 360.0 { 0.0 } else { r }
}

/// sin and cos of `d` degrees, exact at whole quarter turns.
pub(crate) fn sin_cos_deg(d: f64) -> (f64, f64) {
    let q = d / 90.0;
    if q.fract() == 0.0 && q.abs() < 1e15 {
        match (q as i64).rem_euclid(4) {
            0 => (0.0, 1.0),
            1 => (1.0, 0.0),
            2 => (0.0, -1.0),
            _ => (-1.0, 0.0),
        }
    } else {
        d.to_radians().sin_cos()
    }
}

/// The direction of `v`, degrees in [0, 360), exact along the axes.
pub(crate) fn angle_of(v: [f64; 2]) -> f64 {
    match (v[0], v[1]) {
        (x, y) if y == 0.0 && x > 0.0 => 0.0,
        (x, y) if x == 0.0 && y > 0.0 => 90.0,
        (x, y) if y == 0.0 && x < 0.0 => 180.0,
        (x, y) if x == 0.0 && y < 0.0 => 270.0,
        (x, y) => degrees(y.atan2(x).to_degrees()),
    }
}

/// A rotation by `d` degrees about the origin, exact at whole quarter turns.
pub(crate) fn rotation(d: f64) -> Transform {
    let q = d / 90.0;
    if q.fract() == 0.0 && q.abs() < 1e15 {
        Transform::rotate(
            (q as i64).rem_euclid(4) as f64 * FRAC_PI_2,
            Point::default(),
        )
    } else {
        Transform::rotate(d.to_radians(), Point::default())
    }
}

/// A shape on a layer, cut by an etch: one piece of a [`Drawing`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Item {
    /// The layer it is drawn on.
    pub layer: String,
    /// The shape.
    pub shape: Shape,
    /// How its edges are etched ([`Etch::FULL`] for a strip).
    pub etch: Etch,
}

/// A component drawn: its kind, its shapes on their layers, and its ports.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawing {
    kind: String,
    items: Vec<Item>,
    ports: Vec<Port>,
}

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("component", reason)
}

impl Drawing {
    /// An empty drawing of a component of `kind`.
    pub fn new(kind: &str) -> Drawing {
        Drawing {
            kind: kind.to_owned(),
            items: Vec::new(),
            ports: Vec::new(),
        }
    }

    /// Adds `shape` on `layer`, etched through it.
    pub fn add(&mut self, layer: &str, shape: Shape) -> &mut Drawing {
        self.add_etched(layer, shape, Etch::FULL)
    }

    /// Adds `shape` on `layer`, cut by `etch`.
    pub fn add_etched(&mut self, layer: &str, shape: Shape, etch: Etch) -> &mut Drawing {
        self.items.push(Item {
            layer: layer.to_owned(),
            shape,
            etch,
        });
        self
    }

    /// Adds `port`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if its name is empty, has a dot, or is another port's.
    pub fn add_port(&mut self, port: Port) -> Result<&mut Drawing> {
        if port.name.is_empty() || port.name.contains('.') {
            return Err(invalid(format!(
                "a port's name needs to be a word without dots, got \"{}\"",
                port.name
            )));
        }
        if self.ports.iter().any(|p| p.name == port.name) {
            return Err(invalid(format!(
                "a {} has two ports named {}",
                self.kind, port.name
            )));
        }
        self.ports.push(port);
        Ok(self)
    }

    /// What it is, e.g. `"ring"`.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Its shapes, in the order they are drawn.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Its ports, in their order.
    pub fn ports(&self) -> &[Port] {
        &self.ports
    }

    /// The port named `name`.
    pub fn port(&self, name: &str) -> Option<&Port> {
        self.ports.iter().find(|p| p.name == name)
    }

    /// The port named `name`, or an error naming the ports there are.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if there is none.
    pub fn port_named(&self, name: &str) -> Result<&Port> {
        self.port(name).ok_or_else(|| {
            let names: Vec<&str> = self.ports.iter().map(|p| p.name.as_str()).collect();
            invalid(format!(
                "a {} has no port {name}: its ports are {}",
                self.kind,
                names.join(", ")
            ))
        })
    }

    /// The drawing moved by `t`: every shape and every port.
    pub fn transformed(&self, t: &Transform) -> Drawing {
        Drawing {
            kind: self.kind.clone(),
            items: self
                .items
                .iter()
                .map(|i| Item {
                    shape: i.shape.transformed(t),
                    ..i.clone()
                })
                .collect(),
            ports: self.ports.iter().map(|p| p.transformed(t)).collect(),
        }
    }

    /// The drawing turned (and, with `mirrored`, first reflected in the x axis) and moved so
    /// that its port `port` meets `to`: on it, facing it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if it has no port `port`.
    pub fn connected(&self, port: &str, to: &Port, mirrored: bool) -> Result<Drawing> {
        let mut t = if mirrored {
            Transform::mirror(Point::default(), 0.0)
        } else {
            Transform::IDENTITY
        };
        let own = self.port_named(port)?.transformed(&t);
        t = t.then(&rotation(to.direction + 180.0 - own.direction));
        let p = t.apply(self.port_named(port)?.position);
        t = t.then(&Transform::translate(
            to.position.x - p.x,
            to.position.y - p.y,
        ));
        Ok(self.transformed(&t))
    }

    /// Draws its shapes on `s`, after what is there.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the stack has no layer the drawing uses, or an etch doesn't
    /// fit its layer.
    pub fn draw_on(&self, s: &mut Structure) -> Result<()> {
        for i in &self.items {
            s.draw_etched(&i.layer, i.shape.clone(), i.etch)?;
        }
        Ok(())
    }

    /// The drawing on `stack`.
    ///
    /// # Errors
    ///
    /// As [`Drawing::draw_on`].
    pub fn structure(&self, stack: LayerStack) -> Result<Structure> {
        let mut s = Structure::new(stack);
        self.draw_on(&mut s)?;
        Ok(s)
    }

    /// The box around its shapes and ports; `None` for a drawing of nothing.
    pub fn bounds(&self) -> Option<Bounds> {
        let mut points: Vec<Point> = Vec::new();
        for i in &self.items {
            let b = i.shape.bounds();
            points.extend([b.min, b.max]);
        }
        points.extend(self.ports.iter().map(|p| p.position));
        let first = points.first()?;
        let mut b = Bounds {
            min: *first,
            max: *first,
        };
        let lo = |a: Length, b: Length| um(a.to_um().min(b.to_um()));
        let hi = |a: Length, b: Length| um(a.to_um().max(b.to_um()));
        for p in &points {
            b.min = Point {
                x: lo(b.min.x, p.x),
                y: lo(b.min.y, p.y),
            };
            b.max = Point {
                x: hi(b.max.x, p.x),
                y: hi(b.max.y, p.y),
            };
        }
        Some(b)
    }

    /// Whether `p` is inside a shape on `layer`.
    pub fn contains(&self, layer: &str, p: Point) -> bool {
        self.items
            .iter()
            .any(|i| i.layer == layer && i.shape.contains(p))
    }

    /// The signed distance from `p` to the nearest shape on `layer`'s boundary, negative inside
    /// one: the least of the shapes' distances (exact where they don't overlap). `None` when
    /// nothing is drawn on `layer`.
    pub fn distance(&self, layer: &str, p: Point) -> Option<Length> {
        self.items
            .iter()
            .filter(|i| i.layer == layer)
            .map(|i| i.shape.distance(p).signed.to_um())
            .reduce(f64::min)
            .map(Length::um)
    }

    /// Its ports as a circuit's: the same names, in the same order, their modes unstated.
    pub fn circuit_ports(&self) -> Vec<crate::circuit::Port> {
        self.ports
            .iter()
            .map(|p| crate::circuit::Port::new(p.name.clone()))
            .collect()
    }
}

/// Refuses a length that isn't finite and positive (or, with `zero`, at least zero), naming
/// the component and the parameter.
pub(crate) fn positive(kind: &str, name: &str, l: Length, zero: bool) -> Result<f64> {
    let v = l.to_um();
    if v.is_finite() && (v > 0.0 || (zero && v == 0.0)) {
        Ok(v)
    } else {
        Err(invalid(format!(
            "a {kind}'s {name} must be {}, got {l}",
            if zero { "zero or positive" } else { "positive" }
        )))
    }
}

/// Refuses a number that isn't finite, naming the component and the parameter.
pub(crate) fn finite(kind: &str, name: &str, v: f64) -> Result<f64> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(invalid(format!(
            "a {kind}'s {name} must be finite, got {v}"
        )))
    }
}

/// An error naming the component.
pub(crate) fn refuse(kind: &str, reason: impl fmt::Display) -> Error {
    invalid(format!("a {kind}: {reason}"))
}

/// The length in µm.
pub(crate) fn um(v: f64) -> Length {
    Length::um(v)
}
