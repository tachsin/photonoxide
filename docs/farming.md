# Farming: a sweep's points and whole jobs on other processes

A sweep's points, a population of candidates, process corners and a Monte Carlo draw are
independent evaluations. photonoxide can send them to worker processes, on this machine or on
others over TCP, and gather the results by index ([`photonoxide::farm`](../src/farm.rs)): pure
Rust, nothing to install, and the same bits as a run in one process.

## Running workers

On this machine, the run starts its own workers and stops them when it ends:

```sh
photonoxide run jobs/mmi-fdfd.toml --workers 4             # four workers, the threads shared among them
photonoxide run jobs/mmi-fdfd.toml --workers 4 --headless  # the same without a window
```

On other machines, start a worker on each and give the run their addresses:

```sh
# on each worker machine (the token is a secret you choose; or set PHOTONOXIDE_FARM_TOKEN)
photonoxide worker --listen 0.0.0.0:7878 --token <token> [--slots 4]

# on yours
photonoxide run jobs/mmi-fdfd.toml --worker host1:7878 --worker host2:7878 --token <token>
```

Local and remote workers mix: `--workers 2 --worker host1:7878`. A worker serves one
coordinator after another until stopped (`--once`: one, then it exits); `--slots n` has it take
n tasks at once, each on its own thread (1 by default, which suits a worker with all of a
machine's threads to itself: photonoxide's solvers use them inside each point).
`--task-timeout <seconds>` bounds each point: one past it fails with that reason and its worker
is dropped (a local one killed), as one that may be stuck. The job's own `timeout_minutes`
bounds the whole run, farm included.

What is farmed is a `modes` or `fdfd` job's `[task.sweep]`; a job without one runs as it always
does, and the program says so. The window shows a farmed run live like any run: the run records
each point as it comes back, in the points' order, into `runs/<run>/events.jsonl`, which is all
the window follows.

All of a farm's workers must be the same photonoxide version as the run: the bits depend on
the build, and a worker of another version is refused. A job whose direct solver is `auto` (the
default) lets each worker choose from its own machine's benchmark records, which can differ from
the run's choice: such a worker is dropped at the sweep with that reason (below), so name the
solver in the job (`[solver] direct = "photonoxide"`) when the workers' machines differ.

## The same record

The farmed run's `events.jsonl` is the one the run writes on its own, to the last bit, on any
number of workers, after any retries; only the `finished` event's seconds differ, as between any
two runs. That holds because:

- every worker sets the job up as the run does (the structure, the cross-section, an FDFD sweep's
  first point, whose sparsity analysis the others reuse), and the run checks it: the worker
  sends a digest (FNV-1a) of the events its setup recorded, and the run compares it with its own
  before it hands out a point;
- photonoxide's solvers give the same bits on any number of threads, so a point solved on a
  worker with 2 threads is the point solved in the run with 20;
- a point's events travel as the lines the worker's run would have written, and are written as
  they are: no number is read and printed again on the way;
- the results are placed by index and recorded only as a complete prefix, so the order in which
  workers finish doesn't show.

The tests check it: a modes sweep and two 2D FDFD sweeps farmed to 1, 2, 4 and 8 workers (and to
3 of 2 slots each) against the same sweeps in one process; a worker whose connection breaks
while it has a point; a worker process killed while the program's run is under way
(`studio/src-tauri/tests/farm.rs`); jobs that finish out of order coming back in order; a point
that fails, a task past its timeout and a run past its deadline.

## The protocol

Version 1, over TCP. Each message is a 4-byte big-endian length followed by that many bytes of
JSON, at most 256 MiB.

