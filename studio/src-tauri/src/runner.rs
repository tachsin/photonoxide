//! `photonoxide bench --tier`: the catalogue's problems (photonoxide::bench::catalogue) over
//! backends and thread counts, each run in a process of its own, recorded in a results
//! database, and summarized from it.
//!
//! - **Isolation:** a child process per run, so its peak memory is its own and a library that
//!   aborts or hangs can't take the runner down; a timeout per run; a run predicted to need
//!   more than the machine's free memory is skipped, and says so.
//! - **The database:** JSON lines, a [`Record`] each, in the app's data folder (`--db` to name
//!   another), every record tagged with the machine and photonoxide's version. `--import`
//!   appends another machine's.
//! - **Summaries** ([`summarize`]) from the records of one machine: each backend's speed-up and
//!   memory against photonoxide's own, time against unknowns with its fitted exponent, the
//!   crossover sizes and the best backend per family and size. A run that misses its accuracy
//!   check doesn't count.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant, SystemTime};

use photonoxide::backend::{self, Choice};
use photonoxide::bench::catalogue::{self, Entry, Tier};
use photonoxide::bench::{Measurement, Phase};
use serde::{Deserialize, Serialize};

use crate::bench::{cpu_name, gigabytes, grouped, memory};

/// The machine a record was measured on.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Machine {
    /// The processor's name.
    pub cpu: String,
    /// Its logical processors.
    pub logical_processors: usize,
    /// Its memory, in bytes.
    pub memory_bytes: Option<u64>,
    /// `windows`, `linux` or `macos`.
    pub os: String,
}

impl Machine {
    pub(crate) fn here() -> Machine {
        Machine {
            cpu: cpu_name().unwrap_or_else(|| "an unnamed processor".into()),
            logical_processors: std::thread::available_parallelism().map_or(1, |n| n.get()),
            memory_bytes: system_memory().map(|m| m.0),
            os: std::env::consts::OS.into(),
        }
    }
}

/// One run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Record {
    /// The catalogue's id.
    pub id: String,
    /// Its family's name.
    pub family: String,
    /// The size its family is scaled by.
    pub size: usize,
    /// The grid.
    pub grid: String,
    /// The unknowns.
    pub unknowns: usize,
    /// The backend's name.
    pub backend: String,
    /// Its library's version.
    pub backend_version: String,
    /// Whether it declares the same bits on every run.
    pub deterministic: bool,
    /// The threads it ran on (`RAYON_NUM_THREADS`).
    pub threads: usize,
    /// The timed phases.
    pub phases: Vec<Phase>,
    /// Their total, in seconds.
    pub seconds: f64,
    /// The process's peak resident set, in bytes.
    pub peak_bytes: Option<u64>,
    /// The factors' entries of its sparse direct solve, as the backend reports them (none in
    /// records written before they were kept).
    #[serde(default)]
    pub factor_entries: Option<u64>,
    /// The accuracy check's error, and what it allows (none: the problem checks itself).
    pub error: Option<f64>,
    /// The error the check allows.
    pub tolerance: Option<f64>,
    /// Whether the check passed: a run that fails it doesn't count.
    pub accurate: bool,
    /// Why the run failed or was skipped.
    pub failure: Option<String>,
    /// The one-minute load average when it started, where the system says (Linux, macOS): a
    /// machine busy with other work is flagged.
    pub load: Option<f64>,
    /// The machine.
    pub machine: Machine,
    /// photonoxide's version.
    pub version: String,
    /// When, in seconds since 1970.
    pub unix_seconds: u64,
}

impl Record {
    /// Whether it counts in the summaries: run, and accurate.
    pub fn counts(&self) -> bool {
        self.failure.is_none() && self.accurate
    }
}

/// What a child process reports.
#[derive(Serialize, Deserialize)]
struct Report {
    measurement: Measurement,
    peak_bytes: Option<u64>,
    version: String,
    deterministic: bool,
}

