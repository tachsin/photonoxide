//! Apple Accelerate's sparse direct solvers as a [`DirectSolver`] (#187): part of macOS,
//! nothing to install.
//!
//! Written from the macOS SDK's headers (`vecLib.framework/Headers/Sparse/Solve.h` and its
//! implementation headers, SDKs 14.5, 15.5 and 26.5).
//!
//! - **Which macOS.** Complex matrices came with macOS 15.5, and with them the LU
//!   factorizations; a complex *symmetric* matrix's L D Lᵀ came with macOS 26 (15.5 has the
//!   Hermitian one only, which FDFD's matrices aren't). What this macOS has is asked of the
//!   library itself, by its symbols: without the complex LU the backend is unavailable and
//!   says so, without the symmetric factorization it declares the general form only. Nothing
//!   is emulated in real arithmetic.
//! - **The calls.** The header's `SparseFactor` and `SparseSolve` are inline functions over
//!   exported ones that take their structures by pointer (`_SparseSymbolicFactorLU`,
//!   `_SparseNumericFactorLU_Complex_Double`, `_SparseSolveOpaque_Complex_Double`, ...): those
//!   are called, as the inline ones call them. The structures are declared here as the headers
//!   declare them, their bit-fields as the integers they pack into.
//! - **Phases.** The analysis is the symbolic factorization, kept for every matrix of the
//!   structure (Accelerate counts its references). A factorization is the numeric one, in
//!   storage Accelerate's `malloc` gives and it frees; a solve is in place, the transposed
//!   system by the factorization's `transpose` attribute, as `SparseGetTranspose` sets it.
//! - **Settings:** Accelerate's defaults: its default LU and L D Lᵀ (both with threshold
//!   partial pivoting today, by the header), its default ordering and scaling, its pivot
//!   tolerance of 0.01.
//! - **Errors.** A parameter Accelerate refuses is reported to a function given it (without
//!   one it stops the process); a factorization's failure is its status.
//! - **Every solve is checked.** A matrix with a zero pivot is factorized without a failed
//!   status: the LU's solution then isn't finite, and the L D Lᵀ takes the pivot as zero and
//!   answers all the same (`SparseGetInertia` counts no pivots of a complex factorization).
//!   So each solution's normwise backward error, ‖A x − b‖ / (‖A‖ ‖x‖ + ‖b‖) in the
//!   infinity norm (J. L. Rigal, J. Gaches, J. ACM 14, 543 (1967),
//!   doi:10.1145/321406.321416), is computed, one product with the matrix, and a solve that
//!   leaves more than 1e-8 is an error.
//! - **Report:** the bytes of the factors and the workspace, as the symbolic factorization
//!   gives them. It doesn't say how many entries the factors have.
//! - **Threads** are Accelerate's own. Not declared deterministic.

use std::ffi::{CStr, c_char, c_int, c_void};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::Result;
use photonoxide::backend::{
    Analysis, Capabilities, DirectSolver, Factorization, Form, Matrix, Report, Threads,
};

use crate::Probe;
use crate::discovery::{Candidate, Discovery, Source, Spec, Status};
use crate::library::{Library, error};

/// The framework, which the system's loader finds by this name though no file is there.
const FRAMEWORK: &str = "/System/Library/Frameworks/Accelerate.framework/Accelerate";
/// The system's C library: `malloc` and `free` for Accelerate's options, and the system's
/// version.
const SYSTEM: &str = "/usr/lib/libSystem.B.dylib";

/// Accelerate, for the Libraries page: it is part of macOS, so nothing is searched for.
pub const ACCELERATE: Spec = Spec {
    name: "Accelerate",
    files: &[],
    variables: &[],
    install: Vec::new,
    wheel: None,
};

/// `SparseAttributes_t` and `SparseAttributesComplex_t`, 16 bits of bit-fields in an `unsigned
/// int`: `transpose` is bit 0, `triangle` bit 1, `kind` from bit 2 (two bits, three for
/// complex), and for complex `conjugate_transpose` bit 5.
mod attributes {
    pub const TRANSPOSE: u32 = 1;
    /// `SparseLowerTriangle`.
    pub const LOWER: u32 = 1 << 1;
    /// `SparseSymmetric` (3) as the kind.
    pub const SYMMETRIC: u32 = 3 << 2;
    pub const CONJUGATE_TRANSPOSE: u32 = 1 << 5;
}

