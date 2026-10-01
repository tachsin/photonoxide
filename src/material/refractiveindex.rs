//! Reading the refractiveindex.info database's material files.
//!
//! The database (M. N. Polyanskiy, Sci. Data 11, 94 (2024),
//! [doi:10.1038/s41597-023-02898-2](https://doi.org/10.1038/s41597-023-02898-2); public domain,
//! CC0 1.0) stores each material as a YAML file: its `REFERENCES`, `COMMENTS`, `CONDITIONS`, and
//! one or two `DATA` entries. An entry is a dispersion formula (`formula 1` to `formula 9`, with
//! `coefficients` and a `wavelength_range`), or measured values (`tabulated n`, `tabulated k`,
//! `tabulated nk`, one wavelength per line). The formulas are those of the database's document
//! "Dispersion formulas" (refractiveindex.info, 2014-06-29), implemented here as it writes them.

use yaml_rust2::{Yaml, YamlLoader};

use super::{CC0, Curve, Material, Model, Provenance, Table};
use crate::units::Wavelength;
use crate::{Error, Result};

/// The coefficient C_i (1-based), zero when the file doesn't give it.
fn c(coefficients: &[f64], i: usize) -> f64 {
    coefficients.get(i - 1).copied().unwrap_or(0.0)
}

/// The sum over the pairs (C_a, C_b), (C_a+2, C_b+2), … up to C_last, of `term(C_a, C_b)`,
/// skipping pairs whose first coefficient is zero (so missing terms never divide 0 by 0).
fn pairs(coefficients: &[f64], first: usize, last: usize, term: impl Fn(f64, f64) -> f64) -> f64 {
    (first..last)
        .step_by(2)
        .map(|i| (c(coefficients, i), c(coefficients, i + 1)))
        .filter(|&(a, _)| a != 0.0)
        .map(|(a, b)| term(a, b))
        .sum()
}

/// The relative permittivity n² of formula `number` at λ (µm), as "Dispersion formulas" defines
/// it. An unknown number gives NaN (the importer rejects it first).
pub(super) fn formula(number: u8, coefficients: &[f64], lam: f64) -> f64 {
    let cs = coefficients;
    let l2 = lam * lam;
    match number {
        // 1, Sellmeier: n² − 1 = C1 + Σ C_i λ² / (λ² − C_{i+1}²), i = 2, 4, …, 16
        1 => 1.0 + c(cs, 1) + pairs(cs, 2, 17, |a, b| a * l2 / (l2 - b * b)),
        // 2, Sellmeier-2: n² − 1 = C1 + Σ C_i λ² / (λ² − C_{i+1})
        2 => 1.0 + c(cs, 1) + pairs(cs, 2, 17, |a, b| a * l2 / (l2 - b)),
        // 3, polynomial: n² = C1 + Σ C_i λ^C_{i+1}
        3 => c(cs, 1) + pairs(cs, 2, 17, |a, b| a * lam.powf(b)),
        // 4, RefractiveIndex.INFO: n² = C1 + C2 λ^C3 / (λ² − C4^C5) + C6 λ^C7 / (λ² − C8^C9)
        //    + Σ C_i λ^C_{i+1}, i = 10, …, 16
        4 => {
            let pole = |a: usize| {
                let k = c(cs, a);
                if k == 0.0 {
                    0.0
                } else {
                    k * lam.powf(c(cs, a + 1)) / (l2 - c(cs, a + 2).powf(c(cs, a + 3)))
                }
            };
            c(cs, 1) + pole(2) + pole(6) + pairs(cs, 10, 17, |a, b| a * lam.powf(b))
        }
        // 5, Cauchy: n = C1 + Σ C_i λ^C_{i+1}, i = 2, …, 10
        5 => {
            let n = c(cs, 1) + pairs(cs, 2, 11, |a, b| a * lam.powf(b));
            n * n
        }
        // 6, gases: n − 1 = C1 + Σ C_i / (C_{i+1} − λ⁻²), i = 2, …, 10
        6 => {
            let n = 1.0 + c(cs, 1) + pairs(cs, 2, 11, |a, b| a / (b - 1.0 / l2));
            n * n
        }
        // 7, Herzberger: n = C1 + C2/(λ² − 0.028) + C3 (1/(λ² − 0.028))² + C4 λ² + C5 λ⁴ + C6 λ⁶
        7 => {
            let q = 1.0 / (l2 - 0.028);
            let n = c(cs, 1)
                + c(cs, 2) * q
                + c(cs, 3) * q * q
                + c(cs, 4) * l2
                + c(cs, 5) * l2 * l2
                + c(cs, 6) * l2 * l2 * l2;
            n * n
        }
        // 8, retro: (n² − 1)/(n² + 2) = C1 + C2 λ²/(λ² − C3) + C4 λ², so n² = (1 + 2R)/(1 − R)
        8 => {
            let r = c(cs, 1) + c(cs, 2) * l2 / (l2 - c(cs, 3)) + c(cs, 4) * l2;
            (1.0 + 2.0 * r) / (1.0 - r)
        }
        // 9, exotic: n² = C1 + C2/(λ² − C3) + C4 (λ − C5) / ((λ − C5)² + C6)
        9 => {
            let d = lam - c(cs, 5);
            c(cs, 1) + c(cs, 2) / (l2 - c(cs, 3)) + c(cs, 4) * d / (d * d + c(cs, 6))
        }
        _ => f64::NAN,
    }
}

