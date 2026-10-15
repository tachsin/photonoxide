//! `[[task.component]]` in jobs: drawn, checked and previewed, their ports found by name.

use toml::{Table, Value};

use super::components::{self, EXPRESSIONS};
use super::{Event, StructureTask, check, params, preview};
use crate::geometry::Point;
use crate::run::Job;
use crate::units::Length;

#[test]
fn components_with_expressions_are_the_generators_with_numbers() {
    assert_eq!(components::expression_errors(), 0);
    let job = Job::parse(EXPRESSIONS).unwrap();
    check(&job).unwrap();
    let Event::Scene { shapes, .. } = preview(&job).unwrap() else {
        panic!("a scene")
    };
    assert!(shapes.len() > 30, "{}", shapes.len());
    // drawn on the structure: the ring's guide is silicon, its middle isn't
    let task: StructureTask = params::task(&job).unwrap();
    let s = task.structure().unwrap();
    let (_, bottom, top) = s.stack().layer("Si").unwrap();
    let z = (bottom + top) / 2.0;
    let at = |x: f64, y: f64| s.material_at(Point::um(x, y), z).name().to_owned();
    assert_eq!(at(0.0, 0.0), "Si", "the bus");
    assert_eq!(at(0.0, 0.5 + 0.2 + 0.25), "Si", "the ring's foot");
    assert_ne!(at(0.0, 0.5 + 0.2 + 5.0), "Si", "the ring's middle");
    // the lead continues the bus beyond the ring's through port
    assert_eq!(at(6.5 + 2.9, 0.0), "Si");
}

#[test]
fn a_port_by_name_is_the_port_written_out() {
    assert_eq!(components::port_errors(), 0);
}

/// The task of `text`, resolved.
fn resolved(text: &str) -> Table {
    params::resolve(Job::parse(text).unwrap().task(), None).unwrap()
}

#[test]
fn fdtd_sources_and_monitors_find_their_ports() {
    let text = r#"
name = "ring-fdtd"

[task]
kind = "fdtd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-6.0, 6.0]
y_um = [-2.0, 7.0]
step_nm = 50.0

[[task.component]]
type = "ring"
name = "ring"
width = "500 nm"
radius = "2.5 um"
gap = "200 nm"
drop_gap = "200 nm"
reach = "4.5 um"

[[task.component]]
type = "crossing"
name = "x"
width = "400 nm"
arm = "1 um"
origin = ["0 um", "-1.5 um"]

[[task.source]]
type = "mode"
port = "ring.in"

[[task.monitor]]
type = "mode"
port = "ring.drop"

[[task.monitor]]
type = "mode"
name = "down"
port = "x.o4"
"#;
    let task = resolved(text);
    let source = &task["source"][0];
    assert_eq!(source["normal"].as_str(), Some("x"));
    assert_eq!(source["at_um"].as_float(), Some(-4.5));
    assert_eq!(source["direction"].as_str(), Some("+"));
    // the bus's window: its width and 1 µm, but no lower than the task's window
    assert_eq!(source["y_um"][0].as_float(), Some(-1.25));
    assert_eq!(source["y_um"][1].as_float(), Some(1.25));
    let drop = &task["monitor"][0];
    assert_eq!(drop["name"].as_str(), Some("ring.drop"));
    // the drop bus at 2(w + R) + 2 gap: its window down to half way to nothing, up to the
    // task's window
    let yd = 2.0 * (0.5 + 2.5) + 0.4;
    assert_eq!(drop["y_um"][0].as_float(), Some(yd - 1.25));
    assert_eq!(drop["y_um"][1].as_float(), Some(7.0));
    // a port facing down: a plane normal to y, its window along x
    let down = &task["monitor"][1];
    assert_eq!(down["name"].as_str(), Some("down"));
    assert_eq!(down["normal"].as_str(), Some("y"));
    assert_eq!(down["at_um"].as_float(), Some(-2.5));
    assert_eq!(down["x_um"][0].as_float(), Some(-1.2));
    check(&Job::parse(text).unwrap()).unwrap_or_else(|e| {
        // the crossing's port is at the window's edge, in its PML: the check says so, naming it
        assert!(e.to_string().contains("monitor"), "{e}");
    });
}

