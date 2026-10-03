use super::*;
use crate::compact::checks::{Ring, ring_parametric, ring_spectrum};

// Bogaerts et al. 2012, Eq. 1 is (r − a z)/(1 − r a z): a ratio of functions linear in r, so
// numerator and denominator of degree 1 in r are exact
#[test]
fn the_rings_coupling_is_exact_at_low_degree() {
    let c = ring_parametric(
        "r",
        (0.90, 0.97),
        5,
        12,
        Interpolation::Polynomial { degree: 2 },
    );
    assert!(
        c.fitted < 1e-8 && c.held_out < 1e-8,
        "{} {}",
        c.fitted,
        c.held_out
    );
    assert!(c.stable < 1e-6, "{}", c.stable);
}

// the resonances shift by 0.8 of a free spectral range over the samples
#[test]
fn a_shifting_resonance_is_followed() {
    let c = ring_parametric(
        "n0",
        (2.39, 2.41),
        13,
        16,
        Interpolation::Polynomial { degree: 6 },
    );
    assert!(c.held_out < 5e-6, "{}", c.held_out);
    assert!(c.fitted < 1e-7, "{}", c.fitted);
    // shared poles alone can't follow it
    assert!(c.shared > 0.1, "{}", c.shared);
    // stable models cost little
    assert!(c.stable < 5e-6, "{}", c.stable);
}

#[test]
fn two_parameters() {
    let (rs, as_) = ([0.92, 0.935, 0.95], [0.96, 0.97, 0.98]);
    let mut samples = Vec::new();
    for &r in &rs {
        for &a in &as_ {
            let mut ring = Ring::example();
            ring.r = r;
            ring.a = a;
            samples.push(Sample {
                values: vec![r, a],
                spectrum: ring_spectrum(&ring, 151),
            });
        }
    }
    let params = vec![
        Parameter::new("r", "", 0.935, 0.9, 0.99),
        Parameter::new("a", "", 0.97, 0.9, 0.99),
    ];
    let m = ParametricModel::fit(
        params,
        &samples,
        &Options::new(12),
        Interpolation::Polynomial { degree: 2 },
    )
    .unwrap();
    // in between
    let mut ring = Ring::example();
    ring.r = 0.94;
    ring.a = 0.965;
    let s = m
        .s_matrix(Wavelength::um(1.55).unwrap(), &[0.94, 0.965])
        .unwrap();
    assert!(
        (s[(0, 0)] - ring.through(1.55)).norm() < 1e-6,
        "{}",
        (s[(0, 0)] - ring.through(1.55)).norm()
    );
    // a component with those parameters, inside their sampled range only
    assert_eq!(m.parameters().len(), 2);
    assert!(
        m.s_matrix(Wavelength::um(1.55).unwrap(), &[0.96, 0.97])
            .is_err()
    );
    assert!(
        m.s_matrix(Wavelength::um(1.6).unwrap(), &[0.94, 0.97])
            .is_err()
    );
    assert!(m.s_matrix(Wavelength::um(1.55).unwrap(), &[0.94]).is_err());
    let p = m.provenance();
    assert_eq!(p.fidelity, Fidelity::Compact);
    assert!(
        p.source.contains("degree 2") && p.source.contains("9 parameter samples"),
        "{}",
        p.source
    );
    // its pole-residue form at a point is the same function
    let r = m.rational(&[0.94, 0.965]).unwrap();
    let z = laplace(Wavelength::um(1.55).unwrap());
    assert!((r.evaluate(0, z) - s[(0, 0)]).norm() < 1e-9);
}

#[test]
fn the_monomials_are_every_one_up_to_the_degree() {
    assert_eq!(monomials(1, 3), vec![vec![0], vec![1], vec![2], vec![3]]);
    assert_eq!(
        monomials(2, 2),
        vec![
            vec![0, 0],
            vec![1, 0],
            vec![0, 1],
            vec![2, 0],
            vec![1, 1],
            vec![0, 2]
        ]
    );
    assert_eq!(monomials(3, 2).len(), 10);
    assert_eq!(monomials(0, 3), vec![Vec::<usize>::new()]);
}

#[test]
fn bad_samples_are_errors() {
    let ring = Ring::example();
    let sample = |v: f64| Sample {
        values: vec![v],
        spectrum: ring_spectrum(&ring, 41),
    };
    let p = || vec![Parameter::new("r", "", 0.9, 0.9, 1.0)];
    let fit = |samples: &[Sample], o: &Options, d: usize| {
        ParametricModel::fit(p(), samples, o, Interpolation::Polynomial { degree: d })
            .map(|_| ())
            .unwrap_err()
            .to_string()
    };
    assert!(fit(&[], &Options::new(4), 1).contains("needs samples"));
    assert!(fit(&[sample(0.9), sample(0.95)], &Options::new(4), 2).contains("3 coefficients"));
    assert!(fit(&[sample(0.9), sample(0.9)], &Options::new(4), 1).contains("doesn't vary"));
    assert!(fit(&[sample(0.9), sample(1.5)], &Options::new(4), 1).contains("outside its range"));
    let three = [sample(0.9), sample(0.95), sample(1.0)];
    assert!(
        fit(
            &three,
            &Options {
                proportional: true,
                ..Options::new(4)
            },
            1
        )
        .contains("proportional")
    );
    assert!(
        fit(
            &three,
            &Options {
                delay: Delay::Estimate,
                ..Options::new(4)
            },
            1
        )
        .contains("estimated")
    );
    let other = Sample {
        values: vec![0.95],
        spectrum: ring_spectrum(&ring, 43),
    };
    assert!(
        fit(&[sample(0.9), other, sample(1.0)], &Options::new(4), 1)
            .contains("same ports and wavelengths")
    );
    let wrong = Sample {
        values: vec![0.95, 1.0],
        spectrum: ring_spectrum(&ring, 41),
    };
    assert!(fit(&[sample(0.9), wrong], &Options::new(4), 1).contains("2 values"));
}

