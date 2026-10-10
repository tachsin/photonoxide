"""The package's functions: published numbers through Python (the validation report's cases
the package reaches), and what the functions promise: shapes, orders, units, errors, stops."""

from __future__ import annotations

import _thread
import inspect
import math
import re
import threading
import time
from pathlib import Path

import numpy as np
import pytest

import photonoxide as po

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = Path(__file__).resolve().parents[1]


# --- the validation report's numbers, through Python ---------------------------------------


def test_the_books_slab_modes() -> None:
    # mode/slab-te-book and mode/slab-tm-book: 220 nm of silicon (3.473) in oxide (1.444) at
    # 1550 nm against L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015),
    # doi:10.1017/CBO9781316084168, Section 3.2.2: 2.845 and 2.051, to 5e-4
    for polarization, book, report in (("te", 2.845, "2.84482"), ("tm", 2.051, "2.05110")):
        modes = po.slab_modes(
            below=1.444,
            core=3.473,
            above=1.444,
            thickness_um=0.22,
            polarization=polarization,
            wavelength_um=1.55,
        )
        n = modes[0].effective_index
        assert abs(n - book) <= 5e-4
        assert f"{n:.5f}" == report


def test_the_ring_free_spectral_range() -> None:
    # circuit/ring-fsr-bogaerts: an all-pass ring (a coupler of kappa^2 = 0.1 fed back through
    # 62.8 um of a guide of n_eff 2.4, n_g 4.2, 3 dB/cm), its resonances either side of 1.55 um
    # against W. Bogaerts et al., Laser Photonics Rev. 6, 47 (2012), doi:10.1002/lpor.201100017,
    # Eq. 9: lambda^2 / (n_g L), to 1e-4 nm; the report shows 9.08149 nm
    length = 2 * math.pi * 10.0

    def through(wavelength_um: np.ndarray) -> np.ndarray:
        s = po.circuit_spectrum(
            instances={
                "c": {"kind": "coupler", "coupling": 0.1},
                "ring": {
                    "kind": "waveguide",
                    "n_eff": 2.4,
                    "n_g": 4.2,
                    "wavelength_um": 1.55,
                    "length": length,
                    "loss": 3.0,
                },
            },
            connections=[("c.o3", "ring.o1"), ("ring.o2", "c.o2")],
            ports={"in": "c.o1", "through": "c.o4"},
            wavelength_um=wavelength_um,
        ).s
        return np.abs(s[:, 1, 0]) ** 2

    def golden(a: float, b: float) -> float:
        g = (math.sqrt(5.0) - 1.0) / 2.0
        c, d = b - g * (b - a), a + g * (b - a)
        fc, fd = through(np.array([c]))[0], through(np.array([d]))[0]
        while b - a > 1e-12:
            if fc < fd:
                b, d, fd = d, c, fc
                c = b - g * (b - a)
                fc = through(np.array([c]))[0]
            else:
                a, c, fc = c, d, fd
                d = a + g * (b - a)
                fd = through(np.array([d]))[0]
        return 0.5 * (a + b)

    grid = 1.53 + 1e-4 * np.arange(400)
    t = through(grid)
    minima = [
        golden(grid[i - 1], grid[i + 1])
        for i in range(1, len(grid) - 1)
        if t[i] < t[i - 1] and t[i] <= t[i + 1]
    ]
    below = max(w for w in minima if w < 1.55)
    above = min(w for w in minima if w >= 1.55)
    mid = 0.5 * (below + above)
    measured = (above - below) * 1e3
    assert abs(measured - mid**2 / (4.2 * length) * 1e3) <= 1e-4
    assert f"{measured:.5f}" == "9.08149"


def test_silicon_is_lis_table() -> None:
    # material/silicon-li-table: Li's Table 1 (H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980),
    # doi:10.1063/1.555624, 293 K) through a natural spline, which passes through its points, to
    # 1e-12: 3.4799, 3.4757 and 3.4719 at 1.50, 1.55 and 1.60 um
    n = po.refractive_index("si", wavelength_um=np.array([1.5, 1.55, 1.6]))
    assert np.all(n.imag == 0.0)
    assert np.max(np.abs(n.real - [3.4799, 3.4757, 3.4719])) <= 1e-12


