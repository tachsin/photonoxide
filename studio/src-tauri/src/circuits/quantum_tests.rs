//! The quantum view's commands: what they compute against closed forms, and what they refuse.

use std::path::Path;

use photonoxide::Complex64 as c64;
use photonoxide::circuit::{SMatrix, Spectrum};
use photonoxide::units::Wavelength;

use super::quantum::{
    BudgetRequest, DipRequest, Input, Leg, MOST_PATTERNS, MOST_PHOTONS, Photon, budget, hom,
    statistics,
};
use super::*;

fn here() -> &'static Path {
    Path::new(".")
}

fn placed(name: &str, kind: &str, values: &[(&str, f64)]) -> Placed {
    Placed {
        name: name.into(),
        kind: kind.into(),
        x: 0.0,
        y: 0.0,
        rotation: 0,
        mirror: false,
        values: values.iter().map(|&(k, v)| (k.into(), v)).collect(),
        file: None,
        convention: None,
        device: None,
    }
}

fn exposed(name: &str, at: &str) -> External {
    External {
        name: name.into(),
        at: at.into(),
        x: 0.0,
        y: 0.0,
        rotation: 0,
    }
}

/// One ideal coupler of `coupling`, its ports `a`, `b` (inputs) and `c`, `d` (outputs).
fn coupler(coupling: f64) -> Chip {
    let mut chip = Chip::new("coupler");
    chip.instances
        .push(placed("x", "coupler", &[("coupling", coupling)]));
    for (name, at) in [("a", "x.o1"), ("b", "x.o2"), ("c", "x.o3"), ("d", "x.o4")] {
        chip.ports.push(exposed(name, at));
    }
    chip
}

fn mzi() -> Chip {
    let (_, text) = example_texts()
        .into_iter()
        .find(|(f, _)| *f == "mzi.toml")
        .expect("the built-in MZI");
    Chip::parse(text).unwrap()
}

fn on(ports: &[(&str, usize)]) -> Vec<Input> {
    ports
        .iter()
        .map(|&(port, photons)| Input {
            port: port.into(),
            photons,
        })
        .collect()
}

#[test]
fn two_photons_on_a_balanced_coupler_leave_together() {
    let st = statistics(&coupler(0.5), here(), 1.55, &on(&[("a", 1), ("b", 1)])).unwrap();
    let json = serde_json::to_value(&st).unwrap();
    assert_eq!(json["ports"], serde_json::json!(["a", "b", "c", "d"]));
    assert_eq!(json["loss_modes"], 0);
    assert_eq!(json["photons"], 2);
    // the patterns of every photon arriving: 2 photons in 4 ports
    assert_eq!(json["arriving_patterns"], 10);
    // none goes back to a, b: those 7 are left out
    assert_eq!(json["never"], 7);
    assert_eq!(json["patterns"].as_array().unwrap().len(), 3);
    let p = |occupations: [usize; 4], k: usize| -> f64 {
        json["patterns"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["occupations"] == serde_json::json!(occupations))
            .map_or(0.0, |r| r["probability"][k].as_f64().unwrap())
    };
    // Hong, Ou and Mandel: never one in each output, while distinguishable photons split half
    // the time; both kinds keep every photon
    assert!(p([0, 0, 1, 1], 0) < 1e-15, "{}", p([0, 0, 1, 1], 0));
    assert!((p([0, 0, 1, 1], 1) - 0.5).abs() < 1e-15);
    assert!((p([0, 0, 2, 0], 0) - 0.5).abs() < 1e-15);
    assert!((p([0, 0, 2, 0], 1) - 0.25).abs() < 1e-15);
    for k in 0..2 {
        assert!((json["all_arrive"][k].as_f64().unwrap() - 1.0).abs() < 1e-14);
    }
    assert!((json["bunching"][0].as_f64().unwrap() - 1.0).abs() < 1e-14);
    assert!((json["bunching"][1].as_f64().unwrap() - 0.5).abs() < 1e-14);
    // a detector on c sees 0 or 2 photons, half the time each, and 1 on average
    let c = &json["marginals"][2];
    assert_eq!(c["port"], "c");
    let alike: Vec<f64> = serde_json::from_value(c["probabilities"][0].clone()).unwrap();
    assert!((alike[0] - 0.5).abs() < 1e-14 && alike[1] < 1e-15 && (alike[2] - 0.5).abs() < 1e-14);
    assert!((c["mean"][0].as_f64().unwrap() - 1.0).abs() < 1e-14);
    // the transfer matrix as drawn: |S| of 1/√2 between the sides, none back
    let m = &json["magnitude"];
    assert!((m[2][0].as_f64().unwrap() - 0.5f64.sqrt()).abs() < 1e-15);
    assert_eq!(m[0][0].as_f64().unwrap(), 0.0);
}

