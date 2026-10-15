//! Work the studio hands to a process of its own: an example (`photonoxide example <name>`) or
//! the validation report (`photonoxide validate`). Its output is kept line by line as it comes,
//! for the window to poll, and it can be stopped.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;

/// A task's output so far.
#[derive(Default)]
struct Output {
    lines: Vec<String>,
    /// Its exit code once it ended (−1 when it was stopped or crashed).
    code: Option<i32>,
}

struct Task {
    child: Arc<Mutex<Child>>,
    output: Arc<Mutex<Output>>,
    started: Instant,
}

/// The tasks started from the window.
#[derive(Default)]
pub struct Tasks {
    next: Mutex<u64>,
    running: Mutex<HashMap<u64, Task>>,
}

/// What [`Tasks::output`] returns: the lines from the `from`-th on.
#[derive(Serialize)]
pub struct Progress {
    pub lines: Vec<String>,
    pub code: Option<i32>,
    pub seconds: f64,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Tasks {
    /// Starts this program with `args`, its output read line by line; returns the task's id.
    pub fn start(&self, args: &[&str]) -> Result<u64, String> {
        let exe = std::env::current_exe().map_err(|e| format!("can't find the program: {e}"))?;
        let mut command = Command::new(exe);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            // no console window for the child
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        #[cfg(unix)]
        {
            // a group of its own, so stopping it stops the processes it started (a benchmark's)
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("can't start {args:?}: {e}"))?;
        let output = Arc::new(Mutex::new(Output::default()));
        let pipes = [
            child
                .stdout
                .take()
                .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
            child
                .stderr
                .take()
                .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
        ];
        let child = Arc::new(Mutex::new(child));
        let readers: Vec<_> = pipes
            .into_iter()
            .flatten()
            .map(|pipe| {
                let output = output.clone();
                std::thread::spawn(move || {
                    for line in BufReader::new(pipe).lines() {
                        let Ok(line) = line else { break };
                        lock(&output).lines.push(line);
                    }
                })
            })
            .collect();
        {
            // once both pipes closed, the exit code
            let (child, output) = (child.clone(), output.clone());
            std::thread::spawn(move || {
                for r in readers {
                    let _ = r.join();
                }
                let code = loop {
                    match lock(&child).try_wait() {
                        Ok(Some(status)) => break status.code().unwrap_or(-1),
                        Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
                        Err(_) => break -1,
                    }
                };
                lock(&output).code = Some(code);
            });
        }
        let id = {
            let mut next = lock(&self.next);
            *next += 1;
            *next
        };
        lock(&self.running).insert(
            id,
            Task {
                child,
                output,
                started: Instant::now(),
            },
        );
        Ok(id)
    }

    /// Task `id`'s lines from the `from`-th on, and its exit code once it ended.
    pub fn output(&self, id: u64, from: usize) -> Result<Progress, String> {
        let running = lock(&self.running);
        let task = running.get(&id).ok_or_else(|| format!("no task {id}"))?;
        let output = lock(&task.output);
        Ok(Progress {
            lines: output.lines.get(from..).unwrap_or_default().to_vec(),
            code: output.code,
            seconds: task.started.elapsed().as_secs_f64(),
        })
    }

    /// Stops task `id`, if it is still running.
    pub fn stop(&self, id: u64) {
        if let Some(task) = lock(&self.running).get(&id) {
            kill_tree(&mut lock(&task.child));
        }
    }

    /// Stops every task: the window is closing.
    pub fn stop_all(&self) {
        for task in lock(&self.running).values() {
            kill_tree(&mut lock(&task.child));
        }
    }
}

/// Stops a task and the processes it started: a benchmark runs each problem in a child of its
/// own, which would otherwise run on to its end.
pub(crate) fn kill_tree(child: &mut Child) {
    if let Ok(None) = child.try_wait() {
        let pid = child.id().to_string();
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let _ = Command::new("taskkill")
                .args(["/PID", &pid, "/T", "/F"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(CREATE_NO_WINDOW)
                .status();
        }
        #[cfg(unix)]
        {
            // the task leads its own group (see `start`)
            let _ = Command::new("kill")
                .args(["-KILL", "--", &format!("-{pid}")])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        #[cfg(not(any(windows, unix)))]
        let _ = pid;
    }
    let _ = child.kill();
}
