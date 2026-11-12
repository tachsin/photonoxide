//! The `photonoxide` program: opens the studio; runs a job, live in the studio window or
//! headless, its sweep farmed to workers if asked; serves as a worker; replays a run in the
//! studio; runs the built-in examples; checks the validation report; times the benchmark
//! problems.

// the headless program's modules: every command but the window's
mod auto;
mod bench;
mod examples;
mod exports;
mod farm;
#[cfg(any(test, feature = "window"))]
mod job_text;
mod libraries;
mod optimize;
mod references;
mod runner;
mod signals;
mod watch;
// the window's modules (the `window` feature, on by default): the studio, its pages, the
// nightly channel and the Windows setup
#[cfg(feature = "window")]
mod academy;
#[cfg(feature = "window")]
mod benchmarks;
#[cfg(feature = "window")]
mod channels;
#[cfg(feature = "window")]
mod charts;
#[cfg(feature = "window")]
mod circuits;
#[cfg(feature = "window")]
mod devices;
#[cfg(feature = "window")]
mod diagrams;
#[cfg(feature = "window")]
mod drawn;
#[cfg(feature = "window")]
mod expressions;
#[cfg(feature = "window")]
mod fabrication;
#[cfg(feature = "window")]
mod form;
#[cfg(feature = "window")]
mod frames;
#[cfg(feature = "window")]
mod materials;
#[cfg(feature = "window")]
mod media;
#[cfg(all(test, feature = "window"))]
mod navigation;
#[cfg(feature = "window")]
mod nonlinear;
#[cfg(feature = "window")]
mod papers;
#[cfg(feature = "window")]
mod settings;
#[cfg(feature = "window")]
mod setup;
#[cfg(feature = "window")]
mod sources;
#[cfg(feature = "window")]
mod studio;
#[cfg(feature = "window")]
mod tasks;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use photonoxide::job;
use photonoxide::run::{Job, Run, Stop, StopReason};

