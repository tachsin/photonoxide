//! Regions made of others: booleans ([`Boolean`]), offsets ([`Offset`]) and corner rounding
//! ([`Rounded`]) of any shapes, each a [`Shape`] again.
//!
//! **What is exact.** A boolean's inside test is its operands', combined as a regularized set
//! operation (Requicha 1980): where a point is on both boundaries, which side each region is on
//! (their normals) decides, so two shapes that share an edge are one region across it (the edge
//! is inside their union), and two that only touch have no intersection there. Its signed
//! distance is the operands' composed by min and max (Ricci 1973): min for a union, max for an
//! intersection, max(d_A, −d_B) for a difference, the union of the two differences for a xor.
//! The composition has the right sign everywhere and is exact on the boundary; off it, its
//! magnitude is a lower bound of the true distance, exact wherever the nearest point it comes
//! from is on the result's boundary: always outside a union and inside an intersection, and near
//! the boundary away from where the operands' boundaries cross. Elsewhere (deep inside a union,
//! near a crossing, on an edge two operands share), the distance is to the result's polygons
//! within [`super::FILL_TOLERANCE`] of its size. A round offset, d ≤ r for the operand's signed
//! distance d, is exact in the same way: its inside test exactly, d − r exactly outside a grown
//! shape and inside a shrunk one.
//!
//! **What is from polygons.** Areas, perimeters, fills and bounds are from the result's polygons
//! within [`super::FILL_TOLERANCE`] of its size (the operands' polygons that close, combined by
//! [`super::clip`]), as are everything of a miter or square offset and of corner rounding,
//! whose corners exist only on polygons. Polygons asked for at a tolerance are the operands'
//! combined; a boolean's and a round offset's are measured against the exact distance (their
//! vertices and the middles of their edges), and the operands' polygons tightened until they
//! are within the tolerance.

use std::fmt;
use std::sync::Arc;

use super::clip::{self, Join, Operation, Part};
use super::flat::Flat;
use super::primitives::{FILL_TOLERANCE, point, um};
use super::region::{Distance, Location, Polygons, Region, Tolerance};
use super::transform::Transform;
use super::{Bounds, Point, Shape};
use crate::units::Length;
use crate::{Error, Result};

/// The larger side of a box, µm.
pub(super) fn size_of(b: &Bounds) -> f64 {
    (b.max.x - b.min.x).to_um().max((b.max.y - b.min.y).to_um())
}

fn widen(b: Bounds, by: f64) -> Bounds {
    let [lo, hi] = b.corners();
    Bounds {
        min: point([lo[0] - by, lo[1] - by]),
        max: point([hi[0] + by, hi[1] + by]),
    }
}

pub(super) fn hull(a: &Bounds, b: &Bounds) -> Bounds {
    let ([a0, a1], [b0, b1]) = (a.corners(), b.corners());
    Bounds {
        min: point([a0[0].min(b0[0]), a0[1].min(b0[1])]),
        max: point([a1[0].max(b1[0]), a1[1].max(b1[1])]),
    }
}

fn meet(a: &Bounds, b: &Bounds) -> Bounds {
    let ([a0, a1], [b0, b1]) = (a.corners(), b.corners());
    let lo = [a0[0].max(b0[0]), a0[1].max(b0[1])];
    let hi = [a1[0].min(b1[0]).max(lo[0]), a1[1].min(b1[1]).max(lo[1])];
    Bounds {
        min: point(lo),
        max: point(hi),
    }
}

/// A shape's polygons within `tolerance` µm, unsnapped, as parts, and their deviation.
pub(super) fn fine(shape: &Shape, tolerance: f64) -> (Vec<Part>, f64) {
    let t = Tolerance::unsnapped(Length::um(tolerance)).expect("a positive tolerance");
    let p = shape.polygons(t);
    (clip::parts(&p), p.deviation.to_um())
}

/// How far the polygons `parts` stray from `region`'s boundary: the largest distance from it of
/// their vertices and of their edges' middles.
fn measured(region: &dyn Region, parts: &[Part]) -> f64 {
    let mut worst = 0.0f64;
    for ring in parts.iter().flat_map(|(o, h)| std::iter::once(o).chain(h)) {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            for q in [a, [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])]] {
                worst = worst.max(region.distance(point(q)).signed.to_um().abs());
            }
        }
    }
    worst
}

