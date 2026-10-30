//! Implicit regions: f(x, y) ≤ 0 inside a window, f an expression of `x` and `y`.
//!
//! **The boundary.** f is sampled on a grid over the window (256 cells along its longer side by
//! default) and its zero set traced by marching squares (Lorensen and Cline 1987, in two
//! dimensions, doi:10.1145/37402.37422): each cell's crossings, found on its edges by bracketed
//! root finding to the last bit, joined with the region on their left; a cell whose diagonal corners alone are inside
//! (a saddle) is decided by f at its centre. The crossings are vertices on the zero set itself;
//! where the gradient turns by more than 0.1 rad between two of them, the chord between is
//! halved and its middle moved onto the zero set along the chord's normal, until it doesn't.
//! Between two vertices the boundary is the curve itself, a graph over their chord: its point
//! at τ the root of f(a + τ e + σ n) = 0 along the chord's normal n, its derivative
//! e − (∇f·e / ∇f·n) n. The engine of [`super::trace`] answers from that: exact points, a tight
//! box, ∮ x dy and the nearest point by root finding on the curve.
//!
//! **What it resolves.** A part of the region (or a hole) that falls between the grid's nodes,
//! narrower than a cell, may be missed; the grid can be made finer
//! ([`Implicit::with_cells`]).
//!
//! **Distance.** f/|∇f| ([`Implicit::estimate`]) is exact on the boundary and in error by
//! O(κ d²) a distance d off it (κ the curvature). [`Region::distance`] corrects it to the exact
//! distance: the foot of the point found by root finding of (p − c)·c′ on the curve near it,
//! from the nearest pieces of the traced boundary.

use std::sync::Arc;

use super::curves::{Frame, Function, SLOT_X, SLOT_Y, check_expr, traced_region};
use super::outline::root;
use super::primitives::um;
use super::region::{Distance, Location, Polygons, Region, Tolerance};
use super::trace::{Curves, Span, Trace};
use super::transform::Transform;
use super::{Bounds, Point, boundary};
use crate::expr::{Expr, Parameters};
use crate::units::Length;
use crate::{Error, Result};

type P = [f64; 2];

/// The largest turn of the gradient between two vertices of the traced zero set, radians.
const TURN: f64 = 0.1;

/// The default number of the sampling grid's cells along the window's longer side.
pub const IMPLICIT_CELLS: usize = 256;

/// An implicit region: the points of a window where f(x, y) ≤ 0, f an expression of `x` and `y`
/// (lengths) and of parameters, of any dimension (only its sign counts): level sets,
/// superformulas, smooth unions. f must be positive on the window's edge, so that the region
/// lies inside it, and its gradient must not vanish on its zero set.
///
/// Its inside test is the definition: the sign of f. See the module for its boundary. Cheap to
/// clone.
#[derive(Clone, Debug)]
pub struct Implicit {
    data: Arc<Data>,
}

#[derive(Clone, Debug, PartialEq)]
struct Spec {
    f: Function,
    window: [P; 2],
    cells: usize,
    frame: Frame,
}

impl PartialEq for Implicit {
    fn eq(&self, other: &Implicit) -> bool {
        self.data.spec == other.data.spec
    }
}

/// The zero set's vertices (in the definition's frame) joined by chords, each standing for the
/// curve between its ends.
#[derive(Debug)]
struct Chords {
    f: Function,
    frame: Frame,
    chords: Vec<[P; 2]>,
}

#[derive(Debug)]
struct Data {
    spec: Spec,
    chords: Chords,
    trace: Trace,
}

fn implicit_error(reason: impl Into<String>) -> Error {
    Error::invalid("implicit region", reason.into())
}

/// f at q.
fn value(f: &Function, q: P) -> f64 {
    f.value(&[(SLOT_X, q[0]), (SLOT_Y, q[1])])
}

/// ∇f at q.
fn gradient(f: &Function, q: P) -> P {
    let set = [(SLOT_X, q[0]), (SLOT_Y, q[1])];
    [
        f.dual(&set, SLOT_X).derivative,
        f.dual(&set, SLOT_Y).derivative,
    ]
}

