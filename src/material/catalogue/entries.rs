//! The catalogue's entries.

use super::models::{self, ZelmonColumn};
use super::references::references;
use super::symmetry::cells;
use super::{
    Axis, Category, Clamping, CoefficientTable, Constant, Crystal, CrystalSystem, Entry,
    IndexModel, Missing, OpticalClass, Parameter, Source, Tensor, TensorKind,
};
use crate::Result;
use crate::material::{self, Material};

pub(super) fn src(reference: &str, location: &str) -> Source {
    Source {
        reference: reference.into(),
        location: location.into(),
    }
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

fn table(caption: &str, columns: &[&str], rows: Vec<Vec<String>>) -> CoefficientTable {
    CoefficientTable {
        caption: caption.into(),
        columns: strings(columns),
        rows,
    }
}

fn temperature(min: f64, max: f64, default: f64) -> Option<Parameter> {
    Some(Parameter {
        symbol: "T".into(),
        name: "temperature".into(),
        unit: "K".into(),
        min,
        max,
        default,
    })
}

fn missing(property: &str, reason: &str) -> Missing {
    Missing {
        property: property.into(),
        reason: reason.into(),
    }
}

const D_CONVENTION: &str =
    "d = χ⁽²⁾/2 in pm/V, Voigt notation (jk = 11, 22, 33, 23, 13, 12 → l = 1 … 6)";
const R_CONVENTION: &str =
    "Δ(1/n²)_i = Σ_k r_ik E_k in pm/V, Voigt notation (i = 1 … 6 for 11, 22, 33, 23, 13, 12)";

/// A tensor that symmetry makes zero.
fn zero_tensor(kind: TensorKind, point_group: &str, why: &str) -> Tensor {
    Tensor {
        kind,
        label: "zero by symmetry".into(),
        point_group: point_group.into(),
        cells: cells(kind, point_group, &[]),
        wavelength: None,
        clamping: Clamping::None,
        convention: match kind {
            TensorKind::SecondOrder => D_CONVENTION.into(),
            TensorKind::ElectroOptic => R_CONVENTION.into(),
        },
        source: None,
        notes: why.into(),
    }
}

/// The isotropic, centrosymmetric tensors of a glass or of silicon.
fn centrosymmetric(point_group: &str, why: &str) -> Vec<Tensor> {
    vec![
        zero_tensor(TensorKind::SecondOrder, point_group, why),
        zero_tensor(TensorKind::ElectroOptic, point_group, why),
    ]
}

pub(super) fn all() -> Vec<Entry> {
    vec![
        silica(),
        silicon_nitride(),
        silicon(),
        gaas(),
        algaas(),
        ingap(),
        inp(),
        aln(),
        lithium_niobate(),
        lithium_niobate_mgo(),
    ]
}

// ---------------------------------------------------------------------------------------------
// Dielectrics
// ---------------------------------------------------------------------------------------------

fn amorphous(notes: &str) -> Crystal {
    Crystal {
        system: CrystalSystem::Amorphous,
        point_group: "∞∞m".into(),
        space_group: None,
        structure: "amorphous".into(),
        optical: OpticalClass::Isotropic,
        centrosymmetric: true,
        notes: notes.into(),
    }
}

/// The range of a built-in model's material at its default conditions, in µm.
fn nominal(m: Result<Material>) -> (f64, f64) {
    m.map_or((f64::NAN, f64::NAN), |m| {
        let (lo, hi) = m.range();
        (lo.to_um(), hi.to_um())
    })
}

fn one(m: Result<Material>) -> Result<Vec<Material>> {
    m.map(|m| vec![m])
}

fn silica() -> Entry {
    let malitson = material::silica();
    let sellmeier_rows = vec![
        strings(&["1", "0.6961663", "0.0684043"]),
        strings(&["2", "0.4079426", "0.1162414"]),
        strings(&["3", "0.8974794", "9.896161"]),
    ];
    let leviton_rows = ["constant", "$T$", "$T^2$", "$T^3$", "$T^4$"]
        .iter()
        .zip(models::LEVITON_PRINTED)
        .map(|(label, row)| {
            let mut r = vec![(*label).to_owned()];
            r.extend(row.iter().map(|s| (*s).to_owned()));
            r
        })
        .collect();
    Entry {
        id: "sio2".into(),
        name: "Fused silica".into(),
        formula: "SiO₂".into(),
        category: Category::Dielectric,
        summary: "The cladding and buried oxide of silicon photonics; a glass, isotropic and centrosymmetric.".into(),
        crystal: amorphous("thermal and deposited oxides differ from fused silica by up to about 1e-2"),
        index: vec![
            IndexModel {
                id: "malitson-1965".into(),
                name: "Malitson 1965".into(),
                axes: vec![Axis::Isotropic],
                equation: r"n^2 - 1 = \sum_{i=1}^{3} \frac{A_i\,\lambda^2}{\lambda^2 - \lambda_i^2}".into(),
                symbols: r"$\lambda$ in µm, in air; $A_i$ and $\lambda_i$ (µm) from Eq. (1)".into(),
                coefficients: vec![table(
                    "Eq. (1), fused silica at 20 °C",
                    &["$i$", "$A_i$", r"$\lambda_i$ (µm)"],
                    sellmeier_rows,
                )],
                wavelength: (0.21, 3.71),
                temperature: None,
                composition: None,
                accuracy: "interpolates the 60 measured wavelengths to 1.05e-5 on average (five decimals)".into(),
                sources: vec![src("malitson-1965", "Eq. (1), Table I")],
                default: true,
                notes: format!(
                    "20 °C (Provenance: {} K); the index relative to air",
                    malitson.provenance().temperature.unwrap_or_default()
                ),
                eval: |_, _| Ok(vec![material::silica()]),
            },
            IndexModel {
                id: "leviton-frey-2006".into(),
                name: "Leviton & Frey 2006".into(),
                axes: vec![Axis::Isotropic],
                equation: r"n^2(\lambda, T) - 1 = \sum_{i=1}^{3} \frac{S_i(T)\,\lambda^2}{\lambda^2 - \lambda_i^2(T)}, \quad S_i(T) = \sum_{j=0}^{4} S_{ij}\,T^j, \quad \lambda_i(T) = \sum_{j=0}^{4} \lambda_{ij}\,T^j".into(),
                symbols: r"$\lambda$ in µm, $T$ in K; the absolute (vacuum) index of Corning 7980".into(),
                coefficients: vec![table(
                    "Table 3, Corning 7980 fused silica",
                    &["term", "$S_1$", "$S_2$", "$S_3$", r"$\lambda_1$", r"$\lambda_2$", r"$\lambda_3$"],
                    leviton_rows,
                )],
                wavelength: models::LEVITON_LAMBDA,
                temperature: temperature(models::LEVITON_T.0, models::LEVITON_T.1, 295.0),
                composition: None,
                accuracy: "the measurements' uncertainty: 0.7e-5 to 1.8e-5 (Table 1); the fit's residuals below 2e-6 rms".into(),
                sources: vec![src("leviton-frey-2006", "Table 3 (Table 4 tabulates it)")],
                default: false,
                notes: "cryogenic to room temperature; agrees with Malitson's formula (corrected to vacuum) to better than 1e-5 below 2 µm, per the paper".into(),
                eval: |t, _| one(models::leviton_frey(t)),
            },
        ],
        tensors: centrosymmetric("∞∞m", "a glass has a centre of inversion: χ⁽²⁾ and r vanish"),
        constants: vec![],
        missing: vec![],
        references: references(&["malitson-1965", "leviton-frey-2006"]),
    }
}

fn silicon_nitride() -> Entry {
    Entry {
        id: "si3n4".into(),
        name: "Silicon nitride (LPCVD)".into(),
        formula: "Si₃N₄".into(),
        category: Category::Dielectric,
        summary: "Stoichiometric LPCVD nitride, the low-loss waveguide core of nitride photonics.".into(),
        crystal: amorphous("a deposited film; its index depends on the deposition"),
        index: vec![IndexModel {
            id: "luke-2015".into(),
            name: "Luke et al. 2015".into(),
            axes: vec![Axis::Isotropic],
            equation: r"n^2 = 1 + \frac{3.0249\,\lambda^2}{\lambda^2 - 135.3406^2} + \frac{40314\,\lambda^2}{\lambda^2 - 1239842^2}".into(),
            symbols: r"$\lambda$ in nm, as the paper writes it".into(),
            coefficients: vec![table(
                "Eq. (1)",
                &["term", "strength", "resonance (nm)"],
                vec![strings(&["1", "3.0249", "135.3406"]), strings(&["2", "40314", "1239842"])],
            )],
            wavelength: (0.31, 5.504),
            temperature: None,
            composition: None,
            accuracy: "a fit to ellipsometry from 193 nm to 33 µm; the paper states no uncertainty".into(),
            sources: vec![src("luke-2015", "Eq. (1)")],
            default: true,
            notes: "a 340 nm film annealed at 1200 °C, on thermal oxide; no temperature stated".into(),
            eval: |_, _| Ok(vec![material::silicon_nitride()]),
        }],
        tensors: centrosymmetric(
            "∞∞m",
            "an isotropic amorphous film: χ⁽²⁾ and r vanish by symmetry (a χ⁽²⁾ induced by poling or interfaces is not a material constant)",
        ),
        constants: vec![],
        missing: vec![],
        references: references(&["luke-2015"]),
    }
}

fn silicon() -> Entry {
    Entry {
        id: "si".into(),
        name: "Silicon".into(),
        formula: "Si".into(),
        category: Category::Semiconductor,
        summary: "Crystalline silicon, the core of silicon photonics; centrosymmetric, so it has no bulk χ⁽²⁾ or Pockels effect.".into(),
        crystal: Crystal {
            system: CrystalSystem::Cubic,
            point_group: "m-3m".into(),
            space_group: Some("Fd-3m (No. 227)".into()),
            structure: "diamond".into(),
            optical: OpticalClass::Isotropic,
            centrosymmetric: true,
            notes: "strain breaks the inversion symmetry and gives a small Pockels effect, which depends on the device".into(),
        },
        index: vec![IndexModel {
            id: "li-1980".into(),
            name: "Li 1980".into(),
            axes: vec![Axis::Isotropic],
            equation: r"n(\lambda) = \text{Table 1 (293 K), through a natural cubic spline}".into(),
            symbols: r"$\lambda$ in µm".into(),
            coefficients: vec![],
            wavelength: (1.2, 14.0),
            temperature: None,
            composition: None,
            accuracy: "±2e-4 (Li's stated uncertainty)".into(),
            sources: vec![src("li-1980", "Table 1, 293 K")],
            default: true,
            notes: "293 K; 35 recommended values".into(),
            eval: |_, _| Ok(vec![material::silicon()]),
        }],
        tensors: centrosymmetric("m-3m", "diamond structure has a centre of inversion: χ⁽²⁾ and r vanish"),
        constants: vec![],
        missing: vec![],
        references: references(&["li-1980"]),
    }
}

// ---------------------------------------------------------------------------------------------
// III-V semiconductors
// ---------------------------------------------------------------------------------------------

fn zincblende(notes: &str) -> Crystal {
    Crystal {
        system: CrystalSystem::Cubic,
        point_group: "-43m".into(),
        space_group: Some("F-43m (No. 216)".into()),
        structure: "zincblende".into(),
        optical: OpticalClass::Isotropic,
        centrosymmetric: false,
        notes: notes.into(),
    }
}

/// Gehrsitz et al.'s model, as an index model; `x` fixed for GaAs.
fn gehrsitz_model(fixed_gaas: bool) -> IndexModel {
    let table_iv = models::TABLE_IV_PRINTED
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut r = vec![format!("$c_{i}$")];
            r.extend(row.iter().map(|s| (*s).to_owned()));
            r
        })
        .collect();
    let table_ii = vec![
        strings(&["$A$", "5.9613", "7.178", "−0.953"]),
        strings(&["$E_1^2$ (µm⁻²)", "4.7171", "−3.237", "−1.358"]),
    ];
    let gap = vec![
        strings(&[r"$E_\Gamma(0)$", "1.5192 eV"]),
        strings(&[r"$E_\mathrm{Deb}$", "15.9 meV"]),
        strings(&[r"$E_\mathrm{TO}$", "33.6 meV"]),
        strings(&["$S$", "1.8"]),
        strings(&[r"$S_\mathrm{TO}$", "1.1"]),
    ];
    let reststrahl = vec![
        strings(&["GaAs", "1.55", "0.724"]),
        strings(&["AlAs", "2.61", "1.331"]),
    ];
    let mut coefficients = vec![];
    if !fixed_gaas {
        coefficients.push(table(
            "Table IV: c(x, T) = c₀(T) + c₁x + … + c₅x⁵",
            &[
                "",
                "$A$",
                r"$C_1$ (µm⁻²)",
                r"$E_1^2$ (µm⁻²)",
                r"$1/C_0$ (µm²)",
                r"$E_0$ (µm⁻¹)",
            ],
            table_iv,
        ));
    } else {
        coefficients.push(table(
            "Table IV at x = 0 (Table II, GaAs Fit 2)",
            &["", "value"],
            vec![
                strings(&["$C_1$ (µm⁻²)", "21.5647"]),
                strings(&["$1/C_0$ (µm²)", "50.535"]),
            ],
        ));
    }
    coefficients.push(table(
        "Table II, GaAs Fit 2: A₀(T) = a₀ + a₁T + a₂T², E₁₀²(T) = e₀ + e₁T + e₂T²",
        &[
            "",
            "$a_0$, $e_0$",
            r"$a_1, e_1$ ($\times 10^{-4}$ K⁻¹)",
            r"$a_2, e_2$ ($\times 10^{-6}$ K⁻²)",
        ],
        table_ii,
    ));
    coefficients.push(table(
        "Eq. (11), the direct gap of GaAs",
        &["", "value"],
        gap,
    ));
    coefficients.push(table(
        "Table II, the reststrahl terms of Eq. (13)",
        &["", r"$10^3 C_2$ (µm⁻²)", r"$10^3 E_2^2$ (µm⁻²)"],
        reststrahl,
    ));
    IndexModel {
        id: if fixed_gaas { "gehrsitz-2000-gaas" } else { "gehrsitz-2000" }.into(),
        name: "Gehrsitz et al. 2000".into(),
        axes: vec![Axis::Isotropic],
        equation: r"n^2 = A + \frac{C_0}{E_0^2 - E^2} + \frac{C_1}{E_1^2 - E^2} + (1-x)\frac{C_2}{E_2^2 - E^2} + x\,\frac{C_3}{E_3^2 - E^2}, \quad E = \frac{1}{\lambda}, \quad E_0 = E_\Gamma".into(),
        symbols: r"$E$ in µm⁻¹ ($E$/eV $= 1.239856\,E$/µm⁻¹); $A$, $C_1$, $E_1^2$, $1/C_0$, $E_0$ are polynomials in $x$ (Eq. (16), Table IV) whose constant terms follow GaAs's temperature dependence: $A_0(T)$, $E_{10}^2(T)$ (Table II) and $E_{\Gamma}(T) = E_\Gamma(0) + S E_\mathrm{Deb}[1 - \coth(E_\mathrm{Deb}/2k_BT)] + S_\mathrm{TO} E_\mathrm{TO}[1 - \coth(E_\mathrm{TO}/2k_BT)]$ (Eq. (11)); the last two terms are the GaAs- and AlAs-like reststrahl bands (Eq. (13))".into(),
        coefficients,
        wavelength: nominal(models::gehrsitz(if fixed_gaas { 0.0 } else { 0.3 }, 296.15)),
        temperature: temperature(models::GEHRSITZ_T.0, models::GEHRSITZ_T.1, 296.15),
        composition: (!fixed_gaas).then(|| Parameter {
            symbol: "x".into(),
            name: "Al fraction".into(),
            unit: String::new(),
            min: models::GEHRSITZ_X.0,
            max: models::GEHRSITZ_X.1,
            default: 0.3,
        }),
        accuracy: "against the measured data at 23 °C (Table IV): σ ≤ 2.5e-3 and Δn_max ≤ 9.5e-3, except x = 0.427 (σ 9.9e-3), which the paper attributes to a systematic error of that sample; the measurements themselves: Δn = 5e-4 (grating coupling, 0.73–0.83 µm), < 4e-3 (transmission, 0.5–2 µm), < 8e-3 beyond 2 µm".into(),
        sources: vec![
            src("gehrsitz-2000", "Eqs. (11)–(13), (16); Tables II and IV"),
        ],
        default: true,
        notes: "the range starts 30 meV below the direct gap (0.024 µm⁻¹, where the paper's agreement holds) and at 0.47 µm at the shortest (the shortest wavelength measured); it ends at 3 µm, the longest measured. Temperatures: GaAs's dependence was fitted from 103 to 298 K; the ternaries' dn/dT was measured from 15 to 40 °C".into(),
        eval: if fixed_gaas {
            |t, _| one(models::gehrsitz(0.0, t))
        } else {
            |t, x| one(models::gehrsitz(x, t))
        },
    }
}

