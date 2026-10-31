//! A design's process flow: a job's `[device]` and a `[task]` of kind `"fabrication"`.
//!
//! The run records the design's scene, then the flow that would make it ([`Event::Flow`],
//! [`crate::fabrication`]): the steps in order, each with its process, what it does to the
//! wafer, why it was chosen and what else could do it, the capabilities it relies on with their
//! sources, and its uncertainties; the warnings, what no source in hand states, the process
//! library's version, and the round trip: the flow executed on the library's geometry against
//! the design, which must give it back to the bit. A flow is data, fully set by the design and
//! the library, so its record replays bit for bit.
//!
//! The run also writes the flow's export into its folder ([`crate::export::flow`]): `flow.json`
//! (the flow and the assessment as the record holds them, with the wafer's layers after each
//! step), `flow.md` (a process traveller) and `flow-01.svg`, ... (the cross-section after each
//! step).
//!
//! ```yaml
//! name: tfln-flow
//!
//! device:
//!   kind: TFLN Mach-Zehnder modulator
//!
//! task:
//!   kind: fabrication
//!   prefer: [ebl-hsq]     # process ids to prefer where several qualify
//!   avoid: [bosch-si]     # process ids not to use
//! ```

use schemars::{JsonSchema, SchemaGenerator};
use serde::{Deserialize, Serialize};

use super::device::{computed, spec_of};
use super::kind::TaskKind;
use super::scene::{SceneEtched, scene, scene_etched};
use super::{Event, task_error};
use crate::Result;
use crate::device::Spec;
use crate::export::flow::FlowExport;
use crate::fabrication::{
    Assessment, Choices, FlowRecord, Physical, assess, fabricate, generate, round_trip, stages,
};
use crate::run::{Job, Run, Stop};
use crate::units::Wavelength;

/// A `"fabrication"` task: the process flow of the `[device]`'s design.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct FabricationTask {
    #[allow(dead_code)] // looked up by the registry (job::kind); kept for serde and the schema
    kind: String,
    /// The ids of processes to prefer where several qualify, e.g. `"ebl-hsq"`.
    #[serde(default)]
    prefer: Vec<String>,
    /// The ids of processes not to use.
    #[serde(default)]
    avoid: Vec<String>,
}

/// What `job`'s task prefers and avoids, checked against the library.
fn choices_of(job: &Job) -> Result<Choices> {
    let task: FabricationTask = job
        .task()
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| task_error(e.to_string()))?;
    let choices = Choices {
        prefer: task.prefer,
        avoid: task.avoid,
    };
    choices.check().map_err(|e| task_error(e.to_string()))?;
    Ok(choices)
}

/// The flow of `job`'s design, as its task asks, with its round trip, and what the rules find
/// in it.
fn flow_of(job: &Job) -> Result<(FlowRecord, Assessment)> {
    let choices = choices_of(job)?;
    let (device, design) = spec_of(job)?.build()?;
    let physical = device.physical(design.values())?;
    let record = fabricate(&physical, &choices).map_err(|e| task_error(e.to_string()))?;
    let flow = generate(&physical, &choices)?;
    Ok((record, assess(&physical, &flow)?))
}

/// The `"fabrication"` kind beside a `[device]`: the design's process flow.
pub(super) struct FabricationKind;

impl TaskKind for FabricationKind {
    fn name(&self) -> &'static str {
        "fabrication"
    }

    fn schema(&self, g: &mut SchemaGenerator) -> serde_json::Value {
        super::schema::task::<FabricationTask>(g)
    }

    fn check(&self, job: &Job) -> Result<()> {
        super::device_scene(job)?;
        flow_of(job).map(|_| ())
    }

    fn preview(&self, job: &Job) -> Result<Event> {
        super::device_scene(job)
    }

    fn execute(&self, job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
        computed(job, run, stop, |job, run, _, scene| {
            run.record(&scene)?;
            let (flow, assessment) = flow_of(job)?;
            run.record(&Event::Flow {
                flow: Box::new(flow),
            })?;
            run.record(&Event::FlowAssessment {
                assessment: Box::new(assessment),
            })?;
            // the flow's JSON, traveller and cross-sections, beside the record
            if !run.dir().as_os_str().is_empty() {
                FlowExport::of_design(&spec_of(job)?, &choices_of(job)?)?.write(run.dir())?;
            }
            Ok(())
        })
    }

    fn farms(&self, _: &Job) -> bool {
        false
    }

    fn exports(&self, _: &Job) -> (&'static str, &'static [&'static str]) {
        ("a fabrication", &[])
    }
}

