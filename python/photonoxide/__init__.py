"""Validated photonics from Rust, for Python: materials with provenance, mode solvers, jobs and
2D FDFD S-parameters, circuits and Touchstone files, and devices that know their geometry, from
`photonoxide <https://github.com/tachsin/photonoxide>`_.

220 nm of silicon in oxide at 1550 nm, each material with its published dispersion, and the
slab's modes solved exactly:

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

**Long calls** (:func:`vector_modes`, :func:`run_job`, :func:`device_spectrum`) take
``timeout_s`` and raise ``TimeoutError`` past it; Ctrl+C stops them too. Every call releases the GIL while Rust works.

**The same numbers as Rust.** Each function calls photonoxide's façade, which calls the library
as a Rust program would; the package's tests check its results against the library's to the bit.
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
    "run_tables",
    "export_run",
    "FieldFile",
    "estimate_fields",
    "export_fields",
    "Device",
    "Stack",
    "StackLayer",
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
    "stacks",
    "device_spectrum",
    "device_derived",
    "device_geometry",
    "device_job",
    "Fabrication",
    "device_fabrication",
    "device_fabrication_job",
    "export_fabrication",
]

core_version: str = _native.core_version
"""The version of the Rust crate the package was built with, e.g. ``"0.6.0"``: the same as
:data:`__version__` for a release."""

try:
    _installed = metadata.version("photonoxide")
except metadata.PackageNotFoundError:  # built in place, not installed
    _installed = core_version
__version__: str = _installed
"""The package's version, e.g. ``"0.6.0"``; a fix to the package alone is a post-release,
``"0.6.0.post1"``."""

Job = Union[str, "os.PathLike[str]", Mapping[str, Any]]
"""A job: the path of a job file (YAML, TOML or JSON, by its extension), or the job as data, a
dict with a job file's fields (``name``, ``timeout_minutes``, ``task``, ...)."""


@dataclass(frozen=True)
class IndexModel:
    """An index model of a material, from one paper."""

    id: str
    """Its id, e.g. ``"li-1980"``."""
    name: str
    """Its name, e.g. ``"Li 1980"``."""
    default: bool
    """Whether it is the material's default model."""
    axes: list[str]
    """The indices it gives: ``["isotropic"]``, or ``["ordinary", "extraordinary"]``."""
    range_um: tuple[float, float]
    """The wavelengths it is valid for at its default conditions, µm."""
    temperature_k: float | None
    """Its default temperature, K, when it has one."""
    composition: float | None
    """Its default composition (e.g. x in AlₓGa₁₋ₓAs), when it has one."""
    accuracy: str
    """The accuracy its paper states."""
    sources: list[str]
    """Where it comes from: each paper's citation, DOI and place."""


@dataclass(frozen=True)
class Material:
    """A material of photonoxide's catalogue, with its index models."""

    id: str
    """Its id, e.g. ``"si"`` or ``"linbo3"``: what :func:`refractive_index` takes."""
    name: str
    """Its name, e.g. ``"Silicon"``."""
    formula: str
    """Its formula, e.g. ``"Si"``."""
    models: list[IndexModel]
    """Its index models, the default first."""


@dataclass(frozen=True)
class SlabMode:
    """A guided mode of a three-layer slab."""

    polarization: str
    """``"te"`` or ``"tm"``."""
    order: int
    """Its order: 0 for the fundamental mode of its polarization."""
    effective_index: float
    """The effective index n_eff = β/k."""
    x_um: np.ndarray
    """Where its field is given across the slab, µm, 0 at the core's bottom."""
    field: np.ndarray
    """Its field there: E_y for TE, H_y for TM, unnormalized, 1 (TE) or h/q̄ (TM) at x = 0."""


@dataclass(frozen=True)
class VectorMode:
    """A full-vector mode of a cross-section, with its six field components at the centres of
    the grid's cells, each ``(ny, nx)``: ``ex[j, i]`` at ``(x_um[i], y_um[j])``."""

    effective_index: complex
    """The effective index n_eff = β/k; its imaginary part, ≥ 0 in a passive structure, is the
    loss along z."""
    te_fraction: float
    """The share of the transverse magnetic field in H_y: near 1 for a TE-like mode (E mostly
    along x), near 0 for a TM-like one."""
    x_um: np.ndarray
    """The cells' centres along x, µm."""
    y_um: np.ndarray
    """The cells' centres along y, µm."""
    ex: np.ndarray
    """E_x, in units of Z₀ times H's."""
    ey: np.ndarray
    """E_y, in the same units as ``ex`` (Z₀ times H's)."""
    ez: np.ndarray
    """E_z, in the same units as ``ex`` (Z₀ times H's)."""
    hx: np.ndarray
    """H_x, the largest transverse component of H at the grid's nodes 1."""
    hy: np.ndarray
    """H_y, in the same units as ``hx``."""
    hz: np.ndarray
    """H_z, in the same units as ``hx``."""


