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
fn a_transient_s_frames_are_its_temperature_on_the_way_and_its_times_mark_the_rise() {
    let framed = TURN_ON.replace("duration_us = 30.0", "duration_us = 30.0\nframes = 6");
    let ev = events("frames", &framed);
    let frames: Vec<(f64, &crate::raster::Raster)> = ev
        .iter()
        .filter_map(|e| match e {
            Event::StaticFrame {
                time_us,
                unit,
                axes,
                raster,
            } => {
                assert_eq!(
                    (unit.as_str(), axes),
                    ("K", &["y".to_owned(), "z".to_owned()])
                );
                Some((*time_us, raster))
            }
            _ => None,
        })
        .collect();
    // the start and every 50 of the 300 steps of 0.1 us
    let times: Vec<f64> = frames.iter().map(|f| f.0).collect();
    assert_eq!(times.len(), 7);
    for (k, t) in times.iter().enumerate() {
        assert!((t - 5.0 * k as f64).abs() < 1e-9, "{times:?}");
    }
    // the last is the last field, to the bit, on the steady field's pixels; the start is the
    // ambient, and the guide's probe warms frame by frame
    let last = ev
        .iter()
        .find_map(|e| match e {
            Event::StaticField { label, raster, .. } if label == "temperature at 30 us" => {
                Some(raster)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(frames[6].1, last);
    assert!(frames[0].1.values.iter().all(|v| *v == 0.0));
    let peak = |r: &crate::raster::Raster| r.values.iter().copied().fold(f32::MIN, f32::max);
    assert!(frames.windows(2).all(|w| peak(w[1].1) > peak(w[0].1)));
    // the times the studio marks: 10 % and 90 %, whose difference is the 10-90 % time, and the
    // probe's mean crossing them there
    let values = result(&ev).3;
    let (a, b) = (
        named(&values, "10 % time of guide"),
        named(&values, "90 % time of guide"),
    );
    assert_eq!(b - a, named(&values, "10-90 % time of guide"));
    let steady = named(&values, "mean over guide");
    let s = steps(&ev);
    let at = |t: f64| {
        let k = s.iter().position(|p| p.0 >= t).unwrap();
        let (p, q) = (s[k - 1], s[k]);
        p.1 + (t - p.0) / (q.0 - p.0) * (q.1 - p.1)
    };
    // the record's own end is 30 us, short of the steady state: its share there
    assert!((at(a * 1e6) / steady - 0.1).abs() < 1e-9, "{}", at(a * 1e6));
    assert!((at(b * 1e6) / steady - 0.9).abs() < 1e-9, "{}", at(b * 1e6));
    // without frames none are recorded; too many are refused
    assert!(
        !events("no-frames", TURN_ON)
            .iter()
            .any(|e| matches!(e, Event::StaticFrame { .. }))
    );
    let many = TURN_ON.replace("duration_us = 30.0", "duration_us = 30.0\nframes = 201");
    let e = check(&Job::parse(&many).unwrap()).unwrap_err().to_string();
    assert!(e.contains("1 to 200 pictures"), "{e}");
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

/// `[task.export]` writes the temperature as `static.npz`, the solver's own values on its graded
/// grid: their largest is the run's own "largest" to the bit, and `static.csv` holds the run's
/// numbers to the bit.
#[test]
fn a_static_export_writes_the_temperature_and_the_numbers_to_the_bit() {
    let dir = temp("export");
    let job = Job::parse(&format!("{HEATER}\n[task.export]\nformat = \"both\"\n")).unwrap();
    let mut run = Run::create(&dir.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let ev: Vec<Event> = replay(run.dir()).unwrap();
    let (_, _, _, values) = result(&ev);
    let (arrays, meta) = crate::export::read::npz(&run.dir().join("static.npz"));
    assert!(meta.contains("graded"), "{meta}");
    assert!(!arrays.contains_key("potential"));
    crate::export::assert_estimates(&run.dir().join("static.npz"));
    let (y, z) = (arrays["y_um"].f64s(), arrays["z_um"].f64s());
    assert_eq!(arrays["y_nodes_um"].f64s().len(), y.len() + 1);
    assert_eq!(arrays["z_nodes_um"].f64s().len(), z.len() + 1);
    let t = arrays["temperature"].f64s();
    assert_eq!(arrays["temperature"].shape, [z.len(), y.len()]);
    let largest = t.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let recorded = values.iter().find(|v| v.name == "largest").unwrap().value;
    assert_eq!(largest.to_bits(), recorded.to_bits());

    let csv = std::fs::read_to_string(run.dir().join("static.csv")).unwrap();
    let rows: Vec<Vec<&str>> = csv
        .lines()
        .skip(1)
        .map(|l| l.split(',').collect())
        .collect();
    assert_eq!(rows.len(), values.len());
    for (row, v) in rows.iter().zip(&values) {
        assert_eq!(row[1], v.name);
        assert_eq!(row[2].parse::<f64>().unwrap().to_bits(), v.value.to_bits());
    }
}

/// Two SOI strips 6 µm apart, a TiN heater 2 µm above each, as a phase shifter: the catalogue's
/// indices and dn/dT, the heaters' turn-on in time.
const SHIFTER: &str = r#"
name = "shifter"
timeout_minutes = 5

[task]
kind = "static"
physics = "heat"
stack = "soi_220"
y_um = [-12.0, 12.0]
z_um = [-4.0, 5.0]
step_nm = 1000.0
fine_nm = 50.0

[[task.rect]]
layer = "Si"
center_um = [0.0, -3.0]
size_um = [10.0, 0.5]

[[task.rect]]
layer = "Si"
center_um = [0.0, 3.0]
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
step_ns = 200.0
duration_us = 60.0

[task.boundary]
bottom = { fixed = 0.0 }
top = { convective = 10.0, ambient = 0.0 }

[[task.region]]
y_um = [-4.0, -2.0]
z_um = [4.22, 4.34]
medium = "TiN"

[[task.region]]
y_um = [2.0, 4.0]
z_um = [4.22, 4.34]
medium = "TiN"

[[task.heater]]
name = "left"
y_um = [-4.0, -2.0]
z_um = [4.22, 4.34]
power_w_per_m = 10.0

[[task.heater]]
name = "right"
y_um = [2.0, 4.0]
z_um = [4.22, 4.34]
power_w_per_m = 10.0

[task.shifter]
wavelength_um = 1.55
step_nm = 40.0

[[task.guide]]
name = "left"
y_um = [-4.0, -2.0]
z_um = [1.2, 3.0]

[[task.guide]]
name = "right"
y_um = [2.0, 4.0]
z_um = [1.2, 3.0]
"#;

#[test]
fn a_phase_shifter_job_records_p_pi_crosstalk_and_times() {
    let ev = events("shifter", SHIFTER);
    let (_, _, _, values) = result(&ev);
    let p = named(&values, "P_pi of left on left");
    // a 2D number, W, for a TiN heater 2 µm above a strip: tens of mW
    assert!(p > 0.01 && p < 0.05, "{p}");
    assert!((named(&values, "P_pi of right on right") / p - 1.0).abs() < 1e-3);
    let x = named(&values, "crosstalk of left on right");
    assert!(x > 0.0 && x < 0.5, "{x}");
    assert!((named(&values, "crosstalk of right on left") / x - 1.0).abs() < 1e-2);
    let n = named(&values, "n_eff of left");
    assert!(n > 2.0 && n < 3.0, "{n}");
    assert!(named(&values, "dn_eff/dT of left") > 1e-4);
    // the catalogue's silicon at 300 K gave its index and dn/dT
    assert!(
        values
            .iter()
            .any(|v| v.name.starts_with("dn/dT of Si (") && v.value > 1.5e-4)
    );
    let tau = named(&values, "phase 1/e time of left by left");
    assert!(tau > 1e-6 && tau < 3e-5, "{tau}");
    assert!(named(&values, "phase 10-90 % time of left by left") > tau);
    assert!((named(&values, "P_pi * 1/e time of left") / (p * tau) - 1.0).abs() < 1e-12);
    assert!(named(&values, "phase -3 dB frequency of left by left") > 1e3);
}

#[test]
fn a_phase_shifter_job_names_what_it_lacks() {
    let no_guides = SHIFTER.split("[[task.guide]]").next().unwrap().to_owned();
    let no_shifter = SHIFTER.replace("[task.shifter]\nwavelength_um = 1.55\nstep_nm = 40.0\n", "");
    for (text, says) in [
        (no_guides, "needs a guide"),
        (no_shifter, "give it a [task.shifter]"),
        (
            SHIFTER.replace("wavelength_um = 1.55", "wavelength_um = -1.0"),
            "wavelength_um",
        ),
    ] {
        assert_ne!(text, SHIFTER, "{says}");
        let job = Job::parse(&text).unwrap();
        let e = check(&job).unwrap_err().to_string();
        assert!(e.contains(says), "{says}: {e}");
    }
}

#[test]
fn a_phase_shifter_job_fits_a_thermal_compact_model() {
    let text = SHIFTER.replace("step_nm = 40.0\n", "step_nm = 40.0\npoles = 4\n");
    let ev = events("shifter-rc", &text);
    let (_, _, _, values) = result(&ev);
    let e1 = named(&values, "thermal model error, 1 poles");
    let e4 = named(&values, "thermal model error, 4 poles");
    assert!(e4 < 0.2 * e1 && e4 < 0.02, "{e1} then {e4}");
    let mut steady = 0.0;
    for k in 1..=4 {
        assert!(named(&values, &format!("thermal model time constant {k}")) > 0.0);
        steady += named(
            &values,
            &format!("thermal model residue {k} of left by left"),
        );
    }
    // the model's steady phase is the run's own, 2π s/λ per W
    let s = named(&values, "dn_eff/dP of left by left");
    let own = 2.0 * std::f64::consts::PI * s / 1.55e-6;
    assert!((steady / own - 1.0).abs() < 1e-9, "{steady} against {own}");
    let no_transient = text.replace(
        "[task.transient]\nstep_ns = 200.0\nduration_us = 60.0\n",
        "",
    );
    let e = check(&Job::parse(&no_transient).unwrap())
        .unwrap_err()
        .to_string();
    assert!(e.contains("give a [task.transient]"), "{e}");
    let e = check(&Job::parse(&text.replace("poles = 4", "poles = 20")).unwrap())
        .unwrap_err()
        .to_string();
    assert!(e.contains("1 to 12 poles"), "{e}");
}

#[test]
fn a_phase_shifter_job_gives_the_same_bits_on_one_thread_and_many() {
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| result(&events(&format!("shifter-bits{threads}"), SHIFTER)).3)
    };
    assert_eq!(run(1), run(8));
}

/// jobs/pcm-switching.yaml on a coarser grid (5 nm at its finest, steps of 2 ns), for speed.
fn pcm_job(edit: &dyn Fn(String) -> String) -> Job {
    let text = include_str!("../../jobs/pcm-switching.yaml")
        .replace("fine_nm: 1.25", "fine_nm: 5.0")
        .replace("step_ns: 0.5", "step_ns: 2.0");
    Job::parse_as(&edit(text), crate::run::Format::Yaml).unwrap()
}

fn pcm_values(tag: &str, edit: &dyn Fn(String) -> String) -> Vec<StaticValue> {
    let job = pcm_job(edit);
    check(&job).unwrap();
    let root = temp(tag);
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    result(&replay(run.dir()).unwrap()).3
}

#[test]
fn a_pulse_amorphizes_the_phase_change_film_and_says_so() {
    let values = pcm_values("pcm-pulse", &|t| t);
    let cold = named(&values, "highest of film's coldest cell");
    let hot = named(&values, "highest of film's hottest cell");
    // static/pcm-cell-convergence's cell on this grid, in steps of 2 ns: 931.3 and 1106.7 K
    assert!(
        (cold - 931.3).abs() < 1.5 && (hot - 1106.7).abs() < 1.5,
        "{cold} {hot}"
    );
    assert_eq!(named(&values, "T_m of sb2se3"), 884.0);
    assert_eq!(named(&values, "film after the pulse: amorphized"), 1.0);
    assert!((named(&values, "pulse") - 1e-7).abs() < 1e-20);
    // the film's κ and ρc_p are the catalogue's
    assert_eq!(
        named(
            &values,
            "thermal conductivity of sb2se3-crystalline (Sb₂Se₃, aryana-2023)"
        ),
        0.72
    );
    assert!(
        values
            .iter()
            .any(|v| v.name.starts_with("heat capacity of sb2se3-crystalline"))
    );
    // a weaker pulse leaves the whole film below its melting temperature
    let values = pcm_values("pcm-weak", &|t| {
        t.replace("power_w_per_m: 8000.0", "power_w_per_m: 4000.0")
    });
    assert!(
        values
            .iter()
            .any(|v| v.name.starts_with("film after the pulse: ")
                && v.name.ends_with("crystallized")),
        "{values:?}"
    );
}

#[test]
fn a_pulse_and_its_switching_name_what_they_lack() {
    type Edit = Box<dyn Fn(String) -> String>;
    let cases: [(Edit, &str); 6] = [
        (
            Box::new(|t| t.replace("pulse_ns: 100.0", "pulse_ns: 101.0")),
            "whole number of steps",
        ),
        (
            Box::new(|t| t.replace("pulse_ns: 100.0", "pulse_ns: 400.0")),
            "shorter than duration_us",
        ),
        (
            Box::new(|t| t.replace("    pulse_ns: 100.0", "")),
            "give [task.transient] a pulse_ns",
        ),
        (
            Box::new(|t| t.replace("probe: film, material", "probe: guide, material")),
            "which no [[task.probe]] is",
        ),
        (
            Box::new(|t| t.replace("material: sb2se3 }", "material: glass }")),
            "sb2se3, sb2s3 or gst",
        ),
        (
            Box::new(|t| t.replace("pulse_ns: 100.0", "pulse_ns: 100.0\n    turn: \"off\"")),
            "leave turn out",
        ),
    ];
    for (edit, words) in cases {
        let e = check(&pcm_job(&*edit)).unwrap_err().to_string();
        assert!(e.contains(words), "{words}: {e}");
    }
    // every catalogued material has its switching temperatures now
    for id in ["sb2s3", "gst"] {
        let job =
            pcm_job(&|t: String| t.replace("material: sb2se3 }", &format!("material: {id} }}")));
        assert!(check(&job).is_ok(), "{id}");
    }
}
