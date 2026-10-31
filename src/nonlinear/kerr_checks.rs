//! The validation cases of third-order processes: the nonlinear Schrödinger equation
//! ([`super::pulse`]), four-wave mixing ([`super::fwm`]) and the Lugiato–Lefever equation
//! ([`super::resonator`]), against closed forms.

use std::f64::consts::PI;

use num_complex::Complex64 as c64;

use crate::material::{Model, silica};
use crate::nonlinear::pulse::{
    C_M_PER_PS, Dispersion, Guide, Pulse, Raman, angular_frequency, dispersion_length, loss_per_m,
    second_order_soliton, soliton_period, soliton_power, spm_peak_phase, spm_rms_broadening,
};
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

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

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    pulse_cases()
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

#[cfg(test)]
#[path = "kerr_checks_tests.rs"]
mod tests;
