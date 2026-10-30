//! The suggestions by what this machine's own benchmark records measured (#564): each suggested
//! library's backends against photonoxide's own, on the problems `photonoxide bench` ran here.
//!
//! - **Only what was measured.** Every figure is a stored record's: its problem, its threads,
//!   its seconds and photonoxide's on the same problem, and the day it ran. A library with no
//!   record says so, and keeps its place by the hardware.
//! - **The latest run.** Each (problem, backend, threads) takes its latest counted run (run, and
//!   accurate), so a library updated since and measured again speaks for itself.
//! - **The largest problem.** Of the problems measured with both, the one with the most
//!   unknowns speaks for the library (the most threads next, then its fastest backend there),
//!   with how many of the others it was faster on.
//! - **Slower is said.** A library slower than photonoxide's own here is shown as slower, and
//!   ranked after those not measured yet; never hidden.
//! - **Stale.** A record from another version of photonoxide says so: its own solvers may have
//!   changed since.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::Serialize;

use super::hardware::Suggestion;
use crate::runner::Record;

/// What the records measured of a suggested library here: its backend against photonoxide's own
/// on one problem, from two stored records.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Measured {
    /// The backend measured, by its name (`mumps`, `pardiso`).
    pub backend: String,
    /// The problem: the catalogue's id, its family, grid and unknowns.
    pub id: String,
    pub family: String,
    pub grid: String,
    pub unknowns: usize,
    pub threads: usize,
    /// The backend's seconds, and photonoxide's own on the same problem and threads.
    pub seconds: f64,
    pub own_seconds: f64,
    /// photonoxide's time over the backend's: above 1, the backend was faster.
    pub speedup: f64,
    /// The day the backend's record ran, as `2026-10-30`, and photonoxide's own record's.
    pub date: String,
    pub own_date: String,
    /// The photonoxide version that recorded it.
    pub version: String,
    /// The problems measured with this backend and photonoxide's own at these threads, and how
    /// many of them the backend was faster on.
    pub problems: usize,
    pub faster_on: usize,
    /// Why the record may no longer hold, if it may not.
    pub stale: Option<String>,
    /// All of it, in a sentence or two, for the window.
    pub said: String,
}

/// The latest counted run of each (id, backend, threads).
fn latest(records: &[Record]) -> BTreeMap<(&str, &str, usize), &Record> {
    let mut out: BTreeMap<(&str, &str, usize), &Record> = BTreeMap::new();
    for r in records.iter().filter(|r| r.counts() && r.seconds > 0.0) {
        let key = (r.id.as_str(), r.backend.as_str(), r.threads);
        if out
            .get(&key)
            .is_none_or(|f| r.unix_seconds >= f.unix_seconds)
        {
            out.insert(key, r);
        }
    }
    out
}

/// A problem's family, in words.
fn family(name: &str) -> String {
    match name {
        "fdfd2d" => "the 2D FDFD problem".into(),
        "fdfd3d" => "the 3D FDFD problem".into(),
        "fdfd3d-iterative" => "the iterative 3D FDFD problem".into(),
        "modes" => "the mode solve".into(),
        "circuit" => "the circuit".into(),
        "dense" => "the dense problem".into(),
        "job" => "the job".into(),
        "fdtd" => "the FDTD run".into(),
        other => format!("the {other} problem"),
    }
}

/// Seconds to three figures.
fn time(s: f64) -> String {
    if s >= 100.0 {
        format!("{s:.0} s")
    } else if s >= 10.0 {
        format!("{s:.1} s")
    } else if s >= 1.0 {
        format!("{s:.2} s")
    } else if s >= 0.1 {
        format!("{:.0} ms", s * 1e3)
    } else if s >= 0.01 {
        format!("{:.1} ms", s * 1e3)
    } else {
        format!("{:.2} ms", s * 1e3)
    }
}

fn day(unix_seconds: u64) -> String {
    crate::watch::date(i64::try_from(unix_seconds / 86_400).unwrap_or(0))
}

