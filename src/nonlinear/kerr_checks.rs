//! The validation cases of third-order processes: the nonlinear Schrödinger equation
//! ([`super::pulse`]), four-wave mixing ([`super::fwm`]) and the Lugiato–Lefever equation
//! ([`super::resonator`]), against closed forms and Coen et al.'s fully specified resonator.

use std::f64::consts::PI;

use num_complex::Complex64 as c64;

use crate::material::{Model, silica};
use crate::nonlinear::checks::linear_index;
use crate::nonlinear::fwm::{FourWave, Waves, signal_gain};
use crate::nonlinear::pairs::{Matching, Pump, gaussian_gamma, gaussian_purity, joint_spectrum};
use crate::nonlinear::phase::{Device, Process, State, Wave};
use crate::nonlinear::pulse::{
    C_M_PER_PS, Dispersion, Guide, Pulse, Raman, angular_frequency, dispersion_length, loss_per_m,
    second_order_soliton, soliton_period, soliton_power, spm_peak_phase, spm_rms_broadening,
};
use crate::nonlinear::resonator::{Resonator, Ring, peak_width};
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier, note_numerical};

fn relative(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance: tolerance * expected.abs(),
        error: (measured - expected).abs(),
    }
}

fn absolute(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

// ---------------------------------------------------------------------------------------------
// The dispersion fit against a Sellmeier formula's own derivatives
// ---------------------------------------------------------------------------------------------

/// Truncated power series in x = ω − ω₀: c[k] the coefficient of x^k.
fn series_mul(a: &[f64], b: &[f64]) -> Vec<f64> {
    let n = a.len();
    (0..n)
        .map(|k| (0..=k).map(|j| a[j] * b[k - j]).sum())
        .collect()
}

/// 1/a as a series (a[0] ≠ 0).
fn series_recip(a: &[f64]) -> Vec<f64> {
    let mut r = vec![0.0; a.len()];
    r[0] = 1.0 / a[0];
    for k in 1..a.len() {
        let s: f64 = (1..=k).map(|j| a[j] * r[k - j]).sum();
        r[k] = -s / a[0];
    }
    r
}

/// √a as a series (a[0] > 0).
fn series_sqrt(a: &[f64]) -> Vec<f64> {
    let mut r = vec![0.0; a.len()];
    r[0] = a[0].sqrt();
    for k in 1..a.len() {
        let s: f64 = (1..k).map(|j| r[j] * r[k - j]).sum();
        r[k] = (a[k] - s) / (2.0 * r[0]);
    }
    r
}

/// β₁ … β_order of a Sellmeier medium (n² = A + Σ B_iλ²/(λ² − C_i²), C_i in µm) at `lam` µm
/// from its formula's Taylor series: n²(ω) = A + Σ B_i/(1 − C_i²ω²/(2πc)²), β = ωn/c, each
/// β_k = k!·(the x^k coefficient), exact to round-off.
pub(crate) fn sellmeier_betas(a: f64, terms: &[(f64, f64)], lam: f64, order: usize) -> Vec<f64> {
    let w0 = angular_frequency(lam);
    let two_pi_c = 2.0 * PI * C_M_PER_PS * 1e6; // µm/ps
    let len = order + 1;
    // ω² = ω₀² + 2ω₀x + x²
    let mut w2 = vec![0.0; len];
    w2[0] = w0 * w0;
    if len > 1 {
        w2[1] = 2.0 * w0;
    }
    if len > 2 {
        w2[2] = 1.0;
    }
    let mut n2 = vec![0.0; len];
    n2[0] = a;
    for &(b, c) in terms {
        let k = c * c / (two_pi_c * two_pi_c);
        let den: Vec<f64> = w2
            .iter()
            .enumerate()
            .map(|(j, v)| if j == 0 { 1.0 - k * v } else { -k * v })
            .collect();
        for (n, r) in n2.iter_mut().zip(series_recip(&den)) {
            *n += b * r;
        }
    }
    let n = series_sqrt(&n2);
    let mut omega = vec![0.0; len];
    omega[0] = w0;
    if len > 1 {
        omega[1] = 1.0;
    }
    let beta = series_mul(&omega, &n);
    let mut factorial = 1.0;
    (1..=order)
        .map(|k| {
            factorial *= k as f64;
            beta[k] * factorial / C_M_PER_PS
        })
        .collect()
}

/// The range of the silica fit, µm: wide, which holds the higher orders best.
const SILICA_RANGE: (f64, f64) = (1.0, 2.2);

/// Silica (Malitson's Sellmeier formula, the catalogue's) at 1.55 µm: (the fit's β_k, its
/// estimated error, the formula's Taylor β_k).
pub(crate) fn silica_beta(k: usize) -> (f64, f64, f64) {
    let m = silica();
    let Model::Sellmeier { a, terms } = m.model() else {
        return (f64::NAN, f64::NAN, f64::NAN);
    };
    let terms: Vec<(f64, f64)> = terms.iter().map(|t| (t.b, t.c)).collect();
    let fit = Dispersion::of_index(
        |l| Ok(m.refractive_index(Wavelength::um(l)?)?.re),
        1.55,
        SILICA_RANGE,
        4,
    );
    let Ok(fit) = fit else {
        return (f64::NAN, f64::NAN, f64::NAN);
    };
    let exact = sellmeier_betas(*a, &terms, 1.55, 4);
    (fit.beta(k), fit.error(k), exact[k - 1])
}

fn silica_case(k: usize, tolerance: f64) -> Outcome {
    let (fit, _, exact) = silica_beta(k);
    relative(fit, exact, tolerance)
}

fn silica_beta2() -> Outcome {
    silica_case(2, SILICA_BETA2_TOLERANCE)
}

fn silica_beta4() -> Outcome {
    silica_case(4, SILICA_BETA4_TOLERANCE)
}

const SILICA_BETA2_TOLERANCE: f64 = 2e-8;
const SILICA_BETA4_TOLERANCE: f64 = 1e-5;

// ---------------------------------------------------------------------------------------------
// The NLSE: solitons, SPM, the photon number
// ---------------------------------------------------------------------------------------------

/// The guide of the soliton cases: β₂ = −0.05 ps²/m, γ = 1 /(W·m), no loss; T₀ = 0.1 ps.
const BETA2: f64 = -0.05;
const T0: f64 = 0.1;

fn soliton_guide() -> Guide {
    Guide::new(
        Dispersion::new(1.55, &[BETA2]).expect("a valid dispersion"),
        1.0,
        0.0,
    )
}

/// The largest |A − A_exact| over the grid.
fn max_error(a: &Pulse, exact: impl Fn(f64) -> c64) -> f64 {
    a.times()
        .iter()
        .zip(&a.field)
        .map(|(&t, v)| (v - exact(t)).norm())
        .fold(0.0, f64::max)
}

/// The fundamental soliton √P₀ sech(T/T₀), P₀ = |β₂|/(γT₀²), after five soliton periods on
/// `steps` split steps (1024 points across 50 T₀): the largest |A − √P₀ sech(T/T₀) e^{iz/2L_D}|
/// over √P₀ (Agrawal Eq. (5.2.16)).
pub(crate) fn fundamental_soliton(steps: usize) -> f64 {
    let p0 = soliton_power(1.0, T0, BETA2);
    let ld = dispersion_length(T0, BETA2);
    let z = 5.0 * soliton_period(T0, BETA2);
    let run = || -> crate::Result<f64> {
        let start = Pulse::sech(p0, T0, 1024, 50.0 * T0)?;
        let end = soliton_guide().propagate(&start, z, steps)?;
        Ok(max_error(&end, |t| {
            c64::new(p0.sqrt() / (t / T0).cosh(), 0.0) * c64::new(0.0, 0.5 * z / ld).exp()
        }) / p0.sqrt())
    };
    run().unwrap_or(f64::NAN)
}

/// The second-order soliton, 2√P₀ sech(T/T₀) with P₀ the fundamental's, at half its period
/// (ξ = π/4) on `steps` split steps (1024 points across 40 T₀): the largest difference from
/// Agrawal's Eq. (5.2.23), over 2√P₀.
pub(crate) fn second_order(steps: usize) -> f64 {
    let p1 = soliton_power(1.0, T0, BETA2);
    let ld = dispersion_length(T0, BETA2);
    let z = 0.5 * soliton_period(T0, BETA2);
    let run = || -> crate::Result<f64> {
        let start = Pulse::sech(4.0 * p1, T0, 1024, 40.0 * T0)?;
        let end = soliton_guide().propagate(&start, z, steps)?;
        Ok(
            max_error(&end, |t| second_order_soliton(z / ld, t / T0) * p1.sqrt())
                / (2.0 * p1.sqrt()),
        )
    };
    run().unwrap_or(f64::NAN)
}

/// The third-order soliton, 3√P₀ sech(T/T₀), after one soliton period z₀ = (π/2)L_D on
/// `steps` split steps (1024 points across 40 T₀): the largest difference of |A|² from the
/// input's, over 9P₀ (Agrawal, p. 144: |u|² is periodic in ξ with period π/2 for N ≥ 2).
pub(crate) fn third_order_period(steps: usize) -> f64 {
    let p0 = 9.0 * soliton_power(1.0, T0, BETA2);
    let run = || -> crate::Result<f64> {
        let start = Pulse::sech(p0, T0, 1024, 40.0 * T0)?;
        let end = soliton_guide().propagate(&start, soliton_period(T0, BETA2), steps)?;
        Ok(end
            .power()
            .iter()
            .zip(start.power())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max)
            / p0)
    };
    run().unwrap_or(f64::NAN)
}

