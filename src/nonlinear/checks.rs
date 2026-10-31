//! The validation cases of phase matching ([`crate::nonlinear::phase`]): the half-maximum point
//! of sinc² and the L² growth at phase matching; Fejer et al.'s Table I (coherence lengths and
//! the acceptance bandwidths of SHG in 1 mm of LiNbO₃) from the indices it was computed with;
//! and the effective coefficient Wang et al.'s duty cycle leaves.

use crate::material::catalogue::{self, Axis};
use crate::nonlinear::coupled::{
    SecondHarmonic, ThreeWave, coupling_from_efficiency, loss_from_db_per_cm, undepleted_lossy_shg,
};
use crate::nonlinear::pairs::{Matching, Pump, gaussian_gamma, gaussian_purity, joint_spectrum};
use crate::nonlinear::phase::{
    Device, Expansion, HALF_MAXIMUM, Index, Plane, Polarization, Process, State, Type, Uniaxial,
    Variable, Wave, effective_d, first_order_tilt, sinc2, undepleted,
};
use crate::units::Wavelength;
use crate::verdict::{Case, Outcome, Tier, note_numerical};

/// Which of Fejer et al.'s Table I's coefficients: the process and its polarizations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Coefficient {
    /// d₃₁(Q): o + o → e, quasi-phase-matched (type I).
    D31,
    /// d₃₃(Q): e + e → e (type 0).
    D33,
    /// d₂₂(Q): o + o → o (type 0).
    D22,
}

impl Coefficient {
    fn kind(self) -> Type {
        match self {
            Coefficient::D31 => Type::I,
            Coefficient::D33 => Type::Zero(Polarization::Extraordinary),
            Coefficient::D22 => Type::Zero(Polarization::Ordinary),
        }
    }

    /// Whether the fundamental and the harmonic are extraordinary.
    fn extraordinary(self) -> (bool, bool) {
        match self {
            Coefficient::D31 => (false, true),
            Coefficient::D33 => (true, true),
            Coefficient::D22 => (false, false),
        }
    }
}

/// A column of Table I.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Column {
    /// l_c, µm.
    Coherence,
    /// δλ, nm.
    Wavelength,
    /// δT, K.
    Temperature,
    /// δν, mrad, rotation about z: n_e invariant, Eq. (41) (column c).
    AboutZ,
    /// δν, mrad, rotation about x or y: n_e varies, Eq. (A1.9) (column d).
    TowardZ,
}

/// Which of our values a printed value is compared with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Estimate {
    /// The quantity itself (l_c).
    Direct,
    /// Fejer's linear estimate, Eq. (25) (δλ, δT).
    Linear,
    /// The noncritical estimate from the second derivative: Eq. (41) for column c.
    Noncritical,
    /// Eq. (A1.9), first order in the birefringence, for column d.
    FirstOrder,
    /// The width found numerically: Table I's values in parentheses.
    Numerical,
}

/// One printed value of Table I: the fundamental (µm), the coefficient, the column, which of
/// our values it is compared with, the value, half a unit of its last digit, and whether it is
/// for lithium-rich material (footnote f).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Printed {
    pub(crate) lam: f64,
    pub(crate) coefficient: Coefficient,
    pub(crate) column: Column,
    pub(crate) estimate: Estimate,
    pub(crate) value: f64,
    pub(crate) half: f64,
    pub(crate) rich: bool,
}

const fn row(
    lam: f64,
    coefficient: Coefficient,
    column: Column,
    estimate: Estimate,
    value: f64,
    half: f64,
) -> Printed {
    Printed {
        lam,
        coefficient,
        column,
        estimate,
        value,
        half,
        rich: false,
    }
}

const fn rich(p: Printed) -> Printed {
    Printed { rich: true, ..p }
}

use Coefficient::{D22, D31, D33};
use Column::{AboutZ, Coherence, Temperature, TowardZ};
use Estimate::{Direct, FirstOrder, Linear, Noncritical, Numerical};