fn skauli_model() -> IndexModel {
    let rows = vec![
        strings(&["$E_0$ (eV)", "1.425 000", "−0.000 371 64", "−7.497×10⁻⁷"]),
        strings(&["$E_1$ (eV)", "2.400 356", "−0.000 514 58", ""]),
        strings(&["$E_2$ (eV)", "7.691 979", "−0.000 465 45", ""]),
        strings(&["$E_3$ (eV)", "0.034 303", "+0.000 011 36", ""]),
        strings(&[r"$\langle\varepsilon_2\rangle$", "12.993 86", "", ""]),
        strings(&["$G_3$", "0.002 181 76", "", ""]),
        strings(&["$A$", "0.689 578", "", ""]),
    ];
    IndexModel {
        id: "skauli-2003".into(),
        name: "Skauli et al. 2003".into(),
        axes: vec![Axis::Isotropic],
        equation: r"n^2 = 1 + \frac{A}{\pi}\ln\frac{E_1^2 - (\hbar\omega)^2}{E_0^2 - (\hbar\omega)^2} + \frac{\langle\varepsilon_2\rangle}{\pi}\ln\frac{E_2^2 - (\hbar\omega)^2}{E_1^2 - (\hbar\omega)^2} + \frac{G_3}{E_3^2 - (\hbar\omega)^2}".into(),
        symbols: r"$\hbar\omega = 1.239842/\lambda$ in eV ($\lambda$ in µm); each energy is a constant plus a linear (and for $E_0$ a quadratic) term in $\Delta T = T - 22$ °C. This is the Pikhtin form, Eq. (12), which the paper recommends over its Sellmeier form, Eq. (13)".into(),
        coefficients: vec![table(
            "Table II, the Pikhtin form (Eq. (12))",
            &["", "constant", r"$\times\Delta T$", r"$\times\Delta T^2$"],
            rows,
        )],
        wavelength: models::SKAULI_LAMBDA,
        temperature: temperature(models::SKAULI_T.0, models::SKAULI_T.1, models::SKAULI_T0),
        composition: None,
        accuracy: "n to ±0.2% (a systematic error of the sample's thickness, the same at every wavelength); differences of indices to 0.2% of themselves (uncorrelated errors ≤ ±0.007% in n); dn/dT to ±1.4%, d²n/dT² to ±15%".into(),
        sources: vec![src("skauli-2003", "Eq. (12), Table II")],
        default: true,
        notes: "measured from 22 to 95 °C (FTIR and interferometry); the paper also checks the fits against QPM data at 21 °C (Table I), which this range admits. Toward the band gap both fits leave Palik's data, so the range stops at 0.97 µm, the shortest measured. The default for GaAs: measured on GaAs itself across its transparency and checked by QPM; Gehrsitz et al.'s model at x = 0 reaches the gap and 103 K".into(),
        eval: |t, _| one(models::skauli(t)),
    }
}

