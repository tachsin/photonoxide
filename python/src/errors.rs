//! photonoxide's errors as Python exceptions: the classes of `photonoxide.errors`, one per kind
//! of [`photonoxide::Error`], each with the error's message and its fields as attributes.

use photonoxide::Error;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// A photonoxide result made a Python one.
pub trait Fail<T> {
    /// The value, or the error raised as its `photonoxide.errors` class.
    fn fail(self, py: Python<'_>) -> PyResult<T>;
}

impl<T> Fail<T> for photonoxide::Result<T> {
    fn fail(self, py: Python<'_>) -> PyResult<T> {
        self.map_err(|e| exception(py, &e))
    }
}

/// The exception for `error`: its class's name and its attributes.
fn exception(py: Python<'_>, error: &Error) -> PyErr {
    let raised = || -> PyResult<PyErr> {
        let attributes = PyDict::new(py);
        let class = match error {
            Error::InvalidValue { what, reason } => {
                attributes.set_item("what", *what)?;
                attributes.set_item("reason", reason)?;
                "InvalidValue"
            }
            Error::OutsideValidity {
                material,
                wavelength_um,
                shortest_um,
                longest_um,
            } => {
                attributes.set_item("material", material)?;
                attributes.set_item("wavelength_um", wavelength_um)?;
                attributes.set_item("shortest_um", shortest_um)?;
                attributes.set_item("longest_um", longest_um)?;
                "OutsideValidity"
            }
            Error::Io { path, reason } => {
                attributes.set_item("path", path)?;
                attributes.set_item("reason", reason)?;
                "IoError"
            }
            Error::Parse { what, reason } => {
                attributes.set_item("what", what)?;
                attributes.set_item("reason", reason)?;
                "ParseError"
            }
            Error::Netlist(_) => "NetlistError",
            Error::Gpu { reason } => {
                attributes.set_item("reason", reason)?;
                "GpuError"
            }
            // a kind of error newer than this package: the base class
            _ => "Error",
        };
        let errors = py.import("photonoxide.errors")?;
        // the classes take the error's fields as keyword arguments, and keep them as attributes
        let instance = errors
            .getattr(class)?
            .call((error.to_string(),), Some(&attributes))?;
        Ok(PyErr::from_value(instance))
    };
    raised().unwrap_or_else(|e| {
        PyRuntimeError::new_err(format!("{error} (and photonoxide.errors failed: {e})"))
    })
}