/// The point of the zero set on the line m + σ n with |σ| ≤ `reach`, by bracketed root
/// finding; `None` if f doesn't change sign over it.
fn project(f: &Function, m: P, n: P, reach: f64) -> Option<P> {
    let h = |s: f64| value(f, [m[0] + s * n[0], m[1] + s * n[1]]);
    let (fa, fb) = (h(-reach), h(reach));
    if !(fa.is_finite() && fb.is_finite()) || (fa > 0.0) == (fb > 0.0) {
        return None;
    }
    let (f0, s) = (h(0.0), 0.0);
    let s = if f0 == 0.0 {
        s
    } else if (f0 > 0.0) == (fa > 0.0) {
        root(h, 0.0, reach, f0, fb)
    } else {
        root(h, -reach, 0.0, fa, f0)
    };
    Some([m[0] + s * n[0], m[1] + s * n[1]])
}

/// The unit normal to the right of the chord from a to b, and its length.
fn chord_normal(a: P, b: P) -> (P, f64) {
    let e = [b[0] - a[0], b[1] - a[1]];
    let l = e[0].hypot(e[1]);
    ([e[1] / l, -e[0] / l], l)
}

impl Curves for Chords {
    fn at(&self, curve: usize, tau: f64) -> (P, P) {
        let [a, b] = self.chords[curve];
        let e = [b[0] - a[0], b[1] - a[1]];
        let (n, l) = chord_normal(a, b);
        let q = if tau == 0.0 {
            a
        } else if tau == 1.0 {
            b
        } else {
            let m = [a[0] + tau * e[0], a[1] + tau * e[1]];
            [0.25, 1.0, 4.0]
                .iter()
                .find_map(|r| project(&self.f, m, n, r * l))
                .unwrap_or([f64::NAN; 2])
        };
        let g = gradient(&self.f, q);
        let (gn, ge) = (g[0] * n[0] + g[1] * n[1], g[0] * e[0] + g[1] * e[1]);
        let ds = -ge / gn;
        self.frame.out((q, [e[0] + ds * n[0], e[1] + ds * n[1]]))
    }
}

impl Data {
    fn locate(&self, p: P) -> Location {
        let s = &self.spec;
        let q = s.frame.from.map_um(p);
        let [lo, hi] = s.window;
        if !(q[0] >= lo[0] && q[0] <= hi[0] && q[1] >= lo[1] && q[1] <= hi[1]) {
            return Location::Outside;
        }
        let v = value(&s.f, q);
        if v.is_nan() {
            return Location::Outside;
        }
        Location::of(v.total_cmp(&0.0))
    }
}

traced_region!(Implicit, chords);

impl Implicit {
    /// The region where `f` ≤ 0 inside `window`, sampled on a grid of [`IMPLICIT_CELLS`] cells
    /// along its longer side ([`Implicit::with_cells`]), the expression parsed with
    /// `parameters`' scope and evaluated at their values.
    ///
    /// # Errors
    ///
    /// As [`Implicit::with_cells`].
    pub fn new(f: &Expr, window: Bounds, parameters: &Parameters) -> Result<Implicit> {
        Implicit::with_cells(f, window, IMPLICIT_CELLS, parameters)
    }

    /// The region where `f` ≤ 0 inside `window`, sampled on a grid of `cells` cells along the
    /// window's longer side (and as many of the same size as fit along the other, at least 2).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `f` reads no coordinate but x and y, the window is finite
    /// and not empty, 2 ≤ cells ≤ 4096, f is finite at every node of the grid and positive on
    /// the window's edge, ≤ 0 at one node at least (f has a zero set inside the window), and its
    /// gradient isn't zero where the boundary is traced.
    pub fn with_cells(
        f: &Expr,
        window: Bounds,
        cells: usize,
        parameters: &Parameters,
    ) -> Result<Implicit> {
        check_expr("implicit region", f, &[SLOT_X, SLOT_Y], None, "f")?;
        let [lo, hi] = window.corners();
        if !(lo.iter().chain(&hi).all(|v| v.is_finite()) && hi[0] > lo[0] && hi[1] > lo[1]) {
            return Err(implicit_error(format!(
                "needs a finite window that isn't empty, got ({}, {}) to ({}, {})",
                window.min.x, window.min.y, window.max.x, window.max.y
            )));
        }
        if !(2..=4096).contains(&cells) {
            return Err(implicit_error(format!(
                "its sampling grid needs 2 to 4096 cells along the window's longer side, got \
                 {cells}"
            )));
        }
        let spec = Spec {
            f: Function::new(f, parameters),
            window: [lo, hi],
            cells,
            frame: Frame::new(Transform::IDENTITY),
        };
        Implicit::build(spec)
    }

