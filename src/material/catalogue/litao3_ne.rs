//! Undoped congruent lithium tantalate's extraordinary index against temperature, two models:
//!
//! - N. A. Barboza and R. S. Cudney, Appl. Phys. B 95, 453 (2009),
//!   doi:10.1007/s00340-009-3434-y: Eq. (7) with the temperature terms of Table 2 (in λ_A, B and
//!   C), fitted to minimum-deviation indices on prisms of z-cut congruent wafers (457.9 to
//!   1064.2 nm, δn/n ≈ 5e-4) and to idler indices from quasi-phase-matched parametric generation
//!   (1.3 to 5 µm, 23 to 230 °C); stated for 0.3 to 5 µm and 23 to above 200 °C.
//! - A. Bruner, D. Eger, M. B. Oron, P. Blau, M. Katz and S. Ruschin, Proc. SPIE 4628, 66 (2002),
//!   doi:10.1117/12.460817: Eq. (4) with Table 1's congruent row, the temperature terms b and c in
//!   the first pole, fitted to quasi-phase-matched SHG wavelengths (pumps 0.76 to 1.85 µm, 30 to
//!   200 °C) and Bond's room-temperature indices; stated for 0.39 to 1.85 µm.
//!
//! Both give n_e only, so the ordinary index against temperature stays coming, and so does a
//! direct measurement of this crystal's dn/dT in the infrared: both models take their temperature
//! dependence from phase-matching wavelengths, which constrain the dispersion more than the index
//! itself. In their overlap they differ (the catalogue's write-up has the table): a finding,
//! not tuned away.

use super::entries::{src, strings, table, temperature};
use super::models::check_range;
use super::{Axis, IndexModel};
use crate::Result;
use crate::material::{Material, Model, Provenance};
use crate::units::Wavelength;

// ---------------------------------------------------------------------------------------------
// Barboza & Cudney 2009
// ---------------------------------------------------------------------------------------------

/// Table 1, T = 23 °C: A, λ_A (µm, fixed), B, λ_B (µm), C (µm⁻²).
pub(crate) const BARBOZA_TABLE_1: [f64; 5] = [2.97584, 0.138, 0.54622, 0.24028, -0.023497];
/// Table 1 as printed, with the stated uncertainties.
pub(crate) const BARBOZA_TABLE_1_PRINTED: [&str; 5] = [
    "2.97584 ± 0.00038",
    "0.138 µm (fixed arbitrarily)",
    "0.54622 ± 0.0007",
    "0.24028 ± 0.00065 µm",
    "−0.023497 ± 5.9 × 10⁻⁵ µm⁻²",
];

/// Table 2: A; λ_A = a₀ + a₂T²; B = b₀ + b₁T + b₂T²; λ_B; C = c₀ + c₁T + c₂T² (T in °C, λ in µm).
pub(crate) const BARBOZA_A: f64 = 2.976;
pub(crate) const BARBOZA_LAMBDA_A: [f64; 2] = [0.13798, 3.2e-8];
pub(crate) const BARBOZA_B: [f64; 3] = [0.5387, 3.6e-4, -1.2e-7];
pub(crate) const BARBOZA_LAMBDA_B: f64 = 0.24028;
pub(crate) const BARBOZA_C: [f64; 3] = [-0.02337, -4.3e-6, 3.5e-9];
/// Table 2 as printed.
pub(crate) const BARBOZA_TABLE_2_PRINTED: [&str; 5] = [
    "2.976",
    "(0.13798 + 3.2 × 10⁻⁸T²) µm",
    "0.5387 + 3.6 × 10⁻⁴T − 1.2 × 10⁻⁷T²",
    "0.24028 µm",
    "(−0.02337 − 4.3 × 10⁻⁶T + 3.5 × 10⁻⁹T²) µm⁻²",
];

