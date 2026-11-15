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

// ---- batch 2: rings, chips and optimizations (#783's A10, A12, A13, A14) ----

/// A job's run in memory, checked: its warnings, in order.
fn warned(job: &Job) -> Vec<String> {
    check(job).unwrap();
    let mut run = Run::in_memory();
    execute(job, &mut run, &Stop::new(None)).unwrap();
    run.lines()
        .iter()
        .filter_map(|l| match serde_json::from_str(l) {
            Ok(Event::Warning { text }) => Some(text),
            _ => None,
        })
        .collect()
}

/// The count a band warning asks for: "take points at least N".
fn asked_points(warning: &str) -> usize {
    let at = warning.find("take points at least ").expect("a count") + 21;
    warning[at..].split(' ').next().unwrap().parse().unwrap()
}

#[test]
fn a_ring_s_band_coarser_than_its_resonance_is_warned_of() {
    // the device's own band, 100 nm at 101 points: a 1 nm step against a resonance under 0.1 nm
    // wide (the experimentalist's −15 dB for a −45 dB dip)
    let ring = |task: &str| device_job("all-pass ring", "    radius: \"10\"\n", task);
    let w = warned(&ring("  kind: spectrum\n"));
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        w[0].starts_with(
            "the band's step, 1.00 nm (101 points from 1.5 to 1.6 um), is coarser than the \
             all-pass ring's resonance (its resonance at "
        ) && w[0].contains("over its loaded Q of ")
            && w[0].contains("nm wide at half its depth: every sample can miss a dip's bottom"),
        "{}",
        w[0]
    );
    // the count it asks for clears it; one fewer doesn't (the step then the width or finer)
    let need = asked_points(&w[0]);
    let band = |points: usize| {
        format!("  kind: spectrum\n  from_um: 1.5\n  to_um: 1.6\n  points: {points}\n")
    };
    assert!(warned(&ring(&band(need))).is_empty());
    assert_eq!(warned(&ring(&band(need - 1))).len(), 1);
    // a study's points: the narrowest of them, said once
    let sweep =
        ring("  kind: sweep\n  axis:\n    - { parameter: gap, from: 0.2, to: 0.3, points: 3 }\n");
    let w = warned(&sweep);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        w[0].contains("at the study's point 3, its narrowest"),
        "{}",
        w[0]
    );
}

#[test]
fn a_chip_s_band_coarser_than_its_loop_s_resonance_is_warned_of() {
    // `photonoxide new chip`: 2001 points over 20 nm, a step an eighth of the dips' width
    let text = templates::template("chip").unwrap().text.to_owned();
    assert!(warned(&yaml(&text)).is_empty());
    // its old 201 points, a 0.1 nm step against 0.081 nm: T = 0.812 where the dip is 0.712
    let coarse = yaml(&text.replace("points: 2001", "points: 201"));
    let w = warned(&coarse);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        w[0].starts_with(
            "the band's step, 0.100 nm (201 points from 1.54 to 1.56 um), is coarser than the \
             narrowest resonance of the chip's loop through coupler, ring (from its round trip at \
             1.5500 um), 0.081"
        ),
        "{}",
        w[0]
    );
    assert_eq!(asked_points(&w[0]), 248);
    // the dip the 201 points show against the one the 2001 do
    let through_min = |job: &Job| {
        let mut run = Run::in_memory();
        execute(job, &mut run, &Stop::new(None)).unwrap();
        run.lines()
            .iter()
            .filter_map(|l| match serde_json::from_str(l) {
                Ok(Event::SParameters { s, .. }) => {
                    Some(s[1][0][0] * s[1][0][0] + s[1][0][1] * s[1][0][1])
                }
                _ => None,
            })
            .fold(f64::INFINITY, f64::min)
    };
    let (shown, fine) = (through_min(&coarse), through_min(&yaml(&text)));
    assert!(shown > 0.8 && fine < 0.72, "{shown} {fine}");
}

