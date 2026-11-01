#!/usr/bin/env bash
# Checks the git hooks' detection of an agent's worktree (.github/hooks/agent-worktree.sh) on a
# throwaway repository: a main checkout, a linked worktree under .claude/worktrees/ (an agent's),
# a linked worktree elsewhere, and a plain clone inside .claude/worktrees/. It runs the real
# hooks on real merges and pushes, with stubs for the full suite and the lints, so nothing builds.
#
#   bash .github/scripts/test-hooks.sh        (CI's Docs job runs it; works in Git Bash too)
set -uo pipefail
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE CARGO_TARGET_DIR LINT_STAMPS_DIR
repo=$(cd "$(dirname "$0")/../.." && pwd)
export GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@example.com GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@example.com

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
# a folder's path as the hooks print it (C:/Users/... in Git Bash)
abs() { (cd "$1" && { pwd -W 2> /dev/null || pwd -P; }); }
failures=0
check() { # check NAME ACTUAL EXPECTED
  if [ "$2" = "$3" ]; then
    echo "ok      $1"
  else
    echo "FAILED  $1: got '$2', expected '$3'"
    failures=$((failures + 1))
  fi
}

# --- a main checkout with the real hooks and stub scripts
main="$tmp/main"
mkdir -p "$main/.github"
git init -q -b main "$main"
git -C "$main" config core.autocrlf false
git -C "$main" config core.hooksPath "$(abs "$repo")/.github/hooks"
cat > "$main/.github/full-suite.sh" <<'EOF'
echo "stub full suite"
EOF
# the real lints.sh (its stamps), with one stub lint that shows where cargo would build
cp "$repo/.github/lints.sh" "$main/.github/lints.sh"
cat >> "$main/.github/lints.sh" <<'EOF'
LINTS=('show|echo "TARGET=${CARGO_TARGET_DIR:-unset}"')
EOF
printf 'target/\n.claude/\n' > "$main/.gitignore"
echo 'fn main() {}' > "$main/a.rs"
echo '// a file whose mtime the tests age' > "$main/old.rs"
git -C "$main" add -A
git -C "$main" commit -q -m base
git -C "$main" branch topic
git init -q --bare -b main "$tmp/remote.git"
git -C "$main" remote add origin "$tmp/remote.git"
git -C "$main" push -q origin main 2> /dev/null # origin/main, which the hook diffs against
main_abs=$(abs "$main")

# --- the layouts
mkdir -p "$main/.claude/worktrees"
git -C "$main" worktree add -q "$main/.claude/worktrees/agent-1" -b agent-1
git -C "$main" worktree add -q "$main/.claude/worktrees/Agent-2" -b agent-2
git -C "$main" worktree add -q "$tmp/elsewhere" -b elsewhere
git clone -q "$tmp/remote.git" "$tmp/clone" 2> /dev/null
mkdir -p "$tmp/holder/.claude/worktrees"
git clone -q "$main" "$tmp/holder/.claude/worktrees/plain"
agent="$main/.claude/worktrees/agent-1"
agent2="$main/.claude/worktrees/Agent-2"

detect() { (cd "$1" && source "$repo/.github/hooks/agent-worktree.sh" && if agent_worktree; then echo agent; else echo normal; fi); }
check "main checkout is not an agent worktree" "$(detect "$main")" normal
check "linked worktree under .claude/worktrees is an agent worktree" "$(detect "$agent")" agent
check "a subfolder of an agent worktree is one too" "$(mkdir -p "$agent/sub" && detect "$agent/sub")" agent
check "linked worktree elsewhere is not" "$(detect "$tmp/elsewhere")" normal
check "plain clone inside .claude/worktrees is not" "$(detect "$tmp/holder/.claude/worktrees/plain")" normal
target_of() { (cd "$1" && source "$repo/.github/hooks/agent-worktree.sh" && agent_target_dir); }
check "an agent worktree's target is its own, under the main checkout's target/agents" \
  "$(target_of "$agent")" "$main_abs/target/agents/agent-1"
