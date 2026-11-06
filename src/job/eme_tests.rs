//! The `"eme"` job ([`super::eme`]): what [`super::check`] refuses, and that a run's
//! S-parameters are the library's on the same sections, to the bit.

use num_complex::Complex64 as c64;

use super::{Event, check, execute};
use crate::eme::{Eme, Section, Slice};
use crate::mode::Polarization;
use crate::run::{Job, Run, Stop};
use crate::units::{Length, Wavelength};

/// A 220 to 300 nm slab taper in four equal steps, 12 modes, between electric walls 3 µm apart.
const TAPER: &str = r#"
name = "taper"
timeout_minutes = 5

[task]
kind = "eme"
wavelength_um = 1.55
modes = 12
window_um = 3.0
cladding = 1.444
core = 3.473

[[task.section]]
length_um = 0.0
core_um = 0.22

[[task.section]]
length_um = 0.4
core_um = [0.22, 0.30]
steps = 4

[[task.section]]
length_um = 0.0
core_um = 0.30
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

fn slab(t: f64) -> Slice {
    Slice::slab(
        c64::new(3.473, 0.0),
        Length::um(t),
        c64::new(1.444, 0.0),
        Length::um(3.0),
    )
    .unwrap()
}

#[test]
fn a_runs_s_parameters_are_the_librarys_to_the_bit() {
    let job = Job::parse(TAPER).unwrap();
    check(&job).unwrap();
    let got = s_parameters(&events(&job));
    assert_eq!(got.len(), 1);
    let (ports, s) = &got[0];
    // the 220 nm slab guides TE0 alone, the 300 nm one TE0 and TE1 (cut off near 255 nm)
    assert_eq!(ports, &["1 TE0", "2 TE0", "2 TE1"]);
    let mut sections = vec![Section::new(slab(0.22), Length::ZERO)];
    sections.extend(
        Section::sampled(Length::um(0.4), 4, |z| {
            Ok(slab(0.22 + (0.30 - 0.22) * (z.to_um() / 0.4)))
        })
        .unwrap(),
    );
    sections.push(Section::new(slab(0.30), Length::ZERO));
    let lib = Eme::new(Polarization::Te, Wavelength::um(1.55).unwrap(), 12)
        .unwrap()
        .s_matrix(&sections)
        .unwrap();
    let pairs = [
        (lib.s11[0][0], s[0][0]),
        (lib.s12[0][0], s[0][1]),
        (lib.s12[0][1], s[0][2]),
        (lib.s21[0][0], s[1][0]),
        (lib.s21[1][0], s[2][0]),
        (lib.s22[0][0], s[1][1]),
        (lib.s22[1][1], s[2][2]),
        (lib.s22[0][1], s[1][2]),
    ];
    for (want, got) in pairs {
        assert_eq!([want.re, want.im], got);
    }
}

#[test]
fn a_symmetric_job_is_the_half_with_its_plane_of_symmetry() {
    let half = TAPER
        .replace("window_um = 3.0", "window_um = 1.5\nsymmetric = true")
        .replace("modes = 12", "modes = 6")
        + "\n[task.pml]\nthickness_um = 4.0\nstrength = 0.1\n";
    let got = s_parameters(&events(&Job::parse(&half).unwrap()));
    let (ports, s) = &got[0];
    // the even modes only: TE1 is odd
    assert_eq!(ports, &["1 TE0", "2 TE0"]);
    let half_slab = |t: f64| {
        Slice::new(&[
            (c64::new(3.473, 0.0), Length::um(0.5 * t)),
            (c64::new(1.444, 0.0), Length::um(1.5 - 0.5 * t)),
        ])
        .unwrap()
    };
    let mut sections = vec![Section::new(half_slab(0.22), Length::ZERO)];
    sections.extend(
        Section::sampled(Length::um(0.4), 4, |z| {
            Ok(half_slab(0.22 + (0.30 - 0.22) * (z.to_um() / 0.4)))
        })
        .unwrap(),
    );
    sections.push(Section::new(half_slab(0.30), Length::ZERO));
    let lib = Eme::new(Polarization::Te, Wavelength::um(1.55).unwrap(), 6)
        .unwrap()
        .with_walls(crate::eme::Wall::Magnetic, crate::eme::Wall::Electric)
        .with_upper_pml(Length::um(4.0), 0.1)
        .unwrap()
        .s_matrix(&sections)
        .unwrap();
    assert_eq!([lib.s21[0][0].re, lib.s21[0][0].im], s[1][0]);
    assert_eq!([lib.s11[0][0].re, lib.s11[0][0].im], s[0][0]);
    // its walls are its own
    let walls = half.replace(
        "symmetric = true",
        "symmetric = true\nwalls = [\"electric\", \"electric\"]",
    );
    let err = check(&Job::parse(&walls).unwrap()).unwrap_err().to_string();
    assert!(err.contains("leave walls out"), "{err}");
}

