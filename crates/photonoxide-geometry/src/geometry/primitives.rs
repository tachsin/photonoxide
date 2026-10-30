//! The primitives beyond the rectangle, the circle, the ring and the polygon: an ellipse, an
//! annular sector, a regular polygon, a superellipse and a rectangle with rounded corners. Each
//! keeps its exact definition and an orientation, so that a similarity transform maps it to one
//! of its own kind ([`super::Transform`]).

use std::f64::consts::{PI, TAU};
use std::sync::OnceLock;

use super::boundary::{self, Conic, P, Piece};
use super::region::{Distance, Location, Polygons, Region, Tolerance, fill_of, polygons_of};
use super::{Bounds, Point, Polygon};
use crate::units::Length;
use crate::{Error, Result};

pub(crate) fn um(p: Point) -> P {
    [p.x.to_um(), p.y.to_um()]
}

pub(crate) fn point(p: P) -> Point {
    Point::um(p[0], p[1])
}

fn finite(v: &[f64]) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// A frame: a centre and the direction of the first axis, as (cos, sin).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    c: P,
    cos: f64,
    sin: f64,
}

impl Frame {
    fn new(c: P, angle: f64) -> Frame {
        let (sin, cos) = super::transform::sin_cos(angle);
        Frame { c, cos, sin }
    }

    /// `p` in the frame's own coordinates.
    fn local(&self, p: P) -> P {
        let d = [p[0] - self.c[0], p[1] - self.c[1]];
        [
            self.cos * d[0] + self.sin * d[1],
            -self.sin * d[0] + self.cos * d[1],
        ]
    }

    /// A point of the frame in the plane's coordinates.
    fn global(&self, q: P) -> P {
        [
            self.c[0] + self.cos * q[0] - self.sin * q[1],
            self.c[1] + self.sin * q[0] + self.cos * q[1],
        ]
    }

    /// A direction of the frame in the plane's.
    fn turn(&self, v: P) -> P {
        [
            self.cos * v[0] - self.sin * v[1],
            self.sin * v[0] + self.cos * v[1],
        ]
    }

    /// The half-widths along x and y of a centred shape whose support function in the frame
    /// is `h`.
    fn extents(&self, h: impl Fn(P) -> f64) -> Bounds {
        let ex = h([self.cos, -self.sin]);
        let ey = h([self.sin, self.cos]);
        Bounds {
            min: Point::um(self.c[0] - ex, self.c[1] - ey),
            max: Point::um(self.c[0] + ex, self.c[1] + ey),
        }
    }
}

fn normalize(v: P) -> P {
    let l = v[0].hypot(v[1]);
    if l > 0.0 {
        [v[0] / l, v[1] / l]
    } else {
        [1.0, 0.0]
    }
}

/// An ellipse: its centre, its semi-axes and the angle of the first from x.
#[derive(Clone, Debug, PartialEq)]
pub struct Ellipse {
    frame: Frame,
    angle: f64,
    a: f64,
    b: f64,
}

impl Ellipse {
    /// The ellipse about `center` with semi-axes `semi_axes`, the first at `angle` (radians,
    /// counterclockwise) from x.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless everything is finite and both semi-axes are positive.
    pub fn new(center: Point, semi_axes: [Length; 2], angle: f64) -> Result<Ellipse> {
        let (a, b) = (semi_axes[0].to_um(), semi_axes[1].to_um());
        let c = um(center);
        if !(finite(&[c[0], c[1], a, b, angle]) && a > 0.0 && b > 0.0) {
            return Err(Error::invalid(
                "ellipse",
                format!(
                    "needs a finite centre and angle and positive semi-axes, got {} and {}",
                    semi_axes[0], semi_axes[1]
                ),
            ));
        }
        Ok(Ellipse {
            frame: Frame::new(c, angle),
            angle,
            a,
            b,
        })
    }

    /// The centre.
    pub fn center(&self) -> Point {
        point(self.frame.c)
    }

    /// The semi-axes, the first along [`Ellipse::angle`].
    pub fn semi_axes(&self) -> [Length; 2] {
        [Length::um(self.a), Length::um(self.b)]
    }

