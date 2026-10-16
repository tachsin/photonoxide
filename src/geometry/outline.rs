//! A path's outline and what a region answers from it.
//!
//! The outline is one loop, the region on its left: the right edge forward, the end's flat
//! cap, the left edge back, the start's cap. Each edge is c(u) ± (w/2) n(u), the centre line
//! offset along its normal by half the width. Along a straight or an arc of constant width the
//! edges are a segment and arcs of circles, kept exact ([`Piece`]); elsewhere they are curves,
//! one piece per interval between two knots of the centre line.
//!
//! - **Inside:** a point p is in the region when it is c(u) + v n(u) with |v| ≤ w(u)/2 for some
//!   u: its foot on the centre line, where g(u) = (p − c(u))·t(u) = 0, found in each interval
//!   where g falls through zero, by bracketed root finding to the last bit. g′ = −(1 − vκ)
//!   s′(u) < 0 wherever |v| < 1/|κ|, and the knots are closer than a quarter of the gap
//!   between the inner edge and the centre of curvature, so an interval near the region holds
//!   at most one foot and its bracket shows it.
//! - **Distance:** the nearest point of the outline: of each exact piece in closed form, of each
//!   curve piece at its ends or where (p − E(u))·E′(u) falls through zero; the pieces searched
//!   nearest first through a bounding-volume hierarchy.
//! - **Polygons:** each curve piece's chords halved until the curve strays from each by at
//!   most 3/4 of the budget, then each vertex inside a run of curve chords moved out along the
//!   normal by λ times the mean of its chords' sagittas, λ (about 2/3) the root of the quadratic
//!   that gives the polygon the path's area, ∫ w ds, exactly; the arcs flattened as every
//!   shape's are ([`boundary::flatten`]), keeping their area.

use super::boundary::{self, Conic, P, Piece};
use super::index::Index;
use super::path::{Part, Path, Profile};
use super::primitives::point;
use super::region::{Distance, Location, Polygons, Tolerance, assemble, fill_of};
use super::{Bounds, Point, Region, bounds_um};
use crate::units::Length;
use crate::{Error, Result};

use std::f64::consts::{FRAC_PI_2, PI};

/// A piece of the outline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Edge {
    /// A segment or an arc of a circle.
    Exact(Piece),
    /// The curve c(u) + side (w/2) n(u) of `part` between its knots `k` and `k + 1`, walked
    /// with u increasing (`forward`) or decreasing.
    Curve {
        part: usize,
        side: f64,
        k: usize,
        forward: bool,
    },
}

/// A path's outline, with the hierarchies its queries search.
#[derive(Clone, Debug)]
pub(crate) struct Outline {
    edges: Vec<Edge>,
    edge_boxes: Vec<Bounds>,
    edge_index: Index,
    /// Every interval of the centre line, (part, knot), and their boxes widened by the half
    /// width.
    centre: Vec<(usize, usize)>,
    centre_index: Index,
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}

fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn dist(a: P, b: P) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// A point of an edge and its derivative with respect to u.
pub(crate) fn edge_at(part: &Part, profile: Option<&Profile>, side: f64, u: f64) -> (P, P) {
    let at = part.at(u);
    let (w, ws) = part.width(profile, u);
    let n = at.n();
    let h = side * 0.5 * w;
    let e = [at.p[0] + h * n[0], at.p[1] + h * n[1]];
    let a = 1.0 - h * at.kappa;
    let b = side * 0.5 * ws;
    let d = [
        at.speed * (a * at.t[0] + b * n[0]),
        at.speed * (a * at.t[1] + b * n[1]),
    ];
    (e, d)
}

/// A root of `f` between `a` and `b`, where f(a) and f(b) have opposite signs, by the Illinois
/// method: superlinear, and never leaving the bracket. Stops when the bracket can't shrink.
pub(crate) fn root(
    f: impl Fn(f64) -> f64,
    mut a: f64,
    mut b: f64,
    mut fa: f64,
    mut fb: f64,
) -> f64 {
    let mut side = 0i8;
    let mut x = a;
    for _ in 0..200 {
        let denom = fb - fa;
        x = if denom == 0.0 {
            0.5 * (a + b)
        } else {
            (a * fb - b * fa) / denom
        };
        if !(x > a.min(b) && x < a.max(b)) {
            x = 0.5 * (a + b);
            if x == a || x == b {
                break;
            }
        }
        let fx = f(x);
        if fx == 0.0 {
            return x;
        }
        if (fx > 0.0) == (fb > 0.0) {
            b = x;
            fb = fx;
            if side == -1 {
                fa *= 0.5;
            }
            side = -1;
        } else {
            a = x;
            fa = fx;
            if side == 1 {
                fb *= 0.5;
            }
            side = 1;
        }
        if (b - a).abs() <= 2.0 * f64::EPSILON * a.abs().max(b.abs()) {
            break;
        }
    }
    x
}

