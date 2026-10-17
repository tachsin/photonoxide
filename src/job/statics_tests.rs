//! The `"static"` job ([`super::statics`]): what [`super::check`] refuses, what a run records,
//! its replay and the same bits on any number of threads.

use super::*;
use crate::run::replay;

/// A run directory under the system's temporary one, removed when dropped.
struct TempDir(std::path::PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn temp(tag: &str) -> TempDir {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let n = COUNT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "photonoxide-static-{tag}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    TempDir(dir)
}

/// The module docs' heater over the SOI strip.
const HEATER: &str = r#"
name = "heater"
timeout_minutes = 5

[task]
kind = "static"
physics = "heat"
stack = "soi_220"
y_um = [-20.0, 20.0]
z_um = [-10.0, 6.0]
step_nm = 1000.0
fine_nm = 20.0

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [10.0, 0.5]

[task.thermal_conductivity]
Si = 148.0
SiO2 = 1.4
TiN = 28.0

[task.boundary]
bottom = { fixed = 0.0 }
top = { convective = 10.0, ambient = 0.0 }

[[task.region]]
y_um = [-1.0, 1.0]
z_um = [4.22, 4.34]
medium = "TiN"

[[task.heater]]
y_um = [-1.0, 1.0]
z_um = [4.22, 4.34]
power_w_per_m = 10.0

[[task.probe]]
name = "guide"
y_um = [-0.25, 0.25]
z_um = [2.0, 2.22]
"#;

/// Two electrodes beside a strip of lithium niobate's x-cut permittivity, in oxide.
const ELECTRODES: &str = r#"
name = "electrodes"

[task]
kind = "static"
physics = "electrostatic"
stack = "soi_220"
y_um = [-30.0, 30.0]
z_um = [-10.0, 20.0]
step_nm = 2000.0
fine_nm = 25.0

[task.permittivity]
Si = 11.7
SiO2 = 3.9
LN = [28.0, 44.0]

[task.boundary]
left = { fixed = 0.0 }
right = { fixed = 0.0 }
bottom = { fixed = 0.0 }
top = { fixed = 0.0 }

[[task.region]]
y_um = [-1.0, 1.0]
z_um = [2.22, 2.82]
medium = "LN"

[[task.conductor]]
y_um = [-12.0, -2.5]
z_um = [2.22, 3.02]
potential_v = 0.0

[[task.conductor]]
y_um = [2.5, 12.0]
z_um = [2.22, 3.02]
potential_v = 1.0

[[task.probe]]
name = "gap"
y_um = [-1.0, 1.0]
z_um = [2.22, 2.82]
"#;

/// A doped bar between two contacts, its Joule heat conducted to the substrate.
const JOULE: &str = r#"
name = "joule"

[task]
kind = "static"
physics = "current"
stack = "soi_220"
y_um = [-10.0, 10.0]
z_um = [-5.0, 5.0]
step_nm = 500.0
fine_nm = 20.0

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [10.0, 4.0]

[task.electrical_conductivity]
Si = 1e4
SiO2 = 1e-12

[task.thermal_conductivity]
Si = 148.0
SiO2 = 1.4

[task.boundary]
bottom = { fixed = 0.0 }

[[task.conductor]]
y_um = [-2.0, -1.5]
z_um = [2.0, 2.22]
potential_v = 0.0

[[task.conductor]]
y_um = [1.5, 2.0]
z_um = [2.0, 2.22]
potential_v = 1.0
"#;

fn events(tag: &str, text: &str) -> Vec<Event> {
    let root = temp(tag);
    let job = Job::parse(text).unwrap();
    check(&job).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    replay(run.dir()).unwrap()
}

fn result(events: &[Event]) -> (String, [usize; 2], [f64; 2], Vec<StaticValue>) {
    events
        .iter()
        .find_map(|e| match e {
            Event::StaticResult {
                physics,
                cells,
                step_um,
                values,
            } => Some((physics.clone(), *cells, *step_um, values.clone())),
            _ => None,
        })
        .expect("a result")
}

fn named(values: &[StaticValue], name: &str) -> f64 {
    values
        .iter()
        .find(|v| v.name == name)
        .unwrap_or_else(|| panic!("no {name} in {values:?}"))
        .value
}

#[test]
fn a_heat_job_records_its_temperature_and_numbers() {
    let ev = events("heat", HEATER);
    assert!(matches!(&ev[0], Event::Started { kind, .. } if kind == "static"));
    assert!(matches!(&ev[1], Event::Scene { .. }));
    assert!(
        matches!(&ev[2], Event::StaticField { label, unit, raster, .. } if label == "temperature" && unit == "K" && raster.nx == 400)
    );
    let (physics, cells, step, values) = result(&ev);
    assert_eq!(physics, "heat");
    assert!(cells[0] > 40 && cells[1] > 40, "{cells:?}");
    assert!(
        (step[0] - 0.02).abs() < 0.011 && step[1] <= 1.0 + 1e-12,
        "{step:?}"
    );
    assert!(named(&values, "imbalance") < 1e-12);
    let guide = named(&values, "mean over guide");
    assert!(guide > 0.0 && guide < named(&values, "largest"), "{guide}");
    // the heater's 10 W/m leave through the substrate and the convective top
    let out = named(&values, "out through bottom") + named(&values, "out through top");
    assert!((out - 10.0).abs() < 1e-9, "{out}");
    assert!(matches!(
        ev.last(),
        Some(Event::Finished { stopped: None, .. })
    ));
}

#[test]
fn a_heat_job_gives_the_same_bits_on_one_thread_and_many() {
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| events(&format!("bits{threads}"), HEATER))
    };
    let strip = |ev: Vec<Event>| -> Vec<Event> {
        ev.into_iter()
            .filter(|e| !matches!(e, Event::Finished { .. }))
            .collect()
    };
    assert_eq!(strip(run(1)), strip(run(8)));
}