/// The steps of the soliton cases.
const SOLITON_STEPS: usize = 4000;
const PERIOD_STEPS: usize = 8000;

fn fundamental_case() -> Outcome {
    absolute(
        fundamental_soliton(SOLITON_STEPS),
        0.0,
        FUNDAMENTAL_TOLERANCE,
    )
}

fn second_order_case() -> Outcome {
    absolute(second_order(SOLITON_STEPS), 0.0, SECOND_ORDER_TOLERANCE)
}

fn third_order_case() -> Outcome {
    absolute(third_order_period(PERIOD_STEPS), 0.0, THIRD_ORDER_TOLERANCE)
}

const FUNDAMENTAL_TOLERANCE: f64 = 2e-5;
const SECOND_ORDER_TOLERANCE: f64 = 2e-5;
const THIRD_ORDER_TOLERANCE: f64 = 6e-5;

/// The split-step's order from the second-order soliton's errors on 1000 and 2000 steps, to
/// two decimals.
fn split_step_order() -> Outcome {
    let order = (second_order(1000) / second_order(2000)).log2();
    let shown = (order * 100.0).round() / 100.0;
    absolute(shown, 2.0, 0.1)
}

/// SPM alone (β₂ = 0) of a Gaussian, T₀ = 1 ps, over 2 cm at 3 dB/cm, γ = 1 /(W·m),
/// P₀ = 230 W, on `steps` steps: (the phase at the peak, γP₀L_eff).
pub(crate) fn spm_phase(steps: usize) -> (f64, f64) {
    let (p0, length) = (230.0, 0.02);
    let loss = loss_per_m(3.0);
    let run = || -> crate::Result<f64> {
        let guide = Guide::new(Dispersion::new(1.55, &[0.0])?, 1.0, loss);
        let start = Pulse::gaussian(p0, 1.0, 512, 64.0)?;
        let end = guide.propagate(&start, length, steps)?;
        let j = start.field.len() / 2;
        Ok((end.field[j] / start.field[j]).arg())
    };
    (
        run().unwrap_or(f64::NAN),
        spm_peak_phase(1.0, p0, loss, length),
    )
}

