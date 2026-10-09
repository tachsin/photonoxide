//! What `auto` knows when a job starts: this machine's benchmark records, as the measurements
//! `photonoxide::backend::auto` chooses a direct solver from, and the external libraries
//! registered when the job or those records call for them.
//!
//! The records are the runner's (`photonoxide bench --tier`, the Benchmarks page), from the
//! results database in the app's data folder. Only those measured on this machine count, and
//! only the catalogue's direct problems: a 2D or 3D system factorized as general or as complex
//! symmetric. A run that failed its accuracy check keeps its backend from being chosen for that
//! kind of problem; one that didn't finish says nothing.
//!
//! Finding and smoke-testing the libraries takes a moment, so it is done once in a process, and
//! only for a job that names an external backend or an `fdfd` job on `auto` when the records
//! hold a run of one.

use std::sync::{Arc, Once};

use photonoxide::backend::auto::{Measured, Outcome};
use photonoxide::backend::{Choice, Form};
use photonoxide::bench::catalogue::{self, Task};
use photonoxide::run::Job;

use crate::runner::{self, Machine, Record};

/// The backends that are part of photonoxide: nothing to load for them.
fn built_in(name: &str) -> bool {
    matches!(name, "photonoxide" | "faer")
}

/// The form a catalogue problem is factorized in, if it is a direct one.
fn form(id: &str) -> Option<Form> {
    match catalogue::entry(id)?.task {
        Task::DirectGeneral => Some(Form::General),
        Task::DirectSymmetric => Some(Form::Symmetric),
        _ => None,
    }
}

/// The measurements in `records` that are `here`'s and of a direct problem. A run's time is its
/// analysis, factorization and solve: the assembly is photonoxide's, whichever the backend.
pub(crate) fn measured(records: &[Record], here: &Machine) -> Vec<Measured> {
    records
        .iter()
        .filter(|r| r.machine == *here)
        .filter_map(|r| {
            let form = form(&r.id)?;
            let solved: f64 = r
                .phases
                .iter()
                .filter(|p| p.name != "assembly")
                .map(|p| p.seconds)
                .sum();
            Some(Measured {
                family: r.family.clone(),
                form,
                unknowns: r.unknowns,
                backend: r.backend.clone(),
                threads: r.threads,
                seconds: if solved > 0.0 { solved } else { r.seconds },
                peak_bytes: r.peak_bytes,
                outcome: if r.failure.is_some() {
                    Outcome::Failed
                } else if r.accurate {
                    Outcome::Accurate
                } else {
                    Outcome::Inaccurate
                },
                unix_seconds: r.unix_seconds,
            })
        })
        .collect()
}

/// Whether `job` needs the external libraries found and registered: it names one, or it is an
/// `fdfd` job on `auto` and `measurements` hold an accurate run of one.
pub(crate) fn needs_libraries(job: &Job, measurements: &[Measured]) -> bool {
    match job.direct() {
        Choice::Named(name) => !built_in(name),
        Choice::Auto => {
            job.task().get("kind").and_then(|k| k.as_str()) == Some("fdfd")
                && measurements
                    .iter()
                    .any(|m| !built_in(&m.backend) && m.outcome == Outcome::Accurate)
        }
        _ => false,
    }
}

/// Before `job` is checked or run: gives `auto` this machine's measurements and its free
/// memory, and registers the external libraries if the job needs them (once in a process).
pub(crate) fn prepare(job: &Job) {
    static LIBRARIES: Once = Once::new();
    let records = runner::default_database()
        .and_then(|db| runner::read(&db).ok())
        .unwrap_or_default();
    let measurements = measured(&records, &Machine::here());
    if needs_libraries(job, &measurements) {
        LIBRARIES.call_once(|| {
            photonoxide_native::register_all();
        });
    }
    photonoxide::backend::auto::configure(
        measurements,
        Some(Arc::new(|| runner::system_memory().map(|(_, free)| free))),
    );
}

#[cfg(test)]
mod tests {
    use photonoxide::backend::auto::{Problem, choose};
    use photonoxide::bench::Phase;

    use super::*;

    fn machine(cpu: &str) -> Machine {
        Machine {
            cpu: cpu.into(),
            logical_processors: 8,
            memory_bytes: Some(16 << 30),
            os: "linux".into(),
        }
    }

