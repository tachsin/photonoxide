//! Planar shapes: what is drawn on a layer, seen from above.
//!
//! Coordinates are [`Length`]s in the plane of the chip: x along the chip, y across it. Shapes
//! are validated when they are made (finite coordinates, a non-zero area), so a solver never
//! meets a degenerate one.
//!
//! Every shape is a [`Region`]: it says whether a point is inside, its signed distance and
//! outward normal, its bounds, area and perimeter, its area inside a cell ([`Region::fill`]),
//! and polygons within a [`Tolerance`]. A shape keeps its exact definition (an arc stays an
//! arc) and becomes polygons only when asked. A [`Transform`] (translation, rotation, mirror,
//! scaling) maps a shape to one of its own kind, exactly. An [`Index`] over a layer's shapes
//! finds those near a point or a cell. See docs/methods/geometry.md.

use std::f64::consts::TAU;

use crate::units::Length;
use crate::{Error, Result};

mod boundary;
pub(crate) mod checks;
mod index;
mod primitives;
mod region;
#[cfg(test)]
mod region_tests;
mod transform;

pub use index::Index;
pub use primitives::{Ellipse, FILL_TOLERANCE, RegularPolygon, RoundedRect, Sector, Superellipse};
pub use region::{Contour, Distance, Location, Polygons, Region, Tolerance};
pub use transform::Transform;

use boundary::{Conic, Piece};
use primitives::um;

/// A point in the plane of the chip.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// Along the chip.
    pub x: Length,
    /// Across the chip.
    pub y: Length,
}

impl Point {
    /// The point (x, y), in micrometres.
    pub const fn um(x: f64, y: f64) -> Point {
        Point {
            x: Length::um(x),
            y: Length::um(y),
        }
    }

    fn is_finite(self) -> bool {
        self.x.to_um().is_finite() && self.y.to_um().is_finite()
    }
}

/// An axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    /// The corner with the smallest x and y.
    pub min: Point,
    /// The corner with the largest x and y.
    pub max: Point,
}

impl Bounds {
    /// Whether the box contains `p`, edges included.
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
}

/// A simple polygon: its vertices in counterclockwise order, without the first repeated at the
/// end. Build it with [`Polygon::new`].
#[derive(Clone, Debug, PartialEq)]
pub struct Polygon {
    vertices: Vec<Point>,
}

impl Polygon {
    /// A polygon through `vertices`, in either orientation; it is stored counterclockwise.
    /// A closing vertex equal to the first is dropped.
    ///
    /// Self-intersection isn't checked here; layout checks it with the design rules.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] with fewer than three distinct vertices, a coordinate that isn't
    /// finite, two consecutive equal vertices, or a zero area.
    pub fn new(mut vertices: Vec<Point>) -> Result<Polygon> {
        if vertices.len() > 1 && vertices.first() == vertices.last() {
            vertices.pop();
        }
        if vertices.len() < 3 {
            return Err(Error::invalid(
                "polygon",
                format!("needs at least 3 vertices, got {}", vertices.len()),
            ));
        }
        if let Some(p) = vertices.iter().find(|p| !p.is_finite()) {
            return Err(Error::invalid(
                "polygon",
                format!("vertex ({}, {}) isn't finite", p.x, p.y),
            ));
        }
        let n = vertices.len();
        if (0..n).any(|i| vertices[i] == vertices[(i + 1) % n]) {
            return Err(Error::invalid(
                "polygon",
                "has two consecutive equal vertices",
            ));
        }
        let area = signed_area(&vertices);
        if area == 0.0 {
            return Err(Error::invalid("polygon", "has zero area"));
        }
        if area < 0.0 {
            vertices.reverse();
        }
        Ok(Polygon { vertices })
    }

    /// The vertices, counterclockwise.
    pub fn vertices(&self) -> &[Point] {
        &self.vertices
    }

    /// Vertices known to be counterclockwise, as a transform of a polygon leaves them.
    pub(crate) fn from_ccw(vertices: Vec<Point>) -> Polygon {
        Polygon { vertices }
    }

    /// The boundary: one loop of segments, counterclockwise.
    fn loops(&self) -> Vec<Vec<Piece>> {
        let v = &self.vertices;
        vec![
            (0..v.len())
                .map(|i| Piece::Line(um(v[i]), um(v[(i + 1) % v.len()])))
                .collect(),
        ]
    }

    /// The area, in µm².
    pub fn area(&self) -> f64 {
        signed_area(&self.vertices)
    }

