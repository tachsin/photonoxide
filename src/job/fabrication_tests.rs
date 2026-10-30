//! A `"fabrication"` job: checked, run, its record replayed to the bit on any threads.

use super::*;
use crate::run::{replay, replay_lines};

const TFLN: &str = "name = \"tfln-flow\"\n\n[device]\nkind = \"TFLN Mach-Zehnder modulator\"\n\n[task]\nkind = \"fabrication\"\n";

const HEATER: &str = "name = \"heater-flow\"\n\n[device]\nkind = \"thermo-optic phase shifter\"\n\n[task]\nkind = \"fabrication\"\n";

struct Dir(std::path::PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn dir(tag: &str) -> Dir {
    let d = std::env::temp_dir().join(format!(
        "photonoxide-fabrication-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    Dir(d)
}

/// The record's lines on a pool of `threads`, without the finish's (its time).
fn lines_on(text: &str, threads: usize, tag: &str) -> (Vec<String>, Vec<Event>) {
    let root = dir(&format!("{tag}-{threads}"));
    let job = Job::parse(text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    pool.install(|| execute(&job, &mut run, &Stop::new(None)))
        .unwrap();
    let lines = replay_lines(run.dir())
        .unwrap()
        .into_iter()
        .filter(|l| !l.starts_with("{\"type\":\"finished\""))
        .collect();
    let events = replay(run.dir()).unwrap();
    (lines, events)
}

#[test]
fn a_fabrication_job_records_its_flow_and_the_round_trip_is_exact() {
    for (text, steps) in [(TFLN, 8), (HEATER, 12)] {
        let job = Job::parse(text).unwrap();
        check(&job).unwrap();
        assert!(matches!(preview(&job).unwrap(), Event::Scene { .. }));
        let (_, events) = lines_on(text, 1, "record");
        let flow = events
            .iter()
            .find_map(|e| match e {
                Event::Flow { flow } => Some(flow),
                _ => None,
            })
            .expect("a flow");
        assert_eq!(flow.steps.len(), steps, "{flow:#?}");
        let trip = flow.round_trip.expect("its round trip");
        assert!(trip.exact(), "{trip:?}");
        assert_eq!(flow.library, crate::fabrication::LIBRARY_VERSION);
        assert!(events.iter().any(|e| matches!(e, Event::Scene { .. })));
    }
}

#[test]
fn a_fabrication_record_replays_and_repeats_on_any_threads() {
    let (one, events) = lines_on(TFLN, 1, "threads");
    let (four, _) = lines_on(TFLN, 4, "threads");
    assert_eq!(one, four);
    // each line read back and written again is the same line
    for (line, event) in one.iter().zip(&events) {
        assert_eq!(&serde_json::to_string(event).unwrap(), line);
    }
}

#[test]
fn a_fabrication_job_refuses_processes_the_library_hasnt_and_preferences_that_do_nothing() {
    let refused = |extra: &str, words: &str| {
        let job = Job::parse(&format!("{TFLN}{extra}\n")).unwrap();
        let e = check(&job).unwrap_err().to_string();
        assert!(e.contains(words), "{extra}: {e}");
    };
    refused("prefer = [\"laser-writing\"]", "laser-writing");
    refused(
        "avoid = [\"ebl-hsq\"]\nprefer = [\"ebl-hsq\"]",
        "both preferred and avoided",
    );
    // a silicon etch makes no step of a lithium niobate design
    refused("prefer = [\"icp-si-hbr\"]", "makes no step");
    // a preference that qualifies is taken
    let job = Job::parse(&format!("{TFLN}prefer = [\"ebl-pmma\"]\n")).unwrap();
    check(&job).unwrap();
}

#[test]
fn the_fabrication_view_has_the_wafer_after_every_step() {
    let job = Job::parse(TFLN).unwrap();
    let spec = super::device::spec_of(&job).unwrap();
    let view = fabrication_of(&spec, &[], &[]).unwrap();
    assert_eq!(view.stages.len(), view.flow.steps.len());
    assert!(view.flow.round_trip.unwrap().exact());
    // the rib's etch is in the last stage: 0.3 µm deep, walls at 70°, on the 0.6 µm film
    let last = view.stages.last().unwrap();
    let rib = last.etched.first().expect("an etched shape");
    assert_eq!(rib.layer, "LN");
    assert!((rib.depth_um - 0.3).abs() < 1e-12, "{rib:?}");
    assert!((rib.sidewall_deg - 70.0).abs() < 1e-9, "{rib:?}");
    assert!((rib.z_um[1] - rib.z_um[0] - 0.6).abs() < 1e-12, "{rib:?}");
    assert!(last.etched.iter().all(|e| e.layer == "LN"));
    assert!(matches!(last.scene, Event::Scene { .. })); // the first stage is the bare wafer, open to the air above it
    let Event::Scene {
        cladding, shapes, ..
    } = &view.stages[0].scene
    else {
        panic!("a scene")
    };
    assert!(shapes.is_empty());
    assert!(cladding.eps < 1.0 + 1e-12, "{cladding:?}");
}
