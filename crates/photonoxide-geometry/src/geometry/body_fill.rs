//! The volume a body fills of an axis-aligned cell ([`Body::fill`]): what averaging over a 3D
//! grid's cells needs.
//!
//! - **Planar faces, exactly.** A half-space's share of a box is the volume its plane cuts off,
//!   in closed form ([`plane_share`]); a slab is two of them; an axis-aligned box the overlap of
//!   two boxes; an extrusion with vertical walls the shape's own area in the cell's footprint
//!   ([`super::Region::fill`]: exact for polygons by clipping) times the heights they share.
//!   Several planes meeting in a box are clipped one after another from it, the polyhedron's
//!   volume by the divergence theorem ([`clipped_volume`]).
//! - **Quadrics, by slices.** The planes across one of the grid's axes cut an ellipsoid, an
//!   elliptic cylinder, a cylinder or a cone in ellipses, each one's area in the cell exact by
//!   clipping its arc ([`super::boundary`]). The area is analytic in the slice's place but where
//!   the ellipse vanishes, touches a side of the cell or passes through a corner: the roots of
//!   quadratics, found in closed form. Between them it is integrated by 20-point Gauss–Legendre
//!   after the substitution v = a + (b − a) sin²(πτ/2), which makes the square-root behaviour at
//!   those points analytic, halving each piece until two halves agree with the whole to 1e-11
//!   of the piece's box.
//! - **What two surfaces cross.** A boolean, and a cylinder or cone along an oblique axis (its
//!   side against its caps' slab): a cell crossed by one surface alone takes that surface's own
//!   exact fill; one that two cross is halved along each axis, down to [`DEPTH`] levels, where
//!   each surface is its tangent plane through its nearest point and the planes are clipped
//!   exactly. That error falls as the cube of the last level's size along the curve where the
//!   surfaces meet, and Richardson's extrapolation from the last level and its halves removes it
//!   to the next order.
//! - **Sloped walls.** An extrusion's walls are planar between its outline's vertices: a part of
//!   a cell whose planes are the same at its centre and its eight corners is taken from them,
//!   exactly; the rest (round corners, a curved outline) is halved to [`DEPTH`] levels, its
//!   tangent planes' error, which falls as the square of the size over a surface, extrapolated
//!   away the same way.

use std::f64::consts::{PI, TAU};

use super::body::{Body, Bounds3d, Frustum, Kind, cross, dot, norm, scale, sub};
use super::boundary::{self, Conic, Piece};
use super::{Operation, Region, Shape};

/// The levels a cell two surfaces cross is halved to.
pub const DEPTH: u32 = 6;

/// The share of a box of `size` about the origin on the side of the plane n·x + d = 0 that n
/// points away from (n·x + d < 0), n a unit vector: exact.
///
/// With tᵢ ∈ [0, 1] across the box and aᵢ = |nᵢ| sizeᵢ, it is the volume of Σ aᵢtᵢ < s,
/// s = Σaᵢ/2 − d. Along the axis of the smallest a, the area of the rest's cut is piecewise
/// quadratic between the sums of the rest's a, so two-point Gauss–Legendre on each piece is
/// exact, with no division by a small a.
pub(crate) fn plane_share(n: [f64; 3], d: f64, size: [f64; 3]) -> f64 {
    let mut a: Vec<f64> = (0..3).map(|i| n[i].abs() * size[i]).collect();
    let total: f64 = a.iter().sum();
    let s = 0.5 * total - d;
    if s <= 0.0 {
        return 0.0;
    }
    if s >= total {
        return 1.0;
    }
    a.retain(|&v| v > 1e-14 * total);
    a.sort_by(f64::total_cmp);
    // the 1D and 2D cuts, the larger a last
    let line = |s: f64, a: f64| (s / a).clamp(0.0, 1.0);
    let area = |s: f64, a: f64, b: f64| -> f64 {
        let half = |s: f64| {
            if s <= 0.0 {
                0.0
            } else if s <= a {
                s * s / (2.0 * a * b)
            } else {
                (2.0 * s - a) / (2.0 * b)
            }
        };
        if 2.0 * s <= a + b {
            half(s)
        } else {
            1.0 - half(a + b - s)
        }
    };
    match a.len() {
        1 => line(s, a[0]),
        2 => area(s, a[0], a[1]),
        _ => {
            // ∫₀¹ area(s − a₀t) dt, split where s − a₀t crosses 0, a₁, a₂ or a₁ + a₂
            let mut cuts = vec![0.0, 1.0];
            for b in [0.0, a[1], a[2], a[1] + a[2]] {
                let t = (s - b) / a[0];
                if t > 0.0 && t < 1.0 {
                    cuts.push(t);
                }
            }
            cuts.sort_by(f64::total_cmp);
            let g = 0.5 / 3f64.sqrt();
            cuts.windows(2)
                .map(|w| {
                    let (mid, half) = (0.5 * (w[0] + w[1]), w[1] - w[0]);
                    let at = |t: f64| area(s - a[0] * t, a[1], a[2]);
                    0.5 * half * (at(mid - g * half) + at(mid + g * half))
                })
                .sum()
        }
    }
}

