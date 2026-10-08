//! Similarity transforms of the plane: translations, rotations, mirrors and uniform scalings,
//! and their compositions. They keep shapes' kinds: a rotated ellipse is an ellipse, a
//! mirrored sector a sector, so a transformed shape is still exact.

use std::f64::consts::FRAC_PI_2;

use super::primitives::{Ellipse, RegularPolygon, RoundedRect, Sector, Superellipse};
use super::{Point, Polygon, Shape};
use crate::units::Length;
use crate::{Error, Result};

/// sin and cos of `angle`, exact at whole quarter turns (where `f64::sin_cos` leaves 6e-17 for
/// a zero), so that a quarter turn keeps a rectangle a rectangle.
pub(crate) fn sin_cos(angle: f64) -> (f64, f64) {
    let quarters = angle / FRAC_PI_2;
    if quarters.fract() == 0.0 && quarters.abs() < 1e15 {
        match (quarters as i64).rem_euclid(4) {
            0 => (0.0, 1.0),
            1 => (1.0, 0.0),
            2 => (0.0, -1.0),
            _ => (-1.0, 0.0),
        }
    } else {
        angle.sin_cos()
    }
}

/// p ↦ M p + t, with M a rotation (or a mirror) times a positive scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    m: [[f64; 2]; 2],
    t: [f64; 2],
    /// The scale, kept as the product of the factors given rather than read back from M, so
    /// that rotations and mirrors keep lengths to the bit.
    s: f64,
}

impl Default for Transform {
    fn default() -> Transform {
        Transform::IDENTITY
    }
}

impl Transform {
    /// Nothing moves.
    pub const IDENTITY: Transform = Transform {
        m: [[1.0, 0.0], [0.0, 1.0]],
        t: [0.0, 0.0],
        s: 1.0,
    };

    /// A translation by (dx, dy).
    pub fn translate(dx: Length, dy: Length) -> Transform {
        Transform {
            t: [dx.to_um(), dy.to_um()],
            ..Transform::IDENTITY
        }
    }

    /// A rotation by `angle` (radians, counterclockwise) about `center`.
    pub fn rotate(angle: f64, center: Point) -> Transform {
        let (s, c) = sin_cos(angle);
        Transform::about([[c, -s], [s, c]], center, 1.0)
    }

    /// A mirror in the line through `point` at `angle` (radians) from x.
    pub fn mirror(point: Point, angle: f64) -> Transform {
        let (s, c) = sin_cos(2.0 * angle);
        Transform::about([[c, s], [s, -c]], point, 1.0)
    }

    /// A scaling by `factor` about `center`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the factor is positive and finite.
    pub fn scale(factor: f64, center: Point) -> Result<Transform> {
        if !(factor.is_finite() && factor > 0.0) {
            return Err(Error::invalid(
                "transform",
                format!("a scale needs a positive, finite factor, got {factor}"),
            ));
        }
        Ok(Transform::about(
            [[factor, 0.0], [0.0, factor]],
            center,
            factor,
        ))
    }

    /// M, of scale `s`, fixing `center`: p ↦ M (p − c) + c.
    fn about(m: [[f64; 2]; 2], center: Point, s: f64) -> Transform {
        let c = [center.x.to_um(), center.y.to_um()];
        let mc = [
            m[0][0] * c[0] + m[0][1] * c[1],
            m[1][0] * c[0] + m[1][1] * c[1],
        ];
        Transform {
            m,
            t: [c[0] - mc[0], c[1] - mc[1]],
            s,
        }
    }

    /// This transform, then `next`.
    pub fn then(&self, next: &Transform) -> Transform {
        let (a, b) = (next.m, self.m);
        let m = [
            [
                a[0][0] * b[0][0] + a[0][1] * b[1][0],
                a[0][0] * b[0][1] + a[0][1] * b[1][1],
            ],
            [
                a[1][0] * b[0][0] + a[1][1] * b[1][0],
                a[1][0] * b[0][1] + a[1][1] * b[1][1],
            ],
        ];
        Transform {
            m,
            t: next.map(self.t),
            s: self.s * next.s,
        }
    }

    /// The image of `p`.
    pub fn apply(&self, p: Point) -> Point {
        let q = self.map([p.x.to_um(), p.y.to_um()]);
        Point::um(q[0], q[1])
    }

    fn map(&self, p: [f64; 2]) -> [f64; 2] {
        [
            self.m[0][0] * p[0] + self.m[0][1] * p[1] + self.t[0],
            self.m[1][0] * p[0] + self.m[1][1] * p[1] + self.t[1],
        ]
    }

    /// How much it scales lengths.
    pub fn factor(&self) -> f64 {
        self.s
    }

