"""photonoxide in MATLAB: the Python side of the ``.m`` wrappers shipped with this package.

MATLAB (R2022b or later) calls Python through its ``py.`` interface. The wrappers, in the
``+photonoxide`` folder that :func:`path` returns, call the functions here, which take and
return only what MATLAB converts without help: numbers, strings, lists of numbers, and dicts
that MATLAB's ``struct`` turns into structs. Arrays travel flat, in MATLAB's column-major order,
with their shape; complex ones as their real and imaginary parts. So the conversions are here,
in Python, where they are tested, and the ``.m`` files stay a few lines each.

In MATLAB::

    pyenv(Version="/path/to/python");            % a Python with photonoxide installed
    addpath(string(py.photonoxide.matlab.path()));
    n = photonoxide.refractive_index("si", [1.31 1.55]);

An error raised here carries its class in its message, ``photonoxide:InvalidValue: ...``, which
the wrappers turn into the MATLAB error identifier ``photonoxide:InvalidValue``.
"""

from __future__ import annotations

import functools
import json
import os
from array import array
from pathlib import Path
from typing import Any, Callable, TypeVar

import numpy as np

import photonoxide as _po

__all__ = [
    "path",
    "version",
    "materials",
    "refractive_index",
    "group_index",
    "slab_modes",
    "vector_modes",
    "run_job",
    "fdfd_s_parameters",
    "circuit_spectrum",
    "read_touchstone",
    "write_touchstone",
]

F = TypeVar("F", bound=Callable[..., Any])


def _identified(function: F) -> F:
    """Raises photonoxide's errors, and a timeout, with their class first in the message, for
    the wrappers to make MATLAB error identifiers of."""

    @functools.wraps(function)
    def call(*args: Any, **kwargs: Any) -> Any:
        try:
            return function(*args, **kwargs)
        except _po.errors.Error as e:
            raise type(e)(f"photonoxide:{type(e).__name__}: {e}", **_fields(e)) from e
        except TimeoutError as e:
            raise TimeoutError(f"photonoxide:Timeout: {e}") from e

    return call  # type: ignore[return-value]


def _fields(e: BaseException) -> dict[str, Any]:
    return {k: v for k, v in vars(e).items() if not k.startswith("_")}


def _vector(values: Any) -> np.ndarray:
    """Numbers as MATLAB passes them (a number, a list, an array.array, a memoryview) as a
    flat array."""
    return np.asarray(list(values) if isinstance(values, memoryview) else values, dtype=float).ravel()


def _flat(values: np.ndarray) -> dict[str, Any]:
    """An array as MATLAB rebuilds it: its values in column-major order, real and imaginary
    parts apart, and its shape (at least two dimensions, as MATLAB's are)."""
    a = np.asarray(values)
    shape = list(a.shape) if a.ndim >= 2 else [1, a.size]
    flat = a.ravel(order="F")
    out: dict[str, Any] = {"shape": array("d", [float(n) for n in shape])}
    if np.iscomplexobj(flat):
        out["re"] = array("d", flat.real.tolist())
        out["im"] = array("d", flat.imag.tolist())
    else:
        out["re"] = array("d", flat.astype(float).tolist())
    return out


def _option(value: Any) -> Any:
    """MATLAB's "not given": an empty string or array, or NaN, is None."""
    if value is None:
        return None
    if isinstance(value, str) and value == "":
        return None
    if isinstance(value, float) and value != value:
        return None
    return value


def path() -> str:
    """The folder holding the ``+photonoxide`` wrappers, for MATLAB's ``addpath``."""
    return str(Path(__file__).resolve().parent)


def version() -> dict[str, str]:
    """The package's version and photonoxide's (the crate it was built with)."""
    return {"package": _po.__version__, "core": _po.core_version}


@_identified
def materials() -> dict[str, Any]:
    """The catalogue's materials as columns: ``id``, ``name``, ``formula``, and each one's
    default model's ``model`` (its id), ``axes`` (joined by commas) and ``range_um`` (2 × n)."""
    ms = _po.materials()
    default = [next(m for m in x.models if m.default) for x in ms]
    return {
        "id": [m.id for m in ms],
        "name": [m.name for m in ms],
        "formula": [m.formula for m in ms],
        "model": [d.id for d in default],
        "axes": [",".join(d.axes) for d in default],
        "range_um": _flat(np.array([d.range_um for d in default]).T),
    }