const SPM_STEPS: usize = 400;

fn spm_phase_case() -> Outcome {
    let (ours, closed) = spm_phase(SPM_STEPS);
    relative(ours, closed, SPM_PHASE_TOLERANCE)
}

const SPM_PHASE_TOLERANCE: f64 = 2e-6;

/// SPM alone of a lossless Gaussian (T₀ = 1 ps) to φ_max = 4.5π, on `points` across 64 ps:
/// (its RMS spectral width over the input's, Agrawal's Eq. (4.1.16)).
pub(crate) fn spm_broadening(points: usize) -> (f64, f64) {
    let phi = 4.5 * PI;
    let run = || -> crate::Result<f64> {
        let guide = Guide::new(Dispersion::new(1.55, &[0.0])?, 1.0, 0.0);
        let start = Pulse::gaussian(phi, 1.0, points, 64.0)?;
        let end = guide.propagate(&start, 1.0, 1)?;
        Ok(end.rms_bandwidth() / start.rms_bandwidth())
    };
    (run().unwrap_or(f64::NAN), spm_rms_broadening(phi))
}

fn spm_broadening_case() -> Outcome {
    let (ours, closed) = spm_broadening(1024);
    relative(ours, closed, SPM_BROADENING_TOLERANCE)
}

const SPM_BROADENING_TOLERANCE: f64 = 1e-12;

/// A third-order soliton (T₀ = 50 fs) with β₃ = 0.001 ps³/m, self-steepening and silica's
/// Raman response over 0.1 m on `steps` steps (1024 points across 3 ps): |N(L)/N(0) − 1| for
/// the photon number N ∝ Σ|Ã|²/(ω₀ + Ω), which the generalized equation conserves without
/// loss (Agrawal, p. 43); and the energy's change, which Raman takes.
pub(crate) fn photon_number(steps: usize) -> (f64, f64) {
    let t0 = 0.05;
    let run = || -> crate::Result<(f64, f64)> {
        let guide = Guide::new(Dispersion::new(1.55, &[BETA2, 0.001])?, 1.0, 0.0)
            .with_raman(Raman::SILICA)
            .with_self_steepening();
        let p0 = 9.0 * soliton_power(1.0, t0, BETA2);
        let start = Pulse::sech(p0, t0, 1024, 60.0 * t0)?;
        let end = guide.propagate(&start, 0.1, steps)?;
        let w0 = guide.dispersion.omega();
        Ok((
            (end.photons(w0) / start.photons(w0) - 1.0).abs(),
            end.energy() / start.energy() - 1.0,
        ))
    };
    run().unwrap_or((f64::NAN, f64::NAN))
}

fn photon_number_case() -> Outcome {
    absolute(photon_number(2000).0, 0.0, PHOTON_TOLERANCE)
}

const PHOTON_TOLERANCE: f64 = 2e-9;

// ---------------------------------------------------------------------------------------------
// Four-wave mixing
// ---------------------------------------------------------------------------------------------

/// The four-wave cases' pump: 1 W, γ = 1 /(W·m), 2 m.
const FWM_PUMP: f64 = 1.0;
const FWM_LENGTH: f64 = 2.0;

