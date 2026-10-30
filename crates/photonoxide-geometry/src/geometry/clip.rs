//! Booleans, offsets and corner rounding on polygons: what the layout and the studio draw, and
//! what the regions of [`super::compose`] answer their areas and fills from.
//!
//! - **Booleans** run on [`i_overlay`]'s integer overlay: the vertices are put on an integer grid
//!   (the tolerance's grid, or for none a grid of about 2⁻⁶⁰ of the polygons' size), every
//!   crossing is found and rounded to the grid, and the edges are kept by the windings on their
//!   two sides (non-zero). Shared edges, touching vertices and collinear overlaps are therefore
//!   decided on integers, exactly. docs/methods/booleans.md has the measurements that chose it.
//! - **Offsets** are unions (growing) or differences (shrinking) of the polygons with pieces
//!   built here: a quadrilateral along each edge, the offset's width wide, and at each corner the
//!   offset turns round a join: an arc (round), the two offset edges continued to where they meet,
//!   cut square where they would reach farther than the miter limit (miter), or cut square at the
//!   offset's distance (square). The arcs are flattened as [`boundary::flatten`] flattens arcs,
//!   keeping their area.
//! - **Corner rounding** is the opening and closing of mathematical morphology (Haralick,
//!   Sternberg and Zhuang 1987): shrinking then growing by a radius with round joins rounds every
//!   convex corner to that radius and removes what is narrower than twice it; growing then
//!   shrinking fills every concave corner with a fillet of its radius and closes gaps narrower
//!   than twice it.
//! - **Slivers**, polygons or holes thinner than the tolerance (no disk of the tolerance's
//!   diameter fits in them), are dropped from every result.
//!
//! Coordinates are µm, as `[x, y]`; a ring has the region on its left (an outer boundary
//! counterclockwise, a hole clockwise).

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::core::solver::Solver;
use i_overlay::float::overlay::{FloatOverlay, OverlayOptions};

use super::boundary::{self, Conic, P};
use super::primitives::{point, um};
use super::region::{Contour, Polygons, Tolerance};
use crate::units::Length;
use crate::{Error, Result};

/// A loop of vertices, the first not repeated.
pub(crate) type Ring = Vec<P>;

/// A polygon with holes: its outer ring counterclockwise and its holes clockwise.
pub(crate) type Part = (Ring, Vec<Ring>);

/// A boolean operation on two regions, A and B.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operation {
    /// A ∪ B: in either.
    Union,
    /// A ∩ B: in both.
    Intersection,
    /// A \ B: in A and not in B.
    Difference,
    /// A △ B: in one and not the other.
    Xor,
}

impl Operation {
    /// Its name, as job files write it.
    pub fn name(self) -> &'static str {
        match self {
            Operation::Union => "union",
            Operation::Intersection => "intersection",
            Operation::Difference => "difference",
            Operation::Xor => "xor",
        }
    }

    fn rule(self) -> OverlayRule {
        match self {
            Operation::Union => OverlayRule::Union,
            Operation::Intersection => OverlayRule::Intersect,
            Operation::Difference => OverlayRule::Difference,
            Operation::Xor => OverlayRule::Xor,
        }
    }
}

/// How an offset's sides meet where the boundary turns away from the side it moves to (a
/// convex corner when growing, a concave one when shrinking).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Join {
    /// An arc of the offset's radius about the corner: every point of the result at most the
    /// offset's distance from the shape (the Minkowski sum with a disk, or its erosion).
    Round,
    /// The two offset sides continued until they meet, the corner kept sharp; where they would
    /// meet farther than `limit` times the offset's distance from the corner, the corner is cut
    /// square at that distance (perpendicular to the corner's bisector). `limit` is at least 1.
    Miter {
        /// The farthest the corner reaches, in offset distances from the original corner.
        limit: f64,
    },
    /// The corner cut square at the offset's distance from the original corner: a miter with a
    /// limit of 1.
    Square,
}

impl Join {
    /// A miter join with `limit` ([`Join::Miter`]).
    pub fn miter(limit: f64) -> Join {
        Join::Miter { limit }
    }