/// The box of `points` widened by `margin`, and by a hair for rounding.
fn widened(points: &[P], margin: f64) -> Bounds {
    let b = bounds_um(points);
    let [lo, hi] = b.corners();
    let scale = lo.iter().chain(&hi).fold(1.0f64, |m, v| m.max(v.abs()));
    let m = margin + 1e-12 * scale;
    Bounds {
        min: Point::um(lo[0] - m, lo[1] - m),
        max: Point::um(hi[0] + m, hi[1] + m),
    }
}

/// An arc of a circle's points that bound it: its ends and its extremes along x and y.
fn conic_points(c: &Conic) -> Vec<P> {
    let at = |t: f64| [c.c[0] + c.rx * t.cos(), c.c[1] + c.ry * t.sin()];
    let (lo, hi) = (c.t0.min(c.t1), c.t0.max(c.t1));
    let mut points = vec![at(c.t0), at(c.t1)];
    for k in 0..4 {
        let t = f64::from(k) * FRAC_PI_2;
        let t = t + 2.0 * PI * ((lo - t) / (2.0 * PI)).ceil();
        if t <= hi {
            points.push(at(t));
        }
    }
    points
}

/// The exact pieces of a straight or an arc of constant width: its right edge forward and its
/// left edge back.
fn exact_pieces(part: &Part) -> (Piece, Piece) {
    let w = part.w[0];
    let start = part.at(0.0);
    let end = part.at(part.umax);
    match part.kind {
        super::path::Kind::Arc { radius, angle } => {
            let (sin, cos) = (part.sin, part.cos);
            let circle = |r: f64, t0: f64, t1: f64, c: P| {
                Piece::Conic(Conic {
                    c,
                    rx: r,
                    ry: r,
                    cos: 1.0,
                    sin: 0.0,
                    t0,
                    t1,
                })
            };
            if angle > 0.0 {
                let c = [part.o[0] - radius * sin, part.o[1] + radius * cos];
                let t0 = part.heading - FRAC_PI_2;
                let t1 = t0 + angle;
                (
                    circle(radius + 0.5 * w, t0, t1, c),
                    circle(radius - 0.5 * w, t1, t0, c),
                )
            } else {
                let c = [part.o[0] + radius * sin, part.o[1] - radius * cos];
                let t0 = part.heading + FRAC_PI_2;
                let t1 = t0 + angle;
                (
                    circle(radius - 0.5 * w, t0, t1, c),
                    circle(radius + 0.5 * w, t1, t0, c),
                )
            }
        }
        _ => {
            let n = start.n();
            let h = 0.5 * w;
            let off = |c: P, s: f64| [c[0] + s * h * n[0], c[1] + s * h * n[1]];
            (
                Piece::Line(off(start.p, -1.0), off(end.p, -1.0)),
                Piece::Line(off(end.p, 1.0), off(start.p, 1.0)),
            )
        }
    }
}

/// A node of an outline being turned into a polygon: a vertex, with the direction it may move
/// in (zero: fixed), or an arc kept exact.
#[derive(Clone, Copy, Debug)]
enum Node {
    V(P, P),
    C(Conic),
}

/// A chord between two curve vertices: the node it starts at and the piece of curve it
/// replaces.
#[derive(Clone, Copy, Debug)]
struct Chord {
    node: usize,
    part: usize,
    side: f64,
    u0: f64,
    u1: f64,
}

