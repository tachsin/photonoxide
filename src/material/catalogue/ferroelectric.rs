//! Lithium tantalate's and barium titanate's index models, and their validation cases.
//!
//! - **Lithium tantalate:** photonoxide's own two-term Sellmeier fits to W. L. Bond's table of
//!   n_o and n_e from 0.45 to 4.0 µm at room temperature (J. Appl. Phys. 36, 1674 (1965),
//!   doi:10.1063/1.1703106, Table IV), as the refractiveindex.info database transcribes it (CC0)
//!   and as the paper prints it: the two agree in all 44 numbers but n_e at 3.00 µm, printed
//!   2.0299, a misprint for the database's 2.0799 (n_e − n_o would be −0.046 there and +0.004 to
//!   +0.005 at every other row). The cases check the fits against the table's rows and against
//!   the indices J. L. Casson et al. print in their Table 2 (J. Opt. Soc. Am. B 21, 1948 (2004),
//!   doi:10.1364/JOSAB.21.001948), which they take from Gervais & Fonseca's handbook chapter.
//! - **Lithium tantalate with temperature:** K. S. Abedin and H. Ito's Sellmeier equation in λ
//!   and T (J. Appl. Phys. 80, 6561 (1996), doi:10.1063/1.363679, Eq. (1), Tables I and II), from
//!   0.4 to 4.0 µm and 25 to 300 °C: its A_i fitted to Bond's table at 25 °C, its B_i to Iwasaki
//!   et al.'s indices at 468 and 644 nm from 25 to 300 °C. The cases reproduce the paper's
//!   standard deviations against Bond's table (Fig. 2), its fit of the A_i (Fig. 1, Table I), and
//!   its standard deviations against the points of Fig. 3, read off the figure's bitmap.
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

/// Bond's Table IV for LiTaO₃ (ω, ε): (λ µm, n_o, n_e), room temperature, as the paper prints
/// it but for n_e at 3.00 µm, printed 2.0299: the database's 2.0799 here.
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
        data: "our own two-term Sellmeier fits to the paper's Table IV (the refractiveindex.info database's transcription, CC0, checked against the paper: n_e at 3.00 um, printed 2.0299, is its misprint for 2.0799)".into(),
        temperature: None,
        notes: "room temperature, 0.45 to 4.0 um; crystals grown by A. Ballman (Bell Labs), prisms by minimum deviation; the fits are within 3.3e-4 (n_o) and 6.7e-4 (n_e) of the table, and within 5e-4 of the indices Casson et al. 2004 print at 632.8 to 1558 nm".into(),
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
            "photonoxide's fits to Bond's Table IV (22 wavelengths from 0.45 to 4.0 µm), by least squares in n²",
            &["", "$n_o$", "$n_e$"],
            sellmeier_rows([LITAO3_O, LITAO3_E]),
        )],
        wavelength: BOND_LAMBDA,
        temperature: None,
        composition: None,
        accuracy: "the fits are within 3.3e-4 (n_o) and 6.7e-4 (n_e) of the table's four decimals; the table itself agrees with the indices Casson et al. use (from Gervais & Fonseca's handbook chapter) within 5e-4".into(),
        dndt: None,
        sources: vec![
            src(
                "bond-1965",
                "Table IV, through refractiveindex.info (CC0), checked against the paper",
            ),
            src("casson-2004", "Table 2, for the check"),
        ],
        default: true,
        notes: "room temperature (not stated more closely); minimum deviation on prisms of crystals grown by A. Ballman at Bell Labs, read with a clinometer to 10″. The database's transcription matches the paper's Table IV in all 44 numbers but n_e at 3.00 µm, which the paper prints 2.0299: a misprint (n_e − n_o would be −0.046 there, +0.004 to +0.005 at every other wavelength), the database's 2.0799 kept. Casson et al.'s independent print of the handbook's indices agrees to 5e-4 from 632.8 to 1558 nm. Lithium tantalate is uniaxial positive, n_e above n_o by about 0.004 in the near infrared. For temperatures other than room temperature, the model abedin-ito-1996".into(),
        eval: |_, _| litao3_bond(),
    }
}

