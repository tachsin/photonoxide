use std::path::Path;

use photonoxide::job::{self, Event, SceneShape};
use photonoxide::run::{Format, Job};

use super::*;

/// The repository's job files and the library's templates without a device: name, text, format.
fn jobs() -> Vec<(String, String, Format)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jobs");
    let mut all: Vec<(String, String, Format)> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter_map(|p| {
            let format = Format::from_path(&p)?;
            Some((
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap().replace("\r\n", "\n"),
                format,
            ))
        })
        .collect();
    all.extend(
        job::TEMPLATES
            .iter()
            .map(|t| (format!("{}.yaml", t.name), t.text.to_owned(), Format::Yaml)),
    );
    all.sort_by(|a, b| a.0.cmp(&b.0));
    all
}

/// Whether `a` and `b` hold the same shapes, in any order.
fn same_shapes(a: &[SceneShape], b: &[SceneShape]) -> bool {
    a.len() == b.len() && added(a, b).is_empty()
}

#[test]
fn every_shape_a_jobs_scene_draws_is_found_in_one_entry_of_its_text() {
    let mut seen = 0;
    for (name, text, format) in jobs() {
        let parsed = Job::parse_as(&text, format).unwrap();
        if parsed.device().is_some() || parsed.circuit().is_some() {
            continue;
        }
        let kind = parsed.task().get("kind").and_then(|k| k.as_str());
        // a structure job's scene draws every list; a modes or fdfd job's its rectangles, disks
        // and rings, which are all such a job has
        if !matches!(kind, Some("structure" | "modes" | "fdfd")) {
            continue;
        }
        let Event::Scene { shapes, .. } = job::preview(&parsed).unwrap() else {
            panic!("{name}: no scene");
        };
        let d = drawing(&text, format).unwrap();
        let all: Vec<SceneShape> = d.entries.iter().flat_map(|e| e.shapes.clone()).collect();
        assert!(
            same_shapes(&all, &shapes),
            "{name}: the entries draw other shapes than its scene"
        );
        assert!(
            d.entries.iter().all(|e| !e.shapes.is_empty()),
            "{name}: an entry draws nothing"
        );
        assert!(!d.layers.is_empty(), "{name}: no layers");
        seen += 1;
    }
    assert!(seen >= 10, "only {seen} jobs looked at");
}

#[test]
fn a_shape_on_a_line_is_found_where_the_text_has_it() {
    // two rectangles as tables, then a ring, a rectangle and a disk on lines: the library reads
    // the line's rectangle in after the tables' (rect 2), and the text has it at shapes[1]
    let text = "\
name: lines
task:
  kind: structure
  stack: soi_220
  wavelength: 1.55 um
  layer: Si
  x: [-4 um, 4 um]
  y: [-3 um, 3 um]
  step: 20 nm
  rect:
    - { layer: Si, center_um: [0, -2], size_um: [8, 0.5] }
    - { layer: Si, center_um: [0, 2], size_um: [8, 0.5] }
  shapes:
    - ring: { layer: Si, center: [0, 0], radius: 1.2 um, width: 400 nm }
    - rect: { layer: Si, center: [3 um, 0], size: [1 um, 1 um] }
    - circle: { layer: Si, center: [-3 um, 0], radius: 300 nm }
";
    let d = drawing(text, Format::Yaml).unwrap();
    let found: Vec<(String, usize, Vec<Key>)> = d
        .entries
        .iter()
        .map(|e| (e.list.clone(), e.index, e.path.clone()))
        .collect();
    let at = |list: &str, i: usize| place(list, i);
    let line = |j: usize| {
        vec![
            Key::Name("task".into()),
            Key::Name("shapes".into()),
            Key::Index(j),
        ]
    };
    assert_eq!(
        found,
        vec![
            ("rect".into(), 0, at("rect", 0)),
            ("rect".into(), 1, at("rect", 1)),
            ("rect".into(), 2, line(1)),
            ("circle".into(), 0, line(2)),
            ("ring".into(), 0, line(0)),
        ]
    );
    // each its own shape: the line's rectangle is the square at x = 3
    assert!(d.entries.iter().all(|e| e.shapes.len() == 1));
    let square = &d.entries[2].shapes[0].outline;
    assert!(
        square
            .iter()
            .all(|[x, _]| (2.5 - 1e-9..=3.5 + 1e-9).contains(x))
    );
    assert_eq!(d.layers, ["BOX", "Si"]);
}

#[test]
fn a_component_connected_to_another_is_drawn_after_it_alone() {
    // the second straight is placed on the first one's port: drawn one more at a time, each
    // entry has its own straight
    let text = r#"
name = "connected"
[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1, 9]
y_um = [-2, 2]
step_nm = 20

[[task.component]]
type = "straight"
name = "a"
width = "500 nm"
length = "3 um"

[[task.component]]
type = "straight"
name = "b"
width = "500 nm"
length = "2 um"
connect = ["o1", "a.o2"]
"#;
    let d = drawing(text, Format::Toml).unwrap();
    assert_eq!(d.entries.len(), 2);
    let right = |e: &Drawn| {
        e.shapes[0]
            .outline
            .iter()
            .map(|p| p[0])
            .fold(f64::MIN, f64::max)
    };
    assert!((right(&d.entries[0]) - 3.0).abs() < 1e-9);
    assert!((right(&d.entries[1]) - 5.0).abs() < 1e-9);
    assert_eq!(d.entries[1].path, place("component", 1));
}

#[test]
fn a_job_that_doesnt_read_is_an_error_and_a_device_draws_nothing_here() {
    assert!(drawing("task: [", Format::Yaml).is_err());
    let device = job::template("device").unwrap().text;
    assert!(drawing(device, Format::Yaml).unwrap().entries.is_empty());
}