/// Whether a composed distance `d` with normal `n` at `p` is the true one: whether the point it
/// measures to, p − d n, is on `region`'s boundary, a step `step` µm along the normal either
/// side of it being inside and outside. Within a step of the boundary, where the normal is the
/// rounding's, `near` says whether p is on the result's boundary.
pub(super) fn exact(
    region: &dyn Region,
    p: P2,
    d: f64,
    n: [f64; 2],
    step: f64,
    near: impl FnOnce() -> bool,
) -> bool {
    if d.abs() <= step {
        return near();
    }
    let q = [p[0] - d * n[0], p[1] - d * n[1]];
    let step = step.max(1e-13 * (q[0].abs() + q[1].abs()));
    region.locate(point([q[0] - step * n[0], q[1] - step * n[1]])) == Location::Inside
        && region.locate(point([q[0] + step * n[0], q[1] + step * n[1]])) == Location::Outside
}

pub(super) type P2 = [f64; 2];

/// The polygons of a region at `tolerance`, from `make(e)`, the region's polygons built with
/// the operands' within `e` µm, and their own deviation: measured against the region's
/// distance, `e` cut by 4 until they are within the tolerance (at most 6 times). A tolerance
/// within 100 times `floor`, how closely the region's distance itself is known where it isn't
/// exact, isn't measured: the polygons are made once, their deviation their own.
pub(super) fn tightened(
    region: &dyn Region,
    tolerance: Tolerance,
    floor: f64,
    make: impl Fn(f64) -> (Vec<Part>, f64),
) -> Polygons {
    let geometric = tolerance.geometric().to_um();
    let mut e = 0.5 * tolerance.budget();
    if geometric <= 100.0 * floor {
        let (parts, own) = make(e);
        return clip::polygons(parts, own + clip::snapping(tolerance), tolerance);
    }
    let mut last = (Vec::new(), 0.0);
    for _ in 0..6 {
        let (parts, own) = make(e);
        let dev = measured(region, &parts).max(own);
        last = (parts, dev);
        if dev <= geometric {
            break;
        }
        e *= 0.25;
    }
    clip::polygons(last.0, last.1, tolerance)
}

// ---------------------------------------------------------------------------------------------
// Booleans

/// A boolean of two shapes: their union, intersection, difference or xor (see [`Operation`]),
/// a region of its own whose operands may be any shapes, booleans among them. Cheap to clone.
///
/// Its inside test and signed distance are its operands' composed; its area, fill and bounds
/// are from its polygons within [`super::FILL_TOLERANCE`] of its size. See the module's docs.
#[derive(Clone)]
pub struct Boolean {
    data: Arc<BooleanData>,
}

struct BooleanData {
    op: Operation,
    a: Shape,
    b: Shape,
    flat: Flat,
    /// A step along a normal far below the polygons' tolerance, µm.
    step: f64,
}

impl PartialEq for Boolean {
    fn eq(&self, other: &Boolean) -> bool {
        let (a, b) = (&self.data, &other.data);
        a.op == b.op && a.a == b.a && a.b == b.b
    }
}

impl fmt::Debug for Boolean {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Boolean")
            .field("op", &self.data.op)
            .field("a", &self.data.a)
            .field("b", &self.data.b)
            .finish()
    }
}

/// Where a point is in A op B, from where it is in A and in B and, when it is on both
/// boundaries, from the two regions' outward normals there.
fn combine(op: Operation, la: Location, lb: Location, normals: impl Fn() -> (P2, P2)) -> Location {
    let f = |a: bool, b: bool| match op {
        Operation::Union => a || b,
        Operation::Intersection => a && b,
        Operation::Difference => a && !b,
        Operation::Xor => a != b,
    };
    let of = |v: bool| {
        if v {
            Location::Inside
        } else {
            Location::Outside
        }
    };
    let sides = |l: Location| match l {
        Location::Inside => vec![true],
        Location::Outside => vec![false],
        Location::Boundary => vec![true, false],
    };
    // the memberships a point near p can have
    let cases: Vec<(bool, bool)> = match (la, lb) {
        (Location::Boundary, Location::Boundary) => {
            let (na, nb) = normals();
            let c = na[0] * nb[0] + na[1] * nb[1];
            if c < -1.0 + 1e-9 {
                // back to back: one or the other
                vec![(true, false), (false, true)]
            } else if c > 1.0 - 1e-9 {
                // the same side: both or neither
                vec![(true, true), (false, false)]
            } else {
                vec![(true, true), (true, false), (false, true), (false, false)]
            }
        }
        _ => sides(la)
            .into_iter()
            .flat_map(|a| sides(lb).into_iter().map(move |b| (a, b)))
            .collect(),
    };
    let first = f(cases[0].0, cases[0].1);
    if cases.iter().all(|&(a, b)| f(a, b) == first) {
        of(first)
    } else {
        Location::Boundary
    }
}

