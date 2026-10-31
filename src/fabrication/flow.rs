//! A flow: the steps that make a design, in order, each with its process, its geometry
//! operation, its reason, its alternatives, the capabilities it relies on and its
//! uncertainties; and a flow's record, which a run keeps.

use serde::{Deserialize, Serialize};

use crate::geometry::Shape;
use crate::units::Length;

use super::library::StepType;
use super::physical::Film;
use super::stated::{Capability, Fact, number};

/// A layer laid down by a step: its name and thickness.
#[derive(Clone, Debug, PartialEq)]
pub struct Laid {
    /// The layer's name, as the design's stack has it.
    pub name: String,
    /// Its thickness.
    pub thickness: Length,
}

/// How deep an etch goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Depth {
    /// Through the layer, stopping on the film below.
    ToStop,
    /// For a time that takes it this deep.
    Timed(Length),
}

/// Where the resist stays after it is developed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    /// On the drawn shapes: an etch mask.
    Keep,
    /// Everywhere but the drawn shapes, which are opened: a lift-off stencil.
    Open,
}

/// What a step does to the wafer's geometry.
#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    /// The wafer as bought: its handle and its layers, bottom to top, none patterned.
    Wafer {
        /// The handle wafer, below everything.
        substrate: Film,
        /// Its layers, bottom to top, with their films.
        layers: Vec<(Laid, Film)>,
    },
    /// A film over the whole wafer: it fills every region etched open so far, then lies in
    /// `layers` (bottom to top) and, if `cladding`, above everything as the cladding; `extra`
    /// more of it is deposited, for a polish to take off.
    Deposit {
        /// The film.
        film: Film,
        /// The layers it makes.
        layers: Vec<Laid>,
        /// Whether it is the cladding above everything.
        cladding: bool,
        /// The thickness deposited beyond them, which a polish removes.
        extra: Length,
    },
    /// The surface polished flat, `removed` taken off the top.
    Planarize {
        /// What the polish takes off.
        removed: Length,
    },
    /// Resist coated, exposed and developed: it stays on the shapes (`Tone::Keep`) or opens
    /// them (`Tone::Open`).
    Pattern {
        /// The resist's id.
        resist: String,
        /// Its thickness, when a source states it.
        thickness: Option<Length>,
        /// The design's shapes.
        mask: Vec<Shape>,
        /// Where the resist stays.
        tone: Tone,
    },
    /// The layer `layer` etched where the resist leaves it bare.
    Etch {
        /// The layer.
        layer: String,
        /// How deep.
        depth: Depth,
        /// The walls' angle from the plane, radians.
        sidewall: f64,
    },
    /// A film evaporated over the resist and into its openings.
    Evaporate {
        /// The film.
        film: Film,
        /// Its thickness.
        thickness: Length,
    },
    /// The resist dissolved, taking the film on it: what landed in the openings is the layer
    /// `layer`.
    LiftOff {
        /// The layer.
        layer: String,
    },
    /// The resist removed.
    Strip,
    /// Something the design doesn't draw, listed and not executed: e.g. a heater's contacts.
    Undrawn {
        /// What, in plain words.
        what: String,
    },
}

impl Operation {
    /// What it does, in a short phrase.
    pub fn describe(&self) -> String {
        let um = |l: Length| format!("{} µm", number(l.to_um()));
        match self {
            Operation::Wafer { layers, .. } => format!(
                "the wafer: {}",
                layers
                    .iter()
                    .rev()
                    .map(|(l, f)| format!("{} {} ({})", um(l.thickness), f.name, l.name))
                    .collect::<Vec<_>>()
                    .join(" on ")
            ),
            Operation::Deposit {
                film,
                layers,
                cladding,
                extra,
            } => {
                let mut parts: Vec<String> = layers
                    .iter()
                    .map(|l| format!("{} as {}", um(l.thickness), l.name))
                    .collect();
                if *cladding {
                    parts.push("the cladding".into());
                }
                let mut s = format!("deposit {}", film.name);
                if !parts.is_empty() {
                    s.push_str(&format!(": {}", parts.join(", then ")));
                }
                if extra.to_um() > 0.0 {
                    s.push_str(&format!(", {} more for the polish", um(*extra)));
                }
                s
            }
            Operation::Planarize { removed } => format!("polish {} off, flat", um(*removed)),
            Operation::Pattern {
                resist, tone, mask, ..
            } => format!(
                "{resist} {} on {} shape{}",
                match tone {
                    Tone::Keep => "kept",
                    Tone::Open => "opened",
                },
                mask.len(),
                if mask.len() == 1 { "" } else { "s" }
            ),
            Operation::Etch {
                layer,
                depth,
                sidewall,
            } => format!(
                "etch {layer} {} at {}°",
                match depth {
                    Depth::ToStop => "through, to the film below".into(),
                    Depth::Timed(d) => format!("{} deep, timed", um(*d)),
                },
                number(sidewall.to_degrees())
            ),
            Operation::Evaporate { film, thickness } => {
                format!("evaporate {} of {}", um(*thickness), film.name)
            }
            Operation::LiftOff { layer } => format!("lift off: {layer} left in the openings"),
            Operation::Strip => "remove the resist".into(),
            Operation::Undrawn { what } => format!("not drawn by the design: {what}"),
        }
    }

