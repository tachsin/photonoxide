//! Solver backends: the one place photonoxide's solvers ask for a sparse direct solve, so that
//! another library can stand in for photonoxide's own.
//!
//! A backend is a [`DirectSolver`]. It analyses a matrix's structure ([`DirectSolver::analyse`]),
//! factorizes values on an [`Analysis`] (the same one for every matrix of a sweep, as
//! [`crate::fdfd::Solver2d::reuse`] does), and a [`Factorization`] solves with the matrix and
//! with its transpose (the adjoints and QMR's preconditioner need the transpose). Each backend
//! declares its [`Capabilities`]: the forms it takes, how its threads are set, whether it gives
//! the same bits on every run, and its library's name, version and licence.
//!
//! Two backends are always registered:
//!
//! - `photonoxide`: the multifrontal LU and L D Lᵀ of this crate, the default and the
//!   reference. It gives the same bits on any number of threads.
//! - `faer`: faer's sparse LU, the baseline photonoxide's own was measured against
//!   (docs/baselines.md). General matrices only.
//!
//! Others are registered at run time by whoever has them ([`register`]); photonoxide itself
//! links no external library. A library that was looked for and not found, or that failed its
//! check, is registered as unavailable with the reason ([`register_unavailable`]), so that
//! asking for it by name says why it can't be had.
//!
//! A solver takes a [`Choice`]: `auto` (photonoxide's own), `photonoxide`, or a backend's name.
//! A named backend that isn't available is an error, never a silent fallback.
//!
//! Dense kernels for the multifrontal fronts, iterative solvers and eigensolvers will get
//! traits of their own beside these when a backend for them exists; none is sketched here
//! ahead of its first implementation.

use std::fmt;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Instant;

use faer::Mat;
use faer::linalg::solvers::Solve;
use faer::sparse::{SparseColMat, SparseColMatRef, SymbolicSparseColMatRef, Triplet};
use num_complex::Complex64 as c64;

use crate::sparse;
use crate::{Error, Result};

mod iterative;
pub use iterative::{
    IluFactors, IterativeSolver, MultigridCycle, MultigridLevel, QmrRun, RowMatrix, iterative,
    iterative_solvers, register_iterative, register_iterative_unavailable,
};

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("backend", reason)
}

/// The form a matrix is factorized in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// Any square matrix: an LU.
    General,
    /// A complex symmetric matrix, A = Aᵀ without conjugation: an L D Lᵀ, half an LU's storage.
    Symmetric,
}

/// A square sparse complex matrix by columns, as a backend is given it: column j's entries are
/// `row_indices[column_starts[j]..column_starts[j + 1]]`, ascending and without repeats, with
/// their `values`. A symmetric matrix is given whole, both triangles.
#[derive(Clone, Copy, Debug)]
pub struct Matrix<'a> {
    n: usize,
    column_starts: &'a [usize],
    row_indices: &'a [usize],
    values: &'a [c64],
    positions: Option<&'a [[f64; 3]]>,
    form: Form,
}

