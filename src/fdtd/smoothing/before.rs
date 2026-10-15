//! FDTD's bodies as they were before #227's phase 9 moved them to [`crate::geometry::Body`]
//! (commit ff50758), their arithmetic word for word: what the checks compare the shared bodies
//! with, bit for bit, on whatever system they run ([`super::checks::bodies_differences`]).

use crate::Result;
use crate::geometry::{Point, Shape};

type Matrix = [[f64; 3]; 3];

fn invalid(reason: impl Into<String>) -> crate::Error {
    crate::Error::invalid("fdtd", reason)
}
/// A region of space filled with one medium. Bodies are made by their functions, which check
/// them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Body(Kind);

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    HalfSpace {
        point: [f64; 3],
        normal: [f64; 3],
    },
    Ellipsoid {
        center: [f64; 3],
        /// The reciprocal semi-axes, 0 for an infinite one.
        inverse: [f64; 3],
        axes: [[f64; 3]; 3],
    },
    Extruded {
        shape: Shape,
        /// Its bounding box, µm: the least x and y, then the greatest.
        bounds: [f64; 4],
    },
    Etched {
        shape: Shape,
        /// The top face's bounding box, µm: the least x and y, then the greatest.
        bounds: [f64; 4],
        /// Its bottom and top, µm.
        z: [f64; 2],
        /// How far the walls reach out per µm down, cot θ.
        run: f64,
    },
}

impl Body {
    /// The half-space on the side of the plane through `point` (µm) that `normal` (any length
    /// but zero) points away from.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless everything is finite and the normal isn't zero.
    pub(crate) fn half_space(point: [f64; 3], normal: [f64; 3]) -> Result<Body> {
        let length = norm(normal);
        if !(point.iter().all(|v| v.is_finite()) && length.is_finite() && length > 0.0) {
            return Err(invalid(format!(
                "a half-space needs a finite point and normal, got {point:?} and {normal:?}"
            )));
        }
        Ok(Body(Kind::HalfSpace {
            point,
            normal: normal.map(|v| v / length),
        }))
    }

    /// An ellipsoid about `center` (µm) with semi-axes `semi_axes` (µm) along the orthonormal
    /// directions `axes` (one per row). A semi-axis may be infinite: one makes an elliptic
    /// cylinder, two a slab.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless the centre is finite, the semi-axes positive (at
    /// most two infinite) and the axes orthonormal to 1e-9.
    pub(crate) fn ellipsoid(
        center: [f64; 3],
        semi_axes: [f64; 3],
        axes: [[f64; 3]; 3],
    ) -> Result<Body> {
        let finite = semi_axes.iter().filter(|s| s.is_finite()).count();
        if !(center.iter().all(|v| v.is_finite())
            && semi_axes.iter().all(|s| *s > 0.0)
            && finite >= 1
            && orthonormal(&axes))
        {
            return Err(invalid(format!(
                "an ellipsoid needs a finite centre, positive semi-axes (one at least finite) and \
                 orthonormal axes, got {center:?}, {semi_axes:?} and {axes:?}"
            )));
        }
        Ok(Body(Kind::Ellipsoid {
            center,
            inverse: semi_axes.map(|s| 1.0 / s),
            axes,
        }))
    }