fn gaas_r(
    label: &str,
    wavelength: f64,
    clamping: Clamping,
    value: (f64, f64),
    source: Source,
    notes: &str,
) -> Tensor {
    Tensor {
        kind: TensorKind::ElectroOptic,
        label: label.into(),
        point_group: "-43m".into(),
        cells: cells(
            TensorKind::ElectroOptic,
            "-43m",
            &[(4, 1, value.0, Some(value.1))],
        ),
        wavelength: Some(wavelength),
        clamping,
        convention: R_CONVENTION.into(),
        source: Some(source),
        notes: notes.into(),
    }
}

const BERSETH_SIGN: &str = "the sign as Berseth et al. define it: Δ(1/n²) = −r41·E for TE light along [110] with E along [001] (their Eq. (1)); it depends on the choice of axes";

fn gaas() -> Entry {
    let mut gehrsitz = gehrsitz_model(true);
    gehrsitz.default = false;
    let berseth = src("berseth-1992", "abstract; p. 2823; Table I");
    let sugie = src("sugie-tada-1976", "Table I");
    let sugie_note = "1.5 kHz modulation (unclamped); the paper gives magnitudes only. Berseth et al.'s Table I takes them as negative in its convention and adds +0.2 pm/V of piezoelectric contribution for r^S";
    Entry {
        id: "gaas".into(),
        name: "Gallium arsenide".into(),
        formula: "GaAs".into(),
        category: Category::Semiconductor,
        summary: "The direct-gap III-V semiconductor of lasers and of quasi-phase-matched mid-infrared nonlinear optics.".into(),
        crystal: zincblende(""),
        index: vec![skauli_model(), gehrsitz],
        tensors: vec![
            Tensor {
                kind: TensorKind::SecondOrder,
                label: "SHG, 1.533 µm".into(),
                point_group: "-43m".into(),
                cells: cells(TensorKind::SecondOrder, "-43m", &[(1, 4, 119.0, None)]),
                wavelength: Some(1.533),
                clamping: Clamping::None,
                convention: D_CONVENTION.into(),
                source: Some(src("shoji-1997", "Table 11 (d36 = d14)")),
                notes: "magnitude; Maker fringes on (111) plates, d_eff = (2/3)^½ d36; overall accuracy better than 10% (most probably ±5%)".into(),
            },
            Tensor {
                kind: TensorKind::SecondOrder,
                label: "SHG, 1.064 µm".into(),
                point_group: "-43m".into(),
                cells: cells(TensorKind::SecondOrder, "-43m", &[(1, 4, 170.0, None)]),
                wavelength: Some(1.064),
                clamping: Clamping::None,
                convention: D_CONVENTION.into(),
                source: Some(src("shoji-1997", "Table 11 (d36 = d14)")),
                notes: "magnitude; the second harmonic (532 nm) lies above the gap; older values at 1.06 µm range from 140 to 230 pm/V (p. 2284)".into(),
            },
            gaas_r(
                "clamped, 1.52 µm",
                1.52,
                Clamping::Clamped,
                (-1.50, 0.08),
                berseth.clone(),
                &format!(
                    "strip-loaded AlGaAs/GaAs waveguides, corrected for the AlGaAs spacers; the paper calls waveguide measurements clamped (r^S) and states no modulation frequency; {BERSETH_SIGN}"
                ),
            ),
            gaas_r(
                "clamped, 1.32 µm",
                1.32,
                Clamping::Clamped,
                (-1.54, 0.08),
                berseth,
                BERSETH_SIGN,
            ),
            gaas_r(
                "unclamped, 1.15 µm",
                1.15,
                Clamping::Unclamped,
                (-1.43, 0.07),
                sugie.clone(),
                sugie_note,
            ),
            gaas_r(
                "unclamped, 3.39 µm",
                3.39,
                Clamping::Unclamped,
                (-1.24, 0.04),
                sugie.clone(),
                sugie_note,
            ),
            gaas_r(
                "unclamped, 10.6 µm",
                10.6,
                Clamping::Unclamped,
                (-1.51, 0.05),
                sugie,
                sugie_note,
            ),
        ],
        constants: vec![Constant {
            symbol: r"d_{14}(\mathrm{GaAs}) / d_{33}(\mathrm{LiNbO_3})".into(),
            name: "the ratio of the d coefficients, SHG in orientation-patterned GaAs against PPLN".into(),
            value: 5.01,
            uncertainty: Some(0.3),
            unit: String::new(),
            conditions: "fundamental 4.135 µm".into(),
            source: src("skauli-2002", "abstract; p. 629"),
        }],
        missing: vec![],
        references: references(&[
            "skauli-2003",
            "gehrsitz-2000",
            "berseth-1992",
            "sugie-tada-1976",
            "skauli-2002",
            "shoji-1997",
        ]),
    }
}

