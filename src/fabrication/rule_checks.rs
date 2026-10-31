//! The rule engine's validation cases (F4 of #622): the Rayleigh criterion against a textbook's
//! table, Deal and Grove's law against a textbook's worked example, the library's minimum
//! features against the Rayleigh limit, the rules' choices on cases with textbook answers, the
//! accuracy read-out's combination, and the round trip over every library device the rules make.

use crate::device::{Device, ThermoShifter, Values};
use crate::geometry::{Point, Shape};
use crate::stack::{Etch, LayerStack, Structure};
use crate::units::Length;
use crate::validation::{Case, Outcome, Tier};

use super::checks::{first_etch, soi_strip};
use super::oxidation::{Ambient, printed};
use super::rayleigh::{Optics, Reach, resolution};
use super::rules::k1_limit;
use super::{
    Choices, Depth, Kind, Physical, Quantity, Rule, Verdict, assess, generate, library, oxidation,
    process, round_trip, thermal,
};

fn exact(count: usize) -> Outcome {
    let e = count as f64;
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: 0.0,
        error: e,
    }
}

/// Franssila's Table 10.1 (p. 117): wavelength nm, NA, k₁, the printed linewidth µm. The fourth
/// row (248 nm, NA 0.65, k₁ 0.5, printed 0.18 µm) is left out: k₁λ/NA gives 0.1908 µm, outside
/// half a unit of its last digit, and the table says nothing that would explain it (reported).
pub(crate) const FRANSSILA: [(f64, f64, f64, &str); 3] = [
    (436.0, 0.38, 0.8, "1"),
    (365.0, 0.48, 0.6, "0.5"),
    (248.0, 0.60, 0.6, "0.25"),
];

/// For each row, |k₁λ/NA − printed| over half a unit of the printed linewidth's last digit.
pub(crate) fn franssila_misfits() -> Vec<f64> {
    FRANSSILA
        .iter()
        .filter_map(|&(wl, na, k1, w)| {
            let (v, half) = printed(w)?;
            Some((resolution(k1, wl, na) / 1000.0 - v).abs() / half)
        })
        .collect()
}

fn rayleigh_franssila() -> Outcome {
    let worst = franssila_misfits().into_iter().fold(0.0, f64::max);
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1.0,
        error: worst,
    }
}

/// Campbell's Example 4.1 (pp. 80–81): 120 min of steam oxidation at 920 °C on 1000 Å of oxide.
/// Our (τ h, total oxide µm).
pub(crate) fn campbell_example() -> (f64, f64) {
    let (a, b, _) = oxidation::constants(Ambient::Wet, 920.0).unwrap_or((f64::NAN, f64::NAN, 0.0));
    let x = 0.1;
    let tau = (x * x + a * x) / b;
    (tau, oxidation::thickness(a, b, tau, 2.0))
}

fn deal_grove_campbell() -> Outcome {
    let (_, x) = campbell_example();
    Outcome {
        measured: x,
        expected: 0.48,
        tolerance: 0.005,
        error: (x - 0.48).abs(),
    }
}

/// The projection lithographies' cited minimum features that ask for a k₁ below the theoretical
/// limit at the class's highest cited NA: none should.
pub(crate) fn below_limit() -> Vec<String> {
    let limit = k1_limit().low().unwrap_or(0.25);
    let mut out = Vec::new();
    for p in library() {
        let Some(o) = Optics::of(p) else {
            continue;
        };
        for c in p.capabilities_of(&Quantity::MinFeature) {
            let Some(v) = c.value.low().or(c.value.high()) else {
                continue;
            };
            let nm = match c.value.unit() {
                Some(super::Unit::Um) => v * 1000.0,
                _ => v,
            };
            if o.k1(nm) < limit - 1e-12 {
                out.push(format!("{}: {nm} nm", p.id));
            }
        }
    }
    out
}

fn min_features_above_limit() -> Outcome {
    exact(below_limit().len())
}