@dataclass(frozen=True)
class JobRun:
    """A job's run: its folder and its record."""

    dir: Path
    """The run's folder, ``<runs_dir>/<YYYYMMDD>-<HHMMSS>-<name>``, which the photonoxide program
    replays (``photonoxide view <folder>``)."""
    events: list[dict[str, Any]]
    """Its events in order, each a dict with its ``"type"``, as the folder's ``events.jsonl``
    has them."""
    stopped: str | None
    """Why it ended early, if it did: ``"timeout"`` or ``"requested"``."""


@dataclass(frozen=True)
class SParameters:
    """An ``"fdfd"`` job's S-parameters by 2D FDFD: an estimate, not a device's performance
    in 3D."""

    ports: list[str]
    """The ports' names, in the job's order, e.g. ``"1 (left, x = -3.5 um)"``."""
    wavelength_um: np.ndarray
    """The wavelengths, µm."""
    s: np.ndarray
    """The power-normalized S-matrices, ``(nλ, n, n)``: ``s[k, q, p]`` from port p into port q,
    the phases referred to the ports' columns."""
    effective_index: np.ndarray
    """Each port mode's effective index at each wavelength, ``(nλ, n)``."""
    cell_um: tuple[float, float]
    """The grid's cell, dx and dy, µm."""
    polarization: str
    """The slab mode's polarization the plane's indices come from: ``"te"`` (H along z in the
    plane) or ``"tm"`` (E along z)."""


@dataclass(frozen=True)
class Spectrum:
    """S-matrices at several wavelengths between named ports: a circuit's spectrum, or a
    Touchstone file's."""

    ports: list[str]
    """The ports' names, in the matrices' order."""
    wavelength_um: np.ndarray
    """The vacuum wavelengths, µm."""
    s: np.ndarray
    """The S-matrices, ``(nλ, n, n)``: ``s[k, q, p]`` from port p into port q, in photonoxide's
    e^(−iωt) convention."""


def _wavelengths(wavelength_um: Any) -> tuple[np.ndarray, tuple[int, ...]]:
    w = np.asarray(wavelength_um, dtype=np.float64)
    return np.ascontiguousarray(w.ravel()), w.shape


def _shaped(values: np.ndarray, shape: tuple[int, ...]) -> Any:
    if shape == ():
        return values[0].item()
    return values.reshape(shape)


