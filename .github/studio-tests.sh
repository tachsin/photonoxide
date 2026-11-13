#!/usr/bin/env bash
# The studio's tests, run on your workstation: CI runs none of them (CONTRIBUTING.md, "Studio
# tests run on your workstation, not in CI"). Two tiers, split by .github/studio-slow-tests.txt:
#
#   bash .github/studio-tests.sh fast   # `just studio-check`: the web view's lint, type-check,
#                                       # build and tests, then every test of the studio but the
#                                       # slow ones (the README's counts, the roadmap against the
#                                       # site, the methods pages, the lessons' links, the schema,
#                                       # the components' defaults, ...); minutes
#   bash .github/studio-tests.sh slow   # `just studio-slow`: the slow ones alone (physics solves,
#                                       # a minute or more each)
#   bash .github/studio-tests.sh all    # both
#
# The pre-push hook runs the fast tier when a push changes what the studio reads.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
mode=${1:-fast}

# the slow tests' full names, the first word of each line
slow=()
while read -r name _; do
  name=${name%$'\r'}
  case "$name" in '' | '#'*) continue ;; esac
  slow+=("$name")
done < .github/studio-slow-tests.txt

# the web view: its lint and type-check (pnpm build runs both) and build, which the studio's Rust
# code embeds, and its own tests
web() { (cd studio && pnpm install --frozen-lockfile && pnpm build && pnpm test); }

# a slow test renamed without its line in the list would run in the fast tier again: say so
names_check() {
  local listed name
  listed=$(cargo test -p photonoxide-studio -p photonoxide-setup -q -- --list 2> /dev/null | sed -n 's/: test$//p')
  for name in "${slow[@]}"; do
    if ! grep -qxF "$name" <<< "$listed"; then
      echo "studio-tests: .github/studio-slow-tests.txt names $name, which no test of the studio has: the fast tier runs it again" >&2
    fi
  done
}

case "$mode" in
  fast)
    web
    args=(--exact)
    for name in "${slow[@]}"; do args+=(--skip "$name"); done
    cargo test -p photonoxide-studio -p photonoxide-setup -- "${args[@]}"
    names_check
    ;;
  slow)
    # the built web view, which the studio's Rust code embeds
    (cd studio && pnpm install --frozen-lockfile && pnpm build)
    cargo test -p photonoxide-studio -- --exact "${slow[@]}"
    ;;
  all)
    web
    cargo test -p photonoxide-studio -p photonoxide-setup
    ;;
  *)
    echo "usage: bash .github/studio-tests.sh [fast|slow|all]" >&2
    exit 2
    ;;
esac
