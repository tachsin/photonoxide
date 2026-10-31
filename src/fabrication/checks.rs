//! The fabrication engine's validation cases, shared by the tests and the validation report:
//! the round trip for the three first designs at their defaults and corners, every capability's
//! source, Deal and Grove's tables, and the rules on cases with textbook answers and on the flows
//! papers report.

use crate::device::{Device, StripWaveguide, TflnModulator, ThermoShifter, Values};
use crate::geometry::{Point, Shape};
use crate::stack::{LayerStack, Structure};
use crate::units::Length;
use crate::validation::{Case, Outcome, Tier};

use super::{
    Choices, Depth, Flow, Operation, Physical, StepType, budget, generate, library, oxidation,
    process, reference, round_trip, thermal,
};

/// The parameters whose corners the round trip runs through, for each of the three designs:
/// those that shape the layers and the shapes.
pub(crate) fn designs() -> Vec<(Box<dyn Device>, &'static [&'static str])> {
    vec![
        (
            Box::new(StripWaveguide::new()),
            &["width", "thickness", "slab", "sidewall"],
        ),
        (
            Box::new(TflnModulator::new()),
            &["film", "slab", "width", "sidewall", "gap", "metal"],
        ),
        (
            Box::new(ThermoShifter::metal()),
            &[
                "width",
                "thickness",
                "heater_width",
                "heater_thickness",
                "height",
                "cover",
            ],
        ),
    ]
}

/// The design's values at its defaults, then at every corner of `corners`' ranges (the others
/// at their defaults); those that don't draw are left out.
pub(crate) fn designs_of(device: &dyn Device, corners: &[&str]) -> Vec<Values> {
    let defaults = Values::defaults(device);
    let mut out = vec![defaults.clone()];
    let n = corners.len();
    for mask in 0..(1usize << n) {
        let mut v = defaults.clone();
        let mut ok = true;
        for (k, name) in corners.iter().enumerate() {
            let Some(p) = device.parameters().iter().find(|p| p.name == *name) else {
                ok = false;
                break;
            };
            let value = if mask & (1 << k) == 0 { p.min } else { p.max };
            match v.with(device, name, value) {
                Ok(w) => v = w,
                Err(_) => {
                    ok = false;
                    break;
                }
            }
        }
        if ok && device.geometry(&v).is_ok() {
            out.push(v);
        }
    }
    out
}

/// The round trip of a device over its defaults and corners: the designs run, and the points
/// that differ in all, a structure or a film that isn't the design's counting as every point of
/// its design; a design whose flow can't be made counts as one.
pub(crate) fn round_trips(device: &dyn Device, corners: &[&str]) -> (usize, usize) {
    let mut differing = 0;
    let all = designs_of(device, corners);
    for values in &all {
        let Ok(design) = device.physical(values) else {
            differing += 1;
            continue;
        };
        let trip = generate(&design, &Choices::default()).and_then(|f| round_trip(&f, &design));
        match trip {
            Ok(t) if t.structure_equal && t.films_equal => differing += t.differing,
            Ok(t) => differing += t.points.max(1),
            Err(_) => differing += 1,
        }
    }
    (all.len(), differing)
}

fn exact(count: usize) -> Outcome {
    let e = count as f64;
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: 0.0,
        error: e,
    }
}

fn round_trip_case(k: usize) -> Outcome {
    let all = designs();
    let (device, corners) = &all[k];
    exact(round_trips(device.as_ref(), corners).1)
}

fn round_trip_waveguide() -> Outcome {
    round_trip_case(0)
}

fn round_trip_tfln() -> Outcome {
    round_trip_case(1)
}

fn round_trip_heater() -> Outcome {
    round_trip_case(2)
}

