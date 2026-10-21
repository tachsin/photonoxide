//! A device's compact models ([`super::fit_from_device`]): the ring's closed form fitted and
//! used through its compact fidelity, the plan's samples, the same bits on any number of
//! threads and on reload, and what is refused.

use super::fit_checks::{
    fitted, ring_against_closed_form, ring_derivative_error, ring_over_gap, spec, temp,
};
use super::*;
use crate::circuit::Fidelity;
use crate::compact::Interpolation;
use crate::device::{FitAxis, FitPlan, Fitted, Sampling, Spec, Values};

// Bogaerts et al.'s closed form, fitted over the ring's gap and used as its compact fidelity:
// within its held-out error of the closed form at gaps it was neither fitted nor checked at
#[test]
fn the_rings_closed_form_is_fitted_and_used() {
    let f = ring_over_gap(7, 4).unwrap();
    assert!(f.held_out_error < 1e-4, "{}", f.held_out_error);
    assert!(
        f.fitted_error <= f.held_out_error * 10.0,
        "{} {}",
        f.fitted_error,
        f.held_out_error
    );
    assert!(f.passivity < 1.0 + 1e-4, "{}", f.passivity);
    let between = ring_against_closed_form(&f, &[0.1537, 0.18, 0.2111, 0.2463]).unwrap();
    assert!(
        between < 3.0 * f.held_out_error.max(1e-9),
        "{between} {}",
        f.held_out_error
    );
    assert_eq!(f.parameters, ["gap"]);
    assert_eq!(f.source, "analytic");
    assert_eq!(f.samples, 7);
    assert_eq!(f.held_out, 6);
    assert!(f.fixed.iter().any(|(n, v)| n == "radius" && *v == 10.0));
}

#[test]
fn the_derivatives_in_the_covered_parameters_are_exact() {
    let f = ring_over_gap(7, 4).unwrap();
    let e = ring_derivative_error(&f).unwrap();
    assert!(e < 1e-6, "{e}");
}

#[test]
fn a_fit_is_the_same_bits_on_any_threads_and_on_reload() {
    let plan = FitPlan::new(vec![
        FitAxis::new("gap", 0.15, 0.25, 0),
        FitAxis::new("radius", 9.99, 10.01, 0),
    ])
    .sampled(Sampling::LatinHypercube {
        points: 12,
        seed: 3,
    })
    .holding_out(4)
    .with_poles(12)
    .interpolated(Interpolation::Polynomial { degree: 2 });
    let s = spec("all-pass ring", Fidelity::Analytic, &[]);
    let on = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| fitted(&s, &plan, (1.54, 1.56, 31)).unwrap().1)
    };
    let (one, many) = (on(1), on(6));
    let text = one.to_json().unwrap();
    assert_eq!(text, many.to_json().unwrap());
    let back = Fitted::from_json(&text).unwrap();
    assert_eq!(back, one);
    assert_eq!(back.to_json().unwrap(), text);
}

