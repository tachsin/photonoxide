//! Errors returned by photonoxide.

use std::fmt;

/// Errors returned by photonoxide.
///
/// Invalid settings and values are reported as an `Error`, never as a panic. The few functions
/// that panic say so under `# Panics`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// A value is invalid: not finite, out of its allowed range, or inconsistent with another.
    InvalidValue {
        /// What the value is, e.g. `"wavelength"`.
        what: &'static str,
        /// Why it is invalid, e.g. `"must be positive, got -1.55 um"`.
        reason: String,
    },
    /// A material was asked for a wavelength outside the range its data is valid for.
    OutsideValidity {
        /// The material's name.
        material: String,
        /// The wavelength asked for, in micrometres.
        wavelength_um: f64,
        /// The shortest wavelength of the material's range, in micrometres.
        shortest_um: f64,
        /// The longest wavelength of the material's range, in micrometres.
        longest_um: f64,
    },
    /// A file or directory can't be read or written.
    Io {
        /// The path.
        path: String,
        /// The operating system's reason.
        reason: String,
    },
    /// Text can't be read as what it should be, e.g. a job file, or an event of a run's record.
    Parse {
        /// What was being read, e.g. a path and a line number.
        what: String,
        /// Why it can't be read.
        reason: String,
    },
}

impl Error {
    /// An [`Error::InvalidValue`].
    pub(crate) fn invalid(what: &'static str, reason: impl Into<String>) -> Self {
        Error::InvalidValue {
            what,
            reason: reason.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidValue { what, reason } => write!(f, "invalid {what}: {reason}"),
            Error::OutsideValidity {
                material,
                wavelength_um,
                shortest_um,
                longest_um,
            } => write!(
                f,
                "{material} is only valid from {shortest_um} to {longest_um} um, not at {wavelength_um} um"
            ),
            Error::Io { path, reason } => write!(f, "{path}: {reason}"),
            Error::Parse { what, reason } => write!(f, "can't read {what}: {reason}"),
        }
    }
}

impl std::error::Error for Error {}

/// A `Result` with photonoxide's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_invalid_value_says_what_and_why() {
        let e = Error::InvalidValue {
            what: "wavelength",
            reason: "must be positive, got -1.55 um".into(),
        };
        assert_eq!(
            e.to_string(),
            "invalid wavelength: must be positive, got -1.55 um"
        );
    }

    #[test]
    fn errors_are_std_errors() {
        fn takes(_: &dyn std::error::Error) {}
        takes(&Error::InvalidValue {
            what: "x",
            reason: "y".into(),
        });
    }
}