#[test]
fn a_ring_s_radius_below_the_bend_s_range_is_warned_of() {
    // #783's A12: the ring's mode is the straight strip's, which the bend's model holds from 5 µm
    let ring = |radius: &str| {
        device_job(
            "all-pass ring",
            &format!("    radius: \"{radius}\"\n"),
            SPECTRUM,
        )
    };
    let w = warnings(&ring("1"));
    assert_eq!(
        w,
        [
            "the all-pass ring's radius, 1 um, is outside 5 to 10000 um, where its analytic model \
          holds (its stated error, FSR within 0.5 % of 3D, is measured there): take radius \
          within it, or a model that holds there"
        ]
    );
    assert!(warnings(&ring("5")).is_empty());
    // the run records it too
    assert!(warned(&ring("1")).contains(&w[0]));
    // the 3D model takes the same straight mode
    let three = yaml(
        "name: d\ndevice:\n  kind: add-drop ring\n  fidelity: 3D\n  parameters:\n    radius: \"3\"\n\
         task:\n  kind: spectrum\n",
    );
    assert_eq!(
        warnings(&three),
        [
            "the add-drop ring's radius, 3 um, is outside 5 to 10000 um, where its 3D model holds: \
          take radius within it, or a model that holds there"
        ]
    );
    // a study's designs, together
    let sweep = device_job(
        "all-pass ring",
        "    radius: \"10\"\n",
        "  kind: sweep\n  from_um: 1.5\n  to_um: 1.6\n  points: 3\n  axis:\n    - { parameter: radius, from: 2, to: 20, points: 4 }\n",
    );
    let w = warnings(&sweep);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        w[0].starts_with(
            "the study's designs take the all-pass ring's radius from 2 to 20 um, 1 of 4 outside \
             5 to 10000 um"
        ),
        "{}",
        w[0]
    );
    // a chip's bend, named by its instance
    let chip = yaml(
        "name: c\ncircuit:\n  instance:\n    - name: arc\n      device: { kind: bend, fidelity: \
         analytic, parameters: { radius: \"2\" } }\n  port:\n    - { name: a, at: arc.o1 }\n    \
         - { name: b, at: arc.o2 }\ntask:\n  kind: spectrum\n  from_um: 1.54\n  to_um: 1.56\n  \
         points: 11\n",
    );
    let w = warnings(&chip);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        w[0].starts_with("arc: the bend's radius, 2 um, is outside 5 to 10000 um"),
        "{}",
        w[0]
    );
    // an optimization that may take it there
    let opt = device_job(
        "all-pass ring",
        "    radius: \"10\"\ntimeout_minutes: 1\n",
        "  kind: optimize\n  method: lbfgsb\n  seed: 1\n  evaluations: 10\n  variable:\n    - { parameter: radius, from: 1, to: 20 }\n  objective:\n    - { name: Q, goal: maximize }\n",
    );
    let w = warnings(&opt);
    assert!(
        w.iter().any(|w| w.starts_with("the optimization's radius goes from 1 to 20 um, and the all-pass ring's analytic model holds over 5 to 10000 um")),
        "{w:?}"
    );
}

#[test]
fn the_built_in_templates_warn_of_nothing() {
    for t in templates::TEMPLATES {
        let job = Job::parse_as(t.text, Format::Yaml).unwrap();
        assert!(
            warnings(&job).is_empty(),
            "{}: {:?}",
            t.name,
            warnings(&job)
        );
    }
}

/// The designer's switch (#783's A13): two 3 dB couplers, a thermo-optic shifter 100 µm long in
/// one arm and a waveguide as long in the other, unheated; with `fixed`, a waveguide of the
/// shifter's length in series with it.
fn switch(fixed: bool) -> Job {
    let (heater_out, guide) = if fixed {
        (
            "[heater.o2, guide.o1]\n    - [guide.o2, combine.o2]",
            "\n    - { name: guide, device: { kind: waveguide, fidelity: analytic, parameters: { length: \"100\" } } }",
        )
    } else {
        ("[heater.o2, combine.o2]", "")
    };
    yaml(&format!(
        "name: s\ncircuit:\n  instance:\n    - {{ name: split, component: coupler, values: {{ coupling: 0.5 }} }}\n    \
         - {{ name: heater, device: {{ kind: thermo-optic phase shifter, fidelity: analytic, parameters: {{ length: \"100\" }} }} }}\n    \
         - {{ name: reference, device: {{ kind: waveguide, fidelity: analytic, parameters: {{ length: \"100\" }} }} }}\n    \
         - {{ name: combine, component: coupler, values: {{ coupling: 0.5 }} }}{guide}\n  \
         connections:\n    - [split.o3, heater.o1]\n    - {heater_out}\n    - [split.o4, reference.o1]\n    \
         - [reference.o2, combine.o1]\n  port:\n    - {{ name: in, at: split.o1 }}\n    \
         - {{ name: in2, at: split.o2 }}\n    \
         - {{ name: a, at: combine.o3 }}\n    - {{ name: b, at: combine.o4 }}\n\
         task:\n  kind: spectrum\n  from_um: 1.55\n  to_um: 1.55\n  points: 1\n"
    ))
}

