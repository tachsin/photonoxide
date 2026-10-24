"""The native module of the photonoxide package: photonoxide's façade, one function for one.
Not for users: the package's functions (``photonoxide.refractive_index`` and the rest) wrap
these, with their result classes."""

from os import PathLike
from typing import Any

import numpy as np
import numpy.typing as npt

#: photonoxide's version: the crate the package was built with.
core_version: str

def materials() -> list[dict[str, Any]]: ...
def refractive_index(
    material: str,
    wavelength_um: npt.NDArray[np.float64],
    model: str | None = None,
    axis: str | None = None,
    temperature_k: float | None = None,
    composition: float | None = None,
) -> npt.NDArray[np.complex128]: ...
def group_index(
    material: str,
    wavelength_um: npt.NDArray[np.float64],
    model: str | None = None,
    axis: str | None = None,
    temperature_k: float | None = None,
    composition: float | None = None,
) -> npt.NDArray[np.float64]: ...
def slab_modes(
    below: float,
    core: float,
    above: float,
    thickness_um: float,
    polarization: str,
    wavelength_um: float,
    x_um: npt.NDArray[np.float64],
) -> list[dict[str, Any]]: ...
def vector_modes(
    x_um: npt.NDArray[np.float64],
    y_um: npt.NDArray[np.float64],
    permittivity: npt.NDArray[np.complex128],
    wavelength_um: float,
    count: int,
    near_index: float | None,
    boundaries: list[str],
    pml_um: list[float],
    pml_strength: float,
    timeout_s: float | None,
) -> list[dict[str, Any]]: ...
def check_job(path: str | PathLike[str] | None, text: str | None, format: str) -> None: ...
def run_job(
    path: str | PathLike[str] | None,
    text: str | None,
    format: str,
    runs_dir: str | PathLike[str],
    timeout_s: float | None,
) -> dict[str, Any]: ...
def fdfd_s_parameters(
    path: str | PathLike[str] | None,
    text: str | None,
    format: str,
    wavelength_um: npt.NDArray[np.float64] | None,
) -> dict[str, Any]: ...
def circuit_spectrum(netlist: str, wavelength_um: npt.NDArray[np.float64]) -> dict[str, Any]: ...
def read_touchstone(path: str | PathLike[str], convention: str) -> dict[str, Any]: ...
def write_touchstone(
    path: str | PathLike[str],
    ports: list[str],
    wavelength_um: npt.NDArray[np.float64],
    s: npt.NDArray[np.complex128],
    convention: str,
    significant_digits: int | None,
) -> None: ...
def devices() -> list[dict[str, Any]]: ...
def stacks() -> list[dict[str, Any]]: ...
def device_spectrum(
    kind: str,
    values: list[tuple[str, float]],
    fidelity: str | None,
    stack: str | None,
    compact: str | None,
    wavelength_um: npt.NDArray[np.float64],
    timeout_s: float | None,
) -> dict[str, Any]: ...
def device_derived(
    kind: str,
    values: list[tuple[str, float]],
    fidelity: str | None,
    stack: str | None,
    compact: str | None,
) -> list[dict[str, Any]]: ...
def device_geometry(
    kind: str,
    values: list[tuple[str, float]],
    stack: str | None,
) -> dict[str, Any]: ...
def device_job(
    kind: str,
    values: list[tuple[str, float]],
    fidelity: str | None,
    stack: str | None,
    compact: str | None,
    name: str,
    format: str,
) -> str: ...
def run_tables(run_dir: str | PathLike[str]) -> dict[str, dict[str, Any]]: ...
def export_run(
    run_dir: str | PathLike[str],
    tables: list[str] | None,
    out_dir: str | PathLike[str] | None,
) -> list[str]: ...
def estimate_fields(
    run_dir: str | PathLike[str], precision: str, level: int
) -> list[dict[str, Any]]: ...
def export_fields(
    run_dir: str | PathLike[str],
    files: list[str] | None,
    out_dir: str | PathLike[str],
    precision: str,
    level: int,
) -> list[str]: ...
def _conformance(scratch: str | PathLike[str]) -> list[dict[str, Any]]: ...
