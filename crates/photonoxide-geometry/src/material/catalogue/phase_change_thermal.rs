//! The validation cases of the phase-change materials' thermal constants: GST's melting by Kalb,
//! Spaepen & Wuttig 2003 (J. Appl. Phys. 93, 2389, doi:10.1063/1.1540227) against Zhao et al. 2022
//! (Adv. Mater. Interfaces 9, 2200429, doi:10.1002/admi.202200429), its densities by Njoroge,
//! Wöltgens & Wuttig 2002 (J. Vac. Sci. Technol. A 20, 230, doi:10.1116/1.1430249), and
//! crystalline Sb₂S₃'s heat capacity by Gurevich et al. 2002 (Geokhimiya 2002 (2), 194), whose
//! Eq. (1) (Debye and Einstein functions, their Table 5) and Eq. (7) are evaluated here against
//! their Tables 2 and 3.

use super::phase_change::{
    self, GUREVICH_CP_298, KAITO_TABLE_1, NJOROGE_DENSITY, njoroge_xrd_density, sb2s3_density,
};
use crate::verdict::Outcome;

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// The molar gas constant, J/(mol·K).
const R: f64 = 8.314_462_618;

/// Kalb et al.'s heat of fusion of GST (J/mol) and entropy of fusion (J/(mol·K)), Table II.
pub(crate) const KALB_FUSION: (f64, f64) = (14_700.0, 16.5);

/// Zhao et al.'s melting peak of GST, K (627 ± 15 °C), and molar melting enthalpy, J/mol of
/// atoms (16.7 ± 3.4 kJ).
pub(crate) const ZHAO_MELTING: (f64, f64) = (627.0 + 273.15, 15.0);
#[cfg(test)]
pub(crate) const ZHAO_FUSION: (f64, f64) = (16_700.0, 3_400.0);

#[cfg(test)]
fn gst() -> Option<phase_change::PhaseChange> {
    phase_change::phase_change("gst")
}

fn melting(id: &str) -> f64 {
    phase_change::phase_change(id)
        .and_then(|p| p.melting)
        .map_or(f64::NAN, |c| c.value)
}

/// Kalb et al.'s entropy of fusion from their heat of fusion over the catalogue's T_m (their
/// Eq. (1)), J/(mol·K): a check that 621 °C is the T_m they divide by.
pub(crate) fn gst_kalb_fusion() -> Outcome {
    outcome(KALB_FUSION.0 / melting("gst"), KALB_FUSION.1, 0.1)
}

/// GST's T_m (Kalb et al.'s DSC of a thick film) against Zhao et al.'s nanocalorimetry of 10 to
/// 40 nm films, 900 ± 15 K (K shown).
pub(crate) fn gst_zhao_melting() -> Outcome {
    outcome(melting("gst"), ZHAO_MELTING.0, ZHAO_MELTING.1)
}

/// Njoroge et al.'s density changes on crystallization from their three densities, against
/// their 6.8 ± 0.2 % (NaCl-type) and 8.8 ± 0.2 % (hexagonal): the larger deviation, in percent.
pub(crate) fn gst_njoroge_change() -> Outcome {
    let [a, c, h] = NJOROGE_DENSITY;
    let d = (100.0 * (c / a - 1.0) - 6.8)
        .abs()
        .max((100.0 * (h / a - 1.0) - 8.8).abs());
    outcome(d, 0.0, 0.2)
}

/// Njoroge et al.'s XRD densities of the NaCl-type phase from the film's stoichiometry and the
/// lattice parameters they measure (6.016 Å after 170 °C, 6.000 Å after 260 °C), against their
/// 6.43 and 6.48 g/cm³: the larger deviation, g/cm³.
pub(crate) fn gst_njoroge_xrd() -> Outcome {
    let d = (njoroge_xrd_density(6.016) * 1e-3 - 6.43)
        .abs()
        .max((njoroge_xrd_density(6.000) * 1e-3 - 6.48).abs());
    outcome(d, 0.0, 0.005)
}

/// The catalogue's GST heat capacities per volume, both states, J/(m³·K): Zhao et al.'s c_p and
/// Njoroge et al.'s ρ.
#[cfg(test)]
pub(crate) fn gst_heat_capacities() -> (f64, f64) {
    let Some(p) = gst() else {
        return (f64::NAN, f64::NAN);
    };
    let of = |s| p.heat_capacity_of(s).unwrap_or(f64::NAN);
    (of(super::State::Amorphous), of(super::State::Crystalline))
}

