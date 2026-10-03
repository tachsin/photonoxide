use std::f64::consts::TAU;
use std::sync::Arc;

use super::super::ideal::{self, Waveguide, um};
use super::super::*;

/// A silicon wire's numbers: n_eff 2.4, n_g 4.2 at 1.55 µm, 3 dB/cm.
fn wire() -> Waveguide {
    ideal::wire()
}

fn sweep() -> Vec<Wavelength> {
    ideal::sweep()
}

#[test]
fn two_waveguides_in_series_are_one() {
    let guide = wire();
    let mut n = Netlist::new();
    for (name, length) in [("x", 12.5), ("y", 30.25)] {
        n.add(name, Arc::new(guide.clone())).unwrap();
        n.set(name, "length", length).unwrap();
    }
    n.connect("x.b", "y.a").unwrap();
    n.expose("in", "x.a").unwrap();
    n.expose("out", "y.b").unwrap();
    let circuit = n.compile().unwrap();
    for w in sweep() {
        let s = circuit.s_matrix(w).unwrap();
        let one = guide.s_matrix(w, &[42.75]).unwrap();
        // the phases, about 400 rad, are summed in a different order: round-off of 400 ε
        assert!(s.max_difference(&one).unwrap() < 1e-12, "{s:?} {one:?}");
        let grown = circuit.s_matrix_by_growth(w).unwrap();
        assert!(s.max_difference(&grown).unwrap() < 1e-15);
    }
}

#[test]
fn an_mzi_is_its_closed_form() {
    let guide = wire();
    for kappa2 in [0.5, 0.3] {
        let circuit = ideal::mzi(&guide, 100.0, 120.0, kappa2).unwrap();
        for w in sweep() {
            let s = circuit.s_matrix(w).unwrap();
            let (tu, tl) = (
                guide.transmission(w.to_um(), 100.0),
                guide.transmission(w.to_um(), 120.0),
            );
            for (q, p) in [(2, 0), (3, 0), (2, 1), (3, 1)] {
                let expected = ideal::mzi_closed_form(tu, tl, kappa2, q - 2, p);
                assert!((s[(q, p)] - expected).norm() < 1e-14);
                // and reciprocal
                assert!((s[(p, q)] - expected).norm() < 1e-14);
            }
            // nothing comes back
            assert!(s[(0, 0)].norm() < 1e-15 && s[(1, 0)].norm() < 1e-15);
        }
    }
    // a balanced MZI sends sin²(Δφ/2) to the bar port
    let lossless = Waveguide::new(2.4, 4.2, 1.55, 0.0);
    let circuit = ideal::mzi(&lossless, 100.0, 120.0, 0.5).unwrap();
    let w = um(1.55);
    let dphi = TAU * 2.4 * 20.0 / 1.55;
    let bar = circuit.s_matrix(w).unwrap().power(2, 0);
    assert!((bar - (dphi / 2.0).sin().powi(2)).abs() < 1e-14);
}

#[test]
fn an_all_pass_ring_follows_bogaerts_equations_1_and_2() {
    let guide = wire();
    let length = TAU * 10.0;
    for kappa2 in [0.1, 0.02] {
        let circuit = ideal::all_pass(&guide, length, kappa2).unwrap();
        let r = (1.0 - kappa2).sqrt();
        for w in sweep() {
            let t = guide.transmission(w.to_um(), length);
            let (a, phi) = (t.norm(), t.arg());
            let s = circuit.s_matrix(w).unwrap();
            assert!((s[(1, 0)] - ideal::bogaerts_eq1(r, a, phi)).norm() < 1e-13);
            assert!((s.power(1, 0) - ideal::bogaerts_eq2(r, a, phi)).abs() < 1e-13);
        }
    }
}

#[test]
fn an_add_drop_ring_follows_bogaerts_equations_5_and_6() {
    let guide = wire();
    let length = TAU * 10.0;
    let (k1, k2) = (0.1, 0.05);
    let circuit = ideal::add_drop(&guide, length, k1, k2).unwrap();
    let (r1, r2) = ((1.0 - k1).sqrt(), (1.0 - k2).sqrt());
    for w in sweep() {
        let t = guide.transmission(w.to_um(), length);
        let (a, phi) = (t.norm(), t.arg());
        let s = circuit.s_matrix(w).unwrap();
        assert!((s.power(1, 0) - ideal::bogaerts_eq5(r1, r2, a, phi)).abs() < 1e-13);
        assert!((s.power(3, 0) - ideal::bogaerts_eq6(r1, r2, a, phi)).abs() < 1e-13);
        assert!(s.reciprocity_error() < 1e-14);
        assert!(s.largest_singular_value().unwrap() < 1.0);
    }
}

#[test]
fn the_sparse_solve_is_sub_network_growth_to_round_off() {
    let circuit = ideal::tangle();
    let mut worst: f64 = 0.0;
    for w in sweep() {
        let s = circuit.s_matrix(w).unwrap();
        let grown = circuit.s_matrix_by_growth(w).unwrap();
        worst = worst.max(s.max_difference(&grown).unwrap());
        // reciprocal and passive, as its parts are
        assert!(s.reciprocity_error() < 1e-14);
        assert!(s.is_passive(1e-14).unwrap());
    }
    assert!(worst < 1e-14, "{worst}");
}