def _index_options(model: Any, axis: Any, temperature_k: Any, composition: Any) -> dict[str, Any]:
    return {
        "model": _option(model),
        "axis": _option(axis),
        "temperature_k": _option(None if temperature_k is None else float(temperature_k)),
        "composition": _option(None if composition is None else float(composition)),
    }


@_identified
def refractive_index(
    material: str,
    wavelength_um: Any,
    model: Any = None,
    axis: Any = None,
    temperature_k: Any = None,
    composition: Any = None,
) -> dict[str, Any]:
    """:func:`photonoxide.refractive_index` at a list of wavelengths, flat."""
    n = _po.refractive_index(
        str(material),
        wavelength_um=_vector(wavelength_um),
        **_index_options(model, axis, temperature_k, composition),
    )
    return _flat(np.asarray(n, dtype=complex))


@_identified
def group_index(
    material: str,
    wavelength_um: Any,
    model: Any = None,
    axis: Any = None,
    temperature_k: Any = None,
    composition: Any = None,
) -> dict[str, Any]:
    """:func:`photonoxide.group_index` at a list of wavelengths, flat."""
    n = _po.group_index(
        str(material),
        wavelength_um=_vector(wavelength_um),
        **_index_options(model, axis, temperature_k, composition),
    )
    return _flat(np.asarray(n, dtype=float))


@_identified
def slab_modes(
    below: float,
    core: float,
    above: float,
    thickness_um: float,
    polarization: str,
    wavelength_um: float,
    x_um: Any = None,
) -> dict[str, Any]:
    """:func:`photonoxide.slab_modes`: the modes' ``order`` and ``effective_index`` as lists,
    and their fields at ``x_um`` as a matrix, a column per mode."""
    x = _vector([] if x_um is None else x_um)
    modes = _po.slab_modes(
        below=float(below),
        core=float(core),
        above=float(above),
        thickness_um=float(thickness_um),
        polarization=str(polarization),
        wavelength_um=float(wavelength_um),
        x_um=x,
    )
    field = np.array([m.field for m in modes]).T.reshape(x.size, len(modes))
    return {
        "order": array("d", [float(m.order) for m in modes]),
        "effective_index": array("d", [m.effective_index for m in modes]),
        "field": _flat(field),
    }


@_identified
def vector_modes(
    x_um: Any,
    y_um: Any,
    eps_re: Any,
    eps_im: Any,
    wavelength_um: float,
    count: Any = 1,
    near_index: Any = None,
    boundaries: Any = None,
    pml_um: Any = None,
    pml_strength: Any = 0.0,
    timeout_s: Any = 600.0,
) -> list[dict[str, Any]]:
    """:func:`photonoxide.vector_modes`, the permittivity as MATLAB holds it: ``eps_re`` and
    ``eps_im`` flat in column-major order, a row per y and a column per x (``ny × nx``). Each
    mode a dict: ``effective_index`` (real and imaginary parts), ``te_fraction``, ``x_um``,
    ``y_um`` and its six fields, flat with their shape, ``ny × nx``."""
    x = _vector(x_um)
    y = _vector(y_um)
    ny, nx = y.size - 1, x.size - 1
    re = _vector(eps_re)
    im = _vector(eps_im) if eps_im is not None and len(eps_im) else np.zeros_like(re)
    if re.size != nx * ny or im.size != nx * ny:
        raise ValueError(
            f"the permittivity needs ny × nx = {ny} × {nx} values, got {re.size} and {im.size}"
        )
    eps = (re + 1j * im).reshape((ny, nx), order="F")
    modes = _po.vector_modes(
        x_um=x,
        y_um=y,
        permittivity=eps,
        wavelength_um=float(wavelength_um),
        count=int(count),
        near_index=_option(None if near_index is None else float(near_index)),
        boundaries=tuple(str(b) for b in boundaries) if boundaries else ("zero",) * 4,
        pml_um=tuple(_vector(pml_um)) if pml_um is not None and len(pml_um) else (0.0,) * 4,
        pml_strength=float(pml_strength),
        timeout_s=_option(None if timeout_s is None else float(timeout_s)),
    )
    return [
        {
            "effective_index": array("d", [m.effective_index.real, m.effective_index.imag]),
            "te_fraction": m.te_fraction,
            "x_um": array("d", m.x_um.tolist()),
            "y_um": array("d", m.y_um.tolist()),
            **{name: _flat(getattr(m, name)) for name in ("ex", "ey", "ez", "hx", "hy", "hz")},
        }
        for m in modes
    ]


