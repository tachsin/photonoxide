//! What the active devices share: the component a drive's phase gives ([`Driven`]), and a
//! measured device's file ([`Measurement`]), the fidelity a device is measured at.

use std::f64::consts::LN_10;
use std::path::Path;

use num_complex::Complex64 as c64;
use toml::{Table, Value};

use crate::circuit::{Component, Fidelity, Parameter, Port, Provenance, SMatrix};
use crate::units::Wavelength;
use crate::{Error, Result};

use super::{Figure, Range, Values};

/// A component whose transmission a drive sets: ports `o1` and `o2`, parameters `phase` (rad,
/// at the design's wavelength λ₀) and `loss` (dB, the insertion loss), reciprocal,
/// S₂₁ = S₁₂ = 10^(−loss/20) e^(iφλ₀/λ): the phase a drive gives a guide moves as 1/λ, its index
/// change held. A heater's, its phase the device's model's.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Driven {
    kind: String,
    wavelength_um: f64,
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
    provenance: Provenance,
}

impl Driven {
    /// A component of kind `kind`, its phase stated at `wavelength_um`, from `provenance`.
    pub(crate) fn new(kind: &str, wavelength_um: f64, provenance: Provenance) -> Driven {
        Driven {
            kind: kind.to_owned(),
            wavelength_um,
            ports: crate::circuit::ports(&["o1", "o2"]),
            parameters: vec![
                Parameter::new("phase", "rad", 0.0, -1e6, 1e6),
                Parameter::new("loss", "dB", 0.0, 0.0, 1e3),
            ],
            provenance,
        }
    }

    /// The transmission and its derivatives in the phase and the loss.
    fn transmission(&self, wavelength: Wavelength, values: &[f64]) -> Result<[c64; 3]> {
        if values.len() != 2
            || !self
                .parameters
                .iter()
                .zip(values)
                .all(|(p, &v)| p.allows(v))
        {
            return Err(Error::invalid(
                "driven component",
                format!(
                    "a {} takes a phase (rad) and a loss (dB), got {values:?}",
                    self.kind
                ),
            ));
        }
        let scale = self.wavelength_um / wavelength.to_um();
        let theta = values[0] * scale;
        let a = 10f64.powf(-values[1] / 20.0);
        let t = c64::from_polar(a, theta);
        Ok([t, c64::new(0.0, scale) * t, -t * (LN_10 / 20.0)])
    }
}

/// A two-port's reciprocal S-matrix of transmission `t`.
fn through(t: c64) -> SMatrix {
    let mut s = SMatrix::zeros(2);
    s[(1, 0)] = t;
    s[(0, 1)] = t;
    s
}

impl Component for Driven {
    fn kind(&self) -> &str {
        &self.kind
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        Ok(through(self.transmission(wavelength, values)?[0]))
    }

    fn provenance(&self) -> Provenance {
        self.provenance.clone()
    }

    fn derivatives(&self, wavelength: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        let [_, phase, loss] = self.transmission(wavelength, values)?;
        Ok(Some(vec![through(phase), through(loss)]))
    }
}

/// A measured device: what was measured and where it is written up, the design it was made to
/// (the device's parameters the paper gives, by name, in their units), and the figures measured
/// on it (a power for a π shift, a time constant, a bandwidth), each in the unit the device's
/// own figure of that symbol takes. Read from a file ([`Measurement::parse`]):
///
/// ```toml
/// kind = "thermo-optic phase shifter"   # the device it is a design of
/// source = "M. Jacques et al., Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456"
///
/// [design]                              # what the paper gives; the rest may be anything
/// heater_width = 7.5
/// length = 320.0
///
/// [[figure]]
/// symbol = "P_pi"
/// name = "power for a π shift"
/// value = 29.2
/// unit = "mW"
/// where = "Fig. 8, the MZI's arms 200 µm apart"
/// ```
///
/// A device is computed at its measured fidelity at the design's values alone: the measurement
/// is of one made device, not a model of others.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Measurement {
    /// The kind of device measured ([`super::Device::kind`]).
    pub kind: String,
    /// Where the measurement is published, with its DOI.
    pub source: String,
    /// The design's parameters the source gives, (name, value) in the device's units.
    pub design: Vec<(String, f64)>,
    /// The figures measured, each of fidelity [`Fidelity::Measured`], its grid naming where in
    /// the source it is.
    pub figures: Vec<Figure>,
}

fn parse(reason: impl Into<String>) -> Error {
    Error::Parse {
        what: "measured device".into(),
        reason: reason.into(),
    }
}