#[test]
fn the_built_in_job_gives_the_numbers_its_comment_quotes() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
    let job = Job::load(&dir.join("eme-taper.yaml")).unwrap();
    let events = events(&job);
    let got = s_parameters(&events);
    let (ports, s) = &got[0];
    assert_eq!(ports, &["1 TE0", "2 TE0", "2 TE2"]);
    let t = s[1][0][0].powi(2) + s[1][0][1].powi(2);
    assert!((t - 0.99808).abs() < 5e-6, "{t}");
    let steps = events.iter().find_map(|e| match e {
        Event::SolveError { measure, .. } => Some(measure.clone()),
        _ => None,
    });
    assert!(steps.unwrap().contains("64 adaptive steps"));
}

#[test]
fn layers_given_one_by_one_are_the_same_as_a_core() {
    let layered = TAPER.replace(
        "[[task.section]]\nlength_um = 0.0\ncore_um = 0.30\n",
        "[[task.section]]\nlength_um = 0.0\nlayers = [{ medium = 1.444, thickness_um = 1.35 }, \
         { medium = 3.473, thickness_um = 0.30 }, { medium = 1.444, thickness_um = 1.35 }]\n",
    );
    assert_ne!(layered, TAPER);
    let a = s_parameters(&events(&Job::parse(TAPER).unwrap()));
    let b = s_parameters(&events(&Job::parse(&layered).unwrap()));
    assert_eq!(a, b);
}

