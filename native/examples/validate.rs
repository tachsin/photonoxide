//! The validation cases with an external backend in photonoxide's own place, beside photonoxide's
//! own results: `cargo run -p photonoxide-native --release --example validate -- <backend>
//! [case prefix ...]`, e.g. `-- mumps fdfd/ fdfd3d/`.
//!
//! The cases ask for `auto`, as the validation report does. Here `auto` is given records that
//! say the backend is twice as fast as photonoxide's own at every size, in both families and
//! both forms, so every direct solve of a 2D or 3D FDFD solver goes to it; the backend is
//! wrapped to count its factorizations, which shows which cases solved through it. A case that
//! factorized nothing isn't listed. Each listed case is then run again with no records, on
//! photonoxide's own, and the two printed as a Markdown table: the values each reported, their
//! difference, and whether the backend's run is within the case's tolerance. It fails if one
//! isn't. The validation report itself (docs/validation.md) is photonoxide's own solvers'.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use photonoxide::Result;
use photonoxide::backend::auto::{self, Measured, Outcome as Ran};
use photonoxide::backend::{
    self, Analysis, Capabilities, Choice, DirectSolver, Factorization, Form, Matrix,
};
use photonoxide::validation::{self, Outcome};

/// The factorizations the backend made.
static FACTORIZATIONS: AtomicUsize = AtomicUsize::new(0);

/// A backend that counts its factorizations.
struct Counting(Arc<dyn DirectSolver>);

struct CountingAnalysis(Arc<dyn Analysis>);

impl DirectSolver for Counting {
    fn capabilities(&self) -> Capabilities {
        self.0.capabilities()
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
        FACTORIZATIONS.fetch_add(1, Ordering::Relaxed);
        self.0.factorize(matrix)
    }
}

/// Records in which `name` is twice as fast as photonoxide's own at every power of 2 of
/// unknowns, for both families and both forms, on this many threads.
fn records(name: &str) -> Vec<Measured> {
    let threads = rayon::current_num_threads();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let mut out = Vec::new();
    for family in ["fdfd2d", "fdfd3d"] {
        for form in [Form::General, Form::Symmetric] {
            for k in 0..=40 {
                for (backend, seconds) in [("photonoxide", 2.0), (name, 1.0)] {
                    out.push(Measured {
                        family: family.into(),
                        form,
                        unknowns: 1 << k,
                        backend: backend.into(),
                        threads,
                        seconds,
                        peak_bytes: None,
                        outcome: Ran::Accurate,
                        unix_seconds: now,
                    });
                }
            }
        }
    }
    out
}

/// A case's outcome and its seconds, or what its panic said.
fn run(case: &validation::Case) -> (std::result::Result<Outcome, String>, f64) {
    let start = Instant::now();
    let outcome = catch_unwind(AssertUnwindSafe(case.run)).map_err(|p| {
        p.downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_owned()))
            .unwrap_or_else(|| "a panic".into())
    });
    (outcome, start.elapsed().as_secs_f64())
}

fn number(x: f64) -> String {
    if x == 0.0 {
        "0".into()
    } else if (1e-3..1e5).contains(&x.abs()) {
        format!("{x:.6}")
    } else {
        format!("{x:.6e}")
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(name) = args.next() else {
        eprintln!("usage: validate <backend> [case prefix ...]");
        return ExitCode::FAILURE;
    };
    let prefixes: Vec<String> = args.collect();
    for probe in photonoxide_native::register_all() {
        eprintln!("{probe}");
    }
    let solver = match Choice::parse(&name).and_then(|c| backend::direct(&c)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let capabilities = solver.capabilities();
    if let Err(e) = backend::register(Arc::new(Counting(solver))) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    let cases: Vec<validation::Case> = validation::cases()
        .into_iter()
        .filter(|c| prefixes.is_empty() || prefixes.iter().any(|p| c.id.starts_with(p.as_str())))
        .collect();
    eprintln!(
        "{} {} in auto's place, {} cases, {} threads",
        capabilities.name,
        capabilities.version,
        cases.len(),
        rayon::current_num_threads()
    );
    let mut rows = Vec::new();
    let (mut unused, mut failed) = (0, 0);
    let total = Instant::now();
    for case in &cases {
        auto::configure(records(&name), None);
        FACTORIZATIONS.store(0, Ordering::Relaxed);
        let (theirs, their_seconds) = run(case);
        let factorizations = FACTORIZATIONS.swap(0, Ordering::Relaxed);
        if factorizations == 0 {
            unused += 1;
            eprintln!("{}: no direct solve ({their_seconds:.1} s)", case.id);
            continue;
        }
        auto::configure(Vec::new(), None);
        let (own, own_seconds) = run(case);
        if FACTORIZATIONS.load(Ordering::Relaxed) != 0 {
            eprintln!("{}: the backend was used without records", case.id);
            return ExitCode::FAILURE;
        }
        let row = match (&own, &theirs) {
            (Ok(own), Ok(theirs)) => {
                let difference = (theirs.measured - own.measured).abs();
                let passed = theirs.passed();
                if !passed {
                    failed += 1;
                }
                format!(
                    "| `{}` | {} | {} | {} | {} | {} | {} |",
                    case.id,
                    factorizations,
                    number(own.measured),
                    number(theirs.measured),
                    number(difference),
                    number(theirs.tolerance),
                    if passed { "yes" } else { "**no**" }
                )
            }
            _ => {
                failed += 1;
                let said = |r: &std::result::Result<Outcome, String>| match r {
                    Ok(o) => number(o.measured),
                    Err(e) => format!("panicked: {e}"),
                };
                format!(
                    "| `{}` | {} | {} | {} | | | **no** |",
                    case.id,
                    factorizations,
                    said(&own),
                    said(&theirs)
                )
            }
        };
        eprintln!("{row} ({own_seconds:.1} s, {their_seconds:.1} s)");
        rows.push(row);
    }
    println!(
        "{} {}, {} threads: {} of {} cases factorized through it, {} failed ({:.0} s)\n",
        capabilities.name,
        capabilities.version,
        rayon::current_num_threads(),
        rows.len(),
        cases.len(),
        failed,
        total.elapsed().as_secs_f64()
    );
    println!(
        "| Case | Factorizations | photonoxide | {} | Difference | Tolerance | Within |",
        capabilities.name
    );
    println!("|---|---|---|---|---|---|---|");
    for row in &rows {
        println!("{row}");
    }
    eprintln!("{unused} cases made no direct solve");
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
