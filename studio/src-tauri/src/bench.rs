//! `photonoxide bench`: the library's fixed problems (photonoxide::bench) and the slowest
//! examples, each timed in a process of its own on a stated number of threads, beside the
//! machine's memory bandwidth, written up as a Markdown report.
//!
//! A process per problem makes its peak memory its own (the peak resident set only grows) and
//! sets its threads: `RAYON_NUM_THREADS`, which faer follows too.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use photonoxide::bench::{Measurement, Phase, problems};
use serde::{Deserialize, Serialize};

use crate::examples;

/// The examples timed beside the problems: the mode solvers', and a circuit's, as the
/// performance plan measured them; `true` for the heavy ones.
const EXAMPLES: [(&str, bool); 6] = [
    ("strip_waveguide", false),
    ("hadley_corners", false),
    ("group_index", false),
    ("circuit_fit", false),
    ("leaky_wire_benchmark", true),
    ("mzi_dwivedi", true),
];

/// The triad's arrays: 2²⁴ doubles, 128 MiB each, several times any CPU's last-level cache.
const TRIAD_LEN: usize = 1 << 24;

/// One thing to time.
struct Entry {
    id: String,
    title: String,
    heavy: bool,
}

fn entries() -> Vec<Entry> {
    let mut all: Vec<Entry> = problems()
        .into_iter()
        .map(|p| Entry {
            id: p.id.into(),
            title: p.title.into(),
            heavy: p.heavy,
        })
        .collect();
    let list = examples::list();
    for (name, heavy) in EXAMPLES {
        let what = list
            .iter()
            .find(|e| e.name == name)
            .map_or(String::new(), |e| e.what.clone());
        all.push(Entry {
            id: format!("example/{name}"),
            title: format!("{what} (examples/{name}.rs)"),
            heavy,
        });
    }
    all
}

/// What a child process reports.
#[derive(Serialize, Deserialize)]
struct Report {
    measurement: Measurement,
    /// The process's peak resident set, in bytes.
    peak_bytes: Option<u64>,
}

/// One timed run, as the parent collects it.
struct Timed {
    id: String,
    threads: usize,
    outcome: Result<Report, String>,
}

