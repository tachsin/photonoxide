//! The studio's 3D preview: a job's scene and its ports.

use serde::{Deserialize, Serialize};

use super::check::{check, task_error};
use super::event::Event;
use super::{bodies, components, device, params};
use crate::Result;
use crate::geometry::{Point, Shape};
use crate::run::{Job, Run};
use crate::stack::Structure;
use crate::units::{Length, Wavelength};

/// A port of a [`Event::Ports`]: where a component's guide ends, by its name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ScenePort {
    /// `component.port`, e.g. `"ring.through"`, as an FDFD or FDTD job finds it.
    pub name: String,
    /// The layer its guide is drawn on.
    pub layer: String,
    /// The centre of the guide's end, µm.
    pub position_um: [f64; 2],
    /// The direction light leaves by, degrees counterclockwise from +x.
    pub direction_deg: f64,
    /// The guide's width there, µm.
    pub width_um: f64,
}

/// The named ports of `job`'s components, as [`Event::Ports`] records them: what the studio
/// marks on the scene [`preview`](crate::job::preview) draws. A job's `[device]` gives its device's ports; a task
/// without components, none.
///
/// # Errors
///
/// The errors of [`check`].
pub fn preview_ports(job: &Job) -> Result<Vec<ScenePort>> {
    if job.device().is_some() {
        return device::scene_ports(job);
    }
    check(job)?;
    component_ports(job.task())
}

/// The named ports of the components in `task`.
fn component_ports(task: &toml::Table) -> Result<Vec<ScenePort>> {
    let resolved = params::resolve(task, None)?;
    let Some(toml::Value::Array(entries)) = resolved.get("component") else {
        return Ok(Vec::new());
    };
    let tables: Vec<toml::Table> = entries
        .iter()
        .filter_map(|v| v.as_table().cloned())
        .collect();
    Ok(components::ports(&tables)?
        .into_iter()
        .map(|(name, p)| ScenePort {
            name,
            layer: p.layer.clone(),
            position_um: [p.position.x.to_um(), p.position.y.to_um()],
            direction_deg: p.direction,
            width_um: p.width.to_um(),
        })
        .collect())
}

/// Records the job's components' ports after its scene, if it has any.
pub(super) fn record_ports(job: &Job, run: &mut Run) -> Result<()> {
    let ports = if job.device().is_some() {
        device::scene_ports(job)?
    } else {
        component_ports(job.task())?
    };
    if !ports.is_empty() {
        run.record(&Event::Ports { ports })?;
    }
    Ok(())
}

/// A medium of a [`Event::Scene`]: its material and the real part of its permittivity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneMedium {
    /// The material's name, e.g. `"SiO2"`.
    pub material: String,
    /// Re ε at the scene's wavelength.
    pub eps: f64,
}

/// A layer of a [`Event::Scene`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneLayer {
    /// Its name, e.g. `"Si"`.
    pub name: String,
    /// Its bottom and top, µm.
    pub z_um: [f64; 2],
    /// What shapes on it are made of.
    pub material: SceneMedium,
    /// What fills it elsewhere.
    pub background: SceneMedium,
}

/// A shape of a [`Event::Scene`]: its layer and its outline, counterclockwise, µm.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneShape {
    /// The layer it is drawn on.
    pub layer: String,
    /// Its outline's vertices, (x, y), counterclockwise; a circle as a 64-gon. A shape with a
    /// hole (a ring) is one outline that also walks the hole: the outer boundary
    /// counterclockwise and back to its first vertex, then the hole clockwise from its first
    /// vertex and back to it, the two joined by a cut of zero width. Its first vertex
    /// recurring marks where the hole starts.
    pub outline: Vec<[f64; 2]>,
}

