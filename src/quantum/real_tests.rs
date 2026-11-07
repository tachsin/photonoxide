//! The real components' unit tests (Q3): photons with spectra, pairs in a circuit, budgets.

use std::sync::Arc;

use num_complex::Complex64 as c64;

use super::*;
use crate::circuit::components::Coupler;
use crate::circuit::{Netlist, SMatrix, Spectrum};
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
    assert!((b.event(3) - 0.45f64.powi(3)).abs() < 1e-15);
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
    assert!((eta - 1e-3 / 0.8 * (1.0 + 1.0 / 1f64.tanh())).abs() < 1e-15);
}