/// The composed signed distance and normal of A op B from the operands'.
fn compose(op: Operation, da: Distance, db: Distance) -> (f64, P2) {
    let a = (da.signed.to_um(), da.normal);
    let b = (db.signed.to_um(), db.normal);
    let neg = |(d, n): (f64, P2)| (-d, [-n[0], -n[1]]);
    let min = |x: (f64, P2), y: (f64, P2)| if y.0 < x.0 { y } else { x };
    let max = |x: (f64, P2), y: (f64, P2)| if y.0 > x.0 { y } else { x };
    match op {
        Operation::Union => min(a, b),
        Operation::Intersection => max(a, b),
        Operation::Difference => max(a, neg(b)),
        Operation::Xor => min(max(a, neg(b)), max(b, neg(a))),
    }
}

impl Boolean {
    /// `a` `op` `b`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when the result is empty (two shapes that don't meet have no
    /// intersection, a shape inside another no difference from it).
    pub fn new(op: Operation, a: Shape, b: Shape) -> Result<Boolean> {
        let (ba, bb) = (a.bounds(), b.bounds());
        let size = size_of(&hull(&ba, &bb));
        let t = FILL_TOLERANCE * size;
        let (pa, da) = fine(&a, t);
        let (pb, db) = fine(&b, t);
        let parts = clip::boolean(&pa, &pb, op, 0.0, t);
        let flat = Flat::new(parts, da.max(db)).ok_or_else(|| {
            Error::invalid(
                op.name(),
                format!(
                    "the {} is empty: the shapes leave nothing (the intersection of shapes that \
                     don't overlap, or the difference of a shape and one that covers it)",
                    op.name()
                ),
            )
        })?;
        Ok(Boolean {
            data: Arc::new(BooleanData {
                op,
                a,
                b,
                flat,
                step: 1e-3 * t,
            }),
        })
    }

    /// The operation.
    pub fn operation(&self) -> Operation {
        self.data.op
    }

    /// The two operands, A and B.
    pub fn operands(&self) -> [&Shape; 2] {
        [&self.data.a, &self.data.b]
    }

    /// The image under `t`: the operands' images combined.
    ///
    /// # Panics
    ///
    /// If the image is empty, which a transform of a region that isn't can't make.
    pub fn transformed(&self, t: &Transform) -> Boolean {
        Boolean::new(
            self.data.op,
            self.data.a.transformed(t),
            self.data.b.transformed(t),
        )
        .expect("a transformed boolean is one")
    }

    /// Its polygons within `tolerance` µm each way of its operands', unsnapped and combined.
    fn combined(&self, e: f64, grid: f64, sliver: f64) -> (Vec<Part>, f64) {
        let (pa, da) = fine(&self.data.a, e);
        let (pb, db) = fine(&self.data.b, e);
        (
            clip::boolean(&pa, &pb, self.data.op, grid, sliver),
            da.max(db),
        )
    }
}

impl Region for Boolean {
    fn locate(&self, p: Point) -> Location {
        let d = &self.data;
        let (la, lb) = (d.a.locate(p), d.b.locate(p));
        combine(d.op, la, lb, || {
            (d.a.distance(p).normal, d.b.distance(p).normal)
        })
    }

    fn distance(&self, p: Point) -> Distance {
        let data = &self.data;
        let (da, db) = (data.a.distance(p), data.b.distance(p));
        let (d, n) = compose(data.op, da, db);
        let near = || {
            // each operand's place, within a step of its boundary counted on it
            let at = |x: &Distance| {
                let s = x.signed.to_um();
                if s.abs() <= data.step {
                    Location::Boundary
                } else if s < 0.0 {
                    Location::Inside
                } else {
                    Location::Outside
                }
            };
            combine(data.op, at(&da), at(&db), || (da.normal, db.normal)) == Location::Boundary
        };
        if exact(self, um(p), d, n, data.step, near) {
            return Distance {
                signed: Length::um(d),
                normal: n,
            };
        }
        let (d, n) = data.flat.distance(p, d);
        Distance {
            signed: Length::um(d),
            normal: n,
        }
    }

