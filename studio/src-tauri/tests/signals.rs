//! SIGTERM, as `docker stop` and a scheduler's time limit send it: a run stops as its window
//! closing stops it, with exit code 4 and a record that ends in its `stopped` event, and a
//! worker drops its tasks and exits 4, its coordinator finishing the sweep on the others. Unix
//! only: Windows has no SIGTERM.
#![cfg(unix)]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const PROGRAM: &str = env!("CARGO_BIN_EXE_photonoxide");

/// A 2D FDFD wavelength sweep of 24 points (tests/farm.rs's).
const JOB: &str = r#"
name = "signal-test"
timeout_minutes = 10

[solver]
direct = "photonoxide"

[task]
kind = "fdfd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-3.0, 3.0]
y_um = [-2.0, 2.0]
step_nm = 20.0

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [8.0, 0.5]

[[task.port]]
x_um = -2.0
side = "left"

[[task.port]]
x_um = 2.0
side = "right"

[task.sweep]
parameter = "wavelength"
from = 1.5
to = 1.6
points = 24
"#;

/// A fresh directory with the job in it, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> TempDir {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let n = COUNT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "photonoxide-signals-{tag}-{}-{n}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("signal-test.toml"), JOB).unwrap();
        TempDir(dir)
    }

    fn job(&self) -> PathBuf {
        self.0.join("signal-test.toml")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A child process killed when dropped.
struct Killed(Child);

impl Drop for Killed {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Sends SIGTERM to `child`, as `docker stop` and Slurm do (the shell's own `kill`: a slim
/// system may have no kill program).
fn terminate(child: &Child) {
    let status = Command::new("sh")
        .args(["-c", &format!("kill -TERM {}", child.id())])
        .status()
        .unwrap();
    assert!(status.success());
}

/// `photonoxide run <job> --headless --out <dir>` with `extra`, as a child; its run's directory
/// is the first line it writes.
fn start_run(job: &Path, out: &Path, extra: &[&str]) -> (Child, PathBuf) {
    let mut child = Command::new(PROGRAM)
        .arg("run")
        .arg(job)
        .args(["--headless", "--out"])
        .arg(out)
        .args(extra)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    (child, PathBuf::from(line.trim()))
}

/// The sweep's points the run recorded so far.
fn points(run: &Path) -> usize {
    std::fs::read_to_string(run.join("events.jsonl"))
        .unwrap_or_default()
        .matches("{\"type\":\"s_parameters\"")
        .count()
}

/// Waits until the run recorded `n` points.
fn wait_for(run: &Path, n: usize) {
    let started = Instant::now();
    while points(run) < n {
        assert!(
            started.elapsed() < Duration::from_secs(300),
            "the sweep didn't start"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The record of a run, without the time its `finished` event gives.
fn record(run: &Path) -> Vec<String> {
    std::fs::read_to_string(run.join("events.jsonl"))
        .unwrap()
        .lines()
        .map(|l| {
            if l.starts_with("{\"type\":\"finished\"") {
                let mut v: serde_json::Value = serde_json::from_str(l).unwrap();
                v["seconds"] = serde_json::Value::Null;
                v.to_string()
            } else {
                l.to_owned()
            }
        })
        .collect()
}

#[test]
fn a_run_sent_sigterm_mid_sweep_stops_with_exit_4_and_a_whole_record() {
    let dir = TempDir::new("run");
    let (child, run) = start_run(&dir.job(), &dir.0, &[]);
    wait_for(&run, 2);
    terminate(&child);
    let output = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(4), "{stderr}");
    assert!(stderr.contains("SIGTERM"), "{stderr}");
    let done = points(&run);
    assert!(
        (2..24).contains(&done),
        "{done} points: it didn't stop mid-sweep"
    );
    // the record ends in its stopped event, as the window closing ends it
    let events = std::fs::read_to_string(run.join("events.jsonl")).unwrap();
    let last: serde_json::Value = serde_json::from_str(events.lines().last().unwrap()).unwrap();
    assert_eq!(last["type"], "finished", "{last}");
    assert_eq!(last["stopped"], "requested", "{last}");
    // and results.json reads, and says why
    let results: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(run.join("results.json")).unwrap()).unwrap();
    assert_eq!(results["outcome"], "stopped", "{results}");
    assert_eq!(results["stopped"], "requested", "{results}");
}

/// `photonoxide worker` on a free port of this machine, and where it listens.
fn worker(token: &str) -> (Killed, String) {
    let mut child = Command::new(PROGRAM)
        .args(["worker", "--listen", "127.0.0.1:0", "--token", token])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let address = line
        .trim()
        .strip_prefix("listening on ")
        .unwrap_or_else(|| panic!("{line:?}"))
        .to_owned();
    (Killed(child), address)
}

#[test]
fn a_worker_sent_sigterm_is_dropped_and_the_run_finishes_on_the_others() {
    let dir = TempDir::new("worker");
    let (child, alone) = start_run(&dir.job(), &dir.0, &[]);
    assert!(child.wait_with_output().unwrap().status.success());
    let alone = record(&alone);
    let token = "a token for this test";
    let mut workers: Vec<(Killed, String)> = (0..3).map(|_| worker(token)).collect();
    let mut args = vec!["--token", token];
    for (_, address) in &workers {
        args.extend(["--worker", address.as_str()]);
    }
    let (child, farmed) = start_run(&dir.job(), &dir.0, &args);
    wait_for(&farmed, 3);
    let before = points(&farmed);
    let (mut stopped, _) = workers.remove(0);
    terminate(&stopped.0);
    let status = stopped.0.wait().unwrap();
    let mut said = String::new();
    std::io::Read::read_to_string(&mut stopped.0.stderr.take().unwrap(), &mut said).unwrap();
    assert_eq!(status.code(), Some(4), "{said}");
    assert!(said.contains("SIGTERM"), "{said}");
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(before < 24, "the sweep was over before the signal");
    // the same record, to the last bit, as the run alone
    assert_eq!(record(&farmed), alone);
}
