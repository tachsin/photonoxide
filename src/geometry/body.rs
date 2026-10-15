//! Bodies in space: what fills a region of (x, y, z), for the solvers that see a structure in
//! three dimensions (#227, §1 and §6).
//!
//! A [`Body`] answers as a [`super::Region`] does in the plane: whether it holds a point
//! ([`Body::locate`]), the signed distance to its surface with the outward normal
//! ([`Body::distance`]), its bounds and volume, and the volume it fills of an axis-aligned cell
//! ([`Body::fill`]), what averaging over a grid's cells needs. Coordinates are µm, as `[x, y,
//! z]`; volumes µm³.
//!
//! **The bodies.** A half-space; an ellipsoid at any angle (a sphere; with one or two infinite
//! semi-axes an elliptic cylinder or a slab); an axis-aligned box; a cylinder and a cone or
//! frustum along any axis; any planar [`Shape`] extruded without end along z, or between two
//! heights with its walls leaning out at a sidewall angle ([`crate::stack::Etch`]'s, so a
//! strip's cross-section is a trapezoid); and the union, intersection, difference or xor of
//! two bodies ([`Operation`]).
//!
//! **Distances.** Exact for the half-space, the box, the cylinder and cone (the distance in the
//! plane through the axis to the profile they turn), and the ellipsoid (its nearest point by
//! the Lagrange condition, one monotone equation in one variable solved by bisection to the
//! last bit). An extrusion's is the distance in the plane of (s, z), s the planar shape's own
//! signed distance, to the profile its walls make: exact where s is and the nearest point is
//! regular, and never more than the true distance, s being 1-Lipschitz. Booleans compose their
//! operands' by min and max: exact outside a union and inside an intersection, a bound below
//! the true distance elsewhere.
//!
//! **FDTD's smoothing reads these bodies** ([`crate::fdtd::Body`] is this type): each surface
//! as the plane through its nearest point, [`Body::surface`]'s estimate, which for an
//! ellipsoid is f/|∇f| (second order in itself, as #161 has always taken it) and exact
//! otherwise. docs/methods/bodies.md has the methods and their validation.

use std::f64::consts::PI;

use super::region::{Location, Region};
use super::{Bounds, Operation, Point, Shape, boundary};
use crate::units::Length;
use crate::{Error, Result};

/// An axis-aligned box in space, µm: the corner with the least x, y and z, and the one with the
/// greatest. A side may be infinite, as a half-space's or an extrusion's bounds are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds3d {
    /// The corner with the least x, y and z.
    pub min: [f64; 3],
    /// The corner with the greatest.
    pub max: [f64; 3],
}

impl Bounds3d {
    /// Everything.
    pub const EVERYWHERE: Bounds3d = Bounds3d {
        min: [f64::NEG_INFINITY; 3],
        max: [f64::INFINITY; 3],
    };

    /// The box with corners `a` and `b`, in either order.
    pub fn new(a: [f64; 3], b: [f64; 3]) -> Bounds3d {
        Bounds3d {
            min: std::array::from_fn(|k| a[k].min(b[k])),
            max: std::array::from_fn(|k| a[k].max(b[k])),
        }
    }

    /// The box of `size` (µm) centred on `centre`.
    pub fn about(centre: [f64; 3], size: [f64; 3]) -> Bounds3d {
        Bounds3d {
            min: std::array::from_fn(|k| centre[k] - 0.5 * size[k]),
            max: std::array::from_fn(|k| centre[k] + 0.5 * size[k]),
        }
    }

    /// Its centre.
    pub fn centre(&self) -> [f64; 3] {
        std::array::from_fn(|k| 0.5 * (self.min[k] + self.max[k]))
    }

    /// Its size along x, y and z.
    pub fn size(&self) -> [f64; 3] {
        std::array::from_fn(|k| self.max[k] - self.min[k])
    }

    /// Its volume, µm³: zero for an empty box, infinite for an unbounded one.
    pub fn volume(&self) -> f64 {
        if self.is_empty() {
            return 0.0;
        }
        self.size().iter().product()
    }

    /// Whether it holds no volume: a side of zero or negative length.
    pub fn is_empty(&self) -> bool {
        !(0..3).all(|k| self.max[k] > self.min[k])
    }

    /// Whether it holds `p`, faces included.
    pub fn contains(&self, p: [f64; 3]) -> bool {
        (0..3).all(|k| p[k] >= self.min[k] && p[k] <= self.max[k])
    }

    /// Whether the two boxes share any point, faces included.
    pub fn overlaps(&self, other: &Bounds3d) -> bool {
        (0..3).all(|k| self.min[k] <= other.max[k] && other.min[k] <= self.max[k])
    }

    /// Whether this box holds all of `other`.
    pub fn encloses(&self, other: &Bounds3d) -> bool {
        (0..3).all(|k| self.min[k] <= other.min[k] && self.max[k] >= other.max[k])
    }

    /// The volume this box shares with `other`, µm³.
    pub fn overlap(&self, other: &Bounds3d) -> f64 {
        let mut v = 1.0;
        for k in 0..3 {
            let l = self.max[k].min(other.max[k]) - self.min[k].max(other.min[k]);
            if l.is_nan() || l <= 0.0 {
                return 0.0;
            }
            v *= l;
        }
        v
    }

    /// The part of this box inside `other` (empty when they don't overlap).
    pub fn intersection(&self, other: &Bounds3d) -> Bounds3d {
        Bounds3d {
            min: std::array::from_fn(|k| self.min[k].max(other.min[k])),
            max: std::array::from_fn(|k| self.max[k].min(other.max[k])),
        }
    }

    /// The smallest box holding both.
    pub fn hull(&self, other: &Bounds3d) -> Bounds3d {
        Bounds3d {
            min: std::array::from_fn(|k| self.min[k].min(other.min[k])),
            max: std::array::from_fn(|k| self.max[k].max(other.max[k])),
        }
    }

    /// Its footprint in the plane: x and y.
    pub fn plan(&self) -> Bounds {
        Bounds {
            min: Point::um(self.min[0], self.min[1]),
            max: Point::um(self.max[0], self.max[1]),
        }
    }