fn afromowitz_model() -> IndexModel {
    IndexModel {
        id: "afromowitz-1974".into(),
        name: "Afromowitz 1974".into(),
        axes: vec![Axis::Isotropic],
        equation: r"n^2 = 1 + M_{-1} + M_{-3}E^2 + \frac{\eta}{\pi}E^4 \ln\frac{E_f^2 - E^2}{E_\Gamma^2 - E^2}, \quad E_f^2 = 2E_0^2 - E_\Gamma^2, \quad \eta = \frac{\pi E_d}{2E_0^3(E_0^2 - E_\Gamma^2)}, \quad M_{-1} = \frac{\eta}{2\pi}(E_f^4 - E_\Gamma^4), \quad M_{-3} = \frac{\eta}{\pi}(E_f^2 - E_\Gamma^2)".into(),
        symbols: r"$E$ the photon energy in eV; $E_0$ and $E_d$ the single oscillator's energies and $E_\Gamma$ the direct gap, each a function of $x$ (the appendix)".into(),
        coefficients: vec![table(
            "Appendix, Ga₁₋ₓAlₓAs (eV)",
            &["", "constant", "$x$", "$x^2$"],
            vec![
                strings(&["$E_0$", "3.65", "0.871", "0.179"]),
                strings(&["$E_d$", "36.1", "−2.45", ""]),
                strings(&[r"$E_\Gamma$", "1.424", "1.266", "0.26"]),
            ],
        )],
        wavelength: nominal(models::afromowitz(0.3)),
        temperature: None,
        composition: Some(Parameter {
            symbol: "x".into(),
            name: "Al fraction".into(),
            unit: String::new(),
            min: 0.0,
            max: 1.0,
            default: 0.3,
        }),
        accuracy: "within 0.004 of GaAs data from 0.895 to 1.7 µm; of AlAs within 0.004 to 1.5 eV and 0.014 to 2 eV; compared with alloy data up to x = 0.38".into(),
        sources: vec![src("afromowitz-1974", "Eqs. (7)–(12), appendix")],
        default: false,
        notes: "room temperature. The range starts 0.039 eV below the gap (where GaAs was checked, 0.895 µm) but not above 2 eV (where AlAs was), and ends at 1.7 µm. An older model kept for comparison; Gehrsitz et al. is the default".into(),
        eval: |_, x| one(models::afromowitz(x)),
    }
}

fn algaas() -> Entry {
    Entry {
        id: "algaas".into(),
        name: "Aluminium gallium arsenide".into(),
        formula: "AlₓGa₁₋ₓAs".into(),
        category: Category::Semiconductor,
        summary: "The GaAs-lattice-matched alloy of III-V photonics: its gap and index set by the Al fraction x.".into(),
        crystal: zincblende("a random alloy; lattice matched to GaAs for every x"),
        index: vec![gehrsitz_model(false), afromowitz_model()],
        tensors: vec![],
        constants: vec![],
        missing: vec![
            missing(
                "d14(x)",
                "Ohashi et al. 1993 measure only |d(x)/d(GaAs)| at 1.064 µm, plotted in Fig. 6 (±20%), with no table and no absolute value: no number to ship",
            ),
            missing(
                "r41(x)",
                "Adachi 1985 has no electro-optic section; no primary measurement of AlGaAs's r41 is in hand",
            ),
            missing(
                "Adachi's index model",
                "Adachi 1985, Eqs. (73)–(78), states no range or accuracy, so it isn't shipped",
            ),
        ],
        references: references(&[
            "gehrsitz-2000",
            "papatryfonos-2021",
            "afromowitz-1974",
            "ohashi-1993",
            "adachi-1985",
        ]),
    }
}

// ---------------------------------------------------------------------------------------------
// Lithium niobate
// ---------------------------------------------------------------------------------------------

fn trigonal_3m() -> Crystal {
    Crystal {
        system: CrystalSystem::Trigonal,
        point_group: "3m".into(),
        space_group: Some("R3c (No. 161)".into()),
        structure: "lithium niobate (ilmenite-like)".into(),
        optical: OpticalClass::Uniaxial {
            positive: false,
            optic_axis: "z (the c axis)".into(),
        },
        centrosymmetric: false,
        notes: "ferroelectric below its Curie temperature; the mirror plane is perpendicular to x"
            .into(),
    }
}

fn zelmon_rows(printed: [[&str; 2]; 6]) -> Vec<Vec<String>> {
    ["$A$", "$B$ (µm²)", "$C$", "$D$ (µm²)", "$E$", "$F$ (µm²)"]
        .iter()
        .zip(printed)
        .map(|(label, [ne, no])| strings(&[label, ne, no]))
        .collect()
}

const ZELMON_EQUATION: &str = r"n^2 - 1 = \frac{A\lambda^2}{\lambda^2 - B} + \frac{C\lambda^2}{\lambda^2 - D} + \frac{E\lambda^2}{\lambda^2 - F}";

fn zelmon_pair(columns: (ZelmonColumn, ZelmonColumn), table: &str) -> Result<Vec<Material>> {
    let (e, o) = columns;
    Ok(vec![
        models::zelmon("LiNbO3 (o)", o, table)?,
        models::zelmon("LiNbO3 (e)", e, table)?,
    ])
}