impl<'a> Matrix<'a> {
    /// The n × n matrix of these columns, to be factorized as `form`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless there are n + 1 column starts from 0 up to the number of
    /// entries, never decreasing, and each column's rows are below n, ascending and distinct.
    pub fn new(
        n: usize,
        column_starts: &'a [usize],
        row_indices: &'a [usize],
        values: &'a [c64],
        form: Form,
    ) -> Result<Matrix<'a>> {
        if column_starts.len() != n + 1
            || column_starts[0] != 0
            || column_starts[n] != row_indices.len()
            || row_indices.len() != values.len()
        {
            return Err(invalid(format!(
                "a matrix of {n} columns needs {} column starts from 0 to its {} entries",
                n + 1,
                values.len()
            )));
        }
        for j in 0..n {
            let (from, to) = (column_starts[j], column_starts[j + 1]);
            if from > to || to > row_indices.len() {
                return Err(invalid(format!(
                    "column {j}'s entries run from {from} to {to}"
                )));
            }
            let rows = &row_indices[from..to];
            if rows.iter().any(|&r| r >= n) || rows.windows(2).any(|p| p[0] >= p[1]) {
                return Err(invalid(format!(
                    "column {j}'s rows must be below {n}, ascending and distinct"
                )));
            }
        }
        Ok(Matrix {
            n,
            column_starts,
            row_indices,
            values,
            positions: None,
            form,
        })
    }

    /// With each unknown's place on its grid (any units), which a backend may order by:
    /// photonoxide's nested dissection cuts the grid by them.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless there is one position per unknown.
    pub fn with_positions(mut self, positions: &'a [[f64; 3]]) -> Result<Matrix<'a>> {
        if positions.len() != self.n {
            return Err(invalid(format!(
                "{} positions for {} unknowns",
                positions.len(),
                self.n
            )));
        }
        self.positions = Some(positions);
        Ok(self)
    }

    /// Its unknowns.
    pub fn n(&self) -> usize {
        self.n
    }

    /// Where each column's entries start, and after the last where they end.
    pub fn column_starts(&self) -> &'a [usize] {
        self.column_starts
    }

    /// The entries' rows, column after column.
    pub fn row_indices(&self) -> &'a [usize] {
        self.row_indices
    }

    /// The entries' values, column after column.
    pub fn values(&self) -> &'a [c64] {
        self.values
    }

    /// The unknowns' places on their grid, if given.
    pub fn positions(&self) -> Option<&'a [[f64; 3]]> {
        self.positions
    }

    /// The form to factorize it in.
    pub fn form(&self) -> Form {
        self.form
    }

    /// The matrix as faer's, without copying.
    fn as_faer(&self) -> SparseColMatRef<'a, usize, c64> {
        // checked in `new`
        let symbolic = SymbolicSparseColMatRef::new_checked(
            self.n,
            self.n,
            self.column_starts,
            None,
            self.row_indices,
        );
        SparseColMatRef::new(symbolic, self.values)
    }
}

/// How a backend's threads are set.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Threads {
    /// One thread.
    One,
    /// rayon's: the pool the solve is called in, or `RAYON_NUM_THREADS`.
    Rayon,
    /// The library's own, set as these words say (an environment variable, a call).
    Library(String),
}

/// What a backend is and can do.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Capabilities {
    /// The name it is chosen by: lower-case letters, digits, `-` and `_`.
    pub name: String,
    /// The library's version.
    pub version: String,
    /// The library's licence, by name.
    pub licence: String,
    /// Whether it factorizes complex matrices. photonoxide's are complex.
    pub complex: bool,
    /// Whether it takes [`Form::Symmetric`].
    pub symmetric: bool,
    /// Whether it solves with the transpose.
    pub transpose: bool,
    /// How its threads are set.
    pub threads: Threads,
    /// Whether a solve gives the same bits on every run and any number of threads.
    pub deterministic: bool,
}

impl Capabilities {
    /// A backend of this name, version and licence, that takes general complex matrices and
    /// nothing else on one thread, not deterministic: set what it can do beyond that.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        licence: impl Into<String>,
    ) -> Capabilities {
        Capabilities {
            name: name.into(),
            version: version.into(),
            licence: licence.into(),
            complex: true,
            symmetric: false,
            transpose: false,
            threads: Threads::One,
            deterministic: false,
        }
    }
}

/// What a factorization took.
#[derive(Clone, Debug, PartialEq, Default)]
#[non_exhaustive]
pub struct Report {
    /// The factors' entries, if the library says.
    pub factor_entries: Option<usize>,
    /// The factorization's peak memory in bytes, if the library says.
    pub peak_memory_bytes: Option<u64>,
    /// The pivots too small to divide by, replaced: 0 for a factorization to trust as it is.
    pub perturbed_pivots: usize,
    /// The seconds the analysis took (the one this factorization used, perhaps an earlier
    /// matrix's).
    pub analysis_seconds: f64,
    /// The seconds the numerical factorization took.
    pub factorization_seconds: f64,
}

/// A sparse direct solver.
pub trait DirectSolver: Send + Sync {
    /// What it is and can do.
    fn capabilities(&self) -> Capabilities;