const USAGE: &str = "usage:
  photonoxide
      open the studio, to run a job from jobs/ or reopen a run from runs/
  photonoxide run <job.toml|.json|.yaml> [--out <dir>] [--headless] [--linger <seconds>]
                  [--timeout <minutes>]
      run a job; the studio window shows it live and closes by itself when it's done
      (--headless: no window; --out: where run directories go, default runs/;
       --linger: how long the window stays after the run, default 5 s;
       --timeout: a hard limit, the shorter of it and the job's own applying).
      [--workers <n>] [--worker <host:port>]... [--token <token>] [--task-timeout <seconds>]
      farm the sweep's points: to n workers started here, and to workers listening
      elsewhere (their token: --token or PHOTONOXIDE_FARM_TOKEN); the same record, gathered
      in order; a point past --task-timeout fails, and its worker is dropped
      It prints the run's directory, which holds results.json at the end
      (docs/runs.md); it exits 0 finished, 1 failed, 2 usage, 3 stopped by its
      timeout, 4 stopped on request (the window closed early)
  photonoxide worker [--listen <address>] [--token <token>] [--slots <n>] [--once]
      serve coordinators' sweeps and jobs, one coordinator at a time, on 127.0.0.1:7878
      by default (an address beyond this machine needs a token); --slots: tasks at once,
      default 1; --once: one coordinator, then exit. Not for untrusted networks: the
      traffic isn't encrypted
  photonoxide new <kind> [<file>]
      write a short job of that kind to start from, which runs as it is: in YAML with a
      comment on each field, or in TOML or JSON by the file's extension (without the
      comments); new-<kind>.yaml here by default, and never over a file that exists
      (`photonoxide new --list` lists the kinds)
  photonoxide schema [--results] [--write <file> | --check <file>]
      print the job file's JSON Schema, which editors complete and check jobs with, or
      with --results results.json's, which any language generates its types from;
      --write saves it, --check fails unless <file> holds exactly this schema
  photonoxide view <run directory>
      replay a finished run in the studio
  photonoxide export <run directory> [--tables <name,...>] [--fields <file,...|all>]
                     [--precision f64|f32] [--level <0-9>] [--to <dir>] [--list]
      write a run's tables as CSV (every one it has by default) into its directory, or into
      --to: modes, sweep, s_parameters, spectra, resonances, study, history, static,
      transient. Its fields (.npz) are written as it runs, when its job's [task.export] asks
      for them; --fields writes them again into --to, at --precision (f32: the fields
      downcast) and deflate --level (0 stores, 9 is smallest; 6 by default). --list lists
      its tables and files, each field file with its size written again at that precision
      and level, estimated before writing
  photonoxide example <name>
      run one of the built-in examples, each a published result reproduced
      (`photonoxide example --list` lists them)
  photonoxide validate [--write <file> | --check <file>]
      run every validation case and print the report; --write saves it, --check fails
      unless <file> holds exactly this report
  photonoxide references [--write <file> | --check <file> | --crossref [--all]]
      the references index (docs/references.md): every source docs/references.yaml lists and
      every place the repository cites it; --write saves it, --check fails unless <file> holds
      exactly it; --crossref adds the entries of newly cited DOIs from Crossref (--all
      compares every entry with Crossref)
  photonoxide bench [--all] [--threads <n,n,...>] [--write <file>] [--json <file>] [<id>...]
      time the benchmark problems and the slowest examples, each in its own process, and
      print the report (--all: the heavy ones too; <id>: only those whose id starts with it;
      --threads: the thread counts, default all the machine's; `--list` lists them)
  photonoxide bench --export <dir> [<id>...]
      write the direct solvers' systems (slab-2d, strip-24, strip-32, strip-40) as Matrix
      Market files with their solutions, for other solvers to factorize
  photonoxide bench --tier <quick|standard|full> [--backends <name,...|all>] [--threads <n,...>]
                    [--timeout <seconds>] [--db <file>] [--write <file>] [<id>...]
      run the catalogue's problems with each backend (photonoxide's own always) in a process
      each, record every run in the results database (default: the app's data folder) and
      print the summary of this machine's records (--write: to a file; --timeout: per run,
      default 600 s)
  photonoxide bench --report [--db <file>] [--write <file>]
      the summary alone, from the database
  photonoxide bench --import <file> [--db <file>]
      add another database's records, from this machine or another
  photonoxide libraries [--json | --write <file>]
      the external libraries found here (oneMKL, OpenBLAS, the CUDA runtime, cuSPARSE, cuDSS,
      MUMPS, SuperLU, and Accelerate on macOS): where each was found, its version, and the
      backends that passed their smoke tests
  photonoxide nightly <check | prerequisites | build <commit> | install | rollback>
      the nightly channel: main's head and the commits since this build, what building
      here needs, a build of main at its head (which runs main's code), its install in
      place of this copy, and the way back
  photonoxide setup [/P | /S] [/UPDATE] [/R] [/D=<folder>] [/ARGS <args>...]
      on Windows, what photonoxide's setup runs: the install, in a window of its own (/P: the
      progress only, /S: nothing; /UPDATE or /R: start the program again, with the arguments
      after /ARGS)
  photonoxide uninstall [/S] [--delete-data]
      on Windows, what Apps & Features runs: removes this install (--delete-data: its
      settings and benchmark records too)
  photonoxide --version";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // SIGTERM and SIGINT: a run stops and keeps its record, anything else ends (Unix)
    signals::listen();
    // a job's check and its run may ask for Zstandard: the user's libzstd, when it is here
    if matches!(
        args.first().map(String::as_str),
        None | Some("run" | "worker")
    ) {
        optimize::exporters();
    }
    match args.first().map(String::as_str) {
        None => open(),
        Some("run") => run(&args[1..]),
        Some("new") => new(&args[1..]),
        Some("schema") => schema(&args[1..]),
        Some("view") => view(&args[1..]),
        Some("export") => export(&args[1..]),
        Some("worker") => farm::worker(&args[1..]),
        Some("validate") => validate(&args[1..]),
        Some("references") => references::run(&args[1..]),
        Some("example") => example(&args[1..]),
        Some("bench") => bench::run(&args[1..]),
        Some("libraries") => libraries::run(&args[1..]),
        #[cfg(feature = "window")]
        Some("nightly") => channels::run(&args[1..]),
        #[cfg(feature = "window")]
        Some("setup") => setup::run(&args[1..]),
        #[cfg(feature = "window")]
        Some("uninstall") => setup::remove(&args[1..]),
        #[cfg(not(feature = "window"))]
        Some(command @ ("nightly" | "setup" | "uninstall")) => no_window(&format!("`{command}`")),
        Some("--version" | "-V") => {
            println!("photonoxide {}", version());
            ExitCode::SUCCESS
        }
        _ => usage(),
    }
}

