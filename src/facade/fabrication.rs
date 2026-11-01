//! A device's process flow ([`crate::fabrication`]) as plain data: [`device_fabrication`] (the
//! flow, what the rules find in it, and its export as JSON, a Markdown traveller and an SVG of
//! each step's cross-section), [`device_fabrication_job`] and [`export_fabrication`]; and each
//! with the flow's start ([`device_fabrication_from`], [`device_fabrication_job_from`],
//! [`export_fabrication_from`]): an SOI design from bare silicon, by Smart Cut.

use std::path::{Path, PathBuf};

use crate::export::flow::FlowExport;
use crate::fabrication::{Choices, Start};
use crate::run::Format;
use crate::{Error, Result};

use super::DeviceDesign;

/// A design's process flow, and its export.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DeviceFabrication {
    /// The flow as JSON, exactly as a `"fabrication"` job's run records it in its `flow` event
    /// ([`crate::fabrication::FlowRecord`]): its steps in order, each with its process, what it
    /// does, its reason, its alternatives, the capabilities it relies on with their sources and
    /// its uncertainties; the warnings, what is not specified, the library's version, the round
    /// trip, the references and the glossary.
    pub flow: String,
    /// What the rules find, as JSON, exactly as the run's `flow_assessment` event records it
    /// ([`crate::fabrication::Assessment`]): the checks, each step's processes ranked, and the
    /// accuracy of each fabricated dimension with its contributions and their sources.
    pub assessment: String,
    /// The export's JSON, as a run's `flow.json` holds it ([`crate::export::flow::FlowFile`]): the
    /// device and its parameters, the cut, the flow, the assessment and the wafer's layers after
    /// each step, from the top down.
    pub json: String,
    /// The process traveller, as a run's `flow.md` holds it.
    pub markdown: String,
    /// Each step's cross-section, a standalone SVG, as a run's `flow-01.svg`, ... hold them.
    pub svgs: Vec<String>,
}

fn choices(prefer: &[String], avoid: &[String]) -> Choices {
    Choices {
        prefer: prefer.to_vec(),
        avoid: avoid.to_vec(),
    }
}

fn export_of(
    design: &DeviceDesign,
    prefer: &[String],
    avoid: &[String],
    start: &str,
) -> Result<FlowExport> {
    FlowExport::of_design_from(
        &design.spec()?,
        &choices(prefer, avoid),
        Start::parse(start)?,
    )
}

/// The process flow of `design` (its kind, values and stack; its fidelity is not used),
/// preferring the processes `prefer` names by id where several qualify and avoiding those
/// `avoid` names: what a `"fabrication"` job's run records for the same design, to the bit, and
/// the files it writes, as text.
///
/// # Errors
///
/// [`Error::InvalidValue`] for an unknown kind, parameter or stack, a value outside its
/// parameter's range, a process the library hasn't, one both preferred and avoided, a
/// preference that makes no step, and a design no process in hand makes (a feature below every
/// cited lithography's, a film no process deposits).
pub fn device_fabrication(
    design: &DeviceDesign,
    prefer: &[String],
    avoid: &[String],
) -> Result<DeviceFabrication> {
    device_fabrication_from(design, prefer, avoid, "bought")
}

/// The process flow [`device_fabrication`] gives, starting from `start`: `"bought"`, the wafer
/// as bought (what [`device_fabrication`] does), or, for silicon on insulator, `"smart_cut"`,
/// bare silicon wafers with the Smart Cut steps that make the SOI wafer first
/// ([`crate::fabrication::Start`]).
///
/// # Errors
///
/// [`device_fabrication`]'s, an unknown start, and a Smart Cut start for a design whose wafer
/// isn't silicon on insulator.
pub fn device_fabrication_from(
    design: &DeviceDesign,
    prefer: &[String],
    avoid: &[String],
    start: &str,
) -> Result<DeviceFabrication> {
    let export = export_of(design, prefer, avoid, start)?;
    Ok(DeviceFabrication {
        flow: line(&export.file.flow),
        assessment: line(&export.file.assessment),
        json: export.file.json(),
        markdown: export.file.markdown(),
        svgs: export.svgs(),
    })
}

/// A record as one line of JSON, as a run's `events.jsonl` writes it.
fn line<T: serde::Serialize>(v: &T) -> String {
    // plain data: strings, numbers, lists and records, which always serialize
    serde_json::to_string(v).expect("plain data always serializes")
}

/// The job file that gives `design`'s process flow, named `name`, as text in `format`
/// (`"yaml"`, the format we recommend, `"toml"` or `"json"`): a `device` table holding the design
/// and a `"fabrication"` task with `prefer` and `avoid` ([`crate::job::from_fabrication`]).
/// [`super::run_job`] runs it; its run records what [`device_fabrication`] gives, to the bit, and
/// writes `flow.json`, `flow.md` and the cross-sections into its folder.
///
/// # Errors
///
/// [`Error::InvalidValue`] for an unknown format, a name that isn't a job's, and
/// [`device_fabrication`]'s.
pub fn device_fabrication_job(
    design: &DeviceDesign,
    name: &str,
    format: &str,
    prefer: &[String],
    avoid: &[String],
) -> Result<String> {
    device_fabrication_job_from(design, name, format, prefer, avoid, "bought")
}

/// The job [`device_fabrication_job`] writes, its flow starting from `start` (as
/// [`device_fabrication_from`] takes it): the task's `start`, written when it isn't
/// `"bought"`. Its run records what [`device_fabrication_from`] gives, to the bit.
///
/// # Errors
///
/// [`device_fabrication_job`]'s and [`device_fabrication_from`]'s.
pub fn device_fabrication_job_from(
    design: &DeviceDesign,
    name: &str,
    format: &str,
    prefer: &[String],
    avoid: &[String],
    start: &str,
) -> Result<String> {
    let format = match format.to_ascii_lowercase().as_str() {
        "toml" => Format::Toml,
        "json" => Format::Json,
        "yaml" | "yml" => Format::Yaml,
        _ => {
            return Err(Error::invalid(
                "job format",
                format!("is \"toml\", \"json\" or \"yaml\", not {format:?}"),
            ));
        }
    };
    crate::job::from_fabrication_from(name, &design.spec()?, prefer, avoid, Start::parse(start)?)?
        .to_text(format)
}

/// Writes `design`'s process flow ([`device_fabrication`]) into `out_dir`, made if it isn't
/// there: `flow.json`, `flow.md` and each step's cross-section, `flow-01.svg`, ..., as a
/// `"fabrication"` job's run writes them into its folder; returns the files written, in that
/// order.
///
/// # Errors
///
/// [`device_fabrication`]'s, and [`Error::Io`] if the folder can't be made or a file written.
pub fn export_fabrication(
    design: &DeviceDesign,
    prefer: &[String],
    avoid: &[String],
    out_dir: &Path,
) -> Result<Vec<PathBuf>> {
    export_fabrication_from(design, prefer, avoid, out_dir, "bought")
}

/// Writes the process flow [`device_fabrication_from`] gives, starting from `start`, as
/// [`export_fabrication`] writes it.
///
/// # Errors
///
/// [`export_fabrication`]'s and [`device_fabrication_from`]'s.
pub fn export_fabrication_from(
    design: &DeviceDesign,
    prefer: &[String],
    avoid: &[String],
    out_dir: &Path,
    start: &str,
) -> Result<Vec<PathBuf>> {
    export_of(design, prefer, avoid, start)?.write(out_dir)
}
