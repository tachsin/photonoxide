#!/usr/bin/env bash
# The shards: the jobs that run what the area map (.github/areas.sh) picked, its validation cases,
# examples and unit tests split among runners by their measured times, so that the run's longest
# job, which sets its wall time, is short. CI's "What changed" job runs it; run it yourself to see
# CI's jobs for a branch:
#
#   git diff --name-only origin/main...HEAD | AREAS_RANGE=origin/main...HEAD bash .github/areas.sh | bash .github/shards.sh 1
#   bash .github/areas.sh --all < /dev/null | bash .github/shards.sh 2     # Tier 2 and 3
#
# It reads the area map's KEY=VALUE lines on stdin, takes the tier (1, 2 or 3) as its argument,
# and prints KEY=VALUE lines for $GITHUB_OUTPUT, the first three each the JSON list of a matrix's
# entries:
#
#   release     the release build's jobs on Linux: {name, os, cases, examples, head}: their
#               validation cases (as PHOTONOXIDE_VALIDATION_CASES takes them,
#               tests/validation_report.rs) and examples, run from the binaries one job builds
#               for all of them (ci.yml's "Release build"); head is true in the one job that also
#               checks docs/validation.md's head and list of cases, which every job's rows share
#   release_os  the same on Windows and macOS (examples only, Tier 2 and 3), each job building its own
#   test        the unit tests' jobs: {os, part, filters, native}: the filters (and --skip's) given
#               (with POOLED=true, a pull request's run: one job on each system, no split; the
#               release entries are then run side by side in one job, .github/run-pool.sh)
#               to the test binaries, and whether photonoxide-native's tests run there too
#               (in the same Cargo command as photonoxide's where the job runs every test)
#   build_cases     true when a Linux job runs validation cases: the build makes their binary
#   build_examples  the examples the Linux jobs run, which the build makes
#
# The unit tests run on the systems the area map's `systems` names (ci.yml sets it: Linux on a
# pull request, in the merge queue and on a push to main; all three in the Weekly run, by hand, on
# the release PR and with full-ci); the
# examples on Windows and macOS only on Tier 2 and 3 with systems=all. A group of cases that would
# run none of the report's cases makes no job.
#
# Then it checks the split: every validation case of docs/validation.md the run wants is in exactly
# one job, and no case is in two; every example the run wants is in exactly one job on each system
# that runs it, and none is in two there; photonoxide-native's tests, when wanted, are in exactly
# one job on each system, and never behind a filter that would leave them out. A case or an
# example in no job, or in two, fails it (and CI's "What changed" job, so nothing else runs).
#
# Bash only, no Python and no jq (AGENTS.md; jq isn't on every machine that runs this).
set -euo pipefail
# globs are matched by `case`, never expanded against the files here
set -f

tier=${1:?the tier: 1, 2 or 3}
areas=$(cat)
get() { sed -n "s/^$1=//p" <<<"$areas"; }
tests=$(get tests) cases=$(get cases) examples=$(get examples)
systems_key=$(get systems)
# photonoxide-native's tests run in no CI job (by hand: the Libraries workflow, `just
# native-tests`; CONTRIBUTING.md), whatever the area map says
native=false

# The values go into JSON strings as they are: the names, prefixes and filters here have no
# quotes, backslashes or control characters
plain() {
  local v
  for v in "$@"; do
    case "$v" in *[\"\\[:cntrl:]]*) echo "::error::shards.sh: a quote, backslash or control character in \"$v\"" >&2; exit 1 ;; esac
  done
}
release=() tested=()
# the first Linux job with cases checks the report's head and list of cases (head: true)
head_taken=false
# entry NAME OS CASES EXAMPLES: a job of the release build, its validation cases and examples
entry() {
  plain "$@"
  local head=false
  if [ "$2" = ubuntu-latest ] && [ -n "$3" ] && [ "$head_taken" = false ]; then head=true head_taken=true; fi
  release+=("{\"name\":\"$1\",\"os\":\"$2\",\"cases\":\"$3\",\"examples\":\"$4\",\"head\":$head}")
}
# test_entry OS PART FILTERS NATIVE: a job of the unit tests (NATIVE true or false)
test_entry() {
  plain "$@"
  case "$4" in true | false) ;; *) echo "::error::shards.sh: native is \"$4\"" >&2; exit 1 ;; esac
  tested+=("{\"os\":\"$1\",\"part\":\"$2\",\"filters\":\"$3\",\"native\":$4}")
}
json() { local IFS=,; echo "[$*]"; }

