#!/bin/bash
# A farmed run on a Slurm allocation: a photonoxide worker on every node of the job (srun), and
# the run, on the job's first node, sending its sweep's points to them. The recipe for a real
# cluster too (docs/farming.md, "On a Slurm cluster"):
#
#   sbatch --nodes=3 farm.sh job.yaml [runs]
#
# The token is PHOTONOXIDE_FARM_TOKEN when set, a fresh random one otherwise; it reaches the
# workers through the environment srun passes on, never a command line. The workers and the run
# must be the same photonoxide version. The farm's traffic isn't encrypted: keep it inside the
# cluster's own network.
#SBATCH --job-name=photonoxide-farm
#SBATCH --ntasks-per-node=1
#SBATCH --output=farm-%j.out
set -euo pipefail

job=${1:?usage: sbatch farm.sh <job file> [<runs folder>]}
out=${2:-runs}
port=${PHOTONOXIDE_FARM_PORT:-7878}
program=${PHOTONOXIDE:-photonoxide}
export PHOTONOXIDE_FARM_TOKEN=${PHOTONOXIDE_FARM_TOKEN:-$(od -An -N16 -tx1 /dev/urandom | tr -d ' \n')}

# a worker on each node, its first line ("listening on ...") in a file of its own
logs=$(mktemp -d "${TMPDIR:-/tmp}/photonoxide-farm.XXXXXX")
srun --ntasks-per-node=1 --kill-on-bad-exit=0 --output="$logs/worker-%N.out" \
  "$program" worker --listen "0.0.0.0:$port" &
workers=$!

# the run's --worker list: every node of the allocation
nodes=$(scontrol show hostnames "$SLURM_JOB_NODELIST")
args=()
for node in $nodes; do args+=(--worker "$node:$port"); done

# wait until each worker listens (at most a minute)
want=$(wc -w <<<"$nodes")
for _ in $(seq 1 120); do
  have=$(cat "$logs"/worker-*.out 2>/dev/null | grep -c '^listening on' || true)
  [ "$have" -ge "$want" ] && break
  sleep 0.5
done
if [ "$have" -lt "$want" ]; then
  echo "only $have of $want workers listen" >&2
  kill "$workers" 2>/dev/null || true
  exit 1
fi

status=0
"$program" run "$job" --headless --out "$out" "${args[@]}" || status=$?
echo "photonoxide run exited $status"
# the workers stop (each exits 4 on the signal srun passes on)
kill "$workers" 2>/dev/null || true
wait "$workers" 2>/dev/null || true
rm -rf "$logs"
exit "$status"
