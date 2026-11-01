//! The films a flow's frames draw over the wafer's topography, which the planar model can't hold:
//! a resist spun over etched layers, a film deposited over an etched step, a film evaporated onto
//! a stencil. Each is drawn in pieces ([`FrameFilm`]), each a solid between two outlines.
//!
//! - **A resist** is spun on and planarizes: it fills the regions etched open and its top is flat,
//!   its thickness over the highest top. That is the frames' model of a spin-on film, not a
//!   source's.
//! - **A deposited film** is conformal: it follows the surface, its thickness `t` on the flat
//!   (tops and floors) and `n` on the walls, measured normal to them; `n / t` is the process's
//!   step coverage. Its outer surface is the surface below offset by those thicknesses, corners
//!   mitred (sharp, not rounded). Over vertical walls the pieces are exact prisms from the
//!   geometry kernel's booleans and offsets, merging where films from two ridges meet; over sloped
//!   walls they are lofts between the ridges' outlines mitred out, drawn over the ridges (which a
//!   picture draws on top of them), one set per ridge.
//!
//! The pieces over sloped walls overlap the ridges they cover, as a film around the shapes does in
//! the 3D view; every other piece is exact.

use serde::Serialize;

use crate::fabrication::Made;
use crate::geometry::{Join, Operation, Point, Polygons, Shape, Tolerance};
use crate::units::Length;

/// A piece of a film a frame draws over its scene: the solid between an outline at its bottom
/// and an outline at its top, with the same vertices in turn (the top's outline the bottom's
/// moved in or out, each edge parallel to itself, or the same outline: a prism).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FrameFilm {
    /// The layer it belongs to: `"resist"`, or the layer a deposition makes.
    pub layer: String,
    /// The film's name, e.g. `"photoresist"`, `"photoresist, exposed"`, `"SiO2"`.
    pub film: String,
    /// The real part of the permittivity it is drawn with.
    pub eps: f64,
    /// Its bottom and top, µm.
    pub z_um: [f64; 2],
    /// Its outline at the bottom, as [`super::SceneShape::outline`]: the outer boundary
    /// counterclockwise, back to its first vertex, then each hole clockwise, back to its own.
    pub bottom: Vec<[f64; 2]>,
    /// Its outline at the top, the same vertices in turn.
    pub top: Vec<[f64; 2]>,
}

type Ring = Vec<[f64; 2]>;

/// A polygon with its holes, µm: the outer ring counterclockwise, each hole clockwise.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Contour {
    pub outer: Ring,
    pub holes: Vec<Ring>,
}

/// What a piece is painted as: its layer, film and permittivity.
#[derive(Clone, Debug)]
pub(super) struct Paint {
    pub layer: String,
    pub film: String,
    pub eps: f64,
}

/// A region of the plan, as the geometry kernel's polygons (on its 1 nm grid); `None` is empty.
pub(super) type Area = Option<Polygons>;

/// Polygons of `contours`, as drawn.
fn polygons_of(contours: Vec<crate::geometry::Contour>) -> Area {
    (!contours.is_empty()).then(|| Polygons {
        contours,
        tolerance: Tolerance::default(),
        deviation: Length::ZERO,
    })
}

fn points(ring: &[[f64; 2]]) -> Vec<Point> {
    ring.iter().map(|p| Point::um(p[0], p[1])).collect()
}