check "a second agent worktree gets another target (lower case)" "$(target_of "$agent2")" "$main_abs/target/agents/agent-2"
check "two agent worktrees never share a target" "$([ "$(target_of "$agent")" != "$(target_of "$agent2")" ] && echo different || echo same)" different
check "a subfolder of an agent worktree has the worktree's target" "$(target_of "$agent/sub")" "$main_abs/target/agents/agent-1"
check "the main checkout has no agent target" "$(target_of "$main" > /dev/null && echo some || echo none)" none
name_of() { (source "$repo/.github/hooks/agent-worktree.sh" && agent_worktree_name "$1"); }
# shellcheck disable=SC1003 # the path ends in a backslash, as a Windows folder path may
check "a Windows path names its worktree" "$(name_of 'C:\Users\x\repo\.claude\worktrees\Agent-A01\')" agent-a01
check "a nested worktree folder keeps its whole path in the name" "$(name_of /r/.claude/worktrees/group/agent-1)" group_agent-1

# --- the `pwd -P` branch of the path handling, taken on Linux and macOS (Git Bash has `pwd -W`
# and never takes it): the main checkout reached through a symlink still has its real path
if (pwd -W > /dev/null 2>&1); then
  echo "skipped  the pwd -P branch (this shell has pwd -W)"
else
  ln -s "$main" "$tmp/link"
  check "a symlinked folder resolves to its real path" "$(abs "$tmp/link")" "$main_abs"
  check "an agent worktree reached through a symlink gets the real target folder" \
    "$(target_of "$tmp/link/.claude/worktrees/agent-1")" "$main_abs/target/agents/agent-1"
  check "an agent worktree reached through a symlink is still detected" "$(detect "$tmp/link/.claude/worktrees/agent-1")" agent
fi

# --- touch-sources.sh: an agent worktree's sources are made newer than the shared target's builds
ref="$tmp/ref"
touch -d 2010-01-01 "$ref"
age() { touch -d 2001-01-01 "$1/old.rs"; } # older than $ref
fresh() { if [ "$1/old.rs" -nt "$ref" ]; then echo touched; else echo stale; fi; }
age "$agent"
out=$(cd "$agent" && bash "$repo/.github/scripts/touch-sources.sh")
check "touch-sources touches a stale source in an agent worktree" "$(fresh "$agent")" touched
check "touch-sources prints one line" "$(echo "$out" | wc -l | tr -d ' ')" 1
age "$main"
bash -c "cd '$main' && bash '$repo/.github/scripts/touch-sources.sh'" > /dev/null
check "touch-sources leaves the main checkout alone" "$(fresh "$main")" stale
age "$tmp/elsewhere"
bash -c "cd '$tmp/elsewhere' && bash '$repo/.github/scripts/touch-sources.sh'" > /dev/null
check "touch-sources leaves a normal linked worktree alone" "$(fresh "$tmp/elsewhere")" stale
bash -c "cd '$main' && bash '$repo/.github/scripts/touch-sources.sh' --force" > /dev/null
check "touch-sources --force touches the main checkout" "$(fresh "$main")" touched

# --- post-merge: a real merge in each checkout
echo b > "$main/b.txt"
git -C "$main" add b.txt
git -C "$main" commit -q -m b
out=$(git -C "$agent" merge --no-edit main 2>&1)
check "post-merge in an agent worktree says it is skipped" "$(echo "$out" | grep -c 'post-merge: the full suite is skipped in an agent worktree')" 1
check "post-merge in an agent worktree starts nothing and builds no target/" "$([ -e "$agent/target" ] && echo target || echo none)" none
git -C "$tmp/elsewhere" merge --no-edit main > /dev/null 2>&1
for _ in 1 2 3 4 5 6 7 8 9 10; do
  [ -s "$tmp/elsewhere/target/full-suite.log" ] && break
  sleep 1