# the examples' systems: Linux on Tier 1, the three on Tier 2 and 3 where the run goes to Windows
# and macOS (the area map's systems=all: not on a pull request but the release PR's and full-ci's)
if [ "$tier" = 1 ] || [ "$systems_key" != all ]; then systems="ubuntu-latest"; else systems="ubuntu-latest windows-latest macos-latest"; fi
# the unit tests': as the area map says (Linux alone when it says nothing)
case "$systems_key" in
  all) test_systems="ubuntu-latest windows-latest macos-latest" ;;
  linux | "") test_systems="ubuntu-latest" ;;
  *) echo "::error::shards.sh: systems is \"$systems_key\"" >&2; exit 1 ;;
esac

# --- The validation cases ----------------------------------------------------------------------
#
# The slowest groups each on a runner of their own: a group is a name and its prefixes, and the
# prefixes of the earlier groups inside it are left out; what no group has runs in "the rest".
# Every case runs in exactly one job, and together they check the whole report (checked below).
# Measured on CI's runners (0.5.2, #321 and #328): fdtd's adjoints 4.7 min, its smoothing 4.8,
# its comparisons with fdfd 2.5, the rest of fdtd 4.4, fdfd3d 6.3; the release build about 2
# minutes more each. The fabrication engine's round trips (#622) took 6.4 minutes for the
# library's alone (#635). On main's push of the 0.6.2 release (4 November 2026, run 41780269185;
# job minutes, the release build included): shifter/ and line/ 14.5, the rest of device/ with
# mode/ 13.1, fdfd3d 12.3, fabrication 12.0, the fdtd groups 5.8 to 7.4.
# Measured again on the pull requests of 6 and 7 November 2026 (runs 42298707033 and 42336929089,
# minutes of cases): the rest had grown to 27.8 (a job of 31 to 50 minutes with the examples it
# ran too, once past the limit of 60), eme/ 18.4 of them (its 3D sections cut from a drawing,
# eme/3d-cut, 9.5; the rest of eme/ 8.9), and device/'s pcm, thermo, threads and compact 14.0
# (pcm 5.4, threads 1.7, thermo 3.7, compact 3.2). So eme/ is split in two, device/'s four in two
# (pcm with threads 7.1, thermo with compact 6.9), and the rest is left with 9.4.
# Measured again from the cases' times in the CI logs of 7 to 12 November 2026 (minutes of cases,
# 978 cases): eme/ without its 3D cut had grown to 22.0 (a job of 16 minutes at its median, 28 at
# p90 and 52 at most) and the rest to 15.7 (quantum/ 3.1, job/ 2.8, nonlinear/ 2.1, pockels/ 1.4),
# against a median of 7 for the other groups. So eme/ is split in three more (its drawn 3D taper
# 6.0, the other 3D sections 7.0, the rest of eme/ with the bends 9.0), and two groups come out of
# the rest (quantum/ with nonlinear/ 5.2, job/ with pockels/ 4.2), which leaves it 6.3.
gnames=() gprefixes=()
group() { gnames+=("$1"); gprefixes+=("$2"); }
group "fdtd/adjoint" "fdtd/adjoint"
group "fdtd/smoothing" "fdtd/smoothing"
group "fdtd/fdfd" "fdtd/fdfd"
group "fdtd/" "fdtd/"
group "fdfd3d/" "fdfd3d/"
group "fabrication/" "fabrication/"
group "device/ (pcm, threads)" "device/pcm device/threads"
group "device/ (thermo, compact)" "device/thermo device/compact"
group "device/ (the rest) and mode/" "device/ mode/"
group "shifter/ and line/" "shifter/ line/"
group "eme/3d-cut" "eme/3d-cut"
group "eme/3d-drawing" "eme/3d-drawing"
group "eme/3d (the rest)" "eme/3d-"
group "eme/ (the rest)" "eme/"
group "quantum/ and nonlinear/" "quantum/ nonlinear/"
group "job/ and pockels/" "job/ pockels/"
# selects PREFIXES ID: whether a job with these cases runs the case ID, as
# tests/validation_report.rs reads them (no prefixes: the job runs no cases, its step is skipped)
selects() {
  local p include=false any=false
  [ -n "$1" ] || return 1
  for p in $1; do
    case "$p" in
      -*) case "$2" in "${p#-}"*) return 1 ;; esac ;;
      *) any=true; case "$2" in "$p"*) include=true ;; esac ;;
    esac
  done
  [ "$any" = false ] || [ "$include" = true ]
}
# the report's case ids, in order
report_ids=()
while IFS= read -r line; do
  id=${line#| \`}; report_ids+=("${id%%\`*}")
done < <(grep -E '^\| `' docs/validation.md)
# runs_any PREFIXES: whether a job with these cases would run at least one case of the report: a
# job that would run none isn't made (it would start a runner and fetch the build for nothing)
runs_any() {
  local id
  for id in "${report_ids[@]}"; do
    if selects "$1" "$id"; then return 0; fi
  done
  return 1
}
# first_group ID_PREFIX: the first group (the most specific: the groups go from narrow to wide)
# with a prefix that holds the area's prefix, which alone runs it
first_group() {
  local i p
  for i in "${!gnames[@]}"; do
    for p in ${gprefixes[$i]}; do
      case "$1" in "$p"*) echo "$i"; return ;; esac
    done
  done
  echo -1
}
rest=""
if [ -n "$cases" ]; then
  for i in "${!gnames[@]}"; do
    # the group's prefixes that the changed areas reach: a prefix inside an area's (fdtd/adjoint
    # in fdtd/) whole; an area's inside the prefix (device/study- in device/) alone, in the first
    # group that holds it, not the whole of the group's prefix
    mine=""
    for p in ${gprefixes[$i]}; do
      if [ "$cases" = "*" ]; then
        mine+=" $p"
        continue
      fi
      for c in $cases; do
        case "$p" in "$c"*) mine+=" $p"; continue ;; esac
        case "$c" in "$p"*) if [ "$(first_group "$c")" = "$i" ]; then mine+=" $c"; fi ;; esac
      done
    done
    mine=$(tr ' ' '\n' <<<"$mine" | awk 'NF && !seen[$0]++' | tr '\n' ' ')
    [ -n "${mine// /}" ] || continue
    these="${mine% }"
    for ((j = 0; j < i; j++)); do
      for q in ${gprefixes[$j]}; do
        for p in $mine; do
          case "$q" in "$p"*) these+=" -$q" ;; esac
        done
      done
    done
    runs_any "$these" || continue
    entry "${gnames[$i]}" ubuntu-latest "$these" ""
  done
  if [ "$cases" = "*" ]; then
    for i in "${!gnames[@]}"; do
      for p in ${gprefixes[$i]}; do rest+=" -$p"; done
    done
  else
    # an area that no prefix of a group covers, less the prefixes inside it
    for c in $cases; do
      covered=false
      for i in "${!gnames[@]}"; do
        for p in ${gprefixes[$i]}; do
          case "$c" in "$p"*) covered=true ;; esac
        done
      done
      [ "$covered" = true ] && continue
      rest+=" $c"
      for i in "${!gnames[@]}"; do
        for p in ${gprefixes[$i]}; do
          case "$p" in "$c"*) rest+=" -$p" ;; esac
        done
      done
    done
  fi