    /// An elliptic cylinder along z: the ellipse about `center` (x, y, µm) with semi-axes
    /// `semi_axes` (µm), the first at `angle` (radians) from x, counterclockwise.
    ///
    /// # Errors
    ///
    /// As [`Body::ellipsoid`].
    pub(crate) fn ellipse(center: [f64; 2], semi_axes: [f64; 2], angle: f64) -> Result<Body> {
        let (s, c) = angle.sin_cos();
        Body::ellipsoid(
            [center[0], center[1], 0.0],
            [semi_axes[0], semi_axes[1], f64::INFINITY],
            [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]],
        )
    }

    /// A planar shape (as [`crate::geometry`] draws it, x and y in µm) extended without end
    /// along z.
    pub(crate) fn extruded(shape: Shape) -> Body {
        let b = shape.bounds();
        let bounds = [b.min.x, b.min.y, b.max.x, b.max.y].map(|v| v.to_um());
        Body(Kind::Extruded { shape, bounds })
    }

    /// A planar shape's etched body ([`crate::stack::Etch`]): `shape` (x and y in µm) its top
    /// face at z = `z[1]`, standing down to `z[0]`, its walls leaning out going down at
    /// `sidewall` radians from the plane (π/2: vertical, the shape extruded between the two
    /// heights). Its surface is the walls' planes through the outline's nearest point, and the
    /// two faces: a trapezoid's sides exactly, so the smoothing takes each cell's exact share
    /// of a straight sloped wall, with its normal.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] unless the heights are finite, the top above the bottom,
    /// and the sidewall above 0 and at most π/2.
    pub(crate) fn etched(shape: Shape, z: [f64; 2], sidewall: f64) -> Result<Body> {
        if !(z[0].is_finite()
            && z[1].is_finite()
            && z[1] > z[0]
            && sidewall > 0.0
            && sidewall <= std::f64::consts::FRAC_PI_2 + 1e-12)
        {
            return Err(invalid(format!(
                "an etched body needs a top above its bottom and a sidewall above 0 and at \
                 most 90 degrees, got {z:?} and {sidewall} rad"
            )));
        }
        let run = crate::stack::Etch::FULL.with_sidewall(sidewall).run();
        let b = shape.bounds();
        let bounds = [b.min.x, b.min.y, b.max.x, b.max.y].map(|v| v.to_um());
        Ok(Body(Kind::Etched {
            shape,
            bounds,
            z,
            run,
        }))
    }

    /// Whether `p` is farther than `r` (µm) from the body's bounding box, outside it: then it
    /// is outside the body and farther than `r` from its surface, known without the surface's
    /// distance, which for a polygon costs a pass over its edges. Never for the bodies without
    /// a box.
    pub(crate) fn beyond(&self, p: [f64; 3], r: f64) -> bool {
        match &self.0 {
            Kind::Extruded { bounds, .. } => {
                let dx = (bounds[0] - p[0]).max(p[0] - bounds[2]).max(0.0);
                let dy = (bounds[1] - p[1]).max(p[1] - bounds[3]).max(0.0);
                dx.hypot(dy) > r
            }
            Kind::Etched { bounds, z, run, .. } => {
                let out = (z[1] - z[0]) * run;
                let dx = (bounds[0] - out - p[0])
                    .max(p[0] - bounds[2] - out)
                    .max(0.0);
                let dy = (bounds[1] - out - p[1])
                    .max(p[1] - bounds[3] - out)
                    .max(0.0);
                let dz = (z[0] - p[2]).max(p[2] - z[1]).max(0.0);
                dx.hypot(dy).hypot(dz) > r
            }
            _ => false,
        }
    }

    /// The signed distance from `p` to the surface, negative inside (exact for a half-space, a
    /// sphere and the planar shapes, to second order in itself for an ellipsoid), and the unit
    /// normal pointing out of the body at the nearest point of the surface.
    pub(crate) fn surface(&self, p: [f64; 3]) -> (f64, [f64; 3]) {
        match &self.0 {
            Kind::HalfSpace { point, normal } => (dot(*normal, sub(p, *point)), *normal),
            Kind::Ellipsoid {
                center,
                inverse,
                axes,
            } => {
                // u in the ellipsoid's own units, |u| = 1 on its surface; f = |u| − 1 has the
                // gradient g/|u|, g = Σ uᵢ aᵢ/sᵢ, and the distance is f/|∇f|: exact for a
                // sphere, second order in itself otherwise
                let q = sub(p, *center);
                let u: [f64; 3] = std::array::from_fn(|i| dot(q, axes[i]) * inverse[i]);
                let rho = norm(u);
                let g: [f64; 3] =
                    std::array::from_fn(|k| (0..3).map(|i| u[i] * inverse[i] * axes[i][k]).sum());
                let length = norm(g);
                if rho == 0.0 || length == 0.0 {
                    // the centre: deep inside
                    let smallest = inverse.iter().fold(0.0f64, |m, v| m.max(*v));
                    return (-1.0 / smallest, axes[0]);
                }
                ((rho - 1.0) * rho / length, g.map(|v| v / length))
            }
            Kind::Extruded { shape, .. } => {
                let (d, n) = planar(shape, p[0], p[1]);
                (d, [n[0], n[1], 0.0])
            }
            Kind::Etched { shape, z, run, .. } => {
                // the wall: d − (top − z) cot θ over its gradient's length, the distance to
                // its plane through the outline's nearest point
                let (d, n) = planar(shape, p[0], p[1]);
                let length = run.hypot(1.0);
                let wall = (
                    (d - (z[1] - p[2]) * run) / length,
                    [n[0] / length, n[1] / length, run / length],
                );
                // and the faces: the body is where all three are negative
                [
                    wall,
                    (p[2] - z[1], [0.0, 0.0, 1.0]),
                    (z[0] - p[2], [0.0, 0.0, -1.0]),
                ]
                .into_iter()
                .fold((f64::NEG_INFINITY, [0.0; 3]), |best, s| {
                    if s.0 > best.0 { s } else { best }
                })
            }
        }
    }
}