/// `SparseMatrixStructure`, and `SparseMatrixStructureComplex`, which differs in its
/// attributes' bits alone.
#[repr(C)]
#[derive(Clone, Copy)]
struct Structure {
    row_count: c_int,
    column_count: c_int,
    column_starts: *mut i64,
    row_indices: *mut c_int,
    attributes: u32,
    block_size: u8,
}

/// `SparseMatrix_Complex_Double`.
#[repr(C)]
struct SparseMatrix {
    structure: Structure,
    data: *mut c64,
}

/// `DenseMatrix_Complex_Double`.
#[repr(C)]
struct Dense {
    row_count: c_int,
    column_count: c_int,
    column_stride: c_int,
    attributes: u32,
    data: *mut c64,
}

type Malloc = unsafe extern "C" fn(usize) -> *mut c_void;
type Free = unsafe extern "C" fn(*mut c_void);

/// `SparseSymbolicFactorOptions`.
#[repr(C)]
struct SymbolicOptions {
    control: u32,
    order_method: u8,
    order: *mut c_int,
    ignore_rows_and_columns: *mut c_int,
    malloc: Malloc,
    free: Free,
    report_error: Option<unsafe extern "C" fn(*const c_char)>,
}

/// `SparseNumericFactorOptions`.
#[repr(C)]
struct NumericOptions {
    control: u32,
    scaling_method: u8,
    scaling: *mut c_void,
    pivot_tolerance: f64,
    zero_tolerance: f64,
}

/// `SparseOpaqueSymbolicFactorization`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Symbolic {
    status: c_int,
    row_count: c_int,
    column_count: c_int,
    attributes: u32,
    block_size: u8,
    kind: u8,
    factorization: *mut c_void,
    workspace_size_float: usize,
    workspace_size_double: usize,
    factor_size_float: usize,
    factor_size_double: usize,
}

/// `SparseOpaqueFactorization_Complex_Double`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Numeric {
    status: c_int,
    attributes: u32,
    symbolic: Symbolic,
    user_factor_storage: bool,
    numeric_factorization: *mut c_void,
    solve_workspace_static: usize,
    solve_workspace_per_rhs: usize,
}

// SparseFactorization_t
const LDLT: u8 = 1;
const LU: u8 = 80;
// SparseStatus_t
const OK: c_int = 0;

/// `_SparseSymbolicFactorLU` and `_SparseSymbolicFactorSymmetric`.
type SymbolicFactor =
    unsafe extern "C" fn(u8, *const Structure, *const SymbolicOptions) -> Symbolic;
/// `_SparseNumericFactorLU_Complex_Double` and `_SparseNumericFactorSymmetric_Complex_Double`:
/// the symbolic factorization, the matrix, the options, the factor's storage, the workspace.
type NumericFactor = unsafe extern "C" fn(
    *mut Symbolic,
    *const SparseMatrix,
    *const NumericOptions,
    *mut c_void,
    *mut c_void,
) -> Numeric;
/// `_SparseSolveOpaque_Complex_Double`: the factorization, the right-hand side (null in
/// place), the solution, the workspace.
type SolveOpaque = unsafe extern "C" fn(*const Numeric, *const Dense, *const Dense, *mut c_void);
type DestroyNumeric = unsafe extern "C" fn(*mut Numeric);
type DestroySymbolic = unsafe extern "C" fn(*mut Symbolic);
/// `int sysctlbyname(const char *, void *, size_t *, void *, size_t)`.
type Sysctl =
    unsafe extern "C" fn(*const c_char, *mut c_void, *mut usize, *mut c_void, usize) -> c_int;

struct Api {
    symbolic_lu: SymbolicFactor,
    symbolic_symmetric: SymbolicFactor,
    numeric_lu: NumericFactor,
    /// From macOS 26.
    numeric_symmetric: Option<NumericFactor>,
    solve: SolveOpaque,
    destroy_numeric: DestroyNumeric,
    destroy_symbolic: DestroySymbolic,
    malloc: Malloc,
    free: Free,
    /// macOS's version.
    version: String,
    // last: the functions above must not outlive them
    _system: Library,
    _library: Library,
}

/// Factorizations made and destroyed one at a time: they share a symbolic factorization's
/// reference count.
static CALLS: Mutex<()> = Mutex::new(());

