//! Thermal conductivities, beside the indices: each material's models, each from its paper,
//! with its temperature range and, for a film, its thickness.
//!
//! - Silicon in bulk: Ho, Powell & Liley's recommended values (J. Phys. Chem. Ref. Data 1, 279
//!   (1972), doi:10.1063/1.3253100), 200 to 1000 K, the default; and Kremer et al.'s fit to
//!   natural silicon (Solid State Commun. 131, 499 (2004), doi:10.1016/j.ssc.2004.06.022),
//!   220 to 320 K; and Glassbrenner & Slack's measured single crystals (Phys. Rev. 134, A1058
//!   (1964), doi:10.1103/PhysRev.134.A1058, Table I), 50 K to the melting point, 1681 K.
//! - Silicon films: Ju & Goodson's effective mean free path of 0.3 µm at 300 K (Appl. Phys. Lett.
//!   74, 3005 (1999), doi:10.1063/1.123994) in Sondheimer's reduction of a film's conductivity
//!   along it, on Ho et al.'s bulk value.
//! - Fused silica: Powell, Ho & Liley's recommended values for clear fused quartz (NSRDS-NBS 8
//!   (1966), doi:10.6028/NBS.NSRDS.8), 100 to 900 K.
//! - The heater metals tungsten, titanium and platinum: Ho et al.'s recommended values for the
//!   pure, annealed metals, 200 to 1000 K. A deposited film conducts less.
//! - TiN, a film: Goel et al.'s 86 nm ALD films at room temperature (Nanoscale Adv. 8, 612
//!   (2026), doi:10.1039/d5na00914f, p. 619), plasma-enhanced (the default) and thermal; for
//!   that thickness only, since sputtered films measure far lower (the model's notes).
//!
//! A table's values between its rows are taken as a power law of T (linear in log k against
//! log T), which its in-between rows check (the validation cases).

use serde::Serialize;

use super::references::references;
use super::{CoefficientTable, Parameter, Reference, Source};
use crate::Result;

/// A material's thermal conductivity: its models, the default first, and their papers.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct Conductivity {
    /// The id: the catalogue entry's ([`super::Entry::id`]) where the material has one, e.g.
    /// `"si"`; a metal's symbol in lower case, e.g. `"w"`.
    pub id: String,
    /// The name, e.g. `"Silicon"`.
    pub name: String,
    /// The formula, e.g. `"Si"`.
    pub formula: String,
    /// The models; the default first.
    pub models: Vec<ConductivityModel>,
    /// Every paper the models cite.
    pub references: Vec<Reference>,
}

/// k(T) in closed form: T (K) and, for a film, its thickness (µm) to W/(m·K). No range check.
type Eval = fn(f64, f64) -> f64;

/// A thermal-conductivity model of a material, from one paper.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct ConductivityModel {
    /// A short id, unique within its material, e.g. `"ho-1972"`.
    pub id: String,
    /// A short name, e.g. `"Ho, Powell & Liley 1972"`.
    pub name: String,
    /// The equation, in LaTeX.
    pub equation: String,
    /// What the symbols are, in text with LaTeX between `$`s.
    pub symbols: String,
    /// The values or coefficients as printed.
    pub coefficients: Vec<CoefficientTable>,
    /// The temperature, K.
    pub temperature: Parameter,
    /// A film's thickness (µm), for a model of a film.
    pub thickness: Option<Parameter>,
    /// The accuracy the paper states.
    pub accuracy: String,
    /// The papers and the places in them.
    pub sources: Vec<Source>,
    /// Whether this is the material's default model.
    pub default: bool,
    /// Anything else a user should know.
    pub notes: String,
    #[serde(skip)]
    eval: Eval,
}

impl ConductivityModel {
    /// The conductivity, W/(m·K), at `temperature` (K) and, for a film, `thickness_um`; `None`
    /// takes the model's default.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] for a temperature or a thickness outside the model's
    /// range, or a thickness given to a model of bulk.
    pub fn conductivity(&self, temperature: Option<f64>, thickness_um: Option<f64>) -> Result<f64> {
        let invalid = |reason: String| crate::Error::invalid("thermal conductivity", reason);
        let within = |p: &Parameter, value: Option<f64>| -> Result<f64> {
            let v = value.unwrap_or(p.default);
            if !(v.is_finite() && v >= p.min && v <= p.max) {
                return Err(invalid(if p.min == p.max {
                    format!(
                        "{} is given at {} {} {} only, not {v}",
                        self.name, p.name, p.min, p.unit
                    )
                } else {
                    format!(
                        "{}'s {} runs from {} to {} {}, not {v}",
                        self.name, p.name, p.min, p.max, p.unit
                    )
                }));
            }
            Ok(v)
        };
        let t = within(&self.temperature, temperature)?;
        let d = match (&self.thickness, thickness_um) {
            (Some(p), d) => within(p, d)?,
            (None, None) => f64::INFINITY,
            (None, Some(_)) => {
                return Err(invalid(format!(
                    "{} is a model of bulk: it takes no thickness",
                    self.name
                )));
            }
        };
        Ok((self.eval)(t, d))
    }
}

