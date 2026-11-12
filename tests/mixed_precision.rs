//! `photonoxide-mixed`, photonoxide's own factorization in single precision refined in double
//! (docs/methods/mixed-precision.md), as a user has it: by name, never by `auto`; and the
//! validation cases run on it in `auto`'s place.
//!
//! The validation run: `cargo test --release --test mixed_precision -- --ignored --nocapture`,
//! with `PHOTONOXIDE_MIXED_CASES` the case prefixes (`fdfd3d/` by default). Each case runs with
//! `photonoxide-mixed` preferred for `auto` (`backend::prefer_direct`), wrapped to count its
//! factorizations; a case that made none isn't listed. Each listed case runs again on
//! photonoxide's own, and both are printed as a Markdown table (to `PHOTONOXIDE_MIXED_OUT` too,
//! if set): the values each reported, their difference, the case's tolerance and whether the
//! mixed run is within it, by the case's own rule. It fails if one isn't.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::Result;
use photonoxide::backend::auto::{self, Measured, Outcome as Ran};
use photonoxide::backend::{
    self, Analysis, Capabilities, Choice, DirectSolver, Factorization, Form, MIXED, Matrix,
};
use photonoxide::validation::{self, Outcome};

/// The factorizations through the backend.
static CALLS: AtomicUsize = AtomicUsize::new(0);

const COUNTED: &str = "photonoxide-mixed-counted";

/// photonoxide-mixed under another name, counting its factorizations.
struct Counting(Arc<dyn DirectSolver>);

struct CountingAnalysis(Arc<dyn Analysis>);

impl DirectSolver for Counting {
    fn capabilities(&self) -> Capabilities {
        let mut c = self.0.capabilities();
        c.name = COUNTED.into();
        c
    }

    fn analyse(&self, matrix: &Matrix<'_>) -> Result<Option<Arc<dyn Analysis>>> {
        Ok(self
            .0
            .analyse(matrix)?
            .map(|a| Arc::new(CountingAnalysis(a)) as Arc<dyn Analysis>))
    }
}

impl Analysis for CountingAnalysis {
    fn form(&self) -> Form {
        self.0.form()
    }

    fn factorize(&self, matrix: &Matrix<'_>) -> Result<Box<dyn Factorization>> {
        CALLS.fetch_add(1, Ordering::Relaxed);
        self.0.factorize(matrix)
    }
}

#[test]
fn it_is_registered_taken_by_name_and_never_chosen_by_auto() {
    let listed = backend::direct_solvers().unwrap();
    let mixed = listed.iter().find(|l| l.name == MIXED).unwrap();
    let capabilities = mixed.capabilities.clone().unwrap();
    assert!(capabilities.refined && capabilities.symmetric && capabilities.transpose);
    assert!(capabilities.deterministic);
    // photonoxide's own name: nobody else registers under it
    assert!(backend::register_unavailable(MIXED, "no").is_err());
    // records in which it is ten times faster than photonoxide's own at every size: auto still
    // keeps photonoxide's own, and doesn't call it unavailable
    let records: Vec<Measured> = (10..=24)
        .flat_map(|k| {
            [("photonoxide", 10.0), (MIXED, 1.0)].map(|(name, seconds)| Measured {
                family: "fdfd3d".into(),
                form: Form::Symmetric,
                unknowns: 1 << k,
                backend: name.into(),
                threads: rayon::current_num_threads(),
                seconds,
                peak_bytes: None,
                outcome: Ran::Accurate,
                unix_seconds: 1_791_763_200,
            })
        })
        .collect();
    let problem = auto::Problem {
        family: "fdfd3d",
        form: Form::Symmetric,
        unknowns: 1 << 17,
    };
    let decision = auto::with(records, None, || auto::decide(problem));
    assert_eq!(decision.backend, "photonoxide", "{}", decision.reason);
    assert!(!decision.reason.contains(MIXED), "{}", decision.reason);
}

