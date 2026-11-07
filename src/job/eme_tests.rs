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

/// `jobs/eme-grating.yaml` gives the stop band its comment quotes.
#[test]
#[ignore = "81 wavelengths: in release"]
fn the_built_in_grating_gives_the_numbers_its_comment_quotes() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
    let job = Job::load(&dir.join("eme-grating.yaml")).unwrap();
    let events = events(&job);
    let mut rows = Vec::new();
    let mut bloch = Vec::new();
    for e in &events {
        match e {
            Event::SParameters {
                wavelength_um, s, ..
            } => rows.push((
                *wavelength_um,
                s[1][0][0].powi(2) + s[1][0][1].powi(2),
                s[0][0][0].powi(2) + s[0][0][1].powi(2),
            )),
            Event::SweepPoint {
                effective_indices, ..
            } => bloch.push(effective_indices[0]),
            _ => {}
        }
    }
    assert_eq!(rows.len(), 81);
    assert_eq!(bloch.len(), 81);
    // the stop band: where the fundamental Bloch mode decays, Im K/k0 above 0.01
    let band: Vec<f64> = rows
        .iter()
        .zip(&bloch)
        .filter(|(_, b)| b[1] > 0.01)
        .map(|(r, _)| r.0)
        .collect();
    assert_eq!(band.len(), 24, "{band:?}");
    assert!((band[0] - 1.6025).abs() < 1e-9 && (band[23] - 1.66).abs() < 1e-9);
    let (l, _, r) = rows
        .iter()
        .copied()
        .fold((0.0, 0.0, 0.0), |a, b| if b.2 > a.2 { b } else { a });
    assert!(
        (l - 1.6375).abs() < 1e-9 && (r - 0.998307).abs() < 5e-7,
        "{l} {r}"
    );
    let decay = bloch.iter().map(|b| b[1]).fold(0.0, f64::max);
    assert!((decay - 0.066627).abs() < 5e-7, "{decay}");
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

/// A TE slab, 220 nm, through a 90° bend at 2 µm to its centre between two straights, the core
/// 0.75 µm below the window's middle.
fn bend_job() -> String {
    r#"
name = "bend"
timeout_minutes = 5

[task]
kind = "eme"
wavelength_um = 1.55
modes = 8
window_um = 4.5
cladding = 1.444
core = 3.473

[task.pml]
thickness_um = 2.0
strength = 0.5
sides = "upper"

[[task.section]]
length_um = 0.0
core_um = 0.22
center_um = -0.75

[[task.section]]
radius_um = 2.0
angle_deg = 90.0
core_um = 0.22
center_um = -0.75

[[task.section]]
length_um = 0.0
core_um = 0.22
center_um = -0.75
"#
    .to_owned()
}

