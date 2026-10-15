//! What the validation report and the tests check booleans, offsets and corner rounding on:
//! inclusion–exclusion with the exact areas of lenses and overlaps, the degenerate cases of
//! polygon booleans, the composed distance against the exact boundary, offsets and fillets
//! against Steiner's formula and the corner cuts, the polygons' convergence, and Zhang et al.'s
//! Y-junction assembled by union.

use std::f64::consts::{PI, TAU};

use super::*;
use crate::units::Length;
use crate::validation::{Case, Outcome, Tier};

fn um(v: f64) -> Length {
    Length::um(v)
}

/// The area of the lens two disks of radii `r1` and `r2` with centres `d` apart share.
pub(crate) fn lens(r1: f64, r2: f64, d: f64) -> f64 {
    if d >= r1 + r2 {
        return 0.0;
    }
    if d <= (r1 - r2).abs() {
        let r = r1.min(r2);
        return PI * r * r;
    }
    let a1 = ((d * d + r1 * r1 - r2 * r2) / (2.0 * d * r1)).acos();
    let a2 = ((d * d + r2 * r2 - r1 * r1) / (2.0 * d * r2)).acos();
    let k = 0.5 * ((-d + r1 + r2) * (d + r1 - r2) * (d - r1 + r2) * (d + r1 + r2)).sqrt();
    r1 * r1 * a1 + r2 * r2 * a2 - k
}

/// An axis-aligned rectangle from its corners.
pub(crate) fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Shape {
    Shape::rect(
        Point::um(0.5 * (x0 + x1), 0.5 * (y0 + y1)),
        um(x1 - x0),
        um(y1 - y0),
    )
    .expect("a rectangle")
}

pub(crate) fn circle(x: f64, y: f64, r: f64) -> Shape {
    Shape::circle(Point::um(x, y), um(r)).expect("a circle")
}

pub(crate) fn polygon(v: &[[f64; 2]]) -> Shape {
    Shape::Polygon(
        Polygon::new(v.iter().map(|p| Point::um(p[0], p[1])).collect()).expect("a polygon"),
    )
}

/// The four operations.
pub(crate) const OPERATIONS: [Operation; 4] = [
    Operation::Union,
    Operation::Intersection,
    Operation::Difference,
    Operation::Xor,
];

/// The areas of A ∪ B, A ∩ B, A \ B and A △ B from |A|, |B| and |A ∩ B|.
fn areas(a: f64, b: f64, i: f64) -> [f64; 4] {
    [a + b - i, i, a - i, a + b - 2.0 * i]
}

/// Pairs of shapes that overlap, with the exact areas of their union, intersection,
/// difference and xor: two equal disks, two unequal ones, two rectangles, a disk on a
/// rectangle's edge touching two of its sides, half an annulus, a square and a diamond.
pub(crate) fn pairs() -> Vec<(&'static str, Shape, Shape, [f64; 4])> {
    let s2 = 2f64.sqrt();
    let (c1, r1, c2, r2): ([f64; 2], f64, [f64; 2], f64) = ([0.31, -0.17], 1.2, [1.5, 0.4], 0.7);
    let d12 = (c2[0] - c1[0]).hypot(c2[1] - c1[1]);
    let octagon = 8.0 * s2 - 8.0;
    vec![
        (
            "equal disks",
            circle(0.0, 0.0, 1.0),
            circle(1.0, 0.0, 1.0),
            areas(PI, PI, lens(1.0, 1.0, 1.0)),
        ),
        (
            "unequal disks",
            circle(c1[0], c1[1], r1),
            circle(c2[0], c2[1], r2),
            areas(PI * r1 * r1, PI * r2 * r2, lens(r1, r2, d12)),
        ),
        (
            "rectangles",
            rect(0.0, 0.0, 2.0, 1.0),
            rect(1.2, 0.4, 3.0, 1.7),
            areas(2.0, 1.8 * 1.3, 0.8 * 0.6),
        ),
        (
            "disk on an edge, tangent to two",
            rect(0.0, 0.0, 2.0, 1.0),
            circle(2.0, 0.5, 0.5),
            areas(2.0, PI * 0.25, PI * 0.125),
        ),
        (
            "half an annulus",
            Shape::ring(Point::um(0.0, 0.0), um(0.75), um(0.5)).expect("a ring"),
            rect(0.0, -2.0, 2.0, 2.0),
            areas(PI * 0.75, 8.0, PI * 0.375),
        ),
        (
            "square and diamond",
            rect(-1.0, -1.0, 1.0, 1.0),
            polygon(&[[s2, 0.0], [0.0, s2], [-s2, 0.0], [0.0, -s2]]),
            areas(4.0, 4.0, octagon),
        ),
    ]
}

