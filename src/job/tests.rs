use super::*;
use crate::run::replay;
use std::path::PathBuf;

const JOB: &str = r#"
name = "strip"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 1.0]
y_um = [-1.0, 1.0]
step_nm = 50.0

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [2.0, 0.5]
"#;

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn temp(tag: &str) -> TempDir {
    let dir = std::env::temp_dir().join(format!("photonoxide-job-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    TempDir(dir)
}

/// The `results.json` a run wrote, checked against its schema (`docs/results.schema.json`)
/// as it was written, then read.
fn results_of(dir: &std::path::Path) -> results::Results {
    let text = std::fs::read_to_string(dir.join("results.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    if let Some(e) = results::invalid_by_schema(&json) {
        panic!("results.json isn't valid by its schema: {e}\n{text}");
    }
    results::Results::read(dir).unwrap()
}

#[test]
fn a_structure_job_records_a_top_and_a_side_view() {
    let root = temp("views");
    let job = Job::parse(JOB).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    assert_eq!(events.len(), 5);
    assert!(matches!(&events[0], Event::Started { kind, .. } if kind == "structure"));
    let Event::Scene { layers, shapes, .. } = &events[1] else {
        panic!("{:?}", events[1])
    };
    // soi_220: the buried oxide and the silicon layer; the strip a rectangle on it
    assert_eq!(layers.len(), 2);
    assert_eq!(shapes.len(), 1);
    assert_eq!(shapes[0].outline.len(), 4);
    assert!(layers[1].material.eps > 11.0 && layers[1].background.eps < 2.2);
    let Event::Permittivity { raster, axes, .. } = &events[2] else {
        panic!("{:?}", events[2])
    };
    assert_eq!(axes, &["x".to_owned(), "y".to_owned()]);
    assert_eq!((raster.nx, raster.ny), (40, 40));
    let Event::Permittivity { raster, axes, .. } = &events[3] else {
        panic!("{:?}", events[3])
    };
    assert_eq!(axes[1], "z");
    // the default cut spans 1 um below and above the 220 nm layer
    assert!((raster.y1 - raster.y0 - 2.22).abs() < 1e-9);
    assert!(matches!(&events[4], Event::Finished { stopped: None, .. }));
}

const PATH_JOB: &str = r#"
name = "euler"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 8.0]
y_um = [-1.0, 8.0]
step_nm = 100.0

[task.parameters]
w = "500 nm"
r = "5 um"

[[task.path]]
layer = "Si"
start = ["-1 um", 0]
width = "w"
segments = [
  { straight = "1 um" },
  { euler = "90 deg", radius = "r", p = 0.2 },
  { straight = "2 um", width = "w + 100 nm" },
  { sine = "4 um", offset = "-1 um" },
  { arc_rad = 0.5, radius_um = 3 },
]
"#;

#[test]
fn a_path_in_a_job_is_checked_drawn_and_rasterized() {
    let job = Job::parse(PATH_JOB).unwrap();
    check(&job).unwrap();
    let Event::Scene { shapes, .. } = preview(&job).unwrap() else {
        panic!("a structure's preview is a scene")
    };
    assert_eq!(shapes.len(), 1);
    assert!(shapes[0].outline.len() > 100);
    let task: StructureTask = params::task(&job).unwrap();
    let s = task.structure().unwrap();
    // on the Euler bend's centre line, and beside it
    let path = task.path[0].path(0).unwrap();
    let mid = path.point(Length::um(3.0));
    let si = s.stack().layer("Si").unwrap().1.to_um() + 0.1;
    let at = |p: Point| s.material_at(p, Length::um(si)).name().to_owned();
    assert_eq!(at(mid), "Si");
    assert_ne!(
        at(Point::um(mid.x.to_um() + 1.0, mid.y.to_um() - 1.0)),
        "Si"
    );
    // the widths: 500 nm, then 600 nm after the taper
    assert_eq!(path.start().width, Length::um(0.5));
    assert!((path.end().width.to_um() - 0.6).abs() < 1e-15);
}

#[test]
fn what_a_path_in_a_job_refuses() {
    let bad = |from: &str, to: &str, says: &str| {
        let job = Job::parse(&PATH_JOB.replace(from, to)).unwrap();
        let e = check(&job).unwrap_err().to_string();
        assert!(e.contains(says), "{e}");
    };
    bad(
        "euler = \"90 deg\"",
        "euler = \"90\"",
        "needs an angle, got a number",
    );
    bad("radius = \"r\", p = 0.2", "p = 0.2", "needs its radius");
    bad(
        "{ straight = \"1 um\" }",
        "{ straight = \"1 um\", arc = \"1 rad\" }",
        "exactly one",
    );
    bad("width = \"w\"", "width = \"20 um\"", "twice its");
    bad("p = 0.2", "p = \"2 um\"", "plain number");
    bad(
        "{ straight = \"1 um\" }",
        "{ straight = \"1 um\", straight_um = 1 }",
        "not both",
    );
}

#[test]
fn results_json_holds_the_s_parameters_of_a_direct_call_bit_for_bit() {
    let root = temp("results-fdfd");
    let job = Job::parse(FDFD).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let r = results_of(run.dir());
    assert_eq!(
        (r.schema, r.outcome.as_str(), r.kind.as_str()),
        (1, "finished", "fdfd")
    );
    assert_eq!(r.job, "strip-fdfd");
    assert!(r.error.is_none() && r.stopped.is_none() && r.seconds.is_some());
    assert!(
        !r.grids.is_empty(),
        "the solver's grid is recorded beside the numbers"
    );
    let s = r.s_parameters.as_ref().unwrap();
    assert_eq!(s.s.shape, [2, 2, 2]);
    assert_eq!(s.effective_indices.shape, [2, 2]);
    // the library's own call, on the same job: the same bits
    let direct = fdfd_s_parameters(&job, None).unwrap();
    assert_eq!(s.ports, direct.ports);
    assert_eq!(s.wavelengths_um, direct.wavelengths_um);
    let values = s.s.values(run.dir()).unwrap();
    let expected: Vec<f64> = direct
        .s
        .iter()
        .flatten()
        .flatten()
        .flat_map(|z| [z.re, z.im])
        .collect();
    assert_eq!(values, expected);
    let indices = s.effective_indices.values(run.dir()).unwrap();
    let expected: Vec<f64> = direct.effective_indices.iter().flatten().copied().collect();
    assert_eq!(indices, expected);
}

#[test]
fn results_json_holds_each_modes_numbers_as_recorded() {
    let root = temp("results-modes");
    let job = Job::parse(MODES).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    let r = results_of(run.dir());
    let recorded: Vec<([f64; 2], f64)> = events
        .iter()
        .filter_map(|e| match e {
            Event::Mode {
                effective_index,
                te_fraction,
                ..
            } => Some((*effective_index, *te_fraction)),
            _ => None,
        })
        .collect();
    assert!(!recorded.is_empty());
    let held: Vec<([f64; 2], f64)> = r
        .modes
        .iter()
        .map(|m| (m.effective_index, m.te_fraction))
        .collect();
    assert_eq!(held, recorded);
    assert_eq!(r.outcome, "finished");
    assert!(r.grids.iter().any(|g| g.step_um > 0.0 && g.unknowns > 0));
    // solved on one grid: each index is flagged unchecked, with its grid
    assert_eq!(r.convergence.len(), r.modes.len());
    for (c, m) in r.convergence.iter().zip(&r.modes) {
        assert_eq!(c.check.verdict, crate::convergence::Verdict::Unchecked);
        assert_eq!(
            (c.check.value, c.check.step_um),
            (m.effective_index[0], 0.025)
        );
    }
}

/// ROADMAP's pitfall, "a result quoted on one grid changed by 0.07 on the next", ruled
/// out for a modes job's indices: with `[task.convergence]` each is solved on the 50 and
/// 100 nm grids beside the run's 25 nm, and results.json says whether it has converged; an
/// index on an unconverged grid is caught, and refused where the task requires it.
#[test]
fn a_modes_jobs_index_on_an_unconverged_grid_is_caught() {
    let one = MODES.replace("modes = 2", "modes = 1");
    let checked = |tolerance: &str, require: bool| {
        let text = format!(
            "{one}\n[task.convergence]\ntolerance = {tolerance}\n{}",
            if require { "require = true\n" } else { "" }
        );
        let job = Job::parse(&text).unwrap();
        let root = temp(&format!("results-convergence-{tolerance}-{require}"));
        let mut run = Run::create(&root.0, &job).unwrap();
        let outcome = execute(&job, &mut run, &Stop::new(None));
        (outcome, results_of(run.dir()), root)
    };
    // the 25 nm grid's index moves by about 1e-3 from 50 nm: not within 1e-6
    let (outcome, r, _root) = checked("1e-6", false);
    outcome.unwrap();
    let c = &r.convergence[0].check;
    assert_eq!(
        c.verdict,
        crate::convergence::Verdict::NotConverged,
        "{c:?}"
    );
    assert_eq!(c.steps_um, [0.1, 0.05, 0.025]);
    assert_eq!(c.value, r.modes[0].effective_index[0]);
    assert!(c.error.is_some_and(|e| e > 1e-6), "{c:?}");
    assert!(c.gated().is_err());
    // within 0.05 it has
    let (outcome, r, _root) = checked("0.05", false);
    outcome.unwrap();
    let c = &r.convergence[0].check;
    assert!(c.converged(), "{c:?}");
    assert_eq!(c.gated().unwrap(), r.modes[0].effective_index[0]);
    // required, the unconverged run fails and says why
    let (outcome, r, _root) = checked("1e-6", true);
    let e = outcome.unwrap_err().to_string();
    assert!(e.contains("requires every mode's grid to converge"), "{e}");
    assert_eq!(r.outcome, "failed");
    // and what it refuses
    for (table, says) in [
        ("tolerance = 0.0", "above 0"),
        ("tolerance = 1e-4\ngrids = 4", "2 or 3"),
        (
            "tolerance = 1e-4\ngrids = 3\n[task.pml]\nthickness_um = [0.25, 0.25, 0.0, 0.0]",
            "PML",
        ),
    ] {
        let job = Job::parse(&format!("{one}\n[task.convergence]\n{table}\n")).unwrap();
        let e = check(&job).unwrap_err().to_string();
        assert!(e.contains(says), "{e} lacks {says}");
    }
}

#[test]
fn a_failed_run_records_its_error_and_its_results_say_so() {
    let root = temp("results-failed");
    let job = Job::parse(&JOB.replace("kind = \"structure\"", "kind = \"no-such-kind\"")).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    let e = execute(&job, &mut run, &Stop::new(None))
        .unwrap_err()
        .to_string();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    let Some(Event::Failed { error, .. }) = events.last() else {
        panic!("{events:?}")
    };
    assert_eq!(*error, e);
    let r = results_of(run.dir());
    assert_eq!(r.outcome, "failed");
    assert_eq!(r.error.as_deref(), Some(e.as_str()));
    // and one stopped says why
    let root = temp("results-stopped");
    let job = Job::parse(JOB).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    let stop = Stop::new(None);
    stop.request();
    execute(&job, &mut run, &stop).unwrap();
    let r = results_of(run.dir());
    assert_eq!(
        (r.outcome.as_str(), r.stopped.as_deref()),
        ("stopped", Some("requested"))
    );
}

#[test]
fn a_stopped_job_says_why() {
    let root = temp("stopped");
    let job = Job::parse(JOB).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    let stop = Stop::new(None);
    stop.request();
    execute(&job, &mut run, &stop).unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    // the scene and the top view, then it stops before the side view
    assert_eq!(events.len(), 4);
    assert!(matches!(
        &events[3],
        Event::Finished { stopped: Some(r), .. } if r == "requested"
    ));
}

const DEVICE: &str = r#"
name = "narrow-ring"

[device]
kind = "all-pass ring"
fidelity = "analytic"
stack = "soi_220"

[device.parameters]
w = "450 nm"
width = "w"
gap = "w/2.5"
radius = "5 um"
"#;

#[test]
fn a_device_job_is_checked_drawn_and_recorded() {
    let job = Job::parse(DEVICE).unwrap();
    check(&job).unwrap();
    let Event::Scene {
        x_um,
        y_um,
        z_um,
        shapes,
        layers,
        ..
    } = preview(&job).unwrap()
    else {
        panic!("not a scene")
    };
    // the bus and the ring on the Si layer, 1 um around them, 1 um below and above it
    assert_eq!(shapes.len(), 2);
    assert!(shapes.iter().all(|s| s.layer == "Si"));
    assert_eq!(layers.len(), 2);
    assert!((z_um[0] - 1.0).abs() < 1e-12 && (z_um[1] - 3.22).abs() < 1e-12);
    // the bus reaches past the ring by its width and 1 um, the ring's top is 2r + w + gap up
    let reach = 5.0 + 0.45 + 1.0;
    assert!((x_um[0] + reach + 1.0).abs() < 1e-9 && (x_um[1] - reach - 1.0).abs() < 1e-9);
    assert!(
        (y_um[1] - (0.45 + 0.18 + 10.0 + 0.225 + 1.0)).abs() < 1e-3,
        "{y_um:?}"
    );
    // a run records the scene
    let root = temp("device");
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    assert!(matches!(&events[0], Event::Started { kind, .. } if kind == "device"));
    assert!(matches!(&events[1], Event::Scene { .. }));
    assert!(matches!(&events[2], Event::Finished { stopped: None, .. }));
    // what isn't a design is refused, as is a task beside it
    for bad in [
        DEVICE.replace("all-pass ring", "taper"),
        DEVICE.replace("w/2.5", "w/0"),
        DEVICE.replace("\"5 um\"", "\"5 deg\""),
        format!("{DEVICE}\n[task]\nkind = \"structure\"\n"),
    ] {
        let job = Job::parse(&bad).unwrap();
        assert!(check(&job).is_err(), "{bad}");
    }
    // its text in JSON is the same job
    let json = crate::run::Format::Json;
    let json = Job::parse_as(&job.to_text(json).unwrap(), json).unwrap();
    assert_eq!(json.device(), job.device());
}

const MODES: &str = r#"
name = "strip-modes"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 1.0]
step_nm = 25.0
modes = 2

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [0.5, 10.0]
"#;

#[test]
fn a_modes_job_records_the_strips_modes() {
    let root = temp("modes");
    let job = Job::parse(MODES).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = found(&run);
    // started, the scene, the cross-section, two modes and their fields, finished
    assert_eq!(events.len(), 8, "{events:?}");
    let Event::Mode {
        effective_index,
        te_fraction,
        intensity,
        ..
    } = &events[3]
    else {
        panic!("{:?}", events[3])
    };
    // the 500 x 220 nm strip's TE-like mode, first; its field peaks in the core
    assert!(
        effective_index[0] > 2.3 && effective_index[0] < 2.6,
        "{effective_index:?}"
    );
    assert!(*te_fraction > 0.9);
    let (lo, hi) = intensity.range();
    assert!(lo >= 0.0 && (hi - 1.0).abs() < 1e-6);
    // its signed field: E_x, of one sign, on the same grid, peaking where |E|² does
    let Event::ModeField {
        label,
        component,
        values,
        ..
    } = &events[4]
    else {
        panic!("{:?}", events[4])
    };
    assert_eq!(label, "mode 1 of 2");
    assert_eq!(component, "Ex");
    assert_eq!((values.nx, values.ny), (intensity.nx, intensity.ny));
    assert!((values.range().1 - 1.0).abs() < 1e-6);
    // of one sign, but for a cell or two at the core's corners, where E_x is singular
    for j in 0..values.ny {
        for i in 0..values.nx {
            let v = values.at(i, j);
            let x = values.x0 + (i as f64 + 0.5) * (values.x1 - values.x0) / values.nx as f64;
            let z = values.y0 + (j as f64 + 0.5) * (values.y1 - values.y0) / values.ny as f64;
            let corner = [-0.25, 0.25]
                .iter()
                .any(|&cx| [2.0, 2.22].iter().any(|&cz| (x - cx).hypot(z - cz) < 0.06));
            assert!(v > -0.02 || corner, "{v} at ({x}, {z})");
        }
    }
    let top = values.values.iter().position(|&v| v == 1.0).unwrap();
    assert!(intensity.values[top] > 0.8, "{}", intensity.values[top]);
    // E_x is most of |E|² in the fundamental TE mode: its square follows |E|² closely
    let squares: Vec<f64> = values.values.iter().map(|&v| f64::from(v * v)).collect();
    let total: Vec<f64> = intensity.values.iter().map(|&v| f64::from(v)).collect();
    let mean = |a: &[f64]| a.iter().sum::<f64>() / a.len() as f64;
    let (ma, mb) = (mean(&squares), mean(&total));
    let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
    for (a, b) in squares.iter().zip(&total) {
        ab += (a - ma) * (b - mb);
        aa += (a - ma) * (a - ma);
        bb += (b - mb) * (b - mb);
    }
    let correlation = ab / (aa * bb).sqrt();
    assert!(correlation > 0.95, "{correlation}");
    let Event::Mode {
        effective_index: second,
        te_fraction,
        ..
    } = &events[5]
    else {
        panic!("{:?}", events[5])
    };
    assert!(second[0] < effective_index[0] && *te_fraction < 0.1);
    // the TM-like mode's field is mostly vertical
    assert!(matches!(&events[6], Event::ModeField { component, .. } if component == "Ez"));
}

/// `[task.export]` writes each mode into the run's directory, its `.npz`'s own
/// `effective_index` and the csv's row the same bits as the run's own [`Event::Mode`]: not a
/// separate recomputation, the solve's own result.
#[test]
fn a_modes_export_writes_each_mode_to_the_runs_directory() {
    use std::io::Read as _;
    let root = temp("modes-export");
    let text = format!("{MODES}\n[task.export]\nformat = \"both\"\n");
    let job = Job::parse(&text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = found(&run);
    let modes: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::Mode {
                label,
                effective_index,
                ..
            } => Some((label.clone(), *effective_index)),
            _ => None,
        })
        .collect();
    assert_eq!(modes.len(), 2, "{events:?}");

    let csv = std::fs::read_to_string(run.dir().join("modes.csv")).unwrap();
    let mut lines = csv.lines();
    assert_eq!(
        lines.next().unwrap(),
        "label,wavelength_um,n_eff_re,n_eff_im,loss_db_per_cm,te_fraction"
    );
    for (label, effective_index) in &modes {
        let path = run.dir().join(format!("{}.npz", label.replace(' ', "-")));
        let file = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        let mut archive = ::zip::ZipArchive::new(file).unwrap();
        let mut entry = archive.by_name("effective_index.npy").unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        // a scalar complex128 .npy: header then 16 bytes, re then im
        let data = &bytes[bytes.len() - 16..];
        let re = f64::from_le_bytes(data[0..8].try_into().unwrap());
        let im = f64::from_le_bytes(data[8..16].try_into().unwrap());
        assert_eq!([re, im], *effective_index, "{label}");

        let row = csv
            .lines()
            .find(|line| line.starts_with(label.as_str()))
            .unwrap_or_else(|| panic!("no csv row for {label}: {csv}"));
        let fields: Vec<&str> = row.split(',').collect();
        assert_eq!(fields[2].parse::<f64>().unwrap(), effective_index[0]);
        assert_eq!(fields[3].parse::<f64>().unwrap(), effective_index[1]);
    }
}

