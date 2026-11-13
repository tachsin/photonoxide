//! The real components' unit tests (Q3): photons with spectra, pairs in a circuit, budgets and
//! the routes they run along.

use std::sync::Arc;

use num_complex::Complex64 as c64;

use super::*;
use crate::circuit::components::{Coupler, Dispersion, Waveguide};
use crate::circuit::{Circuit, Component, Netlist, SMatrix, Spectrum};
use crate::portable;
use crate::units::Wavelength;

const OK: &str = "a valid test";

fn um(x: f64) -> Wavelength {
    Wavelength::um(x).expect(OK)
}

fn coupler_spectrum(kappa2: f64, wavelengths: &[Wavelength]) -> Spectrum {
    let mut net = Netlist::new();
    net.add("c", Arc::new(Coupler::new())).expect(OK);
    net.set("c", "coupling", kappa2).expect(OK);
    for p in ["o1", "o2", "o3", "o4"] {
        net.expose(p, &format!("c.{p}")).expect(OK);
    }
    net.compile().expect(OK).spectrum(wavelengths).expect(OK)
}

fn band() -> Vec<Wavelength> {
    (0..=60)
        .map(|k| um(1.545 + 1e-4 * f64::from(k) * 1.6))
        .collect()
}

#[test]
fn identical_photons_on_a_balanced_coupler_never_split() {
    let spectrum = coupler_spectrum(0.5, &band());
    let photon = Wavepacket::gaussian(um(1.55), 2.0).expect(OK);
    let input = SpectralInput::new(
        &spectrum,
        &[
            SpectralPhoton::new("o1", photon.clone()),
            SpectralPhoton::new("o2", photon),
        ],
    )
    .expect(OK);
    let split = Fock::new(vec![0, 0, 1, 1]);
    assert!(input.probability(&split).expect(OK) < 1e-15);
    let d = input.distribution().expect(OK);
    assert!((d.total() - 1.0).abs() < 1e-13, "{}", d.total());
}

#[test]
fn photons_far_apart_in_time_are_distinguishable() {
    let spectrum = coupler_spectrum(0.3, &band());
    let photon = Wavepacket::gaussian(um(1.55), 2.0).expect(OK);
    let input = SpectralInput::new(
        &spectrum,
        &[
            // 10/σ: an overlap of e^(−100), the phase turning 1.9 rad between the band's points
            SpectralPhoton::new("o1", photon.clone()).delayed(1.5e-11),
            SpectralPhoton::new("o2", photon),
        ],
    )
    .expect(OK);
    let split = input.probability(&Fock::new(vec![0, 0, 1, 1])).expect(OK);
    // R² + T², with the delay's residual overlap e^(−σ²τ²) far below round-off
    assert!((split - (0.49 + 0.09)).abs() < 1e-12, "{split}");
}

#[test]
fn spectral_inputs_refuse_what_they_cant_take() {
    let spectrum = coupler_spectrum(0.5, &band());
    let photon = Wavepacket::gaussian(um(1.55), 2.0).expect(OK);
    assert!(SpectralInput::new(&spectrum, &[]).is_err());
    assert!(SpectralInput::new(&spectrum, &[SpectralPhoton::new("x", photon.clone())]).is_err());
    let far = Wavepacket::gaussian(um(1.3), 1.0).expect(OK);
    assert!(SpectralInput::new(&spectrum, &[SpectralPhoton::new("o1", far)]).is_err());
    let one = coupler_spectrum(0.5, &[um(1.55)]);
    assert!(SpectralInput::new(&one, &[SpectralPhoton::new("o1", photon.clone())]).is_err());
    let input = SpectralInput::new(&spectrum, &[SpectralPhoton::new("o1", photon)]).expect(OK);
    assert!(input.probability(&Fock::new(vec![0, 0, 1, 1])).is_err());
    assert!(input.probability(&Fock::new(vec![0, 1])).is_err());
}

