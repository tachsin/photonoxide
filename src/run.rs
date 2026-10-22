//! Runs: job descriptions, run directories, event records and replay.
//!
//! A [`Job`] is read from a TOML, JSON or YAML file ([`Format`]): a name, an optional hard
//! timeout, and the task, which each solver reads in its own way. [`Run::create`] makes the
//! run's directory, `<root>/<UTC time>-<name>/`, with a copy of the job as it was given
//! (`job.toml`, `job.json` or `job.yaml`), what ran it (`meta.json`),
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

use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::backend::Choice;
use crate::{Error, Result};

/// The job file's layout, the same in every [`Format`].
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct JobFile {
    /// The schema an editor checks the file with ([`crate::job::schema`]): read and left out,
    /// for a JSON job, which has no comment to point an editor at it.
    #[serde(rename = "$schema", default, skip_serializing)]
    #[schemars(skip)]
    _schema: Option<String>,
    /// The job's name: its runs' directories end in it.
    name: String,
    /// The hard limit on the run, in minutes: it stops when the limit passes.
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout_minutes: Option<f64>,
    /// The solver backends the job asks for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    solver: Option<SolverFile>,
    /// A device of the library: the structure, which the task computes, studies or optimizes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<crate::job::schema::Open>")]
    device: Option<toml::Table>,
    /// A chip of instances: the structure, which the task computes, studies or optimizes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<crate::job::schema::Open>")]
    circuit: Option<toml::Table>,
    /// What to compute, by its kind.
    #[serde(default)]
    #[schemars(with = "crate::job::schema::Open")]
    task: toml::Table,
}

/// The job file's `[solver]` table.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SolverFile {
    /// The direct solver of an `"fdfd"` job: `"auto"` (the default, photonoxide's own),
    /// `"photonoxide"`, or a backend registered under another name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    direct: Option<String>,
}

/// The format of a job file: TOML, JSON or YAML, the same job in each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    /// TOML, the format of every built-in job.
    Toml,
    /// JSON. It has no comments, so a JSON job has no description.
    Json,
    /// YAML 1.2: `no`, `on` and the other YAML 1.1 booleans are strings, as `1:30` is.
    Yaml,
}

impl Format {
    /// The three formats.
    pub const ALL: [Format; 3] = [Format::Toml, Format::Json, Format::Yaml];

    /// The format a file's extension names: `.toml`, `.json`, `.yaml` or `.yml`; none for
    /// another.
    pub fn from_path(path: &Path) -> Option<Format> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "toml" => Some(Format::Toml),
            "json" => Some(Format::Json),
            "yaml" | "yml" => Some(Format::Yaml),
            _ => None,
        }
    }

    /// The extension a job in this format is saved with: `toml`, `json` or `yaml`.
    pub fn extension(self) -> &'static str {
        match self {
            Format::Toml => "toml",
            Format::Json => "json",
            Format::Yaml => "yaml",
        }
    }
}

/// A job: what to run, under which name, for at most how long.
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    name: String,
    timeout_minutes: Option<f64>,
    timeout: Option<Duration>,
    solver: Option<SolverFile>,
    direct: Choice,
    device: Option<toml::Table>,
    circuit: Option<toml::Table>,
    task: toml::Table,
    text: String,
    format: Format,
    /// The folder of the file it was read from, where the files it names are found.
    dir: Option<PathBuf>,
}