#[test]
fn a_latin_hypercubes_samples_take_each_level_once() {
    let plan = FitPlan::new(vec![
        FitAxis::new("gap", 0.1, 0.3, 0),
        FitAxis::new("radius", 5.0, 15.0, 0),
    ])
    .sampled(Sampling::LatinHypercube { points: 9, seed: 7 })
    .holding_out(5);
    let samples = plan.samples();
    assert_eq!(samples.len(), 9);
    for (d, (lo, hi)) in [(0.1, 0.3), (5.0, 15.0)].iter().enumerate() {
        let mut levels: Vec<f64> = samples
            .iter()
            .map(|s| (s[d] - lo) / (hi - lo) * 8.0)
            .collect();
        levels.sort_by(f64::total_cmp);
        for (k, l) in levels.iter().enumerate() {
            assert!((l - k as f64).abs() < 1e-9, "{levels:?}");
        }
    }
    // the same samples from the same seed, others from another
    assert_eq!(plan.samples(), samples);
    let other = plan
        .clone()
        .sampled(Sampling::LatinHypercube { points: 9, seed: 8 });
    assert_ne!(other.samples(), samples);
    // held out: inside the range, each level the middle of a stratum
    let held = plan.held_out_samples();
    assert_eq!(held.len(), 5);
    for h in &held {
        assert!(
            h[0] > 0.1 && h[0] < 0.3 && h[1] > 5.0 && h[1] < 15.0,
            "{h:?}"
        );
    }
    // a grid's: every combination, the last axis fastest
    let grid = FitPlan::new(vec![
        FitAxis::new("gap", 0.1, 0.3, 3),
        FitAxis::new("radius", 5.0, 15.0, 2),
    ]);
    let g = grid.samples();
    assert_eq!(g.len(), 6);
    assert_eq!(g[0], [0.1, 5.0]);
    assert_eq!(g[1], [0.1, 15.0]);
    assert_eq!(g[2], [0.2, 5.0]);
    assert_eq!(grid.describe_sampling(), "a 3 × 2 grid");
}

#[test]
fn what_a_fit_refuses() {
    let s = spec("all-pass ring", Fidelity::Analytic, &[]);
    let refuse = |plan: FitPlan, s: &Spec| {
        fit_from_device("fit", s, &Compute::model(), &plan)
            .unwrap_err()
            .to_string()
    };
    let gap = || FitAxis::new("gap", 0.15, 0.25, 5);
    assert!(refuse(FitPlan::new(vec![]), &s).contains("1 to 4 parameters"));
    assert!(
        refuse(FitPlan::new(vec![FitAxis::new("depth", 0.1, 0.2, 3)]), &s)
            .contains("no parameter depth")
    );
    assert!(refuse(FitPlan::new(vec![gap(), gap()]), &s).contains("twice"));
    assert!(refuse(FitPlan::new(vec![FitAxis::new("gap", 0.25, 0.15, 5)]), &s).contains("range"));
    assert!(
        refuse(FitPlan::new(vec![FitAxis::new("gap", 0.01, 0.25, 5)]), &s).contains("must be from")
    );
    assert!(
        refuse(FitPlan::new(vec![FitAxis::new("gap", 0.15, 0.25, 1)]), &s).contains("2 points")
    );
    assert!(refuse(FitPlan::new(vec![gap()]).holding_out(0), &s).contains("at least one"));
    assert!(
        refuse(
            FitPlan::new(vec![FitAxis::new("gap", 0.15, 0.25, 3)])
                .interpolated(Interpolation::Polynomial { degree: 4 }),
            &s
        )
        .contains("5 coefficients")
    );
    assert!(
        refuse(
            FitPlan::new(vec![FitAxis::new("gap", 0.15, 0.25, 0)])
                .sampled(Sampling::LatinHypercube { points: 9, seed: 1 })
                .interpolated(Interpolation::PiecewiseLinear),
            &s
        )
        .contains("on a grid")
    );
    // a source that is the compact fidelity, or one the device hasn't
    let mut c = s.clone();
    c.fidelity = Fidelity::TwoD;
    let e = refuse(FitPlan::new(vec![gap()]), &c);
    assert!(e.contains("no 2D model"), "{e}");
    // a parameter the covered ones move through its expression
    let follows = spec(
        "all-pass ring",
        Fidelity::Analytic,
        &[("coupling_length", "gap*10")],
    );
    assert!(refuse(FitPlan::new(vec![gap()]), &follows).contains("follows the covered parameters"));
}

