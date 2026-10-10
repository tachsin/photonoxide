//! Farming: independent evaluations sent to worker processes, on this machine or on others over
//! TCP, and gathered by index.
//!
//! A worker is a `photonoxide worker` process ([`Server`]); a coordinator is a [`Farm`] of
//! them. What they farm is photonoxide jobs and nothing else, in two forms:
//!
//! - **A sweep's points** ([`Farm::farm_sweep`]). A `modes` or `fdfd` job with a `[task.sweep]`
//!   hands its points to the workers instead of rayon's threads. Each worker sets the job up
//!   once, as the run does (the structure, the cross-section, an FDFD sweep's first point, whose
//!   sparsity analysis the others reuse), then solves the points it is sent. The run records
//!   each point's events in the points' order as they complete, so a window follows a farmed
//!   run as it follows any run, and the record is the one the run writes on its own, to the
//!   last bit.
//! - **Whole jobs** ([`Farm::run_jobs`]): a population of candidates, process corners, a Monte
//!   Carlo draw, each a job with its parameters set; their records come back in the jobs'
//!   order. A genoxide population is evaluated this way, by a batch fitness function
//!   (genoxide's `Batch`) that turns each genome into a job.
//!
//! **The protocol** (version [`PROTOCOL`]): over TCP, each message a 4-byte big-endian length
//! then that many bytes of JSON, at most 256 MiB. The coordinator says hello (its protocol,
//! photonoxide version and token); the worker welcomes it (and says how many tasks it takes at
//! once) or refuses it, as it does a coordinator of another protocol or another photonoxide
//! version, whose bits might differ. Then either a sweep (`open` a job, the worker's `ready` with
//! a digest of what its setup recorded, then `point`s answered by `events`, `failed` or
//! `stopped`) or whole jobs (`run`s answered the same way), and `close`. Events travel as the
//! lines `events.jsonl` has, written as they came: no number is read and written again on the
//! way.
//!
//! **Scheduling.** Each worker is kept busy with up to its slots of tasks from one queue. A
//! worker that dies or breaks off: its tasks go back into the queue for the others, and a task
//! that has lost [`FarmOptions::attempts`] workers fails with that reason. A task that fails is
//! a result, its error: in a sweep, as in one run, the points before it are recorded and the run
//! fails with it. A task past [`FarmOptions::task_timeout`] fails, and its worker is dropped (a
//! local one killed). The run's stop is the farm's: at it the points done are recorded in order
//! and the run ends as any run stopped does; a worker is sent the time left, and stops by itself
//! at it or when its coordinator goes away.
//!
//! **The same bits.** Results are placed by index and a sweep's are recorded only as a complete
//! prefix, so the record doesn't depend on which worker finished first, on their number, or on
//! retries. photonoxide's solvers give the same bits on any number of threads, and at the sweep
//! each worker's setup is checked against the run's: a worker whose record before the sweep
//! differs (`auto` chose another direct solver there) is dropped with that reason.
//!
//! **Security.** A worker runs photonoxide jobs, never code: it parses a job and runs one of the
//! built-in kinds, and writes no files. It listens on 127.0.0.1 unless told otherwise, and an
//! address beyond this machine needs a token, which every coordinator must then give. The token
//! is sent in the clear and the traffic isn't encrypted: farm only machines you control, on a
//! network you trust (or through an SSH tunnel or a VPN), never on an untrusted network.
//!
//! The `photonoxide` program runs workers (`photonoxide worker --listen <address>`) and farms a
//! run's sweep (`photonoxide run <job> --workers <n> --worker <address>`); see the docs'
//! `farming.md`.

mod coordinator;
mod protocol;
#[cfg(test)]
mod tests;
mod worker;

use std::time::Duration;

pub use coordinator::{Farm, FarmOptions};
pub use protocol::PROTOCOL;
pub use worker::{ServeOptions, Served, Server};

/// The address a worker listens on by default: this machine only.
pub const DEFAULT_ADDRESS: &str = "127.0.0.1:7878";

/// The photonoxide version, which a coordinator and its workers must share.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How long a connection and its hello may take.
const HANDSHAKE: Duration = Duration::from_secs(10);

fn farm_error(reason: impl Into<String>) -> crate::Error {
    crate::Error::Farm {
        reason: reason.into(),
    }
}

/// Whether two tokens are the same, in a time that doesn't depend on where they first differ.
fn same_token(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut differ = a.len() ^ b.len();
    for k in 0..a.len().max(b.len()) {
        let x = a.get(k).copied().unwrap_or(0);
        let y = b.get(k).copied().unwrap_or(0);
        differ |= usize::from(x ^ y);
    }
    differ == 0
}
