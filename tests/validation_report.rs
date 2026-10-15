//! Every validation case passes, and docs/validation.md is exactly the report the code writes.
//! Slow in a debug build: CI runs it in release (`cargo test --release --test validation_report
//! -- --ignored`). After a change, rewrite the report with `photonoxide validate --write
//! docs/validation.md`. With `PHOTONOXIDE_VALIDATION_OUT` set, it writes the report it made
//! there too, so that a run on another system can be compared (the Systems workflow).
//!
//! With `PHOTONOXIDE_VALIDATION_CASES` set to id prefixes separated by spaces (`circuit/
//! components/`), only the cases whose id starts with one of them run, and only their rows are
//! compared with the committed report: a pull request's CI runs the cases of the areas it
//! changed (`.github/areas.sh`). A prefix after a `-` leaves its cases out instead (`-fdtd/
//! -fdfd3d/`: every case but those). The rows are independent, so a row that is right on its
//! own is right in the whole report. The report's head and its list of cases, in order, are
//! checked whole every time, which runs nothing; so the runs of a set of prefixes that between
//! them cover every case check the whole report, as the release pull request's and the weekly
//! run's shards do.
//!
//! With `--nocapture`, each case's time is printed as it finishes.

use std::time::Instant;

use photonoxide::validation::{Case, cases, header, row};

const REPORT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/validation.md");

#[test]
#[ignore = "runs every validation case: slow, run in release by the Validation report job"]
fn the_validation_report_passes_and_is_up_to_date() {
    let committed = std::fs::read_to_string(REPORT)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    let prefixes = std::env::var("PHOTONOXIDE_VALIDATION_CASES").unwrap_or_default();
    let prefixes: Vec<&str> = prefixes.split_whitespace().collect();
    if prefixes.is_empty() {
        every_case(&committed);
    } else {
        some_cases(&committed, &prefixes);
    }
}

/// Runs a case, printing its time, and returns its row and whether it passed.
fn timed(case: &Case) -> (String, bool) {
    let start = Instant::now();
    let (row, passed) = row(case);
    eprintln!("{:8.1} s  {}", start.elapsed().as_secs_f64(), case.id);
    (row, passed)
}

/// The whole report, made and compared whole.
fn every_case(committed: &str) {
    let mut report = header();
    let mut passed = true;
    for case in cases() {
        let (row, ok) = timed(&case);
        passed &= ok;
        report.push_str(&row);
    }
    if let Some(out) = std::env::var_os("PHOTONOXIDE_VALIDATION_OUT") {
        std::fs::write(out, &report).expect("the report is written");
    }
    assert!(passed, "some validation cases failed:\n{report}");
    assert!(
        committed == report,
        "docs/validation.md is out of date: run `photonoxide validate --write docs/validation.md`"
    );
}

/// The id of a report's row: the text between its first two backticks.
fn id_of(line: &str) -> Option<&str> {
    line.strip_prefix("| `")?.split('`').next()
}

/// The cases `prefixes` select, each row against the committed one; and the committed report's
/// head and ids, in order, against the code's.
fn some_cases(committed: &str, prefixes: &[&str]) {
    let (exclude, include): (Vec<&str>, Vec<&str>) =
        prefixes.iter().partition(|p| p.starts_with('-'));
    let exclude: Vec<&str> = exclude.iter().map(|p| &p[1..]).collect();
    let selected = |id: &str| {
        (include.is_empty() || include.iter().any(|p| id.starts_with(p)))
            && !exclude.iter().any(|p| id.starts_with(p))
    };
    let start = committed.find("\n| `").map_or(committed.len(), |i| i + 1);
    let (head, rows) = committed.split_at(start);
    let committed_rows: Vec<(&str, &str)> = rows
        .lines()
        .map(|line| (id_of(line).unwrap_or(line), line))
        .collect();
    let all = cases();
    let stale =
        "docs/validation.md is out of date: run `photonoxide validate --write docs/validation.md`";
    assert!(head == header(), "{stale} (its head)");
    let whole: String = committed_rows
        .iter()
        .map(|(_, l)| format!("{l}\n"))
        .collect();
    assert!(rows == whole, "{stale} (a row isn't a line of its own)");
    let ids: Vec<&str> = all.iter().map(|c| c.id).collect();
    let committed_ids: Vec<&str> = committed_rows.iter().map(|(id, _)| *id).collect();
    assert!(
        ids == committed_ids,
        "{stale} (its cases: {} in the code, {} in the report, or in another order)",
        ids.len(),
        committed_ids.len()
    );

    let mut failed = Vec::new();
    let mut out_of_date = Vec::new();
    let mut ran = 0;
    for (case, (_, line)) in all.iter().zip(&committed_rows) {
        if !selected(case.id) {
            continue;
        }
        ran += 1;
        let (row, passed) = timed(case);
        if !passed {
            failed.push(row.clone());
        }
        if row.trim_end() != *line {
            out_of_date.push(format!("committed: {line}\nnow:       {row}"));
        }
    }
    eprintln!("{ran} of {} cases, those {prefixes:?} select", all.len());
    assert!(
        failed.is_empty(),
        "some validation cases failed:\n{}",
        failed.concat()
    );
    assert!(
        out_of_date.is_empty(),
        "{stale}:\n{}",
        out_of_date.join("\n")
    );
}