def materials() -> list[Material]:
    """Every material of photonoxide's catalogue, with its index models, their ranges and their
    sources by DOI. A material's ``id`` is what :func:`refractive_index` takes:

    >>> si = next(m for m in phox.materials() if m.id == "si")
    >>> si.name, si.models[0].id, si.models[0].range_um
    ('Silicon', 'li-1980', (1.2, 14.0))
    """
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
    :class:`~photonoxide.errors.OutsideValidity`: photonoxide doesn't extrapolate:

    >>> n = phox.refractive_index("si", wavelength_um=1.55)  # Li 1980
    >>> round(n.real, 4), n.imag
    (3.4757, 0.0)
    >>> phox.refractive_index("si", wavelength_um=[1.5, 1.6]).round(4)
    array([3.4799+0.j, 3.4719+0.j])
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
    µm, as :func:`refractive_index` takes them: a float for one, an array for several:

    >>> round(phox.group_index("si", wavelength_um=1.55), 3)
    3.599
    """
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
    ``x_um`` (0 at the core's bottom; none when not given). Empty when the slab guides none.
    A 220 nm silicon slab in oxide guides one TE mode at 1550 nm:

    >>> modes = phox.slab_modes(below=1.444, core=3.473, above=1.444, thickness_um=0.22,
    ...                         polarization="te", wavelength_um=1.55)
    >>> [(m.order, round(m.effective_index, 4)) for m in modes]
    [(0, 2.8448)]
    """
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

    A 500 × 220 nm silicon strip in oxide, on a 50 nm grid, 2 µm across:

    >>> x = np.linspace(-1.0, 1.0, 41)
    >>> y = np.linspace(-1.0, 1.0, 41)
    >>> inside = (np.abs((x[:-1] + x[1:]) / 2)[None, :] < 0.25) & (
    ...     np.abs((y[:-1] + y[1:]) / 2)[:, None] < 0.11)
    >>> eps = np.where(inside, 3.473**2, 1.444**2)  # (ny, nx)
    >>> [mode] = phox.vector_modes(x_um=x, y_um=y, permittivity=eps, wavelength_um=1.55)
    >>> round(mode.effective_index.real, 4), round(mode.te_fraction, 2), mode.ex.shape
    (2.3639, 0.97, (40, 40))
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
    what the run would raise for its settings, and returns ``None`` when the job is fine:

    >>> phox.check_job({"name": "x", "task": {"kind": "modes", "nonsense": 1}})
    Traceback (most recent call last):
        ...
    photonoxide.errors.ParseError: ...
    """
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

    The fundamental mode of a 500 nm silicon strip (220 nm thick, in oxide), solved on a 50 nm
    grid, into a ``runs`` folder:

    >>> job = {"name": "strip", "task": {
    ...     "kind": "modes", "stack": "soi_220", "wavelength_um": 1.55, "layer": "Si",
    ...     "propagation": "x", "y_um": [-1.0, 1.0], "step_nm": 50.0, "modes": 1,
    ...     "rect": [{"layer": "Si", "center_um": [0.0, 0.0], "size_um": [10.0, 0.5]}]}}
    >>> run = phox.run_job(job, runs_dir="runs")
    >>> run.stopped, run.events[0]["type"], run.events[-1]["type"]
    (None, 'started', 'finished')
    >>> round(float(phox.run_tables(run.dir)["modes"]["n_eff_re"][0]), 4)
    2.4447
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
    any number of them. A straight 500 nm guide passes its mode whole:

    >>> job = {"name": "straight", "task": {
    ...     "kind": "fdfd", "stack": "soi_220", "wavelength_um": 1.55, "layer": "Si",
    ...     "polarization": "te", "x_um": [-3.0, 3.0], "y_um": [-2.0, 2.0], "step_nm": 50.0,
    ...     "rect": [{"layer": "Si", "center_um": [0.0, 0.0], "size_um": [6.0, 0.5]}],
    ...     "port": [{"x_um": -1.5, "side": "left"}, {"x_um": 1.5, "side": "right"}]}}
    >>> result = phox.fdfd_s_parameters(job, wavelength_um=[1.5, 1.55])
    >>> result.s.shape, bool(np.all(np.abs(result.s[:, 1, 0]) > 0.99))
    ((2, 2, 2), True)
    """
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
    ``ports``, each name to the instance's port it exposes, in the matrices' order:

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
    expressions such as ``"450 nm"`` or ``"w/2.5"``):

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
    ``o2``, ... A file written by :func:`write_touchstone` reads back to the same values:

    >>> s = np.zeros((2, 2, 2), dtype=complex)
    >>> s[:, 1, 0] = s[:, 0, 1] = 1.0  # a lossless through
    >>> through = phox.Spectrum(ports=["a", "b"], wavelength_um=np.array([1.5, 1.55]), s=s)
    >>> phox.write_touchstone("through.s2p", through, convention="physics")
    >>> back = phox.read_touchstone("through.s2p", convention="physics")
    >>> back.ports, back.wavelength_um.round(2), bool(np.array_equal(back.s, s))
    (['o1', 'o2'], array([1.55, 1.5 ]), True)
    """
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


# --- a run's results ------------------------------------------------------------------------


def run_tables(run_dir: str | os.PathLike[str]) -> dict[str, dict[str, Any]]:
    """The tables of the run in ``run_dir``, from its record: each table's name (``"modes"``,
    ``"s_parameters"``, ``"spectra"``, ``"study"``, ``"history"``, ``"static"``, ...) to its
    columns in order, each a float array (NaN where the table has no number) or a list of text.
    Every number is the one the run recorded, to the bit: the same as its CSV file's, which
    :func:`export_run` writes. ``pandas.DataFrame(tables["modes"])`` makes one a data frame.
    For the run of :func:`run_job`'s example:

    >>> list(phox.run_tables(run.dir)["modes"])
    ['label', 'wavelength_um', 'n_eff_re', 'n_eff_im', 'loss_db_per_cm', 'te_fraction']
    """
    return _native.run_tables(os.fspath(run_dir))


def export_run(
    run_dir: str | os.PathLike[str],
    *,
    tables: list[str] | None = None,
    out_dir: str | os.PathLike[str] | None = None,
) -> list[Path]:
    """Writes the tables of the run in ``run_dir`` as CSV files into ``out_dir`` (the run's own
    folder when ``None``): those named in ``tables``, every one it has when ``None``. Returns the
    files written. A run writes its tables itself when it ends; its fields (``.npz``) are
    written then too, when its job's ``[task.export]`` asks for them. For the run of
    :func:`run_job`'s example:

    >>> [file.name for file in phox.export_run(run.dir, tables=["modes"], out_dir="csv")]
    ['modes.csv']
    """
    paths = _native.export_run(
        os.fspath(run_dir),
        None if tables is None else [str(t) for t in tables],
        None if out_dir is None else os.fspath(out_dir),
    )
    return [Path(p) for p in paths]


@dataclass(frozen=True)
class FieldFile:
    """A field file of a run (an ``.npz``, or a container's ``.h5``), and its size written
    again at a precision and a deflate level, estimated before writing it."""

    file: str
    """Its name in the run's folder, e.g. ``"mode-1-of-2.npz"``."""
    bytes: int
    """Its size as the run wrote it, bytes."""
    estimate: int
    """Its size written again as asked, bytes (``bytes`` for a file copied as it is)."""
    exact: bool
    """Whether ``estimate`` is the size to the byte, not an estimate from slices of it."""
    precision: str
    """The precision its fields have written again, ``"f64"`` or ``"f32"``; ``"as written"``
    for a file copied as it is."""
    rewritable: bool
    """Whether it can be written again: an ``.h5``, or an ``.npz`` compressed with Zstandard,
    is copied as it is."""


def estimate_fields(
    run_dir: str | os.PathLike[str], *, precision: str = "f64", level: int = 6
) -> list[FieldFile]:
    """The field files of the run in ``run_dir``, each with its size and its size written again
    at ``precision`` (``"f64"`` or ``"f32"``) and deflate ``level`` (0 stores, 1 to 9 deflate),
    estimated before writing: what :func:`export_fields` writes. Stored, the estimate is the
    size to the byte; deflated, it is for members up to 1 MiB, and from slices of larger ones."""
    return [
        FieldFile(**f) for f in _native.estimate_fields(os.fspath(run_dir), precision, int(level))
    ]


def export_fields(
    run_dir: str | os.PathLike[str],
    out_dir: str | os.PathLike[str],
    *,
    files: list[str] | None = None,
    precision: str = "f64",
    level: int = 6,
) -> list[Path]:
    """Writes the field files of the run in ``run_dir`` into ``out_dir``, another folder: those
    named in ``files`` (every one when ``None``), each written again at ``precision`` (the
    fields downcast to ``"f32"`` if asked, the grid kept ``f64``) and deflate ``level``, its
    ``meta.json`` saying so. Returns the files written."""
    paths = _native.export_fields(
        os.fspath(run_dir),
        None if files is None else [str(f) for f in files],
        os.fspath(out_dir),
        precision,
        int(level),
    )
    return [Path(p) for p in paths]


# --- devices --------------------------------------------------------------------------------


@dataclass(frozen=True)
class DeviceParameter:
    """A parameter of a device: its name, unit, default and range."""

    name: str
    """Its name, e.g. ``"gap"``."""
    unit: str
    """Its unit, e.g. ``"µm"`` or ``"°"``, or ``""`` for a pure number."""
    default: float
    """Its default."""
    min: float
    """The smallest value it takes."""
    max: float
    """The largest."""


@dataclass(frozen=True)
class StatedError:
    """A model's stated error: the largest difference in one quantity against another
    fidelity, measured by photonoxide's validation report."""

    quantity: str
    """The quantity, e.g. ``"n_eff"``."""
    bound: float
    """The largest difference, in ``unit``."""
    unit: str
    """Its unit: ``""`` for a pure number, ``"%"`` for a relative one."""
    against: str
    """The fidelity it is measured against."""


@dataclass(frozen=True)
class ValidityRange:
    """A range a model holds over, for one parameter or the wavelength (µm)."""

    name: str
    """The parameter's name, or ``"wavelength"``."""
    min: float
    """The smallest value."""
    max: float
    """The largest."""


@dataclass(frozen=True)
class DeviceModel:
    """One of a device's models: its fidelity, what computes it, its write-up and papers, its
    stated error against the next fidelity up, and where it holds."""

    fidelity: str
    """``"analytic"``, ``"compact"``, ``"2D"``, ``"3D"`` or ``"measured"``."""
    name: str
    """What computes it, in a sentence."""
    write_up: str
    """Its write-up in photonoxide's docs/methods, e.g. ``"devices"``."""
    papers: list[str]
    """The papers it implements, each with its DOI."""
    error: StatedError | None
    """Its error against the next fidelity up over its validity; ``None`` for the highest."""
    validity: list[ValidityRange]
    """Where it holds: a range for each parameter it limits, and ``"wavelength"``."""


@dataclass(frozen=True)
class Device:
    """A device of photonoxide's library: a component that knows its geometry."""

    kind: str
    """Its kind, what the device functions take, e.g. ``"directional coupler"``."""
    about: str
    """What it is, in a sentence or two."""
    parameters: list[DeviceParameter]
    """Its parameters, in order."""
    fidelities: list[DeviceModel]
    """Its models, cheapest first: the first is a design's when it names none."""
    ports: list[str]
    """Its ports' names, in the order of its S-matrices."""


@dataclass(frozen=True)
class StackLayer:
    """A layer of a stack of the catalogue."""

    name: str
    """Its name, e.g. ``"BOX"``."""
    thickness_um: float
    """Its thickness, µm; the core's is the stack's own until a device sets it to its
    ``thickness``."""
    material: str
    """What the shapes drawn on it are made of: the catalogue's id (see :func:`materials`),
    e.g. ``"si"``."""
    background: str
    """What fills it elsewhere, the catalogue's id."""


@dataclass(frozen=True)
class Stack:
    """A stack of the catalogue, by the name the device functions take as ``stack``: its layers
    of the materials catalogue's, bottom to top, between a substrate and a cladding."""

    name: str
    """Its name, e.g. ``"soi_220"``."""
    about: str
    """What it is, in a sentence or two."""
    substrate: str
    """The material below the layers, the catalogue's id, e.g. ``"si"``."""
    layers: list[StackLayer]
    """The layers, bottom to top."""
    cladding: str
    """The material above them, the catalogue's id."""
    core: str
    """The layer a device draws its guides on, e.g. ``"Si"``: its thickness is the device's
    ``thickness`` parameter."""


@dataclass(frozen=True)
class DeviceSpectrum:
    """A device's S-parameters over wavelength, with the model they come from. A 2D result is
    an estimate, not the device's performance."""

    ports: list[str]
    """The device's ports' names, in the matrices' order."""
    wavelength_um: np.ndarray
    """The vacuum wavelengths, µm."""
    s: np.ndarray
    """The S-matrices, ``(nλ, n, n)``: ``s[k, q, p]`` from port p into port q, in photonoxide's
    e^(−iωt) convention."""
    fidelity: str
    """The fidelity of the model: ``"analytic"``, ``"2D"``, ``"3D"``, ..."""
    grid: str
    """The grid its figures name, or ``"exact: no grid"`` for a closed form; a fit names the grid
    of the solves it was fitted to."""
    source: str
    """What the model reproduces: a paper with its DOI and equation, or a solver with its grid."""
    validity_um: tuple[float, float] | None
    """The wavelengths it holds for, µm, if limited."""


@dataclass(frozen=True)
class DeviceFigure:
    """One of a device's figures, with the fidelity and grid it comes from."""

    name: str
    """What it is, e.g. ``"effective index"``."""
    symbol: str
    """Its symbol, unique among a device's figures, e.g. ``"n_eff"``."""
    value: float
    """Its value, in ``unit``."""
    unit: str
    """Its unit, e.g. ``"µm"``, or ``""`` for a pure number."""
    fidelity: str
    """The fidelity of the model that gives it."""
    grid: str
    """The grid it was computed on, or ``"exact: no grid"`` for a closed form."""


@dataclass(frozen=True)
class DevicePolygon:
    """A polygon with holes, seen from above, and how its edges were etched."""

    outer: np.ndarray
    """Its outer boundary, ``(n, 2)``: (x, y) in µm, counterclockwise, the first vertex not
    repeated."""
    holes: list[np.ndarray]
    """Its holes, each ``(n, 2)``, clockwise."""
    etch_depth_um: float | None
    """How deep the etch around it went from the layer's top, µm: ``None`` through the layer (a
    strip), less than the layer's thickness for a rib."""
    sidewall_deg: float
    """Its walls' angle from the layer's plane, degrees: 90 vertical."""


@dataclass(frozen=True)
class GeometryLayer:
    """A layer of a device's stack, and the polygons drawn on it."""

    name: str
    """Its name, e.g. ``"Si"``."""
    z_um: tuple[float, float]
    """Its bottom and top, µm."""
    material: str
    """What the polygons are made of."""
    background: str
    """What fills the layer elsewhere."""
    polygons: list[DevicePolygon]
    """The polygons drawn on it."""


@dataclass(frozen=True)
class DevicePort:
    """A device's port: where it crosses the device's edge, the direction light leaves by, the
    guide's width there, and its layer."""

    name: str
    """Its name, e.g. ``"o1"``."""
    position_um: tuple[float, float]
    """Its centre seen from above, µm."""
    direction_deg: float
    """The direction light leaves by, degrees counterclockwise from +x."""
    width_um: float
    """The guide's width there, µm."""
    layer: str
    """The layer the guide is drawn on."""


@dataclass(frozen=True)
class DeviceGeometry:
    """A device's geometry: its stack's layers bottom to top with the polygons on each, and its
    ports."""

    layers: list[GeometryLayer]
    """The layers, bottom to top."""
    substrate: str
    """The material below them."""
    cladding: str
    """The material above them."""
    ports: list[DevicePort]
    """The ports, in the order of the device's S-matrices."""


def _path(path: str | os.PathLike[str] | None) -> str | None:
    return None if path is None else os.fspath(path)


def _values(values: Mapping[str, float] | None) -> list[tuple[str, float]]:
    return [] if values is None else [(str(k), float(v)) for k, v in values.items()]


def devices() -> list[Device]:
    """The devices of photonoxide's library: the waveguides (strip and rib), the bend, the
    directional coupler, the rings, the MMIs, the Y-branch, the MZIs, the phase shifters
    (thermo-optic and phase-change) and the thin-film lithium niobate modulator, each with its
    parameters (name, unit, default, range), its models (cheapest first, each with its papers,
    stated error and validity) and its ports. A device's ``kind`` is what the other device
    functions take:

    >>> ring = next(d for d in phox.devices() if d.kind == "all-pass ring")
    >>> [(p.name, p.unit) for p in ring.parameters][:3], ring.ports
    ([('width', 'µm'), ('thickness', 'µm'), ('radius', 'µm')], ['in', 'through'])
    >>> [model.fidelity for model in ring.fidelities]
    ['analytic', '3D']
    """
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


def stacks() -> list[Stack]:
    """The stacks of photonoxide's catalogue, each by the name the device functions take as
    ``stack``: ``"soi_220"`` (220 nm of silicon on 2 µm of oxide, the default), ``"soi"`` and
    ``"nitride"``, each with its layers of the materials catalogue's (their ids, which
    :func:`refractive_index` takes) and the core layer a device draws its guides on:

        import photonoxide as phox

        for stack in phox.stacks():
            mmi = phox.device_spectrum("MMI 1x2", stack=stack.name, fidelity="2D",
                                       wavelength_um=1.55)
    """
    return [
        Stack(**{**s, "layers": [StackLayer(**layer) for layer in s["layers"]]})
        for s in _native.stacks()
    ]


def device_spectrum(
    kind: str,
    *,
    values: Mapping[str, float] | None = None,
    fidelity: str | None = None,
    stack: str | None = None,
    compact: str | os.PathLike[str] | None = None,
    wavelength_um: Any,
    timeout_s: float | None = None,
) -> DeviceSpectrum:
    """The S-parameters at ``wavelength_um`` (one or an array) of a device of the library (its
    ``kind``, see :func:`devices`), with its parameters at ``values`` (in their units; those not
    given at their defaults), computed at ``fidelity`` (``"analytic"``, ``"2D"``, ``"3D"``, ...
    as the device has them; its cheapest when ``None``) on ``stack``, one of :func:`stacks` by
    name (``"soi_220"``, the default, ``"soi"`` or ``"nitride"``). ``fidelity="compact"`` takes a compact model fitted to the
    device's runs, its file ``compact`` (the ``compact.json`` that a ``"fit"`` job's run writes in
    its folder):

        import numpy as np
        import photonoxide as phox

        w = np.linspace(1.5, 1.6, 201)
        for gap in (0.15, 0.2, 0.25):
            dc = phox.device_spectrum("directional coupler", values={"gap": gap, "length": 10.0},
                                    fidelity="analytic", wavelength_um=w)
            cross = np.abs(dc.s[:, 2, 0]) ** 2

    These are the numbers a component's page in the photonoxide studio shows for the same
    design, to the bit. A solver-backed fidelity solves once (a mode solve, or 2D or 3D FDFD:
    seconds to minutes) and every wavelength comes from that model. A 2D result is an estimate,
    not the device's performance.

    The call stops after ``timeout_s`` seconds (``None``: never) with ``TimeoutError``, and on
    Ctrl+C with ``KeyboardInterrupt``, at once: the solve already started is let go, its result
    discarded, and runs on to its end on a thread of its own.
    """
    w, _ = _wavelengths(wavelength_um)
    s = _native.device_spectrum(
        kind,
        _values(values),
        fidelity,
        stack,
        _path(compact),
        w,
        None if timeout_s is None else float(timeout_s),
    )
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
    gives them, each with its fidelity and grid:

    >>> figures = phox.device_derived("all-pass ring", values={"radius": 5.0}, fidelity="analytic")
    >>> {f.symbol: (round(f.value, 3), f.unit) for f in figures}["FSR"]
    (18.326, 'nm')
    """
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
    a 1 nm grid, as GDSII's usual database unit), how each was etched, and the device's ports:

    >>> geometry = phox.device_geometry("directional coupler")
    >>> [layer.name for layer in geometry.layers], [port.name for port in geometry.ports]
    (['BOX', 'Si'], ['o1', 'o2', 'o3', 'o4'])
    """
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
    Simulate tab in the studio writes it: a ``device`` table holding the design and a
    ``"spectrum"`` task over the device's own band. Named ``name`` (the kind's, by default).

    Without a ``format``, the job as data, a dict, which :func:`run_job` and :func:`check_job`
    take; with ``format`` (``"yaml"``, the format we recommend, ``"toml"`` or ``"json"``), the
    job file's text, to save and run with ``photonoxide run`` or open in the studio. Its run records what
    :func:`device_spectrum` gives, to the bit:

    >>> job = phox.device_job("all-pass ring", values={"radius": 5.0})
    >>> job["name"], job["task"]["kind"]
    ('all-pass-ring', 'spectrum')
    >>> phox.check_job(job)
    """
    job_name = name if name is not None else str(kind).lower().replace(" ", "-")
    if format is None:
        return json.loads(
            _native.device_job(kind, _values(values), fidelity, stack, _path(compact), job_name, "json")
        )
    return _native.device_job(kind, _values(values), fidelity, stack, _path(compact), job_name, format)


# --- a design's process flow ----------------------------------------------------------------


@dataclass(frozen=True)
class Fabrication:
    """A design's process flow, as a ``"fabrication"`` job's run records it, and its export.

    ``flow`` and ``assessment`` are plain Python data (dicts, lists, strings, floats in their
    units: ``thickness_um``, ``depth_um``, ``sidewall_deg``), read from photonoxide's own JSON,
    so every number is the one the run records, to the bit. The texts are the files the run
    writes into its folder: ``flow.json``, ``flow.md`` and ``flow-01.svg``, ..."""

    flow: dict[str, Any]
    """The flow, as the run's ``flow`` event: ``library`` (the process library's version),
    ``steps`` in order (each with its ``process`` id and ``name``, its ``step`` type, ``plain``
    (what the process does), ``operation`` and ``does`` (what it does to the wafer), the
    ``layer``, ``film``, ``thickness_um``, ``depth_um`` and ``timed``, ``sidewall_deg`` and
    ``tone`` it sets, its ``reason``, its ``alternatives`` (``process``, ``name``, ``why_not``),
    the ``capabilities`` it relies on (each a ``quantity``, a ``value`` with its ``low``,
    ``high``, ``unit`` and ``source``, or ``{"kind": "not_specified"}``, and its ``conditions``),
    ``facts`` and ``uncertainties``), ``warnings``, ``not_specified``, ``round_trip`` (whether
    the executed flow gives the design back, and on how many of the points compared it
    differs), ``references`` (each with its ``citation``, ``title``, ``doi`` or ``isbn``) and
    ``glossary``."""
    assessment: dict[str, Any]
    """What the rules find, as the run's ``flow_assessment`` event: ``checks`` (each with its
    ``rule``, ``layer``, ``step`` from 0, ``verdict`` and ``text``), ``ranking`` (each step's
    processes, best first), ``accuracy`` (see :attr:`accuracy`), ``combination`` (how the
    contributions are combined, with its sources) and ``optics`` (each projection lithography's
    wavelength, numerical apertures and the lines it prints)."""
    json: str
    """The export's JSON, as the run's ``flow.json``: the ``device`` and its ``parameters``, the
    ``cut``, the ``flow``, the ``assessment`` and the ``stacks`` (the wafer's layers after each
    step, from the top down), its keys in a fixed order."""
    markdown: str
    """The process traveller, as the run's ``flow.md``: the design and its layers from the top
    down, the substrate last; each step with what it does, why, the process, the cited
    capabilities with their sources, the alternatives, the rules' checks, the dimensions it makes
    and how accurately, and the wafer after it; the accuracy of every dimension, the warnings,
    what is not specified, the round trip and the references."""
    svgs: list[str]
    """Each step's cross-section, a standalone SVG, as the run's ``flow-01.svg``, ...: the wafer
    after the step, cut across y at the middle of the design in x, its layers listed from the
    top down beside it."""

    @property
    def steps(self) -> list[dict[str, Any]]:
        """The flow's steps, in order: ``flow["steps"]``."""
        return self.flow["steps"]

    @property
    def accuracy(self) -> list[dict[str, Any]]:
        """How accurately each fabricated dimension is made: ``assessment["accuracy"]``, each
        with ``what`` it is, its ``layer`` and ``kind``, its design value ``nominal`` in its
        ``unit`` (``"nm"`` or ``"degree"``), its ``contributions`` (each a ``bound`` ± in that
        unit, or ``None`` when not specified, with its ``source``), their root sum of squares
        ``combined`` and their sum ``worst``, whether it is ``complete``, and its ``text``."""
        return self.assessment["accuracy"]


