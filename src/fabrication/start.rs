//! Where a flow starts: from its wafer as bought, every flow's start unless asked otherwise; or,
//! for silicon on insulator, from bare silicon, with the Smart Cut steps that make the SOI wafer
//! ([`super::smart_cut`]) in place of the bought wafer. The rest of the flow is the same.

use serde::{Deserialize, Serialize};

use crate::units::Length;
use crate::{Error, Result};

use super::flow::{Flow, Laid, Operation, Step};
use super::generate::{Choices, generate};
use super::library::{Process, process, route_only};
use super::physical::Film;
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
}

impl Start {
    /// Its name, as a job writes it: `"bought"` or `"smart_cut"`.
    pub fn name(self) -> &'static str {
        match self {
            Start::Bought => "bought",
            Start::SmartCut => "smart_cut",
        }
    }

    /// The start named `text`, as [`Start::name`] writes it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for any other name.
    pub fn parse(text: &str) -> Result<Start> {
        [Start::Bought, Start::SmartCut]
            .into_iter()
            .find(|s| s.name() == text)
            .ok_or_else(|| {
                invalid(format!(
                    "a flow starts \"bought\" or \"smart_cut\", not {text:?}"
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
        Start::SmartCut => made(design, choices),
    }
}

fn made(design: &super::Physical, choices: &Choices) -> Result<Flow> {
    choices.check()?;
    let own = |id: &String| process(id).is_some_and(route_only);
    if let Some(id) = choices
        .avoid
        .iter()
        .find(|id| own(id) || NEEDED.contains(&id.as_str()))
    {
        return Err(invalid(format!(
            "the Smart Cut start needs \"{id}\", which the choices avoid"
        )));
    }
    // the generator chooses none of the route's own steps: their preferences are the route's
    let base = Choices {
        prefer: choices
            .prefer
            .iter()
            .filter(|id| !own(id))
            .cloned()
            .collect(),
        avoid: choices.avoid.clone(),
    };
    let flow = generate(design, &base)?;
    let first = flow
        .steps
        .first()
        .ok_or_else(|| invalid("the flow has no steps".into()))?;
    let (Operation::Wafer { substrate, layers }, "soi-smart-cut") =
        (&first.operation, first.process.as_str())
    else {
        return Err(invalid(format!(
            "the Smart Cut start makes silicon on insulator; this design's flow starts from {}",
            first.name
        )));
    };
    let [(oxide, oxide_film), (film, silicon)] = layers.as_slice() else {
        return Err(invalid(
            "the SOI wafer has a film and its buried oxide, no more".into(),
        ));
    };
    let route = smart_cut::route(film.thickness.to_um(), oxide.thickness.to_um())?;
    let mut steps = route_steps(&route, substrate, (oxide, oxide_film), (film, silicon))?;
    let mut not_specified = flow.not_specified;
    not_specified.push(
        "Smart Cut: how much the touch-up polish and the sacrificial oxide take off (no source in \
         hand states it; the flow takes one straggle each)"
            .into(),
    );
    steps.extend(flow.steps.into_iter().skip(1));
    Ok(Flow {
        steps,
        warnings: flow.warnings,
        not_specified,
    })
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