impl Conductivity {
    /// The default model.
    pub fn default_model(&self) -> &ConductivityModel {
        self.models
            .iter()
            .find(|m| m.default)
            .unwrap_or(&self.models[0])
    }

    /// A model by its id, or the default for `None`.
    pub fn model(&self, id: Option<&str>) -> Option<&ConductivityModel> {
        match id {
            None => Some(self.default_model()),
            Some(id) => self.models.iter().find(|m| m.id == id),
        }
    }

    /// The model of a film, when the material has one: it takes the film's thickness.
    pub fn film(&self) -> Option<&ConductivityModel> {
        self.models.iter().find(|m| m.thickness.is_some())
    }
}

/// Every material with a thermal conductivity.
pub fn conductivities() -> Vec<Conductivity> {
    vec![
        silicon(),
        silica(),
        tungsten(),
        titanium(),
        platinum(),
        titanium_nitride(),
    ]
}

/// The conductivity of the material with this id (`"si"`) or formula as a stack names it
/// (`"Si"`, `"SiO2"`, `"W"`; subscripts as digits), in any case.
pub fn conductivity(name: &str) -> Option<Conductivity> {
    let plain = |s: &str| -> String {
        s.chars()
            .map(|c| match c {
                '₀'..='₉' => char::from(b'0' + (c as u32 - '₀' as u32) as u8),
                c => c,
            })
            .collect::<String>()
            .to_lowercase()
    };
    let name = plain(name);
    conductivities()
        .into_iter()
        .find(|c| c.id == name || plain(&c.formula) == name)
}

// ---------------------------------------------------------------------------------------------
// Tables, as a power law of T between rows
// ---------------------------------------------------------------------------------------------

/// k at T from a table's rows (T in K, k in W/(cm·K) as printed), as a power law between the
/// rows about T, in W/(m·K). T outside the rows takes the nearest end's power law.
pub(crate) fn power_law(rows: &[(f64, f64)], t: f64) -> f64 {
    let k = rows
        .windows(2)
        .position(|w| t <= w[1].0)
        .unwrap_or(rows.len() - 2);
    let ((t0, k0), (t1, k1)) = (rows[k], rows[k + 1]);
    let m = (k1 / k0).ln() / (t1 / t0).ln();
    100.0 * k0 * (t / t0).powf(m)
}