impl Outline {
    pub(crate) fn new(parts: &[Part], profile: Option<&Profile>, _wmax: f64) -> Result<Outline> {
        let mut edges = Vec::new();
        let mut lefts: Vec<Vec<Edge>> = Vec::with_capacity(parts.len());
        for (i, part) in parts.iter().enumerate() {
            let nk = part.knots.len() - 1;
            if part.exact(profile) {
                let (right, left) = exact_pieces(part);
                edges.push(Edge::Exact(right));
                lefts.push(vec![Edge::Exact(left)]);
            } else {
                edges.extend((0..nk).map(|k| Edge::Curve {
                    part: i,
                    side: -1.0,
                    k,
                    forward: true,
                }));
                lefts.push(
                    (0..nk)
                        .rev()
                        .map(|k| Edge::Curve {
                            part: i,
                            side: 1.0,
                            k,
                            forward: false,
                        })
                        .collect(),
                );
            }
        }
        let last = parts
            .last()
            .ok_or_else(|| Error::invalid("path", "no parts"))?;
        let end_r = edge_at(last, profile, -1.0, last.umax).0;
        let end_l = edge_at(last, profile, 1.0, last.umax).0;
        edges.push(Edge::Exact(Piece::Line(end_r, end_l)));
        for left in lefts.into_iter().rev() {
            edges.extend(left);
        }
        let start_l = edge_at(&parts[0], profile, 1.0, 0.0).0;
        let start_r = edge_at(&parts[0], profile, -1.0, 0.0).0;
        edges.push(Edge::Exact(Piece::Line(start_l, start_r)));

        let boxes: Vec<Bounds> = edges
            .iter()
            .map(|e| match e {
                Edge::Exact(Piece::Line(a, b)) => widened(&[*a, *b], 0.0),
                Edge::Exact(Piece::Conic(c)) => widened(&conic_points(c), 0.0),
                Edge::Curve { part, side, k, .. } => {
                    let p = &parts[*part];
                    let (a, b) = (p.knots[*k], p.knots[*k + 1]);
                    let pts: Vec<P> = (0..=8)
                        .map(|j| edge_at(p, profile, *side, a + (b - a) * f64::from(j) / 8.0).0)
                        .collect();
                    let gap = pts.windows(2).map(|w| dist(w[0], w[1])).fold(0.0, f64::max);
                    widened(&pts, 0.3 * gap)
                }
            })
            .collect();
        let edge_index = Index::new(&boxes);

        let mut centre = Vec::new();
        let mut centre_boxes = Vec::new();
        for (i, part) in parts.iter().enumerate() {
            for k in 0..part.knots.len() - 1 {
                let (a, b) = (part.knots[k], part.knots[k + 1]);
                let mut pts = Vec::with_capacity(9);
                let mut half = 0.0f64;
                for j in 0..=8 {
                    let u = a + (b - a) * f64::from(j) / 8.0;
                    pts.push(part.at(u).p);
                    half = half.max(0.5 * part.width(profile, u).0);
                }
                let gap = pts.windows(2).map(|w| dist(w[0], w[1])).fold(0.0, f64::max);
                centre.push((i, k));
                centre_boxes.push(widened(&pts, half + 0.3 * gap));
            }
        }
        Ok(Outline {
            edges,
            edge_boxes: boxes,
            edge_index,
            centre_index: Index::new(&centre_boxes),
            centre,
        })
    }
}

impl Path {
    fn parts(&self) -> &[Part] {
        &self.data.parts
    }

    /// Where `p` is: from its feet on the centre line.
    fn locate_um(&self, p: P) -> Location {
        let parts = self.parts();
        let profile = self.profile();
        let outline = &self.data.outline;
        let last = parts.len() - 1;
        let mut boundary = false;
        let inside = outline.centre_index.any_at(point(p), |item| {
            let (i, k) = outline.centre[item];
            let part = &parts[i];
            let (a, b) = (part.knots[k], part.knots[k + 1]);
            let g = |u: f64| {
                let at = part.at(u);
                dot(sub(p, at.p), at.t)
            };
            let (ga, gb) = (g(a), g(b));
            if !(ga >= 0.0 && gb <= 0.0) {
                return false;
            }
            let u = if ga == 0.0 {
                a
            } else if gb == 0.0 {
                b
            } else {
                root(g, a, b, ga, gb)
            };
            let at = part.at(u);
            let v = dot(sub(p, at.p), at.n()).abs();
            let half = 0.5 * part.width(profile, u).0;
            // on a cap: the foot at the path's very start or end
            let cap = (i == 0 && u == 0.0 && ga == 0.0)
                || (i == last && u == b && gb == 0.0 && k + 2 == part.knots.len());
            if v < half && !cap {
                return true;
            }
            if v <= half {
                boundary = true;
            }
            false
        });
        if inside {
            Location::Inside
        } else if boundary {
            Location::Boundary
        } else {
            Location::Outside
        }
    }

