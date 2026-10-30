//! Electrical resistivities of films, beside the thermal conductivities: what a heater's
//! resistance, and so its voltage for a power, takes. Each value is a measurement on films made
//! one way, with their thickness, how they were made, the temperature coefficient where given and
//! the paper.
//!
//! - TiN: Tsai et al.'s reactively sputtered films on thermal oxide, 45 to 100 nm, room
//!   temperature (J. Appl. Phys. 73, 4462 (1993), doi:10.1063/1.352785, Tables II and III), and
//!   Kearney et al.'s RF reactively sputtered films on fused silica, 58 and 62.5 nm (Thin Solid
//!   Films 661, 78 (2018), doi:10.1016/j.tsf.2018.07.001, Section 3).
//!
//! A film's resistivity depends on how it was made far more than on its material: TiN's runs from
//! 45 to 350 µΩ·cm here, against 23 µΩ·cm for a single crystal. So each value keeps its film.

use serde::Serialize;

use super::references::references;
use super::{Reference, Source};

/// A material's electrical resistivity: its measured films and their papers.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct Resistivity {
    /// The id: the formula in lower case, e.g. `"tin"`.
    pub id: String,
    /// The name, e.g. `"Titanium nitride"`.
    pub name: String,
    /// The formula, e.g. `"TiN"`.
    pub formula: String,
    /// The measurements, by paper and film.
    pub values: Vec<ResistivityValue>,
    /// Every paper the values cite.
    pub references: Vec<Reference>,
}

/// One measurement of films made one way.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ResistivityValue {
    /// How the films were made, e.g. `"reactively sputtered, collimated, at 500 °C"`.
    pub film: String,
    /// The films' thickness, nm: one (both ends equal) or the range.
    pub thickness_nm: (f64, f64),
    /// The resistivity at room temperature, µΩ·cm as the papers print it: one value (both ends
    /// equal) or the range given.
    pub micro_ohm_cm: (f64, f64),
    /// The temperature coefficient of resistance, per K, when given.
    pub tcr: Option<f64>,
    /// The temperatures the coefficient spans, K.
    pub tcr_range: Option<(f64, f64)>,
    /// Where it comes from.
    pub source: Source,
    /// Anything else.
    pub notes: String,
}

impl ResistivityValue {
    /// The conductivity, S/m, at the ends of the resistivity's range (the larger first).
    pub fn conductivity(&self) -> (f64, f64) {
        // 1 µΩ·cm = 1e-8 Ω·m
        (1e8 / self.micro_ohm_cm.0, 1e8 / self.micro_ohm_cm.1)
    }
}

/// Every material with an electrical resistivity.
pub fn resistivities() -> Vec<Resistivity> {
    vec![titanium_nitride()]
}

/// The material with this id or formula (`"TiN"`, `"tin"`), in any case.
pub fn resistivity(name: &str) -> Option<Resistivity> {
    let name = name.to_lowercase();
    resistivities()
        .into_iter()
        .find(|m| m.id == name || m.formula.to_lowercase() == name)
}

/// Tsai et al.'s Table II: (collimated, substrate °C, rf bias V, ρ low and high µΩ·cm), films
/// 45 to 100 nm on thermal SiO₂ at room temperature.
pub(crate) const TSAI_TABLE_II: [(bool, f64, f64, (f64, f64)); 6] = [
    (false, 40.0, 0.0, (250.0, 350.0)),
    (true, 40.0, 0.0, (205.0, 215.0)),
    (false, 500.0, 0.0, (175.0, 200.0)),
    (true, 40.0, -115.0, (120.0, 135.0)),
    (true, 500.0, 0.0, (65.0, 70.0)),
    (true, 500.0, -100.0, (45.0, 60.0)),
];

/// Tsai et al.'s Table III's temperature coefficients of resistance, 5 to 280 K, ppm per K, in
/// Table II's order (none for the uncollimated films at 500 °C).
pub(crate) const TSAI_TABLE_III: [Option<f64>; 6] = [
    Some(228.0),
    Some(264.0),
    None,
    Some(561.0),
    Some(1018.0),
    Some(1355.0),
];

/// Tsai et al.'s Table IV, the parameters of their Eq. (3) for four of Table II's rows (in
/// Table II's order 0, 1, 4, 5): ρ_grain (µΩ·cm) and the electrons' mean free path l₀ (nm).
pub(crate) const TSAI_TABLE_IV: [(usize, f64, f64); 4] = [
    (0, 232.0, 39.0),
    (1, 181.0, 41.0),
    (4, 31.0, 72.0),
    (5, 14.0, 96.0),
];

/// Their ρ₀, a single crystal's, µΩ·cm (p. 4467).
pub(crate) const TSAI_RHO_0: f64 = 23.0;

/// Their Eq. (3): ρ = ρ₀(1 + 0.375 l₀/d) + ρ_grain, µΩ·cm, d and l₀ in nm.
pub(crate) fn tsai_eq_3(rho_grain: f64, l0: f64, d: f64) -> f64 {
    TSAI_RHO_0 * (1.0 + 0.375 * l0 / d) + rho_grain
}

/// The lowest-resistivity films (collimated, 500 °C, −100 V): Eq. (3) with Table IV's last row
/// at 100 nm, the thickest of Table II's films, against the 45 µΩ·cm of the abstract and Table
/// II's lower end (µΩ·cm shown).
pub(crate) fn tsai_lowest() -> crate::verdict::Outcome {
    let (_, rho_grain, l0) = TSAI_TABLE_IV[3];
    let rho = tsai_eq_3(rho_grain, l0, 100.0);
    crate::verdict::Outcome {
        measured: rho,
        expected: 45.0,
        tolerance: 0.5,
        error: (rho - 45.0).abs(),
    }
}

