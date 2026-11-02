#!/usr/bin/env bash
# Keeps one issue up to date with the table a command writes, for the Releases workflow:
#
#   keep-issue.sh "<issue title>" [--formerly "<former title>"] [--merged "<title>"] <command> [<argument>...]
#
# The command is given --previous <the issue's body> (when the issue exists), --write <the new
# body> and --news <a comment, empty when nothing is new>. The issue is created if missing,
# its body rewritten, and a comment added when there is news; a closed issue is opened again
# for news, and it is never closed (only a --merged issue is). Exits with the command's status, after updating the issue.
# Needs GH_TOKEN (issues: write) and GH_REPO.
#
# --formerly renames an issue: when no issue has the title, the one with the former title takes
# it, in the edit that rewrites its body, so it keeps its number, comments and links.
#
# --merged retires an issue whose table this one took over: when an open issue has that title it
# gets the comment "merged into #<number>" and is closed, once. A closed issue, or none, is left
# alone, so the option is safe to keep in the workflow; the issue is never deleted.
set -uo pipefail

export TITLE=$1
shift
export FORMER=
export MERGED=
while [ "${1:-}" = --formerly ] || [ "${1:-}" = --merged ]; do
  [ $# -ge 2 ] || { echo "::error::$1 needs a title"; exit 1; }
  case $1 in
    --formerly) FORMER=$2 ;;
    --merged) MERGED=$2 ;;
  esac
  shift 2
done
dir=$(mktemp -d)

# an issue of a title: an open one first, else a closed one
find_issue() {
  gh issue list --state all --limit 1000 --json number,title,state \
    --jq "map(select(.title == env.$1)) | sort_by(.state != \"OPEN\") | .[0].number // empty"
}

# the open issue of a title, if any
open_issue() {
  gh issue list --state open --limit 1000 --json number,title \
    --jq "map(select(.title == env.$1)) | .[0].number // empty"
}

number=$(find_issue TITLE) || exit 1
rename=()
if [ -z "$number" ] && [ -n "$FORMER" ]; then
  number=$(find_issue FORMER) || exit 1
  if [ -n "$number" ]; then
    rename=(--title "$TITLE")
  fi
fi
previous=()
if [ -n "$number" ]; then
  gh issue view "$number" --json body --jq .body > "$dir/previous.md" || exit 1
  previous=(--previous "$dir/previous.md")
fi

status=0
"$@" "${previous[@]}" --write "$dir/body.md" --news "$dir/news.md" || status=$?
if [ ! -s "$dir/body.md" ]; then
  echo "::error::$TITLE: no table was written"
  exit 1
fi
{ echo "## $TITLE"; echo; cat "$dir/body.md"; echo; } >> "${GITHUB_STEP_SUMMARY:-/dev/null}"

if [ -z "$number" ]; then
  url=$(gh issue create --title "$TITLE" --body-file "$dir/body.md") || exit 1
  echo "$url"
  number=${url##*/}
else
  gh issue edit "$number" "${rename[@]}" --body-file "$dir/body.md" > /dev/null || exit 1
  if [ ${#rename[@]} -gt 0 ]; then
    echo "#$number renamed from \"$FORMER\" to \"$TITLE\""
  fi
  if [ -s "$dir/news.md" ]; then
    if [ "$(gh issue view "$number" --json state --jq .state)" != OPEN ]; then
      gh issue reopen "$number" || exit 1
    fi
    gh issue comment "$number" --body-file "$dir/news.md" || exit 1
  fi
  echo "https://github.com/$GH_REPO/issues/$number"
fi

# the issue this one took over is closed, with the comment saying where its table went
if [ -n "$MERGED" ]; then
  merged=$(open_issue MERGED) || merged=
  if [ -n "$merged" ] && [ "$merged" != "$number" ]; then
    if gh issue comment "$merged" --body "merged into #$number" \
      && gh issue close "$merged" --reason completed > /dev/null; then
      echo "#$merged (\"$MERGED\") merged into #$number and closed"
    else
      echo "::warning::#$merged (\"$MERGED\") could not be closed"
    fi
  fi
fi
# something that couldn't be read fails the run, after the issue says what
exit "$status"