    /// The nearest point of an edge to `p`: (distance, point, outward normal there).
    fn nearest_on(&self, edge: &Edge, p: P) -> (f64, P, P) {
        match edge {
            Edge::Exact(piece) => piece.nearest(p),
            Edge::Curve { part, side, k, .. } => {
                let part = &self.parts()[*part];
                let profile = self.profile();
                let (a, b) = (part.knots[*k], part.knots[*k + 1]);
                let side = *side;
                let normal = |d: P| {
                    let l = d[0].hypot(d[1]);
                    [side * -d[1] / l, side * d[0] / l]
                };
                let h = |u: f64| {
                    let (e, d) = edge_at(part, profile, side, u);
                    dot(sub(p, e), d)
                };
                let candidate = |u: f64| {
                    let (e, d) = edge_at(part, profile, side, u);
                    (dist(p, e), e, normal(d))
                };
                let mut best = candidate(a);
                let other = candidate(b);
                if other.0 < best.0 {
                    best = other;
                }
                let (ha, hb) = (h(a), h(b));
                if ha > 0.0 && hb < 0.0 {
                    let inner = candidate(root(h, a, b, ha, hb));
                    if inner.0 <= best.0 {
                        best = inner;
                    }
                }
                best
            }
        }
    }

    /// The nearest point of the outline to `p`: the pieces whose boxes come within the best
    /// distance so far, nearest box first.
    fn nearest(&self, p: P) -> (f64, P, P) {
        let parts = self.parts();
        let outline = &self.data.outline;
        // the caps' middles are on the outline: an upper bound
        let last = &parts[parts.len() - 1];
        let ub = dist(p, parts[0].at(0.0).p).min(dist(p, last.at(last.umax).p));
        let r = ub * (1.0 + 1e-12) + 1e-12;
        let window = Bounds {
            min: Point::um(p[0] - r, p[1] - r),
            max: Point::um(p[0] + r, p[1] + r),
        };
        let mut candidates: Vec<(f64, usize)> = outline
            .edge_index
            .meeting(window)
            .into_iter()
            .map(|e| {
                // a lower bound: the distance to the piece's box
                let [lo, hi] = outline.edge_boxes[e].corners();
                let dx = (lo[0] - p[0]).max(p[0] - hi[0]).max(0.0);
                let dy = (lo[1] - p[1]).max(p[1] - hi[1]).max(0.0);
                (dx.hypot(dy), e)
            })
            .collect();
        candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let mut best = (f64::INFINITY, p, [1.0, 0.0]);
        for (lb, e) in candidates {
            if lb > best.0 {
                break;
            }
            let c = self.nearest_on(&outline.edges[e], p);
            if c.0 < best.0 {
                best = c;
            }
        }
        best
    }

    /// The outline's nodes with curve pieces flattened within `budget` (µm) and arcs flattened
    /// too (`flatten`) or kept, and how far it strays from the outline at most.
    fn nodes(&self, budget: f64, flatten: bool) -> (Vec<Node>, f64) {
        let mut split = 0.75;
        loop {
            let (nodes, dev) = self.attempt(budget, split, flatten);
            if dev <= budget || split < 1e-3 {
                return (nodes, dev);
            }
            split *= 0.5;
        }
    }

