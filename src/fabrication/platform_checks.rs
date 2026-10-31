//! The validation cases of the fabrication engine's further platforms (F6 of #622): silicon
//! nitride, by deposition and etching or by the photonic Damascene process. Each round trip is
//! exact by construction; each rule's choice is categorical; each reported flow is the sequence a
//! paper prints for its own device, contained in order.

use crate::device::{Device, Spec, StackSpec};
use crate::validation::{Case, Outcome, Tier};

use super::rule_checks::library_designs;
use super::{Choices, Flow, Operation, Physical, generate, round_trip};

fn exact(count: usize) -> Outcome {
    let e = count as f64;
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: 0.0,
        error: e,
    }
}

/// The library device `kind` on `stack`, its parameters `values` (name, expression) and the
/// others at their defaults: the device and its physical design.
pub(crate) fn built(
    kind: &str,
    stack: StackSpec,
    values: &[(&str, &str)],
) -> Option<(Box<dyn Device>, Physical)> {
    let spec = Spec {
        stack,
        parameters: values
            .iter()
            .map(|(n, v)| ((*n).to_string(), (*v).to_string()))
            .collect(),
        ..Spec::new(kind)
    };
    let (device, design) = spec.build().ok()?;
    let physical = device.physical(design.values()).ok()?;
    Some((device, physical))
}

