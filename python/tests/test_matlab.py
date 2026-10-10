"""MATLAB's side of the package, as far as it can be tested without MATLAB.

The ``.m`` wrappers call ``photonoxide.matlab``, which does every conversion: these tests call
it as the wrappers do (flat lists of floats, as ``num2cell`` makes them, and ``array.array``)
and check that what comes back, rebuilt as the wrappers rebuild it (``reshape`` in column-major
order), is the Python package's result to the bit. That the wrappers and the module agree on
names is checked from the ``.m`` files' text. MATLAB itself isn't run: see docs/python.md.
"""

from __future__ import annotations

import json
import re
from array import array
from pathlib import Path
from typing import Any

import numpy as np
import pytest

import photonoxide as po
from photonoxide import matlab

WRAPPERS = Path(matlab.path()) / "+photonoxide"


def rebuilt(d: dict[str, Any]) -> np.ndarray:
    """An array as the wrappers' ``unflat.m`` rebuilds it: reshape(values, shape) in MATLAB's
    column-major order, the imaginary part added when there is one."""
    values = np.asarray(d["re"], dtype=float)
    if "im" in d:
        values = values + 1j * np.asarray(d["im"], dtype=float)
    shape = tuple(int(n) for n in d["shape"])
    return values.reshape(shape, order="F")


def matlab_list(a: Any) -> list[float]:
    """A MATLAB array as ``pylist.m`` passes it: its values in column-major order."""
    return [float(v) for v in np.asarray(a).ravel(order="F")]


def test_the_wrappers_are_shipped_and_call_what_the_module_has() -> None:
    files = sorted(p for p in WRAPPERS.glob("*.m"))
    assert files, "no .m wrappers in the package"
    called = set()
    for file in files:
        text = file.read_text(encoding="utf-8")
        names = re.findall(r"py\.photonoxide\.matlab\.(\w+)\(", text)
        assert len(names) == 1, f"{file.name} calls {names}"
        assert names[0] == file.stem, f"{file.name} calls {names[0]}"
        assert names[0] in matlab.__all__
        assert text.startswith(f"function"), file.name
        assert f"%{file.stem.upper()}  " in text, f"{file.name} has no help line"
        called.add(names[0])
    # every function of the module has its wrapper, but path, which MATLAB calls before addpath
    assert called == set(matlab.__all__) - {"path"}
    for helper in ("call", "pylist", "unflat", "strings", "jobtext"):
        assert (WRAPPERS / "private" / f"{helper}.m").is_file()


def test_indices_come_back_in_matlabs_order_to_the_bit() -> None:
    w = np.array([[1.31, 1.45], [1.55, 2.0]])
    r = matlab.refractive_index("si", matlab_list(w))
    n = rebuilt(r).reshape(w.shape, order="F")
    assert np.array_equal(n, po.refractive_index("si", wavelength_um=w))
    r = matlab.refractive_index("linbo3", array("d", [1.55]), axis="extraordinary")
    assert rebuilt(r)[0, 0] == po.refractive_index("linbo3", wavelength_um=1.55, axis="extraordinary")
    ng = rebuilt(matlab.group_index("sio2", [1.55]))
    assert ng[0, 0] == po.group_index("sio2", wavelength_um=1.55)


def test_slab_modes_come_back_as_columns() -> None:
    x = [0.0, 0.05, 0.11]
    r = matlab.slab_modes(1.444, 3.473, 1.444, 1.0, "te", 1.55, x)
    modes = po.slab_modes(below=1.444, core=3.473, above=1.444, thickness_um=1.0, polarization="te", wavelength_um=1.55, x_um=x)
    assert list(r["effective_index"]) == [m.effective_index for m in modes]
    assert list(r["order"]) == [float(m.order) for m in modes]
    field = rebuilt(r["field"])
    assert field.shape == (3, len(modes))
    for k, m in enumerate(modes):
        assert np.array_equal(field[:, k], m.field)
    # no points, no fields: an empty column per mode
    assert rebuilt(matlab.slab_modes(1.444, 3.473, 1.444, 0.22, "te", 1.55, []).get("field")).size == 0


def test_a_cross_section_goes_and_comes_back_in_matlabs_order() -> None:
    x = np.arange(0, 21) * 0.05
    y = np.arange(0, 13) * 0.0625
    eps = np.full((12, 20), 2.085 + 0j)
    eps[:2, :5] = 12.0 + 0.001j
    (ours,) = matlab.vector_modes(
        matlab_list(x),
        matlab_list(y),
        matlab_list(eps.real),
        matlab_list(eps.imag),
        1.55,
        1,
        None,
        ["electric", "zero", "magnetic", "zero"],
        [0.0, 0.0, 0.0, 0.0],
        0.0,
        600.0,
    )
    (mode,) = po.vector_modes(
        x_um=x,
        y_um=y,
        permittivity=eps,
        wavelength_um=1.55,
        boundaries=("electric", "zero", "magnetic", "zero"),
    )
    n = list(ours["effective_index"])
    assert complex(n[0], n[1]) == mode.effective_index
    for name in ("ex", "ey", "ez", "hx", "hy", "hz"):
        field = rebuilt(ours[name])
        assert field.shape == (12, 20)
        assert np.array_equal(field, getattr(mode, name)), name
    assert np.array_equal(np.asarray(ours["x_um"]), mode.x_um)


