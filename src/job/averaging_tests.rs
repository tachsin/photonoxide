//! `averaging` in `"fdfd"` jobs: checked, and solved from the fill.

use super::*;

/// A 510 nm strip along x on SOI by 2D FDFD (its edges between the 40 nm grid's lines), with
/// `extra` in the task and `shapes` after the strip.
fn strip_job(extra: &str, shapes: &str) -> String {
    format!(
        r#"
name = "strip-fill"

[task]
kind = "fdfd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.4, 1.4]
y_um = [-1.4, 1.4]
step_nm = 40.0
{extra}

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.013]
size_um = [4.0, 0.51]
{shapes}

[[task.port]]
x_um = -0.4
side = "left"

[[task.port]]
x_um = 0.4
side = "right"
"#
    )
}

fn s_of(text: &str) -> FdfdSParameters {
    let job = Job::parse(text).unwrap();
    check(&job).unwrap();
    fdfd_s_parameters(&job, None).unwrap()
}

#[test]
fn the_default_is_samples_to_the_bit() {
    let plain = s_of(&strip_job("", ""));
    let samples = s_of(&strip_job(r#"averaging = "samples""#, ""));
    assert_eq!(plain, samples);
}

#[test]
fn a_strip_averaged_from_the_fill_passes_everything_through() {
    for pol in ["te", "tm"] {
        let samples = s_of(&strip_job(&format!("polarization = \"{pol}\""), ""));
        let sampled = samples.s[0][1][0].norm();
        for name in ["point", "mean", "diagonal", "tensor"] {
            let s = s_of(&strip_job(
                &format!("averaging = \"{name}\"\npolarization = \"{pol}\""),
                "",
            ));
            // as much through as with samples (the TM mode, less confined, reaches the PMLs a
            // little either way)
            let through = s.s[0][1][0].norm();
            assert!(
                (through - sampled).abs() < 1e-5,
                "{name} {pol}: {through} {sampled}"
            );
            assert!((through - 1.0).abs() < 1e-4, "{name} {pol}: {through}");
            // the strip's edges are between the lines: its index moves with the averaging
            let (a, b) = (s.effective_indices[0][0], samples.effective_indices[0][0]);
            assert!(a != b && (a - b).abs() < 0.05, "{name} {pol}: {a} {b}");
        }
    }
}

#[test]
fn averaging_is_checked() {
    let e = check(&Job::parse(&strip_job(r#"averaging = "staircase""#, "")).unwrap())
        .unwrap_err()
        .to_string();
    assert!(e.contains("staircase"), "{e}");
    // a sloped wall changes the column's index across it: refused from the fill
    let sloped = strip_job(r#"averaging = "mean""#, "").replace(
        "size_um = [4.0, 0.51]",
        "size_um = [4.0, 0.51]\netch = { sidewall_deg = 80.0 }",
    );
    let e = check(&Job::parse(&sloped).unwrap())
        .unwrap_err()
        .to_string();
    assert!(e.contains("sloped"), "{e}");
    // and is fine with samples
    check(&Job::parse(&sloped.replace(r#"averaging = "mean""#, "")).unwrap()).unwrap();
}

#[test]
fn a_partial_etch_is_two_columns_from_the_fill() {
    let rib = strip_job(r#"averaging = "tensor""#, "").replace(
        "size_um = [4.0, 0.51]",
        "size_um = [4.0, 0.51]\netch = { depth_um = 0.15 }",
    );
    let job = Job::parse(&rib).unwrap();
    check(&job).unwrap();
    let s = fdfd_s_parameters(&job, None).unwrap();
    assert!((s.s[0][1][0].norm() - 1.0).abs() < 1e-6);
    let strip = s_of(&strip_job(r#"averaging = "tensor""#, ""));
    // the rib's mode spreads into its slab, whose index is far above the oxide's: a higher
    // index than the strip's, and near what samples give the rib
    let sampled = s_of(&rib.replace(r#"averaging = "tensor""#, ""));
    let (rib, strip, sampled) = (
        s.effective_indices[0][0],
        strip.effective_indices[0][0],
        sampled.effective_indices[0][0],
    );
    assert!(rib > strip, "{rib} {strip}");
    assert!(
        rib != sampled && (rib - sampled).abs() < 0.05,
        "{rib} {sampled}"
    );
}
