//! The quantum view as a job: the job the window writes runs from its text and records what the
//! window computes in place.

use std::path::Path;

use photonoxide::job::{self, Event};
use photonoxide::run::{Run, Stop};

use super::gaussian::{GaussianRequest, SqueezedInput, gaussian};
use super::quantum::{Input, statistics};
use super::quantum_job::{QuantumJobIn, quantum_job_text};
use super::*;

fn mzi() -> Chip {
    let (_, text) = example_texts()
        .into_iter()
        .find(|(f, _)| *f == "mzi.toml")
        .expect("the built-in MZI");
    Chip::parse(text).unwrap()
}

fn recorded(text: &str) -> Vec<Event> {
    let job = crate::studio::parse_job(text).unwrap();
    let mut run = Run::in_memory();
    job::execute(&job, &mut run, &Stop::new(job.timeout())).unwrap();
    run.lines()
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// The built-in MZI's photons and squeezed light, as a job: its record holds the numbers the
/// Quantum tab computes in place, to the bit.
#[test]
fn the_job_records_what_the_window_computes() {
    let chip = mzi();
    let ports: Vec<String> = chip.ports.iter().map(|p| p.name.clone()).collect();
    let request = QuantumJobIn {
        name: "mzi-quantum".into(),
        timeout_minutes: 1.0,
        task: serde_json::json!({
            "wavelength_um": 1.55,
            "photons": [{ "port": ports[0], "count": 1 }, { "port": ports[1], "count": 1 }],
            "sample": { "samples": 500, "seed": 1 },
            "squeezed": [{ "port": ports[0], "squeezing_db": 6.0 }],
        }),
    };
    let text = quantum_job_text(&chip, &request, Path::new(".")).unwrap();
    let events = recorded(&text);
    let stats = events
        .iter()
        .find_map(|e| match e {
            Event::QuantumStatistics { statistics } => Some(statistics.clone()),
            _ => None,
        })
        .unwrap();
    let window = statistics(
        &chip,
        Path::new("."),
        1.55,
        &[
            Input {
                port: ports[0].clone(),
                photons: 1,
            },
            Input {
                port: ports[1].clone(),
                photons: 1,
            },
        ],
    )
    .unwrap();
    let w = serde_json::to_value(&window).unwrap();
    assert_eq!(stats.all_arrive[0], w["all_arrive"][0].as_f64().unwrap());
    assert_eq!(stats.all_arrive[1], w["all_arrive"][1].as_f64().unwrap());
    assert_eq!(stats.bunching[0], w["bunching"][0].as_f64().unwrap());
    let squeezed = events
        .iter()
        .find_map(|e| match e {
            Event::QuantumSqueezed { squeezed } => Some(squeezed.clone()),
            _ => None,
        })
        .unwrap();
    let g = gaussian(
        &chip,
        Path::new("."),
        1.55,
        &GaussianRequest {
            inputs: vec![SqueezedInput {
                port: ports[0].clone(),
                squeezing_db: 6.0,
                angle_deg: 0.0,
                displacement: [0.0, 0.0],
            }],
            shown: ports[2].clone(),
            photons: 4,
            span: 4.0,
            points: 11,
        },
    )
    .unwrap();
    for (k, s) in g.states.iter().enumerate() {
        assert_eq!(squeezed.squeezed_db[k], s.squeezed_db);
        assert_eq!(squeezed.mean_photons[k], s.mean_photons);
    }
    assert_eq!(squeezed.purity, g.purity);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::QuantumSamples { .. }))
    );
}

/// The job's refusals come back as the library words them.
#[test]
fn a_job_the_library_refuses_says_why() {
    let request = QuantumJobIn {
        name: "nothing".into(),
        timeout_minutes: 1.0,
        task: serde_json::json!({ "wavelength_um": 1.55 }),
    };
    let e = quantum_job_text(&mzi(), &request, Path::new(".")).unwrap_err();
    assert!(e.contains("computes something"), "{e}");
}

/// The window calls the command by the name the program registers it under.
#[test]
fn the_window_and_the_program_name_the_job_command_alike() {
    let studio = include_str!("../studio.rs");
    let api = include_str!("../../../src/lib/api.ts");
    assert!(studio.contains("crate::circuits::quantum_job::chip_quantum_job,"));
    assert!(api.contains("(\"chip_quantum_job\""));
}