impl Job {
    /// A job from the text of a TOML job file:
    ///
    /// ```toml
    /// name = "strip-modes"     # letters, digits, '-', '_' and '.'
    /// timeout_minutes = 30     # optional: a hard limit on the run
    ///
    /// [solver]                 # optional
    /// direct = "photonoxide"   # the direct solver: auto (the default), photonoxide, or a
    ///                          # backend's name (see crate::backend)
    ///
    /// [task]                   # read by the solver that runs the job
    /// width_um = 0.5
    /// ```
    ///
    /// or, in place of a task, a `[device]` table: one of [`crate::device`]'s devices with its
    /// stack, its parameters' expressions and its fidelity ([`crate::device::Spec`]); or, beside
    /// a task, a `[circuit]` table: a chip of devices and components
    /// ([`crate::job::ChipDesign`]).
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] if the text isn't such a file, and [`Error::InvalidValue`] for an empty
    /// name, a name with other characters, a timeout that isn't positive and finite, or a
    /// direct solver that isn't a backend's name (whether one is registered under it is for
    /// [`crate::job::check`] and the run to say).
    pub fn parse(text: &str) -> Result<Job> {
        Job::parse_as(text, Format::Toml)
    }

    /// A job from the text of a job file in `format`: the same fields as [`Job::parse`] takes,
    /// as a JSON object or a YAML mapping. Unknown fields are refused in every format, and a
    /// value TOML can't hold (a JSON or YAML null) is an error.
    ///
    /// # Errors
    ///
    /// As [`Job::parse`]; a parse error names the line where the format gives one.
    pub fn parse_as(text: &str, format: Format) -> Result<Job> {
        let parse = |reason: String| Error::Parse {
            what: "job".into(),
            reason,
        };
        let mut file: JobFile = match format {
            Format::Toml => toml::from_str(text).map_err(|e| parse(e.to_string()))?,
            Format::Json => serde_json::from_str(text).map_err(|e| parse(e.to_string()))?,
            Format::Yaml => serde_saphyr::from_str_with_options(text, yaml_options())
                .map_err(|e| parse(e.to_string()))?,
        };
        // the shorter forms (units in values, shapes one to a line) as the keys they stand for
        let mut tables = toml::Table::new();
        tables.insert("task".into(), std::mem::take(&mut file.task).into());
        for (key, table) in [
            ("device", file.device.take()),
            ("circuit", file.circuit.take()),
        ] {
            if let Some(t) = table {
                tables.insert(key.into(), t.into());
            }
        }
        crate::job::short::read(&mut tables).map_err(parse)?;
        let mut table = |key: &str| match tables.remove(key) {
            Some(toml::Value::Table(t)) => Some(t),
            _ => None,
        };
        file.task = table("task").unwrap_or_default();
        file.device = table("device");
        file.circuit = table("circuit");
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
        let direct = match file.solver.as_ref().and_then(|s| s.direct.as_deref()) {
            None => Choice::Auto,
            Some(name) => Choice::parse(name)?,
        };
        Ok(Job {
            name: file.name,
            timeout_minutes: file.timeout_minutes,
            timeout,
            solver: file.solver,
            direct,
            device: file.device,
            circuit: file.circuit,
            task: file.task,
            text: text.to_owned(),
            format,
            dir: None,
        })
    }

    /// The job in the file at `path`, in the format its extension names ([`Format::from_path`]);
    /// TOML for any other extension, as before there were others.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the file can't be read, and the errors of [`Job::parse_as`].
    pub fn load(path: &Path) -> Result<Job> {
        let text = fs::read_to_string(path).map_err(|e| io(path, &e))?;
        let format = Format::from_path(path).unwrap_or(Format::Toml);
        let job = Job::parse_as(&text, format).map_err(|e| match e {
            Error::Parse { reason, .. } => Error::Parse {
                what: path.display().to_string(),
                reason,
            },
            other => other,
        })?;
        Ok(match path.parent() {
            Some(dir) => job.in_dir(dir),
            None => job,
        })
    }

    /// The same job, the files it names found from `dir`: a design's compact model
    /// (`compact = "ring.compact.json"` in its `[device]` table). [`Job::load`] sets it to the
    /// file's folder.
    #[must_use]
    pub fn in_dir(self, dir: impl Into<PathBuf>) -> Job {
        Job {
            dir: Some(dir.into()),
            ..self
        }
    }

    /// The folder the files the job names are found from, if it has one: the folder of the file
    /// it was loaded from, or one [`Job::in_dir`] gave.
    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
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

    /// The job's `[device]` table, if it has one: a device's design
    /// ([`crate::device::Spec::from_table`] reads it).
    pub fn device(&self) -> Option<&toml::Table> {
        self.device.as_ref()
    }

    /// The job's `[circuit]` table, if it has one: a chip of devices and components
    /// ([`crate::job::ChipDesign::from_table`] reads it).
    pub fn circuit(&self) -> Option<&toml::Table> {
        self.circuit.as_ref()
    }

    /// The direct solver the job asks for in its `[solver]` table: `auto` unless it names one.
    pub fn direct(&self) -> &Choice {
        &self.direct
    }

    /// The format the job was read in.
    pub fn format(&self) -> Format {
        self.format
    }

    /// The text the job was read from.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The job written in `format`: its name, timeout and task, which [`Job::parse_as`] reads
    /// back to the same job, every float to the last bit. Comments aren't kept.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] if the format can't write the task (TOML can't write every table JSON
    /// and YAML can, e.g. a value after a table in an array).
    pub fn to_text(&self, format: Format) -> Result<String> {
        let file = JobFile {
            _schema: None,
            name: self.name.clone(),
            timeout_minutes: self.timeout_minutes,
            solver: self.solver.clone(),
            device: self.device.clone(),
            circuit: self.circuit.clone(),
            task: self.task.clone(),
        };
        let write = |reason: String| Error::Parse {
            what: format!("job as {}", format.extension()),
            reason,
        };
        match format {
            Format::Toml => toml::to_string_pretty(&file).map_err(|e| write(e.to_string())),
            Format::Json => serde_json::to_string_pretty(&file)
                .map(|s| s + "\n")
                .map_err(|e| write(e.to_string())),
            Format::Yaml => {
                let solver = file
                    .solver
                    .as_ref()
                    .map(toml::Value::try_from)
                    .transpose()
                    .map_err(|e| write(e.to_string()))?;
                crate::job::short::yaml(&[
                    ("name", Some(file.name.into())),
                    ("timeout_minutes", file.timeout_minutes.map(Into::into)),
                    ("solver", solver),
                    ("device", file.device.map(Into::into)),
                    ("circuit", file.circuit.map(Into::into)),
                    ("task", Some(file.task.into())),
                ])
                .map_err(write)
            }
        }
    }
}