/// Every stated value's, fact's, stop's, mask's and use's source: what is wrong with each (a
/// reference the list hasn't, one without a DOI or an ISBN, an empty location), over the library
/// and the budgets of the films the flows meet.
pub(crate) fn source_faults() -> Vec<String> {
    let mut faults = Vec::new();
    let mut check = |what: &str, s: &super::Source| match reference(&s.reference) {
        None => faults.push(format!("{what}: no reference {}", s.reference)),
        Some(r) if r.doi.is_empty() && r.isbn.is_empty() => {
            faults.push(format!("{what}: {} has no DOI or ISBN", r.key));
        }
        Some(_) if s.location.trim().is_empty() => {
            faults.push(format!("{what}: no place in {}", s.reference));
        }
        Some(_) => {}
    };
    for p in library() {
        for c in &p.capabilities {
            if let Some(s) = c.value.source() {
                check(&format!("{} {}", p.id, c.quantity.name()), s);
            }
        }
        for f in &p.facts {
            check(&format!("{} fact", p.id), &f.source);
        }
        for st in &p.stops_on {
            check(&format!("{} stop", p.id), &st.source);
        }
        for m in &p.masks {
            for v in [&m.thickness, &m.selectivity, &m.depth] {
                if let Some(s) = v.source() {
                    check(&format!("{} mask", p.id), s);
                }
            }
        }
        for u in &p.used_for {
            check(&format!("{} use", p.id), &u.source);
        }
    }
    for t in super::glossary() {
        check(&format!("the glossary's {}", t.term), &t.source);
    }
    for film in ["Si", "SiO2", "LiNbO3", "TiN", "Al", "Sb₂Se₃"] {
        let b = budget(film);
        for v in [&b.survives, &b.limit] {
            if let Some(s) = v.source() {
                check(&format!("{film}'s budget"), s);
            }
        }
    }
    faults
}

fn capabilities_cited() -> Outcome {
    exact(source_faults().len())
}

fn deal_grove_tables() -> Outcome {
    let worst = oxidation::ratio_consistency()
        .iter()
        .map(|&(_, _, r)| r)
        .fold(0.0, f64::max);
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1.0,
        error: worst,
    }
}

/// A design on 220 nm SOI: one strip `width` µm wide and 10 µm long, etched by `etch`.
pub(crate) fn soi_strip(width: f64, etch: crate::stack::Etch) -> Physical {
    let mut s = Structure::new(LayerStack::soi_220());
    let strip = Shape::rect(Point::um(5.0, 0.0), Length::um(10.0), Length::um(width))
        .expect("a positive width");
    s.draw_etched("Si", strip, etch).expect("on the Si layer");
    Physical::of(s)
}

/// The first etch's depth in a flow.
pub(crate) fn first_etch(flow: &Flow) -> Option<Depth> {
    flow.steps.iter().find_map(|s| match s.operation {
        Operation::Etch { depth, .. } => Some(depth),
        _ => None,
    })
}

/// The first lithography's process in a flow.
pub(crate) fn first_lithography(flow: &Flow) -> Option<String> {
    flow.steps
        .iter()
        .find(|s| s.step == StepType::Lithography)
        .map(|s| s.process.clone())
}

/// Etch stop or timed: a strip through 220 nm of silicon on its buried oxide stops on it, a
/// rib's partial etch is timed, and so is the thin-film lithium niobate rib. The wrong answers.
pub(crate) fn etch_stop_wrong() -> usize {
    let mut wrong = 0;
    let strip = soi_strip(0.5, crate::stack::Etch::FULL);
    match generate(&strip, &Choices::default())
        .ok()
        .as_ref()
        .and_then(first_etch)
    {
        Some(Depth::ToStop) => {}
        _ => wrong += 1,
    }
    let rib = soi_strip(0.5, crate::stack::Etch::partial(Length::nm(130.0)));
    match generate(&rib, &Choices::default())
        .ok()
        .as_ref()
        .and_then(first_etch)
    {
        Some(Depth::Timed(d)) if d == Length::nm(130.0) => {}
        _ => wrong += 1,
    }
    let tfln = TflnModulator::new();
    let ln = tfln
        .physical(&Values::defaults(&tfln))
        .ok()
        .and_then(|d| generate(&d, &Choices::default()).ok());
    match ln.as_ref().and_then(first_etch) {
        Some(Depth::Timed(_)) => {}
        _ => wrong += 1,
    }
    wrong
}

fn etch_stop_or_timed() -> Outcome {
    exact(etch_stop_wrong())
}