/// The share of a switch's output in its brighter port, unheated.
fn brighter_share(job: &Job) -> f64 {
    let mut run = Run::in_memory();
    execute(job, &mut run, &Stop::new(None)).unwrap();
    let s = run
        .lines()
        .iter()
        .find_map(|l| match serde_json::from_str(l) {
            Ok(Event::SParameters { s, .. }) => Some(s),
            _ => None,
        })
        .unwrap();
    let p = |q: usize| s[q][0][0] * s[q][0][0] + s[q][0][1] * s[q][0][1];
    p(2).max(p(3)) / (p(2) + p(3))
}

#[test]
fn a_shifter_without_its_guide_s_propagation_is_warned_of() {
    // the shifter's component is the phase its heater adds and its loss: the arm of it alone
    // lacks the waveguide's 2π n_eff L/λ, so the unheated switch isn't in either state
    let broken = switch(false);
    let shown = brighter_share(&broken);
    assert!(shown < 0.99, "{shown}");
    let w = warnings(&broken);
    assert_eq!(
        w,
        [
            "heater: the thermo-optic phase shifter's component is the phase it adds and its loss, \
          not its guide's own propagation over its 100 um: no waveguide is joined to it, so its \
          arm lacks the guide's phase, 2 pi n_eff L / wavelength, that an arm of a waveguide \
          has, and an interferometer of the two isn't balanced without power; put a waveguide \
          100 um long in series with it"
        ]
    );
    assert!(warned(&broken).contains(&w[0]));
    // with the waveguide in series the switch is balanced: all of it in one port
    let fixed = switch(true);
    assert!(warnings(&fixed).is_empty(), "{:?}", warnings(&fixed));
    let share = brighter_share(&fixed);
    assert!(share > 1.0 - 1e-12, "{share}");
}

#[test]
fn a_heater_s_neighbour_beyond_the_measured_crosstalk_is_warned_of() {
    // #783's A14: the designer's 20 µm pitch, where the 2D crosstalk is about 3 times below the
    // measured; 7.3 µm is the largest pitch it is checked at (device::thermo::CROSSTALK_PITCH)
    let shifter = |kind: &str, fidelity: &str, pitch: &str| {
        yaml(&format!(
            "name: h\ndevice:\n  kind: {kind}\n  fidelity: {fidelity}\n  parameters:\n    pitch: \"{pitch}\"\ntask:\n  kind: spectrum\n"
        ))
    };
    let w = warnings(&shifter("thermo-optic phase shifter", "2D", "20"));
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        w[0].starts_with(
            "the thermo-optic phase shifter's pitch, 20 um, is beyond 7.3 um, where its 2D \
             model's crosstalk on the neighbour is checked: it agrees with Jacques et al. 2019's \
             measured 0.141 at 7.3 um"
        ),
        "{}",
        w[0]
    );
    assert_eq!(
        warnings(&shifter(
            "doped thermo-optic phase shifter",
            "analytic",
            "10"
        ))
        .len(),
        1
    );
    // the default 5 µm and the measured 7.3 µm: nothing; the heater in the guide has no
    // measured crosstalk to hold it to
    assert!(warnings(&shifter("thermo-optic phase shifter", "2D", "5")).is_empty());
    assert!(warnings(&shifter("doped thermo-optic phase shifter", "2D", "7.3")).is_empty());
    assert!(warnings(&shifter("in-guide thermo-optic phase shifter", "2D", "20")).is_empty());
}

#[test]
fn the_built_in_jobs_warn_only_where_they_mean_it() {
    // jobs/ and its components, as the studio opens them
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
    let (mut seen, mut warned) = (0, Vec::new());
    for dir in [root.clone(), root.join("components")] {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
                continue;
            }
            let job = Job::load(&path).unwrap();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            warned.extend(warnings(&job).into_iter().map(|w| format!("{name}: {w}")));
            seen += 1;
        }
    }
    assert!(seen > 40, "{seen}");
    warned.sort();
    // the self-imaging jobs take sections past the 2D model's widths on purpose: their figures
    // are Soldano and Pennings's closed forms, which examples/mmi_soldano.rs checks, but the
    // model's stated |S|² error isn't measured there, and the job says so
    let names: Vec<&str> = warned
        .iter()
        .map(|w| w.split(':').next().unwrap())
        .collect();
    assert_eq!(
        names,
        ["mmi-1x2-self-imaging.yaml", "mmi-2x2-self-imaging.yaml"],
        "{warned:#?}"
    );
}