/// The regions' areas against the closed forms, and inclusion–exclusion, |A ∪ B| + |A ∩ B| =
/// |A| + |B|, |A △ B| = |A ∪ B| − |A ∩ B|, |A \ B| = |A| − |A ∩ B| among the computed areas:
/// the largest relative difference.
pub(crate) fn area_error() -> f64 {
    let mut worst = 0.0f64;
    for (_, a, b, exact) in pairs() {
        let mut got = [0.0; 4];
        for (k, op) in OPERATIONS.into_iter().enumerate() {
            let Ok(s) = a.boolean(op, &b) else {
                return f64::INFINITY;
            };
            got[k] = s.area();
            worst = worst.max(((got[k] - exact[k]) / exact[k]).abs());
        }
        let (aa, ab) = (a.area(), b.area());
        let scale = aa + ab;
        worst = worst
            .max((got[0] + got[1] - aa - ab).abs() / scale)
            .max((got[3] - got[0] + got[1]).abs() / scale)
            .max((got[2] - aa + got[1]).abs() / scale);
    }
    worst
}

/// The same booleans' polygons at the default tolerance (1 nm on a grid of 1 nm): (the largest
/// distance reached, nm; the largest area difference over the tolerance times the exact
/// boundary's length; the polygons and holes wrongly oriented or thinner than the tolerance).
pub(crate) fn polygon_errors() -> (f64, f64, usize) {
    let t = Tolerance::default();
    let tol = t.geometric().to_um();
    let (mut dev, mut area, mut bad) = (0.0f64, 0.0f64, 0usize);
    for (_, a, b, exact) in pairs() {
        for (k, op) in OPERATIONS.into_iter().enumerate() {
            let Ok(s) = a.boolean(op, &b) else {
                return (f64::INFINITY, f64::INFINITY, 1);
            };
            let p = s.polygons(t);
            dev = dev.max(p.deviation.to_um() * 1e3);
            let length = s.perimeter().map_or(f64::INFINITY, Length::to_um);
            area = area.max((p.area() - exact[k]).abs() / (tol * length));
            bad += misdrawn(&p, tol);
        }
    }
    (dev, area, bad)
}

/// How many of the polygons' rings are wrongly oriented, or have less area than `width` times
/// half their length (as a sliver of width `width` would).
fn misdrawn(p: &Polygons, width: f64) -> usize {
    let ring = |r: &[Point], outer: bool| {
        let v: Vec<[f64; 2]> = r.iter().map(|q| [q.x.to_um(), q.y.to_um()]).collect();
        let a = clip::shoelace(&v);
        usize::from((a > 0.0) != outer || 2.0 * a.abs() < width * clip::perimeter(&v))
    };
    p.contours
        .iter()
        .map(|c| ring(&c.outer, true) + c.holes.iter().map(|h| ring(h, false)).sum::<usize>())
        .sum()
}

