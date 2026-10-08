//! The check a backend passes before it is offered: a small complex system solved and compared
//! with photonoxide's own answer.

use std::sync::Arc;

use num_complex::Complex64 as c64;
use photonoxide::Result;
use photonoxide::backend::{self, Choice, DirectSolver, Form, IterativeSolver, Matrix, QmrRun};
use photonoxide::fdfd::Stopping;

use crate::library::error;

/// The largest difference allowed from photonoxide's answer, relative to its largest entry.
pub const TOLERANCE: f64 = 1e-10;

const N: usize = 10;

/// A small complex matrix by columns: tridiagonal with corners, complex symmetric for
/// [`Form::Symmetric`], not for [`Form::General`].
fn system(form: Form) -> (Vec<usize>, Vec<usize>, Vec<c64>) {
    let entry = |i: usize, j: usize| -> Option<c64> {
        let lower = c64::new(-1.0, if form == Form::Symmetric { 0.3 } else { -0.2 });
        match (i, j) {
            _ if i == j => Some(c64::new(4.0, (i + 1) as f64 / N as f64)),
            _ if i == j + 1 => Some(lower),
            _ if j == i + 1 => Some(c64::new(-1.0, 0.3)),
            (0, j) if j == N - 1 => Some(c64::new(0.0, 0.5)),
            (i, 0) if i == N - 1 => Some(c64::new(0.0, 0.5)),
            _ => None,
        }
    };
    let (mut starts, mut rows, mut values) = (vec![0], Vec::new(), Vec::new());
    for j in 0..N {
        for i in 0..N {
            if let Some(v) = entry(i, j) {
                rows.push(i);
                values.push(v);
            }
        }
        starts.push(rows.len());
    }
    (starts, rows, values)
}

/// The largest relative difference between `solver`'s solutions and photonoxide's, over the
/// forms and solves it declares.
///
/// # Errors
///
/// The backend's own errors, and when the difference is larger than [`TOLERANCE`].
pub fn smoke_test(solver: &dyn DirectSolver) -> Result<f64> {
    let capabilities = solver.capabilities();
    let own = backend::direct(&Choice::Photonoxide)?;
    let b: Vec<c64> = (0..N)
        .map(|k| c64::new((k + 1) as f64, (N - k) as f64))
        .collect();
    let mut forms = vec![Form::General];
    if capabilities.symmetric {
        forms.push(Form::Symmetric);
    }
    let mut largest = 0.0f64;
    for form in forms {
        let (starts, rows, values) = system(form);
        let matrix = Matrix::new(N, &starts, &rows, &values, form)?;
        let solve = |s: &dyn DirectSolver, transpose: bool| -> Result<Vec<c64>> {
            let analysis = s.analyse(&matrix)?.ok_or_else(|| {
                error(format!(
                    "{} refused the smoke test's {form:?} matrix",
                    s.capabilities().name
                ))
            })?;
            let factors = analysis.factorize(&matrix)?;
            if transpose {
                factors.solve_transpose(&b)
            } else {
                factors.solve(&b)
            }
        };
        for transpose in [false, true] {
            if transpose && !capabilities.transpose {
                continue;
            }
            let theirs = solve(solver, transpose)?;
            let ours = solve(own.as_ref(), transpose)?;
            if theirs.len() != N {
                return Err(error(format!(
                    "{} returned {} values for {N} unknowns",
                    capabilities.name,
                    theirs.len()
                )));
            }
            let scale = ours.iter().map(|v| v.norm()).fold(0.0, f64::max);
            let difference = theirs
                .iter()
                .zip(&ours)
                .map(|(a, b)| (a - b).norm())
                .fold(0.0, f64::max)
                / scale;
            // NaN fails too
            if difference.is_nan() || difference > TOLERANCE {
                return Err(error(format!(
                    "{}'s {form:?} solve{} differs from photonoxide's by {difference:.1e}, more \
                     than {TOLERANCE:.0e}",
                    capabilities.name,
                    if transpose { " with the transpose" } else { "" }
                )));
            }
            largest = largest.max(difference);
        }
    }
    Ok(largest)
}

/// Registers `solver` if it passes [`smoke_test`], or registers its name as unavailable with
/// the reason. Whether it was offered.
///
/// # Errors
///
/// The registry's, for a name it doesn't take.
pub fn offer(solver: Arc<dyn DirectSolver>) -> Result<bool> {
    match smoke_test(solver.as_ref()) {
        Ok(_) => backend::register(solver).map(|()| true),
        Err(e) => {
            let name = solver.capabilities().name;
            backend::register_unavailable(&name, format!("its smoke test failed: {e}"))
                .map(|()| false)
        }
    }
}

/// The largest relative difference between `solver`'s QMR, run to 1e-12, and photonoxide's
/// direct solve of the same small systems, in both forms if it declares the symmetric one.
///
/// # Errors
///
/// The backend's own errors, a run that doesn't reach its tolerance, and a difference larger
/// than 1e-9.
pub fn smoke_test_iterative(solver: &dyn IterativeSolver) -> Result<f64> {
    let capabilities = solver.capabilities();
    let own = backend::direct(&Choice::Photonoxide)?;
    let b: Vec<c64> = (0..N)
        .map(|k| c64::new((k + 1) as f64, (N - k) as f64))
        .collect();
    let mut forms = vec![Form::General];
    if capabilities.symmetric {
        forms.push(Form::Symmetric);
    }
    let mut largest = 0.0f64;
    for form in forms {
        let (starts, rows, values) = system(form);
        let matrix = Matrix::new(N, &starts, &rows, &values, form)?;
        let stopping = Stopping {
            tolerance: 1e-12,
            max_iterations: 10 * N,
        };
        let theirs = match solver.qmr_run(&matrix, &b, stopping)? {
            QmrRun::Done(x, _) => x,
            QmrRun::Broken(_, history, d) => {
                return Err(error(format!(
                    "{}'s QMR broke down at step {} (|wᵀv| = {d:e})",
                    capabilities.name,
                    history.len()
                )));
            }
        };
        let general = Matrix::new(N, &starts, &rows, &values, Form::General)?;
        let ours = own
            .analyse(&general)?
            .ok_or_else(|| error("photonoxide's own solver refused the smoke test's matrix"))?
            .factorize(&general)?
            .solve(&b)?;
        let scale = ours.iter().map(|v| v.norm()).fold(0.0, f64::max);
        let difference = theirs
            .iter()
            .zip(&ours)
            .map(|(a, b)| (a - b).norm())
            .fold(0.0, f64::max)
            / scale;
        // NaN fails too
        if difference.is_nan() || difference > 1e-9 {
            return Err(error(format!(
                "{}'s {form:?} QMR differs from photonoxide's solve by {difference:.1e}",
                capabilities.name
            )));
        }
        largest = largest.max(difference);
    }
    Ok(largest)
}

/// Registers an iterative `solver` if it passes [`smoke_test_iterative`], or its name as
/// unavailable with the reason. Whether it was offered.
///
/// # Errors
///
/// The registry's, for a name it doesn't take.
pub fn offer_iterative(solver: std::sync::Arc<dyn IterativeSolver>) -> Result<bool> {
    match smoke_test_iterative(solver.as_ref()) {
        Ok(_) => backend::register_iterative(solver).map(|()| true),
        Err(e) => {
            let name = solver.capabilities().name;
            backend::register_iterative_unavailable(&name, format!("its smoke test failed: {e}"))
                .map(|()| false)
        }
    }
}