/// A shape's outline, counterclockwise: a rectangle's corners, a circle as a 64-gon, a ring as
/// its two 64-gons joined by a cut (see [`SceneShape::outline`]).
pub(super) fn outline(shape: &Shape) -> Vec<[f64; 2]> {
    match shape {
        Shape::Rect {
            center,
            width,
            height,
        } => {
            let (cx, cy, w, h) = (
                center.x.to_um(),
                center.y.to_um(),
                width.to_um() / 2.0,
                height.to_um() / 2.0,
            );
            vec![
                [cx - w, cy - h],
                [cx + w, cy - h],
                [cx + w, cy + h],
                [cx - w, cy + h],
            ]
        }
        Shape::Circle { center, radius } => polygon64(*center, radius.to_um(), 1.0),
        Shape::Ring {
            center,
            inner,
            outer,
        } => {
            let mut o = polygon64(*center, outer.to_um(), 1.0);
            o.push(o[0]);
            let hole = polygon64(*center, inner.to_um(), -1.0);
            o.extend(&hole);
            o.push(hole[0]);
            o
        }
        Shape::Polygon(p) => p
            .vertices()
            .iter()
            .map(|v| [v.x.to_um(), v.y.to_um()])
            .collect(),
        // the other primitives (not yet in jobs): their polygons within 1 nm, holes joined as
        // a ring's are
        other => {
            use crate::geometry::Region;
            let xy = |v: &Point| [v.x.to_um(), v.y.to_um()];
            let mut o = Vec::new();
            for c in other
                .polygons(crate::geometry::Tolerance::default())
                .contours
            {
                o.extend(c.outer.iter().map(xy));
                if let Some(first) = c.outer.first()
                    && !c.holes.is_empty()
                {
                    o.push(xy(first));
                }
                for hole in &c.holes {
                    o.extend(hole.iter().map(xy));
                    if let Some(first) = hole.first() {
                        o.push(xy(first));
                    }
                }
            }
            o
        }
    }
}

/// A circle about `center` as a 64-gon from angle 0, counterclockwise for `turn` 1, clockwise for
/// −1.
fn polygon64(center: Point, radius: f64, turn: f64) -> Vec<[f64; 2]> {
    (0..64)
        .map(|k| {
            let a = turn * std::f64::consts::TAU * f64::from(k) / 64.0;
            [
                center.x.to_um() + radius * a.cos(),
                center.y.to_um() + radius * a.sin(),
            ]
        })
        .collect()
}

/// Every shape of `s`, layer by layer from the bottom, as a scene lists them.
pub(super) fn scene_shapes(s: &Structure) -> Vec<SceneShape> {
    s.stack()
        .layers()
        .iter()
        .flat_map(|layer| {
            s.shapes(&layer.name)
                .iter()
                .flat_map(outlines)
                .map(|outline| SceneShape {
                    layer: layer.name.clone(),
                    outline,
                })
        })
        .collect()
}

/// A shape of an etched layer (a partial etch, sloped walls) as drawn: a scene draws such a
/// layer in bands ([`Event::Scene`]'s `name 2`, `name 3`, … below it), its walls a staircase; a
/// cross-section that wants them exact draws each shape from its top face down to its etch's
/// floor, its walls leaning out by its sidewall.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SceneEtched {
    /// The layer it is drawn on, as the stack names it.
    pub layer: String,
    /// The layer's bottom and top, µm.
    pub z_um: [f64; 2],
    /// Its top face's outline, as [`SceneShape::outline`].
    pub outline: Vec<[f64; 2]>,
    /// How deep the etch went from the layer's top, µm: the layer's thickness for a full etch.
    pub depth_um: f64,
    /// The walls' angle from the layer's plane, degrees: 90 for vertical walls.
    pub sidewall_deg: f64,
}

/// Every shape of `s`'s etched layers (those a scene draws in bands), each with its etch: what a
/// cross-section draws in place of the bands' staircase. Layers etched through with vertical
/// walls have none here: the scene's shapes are already exact.
pub fn scene_etched(s: &Structure) -> Vec<SceneEtched> {
    let mut out = Vec::new();
    let mut bottom = 0.0;
    for layer in s.stack().layers() {
        let t = layer.thickness.to_um();
        if !s.is_plain(&layer.name) {
            for (shape, e) in s.shapes(&layer.name).iter().zip(s.etches(&layer.name)) {
                for outline in outlines(shape) {
                    out.push(SceneEtched {
                        layer: layer.name.clone(),
                        z_um: [bottom, bottom + t],
                        outline,
                        depth_um: e.depth.map_or(t, Length::to_um),
                        sidewall_deg: e.sidewall.to_degrees(),
                    });
                }
            }
        }
        bottom += t;
    }
    out
}
/// An array's or a reference's elements, when no two of them meet: each drawn as its own
/// outline (circles as 64-gons, as everywhere in a scene); `None` for any other shape, or for
/// elements that touch or overlap, which are drawn as their union's polygons.
fn separate(shape: &Shape) -> Option<&[crate::geometry::Element]> {
    let a = match shape {
        Shape::Array(a) => a,
        Shape::Reference(r) => r.array(),
        _ => return None,
    };
    a.is_separate().then(|| a.elements())
}