    /// Its kind, as the record names it.
    pub fn kind(&self) -> &'static str {
        match self {
            Operation::Wafer { .. } => "wafer",
            Operation::Deposit { .. } => "deposit",
            Operation::Planarize { .. } => "planarize",
            Operation::Pattern { .. } => "pattern",
            Operation::Etch { .. } => "etch",
            Operation::Evaporate { .. } => "evaporate",
            Operation::LiftOff { .. } => "lift_off",
            Operation::Strip => "strip",
            Operation::Undrawn { .. } => "undrawn",
        }
    }
}

/// A process that could have done a step, and why it wasn't chosen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Alternative {
    /// The process's id.
    pub process: String,
    /// Its name.
    pub name: String,
    /// Why it wasn't chosen, in plain words.
    pub why_not: String,
}

/// One step of a flow.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    /// The process's id.
    pub process: String,
    /// The process's name.
    pub name: String,
    /// Its step type.
    pub step: StepType,
    /// What the process does, for someone new to it.
    pub plain: String,
    /// What it does to the wafer.
    pub operation: Operation,
    /// Why this process, for this design, in plain words.
    pub reason: String,
    /// What else could do it, and why not.
    pub alternatives: Vec<Alternative>,
    /// The capabilities it relies on, each with its source or not specified.
    pub capabilities: Vec<Capability>,
    /// What its sources say that matters here.
    pub facts: Vec<Fact>,
    /// What isn't certain about it.
    pub uncertainties: Vec<String>,
}

/// The steps that make a design, in order, with the warnings and what was not specified.
#[derive(Clone, Debug, PartialEq)]
pub struct Flow {
    /// The steps.
    pub steps: Vec<Step>,
    /// What the rules found that the design or the library can't settle: a sidewall outside
    /// every cited range, a budget not shown.
    pub warnings: Vec<String>,
    /// What no source in hand states, step by step.
    pub not_specified: Vec<String>,
}

/// A step as a run's record keeps it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StepRecord {
    /// The process's id.
    pub process: String,
    /// Its name.
    pub name: String,
    /// Its step type.
    pub step: StepType,
    /// What it does, plainly.
    pub plain: String,
    /// The operation's kind: `"wafer"`, `"deposit"`, `"planarize"`, `"pattern"`, `"etch"`,
    /// `"evaporate"`, `"lift_off"`, `"strip"` or `"undrawn"`.
    pub operation: String,
    /// The operation, in a short phrase.
    pub does: String,
    /// The layer it makes or acts on, if one.
    pub layer: Option<String>,
    /// The film it puts down, if one.
    pub film: Option<String>,
    /// The thickness it puts down, µm.
    pub thickness_um: Option<f64>,
    /// An etch's depth, µm, and whether it is timed (or stops on the film below).
    pub depth_um: Option<f64>,
    /// Whether an etch is timed.
    pub timed: bool,
    /// An etch's sidewall, degrees from the plane.
    pub sidewall_deg: Option<f64>,
    /// A pattern's tone.
    pub tone: Option<Tone>,
    /// Why this process.
    pub reason: String,
    /// What else, and why not.
    pub alternatives: Vec<Alternative>,
    /// The capabilities relied on.
    pub capabilities: Vec<Capability>,
    /// What the sources say.
    pub facts: Vec<Fact>,
    /// What isn't certain.
    pub uncertainties: Vec<String>,
}

