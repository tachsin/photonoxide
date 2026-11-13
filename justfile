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

# the platform canaries, on 4 threads: the building blocks (the system's libm, faer's kernels, the
# sparse LU, QMR and GMRES, the FFT, a mode and an FDFD solve, threads, FDTD's bits, a job file
# with CRLF) against exact values or themselves, then the existing tests they reuse, by name
# (tests/canary.rs lists them all; CONTRIBUTING.md, "Platform canaries")
canaries $RAYON_NUM_THREADS="4":
    cargo test --release --test canary -- --test-threads=4 --nocapture
    cargo test --release --lib -- --test-threads=4 --nocapture canary_
    cargo test --release --lib -- --test-threads=4 --exact \
        portable::tests::they_are_within_an_ulp_or_two_of_the_systems \
        portable::tests::they_give_the_same_bits_on_every_system \
        fdtd::ring::tests::bessel_functions_match_tabulated_values \
        mode::hadley::tests::bessel_matches_known_values \
        material::catalogue::conductivity::tests::the_exponential_integral_takes_its_known_values_either_side_of_one \
        sparse::multifrontal::tests::the_threaded_product_of_a_fronts_update_is_the_plain_one \
        backend::dense::tests::faers_kernels_behind_the_trait_give_the_solvers_own_bits \
        eigen::schur::tests::the_schur_form_reconstructs_the_matrix_and_has_its_eigenvalues \
        eigen::contour::tests::a_diagonal_pencil_gives_its_eigenvalues_to_rounding \
        eigen::contour::tests::every_eigenvalue_in_a_disc_of_a_non_hermitian_matrix_against_a_dense_solve \
        sparse::multifrontal::tests::it_solves_and_transposes_by_nested_dissection_or_amd \
        sparse::multifrontal::tests::rows_permuted_away_from_the_diagonal_are_matched_back \
        sparse::multifrontal::tests::a_zero_pivot_is_perturbed_and_counted \
        sparse::multifrontal::tests::the_factors_are_the_same_bits_on_any_number_of_threads \
        backend::krylov::tests::krylov_solvers_are_the_same_bit_for_bit_on_any_number_of_threads \
        backend::krylov::tests::the_threaded_product_is_the_sequential_one_bit_for_bit \
        backend::krylov::block_tests::block_qmr_is_the_same_bit_for_bit_on_any_number_of_threads \
        fdfd::three::tests::qmr_without_the_matrix_takes_the_iterations_and_gives_the_field_of_qmr_with_it \
        nonlinear::fft::tests::matches_the_direct_sum_and_inverts \
        mode::slab::tests::the_books_slab_gives_its_effective_indices \
        mode::slab::tests::the_roots_satisfy_the_books_tangent_form \
        mode::vector::tests::a_uniform_box_has_its_exact_discrete_modes \
        mode::vector::tests::a_strip_has_a_te_like_fundamental_mode \
        eigen::contour::tests::the_same_bits_on_one_and_many_threads \
        fdfd::tests::a_slab_reflects_and_transmits_as_the_transfer_matrices_say_at_second_order \
        fdfd::tests::a_step_is_reciprocal_and_a_te_mode_reflects_as_fresnel_says \
        fdfd::fill::tests::the_fills_values_are_the_same_on_any_number_of_threads \
        fdtd::kernel_tests::the_kernel_is_the_plain_loops_bits_on_every_boundary_and_medium \
        fdtd::tests::the_fields_are_the_same_bits_on_any_number_of_threads \
        parallel::tests::the_results_come_in_the_items_order_on_any_number_of_threads \
        circuit::components::tests::spectra_are_the_same_bit_for_bit_on_any_number_of_threads \
        geometry::array_tests::the_same_bits_on_any_number_of_threads \
        device::tests::a_solve_gives_the_same_bits_on_one_thread_and_many \
        compact::touchstone::tests::lines_end_in_lf_crlf_or_cr \
        export::tables::tests::tables_read_back_to_the_events_bits \
        job::results::tests::a_large_array_goes_to_a_file_with_the_same_bits \
        job::short_tests::the_api_docs_job_files_are_yaml_that_reads \
        farm::tests::a_worker_refuses_a_wrong_token \
        farm::tests::a_worker_refuses_what_isnt_a_hello_and_an_overlong_one \
        farm::tests::a_worker_refuses_a_connection_that_says_nothing_within_the_handshake \
        farm::tests::whole_jobs_come_back_in_their_order_whichever_finishes_first

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

# rewrite the references index (docs/references.md, docs/references-by-file.md) from
# docs/references.yaml and the citations, without building the studio
references:
    rustc --edition 2024 -O --out-dir target/references .github/scripts/references.rs
    target/references/references --write

# add the entries of newly cited DOIs to docs/references.yaml, from Crossref (needs the network)
references-crossref:
    rustc --edition 2024 -O --out-dir target/references .github/scripts/references.rs
    target/references/references --crossref

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

# photonoxide-native's tests, which CI doesn't run: against its test library, and against the
# external libraries installed here (the Libraries workflow runs them on clean machines, by hand:
# `gh workflow run Libraries`)
native-tests:
    cargo test --all-features -p photonoxide-native

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

# the studio's fast tests, which CI doesn't run (the pre-push hook does, when a push changes what
# the studio reads): its window's lint, type-check, build and tests, then every test of the studio
# but the slow physics solves of .github/studio-slow-tests.txt (needs bash)
studio-check:
    bash .github/studio-tests.sh fast

# the studio's slow tests alone, the physics solves of .github/studio-slow-tests.txt (a minute or
# more each; needs bash): before a push that changes the studio's Rust code, a job or circuit
# file, or a library area the studio uses
studio-slow:
    bash .github/studio-tests.sh slow

# lint the studio's TypeScript, and its Svelte files' script blocks, with oxlint (fails on warnings)
[working-directory: 'studio']
studio-lint: studio-install
    pnpm lint

# before each release, on Windows: the studio's page and tab switches, first loads, WebGL contexts and leaks against studio/scripts/perf-limits.json (needs the release build: docs/studio-performance.md; not in CI)
perf *args:
    node studio/scripts/perf-check.mjs {{args}}

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
    uv run pdoc photonoxide --docformat restructuredtext --no-search -o ../target/python-reference

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

# the release's marks (ROADMAP ✅, README status row, the site's FALLBACK_MILESTONES, CHANGELOG), as the release PR's check; `--at v0.6.3` checks a tag (needs bash)
release-marks *args:
    bash .github/scripts/release-marks.sh {{args}}

# after a release and a deploy: does tachsin.gr show the latest release? Reads the live pages, deploys nothing (`--wait 15` keeps trying)
site-check *args:
    node .github/scripts/site-check.mjs {{args}}

# run the full suite after every merge and rebase, and the lints before every push (an agent's
# worktree under .claude/worktrees/ skips the suite and builds the lints in a target folder of its own,
# <main checkout>/target/agents/<name>)
hooks:
    git config core.hooksPath .github/hooks

# remove the agent worktrees' target folders (target/agents/<name>) whose worktree is gone; the main
# checkout's post-merge hook does it too (`--dry-run` only lists them)
prune-agent-targets *args:
    bash .github/scripts/prune-agent-targets.sh {{args}}

# check the hooks tell an agent's worktree from the main checkout, on a throwaway repository
# (nothing builds; needs bash)
test-hooks:
    bash .github/scripts/test-hooks.sh

# remove the build output
clean:
    cargo clean