/// Two strips `gap` µm apart on 220 nm SOI, each 0.5 µm wide.
pub(crate) fn soi_pair(gap: f64) -> Physical {
    let mut s = Structure::new(LayerStack::soi_220());
    for y in [-(0.25 + gap / 2.0), 0.25 + gap / 2.0] {
        let strip = Shape::rect(Point::um(5.0, y), Length::um(10.0), Length::um(0.5))
            .expect("a positive width");
        s.draw_etched("Si", strip, Etch::FULL)
            .expect("on the Si layer");
    }
    Physical::of(s)
}

fn verdict_of(design: &Physical, choices: &Choices, rule: Rule) -> Option<Verdict> {
    let flow = generate(design, choices).ok()?;
    let a = assess(design, &flow).ok()?;
    a.checks.iter().find(|c| c.rule == rule).map(|c| c.verdict)
}

/// The rules' choices on cases with textbook answers; each wrong answer counted, with what.
pub(crate) fn rule_choices_wrong() -> Vec<&'static str> {
    let mut wrong = Vec::new();
    let prefer = |ids: &[&str]| Choices {
        prefer: ids.iter().map(|s| (*s).to_string()).collect(),
        avoid: Vec::new(),
    };
    // Rayleigh: 193 nm immersion (NA up to 1.35) prints 100 nm as a matter of course, 45 nm only
    // with enhancement, 30 nm not at all
    match process("duv-193i").and_then(Optics::of) {
        Some(o) => {
            if o.reach(100.0) != Reach::Typical {
                wrong.push("193i at 100 nm");
            }
            if o.reach(45.0) != Reach::Enhanced {
                wrong.push("193i at 45 nm");
            }
            if o.reach(30.0) != Reach::Beyond {
                wrong.push("193i at 30 nm");
            }
        }
        None => wrong.push("193i's optics"),
    }
    // a full silicon etch in chlorine ends on the oxide by its selectivity, 200:1
    let strip = soi_strip(0.5, Etch::FULL);
    match generate(&strip, &prefer(&["icp-si-cl2"]))
        .ok()
        .as_ref()
        .and_then(first_etch)
    {
        Some(Depth::ToStop) => {}
        _ => wrong.push("Cl₂ etch to the oxide"),
    }
    // thermal: medium-temperature LPCVD (to 900 °C) refused over aluminium on silicon, PECVD
    // nitride (to 350 °C) not; the conformal LPCVD oxide fills an SOI strip's openings; over
    // lithium niobate (shown to 450 °C) PECVD
    let films = |f: &[&str]| f.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    match (process("lpcvd"), process("pecvd-nitride")) {
        (Some(lp), Some(pe)) => {
            if thermal(lp, &films(&["Si", "Al"])).violations.is_empty() {
                wrong.push("LPCVD over aluminium");
            }
            if !thermal(pe, &films(&["Si", "Al"])).violations.is_empty() {
                wrong.push("PECVD nitride over aluminium");
            }
        }
        _ => wrong.push("the nitride depositions"),
    }
    let cladding = |d: &Physical| {
        generate(d, &Choices::default()).ok().and_then(|f| {
            f.steps
                .iter()
                .rev()
                .find(|s| s.step == super::StepType::Deposition)
                .map(|s| s.process.clone())
        })
    };
    if cladding(&strip).as_deref() != Some("lpcvd") {
        wrong.push("the SOI strip's conformal fill");
    }
    let tfln = crate::device::TflnModulator::new();
    if tfln
        .physical(&Values::defaults(&tfln))
        .ok()
        .and_then(|d| cladding(&d))
        .as_deref()
        != Some("pecvd-oxide")
    {
        wrong.push("the TFLN cladding over lithium niobate");
    }
    // RIE lag: 220 nm deep in a 100 nm gap (2.2:1) warns, in a 300 nm gap (0.73:1) doesn't
    if verdict_of(&soi_pair(0.1), &prefer(&["ebl-hsq"]), Rule::AspectRatio)
        != Some(Verdict::Warning)
    {
        wrong.push("a 2.2:1 opening");
    }
    if verdict_of(&soi_pair(0.3), &Choices::default(), Rule::AspectRatio) != Some(Verdict::Within) {
        wrong.push("a 0.73:1 opening");
    }
    // registration: a heater 7.5 µm wide over a 0.5 µm guide clears 193 nm lithography's 12 nm
    // overlay; one 5 nm wider on each side doesn't
    let heater = ThermoShifter::metal();
    let at = |hw: f64| {
        Values::defaults(&heater)
            .with(&heater, "heater_width", hw)
            .ok()
            .and_then(|v| heater.physical(&v).ok())
    };
    let overlay =
        |hw: f64| at(hw).and_then(|d| verdict_of(&d, &prefer(&["duv-193"]), Rule::Overlay));
    if overlay(7.5) != Some(Verdict::Within) {
        wrong.push("a heater 3.5 µm clear of its guide");
    }
    if overlay(0.51) != Some(Verdict::Warning) {
        wrong.push("a heater 5 nm clear of its guide");
    }
    wrong
}

