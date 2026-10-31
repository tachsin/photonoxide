# photonoxide: notes for AI coding assistants

photonoxide is released milestone by milestone: a library whose public API can change between milestones until 1.0, and
the `photonoxide` program (the studio). [ROADMAP.md](ROADMAP.md) is the plan and the record of
what is done; [CHANGELOG.md](CHANGELOG.md) has what each release changed. 0.1 to 0.6 and their
patches are released, the latest 0.6.0 (thermal and electro-optic devices: heat, thermo-optic
phase shifters as devices, Pockels modulation, travelling-wave electrodes, phase-change materials;
and 0.5.5's job editor and exports). 0.6's leftovers (#543) are on main since: the TFLN modulator
and the phase-change phase shifter as devices, the studio's temperature and field views and its
sweeps, GDS read into jobs. Two published results aren't reproduced yet and stay open, Zhang 2022's
bandwidths (#418) and He 2019's Vπ·L (#420), with the other gaps against papers in #595. 0.6.1,
nonlinear integrated optics, is under way on main: phase matching and the coupled-amplitude
equations (#621), the nonlinear data (#629), the same from a waveguide's modes (#630), and the
NLSE, four-wave mixing and the Lugiato–Lefever equation (#632), and its job kind and studio view
(#637); the published results it doesn't reproduce yet are left (#595). 0.6.3's eigenmode expansion and mode
tools are under way (#432), and the fabrication engine of 0.10 (#622) has its flows and rules on
main (F1 to F4), with F6 and F7 in progress. The library stays one crate: its modules sit in layers that a test keeps (#508). The
library's modules, each with a write-up in `docs/methods/`:

- `units`, `material` (with `material::catalogue`), `geometry` (the kernel: regions, transforms,
  polygons, a spatial index), `expr` (expressions with units and parameters), `stack`, `raster`,
  `averaging` (a grid cell's permittivity from the shapes' fill, for FDFD and the rasters);
- `fabrication`: the process flow that would make a design, each step with its reason,
  alternatives and cited capabilities, the rules that choose them, the executor and its round
  trip back to the design, and how accurately each dimension is made; a job's
  `kind = "fabrication"`;
- `mode`: slabs, multilayers, planar profiles, full-vector and Hadley cross-sections, bends,
  the effective index method, Marcatili, dispersion and fields;
- `fdfd`: 2D and 3D, ports and S-parameters, adjoint gradients, direct and QMR solves;
- `fdtd`: the Yee scheme in 2D and 3D, the convolutional PML, smoothing, sources, monitors,
  dispersive media, adjoint gradients, and the GPU (`fdtd::gpu`, wgpu compute behind the `gpu`
  feature);
- `circuit`: components, netlists, the circuit solve and its adjoint, objectives for genoxide;
- `device`: components that know their geometry (a stack, parameters as expressions, shapes
  with ports, models from a fit to 3D): the strip waveguide, bend, directional coupler and rings,
  the rib, the MMIs, the Y-branch and the MZIs; the thermo-optic and phase-change phase shifters
  and the TFLN Mach–Zehnder modulator;
- `compact`: vector fitting, models over parameters, Touchstone files;
- `eme`: eigenmode expansion, 2D for slab stacks; `sweep`: adaptive sweeps by AAA with an error
  estimate, modes followed by their overlaps;
- `nonlinear`: phase matching (`phase`), the coupled-amplitude equations (`coupled`), photon
  pairs (`pairs`), Miller's rule (`miller`), all of them in waveguides from the modes (`guide`),
  the nonlinear Schrödinger equation (`pulse`), four-wave mixing (`fwm`) and the Lugiato–Lefever
  equation (`resonator`); a job's `kind = "nonlinear"`, which the studio's Materials page edits;
- `thermal`, `electro`: the static solver on a cross-section (heat conduction, electrostatics,
  current continuity and its Joule heat) on graded grids, over a core inside the crate
  (`statics`) both re-export; a job's `kind = "static"`;
- `backend`: the solver backends, photonoxide's own and the external libraries' (through
  `photonoxide-native`);
- `job`, `run`: job files (a device computed at a fidelity or by FDTD: `job::from_device`), run
  records and replay; `validation`: the report's cases; `bench`: the benchmark problems;
  `convergence`: the gate a grid solve's reported number passes (its grid study, a modes job's
  `[task.convergence]`, results.json's `convergence`, the examples' `Checks::converged`), and
  `ToTolerance`, a solve refined until it meets a stated tolerance.
- `facade`: what the bindings wrap, plain types only: materials, slab and full-vector modes, jobs
  and their runs, an FDFD job's S-parameters, circuits from a netlist given as data, Touchstone
  files; and `facade::conformance()`, calls with their results for the bindings' tests. A
  change elsewhere in the API that touches it is absorbed in it, in the same PR.

photonoxide is one crate, and its modules sit in layers (#508): production code imports only
downward: [docs/architecture.md](docs/architecture.md) has the layers, `src/layering.rs` the test
that keeps them (a new module goes in its table). The checks and tests that reach above their
module, such as a geometry check that builds an FDTD structure, are in `src/cases/<module>/`, the
module's own path, and are declared by the module with `#[path]`.

## Commands

The [justfile](justfile) has these as recipes (`just` lists them), the same on Linux, macOS and
Windows. uv is the Python tool, not bare pip.

```sh
cargo test                                              # unit tests
cargo clippy --all-targets --all-features -- -D warnings
cargo run --release --example strip_waveguide           # an example: a published result, checked
cargo run --release --quiet --example <name> > examples/output/<name>.txt   # its output, which CI compares
cargo test --release --test validation_report -- --ignored   # every validation case, and the report is current
cargo run -p photonoxide-native --example libraries      # the external libraries found here (CUDA, cuSPARSE, cuDSS)
cargo run -p photonoxide-native --release --example validate -- mumps   # the validation cases on a backend, beside photonoxide's own
cargo test --release --features gpu --lib fdtd::gpu      # FDTD on the GPU against the CPU (skips without a GPU)
cargo test --release --features gpu --test gpu_report -- --ignored   # docs/validation-gpu.md, before each release
cargo clippy -p photonoxide-python --all-targets -- -D warnings   # the Python package's Rust (not in the default members)
just python-test                                       # the Python package: python/.venv by uv, maturin develop, pytest
PHOTONOXIDE_VALIDATION_CASES="circuit/ components/" cargo test --release --test validation_report -- --ignored   # some cases' rows
bash .github/full-suite.sh                              # the full suite here, as the release PR runs it (--fast: no release build)
bash .github/codebase-report.sh                         # the codebase report: size by area and the download against the last release, as CI posts it on the release PR
just codebase-report-crates                             # the same with the program's code by crate (cargo-bloat; builds the release binary)
cargo machete                                           # dependencies nothing uses (a Lint step in CI and the hooks)
git diff --name-only origin/main...HEAD | AREAS_RANGE=origin/main...HEAD bash .github/areas.sh   # what CI runs for a branch
cargo semver-checks check-release -p photonoxide --baseline-rev origin/main   # a branch's own breaks (CI's Semver job on a PR)
cargo semver-checks check-release -p photonoxide        # the public API against the latest release (the release PR's check)
```

CI runs in tiers (issue #315). **Tier 1**, every PR and push to main: on Linux, lint, MSRV, and the
unit tests, validation cases and examples (not the 3D devices) of the areas a change touches and
of the areas that use them, as the area map [.github/areas.sh](.github/areas.sh) has them; a file
in no area (the library's root, units, geometry, the manifests) runs everything, and files nothing
reads (most Markdown, the site, the other workflows) run only the docs' links. A new area, module
or test-read file gets its line in the map. A PR that changes the library (`src/`, the root
`Cargo.toml`) also runs the Semver job, cargo-semver-checks against the PR's base on main, so a
PR answers for its own breaks only; the release PR is checked against the latest release on
crates.io.
**Tier 2**, the release PR, a PR labelled `full-ci`, and
a change to `ci.yml`, `Cargo.toml` or `Cargo.lock`: every test on Linux, Windows and macOS (and on
one thread), every example on the three systems, the 3D devices, the whole validation report. **Tier
3**, the Weekly workflow on Mondays when code changed that week: Tier 2, every Python wheel, the
Systems comparison and the Libraries workflow's installs (those every week); a failure opens the
issue "Weekly run failed". Run the full suite here after every merge or rebase
(`.github/full-suite.sh`; `git config core.hooksPath .github/hooks` runs it in the background).
The same hooks run CI's Lint steps (`.github/lints.sh`: fmt, the clippies, docs) before every push
that changes Rust code or a manifest, on the commit being pushed, and refuse the push when one
fails. A lint that passed on a commit, in the hook or the full suite, isn't run again on it.

The Python package ([docs/python.md](docs/python.md)) wraps `photonoxide::facade` only. A change to
the façade's functions or results changes `python/` (the native module, the Python functions
and their stubs) in the same PR; its tests compare each function with the
façade's conformance cases to the bit. A function added to the façade gets its row in
[docs/features.md](docs/features.md), the table of what works in Rust and Python (a test
checks every façade function is named there). Every other language runs the `photonoxide`
program on a job file and reads its run's folder ([docs/languages.md](docs/languages.md)): no
other language gets a package of its own. CI lints the binding crate on every code change, and
builds and tests the wheels (`.github/workflows/python-wheels.yml`) only on PRs that touch
`python/`, the façade, `jobs/`, `docs/python.md`, the method write-ups with Python calls (which
`python/tests/test_methods.py` runs) or the manifests, and weekly.

The `photonoxide` program is the studio, a Tauri app in `studio/` (Rust in `studio/src-tauri`,
the window in Svelte 5, TypeScript, Tailwind CSS with daisyUI, and three.js in `studio/src`;
`pnpm build` there lints it with oxlint, type-checks it with svelte-check, and fails on warnings). Build it with the Tauri CLI, never with
plain `cargo build`, which leaves the window pointing at the dev server:

```sh
cd studio && pnpm install && pnpm tauri build           # target/release/photonoxide(.exe)
target/release/photonoxide                                # the studio: examples, components, runs, validation
target/release/photonoxide run jobs/strip-and-ring.yaml   # a job, live in the studio window
target/release/photonoxide run jobs/strip-modes.yaml      # a strip's modes and a wavelength sweep, live
target/release/photonoxide run <job.toml> --headless      # the same run without a window
target/release/photonoxide new modes                      # a job to start from, in YAML (--list: every kind)
target/release/photonoxide schema --write docs/job.schema.json   # the job file's JSON Schema, which a test checks is current
target/release/photonoxide schema --results --write docs/results.schema.json   # results.json's, the same
target/release/photonoxide view runs/<run>                # replay a finished run
target/release/photonoxide example slab_soi               # a built-in example (--list lists them)
cargo run -p photonoxide-studio --release -- validate --write docs/validation.md   # rewrite the report
target/release/photonoxide bench --threads 1,20 --write docs/benchmarks.md   # time the benchmark problems (--all: heavy too)
```

Every example in `examples/` is built into the program (`studio/src-tauri/src/examples.rs`): a
new one needs a `pub fn main`, an entry in `examples!` and a title there; a test fails otherwise.
Every job in `jobs/` is built in too (`jobs()` there), and must pass `job::check`.

CI fails when a validation case fails or `docs/validation.md` isn't the report the code writes:
regenerate and commit it with any change that adds or alters a case. A PR runs its areas' cases and
checks their rows (and the report's head and list of cases); the release PR checks every row.

## Rules

- **Rust only, but for the Python bindings** ([docs/plans/bindings.md](docs/plans/bindings.md), #271).
  - Python only in `python/`: the bindings (the `photonoxide` package for Python) and their
    tests. No Python helper scripts, no reference implementations,
    nothing else in Python, anywhere else.
  - The library stays Rust only. The bindings wrap its façade, a small module of plain types, not
    the rest of the API.
  - The binding crate writes no `unsafe` itself, and keeps `#![forbid(unsafe_code)]`: PyO3 needs
    `unsafe`, in PyO3 and in what its macros expand to, and that is all the `unsafe` there is.
  - No C or Fortran dependencies: nothing linked at build time, and the library keeps
    `#![forbid(unsafe_code)]`.
- **External libraries are optional backends, loaded at run time** ([docs/plans/backends.md](docs/plans/backends.md)).
  - Only through `photonoxide-native`, the one crate allowed `unsafe`. Never linked at build time, never redistributed, never GPL.
  - photonoxide's own solvers stay the default and the reference. A backend is checked against them before it is offered, and its speed counts only at equal accuracy.
  - The default build, CI and the released program work with no external library. Tests that need one skip when it isn't found, and say so.
  - A backend declares whether it gives the same bits on every run. photonoxide's own solvers must, on any thread count.
  - photonoxide's own GPU kernels (wgpu, the `gpu` feature) repeat bit for bit on the same device and driver: no floating-point atomics, fixed workgroup sizes, every sum taken in a fixed order. Against the CPU they agree to a tolerance stated per quantity (ROADMAP's principle 9). f32 on the GPU, f64 (Vulkan) for checking.
  - GitHub's runners have no GPU: the GPU's tests skip there and say so, and run on tachsin's machine before each release, which writes docs/validation-gpu.md. docs/validation.md must be the same with or without a GPU.
- **Minimum Rust follows the dependencies.** Use a dependency's current release; when it needs a newer Rust than `rust-version`, raise `rust-version` and the CI's MSRV job to what it needs, in the same PR, and say so in the PR.
- **Validation before features.** A solver or device isn't done without three things:
  - an analytic test;
  - a reproduction of a published result;
  - a convergence test.
  Adjoint gradients are checked against finite differences.
- **Examples are published results.** Each file in `examples/` reproduces one paper's numbers, checks them with `common::Checks` and fails when they disagree; CI runs them (a PR those of its areas, the release PR all of them). See [examples/README.md](examples/README.md).
- **Cite the source.** Every method's docs name the paper it implements, by DOI. Check a DOI before citing it.
- **No GPL code.** Meep, MPB, KLayout and SPINS-B may be run as external programs for comparison. Never read them to port code. photonoxide is MIT OR Apache-2.0.
- **Material data is CC0 or our own, with provenance:** source, validity range and temperature.
- **Never quote a number without its grid,** and never quote a 2D number as a device's performance.
- **Optimizations run in the studio:**
  - the window takes the CLI job, starts by itself, shows the run live and exits when done;
  - a hard timeout is always set;
  - headless only for batches.
- **Optimizers come from genoxide.** photonoxide supplies objectives, gradients and parametrizations. A missing method is added to genoxide as a general method, not written here and not made photonics-specific.
- **A public API change that breaks 0.x semver needs `!` in the PR title** (or the `breaking`
  label); CI's Semver job enforces it, since release-plz then raises the minor version. Keep
  0.x patches compatible where it costs little: a new field on a public struct that users build
  with a literal breaks them; put new settings in a type of their own, `#[non_exhaustive]`, taken
  by a new method.
- **Commits:** no AI attribution lines (no 🤖 footer, no Co-Authored-By trailer), in any commit of a PR or in its description: a squash merge copies every commit's message into main's. CI's Attribution check fails a PR that has one, and `git config core.hooksPath .github/hooks` strips them from new commits.

## From scratch

Everything is written from scratch to the roadmap; no code is carried over from earlier projects. The roadmap's "Pitfalls ruled out by design" table lists known mistakes, and each one must have a test here.
