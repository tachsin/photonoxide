//! Regions bounded by curves given as functions: what a region between two curves, a polar
//! region, a closed parametric curve and an implicit region share.
//!
//! The boundary is loops of curves, each walked with the region on its left (outer boundaries
//! counterclockwise, holes clockwise). Each curve is cut into intervals over which it turns by
//! at most 0.1 rad and is monotone in x and in y (cut where x′ or y′ changes sign, found to the
//! last bit), so that:
//!
//! - an interval's box is its end points' box, and the region's bounds those of every
//!   interval's ends: tight;
//! - a horizontal ray meets an interval at most once: the inside test counts the crossings of
//!   the ray from the point towards +x with the exact curve (each found by bracketed root
//!   finding), even-odd, through a bounding-volume hierarchy over the intervals;
//! - the nearest point of an interval is one of its ends or a root of (p − c)·c′ where it falls
//!   through zero;
//! - the area, ∮ x dy, and the perimeter, ∮ |c′|, are Gauss–Legendre integrals over each
//!   interval: to round-off for curves smooth over an interval.
//!
//! Polygons follow the paths' and the superellipse's way: each interval's chords halved until
//! the curve strays from each by at most 3/4 of the budget, then each vertex not at a corner
//! moved out along the normal by λ times the mean of its chords' signed sagittas, λ the root
//! (near 2/3) of the quadratic that gives each loop its exact area. The distance from the curve
//! is measured again at nine points of each chord, and the chords tightened if it must.

use std::sync::OnceLock;

use super::boundary::{self, P, Piece};
use super::index::Index;
use super::outline::{root, segments_meet};
use super::region::{Location, Polygons, Tolerance, assemble, fill_of};
use super::{Bounds, Point, bounds_um};

/// The largest turn of the boundary over one interval, radians.
const TURN: f64 = 0.1;

/// The polygon [`Trace::fill`] clips strays from the boundary by at most this much of the
/// region's larger side.
pub(crate) const FILL_TOLERANCE: f64 = super::FILL_TOLERANCE;

/// What a region bounded by curves gives the engine: each curve's points.
pub(crate) trait Curves {
    /// The point of curve `curve` at parameter `u`, and its derivative with respect to u, µm,
    /// in the plane.
    fn at(&self, curve: usize, u: f64) -> (P, P);
}

/// A stretch of a loop: `curve` walked from `u0` to `u1` (either way), whether its start is a
/// corner (where polygons keep their vertex on the boundary), and how many equal pieces it is cut
/// into before they are halved where it turns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Span {
    pub curve: usize,
    pub u0: f64,
    pub u1: f64,
    pub corner: bool,
    pub pieces: u32,
}

/// A piece of a loop between two knots: monotone in x and y, turning by at most [`TURN`].
#[derive(Clone, Copy, Debug, PartialEq)]
struct Interval {
    curve: usize,
    u0: f64,
    u1: f64,
    corner: bool,
}

