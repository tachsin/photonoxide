#!/usr/bin/env bash
# The lints CI's Lint job runs, shared by .github/full-suite.sh and the pre-push hook, so that
# neither runs a lint the other already passed on the same code.
#
# A lint that passes is stamped in target/lint-stamps/, keyed by the commit's tree and the Rust
# toolchain, and only when the tracked files are exactly the commit (nothing modified or staged):
# a run on uncommitted changes neither reads nor writes stamps. `cargo clean` removes the stamps.
#
# Sourced, not run: it defines `lint_key`, `lint_passed`, `lint_stamp` and `LINTS`.

# The steps, in order: a name, then the command (run with eval from the repository's root).
LINTS=(
  "fmt|cargo fmt --all --check"
  "clippy|cargo clippy --all-targets --all-features -- -D warnings"
  "clippy, native|cargo clippy -p photonoxide-native -p photonoxide-native-testlib --all-targets -- -D warnings"
  "clippy, Python|cargo clippy -p photonoxide-python --all-targets -- -D warnings"
  "docs|env RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps --all-features"
)

# The stamps' key for the code checked out here, or nothing when tracked files differ from HEAD.
lint_key() {
  git diff --quiet HEAD -- 2> /dev/null || return 1
  local tree toolchain
  tree=$(git rev-parse 'HEAD^{tree}') || return 1
  toolchain=$(rustc -V 2> /dev/null | cksum | cut -d' ' -f1)
  echo "$tree-$toolchain"
}

lint_file() { echo "target/lint-stamps/$1/$(echo "$2" | tr -c 'A-Za-z0-9\n' '-')"; }

# Whether the lint named $2 already passed for key $1.
lint_passed() { [ -n "$1" ] && [ -f "$(lint_file "$1" "$2")" ]; }

# Records that the lint named $2 passed for key $1.
lint_stamp() {
  [ -n "$1" ] || return 0
  local file
  file=$(lint_file "$1" "$2")
  mkdir -p "$(dirname "$file")" && date -u +%FT%TZ > "$file"
}