const D_SCALE_NOTE: &str = "magnitudes (the paper gives no signs); SHG Maker fringes with the multiple reflections in the plate taken into account. The paper puts its overall accuracy at better than 10% (most probably ±5%) and finds many older standard values too large for neglecting those reflections: Choy & Byer 1976's scale (d36(KDP) = 0.63 pm/V) gives d33 = 34.4 pm/V at 1.06 µm, Miller et al.'s 27.2 pm/V; Shoji's rests on d36(KDP) = 0.39 pm/V";

/// Shoji et al.'s d33 and d31 of a lithium niobate at one fundamental wavelength.
fn shoji_d(wavelength: f64, d33: f64, d31: f64, location: &str) -> Tensor {
    Tensor {
        kind: TensorKind::SecondOrder,
        label: format!("SHG, {wavelength} µm"),
        point_group: "3m".into(),
        cells: cells(
            TensorKind::SecondOrder,
            "3m",
            &[(3, 3, d33, None), (3, 1, d31, None)],
        ),
        wavelength: Some(wavelength),
        clamping: Clamping::None,
        convention: format!(
            "{D_CONVENTION}; Shoji et al. follow Roberts 1992 (d_rpq = χ⁽²⁾_rpq/2, IEEE/ANSI axes, x ⊥ the mirror plane)"
        ),
        source: Some(src("shoji-1997", location)),
        notes: format!(
            "{D_SCALE_NOTE}. d22 and d15 aren't measured; under Kleinman's symmetry d15 = d31"
        ),
    }
}

fn jundt_model() -> IndexModel {
    IndexModel {
        id: "jundt-1997".into(),
        name: "Jundt 1997".into(),
        axes: vec![Axis::Extraordinary],
        equation: JUNDT_EQUATION.into(),
        symbols: JUNDT_SYMBOLS.into(),
        coefficients: vec![table(
            "Table 2, congruent LiNbO₃, n_e",
            &["", "value", "", "value"],
            vec![
                strings(&["$a_1$", "5.35583", "$b_1$", "4.629×10⁻⁷"]),
                strings(&["$a_2$", "0.100473", "$b_2$", "3.862×10⁻⁸"]),
                strings(&["$a_3$", "0.20692", "$b_3$", "−0.89×10⁻⁸"]),
                strings(&["$a_4$", "100", "$b_4$", "2.657×10⁻⁵"]),
                strings(&["$a_5$", "11.34927", "", ""]),
                strings(&["$a_6$", "1.5334×10⁻²", "", ""]),
            ],
        )],
        wavelength: models::JUNDT_LAMBDA,
        temperature: temperature(models::JUNDT_T.0, models::JUNDT_T.1, 297.65),
        composition: None,
        accuracy: "no index accuracy stated; fitted (χ² = 89 for 192 points) to index data normalized to 20 °C (uncertainty 2.4e-4) and to PPLN OPO tuning, 'accurate for temperatures between room temperature and 250 °C and wavelengths from 0.4 to 5 µm'".into(),
        sources: vec![src("jundt-1997", "Eqs. (4)–(5), Table 2")],
        default: false,
        notes: "the extraordinary index only; congruent composition (48.38 mol% Li₂O); a₄ was held at 100 µm². T in °C in f, which uses 273.16 as the paper writes it".into(),
        eval: |t, _| one(models::jundt(t)),
    }
}

const JUNDT_EQUATION: &str = r"n^2 = a_1 + b_1 f + \frac{a_2 + b_2 f}{\lambda^2 - (a_3 + b_3 f)^2} + \frac{a_4 + b_4 f}{\lambda^2 - a_5^2} - a_6\lambda^2, \quad f = (T - 24.5)(T + 570.82)";
const JUNDT_SYMBOLS: &str =
    r"$\lambda$ in µm; $T$ in °C; $f$ vanishes at the reference temperature 24.5 °C";

fn lithium_niobate() -> Entry {
    let r_s = Tensor {
        kind: TensorKind::ElectroOptic,
        label: "bulk, clamped, 633 nm".into(),
        point_group: "3m".into(),
        cells: cells(
            TensorKind::ElectroOptic,
            "3m",
            &[
                (2, 2, 3.40, Some(0.05)),
                (1, 3, 9.10, Some(0.3)),
                (3, 3, 31.2, Some(0.4)),
                (5, 1, 18.1, Some(1.0)),
            ],
        ),
        wavelength: Some(0.633),
        clamping: Clamping::Clamped,
        convention: R_CONVENTION.into(),
        source: Some(src(
            "jazbinsek-zgonik-2002",
            "Table 5, lines 21–24 (r^S_113, r^S_333, r^S_131, r^S_222)",
        )),
        notes: "congruent, nominally undoped, 25 °C; a least-squares fit to many measurements"
            .into(),
    };
    let r_t = Tensor {
        kind: TensorKind::ElectroOptic,
        label: "bulk, unclamped, 633 nm".into(),
        point_group: "3m".into(),
        cells: cells(
            TensorKind::ElectroOptic,
            "3m",
            &[
                (2, 2, 6.64, None),
                (1, 3, 10.12, None),
                (3, 3, 31.45, None),
                (5, 1, 33.96, None),
            ],
        ),
        wavelength: Some(0.633),
        clamping: Clamping::Unclamped,
        convention: R_CONVENTION.into(),
        source: Some(src(
            "jazbinsek-zgonik-2002",
            "Table 4, the fitted values of lines 67–72 (r^T_113, r^T_333, r^T_131, r^T_222)",
        )),
        notes: "the unclamped coefficients the fitted parameters of Table 5 imply; the paper gives no uncertainty for them, only for the measured inputs".into(),
    };
    let thin_film = Tensor {
        kind: TensorKind::ElectroOptic,
        label: "thin film (x-cut LNOI), clamped, 1550 nm".into(),
        point_group: "3m".into(),
        cells: cells(TensorKind::ElectroOptic, "3m", &[(3, 3, 26.9, Some(0.9))]),
        wavelength: Some(1.55),
        clamping: Clamping::Clamped,
        convention: R_CONVENTION.into(),
        source: Some(src(
            "chelladurai-2025",
            "main text, Fig. 4(a); Supplementary Fig. S7 (±0.9 at 10 GHz)",
        )),
        notes: "flat from 100 MHz to 330 GHz; the film's other elements are resolved only as ½(r13 + 2r42) (see the constants); the paper ascribes the difference from bulk to the wavelength (1550 nm, not 633 nm)".into(),
    };
    Entry {
        id: "linbo3".into(),
        name: "Lithium niobate (congruent)".into(),
        formula: "LiNbO₃".into(),
        category: Category::NonlinearCrystal,
        summary: "The ferroelectric crystal of electro-optic modulators and periodically poled frequency conversion, in bulk and as thin film (LNOI).".into(),
        crystal: trigonal_3m(),
        index: vec![
            IndexModel {
                id: "zelmon-1997-congruent".into(),
                name: "Zelmon et al. 1997".into(),
                axes: vec![Axis::Ordinary, Axis::Extraordinary],
                equation: ZELMON_EQUATION.into(),
                symbols: r"$\lambda$ in µm; $B$, $D$, $F$ in µm²".into(),
                coefficients: vec![table(
                    "Table 1, congruently grown LiNbO₃",
                    &["", "$n_e$", "$n_o$"],
                    zelmon_rows(models::ZELMON_CONGRUENT_PRINTED),
                )],
                wavelength: models::ZELMON_LAMBDA,
                temperature: None,
                composition: None,
                accuracy: "the fit is within 2e-4 of every measured index; the measurements' standard deviation is below 2e-4".into(),
                sources: vec![src("zelmon-1997", "Table 1 and the equation of Section 3")],
                default: true,
                notes: "21 °C; minimum deviation on prisms".into(),
                eval: |_, _| zelmon_pair(models::ZELMON_CONGRUENT, "Table 1"),
            },
            jundt_model(),
        ],
        tensors: vec![
            shoji_d(1.064, 25.2, 4.6, "Table 10"),
            shoji_d(1.313, 19.5, 3.2, "Table 10"),
            shoji_d(0.852, 25.7, 4.8, "Table 10"),
            r_s,
            r_t,
            thin_film,
        ],
        constants: vec![
            Constant {
                symbol: r"\varepsilon^S_{11}".into(),
                name: "static permittivity, clamped, perpendicular to c".into(),
                value: 45.5,
                uncertainty: Some(0.3),
                unit: String::new(),
                conditions: "25 °C".into(),
                source: src("jazbinsek-zgonik-2002", "Table 5, line 11"),
            },
            Constant {
                symbol: r"\varepsilon^S_{33}".into(),
                name: "static permittivity, clamped, along c".into(),
                value: 26.2,
                uncertainty: Some(0.2),
                unit: String::new(),
                conditions: "25 °C".into(),
                source: src("jazbinsek-zgonik-2002", "Table 5, line 12"),
            },
            Constant {
                symbol: r"\tfrac12(r_{13} + 2r_{42})".into(),
                name: "the combination the thin film's in-plane angle sweep resolves".into(),
                value: 15.0,
                uncertainty: None,
                unit: "pm/V".into(),
                conditions: "thin film (x-cut), 1550 nm, 100 MHz to 330 GHz".into(),
                source: src("chelladurai-2025", "main text, Fig. 4(a)"),
            },
        ],
        missing: vec![missing(
            "d22, d15",
            "Shoji et al. measure d33 and d31 only; no other primary absolute measurement is in hand",
        )],
        references: references(&[
            "zelmon-1997",
            "jundt-1997",
            "shoji-1997",
            "roberts-1992",
            "choy-byer-1976",
            "jazbinsek-zgonik-2002",
            "chelladurai-2025",
        ]),
    }
}