#[test]
fn a_lossy_chip_loses_photons_to_its_loss_modes() {
    let st = statistics(&mzi(), here(), 1.55, &on(&[("in1", 1), ("in2", 1)])).unwrap();
    let json = serde_json::to_value(&st).unwrap();
    assert!(json["loss_modes"].as_u64().unwrap() > 0);
    assert!(json["dilation_error"].as_f64().unwrap() < 1e-12);
    // the arms lose 2.7 dB/cm over 100 and 150 µm: both photons arrive a little less than
    // always, and each port's mean photon number sums to what arrives
    for k in 0..2 {
        let all = json["all_arrive"][k].as_f64().unwrap();
        assert!(all < 1.0 && all > 0.98, "{all}");
        let shown: f64 = json["patterns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["probability"][k].as_f64().unwrap())
            .sum::<f64>()
            + json["others"][k].as_f64().unwrap();
        assert!((shown - all).abs() < 1e-14);
    }
}

#[test]
fn the_statistics_refuse_what_the_view_cant_take() {
    let chip = coupler(0.5);
    let refused = |inputs: &[(&str, usize)], wavelength: f64| {
        statistics(&chip, here(), wavelength, &on(inputs)).unwrap_err()
    };
    assert!(refused(&[], 1.55).contains("put a photon"));
    let many = refused(&[("a", MOST_PHOTONS), ("b", 1)], 1.55);
    assert!(many.contains(&format!("up to {MOST_PHOTONS}")), "{many}");
    let far = refused(&[("a", 1)], 1.31);
    assert!(far.contains("in the chip's sweep, 1.5 to 1.6 µm"), "{far}");
    // the library's own: a port that isn't one
    let port = refused(&[("e", 1)], 1.55);
    assert!(port.contains("no port e"), "{port}");
    // an incomplete chip says why it can't be computed
    let mut open = chip.clone();
    open.ports.pop();
    assert!(statistics(&open, here(), 1.55, &on(&[("a", 1)])).is_err());

    // too many patterns: eight lossy guides, 16 ports and their loss modes
    let mut guides = Chip::new("guides");
    for k in 0..8 {
        let name = format!("g{k}");
        guides.instances.push(placed(&name, "waveguide", &[]));
        guides
            .ports
            .push(exposed(&format!("i{k}"), &format!("{name}.o1")));
        guides
            .ports
            .push(exposed(&format!("o{k}"), &format!("{name}.o2")));
    }
    let e = statistics(&guides, here(), 1.55, &on(&[("i0", 4), ("i1", 4)])).unwrap_err();
    assert!(e.contains(&format!("up to {MOST_PATTERNS}")), "{e}");
    // and too many ports
    guides.instances.push(placed("g8", "waveguide", &[]));
    guides.ports.push(exposed("i8", "g8.o1"));
    guides.ports.push(exposed("o8", "g8.o2"));
    let e = statistics(&guides, here(), 1.55, &on(&[("i0", 1)])).unwrap_err();
    assert!(e.contains("18 external ports"), "{e}");
}

#[test]
fn a_chip_with_gain_is_refused_by_the_library() {
    let dir = std::env::temp_dir().join(format!("photonoxide-gain-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // a two-port that gives back 1.44 times the power it gets
    let wavelengths: Vec<Wavelength> = (0..5)
        .map(|k| Wavelength::um(1.5 + 0.025 * f64::from(k)).unwrap())
        .collect();
    let gain = SMatrix::from_fn(2, |i, j| c64::new(if i == j { 0.0 } else { 1.2 }, 0.0));
    let spectrum =
        Spectrum::new(vec!["o1".into(), "o2".into()], wavelengths, vec![gain; 5]).unwrap();
    let path = dir.join("gain.s2p");
    write_touchstone(&path, &spectrum).unwrap();
    let mut chip = Chip::new("gain");
    let mut amp = placed("amp", MEASURED, &[]);
    amp.file = Some(path.display().to_string());
    amp.convention = Some(TimeConvention::Physics);
    chip.instances.push(amp);
    chip.ports.push(exposed("in", "amp.o1"));
    chip.ports.push(exposed("out", "amp.o2"));
    let e = statistics(&chip, &dir, 1.55, &on(&[("in", 1)])).unwrap_err();
    assert!(e.contains("isn't passive"), "{e}");
    std::fs::remove_dir_all(&dir).unwrap();
}

fn photon(center_um: f64, fwhm_nm: f64) -> Photon {
    Photon { center_um, fwhm_nm }
}

fn dip(photons: [Photon; 2], span_ps: Option<f64>) -> DipRequest {
    DipRequest {
        inputs: ["a".into(), "b".into()],
        outputs: ["c".into(), "d".into()],
        photons,
        span_ps,
        points: 101,
    }
}

#[test]
fn identical_photons_on_a_balanced_coupler_dip_to_nothing() {
    let view = hom(&coupler(0.5), here(), &dip([photon(1.55, 1.0); 2], None)).unwrap();
    let json = serde_json::to_value(&view).unwrap();
    // V = 2RT/(R² + T²) = 1 at R = T, at no delay; and half the pairs split far from it
    assert!((json["visibility"].as_f64().unwrap() - 1.0).abs() < 1e-9);
    assert!(json["deepest_ps"].as_f64().unwrap().abs() < 1e-12);
    assert!((json["distinguishable"].as_f64().unwrap() - 0.5).abs() < 1e-12);
    let c: Vec<f64> = serde_json::from_value(json["coincidences"].clone()).unwrap();
    assert!((c[0] - 0.5).abs() < 1e-6, "{}", c[0]);
    assert!(json["notes"].as_array().unwrap().is_empty());
    // an unbalanced coupler: V = 2RT/(R² + T²)
    let view = hom(&coupler(0.3), here(), &dip([photon(1.55, 1.0); 2], None)).unwrap();
    let v = serde_json::to_value(&view).unwrap()["visibility"]
        .as_f64()
        .unwrap();
    assert!((v - 2.0 * 0.3 * 0.7 / (0.09 + 0.49)).abs() < 1e-9, "{v}");
}

#[test]
fn the_dip_refuses_what_the_sweep_doesnt_resolve() {
    let chip = coupler(0.5);
    // the default sweep's step is 0.2 nm: a 0.5 nm photon spans 2.5 of them
    let e = hom(
        &chip,
        here(),
        &dip([photon(1.55, 0.5), photon(1.55, 1.0)], None),
    )
    .unwrap_err();
    assert!(
        e.contains("photon 1's width") && e.contains("0.2000 nm"),
        "{e}"
    );
    let e = hom(&chip, here(), &dip([photon(1.55, 1.0); 2], Some(100.0))).unwrap_err();
    assert!(e.contains("more than π"), "{e}");
    let e = hom(&chip, here(), &dip([photon(1.55, 1.0); 2], Some(-1.0))).unwrap_err();
    assert!(e.contains("positive"), "{e}");
    let e = hom(
        &chip,
        here(),
        &dip([photon(1.7, 1.0), photon(1.55, 1.0)], None),
    )
    .unwrap_err();
    assert!(e.contains("photon 1") && e.contains("sweep"), "{e}");
    let mut same = dip([photon(1.55, 1.0); 2], None);
    same.outputs = ["c".into(), "c".into()];
    assert!(
        hom(&chip, here(), &same)
            .unwrap_err()
            .contains("two different output ports")
    );
    same.points = 1;
    assert!(
        hom(&chip, here(), &same)
            .unwrap_err()
            .contains("2 to 2001 delays")
    );
    // a photon near the sweep's end is computed, with a note
    let view = hom(&chip, here(), &dip([photon(1.597, 2.0); 2], None)).unwrap();
    let notes = serde_json::to_value(&view).unwrap()["notes"].clone();
    assert_eq!(notes.as_array().unwrap().len(), 2, "{notes}");
}

fn leg(from: &str, to: &str) -> Leg {
    Leg {
        from: from.into(),
        to: to.into(),
        route: None,
    }
}

#[test]
fn the_budget_follows_each_route_through_the_mzi() {
    let request = BudgetRequest {
        wavelength_um: 1.55,
        photons: vec![leg("in1", "out1"), leg("in2", "out2")],
        source: 0.9,
        detector: 0.8,
    };
    let view = budget(&mzi(), here(), &request).unwrap();
    let json = serde_json::to_value(&view).unwrap();
    let first = &json["photons"][0];
    let routes = first["routes"].as_array().unwrap();
    // two arms; each route the source, three hops and the detector
    assert_eq!(routes.len(), 2);
    let through: Vec<Vec<String>> = routes
        .iter()
        .map(|r| serde_json::from_value(r["through"].clone()).unwrap())
        .collect();
    assert_eq!(
        through,
        [["split", "upper", "combine"], ["split", "lower", "combine"]]
    );
    for r in routes {
        let stages = r["stages"].as_array().unwrap();
        assert_eq!(stages.len(), 5);
        assert_eq!(stages[0]["name"], "source");
        assert_eq!(stages[4]["name"], "detector");
        let product: f64 = stages
            .iter()
            .map(|s| s["transmission"].as_f64().unwrap())
            .product();
        assert!((product / r["transmission"].as_f64().unwrap() - 1.0).abs() < 1e-14);
        // half at each coupler, the source and the detector, and the arm's loss
        let t = r["transmission"].as_f64().unwrap();
        assert!(t < 0.25 * 0.72 && t > 0.24 * 0.72, "{t}");
    }
    // the route kept is the one that keeps the most: the shorter arm
    assert_eq!(first["chosen"], 0);
    assert_eq!(first["more"], false);
    // the event: both photons, each along its route
    let event = json["event"].as_f64().unwrap();
    let chosen = |k: usize| {
        let p = &json["photons"][k];
        let c = p["chosen"].as_u64().unwrap() as usize;
        p["routes"][c]["transmission"].as_f64().unwrap()
    };
    assert!((event / (chosen(0) * chosen(1)) - 1.0).abs() < 1e-14);
    assert_eq!(json["thresholds"].as_array().unwrap().len(), 2);
    // a route chosen by hand
    let mut by_hand = request.clone();
    by_hand.photons[0].route = Some(1);
    let view = serde_json::to_value(budget(&mzi(), here(), &by_hand).unwrap()).unwrap();
    assert_eq!(view["photons"][0]["chosen"], 1);
}

#[test]
fn the_budget_refuses_what_it_cant_take() {
    let ask = |photons: Vec<Leg>, source: f64, detector: f64| BudgetRequest {
        wavelength_um: 1.55,
        photons,
        source,
        detector,
    };
    let refused = |r: BudgetRequest| budget(&mzi(), here(), &r).unwrap_err();
    assert!(refused(ask(vec![], 1.0, 1.0)).contains("give a photon"));
    let many = vec![leg("in1", "out1"); MOST_PHOTONS + 1];
    assert!(refused(ask(many, 1.0, 1.0)).contains(&format!("up to {MOST_PHOTONS}")));
    assert!(refused(ask(vec![leg("in1", "out1")], 1.2, 1.0)).contains("source's efficiency"));
    assert!(refused(ask(vec![leg("in1", "out1")], 1.0, 0.0)).contains("detector's efficiency"));
    // the library's: a route back to where it started, and a port that isn't one
    assert!(refused(ask(vec![leg("in1", "in1")], 1.0, 1.0)).contains("one port to another"));
    assert!(refused(ask(vec![leg("in1", "x")], 1.0, 1.0)).contains("no external port x"));
    // a photon with no route has no event
    let view = budget(
        &coupler(1.0),
        here(),
        &BudgetRequest {
            wavelength_um: 1.55,
            photons: vec![leg("a", "d")],
            source: 1.0,
            detector: 1.0,
        },
    )
    .unwrap();
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(json["event"], 0.0);
    assert!(json["event_db"].is_null());
    assert!(json["photons"][0]["chosen"].is_null());
}

/// The window calls each command by the name the program registers it under.
#[test]
fn the_window_and_the_program_name_the_commands_alike() {
    let studio = include_str!("../studio.rs");
    let api = include_str!("../../../src/lib/api.ts");
    for command in [
        "chip_quantum_statistics",
        "chip_hom_dip",
        "chip_loss_budget",
    ] {
        assert!(
            studio.contains(&format!("crate::circuits::quantum::{command},")),
            "{command} isn't registered in studio.rs"
        );
        assert!(
            api.contains(&format!("(\"{command}\"")),
            "api.ts doesn't call {command}"
        );
    }
}