    /// The analysis of `matrix`'s structure in its form: the ordering and the symbolic
    /// factorization, for this matrix and any other of the same structure. `None` if this
    /// matrix can't be factorized in that form (a matrix asked for as [`Form::Symmetric`] that
    /// isn't, or whose pivots can't stay on its diagonal): the caller then asks for
    /// [`Form::General`].
    ///
    /// # Errors
    ///
    /// The backend's, as photonoxide's [`Error`]: a form it doesn't take, a structurally
    /// singular matrix, the library's own failure.
    fn analyse(&self, matrix: &Matrix<'_>) -> Result<Option<Arc<dyn Analysis>>>;
}

/// A structure's analysis: factorizes any matrix of that structure.
pub trait Analysis: Send + Sync {
    /// The form it factorizes.
    fn form(&self) -> Form;

    /// The factors of `matrix`, whose structure is the analysed one.
    ///
    /// # Errors
    ///
    /// The backend's, as photonoxide's [`Error`]: another structure, a singular matrix, the
    /// library's own failure.
    fn factorize(&self, matrix: &Matrix<'_>) -> Result<Box<dyn Factorization>>;
}

/// A matrix's factors.
pub trait Factorization: Send + Sync {
    /// x with A x = `b`.
    ///
    /// # Errors
    ///
    /// The backend's, as photonoxide's [`Error`].
    fn solve(&self, b: &[c64]) -> Result<Vec<c64>>;

    /// x with Aᵀ x = `b` (the transpose, not conjugated).
    ///
    /// # Errors
    ///
    /// The backend's, as photonoxide's [`Error`]; a backend without transpose solves says so.
    fn solve_transpose(&self, b: &[c64]) -> Result<Vec<c64>>;

    /// What the factorization took.
    fn report(&self) -> Report;
}

/// Which backend a solver uses.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Choice {
    /// photonoxide's choice: its own solver.
    #[default]
    Auto,
    /// photonoxide's own solver.
    Photonoxide,
    /// The backend registered under this name.
    Named(String),
}

impl Choice {
    /// The choice a job file or a setting names: `auto`, `photonoxide` or a backend's name.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the name is lower-case letters, digits, `-` and `_`.
    pub fn parse(name: &str) -> Result<Choice> {
        check_name(name)?;
        Ok(match name {
            "auto" => Choice::Auto,
            OWN => Choice::Photonoxide,
            other => Choice::Named(other.to_owned()),
        })
    }

    /// The name it is written as.
    pub fn name(&self) -> &str {
        match self {
            Choice::Auto => "auto",
            Choice::Photonoxide => OWN,
            Choice::Named(name) => name,
        }
    }
}

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

const OWN: &str = "photonoxide";
const FAER: &str = "faer";

fn check_name(name: &str) -> Result<()> {
    let valid = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
    if name.is_empty() || !name.chars().all(valid) {
        return Err(invalid(format!(
            "a backend's name is lower-case letters, digits, '-' and '_', got {name:?}"
        )));
    }
    Ok(())
}

/// A backend in the registry: there to be used, or known and not to be had.
#[derive(Clone)]
enum Entry {
    Available(Arc<dyn DirectSolver>),
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
    REGISTRY.get_or_init(|| {
        RwLock::new(vec![
            Entry::Available(Arc::new(Photonoxide)),
            Entry::Available(Arc::new(Faer)),
        ])
    })
}

fn poisoned<T>(_: T) -> Error {
    invalid("the registry was left locked by a thread that panicked")
}

/// A backend as the registry lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    /// Its name.
    pub name: String,
    /// What it can do, if it is there to be used.
    pub capabilities: Option<Capabilities>,
    /// Why it can't be had, if it isn't.
    pub unavailable: Option<String>,
}

