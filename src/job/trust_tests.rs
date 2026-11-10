//! What a job file says that it may not mean, which a run would otherwise answer with a number
//! and exit 0 (issue #783): a misspelled device parameter, and a band outside where the analytic
//! fit holds or typed in nanometres.

use super::*;
use crate::run::Format;

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
