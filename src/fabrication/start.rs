//! Where a flow starts: from its wafer as bought, every flow's start unless asked otherwise; or,
//! for silicon on insulator, from bare silicon, with the Smart Cut or SIMOX steps that make the
//! SOI wafer ([`super::smart_cut`], [`super::simox`]) in place of the bought wafer. The rest of
//! the flow is the same.

use serde::{Deserialize, Serialize};

use crate::units::Length;
use crate::{Error, Result};

use super::flow::{Flow, Laid, Operation, Step};
use super::generate::{Choices, generate};
use super::library::{Process, process, route_only};
use super::physical::Film;
use super::simox;
use super::smart_cut::{self, Route};
use super::stated::number;

/// Where a flow starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Start {
    /// From the wafer as bought, with its first layers: silicon or lithium niobate on insulator,
    /// GaAs grown on its wafer, or bare silicon for a deposited guide film.
    #[default]
    Bought,
    /// For silicon on insulator, from bare silicon wafers: the donor oxidized, implanted with
    /// hydrogen, bonded to a handle, split, annealed, polished and thinned to the design's film,
    /// before the steps on the bought wafer.
    SmartCut,
    /// For silicon on insulator, from one bare silicon wafer: oxygen implanted into it hot and
    /// annealed above 1300 °C into a buried oxide under a film of its silicon, which a
    /// sacrificial oxide thins to the design's, before the steps on the bought wafer
    /// ([`super::simox`]).
    Simox,
}

impl Start {
    /// Its name, as a job writes it: `"bought"`, `"smart_cut"` or `"simox"`.
    pub fn name(self) -> &'static str {
        match self {
            Start::Bought => "bought",
            Start::SmartCut => "smart_cut",
            Start::Simox => "simox",
        }
    }

    /// The start named `text`, as [`Start::name`] writes it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for any other name.
    pub fn parse(text: &str) -> Result<Start> {
        [Start::Bought, Start::SmartCut, Start::Simox]
            .into_iter()
            .find(|s| s.name() == text)
            .ok_or_else(|| {
                invalid(format!(
                    "a flow starts \"bought\", \"smart_cut\" or \"simox\", not {text:?}"
                ))
            })
    }
}

fn invalid(message: String) -> Error {
    Error::invalid("fabrication", message)
}

/// The processes the Smart Cut route uses: the library's bare wafer and thermal oxidation, and
/// its own.
const NEEDED: [&str; 2] = ["si-wafer", "thermal-oxidation"];

/// The processes the SIMOX route uses beside its own: the library's bare wafer, and the
/// thermal oxidation and its strip that thin the film.
const SIMOX_NEEDED: [&str; 3] = ["si-wafer", "thermal-oxidation", "hf-oxide-strip"];

/// The SIMOX route's own processes.
const SIMOX_OWN: [&str; 2] = ["o-implant", "simox-anneal"];

/// The flow that makes `design`, as `choices` ask, from `start`: [`generate`]'s for
/// [`Start::Bought`]; for [`Start::SmartCut`], the same with the Smart Cut route in place of the
/// bought SOI wafer.
///
/// # Errors
///
/// [`generate`]'s; for [`Start::SmartCut`], a design whose wafer isn't silicon on insulator,
/// choices that avoid a step the route needs, and [`smart_cut::route`]'s.
pub fn generate_from(design: &super::Physical, choices: &Choices, start: Start) -> Result<Flow> {
    match start {
        Start::Bought => generate(design, choices),
        Start::SmartCut | Start::Simox => made(design, choices, start),
    }
}

