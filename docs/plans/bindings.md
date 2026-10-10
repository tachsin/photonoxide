# Python and MATLAB bindings: a plan

*A plan, 2026-10-09. Nothing here is implemented or on the roadmap: I decide first. The
decisions are tracked in #271.*

I want Python and MATLAB users to be able to use photonoxide some day. Today the rules
say otherwise: AGENTS.md has "No Python anywhere: no bindings, no helper scripts, no reference
implementations", ROADMAP.md's principle 6 has "no Python bindings", and its "Not planned" list
has "Python bindings: photonoxide is Rust only". This page asks what bindings would take, from the
code as it is on main (0.5.0), and proposes a way that changes those rules as little as possible.
It has six parts:

1. [What's worth binding](#1-whats-worth-binding): the entry points a Python or MATLAB user would
   call first, and what stays internal.
2. [What makes binding hard here](#2-what-makes-binding-hard-here): closures, traits, borrowed
   views, units, arrays, long runs, errors and determinism, each with where it is in the code.
3. [Three layers, cheapest first](#3-three-layers-cheapest-first): no binding at all (job files
   and run records), Python, and MATLAB.
4. [Phases](#4-phases): what each delivers, how it is validated, and what it costs to keep.
5. [Decisions](#5-decisions).
6. [Facts checked](#6-facts-checked): versions, names and licences, with dates.

**Rules this page follows.**

- File and line references are to main at 00a11a8 (2026-10-09). Lines move; the names don't.
- Versions, names and licences were checked on 2026-10-09 (section 6).
- Other projects are described by what they do. Nothing is copied from another project's
  bindings.
- The page adds no Python, C or MATLAB code to the repository: its snippets show shapes of APIs,
  not code to run.

## Summary

1. **Any language can use photonoxide today, without a binding,** through a job file (TOML, JSON
   or YAML) and `photonoxide run job.toml --headless`, which writes a run record of JSON lines.
   The record was made for the studio's views, though: fields are stored as `f32`, normalized to a
   peak of 1, and FDTD frames as 8-bit values. A small, Rust-only step (phase 0) makes this route
   useful for scripts: a versioned `results.json` with the run's numbers at full precision and
   their grids, the error recorded when a run fails, and a timeout on the command line.
2. **Bind a small façade, not the whole API.** The library has about 670 public functions and
   220 public types (grep counts, section 1), and until 1.0 any of them can change between
   milestones. A façade module, pure Rust and in this crate, with owned values, plain numbers named
   with their units, arrays with their shapes and grids, geometry as data, and a timeout on every
   long call, is what the bindings wrap one for one. API changes are absorbed in the façade, in
   the same pull request, and checked by its tests here; the bindings change only when the façade
   does.
3. **Python by PyO3 and maturin,** with abi3 wheels for CPython 3.10 and later on Linux, macOS and
   Windows, published to PyPI by trusted publishing. genoxide already does exactly this (its
   `python/` crate, section 6), so the tooling and the release workflow are known.
4. **MATLAB through the Python package,** with thin `.m` wrappers: MATLAB R2022b or later calls
   it through its Python interface (`py.`). No C interface and no MEX gateway: both need a second
   hand-written API and `unsafe` code, and MEX needs MATLAB to build.
5. **Where the code lives is the first decision.** Recommended: the bindings in repositories of
   their own (`photonoxide-python`, with the MATLAB wrappers inside it), depending on the
   published crate; this repository stays Rust only and gains the façade and one sentence in
   AGENTS.md. The alternative, a workspace crate here as genoxide has, keeps versions in step for
   free but brings Python into this repository and wheel builds into its CI. Both are weighed in
   [decision 1](#5-decisions).

## 1. What's worth binding

### The entry points a user would call first

In the order a Python or MATLAB user would meet them:

| Task | Entry point | Where |
|---|---|---|
| Lengths and wavelengths | `Length::um`, `Length::nm` (infallible); `Wavelength::um`, `Wavelength::nm` (fallible: positive and finite) | `src/units.rs:46`, `:51`, `:149`, `:165` |
| A material's index | `silicon()`, `silica()`; `Material::refractive_index(Wavelength) -> Result<Complex64>`, `group_index` | `src/material/mod.rs:577`, `:646`, `:513` |
| The catalogue | `catalogue()`, `entry(id)` (by id, e.g. `"linbo3"`); `IndexModel::materials(Conditions)`, one material per axis | `src/material/catalogue/mod.rs:393`, `:398`, `:220` |
| refractiveindex.info files | `from_refractiveindex_info(name, yaml, source)` | `src/material/refractiveindex.rs:241` |
| Slab modes | `Slab::new(below, core, above, thickness)`, `modes(polarization, wavelength)` | `src/mode/slab.rs:44`, `:94` |
| Full-vector modes | `CrossSection::new(x, y, cells)`; `vector::modes(cs, wavelength, count, near)`; `hadley::modes` | `src/mode/vector.rs:189`, `:1008`; `src/mode/hadley.rs:733` |
| A mode's fields | `VectorMode::effective_index`, `fields(&CrossSection) -> Fields`; `Fields::e(i, j)` | `src/mode/vector.rs:861`, `:893`; `src/mode/fields.rs:53` |
| 2D FDFD | `Solver2d::new(grid, polarization, wavelength, eps, boundaries)`, `from_cells`, `solve`, `Field2d::at` | `src/fdfd/mod.rs:268`, `:394`, `:601`, `:786` |
| S-parameters | `Solver2d::port_modes`, `s_matrix(&[Port]) -> Vec<Vec<c64>>`; 3D: `Solver3d::s_matrix` | `src/fdfd/ports.rs:164`, `:372`; `src/fdfd/three.rs:927` |
| Adjoint gradients | `mode_power_gradient(field, mode, direction) -> (f64, Vec<f64>)` | `src/fdfd/adjoint.rs:50` |
| FDTD | `Simulation::new(grid, eps, boundaries, courant)`, sources, monitors, `run`, `run_until` | `src/fdtd.rs:475`, `:1042`, `:1061` |
| Circuits | `Netlist::new`, `add(name, Arc<dyn Component>)`, `connect`; `compile`, `Circuit::spectrum` | `src/circuit/netlist.rs:281`, `:343`; `src/circuit/solve.rs:95`, `:251` |
| Circuit gradients | `Circuit::gradient(wavelengths, values, objective)` | `src/circuit/adjoint.rs:117` |
| Touchstone | `Touchstone::read(path)`, `parse(text, ports)`, `write_file(path, precision)` | `src/compact/touchstone.rs:313`, `:341`, `:326` |
| Compact models | `vector_fit(s, responses, options)`; `CompactModel::fit` | `src/compact/fit.rs:449` |
| Jobs | `run::Job::load`, `job::check`, `job::execute(job, run, stop)`; `job::fdfd_s_parameters(job, wavelengths_um)` | `src/job/mod.rs:1605`, `:1564`; `src/job/fdfd.rs:454` |

The last row matters most for bindings. A job file is already a complete, closure-free
description of a structure and a simulation, in micrometres, with the check
(`job::check`) refusing what the run would refuse. A binding that accepts a job (a file, or a
Python dict or MATLAB struct turned into JSON) reaches modes, FDFD and FDTD without binding the
solvers' constructors at all.

### What stays internal

- `backend` (the solver traits and registry), `bench`, `parallel`, `validation`, `raster` and
  `expr`: they serve photonoxide's own solvers, benchmark and report. A binding needs at most the
  backend's *choice* (`backend::Choice`, `src/backend.rs:311`), as a string, as job files have it.
- `geometry` and `stack` as Rust types: a binding describes shapes and stacks as data, as job
  files do, and lets the façade build the types.
- The solvers' setters and intermediate types (preconditioners, `Stopping`, grids' fields),
  until users ask for them.

### Breadth

grep counts of `pub fn` and `pub struct`/`pub enum` per module (including some in check files):
units 12 and 3, material 26 and 29, mode 70 and 21, fdfd 82 and 23, fdtd 107 and 33, circuit 114
and 27, compact 43 and 21, backend 42 and 12, geometry 70 and 16, expr 28 and 7, run 20 and 6, job
4 and 7, stack 15 and 3, raster 4 and 1, bench 26 and 10, validation 4 and 3. About 670 functions
and 220 types in all. Binding them one for one would be a second API of that size to keep in step;
genoxide's Python crate, which binds most of genoxide, is about 450 kB of Rust in 19 files, with
19 test files.

## 2. What makes binding hard here

### Closures where a binding has data

Most constructors take the structure as a function of position:

- `Solver2d::new(..., eps: impl Fn(f64, f64) -> c64, ...)` (`src/fdfd/mod.rs:268`; `from_cells`
  at `:394` takes an array instead), `Solver3d::new` (`src/fdfd/three.rs:613`) and the iterative
  3D solver (`src/fdfd/three/iterative.rs:80`), with no array form;
- `fdtd::Simulation::new(..., eps: impl Fn(f64, f64, f64) -> f64, ...)` (`src/fdtd.rs:475`), and
  the media's `inside: impl Fn(..) -> bool` (`src/fdtd/media.rs:725`);
- `CrossSection::uniform` and `bent` (`src/mode/vector.rs:357`, `:401`); `CrossSection::new` takes
  arrays;
- the dispersion and component builders that re-solve per wavelength,
  `impl FnMut(Wavelength) -> Result<CrossSection>` (`src/mode/dispersion.rs:134` and the
  components' `from_modes`);
- the circuit adjoint's objective, `F: FnOnce(&[SMatrix]) -> (f64, Vec<SMatrix>)`
  (`src/circuit/adjoint.rs:117`).

A Python callable can stand behind a Rust closure, but it is called once per grid point (millions
of times in 3D), under Python's global interpreter lock (GIL), and the closures must often be
`Send + Sync`. The way round is to give the binding data, not functions: permittivity as an array
on the grid, or shapes on a stack as job files have them, and the ready-made circuit objectives
(`power`, `power_error`, `matrix_error`, `src/circuit/objective.rs:34`) by name.

### Traits a user would implement

`circuit::Component` (`src/circuit/mod.rs:404`) is `Debug + Send + Sync` and enters a netlist as
`Arc<dyn Component>`. A component written in Python would hold a Python object, called with the
GIL from Rust's threads (`Spectrum::of` solves wavelengths in parallel). It can be done, but a
first release needs only the built-in components and `Sampled` (a component from S-matrices at
wavelengths, which arrays describe). The backend traits (`DirectSolver`, `IterativeSolver`) are
for `photonoxide-native`, not for users.

### Borrowed views and lifetimes

User-facing types are mostly owned. The borrows are accessors: `Simulation::e(axis) -> &[f64]`
(`src/fdtd.rs:792`), `probe(n) -> &[f64]` (`:990`), `Field3d::values() -> &[c64]`, and the
`_mut` setters. A binding copies them into numpy arrays; a zero-copy view would tie the array's
life to the simulation's and is not worth it for a first release. Lifetimes in public types
(`backend::Matrix<'a>`, `src/backend.rs:65`, and the multigrid's) are in the backend API, which
isn't bound.

### Generics

Few in user-facing signatures: `run::replay<E>` (`src/run.rs:385`), `parallel::map_in_order`
(`src/parallel.rs:18`) and the circuit gradient's `F`. None is a problem: the façade fixes them.

### Units and plain numbers

`Length`, `Wavelength` and `Frequency` are newtypes over `f64` (`src/units.rs:39`, `:141`, `:213`):
ROADMAP's principle 7, "units are types". Python and MATLAB users pass floats and arrays. The job
files already solved this: every number's name carries its unit (`wavelength_um`, `step_nm`,
`size_um`), and the façade can do the same, converting to the types and returning their errors
(a wavelength that isn't positive is `Error::InvalidValue`, never a silent NaN). Keyword names
keep principle 7's intent: a length can't be passed where a wavelength goes without the name
saying so. See [decision 3](#5-decisions).

### Complex numbers and arrays

- No faer or ndarray type appears in a public signature; faer is internal. Arrays come out as
  flat `Vec<f64>` or `Vec<c64>` (`Complex64` from num-complex), as nested `Vec<Vec<c64>>`
  (FDFD S-matrices, `src/fdfd/ports.rs:372`), as a row-major `SMatrix` (`src/circuit/mod.rs:82`),
  or through per-point accessors (`Field2d::at`, `Fields::e(i, j) -> [c64; 3]`).
- The layouts differ: the 2D FDFD gradient is indexed `j * nx + i`, a mode's `Fields` `i * ny + j`
  (`src/mode/fields.rs:31`). The façade should return one convention: C order with shape
  `(ny, nx)` (`array[j, i]`), which is numpy's natural indexing and what `Raster` already uses
  (`src/raster.rs:14`), and say so once.
- numpy has `complex128`, the same layout as `Complex64`; rust-numpy converts `Vec<Complex64>`
  into a numpy array that owns the memory, without a copy. MATLAB's conversion of a numpy array
  (`double(p)`) copies it; MATLAB is column-major, so the wrappers keep numpy's shape and the
  documentation says that `a(j, i)` is the point (x_i, y_j).
- AGENTS.md: "never quote a number without its grid". Every array the façade returns comes with
  its coordinates (`x_um`, `y_um`, and `z_um` in 3D), as `Raster` carries its edges.

### Long runs: the GIL, progress, cancellation and the hard timeout

- A solve can take minutes. The binding releases the GIL while Rust works, so other Python threads
  run; photonoxide's own threads (rayon) never touch Python.
- **Cancellation exists for jobs:** `run::Stop` (`src/run.rs:421`), an `Arc<AtomicBool>` and a
  deadline, polled by the job's solvers. Direct API calls mostly have none:
  `vector::modes_until(..., stop: impl Fn() -> bool)` (`src/mode/vector.rs:1026`) is the
  exception, and FDTD's `run` and `run_until` have no hook (a binding can run steps in chunks).
  The façade should take a `Stop` (a timeout, and a request the binding sets on Ctrl+C) on every
  long call.
- **Ctrl+C in Python:** Python handles the signal only when the main thread runs Python code. The
  pattern genoxide's binding uses: the work runs on another thread, and the calling thread waits,
  checks for signals, and on `KeyboardInterrupt` asks the work to stop.
- **The hard timeout** (AGENTS.md: "a hard timeout is always set"): a job has
  `timeout_minutes`; a façade call takes `timeout_s` and has a default, so a script can't run
  forever by accident.
- **Progress:** there is no callback trait; a job's progress is its events (`FdtdProgress`,
  `SweepPoint`), written to `events.jsonl` as they happen. A binding can return the events as an
  iterator, or take an optional Python callback called from the waiting thread, never from
  rayon's.
- AGENTS.md's "optimizations run in the studio" stays: a binding exposes objectives and
  gradients; a long optimization from Python is the user's own script, and the studio stays the
  place to watch one.

### Errors

`photonoxide::Error` (`src/error.rs:11`) is a `#[non_exhaustive]` enum: `InvalidValue`,
`OutsideValidity` (with the material, the wavelength and the range), `Io`, `Parse`, `Netlist` and
`Gpu`. In Python, one exception class per variant under a base `photonoxide.Error`, each also
deriving from the built-in it resembles (`ValueError` for `InvalidValue`, `OSError` for `Io`),
with the variant's fields as attributes; an unknown future variant maps to the base class. In
MATLAB, the wrappers rethrow with identifiers such as `photonoxide:OutsideValidity`. photonoxide
reports invalid input as an `Error`, never a panic (`src/error.rs:7`); a panic that still happens
becomes Python's `PanicException`, not a crash.

### Determinism

photonoxide's own solvers give the same bits on any number of threads (`src/parallel.rs:4`;
`compact`'s fits run on one thread for this, `src/compact/fit.rs:456`). A binding calls the same
Rust code, so a call through Python gives the same bits as the same call in Rust, with the same
crate version and build settings, on the same machine. That is testable, and the tests in phase 2
check it with exact equality. Wheels are built for the platform's baseline CPU, as the program's
releases are; a user's own `-C target-cpu=native` build can differ in the last bits, as it can in
Rust.

### The `gpu` feature and the run-time backends

- `gpu` (FDTD through wgpu, `src/fdtd.rs:1472`) loads the system's driver at run time and links no
  C, so a wheel could include it. It lengthens the build and enlarges the wheel; the first wheels
  leave it out, and a later one can add it once its tests (which skip without a GPU) run in the
  wheel workflow.
- The run-time backends (PARDISO, cuDSS, the GPU Krylov solvers) live in `photonoxide-native`,
  which is `publish = false` (`native/Cargo.toml:10`). A binding outside this repository can't
  depend on it until it is published. The backends are chosen by name, as a string, so binding
  them is cheap once they are reachable; they come in a late phase.

## 3. Three layers, cheapest first

### Layer 0: no binding at all

**What exists.** `photonoxide run <job.toml|.json|.yaml> [--out <dir>] [--headless]`
(`studio/src-tauri/src/main.rs:28`) loads the job, creates a run folder
`runs/<YYYYMMDD>-<HHMMSS>-<name>/`, prints that folder's path on stdout and runs the job
(`main.rs:122`-`:162`). The folder holds:

- the job as given (`job.toml`, `job.json` or `job.yaml`);
- `meta.json`: photonoxide's version, the job's name, the start time and the threads
  (`src/run.rs:270`);
- `events.jsonl`: one JSON event per line, flushed as it is written (`src/run.rs:360`), with a
  `type` tag (`src/job/mod.rs:307`): `mode` (effective index as `[re, im]`, TE fraction, an
  intensity raster), `sweep_point`, `s_parameters` (`s[q][p]` from port p into port q, each
  `[re, im]`, `src/job/mod.rs:401`), `fdtd_spectrum`, `fdtd_resonances`, `finished`, and more.

Python reads that with `json` and a loop; MATLAB with `fileread` and `jsondecode` (R2016b and
later). Scalars are exact: serde_json writes floats so that they read back to the same bits.
`run::replay` (`src/run.rs:385`) is the Rust reader, and skips a last line cut off mid-write.

**What's missing for scripts.**

1. **Full-precision results.** The record was made for the studio's views. A `Raster`'s values
   are `f32` (`src/raster.rs:31`); a mode's intensity is normalized to a peak of 1 and its field
   components to ±1; FDTD frames are 8-bit values in base64 (`src/job/mod.rs:598`). A script
   that wants a mode's field, an FDFD field or a monitor's DFT at full precision can't get it from
   the record.
2. **A version.** Neither the job, the events nor `meta.json` has a schema version; `Event` is
   `#[non_exhaustive]` and grows at its end. A reader can't tell what it is reading except by
   photonoxide's version.
3. **The error.** When a run fails, `job::execute` returns early and records no `finished` event
   (`src/job/mod.rs:1564`); the reason is printed on stderr only (`main.rs:93`).
4. **The exit code.** A run stopped by its timeout exits 0, like one that finished; only the
   `finished` event's `stopped` says `"timeout"`.
5. **A timeout from the command line.** Only the job's `timeout_minutes` sets one; a script that
   runs someone else's job can't impose its own.
6. **A documented format.** The events are documented in rustdoc only. A page describing the run
   folder, with the units of every field, is what a Python or MATLAB user would read.

**The proposal (phase 0, Rust only, in this repository):**

- `results.json` in each run folder, written at the end: `"schema": 1`, photonoxide's version,
  and the run's numbers by kind (effective indices and group indices with the cross-section's
  grid; S-matrices per wavelength; spectra; resonances), each array with its shape, its unit and
  its grid. Arrays above a size threshold go to a raw little-endian file beside it (`f64`, or
  `complex128` as interleaved pairs), named in the JSON with its dtype, shape and order, which
  `numpy.fromfile` and MATLAB's `fread` read with no reader library. (NumPy's `.npy` format is the
  alternative: one call in Python, but MATLAB has no reader for it.)
- A `failed` event with the error's text, written before `execute` returns its error.
- `--timeout <minutes>` on `photonoxide run`, the shorter of it and the job's applying; exit codes
  documented: 0 finished, 1 failed, 2 usage, and a distinct one (e.g. 3) for stopped by its
  timeout.
- `docs/runs.md`: the folder, `meta.json`, the events and `results.json`, with units.
- The program is what a script runs, so it is what a user installs: the releases have it for
  Linux (an AppImage that runs on clusters), Windows and macOS. Nothing else is needed.

**Cost:** one or two pull requests, Rust and Markdown only. It changes no rule, and the 0.5.2
milestone's farming across processes may want the same machine-readable results.

### Layer 1: Python

**The tools.** PyO3 0.29 (MIT OR Apache-2.0) for the module, rust-numpy 0.29 (BSD-2-Clause) for
arrays, maturin 1.15 (MIT OR Apache-2.0) to build wheels. PyO3's `abi3-py310` feature builds one
wheel per platform for every CPython from 3.10; free-threaded CPython 3.14t needs a wheel of its
own (it can't load abi3), as genoxide's workflow builds. PyO3 needs `unsafe` internally, but
the binding's own code can be safe Rust: `#[pyfunction]`, `#[pyclass]` and conversions are macros.

**The shape of the API** (an illustration, not a design):

```python
import numpy as np
import photonoxide as po

si = po.material("silicon")                       # Li 1980, with its provenance
n = si.refractive_index(wavelength_um=np.linspace(1.5, 1.6, 11))   # complex128 array

slab = po.Slab(below=1.444, core=3.473, above=1.444, thickness_nm=220)
modes = slab.modes("te", wavelength_um=1.55)      # [SlabMode(order=0, effective_index=2.8448...)]

result = po.run("jobs/strip-modes.toml", timeout_minutes=10)   # or a dict: the job as data
result.modes[0].effective_index, result.modes[0].field.x_um    # numpy arrays with their grid

circuit = po.Netlist({"dc1": po.DirectionalCoupler(...), ...}, connections=[...])
spectrum = circuit.spectrum(wavelength_um=np.linspace(1.5, 1.6, 401))   # S as (nλ, n, n)
po.Touchstone.read("device.s4p").spectrum()
```

- Units in keyword names, as job files have them (decision 3).
- Vectorized over wavelengths where Rust already loops (`Circuit::spectrum`, sweeps), so a Python
  user doesn't write the loop.
- Results as small classes with numpy arrays and their grids, with type stubs (`.pyi`) for
  editors, as genoxide's package ships.
- `po.run(job)` runs a job in-process through `job::execute` (no program needed) and returns
  the same results as `results.json`, with `timeout_minutes` and Ctrl+C handled as above.

**A first release would expose:** materials and the catalogue (index, group index, ranges,
provenance); the slab and multilayer (modes, reflection); full-vector and Hadley modes from a stack
and shapes; jobs of every kind run in-process, with their results; 2D FDFD S-parameters through
jobs; circuits of the built-in components and `Sampled`, with spectra and the circuit adjoint's
gradients for the ready-made objectives; Touchstone read and write. Not in it: user-written
components, FDTD built in code, 3D adjoints, backends, the GPU.

**Tests (validation before features).** Each bound function has:

- **bit-for-bit agreement with Rust:** the façade ships conformance cases, each a call described as
  data and its result computed by Rust (`facade::conformance()`, phase 1); the binding's tests run
  each through Python and compare with `==`, not a tolerance;
- **the validation report's numbers:** the cases the binding reaches (the slab's 2.84482 against
  Chrostowski and Hochberg's 2.845, the EIM strip, a ring's free spectral range) recomputed
  through Python, against the report's references and tolerances;
- **two examples in Python,** each reproducing its paper's numbers and failing when they
  disagree, as `examples/` does in Rust.

**Wheels and PyPI.** A workflow like genoxide's `python-wheels.yml`: Linux x86_64 and aarch64
(manylinux and musllinux), macOS arm64 and x86_64, Windows x64; tests on the lowest supported
Python; an sdist; PyPI by trusted publishing (no stored token), triggered by a release. Each wheel
is a release build of photonoxide (not measured here; the library's release build is longer than
genoxide's, with faer and the solvers).

**Docs.** A getting-started page in Python, the API reference from the stubs, and each function's
page linking the Rust item and its method write-up in `docs/methods/`, where the paper is cited.

### Layer 2: MATLAB

Three routes:

| | Through the Python package (`py.`) | A C interface (cdylib) with `loadlibrary`/`calllib` | A MEX gateway |
|---|---|---|---|
| What's written | `.m` wrappers that call the Python package and turn results into MATLAB arrays and structs | a C ABI over the façade (`extern "C"`, raw pointers, `unsafe`), a header (cbindgen), `.m` wrappers | a `mexFunction` (C or C++, or Rust linked against MATLAB's `libmx`/`libmex`) and `.m` wrappers |
| Built with | nothing new: the Python wheels | a cdylib per platform | MATLAB's headers and libraries, per platform; MATLAB needed to build |
| The user needs | MATLAB R2022b or later (the first release that supports Python 3.10), Python, `pip install photonoxide` | a supported C compiler and Perl, which `loadlibrary` requires to parse the header; MathWorks recommends a published C/C++ interface instead since R2022a | the MEX file for their platform and MATLAB release |
| `unsafe` / C | none beyond PyO3's | a C ABI crate with `unsafe` | C or C++ code, or Rust linking MATLAB's C libraries at build time |
| Complex arrays | MATLAB converts complex arrays to numpy `complex128` and back with `double(...)`, copying | pointers to interleaved complex data | MATLAB's interleaved complex API (R2018a and later) |
| Long runs | the Python package's timeout and stop; MATLAB's Ctrl+C reaches Python only partly (to check) | the C ABI must expose stop and timeout | the MEX gateway must expose stop and timeout too |
| Cost to keep | the `.m` layer only | a second hand-written API, kept in step with the façade | the same, plus MATLAB's versions |

**Recommended: through the Python package.** It reuses everything layer 1 builds and tests, needs
no compiler on the user's machine, and its MATLAB part is thin. The `.m` files can ship inside the
Python wheel (as package data), with `photonoxide.matlab_path()` returning their folder for
MATLAB's `addpath`: one install, one version, and nothing built for MATLAB. MATLAB's
`pyenv(ExecutionMode="OutOfProcess")` runs Python in a separate process, which also keeps
MATLAB's own libraries and photonoxide's apart.

**Licences.** MATLAB is proprietary; the wrappers are photonoxide's own code, MIT OR Apache-2.0,
and call only documented MATLAB functions. Nothing of MATLAB is redistributed.

**Testing without a MATLAB licence.** This is the weak point, and it should be said plainly in the
MATLAB page:

- MathWorks' `setup-matlab` GitHub action licenses MATLAB automatically for public projects, but
  its README says that public projects and batch licensing "do not support external language
  interfaces, including MATLAB Engine APIs for Python". MathWorks lists calling Python from MATLAB
  among its external language interfaces, so the `py.` route most likely can't be tested on
  GitHub's runners (to confirm with MathWorks). A self-hosted runner with a regular licence can.
- GNU Octave has no `py.` interface of its own; the Pythonic package (GPL-3.0, early, version 0.0.x)
  adds one, aiming at MATLAB's syntax. Run as an external program in CI, as AGENTS.md allows for
  GPL programs, it could smoke-test the wrappers where it is compatible; whether it handles numpy's
  complex arrays is to check. Octave can't stand in for MATLAB.
- So: the logic lives in Python and is tested there; the `.m` wrappers are kept thin enough to
  review by eye; Octave smoke-tests them where it can; and someone with a MATLAB licence runs the
  MATLAB tests by hand before each release, and the docs list the MATLAB releases checked. If the
  I have no licence, MATLAB support is "untested in MATLAB" until a contributor with one checks
  it, and the docs say so.

## 4. Phases

Each phase is done when its validation is, as AGENTS.md asks of a solver.

### Phase 0: results for scripts (this repository, Rust only)

- **Delivers:** `results.json` (schema 1) and its array files; the `failed` event; `--timeout` and
  the exit codes; `docs/runs.md`.
- **Validated by:** a test runs each job in `jobs/` and checks that `results.json` holds the same
  bits as the library's direct calls (`job::fdfd_s_parameters` for the S-matrices, the mode
  solver for the indices); the strip's and the slab's numbers against the validation report's.
- **Costs:** small; the schema becomes a promise, so changes to it bump `schema`.

### Phase 1: the façade (this repository, Rust only)

- **Delivers:** a module (working name `photonoxide::facade`) with owned values only: no
  lifetimes, generics, closures or trait objects in its signatures; numbers as `f64` named with
  their units; arrays with their shape and grid, in one layout (C order, `(ny, nx)`); geometry as
  data, reusing the job files' shapes and stacks; a timeout and a stop on every long call; results
  that serialize to `results.json`'s schema. And `facade::conformance()`: calls as data with their
  results, for the bindings' tests.
- **Validated by:** the façade's tests against the underlying calls, bit for bit; the conformance
  cases regenerated and compared in CI, as `docs/validation.md` is.
- **Costs:** a module to keep in step with the rest of the library, in the same pull requests. In
  return, an internal change (a renamed constructor, a new builder) stops at the façade.
  cargo-semver-checks, which release-plz already runs, reports the façade's breaking changes, and
  CHANGELOG.md gets a "Façade" line when it changes.

### Phase 2: Python 0.x (separate repository, by default)

- **Delivers:** the package of layer 1's first release, wheels for the platforms above, PyPI.
- **Validated by:** the conformance cases bit for bit, the validation numbers it reaches, two
  examples reproducing their papers, all in the wheel workflow on Linux, macOS and Windows.
- **Costs:** a release per photonoxide release that changes the façade; the wheel matrix (about
  seven builds per release, as genoxide's); a scheduled job building against photonoxide's main
  branch, so a façade change is noticed before it is released.

### Phase 3: MATLAB (with the Python package)

- **Delivers:** `.m` wrappers for phase 2's functions, results as MATLAB arrays and structs,
  errors with `photonoxide:` identifiers, a MATLAB getting-started page.
- **Validated by:** the Python tests (the logic), an Octave smoke test where Pythonic works, and a
  manual run in MATLAB before each release, recorded with the MATLAB release used.
- **Costs:** small while the wrappers stay thin; the manual check is the recurring cost.

### Phase 4: later, each when asked for

- FDTD built in code (structures as data, sources, monitors, run in chunks with a stop).
- Adjoint gradients (FDFD 2D and 3D, FDTD 3D, circuits with user objectives as arrays), for
  optimizers in Python, where genoxide's own Python package (on PyPI) supplies the methods:
  AGENTS.md's "optimizers come from genoxide" holds on the Python side too.
- Components written in Python (the `Component` trait behind a Python object, with the GIL).
- The run-time backends, once `photonoxide-native` is published; the `gpu` feature in the wheels.
- conda-forge, once PyPI's package is stable.

### What it costs to keep, and how to keep it cheap

Every milestone until 1.0 may change the API. What keeps the bindings cheap:

- **They bind the façade only,** which is small (tens of functions, not hundreds) and changes
  deliberately. The library's churn stops there.
- **The conformance cases travel with the crate,** so a binding finds out what changed by running
  them, not by reading the diff.
- **Data over functions:** structures as job-file data, objectives by name, arrays in and out.
  Data formats grow by adding fields; function signatures break.
- **Not every release needs a binding release:** only those that change the façade or that users
  want for a new feature.

## 5. Decisions

1. **AGENTS.md's rule and where the bindings live.** AGENTS.md says "No Python anywhere: no
   bindings, no helper scripts, no reference implementations", and the library keeps
   `#![forbid(unsafe_code)]`. The options:
   - **(a) Separate repositories** (`tachsin/photonoxide-python`, the MATLAB wrappers inside it or
     in `tachsin/photonoxide-matlab`), depending on the published crate. This repository stays
     Rust only; AGENTS.md keeps its rule and gains one sentence: "Bindings to other languages are
     separate projects in their own repositories; this repository stays Rust only." ROADMAP's
     principle 6 and its "Not planned" line change to point at them. *For:* the rule holds here,
     this repository's CI doesn't build wheels, the bindings release at their own pace. *Against:*
     the bindings follow released versions only and lag them; a breaking change here is found when
     the binding upgrades (the scheduled build against main narrows that); two release processes;
     `photonoxide-native` must be published before a binding can offer backends.
   - **(b) A workspace crate here** (`python/`), as genoxide has: the binding shares the version
     and `Cargo.lock`, release-plz bumps it, and a pull request that breaks it fails its own CI.
     *For:* versions in step for free, breakage caught where it is made, a template that already
     works in genoxide. *Against:* AGENTS.md's rule becomes "no Python outside `python/`"; Python
     files, a `pyproject.toml` and pytest tests enter the repository; since nearly every pull
     request touches `src/`, the wheel builds would run on most of them (genoxide runs its wheel
     workflow on changes to `src/**` too), on top of a CI that already builds the 3D examples on
     runners of their own; every API change in a milestone must also update the binding in the
     same pull request.
   - **(c) (a), plus the façade here** (phases 0 and 1 in this repository, pure Rust). The façade
     and its conformance cases give most of (b)'s in-step checking without Python here: the
     library's changes are absorbed in the façade in the same pull request, and the binding
     changes only when the façade does.

   *Recommended: (c).* photonoxide's rule is stricter than genoxide's, its CI heavier, and its API
   broader and changing faster; the façade is where the cost of keeping bindings belongs, and it
   is Rust.

2. **Which layer first.** (a) Phase 0 only, and wait for users to ask for more; (b) phases 0 and 1
   now, Python when there is time; (c) straight to Python. *Recommended: (b), phase 0 first.* Phase
   0 helps every language and the studio's own runs at once, needs no rule change, and the façade
   is what makes the bindings cheap later. When: after 0.5.2, so the 0.5.2 work on farming across
   processes can share `results.json`.

3. **The Python API's style.**
   - Units: (a) keyword names with the unit (`wavelength_um=1.55`, `thickness_nm=220`), as job
     files have them; (b) unit objects mirroring Rust (`po.Wavelength.um(1.55)`); (c) accepting a
     units library's quantities. *Recommended: (a),* the same names in a job file, in Python and in
     MATLAB's name-value arguments, and arrays of wavelengths without wrapping each one.
   - Shape: (a) Pythonic functions and small result classes over the façade, vectorized over
     wavelength; (b) Rust's types mirrored one for one. *Recommended: (a).*
   - Arrays: C order, `array[j, i]` at (x_i, y_j), always with their coordinates. *Recommended:
     yes.*

4. **The MATLAB route.** (a) Through the Python package, `.m` files shipped in the wheel; (b) a C
   ABI with `loadlibrary`; (c) a MEX gateway. *Recommended: (a).* And: is MATLAB support stated as
   "checked by hand in MATLAB release X before each release" (needs someone with a licence) or as
   "tested in Octave only, untested in MATLAB" until one is found? *Recommended:* say which, on
   the MATLAB page; I say whether a licence is at hand.

5. **Versioning.** (a) The package's version is the crate's it wraps (`photonoxide` 0.5.1 on PyPI
   wraps the crate 0.5.1), with binding-only fixes as PEP 440 post-releases (`0.5.1.post1`), and
   `po.__version__` beside `po.core_version`; (b) the binding's own version, with the crate's in
   its metadata; (c) only the minor follows the crate. photonoxide's patch versions are milestones
   (0.4.1 to 0.4.3, 0.5.1), so (c) would hide real changes. *Recommended: (a),* with the binding's
   `Cargo.toml` pinning the crate exactly (`photonoxide = "=0.5.1"`), and not every crate release
   followed by a binding release.

6. **Names.** `photonoxide` is free on PyPI (the JSON API and the simple index both answer 404,
   2026-10-09) and on conda-forge (404 at anaconda.org's API); the GitHub repositories
   `tachsin/photonoxide-python` and `tachsin/photonoxide-matlab` don't exist. The options: (a)
   `photonoxide` on PyPI, imported as `photonoxide`, as genoxide is `genoxide` on both; (b) a
   different PyPI name (`photonoxide-py`). PyPI discourages registering a name with an empty
   placeholder, so the name is taken by the first real release. *Recommended: (a),* the
   repository `tachsin/photonoxide-python`, MATLAB's wrappers inside it, and a File Exchange entry
   pointing at it once MATLAB support is checked.

7. **The README's line.** README.md says "There are no Python bindings." *Recommended:* keep it
   true until a binding is released, then link the binding from the README's install section.

## 6. Facts checked

On 2026-10-09:

- **PyPI:** `https://pypi.org/pypi/photonoxide/json` and `https://pypi.org/simple/photonoxide/`
  answer 404: the name is free. `genoxide` 0.13.1 is on PyPI, owned by tachsin.
- **conda-forge:** `https://api.anaconda.org/package/conda-forge/photonoxide` answers 404.
- **GitHub:** `tachsin/photonoxide-python` and `tachsin/photonoxide-matlab` answer 404.
- **PyO3** 0.29.3 (2026-09-30, MIT OR Apache-2.0, minimum Rust 1.83, below photonoxide's 1.95);
  **rust-numpy** (`numpy` on crates.io) 0.29.0 (2026-06-13, BSD-2-Clause); **maturin** 1.15.0
  (MIT OR Apache-2.0).
- **genoxide's Python package** (its repository, main): a workspace crate `python/`
  (`genoxide-python`, `publish = false`, version from the workspace), `pyo3` 0.29 with
  `abi3-py310`, `numpy` 0.29, maturin `>=1.15,<2`; 19 Rust files of about 450 kB in all, 19 test
  files; a `python-wheels.yml` workflow building seven platforms plus free-threaded 3.14t, run on
  pull requests that touch `python/**`, `src/**`, `Cargo.toml` or `Cargo.lock`; PyPI by trusted
  publishing after release-plz releases the crate.
- **MATLAB and Python** (MathWorks' compatibility table): Python 3.10 is supported from R2022b;
  R2025b supports 3.9 to 3.12, R2026a 3.9 to 3.13.
- **MATLAB's `loadlibrary`** (MathWorks' reference page): "You must have a supported C compiler
  and Perl must be available"; since R2022a MathWorks recommends publishing a MATLAB interface to
  the library instead.
- **MATLAB in GitHub Actions** (`matlab-actions/setup-matlab`'s README): public projects are
  licensed automatically, except transformation products; "public projects and batch licensing
  do not support external language interfaces, including MATLAB Engine APIs for Python".
- **Octave's Pythonic package** (`gnu-octave/octave-pythonic`): a `py.` interface for Octave,
  GPL-3.0, early (0.0.x).