impl Interval {
    fn u(&self, tau: f64) -> f64 {
        if tau == 1.0 {
            self.u1
        } else {
            self.u0 + tau * (self.u1 - self.u0)
        }
    }
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

/// The unit normal to the right of a direction: outward, the region being on the left.
fn right(d: P) -> P {
    let l = d[0].hypot(d[1]);
    if l > 0.0 {
        [d[1] / l, -d[0] / l]
    } else {
        [1.0, 0.0]
    }
}

/// The signed distance of `s` from the chord ab, positive to its right (outward).
fn offset(a: P, b: P, s: P) -> f64 {
    let e = sub(b, a);
    let l = e[0].hypot(e[1]);
    if l == 0.0 {
        return 0.0;
    }
    (e[1] * (s[0] - a[0]) - e[0] * (s[1] - a[1])) / l
}

/// A region's boundary, traced: its loops cut into intervals, with the hierarchy its queries
/// search, each loop's area and which loops are holes of which.
#[derive(Debug)]
pub(crate) struct Trace {
    loops: Vec<Vec<Interval>>,
    /// Each interval's start; an interval ends where the next starts, so every loop closes.
    starts: Vec<Vec<P>>,
    /// The intervals in the hierarchy's order: (loop, interval).
    items: Vec<(usize, usize)>,
    boxes: Vec<Bounds>,
    index: Index,
    /// Each loop's signed area, µm²: positive for an outer boundary, negative for a hole.
    areas: Vec<f64>,
    /// The outer loops, each with its holes.
    groups: Vec<(usize, Vec<usize>)>,
    bounds: Bounds,
    fine: OnceLock<Vec<Vec<Piece>>>,
}

impl Trace {
    /// The boundary `loops` of `curves`, each a list of spans walked in order with the region on
    /// their left.
    ///
    /// # Errors
    ///
    /// The reason, for a curve that stops (a zero derivative) or isn't finite, or a hole that no
    /// outer loop holds.
    pub(crate) fn new<C: Curves + ?Sized>(
        curves: &C,
        loops: &[Vec<Span>],
    ) -> Result<Trace, String> {
        let mut out: Vec<Vec<Interval>> = Vec::with_capacity(loops.len());
        for spans in loops {
            let mut intervals = Vec::new();
            for span in spans.iter().filter(|s| s.u0 != s.u1) {
                let first = intervals.len();
                split(curves, span, &mut intervals)?;
                if let Some(i) = intervals.get_mut(first) {
                    i.corner = span.corner;
                }
            }
            if !intervals.is_empty() {
                out.push(intervals);
            }
        }
        if out.is_empty() {
            return Err("its boundary is empty".to_owned());
        }
        let starts: Vec<Vec<P>> = out
            .iter()
            .map(|l| l.iter().map(|i| curves.at(i.curve, i.u0).0).collect())
            .collect();
        let mut items = Vec::new();
        let mut boxes = Vec::new();
        let mut all: Vec<P> = Vec::new();
        for (l, intervals) in out.iter().enumerate() {
            let n = intervals.len();
            for (k, iv) in intervals.iter().enumerate() {
                let mut pts = vec![starts[l][k], starts[l][(k + 1) % n]];
                for j in 1..4 {
                    pts.push(curves.at(iv.curve, iv.u(f64::from(j) / 4.0)).0);
                }
                items.push((l, k));
                boxes.push(bounds_um(&pts));
                all.push(starts[l][k]);
            }
        }
        if !all.iter().all(|p| p[0].is_finite() && p[1].is_finite()) {
            return Err("its boundary isn't finite".to_owned());
        }
        let index = Index::new(&boxes);
        let bounds = bounds_um(&all);
        let mut trace = Trace {
            loops: out,
            starts,
            items,
            boxes,
            index,
            areas: Vec::new(),
            groups: Vec::new(),
            bounds,
            fine: OnceLock::new(),
        };
        trace.areas = (0..trace.loops.len())
            .map(|l| trace.loop_area(curves, l))
            .collect();
        let outers: Vec<usize> = (0..trace.loops.len())
            .filter(|&l| trace.areas[l] > 0.0)
            .collect();
        let mut groups: Vec<(usize, Vec<usize>)> =
            outers.iter().map(|&o| (o, Vec::new())).collect();
        for h in (0..trace.loops.len()).filter(|&l| trace.areas[l] <= 0.0) {
            let p = trace.starts[h][0];
            let holder = outers
                .iter()
                .enumerate()
                .filter(|(_, o)| trace.crossings(curves, Some(**o), p).unwrap_or(false))
                .min_by(|a, b| trace.areas[*a.1].total_cmp(&trace.areas[*b.1]));
            match holder {
                Some((g, _)) => groups[g].1.push(h),
                None => {
                    return Err(format!(
                        "a clockwise loop of its boundary (a hole) near ({:.6}, {:.6}) um lies in \
                         no outer boundary",
                        p[0], p[1]
                    ));
                }
            }
        }
        trace.groups = groups;
        Ok(trace)
    }

    /// The point of interval k of loop l at τ ∈ [0, 1]: its ends the loop's own.
    fn point<C: Curves + ?Sized>(&self, curves: &C, l: usize, k: usize, tau: f64) -> P {
        if tau == 0.0 {
            return self.starts[l][k];
        }
        if tau == 1.0 {
            return self.starts[l][(k + 1) % self.loops[l].len()];
        }
        let iv = &self.loops[l][k];
        curves.at(iv.curve, iv.u(tau)).0
    }

    /// The derivative of interval k of loop l with respect to τ.
    fn derivative<C: Curves + ?Sized>(&self, curves: &C, l: usize, k: usize, tau: f64) -> P {
        let iv = &self.loops[l][k];
        let d = curves.at(iv.curve, iv.u(tau)).1;
        let du = iv.u1 - iv.u0;
        [d[0] * du, d[1] * du]
    }