fn made(design: &super::Physical, choices: &Choices, start: Start) -> Result<Flow> {
    choices.check()?;
    let (what, needed): (&str, &[&str]) = if start == Start::Simox {
        ("SIMOX", &SIMOX_NEEDED)
    } else {
        ("Smart Cut", &NEEDED)
    };
    // each route needs its own steps, not the other's
    let own = |id: &String| {
        process(id).is_some_and(route_only) && SIMOX_OWN.contains(&id.as_str()) == (what == "SIMOX")
    };
    if let Some(id) = choices
        .avoid
        .iter()
        .find(|id| own(id) || needed.contains(&id.as_str()))
    {
        return Err(invalid(format!(
            "the {what} start needs \"{id}\", which the choices avoid"
        )));
    }
    // the generator chooses none of the route's own steps: their preferences are the route's
    let flow = generate(design, &base(choices))?;
    let (substrate, [(oxide, oxide_film), (film, silicon)]) = soi_wafer(&flow)? else {
        return Err(invalid(
            "the SOI wafer has a film and its buried oxide, no more".into(),
        ));
    };
    let (f, o) = (film.thickness.to_um(), oxide.thickness.to_um());
    let (mut steps, note) = if start == Start::Simox {
        let route = simox::route(f, o)?;
        (
            simox_steps(&route, substrate, (oxide, oxide_film), (film, silicon))?,
            "SIMOX: the film the anneal leaves over the oxide, for any dose of 1.2–2.2×10¹⁸ \
             cm⁻² (the flow takes Celler and Cristoloveanu's about 200 nm at 200 keV; Lam gives \
             100–300 nm over those doses), and so what the sacrificial oxide takes off",
        )
    } else {
        let route = smart_cut::route(f, o)?;
        (
            route_steps(&route, substrate, (oxide, oxide_film), (film, silicon))?,
            "Smart Cut: how much the touch-up polish and the sacrificial oxide take off (no \
             source in hand states it; the flow takes one straggle each)",
        )
    };
    let mut not_specified = flow.not_specified;
    not_specified.push(note.into());
    steps.extend(flow.steps.into_iter().skip(1));
    Ok(Flow {
        steps,
        warnings: flow.warnings,
        not_specified,
    })
}

/// The bought SOI wafer `flow` starts from: its substrate and its layers.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a flow that starts from another wafer.
fn soi_wafer(flow: &Flow) -> Result<(&Film, &[(Laid, Film)])> {
    let first = flow
        .steps
        .first()
        .ok_or_else(|| invalid("the flow has no steps".into()))?;
    match (&first.operation, first.process.as_str()) {
        (Operation::Wafer { substrate, layers }, "soi-smart-cut") => Ok((substrate, layers)),
        _ => Err(invalid(format!(
            "a start from bare silicon makes silicon on insulator; this design's flow starts \
             from {}",
            first.name
        ))),
    }
}

/// The starts a design can be made from, with the routes to it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Starts {
    /// The starts: the wafer as bought, then those of the routes below that make its wafer.
    pub starts: Vec<Start>,
    /// The Smart Cut route, when it makes the wafer.
    pub smart_cut: Option<Route>,
    /// The SIMOX route, when it makes the wafer.
    pub simox: Option<simox::Route>,
    /// Why SIMOX doesn't make it, for silicon on insulator it doesn't: [`simox::route`]'s words.
    pub simox_refused: Option<String>,
}

/// The starts `design` can be made from as `choices` ask: the wafer as bought; Smart Cut when
/// that wafer is silicon on insulator, a film on its buried oxide, which a proton energy of the
/// range table reaches; and SIMOX when its published conditions make that film and oxide
/// ([`simox::route`]); with the routes to it.
///
/// # Errors
///
/// [`generate`]'s.
pub(crate) fn starts(design: &super::Physical, choices: &Choices) -> Result<Starts> {
    let flow = generate(design, &base(choices))?;
    let mut s = Starts {
        starts: vec![Start::Bought],
        smart_cut: None,
        simox: None,
        simox_refused: None,
    };
    if let Ok((_, [(oxide, _), (film, _)])) = soi_wafer(&flow) {
        let (f, o) = (film.thickness.to_um(), oxide.thickness.to_um());
        s.smart_cut = smart_cut::route(f, o).ok();
        if s.smart_cut.is_some() {
            s.starts.push(Start::SmartCut);
        }
        match simox::route(f, o) {
            Ok(r) => {
                s.simox = Some(r);
                s.starts.push(Start::Simox);
            }
            Err(e) => s.simox_refused = Some(e.to_string()),
        }
    }
    Ok(s)
}

/// `choices` for the generator under the route: the route's own steps are the route's, so a
/// preference for one is dropped.
fn base(choices: &Choices) -> Choices {
    let own = |id: &String| process(id).is_some_and(route_only);
    Choices {
        prefer: choices
            .prefer
            .iter()
            .filter(|id| !own(id))
            .cloned()
            .collect(),
        avoid: choices.avoid.clone(),
    }
}

fn must(id: &str) -> Result<&'static Process> {
    process(id).ok_or_else(|| invalid(format!("the library has no process {id}")))
}

fn step_of(id: &str, operation: Operation, reason: String) -> Result<Step> {
    let p = must(id)?;
    Ok(Step {
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
    })
}