fn gayer_model(extraordinary: bool) -> IndexModel {
    let (axis, id, column, lam, t, notes) = if extraordinary {
        (
            Axis::Extraordinary,
            "gayer-2008-e",
            [
                "5.756",
                "0.0983",
                "0.2020",
                "189.32",
                "12.52",
                "1.32×10⁻²",
                "2.860×10⁻⁶",
                "4.700×10⁻⁸",
                "6.113×10⁻⁸",
                "1.516×10⁻⁴",
            ],
            models::GAYER_E_LAMBDA,
            models::GAYER_E_T,
            "valid from 0.5 to 4 µm and 20 to 200 °C (Sec. 5); fitted to QPM SHG and OPO tuning of 5% MgO-doped CLN",
        )
    } else {
        (
            Axis::Ordinary,
            "gayer-2008-o",
            [
                "5.653",
                "0.1185",
                "0.2091",
                "89.61",
                "10.85",
                "1.97×10⁻²",
                "7.941×10⁻⁷",
                "3.134×10⁻⁸",
                "−4.641×10⁻⁹",
                "−2.188×10⁻⁶",
            ],
            models::GAYER_O_LAMBDA,
            models::GAYER_O_T,
            "'considered reliable only for wavelengths up to 1620 nm and roughly 20–100 °C' (Sec. 5)",
        )
    };
    let labels = ["a1", "a2", "a3", "a4", "a5", "a6", "b1", "b2", "b3", "b4"];
    IndexModel {
        id: id.into(),
        name: format!(
            "Gayer et al. 2008, {}",
            if extraordinary { "nₑ" } else { "nₒ" }
        ),
        axes: vec![axis],
        equation: JUNDT_EQUATION.into(),
        symbols: JUNDT_SYMBOLS.into(),
        coefficients: vec![table(
            "Table 1 as corrected by the erratum (Appl. Phys. B 101, 481 (2010)), 5% MgO-doped CLN",
            &["", if extraordinary { "$n_e$" } else { "$n_o$" }],
            labels
                .iter()
                .zip(column)
                .map(|(l, v)| vec![format!("${}_{}$", &l[..1], &l[1..]), v.to_owned()])
                .collect(),
        )],
        wavelength: lam,
        temperature: temperature(t.0, t.1, 297.65),
        composition: None,
        accuracy: "no absolute accuracy stated; against Paul et al.'s n_e below 1.7e-4 (1.05–4 µm, 20–200 °C) and against Zelmon et al. (columns exchanged) below 3.1e-4 for n_e (0.5–3 µm) and 2.2e-4 for n_o".into(),
        sources: vec![src("gayer-2008", "Eqs. (2)–(3); Table 1 of the 2010 erratum")],
        default: false,
        notes: notes.into(),
        eval: if extraordinary {
            |t, _| one(models::gayer(true, t))
        } else {
            |t, _| one(models::gayer(false, t))
        },
    }
}

fn lithium_niobate_mgo() -> Entry {
    Entry {
        id: "linbo3-mgo".into(),
        name: "Lithium niobate, 5 mol% MgO".into(),
        formula: "MgO:LiNbO₃".into(),
        category: Category::NonlinearCrystal,
        summary: "Congruent lithium niobate doped with 5 mol% MgO against photorefractive damage, the usual crystal of periodically poled frequency converters.".into(),
        crystal: trigonal_3m(),
        index: vec![
            IndexModel {
                id: "zelmon-1997-mgo".into(),
                name: "Zelmon et al. 1997".into(),
                axes: vec![Axis::Ordinary, Axis::Extraordinary],
                equation: ZELMON_EQUATION.into(),
                symbols: r"$\lambda$ in µm; $B$, $D$, $F$ in µm²".into(),
                coefficients: vec![table(
                    "Table 2, congruently grown LiNbO₃ doped with 5 mol% MgO, with the headings as printed",
                    &["", "$n_e$ (printed; is $n_o$)", "$n_o$ (printed; is $n_e$)"],
                    zelmon_rows(models::ZELMON_MGO_PRINTED),
                )],
                wavelength: models::ZELMON_LAMBDA,
                temperature: None,
                composition: None,
                accuracy: "the fit is within 2e-4 of every measured index".into(),
                sources: vec![
                    src("zelmon-1997", "Table 2, Fig. 2"),
                    src("gayer-2008", "Sec. 4.3.1 (the exchanged columns)"),
                ],
                default: true,
                notes: "21 °C. Table 2's columns are exchanged: the column printed as n_e gives the larger index, which lithium niobate's n_o is (it is negative uniaxial), and which the paper's own Fig. 2 shows as n_o; Gayer et al. (2008) reach the same conclusion. The catalogue uses the column printed n_e as n_o and the one printed n_o as n_e".into(),
                eval: |_, _| zelmon_pair(
                    (models::ZELMON_MGO.1, models::ZELMON_MGO.0),
                    "Table 2, columns exchanged",
                ),
            },
            gayer_model(true),
            gayer_model(false),
        ],
        tensors: vec![
            shoji_d(1.064, 25.0, 4.4, "Table 10"),
            shoji_d(1.313, 20.3, 3.4, "Table 10"),
            shoji_d(0.852, 28.4, 4.9, "Table 10"),
        ],
        constants: vec![],
        missing: vec![missing(
            "r_ij",
            "no primary measurement of MgO-doped lithium niobate's electro-optic tensor is in hand",
        )],
        references: references(&["zelmon-1997", "gayer-2008", "shoji-1997", "roberts-1992"]),
    }
}

// ---------------------------------------------------------------------------------------------
// InGaP, InP, AlN
// ---------------------------------------------------------------------------------------------