    /// The first semi-axis's angle from x, radians.
    pub fn angle(&self) -> f64 {
        self.angle
    }

    pub(crate) fn loops(&self) -> Vec<Vec<Piece>> {
        vec![vec![Piece::Conic(Conic {
            c: self.frame.c,
            rx: self.a,
            ry: self.b,
            cos: self.frame.cos,
            sin: self.frame.sin,
            t0: 0.0,
            t1: TAU,
        })]]
    }
}

impl Region for Ellipse {
    fn locate(&self, p: Point) -> Location {
        let q = self.frame.local(um(p));
        let f = (q[0] / self.a).powi(2) + (q[1] / self.b).powi(2);
        Location::of(f.total_cmp(&1.0))
    }

    fn distance(&self, p: Point) -> Distance {
        let pp = um(p);
        let q = self.frame.local(pp);
        let near = boundary::ellipse_nearest(self.a, self.b, q);
        let n = normalize([near[0] / (self.a * self.a), near[1] / (self.b * self.b)]);
        let g = self.frame.global(near);
        let d = (pp[0] - g[0]).hypot(pp[1] - g[1]);
        let inside = self.locate(p) != Location::Outside;
        Distance::from(boundary::signed((d, g, self.frame.turn(n)), pp, inside))
    }

    fn bounds(&self) -> Bounds {
        self.frame.extents(|u| (self.a * u[0]).hypot(self.b * u[1]))
    }

    fn polygons(&self, tolerance: Tolerance) -> Polygons {
        polygons_of(&self.loops(), tolerance)
    }

    fn area(&self) -> f64 {
        PI * self.a * self.b
    }

    /// Exact: Gauss's arithmetic-geometric mean, 2π (a² − Σ 2ⁿ⁻¹ cₙ²) / M(a, b).
    fn perimeter(&self) -> Option<Length> {
        let (mut a, mut b) = (self.a.max(self.b), self.a.min(self.b));
        let mut sum = 0.5 * (a * a - b * b);
        let mut power = 0.5;
        let start = a * a;
        for _ in 0..64 {
            let c = 0.5 * (a - b);
            if c == 0.0 {
                break;
            }
            let (next_a, next_b) = (0.5 * (a + b), (a * b).sqrt());
            power *= 2.0;
            sum += power * c * c;
            (a, b) = (next_a, next_b);
        }
        Some(Length::um(TAU * (start - sum) / a))
    }

    fn fill(&self, cell: Bounds) -> f64 {
        fill_of(&self.loops(), self.bounds(), self.area(), cell)
    }
}

/// An annular sector: the part of the ring between two radii about a centre that lies between
/// two angles. With an inner radius of zero, a pie slice.
#[derive(Clone, Debug, PartialEq)]
pub struct Sector {
    c: P,
    inner: f64,
    outer: f64,
    start: f64,
    sweep: f64,
}

impl Sector {
    /// The sector about `center` from radius `inner` to `outer`, from angle `start` counterclockwise
    /// through `sweep` (radians).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless everything is finite, 0 ≤ inner < outer and
    /// 0 < sweep < 2π (a full turn is a [`super::Shape::Ring`] or a circle).
    pub fn new(
        center: Point,
        inner: Length,
        outer: Length,
        start: f64,
        sweep: f64,
    ) -> Result<Sector> {
        let c = um(center);
        let (r0, r1) = (inner.to_um(), outer.to_um());
        if !(finite(&[c[0], c[1], r0, r1, start, sweep])
            && r0 >= 0.0
            && r1 > r0
            && sweep > 0.0
            && sweep < TAU)
        {
            return Err(Error::invalid(
                "sector",
                format!(
                    "needs a finite centre, 0 <= inner < outer and 0 < sweep < 2 pi, got radii \
                     {inner} to {outer} and a sweep of {sweep} rad"
                ),
            ));
        }
        Ok(Sector {
            c,
            inner: r0,
            outer: r1,
            start,
            sweep,
        })
    }

    /// The centre.
    pub fn center(&self) -> Point {
        point(self.c)
    }

    /// The inner and outer radii.
    pub fn radii(&self) -> [Length; 2] {
        [Length::um(self.inner), Length::um(self.outer)]
    }