    /// A ring of waveguide `width` whose centre line has `radius`: the annulus from
    /// radius − width/2 to radius + width/2, as ring resonators are specified.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless everything is finite and 0 < width < 2 radius.
    pub fn ring(center: Point, radius: Length, width: Length) -> Result<Shape> {
        let (r, w) = (radius.to_um(), width.to_um());
        if !center.is_finite() || !r.is_finite() || !w.is_finite() || w <= 0.0 || w >= 2.0 * r {
            return Err(Error::invalid(
                "ring",
                format!(
                    "needs a finite centre and 0 < width < 2 radius, got radius {radius} and width {width}"
                ),
            ));
        }
        Ok(Shape::Ring {
            center,
            inner: radius - width / 2.0,
            outer: radius + width / 2.0,
        })
    }

    /// The bounding box.
    pub fn bounds(&self) -> Bounds {
        bounds_of(&self.vertices)
    }

    /// Whether the polygon contains `p` (even-odd rule; a point exactly on an edge may count
    /// either way).
    pub fn contains(&self, p: Point) -> bool {
        let (px, py) = (p.x.to_um(), p.y.to_um());
        let v = &self.vertices;
        let mut inside = false;
        let mut j = v.len() - 1;
        for i in 0..v.len() {
            let (xi, yi) = (v[i].x.to_um(), v[i].y.to_um());
            let (xj, yj) = (v[j].x.to_um(), v[j].y.to_um());
            if (yi > py) != (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
                inside = !inside;
            }
            j = i;
        }
        inside
    }

    /// The polygon moved by (dx, dy).
    pub fn translated(&self, dx: Length, dy: Length) -> Polygon {
        Polygon {
            vertices: self
                .vertices
                .iter()
                .map(|p| Point {
                    x: p.x + dx,
                    y: p.y + dy,
                })
                .collect(),
        }
    }
}

/// The shoelace formula: positive for counterclockwise vertices, in µm².
fn signed_area(v: &[Point]) -> f64 {
    let n = v.len();
    (0..n)
        .map(|i| {
            let (a, b) = (v[i], v[(i + 1) % n]);
            a.x.to_um() * b.y.to_um() - b.x.to_um() * a.y.to_um()
        })
        .sum::<f64>()
        / 2.0
}

/// The bounds of points given in µm.
pub(crate) fn bounds_um(v: &[[f64; 2]]) -> Bounds {
    let points: Vec<Point> = v.iter().map(|p| Point::um(p[0], p[1])).collect();
    bounds_of(&points)
}

fn bounds_of(v: &[Point]) -> Bounds {
    let fold = |f: fn(f64, f64) -> f64, get: fn(&Point) -> f64, start: f64| {
        Length::um(v.iter().map(get).fold(start, f))
    };
    Bounds {
        min: Point {
            x: fold(f64::min, |p| p.x.to_um(), f64::INFINITY),
            y: fold(f64::min, |p| p.y.to_um(), f64::INFINITY),
        },
        max: Point {
            x: fold(f64::max, |p| p.x.to_um(), f64::NEG_INFINITY),
            y: fold(f64::max, |p| p.y.to_um(), f64::NEG_INFINITY),
        },
    }
}

/// A shape drawn on a layer.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Shape {
    /// An axis-aligned rectangle.
    Rect {
        /// Its centre.
        center: Point,
        /// Its width (along x).
        width: Length,
        /// Its height (along y).
        height: Length,
    },
    /// A disk.
    Circle {
        /// Its centre.
        center: Point,
        /// Its radius.
        radius: Length,
    },
    /// A polygon.
    Polygon(Polygon),
    /// An annulus: the region between two circles about one centre, as a ring resonator's
    /// waveguide.
    Ring {
        /// Its centre.
        center: Point,
        /// The inner radius.
        inner: Length,
        /// The outer radius.
        outer: Length,
    },
    /// An ellipse at any angle.
    Ellipse(Ellipse),
    /// An annular sector; with no inner radius, a pie slice.
    Sector(Sector),
    /// A regular polygon.
    RegularPolygon(RegularPolygon),
    /// A superellipse, |x/a|ⁿ + |y/b|ⁿ ≤ 1 in its own frame.
    Superellipse(Superellipse),
    /// A rectangle at any angle, its corners rounded (a radius of zero: sharp).
    RoundedRect(RoundedRect),
}