    /// Its name, as job files write it.
    pub fn name(self) -> &'static str {
        match self {
            Join::Round => "round",
            Join::Miter { .. } => "miter",
            Join::Square => "square",
        }
    }

    /// The join checked: a miter's limit finite and at least 1.
    pub(crate) fn checked(self) -> Result<Join> {
        if let Join::Miter { limit } = self
            && !(limit.is_finite() && limit >= 1.0)
        {
            return Err(Error::invalid(
                "offset",
                format!("a miter's limit is at least 1 (offset distances), got {limit}"),
            ));
        }
        Ok(self)
    }
}

pub(crate) fn shoelace(v: &[P]) -> f64 {
    let n = v.len();
    (0..n)
        .map(|i| {
            let (a, b) = (v[i], v[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

pub(crate) fn perimeter(v: &[P]) -> f64 {
    let n = v.len();
    (0..n)
        .map(|i| {
            let (a, b) = (v[i], v[(i + 1) % n]);
            (b[0] - a[0]).hypot(b[1] - a[1])
        })
        .sum()
}

/// Every ring of `parts`, outer rings and holes alike, each with the region on its left.
fn rings(parts: &[Part]) -> Vec<Ring> {
    parts
        .iter()
        .flat_map(|(o, h)| std::iter::once(o).chain(h))
        .filter(|r| r.len() >= 3)
        .cloned()
        .collect()
}

/// The overlay of `subject` and `clip` (rings, the region of each where its winding number is
/// not zero) by `op`, on a grid of `grid` µm (zero: the finest the integers hold).
fn overlay(subject: &[Ring], clip: &[Ring], op: Operation, grid: f64) -> Vec<Part> {
    if subject.is_empty() && clip.is_empty() {
        return Vec::new();
    }
    let options = OverlayOptions::<f64, i64>::default();
    let solver = Solver::default();
    let fixed = (grid > 0.0)
        .then(|| {
            FloatOverlay::<P, i64>::from_subj_and_clip_fixed_scale_custom(
                subject,
                clip,
                options,
                solver,
                1.0 / grid,
            )
            .ok()
        })
        .flatten();
    let mut o = fixed.unwrap_or_else(|| {
        FloatOverlay::<P, i64>::from_subj_and_clip_custom(subject, clip, options, solver)
    });
    let scale = subject
        .iter()
        .chain(clip)
        .flatten()
        .fold(1.0f64, |m, p| m.max(p[0].abs()).max(p[1].abs()));
    o.overlay(op.rule(), FillRule::NonZero)
        .into_iter()
        .filter_map(|shape| {
            let mut contours = shape.into_iter().map(|c| despiked(c, scale));
            let outer = contours.next()?;
            (outer.len() >= 3).then(|| (outer, contours.filter(|h| h.len() >= 3).collect()))
        })
        .collect()
}

/// A ring without spikes: vertices where it turns back on itself (by half a turn, within 10⁻⁹
/// rad), and vertices within 10⁻¹³ of `scale` of the one before, dropped until none is left.
/// Rounding can leave a crack or a needle of no width where two pieces meet, which encloses
/// nothing but which an offset would grow.
fn despiked(mut ring: Ring, scale: f64) -> Ring {
    let near = 1e-13 * scale;
    loop {
        let n = ring.len();
        if n < 3 {
            return ring;
        }
        let drop = (0..n).find(|&i| {
            let (a, b, c) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
            let (e1, e2) = (sub(b, a), sub(c, b));
            let (l1, l2) = (e1[0].hypot(e1[1]), e2[0].hypot(e2[1]));
            l1 <= near || (dot(e1, e2) < 0.0 && cross(e1, e2).abs() <= 1e-9 * l1 * l2)
        });
        match drop {
            Some(i) => {
                ring.remove(i);
            }
            None => return ring,
        }
    }
}

/// `a` op `b` on a grid of `grid` (zero: none), without slivers thinner than `width` µm.
pub(crate) fn boolean(a: &[Part], b: &[Part], op: Operation, grid: f64, width: f64) -> Vec<Part> {
    without_slivers(overlay(&rings(a), &rings(b), op, grid), width)
}

/// The union of `parts` with themselves: overlaps merged, on a grid of `grid`.
pub(crate) fn simplified(parts: &[Part], grid: f64) -> Vec<Part> {
    overlay(&rings(parts), &[], Operation::Union, grid)
}

/// Whether a ring is thinner than `width`: no disk of diameter `width` fits in it. A ring whose
/// area is at least `width` times half its perimeter has room for one if it is convex, and is
/// kept without looking further; one with less is shrunk by `width`/2, and is a sliver if
/// nothing is left.
fn thin(ring: &Ring, width: f64) -> bool {
    if width <= 0.0 {
        return false;
    }
    let a = shoelace(ring).abs();
    if 2.0 * a >= width * perimeter(ring) {
        return false;
    }
    let mut r = ring.clone();
    if shoelace(&r) < 0.0 {
        r.reverse();
    }
    offset(
        &[(r, Vec::new())],
        -0.5 * width,
        Join::Round,
        0.125 * width,
        0.0,
    )
    .is_empty()
}

/// `parts` without the polygons and holes thinner than `width` µm: a thin polygon goes with its
/// holes, a thin hole is filled.
pub(crate) fn without_slivers(parts: Vec<Part>, width: f64) -> Vec<Part> {
    parts
        .into_iter()
        .filter(|(outer, _)| !thin(outer, width))
        .map(|(outer, holes)| {
            (
                outer,
                holes.into_iter().filter(|h| !thin(h, width)).collect(),
            )
        })
        .collect()
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}

fn add(a: P, b: P, s: f64) -> P {
    [a[0] + s * b[0], a[1] + s * b[1]]
}

fn cross(a: P, b: P) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn unit(a: P) -> Option<P> {
    let l = a[0].hypot(a[1]);
    (l > 0.0 && l.is_finite()).then(|| [a[0] / l, a[1] / l])
}

/// A ring counterclockwise; `None` if it has no area.
fn ccw(mut ring: Ring) -> Option<Ring> {
    let a = shoelace(&ring);
    if a == 0.0 || !a.is_finite() {
        return None;
    }
    if a < 0.0 {
        ring.reverse();
    }
    Some(ring)
}

/// The piece between the directions `u1` and `u2` (unit vectors from the corner `v`, less than
/// half a turn apart) out to `r`, as `join` has it: the arc's sector within `budget` (its area
/// kept), the miter's quadrilateral, or a cut miter's pentagon.
fn join_piece(v: P, u1: P, u2: P, r: f64, join: Join, budget: f64) -> Option<Ring> {
    let mut ring = join_chain(v, u1, u2, r, join, budget)?;
    ring.insert(0, v);
    ccw(ring)
}

/// The outer side of a join: from v + r u1 to v + r u2, both to the bit as the edges' offsets
/// have them, through the arc's vertices, the miter's point or a cut miter's two corners.
fn join_chain(v: P, u1: P, u2: P, r: f64, join: Join, budget: f64) -> Option<Ring> {
    let phi = cross(u1, u2).atan2(dot(u1, u2));
    if phi == 0.0 || !phi.is_finite() {
        return None;
    }
    let mut chain = match join {
        Join::Round => {
            let t0 = u1[1].atan2(u1[0]);
            let arc = Conic {
                c: v,
                rx: r,
                ry: r,
                cos: 1.0,
                sin: 0.0,
                t0,
                t1: t0 + phi,
            };
            let (w, _) = boundary::flatten(&arc, budget, false);
            // its first vertex is on the arc: the start itself
            w.into_iter().skip(1).collect()
        }
        Join::Miter { .. } | Join::Square => {
            let limit = match join {
                Join::Miter { limit } => limit,
                _ => 1.0,
            };
            let half = 0.5 * phi.abs();
            // the bisector: for a near half turn, square to u1 on the turn's side
            let m = unit([u1[0] + u2[0], u1[1] + u2[1]]).unwrap_or_else(|| {
                let s = phi.signum();
                [-s * u1[1], s * u1[0]]
            });
            let reach = r / half.cos();
            if reach.is_finite() && reach <= limit * r {
                vec![add(v, m, reach)]
            } else {
                // cut perpendicular to the bisector at limit r: along each offset side from its
                // foot, t = (L − r cos(φ/2)) / sin(φ/2)
                let l = limit * r;
                let t = (l - r * half.cos()) / half.sin();
                let w1 = unit(add(m, u1, -dot(m, u1)))?;
                let w2 = unit(add(m, u2, -dot(m, u2)))?;
                let (f1, f2) = (add(v, u1, r), add(v, u2, r));
                vec![add(f1, w1, t), add(f2, w2, t)]
            }
        }
    };
    chain.insert(0, add(v, u1, r));
    chain.push(add(v, u2, r));
    Some(chain)
}

/// An edge of a ring: its ends and its outward unit normal.
#[derive(Clone, Copy)]
struct Edge {
    a: P,
    b: P,
    n: P,
}

impl Edge {
    fn direction(&self) -> P {
        [-self.n[1], self.n[0]]
    }
}

/// The pieces that offset one ring (the region on its left) by `r` µm, outward for `s` = 1 and
/// inward for −1: bands along runs of its edges, each the region between the run and its
/// offset, and joins where a run ends at a corner that turns away from the side it moves to.
///
/// A run goes on through a corner turning away (a join's side), its join in the band, while
/// the run turns by at most a quarter turn in all; and through a corner turning towards the
/// side (where the two offsets cross), the band's vertex there where they cross, while both
/// offset edges keep their direction. The band is then the union of the run's edges' strips
/// and joins, and simple; a strip per edge would give the same union, but along a curve's many
/// short edges their overlaps would cost the overlay dearly.
fn ring_pieces(ring: &[P], r: f64, s: f64, join: Join, budget: f64, out: &mut Vec<Ring>) {
    let n = ring.len();
    let edges: Vec<Edge> = (0..n)
        .filter_map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            unit(sub(b, a)).map(|e| Edge {
                a,
                b,
                n: [e[1], -e[0]],
            })
        })
        .collect();
    let m = edges.len();
    if m == 0 {
        return;
    }
    let off = |k: usize| -> P { [s * edges[k].n[0], s * edges[k].n[1]] };
    // the turn at the start of edge k, from edge k − 1: positive to the left
    let turn: Vec<f64> = (0..m)
        .map(|k| {
            let (d1, d2) = (edges[(k + m - 1) % m].direction(), edges[k].direction());
            cross(d1, d2).atan2(dot(d1, d2))
        })
        .collect();
    // a corner that turns away from the side the offset moves to (left, when growing) joins
    let joins = |k: usize| turn[k] * s > 0.0;
    let mut done = 0;
    while done < m {
        let k0 = done;
        if joins(k0) {
            let prev = (k0 + m - 1) % m;
            out.extend(join_piece(edges[k0].a, off(prev), off(k0), r, join, budget));
        }
        let mut original = vec![edges[k0].a, edges[k0].b];
        let mut offset = vec![add(edges[k0].a, edges[k0].n, s * r)];
        let mut turned = 0.0;
        let mut k = k0;
        while k + 1 < m {
            let next = k + 1;
            let (e, f) = (edges[k], edges[next]);
            let t = turn[next].abs();
            let added: Option<Ring> = if turn[next] == 0.0 {
                Some(vec![add(e.b, e.n, s * r)])
            } else if turned + t > 0.5 * std::f64::consts::PI {
                None
            } else if joins(next) {
                join_chain(e.b, off(k), off(next), r, join, budget)
            } else {
                // where the two offsets cross: v + r (o₁ + o₂)/(1 + o₁·o₂)
                let (o1, o2) = (off(k), off(next));
                let w = r / (1.0 + dot(o1, o2));
                let x = [e.b[0] + w * (o1[0] + o2[0]), e.b[1] + w * (o1[1] + o2[1])];
                let from = *offset.last().expect("a start");
                let to = add(f.b, f.n, s * r);
                (dot(sub(x, from), e.direction()) > 0.0 && dot(sub(to, x), f.direction()) > 0.0)
                    .then(|| vec![x])
            };
            let Some(added) = added else { break };
            turned += t;
            offset.extend(added);
            original.push(f.b);
            k = next;
        }
        offset.push(add(edges[k].b, edges[k].n, s * r));
        offset.reverse();
        original.extend(offset);
        out.extend(ccw(original));
        done = k + 1;
    }
}

/// `parts` offset by `distance` µm (positive grows, negative shrinks) with `join`, its arcs
/// within `budget` µm, on a grid of `grid` (zero: none).
pub(crate) fn offset(
    parts: &[Part],
    distance: f64,
    join: Join,
    budget: f64,
    grid: f64,
) -> Vec<Part> {
    let base = rings(parts);
    if distance == 0.0 || base.is_empty() {
        return overlay(&base, &[], Operation::Union, grid);
    }
    let grow = distance > 0.0;
    let (r, s) = (distance.abs(), distance.signum());
    if !grow {
        // every point of a region lies within half its box's narrower side of its boundary
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for p in base.iter().flatten() {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        if 2.0 * r >= (hi[0] - lo[0]).min(hi[1] - lo[1]) {
            return Vec::new();
        }
    }
    let mut pieces: Vec<Ring> = Vec::new();
    for ring in &base {
        ring_pieces(ring, r, s, join, budget, &mut pieces);
    }
    let pieces = rings(&union_all(&pieces));
    if grow {
        overlay(&base, &pieces, Operation::Union, grid)
    } else {
        overlay(&base, &pieces, Operation::Difference, grid)
    }
}

/// The union of many pieces, halves merged in turn (in their order, so the same bits every
/// time): pieces that overlap many of their neighbours (a shrink deeper than a curve's radius)
/// are merged a few at a time instead of all their crossings at once.
fn union_all(pieces: &[Ring]) -> Vec<Part> {
    if pieces.len() <= 32 {
        return overlay(pieces, &[], Operation::Union, 0.0);
    }
    let (a, b) = pieces.split_at(pieces.len() / 2);
    overlay(
        &rings(&union_all(a)),
        &rings(&union_all(b)),
        Operation::Union,
        0.0,
    )
}

/// `parts` with their convex corners rounded to `convex` µm (an opening: shrunk then grown) and
/// then their concave corners to `concave` µm (a closing: grown then shrunk), round joins within
/// `budget` µm each, the result on a grid of `grid`.
pub(crate) fn rounded(
    parts: &[Part],
    convex: f64,
    concave: f64,
    budget: f64,
    grid: f64,
) -> Vec<Part> {
    let mut out = parts.to_vec();
    let steps: Vec<f64> = [(-convex, convex), (concave, -concave)]
        .into_iter()
        .filter(|(d, _)| *d != 0.0)
        .flat_map(|(a, b)| [a, b])
        .collect();
    for (k, d) in steps.iter().enumerate() {
        let last = k + 1 == steps.len();
        out = offset(&out, *d, Join::Round, budget, if last { grid } else { 0.0 });
        if out.is_empty() {
            break;
        }
    }
    if steps.is_empty() {
        out = simplified(&out, grid);
    }
    out
}

/// A region's polygons as parts, µm.
pub(crate) fn parts(p: &Polygons) -> Vec<Part> {
    p.contours
        .iter()
        .map(|c| {
            (
                c.outer.iter().map(|v| um(*v)).collect(),
                c.holes
                    .iter()
                    .map(|h| h.iter().map(|v| um(*v)).collect())
                    .collect(),
            )
        })
        .collect()
}

/// Parts as polygons, as computed (already on the tolerance's grid), with the `deviation` they
/// reach: rings of fewer than 3 vertices or no area dropped.
pub(crate) fn polygons(parts: Vec<Part>, deviation: f64, tolerance: Tolerance) -> Polygons {
    let ring = |r: Ring| -> Option<Vec<super::Point>> {
        (r.len() >= 3 && shoelace(&r) != 0.0).then(|| r.into_iter().map(point).collect())
    };
    Polygons {
        contours: parts
            .into_iter()
            .filter_map(|(o, h)| {
                Some(Contour {
                    outer: ring(o)?,
                    holes: h.into_iter().filter_map(ring).collect(),
                })
            })
            .collect(),
        tolerance,
        deviation: Length::um(deviation),
    }
}

/// The snapping's own error: half a grid cell's diagonal.
pub(crate) fn snapping(tolerance: Tolerance) -> f64 {
    tolerance.grid().to_um() * std::f64::consts::FRAC_1_SQRT_2
}

impl Polygons {
    /// These polygons `op` `other`, oriented, with their holes, on `tolerance`'s grid, and
    /// without polygons or holes thinner than its geometric tolerance (slivers). Shared edges,
    /// touching vertices and collinear overlaps are decided exactly on the grid.
    ///
    /// The result's deviation is the larger of the two inputs' plus the snapping's (half a grid
    /// cell's diagonal): where two boundaries cross at an angle θ, an input's deviation can move
    /// the crossing by up to that over sin θ, which a region's [`super::Region::polygons`]
    /// measures and tightens ([`super::Boolean`]).
    pub fn boolean(&self, other: &Polygons, op: Operation, tolerance: Tolerance) -> Polygons {
        let out = boolean(
            &parts(self),
            &parts(other),
            op,
            tolerance.grid().to_um(),
            tolerance.geometric().to_um(),
        );
        let dev = self.deviation.to_um().max(other.deviation.to_um()) + snapping(tolerance);
        polygons(out, dev, tolerance)
    }

    /// These polygons grown by `distance` (shrunk where it is negative) with `join` at their
    /// corners, the arcs of round joins within `tolerance`'s budget, on its grid, without
    /// slivers.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the distance is finite and a miter's limit at least 1.
    pub fn offset(&self, distance: Length, join: Join, tolerance: Tolerance) -> Result<Polygons> {
        let d = distance.to_um();
        if !d.is_finite() {
            return Err(Error::invalid(
                "offset",
                format!("needs a finite distance, got {distance}"),
            ));
        }
        let join = join.checked()?;
        let out = without_slivers(
            offset(
                &parts(self),
                d,
                join,
                tolerance.budget(),
                tolerance.grid().to_um(),
            ),
            tolerance.geometric().to_um(),
        );
        let dev = self.deviation.to_um() + tolerance.budget() + snapping(tolerance);
        Ok(polygons(out, dev, tolerance))
    }

    /// These polygons with their convex corners rounded to at least `convex` and their concave
    /// corners to at least `concave` (either zero: left as they are): the opening by `convex`,
    /// then the closing by `concave` (docs/methods/booleans.md). What is narrower than twice
    /// `convex` goes, and gaps narrower than twice `concave` close.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless both radii are finite and at least zero.
    pub fn rounded(
        &self,
        convex: Length,
        concave: Length,
        tolerance: Tolerance,
    ) -> Result<Polygons> {
        let (r1, r2) = (convex.to_um(), concave.to_um());
        check_radii(r1, r2)?;
        let stages = 2.0 * (f64::from(u8::from(r1 > 0.0)) + f64::from(u8::from(r2 > 0.0)));
        let budget = tolerance.budget() / stages.max(1.0);
        let out = without_slivers(
            rounded(&parts(self), r1, r2, budget, tolerance.grid().to_um()),
            tolerance.geometric().to_um(),
        );
        let dev = self.deviation.to_um() + tolerance.budget() + snapping(tolerance);
        Ok(polygons(out, dev, tolerance))
    }
}

/// Corner radii checked: finite and at least zero.
pub(crate) fn check_radii(convex: f64, concave: f64) -> Result<()> {
    if !(convex.is_finite() && concave.is_finite() && convex >= 0.0 && concave >= 0.0) {
        return Err(Error::invalid(
            "corner rounding",
            format!(
                "needs finite radii of at least zero, got {convex} µm (convex) and {concave} µm \
                 (concave)"
            ),
        ));
    }
    Ok(())
}