impl Body {
    /// The volume of the body inside the axis-aligned `cell`, µm³. Exact (to round-off) for
    /// planar faces, by clipping, and for the quadrics (ellipsoids, elliptic cylinders, and
    /// cylinders and cones along x, y or z) by their slices. Where two surfaces cross the cell
    /// (booleans, oblique cylinders and cones) or a sloped wall curves, to a tolerance: each is
    /// its tangent plane in the smallest of [`DEPTH`] halvings, the planes' error extrapolated
    /// away (see the module; within about 1e-7 of the volume in the checks). An extrusion with
    /// vertical walls is its shape's [`Region::fill`] times the heights: exact for polygons, to
    /// the shape's own tolerance for curves.
    pub fn fill(&self, cell: Bounds3d) -> f64 {
        if cell.is_empty() || !self.bounds().overlaps(&cell) {
            return 0.0;
        }
        match self.kind() {
            Kind::HalfSpace { point, normal } => {
                let d = dot(*normal, sub(cell.centre(), *point));
                plane_share(*normal, d, cell.size()) * cell.volume()
            }
            Kind::Ellipsoid {
                center,
                inverse,
                axes,
            } => ellipsoid_fill(*center, *inverse, axes, &cell),
            Kind::Extruded { shape, .. } => shape.fill(cell.plan()) * (cell.max[2] - cell.min[2]),
            Kind::Etched { shape, z, run, .. } => {
                let (lo, hi) = (cell.min[2].max(z[0]), cell.max[2].min(z[1]));
                if hi.is_nan() || lo.is_nan() || hi <= lo {
                    return 0.0;
                }
                if *run == 0.0 {
                    return shape.fill(cell.plan()) * (hi - lo);
                }
                if let Shape::Circle { center, radius } = shape {
                    // a cone along z: radius R + (z₁ − z) run
                    let r = radius.to_um();
                    let slices = Slices::new(
                        2,
                        [center.x.to_um(), center.y.to_um(), z[1]],
                        [[1.0, 0.0], [0.0, 1.0]],
                        [0.0, 0.0],
                        [r * r, -2.0 * r * run, run * run],
                        *z,
                    );
                    return slices.map_or(0.0, |s| s.fill(&cell));
                }
                subdivided(self, cell, DEPTH)
            }
            Kind::Cuboid { min, max } => Bounds3d {
                min: *min,
                max: *max,
            }
            .overlap(&cell),
            Kind::Frustum(f) => frustum_fill(self, f, &cell),
            Kind::Boolean { op, operands } => {
                boolean_fill(*op, &operands[0], &operands[1], cell, DEPTH)
            }
        }
    }
}

/// How a cell meets a body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// Wholly outside.
    Out,
    /// Wholly inside.
    In,
    /// Its surface may cross it.
    Cross,
}

/// The cell against the body: its distance at the cell's centre (exact, or never more than the
/// true distance) against the half-diagonal.
fn state(body: &Body, cell: &Bounds3d) -> State {
    if !body.bounds().overlaps(cell) {
        return State::Out;
    }
    let half = 0.5 * norm(cell.size());
    let (d, _) = body.distance_um(cell.centre());
    if d >= half {
        State::Out
    } else if d <= -half {
        State::In
    } else {
        State::Cross
    }
}

/// The eight halves of a cell.
fn children(cell: &Bounds3d) -> [Bounds3d; 8] {
    let c = cell.centre();
    std::array::from_fn(|k| {
        let mut b = *cell;
        for (axis, &ca) in c.iter().enumerate() {
            if (k >> axis) & 1 == 0 {
                b.max[axis] = ca;
            } else {
                b.min[axis] = ca;
            }
        }
        b
    })
}

/// The volumes of A and B and of A ∩ B in a cell combined by `op`.
fn combined(op: Operation, a: f64, b: f64, ab: f64) -> f64 {
    let v = match op {
        Operation::Union => a + b - ab,
        Operation::Intersection => ab,
        Operation::Difference => a - ab,
        Operation::Xor => a + b - 2.0 * ab,
    };
    v.max(0.0)
}