impl Shape {
    /// An ellipse ([`Ellipse::new`]).
    ///
    /// # Errors
    ///
    /// As [`Ellipse::new`].
    pub fn ellipse(center: Point, semi_axes: [Length; 2], angle: f64) -> Result<Shape> {
        Ellipse::new(center, semi_axes, angle).map(Shape::Ellipse)
    }

    /// An annular sector ([`Sector::new`]).
    ///
    /// # Errors
    ///
    /// As [`Sector::new`].
    pub fn sector(
        center: Point,
        inner: Length,
        outer: Length,
        start: f64,
        sweep: f64,
    ) -> Result<Shape> {
        Sector::new(center, inner, outer, start, sweep).map(Shape::Sector)
    }

    /// A regular polygon ([`RegularPolygon::new`]).
    ///
    /// # Errors
    ///
    /// As [`RegularPolygon::new`].
    pub fn regular_polygon(
        center: Point,
        radius: Length,
        sides: usize,
        angle: f64,
    ) -> Result<Shape> {
        RegularPolygon::new(center, radius, sides, angle).map(Shape::RegularPolygon)
    }

    /// A superellipse ([`Superellipse::new`]).
    ///
    /// # Errors
    ///
    /// As [`Superellipse::new`].
    pub fn superellipse(
        center: Point,
        semi_axes: [Length; 2],
        exponent: f64,
        angle: f64,
    ) -> Result<Shape> {
        Superellipse::new(center, semi_axes, exponent, angle).map(Shape::Superellipse)
    }

    /// A rectangle with rounded corners at an angle ([`RoundedRect::new`]).
    ///
    /// # Errors
    ///
    /// As [`RoundedRect::new`].
    pub fn rounded_rect(
        center: Point,
        width: Length,
        height: Length,
        radius: Length,
        angle: f64,
    ) -> Result<Shape> {
        RoundedRect::new(center, width, height, radius, angle).map(Shape::RoundedRect)
    }

    /// The primitives that answer as regions themselves; `None` for the rectangle, circle,
    /// polygon and ring, which answer here.
    fn primitive(&self) -> Option<&dyn Region> {
        match self {
            Shape::Ellipse(s) => Some(s),
            Shape::Sector(s) => Some(s),
            Shape::RegularPolygon(s) => Some(s),
            Shape::Superellipse(s) => Some(s),
            Shape::RoundedRect(s) => Some(s),
            Shape::Rect { .. } | Shape::Circle { .. } | Shape::Polygon(_) | Shape::Ring { .. } => {
                None
            }
        }
    }

    /// The boundary of a rectangle, circle, polygon or ring: loops of pieces, the region on
    /// their left.
    fn loops(&self) -> Vec<Vec<Piece>> {
        let circle = |center: &Point, r: Length, ccw: bool| {
            let (t0, t1) = if ccw { (0.0, TAU) } else { (TAU, 0.0) };
            vec![Piece::Conic(Conic {
                c: um(*center),
                rx: r.to_um(),
                ry: r.to_um(),
                cos: 1.0,
                sin: 0.0,
                t0,
                t1,
            })]
        };
        match self {
            Shape::Rect { .. } => {
                let b = self.bounds();
                let ([x0, y0], [x1, y1]) = (um(b.min), um(b.max));
                vec![vec![
                    Piece::Line([x0, y0], [x1, y0]),
                    Piece::Line([x1, y0], [x1, y1]),
                    Piece::Line([x1, y1], [x0, y1]),
                    Piece::Line([x0, y1], [x0, y0]),
                ]]
            }
            Shape::Circle { center, radius } => vec![circle(center, *radius, true)],
            Shape::Ring {
                center,
                inner,
                outer,
            } => vec![circle(center, *outer, true), circle(center, *inner, false)],
            Shape::Polygon(p) => p.loops(),
            _ => Vec::new(),
        }
    }
    /// An axis-aligned rectangle.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the width and height are positive and everything is
    /// finite.
    pub fn rect(center: Point, width: Length, height: Length) -> Result<Shape> {
        let ok = |l: Length| l.to_um().is_finite() && l.to_um() > 0.0;
        if !center.is_finite() || !ok(width) || !ok(height) {
            return Err(Error::invalid(
                "rectangle",
                format!("needs a finite centre and a positive size, got {width} x {height}"),
            ));
        }
        Ok(Shape::Rect {
            center,
            width,
            height,
        })
    }

