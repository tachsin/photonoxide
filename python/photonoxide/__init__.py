"""Validated photonics from Rust, for Python: materials with provenance, mode solvers, jobs and
2D FDFD S-parameters, circuits and Touchstone files, from
`photonoxide <https://github.com/tachsin/photonoxide>`_.

220 nm of silicon in oxide at 1550 nm, each material with its published dispersion, and the
slab's modes solved exactly::

    import photonoxide as po

    si = po.refractive_index("si", wavelength_um=1.55).real    # Li 1980
    ox = po.refractive_index("sio2", wavelength_um=1.55).real  # Malitson 1965
    for polarization in ("te", "tm"):
        for mode in po.slab_modes(below=ox, core=si, above=ox, thickness_um=0.22,
                                  polarization=polarization, wavelength_um=1.55):
            print(f"{polarization.upper()}{mode.order}: n_eff = {mode.effective_index:.4f}")

It prints ``TE0: n_eff = 2.8475`` and ``TM0: n_eff = 2.0531``.

**Units are in the names,** as photonoxide's job files have them: ``wavelength_um``,
``thickness_um``, ``loss_db_per_cm``. Functions that loop over wavelengths in Rust take one or an
array of them (``wavelength_um=np.linspace(1.5, 1.6, 101)``) and return one value or an array of
the same shape.

**Arrays are NumPy's, in C order, with their coordinates.** A field on a cross-section is
``(ny, nx)``: ``ex[j, i]`` is E_x at ``(x_um[i], y_um[j])``. S-matrices at many wavelengths are
``(nλ, n, n)``: ``s[k, q, p]`` is S_qp at the k-th wavelength, from port p into port q, in
photonoxide's e^(−iωt) convention.

**Errors** are photonoxide's, with its messages, as the classes of :mod:`photonoxide.errors`,
each also a built-in (an invalid value is a ``ValueError``).

**Long calls** (:func:`vector_modes`, :func:`run_job`) take ``timeout_s`` and raise
``TimeoutError`` past it; Ctrl+C stops them too. Every call releases the GIL while Rust works.

**The same numbers as Rust.** Each function calls photonoxide's façade, which calls the library
as a Rust program would; the package's tests check its results against the library's to the bit.

MATLAB reaches the same functions through this package: see :mod:`photonoxide.matlab`.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass
from importlib import metadata
from pathlib import Path
from typing import Any, Mapping, Sequence, Union

import numpy as np

from . import errors
from ._photonoxide import core_version
from . import _photonoxide as _native

__all__ = [
    "__version__",
    "core_version",
    "errors",
    "Material",
    "IndexModel",
    "SlabMode",
    "VectorMode",
    "JobRun",
    "SParameters",
    "Spectrum",
    "materials",
    "refractive_index",
    "group_index",
    "slab_modes",
    "vector_modes",
    "check_job",
    "run_job",
    "fdfd_s_parameters",
    "circuit_spectrum",
    "read_touchstone",
    "write_touchstone",
]

try:
    __version__: str = metadata.version("photonoxide")
except metadata.PackageNotFoundError:  # built in place, not installed
    __version__ = core_version

#: A job: the path of a job file (TOML, JSON or YAML, by its extension), or the job as data, a
#: dict with a job file's fields (``name``, ``timeout_minutes``, ``task``, ...).
Job = Union[str, "os.PathLike[str]", Mapping[str, Any]]


@dataclass(frozen=True)
class IndexModel:
    """An index model of a material, from one paper."""

    #: Its id, e.g. ``"li-1980"``.
    id: str
    #: Its name, e.g. ``"Li 1980"``.
    name: str
    #: Whether it is the material's default model.
    default: bool
    #: The indices it gives: ``["isotropic"]``, or ``["ordinary", "extraordinary"]``.
    axes: list[str]
    #: The wavelengths it is valid for at its default conditions, µm.
    range_um: tuple[float, float]
    #: Its default temperature, K, when it has one.
    temperature_k: float | None
    #: Its default composition (e.g. x in AlₓGa₁₋ₓAs), when it has one.
    composition: float | None
    #: The accuracy its paper states.
    accuracy: str
    #: Where it comes from: each paper's citation, DOI and place.
    sources: list[str]


@dataclass(frozen=True)
class Material:
    """A material of photonoxide's catalogue, with its index models."""

    #: Its id, e.g. ``"si"`` or ``"linbo3"``: what :func:`refractive_index` takes.
    id: str
    #: Its name, e.g. ``"Silicon"``.
    name: str
    #: Its formula, e.g. ``"Si"``.
    formula: str
    #: Its index models, the default first.
    models: list[IndexModel]