fn signed_area(r: &[[f64; 2]]) -> f64 {
    0.5 * (0..r.len())
        .map(|i| {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
}

/// A scene outline's rings (see [`FrameFilm::bottom`]): its boundary, then its holes.
fn rings(outline: &[[f64; 2]]) -> Vec<Ring> {
    let mut out = Vec::new();
    let mut rest = outline;
    while !rest.is_empty() {
        let first = rest[0];
        let end = rest
            .iter()
            .skip(1)
            .position(|p| *p == first)
            .map_or(rest.len(), |k| k + 1);
        if end >= 3 {
            out.push(rest[..end].to_vec());
        }
        rest = if end < rest.len() {
            &rest[end + 1..]
        } else {
            &[]
        };
    }
    out
}

/// The rectangle `x` × `y`, µm.
pub(super) fn rect(x: [f64; 2], y: [f64; 2]) -> Area {
    polygons_of(vec![crate::geometry::Contour {
        outer: points(&[[x[0], y[0]], [x[1], y[0]], [x[1], y[1]], [x[0], y[1]]]),
        holes: Vec::new(),
    }])
}

/// The union of `shapes`, from the outlines a scene draws them by (so a film around them has their
/// vertices, not a finer flattening of their curves).
pub(super) fn union(shapes: &[Shape]) -> Area {
    let mut u: Area = None;
    for outline in shapes.iter().flat_map(super::scene::outlines) {
        let mut r = rings(&outline).into_iter();
        let Some(mut outer) = r.next() else {
            continue;
        };
        if signed_area(&outer) < 0.0 {
            outer.reverse();
        }
        let holes = r
            .map(|mut h| {
                if signed_area(&h) > 0.0 {
                    h.reverse();
                }
                points(&h)
            })
            .collect();
        let one = polygons_of(vec![crate::geometry::Contour {
            outer: points(&outer),
            holes,
        }]);
        u = match (u, one) {
            (None, b) => b,
            (a, None) => a,
            (Some(a), Some(b)) => polygons_of(
                a.boolean(&b, Operation::Union, Tolerance::default())
                    .contours,
            ),
        };
    }
    u
}

/// `a` less `b`.
pub(super) fn minus(a: &Area, b: &Area) -> Area {
    match (a, b) {
        (None, _) => None,
        (Some(a), None) => Some(a.clone()),
        (Some(a), Some(b)) => polygons_of(
            a.boolean(b, Operation::Difference, Tolerance::default())
                .contours,
        ),
    }
}

/// `a` ∩ `b`.
pub(super) fn and(a: &Area, b: &Area) -> Area {
    match (a, b) {
        (Some(a), Some(b)) => polygons_of(
            a.boolean(b, Operation::Intersection, Tolerance::default())
                .contours,
        ),
        _ => None,
    }
}

/// `a` grown by `d` µm (shrunk where `d` is negative), its corners mitred.
fn grown(a: &Area, d: f64) -> Area {
    if d == 0.0 {
        return a.clone();
    }
    a.as_ref()?
        .offset(
            Length::um(d),
            Join::Miter { limit: 4.0 },
            Tolerance::default(),
        )
        .ok()
        .and_then(|p| polygons_of(p.contours))
}

/// The polygons of `a`, each with its holes.
pub(super) fn contours(a: &Area) -> Vec<Contour> {
    let Some(p) = a else {
        return Vec::new();
    };
    let xy = |p: &Point| [p.x.to_um(), p.y.to_um()];
    p.contours
        .iter()
        .filter(|c| c.outer.len() >= 3 && c.area() > 0.0)
        .map(|c| Contour {
            outer: c.outer.iter().map(xy).collect(),
            holes: c
                .holes
                .iter()
                .filter(|h| h.len() >= 3)
                .map(|h| h.iter().map(xy).collect())
                .collect(),
        })
        .collect()
}

/// A contour as a scene outline (see [`FrameFilm::bottom`]).
fn walk(c: &Contour) -> Vec<[f64; 2]> {
    let mut o = c.outer.clone();
    if !c.holes.is_empty() {
        o.push(c.outer[0]);
    }
    for h in &c.holes {
        o.extend(h);
        o.push(h[0]);
    }
    o
}

/// The prisms of `area` from `z0` to `z1`, one a polygon.
pub(super) fn prism(p: &Paint, area: &Area, z0: f64, z1: f64) -> Vec<FrameFilm> {
    if z1 - z0 <= 1e-12 {
        return Vec::new();
    }
    contours(area)
        .iter()
        .map(|c| loft(p, c, z0, c, z1))
        .collect()
}

/// The solid from `bottom` at `z0` to `top` at `z1`, of the same vertices in turn.
fn loft(p: &Paint, bottom: &Contour, z0: f64, top: &Contour, z1: f64) -> FrameFilm {
    FrameFilm {
        layer: p.layer.clone(),
        film: p.film.clone(),
        eps: p.eps,
        z_um: [z0, z1],
        bottom: walk(bottom),
        top: walk(top),
    }
}

/// `ring` moved `d` to the right of its walk (out of the material for an outer ring walked
/// counterclockwise and for a hole walked clockwise), each corner along its mitre, at most 4 d
/// from where it was: the studio's own mitre (studio/src-tauri's fabrication::mesh), so a loft's
/// walls are planes.
pub(super) fn mitre(ring: &[[f64; 2]], d: f64) -> Ring {
    if d == 0.0 {
        return ring.to_vec();
    }
    let n = ring.len();
    let normal = |p: [f64; 2], q: [f64; 2]| {
        let (ex, ey) = (q[0] - p[0], q[1] - p[1]);
        let l = ex.hypot(ey).max(f64::MIN_POSITIVE);
        [ey / l, -ex / l]
    };
    (0..n)
        .map(|i| {
            let (p, v, q) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
            let (a, b) = (normal(p, v), normal(v, q));
            let k = (1.0 + a[0] * b[0] + a[1] * b[1]).max(0.125);
            [v[0] + d * (a[0] + b[0]) / k, v[1] + d * (a[1] + b[1]) / k]
        })
        .collect()
}

/// Whether `ring` mitred by `d` keeps every edge running the way it ran: a hole shrunk that
/// hasn't closed up.
fn holds(ring: &[[f64; 2]], d: f64) -> bool {
    let m = mitre(ring, d);
    let n = ring.len();
    (0..n).all(|i| {
        let j = (i + 1) % n;
        let (a, b) = (
            [ring[j][0] - ring[i][0], ring[j][1] - ring[i][1]],
            [m[j][0] - m[i][0], m[j][1] - m[i][1]],
        );
        a[0] * b[0] + a[1] * b[1] > 0.0
    })
}

/// The largest mitre `ring` (a hole) holds, up to `most`.
fn holds_up_to(ring: &[[f64; 2]], most: f64) -> f64 {
    if holds(ring, most) {
        return most;
    }
    let (mut lo, mut hi) = (0.0, most);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if holds(ring, mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// A layer of a stage with air in it: etched and not yet filled.
#[derive(Clone, Debug)]
pub(super) struct Open {
    /// Its name.
    pub name: String,
    /// Its bottom and top, µm.
    pub z: [f64; 2],
    /// Where the air in it starts: the shallowest etch's floor (a ridge's), or the layer's bottom
    /// (a trench's).
    pub floor: f64,
    /// Its shapes.
    pub shapes: Vec<Shape>,
    /// Whether its shapes are ridges with air around them (else trenches in its film).
    pub ridge: bool,
    /// The walls' angle from the plane, radians, when every shape is etched alike; `None` when
    /// they are etched to different depths or walls.
    pub sidewall: Option<f64>,
}

impl Open {
    fn vertical(&self) -> bool {
        self.sidewall
            .is_some_and(|s| s >= std::f64::consts::FRAC_PI_2 - 1e-12)
    }
}

/// The stage's top, µm: its layers' (a bare wafer's stand-in air layer left out).
pub(super) fn top_of(made: &Made) -> f64 {
    let s = &made.structure;
    s.stack()
        .layers()
        .iter()
        .zip(&made.layers)
        .filter(|(l, names)| !stand_in(l, names, s))
        .map(|(l, _)| l.thickness.to_um())
        .sum()
}

/// Whether a layer is a bare wafer's stand-in: air, drawn over nothing.
pub(super) fn stand_in(
    l: &crate::stack::Layer,
    names: &(String, String),
    s: &crate::stack::Structure,
) -> bool {
    l.name == "air" && names.0 == "air" && names.1 == "air" && s.shapes("air").is_empty()
}

/// The layers of `made` with air in them, bottom to top.
pub(super) fn open_layers(made: &Made) -> Vec<Open> {
    let s = &made.structure;
    let mut out = Vec::new();
    let mut bottom = 0.0;
    for (l, (drawn, background)) in s.stack().layers().iter().zip(&made.layers) {
        let t = l.thickness.to_um();
        let shapes = s.shapes(&l.name);
        let ridge = background == "air" && drawn != "air";
        let trench = drawn == "air" && background != "air";
        if !shapes.is_empty() && (ridge || trench) {
            let etches = s.etches(&l.name);
            let depth = |e: &crate::stack::Etch| e.depth.map_or(t, Length::to_um).min(t);
            let shallowest = etches.iter().map(depth).fold(t, f64::min);
            let alike = etches.windows(2).all(|w| w[0] == w[1]);
            out.push(Open {
                name: l.name.clone(),
                z: [bottom, bottom + t],
                floor: if ridge {
                    bottom + t - shallowest
                } else {
                    bottom
                },
                shapes: shapes.to_vec(),
                ridge,
                sidewall: alike.then(|| {
                    etches
                        .first()
                        .map_or(std::f64::consts::FRAC_PI_2, |e| e.sidewall)
                }),
            });
        }
        bottom += t;
    }
    out
}

/// A film filling the open layers of a stage up to its top, over `area` of the plan: in each
/// open layer from its floor to its top, around its ridges or in its trenches. Around sloped (or
/// unlike) ridges it is drawn over them, a prism of `area`, which the ridges are drawn over.
pub(super) fn fill(p: &Paint, open: &[Open], area: &Area) -> Vec<FrameFilm> {
    let mut out = Vec::new();
    for o in open {
        let region = match (o.ridge, o.vertical()) {
            (true, true) => minus(area, &union(&o.shapes)),
            (true, false) => area.clone(),
            (false, _) => and(area, &union(&o.shapes)),
        };
        out.extend(prism(p, &region, o.floor, o.z[1]));
    }
    out
}

/// How far a film `t` thick on the flat and `n` on the walls, conformal over `o`, reaches beyond
/// its shapes, µm.
pub(super) fn reach(o: &Open, t: f64, n: f64) -> f64 {
    match o.sidewall {
        Some(s) if o.ridge && s < std::f64::consts::FRAC_PI_2 - 1e-12 => {
            let a = |z: f64| (o.z[1] - z) / s.tan() + n / s.sin();
            a(o.floor + t).max(0.0)
        }
        _ if o.ridge => n,
        _ => 0.0,
    }
}

/// A film `t` thick on the flat and `n` on the walls, conformal over the open layer `o` of a
/// stage, over the window `window`: its pieces, or `None` for sloped trenches or shapes etched
/// unlike, which the frames don't draw conformal.
pub(super) fn conformal(
    p: &Paint,
    o: &Open,
    window: &Area,
    t: f64,
    n: f64,
) -> Option<Vec<FrameFilm>> {
    let sidewall = o.sidewall?;
    let (zf, zt) = (o.floor, o.z[1]);
    let h = zt - zf;
    let w: Area = window.clone();
    let shapes = union(&o.shapes);
    let mut out = Vec::new();
    if o.ridge && o.vertical() {
        let grown = grown(&shapes, n);
        if t >= h {
            out.extend(prism(p, &minus(&w, &shapes), zf, zt));
            out.extend(prism(p, &w, zt, zf + t));
            out.extend(prism(p, &grown, zf + t, zt + t));
        } else {
            out.extend(prism(p, &minus(&w, &shapes), zf, zf + t));
            out.extend(prism(p, &minus(&grown, &shapes), zf + t, zt));
            out.extend(prism(p, &grown, zt, zt + t));
        }
        return Some(out);
    }
    if !o.ridge {
        if !o.vertical() {
            return None;
        }
        let narrowed = grown(&shapes, -n);
        if t >= h {
            out.extend(prism(p, &shapes, zf, zt));
            out.extend(prism(p, &w, zt, zf + t));
            out.extend(prism(p, &minus(&w, &narrowed), zf + t, zt + t));
        } else {
            out.extend(prism(p, &shapes, zf, zf + t));
            out.extend(prism(p, &minus(&shapes, &narrowed), zf + t, zt));
            out.extend(prism(p, &minus(&w, &narrowed), zt, zt + t));
        }
        return Some(out);
    }
    // sloped ridges: the flat film over the window, then each ridge's envelope above it, its
    // outline mitred out by a(z) = (z_t − z) / tan θ + n / sin θ (the wall's own run, and the
    // film's normal thickness n across it), and by no less than nothing over the top
    out.extend(prism(p, &w, zf, zf + t));
    let a = |z: f64| (zt - z) / sidewall.tan() + n / sidewall.sin();
    let (z0, z1) = (zf + t, zt + t);
    // where a reaches 0: above it the film over the top is the ridge's own outline
    let zk = zt + n / sidewall.cos();
    let mut segments: Vec<(f64, f64, f64, f64)> = Vec::new();
    if zk <= z0 {
        segments.push((z0, z1, 0.0, 0.0));
    } else if zk < z1 {
        segments.push((z0, zk, a(z0), 0.0));
        segments.push((zk, z1, 0.0, 0.0));
    } else {
        segments.push((z0, z1, a(z0), a(z1)));
    }
    for c in contours(&shapes) {
        // the largest mitre each hole holds before it closes (filled with the film below that)
        let most = segments.iter().map(|s| s.2).fold(0.0, f64::max);
        let held: Vec<f64> = c.holes.iter().map(|hole| holds_up_to(hole, most)).collect();
        for &(za, zb, ea, eb) in &segments {
            // split where a hole opens: e falls from ea at za to eb at zb
            let at = |e: f64| {
                if (ea - eb).abs() < 1e-15 {
                    za
                } else {
                    za + (ea - e) / (ea - eb) * (zb - za)
                }
            };
            let mut cuts: Vec<f64> = held
                .iter()
                .filter(|&&m| m < ea && m > eb)
                .map(|&m| at(m))
                .collect();
            cuts.push(za);
            cuts.push(zb);
            cuts.sort_by(f64::total_cmp);
            cuts.dedup_by(|x, y| (*x - *y).abs() < 1e-12);
            for pair in cuts.windows(2) {
                let (lo, hi) = (pair[0], pair[1]);
                if hi - lo <= 1e-12 {
                    continue;
                }
                let e = |z: f64| {
                    if (zb - za).abs() < 1e-15 {
                        ea
                    } else {
                        ea + (eb - ea) * (z - za) / (zb - za)
                    }
                };
                let (elo, ehi) = (e(lo), e(hi));
                let kept: Vec<&Ring> = c
                    .holes
                    .iter()
                    .zip(&held)
                    .filter(|(_, m)| **m >= elo - 1e-12)
                    .map(|(hole, _)| hole)
                    .collect();
                let at_e = |d: f64| Contour {
                    outer: mitre(&c.outer, d),
                    holes: kept.iter().map(|hole| mitre(hole, d)).collect(),
                };
                out.push(loft(p, &at_e(elo), lo, &at_e(ehi), hi));
            }
        }
    }
    Some(out)
}
