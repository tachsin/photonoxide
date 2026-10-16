//! Tests of `[[task.body]]`: bodies in space in jobs, checked, placed and drawn.

use super::{Event, StructureTask, check, params, preview};
use crate::geometry::Point;
use crate::run::Job;
use crate::units::{Length, Wavelength};

const JOB: &str = r#"
name = "bodies"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-4.0, 4.0]
y_um = [-4.0, 4.0]
step_nm = 100.0

[task.parameters]
r = "300 nm"
top = "2.22 um"

[[task.body]]
type = "sphere"
center = ["-2 um", "-2 um", "top + 0.5 um"]
radius = "r"
material = "si"

[[task.body]]
type = "ellipsoid"
center = ["2 um", "-2 um", "top + 0.5 um"]
semi_axes = ["0.6 um", "0.3 um", "0.2 um"]
angle = "30 deg"
material = "si3n4"

[[task.body]]
type = "box"
center = [0, 0, "top + 0.3 um"]
size = ["1 um", "1 um", "0.6 um"]
permittivity = 4

[[task.body]]
type = "cylinder"
base = ["2 um", "2 um", "top"]
top = ["2 um", "2 um", "top + 1 um"]
radius = "r"
material = "si"

[[task.body]]
type = "cone"
base = ["-2 um", "2 um", "top"]
top = ["-2 um", "2 um", "top + 1 um"]
radii = ["0.5 um", 0]
material = "si"

[[task.body]]
type = "extrusion"
z = ["top + 0.1 um", "top + 0.4 um"]
sidewall = "80 deg"
of = [{ type = "circle", center = [0, "-3 um"], radius = "0.8 um" }]
index = "1.5 + 0.1*exp(-(x/1um)^2 - ((y + 3um)/1um)^2)"

[[task.body]]
type = "difference"
material = "si"
of = [
  { type = "box", center = [0, "3 um", "top + 0.5 um"], size = ["2 um", "0.8 um", "0.4 um"] },
  { type = "cylinder", base = [0, "3 um", "top"], top = [0, "3 um", "top + 1 um"], radius = "0.2 um" },
]
"#;

#[test]
fn bodies_in_a_job_are_checked_placed_and_previewed() {
    let job = Job::parse(JOB).unwrap();
    check(&job).unwrap();
    let Event::Scene { layers, shapes, .. } = preview(&job).unwrap() else {
        panic!("a structure's preview is a scene")
    };
    // the bodies' bands first, then the stack's layers, its top last
    assert_eq!(layers.last().unwrap().name, "Si");
    for k in 1..=7 {
        let name = format!("body {k}");
        assert!(
            layers.iter().any(|l| l.name.starts_with(&name)),
            "{name}: {:?}",
            layers.iter().map(|l| &l.name).collect::<Vec<_>>()
        );
        assert!(shapes.iter().any(|s| s.layer.starts_with(&name)), "{name}");
    }
    // a box, a cylinder along z and a difference of them are one band each
    assert_eq!(
        layers
            .iter()
            .filter(|l| l.name.starts_with("body 3"))
            .count(),
        1
    );
    assert_eq!(
        layers
            .iter()
            .filter(|l| l.name.starts_with("body 4"))
            .count(),
        1
    );
    assert_eq!(
        layers
            .iter()
            .filter(|l| l.name.starts_with("body 7"))
            .count(),
        1
    );
    assert!(
        layers
            .iter()
            .filter(|l| l.name.starts_with("body 1"))
            .count()
            > 1
    );
    let task: StructureTask = params::task(&job).unwrap();
    let s = task.structure().unwrap();
    assert_eq!(s.bodies().len(), 7);
    let top = 2.22;
    let lam = Wavelength::um(1.55).unwrap();
    let at = |x: f64, y: f64, z: f64| {
        s.material_at(Point::um(x, y), Length::um(z))
            .name()
            .to_owned()
    };
    let eps = |x: f64, y: f64, z: f64| {
        s.permittivity_at(Point::um(x, y), Length::um(z), lam)
            .unwrap()
            .re
    };
    assert_eq!(at(-2.0, -2.0, top + 0.5), "Si");
    assert_eq!(at(-2.0, -2.0, top + 0.85), "SiO2");
    assert_eq!(at(2.5, -2.0 + 0.5 * 0.5, top + 0.5), "Si3N4");
    assert_eq!(eps(0.0, 0.0, top + 0.3), 4.0);
    assert_eq!(at(2.0, 2.0, top + 0.9), "Si");
    assert_eq!(at(-1.9, 2.0, top + 0.9), "SiO2");
    assert_eq!(at(-2.0, 2.0, top + 0.9), "Si");
    assert_eq!(at(-2.0, 2.0, top + 0.1), "Si");
    // the graded body: 1.6 at its axis, its walls leaning out going down from its top face
    assert!((eps(0.0, -3.0, top + 0.2) - 2.56).abs() < 1e-12);
    let reach = 0.2 / 80f64.to_radians().tan();
    let n = 1.5 + 0.1 * (-(0.8f64 + 0.5 * reach).powi(2)).exp();
    assert!((eps(0.8 + 0.5 * reach, -3.0, top + 0.2) - n * n).abs() < 1e-12);
    assert_ne!(at(0.8 + 1.5 * reach, -3.0, top + 0.2), "Si");
    assert_eq!(at(0.0, 3.0, top + 0.5), "SiO2");
    assert_eq!(at(0.5, 3.0, top + 0.5), "Si");
}

#[test]
fn what_a_body_in_a_job_refuses() {
    let bad = |from: &str, to: &str, says: &str| {
        let text = JOB.replace(from, to);
        let job = Job::parse(&text).unwrap();
        let e = check(&job).unwrap_err().to_string();
        assert!(e.contains(says), "{e}");
    };
    bad(
        "type = \"sphere\"",
        "type = \"torus\"",
        "[[task.body]] #1: unknown type \"torus\"",
    );
    bad(
        "radius = \"r\"\nmaterial = \"si\"\n\n[[task.body]]\ntype = \"ellipsoid\"",
        "material = \"si\"\n\n[[task.body]]\ntype = \"ellipsoid\"",
        "#1 (sphere), radius: is needed",
    );
    bad(
        "permittivity = 4",
        "permittivity = 4\nmaterial = \"si\"",
        "give one medium",
    );
    bad(
        "index = \"1.5 + 0.1*exp(-(x/1um)^2 - ((y + 3um)/1um)^2)\"",
        "index = \"1.5 + x\"",
        "#6 (extrusion), index:",
    );
    bad(
        "angle = \"30 deg\"",
        "angle = \"30 deg\"\nradius = \"r\"",
        "radius isn't for this type",
    );
    bad(
        "angle = \"30 deg\"",
        "angle = \"30\"",
        "angle: needs an angle",
    );
    bad(
        "material = \"si3n4\"",
        "material = \"unobtainium\"",
        "unobtainium",
    );
    bad(
        "radii = [\"0.5 um\", 0]",
        "radii = [0, 0]",
        "#5 (cone): needs finite ends",
    );
}

#[test]
fn a_2d_fdtd_job_refuses_a_body() {
    let text = r#"
name = "body-2d"
[task]
kind = "fdtd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 1.0]
y_um = [-1.0, 1.0]
step_nm = 50.0
[[task.body]]
type = "sphere"
center = [0, 0, "2.11 um"]
radius = "200 nm"
material = "si"
[[task.source]]
type = "mode"
normal = "x"
at_um = -0.8
"#;
    let job = Job::parse(text).unwrap();
    let e = check(&job).unwrap_err().to_string();
    assert!(e.contains("for a 3D job"), "{e}");
}