/// The stated range: 0.3 to 5 µm (abstract; conclusions, p. 458).
pub(crate) const BARBOZA_LAMBDA: (f64, f64) = (0.3, 5.0);
/// 23 °C, the lowest temperature of the data, to 230 °C, the highest (p. 455): "from 23 to
/// above 200 °C" (abstract).
pub(crate) const BARBOZA_T: (f64, f64) = (296.15, 503.15);
/// The relative uncertainty of the indices measured by minimum deviation, δn/n ≈ 5e-4 (p. 454).
pub(crate) const BARBOZA_DN_OVER_N: f64 = 5e-4;
/// The wavelengths measured by minimum deviation (µm, p. 454).
pub(crate) const BARBOZA_PRISM_LAMBDAS: [f64; 7] =
    [0.4579, 0.4765, 0.488, 0.4965, 0.5145, 0.6328, 1.0642];

/// Eq. (7): n_e² = 1 + Aλ²/(λ² − λ_A²) + Bλ²/(λ² − λ_B²) + Cλ², as the refractiveindex.info
/// formula 4.
fn barboza_form([a, lambda_a, b, lambda_b, c]: [f64; 5]) -> Model {
    Model::Formula {
        number: 4,
        coefficients: vec![1.0, a, 2.0, lambda_a, 2.0, b, 2.0, lambda_b, 2.0, c, 2.0],
    }
}

/// Table 2's coefficients at `celsius`: A, λ_A, B, λ_B, C.
pub(crate) fn barboza_coefficients(celsius: f64) -> [f64; 5] {
    let t = celsius;
    let [a0, a2] = BARBOZA_LAMBDA_A;
    let [b0, b1, b2] = BARBOZA_B;
    let [c0, c1, c2] = BARBOZA_C;
    [
        BARBOZA_A,
        a0 + a2 * t * t,
        b0 + b1 * t + b2 * t * t,
        BARBOZA_LAMBDA_B,
        c0 + c1 * t + c2 * t * t,
    ]
}

fn barboza_provenance(kelvin: Option<f64>, data: &str) -> Provenance {
    Provenance {
        reference: "N. A. Barboza, R. S. Cudney, Appl. Phys. B 95, 453 (2009)".into(),
        doi: "10.1007/s00340-009-3434-y".into(),
        data: data.into(),
        temperature: kelvin,
        notes: "congruently grown LiTaO3, extraordinary index only; minimum deviation on z-cut prisms at 457.9-1064.2 nm (dn/n about 5e-4) and idler indices from QPM parametric generation at 1.3-5 um, 23-230 C; stated for 0.3-5 um and 23 to above 200 C".into(),
    }
}

/// Congruent LiTaO₃'s n_e at a temperature (K), from Barboza & Cudney's Eq. (7) and Table 2.
pub(crate) fn barboza(kelvin: f64) -> Result<Material> {
    check_range("temperature", kelvin, BARBOZA_T.0, BARBOZA_T.1)?;
    Material::new(
        "LiTaO3 (e)",
        barboza_form(barboza_coefficients(kelvin - 273.15)),
        Wavelength::um(BARBOZA_LAMBDA.0)?,
        Wavelength::um(BARBOZA_LAMBDA.1)?,
        barboza_provenance(
            Some(kelvin),
            "our own transcription of Eq. (7) and Table 2, read from the page images",
        ),
    )
}

/// The same at 23 °C with Table 1's coefficients, the room-temperature fit.
pub(crate) fn barboza_table_1() -> Result<Material> {
    Material::new(
        "LiTaO3 (e)",
        barboza_form(BARBOZA_TABLE_1),
        Wavelength::um(BARBOZA_LAMBDA.0)?,
        Wavelength::um(BARBOZA_LAMBDA.1)?,
        barboza_provenance(
            Some(296.15),
            "our own transcription of Eq. (7) and Table 1, read from the page images",
        ),
    )
}

