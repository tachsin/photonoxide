//! Shapes defined by functions: the region between two curves, a polar region and a closed
//! parametric curve, each given by expressions ([`crate::expr`]) and traced by [`super::trace`].
//!
//! Each keeps its definition in a frame of its own and a similarity [`Transform`] from that
//! frame to the plane, so that a transformed one is one of its own kind, exactly: its inside
//! test maps the point back to its frame and asks the definition, and its boundary is the
//! definition's, mapped.

use std::f64::consts::TAU;
use std::sync::Arc;

use super::boundary::{self, P};
use super::primitives::{point, um};
use super::region::{Distance, Location, Polygons, Region, Tolerance};
use super::trace::{Curves, Span, Trace};
use super::transform::Transform;
use super::{Bounds, Point};
use crate::expr::{COORDINATES, Dim, Dual, Expr, Parameters};
use crate::units::Length;
use crate::{Error, Result};

/// The slots of the coordinates x, y and t in an expression's values.
pub(crate) const SLOT_X: usize = 0;
pub(crate) const SLOT_Y: usize = 1;
pub(crate) const SLOT_T: usize = 3;

/// How many points along its parameter a definition is checked at, besides its intervals' knots.
const SAMPLES: u32 = 2048;

/// An expression with the parameters' values, evaluated with some coordinates set.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Function {
    e: Expr,
    values: Vec<f64>,
}

impl Function {
    pub(crate) fn new(e: &Expr, parameters: &Parameters) -> Function {
        Function {
            e: e.clone(),
            values: parameters.values().to_vec(),
        }
    }

    pub(crate) fn expr(&self) -> &Expr {
        &self.e
    }

    /// The value with the coordinates `set`, (slot, value) pairs.
    pub(crate) fn value(&self, set: &[(usize, f64)]) -> f64 {
        let mut buf = [0.0f64; 64];
        let mut long;
        let values: &mut [f64] = if self.values.len() <= 64 {
            &mut buf[..self.values.len()]
        } else {
            long = vec![0.0; self.values.len()];
            &mut long
        };
        values.copy_from_slice(&self.values);
        for &(slot, v) in set {
            values[slot] = v;
        }
        self.e.eval(values)
    }

    /// The value and its derivative with respect to the coordinate in slot `seed`, the
    /// coordinates `set`.
    pub(crate) fn dual(&self, set: &[(usize, f64)], seed: usize) -> Dual {
        let mut buf = [Dual::default(); 64];
        let mut long;
        let values: &mut [Dual] = if self.values.len() <= 64 {
            &mut buf[..self.values.len()]
        } else {
            long = vec![Dual::default(); self.values.len()];
            &mut long
        };
        for (d, v) in values.iter_mut().zip(&self.values) {
            *d = Dual::constant(*v);
        }
        for &(slot, v) in set {
            values[slot] = if slot == seed {
                Dual::variable(v)
            } else {
                Dual::constant(v)
            };
        }
        self.e.eval_dual(values)
    }
}

/// Checks that `e` reads no coordinate but those in `allowed`, and is of dimension `dim` (or a
/// bare zero; `None`: any).
pub(crate) fn check_expr(
    kind: &'static str,
    e: &Expr,
    allowed: &[usize],
    dim: Option<Dim>,
    what: &str,
) -> Result<()> {
    if let Some(&slot) = e
        .slots()
        .iter()
        .find(|&&s| s < COORDINATES.len() && !allowed.contains(&s))
    {
        let names: Vec<&str> = allowed.iter().map(|&s| COORDINATES[s].0).collect();
        return Err(Error::invalid(
            kind,
            format!(
                "{what} \"{}\" reads {}, which it has no value of: only {}",
                e.text(),
                COORDINATES[slot].0,
                names.join(" and ")
            ),
        ));
    }
    match (dim, e.dim()) {
        (None, _) | (_, None) => Ok(()),
        (Some(want), Some(d)) if d == want => Ok(()),
        (Some(want), Some(d)) => Err(Error::invalid(
            kind,
            format!("{what} \"{}\" needs {want}, got {d}", e.text()),
        )),
    }
}

/// A frame: the map from a definition's own coordinates to the plane, and back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Frame {
    pub to: Transform,
    pub from: Transform,
}

impl Frame {
    pub(crate) fn new(to: Transform) -> Frame {
        Frame {
            to,
            from: to.inverse(),
        }
    }