    fn attempt(&self, budget: f64, split: f64, flatten: bool) -> (Vec<Node>, f64) {
        let parts = self.parts();
        let profile = self.profile();
        let mut nodes: Vec<Node> = Vec::new();
        let mut chords: Vec<Chord> = Vec::new();
        let mut sags: Vec<f64> = Vec::new();
        let mut dev = 0.0f64;
        // the signed distance of the curve from a chord, outward (to its right) positive, at 7
        // points
        let sag = |part: &Part, side: f64, u0: f64, u1: f64| -> f64 {
            let a = edge_at(part, profile, side, u0).0;
            let b = edge_at(part, profile, side, u1).0;
            let e = sub(b, a);
            let l = e[0].hypot(e[1]);
            let mut worst = 0.0f64;
            for j in 1..8 {
                let s = edge_at(part, profile, side, u0 + (u1 - u0) * f64::from(j) / 8.0).0;
                let d = (e[1] * (s[0] - a[0]) - e[0] * (s[1] - a[1])) / l;
                if d.abs() > worst.abs() {
                    worst = d;
                }
            }
            worst
        };
        for edge in &self.data.outline.edges {
            match edge {
                Edge::Exact(Piece::Line(a, _)) => nodes.push(Node::V(*a, [0.0, 0.0])),
                Edge::Exact(Piece::Conic(c)) => {
                    if flatten {
                        let (v, d) = boundary::flatten(c, budget, false);
                        dev = dev.max(d);
                        nodes.extend(v.into_iter().map(|p| Node::V(p, [0.0, 0.0])));
                    } else {
                        nodes.push(Node::C(*c));
                    }
                }
                Edge::Curve {
                    part,
                    side,
                    k,
                    forward,
                } => {
                    let p = &parts[*part];
                    let (a, b) = (p.knots[*k], p.knots[*k + 1]);
                    let (ua, ub) = if *forward { (a, b) } else { (b, a) };
                    let mut stack = vec![(ua, ub, 0u32)];
                    while let Some((u0, u1, depth)) = stack.pop() {
                        let s = sag(p, *side, u0, u1);
                        if s.abs() > split * budget && depth < 40 {
                            let m = 0.5 * (u0 + u1);
                            stack.push((m, u1, depth + 1));
                            stack.push((u0, m, depth + 1));
                        } else {
                            chords.push(Chord {
                                node: nodes.len(),
                                part: *part,
                                side: *side,
                                u0,
                                u1,
                            });
                            sags.push(s);
                            nodes.push(Node::V(edge_at(p, profile, *side, u0).0, [0.0, 0.0]));
                        }
                    }
                }
            }
        }
        // the vertices inside a run of curve chords move along their outward normal
        let n = nodes.len();
        let mut starts = vec![usize::MAX; n];
        for (c, chord) in chords.iter().enumerate() {
            starts[chord.node] = c;
        }
        for (c, chord) in chords.iter().enumerate() {
            let prev = (chord.node + n - 1) % n;
            let before = starts[prev];
            if before == usize::MAX || chords[before].node != prev {
                continue;
            }
            let part = &parts[chord.part];
            let (_, d) = edge_at(part, profile, chord.side, chord.u0);
            let l = d[0].hypot(d[1]);
            let walk = if chord.u1 > chord.u0 { 1.0 } else { -1.0 };
            // the outward normal: to the right of the direction walked
            let out = [walk * d[1] / l, -walk * d[0] / l];
            let w = 0.5 * (sags[before] + sags[c]);
            if let Node::V(p, _) = nodes[chord.node] {
                nodes[chord.node] = Node::V(p, [w * out[0], w * out[1]]);
            }
        }
        // the area as a0 + a1 λ + a2 λ², against ∫ w ds
        let ends = |node: &Node| -> (P, P, P, P) {
            match node {
                Node::V(p, o) => (*p, *o, *p, *o),
                Node::C(c) => {
                    let piece = Piece::Conic(*c);
                    (piece.start(), [0.0, 0.0], piece.end(), [0.0, 0.0])
                }
            }
        };
        let (mut a0, mut a1, mut a2) = (0.0, 0.0, 0.0);
        for i in 0..n {
            if let Node::C(c) = &nodes[i] {
                a0 += Piece::Conic(*c).x_dy();
            }
            let (_, _, pa, oa) = ends(&nodes[i]);
            let (pb, ob, _, _) = ends(&nodes[(i + 1) % n]);
            let (x0, x1) = (0.5 * (pa[0] + pb[0]), 0.5 * (oa[0] + ob[0]));
            let (y0, y1) = (pb[1] - pa[1], ob[1] - oa[1]);
            a0 += x0 * y0;
            a1 += x0 * y1 + x1 * y0;
            a2 += x1 * y1;
        }
        let deficit = self.area_um() - a0;
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
        for node in &mut nodes {
            if let Node::V(p, o) = node {
                *node = Node::V([p[0] + lambda * o[0], p[1] + lambda * o[1]], [0.0, 0.0]);
            }
        }
        // how far the chords stray from the curve, at 9 points of each
        for chord in &chords {
            let part = &parts[chord.part];
            let a = match nodes[chord.node] {
                Node::V(p, _) => p,
                Node::C(_) => continue,
            };
            let b = match nodes[(chord.node + 1) % n] {
                Node::V(p, _) => p,
                Node::C(c) => Piece::Conic(c).start(),
            };
            let segment = Piece::Line(a, b);
            for j in 0..=8 {
                let s = edge_at(
                    part,
                    profile,
                    chord.side,
                    chord.u0 + (chord.u1 - chord.u0) * f64::from(j) / 8.0,
                )
                .0;
                dev = dev.max(segment.nearest(s).0);
            }
        }
        (nodes, dev)
    }

