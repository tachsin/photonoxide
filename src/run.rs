//! Runs: job descriptions, run directories, event records and replay.
//!
//! A [`Job`] is read from a TOML file: a name, an optional hard timeout, and the task, which
//! each solver reads in its own way. [`Run::create`] makes the run's directory,
//! `<root>/<UTC time>-<name>/`, with a copy of the job (`job.toml`), what ran it (`meta.json`),
//! and the record of what happened (`events.jsonl`, one JSON event per line, written as it
//! happens so a window can follow a run live). [`replay`] reads the events back: a replayed run
//! is the same run, to the last bit of every float.
//!
//! A [`Stop`] tells a running job to stop, when its deadline passes or when asked (a window
//! closed, a user cancelled). Every long loop checks it.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// The job file's layout.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JobFile {
    name: String,
    timeout_minutes: Option<f64>,
    #[serde(default)]
    task: toml::Table,
}

/// A job: what to run, under which name, for at most how long.
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    name: String,
    timeout: Option<Duration>,
    task: toml::Table,
    text: String,
}

impl Job {
    /// A job from the text of a job file:
    ///
    /// ```toml
    /// name = "strip-modes"     # letters, digits, '-', '_' and '.'
    /// timeout_minutes = 30     # optional: a hard limit on the run
    ///
    /// [task]                   # read by the solver that runs the job
    /// width_um = 0.5
    /// ```
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] if the text isn't such a file, and [`Error::InvalidValue`] for an empty
    /// name, a name with other characters, or a timeout that isn't positive and finite.
    pub fn parse(text: &str) -> Result<Job> {
        let file: JobFile = toml::from_str(text).map_err(|e| Error::Parse {
            what: "job".into(),
            reason: e.to_string(),
        })?;
        let valid = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
        if file.name.is_empty() || !file.name.chars().all(valid) {
            return Err(Error::invalid(
                "job name",
                format!(
                    "must be letters, digits, '-', '_' or '.', got {:?}",
                    file.name
                ),
            ));
        }
        let timeout = match file.timeout_minutes {
            None => None,
            Some(m) if m.is_finite() && m > 0.0 => Some(Duration::from_secs_f64(m * 60.0)),
            Some(m) => {
                return Err(Error::invalid(
                    "timeout",
                    format!("must be positive and finite, got {m} minutes"),
                ));
            }
        };
        Ok(Job {
            name: file.name,
            timeout,
            task: file.task,
            text: text.to_owned(),
        })
    }

    /// The job in the file at `path`.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the file can't be read, and the errors of [`Job::parse`].
    pub fn load(path: &Path) -> Result<Job> {
        let text = fs::read_to_string(path).map_err(|e| io(path, &e))?;
        Job::parse(&text).map_err(|e| match e {
            Error::Parse { reason, .. } => Error::Parse {
                what: path.display().to_string(),
                reason,
            },
            other => other,
        })
    }

    /// The job's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The hard limit on the run, if any.
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// The task, for the solver to read.
    pub fn task(&self) -> &toml::Table {
        &self.task
    }
}

fn io(path: &Path, e: &std::io::Error) -> Error {
    Error::Io {
        path: path.display().to_string(),
        reason: e.to_string(),
    }
}

/// What ran a run, saved as its `meta.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    /// The photonoxide version.
    pub photonoxide: String,
    /// The job's name.
    pub job: String,
    /// When the run started, UTC, e.g. `"2026-10-01T12:00:00Z"`.
    pub started: String,
    /// The number of threads the machine offered.
    pub threads: usize,
}

/// A run: its directory, and the record of its events.
#[derive(Debug)]
pub struct Run {
    dir: PathBuf,
    events: BufWriter<File>,
    started: Instant,
}