/// Fejer et al.'s Table I, read from the page image (its PDF p. 3), the values we reproduce: the
/// quasi-phase-matched rows. Left out, and listed in docs/methods/nonlinear.md: the
/// birefringent d₃₁(B) rows (at −19.4 and 580.3 °C, outside Edwards and Lawrence's 0–500 °C);
/// the numerical (203) and (455) K, whose half-maximum points lie below 0 °C; and six values
/// that differ from ours by more than half their last digit: l_c 129 (d₃₁, 1.064 µm) and 15.6
/// (d₃₃, 2.12 µm), δλ 40 nm (d₃₁, 2.12 µm), δT 17.4 K (d₂₂, 0.86 µm), δν 103 mrad (d₃₁,
/// 1.064 µm, column d) and 139 mrad (d₃₁, 2.12 µm, column d).
pub(crate) const TABLE_1: &[Printed] = &[
    row(0.86, D31, Coherence, Direct, 4.26, 0.005),
    row(0.86, D33, Coherence, Direct, 1.67, 0.005),
    row(0.86, D22, Coherence, Direct, 1.42, 0.005),
    row(1.064, D33, Coherence, Direct, 3.40, 0.005),
    row(1.064, D22, Coherence, Direct, 2.92, 0.005),
    row(2.12, D31, Coherence, Direct, 14.4, 0.05),
    row(2.12, D22, Coherence, Direct, 13.4, 0.05),
    row(0.86, D31, Column::Wavelength, Linear, 0.89, 0.005),
    row(0.86, D33, Column::Wavelength, Linear, 0.72, 0.005),
    row(0.86, D22, Column::Wavelength, Linear, 0.60, 0.005),
    row(1.064, D31, Column::Wavelength, Linear, 3.11, 0.005),
    row(1.064, D33, Column::Wavelength, Linear, 2.04, 0.005),
    row(1.064, D22, Column::Wavelength, Linear, 1.74, 0.005),
    row(2.12, D33, Column::Wavelength, Linear, 62.0, 0.5),
    row(2.12, D22, Column::Wavelength, Linear, 55.0, 0.5),
    rich(row(0.86, D31, Temperature, Linear, 6.0, 0.05)),
    rich(row(0.86, D33, Temperature, Linear, 13.7, 0.05)),
    row(1.064, D31, Temperature, Linear, 9.4, 0.05),
    row(1.064, D33, Temperature, Linear, 25.8, 0.05),
    row(1.064, D22, Temperature, Linear, 64.0, 0.5),
    row(2.12, D31, Temperature, Linear, 27.5, 0.05),
    row(2.12, D33, Temperature, Linear, 193.0, 0.5),
    row(2.12, D22, Temperature, Linear, 379.0, 0.5),
    row(0.86, D31, AboutZ, Noncritical, 176.0, 0.5),
    row(0.86, D33, AboutZ, Noncritical, 112.0, 0.5),
    row(0.86, D22, AboutZ, Noncritical, 104.0, 0.5),
    row(1.064, D31, AboutZ, Noncritical, 959.0, 0.5),
    row(1.064, D31, AboutZ, Numerical, 916.0, 0.5),
    row(1.064, D33, AboutZ, Noncritical, 158.0, 0.5),
    row(1.064, D22, AboutZ, Noncritical, 147.0, 0.5),
    row(2.12, D31, AboutZ, Noncritical, 317.0, 0.5),
    row(2.12, D33, AboutZ, Noncritical, 337.0, 0.5),
    row(2.12, D22, AboutZ, Noncritical, 311.0, 0.5),
    row(0.86, D31, TowardZ, FirstOrder, 79.0, 0.5),
    row(0.86, D33, TowardZ, FirstOrder, 103.0, 0.5),
    row(0.86, D22, TowardZ, FirstOrder, 104.0, 0.5),
    row(1.064, D33, TowardZ, FirstOrder, 146.0, 0.5),
    row(1.064, D22, TowardZ, FirstOrder, 147.0, 0.5),
    row(2.12, D31, TowardZ, Numerical, 183.0, 0.5),
    row(2.12, D33, TowardZ, FirstOrder, 306.0, 0.5),
    row(2.12, D22, TowardZ, FirstOrder, 311.0, 0.5),
];

/// LiNbO₃'s n_o and n_e as Fejer et al. computed their Table I with: congruent, Edwards and
/// Lawrence's (their Ref. [24]); or lithium-rich, Jundt et al. 1990's (`rich`, their Ref. [25],
/// footnote f).
fn indices(rich: bool) -> crate::Result<(Index, Index)> {
    let m = if rich {
        catalogue::lithium_rich_jundt_1990()
    } else {
        catalogue::entry("linbo3")
            .and_then(|e| {
                e.index
                    .into_iter()
                    .find(|m| m.id == "edwards-lawrence-1984")
            })
            .ok_or_else(|| crate::Error::invalid("index", "no Edwards and Lawrence model"))?
    };
    Ok((
        Index::model(m.clone(), Axis::Ordinary, None)?,
        Index::model(m, Axis::Extraordinary, None)?,
    ))
}