/// The signal's gain along the coupled equations, depleted and with every phase term, from
/// 10⁻¹² W against the 1 W pump with no idler, on `steps` RK4 steps: (ours, Agrawal's Eq.
/// (10.4.7)) at the linear mismatch `mismatch` (rad/m).
pub(crate) fn fwm_gain(mismatch: f64, steps: usize) -> (f64, f64) {
    let fw = FourWave {
        gamma: 1.0,
        mismatch,
    };
    let start = Waves {
        pump: c64::new(FWM_PUMP.sqrt(), 0.0),
        signal: c64::new(1e-6, 0.0),
        idler: c64::new(0.0, 0.0),
    };
    let ours = fw
        .propagate(start, FWM_LENGTH, steps)
        .map(|w| w.signal.norm_sqr() / start.signal.norm_sqr())
        .unwrap_or(f64::NAN);
    (ours, signal_gain(1.0, FWM_PUMP, mismatch, FWM_LENGTH))
}

const FWM_STEPS: usize = 800;

fn fwm_matched_case() -> Outcome {
    // κ = Δk + 2γP = 0
    let (ours, closed) = fwm_gain(-2.0, FWM_STEPS);
    relative(ours, closed, FWM_MATCHED_TOLERANCE)
}

fn fwm_mismatched_case() -> Outcome {
    // κ/2 = 1.5 > γP = 1: the gain is imaginary, the signal oscillates
    let (ours, closed) = fwm_gain(1.0, FWM_STEPS);
    relative(ours, closed, FWM_MISMATCHED_TOLERANCE)
}

const FWM_MATCHED_TOLERANCE: f64 = 2e-9;
const FWM_MISMATCHED_TOLERANCE: f64 = 4e-11;

/// Depleted four-wave mixing, a 0.1 W signal against the 1 W pump, κ = 0, on `steps` steps:
/// the larger drift of |A_p|² + |A_s|² + |A_i|² and of |A_s|² − |A_i|², over the power in.
pub(crate) fn fwm_invariants(steps: usize) -> f64 {
    let fw = FourWave {
        gamma: 1.0,
        mismatch: -2.0,
    };
    let start = Waves {
        pump: c64::new(1.0, 0.0),
        signal: c64::new(0.1f64.sqrt(), 0.0),
        idler: c64::new(0.0, 0.0),
    };
    let Ok(end) = fw.propagate(start, FWM_LENGTH, steps) else {
        return f64::NAN;
    };
    let total = (end.power() - start.power()).abs();
    let flux = (end.signal.norm_sqr() - end.idler.norm_sqr() - start.signal.norm_sqr()).abs();
    total.max(flux) / start.power()
}

fn fwm_invariants_case() -> Outcome {
    absolute(fwm_invariants(FWM_STEPS), 0.0, FWM_INVARIANT_TOLERANCE)
}

const FWM_INVARIANT_TOLERANCE: f64 = 5e-10;

/// The same gain from the NLSE: a 1 W CW pump and a weak signal at the gain's peak,
/// Ω = (2γP/|β₂|)^½, in one envelope (Agrawal Eq. (10.2.26)), on `steps` split steps (128
/// points across a window of ten of Ω's periods): (the signal's gain, Eq. (10.4.7) with
/// Δk = β₂Ω² from the dispersion).
pub(crate) fn fwm_nlse(steps: usize) -> (f64, f64) {
    let omega = (2.0 * FWM_PUMP / BETA2.abs()).sqrt();
    let window = 10.0 * 2.0 * PI / omega;
    let points = 128;
    let run = || -> crate::Result<(f64, f64)> {
        let d = Dispersion::new(1.55, &[BETA2])?;
        let mismatch = d.four_wave_mismatch(omega);
        let guide = Guide::new(d, 1.0, 0.0);
        let a = 1e-6;
        let start = Pulse::sampled(points, window, |t| {
            c64::new(FWM_PUMP.sqrt(), 0.0) + c64::new(0.0, -omega * t).exp() * a
        })?;
        let end = guide.propagate(&start, FWM_LENGTH, steps)?;
        // the signal's bin: Ω = 10·2π/window
        let bin = |p: &Pulse| p.spectrum()[10].norm_sqr();
        Ok((
            bin(&end) / bin(&start),
            signal_gain(1.0, FWM_PUMP, mismatch, FWM_LENGTH),
        ))
    };
    run().unwrap_or((f64::NAN, f64::NAN))
}

const FWM_NLSE_STEPS: usize = 2000;

fn fwm_nlse_case() -> Outcome {
    let (ours, closed) = fwm_nlse(FWM_NLSE_STEPS);
    relative(ours, closed, FWM_NLSE_TOLERANCE)
}

const FWM_NLSE_TOLERANCE: f64 = 3e-6;

