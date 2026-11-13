//! SIGTERM and SIGINT, on Unix: `docker stop`, a scheduler's time limit (Slurm sends SIGTERM)
//! and Ctrl-C. A run stops at its next check as it does when its window closes: its record ends
//! with the `stopped` event, `results.json` says why, and it exits 4. A worker drops its tasks
//! and exits 4: its coordinator drops it, as it drops a worker that was killed. Every other
//! command ends at once, 128 + the signal's number, as a shell's `kill` would end it; in a
//! container that matters, since the kernel delivers no signal to a PID 1 that has no handler
//! for it, and `docker stop` would otherwise wait out its grace period and kill it.
//!
//! Windows has no SIGTERM: there, nothing here does anything, and Ctrl-C ends a program as
//! before.

use std::sync::Mutex;

use photonoxide::run::Stop;

/// What a signal does now (read only where there are signals).
#[cfg_attr(not(unix), allow(dead_code))]
enum On {
    /// Ends the program at once: with this code and this line, or 128 + the signal.
    Exit(Option<(u8, &'static str)>),
    /// Asks the run to stop; a second signal ends it at once.
    Stop(Stop),
    /// Nothing: a worker a run started, which ends with its run.
    Ignore,
}

static ON: Mutex<On> = Mutex::new(On::Exit(None));

fn set(on: On) {
    *ON.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = on;
}

/// From now on the first SIGTERM or SIGINT asks `stop` to stop, and a second ends the program
/// at once.
pub fn stop_run(stop: &Stop) {
    set(On::Stop(stop.clone()));
}

/// From now on a signal ends this worker at once, its tasks dropped, with exit code 4.
pub fn end_worker() {
    set(On::Exit(Some((
        4,
        "the worker stopped: its tasks are dropped, and its coordinator drops it",
    ))));
}

/// From now on signals do nothing: a worker started by a run (`--attached`), which ends when
/// its run closes its standard input; Ctrl-C reaches it too, and its run is the one to stop.
pub fn ignore() {
    set(On::Ignore);
}

/// Starts listening for SIGTERM and SIGINT, on a thread of its own. Called once, first thing.
#[cfg(unix)]
pub fn listen() {
    use signal_hook::consts::{SIGINT, SIGTERM};
    use signal_hook::iterator::Signals;
    let mut signals = match Signals::new([SIGTERM, SIGINT]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("warning: SIGTERM and SIGINT are not handled: {e}");
            return;
        }
    };
    std::thread::spawn(move || {
        let mut asked = false;
        for signal in signals.forever() {
            let name = if signal == SIGTERM {
                "SIGTERM"
            } else {
                "SIGINT"
            };
            let at_once = u8::try_from(128 + signal).unwrap_or(u8::MAX);
            let on = ON.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            match &*on {
                On::Ignore => {}
                On::Stop(stop) if !asked => {
                    asked = true;
                    eprintln!(
                        "{name}: the run stops at its next check and keeps its record (again: \
                         it ends at once)"
                    );
                    stop.request();
                }
                On::Stop(_) => {
                    eprintln!("{name} again: the run ends at once");
                    std::process::exit(at_once.into());
                }
                On::Exit(Some((code, line))) => {
                    eprintln!("{name}: {line}");
                    std::process::exit((*code).into());
                }
                On::Exit(None) => std::process::exit(at_once.into()),
            }
        }
    });
}

/// Windows has no SIGTERM: nothing to listen for.
#[cfg(not(unix))]
pub fn listen() {}
