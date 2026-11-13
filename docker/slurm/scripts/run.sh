#!/bin/bash
# A headless run as a batch job, on one node:
#
#   sbatch run.sh job.yaml [runs]
#
# At the job's time limit Slurm sends SIGTERM: the run stops at its next check, keeps a whole
# record and exits 4, within the KillWait seconds Slurm leaves before SIGKILL (30 by default).
#SBATCH --job-name=photonoxide
#SBATCH --nodes=1
#SBATCH --ntasks=1
#SBATCH --output=run-%j.out
set -uo pipefail

job=${1:?usage: sbatch run.sh <job file> [<runs folder>]}
out=${2:-runs}
# the program: photonoxide, or a command that runs it ("apptainer exec photonoxide.sif photonoxide")
read -ra program <<<"${PHOTONOXIDE:-photonoxide}"
# the signal is the program's to act on; this script only says that it came, and the run's exit
trap 'echo "batch script: SIGTERM"' TERM
"${program[@]}" run "$job" --headless --out "$out" &
wait $!
status=$?
# a trapped signal interrupts the wait: wait again for the run's own exit
while kill -0 $! 2>/dev/null; do wait $!; status=$?; done
echo "photonoxide run exited $status"
exit "$status"
