//! A heated circuit's cases: a Mach–Zehnder interferometer whose arms are two of Jacques et
//! al.'s N++ shifters side by side, its phases through their thermal compact model against the
//! heat solver's own, in time and in frequency.

use std::sync::Arc;

use num_complex::Complex64 as c64;

use super::components::{Coupler, PhaseShifter, mzi};
use super::{Circuit, Component, Heated};
use crate::thermal::rc_checks::pair_steps;
use crate::thermal::{RcFit, RcModel};
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid circuit";

/// The pitch of the two shifters, µm: an MZI's arms 20 µm apart.
const PITCH: f64 = 20.0;

/// A balanced MZI of two ideal 3 dB couplers, its arms `upper` and `lower` ideal phase shifters.
fn mzi_circuit() -> Circuit {
    let coupler: Arc<dyn Component> = Arc::new(Coupler::new());
    let arm: Arc<dyn Component> = Arc::new(PhaseShifter::new());
    mzi(coupler.clone(), coupler, arm.clone(), arm)
        .expect(OK)
        .compile()
        .expect(OK)
}

fn wavelength() -> Wavelength {
    Wavelength::um(1.55).expect(OK)
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// The MZI's cross power, o1 to o3, in time when the upper arm's heater is turned on at the
/// power that shifts its own arm by π: through the 6-pole model fitted to the pair's step
/// responses, and through those responses themselves. The largest difference over the record.
fn transient_difference() -> f64 {
    let (_, r) = pair_steps(PITCH);
    let fit = RcModel::fit(&r.times_s, &r.steps, Some(&r.steady), &RcFit::new(6)).expect(OK);
    let p = std::f64::consts::PI / r.steady[0][0];
    let n = r.times_s.len();
    let heated = Heated::new(mzi_circuit(), &["upper", "lower"], fit.model).expect(OK);
    let model = heated
        .transient(wavelength(), &r.times_s, &[vec![p; n], vec![0.0; n]])
        .expect(OK);
    let direct = mzi_circuit();
    let (upper, lower) = (
        direct.parameter("upper.phase").expect(OK),
        direct.parameter("lower.phase").expect(OK),
    );
    (0..n)
        .map(|m| {
            let mut v = direct.values().to_vec();
            v[upper] = (r.steps[0][0][m] * p).rem_euclid(std::f64::consts::TAU);
            v[lower] = (r.steps[0][1][m] * p).rem_euclid(std::f64::consts::TAU);
            let s = direct.s_matrix_with(wavelength(), &v).expect(OK);
            (s[(2, 0)].norm_sqr() - model.s[m][(2, 0)].norm_sqr()).abs()
        })
        .fold(0.0, f64::max)
}

/// The MZI biased at quadrature (the upper heater at half its own Pπ): its cross power's
/// response to the upper heater's modulation at 1, 10, 30 and 100 kHz through the model against
/// the heat solver's small-signal response of each arm, (K + iωM) T̂ = Q̂ on the same grid. The
/// largest difference over the steady response's magnitude.
fn modulation_difference() -> f64 {
    let (s, r) = pair_steps(PITCH);
    let fit = RcModel::fit(&r.times_s, &r.steps, Some(&r.steady), &RcFit::new(6)).expect(OK);
    let heated = Heated::new(mzi_circuit(), &["upper", "lower"], fit.model).expect(OK);
    let bias = [0.5 * std::f64::consts::PI / r.steady[0][0], 0.0];
    let frequencies = [1e3, 1e4, 3e4, 1e5];
    let m = heated
        .modulation(wavelength(), &bias, (2, 0), 0, &frequencies)
        .expect(OK);
    let dc: f64 = m
        .by_phase
        .iter()
        .zip(&r.steady)
        .map(|(g, z)| g * z[0])
        .sum();
    frequencies
        .iter()
        .zip(&m.per_watt)
        .map(|(&f, model)| {
            let exact: c64 = (0..2)
                .map(|k| m.by_phase[k] * r.steady[k][0] * s.response(0, k, f).expect(OK))
                .sum();
            (model - exact).norm() / dc.abs()
        })
        .fold(0.0, f64::max)
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "circuit/heated-mzi-transient",
            title: r"Heated circuit, analytic: a balanced MZI of ideal 3 dB couplers whose arms are two of Jacques et al.'s N++ shifters 20 µm apart (their region's depth, 20 µm beyond each; the heat graded from 20 nm, the modes on 10 nm), the upper heater turned on at its own arm's $P_\pi$; the cross power in time (every 0.2 µs to 200 µs) with the arms' phases through a 6-pole thermal compact model of both heaters' step responses, against the same circuit with the heat solver's phases; the largest difference (shown)",
            tier: Tier::Analytic,
            source: r"the heat solver's own transient on the same grid (BDF2)",
            run: || outcome(transient_difference(), 0.0, 1e-3),
        },
        Case {
            id: "circuit/heated-mzi-modulation",
            title: r"Heated circuit, analytic: the same MZI at quadrature, its cross power's small-signal response to the upper heater's power at 1, 10, 30 and 100 kHz, the arms' $Z(i\omega)$ the 6-pole model's and $\partial\lvert S\rvert^2/\partial\varphi$ the circuit adjoint's, against the heat solver's small-signal response of both arms, $(K + i\omega M)\hat T = \hat Q$; the largest difference over the steady response (shown)",
            tier: Tier::Analytic,
            source: r"the heat equation in frequency on the same grid; 1e-2 for the transient's 0.2 µs steps the model is fitted to",
            run: || outcome(modulation_difference(), 0.0, 1e-2),
        },
    ]
}
