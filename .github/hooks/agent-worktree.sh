#!/usr/bin/env bash
# Sourced by the hooks (not a hook itself): whether this checkout is an agent's worktree, and
# where the shared build output of those lives. An agent's worktree is a linked git worktree
# (its git dir is not the common git dir) whose top-level folder is under `.claude/worktrees/`.
# Both must hold: the path alone would catch a plain clone kept in such a folder, and a linked
# worktree alone would catch the ones you made by hand with `git worktree add`.
#
# Defines `agent_worktree` (status 0 for an agent's worktree, run from inside the checkout),
# `agent_target_dir` (the shared target folder for those: <main checkout>/target/audit) and
# `agent_stamps_dir`. Windows-safe: backslashes and drive-letter case are normalised, and the
# folders are printed in the form `pwd -W` gives in Git Bash (C:/Users/...), which cargo accepts.

# A folder's absolute path, symlinks resolved, in the form native Windows programs accept.
_abs_dir() {
  (cd "$1" 2> /dev/null && { pwd -W 2> /dev/null || pwd -P; })
}

# The common git dir (the main checkout's .git), absolute.
_common_dir() {
  _abs_dir "$(git rev-parse --git-common-dir 2> /dev/null)"
}

agent_worktree() {
  local top gitdir common
  top=$(git rev-parse --show-toplevel 2> /dev/null) || return 1
  gitdir=$(_abs_dir "$(git rev-parse --git-dir 2> /dev/null)") || return 1
  common=$(_common_dir) || return 1
  [ -n "$gitdir" ] && [ -n "$common" ] || return 1
  # the main checkout: its git dir is the common one
  [ "$gitdir" != "$common" ] || return 1
  # lower case, forward slashes: C:\Users\..\.claude\worktrees\x and c:/users/../.claude/worktrees/x
  top=$(printf '%s' "$top" | tr 'A-Z\\' 'a-z/')
  case "$top/" in
    */.claude/worktrees/*) return 0 ;;
  esac
  return 1
}

# The folder every agent worktree builds into: <main checkout>/target/audit, computed from the
# common git dir (no fixed path). Outside a normal `.git` layout, a folder inside the git dir.
agent_target_dir() {
  local common
  common=$(_common_dir) || return 1
  case "$common" in
    */.git) echo "${common%/.git}/target/audit" ;;
    *) echo "$common/target-audit" ;;
  esac
}

# Where the lints' stamps go for those: with the shared build output, not in the worktree.
agent_stamps_dir() {
  local dir
  dir=${CARGO_TARGET_DIR:-$(agent_target_dir)} || return 1
  echo "$dir/lint-stamps"
}