@dataclass(frozen=True)
class SlabMode:
    """A guided mode of a three-layer slab."""

    #: ``"te"`` or ``"tm"``.
    polarization: str
    #: Its order: 0 for the fundamental mode of its polarization.
    order: int
    #: The effective index n_eff = β/k.
    effective_index: float
    #: Where its field is given across the slab, µm, 0 at the core's bottom.
    x_um: np.ndarray
    #: Its field there: E_y for TE, H_y for TM, unnormalized, 1 (TE) or h/q̄ (TM) at x = 0.
    field: np.ndarray


@dataclass(frozen=True)
class VectorMode:
    """A full-vector mode of a cross-section, with its six field components at the centres of
    the grid's cells, each ``(ny, nx)``: ``ex[j, i]`` at ``(x_um[i], y_um[j])``."""

    #: The effective index n_eff = β/k; its imaginary part, ≥ 0 in a passive structure, is the
    #: loss along z.
    effective_index: complex
    #: The share of the transverse magnetic field in H_y: near 1 for a TE-like mode (E mostly
    #: along x), near 0 for a TM-like one.
    te_fraction: float
    #: The cells' centres along x, µm.
    x_um: np.ndarray
    #: The cells' centres along y, µm.
    y_um: np.ndarray
    #: E_x, in units of Z₀ times H's.
    ex: np.ndarray
    #: E_y.
    ey: np.ndarray
    #: E_z.
    ez: np.ndarray
    #: H_x, the largest transverse component of H at the grid's nodes 1.
    hx: np.ndarray
    #: H_y.
    hy: np.ndarray
    #: H_z.
    hz: np.ndarray


@dataclass(frozen=True)
class JobRun:
    """A job's run: its folder and its record."""

    #: The run's folder, ``<runs_dir>/<YYYYMMDD>-<HHMMSS>-<name>``, which the photonoxide program
    #: replays (``photonoxide view <folder>``).
    dir: Path
    #: Its events in order, each a dict with its ``"type"``, as the folder's ``events.jsonl``
    #: has them.
    events: list[dict[str, Any]]
    #: Why it ended early, if it did: ``"timeout"`` or ``"requested"``.
    stopped: str | None


@dataclass(frozen=True)
class SParameters:
    """An ``"fdfd"`` job's S-parameters by 2D FDFD: an estimate, not a device's performance
    in 3D."""

    #: The ports' names, in the job's order, e.g. ``"1 (left, x = -3.5 um)"``.
    ports: list[str]
    #: The wavelengths, µm.
    wavelength_um: np.ndarray
    #: The power-normalized S-matrices, ``(nλ, n, n)``: ``s[k, q, p]`` from port p into port q,
    #: the phases referred to the ports' columns.
    s: np.ndarray
    #: Each port mode's effective index at each wavelength, ``(nλ, n)``.
    effective_index: np.ndarray
    #: The grid's cell, dx and dy, µm.
    cell_um: tuple[float, float]
    #: The slab mode's polarization the plane's indices come from: ``"te"`` (H along z in the
    #: plane) or ``"tm"`` (E along z).
    polarization: str


@dataclass(frozen=True)
class Spectrum:
    """S-matrices at several wavelengths between named ports: a circuit's spectrum, or a
    Touchstone file's."""

    #: The ports' names, in the matrices' order.
    ports: list[str]
    #: The vacuum wavelengths, µm.
    wavelength_um: np.ndarray
    #: The S-matrices, ``(nλ, n, n)``: ``s[k, q, p]`` from port p into port q, in photonoxide's
    #: e^(−iωt) convention.
    s: np.ndarray


def _wavelengths(wavelength_um: Any) -> tuple[np.ndarray, tuple[int, ...]]:
    w = np.asarray(wavelength_um, dtype=np.float64)
    return np.ascontiguousarray(w.ravel()), w.shape


