//! Planar shapes: what is drawn on a layer, seen from above.
//!
//! Coordinates are [`Length`]s in the plane of the chip: x along the chip, y across it. Shapes
//! are validated when they are made (finite coordinates, a non-zero area), so a solver never
//! meets a degenerate one.

use std::f64::consts::TAU;

use crate::units::Length;
use crate::{Error, Result};

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

    /// The area, in µm².
    pub fn area(&self) -> f64 {
        signed_area(&self.vertices)
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
}

impl Shape {
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
            Shape::Circle { center, radius } => Bounds {
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
        }
    }

    /// The area, in µm².
    pub fn area(&self) -> f64 {
        match self {
            Shape::Rect { width, height, .. } => width.to_um() * height.to_um(),
            Shape::Circle { radius, .. } => TAU / 2.0 * radius.to_um() * radius.to_um(),
            Shape::Polygon(p) => p.area(),
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
    fn translation_moves_every_vertex() {
        let p = Polygon::new(square())
            .unwrap()
            .translated(Length::um(1.0), Length::um(-1.0));
        assert_eq!(p.vertices()[0], Point::um(1.0, -1.0));
        assert_eq!(p.area(), 2.0);
    }
}
