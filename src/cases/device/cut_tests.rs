//! The device cut's tests: where it places the sections, what it refuses, and (ignored, in
//! release) the measurements behind its cases' tolerances.

use super::cut_checks::{device_cut, eme, fdfd, mmi, taper};
use super::{Cut, Device, StripYBranch, Values};
use crate::units::Wavelength;

fn lam() -> Wavelength {
    Wavelength::from_um_unchecked(1.55)
}

#[test]
fn an_mmi_is_cut_at_its_faces_into_one_section_each() {
    let (geometry, length) = mmi();
    let cut = device_cut(0.1).geometry(&geometry, lam()).expect("a cut");
    // the input guide, the body, the outputs: no sloped run
    assert_eq!(cut.sections.len(), 3, "{:?}", cut.edges);
    assert!((cut.edges[1] - cut.edges[0] - length).abs() < 1e-9);
    assert_eq!(cut.sections[1].length.to_um(), length);
    assert_eq!(cut.grid.nx, 24);
    assert_eq!(cut.grid.ny, 16);
}

#[test]
fn a_taper_is_cut_into_equal_steps_no_longer_than_asked() {
    let geometry = taper();
    for (along, steps) in [(0.25, 4), (0.3, 4), (1.0, 1), (0.1, 10)] {
        let cut = device_cut(along).geometry(&geometry, lam()).expect("a cut");
        assert_eq!(cut.sections.len(), steps + 2, "steps of {along} um");
        for s in &cut.sections[1..=steps] {
            assert!((s.length.to_um() - 1.0 / steps as f64).abs() < 1e-12);
        }
        assert_eq!(cut.sections[0].length.to_um(), 0.0);
    }
}

#[test]
fn an_exact_cut_holds_every_plane_a_fine_cut_sees() {
    // the taper's sloped wall crosses the cells' sampling points every 31.25 nm along x: the
    // exact cut's planes are every plane a cut in 2 nm steps meets, each once in a row
    let geometry = taper();
    let exact = device_cut(1.0)
        .exactly()
        .geometry(&geometry, lam())
        .expect("a cut");
    let fine = device_cut(0.002).geometry(&geometry, lam()).expect("a cut");
    let planes: Vec<&crate::eme::Plane> = exact.sections.iter().map(|s| &s.plane).collect();
    for s in &fine.sections {
        assert!(planes.contains(&&s.plane), "a plane the exact cut missed");
    }
    for w in exact.sections[1..exact.sections.len() - 1].windows(2) {
        assert_ne!(w[0].plane, w[1].plane, "a cut where nothing changes");
    }
    let total: f64 = exact.sections.iter().map(|s| s.length.to_um()).sum();
    assert!((total - 1.0).abs() < 1e-12, "{total}");
}

#[test]
fn a_mirrored_cut_refuses_a_device_that_isnt_symmetric() {
    // a 2 x 2 MMI's access guides sit at -W/6 and W/6 on both sides: symmetric; a Y-branch is
    // too. Shifted off its axis, the taper isn't.
    let mut geometry = taper();
    let shifted = crate::geometry::Shape::rect(
        crate::geometry::Point::um(0.5, 0.3),
        crate::units::Length::um(1.0),
        crate::units::Length::um(0.2),
    )
    .expect("a strip");
    geometry
        .structure
        .draw("core", shifted)
        .expect("the core layer");
    let e = device_cut(0.25)
        .geometry(&geometry, lam())
        .expect_err("not its own mirror image");
    assert!(e.to_string().contains("mirror"), "{e}");
}

#[test]
fn a_cut_refuses_bad_steps_and_ports_off_the_axis() {
    assert!(Cut::new(0.0, 0.05, 0.1).is_err());
    assert!(Cut::new(0.05, 0.05, f64::NAN).is_err());
    assert!(
        Cut::new(0.05, 0.05, 0.1)
            .and_then(|c| c.with_margins(-0.1, 0.2))
            .is_err()
    );
    let mut geometry = taper();
    geometry.ports[1].direction = 90.0;
    assert!(device_cut(0.1).geometry(&geometry, lam()).is_err());
}

#[test]
fn the_same_code_cuts_a_y_branch() {
    let device = StripYBranch::default();
    let geometry = device
        .geometry(&Values::defaults(&device))
        .expect("its geometry");
    let cut = Cut::new(0.05, 0.055, 0.1)
        .and_then(|c| c.with_margins(0.4, 0.33))
        .expect("a cut")
        .mirrored()
        .geometry(&geometry, lam())
        .expect("a Y-branch cut");
    // its curved arms are polygons of many short sloped edges: runs in steps of at most 0.1 µm;
    // only the stretches whose outlines run along x are longer, one section each
    let lengths: Vec<f64> = cut.sections.iter().map(|s| s.length.to_um()).collect();
    let long = lengths.iter().filter(|&&l| l > 0.1 + 1e-12).count();
    assert!(long <= 2, "{lengths:?}");
    assert!(cut.sections.len() > 10, "{lengths:?}");
}