impl Run {
    /// Creates the run's directory under `root`, named after the current UTC time and the job
    /// (`20261001-120000-strip-modes`, with `-2`, `-3`, … if that exists), and writes `job.toml`
    /// and `meta.json` into it.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the directory or its files can't be written.
    pub fn create(root: &Path, job: &Job) -> Result<Run> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let (date, time) = utc(now);
        let base = format!(
            "{}-{}-{}",
            date.replace('-', ""),
            time.replace(':', ""),
            job.name
        );
        fs::create_dir_all(root).map_err(|e| io(root, &e))?;
        let mut dir = root.join(&base);
        let mut n = 1;
        loop {
            match fs::create_dir(&dir) {
                Ok(()) => break,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    n += 1;
                    dir = root.join(format!("{base}-{n}"));
                }
                Err(e) => return Err(io(&dir, &e)),
            }
        }
        let write = |name: &str, text: &str| {
            let path = dir.join(name);
            fs::write(&path, text).map_err(|e| io(&path, &e))
        };
        write("job.toml", &job.text)?;
        let meta = Meta {
            photonoxide: env!("CARGO_PKG_VERSION").into(),
            job: job.name.clone(),
            started: format!("{date}T{time}Z"),
            threads: std::thread::available_parallelism().map_or(1, |n| n.get()),
        };
        let json = serde_json::to_string_pretty(&meta).map_err(|e| Error::Parse {
            what: "meta.json".into(),
            reason: e.to_string(),
        })?;
        write("meta.json", &json)?;
        let path = dir.join("events.jsonl");
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|e| io(&path, &e))?;
        Ok(Run {
            dir,
            events: BufWriter::new(file),
            started: Instant::now(),
        })
    }

    /// The run's directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The time since the run was created.
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// Appends `event` to `events.jsonl`, as one line, and flushes it so a live viewer sees it.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] if the event can't be serialized, [`Error::Io`] if it can't be written.
    pub fn record<E: Serialize>(&mut self, event: &E) -> Result<()> {
        let path = self.dir.join("events.jsonl");
        serde_json::to_writer(&mut self.events, event).map_err(|e| Error::Parse {
            what: "event".into(),
            reason: e.to_string(),
        })?;
        self.events
            .write_all(b"\n")
            .and_then(|()| self.events.flush())
            .map_err(|e| io(&path, &e))
    }
}

/// The events of the run in `dir`, in order. A last line without its newline (a run killed
/// while writing it) is left out.
///
/// # Errors
///
/// [`Error::Io`] if `events.jsonl` can't be read, and [`Error::Parse`] for a complete line
/// that isn't an `E`.
pub fn replay<E: DeserializeOwned>(dir: &Path) -> Result<Vec<E>> {
    let path = dir.join("events.jsonl");
    let file = File::open(&path).map_err(|e| io(&path, &e))?;
    let mut reader = BufReader::new(file);
    let mut events = Vec::new();
    let mut line = String::new();
    let mut number = 0;
    loop {
        line.clear();
        if reader.read_line(&mut line).map_err(|e| io(&path, &e))? == 0 {
            break;
        }
        number += 1;
        if !line.ends_with('\n') {
            break; // cut off mid-write
        }
        let event = serde_json::from_str(line.trim_end()).map_err(|e| Error::Parse {
            what: format!("{} line {number}", path.display()),
            reason: e.to_string(),
        })?;
        events.push(event);
    }
    Ok(events)
}

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

