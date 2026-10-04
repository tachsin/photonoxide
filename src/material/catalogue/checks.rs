//! The catalogue's validation cases: each model against the numbers its paper prints.
//!
//! `crate::validation` lists them; the functions here run them.

use super::models;
use crate::material::Material;
use crate::units::Wavelength;
use crate::validation::Outcome;

fn n(material: &Material, lam: f64) -> f64 {
    Wavelength::um(lam)
        .and_then(|w| material.refractive_index(w))
        .map_or(f64::NAN, |n| n.re)
}

fn largest(errors: impl Iterator<Item = f64>) -> f64 {
    errors.fold(
        0.0,
        |a: f64, e| if e.is_nan() { f64::NAN } else { a.max(e) },
    )
}

/// Leviton & Frey's Table 4: n of Corning 7980 at 16 wavelengths (µm) and these temperatures (K),
/// computed by the authors from their Table 3 and printed to five decimals.
pub(crate) const LEVITON_TABLE_4_T: [f64; 13] = [
    30.0, 40.0, 50.0, 60.0, 80.0, 100.0, 120.0, 160.0, 200.0, 240.0, 275.0, 295.0, 300.0,
];
pub(crate) const LEVITON_TABLE_4: [(f64, [f64; 13]); 16] = [
    (
        0.4,
        [
            1.46899, 1.46902, 1.46905, 1.46907, 1.46912, 1.46918, 1.46926, 1.46948, 1.46974,
            1.47005, 1.47036, 1.47054, 1.47059,
        ],
    ),
    (
        0.5,
        [
            1.46129, 1.46131, 1.46133, 1.46135, 1.46140, 1.46146, 1.46154, 1.46175, 1.46199,
            1.46228, 1.46257, 1.46275, 1.46279,
        ],
    ),
    (
        0.6,
        [
            1.45704, 1.45706, 1.45708, 1.45710, 1.45715, 1.45721, 1.45729, 1.45748, 1.45773,
            1.45801, 1.45829, 1.45846, 1.45850,
        ],
    ),
    (
        0.7,
        [
            1.45432, 1.45433, 1.45435, 1.45437, 1.45442, 1.45448, 1.45456, 1.45475, 1.45499,
            1.45527, 1.45554, 1.45571, 1.45575,
        ],
    ),
    (
        0.8,
        [
            1.45236, 1.45237, 1.45239, 1.45240, 1.45245, 1.45252, 1.45259, 1.45278, 1.45302,
            1.45329, 1.45356, 1.45373, 1.45377,
        ],
    ),
    (
        0.9,
        [
            1.45080, 1.45081, 1.45083, 1.45085, 1.45090, 1.45096, 1.45104, 1.45122, 1.45146,
            1.45173, 1.45200, 1.45216, 1.45220,
        ],
    ),
    (
        1.0,
        [
            1.44947, 1.44948, 1.44950, 1.44952, 1.44957, 1.44963, 1.44970, 1.44989, 1.45012,
            1.45039, 1.45066, 1.45082, 1.45086,
        ],
    ),
    (
        1.2,
        [
            1.44711, 1.44713, 1.44714, 1.44716, 1.44721, 1.44727, 1.44735, 1.44753, 1.44776,
            1.44803, 1.44829, 1.44845, 1.44850,
        ],
    ),
    (
        1.5,
        [
            1.44369, 1.44370, 1.44372, 1.44374, 1.44379, 1.44385, 1.44392, 1.44411, 1.44433,
            1.44460, 1.44486, 1.44502, 1.44507,
        ],
    ),
    (
        1.6,
        [
            1.44250, 1.44251, 1.44252, 1.44254, 1.44259, 1.44265, 1.44272, 1.44291, 1.44314,
            1.44340, 1.44367, 1.44383, 1.44387,
        ],
    ),
    (
        1.8,
        [
            1.43995, 1.43996, 1.43998, 1.44000, 1.44004, 1.44010, 1.44018, 1.44036, 1.44059,
            1.44086, 1.44112, 1.44128, 1.44132,
        ],
    ),
    (
        2.0,
        [
            1.43716, 1.43717, 1.43718, 1.43720, 1.43725, 1.43731, 1.43738, 1.43757, 1.43779,
            1.43806, 1.43833, 1.43849, 1.43853,
        ],
    ),
    (
        2.2,
        [
            1.43407, 1.43408, 1.43410, 1.43411, 1.43416, 1.43422, 1.43430, 1.43448, 1.43471,
            1.43498, 1.43524, 1.43541, 1.43545,
        ],
    ),
    (
        2.4,
        [
            1.43065, 1.43067, 1.43068, 1.43070, 1.43074, 1.43081, 1.43088, 1.43107, 1.43129,
            1.43156, 1.43183, 1.43200, 1.43204,
        ],
    ),
    (
        2.5,
        [
            1.42881, 1.42882, 1.42884, 1.42885, 1.42890, 1.42896, 1.42904, 1.42922, 1.42945,
            1.42972, 1.42999, 1.43016, 1.43021,
        ],
    ),
    (
        2.6,
        [
            1.42688, 1.42688, 1.42690, 1.42692, 1.42696, 1.42703, 1.42710, 1.42729, 1.42752,
            1.42779, 1.42806, 1.42823, 1.42828,
        ],
    ),
];

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// A largest deviation reported against zero.
fn deviation(largest: f64, tolerance: f64) -> Outcome {
    outcome(largest, 0.0, tolerance)
}