#[test]
fn two_sources_behind_balanced_couplers_make_a_full_fringe() {
    let s = SMatrix::from_fn(4, |i, j| {
        let h = std::f64::consts::FRAC_1_SQRT_2;
        match (i, j) {
            (2, 0) | (3, 1) => c64::new(h, 0.0),
            (2, 1) | (3, 0) => c64::new(0.0, h),
            _ => c64::new(0.0, 0.0),
        }
    });
    let t = Transfer::new(&s, ["o1", "o2", "o3", "o4"].map(String::from).to_vec()).expect(OK);
    let birth = |port: &str, a: f64| PairBirth {
        signal: port.into(),
        idler: port.into(),
        amplitude: c64::new(a, 0.0),
    };
    let x = pair_amplitude(&t, &t, &[birth("o1", 0.5)], ["o3", "o4"], true).expect(OK);
    let y = pair_amplitude(&t, &t, &[birth("o2", 0.5)], ["o3", "o4"], true).expect(OK);
    assert!((two_path_visibility(x, y) - 1.0).abs() < 1e-15);
    // an unbalanced pump: R = 0.6 to one source, 0.4 to the other, V = 4RT/(R + T)² on the
    // pair amplitudes R and T
    let y = pair_amplitude(&t, &t, &[birth("o2", 0.4 / 0.6 * 0.5)], ["o3", "o4"], true).expect(OK);
    let (r, tt) = (0.6, 0.4);
    assert!((two_path_visibility(x, y) - 4.0 * r * tt).abs() < 1e-15);
    assert!(pair_amplitude(&t, &t, &[birth("o1", 1.0)], ["o3", "o3"], true).is_err());
    assert!(pair_amplitude(&t, &t, &[birth("x", 1.0)], ["o3", "o4"], false).is_err());
    assert_eq!(
        two_path_visibility(c64::new(0.0, 0.0), c64::new(0.0, 0.0)),
        0.0
    );
}

#[test]
fn budgets_multiply_their_stages() {
    let b = Budget::new()
        .with("source", 0.9, "stated")
        .expect(OK)
        .with("chip", 0.5, "stated")
        .expect(OK);
    assert!((b.transmission() - 0.45).abs() < 1e-15);
    assert!((b.loss_db() - 3.467_874_862_246_563).abs() < 1e-12);
    assert!((b.event(3) - portable::powi(0.45f64, 3)).abs() < 1e-15);
    assert!((event(&[b.clone(), b.clone()]) - 0.2025).abs() < 1e-15);
    assert!(Budget::new().with("bad", 1.1, "").is_err());
    assert!(Budget::new().with("bad", -0.1, "").is_err());
    assert_eq!(b.stages().len(), 2);
}

#[test]
fn thresholds_follow_their_papers() {
    // at the threshold, Varnava et al.'s condition (1 − ε) η_D = 1/2
    let eta_d = 0.9;
    let eta_s = 2.0 / 3.0 / eta_d;
    assert!(((1.0 - varnava_loss(eta_s, eta_d)) * eta_d - 0.5).abs() < 1e-15);
    assert!(Threshold::Varnava2008.margin(0.7) > 0.0);
    assert!(Threshold::Bartolucci2023.margin(0.95) < 0.0);
    assert!(!Threshold::Varnava2008.assumptions().is_empty());
    assert!((fusion_loss(0.973, 0.25) - 0.103_704).abs() < 1e-6);
    // Qi et al.: perfect detectors and no loss leave squeezed light non-classical
    assert!(qi_simulable_error(0.5, 1.0, 1.0, 0.0, 10) > 1.0);
    assert!((qi_fidelity(0.3, 0.0, 1.0, 0.0) - 1.0).abs() < 1e-15);
    let eta = qi_exact_transmission(1.0, 0.8, 1e-3);
    assert!((eta - 1e-3 / 0.8 * (1.0 + 1.0 / portable::tanh(1f64))).abs() < 1e-15);
}

/// A Mach–Zehnder interferometer of two balanced couplers (0.1 and 0.3 dB of excess loss) and
/// two guides losing 2.5 dB/cm, 1 mm and 1.2 mm long: ports `in1`, `in2`, `out1`, `out2`.
fn lossy_mzi() -> Circuit {
    let mut net = Netlist::new();
    for (name, length) in [("upper", 1000.0), ("lower", 1200.0)] {
        let guide = Dispersion::new(um(1.55), 2.4, 4.2).with_loss(2.5);
        net.add(name, Arc::new(Waveguide::new(guide))).expect(OK);
        net.set(name, "length", length).expect(OK);
    }
    for (name, excess) in [("split", 0.1), ("combine", 0.3)] {
        let coupler = Coupler::new().with_excess_loss(excess).expect(OK);
        net.add(name, Arc::new(coupler)).expect(OK);
        net.set(name, "coupling", 0.5).expect(OK);
    }
    for (a, b) in [
        ("split.o3", "upper.o1"),
        ("split.o4", "lower.o1"),
        ("upper.o2", "combine.o2"),
        ("lower.o2", "combine.o1"),
    ] {
        net.connect(a, b).expect(OK);
    }
    for (name, at) in [
        ("in1", "split.o2"),
        ("in2", "split.o1"),
        ("out1", "combine.o3"),
        ("out2", "combine.o4"),
    ] {
        net.expose(name, at).expect(OK);
    }
    net.compile().expect(OK)
}