    /// Whether it reverses orientation (a mirror).
    pub fn mirrored(&self) -> bool {
        self.m[0][0] * self.m[1][1] - self.m[0][1] * self.m[1][0] < 0.0
    }

    /// The direction a direction at `angle` from x goes to, as an angle from x.
    fn direction(&self, angle: f64) -> f64 {
        let (s, c) = sin_cos(angle);
        let v = [
            self.m[0][0] * c + self.m[0][1] * s,
            self.m[1][0] * c + self.m[1][1] * s,
        ];
        v[1].atan2(v[0])
    }

    fn length(&self, l: Length) -> Length {
        l * self.factor()
    }
}

impl Polygon {
    /// The polygon's image under `t`, counterclockwise again if `t` mirrors it.
    pub fn transformed(&self, t: &Transform) -> Polygon {
        let mut vertices: Vec<Point> = self.vertices().iter().map(|p| t.apply(*p)).collect();
        if t.mirrored() {
            vertices.reverse();
        }
        Polygon::from_ccw(vertices)
    }
}

impl Shape {
    /// The shape's image under `t`, of the same kind where the kind allows: a circle, a ring,
    /// an ellipse, a sector, a regular polygon, a superellipse or a rounded rectangle stays one,
    /// with its centre moved, its lengths scaled and its angles turned; a polygon's vertices
    /// move. A rectangle stays one when `t` keeps it axis-aligned (a quarter turn, a mirror in x
    /// or y), and becomes a [`Shape::RoundedRect`] of radius zero at its new angle otherwise.
    ///
    /// # Panics
    ///
    /// If the image's coordinates overflow to infinity (a transform beyond 10³⁰⁰ µm).
    pub fn transformed(&self, t: &Transform) -> Shape {
        let f = |l: Length| t.length(l);
        match self {
            Shape::Rect {
                center,
                width,
                height,
            } => {
                let m = t.m;
                if m[0][1] == 0.0 && m[1][0] == 0.0 {
                    Shape::Rect {
                        center: t.apply(*center),
                        width: *width * m[0][0].abs(),
                        height: *height * m[1][1].abs(),
                    }
                } else if m[0][0] == 0.0 && m[1][1] == 0.0 {
                    Shape::Rect {
                        center: t.apply(*center),
                        width: *height * m[0][1].abs(),
                        height: *width * m[1][0].abs(),
                    }
                } else {
                    Shape::RoundedRect(
                        RoundedRect::new(
                            t.apply(*center),
                            f(*width),
                            f(*height),
                            Length::ZERO,
                            t.direction(0.0),
                        )
                        .expect("a transformed rectangle is a rectangle"),
                    )
                }
            }
            Shape::Circle { center, radius } => Shape::Circle {
                center: t.apply(*center),
                radius: f(*radius),
            },
            Shape::Ring {
                center,
                inner,
                outer,
            } => Shape::Ring {
                center: t.apply(*center),
                inner: f(*inner),
                outer: f(*outer),
            },
            Shape::Polygon(p) => Shape::Polygon(p.transformed(t)),
            Shape::Ellipse(e) => {
                let [a, b] = e.semi_axes();
                Shape::Ellipse(
                    Ellipse::new(t.apply(e.center()), [f(a), f(b)], t.direction(e.angle()))
                        .expect("a transformed ellipse is an ellipse"),
                )
            }
            Shape::Sector(s) => {
                let [inner, outer] = s.radii();
                let [start, sweep] = s.angles();
                // a mirror reverses the sweep: it starts where the image of its end is
                let from = if t.mirrored() {
                    t.direction(start + sweep)
                } else {
                    t.direction(start)
                };
                Shape::Sector(
                    Sector::new(t.apply(s.center()), f(inner), f(outer), from, sweep)
                        .expect("a transformed sector is a sector"),
                )
            }
            Shape::RegularPolygon(r) => Shape::RegularPolygon(
                RegularPolygon::new(
                    t.apply(r.center()),
                    f(r.radius()),
                    r.sides(),
                    t.direction(r.angle()),
                )
                .expect("a transformed regular polygon is one"),
            ),
            Shape::Superellipse(s) => {
                let [a, b] = s.semi_axes();
                Shape::Superellipse(
                    Superellipse::new(
                        t.apply(s.center()),
                        [f(a), f(b)],
                        s.exponent(),
                        t.direction(s.angle()),
                    )
                    .expect("a transformed superellipse is one"),
                )
            }
            Shape::RoundedRect(r) => {
                let [w, h] = r.size();
                Shape::RoundedRect(
                    RoundedRect::new(
                        t.apply(r.center()),
                        f(w),
                        f(h),
                        f(r.radius()),
                        t.direction(r.angle()),
                    )
                    .expect("a transformed rounded rectangle is one"),
                )
            }
        }
    }
}
