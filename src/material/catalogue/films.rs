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
//!
//! Molybdenum silicide's, from D. V. Reddy, R. R. Nerem, S. W. Nam, R. P. Mirin, V. B. Verma,
//! Optica 7, 1649 (2020), doi:10.1364/OPTICA.400751, its Supplement 1 (Section 2). Their 4.1 nm
//! MoSi film (thickness by X-ray reflectivity) under its 2 nm sputtered silicon cap, deposited on
//! UV-fused silica, was fitted to the spectrophotometer's transmission and reflection: (n, κ) =
//! (5.817, 6.033) at 1550 nm (supplement p. 2), and Fig. S3 (p. 3) plots the n and κ "estimated
//! for the superconducting film" from 1300 to 1700 nm as vector curves; the table here is our
//! reading of those curves, which gives 5.817 and 6.033 at 1550 nm.

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

/// MoSi's n and κ against the wavelength (nm), read from the vector curves of Reddy et al.'s
/// Supplement 1, Fig. S3 (p. 3), against the figure's tick lines: (λ, n, κ).
pub(crate) const REDDY_MOSI: [(f64, f64, f64); 17] = [
    (1300.0, 5.438, 5.560),
    (1325.0, 5.474, 5.601),
    (1350.0, 5.509, 5.647),
    (1375.0, 5.544, 5.694),
    (1400.0, 5.578, 5.735),
    (1425.0, 5.615, 5.776),
    (1450.0, 5.657, 5.824),
    (1475.0, 5.690, 5.877),
    (1500.0, 5.735, 5.930),
    (1525.0, 5.787, 5.984),
    (1550.0, 5.817, 6.033),
    (1575.0, 5.846, 6.076),
    (1600.0, 5.895, 6.120),
    (1625.0, 5.938, 6.167),
    (1650.0, 5.975, 6.186),
    (1675.0, 6.018, 6.223),
    (1700.0, 6.053, 6.281),
];

/// MoSi's index at 1550 nm as Reddy et al. print it (Supplement 1, p. 2): n + iκ.
pub(crate) const REDDY_MOSI_1550: (f64, f64) = (5.817, 6.033);

const MOSI_NOTES: &str = "one film's fit, not a measurement of the material alone: a 4.1 nm MoSi film (thickness by X-ray reflectivity) under a 2 nm sputtered Si cap of real index 2.274, deposited on UV-fused silica, fitted to spectrophotometric transmission and reflection (Supplement 1, Section 2, p. 2); the composition, the structure and the temperature of the measurement aren't given, and a detector works below the film's critical temperature, where no index was measured; a thinner or thicker film, or one grown otherwise, may differ";

/// MoSi after Reddy et al.: their Fig. S3's n and κ from 1.3 to 1.7 µm, through natural cubic
/// splines in λ; it refuses wavelengths outside that range.
///
/// # Errors
///
/// None in practice: the table is fixed.
pub(crate) fn reddy_mosi() -> Result<Material> {
    let w: Vec<f64> = REDDY_MOSI.iter().map(|r| r.0 * 1e-3).collect();
    let (lo, hi) = (w[0], w[w.len() - 1]);
    Material::new(
        "MoSi",
        Model::Tabulated(Table::new(
            w,
            REDDY_MOSI.iter().map(|r| r.1).collect(),
            Some(REDDY_MOSI.iter().map(|r| r.2).collect()),
        )?),
        Wavelength::um(lo)?,
        Wavelength::um(hi)?,
        Provenance {
            reference: "D. V. Reddy, R. R. Nerem, S. W. Nam, R. P. Mirin, V. B. Verma, Optica 7, 1649 (2020), Supplement 1".into(),
            doi: "10.1364/OPTICA.400751".into(),
            data: "our reading of the vector curves of Supplement 1's Fig. S3 (p. 3), n and κ from 1300 to 1700 nm; (5.817, 6.033) printed at 1550 nm (p. 2)".into(),
            temperature: None,
            notes: MOSI_NOTES.into(),
        },
    )
}

