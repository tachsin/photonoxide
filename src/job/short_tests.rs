//! The shorter forms of a job file ([`super::short`]): units in values, shapes one to a line,
//! YAML's anchors, each the same job as the keys it stands for.

use std::path::Path;

use toml::Value;

use super::schema::tests_support::takes;
use crate::run::{Format, Job};

/// The built-in jobs, each with its file's name.
fn built_in() -> Vec<(String, Job)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if Format::from_path(&path).is_some() {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            out.push((name, Job::load(&path).unwrap()));
        }
    }
    assert!(out.len() >= 20, "{} jobs", out.len());
    out
}

/// What a run reads of a job: not its text, nor its format.
fn content(
    job: &Job,
) -> (
    String,
    Option<toml::Table>,
    Option<toml::Table>,
    toml::Table,
) {
    (
        job.name().to_owned(),
        job.device().cloned(),
        job.circuit().cloned(),
        job.task().clone(),
    )
}

/// A YAML job's text as the JSON the schema checks, as it is written: no form turned into keys.
fn raw(yaml: &str) -> serde_json::Value {
    serde_saphyr::from_str(yaml).unwrap()
}

#[test]
fn every_built_in_job_is_the_same_job_in_the_shorter_forms() {
    for (name, job) in built_in() {
        let yaml = job.to_text(Format::Yaml).unwrap();
        let again =
            Job::parse_as(&yaml, Format::Yaml).unwrap_or_else(|e| panic!("{name}: {e}\n{yaml}"));
        assert_eq!(content(&again), content(&job), "{name}\n{yaml}");
        crate::job::check(&again).unwrap_or_else(|e| panic!("{name}: {e}"));
        // the YAML is in the shorter forms, and the schema takes them as they are written
        let task = job.task();
        if task.contains_key("step_nm") {
            assert!(yaml.contains("  step: "), "{name}\n{yaml}");
            assert!(!yaml.contains("step_nm"), "{name}\n{yaml}");
        }
        if task.contains_key("wavelength_um") {
            assert!(yaml.contains("  wavelength: "), "{name}\n{yaml}");
        }
        if task.contains_key("rect") {
            assert!(yaml.contains("  - rect: {"), "{name}\n{yaml}");
            assert!(!yaml.contains("  rect:"), "{name}\n{yaml}");
        }
        takes(&raw(&yaml)).unwrap_or_else(|e| panic!("{name}: {e}\n{yaml}"));
    }
}

/// The built-in jobs are YAML (#430), each the job its TOML was, to the bit: `tests/jobs-toml`
/// holds the files as they were, and TOML is still read. The schema takes each file as written.
/// A job added since (the examples' jobs) was never TOML, and has no copy there.
#[test]
fn every_built_in_job_is_its_old_toml_to_the_bit() {
    let old = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/jobs-toml");
    let jobs = built_in();
    let mut copies = 0;
    for (name, job) in &jobs {
        let stem = name
            .strip_suffix(".yaml")
            .unwrap_or_else(|| panic!("{name}: the built-in jobs are YAML"));
        // as git keeps it: a checkout with CRLF would put them in a multi-line string
        let Ok(toml) = std::fs::read_to_string(old.join(format!("{stem}.toml"))) else {
            continue;
        };
        copies += 1;
        let toml = Job::parse(&toml.replace("\r\n", "\n")).unwrap();
        assert_eq!(content(job), content(&toml), "{name}");
        // floats by their shortest round trip: the same text is the same bits
        assert_eq!(
            format!("{:?}", content(job)),
            format!("{:?}", content(&toml)),
            "{name}"
        );
        assert_eq!(
            job.to_text(Format::Json).unwrap(),
            toml.to_text(Format::Json).unwrap(),
            "{name}"
        );
        let text = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("jobs")
                .join(name),
        )
        .unwrap();
        takes(&raw(&text)).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    // every old copy is still a built-in job's
    assert_eq!(copies, std::fs::read_dir(&old).unwrap().count());
}

/// A modes job of a 500 nm strip, with `task` in its task (two spaces in) and `shapes` as its
/// shapes.
fn modes(task: &str, shapes: &str) -> String {
    format!(
        "name: m\ntask:\n  kind: modes\n  stack: soi_220\n  layer: Si\n  propagation: x\n{task}\n{shapes}\n"
    )
}

const WINDOW: &str = "  wavelength: 1.55 um\n  y: [-1.2 um, 1.2 um]\n  step: 20 nm";
const STRIP: &str =
    "  shapes:\n    - rect: { layer: Si, center_um: [0.0, 0.0], size_um: [10.0, 0.5] }";