#[test]
fn lossless_netlists_are_unitary() {
    let circuit = ideal::lossless();
    for w in sweep() {
        let s = circuit.s_matrix(w).unwrap();
        assert!(s.unitarity_error() < 1e-13, "{}", s.unitarity_error());
        assert!((s.largest_singular_value().unwrap() - 1.0).abs() < 1e-13);
    }
}

#[test]
fn a_sweep_reuses_the_analysis_and_matches_single_solves() {
    let circuit = ideal::tangle();
    let wavelengths = sweep();
    let spectrum = circuit.spectrum(&wavelengths).unwrap();
    assert_eq!(spectrum.ports(), ["in", "add", "x3", "x4", "x5", "y5"]);
    for (w, s) in wavelengths.iter().zip(spectrum.matrices()) {
        assert_eq!(&circuit.s_matrix(*w).unwrap(), s);
    }
}

#[test]
fn a_circuit_is_a_component() {
    let guide = wire();
    let mzi = ideal::mzi(&guide, 100.0, 120.0, 0.5).unwrap();
    assert_eq!(mzi.kind(), "circuit");
    let names: Vec<&str> = mzi.ports().iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["in1", "in2", "out1", "out2"]);
    let parameters: Vec<&str> = mzi.parameters().iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        parameters,
        ["c1.kappa2", "c2.kappa2", "upper.length", "lower.length"]
    );
    assert_eq!(mzi.values(), [0.5, 0.5, 100.0, 120.0]);
    assert_eq!(mzi.defaults(), mzi.values());
    assert!(mzi.reciprocal());
    assert_eq!(mzi.provenance().fidelity, Fidelity::Analytic);
    // nested, it gives what it gives alone
    let mut n = Netlist::new();
    n.add("m", Arc::new(mzi.clone())).unwrap();
    for (name, port) in [
        ("a", "m.in1"),
        ("b", "m.in2"),
        ("c", "m.out1"),
        ("d", "m.out2"),
    ] {
        n.expose(name, port).unwrap();
    }
    let nested = n.compile().unwrap();
    let w = um(1.55);
    assert_eq!(nested.s_matrix(w).unwrap(), mzi.s_matrix(w).unwrap());
    // and its parameters move it
    let mut tuned = mzi.clone();
    tuned.set("upper", "length", 120.0).unwrap();
    assert_eq!(tuned.values(), [0.5, 0.5, 120.0, 120.0]);
    assert_eq!(tuned.defaults(), tuned.values());
    let s = tuned.s_matrix(w).unwrap();
    // equal arms: everything crosses, less the arm's loss
    let loss = guide.transmission(1.55, 120.0).norm_sqr();
    assert!(s.power(2, 0) < 1e-28 && (s.power(3, 0) - loss).abs() < 1e-14);
    let with_values = mzi.s_matrix_with(w, &[0.5, 0.5, 120.0, 120.0]).unwrap();
    assert_eq!(with_values, s);
    assert!(mzi.s_matrix_with(w, &[0.5]).is_err());
    assert!(tuned.set("upper", "length", -1.0).is_err());
}

#[test]
fn a_component_of_the_wrong_size_is_caught() {
    #[derive(Debug)]
    struct Wrong(Vec<Port>);
    impl Component for Wrong {
        fn kind(&self) -> &str {
            "wrong"
        }
        fn ports(&self) -> &[Port] {
            &self.0
        }
        fn parameters(&self) -> &[Parameter] {
            &[]
        }
        fn s_matrix(&self, _: Wavelength, _: &[f64]) -> Result<SMatrix> {
            Ok(SMatrix::zeros(3))
        }
        fn provenance(&self) -> Provenance {
            Fixed::new("x", &[], SMatrix::zeros(0))
                .unwrap()
                .provenance()
        }
    }
    let mut n = Netlist::new();
    n.add("w", Arc::new(Wrong(ports(&["a", "b"])))).unwrap();
    n.expose("a", "w.a").unwrap();
    n.expose("b", "w.b").unwrap();
    let circuit = n.compile().unwrap();
    assert!(matches!(
        circuit.s_matrix(um(1.55)),
        Err(Error::Netlist(NetlistError::SizeMismatch { size: 3, .. }))
    ));
    // an incomplete netlist doesn't compile
    let mut open = Netlist::new();
    open.add("w", Arc::new(Wrong(ports(&["a", "b"])))).unwrap();
    assert!(matches!(
        open.compile(),
        Err(Error::Netlist(NetlistError::Dangling(_)))
    ));
}

#[test]
fn a_circuit_without_external_ports_has_an_empty_s_matrix() {
    let mut n = Netlist::new();
    n.add("w", Arc::new(wire())).unwrap();
    n.connect("w.a", "w.b").unwrap();
    let circuit = n.compile().unwrap();
    assert_eq!(circuit.s_matrix(um(1.55)).unwrap().size(), 0);
}