impl Measurement {
    /// A measured device from its file's text (TOML, as the type's docs show it).
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for text that isn't one: a key missing or of the wrong type, an unknown
    /// key, a figure named twice.
    pub fn parse(text: &str) -> Result<Measurement> {
        let table: Table = text
            .parse()
            .map_err(|e: toml::de::Error| parse(e.to_string()))?;
        if let Some(k) = table
            .keys()
            .find(|k| !["kind", "source", "design", "figure"].contains(&k.as_str()))
        {
            return Err(parse(format!(
                "unknown key {k}; a measured device has kind, source, design and figure"
            )));
        }
        let text_of = |t: &Table, key: &str, at: &str| -> Result<String> {
            t.get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| parse(format!("{at} needs {key} = \"...\"")))
        };
        let number = |v: &Value, at: &str| -> Result<f64> {
            match v {
                Value::Float(x) => Ok(*x),
                #[allow(clippy::cast_precision_loss)]
                Value::Integer(i) => Ok(*i as f64),
                _ => Err(parse(format!("{at} must be a number"))),
            }
        };
        let kind = text_of(&table, "kind", "a measured device")?;
        let source = text_of(&table, "source", "a measured device")?;
        let design = match table.get("design") {
            Some(Value::Table(t)) => t
                .iter()
                .map(|(k, v)| Ok((k.clone(), number(v, &format!("[design] {k}"))?)))
                .collect::<Result<Vec<_>>>()?,
            None => Vec::new(),
            Some(_) => return Err(parse("[design] must be a table: name = value")),
        };
        let figures = match table.get("figure") {
            Some(Value::Array(a)) if !a.is_empty() => a
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let at = format!("[[figure]] {}", i + 1);
                    let Value::Table(f) = f else {
                        return Err(parse(format!("{at} must be a table")));
                    };
                    if let Some(k) = f.keys().find(|k| {
                        !["symbol", "name", "value", "unit", "where"].contains(&k.as_str())
                    }) {
                        return Err(parse(format!(
                            "{at}: unknown key {k}; a figure has symbol, name, value, unit and \
                             where"
                        )));
                    }
                    let value = number(
                        f.get("value")
                            .ok_or_else(|| parse(format!("{at} needs a value")))?,
                        &format!("{at} value"),
                    )?;
                    Ok(Figure {
                        name: text_of(f, "name", &at)?,
                        symbol: text_of(f, "symbol", &at)?,
                        value,
                        unit: f
                            .get("unit")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_owned(),
                        fidelity: Fidelity::Measured,
                        grid: format!("measured: {}", text_of(f, "where", &at)?),
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            _ => return Err(parse("a measured device needs its figures: [[figure]]")),
        };
        for (k, f) in figures.iter().enumerate() {
            if figures[..k].iter().any(|g| g.symbol == f.symbol) {
                return Err(parse(format!("the figure {} is given twice", f.symbol)));
            }
        }
        Ok(Measurement {
            kind,
            source,
            design,
            figures,
        })
    }

    /// A measured device from the file at `path`.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] for a file that can't be read, and those of [`Measurement::parse`].
    pub fn load(path: &Path) -> Result<Measurement> {
        let text = std::fs::read_to_string(path).map_err(|e| Error::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        Measurement::parse(&text)
    }

    /// The figure measured of symbol `symbol`, if it was.
    pub fn figure(&self, symbol: &str) -> Option<&Figure> {
        self.figures.iter().find(|f| f.symbol == symbol)
    }

    /// Whether `values` are this design's: each parameter it gives at its value.
    pub(crate) fn matches(&self, values: &Values) -> bool {
        self.design.iter().all(|(n, v)| {
            values
                .get(n)
                .is_some_and(|x| (x - v).abs() <= 1e-9 * v.abs().max(1.0))
        })
    }

    /// The design's values as the ranges a measured model holds over.
    pub(crate) fn validity(&self) -> Vec<Range> {
        self.design
            .iter()
            .map(|(n, v)| Range::new(n, *v, *v))
            .collect()
    }

    /// Refuses a measurement of another kind, or one whose design names a parameter `device`
    /// hasn't or a value outside its range.
    pub(crate) fn check_for(&self, kind: &str, parameters: &[Parameter]) -> Result<()> {
        if self.kind != kind {
            return Err(Error::invalid(
                "measured device",
                format!("is of a {}, not a {kind}", self.kind),
            ));
        }
        for (n, v) in &self.design {
            let p = parameters.iter().find(|p| p.name == *n).ok_or_else(|| {
                Error::invalid("measured device", format!("a {kind} has no parameter {n}"))
            })?;
            if !p.allows(*v) {
                return Err(Error::invalid(
                    "measured device",
                    format!(
                        "its {n}, {v} {}, is outside a {kind}'s range ({} to {})",
                        p.unit, p.min, p.max
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Refuses `values` that aren't the design's, naming those that differ.
    pub(crate) fn require(&self, values: &Values) -> Result<()> {
        if self.matches(values) {
            return Ok(());
        }
        let wanted: Vec<String> = self
            .design
            .iter()
            .map(|(n, v)| format!("{n} = {v}"))
            .collect();
        Err(Error::invalid(
            "device",
            format!(
                "a {}'s measured fidelity is the device measured by {}: its design is {}",
                self.kind,
                self.source,
                wanted.join(", ")
            ),
        ))
    }
}
