#!/usr/bin/env bash
# .github/case-times.txt from a CI run on main that ran every case and example in shards (a push
# run): each case's and example's time, as its shard printed it. Needs gh, signed in.
#
#   bash .github/scripts/case-times.sh RUN_ID > .github/case-times.txt
set -euo pipefail
run=${1:?the id of a CI run on main that ran every case and example}

cat << EOF
# The time each validation case and example took on CI's Linux runner, alone on it (seconds), as
# printed in a push run of main (run $run). .github/shards.sh adds them up for a pull request's
# cases and examples, to choose between one job of them side by side and the shards (ci.yml). A
# case or an example missing here counts 10 s. To refresh it from a newer run:
# bash .github/scripts/case-times.sh RUN_ID > .github/case-times.txt
EOF
gh api "repos/{owner}/{repo}/actions/runs/$run/jobs?per_page=100" \
  --jq '.jobs[] | select(.conclusion == "success" and (.name | startswith("Validation and examples ("))) | .id' |
  while read -r job; do
    gh api --allow-escape-sequences "repos/{owner}/{repo}/actions/jobs/$job/logs"
  done |
  # each log line starts with its time stamp; a case's line is indented, an example's isn't
  sed -E 's/^[^ ]+ //' |
  awk '/^ +[0-9.]+ s  [^ ]+$/ { print "case", $3, $1; next }
       /^[0-9]+ s  [^ ]+$/ { print "example", $3, $1 }' |
  sort -k1,1 -k2,2
