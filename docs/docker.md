# Docker and Apptainer: the program in a container

The `photonoxide` program is also an image: the same program without its studio window, with
every other command (`run --headless`, `worker`, `new`, `schema`, `export`, `example`,
`validate`, `bench`, `libraries`). It runs jobs, serves as a farm worker and runs the built-in
examples and the validation on a laptop, a cloud machine or a CI job, and on a cluster through
Apptainer. Its page on Docker Hub is [docker/README.md](../docker/README.md).

| | |
|---|---|
| Docker Hub | `tachsin/photonoxide` |
| GitHub's registry | `ghcr.io/tachsin/photonoxide`, the same tags |
| Tags | `<version>` (`0.7.0`), `<minor>` (`0.7`) and `latest`, made for each release; no image of unreleased code |
| Architectures | `linux/amd64` and `linux/arm64` |
| Size (linux/amd64, 0.6.4's code) | 46.7 MB to download, 187 MB unpacked: Debian 13 slim (glibc), its CA certificates, and the program (48 MB) |
| Attestations | an SBOM and a provenance attestation for each architecture |
| User | `photonoxide` (uid 1000), not root; working folder `/work` |

The studio's window is not in the image, and a window forwarded from a container (X or
Wayland) isn't supported: for the window, install the program from a
[release](https://github.com/tachsin/photonoxide/releases). `photonoxide` alone and `photonoxide
view` say that this build has no window and exit 2.

## Running a job

Mount the folder that holds the job at `/work`, and run as your own user, so the files the run
writes are yours:

```sh
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/work" tachsin/photonoxide run job.yaml --headless
```

The run's folder is `runs/<time>-<job>/` in that folder, as [runs.md](runs.md) describes:
`events.jsonl`, `results.json` and the tables. Anything the job reads by a path (a layout file,
a material's data) must be in the mounted folder, with a path relative to it. Without `--user`,
the run writes as the image's own user, uid 1000, which on Linux owns the files it writes on
your side of the mount. On Docker Desktop (Windows, macOS) the mounted folder's files are yours
either way.

Nothing needs mounting to look around: `docker run --rm tachsin/photonoxide example --list`,
`new --list`, `--version`. Every command's usage: `docker run --rm tachsin/photonoxide --help`.

The image needs nothing written outside the mounted folder, so it runs with a read-only root:
`--read-only --tmpfs /tmp`.

### Stopping a run

`docker stop` sends SIGTERM, and Ctrl-C SIGINT. Either stops a run as closing its window does: at
its next check (between two points of a sweep, a solver's iterations, an FDTD time step), its
record ends with its `stopped` event, `results.json` says `"outcome": "stopped"` and
`"stopped": "requested"`, and the program exits 4. A second signal ends it at once. Docker
kills the container 10 s after `docker stop`; a run whose checks are further apart than that
needs more time, `docker stop -t 60`. A worker drops its tasks and exits 4 (its coordinator
drops it, as it drops a worker that was killed); any other command ends at once. The program
handles the signals itself as the container's first process, so it needs no init (`--init`).

The exit codes are the program's: 0 finished, 1 failed, 2 usage, 3 stopped by its timeout, 4
stopped on request.

### Threads

The solvers use as many threads as the CPUs the container may use: `--cpus 2` runs them on two,
and a run's `meta.json` says so (`"threads": 2`). `RAYON_NUM_THREADS` sets the number directly.
The results are the same bits on any number of threads.

## Farm workers in containers

A worker in a container listens on the container's port, published on the machine's
([farming.md](farming.md)):

```sh
docker run -d --rm -p 7878:7878 -e PHOTONOXIDE_FARM_TOKEN=<token> tachsin/photonoxide worker --listen 0.0.0.0:7878
photonoxide run job.yaml --headless --worker host1:7878 --worker host2:7878 --token <token>
```

The run and its workers must be the same photonoxide version, so name the same tag everywhere.
The farm's traffic isn't encrypted: farm containers on a network you trust.

## On a cluster, with Apptainer

Clusters run containers with Apptainer (formerly Singularity), as your own user, with your home
and current folders mounted:

```sh
apptainer pull photonoxide.sif docker://tachsin/photonoxide:0.7.0
apptainer exec photonoxide.sif photonoxide run job.yaml --headless
```

The `.sif` file is the image, kept on the cluster's shared file system; `apptainer exec
docker-archive:photonoxide.tar photonoxide ...` runs an image saved by `docker save` without a
registry. In a Slurm batch script, give the program as `apptainer exec photonoxide.sif
photonoxide` to the scripts of [farming.md, "On a Slurm cluster"](farming.md#on-a-slurm-cluster)
(their `PHOTONOXIDE` variable), and a farm runs the same way.

## Building the image

From the repository's root:

```sh
docker build -f docker/Dockerfile -t photonoxide .
```

The builder is the Rust that `rust-toolchain.toml` pins (the build fails if the Dockerfile's
`RUST_VERSION` differs: pass `--build-arg RUST_VERSION=<that version>`), and it builds
`cargo build --locked --release -p photonoxide-studio --no-default-features`: the program
without its `window` feature, which links no GTK or WebKit library. `--build-arg
PHOTONOXIDE_COMMIT=$(git rev-parse HEAD)` gives `--version` the commit, since the build's context
has no `.git`. On a machine of few cores, `--build-arg CARGO_BUILD_JOBS=4` builds with fewer
compiles at once.

Debian slim with glibc, not a static musl build: Rust's `f64` functions (`sin`, `exp`, ...) call
the platform's libm, and musl's could give other bits than the Linux packages give.

## What CI checks

On every pull request that touches the image, the program's crate or the manifests, and weekly
(`.github/workflows/docker.yml`); and before each release's image is pushed, on each
architecture (`.github/workflows/docker-publish.yml`):

- the program built without the window has no GTK, WebKit or Tauri crate in its tree and links
  no such library (`ldd`), and the program's tests pass on that build;
- the image builds; `example --list` and `new --list` run with nothing mounted;
- it runs as its own user, and as the runner's user with a read-only root, and the files a run
  writes are that user's;
- three of `photonoxide new`'s templates (`modes`, `fdfd`, `fdtd`) run in the image give the
  same `results.json`, `events.jsonl`, `events.bin` and tables, to the bit, as the same jobs run
  by the program built natively on the runner (Ubuntu's glibc against Debian's);
- `--cpus 2` gives the run 2 threads (`meta.json`, and the process's own count of threads);
- `docker stop` mid-sweep ends the run within its grace period, with exit 4 and a whole record;
- Apptainer runs the same three jobs from the image's archive, to the same bits;
- the Slurm cluster in Docker, below.

The checks are `docker/check.sh`, which runs them on any machine with Docker.

## A Slurm cluster in Docker

`docker/slurm/` is a cluster on one machine: a controller and three compute nodes, each a
container with munge, Debian's own Slurm (`slurm-wlm`) and the image's program, sharing a
folder. It is for correctness, signals and the scheduler, before the scripts meet real
hardware:

```sh
docker build -f docker/Dockerfile -t photonoxide:dev .
bash docker/slurm/test.sh photonoxide:dev
```

The test starts the cluster, runs a headless batch job, a job array over a sweep's 4 parts, a
farm with a worker on each of the 3 nodes (whose `events.jsonl` must be the one-process run's,
to the bit), a farm with a compute node killed mid-run (its worker dropped, the run finished on
the other two, the same record), and a run past its time limit (Slurm's SIGTERM, exit 4, a whole
record), then removes the cluster (`KEEP=1` leaves it up, to try things by hand:
`docker compose -f docker/slurm/compose.yaml -p photonoxide-slurm exec -u phx -w /work slurmctld bash`).

The containers share one machine's cores, memory and network, so the cluster tells nothing
about speed: scaling over machines is measured on a real cluster, never on this one.
