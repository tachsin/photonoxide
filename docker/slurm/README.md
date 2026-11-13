# A Slurm cluster in Docker

A controller (`slurmctld`) and three compute nodes (`c1`, `c2`, `c3`), each a container of 2 CPUs
with munge, Debian's own Slurm (`slurm-wlm`) and the photonoxide image's program, sharing the
folder `/work`. It tests the batch scripts in [scripts/](scripts) and the program under a
scheduler before they meet a real cluster: correctness, signals and the scheduler. The
containers share one machine's cores, memory and network, so it says nothing about speed.

```sh
docker build -f docker/Dockerfile -t photonoxide:dev .   # the image, from the repository's root
bash docker/slurm/test.sh photonoxide:dev                # start, check, remove
KEEP=1 bash docker/slurm/test.sh photonoxide:dev         # and leave it up afterwards
docker compose -f docker/slurm/compose.yaml -p photonoxide-slurm exec -u phx -w /work slurmctld bash
docker compose -f docker/slurm/compose.yaml -p photonoxide-slurm down -v
```

`test.sh` runs, and checks:

1. `run.sh`, a headless run as a batch job;
2. `array.sh`, a job array over a sweep's 4 parts;
3. `farm.sh`, a farm with a worker on each of the 3 nodes: its `events.jsonl` is the batch job's,
   to the bit;
4. the same with `c3` killed mid-run: its worker is dropped and the run finishes on `c1` and
   `c2`, with the same record;
5. a run past its time limit: Slurm's SIGTERM stops it with exit 4 and a whole record.

The files: [Dockerfile](Dockerfile) (one image for every node), [compose.yaml](compose.yaml),
[slurm.conf](slurm.conf) (no accounting and no cgroups), [entrypoint.sh](entrypoint.sh) (munge,
then the node's daemon). The scripts in [scripts/](scripts) are the recipes of
[docs/farming.md, "On a Slurm cluster"](../../docs/farming.md#on-a-slurm-cluster) as they are.