#[test]
fn what_a_modes_export_refuses() {
    let bad = |extra: &str| {
        let text = format!("{MODES}\n[task.export]\n{extra}\n");
        check(&Job::parse(&text).unwrap()).unwrap_err().to_string()
    };
    assert!(bad("format = \"hdf5\"").contains("format must be"));
    assert!(bad("format = \"npz\"\nprecision = \"f16\"").contains("precision must be"));
    assert!(bad("format = \"npz\"\nlevel = 10").contains("level must be at most 9"));
    assert!(bad("fields = [\"Bx\"]").contains("has no field \"Bx\""));
    assert!(bad("compression = \"lz4\"").contains("compression must be"));
    assert!(bad("compression = \"zstd\"\nlevel = 23").contains("1 to 22"));
    // no program here loaded libzstd: the check says so, rather than a run failing late
    let e = bad("format = \"npz\"\ncompression = \"zstd\"");
    assert!(e.contains("zstd isn't available"), "{e}");
    assert!(bad("container = \"zarr\"").contains("container must be"));
    assert!(bad("container = \"hdf5\"\ncompression = \"zstd\"").contains("libhdf5's own"));
    let e = bad("format = \"npz\"\ncontainer = \"hdf5\"");
    assert!(e.contains("hdf5 isn't available"), "{e}");
    assert!(bad("tables = [\"fields\"]").contains("has no table \"fields\""));
    assert!(bad("fields = [\"temperature\"]").contains("a modes job has no field"));
    assert!(bad("format = \"csv\"\nfields = [\"Ex\"]").contains("written as npz"));
    assert!(bad("format = \"npz\"\ntables = [\"modes\"]").contains("written as csv"));
    // an fdfd job in TE solves H along z, and its tables are all a device's study has
    let fdfd = format!("{FDFD}\n[task.export]\nfields = [\"Ez\"]\n");
    let e = check(&Job::parse(&fdfd).unwrap()).unwrap_err().to_string();
    assert!(e.contains("an fdfd job in TE has no field \"Ez\""), "{e}");
    let structure = format!("{JOB}\n[task.export]\nformat = \"csv\"\n");
    assert!(check(&Job::parse(&structure).unwrap()).is_err());
}

/// A run's pictures go into `events.bin`, and its record read back is the record the same
/// job writes in memory, line for line and byte for byte (the `finished` event's seconds
/// aside): a modes job's mode pictures, an fdfd job's field.
#[test]
fn a_runs_pictures_go_beside_its_record_and_replay_to_the_bit() {
    let root = temp("arrays");
    for text in [MODES, FDFD] {
        let job = Job::parse(text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        let mut memory = Run::in_memory();
        execute(&job, &mut memory, &Stop::new(None)).unwrap();
        let dir = run.dir();
        assert!(dir.join(crate::run::arrays::FILE).is_file(), "{dir:?}");
        let lines = crate::run::replay_lines(dir).unwrap();
        let unfinished = |l: &&String| !l.starts_with("{\"type\":\"finished\"");
        let ours: Vec<&String> = lines.iter().filter(unfinished).collect();
        let theirs: Vec<&String> = memory.lines().iter().filter(unfinished).collect();
        assert_eq!(ours, theirs);
        let record = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        let inline: usize = lines.iter().map(|l| l.len() + 1).sum();
        assert!(record.len() * 2 < inline, "{} of {inline}", record.len());
        let replayed: Vec<Event> = replay(dir).unwrap();
        let live: Vec<Event> = memory
            .lines()
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(replayed.len(), live.len());
        for (a, b) in replayed.iter().zip(&live) {
            if !matches!(a, Event::Finished { .. }) {
                assert_eq!(a, b);
            }
        }
    }
}

/// With no `[task.export]` a run writes its tables and no fields; asking for one field
/// writes it alone, beside the tables.
#[test]
fn a_run_writes_its_tables_by_default_and_fields_only_when_asked() {
    let npz_in = |run: &Run| -> Vec<String> {
        std::fs::read_dir(run.dir())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".npz"))
            .collect()
    };
    let root = temp("export-default");
    let job = Job::parse(MODES).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    assert!(run.dir().join("modes.csv").exists());
    assert!(npz_in(&run).is_empty(), "{:?}", npz_in(&run));

    let text = format!("{MODES}\n[task.export]\nfields = [\"Ey\"]\nprecision = \"f32\"\n");
    let job = Job::parse(&text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    assert!(run.dir().join("modes.csv").exists());
    let (arrays, meta) = crate::export::read::npz(&run.dir().join("mode-1-of-2.npz"));
    let names: Vec<_> = arrays.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            "Ey",
            "effective_index",
            "power",
            "te_fraction",
            "wavelength_um",
            "x_um",
            "y_um"
        ]
    );
    assert_eq!(arrays["Ey"].descr, "<c8");
    assert_eq!(arrays["x_um"].descr, "<f8");
    assert!(meta.contains("f32, downcast from f64"), "{meta}");
    assert!(meta.contains("[task.export]"), "the job's text: {meta}");
    // its size estimated before it is written again, at each precision and level
    crate::export::assert_estimates(&run.dir().join("mode-1-of-2.npz"));

    // format = "none": nothing beside the record
    let text = format!("{MODES}\n[task.export]\nformat = \"none\"\n");
    let job = Job::parse(&text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    assert!(!run.dir().join("modes.csv").exists());
    assert!(npz_in(&run).is_empty());
}

