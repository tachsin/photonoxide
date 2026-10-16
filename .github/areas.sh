#!/usr/bin/env bash
# The area map: what a pull request's change runs (Tier 1, issue #315). CI's "What changed" job
# runs it; run it yourself to see what CI will run for a branch:
#
#   git diff --name-only origin/main...HEAD | AREAS_RANGE=origin/main...HEAD bash .github/areas.sh
#   bash .github/areas.sh --all < /dev/null     # everything (Tier 2 and 3)
#
# It reads the changed files' paths on stdin and prints KEY=VALUE lines for $GITHUB_OUTPUT:
#
#   code       true when anything is compiled or tested; false: only the docs' links are checked
#   areas      the areas that run, for the log
#   tests      the library's unit tests: filters as `cargo test` takes them, "*" for every test
#   native     true when photonoxide-native's tests run
#   cases      the validation cases: id prefixes, "*" for all, empty for none
#   examples   the examples, each against its committed output: names, or "*" for every one but
#              the 3D devices (which run on Tier 2 and 3, and when their own file changes)
#   studio     the studio's checks: empty, "linux", or "all" (Linux, Windows and macOS)
#
# Each area owns some files (globs, in which * matches / too), uses some other areas' code, and
# is checked by its unit tests, its validation cases and its examples. A change to an area runs
# its checks and those of every area that uses it, directly or through another: the areas its
# change can break. The uses are the library code's own (`use crate::...`, tests left out); an
# area's tests that use another area's code (sparse's and native's solve bench's problems) run
# when that area runs, without running further. A file that other areas use more narrowly than
# its own area is in theirs too: fdtd's media fit their poles with compact's least squares, and
# its kernel's rates are timed with bench's triad.
#
# A file in no area is shared and runs everything (the library's root, units, geometry, expr,
# error, parallel, portable, stack (and src/stack/, its etches' validation cases and its boxes' fill),
# raster and averaging (src/averaging.rs and src/averaging/, its analytic cases, which FDFD and the
# rasters read);
# Cargo.toml and Cargo.lock; anything new). When in doubt, a file is shared. Files nothing builds or reads (the site, Markdown no test reads, the
# other workflows) run nothing but the docs' links. The Python wheels (python-wheels.yml) and the
# Libraries workflow's installs (libraries.yml) start by their own `paths`.
#
# The release pull request and the weekly run run everything whatever changed, and catch what a
# missing entry here would let through.
#
# Bash only, no Python (AGENTS.md).
set -euo pipefail
# globs are matched by `case`, never expanded against the files here
set -f

declare -A files uses tested_with tests cases examples
areas=()

# area NAME FILES USES TESTS CASES EXAMPLES [TESTED_WITH]
area() {
  areas+=("$1")
  files[$1]=$2 uses[$1]=$3 tests[$1]=$4 cases[$1]=$5 examples[$1]=$6 tested_with[$1]=${7:-}
}

area material \
  "src/material/*" \
  "" \
  "material::" "material/" \
  "silicon_index silica_index group_index directional_coupler mzi_dwivedi"

# the linear algebra under the solvers: sparse factorizations, the backends' trait and dense
# kernels, the bytes they move. (The iterative backend, which takes fdfd's stopping rules, is
# fdfd's.)
area sparse \
  "src/sparse.rs src/sparse/* src/backend.rs src/backend/auto.rs src/backend/dense.rs src/traffic.rs" \
  "" \
  "sparse:: backend:: traffic::" "" "" \
  "bench"

# the mode solvers, and the eigensolvers under them (their contour integrals factorize with sparse)
area mode \
  "src/mode/* src/eigen.rs src/eigen/*" \
  "sparse" \
  "mode:: eigen::" "mode/" \
  "slab_soi slab_yariv_yeh bend_loss effective_index_method group_index leaky_waves leaky_wire_benchmark marcatili multilayer_chilwell hadley_corners strip_waveguide directional_coupler mzi_dwivedi"

area fdfd \
  "src/fdfd/* src/backend/iterative.rs" \
  "mode sparse" \
  "fdfd:: backend::iterative::" "fdfd/ fdfd3d/" \
  "bump_oskooi cpml_roden_gedney lorentz_okoniewski pml_oskooi subpixel_holes tfsf_square_cylinder"