    /// The start angle and the sweep, radians.
    pub fn angles(&self) -> [f64; 2] {
        [self.start, self.sweep]
    }

    fn arc(&self, r: f64, t0: f64, t1: f64) -> Piece {
        Piece::Conic(Conic {
            c: self.c,
            rx: r,
            ry: r,
            cos: 1.0,
            sin: 0.0,
            t0,
            t1,
        })
    }

    fn at(&self, r: f64, t: f64) -> P {
        [self.c[0] + r * t.cos(), self.c[1] + r * t.sin()]
    }

    pub(crate) fn loops(&self) -> Vec<Vec<Piece>> {
        let (s, e) = (self.start, self.start + self.sweep);
        let outer = self.arc(self.outer, s, e);
        if self.inner == 0.0 {
            return vec![vec![
                Piece::Line(self.c, outer.start()),
                outer,
                Piece::Line(outer.end(), self.c),
            ]];
        }
        let inner = self.arc(self.inner, e, s);
        vec![vec![
            Piece::Line(inner.end(), outer.start()),
            outer,
            Piece::Line(outer.end(), inner.start()),
            inner,
        ]]
    }
}

impl Region for Sector {
    fn locate(&self, p: Point) -> Location {
        let q = {
            let p = um(p);
            [p[0] - self.c[0], p[1] - self.c[1]]
        };
        let r2 = q[0] * q[0] + q[1] * q[1];
        let (lo, hi) = (self.inner * self.inner, self.outer * self.outer);
        if r2 < lo || r2 > hi {
            return Location::Outside;
        }
        let (s0, c0) = super::transform::sin_cos(self.start);
        let (s1, c1) = super::transform::sin_cos(self.start + self.sweep);
        let a = c0 * q[1] - s0 * q[0];
        let b = q[0] * s1 - q[1] * c1;
        let within = if self.sweep <= PI {
            a >= 0.0 && b >= 0.0
        } else {
            !(a < 0.0 && b < 0.0)
        };
        if !within {
            return Location::Outside;
        }
        let on_edge = (a == 0.0 && c0 * q[0] + s0 * q[1] >= 0.0)
            || (b == 0.0 && c1 * q[0] + s1 * q[1] >= 0.0);
        if r2 == lo || r2 == hi || on_edge {
            Location::Boundary
        } else {
            Location::Inside
        }
    }

    fn distance(&self, p: Point) -> Distance {
        let inside = self.locate(p) != Location::Outside;
        Distance::from(boundary::distance(&self.loops(), um(p), inside))
    }

    fn bounds(&self) -> Bounds {
        let (s, e) = (self.start, self.start + self.sweep);
        let mut points = vec![
            self.at(self.outer, s),
            self.at(self.outer, e),
            self.at(self.inner, s),
            self.at(self.inner, e),
        ];
        // the outer arc's extremes along x and y within the sweep
        for k in 0..4 {
            let t = f64::from(k) * PI / 2.0;
            let t = t + TAU * ((s - t) / TAU).ceil();
            if t <= e {
                points.push(self.at(self.outer, t));
            }
        }
        super::bounds_um(&points)
    }

    fn polygons(&self, tolerance: Tolerance) -> Polygons {
        polygons_of(&self.loops(), tolerance)
    }

    fn area(&self) -> f64 {
        0.5 * self.sweep * (self.outer * self.outer - self.inner * self.inner)
    }

    fn perimeter(&self) -> Option<Length> {
        Some(Length::um(
            self.sweep * (self.inner + self.outer) + 2.0 * (self.outer - self.inner),
        ))
    }

    fn fill(&self, cell: Bounds) -> f64 {
        fill_of(&self.loops(), self.bounds(), self.area(), cell)
    }
}

/// A regular polygon: its centre, its circumradius, its number of sides and the angle of its
/// first vertex from x.
#[derive(Clone, Debug, PartialEq)]
pub struct RegularPolygon {
    c: P,
    radius: f64,
    sides: usize,
    angle: f64,
    polygon: Polygon,
}