/// Spontaneous four-wave mixing of a 1.55 µm pump into 1.50 µm and its idler over 1 cm,
/// the signal's, idler's and pump's group indices 2.05, 1.96 and 2.00, a Gaussian pump 1e-3
/// /µm wide (0.24 nm) and Gaussian phase matching: (our purity on an `n` × `n` grid ±0.025 /µm,
/// the closed form). With linear indices Δk = k_s + k_i − 2k_p((ν_s + ν_i)/2) is linear in the
/// frequencies, so the joint amplitude is exactly a two-dimensional Gaussian.
pub(crate) fn sfwm_purity(n: usize) -> (f64, f64) {
    let length = 10_000.0;
    let (gs, gi, gp) = (2.05, 1.96, 2.00);
    let (lp, ls) = (1.55, 1.50);
    let (nup, nus) = (1.0 / lp, 1.0 / ls);
    let nui = 2.0 * nup - nus;
    // the phase matched at the centre: (g_s − g_p)ν_s + (g_i − g_p)ν_i + b_s + b_i − 2b_p = 0
    let (bp, bs) = (-0.10, -0.08);
    let bi = 2.0 * bp - bs - (gs - gp) * nus - (gi - gp) * nui;
    let run = || -> crate::Result<f64> {
        let pump = Wave::new(Wavelength::um(lp)?, linear_index(gp, bp)?);
        let process = Process::four_wave(
            pump.clone(),
            pump,
            Wave::new(Wavelength::um(ls)?, linear_index(gs, bs)?),
            linear_index(gi, bi)?,
        )?;
        let device = Device::new(process, length, State::default())?;
        let pump = Pump {
            center: nup,
            bandwidth: 1e-3,
        };
        let jsa = joint_spectrum(&device, pump.two_photon(), Matching::Gaussian, 0.025, n)?;
        Ok(jsa.schmidt()?.purity())
    };
    // ln f = −2 ln 2 (δs + δi)²/(√2Δν)² − γ (πL)² (A δs + B δi)², A = g_s − g_p, B = g_i − g_p
    let p = std::f64::consts::LN_2 / 1e-6;
    let g = gaussian_gamma() * (PI * length).powi(2);
    let (a, b) = (gs - gp, gi - gp);
    let closed = gaussian_purity(p + g * a * a, p + g * b * b, p + g * a * b);
    (run().unwrap_or(f64::NAN), closed)
}

fn sfwm_purity_case() -> Outcome {
    let (ours, closed) = sfwm_purity(201);
    relative(ours, closed, SFWM_TOLERANCE)
}

const SFWM_TOLERANCE: f64 = 1e-11;

// ---------------------------------------------------------------------------------------------
// The Lugiato–Lefever equation
// ---------------------------------------------------------------------------------------------

/// The bistable resonator of the homogeneous cases: ζ₀ = 3, f² = 4, between the turning
/// points' drives (f² from 2.91 to 5.09), so three homogeneous states.
const LLE_DETUNING: f64 = 3.0;
const LLE_DRIVE: f64 = 2.0;

/// The homogeneous state the split-step settles on from ψ = 0 (`upper` false) or from 1.2
/// times the upper state's field, over τ = 40 on `steps` steps: (its power, the cubic's root
/// it should be, the lowest or the highest).
pub(crate) fn lle_homogeneous(upper: bool, steps: usize) -> (f64, f64) {
    let run = || -> crate::Result<(f64, f64)> {
        let r = Resonator::quadratic(LLE_DETUNING, LLE_DRIVE, 0.02, 4)?;
        let roots = r.homogeneous();
        let target = if upper {
            roots[roots.len() - 1]
        } else {
            roots[0]
        };
        let start = if upper {
            r.homogeneous_field(target) * 1.2
        } else {
            c64::new(0.0, 0.0)
        };
        let end = r.evolve(&[start; 4], 40.0, steps)?;
        Ok((end[0].norm_sqr(), target))
    };
    run().unwrap_or((f64::NAN, f64::NAN))
}

const LLE_STEPS: usize = 12_800;

fn lle_homogeneous_case(upper: bool) -> Outcome {
    let (ours, root) = lle_homogeneous(upper, LLE_STEPS);
    relative(ours, root, LLE_HOMOGENEOUS_TOLERANCE)
}

fn lle_lower() -> Outcome {
    lle_homogeneous_case(false)
}

fn lle_upper() -> Outcome {
    lle_homogeneous_case(true)
}

const LLE_HOMOGENEOUS_TOLERANCE: f64 = 3e-5;

/// The growth of a perturbation of the homogeneous state X = 1.5 at ζ₀ = 1 in the modes ±5,
/// d₂ = 0.05 (64 modes), measured between τ = 10 and 20 on `steps` steps each: (ours, the
/// larger root of Lugiato and Lefever's λ² + 2λ + ζ = 0 with their Eq. (4)).
pub(crate) fn lle_instability(steps: usize) -> (f64, f64) {
    let (z, x, n, mode) = (1.0, 1.5, 64usize, 5i64);
    let run = || -> crate::Result<(f64, f64)> {
        let r = Resonator::quadratic(z, crate::nonlinear::resonator::drive_for(z, x), 0.05, n)?;
        let psi_s = r.homogeneous_field(x);
        let start: Vec<c64> = (0..n)
            .map(|j| psi_s + 1e-9 * (2.0 * PI * mode as f64 * j as f64 / n as f64).cos())
            .collect();
        let a = r.evolve(&start, 10.0, steps)?;
        let b = r.evolve(&a, 10.0, steps)?;
        let amplitude = |v: &[c64]| {
            let m: c64 = v.iter().sum::<c64>() / n as f64;
            v.iter().map(|x| (x - m).norm_sqr()).sum::<f64>().sqrt()
        };
        Ok((
            (amplitude(&b) / amplitude(&a)).ln() / 10.0,
            r.instability_rate(x, mode),
        ))
    };
    run().unwrap_or((f64::NAN, f64::NAN))
}

