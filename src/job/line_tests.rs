//! A static job's `line` table ([`super::line`]): jobs/tfln-line.yaml's line against the
//! library's case of the same line, and what it refuses.

use super::*;
use crate::run::replay;

/// jobs/tfln-line.yaml on a coarser grid (8 nm at the edges, a 40 nm mode), for speed, with
/// `edit` applied to its text.
fn job_text(edit: impl Fn(String) -> String) -> String {
    let text = include_str!("../../jobs/tfln-line.yaml")
        .replace("fine_nm: 4.0", "fine_nm: 8.0")
        .replace("    step_nm: 20.0", "    step_nm: 40.0");
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
    let checks: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::GridConvergence { check, .. } => Some(check.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(checks.len(), 3, "{checks:?}");
    assert!(checks.iter().all(|c| c.converged()), "{checks:?}");
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
}