/// A shape's outlines: one ([`outline`]); an array's or a reference's elements' each, when
/// they don't meet; or for a boolean, offset, rounding (which may fall apart into several
/// polygons), and an array whose elements meet, one for each of its polygons within 1 nm,
/// holes joined as a ring's are.
fn outlines(shape: &Shape) -> Vec<Vec<[f64; 2]>> {
    if let Some(elements) = separate(shape) {
        return elements.iter().flat_map(|e| outlines(&e.shape)).collect();
    }
    if !matches!(
        shape,
        Shape::Boolean(_)
            | Shape::Offset(_)
            | Shape::Rounded(_)
            | Shape::Array(_)
            | Shape::Reference(_)
    ) {
        return vec![outline(shape)];
    }
    use crate::geometry::Region;
    shape
        .polygons(crate::geometry::Tolerance::default())
        .contours
        .iter()
        .map(|c| joined(c, 0.0))
        .collect()
}

/// A polygon with holes as one outline (see [`SceneShape::outline`]), each ring moved `d` µm out
/// of the material, its corners mitred.
fn joined(c: &crate::geometry::Contour, d: f64) -> Vec<[f64; 2]> {
    let xy = |v: &Point| [v.x.to_um(), v.y.to_um()];
    let ring = |r: &[Point]| {
        let v: Vec<[f64; 2]> = r.iter().map(xy).collect();
        if d == 0.0 { v } else { mitred(&v, d) }
    };
    let mut o = ring(&c.outer);
    if !c.holes.is_empty()
        && let Some(first) = o.first().copied()
    {
        o.push(first);
    }
    for hole in &c.holes {
        let h = ring(hole);
        o.extend(&h);
        if let Some(first) = h.first() {
            o.push(*first);
        }
    }
    o
}

/// A band of an etched layer as a scene draws it: its name, its bottom and top (µm), whether a
/// partial etch's slab fills it, and its shapes' outlines at its middle.
struct Band {
    name: String,
    z: [f64; 2],
    slab: bool,
    outlines: Vec<Vec<[f64; 2]>>,
}

/// An etched layer (a partial etch, sloped walls) as a scene draws it: bands between its top,
/// each partial etch's floor and its bottom, a sloped etch's in four, each with its shapes'
/// outlines grown by their walls' reach at its middle. The top band keeps the layer's name; the
/// ones below are `name 2`, `name 3`, … down the layer. A band under a partial etch's floor is
/// the layer's material all through, its slab.
fn bands(s: &Structure, name: &str, bottom: f64, top: f64) -> Vec<Band> {
    let t = top - bottom;
    let shapes = s.shapes(name);
    let etches = s.etches(name);
    let floor = |e: &crate::stack::Etch| e.depth.map_or(t, Length::to_um);
    // depths below the top where a band starts or ends
    let mut depths = vec![0.0, t];
    for e in etches {
        depths.push(floor(e));
        if e.run() > 0.0 {
            depths.extend((1..4).map(|k| floor(e) * f64::from(k) / 4.0));
        }
    }
    depths.sort_by(f64::total_cmp);
    depths.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    let shallowest = etches
        .iter()
        .filter_map(|e| e.depth.map(Length::to_um))
        .fold(f64::INFINITY, f64::min);
    depths
        .windows(2)
        .enumerate()
        .map(|(k, d)| {
            let middle = 0.5 * (d[0] + d[1]);
            let slab = middle > shallowest;
            let outlines = if slab {
                Vec::new()
            } else {
                shapes
                    .iter()
                    .zip(etches)
                    .filter(|(_, e)| middle <= floor(e))
                    .flat_map(|(shape, e)| grown_all(shape, middle * e.run()))
                    .collect()
            };
            Band {
                name: if k == 0 {
                    name.to_owned()
                } else {
                    format!("{name} {}", k + 1)
                },
                z: [top - d[1], top - d[0]],
                slab,
                outlines,
            }
        })
        .collect()
}

