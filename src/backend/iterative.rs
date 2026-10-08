//! Iterative backends: QMR run elsewhere (a GPU), on the same recurrences as photonoxide's own.
//!
//! photonoxide keeps what surrounds a run: the restarts after a breakdown of the Lanczos
//! process, and the tightened tolerance of QMR on a symmetric similar matrix
//! ([`crate::fdfd::IterativeSolver3d`]). A backend does one run, from x₀ = 0 to the tolerance
//! or a breakdown, as `fdfd::krylov` does it: R. W. Freund, N. M. Nachtigal, Numer. Math. 60,
//! 315 (1991), doi:10.1007/BF01385726, without look-ahead; on a complex symmetric matrix, R. W.
//! Freund, SIAM J. Sci. Stat. Comput. 13, 425 (1992), doi:10.1137/0913023.

use std::sync::{Arc, OnceLock, RwLock};

use num_complex::Complex64 as c64;

use super::{Capabilities, Choice, Listed, Matrix, OWN, check_name, invalid};
use crate::Result;
use crate::fdfd::{Convergence, Stopping};

/// How one run of QMR ended.
#[derive(Clone, Debug, PartialEq)]
pub enum QmrRun {
    /// At the tolerance: the solution, and how it went (the true residual at the end).
    Done(Vec<c64>, Convergence),
    /// At a (near-)breakdown, |w_nᵀ v_n| below 1e-14 for unit vectors: the iterate so far, the
    /// updated residual's history, and |w_nᵀ v_n|. The caller restarts from the iterate.
    Broken(Vec<c64>, Vec<f64>, f64),
}

/// A backend that runs QMR.
pub trait IterativeSolver: Send + Sync {
    /// What it is and can do ([`Capabilities::symmetric`]: it runs the complex symmetric QMR).
    fn capabilities(&self) -> Capabilities;

    /// One run of QMR on `matrix` from x₀ = 0 for `b`, to `stopping`'s relative residual
    /// (checked on the true residual) or a breakdown: the complex symmetric QMR, one product
    /// an iteration, for a [`super::Form::Symmetric`] matrix; the general one, with Aᵀ's
    /// products too, otherwise.
    ///
    /// # Errors
    ///
    /// The backend's, and [`crate::Error::InvalidValue`] if the tolerance isn't reached within
    /// the iterations allowed or the process ends short of it, as photonoxide's own QMR.
    fn qmr_run(&self, matrix: &Matrix<'_>, b: &[c64], stopping: Stopping) -> Result<QmrRun>;

    /// One run of the general QMR on A M⁻¹, M = LU from `ilu` (photonoxide's ILU(0) of
    /// `matrix`), from y₀ = 0 for `b`: the caller takes x = M⁻¹ y. Each iteration one product with
    /// A and one with Aᵀ, two triangular solves with M and two with Mᵀ.
    ///
    /// # Errors
    ///
    /// As [`IterativeSolver::qmr_run`]; by default, that the backend doesn't run it.
    fn qmr_run_ilu(
        &self,
        matrix: &Matrix<'_>,
        ilu: &IluFactors,
        b: &[c64],
        stopping: Stopping,
    ) -> Result<QmrRun> {
        let _ = (matrix, ilu, b, stopping);
        Err(invalid(format!(
            "{} doesn't run QMR with ILU(0)",
            self.capabilities().name
        )))
    }
}

/// ILU(0)'s factors as a backend takes them: L, unit lower triangular, without its diagonal,
/// and U, upper triangular, its diagonal included; each by rows, its columns ascending. M = LU
/// preconditions QMR from the right: QMR runs on A M⁻¹, as photonoxide's own does.
#[derive(Clone, Debug, PartialEq)]
pub struct IluFactors {
    pub(crate) n: usize,
    pub(crate) lower: (Vec<usize>, Vec<usize>, Vec<c64>),
    pub(crate) upper: (Vec<usize>, Vec<usize>, Vec<c64>),
}

impl IluFactors {
    /// The unknowns.
    pub fn n(&self) -> usize {
        self.n
    }

    /// L's rows below the diagonal: n + 1 row starts, each entry's column and value.
    pub fn lower(&self) -> (&[usize], &[usize], &[c64]) {
        (&self.lower.0, &self.lower.1, &self.lower.2)
    }

    /// U's rows, the diagonal first in each: n + 1 row starts, each entry's column and value.
    pub fn upper(&self) -> (&[usize], &[usize], &[c64]) {
        (&self.upper.0, &self.upper.1, &self.upper.2)
    }
}

#[derive(Clone)]
enum Entry {
    Available(Arc<dyn IterativeSolver>),
    Unavailable { name: String, reason: String },
}

impl Entry {
    fn name(&self) -> String {
        match self {
            Entry::Available(solver) => solver.capabilities().name,
            Entry::Unavailable { name, .. } => name.clone(),
        }
    }
}

fn registry() -> &'static RwLock<Vec<Entry>> {
    static REGISTRY: OnceLock<RwLock<Vec<Entry>>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(Vec::new()))
}

fn poisoned<T>(_: T) -> crate::Error {
    invalid("the registry was left locked by a thread that panicked")
}

fn place(name: &str, entry: Entry) -> Result<()> {
    check_name(name)?;
    if matches!(name, "auto" | OWN) {
        return Err(invalid(format!("{name:?} is photonoxide's own name")));
    }
    let mut entries = registry().write().map_err(poisoned)?;
    match entries.iter_mut().find(|e| e.name() == name) {
        Some(existing) => *existing = entry,
        None => entries.push(entry),
    }
    Ok(())
}