const KEYS: &str = "name = \"m\"\n\n[task]\nkind = \"modes\"\nstack = \"soi_220\"\nlayer = \"Si\"\n\
    propagation = \"x\"\nwavelength_um = 1.55\ny_um = [-1.2, 1.2]\nstep_nm = 20.0\n\n[[task.rect]]\n\
    layer = \"Si\"\ncenter_um = [0.0, 0.0]\nsize_um = [10.0, 0.5]\n";

#[test]
fn units_in_values_are_their_keys_to_the_bit() {
    let keys = Job::parse(KEYS).unwrap();
    let short = Job::parse_as(&modes(WINDOW, STRIP), Format::Yaml).unwrap();
    assert_eq!(content(&short), content(&keys));
    crate::job::check(&short).unwrap();
    // other units: converted, and exact where the conversion is
    for (window, wavelength, y, step) in [
        (
            "  wavelength: 1550 nm\n  y: [-1200 nm, 1.2 um]\n  step: 0.02 um",
            1.55,
            [-1.2, 1.2],
            20.0,
        ),
        (
            "  wavelength: 1.55um\n  y: [\"-1.2 um\", 1.2 um]\n  step: 2e1 nm",
            1.55,
            [-1.2, 1.2],
            20.0,
        ),
        (
            "  wavelength: 0.775 um * 2\n  y: [-1.2 um, 1.2 um]\n  step: 10 nm + 10 nm",
            1.55,
            [-1.2, 1.2],
            20.0,
        ),
    ] {
        let job = Job::parse_as(&modes(window, STRIP), Format::Yaml).unwrap();
        let t = job.task();
        assert_eq!(t["wavelength_um"].as_float(), Some(wavelength), "{window}");
        let got: Vec<f64> = t["y_um"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_float().unwrap())
            .collect();
        assert_eq!(got, y, "{window}");
        assert_eq!(t["step_nm"].as_float(), Some(step), "{window}");
        assert!(!t.contains_key("wavelength") && !t.contains_key("step"));
    }
    // an angle, in a source's own unit: degrees
    let text =
        "name: a\ntask:\n  kind: fdtd\n  source:\n    - type: gaussian\n      angle: 0.5 rad\n";
    let job = Job::parse_as(text, Format::Yaml).unwrap();
    let angle = job.task()["source"][0]["angle_deg"].as_float().unwrap();
    assert_eq!(angle, 0.5_f64.to_degrees());
}

#[test]
fn units_in_values_are_read_in_toml_and_json_too() {
    let keys = Job::parse(KEYS).unwrap();
    let toml = "name = \"m\"\n\n[task]\nkind = \"modes\"\nstack = \"soi_220\"\nlayer = \"Si\"\n\
        propagation = \"x\"\nwavelength = \"1.55 um\"\ny = [\"-1.2 um\", \"1.2 um\"]\n\
        step = \"20 nm\"\nshapes = [{ rect = { layer = \"Si\", center_um = [0.0, 0.0], \
        size_um = [10.0, 0.5] } }]\n";
    let json = r#"{"name": "m", "task": {"kind": "modes", "stack": "soi_220", "layer": "Si",
        "propagation": "x", "wavelength": "1.55 um", "y": ["-1.2 um", "1.2 um"],
        "step": "20 nm", "shapes": [{"rect": {"layer": "Si", "center_um": [0.0, 0.0],
        "size_um": [10.0, 0.5]}}]}}"#;
    for (text, format) in [(toml, Format::Toml), (json, Format::Json)] {
        let job = Job::parse_as(text, format).unwrap_or_else(|e| panic!("{format:?}: {e}"));
        assert_eq!(content(&job), content(&keys), "{format:?}");
    }
}

#[test]
fn a_value_without_its_unit_or_with_both_keys_is_refused() {
    for (window, says) in [
        (
            "  wavelength: 1.55\n  y: [-1.2 um, 1.2 um]\n  step: 20 nm",
            "task.wavelength: needs a length, got a number",
        ),
        (
            "  wavelength: 1.55 um\n  y: [-1.2, 1.2 um]\n  step: 20 nm",
            "task.y: needs a length, got a number",
        ),
        (
            "  wavelength: 1.55 um\n  wavelength_um: 1.55\n  y: [-1.2 um, 1.2 um]\n  step: 20 nm",
            "task.wavelength: give wavelength or wavelength_um, not both",
        ),
        (
            "  wavelength: 90 deg\n  y: [-1.2 um, 1.2 um]\n  step: 20 nm",
            "task.wavelength: needs a length, got an angle",
        ),
        (
            "  wavelength: lambda0\n  y: [-1.2 um, 1.2 um]\n  step: 20 nm",
            "task.wavelength: ",
        ),
    ] {
        let e = Job::parse_as(&modes(window, STRIP), Format::Yaml)
            .unwrap_err()
            .to_string();
        assert!(e.contains(says), "{window}: {e}");
    }
    // a zero needs no unit
    let job = Job::parse_as(
        &modes(
            "  wavelength: 1.55 um\n  y: [0, 1.2 um]\n  step: 20 nm",
            STRIP,
        ),
        Format::Yaml,
    )
    .unwrap();
    assert_eq!(job.task()["y_um"][0].as_float(), Some(0.0));
}

