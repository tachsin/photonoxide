//! Lithium tantalate's thermo-optic coefficients measured in the infrared, on doped crystals:
//! Dolev et al.'s interferometry of 0.5 mol% MgO-doped stoichiometric LiTaO₃ at 1.0642 and
//! 3.39 µm (Appl. Phys. B 96, 423 (2009), doi:10.1007/s00340-009-3502-3, Table 1), and
//! Moutzouris et al.'s prism coupling of 8 mol% Mg-doped congruent LiTaO₃ at 1551 nm from 22 to
//! 200 °C (Opt. Mater. Express 1, 458 (2011), doi:10.1364/OME.1.000458, Table 1), with the case
//! that checks the latter's table against their own fit (Eq. (1), Table 2).

use crate::verdict::Outcome;

/// A linear fit's a₀ and a₁.
type Fit = (f64, f64);

/// Dolev et al.'s Table 1: the normalized thermo-optic coefficients β = (1/n) dn/dT = a₀ + a₁T
/// (T in °C, −30 to 80 °C) at the two infrared wavelengths: (λ µm, β_o's a₀ and a₁, β_e's a₀ and
/// a₁), per K. Printed as a₀ × 10⁶ and a₁ × 10⁸ for β_o, a₀ × 10⁵ and a₁ × 10⁸ for β_e.
pub(crate) const DOLEV_TABLE_1: [(f64, Fit, Fit); 2] = [
    (1.0642, (0.5586e-6, 0.92168e-8), (1.8017e-5, 6.1500e-8)),
    (3.390, (0.3329e-6, 1.4888e-8), (1.4945e-5, 3.8973e-8)),
];

/// Dolev et al.'s β at `celsius`: (β_o, β_e) at the k-th wavelength of their Table 1.
pub(crate) fn dolev_beta(k: usize, celsius: f64) -> (f64, f64) {
    let (_, (o0, o1), (e0, e1)) = DOLEV_TABLE_1[k];
    (o0 + o1 * celsius, e0 + e1 * celsius)
}

/// Moutzouris et al.'s wavelengths, µm, and temperatures, °C (Table 1).
pub(crate) const MOUTZOURIS_WAVELENGTHS: [f64; 5] = [0.450, 0.532, 0.6328, 0.964, 1.551];
pub(crate) const MOUTZOURIS_TEMPERATURES: [f64; 10] = [
    22.0, 40.0, 60.0, 80.0, 100.0, 120.0, 140.0, 160.0, 180.0, 200.0,
];

/// Moutzouris et al.'s Table 1, measured: at each temperature, n_o and n_e at each wavelength.
pub(crate) const MOUTZOURIS_TABLE_1: [[(f64, f64); 5]; 10] = [
    [
        (2.2373, 2.2397),
        (2.1980, 2.2007),
        (2.1717, 2.1746),
        (2.1358, 2.1391),
        (2.1138, 2.1162),
    ],
    [
        (2.2379, 2.2414),
        (2.1989, 2.2019),
        (2.1720, 2.1756),
        (2.1359, 2.1399),
        (2.1139, 2.1171),
    ],
    [
        (2.2386, 2.2428),
        (2.1993, 2.2032),
        (2.1725, 2.1767),
        (2.1364, 2.1404),
        (2.1141, 2.1183),
    ],
    [
        (2.2390, 2.2441),
        (2.1998, 2.2045),
        (2.1728, 2.1777),
        (2.1366, 2.1415),
        (2.1144, 2.1193),
    ],
    [
        (2.2397, 2.2458),
        (2.2001, 2.2057),
        (2.1730, 2.1789),
        (2.1367, 2.1423),
        (2.1148, 2.1202),
    ],
    [
        (2.2399, 2.2473),
        (2.2003, 2.2073),
        (2.1732, 2.1800),
        (2.1371, 2.1434),
        (2.1146, 2.1210),
    ],
    [
        (2.2405, 2.2492),
        (2.2007, 2.2087),
        (2.1732, 2.1811),
        (2.1370, 2.1441),
        (2.1151, 2.1219),
    ],
    [
        (2.2403, 2.2505),
        (2.2005, 2.2105),
        (2.1732, 2.1825),
        (2.1368, 2.1449),
        (2.1147, 2.1234),
    ],
    [
        (2.2409, 2.2519),
        (2.2009, 2.2113),
        (2.1733, 2.1838),
        (2.1367, 2.1462),
        (2.1148, 2.1247),
    ],
    [
        (2.2404, 2.2532),
        (2.2010, 2.2126),
        (2.1735, 2.1849),
        (2.1370, 2.1475),
        (2.1147, 2.1261),
    ],
];