    /// Its polygons' box within [`super::FILL_TOLERANCE`] of its size, widened by their
    /// distance from it, within the operands' boxes.
    fn bounds(&self) -> Bounds {
        let d = &self.data;
        let (ba, bb) = (d.a.bounds(), d.b.bounds());
        let outer = match d.op {
            Operation::Union | Operation::Xor => hull(&ba, &bb),
            Operation::Intersection => meet(&ba, &bb),
            Operation::Difference => ba,
        };
        meet(&widen(d.flat.bounds(), d.flat.deviation), &outer)
    }

    fn polygons(&self, tolerance: Tolerance) -> Polygons {
        let (grid, sliver) = (tolerance.grid().to_um(), tolerance.geometric().to_um());
        let floor = self.data.flat.deviation + self.data.step;
        tightened(self, tolerance, floor, |e| self.combined(e, grid, sliver))
    }

    /// Its polygons' within [`super::FILL_TOLERANCE`] of its size.
    fn area(&self) -> f64 {
        self.data.flat.area()
    }

    /// Its polygons' within [`super::FILL_TOLERANCE`] of its size.
    fn perimeter(&self) -> Option<Length> {
        Some(Length::um(self.data.flat.perimeter()))
    }

    /// Its polygons within [`super::FILL_TOLERANCE`] of its size, clipped to the cell.
    fn fill(&self, cell: Bounds) -> f64 {
        self.data.flat.fill(cell)
    }
}

// ---------------------------------------------------------------------------------------------
// Offsets

/// A shape grown or shrunk: every point within `distance` of it (a negative distance: every
/// point of it at least that far inside), its corners joined as [`Join`] says. For process bias
/// and cladding clearances. Cheap to clone.
///
/// A round offset is exact as its operand's distance is; a miter or square one is answered from
/// its polygons within [`super::FILL_TOLERANCE`] of its size. See the module's docs.
#[derive(Clone)]
pub struct Offset {
    data: Arc<OffsetData>,
}

struct OffsetData {
    shape: Shape,
    distance: f64,
    join: Join,
    flat: Flat,
    step: f64,
}

impl PartialEq for Offset {
    fn eq(&self, other: &Offset) -> bool {
        let (a, b) = (&self.data, &other.data);
        a.shape == b.shape && a.distance == b.distance && a.join == b.join
    }
}

impl fmt::Debug for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Offset")
            .field("shape", &self.data.shape)
            .field("distance_um", &self.data.distance)
            .field("join", &self.data.join)
            .finish()
    }
}

impl Offset {
    /// `shape` grown by `distance` (shrunk where it is negative), joined at its corners as
    /// `join` says.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the distance is finite and a miter's limit at least 1,
    /// and when shrinking leaves nothing.
    pub fn new(shape: Shape, distance: Length, join: Join) -> Result<Offset> {
        let r = distance.to_um();
        if !r.is_finite() {
            return Err(Error::invalid(
                "offset",
                format!("needs a finite distance, got {distance}"),
            ));
        }
        let join = join.checked()?;
        let size = size_of(&shape.bounds()) + 2.0 * r.max(0.0);
        let t = FILL_TOLERANCE * size;
        let (parts, dev) = fine(&shape, t);
        let out = clip::without_slivers(clip::offset(&parts, r, join, t, 0.0), t);
        let flat = Flat::new(out, dev + t).ok_or_else(|| {
            Error::invalid(
                "offset",
                format!("shrinking by {} leaves nothing of the shape", -distance),
            )
        })?;
        Ok(Offset {
            data: Arc::new(OffsetData {
                shape,
                distance: r,
                join,
                flat,
                step: 1e-3 * t,
            }),
        })
    }

    /// The shape offset.
    pub fn shape(&self) -> &Shape {
        &self.data.shape
    }

    /// The distance: positive grows, negative shrinks.
    pub fn distance(&self) -> Length {
        Length::um(self.data.distance)
    }

    /// The joins at its corners.
    pub fn join(&self) -> Join {
        self.data.join
    }

    /// The image under `t`: the shape's image offset by the distance scaled.
    ///
    /// # Panics
    ///
    /// If the image is empty, which a transform can't make of a region that isn't.
    pub fn transformed(&self, t: &Transform) -> Offset {
        Offset::new(
            self.data.shape.transformed(t),
            Length::um(self.data.distance * t.factor()),
            self.data.join,
        )
        .expect("a transformed offset is one")
    }

    fn round(&self) -> bool {
        self.data.join == Join::Round
    }
}

