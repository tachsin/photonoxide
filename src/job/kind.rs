//! The kinds of task, as one trait, and the registry of them.
//!
//! A job's `[task]` names its kind (`kind = "fdfd"`), and what the task sits beside decides
//! which kinds there are ([`Holder`]): a task alone, a task beside a `[device]`, or one beside a
//! `[circuit]`. Each kind implements [`TaskKind`] in its own file, and [`kinds`] lists them:
//! checking, previewing, running, farming, the export's fields and the schema look a kind up
//! there instead of matching on its name. A new kind is one file and one line in [`kinds`].
//!
//! The kind is looked up by its name, and its own struct then reads the table. That is why a
//! task isn't one internally tagged enum (`#[serde(tag = "kind")]`): such an enum can't refuse
//! the fields it doesn't know (`deny_unknown_fields`), which each kind's struct does, by name.
//! The struct keeps its `kind` field for serde, which then accepts the key, and for the schema.

use schemars::SchemaGenerator;
use serde_json::Value;

use super::check::task_error;
use super::chip::ChipStudyKind;
use super::device::SpectrumKind;
use super::event::Event;
use super::fabrication::FabricationKind;
use super::fdfd::FdfdKind;
use super::fdtd::FdtdKind;
use super::fit::FitKind;
use super::modes::ModesKind;
use super::optimize::crosstalk::CrosstalkKind;
use super::optimize::{ChipOptimizeKind, DeviceOptimizeKind};
use super::statics::StaticKind;
use super::structure::StructureKind;
use super::study::StudyKind;
use crate::run::{Job, Run, Stop};
use crate::{Error, Result};

/// What a job's task sits beside, which decides the kinds it may be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Holder {
    /// A task alone, which draws or reads its own structure.
    Solver,
    /// A task beside a `[device]`, computing it.
    Device,
    /// A task beside a `[circuit]`, computing the chip.
    Chip,
}

impl Holder {
    /// What `job`'s task sits beside: a `[circuit]` before a `[device]`, a job holding both
    /// being a chip's to refuse.
    pub(super) fn of(job: &Job) -> Holder {
        if job.circuit().is_some() {
            Holder::Chip
        } else if job.device().is_some() {
            Holder::Device
        } else {
            Holder::Solver
        }
    }
}

/// A kind of task: its name, its task table's schema, and how a job of it is checked, drawn,
/// run and farmed. Each kind reads its own table, a struct with `deny_unknown_fields`, in its
/// own file.
pub(super) trait TaskKind: Sync {
    /// The name a task's `kind` gives it, e.g. `"fdfd"`.
    fn name(&self) -> &'static str;

    /// The schema of its task table, as `docs/job.schema.json` holds it: a reference to its
    /// struct's.
    fn schema(&self, g: &mut SchemaGenerator) -> Value;

    /// Checks a job of this kind without running it: [`super::check`] once the holder's own
    /// checks (the direct solver's) have passed.
    fn check(&self, job: &Job) -> Result<()>;

    /// The scene a run of this kind records first, without running it ([`super::preview`]): the
    /// job checked first as [`super::check`] checks it, but for a `[device]`'s computations,
    /// whose design is drawn as it is, while it is edited.
    fn preview(&self, job: &Job) -> Result<Event>;

    /// Runs a job of this kind into `run`, from its start to its end: [`super::execute`] once
    /// its `[task.export]` has been checked.
    fn execute(&self, job: &Job, run: &mut Run, stop: &Stop) -> Result<()>;

    /// Whether a job of this kind has points a farm's workers can solve ([`super::farms`]): by
    /// default, when it has a `[task.sweep]`.
    fn farms(&self, job: &Job) -> bool {
        job.task().contains_key("sweep")
    }

    /// What a job of this kind computes that `[task.export]` may write as fields, and what to
    /// call it in an error.
    fn exports(&self, job: &Job) -> (&'static str, &'static [&'static str]);
}

/// The fields of a job that computes the electromagnetic field, which `[task.export]` may ask
/// for.
pub(super) const EM_FIELDS: &[&str] = &["Ex", "Ey", "Ez", "Hx", "Hy", "Hz"];

/// The kinds of a task alone, in the order the schema lists them.
static SOLVER: [&dyn TaskKind; 6] = [
    &StructureKind,
    &ModesKind,
    &FdfdKind,
    &FdtdKind,
    &StaticKind,
    &CrosstalkKind,
];

/// The kinds of a task beside a `[device]`, in the order the schema lists them.
static DEVICE: [&dyn TaskKind; 7] = [
    &SpectrumKind,
    &StudyKind("sweep"),
    &StudyKind("corners"),
    &FitKind,
    &DeviceOptimizeKind,
    &FdtdKind,
    &FabricationKind,
];

/// The kinds of a task beside a `[circuit]`, in the order the schema lists them.
static CHIP: [&dyn TaskKind; 4] = [
    &ChipStudyKind("spectrum"),
    &ChipStudyKind("sweep"),
    &ChipStudyKind("corners"),
    &ChipOptimizeKind,
];

/// The registry: every kind a task beside `holder` may be.
pub(super) fn kinds(holder: Holder) -> &'static [&'static dyn TaskKind] {
    match holder {
        Holder::Solver => &SOLVER,
        Holder::Device => &DEVICE,
        Holder::Chip => &CHIP,
    }
}

/// The name of the kind a task alone gives, which it must give.
pub(super) fn named(job: &Job) -> Result<&str> {
    job.task()
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| task_error("needs a kind, e.g. kind = \"structure\""))
}

/// What a task alone of a kind the registry hasn't is refused with.
pub(super) fn unknown(name: &str) -> Error {
    task_error(format!("unknown kind \"{name}\""))
}

/// The kind named `name` of a task beside `holder`, if there is one.
pub(super) fn find(holder: Holder, name: &str) -> Option<&'static dyn TaskKind> {
    kinds(holder).iter().copied().find(|k| k.name() == name)
}
