//! Telling a running job to stop: at a deadline, or when asked (a window closed, a user
//! cancelled). Every long loop checks a [`Stop`]. [`crate::run`] re-exports it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Why a run stopped early.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    /// Its deadline passed.
    Timeout,
    /// It was asked to stop.
    Requested,
}

/// Tells a running job to stop: at its deadline, or when [`Stop::request`] is called. Clones
/// share the request, so a window can hold one and the solver another.
#[derive(Clone, Debug)]
pub struct Stop {
    requested: Arc<AtomicBool>,
    deadline: Option<Instant>,
}

impl Stop {
    /// A stop at `timeout` from now, if any, or on request.
    pub fn new(timeout: Option<Duration>) -> Stop {
        Stop {
            requested: Arc::new(AtomicBool::new(false)),
            deadline: timeout.map(|t| Instant::now() + t),
        }
    }

    /// Asks the job to stop.
    pub fn request(&self) {
        self.requested.store(true, Ordering::Relaxed);
    }

    /// The same stop with a deadline at `timeout` from now, if that is sooner than its own: a
    /// request to either is a request to both. A caller's stop, held to a job's own
    /// `timeout_minutes` as well.
    #[must_use]
    pub fn within(&self, timeout: Option<Duration>) -> Stop {
        let other = timeout.map(|t| Instant::now() + t);
        Stop {
            requested: Arc::clone(&self.requested),
            deadline: match (self.deadline, other) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            },
        }
    }

    /// The time left before the deadline, if there is one (zero once it has passed).
    pub fn remaining(&self) -> Option<Duration> {
        self.deadline
            .map(|d| d.saturating_duration_since(Instant::now()))
    }

    /// Why the job should stop now, or `None` to go on.
    pub fn reason(&self) -> Option<StopReason> {
        if self.requested.load(Ordering::Relaxed) {
            Some(StopReason::Requested)
        } else if self.deadline.is_some_and(|d| Instant::now() >= d) {
            Some(StopReason::Timeout)
        } else {
            None
        }
    }
}