/// This build's version: a release's, or a nightly's, with the commit it was built from when
/// known.
#[cfg(feature = "window")]
fn version() -> String {
    let build = channels::this_build();
    match (build.nightly(), build.short) {
        (false, Some(commit)) => format!(
            "{} ({commit}{})",
            build.label,
            if build.dirty {
                ", with changes not committed"
            } else {
                ""
            }
        ),
        _ => build.label,
    }
}

/// This build's version, with the commit it was built from when known (`build.rs`; a build
/// without the window is never the nightly channel's).
#[cfg(not(feature = "window"))]
fn version() -> String {
    let commit = env!("PHOTONOXIDE_COMMIT");
    let dirty = env!("PHOTONOXIDE_DIRTY") == "true";
    match commit.get(..7) {
        Some(short) if commit.chars().all(|c| c.is_ascii_hexdigit()) => format!(
            "{} ({short}{}, without the window)",
            photonoxide::VERSION,
            if dirty {
                ", with changes not committed"
            } else {
                ""
            }
        ),
        _ => format!("{} (without the window)", photonoxide::VERSION),
    }
}

#[cfg(feature = "window")]
fn open() -> ExitCode {
    console::release();
    setup::at_start();
    match studio::show(None, None) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(e),
    }
}

#[cfg(not(feature = "window"))]
fn open() -> ExitCode {
    no_window("the studio")
}

/// What a build without the window says of what needs it: exit 2, as a command it doesn't know.
#[cfg(not(feature = "window"))]
fn no_window(what: &str) -> ExitCode {
    eprintln!(
        "this build of photonoxide has no window: {what} needs the studio's build (studio/README.md).          Every headless command is here: `photonoxide --help` lists them"
    );
    ExitCode::from(2)
}

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

fn fail(e: impl std::fmt::Display) -> ExitCode {
    eprintln!("error: {e}");
    ExitCode::FAILURE
}

fn run(args: &[String]) -> ExitCode {
    let mut job_path = None;
    let mut out = PathBuf::from("runs");
    let mut headless = false;
    let mut linger = Duration::from_secs(5);
    let mut timeout: Option<Duration> = None;
    let mut farming = farm::Options::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match farming.take(a, &mut it) {
            Some(true) => continue,
            Some(false) => {}
            None => return usage(),
        }
        match a.as_str() {
            "--headless" => headless = true,
            "--out" => match it.next() {
                Some(dir) => out = PathBuf::from(dir),
                None => return usage(),
            },
            "--linger" => match it.next().and_then(|s| s.parse::<f64>().ok()) {
                Some(s) if s.is_finite() && s >= 0.0 => linger = Duration::from_secs_f64(s),
                _ => return usage(),
            },
            "--timeout" => match it.next().and_then(|s| s.parse::<f64>().ok()) {
                Some(m) if m.is_finite() && m > 0.0 => {
                    timeout = Some(Duration::from_secs_f64(m * 60.0));
                }
                _ => return usage(),
            },
            _ if job_path.is_none() && !a.starts_with("--") => job_path = Some(PathBuf::from(a)),
            _ => return usage(),
        }
    }
    let Some(job_path) = job_path else {
        return usage();
    };
    if cfg!(not(feature = "window")) && !headless {
        eprintln!("this build of photonoxide has no window: the run is headless");
    }
    let job = match Job::load(&job_path) {
        Ok(j) => j,
        Err(e) => return fail(e),
    };
    // what the job does that it may not mean, though it runs: a chip's misspelled parameter
    for w in job::warnings(&job) {
        eprintln!("warning: {w}");
    }
    // this machine's benchmark records for auto, and the external libraries if the job needs
    // them
    auto::prepare(&job);
    let mut record = match Run::create(&out, &job) {
        Ok(r) => r,
        Err(e) => return fail(e),
    };
    let dir = record.dir().to_path_buf();
    println!("{}", dir.display());
    // the shorter of the job's own limit and the command line's
    let stop = Stop::new(job.timeout()).within(timeout);
    // the sweep's points on workers, here or on other machines: the same record, gathered in
    // order as the points complete
    let workers = if !job::farms(&job) {
        if farming.workers > 0 || !farming.remote.is_empty() {
            eprintln!("the job has no sweep to farm: it runs here");
        }
        None
    } else {
        match farming.farm() {
            Ok(Some(f)) => {
                eprintln!(
                    "the sweep's points farmed to {} workers: {}",
                    f.addresses().len(),
                    f.addresses().join(", ")
                );
                f.farm_sweep(&job, &mut record, &stop);
                Some(f)
            }
            Ok(None) => None,
            Err(e) => return fail(e),
        }
    };
    let (done, finished) = std::sync::mpsc::channel();
    let worker = {
        let (job, stop) = (job.clone(), stop.clone());
        std::thread::spawn(move || {
            let result = crate::optimize::execute(&job, &mut record, &stop);
            // why it stopped, if it did, as the run ended: the window closing afterwards asks
            // for a stop too
            let stopped = stop.reason();
            // the local workers end with the run
            drop(workers);
            let _ = done.send(());
            result.map(|()| stopped)
        })
    };
    // SIGTERM or SIGINT stops it as the window closing does
    signals::stop_run(&stop);
    #[cfg(feature = "window")]
    if !headless {
        // the window runs on the main thread (some platforms require it) while the job works,
        // and closes `linger` after the job finished
        let live = studio::Live {
            linger,
            finished,
            stop: stop.clone(),
        };
        if let Err(e) = studio::show(Some(&dir), Some(live)) {
            stop.request();
            let _ = worker.join();
            return fail(e);
        }
        // closed early: the job stops at its next check
        stop.request();
    }
    #[cfg(not(feature = "window"))]
    let _ = (headless, linger, finished);
    match worker.join() {
        Ok(Ok(stopped)) => {
            // what results.json says of grids not known to have converged, said here too
            if let Ok(results) = job::results::Results::read(&dir) {
                for line in results.convergence_lines() {
                    eprintln!("{line}");
                }
            }
            ended(stopped)
        }
        Ok(Err(e)) => fail(e),
        Err(_) => fail("the job panicked"),
    }
}