/// Gurevich et al.'s Table 5 for Sb₂S₃, 20.92–337.61 K: γ (J/(mol·K²)), θ₁, θ₂, θ₃, θ_E (K).
pub(crate) const GUREVICH_FIT: (f64, [f64; 4]) = (0.002359, [130.0, 273.0, 423.0, 448.0]);

/// ∫₀^u x⁴eˣ/(eˣ − 1)² dx by Simpson's rule, written with e⁻ˣ.
fn debye_integral(u: f64) -> f64 {
    let f = |x: f64| {
        if x < 1e-6 {
            x * x
        } else {
            let e = (-x).exp();
            x.powi(4) * e / ((1.0 - e) * (1.0 - e))
        }
    };
    let n = 4000;
    let h = u / n as f64;
    let s: f64 = (0..=n)
        .map(|i| {
            let w = if i == 0 || i == n {
                1.0
            } else if i % 2 == 1 {
                4.0
            } else {
                2.0
            };
            w * f(i as f64 * h)
        })
        .sum();
    s * h / 3.0
}

/// The paper's Eq. (2), D(θ) = 3R(T/θ)³ ∫₀^{θ/T} x⁴eˣ(eˣ − 1)⁻² dx (R at high T).
fn debye(theta: f64, t: f64) -> f64 {
    3.0 * R * (t / theta).powi(3) * debye_integral(theta / t)
}

/// The paper's Eq. (3), E(θ_E) = 3R x²eˣ(eˣ − 1)⁻², x = θ_E/T.
fn einstein(theta: f64, t: f64) -> f64 {
    let x = theta / t;
    let e = (-x).exp();
    3.0 * R * x * x * e / ((1.0 - e) * (1.0 - e))
}

/// Gurevich et al.'s Eq. (1): C_p = n[γT + 0.846D(θ₁) + 0.760D(θ₂) + 0.800D(θ₃) + 0.200E(θ_E)],
/// n = 5, J/(mol·K).
pub(crate) fn gurevich_cp(t: f64) -> f64 {
    let (gamma, [t1, t2, t3, te]) = GUREVICH_FIT;
    5.0 * (gamma * t
        + 0.846 * debye(t1, t)
        + 0.760 * debye(t2, t)
        + 0.800 * debye(t3, t)
        + 0.200 * einstein(te, t))
}

/// Their Eq. (7), 338 to 823 K: C_p = 166.82 − 0.0031T + 645000T⁻² − 911T^(−1/2), J/(mol·K).
pub(crate) fn gurevich_cp_high(t: f64) -> f64 {
    166.82 - 0.0031 * t + 645_000.0 / (t * t) - 911.0 / t.sqrt()
}

/// Table 2's smoothed C_p (K, J/(mol·K)) at some of its temperatures, and 298.15 K.
pub(crate) const GUREVICH_TABLE_2: [(f64, f64); 11] = [
    (10.0, 1.52),
    (20.0, 9.39),
    (30.0, 20.39),
    (50.0, 40.51),
    (100.0, 77.33),
    (150.0, 98.07),
    (200.0, 109.18),
    (250.0, 115.60),
    (298.15, GUREVICH_CP_298.0),
    (300.0, 119.63),
    (337.0, 121.82),
];

/// Table 3's C_p (K, J/(mol·K)) from Eq. (7) at some of its temperatures.
pub(crate) const GUREVICH_TABLE_3: [(f64, f64); 6] = [
    (338.0, 121.9),
    (400.0, 124.1),
    (500.0, 127.1),
    (600.0, 129.6),
    (700.0, 131.5),
    (823.0, 133.5),
];

/// Eq. (1) with Table 5's parameters against Table 2 from 10 to 337 K: the largest relative
/// deviation.
pub(crate) fn sb2s3_gurevich_low() -> Outcome {
    let d = GUREVICH_TABLE_2.iter().fold(0.0f64, |w, &(t, cp)| {
        w.max((gurevich_cp(t) / cp - 1.0).abs())
    });
    outcome(d, 0.0, 3e-3)
}

