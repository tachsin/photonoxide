#!/usr/bin/env bash
# The full suite on your own machine: what CI runs on the release pull request (Tier 2, issue
# #315), here, after every merge or rebase. From the repository's root:
#
#   bash .github/full-suite.sh             # everything, about an hour on 20 threads
#   bash .github/full-suite.sh --fast      # without the validation report and the examples:
#                                          # about 20 minutes, the builds included
#
# Every step runs even when an earlier one failed; the summary at the end lists each step's time
# and result, and the script fails when any step did. Steps whose tools aren't installed (pnpm
# for the studio, Rust 1.95 for the MSRV check) are skipped, and say so. The GPU's report
# (docs/validation-gpu.md) and the Python package's tests stay as AGENTS.md has them, by hand
# before a release.
#
# To run it after every merge and rebase, in the background, enable the hooks once:
#
#   git config core.hooksPath .github/hooks
#
# They write its output to target/full-suite.log; `git config --unset core.hooksPath` turns them off.
# The same hooks run the lints (.github/lints.sh) before every push; a lint that passed on a
# commit, there or here, is not run again on it.
set -uo pipefail
cd "$(dirname "$0")/.."

fast=false
[ "${1:-}" = --fast ] && fast=true

results=()
failed=0
step() {
  local name=$1
  shift
  echo
  echo "=== $name"
  local start=$SECONDS
  if "$@"; then
    results+=("$(printf '%6ds  ok      %s' $((SECONDS - start)) "$name")")
  else
    results+=("$(printf '%6ds  FAILED  %s' $((SECONDS - start)) "$name")")
    failed=1
    return 1
  fi
}
skip() { results+=("$(printf '%6s  skipped %s (%s)' - "$1" "$2")"); }

examples() {
  local status=0 example name out
  out=$(mktemp -d)
  for example in examples/*.rs; do
    name=$(basename "$example" .rs)
    echo "--- $name"
    if ! cargo run --release --quiet --example "$name" > "$out/$name.txt"; then
      status=1
    elif ! diff -u "examples/output/$name.txt" "$out/$name.txt"; then
      status=1
    fi
  done
  rm -rf "$out"
  return $status
}

studio() {
  (cd studio && pnpm install --frozen-lockfile && pnpm build) &&
    cargo fmt -p photonoxide-studio -p photonoxide-setup --check &&
    cargo clippy -p photonoxide-studio -p photonoxide-setup --all-targets -- -D warnings &&
    cargo test -p photonoxide-studio -p photonoxide-setup
}

# the lints (.github/lints.sh): one that already passed on this commit, here or in the pre-push
# hook, isn't run again
# shellcheck source=lints.sh
source .github/lints.sh
key=$(lint_key || true)
for lint in "${LINTS[@]}"; do
  name=${lint%%|*}
  if lint_passed "$key" "$name"; then
    results+=("$(printf '%6s  ok      %s (passed on this commit already)' - "$name")")
  elif step "$name" eval "${lint#*|}" && [ "$(lint_key || true)" = "$key" ]; then
    lint_stamp "$key" "$name"
  fi
done
step "tests" cargo test --all-features
step "tests, one thread" env RAYON_NUM_THREADS=1 cargo test --all-features
step "tests, native" cargo test -p photonoxide-native
if command -v pnpm > /dev/null; then
  step "studio" studio
else
  skip "studio" "no pnpm"
fi
if rustup toolchain list 2>/dev/null | grep -q '^1\.95'; then
  step "MSRV (1.95)" cargo +1.95 check --all-targets --all-features
else
  skip "MSRV (1.95)" "rustup toolchain install 1.95"
fi
if [ "$fast" = false ]; then
  step "validation report" cargo test --release --test validation_report -- --ignored
  step "examples" examples
else
  skip "validation report" "--fast"
  skip "examples" "--fast"
fi

echo
echo "=== summary ($(git rev-parse --short HEAD))"
printf '%s\n' "${results[@]}"
exit $failed
