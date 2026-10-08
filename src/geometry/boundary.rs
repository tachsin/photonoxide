//! A region's boundary as pieces: straight segments and arcs of conics (circles and ellipses),
//! each loop oriented with the region on its left. From the pieces, in closed form:
//!
//! - the area of the region inside an axis-aligned cell, by Green's theorem, A = ∮ x dy, over
//!   the boundary clipped to the cell one side at a time (each loop's excursions beyond a side
//!   replaced by the side's own segment, which keeps its winding number inside);
//! - the nearest point of the boundary, with the outward normal there;
//! - polygons within a tolerance, the arcs' vertices placed so that each polygon keeps the
//!   region's area exactly (to round-off).
//!
//! Coordinates are µm, as `[x, y]`.

use std::f64::consts::TAU;

pub(crate) type P = [f64; 2];

/// An arc of the conic c + R(ψ)(rx cos t, ry sin t) for t from `t0` to `t1` (counterclockwise
/// when t1 > t0). A circle when rx = ry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Conic {
    pub c: P,
    pub rx: f64,
    pub ry: f64,
    /// cos ψ and sin ψ, the first axis's direction.
    pub cos: f64,
    pub sin: f64,
    pub t0: f64,
    pub t1: f64,
}

/// A piece of a boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Piece {
    Line(P, P),
    Conic(Conic),
}

impl Conic {
    /// The point at parameter t, its radius scaled by `r` (1 on the conic).
    fn at_scaled(&self, t: f64, r: f64) -> P {
        let (u, v) = (r * self.rx * t.cos(), r * self.ry * t.sin());
        [
            self.c[0] + self.cos * u - self.sin * v,
            self.c[1] + self.sin * u + self.cos * v,
        ]
    }

    fn t(&self, u: f64) -> f64 {
        if u == 1.0 {
            self.t1
        } else {
            self.t0 + u * (self.t1 - self.t0)
        }
    }

    /// The coordinate `axis` as c + A cos t + B sin t: (A, B).
    fn coefficients(&self, axis: usize) -> (f64, f64) {
        if axis == 0 {
            (self.rx * self.cos, -self.ry * self.sin)
        } else {
            (self.rx * self.sin, self.ry * self.cos)
        }
    }

    /// ∫ x dy along the arc, in closed form.
    fn x_dy(&self) -> f64 {
        let (p1, p2) = self.coefficients(0);
        let (q1, q2) = self.coefficients(1);
        let (a, b) = (self.t0, self.t1);
        let dy = q1 * (b.cos() - a.cos()) + q2 * (b.sin() - a.sin());
        let cos2 = |t: f64| t / 2.0 + (2.0 * t).sin() / 4.0;
        let sin2 = |t: f64| t / 2.0 - (2.0 * t).sin() / 4.0;
        let sc = |t: f64| t.sin() * t.sin() / 2.0;
        self.c[0] * dy + p1 * q2 * (cos2(b) - cos2(a)) - p2 * q1 * (sin2(b) - sin2(a))
            + (p2 * q2 - p1 * q1) * (sc(b) - sc(a))
    }

    /// The parameters strictly inside the arc's range where the coordinate `axis` is `v`.
    fn crossings(&self, axis: usize, v: f64, out: &mut Vec<f64>) {
        let (a, b) = self.coefficients(axis);
        let r = a.hypot(b);
        if r == 0.0 {
            return;
        }
        let w = (v - self.c[axis]) / r;
        if !(-1.0..=1.0).contains(&w) {
            return;
        }
        let phi = b.atan2(a);
        let half = w.acos();
        let (lo, hi) = (self.t0.min(self.t1), self.t0.max(self.t1));
        for root in [phi + half, phi - half] {
            let mut t = root + TAU * ((lo - root) / TAU).ceil();
            while t < hi {
                if t > lo {
                    let u = (t - self.t0) / (self.t1 - self.t0);
                    if u > 0.0 && u < 1.0 && !out.contains(&u) {
                        out.push(u);
                    }
                }
                t += TAU;
            }
        }
    }