/// A shape's outlines ([`outlines`]) grown by `d` µm, as [`grown`].
fn grown_all(shape: &Shape, d: f64) -> Vec<Vec<[f64; 2]>> {
    if let Some(elements) = separate(shape) {
        return elements
            .iter()
            .flat_map(|e| grown_all(&e.shape, d))
            .collect();
    }
    if !matches!(
        shape,
        Shape::Boolean(_)
            | Shape::Offset(_)
            | Shape::Rounded(_)
            | Shape::Array(_)
            | Shape::Reference(_)
    ) {
        return vec![grown(shape, d)];
    }
    use crate::geometry::Region;
    shape
        .polygons(crate::geometry::Tolerance::default())
        .contours
        .iter()
        .map(|c| joined(c, d))
        .collect()
}

/// A shape's outline ([`outline`]) grown by `d` µm, its corners mitred: what a sloped wall
/// reaches out to at a depth (the true section's convex corners are round; this is a picture).
fn grown(shape: &Shape, d: f64) -> Vec<[f64; 2]> {
    if d == 0.0 {
        return outline(shape);
    }
    match shape {
        Shape::Rect {
            center,
            width,
            height,
        } => outline(
            &Shape::rect(
                *center,
                *width + Length::um(2.0 * d),
                *height + Length::um(2.0 * d),
            )
            .unwrap_or_else(|_| shape.clone()),
        ),
        Shape::Circle { center, radius } => polygon64(*center, radius.to_um() + d, 1.0),
        Shape::Ring {
            center,
            inner,
            outer,
        } => {
            let mut o = polygon64(*center, outer.to_um() + d, 1.0);
            let hole = inner.to_um() - d;
            if hole > 0.0 {
                o.push(o[0]);
                let hole = polygon64(*center, hole, -1.0);
                o.extend(&hole);
                o.push(hole[0]);
            }
            o
        }
        other => {
            use crate::geometry::Region;
            let xy = |v: &Point| [v.x.to_um(), v.y.to_um()];
            let mut o = Vec::new();
            for c in other
                .polygons(crate::geometry::Tolerance::default())
                .contours
            {
                let outer = mitred(&c.outer.iter().map(xy).collect::<Vec<_>>(), d);
                o.extend(&outer);
                if !c.holes.is_empty()
                    && let Some(first) = outer.first()
                {
                    o.push(*first);
                }
                for hole in &c.holes {
                    let hole = mitred(&hole.iter().map(xy).collect::<Vec<_>>(), d);
                    o.extend(&hole);
                    if let Some(first) = hole.first() {
                        o.push(*first);
                    }
                }
            }
            o
        }
    }
}

/// A ring of vertices (material on its left: an outline counterclockwise, a hole clockwise)
/// moved `d` to its right, each vertex along its corner's bisector, mitred.
fn mitred(ring: &[[f64; 2]], d: f64) -> Vec<[f64; 2]> {
    let n = ring.len();
    let normal = |a: [f64; 2], b: [f64; 2]| {
        let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
        let l = ex.hypot(ey).max(f64::MIN_POSITIVE);
        [ey / l, -ex / l]
    };
    (0..n)
        .map(|i| {
            let (p, v, q) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
            let (a, b) = (normal(p, v), normal(v, q));
            // the mitre: along a + b, as far as keeps each edge d away; at most 4d
            let k = (1.0 + a[0] * b[0] + a[1] * b[1]).max(0.125);
            [v[0] + d * (a[0] + b[0]) / k, v[1] + d * (a[1] + b[1]) / k]
        })
        .collect()
}