/// Abedin & Ito's Table I: (A₁, A₂, A₃, A₄) for n_o and n_e, A₃ in µm, A₂ in µm².
pub(crate) const ABEDIN_A: [[f64; 4]; 2] = [
    [4.51224, 0.0847522, 0.19876, -0.0239046],
    [4.52999, 0.0844313, 0.20344, -0.0237909],
];
/// Abedin & Ito's Table II: (B₁, B₂, B₃) for n_o and n_e.
pub(crate) const ABEDIN_B: [[f64; 3]; 2] = [
    [-9.66449e-9, 8.815e-8, 4.25637e-8],
    [1.72995e-7, -4.7733e-7, -8.31467e-8],
];
/// The reference temperature T₀ (°C), Bond's room temperature as Abedin & Ito take it.
pub(crate) const ABEDIN_T0: f64 = 25.0;
/// The paper's ranges: 0.4 to 4.0 µm, 25 to 300 °C.
pub(crate) const ABEDIN_LAMBDA: (f64, f64) = (0.4, 4.0);
pub(crate) const ABEDIN_T: (f64, f64) = (298.15, 573.15);

/// F = (T − T₀)(T + T₀ + 546), T in °C.
pub(crate) fn abedin_f(celsius: f64) -> f64 {
    (celsius - ABEDIN_T0) * (celsius + ABEDIN_T0 + 546.0)
}

/// Eq. (1), n² = A₁ + (A₂ + B₁F)/(λ² − (A₃ + B₂F)²) + B₃F + A₄λ², as the refractiveindex.info
/// formula 4 (one pole, the λ² term in C₁₀λ^C₁₁).
fn abedin_formula(axis: usize, celsius: f64) -> Model {
    let ([a1, a2, a3, a4], [b1, b2, b3]) = (ABEDIN_A[axis], ABEDIN_B[axis]);
    let f = abedin_f(celsius);
    Model::Formula {
        number: 4,
        coefficients: vec![
            a1 + b3 * f,
            a2 + b1 * f,
            0.0,
            a3 + b2 * f,
            2.0,
            0.0,
            0.0,
            0.0,
            2.0,
            a4,
            2.0,
        ],
    }
}

/// Lithium tantalate's n_o and n_e (in that order) at a temperature (K), from Abedin & Ito's
/// Eq. (1) and Tables I and II.
pub(crate) fn litao3_abedin(kelvin: f64) -> Result<Vec<Material>> {
    super::models::check_range("temperature", kelvin, ABEDIN_T.0, ABEDIN_T.1)?;
    let provenance = Provenance {
        reference: "K. S. Abedin, H. Ito, J. Appl. Phys. 80, 6561 (1996)".into(),
        doi: "10.1063/1.363679".into(),
        data: "our own transcription of Eq. (1) and Tables I and II, checked against the paper"
            .into(),
        temperature: Some(kelvin),
        notes: "0.4 to 4.0 um, 25 to 300 C; the A_i fitted to Bond 1965's table at 25 C (standard deviations 2.1e-4 and 3.5e-4), the B_i to Iwasaki et al. 1968's indices at 468 and 644 nm (1.4e-4 to 5.4e-4): the temperature dependence beyond the visible is the equation's".into(),
    };
    let (lo, hi) = (um(ABEDIN_LAMBDA.0)?, um(ABEDIN_LAMBDA.1)?);
    let celsius = kelvin - 273.15;
    Ok(vec![
        Material::new(
            "LiTaO3 (o)",
            abedin_formula(0, celsius),
            lo,
            hi,
            provenance.clone(),
        )?,
        Material::new("LiTaO3 (e)", abedin_formula(1, celsius), lo, hi, provenance)?,
    ])
}

