//! The jobs `photonoxide run` knows, and the events they record.
//!
//! A job file's `[task]` table names its kind. The kinds so far:
//!
//! - `"structure"`: a layer stack with shapes drawn on it, recorded as pictures of its
//!   permittivity, seen from above and on a vertical cut.
//!
//! ```toml
//! name = "strip"
//! timeout_minutes = 5
//!
//! [task]
//! kind = "structure"
//! stack = "soi_220"          # or "soi" / "nitride" with core_nm and bottom_oxide_um
//! wavelength_um = 1.55
//! layer = "Si"               # the layer seen from above
//! x_um = [-3.0, 3.0]
//! y_um = [-2.0, 2.0]
//! step_nm = 20.0
//!
//! [[task.rect]]
//! layer = "Si"
//! center_um = [0.0, 0.0]
//! size_um = [6.0, 0.5]
//!
//! [[task.circle]]
//! layer = "Si"
//! center_um = [1.5, 1.2]
//! radius_um = 0.4
//!
//! [[task.ring]]              # a ring resonator's waveguide
//! layer = "Si"
//! center_um = [-1.5, 0.9]
//! radius_um = 0.8            # to the waveguide's centre line
//! width_um = 0.4
//! ```
//!
//! - `"modes"`: the guided modes of a waveguide's cross-section, the same stack and shapes cut at
//!   y = `cut_y_um` (the modes travel along y), by the full-vector solver
//!   ([`crate::mode::vector`]) on a uniform grid of `step_nm`; recorded as a picture of the
//!   cross-section and, for each mode, one of its |E|² and one of its signed transverse field,
//!   and optionally swept over the wavelength or a rectangle's width, each point recorded as it
//!   is solved, with its modes' pictures and, for a width sweep, its shapes.
//!
//! ```toml
//! name = "strip-modes"
//! timeout_minutes = 10
//!
//! [task]
//! kind = "modes"
//! stack = "soi_220"
//! wavelength_um = 1.55
//! layer = "Si"               # the window's height defaults to 1 µm around it
//! x_um = [-1.2, 1.2]
//! step_nm = 20.0
//! modes = 2                  # how many, from the highest effective index
//!
//! [[task.rect]]
//! layer = "Si"
//! center_um = [0.0, 0.0]
//! size_um = [0.5, 10.0]      # 500 nm wide, along y
//!
//! [task.sweep]
//! parameter = "wavelength"   # or "width": rect `rect`'s size along x
//! from = 1.5
//! to = 1.6
//! points = 11
//! ```
//!
//! - `"fdfd"`: a device on one layer, seen from above, by 2D FDFD ([`crate::fdfd`]) with
//!   ports: each point's permittivity is its slab's effective index squared (the effective
//!   index method, so the results are 2D estimates, not a device's 3D performance). Recorded
//!   as the S-parameters at each wavelength, and the field from the first port at the first.
//!
//! ```toml
//! name = "mmi"
//! timeout_minutes = 20
//!
//! [task]
//! kind = "fdfd"
//! stack = "soi_220"
//! wavelength_um = 1.55
//! layer = "Si"
//! polarization = "te"        # the slab's TE mode, H along z in the 2D problem; or "tm"
//! x_um = [-2.0, 12.5]        # PMLs (pml_cells, 20 by default) inside the window
//! field_um = 1.55            # the field is recorded at the swept wavelength nearest this
//! y_um = [-3.0, 3.0]
//! step_nm = 20.0
//!
//! [[task.rect]]
//! layer = "Si"
//! center_um = [4.275, 0.0]
//! size_um = [8.55, 3.0]
//!
//! [[task.port]]
//! x_um = -1.0
//! side = "left"              # light comes in towards +x
//!
//! [[task.port]]
//! x_um = 11.0
//! side = "right"
//! y_um = [0.0, 3.0]          # one guide of several: the port's window
//!
//! [task.sweep]
//! parameter = "wavelength"
//! from = 1.5
//! to = 1.6
//! points = 11
//! ```

use serde::{Deserialize, Serialize};

use crate::geometry::{Point, Shape};
use crate::raster::Raster;
use crate::run::{Job, Run, Stop, StopReason};
use crate::stack::{LayerStack, Structure};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

mod fdfd;