/// The round trip over every library device built on `stack` (but those in `skip`), at its
/// defaults and each parameter's range ends in turn: (designs made, designs refused, the devices
/// refused with why, differing points). A flow that doesn't give its design back counts every
/// point of it.
pub(crate) fn platform_round_trips(
    stack: &StackSpec,
    skip: &[&str],
) -> (usize, usize, Vec<String>, usize) {
    let (mut made, mut refused, mut why, mut differing) = (0, 0, Vec::new(), 0);
    for kind in crate::device::library()
        .iter()
        .map(|d| d.kind().to_string())
    {
        if skip.contains(&kind.as_str()) {
            continue;
        }
        let Some((device, _)) = built(&kind, stack.clone(), &[]) else {
            continue;
        };
        for values in library_designs(device.as_ref()) {
            let Ok(design) = device.physical(&values) else {
                continue;
            };
            match generate(&design, &Choices::default()) {
                Err(e) => {
                    refused += 1;
                    if !why
                        .iter()
                        .any(|w: &String| w.starts_with(&format!("{kind}:")))
                    {
                        why.push(format!("{kind}: {e}"));
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

/// The nitride stack, with `oxide` µm of oxide under the core.
pub(crate) fn nitride(oxide: f64) -> StackSpec {
    StackSpec::Preset {
        name: "nitride".into(),
        bottom_oxide_um: oxide,
    }
}

/// The library's devices on the nitride stack: all but the lithium niobate modulator, which has
/// its own stack.
pub(crate) const NITRIDE_SKIP: [&str; 1] = ["TFLN Mach-Zehnder modulator"];

fn nitride_round_trip() -> Outcome {
    exact(platform_round_trips(&nitride(2.0), &NITRIDE_SKIP).3)
}

/// The process ids of a flow, in order.
pub(crate) fn ids(flow: &Flow) -> Vec<String> {
    flow.steps.iter().map(|s| s.process.clone()).collect()
}

/// Whether `flow` contains `sequence` in order.
pub(crate) fn contains(flow: &Flow, sequence: &[&str]) -> bool {
    let mut next = 0;
    for s in &flow.steps {
        if next < sequence.len() && s.process == sequence[next] {
            next += 1;
        }
    }
    next == sequence.len()
}

/// A straight nitride guide `width` µm wide and `thickness` µm thick on 2 µm of oxide, its walls
/// at `sidewall`°.
pub(crate) fn nitride_guide(width: f64, thickness: f64, sidewall: f64) -> Option<Physical> {
    let (w, t, s) = (
        width.to_string(),
        thickness.to_string(),
        sidewall.to_string(),
    );
    built(
        "waveguide",
        nitride(2.0),
        &[("width", &w), ("thickness", &t), ("sidewall", &s)],
    )
    .map(|(_, p)| p)
}

/// The nitride rules' choices with answers the sources give; each wrong answer counted, with
/// what. A 220 nm film is deposited and etched with no crack stops (Luke et al.: films crack above
/// 400 nm); a 910 nm one is etched after crack stops (Luke et al.'s own); a 1.2 µm one, thicker
/// than any film a source etched, is made by the photonic Damascene process (Pfeiffer et al.'s
/// 1.2 µm films); with sloped walls it stays etched, with a warning; and every patterned nitride
/// is annealed after it is patterned, before the cladding (Luke et al.; Pfeiffer et al.).
pub(crate) fn nitride_rules_wrong() -> Vec<&'static str> {
    let mut wrong = Vec::new();
    let flow = |w: f64, t: f64, s: f64| {
        nitride_guide(w, t, s).and_then(|d| generate(&d, &Choices::default()).ok())
    };
    let has =
        |f: &Option<Flow>, id: &str| f.as_ref().is_some_and(|f| ids(f).iter().any(|p| p == id));
    let thin = flow(0.5, 0.22, 90.0);
    if thin.is_none() || has(&thin, "crack-stop-trenches") || has(&thin, "damascene-preform") {
        wrong.push("a 220 nm film, etched without crack stops");
    }
    let luke = flow(1.8, 0.91, 90.0);
    if !has(&luke, "crack-stop-trenches")
        || !has(&luke, "icp-si3n4-chf3")
        || has(&luke, "damascene-preform")
    {
        wrong.push("a 910 nm film, etched after crack stops");
    }
    let thick = flow(1.5, 1.2, 90.0);
    if !has(&thick, "damascene-preform")
        || !has(&thick, "cmp-nitride")
        || has(&thick, "icp-si3n4-chf3")
    {
        wrong.push("a 1.2 µm film, by the Damascene process");
    }
    let sloped = flow(1.5, 1.2, 80.0);
    if !has(&sloped, "icp-si3n4-chf3")
        || has(&sloped, "damascene-preform")
        || sloped
            .as_ref()
            .is_none_or(|f| !f.warnings.iter().any(|w| w.contains("Damascene")))
    {
        wrong.push("a 1.2 µm film with sloped walls, etched with a warning");
    }
    for (f, what) in [
        (&thin, "the 220 nm film's anneal"),
        (&luke, "the 910 nm film's anneal"),
        (&thick, "the Damascene film's anneal"),
    ] {
        let Some(f) = f else {
            wrong.push(what);
            continue;
        };
        let at = |id: &str| f.steps.iter().position(|s| s.process == id);
        let anneal = at("anneal-nitride");
        let last_pattern = f.steps.iter().rposition(|s| {
            matches!(
                s.operation,
                Operation::Etch { .. } | Operation::Planarize { .. }
            )
        });
        let cladding = f
            .steps
            .iter()
            .rposition(|s| matches!(s.operation, Operation::Deposit { cladding: true, .. }));
        if !matches!((last_pattern, anneal, cladding), (Some(p), Some(a), Some(c)) if p < a && a < c)
        {
            wrong.push(what);
        }
    }
    wrong
}

fn nitride_rules() -> Outcome {
    exact(nitride_rules_wrong().len())
}

/// The flows papers report for their own nitride devices, as (paper, the design, the processes
/// in order): Luke et al.'s 910 nm by 1.8 µm ring, 680 nm from its guide; Pfeiffer et al.'s
/// 1.35 µm thick guides, 1.5 µm wide, by the photonic Damascene process; Ríos et al.'s guides
/// etched 165 nm into 335 nm of nitride.
pub(crate) fn nitride_reported() -> Vec<(&'static str, Option<Physical>, &'static [&'static str])> {
    vec![
        (
            "luke-2013",
            built(
                "all-pass ring",
                nitride(2.0),
                &[
                    ("thickness", "0.91"),
                    ("width", "1.8"),
                    ("gap", "0.68"),
                    ("radius", "115"),
                ],
            )
            .map(|(_, p)| p),
            &[
                "si-wafer",
                "thermal-oxidation",
                "crack-stop-trenches",
                "lpcvd",
                "ebl-man",
                "icp-si3n4-chf3",
                "resist-strip",
                "anneal-nitride",
                "lpcvd",
            ],
        ),
        (
            "pfeiffer-2016",
            nitride_guide(1.5, 1.35, 90.0),
            &[
                "si-wafer",
                "thermal-oxidation",
                "damascene-preform",
                "resist-strip",
                "lpcvd",
                "cmp-nitride",
                "anneal-nitride",
            ],
        ),
        (
            "rios-2015",
            built(
                "rib waveguide",
                nitride(3.35),
                &[("width", "1.3"), ("thickness", "0.335"), ("slab", "0.17")],
            )
            .map(|(_, p)| p),
            &["ebl-man", "icp-si3n4-chf3", "resist-strip"],
        ),
    ]
}

/// The reported sequences the generated flows don't contain, in order; and those whose design
/// the rules refuse or don't give back exactly.
pub(crate) fn nitride_reported_missing() -> Vec<&'static str> {
    let mut missing = Vec::new();
    for (paper, design, sequence) in nitride_reported() {
        let Some(design) = design else {
            missing.push(paper);
            continue;
        };
        let Ok(flow) = generate(&design, &Choices::default()) else {
            missing.push(paper);
            continue;
        };
        let exact = round_trip(&flow, &design).is_ok_and(|t| t.exact());
        if !exact || !contains(&flow, sequence) {
            missing.push(paper);
        }
    }
    missing
}

fn nitride_reported_flows() -> Outcome {
    exact(nitride_reported_missing().len())
}

pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "fabrication/nitride-round-trip",
            title: "Every library device built on the silicon nitride stack (all but the lithium niobate modulator) whose design the rules make gives back its structure and films to the bit through its generated flow, the nitride deposited on grown oxide: at its defaults and each parameter's range ends in turn (differing points shown)",
            tier: Tier::Analytic,
            source: "an exact identity: the flow's operations draw the design's own shapes, etches and films",
            run: nitride_round_trip,
        },
        Case {
            id: "fabrication/nitride-rules",
            title: "The nitride rules' choices with the sources' answers: 220 nm etched without crack stops, 910 nm etched after crack stops, 1.2 µm by the photonic Damascene process, 1.2 µm with sloped walls etched with a warning, and every patterned film annealed before its cladding (wrong answers shown)",
            tier: Tier::Published,
            source: "K. Luke et al., Opt. Express 21, 22829 (2013), doi:10.1364/OE.21.022829, PDF pp. 2–4; M. H. P. Pfeiffer et al., Optica 3, 20 (2016), doi:10.1364/OPTICA.3.000020, PDF pp. 2–3",
            run: nitride_rules,
        },
        Case {
            id: "fabrication/nitride-reported-flows",
            title: "The generated nitride flows contain, in order, the steps papers report for their own devices, and give the designs back to the bit: Luke et al.'s 910 nm ring, Pfeiffer et al.'s 1.35 µm Damascene guides and Ríos et al.'s partly etched guides (sequences missing shown)",
            tier: Tier::Published,
            source: "Luke et al. 2013, doi:10.1364/OE.21.022829, PDF pp. 3–4; Pfeiffer et al. 2016, doi:10.1364/OPTICA.3.000020, PDF pp. 2–3; C. Ríos et al., Nat. Photonics 9, 725 (2015), doi:10.1038/nphoton.2015.182, Methods (PDF p. 9)",
            run: nitride_reported_flows,
        },
    ]
}