fn bad(source: &str, reason: impl Into<String>) -> Error {
    Error::Parse {
        what: source.to_owned(),
        reason: reason.into(),
    }
}

/// Numbers separated by whitespace.
fn numbers(source: &str, text: &str) -> Result<Vec<f64>> {
    text.split_whitespace()
        .map(|t| {
            t.parse::<f64>()
                .map_err(|_| bad(source, format!("{t:?} isn't a number")))
        })
        .collect()
}

/// A YAML scalar as text: a string, or a number written out.
fn text(y: &Yaml) -> Option<String> {
    match y {
        Yaml::String(s) | Yaml::Real(s) => Some(s.clone()),
        Yaml::Integer(i) => Some(i.to_string()),
        _ => None,
    }
}

/// The text without HTML tags, with runs of whitespace as one space.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The first DOI in a text: "10.", a registrant code, "/", then everything up to a space, a
/// quote, an angle bracket or a closing parenthesis at the end.
fn doi(text: &str) -> Option<String> {
    let start = text.find("10.")?;
    let rest = &text[start..];
    let slash = rest.find('/')?;
    if slash < 4
        || !rest[3..slash]
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.')
    {
        return doi(&rest[3..]);
    }
    let end = rest
        .find(|c: char| c.is_whitespace() || matches!(c, '"' | '<' | '>'))
        .unwrap_or(rest.len());
    let found = rest[..end].trim_end_matches(['.', ',', ';', ')', ']']);
    Some(found.to_owned())
}

/// One `DATA` entry.
enum Entry {
    Formula {
        number: u8,
        coefficients: Vec<f64>,
        range: (f64, f64),
    },
    /// columns: wavelength, then n, k, or n and k
    Tabulated { kind: String, rows: Vec<Vec<f64>> },
}

fn entry(source: &str, y: &Yaml) -> Result<Entry> {
    let kind = y["type"]
        .as_str()
        .ok_or_else(|| bad(source, "a DATA entry has no type"))?;
    if let Some(number) = kind.strip_prefix("formula ") {
        let number: u8 = number
            .trim()
            .parse()
            .ok()
            .filter(|n| (1..=9).contains(n))
            .ok_or_else(|| bad(source, format!("unknown formula {kind:?}")))?;
        let coefficients = numbers(
            source,
            &text(&y["coefficients"])
                .ok_or_else(|| bad(source, "a formula has no coefficients"))?,
        )?;
        let range = numbers(
            source,
            &text(&y["wavelength_range"])
                .ok_or_else(|| bad(source, "a formula has no wavelength_range"))?,
        )?;
        let [lo, hi] = range[..] else {
            return Err(bad(source, "wavelength_range needs two numbers"));
        };
        return Ok(Entry::Formula {
            number,
            coefficients,
            range: (lo, hi),
        });
    }
    let columns = match kind {
        "tabulated n" | "tabulated k" => 2,
        "tabulated nk" => 3,
        other => return Err(bad(source, format!("unsupported DATA type {other:?}"))),
    };
    let data = y["data"]
        .as_str()
        .ok_or_else(|| bad(source, format!("{kind} has no data")))?;
    let rows = data
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let row = numbers(source, l)?;
            if row.len() == columns {
                Ok(row)
            } else {
                Err(bad(
                    source,
                    format!("{kind} needs {columns} numbers per line, got {l:?}"),
                ))
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Entry::Tabulated {
        kind: kind.to_owned(),
        rows,
    })
}