/// The sentence for `m`, its figures as the records hold them.
fn say(m: &Measured, this_version: &str) -> String {
    let threads = if m.threads == 1 {
        "1 thread".to_string()
    } else {
        format!("{} threads", m.threads)
    };
    let problem = format!(
        "{} {} ({}, {} unknowns)",
        family(&m.family),
        m.id,
        m.grid,
        crate::bench::grouped(m.unknowns)
    );
    let times = format!("{} against {}", time(m.seconds), time(m.own_seconds));
    let when = if m.own_date == m.date {
        format!("in the benchmark of {}", m.date)
    } else {
        format!(
            "in the benchmark of {} (photonoxide's own on {})",
            m.date, m.own_date
        )
    };
    let faster = format!("{:.1}", m.speedup);
    let slower = format!("{:.1}", 1.0 / m.speedup);
    let mut out = if m.speedup >= 1.0 && faster != "1.0" {
        format!(
            "{} solved {problem} {faster}× faster than photonoxide's own on {threads}: {times}, {when}.",
            m.backend
        )
    } else if m.speedup < 1.0 && slower != "1.0" {
        format!(
            "{} was {slower}× slower than photonoxide's own on {problem}, on {threads}: {times}, {when}.",
            m.backend
        )
    } else {
        format!(
            "{} was as fast as photonoxide's own, to a tenth, on {problem}, on {threads}: {times}, {when}.",
            m.backend
        )
    };
    if m.problems > 1 {
        out.push_str(&format!(
            " Faster on {} of the {} problems measured with both on {threads}.",
            m.faster_on, m.problems
        ));
    }
    if m.version != this_version {
        out.push_str(&format!(
            " Measured with photonoxide {}; this is {this_version}, so measure it again on the Benchmarks page.",
            m.version
        ));
    }
    out
}

/// What `records` (this machine's) measured of the backends `backends`, against photonoxide's
/// own; none when no problem was measured with both. `this_version` is the running
/// photonoxide's: a record from another is stale.
pub fn measure(backends: &[&str], records: &[Record], this_version: &str) -> Option<Measured> {
    let latest = latest(records);
    // (backend record, photonoxide's own record) on each problem and threads
    let pairs: Vec<(&Record, &Record)> = latest
        .iter()
        .filter(|((_, backend, _), _)| backends.contains(backend))
        .filter_map(|(&(id, _, threads), r)| {
            latest
                .get(&(id, "photonoxide", threads))
                .map(|own| (*r, *own))
        })
        .collect();
    let speedup = |(r, own): &(&Record, &Record)| own.seconds / r.seconds;
    let (r, own) = *pairs.iter().max_by(|a, b| {
        (a.0.unknowns, a.0.threads)
            .cmp(&(b.0.unknowns, b.0.threads))
            .then(speedup(a).total_cmp(&speedup(b)))
            .then(a.0.unix_seconds.cmp(&b.0.unix_seconds))
    })?;
    let same: Vec<&(&Record, &Record)> = pairs
        .iter()
        .filter(|p| p.0.backend == r.backend && p.0.threads == r.threads)
        .collect();
    let mut m = Measured {
        backend: r.backend.clone(),
        id: r.id.clone(),
        family: r.family.clone(),
        grid: r.grid.clone(),
        unknowns: r.unknowns,
        threads: r.threads,
        seconds: r.seconds,
        own_seconds: own.seconds,
        speedup: own.seconds / r.seconds,
        date: day(r.unix_seconds),
        own_date: day(own.unix_seconds),
        version: r.version.clone(),
        problems: same.len(),
        faster_on: same.iter().filter(|p| speedup(p) > 1.0).count(),
        stale: (r.version != this_version).then(|| {
            format!(
                "measured with photonoxide {}, before this version ({this_version})",
                r.version
            )
        }),
        said: String::new(),
    };
    m.said = say(&m, this_version);
    Some(m)
}

/// Where a suggestion goes: measured faster first, the fastest first; then those not measured,
/// in the hardware's order; then those measured slower than photonoxide's own.
fn rank(s: &Suggestion) -> (u8, f64) {
    match &s.measured {
        Some(m) if m.speedup >= 1.0 => (0, -m.speedup),
        None => (1, 0.0),
        Some(m) => (2, -m.speedup),
    }
}