    /// ∮ x dy over loop l, from a point of it, by Gauss–Legendre on each interval.
    fn loop_area<C: Curves + ?Sized>(&self, curves: &C, l: usize) -> f64 {
        let x0 = self.starts[l][0][0];
        let iv = &self.loops[l];
        let floor = 1e-8 * self.size();
        (0..iv.len())
            .map(|k| {
                integrate(
                    &|t| {
                        let i = &iv[k];
                        let (p, d) = curves.at(i.curve, i.u(t));
                        (p[0] - x0) * d[1] * (i.u1 - i.u0)
                    },
                    floor * floor,
                )
            })
            .sum()
    }

    /// The parameters where the intervals of `curve` start.
    pub(crate) fn knots(&self, curve: usize) -> Vec<f64> {
        self.loops
            .iter()
            .flatten()
            .filter(|i| i.curve == curve)
            .map(|i| i.u0)
            .collect()
    }

    /// The larger side of the bounds, µm.
    fn size(&self) -> f64 {
        let b = self.bounds;
        (b.max.x - b.min.x).to_um().max((b.max.y - b.min.y).to_um())
    }

    /// The region's area, µm²: the loops' signed areas summed.
    pub(crate) fn area(&self) -> f64 {
        self.areas.iter().sum()
    }

    /// The tight bounds: every interval's ends, the extremes among them.
    pub(crate) fn bounds(&self) -> Bounds {
        self.bounds
    }

    /// The boundary's length, by Gauss–Legendre on each interval.
    pub(crate) fn perimeter<C: Curves + ?Sized>(&self, curves: &C) -> f64 {
        self.items
            .iter()
            .map(|&(l, k)| {
                integrate(
                    &|t| {
                        let d = self.derivative(curves, l, k, t);
                        d[0].hypot(d[1])
                    },
                    1e-15 * self.size(),
                )
            })
            .sum()
    }

    /// Whether the ray from `q` towards +x crosses the boundary (or loop `only`) an odd number of
    /// times; `None` when `q` is on it.
    fn crossings<C: Curves + ?Sized>(&self, curves: &C, only: Option<usize>, q: P) -> Option<bool> {
        let hi = self.bounds.max.x.to_um().max(q[0]);
        let window = Bounds {
            min: Point::um(q[0], q[1]),
            max: Point::um(hi, q[1]),
        };
        let mut odd = false;
        for item in self.index.meeting(window) {
            let (l, k) = self.items[item];
            if only.is_some_and(|o| o != l) {
                continue;
            }
            let n = self.loops[l].len();
            let (a, b) = (self.starts[l][k], self.starts[l][(k + 1) % n]);
            if (a[1] > q[1]) == (b[1] > q[1]) {
                continue;
            }
            let x = if a[0].min(b[0]) > q[0] {
                odd = !odd;
                continue;
            } else if a[0].max(b[0]) < q[0] {
                continue;
            } else if a[1] == q[1] {
                a[0]
            } else if b[1] == q[1] {
                b[0]
            } else {
                let g = |t: f64| self.point(curves, l, k, t)[1] - q[1];
                let t = root(g, 0.0, 1.0, a[1] - q[1], b[1] - q[1]);
                self.point(curves, l, k, t)[0]
            };
            if x == q[0] {
                return None;
            }
            if x > q[0] {
                odd = !odd;
            }
        }
        Some(odd)
    }

    /// Where `q` is: by the parity of the crossings of the ray from it towards +x.
    pub(crate) fn locate<C: Curves + ?Sized>(&self, curves: &C, q: P) -> Location {
        if !self.bounds.contains(Point::um(q[0], q[1])) {
            return Location::Outside;
        }
        match self.crossings(curves, None, q) {
            None => Location::Boundary,
            Some(true) => Location::Inside,
            Some(false) => Location::Outside,
        }
    }