def _shaped(values: np.ndarray, shape: tuple[int, ...]) -> Any:
    if shape == ():
        return values[0].item()
    return values.reshape(shape)


def materials() -> list[Material]:
    """Every material of photonoxide's catalogue, with its index models, their ranges and their
    sources by DOI."""
    return [
        Material(
            id=m["id"],
            name=m["name"],
            formula=m["formula"],
            models=[
                IndexModel(**{**model, "range_um": tuple(model["range_um"])})
                for model in m["models"]
            ],
        )
        for m in _native.materials()
    ]


def refractive_index(
    material: str,
    *,
    wavelength_um: Any,
    model: str | None = None,
    axis: str | None = None,
    temperature_k: float | None = None,
    composition: float | None = None,
) -> Any:
    """The refractive index n + iκ (κ ≥ 0) of a material of the catalogue (its id, e.g.
    ``"si"``; see :func:`materials`) at vacuum wavelengths, µm: a complex number for one
    wavelength, an array of the wavelengths' shape for several.

    ``model`` picks an index model other than the default; ``axis`` (``"ordinary"`` or
    ``"extraordinary"``) is needed for a material with two; ``temperature_k`` and
    ``composition`` set a model's conditions. A wavelength outside the model's data raises
    :class:`~photonoxide.errors.OutsideValidity`: photonoxide doesn't extrapolate.
    """
    w, shape = _wavelengths(wavelength_um)
    n = _native.refractive_index(material, w, model, axis, temperature_k, composition)
    return _shaped(n, shape)


def group_index(
    material: str,
    *,
    wavelength_um: Any,
    model: str | None = None,
    axis: str | None = None,
    temperature_k: float | None = None,
    composition: float | None = None,
) -> Any:
    """The bulk group index n − λ dn/dλ of a material of the catalogue at vacuum wavelengths,
    µm, as :func:`refractive_index` takes them: a float for one, an array for several."""
    w, shape = _wavelengths(wavelength_um)
    n = _native.group_index(material, w, model, axis, temperature_k, composition)
    return _shaped(n, shape)


def slab_modes(
    *,
    below: float,
    core: float,
    above: float,
    thickness_um: float,
    polarization: str,
    wavelength_um: float,
    x_um: Any = None,
) -> list[SlabMode]:
    """The guided modes of one polarization (``"te"`` or ``"tm"``) of a lossless three-layer
    slab, a core of index ``core`` and thickness ``thickness_um`` between ``below`` and
    ``above``, at ``wavelength_um``, solved exactly: fundamental first, each with its field at
    ``x_um`` (0 at the core's bottom; none when not given). Empty when the slab guides none."""
    x = np.ascontiguousarray(np.asarray([] if x_um is None else x_um, dtype=np.float64).ravel())
    modes = _native.slab_modes(
        float(below),
        float(core),
        float(above),
        float(thickness_um),
        polarization,
        float(wavelength_um),
        x,
    )
    return [SlabMode(**m) for m in modes]


def vector_modes(
    *,
    x_um: Any,
    y_um: Any,
    permittivity: Any,
    wavelength_um: float,
    count: int = 1,
    near_index: float | None = None,
    boundaries: Sequence[str] = ("zero", "zero", "zero", "zero"),
    pml_um: Sequence[float] = (0.0, 0.0, 0.0, 0.0),
    pml_strength: float = 0.0,
    timeout_s: float | None = 600.0,
) -> list[VectorMode]:
    """The ``count`` modes of a cross-section whose effective indices are nearest
    ``near_index`` (the highest index in it when ``None``: the fundamental modes), nearest
    first, by photonoxide's full-vector finite-difference solver, with their fields.

    The cross-section is a rectilinear grid, its nodes ``x_um`` (``nx + 1`` of them, increasing)
    and ``y_um`` (``ny + 1``), and the relative permittivity of each cell, ``(ny, nx)``:
    ``permittivity[j, i]`` fills the cell between ``x_um[i]``, ``x_um[i + 1]``, ``y_um[j]`` and
    ``y_um[j + 1]`` (real, or complex with a positive imaginary part for loss). ``boundaries``
    are the west (smallest x), east, south (smallest y) and north edges: ``"zero"``, or a mirror
    plane of symmetry, ``"electric"`` (for a TE-like mode, the vertical plane through the core)
    or ``"magnetic"`` (the horizontal one). ``pml_um`` are perfectly matched layers inside the
    same edges, µm thick, with stretching ``pml_strength`` (a few units).

    The solve stops after ``timeout_s`` seconds (``None``: never) with ``TimeoutError``.
    """
    eps = np.ascontiguousarray(np.asarray(permittivity, dtype=np.complex128))
    if eps.ndim != 2:
        raise ValueError(f"permittivity must be a 2D array (ny, nx), not of shape {eps.shape}")
    modes = _native.vector_modes(
        np.ascontiguousarray(np.asarray(x_um, dtype=np.float64).ravel()),
        np.ascontiguousarray(np.asarray(y_um, dtype=np.float64).ravel()),
        eps,
        float(wavelength_um),
        int(count),
        None if near_index is None else float(near_index),
        [str(b) for b in boundaries],
        [float(p) for p in pml_um],
        float(pml_strength),
        None if timeout_s is None else float(timeout_s),
    )
    return [VectorMode(**m) for m in modes]


