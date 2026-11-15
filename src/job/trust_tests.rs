//! What a job file says that it may not mean, which a run would otherwise answer with a number
//! and exit 0 (issue #783): a misspelled device parameter, a band outside where the analytic
//! fit holds or typed in nanometres, and a modes job's window, grid or empty solve (the
//! skeptic's scenarios, on `photonoxide new modes`'s strip).

use super::*;
use crate::run::{Format, Run, Stop};

fn yaml(text: &str) -> Job {
    Job::parse_as(text, Format::Yaml).unwrap()
}

/// A device job of `kind` with `parameters` (YAML lines under `parameters:`) and `task` (lines
/// under `task:`).
fn device_job(kind: &str, parameters: &str, task: &str) -> Job {
    yaml(&format!(
        "name: d\ndevice:\n  kind: {kind}\n  fidelity: analytic\n  parameters:\n{parameters}\ntask:\n{task}"
    ))
}

const SPECTRUM: &str = "  kind: spectrum\n  from_um: 1.5\n  to_um: 1.6\n  points: 11\n";

#[test]
fn a_misspelled_device_parameter_is_refused_with_the_nearest_name() {
    let job = device_job("directional coupler", "    gapp: \"0.4\"\n", SPECTRUM);
    let e = check(&job).unwrap_err().to_string();
    assert!(
        e.contains("a directional coupler has no parameter gapp (did you mean gap?); its parameters are width, thickness"),
        "{e}"
    );
    assert!(
        e.contains("no other parameter's expression uses gapp"),
        "{e}"
    );
    // R for the radius: a name the parameter begins
    let ring = device_job("all-pass ring", "    R: \"5\"\n", SPECTRUM);
    let e = check(&ring).unwrap_err().to_string();
    assert!(e.contains("no parameter R (did you mean radius?)"), "{e}");
    // nothing near: no guess
    let far = device_job("waveguide", "    colour: \"5\"\n", SPECTRUM);
    let e = check(&far).unwrap_err().to_string();
    assert!(e.contains("no parameter colour;"), "{e}");
    // a design alone is refused as well
    let alone = yaml("name: d\ndevice:\n  kind: waveguide\n  parameters:\n    widht: \"0.45\"\n");
    let e = check(&alone).unwrap_err().to_string();
    assert!(e.contains("widht (did you mean width?)"), "{e}");
    // a name of one's own that another parameter's expression uses is the design's to name
    let helper = device_job(
        "directional coupler",
        "    w: 450 nm\n    width: w\n    gap: w/2.5\n",
        SPECTRUM,
    );
    check(&helper).unwrap();
    assert!(warnings(&helper).is_empty());
}

#[test]
fn a_chip_warns_of_a_name_no_expression_uses() {
    let chip = |parameters: &str| {
        yaml(&format!(
            "name: c\ncircuit:\n  instance:\n    - name: coupler\n      component: coupler\n      \
             values: {{ coupling: 0.05 }}\n    - name: ring\n      device:\n        kind: \
             waveguide\n        fidelity: analytic\n        parameters: {{ {parameters} }}\n  \
             connections:\n    - [coupler.o3, ring.o1]\n    - [ring.o2, coupler.o2]\n  port:\n    \
             - {{ name: in, at: coupler.o1 }}\n    - {{ name: through, at: coupler.o4 }}\n\
             task:\n  kind: spectrum\n  from_um: 1.54\n  to_um: 1.56\n  points: 11\n"
        ))
    };
    // R is the chip's own, which length uses: no warning
    let good = chip("R: \"10\", length: \"2*pi*R\"");
    check(&good).unwrap();
    assert!(warnings(&good).is_empty(), "{:?}", warnings(&good));
    // lossy: used by nothing, near loss; the chip still runs
    let lossy = chip("R: \"10\", length: \"2*pi*R\", lossy: \"3\"");
    check(&lossy).unwrap();
    let w = warnings(&lossy);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        w[0].starts_with("ring: a waveguide has no parameter lossy (did you mean loss?)"),
        "{}",
        w[0]
    );
}

