//! The studio's Benchmarks page: this machine's benchmark records and their summaries, the
//! plan of a run before it starts, and importing and exporting records. The runs themselves
//! are `photonoxide bench --tier …` in a process of its own (a task, as an example's is), which
//! runs each problem in a child process of its own and appends each record to the database as
//! it ends (runner.rs).
//!
//! The plan follows the runner's loops exactly (thread counts, then the catalogue's entries
//! whose ids start with one asked for, then photonoxide's own and each backend that applies),
//! so the window can tell which run is going from the lines the runner prints.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use photonoxide::bench::catalogue::{self, Entry, Task, Tier};
use serde::{Deserialize, Serialize};

use crate::runner::{self, Machine, Record, Summary};

/// A summary's row against photonoxide's own, with its grid.
#[derive(Debug, PartialEq, Serialize)]
pub struct Against {
    pub id: String,
    pub family: String,
    pub grid: String,
    pub unknowns: usize,
    pub threads: usize,
    pub backend: String,
    pub seconds: f64,
    pub own_seconds: f64,
    pub speedup: f64,
    pub memory_ratio: Option<f64>,
}

/// A fitted exponent of time against unknowns.
#[derive(Debug, PartialEq, Serialize)]
pub struct Exponent {
    pub family: String,
    pub backend: String,
    pub threads: usize,
    pub exponent: f64,
    pub sizes: usize,
}

/// The fastest backend on one problem at one thread count, and by how much over the next.
#[derive(Debug, PartialEq, Serialize)]
pub struct Best {
    pub id: String,
    pub family: String,
    pub grid: String,
    pub unknowns: usize,
    pub threads: usize,
    pub backend: String,
    pub seconds: f64,
    /// The runner-up and its time.
    pub next: Option<(String, f64)>,
    /// The backend with the least peak memory, and its peak.
    pub leanest: Option<(String, u64)>,
}

/// The fewest unknowns from which a backend beats photonoxide's own on every larger problem of
/// a family measured with both.
#[derive(Debug, PartialEq, Serialize)]
pub struct Crossover {
    pub family: String,
    pub backend: String,
    pub threads: usize,
    /// The problem it starts at.
    pub id: String,
    pub grid: String,
    pub unknowns: usize,
}

/// What one machine's records say.
#[derive(Debug, Default, PartialEq, Serialize)]
pub struct Summaries {
    pub against_own: Vec<Against>,
    pub exponents: Vec<Exponent>,
    /// By family, threads and unknowns.
    pub best: Vec<Best>,
    pub crossovers: Vec<Crossover>,
}

/// The fastest counted run of each (id, backend, threads), as the runner's summary takes it.
fn fastest(records: &[Record]) -> BTreeMap<(String, String, usize), &Record> {
    let mut out: BTreeMap<(String, String, usize), &Record> = BTreeMap::new();
    for r in records.iter().filter(|r| r.counts()) {
        let key = (r.id.clone(), r.backend.clone(), r.threads);
        if out.get(&key).is_none_or(|f| r.seconds < f.seconds) {
            out.insert(key, r);
        }
    }
    out
}

/// The fastest backend on each problem and thread count. The runner's own summary ranks by a
/// family's size, which several problems of different unknowns share (a 3D box's shorter
/// side); the page ranks by problem, each with its grid.
fn best(fastest: &BTreeMap<(String, String, usize), &Record>) -> Vec<Best> {
    // (id, threads) → its runs
    let mut by_problem: BTreeMap<(&str, usize), Vec<&Record>> = BTreeMap::new();
    for r in fastest.values() {
        by_problem.entry((&r.id, r.threads)).or_default().push(r);
    }
    let mut out: Vec<Best> = by_problem
        .into_values()
        .map(|mut runs| {
            runs.sort_by(|a, b| a.seconds.total_cmp(&b.seconds));
            let w = runs[0];
            Best {
                id: w.id.clone(),
                family: w.family.clone(),
                grid: w.grid.clone(),
                unknowns: w.unknowns,
                threads: w.threads,
                backend: w.backend.clone(),
                seconds: w.seconds,
                next: runs.get(1).map(|r| (r.backend.clone(), r.seconds)),
                leanest: runs
                    .iter()
                    .filter_map(|r| r.peak_bytes.map(|p| (r.backend.clone(), p)))
                    .min_by_key(|p| p.1),
            }
        })
        .collect();
    out.sort_by(|a, b| {
        (&a.family, a.threads, a.unknowns, &a.id).cmp(&(&b.family, b.threads, b.unknowns, &b.id))
    });
    out
}