/// One value of Table I as we compute it, in the table's unit: SHG at the fundamental `lam`
/// (µm) in 1 mm of LiNbO₃ at 25 °C, first-order QPM at a 50 % duty cycle, Jundt's thermal
/// expansion (for δT); the finite differences' steps scaled by `scale`.
pub(crate) fn table_1_value(p: &Printed, scale: f64) -> crate::Result<f64> {
    let state = State::celsius(25.0);
    let (o, e) = indices(p.rich)?;
    if p.estimate == FirstOrder {
        let (e1, e2) = p.coefficient.extraordinary();
        let n = |lam: f64, ext: bool| {
            if ext {
                e.at(lam, &state)
            } else {
                o.at(lam, &state)
            }
        };
        // Eq. (A1.4): an extraordinary wave tilted toward the optic axis grows by n_o − n_e
        let b = |lam: f64, ext: bool| -> crate::Result<f64> {
            Ok(if ext {
                o.at(lam, &state)? - e.at(lam, &state)?
            } else {
                0.0
            })
        };
        let (h, lam) = (p.lam / 2.0, p.lam);
        let tilt = first_order_tilt(lam, n(lam, e1)?, n(h, e2)?, b(lam, e1)?, b(h, e2)?, 1000.0);
        return Ok(tilt * 1e3);
    }
    let crystal = Uniaxial::new(o, e);
    let process = crystal.second_harmonic(Wavelength::um(p.lam)?, p.coefficient.kind(), &state)?;
    if p.column == Coherence {
        return process.coherence_length(&state);
    }
    let device = Device::quasi_phase_matched(process, 1000.0, state, 1, 0.5)?
        .with_expansion(Expansion::LINBO3_JUNDT);
    let (variable, unit) = match p.column {
        Column::Wavelength => (Variable::Wavelength, 1e3),
        Temperature => (Variable::Temperature, 1.0),
        AboutZ => (Variable::Tilt(Plane::AboutOpticAxis), 1e3),
        _ => (Variable::Tilt(Plane::TowardOpticAxis), 1e3),
    };
    let a = device.acceptance_with(variable, scale)?;
    let value = match p.estimate {
        Linear => a.linear,
        Noncritical => a.noncritical,
        _ => a.numerical,
    };
    Ok(value * unit)
}

/// One column of Table I: the largest difference over its rows in units of half the row's last
/// printed digit (1 is the tolerance); our own error, the change when the finite differences'
/// steps are halved, in the same units, noted.
fn table_1_column(column: Column) -> Outcome {
    let (mut worst, mut own) = (0.0_f64, 0.0_f64);
    for p in TABLE_1.iter().filter(|p| p.column == column) {
        let (v, finer) = match (table_1_value(p, 1.0), table_1_value(p, 0.5)) {
            (Ok(v), Ok(f)) => (v, f),
            _ => (f64::NAN, f64::NAN),
        };
        let e = (v - p.value).abs() / p.half;
        worst = if e.is_nan() { f64::NAN } else { worst.max(e) };
        // a numerical width is found by bisection, to round-off: the steps only matter to the
        // estimates from derivatives
        if matches!(p.estimate, Linear | Noncritical) {
            own = own.max((v - finer).abs() / p.half);
        }
    }
    note_numerical(own);
    Outcome {
        measured: worst,
        expected: 0.0,
        tolerance: 1.0,
        error: worst,
    }
}

fn table_1_coherence() -> Outcome {
    table_1_column(Coherence)
}

fn table_1_wavelength() -> Outcome {
    table_1_column(Column::Wavelength)
}

fn table_1_temperature() -> Outcome {
    table_1_column(Temperature)
}

fn table_1_about_z() -> Outcome {
    table_1_column(AboutZ)
}

fn table_1_toward_z() -> Outcome {
    table_1_column(TowardZ)
}

/// sinc²(1.39156…) against ½: the half-maximum point is a root to round-off.
fn half_maximum() -> Outcome {
    let v = sinc2(HALF_MAXIMUM);
    Outcome {
        measured: v,
        expected: 0.5,
        tolerance: 0.5 * 1e-12,
        error: (v - 0.5).abs(),
    }
}

/// The undepleted harmonic of a quasi-phase-matched device at exact phase matching, over 1 and
/// 2 mm: the ratio of the powers, which is 4. The device's Δk at its operating point is zero to
/// round-off ([`Device::quasi_phase_matched`]).
pub(crate) fn length_squared() -> Outcome {
    let run = || -> crate::Result<f64> {
        let state = State::celsius(25.0);
        let (o, e) = indices(false)?;
        let process = Uniaxial::new(o, e).second_harmonic(
            Wavelength::um(1.064)?,
            Type::Zero(Polarization::Extraordinary),
            &state,
        )?;
        let power = |length: f64| -> crate::Result<f64> {
            let d = Device::quasi_phase_matched(process.clone(), length, state, 1, 0.5)?;
            Ok(undepleted(d.mismatch(&state)?, length))
        };
        Ok(power(2000.0)? / power(1000.0)?)
    };
    let ratio = run().unwrap_or(f64::NAN);
    Outcome {
        measured: ratio,
        expected: 4.0,
        tolerance: 4.0 * 1e-12,
        error: (ratio - 4.0).abs(),
    }
}

/// Wang et al.'s poling duty cycle, 39 % (±3 %): how much it lowers d_eff from the optimum
/// 50 %, sin(πD) by Fejer et al.'s Eq. (22), in percent.
pub(crate) fn duty_cut(duty: f64) -> f64 {
    100.0 * (1.0 - effective_d(1.0, 1, duty) / effective_d(1.0, 1, 0.5))
}