impl RegularPolygon {
    /// The polygon of `sides` sides inscribed in the circle of `radius` about `center`, its
    /// first vertex at `angle` (radians) from x.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless everything is finite, the radius positive and there are
    /// 3 to 10⁶ sides.
    pub fn new(center: Point, radius: Length, sides: usize, angle: f64) -> Result<RegularPolygon> {
        let c = um(center);
        let r = radius.to_um();
        if !(finite(&[c[0], c[1], r, angle]) && r > 0.0 && (3..=1_000_000).contains(&sides)) {
            return Err(Error::invalid(
                "regular polygon",
                format!(
                    "needs a finite centre and angle, a positive radius and 3 to 1000000 sides, \
                     got radius {radius} and {sides} sides"
                ),
            ));
        }
        let vertices = (0..sides)
            .map(|k| {
                let t = angle + TAU * k as f64 / sides as f64;
                Point::um(c[0] + r * t.cos(), c[1] + r * t.sin())
            })
            .collect();
        Ok(RegularPolygon {
            c,
            radius: r,
            sides,
            angle,
            polygon: Polygon::new(vertices)?,
        })
    }

    /// The centre.
    pub fn center(&self) -> Point {
        point(self.c)
    }

    /// The circumradius.
    pub fn radius(&self) -> Length {
        Length::um(self.radius)
    }

    /// The number of sides.
    pub fn sides(&self) -> usize {
        self.sides
    }

    /// The first vertex's angle from x, radians.
    pub fn angle(&self) -> f64 {
        self.angle
    }

    /// The polygon itself.
    pub fn polygon(&self) -> &Polygon {
        &self.polygon
    }
}

impl Region for RegularPolygon {
    fn locate(&self, p: Point) -> Location {
        self.polygon.locate(p)
    }

    fn distance(&self, p: Point) -> Distance {
        self.polygon.distance(p)
    }

    fn bounds(&self) -> Bounds {
        self.polygon.bounds()
    }

    fn polygons(&self, tolerance: Tolerance) -> Polygons {
        self.polygon.polygons(tolerance)
    }

    /// Exact: n R² sin(2π/n) / 2.
    fn area(&self) -> f64 {
        let n = self.sides as f64;
        0.5 * n * self.radius * self.radius * (TAU / n).sin()
    }

    fn perimeter(&self) -> Option<Length> {
        let n = self.sides as f64;
        Some(Length::um(2.0 * n * self.radius * (PI / n).sin()))
    }

    fn fill(&self, cell: Bounds) -> f64 {
        self.polygon.fill(cell)
    }
}

/// A rectangle with rounded corners, at an angle: its centre, width (along its first axis),
/// height, corner radius and the angle of its first axis from x. A radius of zero is a
/// rectangle at an angle.
#[derive(Clone, Debug, PartialEq)]
pub struct RoundedRect {
    frame: Frame,
    angle: f64,
    w: f64,
    h: f64,
    r: f64,
}

impl RoundedRect {
    /// The rectangle `width` × `height` about `center`, its width along `angle` (radians) from x,
    /// its corners rounded to `radius`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless everything is finite, the sides positive and
    /// 0 ≤ radius ≤ min(width, height)/2.
    pub fn new(
        center: Point,
        width: Length,
        height: Length,
        radius: Length,
        angle: f64,
    ) -> Result<RoundedRect> {
        let c = um(center);
        let (w, h, r) = (width.to_um(), height.to_um(), radius.to_um());
        if !(finite(&[c[0], c[1], w, h, r, angle])
            && w > 0.0
            && h > 0.0
            && r >= 0.0
            && 2.0 * r <= w.min(h))
        {
            return Err(Error::invalid(
                "rounded rectangle",
                format!(
                    "needs a finite centre and angle, a positive size and a corner radius from 0 \
                     to half the smaller side, got {width} x {height} with radius {radius}"
                ),
            ));
        }
        Ok(RoundedRect {
            frame: Frame::new(c, angle),
            angle,
            w,
            h,
            r,
        })
    }

    /// The centre.
    pub fn center(&self) -> Point {
        point(self.frame.c)
    }

    /// The width and height.
    pub fn size(&self) -> [Length; 2] {
        [Length::um(self.w), Length::um(self.h)]
    }

    /// The corner radius.
    pub fn radius(&self) -> Length {
        Length::um(self.r)
    }