fn column(rows: &[Vec<f64>], i: usize) -> Vec<f64> {
    rows.iter().map(|r| r[i]).collect()
}

/// A material from the text of a refractiveindex.info database file. `source` says where the
/// file came from (e.g. `"main/SiO2/nk/Malitson.yml"`) and goes into the provenance with the
/// database's citation. The range is the formula's `wavelength_range`, or the span of the
/// table; with a formula for n and a table for k, where both hold.
///
/// # Errors
///
/// [`Error::Parse`] if the text isn't such a file, or uses a kind of data not supported yet,
/// and [`Error::InvalidValue`] for a table that isn't valid (see [`Table::new`]) or a range
/// that is empty.
pub fn from_refractiveindex_info(name: &str, yaml: &str, source: &str) -> Result<Material> {
    let docs = YamlLoader::load_from_str(yaml).map_err(|e| bad(source, e.to_string()))?;
    let doc = docs
        .first()
        .ok_or_else(|| bad(source, "the file is empty"))?;
    let entries = doc["DATA"]
        .as_vec()
        .ok_or_else(|| bad(source, "the file has no DATA list"))?
        .iter()
        .map(|y| entry(source, y))
        .collect::<Result<Vec<_>>>()?;

    let n_model = |e: &Entry| -> Result<Option<(Model, (f64, f64))>> {
        Ok(match e {
            Entry::Formula {
                number,
                coefficients,
                range,
            } => Some((
                Model::Formula {
                    number: *number,
                    coefficients: coefficients.clone(),
                },
                *range,
            )),
            Entry::Tabulated { kind, rows } if kind == "tabulated n" || kind == "tabulated nk" => {
                let w = column(rows, 0);
                let span = (w[0], w[w.len() - 1]);
                let k = (kind == "tabulated nk").then(|| column(rows, 2));
                Some((Model::Tabulated(Table::new(w, column(rows, 1), k)?), span))
            }
            _ => None,
        })
    };
    let (model, (lo, hi)) = match &entries[..] {
        [one] => n_model(one)?.ok_or_else(|| bad(source, "the file has k but no n"))?,
        [first, Entry::Tabulated { kind, rows }] if kind == "tabulated k" => {
            let (n, (lo, hi)) =
                n_model(first)?.ok_or_else(|| bad(source, "the file has k twice and no n"))?;
            let w = column(rows, 0);
            let span = (w[0], w[w.len() - 1]);
            let k = Curve::new(w, column(rows, 1))?;
            (
                Model::WithExtinction { n: Box::new(n), k },
                (lo.max(span.0), hi.min(span.1)),
            )
        }
        _ => {
            return Err(bad(
                source,
                format!(
                    "{} DATA entries: supported are one, or an n model and tabulated k",
                    entries.len()
                ),
            ));
        }
    };

    let references = text(&doc["REFERENCES"]).unwrap_or_default();
    let temperature = doc["CONDITIONS"]["temperature"]
        .as_f64()
        .or_else(|| doc["CONDITIONS"]["temperature"].as_i64().map(|t| t as f64));
    let provenance = Provenance {
        reference: plain(&references),
        doi: doi(&references).unwrap_or_default(),
        data: format!("{CC0}: {source}"),
        temperature,
        notes: plain(&text(&doc["COMMENTS"]).unwrap_or_default()),
    };
    Material::new(
        name,
        model,
        Wavelength::um(lo)?,
        Wavelength::um(hi)?,
        provenance,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{silica, silicon, silicon_nitride};

    // three files of the database, verbatim (CC0)
    const MALITSON: &str = r#"# this file is part of refractiveindex.info database
# refractiveindex.info database is in the public domain
# copyright and related rights waived via CC0 1.0

REFERENCES: |
    1) I. H. Malitson.
    Interspecimen comparison of the refractive index of fused silica.
    <a href="https://doi.org/10.1364/JOSA.55.001205"><i>J. Opt. Soc. Am.</i> <b>55</b>, 1205-1208 (1965)</a><br>
    2) C. Z. Tan.
    Determination of refractive index of silica glass for infrared wavelengths by IR spectroscopy.
    <a href="https://doi.org/10.1016/S0022-3093(97)00438-9"><i>J. Non-Cryst. Solids</i> <b>223</b>, 158-163 (1998)</a><br>
    * Sellmeier formula is reported in Ref. 1 for the 0.21–3.71 μm wavelength range. Ref. 2 verifies the validity of the formula up to 6.7 μm.