/// The catalogue's model.
pub(super) fn barboza_model() -> IndexModel {
    let rows = |printed: [&str; 5]| {
        ["$A$", r"$\lambda_A$", "$B$", r"$\lambda_B$", "$C$"]
            .iter()
            .zip(printed)
            .map(|(s, p)| strings(&[s, p]))
            .collect()
    };
    IndexModel {
        id: "barboza-cudney-2009".into(),
        name: "Barboza & Cudney 2009, n_e with temperature".into(),
        axes: vec![Axis::Extraordinary],
        equation: r"n_e^2 = 1 + \frac{A\lambda^2}{\lambda^2 - \lambda_A^2} + \frac{B\lambda^2}{\lambda^2 - \lambda_B^2} + C\lambda^2".into(),
        symbols: r"$\lambda$ in µm; $T$ in °C in Table 2's $\lambda_A(T)$, $B(T)$ and $C(T)$".into(),
        coefficients: vec![
            table(
                "Table 2, temperature dependent (the model)",
                &["", "value"],
                rows(BARBOZA_TABLE_2_PRINTED),
            ),
            table(
                "Table 1, T = 23 °C (for the check)",
                &["", "value"],
                rows(BARBOZA_TABLE_1_PRINTED),
            ),
        ],
        wavelength: BARBOZA_LAMBDA,
        temperature: temperature(BARBOZA_T.0, BARBOZA_T.1, 296.15),
        composition: None,
        accuracy: "the indices measured by minimum deviation to δn/n ≈ 5e-4 (p. 454); the equation 'accurately predicts' parametric-generation tuning from 1.36 to 4.83 µm between room temperature and above 200 °C and the QPM conditions of UV SHG (p. 458); no index accuracy is stated beyond that. Table 2 at 23 °C lies 1.1e-4 relative above Table 1 from 457.9 to 1064.2 nm (material/litao3-barboza-tables)".into(),
        dndt: None,
        sources: vec![src(
            "barboza-cudney-2009",
            "Eq. (7), Tables 1 and 2 (p. 455), Eq. (8) (p. 456)",
        )],
        default: false,
        notes: "the extraordinary index only: n_o against temperature isn't in the paper, and stays coming. The temperature dependence rests on phase-matched parametric generation (idlers 1.3 to 5 µm at 23, 50, 80, 120, 180 and 230 °C) with the period's thermal expansion of Eq. (8) (a = 1.61e-5, b = 7e-9): it is the dispersion's change that phase matching fixes, so the model's dn_e/dT (8.3e-5 per K at 1.55 µm and 25 °C) is that fit's, not a direct measurement. In the overlap with bruner-2002-congruent (0.5 to 1.8 µm, 25 to 150 °C) the two models differ by 0.6e-3 to 1.2e-2 in n_e and their dn_e/dT by a factor of 7 to 100 (catalogue write-up); the crystal's composition isn't stated (taken near x = 0.485, p. 457)".into(),
        eval: |t, _| barboza(t).map(|m| vec![m]),
    }
}

// ---------------------------------------------------------------------------------------------
// Bruner et al. 2002
// ---------------------------------------------------------------------------------------------

/// Table 1, CLT: A, B, C, D, E, F, then b and c's factors of (T + 273.15)² (λ in µm, T in °C).
pub(crate) const BRUNER_CLT: [f64; 8] = [
    4.514261, 0.011901, 0.110744, -0.02323, 0.076144, 0.195596, 1.82194e-8, 1.5662e-8,
];
/// Table 1, CLT, as printed.
pub(crate) const BRUNER_CLT_PRINTED: [&str; 8] = [
    "4.514261",
    "0.011901",
    "0.110744",
    "-0.02323",
    "0.076144",
    "0.195596",
    "1.82194×10⁻⁸ (T+273.15)²",
    "1.5662×10⁻⁸ (T+273.15)²",
];
/// Table 1, SLT (Eq. (5)): A, B, C, D, E, F, G, H, then b and c's factors; for the check of
/// the supplier's index the paper fixed its fit to.
pub(crate) const BRUNER_SLT: [f64; 10] = [
    4.51584,
    7.294e-3,
    0.179937,
    -0.02358,
    0.073876,
    0.198887,
    0.001,
    7.999976,
    3.482449e-8,
    1.608146e-8,
];
/// The stated range for the congruent crystal: 0.39 to 1.85 µm (summary, p. 73).
pub(crate) const BRUNER_LAMBDA: (f64, f64) = (0.39, 1.85);
/// 25 °C (Bond's room-temperature indices, taken at 25 °C as Abedin & Ito take them, and the
/// reference of Eq. (3)) to 200 °C, the highest QPM temperature (p. 70).
pub(crate) const BRUNER_T: (f64, f64) = (298.15, 473.15);
/// The paper's only direct index: SLT's 2.177 at 633 nm, from the supplier (p. 70).
pub(crate) const BRUNER_SLT_633: f64 = 2.177;