/// YAML 1.2's booleans only: `no`, `yes`, `on` and `off` stay strings.
fn yaml_options() -> serde_saphyr::Options {
    serde_saphyr::options! {
        strict_booleans: true,
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
    events: Sink,
    started: Instant,
    digest: u64,
    // behind a mutex, so a run stays Sync, UnwindSafe and RefUnwindSafe as it was before the
    // farm gave it one (only `&mut self` ever reaches it, through `get_mut` or by taking it)
    points: Option<std::sync::Mutex<Box<dyn Points>>>,
    /// While a task reads back what it recorded (a fit, its samples), each line recorded.
    tap: Option<Vec<String>>,
}

/// Where a run's events go.
#[derive(Debug)]
enum Sink {
    File(BufWriter<File>),
    Memory(Vec<String>),
    Discard,
}

/// What solves a sweep's point: its events, made without the run (`None` when the stop came
/// first).
pub type PointFn<'a> = dyn Fn(usize) -> Result<Option<Vec<crate::job::Event>>> + Sync + 'a;

/// Where a run's sweep solves its points when not side by side in this process: a farm's
/// workers ([`crate::farm`]), or, in a worker, the points its coordinator asks for. A run has
/// none unless [`Run::set_points`] gives it one.
pub trait Points: Send + std::fmt::Debug {
    /// Solves the sweep's points `points`, `point(k)` being point k's events solved here, and
    /// records them into `run` in their order, as a loop over the points would: a stop ends
    /// the sweep at its first point not done, and an error at a point ends it there, after the
    /// points before it.
    ///
    /// # Errors
    ///
    /// A point's error, the farm's (no worker left, a point past its timeout), and
    /// [`Error::Io`] if the record can't be written.
    fn sweep(
        &mut self,
        run: &mut Run,
        stop: &Stop,
        points: std::ops::Range<usize>,
        point: &PointFn<'_>,
    ) -> Result<()>;
}

impl Run {
    /// Creates the run's directory under `root`, named after the current UTC time and the job
    /// (`20261001-120000-strip-modes`, with `-2`, `-3`, … if that exists), and writes the job's
    /// text as it was given (`job.toml`, `job.json` or `job.yaml`) and `meta.json` into it.
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
        write(&format!("job.{}", job.format.extension()), &job.text)?;
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
            events: Sink::File(BufWriter::new(file)),
            started: Instant::now(),
            digest: FNV_OFFSET,
            points: None,
            tap: None,
        })
    }

    /// A run with no directory that keeps its events in memory, as the lines `events.jsonl`
    /// would have: a farm's worker's run of a whole job ([`Run::lines`]).
    pub fn in_memory() -> Run {
        Run::without_file(Sink::Memory(Vec::new()))
    }

    /// A run with no directory that keeps nothing but the [`Run::digest`] of its events: a
    /// farm's worker setting a sweep up.
    pub fn discarding() -> Run {
        Run::without_file(Sink::Discard)
    }

    fn without_file(events: Sink) -> Run {
        Run {
            dir: PathBuf::new(),
            events,
            started: Instant::now(),
            digest: FNV_OFFSET,
            points: None,
            tap: None,
        }
    }

    /// The events recorded so far by a run [`Run::in_memory`], each a line of JSON without its
    /// newline; none for another run.
    pub fn lines(&self) -> &[String] {
        match &self.events {
            Sink::Memory(lines) => lines,
            _ => &[],
        }
    }

    /// A digest of every event recorded so far: FNV-1a (64 bits) of their lines, newlines
    /// included. Two runs that recorded the same events, to the last bit, have the same digest:
    /// how a farm checks that a worker set a job up as the run did.
    pub fn digest(&self) -> u64 {
        self.digest
    }

    /// Has the run's sweeps solve their points with `points` instead of side by side here.
    pub fn set_points(&mut self, points: Box<dyn Points>) {
        self.points = Some(std::sync::Mutex::new(points));
    }

    /// Takes back what [`Run::set_points`] gave, if anything.
    pub fn take_points(&mut self) -> Option<Box<dyn Points>> {
        self.points.take().map(|m| {
            m.into_inner()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        })
    }

    /// Keeps a copy of every line recorded from now on, for [`Run::take_tap`]: a task that
    /// reads back what it recorded, whoever solved it (this process or a farm's workers).
    pub(crate) fn start_tap(&mut self) {
        self.tap = Some(Vec::new());
    }

    /// The lines recorded since [`Run::start_tap`], and stops keeping them.
    pub(crate) fn take_tap(&mut self) -> Vec<String> {
        self.tap.take().unwrap_or_default()
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
        let line = serde_json::to_string(event).map_err(|e| Error::Parse {
            what: "event".into(),
            reason: e.to_string(),
        })?;
        self.record_line(&line)
    }

    /// Appends an event already written as a line of JSON (without its newline), as it is: a
    /// farm's worker's event, written byte for byte as the worker's [`Run::record`] wrote it.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] if `line` has a newline, [`Error::Io`] if it can't be written.
    pub fn record_line(&mut self, line: &str) -> Result<()> {
        if line.contains('\n') {
            return Err(Error::Parse {
                what: "event".into(),
                reason: "an event's line has a newline".into(),
            });
        }
        if let Some(tap) = &mut self.tap {
            tap.push(line.to_owned());
        }
        for b in line.bytes().chain(*b"\n") {
            self.digest = (self.digest ^ u64::from(b)).wrapping_mul(FNV_PRIME);
        }
        match &mut self.events {
            Sink::File(file) => {
                let path = self.dir.join("events.jsonl");
                file.write_all(line.as_bytes())
                    .and_then(|()| file.write_all(b"\n"))
                    .and_then(|()| file.flush())
                    .map_err(|e| io(&path, &e))
            }
            Sink::Memory(lines) => {
                lines.push(line.to_owned());
                Ok(())
            }
            Sink::Discard => Ok(()),
        }
    }

    /// Solves a sweep's points `points` and records their events in order: with what
    /// [`Run::set_points`] gave, or here, `batch` at a time side by side on rayon's threads,
    /// each batch's events recorded point by point, so the record is the one a loop over the
    /// points writes, whatever the threads. A stop ends the sweep at its first point not done;
    /// an error at a point ends it there, after the points before it.
    pub(crate) fn sweep(
        &mut self,
        stop: &Stop,
        points: std::ops::Range<usize>,
        batch: usize,
        point: &PointFn<'_>,
    ) -> Result<()> {
        use rayon::prelude::*;
        if let Some(mut elsewhere) = self.take_points() {
            let done = elsewhere.sweep(self, stop, points, point);
            self.set_points(elsewhere);
            return done;
        }
        let all: Vec<usize> = points.collect();
        for ks in all.chunks(batch.max(1)) {
            if stop.reason().is_some() {
                return Ok(());
            }
            let done: Vec<Result<Option<Vec<crate::job::Event>>>> =
                ks.par_iter().map(|&k| point(k)).collect();
            for events in done {
                let Some(events) = events? else {
                    return Ok(());
                };
                for e in &events {
                    self.record(e)?;
                }
            }
        }
        Ok(())
    }
}