/// Silicon's recommended values from Ho, Powell & Liley's table at whole hundreds of kelvin
/// (and 250 K), W/(cm·K); the rows at 223.2, 273.2 … 973.2 K check them.
pub(crate) const HO_SI: [(f64, f64); 11] = [
    (200.0, 2.64),
    (250.0, 1.91),
    (300.0, 1.48),
    (350.0, 1.19),
    (400.0, 0.989),
    (500.0, 0.762),
    (600.0, 0.619),
    (700.0, 0.508),
    (800.0, 0.422),
    (900.0, 0.359),
    (1000.0, 0.312),
];
/// The rows of the same table between [`HO_SI`]'s.
pub(crate) const HO_SI_BETWEEN: [(f64, f64); 11] = [
    (223.2, 2.25),
    (273.2, 1.68),
    (298.2, 1.49),
    (323.2, 1.33),
    (373.2, 1.08),
    (473.2, 0.814),
    (573.2, 0.651),
    (673.2, 0.536),
    (773.2, 0.442),
    (873.2, 0.374),
    (973.2, 0.323),
];
/// Glassbrenner & Slack's representative values for high-purity single-crystal silicon, their
/// Table I, W/(cm·K), 50 to 1681 K (the melting point). Below 50 K the table leaves silicon out,
/// where its K depends on the sample's size.
pub(crate) const GS_SI: [(f64, f64); 26] = [
    (50.0, 26.0),
    (60.0, 21.0),
    (70.0, 17.0),
    (80.0, 13.9),
    (90.0, 11.4),
    (100.0, 9.5),
    (125.0, 6.0),
    (150.0, 4.20),
    (175.0, 3.25),
    (200.0, 2.66),
    (250.0, 1.95),
    (300.0, 1.56),
    (400.0, 1.05),
    (500.0, 0.80),
    (600.0, 0.64),
    (700.0, 0.52),
    (800.0, 0.43),
    (900.0, 0.356),
    (1000.0, 0.310),
    (1100.0, 0.280),
    (1200.0, 0.261),
    (1300.0, 0.248),
    (1400.0, 0.237),
    (1500.0, 0.227),
    (1600.0, 0.219),
    (1681.0, 0.216),
];
/// The same column as printed: a macron over the last digit (U+0304 after it) where the table
/// puts its bar, "rather uncertain".
#[rustfmt::skip]
pub(crate) const GS_SI_PRINTED: [&str; 26] = [
    "26.", "21.", "17.0", "13.9", "11.4", "9.5", "6.0", "4.20\u{304}", "3.25\u{304}",
    "2.66\u{304}", "1.95", "1.56", "1.05", "0.80", "0.64", "0.52", "0.43", "0.356\u{304}",
    "0.310\u{304}", "0.280\u{304}", "0.261\u{304}", "0.248\u{304}", "0.237\u{304}",
    "0.227\u{304}", "0.219\u{304}", "0.216\u{304}",
];
/// Tungsten's, W/(cm·K). The first digit of the rows from 123.2 to 250 K is faint on the scan
/// and read as 1, as the rows about them have it.
pub(crate) const HO_W: [(f64, f64); 11] = [
    (200.0, 1.85),
    (250.0, 1.80),
    (300.0, 1.74),
    (350.0, 1.67),
    (400.0, 1.59),
    (500.0, 1.46),
    (600.0, 1.37),
    (700.0, 1.30),
    (800.0, 1.25),
    (900.0, 1.21),
    (1000.0, 1.18),
];
/// Tungsten's rows between [`HO_W`]'s (298.2 K's 1.73 below 300 K's 1.74, as printed).
pub(crate) const HO_W_BETWEEN: [(f64, f64); 11] = [
    (223.2, 1.82),
    (273.2, 1.77),
    (298.2, 1.73),
    (323.2, 1.71),
    (373.2, 1.63),
    (473.2, 1.50),
    (573.2, 1.39),
    (673.2, 1.32),
    (773.2, 1.27),
    (873.2, 1.22),
    (973.2, 1.19),
];
/// Polycrystalline titanium's, W/(cm·K).
pub(crate) const HO_TI: [(f64, f64); 11] = [
    (200.0, 0.245),
    (250.0, 0.229),
    (300.0, 0.219),
    (350.0, 0.210),
    (400.0, 0.204),
    (500.0, 0.197),
    (600.0, 0.194),
    (700.0, 0.194),
    (800.0, 0.197),
    (900.0, 0.202),
    (1000.0, 0.207),
];
/// Titanium's rows between [`HO_TI`]'s.
pub(crate) const HO_TI_BETWEEN: [(f64, f64); 11] = [
    (223.2, 0.237),
    (273.2, 0.224),
    (298.2, 0.219),
    (323.2, 0.215),
    (373.2, 0.207),
    (473.2, 0.198),
    (573.2, 0.194),
    (673.2, 0.194),
    (773.2, 0.196),
    (873.2, 0.200),
    (973.2, 0.205),
];
/// Platinum's, W/(cm·K).
pub(crate) const HO_PT: [(f64, f64); 11] = [
    (200.0, 0.726),
    (250.0, 0.718),
    (300.0, 0.716),
    (350.0, 0.717),
    (400.0, 0.718),
    (500.0, 0.723),
    (600.0, 0.732),
    (700.0, 0.743),
    (800.0, 0.756),
    (900.0, 0.771),
    (1000.0, 0.787),
];
/// Platinum's rows between [`HO_PT`]'s.
pub(crate) const HO_PT_BETWEEN: [(f64, f64); 11] = [
    (223.2, 0.721),
    (273.2, 0.717),
    (298.2, 0.716),
    (323.2, 0.716),
    (373.2, 0.717),
    (473.2, 0.722),
    (573.2, 0.729),
    (673.2, 0.740),
    (773.2, 0.752),
    (873.2, 0.767),
    (973.2, 0.783),
];
/// Clear fused quartz's recommended values from Powell, Ho & Liley's Table 14, W/(cm·K),
/// without its 273 K row, which checks them.
pub(crate) const POWELL_SIO2: [(f64, f64); 12] = [
    (100.0, 0.0069),
    (150.0, 0.0095),
    (200.0, 0.0114),
    (250.0, 0.0128),
    (300.0, 0.0138),
    (350.0, 0.0145),
    (400.0, 0.0151),
    (500.0, 0.0162),
    (600.0, 0.0175),
    (700.0, 0.0192),
    (800.0, 0.0217),
    (900.0, 0.0248),
];
/// The 273 K row of the same column.
pub(crate) const POWELL_SIO2_BETWEEN: [(f64, f64); 1] = [(273.0, 0.0133)];