fn rule_choices() -> Outcome {
    exact(rule_choices_wrong().len())
}

/// The heater's oxide under the TiN, polished: our combined bound, and the root of the sum of
/// the squares of its stated contributions computed here from the cited numbers: Krishnan et al.'s
/// 30 nm nonplanarity and Franssila's 10 % of the step polished off.
pub(crate) fn combined_gap() -> Option<(f64, f64)> {
    let heater = ThermoShifter::metal();
    let design = heater.physical(&Values::defaults(&heater)).ok()?;
    let flow = generate(&design, &Choices::default()).ok()?;
    let a = assess(&design, &flow).ok()?;
    let d = a
        .accuracy
        .iter()
        .find(|d| d.layer == "gap" && d.kind == Kind::Thickness)?;
    let step_nm = design.structure().stack().layer("Si")?.0.thickness.to_um() * 1000.0;
    let expected = (30.0_f64.powi(2) + (0.10 * step_nm).powi(2)).sqrt();
    Some((d.combined?, expected))
}

fn accuracy_combined() -> Outcome {
    let (got, want) = combined_gap().unwrap_or((f64::NAN, 1.0));
    Outcome {
        measured: got,
        expected: want,
        tolerance: 1e-12 * want.abs(),
        error: (got - want).abs(),
    }
}

/// The library's devices with the parameters the round trip runs through: each device's own
/// parameters, every one at its range's ends in turn, the others at their defaults.
pub(crate) fn library_designs(device: &dyn Device) -> Vec<Values> {
    let defaults = Values::defaults(device);
    let mut made: Vec<Physical> = device.physical(&defaults).into_iter().collect();
    let mut out = vec![defaults.clone()];
    for p in device.parameters() {
        for v in [p.min, p.max] {
            // a parameter that leaves the design as it was (a length of loss, a wavelength)
            // makes no new design
            if let Ok(w) = defaults.with(device, &p.name, v)
                && let Ok(design) = device.physical(&w)
                && !made.contains(&design)
            {
                made.push(design);
                out.push(w);
            }
        }
    }
    out
}

/// The round trip over every library device the rules make: (designs made, designs refused, the
/// devices refused with why, differing points).
pub(crate) fn library_round_trips() -> (usize, usize, Vec<String>, usize) {
    let (mut made, mut refused, mut why, mut differing) = (0, 0, Vec::new(), 0);
    for device in crate::device::library() {
        for values in library_designs(device.as_ref()) {
            let Ok(design) = device.physical(&values) else {
                continue;
            };
            match generate(&design, &Choices::default()) {
                Err(e) => {
                    refused += 1;
                    let w = format!("{}: {e}", device.kind());
                    if !why
                        .iter()
                        .any(|x: &String| x.starts_with(&format!("{}:", device.kind())))
                    {
                        why.push(w);
                    }
                }
                Ok(flow) => {
                    made += 1;
                    match round_trip(&flow, &design) {
                        Ok(t) if t.structure_equal && t.films_equal => differing += t.differing,
                        Ok(t) => differing += t.points.max(1),
                        Err(_) => differing += 1,
                    }
                }
            }
        }
    }
    (made, refused, why, differing)
}

fn round_trip_library() -> Outcome {
    exact(library_round_trips().3)
}

pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "fabrication/rayleigh-franssila",
            title: r"The Rayleigh criterion $W = k_1\lambda/\mathrm{NA}$ gives the linewidths of Franssila's Table 10.1 at 436, 365 and 248 nm (NA 0.38, 0.48, 0.60; $k_1$ 0.8, 0.6, 0.6) to the printing's rounding (the largest misfit over half a unit of the last printed digit shown)",
            tier: Tier::Published,
            source: "S. Franssila, Introduction to Microfabrication, 2nd ed., Wiley (2010), doi:10.1002/9781119990413, Eq. 10.1 and Table 10.1 (p. 117)",
            run: rayleigh_franssila,
        },
        Case {
            id: "fabrication/deal-grove-campbell",
            title: "Deal and Grove's law with their 920 °C wet constants gives Campbell's Example 4.1: 2 h of steam oxidation over 1000 Å of oxide leaves 0.48 µm (µm)",
            tier: Tier::Published,
            source: "S. A. Campbell, Fabrication Engineering at the Micro- and Nanoscale, 3rd ed., Oxford University Press (2008), ISBN 978-0-19-532017-6, Example 4.1 (pp. 80–81), with B. E. Deal, A. S. Grove, J. Appl. Phys. 36, 3770 (1965), doi:10.1063/1.1713945, Table I",
            run: deal_grove_campbell,
        },
        Case {
            id: "fabrication/min-features-above-limit",
            title: r"Every cited minimum feature of a projection lithography asks for a $k_1$ at or above the theoretical limit of 0.25 at its class's highest cited NA (minimum features below it shown)",
            tier: Tier::Published,
            source: "S. A. Campbell, Fabrication Engineering at the Micro- and Nanoscale, 3rd ed. (2008), ISBN 978-0-19-532017-6, Eq. 7.18 (p. 183) and Table 7.4 (p. 189); C. Mack, Fundamental Principles of Optical Lithography (2007), doi:10.1002/9780470723876, Eq. 2.83 (p. 58)",
            run: min_features_above_limit,
        },
        Case {
            id: "fabrication/rule-choices",
            title: r"The rules' choices with textbook answers: 193 nm immersion prints 100 nm typically, 45 nm only with enhancement, 30 nm not at all; a chlorine silicon etch ends on the oxide by its selectivity; LPCVD is refused over aluminium and PECVD nitride isn't; a conformal LPCVD fills an SOI strip, PECVD covers lithium niobate; a 2.2:1 opening warns of RIE lag and a 0.73:1 one doesn't; a heater 3.5 µm clear of its guide registers within 193 nm lithography's overlay, one 5 nm clear doesn't (wrong answers shown)",
            tier: Tier::Published,
            source: "Campbell (2008), Eq. 7.18 (p. 183), Table 7.4 (p. 189), Section 7.9 (p. 194), Section 11.8 (p. 310); Franssila (2010), doi:10.1002/9781119990413, Sections 5.8 (p. 57), 11.12.3 (pp. 138–139) and 21.7.1 (p. 266); M. J. Madou, Fundamentals of Microfabrication, 3rd ed., Vol. II, doi:10.1201/9781315274164, Table 7.19 (p. 452); Selvaraja et al. 2009, doi:10.1109/JLT.2009.2022282, p. 4076",
            run: rule_choices,
        },
        Case {
            id: "fabrication/accuracy-combined",
            title: "The accuracy read-out combines a dimension's contributions as the root of the sum of their squares: the TiN heater's polished oxide under the heater, from Krishnan et al.'s 30 nm nonplanarity and Franssila's 10 % of the 220 nm polished off (relative error shown)",
            tier: Tier::Analytic,
            source: "Campbell (2008), Eq. 7.27 (p. 194); Krishnan et al. 2009, doi:10.1021/cr900170z, p. 183; Franssila (2010), Table 16.1 (p. 182): an exact identity",
            run: accuracy_combined,
        },
        Case {
            id: "fabrication/round-trip-library",
            title: "Every library device whose design the rules make gives back its structure and films to the bit through its generated flow: at its defaults and each parameter's range ends in turn (differing points shown)",
            tier: Tier::Analytic,
            source: "an exact identity: the flow's operations draw the design's own shapes, etches and films",
            run: round_trip_library,
        },
    ]
}
