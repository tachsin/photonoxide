//! The native module of the `photonoxide` package for Python, `photonoxide._photonoxide`: the
//! library's façade ([`photonoxide::facade`]) one function for one, arrays as NumPy's. The
//! package's Python code (`photonoxide/__init__.py`) turns these into its functions and result
//! classes; this module is not for users.
//!
//! Every call releases the GIL while Rust works. The long ones (the vector mode solver, a job's
//! run, a device's spectrum) work on a thread of their own while the calling thread checks for Ctrl+C, which Python
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
use photonoxide::facade::{self, CrossSection, DeviceDesign, IndexOptions, JobSource, Stop};
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

/// A cross-section's guided full-vector modes, of the `count` nearest `near_index`, each a dict
/// with its fields as `(ny, nx)` arrays: fewer than `count` when fewer are guided (with PMLs,
/// every mode solved).
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

/// The tables of a run: a dict of each table's name to a dict of its columns, in order, each a
/// float array (NaN where the table has no number) or a list of text.
#[pyfunction]
fn run_tables(py: Python<'_>, run_dir: PathBuf) -> PyResult<Bound<'_, PyDict>> {
    let tables = py.detach(|| facade::run_tables(&run_dir)).fail(py)?;
    let out = PyDict::new(py);
    for t in tables {
        let columns = PyDict::new(py);
        for c in t.columns {
            match (c.numbers, c.texts) {
                (Some(numbers), _) => columns.set_item(c.name, PyArray1::from_vec(py, numbers))?,
                (None, texts) => columns.set_item(c.name, texts.unwrap_or_default())?,
            }
        }
        out.set_item(t.name, columns)?;
    }
    Ok(out)
}

/// Writes a run's tables as CSV files: the paths written.
#[pyfunction]
#[pyo3(signature = (run_dir, tables, out_dir))]
fn export_run(
    py: Python<'_>,
    run_dir: PathBuf,
    tables: Option<Vec<String>>,
    out_dir: Option<PathBuf>,
) -> PyResult<Vec<PathBuf>> {
    py.detach(|| facade::export_run(&run_dir, tables.as_deref(), out_dir.as_deref()))
        .fail(py)
}

/// A run's field files, each a dict of its name, its size, its size written again at
/// `precision` and `level` (estimated before writing), whether that is exact, the precision
/// it would have and whether it can be written again.
#[pyfunction]
fn estimate_fields(
    py: Python<'_>,
    run_dir: PathBuf,
    precision: String,
    level: u8,
) -> PyResult<Vec<Bound<'_, PyDict>>> {
    let files = py
        .detach(|| facade::estimate_fields(&run_dir, &precision, level))
        .fail(py)?;
    files
        .into_iter()
        .map(|f| {
            let d = PyDict::new(py);
            d.set_item("file", f.file)?;
            d.set_item("bytes", f.bytes)?;
            d.set_item("estimate", f.estimate)?;
            d.set_item("exact", f.exact)?;
            d.set_item("precision", f.precision)?;
            d.set_item("rewritable", f.rewritable)?;
            Ok(d)
        })
        .collect()
}

/// Writes a run's field files again into another folder: the paths written.
#[pyfunction]
#[pyo3(signature = (run_dir, files, out_dir, precision, level))]
fn export_fields(
    py: Python<'_>,
    run_dir: PathBuf,
    files: Option<Vec<String>>,
    out_dir: PathBuf,
    precision: String,
    level: u8,
) -> PyResult<Vec<PathBuf>> {
    py.detach(|| facade::export_fields(&run_dir, files.as_deref(), &out_dir, &precision, level))
        .fail(py)
}

/// A device's design from the package's arguments.
fn design(
    kind: &str,
    values: Vec<(String, f64)>,
    fidelity: Option<String>,
    stack: Option<String>,
    compact: Option<PathBuf>,
) -> DeviceDesign {
    let mut d = DeviceDesign::new(kind);
    d.values = values;
    d.fidelity = fidelity;
    d.stack = stack;
    d.compact = compact;
    d
}