def test_the_mzi_against_its_transfer_matrices() -> None:
    # circuit/mzi-closed-form: two lossless couplers and arms of 100 and 120 um; the product of
    # the transfer matrices C diag(t1, t2) C, to 1e-13
    w = np.linspace(1.54, 1.56, 41)
    for kappa2 in (0.5, 0.3):
        arm = dict(kind="waveguide", n_eff=2.4, n_g=4.2, wavelength_um=1.55, loss=3.0)
        s = po.circuit_spectrum(
            instances={
                "split": {"kind": "coupler", "coupling": kappa2},
                "upper": {**arm, "length": 100.0},
                "lower": {**arm, "length": 120.0},
                "combine": {"kind": "coupler", "coupling": kappa2},
            },
            connections=[
                ("split.o4", "upper.o1"),
                ("split.o3", "lower.o1"),
                ("upper.o2", "combine.o1"),
                ("lower.o2", "combine.o2"),
            ],
            ports={"in1": "split.o1", "in2": "split.o2", "out1": "combine.o4", "out2": "combine.o3"},
            wavelength_um=w,
        ).s
        n = 2.4 + (2.4 - 4.2) / 1.55 * (w - 1.55)
        alpha = 3.0 * 1e-4 * math.log(10) / 10

        def guide(length: float) -> np.ndarray:
            return np.exp((2j * np.pi * n / w - alpha / 2) * length)

        r, k = math.sqrt(1 - kappa2), 1j * math.sqrt(kappa2)
        c = np.array([[r, k], [k, r]])
        for i in range(len(w)):
            m = c @ np.diag([guide(100.0)[i], guide(120.0)[i]]) @ c
            # numpy's exp against Rust's on phases of about 1000 rad: a few ulps of them
            assert np.max(np.abs(s[i, 2:, :2] - m)) <= 1e-12


# --- shapes, orders and units ---------------------------------------------------------------


def test_one_wavelength_or_an_array_of_any_shape() -> None:
    one = po.refractive_index("sio2", wavelength_um=1.55)
    assert isinstance(one, complex)
    grid = np.linspace(1.3, 1.6, 6).reshape(2, 3)
    n = po.refractive_index("sio2", wavelength_um=grid)
    assert n.shape == (2, 3) and n.dtype == np.complex128
    assert n[1, 2] == po.refractive_index("sio2", wavelength_um=1.6)
    ng = po.group_index("sio2", wavelength_um=[1.31, 1.55])
    assert ng.shape == (2,) and isinstance(po.group_index("sio2", wavelength_um=1.55), float)


def test_fields_are_c_order_with_their_coordinates() -> None:
    # a strip's quarter on a grid with fewer rows than columns: (ny, nx), ex[j, i] at
    # (x_um[i], y_um[j]); a TE-like mode's E_x is largest in the core's corner cell at the origin
    x = np.arange(0, 21) * 0.05
    y = np.arange(0, 13) * 0.0625
    eps = np.full((12, 20), 2.085)
    eps[:2, :5] = 12.0
    (mode,) = po.vector_modes(
        x_um=x,
        y_um=y,
        permittivity=eps,
        wavelength_um=1.55,
        boundaries=("electric", "zero", "magnetic", "zero"),
    )
    assert mode.ex.shape == (12, 20) and mode.x_um.shape == (20,) and mode.y_um.shape == (12,)
    assert np.allclose(mode.x_um, 0.5 * (x[1:] + x[:-1]))
    j, i = np.unravel_index(np.argmax(np.abs(mode.ex)), mode.ex.shape)
    assert eps[j, i] == 12.0
    assert mode.te_fraction > 0.9
    assert 2.0 < mode.effective_index.real < 2.6


def test_s_matrices_are_wavelength_then_ports() -> None:
    s = po.circuit_spectrum(
        instances={"t": {"kind": "fixed", "ports": ["a", "b"], "s": [[[0, 0], [0.6, 0.8]], [[0.6, -0.8], [0, 0]]]}},
        ports={"in": "t.a", "out": "t.b"},
        wavelength_um=[1.5, 1.55, 1.6],
    )
    assert s.s.shape == (3, 2, 2)
    # s[k, q, p]: from port p into port q
    assert s.s[0, 1, 0] == 0.6 - 0.8j and s.s[0, 0, 1] == 0.6 + 0.8j