/// A boolean's volume in `cell`: where at most one operand's surface crosses the cell, from that
/// operand's own fill; where both do, from the halves, `depth` levels down, then from each
/// operand's tangent planes ([`two_planes`]).
fn boolean_fill(op: Operation, a: &Body, b: &Body, cell: Bounds3d, depth: u32) -> f64 {
    let (sa, sb) = (state(a, &cell), state(b, &cell));
    let v = cell.volume();
    if sa == State::Cross && sb == State::Cross {
        if depth > 0 {
            return children(&cell)
                .iter()
                .map(|c| boolean_fill(op, a, b, *c, depth - 1))
                .sum();
        }
        let leaf = |c: &Bounds3d| {
            let p = c.centre();
            let (pa, pb) = (a.planes(p), b.planes(p));
            let size = c.size();
            let both: Vec<(f64, [f64; 3])> = pa.iter().chain(&pb).copied().collect();
            let w = c.volume();
            combined(
                op,
                clipped_volume(size, &pa) * w,
                clipped_volume(size, &pb) * w,
                clipped_volume(size, &both) * w,
            )
        };
        return extrapolated(&leaf, &cell);
    }
    let own = |s: State, body: &Body| match s {
        State::Out => 0.0,
        State::In => v,
        State::Cross => body.fill(cell),
    };
    let (va, vb) = (own(sa, a), own(sb, b));
    let ab = match (sa, sb) {
        (State::Out, _) | (_, State::Out) => 0.0,
        (State::In, _) => vb,
        (_, State::In) => va,
        (State::Cross, State::Cross) => unreachable!("handled above"),
    };
    combined(op, va, vb, ab)
}

/// A body's volume in `cell` by halving where its surface crosses, `depth` levels down at most:
/// a part whose planes ([`Body::planes`]) are the same at its centre and its eight corners is
/// planar-faced there and taken from them, exactly; the last level from its halves' planes and
/// its own, Richardson's extrapolation of their second-order error, (4 V_halves − V_whole)/3.
/// For the sloped walls of an extrusion: exact where the walls are planar, tangent planes where
/// they curve.
fn subdivided(body: &Body, cell: Bounds3d, depth: u32) -> f64 {
    match state(body, &cell) {
        State::Out => 0.0,
        State::In => cell.volume(),
        State::Cross => {
            let whole = planed(body, &cell);
            if planar_across(body, &cell) {
                return whole;
            }
            let halves = children(&cell);
            if depth == 0 {
                let parts: f64 = halves
                    .iter()
                    .map(|c| match state(body, c) {
                        State::Out => 0.0,
                        State::In => c.volume(),
                        State::Cross => planed(body, c),
                    })
                    .sum();
                return ((4.0 * parts - whole) / 3.0).clamp(0.0, cell.volume());
            }
            halves.iter().map(|c| subdivided(body, *c, depth - 1)).sum()
        }
    }
}

/// A cell's volume where two curved surfaces meet in it, from tangent planes (`leaf`): taken
/// over the cell and over its eight halves, and their error, which falls as the cube of the
/// cell's size along the curve the surfaces meet in, extrapolated away, (8 V_halves − V_whole)/7.
fn extrapolated(leaf: &dyn Fn(&Bounds3d) -> f64, cell: &Bounds3d) -> f64 {
    let whole = leaf(cell);
    let halves: f64 = children(cell).iter().map(leaf).sum();
    ((8.0 * halves - whole) / 7.0).clamp(0.0, cell.volume())
}

/// The volume of `cell` inside the body's planes at its centre.
fn planed(body: &Body, cell: &Bounds3d) -> f64 {
    clipped_volume(cell.size(), &body.planes(cell.centre())) * cell.volume()
}

/// Whether the body's planes are the same planes at the cell's centre and at its eight
/// corners: its surface there is planar faces, which its planes hold exactly.
fn planar_across(body: &Body, cell: &Bounds3d) -> bool {
    let c = cell.centre();
    let at_centre = body.planes(c);
    let tol = 1e-12 * norm(cell.size());
    // each plane as its normal and its offset from the origin, n·x + e
    let offset = |(d, n): (f64, [f64; 3]), p: [f64; 3]| d - dot(n, p);
    (0..8).all(|k| {
        let corner: [f64; 3] = std::array::from_fn(|a| {
            if (k >> a) & 1 == 0 {
                cell.min[a]
            } else {
                cell.max[a]
            }
        });
        let there = body.planes(corner);
        there.len() == at_centre.len()
            && there.iter().zip(&at_centre).all(|(&x, &y)| {
                (0..3).all(|a| (x.1[a] - y.1[a]).abs() <= 1e-12)
                    && (offset(x, corner) - offset(y, c)).abs() <= tol
            })
    })
}

/// A cylinder's or cone's volume in `cell`: by slices across its axis when that is a grid
/// axis; otherwise as its side (a quadric, by slices across the grid axis nearest its axis)
/// meets the slab between its caps.
fn frustum_fill(body: &Body, f: &Frustum, cell: &Bounds3d) -> f64 {
    let [r0, r1] = f.radii;
    let kappa_of = |sign: f64| (r1 - r0) * sign / f.length;
    if let Some(s) = (0..3).find(|&k| f.axis[k].abs() == 1.0) {
        let kappa = kappa_of(f.axis[s]);
        let end = f.base[s] + f.axis[s] * f.length;
        let slices = Slices::new(
            s,
            f.base,
            [[1.0, 0.0], [0.0, 1.0]],
            [0.0, 0.0],
            [r0 * r0, 2.0 * r0 * kappa, kappa * kappa],
            [f.base[s].min(end), f.base[s].max(end)],
        );
        return slices.map_or(0.0, |sl| sl.fill(cell));
    }
    match Lateral::of(f) {
        Some(lateral) => lateral_fill(body, f, &lateral, *cell, DEPTH),
        None => subdivided(body, *cell, DEPTH),
    }
}