/// n² of Eq. (4) (or Eq. (5) with the third pole) at λ (µm) and T (°C): the first pole's strength
/// B + b(T) and position C + c(T).
pub(crate) fn bruner_n2(
    coefficients: [f64; 6],
    third: (f64, f64),
    factors: (f64, f64),
    lam: f64,
    celsius: f64,
) -> f64 {
    let [a, b, c, d, e, f] = coefficients;
    let k2 = (celsius + 273.15).powi(2);
    let l2 = lam * lam;
    let (g, h) = third;
    let pole = if g == 0.0 { 0.0 } else { g / (l2 - h * h) };
    a + (b + factors.0 * k2) / (l2 - (c + factors.1 * k2).powi(2))
        + e / (l2 - f * f)
        + pole
        + d * l2
}

/// Eq. (4) with the CLT row at `celsius`, as the refractiveindex.info formula 4.
fn bruner_form(celsius: f64) -> Model {
    let [a, b, c, d, e, f, kb, kc] = BRUNER_CLT;
    let k2 = (celsius + 273.15).powi(2);
    Model::Formula {
        number: 4,
        coefficients: vec![
            a,
            b + kb * k2,
            0.0,
            c + kc * k2,
            2.0,
            e,
            0.0,
            f,
            2.0,
            d,
            2.0,
        ],
    }
}

/// Congruent LiTaO₃'s n_e at a temperature (K), from Bruner et al.'s Eq. (4) and Table 1.
pub(crate) fn bruner(kelvin: f64) -> Result<Material> {
    check_range("temperature", kelvin, BRUNER_T.0, BRUNER_T.1)?;
    Material::new(
        "LiTaO3 (e)",
        bruner_form(kelvin - 273.15),
        Wavelength::um(BRUNER_LAMBDA.0)?,
        Wavelength::um(BRUNER_LAMBDA.1)?,
        Provenance {
            reference: "A. Bruner, D. Eger, M. B. Oron, P. Blau, M. Katz, S. Ruschin, Proc. SPIE 4628, 66 (2002)".into(),
            doi: "10.1117/12.460817".into(),
            data: "our own transcription of Eq. (4) and Table 1's CLT row, read from the page images".into(),
            temperature: Some(kelvin),
            notes: "congruent LiTaO3, extraordinary index only; fitted to QPM SHG wavelengths (30-200 C) and Bond's room-temperature indices; stated for 0.39-1.85 um; 'the actual curves of the refractive indices can shift up or down a little' (p. 72)".into(),
        },
    )
}