impl Region for Offset {
    fn locate(&self, p: Point) -> Location {
        let d = &self.data;
        if !self.round() {
            return d.flat.locate(p);
        }
        Location::of(d.shape.distance(p).signed.to_um().total_cmp(&d.distance))
    }

    fn distance(&self, p: Point) -> Distance {
        let data = &self.data;
        let mut guess = 0.0;
        if self.round() {
            let a = data.shape.distance(p);
            let d = a.signed.to_um() - data.distance;
            // where d_A = r, p is on the boundary: a distance has no maximum off its region
            if exact(self, um(p), d, a.normal, data.step, || true) {
                return Distance {
                    signed: Length::um(d),
                    normal: a.normal,
                };
            }
            guess = d;
        }
        let (d, n) = data.flat.distance(p, guess);
        Distance {
            signed: Length::um(d),
            normal: n,
        }
    }

    /// Grown with round joins, the shape's box widened by the distance, exactly; otherwise its
    /// polygons' box, widened by their distance from it.
    fn bounds(&self) -> Bounds {
        let d = &self.data;
        if self.round() && d.distance >= 0.0 {
            return widen(d.shape.bounds(), d.distance);
        }
        widen(d.flat.bounds(), d.flat.deviation)
    }

    fn polygons(&self, tolerance: Tolerance) -> Polygons {
        let d = &self.data;
        let (grid, sliver) = (tolerance.grid().to_um(), tolerance.geometric().to_um());
        let make = |e: f64| {
            let (parts, dev) = fine(&d.shape, e);
            (
                clip::without_slivers(clip::offset(&parts, d.distance, d.join, e, grid), sliver),
                dev + e,
            )
        };
        if self.round() {
            return tightened(self, tolerance, d.flat.deviation + d.step, make);
        }
        let (parts, dev) = make(0.5 * tolerance.budget());
        clip::polygons(parts, dev + clip::snapping(tolerance), tolerance)
    }

    fn area(&self) -> f64 {
        self.data.flat.area()
    }

    fn perimeter(&self) -> Option<Length> {
        Some(Length::um(self.data.flat.perimeter()))
    }

    fn fill(&self, cell: Bounds) -> f64 {
        self.data.flat.fill(cell)
    }
}

// ---------------------------------------------------------------------------------------------
// Corner rounding

/// A shape with its corners rounded, a first lithography model: every convex corner to at least
/// `convex` (the opening: shrunk by it, then grown back with round joins), then every concave
/// corner to at least `concave` (the closing: grown, then shrunk back). Features narrower than
/// twice `convex` go, and gaps narrower than twice `concave` close, as they would in print.
/// Cheap to clone.
///
/// Answered from its polygons within [`super::FILL_TOLERANCE`] of its size.
#[derive(Clone)]
pub struct Rounded {
    data: Arc<RoundedData>,
}

struct RoundedData {
    shape: Shape,
    convex: f64,
    concave: f64,
    flat: Flat,
}

impl PartialEq for Rounded {
    fn eq(&self, other: &Rounded) -> bool {
        let (a, b) = (&self.data, &other.data);
        a.shape == b.shape && a.convex == b.convex && a.concave == b.concave
    }
}

impl fmt::Debug for Rounded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rounded")
            .field("shape", &self.data.shape)
            .field("convex_um", &self.data.convex)
            .field("concave_um", &self.data.concave)
            .finish()
    }
}

impl Rounded {
    /// `shape` with its convex corners rounded to at least `convex` and its concave ones to at
    /// least `concave` (zero: left sharp).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless both radii are finite and at least zero, and when the
    /// rounding leaves nothing (a shape narrower than twice `convex` everywhere).
    pub fn new(shape: Shape, convex: Length, concave: Length) -> Result<Rounded> {
        let (r1, r2) = (convex.to_um(), concave.to_um());
        clip::check_radii(r1, r2)?;
        let size = size_of(&shape.bounds()) + 2.0 * r2;
        let t = FILL_TOLERANCE * size;
        let (parts, dev) = fine(&shape, t);
        let out = clip::without_slivers(clip::rounded(&parts, r1, r2, t, 0.0), t);
        let flat = Flat::new(out, dev + 4.0 * t).ok_or_else(|| {
            Error::invalid(
                "corner rounding",
                format!(
                    "leaves nothing: the shape is nowhere wider than twice the convex radius \
                     ({convex})"
                ),
            )
        })?;
        Ok(Rounded {
            data: Arc::new(RoundedData {
                shape,
                convex: r1,
                concave: r2,
                flat,
            }),
        })
    }