    /// A disk.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the radius is positive and everything is finite.
    pub fn circle(center: Point, radius: Length) -> Result<Shape> {
        if !center.is_finite() || !radius.to_um().is_finite() || radius.to_um() <= 0.0 {
            return Err(Error::invalid(
                "circle",
                format!("needs a finite centre and a positive radius, got {radius}"),
            ));
        }
        Ok(Shape::Circle { center, radius })
    }

    /// A ring of waveguide `width` whose centre line has `radius`: the annulus from
    /// radius − width/2 to radius + width/2, as ring resonators are specified.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless everything is finite and 0 < width < 2 radius.
    pub fn ring(center: Point, radius: Length, width: Length) -> Result<Shape> {
        let (r, w) = (radius.to_um(), width.to_um());
        if !center.is_finite() || !r.is_finite() || !w.is_finite() || w <= 0.0 || w >= 2.0 * r {
            return Err(Error::invalid(
                "ring",
                format!(
                    "needs a finite centre and 0 < width < 2 radius, got radius {radius} and width {width}"
                ),
            ));
        }
        Ok(Shape::Ring {
            center,
            inner: radius - width / 2.0,
            outer: radius + width / 2.0,
        })
    }

    /// The bounding box.
    pub fn bounds(&self) -> Bounds {
        match self {
            Shape::Rect {
                center,
                width,
                height,
            } => Bounds {
                min: Point {
                    x: center.x - *width / 2.0,
                    y: center.y - *height / 2.0,
                },
                max: Point {
                    x: center.x + *width / 2.0,
                    y: center.y + *height / 2.0,
                },
            },
            Shape::Circle { center, radius }
            | Shape::Ring {
                center,
                outer: radius,
                ..
            } => Bounds {
                min: Point {
                    x: center.x - *radius,
                    y: center.y - *radius,
                },
                max: Point {
                    x: center.x + *radius,
                    y: center.y + *radius,
                },
            },
            Shape::Polygon(p) => p.bounds(),
            other => other.primitive().map(Region::bounds).unwrap_or(Bounds {
                min: Point::default(),
                max: Point::default(),
            }),
        }
    }

    /// The area, in µm².
    pub fn area(&self) -> f64 {
        match self {
            Shape::Rect { width, height, .. } => width.to_um() * height.to_um(),
            Shape::Circle { radius, .. } => TAU / 2.0 * radius.to_um() * radius.to_um(),
            Shape::Polygon(p) => p.area(),
            Shape::Ring { inner, outer, .. } => {
                TAU / 2.0 * (outer.to_um() * outer.to_um() - inner.to_um() * inner.to_um())
            }
            other => other.primitive().map_or(0.0, Region::area),
        }
    }

    /// Whether the shape contains `p` (a point exactly on the boundary may count either way).
    pub fn contains(&self, p: Point) -> bool {
        match self {
            Shape::Rect { .. } => self.bounds().contains(p),
            Shape::Circle { center, radius } => {
                let (dx, dy) = ((p.x - center.x).to_um(), (p.y - center.y).to_um());
                dx * dx + dy * dy <= radius.to_um() * radius.to_um()
            }
            Shape::Polygon(poly) => poly.contains(p),
            Shape::Ring {
                center,
                inner,
                outer,
            } => {
                let (dx, dy) = ((p.x - center.x).to_um(), (p.y - center.y).to_um());
                let d2 = dx * dx + dy * dy;
                d2 >= inner.to_um() * inner.to_um() && d2 <= outer.to_um() * outer.to_um()
            }
            other => other.primitive().is_some_and(|r| r.contains(p)),
        }
    }
}

impl Region for Polygon {
    fn locate(&self, p: Point) -> Location {
        let q = um(p);
        let v = &self.vertices;
        for i in 0..v.len() {
            let (a, b) = (um(v[i]), um(v[(i + 1) % v.len()]));
            let cross = (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0]);
            if cross == 0.0
                && q[0] >= a[0].min(b[0])
                && q[0] <= a[0].max(b[0])
                && q[1] >= a[1].min(b[1])
                && q[1] <= a[1].max(b[1])
            {
                return Location::Boundary;
            }
        }
        if Polygon::contains(self, p) {
            Location::Inside
        } else {
            Location::Outside
        }
    }

    fn contains(&self, p: Point) -> bool {
        Polygon::contains(self, p)
    }

    fn distance(&self, p: Point) -> Distance {
        Distance::from(boundary::distance(
            &self.loops(),
            um(p),
            Polygon::contains(self, p),
        ))
    }

    fn bounds(&self) -> Bounds {
        Polygon::bounds(self)
    }

    fn polygons(&self, tolerance: Tolerance) -> Polygons {
        region::assemble(
            vec![(self.vertices.iter().map(|p| um(*p)).collect(), Vec::new())],
            0.0,
            tolerance,
        )
    }

    fn area(&self) -> f64 {
        Polygon::area(self)
    }

    fn perimeter(&self) -> Option<Length> {
        Some(Length::um(boundary::length(&self.loops())))
    }

    /// Exact: the polygon clipped to the cell.
    fn fill(&self, cell: Bounds) -> f64 {
        region::fill_of(
            &self.loops(),
            Polygon::bounds(self),
            Polygon::area(self),
            cell,
        )
    }
}

