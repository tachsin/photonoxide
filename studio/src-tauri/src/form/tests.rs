use std::path::Path;

use photonoxide::job;
use photonoxide::run::{Format, Job};
use toml::Value;

use super::*;

/// Whether two values are the same, a number the same whether written as an integer or a float.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Integer(x), Value::Float(y)) | (Value::Float(y), Value::Integer(x)) => {
            *x as f64 == *y
        }
        (Value::Float(x), Value::Float(y)) => x.to_bits() == y.to_bits(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Table(x), Value::Table(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

/// The repository's job files, by name.
fn jobs() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jobs");
    let mut all: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap().replace("\r\n", "\n"),
            )
        })
        .collect();
    all.sort();
    all
}

/// The job `text` is, and the job its form writes back, the same: name, timeout, task and every
/// table, each number the same.
fn round_trip(name: &str, text: &str) -> Form {
    let form = Form::of(text, Format::Toml).unwrap_or_else(|e| panic!("{name}: {e}"));
    let back = form.text().unwrap();
    let (a, b): (toml::Table, toml::Table) = (
        toml::from_str(text).unwrap(),
        toml::from_str(&back).unwrap(),
    );
    for (k, v) in &a {
        assert!(
            b.get(k).is_some_and(|w| same(v, w)),
            "{name}: {k} differs:\n{v:?}\n{:?}\n{back}",
            b.get(k)
        );
    }
    assert_eq!(a.len(), b.len(), "{name}: {back}");
    let job = Job::parse(&back).unwrap();
    job::check(&job).unwrap_or_else(|e| panic!("{name}: {e}\n{back}"));
    // the same text back again: the form is a fixed point of its own text
    let again = Form::of(&back, Format::Toml).unwrap();
    assert_eq!(again.text().unwrap(), back, "{name}");
    form
}

#[test]
fn every_job_in_jobs_comes_back_through_the_form_as_the_same_job() {
    let all = jobs();
    assert!(all.len() >= 12);
    for (name, text) in &all {
        let job = Job::parse(text).unwrap();
        if job.device().is_some() {
            // a device's job opens on its device's page
            assert!(Form::of(text, Format::Toml).is_err(), "{name}");
            crate::devices::read_text(text).unwrap();
            continue;
        }
        round_trip(name, text);
    }
}

#[test]
fn a_form_edit_keeps_the_expressions_and_the_parameters_sweep() {
    let text = jobs()
        .into_iter()
        .find(|(n, _)| n == "coupler-supermodes.toml")
        .unwrap()
        .1;
    let mut form = round_trip("coupler-supermodes", &text);
    assert_eq!(
        form.parameters,
        [
            ("gap".to_owned(), Num::Text("200 nm".into())),
            ("w".to_owned(), Num::Text("500 nm".into()))
        ]
    );
    let sweep = form.sweep.clone().unwrap();
    assert_eq!(sweep.parameter, "gap");
    assert_eq!(
        (sweep.from, sweep.to, sweep.points),
        (Num::Text("100 nm".into()), Num::Text("400 nm".into()), 16)
    );
    // the rectangles' places are expressions: kept as the file has them, and said so
    assert_eq!(form.kept, ["task.rect"]);
    // an edit in the form: a finer grid and a wider pair
    form.step_nm = 5.0;
    form.parameters[1].1 = Num::Text("450 nm".into());
    let edited = form.text().unwrap();
    let job = Job::parse(&edited).unwrap();
    job::check(&job).unwrap();
    let task = job.task();
    assert_eq!(task["step_nm"].as_float(), Some(5.0));
    assert_eq!(task["parameters"]["w"].as_str(), Some("450 nm"));
    assert_eq!(task["parameters"]["gap"].as_str(), Some("200 nm"));
    assert_eq!(task["sweep"]["parameter"].as_str(), Some("gap"));
    assert_eq!(task["sweep"]["from"].as_str(), Some("100 nm"));
    assert_eq!(task["sweep"]["to"].as_str(), Some("400 nm"));
    let rects = task["rect"].as_array().unwrap();
    assert_eq!(rects.len(), 2);
    assert_eq!(
        rects[1]["center"].as_array().unwrap()[1].as_str(),
        Some("(gap + w)/2")
    );
    assert_eq!(rects[0]["size"].as_array().unwrap()[1].as_str(), Some("w"));
}

#[test]
fn what_the_form_cant_hold_is_kept_and_named() {
    let text = r#"# A ring by its generator, its port found by name.
name = "ring-ports"

[solver]
direct = "photonoxide"

[task]
kind = "fdfd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
polarization = "te"
x_um = [-8.0, 8.0]
y_um = [-3.0, 9.0]
step_nm = 50.0
averaging = "mean"

[task.parameters]
r = "5 um"

[[task.component]]
type = "ring"
name = "ring"
width = "500 nm"
radius = "r"
gap = "200 nm"

[[task.port]]
port = "ring.in"

[[task.port]]
port = "ring.through"
"#;
    let form = Form::of(text, Format::Toml).unwrap();
    assert_eq!(
        form.kept,
        ["solver", "task.averaging", "task.component", "task.port"]
    );
    assert_eq!(
        form.about,
        "A ring by its generator, its port found by name."
    );
    let back = form.text().unwrap();
    assert!(back.starts_with("# A ring by its generator"), "{back}");
    let (a, b) = (Job::parse(text).unwrap(), Job::parse(&back).unwrap());
    assert!(
        same(
            &Value::Table(a.task().clone()),
            &Value::Table(b.task().clone())
        ),
        "{back}"
    );
    assert_eq!(a.direct(), b.direct());
    // a JSON job, read as the same job
    let json = a.to_text(Format::Json).unwrap();
    let from_json = Form::of(&json, Format::Json).unwrap();
    assert_eq!(from_json.kept, form.kept);
    assert_eq!(from_json.about, "");
    // a device's job isn't a form's
    assert!(
        Form::of(
            "name = \"d\"\n\n[device]\nkind = \"waveguide\"\n",
            Format::Toml
        )
        .is_err()
    );
}