fn temperature(min: f64, max: f64, default: f64) -> Parameter {
    Parameter {
        symbol: "T".into(),
        name: "temperature".into(),
        unit: "K".into(),
        min,
        max,
        default,
    }
}

fn src(reference: &str, location: &str) -> Source {
    Source {
        reference: reference.into(),
        location: location.into(),
    }
}

fn rows_table(caption: &str, rows: &[&[(f64, f64)]]) -> CoefficientTable {
    let mut all: Vec<(f64, f64)> = rows.iter().flat_map(|r| r.iter().copied()).collect();
    all.sort_by(|a, b| a.0.total_cmp(&b.0));
    CoefficientTable {
        caption: caption.into(),
        columns: vec!["$T$ (K)".into(), r"$k$ (W cm$^{-1}$ K$^{-1}$)".into()],
        rows: all
            .into_iter()
            .map(|(t, k)| vec![format!("{t}"), format!("{k}")])
            .collect(),
    }
}

const POWER_LAW: &str =
    r"k(T) = k_i \, (T / T_i)^{\ln(k_{i+1}/k_i) / \ln(T_{i+1}/T_i)}, \quad T_i \le T \le T_{i+1}";
const POWER_LAW_SYMBOLS: &str = r"$T_i$, $k_i$: the table's rows at whole hundreds of kelvin (and 250 K), between which $k$ is a power law of $T$";

/// One of Ho et al.'s tables as a model.
fn ho_model(
    page: &str,
    caption: &str,
    accuracy: &str,
    notes: &str,
    rows: &[(f64, f64)],
    between: &[(f64, f64)],
    eval: Eval,
) -> ConductivityModel {
    ConductivityModel {
        id: "ho-1972".into(),
        name: "Ho, Powell & Liley 1972".into(),
        equation: POWER_LAW.into(),
        symbols: POWER_LAW_SYMBOLS.into(),
        coefficients: vec![rows_table(caption, &[rows, between])],
        temperature: temperature(rows[0].0, rows[rows.len() - 1].0, 300.0),
        thickness: None,
        accuracy: accuracy.into(),
        sources: vec![src("ho-powell-liley-1972", page)],
        default: true,
        notes: notes.into(),
        eval,
    }
}

// ---------------------------------------------------------------------------------------------
// Silicon
// ---------------------------------------------------------------------------------------------

/// Kremer et al.'s thermal resistance of natural silicon, 1/κ = a T + b T² (m·K/W), 220–320 K.
pub(crate) const KREMER: (f64, f64) = (1.60e-5, 2.37e-8);

fn kremer(t: f64, _: f64) -> f64 {
    1.0 / (KREMER.0 * t + KREMER.1 * t * t)
}

/// Ju & Goodson's effective mean free path of the phonons in silicon at 300 K, µm.
pub(crate) const JU_GOODSON_PATH_UM: f64 = 0.3;

/// The exponential integral E₁(x), x > 0: its series below 1, a continued fraction (evaluated
/// by Lentz's method) above.
pub(crate) fn e1(x: f64) -> f64 {
    const EULER: f64 = std::f64::consts::EULER_GAMMA;
    if x < 1.0 {
        // E₁ = −γ − ln x + Σ_{k≥1} (−1)^{k+1} x^k / (k k!)
        let (mut sum, mut term) = (0.0, 1.0);
        for k in 1..60 {
            let k = f64::from(k);
            term *= -x / k;
            let add = -term / k;
            sum += add;
            if add.abs() < 1e-17 * sum.abs() {
                break;
            }
        }
        -EULER - x.ln() + sum
    } else {
        // E₁ = e^{−x} / (x + 1 − 1² / (x + 3 − 2² / (x + 5 − …)))
        let tiny = 1e-300;
        let mut b = x + 1.0;
        let mut c = 1.0 / tiny;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..200 {
            let a = -f64::from(i * i);
            b += 2.0;
            d = 1.0 / (a * d + b);
            c = b + a / c;
            let step = c * d;
            h *= step;
            if (step - 1.0).abs() < 1e-16 {
                break;
            }
        }
        h * (-x).exp()
    }
}

/// Eₙ(x) for n = 1 … `n` by the recurrence E_{k+1} = (e^{−x} − x E_k) / k.
pub(crate) fn exponential_integrals(x: f64, n: usize) -> Vec<f64> {
    let mut e = vec![e1(x)];
    for k in 1..n {
        let next = ((-x).exp() - x * e[k - 1]) / k as f64;
        e.push(next);
    }
    e
}