impl Region for Shape {
    fn locate(&self, p: Point) -> Location {
        if let Some(r) = self.primitive() {
            return r.locate(p);
        }
        let radial = |center: &Point| {
            let (dx, dy) = ((p.x - center.x).to_um(), (p.y - center.y).to_um());
            dx * dx + dy * dy
        };
        match self {
            Shape::Rect { .. } => {
                let b = self.bounds();
                if !b.contains(p) {
                    Location::Outside
                } else if p.x == b.min.x || p.x == b.max.x || p.y == b.min.y || p.y == b.max.y {
                    Location::Boundary
                } else {
                    Location::Inside
                }
            }
            Shape::Circle { center, radius } => {
                Location::of(radial(center).total_cmp(&(radius.to_um() * radius.to_um())))
            }
            Shape::Ring {
                center,
                inner,
                outer,
            } => {
                let d2 = radial(center);
                let (i2, o2) = (inner.to_um() * inner.to_um(), outer.to_um() * outer.to_um());
                if d2 < i2 || d2 > o2 {
                    Location::Outside
                } else if d2 == i2 || d2 == o2 {
                    Location::Boundary
                } else {
                    Location::Inside
                }
            }
            Shape::Polygon(poly) => poly.locate(p),
            _ => Location::Outside,
        }
    }

    /// As [`Shape::contains`]: for a rectangle, circle, polygon or ring, a point exactly on the
    /// boundary may count either way, as it always has.
    fn contains(&self, p: Point) -> bool {
        Shape::contains(self, p)
    }

    fn distance(&self, p: Point) -> Distance {
        match self.primitive() {
            Some(r) => r.distance(p),
            None => Distance::from(boundary::distance(
                &self.loops(),
                um(p),
                Shape::contains(self, p),
            )),
        }
    }

    fn bounds(&self) -> Bounds {
        Shape::bounds(self)
    }

    fn polygons(&self, tolerance: Tolerance) -> Polygons {
        match (self.primitive(), self) {
            (Some(r), _) => r.polygons(tolerance),
            (None, Shape::Polygon(p)) => p.polygons(tolerance),
            (None, _) => region::polygons_of(&self.loops(), tolerance),
        }
    }

    fn area(&self) -> f64 {
        Shape::area(self)
    }

    fn perimeter(&self) -> Option<Length> {
        if let Some(r) = self.primitive() {
            return r.perimeter();
        }
        Some(match self {
            Shape::Rect { width, height, .. } => (*width + *height) * 2.0,
            Shape::Circle { radius, .. } => *radius * TAU,
            Shape::Ring { inner, outer, .. } => (*inner + *outer) * TAU,
            Shape::Polygon(p) => return p.perimeter(),
            _ => return None,
        })
    }