#[test]
#[ignore = "runs the validation cases on photonoxide-mixed: cargo test --release --test mixed_precision -- --ignored --nocapture"]
fn the_validation_cases_pass_on_single_precision_factors() {
    let prefixes = std::env::var("PHOTONOXIDE_MIXED_CASES").unwrap_or_else(|_| "fdfd3d/".into());
    let prefixes: Vec<&str> = prefixes.split_whitespace().collect();
    let mixed = backend::direct(&Choice::Named(MIXED.into())).unwrap();
    backend::register(Arc::new(Counting(mixed))).unwrap();
    let cases: Vec<validation::Case> = validation::cases()
        .into_iter()
        .filter(|c| prefixes.iter().any(|p| c.id.starts_with(p)))
        .collect();
    let run =
        |case: &validation::Case, choice: &Choice| -> (std::result::Result<Outcome, String>, f64) {
            backend::prefer_direct(choice).unwrap();
            let start = Instant::now();
            let outcome =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(case.run)).map_err(|p| {
                    p.downcast_ref::<String>()
                        .cloned()
                        .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                        .unwrap_or_else(|| "a panic".into())
                });
            backend::prefer_direct(&Choice::Auto).unwrap();
            (outcome, start.elapsed().as_secs_f64())
        };
    let number = |x: f64| {
        if x == 0.0 {
            "0".to_string()
        } else if (1e-3..1e5).contains(&x.abs()) {
            format!("{x:.6}")
        } else {
            format!("{x:.6e}")
        }
    };
    let mut rows = Vec::new();
    let mut failed = 0;
    let mut unused = Vec::new();
    for (k, case) in cases.iter().enumerate() {
        CALLS.store(0, Ordering::Relaxed);
        let (theirs, their_seconds) = run(case, &Choice::Named(COUNTED.into()));
        let calls = CALLS.swap(0, Ordering::Relaxed);
        if calls == 0 {
            unused.push(case.id);
            eprintln!(
                "[{}/{}] {}: no factorization ({their_seconds:.1} s)",
                k + 1,
                cases.len(),
                case.id
            );
            continue;
        }
        let (own, own_seconds) = run(case, &Choice::Auto);
        assert_eq!(
            CALLS.load(Ordering::Relaxed),
            0,
            "{}: mixed used on the own run",
            case.id
        );
        let row = match (&own, &theirs) {
            (Ok(own), Ok(theirs)) => {
                let passed = theirs.passed();
                if !passed {
                    failed += 1;
                }
                format!(
                    "| `{}` | {calls} | {} | {} | {} | {} | {} |",
                    case.id,
                    number(own.measured),
                    number(theirs.measured),
                    number((theirs.measured - own.measured).abs()),
                    number(theirs.tolerance),
                    if passed { "yes" } else { "**no**" }
                )
            }
            _ => {
                failed += 1;
                let said = |r: &std::result::Result<Outcome, String>| match r {
                    Ok(o) => number(o.measured),
                    Err(e) => format!("panicked: {}", e.replace('|', "/").replace('\n', " ")),
                };
                format!(
                    "| `{}` | {calls} | {} | {} | | | **no** |",
                    case.id,
                    said(&own),
                    said(&theirs)
                )
            }
        };
        eprintln!(
            "[{}/{}] {row} (own {own_seconds:.1} s, mixed {their_seconds:.1} s)",
            k + 1,
            cases.len()
        );
        rows.push(row);
    }
    let mut table = format!(
        "{} of {} cases factorized ({} threads); {failed} failed\n\n| Case | Factorizations | \
         photonoxide | photonoxide-mixed | Difference | Tolerance | Within |\n|---|---|---|---|---|---|---|\n",
        rows.len(),
        cases.len(),
        rayon::current_num_threads()
    );
    for row in &rows {
        table.push_str(row);
        table.push('\n');
    }
    table.push_str(&format!("\nNo factorization: {}\n", unused.join(", ")));
    println!("{table}");
    if let Some(out) = std::env::var_os("PHOTONOXIDE_MIXED_OUT") {
        std::fs::write(out, &table).unwrap();
    }
    assert_eq!(failed, 0, "{table}");
    let _ = c64::new(0.0, 0.0);
}