const LLE_RATE_STEPS: usize = 6400;

fn lle_instability_case() -> Outcome {
    let (ours, closed) = lle_instability(LLE_RATE_STEPS);
    relative(ours, closed, LLE_RATE_TOLERANCE)
}

const LLE_RATE_TOLERANCE: f64 = 6e-6;

/// The Lugiato–Lefever split-step's order from the growth rate's errors on 800 and 1600 steps,
/// to two decimals.
fn lle_order() -> Outcome {
    let e = |n| {
        let (a, b) = lle_instability(n);
        (a / b - 1.0).abs()
    };
    let order = (e(800) / e(1600)).log2();
    let shown = (order * 100.0).round() / 100.0;
    absolute(shown, 2.0, 0.1)
}

/// Coen et al.'s resonator of Fig. 2 (Opt. Lett. 38, 37, p. 38): a critically coupled MgF₂ disc,
/// FSR 18.2 GHz, γ = 0.032 /(W·km), β₂ = −13 ps²/km, α = θ = 1.75 × 10⁻⁵, P_in = 55.6 mW,
/// L = 11.9 mm, δ₀ = 0.0012.
pub(crate) fn coen_2012_ring() -> crate::Result<Ring> {
    Ok(Ring {
        length: 0.0119,
        roundtrip: 1e12 / 18.2e9,
        loss: 1.75e-5,
        coupling: 1.75e-5,
        detuning: 0.0012,
        gamma: 0.032e-3,
        dispersion: Dispersion::new(1.55, &[-13e-3])?,
        pump: 0.0556,
    })
}

/// Its single soliton, settled from the approximate one on 512 modes with steps of `step`:
/// (its FWHM in fs, its peak power inside in W).
pub(crate) fn coen_2012_soliton(step: f64) -> (f64, f64) {
    let run = || -> crate::Result<(f64, f64)> {
        let ring = coen_2012_ring()?;
        let r = ring.resonator(512)?;
        let s = r
            .soliton(ring.d2())
            .ok_or_else(|| crate::Error::invalid("soliton", "none at this detuning and drive"))?;
        let (end, _) = r.settle(&r.soliton_field(&s), step, 0.5, 1e-10, 200.0)?;
        let peak = end.iter().map(|v| v.norm_sqr()).fold(0.0, f64::max);
        Ok((
            ring.time_of(peak_width(&end)?) * 1e3,
            peak * ring.power_scale(),
        ))
    };
    run().unwrap_or((f64::NAN, f64::NAN))
}

fn coen_2012_case() -> Outcome {
    let (ours, _) = coen_2012_soliton(4e-4);
    let (fine, _) = coen_2012_soliton(2e-4);
    note_numerical((fine - ours).abs() * 4.0 / 3.0);
    absolute(ours, 400.0, 50.0)
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    let mut cases = pulse_cases();
    cases.extend(fwm_cases());
    cases.extend(lle_cases());
    cases
}

