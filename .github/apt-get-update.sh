#!/usr/bin/env bash
# `sudo apt-get update` on a GitHub runner, retried: an Ubuntu mirror can stall it for over an
# hour, and a stalled try used to wait out three timeouts of 3 minutes before failing the job
# (9.6 minutes, seven times in the 103 CI runs measured in November 2026). Each try now gives up
# on a connection that sends nothing for 30 s and retries each download three times itself
# (apt's own Acquire options), and stops after 2 minutes; the next try waits 10 s, then 30 s. The
# third failure fails the step. The workflows that run apt on the checked-out tree call it first
# (binaries.yml builds a release's tag, which may predate it, and keeps its own loop):
#
#   bash .github/apt-get-update.sh [apt-get update options, e.g. -q]
set -uo pipefail

pauses=(10 30)
for try in 1 2 3; do
  if sudo timeout 120 apt-get update -o Acquire::Retries=3 -o Acquire::http::Timeout=30 \
    -o Acquire::https::Timeout=30 "$@"; then
    exit 0
  fi
  if [ "$try" -eq 3 ]; then
    echo "::error::apt-get update failed three times: an Ubuntu mirror is down or stalled; rerun the job"
    exit 1
  fi
  pause=${pauses[$((try - 1))]}
  echo "::warning::apt-get update: try $try of 3 failed or ran past 2 minutes; again in $pause s"
  # a half-downloaded list can fail the next try too
  sudo rm -rf /var/lib/apt/lists/partial/*
  sleep "$pause"
done
