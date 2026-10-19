"""Validated photonics from Rust, for Python: materials with provenance, mode solvers, jobs and
2D FDFD S-parameters, circuits and Touchstone files, and devices that know their geometry, from
`photonoxide <https://github.com/tachsin/photonoxide>`_.

220 nm of silicon in oxide at 1550 nm, each material with its published dispersion, and the
slab's modes solved exactly::

    import photonoxide as phox

    si = phox.refractive_index("si", wavelength_um=1.55).real    # Li 1980
    ox = phox.refractive_index("sio2", wavelength_um=1.55).real  # Malitson 1965
    for polarization in ("te", "tm"):
        for mode in phox.slab_modes(below=ox, core=si, above=ox, thickness_um=0.22,
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
    "Device",
    "DeviceParameter",
    "DeviceModel",
    "StatedError",
    "ValidityRange",
    "DeviceSpectrum",
    "DeviceFigure",
    "DeviceGeometry",
    "GeometryLayer",
    "DevicePolygon",
    "DevicePort",
    "devices",
    "device_spectrum",
    "device_derived",
    "device_geometry",
    "device_job",
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
    """Of the ``count`` modes of a cross-section whose effective indices are nearest
    ``near_index`` (the highest index in it when ``None``: the fundamental modes), the guided
    ones, nearest first, by photonoxide's full-vector finite-difference solver, with their
    fields.

    A mode is guided when its effective index is above the highest index of the media around
    the guides, those at the window's open edges (``"zero"``), as a modes job keeps them. The
    window is closed: below that index it has modes of its own, standing waves of its box and
    not the guides', and they are left out. So when fewer modes are guided than ``count``,
    fewer come back, and an empty list when none is; nothing more is solved to make the count
    up. With PMLs (``pml_um`` not all 0, ``pml_strength`` above 0) the window is open instead,
    and every mode solved comes back: leaky modes below the media around the guides, with
    their loss, and the PMLs' own modes, their loss large.

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
        import photonoxide as phox

        arm = dict(kind="waveguide", n_eff=2.44506, n_g=4.172901, wavelength_um=1.55)
        mzi = phox.circuit_spectrum(
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

    An instance can instead name a ``device`` of the library (see :func:`devices`) in place of
    a ``kind``: its design as a job's ``[device]`` table holds it, ``kind``, ``fidelity`` (its
    cheapest when not given), ``stack`` and ``parameters`` (numbers in their units, or
    expressions such as ``"450 nm"`` or ``"w/2.5"``)::

        ring = {"device": {"kind": "all-pass ring", "fidelity": "analytic",
                           "parameters": {"radius": 5.0, "gap": 0.18}}}

    It is built at its fidelity, as :func:`device_spectrum` builds it, and its component's
    parameters take the values the design gives them; the instance's other members then set
    them by name (``"loss": 3.0``), as for a kind.

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


# --- devices --------------------------------------------------------------------------------


@dataclass(frozen=True)
class DeviceParameter:
    """A parameter of a device: its name, unit, default and range."""

    #: Its name, e.g. ``"gap"``.
    name: str
    #: Its unit, e.g. ``"µm"`` or ``"°"``, or ``""`` for a pure number.
    unit: str
    #: Its default.
    default: float
    #: The smallest value it takes.
    min: float
    #: The largest.
    max: float


@dataclass(frozen=True)
class StatedError:
    """A model's stated error: the largest difference in one quantity against another
    fidelity, measured by photonoxide's validation report."""

    #: The quantity, e.g. ``"n_eff"``.
    quantity: str
    #: The largest difference, in ``unit``.
    bound: float
    #: Its unit: ``""`` for a pure number, ``"%"`` for a relative one.
    unit: str
    #: The fidelity it is measured against.
    against: str


@dataclass(frozen=True)
class ValidityRange:
    """A range a model holds over, for one parameter or the wavelength (µm)."""

    #: The parameter's name, or ``"wavelength"``.
    name: str
    #: The smallest value.
    min: float
    #: The largest.
    max: float


@dataclass(frozen=True)
class DeviceModel:
    """One of a device's models: its fidelity, what computes it, its write-up and papers, its
    stated error against the next fidelity up, and where it holds."""

    #: ``"analytic"``, ``"compact"``, ``"2D"``, ``"3D"`` or ``"measured"``.
    fidelity: str
    #: What computes it, in a sentence.
    name: str
    #: Its write-up in photonoxide's docs/methods, e.g. ``"devices"``.
    write_up: str
    #: The papers it implements, each with its DOI.
    papers: list[str]
    #: Its error against the next fidelity up over its validity; ``None`` for the highest.
    error: StatedError | None
    #: Where it holds: a range for each parameter it limits, and ``"wavelength"``.
    validity: list[ValidityRange]