/// The lithography's choice just inside and just outside the cited minimum features: an SOI
/// strip 150 nm wide takes 193 nm lithography (Selvaraja et al.'s 150 nm), 149 nm the electron
/// beam (HSQ, at most 5 nm); 5 nm still the electron beam, and 4.9 nm none, refused. The wrong
/// answers.
pub(crate) fn lithography_wrong() -> usize {
    let pick = |w: f64| {
        generate(&soi_strip(w, crate::stack::Etch::FULL), &Choices::default())
            .ok()
            .as_ref()
            .and_then(first_lithography)
    };
    let mut wrong = 0;
    for (w, want) in [
        (0.150, Some("duv-193")),
        (0.149, Some("ebl-hsq")),
        (0.005, Some("ebl-hsq")),
        (0.0049, None),
    ] {
        if pick(w).as_deref() != want {
            wrong += 1;
        }
    }
    wrong
}

fn lithography_choice() -> Outcome {
    exact(lithography_wrong())
}

/// Thermal ordering: thermal oxidation (to 1300 °C) refused over aluminium on silicon (its
/// eutectic, 577 °C), PECVD oxide (at most 350 °C) not; PECVD refused over amorphous Sb₂Se₃
/// (crystallizing at 200 °C); titanium nitride shown to survive PECVD (600 °C). The wrong
/// answers.
pub(crate) fn thermal_wrong() -> usize {
    let films = |f: &[&str]| f.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    let (Some(oxidation), Some(pecvd)) = (process("thermal-oxidation"), process("pecvd-oxide"))
    else {
        return 4;
    };
    let mut wrong = 0;
    if thermal(oxidation, &films(&["Si", "Al"]))
        .violations
        .is_empty()
    {
        wrong += 1;
    }
    if !thermal(pecvd, &films(&["Si", "Al"])).violations.is_empty() {
        wrong += 1;
    }
    if thermal(pecvd, &films(&["Si", "Sb₂Se₃"]))
        .violations
        .is_empty()
    {
        wrong += 1;
    }
    let tin = thermal(pecvd, &films(&["TiN"]));
    if !tin.violations.is_empty() || tin.shown.is_empty() {
        wrong += 1;
    }
    wrong
}

fn thermal_order() -> Outcome {
    exact(thermal_wrong())
}

/// The sequences papers report for their own devices, as (paper, the device it is checked on,
/// the processes in order).
pub(crate) const REPORTED: [(&str, usize, &[&str]); 3] = [
    (
        "selvaraja-2009",
        0,
        &["soi-smart-cut", "duv-193", "icp-si-hbr", "resist-strip"],
    ),
    (
        "zhang-2017-optica",
        1,
        &[
            "lnoi-ion-slicing",
            "ebl-hsq",
            "ar-milling-ln",
            "resist-strip",
            "pecvd-oxide",
        ],
    ),
    (
        "zhang-2022",
        1,
        &[
            "lnoi-ion-slicing",
            "ebl-hsq",
            "ar-milling-ln",
            "uv-photolithography",
            "evaporation",
            "lift-off",
            "pecvd-oxide",
        ],
    ),
];

/// The reported sequences the generated flows don't contain, in order.
pub(crate) fn reported_missing() -> Vec<&'static str> {
    let all = designs();
    let mut missing = Vec::new();
    for (paper, k, sequence) in REPORTED {
        let device = all[k].0.as_ref();
        let flow = device
            .physical(&Values::defaults(device))
            .ok()
            .and_then(|d| generate(&d, &Choices::default()).ok());
        let Some(flow) = flow else {
            missing.push(paper);
            continue;
        };
        let mut next = 0;
        for s in &flow.steps {
            if next < sequence.len() && s.process == sequence[next] {
                next += 1;
            }
        }
        if next < sequence.len() {
            missing.push(paper);
        }
    }
    missing
}

fn reported_flows() -> Outcome {
    exact(reported_missing().len())
}

pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "fabrication/round-trip-waveguide",
            title: "The SOI strip waveguide's generated flow, executed on the library's own geometry, gives back its structure and films to the bit: at its defaults and the corners of its width, thickness, slab and sidewall that draw (differing points shown)",
            tier: Tier::Analytic,
            source: "an exact identity: the flow's operations draw the design's own shapes, etches and films",
            run: round_trip_waveguide,
        },
        Case {
            id: "fabrication/round-trip-tfln",
            title: "The TFLN modulator's generated flow gives back its structure and films (the electrodes copper and gold) to the bit: at its defaults and the corners of its film, slab, width, sidewall, gap and metal that draw (differing points shown)",
            tier: Tier::Analytic,
            source: "an exact identity: the flow's operations draw the design's own shapes, etches and films",
            run: round_trip_tfln,
        },
        Case {
            id: "fabrication/round-trip-heater",
            title: "The TiN thermo-optic phase shifter's generated flow gives back its structure and films (the heater titanium nitride) to the bit: at its defaults and the corners of its width, thickness, heater width and thickness, height and cover that draw (differing points shown)",
            tier: Tier::Analytic,
            source: "an exact identity: the flow's operations draw the design's own shapes, etches and films",
            run: round_trip_heater,
        },
        Case {
            id: "fabrication/capabilities-cited",
            title: "Every number and statement in the process library and its glossary names a reference in its list, with a DOI or an ISBN, and a place in it (faults shown)",
            tier: Tier::Analytic,
            source: "the library's own references, each DOI checked on Crossref and each ISBN read in the copy in hand",
            run: capabilities_cited,
        },
        Case {
            id: "fabrication/deal-grove-tables",
            title: r"Deal and Grove's printed rate constants hold together: in every row of Tables I and II, the printed $B/A$ is $B$ over $A$ to the rounding of the three printed numbers (the largest misfit over what the rounding allows shown)",
            tier: Tier::Published,
            source: "B. E. Deal, A. S. Grove, J. Appl. Phys. 36, 3770 (1965), doi:10.1063/1.1713945, Tables I (wet O₂) and II (dry O₂)",
            run: deal_grove_tables,
        },
        Case {
            id: "fabrication/etch-stop-or-timed",
            title: "A strip etched through 220 nm of silicon stops on its buried oxide; a rib's partial etch and the lithium niobate rib's are timed (wrong answers shown)",
            tier: Tier::Analytic,
            source: "S. K. Selvaraja et al., J. Lightwave Technol. 27, 4076 (2009), doi:10.1109/JLT.2009.2022282, Section IV: the HBr/O₂ etch stops on the buried oxide; a partial etch has no film below its floor to stop it",
            run: etch_stop_or_timed,
        },
        Case {
            id: "fabrication/lithography-choice",
            title: "A strip 150 nm wide takes 193 nm lithography and 149 nm the electron beam; 5 nm still the electron beam, 4.9 nm is refused (wrong answers shown)",
            tier: Tier::Published,
            source: "the cited minimum features: 150 nm printed linearly at 193 nm (Selvaraja et al. 2009, doi:10.1109/JLT.2009.2022282, Section III), at most 5 nm in HSQ (Y. Chen, Microelectron. Eng. 135, 57 (2015), doi:10.1016/j.mee.2015.02.042, Table 1)",
            run: lithography_choice,
        },
        Case {
            id: "fabrication/thermal-order",
            title: "Thermal oxidation is refused over aluminium on silicon and PECVD isn't; PECVD is refused over amorphous Sb₂Se₃; titanium nitride is shown to survive PECVD (wrong answers shown)",
            tier: Tier::Published,
            source: "the Al/Si eutectic at 577 °C (S. A. Campbell, Fabrication Engineering at the Micro- and Nanoscale, 3rd ed., Fig. 15.24), Sb₂Se₃ crystallizing at 200 °C (Delaney et al. 2020, doi:10.1002/adfm.202002447), TiN annealed at 600 °C (Kearney et al. 2018, doi:10.1016/j.tsf.2018.07.001), PECVD at 100–350 °C (Madou, Vol. II, Table 7.19)",
            run: thermal_order,
        },
        Case {
            id: "fabrication/reported-flows",
            title: "The generated flows contain, in order, the steps papers report for their own devices: Selvaraja et al.'s SOI wires, Zhang et al.'s lithium niobate rings (2017) and modulator (2022) (sequences missing shown)",
            tier: Tier::Published,
            source: "Selvaraja et al. 2009, doi:10.1109/JLT.2009.2022282, Sections II–IV; M. Zhang et al., Optica 4, 1536 (2017), doi:10.1364/OPTICA.4.001536; Y. Zhang et al., Photon. Res. 10, 2380 (2022), doi:10.1364/PRJ.468518, Section 3.A",
            run: reported_flows,
        },
    ]
}