/// For each family, thread count and backend, the fewest unknowns from which the backend is
/// faster than photonoxide's own on every problem measured with both.
fn crossovers(fastest: &BTreeMap<(String, String, usize), &Record>) -> Vec<Crossover> {
    // (family, backend, threads) → each problem, and whether the backend was faster there
    type Runs<'a> = Vec<(&'a Record, bool)>;
    let mut against: BTreeMap<(&str, &str, usize), Runs> = BTreeMap::new();
    for ((id, backend, threads), r) in fastest {
        if backend == "photonoxide" {
            continue;
        }
        if let Some(own) = fastest.get(&(id.clone(), "photonoxide".to_string(), *threads)) {
            against
                .entry((&r.family, backend, *threads))
                .or_default()
                .push((r, r.seconds < own.seconds));
        }
    }
    let mut out = Vec::new();
    for ((family, backend, threads), mut runs) in against {
        runs.sort_by(|a, b| (a.0.unknowns, &a.0.id).cmp(&(b.0.unknowns, &b.0.id)));
        // the largest first: walk down while the backend stays faster
        let mut from = None;
        for (r, faster) in runs.iter().rev() {
            if !faster {
                break;
            }
            from = Some(*r);
        }
        if let Some(r) = from {
            out.push(Crossover {
                family: family.into(),
                backend: backend.into(),
                threads,
                id: r.id.clone(),
                grid: r.grid.clone(),
                unknowns: r.unknowns,
            });
        }
    }
    out
}

/// What one machine's records say: the runner's speed-ups and exponents, each row with its
/// grid, the fastest on each problem, and the crossovers by unknowns.
pub fn summaries(records: &[Record]) -> Summaries {
    let Summary {
        against_own,
        exponents,
        ..
    } = runner::summarize(records);
    let by_id: BTreeMap<&str, &Record> = records.iter().map(|r| (r.id.as_str(), r)).collect();
    let fastest = fastest(records);
    Summaries {
        against_own: against_own
            .into_iter()
            .map(|a| {
                let r = by_id.get(a.id.as_str());
                Against {
                    family: r.map_or_else(String::new, |r| r.family.clone()),
                    grid: r.map_or_else(String::new, |r| r.grid.clone()),
                    unknowns: r.map_or(0, |r| r.unknowns),
                    id: a.id,
                    threads: a.threads,
                    backend: a.backend,
                    seconds: a.seconds,
                    own_seconds: a.own_seconds,
                    speedup: a.speedup,
                    memory_ratio: a.memory_ratio,
                }
            })
            .collect(),
        exponents: exponents
            .into_iter()
            .map(|(family, backend, threads, exponent, sizes)| Exponent {
                family,
                backend,
                threads,
                exponent,
                sizes,
            })
            .collect(),
        best: best(&fastest),
        crossovers: crossovers(&fastest),
    }
}

/// One machine's records and what they say.
#[derive(Serialize)]
pub struct MachineData {
    pub machine: Machine,
    /// Whether it is this machine.
    pub here: bool,
    pub records: Vec<Record>,
    pub summaries: Summaries,
}

/// The database, by machine, this one first.
#[derive(Serialize)]
pub struct BenchData {
    pub database: String,
    pub machines: Vec<MachineData>,
    /// This machine, even with no records.
    pub here: Machine,
    /// Its free memory now, in bytes, where the system says.
    pub free_bytes: Option<u64>,
}

/// This machine, read once: its processor's name comes from a program (`reg` on Windows).
fn here() -> Machine {
    static HERE: std::sync::OnceLock<Machine> = std::sync::OnceLock::new();
    HERE.get_or_init(Machine::here).clone()
}

fn database() -> Result<PathBuf, String> {
    runner::default_database().ok_or_else(|| "no data folder for the results database".into())
}

/// The records of `path`, grouped by machine, `here` first.
pub fn by_machine(records: Vec<Record>, here: &Machine) -> Vec<MachineData> {
    let mut groups: BTreeMap<Machine, Vec<Record>> = BTreeMap::new();
    for r in records {
        groups.entry(r.machine.clone()).or_default().push(r);
    }
    let mut out: Vec<MachineData> = groups
        .into_iter()
        .map(|(machine, records)| MachineData {
            here: &machine == here,
            summaries: summaries(&records),
            machine,
            records,
        })
        .collect();
    out.sort_by_key(|m| !m.here);
    out
}

