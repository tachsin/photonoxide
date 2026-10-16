use super::*;
use crate::compact::checks::{Ring, ring_parametric, ring_spectrum};

// Bogaerts et al. 2011, Eq. 1 is (r − a z)/(1 − r a z): a ratio of functions linear in r, so
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
    // (the residues N(z)/D′(z) carry the round-off of the poles: 1e-9 here)
    assert!((r.evaluate(0, z) - s[(0, 0)]).norm() < 1e-8);
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

/// The example ring over its coupling and round trip, (r, a) on a 3 × 3 grid, fitted by
/// `interpolation`.
fn two_parameter_model(interpolation: Interpolation) -> ParametricModel {
    let mut samples = Vec::new();
    for &r in &[0.92, 0.935, 0.95] {
        for &a in &[0.96, 0.97, 0.98] {
            let mut ring = Ring::example();
            ring.r = r;
            ring.a = a;
            samples.push(Sample {
                values: vec![r, a],
                spectrum: ring_spectrum(&ring, 101),
            });
        }
    }
    ParametricModel::fit(
        vec![
            Parameter::new("r", "", 0.935, 0.9, 0.99),
            Parameter::new("a", "", 0.97, 0.9, 0.99),
        ],
        &samples,
        &Options::new(12),
        interpolation,
    )
    .unwrap()
}

/// The largest difference between the exact derivatives at `values` and central differences
/// of step `h` (in each parameter's unit), relative to the largest derivative, over a few
/// wavelengths.
fn derivative_error(m: &ParametricModel, values: &[f64], h: f64) -> f64 {
    let (mut worst, mut scale) = (0.0f64, 0.0f64);
    for w in [1.531, 1.5432, 1.55, 1.5611] {
        let w = Wavelength::um(w).unwrap();
        let exact = m.derivatives(w, values).unwrap().unwrap();
        for (k, d) in exact.iter().enumerate() {
            let at = |sign: f64| {
                let mut v = values.to_vec();
                v[k] += sign * h;
                m.s_matrix(w, &v).unwrap()
            };
            let (up, down) = (at(1.0), at(-1.0));
            let fd = (up[(0, 0)] - down[(0, 0)]) / (2.0 * h);
            worst = worst.max((fd - d[(0, 0)]).norm());
            scale = scale.max(d[(0, 0)].norm());
        }
    }
    worst / scale
}

// N and D are linear in the weights: the derivatives are exact, against central differences
// to their own truncation error
#[test]
fn the_derivatives_are_the_models_own() {
    let m = two_parameter_model(Interpolation::Polynomial { degree: 2 });
    let e = derivative_error(&m, &[0.941, 0.9655], 1e-6);
    assert!(e < 1e-6, "{e}");
    // between the nodes of a piecewise-linear model, where it is smooth
    let p = two_parameter_model(Interpolation::PiecewiseLinear);
    let e = derivative_error(&p, &[0.941, 0.9655], 1e-6);
    assert!(e < 1e-6, "{e}");
    // and the one-parameter shifting ring, whose resonances move with n0
    let shift = shifting_model();
    let e = derivative_error(&shift, &[2.4013], 1e-7);
    assert!(e < 1e-5, "{e}");
    // one matrix per parameter, outside the range none
    let w = Wavelength::um(1.55).unwrap();
    assert_eq!(m.derivatives(w, &[0.94, 0.97]).unwrap().unwrap().len(), 2);
    assert!(m.derivatives(w, &[0.99, 0.97]).is_err());
}

/// The example ring over n0 from 2.39 to 2.41 (0.8 of a free spectral range), 13 samples,
/// 16 poles, degree 6.
fn shifting_model() -> ParametricModel {
    let samples: Vec<Sample> = (0..13)
        .map(|k| {
            let v = 2.39 + 0.02 * f64::from(k) / 12.0;
            let mut ring = Ring::example();
            ring.n0 = v;
            Sample {
                values: vec![v],
                spectrum: ring_spectrum(&ring, 151),
            }
        })
        .collect();
    ParametricModel::fit(
        vec![Parameter::new("n0", "", 2.4, 2.39, 2.41)],
        &samples,
        &Options::new(16),
        Interpolation::Polynomial { degree: 6 },
    )
    .unwrap()
}