fn reddy_model() -> IndexModel {
    IndexModel {
        id: "reddy-2020".into(),
        name: "Reddy et al. 2020".into(),
        axes: vec![Axis::Isotropic],
        equation: r"\tilde n(\lambda) = n + i\kappa \text{ from the table, through natural cubic splines in } \lambda".into(),
        symbols: r"$\lambda$ in nm in the table, µm in the model; $\kappa > 0$ absorbs".into(),
        coefficients: vec![
            table(
                "Supplement 1, Fig. S3 (p. 3), read from its vector curves",
                &["λ (nm)", "$n$", r"$\kappa$"],
                REDDY_MOSI
                    .iter()
                    .map(|r| vec![format!("{}", r.0), format!("{:.3}", r.1), format!("{:.3}", r.2)])
                    .collect(),
            ),
            table(
                "the value printed at 1550 nm (Supplement 1, p. 2)",
                &["λ (nm)", "$n$", r"$\kappa$"],
                vec![vec![
                    "1550".into(),
                    format!("{:.3}", REDDY_MOSI_1550.0),
                    format!("{:.3}", REDDY_MOSI_1550.1),
                ]],
            ),
        ],
        wavelength: (1.3, 1.7),
        temperature: None,
        composition: None,
        accuracy: "no uncertainty stated for the fit; the table rounds our reading of the curves to three decimals, and the reading at 1550 nm is the printed 5.817 and 6.033".into(),
        sources: vec![src("reddy-2020", "Supplement 1, Section 2 (p. 2) and Fig. S3 (p. 3)")],
        default: true,
        notes: MOSI_NOTES.into(),
        eval: |_, _| Ok(vec![reddy_mosi()?]),
        dndt: None,
    }
}

/// The detectors' films.
pub(super) fn entries() -> Vec<Entry> {
    vec![nbn(), mosi()]
}

fn mosi() -> Entry {
    Entry {
        id: "mosi".into(),
        name: "Molybdenum silicide (film)".into(),
        formula: "MoSi".into(),
        category: Category::Superconductor,
        summary: "The superconducting film of nanowire single-photon detectors, a few nm thick: strongly absorbing, 5.817 + 6.033i at 1550 nm, fitted to one 4.1 nm film under a 2 nm silicon cap.".into(),
        crystal: Crystal {
            system: CrystalSystem::NotSpecified,
            point_group: String::new(),
            space_group: None,
            structure: "not specified".into(),
            optical: OpticalClass::Isotropic,
            centrosymmetric: false,
            notes: "Reddy et al. fit one index to a film's transmission and reflection and name no structure or composition for it, so the catalogue takes it as isotropic and claims no symmetry".into(),
        },
        index: vec![reddy_model()],
        tensors: vec![],
        constants: vec![],
        missing: vec![],
        references: references(&["reddy-2020"]),
    }
}

fn nbn() -> Entry {
    Entry {
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
    }
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

    #[test]
    fn mosi_is_the_printed_index_at_1550_nm_and_refuses_outside_its_figure() {
        let m = &entry("mosi")
            .unwrap()
            .default_model()
            .unwrap()
            .materials(Conditions::default())
            .unwrap()[0];
        let at = |lam: f64| m.refractive_index(Wavelength::um(lam).unwrap());
        let n = at(1.55).unwrap();
        let (printed_n, printed_k) = REDDY_MOSI_1550;
        // the printed value, to its last digit
        assert!(
            (n.re - printed_n).abs() < 5e-4 && (n.im - printed_k).abs() < 5e-4,
            "{n}"
        );
        // the figure's ends as the docs state them, to the table's last digit
        let (lo, hi) = (at(1.3).unwrap(), at(1.7).unwrap());
        assert!(
            (lo.re - 5.438).abs() < 5e-4 && (lo.im - 5.560).abs() < 5e-4,
            "{lo}"
        );
        assert!(
            (hi.re - 6.053).abs() < 5e-4 && (hi.im - 6.281).abs() < 5e-4,
            "{hi}"
        );
        // both rise with the wavelength, κ above n, over the whole range
        let mut last = at(1.3).unwrap();
        for i in 1..40 {
            let n = at(1.3 + 0.01 * f64::from(i)).unwrap();
            assert!(n.re > last.re && n.im > last.im && n.im > n.re, "{i}: {n}");
            last = n;
        }
        assert!(at(1.29).is_err() && at(1.71).is_err());
        let tags = entry("mosi").unwrap().tags();
        for kind in ["system", "point-group", "symmetry"] {
            let t = tags.iter().find(|t| t.kind == kind).unwrap();
            assert!(t.label.contains("not specified"), "{kind}: {}", t.label);
        }
    }
}