    /// The outline as one loop of pieces for [`Region::fill`]: arcs exact, curves within
    /// [`super::FILL_TOLERANCE`] of the widest width, keeping the area.
    fn fine(&self) -> &Vec<Vec<Piece>> {
        self.data.fine.get_or_init(|| {
            let (nodes, _) = self.nodes(super::FILL_TOLERANCE * self.data.wmax, false);
            let mut pieces = Vec::with_capacity(nodes.len());
            let mut first: Option<P> = None;
            let mut prev: Option<P> = None;
            for node in &nodes {
                let (s, e) = match node {
                    Node::V(p, _) => (*p, *p),
                    Node::C(c) => {
                        let piece = Piece::Conic(*c);
                        (piece.start(), piece.end())
                    }
                };
                if let Some(q) = prev
                    && q != s
                {
                    pieces.push(Piece::Line(q, s));
                }
                if first.is_none() {
                    first = Some(s);
                }
                if let Node::C(c) = node {
                    pieces.push(Piece::Conic(*c));
                }
                prev = Some(e);
            }
            if let (Some(e), Some(s)) = (prev, first)
                && e != s
            {
                pieces.push(Piece::Line(e, s));
            }
            vec![pieces]
        })
    }

    /// ∫ w ds: the region's area, exactly where the width is linear along each part, by
    /// Gauss–Legendre on each interval between knots under a profile.
    pub(crate) fn area_um(&self) -> f64 {
        let profile = self.profile();
        self.parts()
            .iter()
            .map(|part| match profile {
                None => 0.5 * (part.w[0] + part.w[1]) * part.len,
                Some(_) => part
                    .knots
                    .windows(2)
                    .map(|k| {
                        boundary::gauss(|u| part.width(profile, u).0 * part.at(u).speed, k[0], k[1])
                    })
                    .sum(),
            })
            .sum()
    }