/// The UTC date (`YYYY-MM-DD`) and time (`HH:MM:SS`) of a Unix time in seconds, by the
/// proleptic Gregorian calendar: `civil_from_days` of H. Hinnant, "chrono-Compatible Low-Level
/// Date Algorithms", <https://howardhinnant.github.io/date_algorithms.html> (checked against it
/// line by line; `div_euclid` is its floor division).
fn utc(unix_seconds: u64) -> (String, String) {
    let days = (unix_seconds / 86_400) as i64;
    let secs = unix_seconds % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (
        format!("{year:04}-{month:02}-{day:02}"),
        format!(
            "{:02}:{:02}:{:02}",
            secs / 3600,
            secs % 3600 / 60,
            secs % 60
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    /// A fresh directory under the system's temporary directory, removed when dropped.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> TempDir {
            static COUNT: AtomicUsize = AtomicUsize::new(0);
            let n = COUNT.fetch_add(1, Ordering::Relaxed);
            let dir =
                std::env::temp_dir().join(format!("photonoxide-test-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            TempDir(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const JOB: &str = "name = \"strip-modes\"\ntimeout_minutes = 1.5\n\n[task]\nwidth_um = 0.5\n";

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Step {
        n: u32,
        value: f64,
    }

    #[test]
    fn a_job_file_is_read_and_checked() {
        let job = Job::parse(JOB).unwrap();
        assert_eq!(job.name(), "strip-modes");
        assert_eq!(job.timeout(), Some(Duration::from_secs(90)));
        assert_eq!(job.task()["width_um"].as_float(), Some(0.5));
        assert!(Job::parse("name = \"a b\"").is_err());
        assert!(Job::parse("name = \"\"").is_err());
        assert!(Job::parse("name = \"a\"\ntimeout_minutes = -1").is_err());
        assert!(matches!(
            Job::parse("name = \"a\"\nunknown = 1"),
            Err(Error::Parse { .. })
        ));
        assert!(Job::parse("name = \"a\"").unwrap().timeout().is_none());
    }

    #[test]
    fn a_run_directory_holds_the_job_its_meta_and_its_events() {
        let root = TempDir::new();
        let job = Job::parse(JOB).unwrap();
        let mut run = Run::create(&root.0, &job).unwrap();
        let dir = run.dir().to_path_buf();
        assert!(
            dir.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with("-strip-modes")
        );
        assert_eq!(fs::read_to_string(dir.join("job.toml")).unwrap(), JOB);
        let meta: Meta =
            serde_json::from_str(&fs::read_to_string(dir.join("meta.json")).unwrap()).unwrap();
        assert_eq!(meta.job, "strip-modes");
        assert_eq!(meta.photonoxide, env!("CARGO_PKG_VERSION"));
        assert!(meta.started.ends_with('Z') && meta.threads >= 1);
        run.record(&Step { n: 1, value: 0.5 }).unwrap();
        // a second run of the same job in the same second gets its own directory
        let other = Run::create(&root.0, &job).unwrap();
        assert_ne!(other.dir(), run.dir());
    }

    #[test]
    fn a_replayed_run_has_the_same_floats_to_the_last_bit() {
        let root = TempDir::new();
        let mut run = Run::create(&root.0, &Job::parse(JOB).unwrap()).unwrap();
        let values = [
            0.1 + 0.2,
            1.0 / 3.0,
            std::f64::consts::PI,
            1e-300,
            f64::MIN_POSITIVE,
            -2.5e17,
        ];
        let steps: Vec<Step> = values
            .iter()
            .enumerate()
            .map(|(n, &value)| Step { n: n as u32, value })
            .collect();
        for s in &steps {
            run.record(s).unwrap();
        }
        let back: Vec<Step> = replay(run.dir()).unwrap();
        assert_eq!(back.len(), steps.len());
        for (a, b) in back.iter().zip(&steps) {
            assert_eq!(a.value.to_bits(), b.value.to_bits());
        }
    }

    #[test]
    fn a_line_cut_off_mid_write_is_left_out() {
        let root = TempDir::new();
        let mut run = Run::create(&root.0, &Job::parse(JOB).unwrap()).unwrap();
        run.record(&Step { n: 1, value: 1.0 }).unwrap();
        let path = run.dir().join("events.jsonl");
        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(b"{\"n\":2,\"val").unwrap();
        let back: Vec<Step> = replay(run.dir()).unwrap();
        assert_eq!(back, vec![Step { n: 1, value: 1.0 }]);
        // a complete line that isn't an event is an error, with its line number
        fs::write(&path, "{\"n\":1,\"value\":1.0}\nnot json\n").unwrap();
        let e = replay::<Step>(run.dir()).unwrap_err();
        assert!(e.to_string().contains("line 2"), "{e}");
    }

    #[test]
    fn stop_on_request_or_at_the_deadline() {
        let stop = Stop::new(None);
        assert_eq!(stop.reason(), None);
        let window = stop.clone();
        window.request();
        assert_eq!(stop.reason(), Some(StopReason::Requested));
        let soon = Stop::new(Some(Duration::ZERO));
        assert_eq!(soon.reason(), Some(StopReason::Timeout));
    }

    #[test]
    fn utc_dates_follow_the_gregorian_calendar() {
        assert_eq!(utc(0), ("1970-01-01".into(), "00:00:00".into()));
        // a leap day, and a well-known Unix time
        assert_eq!(utc(951_782_400), ("2000-02-29".into(), "00:00:00".into()));
        assert_eq!(utc(1_700_000_000), ("2023-11-14".into(), "22:13:20".into()));
        assert_eq!(utc(4_107_542_399), ("2100-02-28".into(), "23:59:59".into()));
    }
}