#[test]
fn a_bare_key_the_table_has_is_left_to_it() {
    // a sweep's from and to are its own (in the swept parameter's unit), not from_um and to_um
    let text = format!(
        "{}  sweep: {{ parameter: wavelength, from: 1.5, to: 1.6, points: 3 }}\n",
        modes(WINDOW, STRIP)
    );
    let job = Job::parse_as(&text, Format::Yaml).unwrap();
    let sweep = job.task()["sweep"].as_table().unwrap();
    assert_eq!(sweep["from"].as_float(), Some(1.5));
    assert!(!sweep.contains_key("from_um"));
    crate::job::check(&job).unwrap();
    // a rectangle's center is an expression the resolver reads, with the task's parameters
    let text = modes(
        &format!("{WINDOW}\n  parameters: {{ w: 500 nm }}"),
        "  shapes:\n    - rect: { layer: Si, center: [0, 0], size: [10 um, w] }",
    );
    let job = Job::parse_as(&text, Format::Yaml).unwrap();
    let rect = job.task()["rect"][0].as_table().unwrap();
    assert_eq!(rect["size"][1].as_str(), Some("w"));
    crate::job::check(&job).unwrap();
}

/// A structure job over a 8 × 4 µm window, with `shapes` as its shapes.
fn structure(shapes: &str) -> String {
    format!(
        "name: s\ntask:\n  kind: structure\n  stack: soi_220\n  layer: Si\n  wavelength: 1.55 um\n  \
         x: [-4 um, 4 um]\n  y: [-2 um, 2 um]\n  step: 20 nm\n{shapes}\n"
    )
}

#[test]
fn shapes_one_to_a_line_are_their_tables() {
    let lines = "  shapes:\n\
        \x20   - rect: { layer: Si, center_um: [0, 0], size_um: [10, 0.5] }\n\
        \x20   - circle: { layer: Si, center: [2 um, 0], radius: 100 nm }\n\
        \x20   - ring: { layer: Si, center_um: [-2, 0], radius_um: 0.4, width_um: 0.1 }\n\
        \x20   - polygon: { layer: Si, points_um: [[3, 0], [3.2, 0], [3.1, 0.2]] }\n\
        \x20   - rect: { layer: Si, center_um: [0, 0.5], size_um: [1, 0.1] }";
    let job = Job::parse_as(&structure(lines), Format::Yaml).unwrap();
    let t = job.task();
    assert!(!t.contains_key("shapes"));
    let len = |k: &str| t[k].as_array().unwrap().len();
    assert_eq!(
        (len("rect"), len("circle"), len("ring"), len("shape")),
        (2, 1, 1, 1)
    );
    // each kind's lines in their order
    assert_eq!(t["rect"][1]["center_um"][1].as_float(), Some(0.5));
    assert_eq!(t["shape"][0]["type"].as_str(), Some("polygon"));
    crate::job::check(&job).unwrap();
    // the same structure as the tables
    let tables = "  rect:\n    - { layer: Si, center_um: [0, 0], size_um: [10, 0.5] }\n\
        \x20   - { layer: Si, center_um: [0, 0.5], size_um: [1, 0.1] }\n\
        \x20 circle: [{ layer: Si, center: [2 um, 0], radius: 100 nm }]\n\
        \x20 ring: [{ layer: Si, center_um: [-2, 0], radius_um: 0.4, width_um: 0.1 }]\n\
        \x20 shape: [{ type: polygon, layer: Si, points_um: [[3, 0], [3.2, 0], [3.1, 0.2]] }]";
    let same = Job::parse_as(&structure(tables), Format::Yaml).unwrap();
    assert_eq!(content(&same), content(&job));
    // and the schema takes the lines as they are written
    takes(&raw(&structure(lines))).unwrap();
}