/// Sondheimer's reduction of a film's conductivity along it, for diffuse surfaces, at δ =
/// thickness / mean free path: 1 − 3/(2δ) ∫₁^∞ (t⁻³ − t⁻⁵)(1 − e^{−δt}) dt, in closed form
/// 1 − 3/(2δ) (1/4 − E₃(δ) + E₅(δ)).
pub(crate) fn sondheimer(delta: f64) -> f64 {
    let e = exponential_integrals(delta, 5);
    1.0 - 1.5 / delta * (0.25 - e[2] + e[4])
}

fn ho_si(t: f64, _: f64) -> f64 {
    power_law(&HO_SI, t)
}

fn glassbrenner_slack(t: f64, _: f64) -> f64 {
    power_law(&GS_SI, t)
}

fn ju_goodson(_: f64, d: f64) -> f64 {
    power_law(&HO_SI, 300.0) * sondheimer(d / JU_GOODSON_PATH_UM)
}

fn silicon() -> Conductivity {
    Conductivity {
        id: "si".into(),
        name: "Silicon".into(),
        formula: "Si".into(),
        models: vec![
            ho_model(
                "p. 394, Thermal conductivity of silicon, recommended values",
                "Recommended values, solid (p. 394), 200–1000 K",
                "±5 % from 300 to 1000 K; below 300 K \"merely typical values\" (the table's footnote)",
                "high-purity bulk silicon; 148 W/(m·K) at 300 K. A film conducts less along it: the \
                 ju-goodson-1999 model",
                &HO_SI,
                &HO_SI_BETWEEN,
                ho_si,
            ),
            ConductivityModel {
                id: "kremer-2004".into(),
                name: "Kremer et al. 2004".into(),
                equation: r"1/\kappa = a T + b T^2".into(),
                symbols: r"$\kappa$ in W m$^{-1}$ K$^{-1}$, $T$ in K".into(),
                coefficients: vec![CoefficientTable {
                    caption: "natural silicon's thermal resistance, 220–320 K (p. 4 of the arXiv copy)"
                        .into(),
                    columns: vec!["$a$ (m W$^{-1}$)".into(), "$b$ (m W$^{-1}$ K$^{-1}$)".into()],
                    rows: vec![vec!["1.60(2)E-05".into(), "2.37(6)E-08".into()]],
                }],
                temperature: temperature(220.0, 320.0, 300.0),
                thickness: None,
                accuracy: "the coefficients' last digits ±2 and ±6; three laboratories within 3 % \
                           above 80 K"
                    .into(),
                sources: vec![src("kremer-2004", "Results and discussion, Fig. 2")],
                default: false,
                notes: "natural silicon, measured in three laboratories; about 3 % below the \
                        standard values at room temperature, the paper finds (144 W/(m·K) at 300 K)"
                    .into(),
                eval: kremer,
            },
            ConductivityModel {
                id: "glassbrenner-slack-1964".into(),
                name: "Glassbrenner & Slack 1964".into(),
                equation: POWER_LAW.into(),
                symbols: r"$T_i$, $k_i$: the table's rows, between which $k$ is a power law of $T$"
                    .into(),
                coefficients: vec![CoefficientTable {
                    caption: "Table I, silicon, 50–1681 K (a bar over the last digit: rather \
                              uncertain)"
                        .into(),
                    columns: vec!["$T$ (K)".into(), r"$k$ (W cm$^{-1}$ K$^{-1}$)".into()],
                    rows: GS_SI
                        .iter()
                        .zip(GS_SI_PRINTED)
                        .map(|((t, _), k)| vec![format!("{t}"), k.into()])
                        .collect(),
                }],
                temperature: temperature(50.0, 1681.0, 300.0),
                thickness: None,
                accuracy: "±5 % (Table I); two points on one sample ±2 % relative (Results); the \
                           rows with a bar \"rather uncertain\""
                    .into(),
                sources: vec![src("glassbrenner-slack-1964", "Table I and Results")],
                default: false,
                notes: "high-purity single crystals, measured from 4.3 to 304 K in a longitudinal \
                        and from 418 to 1577 K in a radial heat-flow apparatus, neither adjusted to \
                        the other; 156 W/(m·K) at 300 K, 5.4 % above Ho et al.'s. Down to 50 K \
                        and up to the melting point, where Ho et al.'s stop at 200 and 1000 K; \
                        above 1000 K electrons and holes carry part of the heat, 40 % at the \
                        melting point"
                    .into(),
                eval: glassbrenner_slack,
            },
            ConductivityModel {
                id: "ju-goodson-1999".into(),
                name: "Ju & Goodson 1999".into(),
                equation: r"k(d) = k_\mathrm{bulk} \left[ 1 - \frac{3}{2\delta} \left( \tfrac14 - E_3(\delta) + E_5(\delta) \right) \right], \quad \delta = d / \Lambda".into(),
                symbols: r"$d$ the film's thickness, $\Lambda = 0.3$ µm the effective mean free path at 300 K, $k_\mathrm{bulk} = 148$ W m$^{-1}$ K$^{-1}$ (Ho et al. at 300 K), $E_n$ the exponential integrals: Sondheimer's reduction for diffuse surfaces".into(),
                coefficients: vec![CoefficientTable {
                    caption: "the effective mean free path at room temperature (p. 3007)".into(),
                    columns: vec![r"$\Lambda$ (µm)".into(), "$T$ (K)".into()],
                    rows: vec![vec!["0.3".into(), "300".into()]],
                }],
                temperature: temperature(300.0, 300.0, 300.0),
                thickness: Some(Parameter {
                    symbol: "d".into(),
                    name: "film thickness".into(),
                    unit: "µm".into(),
                    min: 0.074,
                    max: 10.0,
                    default: 0.22,
                }),
                accuracy: "within 10 % of the paper's measured films from 143 to 247 nm (Fig. 2, \
                           ±15 % error bars); 24 to 26 % below its 78 and 85 nm films"
                    .into(),
                sources: vec![
                    src("ju-goodson-1999", "p. 3007, the effective mean free path; Fig. 2"),
                    src("ho-powell-liley-1972", "p. 394, silicon at 300 K"),
                ],
                default: false,
                notes: "a monocrystalline film's conductivity along it (in-plane), at 300 K only: \
                        90.7 W/(m·K) for 220 nm. Across the film it is lower still; the model \
                        gives the in-plane value for both"
                    .into(),
                eval: ju_goodson,
            },
        ],
        references: references(&[
            "ho-powell-liley-1972",
            "kremer-2004",
            "glassbrenner-slack-1964",
            "ju-goodson-1999",
        ]),
    }
}

