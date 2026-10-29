//! The structure task, and the structures the other tasks draw.

use schemars::{JsonSchema, SchemaGenerator};
use serde::Deserialize;

use super::check::{check_cells, check_finite, check_layer, check_step, check_window, task_error};
use super::event::Event;
use super::execute::solver_run;
use super::kind::TaskKind;
use super::scene::{record_ports, scene};
use super::{bodies, components, params, schema, shapes};
use crate::geometry::{Point, Shape};
use crate::run::{Job, Run, Stop};
use crate::stack::{LayerStack, Structure};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct RectSpec {
    pub(super) layer: String,
    pub(super) center_um: [f64; 2],
    pub(super) size_um: [f64; 2],
    pub(super) etch: Option<EtchSpec>,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct CircleSpec {
    pub(super) layer: String,
    pub(super) center_um: [f64; 2],
    pub(super) radius_um: f64,
    pub(super) etch: Option<EtchSpec>,
}

/// A ring: its centre line's radius and its waveguide's width ([`Shape::ring`]).
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct RingSpec {
    pub(super) layer: String,
    pub(super) center_um: [f64; 2],
    pub(super) radius_um: f64,
    pub(super) width_um: f64,
    pub(super) etch: Option<EtchSpec>,
}

/// A shape's `etch = { depth_um, sidewall_deg }` ([`crate::stack::Etch`]): how deep from its
/// layer's top (through it when left out) and its walls' angle from the layer's plane (90 when
/// left out). `depth` and `sidewall` take expressions with units (`"70 nm"`, `"85 deg"`).
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct EtchSpec {
    depth_um: Option<f64>,
    sidewall_deg: Option<f64>,
}

impl EtchSpec {
    pub(super) fn etch(spec: Option<EtchSpec>) -> crate::stack::Etch {
        let mut etch = crate::stack::Etch::FULL;
        if let Some(e) = spec {
            if let Some(d) = e.depth_um {
                etch = crate::stack::Etch::partial(Length::um(d));
            }
            if let Some(a) = e.sidewall_deg {
                etch = etch.with_sidewall_deg(a);
            }
        }
        etch
    }
}

/// A task's `stack`: a preset's name, or a `[task.stack]` table of layers.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
pub(super) enum StackField {
    Name(String),
    #[schemars(with = "crate::job::schema::Layers")]
    Layers(toml::Table),
}

/// A path ([`crate::geometry::Path`]): its start, heading and width, and its segments, every
/// value resolved to µm and radians.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct PathSpec {
    layer: String,
    #[serde(default)]
    start_um: [f64; 2],
    #[serde(default)]
    heading_rad: f64,
    width_um: f64,
    segments: Vec<SegmentSpec>,
    etch: Option<EtchSpec>,
}

/// One segment of a [`PathSpec`]: exactly one of a straight, an arc, an Euler bend or a sine
/// S-bend, with what it needs, and the width at its end if it tapers.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct SegmentSpec {
    straight_um: Option<f64>,
    arc_rad: Option<f64>,
    euler_rad: Option<f64>,
    sine_um: Option<f64>,
    radius_um: Option<f64>,
    offset_um: Option<f64>,
    p: Option<f64>,
    width_um: Option<f64>,
}

impl PathSpec {
    /// The path, its errors naming the table and segment (`k` from 0).
    pub(super) fn path(&self, k: usize) -> Result<crate::geometry::Path> {
        path_of(
            self.start_um,
            self.heading_rad,
            self.width_um,
            &self.segments,
            &format!("[[task.path]] #{}", k + 1),
        )
    }
}