/// An fdfd job's `field.npz` is the run's own field: its |H_z|² over the pictured cells,
/// scaled as the [`Event::Field`] scales it, is that picture to the bit; and
/// `s_parameters.csv` is every [`Event::SParameters`]'s numbers to the bit.
#[test]
fn an_fdfd_export_is_the_runs_own_field_and_s_parameters_to_the_bit() {
    let root = temp("fdfd-export");
    let text = format!("{FDFD}\n[task.export]\nformat = \"both\"\n");
    let job = Job::parse(&text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = found(&run);
    let (arrays, meta) = crate::export::read::npz(&run.dir().join("field.npz"));
    assert!(meta.contains("unit amplitude"), "{meta}");
    crate::export::assert_estimates(&run.dir().join("field.npz"));
    let x = arrays["x_um"].f64s();
    let y = arrays["y_um"].f64s();
    let hz = arrays["Hz"].c64s();
    assert_eq!(arrays["Hz"].shape, [y.len(), x.len()]);
    assert!(!arrays.contains_key("Ez"));
    let Some(Event::Field {
        intensity,
        wavelength_um,
        ..
    }) = events.iter().find(|e| matches!(e, Event::Field { .. }))
    else {
        panic!("no field: {events:?}")
    };
    assert_eq!(arrays["wavelength_um"].f64s(), [*wavelength_um]);
    // the picture's first column and row: the cell whose left edge is its edge
    let dx = x[1] - x[0];
    let mx = x
        .iter()
        .position(|&c| (c - 0.5 * dx - intensity.x0).abs() < 1e-9)
        .unwrap();
    let my = y
        .iter()
        .position(|&c| (c - 0.5 * dx - intensity.y0).abs() < 1e-9)
        .unwrap();
    let values: Vec<f64> = (0..intensity.ny)
        .flat_map(|j| (0..intensity.nx).map(move |i| (i, j)))
        .map(|(i, j)| hz[(my + j) * x.len() + mx + i].norm_sqr())
        .collect();
    let peak = values.iter().copied().fold(0.0, f64::max);
    let scaled: Vec<f32> = values.iter().map(|v| (v / peak) as f32).collect();
    assert_eq!(scaled, intensity.values);

    let csv = std::fs::read_to_string(run.dir().join("s_parameters.csv")).unwrap();
    let rows: Vec<Vec<String>> = csv
        .lines()
        .skip(1)
        .map(crate::export::tables::fields)
        .collect();
    let mut expected = Vec::new();
    for e in &events {
        if let Event::SParameters {
            wavelength_um,
            ports,
            s,
            ..
        } = e
        {
            for (q, row) in s.iter().enumerate() {
                for (p, z) in row.iter().enumerate() {
                    expected.push((*wavelength_um, ports[q].clone(), ports[p].clone(), *z));
                }
            }
        }
    }
    assert_eq!(rows.len(), expected.len());
    for (row, (w, to, from, z)) in rows.iter().zip(&expected) {
        assert_eq!(row[0].parse::<f64>().unwrap().to_bits(), w.to_bits());
        assert_eq!((&row[1], &row[2]), (to, from));
        assert_eq!(row[3].parse::<f64>().unwrap().to_bits(), z[0].to_bits());
        assert_eq!(row[4].parse::<f64>().unwrap().to_bits(), z[1].to_bits());
    }
}

/// MODES, its modes travelling along x: the same strip turned a quarter turn.
#[test]
fn a_modes_job_finds_eight_modes() {
    // a strip guides two or three modes; the ones after them are the window's, close
    // together. Asking for six or more failed ("didn't converge in 20 restarts") while
    // each restart of the eigensolver kept a Krylov space of one size
    let events = run_events("modes-eight", &MODES.replace("modes = 2", "modes = 8"));
    let indices: Vec<f64> = events
        .iter()
        .filter_map(|e| match e {
            Event::Mode {
                effective_index, ..
            } => Some(effective_index[0]),
            _ => None,
        })
        .collect();
    // of the eight solved, only the strip's guided ones are kept: the window's own fall
    // below the oxide's index
    assert!(indices.len() >= 2 && indices.len() < 8, "{indices:?}");
    assert!(indices.iter().all(|&n| n > 1.44), "{indices:?}");
    // from the highest index down, each a different mode, the first two the strip's own
    assert!(
        indices.windows(2).all(|p| p[0] - p[1] > 1e-9),
        "{indices:?}"
    );
    let two = run_events("modes-eight-two", MODES);
    let first: Vec<f64> = two
        .iter()
        .filter_map(|e| match e {
            Event::Mode {
                effective_index, ..
            } => Some(effective_index[0]),
            _ => None,
        })
        .collect();
    assert_eq!(first.len(), 2);
    for (a, b) in first.iter().zip(&indices) {
        assert!((a - b).abs() < 1e-8, "{a} vs {b}");
    }
}

const MODES_X: &str = r#"
name = "strip-modes-x"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
propagation = "x"
y_um = [-1.0, 1.0]
step_nm = 25.0
modes = 2

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [10.0, 0.5]
"#;

/// The built-in modes jobs keep their modes by the index of the media around their guides,
/// from their stacks; the library's rule for a bare cross-section, from its open edges,
/// gives the same number on the cross-sections they solve (no shape reaches their sides).
#[test]
fn the_modes_jobs_media_are_their_cross_sections_edges() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
    for name in [
        "strip-modes.yaml",
        "strip-guided-modes.yaml",
        "strip-width-sweep.yaml",
        "coupler-supermodes.yaml",
    ] {
        let job = Job::load(&dir.join(name)).unwrap();
        let task = modes_task(&job, None).unwrap();
        let stack = || tensor_stack(&task.stack, task.core_nm, task.bottom_oxide_um).unwrap();
        let s = draw(stack(), &task.rect, &task.circle, &task.ring).unwrap();
        let z = modes_z(&task, &s).unwrap();
        let (along, across, cut) = (
            task.along().unwrap(),
            task.across().unwrap(),
            task.cut().unwrap(),
        );
        let frame = Frame::new(along, &task.rect, &task.circle, &task.ring, cut);
        let fs = frame.draw(stack()).unwrap();
        let lam = Wavelength::um(task.wavelength_um).unwrap();
        let cs = cross_section(
            &fs,
            &frame.rect,
            &frame.circle,
            &frame.ring,
            frame.cut,
            across,
            z,
            task.step_nm / 1000.0,
            lam,
            task.along().unwrap(),
        )
        .unwrap();
        let job_index = window_edge_highest(&task, &s, z, task.wavelength_um).unwrap();
        assert_eq!(cs.surrounding_index(lam).unwrap(), job_index, "{name}");
        assert!(job_index > 1.4, "{name}: {job_index}");
    }
}

fn run_events(tag: &str, text: &str) -> Vec<Event> {
    let root = temp(tag);
    let job = Job::parse(text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    replay(run.dir()).unwrap()
}

/// Every mode's effective index, TE fraction and pictures, in the order recorded.
fn modes_of(events: &[Event]) -> Vec<([f64; 2], f64, Raster, String, Raster)> {
    let mut out = Vec::new();
    for (k, e) in events.iter().enumerate() {
        if let Event::Mode {
            effective_index,
            te_fraction,
            intensity,
            ..
        } = e
        {
            let Event::ModeField {
                component, values, ..
            } = &events[k + 1]
            else {
                panic!("{:?}", events[k + 1])
            };
            out.push((
                *effective_index,
                *te_fraction,
                intensity.clone(),
                component.clone(),
                values.clone(),
            ));
        }
    }
    out
}

#[test]
fn a_guide_along_x_has_the_modes_of_the_same_guide_along_y() {
    // an asymmetric pair of strips, a ring off to one side, a cut off the middle; along y it
    // is the same structure turned a quarter turn: (x, y) along x is (y, −x) along y
    let along_x = r#"
name = "pair-x"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
propagation = "x"
y_um = [-1.2, 1.0]
cut_x_um = 0.7
step_nm = 40.0
modes = 3

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.1]
size_um = [10.0, 0.5]

[[task.rect]]
layer = "Si"
center_um = [0.5, -0.55]
size_um = [6.0, 0.3]

[[task.ring]]
layer = "Si"
center_um = [2.5, 0.0]
radius_um = 2.0
width_um = 0.3
"#;
    let along_y = r#"
name = "pair-y"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.2, 1.0]
cut_y_um = -0.7
step_nm = 40.0
modes = 3

[[task.rect]]
layer = "Si"
center_um = [0.1, 0.0]
size_um = [0.5, 10.0]

[[task.rect]]
layer = "Si"
center_um = [-0.55, -0.5]
size_um = [0.3, 6.0]

[[task.ring]]
layer = "Si"
center_um = [0.0, -2.5]
radius_um = 2.0
width_um = 0.3
"#;
    let (x, y) = (
        run_events("along-x", along_x),
        run_events("along-y", along_y),
    );
    let (mx, my) = (modes_of(&x), modes_of(&y));
    assert_eq!(mx.len(), 3);
    assert_eq!(my.len(), 3);
    for (a, b) in mx.iter().zip(&my) {
        for c in 0..2 {
            assert!((a.0[c] - b.0[c]).abs() < 1e-12, "{:?} {:?}", a.0, b.0);
        }
        assert!((a.1 - b.1).abs() < 1e-12);
        // the same pictures, across the guide (y along x, x along y) and up
        assert_eq!(a.2, b.2);
        assert_eq!(a.4, b.4);
        // the component across the guide is E_y along x and E_x along y
        match a.3.as_str() {
            "Ey" => assert_eq!(b.3, "Ex"),
            other => assert_eq!((other, b.3.as_str()), ("Ez", "Ez")),
        }
    }
    // the cut: named along x, where the job says; the pictures' axes and the scene follow
    assert!(x.iter().any(|e| matches!(
        e,
        Event::Cut { normal, at_um } if normal == "x" && *at_um == 0.7
    )));
    assert!(!y.iter().any(|e| matches!(e, Event::Cut { .. })));
    assert!(x.iter().any(|e| matches!(
        e,
        Event::Mode { cut_y_um, .. } if *cut_y_um == 0.7
    )));
    let Some(Event::Permittivity { view, axes, .. }) =
        x.iter().find(|e| matches!(e, Event::Permittivity { .. }))
    else {
        panic!("no cross-section")
    };
    assert_eq!(view, "cross-section at x = 0.7 um");
    assert_eq!(axes, &["y".to_owned(), "z".to_owned()]);
    let Event::Scene { x_um, y_um, .. } = &x[1] else {
        panic!("{:?}", x[1])
    };
    // a block behind the cut along x, as deep as the window across is wide
    assert!(
        (x_um[0] - (0.7 - 2.2)).abs() < 1e-12 && x_um[1] == 0.7,
        "{x_um:?}"
    );
    assert_eq!(y_um, &[-1.2, 1.0]);
}