/// What Accelerate reported to [`report`] since it was last read.
static REPORTED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// `reportError`: Accelerate's message, kept for the error the call returns.
unsafe extern "C" fn report(message: *const c_char) {
    if message.is_null() {
        return;
    }
    // SAFETY: Accelerate gives a C string
    let text = unsafe { CStr::from_ptr(message) }
        .to_string_lossy()
        .trim()
        .to_owned();
    REPORTED
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(text);
}

/// What was reported during the last call, as its error.
fn reported(what: &str) -> Result<()> {
    let messages: Vec<String> = REPORTED
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .drain(..)
        .collect();
    if messages.is_empty() {
        Ok(())
    } else {
        Err(error(format!(
            "Accelerate's {what}: {}",
            messages.join("; ")
        )))
    }
}

/// A `SparseStatus_t`, in words.
fn status(code: c_int) -> &'static str {
    match code {
        0 => "ok",
        -1 => "the factorization failed",
        -2 => "the matrix is singular",
        -3 => "an internal error",
        -4 => "a parameter was refused",
        _ => "a status Accelerate's header doesn't name",
    }
}

/// macOS's version, from `kern.osproductversion`.
fn system_version(sysctl: Sysctl) -> String {
    let mut text = [0u8; 64];
    let mut length = text.len() - 1;
    // SAFETY: the name is a C string, the buffer has `length` bytes and room for the zero
    let code = unsafe {
        sysctl(
            c"kern.osproductversion".as_ptr(),
            text.as_mut_ptr().cast(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    if code != 0 {
        return "unknown".into();
    }
    let end = text.iter().position(|&b| b == 0).unwrap_or(length);
    String::from_utf8_lossy(&text[..end]).into_owned()
}

fn open() -> Result<Api> {
    if !cfg!(target_os = "macos") {
        return Err(error(
            "Accelerate is part of macOS: it isn't on this system",
        ));
    }
    let system = Library::open(Path::new(SYSTEM))?;
    let library = Library::open(Path::new(FRAMEWORK))?;
    // SAFETY: sysctlbyname's declaration (sys/sysctl.h)
    let version = system_version(unsafe { system.function::<Sysctl>("sysctlbyname")? });
    // SAFETY: the type is the declaration in SolveImplementationTyped.h
    let Ok(numeric_lu) =
        (unsafe { library.function::<NumericFactor>("_SparseNumericFactorLU_Complex_Double") })
    else {
        return Err(error(format!(
            "macOS {version} has no sparse factorizations of complex matrices: they came with \
             macOS 15.5"
        )));
    };
    // SAFETY: each type is the function's declaration in SolveImplementation.h or
    // SolveImplementationTyped.h (SDK 26.5; the same in 15.5), malloc's and free's in stdlib.h
    let api = unsafe {
        Api {
            symbolic_lu: library.function("_SparseSymbolicFactorLU")?,
            symbolic_symmetric: library.function("_SparseSymbolicFactorSymmetric")?,
            numeric_lu,
            numeric_symmetric: library
                .function("_SparseNumericFactorSymmetric_Complex_Double")
                .ok(),
            solve: library.function("_SparseSolveOpaque_Complex_Double")?,
            destroy_numeric: library.function("_SparseDestroyOpaqueNumeric_Complex_Double")?,
            destroy_symbolic: library.function("_SparseDestroyOpaqueSymbolic")?,
            malloc: system.function("malloc")?,
            free: system.function("free")?,
            version,
            _system: system,
            _library: library,
        }
    };
    Ok(api)
}

/// A structure's columns as Accelerate takes them, and its symbolic factorization.
struct Columns {
    api: Arc<Api>,
    form: Form,
    n: c_int,
    starts: Vec<i64>,
    rows: Vec<c_int>,
    /// Where each entry kept is among the matrix's: all of a general matrix's, a symmetric
    /// one's on and below its diagonal.
    kept: Vec<usize>,
    /// The matrix's column starts, to know another structure by.
    given: Vec<usize>,
    symbolic: Mutex<Symbolic>,
}

// SAFETY: the symbolic factorization is Accelerate's, reference-counted, used here under its
// Mutex and the one lock on factorizations; the arrays are owned
unsafe impl Send for Columns {}
// SAFETY: as Send
unsafe impl Sync for Columns {}

impl Columns {
    fn attributes(&self) -> u32 {
        match self.form {
            Form::Symmetric => attributes::LOWER | attributes::SYMMETRIC,
            _ => 0,
        }
    }
}

impl Drop for Columns {
    fn drop(&mut self) {
        let _one = CALLS.lock().unwrap_or_else(|p| p.into_inner());
        let symbolic = self.symbolic.get_mut().unwrap_or_else(|p| p.into_inner());
        if !symbolic.factorization.is_null() {
            // SAFETY: the symbolic factorization made in `analyse`, released once
            unsafe { (self.api.destroy_symbolic)(symbolic) };
        }
    }
}

/// A matrix's numeric factorization.
struct Engine {
    columns: Arc<Columns>,
    numeric: Numeric,
    /// The entries factorized, for each solve's check.
    values: Vec<c64>,
    /// The matrix's largest row sum and largest column sum of moduli.
    norms: (f64, f64),
    report: Report,
}

/// The largest backward error a solve may leave. A direct solve leaves about 1e-16; one that
/// took a zero pivot as zero leaves about 1.
const BACKWARD_ERROR: f64 = 1e-8;

/// The entries a factorization is given, by columns: a general matrix's, or a symmetric one's
/// on and below its diagonal.
struct Pattern<'a> {
    symmetric: bool,
    starts: &'a [i64],
    rows: &'a [c_int],
}

impl Pattern<'_> {
    /// Each entry as (row, column, index).
    fn entries(&self) -> impl Iterator<Item = (usize, usize, usize)> + '_ {
        (0..self.starts.len() - 1).flat_map(move |j| {
            (self.starts[j] as usize..self.starts[j + 1] as usize)
                .map(move |e| (self.rows[e] as usize, j, e))
        })
    }

    /// The largest row sum and the largest column sum of the entries' moduli, a symmetric
    /// matrix's entries above its diagonal counted as their mirrors below it.
    fn norms(&self, values: &[c64]) -> (f64, f64) {
        let n = self.starts.len() - 1;
        let (mut by_row, mut by_column) = (vec![0.0f64; n], vec![0.0f64; n]);
        for (i, j, e) in self.entries() {
            let modulus = values[e].norm();
            by_row[i] += modulus;
            by_column[j] += modulus;
            if self.symmetric && i != j {
                by_row[j] += modulus;
                by_column[i] += modulus;
            }
        }
        (
            by_row.into_iter().fold(0.0, f64::max),
            by_column.into_iter().fold(0.0, f64::max),
        )
    }

    /// A x, or Aᵀ x.
    fn product(&self, values: &[c64], x: &[c64], transpose: bool) -> Vec<c64> {
        let mut y = vec![c64::new(0.0, 0.0); x.len()];
        for (i, j, e) in self.entries() {
            let (to, from) = if transpose { (j, i) } else { (i, j) };
            y[to] += values[e] * x[from];
            if self.symmetric && i != j {
                y[from] += values[e] * x[to];
            }
        }
        y
    }
}