// ---------------------------------------------------------------------------------------------
// Silica and the metals
// ---------------------------------------------------------------------------------------------

fn powell_sio2(t: f64, _: f64) -> f64 {
    power_law(&POWELL_SIO2, t)
}

fn silica() -> Conductivity {
    Conductivity {
        id: "sio2".into(),
        name: "Silica (fused)".into(),
        formula: "SiO₂".into(),
        models: vec![ConductivityModel {
            id: "powell-1966".into(),
            name: "Powell, Ho & Liley 1966".into(),
            equation: POWER_LAW.into(),
            symbols: r"$T_i$, $k_i$: the table's rows, between which $k$ is a power law of $T$"
                .into(),
            coefficients: vec![rows_table(
                "Table 14, quartz (clear fused), 100–900 K",
                &[&POWELL_SIO2, &POWELL_SIO2_BETWEEN],
            )],
            temperature: temperature(100.0, 900.0, 300.0),
            thickness: None,
            accuracy: "±4 % from 200 to 500 K, ±8 % at 50 and 900 K (p. 69)".into(),
            sources: vec![src("powell-ho-liley-1966", "Table 14 and p. 69")],
            default: true,
            notes: "high-purity (99.99+ %) clear fused quartz, the mean of 17 sets of data; 1.38 \
                    W/(m·K) at 300 K. Above about 900 K radiation adds to it"
                .into(),
            eval: powell_sio2,
        }],
        references: references(&["powell-ho-liley-1966"]),
    }
}

fn ho_w(t: f64, _: f64) -> f64 {
    power_law(&HO_W, t)
}

fn ho_ti(t: f64, _: f64) -> f64 {
    power_law(&HO_TI, t)
}

fn ho_pt(t: f64, _: f64) -> f64 {
    power_law(&HO_PT, t)
}

const METAL_NOTE: &str = "the pure, well-annealed metal; a sputtered or evaporated film, with \
                          its grain boundaries and impurities, conducts less";

fn tungsten() -> Conductivity {
    Conductivity {
        id: "w".into(),
        name: "Tungsten".into(),
        formula: "W".into(),
        models: vec![ho_model(
            "p. 408, Thermal conductivity of tungsten, recommended values",
            "Recommended values, solid (p. 408), 200–1000 K",
            "±3 % from 300 to 1500 K, ±5 % from 100 to 300 K (p. 408)",
            METAL_NOTE,
            &HO_W,
            &HO_W_BETWEEN,
            ho_w,
        )],
        references: references(&["ho-powell-liley-1972"]),
    }
}

