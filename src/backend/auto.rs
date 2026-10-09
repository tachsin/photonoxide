//! What `auto` chooses: the direct solver measured fastest on this machine, at equal accuracy,
//! for the kind and size of problem at hand.
//!
//! The measurements are the benchmark runner's records (`photonoxide bench --tier`), given
//! here by whoever keeps them ([`configure`]: the studio, from its results database, for the
//! machine it runs on). The rule ([`choose`]) is a function of them alone:
//!
//! - **Which records:** those of the problem's family and form (general, or complex symmetric),
//!   on the thread count nearest the one the solve will run on, that ran and passed their
//!   accuracy check.
//! - **Which size:** each backend's record nearest the problem's unknowns (by their ratio) and
//!   within a factor of [`REACH`] of them, compared with photonoxide's own on that same
//!   problem. A problem far from every measured size is photonoxide's own.
//! - **A margin:** a backend is taken only if it is at least [`MARGIN`] faster than
//!   photonoxide's own there. Otherwise, and with no records, photonoxide's own.
//! - **Never** a backend that isn't registered and available, nor one that missed an accuracy
//!   check in that family and form on this machine.
//! - **Memory:** a backend whose peak, scaled from its record to the problem's size, is beyond
//!   the machine's free memory is passed over for the next that fits.
//!
//! A [`Decision`] says which backend and why, in words a run's record keeps. A job that names
//! its backend isn't `auto`, and nothing here is asked.

use std::cell::RefCell;
use std::sync::{Arc, OnceLock, RwLock};

use super::{Form, OWN};

/// How much faster than photonoxide's own a backend must be measured to be chosen: a tenth.
pub const MARGIN: f64 = 0.10;

/// How far from the problem's unknowns a record may be and still speak for it: a factor of 4.
pub const REACH: f64 = 4.0;

/// How a measured run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// It ran and passed its accuracy check.
    Accurate,
    /// It ran and missed its accuracy check: its backend isn't chosen for that family and form.
    Inaccurate,
    /// It didn't finish (a timeout, an error, skipped for memory): it says nothing of the time.
    Failed,
}

/// One measured run of a direct solve.
#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    /// The problem's family, as the catalogue names it: `fdfd2d`, `fdfd3d`.
    pub family: String,
    /// The form it was factorized in.
    pub form: Form,
    /// Its unknowns.
    pub unknowns: usize,
    /// The backend's name.
    pub backend: String,
    /// The threads it ran on.
    pub threads: usize,
    /// Its time, in seconds.
    pub seconds: f64,
    /// Its process's peak memory, in bytes, if measured.
    pub peak_bytes: Option<u64>,
    /// How it ended.
    pub outcome: Outcome,
    /// When, in seconds since 1970.
    pub unix_seconds: u64,
}

/// The solve `auto` chooses for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Problem<'a> {
    /// Its family: `fdfd2d` for the 2D solver's systems, `fdfd3d` for the 3D solver's.
    pub family: &'a str,
    /// The form its matrix is factorized in.
    pub form: Form,
    /// Its unknowns.
    pub unknowns: usize,
}

/// What `auto` chose, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    /// The backend's name.
    pub backend: String,
    /// Why, e.g. `"2.1 times faster than photonoxide's own at 120000 unknowns on this
    /// machine, measured 2026-10-12"`.
    pub reason: String,
}

impl Decision {
    fn own(reason: impl Into<String>) -> Decision {
        Decision {
            backend: OWN.into(),
            reason: reason.into(),
        }
    }
}