def _ids(ids: Sequence[str]) -> list[str]:
    if isinstance(ids, str):
        raise TypeError("prefer and avoid are lists of process ids, not one string")
    return [str(i) for i in ids]


def device_fabrication(
    kind: str,
    *,
    values: Mapping[str, float] | None = None,
    stack: str | None = None,
    prefer: Sequence[str] = (),
    avoid: Sequence[str] = (),
) -> Fabrication:
    """The process flow of a device's design (see :func:`device_spectrum` for ``kind``,
    ``values`` and ``stack``): the steps that would make it from published processes, in order,
    each with why it was chosen, what else could do it, the capabilities it relies on with their
    sources, and how accurately each dimension is made, preferring the processes ``prefer``
    names by id where several qualify and never using those ``avoid`` names. The same as a
    ``"fabrication"`` job's run records for the same design, to the bit
    (:func:`device_fabrication_job`), with the files it writes:

    >>> strip = phox.device_fabrication("waveguide")
    >>> [step["process"] for step in strip.steps]
    ['soi-smart-cut', 'duv-193', 'icp-si-hbr', 'resist-strip', 'lpcvd']
    >>> strip.steps[2]["does"], strip.flow["round_trip"]["differing"]
    ('etch Si through, to the film below at 90°', 0)
    >>> [(d["what"], d["nominal"], d["unit"], d["combined"]) for d in strip.accuracy][1]
    ('the narrowest line of Si', 500.0, 'nm', 4.5)
    """
    f = _native.device_fabrication(kind, _values(values), stack, _ids(prefer), _ids(avoid))
    return Fabrication(
        flow=json.loads(f["flow"]),
        assessment=json.loads(f["assessment"]),
        json=f["json"],
        markdown=f["markdown"],
        svgs=list(f["svgs"]),
    )


