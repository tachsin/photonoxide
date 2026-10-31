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
# its own area is in theirs too: fdtd's media fit their poles with src/lstsq.rs, the least
# squares compact's fits use too, and its kernel's rates are timed with the triad in src/traffic.rs.
# The checks and tests that reach above their module sit in src/cases/<module>/ (docs/architecture.md)
# and belong to the area of the module they came from, so each area lists src/cases/<module>/* too.
#
# A file in no area is shared and runs everything (the library's root, units, geometry, expr,
# error (and the netlist errors in it), stop (run::Stop), verdict (the validation types), parallel, portable, stack (and src/stack/, its etches' validation cases and its boxes' fill),
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
  "src/material/* src/cases/material/*" \
  "" \
  "material::" "material/" \
  "silicon_index silica_index group_index directional_coupler mzi_dwivedi"

# the linear algebra under the solvers: sparse factorizations, the backends' trait and dense
# kernels, the bytes they move. (The iterative backend and the Krylov solvers, which fdfd
# stops and measures, are fdfd's.)
area sparse \
  "src/sparse.rs src/sparse/* src/cases/sparse/* src/backend.rs src/backend/auto.rs src/backend/dense.rs src/dense.rs src/traffic.rs" \
  "" \
  "sparse:: backend:: traffic::" "" "" \
  "bench"

# the mode solvers, and the eigensolvers under them (their contour integrals factorize with sparse);
# with the method study on multilayers (tests/method_study.rs, whose small study runs here)
area mode \
  "src/mode/* src/cases/mode/* src/eigen.rs src/eigen/* src/cases/eigen/* tests/method_study.rs" \
  "sparse convergence" \
  "mode:: eigen:: a_small_study_" "mode/" \
  "slab_soi slab_yariv_yeh bend_loss effective_index_method group_index leaky_waves leaky_wire_benchmark marcatili multilayer_chilwell optimize_bragg hadley_corners strip_waveguide directional_coupler mzi_dwivedi"

# with the allocations counted in FDFD's iterations and FDTD's time steps (tests/no_allocation.rs,
# its tests named fdfd:: and fdtd::, which runs with every test, as tests/method_study.rs does)
area fdfd \
  "src/fdfd/* src/cases/fdfd/* src/backend/iterative.rs src/backend/krylov.rs src/backend/krylov/* tests/no_allocation.rs" \
  "mode sparse" \
  "fdfd:: backend::iterative:: backend::krylov::" "fdfd/ fdfd3d/" \
  "bump_oskooi cpml_roden_gedney lorentz_okoniewski pml_oskooi subpixel_holes tfsf_square_cylinder optimize_grating_pareto"

# eigenmode expansion (#432): slab stacks' modes and their S-matrices; its taper case runs 2D FDFD
area eme \
  "src/eme.rs src/eme/* src/cases/eme/* src/validation/eme.rs" \
  "mode fdfd" \
  "eme::" "eme/" ""

# adaptive sweeps (#432): AAA fits with their error estimate, and a vector mode followed by overlap
area sweep \
  "src/sweep.rs src/sweep/* src/validation/sweep.rs" \
  "mode" \
  "sweep::" "sweep/" ""

# nonlinear optics (0.6.1): phase matching, quasi-phase matching and the acceptance bandwidths on
# the catalogue's indices; the catalogue's PPLN OPO cases (src/cases/material/catalogue/opo_checks.rs)
# find their idler through it
area nonlinear \
  "src/nonlinear.rs src/nonlinear/* src/cases/nonlinear/* src/validation/nonlinear.rs src/validation/nonlinear_guides.rs src/validation/nonlinear_kerr.rs src/validation/nonlinear_cavities.rs" \
  "material mode" \
  "nonlinear::" "nonlinear/ material/linbo3-zelmon-opo material/linbo3-jundt-opo" "qpm_fejer ppln_wang pairs_zhao soliton_coen comb_pu"

# with the GPU's report (tests/gpu_report.rs, docs/validation-gpu.md, run by hand)
area fdtd \
  "src/fdtd.rs src/fdtd/* src/cases/fdtd/* src/validation/* tests/gpu_report.rs src/lstsq.rs src/traffic.rs" \
  "mode material fdfd" \
  "fdtd::" "fdtd/" \
  "bump_oskooi cpml_roden_gedney l3_akahane lorentz_okoniewski pml_oskooi subpixel_holes tfsf_square_cylinder"