    /// A point and a derivative of the definition, in the plane.
    pub(crate) fn out(&self, (p, d): (P, P)) -> (P, P) {
        (self.to.map_um(p), self.to.linear(d))
    }

    /// The frame, then `t`.
    pub(crate) fn then(&self, t: &Transform) -> Frame {
        Frame::new(self.to.then(t))
    }

    /// Whether it mirrors: a loop counterclockwise in the definition's frame is clockwise in
    /// the plane, and is walked the other way.
    pub(crate) fn mirrored(&self) -> bool {
        self.to.mirrored()
    }
}

/// A loop of spans walked the other way round: the spans in reverse, each from its end to its
/// start, the corner at each new start the one at the old end.
pub(crate) fn reversed(spans: &[Span]) -> Vec<Span> {
    let n = spans.len();
    (0..n)
        .rev()
        .map(|i| Span {
            curve: spans[i].curve,
            u0: spans[i].u1,
            u1: spans[i].u0,
            corner: spans[(i + 1) % n].corner,
            pieces: spans[i].pieces,
        })
        .collect()
}

/// A traced region's answers, from its definition (`curves`), its inside test and its trace.
macro_rules! traced_region {
    ($ty:ty, $curves:ident) => {
        impl Region for $ty {
            fn locate(&self, p: Point) -> Location {
                self.data.locate(um(p))
            }

            fn distance(&self, p: Point) -> Distance {
                let q = um(p);
                let inside = self.data.locate(q) != Location::Outside;
                Distance::from(boundary::signed(
                    self.data.trace.nearest(&self.data.$curves, q),
                    q,
                    inside,
                ))
            }

            fn bounds(&self) -> Bounds {
                self.data.trace.bounds()
            }

            fn polygons(&self, tolerance: Tolerance) -> Polygons {
                self.data.trace.polygons(&self.data.$curves, tolerance)
            }

            /// ∮ x dy by Gauss–Legendre on each interval of the boundary: to round-off for
            /// curves smooth between their knots.
            fn area(&self) -> f64 {
                self.data.trace.area()
            }

            fn perimeter(&self) -> Option<Length> {
                Some(Length::um(self.data.trace.perimeter(&self.data.$curves)))
            }

            /// The polygons within 10⁻⁷ of the larger side ([`super::FILL_TOLERANCE`]), keeping
            /// the area, clipped to the cell: in error by at most that times the boundary's
            /// length inside it.
            fn fill(&self, cell: Bounds) -> f64 {
                self.data.trace.fill(&self.data.$curves, cell)
            }
        }
    };
}
pub(crate) use traced_region;

/// A region and its trace.
#[derive(Debug)]
struct Data<S> {
    spec: S,
    trace: Trace,
}

// ---------------------------------------------------------------------------------------------
// Between two curves

/// The region between two curves, y_low(x) ≤ y ≤ y_high(x) for x from x₀ to x₁, each an
/// expression of `x` (and of parameters), a length: tapers, Y-branches, adiabatic couplers.
/// The curves may meet at the ends (a pointed end) but not between them.
///
/// Its inside test is the definition itself: x₀ ≤ x ≤ x₁ and the two curves compared with y at
/// x. Cheap to clone.
#[derive(Clone, Debug)]
pub struct Between {
    data: Arc<Data<BetweenSpec>>,
}

#[derive(Clone, Debug, PartialEq)]
struct BetweenSpec {
    lower: Function,
    upper: Function,
    x: [f64; 2],
    frame: Frame,
}

impl PartialEq for Between {
    fn eq(&self, other: &Between) -> bool {
        self.data.spec == other.data.spec
    }
}

impl BetweenSpec {
    fn low(&self, x: f64) -> f64 {
        self.lower.value(&[(SLOT_X, x)])
    }

    fn high(&self, x: f64) -> f64 {
        self.upper.value(&[(SLOT_X, x)])
    }
}

impl Curves for BetweenSpec {
    /// 0: the lower curve, x from x₀ to x₁; 1: the side at x₁, u = y; 2: the upper curve, x from
    /// x₁ back to x₀; 3: the side at x₀, u = y.
    fn at(&self, curve: usize, u: f64) -> (P, P) {
        let local = match curve {
            0 | 2 => {
                let f = if curve == 0 { &self.lower } else { &self.upper };
                let d = f.dual(&[(SLOT_X, u)], SLOT_X);
                ([u, d.value], [1.0, d.derivative])
            }
            1 => ([self.x[1], u], [0.0, 1.0]),
            _ => ([self.x[0], u], [0.0, 1.0]),
        };
        self.frame.out(local)
    }
}