| From | Message | Fields | What it does |
|---|---|---|---|
| coordinator | `hello` | `protocol`, `photonoxide`, `token` | the first message: the protocol's version, photonoxide's, and the token ("" for none) |
| worker | `welcome` | `protocol`, `photonoxide`, `slots` | accepted; how many tasks it takes at once |
| worker | `refused` | `reason` | another protocol, another photonoxide version, or the wrong token; it closes |
| coordinator | `open` | `job`, `format`, `seconds` | a sweep's job (its text and format) to set up once; `seconds`: the time left, if limited |
| worker | `ready` | `digest` | set up; the digest of the events the setup recorded |
| coordinator | `point` | `index` | solve the sweep's point `index` |
| coordinator | `run` | `index`, `job`, `format`, `seconds` | run a whole job, task `index` |
| worker | `events` | `index`, `events` | task or point `index`'s events, each the line `events.jsonl` has |
| worker | `failed` | `index`, `error` | task or point `index` failed (no index: the job couldn't be set up) |
| worker | `stopped` | `index` | its stop came first: the time left ran out, or its coordinator went away |
| coordinator | `close` | | no more tasks; the connection closes |

A coordinator keeps each worker busy with up to its `slots` tasks from one queue. A worker whose
connection closes or breaks is lost: its tasks go back into the queue for the others, and a task
that has lost three workers fails with that reason, so a point that crashes every worker can't
loop for ever. A task that fails is a result: in a sweep, as in a run in one process, the points
before it are recorded and the run fails with its error. With no worker left, the run fails and
names why each was lost.

## Populations and other jobs

`Farm::run_jobs` runs a list of whole jobs on the workers and returns their records in the list's
order, each a job's events or its error. A genoxide population goes through it with genoxide's
`Batch`, which hands the fitness function a whole generation at once: photonoxide supplies the
objective, genoxide the optimizer, and nothing photonics-specific is added to either.

```rust
use genoxide::prelude::*;
use photonoxide::farm::{Farm, FarmOptions};
use photonoxide::run::{Job, Stop as RunStop};

let mut farm = Farm::new(FarmOptions::default());
farm.spawn_local(std::path::Path::new("photonoxide"), 8, None)?;
let fitness = Batch(|genomes: &[&Reals]| {
    // each candidate a job with its parameters set
    let jobs: Vec<Job> = genomes.iter().map(|g| candidate(g)).collect();
    farm.run_jobs(&jobs, &RunStop::new(None))
        .expect("workers")
        .into_iter()
        .map(|record| record.ok().map(|lines| objective(&lines)))
        .collect::<Vec<Option<f64>>>()
});
let outcome = Engine::new(cmaes, fitness).stop_when(Stop::generations(50)).run()?;
```

A failed candidate is `None` here, an invalid solution to genoxide. The library's test
`a_genoxide_population_is_evaluated_on_the_farm` runs CMA-ES this way on a strip's width and
finds the same best, to the last bit, on 1 and 3 workers and with the candidates solved in one
process.

## Security

- **No remote code.** A worker parses a photonoxide job and runs one of the built-in kinds;
  nothing in a job runs a program, and a worker writes no files (it keeps no run directory).
- **This machine by default.** A worker listens on 127.0.0.1:7878 unless told otherwise, and
  refuses to listen on an address beyond this machine without a token. With a token, every
  coordinator must give it; local workers get a fresh random one from their run.
- **Not for untrusted networks.** The token is a shared secret sent in the clear, and the traffic
  (jobs and results) isn't encrypted or authenticated beyond it. Farm machines you control, on a
  network you trust, or through an SSH tunnel or a VPN: with the worker on the other machine's
  own 127.0.0.1:7878, `ssh -N -L 7879:127.0.0.1:7878 host` here, then `--worker 127.0.0.1:7879`. A worker reachable from the internet can be made to
  spend its time on anyone's jobs.

## Measured

The built-in MMI job (`jobs/mmi-fdfd.toml`) as a 2D FDFD wavelength sweep of 40 points from 1.5
to 1.6 µm, on its 20 nm grid: 725 × 300 cells (217 500 unknowns), photonoxide's own direct
solver named, run headless with the release build on an Intel Core Ultra 7 265K (20 threads,
Windows 11). The machine was shared with other work while it ran, so the times moved by up to 30%
between two rounds; the table gives the shorter of the two. The wall time is the program's, from
its start to its exit: starting the workers, every worker's setup and the first point included.

| Run | Workers | Threads each | Wall time | Against 1 worker | Against one process |
|---|---|---|---|---|---|
| one process (points side by side, 7 at a time by their memory) | — | 20 | 33.0 s | | 1 |
| farmed | 1 | 20 | 81.4 s | 1 | 0.41 |
| farmed | 2 | 10 | 42.2 s | 1.9 | 0.78 |
| farmed | 4 | 5 | 24.1 s | 3.4 | 1.37 |
| farmed | 8 | 2 | 20.3 s | 4.0 | 1.63 |

All ten runs wrote the same record (its MD5 with the `finished` seconds blanked, d36b4fd6989d…).
On one machine a run already solves its points side by side, so farming to this machine's own
processes gains only when more points run at once than the run's memory bound lets it (here 8
against 7, with less memory each: a worker holds one factorization), and loses with fewer
workers, each solving one point at a time. Farming pays across machines, where each adds its
cores and its memory.