def _json_default(value: Any) -> Any:
    if isinstance(value, np.generic):
        return value.item()
    if isinstance(value, np.ndarray):
        return value.tolist()
    if isinstance(value, os.PathLike):
        return os.fspath(value)
    raise TypeError(f"{type(value).__name__} can't be written in a job or a netlist")


def _job(job: Job) -> tuple[str | None, str | None]:
    if isinstance(job, Mapping):
        return None, json.dumps(job, default=_json_default)
    return os.fspath(job), None


def check_job(job: Job) -> None:
    """Checks a job without running it, as the photonoxide program does before a run: raises
    what the run would raise for its settings."""
    path, text = _job(job)
    _native.check_job(path, text, "json")


def run_job(
    job: Job, *, runs_dir: str | os.PathLike[str] = "runs", timeout_s: float | None = 3600.0
) -> JobRun:
    """Runs a job as ``photonoxide run --headless`` does, into a new folder under ``runs_dir``,
    and returns its record. Every kind of job runs: ``"structure"``, ``"modes"``, ``"fdfd"`` and
    ``"fdtd"``.

    The run stops at ``timeout_s`` seconds (``None``: no limit of its own) or at the job's own
    ``timeout_minutes``, whichever comes first, or on Ctrl+C. A run stopped by its timeout is
    still returned, its record ending there and its ``stopped`` saying ``"timeout"``; Ctrl+C
    raises ``KeyboardInterrupt`` once the run has stopped.
    """
    path, text = _job(job)
    run = _native.run_job(
        path, text, "json", os.fspath(runs_dir), None if timeout_s is None else float(timeout_s)
    )
    return JobRun(
        dir=Path(run["dir"]),
        events=[json.loads(e) for e in run["events"]],
        stopped=run["stopped"],
    )


def fdfd_s_parameters(job: Job, *, wavelength_um: Any = None) -> SParameters:
    """The S-parameters of an ``"fdfd"`` job's device by 2D FDFD, at ``wavelength_um`` (one or
    an array), or at the job's own wavelengths (its sweep, or its one wavelength) when
    ``None``. A 2D result, by the effective index method: an estimate, not a device's
    performance in 3D. The wavelengths are solved side by side on every core, the same bits on
    any number of them."""
    path, text = _job(job)
    w = None if wavelength_um is None else _wavelengths(wavelength_um)[0]
    s = _native.fdfd_s_parameters(path, text, "json", w)
    return SParameters(**{**s, "cell_um": tuple(s["cell_um"])})