/// The catalogue's model.
pub(super) fn bruner_model() -> IndexModel {
    IndexModel {
        id: "bruner-2002-congruent".into(),
        name: "Bruner et al. 2002, n_e with temperature".into(),
        axes: vec![Axis::Extraordinary],
        equation: r"n_e^2 = A + \frac{B + b(T)}{\lambda^2 - [C + c(T)]^2} + \frac{E}{\lambda^2 - F^2} + D\lambda^2".into(),
        symbols: r"$\lambda$ in µm; $T$ in °C".into(),
        coefficients: vec![table(
            "Table 1, CLT",
            &["", "value"],
            ["$A$", "$B$", "$C$", "$D$", "$E$", "$F$", "$b$", "$c$"]
                .iter()
                .zip(BRUNER_CLT_PRINTED)
                .map(|(s, p)| strings(&[s, p]))
                .collect(),
        )],
        wavelength: BRUNER_LAMBDA,
        temperature: temperature(BRUNER_T.0, BRUNER_T.1, 298.15),
        composition: None,
        accuracy: "'The standard deviation is 0.001327' (p. 71, after Fig. 3 at 170 °C, the crystal not named); the method 'is most sensitive to the refractive index dispersion and to its temperature dependence, but not so much to its absolute value', with one direct index (SLT's, 633 nm), so 'the actual curves ... can shift up or down a little' (p. 72). Against Bond's table, which the fit includes, n_e lies 0.8e-3 to 3.5e-3 below from 0.45 to 1.8 µm at 25 °C (rms 2.1e-3)".into(),
        dndt: None,
        sources: vec![src("bruner-2002", "Eq. (4) (p. 70), Table 1 (p. 71)")],
        default: false,
        notes: "the extraordinary index only: n_o against temperature isn't in the paper, and stays coming. The temperature terms are in the first pole only, from QPM SHG wavelengths of pumps from 0.76 to 1.85 µm, 30 to 200 °C in steps of 10 °C, with the period's thermal expansion of Eq. (3) (α = 1.6e-5, β = 7e-9): it is the dispersion's change that phase matching fixes, so the model's dn_e/dT (1.1e-6 per K at 1.55 µm and 25 °C) is that fit's, not a direct measurement. 25 °C, the lower end, is Bond's room temperature as the catalogue takes it; the measurements start at 30 °C. In the overlap with barboza-cudney-2009 the two models differ (catalogue write-up)".into(),
        eval: |t, _| bruner(t).map(|m| vec![m]),
    }
}

// ---------------------------------------------------------------------------------------------
// The two in their overlap
// ---------------------------------------------------------------------------------------------

/// The comparison's wavelengths (µm) and temperatures (°C): the overlap of the two ranges.
#[cfg(test)]
pub(crate) const COMPARE_LAMBDAS: [f64; 6] = [0.5, 0.8, 1.064, 1.31, 1.55, 1.8];
#[cfg(test)]
pub(crate) const COMPARE_CELSIUS: [f64; 4] = [25.0, 50.0, 100.0, 150.0];

/// One point of the comparison.
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Compared {
    pub(crate) lam: f64,
    pub(crate) celsius: f64,
    /// n_e of Barboza & Cudney and of Bruner et al.
    pub(crate) n: (f64, f64),
    /// dn_e/dT of each, per K.
    pub(crate) dndt: (f64, f64),
}

