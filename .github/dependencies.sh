#!/usr/bin/env bash
# Whether a change can alter the dependency graph or the features: CI's "What changed" job runs
# Tier 2 (everything, on the three systems) when it can. Run it yourself to see:
#
#   bash .github/dependencies.sh origin/main HEAD && echo "Tier 2" || echo "not by the manifests"
#
# It takes two commits, BASE and HEAD, and exits 0 when, between them, Cargo.lock or the root
# Cargo.toml changed in a way that can change what is built:
#
#   Cargo.lock   any line added or removed, but the version lines of the workspace's own packages
#                (those without a `source`, whose version release-plz raises): a package added,
#                removed or at another version, a checksum, a source, a dependency list
#   Cargo.toml   any line (comment lines and blank lines left out; a comment after a value
#                counts), of a [dependencies]-like table
#                ([dependencies], [dev-dependencies], [build-dependencies], their [target.*]
#                forms, [workspace.dependencies]), of [features], [patch.*], [replace] or
#                [workspace] (its members)
#
# It exits 1 when neither changed so: an edit of `exclude`, the package's metadata, a comment or
# a [profile.*] table (all of which Tier 1 checks, the root manifest being shared: every test on
# Linux). When in doubt (a file it can't read at either commit, which is how a new or a removed
# manifest shows) it exits 0. Bash, git, awk and diff only (AGENTS.md).
set -euo pipefail

base=${1:?the base commit} head=${2:?the head commit}

# show COMMIT FILE: the file at the commit, its line ends made Unix's; fails when it isn't there
show() { git show "$1:$2" | tr -d '\r'; }

# the lock without its own packages' version lines, each line after its package's name
lock() {
  awk '
    function flush() {
      for (i = 1; i <= n; i++) if (!(own && lines[i] ~ /^version = /)) print name ": " lines[i]
      n = 0
    }
    /^\[\[package\]\]$/ { flush(); own = 1; name = "" }
    /^name = / { name = $3 }
    /^source = / { own = 0 }
    { lines[++n] = $0 }
    END { flush() }
  '
}

# the manifest's tables that set the dependencies and the features, each line after its header
graph() {
  awk '
    /^[[:space:]]*(#|$)/ { next }
    /^[[:space:]]*\[/ {
      h = $0
      sub(/^[[:space:]]*\[+[[:space:]]*/, "", h); sub(/[[:space:]]*\]+.*$/, "", h)
      on = (h ~ /(^|\.)(dev-|build-)?dependencies(\.|$)/ || h == "features" || h ~ /^patch(\.|$)/ || h == "replace" || h == "workspace")
      if (on) print "[" h "]"
      next
    }
    on { print }
  '
}

changed=$(git diff --name-only "$base" "$head" -- Cargo.lock Cargo.toml)
if grep -qx Cargo.lock <<<"$changed"; then
  old=$(show "$base" Cargo.lock | lock) || { echo "Cargo.lock: not at $base"; exit 0; }
  new=$(show "$head" Cargo.lock | lock) || { echo "Cargo.lock: not at $head"; exit 0; }
  if [ "$old" != "$new" ]; then
    echo "Cargo.lock: the dependencies change"
    diff <(echo "$old") <(echo "$new") | grep -E '^[<>]' | head -n 20 || true
    exit 0
  fi
  echo "Cargo.lock: only the workspace's own versions change"
fi
if grep -qx Cargo.toml <<<"$changed"; then
  old=$(show "$base" Cargo.toml | graph) || { echo "Cargo.toml: not at $base"; exit 0; }
  new=$(show "$head" Cargo.toml | graph) || { echo "Cargo.toml: not at $head"; exit 0; }
  if [ "$old" != "$new" ]; then
    echo "Cargo.toml: its dependencies, features, patches or members change"
    diff <(echo "$old") <(echo "$new") | grep -E '^[<>]' | head -n 20 || true
    exit 0
  fi
  echo "Cargo.toml: neither its dependencies, features, patches nor members change"
fi
exit 1
