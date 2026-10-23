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

import photonoxide as phox
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


def test_every_option_a_wrapper_reads_is_in_its_arguments_block() -> None:
    # MATLAB refuses options.<name> when the arguments block doesn't declare it, which only a
    # run in MATLAB would show: device_derived and device_job read options.compact without
    # declaring it until #291's write-ups (device_spectrum until #446)
    for file in sorted(WRAPPERS.glob("*.m")):
        code = "\n".join(
            line
            for line in file.read_text(encoding="utf-8").splitlines()
            if not line.lstrip().startswith("%")
        )
        declared = set(re.findall(r"^\s*options\.(\w+) \(", code, re.MULTILINE))
        read = set(re.findall(r"options\.(\w+)", code))
        assert read <= declared, f"{file.name} reads options {sorted(read - declared)} it doesn't declare"


def test_indices_come_back_in_matlabs_order_to_the_bit() -> None:
    w = np.array([[1.31, 1.45], [1.55, 2.0]])
    r = matlab.refractive_index("si", matlab_list(w))
    n = rebuilt(r).reshape(w.shape, order="F")
    assert np.array_equal(n, phox.refractive_index("si", wavelength_um=w))
    r = matlab.refractive_index("linbo3", array("d", [1.55]), axis="extraordinary")
    assert rebuilt(r)[0, 0] == phox.refractive_index("linbo3", wavelength_um=1.55, axis="extraordinary")
    ng = rebuilt(matlab.group_index("sio2", [1.55]))
    assert ng[0, 0] == phox.group_index("sio2", wavelength_um=1.55)


