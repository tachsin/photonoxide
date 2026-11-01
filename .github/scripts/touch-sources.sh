#!/usr/bin/env bash
# Touches the tracked files cargo builds from (their modification time becomes now), so a build in
# a target folder another checkout built into is never taken for fresh from that earlier build.
#
# Cargo keys its fingerprints by source paths relative to the checkout and judges freshness by
# file mtime, so checkout B can reuse checkout A's build in a shared target folder when B's files
# are older than A's last build there; the results are then stale, or A's. Touching the sources
# first makes them newer than any build already in the target, so cargo recompiles the crate (the
# dependencies, which sit outside the checkout, stay built). It fixes freshness only, not two
# builds at once: agent worktrees therefore build into a folder of their own (target/agents/<name>,
# .github/hooks/agent-worktree.sh) and the hooks no longer run this; it stays for a target folder
# you share between checkouts by hand.
#
#   .github/scripts/touch-sources.sh            in an agent worktree: touch; elsewhere: nothing
#   .github/scripts/touch-sources.sh --force    touch in any checkout
#
# What is touched: *.rs (build.rs among them), *.toml, Cargo.lock, *.wgsl, and the data files the
# sources pull in with include_str!/include_bytes! (*.yaml, *.yml, *.json, *.md, *.txt).
# Prints one line.
set -uo pipefail
HOOKS_DIR=$(cd "$(dirname "$0")/../hooks" && pwd)
force=false
[ "${1:-}" = "--force" ] && force=true
top=$(git rev-parse --show-toplevel 2> /dev/null) || {
  echo "touch-sources: not inside a git checkout"
  exit 1
}
cd "$top" || exit 1
# shellcheck source=../hooks/agent-worktree.sh
source "$HOOKS_DIR/agent-worktree.sh"
if [ "$force" != true ] && ! agent_worktree; then
  echo "touch-sources: not an agent worktree, nothing touched (--force to touch anyway)"
  exit 0
fi
globs=('*.rs' '*.toml' 'Cargo.lock' '*.wgsl' '*.yaml' '*.yml' '*.json' '*.md' '*.txt')
# -c: a tracked file deleted in the working tree is skipped, not made; xargs batches the files
git ls-files -z -- "${globs[@]}" | xargs -0 touch -c --
count=$(git ls-files -- "${globs[@]}" | wc -l | tr -d ' ')
echo "touch-sources: touched $count build inputs under $top"
