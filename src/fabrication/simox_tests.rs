//! The SIMOX start: its dose balance against the published oxides, its refusals, its route's
//! numbers, and its flows against the bought wafer's.

use super::checks::soi_strip;
use super::simox::{ANNEAL, ANNEAL_LEAST, MADE, OXYGEN, balance, oxide, oxides, route};
use super::simox_checks::{BELOW_1300, cases, round_trips, stack_faults};
use super::smart_cut_checks::soi_design;
use super::{Choices, Start, StepType, assess, fabricate_from, generate, generate_from, stages};
use crate::stack::Etch;

#[test]
fn the_oxygen_of_an_oxide_is_campbells_density_and_fraction() {
    // 2 × 0.44 × 5×10²² cm⁻³
    assert!((OXYGEN / 4.4e22 - 1.0).abs() < 1e-15, "{OXYGEN:e}");
}

#[test]
fn the_dose_balance_gives_the_published_doses_oxides() {
    let nm = |dose: f64| balance(dose) * 1e3;
    // Celler and Cristoloveanu's thin and low-dose oxides, 56 nm and about 100 nm
    assert!((nm(2e17) - 45.4545).abs() < 1e-3);
    assert!((nm(4e17) - 90.9091).abs() < 1e-3);
    // Izumi's 2100 Å and Hemment's 2800 ± 140 Å: below 1300 °C, a quarter to a third less
    assert!((nm(1.2e18) - 272.727).abs() < 1e-2);
    assert!((nm(1.8e18) - 409.091).abs() < 1e-2);
    for (_, dose, measured) in BELOW_1300 {
        let fraction = measured / nm(dose);
        assert!(fraction > 0.6 && fraction < 0.8, "{fraction}");
    }
    // Lam's doses span 0.27 to 0.5 µm
    let (lo, hi) = oxides();
    assert!((lo - 0.272727).abs() < 1e-5 && (hi - 0.5).abs() < 1e-12);
}

#[test]
fn the_balance_is_refused_below_1300_c_and_outside_the_published_doses() {
    assert!(oxide(1.2e18, ANNEAL).is_ok());
    assert!(oxide(1.2e18, ANNEAL_LEAST).is_ok());
    let e = oxide(1.2e18, 1150.0).unwrap_err();
    assert!(e.to_string().contains("precipitates"), "{e}");
    assert!(oxide(1e17, ANNEAL).is_err());
    assert!(oxide(3e18, ANNEAL).is_err());
    assert!(oxide(f64::NAN, ANNEAL).is_err());
}

#[test]
fn the_route_thins_the_annealed_film_to_the_design() {
    let r = route(0.15, 0.4).unwrap();
    assert!((r.dose - 1.76e18).abs() < 1e6);
    assert_eq!(
        (r.energy, r.anneal_temperature, r.anneal_hours),
        (200.0, 1350.0, 4.0)
    );
    assert_eq!(r.implant_temperature, 600.0);
    assert_eq!(r.made, MADE);
    assert_eq!(r.thinned, MADE - 0.15);
    assert!((r.sacrificial - 0.05 / 0.44).abs() < 1e-12);
    assert!((r.consumed + r.swelling - 0.4).abs() < 1e-15);
    assert!(r.sacrificial_hours > 0.0 && r.strip_minutes > 0.0);
    // the film the anneal leaves, kept whole
    let whole = route(MADE, 0.3).unwrap();
    assert_eq!((whole.thinned, whole.sacrificial_hours), (0.0, 0.0));
}

#[test]
fn the_route_refuses_what_the_published_conditions_dont_make() {
    // the photonic standard, 0.22 µm on 2 µm: neither film nor oxide
    let e = route(0.22, 2.0).unwrap_err().to_string();
    assert!(e.contains("buried oxide"), "{e}");
    let e = route(0.22, 0.4).unwrap_err().to_string();
    assert!(e.contains("epitaxial"), "{e}");
    assert!(route(0.1, 0.25).is_err());
    assert!(route(0.1, 0.55).is_err());
    assert!(route(0.0, 0.4).is_err() && route(0.1, f64::INFINITY).is_err());
}