def _job(job: str) -> Any:
    """A job as MATLAB gives it: a path, or the job as JSON text (``jsonencode`` of a
    struct), which starts with ``{``."""
    job = str(job)
    return json.loads(job) if job.lstrip().startswith("{") else job


@_identified
def run_job(job: str, runs_dir: str = "runs", timeout_s: Any = 3600.0) -> dict[str, Any]:
    """:func:`photonoxide.run_job`: the run's ``dir``, its ``events`` as JSON text each (for
    MATLAB's ``jsondecode``) and ``stopped`` (empty when it finished)."""
    run = _po.run_job(
        _job(job),
        runs_dir=str(runs_dir),
        timeout_s=_option(None if timeout_s is None else float(timeout_s)),
    )
    return {
        "dir": str(run.dir),
        "events": [json.dumps(e) for e in run.events],
        "stopped": run.stopped or "",
    }


@_identified
def fdfd_s_parameters(job: str, wavelength_um: Any = None) -> dict[str, Any]:
    """:func:`photonoxide.fdfd_s_parameters`: ``s`` flat with its shape, ``nλ × n × n``, and
    ``effective_index``, ``nλ × n``."""
    w = None if wavelength_um is None or len(_vector(wavelength_um)) == 0 else _vector(wavelength_um)
    s = _po.fdfd_s_parameters(_job(job), wavelength_um=w)
    return {
        "ports": list(s.ports),
        "wavelength_um": array("d", s.wavelength_um.tolist()),
        "s": _flat(s.s),
        "effective_index": _flat(s.effective_index),
        "cell_um": array("d", list(s.cell_um)),
        "polarization": s.polarization,
    }


def _spectrum(s: _po.Spectrum) -> dict[str, Any]:
    return {
        "ports": list(s.ports),
        "wavelength_um": array("d", np.asarray(s.wavelength_um).tolist()),
        "s": _flat(s.s),
    }


@_identified
def circuit_spectrum(netlist: str, wavelength_um: Any) -> dict[str, Any]:
    """:func:`photonoxide.circuit_spectrum`, the netlist as JSON text (``jsonencode`` of a
    struct with ``instances``, ``connections`` and ``ports``): ``s`` flat, ``nλ × n × n``."""
    data = json.loads(str(netlist))
    return _spectrum(
        _po.circuit_spectrum(
            instances=data["instances"],
            connections=data.get("connections", []),
            ports=data["ports"],
            wavelength_um=_vector(wavelength_um),
        )
    )


@_identified
def read_touchstone(file: str, convention: str) -> dict[str, Any]:
    """:func:`photonoxide.read_touchstone`: ``s`` flat, ``nλ × n × n``."""
    return _spectrum(_po.read_touchstone(str(file), convention=str(convention)))


@_identified
def write_touchstone(
    file: str,
    ports: Any,
    wavelength_um: Any,
    s_re: Any,
    s_im: Any,
    convention: str,
    significant_digits: Any = None,
) -> None:
    """:func:`photonoxide.write_touchstone`, S as MATLAB holds it: ``nλ × n × n``, flat in
    column-major order, real and imaginary parts apart."""
    w = _vector(wavelength_um)
    names = [str(p) for p in ports]
    n = len(names)
    s = (_vector(s_re) + 1j * _vector(s_im)).reshape((w.size, n, n), order="F")
    digits = _option(None if significant_digits is None else float(significant_digits))
    _po.write_touchstone(
        os.fspath(file),
        _po.Spectrum(ports=names, wavelength_um=w, s=s),
        convention=str(convention),
        significant_digits=None if digits is None else int(digits),
    )
