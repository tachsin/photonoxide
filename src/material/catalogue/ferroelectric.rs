//! Lithium tantalate's and barium titanate's index models, and their validation cases.
//!
//! - **Lithium tantalate:** photonoxide's own two-term Sellmeier fits to W. L. Bond's table of
//!   n_o and n_e from 0.45 to 4.0 µm at room temperature (J. Appl. Phys. 36, 1674 (1965),
//!   doi:10.1063/1.1703106), as the refractiveindex.info database transcribes it (CC0). The
//!   cases check the fits against the table's rows and against the indices J. L. Casson et al.
//!   print in their Table 2 (J. Opt. Soc. Am. B 21, 1948 (2004), doi:10.1364/JOSAB.21.001948),
//!   which they take from Gervais & Fonseca's handbook chapter.
//! - **Barium titanate:** the index S. Abel et al. measured on their films at 1550 nm (Nat.
//!   Mater. 18, 42 (2019), doi:10.1038/s41563-018-0208-0, Supplementary Note 6: the mean n =
//!   2.286 by ellipsometry with a Cauchy model, and a birefringence of 0.03 they extrapolated
//!   from the bulk crystal's), constant over 1.50 to 1.60 µm: no film's dispersion is tabulated
//!   in the papers read.

use super::entries::{src, strings, table};
use super::{Axis, IndexModel};
use crate::Result;
use crate::material::{Material, Model, Provenance, SellmeierTerm};
use crate::units::Wavelength;
use crate::validation::Outcome;
use num_complex::Complex64 as c64;

/// A two-term Sellmeier fit, n² = A + Σ B λ²/(λ² − C²) (λ, C in µm): (A, [(B, C); 2]).
type Sellmeier = (f64, [(f64, f64); 2]);

/// Bond's table as refractiveindex.info has it: (λ µm, n_o, n_e), room temperature.
pub(crate) const BOND_TABLE: [(f64, f64, f64); 22] = [
    (0.45, 2.2420, 2.2468),
    (0.50, 2.2160, 2.2205),
    (0.60, 2.1834, 2.1878),
    (0.70, 2.1652, 2.1696),
    (0.80, 2.1538, 2.1578),
    (0.90, 2.1454, 2.1493),
    (1.00, 2.1391, 2.1432),
    (1.20, 2.1305, 2.1341),
    (1.40, 2.1236, 2.1273),
    (1.60, 2.1174, 2.1213),
    (1.80, 2.1120, 2.1170),
    (2.00, 2.1066, 2.1115),
    (2.20, 2.1009, 2.1053),
    (2.40, 2.0951, 2.0993),
    (2.60, 2.0891, 2.0936),
    (2.80, 2.0825, 2.0871),
    (3.00, 2.0755, 2.0799),
    (3.20, 2.0680, 2.0727),
    (3.40, 2.0601, 2.0649),
    (3.60, 2.0513, 2.0561),
    (3.80, 2.0424, 2.0473),
    (4.00, 2.0335, 2.0377),
];

/// The wavelengths (µm) Bond's table covers.
pub(crate) const BOND_LAMBDA: (f64, f64) = (0.45, 4.0);

/// n_o: within 3.3e-4 of Bond's 22 rows (rms 1.9e-4).
pub(crate) const LITAO3_O: Sellmeier = (2.240001, [(2.270321, 0.1944899), (18.71168, 28.29262)]);
/// n_e: within 6.7e-4 of the 22 rows (rms 2.5e-4; the table's own steps are irregular from 1.6
/// to 2.0 µm).
pub(crate) const LITAO3_E: Sellmeier = (2.181893, [(2.343314, 0.192952), (7.179453, 17.87132)]);

/// Casson et al.'s Table 2: (λ µm, n_e, n_o), the curvature-adjusted indices they use.
pub(crate) const CASSON_INDICES: [(f64, f64, f64); 4] = [
    (0.6328, 2.1809, 2.1763),
    (0.980, 2.1443, 2.1402),
    (1.330, 2.1296, 2.1259),
    (1.558, 2.1224, 2.1186),
];

/// Abel et al.'s films at 1550 nm: the mean index by ellipsometry and the birefringence n_o −
/// n_e they take from the bulk crystal's.
pub(crate) const ABEL_MEAN: f64 = 2.286;
/// See [`ABEL_MEAN`].
pub(crate) const ABEL_BIREFRINGENCE: f64 = 0.03;
/// The band (µm) the constant index is given for.
pub(crate) const ABEL_LAMBDA: (f64, f64) = (1.50, 1.60);

fn um(value: f64) -> Result<Wavelength> {
    Wavelength::um(value)
}