/// Eq. (7) against Table 3 from 338 to 823 K: the largest deviation, J/(mol·K).
pub(crate) fn sb2s3_gurevich_high() -> Outcome {
    let d = GUREVICH_TABLE_3.iter().fold(0.0f64, |w, &(t, cp)| {
        w.max((gurevich_cp_high(t) - cp).abs())
    });
    outcome(d, 0.0, 0.05)
}

/// Kaito et al.'s density deficit of amorphous Sb₂S₃ films from Table 1's two densities, the
/// catalogue's amorphous 3420 kg/m³ against their crystal's 4.64 g/cm³, against the 26 % the same
/// table prints (percent).
pub(crate) fn sb2s3_kaito_deficit() -> Outcome {
    let amorphous = phase_change::phase_change("sb2s3")
        .and_then(|p| p.amorphous_density)
        .map_or(f64::NAN, |c| c.value);
    let (crystal, deficit) = KAITO_TABLE_1;
    outcome(100.0 * (1.0 - amorphous / (crystal * 1e3)), deficit, 0.5)
}

/// Sb₂S₃'s crystal density from Gurevich et al.'s cell against the 4640 kg/m³ Aryana et al.
/// 2023 take for the crystal (from Popescu's book): the relative deviation.
pub(crate) fn sb2s3_density_check() -> Outcome {
    outcome((sb2s3_density() / 4640.0 - 1.0).abs(), 0.0, 0.01)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::catalogue::State;
    use crate::material::catalogue::phase_change::{ZHAO_CP, gst_atomic_mass};

    #[test]
    fn every_case_passes() {
        for (name, o) in [
            ("kalb", gst_kalb_fusion()),
            ("zhao tm", gst_zhao_melting()),
            ("njoroge change", gst_njoroge_change()),
            ("njoroge xrd", gst_njoroge_xrd()),
            ("gurevich low", sb2s3_gurevich_low()),
            ("gurevich high", sb2s3_gurevich_high()),
            ("sb2s3 density", sb2s3_density_check()),
            ("sb2s3 kaito", sb2s3_kaito_deficit()),
        ] {
            assert!(o.error <= o.tolerance, "{name}: {o:?}");
        }
    }

    #[test]
    fn the_numbers() {
        // ΔS_f = 16.44 J/(mol·K); the XRD densities 6426 and 6478 kg/m³
        assert!((KALB_FUSION.0 / melting("gst") - 16.44).abs() < 5e-3);
        assert!((njoroge_xrd_density(6.016) - 6426.1).abs() < 0.5);
        assert!((njoroge_xrd_density(6.000) - 6477.6).abs() < 0.5);
        // Kalb's heat of fusion and Zhao's agree within their uncertainties
        assert!((KALB_FUSION.0 - ZHAO_FUSION.0).abs() < 700.0 + ZHAO_FUSION.1);
        // Eq. (1) and Eq. (7) meet at 338 K, where Gurevich et al. joined them
        assert!((gurevich_cp(338.0) - gurevich_cp_high(338.0)).abs() < 0.01);
        // the Debye function tends to R, so Eq. (1) to n(γT + 2.406R + 0.6R) at high T
        assert!((debye(10.0, 1e4) / R - 1.0).abs() < 1e-4);
        assert!((sb2s3_density() - 4662.9).abs() < 0.5);
        let p = gst().unwrap();
        let cp = |s| p.specific_heat_of(s).unwrap().value;
        assert!((cp(State::Amorphous) - 238.42).abs() < 0.01);
        assert!((cp(State::Crystalline) - 197.88).abs() < 0.01);
        assert!((ZHAO_CP.0 / gst_atomic_mass() - 238.42).abs() < 0.01);
        let (a, c) = gst_heat_capacities();
        assert!(
            (a - 1.3995e6).abs() < 1e3 && (c - 1.2407e6).abs() < 1e3,
            "{a} {c}"
        );
        let s = phase_change::phase_change("sb2s3").unwrap();
        assert!((s.specific_heat.as_ref().unwrap().value - 351.92).abs() < 0.01);
        assert!((s.heat_capacity().unwrap() - 1.641e6).abs() < 1e3);
        // the amorphous state Kaito et al.'s density with the crystal's c_p: 1.204 MJ/(m³·K)
        let a = s.heat_capacity_of(State::Amorphous).unwrap();
        assert!((a - 3420.0 * 351.92).abs() < 50.0, "{a}");
        assert!((sb2s3_kaito_deficit().measured - 26.293).abs() < 1e-3);
    }
}