/// The side of an oblique cylinder or cone, without its caps: the quadric (q − c)ᵀ M (q − c) ≤
/// κ, and its slices.
struct Lateral {
    slices: Slices,
    /// For a cone, its apex's place along the axis from the base and the slope of its radius;
    /// `None` for a cylinder.
    apex: Option<(f64, f64)>,
}

impl Lateral {
    fn of(f: &Frustum) -> Option<Lateral> {
        let [r0, r1] = f.radii;
        let a = f.axis;
        let outer = |s: f64| -> [[f64; 3]; 3] {
            std::array::from_fn(|i| {
                std::array::from_fn(|j| f64::from(u8::from(i == j)) - s * a[i] * a[j])
            })
        };
        let (c, m, kappa, apex) = if r0 == r1 {
            (f.base, outer(1.0), r0 * r0, None)
        } else {
            let k = (r1 - r0) / f.length;
            let t = -r0 / k;
            (
                super::body::add(f.base, scale(a, t)),
                outer(1.0 + k * k),
                0.0,
                Some((t, k)),
            )
        };
        // the grid axis nearest the frustum's: its planes cut the side in ellipses
        let mut axes = [0usize, 1, 2];
        axes.sort_by(|&i, &j| a[j].abs().total_cmp(&a[i].abs()));
        let s = axes[0];
        let slices = Slices::quadric(
            c,
            m,
            kappa,
            s,
            [f64::NEG_INFINITY, f64::INFINITY],
            apex.is_none(),
        )?;
        Some(Lateral { slices, apex })
    }

    /// The cell against the side: the distance in the plane through the axis to the side's
    /// generators (never more than the true distance), against the half-diagonal.
    fn state(&self, f: &Frustum, cell: &Bounds3d) -> State {
        let half = 0.5 * norm(cell.size());
        let q = sub(cell.centre(), f.base);
        let t = dot(q, f.axis);
        let rho = norm(sub(q, scale(f.axis, t)));
        let d = match self.apex {
            None => rho - f.radii[0],
            Some((ta, k)) => {
                // the double cone ρ ≤ |k| |t − ta|: its two generators in this plane
                let (k, tau) = (k.abs(), t - ta);
                let l = k.hypot(1.0);
                let inside = rho <= k * tau.abs();
                let along = |sign: f64| {
                    // the distance to the ray from the apex
                    let e = [sign / l, k / l];
                    let s = (tau * e[0] + rho * e[1]).max(0.0);
                    (tau - s * e[0]).hypot(rho - s * e[1])
                };
                let near = along(1.0).min(along(-1.0));
                if inside { -near } else { near }
            }
        };
        if d >= half {
            State::Out
        } else if d <= -half {
            State::In
        } else {
            State::Cross
        }
    }
}

/// An oblique frustum's volume in `cell`: its side against its caps' slab, as a boolean
/// intersection of the two.
fn lateral_fill(body: &Body, f: &Frustum, lateral: &Lateral, cell: Bounds3d, depth: u32) -> f64 {
    let v = cell.volume();
    let half = 0.5 * norm(cell.size());
    let c = cell.centre();
    let t = dot(sub(c, f.base), f.axis);
    // the slab between the caps, t from 0 to L
    let (below, above) = (-t, t - f.length);
    let slab = if below >= half || above >= half {
        State::Out
    } else if below <= -half && above <= -half {
        State::In
    } else {
        State::Cross
    };
    let side = lateral.state(f, &cell);
    match (slab, side) {
        (State::Out, _) | (_, State::Out) => 0.0,
        (State::In, State::In) => v,
        (State::In, State::Cross) => lateral.slices.fill(&cell),
        (State::Cross, State::In) => {
            let size = cell.size();
            let share = plane_share(scale(f.axis, -1.0), below, size)
                + plane_share(f.axis, above, size)
                - 1.0;
            share.max(0.0) * v
        }
        (State::Cross, State::Cross) => {
            if depth == 0 {
                return extrapolated(&|c: &Bounds3d| planed(body, c), &cell);
            }
            children(&cell)
                .iter()
                .map(|k| lateral_fill(body, f, lateral, *k, depth - 1))
                .sum()
        }
    }
}