    /// The width's angle from x, radians.
    pub fn angle(&self) -> f64 {
        self.angle
    }

    pub(crate) fn loops(&self) -> Vec<Vec<Piece>> {
        let (hw, hh, r) = (self.w / 2.0, self.h / 2.0, self.r);
        let (cx, cy) = (hw - r, hh - r);
        let f = &self.frame;
        // the bottom, right, top and left sides, each followed by the corner after it
        let sides = [
            ([-cx, -hh], [cx, -hh]),
            ([hw, -cy], [hw, cy]),
            ([cx, hh], [-cx, hh]),
            ([-hw, cy], [-hw, -cy]),
        ];
        let corners = [
            ([cx, -cy], -PI / 2.0),
            ([cx, cy], 0.0),
            ([-cx, cy], PI / 2.0),
            ([-cx, -cy], PI),
        ];
        let mut pieces = Vec::with_capacity(8);
        for ((a, b), (centre, t0)) in sides.iter().zip(&corners) {
            if a != b {
                pieces.push(Piece::Line(f.global(*a), f.global(*b)));
            }
            if r > 0.0 {
                pieces.push(Piece::Conic(Conic {
                    c: f.global(*centre),
                    rx: r,
                    ry: r,
                    cos: f.cos,
                    sin: f.sin,
                    t0: *t0,
                    t1: t0 + PI / 2.0,
                }));
            }
        }
        vec![pieces]
    }
}

impl Region for RoundedRect {
    fn locate(&self, p: Point) -> Location {
        let q = self.frame.local(um(p));
        let (x, y) = (q[0].abs(), q[1].abs());
        let (hw, hh, r) = (self.w / 2.0, self.h / 2.0, self.r);
        if x > hw || y > hh {
            return Location::Outside;
        }
        let (dx, dy) = (x - (hw - r), y - (hh - r));
        if dx > 0.0 && dy > 0.0 {
            return Location::of((dx * dx + dy * dy).total_cmp(&(r * r)));
        }
        if x == hw || y == hh {
            Location::Boundary
        } else {
            Location::Inside
        }
    }

    fn distance(&self, p: Point) -> Distance {
        let inside = self.locate(p) != Location::Outside;
        Distance::from(boundary::distance(&self.loops(), um(p), inside))
    }

    fn bounds(&self) -> Bounds {
        let (a, b) = (self.w / 2.0 - self.r, self.h / 2.0 - self.r);
        self.frame
            .extents(|u| a * u[0].abs() + b * u[1].abs() + self.r)
    }

    fn polygons(&self, tolerance: Tolerance) -> Polygons {
        polygons_of(&self.loops(), tolerance)
    }

    /// Exact: w h − (4 − π) r².
    fn area(&self) -> f64 {
        self.w * self.h - (4.0 - PI) * self.r * self.r
    }

    fn perimeter(&self) -> Option<Length> {
        Some(Length::um(
            2.0 * (self.w + self.h) - 8.0 * self.r + TAU * self.r,
        ))
    }

    fn fill(&self, cell: Bounds) -> f64 {
        fill_of(&self.loops(), self.bounds(), self.area(), cell)
    }
}

/// A superellipse, |x/a|ⁿ + |y/b|ⁿ ≤ 1 in its own frame: n = 2 is an ellipse, n = 1 a rhombus,
/// and as n grows it tends to the rectangle 2a × 2b.
#[derive(Clone, Debug, PartialEq)]
pub struct Superellipse {
    frame: Frame,
    angle: f64,
    a: f64,
    b: f64,
    n: f64,
    /// The polygon [`Region::fill`] clips, made once: within [`FILL_TOLERANCE`] of the larger
    /// semi-axis.
    fine: Fine,
}

/// The polygon a superellipse's [`Region::fill`] clips strays from it by at most this much of
/// its larger semi-axis.
pub const FILL_TOLERANCE: f64 = 1e-7;

/// A cached polygon, which every copy of a shape agrees on: equal whether made or not.
#[derive(Clone, Debug, Default)]
struct Fine(OnceLock<Polygon>);

impl PartialEq for Fine {
    fn eq(&self, _: &Fine) -> bool {
        true
    }
}