def test_a_touchstone_round_trip(tmp_path: Path) -> None:
    w = np.linspace(1.5, 1.6, 5)
    s = np.zeros((5, 2, 2), dtype=complex)
    s[:, 1, 0] = s[:, 0, 1] = np.exp(1j * np.linspace(0, 1, 5))
    file = tmp_path / "device.s2p"
    po.write_touchstone(file, po.Spectrum(ports=["a", "b"], wavelength_um=w, s=s), convention="engineering")
    back = po.read_touchstone(file, convention="engineering")
    # in the file's order, decreasing wavelength; the values exactly, the wavelengths c/f
    assert back.ports == ["o1", "o2"]
    assert np.array_equal(back.s, s[::-1])
    assert np.allclose(back.wavelength_um, w[::-1], rtol=1e-15)
    physics = po.read_touchstone(file, convention="physics")
    assert np.array_equal(physics.s, np.conj(back.s))


# --- jobs ----------------------------------------------------------------------------------


def test_a_job_file_runs_and_its_record_comes_back(tmp_path: Path) -> None:
    job = tmp_path / "strip.toml"
    job.write_text(
        'name = "strip"\n[task]\nkind = "modes"\nstack = "soi_220"\nwavelength_um = 1.55\n'
        'layer = "Si"\npropagation = "x"\ny_um = [-1.0, 1.0]\nstep_nm = 40.0\nmodes = 1\n'
        '[[task.rect]]\nlayer = "Si"\ncenter_um = [0.0, 0.0]\nsize_um = [10.0, 0.5]\n',
        encoding="utf-8",
    )
    po.check_job(job)
    run = po.run_job(job, runs_dir=tmp_path / "runs")
    assert run.dir.parent == tmp_path / "runs" and (run.dir / "events.jsonl").is_file()
    assert run.events[0]["type"] == "started" and run.events[-1]["type"] == "finished"
    assert run.stopped is None
    mode = next(e for e in run.events if e["type"] == "mode")
    assert 2.0 < mode["effective_index"][0] < 2.6


def test_a_job_as_data_gives_its_s_parameters() -> None:
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
    s = po.fdfd_s_parameters(job, wavelength_um=np.array([1.5, 1.55]))
    assert s.s.shape == (2, 2, 2) and s.effective_index.shape == (2, 2)
    assert s.cell_um == (0.05, 0.05) and s.polarization == "te"
    # a straight guide passes its mode whole
    assert np.all(np.abs(s.s[:, 1, 0]) > 0.99)


def test_a_job_stops_at_its_timeout(tmp_path: Path) -> None:
    ring = ROOT / "jobs" / "ring-fdfd.toml"
    run = po.run_job(ring, runs_dir=tmp_path, timeout_s=0.5)
    assert run.stopped == "timeout"
    assert run.events[-1]["type"] == "finished" and run.events[-1]["stopped"] == "timeout"


def test_a_mode_solve_stops_at_its_timeout() -> None:
    n = 400
    x = np.linspace(0.0, 4.0, n + 1)
    eps = np.full((n, n), 2.085)
    eps[180:220, 150:250] = 12.0
    with pytest.raises(TimeoutError):
        po.vector_modes(x_um=x, y_um=x, permittivity=eps, wavelength_um=1.55, count=40, timeout_s=0.05)


def test_ctrl_c_stops_a_mode_solve() -> None:
    # 60 modes of a 200 x 200 grid take over a minute; Ctrl+C (as interrupt_main sends it)
    # after 0.3 s stops the solver at its next step and raises KeyboardInterrupt
    n = 200
    x = np.linspace(0.0, 4.0, n + 1)
    eps = np.full((n, n), 2.085)
    eps[90:110, 75:125] = 12.0
    threading.Timer(0.3, _thread.interrupt_main).start()
    start = time.monotonic()
    with pytest.raises(KeyboardInterrupt):
        po.vector_modes(x_um=x, y_um=x, permittivity=eps, wavelength_um=1.55, count=60, timeout_s=None)
    assert time.monotonic() - start < 30.0


