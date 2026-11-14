//! `photonoxide run --strict` as a process (#783's A5): with it, a finished run that printed a
//! `warning:` or a `convergence:` line exits 5; without it, 0; the results are written either
//! way.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PROGRAM: &str = env!("CARGO_BIN_EXE_photonoxide");

/// A silicon strip's modes, `window` µm either side across, on `step` nm, with a
/// `[task.convergence]` of `tolerance` if given.
fn strip(window: f64, step: f64, tolerance: Option<f64>) -> String {
    let mut job = format!(
        r#"
name = "strip"
timeout_minutes = 5.0

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
propagation = "x"
y_um = [-{window}, {window}]
step_nm = {step}
modes = 1

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [10.0, 0.5]
"#
    );
    if let Some(t) = tolerance {
        job.push_str(&format!("\n[task.convergence]\ntolerance = {t}\n"));
    }
    job
}

fn folder(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("photonoxide-strict-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs `job` headless into `dir`, `args` before the job's path or after it.
fn run(dir: &Path, job: &str, args: &[&str], flags_first: bool) -> Output {
    let path = dir.join("job.toml");
    std::fs::write(&path, job).unwrap();
    let mut command = Command::new(PROGRAM);
    command.arg("run");
    if flags_first {
        command.args(args);
    }
    command.arg(&path).args(["--headless", "--out"]).arg(dir);
    if !flags_first {
        command.args(args);
    }
    command.output().unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The run's `results.json`: the first line of stdout names its folder.
fn results(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let run = PathBuf::from(stdout.lines().next().unwrap().trim());
    std::fs::read_to_string(run.join("results.json")).unwrap()
}

#[test]
fn a_grid_not_checked_exits_5_under_strict_and_0_without() {
    let dir = folder("one-grid");
    let job = strip(1.2, 40.0, None);
    let plain = run(&dir, &job, &[], false);
    assert_eq!(plain.status.code(), Some(0), "{}", stderr(&plain));
    assert!(
        stderr(&plain).contains("convergence: unchecked"),
        "{}",
        stderr(&plain)
    );
    // the scheme is said, and doesn't count
    assert!(stderr(&plain).contains("scheme: mode::vector solved by standard finite differences"));
    let strict = run(&dir, &job, &["--strict"], false);
    assert_eq!(strict.status.code(), Some(5), "{}", stderr(&strict));
    assert!(
        stderr(&strict)
            .contains("--strict: the run finished with 0 warnings and 1 convergence line"),
        "{}",
        stderr(&strict)
    );
    // the results are written either way
    for output in [&plain, &strict] {
        let text = results(output);
        assert!(text.contains("\"outcome\": \"finished\""), "{text}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_converged_run_exits_0_under_strict_and_a_warning_5() {
    let dir = folder("converged");
    let strict = run(&dir, &strip(1.2, 20.0, Some(0.05)), &["--strict"], false);
    assert_eq!(strict.status.code(), Some(0), "{}", stderr(&strict));
    assert!(
        !stderr(&strict).contains("convergence:"),
        "{}",
        stderr(&strict)
    );
    assert!(!stderr(&strict).contains("warning:"), "{}", stderr(&strict));
    assert!(results(&strict).contains("\"verdict\": \"converged\""));
    // the skeptic's 0.4 um window on the 500 nm strip: a warning, exit 0 without the flag
    let narrow = strip(0.2, 20.0, Some(0.05));
    let plain = run(&dir, &narrow, &[], false);
    assert_eq!(plain.status.code(), Some(0), "{}", stderr(&plain));
    assert!(
        stderr(&plain).contains("warning: mode 1 of 1"),
        "{}",
        stderr(&plain)
    );
    let strict = run(&dir, &narrow, &["--strict"], false);
    assert_eq!(strict.status.code(), Some(5), "{}", stderr(&strict));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn strict_is_accepted_before_and_after_the_job_path() {
    let dir = folder("position");
    let job = strip(1.2, 40.0, None);
    let before = run(&dir, &job, &["--strict"], true);
    assert_eq!(before.status.code(), Some(5), "{}", stderr(&before));
    let after = run(&dir, &job, &["--strict"], false);
    assert_eq!(after.status.code(), Some(5), "{}", stderr(&after));
    let _ = std::fs::remove_dir_all(&dir);
}