fn pulse_cases() -> Vec<Case> {
    vec![
        Case {
            id: "nonlinear/nlse-dispersion-silica-beta2",
            title: r"A guide's $\beta_2$ from its index over wavelength, fitted by Chebyshev interpolants of $n(\omega)$ across 1.0–2.2 µm: silica (Malitson's formula) at 1.55 µm against the formula's own Taylor series ($\beta_2$ shown, ps²/m)",
            tier: Tier::Analytic,
            source: r"$\beta = \omega n(\omega)/c$, $\beta_k = d^k\beta/d\omega^k$ (Agrawal Eq. (2.3.24)), from the Taylor series of $n^2 = 1 + \sum B_i/(1 - C_i^2\omega^2/(2\pi c)^2)$",
            run: silica_beta2,
        },
        Case {
            id: "nonlinear/nlse-dispersion-silica-beta4",
            title: r"The same fit's $\beta_4$, the highest order the cases take ($\beta_4$ shown, ps⁴/m)",
            tier: Tier::Analytic,
            source: r"the formula's Taylor series, as above",
            run: silica_beta4,
        },
        Case {
            id: "nonlinear/nlse-fundamental-soliton",
            title: r"The fundamental soliton $\sqrt{P_0}\operatorname{sech}(T/T_0)$, $P_0 = \lvert\beta_2\rvert/\gamma T_0^2$ ($\beta_2 = -0.05$ ps²/m, $\gamma = 1$ /(W·m), $T_0 = 0.1$ ps), after five soliton periods by the symmetrized split-step (4000 steps, 1024 points): the largest difference from the closed form, over $\sqrt{P_0}$",
            tier: Tier::Analytic,
            source: r"Agrawal, Nonlinear Fiber Optics, 6th ed. (2019), doi:10.1016/C2018-0-01168-8, Eqs. (5.2.16) and (5.2.20): $u = \operatorname{sech}\tau\thinspace e^{i\xi/2}$",
            run: fundamental_case,
        },
        Case {
            id: "nonlinear/nlse-second-order-soliton",
            title: r"The second-order soliton $2\sqrt{P_0}\operatorname{sech}(T/T_0)$ at half its period ($\xi = \pi/4$), the same guide and grid: the largest difference from the closed form, over $2\sqrt{P_0}$",
            tier: Tier::Analytic,
            source: r"Agrawal (2019), doi:10.1016/C2018-0-01168-8, Eq. (5.2.23)",
            run: second_order_case,
        },
        Case {
            id: "nonlinear/nlse-third-order-period",
            title: r"The third-order soliton $3\sqrt{P_0}\operatorname{sech}(T/T_0)$ after one soliton period $z_0 = (\pi/2)T_0^2/\lvert\beta_2\rvert$ (8000 steps): the largest change of $\lvert A\rvert^2$ from the input's, over $9P_0$",
            tier: Tier::Analytic,
            source: r"Agrawal (2019), doi:10.1016/C2018-0-01168-8, Eq. (5.2.24) and p. 144: $\lvert u\rvert^2$ is periodic in $\xi$ with period $\pi/2$ for $N \geq 2$",
            run: third_order_case,
        },
        Case {
            id: "nonlinear/nlse-split-step-order",
            title: r"The symmetrized split-step's order, from the second-order soliton's errors on 1000 and 2000 steps (order shown)",
            tier: Tier::Analytic,
            source: r"Agrawal (2019), doi:10.1016/C2018-0-01168-8, Eq. (2.4.8) and p. 47: the leading error the double commutator, third order per step, second over a length",
            run: split_step_order,
        },
        Case {
            id: "nonlinear/nlse-spm-phase",
            title: r"Self-phase modulation alone ($\beta_2 = 0$) of a Gaussian over 2 cm at 3 dB/cm, $\gamma P_0 = 230$ /m (400 steps): the phase at the peak against $\gamma P_0 L_\mathrm{eff}$ (rad shown)",
            tier: Tier::Analytic,
            source: r"Agrawal (2019), doi:10.1016/C2018-0-01168-8, Eqs. (4.1.4)–(4.1.6): $\phi_\mathrm{max} = \gamma P_0 L_\mathrm{eff}$, $L_\mathrm{eff} = (1 - e^{-\alpha L})/\alpha$",
            run: spm_phase_case,
        },
        Case {
            id: "nonlinear/nlse-spm-broadening",
            title: r"A Gaussian's RMS spectral width after self-phase modulation alone to $\phi_\mathrm{max} = 4.5\pi$, over its own before (1024 points; ratio shown)",
            tier: Tier::Analytic,
            source: r"Agrawal (2019), doi:10.1016/C2018-0-01168-8, Eq. (4.1.16): $\Delta\omega_\mathrm{rms}/\Delta\omega_0 = (1 + 4\phi_\mathrm{max}^2/3\sqrt{3})^{1/2}$",
            run: spm_broadening_case,
        },
        Case {
            id: "nonlinear/nlse-photon-number",
            title: r"The generalized equation with $\beta_3$, self-steepening and silica's Raman response: a third-order 50 fs soliton over 0.1 m (2000 steps), the photon number's drift (shown); the energy falls by 3.4 %, which Raman takes",
            tier: Tier::Analytic,
            source: r"Agrawal (2019), doi:10.1016/C2018-0-01168-8, Eq. (2.3.36) and p. 43: without loss it conserves the number of photons, not the energy",
            run: photon_number_case,
        },
    ]
}