    /// The nearest point of interval `item` to `p`: (distance, point, outward normal).
    fn nearest_on<C: Curves + ?Sized>(&self, curves: &C, item: usize, p: P) -> (f64, P, P) {
        let (l, k) = self.items[item];
        let candidate = |t: f64| {
            let q = self.point(curves, l, k, t);
            (dist(p, q), q, right(self.derivative(curves, l, k, t)))
        };
        let mut best = candidate(0.0);
        let end = candidate(1.0);
        if end.0 < best.0 {
            best = end;
        }
        let h = |t: f64| {
            dot(
                sub(p, self.point(curves, l, k, t)),
                self.derivative(curves, l, k, t),
            )
        };
        let mut prev = (0.0, h(0.0));
        for j in 1..=4 {
            let t = f64::from(j) / 4.0;
            let ht = h(t);
            let foot = if ht == 0.0 {
                Some(t)
            } else if prev.1 > 0.0 && ht < 0.0 {
                Some(root(h, prev.0, t, prev.1, ht))
            } else {
                None
            };
            if let Some(foot) = foot {
                let c = candidate(foot);
                if c.0 <= best.0 {
                    best = c;
                }
            }
            prev = (t, ht);
        }
        best
    }

    /// The nearest point of the boundary to `p`: (distance, point, outward normal there), the
    /// intervals tried nearest box first until a box is farther than the best so far.
    pub(crate) fn nearest<C: Curves + ?Sized>(&self, curves: &C, p: P) -> (f64, P, P) {
        let ub = dist(p, self.starts[0][0]);
        let r = ub * (1.0 + 1e-12) + 1e-12;
        let window = Bounds {
            min: Point::um(p[0] - r, p[1] - r),
            max: Point::um(p[0] + r, p[1] + r),
        };
        let mut candidates: Vec<(f64, usize)> = self
            .index
            .meeting(window)
            .into_iter()
            .map(|e| {
                let [lo, hi] = self.boxes[e].corners();
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
            let c = self.nearest_on(curves, e, p);
            if c.0 < best.0 {
                best = c;
            }
        }
        best
    }

    /// The loops as polygons within `budget` (µm), each keeping its area: their vertices, loop by
    /// loop, and the largest distance from the boundary.
    fn rings<C: Curves + ?Sized>(&self, curves: &C, budget: f64) -> (Vec<Vec<P>>, f64) {
        let mut split = 0.75;
        loop {
            let (rings, dev) = self.attempt(curves, budget, split);
            if dev <= budget || split < 1e-3 {
                return (rings, dev);
            }
            split *= 0.5;
        }
    }

    fn attempt<C: Curves + ?Sized>(
        &self,
        curves: &C,
        budget: f64,
        split: f64,
    ) -> (Vec<Vec<P>>, f64) {
        let mut rings = Vec::with_capacity(self.loops.len());
        let mut dev = 0.0f64;
        for (l, intervals) in self.loops.iter().enumerate() {
            let sag = |k: usize, t0: f64, t1: f64| -> f64 {
                let (a, b) = (self.point(curves, l, k, t0), self.point(curves, l, k, t1));
                let mut worst = 0.0f64;
                for j in 1..8 {
                    let s = self.point(curves, l, k, t0 + (t1 - t0) * f64::from(j) / 8.0);
                    let d = offset(a, b, s);
                    if d.abs() > worst.abs() {
                        worst = d;
                    }
                }
                worst
            };
            // (interval, t0, t1) of each chord, its sagitta, its start and the way it may move
            let mut chords: Vec<(usize, f64, f64)> = Vec::new();
            let mut sags: Vec<f64> = Vec::new();
            let mut on: Vec<P> = Vec::new();
            let mut normal: Vec<P> = Vec::new();
            for (k, iv) in intervals.iter().enumerate() {
                let mut stack = vec![(0.0, 1.0, 0u32)];
                while let Some((t0, t1, depth)) = stack.pop() {
                    let s = sag(k, t0, t1);
                    if s.abs() > split * budget && depth < 40 {
                        let m = 0.5 * (t0 + t1);
                        stack.push((m, t1, depth + 1));
                        stack.push((t0, m, depth + 1));
                    } else {
                        chords.push((k, t0, t1));
                        sags.push(s);
                        on.push(self.point(curves, l, k, t0));
                        normal.push(if t0 == 0.0 && iv.corner {
                            [0.0, 0.0]
                        } else {
                            right(self.derivative(curves, l, k, t0))
                        });
                    }
                }
            }
            let m = on.len();
            let o = on[0];
            let rel: Vec<P> = on.iter().map(|v| sub(*v, o)).collect();
            let out: Vec<P> = (0..m)
                .map(|i| {
                    let w = 0.5 * (sags[(i + m - 1) % m] + sags[i]);
                    [w * normal[i][0], w * normal[i][1]]
                })
                .collect();
            // the shoelace area of rel + λ out: a0 + a1 λ + a2 λ²
            let cross = |a: P, b: P| a[0] * b[1] - a[1] * b[0];
            let (mut a0, mut a1, mut a2) = (0.0, 0.0, 0.0);
            for i in 0..m {
                let j = (i + 1) % m;
                a0 += 0.5 * cross(rel[i], rel[j]);
                a1 += 0.5 * (cross(rel[i], out[j]) + cross(out[i], rel[j]));
                a2 += 0.5 * cross(out[i], out[j]);
            }
            let deficit = self.areas[l] - a0;
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
            let ring: Vec<P> = (0..m)
                .map(|i| [on[i][0] + lambda * out[i][0], on[i][1] + lambda * out[i][1]])
                .collect();
            for (i, &(k, t0, t1)) in chords.iter().enumerate() {
                let segment = Piece::Line(ring[i], ring[(i + 1) % m]);
                for j in 0..=8 {
                    let s = self.point(curves, l, k, t0 + (t1 - t0) * f64::from(j) / 8.0);
                    dev = dev.max(segment.nearest(s).0);
                }
            }
            rings.push(ring);
        }
        (rings, dev)
    }

    /// The region as polygons within `tolerance`, each outer loop with its holes.
    pub(crate) fn polygons<C: Curves + ?Sized>(
        &self,
        curves: &C,
        tolerance: Tolerance,
    ) -> Polygons {
        let (mut rings, dev) = self.rings(curves, tolerance.budget());
        let contours = self
            .groups
            .iter()
            .map(|(o, holes)| {
                (
                    std::mem::take(&mut rings[*o]),
                    holes
                        .iter()
                        .map(|h| std::mem::take(&mut rings[*h]))
                        .collect(),
                )
            })
            .collect::<Vec<_>>();
        assemble(contours, dev, tolerance)
    }

    /// The loops within [`FILL_TOLERANCE`] of the region's larger side, as segments, each
    /// keeping its area.
    fn fine<C: Curves + ?Sized>(&self, curves: &C) -> &Vec<Vec<Piece>> {
        self.fine.get_or_init(|| {
            let b = self.bounds;
            let size = (b.max.x - b.min.x).to_um().max((b.max.y - b.min.y).to_um());
            let (rings, _) = self.rings(curves, FILL_TOLERANCE * size);
            rings
                .into_iter()
                .map(|r| {
                    let n = r.len();
                    (0..n).map(|i| Piece::Line(r[i], r[(i + 1) % n])).collect()
                })
                .collect()
        })
    }

    /// The area inside `cell`: the polygons within [`FILL_TOLERANCE`] of the region's larger
    /// side clipped to it, in error by at most that times the boundary's length in the cell.
    pub(crate) fn fill<C: Curves + ?Sized>(&self, curves: &C, cell: Bounds) -> f64 {
        if !self.bounds.overlaps(&cell) {
            return 0.0;
        }
        if cell.encloses(&self.bounds) {
            return self.area();
        }
        fill_of(self.fine(curves), self.bounds, self.area(), cell)
    }

    /// Whether the boundary crosses or touches itself, checked on its polygons within `budget`:
    /// the reason, where it does.
    pub(crate) fn check_simple<C: Curves + ?Sized>(
        &self,
        curves: &C,
        budget: f64,
    ) -> Result<(), String> {
        let (rings, _) = self.rings(curves, budget);
        let mut segments: Vec<(usize, usize, P, P)> = Vec::new();
        for (l, r) in rings.iter().enumerate() {
            let n = r.len();
            for i in 0..n {
                segments.push((l, i, r[i], r[(i + 1) % n]));
            }
        }
        let boxes: Vec<Bounds> = segments.iter().map(|s| bounds_um(&[s.2, s.3])).collect();
        let index = Index::new(&boxes);
        for (a, s) in segments.iter().enumerate() {
            for b in index.meeting(boxes[a]) {
                if b <= a {
                    continue;
                }
                let t = &segments[b];
                if s.0 == t.0 {
                    let n = rings[s.0].len();
                    let (i, j) = (s.1, t.1);
                    if j == (i + 1) % n || i == (j + 1) % n {
                        continue;
                    }
                }
                if segments_meet(s.2, s.3, t.2, t.3) {
                    return Err(format!(
                        "its outline crosses or touches itself near ({:.6}, {:.6}) um",
                        t.2[0], t.2[1]
                    ));
                }
            }
        }
        Ok(())
    }
}

/// `span` cut into intervals over which the curve turns by at most [`TURN`] and is monotone in x
/// and y, appended to `out`.
fn split<C: Curves + ?Sized>(
    curves: &C,
    span: &Span,
    out: &mut Vec<Interval>,
) -> Result<(), String> {
    let u = |t: f64| {
        if t == 1.0 {
            span.u1
        } else {
            span.u0 + t * (span.u1 - span.u0)
        }
    };
    let tangent = |t: f64| -> Result<P, String> {
        let (p, d) = curves.at(span.curve, u(t));
        let l = d[0].hypot(d[1]);
        if !(p[0].is_finite() && p[1].is_finite() && l.is_finite() && l > 0.0) {
            return Err(format!(
                "its boundary stops (its derivative is zero) or isn't finite at ({:.6}, {:.6}) um",
                p[0], p[1]
            ));
        }
        Ok([d[0] / l, d[1] / l])
    };
    let angle = |a: P, b: P| (a[0] * b[1] - a[1] * b[0]).atan2(dot(a, b)).abs();
    let mut taus = vec![0.0];
    let n = span.pieces.max(1);
    let mut stack: Vec<(f64, f64, u32)> = (0..n)
        .rev()
        .map(|k| {
            (
                f64::from(k) / f64::from(n),
                f64::from(k + 1) / f64::from(n),
                0,
            )
        })
        .collect();
    while let Some((a, b, depth)) = stack.pop() {
        let ts: Vec<P> = (0..=4)
            .map(|j| tangent(a + (b - a) * f64::from(j) / 4.0))
            .collect::<Result<_, _>>()?;
        let turn: f64 = ts.windows(2).map(|w| angle(w[0], w[1])).sum();
        if turn > TURN && depth < 40 {
            let m = 0.5 * (a + b);
            stack.push((m, b, depth + 1));
            stack.push((a, m, depth + 1));
        } else {
            taus.push(b);
        }
    }
    for w in taus.windows(2) {
        let (a, b) = (w[0], w[1]);
        let mut cuts: Vec<f64> = Vec::new();
        for axis in 0..2 {
            let g = |t: f64| curves.at(span.curve, u(t)).1[axis];
            let mut last: Option<(f64, f64)> = None;
            for j in 0..=8 {
                let t = if j == 8 {
                    b
                } else {
                    a + (b - a) * f64::from(j) / 8.0
                };
                let v = g(t);
                if v == 0.0 {
                    continue;
                }
                if let Some((lt, lv)) = last
                    && (lv > 0.0) != (v > 0.0)
                {
                    let r = root(g, lt, t, lv, v);
                    if r > a && r < b {
                        cuts.push(r);
                    }
                }
                last = Some((t, v));
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        let mut prev = a;
        for t in cuts.into_iter().chain(std::iter::once(b)) {
            if t > prev {
                out.push(Interval {
                    curve: span.curve,
                    u0: u(prev),
                    u1: u(t),
                    corner: false,
                });
                prev = t;
            }
        }
    }
    Ok(())
}

/// ∫ f from 0 to 1 by 20-point Gauss–Legendre, the interval halved wherever one rule and the
/// two on its halves disagree by more than round-off and `floor` (a kink of a piecewise
/// definition), at most 16 times.
fn integrate(f: &dyn Fn(f64) -> f64, floor: f64) -> f64 {
    fn part(f: &dyn Fn(f64) -> f64, a: f64, b: f64, whole: f64, floor: f64, depth: u32) -> f64 {
        let m = 0.5 * (a + b);
        let (l, r) = (boundary::gauss(f, a, m), boundary::gauss(f, m, b));
        let tol = (8.0 * f64::EPSILON * (l.abs() + r.abs())).max(floor);
        if depth >= 16 || (l + r - whole).abs() <= tol {
            return l + r;
        }
        part(f, a, m, l, floor, depth + 1) + part(f, m, b, r, floor, depth + 1)
    }
    part(f, 0.0, 1.0, boundary::gauss(f, 0.0, 1.0), floor, 0)
}