/// The catalogue's temperature model for lithium tantalate.
pub(super) fn litao3_abedin_model() -> IndexModel {
    let row = |label: &str, o: &str, e: &str| strings(&[label, o, e]);
    IndexModel {
        id: "abedin-ito-1996".into(),
        name: "Abedin & Ito 1996, with temperature".into(),
        axes: vec![Axis::Ordinary, Axis::Extraordinary],
        equation: r"n_{o,e}^2 = A_1 + \frac{A_2 + B_1 F}{\lambda^2 - (A_3 + B_2 F)^2} + B_3 F + A_4\lambda^2, \quad F = (T - T_0)(T + T_0 + 546)".into(),
        symbols: r"$\lambda$ in µm; $T$ in °C, $T_0 = 25$ °C; $A_3$ in µm".into(),
        coefficients: vec![
            table(
                "Table I, the temperature-independent constants",
                &["", "$n_o$", "$n_e$"],
                vec![
                    row("$A_1$", "4.51224", "4.52999"),
                    row("$A_2$", "0.0847522", "0.0844313"),
                    row("$A_3$", "0.19876", "0.20344"),
                    row("$A_4$", "−0.0239046", "−0.0237909"),
                ],
            ),
            table(
                "Table II, the temperature terms",
                &["", "$n_o$", "$n_e$"],
                vec![
                    row("$B_1$", "−9.66449×10⁻⁹", "1.72995×10⁻⁷"),
                    row("$B_2$", "8.815×10⁻⁸", "−4.7733×10⁻⁷"),
                    row("$B_3$", "4.25637×10⁻⁸", "−8.31467×10⁻⁸"),
                ],
            ),
        ],
        wavelength: ABEDIN_LAMBDA,
        temperature: super::entries::temperature(ABEDIN_T.0, ABEDIN_T.1, 298.15),
        composition: None,
        accuracy: "standard deviations 2.1e-4 (n_o) and 3.5e-4 (n_e) against Bond's 22 rows at 25 °C (Fig. 2), and 3.5e-4 and 1.4e-4 (n_o at 468 and 644 nm), 5.4e-4 and 5.2e-4 (n_e) against Iwasaki et al.'s indices from 25 to 300 °C (Fig. 3), 'consistent with the uncertainties involved with the reading of data from their curve'".into(),
        dndt: None,
        sources: vec![
            src("abedin-ito-1996", "Eq. (1), Tables I and II, Figs. 1–3"),
            src("bond-1965", "Table IV, the data of Table I's fit"),
        ],
        default: false,
        notes: "the temperature terms rest on two visible wavelengths (468 and 644 nm), interpolated by the authors from H. Iwasaki et al.'s curves (Jpn. J. Appl. Phys. 7, 185 (1968)) at 23 temperatures; in the infrared they are the equation's extrapolation, and there the two indices part ways: at 1550 nm, from 25 to 300 °C, n_o rises by 8.0e-6 per K on average while n_e falls by 2.7e-6 per K (in the visible both rise, n_e by 8e-5 per K at 468 nm); no paper in hand measures the crystal's dn/dT in the infrared to check that against, so the model's infrared dn/dT is unvalidated. At 25 °C the equation is Eq. (2), a fit to the same table as bond-1965 (which is closer to it: 1.9e-4 and 2.5e-4 rms). The crystals' composition isn't stated (Bond's were grown by A. Ballman; Iwasaki et al.'s are not described in this paper)".into(),
        eval: |t, _| litao3_abedin(t),
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

/// Fig. 3's points, the indices Abedin & Ito interpolated from Iwasaki et al.'s curves, at 25 to
/// 300 °C in steps of 12.5 °C: n_o at 468 and 644 nm (panel (a)), n_e at 468 and 644 nm (panel
/// (b)). Read off the figure's bitmap (1020 × 1439 pixels): each hollow square's centre,
/// between the axis's nearest ticks (every 0.005 in panel (a), 1.15e-4 per pixel; every 0.01 in
/// panel (b), 1.49e-4 per pixel: the scan's scale drifts by a pixel over the panel), the 23
/// squares of each series at the 23 temperatures to 0.3 °C.
pub(crate) const ABEDIN_FIG3: [(usize, f64, [f64; 23]); 4] = [
    (
        0,
        0.468,
        [
            2.23236, 2.23236, 2.23247, 2.23247, 2.23247, 2.23259, 2.23259, 2.23270, 2.23282,
            2.23293, 2.23316, 2.23328, 2.23339, 2.23362, 2.23385, 2.23408, 2.23431, 2.23466,
            2.23500, 2.23534, 2.23569, 2.23603, 2.23649,
        ],
    ),
    (
        0,
        0.644,
        [
            2.17477, 2.17477, 2.17477, 2.17477, 2.17489, 2.17489, 2.17489, 2.17500, 2.17500,
            2.17511, 2.17511, 2.17523, 2.17534, 2.17546, 2.17557, 2.17569, 2.17580, 2.17592,
            2.17603, 2.17626, 2.17638, 2.17661, 2.17684,
        ],
    ),
    (
        1,
        0.468,
        [
            2.23757, 2.23787, 2.23860, 2.23904, 2.23949, 2.24007, 2.24066, 2.24125, 2.24199,
            2.24287, 2.24390, 2.24478, 2.24581, 2.24684, 2.24787, 2.24904, 2.25007, 2.25142,
            2.25291, 2.25425, 2.25560, 2.25709, 2.25873,
        ],
    ),
    (
        1,
        0.644,
        [
            2.17918, 2.17933, 2.17963, 2.18007, 2.18037, 2.18097, 2.18142, 2.18172, 2.18216,
            2.18276, 2.18336, 2.18410, 2.18485, 2.18545, 2.18619, 2.18709, 2.18784, 2.18873,
            2.18978, 2.19067, 2.19187, 2.19306, 2.19425,
        ],
    ),
];
/// The temperatures (°C) where Fig. 3's dashed curves, the paper's Eq. (1), are read.
pub(crate) const ABEDIN_CURVE_T: [f64; 5] = [100.0, 150.0, 200.0, 250.0, 290.0];
/// Fig. 3's dashed curves at those temperatures, the same four series: the mean row of the
/// dashes' pixels in the columns either side of the temperature, against the same ticks.
pub(crate) const ABEDIN_CURVES: [[f64; 5]; 4] = [
    [2.23236, 2.23313, 2.23402, 2.23502, 2.23598],
    [2.17487, 2.17522, 2.17563, 2.17603, 2.17644],
    [2.24029, 2.24375, 2.24782, 2.25282, 2.25731],
    [2.18152, 2.18396, 2.18672, 2.18973, 2.19239],
];
/// The standard deviations Fig. 3 prints for those four series, in the same order.
pub(crate) const ABEDIN_FIG3_SD: [f64; 4] = [3.5e-4, 1.4e-4, 5.4e-4, 5.2e-4];
/// The standard deviations against Bond's table Fig. 2's caption prints, n_o and n_e.
pub(crate) const ABEDIN_FIG2_SD: [f64; 2] = [2.1247e-4, 3.547e-4];

/// The root-mean-square difference (the population standard deviation, as the paper's
/// standard deviations are) of model against data.
fn rms(d: impl Iterator<Item = f64>) -> f64 {
    let (s, k) = d.fold((0.0, 0usize), |(s, k), x| (s + x * x, k + 1));
    (s / k as f64).sqrt()
}

/// Abedin & Ito's equation at 25 °C (their Eq. (2)) against Bond's 22 rows: the standard
/// deviations of n_o and n_e.
pub(crate) fn abedin_bond_sd() -> [f64; 2] {
    let Ok(m) = litao3_abedin(298.15) else {
        return [f64::NAN; 2];
    };
    [0, 1].map(|axis| {
        rms(BOND_TABLE
            .iter()
            .map(|&(lam, no, ne)| n(&m[axis], lam) - if axis == 0 { no } else { ne }))
    })
}

/// Their Fig. 2's standard deviations reproduced from Table I and Bond's table: the larger
/// difference.
pub(crate) fn litao3_abedin_bond() -> Outcome {
    let sd = abedin_bond_sd();
    let worst = (sd[0] - ABEDIN_FIG2_SD[0])
        .abs()
        .max((sd[1] - ABEDIN_FIG2_SD[1]).abs());
    deviation(worst, 1e-7)
}

/// A₁, A₂ and A₄ by linear least squares in n² for a fixed A₃ (the paper's Eqs. (3)–(7)), and
/// the variance Q they leave.
fn abedin_linear(axis: usize, a3: f64) -> ([f64; 3], f64) {
    let rows: Vec<([f64; 3], f64)> = BOND_TABLE
        .iter()
        .map(|&(lam, no, ne)| {
            let l2 = lam * lam;
            let y = if axis == 0 { no * no } else { ne * ne };
            ([1.0, 1.0 / (l2 - a3 * a3), l2], y)
        })
        .collect();
    let mut m = [[0.0; 4]; 3];
    for (f, y) in &rows {
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += f[i] * f[j];
            }
            m[i][3] += f[i] * y;
        }
    }
    // Gaussian elimination with partial pivoting on the 3 × 3 normal equations
    for c in 0..3 {
        let p = (c..3)
            .max_by(|&a, &b| m[a][c].abs().total_cmp(&m[b][c].abs()))
            .unwrap_or(c);
        m.swap(c, p);
        let pivot = m[c];
        for row in m.iter_mut().skip(c + 1) {
            let k = row[c] / pivot[c];
            for (x, p) in row.iter_mut().zip(pivot).skip(c) {
                *x -= k * p;
            }
        }
    }
    let mut x = [0.0; 3];
    for c in (0..3).rev() {
        let s: f64 = ((c + 1)..3).map(|j| m[c][j] * x[j]).sum();
        x[c] = (m[c][3] - s) / m[c][c];
    }
    let q = rows
        .iter()
        .map(|(f, y)| {
            let r = y - f.iter().zip(&x).map(|(a, b)| a * b).sum::<f64>();
            r * r
        })
        .sum();
    (x, q)
}