    /// The distance from `p` to the box, zero inside it.
    pub(crate) fn gap(&self, p: [f64; 3]) -> f64 {
        let d: [f64; 3] =
            std::array::from_fn(|k| (self.min[k] - p[k]).max(p[k] - self.max[k]).max(0.0));
        norm(d)
    }
}

/// The signed distance from a point to a body's surface and the outward normal there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Distance3d {
    /// The distance to the nearest point of the surface: negative inside, positive outside.
    pub signed: Length,
    /// The unit normal pointing out of the body at that point: along the line from the point
    /// to it (away from the body) when the point is off the surface.
    pub normal: [f64; 3],
}

/// A body in space, filled with one medium. Made by its functions, which check it; cheap to
/// clone but for the planar shapes it holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Body(Kind);

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Kind {
    HalfSpace {
        point: [f64; 3],
        normal: [f64; 3],
    },
    Ellipsoid {
        center: [f64; 3],
        /// The reciprocal semi-axes, 0 for an infinite one.
        inverse: [f64; 3],
        axes: [[f64; 3]; 3],
    },
    Extruded {
        shape: Shape,
        /// Its bounding box, µm: the least x and y, then the greatest.
        bounds: [f64; 4],
    },
    Etched {
        shape: Shape,
        /// The top face's bounding box, µm: the least x and y, then the greatest.
        bounds: [f64; 4],
        /// Its bottom and top, µm.
        z: [f64; 2],
        /// How far the walls reach out per µm down, cot θ.
        run: f64,
    },
    Cuboid {
        min: [f64; 3],
        max: [f64; 3],
    },
    Frustum(Frustum),
    Boolean {
        op: Operation,
        operands: Box<[Body; 2]>,
    },
}

/// A cylinder or a cone or frustum: the disks of radius r(t) = r₀ + (r₁ − r₀) t/L across the
/// axis, t from 0 at the base to L at the top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Frustum {
    pub base: [f64; 3],
    /// The unit vector from the base to the top.
    pub axis: [f64; 3],
    pub length: f64,
    /// The radii at the base and at the top.
    pub radii: [f64; 2],
    /// A unit vector across the axis: the radial direction on the axis itself.
    pub across: [f64; 3],
}

impl Frustum {
    /// The point's place in the plane through the axis: t along it from the base, ρ from it,
    /// and the unit radial direction.
    fn meridian(&self, p: [f64; 3]) -> (f64, f64, [f64; 3]) {
        let q = sub(p, self.base);
        let t = dot(q, self.axis);
        let perp = sub(q, scale(self.axis, t));
        let rho = norm(perp);
        let radial = if rho > 0.0 {
            scale(perp, 1.0 / rho)
        } else {
            self.across
        };
        (t, rho, radial)
    }

    fn radius(&self, t: f64) -> f64 {
        let [r0, r1] = self.radii;
        r0 + (r1 - r0) * t / self.length
    }

    /// The outward normal of the side in the plane through the axis, (along, out).
    fn side_normal(&self) -> [f64; 2] {
        let [r0, r1] = self.radii;
        let (a, b) = (-(r1 - r0), self.length);
        let l = a.hypot(b);
        [a / l, b / l]
    }

    fn locate(&self, p: [f64; 3]) -> Location {
        let (t, rho, _) = self.meridian(p);
        if t < 0.0 || t > self.length {
            return Location::Outside;
        }
        let r = self.radius(t);
        let order = rho.total_cmp(&r);
        if t == 0.0 || t == self.length {
            return if order == std::cmp::Ordering::Greater {
                Location::Outside
            } else {
                Location::Boundary
            };
        }
        Location::of(order)
    }

    /// The exact signed distance and outward normal: the distance in the plane through the
    /// axis from (t, ρ) to the profile, the base, the side and the top.
    fn distance(&self, p: [f64; 3]) -> (f64, [f64; 3]) {
        let (t, rho, radial) = self.meridian(p);
        let [r0, r1] = self.radii;
        let l = self.length;
        let inside = t >= 0.0 && t <= l && rho <= self.radius(t);
        let segments: [([f64; 2], [f64; 2], [f64; 2]); 3] = [
            ([0.0, 0.0], [0.0, r0], [-1.0, 0.0]),
            ([0.0, r0], [l, r1], self.side_normal()),
            ([l, r1], [l, 0.0], [1.0, 0.0]),
        ];
        let q = [t, rho];
        let mut best = (f64::INFINITY, q, [1.0, 0.0]);
        for (a, b, n) in segments {
            let (d, near) = segment_nearest(q, a, b);
            if d < best.0 {
                best = (d, near, n);
            }
        }
        let (d2, n2) = boundary::signed(best, q, inside);
        let n = add(scale(self.axis, n2[0]), scale(radial, n2[1]));
        (d2, n)
    }

    /// The side's tangent plane through the point's place, and the two caps' planes: the
    /// frustum is where all three are negative.
    fn planes(&self, p: [f64; 3]) -> [(f64, [f64; 3]); 3] {
        let (t, rho, radial) = self.meridian(p);
        let n = self.side_normal();
        let side = n[0] * t + n[1] * (rho - self.radii[0]);
        [
            (side, add(scale(self.axis, n[0]), scale(radial, n[1]))),
            (-t, scale(self.axis, -1.0)),
            (t - self.length, self.axis),
        ]
    }
}