/// A flow as a run's record keeps it: its steps, warnings, what was not specified, the
/// library's version, and the round trip's result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlowRecord {
    /// The library's version ([`super::LIBRARY_VERSION`]).
    pub library: String,
    /// The steps.
    pub steps: Vec<StepRecord>,
    /// The warnings.
    pub warnings: Vec<String>,
    /// What was not specified.
    pub not_specified: Vec<String>,
    /// The round trip: whether the executed flow gives the design's structure and films, and
    /// how many of the points compared differ.
    pub round_trip: Option<super::execute::RoundTrip>,
    /// Every reference the flow cites, by key, with its citation and DOI or ISBN: the steps'
    /// and the glossary's.
    pub references: Vec<super::references::Reference>,
    /// The words the narration uses, each with its plain definition and source.
    pub glossary: Vec<super::glossary::Term>,
}

impl Flow {
    /// Its record, with the round trip's result.
    pub fn record(&self, round_trip: Option<super::execute::RoundTrip>) -> FlowRecord {
        let steps = self.steps.iter().map(Step::record).collect::<Vec<_>>();
        let mut keys: Vec<String> = Vec::new();
        for s in &self.steps {
            let sources = s
                .capabilities
                .iter()
                .filter_map(|c| c.value.source())
                .chain(s.facts.iter().map(|f| &f.source));
            for src in sources {
                if !keys.contains(&src.reference) {
                    keys.push(src.reference.clone());
                }
            }
        }
        let glossary = super::glossary::glossary();
        for t in &glossary {
            if !keys.contains(&t.source.reference) {
                keys.push(t.source.reference.clone());
            }
        }
        FlowRecord {
            library: super::LIBRARY_VERSION.into(),
            steps,
            warnings: self.warnings.clone(),
            not_specified: self.not_specified.clone(),
            round_trip,
            references: keys
                .iter()
                .filter_map(|k| super::references::reference(k))
                .collect(),
            glossary,
        }
    }
}

impl Step {
    /// Its record.
    pub fn record(&self) -> StepRecord {
        let mut r = StepRecord {
            process: self.process.clone(),
            name: self.name.clone(),
            step: self.step,
            plain: self.plain.clone(),
            operation: self.operation.kind().into(),
            does: self.operation.describe(),
            layer: None,
            film: None,
            thickness_um: None,
            depth_um: None,
            timed: false,
            sidewall_deg: None,
            tone: None,
            reason: self.reason.clone(),
            alternatives: self.alternatives.clone(),
            capabilities: self.capabilities.clone(),
            facts: self.facts.clone(),
            uncertainties: self.uncertainties.clone(),
        };
        match &self.operation {
            Operation::Wafer { layers, .. } => {
                r.layer = layers.last().map(|(l, _)| l.name.clone());
                r.film = layers.last().map(|(_, f)| f.name.clone());
                r.thickness_um = layers.last().map(|(l, _)| l.thickness.to_um());
            }
            Operation::Deposit {
                film,
                layers,
                extra,
                ..
            } => {
                r.film = Some(film.name.clone());
                r.layer = layers.first().map(|l| l.name.clone());
                let t: f64 = layers.iter().map(|l| l.thickness.to_um()).sum();
                r.thickness_um = Some(t + extra.to_um());
            }
            Operation::Planarize { removed } => r.thickness_um = Some(removed.to_um()),
            Operation::Pattern {
                resist,
                thickness,
                tone,
                ..
            } => {
                r.film = Some(resist.clone());
                r.thickness_um = thickness.map(Length::to_um);
                r.tone = Some(*tone);
            }
            Operation::Etch {
                layer,
                depth,
                sidewall,
            } => {
                r.layer = Some(layer.clone());
                r.sidewall_deg = Some(sidewall.to_degrees());
                match depth {
                    Depth::ToStop => {}
                    Depth::Timed(d) => {
                        r.depth_um = Some(d.to_um());
                        r.timed = true;
                    }
                }
            }
            Operation::Evaporate { film, thickness } => {
                r.film = Some(film.name.clone());
                r.thickness_um = Some(thickness.to_um());
            }
            Operation::LiftOff { layer } => r.layer = Some(layer.clone()),
            Operation::Strip | Operation::Undrawn { .. } => {}
        }
        r
    }
}