fn um(v: f64) -> String {
    format!("{} µm", number(v))
}

/// The route's steps, from the bare donor to the wafer as it would have been bought: `oxide`
/// and `film` are the bought wafer's layers, over `substrate`.
fn route_steps(
    r: &Route,
    substrate: &Film,
    oxide: (&Laid, &Film),
    film: (&Laid, &Film),
) -> Result<Vec<Step>> {
    let (o, f) = (oxide.0.thickness.to_um(), film.0.thickness.to_um());
    let donor = step_of(
        "si-wafer",
        Operation::Wafer {
            substrate: substrate.clone(),
            layers: Vec::new(),
        },
        "The flow makes its SOI wafer instead of buying it, as its start asks, by Smart Cut \
         (bruel-1995, Fig. 1): from two bare silicon wafers. This one, the donor, gives the film; the second, the \
         handle, is bonded to it and carries everything after. The drawing follows the donor \
         until the bond, then the handle."
            .into(),
    )?;
    let oxidize = step_of(
        "thermal-oxidation",
        Operation::Deposit {
            film: oxide.1.clone(),
            layers: vec![oxide.0.clone()],
            cladding: false,
            extra: Length::ZERO,
        },
        format!(
            "The donor is oxidized to the buried oxide's {}: {} hours in wet oxygen at 1100 °C by \
             Deal and Grove's law (deal-grove-1965, Table I, Eq. 12). The oxide grows from the \
             silicon, consuming {} of it (about 44 % of the oxide's thickness: campbell-2008, p. \
             74). It becomes the buried oxide, and the hydrogen is implanted through it, as Bruel \
             prefers (bruel-1995, Fig. 1, step 1).",
            um(o),
            number(r.oxide_hours),
            um(r.oxide_consumed)
        ),
    )?;
    let mut implant = step_of(
        "h-implant",
        Operation::Undrawn {
            what: format!(
                "implant H⁺ at {} keV through the {} of oxide: its peak {} below the oxide",
                number(r.energy),
                um(o),
                um(r.split)
            ),
        },
        format!(
            "The energy, {} keV, puts the hydrogen's peak {} below the oxide: the protons cross \
             the oxide, leaving it with {} keV (its CSDA range), and stop around silicon's \
             projected range at that energy (pstar-1993, ICRU Report 49's ranges). The dose, \
             from 2×10¹⁶ to 1×10¹⁷ cm⁻², sets no depth (bruel-1995, PDF p. 2). The {} of \
             silicon above the peak is the design's film, {}, and two straggles of {} \
             (campbell-2008, Eq. 5.19, over the peak's whole depth of {}): the silicon the \
             hydrogen damages near the split, which the polish and a sacrificial oxide take off.",
            number(r.energy),
            um(r.split),
            number(r.entering),
            um(r.split),
            um(f),
            um(r.straggle),
            um(r.range)
        ),
    )?;
    implant.uncertainties = vec![
        "the straggle is Campbell's estimate from the masses (Eq. 5.19), not a table's".into(),
        "the split is taken at the implanted peak".into(),
        "the protons' path through the oxide is taken as straight (its CSDA range)".into(),
    ];
    let bond = step_of(
        "direct-bonding",
        Operation::Deposit {
            film: film.1.clone(),
            layers: vec![film.0.clone()],
            cladding: false,
            extra: Length::um(r.excess),
        },
        format!(
            "The donor is turned over and bonded, oxide down, onto the handle at room \
             temperature (bruel-1995, Fig. 1, step 2). From here the handle is the wafer below, \
             and the donor's {} of silicon between the oxide and the implanted plane is the film \
             the split will leave. The drawing shows the design's {} of it; the {} beyond, which \
             the polish and the sacrificial oxide take off, isn't drawn, as a deposit's excess \
             before its polish isn't, nor is the rest of the donor.",
            um(r.split),
            um(f),
            um(r.excess)
        ),
    )?;
    let split = step_of(
        "smart-cut-split",
        Operation::Undrawn {
            what: format!(
                "split along the implanted plane: {} of silicon stays on the oxide",
                um(r.split)
            ),
        },
        "Heated to 400–600 °C, the donor splits along the implanted plane; the film stays on the \
         handle and the rest of the donor is recycled (bruel-1995, Fig. 1, step 3)."
            .into(),
    )?;
    let anneal = step_of(
        "anneal-bond",
        Operation::Undrawn {
            what: "the bond strengthened".into(),
        },
        "Then above 1000 °C, the heat treatment's second phase, which strengthens the bond \
         (bruel-1995, Fig. 1, step 3)."
            .into(),
    )?;
    let mut polish = step_of(
        "cmp-si",
        Operation::Planarize {
            removed: Length::um(r.polish),
        },
        format!(
            "The split surface is micro-rough, and its silicon damaged by the hydrogen: a fine \
             polish (bruel-1995, Fig. 1, step 4) takes {} off, one straggle, leaving {}.",
            um(r.polish),
            um(r.split - r.polish)
        ),
    )?;
    polish.uncertainties.push(
        "how much the polish takes off: no source in hand states it; the flow takes one straggle"
            .into(),
    );
    let mut thin = step_of(
        "thermal-oxidation",
        Operation::Planarize {
            removed: Length::um(r.consumed),
        },
        format!(
            "The film is thinned to the design's {} by a sacrificial oxide: {} grown in wet \
             oxygen at 1100 °C, {} hours (deal-grove-1965, Table I), consumes {} of the silicon \
             (about 44 %: campbell-2008, p. 74), the second straggle of damaged silicon. The \
             oxide isn't drawn; it comes off next.",
            um(f),
            um(r.sacrificial),
            number(r.sacrificial_hours),
            um(r.consumed)
        ),
    )?;
    thin.uncertainties.push(
        "how much the sacrificial oxide takes off: no source in hand states it; the flow takes \
         one straggle"
            .into(),
    );
    let strip = step_of(
        "hf-oxide-strip",
        Operation::Undrawn {
            what: format!(
                "strip the {} sacrificial oxide in HF: {} of silicon on {} of buried oxide",
                um(r.sacrificial),
                um(f),
                um(o)
            ),
        },
        format!(
            "The sacrificial oxide is dissolved in 6:1 hydrofluoric acid, about {} minutes at \
             about 1200 Å/min; the acid etches oxide more than 100 times faster than silicon, so \
             the film keeps its thickness (campbell-2008, Section 11.1, p. 285). The wafer is now \
             the one a flow would buy: {} of silicon on {} of oxide over a silicon handle.",
            number(r.strip_minutes),
            um(f),
            um(o)
        ),
    )?;
    Ok(vec![
        donor, oxidize, implant, bond, split, anneal, polish, thin, strip,
    ])
}

