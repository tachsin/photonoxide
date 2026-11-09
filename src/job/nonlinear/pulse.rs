//! A four-wave mixing task's `[task.pulse]`: a pulse along the task's Kerr guide by the NLSE's
//! split-step ([`crate::nonlinear::pulse`]), and with `[task.pulse.second]` a second pulse at the
//! signal's wavelength beside it, the two coupled by cross-phase modulation
//! ([`crate::nonlinear::xpm`]).

use std::f64::consts::TAU;

use num_complex::Complex64 as c64;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Kind, Setup, task_error};
use crate::Result;
use crate::nonlinear::pulse::{
    Dispersion, FreeCarriers, Guide, Pulse, Raman, TwoPhoton, dispersion_length, nonlinear_length,
    soliton_order,
};
use crate::nonlinear::xpm::TwoPulses;

/// `[task.pulse]`: a pulse launched into four-wave mixing's guide at the pump's wavelength, its
/// γ the guide's `gamma_per_w_m`, its loss the pump's, over the task's `length_mm`.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct PulseSpec {
    /// `"sech"` (the default), √P₀ sech(T/T₀), or `"gaussian"`, √P₀ exp(−T²/2T₀²).
    shape: Option<String>,
    /// The peak power P₀, W.
    peak_power_w: f64,
    /// T₀, ps.
    t0_ps: f64,
    /// β₂, β₃, … at the pump's wavelength, ps^k/m (1 ps²/m is 1000 ps²/km): at least β₂.
    dispersion_ps_per_m: Vec<f64>,
    /// The time window, ps: 50 T₀ by default. The grid is periodic, so the window must hold the
    /// pulse and what it sheds.
    window_ps: Option<f64>,
    /// The points across the window, a power of two: 1024 by default.
    points: Option<usize>,
    /// The split steps over the length: 2000 by default.
    steps: Option<usize>,
    /// Self-steepening, γ/ω₀ (false by default).
    self_steepening: Option<bool>,
    /// Silica's delayed Raman response (Agrawal's Eq. (2.3.39); false by default).
    raman: Option<bool>,
    /// Two-photon absorption, and the free carriers it makes (Lin, Painter and Agrawal 2007).
    two_photon: Option<TwoPhotonSpec>,
    /// A second pulse at the signal's wavelength, coupled to the first by cross-phase
    /// modulation (Kerr only: no Raman, self-steepening or two-photon absorption with it).
    second: Option<SecondSpec>,
}

/// `[task.pulse.two_photon]`.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TwoPhotonSpec {
    /// β_T, m/W (0.5 cm/GW is 5e-12 m/W).
    coefficient_m_per_w: f64,
    /// The mode's effective area ā, µm².
    area_um2: f64,
    /// The free carriers, silicon's (σ_a and σ_n of Lin et al.'s Eq. (20), scaled by
    /// (λ/1.55 µm)²); none by default.
    carriers: Option<CarriersSpec>,
}

/// `[task.pulse.two_photon.carriers]`: silicon's free carriers.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CarriersSpec {
    /// The effective lifetime τ₀, ps.
    lifetime_ps: f64,
    /// n₀/n, the material's index over the mode's (1 by default).
    index_ratio: Option<f64>,
}

/// `[task.pulse.second]`: a second pulse at the signal's wavelength, centred `delay_ps` after
/// the first, on the same grid; its loss the signal's.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SecondSpec {
    /// `"sech"` or `"gaussian"`: the first pulse's shape by default.
    shape: Option<String>,
    /// Its peak power, W.
    peak_power_w: f64,
    /// Its T₀, ps.
    t0_ps: f64,
    /// Its β₂, β₃, … at the signal's wavelength, ps^k/m.
    dispersion_ps_per_m: Vec<f64>,
    /// The walk-off d = 1/v_g2 − 1/v_g1, ps/m: positive when the second pulse is slower.
    walk_off_ps_per_m: f64,
    /// Where it is centred at the start, ps after the first (0 by default).
    delay_ps: Option<f64>,
    /// Its γ, 1/(W·m): the guide's times the pump's wavelength over the signal's by default
    /// (one effective area and n₂ for both).
    gamma_per_w_m: Option<f64>,
}