/// The nearest point of the segment from `a` to `b` to `q`, and its distance.
fn segment_nearest(q: [f64; 2], a: [f64; 2], b: [f64; 2]) -> (f64, [f64; 2]) {
    let e = [b[0] - a[0], b[1] - a[1]];
    let ee = e[0] * e[0] + e[1] * e[1];
    let s = if ee > 0.0 {
        (((q[0] - a[0]) * e[0] + (q[1] - a[1]) * e[1]) / ee).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let near = [a[0] + s * e[0], a[1] + s * e[1]];
    ((q[0] - near[0]).hypot(q[1] - near[1]), near)
}

fn invalid(what: &'static str, reason: String) -> Error {
    Error::invalid(what, reason)
}

fn finite3(v: [f64; 3]) -> bool {
    v.iter().all(|x| x.is_finite())
}

impl Body {
    /// The half-space on the side of the plane through `point` (µm) that `normal` (any length
    /// but zero) points away from.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless everything is finite and the normal isn't zero.
    pub fn half_space(point: [f64; 3], normal: [f64; 3]) -> Result<Body> {
        let length = norm(normal);
        if !(finite3(point) && length.is_finite() && length > 0.0) {
            return Err(Error::invalid(
                "body",
                format!(
                    "a half-space needs a finite point and normal, got {point:?} and {normal:?}"
                ),
            ));
        }
        Ok(Body(Kind::HalfSpace {
            point,
            normal: normal.map(|v| v / length),
        }))
    }

    /// An ellipsoid about `center` (µm) with semi-axes `semi_axes` (µm) along the orthonormal
    /// directions `axes` (one per row). A semi-axis may be infinite: one makes an elliptic
    /// cylinder, two a slab.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless the centre is finite, the semi-axes positive (at
    /// most two infinite) and the axes orthonormal to 1e-9.
    pub fn ellipsoid(center: [f64; 3], semi_axes: [f64; 3], axes: [[f64; 3]; 3]) -> Result<Body> {
        let finite = semi_axes.iter().filter(|s| s.is_finite()).count();
        if !(finite3(center)
            && semi_axes.iter().all(|s| *s > 0.0)
            && finite >= 1
            && orthonormal(&axes))
        {
            return Err(Error::invalid(
                "body",
                format!(
                    "an ellipsoid needs a finite centre, positive semi-axes (one at least finite) and \
                 orthonormal axes, got {center:?}, {semi_axes:?} and {axes:?}"
                ),
            ));
        }
        Ok(Body(Kind::Ellipsoid {
            center,
            inverse: semi_axes.map(|s| 1.0 / s),
            axes,
        }))
    }

    /// A sphere about `center` (µm) of `radius` (µm).
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless the centre is finite and the radius positive and
    /// finite.
    pub fn sphere(center: [f64; 3], radius: f64) -> Result<Body> {
        if !(radius.is_finite() && radius > 0.0) {
            return Err(invalid(
                "sphere",
                format!("needs a positive, finite radius, got {radius} um"),
            ));
        }
        Body::ellipsoid(
            center,
            [radius; 3],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        )
    }

    /// An elliptic cylinder along z: the ellipse about `center` (x, y, µm) with semi-axes
    /// `semi_axes` (µm), the first at `angle` (radians) from x, counterclockwise.
    ///
    /// # Errors
    ///
    /// As [`Body::ellipsoid`].
    pub fn ellipse(center: [f64; 2], semi_axes: [f64; 2], angle: f64) -> Result<Body> {
        let (s, c) = angle.sin_cos();
        Body::ellipsoid(
            [center[0], center[1], 0.0],
            [semi_axes[0], semi_axes[1], f64::INFINITY],
            [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]],
        )
    }

    /// The axis-aligned box with corners `a` and `b` (µm), in either order.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless the corners are finite and the box has a positive
    /// size along every axis.
    pub fn cuboid(a: [f64; 3], b: [f64; 3]) -> Result<Body> {
        let bounds = Bounds3d::new(a, b);
        if !(finite3(a) && finite3(b)) || bounds.is_empty() {
            return Err(invalid(
                "box",
                format!("needs finite corners a positive size apart, got {a:?} and {b:?}"),
            ));
        }
        Ok(Body(Kind::Cuboid {
            min: bounds.min,
            max: bounds.max,
        }))
    }

    /// A cylinder of `radius` (µm) whose axis runs from the centre of its base `base` to that of
    /// its top `top` (µm).
    ///
    /// # Errors
    ///
    /// As [`Body::cone`].
    pub fn cylinder(base: [f64; 3], top: [f64; 3], radius: f64) -> Result<Body> {
        Body::cone(base, top, [radius, radius])
    }

    /// A cone or frustum: its axis from the centre of its base `base` to that of its top `top`
    /// (µm), its radius `radii[0]` at the base and `radii[1]` at the top (µm), changing
    /// linearly between; a radius of zero is an apex.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless the ends are finite and apart, and the radii finite,
    /// at least zero and not both zero.
    pub fn cone(base: [f64; 3], top: [f64; 3], radii: [f64; 2]) -> Result<Body> {
        let d = sub(top, base);
        let length = norm(d);
        if !(finite3(base)
            && finite3(top)
            && length > 0.0
            && length.is_finite()
            && radii.iter().all(|r| r.is_finite() && *r >= 0.0)
            && radii.iter().any(|r| *r > 0.0))
        {
            return Err(invalid(
                "cone",
                format!(
                    "needs finite ends apart and finite radii, at least zero and not both zero, \
                     got {base:?} to {top:?} with radii {radii:?}"
                ),
            ));
        }
        let axis = scale(d, 1.0 / length);
        // the grid's axis least along the axis, crossed with it
        let least = (0..3)
            .min_by(|&i, &j| axis[i].abs().total_cmp(&axis[j].abs()))
            .unwrap_or(0);
        let mut e = [0.0; 3];
        e[least] = 1.0;
        let c = cross(axis, e);
        Ok(Body(Kind::Frustum(Frustum {
            base,
            axis,
            length,
            radii,
            across: scale(c, 1.0 / norm(c)),
        })))
    }

    /// A planar shape (as [`crate::geometry`] draws it, x and y in µm) extended without end
    /// along z.
    pub fn extruded(shape: Shape) -> Body {
        let b = shape.bounds();
        let bounds = [b.min.x, b.min.y, b.max.x, b.max.y].map(|v| v.to_um());
        Body(Kind::Extruded { shape, bounds })
    }

    /// A planar shape's etched body ([`crate::stack::Etch`]): `shape` (x and y in µm) its top
    /// face at z = `z[1]`, standing down to `z[0]`, its walls leaning out going down at
    /// `sidewall` radians from the plane (π/2: vertical, the shape extruded between the two
    /// heights). Its surface is the walls' planes through the outline's nearest point, and the
    /// two faces: a trapezoid's sides exactly, so the smoothing takes each cell's exact share
    /// of a straight sloped wall, with its normal.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless the heights are finite, the top above the bottom,
    /// and the sidewall above 0 and at most π/2.
    pub fn etched(shape: Shape, z: [f64; 2], sidewall: f64) -> Result<Body> {
        if !(z[0].is_finite()
            && z[1].is_finite()
            && z[1] > z[0]
            && sidewall > 0.0
            && sidewall <= std::f64::consts::FRAC_PI_2 + 1e-12)
        {
            return Err(Error::invalid(
                "body",
                format!(
                    "an etched body needs a top above its bottom and a sidewall above 0 and at \
                 most 90 degrees, got {z:?} and {sidewall} rad"
                ),
            ));
        }
        let run = crate::stack::Etch::FULL.with_sidewall(sidewall).run();
        let b = shape.bounds();
        let bounds = [b.min.x, b.min.y, b.max.x, b.max.y].map(|v| v.to_um());
        Ok(Body(Kind::Etched {
            shape,
            bounds,
            z,
            run,
        }))
    }

    /// `shape` extruded between the heights `z` (µm) with vertical walls: [`Body::etched`] at
    /// π/2.
    ///
    /// # Errors
    ///
    /// As [`Body::etched`].
    pub fn prism(shape: Shape, z: [f64; 2]) -> Result<Body> {
        Body::etched(shape, z, std::f64::consts::FRAC_PI_2)
    }

    /// The boolean `op` of two bodies: their union, intersection, difference (`a` less `b`) or
    /// xor.
    pub fn boolean(op: Operation, a: Body, b: Body) -> Body {
        Body(Kind::Boolean {
            op,
            operands: Box::new([a, b]),
        })
    }

    /// The union of two bodies.
    pub fn union(a: Body, b: Body) -> Body {
        Body::boolean(Operation::Union, a, b)
    }

    /// The intersection of two bodies.
    pub fn intersection(a: Body, b: Body) -> Body {
        Body::boolean(Operation::Intersection, a, b)
    }

    /// `a` less `b`.
    pub fn difference(a: Body, b: Body) -> Body {
        Body::boolean(Operation::Difference, a, b)
    }

    /// Whether `p` is inside, on the surface or outside, exactly as far as the definition's
    /// arithmetic allows. An extruded or etched shape answers as the shape does in its plane.
    pub fn locate(&self, p: [f64; 3]) -> Location {
        if self.beyond(p, 0.0) {
            return Location::Outside;
        }
        match &self.0 {
            Kind::HalfSpace { point, normal } => {
                Location::of(dot(*normal, sub(p, *point)).total_cmp(&0.0))
            }
            Kind::Ellipsoid {
                center,
                inverse,
                axes,
            } => {
                let q = sub(p, *center);
                let rho2: f64 = (0..3)
                    .map(|i| {
                        let u = dot(q, axes[i]) * inverse[i];
                        u * u
                    })
                    .sum();
                Location::of(rho2.total_cmp(&1.0))
            }
            Kind::Extruded { shape, .. } => shape.locate(Point::um(p[0], p[1])),
            Kind::Etched { shape, z, run, .. } => {
                if p[2] < z[0] || p[2] > z[1] {
                    return Location::Outside;
                }
                let plan = Point::um(p[0], p[1]);
                let at = if *run == 0.0 {
                    shape.locate(plan)
                } else {
                    let reach = (z[1] - p[2]) * run;
                    Location::of(shape.distance(plan).signed.to_um().total_cmp(&reach))
                };
                if at != Location::Outside && (p[2] == z[0] || p[2] == z[1]) {
                    Location::Boundary
                } else {
                    at
                }
            }
            Kind::Cuboid { min, max } => {
                if (0..3).any(|k| p[k] < min[k] || p[k] > max[k]) {
                    Location::Outside
                } else if (0..3).any(|k| p[k] == min[k] || p[k] == max[k]) {
                    Location::Boundary
                } else {
                    Location::Inside
                }
            }
            Kind::Frustum(f) => f.locate(p),
            Kind::Boolean { op, operands } => {
                combine(*op, operands[0].locate(p), operands[1].locate(p))
            }
        }
    }

    /// Whether the closed body holds `p`.
    pub fn contains(&self, p: [f64; 3]) -> bool {
        self.locate(p) != Location::Outside
    }

    /// The signed distance from `p` to the surface, negative inside, and the outward normal at
    /// the nearest point. Exact for the primitives; see the module for extrusions and booleans.
    pub fn distance(&self, p: [f64; 3]) -> Distance3d {
        let (d, n) = self.distance_um(p);
        Distance3d {
            signed: Length::um(d),
            normal: n,
        }
    }

    /// [`Body::distance`] in µm.
    pub(crate) fn distance_um(&self, p: [f64; 3]) -> (f64, [f64; 3]) {
        match &self.0 {
            Kind::HalfSpace { .. } | Kind::Extruded { .. } | Kind::Cuboid { .. } => self.surface(p),
            Kind::Ellipsoid {
                center,
                inverse,
                axes,
            } => ellipsoid_distance(*center, *inverse, axes, p),
            Kind::Etched { shape, z, run, .. } => {
                let (s, n) = planar(shape, p[0], p[1]);
                let (d, m) = profile_distance(s, p[2], *z, *run);
                (d, [m[0] * n[0], m[0] * n[1], m[1]])
            }
            Kind::Frustum(f) => f.distance(p),
            Kind::Boolean { op, operands } => {
                compose(*op, operands[0].distance_um(p), operands[1].distance_um(p))
            }
        }
    }

    /// The smallest axis-aligned box holding the body; infinite along an axis it has no end
    /// along, empty for a body with nothing in it (an intersection of two that don't meet).
    pub fn bounds(&self) -> Bounds3d {
        match &self.0 {
            Kind::HalfSpace { point, normal } => {
                let mut b = Bounds3d::EVERYWHERE;
                let along = (0..3).filter(|&k| normal[k] != 0.0).collect::<Vec<_>>();
                if let [k] = along[..] {
                    if normal[k] > 0.0 {
                        b.max[k] = point[k];
                    } else {
                        b.min[k] = point[k];
                    }
                }
                b
            }
            Kind::Ellipsoid {
                center,
                inverse,
                axes,
            } => {
                let half: [f64; 3] = std::array::from_fn(|k| {
                    let mut sum = 0.0;
                    for i in 0..3 {
                        if axes[i][k] == 0.0 {
                            continue;
                        }
                        if inverse[i] == 0.0 {
                            return f64::INFINITY;
                        }
                        let e = axes[i][k] / inverse[i];
                        sum += e * e;
                    }
                    sum.sqrt()
                });
                Bounds3d {
                    min: std::array::from_fn(|k| center[k] - half[k]),
                    max: std::array::from_fn(|k| center[k] + half[k]),
                }
            }
            Kind::Extruded { bounds, .. } => Bounds3d {
                min: [bounds[0], bounds[1], f64::NEG_INFINITY],
                max: [bounds[2], bounds[3], f64::INFINITY],
            },
            Kind::Etched { bounds, z, run, .. } => {
                let out = (z[1] - z[0]) * run;
                Bounds3d {
                    min: [bounds[0] - out, bounds[1] - out, z[0]],
                    max: [bounds[2] + out, bounds[3] + out, z[1]],
                }
            }
            Kind::Cuboid { min, max } => Bounds3d {
                min: *min,
                max: *max,
            },
            Kind::Frustum(f) => {
                let top = add(f.base, scale(f.axis, f.length));
                let ends = [(f.base, f.radii[0]), (top, f.radii[1])];
                let mut b = Bounds3d {
                    min: [f64::INFINITY; 3],
                    max: [f64::NEG_INFINITY; 3],
                };
                for (c, r) in ends {
                    for (k, ck) in c.iter().enumerate() {
                        let e = r * (1.0 - f.axis[k] * f.axis[k]).max(0.0).sqrt();
                        b.min[k] = b.min[k].min(ck - e);
                        b.max[k] = b.max[k].max(ck + e);
                    }
                }
                b
            }
            Kind::Boolean { op, operands } => {
                let (a, b) = (operands[0].bounds(), operands[1].bounds());
                match op {
                    Operation::Union | Operation::Xor => a.hull(&b),
                    Operation::Intersection => a.intersection(&b),
                    Operation::Difference => a,
                }
            }
        }
    }

    /// The volume, µm³: infinite for a body without end. Exact for the primitives and for an
    /// extrusion with vertical walls (the shape's area times the height) or of a circle (a
    /// frustum); otherwise [`Body::fill`] of its bounds, to its tolerance.
    pub fn volume(&self) -> f64 {
        match &self.0 {
            Kind::HalfSpace { .. } | Kind::Extruded { .. } => f64::INFINITY,
            Kind::Ellipsoid { inverse, .. } => {
                if inverse.contains(&0.0) {
                    f64::INFINITY
                } else {
                    4.0 / 3.0 * PI / (inverse[0] * inverse[1] * inverse[2])
                }
            }
            Kind::Etched { shape, z, run, .. } => {
                let h = z[1] - z[0];
                if *run == 0.0 {
                    return shape.area() * h;
                }
                if let Shape::Circle { radius, .. } = shape {
                    let (r1, r0) = (radius.to_um(), radius.to_um() + h * run);
                    return PI * h * (r0 * r0 + r0 * r1 + r1 * r1) / 3.0;
                }
                self.fill(self.bounds())
            }
            Kind::Cuboid { min, max } => (0..3).map(|k| max[k] - min[k]).product(),
            Kind::Frustum(f) => {
                let [r0, r1] = f.radii;
                PI * f.length * (r0 * r0 + r0 * r1 + r1 * r1) / 3.0
            }
            Kind::Boolean { .. } => {
                let b = self.bounds();
                if b.is_empty() {
                    0.0
                } else if !b.volume().is_finite() {
                    f64::INFINITY
                } else {
                    self.fill(b)
                }
            }
        }
    }

    /// Whether `p` is farther than `r` (µm) from the body's bounding box, outside it: then it
    /// is outside the body and farther than `r` from its surface, known without the surface's
    /// distance, which for a polygon costs a pass over its edges. Never for a half-space or an
    /// ellipsoid.
    pub(crate) fn beyond(&self, p: [f64; 3], r: f64) -> bool {
        match &self.0 {
            Kind::Extruded { bounds, .. } => {
                let dx = (bounds[0] - p[0]).max(p[0] - bounds[2]).max(0.0);
                let dy = (bounds[1] - p[1]).max(p[1] - bounds[3]).max(0.0);
                dx.hypot(dy) > r
            }
            Kind::Etched { bounds, z, run, .. } => {
                let out = (z[1] - z[0]) * run;
                let dx = (bounds[0] - out - p[0])
                    .max(p[0] - bounds[2] - out)
                    .max(0.0);
                let dy = (bounds[1] - out - p[1])
                    .max(p[1] - bounds[3] - out)
                    .max(0.0);
                let dz = (z[0] - p[2]).max(p[2] - z[1]).max(0.0);
                dx.hypot(dy).hypot(dz) > r
            }
            Kind::HalfSpace { .. } | Kind::Ellipsoid { .. } => false,
            Kind::Cuboid { .. } | Kind::Frustum(_) | Kind::Boolean { .. } => {
                self.bounds().gap(p) > r
            }
        }
    }

    /// The signed distance from `p` to the surface, negative inside (exact for a half-space, a
    /// sphere and the planar shapes, to second order in itself for an ellipsoid), and the unit
    /// normal pointing out of the body at the nearest point of the surface: what FDTD's
    /// smoothing takes as the plane through the nearest point.
    pub(crate) fn surface(&self, p: [f64; 3]) -> (f64, [f64; 3]) {
        match &self.0 {
            Kind::HalfSpace { point, normal } => (dot(*normal, sub(p, *point)), *normal),
            Kind::Ellipsoid {
                center,
                inverse,
                axes,
            } => {
                // u in the ellipsoid's own units, |u| = 1 on its surface; f = |u| − 1 has the
                // gradient g/|u|, g = Σ uᵢ aᵢ/sᵢ, and the distance is f/|∇f|: exact for a
                // sphere, second order in itself otherwise
                let q = sub(p, *center);
                let u: [f64; 3] = std::array::from_fn(|i| dot(q, axes[i]) * inverse[i]);
                let rho = norm(u);
                let g: [f64; 3] =
                    std::array::from_fn(|k| (0..3).map(|i| u[i] * inverse[i] * axes[i][k]).sum());
                let length = norm(g);
                if rho == 0.0 || length == 0.0 {
                    // the centre: deep inside
                    let smallest = inverse.iter().fold(0.0f64, |m, v| m.max(*v));
                    return (-1.0 / smallest, axes[0]);
                }
                ((rho - 1.0) * rho / length, g.map(|v| v / length))
            }
            Kind::Extruded { shape, .. } => {
                let (d, n) = planar(shape, p[0], p[1]);
                (d, [n[0], n[1], 0.0])
            }
            Kind::Etched { shape, z, run, .. } => {
                // the wall: d − (top − z) cot θ over its gradient's length, the distance to
                // its plane through the outline's nearest point
                let (d, n) = planar(shape, p[0], p[1]);
                let length = run.hypot(1.0);
                let wall = (
                    (d - (z[1] - p[2]) * run) / length,
                    [n[0] / length, n[1] / length, run / length],
                );
                // and the faces: the body is where all three are negative
                [
                    wall,
                    (p[2] - z[1], [0.0, 0.0, 1.0]),
                    (z[0] - p[2], [0.0, 0.0, -1.0]),
                ]
                .into_iter()
                .fold((f64::NEG_INFINITY, [0.0; 3]), |best, s| {
                    if s.0 > best.0 { s } else { best }
                })
            }
            Kind::Cuboid { min, max } => box_distance(*min, *max, p),
            Kind::Frustum(f) => f.distance(p),
            Kind::Boolean { op, operands } => {
                compose(*op, operands[0].surface(p), operands[1].surface(p))
            }
        }
    }

    /// The half-spaces whose intersection is the body near `p`, each as its signed distance
    /// from `p` and its outward normal (n·(x − p) + d ≤ 0 inside): exact for planar faces, the
    /// tangent plane at the nearest point of a curved one. A boolean gives its composed
    /// surface's plane alone.
    pub(crate) fn planes(&self, p: [f64; 3]) -> Vec<(f64, [f64; 3])> {
        match &self.0 {
            Kind::HalfSpace { .. } | Kind::Extruded { .. } => vec![self.surface(p)],
            Kind::Ellipsoid { .. } | Kind::Boolean { .. } => vec![self.distance_um(p)],
            Kind::Etched { shape, z, run, .. } => {
                let (d, n) = planar(shape, p[0], p[1]);
                let length = run.hypot(1.0);
                vec![
                    (
                        (d - (z[1] - p[2]) * run) / length,
                        [n[0] / length, n[1] / length, run / length],
                    ),
                    (p[2] - z[1], [0.0, 0.0, 1.0]),
                    (z[0] - p[2], [0.0, 0.0, -1.0]),
                ]
            }
            Kind::Cuboid { min, max } => {
                let mut out = Vec::with_capacity(6);
                for k in 0..3 {
                    let mut e = [0.0; 3];
                    e[k] = 1.0;
                    out.push((p[k] - max[k], e));
                    e[k] = -1.0;
                    out.push((min[k] - p[k], e));
                }
                out
            }
            Kind::Frustum(f) => f.planes(p).to_vec(),
        }
    }

    pub(crate) fn kind(&self) -> &Kind {
        &self.0
    }

    /// Whether its sections by horizontal planes are all the same: a box, an extrusion with
    /// vertical walls, a cylinder or an elliptic cylinder along z, a half-space with a
    /// horizontal normal, and booleans of those.
    pub(crate) fn upright(&self) -> bool {
        match &self.0 {
            Kind::Cuboid { .. } | Kind::Extruded { .. } => true,
            Kind::Etched { run, .. } => *run == 0.0,
            Kind::Frustum(f) => f.axis[2].abs() == 1.0 && f.radii[0] == f.radii[1],
            Kind::Ellipsoid { inverse, axes, .. } => {
                inverse.iter().filter(|v| **v == 0.0).count() == 1
                    && (0..3).any(|i| inverse[i] == 0.0 && axes[i][2].abs() == 1.0)
            }
            Kind::HalfSpace { normal, .. } => normal[2] == 0.0,
            Kind::Boolean { operands, .. } => operands[0].upright() && operands[1].upright(),
        }
    }
}