/// The date of a time in seconds since 1970, as `2026-10-12` (the proleptic Gregorian
/// calendar, UTC).
fn date(unix_seconds: u64) -> String {
    // days since 1970-01-01 to a civil date, by eras of 400 years from 0000-03-01
    let z = (unix_seconds / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// How a factorization's memory grows from `from` unknowns to `to`: as the factors of a nested
/// dissection do, n log₂ n on a 2D grid and n^(4/3) on a 3D one.
fn growth(family: &str, from: usize, to: usize) -> f64 {
    let (a, b) = (from.max(2) as f64, to.max(2) as f64);
    if family.contains("3d") {
        (b / a).powf(4.0 / 3.0)
    } else {
        (b * b.log2()) / (a * a.log2())
    }
}

/// The rule: the backend for `problem` from `records`, among the `available` ones (registered
/// and usable; photonoxide's own is always among them), for a solve on `threads` threads with
/// `free_memory` bytes free (unknown: memory isn't considered).
pub fn choose(
    problem: Problem<'_>,
    records: &[Measured],
    available: &[String],
    threads: usize,
    free_memory: Option<u64>,
) -> Decision {
    let of_problem: Vec<&Measured> = records
        .iter()
        .filter(|r| r.family == problem.family && r.form == problem.form)
        .collect();
    if of_problem.is_empty() {
        return Decision::own("no benchmark records on this machine for this kind of problem");
    }
    // the thread count measured nearest the solve's
    let Some(nearest_threads) = of_problem
        .iter()
        .filter(|r| r.outcome == Outcome::Accurate)
        .map(|r| r.threads)
        .min_by_key(|&t| (t.abs_diff(threads), t))
    else {
        return Decision::own("no benchmark run of this kind of problem passed its check");
    };
    let accurate: Vec<&Measured> = of_problem
        .iter()
        .copied()
        .filter(|r| {
            r.outcome == Outcome::Accurate
                && r.threads == nearest_threads
                && r.seconds.is_finite()
                && r.seconds > 0.0
        })
        .collect();
    // photonoxide's own fastest run of a size
    let own_at = |unknowns: usize| -> Option<f64> {
        accurate
            .iter()
            .filter(|r| r.backend == OWN && r.unknowns == unknowns)
            .map(|r| r.seconds)
            .min_by(f64::total_cmp)
    };
    let distance = |unknowns: usize| -> f64 {
        (unknowns.max(1) as f64 / problem.unknowns.max(1) as f64)
            .ln()
            .abs()
    };
    // each other backend: its record nearest the problem's size that photonoxide's own also
    // has, and how much faster it was there
    struct Candidate<'a> {
        record: &'a Measured,
        speedup: f64,
    }
    let mut names: Vec<&str> = accurate
        .iter()
        .map(|r| r.backend.as_str())
        .filter(|&b| b != OWN)
        .collect();
    names.sort_unstable();
    names.dedup();
    let mut candidates: Vec<Candidate<'_>> = Vec::new();
    let mut passed_over: Vec<String> = Vec::new();
    for name in names {
        if !available.iter().any(|a| a == name) {
            passed_over.push(format!("{name} isn't available"));
            continue;
        }
        if of_problem
            .iter()
            .any(|r| r.backend == name && r.outcome == Outcome::Inaccurate)
        {
            passed_over.push(format!("{name} missed an accuracy check here"));
            continue;
        }
        let nearest = accurate
            .iter()
            .filter(|r| {
                r.backend == name
                    && distance(r.unknowns) <= REACH.ln()
                    && own_at(r.unknowns).is_some()
            })
            .min_by(|a, b| {
                distance(a.unknowns)
                    .total_cmp(&distance(b.unknowns))
                    .then(a.seconds.total_cmp(&b.seconds))
            });
        if let Some(record) = nearest
            && let Some(own) = own_at(record.unknowns)
        {
            candidates.push(Candidate {
                record,
                speedup: own / record.seconds,
            });
        }
    }
    // the fastest first; equal speed-ups by name, so the choice never depends on an order
    candidates.sort_by(|a, b| {
        b.speedup
            .total_cmp(&a.speedup)
            .then(a.record.backend.cmp(&b.record.backend))
    });
    for c in &candidates {
        if c.speedup < 1.0 + MARGIN {
            break;
        }
        let r = c.record;
        if let (Some(free), Some(peak)) = (free_memory, r.peak_bytes) {
            let predicted = peak as f64 * growth(problem.family, r.unknowns, problem.unknowns);
            if predicted > free as f64 {
                passed_over.push(format!(
                    "{} would need about {:.1} GB of the {:.1} GB free",
                    r.backend,
                    predicted / 1e9,
                    free as f64 / 1e9
                ));
                continue;
            }
        }
        let mut reason = format!(
            "{:.1} times faster than photonoxide's own at {} unknowns on {} thread{} on this \
             machine, measured {}",
            c.speedup,
            r.unknowns,
            r.threads,
            if r.threads == 1 { "" } else { "s" },
            date(r.unix_seconds)
        );
        if !passed_over.is_empty() {
            reason.push_str(&format!("; {}", passed_over.join("; ")));
        }
        return Decision {
            backend: r.backend.clone(),
            reason,
        };
    }
    let mut reason = match candidates.first() {
        Some(c) if c.speedup < 1.0 + MARGIN => format!(
            "no backend was measured {:.0}% faster on this machine (the best, {}, {:.2} times \
             at {} unknowns)",
            MARGIN * 100.0,
            c.record.backend,
            c.speedup,
            c.record.unknowns
        ),
        Some(_) => "no faster backend fits the free memory".into(),
        None => "no other backend was measured against photonoxide's own on this machine".into(),
    };
    if !passed_over.is_empty() {
        reason.push_str(&format!("; {}", passed_over.join("; ")));
    }
    Decision::own(reason)
}

/// What reads the machine's free memory, in bytes.
pub type FreeMemory = Arc<dyn Fn() -> Option<u64> + Send + Sync>;

#[derive(Clone, Default)]
struct State {
    records: Arc<Vec<Measured>>,
    free_memory: Option<FreeMemory>,
}

fn state() -> &'static RwLock<State> {
    static STATE: OnceLock<RwLock<State>> = OnceLock::new();
    STATE.get_or_init(|| RwLock::new(State::default()))
}