/// Every direct solver registered, photonoxide's own first, the unavailable ones among them.
///
/// # Errors
///
/// [`Error::InvalidValue`] if a thread panicked while registering.
pub fn direct_solvers() -> Result<Vec<Listed>> {
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

fn place(name: &str, entry: Entry) -> Result<()> {
    check_name(name)?;
    if matches!(name, "auto" | OWN | FAER) {
        return Err(invalid(format!("{name:?} is photonoxide's own name")));
    }
    let mut entries = registry().write().map_err(poisoned)?;
    match entries.iter_mut().find(|e| e.name() == name) {
        // found after having been missing, or missing after a failed check: the latest stands
        Some(existing) => *existing = entry,
        None => entries.push(entry),
    }
    Ok(())
}

/// Registers a direct solver under its capabilities' name, in place of one already registered
/// under it. It must factorize complex matrices.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a name that isn't lower-case letters, digits, `-` and `_`, for
/// `auto`, `photonoxide` or `faer`, and for a solver that isn't complex.
pub fn register(solver: Arc<dyn DirectSolver>) -> Result<()> {
    let capabilities = solver.capabilities();
    if !capabilities.complex {
        return Err(invalid(format!(
            "{} doesn't factorize complex matrices, which photonoxide's are",
            capabilities.name
        )));
    }
    place(&capabilities.name, Entry::Available(solver))
}

/// Registers a backend that can't be had, with the reason (not installed, a version too old,
/// a failed check), in place of one already registered under the name: choosing it then fails
/// with that reason.
///
/// # Errors
///
/// As [`register`], for the name.
pub fn register_unavailable(name: &str, reason: impl Into<String>) -> Result<()> {
    place(
        name,
        Entry::Unavailable {
            name: name.to_owned(),
            reason: reason.into(),
        },
    )
}

/// The direct solver of a choice: photonoxide's own for `auto`.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a name nothing is registered under, and for a backend
/// registered as unavailable, with its reason. Never another backend in its place.
pub fn direct(choice: &Choice) -> Result<Arc<dyn DirectSolver>> {
    let name = match choice {
        Choice::Auto | Choice::Photonoxide => OWN,
        Choice::Named(name) => name,
    };
    let entries = registry().read().map_err(poisoned)?;
    match entries.iter().find(|e| e.name() == name) {
        Some(Entry::Available(solver)) => Ok(solver.clone()),
        Some(Entry::Unavailable { reason, .. }) => Err(invalid(format!(
            "the direct solver {name:?} isn't available: {reason}"
        ))),
        None => {
            let known: Vec<String> = entries.iter().map(Entry::name).collect();
            Err(invalid(format!(
                "no direct solver is registered as {name:?}: there are auto, {}",
                known.join(", ")
            )))
        }
    }
}

/// photonoxide's own: the multifrontal LU and L D Lᵀ of [`crate::sparse`].
struct Photonoxide;

struct OwnAnalysis {
    analysis: Arc<sparse::Analysis>,
    seconds: f64,
}

struct OwnFactors {
    factors: sparse::Multifrontal,
    report: Report,
}

impl DirectSolver for Photonoxide {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            symmetric: true,
            transpose: true,
            threads: Threads::Rayon,
            deterministic: true,
            ..Capabilities::new(OWN, env!("CARGO_PKG_VERSION"), "MIT OR Apache-2.0")
        }
    }

    fn analyse(&self, matrix: &Matrix<'_>) -> Result<Option<Arc<dyn Analysis>>> {
        let clock = Instant::now();
        let a = matrix.as_faer();
        let analysis = match matrix.form() {
            Form::General => Some(sparse::Analysis::new(a, matrix.positions())?),
            Form::Symmetric => sparse::Analysis::new_symmetric(a, matrix.positions())?,
        };
        Ok(analysis.map(|analysis| {
            Arc::new(OwnAnalysis {
                analysis: Arc::new(analysis),
                seconds: clock.elapsed().as_secs_f64(),
            }) as Arc<dyn Analysis>
        }))
    }
}

impl Analysis for OwnAnalysis {
    fn form(&self) -> Form {
        if self.analysis.symmetric() {
            Form::Symmetric
        } else {
            Form::General
        }
    }

    fn factorize(&self, matrix: &Matrix<'_>) -> Result<Box<dyn Factorization>> {
        let clock = Instant::now();
        let factors = sparse::Multifrontal::new(self.analysis.clone(), matrix.as_faer())?;
        let report = Report {
            factor_entries: Some(self.analysis.factor_entries()),
            peak_memory_bytes: None,
            perturbed_pivots: factors.perturbed(),
            analysis_seconds: self.seconds,
            factorization_seconds: clock.elapsed().as_secs_f64(),
        };
        Ok(Box::new(OwnFactors { factors, report }))
    }
}

impl Factorization for OwnFactors {
    fn solve(&self, b: &[c64]) -> Result<Vec<c64>> {
        check_rhs(b, self.factors.analysis().n())?;
        Ok(self.factors.solve(b))
    }

    fn solve_transpose(&self, b: &[c64]) -> Result<Vec<c64>> {
        check_rhs(b, self.factors.analysis().n())?;
        Ok(self.factors.solve_transpose(b))
    }

