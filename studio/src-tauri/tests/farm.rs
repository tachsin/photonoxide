//! The program's farm: `photonoxide run --workers` and `photonoxide worker`, as processes. A
//! farmed run records what the run records on its own, to the last bit, on any number of
//! workers and with a worker killed while it works.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const PROGRAM: &str = env!("CARGO_BIN_EXE_photonoxide");

/// A 2D FDFD wavelength sweep: a strip between two ports, 300 × 200 cells of 20 nm.
const JOB: &str = r#"
name = "farm-test"
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

/// A fresh directory, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> TempDir {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let n = COUNT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "photonoxide-farm-program-{tag}-{}-{n}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
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

fn finish(child: Child) {
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn job_file(dir: &TempDir) -> PathBuf {
    let path = dir.0.join("farm-test.toml");
    std::fs::write(&path, JOB).unwrap();
    path
}

#[test]
fn a_farmed_run_records_what_the_run_alone_does_on_any_number_of_workers() {
    let dir = TempDir::new("workers");
    let job = job_file(&dir);
    let (child, alone) = start_run(&job, &dir.0, &[]);
    finish(child);
    let alone = record(&alone);
    assert_eq!(
        alone
            .iter()
            .filter(|l| l.starts_with("{\"type\":\"s_parameters\""))
            .count(),
        24
    );
    for workers in ["1", "2", "4", "8"] {
        let (child, farmed) = start_run(&job, &dir.0, &["--workers", workers]);
        finish(child);
        assert_eq!(record(&farmed), alone, "{workers} workers");
    }
}

/// `photonoxide worker` on a free port of this machine, and where it listens.
fn worker(token: &str) -> (Killed, String) {
    let mut child = Command::new(PROGRAM)
        .args(["worker", "--listen", "127.0.0.1:0", "--token", token])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
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
fn a_worker_killed_mid_run_leaves_the_record_unchanged() {
    let dir = TempDir::new("killed");
    let job = job_file(&dir);
    let (child, alone) = start_run(&job, &dir.0, &[]);
    finish(child);
    let alone = record(&alone);
    let token = "a token for this test";
    let mut workers: Vec<(Killed, String)> = (0..3).map(|_| worker(token)).collect();
    let mut args = vec!["--token", token];
    for (_, address) in &workers {
        args.extend(["--worker", address.as_str()]);
    }
    let (child, farmed) = start_run(&job, &dir.0, &args);
    // once the sweep is under way, the first worker is killed while it works
    let points = |run: &Path| {
        std::fs::read_to_string(run.join("events.jsonl"))
            .unwrap_or_default()
            .matches("{\"type\":\"s_parameters\"")
            .count()
    };
    let started = Instant::now();
    while points(&farmed) < 3 {
        assert!(
            started.elapsed() < Duration::from_secs(300),
            "the sweep didn't start"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let before = points(&farmed);
    let (mut killed, _) = workers.remove(0);
    killed.0.kill().unwrap();
    let _ = killed.0.wait();
    finish(child);
    assert!(before < 24, "the sweep was over before the kill");
    assert_eq!(record(&farmed), alone);
}