/// The library's devices, each a dict: its parameters and models dicts.
#[pyfunction]
fn devices(py: Python<'_>) -> PyResult<Bound<'_, PyList>> {
    let all = py.detach(facade::devices).fail(py)?;
    let list = PyList::empty(py);
    for d in all {
        let parameters = PyList::empty(py);
        for p in &d.parameters {
            let item = PyDict::new(py);
            item.set_item("name", &p.name)?;
            item.set_item("unit", &p.unit)?;
            item.set_item("default", p.default)?;
            item.set_item("min", p.min)?;
            item.set_item("max", p.max)?;
            parameters.append(item)?;
        }
        let fidelities = PyList::empty(py);
        for m in &d.fidelities {
            let item = PyDict::new(py);
            item.set_item("fidelity", &m.fidelity)?;
            item.set_item("name", &m.name)?;
            item.set_item("write_up", &m.write_up)?;
            item.set_item("papers", &m.papers)?;
            match &m.error {
                Some(e) => {
                    let error = PyDict::new(py);
                    error.set_item("quantity", &e.quantity)?;
                    error.set_item("bound", e.bound)?;
                    error.set_item("unit", &e.unit)?;
                    error.set_item("against", &e.against)?;
                    item.set_item("error", error)?;
                }
                None => item.set_item("error", py.None())?,
            }
            let validity = PyList::empty(py);
            for r in &m.validity {
                let range = PyDict::new(py);
                range.set_item("name", &r.name)?;
                range.set_item("min", r.min)?;
                range.set_item("max", r.max)?;
                validity.append(range)?;
            }
            item.set_item("validity", validity)?;
            fidelities.append(item)?;
        }
        let item = PyDict::new(py);
        item.set_item("kind", &d.kind)?;
        item.set_item("about", &d.about)?;
        item.set_item("parameters", parameters)?;
        item.set_item("fidelities", fidelities)?;
        item.set_item("ports", &d.ports)?;
        list.append(item)?;
    }
    Ok(list)
}

/// The catalogue's stacks, each a dict, its layers dicts.
#[pyfunction]
fn stacks(py: Python<'_>) -> PyResult<Bound<'_, PyList>> {
    let list = PyList::empty(py);
    for s in facade::stacks() {
        let layers = PyList::empty(py);
        for l in &s.layers {
            let item = PyDict::new(py);
            item.set_item("name", &l.name)?;
            item.set_item("thickness_um", l.thickness_um)?;
            item.set_item("material", &l.material)?;
            item.set_item("background", &l.background)?;
            layers.append(item)?;
        }
        let item = PyDict::new(py);
        item.set_item("name", &s.name)?;
        item.set_item("about", &s.about)?;
        item.set_item("substrate", &s.substrate)?;
        item.set_item("layers", layers)?;
        item.set_item("cladding", &s.cladding)?;
        item.set_item("core", &s.core)?;
        list.append(item)?;
    }
    Ok(list)
}

/// A device's S-parameters at its fidelity, a dict with S as an `(nλ, n, n)` array, stopped at
/// `timeout_s` or on Ctrl+C ([`facade::device_spectrum_until`]).
#[pyfunction]
#[pyo3(signature = (kind, values, fidelity, stack, compact, wavelength_um, timeout_s))]
#[allow(clippy::too_many_arguments)] // the design's fields, one for one, and the call's
fn device_spectrum<'py>(
    py: Python<'py>,
    kind: &str,
    values: Vec<(String, f64)>,
    fidelity: Option<String>,
    stack: Option<String>,
    compact: Option<PathBuf>,
    wavelength_um: PyReadonlyArray1<'py, f64>,
    timeout_s: Option<f64>,
) -> PyResult<Bound<'py, PyDict>> {
    let w = vector(&wavelength_um);
    let d = design(kind, values, fidelity, stack, compact);
    let stop = stop_after(timeout_s)?;
    let s = interruptible(py, &stop, || facade::device_spectrum_until(&d, &w, &stop))?
        .fail(py)?
        .ok_or_else(|| {
            PyTimeoutError::new_err(format!(
                "the device's solve stopped at its timeout, {} s",
                timeout_s.unwrap_or(f64::NAN)
            ))
        })?;
    let (k, n) = (s.wavelength_um.len(), s.ports.len());
    let out = PyDict::new(py);
    out.set_item("ports", s.ports)?;
    out.set_item("wavelength_um", PyArray1::from_vec(py, s.wavelength_um))?;
    out.set_item("s", PyArray1::from_vec(py, s.s).reshape([k, n, n])?)?;
    out.set_item("fidelity", s.fidelity)?;
    out.set_item("grid", s.grid)?;
    out.set_item("source", s.source)?;
    out.set_item("validity_um", s.validity_um)?;
    Ok(out)
}