/// A pulse at the guide's end against the same pulse at its start ([`super::Event::NonlinearPulse`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PulsePropagation {
    /// `"pulse"`, or `"first pulse"` and `"second pulse"` of two.
    pub label: String,
    /// Its carrier's vacuum wavelength, µm.
    pub wavelength_um: f64,
    /// The guide's length, mm.
    pub length_mm: f64,
    /// The split steps.
    pub steps: usize,
    /// The times T, ps, in the frame of the first pulse's group velocity.
    pub times_ps: Vec<f64>,
    /// |A|² at the start, W.
    pub power_in_w: Vec<f64>,
    /// |A|² at the end, W.
    pub power_out_w: Vec<f64>,
    /// The phase the pulse gained, arg(A_out A_in*), rad: 0 where the pulse at the start is
    /// below 1e-12 of its peak.
    pub phase_rad: Vec<f64>,
    /// The frequencies from the carrier, Ω/2π, THz, increasing (positive is bluer).
    pub frequencies_thz: Vec<f64>,
    /// |Ã(Ω)|² at the start, over its largest.
    pub spectrum_in: Vec<f64>,
    /// |Ã(Ω)|² at the end, over the start's largest.
    pub spectrum_out: Vec<f64>,
    /// The peak power at the start and the end, W.
    pub peak_power_w: [f64; 2],
    /// The energy ∫|A|² dT at the start and the end, pJ.
    pub energy_pj: [f64; 2],
    /// The RMS spectral width at the start and the end, THz (Δω_rms/2π).
    pub rms_bandwidth_thz: [f64; 2],
    /// The guide's lengths for this pulse, and what it carries.
    pub notes: Vec<String>,
}

/// A shape's pulse.
fn launched(
    shape: Option<&str>,
    peak: f64,
    t0: f64,
    delay: f64,
    points: usize,
    window: f64,
) -> Result<Pulse> {
    if !(peak.is_finite() && peak >= 0.0 && t0.is_finite() && t0 > 0.0 && delay.is_finite()) {
        return Err(task_error(format!(
            "[task.pulse] needs a peak power of at least 0, a positive t0_ps and a finite delay, \
             got {peak} W, {t0} ps, {delay} ps"
        )));
    }
    let a = peak.sqrt();
    match shape.unwrap_or("sech") {
        "sech" => Pulse::sampled(points, window, |t| {
            c64::new(a / ((t - delay) / t0).cosh(), 0.0)
        }),
        "gaussian" => Pulse::sampled(points, window, |t| {
            c64::new(a * (-0.5 * ((t - delay) / t0).powi(2)).exp(), 0.0)
        }),
        other => Err(task_error(format!(
            "[task.pulse] unknown shape \"{other}\": sech or gaussian"
        ))),
    }
}

/// A dispersion of at least β₂ at `wavelength_um`.
fn dispersion(wavelength_um: f64, betas: &[f64], what: &str) -> Result<Dispersion> {
    if betas.is_empty() {
        return Err(task_error(format!(
            "{what} dispersion_ps_per_m needs at least beta2 (0 for none)"
        )));
    }
    Dispersion::new(wavelength_um, betas).map_err(|e| task_error(format!("{what} {e}")))
}