fn titanium() -> Conductivity {
    Conductivity {
        id: "ti".into(),
        name: "Titanium".into(),
        formula: "Ti".into(),
        models: vec![ho_model(
            "p. 407, Thermal conductivity of titanium, recommended values",
            "Recommended values, solid, polycrystalline (p. 407), 200–1000 K",
            "±10 % at moderate temperatures (p. 407)",
            METAL_NOTE,
            &HO_TI,
            &HO_TI_BETWEEN,
            ho_ti,
        )],
        references: references(&["ho-powell-liley-1972"]),
    }
}

fn platinum() -> Conductivity {
    Conductivity {
        id: "pt".into(),
        name: "Platinum".into(),
        formula: "Pt".into(),
        models: vec![ho_model(
            "p. 379, Thermal conductivity of platinum, recommended values",
            "Recommended values, solid (p. 379), 200–1000 K",
            "±3 % near room temperature, ±6 % at about 1200 K (p. 379)",
            METAL_NOTE,
            &HO_PT,
            &HO_PT_BETWEEN,
            ho_pt,
        )],
        references: references(&["ho-powell-liley-1972"]),
    }
}

// ---------------------------------------------------------------------------------------------
// Titanium nitride, a film
// ---------------------------------------------------------------------------------------------

/// Goel et al.'s 86 nm films at room temperature, W/(m·K): plasma-enhanced and thermal ALD
/// (p. 619 and the abstract; the first is 26.95 in the conclusions, p. 621).
pub(crate) const GOEL_KAPPA: (f64, f64) = (26.96, 7.01);

/// Their films' thickness, µm.
pub(crate) const GOEL_THICKNESS_UM: f64 = 0.086;

fn goel_peald(_: f64, _: f64) -> f64 {
    GOEL_KAPPA.0
}

fn goel_thermal(_: f64, _: f64) -> f64 {
    GOEL_KAPPA.1
}

/// The films measured beside Goel et al.'s, sputtered and far lower: the spread, not averaged.
const TIN_SPREAD: &str = "a TiN film's conductivity depends on how it is made: V. Moraes et \
    al., J. Appl. Phys. 119, 225304 (2016), measure a 580 nm dc-sputtered film (500 °C, on \
    Si(100)) by the 3ω method at 3.1 W/(m·K) at room temperature, 2.15 of it phonons (abstract, \
    Table II); K. Jagannadham, J. Vac. Sci. Technol. A 33, 031514 (2015), seven films sputtered \
    at 300 to 600 °C onto silicon, 0.7 to 1.2 µm thick, at 0.53 to 4.7 W/(m·K) by transient \
    thermoreflectance, ±15 % (Table II, p. 031514-8). Jacques et al. 2019 take 28 W/(m·K) from a \
    handbook";

/// One of Goel et al.'s films as a model: an 86 nm film at room temperature (taken as 300 K).
fn goel_model(peald: bool) -> ConductivityModel {
    let (id, name, value, eval, how): (_, _, _, Eval, _) = if peald {
        (
            "goel-2026-peald",
            "Goel et al. 2026, plasma-enhanced ALD",
            GOEL_KAPPA.0,
            goel_peald,
            "plasma-enhanced ALD (TiCl₄ and NH₃, 2500 W inductive plasma, Picosun R200)",
        )
    } else {
        (
            "goel-2026-thermal-ald",
            "Goel et al. 2026, thermal ALD",
            GOEL_KAPPA.1,
            goel_thermal,
            "thermal ALD (TiCl₄ and NH₃, Picosun R200)",
        )
    };
    ConductivityModel {
        id: id.into(),
        name: name.into(),
        equation: format!(r"k = {value}\ \mathrm{{W\,m^{{-1}}\,K^{{-1}}}}"),
        symbols: "one measured value: an 86 nm film at room temperature".into(),
        coefficients: vec![CoefficientTable {
            caption: "the 86 nm films at room temperature (p. 619)".into(),
            columns: vec!["film".into(), r"$k$ (W m$^{-1}$ K$^{-1}$)".into()],
            rows: vec![
                vec!["plasma-enhanced ALD".into(), "26.96".into()],
                vec!["thermal ALD".into(), "7.01".into()],
            ],
        }],
        temperature: temperature(300.0, 300.0, 300.0),
        thickness: Some(Parameter {
            symbol: "d".into(),
            name: "film thickness".into(),
            unit: "µm".into(),
            min: GOEL_THICKNESS_UM,
            max: GOEL_THICKNESS_UM,
            default: GOEL_THICKNESS_UM,
        }),
        accuracy: "none stated (the fit is in the paper's supplement, not in hand); the value is \
                   printed to four figures"
            .into(),
        sources: vec![src(
            "goel-2026",
            "Section 2.3 (p. 619), the abstract; Section 3.4 (p. 620)",
        )],
        default: peald,
        notes: format!(
            "an 86 nm film by {how}, on Si or glass (the 86 nm films' growth temperature and cycle \
             count aren't given), by nanosecond transient thermoreflectance (Linseis TF-LFA, \
             1064 nm pump, 476 nm probe, Ti/Au transducer 17/176 nm), its multilayer fit taking \
             bulk TiN's density and heat capacity (5450 kg/m³, 636 J/(kg·K)); the paper calls the \
             result in-plane. ALD, not sputtered as a heater's TiN usually is. Its resistivity \
             (0.91e-4 Ω·cm, p. 616, for plasma-enhanced ALD at 400 °C) was measured on thinner \
             films (31 nm, Table 1), not on this one. Room temperature taken as 300 K. Beside \
             it, {TIN_SPREAD}"
        ),
        eval,
    }
}

