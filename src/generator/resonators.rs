//! Resonators with their buses: a ring or a racetrack, and a disk, all-pass or add-drop.

use std::f64::consts::PI;

use super::waveguides::strip;
use super::{Drawing, Generator, Port, positive, refuse, um};
use crate::Result;
use crate::geometry::{Point, Shape};
use crate::units::Length;

/// A ring resonator, or a racetrack, with its bus along y = 0 and, for an add-drop ring, a
/// second bus above it.
///
/// The bus is a straight `width` wide along y = 0 from x = −`reach` to `reach`. The ring's
/// guide, `width` wide too, has its centre line of `radius` about (0, w + gap + R), so `gap`
/// lies between the bus's top edge and the ring's foot; a racetrack's two half rings are
/// `coupling_length` apart, joined by straights at its top and foot. The drop bus lies
/// `drop_gap` above the ring's top. Ports `in` (−reach, 0) facing 180° and `through` (reach, 0)
/// facing 0°, and on the drop bus `add` (reach, y_d) facing 0° and `drop` (−reach, y_d) facing
/// 180°, y_d = 2(w + R) + gap + drop_gap: the names of the circuit's rings
/// ([`crate::circuit::components::AllPassRing`], [`crate::circuit::components::AddDropRing`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Ring {
    /// The guides' width.
    pub width: Length,
    /// The radius of the ring's centre line (a racetrack's half rings').
    pub radius: Length,
    /// The gap between the bus and the ring, edge to edge.
    pub gap: Length,
    /// The straights between a racetrack's half rings (zero: a ring).
    pub coupling_length: Length,
    /// The gap between the ring and the drop bus; `None`: all-pass, no drop bus.
    pub drop_gap: Option<Length>,
    /// The buses' half length; `None`: coupling_length/2 + radius + width + 1 µm.
    pub reach: Option<Length>,
}

impl Ring {
    /// An all-pass ring of `radius`, `gap` from its bus.
    pub fn new(width: Length, radius: Length, gap: Length) -> Ring {
        Ring {
            width,
            radius,
            gap,
            coupling_length: Length::ZERO,
            drop_gap: None,
            reach: None,
        }
    }

    /// An all-pass racetrack: half rings of `radius` with straights `coupling_length` long.
    pub fn racetrack(width: Length, radius: Length, gap: Length, coupling_length: Length) -> Ring {
        Ring {
            coupling_length,
            ..Ring::new(width, radius, gap)
        }
    }

    /// The same ring with a drop bus `drop_gap` above it.
    #[must_use]
    pub fn with_drop(mut self, drop_gap: Length) -> Ring {
        self.drop_gap = Some(drop_gap);
        self
    }

    /// The same ring with buses reaching `reach` either side of its centre.
    #[must_use]
    pub fn with_reach(mut self, reach: Length) -> Ring {
        self.reach = Some(reach);
        self
    }
}