impl Columns {
    fn pattern(&self) -> Pattern<'_> {
        Pattern {
            symmetric: self.form == Form::Symmetric,
            starts: &self.starts,
            rows: &self.rows,
        }
    }
}

// SAFETY: the factorization is Accelerate's, used by one thread at a time (behind a Mutex)
unsafe impl Send for Engine {}

impl Engine {
    fn new(columns: &Arc<Columns>, matrix: &Matrix<'_>) -> Result<Engine> {
        let api = &columns.api;
        let start = Instant::now();
        let given = matrix.values();
        let mut values: Vec<c64> = columns.kept.iter().map(|&e| given[e]).collect();
        // Accelerate reads the structure and doesn't keep it
        let (mut starts, mut rows) = (columns.starts.clone(), columns.rows.clone());
        let a = SparseMatrix {
            structure: Structure {
                row_count: columns.n,
                column_count: columns.n,
                column_starts: starts.as_mut_ptr(),
                row_indices: rows.as_mut_ptr(),
                attributes: columns.attributes(),
                block_size: 1,
            },
            data: values.as_mut_ptr(),
        };
        // _SparseDefaultNumericFactorOptions_Complex_Double
        let options = NumericOptions {
            control: 0,
            scaling_method: 0,
            scaling: std::ptr::null_mut(),
            pivot_tolerance: 0.01,
            zero_tolerance: 1e-4 * f64::EPSILON,
        };
        let factor = match columns.form {
            Form::Symmetric => api
                .numeric_symmetric
                .ok_or_else(|| error("this macOS factorizes no complex symmetric matrix"))?,
            _ => api.numeric_lu,
        };
        let one = CALLS.lock().unwrap_or_else(|p| p.into_inner());
        let mut symbolic = columns.symbolic.lock().unwrap_or_else(|p| p.into_inner());
        // a complex factor takes twice the bytes the symbolic factorization gives for doubles
        let (factor_bytes, workspace_bytes) = (
            2 * symbolic.factor_size_double,
            2 * symbolic.workspace_size_double,
        );
        // SAFETY: as the header's inline SparseFactor: storage and workspace of the sizes the
        // symbolic factorization asks, from the malloc its options name; the matrix's arrays
        // live through the call; the workspace is freed, the storage is Accelerate's to free
        // with the factorization (userFactorStorage false)
        let numeric = unsafe {
            let storage = (api.malloc)(factor_bytes.max(1));
            let workspace = (api.malloc)(workspace_bytes.max(1));
            if storage.is_null() || workspace.is_null() {
                (api.free)(storage);
                (api.free)(workspace);
                return Err(error(format!(
                    "Accelerate's factorization: {} bytes couldn't be had",
                    factor_bytes + workspace_bytes
                )));
            }
            let mut numeric = factor(&mut *symbolic, &a, &options, storage, workspace);
            (api.free)(workspace);
            numeric.user_factor_storage = false;
            if numeric.numeric_factorization.is_null() {
                // nothing was made of the storage
                (api.free)(storage);
            }
            numeric
        };
        let norms = columns.pattern().norms(&values);
        let mut engine = Engine {
            columns: columns.clone(),
            numeric,
            values,
            norms,
            report: Report::default(),
        };
        drop(symbolic);
        drop(one);
        reported("factorization")?;
        if engine.numeric.status != OK {
            return Err(error(format!(
                "Accelerate's factorization: {} (status {})",
                status(engine.numeric.status),
                engine.numeric.status
            )));
        }
        engine.report.peak_memory_bytes = Some((factor_bytes + workspace_bytes) as u64);
        engine.report.factorization_seconds = start.elapsed().as_secs_f64();
        Ok(engine)
    }

