#!/usr/bin/env bash
# A pull request's validation cases and examples in one job: the entries the shards would give
# jobs of their own (.github/shards.sh's `release` list, in RELEASE), run side by side as processes
# on this runner's cores, from the binaries the job built (BUILD: validation_report, examples/).
# CI's "Validation and examples" job runs it; every push to main and the Weekly run give each
# entry a job of its own instead (ci.yml).
#
# Each case group is one task (its cases in one run of the report's test binary, as a shard job
# runs them: rows only but in the entry with head: true); each example is a task of its own. At
# most POOL tasks run at once (default: the runner's cores), and at most HEAVY (default 2) of the 3D
# groups (fdfd3d, eme/3d...), the largest in memory. Each task's output is printed whole when it ends,
# with its time; the script fails when any task failed, after all have run.
#
#   RELEASE='[...]' BUILD=$RUNNER_TEMP/build bash .github/run-pool.sh
set -uo pipefail

: "${RELEASE:?the release entries, as JSON}" "${BUILD:?the folder of the built binaries}"
pool=${POOL:-$(nproc 2> /dev/null || echo 2)}
heavy_max=${HEAVY:-2}
logs=$(mktemp -d)

# the entries, one a line (the shards' JSON: flat objects whose values hold no quotes or braces,
# .github/shards.sh), and a field of one
entries=$(sed -e 's/^\[{//' -e 's/}\]$//' -e 's/},{/\n/g' <<< "$RELEASE")
field() { sed -E "s/.*\"$2\":(\"([^\"]*)\"|([a-z]+)).*/\2\3/" <<< "$1"; }

# the tasks, one a line: kind<TAB>name<TAB>cases<TAB>head; the slowest examples first (they set
# the wall time: .github/shards.sh's `slowest`), then the case groups, then the other examples
slowest=" optimize_mesh_selfconfig optimize_ybranch l3_akahane bump_oskooi "
tasks=() first=() later=()
while IFS= read -r e; do
  [ -n "$e" ] || continue
  cases=$(field "$e" cases)
  [ -z "$cases" ] || tasks+=("cases	$(field "$e" name)	$cases	$(field "$e" head)")
  for name in $(field "$e" examples); do
    case "$slowest" in *" $name "*) first+=("example	$name		") ;; *) later+=("example	$name		") ;; esac
  done
done <<< "$entries"
tasks=("${first[@]}" "${tasks[@]}" "${later[@]}")
if [ "${#tasks[@]}" -eq 0 ]; then
  echo "::error::run-pool.sh: no task in RELEASE: $RELEASE"
  exit 1
fi
echo "${#tasks[@]} tasks, $pool at a time"

# run_task KIND NAME CASES HEAD: one task, its output to stdout
run_task() {
  local start=$SECONDS status=0
  if [ "$1" = cases ]; then
    if [ "$4" != true ]; then export PHOTONOXIDE_VALIDATION_ROWS_ONLY=1; fi
    PHOTONOXIDE_VALIDATION_CASES="$3" CARGO_MANIFEST_DIR="$GITHUB_WORKSPACE" \
      "$BUILD/validation_report" --ignored --nocapture 2>&1 || status=$?
  else
    "$BUILD/examples/$2" > "$logs/$2.out" 2>&1 || status=$?
    if [ "$status" = 0 ]; then diff -u "examples/output/$2.txt" "$logs/$2.out" || status=$?; else cat "$logs/$2.out"; fi
  fi
  echo "$((SECONDS - start)) s  $1 $2 (exit $status)"
  return "$status"
}

declare -A pid_task=() heavy_pids=()
failed=()
# finish: wait for one task, print its output, note a failure
finish() {
  local done_pid status i kind name
  wait -n -p done_pid "${!pid_task[@]}"
  status=$?
  i=${pid_task[$done_pid]}
  unset "pid_task[$done_pid]"
  unset "heavy_pids[$done_pid]"
  IFS=$'\t' read -r kind name _ <<< "${tasks[$i]}"
  echo "::group::$kind $name"
  cat "$logs/$i.log"
  echo "::endgroup::"
  tail -n 1 "$logs/$i.log"
  if [ "$status" != 0 ]; then
    failed+=("$kind $name")
    echo "::error::$kind $name failed (exit $status): its output is in the group above"
  fi
}
heavy() { case "$1" in *3d*) return 0 ;; *) return 1 ;; esac; }

pending=("${!tasks[@]}")
while [ "${#pending[@]}" -gt 0 ]; do
  # the first pending task that may start now (a 3D group waits while HEAVY others run)
  pick=""
  for k in "${!pending[@]}"; do
    i=${pending[$k]}
    IFS=$'\t' read -r kind name _ <<< "${tasks[$i]}"
    if [ "$kind" = cases ] && heavy "$name" && [ "${#heavy_pids[@]}" -ge "$heavy_max" ]; then continue; fi
    pick=$k
    break
  done
  if [ -z "$pick" ] || [ "${#pid_task[@]}" -ge "$pool" ]; then
    finish
    continue
  fi
  i=${pending[$pick]}
  unset "pending[$pick]"
  pending=("${pending[@]}")
  IFS=$'\t' read -r kind name cases head <<< "${tasks[$i]}"
  run_task "$kind" "$name" "$cases" "$head" > "$logs/$i.log" 2>&1 &
  pid_task[$!]=$i
  if [ "$kind" = cases ] && heavy "$name"; then heavy_pids[$!]=1; fi
done
while [ "${#pid_task[@]}" -gt 0 ]; do finish; done

if [ "${#failed[@]}" -gt 0 ]; then
  echo "::error::${#failed[@]} of ${#tasks[@]} tasks failed: ${failed[*]}"
  exit 1
fi
echo "all ${#tasks[@]} tasks passed"
