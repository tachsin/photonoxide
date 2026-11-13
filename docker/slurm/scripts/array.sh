#!/bin/bash
# A sweep split into parts, one array task each: task k runs the job with its sweep cut to the
# k-th of SLURM_ARRAY_TASK_COUNT runs of points, as a run of its own (its own folder, its own
# results.json). For a job in TOML whose [task.sweep] gives `from`, `to` and `points`:
#
#   sbatch --array=0-3 array.sh job.toml [runs]
#
# Farming (farm.sh) gives one record of the whole sweep instead; an array suits a scheduler that
# fits small jobs into gaps, and parts that may run days apart.
#SBATCH --job-name=photonoxide-array
#SBATCH --nodes=1
#SBATCH --ntasks=1
#SBATCH --output=array-%A_%a.out
set -euo pipefail

job=${1:?usage: sbatch --array=0-<n-1> array.sh <job.toml> [<runs folder>]}
out=${2:-runs}
# the program: photonoxide, or a command that runs it ("apptainer exec photonoxide.sif photonoxide")
read -ra program <<<"${PHOTONOXIDE:-photonoxide}"
k=${SLURM_ARRAY_TASK_ID:?not an array task}
n=${SLURM_ARRAY_TASK_COUNT:?not an array task}

# the sweep's values, from the [task.sweep] table
value() { awk -v key="$1" '/^\[/{in_sweep=($0=="[task.sweep]")} in_sweep && $1==key {print $3}' "$job"; }
from=$(value from)
to=$(value to)
points=$(value points)
# this task's points: an even share, the first tasks one more when they don't divide
read -r first count < <(awk -v p="$points" -v n="$n" -v k="$k" 'BEGIN {
  base = int(p / n); extra = p % n
  first = k * base + (k < extra ? k : extra)
  print first, base + (k < extra ? 1 : 0) }')
if [ "$count" -lt 1 ]; then
  echo "array task $k of $n has no point of $points"
  exit 0
fi
read -r part_from part_to < <(awk -v a="$from" -v b="$to" -v p="$points" -v f="$first" -v c="$count" 'BEGIN {
  h = (b - a) / (p - 1)
  printf "%.15g %.15g\n", a + f * h, a + (f + c - 1) * h }')

part=$(mktemp "${TMPDIR:-/tmp}/part-$k-XXXXXX.toml")
awk -v f="$part_from" -v t="$part_to" -v c="$count" -v k="$k" '
  /^name *=/ { sub(/" *$/, "-part-" k "\""); print; next }
  /^\[/ { in_sweep = ($0 == "[task.sweep]") }
  in_sweep && $1 == "from" { print "from = " f; next }
  in_sweep && $1 == "to" { print "to = " t; next }
  in_sweep && $1 == "points" { print "points = " c; next }
  { print }' "$job" > "$part"
# a part of a single point is a sweep of one: from and to the same
echo "array task $k of $n: points $first to $((first + count - 1)) of $points, $part_from to $part_to"
status=0
"${program[@]}" run "$part" --headless --out "$out" || status=$?
rm -f "$part"
echo "photonoxide run exited $status"
exit "$status"