/// A degenerate pair for polygon booleans: the shapes, and for each operation the polygons'
/// expected area, number of polygons and of holes at the default tolerance (`None`: empty, the
/// region refused).
type Degenerate = (&'static str, Shape, Shape, [Option<(f64, usize, usize)>; 4]);

pub(crate) fn degenerate() -> Vec<Degenerate> {
    let eps = 1e-7;
    let far = 10_000.0;
    let ring = |x0: f64, y0: f64, x1: f64, y1: f64, hx0: f64, hy0: f64, hx1: f64, hy1: f64| {
        rect(x0, y0, x1, y1)
            .difference(&rect(hx0, hy0, hx1, hy1))
            .expect("a frame")
    };
    let thin = [[0.0, 0.0], [10.0, 0.0], [10.0, 0.5], [0.0, 0.5]];
    let turned: Vec<[f64; 2]> = thin
        .iter()
        .map(|p| {
            let (s, c) = 1e-9f64.sin_cos();
            let (dx, dy) = (p[0] - 5.0, p[1] - 0.25);
            [5.0 + c * dx - s * dy, 0.25 + s * dx + c * dy]
        })
        .collect();
    vec![
        (
            "a shared edge",
            rect(0.0, 0.0, 1.0, 1.0),
            rect(1.0, 0.0, 2.0, 1.0),
            [
                Some((2.0, 1, 0)),
                None,
                Some((1.0, 1, 0)),
                Some((2.0, 1, 0)),
            ],
        ),
        (
            "a touching vertex",
            rect(0.0, 0.0, 1.0, 1.0),
            rect(1.0, 1.0, 2.0, 2.0),
            [
                Some((2.0, 2, 0)),
                None,
                Some((1.0, 1, 0)),
                Some((2.0, 2, 0)),
            ],
        ),
        (
            "a collinear overlap",
            rect(0.0, 0.0, 2.0, 1.0),
            rect(1.0, 1.0, 3.0, 2.0),
            [
                Some((4.0, 1, 0)),
                None,
                Some((2.0, 1, 0)),
                Some((4.0, 1, 0)),
            ],
        ),
        (
            "a vertex on an edge",
            rect(0.0, 0.0, 2.0, 1.0),
            polygon(&[[0.5, 1.0], [1.5, 1.0], [1.0, 2.0]]),
            [
                Some((2.5, 1, 0)),
                None,
                Some((2.0, 1, 0)),
                Some((2.5, 1, 0)),
            ],
        ),
        (
            "the same shape",
            rect(0.0, 0.0, 1.0, 1.0),
            rect(0.0, 0.0, 1.0, 1.0),
            [Some((1.0, 1, 0)), Some((1.0, 1, 0)), None, None],
        ),
        (
            "a gap of 0.1 pm",
            rect(0.0, 0.0, 1.0, 1.0),
            rect(1.0 + eps, 0.0, 2.0, 1.0),
            [
                Some((2.0, 1, 0)),
                None,
                Some((1.0, 1, 0)),
                Some((2.0, 1, 0)),
            ],
        ),
        (
            "an overlap of 0.1 pm",
            rect(0.0, 0.0, 1.0, 1.0),
            rect(1.0 - eps, 0.0, 2.0, 1.0),
            [
                Some((2.0, 1, 0)),
                None,
                Some((1.0, 1, 0)),
                Some((2.0, 1, 0)),
            ],
        ),
        (
            "a hole",
            rect(0.0, 0.0, 4.0, 4.0),
            rect(1.0, 1.0, 3.0, 3.0),
            [
                Some((16.0, 1, 0)),
                Some((4.0, 1, 0)),
                Some((12.0, 1, 1)),
                Some((12.0, 1, 1)),
            ],
        ),
        (
            "a hole opening on an edge",
            rect(0.0, 0.0, 4.0, 4.0),
            rect(1.0, 0.0, 3.0, 3.0),
            [
                Some((16.0, 1, 0)),
                Some((6.0, 1, 0)),
                Some((10.0, 1, 0)),
                Some((10.0, 1, 0)),
            ],
        ),
        (
            "two frames",
            ring(0.0, 0.0, 4.0, 4.0, 1.0, 1.0, 3.0, 3.0),
            ring(2.0, 0.0, 6.0, 4.0, 3.0, 1.0, 5.0, 3.0),
            [
                Some((20.0, 1, 2)),
                Some((4.0, 2, 0)),
                Some((8.0, 2, 0)),
                Some((16.0, 3, 0)),
            ],
        ),
        (
            "edges 1e-9 rad apart",
            polygon(&thin),
            polygon(&turned),
            [Some((5.0, 1, 0)), Some((5.0, 1, 0)), None, None],
        ),
        (
            "a shared edge 1 cm away",
            rect(far, far, far + 1.0, far + 1.0),
            rect(far + 1.0, far, far + 2.0, far + 1.0),
            [
                Some((2.0, 1, 0)),
                None,
                Some((1.0, 1, 0)),
                Some((2.0, 1, 0)),
            ],
        ),
    ]
}

/// The degenerate pairs' polygons at the default tolerance against what they should be: the
/// largest area difference (µm²), infinite where the number of polygons or holes is wrong, a
/// ring is misdrawn, or a region that should be refused isn't (or one that shouldn't be is).
/// A region refused for being empty is right where the polygons are empty.
pub(crate) fn degenerate_error() -> f64 {
    let t = Tolerance::default();
    let mut worst = 0.0f64;
    for (_, a, b, want) in degenerate() {
        let pa = a.polygons(t);
        let pb = b.polygons(t);
        for (k, op) in OPERATIONS.into_iter().enumerate() {
            let p = pa.boolean(&pb, op, t);
            let holes: usize = p.contours.iter().map(|c| c.holes.len()).sum();
            match want[k] {
                Some((area, outers, h)) => {
                    if p.contours.len() != outers || holes != h || misdrawn(&p, 1e-3) > 0 {
                        return f64::INFINITY;
                    }
                    worst = worst.max((p.area() - area).abs());
                }
                None => {
                    if !p.contours.is_empty() {
                        return f64::INFINITY;
                    }
                }
            }
        }
    }
    worst
}

/// The distance from `p` to the arc of the circle (centre `c`, radius `r`) of the angles from
/// `t0` through `sweep` (radians, counterclockwise).
fn to_arc(p: [f64; 2], c: [f64; 2], r: f64, t0: f64, sweep: f64) -> f64 {
    let t = (p[1] - c[1]).atan2(p[0] - c[0]);
    let along = (t - t0).rem_euclid(TAU);
    if along <= sweep {
        return ((p[0] - c[0]).hypot(p[1] - c[1]) - r).abs();
    }
    let end = |a: f64| (p[0] - c[0] - r * a.cos()).hypot(p[1] - c[1] - r * a.sin());
    end(t0).min(end(t0 + sweep))
}

/// The exact distance to the boundary of `op` on the disks (centre, radius) `a` and `b`, which
/// cross: each boundary is arcs of the two circles, each circle's arc outside the other disk or
/// inside it.
fn disks_distance(p: [f64; 2], a: ([f64; 2], f64), b: ([f64; 2], f64), op: Operation) -> f64 {
    // the arc of circle `s` inside the disk `o`: the angles within ±α of the direction to o
    let inside = |s: ([f64; 2], f64), o: ([f64; 2], f64)| {
        let d = (o.0[0] - s.0[0]).hypot(o.0[1] - s.0[1]);
        let alpha = ((d * d + s.1 * s.1 - o.1 * o.1) / (2.0 * d * s.1)).acos();
        let toward = (o.0[1] - s.0[1]).atan2(o.0[0] - s.0[0]);
        (toward - alpha, 2.0 * alpha)
    };
    let (ia, ib) = (inside(a, b), inside(b, a));
    let arc_in = |s: ([f64; 2], f64), (t0, sw): (f64, f64)| to_arc(p, s.0, s.1, t0, sw);
    let arc_out = |s: ([f64; 2], f64), (t0, sw): (f64, f64)| to_arc(p, s.0, s.1, t0 + sw, TAU - sw);
    match op {
        Operation::Union => arc_out(a, ia).min(arc_out(b, ib)),
        Operation::Intersection => arc_in(a, ia).min(arc_in(b, ib)),
        Operation::Difference => arc_out(a, ia).min(arc_in(b, ib)),
        Operation::Xor => arc_in(a, ia)
            .min(arc_out(a, ia))
            .min(arc_in(b, ib))
            .min(arc_out(b, ib)),
    }
}

/// Points about a box: a lattice of 15 × 15 over it widened by half its size each way.
fn points_about(b: Bounds) -> Vec<Point> {
    let [lo, hi] = b.corners();
    let (w, h) = (hi[0] - lo[0], hi[1] - lo[1]);
    let mut out = Vec::new();
    for i in 0..15 {
        for j in 0..15 {
            // an irrational offset keeps the points off the shapes' symmetries
            let x = lo[0] - 0.5 * w + 2.0 * w * (f64::from(i) + 0.3819) / 15.0;
            let y = lo[1] - 0.5 * h + 2.0 * h * (f64::from(j) + 0.6180) / 15.0;
            out.push(Point::um(x, y));
        }
    }
    out
}

/// The composed distance of the four operations on two crossing disks, at 225 points about
/// them each, against the exact distance to their boundary (arcs of the two circles), signed by
/// the exact inside test of the two disks: the largest difference, nm; and the largest distance
/// of points on the exact boundary (both arcs of each, 64 points each) from it, nm.
pub(crate) fn distance_errors() -> (f64, f64) {
    let (a, b) = (([0.31, -0.17], 1.2), ([1.5, 0.4], 0.7));
    let (sa, sb) = (circle(a.0[0], a.0[1], a.1), circle(b.0[0], b.0[1], b.1));
    let (mut off, mut on) = (0.0f64, 0.0f64);
    for op in OPERATIONS {
        let Ok(s) = sa.boolean(op, &sb) else {
            return (f64::INFINITY, f64::INFINITY);
        };
        for p in points_about(s.bounds()) {
            let q = [p.x.to_um(), p.y.to_um()];
            let ina = (q[0] - a.0[0]).hypot(q[1] - a.0[1]) < a.1;
            let inb = (q[0] - b.0[0]).hypot(q[1] - b.0[1]) < b.1;
            let inside = match op {
                Operation::Union => ina || inb,
                Operation::Intersection => ina && inb,
                Operation::Difference => ina && !inb,
                Operation::Xor => ina != inb,
            };
            let exact = disks_distance(q, a, b, op);
            let exact = if inside { -exact } else { exact };
            off = off.max((s.distance(p).signed.to_um() - exact).abs() * 1e3);
        }
        // points of the boundary: each circle's arcs that bound the result
        for (c, r) in [a, b] {
            for k in 0..64 {
                let t = TAU * (f64::from(k) + 0.5) / 64.0;
                let q = [c[0] + r * t.cos(), c[1] + r * t.sin()];
                if disks_distance(q, a, b, op) < 1e-12 {
                    on = on.max(s.distance(Point::um(q[0], q[1])).signed.to_um().abs() * 1e3);
                }
            }
        }
    }
    (off, on)
}

/// The turns at the corners of a convex polygon (counterclockwise), radians.
pub(crate) fn turns(v: &[[f64; 2]]) -> Vec<f64> {
    let n = v.len();
    (0..n)
        .map(|i| {
            let (a, b, c) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
            let (e1, e2) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
            (e1[0] * e2[1] - e1[1] * e2[0]).atan2(e1[0] * e2[0] + e1[1] * e2[1])
        })
        .collect()
}

/// The area a corner turning by φ adds to a convex polygon grown by d, over d²: the sector
/// φ/2 (round), the kite tan(φ/2) (miter), less the triangle beyond the cut where the miter
/// reaches farther than `limit` d, (1/cos(φ/2) − limit)² cot(φ/2) (square: limit 1).
fn corner(phi: f64, join: Join) -> f64 {
    let limit = match join {
        Join::Round => return 0.5 * phi,
        Join::Miter { limit } => limit,
        _ => 1.0,
    };
    let half = 0.5 * phi;
    let reach = 1.0 / half.cos();
    let kite = half.tan();
    if reach <= limit {
        kite
    } else {
        kite - (reach - limit).powi(2) / half.tan()
    }
}

/// Convex polygons: a rectangle, a regular hexagon and a triangle with a 25° corner.
pub(crate) fn convex() -> Vec<(&'static str, Vec<[f64; 2]>)> {
    let hexagon: Vec<[f64; 2]> = (0..6)
        .map(|k| {
            let t = TAU * f64::from(k) / 6.0 + 0.1;
            [0.2 + 1.1 * t.cos(), -0.3 + 1.1 * t.sin()]
        })
        .collect();
    let a = 25f64.to_radians();
    vec![
        (
            "rectangle",
            vec![[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]],
        ),
        ("hexagon", hexagon),
        (
            "sharp triangle",
            vec![
                [0.0, 0.0],
                [3.0, 0.0],
                [3.0 * a.cos() * 0.8, 3.0 * a.sin() * 0.8],
            ],
        ),
    ]
}

/// Offsets against closed forms: convex polygons grown by d with round, miter (limits 2 and 1.5)
/// and square joins, A + P d + d² Σ J(φ), J the corner's piece over d² ([`corner`]; Steiner's
/// A + P d + π d² for round joins); a disk and an ellipse grown (Steiner's formula, the ellipse's
/// perimeter by Gauss–Legendre) and shrunk (a disk: π (r − d)²); a rectangle shrunk, (w − 2d)
/// (h − 2d) with any join. Each as a region and as polygons at 1 nm unsnapped (their area
/// difference over the tolerance times the length): the largest relative difference of the
/// regions' areas, and the largest of the polygons' over 1.
pub(crate) fn offset_errors() -> (f64, f64) {
    let tol = Tolerance::unsnapped(Length::nm(1.0)).expect("1 nm");
    let (mut region, mut poly) = (0.0f64, 0.0f64);
    let mut check = |s: Result<Shape>, exact: f64| {
        let Ok(s) = s else {
            region = f64::INFINITY;
            return;
        };
        region = region.max(((s.area() - exact) / exact).abs());
        let length = s.perimeter().map_or(f64::INFINITY, Length::to_um);
        poly = poly.max((s.polygons(tol).area() - exact).abs() / (1e-3 * length));
    };
    let d = 0.15;
    for (_, v) in convex() {
        let s = polygon(&v);
        let (a, p) = (s.area(), s.perimeter().map_or(0.0, Length::to_um));
        for join in [
            Join::Round,
            Join::miter(2.0),
            Join::miter(1.5),
            Join::Square,
        ] {
            let pieces: f64 = turns(&v).into_iter().map(|phi| corner(phi, join)).sum();
            check(s.offset(um(d), join), a + p * d + d * d * pieces);
        }
    }
    let disk = circle(0.3, -0.2, 0.9);
    check(disk.offset(um(d), Join::Round), PI * (0.9 + d).powi(2));
    check(disk.offset(um(-d), Join::Round), PI * (0.9 - d).powi(2));
    let ellipse = Shape::ellipse(Point::um(0.1, 0.2), [um(1.3), um(0.6)], 0.4).expect("an ellipse");
    let (a, p) = (
        ellipse.area(),
        ellipse.perimeter().map_or(0.0, Length::to_um),
    );
    check(ellipse.offset(um(d), Join::Round), a + p * d + PI * d * d);
    let r = rect(0.0, 0.0, 2.0, 1.0);
    for join in [Join::Round, Join::miter(2.0), Join::Square] {
        check(r.offset(um(-d), join), (2.0 - 2.0 * d) * (1.0 - 2.0 * d));
    }
    (region, poly)
}

/// The area a fillet of radius r cuts from a corner turning by φ, over r²: the corner's kite
/// less its sector, tan(φ/2) − φ/2.
pub(crate) fn fillet(phi: f64) -> f64 {
    (0.5 * phi).tan() - 0.5 * phi
}

/// Fillets against the corner cuts: the convex polygons' convex corners rounded to r, A − r²
/// Σ (tan(φ/2) − φ/2); an L's concave corner rounded to r (adding r²(1 − π/4)), and both at
/// once; a rectangle rounded against the rounded rectangle it becomes. As regions and as
/// polygons at 1 nm unsnapped: (the largest relative difference of the regions' areas, the
/// largest of the polygons' over the tolerance times the length).
pub(crate) fn fillet_errors() -> (f64, f64) {
    let tol = Tolerance::unsnapped(Length::nm(1.0)).expect("1 nm");
    let (mut region, mut poly) = (0.0f64, 0.0f64);
    let mut check = |s: Result<Shape>, exact: f64| {
        let Ok(s) = s else {
            region = f64::INFINITY;
            return;
        };
        region = region.max(((s.area() - exact) / exact).abs());
        let length = s.perimeter().map_or(f64::INFINITY, Length::to_um);
        poly = poly.max((s.polygons(tol).area() - exact).abs() / (1e-3 * length));
    };
    let r = 0.12;
    for (_, v) in convex() {
        let s = polygon(&v);
        let cut: f64 = turns(&v).into_iter().map(fillet).sum();
        check(s.rounded(um(r), um(0.0)), s.area() - r * r * cut);
    }
    let l = polygon(&[
        [0.0, 0.0],
        [2.0, 0.0],
        [2.0, 1.0],
        [1.0, 1.0],
        [1.0, 2.0],
        [0.0, 2.0],
    ]);
    let quarter = fillet(0.5 * PI);
    check(l.rounded(um(0.0), um(r)), 3.0 + r * r * quarter);
    check(l.rounded(um(r), um(0.0)), 3.0 - 5.0 * r * r * quarter);
    check(
        l.rounded(um(r), um(0.2)),
        3.0 - 5.0 * r * r * quarter + 0.04 * quarter,
    );
    let rr =
        Shape::rounded_rect(Point::um(1.0, 0.5), um(2.0), um(1.0), um(r), 0.0).expect("a rect");
    check(rect(0.0, 0.0, 2.0, 1.0).rounded(um(r), um(0.0)), rr.area());
    (region, poly)
}

/// The largest distance of polygons' vertices and their edges' middles from a region's
/// boundary, µm, and how many vertices they have.
pub(crate) fn deviation(region: &dyn Region, p: &Polygons) -> (f64, usize) {
    let mut worst = 0.0f64;
    let mut count = 0;
    for c in &p.contours {
        for ring in std::iter::once(&c.outer).chain(&c.holes) {
            let n = ring.len();
            count += n;
            for i in 0..n {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                let mid = Point {
                    x: (a.x + b.x) / 2.0,
                    y: (a.y + b.y) / 2.0,
                };
                for q in [a, mid] {
                    worst = worst.max(region.distance(q).signed.to_um().abs());
                }
            }
        }
    }
    (worst, count)
}

/// The order of the polygons' distance from the boundary in their number of vertices, for the
/// union of two disks, an ellipse grown with round joins and a rectangle rounded (against the
/// rounded rectangle it is), at unsnapped tolerances from 1 to 0.001 nm: the least slope of
/// −log distance against log vertices of the three.
pub(crate) fn order() -> f64 {
    let rounded =
        Shape::rounded_rect(Point::um(1.0, 0.5), um(2.0), um(1.0), um(0.3), 0.0).expect("a rect");
    // each shape, and the exact region its polygons are measured against (itself, where its
    // distance is exact)
    let shapes: Vec<(Result<Shape>, Option<Shape>)> = vec![
        (circle(0.0, 0.0, 1.0).union(&circle(1.2, 0.3, 0.8)), None),
        (
            Shape::ellipse(Point::um(0.1, 0.2), [um(1.3), um(0.6)], 0.4)
                .and_then(|e| e.offset(um(0.2), Join::Round)),
            None,
        ),
        (
            rect(0.0, 0.0, 2.0, 1.0).rounded(um(0.3), um(0.0)),
            Some(rounded),
        ),
    ];
    let mut least = f64::INFINITY;
    for (s, reference) in shapes {
        let Ok(s) = s else {
            return f64::NAN;
        };
        let mut points = Vec::new();
        for nm in [1.0, 0.1, 0.01, 0.001] {
            let p = s.polygons(Tolerance::unsnapped(Length::nm(nm)).expect("a tolerance"));
            let (d, n) = deviation(reference.as_ref().unwrap_or(&s), &p);
            points.push(((n as f64).ln(), d.ln()));
        }
        let m = points.len() as f64;
        let (sx, sy) = points
            .iter()
            .fold((0.0, 0.0), |(a, b), p| (a + p.0, b + p.1));
        let (mx, my) = (sx / m, sy / m);
        let (num, den) = points.iter().fold((0.0, 0.0), |(a, b), p| {
            (a + (p.0 - mx) * (p.1 - my), b + (p.0 - mx) * (p.0 - mx))
        });
        least = least.min(-num / den);
    }
    least
}

/// Zhang et al.'s Y-junction drawn by union: the taper between y = ∓w(x)/2 (their Table 1's
/// widths, [`super::curve_checks::zhang_taper`]), its input guide (0.5 µm wide, 1 µm long) and
/// its two output guides (0.5 µm wide, 0.2 µm apart, 1 µm long), each sharing an edge with the
/// taper. The largest of: its area against the four's (∫w + 0.5 + 2 × 0.5 µm²), its polygons'
/// at the default tolerance against it over the tolerance times the length, the distance across
/// the 0.2 µm gap between the arms, and its inside test across the shared edges (µm or µm²);
/// infinite unless it is one polygon without holes.
pub(crate) fn zhang_union_error() -> f64 {
    let Ok(taper) = super::curve_checks::zhang_taper() else {
        return f64::INFINITY;
    };
    let taper = Shape::Between(taper);
    let input = rect(-1.0, -0.25, 0.0, 0.25);
    let upper = rect(2.0, 0.1, 3.0, 0.6);
    let lower = rect(2.0, -0.6, 3.0, -0.1);
    let Ok(y) = input
        .union(&taper)
        .and_then(|s| s.union(&upper))
        .and_then(|s| s.union(&lower))
    else {
        return f64::INFINITY;
    };
    let exact = taper.area() + 0.5 + 2.0 * 0.5;
    let t = Tolerance::default();
    let p = y.polygons(t);
    if p.contours.len() != 1 || !p.contours[0].holes.is_empty() {
        return f64::INFINITY;
    }
    let length = y.perimeter().map_or(f64::INFINITY, Length::to_um);
    let mut worst = (y.area() - exact).abs();
    if (p.area() - exact).abs() > t.geometric().to_um() * length {
        return f64::INFINITY;
    }
    // across the gap: halfway between the arms, 0.1 µm from each
    worst = worst.max((y.distance(Point::um(2.5, 0.0)).signed.to_um() - 0.1).abs());
    // the shared edges are inside; the taper's end between the arms is its boundary
    let inside = [(0.0, 0.0), (0.0, 0.2), (2.0, 0.3), (2.0, -0.4)]
        .iter()
        .all(|&(x, yy)| y.locate(Point::um(x, yy)) == Location::Inside);
    let boundary = y.locate(Point::um(2.0, 0.0)) == Location::Boundary
        && y.locate(Point::um(2.0 + 1e-9, 0.0)) == Location::Outside;
    if !inside || !boundary {
        return f64::INFINITY;
    }
    worst
}

fn error(e: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance,
        error: e,
    }
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// The validation cases of booleans, offsets and corner rounding.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "geometry/booleans/areas",
            title: r"Union, intersection, difference and xor of two equal and two unequal crossing disks, two rectangles, a disk on a rectangle's edge tangent to two of its sides, a ring and a half-plane, a square and a diamond, as regions: their areas against the exact lens, overlap and octagon areas, and inclusion–exclusion $\lvert A\cup B\rvert + \lvert A\cap B\rvert = \lvert A\rvert + \lvert B\rvert$, $\lvert A \triangle B\rvert = \lvert A\cup B\rvert - \lvert A\cap B\rvert$, $\lvert A\setminus B\rvert = \lvert A\rvert - \lvert A\cap B\rvert$ (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"the lens $r_1^2\alpha_1 + r_2^2\alpha_2 - \frac12\sqrt{(-d + r_1 + r_2)(d + r_1 - r_2)(d - r_1 + r_2)(d + r_1 + r_2)}$; the regions' areas are their polygons' within $10^{-7}$ of their size, each arc's vertices placed to keep its area, in error where the circles cross by about a chord times that tolerance; measured 1.2e-10",
            run: || error(area_error(), 1e-9),
        },
        Case {
            id: "geometry/booleans/polygons",
            title: r"The same booleans as polygons at the default tolerance (1 nm on a grid of 1 nm): the largest distance of their vertices and edges' middles from the exact boundary (nm, shown), their areas against the exact ones within the tolerance times the boundary's length, every polygon counterclockwise, every hole clockwise and none thinner than the tolerance",
            tier: Tier::Analytic,
            source: r"the operands' polygons within half the budget, overlaid on the 1 nm grid by i_overlay 9.1 (integer coordinates; docs/methods/booleans.md), measured against the composed distance and tightened until within 1 nm; measured 0.82 nm",
            run: || {
                let (d, a, bad) = polygon_errors();
                let mut o = error(d, 1.0);
                if a.is_nan() || a > 1.0 || bad > 0 {
                    o.error = f64::INFINITY;
                }
                o
            },
        },
        Case {
            id: "geometry/booleans/degenerate",
            title: r"Polygon booleans on degenerate pairs at the default tolerance: a shared edge, a touching vertex, a collinear overlap, a vertex on an edge, the same square twice, a gap and an overlap of 0.1 pm (below the grid), a hole, a hole opening on an edge, two frames with holes, edges $10^{-9}$ rad apart, a shared edge 1 cm from the origin: every result's number of polygons and holes as it should be, oriented, without slivers, and its area (largest difference shown, µm²)",
            tier: Tier::Analytic,
            source: r"the exact results: a shared edge merges, a touching vertex keeps two polygons, an intersection along an edge or at a vertex is empty, what is within the grid snaps together, a hole meeting the boundary opens; measured 0",
            run: || error(degenerate_error(), 1e-12),
        },
        Case {
            id: "geometry/booleans/distance",
            title: r"The signed distance of the union, intersection, difference and xor of two crossing disks (radii 1.2 and 0.7 µm) at 225 points about each, against the exact distance to their boundary, arcs of the two circles (largest difference shown, nm); and at 64 points on each circle's arcs that bound the result",
            tier: Tier::Analytic,
            source: r"the operands' distances composed by min and max (A. Ricci, Comput. J. 16, 157 (1973), doi:10.1093/comjnl/16.2.157), exact where the nearest point they give is on the result's boundary; elsewhere the distance to its polygons within $10^{-7}$ of its size (2.9e-4 nm here); measured 1.8e-4 nm off the boundary and 6.3e-13 nm on it",
            run: || {
                let (off, on) = distance_errors();
                let mut o = error(off, 1e-3);
                if on.is_nan() || on > 1e-9 {
                    o.error = f64::INFINITY;
                }
                o
            },
        },
        Case {
            id: "geometry/booleans/offsets",
            title: r"Offsets against closed forms: a rectangle, a regular hexagon and a triangle with a 25° corner grown by 0.15 µm with round, miter (limits 2 and 1.5) and square joins, a disk and an ellipse grown, a disk and a rectangle shrunk, as regions (largest relative difference of their areas shown) and as polygons at 1 nm (their areas within the tolerance times the length)",
            tier: Tier::Analytic,
            source: r"Steiner's formula $A + P d + \pi d^2$ for a convex region grown with round joins; with corners, $A + P d + d^2 \sum J(\varphi)$, a corner turning by $\varphi$ adding $\varphi/2$ (round), $\tan\frac\varphi2$ (miter), less $(\sec\frac\varphi2 - L)^2 \cot\frac\varphi2$ when cut at $L$ times the distance (square: $L = 1$); $\pi(r - d)^2$, $(w - 2d)(h - 2d)$ shrunk; measured 4.0e-8, from the curves' polygons within $10^{-7}$ of their size",
            run: || {
                let (region, poly) = offset_errors();
                let mut o = error(region, 1e-6);
                if poly.is_nan() || poly > 1.0 {
                    o.error = f64::INFINITY;
                }
                o
            },
        },
        Case {
            id: "geometry/booleans/fillets",
            title: r"Corner rounding against the corner cuts: a rectangle, a regular hexagon and a triangle with a 25° corner with their convex corners rounded to 0.12 µm, an L's concave corner rounded, its convex ones, and both, and the rounded rectangle against the primitive, as regions (largest relative difference shown) and as polygons at 1 nm",
            tier: Tier::Analytic,
            source: r"opening and closing (R. M. Haralick, S. R. Sternberg, X. Zhuang, IEEE Trans. Pattern Anal. Mach. Intell. 9, 532 (1987), doi:10.1109/TPAMI.1987.4767941): a fillet of radius $r$ in a corner turning by $\varphi$ cuts $r^2(\tan\frac\varphi2 - \frac\varphi2)$ from a convex corner and adds it to a concave one; the regions from polygons within $10^{-7}$ of their size, offset four times for both radii; measured 2.3e-9",
            run: || {
                let (region, poly) = fillet_errors();
                let mut o = error(region, 1e-8);
                if poly.is_nan() || poly > 1.0 {
                    o.error = f64::INFINITY;
                }
                o
            },
        },
        Case {
            id: "geometry/booleans/convergence",
            title: r"The union of two disks, an ellipse grown with round joins and a rectangle with its corners rounded, as polygons at unsnapped tolerances from 1 to 0.001 nm: the order of the distance they reach from the exact boundary in their number of vertices, the slope of its $\log$ against $\log$ vertices (the least of the three, shown)",
            tier: Tier::Analytic,
            source: r"each arc's chords of length $h$ stray by $\kappa h^2/8$, and the operations move no vertex farther: second order (the slope shown to two decimals)",
            run: || outcome((order() * 100.0).round() / 100.0, 2.0, 0.15),
        },
        Case {
            id: "geometry/booleans/zhang-y-junction",
            title: r"Zhang et al.'s Y-junction drawn as a union: its taper (2 µm long, $w_1$ to $w_{13}$ of their Table 1, between $y = \mp w(x)/2$), its 0.5 µm input guide and its two 0.5 µm output guides 0.2 µm apart, each 1 µm long and sharing an edge with the taper: one polygon without holes, its area against the four's, its polygons' at 1 nm within the tolerance, the 0.2 µm gap between the arms kept (the distance halfway across it, 0.1 µm) and the shared edges inside (largest difference shown, µm²)",
            tier: Tier::Published,
            source: r"Y. Zhang et al., Opt. Express 21, 1310 (2013), doi:10.1364/OE.21.001310, Table 1 and Fig. 1(a): input 0.5 µm, output two 0.5 µm arms with a 0.2 µm gap (1.2 µm); the union's shared edges are where the polygon booleans meet collinear overlaps; measured 1.3e-14",
            run: || error(zhang_union_error(), 1e-12),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "writes the cases' rows of the report to PHOTONOXIDE_BOOLEAN_ROWS"]
    fn rows() {
        let rows: String = cases()
            .iter()
            .map(|c| crate::validation::row(c).0)
            .collect();
        if let Some(out) = std::env::var_os("PHOTONOXIDE_BOOLEAN_ROWS") {
            std::fs::write(out, rows).expect("the rows are written");
        }
    }

    #[test]
    #[ignore = "prints the cases' values; cargo test --release --lib boolean_checks -- --ignored --nocapture"]
    fn print() {
        for case in cases() {
            let start = std::time::Instant::now();
            let o = (case.run)();
            println!(
                "{}: measured {:.6e} expected {} error {:.3e} tolerance {} {} ({:.2} s)",
                case.id,
                o.measured,
                o.expected,
                o.error,
                o.tolerance,
                o.passed(),
                start.elapsed().as_secs_f64()
            );
        }
        println!("polygons {:?}", polygon_errors());
        println!("distances {:?}", distance_errors());
        println!("offsets {:?}", offset_errors());
        println!("fillets {:?}", fillet_errors());
    }
}
