//! The catalogue's dispersion models, with their coefficients as the papers print them.
//!
//! Each model keeps its numbers twice: as `f64` constants the code evaluates, and as the strings
//! of the paper's table (for display, digit for digit). A test parses the strings back and checks
//! they are the constants, so the two can't drift apart.

use crate::material::{Material, Model, Provenance, SellmeierTerm};
use crate::units::Wavelength;
use crate::{Error, Result};

/// Electronvolts per inverse micrometre: Gehrsitz et al. express energies as 1/λ in µm⁻¹ and
/// "the energies expressed in eV just have to be reduced by a factor of 1/1.239 856".
pub(crate) const EV_UM: f64 = 1.239856;

fn um(value: f64) -> Result<Wavelength> {
    Wavelength::um(value)
}

/// Checks that a condition lies in its range.
pub(crate) fn check_range(what: &'static str, value: f64, min: f64, max: f64) -> Result<()> {
    if !value.is_finite() || value < min || value > max {
        return Err(Error::invalid(
            what,
            format!("{value} is outside the model's range {min} to {max}"),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Fused silica: Leviton & Frey 2006, Table 3
// ---------------------------------------------------------------------------------------------

/// Leviton & Frey's Table 3: rows are the T⁰ … T⁴ terms, columns S₁, S₂, S₃, λ₁, λ₂, λ₃ (µm).
pub(crate) const LEVITON: [[f64; 6]; 5] = [
    [
        1.10127E+00,
        1.78752E-05,
        7.93552E-01,
        -8.90600E-02,
        2.97562E-01,
        9.34454E+00,
    ],
    [
        -4.94251E-05,
        4.76391E-05,
        -1.27815E-03,
        9.08730E-06,
        -8.59578E-04,
        -7.09788E-03,
    ],
    [
        5.27414E-07,
        -4.49019E-07,
        1.84595E-05,
        -6.53638E-08,
        6.59069E-06,
        1.01968E-04,
    ],
    [
        -1.59700E-09,
        1.44546E-09,
        -9.20275E-08,
        7.77072E-11,
        -1.09482E-08,
        -5.07660E-07,
    ],
    [
        1.75949E-12,
        -1.57223E-12,
        1.48829E-10,
        6.84605E-14,
        7.85145E-13,
        8.21348E-10,
    ],
];

/// The same table as printed.
pub(crate) const LEVITON_PRINTED: [[&str; 6]; 5] = [
    [
        "1.10127E+00",
        "1.78752E-05",
        "7.93552E-01",
        "-8.90600E-02",
        "2.97562E-01",
        "9.34454E+00",
    ],
    [
        "-4.94251E-05",
        "4.76391E-05",
        "-1.27815E-03",
        "9.08730E-06",
        "-8.59578E-04",
        "-7.09788E-03",
    ],
    [
        "5.27414E-07",
        "-4.49019E-07",
        "1.84595E-05",
        "-6.53638E-08",
        "6.59069E-06",
        "1.01968E-04",
    ],
    [
        "-1.59700E-09",
        "1.44546E-09",
        "-9.20275E-08",
        "7.77072E-11",
        "-1.09482E-08",
        "-5.07660E-07",
    ],
    [
        "1.75949E-12",
        "-1.57223E-12",
        "1.48829E-10",
        "6.84605E-14",
        "7.85145E-13",
        "8.21348E-10",
    ],
];

/// Leviton & Frey's validity: 30 K ≤ T ≤ 300 K, 0.4 µm ≤ λ ≤ 2.6 µm (Table 3's heading).
pub(crate) const LEVITON_T: (f64, f64) = (30.0, 300.0);
pub(crate) const LEVITON_LAMBDA: (f64, f64) = (0.4, 2.6);

/// Corning 7980 fused silica at a temperature (kelvin), from Leviton & Frey's temperature-dependent
/// Sellmeier model: n²(λ, T) − 1 = Σᵢ Sᵢ(T) λ² / (λ² − λᵢ(T)²), Sᵢ and λᵢ quartic in T.
pub(crate) fn leviton_frey(temperature: f64) -> Result<Material> {
    check_range("temperature", temperature, LEVITON_T.0, LEVITON_T.1)?;
    let poly = |col: usize| {
        LEVITON
            .iter()
            .rev()
            .fold(0.0, |acc, row| acc * temperature + row[col])
    };
    let terms = (0..3)
        .map(|i| SellmeierTerm {
            b: poly(i),
            c: poly(i + 3).abs(),
        })
        .collect();
    Material::new(
        "SiO2 (Corning 7980)",
        Model::Sellmeier { a: 1.0, terms },
        um(LEVITON_LAMBDA.0)?,
        um(LEVITON_LAMBDA.1)?,
        Provenance {
            reference: "D. B. Leviton, B. J. Frey, Proc. SPIE 6273, 62732K (2006)".into(),
            doi: "10.1117/12.672853".into(),
            data: "our own transcription of Table 3 (arXiv:0805.0091), checked against Table 4".into(),
            temperature: Some(temperature),
            notes: "Corning 7980 synthetic fused silica; absolute (vacuum) index, measured by minimum deviation in CHARMS; uncertainty about 1e-5".into(),
        },
    )
}

// ---------------------------------------------------------------------------------------------
// Lithium niobate: Zelmon, Small & Jundt 1997, Tables 1 and 2
// ---------------------------------------------------------------------------------------------

/// A Zelmon table: A, B, C, D, E, F for one polarization (B, D, F in µm²).
pub(crate) type ZelmonColumn = [f64; 6];

/// Table 1, congruent LiNbO₃: (n_e, n_o).
pub(crate) const ZELMON_CONGRUENT: (ZelmonColumn, ZelmonColumn) = (
    [2.9804, 0.02047, 0.5981, 0.0666, 8.9543, 416.08],
    [2.6734, 0.01764, 1.2290, 0.05914, 12.614, 474.6],
);

/// Table 2, congruent LiNbO₃ doped with 5 mol% MgO: (n_e, n_o).
pub(crate) const ZELMON_MGO: (ZelmonColumn, ZelmonColumn) = (
    [2.4272, 0.01478, 1.4617, 0.05612, 9.6536, 371.216],
    [2.2454, 0.01242, 1.3005, 0.05313, 6.8972, 331.33],
);

/// The tables as printed, rows A … F, columns n_e, n_o.
pub(crate) const ZELMON_CONGRUENT_PRINTED: [[&str; 2]; 6] = [
    ["2.9804", "2.6734"],
    ["0.02047", "0.01764"],
    ["0.5981", "1.2290"],
    ["0.0666", "0.05914"],
    ["8.9543", "12.614"],
    ["416.08", "474.6"],
];
pub(crate) const ZELMON_MGO_PRINTED: [[&str; 2]; 6] = [
    ["2.4272", "2.2454"],
    ["0.01478", "0.01242"],
    ["1.4617", "1.3005"],
    ["0.05612", "0.05313"],
    ["9.6536", "6.8972"],
    ["371.216", "331.33"],
];

/// Zelmon's measurements: 0.4 to 5.0 µm, at 21 °C.
pub(crate) const ZELMON_LAMBDA: (f64, f64) = (0.4, 5.0);
pub(crate) const ZELMON_T: f64 = 294.15;

/// One polarization of Zelmon's three-oscillator Sellmeier formula,
/// n² − 1 = Aλ²/(λ² − B) + Cλ²/(λ² − D) + Eλ²/(λ² − F).
pub(crate) fn zelmon(name: &str, column: ZelmonColumn, table: &str) -> Result<Material> {
    let [a, b, c, d, e, f] = column;
    let terms = [(a, b), (c, d), (e, f)]
        .iter()
        .map(|&(strength, square)| SellmeierTerm {
            b: strength,
            c: square.sqrt(),
        })
        .collect();
    Material::new(
        name,
        Model::Sellmeier { a: 1.0, terms },
        um(ZELMON_LAMBDA.0)?,
        um(ZELMON_LAMBDA.1)?,
        Provenance {
            reference: "D. E. Zelmon, D. L. Small, D. Jundt, J. Opt. Soc. Am. B 14, 3319 (1997)"
                .into(),
            doi: "10.1364/JOSAB.14.003319".into(),
            data: format!("our own transcription of {table}, checked against the paper"),
            temperature: Some(ZELMON_T),
            notes: "minimum deviation on prisms, 0.4 to 5.0 um at 21 C; the fit is within 2e-4 of every measured index".into(),
        },
    )
}

// ---------------------------------------------------------------------------------------------
// AlxGa1-xAs: Gehrsitz, Reinhart, Gourgon, Herres, Vonlanthen & Sigg 2000
// ---------------------------------------------------------------------------------------------

/// Eq. (11), the direct gap of GaAs: E_Γ(0) = 1.5192 eV, E_Deb = 15.9 meV, E_TO = 33.6 meV,
/// S = 1.8, S_TO = 1.1, k_B = 0.086 1708 meV/K.
pub(crate) const GAP: [f64; 6] = [1.5192, 15.9e-3, 33.6e-3, 1.8, 1.1, 0.0861708e-3];

/// Table II, GaAs Fit 2, the temperature dependence: A = a₀ + a₁T + a₂T², E₁² = e₀ + e₁T + e₂T²
/// (the table prints a₁·10⁴·K, a₂·10⁶·K², and likewise for e).
pub(crate) const A0_T: [f64; 3] = [5.9613, 7.178e-4, -0.953e-6];
pub(crate) const E10_T: [f64; 3] = [4.7171, -3.237e-4, -1.358e-6];

/// Table IV: c₁ … c₅ of A, C₁, E₁², 1/C₀ and E₀ (c₀ is GaAs's, temperature-dependent for A,
/// E₁² and E₀; the constant c₀ of C₁ and 1/C₀ is given).
pub(crate) const TABLE_IV_A: [f64; 5] = [-16.159, 43.511, -71.317, 57.535, -17.451];
pub(crate) const TABLE_IV_C1: [f64; 6] = [21.5647, 113.74, -122.5, 108.401, -47.318, 0.0];
pub(crate) const TABLE_IV_E1: [f64; 2] = [11.006, -3.08];
pub(crate) const TABLE_IV_INV_C0: [f64; 6] = [50.535, -150.7, -62.209, 797.16, -1125.0, 503.79];
pub(crate) const TABLE_IV_E0: [f64; 2] = [1.1308, 0.1436];

/// Table II: the reststrahl terms, C₂ and E₂² of GaAs and of AlAs (the table prints 10³·C₂ and
/// 10³·E₂²); Eq. (13) weighs them by 1 − x and x.
pub(crate) const RESTSTRAHL_GAAS: (f64, f64) = (1.55e-3, 0.724e-3);
pub(crate) const RESTSTRAHL_ALAS: (f64, f64) = (2.61e-3, 1.331e-3);

/// Table IV as printed: rows c₀ … c₅, columns A, C₁, E₁², 1/C₀, E₀.
pub(crate) const TABLE_IV_PRINTED: [[&str; 5]; 6] = [
    [
        "$A_0(T)$",
        "21.5647",
        "$E_{10}^2(T)$",
        "50.535",
        r"$E_{\Gamma}(T)$ of GaAs",
    ],
    ["−16.159", "113.74", "11.006", "−150.7", "1.1308"],
    ["43.511", "−122.5", "−3.08", "−62.209", "0.1436"],
    ["−71.317", "108.401", "0", "797.16", "0"],
    ["57.535", "−47.318", "0", "−1125", "0"],
    ["−17.451", "0", "0", "503.79", "0"],
];

/// The model's ranges: the composition 0 ≤ x ≤ 1, wavelengths to 3 µm (the longest measured),
/// and temperatures from GaAs's lowest fitted temperature (103 K) to the highest measured for the
/// ternaries (40 °C).
pub(crate) const GEHRSITZ_X: (f64, f64) = (0.0, 1.0);
pub(crate) const GEHRSITZ_T: (f64, f64) = (103.0, 313.15);
pub(crate) const GEHRSITZ_LONGEST: f64 = 3.0;
/// The paper's agreement holds to 30 meV (0.024 µm⁻¹) below the direct gap.
pub(crate) const GEHRSITZ_GAP_MARGIN: f64 = 0.024;
/// The shortest wavelength measured on any sample (x = 0.865: λ > 0.47 µm).
pub(crate) const GEHRSITZ_SHORTEST: f64 = 0.47;

fn coth(v: f64) -> f64 {
    1.0 / v.tanh()
}

/// Eq. (11): the direct gap of GaAs at T, in µm⁻¹.
pub(crate) fn gap_gaas(t: f64) -> f64 {
    let [e0, deb, to, s, s_to, kb] = GAP;
    let ev = e0
        + s * deb * (1.0 - coth(deb / (2.0 * kb * t)))
        + s_to * to * (1.0 - coth(to / (2.0 * kb * t)));
    ev / EV_UM
}

fn quadratic(c: [f64; 3], t: f64) -> f64 {
    c[0] + c[1] * t + c[2] * t * t
}

/// x, x², … x⁵ dotted with the coefficients from c₁ on.
fn series(c: &[f64], x: f64) -> f64 {
    c.iter()
        .enumerate()
        .map(|(i, ci)| ci * x.powi(i as i32 + 1))
        .sum()
}

/// The parameters of Eq. (12) at a composition and temperature: (A, C₀, E₀, C₁, E₁²).
pub(crate) fn gehrsitz_parameters(x: f64, t: f64) -> (f64, f64, f64, f64, f64) {
    let a = quadratic(A0_T, t) + series(&TABLE_IV_A, x);
    let c1 = TABLE_IV_C1[0] + series(&TABLE_IV_C1[1..], x);
    let e1_sq = quadratic(E10_T, t) + series(&TABLE_IV_E1, x);
    let c0 = 1.0 / (TABLE_IV_INV_C0[0] + series(&TABLE_IV_INV_C0[1..], x));
    let e0 = gap_gaas(t) + series(&TABLE_IV_E0, x);
    (a, c0, e0, c1, e1_sq)
}

/// Eq. (12) with the oscillators written in λ: C/(E² − 1/λ²) = (C/E²)·λ²/(λ² − 1/E²).
fn oscillator(c: f64, e_sq: f64) -> SellmeierTerm {
    SellmeierTerm {
        b: c / e_sq,
        c: 1.0 / e_sq.sqrt(),
    }
}

/// AlₓGa₁₋ₓAs at a composition and temperature (kelvin), from Gehrsitz et al.'s Eqs. (11)–(13)
/// and Table IV.
pub(crate) fn gehrsitz(x: f64, t: f64) -> Result<Material> {
    check_range("composition", x, GEHRSITZ_X.0, GEHRSITZ_X.1)?;
    check_range("temperature", t, GEHRSITZ_T.0, GEHRSITZ_T.1)?;
    let (a, c0, e0, c1, e1_sq) = gehrsitz_parameters(x, t);
    let (c2, e2_sq) = RESTSTRAHL_GAAS;
    let (c3, e3_sq) = RESTSTRAHL_ALAS;
    let terms = vec![
        oscillator(c0, e0 * e0),
        oscillator(c1, e1_sq),
        oscillator((1.0 - x) * c2, e2_sq),
        oscillator(x * c3, e3_sq),
    ];
    let shortest = (1.0 / (e0 - GEHRSITZ_GAP_MARGIN)).max(GEHRSITZ_SHORTEST);
    let name = if x == 0.0 {
        "GaAs".to_owned()
    } else {
        format!("Al{}Ga{}As", short(x), short(1.0 - x))
    };
    Material::new(
        name,
        Model::Sellmeier { a, terms },
        um(shortest)?,
        um(GEHRSITZ_LONGEST)?,
        Provenance {
            reference: "S. Gehrsitz et al., J. Appl. Phys. 87, 7825 (2000)".into(),
            doi: "10.1063/1.373462".into(),
            data: "our own transcription of Eqs. (11)-(13), Table II and Table IV, checked against the paper".into(),
            temperature: Some(t),
            notes: format!(
                "AlxGa1-xAs with x = {x}, below the direct gap ({:.4} um) to 3 um; fit to the measured data with sigma <= 2.5e-3 (9.9e-3 at x = 0.427)",
                1.0 / e0
            ),
        },
    )
}

/// Table III: one sample's own fit (x; A, C₁, E₁², C₀, E₀²; 10³·Δn_max), at 23 °C.
pub(crate) const TABLE_III: [(f64, [f64; 5], f64); 9] = [
    (0.0, [7.4704, 12.36151, 3.581, 0.0154, 1.323], 0.9),
    (0.176, [3.1739, 53.3589, 7.486, 0.0622, 1.855], 2.5),
    (0.334, [3.3646, 50.37298, 7.835, 0.0912, 2.383], 0.3),
    (0.410, [4.5043, 36.7799, 7.151, 0.0677, 2.626], 3.1),
    (0.427, [4.2524, 38.5382, 7.227, 0.0552, 2.607], 2.9),
    (0.615, [2.9993, 59.0641, 9.791, 0.2833, 3.600], 2.4),
    (0.753, [2.5234, 67.5218, 10.975, 0.3460, 4.206], 2.5),
    (0.865, [1.6989, 82.5887, 12.401, 0.3759, 5.066], 2.9),
    (1.0, [2.2863, 72.92714, 12.507, 0.3315, 5.878], 2.3),
];

/// Table IV's quality of fit: x and 10³·Δn_max of the analytic model against the measured data.
pub(crate) const TABLE_IV_QUALITY: [(f64, f64); 9] = [
    (0.0, 1.6),
    (0.176, 4.4),
    (0.334, 0.9),
    (0.410, 9.5),
    (0.427, 47.0),
    (0.615, 6.1),
    (0.753, 3.6),
    (0.865, 8.1),
    (1.0, 2.5),
];

/// n of one Table III sample at λ (µm), from Eq. (12) with its own parameters and the
/// reststrahl term of Eq. (13).
pub(crate) fn table_iii_index(sample: &(f64, [f64; 5], f64), lam: f64) -> f64 {
    let (x, [a, c1, e1_sq, c0, e0_sq], _) = *sample;
    let e_sq = 1.0 / (lam * lam);
    let (c2, e2_sq) = RESTSTRAHL_GAAS;
    let (c3, e3_sq) = RESTSTRAHL_ALAS;
    let n_sq = a
        + c0 / (e0_sq - e_sq)
        + c1 / (e1_sq - e_sq)
        + (1.0 - x) * c2 / (e2_sq - e_sq)
        + x * c3 / (e3_sq - e_sq);
    n_sq.sqrt()
}

// ---------------------------------------------------------------------------------------------
// Lithium niobate with temperature: Jundt 1997 (congruent, n_e) and Gayer et al. 2008 (5% MgO)
// ---------------------------------------------------------------------------------------------

/// The coefficients of the Jundt form, a₁ … a₆ and b₁ … b₄.
pub(crate) type JundtForm = ([f64; 6], [f64; 4]);

/// Jundt's Table 2, congruent LiNbO₃, n_e (a₄ = 100 µm² held fixed).
pub(crate) const JUNDT: JundtForm = (
    [5.35583, 0.100473, 0.20692, 100.0, 11.34927, 1.5334e-2],
    [4.629e-7, 3.862e-8, -0.89e-8, 2.657e-5],
);

/// Gayer et al.'s Table 1 as corrected by the 2010 erratum, 5% MgO-doped congruent LiNbO₃:
/// n_e and n_o. (The erratum changes only b₁ of the 1% MgO stoichiometric column, which the
/// catalogue doesn't use; these two columns are the same in the original.)
pub(crate) const GAYER_E: JundtForm = (
    [5.756, 0.0983, 0.2020, 189.32, 12.52, 1.32e-2],
    [2.860e-6, 4.700e-8, 6.113e-8, 1.516e-4],
);
pub(crate) const GAYER_O: JundtForm = (
    [5.653, 0.1185, 0.2091, 89.61, 10.85, 1.97e-2],
    [7.941e-7, 3.134e-8, -4.641e-9, -2.188e-6],
);

/// The temperature function of Jundt's Eq. (5) and Gayer's Eq. (3), T in °C:
/// f = (T − 24.5)(T + 570.82).
pub(crate) fn jundt_f(celsius: f64) -> f64 {
    (celsius - 24.5) * (celsius + 570.82)
}

/// n² = a₁ + b₁f + (a₂ + b₂f)/(λ² − (a₃ + b₃f)²) + (a₄ + b₄f)/(λ² − a₅²) − a₆λ², as the
/// refractiveindex.info formula 4 (C₁ + C₂λ^C₃/(λ² − C₄^C₅) + C₆λ^C₇/(λ² − C₈^C₉) + C₁₀λ^C₁₁).
pub(crate) fn jundt_model(form: JundtForm, celsius: f64) -> Model {
    let ([a1, a2, a3, a4, a5, a6], [b1, b2, b3, b4]) = form;
    let f = jundt_f(celsius);
    Model::Formula {
        number: 4,
        coefficients: vec![
            a1 + b1 * f,
            a2 + b2 * f,
            0.0,
            a3 + b3 * f,
            2.0,
            a4 + b4 * f,
            0.0,
            a5,
            2.0,
            -a6,
            2.0,
        ],
    }
}

/// Jundt's validity: 0.4 to 5 µm, room temperature (20 °C) to 250 °C.
pub(crate) const JUNDT_LAMBDA: (f64, f64) = (0.4, 5.0);
pub(crate) const JUNDT_T: (f64, f64) = (293.15, 523.15);
/// Gayer's validity: n_e 0.5 to 4 µm and 20 to 200 °C; n_o "reliable only for wavelengths up
/// to 1620 nm and roughly 20–100 °C".
pub(crate) const GAYER_E_LAMBDA: (f64, f64) = (0.5, 4.0);
pub(crate) const GAYER_E_T: (f64, f64) = (293.15, 473.15);
pub(crate) const GAYER_O_LAMBDA: (f64, f64) = (0.5, 1.62);
pub(crate) const GAYER_O_T: (f64, f64) = (293.15, 373.15);

/// Congruent LiNbO₃'s n_e at a temperature (K), from Jundt 1997.
pub(crate) fn jundt(kelvin: f64) -> Result<Material> {
    check_range("temperature", kelvin, JUNDT_T.0, JUNDT_T.1)?;
    Material::new(
        "LiNbO3 (e)",
        jundt_model(JUNDT, kelvin - 273.15),
        um(JUNDT_LAMBDA.0)?,
        um(JUNDT_LAMBDA.1)?,
        Provenance {
            reference: "D. H. Jundt, Opt. Lett. 22, 1553 (1997)".into(),
            doi: "10.1364/OL.22.001553".into(),
            data: "our own transcription of Eqs. (4)-(5) and Table 2, checked against the paper".into(),
            temperature: Some(kelvin),
            notes: "congruent LiNbO3 (48.38 mol% Li2O), extraordinary index only; fitted to index data and PPLN OPO tuning".into(),
        },
    )
}

/// 5% MgO-doped congruent LiNbO₃ at a temperature (K), from Gayer et al. 2008: n_e
/// (`extraordinary`) or n_o.
pub(crate) fn gayer(extraordinary: bool, kelvin: f64) -> Result<Material> {
    let (form, lam, t, name) = if extraordinary {
        (GAYER_E, GAYER_E_LAMBDA, GAYER_E_T, "MgO:LiNbO3 (e)")
    } else {
        (GAYER_O, GAYER_O_LAMBDA, GAYER_O_T, "MgO:LiNbO3 (o)")
    };
    check_range("temperature", kelvin, t.0, t.1)?;
    Material::new(
        name,
        jundt_model(form, kelvin - 273.15),
        um(lam.0)?,
        um(lam.1)?,
        Provenance {
            reference: "O. Gayer, Z. Sacks, E. Galun, A. Arie, Appl. Phys. B 91, 343 (2008)".into(),
            doi: "10.1007/s00340-008-2998-2".into(),
            data: "our own transcription of Eqs. (2)-(3) and Table 1 as corrected by the erratum (Appl. Phys. B 101, 481 (2010))".into(),
            temperature: Some(kelvin),
            notes: "5 mol% MgO-doped congruent LiNbO3; fitted to QPM SHG and OPO tuning".into(),
        },
    )
}

// ---------------------------------------------------------------------------------------------
// GaAs: Skauli et al. 2003, the Pikhtin form (Eq. 12, Table II)
// ---------------------------------------------------------------------------------------------

/// Skauli et al.'s Table II, the Pikhtin form: Eᵢ = constant + linear·ΔT (+ quadratic·ΔT² for
/// E₀), ΔT = T − 22 °C, energies in eV; then ⟨ε₂⟩, G₃ (eV²) and A.
pub(crate) const SKAULI_E: [[f64; 3]; 4] = [
    [1.425000, -0.00037164, -7.497e-7],
    [2.400356, -0.00051458, 0.0],
    [7.691979, -0.00046545, 0.0],
    [0.034303, 0.00001136, 0.0],
];
pub(crate) const SKAULI_EPS2: f64 = 12.99386;
pub(crate) const SKAULI_G3: f64 = 0.00218176;
pub(crate) const SKAULI_A: f64 = 0.689578;
/// The reference temperature of ΔT, 22 °C.
pub(crate) const SKAULI_T0: f64 = 295.15;
/// Measured from 0.97 to 17 µm; temperatures from 22 to 95 °C (the paper checks its fits
/// against QPM data at 21 °C too, Table I).
pub(crate) const SKAULI_LAMBDA: (f64, f64) = (0.97, 17.0);
pub(crate) const SKAULI_T: (f64, f64) = (294.15, 368.15);

/// GaAs at a temperature (K), from Skauli et al.'s recommended (Pikhtin) form.
pub(crate) fn skauli(kelvin: f64) -> Result<Material> {
    check_range("temperature", kelvin, SKAULI_T.0, SKAULI_T.1)?;
    let dt = kelvin - SKAULI_T0;
    let energies = SKAULI_E.map(|[c, l, q]| c + l * dt + q * dt * dt);
    Material::new(
        "GaAs",
        Model::Pikhtin {
            a: SKAULI_A,
            eps2: SKAULI_EPS2,
            g3: SKAULI_G3,
            energies,
        },
        um(SKAULI_LAMBDA.0)?,
        um(SKAULI_LAMBDA.1)?,
        Provenance {
            reference: "T. Skauli et al., J. Appl. Phys. 94, 6447 (2003)".into(),
            doi: "10.1063/1.1621740".into(),
            data: "our own transcription of Eq. (12) and Table II (the Pikhtin form), checked against the paper".into(),
            temperature: Some(kelvin),
            notes: "undoped GaAs; etalon fringes (FTIR) and QPM SHG; n to +-0.2% (systematic), dn/dT to +-1.4%".into(),
        },
    )
}

/// The QPM period of first-order SHG, Λ = λ_ω / (2(n_2ω − n_ω)), in µm, for a fundamental λ_ω
/// (µm): Skauli et al.'s Eqs. (1)–(2).
pub(crate) fn shg_period(material: &Material, fundamental: f64) -> Result<f64> {
    let n = |lam: f64| -> Result<f64> { Ok(material.refractive_index(um(lam)?)?.re) };
    Ok(fundamental / (2.0 * (n(fundamental / 2.0)? - n(fundamental)?)))
}

// ---------------------------------------------------------------------------------------------
// AlGaAs: Afromowitz 1974
// ---------------------------------------------------------------------------------------------

/// Afromowitz's appendix for Ga₁₋ₓAlₓAs (eV): E₀ = 3.65 + 0.871x + 0.179x²,
/// E_d = 36.1 − 2.45x, E_Γ = 1.424 + 1.266x + 0.26x².
pub(crate) const AFROMOWITZ_E0: [f64; 3] = [3.65, 0.871, 0.179];
pub(crate) const AFROMOWITZ_ED: [f64; 2] = [36.1, -2.45];
pub(crate) const AFROMOWITZ_GAP: [f64; 3] = [1.424, 1.266, 0.26];
/// GaAs was checked from 0.895 µm (0.039 eV below E_Γ) to 1.7 µm; AlAs to 2 eV.
pub(crate) const AFROMOWITZ_LONGEST: f64 = 1.7;
pub(crate) const AFROMOWITZ_MARGIN_EV: f64 = 0.039;
pub(crate) const AFROMOWITZ_HIGHEST_EV: f64 = 2.0;

/// (E₀, E_d, E_Γ) at a composition, in eV.
pub(crate) fn afromowitz_parameters(x: f64) -> (f64, f64, f64) {
    let q = |c: [f64; 3]| c[0] + c[1] * x + c[2] * x * x;
    (
        q(AFROMOWITZ_E0),
        AFROMOWITZ_ED[0] + AFROMOWITZ_ED[1] * x,
        q(AFROMOWITZ_GAP),
    )
}

/// E_f and η of Eqs. (10)–(11), which the paper prints for GaAs and AlAs.
pub(crate) fn afromowitz_constants(e0: f64, ed: f64, gap: f64) -> (f64, f64) {
    let ef = (2.0 * e0 * e0 - gap * gap).sqrt();
    let eta = std::f64::consts::PI * ed / (2.0 * e0.powi(3) * (e0 * e0 - gap * gap));
    (ef, eta)
}

/// AlₓGa₁₋ₓAs at room temperature from Afromowitz's model.
pub(crate) fn afromowitz(x: f64) -> Result<Material> {
    check_range("composition", x, 0.0, 1.0)?;
    let (e0, ed, gap) = afromowitz_parameters(x);
    let hc = crate::material::PHOTON_EV_UM;
    let shortest = (hc / (gap - AFROMOWITZ_MARGIN_EV)).max(hc / AFROMOWITZ_HIGHEST_EV);
    Material::new(
        format!("Al{}Ga{}As", short(x), short(1.0 - x)),
        Model::Afromowitz { e0, ed, gap },
        um(shortest)?,
        um(AFROMOWITZ_LONGEST)?,
        Provenance {
            reference: "M. A. Afromowitz, Solid State Commun. 15, 59 (1974)".into(),
            doi: "10.1016/0038-1098(74)90014-3".into(),
            data: "our own transcription of Eqs. (7)-(12) and the appendix, checked against the paper".into(),
            temperature: None,
            notes: "room temperature; within 0.004 of GaAs data from 0.895 to 1.7 um, of AlAs to 1.5 eV (0.014 to 2 eV); alloys compared up to x = 0.38".into(),
        },
    )
}

/// A composition for a name: up to four decimals, trailing zeros dropped.
pub(crate) fn short(x: f64) -> String {
    let s = format!("{x:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

// ---------------------------------------------------------------------------------------------
// InGaP: Tanaka, Kawamura & Asahi 1986 (single effective oscillator)
// ---------------------------------------------------------------------------------------------

/// Tanaka et al.'s Eqs. (3)–(4) for In₀.₄₉Ga₀.₅₁₋ₓAlₓP (eV): E₀ = 3.39 + 1.216x,
/// E_d = 28.07 + 3.373x; x = 0 is lattice-matched In₀.₄₉Ga₀.₅₁P.
pub(crate) const TANAKA_E0: [f64; 2] = [3.39, 1.216];
pub(crate) const TANAKA_ED: [f64; 2] = [28.07, 3.373];
/// Al content 0 to 0.51; measured from 0.6 to 1.3 eV at room temperature.
pub(crate) const TANAKA_X: (f64, f64) = (0.0, 0.51);
pub(crate) const TANAKA_EV: (f64, f64) = (0.6, 1.3);

/// In₀.₄₉Ga₀.₅₁₋ₓAlₓP from Tanaka et al.'s Eq. (2), n² − 1 = E₀E_d/(E₀² − E²), as a Sellmeier
/// term: (E_d/E₀)·λ²/(λ² − (hc/E₀)²).
pub(crate) fn tanaka(x: f64) -> Result<Material> {
    check_range("composition", x, TANAKA_X.0, TANAKA_X.1)?;
    let hc = crate::material::PHOTON_EV_UM;
    let e0 = TANAKA_E0[0] + TANAKA_E0[1] * x;
    let ed = TANAKA_ED[0] + TANAKA_ED[1] * x;
    let name = if x == 0.0 {
        "In0.49Ga0.51P".to_owned()
    } else {
        format!("In0.49Ga{}Al{}P", short(0.51 - x), short(x))
    };
    Material::new(
        name,
        Model::Sellmeier {
            a: 1.0,
            terms: vec![SellmeierTerm {
                b: ed / e0,
                c: hc / e0,
            }],
        },
        um(hc / TANAKA_EV.1)?,
        um(hc / TANAKA_EV.0)?,
        Provenance {
            reference: "H. Tanaka, Y. Kawamura, H. Asahi, J. Appl. Phys. 59, 985 (1986)".into(),
            doi: "10.1063/1.336581".into(),
            data: "our own transcription of Eqs. (2)-(4), checked against the paper".into(),
            temperature: Some(300.0),
            notes: "MBE layers lattice matched to GaAs; reflectance minima from 0.6 to 1.3 eV at room temperature; no accuracy stated".into(),
        },
    )
}

// ---------------------------------------------------------------------------------------------
// InP: Pettit & Turner 1965
// ---------------------------------------------------------------------------------------------

/// Pettit & Turner's Table I: (T in K, A, B, C² in Å²) for n² = A + Bλ²/(λ² − C²), λ in Å; and
/// the band edge of their Table II (E_c, eV) where the measurements end.
pub(crate) const PETTIT_TURNER: [(f64, f64, f64, f64, f64); 2] = [
    (298.0, 7.255, 2.316, 0.3922e8, 1.340),
    (77.0, 7.781, 1.661, 0.4397e8, 1.407),
];
/// Measured from 0.60 eV.
pub(crate) const PETTIT_TURNER_LOWEST_EV: f64 = 0.60;

/// InP at 298 K or 77 K, from Pettit & Turner's Eq. (1) and Table I.
pub(crate) fn pettit_turner(kelvin: f64) -> Result<Material> {
    let &(t, a, b, c_sq, gap) = PETTIT_TURNER
        .iter()
        .find(|p| p.0 == kelvin)
        .ok_or_else(|| {
            Error::invalid(
                "temperature",
                format!("{kelvin} K: the paper fits 298 K and 77 K only"),
            )
        })?;
    let hc = crate::material::PHOTON_EV_UM;
    Material::new(
        "InP",
        Model::Sellmeier {
            a,
            terms: vec![SellmeierTerm {
                b,
                // Å² to µm²
                c: (c_sq * 1e-8).sqrt(),
            }],
        },
        um(hc / gap)?,
        um(hc / PETTIT_TURNER_LOWEST_EV)?,
        Provenance {
            reference: "G. D. Pettit, W. J. Turner, J. Appl. Phys. 36, 2081 (1965)".into(),
            doi: "10.1063/1.1714410".into(),
            data: "our own transcription of Eq. (1) and Table I, checked against the paper".into(),
            temperature: Some(t),
            notes: "minimum deviation on a prism, n-type 5e16 cm^-3, from 0.60 eV to the gap; the fit is within 0.007 of the measurements".into(),
        },
    )
}
