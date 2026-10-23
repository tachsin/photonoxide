//! A static job's `electro_optic` table ([`super::pockels`]): Vπ·L of Wang et al. 2018's
//! thin-film modulator from a job, against the library's case, and what it refuses.

use super::*;
use crate::run::replay;

/// Wang et al. 2018's cross-section (the `pockels/wang-2018` case's): a 700 nm x-cut film
/// etched 400 nm to a rib 900 nm wide on top, 70° walls, on 2 µm of oxide over silicon, oxide
/// over it all (the case has air 1.5 µm over the slab), the 3.5 µm gap between a ground electrode 15 µm wide and a signal
/// one to the window's mirror 5 µm on, both 0.3 µm thick on the slab. The light along x, its c
/// axis across (the chip's y): an x-cut with the crystal's y along the chip's x.
pub(super) const WANG: &str = r#"
name = "tfln-modulator"
timeout_minutes = 10

[task]
kind = "static"
physics = "electrostatic"
y_um = [-16.75, 6.75]
z_um = [-20.0, 23.8]
step_nm = 1000.0
fine_nm = 20.0
growth = 1.15

[task.stack]
substrate = "si"
cladding = "sio2"

[[task.stack.layer]]
name = "BOX"
thickness_um = 2.0
material = "sio2"
background = "sio2"

[[task.stack.layer]]
name = "LN"
thickness_um = 0.7
material = { id = "linbo3", orientation = "x-cut" }
background = "sio2"

[[task.rect]]
layer = "LN"
center_um = [0.0, 0.0]
size_um = [10.0, 0.9]
etch = { depth_um = 0.4, sidewall_deg = 70.0 }

[task.permittivity]
Si = 11.7
SiO2 = 3.9

[task.boundary]
left = "insulated"
right = "insulated"

[[task.conductor]]
y_um = [-17.75, -1.75]
z_um = [2.3, 2.6]
potential_v = 0.0

[[task.conductor]]
y_um = [1.75, 7.75]
z_um = [2.3, 2.6]
potential_v = 1.0

[task.electro_optic]
layer = "LN"
entry = "linbo3"
tensor = "thin film (x-cut LNOI), clamped, 1550 nm"
complete_from = "bulk, clamped, 633 nm"
wavelength_um = 1.55
y_um = [-3.0, 3.0]
z_um = [0.5, 4.0]
step_nm = 20.0
"#;

fn run_job(tag: &str, text: &str) -> Vec<Event> {
    let dir =
        std::env::temp_dir().join(format!("photonoxide-pockels-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let job = Job::parse(text).unwrap();
    check(&job).unwrap();
    let mut run = Run::create(&dir, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events = replay(run.dir()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    events
}

fn values(events: &[Event]) -> Vec<statics::StaticValue> {
    events
        .iter()
        .find_map(|e| match e {
            Event::StaticResult { values, .. } => Some(values.clone()),
            _ => None,
        })
        .expect("a result")
}

fn named(values: &[statics::StaticValue], name: &str) -> f64 {
    values
        .iter()
        .find(|v| v.name.starts_with(name))
        .unwrap_or_else(|| panic!("no {name} in {values:?}"))
        .value
}

#[test]
fn a_job_gives_wang_et_al_s_vpi_l_as_the_library_s_case_does() {
    let v = values(&run_job("wang", WANG));
    let vpi = named(&v, "Vpi L (push-pull");
    // the library's case with 3 µm of oxide over the slab on the same grids (20 nm), its own
    // sampling of the cells: 1.863 (1.874 here)
    assert!((vpi - 1.863).abs() < 0.02, "{vpi} {v:?}");
    let first = named(&v, "dn_eff/dV by first-order");
    let resolved = named(&v, "dn_eff/dV by re-solve");
    assert!((resolved / first - 1.0).abs() < 3e-3, "{first} {resolved}");
    assert!(named(&v, "mode's TE fraction") > 0.9);
}

#[test]
fn a_job_checks_vpi_l_on_coarser_grids() {
    let text = WANG.replace(
        "step_nm = 20.0\n",
        "step_nm = 40.0\nresolve = false\nconvergence = { tolerance = 0.05, grids = 2 }\n",
    );
    let ev = run_job("convergence", &text);
    let c = ev
        .iter()
        .find_map(|e| match e {
            Event::GridConvergence { check, .. } => Some(check.clone()),
            _ => None,
        })
        .expect("a grid study");
    assert_eq!(c.steps_um, vec![0.08, 0.04]);
    assert!(c.converged(), "{}", c.reason);
}

#[test]
fn what_the_electro_optic_table_refuses() {
    for (from, to, says) in [
        (
            "tensor = \"thin film",
            "tensor = \"no such",
            "has no tensor",
        ),
        (
            "complete_from = \"bulk, clamped, 633 nm\"\n",
            "",
            "doesn't give",
        ),
        (
            "layer = \"LN\"\nentry",
            "layer = \"BOX\"\nentry",
            "isn't an oriented crystal",
        ),
        (
            "physics = \"electrostatic\"",
            "physics = \"heat\"",
            "electro_optic is for",
        ),
        ("potential_v = 1.0", "potential_v = 0.0", "two potentials"),
        (
            "wavelength_um = 1.55\n",
            "wavelength_um = 1.55\ndrive = \"sideways\"\n",
            "push-pull",
        ),
    ] {
        assert!(WANG.contains(from), "{from}");
        let job = Job::parse(&WANG.replace(from, to)).unwrap();
        let e = match check(&job) {
            Err(e) => e.to_string(),
            Ok(()) => {
                let dir = std::env::temp_dir()
                    .join(format!("photonoxide-pockels-refuse-{}", std::process::id()));
                let mut run = Run::create(&dir, &job).unwrap();
                let r = execute(&job, &mut run, &Stop::new(None));
                let _ = std::fs::remove_dir_all(&dir);
                r.expect_err("a refusal").to_string()
            }
        };
        assert!(e.contains(says), "{says}: {e}");
    }
}