#[test]
fn an_analytic_band_outside_the_fit_or_in_nanometres_is_refused() {
    let band = |from: f64, to: f64| {
        format!("  kind: spectrum\n  from_um: {from}\n  to_um: {to}\n  points: 11\n")
    };
    let coupler = |task: &str| device_job("directional coupler", "    gap: \"0.2\"\n", task);
    check(&coupler(&band(1.5, 1.6))).unwrap();
    // nanometres in a micrometre field
    let e = check(&coupler(&band(1500.0, 1600.0)))
        .unwrap_err()
        .to_string();
    assert!(
        e.contains(
            "a directional coupler's analytic fit holds from 1.5 to 1.6 um, not 1500 to 1600 um"
        ),
        "{e}"
    );
    assert!(
        e.contains("reads as nanometres: write 1.5 to 1.6 um"),
        "{e}"
    );
    // far outside the fit: solve the modes
    let e = check(&coupler(&band(0.3, 5.0))).unwrap_err().to_string();
    assert!(e.contains("not 0.3 to 5 um: solve the modes (3D)"), "{e}");
    // a study and a fit refuse it too
    let sweep = coupler(
        "  kind: sweep\n  from_um: 1.2\n  to_um: 1.7\n  points: 5\n  axis:\n    - { parameter: gap, from: 0.2, to: 0.3, points: 2 }\n",
    );
    let e = check(&sweep).unwrap_err().to_string();
    assert!(e.contains("not 1.2 to 1.7 um"), "{e}");
    // the device's own band (its wavelength ± span/2) stays inside it
    check(&coupler("  kind: spectrum\n")).unwrap();
    // a 3D model is the solver's, over the band it samples: not this check's
    assert!(
        crate::device::check_analytic_band(
            "directional coupler",
            crate::circuit::Fidelity::ThreeD,
            &[0.3, 5.0]
        )
        .is_ok()
    );
}

#[test]
fn a_chip_band_outside_an_analytic_instance_names_it() {
    let job = yaml(
        "name: c\ncircuit:\n  instance:\n    - name: arm\n      device: { kind: waveguide, fidelity: analytic }\n  \
         port:\n    - { name: a, at: arm.o1 }\n    - { name: b, at: arm.o2 }\ntask:\n  kind: spectrum\n  \
         from_um: 1550\n  to_um: 1560\n  points: 11\n",
    );
    let e = check(&job).unwrap_err().to_string();
    assert!(
        e.contains("arm: a waveguide's analytic fit holds from 1.5 to 1.6 um"),
        "{e}"
    );
    assert!(e.contains("write 1.55 to 1.56 um"), "{e}");
}

#[test]
fn the_console_hears_of_grids_not_known_to_have_converged() {
    use crate::convergence::{Convergence, Study};
    let mut r = results::Results::collect(&[]);
    r.kind = "modes".into();
    assert!(r.convergence_lines().is_empty());
    // a modes sweep: each point on one grid
    r.sweep = Some(results::Sweep {
        parameter: "width".into(),
        points: vec![
            results::SweepPoint {
                value: 0.5,
                wavelength_um: 1.55,
                effective_indices: vec![[2.4, 0.0]],
                te_fractions: vec![1.0],
            };
            3
        ],
    });
    assert_eq!(
        r.convergence_lines(),
        [
            "convergence: unchecked: the 3 points of the sweep over width, each solved on one grid: no check"
        ]
    );
    r.sweep = None;
    // two modes on one grid, together; one not converged, with why
    let check = |quantity: &str, c: Convergence| results::GridCheck {
        label: quantity.into(),
        wavelength_um: 1.55,
        check: c,
    };
    let unconverged = Study::new("Re n_eff of mode 3")
        .on(0.04, 2.40)
        .unwrap()
        .on(0.02, 2.43)
        .unwrap()
        .check(1e-4)
        .unwrap();
    r.convergence = vec![
        check(
            "mode 1",
            Convergence::unchecked("Re n_eff of mode 1", 0.02, 2.44),
        ),
        check(
            "mode 2",
            Convergence::unchecked("Re n_eff of mode 2", 0.02, 1.8),
        ),
        check("mode 3", unconverged),
    ];
    let lines = r.convergence_lines();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(
        lines[0]
            .starts_with("convergence: not converged: Re n_eff of mode 3 at 1.55 um: two grids"),
        "{}",
        lines[0]
    );
    assert_eq!(
        lines[1],
        "convergence: unchecked: Re n_eff of mode 1, Re n_eff of mode 2 at 1.55 um: one grid \
         (0.02 um): no check ([task.convergence] checks it)"
    );
}