#[test]
fn an_electrostatic_job_records_its_capacitance() {
    let ev = events("electro", ELECTRODES);
    let (physics, _, _, values) = result(&ev);
    assert_eq!(physics, "electrostatic");
    let (c11, c12, c22) = (
        named(&values, "C[1][1]"),
        named(&values, "C[1][2]"),
        named(&values, "C[2][2]"),
    );
    assert!(c11 > 0.0 && c22 > 0.0 && c12 < 0.0, "{c11} {c12} {c22}");
    assert!(
        (c12 - named(&values, "C[2][1]")).abs() <= 1e-9 * c11,
        "symmetric"
    );
    // the probe between the electrodes at 0 and 1 V
    let gap = named(&values, "mean over gap");
    assert!(gap > 0.3 && gap < 0.7, "{gap}");
}

#[test]
fn a_current_job_heats_with_its_joule_power() {
    let ev = events("joule", JOULE);
    let (physics, _, _, values) = result(&ev);
    assert_eq!(physics, "current");
    let power = named(&values, "Joule power");
    // 1 V across 3 µm of 0.22 µm silicon at 1e4 S/m: σ t / L
    let exact = 1e4 * 0.22 / 3.0;
    assert!((power - exact).abs() < 1e-6 * exact, "{power} {exact}");
    let heat_out = named(&values, "heat: out through bottom");
    assert!(
        (heat_out - power).abs() < 1e-9 * power,
        "{heat_out} {power}"
    );
    assert!(
        ev.iter()
            .filter(|e| matches!(e, Event::StaticField { .. }))
            .count()
            == 2
    );
}

#[test]
fn check_names_a_medium_without_a_coefficient() {
    let job = Job::parse(&HEATER.replace("SiO2 = 1.4\n", "")).unwrap();
    let e = check(&job).unwrap_err().to_string();
    assert!(e.contains("no value for the medium SiO2"), "{e}");
}

#[test]
fn check_refuses_an_unknown_physics_and_side() {
    let job = Job::parse(&HEATER.replace("physics = \"heat\"", "physics = \"magnetic\"")).unwrap();
    let e = check(&job).unwrap_err().to_string();
    assert!(
        e.contains("\"heat\", \"electrostatic\" or \"current\""),
        "{e}"
    );
    let job = Job::parse(&HEATER.replace("bottom = { fixed = 0.0 }", "bottom = \"cold\"")).unwrap();
    let e = check(&job).unwrap_err().to_string();
    assert!(e.contains("insulated"), "{e}");
}

#[test]
fn check_refuses_a_grid_too_fine() {
    let job = Job::parse(&HEATER.replace(
        "fine_nm = 20.0",
        "fine_nm = 1.0
growth = 1.0",
    ))
    .unwrap();
    let e = check(&job).unwrap_err().to_string();
    assert!(e.contains("cells"), "{e}");
}

#[test]
fn check_refuses_a_job_that_holds_nothing() {
    let text = HEATER
        .replace("bottom = { fixed = 0.0 }\n", "")
        .replace("top = { convective = 10.0, ambient = 0.0 }\n", "");
    let job = Job::parse(&text).unwrap();
    let e = check(&job).unwrap_err().to_string();
    assert!(e.contains("nothing holds"), "{e}");
}