impl Superellipse {
    /// The superellipse about `center` with semi-axes `semi_axes`, the first at `angle`
    /// (radians) from x, and exponent `exponent`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless everything is finite, the semi-axes positive and the
    /// exponent from 1 to 100.
    pub fn new(
        center: Point,
        semi_axes: [Length; 2],
        exponent: f64,
        angle: f64,
    ) -> Result<Superellipse> {
        let c = um(center);
        let (a, b) = (semi_axes[0].to_um(), semi_axes[1].to_um());
        if !(finite(&[c[0], c[1], a, b, angle, exponent])
            && a > 0.0
            && b > 0.0
            && (1.0..=100.0).contains(&exponent))
        {
            return Err(Error::invalid(
                "superellipse",
                format!(
                    "needs a finite centre and angle, positive semi-axes and an exponent from 1 \
                     to 100, got {} and {} with exponent {exponent}",
                    semi_axes[0], semi_axes[1]
                ),
            ));
        }
        Ok(Superellipse {
            frame: Frame::new(c, angle),
            angle,
            a,
            b,
            n: exponent,
            fine: Fine::default(),
        })
    }

    /// The centre.
    pub fn center(&self) -> Point {
        point(self.frame.c)
    }

    /// The semi-axes.
    pub fn semi_axes(&self) -> [Length; 2] {
        [Length::um(self.a), Length::um(self.b)]
    }

    /// The exponent n.
    pub fn exponent(&self) -> f64 {
        self.n
    }

    /// The first semi-axis's angle from x, radians.
    pub fn angle(&self) -> f64 {
        self.angle
    }

    /// The point at parameter θ, in the frame: (a sgn cos θ |cos θ|^(2/n), b sgn sin θ |sin θ|^(2/n)).
    fn local_at(&self, t: f64) -> P {
        let (s, c) = t.sin_cos();
        let e = 2.0 / self.n;
        [
            self.a * c.abs().powf(e).copysign(c),
            self.b * s.abs().powf(e).copysign(s),
        ]
    }

    /// The outward normal at a point of the curve, in the frame.
    fn local_normal(&self, q: P) -> P {
        let g = |v: f64, s: f64| (v / s).abs().powf(self.n - 1.0).copysign(v) / s;
        normalize([g(q[0], self.a), g(q[1], self.b)])
    }

    fn level(&self, q: P) -> f64 {
        (q[0] / self.a).abs().powf(self.n) + (q[1] / self.b).abs().powf(self.n)
    }

    /// The vertices of a polygon within `budget` of the curve (µm), counterclockwise, with the
    /// curve's area, and how far it strays: [`Superellipse::attempt`] with chords split until the
    /// curve strays from each by at most 3/4 of the budget, tighter until the polygon keeps
    /// within it.
    fn vertices(&self, budget: f64) -> (Vec<P>, f64) {
        let mut split = 0.75;
        loop {
            let (v, dev) = self.attempt(budget, split);
            if dev <= budget || split < 1e-3 {
                return (v, dev);
            }
            split *= 0.5;
        }
    }