pub fn run(args: &[String]) -> ExitCode {
    if let [flag, id, out] = args
        && flag == "--child"
    {
        return child(id, Path::new(out));
    }
    let mut all = false;
    let mut counts = Vec::new();
    let mut write = None;
    let mut json = None;
    let mut export = None;
    let mut wanted = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--list" => {
                for e in entries() {
                    let heavy = if e.heavy { " (heavy)" } else { "" };
                    println!("{:<28} {}{heavy}", e.id, e.title);
                }
                return ExitCode::SUCCESS;
            }
            "--all" => all = true,
            "--threads" => {
                let parsed: Option<Vec<usize>> = it.next().and_then(|s| {
                    s.split(',')
                        .map(|n| n.trim().parse().ok().filter(|&n| n > 0))
                        .collect()
                });
                match parsed {
                    Some(t) => counts = t,
                    None => return crate::usage(),
                }
            }
            "--write" => match it.next() {
                Some(f) => write = Some(PathBuf::from(f)),
                None => return crate::usage(),
            },
            "--json" => match it.next() {
                Some(f) => json = Some(PathBuf::from(f)),
                None => return crate::usage(),
            },
            "--export" => match it.next() {
                Some(d) => export = Some(PathBuf::from(d)),
                None => return crate::usage(),
            },
            _ if !a.starts_with("--") => wanted.push(a.clone()),
            _ => return crate::usage(),
        }
    }
    if let Some(dir) = export {
        return export_systems(&dir, &wanted);
    }
    if counts.is_empty() {
        counts.push(std::thread::available_parallelism().map_or(1, |n| n.get()));
    }
    let chosen: Vec<Entry> = entries()
        .into_iter()
        .filter(|e| {
            if wanted.is_empty() {
                all || !e.heavy
            } else {
                wanted.iter().any(|w| e.id.starts_with(w.as_str()))
            }
        })
        .collect();
    if chosen.is_empty() {
        return crate::fail("no benchmark problem matches: see `photonoxide bench --list`");
    }

    let mut bandwidth = Vec::new();
    let mut runs = Vec::new();
    for &t in &counts {
        match spawn("triad", t) {
            Ok(r) => {
                let gb_s = r.measurement.phases.first().map_or(f64::NAN, |p| p.seconds);
                eprintln!("triad on {}: {gb_s:.1} GB/s", threads(t));
                bandwidth.push((t, gb_s));
            }
            Err(e) => eprintln!("triad on {}: {e}", threads(t)),
        }
        for e in &chosen {
            eprint!("{} on {}: ", e.id, threads(t));
            let outcome = spawn(&e.id, t);
            match &outcome {
                Ok(r) => eprintln!("{:.2} s", r.measurement.seconds()),
                Err(err) => eprintln!("failed: {err}"),
            }
            runs.push(Timed {
                id: e.id.clone(),
                threads: t,
                outcome,
            });
        }
    }

    let report = markdown(&chosen, &bandwidth, &runs);
    print!("{report}");
    if let Some(path) = write
        && let Err(e) = std::fs::write(&path, &report)
    {
        return crate::fail(format!("{}: {e}", path.display()));
    }
    if let Some(path) = json {
        let rows: Vec<serde_json::Value> = runs
            .iter()
            .map(|r| match &r.outcome {
                Ok(report) => serde_json::json!({
                    "id": r.id, "threads": r.threads,
                    "measurement": report.measurement, "peak_bytes": report.peak_bytes,
                }),
                Err(e) => serde_json::json!({ "id": r.id, "threads": r.threads, "error": e }),
            })
            .collect();
        let doc = serde_json::json!({
            "version": photonoxide::VERSION,
            "machine": machine(),
            "triad_gb_s": bandwidth,
            "runs": rows,
        });
        let text = serde_json::to_string_pretty(&doc).unwrap_or_default();
        if let Err(e) = std::fs::write(&path, text + "\n") {
            return crate::fail(format!("{}: {e}", path.display()));
        }
    }
    if runs.iter().all(|r| r.outcome.is_ok()) {
        ExitCode::SUCCESS
    } else {
        crate::fail("some benchmark problems failed")
    }
}

/// Writes the direct solvers' systems (`photonoxide::bench::export`) into `dir`, each with its
/// solution, for other solvers to factorize: those whose id starts with one of `wanted`, or all.
fn export_systems(dir: &Path, wanted: &[String]) -> ExitCode {
    use photonoxide::bench::export;
    for id in export::IDS {
        if !wanted.is_empty() && !wanted.iter().any(|w| id.starts_with(w.as_str())) {
            continue;
        }
        eprint!("{id}: ");
        let t = std::time::Instant::now();
        let written = export::system(id).and_then(|system| {
            let x = export::solve(&system)?;
            export::write(dir, &system, &x)?;
            Ok(system)
        });
        match written {
            Ok(system) => eprintln!(
                "{} unknowns, {} nonzeros, in {:.1} s",
                grouped(system.n),
                grouped(system.entries.len()),
                t.elapsed().as_secs_f64()
            ),
            Err(e) => return crate::fail(e),
        }
    }
    ExitCode::SUCCESS
}