/// What a pulse was at the start and is at the end.
fn record(
    label: &str,
    wavelength_um: f64,
    length_mm: f64,
    steps: usize,
    start: &Pulse,
    end: &Pulse,
    notes: Vec<String>,
) -> PulsePropagation {
    let peak = start.peak_power();
    let phase = start
        .field
        .iter()
        .zip(&end.field)
        .map(|(a, b)| {
            if a.norm_sqr() > 1e-12 * peak {
                (b * a.conj()).arg()
            } else {
                0.0
            }
        })
        .collect();
    // the spectrum in increasing frequency: the FFT's order turned at its middle
    let n = start.field.len();
    let order: Vec<usize> = (n / 2..n).chain(0..n / 2).collect();
    let (s0, s1) = (start.spectrum(), end.spectrum());
    let top = s0.iter().map(|v| v.norm_sqr()).fold(0.0, f64::max);
    let scale = if top > 0.0 { 1.0 / top } else { 1.0 };
    let w = start.frequencies();
    PulsePropagation {
        label: label.to_owned(),
        wavelength_um,
        length_mm,
        steps,
        times_ps: start.times(),
        power_in_w: start.power(),
        power_out_w: end.power(),
        phase_rad: phase,
        frequencies_thz: order.iter().map(|&k| w[k] / TAU).collect(),
        spectrum_in: order.iter().map(|&k| s0[k].norm_sqr() * scale).collect(),
        spectrum_out: order.iter().map(|&k| s1[k].norm_sqr() * scale).collect(),
        peak_power_w: [peak, end.peak_power()],
        energy_pj: [start.energy(), end.energy()],
        rms_bandwidth_thz: [start.rms_bandwidth() / TAU, end.rms_bandwidth() / TAU],
        notes,
    }
}

/// The lengths that say what a pulse does over the guide: L_D, L_NL and the soliton order.
fn lengths(gamma: f64, peak: f64, t0: f64, beta2: f64, length_m: f64) -> String {
    let ld = dispersion_length(t0, beta2);
    let lnl = nonlinear_length(gamma, peak);
    format!(
        "over {length_m} m: the dispersion length T0^2/|beta2| is {} m, the nonlinear length \
         1/(gamma P0) {} m, the soliton order {:.4}",
        if ld.is_finite() {
            format!("{ld:.6}")
        } else {
            "infinite".into()
        },
        if lnl.is_finite() {
            format!("{lnl:.6}")
        } else {
            "infinite".into()
        },
        soliton_order(gamma, peak, t0, beta2)
    )
}