#[test]
fn two_ports_in_a_column_share_it() {
    // a directional coupler's two left ports: each window half way to the other
    let text = r#"
name = "dc"

[task]
kind = "fdfd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 11.0]
y_um = [-3.0, 3.0]
step_nm = 50.0

[[task.component]]
type = "directional_coupler"
name = "dc"
width = "500 nm"
gap = "300 nm"
length = "10 um"

[[task.port]]
port = "dc.o1"

[[task.port]]
port = "dc.o2"
"#;
    let task = resolved(text);
    let c = (0.3 + 0.5) / 2.0;
    let one = &task["port"][0];
    assert_eq!(one["x_um"].as_float(), Some(0.0));
    assert_eq!(one["side"].as_str(), Some("left"));
    assert_eq!(one["y_um"][0].as_float(), Some(-c - c));
    assert_eq!(one["y_um"][1].as_float(), Some(0.0));
    assert_eq!(task["port"][1]["y_um"][0].as_float(), Some(0.0));
}

#[test]
fn what_a_component_in_a_job_refuses() {
    let base = r#"
name = "refused"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-10.0, 10.0]
y_um = [-10.0, 10.0]
step_nm = 100.0

[task.parameters]
w = "500 nm"

[[task.component]]
type = "ring"
name = "ring"
width = "w"
radius = "5 um"
gap = "200 nm"
"#;
    check(&Job::parse(base).unwrap()).unwrap();
    let bad = |from: &str, to: &str, says: &str| {
        let text = base.replace(from, to);
        assert_ne!(text, base, "{from}");
        let e = check(&Job::parse(&text).unwrap()).unwrap_err().to_string();
        assert!(e.contains(says), "{e}");
    };
    bad(
        "type = \"ring\"",
        "type = \"resonator\"",
        "[[task.component]] #1: unknown type \"resonator\": straight, taper",
    );
    bad(
        "gap = \"200 nm\"",
        "gap = \"200\"",
        "[[task.component]] #1 (ring \"ring\"), gap: needs a length, got a number",
    );
    bad(
        "gap = \"200 nm\"",
        "gap = \"200 nm\"\npitch = \"1 um\"",
        "pitch isn't for a ring: it takes width, radius, gap",
    );
    bad(
        "gap = \"200 nm\"\n",
        "",
        "(ring \"ring\"), gap: is needed: a length",
    );
    bad(
        "gap = \"200 nm\"",
        "gap = \"-200 nm\"",
        "(ring \"ring\"): a ring's gap must be positive",
    );
    bad(
        "width = \"w\"",
        "width = \"wdth\"",
        "(ring \"ring\"), width: column 1: unknown name",
    );
    bad(
        "name = \"ring\"",
        "name = \"my.ring\"",
        "a name is a word without dots",
    );
    let with = |extra: &str| format!("{base}\n{extra}");
    let refused = |text: String, says: &str| {
        let e = check(&Job::parse(&text).unwrap()).unwrap_err().to_string();
        assert!(e.contains(says), "{e}");
    };
    refused(
        with(
            "[[task.component]]\ntype = \"straight\"\nwidth = \"w\"\nlength = \"1 um\"\nconnect = [\"o1\", \"ring.drop\"]\n",
        ),
        "connect: the ring \"ring\" has no port drop: its ports are in, through",
    );
    refused(
        with(
            "[[task.component]]\ntype = \"straight\"\nwidth = \"w\"\nlength = \"1 um\"\nconnect = [\"o1\", \"rng.in\"]\n",
        ),
        "connect: no component named \"rng\" before it: there are ring",
    );
    refused(
        with(
            "[[task.component]]\ntype = \"straight\"\nname = \"ring\"\nwidth = \"w\"\nlength = \"1 um\"\n",
        ),
        "there is already a component named \"ring\"",
    );
    refused(
        with(
            "[[task.component]]\ntype = \"straight\"\nwidth = \"w\"\nlength = \"1 um\"\nconnect = [\"o1\", \"ring.in\"]\norigin = [0, 0]\n",
        ),
        "give it, or origin and rotate, not both",
    );
    refused(
        with("[[task.component]]\ntype = \"bend\"\nwidth = \"w\"\nradius = \"5 um\"\nangle = 90\n"),
        "angle: needs an angle, got a number",
    );
    refused(
        with(
            "[[task.component]]\ntype = \"spiral\"\nwidth = \"w\"\npitch = \"2 um\"\ninner_radius = \"5 um\"\nturns = 2.5\n",
        ),
        "turns: must be a whole number",
    );
    refused(
        with(
            "[[task.component]]\ntype = \"ring\"\nname = \"other\"\nlayer = \"SiN\"\nwidth = \"w\"\nradius = \"5 um\"\ngap = \"200 nm\"\n",
        ),
        "[[task.component]] #2: the stack has no layer SiN",
    );
    // ports by name in an fdfd job
    let fdfd = base
        .replace("kind = \"structure\"", "kind = \"fdfd\"")
        .replace("step_nm = 100.0", "step_nm = 100.0\n");
    refused(
        format!("{fdfd}\n[[task.port]]\nport = \"ring.out\"\n"),
        "[[task.port]] #1: no port \"ring.out\": the ports are ring.in, ring.through",
    );
    refused(
        format!("{fdfd}\n[[task.port]]\nport = \"ring.in\"\nside = \"left\"\n"),
        "give port or side, not both",
    );
    let turned = fdfd.replace("gap = \"200 nm\"", "gap = \"200 nm\"\nrotate = \"90 deg\"");
    refused(
        format!("{turned}\n[[task.port]]\nport = \"ring.in\"\n"),
        "the port \"ring.in\" faces 270°: an fdfd job's ports are columns normal to x",
    );
    let plain = base.replace(
        "[[task.component]]\ntype = \"ring\"\nname = \"ring\"\nwidth = \"w\"\nradius = \"5 um\"\ngap = \"200 nm\"\n",
        "",
    );
    refused(
        format!(
            "{}\n[[task.port]]\nport = \"ring.in\"\n",
            plain.replace("kind = \"structure\"", "kind = \"fdfd\"")
        ),
        "the task's components have no named ports",
    );
}