/// A device's figures at its fidelity, each a dict.
#[pyfunction]
#[pyo3(signature = (kind, values, fidelity, stack, compact))]
fn device_derived<'py>(
    py: Python<'py>,
    kind: &str,
    values: Vec<(String, f64)>,
    fidelity: Option<String>,
    stack: Option<String>,
    compact: Option<PathBuf>,
) -> PyResult<Bound<'py, PyList>> {
    let d = design(kind, values, fidelity, stack, compact);
    let figures = py.detach(|| facade::device_derived(&d)).fail(py)?;
    let list = PyList::empty(py);
    for f in figures {
        let item = PyDict::new(py);
        item.set_item("name", f.name)?;
        item.set_item("symbol", f.symbol)?;
        item.set_item("value", f.value)?;
        item.set_item("unit", f.unit)?;
        item.set_item("fidelity", f.fidelity)?;
        item.set_item("grid", f.grid)?;
        list.append(item)?;
    }
    Ok(list)
}

/// (x, y) points as an `(n, 2)` array.
fn points<'py>(py: Python<'py>, p: &[(f64, f64)]) -> PyResult<Bound<'py, PyAny>> {
    let flat: Vec<f64> = p.iter().flat_map(|&(x, y)| [x, y]).collect();
    Ok(PyArray1::from_vec(py, flat)
        .reshape([p.len(), 2])?
        .into_any())
}

/// A device's geometry: its layers with their polygons (`(n, 2)` arrays), and its ports.
#[pyfunction]
#[pyo3(signature = (kind, values, stack))]
fn device_geometry<'py>(
    py: Python<'py>,
    kind: &str,
    values: Vec<(String, f64)>,
    stack: Option<String>,
) -> PyResult<Bound<'py, PyDict>> {
    let d = design(kind, values, None, stack, None);
    let g = py.detach(|| facade::device_geometry(&d)).fail(py)?;
    let layers = PyList::empty(py);
    for l in &g.layers {
        let polygons = PyList::empty(py);
        for p in &l.polygons {
            let item = PyDict::new(py);
            item.set_item("outer", points(py, &p.outer)?)?;
            let holes = PyList::empty(py);
            for h in &p.holes {
                holes.append(points(py, h)?)?;
            }
            item.set_item("holes", holes)?;
            item.set_item("etch_depth_um", p.etch_depth_um)?;
            item.set_item("sidewall_deg", p.sidewall_deg)?;
            polygons.append(item)?;
        }
        let item = PyDict::new(py);
        item.set_item("name", &l.name)?;
        item.set_item("z_um", l.z_um)?;
        item.set_item("material", &l.material)?;
        item.set_item("background", &l.background)?;
        item.set_item("polygons", polygons)?;
        layers.append(item)?;
    }
    let ports = PyList::empty(py);
    for p in &g.ports {
        let item = PyDict::new(py);
        item.set_item("name", &p.name)?;
        item.set_item("position_um", p.position_um)?;
        item.set_item("direction_deg", p.direction_deg)?;
        item.set_item("width_um", p.width_um)?;
        item.set_item("layer", &p.layer)?;
        ports.append(item)?;
    }
    let out = PyDict::new(py);
    out.set_item("layers", layers)?;
    out.set_item("substrate", &g.substrate)?;
    out.set_item("cladding", &g.cladding)?;
    out.set_item("ports", ports)?;
    Ok(out)
}

/// The job file that computes a device's design, as text in a format.
#[pyfunction]
#[pyo3(signature = (kind, values, fidelity, stack, compact, name, format))]
#[allow(clippy::too_many_arguments)] // the design's fields, one for one
fn device_job(
    py: Python<'_>,
    kind: &str,
    values: Vec<(String, f64)>,
    fidelity: Option<String>,
    stack: Option<String>,
    compact: Option<PathBuf>,
    name: &str,
    format: &str,
) -> PyResult<String> {
    let d = design(kind, values, fidelity, stack, compact);
    py.detach(|| facade::device_job(&d, name, format)).fail(py)
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
    module.add_function(wrap_pyfunction!(run_tables, module)?)?;
    module.add_function(wrap_pyfunction!(export_run, module)?)?;
    module.add_function(wrap_pyfunction!(estimate_fields, module)?)?;
    module.add_function(wrap_pyfunction!(export_fields, module)?)?;
    module.add_function(wrap_pyfunction!(devices, module)?)?;
    module.add_function(wrap_pyfunction!(stacks, module)?)?;
    module.add_function(wrap_pyfunction!(device_spectrum, module)?)?;
    module.add_function(wrap_pyfunction!(device_derived, module)?)?;
    module.add_function(wrap_pyfunction!(device_geometry, module)?)?;
    module.add_function(wrap_pyfunction!(device_job, module)?)?;
    module.add_function(wrap_pyfunction!(_conformance, module)?)?;
    Ok(())
}