/// Moutzouris et al.'s Table 2 for n_o and n_e: A₁ … A₅, B₁ … B₃.
const MOUTZOURIS_TABLE_2: [([f64; 5], [f64; 3]); 2] = [
    (
        [
            4.46828481,
            -0.01791123,
            0.10437434,
            -2.84541849e-3,
            7.87317793e-4,
        ],
        [2.55011881e-8, 2.30759571e-8, 1.84334432e-11],
    ),
    (
        [
            4.49361274,
            -0.02299905,
            0.09659086,
            -1.0505035e-3,
            6.30479079e-4,
        ],
        [2.49194536e-7, 4.34187245e-8, 7.43750502e-11],
    ),
];

/// Their Eq. (1): n² = A₁ + B₁F + A₂λ² + (A₃ + B₂F)/(λ² − (B₃F)²) + A₄/λ⁴ + A₅/λ⁶, with
/// F = (T − 22)(T + 568) (T in °C, λ in µm); `axis` 0 for n_o, 1 for n_e.
pub(crate) fn moutzouris_index(axis: usize, lam: f64, celsius: f64) -> f64 {
    let ([a1, a2, a3, a4, a5], [b1, b2, b3]) = MOUTZOURIS_TABLE_2[axis];
    let f = (celsius - 22.0) * (celsius + 568.0);
    let l2 = lam * lam;
    (a1 + b1 * f
        + a2 * l2
        + (a3 + b2 * f) / (l2 - (b3 * f).powi(2))
        + a4 / (l2 * l2)
        + a5 / (l2 * l2 * l2))
        .sqrt()
}

/// The slope of the least-squares line through Table 1's ten values at 1551 nm, 22 to 200 °C:
/// (dn_o/dT, dn_e/dT), per K.
pub(crate) fn moutzouris_slopes() -> (f64, f64) {
    let ts = MOUTZOURIS_TEMPERATURES;
    let mean_t = ts.iter().sum::<f64>() / ts.len() as f64;
    let slope = |axis: usize| {
        let ns: Vec<f64> = MOUTZOURIS_TABLE_1
            .iter()
            .map(|row| if axis == 0 { row[4].0 } else { row[4].1 })
            .collect();
        let mean_n = ns.iter().sum::<f64>() / ns.len() as f64;
        ts.iter()
            .zip(&ns)
            .map(|(t, n)| (t - mean_t) * (n - mean_n))
            .sum::<f64>()
            / ts.iter().map(|t| (t - mean_t).powi(2)).sum::<f64>()
    };
    (slope(0), slope(1))
}

/// Moutzouris et al.'s Eq. (1) with their Table 2 against their Table 1, all 100 values (both
/// indices, five wavelengths, ten temperatures): the mean absolute deviation, against the paper's
/// "~0.0004" (p. 462).
pub(crate) fn moutzouris_fit() -> Outcome {
    let mut sum = 0.0;
    let mut count = 0.0;
    for (row, &t) in MOUTZOURIS_TABLE_1.iter().zip(&MOUTZOURIS_TEMPERATURES) {
        for (&(no, ne), &lam) in row.iter().zip(&MOUTZOURIS_WAVELENGTHS) {
            sum +=
                (moutzouris_index(0, lam, t) - no).abs() + (moutzouris_index(1, lam, t) - ne).abs();
            count += 2.0;
        }
    }
    let mean = sum / count;
    Outcome {
        measured: mean,
        expected: 4e-4,
        tolerance: 5e-5,
        error: (mean - 4e-4).abs(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_case_passes_and_the_numbers() {
        let o = moutzouris_fit();
        assert!(o.passed(), "{o:?}");
        assert!((o.measured - 4.02e-4).abs() < 1e-6, "{o:?}");
        // the fit is the table at 22 °C, where F = 0 and Eq. (1) is their Eq. (2)
        for (&(no, ne), &lam) in MOUTZOURIS_TABLE_1[0].iter().zip(&MOUTZOURIS_WAVELENGTHS) {
            assert!((moutzouris_index(0, lam, 22.0) - no).abs() < 5e-5, "{lam}");
            assert!((moutzouris_index(1, lam, 22.0) - ne).abs() < 5e-5, "{lam}");
        }
        // the least-squares slopes at 1551 nm: 5.87e-6 and 5.38e-5 per K
        let (o, e) = moutzouris_slopes();
        assert!(
            (o - 5.87e-6).abs() < 1e-8 && (e - 5.376e-5).abs() < 1e-8,
            "{o} {e}"
        );
        // Dolev et al.'s β at 25 °C
        let (bo, be) = dolev_beta(0, 25.0);
        assert!((bo - 7.8902e-7).abs() < 1e-11 && (be - 1.95545e-5).abs() < 1e-10);
        let (bo, be) = dolev_beta(1, 25.0);
        assert!((bo - 7.051e-7).abs() < 1e-11 && (be - 1.5919325e-5).abs() < 1e-12);
    }
}