done
check "post-merge in a normal linked worktree still starts the suite" "$(cat "$tmp/elsewhere/target/full-suite.log" 2> /dev/null)" "stub full suite"
git -C "$main" checkout -q topic
echo e > "$main/e.txt"
git -C "$main" add e.txt
git -C "$main" commit -q -m e
git -C "$main" checkout -q main
# agent targets: one whose worktree is gone, one whose folder was deleted without
# `git worktree remove`, and the two live ones
git -C "$main" worktree add -q "$main/.claude/worktrees/agent-gone" -b agent-gone
git -C "$main" worktree remove "$main/.claude/worktrees/agent-gone"
git -C "$main" worktree add -q "$main/.claude/worktrees/agent-deleted" -b agent-deleted
rm -rf "$main/.claude/worktrees/agent-deleted"
for t in agent-1 agent-2 agent-gone agent-deleted; do mkdir -p "$main/target/agents/$t/debug"; done
out=$(cd "$agent" && bash "$repo/.github/scripts/prune-agent-targets.sh" --dry-run)
check "prune --dry-run lists the targets without a worktree" "$(echo "$out" | grep -c 'would remove')" 2
check "prune --dry-run removes nothing" "$(find "$main/target/agents" -mindepth 1 -maxdepth 1 | wc -l | tr -d ' ')" 4
git -C "$main" merge -q --no-edit topic > /dev/null 2>&1
for _ in 1 2 3 4 5 6 7 8 9 10; do
  [ -s "$main/target/full-suite.log" ] && grep -q 'removed from' "$main/target/prune-agent-targets.log" 2> /dev/null && break
  sleep 1
done
check "post-merge in the main checkout still starts the suite" "$(cat "$main/target/full-suite.log" 2> /dev/null)" "stub full suite"
check "post-merge in the main checkout prunes the targets of gone worktrees" "$(find "$main/target/agents" -mindepth 1 -maxdepth 1 -exec basename {} \; | sort | tr '\n' ' ')" "agent-1 agent-2 "

# --- pre-push: the lints run in all three, and build where expected
push() { # push DIR BRANCH: the hook's output
  echo more >> "$1/a.rs"
  git -C "$1" add -A
  git -C "$1" commit -q -m "rust $2"
  git -C "$1" push -q origin "HEAD:refs/heads/$2" 2>&1
}
age "$agent"
out=$(push "$agent" push-agent)
check "pre-push in an agent worktree touches no sources (its target is its own)" "$(fresh "$agent")" stale
check "pre-push in an agent worktree runs the lints in its own target" "$(echo "$out" | grep -c "TARGET=$main_abs/target/agents/agent-1\$")" 1
check "pre-push in an agent worktree stamps in its own target" \
  "$(find "$main_abs/target/agents/agent-1/lint-stamps" -type f 2> /dev/null | wc -l | tr -d ' ')" 1
check "pre-push in an agent worktree makes no target/ inside the worktree" "$([ -e "$agent/target" ] && echo target || echo none)" none
out=$(push "$agent2" push-agent-2)
check "pre-push in a second agent worktree runs the lints in its other target" "$(echo "$out" | grep -c "TARGET=$main_abs/target/agents/agent-2\$")" 1
check "the two agent worktrees' stamps stay apart" \
  "$(find "$main_abs/target/agents/agent-1/lint-stamps" "$main_abs/target/agents/agent-2/lint-stamps" -type f 2> /dev/null | wc -l | tr -d ' ')" 2
mine=$(abs "$tmp")/mine
out=$(CARGO_TARGET_DIR="$mine" push "$agent" push-agent-env)
check "pre-push keeps a CARGO_TARGET_DIR that is set" "$(echo "$out" | grep -c "TARGET=$mine\$")" 1
check "pre-push stamps in that CARGO_TARGET_DIR" "$(find "$mine/lint-stamps" -type f 2> /dev/null | wc -l | tr -d ' ')" 1
age "$main"
out=$(push "$main" push-main)
check "pre-push in the main checkout touches no sources" "$(fresh "$main")" stale
check "pre-push in the main checkout leaves the target alone" "$(echo "$out" | grep -c 'TARGET=unset$')" 1
check "pre-push in the main checkout stamps in its own target/" \
  "$(find "$main/target/lint-stamps" -type f 2> /dev/null | wc -l | tr -d ' ')" 1
age "$tmp/elsewhere"
out=$(push "$tmp/elsewhere" push-elsewhere)
check "pre-push in a normal linked worktree touches no sources" "$(fresh "$tmp/elsewhere")" stale
check "pre-push in a normal linked worktree leaves the target alone" "$(echo "$out" | grep -c 'TARGET=unset$')" 1

echo
if [ "$failures" -eq 0 ]; then echo "all hook checks passed"; else echo "$failures hook check(s) failed"; fi
exit $((failures > 0))
