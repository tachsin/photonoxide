#!/usr/bin/env bash
# Checks keep-issue.sh (the Releases workflow's issue keeper) against recorded answers: a stand-in
# `gh` answers from a small table of issues, logs every call it gets, and nothing touches GitHub.
# The command it keeps the issue with is a stand-in too: it writes a body and a comment.
#
#   bash .github/scripts/test-keep-issue.sh   (test-hooks.sh runs it)
set -uo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failures=0
check() { # check NAME ACTUAL EXPECTED
  if [ "$2" = "$3" ]; then
    echo "ok      $1"
  else
    echo "FAILED  $1: got '$2', expected '$3'"
    failures=$((failures + 1))
  fi
}

mkdir "$tmp/bin"
# The stand-in gh: issues.tsv has "number<TAB>title<TAB>state<TAB>body" lines; `issue list` answers
# the number --jq asks for (the title is the environment variable it names, open ones first),
# everything is appended to calls.log.
cat > "$tmp/bin/gh" << 'STUB'
#!/usr/bin/env bash
echo "gh $*" >> "$FAKE/calls.log"
cmd="$1 $2"
shift 2
field() { awk -F'\t' -v n="$1" -v c="$2" '$1 == n { print $c }' "$FAKE/issues.tsv"; }
case $cmd in
  "issue list")
    only_open=
    [ "$1 $2" = "--state open" ] && only_open=1
    jq_expr=
    while [ $# -gt 0 ]; do
      [ "$1" = --jq ] && jq_expr=$2
      shift
    done
    var=$(echo "$jq_expr" | sed -n 's/.*env\.\([A-Z]*\).*/\1/p')
    title=${!var}
    # open issues first
    awk -F'\t' -v t="$title" -v o="$only_open" \
      '$2 == t && (!o || $3 == "OPEN") { print ($3 == "OPEN" ? 0 : 1) " " $1 }' "$FAKE/issues.tsv" |
      sort | head -n 1 | cut -d' ' -f2
    ;;
  "issue view")
    n=$1
    if [ "$3" = body ]; then field "$n" 4; else field "$n" 3; fi
    ;;
  "issue create") echo "https://github.com/tachsin/photonoxide/issues/900" ;;
  "issue edit" | "issue comment" | "issue close" | "issue reopen") ;;
  *) echo "the stand-in gh doesn't know: $cmd" >&2; exit 1 ;;
esac
STUB
chmod +x "$tmp/bin/gh"
# the command kept: writes a table, and a comment only if NEWS is set
cat > "$tmp/command.sh" << 'CMD'
#!/usr/bin/env bash
while [ $# -gt 0 ]; do
  case $1 in
    --write) echo "the table" > "$2" ;;
    --news) printf '%s' "${NEWS:-}" > "$2" ;;
  esac
  shift
done
CMD

run() { # run <issues.tsv text> <keep-issue.sh arguments...>
  local issues=$1
  shift
  export FAKE="$tmp/fake"
  rm -rf "$FAKE"
  mkdir "$FAKE"
  printf '%b' "$issues" > "$FAKE/issues.tsv"
  PATH="$tmp/bin:$PATH" GH_REPO=tachsin/photonoxide GITHUB_STEP_SUMMARY="$tmp/summary" \
    bash "$repo/.github/scripts/keep-issue.sh" "$@" > "$FAKE/out.log" 2>&1
  echo $? > "$FAKE/status"
}
calls() { grep -c "$1" "$FAKE/calls.log" || true; }

main='New releases of all libraries'
former='Major updates available'
tab='\t'

# the merged issue is open: it gets the comment and is closed, and the main issue is only edited
run "312${tab}${main}${tab}OPEN${tab}old\n313${tab}${former}${tab}OPEN${tab}old\n" \
  "$main" --formerly "New releases of external libraries" --merged "$former" bash "$tmp/command.sh"
check "an open merged issue: the run succeeds" "$(cat "$FAKE/status")" 0
check "an open merged issue: the main issue is edited" "$(calls 'issue edit 312 ')" 1
check "an open merged issue: it gets 'merged into #312'" "$(calls 'issue comment 313 --body merged into #312')" 1
check "an open merged issue: it is closed" "$(calls 'issue close 313 ')" 1
check "an open merged issue: the main issue isn't closed" "$(calls 'issue close 312')" 0

# once it is closed, a later run leaves it alone
run "312${tab}${main}${tab}OPEN${tab}old\n313${tab}${former}${tab}CLOSED${tab}old\n" \
  "$main" --merged "$former" bash "$tmp/command.sh"
check "a closed merged issue: the run succeeds" "$(cat "$FAKE/status")" 0
check "a closed merged issue: no comment" "$(calls 'issue comment')" 0
check "a closed merged issue: not closed again" "$(calls 'issue close')" 0

# no such issue at all
run "312${tab}${main}${tab}OPEN${tab}old\n" "$main" --merged "$former" bash "$tmp/command.sh"
check "no merged issue: the run succeeds" "$(cat "$FAKE/status")" 0
check "no merged issue: nothing is closed" "$(calls 'issue close')" 0

# news for the main issue is a comment on it, and the merged issue still closes
NEWS="MUMPS 5.10" run "312${tab}${main}${tab}OPEN${tab}old\n313${tab}${former}${tab}OPEN${tab}old\n" \
  "$main" --merged "$former" bash "$tmp/command.sh"
check "news: a comment on the main issue" "$(calls 'issue comment 312 --body-file')" 1
check "news: the merged issue is closed too" "$(calls 'issue close 313 ')" 1

# the main issue is created when missing, and the merged one refers to the new number
run "313${tab}${former}${tab}OPEN${tab}old\n" "$main" --merged "$former" bash "$tmp/command.sh"
check "a missing main issue is created" "$(calls 'issue create --title New releases of all libraries')" 1
check "the merged issue refers to the new number" "$(calls 'issue comment 313 --body merged into #900')" 1

# without --merged nothing is ever closed
run "312${tab}${main}${tab}OPEN${tab}old\n313${tab}${former}${tab}OPEN${tab}old\n" "$main" bash "$tmp/command.sh"
check "without --merged: nothing is closed" "$(calls 'issue close')" 0

# --merged without a title is an error, not a command run with the wrong arguments
run "" "$main" --merged
check "--merged without a title fails" "$(cat "$FAKE/status")" 1

if [ "$failures" -eq 0 ]; then echo "all keep-issue checks passed"; else echo "$failures keep-issue check(s) failed"; fi
exit $((failures > 0))