fn fwm_cases() -> Vec<Case> {
    vec![
        Case {
            id: "nonlinear/fwm-gain-matched",
            title: r"Four-wave mixing with one pump along the coupled equations, depleted and with every phase term (RK4, 800 steps): a 1 W pump, $\gamma = 1$ /(W·m), 2 m, at $\kappa = \Delta k + 2\gamma P = 0$, the signal's gain from $10^{-12}$ W (shown)",
            tier: Tier::Analytic,
            source: r"Agrawal (2019), doi:10.1016/C2018-0-01168-8, Eqs. (10.2.19), (10.2.21) and (10.4.7): $G = 1 + (\gamma P/g)^2\sinh^2 gL$, here $1 + \sinh^2 2$",
            run: fwm_matched_case,
        },
        Case {
            id: "nonlinear/fwm-gain-mismatched",
            title: r"The same at $\Delta k = 1$ /m, where $\lvert\kappa/2\rvert \gt \gamma P$ and $g$ is imaginary: the signal oscillates (gain shown)",
            tier: Tier::Analytic,
            source: r"Agrawal (2019), Eq. (10.4.7) continued, $\sinh(gL)/g \to \sin(\lvert g\rvert L)/\lvert g\rvert$; its limit Eq. (10.4.8)",
            run: fwm_mismatched_case,
        },
        Case {
            id: "nonlinear/fwm-invariants",
            title: r"Depleted four-wave mixing, a 0.1 W signal against the 1 W pump: the larger drift of the total power and of $P_s - P_i$ over the power in (shown)",
            tier: Tier::Analytic,
            source: r"Agrawal (2019), Eqs. (10.2.1)–(10.2.4) with one pump and one $\gamma$: both are conserved (p. 422, $d(P_3 - P_4)/dz = 0$)",
            run: fwm_invariants_case,
        },
        Case {
            id: "nonlinear/fwm-nlse-gain",
            title: r"The same gain from the NLSE: a 1 W CW pump and a weak signal at the gain's peak $\Omega = (2\gamma P/\lvert\beta_2\rvert)^{1/2}$ in one envelope ($\beta_2 = -0.05$ ps²/m, 2 m, 2000 split steps), against the closed form with $\Delta k = \beta_2\Omega^2$ from the dispersion (gain shown)",
            tier: Tier::Analytic,
            source: r"Agrawal (2019), Eqs. (10.2.26), (10.4.7) and (10.4.12)",
            run: fwm_nlse_case,
        },
        Case {
            id: "nonlinear/fwm-sfwm-purity",
            title: r"Photon pairs by spontaneous four-wave mixing: a 1.55 µm pump into 1.50 µm and its idler over 1 cm, group indices 2.05, 1.96 and 2.00, a Gaussian pump 0.24 nm wide and Gaussian phase matching; the joint spectrum's purity on 201 × 201 points (shown)",
            tier: Tier::Analytic,
            source: r"the purity of the two-dimensional Gaussian $\exp(-\frac{1}{2}z^\mathsf{T}Mz)$, $P = (1 - c^2/ab)^{1/2}$; the two pump photons' spectrum the pump's convolved with itself",
            run: sfwm_purity_case,
        },
    ]
}

fn lle_cases() -> Vec<Case> {
    vec![
        Case {
            id: "nonlinear/lle-homogeneous-lower",
            title: r"The Lugiato–Lefever equation by split-step, bistable at $\zeta_0 = 3$, $f^2 = 4$: from $\psi = 0$ it settles on the lower homogeneous state (12 800 steps over $\tau = 40$; $\lvert\psi\rvert^2$ shown)",
            tier: Tier::Analytic,
            source: r"L. A. Lugiato, R. Lefever, Phys. Rev. Lett. 58, 2209 (1987), doi:10.1103/PhysRevLett.58.2209, Eq. (2): $E_I^2 = \lvert E_s\rvert^2(1 + (\lvert E_s\rvert^2 - \theta)^2)$, its lowest root",
            run: lle_lower,
        },
        Case {
            id: "nonlinear/lle-homogeneous-upper",
            title: r"The same, from 1.2 times the upper state's field: it settles on the upper state ($\lvert\psi\rvert^2$ shown)",
            tier: Tier::Analytic,
            source: r"Lugiato and Lefever (1987), doi:10.1103/PhysRevLett.58.2209, Eq. (2), its highest root; S shaped for $\theta \gt \sqrt{3}$",
            run: lle_upper,
        },
        Case {
            id: "nonlinear/lle-instability-rate",
            title: r"The growth rate of a perturbation of the homogeneous state $\lvert\psi\rvert^2 = 1.5$ at $\zeta_0 = 1$ in the modes $\pm 5$ ($d_2 = 0.05$, 64 modes), measured between $\tau = 10$ and 20 (6400 steps each; rate shown)",
            tier: Tier::Analytic,
            source: r"Lugiato and Lefever (1987), doi:10.1103/PhysRevLett.58.2209, Eq. (4): $\lambda^2 + 2\lambda + \zeta = 0$, $\zeta = 1 + (X - \theta)(3X - \theta) + a(a - 2(2X - \theta))$",
            run: lle_instability_case,
        },
        Case {
            id: "nonlinear/lle-split-step-order",
            title: r"The Lugiato–Lefever split-step's order, from the growth rate's errors on 800 and 1600 steps (order shown)",
            tier: Tier::Analytic,
            source: r"Strang splitting with each part solved exactly: second order",
            run: lle_order,
        },
        Case {
            id: "nonlinear/lle-coen-2012-duration",
            title: r"Coen et al.'s MgF₂ resonator (FSR 18.2 GHz, $\gamma = 0.032$ /(W·km), $\beta_2 = -13$ ps²/km, $\alpha = \theta = 1.75\times10^{-5}$, $P_\mathrm{in} = 55.6$ mW, $L = 11.9$ mm, $\delta_0 = 0.0012$): its single soliton settled by split-step on 512 modes, the FWHM against their '∼400 fs' (fs shown)",
            tier: Tier::Published,
            source: r"S. Coen, H. G. Randle, T. Sylvestre, M. Erkintalo, Opt. Lett. 38, 37 (2012), doi:10.1364/OL.38.000037, Fig. 2 and p. 38, '∼400 fs', read as one significant digit (±50 fs)",
            run: coen_2012_case,
        },
    ]
}

#[cfg(test)]
#[path = "kerr_checks_tests.rs"]
mod tests;