fn sellmeier(fit: Sellmeier) -> Model {
    let (a, terms) = fit;
    Model::Sellmeier {
        a,
        terms: terms.iter().map(|&(b, c)| SellmeierTerm { b, c }).collect(),
    }
}

fn sellmeier_rows(fits: [Sellmeier; 2]) -> Vec<Vec<String>> {
    let [o, e] = fits;
    vec![
        strings(&["$A$", &o.0.to_string(), &e.0.to_string()]),
        strings(&["$B_1$", &o.1[0].0.to_string(), &e.1[0].0.to_string()]),
        strings(&["$C_1$ (µm)", &o.1[0].1.to_string(), &e.1[0].1.to_string()]),
        strings(&["$B_2$", &o.1[1].0.to_string(), &e.1[1].0.to_string()]),
        strings(&["$C_2$ (µm)", &o.1[1].1.to_string(), &e.1[1].1.to_string()]),
    ]
}

/// Lithium tantalate's n_o and n_e (in that order), our fits to Bond's table.
pub(crate) fn litao3_bond() -> Result<Vec<Material>> {
    let provenance = Provenance {
        reference: "W. L. Bond, J. Appl. Phys. 36, 1674 (1965)".into(),
        doi: "10.1063/1.1703106".into(),
        data: "our own two-term Sellmeier fits to the paper's table as the refractiveindex.info database transcribes it (CC0)".into(),
        temperature: None,
        notes: "room temperature, 0.45 to 4.0 um; the fits are within 3.3e-4 (n_o) and 6.7e-4 (n_e) of the table, and within 5e-4 of the indices Casson et al. 2004 print at 632.8 to 1558 nm".into(),
    };
    let (lo, hi) = (um(BOND_LAMBDA.0)?, um(BOND_LAMBDA.1)?);
    Ok(vec![
        Material::new(
            "LiTaO3 (o)",
            sellmeier(LITAO3_O),
            lo,
            hi,
            provenance.clone(),
        )?,
        Material::new("LiTaO3 (e)", sellmeier(LITAO3_E), lo, hi, provenance)?,
    ])
}

/// The catalogue's model for lithium tantalate.
pub(super) fn litao3_model() -> IndexModel {
    IndexModel {
        id: "bond-1965".into(),
        name: "Bond 1965 (our fit)".into(),
        axes: vec![Axis::Ordinary, Axis::Extraordinary],
        equation: r"n_{o,e}^2 = A + \frac{B_1\lambda^2}{\lambda^2 - C_1^2} + \frac{B_2\lambda^2}{\lambda^2 - C_2^2}".into(),
        symbols: r"$\lambda$ in µm; $C_1$, $C_2$ in µm".into(),
        coefficients: vec![table(
            "photonoxide's fits to Bond's table (22 wavelengths from 0.45 to 4.0 µm), by least squares in n²",
            &["", "$n_o$", "$n_e$"],
            sellmeier_rows([LITAO3_O, LITAO3_E]),
        )],
        wavelength: BOND_LAMBDA,
        temperature: None,
        composition: None,
        accuracy: "the fits are within 3.3e-4 (n_o) and 6.7e-4 (n_e) of the table's four decimals; the table itself agrees with the indices Casson et al. use (from Gervais & Fonseca's handbook chapter) within 5e-4".into(),
        dndt: None,
        sources: vec![
            src("bond-1965", "the table, as refractiveindex.info transcribes it (CC0)"),
            src("casson-2004", "Table 2, for the check"),
        ],
        default: true,
        notes: "room temperature; minimum deviation on a prism. The paper itself (closed) is not yet read here: the numbers are the database's, which Casson et al.'s independent print of the handbook's indices confirms to 5e-4 from 632.8 to 1558 nm. Lithium tantalate is uniaxial positive, n_e above n_o by about 0.004 in the near infrared".into(),
        eval: |_, _| litao3_bond(),
    }
}

