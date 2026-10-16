use super::*;
use crate::job::chip_checks::{
    adjoint_difference, closed_form_error, mzi, mzi_problem, study_differences,
};
use crate::job::optimize::Optimization;
use crate::job::{Aim, Problem, Variable, check, optimize_from_chip};
use crate::run::replay;

#[test]
fn a_chip_round_trips_through_its_table() {
    let chip = mzi().instance(ChipInstance::component("t", "terminator").with_value("x", 1.0));
    let back = ChipDesign::from_table(&chip.to_table()).unwrap();
    assert_eq!(back, chip);
}

#[test]
fn a_spectrum_job_checks_and_refuses_what_isnt_a_chip() {
    let job = spectrum_from_chip("mzi", &mzi(), (1.5, 1.6, 11), 1.0).unwrap();
    check(&job).unwrap();
    let mut bad = mzi();
    bad.connections.pop();
    let e = spectrum_from_chip("mzi", &bad, (1.5, 1.6, 11), 1.0).unwrap_err();
    assert!(
        e.to_string().contains("neither connected nor exposed"),
        "{e}"
    );
    let text = job
        .text()
        .replace("kind = \"spectrum\"", "kind = \"modes\"");
    let e = check(&Job::parse(&text).unwrap()).unwrap_err();
    assert!(e.to_string().contains("\"spectrum\""), "{e}");
    let bad = mzi().instance(ChipInstance::component("x", "laser"));
    let e = spectrum_from_chip("mzi", &bad, (1.5, 1.6, 11), 1.0).unwrap_err();
    assert!(
        e.to_string().contains("coupler, phase-shifter, terminator"),
        "{e}"
    );
    let e = mzi().resolve("split.gapp").unwrap_err();
    assert!(e.to_string().contains("gap"), "{e}");
    let e = mzi().resolve("splitter.gap").unwrap_err();
    assert!(e.to_string().contains("split, upper"), "{e}");
    let both = job
        .text()
        .replace("[task]", "[device]\nkind = \"waveguide\"\n\n[task]");
    let e = check(&Job::parse(&both).unwrap()).unwrap_err();
    assert!(e.to_string().contains("not both"), "{e}");
}

/// The analytic test: the chip of devices against the closed form of an MZI, the product of its
/// couplers' and arms' transfer matrices, each from its own device.
#[test]
fn an_mzi_of_devices_is_its_closed_form() {
    let e = closed_form_error().unwrap();
    assert!(e < 1e-13, "{e}");
}

/// Each point of a chip's study is the bits of its own chip's spectrum.
#[test]
fn a_chip_study_is_its_points_to_the_bit() {
    assert_eq!(study_differences().unwrap(), 0);
}

#[test]
fn a_chips_spectrum_job_records_its_instances_and_replays() {
    let job = spectrum_from_chip("mzi", &mzi(), (1.54, 1.56, 3), 1.0).unwrap();
    let dir =
        std::env::temp_dir().join(format!("photonoxide-chip-spectrum-{}", std::process::id()));
    let mut run = Run::create(&dir, &job).unwrap();
    crate::job::execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let Event::Chip { instances, ports } = &events[1] else {
        panic!("the chip after its start: {:?}", events[1])
    };
    assert_eq!(ports, &["in1", "in2", "out1", "out2"]);
    assert_eq!(instances[0].kind, "directional coupler");
    assert_eq!(instances[0].fidelity, "analytic");
    assert!(instances.iter().all(|i| !i.grid.is_empty()));
    let s: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::SParameters { .. }))
        .collect();
    assert_eq!(s.len(), 3);
    let expected = spectrum(&mzi(), &band(1.54, 1.56, Some(3), 3).unwrap()).unwrap();
    for (e, x) in s.iter().zip(&expected) {
        let Event::SParameters { s, .. } = e else {
            unreachable!()
        };
        assert_eq!(s, x);
    }
}

#[test]
fn a_chips_exact_derivatives_agree_with_differences() {
    let mut p = mzi_problem().unwrap();
    let checks = p.check_gradients().unwrap();
    assert!(!checks.is_empty());
    for c in &checks {
        assert!(c.agrees, "{c:?}");
    }
    // the coupler's and the arm's lengths exact, the gap differenced
    assert_eq!(p.derivatives(), ["exact", "exact", "differences"]);
    let e = p.evaluate(&[20.0, 150.0, 0.2], true);
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_eq!(e.gradients[0].len(), 3);
    assert_eq!(e.jacobian.len(), 2);
    assert!(e.gradients[0].iter().all(|g| g.is_finite()));
}

#[test]
fn a_chips_gradient_is_the_circuit_adjoints_through_the_device() {
    let d = adjoint_difference().unwrap();
    assert!(d < 1e-9, "{d}");
}