/// This machine's records and every other machine's in the database, each with its summary.
#[tauri::command(async)]
pub fn bench_data() -> Result<BenchData, String> {
    let path = database()?;
    let here = here();
    Ok(BenchData {
        database: path.display().to_string(),
        machines: by_machine(runner::read(&path)?, &here),
        free_bytes: runner::system_memory().map(|m| m.1),
        here,
    })
}

/// One run of a plan.
#[derive(Debug, PartialEq, Serialize)]
pub struct Planned {
    pub id: String,
    pub family: String,
    pub size: usize,
    pub grid: String,
    pub unknowns: usize,
    /// `direct general`, `iterative` and the like.
    pub task: String,
    pub memory_bytes: u64,
    pub backend: String,
    pub threads: usize,
    /// How long it took here before (the fastest counted run of the same problem, backend
    /// and threads), if it ran.
    pub seconds_before: Option<f64>,
}

/// A catalogue entry, for the page's choice of families and sizes.
#[derive(Debug, Serialize)]
pub struct EntryView {
    pub id: String,
    pub family: String,
    pub title: String,
    pub size: usize,
    pub grid: String,
    pub unknowns: usize,
    pub task: String,
    pub memory_bytes: u64,
    pub tier: &'static str,
    pub check: String,
    /// Whether a direct backend, or an iterative one, can solve it.
    pub direct: bool,
    pub iterative: bool,
}

fn task(t: Task) -> String {
    match t {
        Task::DirectGeneral => "direct, general".into(),
        Task::DirectSymmetric => "direct, symmetric".into(),
        Task::Iterative { tolerance } => format!("iterative to {tolerance:.0e}"),
        Task::Eigen { modes } => format!("eigen, {modes} modes"),
        Task::Dense => "dense".into(),
        Task::Mixed => "a whole run".into(),
        Task::TimeSteps { steps } => format!("{steps} time steps"),
        other => format!("{other:?}"),
    }
}

/// Every entry of the catalogue.
#[tauri::command(async)]
pub fn bench_catalogue() -> Vec<EntryView> {
    catalogue::catalogue()
        .into_iter()
        .map(|e| EntryView {
            direct: e.takes_a_backend(),
            iterative: e.takes_an_iterative_backend(),
            family: e.family.name().into(),
            task: task(e.task),
            tier: e.tier.name(),
            id: e.id,
            title: e.title,
            size: e.size,
            grid: e.grid,
            unknowns: e.unknowns,
            memory_bytes: e.memory_bytes,
            check: e.check,
        })
        .collect()
}

/// A backend the page offers, and what it solves.
#[derive(Clone, Debug, Deserialize)]
pub struct Offered {
    pub name: String,
    /// `direct` or `iterative`.
    pub kind: String,
}

/// What to run: as `photonoxide bench --tier <tier> --backends <names> --threads <counts>
/// <ids>` would.
#[derive(Clone, Debug, Deserialize)]
pub struct Request {
    pub tier: String,
    /// The backends besides photonoxide's own, as the Libraries page lists them.
    pub backends: Vec<Offered>,
    pub threads: Vec<usize>,
    /// Id prefixes; none: the whole tier.
    pub ids: Vec<String>,
    /// Seconds a run may take.
    pub timeout: u64,
}

/// The runs `request` makes, in the runner's order.
pub fn plan_of(entries: &[Entry], request: &Request, records: &[Record]) -> Vec<Planned> {
    let fastest = fastest(records);
    let mut names = vec![Offered {
        name: "photonoxide".into(),
        kind: "direct".into(),
    }];
    for b in &request.backends {
        if !names.iter().any(|n| n.name == b.name) {
            names.push(b.clone());
        }
    }
    let mut out = Vec::new();
    for &threads in &request.threads {
        for e in entries.iter().filter(|e| {
            request.ids.is_empty() || request.ids.iter().any(|w| e.id.starts_with(w.as_str()))
        }) {
            for b in &names {
                let applies = b.name == "photonoxide"
                    || (b.kind == "direct" && e.takes_a_backend())
                    || (b.kind == "iterative" && e.takes_an_iterative_backend());
                if !applies {
                    continue;
                }
                out.push(Planned {
                    id: e.id.clone(),
                    family: e.family.name().into(),
                    size: e.size,
                    grid: e.grid.clone(),
                    unknowns: e.unknowns,
                    task: task(e.task),
                    memory_bytes: e.memory_bytes,
                    backend: b.name.clone(),
                    threads,
                    seconds_before: fastest
                        .get(&(e.id.clone(), b.name.clone(), threads))
                        .map(|r| r.seconds),
                });
            }
        }
    }
    out
}