/// The scene of a structure over a window, its permittivities at `wavelength`.
pub(super) fn scene(
    s: &Structure,
    x: [f64; 2],
    y: [f64; 2],
    z: [f64; 2],
    wavelength: Wavelength,
) -> Result<Event> {
    let medium = |m: &crate::material::Material| -> Result<SceneMedium> {
        Ok(SceneMedium {
            material: m.name().to_owned(),
            eps: m.permittivity(wavelength)?.re,
        })
    };
    let stack = s.stack();
    let mut layers = Vec::new();
    let mut bottom = 0.0;
    let etched = stack.layers().iter().any(|l| !s.is_plain(&l.name));
    let mut shapes = if etched { Vec::new() } else { scene_shapes(s) };
    for layer in stack.layers() {
        let top = bottom + layer.thickness.to_um();
        if s.is_plain(&layer.name) {
            layers.push(SceneLayer {
                name: layer.name.clone(),
                z_um: [bottom, top],
                material: medium(&layer.material)?,
                background: medium(&layer.background)?,
            });
            if etched {
                shapes.extend(
                    s.shapes(&layer.name)
                        .iter()
                        .flat_map(outlines)
                        .map(|outline| SceneShape {
                            layer: layer.name.clone(),
                            outline,
                        }),
                );
            }
        } else {
            for band in bands(s, &layer.name, bottom, top) {
                layers.push(SceneLayer {
                    name: band.name.clone(),
                    z_um: band.z,
                    material: medium(&layer.material)?,
                    background: medium(if band.slab {
                        &layer.material
                    } else {
                        &layer.background
                    })?,
                });
                shapes.extend(band.outlines.into_iter().map(|outline| SceneShape {
                    layer: band.name.clone(),
                    outline,
                }));
            }
        }
        bottom = top;
    }
    // the bodies, each as bands of its sections over its height in the window; listed before
    // the stack's layers, whose last one stays the stack's top
    let window = crate::geometry::Bounds {
        min: Point::um(x[0], y[0]),
        max: Point::um(x[1], y[1]),
    };
    let mut banded = Vec::new();
    for (k, (body, medium)) in s.bodies().iter().enumerate() {
        let b = body.bounds();
        let (z0, z1) = (b.min[2].max(z[0]), b.max[2].min(z[1]));
        if z1.is_nan() || z0.is_nan() || z1 <= z0 {
            continue;
        }
        let drawn = SceneMedium {
            material: medium.name().to_owned(),
            eps: medium.permittivity(bodies::centre_of(body), wavelength)?.re,
        };
        let name = format!("body {}", k + 1);
        let count = if body.upright() { 1 } else { 12 };
        for i in 0..count {
            let (a, c) = (
                z0 + (z1 - z0) * i as f64 / count as f64,
                z0 + (z1 - z0) * (i + 1) as f64 / count as f64,
            );
            let Some(section) = body.section(0.5 * (a + c), window)? else {
                continue;
            };
            let band = if count == 1 {
                name.clone()
            } else {
                format!("{name} {}", i + 1)
            };
            banded.push(SceneLayer {
                name: band.clone(),
                z_um: [a, c],
                material: drawn.clone(),
                // nothing around it: what is there is drawn already
                background: SceneMedium {
                    material: "none".into(),
                    eps: 1.0,
                },
            });
            shapes.extend(outlines(&section).into_iter().map(|outline| SceneShape {
                layer: band.clone(),
                outline,
            }));
        }
    }
    layers.splice(0..0, banded);
    Ok(Event::Scene {
        x_um: x,
        y_um: y,
        z_um: z,
        wavelength_um: wavelength.to_um(),
        substrate: medium(stack.substrate())?,
        cladding: medium(stack.cladding())?,
        layers,
        shapes,
    })
}

/// A structure job's scene: the window, 1 µm above and below the layer seen from above unless
/// `z_um` says otherwise.
/// The scene of a job's `[device]`: its structure at its design's values, over its shapes and
/// ports with 1 µm around them, from 1 µm below its core to 1 µm above, its permittivities at its
/// `wavelength` parameter (1.55 µm for a device without one).
pub(super) fn device_scene(job: &Job) -> Result<Event> {
    let (device, design) = device::spec_of(job)?.build()?;
    let values = design.values();
    let geometry = device.geometry(values)?;
    let shapes = scene_shapes(&geometry.structure);
    let points = shapes.iter().flat_map(|s| s.outline.iter().copied()).chain(
        geometry
            .ports
            .iter()
            .map(|p| [p.position.x.to_um(), p.position.y.to_um()]),
    );
    let (mut x, mut y) = (
        [f64::INFINITY, f64::NEG_INFINITY],
        [f64::INFINITY, f64::NEG_INFINITY],
    );
    for [px, py] in points {
        x = [x[0].min(px), x[1].max(px)];
        y = [y[0].min(py), y[1].max(py)];
    }
    if !x[0].is_finite() {
        (x, y) = ([0.0, 0.0], [0.0, 0.0]);
    }
    let core = geometry
        .ports
        .first()
        .map_or_else(|| "Si".to_owned(), |p| p.layer.clone());
    let (_, bottom, top) = geometry
        .structure
        .stack()
        .layer(&core)
        .ok_or_else(|| task_error(format!("the stack has no layer {core}")))?;
    scene(
        &geometry.structure,
        [x[0] - 1.0, x[1] + 1.0],
        [y[0] - 1.0, y[1] + 1.0],
        [bottom.to_um() - 1.0, top.to_um() + 1.0],
        Wavelength::um(values.get("wavelength").unwrap_or(1.55))?,
    )
}
