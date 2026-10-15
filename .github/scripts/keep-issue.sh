#!/usr/bin/env bash
# Keeps one issue up to date with the table a command writes, for the Releases workflow:
#
#   keep-issue.sh "<issue title>" <command> [<argument>...]
#
# The command is given --previous <the issue's body> (when the issue exists), --write <the new
# body> and --news <a comment, empty when nothing is new>. The issue is created if missing,
# its body rewritten, and a comment added when there is news; a closed issue is opened again
# for news, and nothing is closed. Exits with the command's status, after updating the issue.
# Needs GH_TOKEN (issues: write) and GH_REPO.
set -uo pipefail

export TITLE=$1
shift
dir=$(mktemp -d)

# an open issue of that title first, else a closed one
number=$(gh issue list --state all --limit 1000 --json number,title,state \
  --jq 'map(select(.title == env.TITLE)) | sort_by(.state != "OPEN") | .[0].number // empty') || exit 1
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
  gh issue create --title "$TITLE" --body-file "$dir/body.md" || exit 1
else
  gh issue edit "$number" --body-file "$dir/body.md" > /dev/null || exit 1
  if [ -s "$dir/news.md" ]; then
    if [ "$(gh issue view "$number" --json state --jq .state)" != OPEN ]; then
      gh issue reopen "$number" || exit 1
    fi
    gh issue comment "$number" --body-file "$dir/news.md" || exit 1
  fi
  echo "https://github.com/$GH_REPO/issues/$number"
fi
# something that couldn't be read fails the run, after the issue says what
exit "$status"