impl Generator for Ring {
    fn kind(&self) -> &str {
        if self.coupling_length.to_um() > 0.0 {
            "racetrack"
        } else {
            "ring"
        }
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let w = positive(k, "width", self.width, false)?;
        let r = positive(k, "radius", self.radius, false)?;
        let gap = positive(k, "gap", self.gap, false)?;
        let lc = positive(k, "coupling_length", self.coupling_length, true)?;
        if w >= 2.0 * r {
            return Err(refuse(
                k,
                format!("its width {w} um must be less than twice its radius {r} um"),
            ));
        }
        // the input bus along y = 0, the ring's lower straight (or point) w + gap above it
        let low = w + gap;
        let top = low + 2.0 * r;
        let centre = low + r;
        let reach = match self.reach {
            Some(l) => positive(k, "reach", l, false)?,
            None => lc / 2.0 + r + w + 1.0,
        };
        let mut d = Drawing::new(k);
        d.add(layer, strip(w, 0.0, -reach, reach)?);
        if lc > 0.0 {
            d.add(
                layer,
                Shape::sector(
                    Point::um(-lc / 2.0, centre),
                    um(r - w / 2.0),
                    um(r + w / 2.0),
                    PI / 2.0,
                    PI,
                )?,
            );
            d.add(
                layer,
                Shape::sector(
                    Point::um(lc / 2.0, centre),
                    um(r - w / 2.0),
                    um(r + w / 2.0),
                    -PI / 2.0,
                    PI,
                )?,
            );
            for y in [low, top] {
                d.add(layer, strip(w, y, -lc / 2.0, lc / 2.0)?);
            }
        } else {
            d.add(layer, Shape::ring(Point::um(0.0, centre), um(r), um(w))?);
        }
        let port = |name: &str, x: f64, y: f64, direction: f64| {
            Port::new(name, Point::um(x, y), direction, um(w), layer)
        };
        d.add_port(port("in", -reach, 0.0, 180.0))?;
        d.add_port(port("through", reach, 0.0, 0.0))?;
        if let Some(g) = self.drop_gap {
            let drop = top + w + positive(k, "drop_gap", g, false)?;
            d.add(layer, strip(w, drop, -reach, reach)?);
            d.add_port(port("add", reach, drop, 0.0))?;
            d.add_port(port("drop", -reach, drop, 180.0))?;
        }
        Ok(d)
    }
}

/// A disk resonator with its bus along y = 0 and, for an add-drop disk, a second bus above it.
///
/// The disk's edge has `radius` about (0, w/2 + gap + R), so `gap` lies between the bus's top
/// edge and the disk; the drop bus lies `drop_gap` above its top. Its buses and ports are a
/// [`Ring`]'s: `in`, `through`, and `add`, `drop` at y_d = w + 2R + gap + drop_gap.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Disk {
    /// The buses' width.
    pub width: Length,
    /// The disk's radius, to its edge.
    pub radius: Length,
    /// The gap between the bus and the disk, edge to edge.
    pub gap: Length,
    /// The gap between the disk and the drop bus; `None`: all-pass.
    pub drop_gap: Option<Length>,
    /// The buses' half length; `None`: radius + width + 1 µm.
    pub reach: Option<Length>,
}

impl Disk {
    /// An all-pass disk of `radius`, `gap` from its bus `width` wide.
    pub fn new(width: Length, radius: Length, gap: Length) -> Disk {
        Disk {
            width,
            radius,
            gap,
            drop_gap: None,
            reach: None,
        }
    }

    /// The same disk with a drop bus `drop_gap` above it.
    #[must_use]
    pub fn with_drop(mut self, drop_gap: Length) -> Disk {
        self.drop_gap = Some(drop_gap);
        self
    }

    /// The same disk with buses reaching `reach` either side of its centre.
    #[must_use]
    pub fn with_reach(mut self, reach: Length) -> Disk {
        self.reach = Some(reach);
        self
    }
}

impl Generator for Disk {
    fn kind(&self) -> &str {
        "disk"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let w = positive(k, "width", self.width, false)?;
        let r = positive(k, "radius", self.radius, false)?;
        let gap = positive(k, "gap", self.gap, false)?;
        let reach = match self.reach {
            Some(l) => positive(k, "reach", l, false)?,
            None => r + w + 1.0,
        };
        let centre = w / 2.0 + gap + r;
        let mut d = Drawing::new(k);
        d.add(layer, strip(w, 0.0, -reach, reach)?);
        d.add(layer, Shape::circle(Point::um(0.0, centre), um(r))?);
        let port = |name: &str, x: f64, y: f64, direction: f64| {
            Port::new(name, Point::um(x, y), direction, um(w), layer)
        };
        d.add_port(port("in", -reach, 0.0, 180.0))?;
        d.add_port(port("through", reach, 0.0, 0.0))?;
        if let Some(g) = self.drop_gap {
            let drop = centre + r + positive(k, "drop_gap", g, false)? + w / 2.0;
            d.add(layer, strip(w, drop, -reach, reach)?);
            d.add_port(port("add", reach, drop, 0.0))?;
            d.add_port(port("drop", -reach, drop, 180.0))?;
        }
        Ok(d)
    }
}