fn titanium_nitride() -> Conductivity {
    Conductivity {
        id: "tin".into(),
        name: "Titanium nitride (film)".into(),
        formula: "TiN".into(),
        models: vec![goel_model(true), goel_model(false)],
        references: references(&["goel-2026"]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_exponential_integral_takes_its_known_values_either_side_of_one() {
        for (x, e) in [
            (0.5, 0.559_773_594_776_160_8),
            (1.0, 0.219_383_934_395_520_3),
            (2.0, 0.048_900_510_708_061_12),
        ] {
            assert!((e1(x) / e - 1.0).abs() < 1e-14, "{x}");
        }
    }

    #[test]
    fn tin_is_the_86_nm_film_at_room_temperature_only() {
        let tin = conductivity("TiN").unwrap();
        let peald = tin.default_model();
        assert_eq!(peald.id, "goel-2026-peald");
        assert_eq!(tin.film().unwrap().id, "goel-2026-peald");
        assert_eq!(peald.conductivity(None, None).unwrap(), 26.96);
        assert_eq!(peald.conductivity(Some(300.0), Some(0.086)).unwrap(), 26.96);
        assert!(peald.conductivity(Some(300.0), Some(0.1)).is_err());
        assert!(peald.conductivity(Some(350.0), None).is_err());
        let thermal = tin.model(Some("goel-2026-thermal-ald")).unwrap();
        assert_eq!(thermal.conductivity(None, None).unwrap(), 7.01);
        let o = super::super::conductivity_checks::goel_thermal_zt();
        assert!(o.passed() && (o.measured - 2.118e-4).abs() < 1e-7, "{o:?}");
        // the plasma-enhanced film's zT at 300 K: 0.0057 against the printed 0.0056
        assert!((512e-6_f64 * 300.0 / 26.96 - 0.005697).abs() < 1e-6);
    }

    #[test]
    fn a_material_is_found_by_id_or_by_formula_and_its_models_check_their_ranges() {
        for name in ["Si", "si", "SiO2", "SiO₂", "W", "ti", "Pt"] {
            assert!(conductivity(name).is_some(), "{name}");
        }
        assert!(conductivity("TiN").is_some() && conductivity("GaAs").is_none());
        let si = conductivity("Si").unwrap();
        assert_eq!(si.default_model().id, "ho-1972");
        assert_eq!(si.film().unwrap().id, "ju-goodson-1999");
        let bulk = si.default_model();
        assert!((bulk.conductivity(None, None).unwrap() - 148.0).abs() < 1e-12);
        assert!(bulk.conductivity(Some(150.0), None).is_err());
        assert!(bulk.conductivity(None, Some(0.22)).is_err());
        let film = si.film().unwrap();
        assert!(film.conductivity(Some(350.0), Some(0.22)).is_err());
        assert!(film.conductivity(None, Some(0.05)).is_err());
        // thicker films conduct more, toward the bulk
        let k = |d: f64| film.conductivity(None, Some(d)).unwrap();
        assert!(k(0.1) < k(0.22) && k(0.22) < k(1.0) && k(10.0) < 148.0);
        assert!(super::super::entry("si").unwrap().conductivity().is_some());
        assert!(
            super::super::entry("gaas")
                .unwrap()
                .conductivity()
                .is_none()
        );
        for c in conductivities() {
            for m in &c.models {
                for s in &m.sources {
                    assert!(
                        c.references.iter().any(|r| r.key == s.reference),
                        "{}",
                        m.id
                    );
                }
                let k = m
                    .conductivity(None, None)
                    .or_else(|_| m.conductivity(None, Some(0.22)));
                assert!(k.unwrap() > 0.0, "{}", m.id);
            }
        }
    }
}