    /// Chords are halved until the curve strays from each by at most `split` times the budget
    /// (measured at 7 points of each). Each vertex then moves out along the normal by λ times
    /// the mean of its two chords' signed sagittas, λ (about 2/3) chosen so that the polygon's
    /// area is the curve's, in closed form, exactly: a quadratic in λ. The distance from the
    /// curve is measured again at 8 points of each chord.
    fn attempt(&self, budget: f64, split: f64) -> (Vec<P>, f64) {
        let chord_sag = |t0: f64, t1: f64| -> f64 {
            let (a, b) = (self.local_at(t0), self.local_at(t1));
            let e = [b[0] - a[0], b[1] - a[1]];
            let l = e[0].hypot(e[1]);
            let mut worst = 0.0f64;
            for k in 1..8 {
                let s = self.local_at(t0 + (t1 - t0) * f64::from(k) / 8.0);
                // outward (to the right of the chord) positive
                let d = -(e[0] * (s[1] - a[1]) - e[1] * (s[0] - a[0])) / l;
                if d.abs() > worst.abs() {
                    worst = d;
                }
            }
            worst
        };
        let mut ts: Vec<f64> = Vec::new();
        let mut sags: Vec<f64> = Vec::new();
        let mut stack: Vec<(f64, f64, u32)> = (0..64)
            .rev()
            .map(|k| (TAU * f64::from(k) / 64.0, TAU * f64::from(k + 1) / 64.0, 0))
            .collect();
        while let Some((t0, t1, depth)) = stack.pop() {
            let sag = chord_sag(t0, t1);
            if sag.abs() > split * budget && depth < 40 {
                let mid = 0.5 * (t0 + t1);
                stack.push((mid, t1, depth + 1));
                stack.push((t0, mid, depth + 1));
            } else {
                ts.push(t0);
                sags.push(sag);
            }
        }
        let m = ts.len();
        let on: Vec<P> = ts.iter().map(|t| self.local_at(*t)).collect();
        let out: Vec<P> = (0..m)
            .map(|k| {
                let w = 0.5 * (sags[(k + m - 1) % m] + sags[k]);
                let n = self.local_normal(on[k]);
                [w * n[0], w * n[1]]
            })
            .collect();
        // the shoelace area of on + λ out: a0 + a1 λ + a2 λ²
        let cross = |a: P, b: P| a[0] * b[1] - a[1] * b[0];
        let (mut a0, mut a1, mut a2) = (0.0, 0.0, 0.0);
        for k in 0..m {
            let j = (k + 1) % m;
            a0 += 0.5 * cross(on[k], on[j]);
            a1 += 0.5 * (cross(on[k], out[j]) + cross(out[k], on[j]));
            a2 += 0.5 * cross(out[k], out[j]);
        }
        let deficit = self.area() - a0;
        let lambda = if a1.abs() <= f64::MIN_POSITIVE {
            0.0
        } else if a2.abs() <= 1e-300 {
            deficit / a1
        } else {
            let disc = (a1 * a1 + 4.0 * a2 * deficit).max(0.0).sqrt();
            let roots = [(-a1 + disc) / (2.0 * a2), (-a1 - disc) / (2.0 * a2)];
            if (roots[0] - 2.0 / 3.0).abs() <= (roots[1] - 2.0 / 3.0).abs() {
                roots[0]
            } else {
                roots[1]
            }
        };
        let local: Vec<P> = (0..m)
            .map(|k| [on[k][0] + lambda * out[k][0], on[k][1] + lambda * out[k][1]])
            .collect();
        // the distance from the curve, at 7 points of each chord and at the vertices
        let mut dev = 0.0f64;
        for k in 0..m {
            let (a, b) = (local[k], local[(k + 1) % m]);
            let t1 = if k + 1 == m { TAU } else { ts[k + 1] };
            let segment = Piece::Line(a, b);
            dev = dev.max(segment.nearest(self.local_at(ts[k])).0);
            for j in 1..8 {
                let s = self.local_at(ts[k] + (t1 - ts[k]) * f64::from(j) / 8.0);
                dev = dev.max(segment.nearest(s).0);
            }
        }
        (local.iter().map(|q| self.frame.global(*q)).collect(), dev)
    }

    fn fine(&self) -> &Polygon {
        self.fine.0.get_or_init(|| {
            let (v, _) = self.vertices(FILL_TOLERANCE * self.a.max(self.b));
            Polygon::new(v.into_iter().map(point).collect())
                .expect("a superellipse's polygon is a polygon")
        })
    }
}

/// Γ(x) for x ≥ 1/2 by Lanczos's approximation (g = 7, nine terms): 15 digits.
pub(crate) fn gamma(x: f64) -> f64 {
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    let x = x - 1.0;
    let mut a = C[0];
    for (i, c) in C.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    let t = x + 7.5;
    (TAU).sqrt() * t.powf(x + 0.5) * (-t).exp() * a
}

impl Region for Superellipse {
    fn locate(&self, p: Point) -> Location {
        Location::of(self.level(self.frame.local(um(p))).total_cmp(&1.0))
    }