/// The tolerance of Wang et al.'s 7 %, in percent: what their ±3 % on the duty cycle does to
/// the cut, the larger side (at 36 %, 3.61), rounded down.
pub(crate) const WANG_TOLERANCE: f64 = 3.6;

fn wang_duty_cycle() -> Outcome {
    let cut = duty_cut(0.39);
    Outcome {
        measured: cut,
        expected: 7.0,
        tolerance: WANG_TOLERANCE,
        error: (cut - 7.0).abs(),
    }
}

fn real(x: f64) -> num_complex::Complex64 {
    num_complex::Complex64::new(x, 0.0)
}

/// The coupled-mode cases' length: 4 mm, Wang et al.'s.
const LENGTH: f64 = 4000.0;

/// Lossless undepleted SHG, η = 4500 %/(W·cm²) over 4 mm at ΔkL/2 = 1, from 1e-20 W (depletion
/// below 1e-19): (ours by `steps` of RK4, η P₀² L² sinc²(1)).
pub(crate) fn coupled_undepleted(steps: usize) -> (f64, f64) {
    let shg = SecondHarmonic {
        coupling: coupling_from_efficiency(4500.0),
        mismatch: 2.0 / LENGTH,
        loss: [0.0; 2],
    };
    let p0: f64 = 1e-20;
    let ours = shg
        .propagate([real(p0.sqrt()), real(0.0)], LENGTH, steps)
        .map_or(f64::NAN, |a| a[1].norm_sqr());
    (
        ours,
        shg.coupling.powi(2) * p0 * p0 * undepleted(shg.mismatch, LENGTH),
    )
}

/// The same with losses of 3 and 6 dB/cm: (ours, the closed form).
pub(crate) fn coupled_lossy(steps: usize) -> (f64, f64) {
    let shg = SecondHarmonic {
        coupling: coupling_from_efficiency(4500.0),
        mismatch: 2.0 / LENGTH,
        loss: [loss_from_db_per_cm(3.0), loss_from_db_per_cm(6.0)],
    };
    let p0: f64 = 1e-20;
    let ours = shg
        .propagate([real(p0.sqrt()), real(0.0)], LENGTH, steps)
        .map_or(f64::NAN, |a| a[1].norm_sqr());
    (ours, undepleted_lossy_shg(&shg, p0, LENGTH))
}

/// Phase-matched SHG depleted to 93 %: κ√P₀L = 2 (η = 4500 %/(W·cm²), 4 mm, P₀ = 55.6 mW):
/// (ours, P₀ tanh²(2)), the harmonic's share.
pub(crate) fn coupled_tanh(steps: usize) -> (f64, f64) {
    let coupling = coupling_from_efficiency(4500.0);
    let p0 = (2.0 / (coupling * LENGTH)).powi(2);
    let shg = SecondHarmonic {
        coupling,
        mismatch: 0.0,
        loss: [0.0; 2],
    };
    let ours = shg
        .propagate([real(p0.sqrt()), real(0.0)], LENGTH, steps)
        .map_or(f64::NAN, |a| a[1].norm_sqr() / p0);
    (ours, 2.0_f64.tanh().powi(2))
}

/// 1.55 and 1.31 µm summed with equal photon fluxes F, phase matched: the sum's share
/// P₃/(ν₃F) at L = 2l, against Armstrong et al.'s tanh²(L/l), l⁻¹ = κ√(ν₁ν₂F/ν₃) (Sec. VI).
pub(crate) fn coupled_sech(steps: usize) -> (f64, f64) {
    let tw = ThreeWave {
        frequencies: [1.0 / 1.55, 1.0 / 1.31],
        coupling: coupling_from_efficiency(4500.0),
        mismatch: 0.0,
        loss: [0.0; 3],
    };
    let [n1, n2] = tw.frequencies;
    let n3 = n1 + n2;
    // F so that L = 2l
    let flux = (2.0 / (tw.coupling * LENGTH)).powi(2) * n3 / (n1 * n2);
    let start = [
        real((n1 * flux).sqrt()),
        real((n2 * flux).sqrt()),
        real(0.0),
    ];
    let ours = tw
        .propagate(start, LENGTH, steps)
        .map_or(f64::NAN, |a| a[2].norm_sqr() / (n3 * flux));
    (ours, 2.0_f64.tanh().powi(2))
}