/// The task's pulses at the guide's end, or none without a `[task.pulse]`.
pub(super) fn pulses(spec: Option<&PulseSpec>, setup: &Setup) -> Result<Vec<PulsePropagation>> {
    let Some(spec) = spec else {
        return Ok(Vec::new());
    };
    if setup.kind != Kind::Fwm {
        return Err(task_error(
            "[task.pulse] runs in four-wave mixing's Kerr guide: give process = \"fwm\" and the \
             guide's gamma_per_w_m",
        ));
    }
    let Some(gamma) = setup.gamma else {
        return Err(task_error(
            "[task.pulse] needs the guide's gamma_per_w_m, the pulse's Kerr coefficient",
        ));
    };
    let points = spec.points.unwrap_or(1024);
    let steps = spec.steps.unwrap_or(2000);
    if steps == 0 {
        return Err(task_error("[task.pulse] steps must be at least 1"));
    }
    let window = spec.window_ps.unwrap_or(50.0 * spec.t0_ps);
    let length_m = setup.length_um * 1e-6;
    let length_mm = setup.length_um * 1e-3;
    let (pump_um, signal_um) = (setup.waves[0], setup.waves[1]);
    // the losses per µm, per metre
    let (loss_p, loss_s) = (setup.loss[0] * 1e6, setup.loss[1] * 1e6);
    let mut guide = Guide::new(
        dispersion(pump_um, &spec.dispersion_ps_per_m, "[task.pulse]")?,
        gamma,
        loss_p,
    );
    if spec.self_steepening == Some(true) {
        guide = guide.with_self_steepening();
    }
    if spec.raman == Some(true) {
        guide = guide.with_raman(Raman::SILICA);
    }
    let start = launched(
        spec.shape.as_deref(),
        spec.peak_power_w,
        spec.t0_ps,
        0.0,
        points,
        window,
    )
    .map_err(|e| task_error(format!("[task.pulse] {e}")))?;
    let beta2 = spec.dispersion_ps_per_m[0];
    let mut notes = vec![lengths(
        gamma,
        spec.peak_power_w,
        spec.t0_ps,
        beta2,
        length_m,
    )];
    if let Some(second) = &spec.second {
        if spec.self_steepening == Some(true)
            || spec.raman == Some(true)
            || spec.two_photon.is_some()
        {
            return Err(task_error(
                "[task.pulse.second] couples two pulses by the Kerr effect alone: leave out \
                 self_steepening, raman and two_photon",
            ));
        }
        if !second.walk_off_ps_per_m.is_finite() {
            return Err(task_error(
                "[task.pulse.second] walk_off_ps_per_m must be a number",
            ));
        }
        let gamma2 = match second.gamma_per_w_m {
            Some(g) if g.is_finite() && g >= 0.0 => g,
            Some(g) => {
                return Err(task_error(format!(
                    "[task.pulse.second] gamma_per_w_m must be at least 0, got {g}"
                )));
            }
            None => gamma * pump_um / signal_um,
        };
        let other = Guide::new(
            dispersion(
                signal_um,
                &second.dispersion_ps_per_m,
                "[task.pulse.second]",
            )?,
            gamma2,
            loss_s,
        );
        let start2 = launched(
            second.shape.as_deref().or(spec.shape.as_deref()),
            second.peak_power_w,
            second.t0_ps,
            second.delay_ps.unwrap_or(0.0),
            points,
            window,
        )
        .map_err(|e| task_error(format!("[task.pulse.second] {e}")))?;
        let pair = TwoPulses::new(guide, other, second.walk_off_ps_per_m);
        let (end, end2) = pair
            .propagate(&start, &start2, length_m, steps)
            .map_err(|e| task_error(format!("[task.pulse] {e}")))?;
        let walk = second.walk_off_ps_per_m * length_m;
        notes.push(format!(
            "with the second pulse: cross-phase factor 2 (co-polarized), the second pulse \
             walking {walk} ps over the guide"
        ));
        let notes2 = vec![
            lengths(
                gamma2,
                second.peak_power_w,
                second.t0_ps,
                second.dispersion_ps_per_m[0],
                length_m,
            ),
            format!("its gamma {gamma2} /(W m), walking {walk} ps over the guide"),
        ];
        return Ok(vec![
            record(
                "first pulse",
                pump_um,
                length_mm,
                steps,
                &start,
                &end,
                notes,
            ),
            record(
                "second pulse",
                signal_um,
                length_mm,
                steps,
                &start2,
                &end2,
                notes2,
            ),
        ]);
    }
    let end = match &spec.two_photon {
        None => guide.propagate(&start, length_m, steps),
        Some(t) => {
            let carriers = t.carriers.as_ref().map(|c| {
                FreeCarriers::silicon(pump_um, c.lifetime_ps, c.index_ratio.unwrap_or(1.0))
            });
            notes.push(format!(
                "two-photon absorption: beta_T {} m/W over an area of {} um^2{}",
                t.coefficient_m_per_w,
                t.area_um2,
                if carriers.is_some() {
                    ", with silicon's free carriers from none at the window's start"
                } else {
                    ", without free carriers"
                }
            ));
            guide
                .with_two_photon(TwoPhoton {
                    coefficient: t.coefficient_m_per_w,
                    area: t.area_um2 * 1e-12,
                    carriers,
                })
                .propagate(&start, length_m, steps)
        }
    }
    .map_err(|e| task_error(format!("[task.pulse] {e}")))?;
    Ok(vec![record(
        "pulse", pump_um, length_mm, steps, &start, &end, notes,
    )])
}