/// The paper's fit (Fig. 1): A₃ where Q is least, by golden-section search on 0.15 to 0.25 µm,
/// and A₁, A₂, A₄ there: (A₁, A₂, A₃, A₄).
pub(crate) fn abedin_refit(axis: usize) -> [f64; 4] {
    let g = 0.5 * (5f64.sqrt() - 1.0);
    let (mut lo, mut hi) = (0.15, 0.25);
    for _ in 0..80 {
        let (c, d) = (hi - g * (hi - lo), lo + g * (hi - lo));
        if abedin_linear(axis, c).1 < abedin_linear(axis, d).1 {
            hi = d;
        } else {
            lo = c;
        }
    }
    let a3 = 0.5 * (lo + hi);
    let ([a1, a2, a4], _) = abedin_linear(axis, a3);
    [a1, a2, a3, a4]
}

/// Their Table I refitted from Bond's table by their procedure: the largest relative deviation
/// of the eight constants from the printed ones.
pub(crate) fn litao3_abedin_refit() -> Outcome {
    let worst = [0, 1].iter().fold(0.0f64, |w, &axis| {
        abedin_refit(axis)
            .iter()
            .zip(ABEDIN_A[axis])
            .fold(w, |w, (ours, theirs)| w.max((ours / theirs - 1.0).abs()))
    });
    deviation(worst, 2e-4)
}

