//! The films of single-photon detectors (the quantum plan's decision 6): niobium nitride's
//! optical constants, from V. Anant, A. J. Kerman, E. A. Dauler, J. K. W. Yang, K. M.
//! Rosfjord, K. K. Berggren, Opt. Express 16, 10750 (2008), doi:10.1364/OE.16.010750.
//!
//! Anant et al. had J. A. Woollam, Inc. measure NbN by spectroscopic ellipsometry at room
//! temperature, on a 12 nm film on a sapphire wafer (note 18, p. 10751), and print the index at
//! 1550 nm, n = 5.23 − i5.82 in their convention (p. 10754), which is 5.23 + 5.82i in the
//! library's (k > 0 absorbs). Their Fig. 6(b) (p. 10760) plots n and k from 600 to 1650 nm as
//! vector curves; the table here is our reading of those curves, which gives 5.229 and 5.824 at
//! 1550 nm, the printed value within 0.004. They apply it to their 4–6 nm films and name the
//! difference as a possible error: "the refractive index was measured on a thicker film (12 nm)
//! than the film we used to fabricated our devices on (4-6 nm)", the films from different
//! growth batches (p. 10757). Their absorptance at 6 K didn't differ significantly from the one
//! at room temperature (p. 10757).

use super::entries::{src, table};
use super::references::references;
use super::{Axis, Category, Crystal, CrystalSystem, Entry, IndexModel, OpticalClass};
use crate::Result;
use crate::material::{Material, Model, Provenance, Table};
use crate::units::Wavelength;

/// NbN's n and k against the wavelength (nm), read from the vector curves of Anant et al.'s
/// Fig. 6(b) (p. 10760): (λ, n, k).
pub(crate) const ANANT_NBN: [(f64, f64, f64); 13] = [
    (600.0, 2.59, 3.47),
    (700.0, 2.94, 3.89),
    (800.0, 3.27, 4.23),
    (900.0, 3.60, 4.52),
    (1000.0, 3.90, 4.77),
    (1100.0, 4.18, 5.00),
    (1200.0, 4.44, 5.20),
    (1300.0, 4.69, 5.39),
    (1400.0, 4.91, 5.57),
    (1500.0, 5.13, 5.74),
    (1550.0, 5.23, 5.82),
    (1600.0, 5.33, 5.90),
    (1650.0, 5.43, 5.98),
];

/// NbN's index at 1550 nm as Anant et al. print it (p. 10754; note 13, p. 10751), in the
/// library's convention: n + ik.
pub(crate) const ANANT_NBN_1550: (f64, f64) = (5.23, 5.82);

const NBN_NOTES: &str = "measured at room temperature by spectroscopic ellipsometry (J. A. Woollam, Inc.) on a 12 nm NbN film on a sapphire wafer (note 18, p. 10751); Anant et al. apply it to their 4–6 nm films and name the thicker film and the different growth batch as possible errors (p. 10757); a detector works below the film's critical temperature, where no index was measured, though their absorptance at 6 K didn't differ significantly from the one at room temperature (p. 10757)";

/// NbN after Anant et al.: their Fig. 6(b)'s n and k from 0.6 to 1.65 µm, through natural cubic
/// splines in λ; it refuses wavelengths outside that range.
///
/// # Errors
///
/// None in practice: the table is fixed.
pub(crate) fn anant_nbn() -> Result<Material> {
    let w: Vec<f64> = ANANT_NBN.iter().map(|r| r.0 * 1e-3).collect();
    let (lo, hi) = (w[0], w[w.len() - 1]);
    Material::new(
        "NbN",
        Model::Tabulated(Table::new(
            w,
            ANANT_NBN.iter().map(|r| r.1).collect(),
            Some(ANANT_NBN.iter().map(|r| r.2).collect()),
        )?),
        Wavelength::um(lo)?,
        Wavelength::um(hi)?,
        Provenance {
            reference: "V. Anant, A. J. Kerman, E. A. Dauler, J. K. W. Yang, K. M. Rosfjord, K. K. Berggren, Opt. Express 16, 10750 (2008)".into(),
            doi: "10.1364/OE.16.010750".into(),
            data: "our reading of the vector curves of Fig. 6(b) (p. 10760), n and k from 600 to 1650 nm; 5.23 − i5.82 printed at 1550 nm (p. 10754), here 5.23 + 5.82i".into(),
            temperature: None,
            notes: NBN_NOTES.into(),
        },
    )
}