    /// Whether the arc runs counterclockwise.
    fn ccw(&self) -> bool {
        self.t1 > self.t0
    }
}

impl Piece {
    pub(crate) fn start(&self) -> P {
        match self {
            Piece::Line(a, _) => *a,
            Piece::Conic(c) => c.at_scaled(c.t0, 1.0),
        }
    }

    pub(crate) fn end(&self) -> P {
        match self {
            Piece::Line(_, b) => *b,
            Piece::Conic(c) => c.at_scaled(c.t1, 1.0),
        }
    }

    /// The point at u ∈ [0, 1] along the piece.
    fn at(&self, u: f64) -> P {
        match self {
            Piece::Line(a, b) => {
                if u == 1.0 {
                    *b
                } else {
                    [a[0] + u * (b[0] - a[0]), a[1] + u * (b[1] - a[1])]
                }
            }
            Piece::Conic(c) => c.at_scaled(c.t(u), 1.0),
        }
    }

    /// The part from u0 to u1.
    fn part(&self, u0: f64, u1: f64) -> Piece {
        match self {
            Piece::Line(..) => Piece::Line(self.at(u0), self.at(u1)),
            Piece::Conic(c) => Piece::Conic(Conic {
                t0: c.t(u0),
                t1: c.t(u1),
                ..*c
            }),
        }
    }

    /// ∫ x dy along the piece.
    pub(crate) fn x_dy(&self) -> f64 {
        match self {
            Piece::Line(a, b) => (a[0] + b[0]) / 2.0 * (b[1] - a[1]),
            Piece::Conic(c) => c.x_dy(),
        }
    }

    /// The u strictly inside (0, 1) where the coordinate `axis` is `v`.
    fn crossings(&self, axis: usize, v: f64, out: &mut Vec<f64>) {
        match self {
            Piece::Line(a, b) => {
                let d = b[axis] - a[axis];
                if d != 0.0 {
                    let u = (v - a[axis]) / d;
                    if u > 0.0 && u < 1.0 {
                        out.push(u);
                    }
                }
            }
            Piece::Conic(c) => c.crossings(axis, v, out),
        }
    }

