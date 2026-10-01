# photonoxide: notes for AI coding assistants

photonoxide is pre-alpha: the plan is [ROADMAP.md](ROADMAP.md), and there is no API yet.

## Commands

```sh
cargo test                                              # unit tests
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- validate --write docs/validation.md        # rerun every validation case, rewrite the report
cargo run --release --example strip_waveguide           # an example: a published result, checked
cargo run --release --features studio -- run jobs/strip-and-ring.toml   # a job, live in the studio window
cargo run --release -- run <job.toml> --headless          # the same run without a window
cargo run --release --features studio -- view runs/<run>  # replay a finished run
```

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
