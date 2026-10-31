//! The validation cases of the fabrication engine's further platforms (F6 of #622): silicon
//! nitride, by deposition and etching or by the photonic Damascene process. Each round trip is
//! exact by construction; each rule's choice is categorical; each reported flow is the sequence a
//! paper prints for its own device, contained in order.

use crate::device::{Device, Spec, StackSpec, Values};
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

/// A device's designs: at its defaults, then at each range end of the parameters `ends` names
/// in turn (every parameter's with `None`, as the library's round trip), the others at their
/// defaults.
pub(crate) fn designs_with(device: &dyn Device, ends: Option<&[&str]>) -> Vec<Values> {
    let Some(ends) = ends else {
        return library_designs(device);
    };
    let defaults = Values::defaults(device);
    let mut out = vec![defaults.clone()];
    for p in device
        .parameters()
        .iter()
        .filter(|p| ends.contains(&p.name.as_str()))
    {
        for v in [p.min, p.max] {
            if let Ok(w) = defaults.with(device, &p.name, v) {
                out.push(w);
            }
        }
    }
    out
}

/// The round trip over every library device built on `stack` (but those in `skip`), at its
/// defaults and the range ends of the parameters `ends` names (every parameter's with `None`):
/// (designs made, designs refused, the devices refused with why, differing points). A flow that
/// doesn't give its design back counts every point of it.
pub(crate) fn platform_round_trips(
    stack: &StackSpec,
    skip: &[&str],
    ends: Option<&[&str]>,
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
        for values in designs_with(device.as_ref(), ends) {
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
    exact(platform_round_trips(&nitride(2.0), &NITRIDE_SKIP, Some(&["thickness"])).3)
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

/// The library's three phase-change shifters.
pub(crate) const PCM: [&str; 3] = [
    "GST phase-change phase shifter",
    "Sb2S3 phase-change phase shifter",
    "Sb2Se3 phase-change phase shifter",
];

/// The round trip over the three phase-change shifters on their own stack (220 nm SOI), at their
/// defaults and each parameter's range ends: (made, refused, why, differing).
pub(crate) fn pcm_round_trips() -> (usize, usize, Vec<String>, usize) {
    let others: Vec<String> = crate::device::library()
        .iter()
        .map(|d| d.kind().to_string())
        .filter(|k| !PCM.contains(&k.as_str()))
        .collect();
    let skip: Vec<&str> = others.iter().map(String::as_str).collect();
    platform_round_trips(&StackSpec::default(), &skip, None)
}

fn pcm_round_trip() -> Outcome {
    exact(pcm_round_trips().3)
}

/// The phase-change rules' choices with the sources' answers, at each shifter's defaults; each
/// wrong answer counted. The film is sputtered and lifted off (Delaney et al. 2020; Ríos et al.
/// 2015), never etched; the cap is sputtered, as Delaney et al.'s; PECVD and LPCVD oxide, which
/// pass the film's crystallization temperature in the material catalogue (Sb₂Se₃ 200 °C and
/// Sb₂S₃ 270 °C, Delaney et al. 2020; GST 150 °C, Friedrich et al. 2000), are refused; and the
/// flow says the cap's temperature isn't stated.
pub(crate) fn pcm_rules_wrong() -> Vec<String> {
    let mut wrong = Vec::new();
    for kind in PCM {
        let Some((_, design)) = built(kind, StackSpec::default(), &[]) else {
            wrong.push(format!("{kind}: doesn't draw"));
            continue;
        };
        let Ok(flow) = generate(&design, &Choices::default()) else {
            wrong.push(format!("{kind}: refused"));
            continue;
        };
        let ids = ids(&flow);
        if !contains(&flow, &["sputtering-pcm", "lift-off", "sputtering-oxide"]) {
            wrong.push(format!("{kind}: {ids:?}"));
        }
        if !flow.steps.iter().any(super::Step::sputtered) {
            wrong.push(format!("{kind}: the film isn't sputtered onto the resist"));
        }
        let cap = flow.steps.last();
        for refused in ["pecvd-oxide", "lpcvd"] {
            let shown = cap.is_some_and(|s| {
                s.alternatives
                    .iter()
                    .any(|a| a.process == refused && a.why_not.contains("limit"))
            });
            if !shown {
                wrong.push(format!(
                    "{kind}: {refused} not refused for the film's limit"
                ));
            }
        }
        if !flow
            .not_specified
            .iter()
            .any(|n| n.contains("Oxide by RF sputtering") && n.contains("limit"))
        {
            wrong.push(format!("{kind}: the cap's temperature not flagged"));
        }
        if !round_trip(&flow, &design).is_ok_and(|t| t.exact()) {
            wrong.push(format!("{kind}: not given back"));
        }
    }
    wrong
}

fn pcm_rules() -> Outcome {
    exact(pcm_rules_wrong().len())
}

/// Delaney et al. 2021's Sb₂Se₃ cells, 23 nm thick on a 220 nm SOI rib etched 120 nm: the
/// sequence they report (the rib etched, windows opened in resist, the film sputtered, the resist
/// removed, the chip capped by sputtering) contained in order, and the design given back.
pub(crate) fn pcm_reported_missing() -> Vec<&'static str> {
    let mut missing = Vec::new();
    let design = built(
        "Sb2Se3 phase-change phase shifter",
        StackSpec::default(),
        &[
            ("thickness", "0.22"),
            ("slab", "0.1"),
            ("film_thickness", "0.023"),
        ],
    )
    .map(|(_, p)| p);
    let sequence = [
        "soi-smart-cut",
        "resist-strip",
        "sputtering-pcm",
        "lift-off",
        "sputtering-oxide",
    ];
    match design.and_then(|d| generate(&d, &Choices::default()).ok().map(|f| (f, d))) {
        Some((flow, d)) => {
            let timed = flow.steps.iter().any(|s| {
                matches!(s.operation, Operation::Etch { depth: super::Depth::Timed(t), .. } if (t.to_um() - 0.12).abs() < 1e-12)
            });
            if !timed
                || !contains(&flow, &sequence)
                || !round_trip(&flow, &d).is_ok_and(|t| t.exact())
            {
                missing.push("delaney-2021");
            }
        }
        None => missing.push("delaney-2021"),
    }
    missing
}

fn pcm_reported_flows() -> Outcome {
    exact(pcm_reported_missing().len())
}

/// Aluminium nitride, as a design's isotropic film: the catalogue's ordinary index (the guides'
/// in-plane field), under the film's name. The catalogue's AlN is uniaxial, which a device's layer
/// can't take; the fabrication engine reads only the film.
pub(crate) fn aln_material() -> Option<crate::material::Material> {
    let entry = crate::material::catalogue::entry("aln")?;
    let [o, _, _] = entry.principal(None, Default::default()).ok()?;
    let (shortest, longest) = o.range();
    crate::material::Material::new(
        "AlN",
        o.model().clone(),
        shortest,
        longest,
        o.provenance().clone(),
    )
    .ok()
}

/// A design on the nitride stack with its nitride layer made aluminium nitride, every shape and
/// etch as drawn.
pub(crate) fn on_aln(design: &Physical) -> Option<Physical> {
    use crate::stack::{Layer, LayerStack, Structure};
    let s = design.structure();
    let aln = aln_material()?;
    let rename = |n: &str| {
        if n == "SiN" {
            "AlN".to_string()
        } else {
            n.to_string()
        }
    };
    let layers = s
        .stack()
        .layers()
        .iter()
        .map(|l| Layer {
            name: rename(&l.name),
            material: if l.name == "SiN" {
                aln.clone()
            } else {
                l.material.clone()
            },
            ..l.clone()
        })
        .collect();
    let stack = LayerStack::new(
        s.stack().substrate().clone(),
        layers,
        s.stack().cladding().clone(),
    )
    .ok()?;
    let mut out = Structure::new(stack);
    for l in s.stack().layers() {
        for (shape, e) in s.shapes(&l.name).iter().zip(s.etches(&l.name)) {
            out.draw_etched(&rename(&l.name), shape.clone(), *e).ok()?;
        }
    }
    Some(Physical::of(out))
}

/// The round trip over every library device on the nitride stack, its film made 650 nm of
/// aluminium nitride (Xiong et al.'s film), at its defaults:
/// (made, refused, why, differing).
pub(crate) fn aln_round_trips() -> (usize, usize, Vec<String>, usize) {
    let (mut made, mut refused, mut why, mut differing) = (0, 0, Vec::new(), 0);
    for kind in crate::device::library()
        .iter()
        .map(|d| d.kind().to_string())
    {
        if GAAS_SKIP.contains(&kind.as_str()) {
            continue;
        }
        let Some((device, _)) = built(&kind, nitride(2.0), &[("thickness", "0.65")]) else {
            continue;
        };
        for values in designs_with(device.as_ref(), Some(&[])) {
            let Some(design) = device.physical(&values).ok().and_then(|d| on_aln(&d)) else {
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

fn aln_round_trip() -> Outcome {
    exact(aln_round_trips().3)
}

/// An aluminium nitride design: the library device `kind` on 2 µm of oxide, its film 650 nm of
/// AlN, its parameters `values`.
pub(crate) fn aln_design(kind: &str, values: &[(&str, &str)]) -> Option<Physical> {
    let mut all = vec![("thickness", "0.65")];
    all.extend_from_slice(values);
    built(kind, nitride(2.0), &all).and_then(|(_, d)| on_aln(&d))
}

/// The flows papers report for their own AlN devices: Xiong et al.'s 1 µm guides in 650 nm of
/// sputtered AlN on 2 µm of thermal oxide (no cladding stated for them) and Jung et al.'s rings
/// 3.5 µm wide under PECVD oxide.
pub(crate) fn aln_reported() -> Vec<(&'static str, Option<Physical>, &'static [&'static str])> {
    vec![
        (
            "xiong-2012",
            aln_design("waveguide", &[("width", "1")]),
            &[
                "si-wafer",
                "thermal-oxidation",
                "sputtering-aln",
                "ebl-hsq",
                "icp-aln-chlorine",
                "resist-strip",
            ],
        ),
        (
            "jung-2013",
            aln_design("all-pass ring", &[("width", "3.5"), ("radius", "50")]),
            &[
                "si-wafer",
                "thermal-oxidation",
                "sputtering-aln",
                "icp-aln-chlorine",
                "resist-strip",
                "pecvd-oxide",
            ],
        ),
    ]
}

/// The reported AlN sequences the generated flows don't contain, in order, or whose design isn't
/// given back exactly.
pub(crate) fn aln_reported_missing() -> Vec<&'static str> {
    let mut missing = Vec::new();
    for (paper, design, sequence) in aln_reported() {
        let ok = design.is_some_and(|d| {
            generate(&d, &Choices::default()).is_ok_and(|f| {
                contains(&f, sequence) && round_trip(&f, &d).is_ok_and(|t| t.exact())
            })
        });
        if !ok {
            missing.push(paper);
        }
    }
    missing
}

fn aln_reported_flows() -> Outcome {
    exact(aln_reported_missing().len())
}

/// GaAs on 2 µm of Al₀.₃Ga₀.₇As (the catalogue's default composition) on a GaAs wafer, under an
/// oxide cladding: Dietrich et al.'s GaAs/AlGaAs guides, the guide layer the device's core.
pub(crate) fn gaas() -> StackSpec {
    use crate::device::{LayerSpec, MaterialSpec};
    StackSpec::Layers {
        substrate: MaterialSpec::id("gaas"),
        layers: vec![
            LayerSpec {
                name: "AlGaAs".into(),
                thickness_um: 2.0,
                material: MaterialSpec::id("algaas"),
                background: MaterialSpec::id("algaas"),
            },
            LayerSpec {
                name: "GaAs".into(),
                thickness_um: 1.0,
                material: MaterialSpec::id("gaas"),
                background: MaterialSpec::id("sio2"),
            },
        ],
        cladding: MaterialSpec::id("sio2"),
        core: "GaAs".into(),
    }
}

/// The library's devices on the GaAs stack: all but the lithium niobate modulator and the
/// phase-change shifters, which have stacks of their own.
pub(crate) const GAAS_SKIP: [&str; 4] = [
    "TFLN Mach-Zehnder modulator",
    "GST phase-change phase shifter",
    "Sb2S3 phase-change phase shifter",
    "Sb2Se3 phase-change phase shifter",
];

fn gaas_round_trip() -> Outcome {
    exact(platform_round_trips(&gaas(), &GAAS_SKIP, Some(&[])).3)
}

/// The GaAs rules' choices with the sources' answers, on a Dietrich et al. ridge (2.5 µm wide,
/// 2 µm high) and the stack's directional coupler; each wrong answer counted. The guide layers
/// come grown with the wafer (Dietrich et al.: MBE or MOVPE), nothing deposited under them; the
/// guides' etch is said to be not specified (Dietrich et al. name none) and is timed, nothing
/// stopping it on the AlGaAs; the cladding is the coolest deposition, as no source gives GaAs's
/// budget, and the flow says so; and the design comes back to the bit.
pub(crate) fn gaas_rules_wrong() -> Vec<String> {
    let mut wrong = Vec::new();
    for (kind, values) in [
        ("waveguide", vec![("width", "2.5"), ("thickness", "2")]),
        ("directional coupler", vec![]),
    ] {
        let Some((_, design)) = built(kind, gaas(), &values) else {
            wrong.push(format!("{kind}: doesn't draw"));
            continue;
        };
        let Ok(flow) = generate(&design, &Choices::default()) else {
            wrong.push(format!("{kind}: refused"));
            continue;
        };
        let ids = ids(&flow);
        if ids.first().map(String::as_str) != Some("gaas-epitaxy")
            || ids
                .iter()
                .filter(|p| p.as_str() != "pecvd-oxide")
                .any(|p| super::process(p).is_some_and(|q| q.step == super::StepType::Deposition))
        {
            wrong.push(format!("{kind}: {ids:?}"));
        }
        if !flow
            .not_specified
            .iter()
            .any(|n| n.contains("the etch of the GaAs guides"))
        {
            wrong.push(format!("{kind}: the etch not said to be not specified"));
        }
        if !flow.steps.iter().any(|s| {
            matches!(
                s.operation,
                Operation::Etch {
                    depth: super::Depth::Timed(_),
                    ..
                }
            )
        }) {
            wrong.push(format!("{kind}: the etch isn't timed"));
        }
        if ids.last().map(String::as_str) != Some("pecvd-oxide")
            || !flow
                .not_specified
                .iter()
                .any(|n| n.contains("GaAs survives") || n.contains("whether GaAs"))
        {
            wrong.push(format!("{kind}: the cladding {:?}", ids.last()));
        }
        if !round_trip(&flow, &design).is_ok_and(|t| t.exact()) {
            wrong.push(format!("{kind}: not given back"));
        }
    }
    wrong
}

fn gaas_rules() -> Outcome {
    exact(gaas_rules_wrong().len())
}

pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "fabrication/nitride-round-trip",
            title: "Every library device built on the silicon nitride stack (all but the lithium niobate modulator) whose design the rules make gives back its structure and films to the bit through its generated flow, the nitride deposited on grown oxide: at its defaults and at its thickness's range ends, from etched to Damascene (differing points shown)",
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
        Case {
            id: "fabrication/pcm-round-trip",
            title: "The three phase-change shifters (GST, Sb₂S₃, Sb₂Se₃) give back their structure and films to the bit through their generated flows, the cells sputtered, lifted off and capped: at their defaults and each parameter's range ends in turn (differing points shown)",
            tier: Tier::Analytic,
            source: "an exact identity: the flow's operations draw the design's own shapes, etches and films",
            run: pcm_round_trip,
        },
        Case {
            id: "fabrication/pcm-rules",
            title: "The phase-change rules' choices with the sources' answers, for each shifter at its defaults: the film sputtered and lifted off, never etched; the cap sputtered; PECVD and LPCVD oxide refused above the film's crystallization temperature; the cap's unstated temperature flagged (wrong answers shown)",
            tier: Tier::Published,
            source: "M. Delaney et al., Adv. Funct. Mater. 30, 2002447 (2020), doi:10.1002/adfm.202002447, PDF pp. 5, 7 and 9; C. Ríos et al., Nat. Photonics 9, 725 (2015), doi:10.1038/nphoton.2015.182, Methods (PDF p. 9); the material catalogue's crystallization temperatures (Delaney et al. 2020; I. Friedrich et al., J. Appl. Phys. 87, 4130 (2000), doi:10.1063/1.373041, for GST)",
            run: pcm_rules,
        },
        Case {
            id: "fabrication/pcm-reported-flows",
            title: "The generated flow of Delaney et al.'s 23 nm Sb₂Se₃ cells on a 220 nm SOI rib etched 120 nm contains, in order, the steps they report (the rib etched, the film sputtered into windows opened in resist, the resist removed, the chip capped by sputtering), and gives the design back to the bit (sequences missing shown)",
            tier: Tier::Published,
            source: "M. Delaney et al., Sci. Adv. 7, eabg3500 (2021), doi:10.1126/sciadv.abg3500, Materials and Methods (PDF p. 6)",
            run: pcm_reported_flows,
        },
        Case {
            id: "fabrication/aln-round-trip",
            title: "Every library device drawn in 650 nm of aluminium nitride on 2 µm of grown oxide (all but the lithium niobate modulator and the phase-change shifters, whose cells no source in hand puts on AlN) whose design the rules make gives back its structure and films to the bit through its generated flow, the film sputtered and etched in chlorine: at its defaults (differing points shown)",
            tier: Tier::Analytic,
            source: "an exact identity: the flow's operations draw the design's own shapes, etches and films",
            run: aln_round_trip,
        },
        Case {
            id: "fabrication/aln-reported-flows",
            title: "The generated aluminium nitride flows contain, in order, the steps papers report for their own devices, and give the designs back to the bit: Xiong et al.'s guides (grown oxide, sputtered AlN, HSQ, a Cl₂/BCl₃/Ar etch) and Jung et al.'s rings under PECVD oxide (sequences missing shown)",
            tier: Tier::Published,
            source: "C. Xiong, W. H. P. Pernice, H. X. Tang, Nano Lett. 12, 3562 (2012), doi:10.1021/nl3011885, PDF pp. 2–4; H. Jung et al., Opt. Lett. 38, 2810 (2013), doi:10.1364/OL.38.002810, PDF pp. 1–2",
            run: aln_reported_flows,
        },
        Case {
            id: "fabrication/gaas-round-trip",
            title: "Every library device built on GaAs over 2 µm of AlGaAs on a GaAs wafer, under oxide (all but the lithium niobate modulator and the phase-change shifters), whose design the rules make gives back its structure and films to the bit through its generated flow: at its defaults (differing points shown)",
            tier: Tier::Analytic,
            source: "an exact identity: the flow's operations draw the design's own shapes, etches and films",
            run: gaas_round_trip,
        },
        Case {
            id: "fabrication/gaas-rules",
            title: "The GaAs rules' choices on a 2.5 µm by 2 µm ridge and a directional coupler: the guide layers grown with the wafer, the guides' etch said to be not specified and timed, the cladding the coolest deposition with GaAs's budget flagged as not specified, the design given back to the bit (wrong answers shown)",
            tier: Tier::Published,
            source: "C. P. Dietrich et al., Laser Photonics Rev. 10, 870 (2016), doi:10.1002/lpor.201500321, Section 2 (PDF p. 2) and PDF p. 12: epitaxy by MBE or MOVPE, ridges 2–4 µm wide and high, no etch process given",
            run: gaas_rules,
        },
    ]
}
