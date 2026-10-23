//! Custom stacks, partial etches and sidewalls in jobs: checked, drawn and solved.

use super::*;
use crate::run::replay;

/// A rib on SOI along x: a custom stack of the catalogue's materials, the rib etched `etch`.
fn rib_job(kind: &str, etch: &str, extra: &str) -> String {
    format!(
        r#"
name = "rib"

[task]
kind = "{kind}"
wavelength_um = 1.55
layer = "Si"
step_nm = 20.0
{extra}

[task.parameters]
core = "220 nm"

[task.stack]
substrate = "si"
cladding = "sio2"

[[task.stack.layer]]
name = "BOX"
thickness = "2 um"
material = "sio2"
background = "sio2"

[[task.stack.layer]]
name = "Si"
thickness = "core"
material = "si"
background = "sio2"

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size = ["10 um", "500 nm"]
{etch}
"#
    )
}

const MODES: &str = r#"propagation = "x"
y_um = [-1.5, 1.5]
modes = 1"#;

fn modes_index(text: &str, tag: &str) -> f64 {
    let dir = std::env::temp_dir().join(format!("photonoxide-etch-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let job = Job::parse(text).unwrap();
    check(&job).unwrap();
    let mut run = Run::create(&dir, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    events
        .iter()
        .find_map(|e| match e {
            Event::Mode {
                effective_index, ..
            } => Some(effective_index[0]),
            _ => None,
        })
        .unwrap()
}

#[test]
fn a_rib_on_a_custom_stack_is_checked_drawn_and_solved() {
    let rib = rib_job(
        "modes",
        r#"etch = { depth = "150 nm", sidewall = "80 deg" }"#,
        MODES,
    );
    let job = Job::parse(&rib).unwrap();
    check(&job).unwrap();
    // drawn: the etched layer in bands, the slab's the silicon all through
    let Event::Scene { layers, shapes, .. } = preview(&job).unwrap() else {
        panic!("a scene")
    };
    let names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(
        names,
        ["BOX", "Si", "Si 2", "Si 3", "Si 4", "Si 5"],
        "{names:?}"
    );
    let slab = layers.last().unwrap();
    assert_eq!(slab.background.material, slab.material.material);
    assert!(
        (slab.z_um[1] - slab.z_um[0] - 0.07).abs() < 1e-12,
        "{:?}",
        slab.z_um
    );
    // the rib's outline widens going down: 500 nm at the top, 150 nm cot 80° more each side
    // at the bottom band's middle
    let width = |layer: &str| {
        let o = &shapes.iter().find(|s| s.layer == layer).unwrap().outline;
        let ys: Vec<f64> = o.iter().map(|p| p[1]).collect();
        ys.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - ys.iter().copied().fold(f64::INFINITY, f64::min)
    };
    let run = 1.0 / 80f64.to_radians().tan();
    assert!((width("Si") - (0.5 + 2.0 * 0.01875 * run)).abs() < 1e-9);
    assert!((width("Si 4") - (0.5 + 2.0 * 0.13125 * run)).abs() < 1e-9);
    assert!(!shapes.iter().any(|s| s.layer == "Si 5"));
    // solved: more silicon than the strip, less than the film
    let n_rib = modes_index(&rib, "rib");
    let strip = rib_job("modes", "", MODES);
    let n_strip = modes_index(&strip, "strip");
    let vertical = rib_job("modes", r#"etch = { depth_um = 0.15 }"#, MODES);
    let n_vertical = modes_index(&vertical, "vertical");
    assert!(
        n_strip < n_vertical && n_vertical < n_rib,
        "{n_strip} {n_vertical} {n_rib}"
    );
    let slab = crate::mode::slab::Slab::new(1.444, 3.476, 1.444, Length::nm(220.0)).unwrap();
    let film = slab.modes(crate::mode::Polarization::Te, Wavelength::um(1.55).unwrap())[0]
        .effective_index();
    assert!(n_rib < film, "{n_rib} {film}");
}

#[test]
fn a_strip_drawn_through_its_etch_is_the_strip_it_always_was() {
    // a full etch with vertical walls, said or not, is the same job to the bit
    let plain = rib_job("modes", "", MODES);
    let said = rib_job(
        "modes",
        r#"etch = { depth = "220 nm", sidewall = "90 deg" }"#,
        MODES,
    );
    assert_eq!(modes_index(&plain, "plain"), modes_index(&said, "said"));
    // and the scene's layers are the stack's
    let Event::Scene { layers, .. } = preview(&Job::parse(&said).unwrap()).unwrap() else {
        panic!("a scene")
    };
    assert_eq!(layers.len(), 2);
}

#[test]
fn a_path_takes_an_etch_too() {
    let text = r#"
name = "etched-bend"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-2.0, 6.0]
y_um = [-2.0, 6.0]
step_nm = 50.0

[[task.path]]
layer = "Si"
start = ["-1 um", 0]
width = "500 nm"
segments = [{ straight = "1 um" }, { arc = "90 deg", radius = "4 um" }]
etch = { depth = "150 nm", sidewall = "85 deg" }
"#;
    let job = Job::parse(text).unwrap();
    check(&job).unwrap();
    let Event::Scene { layers, shapes, .. } = preview(&job).unwrap() else {
        panic!("a scene")
    };
    assert_eq!(layers.len(), 6, "{layers:?}");
    assert_eq!(shapes.len(), 4);
}

#[test]
fn bad_etches_and_stacks_are_refused_by_the_check() {
    let refused = |text: String, words: &str| {
        let e = check(&Job::parse(&text).unwrap()).unwrap_err().to_string();
        assert!(e.contains(words), "{e}");
    };
    refused(
        rib_job("modes", r#"etch = { depth = "300 nm" }"#, MODES),
        "at most its 0.22 um",
    );
    refused(
        rib_job("modes", r#"etch = { sidewall = "95 deg" }"#, MODES),
        "at most 90 degrees",
    );
    refused(
        rib_job("modes", r#"etch = { sidewall = "85" }"#, MODES),
        "needs an angle",
    );
    refused(
        rib_job("modes", r#"etch = { depth_um = 0.1, deep = 1 }"#, MODES),
        "unknown field",
    );
    refused(
        rib_job("modes", "", &format!("{MODES}\ncore_nm = 220.0")),
        "a named stack's",
    );
    // a 2D job's etches are on its own layer
    let fdfd = r#"x_um = [-2.0, 2.0]
y_um = [-1.5, 1.5]

[[task.port]]
x_um = -1.0
side = "left"

[[task.rect]]
layer = "BOX"
center_um = [0.0, 0.0]
size_um = [1.0, 1.0]
etch = { depth_um = 0.5 }"#;
    refused(rib_job("fdfd", "", fdfd), "on its own layer, Si");
}

#[test]
fn a_ribs_plane_takes_its_slab_beside_it() {
    // the effective index method's columns: the film's over the rib, the slab's beside it,
    // between them under a sloped wall
    let text = rib_job(
        "fdfd",
        r#"etch = { depth = "150 nm", sidewall = "80 deg" }"#,
        r#"x_um = [-2.0, 2.0]
y_um = [-1.5, 1.5]

[[task.port]]
x_um = -1.0
side = "left""#,
    );
    let job = Job::parse(&text).unwrap();
    check(&job).unwrap();
    #[derive(serde::Deserialize)]
    struct Drawn {
        stack: StackField,
        rect: Vec<RectSpec>,
    }
    let task: Drawn = params::task(&job).unwrap();
    let s = draw(
        named_stack(&task.stack, None, None).unwrap(),
        &task.rect,
        &[],
        &[],
    )
    .unwrap();
    let lam = Wavelength::um(1.55).unwrap();
    let te = crate::mode::Polarization::Te;
    let eps = fdfd::etched_plane(&s, "Si", (te, 0), lam).unwrap();
    let si = crate::material::silicon();
    let (core, ox) = (
        si.refractive_index(lam).unwrap().re,
        crate::material::silica().refractive_index(lam).unwrap().re,
    );
    let film = Slab::new(ox, core, ox, Length::nm(220.0))
        .unwrap()
        .modes(te, lam)[0]
        .effective_index();
    let slab = Slab::new(ox, core, ox, Length::nm(70.0))
        .unwrap()
        .modes(te, lam)[0]
        .effective_index();
    assert!((eps(0.0, 0.0) - film * film).abs() < 1e-12);
    assert!(
        (eps(0.0, 1.0) - slab * slab).abs() < 1e-12,
        "{} {}",
        eps(0.0, 1.0),
        slab * slab
    );
    let foot = 0.25 + 0.15 / 80f64.to_radians().tan();
    let mid = eps(0.0, 0.5 * (0.25 + foot));
    assert!(mid < film * film && mid > slab * slab);
}

use crate::mode::slab::Slab;