    fn solve(&mut self, rhs: &[c64], transpose: bool) -> Result<Vec<c64>> {
        let n = self.columns.n;
        if rhs.len() != n as usize {
            return Err(error(format!(
                "a right-hand side of {} for a matrix of {n} unknowns",
                rhs.len()
            )));
        }
        let api = &self.columns.api;
        let mut x = rhs.to_vec();
        let xb = Dense {
            row_count: n,
            column_count: 1,
            column_stride: n,
            attributes: 0,
            data: x.as_mut_ptr(),
        };
        // as SparseGetTranspose: the same factorization, marked transposed
        let mut factored = self.numeric;
        if transpose {
            factored.attributes ^= attributes::TRANSPOSE;
            factored.attributes &= !attributes::CONJUGATE_TRANSPOSE;
        }
        let bytes = factored.solve_workspace_static + factored.solve_workspace_per_rhs;
        // SAFETY: as the header's inline SparseSolve: a completed factorization, one
        // right-hand side of n entries solved in place, a workspace of the size it asks
        unsafe {
            let workspace = (api.malloc)(bytes.max(1));
            if workspace.is_null() {
                return Err(error(format!(
                    "Accelerate's solve: {bytes} bytes couldn't be had"
                )));
            }
            (api.solve)(&factored, std::ptr::null(), &xb, workspace);
            (api.free)(workspace);
        }
        reported("solve")?;
        // a zero pivot isn't a failed status to Accelerate, and its L D Lᵀ takes it as zero
        // and goes on: the solution is checked against its system
        let largest = |v: &[c64]| v.iter().map(|z| z.norm()).fold(0.0, f64::max);
        let product = self.columns.pattern().product(&self.values, &x, transpose);
        let residual = product
            .iter()
            .zip(rhs)
            .map(|(a, b)| (a - b).norm())
            .fold(0.0, f64::max);
        let norm = if transpose {
            self.norms.1
        } else {
            self.norms.0
        };
        let scale = norm * largest(&x) + largest(rhs);
        // (a maximum passes over a NaN, so the solution's finiteness is asked of it)
        let finite = x.iter().all(|z| z.re.is_finite() && z.im.is_finite());
        let satisfied = finite && residual <= BACKWARD_ERROR * scale;
        if !satisfied {
            return Err(error(format!(
                "Accelerate's solve: the solution doesn't satisfy its system (a backward error \
                 of {:.1e}): the matrix is singular to its factorization",
                if finite {
                    residual / scale
                } else {
                    f64::INFINITY
                }
            )));
        }
        Ok(x)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if self.numeric.numeric_factorization.is_null() {
            return;
        }
        let _one = CALLS.lock().unwrap_or_else(|p| p.into_inner());
        // SAFETY: the factorization made in `new`, released once
        unsafe { (self.columns.api.destroy_numeric)(&mut self.numeric) };
    }
}