/// Runs `id` in a child process on `threads` threads, and reads what it reports.
fn spawn(id: &str, threads: usize) -> Result<Report, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let out = std::env::temp_dir().join(format!(
        "photonoxide-bench-{}-{}.json",
        std::process::id(),
        id.replace('/', "-")
    ));
    let _ = std::fs::remove_file(&out);
    let result = Command::new(exe)
        .args(["bench", "--child", id])
        .arg(&out)
        .env("RAYON_NUM_THREADS", threads.to_string())
        // an example prints its checks; the report has the time
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| e.to_string())?;
    let read = std::fs::read_to_string(&out);
    let _ = std::fs::remove_file(&out);
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let last = stderr.lines().rev().find(|l| !l.trim().is_empty());
        return Err(format!(
            "{} ({})",
            last.unwrap_or("no message"),
            result.status
        ));
    }
    let text = read.map_err(|e| format!("no report: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("unreadable report: {e}"))
}

/// The child: times `id` and writes a [`Report`] to `out`.
fn child(id: &str, out: &Path) -> ExitCode {
    // a problem reads its peak when its timed phases end, before a reference solve that may take more
    let mut peak = None;
    let (measurement, code) = if id == "triad" {
        // the bandwidth in GB/s, carried in the one phase's seconds
        let gb_s = photonoxide::bench::triad(TRIAD_LEN, 10);
        (single("triad", gb_s, String::new()), ExitCode::SUCCESS)
    } else if let Some(name) = id.strip_prefix("example/") {
        let t = std::time::Instant::now();
        let Some(code) = examples::run(name) else {
            return crate::fail(format!("no example {name}"));
        };
        let seconds = t.elapsed().as_secs_f64();
        (
            single("the example", seconds, format!("examples/{name}.rs")),
            code,
        )
    } else {
        let Some(problem) = problems().into_iter().find(|p| p.id == id) else {
            return crate::fail(format!("no benchmark problem {id}"));
        };
        match problem.run_with(&mut || peak = memory::peak_bytes()) {
            Ok(m) => (m, ExitCode::SUCCESS),
            Err(e) => return crate::fail(e),
        }
    };
    let report = Report {
        measurement,
        peak_bytes: peak.or_else(memory::peak_bytes),
    };
    let text = serde_json::to_string(&report).unwrap_or_default();
    if let Err(e) = std::fs::write(out, text) {
        return crate::fail(format!("{}: {e}", out.display()));
    }
    code
}

fn single(name: &str, seconds: f64, grid: String) -> Measurement {
    Measurement {
        grid,
        unknowns: 0,
        phases: vec![Phase {
            name: name.into(),
            seconds,
            iterations: None,
            bytes: None,
        }],
        accuracy: None,
    }
}

fn markdown(chosen: &[Entry], bandwidth: &[(usize, f64)], runs: &[Timed]) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# Benchmarks\n");
    let _ = writeln!(
        s,
        "What `photonoxide bench` measured on one machine: the fixed problems of \
         `photonoxide::bench` and the slowest examples, each in a process of its own. Times depend \
         on the machine and on what else ran on it; the errors don't. Peak memory is the process's \
         peak resident set (its peak working set on Windows); ns per unknown per iteration is the \
         QMR phases' time over the unknowns and the iterations. Bandwidth is the iterating phases' \
         bytes over their time, the bytes counted by the kernels themselves (each nonzero's value \
         and index, each row pointer, each vector read or written once: src/traffic.rs), from \
         memory or from cache; beside it, its share of the triad on the same threads, the roof \
         (Williams et al., Commun. ACM 52(4), 65 (2009)). A problem whose vectors fit in the \
         last-level cache can count above it: the 40³ guide's do. See the performance \
         plan, docs/plans/performance.md.\n"
    );
    let _ = writeln!(s, "- photonoxide {}, {}", photonoxide::VERSION, machine());
    if !bandwidth.is_empty() {
        let triads: Vec<String> = bandwidth
            .iter()
            .map(|(t, gb_s)| format!("{gb_s:.1} GB/s on {}", threads(*t)))
            .collect();
        let _ = writeln!(
            s,
            "- memory bandwidth, the STREAM triad over three arrays of 128 MiB: {}",
            triads.join(", ")
        );
    }
    let _ = writeln!(
        s,
        "\n| Problem | Grid | Unknowns | Threads | Time | Iterations | ns/unknown/iteration | Bandwidth (of the triad) | Error | Peak memory |"
    );
    let _ = writeln!(s, "|---|---|---:|---:|---:|---:|---:|---:|---|---:|");
    for r in runs {
        match &r.outcome {
            Ok(report) => {
                let m = &report.measurement;
                let dash = || "—".to_string();
                let roof = bandwidth
                    .iter()
                    .find(|(t, _)| *t == r.threads)
                    .map(|&(_, gb_s)| gb_s);
                let reached = m
                    .gigabytes_per_second()
                    .map_or_else(dash, |gb_s| match roof {
                        Some(roof) => format!("{gb_s:.1} GB/s ({:.0}%)", 100.0 * gb_s / roof),
                        None => format!("{gb_s:.1} GB/s"),
                    });
                let _ = writeln!(
                    s,
                    "| `{}` | {} | {} | {} | {:.2} s | {} | {} | {reached} | {} | {} |",
                    r.id,
                    m.grid,
                    if m.unknowns > 0 {
                        grouped(m.unknowns)
                    } else {
                        dash()
                    },
                    r.threads,
                    m.seconds(),
                    m.iterations().map_or_else(dash, grouped),
                    m.nanoseconds_per_unknown_iteration()
                        .map_or_else(dash, |ns| format!("{ns:.1}")),
                    m.accuracy
                        .as_ref()
                        .map_or_else(dash, |a| format!("{:.1e} ({})", a.error, a.against)),
                    report.peak_bytes.map_or_else(dash, gigabytes),
                );
            }
            Err(e) => {
                let _ = writeln!(
                    s,
                    "| `{}` | | | {} | failed: {} | | | | | |",
                    r.id,
                    r.threads,
                    e.replace('|', "/")
                );
            }
        }
    }
    let _ = writeln!(s, "\n## Phases\n");
    for r in runs {
        if let Ok(report) = &r.outcome
            && report.measurement.phases.len() > 1
        {
            let phases: Vec<String> = report
                .measurement
                .phases
                .iter()
                .map(|p| match p.iterations {
                    Some(i) => format!("{} {:.2} s ({} iterations)", p.name, p.seconds, grouped(i)),
                    None => format!("{} {:.2} s", p.name, p.seconds),
                })
                .collect();
            let _ = writeln!(
                s,
                "- `{}` on {}: {}",
                r.id,
                threads(r.threads),
                phases.join("; ")
            );
        }
    }
    let _ = writeln!(s, "\n## Problems\n");
    for e in chosen {
        let heavy = if e.heavy { " (heavy: `--all`)" } else { "" };
        let _ = writeln!(s, "- `{}`: {}{heavy}", e.id, e.title);
    }
    s
}