fn titanium_nitride() -> Resistivity {
    let tsai = |(collimated, celsius, bias, rho): (bool, f64, f64, (f64, f64)),
                tcr: Option<f64>| {
        let bias = if bias == 0.0 {
            "no bias".to_string()
        } else {
            format!("{bias} V rf bias")
        };
        ResistivityValue {
            film: format!(
                "reactively sputtered (Ti in 40 % N₂/Ar), {}, substrate at {celsius} °C, {bias}, on thermal SiO₂",
                if collimated {
                    "collimated"
                } else {
                    "uncollimated"
                }
            ),
            thickness_nm: (45.0, 100.0),
            micro_ohm_cm: rho,
            tcr: tcr.map(|ppm| ppm * 1e-6),
            tcr_range: tcr.map(|_| (5.0, 280.0)),
            source: Source {
                reference: "tsai-1993".into(),
                location: "Table II (p. 4465); the TCR Table III (p. 4466)".into(),
            },
            notes: "measured by four-point probe minutes after deposition: in air the sheet \
                    resistance rises up to 8 % in 12 h (uncollimated) and 5 % (collimated) (Fig. \
                    1). The TCR's films are 30 to 100 nm"
                .into(),
        }
    };
    let mut values: Vec<ResistivityValue> = TSAI_TABLE_II
        .iter()
        .zip(TSAI_TABLE_III)
        .map(|(&row, tcr)| tsai(row, tcr))
        .collect();
    let kearney = |film: &str, d: f64, rho: f64, tcr: f64, notes: &str| ResistivityValue {
        film: format!(
            "RF reactively sputtered (N₂:Ar 20:1, 3 mTorr, 100 W) on fused silica over 6 nm of Ti, {film}"
        ),
        thickness_nm: (d, d),
        micro_ohm_cm: (rho, rho),
        tcr: Some(tcr),
        tcr_range: Some((293.0, 773.0)),
        source: Source {
            reference: "kearney-2018".into(),
            location: "Section 3 (p. 80), Table 1 (p. 79)".into(),
        },
        notes: notes.into(),
    };
    values.push(kearney(
        "substrate at 600 °C (sample F)",
        58.0,
        70.2,
        0.095e-2,
        "the lowest; van der Pauw at room temperature after cycles to 500 °C in a cryostat (Fig. \
         2); Ti:N 1:0.98, 4.14 g/cm³ by RBS. The TCR from room temperature to 500 °C (Fig. 3)",
    ));
    values.push(kearney(
        "substrate at 350 °C, annealed at 600 °C for 4 h without breaking the vacuum (sample D)",
        62.5,
        78.0,
        0.099e-2,
        "the text gives 78.0 µΩ·cm to sample E but describes sample D (annealed without breaking \
         the vacuum), and adds that D's is 13 % below E's and 10 % above F's (78.0/70.2 = 1.11): \
         Fig. 2(b) has it as D's. Ti:N 1:1.04, 3.78 g/cm³ by RBS. Annealed after exposure to air \
         (sample E) the TCR is 0.069 %/K and the resistivity 13 % higher",
    ));
    Resistivity {
        id: "tin".into(),
        name: "Titanium nitride".into(),
        formula: "TiN".into(),
        values,
        references: references(&["tsai-1993", "kearney-2018"]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn found_by_id_or_formula_and_the_values_hold_together() {
        for name in ["TiN", "tin", "TIN"] {
            assert!(resistivity(name).is_some(), "{name}");
        }
        assert!(resistivity("W").is_none());
        let tin = resistivity("TiN").unwrap();
        assert_eq!(tin.values.len(), 8);
        for v in &tin.values {
            assert!(tin.references.iter().any(|r| r.key == v.source.reference));
            let (lo, hi) = v.micro_ohm_cm;
            assert!(lo >= 45.0 && lo <= hi && hi <= 350.0, "{lo} {hi}");
            if let Some(t) = v.tcr {
                assert!(t > 0.0 && t < 2e-3, "{t}");
            }
        }
        // the lowest: 45 µΩ·cm is 2.2e6 S/m
        let lowest = &tin.values[5];
        assert!((lowest.conductivity().0 - 2.222e6).abs() < 1e3);
        assert!((lowest.tcr.unwrap() - 1.355e-3).abs() < 1e-15);
    }

    #[test]
    fn eq_3_with_table_iv() {
        // the collimated, biased films at 500 °C: 55.4 at 45 nm and 45.3 µΩ·cm at 100 nm
        assert!((tsai_eq_3(14.0, 96.0, 45.0) - 55.4).abs() < 1e-9);
        assert!((tsai_eq_3(14.0, 96.0, 100.0) - 45.28).abs() < 1e-9);
        // the unbiased ones: 67.8 and 60.2 µΩ·cm, below Table II's 65 at 100 nm
        assert!((tsai_eq_3(31.0, 72.0, 45.0) - 67.8).abs() < 1e-9);
        assert!((tsai_eq_3(31.0, 72.0, 100.0) - 60.21).abs() < 1e-9);
        // the case, and its thin end inside Table II's 45 to 60
        assert!(tsai_lowest().passed());
        assert!(tsai_eq_3(14.0, 96.0, 45.0) <= TSAI_TABLE_II[5].3.1);
        // the two high-resistivity rows lie inside their ranges at both ends
        for (row, g, l) in [(0, 232.0, 39.0), (1, 181.0, 41.0)] {
            let (lo, hi) = TSAI_TABLE_II[row].3;
            for d in [45.0, 100.0] {
                let rho = tsai_eq_3(g, l, d);
                assert!(rho >= lo && rho <= hi, "{row} {d}: {rho}");
            }
        }
    }
}
