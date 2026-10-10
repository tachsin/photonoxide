//! `photonoxide worker`, and the options that farm a run's sweep (photonoxide::farm).

use std::io::Read;
use std::process::ExitCode;
use std::time::Duration;

use photonoxide::farm::{DEFAULT_ADDRESS, Farm, FarmOptions, ServeOptions, Server};

/// The environment variable a token can come from, so that it needn't be on a command line.
pub const TOKEN_VARIABLE: &str = "PHOTONOXIDE_FARM_TOKEN";

/// `photonoxide worker [--listen <address>] [--token <token>] [--slots <n>] [--once]
/// [--attached]`.
pub fn worker(args: &[String]) -> ExitCode {
    let mut address = DEFAULT_ADDRESS.to_owned();
    let mut token = std::env::var(TOKEN_VARIABLE).ok().filter(|t| !t.is_empty());
    let mut slots = 1;
    let mut once = false;
    let mut attached = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--listen" => match it.next() {
                Some(v) => address.clone_from(v),
                None => return super::usage(),
            },
            "--token" => match it.next() {
                Some(v) => token = Some(v.clone()),
                None => return super::usage(),
            },
            "--slots" => match it.next().and_then(|v| v.parse::<usize>().ok()) {
                Some(n) if n > 0 => slots = n,
                _ => return super::usage(),
            },
            "--once" => once = true,
            "--attached" => attached = true,
            _ => return super::usage(),
        }
    }
    let server = match Server::bind(
        &address,
        ServeOptions {
            token,
            slots,
            prepare: Some(crate::auto::prepare),
        },
    ) {
        Ok(s) => s,
        Err(e) => return super::fail(e),
    };
    let here = match server.address() {
        Ok(a) => a,
        Err(e) => return super::fail(e),
    };
    // the first line on standard output says where it listens: what a coordinator that
    // started it reads
    println!("listening on {here}");
    if attached {
        // started by a coordinator: it ends when the coordinator does, whose end closes the
        // pipe to its standard input
        std::thread::spawn(|| {
            let mut sink = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut sink);
            std::process::exit(0);
        });
    }
    loop {
        match server.serve_one() {
            Ok(served) => match &served.problem {
                // (a worker started by a run says only what went wrong, on the run's stderr)
                None if attached => {}
                None => eprintln!("{}: {} tasks", served.peer, served.tasks),
                Some(p) => eprintln!("{}: {} tasks; {p}", served.peer, served.tasks),
            },
            Err(e) => return super::fail(e),
        }
        if once {
            return ExitCode::SUCCESS;
        }
    }
}

/// How `photonoxide run` farms its sweep: local workers, remote ones, and their options.
#[derive(Debug, Default)]
pub struct Options {
    pub workers: usize,
    pub remote: Vec<String>,
    pub token: Option<String>,
    pub task_timeout: Option<Duration>,
}

impl Options {
    /// Reads a farm option of `photonoxide run` at `a` (taking its value from `it`): `Some(true)`
    /// when it was one, `Some(false)` when it wasn't, `None` for a bad value.
    pub fn take<'a>(&mut self, a: &str, it: &mut impl Iterator<Item = &'a String>) -> Option<bool> {
        match a {
            "--workers" => {
                self.workers = it.next()?.parse().ok()?;
            }
            "--worker" => self.remote.push(it.next()?.clone()),
            "--token" => self.token = Some(it.next()?.clone()),
            "--task-timeout" => {
                let s: f64 = it.next()?.parse().ok()?;
                if !(s.is_finite() && s > 0.0) {
                    return None;
                }
                self.task_timeout = Some(Duration::from_secs_f64(s));
            }
            _ => return Some(false),
        }
        Some(true)
    }

    /// The farm these options ask for, or none when they ask for no worker.
    pub fn farm(&self) -> Result<Option<Farm>, String> {
        if self.workers == 0 && self.remote.is_empty() {
            return Ok(None);
        }
        let token = self
            .token
            .clone()
            .or_else(|| std::env::var(TOKEN_VARIABLE).ok())
            .filter(|t| !t.is_empty());
        let mut farm = Farm::new(FarmOptions {
            token,
            task_timeout: self.task_timeout,
            ..FarmOptions::default()
        });
        if self.workers > 0 {
            let program = std::env::current_exe().map_err(|e| e.to_string())?;
            farm.spawn_local(&program, self.workers, None)
                .map_err(|e| e.to_string())?;
        }
        for a in &self.remote {
            farm.add_remote(a);
        }
        Ok(Some(farm))
    }
}