/// Accelerate's sparse solvers, as photonoxide's registry has them.
pub struct Accelerate {
    api: Arc<Api>,
}

impl Accelerate {
    /// Accelerate loaded, on a macOS that factorizes complex matrices.
    ///
    /// # Errors
    ///
    /// Why not: another system, or a macOS before 15.5.
    pub fn load() -> std::result::Result<Accelerate, String> {
        let api = open().map_err(|e| match e {
            photonoxide::Error::InvalidValue { reason, .. } => reason,
            other => other.to_string(),
        })?;
        Ok(Accelerate { api: Arc::new(api) })
    }

    /// macOS's version, which is Accelerate's.
    pub fn version(&self) -> &str {
        &self.api.version
    }
}

/// Accelerate: whether it is here, and macOS's version.
pub fn probe() -> Probe {
    let mut discovery = Discovery {
        library: ACCELERATE.name,
        candidates: Vec::new(),
    };
    let mut version = None;
    let mut details = Vec::new();
    if cfg!(target_os = "macos") {
        let status = match Accelerate::load() {
            Ok(accelerate) => {
                version = Some(accelerate.version().to_owned());
                details.push(if accelerate.api.numeric_symmetric.is_some() {
                    "complex LU and symmetric L D Lᵀ".to_owned()
                } else {
                    "complex LU (the symmetric L D Lᵀ needs macOS 26)".to_owned()
                });
                Status::Used
            }
            Err(reason) => Status::Failed(reason),
        };
        discovery.candidates.push(Candidate {
            path: FRAMEWORK.into(),
            source: Source::SystemPath,
            status,
        });
    }
    Probe {
        discovery,
        version,
        details,
    }
}

impl DirectSolver for Accelerate {
    fn capabilities(&self) -> Capabilities {
        let mut c = Capabilities::new(
            "accelerate",
            format!("macOS {}", self.api.version),
            "part of macOS, under Apple's licence for it",
        );
        c.symmetric = self.api.numeric_symmetric.is_some();
        c.transpose = true;
        c.threads = Threads::Library("Accelerate's own".into());
        c.deterministic = false;
        c
    }

