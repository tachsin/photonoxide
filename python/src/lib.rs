//! The native module of the `photonoxide` package for Python, `photonoxide._photonoxide`: the
//! library's façade ([`photonoxide::facade`]) one function for one, arrays as NumPy's. The
//! package's Python code (`photonoxide/__init__.py`) turns these into its functions and result
//! classes; this module is not for users.
//!
//! Every call releases the GIL while Rust works. The long ones (the vector mode solver, a job's
//! run) work on a thread of their own while the calling thread checks for Ctrl+C, which Python
//! handles only there: Ctrl+C then asks the work to stop, waits for it, and raises
//! `KeyboardInterrupt`.
//!
//! The crate writes no `unsafe` code: PyO3's and rust-numpy's own, and what their macros expand
//! to, is all there is.

#![forbid(unsafe_code)]

mod errors;

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use numpy::{
    Complex64, PyArray1, PyArrayMethods, PyReadonlyArray1, PyReadonlyArray2, PyReadonlyArray3,
    PyUntypedArrayMethods,
};
use photonoxide::facade::{self, CrossSection, IndexOptions, JobSource, Stop};
use pyo3::exceptions::{PyTimeoutError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyComplex, PyDict, PyList};

use errors::Fail;

/// How often a waiting call checks for Ctrl+C.
const SIGNAL_CHECK: Duration = Duration::from_millis(50);

/// `work` on a thread of its own, without the GIL, while this thread checks for Ctrl+C: on
/// Ctrl+C, `stop` is requested, the work is waited for, and the `KeyboardInterrupt` returned.
fn interruptible<T: Send>(
    py: Python<'_>,
    stop: &Stop,
    work: impl FnOnce() -> T + Send,
) -> PyResult<T> {
    let mut interrupt = None;
    let result = py.detach(|| {
        std::thread::scope(|scope| {
            let (done, finished) = mpsc::channel();
            let worker = scope.spawn(move || {
                let result = work();
                let _ = done.send(());
                result
            });
            while let Err(RecvTimeoutError::Timeout) = finished.recv_timeout(SIGNAL_CHECK) {
                if interrupt.is_none() {
                    Python::attach(|py| {
                        if let Err(error) = py.check_signals() {
                            stop.request();
                            interrupt = Some(error);
                        }
                    });
                }
            }
            worker
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
        })
    });
    match interrupt {
        Some(error) => Err(error),
        None => Ok(result),
    }
}

/// A stop at `timeout_s` from now, or only on request.
fn stop_after(timeout_s: Option<f64>) -> PyResult<Stop> {
    match timeout_s {
        None => Ok(Stop::new(None)),
        Some(s) if s.is_finite() && s > 0.0 => Ok(Stop::new(Some(Duration::from_secs_f64(s)))),
        Some(s) => Err(PyValueError::new_err(format!(
            "timeout_s must be positive and finite, or None, not {s}"
        ))),
    }
}

fn vector(array: &PyReadonlyArray1<'_, f64>) -> Vec<f64> {
    array.as_array().iter().copied().collect()
}

fn index_options(
    model: Option<String>,
    axis: Option<String>,
    temperature_k: Option<f64>,
    composition: Option<f64>,
) -> IndexOptions {
    let mut options = IndexOptions::default();
    options.model = model;
    options.axis = axis;
    options.temperature_k = temperature_k;
    options.composition = composition;
    options
}

/// photonoxide's version: the crate the package was built with.
const CORE_VERSION: &str = photonoxide::VERSION;