/// n_e and dn_e/dT of the two models over [`COMPARE_LAMBDAS`] and [`COMPARE_CELSIUS`]: dn/dT
/// by each model's [`IndexModel::thermo_optic`].
#[cfg(test)]
pub(crate) fn compare() -> Vec<Compared> {
    let (b, r) = (barboza_model(), bruner_model());
    let mut rows = Vec::new();
    for &celsius in &COMPARE_CELSIUS {
        let conditions = super::Conditions {
            temperature: Some(celsius + 273.15),
            composition: None,
        };
        for &lam in &COMPARE_LAMBDAS {
            let at = |m: &IndexModel| {
                let Ok(w) = Wavelength::um(lam) else {
                    return (f64::NAN, f64::NAN);
                };
                let n = m
                    .materials(conditions)
                    .and_then(|v| v[0].refractive_index(w))
                    .map_or(f64::NAN, |n| n.re);
                let d = m.thermo_optic(w, conditions).map_or(f64::NAN, |d| d[0]);
                (n, d)
            };
            let ((nb, db), (nr, dr)) = (at(&b), at(&r));
            rows.push(Compared {
                lam,
                celsius,
                n: (nb, nr),
                dndt: (db, dr),
            });
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(m: &Material, lam: f64) -> f64 {
        m.refractive_index(Wavelength::um(lam).unwrap()).unwrap().re
    }

    #[test]
    fn the_printed_coefficients_are_the_ones_evaluated() {
        // the leading number of each printed cell, with the paper's minus signs
        let lead = |s: &str| -> f64 {
            s.replace('−', "-")
                .trim_start_matches('(')
                .split([' ', '×'])
                .next()
                .unwrap()
                .parse()
                .unwrap()
        };
        for (v, p) in BARBOZA_TABLE_1.iter().zip(BARBOZA_TABLE_1_PRINTED) {
            assert_eq!(*v, lead(p), "{p}");
        }
        let t2 = [
            BARBOZA_A,
            BARBOZA_LAMBDA_A[0],
            BARBOZA_B[0],
            BARBOZA_LAMBDA_B,
            BARBOZA_C[0],
        ];
        for (v, p) in t2.iter().zip(BARBOZA_TABLE_2_PRINTED) {
            assert_eq!(*v, lead(p), "{p}");
        }
        assert!(BARBOZA_TABLE_2_PRINTED[1].contains("3.2 × 10⁻⁸"));
        assert!(BARBOZA_TABLE_2_PRINTED[2].contains("3.6 × 10⁻⁴T − 1.2 × 10⁻⁷"));
        assert!(BARBOZA_TABLE_2_PRINTED[4].contains("4.3 × 10⁻⁶T + 3.5 × 10⁻⁹"));
        for (v, p) in BRUNER_CLT.iter().zip(BRUNER_CLT_PRINTED) {
            let printed = lead(p) * if p.contains("10⁻⁸") { 1e-8 } else { 1.0 };
            assert!((v - printed).abs() <= 1e-15 * v.abs(), "{p}");
        }
    }

    #[test]
    fn the_formula_is_the_equation() {
        // Bruner's Eq. (4) by hand against the formula the model evaluates
        let m = bruner(373.15).unwrap();
        for lam in [0.4, 1.0, 1.8] {
            let [a, b, c, d, e, f, kb, kc] = BRUNER_CLT;
            let direct = bruner_n2([a, b, c, d, e, f], (0.0, 0.0), (kb, kc), lam, 100.0).sqrt();
            assert!((n(&m, lam) - direct).abs() < 1e-14, "{lam}");
        }
        // Barboza's Eq. (7) at 23 °C with Table 1, by hand
        let m = barboza_table_1().unwrap();
        let [a, la, b, lb, c] = BARBOZA_TABLE_1;
        let l2: f64 = 1.55 * 1.55;
        let direct = (1.0 + a * l2 / (l2 - la * la) + b * l2 / (l2 - lb * lb) + c * l2).sqrt();
        assert!((n(&m, 1.55) - direct).abs() < 1e-14);
    }

    #[test]
    fn the_two_models_in_their_overlap() {
        let rows = compare();
        assert_eq!(rows.len(), 24);
        let at = |lam: f64, celsius: f64| {
            *rows
                .iter()
                .find(|r| r.lam == lam && r.celsius == celsius)
                .unwrap()
        };
        // the numbers of the write-up's table, docs/methods/catalogue.md
        let r = at(1.55, 25.0);
        assert!(
            (r.n.0 - 2.12238).abs() < 5e-6 && (r.n.1 - 2.12045).abs() < 5e-6,
            "{r:?}"
        );
        assert!(
            (r.dndt.0 - 8.32e-5).abs() < 5e-8 && (r.dndt.1 - 1.07e-6).abs() < 5e-9,
            "{r:?}"
        );
        let r = at(1.55, 150.0);
        assert!((r.n.0 - r.n.1 - 0.01177).abs() < 5e-6, "{r:?}");
        // the difference in n_e grows with temperature, 0.6e-3 to 1.2e-2
        let dn: Vec<f64> = rows.iter().map(|r| r.n.0 - r.n.1).collect();
        let (lo, hi) = dn
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), &d| (a.min(d), b.max(d)));
        assert!(
            lo > 5e-4 && lo < 6e-4 && hi > 0.0119 && hi < 0.0120,
            "{lo} {hi}"
        );
        // and Barboza's dn_e/dT is 7 to 100 times Bruner's
        for r in &rows {
            let ratio = r.dndt.0 / r.dndt.1;
            assert!(ratio > 6.8 && ratio < 103.5, "{r:?}");
        }
    }

    #[test]
    fn the_ranges_hold() {
        assert!(barboza(296.15).is_ok() && barboza(503.15).is_ok());
        assert!(barboza(290.0).is_err() && bruner(480.0).is_err());
        let m = bruner(298.15).unwrap();
        assert!(m.refractive_index(Wavelength::um(1.9).unwrap()).is_err());
    }
}