    fn report(&self) -> Report {
        self.report.clone()
    }
}

fn check_rhs(b: &[c64], n: usize) -> Result<()> {
    if b.len() != n {
        return Err(invalid(format!(
            "a right-hand side of {} for a matrix of {n} unknowns",
            b.len()
        )));
    }
    Ok(())
}

/// faer's sparse LU, in its own ordering (COLAMD): the baseline. It analyses and factorizes
/// together, so its analysis only remembers the size.
struct Faer;

struct FaerAnalysis {
    n: usize,
}

struct FaerFactors {
    n: usize,
    lu: faer::sparse::linalg::solvers::Lu<usize, c64>,
    seconds: f64,
}

impl DirectSolver for Faer {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            transpose: true,
            threads: Threads::Rayon,
            ..Capabilities::new(FAER, "0.24", "MIT")
        }
    }

    fn analyse(&self, matrix: &Matrix<'_>) -> Result<Option<Arc<dyn Analysis>>> {
        match matrix.form() {
            Form::General => Ok(Some(Arc::new(FaerAnalysis { n: matrix.n() }))),
            Form::Symmetric => Err(invalid("faer's sparse LU takes general matrices only")),
        }
    }
}

impl Analysis for FaerAnalysis {
    fn form(&self) -> Form {
        Form::General
    }

    fn factorize(&self, matrix: &Matrix<'_>) -> Result<Box<dyn Factorization>> {
        if matrix.n() != self.n {
            return Err(invalid(format!(
                "a matrix of {} unknowns, analysed for {}",
                matrix.n(),
                self.n
            )));
        }
        let clock = Instant::now();
        let n = self.n;
        let triplets: Vec<Triplet<usize, usize, c64>> = (0..n)
            .flat_map(|j| {
                let (from, to) = (matrix.column_starts()[j], matrix.column_starts()[j + 1]);
                (from..to)
                    .map(move |e| Triplet::new(matrix.row_indices()[e], j, matrix.values()[e]))
            })
            .collect();
        let a = SparseColMat::<usize, c64>::try_new_from_triplets(n, n, &triplets)
            .map_err(|e| invalid(format!("faer can't assemble the matrix: {e:?}")))?;
        let lu = a
            .sp_lu()
            .map_err(|e| invalid(format!("faer's LU failed: {e:?}")))?;
        Ok(Box::new(FaerFactors {
            n,
            lu,
            seconds: clock.elapsed().as_secs_f64(),
        }))
    }
}

impl Factorization for FaerFactors {
    fn solve(&self, b: &[c64]) -> Result<Vec<c64>> {
        check_rhs(b, self.n)?;
        let mut rhs = Mat::<c64>::from_fn(self.n, 1, |i, _| b[i]);
        self.lu.solve_in_place(rhs.as_mut());
        Ok((0..self.n).map(|i| rhs[(i, 0)]).collect())
    }

    fn solve_transpose(&self, b: &[c64]) -> Result<Vec<c64>> {
        check_rhs(b, self.n)?;
        let mut rhs = Mat::<c64>::from_fn(self.n, 1, |i, _| b[i]);
        self.lu.solve_transpose_in_place(rhs.as_mut());
        Ok((0..self.n).map(|i| rhs[(i, 0)]).collect())
    }

    fn report(&self) -> Report {
        Report {
            factorization_seconds: self.seconds,
            ..Report::default()
        }
    }
}

/// A matrix of triplets by columns, its repeated entries summed: what [`Matrix`] borrows.
pub(crate) struct Columns {
    n: usize,
    matrix: SparseColMat<usize, c64>,
}

impl Columns {
    /// The n × n matrix of `triplets`, assembled as faer does (so that photonoxide's own
    /// backend sees the matrix it always has, to the bit).
    pub(crate) fn new(n: usize, triplets: &[Triplet<usize, usize, c64>]) -> Result<Columns> {
        let matrix = SparseColMat::<usize, c64>::try_new_from_triplets(n, n, triplets)
            .map_err(|e| invalid(format!("can't assemble the matrix: {e:?}")))?;
        Ok(Columns { n, matrix })
    }

