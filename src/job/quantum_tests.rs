//! The `"quantum"` task beside a `[circuit]`: what its run records against closed forms, and
//! what it refuses.

use super::*;
use crate::portable;
use crate::run::{Job, Run, Stop};

/// A balanced coupler as a chip job's `[circuit]`, and `task`.
fn job(task: &str) -> Job {
    Job::parse(&format!(
        r#"
name = "coupler-quantum"
timeout_minutes = 1

[circuit]
connections = []

[[circuit.instance]]
name = "x"
component = "coupler"
values = {{ coupling = 0.5 }}

[[circuit.port]]
name = "a"
at = "x.o1"

[[circuit.port]]
name = "b"
at = "x.o2"

[[circuit.port]]
name = "c"
at = "x.o3"

[[circuit.port]]
name = "d"
at = "x.o4"

[task]
kind = "quantum"
wavelength_um = 1.55
{task}
"#
    ))
    .unwrap()
}

fn events(job: &Job) -> Vec<Event> {
    let mut run = Run::in_memory();
    super::super::execute(job, &mut run, &Stop::new(job.timeout())).unwrap();
    run.lines()
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// Two photons on a balanced coupler: never apart (Hong, Ou and Mandel), half the time apart when
/// told apart; sampled, never apart; the dip of identical photons falls to nothing; each photon's
/// route keeps half; squeezed light meets the vacuum and halves.
#[test]
fn a_balanced_coupler_records_its_closed_forms() {
    let j = job(r#"
photons = [{ port = "a", count = 1 }, { port = "b", count = 1 }]
sample = { samples = 2000, seed = 3 }
budget = [{ from = "a", to = "c" }]
squeezed = [{ port = "a", squeezing_db = 6.0 }]

[task.dip]
inputs = ["a", "b"]
outputs = ["c", "d"]
center_um = [1.55, 1.55]
fwhm_nm = [2.0, 2.0]
from_um = 1.5
to_um = 1.6
points = 401
delays = 101
"#);
    super::super::check(&j).unwrap();
    let ev = events(&j);
    assert!(matches!(ev.first(), Some(Event::Started { kind, .. }) if kind == "quantum"));
    assert!(matches!(ev.last(), Some(Event::Finished { .. })));
    let stats = ev
        .iter()
        .find_map(|e| match e {
            Event::QuantumStatistics { statistics } => Some(statistics),
            _ => None,
        })
        .unwrap();
    assert_eq!(stats.ports, ["a", "b", "c", "d"]);
    let p = |o: &[usize]| {
        stats
            .patterns
            .iter()
            .find(|q| q.occupations == o)
            .map_or(0.0, |q| q.probability[0])
    };
    let apart = |o: &[usize]| {
        stats
            .patterns
            .iter()
            .find(|q| q.occupations == o)
            .map_or(0.0, |q| q.probability[1])
    };
    assert!(p(&[0, 0, 1, 1]) < 1e-15);
    assert!((p(&[0, 0, 2, 0]) - 0.5).abs() < 1e-12);
    assert!((apart(&[0, 0, 1, 1]) - 0.5).abs() < 1e-12);
    assert!((stats.all_arrive[0] - 1.0).abs() < 1e-12);
    assert!((stats.bunching[0] - 1.0).abs() < 1e-12 && (stats.bunching[1] - 0.5).abs() < 1e-12);
    let samples = ev
        .iter()
        .find_map(|e| match e {
            Event::QuantumSamples { samples } => Some(samples),
            _ => None,
        })
        .unwrap();
    assert_eq!(samples.counts.iter().map(|c| c.count).sum::<usize>(), 2000);
    assert!(
        samples
            .counts
            .iter()
            .all(|c| c.occupations == [0, 0, 2, 0] || c.occupations == [0, 0, 0, 2])
    );
    assert!(samples.distance < 4.0 * samples.expected);
    let dip = ev
        .iter()
        .find_map(|e| match e {
            Event::QuantumDip { dip } => Some(dip),
            _ => None,
        })
        .unwrap();
    assert!(dip.coincidences[50] < 1e-9, "{}", dip.coincidences[50]);
    assert!(dip.delays_ps[50].abs() < 1e-12);
    assert!((dip.distinguishable - 0.5).abs() < 1e-9);
    assert!((dip.visibility - 1.0).abs() < 1e-8);
    let budget = ev
        .iter()
        .find_map(|e| match e {
            Event::QuantumBudget { budget } => Some(budget),
            _ => None,
        })
        .unwrap();
    assert_eq!(budget.legs[0].through, ["x"]);
    assert!((budget.legs[0].transmission - 0.5).abs() < 1e-12);
    assert!((budget.event - 0.5).abs() < 1e-12);
    assert_eq!(budget.thresholds.len(), 2);
    let sq = ev
        .iter()
        .find_map(|e| match e {
            Event::QuantumSqueezed { squeezed } => Some(squeezed),
            _ => None,
        })
        .unwrap();
    let r = 6.0 * std::f64::consts::LN_10 / 20.0;
    let half = 10.0 * portable::log10(0.5 * (portable::exp(-2.0 * r) + 1.0));
    assert!(
        (sq.squeezed_db[2] - half).abs() < 1e-12,
        "{:?}",
        sq.squeezed_db
    );
    assert!((sq.purity - 1.0).abs() < 1e-9);
}

/// A job made from a chip by `quantum_from_chip` runs the same as one written out.
#[test]
fn a_job_from_a_chip_runs_as_written() {
    let written = job(r#"photons = [{ port = "a", count = 2 }]"#);
    let chip = super::super::chip::chip_of(&written).unwrap();
    let mut task = toml::Table::new();
    task.insert("wavelength_um".into(), 1.55.into());
    let mut on = toml::Table::new();
    on.insert("port".into(), "a".into());
    on.insert("count".into(), 2.into());
    task.insert(
        "photons".into(),
        toml::Value::Array(vec![toml::Value::Table(on)]),
    );
    let made = quantum_from_chip("coupler-quantum", &chip, &task, 1.0).unwrap();
    let stats = |j: &Job| {
        events(j).into_iter().find_map(|e| match e {
            Event::QuantumStatistics { statistics } => Some(statistics),
            _ => None,
        })
    };
    assert_eq!(stats(&written), stats(&made));
}

/// Each limit refuses with its numbers.
#[test]
fn a_quantum_task_refuses_what_it_cant_take() {
    let refused = |task: &str| super::super::check(&job(task)).unwrap_err().to_string();
    assert!(refused("").contains("computes something"));
    assert!(refused(r#"photons = [{ port = "z", count = 1 }]"#).contains("no port z"));
    assert!(refused(r#"photons = [{ port = "a", count = 9 }]"#).contains("1 to 8"));
    assert!(
        refused(
            r#"photons = [{ port = "a", count = 1 }]
sample = { samples = 0, seed = 1 }"#
        )
        .contains("1 to 1000000 samples")
    );
    assert!(refused(r#"squeezed = [{ port = "a", squeezing_db = 30.0 }]"#).contains("0 to 20 dB"));
    assert!(
        refused(
            r#"[task.dip]
inputs = ["a", "b"]
outputs = ["c", "d"]
center_um = [1.55, 1.55]
fwhm_nm = [0.1, 1.0]
from_um = 1.5
to_um = 1.6
points = 101"#
        )
        .contains("fewer than 4 of the band's steps")
    );
    assert!(refused(r#"wrong = 1"#).contains("unknown field"));
}