/// The app's data folder, as Tauri finds it for `gr.tachsin.photonoxide`.
fn data_folder() -> Option<PathBuf> {
    const ID: &str = "gr.tachsin.photonoxide";
    let var = |v: &str| std::env::var_os(v).map(PathBuf::from);
    Some(match std::env::consts::OS {
        "windows" => var("APPDATA")?.join(ID),
        "macos" => var("HOME")?
            .join("Library")
            .join("Application Support")
            .join(ID),
        _ => var("XDG_DATA_HOME")
            .or_else(|| var("HOME").map(|h| h.join(".local").join("share")))?
            .join(ID),
    })
}

/// The default database.
pub fn default_database() -> Option<PathBuf> {
    data_folder().map(|d| d.join("bench").join("results.jsonl"))
}

/// The records of a database, those that read; a missing file has none.
///
/// # Errors
///
/// The file's, other than its absence.
pub fn read(path: &Path) -> Result<Vec<Record>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    Ok(text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect())
}

/// Appends records to a database.
///
/// # Errors
///
/// The file's.
pub fn append(path: &Path, records: &[Record]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    for r in records {
        let line = serde_json::to_string(r).map_err(|e| e.to_string())?;
        writeln!(file, "{line}").map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

struct Options {
    tier: Tier,
    wanted: Vec<String>,
    backends: Vec<String>,
    threads: Vec<usize>,
    timeout: Duration,
    database: PathBuf,
    write: Option<PathBuf>,
    import: Option<PathBuf>,
    report_only: bool,
}

fn parse(args: &[String]) -> Option<Options> {
    let mut o = Options {
        tier: Tier::Quick,
        wanted: Vec::new(),
        backends: vec!["photonoxide".into()],
        threads: Vec::new(),
        timeout: Duration::from_secs(600),
        database: default_database()?,
        write: None,
        import: None,
        report_only: false,
    };
    let list = |s: &str| s.split(',').map(|x| x.trim().to_string()).collect();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--tier" => o.tier = Tier::parse(it.next()?)?,
            "--backends" => o.backends = list(it.next()?),
            "--threads" => {
                o.threads = it
                    .next()?
                    .split(',')
                    .map(|n| n.trim().parse().ok().filter(|&n| n > 0))
                    .collect::<Option<_>>()?;
            }
            "--timeout" => o.timeout = Duration::from_secs(it.next()?.parse().ok()?),
            "--db" => o.database = PathBuf::from(it.next()?),
            "--write" => o.write = Some(PathBuf::from(it.next()?)),
            "--import" => o.import = Some(PathBuf::from(it.next()?)),
            "--report" => o.report_only = true,
            _ if !a.starts_with("--") => o.wanted.push(a.clone()),
            _ => return None,
        }
    }
    if o.threads.is_empty() {
        o.threads
            .push(std::thread::available_parallelism().map_or(1, |n| n.get()));
    }
    Some(o)
}

/// The backends of `wanted` (`all`: every one available), photonoxide's own first.
fn backends(wanted: &[String]) -> Vec<String> {
    let available: Vec<String> = backend::direct_solvers()
        .unwrap_or_default()
        .into_iter()
        .chain(backend::iterative_solvers().unwrap_or_default())
        .filter(|l| l.capabilities.is_some())
        .map(|l| l.name)
        .collect();
    let mut out = vec!["photonoxide".to_string()];
    for name in wanted {
        if name == "all" {
            out.extend(available.iter().cloned());
        } else {
            out.push(name.clone());
        }
    }
    let mut seen = std::collections::HashSet::new();
    out.retain(|n| seen.insert(n.clone()));
    out
}