#[test]
fn an_older_job_runs_as_before() {
    // a job without `propagation` is along y, as it always was: naming it changes nothing
    // but the cut it records
    let old = run_events("old", MODES);
    let named = run_events(
        "named-y",
        &MODES.replace(
            "layer = \"Si\"\nx_um",
            "layer = \"Si\"\npropagation = \"y\"\nx_um",
        ),
    );
    assert!(!old.iter().any(|e| matches!(e, Event::Cut { .. })));
    let same = |events: &[Event]| -> Vec<Event> {
        events
            .iter()
            .filter(|e| !matches!(e, Event::Cut { .. } | Event::Finished { .. }))
            .cloned()
            .collect()
    };
    assert_eq!(same(&old), same(&named));
    assert!(named.iter().any(|e| matches!(
        e,
        Event::Cut { normal, at_um } if normal == "y" && *at_um == 0.0
    )));
    // and the pictures name their axes as before
    assert!(old.iter().any(|e| matches!(
        e,
        Event::Permittivity { view, axes, .. }
            if view == "cross-section at y = 0 um" && axes[0] == "x"
    )));
    assert!(
        old.iter()
            .any(|e| matches!(e, Event::ModeField { component, .. } if component == "Ex"))
    );
}

#[test]
fn the_strip_jobs_along_x_find_what_they_found_along_y() {
    // jobs/strip-modes.yaml and jobs/strip-width-sweep.yaml (their TOML, tests/jobs-toml) as they were before light
    // travelled along x, and their effective indices then (0.4.0, this machine's last
    // digits aside): the jobs along x find the same
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/jobs-toml");
    let file = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    let older = |text: &str| {
        text.replace("propagation = \"x\"", "")
            .replace("y_um = [", "x_um = [")
            .replace("size_um = [10.0, 0.5]", "size_um = [0.5, 10.0]")
    };
    for (name, first, last) in [
        (
            "strip-modes.toml",
            [2.4497971939172727, 1.7853122219332593],
            [2.394612987693227, 1.7260069902533284],
        ),
        (
            "strip-width-sweep.toml",
            [2.4497972033509092, 1.7853414527523084],
            [2.68762994950837, 2.1757624280681673],
        ),
    ] {
        let text = file(name);
        assert!(text.contains("propagation = \"x\""), "{name}");
        // a short sweep: its first and last points only
        let short = |t: String| t.replace("points = 11", "points = 2");
        for (tag, events) in [
            ("now", run_events("jobs-now", &short(text.clone()))),
            ("then", run_events("jobs-then", &short(older(&text)))),
        ] {
            let indices: Vec<f64> = modes_of(&events).iter().map(|m| m.0[0]).collect();
            let points: Vec<Vec<f64>> = events
                .iter()
                .filter_map(|e| match e {
                    Event::SweepPoint {
                        effective_indices, ..
                    } => Some(effective_indices.iter().map(|n| n[0]).collect()),
                    _ => None,
                })
                .collect();
            let close = |a: &[f64], b: &[f64]| {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
            };
            assert!(close(&indices, &first), "{name}, {tag}: {indices:?}");
            assert!(close(&points[1], &last), "{name}, {tag}: {points:?}");
        }
    }
}

#[test]
fn a_width_sweep_along_x_widens_the_strip_along_y() {
    let text = format!(
        "{MODES_X}\n[task.sweep]\nparameter = \"width\"\nfrom = 0.45\nto = 0.55\npoints = 2\n"
    );
    let events = run_events("sweep-x", &text.replace("modes = 2", "modes = 1"));
    let spans: Vec<(f64, f64, f64)> = events
        .iter()
        .filter_map(|e| match e {
            Event::SweepShapes { value, shapes, .. } => {
                let span = |c: usize| {
                    let v = shapes[0].outline.iter().map(|p| p[c]);
                    v.clone().fold(f64::MIN, f64::max) - v.fold(f64::MAX, f64::min)
                };
                Some((*value, span(0), span(1)))
            }
            _ => None,
        })
        .collect();
    assert_eq!(spans.len(), 2);
    for (value, along, across) in spans {
        // still 10 µm long, its width across the guide the sweep's
        assert!((along - 10.0).abs() < 1e-9 && (across - value).abs() < 1e-9);
    }
    let points: Vec<f64> = events
        .iter()
        .filter_map(|e| match e {
            Event::SweepPoint {
                effective_indices, ..
            } => Some(effective_indices[0][0]),
            _ => None,
        })
        .collect();
    assert!(points[1] > points[0], "{points:?}");
    assert!(events.iter().any(|e| matches!(
        e,
        Event::SweepPermittivity { view, axes, .. }
            if view == "cross-section at x = 0 um" && axes[0] == "y"
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::SweepMode { component, .. } if component == "Ey"
    )));
}

#[test]
fn a_higher_order_modes_field_changes_sign_across_the_core() {
    let root = temp("te1");
    // a 1 µm strip guides TE0, TE1 and TM0; TE1 is odd in x
    let text = MODES
        .replace("size_um = [0.5, 10.0]", "size_um = [1.0, 10.0]")
        .replace("x_um = [-1.0, 1.0]", "x_um = [-1.2, 1.2]")
        .replace("step_nm = 25.0", "step_nm = 40.0")
        .replace("modes = 2", "modes = 3");
    let job = Job::parse(&text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    let fields: Vec<(&String, &Raster)> = events
        .iter()
        .filter_map(|e| match e {
            Event::ModeField {
                component, values, ..
            } => Some((component, values)),
            _ => None,
        })
        .collect();
    assert_eq!(fields.len(), 3);
    // E_x along the row through the core's middle, a quarter of the width either side
    let across = |r: &Raster, x: f64| {
        let i = (((x - r.x0) / (r.x1 - r.x0)) * r.nx as f64) as usize;
        // the silicon is 2 to 2.22 µm up, on the buried oxide
        let j = (((2.11 - r.y0) / (r.y1 - r.y0)) * r.ny as f64) as usize;
        r.values[j * r.nx + i]
    };
    let (_, te0) = fields[0];
    assert!(across(te0, -0.25) > 0.3 && across(te0, 0.25) > 0.3);
    let odd = fields.iter().any(|(c, r)| {
        let (a, b) = (across(r, -0.25), across(r, 0.25));
        *c == "Ex" && a * b < 0.0 && a.abs() > 0.3 && b.abs() > 0.3
    });
    assert!(odd, "no mode changes sign across the core");
}

#[test]
fn a_width_sweep_records_rising_indices() {
    let root = temp("sweep");
    // 450 to 550 nm: the index rises ever more slowly (a_cross_sections_nodes_hold_its_edges
    // checks the edges are on nodes, so the widths don't snap)
    let text = format!(
        "{MODES}\n[task.sweep]\nparameter = \"width\"\nfrom = 0.45\nto = 0.55\npoints = 3\n"
    );
    let job = Job::parse(&text.replace("modes = 2", "modes = 1")).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    let points: Vec<f64> = events
        .iter()
        .filter_map(|e| match e {
            Event::SweepPoint {
                effective_indices, ..
            } => Some(effective_indices[0][0]),
            _ => None,
        })
        .collect();
    assert_eq!(points.len(), 3);
    assert!(points.windows(2).all(|w| w[1] > w[0]), "{points:?}");
    // rising ever more slowly, as a widening strip does
    assert!(points[1] - points[0] > points[2] - points[1], "{points:?}");
    // each point's structure: the strip at that width
    let widths: Vec<(usize, f64, f64)> = events
        .iter()
        .filter_map(|e| match e {
            Event::SweepShapes {
                point,
                value,
                shapes,
            } => {
                let xs = shapes[0].outline.iter().map(|p| p[0]);
                let span = xs.clone().fold(f64::MIN, f64::max) - xs.fold(f64::MAX, f64::min);
                Some((*point, *value, span))
            }
            _ => None,
        })
        .collect();
    assert_eq!(widths.len(), 3);
    for (k, (point, value, span)) in widths.iter().enumerate() {
        assert_eq!(*point, k);
        assert!((value - (0.45 + 0.05 * k as f64)).abs() < 1e-9);
        assert!((span - value).abs() < 1e-9, "{span} {value}");
    }
    // and its mode, with its pictures, at that point's index and n_eff
    let modes: Vec<&Event> = events
        .iter()
        .filter(|e| matches!(e, Event::SweepMode { .. }))
        .collect();
    assert_eq!(modes.len(), 3);
    for (k, e) in modes.iter().enumerate() {
        let Event::SweepMode {
            point,
            label,
            effective_index,
            intensity,
            component,
            field,
            ..
        } = e
        else {
            unreachable!()
        };
        assert_eq!((*point, label.as_str()), (k, "mode 1 of 1"));
        assert_eq!(effective_index[0], points[k]);
        assert_eq!(component, "Ex");
        // on pixels twice the step: the 2 µm window in 40 pixels of 50 nm
        assert_eq!((intensity.nx, field.nx), (40, 40));
        assert!((intensity.range().1 - 1.0).abs() < 1e-6 && (field.range().1 - 1.0).abs() < 1e-6);
        // to three decimals
        assert!(
            field
                .values
                .iter()
                .all(|v| (v * 1000.0 - (v * 1000.0).round()).abs() < 1e-3)
        );
    }
    // the point's events come after its SweepPoint, the first after the sweep's own
    let first = events
        .iter()
        .position(|e| matches!(e, Event::SweepPoint { .. }))
        .unwrap();
    assert!(matches!(
        &events[first - 1],
        Event::Sweep { parameter, from, to, points: 3 }
            if parameter == "width" && (*from, *to) == (0.45, 0.55)
    ));
    assert!(matches!(
        &events[first + 1],
        Event::SweepShapes { point: 0, .. }
    ));
    assert!(matches!(
        &events[first + 2],
        Event::SweepPermittivity { point: 0, .. }
    ));
    assert!(matches!(
        &events[first + 3],
        Event::SweepMode { point: 0, .. }
    ));
    // each point's cross-section: silicon across the strip's width, wider point by point
    let silicon: Vec<usize> = events
        .iter()
        .filter_map(|e| match e {
            Event::SweepPermittivity { raster, .. } => {
                // the row through the middle of the 220 nm silicon, 2 to 2.22 µm up
                let j = ((2.11 - raster.y0) / (raster.y1 - raster.y0) * raster.ny as f64) as usize;
                Some((0..raster.nx).filter(|&i| raster.at(i, j) > 11.0).count())
            }
            _ => None,
        })
        .collect();
    assert_eq!(silicon.len(), 3);
    assert!(silicon.windows(2).all(|w| w[1] > w[0]), "{silicon:?}");
}

#[test]
fn a_wavelength_sweep_records_each_points_modes() {
    let root = temp("wavelength-sweep");
    let text = format!(
        "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n"
    );
    let job = Job::parse(&text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    // no shapes: the structure stays the job's
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::SweepShapes { .. }))
    );
    let modes: Vec<(usize, f64, String, f64)> = events
        .iter()
        .filter_map(|e| match e {
            Event::SweepMode {
                point,
                wavelength_um,
                label,
                te_fraction,
                ..
            } => Some((*point, *wavelength_um, label.clone(), *te_fraction)),
            _ => None,
        })
        .collect();
    assert_eq!(modes.len(), 4, "{modes:?}");
    for (k, (point, wavelength, label, te)) in modes.iter().enumerate() {
        assert_eq!(*point, k / 2);
        assert!((wavelength - [1.5, 1.6][k / 2]).abs() < 1e-12);
        assert_eq!(label, &format!("mode {} of 2", k % 2 + 1));
        // TE-like first, then TM-like, at both wavelengths
        assert!(if k % 2 == 0 { *te > 0.9 } else { *te < 0.1 });
    }
}