/// The hardware's suggestions with what `records` (this machine's) measured of each, ranked by
/// it; and the libraries the hardware didn't suggest that the records measured faster than
/// photonoxide's own here.
pub fn by_records(
    mut suggestions: Vec<Suggestion>,
    records: &[Record],
    this_version: &str,
) -> Vec<Suggestion> {
    for s in &mut suggestions {
        s.measured = measure(s.backends, records, this_version);
    }
    for guide in super::GUIDES
        .iter()
        .filter(|g| !g.built_in && !g.backends.is_empty())
    {
        if suggestions.iter().any(|s| s.library == guide.library) {
            continue;
        }
        if let Some(m) = measure(guide.backends, records, this_version)
            .filter(|m| m.speedup >= 1.0 && m.faster_on > 0)
        {
            suggestions.push(Suggestion {
                library: guide.library,
                why: guide.about,
                by: "records",
                backends: guide.backends,
                install: None,
                command: None,
                download: guide.download,
                system: false,
                measured: Some(m),
            });
        }
    }
    // a stable sort: the hardware's order stays within each group
    suggestions.sort_by(|a, b| {
        let (ga, ka) = rank(a);
        let (gb, kb) = rank(b);
        ga.cmp(&gb)
            .then(ka.partial_cmp(&kb).unwrap_or(Ordering::Equal))
    });
    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::libraries::hardware::{Cpu, Hardware, suggest};
    use crate::libraries::topology::Topology;
    use crate::runner::Machine;

    const NOW: &str = "0.5.4";
    /// 2026-10-30, 12:00 UTC.
    const DAY: u64 = 1_793_361_600;

    fn record(id: &str, n: usize, backend: &str, threads: usize, seconds: f64) -> Record {
        serde_json::from_value(serde_json::json!({
            "id": id, "family": "fdfd2d", "size": n, "grid": format!("{n} × {n} cells"),
            "unknowns": n * n, "backend": backend, "backend_version": "1",
            "deterministic": true, "threads": threads, "phases": [], "seconds": seconds,
            "peak_bytes": null, "error": 1e-12, "tolerance": 1e-9, "accurate": true,
            "failure": null, "load": null,
            "machine": Machine { cpu: "here".into(), logical_processors: 8,
                                 memory_bytes: None, os: "windows".into() },
            "version": NOW, "unix_seconds": DAY
        }))
        .unwrap()
    }

    fn intel() -> Vec<Suggestion> {
        suggest(&Hardware {
            os: "windows",
            arch: "x86_64",
            cpu: Cpu {
                vendor: "GenuineIntel".into(),
                brand: String::new(),
                threads: 8,
                features: vec![],
                topology: Topology::default(),
            },
            gpu: None,
        })
    }

    fn names(s: &[Suggestion]) -> Vec<&str> {
        s.iter().map(|s| s.library).collect()
    }

    #[test]
    fn a_faster_library_is_said_with_its_record_and_ranked_first() {
        let records = vec![
            record("fdfd2d-0200", 200, "photonoxide", 8, 0.4),
            record("fdfd2d-0200", 200, "mumps", 8, 0.5),
            record("fdfd2d-0400", 400, "photonoxide", 8, 3.1),
            record("fdfd2d-0400", 400, "mumps", 8, 1.0),
        ];
        let s = by_records(intel(), &records, NOW);
        // by the hardware: oneMKL, then MUMPS; MUMPS measured faster, oneMKL not measured
        assert_eq!(names(&s), ["MUMPS", "oneMKL"]);
        let m = s[0].measured.as_ref().unwrap();
        // the largest problem speaks for it, with how many it was faster on
        assert_eq!(m.id, "fdfd2d-0400");
        assert!((m.speedup - 3.1).abs() < 1e-12);
        assert_eq!((m.problems, m.faster_on), (2, 1));
        assert_eq!(m.date, "2026-10-30");
        assert!(m.stale.is_none());
        assert_eq!(
            m.said,
            "mumps solved the 2D FDFD problem fdfd2d-0400 (400 × 400 cells, 160 000 unknowns) \
             3.1× faster than photonoxide's own on 8 threads: 1.00 s against 3.10 s, in the \
             benchmark of 2026-10-30. Faster on 1 of the 2 problems measured with both on 8 \
             threads."
        );
        assert!(s[1].measured.is_none());
    }

    #[test]
    fn a_slower_library_is_said_slower_and_ranked_after_those_not_measured() {
        let records = vec![
            record("fdfd2d-0400", 400, "photonoxide", 8, 1.0),
            record("fdfd2d-0400", 400, "pardiso", 8, 2.5),
        ];
        let s = by_records(intel(), &records, NOW);
        assert_eq!(names(&s), ["MUMPS", "oneMKL"]);
        let m = s[1].measured.as_ref().unwrap();
        assert!((m.speedup - 0.4).abs() < 1e-12);
        assert_eq!(m.faster_on, 0);
        assert!(
            m.said
                .starts_with("pardiso was 2.5× slower than photonoxide's own")
        );
        assert!(m.said.contains("2.50 s against 1.00 s"));
    }

    #[test]
    fn no_record_keeps_the_hardware_s_order_and_says_nothing_measured() {
        let s = by_records(intel(), &[], NOW);
        assert_eq!(names(&s), ["oneMKL", "MUMPS"]);
        assert!(s.iter().all(|s| s.measured.is_none() && s.by == "hardware"));
        // a backend's record without photonoxide's own on the same problem measures nothing,
        // nor one that failed, nor one that missed its check
        let mut failed = record("fdfd2d-0400", 400, "mumps", 8, 0.1);
        failed.failure = Some("timed out".into());
        let mut wrong = record("fdfd2d-0400", 400, "mumps", 8, 0.1);
        wrong.accurate = false;
        let records = vec![
            record("fdfd2d-0200", 200, "mumps", 8, 0.1),
            record("fdfd2d-0400", 400, "photonoxide", 8, 1.0),
            failed,
            wrong,
        ];
        assert!(
            by_records(intel(), &records, NOW)
                .iter()
                .all(|s| s.measured.is_none())
        );
        // another thread count isn't the same run
        let records = vec![
            record("fdfd2d-0400", 400, "photonoxide", 1, 1.0),
            record("fdfd2d-0400", 400, "mumps", 8, 0.1),
        ];
        assert!(measure(&["mumps"], &records, NOW).is_none());
    }

    #[test]
    fn a_record_of_another_version_is_stale_and_the_latest_run_counts() {
        let mut old_own = record("fdfd2d-0400", 400, "photonoxide", 8, 4.0);
        old_own.version = "0.5.3".into();
        old_own.unix_seconds = DAY - 86_400 * 10;
        let mut old = record("fdfd2d-0400", 400, "mumps", 8, 1.0);
        old.version = "0.5.3".into();
        old.unix_seconds = DAY - 86_400 * 10;
        let m = measure(&["mumps"], &[old_own.clone(), old.clone()], NOW).unwrap();
        assert_eq!(m.date, "2026-10-20");
        assert!(m.stale.as_deref().unwrap().contains("0.5.3"));
        assert!(m.said.ends_with(
            "Measured with photonoxide 0.5.3; this is 0.5.4, so measure it again on the \
             Benchmarks page."
        ));
        // measured again since: the latest run speaks, slower this time, and isn't stale
        let records = vec![
            old_own,
            old,
            record("fdfd2d-0400", 400, "photonoxide", 8, 1.0),
            record("fdfd2d-0400", 400, "mumps", 8, 2.0),
        ];
        let m = measure(&["mumps"], &records, NOW).unwrap();
        assert!((m.speedup - 0.5).abs() < 1e-12);
        assert!(m.stale.is_none());
    }

    #[test]
    fn a_library_the_hardware_didn_t_suggest_joins_when_measured_faster() {
        let records = vec![
            record("fdfd2d-0400", 400, "photonoxide", 8, 2.0),
            record("fdfd2d-0400", 400, "superlu", 8, 1.0),
            record("fdfd2d-0400", 400, "cusparse", 8, 4.0),
        ];
        let s = by_records(intel(), &records, NOW);
        // SuperLU joins, first, by its record; cuSPARSE, slower, doesn't join uninvited
        assert_eq!(names(&s), ["SuperLU", "oneMKL", "MUMPS"]);
        assert_eq!(s[0].by, "records");
        assert!(s[0].measured.as_ref().unwrap().said.contains("2.0× faster"));
    }

    #[test]
    fn as_fast_to_a_tenth_is_said_so() {
        let records = vec![
            record("fdfd2d-0400", 400, "photonoxide", 8, 1.0),
            record("fdfd2d-0400", 400, "mumps", 8, 1.02),
        ];
        let m = measure(&["mumps"], &records, NOW).unwrap();
        assert!(
            m.said
                .starts_with("mumps was as fast as photonoxide's own, to a tenth")
        );
    }
}
