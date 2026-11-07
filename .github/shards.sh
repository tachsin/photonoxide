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
# and prints two KEY=VALUE lines for $GITHUB_OUTPUT, each the JSON list of a matrix's entries:
#
#   release  the release build's jobs: {name, os, cases, examples}: their validation cases (as
#            PHOTONOXIDE_VALIDATION_CASES takes them, tests/validation_report.rs) and examples
#   test     the unit tests' jobs: {os, part, filters, native}: the filters (and --skip's) given
#            to the test binaries, and whether photonoxide-native's tests run there too
#            (in the same Cargo command as photonoxide's where the job runs every test)
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
tests=$(get tests) native=$(get native) cases=$(get cases) examples=$(get examples)

# The values go into JSON strings as they are: the names, prefixes and filters here have no
# quotes, backslashes or control characters
plain() {
  local v
  for v in "$@"; do
    case "$v" in *[\"\\[:cntrl:]]*) echo "::error::shards.sh: a quote, backslash or control character in \"$v\"" >&2; exit 1 ;; esac
  done
}
release=() tested=()
# entry NAME OS CASES EXAMPLES: a job of the release build, its validation cases and examples
entry() {
  plain "$@"
  release+=("{\"name\":\"$1\",\"os\":\"$2\",\"cases\":\"$3\",\"examples\":\"$4\"}")
}
# test_entry OS PART FILTERS NATIVE: a job of the unit tests (NATIVE true or false)
test_entry() {
  plain "$@"
  case "$4" in true | false) ;; *) echo "::error::shards.sh: native is \"$4\"" >&2; exit 1 ;; esac
  tested+=("{\"os\":\"$1\",\"part\":\"$2\",\"filters\":\"$3\",\"native\":$4}")
}
json() { local IFS=,; echo "[$*]"; }

if [ "$tier" = 1 ]; then systems="ubuntu-latest"; else systems="ubuntu-latest windows-latest macos-latest"; fi

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
group "eme/ (the rest)" "eme/"
rest=""
if [ -n "$cases" ]; then
  for i in "${!gnames[@]}"; do
    # the group's prefixes that the changed areas reach: a prefix inside an area's (fdtd/adjoint
    # in fdtd/), or an area inside the prefix
    mine=""
    for p in ${gprefixes[$i]}; do
      take=false
      if [ "$cases" = "*" ]; then
        take=true
      else
        for c in $cases; do
          case "$p" in "$c"*) take=true ;; esac
          case "$c" in "$p"*) take=true ;; esac
        done
      fi
      [ "$take" = true ] && mine+=" $p"
    done
    [ -n "$mine" ] || continue
    these="${mine# }"
    for ((j = 0; j < i; j++)); do
      for q in ${gprefixes[$j]}; do
        for p in $mine; do
          case "$q" in "$p"*) these+=" -$q" ;; esac
        done
      done
    done
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
if [ -n "${rest// /}" ]; then
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
# same substrings), so each test runs once. photonoxide-native's tests run in the second when it
# runs every test (its filters are then only --skip's, and CI runs both crates' tests in one Cargo
# command, which reuses the library's build), else in the first, on their own. Measured on
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
if [ "$tests" = "*" ] && [ "$native" = true ] && [ -n "$second" ]; then native1=false native2=true; fi
for os in $systems; do
  if [ -n "$first" ] || [ "$native1" = true ]; then test_entry "$os" "1 of 2" "$first" "$native1"; fi
  if [ -n "$second" ] || [ "$native2" = true ]; then test_entry "$os" "2 of 2" "$second" "$native2"; fi
done

# --- The checks --------------------------------------------------------------------------------

failed=false
fail() { echo "::error::shards.sh: $1" >&2; failed=true; }
field() { sed -E "s/.*\"$2\":\"([^\"]*)\".*/\1/" <<<"$1"; }

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
  for os in $systems; do
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
[ "$failed" = false ] || exit 1
echo "checked: each of the ${#want[@]} examples wanted (by system) in exactly one job" >&2

echo "release=$(json "${release[@]}")"
echo "test=$(json "${tested[@]}")"