# the static solver (heat conduction, electrostatics and current on a cross-section): its shared
# core and the thermal and electro modules on it; it cuts stacks (shared) and factorizes with
# sparse's multifrontal solver. Its job kind (src/job/statics.rs) is the job area's.
area statics \
  "src/statics.rs src/statics/* src/cases/statics/* src/thermal.rs src/electro.rs" \
  "sparse material" \
  "statics:: thermal:: electro::" "static/" ""

# a change of temperature in a full-vector mode, by first-order perturbation and by re-solve
# (src/mode/thermo.rs, in the mode area too); its cases take the catalogue's dn/dT and a heat
# solve's field
area thermo   "src/mode/thermo.rs src/mode/thermo_checks.rs src/cases/mode/thermo_checks.rs"   "mode material statics"   "mode::thermo" "thermo/" ""

# the Pockels effect in a full-vector mode (src/mode/pockels.rs, in the mode area too): the
# catalogue's electro-optic tensor turned into the guide's frame, the static solver's field, the
# perturbation (thermo's) and Vpi L; its cases read the catalogue's lithium niobate
area pockels  "src/mode/pockels.rs src/mode/pockels/* src/cases/mode/pockels/*"   "mode material statics thermo"   "mode::pockels" "pockels/" ""

# travelling-wave electrodes (src/electro/*): a line's quasi-TEM index, impedance and losses from
# the static solver's capacitances and charge, its full-wave RF mode on a Yee lattice of the same
# cross-section (eigen's shift-and-invert), and a modulator's electro-optic response; its
# cases take lithium niobate's static permittivity from the pockels cases
area line     "src/electro/* src/cases/electro/*"   "statics material mode pockels"   "electro::line electro::full_wave" "line/" ""

# the thermo-optic phase shifter (src/thermal/*): a heat solve's rise in a guide's mode, by its
# perturbation's weights, into P_pi, the time constant and the crosstalk between neighbours; and
# the dynamic thermal compact models fitted to its step responses (src/thermal/rc*)
area shifter  "src/thermal/* src/cases/thermal/* src/shifter.rs"   "mode material statics thermo"   "thermal::shifter thermal::rc thermal::correction" "shifter/ thermal-rc/ correction/" ""

# jobs, their runs and replay, and farming them out; the job files in jobs/ are read by the
# library's tests and validation cases, by the studio's and by the Python package's; a job's
# [device] table is read, checked and drawn by the devices, and studied (its sweeps and corners,
# whose cases are the devices', and optimized: its objectives' derivatives, records and fronts,
# the optimize/ cases), and a job's [circuit] table is a chip of devices, computed, studied and
# optimized (the chip/ cases, and optimize_chip_rings, which reads jobs/); its [[task.component]] tables are
# the generators', and its cases of them (components with expressions, ports by name) run here;
# docs/job.schema.json and docs/results.schema.json are the schemas job::schema and
# job::results::schema write, which their tests check; tests/jobs-toml
# holds the jobs as TOML, as they were before they were YAML, which the tests read and compare
area job \
  "src/job/* src/run.rs src/run/* src/farm.rs src/farm/* jobs/* tests/jobs-toml/* docs/job.schema.json docs/results.schema.json" \
  "material mode sparse fdfd fdtd circuit device generator statics shifter line export convergence fabrication nonlinear" \
  "job:: run:: farm::" "job/ generator/job- device/study- device/compact- optimize/ chip/" "optimize_chip_rings"

# grid convergence, the gate a reported number passes (its grid study, by Richardson
# extrapolation, which mode::vector uses too): a modes job's [task.convergence] and results.json
# take it, and the examples' Checks (hadley_corners); and convergence to a tolerance (#432), whose
# cases drive the slab mode solver, 2D FDFD and eigenmode expansion, so their changes run them
area convergence \
  "src/convergence.rs src/convergence/* src/validation/convergence.rs" \
  "" \
  "convergence::" "convergence/" "hadley_corners" \
  "mode fdfd eme"