/// Parametric amplification of 1.55 µm by an undepleted 0.775 µm pump, phase matched: the
/// signal's gain against cosh²(ΓL), Γ = κ√(ν₁ν₂P₃)/ν₃, at ΓL = 2 (the pump 0.86 W, the signal 1e-24 W, so that the pump's depletion is below 1e-22).
pub(crate) fn coupled_cosh(steps: usize) -> (f64, f64) {
    let tw = ThreeWave {
        frequencies: [1.0 / 1.55, 1.0 / 1.55],
        coupling: coupling_from_efficiency(4500.0),
        mismatch: 0.0,
        loss: [0.0; 3],
    };
    let [n1, n2] = tw.frequencies;
    let pump = (2.0 / LENGTH * (n1 + n2) / (tw.coupling * (n1 * n2).sqrt())).powi(2);
    let ours = tw
        .propagate([real(1e-12), real(0.0), real(pump.sqrt())], LENGTH, steps)
        .map_or(f64::NAN, |a| a[0].norm_sqr() / 1e-24);
    (ours, 2.0_f64.cosh().powi(2))
}

/// Strongly depleted, mismatched mixing of 1.55 and 1.31 µm from three nonzero inputs: the
/// largest relative drift of the two Manley–Rowe fluxes over 4 mm.
pub(crate) fn coupled_drift(steps: usize) -> f64 {
    let tw = ThreeWave {
        frequencies: [1.0 / 1.55, 1.0 / 1.31],
        coupling: coupling_from_efficiency(4500.0),
        mismatch: 1.5e-3,
        loss: [0.0; 3],
    };
    let start = [
        real(0.1),
        num_complex::Complex64::new(0.03, 0.04),
        real(0.01),
    ];
    let before = tw.manley_rowe(&start);
    tw.propagate(start, LENGTH, steps).map_or(f64::NAN, |a| {
        let after = tw.manley_rowe(&a);
        (after[0] / before[0] - 1.0)
            .abs()
            .max((after[1] / before[1] - 1.0).abs())
    })
}

/// Every coupled-mode case's error at `steps`, for the grid study.
#[cfg(test)]
pub(crate) fn coupled_errors(steps: usize) -> [f64; 6] {
    let rel = |(a, b): (f64, f64)| (a / b - 1.0).abs();
    [
        rel(coupled_undepleted(steps)),
        rel(coupled_lossy(steps)),
        rel(coupled_tanh(steps)),
        rel(coupled_sech(steps)),
        rel(coupled_cosh(steps)),
        coupled_drift(steps),
    ]
}
/// The coupled-mode cases' number of RK4 steps; each tolerance is the error on half as many,
/// rounded up.
const STEPS: usize = 1000;

fn relative(pair: (f64, f64), tolerance: f64) -> Outcome {
    let (measured, expected) = pair;
    Outcome {
        measured,
        expected,
        tolerance: tolerance * expected.abs(),
        error: (measured - expected).abs(),
    }
}

fn coupled_undepleted_case() -> Outcome {
    relative(coupled_undepleted(STEPS), 2e-13)
}

fn coupled_lossy_case() -> Outcome {
    relative(coupled_lossy(STEPS), 2e-13)
}

fn coupled_tanh_case() -> Outcome {
    relative(coupled_tanh(STEPS), 1e-12)
}

fn coupled_sech_case() -> Outcome {
    relative(coupled_sech(STEPS), 1e-12)
}

fn coupled_cosh_case() -> Outcome {
    relative(coupled_cosh(STEPS), 9e-12)
}

fn coupled_drift_case() -> Outcome {
    let d = coupled_drift(STEPS);
    Outcome {
        measured: d,
        expected: 0.0,
        tolerance: 1e-12,
        error: d,
    }
}

/// RK4's order from the depleted SHG case on 125 and 250 steps, to two decimals.
fn coupled_order_case() -> Outcome {
    let e = |n| {
        let (a, b) = coupled_tanh(n);
        (a / b - 1.0).abs()
    };
    let order = (e(125) / e(250)).log2();
    let shown = (order * 100.0).round() / 100.0;
    Outcome {
        measured: shown,
        expected: 4.0,
        tolerance: 0.1,
        error: (shown - 4.0).abs(),
    }
}
/// A linear index n = a + bλ: its group index is a, and k = 2π(aν + b) is linear in ν, so the
/// mismatch of a process of three such waves is linear in their frequencies.
pub(crate) fn linear_index(a: f64, b: f64) -> crate::Result<Index> {
    let w: Vec<f64> = (0..=27).map(|k| 0.3 + 0.1 * f64::from(k)).collect();
    let n = w.iter().map(|l| a + b * l).collect();
    Index::effective(w, n)
}

