//! Errors returned by photonoxide, and the netlist's errors they wrap.
//!
//! This module names nothing above itself: [`Error`] wraps only types defined here.

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
    /// A circuit's netlist is invalid: see [`NetlistError`].
    Netlist(NetlistError),
    /// A GPU can't be found or opened, or can't do what was asked (the `gpu` feature).
    Gpu {
        /// Why, e.g. `"no GPU found"`.
        reason: String,
    },
    /// A farm ([`crate::farm`]) can't do its work: no worker left, a worker refused, a task
    /// past its timeout, a message that isn't the protocol's.
    Farm {
        /// Why, e.g. `"no worker left: 127.0.0.1:7878: connection refused"`.
        reason: String,
    },
    /// A time-domain run's fields grew again after its sources had ended, and it stopped: a
    /// wave of the grid amplified in a CPML, which a finer grid removes (docs/methods/fdtd.md,
    /// "Late growth"). See [`crate::fdtd::Simulation::run_until_decayed`].
    Growing {
        /// When the run stopped, µm/c.
        time_um: f64,
        /// How many checks in a row |F|² rose at.
        checks: usize,
        /// How many times its level before the rise it reached.
        factor: f64,
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
            Error::Netlist(e) => write!(f, "invalid netlist: {e}"),
            Error::Gpu { reason } => write!(f, "GPU: {reason}"),
            Error::Farm { reason } => write!(f, "farm: {reason}"),
            Error::Growing {
                time_um,
                checks,
                factor,
            } => write!(
                f,
                "the fields grew again, and the run stopped at {time_um:.0} um/c: |field|^2 at \
                 the probes rose at {checks} checks in a row, to {factor:.1e} times its level \
                 before (a wave of the grid amplified in a CPML, which a finer grid removes: \
                 docs/methods/fdtd.md, \"Late growth\")"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// A `Result` with photonoxide's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// A port of an instance in a netlist, written `instance.port`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PortRef {
    /// The instance's name.
    pub instance: String,
    /// The port's name within its component.
    pub port: String,
}

impl PortRef {
    /// `instance.port` read as a port reference: split at the first dot.
    ///
    /// # Errors
    ///
    /// [`NetlistError::InvalidName`] (as an [`Error::Netlist`]) without a dot, or with nothing
    /// on one side of it.
    pub fn parse(text: &str) -> Result<PortRef> {
        match text.split_once('.') {
            Some((instance, port)) if !instance.is_empty() && !port.is_empty() => Ok(PortRef {
                instance: instance.into(),
                port: port.into(),
            }),
            _ => Err(NetlistError::InvalidName {
                name: text.into(),
                reason: "a port is written instance.port".into(),
            }
            .into()),
        }
    }
}

impl fmt::Display for PortRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.instance, self.port)
    }
}

/// Why a netlist is invalid.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum NetlistError {
    /// A name is empty, holds a dot or whitespace, or a port isn't written `instance.port`.
    InvalidName {
        /// The name.
        name: String,
        /// What is wrong with it.
        reason: String,
    },
    /// A component states duplicate port or parameter names, or a default outside its range.
    InvalidComponent {
        /// The instance it was added as.
        instance: String,
        /// What is wrong with it.
        reason: String,
    },
    /// Two instances have the same name.
    DuplicateInstance(String),
    /// No instance has this name.
    UnknownInstance(String),
    /// The instance's component has no port of this name.
    UnknownPort(PortRef),
    /// The instance's component has no parameter of this name.
    UnknownParameter {
        /// The instance.
        instance: String,
        /// The parameter asked for.
        parameter: String,
    },
    /// A parameter's value is not finite or outside its range.
    ValueOutOfRange {
        /// The instance.
        instance: String,
        /// The parameter.
        parameter: String,
        /// The value given.
        value: f64,
        /// The smallest value allowed.
        min: f64,
        /// The largest value allowed.
        max: f64,
    },
    /// A port connected to itself.
    SelfConnection(PortRef),
    /// A port connected or exposed more than once.
    PortUsedTwice {
        /// The port.
        port: PortRef,
        /// What it was already used for, e.g. `"connected to wg.o1"`.
        first: String,
    },
    /// Two external ports have the same name.
    DuplicateExternal(String),
    /// A port neither connected nor exposed: light leaving it would have nowhere to go.
    Dangling(PortRef),
    /// Two connected ports carry modes of different polarizations or orders.
    ModeMismatch {
        /// One port.
        a: PortRef,
        /// The other.
        b: PortRef,
    },
    /// A component returned an S-matrix that isn't one row and column per port.
    SizeMismatch {
        /// The instance.
        instance: String,
        /// Its ports.
        ports: usize,
        /// The S-matrix's size.
        size: usize,
    },
}

impl fmt::Display for NetlistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetlistError::InvalidName { name, reason } => write!(f, "\"{name}\": {reason}"),
            NetlistError::InvalidComponent { instance, reason } => {
                write!(f, "{instance}'s component: {reason}")
            }
            NetlistError::DuplicateInstance(name) => write!(f, "two instances named {name}"),
            NetlistError::UnknownInstance(name) => write!(f, "no instance named {name}"),
            NetlistError::UnknownPort(port) => {
                write!(f, "{} has no port {}", port.instance, port.port)
            }
            NetlistError::UnknownParameter {
                instance,
                parameter,
            } => write!(f, "{instance} has no parameter {parameter}"),
            NetlistError::ValueOutOfRange {
                instance,
                parameter,
                value,
                min,
                max,
            } => write!(
                f,
                "{instance}.{parameter} must be from {min} to {max}, got {value}"
            ),
            NetlistError::SelfConnection(port) => write!(f, "{port} is connected to itself"),
            NetlistError::PortUsedTwice { port, first } => {
                write!(f, "{port} is used twice: it is already {first}")
            }
            NetlistError::DuplicateExternal(name) => write!(f, "two external ports named {name}"),
            NetlistError::Dangling(port) => write!(f, "{port} is neither connected nor exposed"),
            NetlistError::ModeMismatch { a, b } => {
                write!(f, "{a} and {b} carry different modes")
            }
            NetlistError::SizeMismatch {
                instance,
                ports,
                size,
            } => write!(
                f,
                "{instance} has {ports} ports but returned a {size} x {size} S-matrix"
            ),
        }
    }
}

impl From<NetlistError> for Error {
    fn from(e: NetlistError) -> Error {
        Error::Netlist(e)
    }
}