/// The exit code of a run that ended without an error: 0 finished, 3 stopped by its timeout, 4
/// stopped on request (docs/runs.md).
fn ended(stopped: Option<StopReason>) -> ExitCode {
    match stopped {
        None => ExitCode::SUCCESS,
        Some(StopReason::Timeout) => {
            eprintln!("stopped: the run reached its timeout");
            ExitCode::from(3)
        }
        Some(StopReason::Requested) => {
            eprintln!("stopped: the run was stopped before it finished");
            ExitCode::from(4)
        }
    }
}

/// Whether a command's arguments ask for its usage (`--help` or `-h`), which a command taking a
/// run directory first would otherwise read as one.
fn asks_for_help(args: &[String]) -> bool {
    args.iter().any(|a| a == "--help" || a == "-h")
}

fn view(args: &[String]) -> ExitCode {
    let [dir] = args else {
        return usage();
    };
    if asks_for_help(args) {
        return usage();
    }
    let dir = Path::new(dir);
    if !dir.join("events.jsonl").is_file() {
        return fail(format!("{} has no events.jsonl", dir.display()));
    }
    #[cfg(feature = "window")]
    {
        match studio::show(Some(dir), None) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => fail(e),
        }
    }
    #[cfg(not(feature = "window"))]
    {
        no_window("`view`")
    }
}