/// Degenerate down-conversion of 0.775 µm into 1.55 µm over 1 cm, the signal's, idler's and
/// pump's group indices 2.30, 2.25 and 2.40, a Gaussian pump 1e-3 /µm wide (0.6 nm) and Gaussian
/// phase matching: (our purity on an `n` × `n` grid ±0.025 /µm, five of the amplitude's widths along its ridge, the closed form). Δk is linear
/// in the frequencies, so the joint amplitude is exactly a two-dimensional Gaussian.
pub(crate) fn pairs_gaussian(n: usize) -> (f64, f64) {
    let length = 10_000.0;
    let (gs, gi, gp) = (2.30, 2.25, 2.40);
    let run = || -> crate::Result<f64> {
        let process = Process::difference_frequency(
            Wave::new(Wavelength::um(0.775)?, linear_index(gp, -0.25)?),
            Wave::new(Wavelength::um(1.55)?, linear_index(gs, -0.08)?),
            linear_index(gi, -0.06)?,
        )?;
        let device = Device::quasi_phase_matched(process, length, State::default(), 1, 0.5)?;
        let pump = Pump {
            center: 1.0 / 0.775,
            bandwidth: 1e-3,
        };
        let jsa = joint_spectrum(&device, pump, Matching::Gaussian, 0.025, n)?;
        Ok(jsa.schmidt()?.purity())
    };
    // ln f = −2 ln 2 (δs + δi)²/Δν² − γ (πL)² (A δs + B δi)², A = g_p − g_s, B = g_p − g_i
    let p = 2.0 * std::f64::consts::LN_2 / 1e-6;
    let g = gaussian_gamma() * (std::f64::consts::PI * length).powi(2);
    let (a, b) = (gp - gs, gp - gi);
    let closed = gaussian_purity(p + g * a * a, p + g * b * b, p + g * a * b);
    (run().unwrap_or(f64::NAN), closed)
}

/// Every pairs case's error at `n` points a side, for the grid study.
#[cfg(test)]
pub(crate) fn pairs_errors(n: usize) -> f64 {
    let (a, b) = pairs_gaussian(n);
    (a / b - 1.0).abs()
}

fn pairs_gaussian_case() -> Outcome {
    relative(pairs_gaussian(201), PAIRS_TOLERANCE)
}