pub fn run(args: &[String]) -> ExitCode {
    if let [flag, id, backend, out] = args
        && flag == "--child-entry"
    {
        return child(id, backend, Path::new(out));
    }
    let Some(o) = parse(args) else {
        return crate::usage();
    };
    if let Some(from) = &o.import {
        let records = match read(from) {
            Ok(r) => r,
            Err(e) => return crate::fail(e),
        };
        if let Err(e) = append(&o.database, &records) {
            return crate::fail(e);
        }
        eprintln!(
            "{} records from {} added to {}",
            records.len(),
            from.display(),
            o.database.display()
        );
        return ExitCode::SUCCESS;
    }
    let mut failed = false;
    if !o.report_only {
        photonoxide_native::register_all();
        let chosen: Vec<Entry> = catalogue::tier(o.tier)
            .into_iter()
            .filter(|e| {
                o.wanted.is_empty() || o.wanted.iter().any(|w| e.id.starts_with(w.as_str()))
            })
            .collect();
        if chosen.is_empty() {
            return crate::fail("no catalogue entry matches: see `photonoxide bench --list`");
        }
        let names = backends(&o.backends);
        for &threads in &o.threads {
            for entry in &chosen {
                for name in &names {
                    if name != "photonoxide" && !applies(entry, name) {
                        continue;
                    }
                    eprint!("{} with {name} on {threads} threads: ", entry.id);
                    let record = measure(entry, name, threads, o.timeout);
                    match &record.failure {
                        None => eprintln!(
                            "{:.2} s{}",
                            record.seconds,
                            if record.accurate { "" } else { ", inaccurate" }
                        ),
                        Some(why) => eprintln!("{why}"),
                    }
                    failed |= record.failure.is_some() && !record.failure_is_a_skip();
                    if let Err(e) = append(&o.database, std::slice::from_ref(&record)) {
                        return crate::fail(e);
                    }
                }
            }
        }
    }
    let records = match read(&o.database) {
        Ok(r) => r,
        Err(e) => return crate::fail(e),
    };
    let here = Machine::here();
    let mine: Vec<Record> = records.into_iter().filter(|r| r.machine == here).collect();
    let report = markdown(&here, &mine, &summarize(&mine));
    if let Some(path) = &o.write {
        if let Err(e) = std::fs::write(path, &report) {
            return crate::fail(format!("{}: {e}", path.display()));
        }
    } else {
        print!("{report}");
    }
    if failed {
        crate::fail("some runs failed: see their records")
    } else {
        ExitCode::SUCCESS
    }
}

impl Record {
    fn failure_is_a_skip(&self) -> bool {
        self.failure
            .as_deref()
            .is_some_and(|f| f.starts_with("skipped"))
    }
}

/// Whether the backend `name` can solve `entry`: a direct one its direct solve, an iterative
/// one its plain QMR. The others run on photonoxide's own code only.
fn applies(entry: &Entry, name: &str) -> bool {
    let listed = |list: photonoxide::Result<Vec<backend::Listed>>| {
        list.unwrap_or_default().iter().any(|l| l.name == name)
    };
    (entry.takes_a_backend() && listed(backend::direct_solvers()))
        || (entry.takes_an_iterative_backend() && listed(backend::iterative_solvers()))
}

/// Runs one entry with one backend on `threads` threads, in a child process.
fn measure(entry: &Entry, backend: &str, threads: usize, timeout: Duration) -> Record {
    let mut record = Record {
        id: entry.id.clone(),
        family: entry.family.name().into(),
        size: entry.size,
        grid: entry.grid.clone(),
        unknowns: entry.unknowns,
        backend: backend.into(),
        backend_version: String::new(),
        deterministic: false,
        threads,
        phases: Vec::new(),
        seconds: f64::NAN,
        peak_bytes: None,
        factor_entries: None,
        error: None,
        tolerance: entry.tolerance().is_finite().then(|| entry.tolerance()),
        accurate: false,
        failure: None,
        load: load_average(),
        machine: Machine::here(),
        version: photonoxide::VERSION.into(),
        unix_seconds: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    };
    if let Some((_, free)) = system_memory()
        && entry.memory_bytes > free
    {
        record.failure = Some(format!(
            "skipped: it needs about {} and {} is free",
            gigabytes(entry.memory_bytes),
            gigabytes(free)
        ));
        return record;
    }
    match spawn(&entry.id, backend, threads, timeout) {
        Ok(report) => {
            let m = report.measurement;
            record.seconds = m.seconds();
            record.phases = m.phases;
            record.peak_bytes = report.peak_bytes;
            record.factor_entries = m.factor_entries;
            record.backend_version = report.version;
            record.deterministic = report.deterministic;
            record.error = m.accuracy.as_ref().map(|a| a.error);
            record.accurate = match (record.error, record.tolerance) {
                (Some(e), Some(t)) => e <= t,
                // the problem checks itself, and failed if it returned
                _ => true,
            };
        }
        Err(e) => record.failure = Some(e),
    }
    record
}