/// `photonoxide new modes`'s job (a 500 x 220 nm strip in oxide, 20 nm, y from -1.2 to 1.2 um)
/// with its lines `from` changed to `to`.
fn new_modes(changes: &[(&str, &str)]) -> Job {
    let mut text = templates::template("modes").unwrap().text.to_owned();
    for (from, to) in changes {
        assert!(text.contains(from), "the template has {from}");
        text = text.replace(from, to);
    }
    yaml(&text)
}

/// A modes job's run in memory: its modes' Re n_eff and its warnings.
fn modes_run(job: &Job) -> (Vec<f64>, Vec<String>) {
    check(job).unwrap();
    let mut run = Run::in_memory();
    execute(job, &mut run, &Stop::new(None)).unwrap();
    let (mut n, mut w) = (Vec::new(), Vec::new());
    for line in run.lines() {
        match serde_json::from_str::<Event>(line).unwrap() {
            Event::Mode {
                effective_index, ..
            } => n.push(effective_index[0]),
            Event::Warning { text } => w.push(text),
            _ => {}
        }
    }
    (n, w)
}

#[test]
fn a_modes_window_narrower_than_the_strip_is_warned_of() {
    // the template's own window: no warning, and its scheme said
    let (n, w) = modes_run(&new_modes(&[]));
    assert!(w.is_empty(), "{w:?}");
    assert!(n[0] > 2.4, "{n:?}");
    // the skeptic's 0.4 um window on the 500 nm strip: n_eff 2.2354 where 2.44 is right. Inside
    // the strip a wave along the window's side travels at the 220 nm film's index, above the
    // mode's: the field can't decay towards the side (no threshold: n_eff against that index)
    let (n, w) = modes_run(&new_modes(&[(
        "y: [-1.2 um, 1.2 um]",
        "y: [-0.2 um, 0.2 um]",
    )]));
    assert!(n[0] < 2.3, "{n:?}");
    assert!(
        w.iter().any(|w| w.starts_with("mode 1 of")
            && w.contains("its field doesn't decay towards the window's edge at y = -0.2 um")
            && w.contains("widen y_um")),
        "{w:?}"
    );
    // the 0.2 um window: no mode at all, said with why
    let (n, w) = modes_run(&new_modes(&[(
        "y: [-1.2 um, 1.2 um]",
        "y: [-0.1 um, 0.1 um]",
    )]));
    assert!(n.is_empty(), "{n:?}");
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        w[0].starts_with("no mode found: of the 2 modes solved none has n_eff above 1.44")
            && w[0].contains("a guide runs into the window's edge at y = -0.1 um and y = 0.1 um")
            && w[0].contains("widen y_um"),
        "{}",
        w[0]
    );
}

