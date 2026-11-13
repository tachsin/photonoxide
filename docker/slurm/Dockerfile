# syntax=docker/dockerfile:1
#
# A Slurm node for the cluster in Docker (compose.yaml): Debian's own slurm-wlm and munge, and
# the photonoxide image's program. One image for the controller and the compute nodes; the
# entrypoint starts the daemon its first argument names. Built from the repository's root, with
# the photonoxide image to take the program from:
#
#   PHOTONOXIDE_IMAGE=photonoxide:dev docker compose -f docker/slurm/compose.yaml build
#
# For tests of correctness, signals and the scheduler on one machine (docker/slurm/README.md):
# never for speed.

ARG IMAGE=photonoxide:dev
ARG DEBIAN=trixie

FROM ${IMAGE} AS photonoxide

FROM debian:${DEBIAN}-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends slurm-wlm munge ca-certificates procps \
    && rm -rf /var/lib/apt/lists/*
COPY --from=photonoxide /usr/local/bin/photonoxide /usr/local/bin/photonoxide
# one munge key for every node: they are all this image
RUN mkdir -p /etc/munge /run/munge /var/log/munge /var/lib/munge \
    && dd if=/dev/urandom of=/etc/munge/munge.key bs=1 count=1024 status=none \
    && chown -R munge:munge /etc/munge /run/munge /var/log/munge /var/lib/munge \
    && chmod 0700 /etc/munge /var/log/munge /var/lib/munge \
    && chmod 0400 /etc/munge/munge.key
RUN mkdir -p /var/spool/slurmctld /var/spool/slurmd /var/log/slurm /run/slurm \
    && chown -R slurm:slurm /var/spool/slurmctld /var/spool/slurmd /var/log/slurm /run/slurm
COPY docker/slurm/slurm.conf docker/slurm/cgroup.conf /etc/slurm/
COPY docker/slurm/entrypoint.sh /usr/local/sbin/entrypoint.sh
# the batch scripts and the job they run, beside the shared folder
COPY docker/slurm/scripts/ /opt/photonoxide/slurm/
COPY docker/jobs/ /opt/photonoxide/jobs/
# the user who submits; /work is the folder every node shares (a volume)
RUN useradd --uid 1000 --user-group --create-home --shell /bin/bash phx \
    && mkdir -p /work && chown phx:phx /work
ENTRYPOINT ["/usr/local/sbin/entrypoint.sh"]