impl Data<BetweenSpec> {
    fn locate(&self, p: P) -> Location {
        let s = &self.spec;
        let q = s.frame.from.map_um(p);
        if !(q[0] >= s.x[0] && q[0] <= s.x[1]) {
            return Location::Outside;
        }
        let (lo, hi) = (s.low(q[0]), s.high(q[0]));
        if !(q[1] >= lo && q[1] <= hi) {
            return Location::Outside;
        }
        if q[1] == lo || q[1] == hi || q[0] == s.x[0] || q[0] == s.x[1] {
            Location::Boundary
        } else {
            Location::Inside
        }
    }
}

traced_region!(Between, spec);

fn between_error(reason: impl Into<String>) -> Error {
    Error::invalid("region between two curves", reason.into())
}

impl Between {
    /// The region between `lower` and `upper`, y_low(x) ≤ y ≤ y_high(x) for x over `x` (from the
    /// smaller to the larger), the expressions parsed with `parameters`' scope and evaluated at
    /// their values.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless both expressions are lengths reading no coordinate but x,
    /// the range is finite and not empty, both curves are finite over it, and the lower is below
    /// the upper between the ends (they may meet at an end): checked at 2049 points and at every
    /// knot of the traced boundary, the error naming the first x where they meet or cross.
    pub fn new(
        lower: &Expr,
        upper: &Expr,
        x: [Length; 2],
        parameters: &Parameters,
    ) -> Result<Between> {
        const KIND: &str = "region between two curves";
        check_expr(KIND, lower, &[SLOT_X], Some(Dim::LENGTH), "the lower curve")?;
        check_expr(KIND, upper, &[SLOT_X], Some(Dim::LENGTH), "the upper curve")?;
        let (x0, x1) = (x[0].to_um(), x[1].to_um());
        if !(x0.is_finite() && x1.is_finite() && x0 < x1) {
            return Err(between_error(format!(
                "needs a finite range of x from a smaller to a larger value, got {} to {}",
                x[0], x[1]
            )));
        }
        let spec = BetweenSpec {
            lower: Function::new(lower, parameters),
            upper: Function::new(upper, parameters),
            x: [x0, x1],
            frame: Frame::new(Transform::IDENTITY),
        };
        let xs = (0..=SAMPLES).map(|k| {
            if k == SAMPLES {
                x1
            } else {
                x0 + (x1 - x0) * f64::from(k) / f64::from(SAMPLES)
            }
        });
        check_between(&spec, xs)?;
        let region = Between::build(spec)?;
        let s = &region.data.spec;
        let mut knots = region.data.trace.knots(0);
        knots.extend(region.data.trace.knots(2));
        check_between(s, knots.into_iter())?;
        Ok(region)
    }

    fn build(spec: BetweenSpec) -> Result<Between> {
        let [x0, x1] = spec.x;
        let (l0, l1, h0, h1) = (spec.low(x0), spec.low(x1), spec.high(x0), spec.high(x1));
        let spans = vec![
            Span {
                curve: 0,
                u0: x0,
                u1: x1,
                corner: true,
                pieces: 16,
            },
            Span {
                curve: 1,
                u0: l1,
                u1: h1,
                corner: true,
                pieces: 16,
            },
            Span {
                curve: 2,
                u0: x1,
                u1: x0,
                corner: true,
                pieces: 16,
            },
            Span {
                curve: 3,
                u0: h0,
                u1: l0,
                corner: true,
                pieces: 16,
            },
        ];
        let spans = if spec.frame.mirrored() {
            reversed(&spans)
        } else {
            spans
        };
        let trace = Trace::new(&spec, &[spans]).map_err(between_error)?;
        let data = Data { spec, trace };
        Ok(Between {
            data: Arc::new(data),
        })
    }

    /// The expressions of the lower and the upper curve.
    pub fn expressions(&self) -> [&Expr; 2] {
        [self.data.spec.lower.expr(), self.data.spec.upper.expr()]
    }

    /// The range of x, in the region's own frame (before any transform).
    pub fn range(&self) -> [Length; 2] {
        [
            Length::um(self.data.spec.x[0]),
            Length::um(self.data.spec.x[1]),
        ]
    }