/// The catalogue's materials, each a dict, its models dicts.
#[pyfunction]
fn materials(py: Python<'_>) -> PyResult<Bound<'_, PyList>> {
    let list = PyList::empty(py);
    for m in facade::materials() {
        let models = PyList::empty(py);
        for model in &m.models {
            let d = PyDict::new(py);
            d.set_item("id", &model.id)?;
            d.set_item("name", &model.name)?;
            d.set_item("default", model.default)?;
            d.set_item("axes", &model.axes)?;
            d.set_item("range_um", model.range_um)?;
            d.set_item("temperature_k", model.temperature_k)?;
            d.set_item("composition", model.composition)?;
            d.set_item("accuracy", &model.accuracy)?;
            d.set_item("sources", &model.sources)?;
            models.append(d)?;
        }
        let d = PyDict::new(py);
        d.set_item("id", &m.id)?;
        d.set_item("name", &m.name)?;
        d.set_item("formula", &m.formula)?;
        d.set_item("models", models)?;
        list.append(d)?;
    }
    Ok(list)
}

/// n + iκ of a material at each wavelength (µm).
#[pyfunction]
#[pyo3(signature = (material, wavelength_um, model=None, axis=None, temperature_k=None, composition=None))]
fn refractive_index<'py>(
    py: Python<'py>,
    material: &str,
    wavelength_um: PyReadonlyArray1<'py, f64>,
    model: Option<String>,
    axis: Option<String>,
    temperature_k: Option<f64>,
    composition: Option<f64>,
) -> PyResult<Bound<'py, PyArray1<Complex64>>> {
    let w = vector(&wavelength_um);
    let options = index_options(model, axis, temperature_k, composition);
    let n = py
        .detach(|| facade::refractive_index(material, &w, &options))
        .fail(py)?;
    Ok(PyArray1::from_vec(py, n))
}

/// The bulk group index of a material at each wavelength (µm).
#[pyfunction]
#[pyo3(signature = (material, wavelength_um, model=None, axis=None, temperature_k=None, composition=None))]
fn group_index<'py>(
    py: Python<'py>,
    material: &str,
    wavelength_um: PyReadonlyArray1<'py, f64>,
    model: Option<String>,
    axis: Option<String>,
    temperature_k: Option<f64>,
    composition: Option<f64>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let w = vector(&wavelength_um);
    let options = index_options(model, axis, temperature_k, composition);
    let n = py
        .detach(|| facade::group_index(material, &w, &options))
        .fail(py)?;
    Ok(PyArray1::from_vec(py, n))
}

/// A slab's guided modes, each a dict.
#[pyfunction]
#[allow(clippy::too_many_arguments)] // the façade's, one for one
fn slab_modes<'py>(
    py: Python<'py>,
    below: f64,
    core: f64,
    above: f64,
    thickness_um: f64,
    polarization: &str,
    wavelength_um: f64,
    x_um: PyReadonlyArray1<'py, f64>,
) -> PyResult<Bound<'py, PyList>> {
    let x = vector(&x_um);
    let modes = py
        .detach(|| {
            facade::slab_modes(
                below,
                core,
                above,
                thickness_um,
                polarization,
                wavelength_um,
                &x,
            )
        })
        .fail(py)?;
    let list = PyList::empty(py);
    for m in modes {
        let d = PyDict::new(py);
        d.set_item("polarization", &m.polarization)?;
        d.set_item("order", m.order)?;
        d.set_item("effective_index", m.effective_index)?;
        d.set_item("x_um", PyArray1::from_vec(py, m.x_um))?;
        d.set_item("field", PyArray1::from_vec(py, m.field))?;
        list.append(d)?;
    }
    Ok(list)
}

/// A cross-section's full-vector modes, each a dict with its fields as `(ny, nx)` arrays.
#[pyfunction]
#[pyo3(signature = (x_um, y_um, permittivity, wavelength_um, count, near_index, boundaries, pml_um, pml_strength, timeout_s))]
#[allow(clippy::too_many_arguments)] // the façade's, one for one
fn vector_modes<'py>(
    py: Python<'py>,
    x_um: PyReadonlyArray1<'py, f64>,
    y_um: PyReadonlyArray1<'py, f64>,
    permittivity: PyReadonlyArray2<'py, Complex64>,
    wavelength_um: f64,
    count: usize,
    near_index: Option<f64>,
    boundaries: [String; 4],
    pml_um: [f64; 4],
    pml_strength: f64,
    timeout_s: Option<f64>,
) -> PyResult<Bound<'py, PyList>> {
    let mut cs = CrossSection::new(
        vector(&x_um),
        vector(&y_um),
        permittivity.as_array().iter().copied().collect(),
    );
    let shape = permittivity.shape();
    let (ny, nx) = (
        cs.y_um.len().saturating_sub(1),
        cs.x_um.len().saturating_sub(1),
    );
    if shape != [ny, nx] {
        return Err(PyValueError::new_err(format!(
            "permittivity must have one value per cell, shape (ny, nx) = ({ny}, {nx}), not {shape:?}"
        )));
    }
    cs.boundaries = boundaries;
    cs.pml_um = pml_um;
    cs.pml_strength = pml_strength;
    let stop = stop_after(timeout_s)?;
    let modes = interruptible(py, &stop, || {
        facade::vector_modes(&cs, wavelength_um, count, near_index, &stop)
    })?
    .fail(py)?
    .ok_or_else(|| {
        PyTimeoutError::new_err(format!(
            "the mode solve stopped at its timeout, {} s",
            timeout_s.unwrap_or(f64::NAN)
        ))
    })?;
    let list = PyList::empty(py);
    for m in modes {
        let (ny, nx) = (m.y_um.len(), m.x_um.len());
        let d = PyDict::new(py);
        d.set_item(
            "effective_index",
            PyComplex::from_doubles(py, m.effective_index.re, m.effective_index.im),
        )?;
        d.set_item("te_fraction", m.te_fraction)?;
        d.set_item("x_um", PyArray1::from_vec(py, m.x_um))?;
        d.set_item("y_um", PyArray1::from_vec(py, m.y_um))?;
        let [ex, ey, ez] = m.e;
        let [hx, hy, hz] = m.h;
        for (name, values) in [
            ("ex", ex),
            ("ey", ey),
            ("ez", ez),
            ("hx", hx),
            ("hy", hy),
            ("hz", hz),
        ] {
            d.set_item(name, PyArray1::from_vec(py, values).reshape([ny, nx])?)?;
        }
        list.append(d)?;
    }
    Ok(list)
}

