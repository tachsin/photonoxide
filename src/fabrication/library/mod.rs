//! The process library: every process the flow generator may choose, as data, one file per
//! step type, each capability a cited range or "not specified". Only numbers from papers and
//! books read are here (a foundry's numbers come from a PDK the user loads, not from the
//! library).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::stated::{Capability, Fact, Quantity, Source, Stated, Unit};

mod deposition;
mod lithography;
mod planarization;
mod transfer;
mod wafer;

/// The library's version, which a run's record names: it changes whenever a process, a
/// capability or a rule changes, so that an older run says which library it used.
pub const LIBRARY_VERSION: &str = "2";

/// What kind of step a process is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum StepType {
    /// A wafer bought with its first layers.
    Wafer,
    /// A film put down over the wafer.
    Deposition,
    /// A pattern written into a resist.
    Lithography,
    /// A pattern moved from the resist into a film: an etch, a lift-off, the resist's removal.
    PatternTransfer,
    /// The surface polished flat.
    Planarization,
}

impl StepType {
    /// Its name, as the narration writes it.
    pub fn name(self) -> &'static str {
        match self {
            StepType::Wafer => "wafer",
            StepType::Deposition => "deposition",
            StepType::Lithography => "lithography",
            StepType::PatternTransfer => "pattern transfer",
            StepType::Planarization => "planarization",
        }
    }
}

/// A platform: the film a design's guides are made of, on its wafer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Platform {
    /// Silicon on insulator.
    Soi,
    /// Lithium niobate on insulator.
    Lnoi,
}

/// What a process was used for in a source's device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Purpose {
    /// The optical guides.
    Guide,
    /// Metal: electrodes or heaters.
    Metal,
    /// The cladding over the guides.
    Cladding,
}

/// A source's device that used a process, on a platform, for a purpose.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Use {
    /// The platform.
    pub platform: Platform,
    /// What for.
    pub purpose: Purpose,
    /// The source that says so.
    pub source: Source,
}

/// A mask an etch was run with in a source: the resist, how thick, how much faster the film
/// etches than it, and how deep an etch it was shown to last.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mask {
    /// The resist's id: `"photoresist"`, `"hsq"`, `"pmma"`, `"chromium"`.
    pub resist: String,
    /// Its thickness.
    pub thickness: Stated,
    /// The film's etch rate over the mask's.
    pub selectivity: Stated,
    /// The deepest etch the source made with it.
    pub depth: Stated,
}

/// A film an etch stops on, with the source that says so.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stop {
    /// The film, e.g. `"SiO2"`.
    pub film: String,
    /// The source.
    pub source: Source,
}

/// A process: what it does, in plain words, what it acts on, and what it can achieve, each
/// with its source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Process {
    /// A short id, e.g. `"icp-si-hbr"`.
    pub id: String,
    /// Its step type.
    pub step: StepType,
    /// Its name.
    pub name: String,
    /// What it does, for someone new to it.
    pub plain: String,
    /// The films it deposits, etches or lifts off (none for a lithography); a wafer's films
    /// bottom to top over its handle.
    pub films: Vec<String>,
    /// A lithography's resist: its id.
    pub resist: Option<String>,
    /// Whether a deposition comes down in straight lines (evaporation), which lift-off needs.
    pub directional: bool,
    /// What it can achieve.
    pub capabilities: Vec<Capability>,
    /// What its sources say without a number.
    pub facts: Vec<Fact>,
    /// The films an etch stops on.
    pub stops_on: Vec<Stop>,
    /// The masks an etch was run with.
    pub masks: Vec<Mask>,
    /// The sources' devices that used it.
    pub used_for: Vec<Use>,
    /// Every reference it cites, by key.
    pub references: Vec<String>,
}

impl Process {
    /// Its capabilities of `quantity`.
    pub fn capabilities_of<'a>(
        &'a self,
        quantity: &'a Quantity,
    ) -> impl Iterator<Item = &'a Capability> + 'a {
        self.capabilities
            .iter()
            .filter(move |c| &c.quantity == quantity)
    }

    /// Whether a source used it on `platform` for `purpose`.
    pub fn used(&self, platform: Platform, purpose: Purpose) -> Option<&Use> {
        self.used_for
            .iter()
            .find(|u| u.platform == platform && u.purpose == purpose)
    }

    /// Whether it acts on `film`.
    pub fn acts_on(&self, film: &str) -> bool {
        self.films.iter().any(|f| f == film)
    }

    /// Its temperature: the highest any of its sources states, with that source.
    pub fn temperature(&self) -> Option<(f64, &Capability)> {
        self.capabilities_of(&Quantity::Temperature)
            .filter_map(|c| c.value.high().or(c.value.low()).map(|t| (t, c)))
            .fold(
                None,
                |best: Option<(f64, &Capability)>, (t, c)| match best {
                    Some((b, _)) if b >= t => best,
                    _ => Some((t, c)),
                },
            )
    }
}