/// An ellipsoid's volume in `cell`: by slices (an elliptic cylinder's across the grid axis
/// nearest its own), or a slab's by its two planes.
fn ellipsoid_fill(
    center: [f64; 3],
    inverse: [f64; 3],
    axes: &[[f64; 3]; 3],
    cell: &Bounds3d,
) -> f64 {
    let finite: Vec<usize> = (0..3).filter(|&i| inverse[i] > 0.0).collect();
    if let [i] = finite[..] {
        // |n·(q − c)| ≤ s: two half-spaces whose union is everything
        let (n, s) = (axes[i], 1.0 / inverse[i]);
        let d = dot(n, sub(cell.centre(), center));
        let size = cell.size();
        let share = plane_share(n, d - s, size) + plane_share(scale(n, -1.0), -d - s, size) - 1.0;
        return share.max(0.0) * cell.volume();
    }
    let m: [[f64; 3]; 3] = std::array::from_fn(|j| {
        std::array::from_fn(|k| {
            (0..3)
                .map(|i| inverse[i] * inverse[i] * axes[i][j] * axes[i][k])
                .sum()
        })
    });
    let cylinder = finite.len() == 2;
    let s = if cylinder {
        let along = (0..3).find(|&i| inverse[i] == 0.0).unwrap_or(2);
        (0..3)
            .max_by(|&j, &k| axes[along][j].abs().total_cmp(&axes[along][k].abs()))
            .unwrap_or(2)
    } else {
        2
    };
    Slices::quadric(
        center,
        m,
        1.0,
        s,
        [f64::NEG_INFINITY, f64::INFINITY],
        cylinder,
    )
    .map_or(0.0, |sl| sl.fill(cell))
}

/// An ellipse in the plane: its centre, its semi-axes, and the first's angle from x.
type Ellipse = ([f64; 2], [f64; 2], f64);

/// The ellipse the plane at height `z` cuts from an ellipsoid or an elliptic cylinder, as its
/// centre (x, y), semi-axes and the first's angle from x: `None` when the planes across z don't
/// cut it in ellipses (a cylinder lying along the plane, a slab), `Some(None)` where this one
/// misses it.
pub(crate) fn ellipsoid_section(
    center: [f64; 3],
    inverse: [f64; 3],
    axes: &[[f64; 3]; 3],
    z: f64,
) -> Option<Option<Ellipse>> {
    let finite = (0..3).filter(|&i| inverse[i] > 0.0).count();
    if finite < 2 {
        return None;
    }
    let m: [[f64; 3]; 3] = std::array::from_fn(|j| {
        std::array::from_fn(|k| {
            (0..3)
                .map(|i| inverse[i] * inverse[i] * axes[i][j] * axes[i][k])
                .sum()
        })
    });
    let range = [f64::NEG_INFINITY, f64::INFINITY];
    let slices = Slices::quadric(center, m, 1.0, 2, range, finite == 2)?;
    Some(slices.ellipse_at(z))
}

/// A family of ellipses: across the grid axis `s`, at v = origin + δ, the points w of the other
/// two coordinates with (w − w₀(δ))ᵀ A (w − w₀(δ)) ≤ q(δ), w₀ = centre + δ drift, q quadratic,
/// for v in `range`.
#[derive(Clone, Copy, Debug)]
struct Slices {
    s: usize,
    plane: [usize; 2],
    origin: f64,
    centre: [f64; 2],
    drift: [f64; 2],
    a: [[f64; 2]; 2],
    q: [f64; 3],
    range: [f64; 2],
    /// (A⁻¹)ₖₖ: the squared half-width along each axis of the ellipse q = 1.
    reach: [f64; 2],
    det: f64,
    /// The semi-axes of the ellipse q = 1, the first along (cos, sin).
    semi: [f64; 2],
    cos: f64,
    sin: f64,
}

impl Slices {
    /// The family across axis `s` about `centre` (its own coordinate the origin of δ), `None`
    /// unless A is positive definite.
    fn new(
        s: usize,
        centre: [f64; 3],
        a: [[f64; 2]; 2],
        drift: [f64; 2],
        q: [f64; 3],
        range: [f64; 2],
    ) -> Option<Slices> {
        let plane = match s {
            0 => [1, 2],
            1 => [0, 2],
            _ => [0, 1],
        };
        let det = a[0][0] * a[1][1] - a[0][1] * a[1][0];
        if !(det > 0.0 && a[0][0] > 0.0 && det.is_finite()) {
            return None;
        }
        let (p, r, t) = (a[0][0], a[0][1], a[1][1]);
        let mean = 0.5 * (p + t);
        let radius = (0.5 * (p - t)).hypot(r);
        let (l1, l2) = (mean + radius, mean - radius);
        if l2.is_nan() || l2 <= 0.0 {
            return None;
        }
        let psi = 0.5 * (2.0 * r).atan2(p - t);
        Some(Slices {
            s,
            plane,
            origin: centre[s],
            centre: [centre[plane[0]], centre[plane[1]]],
            drift,
            a,
            q,
            range,
            reach: [t / det, p / det],
            det,
            semi: [1.0 / l1.sqrt(), 1.0 / l2.sqrt()],
            cos: psi.cos(),
            sin: psi.sin(),
        })
    }