/// A job from a path, or from a job file's text in a format.
fn job_source(path: Option<PathBuf>, text: Option<String>, format: &str) -> PyResult<JobSource> {
    match (path, text) {
        (Some(path), None) => Ok(JobSource::File(path)),
        (None, Some(text)) => Ok(JobSource::Text {
            text,
            format: format.to_owned(),
        }),
        _ => Err(PyValueError::new_err("a job is a path or a text, not both")),
    }
}

/// Checks a job without running it.
#[pyfunction]
#[pyo3(signature = (path, text, format))]
fn check_job(
    py: Python<'_>,
    path: Option<PathBuf>,
    text: Option<String>,
    format: &str,
) -> PyResult<()> {
    let job = job_source(path, text, format)?;
    py.detach(|| facade::check_job(&job)).fail(py)
}

/// Runs a job into a folder under `runs_dir`: a dict of its folder, its events (JSON text,
/// one each) and why it stopped early, if it did.
#[pyfunction]
#[pyo3(signature = (path, text, format, runs_dir, timeout_s))]
fn run_job<'py>(
    py: Python<'py>,
    path: Option<PathBuf>,
    text: Option<String>,
    format: &str,
    runs_dir: PathBuf,
    timeout_s: Option<f64>,
) -> PyResult<Bound<'py, PyDict>> {
    let job = job_source(path, text, format)?;
    let stop = stop_after(timeout_s)?;
    let run = interruptible(py, &stop, || facade::run_job(&job, &runs_dir, &stop))?.fail(py)?;
    let d = PyDict::new(py);
    d.set_item("dir", run.dir)?;
    d.set_item("events", run.events)?;
    d.set_item("stopped", run.stopped)?;
    Ok(d)
}

/// An `"fdfd"` job's S-parameters, a dict with S as an `(nλ, n, n)` array.
#[pyfunction]
#[pyo3(signature = (path, text, format, wavelength_um))]
fn fdfd_s_parameters<'py>(
    py: Python<'py>,
    path: Option<PathBuf>,
    text: Option<String>,
    format: &str,
    wavelength_um: Option<PyReadonlyArray1<'py, f64>>,
) -> PyResult<Bound<'py, PyDict>> {
    let job = job_source(path, text, format)?;
    let w = wavelength_um.as_ref().map(vector);
    let s = py
        .detach(|| facade::fdfd_s_parameters(&job, w.as_deref()))
        .fail(py)?;
    let (k, n) = (s.wavelength_um.len(), s.ports.len());
    let d = PyDict::new(py);
    d.set_item("ports", s.ports)?;
    d.set_item("wavelength_um", PyArray1::from_vec(py, s.wavelength_um))?;
    d.set_item("s", PyArray1::from_vec(py, s.s).reshape([k, n, n])?)?;
    d.set_item(
        "effective_index",
        PyArray1::from_vec(py, s.effective_index).reshape([k, n])?,
    )?;
    d.set_item("cell_um", s.cell_um)?;
    d.set_item("polarization", s.polarization)?;
    Ok(d)
}