fn single_d14(
    label: &str,
    wavelength: f64,
    value: (f64, Option<f64>),
    source: Source,
    notes: &str,
) -> Tensor {
    Tensor {
        kind: TensorKind::SecondOrder,
        label: label.into(),
        point_group: "-43m".into(),
        cells: cells(TensorKind::SecondOrder, "-43m", &[(1, 4, value.0, value.1)]),
        wavelength: Some(wavelength),
        clamping: Clamping::None,
        convention: D_CONVENTION.into(),
        source: Some(source),
        notes: notes.into(),
    }
}

fn ingap() -> Entry {
    let tanaka = IndexModel {
        id: "tanaka-1986".into(),
        name: "Tanaka et al. 1986".into(),
        axes: vec![Axis::Isotropic],
        equation: r"n^2 - 1 = \frac{E_0 E_d}{E_0^2 - E^2}, \quad E_0 = 3.39 + 1.216x, \quad E_d = 28.07 + 3.373x".into(),
        symbols: r"$E$ the photon energy in eV; Wemple and DiDomenico's single effective oscillator, $E_0$ and $E_d$ in eV; $x$ the Al content of In₀.₄₉Ga₀.₅₁₋ₓAlₓP, $x = 0$ being lattice-matched InGaP".into(),
        coefficients: vec![table(
            "Eqs. (2)–(4) and p. 986 (eV)",
            &["", "In₀.₄₉Ga₀.₅₁P", "In₀.₄₉Al₀.₅₁P", "per unit $x$"],
            vec![
                strings(&["$E_0$", "3.39", "4.01", "1.216"]),
                strings(&["$E_d$", "28.07", "29.79", "3.373"]),
            ],
        )],
        wavelength: nominal(models::tanaka(0.0)),
        temperature: None,
        composition: Some(Parameter {
            symbol: "x".into(),
            name: "Al content (In₀.₄₉Ga₀.₅₁₋ₓAlₓP)".into(),
            unit: String::new(),
            min: models::TANAKA_X.0,
            max: models::TANAKA_X.1,
            default: 0.0,
        }),
        accuracy: "no number stated ('sufficient agreement' with the measured points); n from reflectance minima of 0.4–1.2 µm layers on GaAs".into(),
        sources: vec![src("tanaka-1986", "Eqs. (2)–(4); p. 986")],
        default: true,
        notes: "MBE layers at room temperature (300 K), measured from 0.6 to 1.3 eV (0.95–2.07 µm), the range here. Ueno et al. (1997) and the InGaP-photonics papers use this model for n at 1.55 µm; Ahler et al. (2026) find their bonded films need their own ellipsometry".into(),
        eval: |_, x| one(models::tanaka(x)),
    };
    Entry {
        id: "ingap".into(),
        name: "Indium gallium phosphide".into(),
        formula: "In₀.₄₉Ga₀.₅₁P".into(),
        category: Category::Semiconductor,
        summary: "The GaAs-lattice-matched III-V of χ⁽²⁾ integrated photonics: a large d14 and a 1.9 eV gap, with no two-photon absorption at 1.55 µm.".into(),
        crystal: zincblende("disordered (a random alloy); CuPt ordering of the group-III sites lowers the symmetry to 3m about a ⟨111⟩ axis and makes the crystal slightly birefringent (Ueno et al., Eq. (1))"),
        index: vec![tanaka],
        tensors: vec![
            single_d14(
                "waveguide SHG, 1.55 µm",
                1.55,
                (106.0, Some(4.0)),
                src("ahler-2026", "abstract; lines 173–175 of the manuscript"),
                "an absolute determination: CW SHG in InGaP-on-insulator waveguides of several lengths, fitted with the coupled-amplitude equations, measured losses and a simulated overlap; the weighted mean over MBE and MOCVD films. The paper writes χ⁽²⁾ = 212 pm/V for it",
            ),
            single_d14(
                "Maker fringes, 1.579 µm",
                1.579,
                (110.0, None),
                src("ueno-1997", "p. 1433, Fig. 7"),
                "on an ordered MOVPE layer (δ ≈ 0.2), from its interference with the GaAs substrate's second harmonic, assuming GaAs's d14 = 130 pm/V; no uncertainty stated. In the ordered crystal's 3m tensor d'33 ≲ 60 and d'31 below 20 pm/V",
            ),
        ],
        constants: vec![Constant {
            symbol: r"\varepsilon_\infty".into(),
            name: "high-frequency dielectric constant of Ga₀.₅₁In₀.₄₉P, from ellipsometry".into(),
            value: 9.43,
            uncertainty: Some(0.02),
            unit: String::new(),
            conditions: "room temperature (Table I; the text says ±0.03)".into(),
            source: src("schubert-1995", "Table I"),
        }],
        missing: vec![
            missing(
                "r41",
                "no measurement of InGaP's electro-optic coefficient is in hand",
            ),
            missing(
                "n above 1.3 eV",
                "Schubert et al. (1995) give n and k from 0.8 to 5 eV only as curves; Ahler et al.'s bonded-film model is a data deposit (doi:10.5281/zenodo.17748661), not printed",
            ),
        ],
        references: references(&["tanaka-1986", "schubert-1995", "ueno-1997", "ahler-2026"]),
    }
}

fn pettit_turner_model(kelvin: f64) -> IndexModel {
    let (t, _, _, _, gap) = *models::PETTIT_TURNER
        .iter()
        .find(|p| p.0 == kelvin)
        .expect("298 or 77 K");
    IndexModel {
        id: format!("pettit-turner-1965-{}k", t as u32),
        name: format!("Pettit & Turner 1965, {} K", t as u32),
        axes: vec![Axis::Isotropic],
        equation: r"n^2 = A + \frac{B\lambda^2}{\lambda^2 - C^2}".into(),
        symbols: r"$\lambda$ in Å, as the paper writes it".into(),
        coefficients: vec![table(
            "Table I",
            &["$T$", "$A$", "$B$", "$C^2$ (Å²)"],
            vec![
                strings(&["298 K", "7.255", "2.316", "0.3922×10⁸"]),
                strings(&["77 K", "7.781", "1.661", "0.4397×10⁸"]),
            ],
        )],
        wavelength: nominal(models::pettit_turner(kelvin)),
        temperature: None,
        composition: None,
        accuracy:
            "the fit is within 0.007 of the measured indices; the measurements reproduce to ±0.0003"
                .into(),
        sources: vec![src("pettit-turner-1965", "Eq. (1), Table I")],
        default: kelvin == 298.0,
        notes: format!(
            "{} K; measured from 0.60 eV to the band edge ({} eV, Table II)",
            t as u32, gap
        ),
        eval: if kelvin == 298.0 {
            |_, _| one(models::pettit_turner(298.0))
        } else {
            |_, _| one(models::pettit_turner(77.0))
        },
    }
}

