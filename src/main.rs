//! The `photonoxide` command.

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage:
  photonoxide validate                 run every validation case, print the report
  photonoxide validate --write <file>  ... and write it to <file>
  photonoxide validate --check <file>  ... and fail unless <file> holds exactly this report";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("validate") => validate(&args[1..]),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn validate(args: &[String]) -> ExitCode {
    let (mode, path) = match args {
        [] => (None, None),
        [flag, file] if flag == "--write" || flag == "--check" => {
            (Some(flag.as_str()), Some(PathBuf::from(file)))
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let (report, passed) = photonoxide::validation::report();
    print!("{report}");
    match (mode, path) {
        (Some("--write"), Some(path)) => {
            if let Err(e) = std::fs::write(&path, &report) {
                eprintln!("{}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        }
        (Some("--check"), Some(path)) => {
            let current = std::fs::read_to_string(&path)
                .unwrap_or_default()
                .replace("\r\n", "\n");
            if current != report {
                eprintln!(
                    "{} is out of date: run `photonoxide validate --write {}`",
                    path.display(),
                    path.display()
                );
                return ExitCode::FAILURE;
            }
        }
        _ => {}
    }
    if passed {
        ExitCode::SUCCESS
    } else {
        eprintln!("some validation cases failed");
        ExitCode::FAILURE
    }
}