/// The standard deviations of Eq. (1) against Fig. 3's four series of points.
pub(crate) fn abedin_fig3_sd() -> [f64; 4] {
    ABEDIN_FIG3.map(|(axis, lam, points)| {
        rms(points.iter().enumerate().map(|(k, &p)| {
            let kelvin = 298.15 + 12.5 * k as f64;
            litao3_abedin(kelvin).map_or(f64::NAN, |m| n(&m[axis], lam)) - p
        }))
    })
}

/// Fig. 3's standard deviations reproduced from Tables I and II and the points read off the
/// figure: the largest difference (the paper prints two digits).
pub(crate) fn litao3_abedin_points() -> Outcome {
    let worst = abedin_fig3_sd()
        .iter()
        .zip(ABEDIN_FIG3_SD)
        .fold(0.0f64, |w, (ours, theirs)| w.max((ours - theirs).abs()));
    deviation(worst, 5e-5)
}

/// Eq. (1) against Fig. 3's dashed curves (the paper's own evaluation of it) at 100 to 290 °C,
/// all four series: the largest difference.
pub(crate) fn litao3_abedin_curves() -> Outcome {
    let worst =
        ABEDIN_FIG3
            .iter()
            .zip(ABEDIN_CURVES)
            .fold(0.0f64, |w, (&(axis, lam, _), curve)| {
                ABEDIN_CURVE_T.iter().zip(curve).fold(w, |w, (&t, read)| {
                    let ours = litao3_abedin(t + 273.15).map_or(f64::NAN, |m| n(&m[axis], lam));
                    w.max((ours - read).abs())
                })
            });
    deviation(worst, 2e-4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abedin_and_ito_reproduce() {
        for o in [
            litao3_abedin_bond(),
            litao3_abedin_refit(),
            litao3_abedin_points(),
            litao3_abedin_curves(),
        ] {
            assert!(o.error <= o.tolerance, "{o:?}");
        }
        // Table I's A₃, where Fig. 1's Q is least, to the paper's five decimals
        assert!((abedin_refit(0)[2] - 0.19876).abs() < 1e-5);
        assert!((abedin_refit(1)[2] - 0.20344).abs() < 2e-5);
        // Bond's printed n_e at 3.00 µm (2.0299) would give Eq. (2) a standard deviation of
        // 1e-2: the paper's 3.5e-4 is the corrected table's
        let m = litao3_abedin(298.15).unwrap();
        let printed = rms(BOND_TABLE
            .iter()
            .map(|&(lam, _, ne)| n(&m[1], lam) - if lam == 3.0 { 2.0299 } else { ne }));
        assert!(printed > 1e-2 && abedin_bond_sd()[1] < 4e-4, "{printed}");
    }

    #[test]
    fn abedin_and_ito_at_room_temperature_is_near_the_bond_fit() {
        let (m, b) = (litao3_abedin(298.15).unwrap(), litao3_bond().unwrap());
        let worst = (0..=355).fold(0.0f64, |w, k| {
            let lam = 0.45 + 0.01 * k as f64;
            w.max((n(&m[0], lam) - n(&b[0], lam)).abs())
                .max((n(&m[1], lam) - n(&b[1], lam)).abs())
        });
        assert!(worst < 7e-4, "{worst}");
    }

    #[test]
    fn abedin_and_ito_in_the_infrared() {
        // the temperature terms extrapolated to 1550 nm: n_o rises, n_e falls slightly
        let at = |t: f64| {
            let m = litao3_abedin(t).unwrap();
            (n(&m[0], 1.55), n(&m[1], 1.55))
        };
        let ((o0, e0), (o1, e1)) = (at(298.15), at(573.15));
        let (dno, dne) = ((o1 - o0) / 275.0, (e1 - e0) / 275.0);
        assert!((dno - 8.0e-6).abs() < 0.1e-6, "{dno}");
        assert!((dne + 2.7e-6).abs() < 0.1e-6, "{dne}");
        assert!(litao3_abedin(250.0).is_err() && litao3_abedin(600.0).is_err());
    }
}