fn export(args: &[String]) -> ExitCode {
    let Some((dir, rest)) = args.split_first() else {
        return usage();
    };
    if asks_for_help(args) {
        return usage();
    }
    let dir = Path::new(dir);
    if !dir.join("events.jsonl").is_file() {
        return fail(format!("{} has no events.jsonl", dir.display()));
    }
    let (mut tables, mut fields, mut to, mut list) = (None, None, None, false);
    let (mut precision, mut level) = ("f64".to_owned(), 6u8);
    let mut rest = rest.iter();
    let names = |s: &str| s.split(',').map(str::to_owned).collect::<Vec<_>>();
    while let Some(flag) = rest.next() {
        match (flag.as_str(), rest.clone().next()) {
            ("--list", _) => list = true,
            ("--tables", Some(n)) => {
                tables = Some(names(n));
                rest.next();
            }
            ("--fields", Some(n)) => {
                fields = Some((n != "all").then(|| names(n)));
                rest.next();
            }
            ("--precision", Some(p)) => {
                precision = p.clone();
                rest.next();
            }
            ("--level", Some(l)) => {
                let Ok(l) = l.parse::<u8>() else {
                    return fail(format!("--level takes 0 to 9, not {l}"));
                };
                level = l;
                rest.next();
            }
            ("--to", Some(path)) => {
                to = Some(PathBuf::from(path));
                rest.next();
            }
            _ => return usage(),
        }
    }
    if list {
        let exports = match exports::run_exports(dir) {
            Ok(e) => e,
            Err(e) => return fail(e),
        };
        let estimated = match photonoxide::facade::estimate_fields(dir, &precision, level) {
            Ok(f) => f,
            Err(e) => return fail(e),
        };
        for t in &exports.tables {
            println!("{:<18} {:>6} rows  {}", t.name, t.rows, t.file);
        }
        for f in exports.files.iter().filter(|f| f.name.ends_with(".csv")) {
            println!("{:<18} {:>12} bytes", f.name, f.bytes);
        }
        for f in &estimated {
            let again = if !f.rewritable {
                "copied as it is".to_owned()
            } else {
                format!(
                    "{} {} bytes at {}, level {level}",
                    if f.exact { "exactly" } else { "about" },
                    f.estimate,
                    f.precision
                )
            };
            println!(
                "{:<18} {:>12} bytes  written again: {again}",
                f.file, f.bytes
            );
        }
        return ExitCode::SUCCESS;
    }
    let mut written = Vec::new();
    let only_fields = fields.is_some() && tables.is_none();
    if let Some(files) = fields {
        let Some(out) = to.as_deref() else {
            return fail("--fields needs --to: a run's own fields aren't written over");
        };
        match photonoxide::facade::export_fields(dir, files.as_deref(), out, &precision, level) {
            Ok(w) => written.extend(w),
            Err(e) => return fail(e),
        }
    }
    if !only_fields {
        match photonoxide::facade::export_run(dir, tables.as_deref(), to.as_deref()) {
            Ok(w) => written.extend(w),
            Err(e) => return fail(e),
        }
    }
    for path in written {
        println!("{}", path.display());
    }
    ExitCode::SUCCESS
}