#[test]
fn a_cells_shapes_may_be_lines_too() {
    let text = structure(&format!(
        "{STRIP}\n  cell:\n    - name: dot\n      shapes:\n        - circle: {{ \
         center_um: [0, 0], radius_um: 0.05 }}\n  shape:\n    - {{ type: reference, layer: Si, \
         cell: dot, place: [{{ translate_um: [1, 0.5] }}] }}"
    ));
    let job = Job::parse_as(&text, Format::Yaml).unwrap();
    let shape = &job.task()["cell"][0]["shapes"][0];
    assert_eq!(shape["type"].as_str(), Some("circle"));
    crate::job::check(&job).unwrap();
    takes(&raw(&text)).unwrap();
}

#[test]
fn a_line_that_isnt_one_shape_is_refused() {
    for (shapes, says) in [
        (
            "  shapes:\n    - { rect: { size_um: [1, 1] }, ring: { radius_um: 1 } }",
            "task.shapes[1]: needs one shape",
        ),
        ("  shapes:\n    - rect", "task.shapes[1]: needs one shape"),
        (
            "  shapes:\n    - rect: 3",
            "task.shapes[1]: rect: needs its fields",
        ),
        (
            "  shapes:\n    - polygon: { type: polygon }",
            "its type is the key",
        ),
        ("  shapes: 3", "task.shapes: needs a list"),
    ] {
        let e = Job::parse_as(&modes(WINDOW, shapes), Format::Yaml)
            .unwrap_err()
            .to_string();
        assert!(e.contains(says), "{shapes}: {e}");
    }
}

#[test]
fn yaml_anchors_and_merge_keys_reuse_a_table() {
    // the same rectangle's fields, and a window, given once
    let text = "name: m\n\
        task:\n\
        \x20 kind: modes\n\
        \x20 stack: soi_220\n\
        \x20 layer: Si\n\
        \x20 propagation: x\n\
        \x20 <<: &window { wavelength: 1.55 um, y: [-1.2 um, 1.2 um], step: 20 nm }\n\
        \x20 shapes:\n\
        \x20   - rect: &guide { layer: Si, center_um: [0, 0], size_um: [10, 0.5] }\n\
        \x20   - rect: { <<: *guide, center_um: [0, 0.5], size_um: [1.0, 0.1] }\n";
    let job = Job::parse_as(text, Format::Yaml).unwrap();
    let t = job.task();
    assert_eq!(t["step_nm"].as_float(), Some(20.0));
    let rects = t["rect"].as_array().unwrap();
    assert_eq!(rects[1]["layer"].as_str(), Some("Si"));
    assert_eq!(rects[1]["size_um"][0].as_float(), Some(1.0));
    crate::job::check(&job).unwrap();
}

#[test]
fn the_schema_takes_the_shorter_forms_and_refuses_both_keys() {
    takes(&raw(&modes(WINDOW, STRIP))).unwrap();
    let both = modes(&format!("{WINDOW}\n  wavelength_um: 1.55"), STRIP);
    assert!(Job::parse_as(&both, Format::Yaml).is_err());
    assert!(takes(&raw(&both)).is_err(), "the schema takes both keys");
    // a step's bare form says its unit
    let s = super::schema::schema();
    let step = s["$defs"]["ModesTask"]["properties"]["step"]["description"]
        .as_str()
        .unwrap();
    assert!(step.contains("in place of step_nm"), "{step}");
    // and a key the structs read is never a bare form's: the forms come from the keys
    assert!(
        super::schema::base()["$defs"]["ModesTask"]["properties"]
            .get("step")
            .is_none()
    );
}

#[test]
fn a_float_with_its_unit_reads_back_to_the_bit() {
    for x in [
        1.55,
        0.1,
        1.0 / 3.0,
        -2.5e-300,
        5e-324,
        1e21,
        20.0,
        -0.0,
        123_456_789.123_456_78,
    ] {
        let mut task = toml::Table::new();
        task.insert("wavelength_um".into(), Value::Float(x));
        task.insert("step_nm".into(), Value::Float(x));
        task.insert("kind".into(), Value::String("modes".into()));
        let mut file = toml::Table::new();
        file.insert("task".into(), Value::Table(task.clone()));
        let mut back = super::short::write(&file);
        assert!(back["task"].get("step").is_some(), "{x:e}");
        super::short::read(&mut back).unwrap();
        for key in ["wavelength_um", "step_nm"] {
            let got = back["task"][key].as_float().unwrap();
            assert_eq!(got.to_bits(), x.to_bits(), "{key} {x:e}");
        }
    }
}
