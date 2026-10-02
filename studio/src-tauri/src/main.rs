//! The `photonoxide` program: runs a job, live in the studio window or headless; replays a run in
//! the studio; checks the validation report.

mod studio;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use photonoxide::job;
use photonoxide::run::{Job, Run, Stop};

const USAGE: &str = "usage:
  photonoxide run <job.toml> [--out <dir>] [--headless] [--linger <seconds>]
      run a job; the studio window shows it live and closes by itself when it's done
      (--headless: no window; --out: where run directories go, default runs/;
       --linger: how long the window stays after the run, default 5 s)
  photonoxide view <run directory>
      replay a finished run in the studio
  photonoxide validate [--write <file> | --check <file>]
      run every validation case and print the report; --write saves it, --check fails
      unless <file> holds exactly this report";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("run") => run(&args[1..]),
        Some("view") => view(&args[1..]),
        Some("validate") => validate(&args[1..]),
        _ => usage(),
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
    let mut it = args.iter();
    while let Some(a) = it.next() {
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
    let mut record = match Run::create(&out, &job) {
        Ok(r) => r,
        Err(e) => return fail(e),
    };
    let dir = record.dir().to_path_buf();
    println!("{}", dir.display());
    let stop = Stop::new(job.timeout());
    let (done, finished) = std::sync::mpsc::channel();
    let worker = {
        let (job, stop) = (job.clone(), stop.clone());
        std::thread::spawn(move || {
            let result = job::execute(&job, &mut record, &stop);
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
        if let Err(e) = studio::show(&dir, Some(live)) {
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
    match studio::show(dir, None) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(e),
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
