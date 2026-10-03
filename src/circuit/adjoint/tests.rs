use std::sync::Arc;

use num_complex::Complex64 as c64;

use super::super::ideal::um;
use super::super::{Component, Netlist, Parameter, Port, Provenance, SMatrix, objective};
use super::checks::{self, Check, Shifter};
use crate::Result;
use crate::units::Wavelength;

#[test]
fn the_adjoint_gradient_is_the_finite_differences() {
    // fourth-order central differences of the whole circuit: their round-off and truncation
    // leave about 1e-11 to 5e-10 (docs/methods/circuit-adjoint.md)
    for check in [
        Check::Mzi,
        Check::Ring,
        Check::Mesh,
        Check::Nested,
        Check::Fallback,
    ] {
        let error = check.error();
        assert!(error < 1e-8, "{check:?}: {error:e}");
    }
}

#[test]
fn a_components_finite_differences_are_its_derivative() {
    let error = checks::fallback_error();
    assert!(error < 1e-9, "{error:e}");
}

#[test]
fn the_jacobian_is_the_finite_differences_of_s() {
    let circuit = checks::nested();
    let w = um(1.5503);
    let values = circuit.values().to_vec();
    let (s, jacobian) = circuit.jacobian(w, &values).unwrap();
    assert_eq!(s, circuit.s_matrix(w).unwrap());
    assert_eq!(jacobian.len(), values.len());
    for (k, exact) in jacobian.iter().enumerate() {
        let delta = 1e-5;
        let at = |d: f64| {
            let mut v = values.clone();
            v[k] += d;
            circuit.s_matrix_with(w, &v).unwrap()
        };
        let (a, b, c, d) = (at(-2.0 * delta), at(-delta), at(delta), at(2.0 * delta));
        let fd = SMatrix::from_fn(s.size(), |q, p| {
            (a[(q, p)] - 8.0 * b[(q, p)] + 8.0 * c[(q, p)] - d[(q, p)]) / (12.0 * delta)
        });
        let scale = exact
            .rows()
            .iter()
            .flatten()
            .fold(0.0f64, |m, v| m.max(v.norm()));
        let error = exact.max_difference(&fd).unwrap() / scale;
        assert!(error < 1e-8, "{}: {error:e}", circuit.parameters()[k].name);
    }
    // a circuit's derivatives are its jacobian, so circuits nest exactly
    let nested = circuit.derivatives(w, &values).unwrap().unwrap();
    assert_eq!(nested, jacobian);
}

#[test]
fn a_reciprocal_circuits_jacobian_is_symmetric() {
    let circuit = checks::mesh();
    let (_, jacobian) = circuit.jacobian(um(1.55), circuit.values()).unwrap();
    for d in jacobian {
        assert!(d.reciprocity_error() < 1e-14, "{}", d.reciprocity_error());
    }
}

#[test]
fn parameters_are_found_by_name() {
    let circuit = checks::mzi(&checks::wire(), &checks::split(), 100.0, 120.0, (0.5, 0.3));
    assert_eq!(circuit.parameter("c2.kappa2"), Some(1));
    assert_eq!(circuit.parameter("lower.length"), Some(3));
    assert_eq!(circuit.parameter("lower"), None);
}

#[test]
fn an_objective_of_one_input_solves_for_that_input_only() {
    // the same gradient whether the adjoint solves for one input or two
    let circuit = checks::mzi(&checks::wire(), &checks::split(), 100.0, 120.0, (0.5, 0.3));
    let w = [um(1.55)];
    let one = circuit
        .gradient(&w, circuit.values(), |s| objective::power(s, 2, 0))
        .unwrap();
    let two = circuit
        .gradient(&w, circuit.values(), |s| {
            let (value, mut g) = objective::power(s, 2, 0);
            g[0][(2, 1)] = c64::new(1e-300, 0.0);
            (value, g)
        })
        .unwrap();
    assert_eq!(one.0, two.0);
    for (a, b) in one.1.iter().zip(&two.1) {
        assert!((a - b).abs() < 1e-15 * a.abs().max(1.0));
    }
    // an objective that depends on nothing has no gradient
    let none = circuit
        .gradient(&w, circuit.values(), |s| {
            (1.0, vec![SMatrix::zeros(s[0].size())])
        })
        .unwrap();
    assert_eq!(none, (1.0, vec![0.0; 4]));
}