    fn record(id: &str, backend: &str, seconds: f64, machine: &Machine) -> Record {
        let entry = catalogue::entry(id).unwrap();
        let phase = |name: &str, seconds: f64| Phase {
            name: name.into(),
            seconds,
            iterations: None,
            bytes: None,
        };
        Record {
            id: id.into(),
            family: entry.family.name().into(),
            size: entry.size,
            grid: entry.grid.clone(),
            unknowns: entry.unknowns,
            backend: backend.into(),
            backend_version: "1".into(),
            deterministic: true,
            threads: 4,
            // the assembly is the same whichever backend, and isn't its time
            phases: vec![
                phase("assembly", 100.0),
                phase("analysis and factorization", 0.75 * seconds),
                phase("one solve", 0.25 * seconds),
            ],
            seconds: 100.0 + seconds,
            peak_bytes: Some(1 << 30),
            factor_entries: None,
            error: Some(1e-13),
            tolerance: Some(1e-9),
            accurate: true,
            failure: None,
            load: None,
            machine: machine.clone(),
            version: "0.5.0".into(),
            unix_seconds: 1_791_763_200,
        }
    }

    #[test]
    fn this_machines_direct_runs_become_auto_s_measurements() {
        let (here, other) = (machine("this one"), machine("another"));
        let mut inaccurate = record("fdfd3d/strip-28", "mumps", 1.0, &here);
        inaccurate.accurate = false;
        let mut failed = record("fdfd3d/strip-28", "superlu", 1.0, &here);
        failed.failure = Some("timed out".into());
        let records = vec![
            record("fdfd3d/strip-28", "photonoxide", 8.0, &here),
            record("fdfd3d/strip-28", "pardiso", 4.0, &here),
            // another machine's, however fast
            record("fdfd3d/strip-28", "cudss", 0.1, &other),
            // a general system, an iterative problem and a dense kernel
            record("fdfd3d/grating-24", "photonoxide", 6.0, &here),
            record("fdfd3d-iterative/guide-qmr-20", "photonoxide", 1.0, &here),
            record("dense/lu-256", "photonoxide", 1.0, &here),
            inaccurate,
            failed,
        ];
        let m = measured(&records, &here);
        // this machine's five direct runs, in the records' order
        let backends: Vec<&str> = m.iter().map(|m| m.backend.as_str()).collect();
        assert_eq!(
            backends,
            ["photonoxide", "pardiso", "photonoxide", "mumps", "superlu"]
        );
        assert_eq!(m[0].family, "fdfd3d");
        assert_eq!(m[0].form, Form::Symmetric);
        assert_eq!(m[0].unknowns, 3 * 28 * 28 * 28);
        assert_eq!((m[0].threads, m[0].peak_bytes), (4, Some(1 << 30)));
        // the time without the assembly
        assert_eq!((m[0].seconds, m[1].seconds), (8.0, 4.0));
        assert_eq!(m[2].form, Form::General);
        assert_eq!(m[3].outcome, Outcome::Inaccurate);
        assert_eq!(m[4].outcome, Outcome::Failed);
        // and the rule on them: PARDISO, twice as fast near that size; never the one that
        // missed its check
        let all: Vec<String> = ["photonoxide", "faer", "pardiso", "mumps", "superlu"]
            .map(String::from)
            .into();
        let problem = Problem {
            family: "fdfd3d",
            form: Form::Symmetric,
            unknowns: 60_000,
        };
        let d = choose(problem, &m, &all, 4, None);
        assert_eq!(d.backend, "pardiso");
        assert!(d.reason.starts_with("2.0 times faster"), "{}", d.reason);
        assert!(d.reason.contains("measured 2026-10-12"), "{}", d.reason);
        // another machine has none of these records
        assert!(measured(&records, &machine("a third")).is_empty());
    }

    #[test]
    fn the_libraries_are_looked_for_only_when_a_job_needs_them() {
        let here = machine("this one");
        let job = |kind: &str, solver: &str| {
            Job::parse(&format!(
                "name = \"a\"\n{solver}\n[task]\nkind = \"{kind}\"\n"
            ))
            .unwrap()
        };
        let none = Vec::new();
        let own = measured(
            &[record("fdfd3d/strip-28", "photonoxide", 8.0, &here)],
            &here,
        );
        let external = measured(
            &[
                record("fdfd3d/strip-28", "photonoxide", 8.0, &here),
                record("fdfd3d/strip-28", "pardiso", 4.0, &here),
            ],
            &here,
        );
        // a job that names an external backend, whatever the records
        let named = job("fdfd", "[solver]\ndirect = \"pardiso\"\n");
        assert!(needs_libraries(&named, &none));
        // photonoxide's own and faer's are built in
        for name in ["photonoxide", "faer"] {
            let built = job("fdfd", &format!("[solver]\ndirect = \"{name}\"\n"));
            assert!(!needs_libraries(&built, &external), "{name}");
        }
        // auto: only an fdfd job, and only with a record of an external backend
        let auto = job("fdfd", "");
        assert!(!needs_libraries(&auto, &none));
        assert!(!needs_libraries(&auto, &own));
        assert!(needs_libraries(&auto, &external));
        assert!(!needs_libraries(&job("modes", ""), &external));
    }
}
