//! A static job's `line` table ([`super::line`]): jobs/tfln-line.yaml's line against the
//! library's case of the same line, and what it refuses.

use super::*;
use crate::run::replay;

/// jobs/tfln-line.yaml on a coarser grid (8 nm at the edges, a 40 nm mode; the full-wave grid
/// from 40 nm, its response from 6 solves), for speed, with `edit` applied to its text.
fn job_text(edit: impl Fn(String) -> String) -> String {
    let text = include_str!("../../jobs/tfln-line.yaml")
        .replace("fine_nm: 4.0", "fine_nm: 8.0")
        .replace("    step_nm: 20.0", "    step_nm: 40.0")
        .replace("      fine_nm: 20.0", "      fine_nm: 40.0\n      sweep: 6");
    edit(text)
}

fn run_job(tag: &str, text: &str) -> Vec<Event> {
    let dir = std::env::temp_dir().join(format!("photonoxide-line-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let job = Job::parse_as(text, crate::run::Format::Yaml).unwrap();
    check(&job).unwrap();
    let mut run = Run::create(&dir, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events = replay(run.dir()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    events
}

fn values(events: &[Event]) -> Vec<statics::StaticValue> {
    events
        .iter()
        .find_map(|e| match e {
            Event::StaticResult { values, .. } => Some(values.clone()),
            _ => None,
        })
        .expect("a result")
}

fn named(values: &[statics::StaticValue], name: &str) -> f64 {
    values
        .iter()
        .find(|v| v.name.starts_with(name))
        .unwrap_or_else(|| panic!("no {name} in {values:?}"))
        .value
}

#[test]
fn a_job_gives_the_line_the_librarys_case_does() {
    let events = run_job("tfln", &job_text(|t| t));
    let v = values(&events);
    // the library's line (validation case line/zhang-2022-*): 2.2585 at 100 GHz, 41.08 Ω and
    // 15.24 dB/cm at 250 GHz; the job's regions leave two slivers of thermal oxide
    let n = named(&v, "line at 100 GHz: n_m");
    assert!((n - 2.2585).abs() < 0.01, "{n} {v:?}");
    let z = named(&v, "line at 200 GHz (beyond quasi-TEM): Re Z0");
    assert!((z - 41.14).abs() < 0.5, "{z}");
    let loss = named(&v, "line at 300 GHz (beyond quasi-TEM): loss");
    assert!((loss - 17.09).abs() < 0.5, "{loss}");
    let bandwidth = named(&v, "line: EO 3 dB bandwidth");
    assert!(bandwidth > 50.0 && bandwidth < 150.0, "{bandwidth}");
    assert!(named(&v, "Vpi L (push-pull") > 2.0);
    // the full-wave line (validation cases line/full-wave-*): within 0.5% of quasi-TEM's index
    // and 1% of its impedance at 50 GHz, and at 300 GHz the loss quasi-TEM misses, the silicon
    // half-space's radiation among it
    let n = named(&v, "full-wave line at 50 GHz: n_m");
    assert!((n - named(&v, "line at 50 GHz: n_m")).abs() < 0.012, "{n}");
    let z = named(&v, "full-wave line at 50 GHz: Re Z0");
    assert!((z - named(&v, "line at 50 GHz: Re Z0")).abs() < 0.4, "{z}");
    let loss = named(&v, "full-wave line at 300 GHz: loss");
    assert!(
        loss > named(&v, "line at 300 GHz (beyond quasi-TEM): loss") + 1.0,
        "{loss}"
    );
    let radiated = named(&v, "full-wave line at 300 GHz: radiation loss");
    assert!(radiated > 0.5 && radiated < 3.0, "{radiated}");
    let full = named(&v, "full-wave line: EO 3 dB bandwidth");
    assert!(
        (full - bandwidth).abs() < 0.2 * bandwidth,
        "{full} {bandwidth}"
    );
    let checks: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::GridConvergence { check, .. } => Some(check.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(checks.len(), 6, "{checks:?}");
    // the quasi-TEM line's grids converge; the full-wave's are this test's coarse ones
    assert!(checks[..3].iter().all(|c| c.converged()), "{checks:?}");
    assert!(
        checks[3].quantity.starts_with("full-wave line n_m"),
        "{checks:?}"
    );
}

/// jobs/tfln-segmented.yaml on a coarser grid (8 nm at the edges, a 40 nm mode), with `edit`
/// applied to its text.
fn segmented_text(edit: impl Fn(String) -> String) -> String {
    let text = include_str!("../../jobs/tfln-segmented.yaml")
        .replace("fine_nm: 4.0", "fine_nm: 8.0")
        .replace("    step_nm: 20.0", "    step_nm: 40.0");
    edit(text)
}

#[test]
fn a_segmented_job_gives_the_line_the_librarys_case_does() {
    let events = run_job("segmented", &segmented_text(|t| t));
    let v = values(&events);
    // the library's line (validation case line/kharel-2021-segmented-measured-*): n_B 2.2074
    // and Re Z_B 44.08 Ω at 50 GHz, the gaps loaded too 2.2548; the job's regions leave the
    // rib's walls two slivers of air
    let n = named(&v, "segmented line at 50 GHz: n_B");
    assert!((n - 2.2074).abs() < 0.01, "{n} {v:?}");
    let z = named(&v, "segmented line at 50 GHz: Re Z_B");
    assert!((z - 44.08).abs() < 0.5, "{z}");
    let bound = named(&v, "segmented line at 50 GHz: n_B with the gaps loaded");
    assert!((bound - 2.2548).abs() < 0.01, "{bound}");
    // the channels alone: a lower index, a higher impedance
    assert!(named(&v, "line at 50 GHz: n_m") < n - 0.3);
    assert!(named(&v, "line at 50 GHz: Re Z0") > z + 5.0);
    let quasi = named(&v, "segmented line: n (quasi-static");
    // below the wave's, which the metal's internal inductance raises
    assert!(quasi < n && n - quasi < 0.03, "{quasi} {n}");
    assert!(named(&v, "Vpi L (push-pull") > 2.0);
    let checks: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::GridConvergence { check, .. } => Some(check.clone()),
            _ => None,
        })
        .collect();
    assert!(
        checks
            .iter()
            .any(|c| c.quantity.starts_with("segmented line n_B")),
        "{checks:?}"
    );
    assert!(checks.iter().all(|c| c.converged()), "{checks:?}");
}

#[test]
#[ignore = "runs jobs/tfln-line.yaml on its own grids and prints its record"]
fn print_the_line_job() {
    let t = std::time::Instant::now();
    let events = run_job("line-full", include_str!("../../jobs/tfln-line.yaml"));
    for v in values(&events) {
        println!("{}: {} {}", v.name, v.value, v.unit);
    }
    for e in &events {
        if let Event::GridConvergence { check, .. } = e {
            println!(
                "grid: {} {} {:?}",
                check.quantity, check.reason, check.error
            );
        }
    }
    println!("{:.0} s", t.elapsed().as_secs_f64());
}

#[test]
#[ignore = "runs jobs/tfln-segmented.yaml on its own grids and prints its record"]
fn print_the_segmented_job() {
    let events = run_job(
        "segmented-full",
        include_str!("../../jobs/tfln-segmented.yaml"),
    );
    for v in values(&events) {
        println!("{}: {} {}", v.name, v.value, v.unit);
    }
    for e in &events {
        if let Event::GridConvergence { check, .. } = e {
            println!(
                "grid: {} {} {:?}",
                check.quantity, check.reason, check.error
            );
        }
    }
}

#[test]
fn a_segmented_table_is_refused_where_it_cant_be_one() {
    let refused = |edit: &dyn Fn(String) -> String, what: &str| {
        let job = Job::parse_as(&segmented_text(edit), crate::run::Format::Yaml).unwrap();
        let e = check(&job).expect_err(what).to_string();
        assert!(e.contains(what), "{e}");
    };
    refused(
        &|t| t.replace("{ conductor: 3, of: 1 }", "{ conductor: 1, of: 2 }"),
        "a rail is a conductor of its own",
    );
    refused(
        &|t| t.replace("{ conductor: 4, of: 2 }", "{ conductor: 4, of: 3 }"),
        "a rail is a conductor of its own",
    );
    refused(
        &|t| t.replace("{ conductor: 4, of: 2 }", "{ conductor: 5, of: 2 }"),
        "conductor = 5",
    );
    refused(
        &|t| t.replace("{ conductor: 3, of: 1 }", "{ conductor: 3, of: 2 }"),
        "a rail hanging from the signal",
    );
    refused(
        &|t| t.replace("rail_um: 45.0", "rail_um: 50.0"),
        "rail_um must be above 0 and below period_um",
    );
}

#[test]
fn a_line_table_is_refused_where_it_cant_be_one() {
    let refused = |edit: &dyn Fn(String) -> String, what: &str| {
        let job = Job::parse_as(&job_text(edit), crate::run::Format::Yaml).unwrap();
        let e = check(&job).expect_err(what).to_string();
        assert!(e.contains(what), "{e}");
    };
    refused(&|t| t.replace("signal: 1", "signal: 3"), "signal = 3");
    refused(
        &|t| t.replace("PECVD: 0.006, SiO2", "glass: 0.006, SiO2"),
        "loss_tangent names glass",
    );
    refused(
        &|t| t.replace("conductivity_s_per_m: 5.8e7", "conductivity_s_per_m: -1.0"),
        "positive conductivity",
    );
    refused(
        &|t| {
            t.replace(
                "frequencies_ghz: [10.0, 50.0, 100.0, 200.0, 300.0]",
                "frequencies_ghz: []",
            )
        },
        "frequencies_ghz",
    );
    refused(
        &|t| t.replace("{ bottom: 200.0, right: 200.0 }", "{ under: 200.0 }"),
        "full_wave.open_um names the side \"under\"",
    );
    refused(
        &|t| t.replace("{ bottom: 200.0, right: 200.0 }", "{ bottom: -1.0 }"),
        "thicker than 0",
    );
    refused(
        &|t| t.replace("sweep: 6", "sweep: 2"),
        "full_wave.sweep takes 4",
    );
    refused(
        &|t| {
            t.replace(
                "      sweep: 6",
                "      sweep: 6\n      along: { glass: 4.0 }",
            )
        },
        "full_wave.along names glass",
    );
}

#[test]
fn the_catalogue_gives_the_tangents_the_table_doesnt() {
    let text = super::line_checks::PLATES.replace(
        "    catalogue_loss_tangents: true\n",
        "    catalogue_loss_tangents: true\n    loss_tangent: { SiO2: 0.006 }\n",
    );
    assert!(text.contains("SiO2: 0.006"));
    let v = values(&run_job("catalogue", &text));
    let tangent = |m: &str| {
        v.iter()
            .find(|x| x.name.starts_with(&format!("line: loss tangent of {m} (")))
            .unwrap_or_else(|| panic!("no {m} in {v:?}"))
            .clone()
    };
    // the table's own is kept
    let oxide = tangent("SiO2");
    assert_eq!(oxide.value, 0.006);
    assert!(oxide.name.ends_with("(given)"), "{}", oxide.name);
    // silicon's the catalogue's nearest 70.7 GHz: Lamb's Table I at 100 GHz
    let si = tangent("Si");
    assert!((si.value - 19e-4).abs() < 1e-15, "{}", si.value);
    for part in ["Silicon", "100 GHz", "lamb-1996", "Table I", "70.7107 GHz"] {
        assert!(si.name.contains(part), "{part}: {}", si.name);
    }
    // the air above, which the catalogue hasn't, stays lossless
    let air = tangent("n = 1");
    assert_eq!(air.value, 0.0);
    assert!(air.name.contains("lossless"), "{}", air.name);
    // the line's tangent weighs them by their shares
    let (q_ox, q_si) = (
        named(&v, "line: SiO2's share"),
        named(&v, "line: Si's share"),
    );
    let tan = named(&v, "line: loss tangent (the media's");
    assert!((tan - (q_ox * 0.006 + q_si * 19e-4)).abs() < 1e-15, "{tan}");
    // without it nothing is taken: the given tangent alone, and no notes
    let plain = values(&run_job(
        "no-catalogue",
        &text.replace(
            "catalogue_loss_tangents: true",
            "catalogue_loss_tangents: false",
        ),
    ));
    assert!(
        !plain
            .iter()
            .any(|x| x.name.starts_with("line: loss tangent of"))
    );
    let tan = named(&plain, "line: loss tangent (the media's");
    assert!((tan - q_ox * 0.006).abs() < 1e-15, "{tan}");
}

#[test]
fn a_crystal_is_found_by_its_formula_and_a_band_holds_its_frequency() {
    // lithium niobate as the tensor stack names it, at 120 GHz inside Lee's 91 to 147 GHz: the
    // larger axis's top, the field along the optic axis
    let (t, from) =
        super::line::catalogue_tangent("LiNbO₃ (x-cut, its y along the chip's x)", 120.0).unwrap();
    assert!((t - 7.971e-3).abs() < 1e-12, "{t}");
    assert!(
        from.contains("along the optic axis") && from.contains("lee-2001-linbo3"),
        "{from}"
    );
    // fused silica at 150 GHz: Parshin's three grades and Harvey's band, the largest (QU's top)
    let (t, from) = super::line::catalogue_tangent("SiO2", 150.0).unwrap();
    assert!((t - 15e-4).abs() < 1e-15, "{t}");
    assert!(from.contains("grade QU"), "{from}");
    // a region the catalogue hasn't
    assert!(super::line::catalogue_tangent("PECVD", 50.0).is_none());
}

#[test]
fn a_full_wave_table_is_refused_on_a_segmented_line() {
    let job = Job::parse_as(
        &segmented_text(|t| {
            t.replace("  line:\n", "  line:\n    full_wave: { fine_nm: 40.0 }\n")
                .replace(
                    "  line:\r\n",
                    "  line:\r\n    full_wave: { fine_nm: 40.0 }\r\n",
                )
        }),
        crate::run::Format::Yaml,
    )
    .unwrap();
    let e = check(&job).expect_err("refused").to_string();
    assert!(e.contains("full_wave is for a uniform line"), "{e}");
}
