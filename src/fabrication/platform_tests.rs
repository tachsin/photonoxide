use super::platform_checks::*;
use super::*;

#[test]
fn nitride_flows_follow_their_sources_and_give_their_designs_back() {
    assert_eq!(nitride_rules_wrong(), Vec::<&str>::new());
    assert_eq!(nitride_reported_missing(), Vec::<&str>::new());
}

#[test]
fn phase_change_cells_are_sputtered_lifted_off_and_capped_cool() {
    assert_eq!(pcm_rules_wrong(), Vec::<String>::new());
    assert_eq!(pcm_reported_missing(), Vec::<&str>::new());
    // the film is the catalogue's, so its budget is the catalogue's crystallization temperature
    for (film, limit) in [("Sb2Se3", 200.0), ("Sb2S3", 270.0), ("Ge2Sb2Te5", 150.0)] {
        let b = budget(film);
        assert!(
            b.limit.high().is_some_and(|t| (t - limit).abs() < 0.5),
            "{film}: {b:?}"
        );
    }
}

#[test]
fn aluminium_nitride_is_sputtered_etched_in_chlorine_and_clad_as_its_papers_did() {
    assert_eq!(aln_reported_missing(), Vec::<&str>::new());
    let design = aln_design("waveguide", &[("width", "1")]).unwrap();
    let flow = generate(&design, &Choices::default()).unwrap();
    assert_eq!(
        ids(&flow),
        [
            "si-wafer",
            "thermal-oxidation",
            "sputtering-aln",
            "ebl-hsq",
            "icp-aln-chlorine",
            "resist-strip",
            "pecvd-oxide"
        ]
    );
    // HSQ at Xiong et al.'s 2:1 lasts the 650 nm etch; no source states a stop on the oxide
    assert!(
        flow.warnings.iter().all(|w| !w.contains("hard mask")),
        "{:?}",
        flow.warnings
    );
    assert!(matches!(
        flow.steps[4].operation,
        Operation::Etch {
            depth: Depth::Timed(_),
            ..
        }
    ));
    // the sputtering stays at or below 350 °C, and AlN is shown to survive the cladding's
    assert!(budget("AlN").survives.low() == Some(1000.0));
    assert!(round_trip(&flow, &design).unwrap().exact());
}

#[test]
fn gaas_comes_grown_and_its_etch_is_not_specified() {
    assert_eq!(gaas_rules_wrong(), Vec::<String>::new());
    let (_, design) = built("waveguide", gaas(), &[("width", "2.5")]).unwrap();
    let flow = generate(&design, &Choices::default()).unwrap();
    let etch = flow
        .steps
        .iter()
        .find(|s| s.process == "rie-gaas-chlorine")
        .unwrap();
    assert!(etch.reason.contains("is not specified"), "{}", etch.reason);
    assert!(etch.reason.contains("dietrich-2016"), "{}", etch.reason);
    assert_eq!(
        ids(&flow),
        [
            "gaas-epitaxy",
            "uv-photolithography",
            "rie-gaas-chlorine",
            "resist-strip",
            "pecvd-oxide"
        ]
    );
}

#[test]
fn library_devices_on_nitride_round_trip_at_their_defaults() {
    for kind in [
        "waveguide",
        "directional coupler",
        "all-pass ring",
        "MMI 1x2",
    ] {
        let (_, design) = built(kind, nitride(2.0), &[]).unwrap();
        let flow = generate(&design, &Choices::default()).unwrap();
        let trip = round_trip(&flow, &design).unwrap();
        assert!(trip.exact(), "{kind}: {trip:?}");
        let ids = ids(&flow);
        assert_eq!(
            ids[..3],
            ["si-wafer", "thermal-oxidation", "lpcvd"],
            "{kind}"
        );
        // the cladding, LPCVD oxide as Luke et al.'s, after the anneal
        assert_eq!(ids[ids.len() - 2..], ["anneal-nitride", "lpcvd"], "{kind}");
        for s in &flow.steps {
            assert!(!s.reason.trim().is_empty(), "{kind}: {}", s.name);
        }
    }
}

#[test]
fn a_damascene_flow_runs_stage_by_stage() {
    let design = nitride_guide(1.5, 1.2, 90.0).unwrap();
    let flow = generate(&design, &Choices::default()).unwrap();
    let stages = stages(&flow).unwrap();
    assert_eq!(stages.len(), flow.steps.len());
    // after the preform's etch the trench is air in the oxide; after the fill and the polish it
    // is nitride, the oxide around it
    let etch = flow
        .steps
        .iter()
        .position(|s| s.process == "damascene-preform")
        .unwrap();
    assert_eq!(stages[etch].layers[1], ("air".into(), "SiO2".into()));
    let fill = flow
        .steps
        .iter()
        .rposition(|s| s.process == "lpcvd")
        .unwrap();
    assert_eq!(stages[fill].layers[1], ("Si3N4".into(), "SiO2".into()));
    // avoiding the Damascene process keeps the etch, with crack stops and a warning
    let avoid = Choices {
        prefer: Vec::new(),
        avoid: vec!["damascene-preform".into()],
    };
    let etched = generate(&design, &avoid).unwrap();
    assert!(ids(&etched).contains(&"crack-stop-trenches".to_string()));
    assert!(round_trip(&etched, &design).unwrap().exact());
}

#[test]
fn a_bare_wafer_grows_its_oxide_by_deal_and_grove() {
    let design = nitride_guide(1.0, 0.3, 90.0).unwrap();
    let flow = generate(&design, &Choices::default()).unwrap();
    let grown = &flow.steps[1];
    assert_eq!(grown.process, "thermal-oxidation");
    // 2 µm in steam at 1100 °C: (2² + 0.11 × 2)/0.51 h
    let hours = (2.0f64 * 2.0 + 0.11 * 2.0) / 0.51;
    assert!(
        grown.reason.contains(&stated::number(hours)),
        "{}",
        grown.reason
    );
    assert_eq!(flow.steps[0].operation.describe(), "the wafer: bare Si");
}