fn anant_model() -> IndexModel {
    IndexModel {
        id: "anant-2008".into(),
        name: "Anant et al. 2008".into(),
        axes: vec![Axis::Isotropic],
        equation: r"\tilde n(\lambda) = n + ik \text{ from the table, through natural cubic splines in } \lambda".into(),
        symbols: r"$\lambda$ in nm in the table, µm in the model; $k > 0$ absorbs (the paper writes $n - ik$)".into(),
        coefficients: vec![
            table(
                "Fig. 6(b) (p. 10760), read from its vector curves",
                &["λ (nm)", "$n$", "$k$"],
                ANANT_NBN
                    .iter()
                    .map(|r| vec![format!("{}", r.0), format!("{:.2}", r.1), format!("{:.2}", r.2)])
                    .collect(),
            ),
            table(
                "the value printed at 1550 nm (p. 10754; note 13, p. 10751)",
                &["λ (nm)", "$n$", "$k$"],
                vec![vec![
                    "1550".into(),
                    format!("{:.2}", ANANT_NBN_1550.0),
                    format!("{:.2}", ANANT_NBN_1550.1),
                ]],
            ),
        ],
        wavelength: (0.6, 1.65),
        temperature: None,
        composition: None,
        accuracy: "no uncertainty stated for the ellipsometry; the table rounds our reading of the curves to two decimals, and the reading at 1550 nm (5.229, 5.824) is the printed 5.23 and 5.82 within 0.004".into(),
        sources: vec![src("anant-2008", "Fig. 6(b), p. 10760; p. 10754; notes 13 and 18, p. 10751")],
        default: true,
        notes: NBN_NOTES.into(),
        eval: |_, _| Ok(vec![anant_nbn()?]),
        dndt: None,
    }
}

/// The detectors' films.
pub(super) fn entries() -> Vec<Entry> {
    vec![Entry {
        id: "nbn".into(),
        name: "Niobium nitride (film)".into(),
        formula: "NbN".into(),
        category: Category::Superconductor,
        summary: "The superconducting film of nanowire single-photon detectors, a few nm thick: strongly absorbing, 5.23 + 5.82i at 1550 nm, measured at room temperature on a 12 nm film.".into(),
        crystal: Crystal {
            system: CrystalSystem::NotSpecified,
            point_group: String::new(),
            space_group: None,
            structure: "not specified".into(),
            optical: OpticalClass::Isotropic,
            centrosymmetric: false,
            notes: "Anant et al. measured one index by ellipsometry and name no structure for the film, so the catalogue takes it as isotropic and claims no symmetry".into(),
        },
        index: vec![anant_model()],
        tensors: vec![],
        constants: vec![],
        missing: vec![],
        references: references(&["anant-2008"]),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::catalogue::{Conditions, entry};

    #[test]
    fn nbn_is_the_printed_index_at_1550_nm_and_refuses_outside_its_figure() {
        let m = &entry("nbn")
            .unwrap()
            .default_model()
            .unwrap()
            .materials(Conditions::default())
            .unwrap()[0];
        let at = |lam: f64| m.refractive_index(Wavelength::um(lam).unwrap());
        let n = at(1.55).unwrap();
        let (printed_n, printed_k) = ANANT_NBN_1550;
        // the printed value, to its last digit
        assert!((n.re - printed_n).abs() < 0.005 && (n.im - printed_k).abs() < 0.005);
        // absorbing over its whole range, k > n
        for lam in [0.6, 1.0, 1.31, 1.65] {
            let n = at(lam).unwrap();
            assert!(n.im > n.re && n.re > 2.5, "{lam}: {n}");
        }
        assert!(at(0.59).is_err() && at(1.66).is_err());
        let tags = entry("nbn").unwrap().tags();
        for kind in ["system", "point-group", "symmetry"] {
            let t = tags.iter().find(|t| t.kind == kind).unwrap();
            assert!(t.label.contains("not specified"), "{kind}: {}", t.label);
        }
    }
}