@dataclass(frozen=True)
class Device:
    """A device of photonoxide's library: a component that knows its geometry."""

    #: Its kind, what the device functions take, e.g. ``"directional coupler"``.
    kind: str
    #: What it is, in a sentence or two.
    about: str
    #: Its parameters, in order.
    parameters: list[DeviceParameter]
    #: Its models, cheapest first: the first is a design's when it names none.
    fidelities: list[DeviceModel]
    #: Its ports' names, in the order of its S-matrices.
    ports: list[str]


@dataclass(frozen=True)
class DeviceSpectrum:
    """A device's S-parameters over wavelength, with the model they come from. A 2D result is
    an estimate, not the device's performance."""

    #: The device's ports' names, in the matrices' order.
    ports: list[str]
    #: The vacuum wavelengths, µm.
    wavelength_um: np.ndarray
    #: The S-matrices, ``(nλ, n, n)``: ``s[k, q, p]`` from port p into port q, in photonoxide's
    #: e^(−iωt) convention.
    s: np.ndarray
    #: The fidelity of the model: ``"analytic"``, ``"2D"``, ``"3D"``, ...
    fidelity: str
    #: The grid its figures name, or ``"exact: no grid"`` for a closed form; a fit names the grid
    #: of the solves it was fitted to.
    grid: str
    #: What the model reproduces: a paper with its DOI and equation, or a solver with its grid.
    source: str
    #: The wavelengths it holds for, µm, if limited.
    validity_um: tuple[float, float] | None


@dataclass(frozen=True)
class DeviceFigure:
    """One of a device's figures, with the fidelity and grid it comes from."""

    #: What it is, e.g. ``"effective index"``.
    name: str
    #: Its symbol, unique among a device's figures, e.g. ``"n_eff"``.
    symbol: str
    #: Its value, in ``unit``.
    value: float
    #: Its unit, e.g. ``"µm"``, or ``""`` for a pure number.
    unit: str
    #: The fidelity of the model that gives it.
    fidelity: str
    #: The grid it was computed on, or ``"exact: no grid"`` for a closed form.
    grid: str


@dataclass(frozen=True)
class DevicePolygon:
    """A polygon with holes, seen from above, and how its edges were etched."""

    #: Its outer boundary, ``(n, 2)``: (x, y) in µm, counterclockwise, the first vertex not
    #: repeated.
    outer: np.ndarray
    #: Its holes, each ``(n, 2)``, clockwise.
    holes: list[np.ndarray]
    #: How deep the etch around it went from the layer's top, µm: ``None`` through the layer (a
    #: strip), less than the layer's thickness for a rib.
    etch_depth_um: float | None
    #: Its walls' angle from the layer's plane, degrees: 90 vertical.
    sidewall_deg: float


@dataclass(frozen=True)
class GeometryLayer:
    """A layer of a device's stack, and the polygons drawn on it."""

    #: Its name, e.g. ``"Si"``.
    name: str
    #: Its bottom and top, µm.
    z_um: tuple[float, float]
    #: What the polygons are made of.
    material: str
    #: What fills the layer elsewhere.
    background: str
    #: The polygons drawn on it.
    polygons: list[DevicePolygon]


@dataclass(frozen=True)
class DevicePort:
    """A device's port: where it crosses the device's edge, the direction light leaves by, the
    guide's width there, and its layer."""

    #: Its name, e.g. ``"o1"``.
    name: str
    #: Its centre seen from above, µm.
    position_um: tuple[float, float]
    #: The direction light leaves by, degrees counterclockwise from +x.
    direction_deg: float
    #: The guide's width there, µm.
    width_um: float
    #: The layer the guide is drawn on.
    layer: str


@dataclass(frozen=True)
class DeviceGeometry:
    """A device's geometry: its stack's layers bottom to top with the polygons on each, and its
    ports."""

    #: The layers, bottom to top.
    layers: list[GeometryLayer]
    #: The material below them.
    substrate: str
    #: The material above them.
    cladding: str
    #: The ports, in the order of the device's S-matrices.
    ports: list[DevicePort]


def _path(path: str | os.PathLike[str] | None) -> str | None:
    return None if path is None else os.fspath(path)


def _values(values: Mapping[str, float] | None) -> list[tuple[str, float]]:
    return [] if values is None else [(str(k), float(v)) for k, v in values.items()]


def devices() -> list[Device]:
    """The devices of photonoxide's library: the strip and rib waveguides, the bend, the
    directional coupler, the all-pass and add-drop rings, the 1 × 2 and 2 × 2 MMIs, the
    Y-branch, and the MZIs of directional couplers and of Y-branches, each with its parameters
    (name, unit, default, range), its models (cheapest first, each with its papers, stated error
    and validity) and its ports."""
    return [
        Device(
            kind=d["kind"],
            about=d["about"],
            parameters=[DeviceParameter(**p) for p in d["parameters"]],
            fidelities=[
                DeviceModel(
                    **{
                        **m,
                        "error": None if m["error"] is None else StatedError(**m["error"]),
                        "validity": [ValidityRange(**r) for r in m["validity"]],
                    }
                )
                for m in d["fidelities"]
            ],
            ports=d["ports"],
        )
        for d in _native.devices()
    ]


