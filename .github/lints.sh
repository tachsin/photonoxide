#!/usr/bin/env bash
# The lints CI's Lint job runs, shared by .github/full-suite.sh and the pre-push hook, so that
# neither runs a lint the other already passed on the same code.
#
# A lint that passes is stamped in target/lint-stamps/ (LINT_STAMPS_DIR, when set: the pre-push hook
# sets it in an agent worktree, to that worktree's own target folder), keyed by the commit's tree and the Rust
# toolchain, and only when the tracked files are exactly the commit (nothing modified or staged):
# a run on uncommitted changes neither reads nor writes stamps. `cargo clean` removes the stamps.
#
# Sourced, not run: it defines `lint_key`, `lint_passed`, `lint_stamp` and `LINTS`.

# The steps, in order: a name, then the command (run with eval from the repository's root).
# shellcheck disable=SC2034 # read by the scripts that source this file
LINTS=(
  # the same numbers on every system (#822): the allows of the system's maths functions only go
  # down, libm only in src/portable/, no mul_add (seconds; nothing builds)
  "portable|bash .github/scripts/check-portable.sh"
  "fmt|cargo fmt --all --check"
  # CI's three clippy steps (the library with all its features, the native crates, the Python
  # binding) as one command, so that the library is checked once: run separately, Cargo picks other
  # features for the shared dependencies and checks them, and the library, again. The native crates
  # and the binding have no features of their own, so --all-features changes nothing for them.
  "clippy|cargo clippy -p photonoxide -p photonoxide-native -p photonoxide-native-testlib -p photonoxide-python --all-targets --all-features -- -D warnings"
  "docs|env RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps --all-features"
  # every crate's manifest, for dependencies nothing uses; skipped, and said, without the tool
  "unused dependencies|if command -v cargo-machete > /dev/null; then cargo machete; else echo 'cargo-machete not installed (cargo install cargo-machete): skipped'; fi"
)

# The stamps' key for the code checked out here, or nothing when tracked files differ from HEAD.
lint_key() {
  git diff --quiet HEAD -- 2> /dev/null || return 1
  local tree toolchain
  tree=$(git rev-parse 'HEAD^{tree}') || return 1
  toolchain=$(rustc -V 2> /dev/null | cksum | cut -d' ' -f1)
  echo "$tree-$toolchain"
}

lint_file() { echo "${LINT_STAMPS_DIR:-target/lint-stamps}/$1/$(echo "$2" | tr -c 'A-Za-z0-9\n' '-')"; }

# Whether the lint named $2 already passed for key $1.
lint_passed() { [ -n "$1" ] && [ -f "$(lint_file "$1" "$2")" ]; }

# Records that the lint named $2 passed for key $1.
lint_stamp() {
  [ -n "$1" ] || return 0
  local file
  file=$(lint_file "$1" "$2")
  mkdir -p "$(dirname "$file")" && date -u +%FT%TZ > "$file"
}