/// The signed distance from (s, z) to the profile of an extrusion between heights `z` whose
/// walls reach out `run` per unit down, in the plane of (s, z), s the planar shape's own signed
/// distance: the top face (s ≤ 0 at z₁), the wall, and the bottom face (s ≤ (z₁ − z₀) run at
/// z₀). The normal is in that plane, (along s, up).
fn profile_distance(s: f64, z: f64, heights: [f64; 2], run: f64) -> (f64, [f64; 2]) {
    let [z0, z1] = heights;
    let reach = (z1 - z0) * run;
    let inside = z >= z0 && z <= z1 && s <= (z1 - z) * run;
    let q = [s, z];
    let length = run.hypot(1.0);
    let top = ([s.min(0.0), z1], [0.0, 1.0]);
    let bottom = ([s.min(reach), z0], [0.0, -1.0]);
    let (dw, wall) = segment_nearest(q, [0.0, z1], [reach, z0]);
    let mut best = (dw, wall, [1.0 / length, run / length]);
    for (near, n) in [top, bottom] {
        let d = (q[0] - near[0]).hypot(q[1] - near[1]);
        if d < best.0 {
            best = (d, near, n);
        }
    }
    boundary::signed(best, q, inside)
}

/// The exact signed distance from `p` to an axis-aligned box, and the outward normal.
fn box_distance(min: [f64; 3], max: [f64; 3], p: [f64; 3]) -> (f64, [f64; 3]) {
    let c: [f64; 3] = std::array::from_fn(|k| 0.5 * (min[k] + max[k]));
    let half: [f64; 3] = std::array::from_fn(|k| 0.5 * (max[k] - min[k]));
    let d: [f64; 3] = std::array::from_fn(|k| p[k] - c[k]);
    let q: [f64; 3] = std::array::from_fn(|k| d[k].abs() - half[k]);
    if q.iter().any(|v| *v > 0.0) {
        let v: [f64; 3] = q.map(|x| x.max(0.0));
        let l = norm(v);
        (l, std::array::from_fn(|k| d[k].signum() * v[k] / l))
    } else {
        let k = (0..3).max_by(|&i, &j| q[i].total_cmp(&q[j])).unwrap_or(0);
        let mut n = [0.0; 3];
        n[k] = if d[k] < 0.0 { -1.0 } else { 1.0 };
        (q[k], n)
    }
}