    fn build(spec: Spec) -> Result<Implicit> {
        let mut loops = zero_set(&spec)?;
        if spec.frame.mirrored() {
            for l in &mut loops {
                l.reverse();
            }
        }
        let mut chords = Vec::new();
        let mut spans = Vec::with_capacity(loops.len());
        for l in &loops {
            let n = l.len();
            let mut s = Vec::with_capacity(n);
            for k in 0..n {
                s.push(Span {
                    curve: chords.len(),
                    u0: 0.0,
                    u1: 1.0,
                    corner: l[k].1,
                    pieces: 1,
                });
                chords.push([l[k].0, l[(k + 1) % n].0]);
            }
            spans.push(s);
        }
        let chords = Chords {
            f: spec.f.clone(),
            frame: spec.frame,
            chords,
        };
        let trace = Trace::new(&chords, &spans).map_err(implicit_error)?;
        Ok(Implicit {
            data: Arc::new(Data {
                spec,
                chords,
                trace,
            }),
        })
    }

    /// The expression f.
    pub fn expression(&self) -> &Expr {
        self.data.spec.f.expr()
    }

    /// The window, in the region's own frame (before any transform).
    pub fn window(&self) -> Bounds {
        let [lo, hi] = self.data.spec.window;
        Bounds::new(Point::um(lo[0], lo[1]), Point::um(hi[0], hi[1]))
    }

    /// The sampling grid's cells along the window's longer side.
    pub fn cells(&self) -> usize {
        self.data.spec.cells
    }

    /// The first-order distance f/|∇f| at `p`: exact on the boundary, in error by O(κ d²) a
    /// distance d off it (κ the curvature there); signed as f, negative inside.
    pub fn estimate(&self, p: Point) -> Length {
        let s = &self.data.spec;
        let q = s.frame.from.map_um(um(p));
        let g = gradient(&s.f, q);
        Length::um(value(&s.f, q) / g[0].hypot(g[1]) * s.frame.to.factor())
    }

    /// The region's image under `t`: the same definition in a frame moved by `t`.
    ///
    /// # Panics
    ///
    /// If the image's boundary can't be traced, which a similarity can't do to one that could
    /// (but for overflow).
    pub fn transformed(&self, t: &Transform) -> Implicit {
        let spec = Spec {
            frame: self.data.spec.frame.then(t),
            ..self.data.spec.clone()
        };
        Implicit::build(spec).expect("a transformed implicit region is one")
    }
}

