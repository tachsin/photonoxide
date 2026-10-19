# photonoxide in Python and MATLAB

The `photonoxide` package for Python wraps a part of the library: the materials catalogue,
the exact slab's and the full-vector solver's modes, jobs (run as the program runs them) and
an FDFD job's S-parameters, circuits from a netlist given as data, Touchstone files, and the
devices of a component's page in the studio, with the same parameters, models and numbers. MATLAB
reaches the same functions through the package. The package lives in this repository, in
[`python/`](../python), and wraps the library's façade, `photonoxide::facade`: a small module of
plain types, so that the rest of the library's API can change between milestones without the
package following every change ([the plan](plans/bindings.md), #271).

- [Install](#install)
- [A first example](#a-first-example)
- [Units, arrays and errors](#units-arrays-and-errors)
- [What it does](#what-it-does)
- [Devices](#devices)
- [The same numbers as Rust](#the-same-numbers-as-rust)
- [MATLAB](#matlab)
- [Versions and releases](#versions-and-releases)

## Install

```sh
pip install photonoxide
```

Wheels for CPython 3.10 and later (one abi3 wheel per platform, and one for free-threaded
3.14t) on Linux (x86_64 and ARM64, glibc and musl), macOS (Apple silicon and Intel) and Windows
(x64). Nothing else is needed: the wheel holds photonoxide, compiled, and NumPy is its one
dependency. *(The package reaches PyPI with the first release after its publishing is set up;
until then, build it from the repository.)*

From the repository, with Rust installed:

```sh
pip install ./python          # builds the package with maturin, in release mode
```

or, while working on it, `maturin develop --release` in `python/` (in a virtual environment).

## A first example

220 nm of silicon in oxide at 1550 nm, each material with its published dispersion, and the
slab's modes solved exactly, as the [README](../README.md)'s first example does in Rust:

```python
import photonoxide as phox

si = phox.refractive_index("si", wavelength_um=1.55).real    # Li 1980
ox = phox.refractive_index("sio2", wavelength_um=1.55).real  # Malitson 1965
for polarization in ("te", "tm"):
    for mode in phox.slab_modes(below=ox, core=si, above=ox, thickness_um=0.22,
                              polarization=polarization, wavelength_um=1.55):
        print(f"{polarization.upper()}{mode.order}: n_eff = {mode.effective_index:.4f}")
```

It prints `TE0: n_eff = 2.8475` and `TM0: n_eff = 2.0531`, as the Rust example does.

## Units, arrays and errors

- **Units are in the names,** as job files have them: `wavelength_um`, `thickness_um`,
  `radius_um`, `loss_db_per_cm`. A length can't be passed where a wavelength goes without the
  name saying so.
- **Vectorized over wavelength** where Rust loops: `refractive_index`, `group_index`,
  `circuit_spectrum`, `device_spectrum` and `fdfd_s_parameters` take one wavelength or an array
  (`wavelength_um=np.linspace(1.5, 1.6, 101)`); one gives one value, an array an array of its
  shape.
- **NumPy arrays in C order, with their coordinates.** A field on a cross-section is `(ny, nx)`:
  `mode.ex[j, i]` is E_x at `(mode.x_um[i], mode.y_um[j])`. S-matrices at many wavelengths are
  `(nλ, n, n)`: `s[k, q, p]` is S_qp at the k-th wavelength, from port p into port q, in
  photonoxide's e^(−iωt) convention, power-normalized.
- **Errors are photonoxide's,** with its messages, as the classes of `photonoxide.errors`, each
  also the built-in it resembles: `InvalidValue` and `OutsideValidity` are `ValueError`s,
  `IoError` an `OSError`, and their fields are attributes (`e.material`, `e.shortest_um`). A
  wavelength outside a material's data is an error: photonoxide doesn't extrapolate.
- **Long calls stop.** `vector_modes` (600 s by default), `run_job` (3600 s, and the job's
  own `timeout_minutes`) and `device_spectrum` (no limit by default) take `timeout_s`; Ctrl+C
  stops them too, once the work has stopped. A device's solve is let go at once, its result
  discarded, and runs on to its end on a thread of its own.
  Every call releases the GIL while Rust works, and photonoxide's threads never touch Python.

## What it does

| Function | What | Rust |
|---|---|---|
| `materials()` | the catalogue: each material's index models, their axes, ranges, conditions and sources by DOI | [`material::catalogue`](methods/catalogue.md) |
| `refractive_index(id, wavelength_um=, model=, axis=, temperature_k=, composition=)` | n + iκ | [`Material::refractive_index`](methods/materials.md) |
| `group_index(...)` | the bulk group index | `Material::group_index` |
| `slab_modes(below=, core=, above=, thickness_um=, polarization=, wavelength_um=, x_um=)` | the exact slab's guided modes and their fields | [`mode::slab`](methods/slab.md) |
| `vector_modes(x_um=, y_um=, permittivity=, wavelength_um=, count=, near_index=, boundaries=, pml_um=, pml_strength=)` | a cross-section's guided full-vector modes, with E and H: of `count` solved, those above the media at the window's open edges (fewer when fewer are guided); with PMLs every one, leaky ones with their loss | [`mode::vector`](methods/vector.md) |
| `check_job(job)`, `run_job(job, runs_dir=)` | a job checked, or run into a run folder as `photonoxide run --headless` does, its events returned | [`job`](../src/job/mod.rs) |
| `fdfd_s_parameters(job, wavelength_um=)` | an `"fdfd"` job's S-parameters by 2D FDFD | [`fdfd`](methods/fdfd-ports.md) |
| `circuit_spectrum(instances=, connections=, ports=, wavelength_um=)` | a circuit's S-matrices from a netlist given as data | [`circuit`](methods/circuits.md) |
| `read_touchstone(path, convention=)`, `write_touchstone(path, spectrum, convention=)` | Touchstone files, the time convention always named | [`compact::touchstone`](methods/compact.md) |
| `devices()` | the library's devices: each one's parameters (unit, default, range), models and ports | [`device`](methods/devices.md) |
| `stacks()` | the catalogue's stacks, which the device functions take by name as `stack=`: each one's layers of the catalogue's materials and its core layer | [`device::PRESETS`](methods/devices.md) |
| `device_spectrum(kind, values=, fidelity=, stack=, wavelength_um=, timeout_s=)`, `device_derived(kind, ...)` | a design's S-parameters and figures at a fidelity, each with its model's grid | [`device`](methods/devices.md) |
| `device_geometry(kind, values=, stack=)`, `device_job(kind, ..., format=)` | a design's stack, polygons and ports; the job that computes it | [`device`](methods/devices.md) |

Each function's docstring has the details; `help(phox.circuit_spectrum)` lists the kinds of
component a netlist can hold and their settings. A job is a path to a job file (TOML, JSON or
YAML) or the job as a dict, so every kind of job, FDTD included, runs from Python.
[docs/features.md](features.md) has, for every method of the library, whether it works in Rust,
Python and MATLAB.

A strip's modes, from a cross-section built with NumPy (a quarter of a 500 × 220 nm silicon strip
in oxide, mirror walls through its centre for the TE-like modes, on a 25 nm grid):

```python
import numpy as np
import photonoxide as phox

x = np.arange(0, 41) * 0.025                 # nodes, µm: x from the strip's centre
y = np.arange(0, 33) * 0.0275                # y from its centre
xc, yc = 0.5 * (x[1:] + x[:-1]), 0.5 * (y[1:] + y[:-1])
si = phox.refractive_index("si", wavelength_um=1.55).real ** 2
eps = np.where((xc[None, :] < 0.25) & (yc[:, None] < 0.11), si, 1.444**2)   # (ny, nx)
(te,) = phox.vector_modes(x_um=x, y_um=y, permittivity=eps, wavelength_um=1.55,
                        boundaries=("electric", "zero", "magnetic", "zero"))
print(te.effective_index.real, te.te_fraction, te.ex.shape)
```

A 2D FDFD result (`fdfd_s_parameters`, an `"fdfd"` job) comes from the effective index method:
an estimate, not a device's performance in 3D, and it carries its grid (`cell_um`).

## Devices

The devices are the components of the studio's Components page: a strip or rib waveguide, a
bend, a directional coupler, rings, MMIs, a Y-branch and MZIs, each with its parameters, its
models from a closed form to 3D FDFD, and its ports (`phox.devices()` lists them). A design is a
device's kind, its parameters' values (those not given at their defaults), a fidelity (its
cheapest when not given), a stack of the catalogue's by name (220 nm SOI by default, or
`"soi"` or `"nitride"`; `phox.stacks()` lists them with their layers), and for
the compact fidelity the file of a compact model fitted to the device's runs (`compact=`, the
`compact.json` a `"fit"` job's run writes in its folder). For
the same design, `device_spectrum` and `device_derived` give the numbers the device's page in
the studio shows, to the bit, and every number names its fidelity and grid.

A directional coupler swept over its gap, at its analytic model (a fit to the full-vector
solver):

```python
import numpy as np
import photonoxide as phox

w = np.linspace(1.5, 1.6, 101)
for gap in (0.15, 0.2, 0.25, 0.3):
    design = dict(values={"gap": gap, "length": 10.0}, fidelity="analytic")
    figures = {f.symbol: f.value for f in phox.device_derived("directional coupler", **design)}
    dc = phox.device_spectrum("directional coupler", **design, wavelength_um=w)
    across = np.abs(dc.s[:, 2, 0]) ** 2   # from o1, lower left, into o3, upper right
    print(f"gap {gap:.2f} um: L_x {figures['L_x']:.1f} um, across {across[50]:.3f} at 1.55 um")
print(dc.fidelity, "|", dc.grid)
```

It prints the cross-over length growing with the gap, 23.4 µm at 150 nm to 89.5 µm at 300 nm,
and the power across a 10 µm coupler falling from 0.385 to 0.030, each from the fit's grid of
about 10 nm, which `dc.grid` names. At `fidelity="3D"` the same loop solves each
design's supermodes by the full-vector solver (about ten seconds a design here, on its 10 nm
grid), and `dc.grid` names that grid.
`device_geometry` gives the polygons to draw or check, and `device_job` the job file that
computes a design, which `run_job` runs as the studio does: its record holds the same numbers.

A circuit's instances can be devices too: in `circuit_spectrum`'s netlist, an instance names a
`device` in place of a `kind`, its design as a job's `[device]` table (and a chip file's
instance) holds it, `{"device": {"kind": "all-pass ring", "parameters": {"radius": 5.0}}}`,
and is built at its fidelity as `device_spectrum` builds it.

## The same numbers as Rust

The package computes nothing itself: each function calls the façade, and the façade calls the
library as a Rust program would. Both steps are tested:

- the façade's Rust tests check each of its functions against the library's own calls, bit for
  bit (`src/facade/tests.rs`);
- `photonoxide::facade::conformance()` lists calls with their results, computed by Rust on the
  machine that asks; the package's tests (`python/tests/test_conformance.py`) make each call
  again through Python and compare every number by its bits;
- the validation report's cases the package reaches are checked through Python against their
  published values: the book's slab (`mode/slab-te-book`, 2.84482 against Chrostowski and
  Hochberg's 2.845), a ring's free spectral range against Bogaerts et al.'s Eq. 9
  (`circuit/ring-fsr-bogaerts`, 9.08149 nm), an MZI against its transfer matrices, silicon
  against Li's table;
- [`python/examples`](../python/examples) has Python versions of a few of the Rust examples
  (silicon's index, the book's slab and strip, a ring's free spectral range), each checked
  against its paper as the Rust ones are; those with a Rust twin print what it prints, line
  for line (`python/tests/test_examples.py`).

The bits are the same for the same build on the same machine; another CPU or another build of
photonoxide can differ in the last bits, as it can in Rust.

## MATLAB

MATLAB calls the package through its Python interface (`py.`), from R2022b, the first release
that supports Python 3.10. The `.m` wrappers ship in the wheel, in a `+photonoxide` package
folder; one install gives both.

```matlab
pyenv(Version="/path/to/python");             % a Python with photonoxide installed, once
addpath(string(py.photonoxide.matlab.path()));

n = photonoxide.refractive_index("si", [1.31 1.55]);
m = photonoxide.slab_modes(1.444, 3.473, 1.444, 0.22, "te", 1.55);
m.effective_index                              % 2.8448...

netlist.instances.split = struct("kind", "coupler", "coupling", 0.5);
netlist.instances.upper = struct("kind", "waveguide", "n_eff", 2.44506, "n_g", 4.172901, ...
                                 "wavelength_um", 1.55, "length", 150);
netlist.instances.lower = struct("kind", "waveguide", "n_eff", 2.44506, "n_g", 4.172901, ...
                                 "wavelength_um", 1.55, "length", 100);
netlist.instances.combine = struct("kind", "coupler", "coupling", 0.5);
netlist.connections = {{"split.o3", "upper.o1"}, {"split.o4", "lower.o1"}, ...
                       {"upper.o2", "combine.o2"}, {"lower.o2", "combine.o1"}};
netlist.ports = struct("in1", "split.o2", "in2", "split.o1", "out1", "combine.o3", "out2", "combine.o4");
s = photonoxide.circuit_spectrum(netlist, linspace(1.5, 1.6, 1001));
plot(s.wavelength_um, abs(s.s(:, 3, 1)).^2)    % from in1 to out1
```

The wrappers: `photonoxide.version`, `materials`, `refractive_index`, `group_index`,
`slab_modes`, `vector_modes`, `run_job`, `fdfd_s_parameters`, `circuit_spectrum`,
`read_touchstone`, `write_touchstone`, `devices`, `stacks`, `device_spectrum`,
`device_derived`, `device_geometry` and `device_job`, each with its help (`help photonoxide.vector_modes`);
[docs/matlab.md](matlab.md) is their reference, written from that help.
They take MATLAB's arrays and name-value arguments and return MATLAB arrays and structs:

- arrays keep the Python package's shapes and indexing, so a field is `ny`-by-`nx` and
  `ex(j, i)` is the point `(x_um(i), y_um(j))`; S-matrices are `nλ`-by-`n`-by-`n`,
  `s.s(k, q, p)` from port p into port q;
- a job or a netlist can be a struct, which the wrappers pass on as `jsonencode` writes it;
- photonoxide's errors are MATLAB errors with the identifiers `photonoxide:InvalidValue`,
  `photonoxide:OutsideValidity`, `photonoxide:Timeout` and so on, and photonoxide's messages.

The `.m` files are a few lines each: every conversion is in the package's Python module
`photonoxide.matlab`, which passes arrays flat in MATLAB's column-major order with their shapes.

**What is tested, and what isn't.** The conversions are tested in Python
(`python/tests/test_matlab.py`): each function is called as the wrappers call it, and what comes
back, rebuilt as the wrappers rebuild it, is checked against the Python package's results to the
bit; the tests also check that the wrappers ship in the wheel and call the module's functions by
their names. MATLAB itself isn't run: MathWorks' GitHub action doesn't license MATLAB's external
language interfaces, Python's among them, on public projects, and GNU Octave has no `py.`
interface of its own. **MATLAB support is untested in MATLAB** until someone with a licence
runs the wrappers by hand; the MATLAB releases checked will be listed here.

## Versions and releases

The package's version is the crate's it wraps (`photonoxide` 0.5.1 on PyPI is the crate 0.5.1):
both come from `[workspace.package]` in the root `Cargo.toml`, which a release bumps, so they
can't drift. `phox.__version__` is the package's and `phox.core_version` the crate's. A fix to the
package alone, between releases, is a PEP 440 post-release, `0.5.1.post1`: set
`version = "0.5.1.post1"` under `[project]` in `python/pyproject.toml` (and take `"version"`
out of its `dynamic`), merge, and publish by hand (below); the next release takes the static
version out again.

**CI.** `.github/workflows/python-wheels.yml` runs on pull requests that touch `python/`, the
library (`src/`), `Cargo.toml`, `Cargo.lock` or `jobs/`, and only on those: it lints the binding
crate and builds and tests the wheels of Linux x86_64, macOS ARM64 and Windows x64, on Python
3.10 and on free-threaded 3.14t. A change to `python/` alone runs none of the main CI's Rust
jobs; the other platforms and the sdist are built when publishing.

**Publishing.** PyPI publishes by trusted publishing: PyPI trusts this repository's
`python-wheels.yml` in the `pypi` environment, so no token is stored. Setting it up is done once,
by hand:

1. On PyPI, signed in as the account that will own the project, open
   [pypi.org/manage/account/publishing](https://pypi.org/manage/account/publishing/) (*Your
   account* → *Publishing*) and, under *Add a new pending publisher*, choose *GitHub* and fill
   in
   - PyPI project name: `photonoxide`
   - Owner: `tachsin`
   - Repository name: `photonoxide`
   - Workflow name: `python-wheels.yml`
   - Environment name: `pypi`

   (A pending publisher creates the project on its first upload.)
2. On GitHub, in the repository's *Settings* → *Environments*, create the environment `pypi`
   (optionally requiring a reviewer, or limited to tags `v*` and `main`).
3. In *Settings* → *Secrets and variables* → *Actions* → *Variables*, add the repository
   variable `PYPI_PUBLISH` with the value `true`.

From then on, each release (the release PR merged) starts `python-wheels.yml` at the new tag
with `publish`: every platform's wheels and the sdist are built, tested and uploaded. Until the
variable is set, a release publishes no package and fails nothing. To publish by hand (a
post-release, or a release made before the setup): *Actions* → *Python wheels* → *Run
workflow*, on `main` or a tag, with *publish* ticked.