// Triverio et al. 2009: each sample fitted alone and rewritten on the basis (Theorem 1),
// piecewise linear between samples; the coupling moves no resonance
#[test]
fn the_piecewise_linear_model_of_the_coupling_is_uniformly_stable() {
    let c = ring_parametric("r", (0.90, 0.97), 5, 12, Interpolation::PiecewiseLinear);
    // at the samples, the local fits themselves; between them, a straight line's error
    assert!(c.fitted < 1e-9, "{}", c.fitted);
    assert!(c.held_out < 2e-3, "{}", c.held_out);
    // no crossing, and sampling agrees
    assert_eq!(c.crossings.as_ref().map(Vec::len), Some(0));
    assert!(c.sampled < 0.0, "{}", c.sampled);
}

// a resonance that shifts by more than its width between samples: the piecewise-linear model
// makes poles cross the axis, and the test finds every crossing the sampled poles show
#[test]
fn the_uniform_stability_test_finds_every_crossing() {
    let c = ring_parametric("n0", (2.39, 2.41), 13, 16, Interpolation::PiecewiseLinear);
    assert!(c.held_out > 1.0, "{}", c.held_out);
    let crossings = c.crossings.unwrap();
    assert!(crossings.len() > 10, "{}", crossings.len());
    assert_eq!(c.unconfirmed, Some(0));
    assert_eq!(c.missed, Some(0));
    assert!(c.sampled > 0.0);
}

#[test]
fn piecewise_linear_needs_a_grid_and_decides_one_parameter() {
    let mut samples = Vec::new();
    for &r in &[0.92, 0.95] {
        for &a in &[0.96, 0.98] {
            let mut ring = Ring::example();
            ring.r = r;
            ring.a = a;
            samples.push(Sample {
                values: vec![r, a],
                spectrum: ring_spectrum(&ring, 101),
            });
        }
    }
    let params = || {
        vec![
            Parameter::new("r", "", 0.935, 0.9, 0.99),
            Parameter::new("a", "", 0.97, 0.9, 0.99),
        ]
    };
    let m = ParametricModel::fit(
        params(),
        &samples,
        &Options::new(12),
        Interpolation::PiecewiseLinear,
    )
    .unwrap();
    // at a corner, the corner's own fit
    let mut ring = Ring::example();
    ring.r = 0.95;
    ring.a = 0.96;
    let s = m
        .s_matrix(Wavelength::um(1.55).unwrap(), &[0.95, 0.96])
        .unwrap();
    assert!((s[(0, 0)] - ring.through(1.55)).norm() < 1e-9);
    assert!(m.provenance().source.contains("piecewise linear"));
    // the exact test is for one parameter
    assert!(
        m.uniform_stability()
            .unwrap_err()
            .to_string()
            .contains("one parameter")
    );
    // three of four corners aren't a grid
    let e = ParametricModel::fit(
        params(),
        &samples[..3],
        &Options::new(12),
        Interpolation::PiecewiseLinear,
    )
    .unwrap_err();
    assert!(e.to_string().contains("full grid"), "{e}");
    // a polynomial model's poles are sampled instead
    let samples: Vec<Sample> = [0.9, 0.95, 0.97]
        .iter()
        .map(|&r| {
            let mut ring = Ring::example();
            ring.r = r;
            Sample {
                values: vec![r],
                spectrum: ring_spectrum(&ring, 101),
            }
        })
        .collect();
    let p = ParametricModel::fit(
        vec![Parameter::new("r", "", 0.9, 0.9, 0.97)],
        &samples,
        &Options::new(12),
        Interpolation::Polynomial { degree: 1 },
    )
    .unwrap();
    assert!(
        p.uniform_stability()
            .unwrap_err()
            .to_string()
            .contains("piecewise-linear")
    );
}

#[test]
fn grid_weights_are_multilinear_hats() {
    let axes = vec![vec![0.0, 1.0, 3.0], vec![10.0, 20.0]];
    // at a node, that node alone (first parameter fastest)
    assert_eq!(
        grid_weights(&axes, &[1.0, 20.0]),
        vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0]
    );
    // inside a cell, the four corners
    let w = grid_weights(&axes, &[2.0, 12.5]);
    let expect = [0.0, 0.5 * 0.75, 0.5 * 0.75, 0.0, 0.5 * 0.25, 0.5 * 0.25];
    for (a, b) in w.iter().zip(expect) {
        assert!((a - b).abs() < 1e-15, "{w:?}");
    }
    assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-15);
}