#[test]
fn a_cross_sections_nodes_hold_its_edges_and_interfaces() {
    // a 450 nm strip on a 20 nm grid from −1 µm: ±0.225 µm aren't multiples of the step, so
    // a uniform grid would snap the width; they, and the layers' interfaces, must be nodes
    let rect = RectSpec {
        layer: "Si".into(),
        center_um: [0.0, 0.0],
        size_um: [0.45, 10.0],
        etch: None,
    };
    let s = draw(LayerStack::soi_220(), std::slice::from_ref(&rect), &[], &[]).unwrap();
    let cs = cross_section(
        &s,
        &[rect],
        &[],
        &[],
        0.0,
        [-1.0, 1.0],
        [-1.0, 1.22],
        0.02,
        Wavelength::um(1.55).unwrap(),
        Along::Y,
    )
    .unwrap();
    let has = |nodes: &[f64], v: f64| nodes.iter().any(|&n| (n - v).abs() < 1e-12);
    assert!(has(cs.x(), -0.225) && has(cs.x(), 0.225), "{:?}", cs.x());
    assert!(has(cs.y(), 0.0) && has(cs.y(), 0.22), "{:?}", cs.y());
    // and the spacing stays about the step
    assert!(
        cs.x()
            .windows(2)
            .all(|w| w[1] - w[0] <= 0.02 + 1e-12 && w[1] - w[0] > 0.005)
    );
}