fn inp() -> Entry {
    let r = |wavelength: f64, clamping: Clamping, value: f64| Tensor {
        kind: TensorKind::ElectroOptic,
        label: format!(
            "{}, {wavelength} µm",
            if clamping == Clamping::Clamped {
                "clamped"
            } else {
                "unclamped"
            }
        ),
        point_group: "-43m".into(),
        cells: cells(TensorKind::ElectroOptic, "-43m", &[(4, 1, value, None)]),
        wavelength: Some(wavelength),
        clamping,
        convention: R_CONVENTION.into(),
        source: Some(src("suzuki-tada-1984", "Table I")),
        notes: if clamping == Clamping::Clamped {
            "r^S = r^T − p44·s44·e14 (their Eq. (5)); the sign is assumed negative 'as in other III-V semiconductors', not measured; errors below ±5%".into()
        } else {
            "transverse geometry with 30 ns field steps, bulk Fe-doped semi-insulating InP; the sign is assumed, not measured; errors below ±5%".into()
        },
    };
    Entry {
        id: "inp".into(),
        name: "Indium phosphide".into(),
        formula: "InP".into(),
        category: Category::Semiconductor,
        summary:
            "The substrate of telecom lasers, modulators and the InP generic integration platforms."
                .into(),
        crystal: zincblende(""),
        index: vec![pettit_turner_model(298.0), pettit_turner_model(77.0)],
        tensors: vec![
            r(1.50, Clamping::Clamped, -1.68),
            r(1.306, Clamping::Clamped, -1.59),
            r(1.208, Clamping::Clamped, -1.54),
            r(1.064, Clamping::Clamped, -1.34),
            r(1.50, Clamping::Unclamped, -1.63),
            r(1.306, Clamping::Unclamped, -1.53),
            r(1.208, Clamping::Unclamped, -1.49),
            r(1.064, Clamping::Unclamped, -1.32),
        ],
        constants: vec![Constant {
            symbol: r"d_{14}(\mathrm{InP}) / d_{14}(\mathrm{GaAs})".into(),
            name: "the ratio of the d coefficients, SHG in wedges against GaAs".into(),
            value: 0.78,
            uncertainty: Some(0.08),
            unit: String::new(),
            conditions: "fundamental 10.55 µm, room temperature".into(),
            source: src("lee-fan-1974", "Table I"),
        }],
        missing: vec![missing(
            "d14",
            "Lee & Fan (1974) measure it only relative to GaAs at 10.6 µm (0.78 ± 0.08), against a GaAs value of (3.2 ± 1)e-7 esu they take from another paper: no absolute value is printed",
        )],
        references: references(&["pettit-turner-1965", "suzuki-tada-1984", "lee-fan-1974"]),
    }
}

fn aln() -> Entry {
    let majkic = src("majkic-2017", "Tables 1 and 2");
    let r_bulk = Tensor {
        kind: TensorKind::ElectroOptic,
        label: "bulk, unclamped, 633 nm".into(),
        point_group: "6mm".into(),
        cells: cells(
            TensorKind::ElectroOptic,
            "6mm",
            &[(3, 3, 1.16, Some(0.13)), (1, 3, 0.11, Some(0.02))],
        ),
        wavelength: Some(0.633),
        clamping: Clamping::Unclamped,
        convention: R_CONVENTION.into(),
        source: Some(majkic.clone()),
        notes: "magnitudes (the interferometer measures |r|); PVT-grown bulk crystal, 2 kHz modulation. The paper estimates the clamped values by Miller's rule as r33^S = 1.00 and r13^S = 0.10 pm/V. r51 isn't measured".into(),
    };
    let r_film = Tensor {
        kind: TensorKind::ElectroOptic,
        label: "sputtered film, 633 nm".into(),
        point_group: "6mm".into(),
        cells: cells(
            TensorKind::ElectroOptic,
            "6mm",
            &[(3, 3, -0.59, Some(0.07)), (1, 3, 0.67, Some(0.07))],
        ),
        wavelength: Some(0.633),
        clamping: Clamping::Unclamped,
        convention: R_CONVENTION.into(),
        source: Some(src("graupner-1992", "p. 4138, abstract")),
        notes: "an effective coefficient of a 1 µm columnar film (c normal to the film) between ITO and Al, at 100 Hz–100 kHz, which the paper doesn't call clamped or unclamped; signs as printed, the convention not stated; no r51 seen. It disagrees with the bulk crystal's values in sign and size".into(),
    };
    let d = Tensor {
        kind: TensorKind::SecondOrder,
        label: "Maker fringes, 1030 nm".into(),
        point_group: "6mm".into(),
        cells: cells(
            TensorKind::SecondOrder,
            "6mm",
            &[(3, 3, 4.3, Some(0.3)), (3, 1, 0.1, None)],
        ),
        wavelength: Some(1.03),
        clamping: Clamping::None,
        convention: D_CONVENTION.into(),
        source: Some(majkic.clone()),
        notes: "magnitudes; |d33| = (0.169 ± 0.009)·d33(LiNbO₃), with Shoji et al.'s 25.2 pm/V; |d31| = |d33|/(45 ± 5), printed as ~0.1; d15 isn't measured".into(),
    };
    Entry {
        id: "aln".into(),
        name: "Aluminium nitride".into(),
        formula: "AlN".into(),
        category: Category::Semiconductor,
        summary: "A wide-gap (6 eV) wurtzite crystal, sputtered or epitaxial, for UV to mid-IR photonics, piezoelectric and χ⁽²⁾ devices.".into(),
        crystal: Crystal {
            system: CrystalSystem::Hexagonal,
            point_group: "6mm".into(),
            space_group: Some("P6₃mc (No. 186)".into()),
            structure: "wurtzite".into(),
            optical: OpticalClass::Uniaxial {
                positive: true,
                optic_axis: "z (the c axis)".into(),
            },
            centrosymmetric: false,
            notes: "positive in bulk (Pastrňák & Roskovcová: n_e − n_o ≈ 0.05); Gräupner et al.'s sputtered film is slightly negative".into(),
        },
        index: vec![],
        tensors: vec![d, r_bulk, r_film],
        constants: vec![
            Constant {
                symbol: "n_o".into(),
                name: "ordinary index of bulk single crystals, interference fringes".into(),
                value: 2.17,
                uncertainty: Some(0.05),
                unit: String::new(),
                conditions: "589 nm (Na D line); temperature not stated".into(),
                source: src("pastrnak-roskovcova-1966", "p. K6"),
            },
            Constant {
                symbol: "n_e".into(),
                name: "extraordinary index of bulk single crystals".into(),
                value: 2.22,
                uncertainty: Some(0.05),
                unit: String::new(),
                conditions: "589 nm (Na D line)".into(),
                source: src("pastrnak-roskovcova-1966", "p. K6"),
            },
            Constant {
                symbol: "n_o".into(),
                name: "ordinary index of a sputtered film, prism coupling".into(),
                value: 2.0920,
                uncertainty: Some(2e-4),
                unit: String::new(),
                conditions: "633 nm, 2.1 µm film on glass, 'a typical result'".into(),
                source: src("graupner-1992", "p. 4137"),
            },
            Constant {
                symbol: "n_e".into(),
                name: "extraordinary index of the same film".into(),
                value: 2.086,
                uncertainty: Some(1e-3),
                unit: String::new(),
                conditions: "633 nm".into(),
                source: src("graupner-1992", "p. 4137"),
            },
            Constant {
                symbol: r"|d_{33}/d_{31}|".into(),
                name: "the ratio Maker fringes give directly".into(),
                value: 45.0,
                uncertainty: Some(5.0),
                unit: String::new(),
                conditions: "1030 nm".into(),
                source: majkic,
            },
        ],
        missing: vec![missing(
            "index model",
            "Pastrňák & Roskovcová (1966) plot n_o and n_e from 0.22 to 0.6 µm but print neither a table nor a formula, only the values at 589 nm; Majkić et al. use Rigler et al.'s Sellmeier (Appl. Phys. Express 8, 042603 (2015)), not in hand. Gräupner et al.'s Cauchy formula is misprinted (its B makes n constant)",
        )],
        references: references(&["pastrnak-roskovcova-1966", "majkic-2017", "graupner-1992", "shoji-1997"]),
    }
}