# exporting a run's results (issue #431): every run's tables as csv, from its record, and the
# fields as `.npz`; used by the job area, which wires `[task.export]` into every kind
area export \
  "src/export.rs src/export/*" \
  "mode job" \
  "export::" "" ""

# the component generators: shapes on layers with named ports, which the devices draw their
# geometry with and jobs place; their cases compare the devices' geometry with what it was
area generator \
  "src/generator.rs src/generator/*" \
  "circuit device" \
  "generator::" "generator/" ""

# a component sampled from an fdfd job's S-parameters runs the job; a heated circuit's phases
# come from the shifter's thermal compact models (src/circuit/heated*)
area circuit \
  "src/circuit/* src/cases/circuit/*" \
  "material mode job shifter" \
  "circuit::" "circuit/ components/" \
  "circuit_fit circuit_ring_critical circuit_splitter optimize_ring_flat optimize_mzi_lattice optimize_ring_mzi optimize_ring_pareto optimize_mesh optimize_mesh_selfconfig ring_q_factor directional_coupler mzi_dwivedi"

# the devices: components that know their geometry, built from the circuit's components on the
# mode solvers' cross-sections and by FDFD of their own drawings (through the 2D jobs' plane);
# the thermo-optic phase shifters on the static solver and the shifter's chain (src/thermal);
# the TFLN modulator on the Pockels cross-section and the travelling-wave line (src/electro)
area device \
  "src/device.rs src/device/* src/cases/device/*" \
  "material mode circuit generator fdfd job compact statics shifter pockels line" \
  "device::" "device/" \
  "strip_waveguide directional_coupler mzi_dwivedi ring_q_factor bend_loss rib_hadley mmi_soldano y_junction_zhang optimize_ybranch thermo_jacques thermo_harris pcm_delaney tfln_zhang"

area compact \
  "src/compact/* src/cases/compact/* src/lstsq.rs" \
  "circuit job" \
  "compact::" "compact/" "compact_mzi_ye"

# the fabrication engine (#622): the process library, the flow generator and the executor on the
# stack's geometry; its cases run the library devices' flows (the device area's), and its job kind
# (src/job/fabrication.rs) is the job area's
area fabrication \
  "src/fabrication/* src/validation/fabrication.rs src/validation/fabrication_rules.rs src/validation/fabrication_platforms.rs" \
  "material device" \
  "fabrication::" "fabrication/" ""

# with the table of what works in Rust and Python, which its tests read; its devices are the
# library's (src/device)
area facade \
  "src/facade.rs src/facade/* docs/features.md" \
  "material mode job circuit compact device" \
  "facade::" "" ""

# the layering test (src/layering.rs; the layers are in docs/architecture.md): it reads every
# file under src/, so any change to the source runs it, whatever area the change is in (below,
# where files are matched), and it makes no file shared
area layering \
  "src/layering.rs docs/architecture.md" \
  "" \
  "layering::" "" ""

# the benchmark problems, every solver's
area bench \
  "src/bench.rs src/bench/*" \
  "sparse mode fdfd fdtd job circuit" \
  "bench::" "" ""

# photonoxide-native: the external libraries' backends
area native \
  "native/*" \
  "sparse fdfd export job" \
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
  "studio/* academy/* docs/methods/* docs/libraries.md examples/README.md CHANGELOG.md ROADMAP.md jobs/* tests/jobs-toml/* circuits/* site/lib/projects/photonoxide/meta.js site/lib/projects/photonoxide/methods.js" \
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
    # the full suite on your own machine, the hooks that start it, and the justfile's recipes
    .github/full-suite.sh | .github/hooks/* | justfile) return 0 ;;
    # the codebase report on the release PR, which its own workflow runs
    .github/codebase-report.sh | .github/workflows/codebase-report.yml) return 0 ;;
    # the cross-code harness (openEMS run on your own machine, `just openems`), a crate of its
    # own outside the workspace that CI never builds (docs/cross-code.md)
    crosscode/*) return 0 ;;
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
      src/validation.rs | src/verdict.rs | tests/validation_report.rs)
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
    case "$f" in src/*) touch_area layering ;; esac
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
