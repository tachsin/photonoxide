//! The validation cases of the catalogue's thermal conductivities: the tables between their
//! rows, the bulk silicons against each other, the films against Ju & Goodson's measurements,
//! and Sondheimer's reduction against its limits and a quadrature.

use super::conductivity::{
    GS_SI, HO_PT, HO_PT_BETWEEN, HO_SI, HO_SI_BETWEEN, HO_TI, HO_TI_BETWEEN, HO_W, HO_W_BETWEEN,
    JU_GOODSON_PATH_UM, POWELL_SIO2, POWELL_SIO2_BETWEEN, conductivity, power_law, sondheimer,
};
use crate::validation::Outcome;

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// k (W/(m·K)) of the catalogue's `model` of `id` at T (K) and a film's thickness (µm).
pub(crate) fn k(id: &str, model: &str, t: f64, thickness_um: Option<f64>) -> f64 {
    conductivity(id)
        .and_then(|c| {
            c.model(Some(model))
                .and_then(|m| m.conductivity(Some(t), thickness_um).ok())
        })
        .unwrap_or(f64::NAN)
}

/// The tables' rows between the whole hundreds of kelvin (Ho et al.'s silicon, tungsten,
/// titanium and platinum, 223.2 to 973.2 K; Powell et al.'s fused quartz at 273 K) against the
/// power law between the rows about them: the largest relative deviation.
pub(crate) fn tables_between() -> Outcome {
    type Rows<'a> = &'a [(f64, f64)];
    let tables: [(Rows, Rows); 5] = [
        (&HO_SI, &HO_SI_BETWEEN),
        (&HO_W, &HO_W_BETWEEN),
        (&HO_TI, &HO_TI_BETWEEN),
        (&HO_PT, &HO_PT_BETWEEN),
        (&POWELL_SIO2, &POWELL_SIO2_BETWEEN),
    ];
    let worst = tables
        .iter()
        .flat_map(|(rows, between)| {
            between
                .iter()
                .map(|&(t, printed)| (power_law(rows, t) / (100.0 * printed) - 1.0).abs())
        })
        .fold(0.0, f64::max);
    outcome(worst, 0.0, 0.01)
}

/// Kremer et al.'s natural silicon at 300 K against Ho et al.'s: how far below (a fraction),
/// which the paper puts at about 3 %.
pub(crate) fn kremer_below_ho() -> Outcome {
    let below = 1.0 - k("si", "kremer-2004", 300.0, None) / k("si", "ho-1972", 300.0, None);
    outcome(below, 0.03, 0.01)
}

/// Ju & Goodson's measured films (Fig. 2, the present study's six points, read off the plot:
/// thickness in nm, k in W/(m·K)).
pub(crate) const JU_GOODSON_FIG_2: [(f64, f64); 6] = [
    (78.0, 75.0),
    (85.0, 76.8),
    (143.0, 82.5),
    (154.0, 87.0),
    (240.0, 90.0),
    (247.0, 99.7),
];

/// The film model against Ju & Goodson's films from 140 nm up: the largest relative deviation.
pub(crate) fn ju_goodson_films() -> Outcome {
    let worst = JU_GOODSON_FIG_2
        .iter()
        .filter(|(d, _)| *d >= 140.0)
        .map(|&(d, measured)| {
            (k("si", "ju-goodson-1999", 300.0, Some(d * 1e-3)) / measured - 1.0).abs()
        })
        .fold(0.0, f64::max);
    outcome(worst, 0.0, 0.12)
}

/// The film model at 220 nm (W/(m·K)), against the 90 W/(m·K) Jacques et al.'s Table 3 takes
/// for a 220 nm film from Asheghi et al.'s measurements.
pub(crate) fn film_220() -> Outcome {
    let k = k("si", "ju-goodson-1999", 300.0, Some(0.22));
    outcome(k, 90.0, 0.05 * 90.0)
}

/// Sondheimer's reduction against its limits: 1 − 3/(8δ) for a thick film (δ = 100, where the
/// rest is e^{−δ} small) and (3δ/4)(ln(1/δ) + 1 − γ) + δ²/2 for a thin one (δ = 1e-3, the rest
/// of order δ³ ln δ). The larger relative deviation.
pub(crate) fn sondheimer_limits() -> Outcome {
    const EULER: f64 = std::f64::consts::EULER_GAMMA;
    let thick: f64 = 100.0;
    let thin: f64 = 1e-3;
    let thick_error = (sondheimer(thick) / (1.0 - 3.0 / (8.0 * thick)) - 1.0).abs();
    let limit = 0.75 * thin * ((1.0 / thin).ln() + 1.0 - EULER) + 0.5 * thin * thin;
    let thin_error = (sondheimer(thin) / limit - 1.0).abs();
    outcome(thick_error.max(thin_error), 0.0, 1e-5)
}

/// The midpoint rule on Sondheimer's integral, ∫₀¹ (u − u³)(1 − e^{−δ/u}) du (t = 1/u), at a
/// 220 nm film's δ: its error against the closed form on `n` points.
pub(crate) fn sondheimer_midpoint_error(n: usize) -> f64 {
    let delta = 0.22 / JU_GOODSON_PATH_UM;
    let h = 1.0 / n as f64;
    let integral: f64 = (0..n)
        .map(|i| {
            let u = (i as f64 + 0.5) * h;
            (u - u * u * u) * (1.0 - (-delta / u).exp())
        })
        .sum::<f64>()
        * h;
    ((1.0 - 1.5 / delta * integral) - sondheimer(delta)).abs()
}

/// Glassbrenner & Slack's Table I between its rows: each inner row against the power law in T
/// of the two about it, the largest relative deviation, within the table's ±5 %. Between
/// neighbouring rows, at half the spacing, the catalogue's power law errs less.
pub(crate) fn glassbrenner_between() -> Outcome {
    let worst = GS_SI
        .windows(3)
        .map(|w| (power_law(&[w[0], w[2]], w[1].0) / (100.0 * w[1].1) - 1.0).abs())
        .fold(0.0, f64::max);
    outcome(worst, 0.0, 0.05)
}

/// The two bulk silicons where both state ±5 %: Glassbrenner & Slack's measured crystals against
/// Ho et al.'s recommended values at Ho et al.'s rows from 300 to 1000 K, the largest relative
/// difference, within the two accuracies summed.
pub(crate) fn glassbrenner_ho() -> Outcome {
    let worst = HO_SI
        .iter()
        .filter(|(t, _)| *t >= 300.0)
        .map(|&(t, _)| {
            (k("si", "glassbrenner-slack-1964", t, None) / k("si", "ho-1972", t, None) - 1.0).abs()
        })
        .fold(0.0, f64::max);
    outcome(worst, 0.0, 0.10)
}

/// The midpoint rule's order on 64 and 128 points, which the closed form makes 2.
pub(crate) fn sondheimer_order() -> Outcome {
    let order = (sondheimer_midpoint_error(64) / sondheimer_midpoint_error(128)).log2();
    outcome(order, 2.0, 0.05)
}
