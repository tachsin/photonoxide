//! The `"eme"` job in 3D (`[task.plane]`): a run's S-parameters are [`crate::eme::Eme3d`]'s on
//! the same sections, to the bit, whether given as strips or cut from a `[device]`; and what
//! [`super::check`] refuses.

use super::{Event, check, execute};
use crate::device::Cut;
use crate::eme::three_checks::{centred, strip};
use crate::eme::{Eme3d, PlaneGrid, Section3d, Wall, Walls};
use crate::run::{Job, Run, Stop};
use crate::units::{Length, Wavelength};

/// A 200 nm silicon strip widening from 400 to 700 nm: 0.3 µm of linear taper in three steps, in
/// a closed box 1.2 × 0.8 µm on a 50 nm grid, 40 modes.
const STRIPS: &str = r#"
name = "taper-3d"
timeout_minutes = 5

[task]
kind = "eme"
wavelength_um = 1.55
modes = 40
cladding = 1.444
core = 3.473

[task.plane]
step_um = [0.05, 0.05]
cells = [24, 16]
height_um = 0.2

[[task.section]]
length_um = 0.0
core_um = 0.4

[[task.section]]
length_um = 0.3
core_um = [0.4, 0.7]
steps = 3

[[task.section]]
length_um = 0.0
core_um = 0.7
"#;

fn events(job: &Job) -> Vec<Event> {
    let mut run = Run::in_memory();
    execute(job, &mut run, &Stop::new(None)).unwrap();
    run.lines()
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// A run's S-parameters: its ports and S.
type Ports = (Vec<String>, Vec<Vec<[f64; 2]>>);

fn s_parameters(events: &[Event]) -> Vec<Ports> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::SParameters { ports, s, .. } => Some((ports.clone(), s.clone())),
            _ => None,
        })
        .collect()
}

fn lam() -> Wavelength {
    Wavelength::um(1.55).unwrap()
}

/// The strip taper's sections on `grid`, the library's way.
fn taper_sections(grid: PlaneGrid, steps: usize) -> Vec<Section3d> {
    let mut sections = vec![Section3d::new(strip(grid, 0.4, 0.2), Length::ZERO)];
    sections.extend(
        Section3d::sampled(Length::um(0.3), steps, |z| {
            Ok(strip(grid, 0.4 + (0.7 - 0.4) * (z.to_um() / 0.3), 0.2))
        })
        .unwrap(),
    );
    sections.push(Section3d::new(strip(grid, 0.7, 0.2), Length::ZERO));
    sections
}

#[test]
fn a_3d_runs_s_parameters_are_the_librarys_to_the_bit() {
    let job = Job::parse(STRIPS).unwrap();
    check(&job).unwrap();
    let got = s_parameters(&events(&job));
    assert_eq!(got.len(), 1);
    let (ports, s) = &got[0];
    let lib = Eme3d::new(lam(), 40)
        .unwrap()
        .s_matrix(&taper_sections(centred(24, 16, 0.05, 0.05), 3))
        .unwrap();
    let n1 = first_end(ports);
    assert!(n1 >= 1 && ports.len() > n1, "{ports:?}");
    assert_eq!(ports[0], "1 m0");
    let at = |q: usize, p: usize| match (q < n1, p < n1) {
        (true, true) => lib.s11[q][p],
        (true, false) => lib.s12[q][p - n1],
        (false, true) => lib.s21[q - n1][p],
        (false, false) => lib.s22[q - n1][p - n1],
    };
    for (q, row) in s.iter().enumerate() {
        for (p, v) in row.iter().enumerate() {
            assert_eq!([at(q, p).re, at(q, p).im], *v, "S[{q}][{p}]");
        }
    }
}

#[test]
fn a_quarter_is_the_library_on_its_quarter() {
    let quarter = STRIPS
        .replace(
            "cells = [24, 16]",
            "cells = [12, 8]\nsymmetry = \"quarter\"",
        )
        .replace("modes = 40", "modes = 20");
    let got = s_parameters(&events(&Job::parse(&quarter).unwrap()));
    let (ports, s) = &got[0];
    let n1 = first_end(ports);
    let grid = PlaneGrid::new(12, 8, (0.05, 0.05), (0.0, 0.0)).with_walls(Walls {
        south: Wall::Magnetic,
        ..Walls::default()
    });
    let lib = Eme3d::new(lam(), 20)
        .unwrap()
        .s_matrix(&taper_sections(grid, 3))
        .unwrap();
    assert_eq!([lib.s11[0][0].re, lib.s11[0][0].im], s[0][0]);
    assert_eq!([lib.s21[0][0].re, lib.s21[0][0].im], s[n1][0]);
}

/// The modes recorded at the first end: the ports named "1 ...".
fn first_end(ports: &[String]) -> usize {
    ports.iter().filter(|p| p.starts_with("1 ")).count()
}

/// A 1 × 2 MMI 1.6 µm wide on the SOI stack, cut on its half, every mode of the plane.
const DEVICE: &str = r#"
name = "mmi-3d-eme"
timeout_minutes = 5

[device]
kind = "MMI 1x2"
fidelity = "3D"

[device.parameters]
width = "1.6"
length = "2.2"
access_width = "0.5"
port_width = "0.5"

[task]
kind = "eme"
wavelength_um = 1.55
modes = 100000

[task.plane]
step_um = [0.05, 0.055]
margins_um = [0.4, 0.33]
symmetry = "half"
"#;

