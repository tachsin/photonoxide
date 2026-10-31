//! The flow generator: rules that read a design layer by layer, with what lies below and above
//! each feature, and choose the steps that make it, each with its reason and its alternatives.
//!
//! - **The wafer.** A single-crystal film on an insulator comes as a bonded wafer (silicon on
//!   insulator, lithium niobate on insulator) and is never deposited.
//! - **Subtractive or additive.** A crystalline film is etched. A metal that comes down in
//!   straight lines (evaporated) is lifted off; one that doesn't (sputtered) is deposited over
//!   everything and etched, since lift-off relies on poor step coverage (Campbell, 11.9).
//! - **The etch.** The one the platform's papers used, else the library's first. Its depth: a
//!   full etch stops on the film below when a source says it does, and is timed otherwise, as a
//!   partial one always is, with the depth's uncertainty its cited uniformity. The design's
//!   sidewall is compared with every cited range and flagged outside them all.
//! - **The lithography.** Among those whose cited minimum feature is at most the layer's
//!   narrowest line and gap (measured, [`super::measure`]): the user's preference, then the one
//!   the platform's papers used for this purpose, then the library's order, the first whose
//!   resist a source shows to last the etch. None resolving the feature refuses the design.
//! - **Deposition.** The film's process that doesn't break a thermal budget of a film already
//!   there and can grow on what is there. A whole layer deposited over an etched step, with a
//!   patterned layer above it, is polished flat (CMP).
//! - **Ties** are broken by the library's order, so a design always gives the same flow.

use crate::units::Length;
use crate::{Error, Result};

use super::flow::{Alternative, Depth, Flow, Laid, Operation, Step, Tone};
use super::library::{Platform, Process, Purpose, StepType, budget, library, process};
use super::measure::{Features, features};
use super::physical::{Film, Physical};
use super::rayleigh::{Optics, Reach};
use super::rules::{k1_limit, over_etch};
use super::stated::{Capability, Quantity, Stated, Unit, number};

/// What the user asks of the choice: processes to prefer where several qualify, and processes
/// not to use.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Choices {
    /// Process ids to prefer.
    pub prefer: Vec<String>,
    /// Process ids not to use.
    pub avoid: Vec<String>,
}

impl Choices {
    /// Checks that every process named is in the library.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] naming the first that isn't.
    pub fn check(&self) -> Result<()> {
        for id in self.prefer.iter().chain(&self.avoid) {
            if process(id).is_none() {
                return Err(invalid(format!(
                    "the library has no process \"{id}\": its processes are {}",
                    library()
                        .iter()
                        .map(|p| p.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
        }
        if let Some(id) = self.prefer.iter().find(|id| self.avoid.contains(id)) {
            return Err(invalid(format!("\"{id}\" is both preferred and avoided")));
        }
        Ok(())
    }
}

fn invalid(message: String) -> Error {
    Error::invalid("fabrication", message)
}

fn um(v: f64) -> String {
    format!("{} µm", number(v))
}

/// The first stated value of `p`'s capabilities of `q`, with the capability.
fn first<'a>(p: &'a Process, q: &'a Quantity) -> Option<&'a Capability> {
    p.capabilities_of(q).find(|c| c.value.is_stated())
}

/// `p`'s first stated step coverage.
pub(crate) fn coverage(p: &Process) -> Option<&Capability> {
    first(p, &Quantity::StepCoverage)
}

/// Whether a source states `p` conformal: a step coverage of 100 %.
pub(crate) fn conformal(p: &Process) -> bool {
    coverage(p).is_some_and(|c| {
        c.value.unit() == Some(Unit::Percent) && c.value.low().is_some_and(|v| v >= 100.0)
    })
}

/// `p`'s first stated selectivity over `film`.
pub(crate) fn selectivity_over<'a>(p: &'a Process, film: &str) -> Option<&'a Capability> {
    p.capabilities.iter().find(|c| {
        matches!(&c.quantity, Quantity::Selectivity { over } if over == film) && c.value.is_stated()
    })
}

/// A capability as the narration cites it: value, conditions and source.
fn cited(c: &Capability) -> String {
    match c.value.source() {
        Some(s) => format!(
            "{} ({}; {}, {})",
            c.value, c.conditions, s.reference, s.location
        ),
        None => format!("not specified ({})", c.conditions),
    }
}

/// What a step needs of a resist: whether the lithography's resist is shown to last the etch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lasts {
    Yes,
    No,
    NotShown,
}

/// The state of the wafer as the rules build it.
struct State {
    platform: Platform,
    /// Films already on the wafer, for the thermal budgets.
    present: Vec<String>,
    /// Layers etched open, by index, with the step height (µm) their etch left.
    open: Vec<(usize, f64)>,
    /// A layer etched open as a preform for its own film (the Damascene process): the next
    /// deposition fills it with the film its shapes are drawn with.
    preform: Option<usize>,
    /// Whether the wafer is bare silicon still, nothing grown or deposited on it.
    bare: bool,
    steps: Vec<Step>,
    warnings: Vec<String>,
    not_specified: Vec<String>,
}

impl State {
    /// What the cladding is for: the cap of phase-change cells when there are some, the
    /// cladding over the guides otherwise.
    fn cladding(&self) -> Purpose {
        if self.present.iter().any(|f| super::library::phase_change(f)) {
            Purpose::Cell
        } else {
            Purpose::Cladding
        }
    }

    fn push(&mut self, step: Step) {
        for c in &step.capabilities {
            if !c.value.is_stated() {
                let note = format!("{}: {} ({})", step.name, c.quantity.name(), c.conditions);
                if !self.not_specified.contains(&note) {
                    self.not_specified.push(note);
                }
            }
        }
        self.steps.push(step);
    }

    fn warn(&mut self, w: String) {
        if !self.warnings.contains(&w) {
            self.warnings.push(w);
        }
    }

    fn note(&mut self, note: String) {
        if !self.not_specified.contains(&note) {
            self.not_specified.push(note);
        }
    }

    /// The thermal check of `p` against the films present: the violations, and the films a
    /// source shows survive it; with `noting`, the films whose budget no source gives are noted
    /// as not specified.
    fn thermal(&mut self, p: &Process, noting: bool) -> (Vec<String>, Vec<String>) {
        let check = thermal(p, &self.present);
        if noting {
            for n in check.unspecified {
                self.note(n);
            }
        }
        (check.violations, check.shown)
    }
}

/// A process's temperature against the thermal budgets of the films on the wafer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Thermal {
    /// The films whose cited limit it exceeds, each with why: the step is refused.
    pub violations: Vec<String>,
    /// The films a source shows survive it.
    pub shown: Vec<String>,
    /// The films no source in hand gives a budget for, noted as not specified.
    pub unspecified: Vec<String>,
}

/// The thermal check of `p`, at the highest temperature its sources state, against `films`.
pub fn thermal(p: &Process, films: &[String]) -> Thermal {
    let mut out = Thermal::default();
    let Some((t, cap)) = p.temperature() else {
        return out;
    };
    for film in films {
        let b = budget(film);
        if let Some(limit) = b.limit.high()
            && t > limit
        {
            out.violations.push(format!(
                "it reaches {} °C ({}), above {film}'s limit of {} ({}; {})",
                number(t),
                cap.value.source().map_or("", |s| s.reference.as_str()),
                b.limit,
                b.conditions,
                b.limit.source().map_or("", |s| s.reference.as_str())
            ));
        } else if b.survives.low().is_some_and(|s| t <= s) {
            out.shown.push(format!("{film} ({})", b.survives));
        } else if !b.limit.is_stated() {
            out.unspecified.push(format!(
                "{}: whether {film} survives its {} °C (no source in hand gives {film}'s \
                 thermal budget)",
                p.name,
                number(t)
            ));
        }
    }
    out
}

fn step_of(p: &Process, operation: Operation, reason: String) -> Step {
    Step {
        process: p.id.clone(),
        name: p.name.clone(),
        step: p.step,
        plain: p.plain.clone(),
        operation,
        reason,
        alternatives: Vec::new(),
        capabilities: p.capabilities.clone(),
        facts: p.facts.clone(),
        uncertainties: Vec::new(),
    }
}

fn must(id: &str) -> Result<&'static Process> {
    process(id).ok_or_else(|| invalid(format!("the library has no process {id}")))
}

/// Whether `film` is a single-crystal film, which comes with its wafer and is never deposited.
fn crystalline(film: &str) -> bool {
    matches!(film, "Si" | "LiNbO3" | "GaAs")
}

/// Whether `film` is a guide film the library deposits on a bare wafer: silicon nitride or
/// aluminium nitride.
pub(crate) fn guide_film(film: &str) -> bool {
    matches!(film, "Si3N4" | "SiN" | "AlN")
}

/// The layers bought with the design's wafer (up to its single-crystal film, none for a bare
/// wafer) and the design's platform; `None` for a design no platform of the library makes.
fn wafer_of(design: &Physical) -> Option<(usize, Platform)> {
    let films = design.layers();
    if let Some(w) = films.iter().position(|(d, _)| crystalline(&d.name)) {
        let platform = match films[w].0.name.as_str() {
            "Si" => Platform::Soi,
            "LiNbO3" => Platform::Lnoi,
            _ => Platform::Gaas,
        };
        return Some((w + 1, platform));
    }
    if design.substrate().name != "Si" {
        return None;
    }
    let guide = films.iter().find(|(d, _)| guide_film(&d.name))?;
    let platform = if guide.0.name == "AlN" {
        Platform::Aln
    } else {
        Platform::Nitride
    };
    Some((0, platform))
}

