//! What a process can do, as numbers with their sources: [`Stated`], [`Capability`] and
//! [`Fact`]. A number can't be written without the paper and the place in it that give it; the
//! only other value says it is not specified.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Where a value comes from: a reference's key ([`super::Reference::key`]) and the place in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// The reference's key, the file name its PDF is kept under, e.g. `"selvaraja-2009"`.
    pub reference: String,
    /// Where in it, e.g. `"Section IV (p. 4081)"` or `"Table 1 (p. 58)"`.
    pub location: String,
}

impl Source {
    /// The place `location` in the reference `reference`.
    pub fn new(reference: &str, location: &str) -> Source {
        Source {
            reference: reference.into(),
            location: location.into(),
        }
    }
}

/// The unit of a stated value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Unit {
    /// Nanometres.
    Nm,
    /// Micrometres.
    Um,
    /// Degrees of angle, from the wafer's plane.
    Degree,
    /// Degrees Celsius.
    Celsius,
    /// A ratio, n:1.
    Ratio,
    /// A pure number, e.g. a numerical aperture.
    Number,
    /// Per cent.
    Percent,
}

impl Unit {
    /// Its symbol, as the narration writes it.
    pub fn symbol(self) -> &'static str {
        match self {
            Unit::Nm => "nm",
            Unit::Um => "µm",
            Unit::Degree => "°",
            Unit::Celsius => "°C",
            Unit::Ratio => ":1",
            Unit::Number => "",
            Unit::Percent => " %",
        }
    }
}

/// A value a source states, or "not specified": the type makes a number without its source
/// impossible to write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Stated {
    /// From `low` to `high` (equal for one value), in `unit`, as `source` gives it.
    Range {
        /// The lower end.
        low: f64,
        /// The upper end.
        high: f64,
        /// The unit.
        unit: Unit,
        /// Where it comes from.
        source: Source,
    },
    /// At most `value` ("below 5 nm"), as `source` gives it.
    AtMost {
        /// The bound.
        value: f64,
        /// The unit.
        unit: Unit,
        /// Where it comes from.
        source: Source,
    },
    /// At least `value` ("above 1000 °C"), as `source` gives it.
    AtLeast {
        /// The bound.
        value: f64,
        /// The unit.
        unit: Unit,
        /// Where it comes from.
        source: Source,
    },
    /// No source in hand gives it; `coming` names the paper it is waiting for, if one is known.
    NotSpecified {
        /// The paper it will come from, if known.
        coming: Option<String>,
    },
}

impl Stated {
    /// One value, `value` `unit`, from `source`.
    pub fn value(value: f64, unit: Unit, source: Source) -> Stated {
        Stated::range(value, value, unit, source)
    }

    /// From `low` to `high` `unit`, from `source`.
    pub fn range(low: f64, high: f64, unit: Unit, source: Source) -> Stated {
        Stated::Range {
            low,
            high,
            unit,
            source,
        }
    }

    /// Not specified by any source in hand.
    pub fn not_specified() -> Stated {
        Stated::NotSpecified { coming: None }
    }

    /// Its source, unless it is not specified.
    pub fn source(&self) -> Option<&Source> {
        match self {
            Stated::Range { source, .. }
            | Stated::AtMost { source, .. }
            | Stated::AtLeast { source, .. } => Some(source),
            Stated::NotSpecified { .. } => None,
        }
    }

    /// Its unit, unless it is not specified.
    pub fn unit(&self) -> Option<Unit> {
        match self {
            Stated::Range { unit, .. }
            | Stated::AtMost { unit, .. }
            | Stated::AtLeast { unit, .. } => Some(*unit),
            Stated::NotSpecified { .. } => None,
        }
    }

    /// The lowest value it allows: a range's lower end, an "at most"'s 0 for lengths and
    /// ratios, an "at least"'s bound.
    pub fn low(&self) -> Option<f64> {
        match *self {
            Stated::Range { low, .. } => Some(low),
            Stated::AtMost { .. } => None,
            Stated::AtLeast { value, .. } => Some(value),
            Stated::NotSpecified { .. } => None,
        }
    }

    /// The highest value it allows: a range's upper end, an "at most"'s bound.
    pub fn high(&self) -> Option<f64> {
        match *self {
            Stated::Range { high, .. } => Some(high),
            Stated::AtMost { value, .. } => Some(value),
            Stated::AtLeast { .. } | Stated::NotSpecified { .. } => None,
        }
    }