/// The measurements: `cargo test --release --lib device::cut_tests -- --ignored --nocapture`.
#[test]
#[ignore = "minutes in release: the measurements behind the cut's cases"]
fn measure_one_fdfd_solve_of_each_device() {
    for (name, geometry) in [("taper", taper()), ("mmi", mmi().0)] {
        let grid = device_cut(0.1)
            .geometry(&geometry, lam())
            .expect("a cut")
            .grid;
        let t = std::time::Instant::now();
        let ((r, s21), unknowns) = fdfd(&geometry, grid, 0.05, (1.0, 1.5));
        println!(
            "{name}: FDFD dx 50 nm, {unknowns} unknowns: |S11|^2 {r:.6e} |S21|^2 {s21:.8} in {:.1} s",
            t.elapsed().as_secs_f64()
        );
        let t = std::time::Instant::now();
        let ((r, s21), _) = eme(&geometry, &device_cut(0.125));
        println!(
            "{name}: EME every mode of {} unknowns: |S11|^2 {r:.6e} |S21|^2 {s21:.8} in {:.1} s",
            grid.unknowns(),
            t.elapsed().as_secs_f64()
        );
    }
}

/// The measurements behind the cut's rules, as the method page's tables give them:
/// `cargo test --release --lib device::cut_tests::measure_the_cross_checks -- --ignored
/// --nocapture` (about 10 minutes).
#[test]
#[ignore = "minutes in release: FDFD's own error, from its steps, PMLs and leads, and the cases' values"]
fn measure_the_cross_checks() {
    use super::cut_checks::{PML_LEAD, taper_exact};
    let t = std::time::Instant::now();
    let ((r, s), _) = taper_exact();
    println!(
        "taper EME cut exactly: |S11|^2 {r:.8e}, |S21|^2 {s:.9} ({:.0} s)",
        t.elapsed().as_secs_f64()
    );
    let steps = [2usize, 3, 4, 5, 6, 7];
    let dx: Vec<f64> = steps.iter().map(|&m| 0.1 / m as f64).collect();
    for (name, geometry) in [("taper", taper()), ("mmi", mmi().0)] {
        let grid = device_cut(0.1)
            .geometry(&geometry, lam())
            .expect("a cut")
            .grid;
        let t = std::time::Instant::now();
        let v: Vec<(f64, f64)> = dx
            .iter()
            .map(|&h| fdfd(&geometry, grid, h, PML_LEAD).0)
            .collect();
        println!(
            "{name}: FDFD on {steps:?} ({:.0} s): {v:?}",
            t.elapsed().as_secs_f64()
        );
        let fit = |k: usize| {
            let r: Vec<f64> = v[k..].iter().map(|p| p.0).collect();
            let s: Vec<f64> = v[k..].iter().map(|p| p.1).collect();
            (
                crate::eme::three_checks::z_limit(&dx[k..], &r),
                crate::eme::three_checks::z_limit(&dx[k..], &s),
            )
        };
        println!(
            "{name}: limits (value, residual) from 2..7 {:?}, from 3..7 {:?}",
            fit(0),
            fit(1)
        );
        for setting in [(0.5, 1.0), (1.0, 1.5), (2.0, 1.5)] {
            let ((r, s), _) = fdfd(&geometry, grid, 0.025, setting);
            println!(
                "{name}: FDFD dx 25 nm, PML and lead {setting:?}: |S11|^2 {r:.6e}, |S21|^2 {s:.8}"
            );
        }
    }
    let ((r, s), m) = eme(&mmi().0, &device_cut(0.1));
    let (sym, unit) = super::cut_checks::invariants(&m);
    println!("mmi EME: |S11|^2 {r:.7e}, |S21|^2 {s:.9}, symmetry {sym:.1e}, unitarity {unit:.1e}");
}

#[test]
#[ignore = "a minute and a half in release: the measurements behind the drawing cases"]
fn measure_the_drawings() {
    let (extruded, n) = super::cut_checks::drawing_extruded();
    println!("stepped slab drawn in a job: {extruded:.2e} over {n} recorded entries");
    let (taper, n) = super::cut_checks::drawing_taper();
    println!("taper drawn as a path: {taper:.2e} over {n} recorded entries");
    let (gds, n) = super::cut_checks::drawing_gds();
    println!("stepped slab from a GDSII file: {gds:.2e} over {n} recorded entries");
}

#[test]
fn a_drawing_read_from_a_gds_file_is_the_one_drawn_as_rectangles() {
    // the guided modes at the two ends, as eme/3d-drawing-extruded's: 3 ports, 9 entries, the
    // same to the bit (the file's whole nanometres are the rectangles' corners)
    let (difference, n) = super::cut_checks::drawing_gds();
    assert_eq!(n, 9);
    assert_eq!(difference, 0.0);
}
