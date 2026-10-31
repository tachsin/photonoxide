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
fn a_fabrication_run_writes_its_flows_json_traveller_and_cross_sections() {
    let root = dir("export");
    let job = Job::parse(HEATER).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events = replay(run.dir()).unwrap();
    let (flow, assessment) = (
        events.iter().find_map(|e| match e {
            Event::Flow { flow } => Some(flow.as_ref().clone()),
            _ => None,
        }),
        events.iter().find_map(|e| match e {
            Event::FlowAssessment { assessment } => Some(assessment.as_ref().clone()),
            _ => None,
        }),
    );
    let text = std::fs::read_to_string(run.dir().join("flow.json")).unwrap();
    let file = crate::export::flow::FlowFile::read(&text).unwrap();
    // the files hold what the record holds, to the bit
    assert_eq!(Some(file.flow.clone()), flow);
    assert_eq!(Some(file.assessment.clone()), assessment);
    assert_eq!(file.device, "thermo-optic phase shifter");
    let md = std::fs::read_to_string(run.dir().join("flow.md")).unwrap();
    assert_eq!(md, file.markdown());
    for k in 0..file.flow.steps.len() {
        let name = crate::export::flow::FlowExport::svg_name(k, file.flow.steps.len());
        let svg = std::fs::read_to_string(run.dir().join(&name)).unwrap();
        assert!(svg.starts_with("<?xml"), "{name}");
        assert!(md.contains(&name));
    }
}

/// The fenced block of `page` that opens with `fence`, its lines.
fn block<'a>(page: &'a str, fence: &str) -> Vec<&'a str> {
    let start = page
        .find(&format!("\n{fence}\n"))
        .unwrap_or_else(|| panic!("a {fence} block"))
        + fence.len()
        + 2;
    let end = start + page[start..].find("\n```\n").expect("the block ends");
    page[start..end].lines().collect()
}

/// docs/methods/fabrication.md's worked example: the strip's job (jobs/strip-flow.yaml, which
/// the page shows as written: job::tests checks that), what its Python block prints, which is the
/// flow's steps and dimensions, and the traveller's excerpt, each of its lines a line of the
/// `flow.md` the job's run writes, in order.
#[test]
fn the_write_ups_strip_flow_is_the_jobs_own() {
    let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let page = std::fs::read_to_string(here.join("docs/methods/fabrication.md"))
        .unwrap()
        .replace("\r\n", "\n");
    let job = Job::load(&here.join("jobs/strip-flow.yaml")).unwrap();
    let root = dir("write-up");
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let md = std::fs::read_to_string(run.dir().join("flow.md")).unwrap();
    let file = crate::export::flow::FlowFile::read(
        &std::fs::read_to_string(run.dir().join("flow.json")).unwrap(),
    )
    .unwrap();
    let printed: Vec<String> = file
        .flow
        .steps
        .iter()
        .enumerate()
        .map(|(k, s)| format!("{}. {} ({}): {}", k + 1, s.name, s.process, s.does))
        .chain(file.assessment.accuracy.iter().map(|d| d.text.clone()))
        .collect();
    assert_eq!(block(&page, "```text"), printed);
    assert_eq!(page.matches("\n```markdown\n").count(), 1);
    let excerpt = block(&page, "```markdown");
    assert!(excerpt.len() > 20, "{}", excerpt.len());
    let mut lines = md.lines();
    for line in excerpt.iter().filter(|l| !l.trim().is_empty()) {
        assert!(
            lines.any(|l| l == *line),
            "the traveller hasn't, in this order: {line}"
        );
    }
    // every dimension is in the excerpt's table
    for d in &file.assessment.accuracy {
        assert!(
            excerpt
                .iter()
                .any(|l| l.starts_with(&format!("| {} |", d.what))),
            "{}",
            d.what
        );
    }
}

#[test]
fn a_fabrication_job_made_from_a_design_is_one_that_runs() {
    let spec = crate::device::Spec::new("TFLN Mach-Zehnder modulator");
    let job = from_fabrication("tfln-flow", &spec, &["ebl-pmma".into()], &[]).unwrap();
    assert_eq!(job.name(), "tfln-flow");
    check(&job).unwrap();
    let text = job.to_text(crate::run::Format::Yaml).unwrap();
    assert!(text.contains("kind: fabrication"), "{text}");
    assert!(text.contains("ebl-pmma"), "{text}");
    assert!(!text.contains("avoid"), "{text}");
    let e = from_fabrication("tfln-flow", &spec, &["icp-si-hbr".into()], &[]).unwrap_err();
    assert!(e.to_string().contains("makes no step"), "{e}");
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
