#!/usr/bin/env bash
# Sourced by the hooks (not a hook itself): whether this checkout is an agent's worktree, and
# where the shared build output of those lives. An agent's worktree is a linked git worktree
# (its git dir is not the common git dir) whose top-level folder is under `.claude/worktrees/`.
# Both must hold: the path alone would catch a plain clone kept in such a folder, and a linked
# worktree alone would catch the ones you made by hand with `git worktree add`.
#
# Defines `agent_worktree` (status 0 for an agent's worktree, run from inside the checkout),
# `agent_target_dir` (this worktree's own target folder: <main checkout>/target/agents/<name>),
# `agent_targets_root`, `agent_worktree_name` and `agent_stamps_dir`. Windows-safe: backslashes
# and drive-letter case are normalised, and the folders are printed in the form `pwd -W` gives in
# Git Bash (C:/Users/...), which cargo accepts.
#
# Why one folder per worktree: cargo gives a package the same artifact hash in every checkout of
# the repository (its paths are relative to the checkout), so worktrees building into one shared
# folder overwrite each other's test binaries and can take each other's builds for up to date; an
# agent's pass could be another tree's result. Each folder holds one tree's build (6 GB for the lints
# and the debug tests; more with release builds) and goes when its worktree goes (.github/scripts/prune-agent-targets.sh,
# which the main checkout's post-merge runs).

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

# The name of an agent worktree's folder: its path below `.claude/worktrees/`, lower case, with
# `/` as `_` (agent-a01 for .claude/worktrees/agent-a01). Takes a path, or the checkout's top.
agent_worktree_name() {
  local top
  top=${1:-$(git rev-parse --show-toplevel 2> /dev/null)} || return 1
  top=$(printf '%s' "$top" | tr 'A-Z\\' 'a-z/')
  top=${top%/}
  case "$top/" in
    */.claude/worktrees/?*) ;;
    *) return 1 ;;
  esac
  top=${top##*/.claude/worktrees/}
  printf '%s\n' "${top//\//_}"
}

# Where the agent worktrees' target folders live: <main checkout>/target/agents, computed from the
# common git dir (no fixed path). Outside a normal `.git` layout, a folder inside the git dir.
agent_targets_root() {
  local common
  common=$(_common_dir) || return 1
  case "$common" in
    */.git) echo "${common%/.git}/target/agents" ;;
    *) echo "$common/target-agents" ;;
  esac
}

# This agent worktree's own target folder: <main checkout>/target/agents/<name>. Two worktrees
# never get the same one.
agent_target_dir() {
  local root name
  root=$(agent_targets_root) || return 1
  name=$(agent_worktree_name) || return 1
  echo "$root/$name"
}

# Where the lints' stamps go for those: with the shared build output, not in the worktree.
agent_stamps_dir() {
  local dir
  dir=${CARGO_TARGET_DIR:-$(agent_target_dir)} || return 1
  echo "$dir/lint-stamps"
}
