//! The Smart Cut start: its physics, its route's numbers, and its flows against the bought
//! wafer's.

use super::checks::soi_strip;
use super::smart_cut::{
    self, CONSUMED, OXIDE, SILICON, depth, energy, projected, proportional, route, straggle,
};
use super::smart_cut_checks::{cases, round_trips, soi_design, stack_faults};
use super::{
    Choices, Physical, Start, StepType, assess, fabricate, fabricate_from, generate, generate_from,
    round_trip, stages,
};
use crate::device::{Device, TflnModulator, Values};
use crate::stack::Etch;
use crate::units::Length;

#[test]
fn the_tables_are_read_back_as_printed_and_rise_with_the_energy() {
    for rows in [&SILICON, &OXIDE] {
        for w in rows.windows(2) {
            assert!(w[0].0 < w[1].0 && w[0].1 < w[1].1 && w[0].2 < w[1].2);
            // a projected range is never longer than the path
            assert!(w[0].2 <= w[0].1);
        }
    }
    for r in &SILICON {
        let um = projected(r.0 * 1000.0).unwrap();
        assert_eq!(um, r.2 / smart_cut::DENSITY.0 * 1e4);
    }
    // 100 keV: 1.962e-4 g/cm² over 2.33 g/cm³, 842 nm
    assert!((projected(100.0).unwrap() - 0.8421).abs() < 1e-4);
    assert!(projected(9.0).is_none() && projected(1001.0).is_none());
    // between rows, between the rows' values
    let mid = projected(110.0).unwrap();
    assert!(mid > projected(100.0).unwrap() && mid < projected(125.0).unwrap());
}

#[test]
fn the_oxide_slows_the_protons_and_the_energy_inverts_the_depth() {
    assert_eq!(depth(100.0, 0.0), projected(100.0));
    assert!(depth(100.0, 0.5).unwrap() < depth(100.0, 0.0).unwrap());
    // a proton of 10 keV stops in a micrometre of oxide
    assert!(depth(10.0, 1.0).is_none());
    for (target, oxide) in [(0.3, 0.0), (0.93, 2.0), (2.0, 3.0)] {
        let e = energy(target, oxide).unwrap();
        assert!((depth(e, oxide).unwrap() - target).abs() < 1e-9 * target);
    }
    // deeper than 1 MeV reaches, and shallower than 10 keV does
    assert!(energy(50.0, 0.0).is_none());
    assert!(energy(0.05, 0.0).is_none());
}

#[test]
fn the_straggle_is_campbells_for_a_proton_in_silicon() {
    let k = (2.0 / 3.0) * 28f64.sqrt() / 29.0;
    assert!((straggle(1.0) - k).abs() < 1e-15);
    assert!((straggle(2.5) - 2.5 * k).abs() < 1e-15);
}

#[test]
fn bruels_proportion_holds_near_100_kev() {
    let (kev, ratio) = proportional();
    assert_eq!(kev, 100.0);
    assert!((ratio - 8.4206).abs() < 1e-3, "{ratio}");
}

#[test]
fn the_route_takes_off_what_the_split_film_has_beyond_the_design_to_the_bit() {
    for (film, oxide) in [(0.22, 2.0), (0.1, 1.0), (1.0, 3.0), (0.3, 2.0)] {
        let r = route(film, oxide).unwrap();
        // the executor takes the polish off the excess, then the oxidation the rest, exactly
        let left = Length::um(r.excess) - Length::um(r.polish);
        assert_eq!(left, Length::um(r.consumed), "{film} on {oxide}");
        assert_eq!(left - Length::um(r.consumed), Length::ZERO);
        assert_eq!(r.split, film + r.excess);
        assert!((r.split - film - 2.0 * r.straggle).abs() < 1e-12);
        assert!((r.straggle - straggle(r.range)).abs() < 1e-12);
        assert!((r.range - oxide - r.split).abs() < 1e-15);
        assert!((depth(r.energy, oxide).unwrap() - r.split).abs() < 1e-9);
        assert!((r.sacrificial * CONSUMED - r.consumed).abs() < 1e-15);
        assert!(r.energy > r.entering && r.entering > 10.0);
    }
    // 220 nm on 2 µm: the split leaves about 0.934 µm, implanted at about 318.7 keV
    let r = route(0.22, 2.0).unwrap();
    assert!((r.split - 0.9337).abs() < 1e-3, "{}", r.split);
    assert!((r.energy - 318.7).abs() < 0.1, "{}", r.energy);
    assert!((r.entering - 110.7).abs() < 0.1, "{}", r.entering);
    assert!(route(0.0, 2.0).is_err() && route(0.22, f64::NAN).is_err());
    // a film too thick for the table's energies
    assert!(route(40.0, 2.0).is_err());
}

#[test]
fn a_flow_bought_is_the_flow_it_always_was() {
    let design = soi_strip(0.5, Etch::FULL);
    let choices = Choices::default();
    assert_eq!(
        generate_from(&design, &choices, Start::Bought).unwrap(),
        generate(&design, &choices).unwrap()
    );
    assert_eq!(Start::default(), Start::Bought);
    assert_eq!(
        fabricate(&design, &choices).unwrap(),
        fabricate_from(&design, &choices, Start::Bought).unwrap()
    );
}