# with the GPU's report (tests/gpu_report.rs, docs/validation-gpu.md, run by hand)
area fdtd \
  "src/fdtd.rs src/fdtd/* src/validation/* tests/gpu_report.rs src/compact/fit.rs src/compact/fit/* src/bench.rs" \
  "mode material fdfd" \
  "fdtd::" "fdtd/" \
  "bump_oskooi cpml_roden_gedney l3_akahane lorentz_okoniewski pml_oskooi subpixel_holes tfsf_square_cylinder"

# jobs, their runs and replay, and farming them out; the job files in jobs/ are read by the
# library's tests and validation cases, by the studio's and by the Python package's; a job's
# [device] table is read, checked and drawn by the devices, and studied (its sweeps and corners,
# whose cases are the devices', and optimized: its objectives' derivatives, records and fronts,
# the optimize/ cases); its [[task.component]] tables are
# the generators', and its cases of them (components with expressions, ports by name) run here
area job \
  "src/job/* src/run.rs src/farm.rs src/farm/* jobs/*" \
  "material mode sparse fdfd fdtd device generator" \
  "job:: run:: farm::" "generator/job- device/study- optimize/" ""

# the component generators: shapes on layers with named ports, which the devices draw their
# geometry with and jobs place; their cases compare the devices' geometry with what it was
area generator \
  "src/generator.rs src/generator/*" \
  "circuit device" \
  "generator::" "generator/" ""

# a component sampled from an fdfd job's S-parameters runs the job
area circuit \
  "src/circuit/*" \
  "material mode job" \
  "circuit::" "circuit/ components/" \
  "circuit_fit circuit_ring_critical circuit_splitter ring_q_factor directional_coupler mzi_dwivedi"

# the devices: components that know their geometry, built from the circuit's components on the
# mode solvers' cross-sections and by FDFD of their own drawings (through the 2D jobs' plane)
area device \
  "src/device.rs src/device/*" \
  "material mode circuit generator fdfd job" \
  "device::" "device/" \
  "strip_waveguide directional_coupler mzi_dwivedi ring_q_factor bend_loss rib_hadley mmi_soldano y_junction_zhang"

area compact \
  "src/compact/*" \
  "circuit job" \
  "compact::" "compact/" ""

# with the table of what works in Rust, Python and MATLAB, which its tests read
area facade \
  "src/facade.rs src/facade/* docs/features.md" \
  "material mode job circuit compact" \
  "facade::" "" ""

# the benchmark problems, every solver's
area bench \
  "src/bench.rs src/bench/*" \
  "sparse mode fdfd fdtd job circuit" \
  "bench::" "" ""

# photonoxide-native: the external libraries' backends
area native \
  "native/*" \
  "sparse fdfd" \
  "" "" "" \
  "bench"

# the Python package (its wheels: python-wheels.yml; every code change lints its binding crate)
area python \
  "python/* docs/python.md" \
  "facade" \
  "" "" ""

# the studio builds the whole library, photonoxide-native and every example in, runs jobs and
# circuits, and reads the methods' write-ups, the examples' table and outputs, the changelog, the
# Academy's lessons and the milestones they name, the validation report and the libraries' page;
# its tests check the project site's static copy against the roadmap, and its methods' categories
# against docs/methods
area studio \
  "studio/* academy/* docs/methods/* docs/libraries.md examples/README.md CHANGELOG.md ROADMAP.md jobs/* circuits/* site/lib/projects/photonoxide/meta.js site/lib/projects/photonoxide/methods.js" \
  "material sparse mode fdfd fdtd job circuit device generator compact facade bench native" \
  "" "" ""

# matches FILE GLOBS: whether the file matches one of the globs
matches() {
  local f=$1 g
  for g in $2; do
    # shellcheck disable=SC2254
    case "$f" in $g) return 0 ;; esac
  done
  return 1
}