/// Leviton & Frey's Table 4 from their Table 3: the largest difference over all 208 entries.
pub(crate) fn leviton_table() -> Outcome {
    let errors = LEVITON_TABLE_4_T.iter().enumerate().flat_map(|(j, &t)| {
        let m = models::leviton_frey(t).ok();
        LEVITON_TABLE_4
            .iter()
            .map(move |(lam, row)| m.as_ref().map_or(f64::NAN, |m| (n(m, *lam) - row[j]).abs()))
            .collect::<Vec<_>>()
    });
    deviation(largest(errors), 6e-6)
}

/// The idler of an OPO pumped at `pump` (µm) in a periodically poled crystal of period
/// `period` (µm), all waves extraordinary (d33): the root of
/// n_p/λ_p − n_s/λ_s − n_i/λ_i − 1/Λ = 0 between degeneracy and `longest`, by bisection.
pub(crate) fn opo_idler(material: &Material, pump: f64, period: f64, longest: f64) -> f64 {
    let mismatch = |idler: f64| {
        let signal = 1.0 / (1.0 / pump - 1.0 / idler);
        n(material, pump) / pump
            - n(material, signal) / signal
            - n(material, idler) / idler
            - 1.0 / period
    };
    let (mut lo, mut hi) = (2.0 * pump + 1e-9, longest);
    if mismatch(lo).signum() == mismatch(hi).signum() {
        return f64::NAN;
    }
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if mismatch(mid).signum() == mismatch(lo).signum() {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Zelmon et al.'s Fig. 3: the predicted idler of Myers et al.'s 1.064 µm-pumped PPLN OPO at a
/// 30 µm grating, read off the plot as 3.35 µm (±0.03).
pub(crate) fn zelmon_opo() -> Outcome {
    let (e, _) = models::ZELMON_CONGRUENT;
    let idler = models::zelmon("LiNbO3 (e)", e, "Table 1")
        .map_or(f64::NAN, |m| opo_idler(&m, 1.064, 30.0, 5.0));
    outcome(idler, 3.35, 0.03)
}

/// Zelmon's congruent n_o and n_e at 633 nm against the values Jazbinšek & Zgonik quote for
/// their tensors (n_o = 2.2864, n_e = 2.2022, their ref. 27): the larger difference.
pub(crate) fn zelmon_633() -> Outcome {
    let (e, o) = models::ZELMON_CONGRUENT;
    let pair = models::zelmon("o", o, "").and_then(|o| Ok((o, models::zelmon("e", e, "")?)));
    let error = pair.map_or(f64::NAN, |(o, e)| {
        (n(&o, 0.633) - 2.2864)
            .abs()
            .max((n(&e, 0.633) - 2.2022).abs())
    });
    deviation(error, 2e-4)
}

/// Jundt's Fig. 1: the idler of a 1.064 µm-pumped PPLN OPO at 250 °C and a 25.5 µm grating
/// (at 25 °C, expanded by Jundt's Eq. (3)), read off the plot as 4.755 µm (±0.02).
pub(crate) fn jundt_opo() -> Outcome {
    let celsius: f64 = 250.0;
    let dt = celsius - 25.0;
    let period = 25.5 * (1.0 + 1.54e-5 * dt + 5.3e-9 * dt * dt);
    let idler =
        models::jundt(celsius + 273.15).map_or(f64::NAN, |m| opo_idler(&m, 1.064, period, 5.0));
    outcome(idler, 4.755, 0.02)
}

/// Gayer et al.'s comparison with Zelmon et al. at 21 °C, after exchanging Zelmon's MgO
/// columns: the largest difference in n_e over 0.5–3 µm and in n_o over 0.5–1.62 µm.
pub(crate) fn gayer_zelmon() -> Outcome {
    let (printed_e, printed_o) = models::ZELMON_MGO;
    let t = models::ZELMON_T;
    let run = || -> crate::Result<f64> {
        // the printed n_e column is n_o, and the printed n_o column n_e (Gayer et al., Sec. 4.3.1)
        let (zo, ze) = (
            models::zelmon("o", printed_e, "")?,
            models::zelmon("e", printed_o, "")?,
        );
        let (go, ge) = (models::gayer(false, t)?, models::gayer(true, t)?);
        let grid = |lo: f64, hi: f64| (0..=250).map(move |i| lo + (hi - lo) * f64::from(i) / 250.0);
        let e = largest(grid(0.5, 3.0).map(|l| (n(&ge, l) - n(&ze, l)).abs()));
        let o = largest(grid(0.5, 1.62).map(|l| (n(&go, l) - n(&zo, l)).abs()));
        Ok(e.max(o))
    };
    deviation(run().unwrap_or(f64::NAN), 5.1e-4)
}

/// Gehrsitz et al.'s Table II, GaAs Fit 2: n∞² = A + C₁/E₁² + C₀/E₀² (Eq. (8)) from the
/// temperature forms at 298, 185 and 103 K against the table's columns.
pub(crate) fn gehrsitz_n_inf() -> Outcome {
    let errors = [(298.0, 10.89761), (185.0, 10.75246), (103.0, 10.65684)]
        .iter()
        .map(|&(t, expected)| {
            let (a, c0, e0, c1, e1_sq) = models::gehrsitz_parameters(0.0, t);
            (a + c1 / e1_sq + c0 / (e0 * e0) - expected).abs()
        });
    deviation(largest(errors), 1e-4)
}

/// Gehrsitz et al.'s Eq. (11) against Table II's E₀² of GaAs at 298, 185 and 103 K.
pub(crate) fn gehrsitz_gap() -> Outcome {
    let errors = [(298.0, 1.321079), (185.0, 1.416284), (103.0, 1.475453)]
        .iter()
        .map(|&(t, expected)| (models::gap_gaas(t).powi(2) - expected).abs());
    deviation(largest(errors), 2e-6)
}

/// Table IV's analytic model against each sample's own fit (Table III) at 23 °C, over the
/// grating-coupling band 0.73–0.83 µm (where every sample was measured to 5e-4): the largest
/// difference as a fraction of that sample's Δn_max(Table IV) + Δn_max(Table III).
pub(crate) fn gehrsitz_samples() -> Outcome {
    let ratios = models::TABLE_III
        .iter()
        .zip(models::TABLE_IV_QUALITY)
        .map(|(sample, (x, iv))| {
            let Ok(m) = models::gehrsitz(x, 296.15) else {
                return f64::NAN;
            };
            let (lo, _) = m.range();
            let band = (0..=100)
                .map(|i| 0.73 + 0.10 * f64::from(i) / 100.0)
                .filter(|&l| l >= lo.to_um());
            let worst =
                largest(band.map(|l| (n(&m, l) - models::table_iii_index(sample, l)).abs()));
            worst / ((iv + sample.2) * 1e-3)
        });
    deviation(largest(ratios), 1.0)
}

/// Papatryfonos et al.'s Table III (MBE AlGaAs, ellipsometry, room temperature) against the
/// model at 23 °C: the largest relative difference over the 17 points below the gap.
pub(crate) fn gehrsitz_papatryfonos() -> Outcome {
    const DATA: [(f64, [f64; 3]); 6] = [
        (0.0, [3.608, 3.405, 3.378]),
        (0.097, [3.557, 3.370, 3.343]),
        (0.219, [3.467, 3.304, 3.283]),
        (0.342, [3.382, 3.239, 3.218]),
        (0.411, [3.331, 3.204, 3.177]),
        (0.452, [3.300, 3.177, 3.150]),
    ];
    let errors = DATA.iter().flat_map(|&(x, values)| {
        let m = models::gehrsitz(x, 296.15).ok();
        [0.825, 1.3, 1.55]
            .into_iter()
            .zip(values)
            .filter_map(move |(lam, v)| {
                let m = m.as_ref()?;
                // 825 nm lies above GaAs's gap: outside the model's range, as it should be
                let w = Wavelength::um(lam).ok()?;
                (w >= m.range().0).then(|| (n(m, lam) / v - 1.0).abs())
            })
    });
    deviation(largest(errors), 0.01)
}

/// Skauli et al.'s Table I: (Λ/m in µm, λ_2ω in nm), first-order-equivalent QPM periods of SHG
/// at 21 °C, corrected for focusing.
pub(crate) const SKAULI_TABLE_I: [(f64, f64); 8] = [
    (61.2, 2065.9),
    (38.6, 1757.5),
    (26.3, 1546.8),
    (20.40, 1427.3),
    (12.24, 1220.3),
    (6.80, 1031.4),
    (5.56, 979.1),
    (5.26, 966.1),
];

/// Skauli et al.'s Table I against the Pikhtin form at 21 °C: the largest relative difference
/// of Λ/m over the rows whose second harmonic lies in the model's range (all but 966.1 nm).
pub(crate) fn skauli_shg() -> Outcome {
    let errors = SKAULI_TABLE_I
        .iter()
        .filter(|(_, sh)| sh / 1000.0 >= models::SKAULI_LAMBDA.0)
        .map(|&(period, sh)| {
            models::skauli(294.15)
                .and_then(|m| models::shg_period(&m, 2.0 * sh / 1000.0))
                .map_or(f64::NAN, |p| (p / period - 1.0).abs())
        });
    deviation(largest(errors), 0.003)
}

/// Skauli et al.: "dn/dT = 2.33 × 10⁻⁴ K⁻¹ obtained from our fits" at 1.5 µm and room
/// temperature (shown in units of 10⁻⁴ K⁻¹).
pub(crate) fn skauli_dndt() -> Outcome {
    let at = |t: f64| models::skauli(t).map_or(f64::NAN, |m| n(&m, 1.5));
    let t = models::SKAULI_T0;
    let dndt = (at(t + 1.0) - at(t - 1.0)) / 2.0;
    outcome(dndt * 1e4, 2.33, 0.005)
}

/// Afromowitz's η of GaAs, printed as 0.1032 (Eq. (11) with E₀ = 3.65, E_d = 36.1, E_Γ = 1.424 eV).
pub(crate) fn afromowitz_eta() -> Outcome {
    let (e0, ed, gap) = models::afromowitz_parameters(0.0);
    let (_, eta) = models::afromowitz_constants(e0, ed, gap);
    outcome(eta, 0.1032, 5e-5)
}

/// Shoji et al.'s Miller Δ₃₃ of congruent LiNbO₃ (Table 12, 10⁻¹³ m/V) from their d33
/// (Table 10) and Zelmon's n_e: Δ = d/[(n²(2ω) − 1)(n²(ω) − 1)²] (Eq. (1)), at the fundamentals
/// 1.313, 1.064 and 0.852 µm; the largest relative difference.
pub(crate) fn shoji_miller() -> Outcome {
    let (e, _) = models::ZELMON_CONGRUENT;
    let errors = [
        (1.313, 19.5, 3.92),
        (1.064, 25.2, 4.73),
        (0.852, 25.7, 4.34),
    ]
    .into_iter()
    .map(|(lam, d, delta)| {
        models::zelmon("e", e, "").map_or(f64::NAN, |m| {
            let chi = |l: f64| n(&m, l).powi(2) - 1.0;
            // pm/V over 10^-13 m/V: a factor of 10
            let computed = 10.0 * d / (chi(lam / 2.0) * chi(lam).powi(2));
            (computed / delta - 1.0).abs()
        })
    });
    deviation(largest(errors), 0.01)
}

/// Ueno et al.'s Table 2 lists InGaP's n_ω = 3.12 at their fundamental, 1.579 µm, from Tanaka
/// et al.'s model: the model here at x = 0.
pub(crate) fn tanaka_ueno() -> Outcome {
    outcome(
        models::tanaka(0.0).map_or(f64::NAN, |m| n(&m, 1.579)),
        3.12,
        0.005,
    )
}

/// The indices Suzuki & Tada list beside InP's r41 (their Table I: 3.29, 3.23, 3.20 and 3.17 at
/// 1.064, 1.208, 1.306 and 1.50 µm, citing Pettit & Turner in the text) against Pettit & Turner's
/// fit at 298 K: the largest difference.
pub(crate) fn pettit_turner_suzuki() -> Outcome {
    let errors = [(1.064, 3.29), (1.208, 3.23), (1.306, 3.20), (1.50, 3.17)]
        .into_iter()
        .map(|(lam, v)| models::pettit_turner(298.0).map_or(f64::NAN, |m| (n(&m, lam) - v).abs()));
    deviation(largest(errors), 0.007)
}

/// Suzuki & Tada's half-wave voltages V = λ₀/(2n₀³|r41^T|) (Table I) from their r41^T and n₀:
/// the largest relative difference from the printed 11.4, 12.0, 12.9 and 14.4 kV.
pub(crate) fn suzuki_tada_voltages() -> Outcome {
    let errors = [
        (1.064, 3.29, 1.32, 11.4),
        (1.208, 3.23, 1.49, 12.0),
        (1.306, 3.20, 1.53, 12.9),
        (1.50, 3.17, 1.63, 14.4),
    ]
    .into_iter()
    .map(|(lam, n0, r, kv): (f64, f64, f64, f64)| {
        // µm over pm/V: 1e-6/1e-12 V, in kV
        let v = lam / (2.0 * n0.powi(3) * r) * 1e3;
        (v / kv - 1.0).abs()
    });
    deviation(largest(errors), 0.015)
}

/// Suzuki & Tada's Faust–Henry analysis (Table II) from the catalogue's clamped r₄₁ at
/// 1.064 µm: d^EO = −n₀⁴r₄₁ˢ/4 with their n₀ = 3.29, d^E = d^EO/(1 + C) with C = −0.53, and
/// d^L = d^EO − d^E, against the printed 39, 83 and −44 pm/V (the catalogue's constants): the
/// largest relative difference.
pub(crate) fn suzuki_tada_faust_henry() -> Outcome {
    let r = super::entry("inp").and_then(|e| {
        e.tensors
            .iter()
            .find(|t| t.clamping == super::Clamping::Clamped && t.wavelength == Some(1.064))
            .and_then(|t| t.value(4, 1))
    });
    let constant = |symbol: &str| {
        super::entry("inp")
            .and_then(|e| e.constants.into_iter().find(|c| c.symbol == symbol))
            .map_or(f64::NAN, |c| c.value)
    };
    let c = constant(r"C_{41}");
    let d_eo = r.map_or(f64::NAN, |r| -3.29_f64.powi(4) * r / 4.0);
    let d_e = d_eo / (1.0 + c);
    let errors = [
        (d_eo, constant(r"d_{41}^{EO}")),
        (d_e, constant(r"d_{41}^{E}")),
        (d_eo - d_e, constant(r"d_{41}^{L}")),
    ]
    .into_iter()
    .map(|(computed, printed)| (computed / printed - 1.0).abs());
    deviation(largest(errors), 0.015)
}

/// AlN's |d33| (Majkić et al.: 4.3 pm/V at 1030 nm) from the ratio they measure,
/// (0.169 ± 0.009)·d33(LiNbO₃), and the catalogue's Shoji d33 of congruent LiNbO₃ at 1.064 µm,
/// 25.2 pm/V, which they use.
pub(crate) fn majkic_d33() -> Outcome {
    let ln = super::entry("linbo3").and_then(|e| {
        e.tensors
            .iter()
            .find(|t| t.kind == super::TensorKind::SecondOrder && t.wavelength == Some(1.064))
            .and_then(|t| t.value(3, 3))
    });
    outcome(ln.map_or(f64::NAN, |d| 0.169 * d), 4.3, 0.05)
}

/// Rigler et al. 2015's Table I at 658 nm against their Sellmeier (Eq. (10)): the largest
/// difference over n_o and n_e of both polarities, against the SE column (`maie` false) or the
/// independent multi-angle ellipsometry column (`maie` true).
fn rigler_2015_658(maie: bool) -> f64 {
    // (n_o, n_e): SE, then MAIE; Al-polar, then N-polar
    const PRINTED: [[(f64, f64); 2]; 2] = [
        [(2.066, 2.114), (2.061, 2.113)],
        [(2.061, 2.113), (2.063, 2.110)],
    ];
    let errors = (0..2).flat_map(|polarity| {
        let (o, e) = PRINTED[polarity][usize::from(maie)];
        let ms = models::rigler_2015(polarity).ok();
        [(0, o), (1, e)].into_iter().map(move |(axis, v)| {
            ms.as_ref()
                .map_or(f64::NAN, |ms| (n(&ms[axis], 0.658) - v).abs())
        })
    });
    largest(errors)
}

/// The SE column of Rigler et al. 2015's Table I, from the same fit: to its three decimals and
/// the rounding of A and B.
pub(crate) fn rigler_2015_se() -> Outcome {
    deviation(rigler_2015_658(false), 1e-3)
}

/// The MAIE column of Rigler et al. 2015's Table I, an independent measurement at 658 nm,
/// within its stated ±0.01.
pub(crate) fn rigler_2015_maie() -> Outcome {
    deviation(rigler_2015_658(true), 0.01)
}

/// Ferrini et al.'s Table 3 holds together: ε₁ = n² − k² and ε₂ = 2nk from its n and k
/// against its printed ε, over all 37 rows (largest difference).
pub(crate) fn ferrini_table_consistency() -> Outcome {
    let errors = models::FERRINI_TABLE_3
        .iter()
        .flat_map(|&(_, n, k, e1, e2)| [(n * n - k * k - e1).abs(), (2.0 * n * k - e2).abs()]);
    deviation(largest(errors), 0.01)
}

/// The catalogue's InGaP above the gap passes through Ferrini et al.'s Table 3: n and k at
/// every printed energy (largest difference; the spline is exact at its knots).
pub(crate) fn ferrini_table_knots() -> Outcome {
    let errors = models::FERRINI_TABLE_3.iter().map(|&(e, n0, k0, _, _)| {
        models::ferrini_table().map_or(f64::NAN, |m| {
            Wavelength::um(crate::material::PHOTON_EV_UM / e)
                .and_then(|w| m.refractive_index(w))
                .map_or(f64::NAN, |c| (c.re - n0).abs().max((c.im - k0).abs()))
        })
    });
    deviation(largest(errors), 1e-12)
}
