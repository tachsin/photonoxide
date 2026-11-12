#!/bin/bash
# The image's checks (#805, D5), for .github/workflows/docker.yml and docker-publish.yml, and to
# run by hand:
#
#   bash docker/check.sh native <program> <out>       the built-in jobs, run by a native build
#   bash docker/check.sh image <image> <out>          the image's checks, and the same jobs in it
#   bash docker/check.sh apptainer <image ref> <out>  the same jobs through Apptainer
#   bash docker/check.sh compare <out> <out>          two such outs hold the same bits
#
# The built-in jobs are three of `photonoxide new`'s templates (modes, fdfd and fdtd): each run's
# record (events.jsonl, with the seconds its finished event gives blanked, and events.bin), its
# results.json (without its seconds) and its tables go into <out>/<template>/, to be compared
# byte for byte: a number written the same is the same f64, to the bit.
set -euo pipefail

TEMPLATES=(modes fdfd fdtd)
# docker/jobs/sweep.toml: a sweep long enough for a signal to come mid-run
here=$(cd "$(dirname "$0")" && pwd)

say() { echo "== $*"; }
fail() {
  echo "FAILED: $*" >&2
  exit 1
}

# Runs the templates with `$@` as the program (in the current folder, which is /work for a
# container), then keeps what is compared in $out.
run_templates() {
  local out=$1
  shift
  for t in "${TEMPLATES[@]}"; do
    rm -f "$t.yaml"
    "$@" new "$t" "$t.yaml" >/dev/null
    local started=$SECONDS
    "$@" run "$t.yaml" --headless --out "runs-$t" >/dev/null
    local dir
    dir=$(ls -d "runs-$t"/*/ | head -n 1)
    mkdir -p "$out/$t"
    sed -E '/^\{"type":"finished"/ s/"seconds":[^,}]*/"seconds":null/' "${dir}events.jsonl" >"$out/$t/events.jsonl"
    # results.json's seconds, at its top level, on a line of its own
    grep -v '^  "seconds": ' "${dir}results.json" >"$out/$t/results.json"
    for f in "${dir}"events.bin "${dir}"*.csv; do
      [ -e "$f" ] && cp "$f" "$out/$t/"
    done
    echo "$t: $(grep -c . "$out/$t/events.jsonl") events in $((SECONDS - started)) s"
  done
}

# A fresh folder of this user's, for a run's files.
workdir() {
  local d
  d=$(mktemp -d "${TMPDIR:-/tmp}/photonoxide-check.XXXXXX")
  chmod 0755 "$d"
  echo "$d"
}

native() {
  local program out
  program=$(realpath "$1")
  out=$(realpath -m "$2")
  mkdir -p "$out"
  local w
  w=$(workdir)
  say "the built-in jobs, natively ($("$program" --version))"
  (cd "$w" && run_templates "$out" "$program")
  rm -rf "$w"
}

apptainer_runs() {
  local ref=$1 out
  out=$(realpath -m "$2")
  mkdir -p "$out"
  local w
  w=$(workdir)
  say "the built-in jobs, through Apptainer ($ref)"
  # Apptainer runs as this user, in this folder, which it binds
  (cd "$w" && run_templates "$out" apptainer exec "$ref" photonoxide)
  rm -rf "$w"
}

image() {
  local image=$1 out
  out=$(realpath -m "$2")
  mkdir -p "$out"
  local me
  me="$(id -u):$(id -g)"

  say "the image: $(docker image inspect "$image" --format '{{.Size}}') bytes"
  docker run --rm "$image" --version

  say "its own user, not root"
  local user
  user=$(docker run --rm --entrypoint id "$image" -u)
  [ "$user" != 0 ] || fail "the image runs as root"
  echo "uid $user"

  say "no window library linked"
  docker run --rm --entrypoint ldd "$image" /usr/local/bin/photonoxide
  if docker run --rm --entrypoint ldd "$image" /usr/local/bin/photonoxide | grep -i -E 'gtk|webkit|soup|gdk|glib|cairo|pango'; then
    fail "the image's program links a window library"
  fi

  say "no window: photonoxide alone says so, exit 2"
  local status=0
  docker run --rm "$image" 2>/dev/null || status=$?
  [ "$status" = 2 ] || fail "photonoxide alone exited $status, not 2"

  say "the lists, with nothing mounted"
  local n
  n=$(docker run --rm "$image" example --list | wc -l)
  [ "$n" -gt 10 ] || fail "example --list lists $n examples"
  echo "example --list: $n examples"
  n=$(docker run --rm "$image" new --list | wc -l)
  [ "$n" -gt 5 ] || fail "new --list lists $n templates"
  echo "new --list: $n templates"

  say "the built-in jobs, as this user ($me), with a read-only root"
  local w
  w=$(workdir)
  (cd "$w" && run_templates "$out" docker run --rm --read-only --tmpfs /tmp --user "$me" -v "$w:/work" "$image")
  local owner
  owner=$(stat -c '%u:%g' "$(ls -d "$w"/runs-fdfd/*/ | head -n 1)results.json")
  [ "$owner" = "$me" ] || fail "the run's files are $owner's, not $me's"
  echo "the run's files are this user's ($owner)"
  rm -rf "$w"

  say "--cpus 2: the solvers' threads"
  w=$(workdir)
  cp "$here/jobs/sweep.toml" "$w/"
  # the threads the run was offered (meta.json), and the program's threads while it solves:
  # rayon's pool, sized by the CPUs the container may use, and a handful of the program's own
  local threads=()
  for cpus in 2 0; do
    local name="photonoxide-check-cpus-$cpus$$"
    local limit=()
    [ "$cpus" = 0 ] || limit=(--cpus "$cpus")
    docker run -d --name "$name" "${limit[@]}" --user "$me" -v "$w:/work" "$image" \
      run sweep.toml --headless --out "runs-cpus-$cpus" >/dev/null
    local most=0
    for _ in $(seq 1 100); do
      local now
      now=$(docker exec "$name" sh -c 'grep "^Threads:" /proc/1/status' 2>/dev/null | awk '{print $2}' || true)
      [ -n "$now" ] && [ "$now" -gt "$most" ] && most=$now
      [ "$(docker inspect -f '{{.State.Running}}' "$name")" = true ] || break
      sleep 0.1
    done
    docker wait "$name" >/dev/null
    docker rm "$name" >/dev/null
    local offered
    offered=$(grep -o '"threads": *[0-9]*' "$w"/runs-cpus-$cpus/*/meta.json | grep -o '[0-9]*$')
    echo "--cpus ${cpus/#0/(none)}: offered $offered threads, at most $most threads in the process"
    threads+=("$offered:$most")
  done
  [ "${threads[0]%%:*}" = 2 ] || fail "--cpus 2 offered ${threads[0]%%:*} threads, not 2"
  local all
  all=$(nproc)
  if [ "$all" -gt 2 ]; then
    # the process with --cpus 2 has the machine's count fewer threads, less 2
    [ $(( ${threads[1]#*:} - ${threads[0]#*:} )) = $((all - 2)) ] ||
      fail "--cpus 2 ran ${threads[0]#*:} threads and no limit ${threads[1]#*:}, on $all CPUs"
  fi
  rm -rf "$w"

  say "docker stop mid-run: exit 4 within its grace period, and a whole record"
  w=$(workdir)
  sed 's/^points = 24/points = 2000/' "$here/jobs/sweep.toml" >"$w/sweep.toml"
  local name="photonoxide-check-stop-$$"
  docker run -d --name "$name" --read-only --tmpfs /tmp --user "$me" -v "$w:/work" "$image" \
    run sweep.toml --headless --out runs >/dev/null
  for _ in $(seq 1 600); do
    [ "$(cat "$w"/runs/*/events.jsonl 2>/dev/null | grep -c '^{"type":"s_parameters"' || true)" -ge 3 ] && break
    sleep 0.1
  done
  local before=$SECONDS
  docker stop -t 30 "$name" >/dev/null
  local took=$((SECONDS - before)) code
  code=$(docker wait "$name")
  docker logs "$name" 2>&1 | tail -n 3
  docker rm "$name" >/dev/null
  [ "$code" = 4 ] || fail "docker stop ended the run with exit $code, not 4"
  [ "$took" -lt 30 ] || fail "the run took $took s to stop: past its grace period"
  local dir
  dir=$(ls -d "$w"/runs/*/ | head -n 1)
  tail -n 1 "${dir}events.jsonl" | grep -q '"type":"finished","stopped":"requested"' ||
    fail "the record doesn't end in its stopped event"
  grep -q '"outcome": "stopped"' "${dir}results.json" || fail "results.json doesn't say stopped"
  echo "stopped in $took s with exit 4, after $(grep -c '^{"type":"s_parameters"' "${dir}events.jsonl") of 2000 points"
  rm -rf "$w"
}

compare() {
  local a=$1 b=$2 differ=0
  for t in "${TEMPLATES[@]}"; do
    for f in "$a/$t"/*; do
      local g
      g="$b/$t/$(basename "$f")"
      if [ ! -e "$g" ]; then
        echo "$t: $(basename "$f") only in $a"
        differ=1
      elif cmp -s "$f" "$g"; then
        echo "$t: $(basename "$f") the same"
      else
        echo "$t: $(basename "$f") differs"
        diff <(head -c 4000 "$f") <(head -c 4000 "$g") | head -n 20 || true
        differ=1
      fi
    done
  done
  [ "$differ" = 0 ] || fail "$a and $b differ"
  say "$a and $b: the same bits"
}

case "${1:-}" in
  native) native "${2:?program}" "${3:?out}" ;;
  image) image "${2:?image}" "${3:?out}" ;;
  apptainer) apptainer_runs "${2:?image reference}" "${3:?out}" ;;
  compare) compare "${2:?out}" "${3:?out}" ;;
  *)
    sed -n '2,13p' "$0" >&2
    exit 2
    ;;
esac
