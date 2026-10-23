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

/// The heater's turn-on in a smaller window: the SOI strip under a TiN heater, Jacques et al.'s
/// heat capacities.
const TURN_ON: &str = r#"
name = "turn-on"
timeout_minutes = 5

[task]
kind = "static"
physics = "heat"
stack = "soi_220"
y_um = [-8.0, 8.0]
z_um = [-4.0, 5.0]
step_nm = 1000.0
fine_nm = 50.0

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [10.0, 0.5]

[task.thermal_conductivity]
Si = 148.0
SiO2 = 1.4
TiN = 28.0

[task.heat_capacity]
Si = 1.657e6
SiO2 = 1.562e6
TiN = 3.134e6

[task.transient]
step_ns = 100.0
duration_us = 30.0

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

fn steps(events: &[Event]) -> Vec<(f64, f64)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::StaticStep { time_us, means } => Some((*time_us, means[0].value)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_transient_records_its_steps_and_times() {
    let ev = events("turn-on", TURN_ON);
    let s = steps(&ev);
    // t = 0 and every one of the 300 steps
    assert_eq!(s.len(), 301);
    assert_eq!(s[0], (0.0, 0.0));
    assert!((s[300].0 - 30.0).abs() < 1e-9, "{:?}", s[300]);
    assert!(s.windows(2).all(|w| w[1].1 > w[0].1), "a turn-on rises");
    assert!(
        ev.iter().any(
            |e| matches!(e, Event::StaticField { label, .. } if label == "temperature at 30 us")
        )
    );
    let (_, _, _, values) = result(&ev);
    let steady = named(&values, "mean over guide");
    assert!(
        s[300].1 < steady && s[300].1 > 0.9 * steady,
        "{} {steady}",
        s[300].1
    );
    let tau = named(&values, "1/e time of guide");
    let rise = named(&values, "10-90 % time of guide");
    let f = named(&values, "-3 dB frequency of guide");
    assert!(tau > 1e-6 && tau < 20e-6, "{tau}");
    // a single pole's 10-90 % time is ln 9 τ, and its -3 dB frequency 1/2πτ: near those
    assert!(
        (rise / tau - 9f64.ln()).abs() < 0.5 * 9f64.ln(),
        "{rise} {tau}"
    );
    let pole = 2.0 * std::f64::consts::PI * f * tau;
    assert!(pole > 0.3 && pole < 3.0, "{pole}");
    assert_eq!(named(&values, "steps"), 300.0);
    assert!((named(&values, "time step") - 1e-7).abs() < 1e-20);
}

#[test]
fn a_turn_off_falls_as_the_turn_on_rises() {
    let on = events("on", TURN_ON);
    let off = events(
        "off",
        &TURN_ON.replace("duration_us = 30.0", "duration_us = 30.0\nturn = \"off\""),
    );
    let steady = named(&result(&on).3, "mean over guide");
    for ((t, a), (u, b)) in steps(&on).into_iter().zip(steps(&off)) {
        assert_eq!(t, u);
        assert!((a + b - steady).abs() < 1e-9 * steady, "{t}: {a} + {b}");
    }
    let (a, b) = (
        named(&result(&on).3, "1/e time of guide"),
        named(&result(&off).3, "1/e time of guide"),
    );
    assert!((a - b).abs() < 1e-9 * a, "{a} {b}");
}

#[test]
fn a_transient_gives_the_same_bits_on_one_thread_and_many() {
    let short = TURN_ON.replace("duration_us = 30.0", "duration_us = 3.0");
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| events(&format!("transient-bits{threads}"), &short))
    };
    let strip = |ev: Vec<Event>| -> Vec<Event> {
        ev.into_iter()
            .filter(|e| !matches!(e, Event::Finished { .. }))
            .collect()
    };
    assert_eq!(strip(run(1)), strip(run(8)));
}

