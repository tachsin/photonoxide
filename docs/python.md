# photonoxide in Python and MATLAB

The `photonoxide` package for Python wraps a part of the library: the materials catalogue,
the exact slab's and the full-vector solver's modes, jobs (run as the program runs them) and
an FDFD job's S-parameters, circuits from a netlist given as data, and Touchstone files. MATLAB
reaches the same functions through the package. The package lives in this repository, in
[`python/`](../python), and wraps the library's façade, `photonoxide::facade`: a small module of
plain types, so that the rest of the library's API can change between milestones without the
package following every change ([the plan](plans/bindings.md), #271).

- [Install](#install)
- [A first example](#a-first-example)
- [Units, arrays and errors](#units-arrays-and-errors)
- [What it does](#what-it-does)
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
import photonoxide as po

si = po.refractive_index("si", wavelength_um=1.55).real    # Li 1980
ox = po.refractive_index("sio2", wavelength_um=1.55).real  # Malitson 1965
for polarization in ("te", "tm"):
    for mode in po.slab_modes(below=ox, core=si, above=ox, thickness_um=0.22,
                              polarization=polarization, wavelength_um=1.55):
        print(f"{polarization.upper()}{mode.order}: n_eff = {mode.effective_index:.4f}")
```

It prints `TE0: n_eff = 2.8475` and `TM0: n_eff = 2.0531`, as the Rust example does.

## Units, arrays and errors

- **Units are in the names,** as job files have them: `wavelength_um`, `thickness_um`,
  `radius_um`, `loss_db_per_cm`. A length can't be passed where a wavelength goes without the
  name saying so.
- **Vectorized over wavelength** where Rust loops: `refractive_index`, `group_index`,
  `circuit_spectrum` and `fdfd_s_parameters` take one wavelength or an array
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
- **Long calls stop.** `vector_modes` (600 s by default) and `run_job` (3600 s, and the job's
  own `timeout_minutes`) take `timeout_s`; Ctrl+C stops them too, once the work has stopped.
  Every call releases the GIL while Rust works, and photonoxide's threads never touch Python.

## What it does

| Function | What | Rust |
|---|---|---|
| `materials()` | the catalogue: each material's index models, their axes, ranges, conditions and sources by DOI | [`material::catalogue`](methods/catalogue.md) |
| `refractive_index(id, wavelength_um=, model=, axis=, temperature_k=, composition=)` | n + iκ | [`Material::refractive_index`](methods/materials.md) |
| `group_index(...)` | the bulk group index | `Material::group_index` |
| `slab_modes(below=, core=, above=, thickness_um=, polarization=, wavelength_um=, x_um=)` | the exact slab's guided modes and their fields | [`mode::slab`](methods/slab.md) |
| `vector_modes(x_um=, y_um=, permittivity=, wavelength_um=, count=, near_index=, boundaries=, pml_um=, pml_strength=)` | full-vector modes of a cross-section, with E and H | [`mode::vector`](methods/vector.md) |
| `check_job(job)`, `run_job(job, runs_dir=)` | a job checked, or run into a run folder as `photonoxide run --headless` does, its events returned | [`job`](../src/job/mod.rs) |
| `fdfd_s_parameters(job, wavelength_um=)` | an `"fdfd"` job's S-parameters by 2D FDFD | [`fdfd`](methods/fdfd-ports.md) |
| `circuit_spectrum(instances=, connections=, ports=, wavelength_um=)` | a circuit's S-matrices from a netlist given as data | [`circuit`](methods/circuits.md) |
| `read_touchstone(path, convention=)`, `write_touchstone(path, spectrum, convention=)` | Touchstone files, the time convention always named | [`compact::touchstone`](methods/compact.md) |

Each function's docstring has the details; `help(po.circuit_spectrum)` lists the kinds of
component a netlist can hold and their settings. A job is a path to a job file (TOML, JSON or
YAML) or the job as a dict, so every kind of job, FDTD included, runs from Python.
[docs/features.md](features.md) has, for every method of the library, whether it works in Rust,
Python and MATLAB.

A strip's modes, from a cross-section built with NumPy (a quarter of a 500 × 220 nm silicon strip
in oxide, mirror walls through its centre for the TE-like modes, on a 25 nm grid):

```python
import numpy as np
import photonoxide as po

x = np.arange(0, 41) * 0.025                 # nodes, µm: x from the strip's centre
y = np.arange(0, 33) * 0.0275                # y from its centre
xc, yc = 0.5 * (x[1:] + x[:-1]), 0.5 * (y[1:] + y[:-1])
si = po.refractive_index("si", wavelength_um=1.55).real ** 2
eps = np.where((xc[None, :] < 0.25) & (yc[:, None] < 0.11), si, 1.444**2)   # (ny, nx)
(te,) = po.vector_modes(x_um=x, y_um=y, permittivity=eps, wavelength_um=1.55,
                        boundaries=("electric", "zero", "magnetic", "zero"))
print(te.effective_index.real, te.te_fraction, te.ex.shape)
```

A 2D FDFD result (`fdfd_s_parameters`, an `"fdfd"` job) comes from the effective index method:
an estimate, not a device's performance in 3D, and it carries its grid (`cell_um`).

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
  against Li's table.

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
`read_touchstone` and `write_touchstone`, each with its help (`help photonoxide.vector_modes`).
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
can't drift. `po.__version__` is the package's and `po.core_version` the crate's. A fix to the
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
