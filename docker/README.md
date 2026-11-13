# photonoxide

[photonoxide](https://github.com/tachsin/photonoxide) is photonics design and simulation
software: waveguide modes, 2D and 3D FDFD and FDTD, eigenmode expansion, circuits and devices,
nonlinear optics and fabrication flows, each method validated against published results. This
image is the `photonoxide` program without its window: it runs jobs, serves as a farm worker,
writes job templates and schemas, exports a run's tables and runs the built-in examples and
validation. It is meant for a laptop, a cloud machine, a CI job, and a cluster through
Apptainer.

The studio's window is not in the image. For the window, install the program from the
[releases](https://github.com/tachsin/photonoxide/releases).

## Tags

| Tag | What it is |
|---|---|
| `0.7.0` | that release |
| `0.7` | the latest release of 0.7 |
| `latest` | the latest release |

Each release is built for `linux/amd64` and `linux/arm64`, with an SBOM and a provenance
attestation. The same tags are on GitHub's registry, `ghcr.io/tachsin/photonoxide`. No image is
made of unreleased code.

## Run a job

The image's working folder is `/work`. Mount yours there, and run as your own user, so the
files a run writes are yours:

```sh
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/work" tachsin/photonoxide run job.yaml --headless
```

The run's folder is `runs/<job>-<time>/` in your folder: its `events.jsonl`, `results.json` and
its tables, as [docs/runs.md](https://github.com/tachsin/photonoxide/blob/main/docs/runs.md)
describes. Without `--user` the run writes as the image's own user (uid 1000).

Nothing needs mounting to look around:

```sh
docker run --rm tachsin/photonoxide --version
docker run --rm tachsin/photonoxide example --list      # the built-in examples
docker run --rm tachsin/photonoxide new --list          # the job templates
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/work" tachsin/photonoxide new fdfd job.yaml
```

The image runs with a read-only root (`--read-only --tmpfs /tmp`): only the mounted folder is
written.

## Stopping a run

`docker stop` (SIGTERM) and Ctrl-C (SIGINT) stop a run at its next check: its record ends with
its `stopped` event, `results.json` says why, and the program exits 4, as when the studio's
window is closed early. A second signal ends it at once. Docker kills a container 10 s after
`docker stop` unless given more (`docker stop -t 60`): a solve that takes longer than that
between checks needs more time.

Exit codes: 0 finished, 1 failed, 2 usage, 3 stopped by its timeout, 4 stopped on request.

## Threads

The solvers use the CPUs the container is given: `--cpus 2` runs them on two threads.
`RAYON_NUM_THREADS` sets the number directly.

## Farm workers

A run can send a sweep's points to workers on other machines
([docs/farming.md](https://github.com/tachsin/photonoxide/blob/main/docs/farming.md)). A worker
in a container:

```sh
docker run --rm -p 7878:7878 -e PHOTONOXIDE_FARM_TOKEN=<token> tachsin/photonoxide worker --listen 0.0.0.0:7878
```

All workers must be the same photonoxide version as the run, so use the same tag everywhere. The
farm's traffic isn't encrypted: farm machines on a network you trust.

## On a cluster, with Apptainer

```sh
apptainer pull photonoxide.sif docker://tachsin/photonoxide:0.7.0
apptainer exec photonoxide.sif photonoxide run job.yaml --headless
```

Apptainer runs as you, in your current folder, so nothing needs mounting.
[docs/docker.md](https://github.com/tachsin/photonoxide/blob/main/docs/docker.md) and the
farming guide's "On a Slurm cluster" give a batch script for Slurm.

## What it doesn't do

- The studio's window, forwarded from a container: not supported.
- GPU solves: the image has no GPU support.
- Speed measurements on containers that share one machine: they share its cores and memory.

## Licence

photonoxide is licensed under either of the
[Apache License, Version 2.0](https://github.com/tachsin/photonoxide/blob/main/LICENSE-APACHE) or
the [MIT licence](https://github.com/tachsin/photonoxide/blob/main/LICENSE-MIT), at your option.
Both are in the image, in `/usr/share/doc/photonoxide/`.

## Publishing

Each release's image is built and pushed by the repository's workflow
(`.github/workflows/docker-publish.yml`, which the release workflow calls with the new tag):
both architectures are built on runners of their own and checked first (`docker/check.sh`: the
image's own checks, and the same jobs run natively giving the same bits), then pushed with their
SBOM and provenance. GitHub's registry needs no setup. Docker Hub needs, once:

1. A Docker Hub account, and a public repository in it, `photonoxide` (the namespace is the
   account's or an organization's).
2. A personal access token of that account (Account settings, Personal access tokens) with
   **Read, Write, Delete** access: Read & Write pushes the image, and Delete is what Docker Hub
   asks to update the repository's page from this file. With Read & Write alone, the push works
   and the page's text is pasted by hand.
3. In the GitHub repository's Settings, Secrets and variables, Actions: the secrets
   `DOCKERHUB_USERNAME` (the account's name) and `DOCKERHUB_TOKEN` (the token); and, when the
   repository isn't `tachsin/photonoxide`, the variable `DOCKERHUB_REPOSITORY` naming it
   (`<namespace>/photonoxide`).

Without the secrets, a release's workflow skips Docker Hub with a notice and pushes to GitHub's
registry alone. A release's image can be made again by running the workflow by hand with its
tag.
