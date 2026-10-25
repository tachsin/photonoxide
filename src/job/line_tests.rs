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
