//! The `photonoxide` program: opens the studio; runs a job, live in the studio window or
//! headless, its sweep farmed to workers if asked; serves as a worker; replays a run in the
//! studio; runs the built-in examples; checks the validation report; times the benchmark
//! problems.

mod academy;
mod auto;
mod bench;
mod benchmarks;
mod channels;
mod charts;
mod circuits;
mod diagrams;
mod examples;
mod farm;
mod libraries;
mod materials;
mod runner;
mod settings;
mod studio;
mod tasks;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use photonoxide::job;
use photonoxide::run::{Job, Run, Stop};

const USAGE: &str = "usage:
  photonoxide
      open the studio, to run a job from jobs/ or reopen a run from runs/
  photonoxide run <job.toml|.json|.yaml> [--out <dir>] [--headless] [--linger <seconds>]
      run a job; the studio window shows it live and closes by itself when it's done
      (--headless: no window; --out: where run directories go, default runs/;
       --linger: how long the window stays after the run, default 5 s)
      [--workers <n>] [--worker <host:port>]... [--token <token>] [--task-timeout <seconds>]
      farm the sweep's points: to n workers started here, and to workers listening
      elsewhere (their token: --token or PHOTONOXIDE_FARM_TOKEN); the same record, gathered
      in order; a point past --task-timeout fails, and its worker is dropped
  photonoxide worker [--listen <address>] [--token <token>] [--slots <n>] [--once]
      serve coordinators' sweeps and jobs, one coordinator at a time, on 127.0.0.1:7878
      by default (an address beyond this machine needs a token); --slots: tasks at once,
      default 1; --once: one coordinator, then exit. Not for untrusted networks: the
      traffic isn't encrypted
  photonoxide view <run directory>
      replay a finished run in the studio
  photonoxide example <name>
      run one of the built-in examples, each a published result reproduced
      (`photonoxide example --list` lists them)
  photonoxide validate [--write <file> | --check <file>]
      run every validation case and print the report; --write saves it, --check fails
      unless <file> holds exactly this report
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
  photonoxide --version";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => open(),
        Some("run") => run(&args[1..]),
        Some("view") => view(&args[1..]),
        Some("worker") => farm::worker(&args[1..]),
        Some("validate") => validate(&args[1..]),
        Some("example") => example(&args[1..]),
        Some("bench") => bench::run(&args[1..]),
        Some("libraries") => libraries::run(&args[1..]),
        Some("nightly") => channels::run(&args[1..]),
        Some("--version" | "-V") => {
            let build = channels::this_build();
            match (build.nightly(), build.short) {
                (false, Some(commit)) => println!(
                    "photonoxide {} ({commit}{})",
                    build.label,
                    if build.dirty {
                        ", with changes not committed"
                    } else {
                        ""
                    }
                ),
                _ => println!("photonoxide {}", build.label),
            }
            ExitCode::SUCCESS
        }
        _ => usage(),
    }
}

fn open() -> ExitCode {
    console::release();
    match studio::show(None, None) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(e),
    }
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
            _ if job_path.is_none() && !a.starts_with("--") => job_path = Some(PathBuf::from(a)),
            _ => return usage(),
        }
    }
    let Some(job_path) = job_path else {
        return usage();
    };
    let job = match Job::load(&job_path) {
        Ok(j) => j,
        Err(e) => return fail(e),
    };
    // this machine's benchmark records for auto, and the external libraries if the job needs
    // them
    auto::prepare(&job);
    let mut record = match Run::create(&out, &job) {
        Ok(r) => r,
        Err(e) => return fail(e),
    };
    let dir = record.dir().to_path_buf();
    println!("{}", dir.display());
    let stop = Stop::new(job.timeout());
    // the sweep's points on workers, here or on other machines: the same record, gathered in
    // order as the points complete
    let workers = if job.task().get("sweep").is_none() {
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
            let result = job::execute(&job, &mut record, &stop);
            // the local workers end with the run
            drop(workers);
            let _ = done.send(());
            result
        })
    };
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
    match worker.join() {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(e)) => fail(e),
        Err(_) => fail("the job panicked"),
    }
}

fn view(args: &[String]) -> ExitCode {
    let [dir] = args else {
        return usage();
    };
    let dir = Path::new(dir);
    if !dir.join("events.jsonl").is_file() {
        return fail(format!("{} has no events.jsonl", dir.display()));
    }
    match studio::show(Some(dir), None) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(e),
    }
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
