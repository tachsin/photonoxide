//! A body's section by a horizontal plane ([`Body::section`]): the planar shape it fills at a
//! height, for drawing it (a job's scene draws a body as bands of its sections).
//!
//! Exact where the section is a shape of the plane's own: a box's rectangle, an ellipsoid's or
//! an elliptic cylinder's ellipse (when the planes cut it in ellipses), a cylinder's or cone's
//! disk along z, an extrusion's shape (grown by its sloped walls, with round joins, as
//! [`crate::stack::Etch`] has them), a half-space's half of the window, and booleans of those.
//! Otherwise (a cylinder lying along the plane, a slab, a cylinder or cone at an angle) a mask of
//! the window, each pixel the body's at its centre.

use super::body::{Body, Bounds3d, Kind};
use super::body_fill::ellipsoid_section;
use super::{Boolean, Bounds, Join, Mask, Offset, Operation, Point, Polygon, Shape};
use crate::Result;
use crate::units::Length;

/// The pixels a mask section has along the window's longer side.
const PIXELS: usize = 128;

impl Body {
    /// The body's section at height `z` (µm) within `window` (x and y): `None` where the plane
    /// misses it. See the module.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] if the window is empty, or a shape can't be made (a
    /// boolean whose result is empty is `None`).
    pub fn section(&self, z: f64, window: Bounds) -> Result<Option<Shape>> {
        let b = self.bounds();
        if z < b.min[2] || z > b.max[2] {
            return Ok(None);
        }
        let um = |v: f64| Length::um(v);
        match self.kind() {
            Kind::Cuboid { min, max } => Ok(Some(Shape::rect(
                Point::um(0.5 * (min[0] + max[0]), 0.5 * (min[1] + max[1])),
                um(max[0] - min[0]),
                um(max[1] - min[1]),
            )?)),
            Kind::Ellipsoid {
                center,
                inverse,
                axes,
            } => match ellipsoid_section(*center, *inverse, axes, z) {
                Some(None) => Ok(None),
                Some(Some((c, semi, angle))) => Ok(Some(Shape::ellipse(
                    Point::um(c[0], c[1]),
                    [um(semi[0]), um(semi[1])],
                    angle,
                )?)),
                None => self.masked(z, window),
            },
            Kind::Frustum(f) if f.axis[2].abs() == 1.0 => {
                let t = (z - f.base[2]) * f.axis[2];
                let r = f.radii[0] + (f.radii[1] - f.radii[0]) * t / f.length;
                if r.is_nan() || r <= 0.0 || t < 0.0 || t > f.length {
                    return Ok(None);
                }
                Ok(Some(Shape::circle(Point::um(f.base[0], f.base[1]), um(r))?))
            }
            Kind::Extruded { shape, .. } => Ok(Some(shape.clone())),
            Kind::Etched {
                shape, z: h, run, ..
            } => {
                let reach = (h[1] - z) * run;
                if reach <= 0.0 {
                    return Ok(Some(shape.clone()));
                }
                Ok(Some(Shape::Offset(Offset::new(
                    shape.clone(),
                    um(reach),
                    Join::Round,
                )?)))
            }
            Kind::HalfSpace { point, normal } => {
                // n_x x + n_y y < n·point − n_z z
                let c = normal[0] * point[0] + normal[1] * point[1] + normal[2] * (point[2] - z);
                if normal[0] == 0.0 && normal[1] == 0.0 {
                    return (c > 0.0).then(|| rect_of(window)).transpose();
                }
                let corners = [
                    [window.min.x.to_um(), window.min.y.to_um()],
                    [window.max.x.to_um(), window.min.y.to_um()],
                    [window.max.x.to_um(), window.max.y.to_um()],
                    [window.min.x.to_um(), window.max.y.to_um()],
                ];
                let kept = half_plane(&corners, [normal[0], normal[1]], c);
                if kept.len() < 3 {
                    return Ok(None);
                }
                Ok(
                    Polygon::new(kept.iter().map(|p| Point::um(p[0], p[1])).collect())
                        .ok()
                        .map(Shape::Polygon),
                )
            }
            Kind::Boolean { op, operands } => {
                let (a, b) = (
                    operands[0].section(z, window)?,
                    operands[1].section(z, window)?,
                );
                Ok(match (a, b) {
                    (None, None) => None,
                    (Some(a), None) => (*op != Operation::Intersection).then_some(a),
                    (None, Some(b)) => matches!(op, Operation::Union | Operation::Xor).then_some(b),
                    (Some(a), Some(b)) => Boolean::new(*op, a, b).ok().map(Shape::Boolean),
                })
            }
            Kind::Frustum(_) => self.masked(z, window),
        }
    }

    /// The section as a mask over the window and the body's bounds: each pixel 1 where the body
    /// holds its centre, 0 elsewhere.
    fn masked(&self, z: f64, window: Bounds) -> Result<Option<Shape>> {
        let b = self.bounds();
        let area = Bounds3d {
            min: [window.min.x.to_um(), window.min.y.to_um(), z],
            max: [window.max.x.to_um(), window.max.y.to_um(), z],
        };
        let lo = [area.min[0].max(b.min[0]), area.min[1].max(b.min[1])];
        let hi = [area.max[0].min(b.max[0]), area.max[1].min(b.max[1])];
        if !(hi[0] > lo[0] && hi[1] > lo[1]) {
            return Ok(None);
        }
        let side = (hi[0] - lo[0]).max(hi[1] - lo[1]) / PIXELS as f64;
        let nx = ((hi[0] - lo[0]) / side).ceil().max(1.0) as usize;
        let ny = ((hi[1] - lo[1]) / side).ceil().max(1.0) as usize;
        let mut values = Vec::with_capacity(nx * ny);
        for j in 0..ny {
            for i in 0..nx {
                let p = [
                    lo[0] + (i as f64 + 0.5) * side,
                    lo[1] + (j as f64 + 0.5) * side,
                    z,
                ];
                values.push(if self.contains(p) { 1.0 } else { 0.0 });
            }
        }
        if !values.iter().any(|v| *v > 0.5) {
            return Ok(None);
        }
        let mask = Mask::new(
            Point::um(lo[0], lo[1]),
            [Length::um(side), Length::um(side)],
            nx,
            values,
        )?;
        Ok(Some(Shape::Mask(mask)))
    }
}

/// The window as a rectangle.
fn rect_of(window: Bounds) -> Result<Shape> {
    Shape::rect(
        Point::um(
            0.5 * (window.min.x.to_um() + window.max.x.to_um()),
            0.5 * (window.min.y.to_um() + window.max.y.to_um()),
        ),
        window.max.x - window.min.x,
        window.max.y - window.min.y,
    )
}

/// The polygon `ring` clipped to n·p < c.
fn half_plane(ring: &[[f64; 2]], n: [f64; 2], c: f64) -> Vec<[f64; 2]> {
    let f = |p: [f64; 2]| n[0] * p[0] + n[1] * p[1] - c;
    let mut out = Vec::with_capacity(ring.len() + 1);
    for k in 0..ring.len() {
        let (p, q) = (ring[k], ring[(k + 1) % ring.len()]);
        let (fp, fq) = (f(p), f(q));
        if fp <= 0.0 {
            out.push(p);
        }
        if (fp < 0.0 && fq > 0.0) || (fp > 0.0 && fq < 0.0) {
            let t = fp / (fp - fq);
            out.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
        }
    }
    out
}