    /// The shape rounded.
    pub fn shape(&self) -> &Shape {
        &self.data.shape
    }

    /// The radii: convex corners', then concave corners'.
    pub fn radii(&self) -> [Length; 2] {
        [Length::um(self.data.convex), Length::um(self.data.concave)]
    }

    /// The image under `t`: the shape's image rounded by the radii scaled.
    ///
    /// # Panics
    ///
    /// If the image is empty, which a transform can't make of a region that isn't.
    pub fn transformed(&self, t: &Transform) -> Rounded {
        Rounded::new(
            self.data.shape.transformed(t),
            Length::um(self.data.convex * t.factor()),
            Length::um(self.data.concave * t.factor()),
        )
        .expect("a transformed rounding is one")
    }
}

impl Region for Rounded {
    fn locate(&self, p: Point) -> Location {
        self.data.flat.locate(p)
    }

    fn distance(&self, p: Point) -> Distance {
        let (d, n) = self.data.flat.distance(p, 0.0);
        Distance {
            signed: Length::um(d),
            normal: n,
        }
    }

    fn bounds(&self) -> Bounds {
        let f = &self.data.flat;
        widen(f.bounds(), f.deviation)
    }

    /// The shape's polygons within half the budget, rounded with arcs within a quarter of what
    /// is left for each of the (up to four) offsets.
    fn polygons(&self, tolerance: Tolerance) -> Polygons {
        let d = &self.data;
        let budget = tolerance.budget();
        let (parts, dev) = fine(&d.shape, 0.5 * budget);
        let out = clip::without_slivers(
            clip::rounded(
                &parts,
                d.convex,
                d.concave,
                0.125 * budget,
                tolerance.grid().to_um(),
            ),
            tolerance.geometric().to_um(),
        );
        clip::polygons(
            out,
            dev + 0.5 * budget + clip::snapping(tolerance),
            tolerance,
        )
    }

    fn area(&self) -> f64 {
        self.data.flat.area()
    }

    fn perimeter(&self) -> Option<Length> {
        Some(Length::um(self.data.flat.perimeter()))
    }

    fn fill(&self, cell: Bounds) -> f64 {
        self.data.flat.fill(cell)
    }
}

// ---------------------------------------------------------------------------------------------
// As shapes

impl Shape {
    /// This shape `op` `other` ([`Boolean::new`]).
    ///
    /// # Errors
    ///
    /// As [`Boolean::new`].
    pub fn boolean(&self, op: Operation, other: &Shape) -> Result<Shape> {
        Boolean::new(op, self.clone(), other.clone()).map(Shape::Boolean)
    }

    /// This shape ∪ `other`.
    ///
    /// # Errors
    ///
    /// As [`Boolean::new`].
    pub fn union(&self, other: &Shape) -> Result<Shape> {
        self.boolean(Operation::Union, other)
    }

    /// This shape ∩ `other`.
    ///
    /// # Errors
    ///
    /// As [`Boolean::new`]: empty when they don't overlap.
    pub fn intersection(&self, other: &Shape) -> Result<Shape> {
        self.boolean(Operation::Intersection, other)
    }

    /// This shape \ `other`.
    ///
    /// # Errors
    ///
    /// As [`Boolean::new`]: empty when `other` covers it.
    pub fn difference(&self, other: &Shape) -> Result<Shape> {
        self.boolean(Operation::Difference, other)
    }

    /// This shape △ `other`: in one and not the other.
    ///
    /// # Errors
    ///
    /// As [`Boolean::new`]: empty when they are the same.
    pub fn xor(&self, other: &Shape) -> Result<Shape> {
        self.boolean(Operation::Xor, other)
    }

    /// This shape grown by `distance` (shrunk where it is negative) ([`Offset::new`]).
    ///
    /// # Errors
    ///
    /// As [`Offset::new`].
    pub fn offset(&self, distance: Length, join: Join) -> Result<Shape> {
        Offset::new(self.clone(), distance, join).map(Shape::Offset)
    }

    /// This shape with its corners rounded ([`Rounded::new`]).
    ///
    /// # Errors
    ///
    /// As [`Rounded::new`].
    pub fn rounded(&self, convex: Length, concave: Length) -> Result<Shape> {
        Rounded::new(self.clone(), convex, concave).map(Shape::Rounded)
    }
}