fn spawn(id: &str, backend: &str, threads: usize, timeout: Duration) -> Result<Report, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let out = std::env::temp_dir().join(format!(
        "photonoxide-bench-{}-{}-{backend}.json",
        std::process::id(),
        id.replace('/', "-")
    ));
    let _ = std::fs::remove_file(&out);
    let mut child = Command::new(exe)
        .args(["bench", "--child-entry", id, backend])
        .arg(&out)
        .env("RAYON_NUM_THREADS", threads.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => break status,
            None if start.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("timed out after {} s", timeout.as_secs()));
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    };
    let read = std::fs::read_to_string(&out);
    let _ = std::fs::remove_file(&out);
    if !status.success() {
        let mut stderr = String::new();
        if let Some(mut e) = child.stderr.take() {
            let _ = std::io::Read::read_to_string(&mut e, &mut stderr);
        }
        let last = stderr.lines().rev().find(|l| !l.trim().is_empty());
        return Err(format!("{} ({status})", last.unwrap_or("no message")));
    }
    let text = read.map_err(|e| format!("no report: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("unreadable report: {e}"))
}

/// The child: runs `id` with `backend` and writes a [`Report`] to `out`.
fn child(id: &str, backend: &str, out: &Path) -> ExitCode {
    if !matches!(backend, "photonoxide" | "faer") {
        photonoxide_native::register_all();
    }
    let Some(entry) = catalogue::entry(id) else {
        return crate::fail(format!("no catalogue entry {id}"));
    };
    let choice = match Choice::parse(backend) {
        Ok(c) => c,
        Err(e) => return crate::fail(e),
    };
    let capabilities = if entry.takes_an_iterative_backend() {
        match backend::iterative(&choice) {
            Ok(Some(s)) => s.capabilities(),
            // photonoxide's own QMR
            Ok(None) => match backend::direct(&choice) {
                Ok(s) => s.capabilities(),
                Err(e) => return crate::fail(e),
            },
            Err(e) => return crate::fail(e),
        }
    } else {
        match backend::direct(&choice) {
            Ok(s) => s.capabilities(),
            Err(e) => return crate::fail(e),
        }
    };
    let measurement = match entry.run(&choice) {
        Ok(m) => m,
        Err(e) => return crate::fail(e),
    };
    let report = Report {
        measurement,
        peak_bytes: memory::peak_bytes(),
        version: capabilities.version,
        deterministic: capabilities.deterministic,
    };
    let text = serde_json::to_string(&report).unwrap_or_default();
    if let Err(e) = std::fs::write(out, text) {
        return crate::fail(format!("{}: {e}", out.display()));
    }
    ExitCode::SUCCESS
}

/// A backend's fastest accurate run against photonoxide's own, on one problem and thread count.
#[derive(Debug, PartialEq)]
pub struct AgainstOwn {
    pub id: String,
    pub threads: usize,
    pub backend: String,
    pub seconds: f64,
    pub own_seconds: f64,
    /// photonoxide's time over the backend's.
    pub speedup: f64,
    /// The backend's peak memory over photonoxide's.
    pub memory_ratio: Option<f64>,
    /// The backend's factor entries over photonoxide's.
    pub factors_ratio: Option<f64>,
}

/// (family, backend, threads) to its (size, unknowns, seconds).
type Series = BTreeMap<(String, String, usize), Vec<(usize, usize, f64)>>;

/// What the records say.
#[derive(Debug, Default, PartialEq)]
pub struct Summary {
    /// Each run of another backend against photonoxide's own on the same problem and threads.
    pub against_own: Vec<AgainstOwn>,
    /// Time against unknowns: (family, backend, threads, the fitted exponent, the sizes).
    pub exponents: Vec<(String, String, usize, f64, usize)>,
    /// The fastest backend: (family, size, threads, backend).
    pub best: Vec<(String, usize, String, usize)>,
    /// The size from which a backend beats photonoxide's own at every larger size measured:
    /// (family, backend, threads, size).
    pub crossovers: Vec<(String, String, usize, usize)>,
}

/// The summaries of records from one machine. Each (problem, backend, threads) takes its
/// fastest accurate run; runs that failed or missed their check don't count.
pub fn summarize(records: &[Record]) -> Summary {
    // (id, backend, threads) → the fastest record
    let mut fastest: BTreeMap<(String, String, usize), &Record> = BTreeMap::new();
    for r in records.iter().filter(|r| r.counts()) {
        let key = (r.id.clone(), r.backend.clone(), r.threads);
        if fastest.get(&key).is_none_or(|f| r.seconds < f.seconds) {
            fastest.insert(key, r);
        }
    }
    let mut s = Summary::default();
    for ((id, backend, threads), r) in &fastest {
        if backend == "photonoxide" {
            continue;
        }
        if let Some(own) = fastest.get(&(id.clone(), "photonoxide".into(), *threads)) {
            let ratio = |a: Option<u64>, b: Option<u64>| match (a, b) {
                (Some(a), Some(b)) if b > 0 => Some(a as f64 / b as f64),
                _ => None,
            };
            s.against_own.push(AgainstOwn {
                id: id.clone(),
                threads: *threads,
                backend: backend.clone(),
                seconds: r.seconds,
                own_seconds: own.seconds,
                speedup: own.seconds / r.seconds,
                memory_ratio: ratio(r.peak_bytes, own.peak_bytes),
                factors_ratio: ratio(r.factor_entries, own.factor_entries),
            });
        }
    }
    // (family, backend, threads) → (size, unknowns, seconds), by size
    let mut series: Series = BTreeMap::new();
    for r in fastest.values() {
        series
            .entry((r.family.clone(), r.backend.clone(), r.threads))
            .or_default()
            .push((r.size, r.unknowns, r.seconds));
    }
    for ((family, backend, threads), points) in &mut series {
        points.sort_by(|a, b| a.0.cmp(&b.0).then(a.2.total_cmp(&b.2)));
        let xy: Vec<(f64, f64)> = points
            .iter()
            .filter(|p| p.1 > 0 && p.2 > 0.0)
            .map(|p| ((p.1 as f64).ln(), p.2.ln()))
            .collect();
        let distinct = {
            let mut u: Vec<usize> = points.iter().map(|p| p.1).collect();
            u.dedup();
            u.len()
        };
        if distinct >= 2 {
            let n = xy.len() as f64;
            let (mx, my) = (
                xy.iter().map(|p| p.0).sum::<f64>() / n,
                xy.iter().map(|p| p.1).sum::<f64>() / n,
            );
            let sxx: f64 = xy.iter().map(|p| (p.0 - mx).powi(2)).sum();
            let sxy: f64 = xy.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
            s.exponents.push((
                family.clone(),
                backend.clone(),
                *threads,
                sxy / sxx,
                distinct,
            ));
        }
    }
    // (family, size, threads) → the fastest backend
    let mut best: BTreeMap<(String, usize, usize), (&str, f64)> = BTreeMap::new();
    for r in fastest.values() {
        let key = (r.family.clone(), r.size, r.threads);
        if best.get(&key).is_none_or(|b| r.seconds < b.1) {
            best.insert(key, (&r.backend, r.seconds));
        }
    }
    s.best = best
        .into_iter()
        .map(|((family, size, threads), (backend, _))| (family, size, backend.to_string(), threads))
        .collect();
    for ((family, backend, threads), points) in &series {
        if backend == "photonoxide" {
            continue;
        }
        let Some(own) = series.get(&(family.clone(), "photonoxide".into(), *threads)) else {
            continue;
        };
        // the sizes both have, the largest first: walk down while the backend stays faster
        let mut crossover = None;
        for p in points.iter().rev() {
            match own.iter().find(|o| o.0 == p.0) {
                Some(o) if p.2 < o.2 => crossover = Some(p.0),
                Some(_) => break,
                None => continue,
            }
        }
        if let Some(size) = crossover {
            s.crossovers
                .push((family.clone(), backend.clone(), *threads, size));
        }
    }
    s
}

fn markdown(machine: &Machine, records: &[Record], s: &Summary) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Benchmarks by backend\n");
    let _ = writeln!(
        out,
        "What `photonoxide bench --tier` measured on one machine: the catalogue's problems, each \
         with each backend in a process of its own, from the results database. Only runs that \
         pass their accuracy check count. Times include each backend's transfers and analysis.\n"
    );
    let _ = writeln!(
        out,
        "- {}, {} logical processors, {}{}, {} records",
        machine.cpu,
        machine.logical_processors,
        machine.os,
        machine
            .memory_bytes
            .map_or(String::new(), |m| format!(", {} of memory", gigabytes(m))),
        records.len()
    );
    let busy: Vec<&Record> = records
        .iter()
        .filter(|r| {
            r.load
                .is_some_and(|l| l > 0.5 * machine.logical_processors as f64)
        })
        .collect();
    if !busy.is_empty() {
        let _ = writeln!(
            out,
            "- {} runs started on a busy machine (load average above half its processors)",
            busy.len()
        );
    }
    if !s.against_own.is_empty() {
        let _ = writeln!(
            out,
            "\n## Against photonoxide's own\n\nMemory and factors: the backend's peak memory and its factors' entries over photonoxide's own.\n\n| Problem | Grid | Threads | Backend | Time | photonoxide | Speed-up | Memory | Factors |\n|---|---|---:|---|---:|---:|---:|---:|---:|"
        );
        let ratio = |r: Option<f64>| r.map_or("—".into(), |r| format!("{r:.2}×"));
        for AgainstOwn {
            id,
            threads,
            backend,
            seconds: t,
            own_seconds: own,
            speedup,
            memory_ratio: memory,
            factors_ratio: factors,
        } in &s.against_own
        {
            let grid = records
                .iter()
                .find(|r| &r.id == id)
                .map_or("", |r| r.grid.as_str());
            let _ = writeln!(
                out,
                "| `{id}` | {grid} | {threads} | {backend} | {t:.3} s | {own:.3} s | {speedup:.2}× | {} | {} |",
                ratio(*memory),
                ratio(*factors)
            );
        }
    }
    if !s.exponents.is_empty() {
        let _ = writeln!(
            out,
            "\n## Time against unknowns\n\n| Family | Backend | Threads | Exponent | Sizes |\n|---|---|---:|---:|---:|"
        );
        for (family, backend, threads, exponent, sizes) in &s.exponents {
            let _ = writeln!(
                out,
                "| {family} | {backend} | {threads} | {exponent:.2} | {sizes} |"
            );
        }
    }
    if !s.best.is_empty() {
        let _ = writeln!(
            out,
            "\n## The fastest\n\n| Family | Size | Threads | Backend |\n|---|---:|---:|---|"
        );
        for (family, size, backend, threads) in &s.best {
            let _ = writeln!(out, "| {family} | {size} | {threads} | {backend} |");
        }
    }
    if !s.crossovers.is_empty() {
        let _ = writeln!(out, "\n## Crossovers\n");
        for (family, backend, threads, size) in &s.crossovers {
            let _ = writeln!(
                out,
                "- {family}: {backend} is faster than photonoxide's own from size {size} up, on {threads} threads"
            );
        }
    }
    let failed: Vec<&Record> = records.iter().filter(|r| !r.counts()).collect();
    if !failed.is_empty() {
        let _ = writeln!(out, "\n## Not counted\n");
        for r in failed {
            let why = r.failure.clone().unwrap_or_else(|| {
                format!(
                    "error {:.1e} above {:.1e}",
                    r.error.unwrap_or(f64::NAN),
                    r.tolerance.unwrap_or(f64::NAN)
                )
            });
            let _ = writeln!(
                out,
                "- `{}` with {} on {} threads: {}",
                r.id, r.backend, r.threads, why
            );
        }
    }
    let _ = writeln!(
        out,
        "\n{} problems measured: {}.",
        records
            .iter()
            .map(|r| &r.id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        grouped(records.iter().filter(|r| r.counts()).count()) + " runs counted"
    );
    out
}

/// The machine's total and free memory, in bytes.
pub(crate) fn system_memory() -> Option<(u64, u64)> {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct MemoryStatusEx {
            length: u32,
            memory_load: u32,
            total_phys: u64,
            avail_phys: u64,
            total_page_file: u64,
            avail_page_file: u64,
            total_virtual: u64,
            avail_virtual: u64,
            avail_extended_virtual: u64,
        }
        unsafe extern "system" {
            fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
        }
        let mut status = MemoryStatusEx {
            length: size_of::<MemoryStatusEx>() as u32,
            memory_load: 0,
            total_phys: 0,
            avail_phys: 0,
            total_page_file: 0,
            avail_page_file: 0,
            total_virtual: 0,
            avail_virtual: 0,
            avail_extended_virtual: 0,
        };
        // SAFETY: a MEMORYSTATUSEX with its length set, which the call fills
        let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
        (ok != 0).then_some((status.total_phys, status.avail_phys))
    }
    #[cfg(target_os = "linux")]
    {
        let text = std::fs::read_to_string("/proc/meminfo").ok()?;
        let field = |name: &str| -> Option<u64> {
            let line = text.lines().find(|l| l.starts_with(name))?;
            let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
            Some(kb * 1024)
        };
        Some((field("MemTotal:")?, field("MemAvailable:")?))
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        None
    }
}