/// A spectrum as a dict, S as an `(nλ, n, n)` array.
fn spectrum_dict<'py>(py: Python<'py>, s: facade::Spectrum) -> PyResult<Bound<'py, PyDict>> {
    let (k, n) = (s.wavelength_um.len(), s.ports.len());
    let d = PyDict::new(py);
    d.set_item("ports", s.ports)?;
    d.set_item("wavelength_um", PyArray1::from_vec(py, s.wavelength_um))?;
    d.set_item("s", PyArray1::from_vec(py, s.s).reshape([k, n, n])?)?;
    Ok(d)
}

/// A circuit's spectrum from its netlist as JSON.
#[pyfunction]
fn circuit_spectrum<'py>(
    py: Python<'py>,
    netlist: &str,
    wavelength_um: PyReadonlyArray1<'py, f64>,
) -> PyResult<Bound<'py, PyDict>> {
    let w = vector(&wavelength_um);
    let s = py
        .detach(|| facade::circuit_spectrum(netlist, &w))
        .fail(py)?;
    spectrum_dict(py, s)
}

/// A Touchstone file's S-parameters.
#[pyfunction]
fn read_touchstone<'py>(
    py: Python<'py>,
    path: PathBuf,
    convention: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let s = py
        .detach(|| facade::read_touchstone(&path, convention))
        .fail(py)?;
    spectrum_dict(py, s)
}

/// Writes S-matrices `(nλ, n, n)` to a Touchstone file.
#[pyfunction]
#[pyo3(signature = (path, ports, wavelength_um, s, convention, significant_digits))]
fn write_touchstone<'py>(
    py: Python<'py>,
    path: PathBuf,
    ports: Vec<String>,
    wavelength_um: PyReadonlyArray1<'py, f64>,
    s: PyReadonlyArray3<'py, Complex64>,
    convention: &str,
    significant_digits: Option<usize>,
) -> PyResult<()> {
    let (k, n) = (wavelength_um.len(), ports.len());
    if s.shape() != [k, n, n] {
        return Err(PyValueError::new_err(format!(
            "s must have the shape (nλ, n, n) = ({k}, {n}, {n}), not {:?}",
            s.shape()
        )));
    }
    let spectrum = facade::Spectrum::new(
        ports,
        vector(&wavelength_um),
        s.as_array().iter().copied().collect(),
    )
    .fail(py)?;
    py.detach(|| facade::write_touchstone(&path, &spectrum, convention, significant_digits))
        .fail(py)
}

/// The façade's conformance cases ([`facade::conformance`]), computed here: dicts of the case's
/// name, function, arguments and result (JSON text each) and the members to ignore.
#[pyfunction]
fn _conformance(py: Python<'_>, scratch: PathBuf) -> PyResult<Bound<'_, PyList>> {
    let cases = py.detach(|| facade::conformance(&scratch)).fail(py)?;
    let list = PyList::empty(py);
    for c in cases {
        let d = PyDict::new(py);
        d.set_item("name", c.name)?;
        d.set_item("function", c.function)?;
        d.set_item("args", c.args)?;
        d.set_item("result", c.result)?;
        d.set_item("ignore", c.ignore)?;
        list.append(d)?;
    }
    Ok(list)
}

#[pymodule]
fn _photonoxide(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("core_version", CORE_VERSION)?;
    module.add_function(wrap_pyfunction!(materials, module)?)?;
    module.add_function(wrap_pyfunction!(refractive_index, module)?)?;
    module.add_function(wrap_pyfunction!(group_index, module)?)?;
    module.add_function(wrap_pyfunction!(slab_modes, module)?)?;
    module.add_function(wrap_pyfunction!(vector_modes, module)?)?;
    module.add_function(wrap_pyfunction!(check_job, module)?)?;
    module.add_function(wrap_pyfunction!(run_job, module)?)?;
    module.add_function(wrap_pyfunction!(fdfd_s_parameters, module)?)?;
    module.add_function(wrap_pyfunction!(circuit_spectrum, module)?)?;
    module.add_function(wrap_pyfunction!(read_touchstone, module)?)?;
    module.add_function(wrap_pyfunction!(write_touchstone, module)?)?;
    module.add_function(wrap_pyfunction!(_conformance, module)?)?;
    Ok(())
}