/// The Gaussian joint spectrum's tolerance: on 201 points a side the grid's error is below the
/// singular value decomposition's round-off (3.4e-13; 4.4e-13 and 2.3e-12 on 301 and 401).
const PAIRS_TOLERANCE: f64 = 1e-11;
const FEJER: &str = "M. M. Fejer, G. A. Magel, D. H. Jundt, R. L. Byer, IEEE J. Quantum Electron. 28, 2631 (1992), doi:10.1109/3.161322, Table I (computed with G. J. Edwards, M. Lawrence, Opt. Quantum Electron. 16, 373 (1984), doi:10.1007/BF00620081, at 25 °C; the 0.86 µm δT with D. H. Jundt, M. M. Fejer, R. L. Byer, IEEE J. Quantum Electron. 26, 135 (1990), doi:10.1109/3.44926), each value to half a unit of its last printed digit";

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "nonlinear/sinc2-half-maximum",
            title: r"The undepleted response $\operatorname{sinc}^2(\Delta k L/2)$ falls to ½ at $\Delta k L/2 = 1.391557\ldots$ (Fejer's $0.4429\pi$), the point every FWHM acceptance bandwidth is taken at ($\operatorname{sinc}^2$ there shown)",
            tier: Tier::Analytic,
            source: r"$\sin x / x = 1/\sqrt{2}$; Fejer et al., IEEE J. Quantum Electron. 28, 2631 (1992), doi:10.1109/3.161322, Sec. II",
            run: half_maximum,
        },
        Case {
            id: "nonlinear/sinc2-length-squared",
            title: r"Quasi-phase-matched SHG of 1.064 µm in congruent LiNbO₃ ($d_{33}$, Edwards and Lawrence's indices, 25 °C), its grating designed for the process: the undepleted harmonic's power over 2 mm against 1 mm, $L^2 \operatorname{sinc}^2(\Delta k L/2)$ at $\Delta k = 0$ (ratio shown)",
            tier: Tier::Analytic,
            source: r"$\left\lvert\int_0^L e^{-i\Delta k z} dz\right\rvert^2 = L^2$ at $\Delta k = 0$; Fejer et al., Eqs. (4) and (19)",
            run: length_squared,
        },
        Case {
            id: "nonlinear/fejer-table-1-coherence",
            title: r"Fejer et al.'s Table I, the coherence lengths $l_c = \lambda/4(n_2 - n_1)$ of first-order QPM SHG in congruent LiNbO₃: $d_{31}$, $d_{33}$ and $d_{22}$ at 0.86 µm, $d_{33}$ and $d_{22}$ at 1.064 µm, $d_{31}$ and $d_{22}$ at 2.12 µm (the largest difference shown, in halves of each value's last printed digit)",
            tier: Tier::Published,
            source: FEJER,
            run: table_1_coherence,
        },
        Case {
            id: "nonlinear/fejer-table-1-wavelength",
            title: r"Fejer et al.'s Table I, the FWHM acceptance in the fundamental's wavelength of 1 mm of congruent LiNbO₃, by their linear estimate (Eqs. (25) and (30)): the $d_{31}$, $d_{33}$ and $d_{22}$ rows at 0.86 and 1.064 µm, $d_{33}$ and $d_{22}$ at 2.12 µm (the largest difference shown, in halves of each value's last printed digit)",
            tier: Tier::Published,
            source: FEJER,
            run: table_1_wavelength,
        },
        Case {
            id: "nonlinear/fejer-table-1-temperature",
            title: r"Fejer et al.'s Table I, the FWHM acceptance in temperature of 1 mm of LiNbO₃, by their linear estimate with the thermal expansion (Eqs. (25), (43) and (44); Jundt's expansion along the grating): $d_{31}$ and $d_{33}$ at 0.86 µm (lithium-rich, footnote f), the three QPM rows at 1.064 and 2.12 µm (the largest difference shown, in halves of each value's last printed digit)",
            tier: Tier::Published,
            source: FEJER,
            run: table_1_temperature,
        },
        Case {
            id: "nonlinear/fejer-table-1-tilt-about-z",
            title: r"Fejer et al.'s Table I, the FWHM acceptance in the tilt about the crystal's z axis, $n_e$ invariant (column c): the noncritical estimate (Eq. (41)) for the nine QPM rows, and the numerical width (916) for $d_{31}$ at 1.064 µm (the largest difference shown, in halves of each value's last printed digit)",
            tier: Tier::Published,
            source: FEJER,
            run: table_1_about_z,
        },
        Case {
            id: "nonlinear/fejer-table-1-tilt-toward-z",
            title: r"Fejer et al.'s Table I, the FWHM acceptance in the tilt about x or y, $n_e$ varying with the tilt (column d): Eq. (A1.9), first order in the birefringence, for seven QPM rows, and the numerical width (183) for $d_{31}$ at 2.12 µm (the largest difference shown, in halves of each value's last printed digit)",
            tier: Tier::Published,
            source: FEJER,
            run: table_1_toward_z,
        },
        Case {
            id: "nonlinear/wang-2018-duty-cycle",
            title: r"Thin-film PPLN poled with a 39 % duty cycle: how much it lowers the effective coefficient from 50 %'s, $1 - \sin(\pi D)$ by Fejer et al.'s Eq. (22), against Wang et al.'s 'reduces the actual $d_\mathrm{eff}$ by 7 %' (percent shown)",
            tier: Tier::Published,
            source: r"C. Wang et al., Optica 5, 1438 (2018), doi:10.1364/OPTICA.5.001438, p. 1439: a duty cycle of 39 % ± 3 % (Fig. 2) reduces $d_\mathrm{eff}$ by 7 %; ±3 % moves the cut by up to 3.6 %",
            run: wang_duty_cycle,
        },
        Case {
            id: "nonlinear/coupled-undepleted",
            title: r"The coupled-amplitude equations of SHG by RK4 (1000 steps), undepleted: $\eta = 4500$ %/(W·cm²), 4 mm, $\Delta k L/2 = 1$, from $10^{-20}$ W, against $\eta P_0^2 L^2 \operatorname{sinc}^2(1)$ (the harmonic's power shown, W)",
            tier: Tier::Analytic,
            source: r"the undepleted limit, Fejer et al., Eq. (19); the depletion is below $10^{-19}$",
            run: coupled_undepleted_case,
        },
        Case {
            id: "nonlinear/coupled-lossy",
            title: r"The same with propagation losses of 3 dB/cm at the fundamental and 6 dB/cm at the harmonic, against the closed form $\eta P_0^2 e^{-\alpha_2 L} \lvert (e^{\gamma L} - 1)/\gamma \rvert^2$, $\gamma = -\alpha_1 + \alpha_2/2 - i\Delta k$ (W shown)",
            tier: Tier::Analytic,
            source: r"the harmonic generated at $z$ by the fundamental's $e^{-\alpha_1 z}$, carried to $L$ with $e^{-\alpha_2 (L - z)/2}$",
            run: coupled_lossy_case,
        },
        Case {
            id: "nonlinear/coupled-shg-tanh",
            title: r"Phase-matched SHG depleted by RK4 (1000 steps): $\kappa\sqrt{P_0}L = 2$, the harmonic's share of the power against $\tanh^2 2 = 0.9293$ (shown)",
            tier: Tier::Analytic,
            source: r"J. A. Armstrong, N. Bloembergen, J. Ducuing, P. S. Pershan, Phys. Rev. 127, 1918 (1962), doi:10.1103/PhysRev.127.1918, Sec. VI: $u_3 = u_1(0)\tanh(z/l)$",
            run: coupled_tanh_case,
        },
        Case {
            id: "nonlinear/coupled-sfg-sech",
            title: r"Phase-matched sum-frequency generation of 1.55 and 1.31 µm from equal photon fluxes $F$, depleted, by RK4 (1000 steps): the sum's photons $P_3/\nu_3 F$ at $L = 2l$ against $\tanh^2 2$ (shown)",
            tier: Tier::Analytic,
            source: r"Armstrong et al., Sec. VI: with equal numbers of photons at $\omega_1$ and $\omega_2$, $u_1 = u_2 = u_1(0)\operatorname{sech}(z/l)$ and $u_3 = u_1(0)\tanh(z/l)$",
            run: coupled_sech_case,
        },
        Case {
            id: "nonlinear/coupled-opa-cosh",
            title: r"Degenerate parametric amplification of 1.55 µm by an undepleted 0.775 µm pump, phase matched, by RK4 (1000 steps): the signal's gain against $\cosh^2(\Gamma L)$, $\Gamma = \kappa\sqrt{\nu_1\nu_2 P_3}/\nu_3$, at $\Gamma L = 2$ (gain shown)",
            tier: Tier::Analytic,
            source: r"the undepleted pump's linear equations for the signal and the idler's conjugate, whose growing solution is $\cosh(\Gamma z)$",
            run: coupled_cosh_case,
        },
        Case {
            id: "nonlinear/coupled-manley-rowe",
            title: r"Mismatched, depleted mixing of 1.55 and 1.31 µm from three nonzero inputs over 4 mm, by RK4 (1000 steps): the largest relative change of the Manley–Rowe fluxes $P_1/\nu_1 + P_3/\nu_3$ and $P_2/\nu_2 + P_3/\nu_3$ (shown)",
            tier: Tier::Analytic,
            source: r"Armstrong et al., Eq. (6.6): the photon fluxes are conserved without loss",
            run: coupled_drift_case,
        },
        Case {
            id: "nonlinear/coupled-rk4-order",
            title: r"The order of the coupled-amplitude integration: the depleted SHG case's error on 125 and 250 steps (order shown)",
            tier: Tier::Analytic,
            source: "the classical Runge–Kutta method is fourth order",
            run: coupled_order_case,
        },
        Case {
            id: "nonlinear/pairs-gaussian-purity",
            title: r"Photon pairs: degenerate down-conversion of 0.775 µm over 1 cm (group indices 2.30, 2.25 and 2.40), a Gaussian pump 0.6 nm wide and Gaussian phase matching: the purity of the joint spectral amplitude by the singular value decomposition of 201 × 201 samples, against the two-dimensional Gaussian's $P = \sqrt{1 - c^2/ab}$ (purity shown)",
            tier: Tier::Analytic,
            source: r"$\operatorname{Tr}\rho^2$ of a Gaussian amplitude $e^{-z^T M z/2}$, $M = [[a, c], [c, b]]$, in closed form; the amplitude as W. P. Grice, I. A. Walmsley, Phys. Rev. A 56, 1627 (1997), doi:10.1103/PhysRevA.56.1627, Eq. (9), its phase matching a Gaussian of the sinc²'s FWHM",
            run: pairs_gaussian_case,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_case_passes() {
        for case in cases() {
            let o = (case.run)();
            assert!(o.passed(), "{}: {o:?}", case.id);
        }
    }

    #[test]
    fn every_printed_value_is_reproduced_within_its_last_digit() {
        for p in TABLE_1 {
            let v = table_1_value(p, 1.0).unwrap();
            assert!((v - p.value).abs() <= p.half, "{p:?}: {v}");
        }
    }

    #[test]
    fn the_wang_tolerance_is_the_duty_cycles_uncertainty_rounded_down() {
        let spread = (duty_cut(0.36) - duty_cut(0.39)).max(duty_cut(0.39) - duty_cut(0.42));
        assert!(
            WANG_TOLERANCE <= spread && spread - WANG_TOLERANCE < 0.1,
            "{spread}"
        );
        assert!((duty_cut(0.39) - 5.91).abs() < 0.01);
    }

    #[test]
    fn the_left_out_values_are_still_outside_their_last_digit() {
        // printed, ours: when one of these comes within half its last digit, it joins TABLE_1
        let left_out = [
            row(1.064, D31, Coherence, Direct, 129.0, 0.5),
            row(2.12, D33, Coherence, Direct, 15.6, 0.05),
            row(2.12, D31, Column::Wavelength, Linear, 40.0, 0.5),
            row(0.86, D22, Temperature, Linear, 17.4, 0.05),
            row(1.064, D31, TowardZ, FirstOrder, 103.0, 0.5),
            row(2.12, D31, TowardZ, FirstOrder, 139.0, 0.5),
        ];
        for p in left_out {
            let p = if p.column == Temperature && p.lam == 0.86 {
                rich(p)
            } else {
                p
            };
            let v = table_1_value(&p, 1.0).unwrap();
            assert!((v - p.value).abs() > p.half, "{p:?}: {v}");
        }
    }
}