    /// The nearest point by a search over the curve's parameter: 512 samples, then golden
    /// sections about the best to the last bit.
    fn distance(&self, p: Point) -> Distance {
        let pp = um(p);
        let q = self.frame.local(pp);
        let d2 = |t: f64| {
            let c = self.local_at(t);
            (c[0] - q[0]).powi(2) + (c[1] - q[1]).powi(2)
        };
        let m = 512;
        let step = TAU / f64::from(m);
        let best = (0..m)
            .map(|k| f64::from(k) * step)
            .min_by(|a, b| d2(*a).total_cmp(&d2(*b)))
            .unwrap_or(0.0);
        let (mut lo, mut hi) = (best - step, best + step);
        let g = 0.5 * (5f64.sqrt() - 1.0);
        let (mut x1, mut x2) = (hi - g * (hi - lo), lo + g * (hi - lo));
        let (mut f1, mut f2) = (d2(x1), d2(x2));
        for _ in 0..200 {
            if f1 <= f2 {
                hi = x2;
                (x2, f2) = (x1, f1);
                x1 = hi - g * (hi - lo);
                f1 = d2(x1);
            } else {
                lo = x1;
                (x1, f1) = (x2, f2);
                x2 = lo + g * (hi - lo);
                f2 = d2(x2);
            }
            if hi - lo <= 1e-15 * (1.0 + best.abs()) {
                break;
            }
        }
        let t = 0.5 * (lo + hi);
        let near = self.local_at(t);
        let n = self.frame.turn(self.local_normal(near));
        let g = self.frame.global(near);
        let d = (pp[0] - g[0]).hypot(pp[1] - g[1]);
        let inside = self.locate(p) != Location::Outside;
        Distance::from(boundary::signed((d, g, n), pp, inside))
    }

    /// Tight: the support function of |x/a|ⁿ + |y/b|ⁿ ≤ 1 is the dual norm,
    /// (|a u|^q + |b v|^q)^(1/q) with 1/n + 1/q = 1.
    fn bounds(&self) -> Bounds {
        self.frame.extents(|u| {
            let (x, y) = ((self.a * u[0]).abs(), (self.b * u[1]).abs());
            if self.n == 1.0 {
                x.max(y)
            } else {
                let q = self.n / (self.n - 1.0);
                let m = x.max(y);
                if m == 0.0 {
                    0.0
                } else {
                    m * ((x / m).powf(q) + (y / m).powf(q)).powf(1.0 / q)
                }
            }
        })
    }

    fn polygons(&self, tolerance: Tolerance) -> Polygons {
        let (v, dev) = self.vertices(tolerance.budget());
        super::region::assemble(vec![(v, Vec::new())], dev, tolerance)
    }

    /// Exact: 4 a b Γ(1 + 1/n)² / Γ(1 + 2/n).
    fn area(&self) -> f64 {
        let g = gamma(1.0 + 1.0 / self.n);
        4.0 * self.a * self.b * g * g / gamma(1.0 + 2.0 / self.n)
    }

    /// By quadrature to round-off: each quarter in two halves about |x/a| = |y/b|, each along
    /// the coordinate the curve is a smooth function of there, graded towards the axis.
    fn perimeter(&self) -> Option<Length> {
        let n = self.n;
        let half = |a: f64, b: f64| {
            // y from 0 to b 2^(−1/n), x = a (1 − (y/b)ⁿ)^(1/n); y = y* u⁴
            let top = b * 2f64.powf(-1.0 / n);
            boundary::integrate(
                |u: f64| {
                    let y = top * u.powi(4);
                    let dy = 4.0 * top * u.powi(3);
                    let s = y / b;
                    let slope = (a / b) * s.powf(n - 1.0) * (1.0 - s.powf(n)).powf(1.0 / n - 1.0);
                    (1.0 + slope * slope).sqrt() * dy
                },
                0.0,
                1.0,
            )
        };
        Some(Length::um(
            4.0 * (half(self.a, self.b) + half(self.b, self.a)),
        ))
    }

    /// The area of a polygon within [`FILL_TOLERANCE`] of the larger semi-axis of the curve,
    /// inside the cell: in error by at most that times the curve's length within the cell.
    fn fill(&self, cell: Bounds) -> f64 {
        let b = self.bounds();
        if !b.overlaps(&cell) {
            return 0.0;
        }
        if cell.encloses(&b) {
            return self.area();
        }
        self.fine().fill(cell)
    }
}