def circuit_spectrum(
    *,
    instances: Mapping[str, Mapping[str, Any]],
    connections: Sequence[Sequence[str]] = (),
    ports: Mapping[str, str],
    wavelength_um: Any,
) -> Spectrum:
    """The S-matrices at ``wavelength_um`` of a circuit given as data: its ``instances`` by
    name, the ``connections`` between their ports (``"instance.port"`` pairs), and its external
    ``ports``, each name to the instance's port it exposes, in the matrices' order::

        import numpy as np
        import photonoxide as po

        arm = dict(kind="waveguide", n_eff=2.44506, n_g=4.172901, wavelength_um=1.55)
        mzi = po.circuit_spectrum(
            instances={
                "split": {"kind": "coupler", "coupling": 0.5},
                "upper": {**arm, "length": 150.0},
                "lower": {**arm, "length": 100.0},
                "combine": {"kind": "coupler", "coupling": 0.5},
            },
            connections=[("split.o3", "upper.o1"), ("split.o4", "lower.o1"),
                         ("upper.o2", "combine.o2"), ("lower.o2", "combine.o1")],
            ports={"in1": "split.o2", "in2": "split.o1", "out1": "combine.o3", "out2": "combine.o4"},
            wavelength_um=np.linspace(1.5, 1.6, 1001),
        )
        bar = np.abs(mzi.s[:, 2, 0]) ** 2   # from in1 to out1

    Each instance names its ``kind``, the settings that build it, and its parameters' values
    (a parameter not given takes its default):

    - ``"waveguide"``: its mode's ``n_eff`` and ``n_g`` at ``wavelength_um``, optionally
      ``dispersion_ps_per_nm_km``; parameters ``length`` (µm) and ``loss`` (dB/cm).
    - ``"bend"``: ``radius_um`` and the guide's settings with ``loss_db_per_cm``; parameter
      ``angle`` (degrees).
    - ``"phase-shifter"``: parameter ``phase`` (rad).
    - ``"coupler"``: ideal, 2 × 2, optionally ``excess_loss_db``; parameter ``coupling`` (κ²).
    - ``"directional-coupler"``: ``coupling_per_um`` and the guide's settings; parameter
      ``length`` (µm).
    - ``"y-branch"``: parameter ``excess_loss`` (dB).
    - ``"ring-all-pass"`` and ``"ring-add-drop"``: the guide's settings, optionally
      ``round_trip_loss_db``; parameters ``length`` (the round trip, µm), ``coupling`` and
      ``coupling_drop``.
    - ``"terminator"``: a perfect absorber on its one port, ``o1``.
    - ``"touchstone"``: the S-parameters of a Touchstone ``file`` in its ``convention``
      (``"physics"`` or ``"engineering"``), ports ``o1``, ``o2``, ...
    - ``"fixed"``: an S-matrix the same at every wavelength: ``ports`` (names) and ``s`` (rows
      of ``[re, im]`` pairs).

    Ports are named ``o1``, ``o2``, ... as photonoxide's components name them (the rings: ``in``,
    ``through``, ``add`` and ``drop``).
    """
    netlist = json.dumps(
        {
            "instances": instances,
            "connections": [list(c) for c in connections],
            "ports": ports,
        },
        default=_json_default,
    )
    w, _ = _wavelengths(wavelength_um)
    return Spectrum(**_native.circuit_spectrum(netlist, w))


def read_touchstone(path: str | os.PathLike[str], *, convention: str) -> Spectrum:
    """The S-parameters of a Touchstone file (Version 1 or 2.0), its values in the time
    convention ``convention``: ``"physics"`` (e^(−iωt), photonoxide's) or ``"engineering"``
    (e^(+jωt), most RF tools'). The format doesn't say which, so the reader must. In the file's
    order: increasing frequency, so decreasing wavelength, λ = c/f; the ports named ``o1``,
    ``o2``, ..."""
    return Spectrum(**_native.read_touchstone(os.fspath(path), convention))


def write_touchstone(
    path: str | os.PathLike[str],
    spectrum: Spectrum,
    *,
    convention: str,
    significant_digits: int | None = None,
) -> None:
    """Writes a spectrum to a Touchstone file (Version 2.0, real and imaginary parts,
    frequencies c/λ in Hz, increasing), its values in the time convention ``convention``
    (``"physics"`` or ``"engineering"``). Each number is the shortest decimal that reads back
    to the same float, or ``significant_digits`` (1 to 17) of it. The ports' names aren't kept:
    Touchstone numbers its ports."""
    s = np.ascontiguousarray(np.asarray(spectrum.s, dtype=np.complex128))
    w, _ = _wavelengths(spectrum.wavelength_um)
    _native.write_touchstone(
        os.fspath(path),
        [str(p) for p in spectrum.ports],
        w,
        s,
        convention,
        None if significant_digits is None else int(significant_digits),
    )