    /// The slices across axis `s` of the quadric (q − c)ᵀ M (q − c) ≤ κ: A the part of M in the
    /// plane, w₀ moving by −A⁻¹b per unit along s (b the plane's column of M along s), and q(δ) =
    /// κ − σδ², σ = m − bᵀA⁻¹b; for a cylinder (`translated`) σ is zero exactly.
    fn quadric(
        c: [f64; 3],
        m: [[f64; 3]; 3],
        kappa: f64,
        s: usize,
        range: [f64; 2],
        translated: bool,
    ) -> Option<Slices> {
        let plane = match s {
            0 => [1, 2],
            1 => [0, 2],
            _ => [0, 1],
        };
        let a = [
            [m[plane[0]][plane[0]], m[plane[0]][plane[1]]],
            [m[plane[1]][plane[0]], m[plane[1]][plane[1]]],
        ];
        let b = [m[plane[0]][s], m[plane[1]][s]];
        let det = a[0][0] * a[1][1] - a[0][1] * a[1][0];
        if det.is_nan() || det <= 0.0 {
            return None;
        }
        let g = [
            (a[1][1] * b[0] - a[0][1] * b[1]) / det,
            (a[0][0] * b[1] - a[1][0] * b[0]) / det,
        ];
        let sigma = if translated {
            0.0
        } else {
            m[s][s] - (b[0] * g[0] + b[1] * g[1])
        };
        Slices::new(s, c, a, [-g[0], -g[1]], [kappa, 0.0, -sigma], range)
    }

    fn q(&self, delta: f64) -> f64 {
        self.q[0] + delta * (self.q[1] + delta * self.q[2])
    }

    /// ∫ q(δ) dδ from δ₀ to δ₁.
    fn q_integral(&self, d0: f64, d1: f64) -> f64 {
        let p = |d: f64| d * (self.q[0] + d * (0.5 * self.q[1] + d * self.q[2] / 3.0));
        p(d1) - p(d0)
    }

    /// The slice at v: its centre, its semi-axes and the first's angle; `None` where it is
    /// empty.
    fn ellipse_at(&self, v: f64) -> Option<Ellipse> {
        let delta = v - self.origin;
        let q = self.q(delta);
        if q.is_nan() || q <= 0.0 || v < self.range[0] || v > self.range[1] {
            return None;
        }
        let k = q.sqrt();
        Some((
            [
                self.centre[0] + delta * self.drift[0],
                self.centre[1] + delta * self.drift[1],
            ],
            [self.semi[0] * k, self.semi[1] * k],
            self.sin.atan2(self.cos),
        ))
    }

    /// The slice's area inside `rect` (in the plane's two coordinates) at v.
    fn area(&self, v: f64, rect: [[f64; 2]; 2]) -> f64 {
        let delta = v - self.origin;
        let q = self.q(delta);
        if q.is_nan() || q <= 0.0 {
            return 0.0;
        }
        let k = q.sqrt();
        let conic = Conic {
            c: [
                self.centre[0] + delta * self.drift[0],
                self.centre[1] + delta * self.drift[1],
            ],
            rx: self.semi[0] * k,
            ry: self.semi[1] * k,
            cos: self.cos,
            sin: self.sin,
            t0: 0.0,
            t1: TAU,
        };
        boundary::fill(&[vec![Piece::Conic(conic)]], rect).max(0.0)
    }

    /// The places along s, from `lo` to `hi`, where the slice's area in `rect` may not be
    /// analytic: where it vanishes, touches a side of the rectangle, or passes through a corner.
    fn breaks(&self, rect: [[f64; 2]; 2], lo: f64, hi: f64) -> Vec<f64> {
        let mut out = vec![lo, hi];
        let mut push = |a: f64, b: f64, c: f64| {
            for delta in roots(a, b, c) {
                let v = self.origin + delta;
                if v > lo && v < hi {
                    out.push(v);
                }
            }
        };
        let [q0, q1, q2] = self.q;
        push(q2, q1, q0);
        let d = self.drift;
        for k in 0..2 {
            for side in [rect[0][k], rect[1][k]] {
                // (e − δ dₖ)² = (A⁻¹)ₖₖ q(δ)
                let e = side - self.centre[k];
                let h = self.reach[k];
                push(
                    d[k] * d[k] - h * q2,
                    -2.0 * e * d[k] - h * q1,
                    e * e - h * q0,
                );
            }
        }
        let a = self.a;
        let ad = [
            a[0][0] * d[0] + a[0][1] * d[1],
            a[1][0] * d[0] + a[1][1] * d[1],
        ];
        for x in [rect[0][0], rect[1][0]] {
            for y in [rect[0][1], rect[1][1]] {
                // (e − δd)ᵀ A (e − δd) = q(δ)
                let e = [x - self.centre[0], y - self.centre[1]];
                let ae = [
                    a[0][0] * e[0] + a[0][1] * e[1],
                    a[1][0] * e[0] + a[1][1] * e[1],
                ];
                push(
                    d[0] * ad[0] + d[1] * ad[1] - q2,
                    -2.0 * (d[0] * ae[0] + d[1] * ae[1]) - q1,
                    e[0] * ae[0] + e[1] * ae[1] - q0,
                );
            }
        }
        out.sort_by(f64::total_cmp);
        out.dedup();
        out
    }