    /// The matrix for a backend, in `form`, with the unknowns' `positions` if known.
    pub(crate) fn matrix<'a>(
        &'a self,
        form: Form,
        positions: Option<&'a [[f64; 3]]>,
    ) -> Result<Matrix<'a>> {
        let a = self.matrix.as_ref();
        let matrix = Matrix::new(
            self.n,
            a.symbolic().col_ptr(),
            a.symbolic().row_idx(),
            a.val(),
            form,
        )?;
        match positions {
            Some(p) => matrix.with_positions(p),
            None => Ok(matrix),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// photonoxide's own backend under another name, counting what is asked of it: the proof
    /// that a solver went through the registry.
    #[derive(Default)]
    pub(crate) struct Recording {
        name: String,
        pub(crate) analyses: Arc<AtomicUsize>,
        pub(crate) factorizations: Arc<AtomicUsize>,
        pub(crate) solves: Arc<AtomicUsize>,
        pub(crate) transpose_solves: Arc<AtomicUsize>,
    }

    impl Recording {
        /// Registered under `name`.
        pub(crate) fn register(name: &str) -> Arc<Recording> {
            let recording = Arc::new(Recording {
                name: name.to_owned(),
                ..Recording::default()
            });
            register(recording.clone()).unwrap();
            recording
        }

        pub(crate) fn counts(&self) -> [usize; 4] {
            [
                self.analyses.load(Ordering::SeqCst),
                self.factorizations.load(Ordering::SeqCst),
                self.solves.load(Ordering::SeqCst),
                self.transpose_solves.load(Ordering::SeqCst),
            ]
        }
    }

    struct RecordingAnalysis {
        inner: Arc<dyn Analysis>,
        factorizations: Arc<AtomicUsize>,
        solves: Arc<AtomicUsize>,
        transpose_solves: Arc<AtomicUsize>,
    }

    struct RecordingFactors {
        inner: Box<dyn Factorization>,
        solves: Arc<AtomicUsize>,
        transpose_solves: Arc<AtomicUsize>,
    }

    impl DirectSolver for Recording {
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                name: self.name.clone(),
                ..Photonoxide.capabilities()
            }
        }

        fn analyse(&self, matrix: &Matrix<'_>) -> Result<Option<Arc<dyn Analysis>>> {
            self.analyses.fetch_add(1, Ordering::SeqCst);
            Ok(Photonoxide.analyse(matrix)?.map(|inner| {
                Arc::new(RecordingAnalysis {
                    inner,
                    factorizations: self.factorizations.clone(),
                    solves: self.solves.clone(),
                    transpose_solves: self.transpose_solves.clone(),
                }) as Arc<dyn Analysis>
            }))
        }
    }

    impl Analysis for RecordingAnalysis {
        fn form(&self) -> Form {
            self.inner.form()
        }

        fn factorize(&self, matrix: &Matrix<'_>) -> Result<Box<dyn Factorization>> {
            self.factorizations.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(RecordingFactors {
                inner: self.inner.factorize(matrix)?,
                solves: self.solves.clone(),
                transpose_solves: self.transpose_solves.clone(),
            }))
        }
    }

    impl Factorization for RecordingFactors {
        fn solve(&self, b: &[c64]) -> Result<Vec<c64>> {
            self.solves.fetch_add(1, Ordering::SeqCst);
            self.inner.solve(b)
        }

        fn solve_transpose(&self, b: &[c64]) -> Result<Vec<c64>> {
            self.transpose_solves.fetch_add(1, Ordering::SeqCst);
            self.inner.solve_transpose(b)
        }

        fn report(&self) -> Report {
            self.inner.report()
        }
    }

    /// A 2D Helmholtz-like matrix on an nx × ny grid, complex symmetric, with its positions.
    fn grid_matrix(nx: usize, ny: usize) -> (Columns, Vec<[f64; 3]>) {
        let n = nx * ny;
        let at = |i: usize, j: usize| i + nx * j;
        let mut t = Vec::new();
        let mut positions = Vec::with_capacity(n);
        for j in 0..ny {
            for i in 0..nx {
                positions.push([i as f64, j as f64, 0.0]);
                let d = c64::new(4.0 - 0.3 * ((i * 7 + j * 3) % 5) as f64, 0.2);
                t.push(Triplet::new(at(i, j), at(i, j), d));
                if i + 1 < nx {
                    t.push(Triplet::new(at(i, j), at(i + 1, j), c64::new(-1.0, 0.05)));
                    t.push(Triplet::new(at(i + 1, j), at(i, j), c64::new(-1.0, 0.05)));
                }
                if j + 1 < ny {
                    t.push(Triplet::new(at(i, j), at(i, j + 1), c64::new(-1.0, 0.0)));
                    t.push(Triplet::new(at(i, j + 1), at(i, j), c64::new(-1.0, 0.0)));
                }
            }
        }
        (Columns::new(n, &t).unwrap(), positions)
    }

    /// The largest |b − A x| (or |b − Aᵀ x|) over the largest |b|.
    fn residual(matrix: &Matrix<'_>, x: &[c64], b: &[c64], transpose: bool) -> f64 {
        let mut r = b.to_vec();
        for j in 0..matrix.n() {
            for e in matrix.column_starts()[j]..matrix.column_starts()[j + 1] {
                let (i, v) = (matrix.row_indices()[e], matrix.values()[e]);
                if transpose {
                    r[j] -= v * x[i];
                } else {
                    r[i] -= v * x[j];
                }
            }
        }
        let scale = b.iter().map(|v| v.norm()).fold(0.0, f64::max);
        r.iter().map(|v| v.norm()).fold(0.0, f64::max) / scale
    }

    #[test]
    fn every_registered_backend_solves_with_the_matrix_and_its_transpose() {
        let (columns, positions) = grid_matrix(23, 17);
        let n = 23 * 17;
        let b: Vec<c64> = (0..n)
            .map(|i| c64::new((i % 7) as f64 - 3.0, (i % 4) as f64))
            .collect();
        for name in [OWN, FAER] {
            let solver = direct(&Choice::parse(name).unwrap()).unwrap();
            let capabilities = solver.capabilities();
            assert_eq!(capabilities.name, name);
            assert!(capabilities.complex && capabilities.transpose);
            let forms: &[Form] = if capabilities.symmetric {
                &[Form::General, Form::Symmetric]
            } else {
                &[Form::General]
            };
            for &form in forms {
                let matrix = columns.matrix(form, Some(&positions)).unwrap();
                let analysis = solver.analyse(&matrix).unwrap().unwrap();
                assert_eq!(analysis.form(), form);
                let factors = analysis.factorize(&matrix).unwrap();
                assert_eq!(factors.report().perturbed_pivots, 0);
                let x = factors.solve(&b).unwrap();
                assert!(residual(&matrix, &x, &b, false) < 1e-13, "{name} {form:?}");
                let x = factors.solve_transpose(&b).unwrap();
                assert!(residual(&matrix, &x, &b, true) < 1e-13, "{name} {form:?}");
                // a right-hand side of another size is an error, not a panic
                assert!(factors.solve(&b[1..]).is_err());
                assert!(factors.solve_transpose(&b[1..]).is_err());
            }
        }
        // faer's LU takes no symmetric form, and says so
        let matrix = columns.matrix(Form::Symmetric, None).unwrap();
        assert!(
            direct(&Choice::parse(FAER).unwrap())
                .unwrap()
                .analyse(&matrix)
                .is_err()
        );
    }

    #[test]
    fn photonoxides_backend_gives_the_bits_of_the_direct_calls() {
        let (columns, positions) = grid_matrix(31, 19);
        let n = 31 * 19;
        let b: Vec<c64> = (0..n)
            .map(|i| c64::new(1.0 / (1.0 + i as f64), 0.5))
            .collect();
        let a = columns.matrix.as_ref();
        for form in [Form::General, Form::Symmetric] {
            let direct_analysis = match form {
                Form::General => sparse::Analysis::new(a, Some(&positions)).unwrap(),
                Form::Symmetric => sparse::Analysis::new_symmetric(a, Some(&positions))
                    .unwrap()
                    .unwrap(),
            };
            let factors = sparse::Multifrontal::new(Arc::new(direct_analysis), a).unwrap();
            let matrix = columns.matrix(form, Some(&positions)).unwrap();
            let through = direct(&Choice::Auto)
                .unwrap()
                .analyse(&matrix)
                .unwrap()
                .unwrap()
                .factorize(&matrix)
                .unwrap();
            assert_eq!(through.solve(&b).unwrap(), factors.solve(&b));
            assert_eq!(
                through.solve_transpose(&b).unwrap(),
                factors.solve_transpose(&b)
            );
            assert_eq!(
                through.report().factor_entries,
                Some(factors.analysis().factor_entries())
            );
        }
    }

    #[test]
    fn a_matrix_that_isnt_one_is_refused() {
        let zero = [c64::new(1.0, 0.0); 3];
        // too few column starts, starts that run backwards, a row out of range, rows repeated
        assert!(Matrix::new(2, &[0, 1], &[0], &zero[..1], Form::General).is_err());
        assert!(Matrix::new(2, &[0, 2, 1], &[0], &zero[..1], Form::General).is_err());
        assert!(Matrix::new(2, &[0, 1, 2], &[0, 2], &zero[..2], Form::General).is_err());
        assert!(Matrix::new(2, &[0, 2, 3], &[1, 1, 0], &zero, Form::General).is_err());
        assert!(Matrix::new(2, &[0, 2, 3], &[1, 0, 0], &zero, Form::General).is_err());
        let ok = Matrix::new(2, &[0, 2, 3], &[0, 1, 1], &zero, Form::General).unwrap();
        assert!(ok.with_positions(&[[0.0; 3]]).is_err());
    }

    #[test]
    fn the_registry_names_what_it_has_and_refuses_what_it_hasnt() {
        // photonoxide's own first, then faer's
        let listed = direct_solvers().unwrap();
        assert_eq!(listed[0].name, OWN);
        assert_eq!(listed[1].name, FAER);
        let own = listed[0].capabilities.clone().unwrap();
        assert!(own.deterministic && own.symmetric && own.transpose);
        assert_eq!(own.threads, Threads::Rayon);
        assert_eq!(own.licence, "MIT OR Apache-2.0");
        assert!(!listed[1].capabilities.clone().unwrap().deterministic);
        // auto and photonoxide are the same backend
        for choice in [Choice::Auto, Choice::Photonoxide] {
            assert_eq!(direct(&choice).unwrap().capabilities().name, OWN);
        }
        // an unknown name, with what there is
        let e = direct(&Choice::parse("pardiso-not-registered").unwrap())
            .err()
            .unwrap()
            .to_string();
        assert!(
            e.contains("pardiso-not-registered") && e.contains("photonoxide"),
            "{e}"
        );
        // one known and not to be had, with its reason; and once it is found, it is used
        register_unavailable("registry-test-library", "not installed: looked in /opt").unwrap();
        let choice = Choice::parse("registry-test-library").unwrap();
        let e = direct(&choice).err().unwrap().to_string();
        assert!(
            e.contains("isn't available") && e.contains("looked in /opt"),
            "{e}"
        );
        let listed = direct_solvers().unwrap();
        let entry = listed
            .iter()
            .find(|l| l.name == "registry-test-library")
            .unwrap();
        assert!(entry.capabilities.is_none() && entry.unavailable.is_some());
        Recording::register("registry-test-library");
        assert_eq!(
            direct(&choice).unwrap().capabilities().name,
            "registry-test-library"
        );
        // names: photonoxide's own can't be taken, nor one that isn't a name
        for name in ["auto", OWN, FAER, "", "Has Capitals", "a b"] {
            assert!(register_unavailable(name, "no").is_err(), "{name:?}");
        }
        assert!(Choice::parse("MKL").is_err());
        assert_eq!(Choice::parse("auto").unwrap(), Choice::Auto);
        assert_eq!(Choice::parse(OWN).unwrap(), Choice::Photonoxide);
        assert_eq!(Choice::parse("mumps").unwrap().to_string(), "mumps");
        assert_eq!(Choice::default(), Choice::Auto);
    }

    #[test]
    fn a_backend_that_isnt_complex_isnt_registered() {
        struct Real;
        impl DirectSolver for Real {
            fn capabilities(&self) -> Capabilities {
                Capabilities {
                    complex: false,
                    ..Capabilities::new("real-only-test", "1", "MIT")
                }
            }
            fn analyse(&self, _: &Matrix<'_>) -> Result<Option<Arc<dyn Analysis>>> {
                Ok(None)
            }
        }
        assert!(register(Arc::new(Real)).is_err());
        assert!(direct(&Choice::parse("real-only-test").unwrap()).is_err());
    }
}