fn tier(name: &str) -> Result<Tier, String> {
    Tier::parse(name).ok_or_else(|| format!("no tier {name}: quick, standard or full"))
}

/// The runs a request makes, with what they took here before.
#[tauri::command(async)]
pub fn bench_plan(request: Request) -> Result<Vec<Planned>, String> {
    let entries = catalogue::tier(tier(&request.tier)?);
    let here = here();
    let records: Vec<Record> = runner::read(&database()?)?
        .into_iter()
        .filter(|r| r.machine == here)
        .collect();
    Ok(plan_of(&entries, &request, &records))
}

/// The command line of a request.
pub fn arguments(request: &Request) -> Result<Vec<String>, String> {
    tier(&request.tier)?;
    if request.threads.is_empty() || request.threads.contains(&0) {
        return Err("give one thread count or more, each above zero".into());
    }
    if request.timeout == 0 {
        return Err("a run needs more than 0 s".into());
    }
    let mut args = vec![
        "bench".to_string(),
        "--tier".into(),
        request.tier.clone(),
        "--threads".into(),
        request
            .threads
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(","),
        "--timeout".into(),
        request.timeout.to_string(),
    ];
    let names: Vec<&str> = request
        .backends
        .iter()
        .map(|b| b.name.as_str())
        .filter(|n| *n != "photonoxide")
        .collect();
    for n in &names {
        if !n.starts_with(|c: char| c.is_ascii_alphanumeric())
            || !n
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(format!("no backend {n}"));
        }
    }
    if !names.is_empty() {
        args.push("--backends".into());
        args.push(names.join(","));
    }
    for id in &request.ids {
        if id.starts_with('-') {
            return Err(format!("no problem {id}"));
        }
        args.push(id.clone());
    }
    Ok(args)
}

/// Adds another database's records to this one, those it doesn't hold already; returns how
/// many.
pub fn import(from: &Path, into: &Path) -> Result<usize, String> {
    let have = runner::read(into)?;
    let new: Vec<Record> = runner::read(from)?
        .into_iter()
        .filter(|r| !have.contains(r))
        .collect();
    if new.is_empty() && !from.is_file() {
        return Err(format!("{} isn't there", from.display()));
    }
    runner::append(into, &new)?;
    Ok(new.len())
}

#[tauri::command(async)]
pub fn bench_import(path: String) -> Result<usize, String> {
    import(Path::new(&path), &database()?)
}

/// The records as CSV, a row each, with their grids.
pub fn csv(records: &[Record]) -> String {
    let quote = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
    let mut out = String::from(
        "id,family,size,grid,unknowns,backend,backend_version,deterministic,threads,seconds,peak_bytes,error,tolerance,accurate,failure,cpu,logical_processors,os,photonoxide,unix_seconds\n",
    );
    let opt = |x: Option<String>| x.unwrap_or_default();
    for r in records {
        let row = [
            quote(&r.id),
            quote(&r.family),
            r.size.to_string(),
            quote(&r.grid),
            r.unknowns.to_string(),
            quote(&r.backend),
            quote(&r.backend_version),
            r.deterministic.to_string(),
            r.threads.to_string(),
            if r.seconds.is_finite() {
                r.seconds.to_string()
            } else {
                String::new()
            },
            opt(r.peak_bytes.map(|b| b.to_string())),
            opt(r.error.map(|e| e.to_string())),
            opt(r.tolerance.map(|t| t.to_string())),
            r.accurate.to_string(),
            quote(r.failure.as_deref().unwrap_or("")),
            quote(&r.machine.cpu),
            r.machine.logical_processors.to_string(),
            quote(&r.machine.os),
            quote(&r.version),
            r.unix_seconds.to_string(),
        ];
        out.push_str(&row.join(","));
        out.push('\n');
    }
    out
}