fn new(args: &[String]) -> ExitCode {
    let (name, path) = match args {
        [flag] if flag == "--list" => {
            for t in &job::TEMPLATES {
                println!("{:<12} {}", t.name, t.about);
            }
            return ExitCode::SUCCESS;
        }
        [name] => (name, PathBuf::from(format!("new-{name}.yaml"))),
        [name, path] => (name, PathBuf::from(path)),
        _ => return usage(),
    };
    match new_job(name, &path) {
        Ok(()) => {
            println!(
                "wrote {}: photonoxide run {} runs it",
                path.display(),
                path.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => fail(e),
    }
}

/// Writes the template `name` to `path`, in the format its extension names: YAML as the template
/// is, TOML or JSON as the library writes the job. Never over a file that exists.
fn new_job(name: &str, path: &Path) -> Result<(), String> {
    let t = job::template(name)
        .ok_or_else(|| format!("no template {name}: see `photonoxide new --list`"))?;
    let format = photonoxide::run::Format::from_path(path).ok_or_else(|| {
        format!(
            "{}: a job file ends in .yaml, .yml, .toml or .json",
            path.display()
        )
    })?;
    let text = match format {
        photonoxide::run::Format::Yaml => t.text.to_owned(),
        other => Job::parse_as(t.text, photonoxide::run::Format::Yaml)
            .and_then(|j| j.to_text(other))
            .map_err(|e| e.to_string())?,
    };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::AlreadyExists => {
                format!("{} exists: name another file", path.display())
            }
            _ => format!("{}: {e}", path.display()),
        })?;
    std::io::Write::write_all(&mut file, text.as_bytes())
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn example(args: &[String]) -> ExitCode {
    match args {
        [flag] if flag == "--list" => {
            for e in examples::list() {
                println!("{:<24} {}", e.name, e.what);
            }
            ExitCode::SUCCESS
        }
        [name] => {
            examples::run(name).unwrap_or_else(|| fail(format!("no example {name}: see --list")))
        }
        _ => usage(),
    }
}

fn schema(args: &[String]) -> ExitCode {
    // the job file's schema, or with --results the one of a run's results.json
    let (text, args, which) = match args {
        [first, rest @ ..] if first == "--results" => {
            (job::results::schema_text(), rest, "--results ")
        }
        _ => (job::schema_text(), args, ""),
    };
    match args {
        [] => print!("{text}"),
        [flag, file] if flag == "--write" => {
            if let Err(e) = std::fs::write(file, &text) {
                return fail(format!("{file}: {e}"));
            }
        }
        [flag, file] if flag == "--check" => {
            let current = std::fs::read_to_string(file)
                .unwrap_or_default()
                .replace("\r\n", "\n");
            if current != text {
                return fail(format!(
                    "{file} is out of date: run `photonoxide schema {which}--write {file}`"
                ));
            }
        }
        _ => return usage(),
    }
    ExitCode::SUCCESS
}

fn validate(args: &[String]) -> ExitCode {
    let (mode, path) = match args {
        [] => (None, None),
        [flag, file] if flag == "--write" || flag == "--check" => {
            (Some(flag.as_str()), Some(PathBuf::from(file)))
        }
        _ => return usage(),
    };
    let (report, passed) = photonoxide::validation::report();
    print!("{report}");
    match (mode, path) {
        (Some("--write"), Some(path)) => {
            if let Err(e) = std::fs::write(&path, &report) {
                return fail(format!("{}: {e}", path.display()));
            }
        }
        (Some("--check"), Some(path)) => {
            let current = std::fs::read_to_string(&path)
                .unwrap_or_default()
                .replace("\r\n", "\n");
            if current != report {
                return fail(format!(
                    "{} is out of date: run `photonoxide validate --write {}`",
                    path.display(),
                    path.display()
                ));
            }
        }
        _ => {}
    }
    if passed {
        ExitCode::SUCCESS
    } else {
        fail("some validation cases failed")
    }
}

/// The console a bare `photonoxide` was given.
#[cfg(feature = "window")]
mod console {
    /// Lets go of the console when it was made for this program alone (it was started from
    /// Explorer, not from a terminal), so it doesn't sit open beside the window.
    #[cfg(windows)]
    pub fn release() {
        unsafe extern "system" {
            fn GetConsoleProcessList(list: *mut u32, count: u32) -> u32;
            fn FreeConsole() -> i32;
        }
        let mut list = [0u32; 2];
        // SAFETY: both are plain Win32 calls; the list is a valid buffer of the length given
        unsafe {
            if GetConsoleProcessList(list.as_mut_ptr(), 2) == 1 {
                FreeConsole();
            }
        }
    }

    #[cfg(not(windows))]
    pub fn release() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_and_view_take_help_for_help_not_for_a_run_directory() {
        let help = |args: &[&str]| args.iter().map(|&a| a.to_owned()).collect::<Vec<_>>();
        assert!(asks_for_help(&help(&["--help"])));
        assert!(asks_for_help(&help(&["runs/a", "-h"])));
        assert!(!asks_for_help(&help(&["runs/a", "--list"])));
        // the usage (exit 2), as every other command gives it, not "--help has no events.jsonl"
        let usage = format!("{:?}", ExitCode::from(2));
        assert_eq!(format!("{:?}", export(&help(&["--help"]))), usage);
        assert_eq!(format!("{:?}", view(&help(&["--help"]))), usage);
    }

    #[test]
    fn new_writes_a_template_in_the_format_its_file_names_and_never_over_a_file() {
        let dir = std::env::temp_dir().join(format!("photonoxide-new-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for t in &job::TEMPLATES {
            let yaml = dir.join(format!("{}.yaml", t.name));
            new_job(t.name, &yaml).unwrap();
            // the template as it is, with its comments
            assert_eq!(std::fs::read_to_string(&yaml).unwrap(), t.text);
            for ext in ["toml", "json"] {
                let other = dir.join(format!("{}.{ext}", t.name));
                new_job(t.name, &other).unwrap();
                let again = Job::load(&other).unwrap();
                assert_eq!(again.task(), Job::load(&yaml).unwrap().task(), "{other:?}");
                job::check(&again).unwrap();
            }
            let e = new_job(t.name, &yaml).unwrap_err();
            assert!(e.contains("exists"), "{e}");
            assert_eq!(std::fs::read_to_string(&yaml).unwrap(), t.text);
        }
        assert!(
            new_job("nothing", &dir.join("a.yaml"))
                .unwrap_err()
                .contains("--list")
        );
        assert!(
            new_job("modes", &dir.join("a.txt"))
                .unwrap_err()
                .contains(".yaml")
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