/// The job that gives `spec`'s design's process flow, named `name`: a `[device]` table holding
/// the design and a `[task]` of kind `"fabrication"`, preferring and avoiding processes by id
/// (each list written only when it isn't empty). It is checked by [`super::check`]; its run
/// records the flow [`FlowExport::of_design`] gives, to the bit, and writes its export.
///
/// # Errors
///
/// [`crate::Error::InvalidValue`] for a name that isn't a job's, a design [`Spec::build`]
/// refuses, and the errors of [`super::check`] for the job it makes (a process the library
/// hasn't, one both preferred and avoided, a preference that makes no step).
pub fn from_fabrication(
    name: &str,
    spec: &Spec,
    prefer: &[String],
    avoid: &[String],
) -> Result<Job> {
    use toml::{Table, Value};
    spec.build()?;
    let list =
        |ids: &[String]| Value::Array(ids.iter().map(|s| Value::String(s.clone())).collect());
    let mut task = Table::new();
    task.insert("kind".into(), "fabrication".into());
    if !prefer.is_empty() {
        task.insert("prefer".into(), list(prefer));
    }
    if !avoid.is_empty() {
        task.insert("avoid".into(), list(avoid));
    }
    let mut file = Table::new();
    file.insert("name".into(), name.into());
    file.insert("device".into(), Value::Table(spec.to_table()));
    file.insert("task".into(), Value::Table(task));
    let text = toml::to_string(&file).map_err(|e| task_error(e.to_string()))?;
    let job = Job::parse(&text)?;
    super::check(&job)?;
    Ok(job)
}

/// A design's flow and the wafer after each of its steps, as the studio's Fabrication tab shows
/// them.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FabricationView {
    /// The flow, with its round trip.
    pub flow: FlowRecord,
    /// The wafer after each step, in order.
    pub stages: Vec<Stage>,
    /// What the rules find: each rule's check, each step's alternatives ranked, and the
    /// accuracy of each fabricated dimension.
    pub assessment: Assessment,
}

/// The wafer after a step: its scene ([`Event::Scene`], air where nothing is yet), and how each
/// etched layer's shapes was etched ([`scene_etched`]), which a cross-section draws exactly.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Stage {
    /// The scene.
    pub scene: Event,
    /// The etched layers' shapes with their etches.
    pub etched: Vec<SceneEtched>,
}

/// The flow of `spec`'s design, preferring and avoiding processes by id, and the wafer after
/// each step, over the design's shapes with 1 µm around them, from 0.5 µm into the substrate to 1
/// µm above its top, at 1.55 µm.
///
/// # Errors
///
/// [`crate::fabrication::generate`]'s, and the design's.
pub fn fabrication_of(spec: &Spec, prefer: &[String], avoid: &[String]) -> Result<FabricationView> {
    let choices = Choices {
        prefer: prefer.to_vec(),
        avoid: avoid.to_vec(),
    };
    let (device, design) = spec.build()?;
    let physical: Physical = device.physical(design.values())?;
    let flow = generate(&physical, &choices)?;
    let trip = round_trip(&flow, &physical)?;
    let s = physical.structure();
    let (mut x, mut y) = (
        [f64::INFINITY, f64::NEG_INFINITY],
        [f64::INFINITY, f64::NEG_INFINITY],
    );
    for l in s.stack().layers() {
        for shape in s.shapes(&l.name) {
            let b = shape.bounds();
            x = [x[0].min(b.min.x.to_um()), x[1].max(b.max.x.to_um())];
            y = [y[0].min(b.min.y.to_um()), y[1].max(b.max.y.to_um())];
        }
    }
    if !x[0].is_finite() {
        (x, y) = ([0.0, 0.0], [0.0, 0.0]);
    }
    let z = [-0.5, s.stack().thickness().to_um() + 1.0];
    let wavelength = Wavelength::um(1.55)?;
    let stages = stages(&flow)?
        .iter()
        .map(|made| {
            let mut drawn = scene(
                &made.structure,
                [x[0] - 1.0, x[1] + 1.0],
                [y[0] - 1.0, y[1] + 1.0],
                z,
                wavelength,
            )?;
            named(&mut drawn, made);
            Ok(Stage {
                scene: drawn,
                etched: scene_etched(&made.structure),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(FabricationView {
        flow: flow.record(Some(trip)),
        stages,
        assessment: assess(&physical, &flow)?,
    })
}

/// A stage's scene with each medium named by the film that is there (the heater's TiN, not the
/// oxide the optical model draws in its place; air where nothing is yet), its permittivities the
/// optical ones. An etched layer's bands (`name 2`, `name 3`, …) take their layer's films.
fn named(scene: &mut Event, made: &crate::fabrication::Made) {
    let Event::Scene {
        substrate,
        cladding,
        layers,
        ..
    } = scene
    else {
        return;
    };
    substrate.material.clone_from(&made.substrate);
    cladding.material.clone_from(&made.cladding);
    let names: Vec<&str> = made
        .structure
        .stack()
        .layers()
        .iter()
        .map(|l| l.name.as_str())
        .collect();
    for l in layers {
        let base = match l.name.rsplit_once(' ') {
            Some((b, n)) if n.parse::<usize>().is_ok() && names.contains(&b) => b,
            _ => l.name.as_str(),
        };
        if let Some(k) = names.iter().position(|n| *n == base) {
            let (drawn, background) = &made.layers[k];
            let slab = !made.structure.is_plain(base)
                && l.material.material == l.background.material
                && drawn != background;
            l.material.material.clone_from(drawn);
            l.background.material = if slab {
                drawn.clone()
            } else {
                background.clone()
            };
        }
    }
}
