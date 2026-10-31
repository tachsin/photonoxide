use super::checks::{
    designs, designs_of, etch_stop_wrong, lithography_wrong, reported_missing, round_trips,
    soi_strip, source_faults, thermal_wrong,
};
use super::*;
use crate::device::Values;
use crate::geometry::{Point, Shape};
use crate::stack::Etch;
use crate::units::Length;

fn rect(x: f64, y: f64, w: f64, h: f64) -> Shape {
    Shape::rect(Point::um(x, y), Length::um(w), Length::um(h)).unwrap()
}

/// The flow of a library device at its defaults.
fn default_flow(k: usize) -> (Flow, Physical) {
    let all = designs();
    let device = all[k].0.as_ref();
    let design = device.physical(&Values::defaults(device)).unwrap();
    (generate(&design, &Choices::default()).unwrap(), design)
}

#[test]
fn the_three_designs_flows_make_them_exactly() {
    for k in 0..3 {
        let (flow, design) = default_flow(k);
        let trip = round_trip(&flow, &design).unwrap();
        assert!(trip.exact(), "design {k}: {trip:?}\n{flow:#?}");
        assert!(trip.points > 1000, "{trip:?}");
    }
}

#[test]
fn the_flows_have_their_steps_in_order() {
    let ids = |k: usize| {
        default_flow(k)
            .0
            .steps
            .iter()
            .map(|s| s.process.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ids(0),
        [
            "soi-smart-cut",
            "duv-193",
            "icp-si-hbr",
            "resist-strip",
            "lpcvd"
        ]
    );
    assert_eq!(
        ids(1),
        [
            "lnoi-ion-slicing",
            "ebl-hsq",
            "ar-milling-ln",
            "resist-strip",
            "uv-photolithography",
            "evaporation",
            "lift-off",
            "pecvd-oxide"
        ]
    );
    assert_eq!(
        ids(2),
        [
            "soi-smart-cut",
            "duv-193",
            "icp-si-hbr",
            "resist-strip",
            "lpcvd",
            "cmp-oxide",
            "sputtering-tin",
            "uv-photolithography",
            "dry-etch-tin",
            "resist-strip",
            "pecvd-oxide",
            "contact-metal"
        ]
    );
}

#[test]
fn every_step_has_a_reason_and_every_flow_says_what_is_not_specified() {
    for k in 0..3 {
        let (flow, _) = default_flow(k);
        for s in &flow.steps {
            assert!(!s.reason.trim().is_empty(), "{}: no reason", s.name);
            assert!(!s.plain.trim().is_empty(), "{}: no plain words", s.name);
        }
        let record = flow.record(None);
        assert_eq!(record.library, LIBRARY_VERSION);
        assert!(!record.references.is_empty());
        // the record goes through JSON and back unchanged
        let text = serde_json::to_string(&record).unwrap();
        let back: FlowRecord = serde_json::from_str(&text).unwrap();
        assert_eq!(back, record);
    }
    // the TFLN rib's walls at 70° are outside argon milling's cited 60–63°
    let (tfln, _) = default_flow(1);
    assert!(
        tfln.warnings
            .iter()
            .any(|w| w.contains("70°") && w.contains("60 to 63°")),
        "{:#?}",
        tfln.warnings
    );
    // the heater's etch and contacts aren't specified by any source in hand
    let (heater, _) = default_flow(2);
    assert!(
        heater
            .not_specified
            .iter()
            .any(|n| n.contains("Titanium nitride by dry etching")),
        "{:#?}",
        heater.not_specified
    );
}

#[test]
fn every_round_trip_at_the_corners_is_exact() {
    // the defaults and the corners that draw: the strip's slab above its thickness and the
    // modulator's 0.9 µm slab on its 0.3 µm film don't
    for ((device, corners), designs) in designs().into_iter().zip([9, 49, 65]) {
        let (run, differing) = round_trips(device.as_ref(), corners);
        assert_eq!(run, designs, "{}", device.kind());
        assert_eq!(differing, 0, "{}", device.kind());
    }
}

#[test]
fn the_corners_include_designs_that_draw_and_leave_out_those_that_dont() {
    let all = designs();
    let (device, corners) = &all[1];
    let tfln = designs_of(device.as_ref(), corners);
    // the slab at its 0.9 µm maximum is above the 0.3 µm film's top: those corners don't draw
    assert!(tfln.len() < 65 && tfln.len() > 16, "{}", tfln.len());
}

#[test]
fn every_source_resolves_and_every_reference_is_used() {
    assert_eq!(source_faults(), Vec::<String>::new());
    let mut used: Vec<String> = library()
        .iter()
        .flat_map(|p| p.references.clone())
        .collect();
    for film in ["Si", "LiNbO3", "TiN", "Al", "Sb₂Se₃"] {
        let b = budget(film);
        used.extend(
            [&b.survives, &b.limit]
                .iter()
                .filter_map(|v| v.source().map(|s| s.reference.clone())),
        );
    }
    for r in references() {
        assert!(used.contains(&r.key), "{} is cited nowhere", r.key);
        assert!(!r.doi.is_empty() || !r.isbn.is_empty(), "{}", r.key);
    }
    let keys: Vec<String> = references().into_iter().map(|r| r.key).collect();
    let mut unique = keys.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), keys.len());
}