fn threads(t: usize) -> String {
    if t == 1 {
        "1 thread".into()
    } else {
        format!("{t} threads")
    }
}

/// 1234567 as "1 234 567", as the docs write numbers.
fn grouped(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

fn gigabytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.2} GB", bytes as f64 / 1e9)
    } else {
        format!("{:.0} MB", bytes as f64 / 1e6)
    }
}

/// The processor, its logical processors and the system.
fn machine() -> String {
    let cpus = std::thread::available_parallelism().map_or(1, |n| n.get());
    let name = cpu_name().unwrap_or_else(|| "an unnamed processor".into());
    format!(
        "{name}, {cpus} logical processors, {}",
        match std::env::consts::OS {
            "windows" => "Windows",
            "linux" => "Linux",
            "macos" => "macOS",
            other => other,
        }
    )
}

fn cpu_name() -> Option<String> {
    let output = |program: &str, args: &[&str]| {
        Command::new(program)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
    };
    let name = match std::env::consts::OS {
        "linux" => std::fs::read_to_string("/proc/cpuinfo")
            .ok()?
            .lines()
            .find(|l| l.starts_with("model name"))?
            .split_once(':')?
            .1
            .to_string(),
        "macos" => output("sysctl", &["-n", "machdep.cpu.brand_string"])?,
        "windows" => output(
            "reg",
            &[
                "query",
                r"HKLM\HARDWARE\DESCRIPTION\System\CentralProcessor\0",
                "/v",
                "ProcessorNameString",
            ],
        )?
        .lines()
        .find(|l| l.contains("ProcessorNameString"))?
        .split("REG_SZ")
        .nth(1)?
        .to_string(),
        _ => return None,
    };
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// The process's peak resident set.
mod memory {
    #[cfg(windows)]
    pub fn peak_bytes() -> Option<u64> {
        #[repr(C)]
        struct ProcessMemoryCounters {
            cb: u32,
            page_fault_count: u32,
            peak_working_set_size: usize,
            working_set_size: usize,
            quota_peak_paged_pool_usage: usize,
            quota_paged_pool_usage: usize,
            quota_peak_non_paged_pool_usage: usize,
            quota_non_paged_pool_usage: usize,
            pagefile_usage: usize,
            peak_pagefile_usage: usize,
        }
        unsafe extern "system" {
            fn GetCurrentProcess() -> isize;
            fn K32GetProcessMemoryInfo(
                process: isize,
                counters: *mut ProcessMemoryCounters,
                cb: u32,
            ) -> i32;
        }
        let cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
        let mut counters = ProcessMemoryCounters {
            cb,
            page_fault_count: 0,
            peak_working_set_size: 0,
            working_set_size: 0,
            quota_peak_paged_pool_usage: 0,
            quota_paged_pool_usage: 0,
            quota_peak_non_paged_pool_usage: 0,
            quota_non_paged_pool_usage: 0,
            pagefile_usage: 0,
            peak_pagefile_usage: 0,
        };
        // SAFETY: plain Win32 calls; the counters are a valid PROCESS_MEMORY_COUNTERS of size cb,
        // and GetCurrentProcess's pseudo-handle needs no closing
        let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, cb) };
        (ok != 0).then_some(counters.peak_working_set_size as u64)
    }

    #[cfg(all(unix, target_pointer_width = "64"))]
    pub fn peak_bytes() -> Option<u64> {
        // struct rusage on 64-bit Linux and macOS: two struct timevals of 16 bytes, then 14 longs,
        // ru_maxrss first
        #[repr(C)]
        struct Rusage([i64; 18]);
        unsafe extern "C" {
            fn getrusage(who: i32, usage: *mut Rusage) -> i32;
        }
        const RUSAGE_SELF: i32 = 0;
        let mut usage = Rusage([0; 18]);
        // SAFETY: getrusage writes one struct rusage, which Rusage's 144 bytes hold
        if unsafe { getrusage(RUSAGE_SELF, &mut usage) } != 0 {
            return None;
        }
        let max_rss = u64::try_from(usage.0[4]).ok()?;
        // kilobytes on Linux, bytes on macOS
        Some(if cfg!(target_os = "macos") {
            max_rss
        } else {
            max_rss * 1024
        })
    }

    #[cfg(not(any(windows, all(unix, target_pointer_width = "64"))))]
    pub fn peak_bytes() -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_timed_example_is_built_in() {
        let list = examples::list();
        for (name, _) in EXAMPLES {
            assert!(list.iter().any(|e| e.name == name), "{name}");
        }
    }

    #[test]
    fn ids_are_unique() {
        let all = entries();
        for (i, e) in all.iter().enumerate() {
            assert!(all[..i].iter().all(|f| f.id != e.id), "{} twice", e.id);
        }
    }

    #[test]
    fn numbers_are_grouped_as_the_docs_write_them() {
        assert_eq!(grouped(7), "7");
        assert_eq!(grouped(192_000), "192 000");
        assert_eq!(grouped(1_869), "1 869");
        assert_eq!(grouped(1_800_000), "1 800 000");
    }

    #[test]
    fn the_peak_memory_is_read() {
        let peak = memory::peak_bytes();
        if cfg!(any(windows, target_os = "linux", target_os = "macos")) {
            // at least the test binary itself
            assert!(peak.is_some_and(|b| b > 1 << 20), "{peak:?}");
        }
    }
}