pub(super) fn src(reference: &str, location: &str) -> Source {
    Source::new(reference, location)
}

pub(super) fn cap(quantity: Quantity, value: Stated, conditions: &str) -> Capability {
    Capability::new(quantity, value, conditions)
}

pub(super) fn fact(text: &str, reference: &str, location: &str) -> Fact {
    Fact::new(text, src(reference, location))
}

pub(super) fn used(platform: Platform, purpose: Purpose, reference: &str, location: &str) -> Use {
    Use {
        platform,
        purpose,
        source: src(reference, location),
    }
}

/// A process with nothing but its id, type, names and films: the files fill in the rest.
pub(super) fn new_process(
    id: &str,
    step: StepType,
    name: &str,
    plain: &str,
    films: &[&str],
) -> Process {
    Process {
        id: id.into(),
        step,
        name: name.into(),
        plain: plain.into(),
        films: films.iter().map(|f| (*f).into()).collect(),
        resist: None,
        directional: false,
        capabilities: Vec::new(),
        facts: Vec::new(),
        stops_on: Vec::new(),
        masks: Vec::new(),
        used_for: Vec::new(),
        references: Vec::new(),
    }
}

impl Process {
    /// Sets its references from every source it names, in the order first named.
    fn referenced(mut self) -> Process {
        let mut keys: Vec<String> = Vec::new();
        let mut add = |s: &Source| {
            if !keys.contains(&s.reference) {
                keys.push(s.reference.clone());
            }
        };
        for c in &self.capabilities {
            if let Some(s) = c.value.source() {
                add(s);
            }
        }
        for f in &self.facts {
            add(&f.source);
        }
        for s in &self.stops_on {
            add(&s.source);
        }
        for m in &self.masks {
            for v in [&m.thickness, &m.selectivity, &m.depth] {
                if let Some(s) = v.source() {
                    add(s);
                }
            }
        }
        for u in &self.used_for {
            add(&u.source);
        }
        self.references = keys;
        self
    }
}

/// Every process, in the library's order: the order that breaks ties.
pub fn library() -> &'static [Process] {
    static LIBRARY: OnceLock<Vec<Process>> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let mut all = Vec::new();
        all.extend(wafer::processes());
        all.extend(deposition::processes());
        all.extend(lithography::processes());
        all.extend(transfer::processes());
        all.extend(planarization::processes());
        all.into_iter().map(Process::referenced).collect()
    })
}

/// The process with this id.
pub fn process(id: &str) -> Option<&'static Process> {
    library().iter().find(|p| p.id == id)
}

/// How hot a film may get once it is on the wafer: the temperature a source shows it survives,
/// and the one above which a source shows it changes (a limit), each stated or not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    /// The film.
    pub film: String,
    /// A temperature a source took it to and it survived.
    pub survives: Stated,
    /// The temperature above which a source shows it changes.
    pub limit: Stated,
    /// What the sources say happened, in plain words.
    pub conditions: String,
}

/// The thermal budget of `film`: what its sources say, or not specified.
pub fn budget(film: &str) -> Budget {
    let deg = |v: f64, reference: &str, location: &str| {
        Stated::value(v, Unit::Celsius, src(reference, location))
    };
    let (survives, limit, conditions) = match film {
        "Si" | "SiO2" => (
            Stated::AtLeast {
                value: 1000.0,
                unit: Unit::Celsius,
                source: src("bruel-1995", "Step 3 (p. 1201)"),
            },
            Stated::not_specified(),
            "an SOI wafer's film and buried oxide are heated above 1000 °C to strengthen the bond",
        ),
        "LiNbO3" => (
            deg(450.0, "poberaj-2012", "PDF p. 4"),
            Stated::not_specified(),
            "the split lithium niobate film is annealed at 450 °C for 8 hours",
        ),
        "TiN" => (
            deg(600.0, "kearney-2018", "abstract (p. 78)"),
            Stated::not_specified(),
            "sputtered titanium nitride films were annealed at 600 °C for four hours",
        ),
        "Al" => (
            Stated::not_specified(),
            deg(
                577.0,
                "campbell-2008",
                "Figure 15.24, the Al/Si phase diagram (its eutectic)",
            ),
            "aluminium on silicon melts where they meet above the eutectic temperature",
        ),
        other => {
            let pcm = crate::material::catalogue::phase_changes()
                .into_iter()
                .find(|p| p.formula == other || p.id == other.to_lowercase());
            match pcm.and_then(|p| p.crystallization) {
                Some(c) => (
                    Stated::not_specified(),
                    Stated::value(
                        c.value - 273.15,
                        Unit::Celsius,
                        Source::new(&c.source.reference, &c.source.location),
                    ),
                    "an amorphous phase-change film crystallizes when held above this",
                ),
                None => (
                    Stated::not_specified(),
                    Stated::not_specified(),
                    "no source in hand gives its thermal budget",
                ),
            }
        }
    };
    Budget {
        film: film.into(),
        survives,
        limit,
        conditions: conditions.into(),
    }
}