fi
if [ -n "${rest// /}" ] && runs_any "${rest# }"; then
  entry "the rest" ubuntu-latest "${rest# }" ""
fi

# --- The examples ------------------------------------------------------------------------------
#
# "*" is every example but the 3D devices: on Linux on Tier 1; on Tier 2 and 3 on all three
# systems, and the 3D devices on Linux, each on a runner of their own (crossing and ring
# together). A 3D device whose file changed runs on a runner of its own on Tier 1 too.
# The others run in two jobs on each system, the four slowest in the first and every other in
# the second: measured on 6 November 2026 (runs 42173428828 and 42336929089, seconds on Linux,
# Windows and macOS), the four took 466, 906 and 579 of 969, 1850 and 1260 (optimize_ybranch
# 124/278/203, l3_akahane 140/228/133, optimize_mesh_selfconfig 129/~250/156, bump_oskooi
# 73/150/87), the others the rest, each under 55 s on Linux. A new example goes to the second.
heavy="splitter_rotator_liu_poon mode_converter_liu_poon mmi_liu_poon coupler_liu_poon crossing_liu_poon ring_liu_poon"
slowest="optimize_ybranch l3_akahane optimize_mesh_selfconfig bump_oskooi"
set +f
all_examples=(examples/*.rs)
set -f
light=""
for example in "${all_examples[@]}"; do
  name=$(basename "$example" .rs)
  case " $heavy " in *" $name "*) ;; *) light+=" $name" ;; esac
done
# split OS NAMES: the two jobs of NAMES (light examples) on OS
split() {
  local first="" second="" name
  for name in $2; do
    case " $slowest " in *" $name "*) first+=" $name" ;; *) second+=" $name" ;; esac
  done
  [ -z "$first" ] || entry "examples 1 of 2" "$1" "" "${first# }"
  [ -z "$second" ] || entry "examples 2 of 2" "$1" "" "${second# }"
}
if [ "$examples" = "*" ]; then
  for os in $systems; do split "$os" "$light"; done
  if [ "$tier" != 1 ]; then
    for h in splitter_rotator_liu_poon mode_converter_liu_poon mmi_liu_poon coupler_liu_poon "crossing_liu_poon ring_liu_poon"; do
      entry "$h" ubuntu-latest "" "$h"
    done
  fi
else
  others=""
  for name in $examples; do
    case " $heavy " in *" $name "*) entry "$name" ubuntu-latest "" "$name" ;; *) others+=" $name" ;; esac
  done
  split ubuntu-latest "$others"
fi

# --- The unit tests ----------------------------------------------------------------------------
#
# In two jobs on each system: job::'s tests in the first, every other in the second. A test whose
# name holds one of the first job's filters runs there, and the second skips it (--skip takes the
# same substrings), so each test runs once. photonoxide-native's tests no longer run here
# (native=false, above); the code below keeps their place for a run that wants them: in the
# second job when there is one, else in the first: with every test in the same Cargo command as photonoxide's
# (its filters are then only --skip's), else in a command of their own after the changed areas'
# unit tests, whose build they reuse while native/Cargo.toml's dev-dependencies repeat
# photonoxide's (checked below; a test filter would reach native's test binaries too). Measured on
# 6 November 2026 (run 42173428828, every test, thread-seconds on Linux and Windows): job:: 736
# and 1175 of 1970 and 3159, the longest one test of 208 s; photonoxide-native's tests 2.2 and
# 5.8 minutes, mostly their build. Each job builds the tests (about 5 minutes on Linux, 15 on
# Windows) and runs its share.
first_modules="job::"
if [ "$tests" = "*" ]; then
  first=$first_modules
  second=""
  for f in $first_modules; do second+=" --skip $f"; done
  second=${second# }
else
  # the changed areas' filters: those of the first job's modules there, the others here, less
  # the first's
  first="" second="" skips=""
  for f in $tests; do
    mine=false
    for m in $first_modules; do
      case "$f" in "$m"*) mine=true ;; esac
    done
    if [ "$mine" = true ]; then first+=" $f" skips+=" --skip $f"; else second+=" $f"; fi
  done
  first=${first# } second=${second# }
  [ -z "$second" ] || second+="$skips"
fi
native1=$native native2=false
if [ "$native" = true ] && [ -n "$second" ]; then native1=false native2=true; fi
for os in $test_systems; do
  if [ "${POOLED:-false}" = true ]; then
    # a pull request's run: one job on each system, every filter in one `cargo test` (its test
    # threads keep the runner's cores busy), no split
    if [ "$tests" = "*" ]; then
      test_entry "$os" "all" "" "$native"
    elif [ -n "$tests" ] || [ "$native" = true ]; then
      test_entry "$os" "the changed areas" "$tests" "$native"
    fi
    continue
  fi
  if [ -n "$first" ] || [ "$native1" = true ]; then test_entry "$os" "1 of 2" "$first" "$native1"; fi
  if [ -n "$second" ] || [ "$native2" = true ]; then test_entry "$os" "2 of 2" "$second" "$native2"; fi
done

# --- The checks --------------------------------------------------------------------------------

failed=false
fail() { echo "::error::shards.sh: $1" >&2; failed=true; }
field() { sed -E "s/.*\"$2\":\"([^\"]*)\".*/\1/" <<<"$1"; }

# (selects: above, with the groups)
if [ -n "$cases" ]; then
  job_cases=()
  for e in "${release[@]}"; do job_cases+=("$(field "$e" cases)"); done
  n=0
  while IFS= read -r line; do
    id=${line#| \`}; id=${id%%\`*}
    wanted=false
    if [ "$cases" = "*" ]; then
      wanted=true
    else
      for c in $cases; do
        case "$id" in "$c"*) wanted=true ;; esac
      done
    fi
    k=0
    for jc in "${job_cases[@]}"; do
      if selects "$jc" "$id"; then k=$((k + 1)); fi
    done
    if [ "$k" -gt 1 ]; then fail "the case $id is in $k jobs"; fi
    if [ "$wanted" = true ] && [ "$k" -eq 0 ]; then fail "the case $id is in no job"; fi
    n=$((n + 1))
  done < <(grep -E '^\| `' docs/validation.md)
  [ "$failed" = true ] || echo "checked: each of the $n cases in one job at most, each wanted one in exactly one" >&2
fi

# the examples each system runs, against those wanted there
want=()
if [ "$examples" = "*" ]; then
  for os in $systems; do
    for name in $light; do want+=("$os $name"); done
  done
  if [ "$tier" != 1 ]; then
    for name in $heavy; do want+=("ubuntu-latest $name"); done
  fi
else
  for name in $examples; do want+=("ubuntu-latest $name"); done
fi
declare -A seen=()
for e in "${release[@]}"; do
  os=$(field "$e" os) these=$(field "$e" examples)
  for name in $these; do
    if [ -n "${seen[$os $name]:-}" ]; then fail "the example $name is in two jobs on $os"; fi
    seen[$os $name]=1
  done
done
for w in "${want[@]}"; do
  if [ -z "${seen[$w]:-}" ]; then fail "the example ${w#* } is in no job on ${w%% *}"; fi
done
if [ "${#want[@]}" -ne "${#seen[@]}" ]; then fail "${#seen[@]} examples run, ${#want[@]} wanted"; fi
# photonoxide-native's tests, when wanted, in exactly one job on each system; in a job that runs
# every test they run in the same command as photonoxide's, so its filters must all be --skip's
# (a filter would leave them out)
if [ "$native" = true ]; then
  for os in $test_systems; do
    k=0
    for e in "${tested[@]}"; do
      case "$e" in *"\"os\":\"$os\""*"\"native\":true"*) ;; *) continue ;; esac
      k=$((k + 1))
      if [ "$tests" = "*" ]; then
        # the filters' words (no globbing: set -f above)
        # shellcheck disable=SC2046
        set -- $(field "$e" filters)
        while [ $# -gt 0 ]; do
          if [ "$1" = --skip ] && [ $# -ge 2 ]; then shift 2; else fail "photonoxide-native's tests on $os are in a job with the filter \"$1\""; break; fi
        done
      fi
    done
    if [ "$k" -ne 1 ]; then fail "photonoxide-native's tests are in $k jobs on $os"; fi
  done
fi
# native's tests reuse the library's tests' build only while Cargo resolves the crates both use to
# the same features: native/Cargo.toml's dev-dependencies are photonoxide's lines, and photonoxide
# with every feature it has but `default`
section() { awk -v h="[$2]" '{ sub(/\r$/, "") } /^\[/ { on = ($0 == h); next } on && !/^[[:space:]]*(#|$)/' "$1"; }
lib_dev=$(section Cargo.toml dev-dependencies)
native_dev=$(section native/Cargo.toml dev-dependencies | sed '/^photonoxide = /d')
if [ "$lib_dev" != "$native_dev" ]; then
  fail "native/Cargo.toml's dev-dependencies aren't photonoxide's (Cargo.toml's): native's tests would build the shared crates again"
fi
lib_features=$(section Cargo.toml features | sed -e 's/[[:space:]]*=.*//' -e '/^default$/d' | sort | tr '\n' ' ')
native_features=$(section native/Cargo.toml dev-dependencies | sed -n 's/^photonoxide = .*features = \[\([^]]*\)\].*/\1/p' | tr -d '" ' | tr ',' '\n' | sed '/^$/d' | sort | tr '\n' ' ')
if [ "$lib_features" != "$native_features" ]; then
  fail "native/Cargo.toml's dev-dependency on photonoxide turns on \"$native_features\", photonoxide has \"$lib_features\""
fi
[ "$failed" = false ] || exit 1
echo "checked: each of the ${#want[@]} examples wanted (by system) in exactly one job" >&2

# the Linux jobs run what one build makes; the others build their own
linux=() others=() build_cases=false build_examples=""
for e in "${release[@]}"; do
  if [ "$(field "$e" os)" = ubuntu-latest ]; then
    linux+=("$e")
    [ -z "$(field "$e" cases)" ] || build_cases=true
    these=$(field "$e" examples)
    [ -z "$these" ] || build_examples+=" $these"
  else
    others+=("$e")
  fi
done
echo "release=$(json "${linux[@]}")"
echo "release_os=$(json "${others[@]}")"
echo "test=$(json "${tested[@]}")"
echo "build_cases=$build_cases"
echo "build_examples=${build_examples# }"