#[test]
fn a_circles_outline_is_a_counterclockwise_64_gon() {
    let c = Shape::circle(Point::um(1.0, 2.0), Length::um(0.5)).unwrap();
    let o = outline(&c);
    assert_eq!(o.len(), 64);
    // counterclockwise: a positive signed area, close to the disk's
    let area: f64 = (0..64)
        .map(|k| {
            let (a, b) = (o[k], o[(k + 1) % 64]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0;
    assert!(
        (area - std::f64::consts::PI * 0.25).abs() < 0.01 * area,
        "{area}"
    );
}

#[test]
fn a_preview_is_the_scene_its_run_records() {
    let root = temp("preview");
    let ring = "
[[task.ring]]
layer = \"Si\"
center_um = [0.0, 0.6]
radius_um = 0.5
width_um = 0.3
";
    for text in [
        format!("{JOB}{ring}"),
        format!("{MODES}{ring}"),
        format!("{MODES_X}{ring}"),
        format!("{FDFD}{ring}"),
    ] {
        let job = Job::parse(&text).unwrap();
        let preview = preview(&job).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        let stop = Stop::new(None);
        stop.request(); // the scene comes first; nothing more is needed
        let _ = execute(&job, &mut run, &stop);
        let events: Vec<Event> = crate::run::replay(run.dir()).unwrap();
        let recorded = events
            .iter()
            .find(|e| matches!(e, Event::Scene { .. }))
            .unwrap();
        assert_eq!(&preview, recorded);
        let Event::Scene { shapes, .. } = preview else {
            unreachable!()
        };
        assert_eq!(
            shapes.last().unwrap().outline.len(),
            130,
            "the ring, with its hole"
        );
    }
}

#[test]
fn a_rings_outline_walks_its_hole_through_a_cut() {
    let r = Shape::ring(Point::um(0.0, 0.0), Length::um(1.0), Length::um(0.5)).unwrap();
    let o = outline(&r);
    assert_eq!(o.len(), 64 + 1 + 64 + 1);
    // the first vertex recurs where the hole starts, and the hole closes on itself
    assert_eq!(o[64], o[0]);
    assert_eq!(o[129], o[65]);
    let area = |v: &[[f64; 2]]| {
        (0..v.len())
            .map(|k| {
                let (a, b) = (v[k], v[(k + 1) % v.len()]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            / 2.0
    };
    // outer counterclockwise, hole clockwise, and the whole walk the ring's area
    assert!(area(&o[..64]) > 0.0 && area(&o[65..129]) < 0.0);
    let exact = std::f64::consts::PI * (1.25 * 1.25 - 0.75 * 0.75);
    assert!((area(&o) - exact).abs() < 0.01 * exact, "{}", area(&o));
}

#[test]
fn the_check_refuses_a_wavelength_a_material_has_no_data_at() {
    // 5 µm is beyond the stack's oxide: the check says what the run would
    let sweep = |from: f64, to: f64| {
        format!(
            "[task.sweep]
parameter = \"wavelength\"
from = {from}
to = {to}
points = 2
"
        )
    };
    let fdfd_sweep = sweep(1.5, 1.6);
    assert!(FDFD.contains(&fdfd_sweep));
    for (what, text) in [
        (
            "structure",
            JOB.replace("wavelength_um = 1.55", "wavelength_um = 5.0"),
        ),
        (
            "modes",
            MODES.replace("wavelength_um = 1.55", "wavelength_um = 5.0"),
        ),
        (
            "modes sweep",
            format!(
                "{MODES}
{}",
                sweep(1.5, 5.0)
            ),
        ),
        (
            "fdfd",
            FDFD.replace("wavelength_um = 1.55", "wavelength_um = 5.0"),
        ),
        ("fdfd sweep", FDFD.replace(&fdfd_sweep, &sweep(5.0, 1.6))),
    ] {
        let job = Job::parse(&text).unwrap();
        let refused = check(&job).unwrap_err();
        assert!(
            matches!(&refused, Error::OutsideValidity { wavelength_um, .. } if *wavelength_um == 5.0),
            "{what}: {refused}"
        );
        // the run's own words
        let root = temp("range");
        let mut run = Run::create(&root.0, &job).unwrap();
        let failed = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
        assert_eq!(refused.to_string(), failed.to_string(), "{what}");
    }
    // a width sweep's ends are widths, not wavelengths
    let width = format!(
        "{MODES}
[task.sweep]
parameter = \"width\"
from = 0.4
to = 5.0
points = 2
"
    );
    check(&Job::parse(&width).unwrap()).unwrap();
}

/// The run's events without how it solved ([`Event::Solver`], [`Event::SolveError`]):
/// what it found, for the tests that read events by position.
fn found(run: &Run) -> Vec<Event> {
    replay(run.dir())
        .unwrap()
        .into_iter()
        .filter(|e| !matches!(e, Event::Solver { .. } | Event::SolveError { .. }))
        .collect()
}

#[test]
fn a_modes_job_records_its_solver_and_each_modes_residual() {
    let root = temp("modes-solver");
    let text = format!(
        "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n"
    );
    let job = Job::parse(&text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    // the solver, once, before the first mode: the 2 µm window in cells of 25 nm
    let at = events
        .iter()
        .position(|e| matches!(e, Event::Solver { .. }))
        .unwrap();
    let first_mode = events
        .iter()
        .position(|e| matches!(e, Event::Mode { .. }))
        .unwrap();
    assert!(at < first_mode);
    let Event::Solver {
        module,
        cells,
        step_um,
        unknowns,
        details,
    } = &events[at]
    else {
        unreachable!()
    };
    assert_eq!((module.as_str(), *step_um), ("mode::vector", 0.025));
    assert!(cells[0] >= 80 && cells[1] >= 80, "{cells:?}");
    // H_x and H_y at each node, less the walls'
    let nodes = (cells[0] + 1) * (cells[1] + 1);
    assert!(*unknowns <= 2 * nodes && *unknowns > nodes, "{unknowns}");
    assert!(details.iter().any(|[name, _]| name == "eigen-solver"));
    // each mode's residual, the job's own and each point's: within the solver's tolerance
    let errors: Vec<(Option<usize>, f64, String, f64)> = events
        .iter()
        .filter_map(|e| match e {
            Event::SolveError {
                point,
                value,
                measure,
                error,
            } => Some((*point, *value, measure.clone(), *error)),
            _ => None,
        })
        .collect();
    let places: Vec<(Option<usize>, f64)> = errors.iter().map(|e| (e.0, e.1)).collect();
    assert_eq!(
        places,
        [
            (None, 1.55),
            (None, 1.55),
            (Some(0), 1.5),
            (Some(0), 1.5),
            (Some(1), 1.6),
            (Some(1), 1.6)
        ]
    );
    for (k, (_, _, measure, error)) in errors.iter().enumerate() {
        assert_eq!(*measure, format!("eigen-residual, mode {} of 2", k % 2 + 1));
        assert!(*error > 0.0 && *error < 1e-9, "{measure}: {error:e}");
    }
}

#[test]
fn an_fdfd_job_records_its_solver_and_each_solves_errors() {
    let root = temp("fdfd-solver");
    let job = Job::parse(FDFD).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    // the solver right after the scene: the 2.8 µm window in cells of 40 nm
    let Event::Solver {
        module,
        cells,
        step_um,
        unknowns,
        details,
    } = &events[2]
    else {
        panic!("{:?}", events[2])
    };
    assert_eq!(
        (module.as_str(), *cells, *step_um, *unknowns),
        ("fdfd", [70, 70], 0.04, 4900)
    );
    assert!(
        details
            .iter()
            .any(|[name, value]| name == "PML" && value.starts_with("20 cells"))
    );
    // at each of the two wavelengths, the field's residual and S's reciprocity: a direct
    // solve's rounding, and a straight strip's S symmetric to it
    let errors: Vec<(Option<usize>, f64, &str, f64)> = events
        .iter()
        .filter_map(|e| match e {
            Event::SolveError {
                point,
                value,
                measure,
                error,
            } => Some((*point, *value, measure.as_str(), *error)),
            _ => None,
        })
        .collect();
    let places: Vec<(Option<usize>, f64, &str)> = errors.iter().map(|e| (e.0, e.1, e.2)).collect();
    assert_eq!(
        places,
        [
            (Some(0), 1.5, "linear residual"),
            (Some(0), 1.5, "reciprocity"),
            (Some(1), 1.6, "linear residual"),
            (Some(1), 1.6, "reciprocity")
        ]
    );
    for (_, _, measure, error) in &errors {
        assert!(*error >= 0.0 && *error < 1e-9, "{measure}: {error:e}");
    }
    // a job at one wavelength: the errors are the job's own
    let sweep = "[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n";
    let job = Job::parse(&FDFD.replace(sweep, "")).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let single: Vec<(Option<usize>, f64)> = replay(run.dir())
        .unwrap()
        .iter()
        .filter_map(|e| match e {
            Event::SolveError { point, value, .. } => Some((*point, *value)),
            _ => None,
        })
        .collect();
    assert_eq!(single, [(None, 1.55), (None, 1.55)]);
}

#[test]
fn a_job_names_its_direct_solver_and_the_record_says_which_solved() {
    use crate::backend::tests::Recording;
    let direct_of = |events: &[Event]| -> String {
        events
            .iter()
            .find_map(|e| match e {
                Event::Solver { details, .. } => details
                    .iter()
                    .find(|[name, _]| name == "direct solver")
                    .map(|[_, value]| value.clone()),
                _ => None,
            })
            .unwrap()
    };
    let version = env!("CARGO_PKG_VERSION");
    let with = |name: &str| format!("{FDFD}\n[solver]\ndirect = \"{name}\"\n");
    // none named: photonoxide's own, as its choice
    let plain = run_events("fdfd-direct-auto", FDFD);
    assert_eq!(
        direct_of(&plain),
        format!("photonoxide {version} (auto: no benchmark records on this machine)")
    );
    // photonoxide's by name: the same run
    let named = run_events("fdfd-direct-own", &with("photonoxide"));
    assert_eq!(direct_of(&named), format!("photonoxide {version}"));
    let s = |events: &[Event]| -> Vec<Event> {
        events
            .iter()
            .filter(|e| matches!(e, Event::SParameters { .. }))
            .cloned()
            .collect()
    };
    assert!(!s(&plain).is_empty());
    assert_eq!(s(&plain), s(&named));
    // a backend registered at run time: the job's solves go to it (two wavelengths: one
    // analysis, two factorizations), and give the same S-matrices
    let recording = Recording::register("recording-job");
    let routed = run_events("fdfd-direct-recording", &with("recording-job"));
    assert_eq!(direct_of(&routed), format!("recording-job {version}"));
    let [analyses, factorizations, solves, _] = recording.counts();
    assert_eq!((analyses, factorizations), (1, 2));
    assert!(solves >= 4, "{solves}");
    assert_eq!(s(&plain), s(&routed));
}

#[test]
fn a_job_that_names_no_backend_records_what_auto_chose_and_why() {
    use crate::backend::Form;
    use crate::backend::auto::{Measured, Outcome, with};
    let direct_of = |events: &[Event]| -> String {
        events
            .iter()
            .find_map(|e| match e {
                Event::Solver { details, .. } => details
                    .iter()
                    .find(|[name, _]| name == "direct solver")
                    .map(|[_, value]| value.clone()),
                _ => None,
            })
            .unwrap()
    };
    // the job's 70 × 70 cells: faer's LU measured 1.5 times as fast near that size
    let record = |backend: &str, seconds: f64| Measured {
        family: "fdfd2d".into(),
        form: Form::Symmetric,
        unknowns: 5_000,
        backend: backend.into(),
        threads: rayon::current_num_threads(),
        seconds,
        peak_bytes: None,
        outcome: Outcome::Accurate,
        unix_seconds: 1_791_763_200,
    };
    let records = vec![record("photonoxide", 3.0), record("faer", 2.0)];
    let chosen = with(records.clone(), None, || {
        run_events("fdfd-auto-chosen", FDFD)
    });
    let said = direct_of(&chosen);
    assert!(said.starts_with("faer "), "{said}");
    assert!(
        said.contains("(auto: 1.5 times faster than photonoxide's own at 5000 unknowns on")
            && said.ends_with("on this machine, measured 2026-10-12)"),
        "{said}"
    );
    // a job that pins its backend keeps it, whatever the records
    let pinned = format!("{FDFD}\n[solver]\ndirect = \"photonoxide\"\n");
    let pinned = with(records, None, || run_events("fdfd-auto-pinned", &pinned));
    assert_eq!(
        direct_of(&pinned),
        format!("photonoxide {}", env!("CARGO_PKG_VERSION"))
    );
    // the same device either way: the S-parameters agree to rounding
    let s = |events: &[Event]| -> Vec<f64> {
        events
            .iter()
            .filter(|e| matches!(e, Event::SParameters { .. }))
            .filter_map(|e| serde_json::to_value(e).ok())
            .flat_map(|v| numbers(&v))
            .collect()
    };
    let (a, b) = (s(&chosen), s(&pinned));
    assert!(
        !a.is_empty() && a.len() == b.len(),
        "{} {}",
        a.len(),
        b.len()
    );
    for (x, y) in a.iter().zip(&b) {
        assert!((x - y).abs() < 1e-9 * (1.0 + y.abs()), "{x} vs {y}");
    }
}

/// Every number in a JSON value, in order.
fn numbers(v: &serde_json::Value) -> Vec<f64> {
    match v {
        serde_json::Value::Number(n) => n.as_f64().into_iter().collect(),
        serde_json::Value::Array(a) => a.iter().flat_map(numbers).collect(),
        serde_json::Value::Object(o) => o.values().flat_map(numbers).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn the_check_and_the_run_refuse_a_direct_solver_that_cant_be_had() {
    let with = |text: &str, name: &str| format!("{text}\n[solver]\ndirect = \"{name}\"\n");
    crate::backend::register_unavailable("job-test-missing", "not installed").unwrap();
    let cases = [
        // nothing registered under the name
        (
            with(FDFD, "job-test-nothing"),
            "no direct solver is registered as \"job-test-nothing\"",
        ),
        // known, and not to be had: with the reason
        (
            with(FDFD, "job-test-missing"),
            "the direct solver \"job-test-missing\" isn't available: not installed",
        ),
        // a job without a direct solve
        (
            with(MODES, "photonoxide"),
            "[solver] direct = \"photonoxide\" is for fdfd jobs: a modes job has no direct \
                 solve to give a backend",
        ),
    ];
    for (k, (text, want)) in cases.iter().enumerate() {
        let job = Job::parse(text).unwrap();
        let checked = check(&job).unwrap_err().to_string();
        assert!(checked.contains(want), "{checked}");
        // the run says the same, before recording anything but its failure
        let root = temp(&format!("direct-refused-{k}"));
        let mut run = Run::create(&root.0, &job).unwrap();
        let ran = execute(&job, &mut run, &Stop::new(None))
            .unwrap_err()
            .to_string();
        assert_eq!(ran, checked);
        let events = replay::<Event>(run.dir()).unwrap();
        assert!(
            matches!(&events[..], [Event::Failed { error, .. }] if *error == ran),
            "{events:?}"
        );
    }
    // auto on any job is no choice at all
    check(&Job::parse(&with(MODES, "auto")).unwrap()).unwrap();
}

#[test]
fn the_check_refuses_what_the_run_would_before_it_solves() {
    // each of these passed the check and failed in the run: the check now says what the
    // run says, and the run still says it
    let port = "[[task.port]]\nx_um = -0.4\nside = \"left\"";
    let with_port = |to: &str| FDFD.replace(port, to);
    let cases = [
        (
            FDFD.replace("step_nm = 40.0", "step_nm = 40.0\npml_cells = 60"),
            "PMLs of 60 and 60 cells leave nothing of 70 cells",
        ),
        (
            FDFD.replace("step_nm = 40.0", "step_nm = 3000.0"),
            "x_um is 2.8 um across, less than 2 cells at a step of 3000 nm",
        ),
        (
            with_port("[[task.port]]\nx_um = -9.0\nside = \"left\""),
            "the port at x = -9 um is outside the window",
        ),
        (
            with_port("[[task.port]]\nx_um = -1.4\nside = \"left\""),
            "column 0 must be two cells clear of the PMLs (20 and 20 cells)",
        ),
        (
            with_port("[[task.port]]\nx_um = -0.4\nside = \"up\""),
            "unknown port side \"up\": left or right",
        ),
        (
            with_port("[[task.port]]\nx_um = -0.4\nside = \"left\"\ny_um = [0.0, 0.04]"),
            "its y_um [0, 0.04] covers 1 row of the window (y from -1.4 to 1.4 um, 70 rows); a \
                 port needs 3 rows or more",
        ),
        (
            with_port("[[task.port]]\nx_um = -0.4\nside = \"left\"\ny_um = [5.0, 6.0]"),
            "its y_um, 5 to 6, must lie inside the window's, -1.4 to 1.4",
        ),
        // a step a thousand times too small, in each kind: refused, not allocated
        (
            JOB.replace("step_nm = 50.0", "step_nm = 0.05"),
            "x_um at a step of 0.05 nm is 40000 cells, more than 10000",
        ),
        (
            JOB.replace("step_nm = 50.0", "step_nm = 50.0\nz_um = [0.0, 600.0]"),
            "z_um at a step of 50 nm is 12000 cells",
        ),
        (
            MODES.replace("step_nm = 25.0", "step_nm = 0.025"),
            "x_um at a step of 0.025 nm is 80000 cells",
        ),
        (
            FDFD.replace("step_nm = 40.0", "step_nm = 0.04"),
            "x_um at a step of 0.04 nm is 70000 cells",
        ),
        // a window as wide as a thousand of the step: a number, not three hundred digits
        (
            JOB.replace("x_um = [-1.0, 1.0]", "x_um = [-1e300, 1e300]"),
            "x_um at a step of 50 nm is 4.0e301 cells, more than 10000",
        ),
        // a step a thousand times too large: refused, not solved on a window of one cell
        (
            MODES.replace("step_nm = 25.0", "step_nm = 5000.0"),
            "x_um is 2 um across, less than 2 cells at a step of 5000 nm",
        ),
        (
            MODES.replace("modes = 2", "modes = 2\nz_um = [1.0, 1.01]"),
            "z_um is 0.01 um across, less than 2 cells at a step of 25 nm",
        ),
        (
            JOB.replace("step_nm = 50.0", "step_nm = 3000.0"),
            "x_um is 2 um across, less than 2 cells at a step of 3000 nm",
        ),
        // the height the job doesn't give, of a layer a metre thick
        (
            MODES.replace(
                "stack = \"soi_220\"",
                "stack = \"soi\"\ncore_nm = 1e9\nbottom_oxide_um = 2.0",
            ),
            "the height, 1 um below and above the layer as z_um isn't given, at a step of 25 \
                 nm is",
        ),
        (
            MODES.replace("modes = 2", "modes = 0"),
            "modes must be at least 1, got 0",
        ),
        // a side view cut outside the window, given or by default
        (
            JOB.replace("step_nm = 50.0", "step_nm = 50.0\nside_y_um = 5.0"),
            "the side view's cut, side_y_um = 5, is outside y_um [-1, 1]",
        ),
        (
            JOB.replace("y_um = [-1.0, 1.0]", "y_um = [2.0, 4.0]"),
            "the side view's cut, side_y_um = 0 by default, is outside y_um [2, 4]",
        ),
    ];
    for (text, says) in cases {
        let job = Job::parse(&text).unwrap();
        let refused = check(&job).unwrap_err().to_string();
        assert!(refused.contains(says), "{refused}");
        let root = temp("refused");
        let mut run = Run::create(&root.0, &job).unwrap();
        let failed = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
        assert_eq!(refused, failed.to_string());
    }
    // the largest grid allowed still passes: 10 000 cells across, and as many up (the
    // default height, 1 um below and above the layer, would be 11 100: its side view was
    // refused only when the run drew it)
    let job =
        Job::parse(&JOB.replace("step_nm = 50.0", "step_nm = 0.2\nz_um = [1.0, 3.0]")).unwrap();
    check(&job).unwrap();
    let job = Job::parse(&JOB.replace("step_nm = 50.0", "step_nm = 0.2")).unwrap();
    let refused = check(&job).unwrap_err().to_string();
    assert!(refused.contains("is 11100 cells"), "{refused}");
}

#[test]
fn the_check_refuses_jobs_that_ran_on_something_other_than_what_they_say() {
    // each of these ran, on a clipped port, a port counted twice, a closed box, or bare
    // cladding: the check and the run now refuse them, in the same words
    let left = "[[task.port]]\nx_um = -0.4\nside = \"left\"";
    let right = "[[task.port]]\nx_um = 0.4\nside = \"right\"";
    assert!(FDFD.contains(left) && FDFD.contains(right));
    let cases = [
        (
            FDFD.replace(left, &format!("{left}\ny_um = [0.0, 5.0]")),
            "the port at x = -0.4 um: its y_um, 0 to 5, must lie inside the window's, -1.4 to 1.4",
        ),
        (
            FDFD.replace(right, left),
            "ports 1 and 2 are the same (x = -0.4 um, left)",
        ),
        (
            FDFD.replace("step_nm = 40.0", "step_nm = 40.0\npml_cells = 0"),
            "pml_cells must be at least 1",
        ),
        (
            MODES.replace("step_nm = 25.0", "step_nm = 25.0\ncut_y_um = 50.0"),
            "the cross-section at y = 50 um meets no shape between x = -1 and 1 um",
        ),
        (
            MODES.split("[[task.rect]]").next().unwrap().to_owned(),
            "the cross-section at y = 0 um meets no shape",
        ),
        (
            MODES.replace("step_nm = 25.0", "step_nm = 25.0\nz_um = [5.0, 6.0]"),
            "z_um, 5 to 6, doesn't reach the layer Si (2 to 2.22 um)",
        ),
        (
            MODES_X.replace("step_nm = 25.0", "step_nm = 25.0\ncut_x_um = 50.0"),
            "the cross-section at x = 50 um meets no shape between y = -1 and 1 um",
        ),
    ];
    for (text, says) in cases {
        let job = Job::parse(&text).unwrap();
        let refused = check(&job).unwrap_err().to_string();
        assert!(refused.contains(says), "{refused}");
        let root = temp("guideless");
        let mut run = Run::create(&root.0, &job).unwrap();
        let failed = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
        assert_eq!(refused, failed.to_string());
    }
    // what is still a job: a port's window that is the window's own, a z window that
    // holds the layer and nothing else, a strip narrower than a cell of the sampling
    for text in [
        FDFD.replace(left, &format!("{left}\ny_um = [-1.4, 1.4]")),
        MODES.replace("step_nm = 25.0", "step_nm = 25.0\nz_um = [1.9, 2.3]"),
        MODES.replace("size_um = [0.5, 10.0]", "size_um = [0.01, 10.0]"),
    ] {
        check(&Job::parse(&text).unwrap()).unwrap();
    }
}

#[test]
fn a_modes_job_asks_for_at_most_50_modes() {
    // hundreds ran minutes past a job's time limit: refused by the check and the run, in
    // the same words (none is refused too, by the_check_refuses_what_the_run_would…)
    for (modes, says) in [
        ("modes = 51", "modes must be at most 50, got 51"),
        ("modes = 500", "modes must be at most 50, got 500"),
    ] {
        let job = Job::parse(&MODES.replace("modes = 2", modes)).unwrap();
        let refused = check(&job).unwrap_err().to_string();
        assert!(refused.contains(says), "{refused}");
        let root = temp("many-modes");
        let mut run = Run::create(&root.0, &job).unwrap();
        let failed = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
        assert_eq!(refused, failed.to_string());
    }
    for modes in ["modes = 1", "modes = 50"] {
        check(&Job::parse(&MODES.replace("modes = 2", modes)).unwrap()).unwrap();
    }
}

#[test]
fn a_threshold_is_refused_by_the_check_as_by_the_run() {
    // a count and a threshold both, a threshold that isn't a positive index, and one at or
    // above the highest index the cross-section holds (silicon's 3.48 at 1.55 µm): refused
    // by the check and the run in the same words
    for (modes, says) in [
        (
            "modes = 2\nmodes_above = 1.444",
            "give a count or a threshold, not both",
        ),
        ("modes_above = 0.0", "modes_above must be a positive"),
        ("modes_above = -1.5", "modes_above must be a positive"),
        ("modes_above = nan", "modes_above must be a positive"),
        ("modes_above = 3.6", "no mode lies above it"),
    ] {
        let job = Job::parse(&MODES.replace("modes = 2", modes)).unwrap();
        let refused = check(&job).unwrap_err().to_string();
        assert!(refused.contains(says), "{modes}: {refused}");
        let root = temp("threshold");
        let mut run = Run::create(&root.0, &job).unwrap();
        let failed = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
        assert_eq!(refused, failed.to_string());
    }
    // a threshold the oxide's: accepted
    check(&Job::parse(&MODES.replace("modes = 2", "modes_above = 1.444")).unwrap()).unwrap();
}

#[test]
fn a_threshold_finds_every_guided_mode_as_a_count_does() {
    // the strip's modes above the oxide, no count given: its TE-like and TM-like modes, and
    // a third the closed window guides at 1.498, the three a count of three finds, the same
    // to the solvers' tolerances
    let above = run_events(
        "modes-above",
        &MODES.replace("modes = 2", "modes_above = 1.444"),
    );
    let three = run_events(
        "modes-above-three",
        &MODES.replace("modes = 2", "modes = 3"),
    );
    let indices = |events: &[Event]| -> Vec<f64> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::Mode {
                    effective_index, ..
                } => Some(effective_index[0]),
                _ => None,
            })
            .collect()
    };
    let (found, want) = (indices(&above), indices(&three));
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(found[2] > 1.444 && found[2] < 1.5, "{found:?}");
    for (a, b) in found.iter().zip(&want) {
        assert!((a - b).abs() < 1e-8, "{a} vs {b}");
    }
    // the record says how the modes were asked for and found
    let Some(Event::Solver { details, .. }) =
        above.iter().find(|e| matches!(e, Event::Solver { .. }))
    else {
        panic!("no solver event")
    };
    assert!(details[0][1].contains("above 1.444"), "{details:?}");
    assert!(details[1][1].contains("contour integrals"), "{details:?}");
}

#[test]
fn a_threshold_heeds_the_stop() {
    // stopped before the search: the run records its scene and stops, with no modes
    let job = Job::parse(&MODES.replace("modes = 2", "modes_above = 1.444")).unwrap();
    let root = temp("threshold-stop");
    let mut run = Run::create(&root.0, &job).unwrap();
    let stop = Stop::new(None);
    stop.request();
    execute(&job, &mut run, &stop).unwrap();
    let events = replay(run.dir()).unwrap();
    assert!(!events.iter().any(|e| matches!(e, Event::Mode { .. })));
}

#[test]
fn bad_tasks_are_errors() {
    let root = temp("bad");
    for (text, says) in [
        (
            JOB.replace("kind = \"structure\"", "kind = \"fem\""),
            "unknown kind",
        ),
        (
            JOB.replace("stack = \"soi_220\"", "stack = \"glass\""),
            "unknown stack",
        ),
        (
            JOB.replace("stack = \"soi_220\"", "stack = \"soi\""),
            "core_nm",
        ),
        (JOB.replace("step_nm", "stepsize"), "stepsize"),
        (
            JOB.replace("layer = \"Si\"\ncenter", "layer = \"M1\"\ncenter"),
            "M1",
        ),
        // windows that run backwards or are empty, a step that isn't positive
        (
            JOB.replace("x_um = [-1.0, 1.0]", "x_um = [1.0, -1.0]"),
            "x_um must go from a smaller to a larger number",
        ),
        (
            JOB.replace("y_um = [-1.0, 1.0]", "y_um = [1.0, 1.0]"),
            "y_um must go",
        ),
        (
            JOB.replace("step_nm = 50.0", "step_nm = 50.0\nz_um = [3.0, 1.0]"),
            "z_um must go",
        ),
        (
            JOB.replace("step_nm = 50.0", "step_nm = 0.0"),
            "step_nm must be positive",
        ),
        (
            MODES.replace("x_um = [-1.0, 1.0]", "x_um = [1.0, -1.0]"),
            "x_um must go",
        ),
        (
            MODES.replace("step_nm = 25.0", "step_nm = -25.0"),
            "step_nm must be positive",
        ),
        (
            MODES.replace("step_nm = 25.0", "step_nm = 25.0\ncut_y_um = nan"),
            "cut_y_um must be a number",
        ),
        // the propagation, and the other propagation's window or cut
        (
            MODES_X.replace("propagation = \"x\"", "propagation = \"z\""),
            "unknown propagation \"z\"",
        ),
        (
            MODES_X.replace("step_nm = 25.0", "step_nm = 25.0\nx_um = [-1.0, 1.0]"),
            "x_um is for a modes job along y",
        ),
        (
            MODES_X.replace("step_nm = 25.0", "step_nm = 25.0\ncut_y_um = 0.5"),
            "cut_y_um is for a modes job along y",
        ),
        (
            MODES.replace("step_nm = 25.0", "step_nm = 25.0\ncut_x_um = 0.5"),
            "cut_x_um is for a modes job along x",
        ),
        (
            MODES_X.replace("y_um = [-1.0, 1.0]\n", ""),
            "a modes job along x needs y_um",
        ),
        (
            MODES_X.replace("y_um = [-1.0, 1.0]", "y_um = [1.0, -1.0]"),
            "y_um must go",
        ),
        (
            MODES_X.replace("step_nm = 25.0", "step_nm = 25.0\ncut_x_um = inf"),
            "cut_x_um must be a number",
        ),
        // sweeps the run can't take
        (
            format!(
                "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.55\nto = 1.55\npoints = 3\n"
            ),
            "must differ",
        ),
        (
            format!(
                "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 1\n"
            ),
            "at least 2 points",
        ),
        (
            format!(
                "{MODES}\n[task.sweep]\nparameter = \"height\"\nfrom = 0.2\nto = 0.3\npoints = 3\n"
            ),
            "unknown sweep parameter",
        ),
        (
            format!(
                "{MODES}\n[task.sweep]\nparameter = \"width\"\nfrom = 0.4\nto = 0.6\npoints = 3\nrect = 2\n"
            ),
            "the width sweep's rect 2 isn't there: rect counts from 0, so it is rectangle 3 of \
                 the job's 1",
        ),
        (
            format!(
                "{MODES}\n[task.sweep]\nparameter = \"width\"\nfrom = -0.1\nto = 0.6\npoints = 3\n"
            ),
            "widths must be positive",
        ),
        (
            FDFD.replace("y_um = [-1.4, 1.4]", "y_um = [1.4, -1.4]"),
            "y_um must go",
        ),
        (FDFD.replace("to = 1.6", "to = 1.5"), "must differ"),
        (
            FDFD.replace("x_um = -0.4", "x_um = nan"),
            "a port's x_um must be a number",
        ),
    ] {
        let job = Job::parse(&text).unwrap();
        // found by the check, without running, as by the run
        let e = check(&job).unwrap_err();
        assert!(e.to_string().contains(says), "check, {says}: {e}");
        let mut run = Run::create(&root.0, &job).unwrap();
        let e = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
        assert!(e.to_string().contains(says), "{says}: {e}");
    }
}

#[test]
fn the_repositorys_jobs_pass_the_check() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
    let mut count = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|x| x == "yaml") {
            check(&Job::load(&path).unwrap()).unwrap_or_else(|e| panic!("{path:?}: {e}"));
            count += 1;
        }
    }
    assert!(count >= 4);
    assert!(check(&Job::parse(FDFD).unwrap()).is_ok());
    let portless = FDFD.split("[[task.port]]").next().unwrap();
    let e = check(&Job::parse(portless).unwrap()).unwrap_err();
    assert!(e.to_string().contains("port"), "{e}");
}

/// The job files the README, the design page and the fabrication write-up show, their ```yaml
/// blocks: each passes the check, as the repository's jobs do, and each is one of jobs/ as
/// written, so that the `photonoxide run jobs/<file>.yaml` beside it runs what the page shows.
#[test]
fn the_readme_and_design_pages_jobs_are_jobs_that_pass_the_check() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let text =
        |path: &std::path::Path| std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
    let jobs: Vec<String> = std::fs::read_dir(root.join("jobs"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|x| x == "yaml"))
        .map(|path| text(&path))
        .collect();
    let mut count = 0;
    for page in ["README.md", "docs/design.md", "docs/methods/fabrication.md"] {
        let page_text = text(&root.join(page));
        let mut lines = page_text.lines();
        while lines.any(|l| l == "```yaml") {
            let yaml: String = lines
                .by_ref()
                .take_while(|l| *l != "```")
                .map(|l| format!("{l}\n"))
                .collect();
            let job = Job::parse_as(&yaml, crate::run::Format::Yaml)
                .unwrap_or_else(|e| panic!("{page}: {e}\n{yaml}"));
            check(&job).unwrap_or_else(|e| panic!("{page}: {e}\n{yaml}"));
            assert!(
                jobs.contains(&yaml),
                "{page}: a job that isn't one of jobs/ as written:\n{yaml}"
            );
            count += 1;
        }
    }
    assert!(count >= 5, "{count} job files");
}

const FDFD: &str = r#"
name = "strip-fdfd"

[task]
kind = "fdfd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.4, 1.4]
y_um = [-1.4, 1.4]
step_nm = 40.0

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [4.0, 0.5]

[[task.port]]
x_um = -0.4
side = "left"

[[task.port]]
x_um = 0.4
side = "right"

[task.sweep]
parameter = "wavelength"
from = 1.5
to = 1.6
points = 2
"#;

#[test]
fn an_fdfd_job_records_s_parameters_and_a_field() {
    let root = temp("fdfd");
    let job = Job::parse(FDFD).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = found(&run);
    // started, the scene, the sweep, S at 1.5 um, the field and the point's, S at 1.6 um,
    // its point's field, finished
    assert_eq!(events.len(), 9, "{events:?}");
    // the scene and the pictures leave out the PMLs (20 cells of 40 nm) and the two cells
    // beside them: the 2.8 µm window's middle 1.04 µm, 26 cells
    let Event::Scene { x_um, y_um, .. } = &events[1] else {
        panic!("{:?}", events[1])
    };
    for (shown, edge) in [
        (x_um[0], -0.52),
        (x_um[1], 0.52),
        (y_um[0], -0.52),
        (y_um[1], 0.52),
    ] {
        assert!((shown - edge).abs() < 1e-12, "{x_um:?} {y_um:?}");
    }
    assert!(matches!(
        &events[2],
        Event::Sweep { parameter, from, to, points: 2 }
            if parameter == "wavelength" && (*from, *to) == (1.5, 1.6)
    ));
    let Event::SParameters {
        wavelength_um,
        ports,
        effective_indices,
        s,
    } = &events[3]
    else {
        panic!("{:?}", events[3])
    };
    assert_eq!(*wavelength_um, 1.5);
    assert_eq!(ports.len(), 2);
    // a straight strip: everything through, in a mode between the oxide's and the slab's
    // index (the slab's TE mode, 2.85, in oxide, 1.444)
    let through = s[1][0][0].hypot(s[1][0][1]);
    assert!((through - 1.0).abs() < 1e-6, "{s:?}");
    assert!(
        effective_indices.iter().all(|&n| n > 1.444 && n < 2.85),
        "{effective_indices:?}"
    );
    let Event::Field { intensity, .. } = &events[4] else {
        panic!("{:?}", events[4])
    };
    assert_eq!((intensity.nx, intensity.ny), (26, 26));
    assert!((intensity.x0 - x_um[0]).abs() < 1e-12 && (intensity.x1 - x_um[1]).abs() < 1e-12);
    assert!(
        matches!(&events[6], Event::SParameters { wavelength_um, .. } if *wavelength_um == 1.6)
    );
    // each point's field after its S: the job's own on 2 x 2 blocks, its peak 1, to three
    // decimals, over the same window
    for (k, at) in [(0, 5), (1, 7)] {
        let Event::SweepField {
            point,
            value,
            wavelength_um,
            intensity: coarse,
            ..
        } = &events[at]
        else {
            panic!("{:?}", events[at])
        };
        assert_eq!((*point, *value, *wavelength_um), (k, [1.5, 1.6][k], *value));
        assert_eq!((coarse.nx, coarse.ny), (13, 13));
        assert_eq!((coarse.x0, coarse.y0), (intensity.x0, intensity.y0));
        assert!(
            (coarse.x1 - intensity.x1).abs() < 1e-12 && (coarse.y1 - intensity.y1).abs() < 1e-12
        );
        assert_eq!(coarse.range().1, 1.0);
        assert!(
            coarse
                .values
                .iter()
                .all(|v| (v * 1000.0 - (v * 1000.0).round()).abs() < 1e-3)
        );
        // the first point is the wavelength the job's own field is at: its blocks' means
        if k == 0 {
            let mean = |i: usize, j: usize| {
                (0..2)
                    .flat_map(|b| (0..2).map(move |a| (a, b)))
                    .map(|(a, b)| f64::from(intensity.at(2 * i + a, 2 * j + b)))
                    .sum::<f64>()
                    / 4.0
            };
            let peak = (0..13)
                .flat_map(|j| (0..13).map(move |i| (i, j)))
                .map(|(i, j)| mean(i, j))
                .fold(0.0, f64::max);
            for (i, j) in [(6, 6), (0, 6), (12, 7), (6, 1)] {
                let expected = mean(i, j) / peak;
                assert!(
                    (f64::from(coarse.at(i, j)) - expected).abs() < 1e-3,
                    "{i} {j}"
                );
            }
        }
    }
}

#[test]
fn an_fdfd_jobs_field_starts_where_its_guide_comes_in() {
    // the strip crosses the whole window, its port 1 at x = −0.4 (column 25); the PML is the
    // first 20 columns. The field is launched at column 22, the picture's first, so from
    // the picture's left edge to the port the guide is as bright as beyond the port (a
    // straight, lossless guide), and as bright at the picture's right edge: the light
    // starts where the picture does and ends where it does
    let root = temp("fdfd-launch");
    let sweep = "[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n";
    let single = FDFD.replace(sweep, "");
    let field_of = |text: &str| {
        let job = Job::parse(text).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        execute(&job, &mut run, &Stop::new(None)).unwrap();
        replay(run.dir())
            .unwrap()
            .into_iter()
            .find_map(|e| match e {
                Event::Field { intensity, .. } => Some(intensity),
                _ => None,
            })
            .unwrap()
    };
    let field = field_of(&single);
    // along the guide's axis, y = 0
    assert_eq!((field.nx, field.ny), (26, 26));
    // along the guide's axis, y = 0: row 35 of the grid, 13 of the picture
    let on_axis = |i: usize| f64::from(field.at(i, 13));
    let beyond = on_axis(18);
    assert!(beyond > 0.5, "{beyond}");
    for i in [0, 1, 2, 25] {
        assert!(
            (on_axis(i) - beyond).abs() < 0.05 * beyond,
            "column {i}: {} against {beyond}",
            on_axis(i)
        );
    }
    // a strip that starts just before the port: at column 22 there is no guide, so the
    // field is launched at the port, and before it only what the device reflects shows
    let short = single.replace(
        "center_um = [0.0, 0.0]\nsize_um = [4.0, 0.5]",
        "center_um = [1.5, 0.0]\nsize_um = [3.9, 0.5]",
    );
    assert_ne!(short, single);
    let field = field_of(&short);
    assert_eq!(field.range().1, 1.0);
    assert!(f64::from(field.at(18, 13)) > 0.5);
    assert!(f64::from(field.at(1, 13)) < 0.05, "{}", field.at(1, 13));
}

#[test]
fn an_fdfd_job_at_one_wavelength_records_no_sweep() {
    let root = temp("fdfd-single");
    let sweep = "[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 2\n";
    let job = Job::parse(&FDFD.replace(sweep, "")).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = found(&run);
    // started, the scene, S, the field, finished
    assert_eq!(events.len(), 5, "{events:?}");
    assert!(matches!(&events[3], Event::Field { .. }));
}

#[test]
fn a_long_sweeps_fields_are_on_larger_blocks() {
    // the ring job's grid: 2 x 2 blocks for its 201 points, larger for ten times as many
    assert_eq!(fdfd::sweep_block(360, 256, 201), 2);
    let b = fdfd::sweep_block(360, 256, 2010);
    assert!(b > 2 && (360 / b) * (256 / b) * 2010 <= 5_000_000, "{b}");
    assert!((360 / (b - 1)) * (256 / (b - 1)) * 2010 > 5_000_000, "{b}");
    // never more than the grid
    assert_eq!(fdfd::sweep_block(3, 1, 10_000_000), 1);
}

#[test]
fn an_fdfd_job_records_the_field_at_the_wavelength_asked() {
    let root = temp("fdfd-field");
    let text = FDFD.replace(
        "step_nm = 40.0",
        "step_nm = 40.0
field_um = 1.58",
    );
    let job = Job::parse(&text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = found(&run);
    // the swept wavelength nearest 1.58 is 1.6, the second: its field follows its S
    let fields: Vec<f64> = events
        .iter()
        .filter_map(|e| match e {
            Event::Field { wavelength_um, .. } => Some(*wavelength_um),
            _ => None,
        })
        .collect();
    assert_eq!(fields, [1.6]);
    assert!(matches!(&events[6], Event::Field { .. }), "{:?}", events[6]);
}

#[test]
fn an_fdfd_job_without_ports_or_with_a_port_in_the_pml_is_an_error() {
    let ports = "[[task.port]]\nx_um = -0.4\nside = \"left\"\n\n[[task.port]]\nx_um = 0.4\nside = \"right\"";
    let in_pml = "[[task.port]]\nx_um = -1.35\nside = \"left\"\n\n[[task.port]]\nx_um = 0.4\nside = \"right\"";
    for (to, error) in [("", "at least one"), (in_pml, "clear of the PMLs")] {
        let root = temp("fdfd-bad");
        let job = Job::parse(&FDFD.replace(ports, to)).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        let e = execute(&job, &mut run, &Stop::new(None)).unwrap_err();
        assert!(e.to_string().contains(error), "{e}");
    }
}

/// The job's events on a pool of `threads`, without the time it took.
fn events_on(threads: usize, text: &str, tag: &str) -> Vec<Event> {
    let root = temp(&format!("{tag}-{threads}"));
    let job = Job::parse(text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    pool.install(|| execute(&job, &mut run, &Stop::new(None)))
        .unwrap();
    replay(run.dir())
        .unwrap()
        .into_iter()
        .filter(|e| !matches!(e, Event::Finished { .. }))
        .collect()
}

#[test]
fn a_sweep_records_the_same_events_on_any_number_of_threads() {
    // one thread solves the points one after another, as a loop over them does; four solve
    // them four at a time, and record them in the same order, to the last bit
    let modes = format!(
        "{MODES}\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 5\n"
    );
    let fdfd = FDFD.replace("points = 2", "points = 5");
    assert_ne!(fdfd, FDFD);
    for (text, tag) in [
        (modes.as_str(), "sweep-modes"),
        (fdfd.as_str(), "sweep-fdfd"),
    ] {
        let one = events_on(1, text, tag);
        let points = one
            .iter()
            .filter(|e| matches!(e, Event::SweepPoint { .. } | Event::SParameters { .. }))
            .count();
        assert_eq!(points, 5, "{tag}");
        for threads in [4, 20] {
            assert_eq!(events_on(threads, text, tag), one, "{tag} on {threads}");
        }
    }
    // and the FDFD sweep's S-parameters called directly, outside a run
    let job = Job::parse(&fdfd).unwrap();
    let on = |threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| fdfd_s_parameters(&job, None))
            .unwrap()
    };
    let one = on(1);
    assert_eq!(one.s.len(), 5);
    assert_eq!(on(4), one);
    assert_eq!(on(20), one);
}