/// FNV-1a's 64-bit offset basis and prime (G. Fowler, L. C. Noll, K.-P. Vo, D. Eastlake, "The
/// FNV Non-Cryptographic Hash Algorithm", IETF draft-eastlake-fnv).
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

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

    /// A run is Send, Sync, UnwindSafe and RefUnwindSafe, as it was in 0.5.1: losing one is a
    /// breaking change (cargo-semver-checks' auto_trait_impl_removed).
    #[test]
    fn a_run_keeps_its_auto_traits() {
        fn assert<T: Send + Sync + std::panic::UnwindSafe + std::panic::RefUnwindSafe>() {}
        assert::<Run>();
    }

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
    fn a_stop_within_a_timeout_keeps_the_sooner_deadline_and_shares_requests() {
        let hour = Some(Duration::from_secs(3600));
        // the sooner deadline wins, whichever has it
        let caller = Stop::new(hour);
        assert_eq!(
            caller.within(Some(Duration::ZERO)).reason(),
            Some(StopReason::Timeout)
        );
        assert_eq!(
            Stop::new(Some(Duration::ZERO)).within(hour).reason(),
            Some(StopReason::Timeout)
        );
        assert_eq!(Stop::new(None).within(None).reason(), None);
        assert_eq!(
            Stop::new(None).within(Some(Duration::ZERO)).reason(),
            Some(StopReason::Timeout)
        );
        // a request to the caller's stop reaches the job's, and back
        let job = caller.within(hour);
        assert_eq!(job.reason(), None);
        caller.request();
        assert_eq!(job.reason(), Some(StopReason::Requested));
        let other = Stop::new(None);
        other.within(None).request();
        assert_eq!(other.reason(), Some(StopReason::Requested));
    }

    #[test]
    fn utc_dates_follow_the_gregorian_calendar() {
        assert_eq!(utc(0), ("1970-01-01".into(), "00:00:00".into()));
        // a leap day, and a well-known Unix time
        assert_eq!(utc(951_782_400), ("2000-02-29".into(), "00:00:00".into()));
        assert_eq!(utc(1_700_000_000), ("2023-11-14".into(), "22:13:20".into()));
        assert_eq!(utc(4_107_542_399), ("2100-02-28".into(), "23:59:59".into()));
    }

    /// The parts of a job that the run reads: not its text, nor its format.
    fn content(job: &Job) -> (&str, Option<f64>, &toml::Table) {
        (job.name(), job.timeout_minutes, job.task())
    }

    #[test]
    fn every_built_in_job_is_the_same_job_in_json_and_yaml() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("jobs");
        let mut count = 0;
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if Format::from_path(&path) != Some(Format::Toml) {
                continue;
            }
            let job = Job::load(&path).unwrap();
            for format in Format::ALL {
                let text = job.to_text(format).unwrap();
                let again = Job::parse_as(&text, format)
                    .unwrap_or_else(|e| panic!("{} as {format:?}: {e}\n{text}", path.display()));
                assert_eq!(content(&again), content(&job), "{}", path.display());
                assert_eq!(again.format(), format);
            }
            count += 1;
        }
        assert!(count >= 5, "{count} jobs");
    }

    #[test]
    fn floats_come_back_to_the_last_bit_in_every_format() {
        let values = [
            0.1,
            1.55,
            1.0 / 3.0,
            -2.5e-300,
            f64::MIN_POSITIVE,
            5e-324,
            f64::MAX,
            1e21,
            123_456_789.123_456_78,
        ];
        let mut task = toml::Table::new();
        for (i, v) in values.iter().enumerate() {
            task.insert(format!("v{i}"), toml::Value::Float(*v));
        }
        task.insert("count".into(), toml::Value::Integer(20));
        task.insert("whole".into(), toml::Value::Float(20.0));
        let job = Job {
            name: "floats".into(),
            timeout_minutes: Some(0.1),
            timeout: Some(Duration::from_secs_f64(6.0)),
            solver: None,
            direct: Choice::Auto,
            device: None,
            circuit: None,
            task,
            text: String::new(),
            format: Format::Toml,
            dir: None,
        };
        for format in Format::ALL {
            let again = Job::parse_as(&job.to_text(format).unwrap(), format).unwrap();
            for (i, v) in values.iter().enumerate() {
                let got = again.task()[&format!("v{i}")].as_float().unwrap();
                assert_eq!(got.to_bits(), v.to_bits(), "{format:?} v{i}: {got:e}");
            }
            // an integer stays an integer, and 20.0 a float
            assert_eq!(
                again.task()["count"],
                toml::Value::Integer(20),
                "{format:?}"
            );
            assert_eq!(
                again.task()["whole"],
                toml::Value::Float(20.0),
                "{format:?}"
            );
            assert_eq!(again.timeout_minutes, Some(0.1), "{format:?}");
        }
    }

    #[test]
    fn json_and_yaml_jobs_read_as_the_toml_one() {
        let json = r#"{"name": "strip-modes", "timeout_minutes": 1.5, "task": {"width_um": 0.5}}"#;
        let yaml = "# a comment\nname: strip-modes\ntimeout_minutes: 1.5\ntask:\n  width_um: 0.5\n";
        let toml = Job::parse(JOB).unwrap();
        for (text, format) in [(json, Format::Json), (yaml, Format::Yaml)] {
            let job = Job::parse_as(text, format).unwrap();
            assert_eq!(content(&job), content(&toml), "{format:?}");
            assert_eq!(job.timeout(), Some(Duration::from_secs(90)));
        }
    }

    #[test]
    fn yaml_keeps_yaml_1_1_words_as_strings() {
        let yaml = "name: words\ntask:\n  a: no\n  b: on\n  c: yes\n  d: off\n  e: 1:30\n  f: false\n  g: 1e3\n";
        let job = Job::parse_as(yaml, Format::Yaml).unwrap();
        let task = job.task();
        for (key, word) in [
            ("a", "no"),
            ("b", "on"),
            ("c", "yes"),
            ("d", "off"),
            ("e", "1:30"),
        ] {
            assert_eq!(task[key], toml::Value::String(word.into()), "{key}");
        }
        assert_eq!(task["f"], toml::Value::Boolean(false));
        assert_eq!(task["g"].as_float(), Some(1000.0));
    }

    #[test]
    fn the_direct_solver_is_read_and_written_in_every_format() {
        // none named: auto, and nothing written
        let plain = Job::parse("name = \"a\"\n").unwrap();
        assert_eq!(plain.direct(), &Choice::Auto);
        assert!(!plain.to_text(Format::Toml).unwrap().contains("solver"));
        for (text, format) in [
            (
                "name = \"a\"\n\n[solver]\ndirect = \"pardiso\"\n",
                Format::Toml,
            ),
            (
                "{\"name\": \"a\", \"solver\": {\"direct\": \"pardiso\"}}",
                Format::Json,
            ),
            ("name: a\nsolver:\n  direct: pardiso\n", Format::Yaml),
        ] {
            let job = Job::parse_as(text, format).unwrap();
            assert_eq!(job.direct(), &Choice::Named("pardiso".into()), "{format:?}");
            // each choice comes back the same from every format
            for to in Format::ALL {
                let again = Job::parse_as(&job.to_text(to).unwrap(), to).unwrap();
                assert_eq!(again.direct(), job.direct(), "{format:?} to {to:?}");
            }
        }
        for (name, choice) in [("auto", Choice::Auto), ("photonoxide", Choice::Photonoxide)] {
            let text = format!("name = \"a\"\n\n[solver]\ndirect = \"{name}\"\n");
            let job = Job::parse(&text).unwrap();
            assert_eq!(job.direct(), &choice);
            let again = Job::parse(&job.to_text(Format::Toml).unwrap()).unwrap();
            assert_eq!(again.direct(), &choice);
        }
        // not a backend's name, and a setting the table doesn't have
        assert!(Job::parse("name = \"a\"\n\n[solver]\ndirect = \"Intel MKL\"\n").is_err());
        let e = Job::parse("name = \"a\"\n\n[solver]\niterative = \"qmr\"\n")
            .unwrap_err()
            .to_string();
        assert!(e.contains("iterative"), "{e}");
    }

    #[test]
    fn every_format_refuses_unknown_fields_and_nulls_and_says_where() {
        for (text, format) in [
            ("name = \"a\"\nspeed = 3\n", Format::Toml),
            ("{\n  \"name\": \"a\",\n  \"speed\": 3\n}", Format::Json),
            ("name: a\nspeed: 3\n", Format::Yaml),
        ] {
            let e = Job::parse_as(text, format).unwrap_err().to_string();
            assert!(e.contains("speed"), "{format:?}: {e}");
        }
        for (text, format) in [
            ("{\"name\": \"a\", \"task\": {\"x\": null}}", Format::Json),
            ("name: a\ntask:\n  x: null\n", Format::Yaml),
            ("name: a\ntask:\n  x: ~\n", Format::Yaml),
        ] {
            assert!(Job::parse_as(text, format).is_err(), "{format:?}: {text}");
        }
        // the line of the mistake: JSON's third line, YAML's second
        let e = Job::parse_as("{\n  \"name\": \"a\",\n  \"task\": [}\n", Format::Json)
            .unwrap_err()
            .to_string();
        assert!(e.contains("line 3"), "{e}");
        let e = Job::parse_as("name: a\ntask: [1, 2\n", Format::Yaml)
            .unwrap_err()
            .to_string();
        assert!(e.contains("line 2"), "{e}");
    }

    #[test]
    fn a_files_extension_names_its_format() {
        for (name, format) in [
            ("a.toml", Some(Format::Toml)),
            ("a.json", Some(Format::Json)),
            ("a.yaml", Some(Format::Yaml)),
            ("a.YML", Some(Format::Yaml)),
            ("a.txt", None),
            ("toml", None),
        ] {
            assert_eq!(Format::from_path(Path::new(name)), format, "{name}");
        }
    }

    #[test]
    fn a_run_keeps_the_job_in_its_own_format() {
        let root = TempDir::new();
        let json = r#"{"name": "strip-modes", "task": {"width_um": 0.5}}"#;
        let job = Job::parse_as(json, Format::Json).unwrap();
        let run = Run::create(&root.0, &job).unwrap();
        assert_eq!(
            fs::read_to_string(run.dir().join("job.json")).unwrap(),
            json
        );
        assert!(!run.dir().join("job.toml").exists());
    }
}
