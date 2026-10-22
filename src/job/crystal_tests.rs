//! Oriented crystals in jobs: read, checked, refused where their tensor isn't taken, and solved.

use super::*;
use crate::run::replay;

/// A lithium niobate strip 1 µm wide and 600 nm thick on oxide, in oxide, its modes along
/// `along`, the film's material `film`.
fn strip(along: &str, film: &str) -> String {
    let (window, size) = match along {
        "x" => ("y_um = [-1.6, 1.6]", r#"size = ["10 um", "1 um"]"#),
        _ => ("x_um = [-1.6, 1.6]", r#"size = ["1 um", "10 um"]"#),
    };
    format!(
        r#"
name = "tfln"

[task]
kind = "modes"
wavelength_um = 1.55
layer = "LN"
step_nm = 25.0
propagation = "{along}"
{window}
modes = 2

[task.stack]
substrate = "sio2"
cladding = "sio2"

[[task.stack.layer]]
name = "BOX"
thickness_um = 2.0
material = "sio2"
background = "sio2"

[[task.stack.layer]]
name = "LN"
thickness_um = 0.6
material = {film}
background = "sio2"

[[task.rect]]
layer = "LN"
center_um = [0.0, 0.0]
{size}
"#
    )
}

/// The modes' effective indices and TE fractions, run in a directory named after `tag`.
fn solve(text: &str, tag: &str) -> Vec<(f64, f64)> {
    let dir =
        std::env::temp_dir().join(format!("photonoxide-crystal-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let job = Job::parse(text).unwrap();
    check(&job).unwrap();
    let mut run = Run::create(&dir, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    events
        .iter()
        .filter_map(|e| match e {
            Event::Mode {
                effective_index,
                te_fraction,
                ..
            } => Some((effective_index[0], *te_fraction)),
            _ => None,
        })
        .collect()
}

/// The TE-like mode of the highest index.
fn te(modes: &[(f64, f64)]) -> f64 {
    modes
        .iter()
        .filter(|m| m.1 > 0.5)
        .map(|m| m.0)
        .fold(f64::NEG_INFINITY, f64::max)
}

#[test]
fn an_x_cut_strips_te_mode_sees_n_e_and_a_z_cuts_n_o() {
    let x_cut = solve(
        &strip("x", r#"{ id = "linbo3", orientation = "x-cut" }"#),
        "x",
    );
    let z_cut = solve(
        &strip("x", r#"{ id = "linbo3", orientation = "z-cut" }"#),
        "z",
    );
    // lithium niobate is negative uniaxial: the x-cut's TE-like mode, polarized along c, is
    // below the z-cut's, polarized across it (25 nm grid: x-cut TE 1.85965, TM 1.86857; z-cut
    // TE 1.92533, TM 1.81123)
    let (tx, tz) = (te(&x_cut), te(&z_cut));
    assert!(tx.is_finite() && tz.is_finite(), "{x_cut:?} {z_cut:?}");
    assert!(tz - tx > 0.03, "x-cut {tx}, z-cut {tz}");
    // the same x-cut strip along y, its c axis turned to stay across the guide (along the
    // chip's x): the same modes, the frame's turn and the tensor's both exact
    let along_y = solve(
        &strip(
            "y",
            r#"{ id = "linbo3", orientation = { cut = "x", along_x = "z" } }"#,
        ),
        "y",
    );
    assert_eq!(along_y.len(), x_cut.len());
    for (a, b) in along_y.iter().zip(&x_cut) {
        assert!((a.0 - b.0).abs() < 1e-10, "{along_y:?} vs {x_cut:?}");
    }
}

#[test]
fn a_job_refuses_a_bad_orientation_and_a_crystal_its_solver_cant_take() {
    let refused = |film: &str, says: &str| {
        let job = Job::parse(&strip("x", film)).unwrap();
        let e = check(&job).unwrap_err().to_string();
        assert!(e.contains(says), "{film}: {e}");
    };
    refused(
        r#"{ id = "linbo3", orientation = { euler_deg = [0, 400, 0] } }"#,
        "from -360 to 360",
    );
    refused(
        r#"{ id = "linbo3", orientation = "q-cut" }"#,
        "unknown orientation",
    );
    // no principal axis along the guide: the solver leaves the coupling out, so it's refused
    refused(
        r#"{ id = "linbo3", orientation = { euler_deg = [30, 40, 50] } }"#,
        "principal axis",
    );
    // an anisotropic crystal without its orientation
    refused(r#""linbo3""#, "is anisotropic");
    // an isotropic one has none to give
    refused(r#"{ id = "si", orientation = "x-cut" }"#, "isotropic");
    // a principal axis along the guide but tilted in the cross-section is taken: ε_xy ≠ 0
    let job = Job::parse(&strip(
        "x",
        r#"{ id = "linbo3", orientation = { euler_deg = [0, 30, 0] } }"#,
    ))
    .unwrap();
    check(&job).unwrap();
    // FDFD, FDTD and static jobs take one permittivity per cell: their stacks refuse it
    let table: toml::Table = toml::from_str(
        r#"
substrate = "sio2"
cladding = { id = "linbo3", orientation = "z-cut" }

[[layer]]
name = "BOX"
thickness_um = 2.0
material = "sio2"
background = "sio2"
"#,
    )
    .unwrap();
    let e = named_stack(&StackField::Layers(table.clone()), None, None)
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("the cladding's") && e.contains("oriented crystal"),
        "{e}"
    );
    assert!(tensor_stack(&StackField::Layers(table), None, None).is_ok());
}