    /// The nearest point of the piece to `p`, its distance, and the outward normal of the
    /// region there (the region on the piece's left).
    pub(crate) fn nearest(&self, p: P) -> (f64, P, P) {
        match self {
            Piece::Line(a, b) => {
                let e = [b[0] - a[0], b[1] - a[1]];
                let l2 = e[0] * e[0] + e[1] * e[1];
                let t = if l2 > 0.0 {
                    (((p[0] - a[0]) * e[0] + (p[1] - a[1]) * e[1]) / l2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let q = self.at(t);
                let l = l2.sqrt();
                let n = if l > 0.0 {
                    [e[1] / l, -e[0] / l]
                } else {
                    [1.0, 0.0]
                };
                ((p[0] - q[0]).hypot(p[1] - q[1]), q, n)
            }
            Piece::Conic(c) => conic_nearest(c, p),
        }
    }
}

/// The nearest point of an arc of a conic.
fn conic_nearest(c: &Conic, p: P) -> (f64, P, P) {
    // in the conic's own frame
    let d = [p[0] - c.c[0], p[1] - c.c[1]];
    let local = [c.cos * d[0] + c.sin * d[1], -c.sin * d[0] + c.cos * d[1]];
    let t = if c.rx == c.ry {
        if local == [0.0, 0.0] {
            c.t0
        } else {
            local[1].atan2(local[0])
        }
    } else {
        let q = ellipse_nearest(c.rx, c.ry, local);
        (q[1] / c.ry).atan2(q[0] / c.rx)
    };
    let (lo, hi) = (c.t0.min(c.t1), c.t0.max(c.t1));
    let t = t + TAU * ((lo - t) / TAU).ceil();
    let sign = if c.ccw() { 1.0 } else { -1.0 };
    let normal_at = |t: f64| {
        // the conic's outward normal, ∝ (cos t / rx, sin t / ry) in its frame
        let (u, v) = (t.cos() / c.rx, t.sin() / c.ry);
        let l = u.hypot(v);
        let (u, v) = (sign * u / l, sign * v / l);
        [c.cos * u - c.sin * v, c.sin * u + c.cos * v]
    };
    let candidate = |t: f64| {
        let q = c.at_scaled(t, 1.0);
        ((p[0] - q[0]).hypot(p[1] - q[1]), q, normal_at(t))
    };
    let mut best = candidate(c.t0);
    let end = candidate(c.t1);
    if end.0 < best.0 {
        best = end;
    }
    if t <= hi {
        let inner = candidate(t);
        if inner.0 <= best.0 {
            best = inner;
        }
    }
    best
}

/// The point of the ellipse (x/a)² + (y/b)² = 1 nearest to `p`, in the ellipse's frame: the
/// Lagrange condition reduces to one monotone equation in one variable, solved by bisection
/// to the last bit.
pub(crate) fn ellipse_nearest(a: f64, b: f64, p: P) -> P {
    if a < b {
        let q = ellipse_nearest(b, a, [p[1], p[0]]);
        return [q[1], q[0]];
    }
    // the first quadrant; signs restored at the end
    let (y0, y1) = (p[0].abs(), p[1].abs());
    let (x0, x1) = if y1 > 0.0 {
        if y0 > 0.0 {
            let z0 = y0 / a;
            let z1 = y1 / b;
            let g = z0 * z0 + z1 * z1 - 1.0;
            if g == 0.0 {
                (y0, y1)
            } else {
                let r0 = (a / b) * (a / b);
                let n0 = r0 * z0;
                // G(s) = (n0/(s + r0))² + (z1/(s + 1))² − 1 is decreasing; bracket its root
                let mut s0 = z1 - 1.0;
                let mut s1 = if g < 0.0 { 0.0 } else { n0.hypot(z1) - 1.0 };
                let mut s = s0;
                for _ in 0..1100 {
                    s = 0.5 * (s0 + s1);
                    if s == s0 || s == s1 {
                        break;
                    }
                    let ratio0 = n0 / (s + r0);
                    let ratio1 = z1 / (s + 1.0);
                    let gs = ratio0 * ratio0 + ratio1 * ratio1 - 1.0;
                    if gs > 0.0 {
                        s0 = s;
                    } else if gs < 0.0 {
                        s1 = s;
                    } else {
                        break;
                    }
                }
                (r0 * y0 / (s + r0), y1 / (s + 1.0))
            }
        } else {
            (0.0, b)
        }
    } else {
        let numer = a * y0;
        let denom = a * a - b * b;
        if numer < denom {
            let x = a * a * y0 / denom;
            let xa = x / a;
            (x, b * (1.0 - xa * xa).max(0.0).sqrt())
        } else {
            (a, 0.0)
        }
    };
    [x0.copysign(p[0]), x1.copysign(p[1])]
}

/// The area enclosed by `loops` (each with the region on its left): Σ ∮ x dy.
pub(crate) fn area_of(loops: &[Vec<Piece>]) -> f64 {
    loops.iter().flatten().map(Piece::x_dy).sum()
}

/// A loop clipped to the half-plane where the coordinate `axis` is ≥ `v` (`above`) or ≤ `v`:
/// the parts inside, joined along the line where the loop leaves and comes back.
fn clip(pieces: &[Piece], axis: usize, v: f64, above: bool) -> Vec<Piece> {
    let inside = |p: P| if above { p[axis] >= v } else { p[axis] <= v };
    let mut parts: Vec<(Piece, bool)> = Vec::with_capacity(pieces.len() + 4);
    let mut us = Vec::new();
    for piece in pieces {
        us.clear();
        piece.crossings(axis, v, &mut us);
        if us.is_empty() {
            let mid = piece.at(0.5);
            parts.push((*piece, inside(mid)));
            continue;
        }
        us.sort_by(f64::total_cmp);
        let mut prev = 0.0;
        for &u in us.iter().chain(std::iter::once(&1.0)) {
            if u <= prev {
                continue;
            }
            let mid = piece.at(0.5 * (prev + u));
            parts.push((piece.part(prev, u), inside(mid)));
            prev = u;
        }
    }
    if parts.iter().all(|p| p.1) {
        return pieces.to_vec();
    }
    if parts.iter().all(|p| !p.1) {
        return Vec::new();
    }
    let n = parts.len();
    let first = (0..n)
        .find(|&k| parts[k].1 && !parts[(k + n - 1) % n].1)
        .unwrap_or(0);
    let mut out = Vec::with_capacity(n + 2);
    let mut last: Option<P> = None;
    let mut start: Option<P> = None;
    for k in 0..n {
        let (piece, kept) = parts[(first + k) % n];
        if !kept {
            continue;
        }
        let s = piece.start();
        match last {
            Some(e) if e != s => out.push(Piece::Line(e, s)),
            None => start = Some(s),
            _ => {}
        }
        out.push(piece);
        last = Some(piece.end());
    }
    if let (Some(e), Some(s)) = (last, start)
        && e != s
    {
        out.push(Piece::Line(e, s));
    }
    out
}

/// The area of the region `loops` bound inside the cell [x0, x1] × [y0, y1].
pub(crate) fn fill(loops: &[Vec<Piece>], cell: [P; 2]) -> f64 {
    let [lo, hi] = cell;
    loops
        .iter()
        .map(|l| {
            let a = clip(l, 0, lo[0], true);
            let a = clip(&a, 0, hi[0], false);
            let a = clip(&a, 1, lo[1], true);
            let a = clip(&a, 1, hi[1], false);
            a.iter().map(Piece::x_dy).sum::<f64>()
        })
        .sum()
}

/// The signed distance from `p` to the boundary `loops` (negative when `inside`) and the
/// outward normal at the nearest point.
pub(crate) fn distance(loops: &[Vec<Piece>], p: P, inside: bool) -> (f64, P) {
    let mut best = (f64::INFINITY, p, [1.0, 0.0]);
    for piece in loops.iter().flatten() {
        let c = piece.nearest(p);
        if c.0 < best.0 {
            best = c;
        }
    }
    signed(best, p, inside)
}

/// A nearest point's distance signed, and the normal: along the line from the point when it is
/// off the boundary, the boundary's own on it.
pub(crate) fn signed((d, q, n): (f64, P, P), p: P, inside: bool) -> (f64, P) {
    if d > 0.0 {
        let away = [(p[0] - q[0]) / d, (p[1] - q[1]) / d];
        if inside {
            (-d, [-away[0], -away[1]])
        } else {
            (d, away)
        }
    } else {
        (0.0, n)
    }
}

/// The length of the boundary: exact for segments and circles, an ellipse's arcs by
/// Gauss–Legendre quadrature to round-off.
pub(crate) fn length(loops: &[Vec<Piece>]) -> f64 {
    loops
        .iter()
        .flatten()
        .map(|piece| match piece {
            Piece::Line(a, b) => (b[0] - a[0]).hypot(b[1] - a[1]),
            Piece::Conic(c) if c.rx == c.ry => c.rx * (c.t1 - c.t0).abs(),
            Piece::Conic(c) => {
                let speed = |t: f64| (c.rx * t.sin()).hypot(c.ry * t.cos());
                integrate(speed, c.t0.min(c.t1), c.t0.max(c.t1))
            }
        })
        .sum()
}

/// ∫ f from a to b by 20-point Gauss–Legendre on 64 equal panels: round-off for the smooth,
/// periodic integrands it is given (an ellipse's speed).
pub(crate) fn integrate(f: impl Fn(f64) -> f64, a: f64, b: f64) -> f64 {
    let panels = 64;
    let h = (b - a) / f64::from(panels);
    (0..panels)
        .map(|k| {
            let mid = a + (f64::from(k) + 0.5) * h;
            GL20.iter()
                .map(|&(x, w)| w * (f(mid + 0.5 * h * x) + f(mid - 0.5 * h * x)))
                .sum::<f64>()
                * 0.5
                * h
        })
        .sum()
}

/// The 20-point Gauss–Legendre rule's positive nodes and their weights.
const GL20: [(f64, f64); 10] = [
    (0.076_526_521_133_497_33, 0.152_753_387_130_725_85),
    (0.227_785_851_141_645_08, 0.149_172_986_472_603_75),
    (0.373_706_088_715_419_56, 0.142_096_109_318_382_05),
    (0.510_867_001_950_827_1, 0.131_688_638_449_176_63),
    (0.636_053_680_726_515, 0.118_194_531_961_518_42),
    (0.746_331_906_460_150_8, 0.101_930_119_817_240_44),
    (0.839_116_971_822_218_8, 0.083_276_741_576_704_75),
    (0.912_234_428_251_326, 0.062_672_048_334_109_06),
    (0.963_971_927_277_913_8, 0.040_601_429_800_386_94),
    (0.993_128_599_185_094_9, 0.017_614_007_139_152_12),
];

/// The vertices of a polygon in place of an arc, from its start up to (not including) its end,
/// or all of them for a closed conic, and how far the polygon strays from the arc at most.
///
/// The arc is the image of a unit circle's arc under an affine map, which keeps ratios of
/// areas. On the unit circle the arc's interior vertices are at a radius R just above 1, chosen
/// so that the polygon's fan from the centre has the sector's area, Φ/2: (N − 2) R² sin Δ + 2 R
/// sin Δ = Φ for N chords of angle Δ = Φ/N between end points on the arc, R² = Φ/(N sin Δ) for
/// a closed conic. The polygon then has the arc's area exactly, and its distance from the arc,
/// R − 1 at the vertices and 1 − R cos(Δ/2) at the chords' middles, is about Δ²/12 against the
/// inscribed polygon's Δ²/8. N is the fewest chords that keep that distance, times the map's
/// largest stretch, within `budget` (µm), and at most 2²⁰.
pub(crate) fn flatten(c: &Conic, budget: f64, closed: bool) -> (Vec<P>, f64) {
    let phi = (c.t1 - c.t0).abs();
    let sign = if c.ccw() { 1.0 } else { -1.0 };
    let stretch = c.rx.max(c.ry);
    let tol = budget / stretch;
    let least = if closed {
        3
    } else {
        2.max((phi / std::f64::consts::PI).floor() as usize + 1)
    };
    let most = 1usize << 20;
    let deviation = |n: usize| -> (f64, f64) {
        let delta = phi / n as f64;
        let (s, half) = (delta.sin(), (0.5 * delta).cos());
        if closed {
            let r = (phi / (n as f64 * s)).sqrt();
            return (r, (r - 1.0).max(1.0 - r * half));
        }
        let r = if n == 2 {
            phi / (2.0 * s)
        } else {
            let m = (n - 2) as f64;
            (-1.0 + (1.0 + m * phi / s).sqrt()) / m
        };
        // the end chords, from radius 1 to R at angle Δ: their least radius
        let chord = (1.0 + r * r - 2.0 * r * delta.cos()).sqrt();
        let foot = r * s / chord;
        let along = (1.0 - r * delta.cos()) / chord;
        let least_r = if along < 0.0 || along > chord {
            1.0f64.min(r)
        } else {
            foot
        };
        let mid = if n > 2 { 1.0 - r * half } else { 0.0 };
        (r, (r - 1.0).max(1.0 - least_r).max(mid))
    };
    let mut n = ((phi / (12.0 * tol).sqrt()).ceil() as usize).clamp(least, most);
    // the estimate is close; step down while it holds, up until it does
    while n > least && deviation(n - 1).1 <= tol {
        n -= 1;
    }
    while n < most && deviation(n).1 > tol {
        n = (n + n / 8 + 1).min(most);
    }
    let (r, dev) = deviation(n);
    let delta = phi / n as f64;
    let vertices = if closed {
        (0..n)
            .map(|k| c.at_scaled(c.t0 + sign * delta * k as f64, r))
            .collect()
    } else {
        let mut v = Vec::with_capacity(n);
        v.push(c.at_scaled(c.t0, 1.0));
        v.extend((1..n).map(|k| c.at_scaled(c.t0 + sign * delta * k as f64, r)));
        v
    };
    (vertices, dev * stretch)
}

/// A loop's vertices within `budget` of it, and the largest distance from it.
pub(crate) fn polygon(pieces: &[Piece], budget: f64) -> (Vec<P>, f64) {
    if let [Piece::Conic(c)] = pieces
        && ((c.t1 - c.t0).abs() - TAU).abs() < 1e-12
    {
        return flatten(c, budget, true);
    }
    let mut v = Vec::new();
    let mut dev = 0.0f64;
    for piece in pieces {
        match piece {
            Piece::Line(a, _) => v.push(*a),
            Piece::Conic(c) => {
                let (w, d) = flatten(c, budget, false);
                v.extend(w);
                dev = dev.max(d);
            }
        }
    }
    (v, dev)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn circle(r: f64) -> Vec<Piece> {
        vec![Piece::Conic(Conic {
            c: [0.3, -0.2],
            rx: r,
            ry: r,
            cos: 1.0,
            sin: 0.0,
            t0: 0.0,
            t1: TAU,
        })]
    }

    #[test]
    fn a_circles_area_by_greens_theorem() {
        let a = area_of(&[circle(2.0)]);
        assert!((a - 4.0 * std::f64::consts::PI).abs() < 1e-13, "{a}");
    }

    #[test]
    fn a_circle_cut_by_a_cell() {
        // the disk of radius 1 about the origin in the cell [0, 2]²: a quarter of it
        let mut l = circle(1.0);
        if let Piece::Conic(c) = &mut l[0] {
            c.c = [0.0, 0.0];
        }
        let q = fill(&[l.clone()], [[0.0, 0.0], [2.0, 2.0]]);
        assert!((q - std::f64::consts::PI / 4.0).abs() < 1e-14, "{q}");
        // a cell straddling the edge, [0.5, 1.5] × [−0.5, 0.5]: the disk beyond x = 0.5 within
        // |y| ≤ 0.5, against the midpoint rule
        let seg = fill(&[l], [[0.5, -0.5], [1.5, 0.5]]);
        let n = 200_000;
        let num: f64 = (0..n)
            .map(|k| {
                let x = 0.5 + (f64::from(k) + 0.5) / f64::from(n) * 0.5;
                2.0 * (1.0 - x * x).sqrt().min(0.5) * 0.5 / f64::from(n)
            })
            .sum();
        // (the rule's error: the square root's at x = 1, h^1.5)
        assert!((seg - num).abs() < 2e-8, "{seg} {num}");
    }

    #[test]
    fn the_nearest_point_of_an_ellipse() {
        let (a, b) = (2.0, 0.5);
        for p in [
            [3.0, 1.0],
            [0.1, 0.2],
            [-1.0, -0.1],
            [0.0, 2.0],
            [1.9, 0.0],
            [0.0, 0.0],
        ] {
            let q = ellipse_nearest(a, b, p);
            assert!(((q[0] / a).powi(2) + (q[1] / b).powi(2) - 1.0).abs() < 1e-14);
            // against a dense search
            let mut best = f64::INFINITY;
            for k in 0..200_000 {
                let t = TAU * k as f64 / 200_000.0;
                best = best.min((a * t.cos() - p[0]).hypot(b * t.sin() - p[1]));
            }
            let d = (q[0] - p[0]).hypot(q[1] - p[1]);
            assert!(d <= best + 1e-12 && d > best - 1e-9, "{p:?}: {d} {best}");
        }
    }
}