#[test]
fn a_chip_refuses_what_its_optimization_cant_take() {
    let o = |v: Variable| {
        optimize_from_chip(
            "o",
            &mzi(),
            (1.55, 1.55, 1),
            &Optimization::new("lbfgsb", 1, 10)
                .variable(v)
                .objective(Aim::expression("T_out1_in1", crate::device::Goal::Maximize)),
            1.0,
        )
    };
    let e = o(Variable::new("split.length", 30.0, 40.0)).unwrap_err();
    assert!(e.to_string().contains("starts from the chip"), "{e}");
    let e = o(Variable::new("split.gap", 0.0, 0.3)).unwrap_err();
    assert!(e.to_string().contains("range"), "{e}");
    let e = optimize_from_chip(
        "o",
        &mzi(),
        (1.55, 1.55, 1),
        &Optimization::new("lbfgsb", 1, 10)
            .variable(Variable::new("split.length", 10.0, 30.0))
            .objective(Aim::declared("split")),
        1.0,
    )
    .unwrap_err();
    assert!(e.to_string().contains("declares no objectives"), "{e}");
    let e = optimize_from_chip(
        "o",
        &mzi(),
        (1.55, 1.55, 1),
        &Optimization::new("lbfgsb", 1, 10)
            .variable(Variable::new("split.length", 10.0, 30.0))
            .objective(Aim::declared("split").of_instance("splitter")),
        1.0,
    )
    .unwrap_err();
    assert!(e.to_string().contains("isn't an instance"), "{e}");
}

#[test]
fn a_chips_optimization_records_its_problem() {
    let mut p = mzi_problem().unwrap();
    let checks = p.check_gradients().unwrap();
    let job = optimize_from_chip(
        "r",
        &mzi(),
        (1.55, 1.55, 1),
        &Optimization::new("mma", 1, 10)
            .variable(Variable::new("split.length", 10.0, 30.0))
            .objective(Aim::expression("T_out1_in1", crate::device::Goal::Maximize)),
        1.0,
    )
    .unwrap();
    assert!(crate::job::optimizes(&job));
    let e = crate::job::execute(
        &job,
        &mut Run::create(&std::env::temp_dir().join("photonoxide-chip-refuse"), &job).unwrap(),
        &Stop::new(None),
    )
    .unwrap_err();
    assert!(e.to_string().contains("genoxide"), "{e}");
    let _ = std::fs::remove_dir_all(std::env::temp_dir().join("photonoxide-chip-refuse"));
    let dir = std::env::temp_dir().join(format!("photonoxide-chip-opt-{}", std::process::id()));
    let mut run = Run::create(&dir, &job).unwrap();
    let mut recorder = crate::job::Recorder::begin(&p, &job, &mut run, checks).unwrap();
    let x = [20.0, 150.0, 0.2];
    let e = p.evaluate(&x, true);
    recorder.record(&mut run, &x, &e).unwrap();
    recorder.finish(&p, &mut run, "evaluations").unwrap();
    let events: Vec<Event> = replay(run.dir()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(matches!(&events[0], Event::Started { kind, .. } if kind == "chip"));
    assert!(matches!(&events[1], Event::Chip { instances, .. } if instances.len() == 4));
    assert!(matches!(&events[2], Event::Optimization { device, .. } if device == "chip"));
    let Some(Event::Optimum { parameters, .. }) = events.last() else {
        panic!("an optimum")
    };
    assert_eq!(
        parameters,
        &vec![
            ("split.length".to_owned(), "20.0".to_owned()),
            ("upper.length".to_owned(), "150.0".to_owned()),
            ("combine.gap".to_owned(), "0.2".to_owned()),
        ]
    );
}

/// A ring's resonance can be sharp on the check's first step: the exact derivative of the chip's
/// drop in a bus coupler's length, which fourth-order differences reach only at a finer step, is
/// still checked and used.
#[test]
fn a_sharp_resonances_exact_derivative_is_checked_at_a_finer_step() {
    let text = include_str!("../../jobs/chip-little-rings.toml");
    let a = text.find("[[task.variable]]").unwrap();
    let b = text.find("[[task.objective]]").unwrap();
    let t = format!(
        "{}[[task.variable]]\nparameter = \"c0.length\"\nfrom = 0.6\nto = 0.9\n\n{}",
        &text[..a],
        &text[b..]
    );
    let mut p = Problem::of(&Job::parse(&t).unwrap()).unwrap();
    let checks = p.check_gradients().unwrap();
    assert!(checks.iter().all(|c| c.agrees), "{checks:?}");
    assert_eq!(p.derivatives(), ["exact"]);
}
