//! A device's studies ([`super::study_from_device`]): each point the run of its own design's
//! job, to the bit, on any number of threads; corners the runs of their offset designs; and what
//! [`super::check`] refuses.

use super::*;
use crate::circuit::Fidelity;
use crate::device::{LayerSpec, MaterialSpec, Spec, StackSpec};
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
        "photonoxide-study-{tag}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    TempDir(dir)
}

fn spec(kind: &str, fidelity: Fidelity, parameters: &[(&str, &str)]) -> Spec {
    let mut s = Spec::new(kind);
    s.fidelity = fidelity;
    s.parameters = parameters
        .iter()
        .map(|&(n, e)| (n.to_owned(), e.to_owned()))
        .collect();
    s
}

/// Runs `job` and reads its record back.
fn run_of(job: &Job, tag: &str) -> Vec<Event> {
    let root = temp(tag);
    let mut run = Run::create(&root.0, job).unwrap();
    execute(job, &mut run, &Stop::new(None)).unwrap();
    replay(run.dir()).unwrap()
}

/// The lines of `job`'s record on `threads` threads, but the finish's time.
fn lines_on(job: &Job, threads: usize, tag: &str) -> Vec<String> {
    let root = temp(tag);
    let mut run = Run::create(&root.0, job).unwrap();
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap()
        .install(|| execute(job, &mut run, &Stop::new(None)))
        .unwrap();
    crate::run::replay_lines(run.dir())
        .unwrap()
        .into_iter()
        .filter(|l| !l.starts_with("{\"type\":\"finished\""))
        .collect()
}

/// The band the tests compute: five wavelengths about 1.55 µm.
fn band() -> Compute {
    Compute {
        band: Some((1.53, 1.57, 5)),
        ..Compute::model()
    }
}

/// The study's points.
fn points(events: &[Event]) -> Vec<Event> {
    events
        .iter()
        .filter(|e| matches!(e, Event::DevicePoint { .. }))
        .cloned()
        .collect()
}

/// Checks that each point of `events` is the run of its design's own spectrum job, to the bit:
/// its figures and its S-parameters at every wavelength.
fn each_point_is_its_designs_run(events: &[Event], designs: &[Spec], compute: &Compute, tag: &str) {
    let points = points(events);
    assert_eq!(points.len(), designs.len(), "{tag}");
    for (k, (p, d)) in points.iter().zip(designs).enumerate() {
        let Event::DevicePoint {
            point,
            error,
            figures,
            s,
            values,
            ..
        } = p
        else {
            unreachable!()
        };
        assert_eq!(*point, k, "{tag}");
        assert_eq!(error, &None, "{tag}: point {k}");
        let (_, design) = d.build().unwrap();
        assert_eq!(values, design.values().as_slice(), "{tag}: point {k}");
        let job = from_device(&format!("{tag}-{k}"), d, compute).unwrap();
        let single = run_of(&job, tag);
        let Some(Event::DeviceModel { figures: f, .. }) = single
            .iter()
            .find(|e| matches!(e, Event::DeviceModel { .. }))
        else {
            panic!("{tag}: no model");
        };
        assert_eq!(figures.len(), f.len(), "{tag}");
        for (a, b) in figures.iter().zip(f) {
            assert_eq!(a.symbol, b.symbol);
            assert_eq!(
                a.value.to_bits(),
                b.value.to_bits(),
                "{tag}: {k} {}",
                a.symbol
            );
            assert_eq!(a.grid, b.grid);
        }
        let spectra: Vec<&Vec<Vec<[f64; 2]>>> = single
            .iter()
            .filter_map(|e| match e {
                Event::SParameters { s, .. } => Some(s),
                _ => None,
            })
            .collect();
        assert_eq!(s.len(), spectra.len(), "{tag}");
        for (w, (a, b)) in s.iter().zip(spectra).enumerate() {
            for (ra, rb) in a.iter().zip(b) {
                for (x, y) in ra.iter().zip(rb) {
                    assert_eq!(
                        (x[0].to_bits(), x[1].to_bits()),
                        (y[0].to_bits(), y[1].to_bits()),
                        "{tag}: point {k}, wavelength {w}"
                    );
                }
            }
        }
    }
}