/// Barium titanate's n_o and n_e (in that order) at 1550 nm: Abel et al.'s mean index split by
/// their birefringence, n_o = n + Δn/2 and n_e = n − Δn/2.
pub(crate) fn batio3_abel() -> Result<Vec<Material>> {
    let provenance = Provenance {
        reference: "S. Abel et al., Nat. Mater. 18, 42 (2019)".into(),
        doi: "10.1038/s41563-018-0208-0".into(),
        data: "our own transcription of Supplementary Note 6, checked against the supplement"
            .into(),
        temperature: None,
        notes: "an epitaxial film on SrTiO3-buffered silicon (225 nm), mean index 2.286 at 1550 nm by ellipsometry (Cauchy model), birefringence 0.03 extrapolated from the bulk crystal's; constant over 1.50 to 1.60 um".into(),
    };
    let (lo, hi) = (um(ABEL_LAMBDA.0)?, um(ABEL_LAMBDA.1)?);
    let constant = |n: f64| Model::Constant(c64::new(n, 0.0));
    Ok(vec![
        Material::new(
            "BaTiO3 film (o)",
            constant(ABEL_MEAN + 0.5 * ABEL_BIREFRINGENCE),
            lo,
            hi,
            provenance.clone(),
        )?,
        Material::new(
            "BaTiO3 film (e)",
            constant(ABEL_MEAN - 0.5 * ABEL_BIREFRINGENCE),
            lo,
            hi,
            provenance,
        )?,
    ])
}

/// The catalogue's model for barium titanate films.
pub(super) fn batio3_model() -> IndexModel {
    IndexModel {
        id: "abel-2018-1550".into(),
        name: "Abel et al. 2018, at 1550 nm".into(),
        axes: vec![Axis::Ordinary, Axis::Extraordinary],
        equation: r"n_o = \bar n + \tfrac12\Delta n, \quad n_e = \bar n - \tfrac12\Delta n".into(),
        symbols: r"$\bar n$ the film's mean index, $\Delta n = n_o - n_e$".into(),
        coefficients: vec![table(
            "Supplementary Note 6",
            &["", "value"],
            vec![
                strings(&[r"$\bar n$", "2.286"]),
                strings(&[r"$\Delta n$", "0.03"]),
            ],
        )],
        wavelength: ABEL_LAMBDA,
        temperature: None,
        composition: None,
        accuracy: "the mean index by ellipsometry (Cauchy model) on the film; the birefringence not measured (the film too thin and its domains mixed) but extrapolated to 1.5 µm from the bulk crystal's Sellmeier equations. Other films at 1550 nm: 2.29 (Eltes et al. 2019, ellipsometry), 2.26 (Chelladurai et al. 2025, Deng et al. 2026)".into(),
        dndt: None,
        sources: vec![src("abel-2018", "Supplementary Note 6, Eq. (4)")],
        default: true,
        notes: "a constant index at 1550 nm, taken flat over 1.50 to 1.60 µm: no film's dispersion is tabulated in the papers read. The axes are a single domain's; a film of in-plane a-axis domains (Abel et al.'s) averages them in the plane. The films' index varies with growth and stoichiometry".into(),
        eval: |_, _| batio3_abel(),
    }
}

// ---------------------------------------------------------------------------------------------
// Validation cases
// ---------------------------------------------------------------------------------------------

fn n(material: &Material, lam: f64) -> f64 {
    Wavelength::um(lam)
        .and_then(|w| material.refractive_index(w))
        .map_or(f64::NAN, |n| n.re)
}

fn deviation(measured: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected: 0.0,
        tolerance,
        error: measured.abs(),
    }
}

/// Our fits against Bond's 22 rows, both axes: the largest difference.
pub(crate) fn litao3_bond_fit() -> Outcome {
    let worst = litao3_bond().map_or(f64::NAN, |m| {
        BOND_TABLE.iter().fold(0.0f64, |w, &(lam, no, ne)| {
            w.max((n(&m[0], lam) - no).abs())
                .max((n(&m[1], lam) - ne).abs())
        })
    });
    deviation(worst, 7e-4)
}

/// Our fits against the indices of Casson et al.'s Table 2 at 632.8, 980, 1330 and 1558 nm:
/// the largest difference.
pub(crate) fn litao3_casson() -> Outcome {
    let worst = litao3_bond().map_or(f64::NAN, |m| {
        CASSON_INDICES.iter().fold(0.0f64, |w, &(lam, ne, no)| {
            w.max((n(&m[0], lam) - no).abs())
                .max((n(&m[1], lam) - ne).abs())
        })
    });
    deviation(worst, 1e-3)
}

/// Abel et al.'s film index at 1550 nm against the other films' there (Eltes et al. 2019's
/// 2.29 by ellipsometry, Chelladurai et al. 2025's and Deng et al. 2026's 2.26): the largest
/// difference of the model's mean.
pub(crate) fn batio3_films() -> Outcome {
    let mean = batio3_abel().map_or(f64::NAN, |m| 0.5 * (n(&m[0], 1.55) + n(&m[1], 1.55)));
    let worst = [2.29, 2.26]
        .iter()
        .fold(0.0f64, |w, other| w.max((mean - other).abs()));
    deviation(worst, 0.03)
}