#[test]
fn a_capability_without_a_source_says_not_specified() {
    let tin = process("dry-etch-tin").unwrap();
    assert!(tin.capabilities.iter().all(|c| !c.value.is_stated()));
    assert_eq!(tin.capabilities[0].value.to_string(), "not specified");
    let duv = process("duv-193").unwrap();
    assert_eq!(
        duv.capabilities[0].value.to_string(),
        "150 to 500 nm",
        "{:?}",
        duv.capabilities[0]
    );
}

#[test]
fn deal_and_groves_law_inverts_and_its_tables_hold_together() {
    for (ambient, t) in [
        (oxidation::Ambient::Wet, 1100.0),
        (oxidation::Ambient::Dry, 1000.0),
    ] {
        let (a, b, tau) = oxidation::constants(ambient, t).unwrap();
        for hours in [0.1, 1.0, 10.0] {
            let x = oxidation::thickness(a, b, tau, hours);
            let back = oxidation::hours(a, b, tau, x);
            assert!(
                (back - hours).abs() < 1e-12 * hours.max(1.0),
                "{back} {hours}"
            );
        }
    }
    // the parabolic limit at long times: x² → B t
    let (a, b, tau) = oxidation::constants(oxidation::Ambient::Wet, 1200.0).unwrap();
    let x = oxidation::thickness(a, b, tau, 1e6);
    assert!((x * x / (b * 1e6) - 1.0).abs() < 1e-3);
    let worst = oxidation::ratio_consistency()
        .iter()
        .map(|r| r.2)
        .fold(0.0, f64::max);
    assert!(worst <= 1.0, "{worst}");
    assert!(oxidation::constants(oxidation::Ambient::Dry, 700.0).is_none());
}

#[test]
fn features_are_measured_as_drawn() {
    // one strip: its width, no gap
    let f = features(&[rect(5.0, 0.0, 10.0, 0.45)]).unwrap();
    assert_eq!(f.line_um, 0.45);
    assert_eq!(f.gap_um, None);
    // two strips 0.2 µm apart
    let f = features(&[rect(5.0, 0.0, 10.0, 0.5), rect(5.0, 0.9, 10.0, 0.9)]).unwrap();
    assert_eq!((f.line_um, f.gap_um), (0.5, Some(0.2)));
    // two overlapping shapes are one line
    let f = features(&[rect(5.0, 0.0, 10.0, 0.5), rect(12.0, 0.0, 6.0, 0.5)]).unwrap();
    assert_eq!((f.line_um, f.gap_um), (0.5, None));
    // a ring: its width, and its hole a gap
    let ring = Shape::ring(Point::um(0.0, 0.0), Length::um(5.0), Length::um(0.4)).unwrap();
    let f = features(&[ring]).unwrap();
    assert!((f.line_um - 0.4).abs() < 2e-3, "{f:?}");
    assert!(f.gap_um.is_some_and(|g| g > 9.0), "{f:?}");
}

#[test]
fn the_rules_answer_the_textbook_cases() {
    assert_eq!(etch_stop_wrong(), 0);
    assert_eq!(lithography_wrong(), 0);
    assert_eq!(thermal_wrong(), 0);
    assert_eq!(reported_missing(), Vec::<&str>::new());
}

#[test]
fn choices_prefer_and_avoid_processes() {
    let strip = soi_strip(0.5, Etch::FULL);
    let prefer = Choices {
        prefer: vec!["ebl-hsq".into()],
        avoid: Vec::new(),
    };
    let flow = generate(&strip, &prefer).unwrap();
    assert_eq!(flow.steps[1].process, "ebl-hsq");
    let avoid = Choices {
        prefer: Vec::new(),
        avoid: vec!["icp-si-hbr".into()],
    };
    let flow = generate(&strip, &avoid).unwrap();
    assert_eq!(flow.steps[2].process, "icp-si-cl2");
    assert!(round_trip(&flow, &strip).unwrap().exact());
    let unknown = Choices {
        prefer: vec!["laser-writing".into()],
        avoid: Vec::new(),
    };
    let e = generate(&strip, &unknown).unwrap_err().to_string();
    assert!(e.contains("laser-writing"), "{e}");
}