#[test]
fn a_sum_over_wavelengths_is_the_sum_of_the_gradients() {
    let circuit = checks::nested();
    let (w1, w2) = (um(1.549), um(1.551));
    let g = |w: &[Wavelength]| {
        circuit
            .gradient(w, circuit.values(), |s| objective::power(s, 3, 0))
            .unwrap()
    };
    let (both, one, other) = (g(&[w1, w2]), g(&[w1]), g(&[w2]));
    assert!((both.0 - one.0 - other.0).abs() < 1e-15);
    for ((b, x), y) in both.1.iter().zip(&one.1).zip(&other.1) {
        assert!((b - x - y).abs() < 1e-13 * b.abs().max(1.0));
    }
}

#[test]
fn a_wrong_objective_is_refused() {
    let circuit = checks::mzi(&checks::wire(), &checks::split(), 100.0, 120.0, (0.5, 0.3));
    let w = [um(1.55)];
    // no sensitivity
    assert!(
        circuit
            .gradient(&w, circuit.values(), |_| (0.0, vec![]))
            .is_err()
    );
    // the wrong size
    assert!(
        circuit
            .gradient(&w, circuit.values(), |_| (0.0, vec![SMatrix::zeros(2)]))
            .is_err()
    );
    // the wrong number of values
    assert!(
        circuit
            .gradient(&w, &[0.5], |s| objective::power(s, 2, 0))
            .is_err()
    );
}

/// A shifter whose derivatives have the wrong size.
#[derive(Debug)]
struct Broken(Shifter);

impl Component for Broken {
    fn kind(&self) -> &str {
        "broken"
    }
    fn ports(&self) -> &[Port] {
        self.0.ports()
    }
    fn parameters(&self) -> &[Parameter] {
        self.0.parameters()
    }
    fn s_matrix(&self, w: Wavelength, values: &[f64]) -> Result<SMatrix> {
        self.0.s_matrix(w, values)
    }
    fn provenance(&self) -> Provenance {
        self.0.provenance()
    }
    fn derivatives(&self, _: Wavelength, _: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        Ok(Some(vec![SMatrix::zeros(3)]))
    }
}

#[test]
fn derivatives_of_the_wrong_size_are_refused() {
    let mut n = Netlist::new();
    n.add("x", Arc::new(Broken(Shifter::new()))).unwrap();
    n.expose("a", "x.a").unwrap();
    n.expose("b", "x.b").unwrap();
    let circuit = n.compile().unwrap();
    let w = [um(1.55)];
    let error = circuit
        .gradient(&w, circuit.values(), |s| objective::power(s, 1, 0))
        .unwrap_err();
    assert!(error.to_string().contains("instance x"), "{error}");
    assert!(circuit.jacobian(w[0], circuit.values()).is_err());
}

#[test]
fn a_phase_shifters_gradient_is_its_closed_form() {
    // an MZI of two 50:50 couplers, phase φ in the upper arm and 0 in the lower: straight
    // through both couplers (in1 to out1) is r e^(iφ) r + iκ iκ = (e^(iφ) − 1)/2, whose power is
    // sin²(φ/2), so ∂/∂φ = sin(φ)/2, and the lower arm's phase the opposite
    let mut n = Netlist::new();
    for name in ["c1", "c2"] {
        n.add(name, checks::split()).unwrap();
        n.set(name, "kappa2", 0.5).unwrap();
    }
    n.add("upper", Arc::new(Shifter::new())).unwrap();
    n.add("lower", Arc::new(Shifter::new())).unwrap();
    let phi = 0.7;
    n.set("upper", "phase", phi).unwrap();
    for (a, b) in [
        ("c1.b1", "upper.a"),
        ("upper.b", "c2.a1"),
        ("c1.b2", "lower.a"),
        ("lower.b", "c2.a2"),
    ] {
        n.connect(a, b).unwrap();
    }
    for (name, port) in [
        ("in1", "c1.a1"),
        ("in2", "c1.a2"),
        ("out1", "c2.b1"),
        ("out2", "c2.b2"),
    ] {
        n.expose(name, port).unwrap();
    }
    let circuit = n.compile().unwrap();
    let (power, gradient) = circuit
        .gradient(&[um(1.55)], circuit.values(), |s| objective::power(s, 2, 0))
        .unwrap();
    assert!((power - (phi / 2.0).sin().powi(2)).abs() < 1e-15);
    let upper = gradient[circuit.parameter("upper.phase").unwrap()];
    let lower = gradient[circuit.parameter("lower.phase").unwrap()];
    assert!((upper - phi.sin() / 2.0).abs() < 1e-15, "{upper}");
    assert!((lower + phi.sin() / 2.0).abs() < 1e-15, "{lower}");
}