#[test]
fn a_component_takes_an_etch_and_is_swept() {
    let text = r#"
name = "etched"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 1.0]
step_nm = 50.0
"#;
    // a modes job draws no components: refused by name
    let modes = format!(
        "{text}\n[[task.component]]\ntype = \"straight\"\nwidth = \"500 nm\"\nlength = \"1 um\"\n"
    );
    let e = check(&Job::parse(&modes).unwrap()).unwrap_err().to_string();
    assert!(e.contains("component"), "{e}");
    let structure = r#"
name = "etched"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.0, 11.0]
y_um = [-2.0, 2.0]
step_nm = 50.0

[task.parameters]
w = "500 nm"

[[task.component]]
type = "straight"
width = "w"
length = "10 um"
etch = { depth = "70 nm" }
"#;
    let job = Job::parse(structure).unwrap();
    check(&job).unwrap();
    let task: StructureTask = params::task(&job).unwrap();
    let s = task.structure().unwrap();
    let (_, bottom, top) = s.stack().layer("Si").unwrap();
    let floor = Length::um(bottom.to_um() + 0.05);
    assert_eq!(s.material_at(Point::um(5.0, 1.5), floor).name(), "Si");
    assert_ne!(
        s.material_at(Point::um(5.0, 1.5), top - Length::um(0.01))
            .name(),
        "Si"
    );
    // at a sweep's point, the component follows the parameter
    let table = params::resolve(job.task(), Some(("w", 0.8))).unwrap();
    let entries: Vec<Table> = table["component"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_table().unwrap().clone())
        .collect();
    let placed = components::build_all(&entries, None).unwrap();
    assert_eq!(placed[0].drawing.ports()[0].width, Length::um(0.8));
    assert!(matches!(entries[0].get("set"), Some(Value::Array(_))));
}
