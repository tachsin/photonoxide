#!/usr/bin/env bash
# The area map's and the shards' own check, which CI's "CI's own files" job runs when CI's files
# change, and which you can run here (bash only):
#
#   bash .github/check-shards.sh
#
# 1. Everything (a push to main, Tier 2 and 3, on Linux and on the three systems): .github/shards.sh
#    puts every validation case of docs/validation.md and every example in exactly one job, and
#    fails otherwise.
# 2. A pull request's run on set lists of changed files: the area map and the shards run without
#    an error (the same exactly-once check), and the jobs the pull request starts are printed:
#    What changed, Docs and CI passed always; Lint for code; Semver for src/ or the root
#    Cargo.toml; one unit-test job; one "Validation and examples" job; "CI's own files" for CI's.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

failed=0
everything() { # TIER SYSTEMS
  if bash .github/areas.sh --all < /dev/null | sed "s/^systems=.*/systems=$2/" |
    bash .github/shards.sh "$1" > /dev/null 2> "${TMPDIR:-/tmp}/check-shards.err"; then
    echo "everything, Tier $1, systems=$2: $(grep -c '^checked' "${TMPDIR:-/tmp}/check-shards.err") checks passed"
  else
    cat "${TMPDIR:-/tmp}/check-shards.err"
    echo "everything, Tier $1, systems=$2: FAILED"
    failed=1
  fi
}
everything 1 linux
everything 2 linux
everything 2 all

# scenario NAME FILE...: a pull request changing these files
scenario() {
  local name=$1 areas shards jobs=() n
  shift
  if ! areas=$(printf '%s\n' "$@" | bash .github/areas.sh 2> /dev/null); then
    echo "$name: the area map FAILED"
    failed=1
    return
  fi
  if ! shards=$(POOLED=true bash .github/shards.sh 1 <<< "$areas" 2> "${TMPDIR:-/tmp}/check-shards.err"); then
    cat "${TMPDIR:-/tmp}/check-shards.err"
    echo "$name: the shards FAILED"
    failed=1
    return
  fi
  get() { sed -n "s/^$1=//p" <<< "$areas"; }
  sh() { sed -n "s/^$1=//p" <<< "$shards"; }
  jobs=("What changed" "Docs")
  [ "$(get code)" != true ] || jobs+=("Lint")
  if printf '%s\n' "$@" | grep -qE '^(src/|Cargo\.toml$)'; then jobs+=("Semver"); fi
  n=$( (grep -o "\"part\":" <<< "$(sh test)" || true) | wc -l | tr -d " ")
  [ "$n" = 0 ] || jobs+=("Test")
  if [ "$(sh release)" != "[]" ] || [ "$(get ci)" = true ]; then jobs+=("Validation and examples"); fi
  [ "$(get ci)" != true ] || jobs+=("CI's own files")
  jobs+=("CI passed")
  printf '%-34s %2d jobs: ' "$name" "${#jobs[@]}"
  local IFS=,
  echo "${jobs[*]}"
}
scenario "docs nothing reads" docs/cross-code.md
scenario "README" README.md
scenario "the studio's code" studio/src-tauri/src/academy.rs
scenario "photonoxide-native" native/src/lib.rs
scenario "library: topology" src/topology/mod.rs
scenario "library: eme" src/eme.rs
scenario "library: fdtd" src/fdtd.rs
scenario "Docker" docker/Dockerfile docs/docker.md
scenario "a workflow" .github/workflows/ci.yml
scenario "the area map" .github/areas.sh
scenario "a workflow and topology" .github/workflows/ci.yml src/topology/mod.rs

[ "$failed" = 0 ] || exit 1
echo "the area map and the shards: every check passed"
