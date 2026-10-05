# photonoxide: notes for AI coding assistants

photonoxide is alpha: a library with a public API that still changes between milestones, and
the `photonoxide` program (the studio). [ROADMAP.md](ROADMAP.md) is the plan and the record of
what is done; [CHANGELOG.md](CHANGELOG.md) has what each release changed. 0.1 to 0.4 and their
patches are released, the latest 0.4.3 (direct solves at PARDISO's fill,
[docs/baselines.md](docs/baselines.md)); next is 0.5, finite-difference time-domain (FDTD). The
library's modules, each with a write-up in `docs/methods/`:

- `units`, `material` (with `material::catalogue`), `geometry`, `stack`, `raster`;
- `mode`: slabs, multilayers, planar profiles, full-vector and Hadley cross-sections, bends,
  the effective index method, Marcatili, dispersion and fields;
- `fdfd`: 2D and 3D, ports and S-parameters, adjoint gradients, direct and QMR solves;
- `circuit`: components, netlists, the circuit solve and its adjoint, objectives for genoxide;
- `compact`: vector fitting, models over parameters, Touchstone files;
- `job`, `run`: job files, run records and replay; `validation`: the report's cases; `bench`: the benchmark problems.

## Commands

```sh
cargo test                                              # unit tests
cargo clippy --all-targets --all-features -- -D warnings
cargo run --release --example strip_waveguide           # an example: a published result, checked
cargo run --release --quiet --example <name> > examples/output/<name>.txt   # its output, which CI compares
cargo test --release --test validation_report -- --ignored   # every validation case, and the report is current
```

The `photonoxide` program is the studio, a Tauri app in `studio/` (Rust in `studio/src-tauri`,
the window in Svelte 5, TypeScript, Tailwind CSS with daisyUI, and three.js in `studio/src`;
`pnpm build` there type-checks it with svelte-check and fails on warnings). Build it with the Tauri CLI, never with
plain `cargo build`, which leaves the window pointing at the dev server:

```sh
cd studio && pnpm install && pnpm tauri build           # target/release/photonoxide(.exe)
target/release/photonoxide                                # the studio: examples, job builder, runs, validation
target/release/photonoxide run jobs/strip-and-ring.toml   # a job, live in the studio window
target/release/photonoxide run jobs/strip-modes.toml      # a strip's modes and a wavelength sweep, live
target/release/photonoxide run <job.toml> --headless      # the same run without a window
target/release/photonoxide view runs/<run>                # replay a finished run
target/release/photonoxide example slab_soi               # a built-in example (--list lists them)
cargo run -p photonoxide-studio --release -- validate --write docs/validation.md   # rewrite the report
target/release/photonoxide bench --threads 1,20 --write docs/benchmarks.md   # time the benchmark problems (--all: heavy too)
```

Every example in `examples/` is built into the program (`studio/src-tauri/src/examples.rs`): a
new one needs a `pub fn main`, an entry in `examples!` and a title there; a test fails otherwise.
Every job in `jobs/` is built in too (`jobs()` there), and must pass `job::check`.

CI fails when a validation case fails or `docs/validation.md` isn't the report the code writes:
regenerate and commit it with any change that adds or alters a case.

## Rules

- **Rust only.**
  - No Python anywhere: no bindings, no helper scripts, no reference implementations.
  - No C or Fortran dependencies.
- **Minimum Rust follows the dependencies.** Use a dependency's current release; when it needs a newer Rust than `rust-version`, raise `rust-version` and the CI's MSRV job to what it needs, in the same PR, and say so in the PR.
- **Validation before features.** A solver or device isn't done without three things:
  - an analytic test;
  - a reproduction of a published result;
  - a convergence test.
  Adjoint gradients are checked against finite differences.
- **Examples are published results.** Each file in `examples/` reproduces one paper's numbers, checks them with `common::Checks` and fails when they disagree; CI runs them all. See [examples/README.md](examples/README.md).
- **Cite the source.** Every method's docs name the paper it implements, by DOI. Check a DOI before citing it.
- **No GPL code.** Meep, MPB, KLayout and SPINS-B may be run as external programs for comparison. Never read them to port code. photonoxide is MIT OR Apache-2.0.
- **Material data is CC0 or our own, with provenance:** source, validity range and temperature.
- **Never quote a number without its grid,** and never quote a 2D number as a device's performance.
- **Optimizations run in the studio:**
  - the window takes the CLI job, starts by itself, shows the run live and exits when done;
  - a hard timeout is always set;
  - headless only for batches.
- **Optimizers come from genoxide.** photonoxide supplies objectives, gradients and parametrizations. A missing method is added to genoxide as a general method, not written here and not made photonics-specific.
- **Commits:** no AI attribution lines (no 🤖 footer, no Co-Authored-By trailer).

## From scratch

Everything is written from scratch to the roadmap; no code is carried over from earlier projects. The roadmap's "Pitfalls ruled out by design" table lists known mistakes, and each one must have a test here.