#[test]
fn a_flow_from_bare_silicon_prepends_the_simox_route_and_makes_the_design() {
    let design = soi_design(0.15, 0.4).unwrap();
    let bought = generate(&design, &Choices::default()).unwrap();
    let made = generate_from(&design, &Choices::default(), Start::Simox).unwrap();
    // five steps in place of the bought wafer
    assert_eq!(made.steps.len(), bought.steps.len() - 1 + 5);
    assert_eq!(made.steps[1].step, StepType::Implant);
    assert_eq!(made.steps[2].step, StepType::Anneal);
    // the dose as a power of ten
    let reason = &made.steps[1].reason;
    assert!(reason.contains("to 1.76×10¹⁸ cm⁻²"), "{reason}");
    let record = made.record(None);
    let ops: Vec<&str> = record.steps[..5]
        .iter()
        .map(|s| s.operation.as_str())
        .collect();
    assert_eq!(ops, ["wafer", "implant", "anneal", "oxidize", "wet_etch"]);
    assert!(
        record.steps[2].does.contains("heals"),
        "{}",
        record.steps[2].does
    );
    for key in [
        "izumi-1978",
        "celler-cristoloveanu-2003",
        "lam-1987",
        "hemment-1984",
    ] {
        assert!(record.references.iter().any(|r| r.key == key), "{key}");
    }
    assert!(made.not_specified.iter().any(|n| n.starts_with("SIMOX")));
    // the steps after the route are the bought flow's
    assert_eq!(made.steps[5..], bought.steps[1..]);
    assert!(stages(&made).is_ok() && assess(&design, &made).is_ok());
    let rec = fabricate_from(&design, &Choices::default(), Start::Simox).unwrap();
    let trip = rec.round_trip.unwrap();
    assert!(trip.structure_equal && trip.films_equal && trip.differing == 0);
    // the film the anneal leaves needs no thinning: three steps
    let whole = soi_design(MADE, 0.3).unwrap();
    let flow = generate_from(&whole, &Choices::default(), Start::Simox).unwrap();
    let bought = generate(&whole, &Choices::default()).unwrap();
    assert_eq!(flow.steps.len(), bought.steps.len() - 1 + 3);
}

#[test]
fn the_simox_start_is_refused_where_it_cant_make_the_wafer() {
    // the library's strip, 0.22 µm on 2 µm
    let strip = soi_strip(0.5, Etch::FULL);
    let e = generate_from(&strip, &Choices::default(), Start::Simox).unwrap_err();
    assert!(e.to_string().contains("buried oxide"), "{e}");
    let found = super::starts(&strip, &Choices::default()).unwrap();
    assert_eq!(found.starts, [Start::Bought, Start::SmartCut]);
    assert!(found.simox.is_none() && found.simox_refused.is_some());
    let thin = soi_design(0.1, 0.4).unwrap();
    let found = super::starts(&thin, &Choices::default()).unwrap();
    assert_eq!(found.starts, [Start::Bought, Start::SmartCut, Start::Simox]);
    assert!(found.simox.is_some() && found.simox_refused.is_none());
    // avoiding a step the route needs; the other route's own steps may be avoided
    for id in [
        "o-implant",
        "simox-anneal",
        "thermal-oxidation",
        "hf-oxide-strip",
        "si-wafer",
    ] {
        let avoid = Choices {
            avoid: vec![id.into()],
            ..Choices::default()
        };
        let e = generate_from(&thin, &avoid, Start::Simox).unwrap_err();
        assert!(e.to_string().contains(id), "{e}");
    }
    let avoid = |id: &str| Choices {
        avoid: vec![id.into()],
        ..Choices::default()
    };
    assert!(generate_from(&thin, &avoid("h-implant"), Start::Simox).is_ok());
    assert!(generate_from(&thin, &avoid("o-implant"), Start::SmartCut).is_ok());
    assert_eq!(Start::parse("simox").unwrap(), Start::Simox);
    assert_eq!(Start::Simox.name(), "simox");
    assert!(Start::parse("SIMOX").is_err());
}

#[test]
fn the_simox_cases_pass() {
    assert_eq!(stack_faults(), Vec::<String>::new());
    let (runs, differing) = round_trips();
    assert_eq!((runs, differing), (12, 0));
    for case in cases() {
        let o = (case.run)();
        assert!(o.passed(), "{}: {o:?}", case.id);
    }
}