    /// The region's image under `t`: the same definition in a frame moved by `t`.
    ///
    /// # Panics
    ///
    /// If the image's boundary can't be traced, which a similarity can't do to one that could
    /// (but for overflow).
    pub fn transformed(&self, t: &Transform) -> Between {
        let spec = BetweenSpec {
            frame: self.data.spec.frame.then(t),
            ..self.data.spec.clone()
        };
        Between::build(spec).expect("a transformed region between two curves is one")
    }
}

/// The lower curve below the upper at each of `xs` but the ends, and at most meeting it there.
fn check_between(spec: &BetweenSpec, xs: impl Iterator<Item = f64>) -> Result<()> {
    let [x0, x1] = spec.x;
    for x in xs {
        let (lo, hi) = (spec.low(x), spec.high(x));
        if !(lo.is_finite() && hi.is_finite()) {
            return Err(between_error(format!(
                "its curves aren't finite at x = {x:.6} um: lower {lo}, upper {hi}"
            )));
        }
        let end = x == x0 || x == x1;
        if lo > hi || (lo == hi && !end) {
            return Err(between_error(format!(
                "its lower curve meets or crosses the upper at x = {x:.6} um (lower {lo:.6} um, \
                 upper {hi:.6} um): it must stay below it, meeting it at most at the ends"
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Polar

/// A polar region: the points within r ≤ ρ(θ) of a centre, ρ an expression of the polar angle,
/// written `t` (radians, from 0 to 2π), and of parameters, a length: deformed microdisks,
/// limaçon and flower resonators. ρ must be positive and periodic: ρ(2π) = ρ(0).
///
/// Its inside test is the definition: the point's radius against ρ at its angle. Cheap to clone.
#[derive(Clone, Debug)]
pub struct Polar {
    data: Arc<Data<PolarSpec>>,
}

#[derive(Clone, Debug, PartialEq)]
struct PolarSpec {
    radius: Function,
    frame: Frame,
}

impl PartialEq for Polar {
    fn eq(&self, other: &Polar) -> bool {
        self.data.spec == other.data.spec
    }
}

impl Curves for PolarSpec {
    fn at(&self, _: usize, t: f64) -> (P, P) {
        let d = self.radius.dual(&[(SLOT_T, t)], SLOT_T);
        let (s, c) = t.sin_cos();
        self.frame.out((
            [d.value * c, d.value * s],
            [
                d.derivative * c - d.value * s,
                d.derivative * s + d.value * c,
            ],
        ))
    }
}

impl Data<PolarSpec> {
    fn locate(&self, p: P) -> Location {
        let q = self.spec.frame.from.map_um(p);
        let r = q[0].hypot(q[1]);
        if r == 0.0 {
            return Location::Inside;
        }
        let mut theta = q[1].atan2(q[0]);
        if theta < 0.0 {
            theta += TAU;
        }
        let rho = self.spec.radius.value(&[(SLOT_T, theta)]);
        Location::of(r.total_cmp(&rho))
    }
}

traced_region!(Polar, spec);

fn polar_error(reason: impl Into<String>) -> Error {
    Error::invalid("polar region", reason.into())
}

impl Polar {
    /// The region r ≤ ρ(θ) about `center`, ρ = `radius`, an expression of `t` (the angle θ,
    /// radians), parsed with `parameters`' scope and evaluated at their values.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `radius` is a length reading no coordinate but t, the
    /// centre is finite, and ρ is positive and finite at 2049 angles from 0 to 2π and periodic,
    /// ρ(2π) within 10⁻⁹ of ρ(0) relative to the largest ρ.
    pub fn new(center: Point, radius: &Expr, parameters: &Parameters) -> Result<Polar> {
        check_expr(
            "polar region",
            radius,
            &[SLOT_T],
            Some(Dim::LENGTH),
            "the radius",
        )?;
        let c = um(center);
        if !(c[0].is_finite() && c[1].is_finite()) {
            return Err(polar_error("needs a finite centre"));
        }
        let spec = PolarSpec {
            radius: Function::new(radius, parameters),
            frame: Frame::new(Transform::translate(center.x, center.y)),
        };
        let mut most = 0.0f64;
        for k in 0..=SAMPLES {
            let t = TAU * f64::from(k) / f64::from(SAMPLES);
            let rho = spec.radius.value(&[(SLOT_T, t)]);
            if !(rho.is_finite() && rho > 0.0) {
                return Err(polar_error(format!(
                    "its radius \"{}\" is {rho} um at t = {t:.6} rad: it must be positive",
                    radius.text()
                )));
            }
            most = most.max(rho);
        }
        let (r0, r1) = (
            spec.radius.value(&[(SLOT_T, 0.0)]),
            spec.radius.value(&[(SLOT_T, TAU)]),
        );
        if (r1 - r0).abs() > 1e-9 * most {
            return Err(polar_error(format!(
                "its radius \"{}\" isn't periodic: {r0} um at t = 0 and {r1} um at t = 2 pi, so \
                 its outline wouldn't close",
                radius.text()
            )));
        }
        let region = Polar::build(spec)?;
        for t in region.data.trace.knots(0) {
            let rho = region.data.spec.radius.value(&[(SLOT_T, t)]);
            if !(rho.is_finite() && rho > 0.0) {
                return Err(polar_error(format!(
                    "its radius \"{}\" is {rho} um at t = {t:.6} rad: it must be positive",
                    radius.text()
                )));
            }
        }
        Ok(region)
    }

    fn build(spec: PolarSpec) -> Result<Polar> {
        let corner = {
            let (a, b) = (spec.at(0, 0.0).1, spec.at(0, TAU).1);
            (a[0] * b[1] - a[1] * b[0])
                .atan2(a[0] * b[0] + a[1] * b[1])
                .abs()
                > 1e-9
        };
        let (u0, u1) = if spec.frame.mirrored() {
            (TAU, 0.0)
        } else {
            (0.0, TAU)
        };
        let span = Span {
            curve: 0,
            u0,
            u1,
            corner,
            pieces: 16,
        };
        let trace = Trace::new(&spec, &[vec![span]]).map_err(polar_error)?;
        Ok(Polar {
            data: Arc::new(Data { spec, trace }),
        })
    }

    /// The centre.
    pub fn center(&self) -> Point {
        point(self.data.spec.frame.to.map_um([0.0, 0.0]))
    }

    /// The expression ρ(t).
    pub fn expression(&self) -> &Expr {
        self.data.spec.radius.expr()
    }

    /// The region's image under `t`: the same definition in a frame moved by `t`.
    ///
    /// # Panics
    ///
    /// If the image's boundary can't be traced, which a similarity can't do to one that could
    /// (but for overflow).
    pub fn transformed(&self, t: &Transform) -> Polar {
        let spec = PolarSpec {
            frame: self.data.spec.frame.then(t),
            ..self.data.spec.clone()
        };
        Polar::build(spec).expect("a transformed polar region is one")
    }
}

// ---------------------------------------------------------------------------------------------
// A closed parametric curve

/// The region inside a closed curve (x(t), y(t)), t from 0 to 1, two expressions of `t` (and
/// of parameters), lengths: anything with a closed-form outline. The curve must close, never
/// stop (a zero derivative) and never cross or touch itself; it may run either way round.
///
/// Its inside test counts the crossings of a ray from the point with the curve itself, each
/// found by root finding to the last bit. Cheap to clone.
#[derive(Clone, Debug)]
pub struct ClosedCurve {
    data: Arc<Data<ClosedSpec>>,
}

#[derive(Clone, Debug, PartialEq)]
struct ClosedSpec {
    x: Function,
    y: Function,
    frame: Frame,
}

impl PartialEq for ClosedCurve {
    fn eq(&self, other: &ClosedCurve) -> bool {
        self.data.spec == other.data.spec
    }
}

impl Curves for ClosedSpec {
    fn at(&self, _: usize, t: f64) -> (P, P) {
        let x = self.x.dual(&[(SLOT_T, t)], SLOT_T);
        let y = self.y.dual(&[(SLOT_T, t)], SLOT_T);
        self.frame
            .out(([x.value, y.value], [x.derivative, y.derivative]))
    }
}

impl Data<ClosedSpec> {
    fn locate(&self, p: P) -> Location {
        self.trace.locate(&self.spec, p)
    }
}

traced_region!(ClosedCurve, spec);

fn closed_error(reason: impl Into<String>) -> Error {
    Error::invalid("closed curve", reason.into())
}

impl ClosedCurve {
    /// The region inside the curve (`x`(t), `y`(t)), t from 0 to 1, the expressions parsed with
    /// `parameters`' scope and evaluated at their values.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless both are lengths reading no coordinate but t, finite with
    /// a derivative that isn't zero at 2049 points and every knot, the curve closes (its end
    /// within 10⁻⁹ of its size of its start), and it doesn't cross or touch itself (checked on
    /// its polygon within 10⁻⁵ of its size).
    pub fn new(x: &Expr, y: &Expr, parameters: &Parameters) -> Result<ClosedCurve> {
        const KIND: &str = "closed curve";
        check_expr(KIND, x, &[SLOT_T], Some(Dim::LENGTH), "x")?;
        check_expr(KIND, y, &[SLOT_T], Some(Dim::LENGTH), "y")?;
        let spec = ClosedSpec {
            x: Function::new(x, parameters),
            y: Function::new(y, parameters),
            frame: Frame::new(Transform::IDENTITY),
        };
        let mut points = Vec::with_capacity(SAMPLES as usize + 1);
        for k in 0..=SAMPLES {
            let t = f64::from(k) / f64::from(SAMPLES);
            let (p, d) = spec.at(0, t);
            if !(p[0].is_finite() && p[1].is_finite() && d[0].is_finite() && d[1].is_finite())
                || d == [0.0, 0.0]
            {
                return Err(closed_error(format!(
                    "the curve ({}, {}) stops (its derivative is zero) or isn't finite at t = \
                     {t:.6}",
                    x.text(),
                    y.text()
                )));
            }
            points.push(p);
        }
        let b = super::bounds_um(&points);
        let size = (b.max.x - b.min.x).to_um().max((b.max.y - b.min.y).to_um());
        let (first, last) = (points[0], points[points.len() - 1]);
        if (first[0] - last[0]).hypot(first[1] - last[1]) > 1e-9 * size {
            return Err(closed_error(format!(
                "the curve ({}, {}) doesn't close: it starts at ({:.6}, {:.6}) um and ends at \
                 ({:.6}, {:.6}) um",
                x.text(),
                y.text(),
                first[0],
                first[1],
                last[0],
                last[1]
            )));
        }
        let curve = ClosedCurve::build(spec)?;
        curve
            .data
            .trace
            .check_simple(&curve.data.spec, 1e-5 * size)
            .map_err(closed_error)?;
        Ok(curve)
    }

    fn build(spec: ClosedSpec) -> Result<ClosedCurve> {
        let corner = {
            let (a, b) = (spec.at(0, 0.0).1, spec.at(0, 1.0).1);
            (a[0] * b[1] - a[1] * b[0])
                .atan2(a[0] * b[0] + a[1] * b[1])
                .abs()
                > 1e-9
        };
        let span = |u0: f64, u1: f64| Span {
            curve: 0,
            u0,
            u1,
            corner,
            pieces: 16,
        };
        // clockwise (by the shoelace area of 4096 of its points): walked the other way
        let n = 4096;
        let points: Vec<P> = (0..n)
            .map(|k| spec.at(0, f64::from(k) / f64::from(n)).0)
            .collect();
        let o = points[0];
        let twice: f64 = (0..n as usize)
            .map(|k| {
                let (a, b) = (points[k], points[(k + 1) % n as usize]);
                (a[0] - o[0]) * (b[1] - o[1]) - (b[0] - o[0]) * (a[1] - o[1])
            })
            .sum();
        let walk = if twice < 0.0 {
            span(1.0, 0.0)
        } else {
            span(0.0, 1.0)
        };
        let trace = Trace::new(&spec, &[vec![walk]]).map_err(closed_error)?;
        Ok(ClosedCurve {
            data: Arc::new(Data { spec, trace }),
        })
    }

    /// The expressions x(t) and y(t).
    pub fn expressions(&self) -> [&Expr; 2] {
        [self.data.spec.x.expr(), self.data.spec.y.expr()]
    }

    /// The region's image under `t`: the same definition in a frame moved by `t`.
    ///
    /// # Panics
    ///
    /// If the image's boundary can't be traced, which a similarity can't do to one that could
    /// (but for overflow).
    pub fn transformed(&self, t: &Transform) -> ClosedCurve {
        let spec = ClosedSpec {
            frame: self.data.spec.frame.then(t),
            ..self.data.spec.clone()
        };
        ClosedCurve::build(spec).expect("a transformed closed curve is one")
    }
}
