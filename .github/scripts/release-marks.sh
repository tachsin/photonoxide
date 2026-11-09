#!/usr/bin/env bash
# The release's marks, which the release PR has to carry: a release's tag is what the project
# site, crates.io and GitHub read, so the tag's own files must already say the release is done.
#
#   bash .github/scripts/release-marks.sh [--at <git ref>] [version]
#
# The version defaults to Cargo.toml's [workspace.package] version; --at reads the files at a
# commit or tag (`--at v0.6.3`) instead of the working tree. For version X.Y.Z, whose milestone
# in ROADMAP.md is "X.Y.Z" (or "X.Y" when Z is 0):
#   - ROADMAP.md: the milestone's heading ends in ✅ (its open boxes are listed as a warning);
#   - README.md: the milestone's row in the status table says "✅ released";
#   - site/lib/projects/photonoxide/meta.js: FALLBACK_MILESTONES has the heading with its ✅
#     (the studio's test site.rs checks the whole list against ROADMAP.md);
#   - CHANGELOG.md: the first version heading is "## [X.Y.Z]".
# The Release marks workflow runs it on the release PR; `just release-marks` runs it here.
set -euo pipefail
cd "$(dirname "$0")/../.."

at=""
if [ "${1:-}" = --at ]; then
  at=$2
  shift 2
fi
# a file of the working tree, or at the ref
show() {
  if [ -n "$at" ]; then git show "$at:$1"; else cat "$1"; fi
}

version=${1:-$(show Cargo.toml | awk '/^\[workspace\.package\]/ { p = 1; next } /^\[/ { p = 0 } p && /^version *=/ { gsub(/[" ]/, "", $0); sub(/^version=/, ""); print; exit }')}
if ! [[ $version =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
  echo "::error::release-marks: no version X.Y.Z (got '$version')"
  exit 2
fi
if [ "${BASH_REMATCH[3]}" = 0 ]; then milestone="${BASH_REMATCH[1]}.${BASH_REMATCH[2]}"; else milestone=$version; fi
esc=${milestone//./\\.}
vesc=${version//./\\.}
failed=0
fail() {
  echo "::error file=$1::$2"
  failed=1
}

# ROADMAP.md: the heading, and the boxes under it up to the next heading
roadmap=$(show ROADMAP.md)
heading=$(grep -E "^### $esc: " <<<"$roadmap" | head -1 || true)
if [ -z "$heading" ]; then
  fail ROADMAP.md "ROADMAP.md has no milestone '### $milestone: …'. Add it (or the release's version is wrong)."
else
  if [[ $heading != *"✅" ]]; then
    fail ROADMAP.md "ROADMAP.md's '$heading' isn't marked released: end the heading with ' ✅'."
  fi
  open=$(awk -v h="$heading" '$0 == h { p = 1; next } p && /^##/ { exit } p && /^- \[ \]/' <<<"$roadmap")
  if [ -n "$open" ]; then
    while IFS= read -r line; do
      item=$(sed -E 's/^- \[ \] (\*\*([^*]+)\*\*)?.*/\2/' <<<"$line")
      item=${item%[:,.]}
      echo "::warning file=ROADMAP.md::$milestone has an open box: ${item:-${line:0:80}}. Tick it, or move what is left to a later milestone or an issue, and say so on the line."
    done <<<"$open"
  fi
fi

# README.md: the status table's row, "| X.Y.Z Title | … | ✅ released |"
# (a minor release's row may name it either way: "0.6.0 …" or "0.6 …")
row=$(show README.md | grep -E "^\| *($vesc|$esc) " | head -1 || true)
if [ -z "$row" ]; then
  fail README.md "README.md's status table has no row for $milestone: add '| $milestone <title> | <what it brings> | ✅ released |'."
elif [[ $row != *"✅ released"* ]]; then
  fail README.md "README.md's status row for $milestone doesn't say '✅ released' (and the next milestone's row should say 🚧): $row"
fi

# meta.js: FALLBACK_MILESTONES
meta=site/lib/projects/photonoxide/meta.js
if ! show "$meta" | awk '/^export const FALLBACK_MILESTONES = \[/ { p = 1; next } p && /^\];/ { exit } p' | grep -qE "^ *\"$esc: .*✅\",?$"; then
  fail "$meta" "FALLBACK_MILESTONES in $meta doesn't have \"$milestone: … ✅\": copy ROADMAP.md's headings (studio/src-tauri/tests/site.rs checks them)."
fi

# CHANGELOG.md: release-plz's section
first=$(show CHANGELOG.md | grep -m1 -oE '^## \[[0-9]+\.[0-9]+\.[0-9]+\]' || true)
if [ "$first" != "## [$version]" ]; then
  fail CHANGELOG.md "CHANGELOG.md's first version heading is '${first:-none}', not '## [$version]'."
fi

if [ "$failed" = 1 ]; then
  echo "The files${at:+ at $at} don't mark the release of $version yet (above). Mark it on main, or on the release PR after the last merge, before merging the release PR: the tag is what tachsin.gr, crates.io and GitHub show."
  exit 1
fi
echo "$version${at:+ at $at}: ROADMAP.md, README.md, FALLBACK_MILESTONES and CHANGELOG.md mark the release."