/// Two operands' locations combined by `op`.
fn combine(op: Operation, a: Location, b: Location) -> Location {
    use Location::{Boundary, Inside, Outside};
    let not = |l: Location| match l {
        Inside => Outside,
        Outside => Inside,
        Boundary => Boundary,
    };
    let and = |a: Location, b: Location| match (a, b) {
        (Outside, _) | (_, Outside) => Outside,
        (Inside, Inside) => Inside,
        _ => Boundary,
    };
    let or = |a: Location, b: Location| not(and(not(a), not(b)));
    match op {
        Operation::Union => or(a, b),
        Operation::Intersection => and(a, b),
        Operation::Difference => and(a, not(b)),
        Operation::Xor => match (a, b) {
            (Boundary, _) | (_, Boundary) => Boundary,
            (x, y) if x == y => Outside,
            _ => Inside,
        },
    }
}

/// Two operands' signed distances and normals composed by `op`: min for a union, max for an
/// intersection, a difference the intersection with the complement, a xor the union less the
/// intersection.
pub(crate) fn compose(op: Operation, a: (f64, [f64; 3]), b: (f64, [f64; 3])) -> (f64, [f64; 3]) {
    let min = |x: (f64, [f64; 3]), y: (f64, [f64; 3])| if y.0 < x.0 { y } else { x };
    let max = |x: (f64, [f64; 3]), y: (f64, [f64; 3])| if y.0 > x.0 { y } else { x };
    let not = |x: (f64, [f64; 3])| (-x.0, x.1.map(|v| -v));
    match op {
        Operation::Union => min(a, b),
        Operation::Intersection => max(a, b),
        Operation::Difference => max(a, not(b)),
        Operation::Xor => max(min(a, b), not(max(a, b))),
    }
}