#[test]
fn a_devices_sections_are_its_cut_and_the_run_is_the_library_to_the_bit() {
    let job = Job::parse(DEVICE).unwrap();
    check(&job).unwrap();
    let events = events(&job);
    let got = s_parameters(&events);
    let (ports, s) = &got[0];
    let n1 = first_end(ports);
    let (device, design) = super::device::spec_of(&job).unwrap().build().unwrap();
    let geometry = device.geometry(design.values()).unwrap();
    let cut = Cut::new(0.05, 0.055, 0.1)
        .and_then(|c| c.with_margins(0.4, 0.33))
        .unwrap()
        .mirrored()
        .geometry(&geometry, lam())
        .unwrap();
    // the input guide, the body, the outputs
    assert_eq!(cut.sections.len(), 3);
    let lib = Eme3d::new(lam(), cut.grid.unknowns())
        .unwrap()
        .s_matrix(&cut.sections)
        .unwrap();
    assert_eq!([lib.s11[0][0].re, lib.s11[0][0].im], s[0][0]);
    assert_eq!([lib.s21[0][0].re, lib.s21[0][0].im], s[n1][0]);
    // the device's scene first, then the solver's settings: every mode of the plane
    let solver = events.iter().find_map(|e| match e {
        Event::Solver { details, .. } => Some(details.clone()),
        _ => None,
    });
    let details = solver.unwrap();
    assert!(
        details
            .iter()
            .any(|[k, v]| k == "modes" && v.contains("every one")),
        "{details:?}"
    );
}

#[test]
fn the_built_in_3d_job_is_the_validation_cases_taper_to_the_bit() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
    let job = Job::load(&dir.join("eme-taper-3d.yaml")).unwrap();
    let got = s_parameters(&events(&job));
    let (ports, s) = &got[0];
    let n1 = first_end(ports);
    let t = s[n1][0][0].powi(2) + s[n1][0][1].powi(2);
    // eme/3d-taper-fdfd's |S21|^2, with every mode, as the job's comment quotes it
    let (r, case) = crate::eme::three_checks::taper_eme(728);
    assert_eq!(t, case);
    assert_eq!(s[0][0][0].powi(2) + s[0][0][1].powi(2), r);
    assert!((t - 0.996238).abs() < 5e-7, "{t}");
}

#[test]
fn the_built_in_mmi_job_gives_the_number_its_comment_quotes() {
    // the length the validation case's MMI takes, as the job's comment says
    assert_eq!(
        format!("{:.1}", crate::device::cut_checks::mmi().1),
        LENGTH,
        "the MMI's length"
    );
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
    let job = Job::load(&dir.join("eme-mmi-3d.yaml")).unwrap();
    let got = s_parameters(&events(&job));
    let (ports, s) = &got[0];
    let n1 = first_end(ports);
    let t = s[n1][0][0].powi(2) + s[n1][0][1].powi(2);
    assert!((t - TRANSMISSION).abs() < 5e-7, "{t} {ports:?}");
}

/// The built-in MMI job's length and |S₂₁|², as its comment quotes them.
const LENGTH: &str = "2.4";
const TRANSMISSION: f64 = 0.906973;

#[test]
fn check_refuses_what_a_3d_run_would() {
    let refused = |text: &str, says: &str| {
        let err = check(&Job::parse(text).unwrap()).unwrap_err().to_string();
        assert!(err.contains(says), "{err}");
    };
    let swap = |from: &str, to: &str| {
        let text = STRIPS.replace(from, to);
        assert_ne!(text, STRIPS, "{from}");
        text
    };
    refused(
        &swap("modes = 40", "modes = 40\npolarization = \"te\""),
        "polarization is a 2D job's",
    );
    refused(
        &swap("modes = 40", "modes = 40\nwindow_um = 1.2"),
        "window_um is a 2D job's",
    );
    refused(
        &(STRIPS.to_owned() + "\n[task.adaptive]\ntolerance = 1e-3\n"),
        "equal steps",
    );
    refused(
        &swap(
            "core_um = 0.7\n",
            "layers = [{ medium = 1.444, thickness_um = 1.2 }]\n",
        ),
        "layers are a 2D",
    );
    refused(
        &swap("core_um = 0.7\n", "core_um = 1.3\n"),
        "inside the window",
    );
    refused(&swap("height_um = 0.2", "height_um = 0.9"), "height_um");
    refused(&swap("cells = [24, 16]", ""), "cells");
    refused(
        &swap(
            "height_um = 0.2",
            "height_um = 0.2\nsymmetry = \"diagonal\"",
        ),
        "symmetry",
    );
    refused(
        &swap("height_um = 0.2", "height_um = 0.2\nalong_um = 0.1"),
        "a [device]'s cut",
    );
    // beside a device: no sections of its own, no quarter, no strips' fields
    refused(
        &(DEVICE.to_owned() + "\n[[task.section]]\nlength_um = 0.0\ncore_um = 0.5\n"),
        "cut from its geometry",
    );
    refused(
        &DEVICE.replace("symmetry = \"half\"", "symmetry = \"quarter\""),
        "not a quarter",
    );
    refused(
        &DEVICE.replace("symmetry = \"half\"", "cells = [10, 10]"),
        "not cells",
    );
    // a device's 2D eme job: it is cut into planes
    refused(
        &DEVICE.replace("[task.plane]\nstep_um = [0.05, 0.055]\nmargins_um = [0.4, 0.33]\nsymmetry = \"half\"\n", ""),
        "give [task.plane]",
    );
}