    /// Whether it is stated (has a source).
    pub fn is_stated(&self) -> bool {
        self.source().is_some()
    }
}

/// A number as the narration writes it: to four significant digits, without trailing zeros.
pub(crate) fn number(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let digits = (3 - v.abs().log10().floor() as i32).max(0) as usize;
    let s = format!("{v:.digits$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

impl fmt::Display for Stated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = |u: Unit| {
            if matches!(
                u,
                Unit::Ratio | Unit::Degree | Unit::Celsius | Unit::Number | Unit::Percent
            ) {
                u.symbol().to_string()
            } else {
                format!(" {}", u.symbol())
            }
        };
        match self {
            Stated::Range {
                low, high, unit: u, ..
            } if low == high => write!(f, "{}{}", number(*low), unit(*u)),
            Stated::Range {
                low, high, unit: u, ..
            } => write!(f, "{} to {}{}", number(*low), number(*high), unit(*u)),
            Stated::AtMost { value, unit: u, .. } => {
                write!(f, "at most {}{}", number(*value), unit(*u))
            }
            Stated::AtLeast { value, unit: u, .. } => {
                write!(f, "at least {}{}", number(*value), unit(*u))
            }
            Stated::NotSpecified { .. } => write!(f, "not specified"),
        }
    }
}

/// What a capability measures.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Quantity {
    /// The smallest line or gap a lithography resolves.
    MinFeature,
    /// How far a stitched or aligned field lands from where it should.
    Overlay,
    /// The wall's angle from the wafer's plane that an etch leaves.
    Sidewall,
    /// How much faster an etch removes its film than `over` (a mask, or the layer below).
    Selectivity {
        /// What the film is etched against.
        over: String,
    },
    /// How much a thickness or an etch's depth varies across the wafer or sample.
    Uniformity,
    /// A surface's or a wall's roughness.
    Roughness,
    /// The temperature the process reaches.
    Temperature,
    /// A film's thickness, as the sources made it.
    Thickness,
    /// A projection lithography's exposure wavelength.
    Wavelength,
    /// A projection lithography's numerical aperture.
    NumericalAperture,
    /// A deposition's step coverage: the film's thickness on a step's wall over its thickness on
    /// the flat (Franssila, Eq. 5.18a); 100 % is conformal.
    StepCoverage,
    /// The deepest etch over the narrowest opening that an etch reaches.
    AspectRatio,
}

impl Quantity {
    /// Its name, as the narration writes it.
    pub fn name(&self) -> String {
        match self {
            Quantity::MinFeature => "minimum feature".into(),
            Quantity::Overlay => "overlay".into(),
            Quantity::Sidewall => "sidewall angle".into(),
            Quantity::Selectivity { over } => format!("selectivity over {over}"),
            Quantity::Uniformity => "uniformity".into(),
            Quantity::Roughness => "roughness".into(),
            Quantity::Temperature => "temperature".into(),
            Quantity::Thickness => "thickness".into(),
            Quantity::Wavelength => "wavelength".into(),
            Quantity::NumericalAperture => "numerical aperture".into(),
            Quantity::StepCoverage => "step coverage".into(),
            Quantity::AspectRatio => "aspect ratio".into(),
        }
    }
}

/// One thing a process can achieve: what, how much (with its source) and under which conditions
/// the source states it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Capability {
    /// What it measures.
    pub quantity: Quantity,
    /// The value, with its source, or not specified.
    pub value: Stated,
    /// The film, tool or setting the source states it for.
    pub conditions: String,
}

impl Capability {
    /// `quantity` is `value` under `conditions`.
    pub fn new(quantity: Quantity, value: Stated, conditions: &str) -> Capability {
        Capability {
            quantity,
            value,
            conditions: conditions.into(),
        }
    }
}

/// A statement a source makes without a number, e.g. that an etch stops on the oxide below.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fact {
    /// The statement, in plain words.
    pub text: String,
    /// Where it comes from.
    pub source: Source,
}

impl Fact {
    /// `text`, as `source` says.
    pub fn new(text: &str, source: Source) -> Fact {
        Fact {
            text: text.into(),
            source,
        }
    }
}