#[test]
fn a_flow_that_doesnt_hold_together_is_refused() {
    let (mut flow, _) = default_flow(0);
    // without the strip, resist would lie under the cladding
    flow.steps.retain(|s| s.process != "resist-strip");
    assert!(execute(&flow).is_err());
    let (mut flow, _) = default_flow(0);
    // without the cladding, the etched silicon is never filled
    flow.steps.retain(|s| s.process != "lpcvd");
    assert!(execute(&flow).is_err());
}

#[test]
fn a_design_without_a_crystalline_film_is_refused_with_why() {
    let mut s = crate::stack::Structure::new(
        crate::stack::LayerStack::nitride(Length::nm(300.0), Length::um(3.0)).unwrap(),
    );
    s.draw("SiN", rect(5.0, 0.0, 10.0, 1.0)).unwrap();
    let e = generate(&Physical::of(s), &Choices::default())
        .unwrap_err()
        .to_string();
    assert!(e.contains("single-crystal"), "{e}");
}

#[test]
fn the_stand_ins_are_the_physical_films() {
    let all = designs();
    let tfln = all[1].0.as_ref();
    let design = tfln.physical(&Values::defaults(tfln)).unwrap();
    let (_, layers, cladding) = design.names();
    assert_eq!(layers.last().unwrap().0, "Cu/Au");
    assert_eq!(cladding, "SiO2");
    assert_eq!(layers[1].0, "LiNbO3", "{layers:?}");
    // the structure is the optical one, stand-ins and all
    assert_eq!(
        *design.structure(),
        tfln.geometry(&Values::defaults(tfln)).unwrap().structure
    );
}

#[test]
fn the_rule_engine_answers_its_textbook_cases() {
    use super::rule_checks::*;
    assert!(
        franssila_misfits().iter().all(|m| *m <= 1.0),
        "{:?}",
        franssila_misfits()
    );
    let (tau, x) = campbell_example();
    // Campbell prints τ as 0.295 h where (0.1² + 0.5·0.1)/0.203 is 0.2956: cut, not rounded
    assert!(
        (tau - 0.2956).abs() <= 0.00005 && (x - 0.48).abs() <= 0.005,
        "{tau} {x}"
    );
    assert_eq!(below_limit(), Vec::<String>::new());
    assert_eq!(rule_choices_wrong(), Vec::<&str>::new());
    let (got, want) = combined_gap().unwrap();
    assert!(((got - want) / want).abs() <= 1e-12, "{got} {want}");
}

#[test]
fn every_dimension_of_the_three_designs_has_its_read_out() {
    for k in 0..3 {
        let (flow, design) = default_flow(k);
        let a = assess(&design, &flow).unwrap();
        assert!(!a.accuracy.is_empty() && !a.checks.is_empty() && !a.ranking.is_empty());
        for d in &a.accuracy {
            assert!(!d.text.is_empty() && !d.contributions.is_empty(), "{d:?}");
            // a stated bound has a source; a missing one says why
            for c in &d.contributions {
                assert_eq!(c.bound.is_some(), c.source.is_some(), "{c:?}");
            }
        }
        // the chosen process ranks first in every ranking
        for r in &a.ranking {
            assert!(r.ranked[0].chosen && r.ranked.iter().filter(|x| x.chosen).count() == 1);
        }
        // the assessment goes through JSON and back unchanged
        let text = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<Assessment>(&text).unwrap(), a);
    }
    // the strip's 220 nm film, from the SOI wafer's 8 nm spread (Bruel 1995)
    let (flow, design) = default_flow(0);
    let a = assess(&design, &flow).unwrap();
    let film = a
        .accuracy
        .iter()
        .find(|d| d.kind == Kind::Thickness && d.layer == "Si")
        .unwrap();
    assert_eq!(film.combined, Some(8.0));
}

#[test]
fn deal_and_groves_law_carries_between_rows_and_bounds_its_thickness() {
    use oxidation::{Ambient, grow, rates};
    // at a printed row the constants are the printed ones
    let r = rates(Ambient::Dry, 1000.0).unwrap();
    assert!(!r.carried && r.a == 0.165 && r.b == 0.0117 && r.tau == 0.37);
    // between rows they are carried by the Arrhenius law: B rises with temperature
    let mid = rates(Ambient::Dry, 1050.0).unwrap();
    assert!(mid.carried && mid.b > 0.0117 && mid.b < 0.027, "{mid:?}");
    assert!(rates(Ambient::Wet, 900.0).is_none());
    // the thickness grows with time and carries a bound from the constants' precision
    let (x1, b1) = grow(Ambient::Wet, 1100.0, 1.0, None).unwrap();
    let (x2, _) = grow(Ambient::Wet, 1100.0, 2.0, None).unwrap();
    assert!(x2 > x1 && b1 > 0.0 && b1 < x1, "{x1} {b1} {x2}");
}
