//! The quantum view's sampling: the exact sampler against the exact distribution, batch after
//! batch, under its hard limit, and what it refuses.

use std::path::Path;

use super::quantum::Input;
use super::sampling::{Count, SampleRequest, sampled};
use super::*;

fn here() -> &'static Path {
    Path::new(".")
}

/// One ideal coupler of `coupling`, its ports `a`, `b` (inputs) and `c`, `d` (outputs).
fn coupler(coupling: f64) -> Chip {
    let mut chip = Chip::new("coupler");
    chip.instances.push(Placed {
        name: "x".into(),
        kind: "coupler".into(),
        x: 0.0,
        y: 0.0,
        rotation: 0,
        mirror: false,
        values: [("coupling".to_owned(), coupling)].into_iter().collect(),
        file: None,
        convention: None,
        device: None,
    });
    for (name, at) in [("a", "x.o1"), ("b", "x.o2"), ("c", "x.o3"), ("d", "x.o4")] {
        chip.ports.push(External {
            name: name.into(),
            at: at.into(),
            x: 0.0,
            y: 0.0,
            rotation: 0,
        });
    }
    chip
}

fn request(inputs: &[(&str, usize)], batch: usize, seed: u64, counts: Vec<Count>) -> SampleRequest {
    SampleRequest {
        inputs: inputs
            .iter()
            .map(|&(port, photons)| Input {
                port: port.into(),
                photons,
            })
            .collect(),
        batch,
        seed,
        limit_ms: 5000,
        counts,
    }
}

/// Two photons on a balanced coupler leave together (Hong, Ou and Mandel): the sampler never
/// draws a coincidence, splits them half and half, and comes as close to the exact distribution
/// as N samples let it.
#[test]
fn a_balanced_coupler_samples_its_bunched_pairs() {
    let v = sampled(
        &coupler(0.5),
        here(),
        1.55,
        &request(&[("a", 1), ("b", 1)], 4000, 7, vec![]),
    )
    .unwrap();
    assert_eq!(v.samples, 4000);
    assert_eq!(v.drawn, 4000);
    assert!(!v.stopped);
    assert_eq!(v.impossible, 0);
    // only (c, c) and (d, d)
    assert_eq!(v.counts.len(), 2, "{:?}", v.counts);
    for c in &v.counts {
        assert!(c.occupations == [0, 0, 2, 0] || c.occupations == [0, 0, 0, 2]);
    }
    // the expected distance at N = 4000: ½ · 2 √(2 · ¼/(π N))
    let expected = (0.5 / (std::f64::consts::PI * 4000.0)).sqrt();
    assert!((v.expected - expected).abs() < 1e-12, "{}", v.expected);
    assert!(
        v.distance < 4.0 * v.expected,
        "{} against {}",
        v.distance,
        v.expected
    );
    assert_eq!(v.patterns.len(), 2);
    assert!((v.patterns[0].exact - 0.5).abs() < 1e-12);
}

/// Batches add up: the counts handed back grow by each batch, and the same seed draws the same
/// samples.
#[test]
fn batches_add_to_the_counts_so_far() {
    let chip = coupler(0.3);
    let first = sampled(&chip, here(), 1.55, &request(&[("a", 2)], 1000, 1, vec![])).unwrap();
    let again = sampled(&chip, here(), 1.55, &request(&[("a", 2)], 1000, 1, vec![])).unwrap();
    assert_eq!(first.counts, again.counts);
    let second = sampled(
        &chip,
        here(),
        1.55,
        &request(&[("a", 2)], 1000, 2, first.counts.clone()),
    )
    .unwrap();
    assert_eq!(second.samples, 2000);
    let total: usize = second.counts.iter().map(|c| c.count).sum();
    assert_eq!(total, 2000);
    assert!(second.distance < 4.0 * second.expected);
}

/// The hard limit stops a batch, and says so.
#[test]
fn the_hard_limit_stops_a_batch() {
    let mut r = request(&[("a", 1), ("b", 1)], 20_000, 3, vec![]);
    r.limit_ms = 1;
    let v = sampled(&coupler(0.5), here(), 1.55, &r).unwrap();
    assert!(v.stopped && v.drawn < 20_000, "{} drawn", v.drawn);
}

/// Each limit refuses with its numbers.
#[test]
fn the_sampling_refuses_what_it_cant_take() {
    let chip = coupler(0.5);
    let refused = |r: SampleRequest| sampled(&chip, here(), 1.55, &r).unwrap_err();
    assert!(refused(request(&[], 10, 1, vec![])).contains("photon on a port first"));
    assert!(refused(request(&[("a", 9)], 10, 1, vec![])).contains("up to 8"));
    assert!(refused(request(&[("a", 1)], 0, 1, vec![])).contains("1 to 20000 samples"));
    let mut slow = request(&[("a", 1)], 10, 1, vec![]);
    slow.limit_ms = 6000;
    assert!(refused(slow).contains("1 to 5000 ms"));
    let wrong = vec![Count {
        occupations: vec![1, 1],
        count: 3,
    }];
    assert!(refused(request(&[("a", 1)], 10, 1, wrong)).contains("start the sampling again"));
    assert!(refused(request(&[("z", 1)], 10, 1, vec![])).contains("no port z"));
}

/// The window calls the command by the name the program registers it under.
#[test]
fn the_window_and_the_program_name_the_sampling_command_alike() {
    let studio = include_str!("../studio.rs");
    let api = include_str!("../../../src/lib/api.ts");
    assert!(studio.contains("crate::circuits::sampling::chip_sample,"));
    assert!(api.contains("(\"chip_sample\""));
}