/// The zero set's loops, each a list of vertices on it (in the definition's frame) with the
/// region on their left, and whether each is a corner: marching squares, then chords halved
/// where the gradient turns. See the module.
fn zero_set(spec: &Spec) -> Result<Vec<Vec<(P, bool)>>> {
    let f = &spec.f;
    let [lo, hi] = spec.window;
    let (w, h) = (hi[0] - lo[0], hi[1] - lo[1]);
    let cells = spec.cells as f64;
    let (nx, ny) = if w >= h {
        (spec.cells, ((cells * h / w).ceil() as usize).max(2))
    } else {
        (((cells * w / h).ceil() as usize).max(2), spec.cells)
    };
    let xs: Vec<f64> = (0..=nx)
        .map(|i| {
            if i == nx {
                hi[0]
            } else {
                lo[0] + w * i as f64 / nx as f64
            }
        })
        .collect();
    let ys: Vec<f64> = (0..=ny)
        .map(|j| {
            if j == ny {
                hi[1]
            } else {
                lo[1] + h * j as f64 / ny as f64
            }
        })
        .collect();
    let at = |i: usize, j: usize| j * (nx + 1) + i;
    let mut fs = vec![0.0; (nx + 1) * (ny + 1)];
    let mut any = false;
    for j in 0..=ny {
        for i in 0..=nx {
            let v = value(f, [xs[i], ys[j]]);
            if !v.is_finite() {
                return Err(implicit_error(format!(
                    "f = \"{}\" is {v} at ({:.6}, {:.6}) um: it must be finite over its window",
                    f.expr().text(),
                    xs[i],
                    ys[j]
                )));
            }
            let edge = i == 0 || j == 0 || i == nx || j == ny;
            if edge && v <= 0.0 {
                return Err(implicit_error(format!(
                    "f = \"{}\" is {v:.6e} <= 0 at ({:.6}, {:.6}) um, on its window's edge: the \
                     region must lie inside its window; widen it",
                    f.expr().text(),
                    xs[i],
                    ys[j]
                )));
            }
            any |= v <= 0.0;
            fs[at(i, j)] = v;
        }
    }
    if !any {
        return Err(implicit_error(format!(
            "f = \"{}\" is positive at all {} x {} points of its sampling grid: it has no zero \
             set inside its window (or one enclosing less than a cell, {:.6} x {:.6} um; a finer \
             grid would find it)",
            f.expr().text(),
            nx + 1,
            ny + 1,
            w / nx as f64,
            h / ny as f64
        )));
    }
    let inside = |i: usize, j: usize| fs[at(i, j)] <= 0.0;
    // edges: horizontal (i, j) from node (i, j) to (i + 1, j), then vertical (i, j) from (i, j)
    // to (i, j + 1)
    let horizontal = |i: usize, j: usize| j * nx + i;
    let vertical = |i: usize, j: usize| (ny + 1) * nx + j * (nx + 1) + i;
    let edges = (ny + 1) * nx + ny * (nx + 1);
    let mut segments: Vec<(usize, usize)> = Vec::new();
    for j in 0..ny {
        for i in 0..nx {
            let corners = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)];
            let ids = [
                horizontal(i, j),
                vertical(i + 1, j),
                horizontal(i, j + 1),
                vertical(i, j),
            ];
            let ins: Vec<bool> = corners.iter().map(|&(a, b)| inside(a, b)).collect();
            let mut out_at = Vec::new();
            let mut in_at = Vec::new();
            for k in 0..4 {
                match (ins[k], ins[(k + 1) % 4]) {
                    (true, false) => out_at.push(k),
                    (false, true) => in_at.push(k),
                    _ => {}
                }
            }
            match out_at.len() {
                0 => {}
                1 => segments.push((ids[out_at[0]], ids[in_at[0]])),
                _ => {
                    let centre =
                        value(f, [0.5 * (xs[i] + xs[i + 1]), 0.5 * (ys[j] + ys[j + 1])]) <= 0.0;
                    for &k in &out_at {
                        let m = if centre { (k + 1) % 4 } else { (k + 3) % 4 };
                        segments.push((ids[k], ids[m]));
                    }
                }
            }
        }
    }
    let mut starting = vec![usize::MAX; edges];
    for (s, &(from, _)) in segments.iter().enumerate() {
        starting[from] = s;
    }
    // the crossing on an edge, by root finding along it from its first node
    let crossing = |e: usize| -> P {
        let (a, b) = if e < (ny + 1) * nx {
            let (i, j) = (e % nx, e / nx);
            ((i, j), (i + 1, j))
        } else {
            let r = e - (ny + 1) * nx;
            let (i, j) = (r % (nx + 1), r / (nx + 1));
            ((i, j), (i, j + 1))
        };
        let (pa, pb) = ([xs[a.0], ys[a.1]], [xs[b.0], ys[b.1]]);
        let (fa, fb) = (fs[at(a.0, a.1)], fs[at(b.0, b.1)]);
        if fa == 0.0 {
            return pa;
        }
        if fb == 0.0 {
            return pb;
        }
        let along = |s: f64| {
            if s == 1.0 {
                pb
            } else {
                [pa[0] + s * (pb[0] - pa[0]), pa[1] + s * (pb[1] - pa[1])]
            }
        };
        let s = root(|s| value(f, along(s)), 0.0, 1.0, fa, fb);
        along(s)
    };
    let tiny = 1e-9 * w.max(h);
    // crossings this close are one: a node where f is zero to round-off
    let merge = 1e-9 * (w / nx as f64).min(h / ny as f64);
    let close = |a: P, b: P| (a[0] - b[0]).hypot(a[1] - b[1]) <= merge;
    let mut used = vec![false; segments.len()];
    let mut loops = Vec::new();
    for first in 0..segments.len() {
        if used[first] {
            continue;
        }
        let mut ring: Vec<P> = Vec::new();
        let mut s = first;
        while !used[s] {
            used[s] = true;
            let p = crossing(segments[s].0);
            if !ring.last().is_some_and(|q| close(*q, p)) {
                ring.push(p);
            }
            s = starting[segments[s].1];
            if s == usize::MAX {
                return Err(implicit_error(
                    "its zero set's trace doesn't close (a crossing with no way on)",
                ));
            }
        }
        while ring.len() > 1 && close(ring[0], ring[ring.len() - 1]) {
            ring.pop();
        }
        if ring.len() < 3 {
            continue;
        }
        loops.push(refine(f, &ring, tiny)?);
    }
    if loops.is_empty() {
        return Err(implicit_error(format!(
            "f = \"{}\" has no zero set the sampling grid resolves inside its window",
            f.expr().text()
        )));
    }
    Ok(loops)
}