COMMENTS: |
    Fused silica, 20 °C
DATA:
  - type: formula 1
    wavelength_range: 0.21 6.7
    coefficients: 0 0.6961663 0.0684043 0.4079426 0.1162414 0.8974794 9.896161
CONDITIONS:
    temperature: 293
"#;

    const LUKE: &str = r#"REFERENCES: |
    K. Luke, Y. Okawachi, M. R. E. Lamont, A. L. Gaeta, M. Lipson.
    Broadband mid-infrared frequency comb generation in a Si<sub>3</sub>N<sub>4</sub> microresonator.
    <a href="https://doi.org/10.1364/OL.40.004823"><i>Opt. Lett.</i> <b>40</b>, 4823-4826 (2015)</a>
COMMENTS: |
    340 nm Si<sub>3</sub>N<sub>4</sub> on 3.1 μm of thermal SiO<sub>2</sub> on silicon.
DATA:
  - type: formula 1
    wavelength_range: 0.310 5.504
    coefficients: 0 3.0249 0.1353406 40314 1239.842
"#;

    const LI_START: &str = r#"REFERENCES: |
    H. H. Li.
    Refractive index of silicon and germanium and its wavelength and temperature derivatives.
    <a href="https://doi.org/10.1063/1.555624"><i>J. Phys. Chem. Ref. Data</i> <b>9</b>, 561-658 (1993)</a>
COMMENTS: |
    293 K (20 °C)
DATA:
  - type: tabulated n
    data: |
        1.20 3.5167
        1.22 3.5133
        1.24 3.5102
        1.26 3.5072
        1.28 3.5043
        1.30 3.5016
        1.32 3.4990
        1.34 3.4965
        1.36 3.4941
        1.38 3.4918
        1.40 3.4896
        1.45 3.4845
        1.50 3.4799
        1.55 3.4757
        1.60 3.4719
        1.65 3.4684
        1.70 3.4653
        1.80 3.4597
        1.90 3.4550
        2.00 3.4510
        2.25 3.4431
        2.50 3.4375
        2.75 3.4334
        3.00 3.4302
        4.00 3.4229
        5.00 3.4195
        6.00 3.4177
        7.00 3.4165
        8.00 3.4158
        9.00 3.4153
        10.0 3.4150
        11.0 3.4147
        12.0 3.4145
        13.0 3.4144
        14.0 3.4142