    /// The family's volume in `cell`.
    fn fill(&self, cell: &Bounds3d) -> f64 {
        let (lo, hi) = (
            cell.min[self.s].max(self.range[0]),
            cell.max[self.s].min(self.range[1]),
        );
        if hi.is_nan() || lo.is_nan() || hi <= lo {
            return 0.0;
        }
        let [u, w] = self.plane;
        let rect = [[cell.min[u], cell.min[w]], [cell.max[u], cell.max[w]]];
        let rect_area = (rect[1][0] - rect[0][0]) * (rect[1][1] - rect[0][1]);
        self.breaks(rect, lo, hi)
            .windows(2)
            .map(|p| self.piece(rect, rect_area, p[0], p[1]))
            .sum()
    }

    /// The volume between `v0` and `v1`, where the slice's area is analytic: in closed form
    /// when the ellipse lies inside the rectangle, covers it or misses it, by quadrature where
    /// it crosses it.
    fn piece(&self, rect: [[f64; 2]; 2], rect_area: f64, v0: f64, v1: f64) -> f64 {
        if v1.is_nan() || v0.is_nan() || v1 <= v0 {
            return 0.0;
        }
        let mid = 0.5 * (v0 + v1);
        let delta = mid - self.origin;
        let q = self.q(delta);
        if q.is_nan() || q <= 0.0 {
            return 0.0;
        }
        let w0 = [
            self.centre[0] + delta * self.drift[0],
            self.centre[1] + delta * self.drift[1],
        ];
        let ext = [(self.reach[0] * q).sqrt(), (self.reach[1] * q).sqrt()];
        let (box_lo, box_hi) = (
            [w0[0] - ext[0], w0[1] - ext[1]],
            [w0[0] + ext[0], w0[1] + ext[1]],
        );
        if (0..2).any(|k| box_hi[k] <= rect[0][k] || box_lo[k] >= rect[1][k]) {
            return 0.0;
        }
        if (0..2).all(|k| box_lo[k] >= rect[0][k] && box_hi[k] <= rect[1][k]) {
            let o = self.origin;
            return PI / self.det.sqrt() * self.q_integral(v0 - o, v1 - o);
        }
        let a = self.a;
        let inside = |x: f64, y: f64| {
            let e = [x - w0[0], y - w0[1]];
            e[0] * (a[0][0] * e[0] + a[0][1] * e[1]) + e[1] * (a[1][0] * e[0] + a[1][1] * e[1]) <= q
        };
        if inside(rect[0][0], rect[0][1])
            && inside(rect[1][0], rect[0][1])
            && inside(rect[0][0], rect[1][1])
            && inside(rect[1][0], rect[1][1])
        {
            return rect_area * (v1 - v0);
        }
        let tolerance = 1e-11 * rect_area * (v1 - v0);
        adaptive(
            &|v| self.area(v, rect),
            v0,
            v1,
            mapped(&|v| self.area(v, rect), v0, v1),
            tolerance,
            10,
        )
    }
}

/// ∫ f from a to b by 20-point Gauss–Legendre after v = a + (b − a) sin²(πτ/2), τ from 0 to 1:
/// square-root behaviour at either end becomes analytic.
fn mapped(f: &dyn Fn(f64) -> f64, a: f64, b: f64) -> f64 {
    let h = b - a;
    boundary::gauss(
        |tau| {
            let (s, c) = (PI * tau).sin_cos();
            f(a + h * 0.5 * (1.0 - c)) * h * 0.5 * PI * s
        },
        0.0,
        1.0,
    )
}

/// `whole` (∫ f from a to b, [`mapped`]) refined: the halves taken until they agree with their
/// whole to `tolerance`, at most `depth` levels down.
fn adaptive(f: &dyn Fn(f64) -> f64, a: f64, b: f64, whole: f64, tolerance: f64, depth: u32) -> f64 {
    let m = 0.5 * (a + b);
    let (left, right) = (mapped(f, a, m), mapped(f, m, b));
    if depth == 0 || (left + right - whole).abs() <= tolerance {
        return left + right;
    }
    adaptive(f, a, m, left, 0.5 * tolerance, depth - 1)
        + adaptive(f, m, b, right, 0.5 * tolerance, depth - 1)
}

/// The real roots of a x² + b x + c = 0 (of b x + c where a is zero).
fn roots(a: f64, b: f64, c: f64) -> Vec<f64> {
    let scale = a.abs().max(b.abs()).max(c.abs());
    if scale == 0.0 || !scale.is_finite() {
        return Vec::new();
    }
    if a.abs() <= 1e-14 * scale {
        return if b != 0.0 { vec![-c / b] } else { Vec::new() };
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return Vec::new();
    }
    let q = -0.5 * (b + disc.sqrt().copysign(b));
    if q == 0.0 {
        return vec![0.0];
    }
    vec![q / a, c / q]
}