#[test]
fn a_flow_from_bare_silicon_prepends_the_smart_cut_route_and_makes_the_design() {
    let design = soi_strip(0.5, Etch::FULL);
    let bought = generate(&design, &Choices::default()).unwrap();
    let made = generate_from(&design, &Choices::default(), Start::SmartCut).unwrap();
    let route: Vec<&str> = made.steps[..9].iter().map(|s| s.process.as_str()).collect();
    assert_eq!(
        route,
        [
            "si-wafer",
            "thermal-oxidation",
            "h-implant",
            "direct-bonding",
            "smart-cut-split",
            "anneal-bond",
            "cmp-si",
            "thermal-oxidation",
            "hf-oxide-strip"
        ]
    );
    // the rest is the bought wafer's flow after its wafer
    assert_eq!(made.steps[9..], bought.steps[1..]);
    let record = made.record(None);
    let kinds: Vec<&str> = record.steps[..9]
        .iter()
        .map(|s| s.operation.as_str())
        .collect();
    assert_eq!(
        kinds,
        [
            "wafer",
            "deposit",
            "implant",
            "bond",
            "split",
            "anneal",
            "planarize",
            "oxidize",
            "wet_etch"
        ]
    );
    assert_eq!(made.steps[2].step, StepType::Implant);
    assert_eq!(made.steps[3].step, StepType::Bond);
    assert_eq!(made.steps[4].step, StepType::Split);
    assert!(record.references.iter().any(|r| r.key == "pstar-1993"));
    let trip = round_trip(&made, &design).unwrap();
    assert!(trip.exact(), "{trip:?}");
    // the wafer after the route is the bought one
    let after = &stages(&made).unwrap()[8];
    let bought_wafer = &stages(&bought).unwrap()[0];
    assert_eq!(after.structure.stack(), bought_wafer.structure.stack());
    assert!(assess(&design, &made).is_ok());
}

#[test]
fn the_smart_cut_start_is_refused_where_it_cant_make_the_wafer() {
    // a lithium niobate film isn't silicon on insulator
    let tfln = TflnModulator::new();
    let design = tfln.physical(&Values::defaults(&tfln)).unwrap();
    let e = generate_from(&design, &Choices::default(), Start::SmartCut).unwrap_err();
    assert!(e.to_string().contains("silicon on insulator"), "{e}");
    // avoiding a step the route needs
    let soi = soi_strip(0.5, Etch::FULL);
    for id in ["h-implant", "thermal-oxidation", "si-wafer"] {
        let avoid = Choices {
            avoid: vec![id.into()],
            ..Choices::default()
        };
        let e = generate_from(&soi, &avoid, Start::SmartCut).unwrap_err();
        assert!(e.to_string().contains(id), "{e}");
    }
    // the route's own processes may be preferred only where they make a step
    let prefer = Choices {
        prefer: vec!["cmp-si".into()],
        ..Choices::default()
    };
    assert!(generate_from(&soi, &prefer, Start::SmartCut).is_ok());
    assert!(generate(&soi, &prefer).is_err());
    assert_eq!(Start::parse("smart_cut").unwrap(), Start::SmartCut);
    assert_eq!(Start::parse("bought").unwrap(), Start::Bought);
    assert!(Start::parse("smart-cut").is_err());
}

#[test]
fn the_route_never_enters_a_bought_flow() {
    // none of the route's own processes is chosen or offered for a design's own layers
    for design in [
        soi_strip(0.5, Etch::FULL),
        soi_strip(0.5, Etch::partial(Length::um(0.1))),
    ]
    .into_iter()
    .chain(soi_design(0.3, 2.0))
    {
        let flow = generate(&design, &Choices::default()).unwrap();
        for s in &flow.steps {
            assert!(!super::library::route_only(
                super::process(&s.process).unwrap()
            ));
            for a in &s.alternatives {
                assert!(
                    super::process(&a.process).is_none_or(|p| !super::library::route_only(p)),
                    "{}",
                    a.process
                );
            }
        }
    }
}

#[test]
fn the_smart_cut_cases_pass() {
    assert_eq!(stack_faults(), Vec::<String>::new());
    let (runs, differing) = round_trips();
    assert!(runs > 2);
    assert_eq!(differing, 0);
    for case in cases() {
        let o = (case.run)();
        assert!(o.passed(), "{}: {o:?}", case.id);
    }
}

#[test]
fn a_design_of_its_own_stack_fabricates_from_bare_silicon() {
    let design: Physical = soi_design(0.3, 1.0).unwrap();
    let record = fabricate_from(&design, &Choices::default(), Start::SmartCut).unwrap();
    assert!(record.round_trip.unwrap().exact());
    assert!(
        record
            .not_specified
            .iter()
            .any(|n| n.starts_with("Smart Cut"))
    );
}