#[test]
fn adaptive_steps_and_convergence_are_recorded() {
    let job = TAPER.replace("steps = 4\n", "steps = 2\n")
        + "\n[task.adaptive]\ntolerance = 1e-3\n\n[task.convergence]\ntolerance = 1e-3\n";
    let events = events(&Job::parse(&job).unwrap());
    let adaptive = events
        .iter()
        .filter_map(|e| match e {
            Event::SolveError { measure, error, .. } => Some((measure.clone(), *error)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(adaptive.len(), 1, "{adaptive:?}");
    assert!(adaptive[0].1 <= 1e-3, "{adaptive:?}");
    let studies = events
        .iter()
        .filter(|e| matches!(e, Event::GridConvergence { .. }))
        .count();
    assert_eq!(studies, 1);
    assert_eq!(s_parameters(&events).len(), 1);
}

#[test]
fn a_sweep_records_s_at_each_wavelength() {
    let job = TAPER.to_owned()
        + "\n[task.sweep]\nparameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 3\n";
    let events = events(&Job::parse(&job).unwrap());
    assert_eq!(s_parameters(&events).len(), 3);
}

#[test]
fn check_refuses_what_the_run_would() {
    let refused = |from: &str, to: &str, says: &str| {
        let text = TAPER.replace(from, to);
        assert_ne!(text, TAPER, "{from}");
        let err = check(&Job::parse(&text).unwrap()).unwrap_err().to_string();
        assert!(err.contains(says), "{err}");
    };
    // a taper as a port
    refused(
        "length_um = 0.0\ncore_um = 0.22",
        "length_um = 0.1\ncore_um = [0.2, 0.22]",
        "the first and last sections are the ports",
    );
    // a core and layers both
    refused(
        "core_um = 0.30\n",
        "core_um = 0.30\nlayers = [{ medium = 1.444, thickness_um = 3.0 }]\n",
        "either core_um or layers",
    );
    // layers that don't fill the window
    refused(
        "core_um = 0.30\n",
        "layers = [{ medium = 1.444, thickness_um = 2.0 }]\n",
        "fill the window",
    );
    // a core wider than the window
    refused("core_um = 0.30\n", "core_um = 3.5\n", "inside the window");
    // an unknown profile, and steps on a straight
    refused("steps = 4\n", "steps = 4\nprofile = \"cubic\"\n", "profile");
    refused(
        "core_um = 0.30\n",
        "core_um = 0.30\nsteps = 3\n",
        "a taper's",
    );
    // a medium the catalogue hasn't
    refused("core = 3.473", "core = \"unobtainium\"", "unobtainium");
    refused("modes = 12", "modes = 0", "modes must be at least 1");
}

/// [`TAPER`] as half the slab in a PML, swept over 1.5 to 1.6 µm at four wavelengths.
fn swept_taper() -> String {
    TAPER.replace("window_um = 3.0", "window_um = 1.5\nsymmetric = true")
        + "\n[task.pml]\nthickness_um = 4.0\nstrength = 0.1\n\n[task.sweep]\n\
           parameter = \"wavelength\"\nfrom = 1.5\nto = 1.6\npoints = 4\n"
}

#[test]
fn an_interpolated_sweep_is_within_its_estimate_of_the_solves() {
    let plain = s_parameters(&events(&Job::parse(&swept_taper()).unwrap()));
    let job = swept_taper() + "\n[task.interpolate]\ntolerance = 1e-6\n";
    let job = Job::parse(&job).unwrap();
    check(&job).unwrap();
    let events = events(&job);
    let got = s_parameters(&events);
    let (measure, estimate) = events
        .iter()
        .find_map(|e| match e {
            Event::SolveError { measure, error, .. } => Some((measure.clone(), *error)),
            _ => None,
        })
        .unwrap();
    assert!(measure.contains("interpolated from"), "{measure}");
    assert!(measure.contains("within its tolerance"), "{measure}");
    assert!(estimate <= 1e-6, "{estimate:e}");
    assert_eq!(got.len(), 4);
    for ((ports, s), (theirs, t)) in got.iter().zip(&plain) {
        assert_eq!(ports, theirs);
        for (row, other) in s.iter().zip(t) {
            for (a, b) in row.iter().zip(other) {
                let d = (c64::new(a[0], a[1]) - c64::new(b[0], b[1])).norm();
                assert!(d <= estimate, "{d:e} against {estimate:e}");
            }
        }
    }
}

#[test]
fn interpolation_needs_a_sweep_and_a_tolerance_above_the_floor() {
    let alone = TAPER.to_owned() + "\n[task.interpolate]\ntolerance = 1e-6\n";
    let err = check(&Job::parse(&alone).unwrap()).unwrap_err().to_string();
    assert!(err.contains("give a [task.sweep]"), "{err}");
    let tight = swept_taper() + "\n[task.interpolate]\ntolerance = 1e-14\n";
    let err = check(&Job::parse(&tight).unwrap()).unwrap_err().to_string();
    assert!(err.contains("at least 1e-13"), "{err}");
    let few = swept_taper() + "\n[task.interpolate]\ntolerance = 1e-6\nmax_solves = 3\n";
    let err = check(&Job::parse(&few).unwrap()).unwrap_err().to_string();
    assert!(err.contains("at least 5"), "{err}");
}

/// `jobs/eme-mmi-sweep.yaml` is the multimode section of `eme/wavelength-mmi`: its interpolant
/// is the case's to the bit, from the solves its comment quotes.
#[test]
#[ignore = "slow in a debug build: in release"]
fn the_built_in_sweep_is_the_cases_interpolant() {
    use crate::eme::spectrum_checks::{MMI_RANGE, mmi};
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
    let job = Job::load(&dir.join("eme-mmi-sweep.yaml")).unwrap();
    let events = events(&job);
    let got = s_parameters(&events);
    assert_eq!(got.len(), 201);
    let um = |l: f64| Wavelength::um(l).unwrap();
    let spectrum = crate::eme::Spectrum::run(
        |l| mmi(l.to_um()),
        um(MMI_RANGE.0),
        um(MMI_RANGE.1),
        (1, 1),
        &crate::sweep::Options::new(1e-6),
    )
    .unwrap();
    let measure = events
        .iter()
        .find_map(|e| match e {
            Event::SolveError { measure, .. } => Some(measure.clone()),
            _ => None,
        })
        .unwrap();
    assert!(
        measure.contains(&format!("from {} solves", spectrum.solves())),
        "{measure}"
    );
    eprintln!("{measure}: estimate {:e}", spectrum.error());
    let (ports, s) = &got[100];
    assert_eq!(ports, &["1 TE0", "2 TE0"]);
    let r = spectrum.at(um(MMI_RANGE.0 + (MMI_RANGE.1 - MMI_RANGE.0) * 100.0 / 200.0));
    assert_eq!(s[1][0], [r[1][0].re, r[1][0].im]);
}

/// A slab grating: 0.3 µm of 300 nm slab and 0.2 µm of 220 nm, ten times, between two 220 nm
/// ports, in [`TAPER`]'s window.
fn grating() -> String {
    TAPER
        .replace(
            "[[task.section]]\nlength_um = 0.4\ncore_um = [0.22, 0.30]\nsteps = 4\n",
            "[[task.section]]\nlength_um = 0.3\ncore_um = 0.30\n\n[[task.section]]\n\
             length_um = 0.2\ncore_um = 0.22\n",
        )
        .replace(
            "length_um = 0.0\ncore_um = 0.30\n",
            "length_um = 0.0\ncore_um = 0.22\n",
        )
        + "\n[task.period]\nsections = [2, 3]\ncount = 10\n"
}

#[test]
fn a_period_is_the_librarys_grating_and_bloch_modes_to_the_bit() {
    let job = Job::parse(&grating()).unwrap();
    check(&job).unwrap();
    let events = events(&job);
    let got = s_parameters(&events);
    assert_eq!(got.len(), 1);
    let (ports, s) = &got[0];
    assert_eq!(ports, &["1 TE0", "2 TE0"]);
    let sections = [
        Section::new(slab(0.22), Length::ZERO),
        Section::new(slab(0.30), Length::um(0.3)),
        Section::new(slab(0.22), Length::um(0.2)),
        Section::new(slab(0.22), Length::ZERO),
    ];
    let eme = Eme::new(Polarization::Te, Wavelength::um(1.55).unwrap(), 12).unwrap();
    let lib = eme.grating(&sections, 1..3, 10).unwrap();
    assert_eq!([lib.s21[0][0].re, lib.s21[0][0].im], s[1][0]);
    assert_eq!([lib.s11[0][0].re, lib.s11[0][0].im], s[0][0]);
    let bloch = eme.bloch(&sections[1..3]).unwrap();
    let point = events
        .iter()
        .find_map(|e| match e {
            Event::SweepPoint {
                effective_indices,
                te_fractions,
                ..
            } => Some((effective_indices.clone(), te_fractions.clone())),
            _ => None,
        })
        .unwrap();
    assert_eq!(point.0.len(), 4);
    assert_eq!(point.1, vec![1.0; 4]);
    for (got, want) in point.0.iter().zip(&bloch.effective_indices) {
        assert_eq!(*got, [want.re, want.im]);
    }
    // the scene draws every period
    let shapes = events
        .iter()
        .find_map(|e| match e {
            Event::Scene { shapes, .. } => Some(shapes.len()),
            _ => None,
        })
        .unwrap();
    assert_eq!(shapes, 20);
}

#[test]
fn check_refuses_a_period_it_cannot_repeat() {
    let refused = |text: String, says: &str| {
        let err = check(&Job::parse(&text).unwrap()).unwrap_err().to_string();
        assert!(err.contains(says), "{err}");
    };
    let g = grating();
    for bad in ["[1, 3]", "[3, 2]", "[2, 4]"] {
        refused(
            g.replace("sections = [2, 3]", &format!("sections = {bad}")),
            "between the ports",
        );
    }
    refused(g.replace("count = 10", "count = 0"), "at least 1");
    refused(
        g.clone() + "\n[task.adaptive]\ntolerance = 1e-3\n",
        "takes equal steps",
    );
}