def test_slab_modes_come_back_as_columns() -> None:
    x = [0.0, 0.05, 0.11]
    r = matlab.slab_modes(1.444, 3.473, 1.444, 1.0, "te", 1.55, x)
    modes = phox.slab_modes(below=1.444, core=3.473, above=1.444, thickness_um=1.0, polarization="te", wavelength_um=1.55, x_um=x)
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
    (mode,) = phox.vector_modes(
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
    ours = phox.circuit_spectrum(**data, wavelength_um=w)
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
    ours = phox.fdfd_s_parameters(job, wavelength_um=1.55)
    assert np.array_equal(rebuilt(r["s"]), ours.s)
    assert np.array_equal(rebuilt(r["effective_index"]), ours.effective_index)
    # an empty list of wavelengths, as pylist([]) gives: the job's own
    assert rebuilt(matlab.fdfd_s_parameters(json.dumps(job), [])["s"]).shape == (1, 2, 2)
    run = matlab.run_job(json.dumps(job), str(tmp_path), 600.0)
    assert run["stopped"] == ""
    assert json.loads(run["events"][0])["type"] == "started"


def test_errors_carry_matlabs_identifier() -> None:
    with pytest.raises(phox.errors.OutsideValidity) as e:
        matlab.refractive_index("si", [0.5])
    message = str(e.value)
    # what call.m reads: photonoxide:<kind>: and photonoxide's message
    found = re.search(r"photonoxide:(\w+): (.*)$", message)
    assert found and found.group(1) == "OutsideValidity"
    assert found.group(2) == "Si is only valid from 1.2 to 14 um, not at 0.5 um"
    assert e.value.material == "Si"
    with pytest.raises(phox.errors.InvalidValue, match=r"^photonoxide:InvalidValue: "):
        matlab.refractive_index("si", [1.55], axis="sideways")


def test_the_catalogue_as_columns() -> None:
    m = matlab.materials()
    ours = phox.materials()
    assert m["id"] == [x.id for x in ours]
    ranges = rebuilt(m["range_um"])
    assert ranges.shape == (2, len(ours))
    si = m["id"].index("si")
    assert tuple(ranges[:, si]) == (1.2, 14.0) and m["axes"][si] == "isotropic"
    assert matlab.version() == {"package": phox.__version__, "core": phox.core_version}


def test_devices_come_back_as_matlabs_json() -> None:
    ours = json.loads(matlab.devices())
    assert [d["kind"] for d in ours] == [d.kind for d in phox.devices()]
    assert ours[2]["parameters"][4]["name"] == "gap"
    # a design as jsonencode writes a struct of its values, and none as "{}" or "[]"
    w = [1.54, 1.55, 1.56]
    r = matlab.device_spectrum("directional coupler", json.dumps({"gap": 0.25}), "analytic", w)
    dc = phox.device_spectrum("directional coupler", values={"gap": 0.25}, fidelity="analytic", wavelength_um=w)
    s = rebuilt(r["s"])
    assert s.shape == (3, 4, 4) and np.array_equal(s, dc.s)
    assert r["ports"] == dc.ports and r["fidelity"] == "analytic" and r["grid"] == dc.grid
    assert list(r["validity_um"]) == list(dc.validity_um or ())
    ring = matlab.device_spectrum("all-pass ring", "{}", "", [1.55], "")
    assert np.array_equal(rebuilt(ring["s"]), phox.device_spectrum("all-pass ring", wavelength_um=[1.55]).s)
    figures = json.loads(matlab.device_derived("waveguide", json.dumps({"width": 0.45}), ""))
    n = next(f for f in figures if f["symbol"] == "n_eff")["value"]
    assert n == next(f.value for f in phox.device_derived("waveguide", values={"width": 0.45}) if f.symbol == "n_eff")


def test_a_devices_geometry_and_job_for_matlab() -> None:
    g = json.loads(matlab.device_geometry("all-pass ring", "[]"))
    ours = phox.device_geometry("all-pass ring")
    si = [layer for layer in g["layers"] if layer["name"] == "Si"][0]
    ring = [p for p in si["polygons"] if p["holes"]][0]
    # each outline n rows of (x, y), as jsondecode makes an n-by-2 matrix of it
    expected = next(p for layer in ours.layers for p in layer.polygons if p.holes)
    assert np.array_equal(np.array(ring["outer"]), expected.outer)
    assert g["ports"][0]["position_um"] == list(ours.ports[0].position_um)
    text = matlab.device_job("waveguide", "{}", "", "", "", "json")
    assert json.loads(text) == phox.device_job("waveguide")
    assert matlab.device_job("waveguide", "{}", "", "", "strip", "yaml").startswith("name: strip")


def test_a_netlist_of_devices_from_matlabs_json() -> None:
    # jsonencode of the struct circuit_spectrum.m's help shows for a device
    netlist = json.dumps(
        {
            "instances": {"ring": {"device": {"kind": "all-pass ring", "parameters": {"radius": 5}}}},
            "ports": {"in": "ring.in", "through": "ring.through"},
        }
    )
    r = matlab.circuit_spectrum(netlist, [1.55, 1.56])
    ours = phox.device_spectrum("all-pass ring", values={"radius": 5.0}, wavelength_um=[1.55, 1.56])
    assert np.array_equal(rebuilt(r["s"]), ours.s)


def test_the_stacks_as_matlabs_json() -> None:
    ours = json.loads(matlab.stacks())
    stacks = phox.stacks()
    assert [s["name"] for s in ours] == [s.name for s in stacks]
    assert ours[0]["layers"][1] == {"name": "Si", "thickness_um": 0.22, "material": "si", "background": "sio2"}
    assert ours[2]["core"] == stacks[2].core


def test_a_devices_spectrum_with_matlabs_timeout() -> None:
    w = [1.55, 1.56]
    ours = phox.device_spectrum("MMI 1x2", fidelity="2D", stack="nitride", wavelength_um=w)
    # Inf, as the wrapper's default, is no timeout; a number is one
    for timeout in (float("inf"), 600.0):
        r = matlab.device_spectrum("MMI 1x2", "{}", "2D", w, "nitride", "", timeout)
        assert np.array_equal(rebuilt(r["s"]), ours.s)
    with pytest.raises(TimeoutError, match=r"^photonoxide:Timeout: "):
        matlab.device_spectrum("MMI 1x2", "{}", "2D", w, "", "", 1e-9)