/// `v` as a power of ten, as the narration writes a dose: `1.76×10¹⁸`.
fn power(v: f64) -> String {
    const SUP: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    let e = v.abs().log10().floor() as i32;
    let digits: String = e
        .unsigned_abs()
        .to_string()
        .chars()
        .map(|c| SUP[c.to_digit(10).unwrap_or(0) as usize])
        .collect();
    let sign = if e < 0 { "⁻" } else { "" };
    format!("{}×10{sign}{digits}", number(v / 10f64.powi(e)))
}

/// The SIMOX route's steps, from the bare wafer to the wafer as it would have been bought:
/// `oxide` and `film` are the bought wafer's layers, over `substrate`. The drawing builds the
/// wafer from the bottom, as every flow's does: the implant draws the buried oxide it makes, and
/// the anneal the film it heals over it, with what the sacrificial oxide then takes off.
fn simox_steps(
    r: &simox::Route,
    substrate: &Film,
    oxide: (&Laid, &Film),
    film: (&Laid, &Film),
) -> Result<Vec<Step>> {
    let (o, f) = (oxide.0.thickness.to_um(), film.0.thickness.to_um());
    let (lo, hi) = simox::oxides();
    let wafer = step_of(
        "si-wafer",
        Operation::Wafer {
            substrate: substrate.clone(),
            layers: Vec::new(),
        },
        "The flow makes its SOI wafer instead of buying it, as its start asks, by SIMOX, \
         separation by implanted oxygen (izumi-1978): from one bare silicon wafer, whose own \
         silicon becomes both the film and, below the buried oxide, the handle."
            .into(),
    )?;
    let mut implant = step_of(
        "o-implant",
        Operation::Deposit {
            film: oxide.1.clone(),
            layers: vec![oxide.0.clone()],
            cladding: false,
            extra: Length::ZERO,
        },
        format!(
            "Oxygen ions are implanted at {} keV to {} cm⁻², the wafer held at {} °C so its \
             surface stays single-crystal (celler-cristoloveanu-2003, p. 4959; lam-1987, p. 6: \
             150–200 keV, 1.2–2.2×10¹⁸ cm⁻², 400 °C or higher). The dose is the buried oxide's \
             oxygen: {} of silicon dioxide holds the dose at 4.4×10²² oxygen atoms per cm³, \
             twice silicon's 5×10²² atoms per cm³ (campbell-2008, p. 43) times the 44 % of its \
             thickness an oxide consumes of silicon (p. 74). The oxide takes the place of {} of \
             silicon and the surface rises by the other {}. The energy is the one whose film a \
             source gives, about 0.2 µm at 200 keV (celler-cristoloveanu-2003, p. 4959); no \
             range table is in hand, so the depth isn't computed. The drawing shows the buried \
             oxide here, as an implant this heavy already makes one (hemment-1984, p. 204), and \
             the silicon over it at the anneal, which heals it.",
            number(r.energy),
            power(r.dose),
            number(r.implant_temperature),
            um(o),
            um(r.consumed),
            um(r.swelling)
        ),
    )?;
    implant.uncertainties = vec![
        format!(
            "the dose balance holds after an anneal above 1300 °C, where only the planar oxide \
             remains (celler-cristoloveanu-2003, p. 4959); the route's oxides, {} to {}, are \
             those of Lam's doses",
            um(lo),
            um(hi)
        ),
        "the depth of the oxide is not computed: no range table of oxygen in silicon is in hand"
            .into(),
    ];
    let mut anneal = step_of(
        "simox-anneal",
        Operation::Deposit {
            film: film.1.clone(),
            layers: vec![film.0.clone()],
            cladding: false,
            extra: Length::um(r.thinned),
        },
        format!(
            "The wafer is annealed at {} °C for {} hours under an oxide cap (campbell-2008, p. \
             129: at least 1300 °C for several hours; celler-cristoloveanu-2003, pp. \
             4959–4960: ∼1350 °C, 4 h). Above 1300 °C the oxide precipitates dissolve and only \
             the planar buried oxide remains, and the damaged silicon over it heals into a \
             single-crystal film, about {} thick at 200 keV (celler-cristoloveanu-2003, p. 4959; \
             lam-1987, p. 6: 100–300 nm). The drawing shows the design's {} of it; the {} \
             beyond, which a sacrificial oxide takes off, isn't drawn.",
            number(r.anneal_temperature),
            number(r.anneal_hours),
            um(r.made),
            um(f),
            um(r.thinned)
        ),
    )?;
    anneal.uncertainties = vec![
        "the film as annealed is Celler and Cristoloveanu's about 200 nm at 200 keV for every \
         dose of the route; Lam gives 100–300 nm over them"
            .into(),
    ];
    let mut steps = vec![wafer, implant, anneal];
    if r.thinned > 0.0 {
        let mut thin = step_of(
            "thermal-oxidation",
            Operation::Planarize {
                removed: Length::um(r.thinned),
            },
            format!(
                "The film is thinned to the design's {} by a sacrificial oxide, as Izumi thinned \
                 his (izumi-1978, p. 593: 600 Å grown and etched off took 300 Å of the surface): \
                 {} grown in wet oxygen at 1100 °C, {} hours (deal-grove-1965, Table I), \
                 consumes {} of the silicon (about 44 %: campbell-2008, p. 74). The oxide isn't \
                 drawn; it comes off next.",
                um(f),
                um(r.sacrificial),
                number(r.sacrificial_hours),
                um(r.thinned)
            ),
        )?;
        thin.uncertainties.push(
            "what it takes off rests on the film as annealed, about 0.2 µm (100–300 nm: \
             lam-1987, p. 6)"
                .into(),
        );
        let strip = step_of(
            "hf-oxide-strip",
            Operation::Undrawn {
                what: format!(
                    "strip the {} sacrificial oxide in HF: {} of silicon on {} of buried oxide",
                    um(r.sacrificial),
                    um(f),
                    um(o)
                ),
            },
            format!(
                "The sacrificial oxide is dissolved in 6:1 hydrofluoric acid, about {} minutes at \
                 about 1200 Å/min, leaving the film (campbell-2008, Section 11.1, p. 285). The \
                 wafer is now the one a flow would buy: {} of silicon on {} of oxide over a \
                 silicon handle.",
                number(r.strip_minutes),
                um(f),
                um(o)
            ),
        )?;
        steps.extend([thin, strip]);
    }
    Ok(steps)
}