def device_spectrum(
    kind: str,
    *,
    values: Mapping[str, float] | None = None,
    fidelity: str | None = None,
    stack: str | None = None,
    compact: str | os.PathLike[str] | None = None,
    wavelength_um: Any,
) -> DeviceSpectrum:
    """The S-parameters at ``wavelength_um`` (one or an array) of a device of the library (its
    ``kind``, see :func:`devices`), with its parameters at ``values`` (in their units; those not
    given at their defaults), computed at ``fidelity`` (``"analytic"``, ``"2D"``, ``"3D"``, ...
    as the device has them; its cheapest when ``None``) on ``stack`` (``"soi_220"``, the
    default, ``"soi"`` or ``"nitride"``). ``fidelity="compact"`` takes a compact model fitted to the
    device's runs, its file ``compact`` (the ``compact.json`` that a ``"fit"`` job's run writes in
    its folder)::

        import numpy as np
        import photonoxide as phox

        w = np.linspace(1.5, 1.6, 201)
        for gap in (0.15, 0.2, 0.25):
            dc = phox.device_spectrum("directional coupler", values={"gap": gap, "length": 10.0},
                                    fidelity="analytic", wavelength_um=w)
            cross = np.abs(dc.s[:, 2, 0]) ** 2

    These are the numbers a component's page in the photonoxide studio shows for the same
    design, to the bit. A solver-backed fidelity solves once (a mode solve, or 2D or 3D FDFD:
    seconds to minutes, not stopped by Ctrl+C until the solve ends) and every wavelength comes
    from that model. A 2D result is an estimate, not the device's performance.
    """
    w, _ = _wavelengths(wavelength_um)
    s = _native.device_spectrum(kind, _values(values), fidelity, stack, _path(compact), w)
    validity = s["validity_um"]
    return DeviceSpectrum(**{**s, "validity_um": None if validity is None else tuple(validity)})


def device_derived(
    kind: str,
    *,
    values: Mapping[str, float] | None = None,
    fidelity: str | None = None,
    stack: str | None = None,
    compact: str | os.PathLike[str] | None = None,
) -> list[DeviceFigure]:
    """A device's figures at ``fidelity`` (see :func:`device_spectrum` for the design): n_eff
    and n_g, the cross-over length, the free spectral range, Q, a bend's loss, as its model
    gives them, each with its fidelity and grid."""
    return [
        DeviceFigure(**f)
        for f in _native.device_derived(kind, _values(values), fidelity, stack, _path(compact))
    ]


def device_geometry(
    kind: str,
    *,
    values: Mapping[str, float] | None = None,
    stack: str | None = None,
) -> DeviceGeometry:
    """A device's geometry at ``values`` (see :func:`device_spectrum`): its stack's layers,
    bottom to top, each with the polygons drawn on it (every curve within 1 nm, every vertex on
    a 1 nm grid, as GDSII's usual database unit), how each was etched, and the device's ports."""
    g = _native.device_geometry(kind, _values(values), stack)
    return DeviceGeometry(
        layers=[
            GeometryLayer(
                name=layer["name"],
                z_um=tuple(layer["z_um"]),
                material=layer["material"],
                background=layer["background"],
                polygons=[DevicePolygon(**p) for p in layer["polygons"]],
            )
            for layer in g["layers"]
        ],
        substrate=g["substrate"],
        cladding=g["cladding"],
        ports=[DevicePort(**{**p, "position_um": tuple(p["position_um"])}) for p in g["ports"]],
    )


def device_job(
    kind: str,
    *,
    values: Mapping[str, float] | None = None,
    fidelity: str | None = None,
    stack: str | None = None,
    compact: str | os.PathLike[str] | None = None,
    name: str | None = None,
    format: str | None = None,
) -> Any:
    """The job that computes a device's design (see :func:`device_spectrum`), as a component's
    Simulate tab in the studio writes it: a ``[device]`` table holding the design and a
    ``"spectrum"`` task over the device's own band. Named ``name`` (the kind's, by default).

    Without a ``format``, the job as data, a dict, which :func:`run_job` and :func:`check_job`
    take; with ``format`` (``"toml"``, ``"json"`` or ``"yaml"``), the job file's text, to save
    and run with ``photonoxide run`` or open in the studio. Its run records what
    :func:`device_spectrum` gives, to the bit."""
    job_name = name if name is not None else str(kind).lower().replace(" ", "-")
    if format is None:
        return json.loads(
            _native.device_job(kind, _values(values), fidelity, stack, _path(compact), job_name, "json")
        )
    return _native.device_job(kind, _values(values), fidelity, stack, _path(compact), job_name, format)