def device_fabrication_job(
    kind: str,
    *,
    values: Mapping[str, float] | None = None,
    stack: str | None = None,
    prefer: Sequence[str] = (),
    avoid: Sequence[str] = (),
    name: str | None = None,
    format: str | None = None,
) -> Any:
    """The job that gives a design's process flow (see :func:`device_fabrication`): a
    ``device`` table holding the design and a ``"fabrication"`` task with ``prefer`` and
    ``avoid``, named ``name`` (the kind's and ``-flow``, by default). Without a ``format``, the
    job as data, a dict, which :func:`run_job` takes; with ``format`` (``"yaml"``, ``"toml"`` or
    ``"json"``), the job file's text. Its run records what :func:`device_fabrication` gives and
    writes ``flow.json``, ``flow.md`` and ``flow-01.svg``, ... into its folder:

    >>> job = phox.device_fabrication_job("waveguide", prefer=["ebl-hsq"])
    >>> job["name"], job["task"]
    ('waveguide-flow', {'kind': 'fabrication', 'prefer': ['ebl-hsq']})
    >>> run = phox.run_job(job, runs_dir="runs")
    >>> sorted(p.name for p in run.dir.glob("flow*"))[:3]
    ['flow-01.svg', 'flow-02.svg', 'flow-03.svg']
    """
    job_name = name if name is not None else str(kind).lower().replace(" ", "-") + "-flow"
    text = _native.device_fabrication_job(
        kind, _values(values), stack, job_name, "json" if format is None else format,
        _ids(prefer), _ids(avoid),
    )
    return json.loads(text) if format is None else text


def export_fabrication(
    kind: str,
    *,
    out_dir: str | os.PathLike[str],
    values: Mapping[str, float] | None = None,
    stack: str | None = None,
    prefer: Sequence[str] = (),
    avoid: Sequence[str] = (),
) -> list[Path]:
    """Writes a design's process flow (see :func:`device_fabrication`) into ``out_dir``, made if
    it isn't there, as a ``"fabrication"`` job's run writes it into its folder: ``flow.json``
    (the flow's data, which reads back to the same flow), ``flow.md`` (the process traveller) and
    each step's cross-section, ``flow-01.svg``, ...; returns the files written, in that order:

    >>> files = phox.export_fabrication("thermo-optic phase shifter", out_dir="heater")
    >>> [file.name for file in files][:3], len(files)
    (['flow.json', 'flow.md', 'flow-01.svg'], 14)
    """
    paths = _native.export_fabrication(
        kind, _values(values), stack, _ids(prefer), _ids(avoid), os.fspath(out_dir)
    )
    return [Path(p) for p in paths]