#[test]
fn a_sweep_in_two_parameters_is_each_designs_run_to_the_bit() {
    let s = spec("directional coupler", Fidelity::Analytic, &[]);
    let study = Study::Sweep(vec![
        SweepAxis::new("gap", 0.15, 0.3, 4),
        SweepAxis::new("length", 5.0, 20.0, 3),
    ]);
    let job = study_from_device("gap-length", &s, &band(), &study).unwrap();
    check(&job).unwrap();
    let events = run_of(&job, "gap-length");
    assert!(matches!(&events[0], Event::Started { kind, .. } if kind == "device"));
    assert!(matches!(&events[1], Event::Scene { .. }));
    assert!(matches!(&events[2], Event::Ports { .. }));
    let Event::DeviceStudy {
        study: kind,
        points: n,
        axes,
        axis_units,
        wavelengths_um,
        ports,
        fidelity,
        ..
    } = &events[3]
    else {
        panic!("no study: {:?}", events[3]);
    };
    assert_eq!(
        (kind.as_str(), *n, fidelity.as_str()),
        ("sweep", 12, "analytic")
    );
    assert_eq!(axes.len(), 2);
    assert_eq!(axis_units, &["µm", "µm"]);
    assert_eq!(wavelengths_um.len(), 5);
    assert_eq!(ports.len(), 4);
    // the second axis fastest
    let at: Vec<Vec<f64>> = points(&events)
        .iter()
        .map(|p| match p {
            Event::DevicePoint { at, .. } => at.clone(),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(at[0], [0.15, 5.0]);
    assert_eq!(at[1], [0.15, 12.5]);
    assert_eq!(at[3][1], 5.0);
    assert_eq!(at[11], [0.3, 20.0]);
    let designs = study.designs(&s).unwrap();
    each_point_is_its_designs_run(&events, &designs, &band(), "gap-length");
    assert!(matches!(
        events.last(),
        Some(Event::Finished { stopped: None, .. })
    ));
}

#[test]
fn a_sweep_records_the_same_bits_on_one_thread_and_twenty() {
    let s = spec("all-pass ring", Fidelity::Analytic, &[]);
    let study = Study::Sweep(vec![SweepAxis::new("radius", 5.0, 20.0, 31)]);
    let job = study_from_device("ring-radius", &s, &band(), &study).unwrap();
    let one = lines_on(&job, 1, "one");
    let twenty = lines_on(&job, 20, "twenty");
    assert_eq!(one.len(), twenty.len());
    assert_eq!(one, twenty);
}

#[test]
fn a_sweep_of_solves_finishes_and_agrees_on_one_thread_and_on_as_many_as_its_points() {
    // each point a 3D solve whose own work runs on rayon's pool, solved on the point's own task:
    // a point that waited on a thread of its own held a worker the solve needed, and with as
    // many points in flight as the pool's workers (four on CI's runners) nothing finished
    let s = spec("directional coupler", Fidelity::ThreeD, &[]);
    let compute = Compute {
        step_nm: Some(50.0),
        margin_um: Some(0.5),
        band: Some((1.55, 1.55, 1)),
        ..Compute::model()
    };
    let study = Study::Sweep(vec![SweepAxis::new("gap", 0.15, 0.25, 3)]);
    let job = study_from_device("solves", &s, &compute, &study).unwrap();
    let one = lines_on(&job, 1, "solves-one");
    let three = lines_on(&job, 3, "solves-three");
    assert_eq!(one, three);
    // and on rayon's own pool, whatever its size (the suite runs it on one thread too)
    let root = temp("solves-global");
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let global: Vec<String> = crate::run::replay_lines(run.dir())
        .unwrap()
        .into_iter()
        .filter(|l| !l.starts_with("{\"type\":\"finished\""))
        .collect();
    assert_eq!(global, one);
    let points = one
        .iter()
        .filter(|l| l.starts_with("{\"type\":\"device_point\""))
        .count();
    assert_eq!(points, 3);
}

#[test]
fn a_parameter_the_expressions_name_is_swept_through_them() {
    // w drives the width and the gap: each point is the design with w at that value, in µm
    let s = spec(
        "directional coupler",
        Fidelity::Analytic,
        &[("w", "450 nm"), ("width", "w"), ("gap", "w/2.5")],
    );
    let study = Study::Sweep(vec![SweepAxis::new("w", 0.4, 0.55, 4)]);
    let designs = study.designs(&s).unwrap();
    assert_eq!(designs[1].parameters[0], ("w".into(), "0.45 um".into()));
    let (_, d) = designs[3].build().unwrap();
    assert_eq!(d.values().get("width"), Some(0.55));
    assert_eq!(d.values().get("gap"), Some(0.55 / 2.5));
    let job = study_from_device("w", &s, &band(), &study).unwrap();
    each_point_is_its_designs_run(&run_of(&job, "w"), &designs, &band(), "w");
}

#[test]
fn corners_are_the_runs_of_their_offset_designs() {
    let s = spec("waveguide", Fidelity::Analytic, &[("length", "100")]);
    let study = Study::Corners(vec![
        Corner::new("wide", &[("width", 0.02)]),
        Corner::new("narrow, thin", &[("width", -0.02), ("thickness", -0.01)]),
        Corner::new("lossy", &[("loss", 1.0)]),
    ]);
    let designs = study.designs(&s).unwrap();
    assert_eq!(designs.len(), 4);
    assert_eq!(designs[0], s);
    let width = |k: usize| designs[k].build().unwrap().1.values().get("width").unwrap();
    assert_eq!(width(1), 0.5 + 0.02);
    assert_eq!(width(2), 0.5 - 0.02);
    let job = study_from_device("corners", &s, &band(), &study).unwrap();
    let events = run_of(&job, "corners");
    let names: Vec<Option<String>> = points(&events)
        .iter()
        .map(|p| match p {
            Event::DevicePoint { corner, .. } => corner.clone(),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(
        names,
        ["nominal", "wide", "narrow, thin", "lossy"].map(|n| Some(n.to_owned()))
    );
    each_point_is_its_designs_run(&events, &designs, &band(), "corners");
    // the job's text reads back to the same study
    let again = Job::parse(job.text()).unwrap();
    assert_eq!(run_of(&again, "again").len(), events.len());
}

#[test]
fn a_corners_temperature_sets_every_material_that_takes_one() {
    // SOI as its preset: Li's silicon and Malitson's silica, both moved by their dn/dT
    let s = spec("waveguide", Fidelity::ThreeD, &[]);
    let study = Study::Corners(vec![Corner::new("hot", &[]).at_temperature(340.0)]);
    let designs = study.designs(&s).unwrap();
    let StackSpec::Layers {
        cladding,
        layers,
        substrate,
        ..
    } = &designs[1].stack
    else {
        panic!("layers");
    };
    assert_eq!(cladding.temperature, Some(340.0));
    assert_eq!(substrate.temperature, Some(340.0));
    assert!(
        layers
            .iter()
            .all(|l| l.material.temperature == Some(340.0)
                && l.background.temperature == Some(340.0))
    );
    let compute = Compute {
        step_nm: Some(50.0),
        margin_um: Some(0.5),
        band: Some((1.55, 1.55, 1)),
        ..Compute::model()
    };
    let job = study_from_device("hot", &s, &compute, &study).unwrap();
    let events = run_of(&job, "hot");
    each_point_is_its_designs_run(&events, &designs, &compute, "hot");
    let n = |p: &Event| match p {
        Event::DevicePoint { figures, .. } => figures[0].value,
        _ => unreachable!(),
    };
    let p = points(&events);
    // silicon's index rises by about 1.9e-4 per K, and the mode's with it: about 47 K × 1.9e-4
    // times the share of the mode in silicon
    let rise = n(&p[1]) - n(&p[0]);
    assert!(rise > 3e-3 && rise < 1e-2, "{} {}", n(&p[1]), n(&p[0]));
}

#[test]
fn a_corners_temperature_is_refused_when_no_material_takes_one() {
    let nitride = MaterialSpec::id("si3n4");
    let mut s = spec("waveguide", Fidelity::ThreeD, &[]);
    s.stack = StackSpec::Layers {
        substrate: nitride.clone(),
        layers: vec![LayerSpec {
            name: "core".into(),
            thickness_um: 0.22,
            material: nitride.clone(),
            background: nitride.clone(),
        }],
        cladding: nitride,
        core: "core".into(),
    };
    let e = Study::Corners(vec![Corner::new("hot", &[]).at_temperature(340.0)])
        .designs(&s)
        .unwrap_err()
        .to_string();
    assert!(e.contains("takes a temperature"), "{e}");
}

#[test]
fn a_point_the_model_refuses_says_why_and_the_study_goes_on() {
    // the analytic fit holds for widths of 0.4 to 0.6 µm or so: wider points are refused there
    let s = spec("waveguide", Fidelity::Analytic, &[]);
    let study = Study::Sweep(vec![SweepAxis::new("width", 0.45, 2.0, 3)]);
    let job = study_from_device("wide", &s, &band(), &study).unwrap();
    let events = run_of(&job, "wide");
    let p = points(&events);
    assert_eq!(p.len(), 3);
    let Event::DevicePoint {
        error, s, values, ..
    } = &p[2]
    else {
        unreachable!()
    };
    assert!(
        error.as_deref().is_some_and(|e| e.contains("width")),
        "{error:?}"
    );
    assert!(s.is_empty());
    assert_eq!(values[0], 2.0);
    assert!(matches!(&p[0], Event::DevicePoint { error: None, .. }));
}

#[test]
fn the_check_refuses_what_isnt_a_study() {
    let s = spec("directional coupler", Fidelity::Analytic, &[]);
    let refused = |study: Study, what: &str| {
        let e = study_from_device("bad", &s, &band(), &study)
            .unwrap_err()
            .to_string();
        assert!(e.contains(what), "{what}: {e}");
    };
    refused(Study::Sweep(vec![]), "one or two");
    refused(
        Study::Sweep(vec![
            SweepAxis::new("gap", 0.1, 0.2, 2),
            SweepAxis::new("length", 1.0, 2.0, 2),
            SweepAxis::new("width", 0.4, 0.5, 2),
        ]),
        "one or two",
    );
    refused(
        Study::Sweep(vec![
            SweepAxis::new("gap", 0.1, 0.2, 2),
            SweepAxis::new("gap", 0.1, 0.3, 2),
        ]),
        "one axis only",
    );
    refused(
        Study::Sweep(vec![SweepAxis::new("gap", 0.1, 0.1, 5)]),
        "distinct ends",
    );
    refused(
        Study::Sweep(vec![SweepAxis::new("gap", 0.1, 0.2, 1)]),
        "2 to 1001",
    );
    refused(
        Study::Sweep(vec![
            SweepAxis::new("gap", 0.1, 0.2, 1001),
            SweepAxis::new("length", 1.0, 2.0, 11),
        ]),
        "at most 10000",
    );
    refused(
        Study::Sweep(vec![SweepAxis::new("radius", 1.0, 2.0, 3)]),
        "no parameter radius",
    );
    // a gap beyond the parameter's range, at the sweep's end
    refused(
        Study::Sweep(vec![SweepAxis::new("gap", 0.1, 50.0, 3)]),
        "gap",
    );
    refused(Study::Corners(vec![]), "1 to 256");
    refused(
        Study::Corners(vec![Corner::new("nominal", &[("gap", 0.01)])]),
        "nominal",
    );
    refused(
        Study::Corners(vec![Corner::new("still", &[])]),
        "moves nothing",
    );
    refused(
        Study::Corners(vec![
            Corner::new("a", &[("gap", 0.01)]),
            Corner::new("a", &[("gap", -0.01)]),
        ]),
        "two corners",
    );
    // a temperature at the analytic fidelity
    refused(
        Study::Corners(vec![Corner::new("hot", &[]).at_temperature(340.0)]),
        "analytic",
    );
    // by FDTD
    let e = study_from_device(
        "fdtd",
        &s,
        &Compute::fdtd(2),
        &Study::Sweep(vec![SweepAxis::new("gap", 0.1, 0.2, 2)]),
    )
    .unwrap_err()
    .to_string();
    assert!(e.contains("FDTD"), "{e}");
    // a job's text: axes in a corners study, an unknown key, an unknown kind
    let text = |task: &str| {
        format!("name = \"t\"\n[device]\nkind = \"directional coupler\"\n[task]\n{task}\n")
    };
    for (task, what) in [
        (
            "kind = \"corners\"\n[[task.axis]]\nparameter = \"gap\"\nfrom = 0.1\nto = 0.2\npoints = 2",
            "not axes",
        ),
        (
            "kind = \"sweep\"\nsteps = 3\n[[task.axis]]\nparameter = \"gap\"\nfrom = 0.1\nto = 0.2\npoints = 2",
            "steps",
        ),
        ("kind = \"simulate\"", "optimize"),
    ] {
        let e = check(&Job::parse(&text(task)).unwrap())
            .unwrap_err()
            .to_string();
        assert!(e.contains(what), "{what}: {e}");
    }
}

#[test]
fn a_study_farms_and_a_spectrum_doesnt() {
    let s = spec("waveguide", Fidelity::Analytic, &[]);
    let study = Study::Sweep(vec![SweepAxis::new("width", 0.45, 0.55, 3)]);
    assert!(farms(&study_from_device("s", &s, &band(), &study).unwrap()));
    assert!(!farms(&from_device("s", &s, &band()).unwrap()));
}

/// A study's run writes `study.csv` by default: a row each point's S-matrix element at each
/// wavelength, after the device's parameters, every number its [`Event::DevicePoint`]'s to the
/// bit.
#[test]
fn a_study_writes_its_points_as_a_table_to_the_bit() {
    let s = spec("all-pass ring", Fidelity::Analytic, &[]);
    let study = Study::Sweep(vec![SweepAxis::new("radius", 5.0, 20.0, 3)]);
    let job = study_from_device("ring-table", &s, &band(), &study).unwrap();
    let root = temp("table");
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    let csv = std::fs::read_to_string(run.dir().join("study.csv")).unwrap();
    let mut lines = csv.lines().map(crate::export::tables::fields);
    let header = lines.next().unwrap();
    let Some(Event::DeviceStudy {
        parameters,
        wavelengths_um,
        ports,
        ..
    }) = events
        .iter()
        .find(|e| matches!(e, Event::DeviceStudy { .. }))
    else {
        panic!("no study")
    };
    assert_eq!(header[2..2 + parameters.len()], parameters[..]);
    let rows: Vec<Vec<String>> = lines.collect();
    let n = 2 + parameters.len() + 1;
    let mut k = 0;
    for e in &events {
        let Event::DevicePoint {
            point, values, s, ..
        } = e
        else {
            continue;
        };
        for (w, m) in wavelengths_um.iter().zip(s) {
            for (q, row) in m.iter().enumerate() {
                for (p, z) in row.iter().enumerate() {
                    let r = &rows[k];
                    k += 1;
                    assert_eq!(r[0], point.to_string());
                    for (c, v) in values.iter().enumerate() {
                        assert_eq!(r[2 + c].parse::<f64>().unwrap().to_bits(), v.to_bits());
                    }
                    assert_eq!(r[n].parse::<f64>().unwrap().to_bits(), w.to_bits());
                    assert_eq!((&r[n + 1], &r[n + 2]), (&ports[q], &ports[p]));
                    assert_eq!(r[n + 3].parse::<f64>().unwrap().to_bits(), z[0].to_bits());
                    assert_eq!(r[n + 4].parse::<f64>().unwrap().to_bits(), z[1].to_bits());
                }
            }
        }
    }
    assert_eq!(k, rows.len());
    assert!(k > 0);
}