#[test]
fn a_bend_is_the_librarys_bend_to_the_bit() {
    let job = Job::parse(&bend_job()).unwrap();
    check(&job).unwrap();
    let events = events(&job);
    let got = s_parameters(&events);
    let (ports, s) = &got[0];
    assert_eq!(ports, &["1 TE0", "2 TE0"]);
    let clad = c64::new(1.444, 0.0);
    let slab = Slice::new(&[
        (clad, Length::um(2.25 - 0.75 - 0.11)),
        (c64::new(3.473, 0.0), Length::um(0.22)),
        (clad, Length::um(2.25 + 0.75 - 0.11)),
    ])
    .unwrap();
    // the radius to the window's middle: the core's 2 µm, plus its 0.75 µm below the middle
    let sections = [
        Section::new(slab.clone(), Length::ZERO),
        Section::bend(slab.clone(), Length::um(2.75), std::f64::consts::FRAC_PI_2).unwrap(),
        Section::new(slab, Length::ZERO),
    ];
    let lib = Eme::new(Polarization::Te, Wavelength::um(1.55).unwrap(), 8)
        .unwrap()
        .with_upper_pml(Length::um(2.0), 0.5)
        .unwrap()
        .s_matrix(&sections)
        .unwrap();
    assert_eq!([lib.s21[0][0].re, lib.s21[0][0].im], s[1][0]);
    assert_eq!([lib.s11[0][0].re, lib.s11[0][0].im], s[0][0]);
    // the bend's arc, 2.75 µm × π/2 at the window's middle, is the section's length
    assert!((sections[1].length.to_um() - 2.75 * std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    // the scene turns: the output faces down, the drawing as tall as it is wide
    let (x, y) = events
        .iter()
        .find_map(|e| match e {
            Event::Scene { x_um, y_um, .. } => Some((*x_um, *y_um)),
            _ => None,
        })
        .unwrap();
    assert!((x[1] - x[0] - (2.75 + 2.25)).abs() < 1e-9, "{x:?}");
    assert!((y[1] - y[0] - (2.75 + 2.25)).abs() < 1e-9, "{y:?}");
}

/// The bend of [`bend_job`] by the library, `modes` modes, its thin layers at most `step` µm.
fn library_bend(modes: usize, step: f64) -> crate::eme::Scattering {
    let clad = c64::new(1.444, 0.0);
    let slab = Slice::new(&[
        (clad, Length::um(2.25 - 0.75 - 0.11)),
        (c64::new(3.473, 0.0), Length::um(0.22)),
        (clad, Length::um(2.25 + 0.75 - 0.11)),
    ])
    .unwrap();
    let sections = [
        Section::new(slab.clone(), Length::ZERO),
        Section::bend(slab.clone(), Length::um(2.75), std::f64::consts::FRAC_PI_2).unwrap(),
        Section::new(slab, Length::ZERO),
    ];
    Eme::new(Polarization::Te, Wavelength::um(1.55).unwrap(), modes)
        .unwrap()
        .with_upper_pml(Length::um(2.0), 0.5)
        .unwrap()
        .with_bend_step(Length::um(step))
        .unwrap()
        .s_matrix(&sections)
        .unwrap()
}

#[test]
fn a_bends_layers_are_the_tasks_and_the_convergence_refines_them() {
    // bend_step_nm: the library's bend with that step, to the bit
    let job =
        Job::parse(&bend_job().replace("modes = 8", "modes = 8\nbend_step_nm = 5.0")).unwrap();
    check(&job).unwrap();
    let (_, s) = &s_parameters(&events(&job))[0];
    let lib = library_bend(8, 0.005);
    assert_eq!([lib.s21[0][0].re, lib.s21[0][0].im], s[1][0]);
    // [task.convergence] drives the modes, then the bend's layers from the default, halving
    let job = Job::parse(&format!(
        "{}\n[task.convergence]\ntolerance = 1e-5\n",
        bend_job()
    ))
    .unwrap();
    check(&job).unwrap();
    let events = events(&job);
    let studies: Vec<&crate::convergence::Convergence> = events
        .iter()
        .filter_map(|e| match e {
            Event::GridConvergence { check, .. } => Some(check),
            _ => None,
        })
        .collect();
    assert_eq!(studies.len(), 2, "the modes' study and the layers'");
    let layers = studies[1];
    assert!(
        layers.quantity.contains("the bends' layers"),
        "{}",
        layers.quantity
    );
    assert_eq!(
        layers.steps_um[0],
        crate::eme::Eme::new(Polarization::Te, Wavelength::um(1.55).unwrap(), 1)
            .unwrap()
            .bend_step()
            .to_um()
    );
    assert!(layers.steps_um.len() >= 2);
    let modes = (1.0 / studies[0].step_um).round() as usize;
    // the run's S is the library's at the modes found and the finest layers studied
    let (_, s) = &s_parameters(&events)[0];
    let lib = library_bend(modes, layers.step_um);
    assert_eq!([lib.s21[0][0].re, lib.s21[0][0].im], s[1][0]);
    assert!(
        check(
            &Job::parse(&bend_job().replace("modes = 8", "modes = 8\nbend_step_nm = 0.0")).unwrap()
        )
        .unwrap_err()
        .to_string()
        .contains("bend_step_nm must be above 0")
    );
}

#[test]
fn check_refuses_a_bend_it_cannot_solve() {
    let refused = |text: String, says: &str| {
        let err = check(&Job::parse(&text).unwrap()).unwrap_err().to_string();
        assert!(err.contains(says), "{err}");
    };
    let b = bend_job();
    refused(
        b.replace("radius_um = 2.0\nangle_deg = 90.0", "radius_um = 2.0"),
        "both radius_um and angle_deg",
    );
    refused(
        b.replace("angle_deg = 90.0", "angle_deg = 90.0\nlength_um = 1.0"),
        "leave length_um out",
    );
    refused(b.replace("angle_deg = 90.0", "angle_deg = 0.0"), "above 0");
    refused(
        b.replace("radius_um = 2.0", "radius_um = 1.0"),
        "on one side of the bend's centre",
    );
    refused(
        b.replacen(
            "length_um = 0.0\ncore_um = 0.22",
            "radius_um = 5.0\nangle_deg = 10.0\ncore_um = 0.22",
            1,
        ),
        "not bends",
    );
    refused(
        b.replace(
            "core_um = 0.22\ncenter_um = -0.75\n\n[[task.section]]\nlength_um = 0.0",
            "core_um = [0.22, 0.3]\ncenter_um = -0.75\n\n[[task.section]]\nlength_um = 0.0",
        ),
        "one thickness",
    );
}

/// `jobs/eme-bend.yaml`'s TM bend at 2 µm with a PML of strength 0.25, 0.5 and 1 and 16 and 24
/// modes: every one is followed (0.5 and 1 lost two modes on the paths from the uniform window,
/// which the closed window's path now takes), and at 24 modes the three agree within the
/// expansion's mode-count gate, 1e-4 as `eme/taper-modes` (measured: 1.5e-5).
#[test]
#[ignore = "18 solves: in release"]
fn the_bend_job_follows_its_modes_at_every_pml_strength() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
    let text = std::fs::read_to_string(dir.join("eme-bend.yaml")).unwrap();
    let mut at_24: Vec<Vec<f64>> = Vec::new();
    for strength in ["0.25", "0.5", "1.0"] {
        for modes in ["16", "24"] {
            let t = text
                .replace("strength: 0.25", &format!("strength: {strength}"))
                .replace("modes: 16", &format!("modes: {modes}"))
                .replace("points: 11", "points: 3");
            let job = Job::parse_as(&t, crate::run::Format::Yaml).unwrap();
            let mut run = Run::in_memory();
            execute(&job, &mut run, &Stop::new(None))
                .unwrap_or_else(|e| panic!("strength {strength}, {modes} modes: {e}"));
            let rows: Vec<f64> = run
                .lines()
                .iter()
                .filter_map(|l| match serde_json::from_str(l).unwrap() {
                    Event::SParameters { s, .. } => Some(s[1][0][0].powi(2) + s[1][0][1].powi(2)),
                    _ => None,
                })
                .collect();
            assert_eq!(rows.len(), 3);
            if modes == "24" {
                at_24.push(rows);
            }
        }
    }
    for other in &at_24[1..] {
        for (a, b) in at_24[0].iter().zip(other) {
            assert!((a - b).abs() < 1e-4, "{at_24:?}");
        }
    }
}

/// `jobs/eme-bend.yaml` gives the numbers its comment quotes.
#[test]
#[ignore = "11 wavelengths: in release"]
fn the_built_in_bend_gives_the_numbers_its_comment_quotes() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
    let job = Job::load(&dir.join("eme-bend.yaml")).unwrap();
    let rows: Vec<(f64, f64)> = events(&job)
        .iter()
        .filter_map(|e| match e {
            Event::SParameters {
                wavelength_um, s, ..
            } => Some((*wavelength_um, s[1][0][0].powi(2) + s[1][0][1].powi(2))),
            _ => None,
        })
        .collect();
    assert_eq!(rows.len(), 11);
    for (at, want) in [(0, 0.938954), (5, 0.899089), (10, 0.841900)] {
        assert!((rows[at].1 - want).abs() < 5e-7, "{:?}", rows[at]);
    }
}