# --- errors ----------------------------------------------------------------------------------


def test_errors_are_photonoxides_with_their_fields() -> None:
    with pytest.raises(po.errors.OutsideValidity) as e:
        po.refractive_index("si", wavelength_um=0.5)
    assert isinstance(e.value, ValueError) and isinstance(e.value, po.errors.Error)
    assert (e.value.material, e.value.wavelength_um, e.value.shortest_um) == ("Si", 0.5, 1.2)
    assert str(e.value) == "Si is only valid from 1.2 to 14 um, not at 0.5 um"
    with pytest.raises(po.errors.InvalidValue) as e:
        po.refractive_index("si", wavelength_um=-1.0)
    assert e.value.what == "wavelength"
    with pytest.raises(po.errors.InvalidValue, match="ordinary and extraordinary"):
        po.refractive_index("linbo3", wavelength_um=1.55)
    with pytest.raises(po.errors.IoError) as e:
        po.read_touchstone("no-such-file.s2p", convention="physics")
    assert isinstance(e.value, OSError) and e.value.path == "no-such-file.s2p"
    with pytest.raises(po.errors.NetlistError):
        po.circuit_spectrum(
            instances={"c": {"kind": "coupler"}}, ports={"x": "c.o9"}, wavelength_um=1.55
        )
    with pytest.raises(po.errors.ParseError):
        po.check_job({"name": "x", "task": {"kind": "modes", "nonsense": 1}})


# --- the package ----------------------------------------------------------------------------


def test_versions() -> None:
    # the package's version is the crate's, or a post-release of it
    assert re.fullmatch(r"\d+\.\d+\.\d+(\.post\d+)?", po.__version__)
    assert po.__version__.split(".post")[0] == po.core_version


def test_the_catalogue_has_its_sources() -> None:
    si = next(m for m in po.materials() if m.id == "si")
    assert si.models[0].default and si.models[0].range_um == (1.2, 14.0)
    assert "doi:10.1063/1.555624" in si.models[0].sources[0]


def test_the_readme_and_the_module_example_run(capsys: pytest.CaptureFixture[str]) -> None:
    readme = (PACKAGE / "README.md").read_text(encoding="utf-8")
    (block,) = re.findall(r"```python\n(.*?)```", readme, re.DOTALL)
    exec(compile(block, "README.md", "exec"), {})
    assert capsys.readouterr().out == "TE0: n_eff = 2.8475\nTM0: n_eff = 2.0531\n"
    example = inspect.cleandoc(po.__doc__ or "").split("::\n")[1].split("\n\nIt prints")[0]
    exec(compile(inspect.cleandoc(example), "photonoxide.__doc__", "exec"), {})
    assert capsys.readouterr().out == "TE0: n_eff = 2.8475\nTM0: n_eff = 2.0531\n"


def test_the_circuit_example_runs() -> None:
    doc = inspect.getdoc(po.circuit_spectrum) or ""
    example = doc.split("::\n")[1].split("\n\nEach instance")[0]
    namespace: dict[str, object] = {}
    exec(compile(inspect.cleandoc(example), "circuit_spectrum.__doc__", "exec"), namespace)
    bar = namespace["bar"]
    assert isinstance(bar, np.ndarray) and bar.shape == (1001,) and 0.0 <= bar.min() < bar.max() <= 1.0


def test_the_python_page_runs(capsys: pytest.CaptureFixture[str]) -> None:
    # docs/python.md's Python blocks, in order; the first is the README's first example
    page = (ROOT / "docs" / "python.md").read_text(encoding="utf-8")
    blocks = re.findall(r"```python\n(.*?)```", page, re.DOTALL)
    assert len(blocks) == 2
    for block in blocks:
        exec(compile(block, "docs/python.md", "exec"), {})
    out = capsys.readouterr().out.splitlines()
    assert out[:2] == ["TE0: n_eff = 2.8475", "TM0: n_eff = 2.0531"]
    # the strip on its 25 nm grid: TE-like, near the book's 2.443 (a 5 nm grid's number)
    n, te, shape = out[2].split(" ", 2)
    assert 2.43 < float(n) < 2.46 and float(te) > 0.9 and shape == "(32, 40)"