#[test]
fn an_interferometer_has_two_routes_from_an_input_to_an_output() {
    let circuit = lossy_mzi();
    let w = um(1.55);
    let found = routes(&circuit, "in1", "out1", w, 16).expect(OK);
    let arms: Vec<&str> = found.iter().map(|r| r[1].instance.as_str()).collect();
    assert_eq!(arms, ["upper", "lower"], "{found:?}");
    for route in &found {
        assert_eq!(route.len(), 3);
        assert_eq!(
            (route[0].instance.as_str(), route[0].from.as_str()),
            ("split", "o2")
        );
        assert_eq!(
            (route[2].instance.as_str(), route[2].to.as_str()),
            ("combine", "o3")
        );
    }
    // each route's budget is its stages' product: half of each coupler, their 0.4 dB of excess
    // loss, and its arm's 0.25 dB/mm
    for (route, mm) in found.iter().zip([1.0, 1.2]) {
        let budget = Budget::along(&circuit, route, w).expect(OK);
        let expected = 0.25 * portable::powf(10f64, -(0.4 + 0.25 * mm) / 10.0);
        let t = budget.transmission();
        assert!((t / expected - 1.0).abs() < 1e-12, "{t} against {expected}");
    }
    // at most `most`, in the order found
    assert_eq!(routes(&circuit, "in1", "out1", w, 1).expect(OK), found[..1]);
}

#[test]
fn the_only_route_carries_the_circuits_own_transmission() {
    let mut net = Netlist::new();
    let guide = Dispersion::new(um(1.55), 2.4, 4.2).with_loss(3.0);
    net.add("g", Arc::new(Waveguide::new(guide))).expect(OK);
    net.set("g", "length", 500.0).expect(OK);
    let coupler = Coupler::new().with_excess_loss(0.2).expect(OK);
    net.add("c", Arc::new(coupler)).expect(OK);
    net.set("c", "coupling", 0.3).expect(OK);
    net.connect("g.o2", "c.o1").expect(OK);
    for (name, at) in [
        ("in", "g.o1"),
        ("b", "c.o2"),
        ("out", "c.o4"),
        ("x", "c.o3"),
    ] {
        net.expose(name, at).expect(OK);
    }
    let mut circuit = net.compile().expect(OK);
    let w = um(1.56);
    let found = routes(&circuit, "in", "out", w, 16).expect(OK);
    assert_eq!(
        found,
        [vec![Hop::new("g", "o1", "o2"), Hop::new("c", "o1", "o4")]]
    );
    let budget = Budget::along(&circuit, &found[0], w).expect(OK);
    let at = |name: &str| {
        circuit
            .ports()
            .iter()
            .position(|p| p.name == name)
            .expect(OK)
    };
    let whole = circuit.s_matrix(w).expect(OK).power(at("out"), at("in"));
    assert!((budget.transmission() / whole - 1.0).abs() < 1e-12);
    // a coupler that sends everything across leaves no route through it (o1 to o4), and the
    // route across (o1 to o3) whole
    circuit.set("c", "coupling", 1.0).expect(OK);
    assert!(routes(&circuit, "in", "out", w, 16).expect(OK).is_empty());
    assert_eq!(routes(&circuit, "in", "x", w, 16).expect(OK).len(), 1);
}

#[test]
fn routes_refuse_ports_that_arent_the_circuits() {
    let circuit = lossy_mzi();
    let w = um(1.55);
    let e = routes(&circuit, "nowhere", "out1", w, 4)
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("no external port nowhere") && e.contains("in1, in2, out1, out2"),
        "{e}"
    );
    assert!(routes(&circuit, "in1", "split.o3", w, 4).is_err());
    let e = routes(&circuit, "in1", "in1", w, 4)
        .unwrap_err()
        .to_string();
    assert!(e.contains("one port to another"), "{e}");
}