#[test]
fn a_design_names_its_compact_model_and_is_computed_by_it() {
    let f = ring_over_gap(5, 2).unwrap();
    let dir = temp("design");
    f.write(&dir.0.join("ring.compact.json")).unwrap();
    // a design next to it, read from its file, names the model by a relative path
    let text = "name = \"ring\"\n\n[device]\nkind = \"all-pass ring\"\nfidelity = \"compact\"\n\
                compact = \"ring.compact.json\"\n\n[device.parameters]\ngap = \"0.21\"\n\n\
                [task]\nkind = \"spectrum\"\nfrom_um = 1.545\nto_um = 1.555\npoints = 11\n";
    let path = dir.0.join("ring.toml");
    std::fs::write(&path, text).unwrap();
    let job = Job::load(&path).unwrap();
    check(&job).unwrap();
    let runs = temp("design-runs");
    let mut run = Run::create(&runs.0, &job).unwrap();
    execute(&job, &mut run, &Stop::new(None)).unwrap();
    // its run keeps the model beside the job, so the job runs again from there
    assert!(run.dir().join("ring.compact.json").exists());
    let again = Job::load(&run.dir().join("job.toml")).unwrap();
    check(&again).unwrap();
    // read from text, the job has no folder to find a relative path from
    let e = check(&Job::parse(text).unwrap()).unwrap_err().to_string();
    assert!(e.contains("ring.compact.json"), "{e}");
    // the device offers the compact fidelity after its own analytic one, labelled with its source
    let mut s = spec("all-pass ring", Fidelity::Compact, &[]);
    s.compact = Some(dir.0.join("ring.compact.json").display().to_string());
    let (device, _) = s.build().unwrap();
    let ladder: Vec<Fidelity> = device.fidelities().iter().map(|m| m.fidelity).collect();
    assert_eq!(
        ladder,
        [Fidelity::Analytic, Fidelity::Compact, Fidelity::ThreeD]
    );
    let m = &device.fidelities()[1];
    assert!(m.name.contains("analytic model's runs"), "{}", m.name);
    let stated = m.error.as_ref().unwrap();
    assert_eq!(stated.against, Fidelity::Analytic);
    assert_eq!(stated.bound, f.held_out_error);
    assert!(
        m.validity
            .iter()
            .any(|r| r.name == "gap" && r.min == 0.15 && r.max == 0.25)
    );
    assert!(
        m.validity
            .iter()
            .any(|r| r.name == "radius" && r.min == 10.0 && r.max == 10.0)
    );
    // each range once: the band is the wavelength's
    let mut names: Vec<&str> = m.validity.iter().map(|r| r.name.as_str()).collect();
    names.sort_unstable();
    let count = names.len();
    names.dedup();
    assert_eq!(names.len(), count, "{names:?}");
    // a parameter it doesn't cover at another value, or a gap outside its range: refused
    let at = |changes: &[(&str, f64)]| {
        let v = Values::new(device.as_ref(), changes).unwrap();
        device
            .component(&v, Fidelity::Compact)
            .map(|_| ())
            .unwrap_err()
            .to_string()
    };
    assert!(at(&[("radius", 12.0)]).contains("fitted at radius = 10"));
    assert!(at(&[("gap", 0.3)]).contains("covers gap from 0.15 to 0.25"));
    // a compact fidelity without a model, or a model of another device
    let e = spec("all-pass ring", Fidelity::Compact, &[])
        .build()
        .unwrap_err()
        .to_string();
    assert!(e.contains("compact = "), "{e}");
    let mut other = spec("waveguide", Fidelity::Analytic, &[]);
    other.compact = s.compact.clone();
    assert!(
        other
            .build()
            .unwrap_err()
            .to_string()
            .contains("all-pass ring's")
    );
    // on another stack, the model is listed but refuses to compute
    let mut moved = s.clone();
    moved.stack = crate::device::StackSpec::preset("soi");
    let (d, design) = moved.build().unwrap();
    let e = d
        .component(design.values(), Fidelity::Compact)
        .map(|_| ())
        .unwrap_err()
        .to_string();
    assert!(e.contains("another stack"), "{e}");
    // its design farms nothing: it computes here
    assert!(!farms(&job));
}