#[test]
fn a_transient_names_what_it_lacks() {
    let refused = |text: String| check(&Job::parse(&text).unwrap()).unwrap_err().to_string();
    let e = refused(TURN_ON.replace("TiN = 3.134e6\n", ""));
    assert!(
        e.contains("[task.heat_capacity] has no value for the medium TiN"),
        "{e}"
    );
    let e = refused(TURN_ON.replace("step_ns = 100.0", "step_ns = 0.0"));
    assert!(e.contains("positive step_ns"), "{e}");
    let e = refused(TURN_ON.replace("step_ns = 100.0", "step_ns = 0.01"));
    assert!(e.contains("more than 100000"), "{e}");
    let e = refused(TURN_ON.replace(
        "duration_us = 30.0",
        "duration_us = 30.0\nscheme = \"euler\"",
    ));
    assert!(e.contains("\"crank-nicolson\" or \"bdf2\""), "{e}");
    let e = refused(TURN_ON.replace("duration_us = 30.0", "duration_us = 30.0\nturn = \"up\""));
    assert!(e.contains("\"on\" or \"off\""), "{e}");
    let e = refused(ELECTRODES.replace(
        "[task.boundary]",
        "[task.transient]\nstep_ns = 1.0\nduration_us = 1.0\n\n[task.boundary]",
    ));
    assert!(e.contains("is for physics = \"heat\""), "{e}");
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
fn a_heat_job_takes_the_catalogue_s_conductivities_for_the_media_it_doesnt_give() {
    let catalogued = HEATER.replace(
        "Si = 148.0
SiO2 = 1.4
",
        "",
    );
    assert_ne!(catalogued, HEATER);
    let (_, _, _, values) = result(&events("catalogue", &catalogued));
    // the 220 nm layer a film, the substrate apart from it in bulk, the oxide fused silica's
    let film = named(
        &values,
        "thermal conductivity of Si (Ju & Goodson 1999, 220 nm film)",
    );
    assert!((film - 90.666).abs() < 1e-3, "{film}");
    let substrate = named(
        &values,
        "thermal conductivity of Si (substrate) (Ho, Powell & Liley 1972)",
    );
    assert!((substrate - 148.0).abs() < 1e-12, "{substrate}");
    let oxide = named(
        &values,
        "thermal conductivity of SiO2 (Powell, Ho & Liley 1966)",
    );
    assert!((oxide - 1.38).abs() < 1e-12, "{oxide}");
    assert!(named(&values, "imbalance") < 1e-12);
    // the guide a little warmer than with the bulk's 148 and 1.4 everywhere: the oxide's 1.38
    let given = named(&result(&events("given", HEATER)).3, "mean over guide");
    let guide = named(&values, "mean over guide");
    assert!(guide > given && guide < 1.03 * given, "{guide} {given}");
    // a job's own value is the one taken, and the run lists only what the catalogue gave
    assert!(
        !result(&events("given-only", HEATER))
            .3
            .iter()
            .any(|v| v.name.starts_with("thermal conductivity"))
    );
}

#[test]
fn what_a_static_job_refuses() {
    // each an edit of HEATER that check refuses, and the words its refusal must hold
    let nothing_holds = HEATER
        .replace(
            "bottom = { fixed = 0.0 }
",
            "",
        )
        .replace(
            "top = { convective = 10.0, ambient = 0.0 }
",
            "",
        );
    for (text, says) in [
        // a medium without a coefficient, which the catalogue hasn't either, is named
        (
            HEATER.replace(
                "TiN = 28.0
",
                "",
            ),
            "no value for the medium TiN",
        ),
        // the catalogue's film of silicon is at 300 K only
        (
            HEATER
                .replace(
                    "Si = 148.0
",
                    "",
                )
                .replace("fine_nm = 20.0", "fine_nm = 20.0\ntemperature_k = 350.0"),
            "at temperature 300 K only",
        ),
        // an unknown physics, and an unknown side condition
        (
            HEATER.replace("physics = \"heat\"", "physics = \"magnetic\""),
            "\"heat\", \"electrostatic\" or \"current\"",
        ),
        (
            HEATER.replace("bottom = { fixed = 0.0 }", "bottom = \"cold\""),
            "insulated",
        ),
        // a grid too fine
        (
            HEATER.replace(
                "fine_nm = 20.0",
                "fine_nm = 1.0
growth = 1.0",
            ),
            "cells",
        ),
        // nothing holding the level
        (nothing_holds, "nothing holds"),
    ] {
        assert_ne!(text, HEATER, "{says}");
        let job = Job::parse(&text).unwrap();
        let e = check(&job).unwrap_err().to_string();
        assert!(e.contains(says), "{says}: {e}");
    }
}
