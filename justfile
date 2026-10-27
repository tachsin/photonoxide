# photonoxide's everyday commands, for just (https://just.systems): `just` lists them.
#
# The same on Linux, macOS and Windows: recipes run in sh, or in Windows PowerShell on Windows,
# and each line is one plain command. The few that run the repository's .sh scripts need bash
# (on Windows, Git Bash's). Environment variables are set through exported parameters ($NAME),
# and directories through [working-directory], so nothing depends on the shell's syntax.

set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

# list the recipes
default:
    @just --list --unsorted

# --- Build ---

# build the library and the command line, in release mode (CI's profile)
build:
    cargo build --release

# build with the dist profile: thin LTO and one codegen unit, as the Python wheels are built
build-dist:
    cargo build --profile dist

# the fastest build for this machine: fat LTO, one codegen unit, and this CPU's instruction sets
build-fastest $RUSTFLAGS="-C target-cpu=native":
    cargo build --profile fastest

# --- Check ---

# the unit tests
test:
    cargo test

# the unit tests on one thread, as CI also runs them
test-one-thread:
    cargo test -- --test-threads=1

# CI's lints: rustfmt, clippy (the library, the native crates, the Python crate) and the docs
lint: fmt-check clippy docs

# check the formatting
fmt-check:
    cargo fmt --all --check

# format everything
fmt:
    cargo fmt --all

# clippy on every crate, warnings as errors
clippy:
    cargo clippy --all-targets --all-features -- -D warnings
    cargo clippy -p photonoxide-native -p photonoxide-native-testlib --all-targets -- -D warnings
    cargo clippy -p photonoxide-python --all-targets -- -D warnings

# the API docs, warnings as errors
docs $RUSTDOCFLAGS="-D warnings":
    cargo doc --no-deps --all-features

# this branch's own breaks of the public API, against main, as CI's Semver job checks a PR
semver:
    cargo semver-checks check-release -p photonoxide --baseline-rev origin/main

# the public API against the latest release, as the release PR is checked
semver-release:
    cargo semver-checks check-release -p photonoxide

# --- Validation and examples ---

# every validation case, and that docs/validation.md is the report the code writes
validate:
    cargo test --release --test validation_report -- --ignored

# some cases only, by id prefix: just validate-cases "circuit/ components/"
validate-cases $PHOTONOXIDE_VALIDATION_CASES:
    cargo test --release --test validation_report -- --ignored

# rewrite docs/validation.md from the code
validation-report:
    cargo run -p photonoxide-studio --release -- validate --write docs/validation.md

# rewrite docs/job.schema.json, the job file's JSON Schema, from the code
schema:
    cargo run -p photonoxide-studio --release -- schema --write docs/job.schema.json

# run an example, a published result checked: just example strip_waveguide
example name:
    cargo run --release --example {{name}}

# rewrite an example's output, which CI compares with what the example prints
example-output name:
    cargo run --release --quiet --example {{name}} > examples/output/{{name}}.txt

# --- GPU and external libraries ---

# FDTD on the GPU against the CPU (skips without a GPU)
gpu:
    cargo test --release --features gpu --lib fdtd::gpu

# docs/validation-gpu.md, before each release
gpu-report:
    cargo test --release --features gpu --test gpu_report -- --ignored

# the external libraries found on this machine (CUDA, cuSPARSE, cuDSS, oneMKL, ...)
libraries:
    cargo run -p photonoxide-native --example libraries

# the validation cases on a backend, beside photonoxide's own: just validate-backend mumps
validate-backend backend:
    cargo run -p photonoxide-native --release --example validate -- {{backend}}

# --- Other codes, run here only (never in CI): docs/cross-code.md ---

# openEMS against the quasi-TEM line, where openEMS is installed (PHOTONOXIDE_OPENEMS or PATH):
# writes docs/validation-openems.md
openems:
    cargo test --manifest-path crosscode/Cargo.toml --release --test openems -- --ignored --nocapture

# Elmer against the static solver (a coplanar line's capacitance, a heater's rise), where
# Elmer is installed (PHOTONOXIDE_ELMER or PATH; PHOTONOXIDE_ELMER_PATH for its libraries'
# folders): writes docs/validation-elmer.md
elmer:
    cargo test --manifest-path crosscode/Cargo.toml --release --test elmer -- --ignored --nocapture

# the cross-code harness's own checks: rustfmt, clippy and its unit tests (no openEMS needed)
crosscode:
    cargo fmt --manifest-path crosscode/Cargo.toml --check
    cargo clippy --manifest-path crosscode/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path crosscode/Cargo.toml

# --- The studio ---

# install the studio's front end's packages
[working-directory: 'studio']
studio-install:
    pnpm install

# build the photonoxide program as released, with the dist profile: target/dist/photonoxide
# (never plain cargo build: the window would point at the dev server)
[working-directory: 'studio']
studio: studio-install
    pnpm tauri build -- --profile dist

# the studio with live reload, for working on it
[working-directory: 'studio']
studio-dev: studio-install
    pnpm tauri dev

# type-check and build the studio's window (fails on warnings)
[working-directory: 'studio']
studio-check: studio-install
    pnpm build

# lint the studio's TypeScript, and its Svelte files' script blocks, with oxlint (fails on warnings)
[working-directory: 'studio']
studio-lint: studio-install
    pnpm lint

# time the benchmark problems: just bench "1,20"
bench threads="1":
    cargo run -p photonoxide-studio --release -- bench --threads {{threads}}

# --- Python ---

# the Python package built into python/.venv with uv, and its tests
[working-directory: 'python']
python-test:
    uv venv --allow-existing
    uv pip install maturin pytest numpy
    uv run maturin develop --release --uv
    uv run pytest tests

# the Python package's API reference, as CI makes it with pdoc, into target/python-reference
[working-directory: 'python']
python-reference:
    uv venv --allow-existing
    uv pip install maturin
    uv run maturin develop --release --uv --extras docs
    uv run pdoc photonoxide --docformat restructuredtext -o ../target/python-reference

# the Python package's wheel for this machine, with the dist profile, into python/dist
[working-directory: 'python']
python-wheel:
    uv tool run maturin build --profile dist --out dist

# --- Before merging ---

# the full suite, as the release pull request runs it (about an hour on 20 threads; needs bash)
suite:
    bash .github/full-suite.sh

# the full suite without the validation report and the examples (about 20 minutes; needs bash)
suite-fast:
    bash .github/full-suite.sh --fast

# the codebase report against the last release, in Markdown, as CI posts it on the release PR
# (needs bash; run `pnpm exec vite build` in studio/ first to measure the front end)
codebase-report:
    bash .github/codebase-report.sh

# the same with the program's code by crate, by cargo-bloat (`cargo install cargo-bloat`), which
# builds the studio's release binary first (minutes)
codebase-report-crates:
    CODEBASE_REPORT_BLOAT=1 bash .github/codebase-report.sh

# run the full suite after every merge and rebase, and the lints before every push
hooks:
    git config core.hooksPath .github/hooks

# remove the build output
clean:
    cargo clean
