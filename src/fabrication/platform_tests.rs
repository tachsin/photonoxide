use super::platform_checks::*;
use super::*;

#[test]
fn nitride_flows_follow_their_sources_and_give_their_designs_back() {
    assert_eq!(nitride_rules_wrong(), Vec::<&str>::new());
    assert_eq!(nitride_reported_missing(), Vec::<&str>::new());
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