/// The share of the box of `size` about the origin where n·x + d ≤ 0 for every plane (d, n):
/// the box clipped by each plane in turn, Sutherland and Hodgman's clipping of each face with
/// the cut closing it, its volume by the divergence theorem. Exact; one plane is
/// [`plane_share`].
pub(crate) fn clipped_volume(size: [f64; 3], planes: &[(f64, [f64; 3])]) -> f64 {
    let h = size.map(|v| 0.5 * v);
    let mut active: Vec<(f64, [f64; 3])> = Vec::new();
    for &(d, n) in planes {
        let reach: f64 = (0..3).map(|k| n[k].abs() * h[k]).sum();
        if d >= reach {
            return 0.0;
        }
        if d > -reach {
            active.push((d, n));
        }
    }
    match active.len() {
        0 => return 1.0,
        1 => return plane_share(active[0].1, active[0].0, size),
        _ => {}
    }
    let total: f64 = size.iter().product();
    let mut faces = box_faces(h);
    for (d, n) in active {
        faces = clip(&faces, n, d);
        if faces.is_empty() {
            return 0.0;
        }
    }
    (volume(&faces) / total).clamp(0.0, 1.0)
}

type Face = Vec<[f64; 3]>;

/// A box's faces, each counterclockwise seen from outside.
fn box_faces(h: [f64; 3]) -> Vec<Face> {
    let mut faces = Vec::with_capacity(6);
    for k in 0..3 {
        let (i, j) = ((k + 1) % 3, (k + 2) % 3);
        for sign in [1.0, -1.0] {
            let corner = |u: f64, w: f64| {
                let mut p = [0.0; 3];
                p[k] = sign * h[k];
                p[i] = u * h[i];
                p[j] = w * h[j];
                p
            };
            let mut f = vec![
                corner(-1.0, -1.0),
                corner(1.0, -1.0),
                corner(1.0, 1.0),
                corner(-1.0, 1.0),
            ];
            if sign < 0.0 {
                f.reverse();
            }
            faces.push(f);
        }
    }
    faces
}

/// The polyhedron `faces` less the side of the plane n·x + d = 0 that n points to, closed by
/// the cut.
fn clip(faces: &[Face], n: [f64; 3], d: f64) -> Vec<Face> {
    let f = |p: [f64; 3]| dot(n, p) + d;
    let mut out: Vec<Face> = Vec::with_capacity(faces.len() + 1);
    let mut cut: Vec<[f64; 3]> = Vec::new();
    for face in faces {
        let mut kept: Face = Vec::with_capacity(face.len() + 2);
        for k in 0..face.len() {
            let (p, q) = (face[k], face[(k + 1) % face.len()]);
            let (fp, fq) = (f(p), f(q));
            if fp <= 0.0 {
                kept.push(p);
                if fp == 0.0 {
                    cut.push(p);
                }
            }
            if (fp < 0.0 && fq > 0.0) || (fp > 0.0 && fq < 0.0) {
                // the crossing from the lesser end, so both faces of the edge get the same bits
                let (a, b, fa, fb) = if p < q {
                    (p, q, fp, fq)
                } else {
                    (q, p, fq, fp)
                };
                let t = fa / (fa - fb);
                let x = std::array::from_fn(|i| a[i] + t * (b[i] - a[i]));
                kept.push(x);
                cut.push(x);
            }
        }
        if kept.len() >= 3 {
            out.push(kept);
        }
    }
    cut.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    cut.dedup();
    if cut.len() >= 3 {
        // the cut's points counterclockwise about n
        let m: [f64; 3] =
            std::array::from_fn(|i| cut.iter().map(|p| p[i]).sum::<f64>() / cut.len() as f64);
        let least = (0..3)
            .min_by(|&i, &j| n[i].abs().total_cmp(&n[j].abs()))
            .unwrap_or(0);
        let mut e = [0.0; 3];
        e[least] = 1.0;
        let u = cross(n, e);
        let u = scale(u, 1.0 / norm(u));
        let w = cross(n, u);
        cut.sort_by(|a, b| {
            let (pa, pb) = (sub(*a, m), sub(*b, m));
            let ta = dot(pa, w).atan2(dot(pa, u));
            let tb = dot(pb, w).atan2(dot(pb, u));
            ta.total_cmp(&tb)
        });
        out.push(cut);
    }
    out
}

/// The volume a closed polyhedron's faces (counterclockwise from outside) bound: the sum of the
/// tetrahedra from the origin to each face's fan.
fn volume(faces: &[Face]) -> f64 {
    faces
        .iter()
        .map(|f| {
            (1..f.len() - 1)
                .map(|k| dot(f[0], cross(f[k], f[k + 1])))
                .sum::<f64>()
        })
        .sum::<f64>()
        / 6.0
}