def test_a_circuit_from_matlabs_json() -> None:
    # jsonencode of the struct the help of circuit_spectrum.m shows
    netlist = json.dumps(
        {
            "instances": {
                "split": {"kind": "coupler", "coupling": 0.5},
                "upper": {"kind": "waveguide", "n_eff": 2.44506, "n_g": 4.172901, "wavelength_um": 1.55, "length": 150},
                "lower": {"kind": "waveguide", "n_eff": 2.44506, "n_g": 4.172901, "wavelength_um": 1.55, "length": 100},
                "combine": {"kind": "coupler", "coupling": 0.5},
            },
            "connections": [["split.o3", "upper.o1"], ["split.o4", "lower.o1"], ["upper.o2", "combine.o2"], ["lower.o2", "combine.o1"]],
            "ports": {"in1": "split.o2", "in2": "split.o1", "out1": "combine.o3", "out2": "combine.o4"},
        }
    )
    w = [1.54, 1.55, 1.56]
    r = matlab.circuit_spectrum(netlist, w)
    data = json.loads(netlist)
    ours = po.circuit_spectrum(**data, wavelength_um=w)
    assert r["ports"] == ours.ports
    s = rebuilt(r["s"])
    assert s.shape == (3, 4, 4) and np.array_equal(s, ours.s)


def test_touchstone_from_matlabs_arrays(tmp_path: Path) -> None:
    w = np.array([1.5, 1.55, 1.6])
    s = np.zeros((3, 2, 2), dtype=complex)
    s[:, 1, 0] = s[:, 0, 1] = np.exp(1j * w)
    file = tmp_path / "m.s2p"
    matlab.write_touchstone(str(file), ["a", "b"], matlab_list(w), matlab_list(s.real), matlab_list(s.imag), "physics", None)
    back = matlab.read_touchstone(str(file), "physics")
    assert np.array_equal(rebuilt(back["s"]), s[::-1])
    assert back["ports"] == ["o1", "o2"]


def test_jobs_as_paths_or_matlabs_json(tmp_path: Path) -> None:
    job = {
        "name": "straight",
        "task": {
            "kind": "fdfd",
            "stack": "soi_220",
            "wavelength_um": 1.55,
            "layer": "Si",
            "polarization": "te",
            "x_um": [-3.0, 3.0],
            "y_um": [-2.0, 2.0],
            "step_nm": 50.0,
            "rect": [{"layer": "Si", "center_um": [0.0, 0.0], "size_um": [6.0, 0.5]}],
            "port": [{"x_um": -1.5, "side": "left"}, {"x_um": 1.5, "side": "right"}],
        },
    }
    r = matlab.fdfd_s_parameters(json.dumps(job), [1.55])
    ours = po.fdfd_s_parameters(job, wavelength_um=1.55)
    assert np.array_equal(rebuilt(r["s"]), ours.s)
    assert np.array_equal(rebuilt(r["effective_index"]), ours.effective_index)
    # an empty list of wavelengths, as pylist([]) gives: the job's own
    assert rebuilt(matlab.fdfd_s_parameters(json.dumps(job), [])["s"]).shape == (1, 2, 2)
    run = matlab.run_job(json.dumps(job), str(tmp_path), 600.0)
    assert run["stopped"] == ""
    assert json.loads(run["events"][0])["type"] == "started"


def test_errors_carry_matlabs_identifier() -> None:
    with pytest.raises(po.errors.OutsideValidity) as e:
        matlab.refractive_index("si", [0.5])
    message = str(e.value)
    # what call.m reads: photonoxide:<kind>: and photonoxide's message
    found = re.search(r"photonoxide:(\w+): (.*)$", message)
    assert found and found.group(1) == "OutsideValidity"
    assert found.group(2) == "Si is only valid from 1.2 to 14 um, not at 0.5 um"
    assert e.value.material == "Si"
    with pytest.raises(po.errors.InvalidValue, match=r"^photonoxide:InvalidValue: "):
        matlab.refractive_index("si", [1.55], axis="sideways")


def test_the_catalogue_as_columns() -> None:
    m = matlab.materials()
    ours = po.materials()
    assert m["id"] == [x.id for x in ours]
    ranges = rebuilt(m["range_um"])
    assert ranges.shape == (2, len(ours))
    si = m["id"].index("si")
    assert tuple(ranges[:, si]) == (1.2, 14.0) and m["axes"][si] == "isotropic"
    assert matlab.version() == {"package": po.__version__, "core": po.core_version}