"#;

    fn lam(v: f64) -> Wavelength {
        Wavelength::um(v).unwrap()
    }

    #[test]
    fn the_imported_files_match_the_built_in_materials() {
        let pairs = [
            (
                from_refractiveindex_info("SiO2", MALITSON, "main/SiO2/nk/Malitson.yml").unwrap(),
                silica(),
            ),
            (
                from_refractiveindex_info("Si3N4", LUKE, "main/Si3N4/nk/Luke.yml").unwrap(),
                silicon_nitride(),
            ),
            (
                from_refractiveindex_info("Si", LI_START, "main/Si/nk/Li-293K.yml").unwrap(),
                silicon(),
            ),
        ];
        for (imported, built_in) in &pairs {
            for at in [1.31, 1.55, 2.0, 3.0] {
                let a = imported.refractive_index(lam(at)).unwrap();
                let b = built_in.refractive_index(lam(at)).unwrap();
                assert!(
                    (a - b).norm() < 1e-14,
                    "{} at {at}: {a} vs {b}",
                    built_in.name()
                );
            }
        }
        // the file's own range: Malitson's formula extended to 6.7 um by the database
        assert_eq!(pairs[0].0.range().1.to_um(), 6.7);
        assert_eq!(pairs[2].0.range().0.to_um(), 1.2);
    }

    #[test]
    fn the_provenance_comes_from_the_file() {
        let m = from_refractiveindex_info("SiO2", MALITSON, "main/SiO2/nk/Malitson.yml").unwrap();
        let p = m.provenance();
        assert_eq!(p.doi, "10.1364/JOSA.55.001205");
        assert!(
            p.reference.starts_with("1) I. H. Malitson."),
            "{}",
            p.reference
        );
        assert!(!p.reference.contains('<'), "{}", p.reference);
        assert_eq!(p.temperature, Some(293.0));
        assert_eq!(p.notes, "Fused silica, 20 °C");
        assert!(p.data.contains("CC0") && p.data.ends_with("main/SiO2/nk/Malitson.yml"));
        let luke = from_refractiveindex_info("Si3N4", LUKE, "Luke.yml").unwrap();
        assert_eq!(
            luke.provenance().notes,
            "340 nm Si 3 N 4 on 3.1 μm of thermal SiO 2 on silicon."
        );
    }

    #[test]
    fn each_formula_follows_its_definition() {
        let l: f64 = 1.3;
        let l2 = l * l;
        let c = [0.5, 1.2, 0.1, 0.3, 0.2, 0.05, 2.0, 0.01, 1.5, 0.002, 2.0];
        // 1: n² − 1 = C1 + C2λ²/(λ²−C3²) + C4λ²/(λ²−C5²) + C6λ²/(λ²−C7²) + C8λ²/(λ²−C9²) + C10λ²/(λ²−C11²)
        let f1 = 1.0
            + 0.5
            + 1.2 * l2 / (l2 - 0.01)
            + 0.3 * l2 / (l2 - 0.04)
            + 0.05 * l2 / (l2 - 4.0)
            + 0.01 * l2 / (l2 - 2.25)
            + 0.002 * l2 / (l2 - 4.0);
        assert!((formula(1, &c, l) - f1).abs() < 1e-14);
        // 2: the same with C_{i+1} not squared
        let f2 = 1.0
            + 0.5
            + 1.2 * l2 / (l2 - 0.1)
            + 0.3 * l2 / (l2 - 0.2)
            + 0.05 * l2 / (l2 - 2.0)
            + 0.01 * l2 / (l2 - 1.5)
            + 0.002 * l2 / (l2 - 2.0);
        assert!((formula(2, &c, l) - f2).abs() < 1e-14);
        // 3: n² = C1 + C2 λ^C3 + …
        let f3 = 0.5
            + 1.2 * l.powf(0.1)
            + 0.3 * l.powf(0.2)
            + 0.05 * l.powf(2.0)
            + 0.01 * l.powf(1.5)
            + 0.002 * l.powf(2.0);
        assert!((formula(3, &c, l) - f3).abs() < 1e-14);
        // 4: n² = C1 + C2λ^C3/(λ² − C4^C5) + C6λ^C7/(λ² − C8^C9) + C10 λ^C11
        let f4 = 0.5
            + 1.2 * l.powf(0.1) / (l2 - 0.3f64.powf(0.2))
            + 0.05 * l.powf(2.0) / (l2 - 0.01f64.powf(1.5))
            + 0.002 * l.powf(2.0);
        assert!((formula(4, &c, l) - f4).abs() < 1e-14);
        // 5: n = C1 + C2 λ^C3 + … + C10 λ^C11
        let n5 = 0.5
            + 1.2 * l.powf(0.1)
            + 0.3 * l.powf(0.2)
            + 0.05 * l.powf(2.0)
            + 0.01 * l.powf(1.5)
            + 0.002 * l.powf(2.0);
        assert!((formula(5, &c, l) - n5 * n5).abs() < 1e-13);
        // 6: n − 1 = C1 + C2/(C3 − λ⁻²) + …
        let g = 1.0 / l2;
        let n6 = 1.0
            + 0.5
            + 1.2 / (0.1 - g)
            + 0.3 / (0.2 - g)
            + 0.05 / (2.0 - g)
            + 0.01 / (1.5 - g)
            + 0.002 / (2.0 - g);
        assert!((formula(6, &c, l) - n6 * n6).abs() < 1e-12);
        // 7: n = C1 + C2/(λ² − 0.028) + C3/(λ² − 0.028)² + C4λ² + C5λ⁴ + C6λ⁶
        let q = 1.0 / (l2 - 0.028);
        let n7 = 0.5 + 1.2 * q + 0.1 * q * q + 0.3 * l2 + 0.2 * l2 * l2 + 0.05 * l2 * l2 * l2;
        assert!((formula(7, &c, l) - n7 * n7).abs() < 1e-12);
        // 8: (n² − 1)/(n² + 2) = C1 + C2λ²/(λ² − C3) + C4λ²
        let r = 0.05 + 0.02 * l2 / (l2 - 0.01) + 0.001 * l2;
        let n2 = formula(8, &[0.05, 0.02, 0.01, 0.001], l);
        assert!(((n2 - 1.0) / (n2 + 2.0) - r).abs() < 1e-14);
        // 9: n² = C1 + C2/(λ² − C3) + C4(λ − C5)/((λ − C5)² + C6)
        let f9 = 0.5 + 1.2 / (l2 - 0.1) + 0.3 * (l - 0.2) / ((l - 0.2) * (l - 0.2) + 0.05);
        assert!((formula(9, &c, l) - f9).abs() < 1e-14);
        assert!(formula(10, &c, l).is_nan());
    }

    #[test]
    fn missing_coefficients_are_zero_and_never_divide_zero_by_zero() {
        // formula 4 with C1 only: the pole terms have C2 = C6 = 0, at lambda = 1 where
        // lambda² − 0⁰ = 0
        assert_eq!(formula(4, &[2.25], 1.0), 2.25);
        assert_eq!(formula(1, &[], 1.0), 1.0);
    }

    #[test]
    fn a_formula_for_n_and_a_table_for_k_combine() {
        let yaml = "DATA:\n  - type: formula 1\n    wavelength_range: 0.5 2.0\n    coefficients: 0 1.0 0.1\n  - type: tabulated k\n    data: |\n        0.6 0.01\n        1.0 0.02\n        1.8 0.03\n";
        let m = from_refractiveindex_info("lossy", yaml, "test").unwrap();
        // the range is where both hold
        let (lo, hi) = m.range();
        assert_eq!((lo.to_um(), hi.to_um()), (0.6, 1.8));
        let n = m.refractive_index(lam(1.0)).unwrap();
        let expected = (1.0 + 1.0 / (1.0 - 0.01f64)).sqrt();
        assert!(
            (n.re - expected).abs() < 1e-14 && (n.im - 0.02).abs() < 1e-14,
            "{n}"
        );
        assert!(m.permittivity(lam(1.0)).unwrap().im > 0.0);
    }

    #[test]
    fn tabulated_nk_has_loss() {
        let yaml = "DATA:\n  - type: tabulated nk\n    data: |\n        0.5 1.5 0.1\n        1.0 1.4 0.2\n        1.5 1.3 0.3\n";
        let m = from_refractiveindex_info("nk", yaml, "test").unwrap();
        let n = m.refractive_index(lam(1.0)).unwrap();
        assert!((n.re - 1.4).abs() < 1e-14 && (n.im - 0.2).abs() < 1e-14);
    }

    #[test]
    fn unsupported_or_broken_files_are_errors() {
        for (yaml, says) in [
            ("REFERENCES: x\n", "no DATA"),
            (
                "DATA:\n  - type: formula 12\n    wavelength_range: 1 2\n    coefficients: 1\n",
                "unknown formula",
            ),
            (
                "DATA:\n  - type: formula 1\n    coefficients: 1\n",
                "wavelength_range",
            ),
            (
                "DATA:\n  - type: tabulated k\n    data: |\n        1 0.1\n        2 0.1\n",
                "no n",
            ),
            (
                "DATA:\n  - type: tabulated n\n    data: |\n        1 1.5 0.1\n",
                "2 numbers",
            ),
            (
                "DATA:\n  - type: tabulated n\n    data: |\n        1 x\n",
                "isn't a number",
            ),
            ("DATA:\n  - type: model x\n", "unsupported"),
            ("DATA: [\n", "test"),
        ] {
            let e = from_refractiveindex_info("x", yaml, "test").unwrap_err();
            assert!(e.to_string().contains(says), "{says}: {e}");
        }
    }

    #[test]
    fn dois_are_found_in_references() {
        assert_eq!(
            doi("<a href=\"https://doi.org/10.1063/1.555624\"><i>J.</i>").as_deref(),
            Some("10.1063/1.555624")
        );
        assert_eq!(
            doi("see doi:10.1016/S0022-3093(97)00438-9.").as_deref(),
            Some("10.1016/S0022-3093(97)00438-9")
        );
        assert_eq!(doi("version 10.5 of it").as_deref(), None);
    }
}