/// Registers an iterative solver under its capabilities' name, in place of one already
/// registered under it.
///
/// # Errors
///
/// [`crate::Error::InvalidValue`] for a name that isn't lower-case letters, digits, `-` and
/// `_`, for `auto` and `photonoxide`, and for a solver that isn't complex.
pub fn register_iterative(solver: Arc<dyn IterativeSolver>) -> Result<()> {
    let capabilities = solver.capabilities();
    if !capabilities.complex {
        return Err(invalid(format!(
            "{} doesn't solve complex systems, which photonoxide's are",
            capabilities.name
        )));
    }
    place(&capabilities.name, Entry::Available(solver))
}

/// Registers an iterative backend that can't be had, with the reason.
///
/// # Errors
///
/// As [`register_iterative`], for the name.
pub fn register_iterative_unavailable(name: &str, reason: impl Into<String>) -> Result<()> {
    place(
        name,
        Entry::Unavailable {
            name: name.to_owned(),
            reason: reason.into(),
        },
    )
}

/// Every iterative backend registered, the unavailable ones among them. photonoxide's own QMR
/// isn't one: it is what `auto` and `photonoxide` choose.
///
/// # Errors
///
/// [`crate::Error::InvalidValue`] if a thread panicked while registering.
pub fn iterative_solvers() -> Result<Vec<Listed>> {
    let entries = registry().read().map_err(poisoned)?;
    Ok(entries
        .iter()
        .map(|e| match e {
            Entry::Available(solver) => {
                let capabilities = solver.capabilities();
                Listed {
                    name: capabilities.name.clone(),
                    capabilities: Some(capabilities),
                    unavailable: None,
                }
            }
            Entry::Unavailable { name, reason } => Listed {
                name: name.clone(),
                capabilities: None,
                unavailable: Some(reason.clone()),
            },
        })
        .collect())
}

/// The iterative backend of a choice: `None` for `auto` and `photonoxide`, whose QMR is
/// photonoxide's own.
///
/// # Errors
///
/// [`crate::Error::InvalidValue`] for a name nothing is registered under, and for a backend
/// registered as unavailable, with its reason. Never another backend in its place.
pub fn iterative(choice: &Choice) -> Result<Option<Arc<dyn IterativeSolver>>> {
    let name = match choice {
        Choice::Auto | Choice::Photonoxide => return Ok(None),
        Choice::Named(name) => name,
    };
    let entries = registry().read().map_err(poisoned)?;
    match entries.iter().find(|e| e.name() == *name) {
        Some(Entry::Available(solver)) => Ok(Some(solver.clone())),
        Some(Entry::Unavailable { reason, .. }) => Err(invalid(format!(
            "the iterative solver {name:?} isn't available: {reason}"
        ))),
        None => {
            let known: Vec<String> = entries.iter().map(Entry::name).collect();
            Err(invalid(format!(
                "no iterative solver is registered as {name:?}: there are auto, photonoxide{}",
                known.iter().map(|k| format!(", {k}")).collect::<String>()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::Form;

    /// A backend that answers every run with zeros, for the registry's tests.
    struct Zeros;

    impl IterativeSolver for Zeros {
        fn capabilities(&self) -> Capabilities {
            Capabilities::new("zeros-for-tests", "1", "MIT OR Apache-2.0")
        }

        fn qmr_run(&self, matrix: &Matrix<'_>, _: &[c64], _: Stopping) -> Result<QmrRun> {
            Ok(QmrRun::Broken(
                vec![c64::new(0.0, 0.0); matrix.n()],
                Vec::new(),
                0.0,
            ))
        }
    }

    #[test]
    fn auto_and_photonoxide_are_photonoxides_own_qmr() {
        assert!(iterative(&Choice::Auto).unwrap().is_none());
        assert!(iterative(&Choice::Photonoxide).unwrap().is_none());
    }

    #[test]
    fn a_backend_is_found_by_name_and_an_unavailable_one_says_why() {
        register_iterative(Arc::new(Zeros)).unwrap();
        let solver = iterative(&Choice::Named("zeros-for-tests".into()))
            .unwrap()
            .unwrap();
        let (starts, rows, values) = (vec![0, 1], vec![0], vec![c64::new(1.0, 0.0)]);
        let m = Matrix::new(1, &starts, &rows, &values, Form::General).unwrap();
        let run = solver.qmr_run(&m, &[c64::new(1.0, 0.0)], Stopping::default());
        assert!(matches!(run, Ok(QmrRun::Broken(..))));
        assert!(
            iterative_solvers()
                .unwrap()
                .iter()
                .any(|l| l.name == "zeros-for-tests")
        );

        register_iterative_unavailable("missing-for-tests", "not installed").unwrap();
        let e = iterative(&Choice::Named("missing-for-tests".into()))
            .err()
            .unwrap();
        assert!(e.to_string().contains("not installed"), "{e}");
        let e = iterative(&Choice::Named("nothing-by-this-name".into()))
            .err()
            .unwrap();
        assert!(e.to_string().contains("no iterative solver"), "{e}");
    }

    #[test]
    fn photonoxides_own_names_are_refused() {
        assert!(register_iterative_unavailable("photonoxide", "x").is_err());
        assert!(register_iterative_unavailable("auto", "x").is_err());
    }
}
