//! Fabrication: the process flow of a design, suggested, checked and explained.
//!
//! A design is a stack of layers and the shapes drawn on them, each etched to a depth with a
//! sidewall angle; that is also what a cleanroom makes. This module reads a design and answers
//! how it would be made: which steps, in which order, why each one, what else could do it, and
//! what each process can achieve, with the paper or book that says so.
//!
//! - **The process library** ([`library`]): wafers, depositions, lithographies, pattern
//!   transfers and planarization, each with its capabilities ([`Capability`]): a range with its
//!   source, or "not specified" ([`Stated`]). The type makes a number without its source
//!   impossible to write. Only numbers from the papers and books read are in it; [`references`]
//!   lists them, each with its checked DOI or ISBN.
//! - **The physical design** ([`Physical`]): the structure and the film physically in each
//!   place, where the optical model draws another (a heater's metal drawn with the oxide's
//!   optics).
//! - **The flow generator** ([`generate`]): rules that read each layer with what lies below and
//!   above it, its narrowest line and gap ([`features`]), the materials and the thermal budgets
//!   ([`budget`]), and return the ordered steps ([`Flow`]) with their reasons, alternatives,
//!   capabilities and uncertainties.
//! - **The executor** ([`execute`]): each step's geometry operation ([`Operation`]) run on the
//!   library's own [`crate::stack::Structure`]. The round trip ([`round_trip`]) compares what
//!   the flow makes with the design: equal to the bit, with the same films, for a flow that
//!   makes the design.
//! - **Thermal oxidation** ([`oxidation`]): Deal and Grove's linear-parabolic law with their
//!   printed constants.
//!
//! A flow is one sound way to make a design from published processes, not a foundry's process
//! and not a sign-off. docs/methods/fabrication.md describes the engine, its rules and its
//! sources; a job runs it as a `fabrication` task beside a `[device]`.

mod assess;
mod execute;
mod flow;
mod generate;
mod glossary;
mod library;
pub mod measure;
pub mod oxidation;
mod physical;
pub mod rayleigh;
mod references;
pub mod rules;
mod stated;

pub(crate) mod checks;
pub(crate) mod rule_checks;
#[cfg(test)]
mod tests;

pub use assess::{
    Assessment, Check, Contribution, Dimension, Kind, Ranked, Ranking, Rule, Verdict, assess,
};
pub use execute::{Made, RoundTrip, execute, round_trip, stages};
pub use flow::{Alternative, Depth, Flow, FlowRecord, Laid, Operation, Step, StepRecord, Tone};
pub use generate::{Choices, Thermal, generate, thermal};
pub use glossary::{Term, glossary};
pub use library::{
    Budget, LIBRARY_VERSION, Mask, Platform, Process, Purpose, StepType, Stop, Use, budget,
    library, process,
};
pub use measure::{Features, features};
pub use physical::{Film, Physical, StandIn, film_name};
pub use references::{Reference, reference, references};
pub use stated::{Capability, Fact, Quantity, Source, Stated, Unit};

/// A flow for `design` as `choices` ask, and its round trip: what a run records.
///
/// # Errors
///
/// [`generate`]'s and [`round_trip`]'s.
pub fn fabricate(design: &Physical, choices: &Choices) -> crate::Result<FlowRecord> {
    let flow = generate(design, choices)?;
    let trip = round_trip(&flow, design)?;
    Ok(flow.record(Some(trip)))
}