    fn fill(&self, cell: Bounds) -> f64 {
        match (self.primitive(), self) {
            (Some(r), _) => r.fill(cell),
            (None, Shape::Rect { .. }) => self.bounds().overlap(&cell),
            (None, _) => region::fill_of(&self.loops(), self.bounds(), self.area(), cell),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<Point> {
        vec![
            Point::um(0.0, 0.0),
            Point::um(2.0, 0.0),
            Point::um(2.0, 1.0),
            Point::um(0.0, 1.0),
        ]
    }

    #[test]
    fn a_polygon_is_stored_counterclockwise_with_a_positive_area() {
        let ccw = Polygon::new(square()).unwrap();
        let mut reversed = square();
        reversed.reverse();
        let cw = Polygon::new(reversed).unwrap();
        assert_eq!(ccw.area(), 2.0);
        assert_eq!(cw.area(), 2.0);
        // a closing vertex is dropped
        let mut closed = square();
        closed.push(Point::um(0.0, 0.0));
        assert_eq!(Polygon::new(closed).unwrap().vertices().len(), 4);
    }

    #[test]
    fn degenerate_polygons_are_errors() {
        assert!(Polygon::new(vec![Point::um(0.0, 0.0), Point::um(1.0, 0.0)]).is_err());
        let line = vec![
            Point::um(0.0, 0.0),
            Point::um(1.0, 0.0),
            Point::um(2.0, 0.0),
        ];
        assert!(Polygon::new(line).is_err());
        let repeated = vec![
            Point::um(0.0, 0.0),
            Point::um(1.0, 0.0),
            Point::um(1.0, 0.0),
            Point::um(0.0, 1.0),
        ];
        assert!(Polygon::new(repeated).is_err());
        let nan = vec![
            Point::um(0.0, 0.0),
            Point::um(f64::NAN, 0.0),
            Point::um(0.0, 1.0),
        ];
        assert!(Polygon::new(nan).is_err());
    }

    #[test]
    fn containment_follows_the_shape() {
        // an L shape: the notch at the top right is outside
        let l = Polygon::new(vec![
            Point::um(0.0, 0.0),
            Point::um(2.0, 0.0),
            Point::um(2.0, 1.0),
            Point::um(1.0, 1.0),
            Point::um(1.0, 2.0),
            Point::um(0.0, 2.0),
        ])
        .unwrap();
        assert!(l.contains(Point::um(0.5, 1.5)));
        assert!(l.contains(Point::um(1.5, 0.5)));
        assert!(!l.contains(Point::um(1.5, 1.5)));
        assert!(!l.contains(Point::um(-0.1, 0.5)));
        assert_eq!(l.area(), 3.0);
        let b = l.bounds();
        assert_eq!((b.min, b.max), (Point::um(0.0, 0.0), Point::um(2.0, 2.0)));
    }

    #[test]
    fn rectangles_and_circles() {
        let r = Shape::rect(Point::um(1.0, 0.0), Length::um(2.0), Length::nm(500.0)).unwrap();
        assert!(r.contains(Point::um(1.9, 0.2)));
        assert!(!r.contains(Point::um(1.9, 0.3)));
        assert!((r.area() - 1.0).abs() < 1e-15);
        let c = Shape::circle(Point::um(0.0, 0.0), Length::um(1.0)).unwrap();
        assert!(c.contains(Point::um(0.7, 0.7)));
        assert!(!c.contains(Point::um(0.75, 0.7)));
        assert!((c.area() - std::f64::consts::PI).abs() < 1e-15);
        assert!(Shape::rect(Point::um(0.0, 0.0), Length::um(-1.0), Length::um(1.0)).is_err());
        assert!(Shape::circle(Point::um(0.0, 0.0), Length::ZERO).is_err());
    }

    #[test]
    fn a_ring_is_the_band_about_its_radius() {
        // a 500 nm waveguide whose centre line has a 2 um radius: from 1.75 to 2.25 um
        let r = Shape::ring(Point::um(1.0, -1.0), Length::um(2.0), Length::nm(500.0)).unwrap();
        assert!(r.contains(Point::um(3.0, -1.0)));
        assert!(r.contains(Point::um(1.0, 1.2)));
        assert!(!r.contains(Point::um(1.0, -1.0)), "the hole");
        assert!(!r.contains(Point::um(1.0 + 1.7, -1.0)));
        assert!(!r.contains(Point::um(1.0 + 2.3, -1.0)));
        let exact = std::f64::consts::PI * (2.25 * 2.25 - 1.75 * 1.75);
        assert!((r.area() - exact).abs() < 1e-12);
        let b = r.bounds();
        assert!((b.max.x.to_um() - 3.25).abs() < 1e-12 && (b.min.y.to_um() + 3.25).abs() < 1e-12);
        assert!(Shape::ring(Point::um(0.0, 0.0), Length::um(1.0), Length::um(2.0)).is_err());
        assert!(Shape::ring(Point::um(0.0, 0.0), Length::um(1.0), Length::ZERO).is_err());
    }

    #[test]
    fn translation_moves_every_vertex() {
        let p = Polygon::new(square())
            .unwrap()
            .translated(Length::um(1.0), Length::um(-1.0));
        assert_eq!(p.vertices()[0], Point::um(1.0, -1.0));
        assert_eq!(p.area(), 2.0);
    }
}
