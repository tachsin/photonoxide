//! A [`ParametricModel`] as a file: JSON, every number to the last bit, which reads back to the
//! same model.
//!
//! ```json
//! {"format":1,"kind":"compact model","ports":["o1","o2"],
//!  "parameters":[{"name":"gap","unit":"µm","default":0.2,"min":0.1,"max":0.3}],
//!  "band_um":[1.5,1.6],"centre":[0.2],"half":[0.1],
//!  "weights":{"monomials":[[0],[1],[2]]},"symmetry":"complex","constant":true,
//!  "basis":[{"complex":[-0.01,-4.05]}, …],"numerator":[[…], …],"denominator":[…],
//!  "delay":0.0,"error":{…},"basis_error":{…},"source":"…"}
//! ```
//!
//! The numerator's and denominator's real unknowns are as the model keeps them: basis column j
//! and weight l at j L + l (L the weights), then the constant term's columns times the weights.

use serde::{Deserialize, Serialize};

use super::{ParametricModel, Weights, monomials};
use crate::circuit::{Parameter, ports};
use crate::compact::fit::{FitError, Pole, Symmetry};
use crate::units::Wavelength;
use crate::{Error, Result};

/// The format this module writes, and the only one it reads.
const FORMAT: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ModelFile {
    format: u32,
    kind: String,
    ports: Vec<String>,
    parameters: Vec<ParameterFile>,
    band_um: [f64; 2],
    centre: Vec<f64>,
    half: Vec<f64>,
    weights: WeightsFile,
    symmetry: SymmetryFile,
    constant: bool,
    basis: Vec<PoleFile>,
    numerator: Vec<Vec<f64>>,
    denominator: Vec<f64>,
    delay: f64,
    error: ErrorFile,
    basis_error: ErrorFile,
    source: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ParameterFile {
    name: String,
    unit: String,
    default: f64,
    min: f64,
    max: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum WeightsFile {
    /// Each monomial's exponents.
    Monomials(Vec<Vec<usize>>),
    /// Each parameter's nodes, increasing.
    Grid(Vec<Vec<f64>>),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SymmetryFile {
    Real,
    Complex,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum PoleFile {
    Real(f64),
    /// Re and Im of the pair's pole with Im > 0.
    Pair([f64; 2]),
    Complex([f64; 2]),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorFile {
    rms: f64,
    max: f64,
    rms_relative: f64,
    max_relative: f64,
}

impl From<FitError> for ErrorFile {
    fn from(e: FitError) -> ErrorFile {
        ErrorFile {
            rms: e.rms,
            max: e.max,
            rms_relative: e.rms_relative,
            max_relative: e.max_relative,
        }
    }
}

impl From<ErrorFile> for FitError {
    fn from(e: ErrorFile) -> FitError {
        FitError {
            rms: e.rms,
            max: e.max,
            rms_relative: e.rms_relative,
            max_relative: e.max_relative,
        }
    }
}

impl From<ParametricModel> for ModelFile {
    fn from(m: ParametricModel) -> ModelFile {
        let pair = |a: num_complex::Complex64| [a.re, a.im];
        ModelFile {
            format: FORMAT,
            kind: m.kind,
            ports: m.ports.iter().map(|p| p.name.clone()).collect(),
            parameters: m
                .parameters
                .into_iter()
                .map(|p| ParameterFile {
                    name: p.name,
                    unit: p.unit,
                    default: p.default,
                    min: p.min,
                    max: p.max,
                })
                .collect(),
            band_um: [m.band.0.to_um(), m.band.1.to_um()],
            centre: m.centre,
            half: m.half,
            weights: match m.weights {
                Weights::Monomials(e) => WeightsFile::Monomials(e),
                Weights::Grid(a) => WeightsFile::Grid(a),
            },
            symmetry: match m.symmetry {
                Symmetry::Real => SymmetryFile::Real,
                Symmetry::Complex => SymmetryFile::Complex,
            },
            constant: m.constant,
            basis: m
                .basis
                .iter()
                .map(|&p| match p {
                    Pole::Real(a) => PoleFile::Real(a),
                    Pole::Pair(a) => PoleFile::Pair(pair(a)),
                    Pole::Complex(a) => PoleFile::Complex(pair(a)),
                })
                .collect(),
            numerator: m.numerator,
            denominator: m.denominator,
            delay: m.delay,
            error: m.error.into(),
            basis_error: m.basis_error.into(),
            source: m.source,
        }
    }
}

impl TryFrom<ModelFile> for ParametricModel {
    type Error = Error;

    fn try_from(f: ModelFile) -> Result<ParametricModel> {
        let bad = |reason: String| Err(Error::invalid("parametric model file", reason));
        if f.format != FORMAT {
            return bad(format!(
                "is format {}; this photonoxide reads format {FORMAT}",
                f.format
            ));
        }
        let vars = f.parameters.len();
        if f.ports.is_empty() || f.centre.len() != vars || f.half.len() != vars {
            return bad("needs ports, and a centre and half-width for each parameter".into());
        }
        if f.half.iter().any(|h| !(h.is_finite() && *h > 0.0))
            || f.centre.iter().any(|c| !c.is_finite())
        {
            return bad("each parameter's range must be finite and of some width".into());
        }
        let (lo, hi) = (Wavelength::um(f.band_um[0])?, Wavelength::um(f.band_um[1])?);
        if hi < lo {
            return bad("its band runs backwards".into());
        }
        let (weights, count) = match f.weights {
            WeightsFile::Monomials(e) => {
                // every monomial up to the degree, in their order: the only ones a fit makes
                let degree = e.iter().map(|x| x.iter().sum::<usize>()).max().unwrap_or(0);
                if e != monomials(vars, degree) {
                    return bad(format!(
                        "its monomials aren't every one of degree up to {degree} in {vars} parameters"
                    ));
                }
                let n = e.len();
                (Weights::Monomials(e), n)
            }
            WeightsFile::Grid(axes) => {
                if axes.len() != vars
                    || axes.iter().any(|a| {
                        a.len() < 2
                            || a.windows(2)
                                .any(|w| w[0].partial_cmp(&w[1]) != Some(std::cmp::Ordering::Less))
                    })
                {
                    return bad(
                        "a grid needs two or more increasing nodes for each parameter".into(),
                    );
                }
                let n = axes.iter().map(Vec::len).product();
                (Weights::Grid(axes), n)
            }
        };
        let symmetry = match f.symmetry {
            SymmetryFile::Real => Symmetry::Real,
            SymmetryFile::Complex => Symmetry::Complex,
        };
        let c = |p: [f64; 2]| num_complex::Complex64::new(p[0], p[1]);
        let basis: Vec<Pole> = f
            .basis
            .iter()
            .map(|p| match *p {
                PoleFile::Real(a) => Pole::Real(a),
                PoleFile::Pair(a) => Pole::Pair(c(a)),
                PoleFile::Complex(a) => Pole::Complex(c(a)),
            })
            .collect();
        let n_den: usize = basis.iter().map(|p| p.unknowns()).sum();
        let n_terms = usize::from(f.constant) * if symmetry == Symmetry::Complex { 2 } else { 1 };
        let responses = f.ports.len() * f.ports.len();
        if f.denominator.len() != n_den * count
            || f.numerator.len() != responses
            || f.numerator
                .iter()
                .any(|x| x.len() != (n_den + n_terms) * count)
        {
            return bad(format!(
                "its coefficients don't fit {} basis poles, {count} weights and {responses} responses",
                basis.len()
            ));
        }
        let names: Vec<&str> = f.ports.iter().map(String::as_str).collect();
        Ok(ParametricModel {
            kind: f.kind,
            ports: ports(&names),
            parameters: f
                .parameters
                .into_iter()
                .map(|p| Parameter::new(p.name, p.unit, p.default, p.min, p.max))
                .collect(),
            band: (lo, hi),
            centre: f.centre,
            half: f.half,
            weights,
            symmetry,
            constant: f.constant,
            basis,
            numerator: f.numerator,
            denominator: f.denominator,
            delay: f.delay,
            error: f.error.into(),
            basis_error: f.basis_error.into(),
            source: f.source,
        })
    }
}

impl ParametricModel {
    /// The model as JSON ([`ParametricModel::from_json`] reads it back to the same model, every
    /// number to the last bit).
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for a number JSON can't hold (an infinite error).
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| Error::Parse {
            what: "parametric model".into(),
            reason: e.to_string(),
        })
    }

    /// The model `text` holds, as [`ParametricModel::to_json`] writes it.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for text that isn't one: not JSON, an unknown field, a format other than
    /// this one's, coefficients that don't fit their basis, weights and ports.
    pub fn from_json(text: &str) -> Result<ParametricModel> {
        serde_json::from_str(text).map_err(|e| Error::Parse {
            what: "parametric model".into(),
            reason: e.to_string(),
        })
    }
}