    fn analyse(&self, matrix: &Matrix<'_>) -> Result<Option<Arc<dyn Analysis>>> {
        let form = matrix.form();
        let lower = match form {
            Form::General => false,
            Form::Symmetric if self.api.numeric_symmetric.is_some() => true,
            _ => {
                return Err(error(format!(
                    "Accelerate on macOS {} takes general matrices only: a complex symmetric \
                     one's factorization came with macOS 26",
                    self.api.version
                )));
            }
        };
        let start = Instant::now();
        let n = c_int::try_from(matrix.n())
            .map_err(|_| error("Accelerate's dimensions are 32-bit: too many unknowns"))?;
        let (cs, ri) = (matrix.column_starts(), matrix.row_indices());
        let (mut starts, mut rows, mut kept) = (vec![0i64], Vec::new(), Vec::new());
        for j in 0..matrix.n() {
            for (e, &row) in (cs[j]..cs[j + 1]).zip(&ri[cs[j]..cs[j + 1]]) {
                if lower && row < j {
                    continue;
                }
                // (below 2³¹: n is)
                rows.push(row as c_int);
                kept.push(e);
            }
            starts.push(rows.len() as i64);
        }
        let mut columns = Columns {
            api: self.api.clone(),
            form,
            n,
            starts,
            rows,
            kept,
            given: cs.to_vec(),
            symbolic: Mutex::new(Symbolic {
                status: -3,
                row_count: 0,
                column_count: 0,
                attributes: 0,
                block_size: 0,
                kind: 0,
                factorization: std::ptr::null_mut(),
                workspace_size_float: 0,
                workspace_size_double: 0,
                factor_size_float: 0,
                factor_size_double: 0,
            }),
        };
        let structure = Structure {
            row_count: n,
            column_count: n,
            column_starts: columns.starts.as_mut_ptr(),
            row_indices: columns.rows.as_mut_ptr(),
            attributes: columns.attributes(),
            block_size: 1,
        };
        // _SparseDefaultSymbolicFactorOptions, with errors reported instead of a trap
        let options = SymbolicOptions {
            control: 0,
            order_method: 0,
            order: std::ptr::null_mut(),
            ignore_rows_and_columns: std::ptr::null_mut(),
            malloc: self.api.malloc,
            free: self.api.free,
            report_error: Some(report),
        };
        let symbolic = {
            let _one = CALLS.lock().unwrap_or_else(|p| p.into_inner());
            // SAFETY: the structure's arrays have n + 1 starts and as many rows as the last
            // start, and live through the call; the options are the header's defaults
            unsafe {
                if lower {
                    (self.api.symbolic_symmetric)(LDLT, &structure, &options)
                } else {
                    (self.api.symbolic_lu)(LU, &structure, &options)
                }
            }
        };
        *columns
            .symbolic
            .get_mut()
            .unwrap_or_else(|p| p.into_inner()) = symbolic;
        reported("symbolic factorization")?;
        if symbolic.status != OK || symbolic.factorization.is_null() {
            return Err(error(format!(
                "Accelerate's symbolic factorization: {} (status {})",
                status(symbolic.status),
                symbolic.status
            )));
        }
        Ok(Some(Arc::new(Analysed {
            columns: Arc::new(columns),
            seconds: start.elapsed().as_secs_f64(),
        })))
    }
}

struct Analysed {
    columns: Arc<Columns>,
    seconds: f64,
}

impl Analysis for Analysed {
    fn form(&self) -> Form {
        self.columns.form
    }

    fn factorize(&self, matrix: &Matrix<'_>) -> Result<Box<dyn Factorization>> {
        let same = matrix.form() == self.columns.form
            && matrix.n() == self.columns.n as usize
            && matrix.column_starts() == self.columns.given.as_slice();
        if !same {
            return Err(error("a matrix of another structure than the one analysed"));
        }
        let mut engine = Engine::new(&self.columns, matrix)?;
        engine.report.analysis_seconds = self.seconds;
        Ok(Box::new(Factors {
            report: engine.report.clone(),
            engine: Mutex::new(engine),
        }))
    }
}

struct Factors {
    engine: Mutex<Engine>,
    report: Report,
}

fn poisoned<T>(_: T) -> photonoxide::Error {
    error("Accelerate's factors' lock was poisoned by a panic")
}

impl Factorization for Factors {
    fn solve(&self, b: &[c64]) -> Result<Vec<c64>> {
        self.engine.lock().map_err(poisoned)?.solve(b, false)
    }

    fn solve_transpose(&self, b: &[c64]) -> Result<Vec<c64>> {
        self.engine.lock().map_err(poisoned)?.solve(b, true)
    }

    fn report(&self) -> Report {
        self.report.clone()
    }
}