/// An event of a run's record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Event {
    /// The run started.
    Started {
        /// The job's name.
        job: String,
        /// The task's kind, e.g. `"structure"`.
        kind: String,
    },
    /// The structure in 3D, for the studio's 3D view: the stack's layers and every shape's
    /// outline, over the window the run looks at.
    Scene {
        /// The window along x, µm.
        x_um: [f64; 2],
        /// Along y, µm.
        y_um: [f64; 2],
        /// Along z (height), µm.
        z_um: [f64; 2],
        /// The vacuum wavelength the permittivities are at, µm.
        wavelength_um: f64,
        /// Below the stack.
        substrate: SceneMedium,
        /// Above it.
        cladding: SceneMedium,
        /// The layers, bottom to top.
        layers: Vec<SceneLayer>,
        /// Every shape drawn on a layer, as an outline.
        shapes: Vec<SceneShape>,
    },
    /// A picture of the permittivity (real part).
    Permittivity {
        /// What the picture shows, e.g. `"top view of layer Si"`.
        view: String,
        /// The axis labels, e.g. `["x", "y"]`.
        axes: [String; 2],
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The picture.
        raster: Raster,
    },
    /// A guided mode of a cross-section.
    Mode {
        /// What it is, e.g. `"mode 1 of 2"`.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The effective index, real and imaginary parts.
        effective_index: [f64; 2],
        /// The share of the transverse magnetic field in H_y (see
        /// [`crate::mode::vector::VectorMode::te_fraction`]): near 1 for a TE-like mode.
        te_fraction: f64,
        /// |E|² at the cells' centres, its largest value 1; x across, z up.
        intensity: Raster,
        /// The y where the cross-section was cut, µm.
        #[serde(default)]
        cut_y_um: f64,
    },
    /// A guided mode's signed field on its cross-section, recorded right after its
    /// [`Event::Mode`]: the transverse component of E that carries most of |E|², with the
    /// mode's global phase chosen to make that component real and positive where its magnitude
    /// peaks, and then its real part. For a lossless guided mode the component is then real
    /// everywhere up to round-off, so along the guide (the modes travel along +y) the field is
    /// this picture times cos(β (y − cut_y) − ωt), with β = k₀ Re n_eff, and a lossy mode's
    /// also decays as e^(−k₀ Im n_eff (y − cut_y)). Unlike |E|², it shows where a higher-order
    /// mode changes sign.
    ModeField {
        /// The [`Event::Mode`]'s label, e.g. `"mode 1 of 2"`.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The component, in the job's axes: `"Ex"` (across) or `"Ez"` (up).
        component: String,
        /// Its real part at the cells' centres, scaled so that its largest magnitude is 1 (from
        /// −1 to 1, positive at the peak); x across, z up, on the [`Event::Mode`]'s grid.
        values: Raster,
    },
    /// A point of a sweep: the modes' effective indices at one value of the parameter.
    SweepPoint {
        /// `"wavelength"` (µm) or `"width"` (µm).
        parameter: String,
        /// The parameter's value.
        value: f64,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The modes' effective indices, highest first, real and imaginary parts.
        effective_indices: Vec<[f64; 2]>,
        /// Their TE fractions, in the same order.
        te_fractions: Vec<f64>,
    },
    /// The shapes at a point of a width sweep, recorded after its [`Event::SweepPoint`]: the
    /// [`Event::Scene`]'s shapes with the swept rectangle at that width (the window and the
    /// layers stay the scene's).
    SweepShapes {
        /// The point's index, from 0, in the order the points are recorded.
        point: usize,
        /// The parameter's value there, µm.
        value: f64,
        /// Every shape, as the scene lists them.
        shapes: Vec<SceneShape>,
    },
    /// A mode at a point of a sweep, recorded after its [`Event::SweepPoint`] (and
    /// [`Event::SweepShapes`]): what [`Event::Mode`] and [`Event::ModeField`] record for the job's
    /// own configuration, on pixels twice the grid's step and to three decimals, so a sweep's
    /// record stays small.
    SweepMode {
        /// The point's index, from 0, in the order the points are recorded.
        point: usize,
        /// The parameter's value there.
        value: f64,
        /// What it is, e.g. `"mode 1 of 2"`, ranked by effective index at that point.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The effective index, real and imaginary parts.
        effective_index: [f64; 2],
        /// The share of the transverse magnetic field in H_y: near 1 for a TE-like mode.
        te_fraction: f64,
        /// |E|², its largest value 1; x across, z up.
        intensity: Raster,
        /// The signed field's component, as [`Event::ModeField`] names it: `"Ex"` or `"Ez"`.
        component: String,
        /// That component's real part, from −1 to 1, as [`Event::ModeField`] records it.
        field: Raster,
        /// The y where the cross-section was cut, µm.
        cut_y_um: f64,
    },
    /// A field of a 2D FDFD run, seen from above.
    Field {
        /// What it shows, e.g. `"|H_z|^2 from port 1, 2D by the effective index method"`.
        label: String,
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The height the 3D view draws it at, µm: the layer's top face.
        z_um: f64,
        /// |field|², its largest value 1; x across, y up.
        intensity: Raster,
    },
    /// A 2D FDFD run's S-parameters at one wavelength.
    SParameters {
        /// The vacuum wavelength, µm.
        wavelength_um: f64,
        /// The ports' names, in order.
        ports: Vec<String>,
        /// Each port's mode's effective index.
        effective_indices: Vec<f64>,
        /// The power-normalized S-matrix, `s[q][p]` from port p into port q, real and
        /// imaginary parts.
        s: Vec<Vec<[f64; 2]>>,
    },
    /// The run ended.
    Finished {
        /// Why it ended early, if it did: `"timeout"` or `"requested"`.
        stopped: Option<String>,
        /// Its duration, seconds.
        seconds: f64,
    },
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
fn outline(shape: &Shape) -> Vec<[f64; 2]> {
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
fn scene_shapes(s: &Structure) -> Vec<SceneShape> {
    s.stack()
        .layers()
        .iter()
        .flat_map(|layer| {
            s.shapes(&layer.name).iter().map(|shape| SceneShape {
                layer: layer.name.clone(),
                outline: outline(shape),
            })
        })
        .collect()
}

/// The scene of a structure over a window, its permittivities at `wavelength`.
fn scene(
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
    for layer in stack.layers() {
        let top = bottom + layer.thickness.to_um();
        layers.push(SceneLayer {
            name: layer.name.clone(),
            z_um: [bottom, top],
            material: medium(&layer.material)?,
            background: medium(&layer.background)?,
        });
        bottom = top;
    }
    let shapes = scene_shapes(s);
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

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RectSpec {
    layer: String,
    center_um: [f64; 2],
    size_um: [f64; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CircleSpec {
    layer: String,
    center_um: [f64; 2],
    radius_um: f64,
}

/// A ring: its centre line's radius and its waveguide's width ([`Shape::ring`]).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RingSpec {
    layer: String,
    center_um: [f64; 2],
    radius_um: f64,
    width_um: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StructureTask {
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    stack: String,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    wavelength_um: f64,
    layer: String,
    x_um: [f64; 2],
    y_um: [f64; 2],
    z_um: Option<[f64; 2]>,
    side_y_um: Option<f64>,
    step_nm: f64,
    #[serde(default)]
    rect: Vec<RectSpec>,
    #[serde(default)]
    circle: Vec<CircleSpec>,
    #[serde(default)]
    ring: Vec<RingSpec>,
}

fn task_error(reason: impl Into<String>) -> Error {
    Error::Parse {
        what: "task".into(),
        reason: reason.into(),
    }
}

/// A named stack: `soi_220`, or `soi` / `nitride` with their core and bottom oxide.
fn named_stack(
    stack: &str,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
) -> Result<LayerStack> {
    let thicknesses = || match (core_nm, bottom_oxide_um) {
        (Some(core), Some(oxide)) => Ok((Length::nm(core), Length::um(oxide))),
        _ => Err(task_error(format!(
            "stack \"{stack}\" needs core_nm and bottom_oxide_um"
        ))),
    };
    match stack {
        "soi_220" => Ok(LayerStack::soi_220()),
        "soi" => {
            let (core, oxide) = thicknesses()?;
            LayerStack::soi(core, oxide)
        }
        "nitride" => {
            let (core, oxide) = thicknesses()?;
            LayerStack::nitride(core, oxide)
        }
        other => Err(task_error(format!(
            "unknown stack \"{other}\": soi_220, soi or nitride"
        ))),
    }
}

/// The stack with the shapes drawn on it.
fn draw(
    stack: LayerStack,
    rects: &[RectSpec],
    circles: &[CircleSpec],
    rings: &[RingSpec],
) -> Result<Structure> {
    let mut s = Structure::new(stack);
    for r in rects {
        let shape = Shape::rect(
            Point::um(r.center_um[0], r.center_um[1]),
            Length::um(r.size_um[0]),
            Length::um(r.size_um[1]),
        )?;
        s.draw(&r.layer, shape)?;
    }
    for c in circles {
        let shape = Shape::circle(
            Point::um(c.center_um[0], c.center_um[1]),
            Length::um(c.radius_um),
        )?;
        s.draw(&c.layer, shape)?;
    }
    for r in rings {
        let shape = Shape::ring(
            Point::um(r.center_um[0], r.center_um[1]),
            Length::um(r.radius_um),
            Length::um(r.width_um),
        )?;
        s.draw(&r.layer, shape)?;
    }
    Ok(s)
}

impl StructureTask {
    fn structure(&self) -> Result<Structure> {
        draw(
            named_stack(&self.stack, self.core_nm, self.bottom_oxide_um)?,
            &self.rect,
            &self.circle,
            &self.ring,
        )
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SweepSpec {
    parameter: String,
    from: f64,
    to: f64,
    points: usize,
    /// For a width sweep: which rectangle, in the order given (0 by default).
    rect: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModesTask {
    #[allow(dead_code)] // read to dispatch, checked by serde
    kind: String,
    stack: String,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    wavelength_um: f64,
    layer: String,
    x_um: [f64; 2],
    z_um: Option<[f64; 2]>,
    cut_y_um: Option<f64>,
    step_nm: f64,
    modes: Option<usize>,
    #[serde(default)]
    rect: Vec<RectSpec>,
    #[serde(default)]
    circle: Vec<CircleSpec>,
    #[serde(default)]
    ring: Vec<RingSpec>,
    sweep: Option<SweepSpec>,
}

/// The cross-section of `s` cut at y over `x` × `z` (µm) on a grid of about `step`, with every
/// shape's edge on the cut and every layer interface on a node, so a sweep over a width varies
/// it smoothly; each cell the permittivity at its centre at `wavelength`.
#[allow(clippy::too_many_arguments)]
fn cross_section(
    s: &Structure,
    rects: &[RectSpec],
    circles: &[CircleSpec],
    rings: &[RingSpec],
    cut_y: f64,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
    wavelength: Wavelength,
) -> Result<crate::mode::vector::CrossSection> {
    // every material the cut can meet must know this wavelength
    let stack = s.stack();
    let mut materials = vec![stack.substrate(), stack.cladding()];
    for layer in stack.layers() {
        materials.push(&layer.material);
        materials.push(&layer.background);
    }
    for m in materials {
        m.permittivity(wavelength)?;
    }
    // the edges the cut crosses, and the layers' interfaces
    let mut x_edges = Vec::new();
    for r in rects {
        if (cut_y - r.center_um[1]).abs() < r.size_um[1] / 2.0 {
            x_edges.extend([
                r.center_um[0] - r.size_um[0] / 2.0,
                r.center_um[0] + r.size_um[0] / 2.0,
            ]);
        }
    }
    let mut circle_edges = |center: [f64; 2], radius: f64| {
        let d = cut_y - center[1];
        if d.abs() < radius {
            let half = (radius * radius - d * d).sqrt();
            x_edges.extend([center[0] - half, center[0] + half]);
        }
    };
    for c in circles {
        circle_edges(c.center_um, c.radius_um);
    }
    for r in rings {
        circle_edges(r.center_um, r.radius_um - r.width_um / 2.0);
        circle_edges(r.center_um, r.radius_um + r.width_um / 2.0);
    }
    let mut z_edges = vec![0.0];
    let mut top = 0.0;
    for layer in stack.layers() {
        top += layer.thickness.to_um();
        z_edges.push(top);
    }
    let grid = |a: f64, b: f64, fixed: &[f64]| {
        crate::mode::vector::graded_nodes(a, b, (a, b), step, 0.0, step, fixed)
    };
    let (xs, zs) = (grid(x[0], x[1], &x_edges), grid(z[0], z[1], &z_edges));
    let mut cells = Vec::with_capacity((xs.len() - 1) * (zs.len() - 1));
    for i in 0..xs.len() - 1 {
        let xc = 0.5 * (xs[i] + xs[i + 1]);
        for j in 0..zs.len() - 1 {
            let zc = 0.5 * (zs[j] + zs[j + 1]);
            let eps = s
                .material_at(Point::um(xc, cut_y), Length::um(zc))
                .permittivity(wavelength)
                .unwrap_or(num_complex::Complex64::new(f64::NAN, 0.0));
            cells.push(crate::mode::vector::Permittivity::isotropic(eps));
        }
    }
    crate::mode::vector::CrossSection::new(xs, zs, cells)
}

/// `value(i, j)` of the cross-section's cells as a picture of about `step` per pixel over
/// `x` × `z`, each pixel the cell it falls in (the grid needn't be uniform), divided by its
/// largest magnitude.
fn picture(
    cs: &crate::mode::vector::CrossSection,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
    value: impl Fn(usize, usize) -> f64,
) -> Raster {
    let (nx, ny) = (
        ((x[1] - x[0]) / step).round().max(1.0) as usize,
        ((z[1] - z[0]) / step).round().max(1.0) as usize,
    );
    // the cell holding a coordinate, along one axis
    let cell =
        |nodes: &[f64], v: f64| nodes.partition_point(|&n| n <= v).clamp(1, nodes.len() - 1) - 1;
    let mut values = vec![0.0f64; nx * ny];
    for j in 0..ny {
        let zv = z[0] + (j as f64 + 0.5) * (z[1] - z[0]) / ny as f64;
        let cj = cell(cs.y(), zv);
        for i in 0..nx {
            let xv = x[0] + (i as f64 + 0.5) * (x[1] - x[0]) / nx as f64;
            values[j * nx + i] = value(cell(cs.x(), xv), cj);
        }
    }
    let peak = values
        .iter()
        .map(|v| v.abs())
        .fold(0.0, f64::max)
        .max(f64::MIN_POSITIVE);
    Raster {
        nx,
        ny,
        x0: x[0],
        x1: x[1],
        y0: z[0],
        y1: z[1],
        values: values.iter().map(|v| (v / peak) as f32).collect(),
    }
}

/// A mode's |E|² as a [`picture`], its largest value 1.
fn intensity(
    fields: &crate::mode::fields::Fields,
    cs: &crate::mode::vector::CrossSection,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
) -> Raster {
    picture(cs, x, z, step, |i, j| {
        fields.e(i, j).iter().map(|c| c.norm_sqr()).sum()
    })
}

/// A mode's signed field as [`Event::ModeField`] records it: the name of the transverse
/// component of E with the larger share of ∫|E|² dA, and the real part of that component, its
/// global phase fixed to make it real and positive at its largest magnitude, as a [`picture`].
fn signed_field(
    fields: &crate::mode::fields::Fields,
    cs: &crate::mode::vector::CrossSection,
    x: [f64; 2],
    z: [f64; 2],
    step: f64,
) -> (String, Raster) {
    let (ni, nj) = (fields.x().len(), fields.y().len());
    let mut share = [0.0f64; 2];
    for i in 0..ni {
        for j in 0..nj {
            let (e, area) = (fields.e(i, j), fields.area[i * nj + j]);
            share[0] += e[0].norm_sqr() * area;
            share[1] += e[1].norm_sqr() * area;
        }
    }
    // the solver's (x, y) cross-section is the job's (x, z)
    let (c, name) = if share[0] >= share[1] {
        (0, "Ex")
    } else {
        (1, "Ez")
    };
    let mut peak = num_complex::Complex64::new(0.0, 0.0);
    for i in 0..ni {
        for j in 0..nj {
            let v = fields.e(i, j)[c];
            if v.norm_sqr() > peak.norm_sqr() {
                peak = v;
            }
        }
    }
    // e^(−i arg) of the peak: it, and with it the whole component, made real
    let turn = if peak.norm() > 0.0 {
        peak.conj() / peak.norm()
    } else {
        num_complex::Complex64::new(1.0, 0.0)
    };
    let raster = picture(cs, x, z, step, |i, j| (fields.e(i, j)[c] * turn).re);
    (name.to_owned(), raster)
}

fn reason(stop: &Stop) -> Option<String> {
    stop.reason().map(|r| match r {
        StopReason::Timeout => "timeout".to_owned(),
        StopReason::Requested => "requested".to_owned(),
    })
}

/// Runs `job` into `run`, recording its [`Event`]s, and stops early when `stop` says so.
///
/// # Errors
///
/// [`Error::Parse`] for a task that isn't a known kind or doesn't fit it, the errors of the
/// task itself (an invalid shape, a wavelength a material has no data for), and
/// [`Error::Io`] if the record can't be written.
pub fn execute(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let kind = job
        .task()
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| task_error("needs a kind, e.g. kind = \"structure\""))?
        .to_owned();
    run.record(&Event::Started {
        job: job.name().to_owned(),
        kind: kind.clone(),
    })?;
    match kind.as_str() {
        "structure" => structure(job, run, stop)?,
        "modes" => modes(job, run, stop)?,
        "fdfd" => fdfd::run(job, run, stop)?,
        other => return Err(task_error(format!("unknown kind \"{other}\""))),
    }
    let seconds = run.elapsed().as_secs_f64();
    run.record(&Event::Finished {
        stopped: reason(stop),
        seconds,
    })
}

/// Checks `job` without running it: its kind is known, its task fits that kind, the stack is
/// known, the shapes are valid, the layer is in the stack and the wavelength is valid; for an
/// `"fdfd"` job, the polarization and that it has ports too. What only a solve finds (a port
/// inside the PML, a material without data at a wavelength) is left to the run.
///
/// # Errors
///
/// The errors [`execute`] would return for those, before recording anything.
pub fn check(job: &Job) -> Result<()> {
    let kind = job
        .task()
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| task_error("needs a kind, e.g. kind = \"structure\""))?;
    let parse = |e: toml::de::Error| task_error(e.to_string());
    let (s, layer, wavelength_um) = match kind {
        "structure" => {
            let task: StructureTask = job.task().clone().try_into().map_err(parse)?;
            (task.structure()?, task.layer, task.wavelength_um)
        }
        "modes" => {
            let task: ModesTask = job.task().clone().try_into().map_err(parse)?;
            let stack = named_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?;
            let s = draw(stack, &task.rect, &task.circle, &task.ring)?;
            (s, task.layer, task.wavelength_um)
        }
        "fdfd" => return fdfd::check(job),
        other => return Err(task_error(format!("unknown kind \"{other}\""))),
    };
    s.stack()
        .layer(&layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {layer}")))?;
    Wavelength::um(wavelength_um)?;
    Ok(())
}

fn structure(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let task: StructureTask = job
        .task()
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(e.to_string()))?;
    let s = task.structure()?;
    let lam = Wavelength::um(task.wavelength_um)?;
    run.record(&structure_scene(&task, &s)?)?;
    let step = Length::nm(task.step_nm);
    let x = (Length::um(task.x_um[0]), Length::um(task.x_um[1]));
    let y = (Length::um(task.y_um[0]), Length::um(task.y_um[1]));
    let top = s.top_view(&task.layer, x, y, step, lam)?;
    run.record(&Event::Permittivity {
        view: format!("top view of layer {}", task.layer),
        axes: ["x".into(), "y".into()],
        wavelength_um: lam.to_um(),
        raster: top,
    })?;
    if stop.reason().is_some() {
        return Ok(());
    }
    let (_, bottom, top_z) = s
        .stack()
        .layer(&task.layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {}", task.layer)))?;
    let z = match task.z_um {
        Some([a, b]) => (Length::um(a), Length::um(b)),
        None => (bottom - Length::um(1.0), top_z + Length::um(1.0)),
    };
    let side_y = Length::um(task.side_y_um.unwrap_or(0.0));
    let side = s.side_view(side_y, x, z, step, lam)?;
    run.record(&Event::Permittivity {
        view: format!("side view at y = {} um", side_y.to_um()),
        axes: ["x".into(), "z".into()],
        wavelength_um: lam.to_um(),
        raster: side,
    })
}

/// A structure job's scene: the window, 1 µm above and below the layer seen from above unless
/// `z_um` says otherwise.
fn structure_scene(task: &StructureTask, s: &Structure) -> Result<Event> {
    let z = match task.z_um {
        Some(z) => z,
        None => {
            let (_, bottom, top) = s
                .stack()
                .layer(&task.layer)
                .ok_or_else(|| task_error(format!("the stack has no layer {}", task.layer)))?;
            [bottom.to_um() - 1.0, top.to_um() + 1.0]
        }
    };
    scene(
        s,
        task.x_um,
        task.y_um,
        z,
        Wavelength::um(task.wavelength_um)?,
    )
}

/// A modes job's window height: `z_um`, else 1 µm below and above its layer.
fn modes_z(task: &ModesTask, s: &Structure) -> Result<[f64; 2]> {
    match task.z_um {
        Some(z) => Ok(z),
        None => {
            let (_, bottom, top) = s
                .stack()
                .layer(&task.layer)
                .ok_or_else(|| task_error(format!("the stack has no layer {}", task.layer)))?;
            Ok([bottom.to_um() - 1.0, top.to_um() + 1.0])
        }
    }
}

/// A modes job's scene: a block behind the cut, as deep as the window is wide.
fn modes_scene(task: &ModesTask, s: &Structure) -> Result<Event> {
    let cut_y = task.cut_y_um.unwrap_or(0.0);
    let depth = task.x_um[1] - task.x_um[0];
    scene(
        s,
        task.x_um,
        [cut_y - depth, cut_y],
        modes_z(task, s)?,
        Wavelength::um(task.wavelength_um)?,
    )
}

/// The scene `job`'s run records first, its structure over its window, without running it: what
/// the studio draws of a job before it runs. Checked as [`check`] does.
///
/// # Errors
///
/// The errors of [`check`].
pub fn preview(job: &Job) -> Result<Event> {
    check(job)?;
    let parse = |e: toml::de::Error| task_error(e.to_string());
    match job.task().get("kind").and_then(|k| k.as_str()) {
        Some("structure") => {
            let task: StructureTask = job.task().clone().try_into().map_err(parse)?;
            structure_scene(&task, &task.structure()?)
        }
        Some("modes") => {
            let task: ModesTask = job.task().clone().try_into().map_err(parse)?;
            let stack = named_stack(&task.stack, task.core_nm, task.bottom_oxide_um)?;
            modes_scene(&task, &draw(stack, &task.rect, &task.circle, &task.ring)?)
        }
        _ => fdfd::preview(job),
    }
}

fn modes(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let task: ModesTask = job
        .task()
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(e.to_string()))?;
    let stack = || named_stack(&task.stack, task.core_nm, task.bottom_oxide_um);
    let count = task.modes.unwrap_or(2).max(1);
    let lam = Wavelength::um(task.wavelength_um)?;
    let step = task.step_nm / 1000.0;
    let cut_y = task.cut_y_um.unwrap_or(0.0);
    let s = draw(stack()?, &task.rect, &task.circle, &task.ring)?;
    let z = modes_z(&task, &s)?;
    run.record(&modes_scene(&task, &s)?)?;
    // the cross-section, as the structure job shows a side view
    run.record(&Event::Permittivity {
        view: format!("cross-section at y = {cut_y} um"),
        axes: ["x".into(), "z".into()],
        wavelength_um: lam.to_um(),
        raster: s.side_view(
            Length::um(cut_y),
            (Length::um(task.x_um[0]), Length::um(task.x_um[1])),
            (Length::um(z[0]), Length::um(z[1])),
            Length::um(step),
            lam,
        )?,
    })?;
    let cs = cross_section(
        &s,
        &task.rect,
        &task.circle,
        &task.ring,
        cut_y,
        task.x_um,
        z,
        step,
        lam,
    )?;
    let found = crate::mode::vector::modes(&cs, lam, count, None)?;
    let mut sorted: Vec<_> = found.iter().collect();
    sorted.sort_by(|a, b| b.effective_index().re.total_cmp(&a.effective_index().re));
    for (k, m) in sorted.iter().enumerate() {
        if stop.reason().is_some() {
            return Ok(());
        }
        let n = m.effective_index();
        let fields = m.fields(&cs)?;
        let label = format!("mode {} of {}", k + 1, sorted.len());
        run.record(&Event::Mode {
            label: label.clone(),
            wavelength_um: lam.to_um(),
            effective_index: [n.re, n.im],
            te_fraction: m.te_fraction(),
            intensity: intensity(&fields, &cs, task.x_um, z, step),
            cut_y_um: cut_y,
        })?;
        let (component, values) = signed_field(&fields, &cs, task.x_um, z, step);
        run.record(&Event::ModeField {
            label,
            wavelength_um: lam.to_um(),
            component,
            values,
        })?;
    }
    let Some(sweep) = &task.sweep else {
        return Ok(());
    };
    if sweep.points < 2 || !(sweep.from.is_finite() && sweep.to.is_finite()) {
        return Err(task_error("a sweep needs from, to and at least 2 points"));
    }
    for k in 0..sweep.points {
        if stop.reason().is_some() {
            return Ok(());
        }
        let value = sweep.from + (sweep.to - sweep.from) * k as f64 / (sweep.points - 1) as f64;
        // the point's shapes, for a width sweep
        let mut shapes = None;
        let (cs, w) = match sweep.parameter.as_str() {
            "wavelength" => {
                let w = Wavelength::um(value)?;
                (
                    cross_section(
                        &s,
                        &task.rect,
                        &task.circle,
                        &task.ring,
                        cut_y,
                        task.x_um,
                        z,
                        step,
                        w,
                    )?,
                    w,
                )
            }
            "width" => {
                let index = sweep.rect.unwrap_or(0);
                let mut rects: Vec<RectSpec> = task.rect.iter().map(RectSpec::clone).collect();
                let r = rects.get_mut(index).ok_or_else(|| {
                    task_error(format!("the width sweep's rect {index} isn't there"))
                })?;
                r.size_um[0] = value;
                let swept = draw(stack()?, &rects, &task.circle, &task.ring)?;
                shapes = Some(scene_shapes(&swept));
                (
                    cross_section(
                        &swept,
                        &rects,
                        &task.circle,
                        &task.ring,
                        cut_y,
                        task.x_um,
                        z,
                        step,
                        lam,
                    )?,
                    lam,
                )
            }
            other => {
                return Err(task_error(format!(
                    "unknown sweep parameter \"{other}\": wavelength or width"
                )));
            }
        };
        let mut found = crate::mode::vector::modes(&cs, w, count, None)?;
        found.sort_by(|a, b| b.effective_index().re.total_cmp(&a.effective_index().re));
        run.record(&Event::SweepPoint {
            parameter: sweep.parameter.clone(),
            value,
            wavelength_um: w.to_um(),
            effective_indices: found
                .iter()
                .map(|m| [m.effective_index().re, m.effective_index().im])
                .collect(),
            te_fractions: found
                .iter()
                .map(crate::mode::vector::VectorMode::te_fraction)
                .collect(),
        })?;
        if let Some(shapes) = shapes {
            run.record(&Event::SweepShapes {
                point: k,
                value,
                shapes,
            })?;
        }
        // each mode's pictures, coarser than the job's own: twice the step, three decimals
        let coarse = |mut r: Raster| {
            for v in &mut r.values {
                // (+ 0.0 makes −0 zero)
                *v = (*v * 1000.0).round() / 1000.0 + 0.0;
            }
            r
        };
        for (rank, m) in found.iter().enumerate() {
            if stop.reason().is_some() {
                return Ok(());
            }
            let fields = m.fields(&cs)?;
            let (component, field) = signed_field(&fields, &cs, task.x_um, z, 2.0 * step);
            let n = m.effective_index();
            run.record(&Event::SweepMode {
                point: k,
                value,
                label: format!("mode {} of {}", rank + 1, found.len()),
                wavelength_um: w.to_um(),
                effective_index: [n.re, n.im],
                te_fraction: m.te_fraction(),
                intensity: coarse(intensity(&fields, &cs, task.x_um, z, 2.0 * step)),
                component,
                field: coarse(field),
                cut_y_um: cut_y,
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::replay;
    use std::path::PathBuf;

    const JOB: &str = r#"
name = "strip"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 1.0]
y_um = [-1.0, 1.0]
step_nm = 50.0

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [2.0, 0.5]
"#;

    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn temp(tag: &str) -> TempDir {
        let dir =
            std::env::temp_dir().join(format!("photonoxide-job-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        TempDir(dir)
    }

    #[test]
    fn a_structure_job_records_a_top_and_a_side_view() {
        let root = temp("views");
        let job = Job::parse(JOB).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        assert_eq!(events.len(), 5);
        assert!(matches!(&events[0], Event::Started { kind, .. } if kind == "structure"));
        let Event::Scene { layers, shapes, .. } = &events[1] else {
            panic!("{:?}", events[1])
        };
        // soi_220: the buried oxide and the silicon layer; the strip a rectangle on it
        assert_eq!(layers.len(), 2);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].outline.len(), 4);
        assert!(layers[1].material.eps > 11.0 && layers[1].background.eps < 2.2);
        let Event::Permittivity { raster, axes, .. } = &events[2] else {
            panic!("{:?}", events[2])
        };
        assert_eq!(axes, &["x".to_owned(), "y".to_owned()]);
        assert_eq!((raster.nx, raster.ny), (40, 40));
        let Event::Permittivity { raster, axes, .. } = &events[3] else {
            panic!("{:?}", events[3])
        };
        assert_eq!(axes[1], "z");
        // the default cut spans 1 um below and above the 220 nm layer
        assert!((raster.y1 - raster.y0 - 2.22).abs() < 1e-9);
        assert!(matches!(&events[4], Event::Finished { stopped: None, .. }));
    }

    #[test]
    fn a_stopped_job_says_why() {
        let root = temp("stopped");
        let job = Job::parse(JOB).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        let stop = Stop::new(None);
        stop.request();
        execute(&job, &mut run, &stop).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // the scene and the top view, then it stops before the side view
        assert_eq!(events.len(), 4);
        assert!(matches!(
            &events[3],
            Event::Finished { stopped: Some(r), .. } if r == "requested"
        ));
    }

    const MODES: &str = r#"
name = "strip-modes"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 1.0]
step_nm = 25.0
modes = 2

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [0.5, 10.0]
"#;

    #[test]
    fn a_modes_job_records_the_strips_modes() {
        let root = temp("modes");
        let job = Job::parse(MODES).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // started, the scene, the cross-section, two modes and their fields, finished
        assert_eq!(events.len(), 8, "{events:?}");
        let Event::Mode {
            effective_index,
            te_fraction,
            intensity,
            ..
        } = &events[3]
        else {
            panic!("{:?}", events[3])
        };
        // the 500 x 220 nm strip's TE-like mode, first; its field peaks in the core
        assert!(
            effective_index[0] > 2.3 && effective_index[0] < 2.6,
            "{effective_index:?}"
        );
        assert!(*te_fraction > 0.9);
        let (lo, hi) = intensity.range();
        assert!(lo >= 0.0 && (hi - 1.0).abs() < 1e-6);
        // its signed field: E_x, of one sign, on the same grid, peaking where |E|² does
        let Event::ModeField {
            label,
            component,
            values,
            ..
        } = &events[4]
        else {
            panic!("{:?}", events[4])
        };
        assert_eq!(label, "mode 1 of 2");
        assert_eq!(component, "Ex");
        assert_eq!((values.nx, values.ny), (intensity.nx, intensity.ny));
        assert!((values.range().1 - 1.0).abs() < 1e-6);
        // of one sign, but for a cell or two at the core's corners, where E_x is singular
        for j in 0..values.ny {
            for i in 0..values.nx {
                let v = values.at(i, j);
                let x = values.x0 + (i as f64 + 0.5) * (values.x1 - values.x0) / values.nx as f64;
                let z = values.y0 + (j as f64 + 0.5) * (values.y1 - values.y0) / values.ny as f64;
                let corner = [-0.25, 0.25]
                    .iter()
                    .any(|&cx| [2.0, 2.22].iter().any(|&cz| (x - cx).hypot(z - cz) < 0.06));
                assert!(v > -0.02 || corner, "{v} at ({x}, {z})");
            }
        }
        let top = values.values.iter().position(|&v| v == 1.0).unwrap();
        assert!(intensity.values[top] > 0.8, "{}", intensity.values[top]);
        // E_x is most of |E|² in the fundamental TE mode: its square follows |E|² closely
        let squares: Vec<f64> = values.values.iter().map(|&v| f64::from(v * v)).collect();
        let total: Vec<f64> = intensity.values.iter().map(|&v| f64::from(v)).collect();
        let mean = |a: &[f64]| a.iter().sum::<f64>() / a.len() as f64;
        let (ma, mb) = (mean(&squares), mean(&total));
        let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
        for (a, b) in squares.iter().zip(&total) {
            ab += (a - ma) * (b - mb);
            aa += (a - ma) * (a - ma);
            bb += (b - mb) * (b - mb);
        }
        let correlation = ab / (aa * bb).sqrt();
        assert!(correlation > 0.95, "{correlation}");
        let Event::Mode {
            effective_index: second,
            te_fraction,
            ..
        } = &events[5]
        else {
            panic!("{:?}", events[5])
        };
        assert!(second[0] < effective_index[0] && *te_fraction < 0.1);
        // the TM-like mode's field is mostly vertical
        assert!(matches!(&events[6], Event::ModeField { component, .. } if component == "Ez"));
    }

    #[test]
    fn a_higher_order_modes_field_changes_sign_across_the_core() {
        let root = temp("te1");
        // a 1 µm strip guides TE0, TE1 and TM0; TE1 is odd in x
        let text = MODES
            .replace("size_um = [0.5, 10.0]", "size_um = [1.0, 10.0]")
            .replace("x_um = [-1.0, 1.0]", "x_um = [-1.2, 1.2]")
            .replace("step_nm = 25.0", "step_nm = 40.0")
            .replace("modes = 2", "modes = 3");
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        let fields: Vec<(&String, &Raster)> = events
            .iter()
            .filter_map(|e| match e {
                Event::ModeField {
                    component, values, ..
                } => Some((component, values)),
                _ => None,
            })
            .collect();
        assert_eq!(fields.len(), 3);
        // E_x along the row through the core's middle, a quarter of the width either side
        let across = |r: &Raster, x: f64| {
            let i = (((x - r.x0) / (r.x1 - r.x0)) * r.nx as f64) as usize;
            // the silicon is 2 to 2.22 µm up, on the buried oxide
            let j = (((2.11 - r.y0) / (r.y1 - r.y0)) * r.ny as f64) as usize;
            r.values[j * r.nx + i]
        };
        let (_, te0) = fields[0];
        assert!(across(te0, -0.25) > 0.3 && across(te0, 0.25) > 0.3);
        let odd = fields.iter().any(|(c, r)| {
            let (a, b) = (across(r, -0.25), across(r, 0.25));
            *c == "Ex" && a * b < 0.0 && a.abs() > 0.3 && b.abs() > 0.3
        });
        assert!(odd, "no mode changes sign across the core");
    }

    #[test]
    fn a_width_sweep_records_rising_indices() {
        let root = temp("sweep");
        // 450 to 550 nm: the index rises ever more slowly (a_cross_sections_nodes_hold_its_edges
        // checks the edges are on nodes, so the widths don't snap)
        let text = format!(
            "{MODES}\n[task.sweep]\nparameter = \"width\"\nfrom = 0.45\nto = 0.55\npoints = 3\n"
        );
        let job = Job::parse(&text.replace("modes = 2", "modes = 1")).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        let points: Vec<f64> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepPoint {
                    effective_indices, ..
                } => Some(effective_indices[0][0]),
                _ => None,
            })
            .collect();
        assert_eq!(points.len(), 3);
        assert!(points.windows(2).all(|w| w[1] > w[0]), "{points:?}");
        // rising ever more slowly, as a widening strip does
        assert!(points[1] - points[0] > points[2] - points[1], "{points:?}");
        // each point's structure: the strip at that width
        let widths: Vec<(usize, f64, f64)> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepShapes {
                    point,
                    value,
                    shapes,
                } => {
                    let xs = shapes[0].outline.iter().map(|p| p[0]);
                    let span = xs.clone().fold(f64::MIN, f64::max) - xs.fold(f64::MAX, f64::min);
                    Some((*point, *value, span))
                }
                _ => None,
            })
            .collect();
        assert_eq!(widths.len(), 3);
        for (k, (point, value, span)) in widths.iter().enumerate() {
            assert_eq!(*point, k);
            assert!((value - (0.45 + 0.05 * k as f64)).abs() < 1e-9);
            assert!((span - value).abs() < 1e-9, "{span} {value}");
        }
        // and its mode, with its pictures, at that point's index and n_eff
        let modes: Vec<&Event> = events
            .iter()
            .filter(|e| matches!(e, Event::SweepMode { .. }))
            .collect();
        assert_eq!(modes.len(), 3);
        for (k, e) in modes.iter().enumerate() {
            let Event::SweepMode {
                point,
                label,
                effective_index,
                intensity,
                component,
                field,
                ..
            } = e
            else {
                unreachable!()
            };
            assert_eq!((*point, label.as_str()), (k, "mode 1 of 1"));
            assert_eq!(effective_index[0], points[k]);
            assert_eq!(component, "Ex");
            // on pixels twice the step: the 2 µm window in 40 pixels of 50 nm
            assert_eq!((intensity.nx, field.nx), (40, 40));
            assert!(
                (intensity.range().1 - 1.0).abs() < 1e-6 && (field.range().1 - 1.0).abs() < 1e-6
            );
            // to three decimals
            assert!(
                field
                    .values
                    .iter()
                    .all(|v| (v * 1000.0 - (v * 1000.0).round()).abs() < 1e-3)
            );
        }
        // the point's events come after its SweepPoint
        let first = events
            .iter()
            .position(|e| matches!(e, Event::SweepPoint { .. }))
            .unwrap();
        assert!(matches!(
            &events[first + 1],
            Event::SweepShapes { point: 0, .. }
        ));
        assert!(matches!(
            &events[first + 2],
            Event::SweepMode { point: 0, .. }
        ));
    }

    #[test]
    fn a_wavelength_sweep_records_each_points_modes() {
        let root = temp("wavelength-sweep");
        let text = format!(
            "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n"
        );
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // no shapes: the structure stays the job's
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::SweepShapes { .. }))
        );
        let modes: Vec<(usize, f64, String, f64)> = events
            .iter()
            .filter_map(|e| match e {
                Event::SweepMode {
                    point,
                    wavelength_um,
                    label,
                    te_fraction,
                    ..
                } => Some((*point, *wavelength_um, label.clone(), *te_fraction)),
                _ => None,
            })
            .collect();
        assert_eq!(modes.len(), 4, "{modes:?}");
        for (k, (point, wavelength, label, te)) in modes.iter().enumerate() {
            assert_eq!(*point, k / 2);
            assert!((wavelength - [1.5, 1.6][k / 2]).abs() < 1e-12);
            assert_eq!(label, &format!("mode {} of 2", k % 2 + 1));
            // TE-like first, then TM-like, at both wavelengths
            assert!(if k % 2 == 0 { *te > 0.9 } else { *te < 0.1 });
        }
    }

    #[test]
    fn a_cross_sections_nodes_hold_its_edges_and_interfaces() {
        // a 450 nm strip on a 20 nm grid from −1 µm: ±0.225 µm aren't multiples of the step, so
        // a uniform grid would snap the width; they, and the layers' interfaces, must be nodes
        let rect = RectSpec {
            layer: "Si".into(),
            center_um: [0.0, 0.0],
            size_um: [0.45, 10.0],
        };
        let s = draw(LayerStack::soi_220(), std::slice::from_ref(&rect), &[], &[]).unwrap();
        let cs = cross_section(
            &s,
            &[rect],
            &[],
            &[],
            0.0,
            [-1.0, 1.0],
            [-1.0, 1.22],
            0.02,
            Wavelength::um(1.55).unwrap(),
        )
        .unwrap();
        let has = |nodes: &[f64], v: f64| nodes.iter().any(|&n| (n - v).abs() < 1e-12);
        assert!(has(cs.x(), -0.225) && has(cs.x(), 0.225), "{:?}", cs.x());
        assert!(has(cs.y(), 0.0) && has(cs.y(), 0.22), "{:?}", cs.y());
        // and the spacing stays about the step
        assert!(
            cs.x()
                .windows(2)
                .all(|w| w[1] - w[0] <= 0.02 + 1e-12 && w[1] - w[0] > 0.005)
        );
    }

    #[test]
    fn a_circles_outline_is_a_counterclockwise_64_gon() {
        let c = Shape::circle(Point::um(1.0, 2.0), Length::um(0.5)).unwrap();
        let o = outline(&c);
        assert_eq!(o.len(), 64);
        // counterclockwise: a positive signed area, close to the disk's
        let area: f64 = (0..64)
            .map(|k| {
                let (a, b) = (o[k], o[(k + 1) % 64]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            / 2.0;
        assert!(
            (area - std::f64::consts::PI * 0.25).abs() < 0.01 * area,
            "{area}"
        );
    }

    #[test]
    fn a_preview_is_the_scene_its_run_records() {
        let root = temp("preview");
        let ring = "
[[task.ring]]
layer = \"Si\"
center_um = [0.0, 0.6]
radius_um = 0.5
width_um = 0.3
";
        for text in [
            format!("{JOB}{ring}"),
            format!("{MODES}{ring}"),
            format!("{FDFD}{ring}"),
        ] {
            let job = Job::parse(&text).unwrap();
            let preview = preview(&job).unwrap();
            let mut run = Run::create(&root.0, &job).unwrap();
            let stop = Stop::new(None);
            stop.request(); // the scene comes first; nothing more is needed
            let _ = execute(&job, &mut run, &stop);
            let events: Vec<Event> = crate::run::replay(run.dir()).unwrap();
            let recorded = events
                .iter()
                .find(|e| matches!(e, Event::Scene { .. }))
                .unwrap();
            assert_eq!(&preview, recorded);
            let Event::Scene { shapes, .. } = preview else {
                unreachable!()
            };
            assert_eq!(
                shapes.last().unwrap().outline.len(),
                130,
                "the ring, with its hole"
            );
        }
    }

    #[test]
    fn a_rings_outline_walks_its_hole_through_a_cut() {
        let r = Shape::ring(Point::um(0.0, 0.0), Length::um(1.0), Length::um(0.5)).unwrap();
        let o = outline(&r);
        assert_eq!(o.len(), 64 + 1 + 64 + 1);
        // the first vertex recurs where the hole starts, and the hole closes on itself
        assert_eq!(o[64], o[0]);
        assert_eq!(o[129], o[65]);
        let area = |v: &[[f64; 2]]| {
            (0..v.len())
                .map(|k| {
                    let (a, b) = (v[k], v[(k + 1) % v.len()]);
                    a[0] * b[1] - b[0] * a[1]
                })
                .sum::<f64>()
                / 2.0
        };
        // outer counterclockwise, hole clockwise, and the whole walk the ring's area
        assert!(area(&o[..64]) > 0.0 && area(&o[65..129]) < 0.0);
        let exact = std::f64::consts::PI * (1.25 * 1.25 - 0.75 * 0.75);
        assert!((area(&o) - exact).abs() < 0.01 * exact, "{}", area(&o));
    }

    #[test]
    fn bad_tasks_are_errors() {
        let root = temp("bad");
        for (text, says) in [
            (
                JOB.replace("kind = \"structure\"", "kind = \"fdtd\""),
                "unknown kind",
            ),
            (
                JOB.replace("stack = \"soi_220\"", "stack = \"glass\""),
                "unknown stack",
            ),
            (
                JOB.replace("stack = \"soi_220\"", "stack = \"soi\""),
                "core_nm",
            ),
            (JOB.replace("step_nm", "step"), "step"),
            (
                JOB.replace("layer = \"Si\"\ncenter", "layer = \"M1\"\ncenter"),
                "M1",
            ),
        ] {
            let job = Job::parse(&text).unwrap();
            // found by the check, without running, as by the run
            let e = check(&job).unwrap_err();
            assert!(e.to_string().contains(says), "check, {says}: {e}");
            let mut run = Run::create(&root.0, &job).unwrap();
            let e = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert!(e.to_string().contains(says), "{says}: {e}");
        }
    }

    #[test]
    fn the_repositorys_jobs_pass_the_check() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
        let mut count = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|x| x == "toml") {
                check(&Job::load(&path).unwrap()).unwrap_or_else(|e| panic!("{path:?}: {e}"));
                count += 1;
            }
        }
        assert!(count >= 4);
        assert!(check(&Job::parse(FDFD).unwrap()).is_ok());
        let portless = FDFD.split("[[task.port]]").next().unwrap();
        let e = check(&Job::parse(portless).unwrap()).unwrap_err();
        assert!(e.to_string().contains("port"), "{e}");
    }

    const FDFD: &str = r#"
name = "strip-fdfd"

[task]
kind = "fdfd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.4, 1.4]
y_um = [-1.4, 1.4]
step_nm = 40.0

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [4.0, 0.5]

[[task.port]]
x_um = -0.4
side = "left"

[[task.port]]
x_um = 0.4
side = "right"

[task.sweep]
parameter = "wavelength"
from = 1.5
to = 1.6
points = 2
"#;

    #[test]
    fn an_fdfd_job_records_s_parameters_and_a_field() {
        let root = temp("fdfd");
        let job = Job::parse(FDFD).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // started, the scene, S at 1.5 um, the field, S at 1.6 um, finished
        assert_eq!(events.len(), 6, "{events:?}");
        assert!(matches!(&events[1], Event::Scene { .. }));
        let Event::SParameters {
            wavelength_um,
            ports,
            effective_indices,
            s,
        } = &events[2]
        else {
            panic!("{:?}", events[2])
        };
        assert_eq!(*wavelength_um, 1.5);
        assert_eq!(ports.len(), 2);
        // a straight strip: everything through, in a mode between the oxide's and the slab's
        // index (the slab's TE mode, 2.85, in oxide, 1.444)
        let through = s[1][0][0].hypot(s[1][0][1]);
        assert!((through - 1.0).abs() < 1e-6, "{s:?}");
        assert!(
            effective_indices.iter().all(|&n| n > 1.444 && n < 2.85),
            "{effective_indices:?}"
        );
        let Event::Field { intensity, .. } = &events[3] else {
            panic!("{:?}", events[3])
        };
        assert_eq!((intensity.nx, intensity.ny), (70, 70));
        assert!(
            matches!(&events[4], Event::SParameters { wavelength_um, .. } if *wavelength_um == 1.6)
        );
    }

    #[test]
    fn an_fdfd_job_records_the_field_at_the_wavelength_asked() {
        let root = temp("fdfd-field");
        let text = FDFD.replace(
            "step_nm = 40.0",
            "step_nm = 40.0
field_um = 1.58",
        );
        let job = Job::parse(&text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let events: Vec<Event> = replay(run.dir()).unwrap();
        // the swept wavelength nearest 1.58 is 1.6, the second: its field follows its S
        let fields: Vec<f64> = events
            .iter()
            .filter_map(|e| match e {
                Event::Field { wavelength_um, .. } => Some(*wavelength_um),
                _ => None,
            })
            .collect();
        assert_eq!(fields, [1.6]);
        assert!(matches!(&events[4], Event::Field { .. }), "{:?}", events[4]);
    }

    #[test]
    fn an_fdfd_job_without_ports_or_with_a_port_in_the_pml_is_an_error() {
        let ports = "[[task.port]]\nx_um = -0.4\nside = \"left\"\n\n[[task.port]]\nx_um = 0.4\nside = \"right\"";
        let in_pml = "[[task.port]]\nx_um = -1.35\nside = \"left\"\n\n[[task.port]]\nx_um = 0.4\nside = \"right\"";
        for (to, error) in [("", "at least one"), (in_pml, "clear of the PMLs")] {
            let root = temp("fdfd-bad");
            let job = Job::parse(&FDFD.replace(ports, to)).unwrap();
            let mut run = Run::create(&root.0, &job).unwrap();
            let e = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
            assert!(e.to_string().contains(error), "{e}");
        }
    }
}