    /// Whether the outline crosses or touches itself, checked on its polygon within a 32nd of
    /// the narrowest width.
    pub(crate) fn check_simple(&self) -> Result<()> {
        let (nodes, _) = self.nodes(self.data.wmin / 32.0, true);
        let ring: Vec<P> = nodes
            .iter()
            .filter_map(|n| match n {
                Node::V(p, _) => Some(*p),
                Node::C(_) => None,
            })
            .collect();
        let n = ring.len();
        let boxes: Vec<Bounds> = (0..n)
            .map(|i| widened(&[ring[i], ring[(i + 1) % n]], 0.0))
            .collect();
        let index = Index::new(&boxes);
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            for j in index.meeting(boxes[i]) {
                if j <= i || j == i + 1 || (i == 0 && j == n - 1) {
                    continue;
                }
                let (c, d) = (ring[j], ring[(j + 1) % n]);
                if segments_meet(a, b, c, d) {
                    return Err(Error::invalid(
                        "path",
                        format!(
                            "its outline crosses itself near ({:.4}, {:.4}) um: its parts come \
                             closer than its width",
                            c[0], c[1]
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Whether the closed segments ab and cd share a point.
pub(crate) fn segments_meet(a: P, b: P, c: P, d: P) -> bool {
    let orient = |p: P, q: P, r: P| {
        let v = (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
        if v > 0.0 {
            1
        } else if v < 0.0 {
            -1
        } else {
            0
        }
    };
    let within = |p: P, q: P, r: P| {
        r[0] >= p[0].min(q[0])
            && r[0] <= p[0].max(q[0])
            && r[1] >= p[1].min(q[1])
            && r[1] <= p[1].max(q[1])
    };
    let (o1, o2, o3, o4) = (
        orient(a, b, c),
        orient(a, b, d),
        orient(c, d, a),
        orient(c, d, b),
    );
    if o1 != o2 && o3 != o4 && o1 * o2 <= 0 && o3 * o4 <= 0 && (o1 != 0 || o2 != 0) {
        return true;
    }
    (o1 == 0 && within(a, b, c))
        || (o2 == 0 && within(a, b, d))
        || (o3 == 0 && within(c, d, a))
        || (o4 == 0 && within(c, d, b))
}

impl Region for Path {
    fn locate(&self, p: Point) -> Location {
        self.locate_um([p.x.to_um(), p.y.to_um()])
    }

    fn distance(&self, p: Point) -> Distance {
        let q = [p.x.to_um(), p.y.to_um()];
        let inside = self.locate_um(q) != Location::Outside;
        Distance::from(boundary::signed(self.nearest(q), q, inside))
    }

    /// Tight: the exact pieces' ends and extremes, and each curve piece's ends and the points
    /// where its tangent is along x or y, found to the last bit.
    fn bounds(&self) -> Bounds {
        *self.data.bounds.get_or_init(|| self.tight_bounds())
    }

    fn polygons(&self, tolerance: Tolerance) -> Polygons {
        let (nodes, dev) = self.nodes(tolerance.budget(), true);
        let ring: Vec<P> = nodes
            .iter()
            .filter_map(|n| match n {
                Node::V(p, _) => Some(*p),
                Node::C(_) => None,
            })
            .collect();
        assemble(vec![(ring, Vec::new())], dev, tolerance)
    }

    /// ∫ w ds (the map (s, v) ↦ c(s) + v n(s) has Jacobian 1 − vκ, whose integral over
    /// |v| ≤ w/2 is w).
    fn area(&self) -> f64 {
        self.area_um()
    }

    /// The edges' lengths, exact along straights and arcs of constant width and by
    /// Gauss–Legendre between knots elsewhere, and the caps' widths.
    fn perimeter(&self) -> Option<Length> {
        let parts = self.parts();
        let profile = self.profile();
        let total: f64 = self
            .data
            .outline
            .edges
            .iter()
            .map(|e| match e {
                Edge::Exact(Piece::Line(a, b)) => dist(*a, *b),
                Edge::Exact(Piece::Conic(c)) => c.rx * (c.t1 - c.t0).abs(),
                Edge::Curve { part, side, k, .. } => {
                    let p = &parts[*part];
                    boundary::gauss(
                        |u| {
                            let d = edge_at(p, profile, *side, u).1;
                            d[0].hypot(d[1])
                        },
                        p.knots[*k],
                        p.knots[*k + 1],
                    )
                }
            })
            .sum();
        Some(Length::um(total))
    }

    /// Exact along straights and arcs of constant width; elsewhere the outline within
    /// [`super::FILL_TOLERANCE`] of the widest width, which keeps the area.
    fn fill(&self, cell: Bounds) -> f64 {
        fill_of(self.fine(), self.bounds(), self.area_um(), cell)
    }
}

impl Path {
    /// Tight bounds: the exact pieces' ends and extremes, and each curve piece's ends and the
    /// points where its tangent is along x or y, found to the last bit.
    fn tight_bounds(&self) -> Bounds {
        let parts = self.parts();
        let profile = self.profile();
        let mut points: Vec<P> = Vec::new();
        for edge in &self.data.outline.edges {
            match edge {
                Edge::Exact(Piece::Line(a, b)) => points.extend([*a, *b]),
                Edge::Exact(Piece::Conic(c)) => points.extend(conic_points(c)),
                Edge::Curve { part, side, k, .. } => {
                    let p = &parts[*part];
                    let (a, b) = (p.knots[*k], p.knots[*k + 1]);
                    let us: Vec<f64> = (0..=8).map(|j| a + (b - a) * f64::from(j) / 8.0).collect();
                    points.push(edge_at(p, profile, *side, a).0);
                    points.push(edge_at(p, profile, *side, b).0);
                    for axis in 0..2 {
                        let f = |u: f64| edge_at(p, profile, *side, u).1[axis];
                        for w in us.windows(2) {
                            let (fa, fb) = (f(w[0]), f(w[1]));
                            if fa == 0.0 {
                                points.push(edge_at(p, profile, *side, w[0]).0);
                            } else if (fa > 0.0) != (fb > 0.0) && fb != 0.0 {
                                let u = root(f, w[0], w[1], fa, fb);
                                points.push(edge_at(p, profile, *side, u).0);
                            }
                        }
                    }
                }
            }
        }
        bounds_um(&points)
    }
}
