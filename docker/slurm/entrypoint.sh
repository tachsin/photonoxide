#!/bin/sh
# A node of the cluster in Docker: munge first, then the daemon this container is for
# (`slurmctld` or `slurmd`), in the foreground so that it is the container's process.
set -eu
runuser -u munge -- /usr/sbin/munged --force
case "${1:-}" in
  slurmctld) exec runuser -u slurm -- /usr/sbin/slurmctld -D -i ;;
  # slurmd runs as root, as on any cluster: it starts the jobs as their users
  slurmd) exec /usr/sbin/slurmd -D ;;
  *) exec "$@" ;;
esac