#[test]
fn a_model_reads_back_to_the_same_bits() {
    for m in [
        two_parameter_model(Interpolation::Polynomial { degree: 2 }),
        two_parameter_model(Interpolation::PiecewiseLinear),
        shifting_model(),
    ] {
        let text = m.to_json().unwrap();
        let back = ParametricModel::from_json(&text).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.to_json().unwrap(), text);
        let values = if m.parameters().len() == 2 {
            vec![0.941, 0.9655]
        } else {
            vec![2.4013]
        };
        for w in [1.531, 1.55, 1.5611] {
            let w = Wavelength::um(w).unwrap();
            let (a, b) = (
                m.s_matrix(w, &values).unwrap(),
                back.s_matrix(w, &values).unwrap(),
            );
            assert_eq!(a[(0, 0)].re.to_bits(), b[(0, 0)].re.to_bits());
            assert_eq!(a[(0, 0)].im.to_bits(), b[(0, 0)].im.to_bits());
        }
    }
    // what isn't a model is refused, and says why
    let m = two_parameter_model(Interpolation::Polynomial { degree: 2 });
    let text = m.to_json().unwrap();
    for (from, to, why) in [
        ("\"format\":1", "\"format\":2", "format"),
        ("\"monomials\":[[0,0],", "\"monomials\":[", "monomials"),
        ("\"half\":[", "\"half\":[-", "range"),
    ] {
        let e = ParametricModel::from_json(&text.replacen(from, to, 1)).unwrap_err();
        assert!(e.to_string().contains(why), "{e}");
    }
    assert!(ParametricModel::from_json("{}").is_err());
}

#[test]
fn what_a_model_covers_and_how() {
    let m = two_parameter_model(Interpolation::Polynomial { degree: 2 });
    let r = m.range();
    assert!((r[0].0 - 0.92).abs() < 1e-15 && (r[0].1 - 0.95).abs() < 1e-15);
    assert!((r[1].0 - 0.96).abs() < 1e-15 && (r[1].1 - 0.98).abs() < 1e-15);
    assert_eq!(m.interpolation(), Interpolation::Polynomial { degree: 2 });
    assert_eq!(m.basis_poles(), 12);
    assert_eq!(
        two_parameter_model(Interpolation::PiecewiseLinear).interpolation(),
        Interpolation::PiecewiseLinear
    );
    // a ring without gain is passive at every value: Eq. 1's |S| ≤ 1, to the fit's error
    let p = m.passivity(201, 5).unwrap();
    assert_eq!(p.points, 201 * 25);
    assert!(p.passive(1e-6), "{}", p.largest);
    assert!(p.largest > 0.99, "{}", p.largest);
    assert!(m.passivity(201, 1).is_err());
}

// the error between samples falls as the degree rises (with the samples to determine it)
#[test]
fn the_error_falls_with_the_degree() {
    let errors: Vec<f64> = [(5, 2), (9, 4), (13, 6)]
        .iter()
        .map(|&(count, degree)| {
            ring_parametric(
                "n0",
                (2.39, 2.41),
                count,
                16,
                Interpolation::Polynomial { degree },
            )
            .held_out
        })
        .collect();
    assert!(
        errors[1] < errors[0] / 10.0 && errors[2] < errors[1] / 10.0,
        "{errors:?}"
    );
}

#[test]
fn grid_slopes_are_the_hats_slopes() {
    let axes = vec![vec![0.0, 1.0, 3.0], vec![10.0, 20.0]];
    let at = [2.0, 12.5];
    for along in 0..2 {
        let s = grid_slopes(&axes, &at, along);
        let h = 1e-6;
        let mut up = at;
        let mut down = at;
        up[along] += h;
        down[along] -= h;
        let (a, b) = (grid_weights(&axes, &up), grid_weights(&axes, &down));
        for (k, v) in s.iter().enumerate() {
            assert!((v - (a[k] - b[k]) / (2.0 * h)).abs() < 1e-8, "{s:?}");
        }
        // the weights sum to 1, so their slopes to 0
        assert!(s.iter().sum::<f64>().abs() < 1e-14);
    }
}