/// Writes the database's records to `path`: JSON lines (the database's own format, which
/// another machine imports) for `.jsonl` and `.json`, CSV for `.csv`, and the runner's report
/// of this machine's records for `.md`.
#[tauri::command(async)]
pub fn bench_export(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    let db = database()?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "md" => {
            // the runner's report, as `photonoxide bench --report --write <file>` writes it
            let exe =
                std::env::current_exe().map_err(|e| format!("can't find the program: {e}"))?;
            let mut command = Command::new(exe);
            command
                .args(["bench", "--report", "--write"])
                .arg(&path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped());
            crate::libraries::no_window(&mut command);
            let output = command.output().map_err(|e| e.to_string())?;
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
            }
        }
        "csv" => std::fs::write(&path, csv(&runner::read(&db)?))
            .map_err(|e| format!("{}: {e}", path.display())),
        _ => {
            let _ = std::fs::remove_file(&path);
            runner::append(&path, &runner::read(&db)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(cpu: &str) -> Machine {
        Machine {
            cpu: cpu.into(),
            logical_processors: 4,
            memory_bytes: Some(1 << 30),
            os: "linux".into(),
        }
    }

    fn record(id: &str, size: usize, backend: &str, seconds: f64, peak: u64) -> Record {
        let line = serde_json::json!({
            "id": id, "family": "fdfd2d", "size": size, "grid": format!("{size} × {size} cells"),
            "unknowns": size * size, "backend": backend, "backend_version": "1",
            "deterministic": true, "threads": 4, "phases": [], "seconds": seconds,
            "peak_bytes": peak, "error": 1e-12, "tolerance": 1e-9, "accurate": true,
            "failure": null, "load": null, "machine": machine("test"), "version": "0.0.0",
            "unix_seconds": 0
        });
        serde_json::from_value(line).unwrap()
    }

    #[test]
    fn summaries_carry_grids_runners_up_and_the_leanest() {
        let records = vec![
            record("a-100", 100, "photonoxide", 2.0, 300),
            record("a-100", 100, "gpu", 1.0, 500),
            record("a-100", 100, "faer", 4.0, 200),
            record("a-200", 200, "photonoxide", 16.0, 1200),
            record("a-200", 200, "gpu", 4.0, 2000),
            record("a-200", 200, "faer", 8.0, 900),
        ];
        let s = summaries(&records);
        let first = &s.best[0];
        assert_eq!(first.backend, "gpu");
        assert_eq!(first.grid, "100 × 100 cells");
        assert_eq!(first.unknowns, 10_000);
        assert_eq!(first.next, Some(("photonoxide".into(), 2.0)));
        assert_eq!(first.leanest, Some(("faer".into(), 200)));
        let gpu = s
            .against_own
            .iter()
            .find(|a| a.id == "a-200" && a.backend == "gpu")
            .unwrap();
        assert_eq!(gpu.grid, "200 × 200 cells");
        assert!((gpu.speedup - 4.0).abs() < 1e-12);
        assert_eq!(s.best.len(), 2);
        assert_eq!(s.best[1].id, "a-200");
        assert_eq!(s.best[1].next, Some(("faer".into(), 8.0)));
        // the GPU is faster on both problems, from the smaller up; faer on the larger only
        let from = |b: &str| s.crossovers.iter().find(|c| c.backend == b).unwrap();
        assert_eq!(s.crossovers.len(), 2);
        assert_eq!(from("gpu").id, "a-100");
        assert_eq!(from("gpu").grid, "100 × 100 cells");
        assert_eq!(from("gpu").unknowns, 10_000);
        assert_eq!(from("faer").id, "a-200");
        // time grows as unknowns^1.5 for photonoxide's: 2 s to 16 s over 4 times the unknowns
        let own = s
            .exponents
            .iter()
            .find(|e| e.backend == "photonoxide")
            .unwrap();
        assert!((own.exponent - 1.5).abs() < 1e-12);
    }

    #[test]
    fn records_group_by_machine_this_one_first() {
        let here = machine("here");
        let mut other = record("a-100", 100, "photonoxide", 1.0, 1);
        other.machine = machine("another");
        let mut mine = record("a-100", 100, "photonoxide", 2.0, 1);
        mine.machine = here.clone();
        let groups = by_machine(vec![other, mine], &here);
        assert_eq!(groups.len(), 2);
        assert!(groups[0].here && groups[0].machine == here);
        assert!(!groups[1].here);
    }

    fn request(backends: &[(&str, &str)], threads: &[usize], ids: &[&str]) -> Request {
        Request {
            tier: "quick".into(),
            backends: backends
                .iter()
                .map(|(n, k)| Offered {
                    name: (*n).into(),
                    kind: (*k).into(),
                })
                .collect(),
            threads: threads.to_vec(),
            ids: ids.iter().map(|s| (*s).into()).collect(),
            timeout: 600,
        }
    }

    #[test]
    fn a_plan_follows_the_runners_loops() {
        let entries = catalogue::tier(Tier::Quick);
        let r = request(
            &[("pardiso", "direct"), ("cusparse", "iterative")],
            &[1, 4],
            &[],
        );
        let plan = plan_of(&entries, &r, &[]);
        // threads outermost, then the entries, then photonoxide's own and each that applies
        let per_thread = plan.len() / 2;
        assert!(plan[..per_thread].iter().all(|p| p.threads == 1));
        assert!(plan[per_thread..].iter().all(|p| p.threads == 4));
        for e in &entries {
            let runs: Vec<&str> = plan[..per_thread]
                .iter()
                .filter(|p| p.id == e.id)
                .map(|p| p.backend.as_str())
                .collect();
            assert_eq!(runs[0], "photonoxide");
            assert_eq!(runs.contains(&"pardiso"), e.takes_a_backend(), "{}", e.id);
            assert_eq!(
                runs.contains(&"cusparse"),
                e.takes_an_iterative_backend(),
                "{}",
                e.id
            );
        }
        // ids are prefixes, as the runner takes them
        let fdfd = plan_of(&entries, &request(&[], &[1], &["fdfd2d"]), &[]);
        assert!(!fdfd.is_empty());
        assert!(
            fdfd.iter()
                .all(|p| p.id.starts_with("fdfd2d") && p.backend == "photonoxide")
        );
    }

    #[test]
    fn a_plan_says_what_a_run_took_before() {
        let entries = catalogue::tier(Tier::Quick);
        let first = &entries[0];
        let mut before = record(&first.id, first.size, "photonoxide", 3.0, 1);
        before.threads = 2;
        let plan = plan_of(&entries, &request(&[], &[2], &[&first.id]), &[before]);
        assert_eq!(plan[0].seconds_before, Some(3.0));
    }

    #[test]
    fn the_command_line_is_the_clis() {
        let r = request(
            &[("pardiso", "direct"), ("cudss", "direct")],
            &[1, 20],
            &["fdfd2d/"],
        );
        assert_eq!(
            arguments(&r).unwrap(),
            [
                "bench",
                "--tier",
                "quick",
                "--threads",
                "1,20",
                "--timeout",
                "600",
                "--backends",
                "pardiso,cudss",
                "fdfd2d/"
            ]
        );
        assert!(arguments(&request(&[], &[], &[])).is_err());
        assert!(arguments(&request(&[("--db", "direct")], &[1], &[])).is_err());
        assert!(arguments(&request(&[], &[1], &["--write"])).is_err());
        let mut huge = request(&[], &[1], &[]);
        huge.tier = "huge".into();
        assert!(arguments(&huge).is_err());
    }

    #[test]
    fn import_adds_only_new_records_and_csv_has_a_row_each() {
        let dir = std::env::temp_dir().join(format!("photonoxide-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (from, into) = (dir.join("from.jsonl"), dir.join("into.jsonl"));
        let a = record("a-100", 100, "photonoxide", 1.0, 1);
        let b = record("a-100", 100, "gpu", 0.5, 1);
        runner::append(&into, std::slice::from_ref(&a)).unwrap();
        runner::append(&from, &[a.clone(), b.clone()]).unwrap();
        assert_eq!(import(&from, &into).unwrap(), 1);
        assert_eq!(import(&from, &into).unwrap(), 0);
        assert_eq!(runner::read(&into).unwrap(), [a, b]);
        assert!(import(&dir.join("missing.jsonl"), &into).is_err());
        let text = csv(&runner::read(&into).unwrap());
        assert_eq!(text.lines().count(), 3);
        assert!(text.lines().nth(1).unwrap().contains("\"100 × 100 cells\""));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