/// The one-minute load average.
fn load_average() -> Option<f64> {
    let text = std::fs::read_to_string("/proc/loadavg").ok()?;
    text.split_whitespace().next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, size: usize, backend: &str, seconds: f64, accurate: bool) -> Record {
        Record {
            id: id.into(),
            family: "fdfd2d".into(),
            size,
            grid: format!("{size} × {size} cells"),
            unknowns: size * size,
            backend: backend.into(),
            backend_version: "1".into(),
            deterministic: backend == "photonoxide",
            threads: 4,
            phases: Vec::new(),
            seconds,
            peak_bytes: Some(100 * size as u64),
            factor_entries: Some(10 * size as u64),
            error: Some(if accurate { 1e-12 } else { 1e-3 }),
            tolerance: Some(1e-9),
            accurate,
            failure: None,
            load: None,
            machine: Machine {
                cpu: "test".into(),
                logical_processors: 4,
                memory_bytes: Some(1 << 30),
                os: "linux".into(),
            },
            version: "0.0.0".into(),
            unix_seconds: 0,
        }
    }

    #[test]
    fn the_database_round_trips() {
        let path =
            std::env::temp_dir().join(format!("photonoxide-db-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut r = record("fdfd2d/slab-ez-100", 100, "cudss", 0.5, true);
        r.failure = Some("timed out after 600 s".into());
        r.tolerance = None;
        r.factor_entries = None;
        let records = vec![
            record("fdfd2d/slab-ez-100", 100, "photonoxide", 1.0, true),
            r,
        ];
        assert_eq!(records[0].factor_entries, Some(1000));
        append(&path, &records[..1]).unwrap();
        append(&path, &records[1..]).unwrap();
        assert_eq!(read(&path).unwrap(), records);
        let _ = std::fs::remove_file(&path);
        assert!(read(&path).unwrap().is_empty());
        // a record written before the factors' entries were kept reads, without them
        let mut old = serde_json::to_value(&records[0]).unwrap();
        old.as_object_mut()
            .unwrap()
            .remove("factor_entries")
            .unwrap();
        std::fs::write(
            &path,
            format!(
                "{old}
"
            ),
        )
        .unwrap();
        let read_back = read(&path).unwrap();
        assert_eq!(read_back.len(), 1);
        assert_eq!(read_back[0].factor_entries, None);
        assert_eq!(read_back[0].peak_bytes, records[0].peak_bytes);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_summary_of_fixed_records_is_exact() {
        // photonoxide's time grows as unknowns^1.5, the GPU's as unknowns^1, from a slower start:
        // 1 s against 1.5 at 10⁴ unknowns, 8 against 6 at 4·10⁴, 64 against 24 at 1.6·10⁵
        let mut records = Vec::new();
        for size in [100usize, 200, 400] {
            let n = (size * size) as f64;
            let id = format!("fdfd2d/slab-{size}");
            records.push(record(&id, size, "photonoxide", 1e-6 * n.powf(1.5), true));
            records.push(record(&id, size, "gpu", 1.5e-4 * n, true));
            // a slower repeat and an inaccurate faster run: neither counts
            records.push(record(&id, size, "gpu", 1e3, true));
            records.push(record(&id, size, "faer", 1e-9, false));
        }
        let s = summarize(&records);
        let exponent = |b: &str| s.exponents.iter().find(|e| e.1 == b).map(|e| e.3).unwrap();
        assert!((exponent("photonoxide") - 1.5).abs() < 1e-12);
        assert!((exponent("gpu") - 1.0).abs() < 1e-12);
        assert!(s.exponents.iter().all(|e| e.1 != "faer"));
        let speedups: Vec<f64> = s.against_own.iter().map(|a| a.speedup).collect();
        assert_eq!(s.against_own.len(), 3);
        assert!((speedups[0] - 2.0 / 3.0).abs() < 1e-12, "{speedups:?}");
        assert!((speedups[2] - 8.0 / 3.0).abs() < 1e-12, "{speedups:?}");
        assert_eq!(s.against_own[0].memory_ratio, Some(1.0));
        assert_eq!(s.against_own[0].factors_ratio, Some(1.0));
        assert_eq!(s.crossovers, vec![("fdfd2d".into(), "gpu".into(), 4, 200)]);
        let best: Vec<&str> = s.best.iter().map(|b| b.2.as_str()).collect();
        assert_eq!(best, ["photonoxide", "gpu", "gpu"]);
    }

    #[test]
    fn the_summary_shows_the_factors_beside_the_memory() {
        let own = record("fdfd2d/slab-100", 100, "photonoxide", 1.0, true);
        let mut gpu = record("fdfd2d/slab-100", 100, "gpu", 0.5, true);
        gpu.factor_entries = Some(500);
        // a backend that doesn't say, or a record from before they were kept
        let mut faer = record("fdfd2d/slab-100", 100, "faer", 0.8, true);
        faer.factor_entries = None;
        let records = vec![own, gpu, faer];
        let md = markdown(&records[0].machine, &records, &summarize(&records));
        assert!(md.contains("| Speed-up | Memory | Factors |"), "{md}");
        assert!(
            md.contains("| gpu | 0.500 s | 1.000 s | 2.00× | 1.00× | 0.50× |"),
            "{md}"
        );
        assert!(
            md.contains("| faer | 0.800 s | 1.000 s | 1.25× | 1.00× | — |"),
            "{md}"
        );
    }

    #[test]
    fn options_parse() {
        let args: Vec<String> = [
            "--tier",
            "standard",
            "--backends",
            "all",
            "--threads",
            "1,20",
            "--timeout",
            "30",
            "--db",
            "x.jsonl",
            "fdfd2d",
        ]
        .map(String::from)
        .to_vec();
        let o = parse(&args).unwrap();
        assert_eq!(o.tier, Tier::Standard);
        assert_eq!(o.backends, ["all"]);
        assert_eq!(o.threads, [1, 20]);
        assert_eq!(o.timeout, Duration::from_secs(30));
        assert_eq!(o.database, PathBuf::from("x.jsonl"));
        assert_eq!(o.wanted, ["fdfd2d"]);
        assert!(parse(&["--tier".to_string(), "huge".into()]).is_none());
    }

    #[test]
    fn the_machine_and_its_memory_are_read() {
        let m = Machine::here();
        assert!(m.logical_processors >= 1);
        if cfg!(any(windows, target_os = "linux")) {
            let (total, free) = system_memory().unwrap();
            assert!(total >= free && free > 0);
        }
    }
}