/// A planar shape's signed distance from (x, y), negative inside, and its outward normal at the
/// nearest point.
pub(crate) fn planar(shape: &Shape, x: f64, y: f64) -> (f64, [f64; 2]) {
    let radial = |center: &Point, x: f64, y: f64| {
        let (dx, dy) = (x - center.x.to_um(), y - center.y.to_um());
        let r = dx.hypot(dy);
        let n = if r > 0.0 {
            [dx / r, dy / r]
        } else {
            [1.0, 0.0]
        };
        (r, n)
    };
    match shape {
        Shape::Rect {
            center,
            width,
            height,
        } => {
            let (dx, dy) = (x - center.x.to_um(), y - center.y.to_um());
            let q = [
                dx.abs() - width.to_um() / 2.0,
                dy.abs() - height.to_um() / 2.0,
            ];
            let sign = [dx.signum(), dy.signum()];
            if q[0] > 0.0 || q[1] > 0.0 {
                let v = [q[0].max(0.0), q[1].max(0.0)];
                let d = v[0].hypot(v[1]);
                (d, [sign[0] * v[0] / d, sign[1] * v[1] / d])
            } else if q[0] > q[1] {
                (q[0], [sign[0], 0.0])
            } else {
                (q[1], [0.0, sign[1]])
            }
        }
        Shape::Circle { center, radius } => {
            let (r, n) = radial(center, x, y);
            (r - radius.to_um(), n)
        }
        Shape::Ring {
            center,
            inner,
            outer,
        } => {
            let (r, n) = radial(center, x, y);
            let (out, inn) = (r - outer.to_um(), inner.to_um() - r);
            if out >= inn {
                (out, n)
            } else {
                (inn, [-n[0], -n[1]])
            }
        }
        Shape::Polygon(polygon) => {
            let v = polygon.vertices();
            let mut best = (f64::INFINITY, [0.0, 0.0], [1.0, 0.0]);
            for i in 0..v.len() {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                let (ax, ay, bx, by) = (a.x.to_um(), a.y.to_um(), b.x.to_um(), b.y.to_um());
                let (ex, ey) = (bx - ax, by - ay);
                let t = (((x - ax) * ex + (y - ay) * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
                let nearest = [ax + t * ex, ay + t * ey];
                let d = (x - nearest[0]).hypot(y - nearest[1]);
                if d < best.0 {
                    // counterclockwise: the edge's outward normal is (e_y, −e_x)
                    let l = ex.hypot(ey);
                    best = (d, nearest, [ey / l, -ex / l]);
                }
            }
            let (d, nearest, edge) = best;
            let inside = polygon.contains(Point::um(x, y));
            let n = if d > 0.0 {
                let away = [(x - nearest[0]) / d, (y - nearest[1]) / d];
                if inside { [-away[0], -away[1]] } else { away }
            } else {
                edge
            };
            (if inside { -d } else { d }, n)
        }
        // the other primitives answer for themselves
        other => {
            let d = Region::distance(other, Point::um(x, y));
            (d.signed.to_um(), d.normal)
        }
    }
}

/// The exact signed distance from `p` to an ellipsoid's surface (with infinite semi-axes, an
/// elliptic cylinder's or a slab's), and the outward normal.
fn ellipsoid_distance(
    center: [f64; 3],
    inverse: [f64; 3],
    axes: &[[f64; 3]; 3],
    p: [f64; 3],
) -> (f64, [f64; 3]) {
    let q = sub(p, center);
    let u: [f64; 3] = std::array::from_fn(|i| dot(q, axes[i]));
    let finite: Vec<usize> = (0..3).filter(|&i| inverse[i] > 0.0).collect();
    let inside = (0..3).map(|i| (u[i] * inverse[i]).powi(2)).sum::<f64>() < 1.0;
    // the nearest point in the ellipsoid's frame, along the finite axes; the infinite ones
    // keep the point's own coordinates
    let mut x = u;
    match finite[..] {
        [i] => x[i] = (1.0 / inverse[i]).copysign(u[i]),
        [i, j] => {
            let n = boundary::ellipse_nearest(1.0 / inverse[i], 1.0 / inverse[j], [u[i], u[j]]);
            x[i] = n[0];
            x[j] = n[1];
        }
        _ => {
            let e = inverse.map(|v| 1.0 / v);
            x = ellipsoid_nearest(e, u);
        }
    }
    let gap = sub(u, x);
    let d = norm(gap);
    let local = if d > 0.0 {
        let s = if inside { -1.0 / d } else { 1.0 / d };
        scale(gap, s)
    } else {
        // on the surface: the gradient's direction
        let g: [f64; 3] = std::array::from_fn(|i| x[i] * inverse[i] * inverse[i]);
        let l = norm(g);
        if l > 0.0 {
            scale(g, 1.0 / l)
        } else {
            [1.0, 0.0, 0.0]
        }
    };
    let n: [f64; 3] = std::array::from_fn(|k| (0..3).map(|i| local[i] * axes[i][k]).sum());
    (if inside { -d } else { d }, n)
}

/// The point of the ellipsoid Σ (xᵢ/eᵢ)² = 1 nearest to `y`, in its frame: the Lagrange
/// condition xᵢ = eᵢ² yᵢ/(t + eᵢ²) leaves one equation, Σ (eᵢ yᵢ/(t + eᵢ²))² = 1, monotone in t
/// where it has its root, solved by bisection to the last bit; a point on a principal plane
/// takes the plane's ellipse unless its nearest point is off the plane.
fn ellipsoid_nearest(e: [f64; 3], y: [f64; 3]) -> [f64; 3] {
    // the axes from the longest, the point in the first octant; both restored at the end
    let mut order = [0usize, 1, 2];
    order.sort_by(|&a, &b| e[b].total_cmp(&e[a]));
    let es = order.map(|i| e[i]);
    let ys = order.map(|i| y[i].abs());
    let xs = octant_nearest(es, ys);
    let mut x = [0.0; 3];
    for (k, &i) in order.iter().enumerate() {
        x[i] = xs[k].copysign(y[i]);
    }
    x
}

/// [`ellipsoid_nearest`] with e₀ ≥ e₁ ≥ e₂ and every yᵢ ≥ 0.
fn octant_nearest(e: [f64; 3], y: [f64; 3]) -> [f64; 3] {
    let plane =
        |a: usize, b: usize| -> [f64; 2] { boundary::ellipse_nearest(e[a], e[b], [y[a], y[b]]) };
    if y[2] > 0.0 {
        if y[1] > 0.0 {
            if y[0] > 0.0 {
                let z = [y[0] / e[0], y[1] / e[1], y[2] / e[2]];
                let g = z[0] * z[0] + z[1] * z[1] + z[2] * z[2] - 1.0;
                if g == 0.0 {
                    return y;
                }
                let r = [(e[0] / e[2]).powi(2), (e[1] / e[2]).powi(2)];
                let n = [r[0] * z[0], r[1] * z[1]];
                // G(s) = Σ (nᵢ/(s + rᵢ))² + (z₂/(s + 1))² − 1 decreases; bracket its root
                let mut s0 = z[2] - 1.0;
                let mut s1 = if g < 0.0 {
                    0.0
                } else {
                    norm([n[0], n[1], z[2]]) - 1.0
                };
                let mut s = s0;
                for _ in 0..1100 {
                    s = 0.5 * (s0 + s1);
                    if s == s0 || s == s1 {
                        break;
                    }
                    let a = n[0] / (s + r[0]);
                    let b = n[1] / (s + r[1]);
                    let c = z[2] / (s + 1.0);
                    let gs = a * a + b * b + c * c - 1.0;
                    if gs > 0.0 {
                        s0 = s;
                    } else if gs < 0.0 {
                        s1 = s;
                    } else {
                        break;
                    }
                }
                [
                    r[0] * y[0] / (s + r[0]),
                    r[1] * y[1] / (s + r[1]),
                    y[2] / (s + 1.0),
                ]
            } else {
                let q = plane(1, 2);
                [0.0, q[0], q[1]]
            }
        } else if y[0] > 0.0 {
            let q = plane(0, 2);
            [q[0], 0.0, q[1]]
        } else {
            [0.0, 0.0, e[2]]
        }
    } else {
        // on the plane of the shortest axis: off it when the point is near enough the centre
        let denom = [e[0] * e[0] - e[2] * e[2], e[1] * e[1] - e[2] * e[2]];
        let numer = [e[0] * y[0], e[1] * y[1]];
        if numer[0] < denom[0] && numer[1] < denom[1] {
            let xd = [numer[0] / denom[0], numer[1] / denom[1]];
            let discr = 1.0 - xd[0] * xd[0] - xd[1] * xd[1];
            if discr > 0.0 {
                return [e[0] * xd[0], e[1] * xd[1], e[2] * discr.sqrt()];
            }
        }
        let q = plane(0, 1);
        [q[0], q[1], 0.0]
    }
}

pub(crate) fn orthonormal(axes: &[[f64; 3]; 3]) -> bool {
    (0..3).all(|i| {
        (0..3).all(|j| {
            let expected = if i == j { 1.0 } else { 0.0 };
            (dot(axes[i], axes[j]) - expected).abs() <= 1e-9
        })
    })
}

pub(crate) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(crate) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(crate) fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub(crate) fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub(crate) fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

pub(crate) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