thread_local! {
    /// Records for this thread alone, in place of the configured ones ([`with`]).
    static OVERRIDE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Gives `auto` this machine's measurements and a way to read its free memory: from then on,
/// a solver that names no backend takes what [`choose`] says. Records of other machines must
/// be left out by the caller. With no records (the state before any call), `auto` is
/// photonoxide's own.
pub fn configure(records: Vec<Measured>, free_memory: Option<FreeMemory>) {
    if let Ok(mut s) = state().write() {
        *s = State {
            records: Arc::new(records),
            free_memory,
        };
    }
}

/// Runs `f` with `records` and `free_memory` as `auto`'s measurements on this thread only,
/// whatever is configured: for a solve whose choice must not depend on the machine's database.
pub fn with<T>(records: Vec<Measured>, free_memory: Option<u64>, f: impl FnOnce() -> T) -> T {
    let scoped = State {
        records: Arc::new(records),
        free_memory: free_memory.map(|bytes| Arc::new(move || Some(bytes)) as FreeMemory),
    };
    let before = OVERRIDE.with(|o| o.borrow_mut().replace(scoped));
    // restored even if `f` panics
    struct Restore(Option<State>);
    impl Drop for Restore {
        fn drop(&mut self) {
            OVERRIDE.with(|o| *o.borrow_mut() = self.0.take());
        }
    }
    let _restore = Restore(before);
    f()
}

/// What `auto` chooses for `problem` now: [`choose`] on the configured measurements, the
/// direct solvers registered and available, rayon's threads and the free memory.
pub fn decide(problem: Problem<'_>) -> Decision {
    let current = OVERRIDE
        .with(|o| o.borrow().clone())
        .or_else(|| state().read().ok().map(|s| s.clone()))
        .unwrap_or_default();
    if current.records.is_empty() {
        return Decision::own("no benchmark records on this machine");
    }
    let available: Vec<String> = super::direct_solvers()
        .map(|all| {
            all.into_iter()
                .filter(|l| l.capabilities.is_some())
                .map(|l| l.name)
                .collect()
        })
        .unwrap_or_else(|_| vec![OWN.into()]);
    let free = current.free_memory.as_ref().and_then(|read| read());
    choose(
        problem,
        &current.records,
        &available,
        rayon::current_num_threads(),
        free,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = 1_791_763_200; // 2026-10-12

    fn run(backend: &str, unknowns: usize, seconds: f64) -> Measured {
        Measured {
            family: "fdfd3d".into(),
            form: Form::Symmetric,
            unknowns,
            backend: backend.into(),
            threads: 4,
            seconds,
            peak_bytes: Some(unknowns as u64 * 10_000),
            outcome: Outcome::Accurate,
            unix_seconds: DAY,
        }
    }

    fn problem(unknowns: usize) -> Problem<'static> {
        Problem {
            family: "fdfd3d",
            form: Form::Symmetric,
            unknowns,
        }
    }

    fn all() -> Vec<String> {
        vec![OWN.into(), "faer".into(), "pardiso".into(), "mumps".into()]
    }

    #[test]
    fn dates_are_the_calendars() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(86_399), "1970-01-01");
        assert_eq!(date(951_782_400), "2000-02-29");
        assert_eq!(date(951_868_800), "2000-03-01");
        assert_eq!(date(1_709_164_800), "2024-02-29");
        assert_eq!(date(1_735_689_599), "2024-12-31");
        assert_eq!(date(DAY), "2026-10-12");
    }

    #[test]
    fn the_fastest_backend_at_the_nearest_size_is_chosen_with_its_reason() {
        let records = vec![
            run(OWN, 12_000, 1.0),
            run("pardiso", 12_000, 1.25),
            run(OWN, 120_000, 42.0),
            run("pardiso", 120_000, 20.0),
            run("mumps", 120_000, 30.0),
        ];
        // near 120 000 unknowns PARDISO was 2.1 times faster; near 12 000, slower
        let d = choose(problem(100_000), &records, &all(), 4, None);
        assert_eq!(d.backend, "pardiso");
        assert_eq!(
            d.reason,
            "2.1 times faster than photonoxide's own at 120000 unknowns on 4 threads on this \
             machine, measured 2026-10-12"
        );
        let d = choose(problem(10_000), &records, &all(), 4, None);
        assert_eq!(d.backend, OWN);
        assert!(
            d.reason.contains("no backend was measured 10% faster"),
            "{}",
            d.reason
        );
        // a problem far from every size a backend was measured at: photonoxide's own
        let far = vec![run(OWN, 120_000, 42.0), run("pardiso", 120_000, 20.0)];
        assert_eq!(
            choose(problem(480_000), &far, &all(), 4, None).backend,
            "pardiso"
        );
        assert_eq!(
            choose(problem(30_000), &far, &all(), 4, None).backend,
            "pardiso"
        );
        assert_eq!(choose(problem(500_000), &far, &all(), 4, None).backend, OWN);
        assert_eq!(choose(problem(29_000), &far, &all(), 4, None).backend, OWN);
        // another family or form has no records: photonoxide's own
        let other = Problem {
            family: "fdfd2d",
            ..problem(100_000)
        };
        assert_eq!(choose(other, &records, &all(), 4, None).backend, OWN);
        let general = Problem {
            form: Form::General,
            ..problem(100_000)
        };
        assert_eq!(choose(general, &records, &all(), 4, None).backend, OWN);
    }

    #[test]
    fn a_backend_within_the_margin_isnt_worth_leaving_photonoxides_own() {
        let near = vec![run(OWN, 50_000, 10.0), run("pardiso", 50_000, 9.2)];
        let d = choose(problem(50_000), &near, &all(), 4, None);
        assert_eq!(d.backend, OWN);
        assert!(d.reason.contains("pardiso, 1.09 times"), "{}", d.reason);
        let beyond = vec![run(OWN, 50_000, 10.0), run("pardiso", 50_000, 9.0)];
        assert_eq!(
            choose(problem(50_000), &beyond, &all(), 4, None).backend,
            "pardiso"
        );
    }

    #[test]
    fn the_fallbacks_are_photonoxides_own() {
        let records = vec![run(OWN, 120_000, 42.0), run("pardiso", 120_000, 20.0)];
        // no records at all
        let d = choose(problem(100_000), &[], &all(), 4, None);
        assert_eq!(d.backend, OWN);
        assert!(d.reason.contains("no benchmark records"));
        // the backend gone: measured, and no longer registered or available
        let d = choose(problem(100_000), &records, &[OWN.into()], 4, None);
        assert_eq!(d.backend, OWN);
        assert!(d.reason.contains("pardiso isn't available"), "{}", d.reason);
        // an accuracy failure, at any size of the family and form
        let mut failed = records.clone();
        failed.push(Measured {
            outcome: Outcome::Inaccurate,
            ..run("pardiso", 12_000, 0.1)
        });
        let d = choose(problem(100_000), &failed, &all(), 4, None);
        assert_eq!(d.backend, OWN);
        assert!(
            d.reason.contains("pardiso missed an accuracy check"),
            "{}",
            d.reason
        );
        // a run that didn't finish says nothing against its backend, and isn't a time either
        let mut timed_out = records.clone();
        timed_out.push(Measured {
            outcome: Outcome::Failed,
            ..run("pardiso", 100_000, 0.001)
        });
        assert_eq!(
            choose(problem(100_000), &timed_out, &all(), 4, None).backend,
            "pardiso"
        );
        // photonoxide's own not measured on the backend's problem: nothing to compare with
        let alone = vec![run(OWN, 12_000, 1.0), run("pardiso", 120_000, 20.0)];
        let d = choose(problem(100_000), &alone, &all(), 4, None);
        assert_eq!(d.backend, OWN);
        assert!(
            d.reason.contains("no other backend was measured against"),
            "{}",
            d.reason
        );
    }

    #[test]
    fn times_that_arent_times_say_nothing() {
        // a zero, a NaN or an infinite time, of either side, is no measurement
        for (own, other) in [
            (42.0, 0.0),
            (42.0, f64::NAN),
            (f64::INFINITY, 20.0),
            (f64::NAN, 20.0),
            (42.0, -1.0),
        ] {
            let records = vec![run(OWN, 120_000, own), run("pardiso", 120_000, other)];
            let d = choose(problem(120_000), &records, &all(), 4, None);
            assert_eq!(d.backend, OWN, "{own} {other}: {}", d.reason);
        }
    }

    #[test]
    fn a_backend_that_wont_fit_is_passed_over_for_one_that_does() {
        let lean = Measured {
            peak_bytes: Some(400_000_000),
            ..run("mumps", 120_000, 30.0)
        };
        let records = vec![
            run(OWN, 120_000, 42.0),
            // the fastest, at 1.2 GB for 120 000 unknowns
            run("pardiso", 120_000, 20.0),
            lean,
        ];
        // a problem of twice the unknowns: 1.2 GB × 2^(4/3) = 3.0 GB for PARDISO, 1.0 for MUMPS
        let twice = problem(240_000);
        assert_eq!(
            choose(twice, &records, &all(), 4, Some(8_000_000_000)).backend,
            "pardiso"
        );
        let d = choose(twice, &records, &all(), 4, Some(2_000_000_000));
        assert_eq!(d.backend, "mumps");
        assert!(
            d.reason
                .contains("pardiso would need about 3.0 GB of the 2.0 GB free"),
            "{}",
            d.reason
        );
        // nothing faster fits: photonoxide's own
        let d = choose(twice, &records, &all(), 4, Some(500_000_000));
        assert_eq!(d.backend, OWN);
        assert!(d.reason.contains("no faster backend fits"), "{}", d.reason);
        // free memory unknown, or a record without its peak: memory isn't considered
        assert_eq!(choose(twice, &records, &all(), 4, None).backend, "pardiso");
    }

    #[test]
    fn the_records_of_the_nearest_thread_count_decide() {
        let on = |threads: usize, backend: &str, seconds: f64| Measured {
            threads,
            ..run(backend, 120_000, seconds)
        };
        // on one thread PARDISO is slower, on twenty faster
        let records = vec![
            on(1, OWN, 100.0),
            on(1, "pardiso", 130.0),
            on(20, OWN, 12.0),
            on(20, "pardiso", 6.0),
        ];
        assert_eq!(
            choose(problem(120_000), &records, &all(), 1, None).backend,
            OWN
        );
        assert_eq!(
            choose(problem(120_000), &records, &all(), 2, None).backend,
            OWN
        );
        assert_eq!(
            choose(problem(120_000), &records, &all(), 16, None).backend,
            "pardiso"
        );
        let d = choose(problem(120_000), &records, &all(), 20, None);
        assert!(d.reason.contains("on 20 threads"), "{}", d.reason);
    }

    #[test]
    fn equal_speed_ups_are_settled_by_name_whatever_the_records_order() {
        let mut records = vec![
            run(OWN, 120_000, 40.0),
            run("pardiso", 120_000, 20.0),
            run("mumps", 120_000, 20.0),
        ];
        assert_eq!(
            choose(problem(120_000), &records, &all(), 4, None).backend,
            "mumps"
        );
        records.reverse();
        assert_eq!(
            choose(problem(120_000), &records, &all(), 4, None).backend,
            "mumps"
        );
    }

    #[test]
    fn auto_decides_from_what_is_configured_on_this_thread_and_only_there() {
        // nothing configured: photonoxide's own
        let d = with(Vec::new(), None, || decide(problem(100_000)));
        assert_eq!(d.backend, OWN);
        // faer's LU is registered: measured faster, it is chosen; a backend that isn't, isn't
        let records = vec![
            run(OWN, 120_000, 42.0),
            run("faer", 120_000, 30.0),
            run("auto-test-unregistered", 120_000, 1.0),
        ];
        let d = with(records.clone(), None, || decide(problem(100_000)));
        assert_eq!(d.backend, "faer");
        assert!(
            d.reason.contains("auto-test-unregistered isn't available"),
            "{}",
            d.reason
        );
        // free memory too small for it
        let d = with(records.clone(), Some(1_000), || decide(problem(100_000)));
        assert_eq!(d.backend, OWN);
        // and another thread sees none of it
        let elsewhere = with(records, None, || {
            std::thread::spawn(|| decide(problem(100_000)))
                .join()
                .unwrap()
        });
        assert_eq!(elsewhere.backend, OWN);
    }
}
