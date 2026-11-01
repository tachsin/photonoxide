#!/usr/bin/env bash
# Removes the agent worktrees' target folders (<main checkout>/target/agents/<name>, see
# .github/hooks/agent-worktree.sh) whose worktree is gone, so a removed worktree leaves no build
# behind (6 GB each for the lints and the debug tests). A folder whose worktree is still listed by
# `git worktree list` is kept, whatever its age; run `cargo clean` in a worktree to empty its own.
# The main checkout's post-merge runs this in the background.
#
#   .github/scripts/prune-agent-targets.sh            remove them
#   .github/scripts/prune-agent-targets.sh --dry-run  only list them
#
# Prints one line per folder removed and a last line with the count.
set -uo pipefail
HOOKS_DIR=$(cd "$(dirname "$0")/../hooks" && pwd)
dry=false
[ "${1:-}" = "--dry-run" ] && dry=true
git rev-parse --git-common-dir > /dev/null 2>&1 || {
  echo "prune-agent-targets: not inside a git checkout"
  exit 1
}
# shellcheck source=../hooks/agent-worktree.sh
source "$HOOKS_DIR/agent-worktree.sh"
root=$(agent_targets_root) || exit 1
[ -d "$root" ] || {
  echo "prune-agent-targets: no agent targets ($root)"
  exit 0
}
# the names of the agent worktrees git still knows and whose folder is there (a worktree deleted
# without `git worktree remove` counts as gone); nothing is removed when the list can't be read
list=$(git worktree list --porcelain) || {
  echo "prune-agent-targets: git worktree list failed, nothing removed"
  exit 1
}
live=" "
while IFS= read -r line; do
  case "$line" in
    "worktree "*)
      path=${line#worktree }
      [ -d "$path" ] || continue
      name=$(agent_worktree_name "$path") && live="$live$name "
      ;;
  esac
done <<< "$list"

removed=0
for dir in "$root"/*/; do
  [ -d "$dir" ] || continue
  dir=${dir%/}
  name=${dir##*/}
  case "$live" in
    *" $name "*) continue ;;
  esac
  if [ "$dry" = true ]; then
    echo "prune-agent-targets: would remove $dir"
  elif rm -rf -- "$dir" 2> /dev/null || [ ! -e "$dir" ]; then
    echo "prune-agent-targets: removed $dir"
  else
    echo "prune-agent-targets: could not remove all of $dir"
    continue
  fi
  removed=$((removed + 1))
done
if [ "$dry" = true ]; then
  echo "prune-agent-targets: $removed folder(s) without a worktree in $root"
else
  echo "prune-agent-targets: $removed folder(s) removed from $root"
fi