#[test]
fn a_modes_field_at_the_window_s_edge_is_measured_against_the_grid_s_error() {
    // 60 nm of oxide beside the strip: the mode decays into it (n_eff above 1.444), but the
    // closed window lowers n_eff by more than the grid's own error at 20 nm (the scheme's
    // dispersion, about 4e-3 here): widen the window. The same strip 1.2 um wide doesn't
    // (the test above). The estimate's basis is modes_trust's doc; its check against the
    // shift it estimates is modes_trust::tests
    let (n, w) = modes_run(&new_modes(&[(
        "y: [-1.2 um, 1.2 um]",
        "y: [-0.31 um, 0.31 um]",
    )]));
    let (wide, _) = modes_run(&new_modes(&[]));
    assert!(n[0] < wide[0], "{n:?} {wide:?}");
    let first: Vec<&String> = w.iter().filter(|w| w.starts_with("mode 1 of")).collect();
    assert_eq!(first.len(), 1, "{w:?}");
    assert!(
        first[0].contains("its field reaches the window's edge at y = ")
            && first[0].contains("the closed window lowers its n_eff by about")
            && first[0].contains("more than the grid's own error at its step, about")
            && first[0].contains("widen y_um"),
        "{}",
        first[0]
    );
    // a job that asks for an accuracy is held to it instead: the template's window is wide for
    // the grid's error but not for 1e-12 (its tail at 1 um of oxide holds a few 1e-9 of n_eff)
    let strict = new_modes(&[(
        "modes: 2                 # how many, highest effective index first",
        "modes: 1\n  convergence: { tolerance: 1e-12, grids: 2 }",
    )]);
    let (_, w) = modes_run(&strict);
    assert!(
        w.iter()
            .any(|w| w.contains("more than the tolerance of [task.convergence], 1e-12")),
        "{w:?}"
    );
}

#[test]
fn a_modes_grid_with_too_few_cells_across_the_strip_is_warned_of() {
    // the skeptic's 200 nm step: the grid puts 2 cells up the 220 nm film and 3 across the
    // 500 nm strip (a node on each interface, the cells about the step between), and n_eff
    // comes out far below the 20 nm grid's. The rule is the scheme's three-point differences:
    // one inside the guide that touches no interface needs four cells
    // (modes_trust::FEWEST_ACROSS), not a number of our own
    let coarse = new_modes(&[("step: 20 nm", "step: 200 nm")]);
    let before = warnings(&coarse);
    assert_eq!(before.len(), 1, "{before:?}");
    assert!(
        before[0].starts_with("the guide at y = -0.25 to 0.25 um, z = ")
            && before[0].contains("has 3 cells across it and 2 cells up it at step_nm = 200")
            && before[0].contains("take step_nm at most 55 (its 0.22 um over 4)"),
        "{}",
        before[0]
    );
    // the run records it too, the same words
    let (_, w) = modes_run(&coarse);
    assert!(w.contains(&before[0]), "{w:?}");
    // 400 nm: 2 cells across, 1 up
    let coarser = new_modes(&[("step: 20 nm", "step: 400 nm")]);
    let before = warnings(&coarser);
    assert!(
        before[0].contains("has 2 cells across it and 1 cell up it at step_nm = 400"),
        "{before:?}"
    );
    // 100 nm: 3 cells up; 50 nm: 5 up and 10 across, and no warning of cells (the grid's error
    // is the convergence line's to tell)
    let w = warnings(&new_modes(&[("step: 20 nm", "step: 100 nm")]));
    assert!(w.len() == 1 && w[0].contains("has 3 cells up it"), "{w:?}");
    assert!(warnings(&new_modes(&[("step: 20 nm", "step: 50 nm")])).is_empty());
    assert!(warnings(&new_modes(&[])).is_empty());
}

#[test]
fn a_modes_run_says_its_scheme() {
    let job = new_modes(&[]);
    check(&job).unwrap();
    let mut run = Run::in_memory();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    let events: Vec<Event> = run
        .lines()
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let lines = results::Results::collect(&events).scheme_lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with(
            "scheme: mode::vector solved by standard finite differences (Fallahkhair, Li and \
             Murphy 2008)"
        ) && lines[0].contains("about first order at the guides' corners"),
        "{}",
        lines[0]
    );
}