/// A path from its start, heading, width and segments, its errors naming it as `what`.
pub(super) fn path_of(
    start_um: [f64; 2],
    heading_rad: f64,
    width_um: f64,
    segments: &[SegmentSpec],
    what: &str,
) -> Result<crate::geometry::Path> {
    use crate::geometry::{Path, Segment};
    let at = |m: String| task_error(format!("{what}: {m}"));
    let mut b = Path::builder(
        Point::um(start_um[0], start_um[1]),
        heading_rad,
        Length::um(width_um),
    );
    for (j, s) in segments.iter().enumerate() {
        let at = |m: &str| at(format!("segment {}: {m}", j + 1));
        let kinds = [s.straight_um, s.arc_rad, s.euler_rad, s.sine_um]
            .iter()
            .filter(|v| v.is_some())
            .count();
        if kinds != 1 {
            return Err(at(
                "needs exactly one of straight, arc, euler and sine (their lengths or angles)",
            ));
        }
        let radius = || {
            s.radius_um
                .map(Length::um)
                .ok_or_else(|| at("an arc or Euler bend needs its radius"))
        };
        let unused = |name: &str, used: bool| {
            if used {
                Err(at(&format!("{name} isn't for this kind of segment")))
            } else {
                Ok(())
            }
        };
        let segment = if let Some(l) = s.straight_um {
            unused("radius", s.radius_um.is_some())?;
            unused("offset", s.offset_um.is_some())?;
            unused("p", s.p.is_some())?;
            Segment::Straight {
                length: Length::um(l),
            }
        } else if let Some(a) = s.arc_rad {
            unused("offset", s.offset_um.is_some())?;
            unused("p", s.p.is_some())?;
            Segment::Arc {
                radius: radius()?,
                angle: a,
            }
        } else if let Some(a) = s.euler_rad {
            unused("offset", s.offset_um.is_some())?;
            Segment::Euler {
                radius: radius()?,
                angle: a,
                p: s.p.unwrap_or(1.0),
            }
        } else {
            unused("radius", s.radius_um.is_some())?;
            unused("p", s.p.is_some())?;
            Segment::Sine {
                length: Length::um(s.sine_um.unwrap_or_default()),
                offset: Length::um(
                    s.offset_um
                        .ok_or_else(|| at("a sine S-bend needs its offset"))?,
                ),
            }
        };
        b = b.segment(segment);
        if let Some(w) = s.width_um {
            b = b.taper(Length::um(w));
        }
    }
    b.build().map_err(|e| at(e.to_string()))
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct StructureTask {
    #[allow(dead_code)] // looked up by the registry (job::kind); kept for serde and the schema
    kind: String,
    stack: StackField,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    pub(super) wavelength_um: f64,
    pub(super) layer: String,
    x_um: [f64; 2],
    y_um: [f64; 2],
    pub(super) z_um: Option<[f64; 2]>,
    side_y_um: Option<f64>,
    pub(super) step_nm: f64,
    #[serde(default)]
    rect: Vec<RectSpec>,
    #[serde(default)]
    circle: Vec<CircleSpec>,
    #[serde(default)]
    ring: Vec<RingSpec>,
    #[serde(default)]
    pub(super) path: Vec<PathSpec>,
    #[serde(default)]
    pub(super) shape: Vec<shapes::ShapeSpec>,
    #[serde(default)]
    #[schemars(with = "Vec<crate::job::schema::Component>")]
    component: Vec<toml::Table>,
    #[serde(default)]
    #[schemars(with = "Vec<crate::job::schema::Body>")]
    body: Vec<toml::Table>,
}

impl StructureTask {
    /// What [`check`] and the run both refuse before drawing anything: the windows and the step.
    pub(super) fn validate(&self) -> Result<()> {
        check_window("x_um", self.x_um)?;
        check_window("y_um", self.y_um)?;
        if let Some(z) = self.z_um {
            check_window("z_um", z)?;
        }
        check_finite("side_y_um", self.side_y_um)?;
        // the side view is cut inside the window, or it shows what isn't drawn
        let side = self.side_y_um.unwrap_or(0.0);
        if !(self.y_um[0]..=self.y_um[1]).contains(&side) {
            return Err(task_error(format!(
                "the side view's cut, side_y_um = {side}{}, is outside y_um [{}, {}]: give a \
                 side_y_um inside it",
                if self.side_y_um.is_some() {
                    ""
                } else {
                    " by default"
                },
                self.y_um[0],
                self.y_um[1]
            )));
        }
        check_step(self.step_nm)?;
        check_cells("x_um", self.x_um, self.step_nm)?;
        check_cells("y_um", self.y_um, self.step_nm)?;
        match self.z_um {
            Some(z) => check_cells("z_um", z, self.step_nm),
            None => Ok(()),
        }
    }
}

/// A 2D job's etches (partial etches, sloped walls) must be on its own layer, the one whose
/// columns the effective index method solves; refused elsewhere, as the run would refuse them.
pub(super) fn check_etches_2d(s: &Structure, layer: &str) -> Result<()> {
    match s
        .stack()
        .layers()
        .iter()
        .find(|l| l.name != layer && !s.is_plain(&l.name))
    {
        Some(other) => Err(task_error(format!(
            "a 2D job's etches are on its own layer, {layer}: {} has a partial etch or a sloped \
             wall",
            other.name
        ))),
        None => Ok(()),
    }
}

/// A task's stack: a named one, `soi_220`, or `soi` / `nitride` with their core and bottom
/// oxide; or a `[task.stack]` table of layers ([`crate::device::spec::task_stack`]), whose
/// crystals may be oriented ([`crate::material::Orientation`]) only in a job whose solver takes
/// them ([`TENSOR_JOBS`]): this one refuses them, for a static job; [`tensor_stack`] and
/// [`diagonal_stack`] take them.
pub(super) fn named_stack(
    stack: &StackField,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
) -> Result<LayerStack> {
    let stack = named_stack_of(stack, core_nm, bottom_oxide_um)?;
    if let Some((place, m)) = stack_media(&stack)
        .into_iter()
        .find(|(_, m)| m.is_anisotropic())
    {
        return Err(task_error(format!(
            "[task.stack]: {place}'s {} is an oriented crystal, whose tensor only a {TENSOR_JOBS} \
             job takes; this job's solver takes one permittivity per cell: give an isotropic \
             material",
            m.name()
        )));
    }
    Ok(stack)
}

/// The jobs whose stacks may hold oriented crystals.
const TENSOR_JOBS: &str = "modes, structure, fdfd or fdtd";

/// [`named_stack`] for an FDFD or FDTD job, whose oriented crystals each give E's components
/// their own permittivity, the diagonal of the tensor in the chip's frame
/// ([`crate::material::Material::diagonal_permittivity`]): an orientation whose tensor couples
/// them is refused, at each of `wavelengths_um`.
pub(super) fn diagonal_stack(
    stack: &StackField,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
    wavelengths_um: &[f64],
) -> Result<LayerStack> {
    let stack = named_stack_of(stack, core_nm, bottom_oxide_um)?;
    for (place, m) in stack_media(&stack) {
        if !m.is_anisotropic() {
            continue;
        }
        for &w in wavelengths_um {
            let Ok(w) = Wavelength::um(w) else { continue };
            // (a wavelength without data is the materials' check's to report)
            if let Err(Error::InvalidValue { .. }) = m.diagonal_permittivity(w) {
                return Err(task_error(format!(
                    "[task.stack]: {place}'s {} is oriented so that its tensor couples E's \
                         components (its principal axes aren't along the chip's x, y and z); an \
                         fdfd or fdtd job gives each component of E its own permittivity, the \
                         tensor's diagonal: orient it by a cut, or by Euler angles in quarter \
                         turns",
                    m.name()
                )));
            }
        }
    }
    Ok(stack)
}

/// A stack's media and where each is: the substrate, the cladding, each layer's two.
pub(super) fn stack_media(stack: &LayerStack) -> Vec<(&str, &crate::material::Material)> {
    let mut media = vec![
        ("the substrate", stack.substrate()),
        ("the cladding", stack.cladding()),
    ];
    for l in stack.layers() {
        media.push((l.name.as_str(), &l.material));
        media.push((l.name.as_str(), &l.background));
    }
    media
}

/// [`named_stack`] with its crystals oriented, for a modes or a structure job.
pub(super) fn tensor_stack(
    stack: &StackField,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
) -> Result<LayerStack> {
    named_stack_of(stack, core_nm, bottom_oxide_um)
}

fn named_stack_of(
    stack: &StackField,
    core_nm: Option<f64>,
    bottom_oxide_um: Option<f64>,
) -> Result<LayerStack> {
    let stack = match stack {
        StackField::Name(name) => name.as_str(),
        StackField::Layers(table) => {
            if core_nm.is_some() || bottom_oxide_um.is_some() {
                return Err(task_error(
                    "core_nm and bottom_oxide_um are a named stack's: a [task.stack] table gives \
                     each layer's thickness",
                ));
            }
            return crate::device::spec::task_stack(table).map_err(|e| match e {
                Error::InvalidValue { reason, .. } => task_error(format!("[task.stack]: {reason}")),
                other => other,
            });
        }
    };
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
pub(super) fn draw(
    stack: LayerStack,
    rects: &[RectSpec],
    circles: &[CircleSpec],
    rings: &[RingSpec],
) -> Result<Structure> {
    let mut s = Structure::new(stack);
    // a shape without an etch is drawn as it always was
    let mut put = |layer: &str, shape: Shape, etch: Option<EtchSpec>, what: String| -> Result<()> {
        match etch {
            None => s.draw(layer, shape).map(|_| ()),
            Some(e) => s
                .draw_etched(layer, shape, EtchSpec::etch(Some(e)))
                .map(|_| ())
                .map_err(|e| match e {
                    Error::InvalidValue { reason, .. } => task_error(format!("{what}: {reason}")),
                    other => other,
                }),
        }
    };
    for (k, r) in rects.iter().enumerate() {
        let shape = Shape::rect(
            Point::um(r.center_um[0], r.center_um[1]),
            Length::um(r.size_um[0]),
            Length::um(r.size_um[1]),
        )?;
        put(&r.layer, shape, r.etch, format!("[[task.rect]] #{}", k + 1))?;
    }
    for (k, c) in circles.iter().enumerate() {
        let shape = Shape::circle(
            Point::um(c.center_um[0], c.center_um[1]),
            Length::um(c.radius_um),
        )?;
        put(
            &c.layer,
            shape,
            c.etch,
            format!("[[task.circle]] #{}", k + 1),
        )?;
    }
    for (k, r) in rings.iter().enumerate() {
        let shape = Shape::ring(
            Point::um(r.center_um[0], r.center_um[1]),
            Length::um(r.radius_um),
            Length::um(r.width_um),
        )?;
        put(&r.layer, shape, r.etch, format!("[[task.ring]] #{}", k + 1))?;
    }
    Ok(s)
}

/// `s` with `paths` drawn on it, after its other shapes, each as its simplest exact shape
/// ([`crate::geometry::Path::shape`]), then `shapes`, the shapes defined by functions and masks.
pub(super) fn draw_paths(
    mut s: Structure,
    paths: &[PathSpec],
    shapes: &[shapes::ShapeSpec],
    components: &[toml::Table],
) -> Result<Structure> {
    for (k, p) in paths.iter().enumerate() {
        let shape = p.path(k)?.shape();
        match p.etch {
            None => s.draw(&p.layer, shape).map(|_| ())?,
            Some(e) => s
                .draw_etched(&p.layer, shape, EtchSpec::etch(Some(e)))
                .map(|_| ())
                .map_err(|e| match e {
                    Error::InvalidValue { reason, .. } => {
                        task_error(format!("[[task.path]] #{}: {reason}", k + 1))
                    }
                    other => other,
                })?,
        }
    }
    components::draw(shapes::draw(s, shapes)?, components)
}

impl StructureTask {
    pub(super) fn structure(&self) -> Result<Structure> {
        draw_paths(
            draw(
                tensor_stack(&self.stack, self.core_nm, self.bottom_oxide_um)?,
                &self.rect,
                &self.circle,
                &self.ring,
            )?,
            &self.path,
            &self.shape,
            &self.component,
        )
        .and_then(|s| bodies::place(s, &self.body))
    }
}

/// The `"structure"` kind: a layer stack with shapes drawn on it, recorded as pictures of its
/// permittivity.
pub(super) struct StructureKind;

impl TaskKind for StructureKind {
    fn name(&self) -> &'static str {
        "structure"
    }

    fn schema(&self, g: &mut SchemaGenerator) -> serde_json::Value {
        schema::task::<StructureTask>(g)
    }

    fn check(&self, job: &Job) -> Result<()> {
        let task: StructureTask = params::task(job)?;
        task.validate()?;
        let s = task.structure()?;
        window_z(task.z_um, &task.layer, &s, task.step_nm)?;
        check_layer(&s, &task.layer, &[task.wavelength_um])
    }

    fn preview(&self, job: &Job) -> Result<Event> {
        super::check(job)?;
        let task: StructureTask = params::task(job)?;
        structure_scene(&task, &task.structure()?)
    }

    fn execute(&self, job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
        solver_run(job, run, stop, self.name(), structure)
    }

    fn exports(&self, _: &Job) -> (&'static str, &'static [&'static str]) {
        ("a structure job", &[])
    }
}

pub(super) fn structure(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let task: StructureTask = params::task(job)?;
    task.validate()?;
    let s = task.structure()?;
    let lam = Wavelength::um(task.wavelength_um)?;
    run.record(&structure_scene(&task, &s)?)?;
    record_ports(job, run)?;
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
    let [z0, z1] = window_z(task.z_um, &task.layer, &s, task.step_nm)?;
    let z = (Length::um(z0), Length::um(z1));
    let side_y = Length::um(task.side_y_um.unwrap_or(0.0));
    let side = s.side_view(side_y, x, z, step, lam)?;
    run.record(&Event::Permittivity {
        view: format!("side view at y = {} um", side_y.to_um()),
        axes: ["x".into(), "z".into()],
        wavelength_um: lam.to_um(),
        raster: side,
    })
}

pub(super) fn structure_scene(task: &StructureTask, s: &Structure) -> Result<Event> {
    let z = window_z(task.z_um, &task.layer, s, task.step_nm)?;
    scene(
        s,
        task.x_um,
        task.y_um,
        z,
        Wavelength::um(task.wavelength_um)?,
    )
}

/// A job's window height: `z_um`, else 1 µm below and above its layer, of as many cells as
/// [`check_cells`] allows a window the job gives (a layer a metre thick is refused here, not
/// as a picture too large to draw).
pub(super) fn window_z(
    z_um: Option<[f64; 2]>,
    layer: &str,
    s: &Structure,
    step_nm: f64,
) -> Result<[f64; 2]> {
    if let Some(z) = z_um {
        // (checked with the job's other windows)
        return Ok(z);
    }
    let (_, bottom, top) = s
        .stack()
        .layer(layer)
        .ok_or_else(|| task_error(format!("the stack has no layer {layer}")))?;
    let z = [bottom.to_um() - 1.0, top.to_um() + 1.0];
    check_cells(
        "the height, 1 um below and above the layer as z_um isn't given,",
        z,
        step_nm,
    )?;
    Ok(z)
}
