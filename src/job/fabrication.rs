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
//! ```toml
//! [device]
//! kind = "TFLN Mach-Zehnder modulator"
//!
//! [task]
//! kind = "fabrication"
//! prefer = ["ebl-hsq"]     # process ids to prefer where several qualify
//! avoid = ["bosch-si"]     # process ids not to use
//! ```

use schemars::{JsonSchema, SchemaGenerator};
use serde::Deserialize;

use super::device::{computed, spec_of};
use super::kind::TaskKind;
use super::{Event, task_error};
use crate::Result;
use crate::fabrication::{Choices, FlowRecord, fabricate};
use crate::run::{Job, Run, Stop};

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

/// The flow of `job`'s design, as its task asks, with its round trip.
fn flow_of(job: &Job) -> Result<FlowRecord> {
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
    let (device, design) = spec_of(job)?.build()?;
    let physical = device.physical(design.values())?;
    fabricate(&physical, &choices).map_err(|e| task_error(e.to_string()))
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
            let flow = flow_of(job)?;
            run.record(&Event::Flow {
                flow: Box::new(flow),
            })
        })
    }

    fn farms(&self, _: &Job) -> bool {
        false
    }

    fn exports(&self, _: &Job) -> (&'static str, &'static [&'static str]) {
        ("a fabrication", &[])
    }
}