/// The thickest nitride a source etched into guides after depositing it, nm: Luke et al.'s 910
/// nm, with crack stops around the devices. A thicker film is made by the Damascene process.
fn subtractive_limit() -> Option<(f64, Capability)> {
    let lpcvd = process("lpcvd")?;
    lpcvd
        .capabilities_of(&Quantity::Thickness)
        .find(|c| c.value.source().is_some_and(|s| s.reference == "luke-2013"))
        .and_then(|c| c.value.high().map(|v| (v, c.clone())))
}

/// Whether layer `g`, a nitride guide layer on a bare wafer, is made by the photonic Damascene
/// process: preferred, or thicker than any nitride a source etched into guides; and only if every
/// shape is etched through with vertical walls over oxide (the preform is a trench in the oxide,
/// filled and polished: its walls are the guide's, and it leaves no slab).
fn damascene(design: &Physical, g: usize, choices: &Choices) -> Option<usize> {
    let s = design.structure();
    let layer = &s.stack().layers()[g];
    let (drawn, background) = &design.layers()[g];
    if !matches!(drawn.name.as_str(), "Si3N4" | "SiN")
        || background.name != "SiO2"
        || g == 0
        || design.layers()[g - 1].0.name != "SiO2"
        || choices.avoid.iter().any(|a| a == "damascene-preform")
    {
        return None;
    }
    let vertical = s
        .etches(&layer.name)
        .iter()
        .all(|e| e.is_full() && (e.sidewall - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    let thick = subtractive_limit()
        .is_some_and(|(limit, _)| layer.thickness.to_um() * 1000.0 > limit + 1e-9);
    let preferred = choices.prefer.iter().any(|p| p == "damascene-preform");
    (vertical && (thick || preferred)).then_some(g)
}

/// The flow that makes `design`, as `choices` ask.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a design the rules can't make yet: no single-crystal film for a
/// wafer, a film no process in the library deposits or etches, a patterned layer under a
/// deposited film of another kind, bodies placed over the stack, a feature no lithography's cited
/// range reaches; or choices naming a process the library hasn't.
pub fn generate(design: &Physical, choices: &Choices) -> Result<Flow> {
    choices.check()?;
    let s = design.structure();
    if !s.bodies().is_empty() {
        return Err(invalid(
            "the design places bodies over its stack, which the flow generator doesn't make yet"
                .into(),
        ));
    }
    let stack = s.stack();
    let layers = stack.layers();
    let films = design.layers();
    // a shape without area (a guide of zero length) is nothing a mask can carry
    for l in layers {
        if let Some(shape) = s.shapes(&l.name).iter().find(|shape| {
            let b = shape.bounds();
            b.max.x.to_um() - b.min.x.to_um() <= 0.0 || b.max.y.to_um() - b.min.y.to_um() <= 0.0
        }) {
            let b = shape.bounds();
            return Err(invalid(format!(
                "{} draws a shape without area (from ({}, {}) to ({}, {}) µm): a mask can't \
                 carry it",
                l.name,
                number(b.min.x.to_um()),
                number(b.min.y.to_um()),
                number(b.max.x.to_um()),
                number(b.max.y.to_um())
            )));
        }
    }
    // the wafer: up to the first single-crystal film, or a bare wafer under a deposited guide film
    let handle = design.substrate().name.as_str();
    let Some((w, platform)) = wafer_of(design) else {
        return Err(invalid(format!(
            "the design has no single-crystal film (silicon, lithium niobate or gallium \
             arsenide) to start from, and no guide film the library deposits on a bare {handle} \
             wafer (silicon nitride or aluminium nitride on oxide over silicon)"
        )));
    };
    let wanted: Vec<&str> = films[..w].iter().map(|(d, _)| d.name.as_str()).collect();
    let wafer = library()
        .iter()
        .find(|p| {
            p.step == StepType::Wafer
                && p.films == wanted
                && super::library::handle(p) == Some(handle)
        })
        .ok_or_else(|| {
            invalid(format!(
                "no wafer in the library has {} on a {handle} handle",
                if wanted.is_empty() {
                    "nothing".to_string()
                } else {
                    wanted.join(" under ")
                }
            ))
        })?;
    for (i, l) in layers[..w].iter().enumerate() {
        let crystal = i + 1 == w && wafer.films.last().is_some_and(|f| crystalline(f));
        if !crystal && (!s.shapes(&l.name).is_empty() || films[i].0.name != films[i].1.name) {
            return Err(invalid(format!(
                "{} lies under the wafer's film and is patterned, which a bought wafer isn't",
                l.name
            )));
        }
    }
    // the guide film: the wafer's own, or the first patterned layer on a bare wafer
    let guide = if w > 0 {
        Some(w - 1)
    } else {
        (0..layers.len()).find(|&i| guide_film(&films[i].0.name))
    };
    if let Some(g) = guide
        && s.shapes(&layers[g].name).is_empty()
        && films[g].0.name != films[g].1.name
    {
        return Err(invalid(format!(
            "{} draws no shapes, its {} all {} (a guide of zero length draws none): the design \
             has nothing of the film to make",
            layers[g].name, films[g].0.name, films[g].1.name
        )));
    }
    let damascene = match guide {
        Some(g) if w == 0 => damascene(design, g, choices),
        _ => None,
    };
    let mut state = State {
        platform,
        present: vec![design.substrate().name.clone()],
        open: Vec::new(),
        preform: None,
        bare: w == 0,
        steps: Vec::new(),
        warnings: Vec::new(),
        not_specified: Vec::new(),
    };
    let laid: Vec<(Laid, Film)> = layers[..w]
        .iter()
        .zip(films)
        .map(|(l, (d, _))| {
            (
                Laid {
                    name: l.name.clone(),
                    thickness: l.thickness,
                },
                d.clone(),
            )
        })
        .collect();
    let reason = if w == 0 {
        format!(
            "The guides' film, {}, is deposited, so the flow starts from a bare {handle} wafer, \
             as the {} papers in hand did; everything above it is grown or deposited.",
            guide.map_or("not one the library knows", |g| films[g].0.name.as_str()),
            platform.name()
        )
    } else {
        let film = &films[w - 1].0.name;
        let below = &layers[..w - 1];
        let how = if super::library::handle(wafer) == Some("Si") {
            "on an insulator is bought as a bonded wafer"
        } else {
            "is grown by epitaxy on the wafer it is bought on"
        };
        format!(
            "A single-crystal film of {film} {how}: it can't be deposited. The design's {} layer \
             ({}) is the wafer's film, on {} of {} ({}).",
            layers[w - 1].name,
            um(layers[w - 1].thickness.to_um()),
            below
                .iter()
                .map(|l| um(l.thickness.to_um()))
                .collect::<Vec<_>>()
                .join(" and "),
            below
                .iter()
                .zip(films)
                .map(|(_, (d, _))| d.name.as_str())
                .collect::<Vec<_>>()
                .join(" and "),
            below
                .iter()
                .map(|l| l.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    state.push(step_of(
        wafer,
        Operation::Wafer {
            substrate: design.substrate().clone(),
            layers: laid,
        },
        reason,
    ));
    for (d, _) in &films[..w] {
        if !state.present.contains(&d.name) {
            state.present.push(d.name.clone());
        }
    }
    if w > 0 && !s.shapes(&layers[w - 1].name).is_empty() {
        etch_layer(design, w - 1, choices, &mut state)?;
    }
    let mut i = w;
    let mut clad = false;
    while i < layers.len() {
        let (drawn, background) = &films[i];
        let patterned = !s.shapes(&layers[i].name).is_empty();
        if !patterned {
            // whole layers of one film, deposited together; with the cladding too when it is the
            // same film and nothing patterned lies above
            let mut j = i;
            while j + 1 < layers.len()
                && s.shapes(&layers[j + 1].name).is_empty()
                && films[j + 1].0 == *drawn
            {
                j += 1;
            }
            let last = j + 1 == layers.len();
            let with_cladding = last && *design.cladding() == *drawn;
            let patterned_above = !last;
            if drawn.name != background.name {
                return Err(invalid(format!(
                    "{} is drawn as {} with a background of {}, but nothing is drawn on it",
                    layers[i].name, drawn.name, background.name
                )));
            }
            // the Damascene preform is etched into this oxide: it is grown that much thicker
            let into = damascene.filter(|&g| g == j + 1 && films[g].1 == *drawn);
            let laid = layers[i..=into.unwrap_or(j)]
                .iter()
                .map(|l| Laid {
                    name: l.name.clone(),
                    thickness: l.thickness,
                })
                .collect();
            let purpose = if with_cladding {
                state.cladding()
            } else if !state
                .steps
                .iter()
                .any(|s| s.step == StepType::PatternTransfer)
            {
                Purpose::Under
            } else {
                Purpose::Guide
            };
            deposit(
                design,
                choices,
                drawn,
                laid,
                with_cladding,
                patterned_above,
                purpose,
                &mut state,
            )?;
            clad = with_cladding;
            i = j + 1;
            continue;
        }
        if damascene == Some(i) {
            damascene_layer(design, i, choices, &mut state)?;
        } else if guide_film(&drawn.name) && w == 0 {
            guide_layer(design, i, choices, &mut state)?;
        } else {
            metal_layer(design, i, choices, &mut state)?;
        }
        i += 1;
    }
    if !clad {
        deposit(
            design,
            choices,
            design.cladding(),
            Vec::new(),
            true,
            false,
            state.cladding(),
            &mut state,
        )?;
    }
    if films.iter().any(|(d, _)| d.name == "TiN") {
        let p = must("contact-metal")?;
        let mut step = step_of(
            p,
            Operation::Undrawn {
                what: "the heater's contacts, vias and pads".into(),
            },
            "A heater carries its current in and out through contacts, vias through the oxide \
             above it, and pads; the design draws the heater alone, so the step is listed and \
             not drawn, its metal as its sources give it."
                .into(),
        );
        step.uncertainties
            .push("the contact metal, its thickness and its vias aren't drawn".into());
        state.push(step);
    }
    for id in &choices.prefer {
        if !state.steps.iter().any(|s| &s.process == id) {
            let why: Vec<String> = state
                .steps
                .iter()
                .flat_map(|s| &s.alternatives)
                .filter(|a| &a.process == id)
                .map(|a| a.why_not.clone())
                .collect();
            return Err(invalid(format!(
                "the preferred process \"{id}\" makes no step of this design{}",
                if why.is_empty() {
                    ": nothing in the design is a step it does".to_string()
                } else {
                    format!(": {}", why.join("; "))
                }
            )));
        }
    }
    Ok(Flow {
        steps: state.steps,
        warnings: state.warnings,
        not_specified: state.not_specified,
    })
}

/// The processes of `step` acting on `film`, the user's preference first, then those the
/// platform's sources used for `purpose`, then the library's order; without those avoided.
fn ordered(
    step: StepType,
    film: Option<&str>,
    platform: Platform,
    purpose: Purpose,
    choices: &Choices,
) -> Vec<&'static Process> {
    let mut all: Vec<&'static Process> = library()
        .iter()
        .filter(|p| p.step == step && film.is_none_or(|f| p.acts_on(f)))
        .filter(|p| !choices.avoid.contains(&p.id))
        .collect();
    let rank = |p: &Process| {
        if choices.prefer.contains(&p.id) {
            0
        } else if p.used(platform, purpose).is_some() {
            1
        } else {
            2
        }
    };
    all.sort_by_key(|p| rank(p));
    all
}

/// Why `p` came after `chosen` in the order, when both could do the step.
fn later(p: &Process, chosen: &Process, platform: Platform, purpose: Purpose) -> String {
    match chosen.used(platform, purpose) {
        Some(u) if p.used(platform, purpose).is_none() => format!(
            "it could do it, but the papers in hand used {} for this ({}, {})",
            chosen.name, u.source.reference, u.source.location
        ),
        _ => "it could do it too; ties go by the library's order".into(),
    }
}

/// Whether the sidewall `deg` lies in `p`'s cited ranges: the closest distance to them, degrees
/// (0 inside), `None` with none stated.
fn sidewall_off(p: &Process, deg: f64) -> Option<f64> {
    p.capabilities_of(&Quantity::Sidewall)
        .filter_map(|c| match (c.value.low(), c.value.high()) {
            (Some(lo), Some(hi)) => Some(if deg < lo - 1e-9 {
                lo - deg
            } else if deg > hi + 1e-9 {
                deg - hi
            } else {
                0.0
            }),
            _ => None,
        })
        .fold(None, |best: Option<f64>, d| {
            Some(best.map_or(d, |b| b.min(d)))
        })
}

/// The resist of lithography `l`, as a mask for an etch `depth` µm deep by `etch`: whether a
/// source shows it lasts, and why.
fn lasts(l: &Process, etch: &Process, depth: f64) -> (Lasts, String, Option<Length>) {
    let Some(resist) = &l.resist else {
        return (Lasts::NotShown, "it has no resist".into(), None);
    };
    let own = first(l, &Quantity::Thickness).and_then(|c| c.value.high());
    let Some(m) = etch.masks.iter().find(|m| &m.resist == resist) else {
        return (
            Lasts::NotShown,
            format!(
                "no source in hand etched with its {resist} as the mask of {}",
                etch.name
            ),
            own.map(|t| Length::um(t * thickness_scale(l))),
        );
    };
    let unit_um = |s: &Stated, v: f64| match s.unit() {
        Some(super::stated::Unit::Nm) => v / 1000.0,
        _ => v,
    };
    let thickness = own
        .map(|t| t * thickness_scale(l))
        .or_else(|| m.thickness.high().map(|t| unit_um(&m.thickness, t)));
    // the selectivity's lower end: a range's worst case
    if let (Some(sel), Some(t)) = (m.selectivity.low().or(m.selectivity.high()), thickness) {
        let reach = sel * t;
        let src = m.selectivity.source().map_or(String::new(), |s| {
            format!("{}, {}", s.reference, s.location)
        });
        return if depth <= reach + 1e-12 {
            (
                Lasts::Yes,
                format!(
                    "its {resist}, {} thick, etched at a selectivity of {} ({src}), lasts down to \
                     {}, the etch needs {}",
                    um(t),
                    m.selectivity,
                    um(reach),
                    um(depth)
                ),
                Some(Length::um(t)),
            )
        } else {
            (
                Lasts::No,
                format!(
                    "its {resist}, {} thick, etched at a selectivity of {} ({src}), lasts only \
                     {} of the {} etch: the etch needs {} of it, or a hard mask",
                    um(t),
                    m.selectivity,
                    um(reach),
                    um(depth),
                    um(depth / sel)
                ),
                Some(Length::um(t)),
            )
        };
    }
    if let Some(d) = m.depth.high() {
        let d = unit_um(&m.depth, d);
        let src = m.depth.source().map_or(String::new(), |s| {
            format!("{}, {}", s.reference, s.location)
        });
        return if depth <= d + 1e-12 {
            (
                Lasts::Yes,
                format!(
                    "a source etched {} with its {resist} as the mask ({src})",
                    um(d)
                ),
                thickness.map(Length::um),
            )
        } else {
            (
                Lasts::NotShown,
                format!(
                    "the deepest etch a source made with its {resist} is {} ({src}); this one is {}",
                    um(d),
                    um(depth)
                ),
                thickness.map(Length::um),
            )
        };
    }
    (
        Lasts::NotShown,
        format!("no source in hand gives its {resist}'s selectivity or a depth it lasted"),
        thickness.map(Length::um),
    )
}

/// A lithography's resist thickness capability's unit factor to µm.
fn thickness_scale(l: &Process) -> f64 {
    match first(l, &Quantity::Thickness).and_then(|c| c.value.unit()) {
        Some(super::stated::Unit::Nm) => 1e-3,
        _ => 1.0,
    }
}

/// The smallest feature `l` resolves, µm: its first stated minimum feature (the typical one;
/// any after it are context).
fn resolves(l: &Process) -> Option<(f64, &Capability)> {
    let c = first(l, &Quantity::MinFeature)?;
    let v = c.value.low().or(c.value.high())?;
    let scale = match c.value.unit() {
        Some(super::stated::Unit::Nm) => 1e-3,
        _ => 1.0,
    };
    Some((v * scale, c))
}

/// The lithography for a layer's shapes of `f` features, for `purpose`, its resist a mask for
/// `etch` `depth` µm deep when there is an etch: the step, with its alternatives.
fn lithography(
    f: Features,
    purpose: Purpose,
    etch: Option<(&Process, f64)>,
    choices: &Choices,
    state: &mut State,
    layer: &str,
) -> Result<(&'static Process, Vec<Alternative>, Option<Length>, String)> {
    let smallest = f.smallest_um();
    let all = ordered(
        StepType::Lithography,
        None,
        state.platform,
        purpose,
        choices,
    );
    let mut fits: Vec<(&'static Process, Lasts, String, Option<Length>)> = Vec::new();
    let mut alternatives = Vec::new();
    for l in &all {
        if let Some(o) = Optics::of(l)
            && o.reach(smallest * 1000.0) == Reach::Beyond
        {
            alternatives.push(Alternative {
                process: l.id.clone(),
                name: l.name.clone(),
                why_not: format!(
                    "the layer's narrowest feature, {}, asks it for k₁ = {} at its highest cited \
                     NA, {}: below the theoretical limit of {} (campbell-2008, Eq. 7.18, p. 183)",
                    um(smallest),
                    number(o.k1(smallest * 1000.0)),
                    number(o.na[1]),
                    k1_limit()
                ),
            });
            continue;
        }
        match resolves(l) {
            Some((r, _)) if r <= smallest + 1e-12 => {
                let (ok, why, t) = match etch {
                    Some((e, d)) => lasts(l, e, d),
                    None => (
                        Lasts::Yes,
                        String::new(),
                        first(l, &Quantity::Thickness)
                            .and_then(|c| c.value.high())
                            .map(|t| Length::um(t * thickness_scale(l))),
                    ),
                };
                fits.push((l, ok, why, t));
            }
            Some((_, c)) => alternatives.push(Alternative {
                process: l.id.clone(),
                name: l.name.clone(),
                why_not: format!(
                    "its cited minimum feature, {}, is above the layer's narrowest feature, {}",
                    cited(c),
                    um(smallest)
                ),
            }),
            None => alternatives.push(Alternative {
                process: l.id.clone(),
                name: l.name.clone(),
                why_not: "its resolution is not specified by the sources in hand".into(),
            }),
        }
    }
    let typical =
        |l: &Process| Optics::of(l).is_none_or(|o| o.reach(smallest * 1000.0) == Reach::Typical);
    let pick = fits
        .iter()
        .position(|(l, ok, _, _)| choices.prefer.contains(&l.id) && *ok != Lasts::No)
        .or_else(|| {
            // the platform's papers' choice, its resist shown to last or not shown not to
            fits.iter().position(|(l, ok, _, _)| {
                l.used(state.platform, purpose).is_some() && *ok == Lasts::Yes && typical(l)
            })
        })
        .or_else(|| {
            fits.iter().position(|(l, ok, _, _)| {
                l.used(state.platform, purpose).is_some() && *ok != Lasts::No && typical(l)
            })
        })
        .or_else(|| {
            fits.iter()
                .position(|(l, ok, _, _)| *ok == Lasts::Yes && typical(l))
        })
        .or_else(|| fits.iter().position(|(_, ok, _, _)| *ok == Lasts::Yes))
        .or(if fits.is_empty() { None } else { Some(0) })
        .ok_or_else(|| {
            invalid(format!(
                "{layer}'s narrowest feature, {}, is below every cited minimum feature of the \
                 library's lithographies: {}",
                um(smallest),
                alternatives
                    .iter()
                    .map(|a| format!("{} ({})", a.name, a.why_not))
                    .collect::<Vec<_>>()
                    .join("; ")
            ))
        })?;
    let (chosen, ok, why, thickness) = fits[pick].clone();
    if ok != Lasts::Yes {
        state.warn(format!(
            "{layer}: {} is kept as {}, but {why}: a thicker resist or a hard mask may be needed \
             (not specified)",
            chosen.name,
            if choices.prefer.contains(&chosen.id) {
                "the preferred lithography"
            } else {
                "the platform's choice, no lithography's resist in the library being shown to \
                 last the etch"
            }
        ));
    }
    for (l, lasts_, why_l, _) in &fits {
        if l.id == chosen.id {
            continue;
        }
        let mut why_not = match lasts_ {
            Lasts::No => format!("it resolves the layer, but {why_l}"),
            _ => later(l, chosen, state.platform, purpose),
        };
        if l.id.starts_with("ebl-") {
            why_not.push_str(
                "; electron-beam lithography writes point by point with no mask, so it suits \
                 prototypes",
            );
        }
        alternatives.push(Alternative {
            process: l.id.clone(),
            name: l.name.clone(),
            why_not,
        });
    }
    let mut reason = format!(
        "The layer's narrowest line is {} and its narrowest gap {}",
        um(f.line_um),
        f.gap_um
            .map_or("none (nothing lies beside anything)".into(), um),
    );
    if let Some((r, c)) = resolves(chosen) {
        reason.push_str(&format!(
            "; {} resolves down to {} ({}).",
            chosen.name,
            um(r),
            cited(c)
        ));
    }
    if let Some(o) = Optics::of(chosen) {
        let k1 = o.k1(smallest * 1000.0);
        reason.push_str(&format!(
            " By the Rayleigh criterion, W = k₁λ/NA (campbell-2008, Eq. 7.18, p. 183), the \
             feature asks for k₁ = {} at its highest cited NA, {}, at {} nm: {}.",
            number(k1),
            number(o.na[1]),
            number(o.wavelength_nm),
            match o.reach(smallest * 1000.0) {
                Reach::Typical => "within the typical 0.4 to 0.8",
                Reach::Enhanced => "below the typical 0.4, above the limit of 0.25",
                Reach::Beyond => "below the limit of 0.25",
            }
        ));
        if o.reach(smallest * 1000.0) == Reach::Enhanced {
            state.warn(format!(
                "{layer}: its narrowest feature, {}, asks {} for k₁ = {}, below the typical \
                 0.4 to 0.8 (campbell-2008, Eq. 7.18, p. 183): it prints only with resolution \
                 enhancement such as tilted illumination (mack-2007, Eq. 2.83, p. 58)",
                um(smallest),
                chosen.name,
                number(k1)
            ));
        }
    }
    if let Some(u) = chosen.used(state.platform, purpose) {
        reason.push_str(&format!(
            " The papers in hand used it for this ({}, {}).",
            u.source.reference, u.source.location
        ));
    }
    if !why.is_empty() {
        reason.push_str(&format!(" As the etch's mask, {why}."));
    }
    Ok((chosen, alternatives, thickness, reason))
}

/// The etch of layer `i`'s shapes: the lithography, the etch and the strip, one patterning per
/// run of shapes drawn with the same etch.
fn etch_layer(design: &Physical, i: usize, choices: &Choices, state: &mut State) -> Result<()> {
    let s = design.structure();
    let layers = s.stack().layers();
    let layer = &layers[i];
    let film = design.layers()[i].0.name.clone();
    let shapes = s.shapes(&layer.name);
    let etches = s.etches(&layer.name);
    let below = if i == 0 {
        design.substrate().name.clone()
    } else {
        design.layers()[i - 1].0.name.clone()
    };
    let mut k = 0;
    while k < shapes.len() {
        let mut n = k + 1;
        while n < shapes.len() && etches[n] == etches[k] {
            n += 1;
        }
        let run = &shapes[k..n];
        let e = etches[k];
        let thickness = layer.thickness.to_um();
        let depth_um = e.depth.map_or(thickness, Length::to_um);
        let deg = e.sidewall.to_degrees();
        // the etch
        let etching = ordered(
            StepType::PatternTransfer,
            Some(&film),
            state.platform,
            Purpose::Guide,
            choices,
        );
        let mut etching: Vec<&Process> =
            etching.into_iter().filter(|p| p.id != "lift-off").collect();
        // an etch whose cited walls include the design's comes before the platform's choice
        etching.sort_by_key(|p| {
            if choices.prefer.contains(&p.id) {
                0
            } else if sidewall_off(p, deg) == Some(0.0) {
                1
            } else {
                2
            }
        });
        let chosen = *etching
            .first()
            .ok_or_else(|| invalid(format!("no process in the library etches {film}")))?;
        let f = features(run)?;
        let (litho, litho_alts, resist_t, litho_reason) = lithography(
            f,
            Purpose::Guide,
            Some((chosen, depth_um)),
            choices,
            state,
            &layer.name,
        )?;
        let mask = run.to_vec();
        let mut pattern = step_of(
            litho,
            Operation::Pattern {
                resist: litho.resist.clone().unwrap_or_default(),
                thickness: resist_t,
                mask,
                tone: Tone::Keep,
            },
            format!(
                "The resist must stay on the {} shape{} of {} that the etch keeps. {litho_reason}",
                run.len(),
                if run.len() == 1 { "" } else { "s" },
                layer.name
            ),
        );
        pattern.alternatives = litho_alts;
        state.push(pattern);
        // depth: a stop or a timed etch
        let stop = chosen.stops_on.iter().find(|st| st.film == below);
        let by_selectivity = selectivity_over(chosen, &below);
        let (depth, depth_reason) = match (e.depth, stop) {
            (None, None) if by_selectivity.is_some() => {
                let c = by_selectivity.expect("checked");
                let s = c.value.low().or(c.value.high()).unwrap_or(1.0);
                let oe = over_etch();
                let loss = oe.high().unwrap_or(0.0) / 100.0 * thickness / s;
                let below_t = if i == 0 {
                    None
                } else {
                    Some(layers[i - 1].thickness.to_um())
                };
                if below_t.is_some_and(|t| loss >= t) {
                    state.warn(format!(
                        "{}: the over-etch that clears the {film} ({oe} of its etch; \
                         franssila-2010, pp. 138–139) at a selectivity of {} over {below} would \
                         remove {} of it, as much as the layer below is thick",
                        layer.name,
                        c.value,
                        um(loss)
                    ));
                }
                (
                    Depth::ToStop,
                    format!(
                        "The etch goes through the {} of {film} and is ended on the {below} \
                         below, which it etches at least {} times more slowly ({}): run {oe} \
                         longer to clear the film everywhere (franssila-2010, pp. 138–139), it \
                         takes at most {} off the {below}.",
                        um(thickness),
                        number(s),
                        cited(c),
                        um(loss)
                    ),
                )
            }
            (None, Some(st)) => (
                Depth::ToStop,
                format!(
                    "The etch goes through the {} of {film} and stops on the {below} below, \
                     which it etches far more slowly ({}, {}): the strip's height is the film's.",
                    um(thickness),
                    st.source.reference,
                    st.source.location
                ),
            ),
            (None, None) => (
                Depth::Timed(layer.thickness),
                format!(
                    "The etch goes through the {} of {film}; no source in hand says it stops on \
                     the {below} below, so it is timed.",
                    um(thickness)
                ),
            ),
            (Some(d), _) => (
                Depth::Timed(d),
                format!(
                    "The etch goes {} into the {} of {film} and leaves a slab: nothing below \
                     stops it there, so it is timed.",
                    um(d.to_um()),
                    um(thickness)
                ),
            ),
        };
        let mut etch_step = step_of(
            chosen,
            Operation::Etch {
                layer: layer.name.clone(),
                depth,
                sidewall: e.sidewall,
            },
            String::new(),
        );
        let mut reason = depth_reason;
        if let Some(u) = chosen.used(state.platform, Purpose::Guide) {
            reason.push_str(&format!(
                " The papers in hand etched {film} guides with it ({}, {}).",
                u.source.reference, u.source.location
            ));
        } else if !etching
            .iter()
            .any(|p| p.used(state.platform, Purpose::Guide).is_some())
        {
            // no paper on the platform names its guides' etch: the process is not specified
            let papers: Vec<String> = library()
                .iter()
                .filter(|p| p.step == StepType::Wafer)
                .filter_map(|p| p.used(state.platform, Purpose::Guide))
                .map(|u| u.source.reference.clone())
                .collect();
            reason.push_str(&format!(
                " The etch of the {film} guides is not specified: the {} papers in hand ({}) \
                 name none. The library's etch of {film} by its chemistry stands in, as the \
                 textbooks give it; its rate, walls, depth control and selectivity over the \
                 film below are what the sources don't state.",
                state.platform.name(),
                if papers.is_empty() {
                    "none".to_string()
                } else {
                    papers.join(", ")
                }
            ));
            state.note(format!(
                "{}: the etch of the {film} guides (the {} papers in hand name none; {} stands \
                 in)",
                layer.name,
                state.platform.name(),
                chosen.name
            ));
        }
        match sidewall_off(chosen, deg) {
            Some(0.0) => reason.push_str(&format!(
                " The design's walls at {}° are within its cited sidewall angles.",
                number(deg)
            )),
            Some(_) | None => {
                let ranges: Vec<String> = chosen
                    .capabilities_of(&Quantity::Sidewall)
                    .map(cited)
                    .collect();
                let mut closest: Vec<(f64, &Process)> = library()
                    .iter()
                    .filter(|p| p.acts_on(&film) && p.step == StepType::PatternTransfer)
                    .filter_map(|p| sidewall_off(p, deg).map(|o| (o, p)))
                    .collect();
                closest.sort_by(|a, b| a.0.total_cmp(&b.0));
                let near = closest
                    .iter()
                    .take(2)
                    .map(|(o, p)| format!("{} ({}° off)", p.name, number(*o)))
                    .collect::<Vec<_>>()
                    .join(", ");
                let w = format!(
                    "{}: the design's walls at {}° are outside every cited sidewall angle of {} \
                     ({}); the library's etches of {film} closest to it: {}",
                    layer.name,
                    number(deg),
                    chosen.name,
                    if ranges.is_empty() {
                        "none stated".into()
                    } else {
                        ranges.join("; ")
                    },
                    if near.is_empty() {
                        "none states one".into()
                    } else {
                        near
                    }
                );
                reason.push_str(&format!(
                    " The design's walls at {}° are outside its cited sidewall angles (see the \
                     warnings).",
                    number(deg)
                ));
                state.warn(w);
            }
        }
        if let Depth::Timed(d) = depth {
            let spread: Vec<String> = chosen
                .capabilities_of(&Quantity::Uniformity)
                .filter(|c| c.value.is_stated())
                .map(cited)
                .collect();
            etch_step.uncertainties.push(if spread.is_empty() {
                format!(
                    "the timed depth of {}: its uncertainty is not specified by the sources in \
                     hand",
                    um(d.to_um())
                )
            } else {
                format!(
                    "the timed depth of {}, uncertain by the etch's cited depth variation: {}",
                    um(d.to_um()),
                    spread.join("; ")
                )
            });
        }
        etch_step.reason = reason;
        for p in etching.iter().skip(1) {
            let side = match sidewall_off(p, deg) {
                Some(0.0) => format!("its cited walls include {}°", number(deg)),
                Some(o) => format!("its cited walls are {}° from the design's", number(o)),
                None => "its sidewall angle is not specified".into(),
            };
            etch_step.alternatives.push(Alternative {
                process: p.id.clone(),
                name: p.name.clone(),
                why_not: format!(
                    "{side}; {}",
                    later(p, chosen, state.platform, Purpose::Guide)
                ),
            });
        }
        state.push(etch_step);
        let strip = must("resist-strip")?;
        state.push(step_of(
            strip,
            Operation::Strip,
            "The resist has done its work as the etch's mask and is removed before the next \
             film goes on."
                .into(),
        ));
        let step_height = depth_um;
        if !state.open.iter().any(|(j, _)| *j == i) {
            state.open.push((i, step_height));
        }
        k = n;
    }
    Ok(())
}

/// A guide film deposited over the wafer and etched into the design's guides (layer `i`): crack
/// stops first for a nitride film thicker than its sources deposit without them, then the
/// deposition, the etch, and the anneal the platform's papers give the film.
fn guide_layer(design: &Physical, i: usize, choices: &Choices, state: &mut State) -> Result<()> {
    let s = design.structure();
    let layer = &s.stack().layers()[i];
    let (drawn, _) = &design.layers()[i];
    let t_nm = layer.thickness.to_um() * 1000.0;
    if !state.open.is_empty() {
        return Err(invalid(format!(
            "{} is deposited over everything while a layer below is still etched open",
            layer.name
        )));
    }
    if let Some(lpcvd) = process("lpcvd")
        && matches!(drawn.name.as_str(), "Si3N4" | "SiN")
    {
        // the thickness above which a source says nitride cracks without crack stops
        let cracks = lpcvd
            .facts
            .iter()
            .find(|f| f.source.reference == "luke-2013" && f.text.contains("400 nm"));
        if let (Some(f), Some(stops)) = (cracks, process("crack-stop-trenches"))
            && t_nm > 400.0
            && !choices.avoid.contains(&stops.id)
        {
            let mut step = step_of(
                stops,
                Operation::Undrawn {
                    what: "crack-stop trenches around the device area".into(),
                },
                format!(
                    "The design's {} of nitride is above the 400 nm over which, its source says, \
                     \"{}\" ({}, {}): trenches around the devices stop the cracks that start at \
                     the wafer's edge. The design doesn't draw them, so the step is listed and \
                     not drawn.",
                    um(t_nm / 1000.0),
                    f.text,
                    f.source.reference,
                    f.source.location
                ),
            );
            if let Some((limit, c)) = subtractive_limit()
                && t_nm > limit
            {
                state.warn(format!(
                    "{}: {} of nitride is thicker than any its sources etched into guides ({}); \
                     the photonic Damascene process makes thicker guides when their walls are \
                     vertical and etched through",
                    layer.name,
                    um(t_nm / 1000.0),
                    cited(&c)
                ));
            }
            step.alternatives.push(Alternative {
                process: "damascene-preform".into(),
                name: "The photonic Damascene process".into(),
                why_not: if s.etches(&layer.name).iter().all(|e| {
                    e.is_full() && (e.sidewall - std::f64::consts::FRAC_PI_2).abs() < 1e-12
                }) {
                    format!(
                        "it makes nitride guides up to 1.35 µm thick without cracks \
                         (pfeiffer-2016, PDF p. 3); the design's {} is within what its sources \
                         etched after crack stops, so the etch is kept (prefer \
                         damascene-preform to choose it)",
                        um(t_nm / 1000.0)
                    )
                } else {
                    "its guides are the oxide preform's trenches filled and polished: they \
                     can't have the design's sloped walls or partial etch"
                        .into()
                },
            });
            state.push(step);
        }
    }
    deposit(
        design,
        choices,
        drawn,
        vec![Laid {
            name: layer.name.clone(),
            thickness: layer.thickness,
        }],
        false,
        false,
        Purpose::Guide,
        state,
    )?;
    etch_layer(design, i, choices, state)?;
    anneal(&drawn.name, choices, state);
    Ok(())
}

/// The anneal the platform's papers give `film` after it is patterned, if one is in the library
/// and not avoided: refused, with a warning, when it passes a film's cited limit.
fn anneal(film: &str, choices: &Choices, state: &mut State) {
    let Some(p) = library().iter().find(|p| {
        p.step == StepType::Anneal
            && p.acts_on(film)
            && p.used(state.platform, Purpose::Guide).is_some()
            && !choices.avoid.contains(&p.id)
    }) else {
        return;
    };
    let (violations, shown) = state.thermal(p, true);
    if !violations.is_empty() {
        state.warn(format!("{} is left out: {}", p.name, violations.join("; ")));
        return;
    }
    let u = p.used(state.platform, Purpose::Guide).expect("checked");
    let mut step =
        step_of(
            p,
            // an anneal changes no shape: listed, and nothing drawn
            Operation::Undrawn {
                what: format!("the {film} annealed"),
            },
            format!(
                "The {} papers in hand anneal their {film} after it is patterned ({}, {}): {}{}",
                state.platform.name(),
                u.source.reference,
                u.source.location,
                p.temperature().map_or(
                    "its temperature is not specified".into(),
                    |(t, c)| format!("at {} °C ({})", number(t), cited(c))
                ),
                if shown.is_empty() {
                    ".".to_string()
                } else {
                    format!(
                        ", which the films already there are shown to survive: {}.",
                        shown.join(", ")
                    )
                }
            ),
        );
    step.uncertainties
        .push("the anneal changes no shape: the film's index and stress, not its outline".into());
    state.push(step);
}

/// A nitride guide layer `i` made by the photonic Damascene process: the guides' trenches etched
/// into the oxide grown for them, the nitride deposited into them in one run, the excess polished
/// off, then the anneal.
fn damascene_layer(
    design: &Physical,
    i: usize,
    choices: &Choices,
    state: &mut State,
) -> Result<()> {
    let s = design.structure();
    let layer = &s.stack().layers()[i];
    let (drawn, _) = &design.layers()[i];
    let t = layer.thickness.to_um();
    let shapes = s.shapes(&layer.name);
    let preform = must("damascene-preform")?;
    let f = features(shapes)?;
    let (litho, litho_alts, resist_t, litho_reason) = lithography(
        f,
        Purpose::Guide,
        Some((preform, t)),
        choices,
        state,
        &layer.name,
    )?;
    let mut pattern = step_of(
        litho,
        Operation::Pattern {
            resist: litho.resist.clone().unwrap_or_default(),
            thickness: resist_t,
            mask: shapes.to_vec(),
            tone: Tone::Open,
        },
        format!(
            "The resist is opened over the {} guide shape{} of {}: the guides are etched into \
             the oxide as trenches, to be filled with {}. {litho_reason}",
            shapes.len(),
            if shapes.len() == 1 { "" } else { "s" },
            layer.name,
            drawn.name
        ),
    );
    pattern.alternatives = litho_alts;
    state.push(pattern);
    let why = if choices.prefer.iter().any(|p| p == &preform.id) {
        "it is preferred".to_string()
    } else {
        match subtractive_limit() {
            Some((_, c)) => format!(
                "the design's {} of nitride is thicker than any its sources etched into guides \
                 ({}), and its walls are vertical and etched through",
                um(t),
                cited(&c)
            ),
            None => "the design's nitride is thick".into(),
        }
    };
    let mut etch = step_of(
        preform,
        Operation::Etch {
            layer: layer.name.clone(),
            depth: Depth::Timed(layer.thickness),
            sidewall: std::f64::consts::FRAC_PI_2,
        },
        format!(
            "The photonic Damascene process, because {why}: the guides' trenches are etched {} \
             into the oxide, a timed etch (the oxide below is the same film, so nothing stops \
             it). The nitride is never etched, so its walls are the trenches' (pfeiffer-2016, \
             PDF pp. 2–3).",
            um(t)
        ),
    );
    etch.uncertainties.push(format!(
        "the trenches' timed depth of {}, and so the guides' height: its uncertainty is not \
         specified by the sources in hand",
        um(t)
    ));
    etch.uncertainties.push(
        "the dense stress-release pattern etched into the oxide with the guides (5 µm squares, \
         10 µm clear of each guide; pfeiffer-2016, PDF p. 3) isn't drawn by the design"
            .into(),
    );
    etch.alternatives.push(Alternative {
        process: "icp-si3n4-chf3".into(),
        name: "The nitride deposited and etched".into(),
        why_not: format!(
            "{} of nitride, deposited and etched, cracks under its own stress (luke-2013, PDF p. \
             2; pfeiffer-2016, PDF p. 3)",
            um(t)
        ),
    });
    state.push(etch);
    state.push(step_of(
        must("resist-strip")?,
        Operation::Strip,
        "The resist has done its work as the preform's mask and is removed before the nitride \
         goes on."
            .into(),
    ));
    state.open.push((i, t));
    state.preform = Some(i);
    deposit(
        design,
        choices,
        drawn,
        Vec::new(),
        false,
        true,
        Purpose::Guide,
        state,
    )?;
    anneal(&drawn.name, choices, state);
    Ok(())
}

/// A whole film `film` deposited: it fills the layers etched open, then lies in `laid` (and is
/// the cladding with `cladding`); polished flat first when it goes over an etched step and a
/// patterned layer comes above it.
#[allow(clippy::too_many_arguments)]
fn deposit(
    design: &Physical,
    choices: &Choices,
    film: &Film,
    laid: Vec<Laid>,
    cladding: bool,
    patterned_above: bool,
    purpose: Purpose,
    state: &mut State,
) -> Result<()> {
    let layers = design.structure().stack().layers();
    for (j, _) in &state.open {
        // a preform is filled with the film its shapes are drawn with, any other opening with
        // its layer's background
        let want = if state.preform == Some(*j) {
            &design.layers()[*j].0
        } else {
            &design.layers()[*j].1
        };
        if want != film {
            return Err(invalid(format!(
                "{} is filled with {} where it was etched, but the next film deposited is {}",
                layers[*j].name, want.name, film.name
            )));
        }
    }
    let candidates = ordered(
        StepType::Deposition,
        Some(&film.name),
        state.platform,
        purpose,
        choices,
    );
    let mut alternatives = Vec::new();
    let mut allowed: Vec<(&Process, usize)> = Vec::new();
    // thermal oxidation grows from the bare wafer's silicon, which it consumes: its heat is
    // checked against nothing else, as nothing else is there
    let grows = state.bare && design.substrate().name == "Si";
    for p in &candidates {
        if p.id == "thermal-oxidation" && !grows {
            alternatives.push(Alternative {
                process: p.id.clone(),
                name: p.name.clone(),
                why_not: "it grows oxide only from silicon, by consuming it: it can't lay a film \
                          over what is on the wafer (deal-grove-1965, Section 2 (PDF p. 4))"
                    .into(),
            });
            continue;
        }
        let check = if p.id == "thermal-oxidation" {
            Thermal::default()
        } else {
            thermal(p, &state.present)
        };
        if !check.violations.is_empty() {
            alternatives.push(Alternative {
                process: p.id.clone(),
                name: p.name.clone(),
                why_not: check.violations.join("; "),
            });
            continue;
        }
        // a process whose temperature no source states shows no film surviving it
        let unshown = if p.temperature().is_none() && p.id != "thermal-oxidation" {
            state.present.len()
        } else {
            check.unspecified.len()
        };
        allowed.push((p, unshown));
    }
    // the preferred first; then the fewest films not shown to survive it, and among those that
    // leave a film's survival unshown the coolest; then, filling etched regions, a conformal film;
    // ties in the platform's and the library's order
    let filling = !state.open.is_empty();
    let heat = |p: &Process, unshown: usize| {
        if unshown == 0 {
            0.0
        } else {
            p.temperature().map_or(f64::INFINITY, |(t, _)| t)
        }
    };
    // the platform's papers' choice, like a lithography's, comes before conformality
    let unused = |p: &Process| p.used(state.platform, purpose).is_none();
    allowed.sort_by(|(a, ua), (b, ub)| {
        (!choices.prefer.contains(&a.id))
            .cmp(&!choices.prefer.contains(&b.id))
            .then(ua.cmp(ub))
            .then(heat(a, *ua).total_cmp(&heat(b, *ub)))
            .then(unused(a).cmp(&unused(b)))
            .then((!(filling && conformal(a))).cmp(&!(filling && conformal(b))))
    });
    for (p, unshown) in allowed.iter().skip(1) {
        let (c, c_unshown) = allowed[0];
        let why_not = if *unshown > c_unshown || heat(p, *unshown) > heat(c, c_unshown) {
            format!(
                "{}, which no source in hand shows every film already there surviving; {} {}",
                p.temperature()
                    .map_or("its temperature is not stated".into(), |(t, _)| {
                        format!("it reaches {} °C", number(t))
                    }),
                c.name,
                c.temperature()
                    .map_or("has no stated temperature".into(), |(t, _)| format!(
                        "stays at {} °C",
                        number(t)
                    ))
            )
        } else if filling && conformal(c) && !conformal(p) {
            format!(
                "its step coverage is {} where {} is conformal ({}): filling the etched \
                 openings, the conformal film follows their walls and floors evenly",
                coverage(p).map_or("not specified".into(), cited),
                c.name,
                coverage(c).map_or(String::new(), cited)
            )
        } else {
            later(p, c, state.platform, purpose)
        };
        alternatives.push(Alternative {
            process: p.id.clone(),
            name: p.name.clone(),
            why_not,
        });
    }
    let chosen = allowed.first().map(|(p, _)| *p);
    let chosen = chosen.ok_or_else(|| {
        invalid(format!(
            "no process in the library deposits {} here{}",
            film.name,
            if alternatives.is_empty() {
                String::new()
            } else {
                format!(
                    ": {}",
                    alternatives
                        .iter()
                        .map(|a| format!("{} ({})", a.name, a.why_not))
                        .collect::<Vec<_>>()
                        .join("; ")
                )
            }
        ))
    })?;
    let grown = chosen.id == "thermal-oxidation";
    let (_, shown) = if grown {
        (Vec::new(), Vec::new())
    } else {
        state.thermal(chosen, true)
    };
    if chosen.temperature().is_none() {
        for f in state.present.clone() {
            let limit = budget(&f).limit;
            if limit.is_stated() {
                state.note(format!(
                    "{}: whether {f} stays below its limit of {limit} (the temperature of the \
                     deposition is not stated by the sources in hand)",
                    chosen.name
                ));
            }
        }
    }
    let step = state.open.iter().map(|(_, h)| *h).fold(0.0f64, f64::max);
    let polish = step > 0.0 && patterned_above;
    let filled: Vec<String> = state
        .open
        .iter()
        .map(|(j, _)| layers[*j].name.clone())
        .collect();
    let mut reason = String::new();
    if !filled.is_empty() {
        reason.push_str(&format!(
            "It fills the regions etched out of {}, as the design fills them with {}. ",
            filled.join(" and "),
            film.name
        ));
    }
    if !laid.is_empty() {
        reason.push_str(&format!(
            "It lays down {}. ",
            laid.iter()
                .map(|l| format!("{} of {} ({})", um(l.thickness.to_um()), film.name, l.name))
                .collect::<Vec<_>>()
                .join(" and ")
        ));
    }
    if cladding {
        reason.push_str(&format!(
            "It is the cladding over everything, {}. ",
            film.name
        ));
    }
    if grown {
        reason.push_str(&grown_reason(chosen, &laid, state.platform));
    } else if let Some((t, c)) = chosen.temperature() {
        reason.push_str(&format!(
            "It stays at or below {} °C ({}), which the films already there are shown to \
             survive: {}.",
            number(t),
            cited(c),
            if shown.is_empty() {
                "none of them has a stated budget".into()
            } else {
                shown.join(", ")
            }
        ));
    } else {
        reason.push_str("Its temperature is not stated by the sources in hand");
        if alternatives.is_empty() {
            reason.push('.');
        } else {
            reason.push_str(&format!(
                "; every other deposition of {} the library has is refused here or shown no \
                 safer (see the alternatives).",
                film.name
            ));
        }
        if let Some(f) = chosen.facts.iter().find(|f| f.text.contains("amorphous")) {
            reason.push_str(&format!(
                " {} ({}, {})",
                f.text, f.source.reference, f.source.location
            ));
        }
    }
    if let Some(u) = chosen.used(state.platform, purpose)
        && (purpose == Purpose::Cell
            || (state.platform != Platform::Soi && state.platform != Platform::Lnoi))
    {
        reason.push_str(&format!(
            " The {} papers in hand used it for this ({}, {}).",
            if purpose == Purpose::Cell {
                "phase-change"
            } else {
                state.platform.name()
            },
            u.source.reference,
            u.source.location
        ));
    }
    if chosen
        .facts
        .iter()
        .any(|f| f.text.contains("step coverage"))
        && step > 0.0
        && !polish
    {
        reason.push_str(
            " Its step coverage is poor, so over the etched step the film isn't flat; the design \
             draws flat layers, and the 2D profile shows the difference.",
        );
    }
    let extra = if polish {
        Length::um(step)
    } else {
        Length::ZERO
    };
    let mut dep = step_of(
        chosen,
        Operation::Deposit {
            film: film.clone(),
            layers: laid,
            cladding,
            extra,
        },
        reason.trim().to_string(),
    );
    dep.alternatives = alternatives;
    if polish {
        dep.reason.push_str(&format!(
            " It is deposited {} thicker than the layer, the height of the step below, so that \
             the polish after it can leave it flat.",
            um(step)
        ));
    }
    state.push(dep);
    if polish {
        let cmp = library()
            .iter()
            .find(|p| {
                p.step == StepType::Planarization
                    && p.acts_on(&film.name)
                    && !choices.avoid.contains(&p.id)
            })
            .ok_or_else(|| {
                invalid(format!(
                    "no polish in the library takes {} flat, which the step below needs",
                    film.name
                ))
            })?;
        let why = if state.preform.is_some() {
            format!(
                "The {} lies over the oxide around the trenches too, {} thick: the polish takes it \
                 off down to the oxide, leaving the {} only in the trenches, the guides, with a \
                 flat top.",
                film.name,
                um(step),
                film.name
            )
        } else {
            format!(
                "The {} deposited over the etched step follows it, and a patterned layer comes \
                 next: lithography and the layer above need a flat surface, so the excess {} is \
                 polished off.",
                film.name,
                um(step)
            )
        };
        let mut p = step_of(
            cmp,
            Operation::Planarize {
                removed: Length::um(step),
            },
            why,
        );
        let flatness: Vec<String> = cmp
            .capabilities_of(&Quantity::Uniformity)
            .map(cited)
            .collect();
        p.uncertainties.push(format!(
            "the surface left flat to within the cited nonplanarity: {}",
            flatness.join("; ")
        ));
        state.push(p);
    }
    state.open.clear();
    state.preform = None;
    state.bare = false;
    if !state.present.contains(&film.name) {
        state.present.push(film.name.clone());
    }
    Ok(())
}

/// Why thermal oxidation grows the oxide under the guides: from the bare wafer's silicon, with
/// the time Deal and Grove's law gives in steam at 1100 °C, and whether the thickness is within
/// the range their law was fitted over.
fn grown_reason(p: &Process, laid: &[Laid], platform: Platform) -> String {
    let x: f64 = laid.iter().map(|l| l.thickness.to_um()).sum();
    let mut s = format!(
        "The oxide is grown from the bare wafer's silicon, which it consumes, in a furnace at \
         700 to 1300 °C (deal-grove-1965, abstract (PDF p. 2)); nothing else is on the wafer to \
         heat. {}",
        p.used(platform, Purpose::Under)
            .map_or(String::new(), |u| format!(
                "The {} papers in hand start from thermally oxidized wafers ({}, {}). ",
                platform.name(),
                u.source.reference,
                u.source.location
            ))
    );
    if let Some((a, b, tau)) = super::oxidation::constants(super::oxidation::Ambient::Wet, 1100.0) {
        s.push_str(&format!(
            "In steam at 1100 °C, Deal and Grove's law with their constants (A = {} µm, B = {} \
             µm²/h; Table I) grows {} in about {} hours",
            number(a),
            number(b),
            um(x),
            number(super::oxidation::hours(a, b, tau, x))
        ));
    }
    match p
        .capabilities_of(&Quantity::Thickness)
        .find_map(|c| c.value.high().map(|h| (h, c)))
    {
        Some((h, c)) if x > h + 1e-12 => s.push_str(&format!(
            ", beyond the {} their law was fitted to ({}): the time is carried past their data.",
            um(h),
            cited(c)
        )),
        _ => s.push('.'),
    }
    s
}

/// A patterned layer above the wafer's film: a metal, lifted off when it is evaporated, etched
/// otherwise.
fn metal_layer(design: &Physical, i: usize, choices: &Choices, state: &mut State) -> Result<()> {
    let s = design.structure();
    let layer = &s.stack().layers()[i];
    let (drawn, _) = &design.layers()[i];
    let film = drawn.name.clone();
    let shapes = s.shapes(&layer.name);
    if s.etches(&layer.name).iter().any(|e| !e.is_full()) {
        return Err(invalid(format!(
            "{} is drawn with partial or sloped etches, which the rules make only for the \
             wafer's film yet",
            layer.name
        )));
    }
    // a phase-change film is a cell on the guides; anything else drawn above them, a metal
    let purpose = if super::library::phase_change(&film) {
        Purpose::Cell
    } else {
        Purpose::Metal
    };
    let deposition = ordered(
        StepType::Deposition,
        Some(&film),
        state.platform,
        purpose,
        choices,
    );
    let source = deposition.first().copied().ok_or_else(|| {
        invalid(format!(
            "no process in the library deposits {film} ({} is drawn with it)",
            layer.name
        ))
    })?;
    let f = features(shapes)?;
    let lift = must("lift-off")?;
    let campbell = lift
        .facts
        .first()
        .map(|f| format!("{} ({}, {})", f.text, f.source.reference, f.source.location))
        .unwrap_or_default();
    // a film comes down in straight lines (evaporated) or its sources lifted it off as it is
    let lifted = lift
        .used(state.platform, purpose)
        .filter(|_| lift.acts_on(&film));
    if (source.directional || lifted.is_some()) && !choices.avoid.contains(&lift.id) {
        // lift-off: the resist opened, the film deposited, the resist dissolved
        let (litho, alts, t, litho_reason) =
            lithography(f, purpose, None, choices, state, &layer.name)?;
        let mut pattern = step_of(
            litho,
            Operation::Pattern {
                resist: litho.resist.clone().unwrap_or_default(),
                thickness: t,
                mask: shapes.to_vec(),
                tone: Tone::Open,
            },
            format!(
                "For a lift-off the resist is opened where the {film} of {} goes. {litho_reason}",
                layer.name
            ),
        );
        pattern.alternatives = alts;
        state.push(pattern);
        // the shapes sit on the film's top where the layer below keeps its full height there
        let on_top = |shape: &crate::geometry::Shape| {
            let b = shape.bounds();
            let (x0, x1, y0, y1) = (
                b.min.x.to_um(),
                b.max.x.to_um(),
                b.min.y.to_um(),
                b.max.y.to_um(),
            );
            let (dx, dy) = (1e-6 * (x1 - x0), 1e-6 * (y1 - y0));
            state.open.iter().all(|(j, _)| {
                let below = &s.stack().layers()[*j];
                [
                    (x0 + dx, y0 + dy),
                    (x1 - dx, y0 + dy),
                    (x0 + dx, y1 - dy),
                    (x1 - dx, y1 - dy),
                ]
                .iter()
                .all(|&(x, y)| {
                    s.kept(&below.name, crate::geometry::Point::um(x, y))
                        .is_some_and(|k| (k.to_um() - below.thickness.to_um()).abs() < 1e-12)
                })
            })
        };
        if !state.open.is_empty() && !(purpose == Purpose::Cell && shapes.iter().all(on_top)) {
            state.push_note_flat(&layer.name);
        }
        let (violations, _) = state.thermal(source, true);
        if !violations.is_empty() {
            state.warn(format!("{}: {}", source.name, violations.join("; ")));
        }
        let mut ev = if source.directional {
            step_of(
                source,
                Operation::Evaporate {
                    film: drawn.clone(),
                    thickness: layer.thickness,
                },
                format!(
                    "{film} is evaporated, {} thick: it comes down in straight lines, onto the \
                     resist and into its openings, which is what lift-off needs.",
                    um(layer.thickness.to_um())
                ),
            )
        } else {
            // the same operation as an evaporation: the film onto the opened resist; the record
            // names it a sputtering, its process not being directional
            step_of(
                source,
                Operation::Evaporate {
                    film: drawn.clone(),
                    thickness: layer.thickness,
                },
                format!(
                    "{film} is sputtered, {} thick, over the resist and into its openings. \
                     Sputtering covers walls too, which lift-off doesn't like, but a film this \
                     thin has been lifted off as it is ({}, {}).",
                    um(layer.thickness.to_um()),
                    lifted.map_or("", |u| u.source.reference.as_str()),
                    lifted.map_or("", |u| u.source.location.as_str())
                ),
            )
        };
        if let Some(u) = source.used(state.platform, purpose) {
            ev.reason.push_str(&format!(
                " The papers in hand made such {} this way ({}, {}).",
                if purpose == Purpose::Cell {
                    "cells"
                } else {
                    "electrodes"
                },
                u.source.reference,
                u.source.location
            ));
        }
        // a sputtered film thicker than any its sources lifted off
        let thickest = source
            .capabilities_of(&Quantity::Thickness)
            .filter(|c| c.value.unit() == Some(Unit::Nm))
            .filter_map(|c| c.value.high().map(|v| (v, c)))
            .fold(
                None,
                |best: Option<(f64, &Capability)>, (v, c)| match best {
                    Some((b, _)) if b >= v => best,
                    _ => Some((v, c)),
                },
            );
        if !source.directional
            && let Some((v, c)) = thickest
            && layer.thickness.to_um() * 1000.0 > v + 1e-9
        {
            ev.uncertainties.push(format!(
                "the design's {} is thicker than any film its sources sputtered and lifted off \
                 (at most {}): whether it lifts off as cleanly is not specified",
                um(layer.thickness.to_um()),
                cited(c)
            ));
        }
        state.push(ev);
        let mut off = step_of(
            lift,
            Operation::LiftOff {
                layer: layer.name.clone(),
            },
            format!(
                "Dissolving the resist takes away the {film} on it; the {film} in the openings is \
                 the design's {}. {}",
                layer.name,
                if source.directional {
                    campbell.clone()
                } else {
                    // the cells' own sources, which lifted off a sputtered film
                    lift.facts
                        .iter()
                        .filter(|f| {
                            f.source.reference != "campbell-2008"
                                && lifted.is_some_and(|u| u.platform == state.platform)
                        })
                        .map(|f| {
                            format!("{} ({}, {})", f.text, f.source.reference, f.source.location)
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            ),
        );
        off.uncertainties.push(if source.directional {
            "the resist-to-metal thickness ratio lift-off needs is not specified".into()
        } else {
            format!("the resist-to-{film} thickness ratio lift-off needs is not specified")
        });
        let etchers: Vec<&Process> = library()
            .iter()
            .filter(|p| p.step == StepType::PatternTransfer && p.id != lift.id && p.acts_on(&film))
            .collect();
        state.open.push((i, layer.thickness.to_um()));
        off.alternatives.push(Alternative {
            process: "etch".into(),
            name: format!("{film} deposited everywhere and etched"),
            why_not: if etchers.is_empty() {
                format!("the library has no etch of {film}")
            } else {
                "evaporated metal lifts off without etching what lies below".into()
            },
        });
        state.push(off);
    } else {
        // a blanket film, patterned and etched
        if !state.open.is_empty() {
            return Err(invalid(format!(
                "{} is deposited over everything while {} is still etched open: the regions \
                 would fill with {film}",
                layer.name,
                state
                    .open
                    .iter()
                    .map(|(j, _)| s.stack().layers()[*j].name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        let (violations, shown) = state.thermal(source, true);
        if !violations.is_empty() {
            state.warn(format!("{}: {}", source.name, violations.join("; ")));
        }
        let mut dep = step_of(
            source,
            Operation::Deposit {
                film: drawn.clone(),
                layers: vec![Laid {
                    name: layer.name.clone(),
                    thickness: layer.thickness,
                }],
                cladding: false,
                extra: Length::ZERO,
            },
            format!(
                "{} of {film} is deposited over the whole wafer, to be patterned by etching: {} \
                 isn't evaporated, and lift-off needs an evaporated film.{}",
                um(layer.thickness.to_um()),
                source.name,
                if shown.is_empty() {
                    String::new()
                } else {
                    format!(
                        " The films already there are shown to survive it: {}.",
                        shown.join(", ")
                    )
                }
            ),
        );
        dep.alternatives.push(Alternative {
            process: lift.id.clone(),
            name: lift.name.clone(),
            why_not: format!(
                "{campbell} Still, a thin sputtered film has been lifted off: {} ({}, {}).",
                lift.facts.get(1).map_or("", |f| f.text.as_str()),
                lift.facts
                    .get(1)
                    .map_or("", |f| f.source.reference.as_str()),
                lift.facts.get(1).map_or("", |f| f.source.location.as_str())
            ),
        });
        state.push(dep);
        if !state.present.contains(&film) {
            state.present.push(film.clone());
        }
        let etching = ordered(
            StepType::PatternTransfer,
            Some(&film),
            state.platform,
            Purpose::Metal,
            choices,
        );
        let etch = *etching
            .iter()
            .find(|p| p.id != lift.id)
            .ok_or_else(|| invalid(format!("no process in the library etches {film}")))?;
        let (litho, alts, t, litho_reason) = lithography(
            f,
            Purpose::Metal,
            Some((etch, layer.thickness.to_um())),
            choices,
            state,
            &layer.name,
        )?;
        let mut pattern = step_of(
            litho,
            Operation::Pattern {
                resist: litho.resist.clone().unwrap_or_default(),
                thickness: t,
                mask: shapes.to_vec(),
                tone: Tone::Keep,
            },
            format!(
                "The resist must stay on the {film} of {} that the etch keeps. {litho_reason}",
                layer.name
            ),
        );
        pattern.alternatives = alts;
        state.push(pattern);
        let below = design.layers()[i - 1].0.name.clone();
        let stop = etch.stops_on.iter().find(|st| st.film == below);
        let depth = if stop.is_some() {
            Depth::ToStop
        } else {
            Depth::Timed(layer.thickness)
        };
        let mut e = step_of(
            etch,
            Operation::Etch {
                layer: layer.name.clone(),
                depth,
                sidewall: std::f64::consts::FRAC_PI_2,
            },
            format!(
                "The {film} is etched through where the resist leaves it bare.{}",
                if stop.is_some() {
                    String::new()
                } else {
                    format!(
                        " No source in hand says the etch stops on the {below} below, so it is \
                         timed."
                    )
                }
            ),
        );
        if stop.is_none() {
            e.uncertainties.push(format!(
                "the timed etch through {}: its uncertainty is not specified",
                um(layer.thickness.to_um())
            ));
        }
        state.push(e);
        let strip = must("resist-strip")?;
        state.push(step_of(
            strip,
            Operation::Strip,
            "The resist has done its work as the etch's mask and is removed.".into(),
        ));
        state.open.push((i, layer.thickness.to_um()));
    }
    if !state.present.contains(&film) {
        state.present.push(film);
    }
    Ok(())
}

impl State {
    /// A note that the design's flat layers put a layer over etched regions the wafer has not
    /// filled: the metal sits on the etched film there, as the 2D profile shows.
    fn push_note_flat(&mut self, layer: &str) {
        self.warn(format!(
            "{layer}: the design's layers are flat, so it draws {layer} over the film's top; on \
             the wafer it sits on the etched surface beside the guides, where it is lifted off, \
             and the 2D profile shows it there"
        ));
    }
}