# Files no build or test reads: only the docs' links. (The Markdown a test reads is in an area
# above; docs/validation.md and examples/ are read below.)
ignored() {
  local a
  case "$1" in
    docs/validation.md | examples/* | src/* | tests/*) return 1 ;;
  esac
  for a in "${areas[@]}"; do
    if matches "$1" "${files[$a]}"; then return 1; fi
  done
  case "$1" in
    site/* | *.md) return 0 ;;
    .github/dependabot.yml | .github/ISSUE_TEMPLATE/* | .github/scripts/*) return 0 ;;
    # the full suite on your own machine, and the hooks that start it
    .github/full-suite.sh | .github/hooks/*) return 0 ;;
    # this workflow and the map run everything; the others run as themselves
    .github/workflows/ci.yml | .github/areas.sh) return 1 ;;
    .github/workflows/*) return 0 ;;
    assets/* | studio/scripts/* | LICENSE* | .gitignore | .gitattributes) return 0 ;;
  esac
  return 1
}

declare -A touched hit run
everything=false
code=false
studio_systems=false
direct_cases=""
direct_examples=""

touch_area() { touched[$1]=1; code=true; }

if [ "${1:-}" = --all ]; then
  everything=true
else
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    if ignored "$f"; then continue; fi
    case "$f" in
      # the validation report: the cases whose rows changed, or all of them when its head did
      docs/validation.md)
        touch_area studio
        if [ -z "${AREAS_RANGE:-}" ]; then
          direct_cases="*"
        else
          while IFS= read -r line; do
            case "$line" in
              [-+]'| `'*) id=${line#?| \`}; direct_cases+=" ${id%%/*}/" ;;
              *) direct_cases="*" ;;
            esac
          done < <(git diff --unified=0 "$AREAS_RANGE" -- docs/validation.md | grep -E '^[-+]' | grep -vE '^(\+\+\+|---) ' || true)
        fi
        continue ;;
      # the cases themselves: all of them (a change here usually comes with its area's too)
      src/validation.rs | tests/validation_report.rs)
        direct_cases="*"; touch_area studio; continue ;;
      # an example or its output: that example (the studio builds the examples in); their
      # helpers: every example using them
      examples/output/*.txt)
        name=${f#examples/output/}; direct_examples+=" ${name%.txt}"; touch_area studio; continue ;;
      examples/pdk/* | examples/common/*)
        helper=${f#examples/}; helper=${helper%%/*}
        set +f; all_examples=(examples/*.rs); set -f
        for e in "${all_examples[@]}"; do
          if grep -qE "^mod $helper;" "$e"; then n=${e#examples/}; direct_examples+=" ${n%.rs}"; fi
        done
        touch_area studio; continue ;;
      examples/*.rs)
        name=${f#examples/}; direct_examples+=" ${name%.rs}"; touch_area studio; continue ;;
    esac
    found=false
    for a in "${areas[@]}"; do
      if matches "$f" "${files[$a]}"; then touch_area "$a"; found=true; fi
    done
    case "$f" in studio/*) studio_systems=true ;; esac
    if [ "$found" = false ]; then
      echo "shared: $f" >&2
      everything=true
    fi
  done
fi

if [ "$everything" = true ]; then
  echo "code=true"
  echo "areas=${areas[*]}"
  echo "tests=*"
  echo "native=true"
  echo "cases=*"
  echo "examples=*"
  # the studio's own code differs by system; the library under it doesn't
  if [ "${1:-}" = --all ] || [ "$studio_systems" = true ]; then echo "studio=all"; else echo "studio=linux"; fi
  exit 0
fi

# every area that uses a touched one, directly or through another
for a in "${!touched[@]}"; do hit[$a]=1; done
grew=true
while [ "$grew" = true ]; do
  grew=false
  for a in "${areas[@]}"; do
    [ -z "${hit[$a]:-}" ] || continue
    for u in ${uses[$a]}; do
      if [ -n "${hit[$u]:-}" ]; then hit[$a]=1; grew=true; break; fi
    done
  done
done
# and the areas whose tests use one of those
for a in "${areas[@]}"; do
  if [ -n "${hit[$a]:-}" ]; then run[$a]=1; continue; fi
  for u in ${tested_with[$a]}; do
    if [ -n "${hit[$u]:-}" ]; then run[$a]=1; break; fi
  done
done

names="" t="" c="" e=""
for a in "${areas[@]}"; do
  [ -n "${run[$a]:-}" ] || continue
  names+=" $a" t+=" ${tests[$a]}" c+=" ${cases[$a]}" e+=" ${examples[$a]}"
done

# each word once, in order
once() { tr ' ' '\n' | awk 'NF && !seen[$0]++' | tr '\n' ' ' | sed 's/ $//'; }

studio=""
if [ -n "${run[studio]:-}" ]; then
  studio=linux
  if [ "$studio_systems" = true ]; then studio=all; fi
fi

echo "code=$code"
echo "areas=$(echo "$names" | once)"
echo "tests=$(echo "$t" | once)"
echo "native=$([ -n "${run[native]:-}" ] && echo true || echo false)"
if [ "$direct_cases" = "*" ]; then echo "cases=*"; else echo "cases=$(echo "$c $direct_cases" | once)"; fi
echo "examples=$(echo "$e $direct_examples" | once)"
echo "studio=$studio"