/// A planar shape's signed distance from (x, y), negative inside, and its outward normal at the
/// nearest point.
fn planar(shape: &Shape, x: f64, y: f64) -> (f64, [f64; 2]) {
    let radial = |center: &Point, x: f64, y: f64| {
        let (dx, dy) = (x - center.x.to_um(), y - center.y.to_um());
        let r = dx.hypot(dy);
        let n = if r > 0.0 {
            [dx / r, dy / r]
        } else {
            [1.0, 0.0]
        };
        (r, n)
    };
    match shape {
        Shape::Rect {
            center,
            width,
            height,
        } => {
            let (dx, dy) = (x - center.x.to_um(), y - center.y.to_um());
            let q = [
                dx.abs() - width.to_um() / 2.0,
                dy.abs() - height.to_um() / 2.0,
            ];
            let sign = [dx.signum(), dy.signum()];
            if q[0] > 0.0 || q[1] > 0.0 {
                let v = [q[0].max(0.0), q[1].max(0.0)];
                let d = v[0].hypot(v[1]);
                (d, [sign[0] * v[0] / d, sign[1] * v[1] / d])
            } else if q[0] > q[1] {
                (q[0], [sign[0], 0.0])
            } else {
                (q[1], [0.0, sign[1]])
            }
        }
        Shape::Circle { center, radius } => {
            let (r, n) = radial(center, x, y);
            (r - radius.to_um(), n)
        }
        Shape::Ring {
            center,
            inner,
            outer,
        } => {
            let (r, n) = radial(center, x, y);
            let (out, inn) = (r - outer.to_um(), inner.to_um() - r);
            if out >= inn {
                (out, n)
            } else {
                (inn, [-n[0], -n[1]])
            }
        }
        Shape::Polygon(polygon) => {
            let v = polygon.vertices();
            let mut best = (f64::INFINITY, [0.0, 0.0], [1.0, 0.0]);
            for i in 0..v.len() {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                let (ax, ay, bx, by) = (a.x.to_um(), a.y.to_um(), b.x.to_um(), b.y.to_um());
                let (ex, ey) = (bx - ax, by - ay);
                let t = (((x - ax) * ex + (y - ay) * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
                let nearest = [ax + t * ex, ay + t * ey];
                let d = (x - nearest[0]).hypot(y - nearest[1]);
                if d < best.0 {
                    // counterclockwise: the edge's outward normal is (e_y, −e_x)
                    let l = ex.hypot(ey);
                    best = (d, nearest, [ey / l, -ex / l]);
                }
            }
            let (d, nearest, edge) = best;
            let inside = polygon.contains(Point::um(x, y));
            let n = if d > 0.0 {
                let away = [(x - nearest[0]) / d, (y - nearest[1]) / d];
                if inside { [-away[0], -away[1]] } else { away }
            } else {
                edge
            };
            (if inside { -d } else { d }, n)
        }
        // the other primitives answer for themselves
        other => {
            let d = crate::geometry::Region::distance(other, Point::um(x, y));
            (d.signed.to_um(), d.normal)
        }
    }
}

/// The share of a box of `size` about the origin on the side of the plane n·x + d = 0 that n
/// points away from (n·x + d < 0), n a unit vector: exact.
///
/// With tᵢ ∈ [0, 1] across the box and aᵢ = |nᵢ| sizeᵢ, it is the volume of Σ aᵢtᵢ < s,
/// s = Σaᵢ/2 − d. Along the axis of the smallest a, the area of the rest's cut is piecewise
/// quadratic between the sums of the rest's a, so two-point Gauss–Legendre on each piece is
/// exact, with no division by a small a.
pub(crate) fn fill(n: [f64; 3], d: f64, size: [f64; 3]) -> f64 {
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

fn orthonormal(axes: &Matrix) -> bool {
    (0..3).all(|i| {
        (0..3).all(|j| {
            let expected = if i == j { 1.0 } else { 0.0 };
            (dot(axes[i], axes[j]) - expected).abs() <= 1e-9
        })
    })
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