/// A loop's chords halved, their middles moved onto the zero set, until the gradient turns by
/// at most [`TURN`] along each (or the chord is shorter than `tiny`: a corner).
fn refine(f: &Function, ring: &[P], tiny: f64) -> Result<Vec<(P, bool)>> {
    let direction = |q: P| -> Result<P> {
        let g = gradient(f, q);
        let l = g[0].hypot(g[1]);
        if !(l.is_finite() && l > 0.0) {
            return Err(implicit_error(format!(
                "the gradient of f = \"{}\" is {l} at ({:.6}, {:.6}) um on its zero set: the \
                 boundary isn't a smooth curve there",
                f.expr().text(),
                q[0],
                q[1]
            )));
        }
        Ok([g[0] / l, g[1] / l])
    };
    let angle = |a: P, b: P| {
        (a[0] * b[1] - a[1] * b[0])
            .atan2(a[0] * b[0] + a[1] * b[1])
            .abs()
    };
    let n = ring.len();
    let mut corner = vec![false; n];
    // the vertices strictly between each ring vertex and the next
    let mut between: Vec<Vec<(P, bool)>> = Vec::with_capacity(n);
    for k in 0..n {
        let (a, b) = (ring[k], ring[(k + 1) % n]);
        let mut stack = vec![(a, b, 0u32)];
        let mut pending: Vec<(P, bool)> = Vec::new();
        while let Some((p, q, depth)) = stack.pop() {
            let (_, l) = chord_normal(p, q);
            let turn = angle(direction(p)?, direction(q)?);
            if turn <= TURN || l <= tiny || depth >= 60 {
                // (still turning on a chord shorter than `tiny`: a corner of the zero set,
                // kept as a vertex)
                let sharp = turn > TURN;
                if q == b {
                    corner[(k + 1) % n] |= sharp;
                } else {
                    pending.push((q, sharp));
                }
                continue;
            }
            let m = [0.5 * (p[0] + q[0]), 0.5 * (p[1] + q[1])];
            let (nn, _) = chord_normal(p, q);
            let Some(mid) = project(f, m, nn, 0.5 * l) else {
                return Err(implicit_error(format!(
                    "its zero set turns too sharply between ({:.6}, {:.6}) and ({:.6}, {:.6}) um \
                     to be followed: a feature below its sampling grid; a finer grid would \
                     resolve it",
                    p[0], p[1], q[0], q[1]
                )));
            };
            stack.push((mid, q, depth + 1));
            stack.push((p, mid, depth + 1));
        }
        between.push(pending);
    }
    let mut out: Vec<(P, bool)> = Vec::with_capacity(2 * n);
    for (k, rest) in between.into_iter().enumerate() {
        for v in std::iter::once((ring[k], corner[k])).chain(rest) {
            match out.last_mut() {
                Some(last) if last.0 == v.0 => last.1 |= v.1,
                _ => out.push(v),
            }
        }
    }
    while out.len() > 1 && out.first().map(|v| v.0) == out.last().map(|v| v.0) {
        if let Some(v) = out.pop() {
            out[0].1 |= v.1;
        }
    }
    Ok(out)
}
