#!/bin/bash
# The Slurm cluster in Docker, started and checked (#805, D6), then removed:
#
#   bash docker/slurm/test.sh <photonoxide image>
#
# 1. a headless run as a batch job;
# 2. a job array over a sweep's parts;
# 3. a farm: a worker on each of 3 nodes (srun), the --worker list from `scontrol show
#    hostnames`, the token in PHOTONOXIDE_FARM_TOKEN; its events.jsonl is the one-process run's,
#    to the bit;
# 4. a compute node killed mid-farm: its worker is dropped and the run finishes on the others,
#    with the same record;
# 5. the time limit: Slurm's SIGTERM ends the run with exit 4 and a whole, readable folder.
#
# The containers share this machine's cores, memory and network: this checks correctness,
# signals and the scheduler, never speed. KEEP=1 leaves the cluster up afterwards.
set -euo pipefail

image=${1:?usage: test.sh <photonoxide image>}
here=$(cd "$(dirname "$0")" && pwd)
project=${PHOTONOXIDE_SLURM_PROJECT:-photonoxide-slurm}
export PHOTONOXIDE_IMAGE=$image
compose() { docker compose -f "$here/compose.yaml" -p "$project" "$@"; }
# a command on the controller, as the submitting user, in the shared folder
on() { compose exec -T -u phx -w /work slurmctld "$@"; }
say() { echo "== $*"; }
fail() {
  echo "FAILED: $*" >&2
  exit 1
}

if [ "${KEEP:-}" != 1 ]; then
  trap 'compose down -v --remove-orphans >/dev/null 2>&1 || true' EXIT
fi
started=$(date +%s)
elapsed() { echo "$(($(date +%s) - started)) s"; }

say "building the nodes on $image"
compose build -q
compose up -d
say "waiting for the three nodes"
for _ in $(seq 1 90); do
  idle=$(on sinfo -h -N -t idle -o '%N' 2>/dev/null | sort -u | wc -l || true)
  [ "$idle" -ge 3 ] && break
  sleep 2
done
on sinfo -N -l
[ "$idle" -ge 3 ] || fail "the nodes didn't come up"

scripts=/opt/photonoxide/slurm
# the jobs: the 24 points of docker/jobs/sweep.toml; 120 of them for the node killed mid-farm;
# and many for the time limit, which stops it long before its end
on cp /opt/photonoxide/jobs/sweep.toml sweep.toml
on sh -c "sed 's/^points = 24/points = 120/; s/^name = \"sweep\"/name = \"sweep-120\"/' sweep.toml > sweep-120.toml"
on sh -c "sed 's/^points = 24/points = 20000/; s/^name = \"sweep\"/name = \"sweep-long\"/' sweep.toml > sweep-long.toml"

# the run's folder under runs/<case>/ (one run in it)
folder() { on sh -c "ls -d runs/$1/*/ | head -n 1"; }
# a run's record without the seconds its finished event gives
record() { on sh -c "sed -E '/^\\{\"type\":\"finished\"/ s/\"seconds\":[^,}]*/\"seconds\":null/' $1events.jsonl"; }
outcome() { on sh -c "grep -o '\"outcome\": *\"[a-z]*\"' $1results.json | sed 's/.*\"\\([a-z]*\\)\"/\\1/'"; }
points() { on sh -c "cat runs/$1/*/events.jsonl 2>/dev/null | grep -c '^{\"type\":\"s_parameters\"' || true"; }

say "1. a headless run as a batch job ($(elapsed))"
on sbatch --wait -o alone.out "$scripts/run.sh" sweep.toml runs/alone || fail "the batch job failed: $(on cat alone.out)"
alone=$(folder alone)
[ "$(outcome "$alone")" = finished ] || fail "the batch run didn't finish"
[ "$(points alone)" = 24 ] || fail "the batch run has $(points alone) points, not 24"
on cat alone.out

say "2. a job array over the sweep's 4 parts ($(elapsed))"
on sbatch --wait --array=0-3 -o 'array-%a.out' "$scripts/array.sh" sweep.toml runs/array || fail "the array failed: $(on sh -c 'cat array-*.out')"
on sh -c 'cat array-*.out'
parts=$(on sh -c 'ls -d runs/array/*/ | wc -l')
[ "$parts" = 4 ] || fail "the array made $parts runs, not 4"
for p in $(on sh -c 'ls -d runs/array/*/'); do
  [ "$(outcome "$p")" = finished ] || fail "$p didn't finish"
done
[ "$(points array)" = 24 ] || fail "the array's parts have $(points array) points, not 24"

say "3. a farm on the 3 nodes ($(elapsed))"
on env PHOTONOXIDE_FARM_TOKEN=a-token-for-the-cluster-test \
  sbatch --wait --nodes=3 -o farm.out "$scripts/farm.sh" sweep.toml runs/farm || fail "the farm failed: $(on cat farm.out)"
on cat farm.out
on grep -q 'farmed to 3 workers' farm.out || fail "the run wasn't farmed to 3 workers"
farm=$(folder farm)
[ "$(record "$farm")" = "$(record "$alone")" ] || fail "the farmed record isn't the one-process run's"
echo "the farmed run's events.jsonl is the one-process run's, to the bit"

say "4. a compute node killed mid-farm ($(elapsed))"
on sbatch --wait -o alone-120.out "$scripts/run.sh" sweep-120.toml runs/alone-120 || fail "the batch job failed: $(on cat alone-120.out)"
alone120=$(folder alone-120)
id=$(on sbatch --parsable --nodes=3 -o killed.out "$scripts/farm.sh" sweep-120.toml runs/killed)
for _ in $(seq 1 600); do
  [ "$(points killed)" -ge 10 ] && break
  sleep 0.2
done
before=$(points killed)
[ "$before" -ge 10 ] && [ "$before" -lt 120 ] || fail "the farm had $before points when the node was to be killed"
compose kill c3
echo "c3 killed at $before of 120 points"
for _ in $(seq 1 600); do
  on squeue -h -j "$id" 2>/dev/null | grep -q . || break
  sleep 1
done
on cat killed.out
on grep -q 'photonoxide run exited 0' killed.out || fail "the farm didn't finish after the node was killed"
killed=$(folder killed)
[ "$(record "$killed")" = "$(record "$alone120")" ] || fail "the record with a node killed isn't the one-process run's"
echo "the run finished on the other nodes, its events.jsonl the one-process run's, to the bit"

say "5. the time limit ($(elapsed))"
# a minute: Slurm sends SIGTERM at the limit, SIGKILL KillWait (30 s) later
on sbatch --wait --time=1 -o limit.out "$scripts/run.sh" sweep-long.toml runs/limit && fail "the job didn't reach its time limit"
on cat limit.out
on grep -q 'photonoxide run exited 4' limit.out || fail "the run didn't exit 4 at the time limit"
limit=$(folder limit)
[ "$(outcome "$limit")" = stopped ] || fail "results.json doesn't say stopped"
on sh -c "tail -n 1 ${limit}events.jsonl" | grep -q '"stopped":"requested"' || fail "the record doesn't end in its stopped event"
echo "stopped at the time limit after $(points limit) points, with exit 4 and a whole record"

say "all passed ($(elapsed))"