#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use std::mem::{offset_of, size_of};

    use super::*;

    #[test]
    fn the_structures_are_laid_out_as_the_headers_lay_them_out() {
        // int, int, long *, int *, 4 bytes of bit-fields, uint8_t
        assert_eq!(size_of::<Structure>(), 32);
        assert_eq!(offset_of!(Structure, attributes), 24);
        assert_eq!(offset_of!(Structure, block_size), 28);
        assert_eq!(size_of::<SparseMatrix>(), 40);
        assert_eq!(offset_of!(SparseMatrix, data), 32);
        // three ints, the attributes, the data
        assert_eq!(size_of::<Dense>(), 24);
        assert_eq!(offset_of!(Dense, data), 16);
        // SparseControl_t (uint32_t), SparseOrder_t (uint8_t), five pointers
        assert_eq!(size_of::<SymbolicOptions>(), 48);
        assert_eq!(offset_of!(SymbolicOptions, order), 8);
        assert_eq!(offset_of!(SymbolicOptions, report_error), 40);
        // SparseControl_t, SparseScaling_t (uint8_t), a pointer, two doubles
        assert_eq!(size_of::<NumericOptions>(), 32);
        assert_eq!(offset_of!(NumericOptions, pivot_tolerance), 16);
        // status, two ints, the attributes, blockSize and type (uint8_t), a pointer, four sizes
        assert_eq!(size_of::<Symbolic>(), 64);
        assert_eq!(offset_of!(Symbolic, kind), 17);
        assert_eq!(offset_of!(Symbolic, factorization), 24);
        assert_eq!(offset_of!(Symbolic, factor_size_double), 56);
        // status, the attributes, the symbolic factorization, a bool, a pointer, two sizes
        assert_eq!(size_of::<Numeric>(), 104);
        assert_eq!(offset_of!(Numeric, symbolic), 8);
        assert_eq!(offset_of!(Numeric, user_factor_storage), 72);
        assert_eq!(offset_of!(Numeric, numeric_factorization), 80);
        assert_eq!(offset_of!(Numeric, solve_workspace_per_rhs), 96);
    }

    #[test]
    fn the_attributes_bits_are_the_bit_fields() {
        // transpose: 1, triangle: 1, kind: 2 (3 for complex), conjugate_transpose: 1 after it
        assert_eq!(attributes::TRANSPOSE, 0b1);
        assert_eq!(attributes::LOWER, 0b10);
        assert_eq!(attributes::SYMMETRIC, 0b1100);
        assert_eq!(attributes::CONJUGATE_TRANSPOSE, 0b10_0000);
    }

    #[test]
    fn a_status_and_a_report_become_errors() {
        assert_eq!(status(-2), "the matrix is singular");
        assert!(reported("solve").is_ok());
        // SAFETY: a C string
        unsafe { report(c"Bad symbolic factor.\n".as_ptr()) };
        let e = reported("solve").unwrap_err().to_string();
        assert!(
            e.contains("Accelerate's solve: Bad symbolic factor."),
            "{e}"
        );
        assert!(reported("solve").is_ok());
    }

    #[test]
    fn a_patterns_products_and_norms_are_the_matrixs() {
        let z = |re: f64, im: f64| c64::new(re, im);
        // [[1, 2i, 0], [3, 4, 0], [0, 5, 6]] by columns
        let general = Pattern {
            symmetric: false,
            starts: &[0, 2, 5, 6],
            rows: &[0, 1, 0, 1, 2, 2],
        };
        let values = [
            z(1.0, 0.0),
            z(3.0, 0.0),
            z(0.0, 2.0),
            z(4.0, 0.0),
            z(5.0, 0.0),
            z(6.0, 0.0),
        ];
        let x = [z(1.0, 0.0), z(0.0, 1.0), z(2.0, 0.0)];
        // A x = [1 + 2i·i, 3 + 4i, 5i + 12]
        assert_eq!(
            general.product(&values, &x, false),
            vec![z(-1.0, 0.0), z(3.0, 4.0), z(12.0, 5.0)]
        );
        // Aᵀ x = [1 + 3i, 2i + 4i + 10, 12]
        assert_eq!(
            general.product(&values, &x, true),
            vec![z(1.0, 3.0), z(10.0, 6.0), z(12.0, 0.0)]
        );
        // rows: 3, 7, 11; columns: 4, 11, 6
        assert_eq!(general.norms(&values), (11.0, 11.0));
        // [[1, 2i, 0], [2i, 4, 5], [0, 5, 6]] by its lower triangle
        let symmetric = Pattern {
            symmetric: true,
            starts: &[0, 2, 4, 5],
            rows: &[0, 1, 1, 2, 2],
        };
        let lower = [
            z(1.0, 0.0),
            z(0.0, 2.0),
            z(4.0, 0.0),
            z(5.0, 0.0),
            z(6.0, 0.0),
        ];
        let whole = vec![z(-1.0, 0.0), z(10.0, 6.0), z(12.0, 5.0)];
        assert_eq!(symmetric.product(&lower, &x, false), whole);
        assert_eq!(symmetric.product(&lower, &x, true), whole);
        // rows and columns alike: 3, 11, 11
        assert_eq!(symmetric.norms(&lower), (11.0, 11.0));
    }

    #[test]
    fn it_isnt_offered_off_macos() {
        if !cfg!(target_os = "macos") {
            let reason = Accelerate::load().err().unwrap();
            assert!(reason.contains("part of macOS"), "{reason}");
            assert!(probe().discovery.candidates.is_empty());
        }
    }
}
