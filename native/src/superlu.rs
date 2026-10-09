//! SuperLU as a [`DirectSolver`] (#177): J. W. Demmel, S. C. Eisenstat, J. R. Gilbert, X. S.
//! Li, J. W. H. Liu, "A supernodal approach to sparse partial pivoting", SIAM J. Matrix Anal.
//! Appl. 20, 720 (1999), doi:10.1137/S0895479895291765.
//!
//! The sequential library, complex double, through its expert driver `zgssvx`, installed by the
//! user under its BSD licence and never redistributed.
//!
//! - **Releases.** The library says nothing of its version, so it is taken from the file's
//!   name, which the packages give it (`libsuperlu.so.7`, `libsuperlu.6.dylib`): majors 5, 6
//!   and 7, whose headers (5.3.0, 6.0.1, 7.0.1) declare `zgssvx` and its structures alike. One
//!   constant differs: "the column ordering given" is 7 in release 5 and 8 from 6, where an
//!   ordering was added before it. A file without its major in its name isn't used (a
//!   SuperLU 4's `zgssvx` takes other arguments), which leaves Windows out: no package there
//!   names it so.
//! - **32-bit indices.** SuperLU's `int_t` is `int` unless it was built otherwise. Which, is
//!   read from the library: a matrix it makes says where its row count sits. A build with
//!   64-bit indices is refused.
//! - **The structures** are given as zeroed buffers with room to spare, their fields set at
//!   the offsets the headers give (`superlu_options_t`, `SuperMatrix`, `SuperLUStat_t`,
//!   `mem_usage_t`); `GlobalLU_t` is SuperLU's own to fill.
//! - **The form:** general matrices only, an LU with partial pivoting by rows, its columns
//!   ordered by COLAMD. A system photonoxide would factorize as complex symmetric is given
//!   whole, as its LU.
//! - **Phases.** The analysis is the column ordering (`get_perm_c`), kept for every matrix of
//!   the structure. A factorization is `zgssvx` with that ordering, with SuperLU's
//!   equilibration; a solve is `zgssvx` on the factors, with one or more steps of iterative
//!   refinement in double precision, the transposed system by its `Trans` option.
//! - **Report:** the factors' entries, nnz(L) + nnz(U) as SuperLU counts them, and the memory
//!   it says it needs (`mem_usage_t.total_needed`). It replaces no pivots.
//! - **One call at a time,** under one lock. One thread; not declared deterministic.
//! - **Errors:** `zgssvx`'s `info`: a zero pivot, with its column, or memory that couldn't be
//!   had.

use std::ffi::{c_char, c_int, c_void};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::Result;
use photonoxide::backend::{
    Analysis, Capabilities, DirectSolver, Factorization, Form, Matrix, Report, Threads,
};

use crate::Probe;
use crate::discovery::Spec;
use crate::library::{Library, error, load};

/// SuperLU's sequential library, by the names that carry its major release.
pub const SUPERLU: Spec = Spec {
    name: "SuperLU",
    files: if cfg!(target_os = "linux") {
        &["libsuperlu.so.7", "libsuperlu.so.6", "libsuperlu.so.5"]
    } else if cfg!(target_os = "macos") {
        &[
            "libsuperlu.7.dylib",
            "libsuperlu.6.dylib",
            "libsuperlu.5.dylib",
        ]
    } else {
        &[]
    },
    variables: &["PHOTONOXIDE_SUPERLU"],
    install: Vec::new,
    wheel: None,
};

/// A buffer for one of SuperLU's structures: 8-byte words, zeroed, larger than the structure.
struct Buffer(Vec<u64>);

impl Buffer {
    fn new(bytes: usize) -> Buffer {
        Buffer(vec![0; bytes.div_ceil(8)])
    }

    fn pointer(&mut self) -> *mut c_void {
        self.0.as_mut_ptr().cast()
    }

    fn int(&self, offset: usize) -> i32 {
        debug_assert!(offset.is_multiple_of(4) && offset + 4 <= 8 * self.0.len());
        // SAFETY: inside the buffer, aligned for an i32 (the buffer is of u64)
        unsafe {
            self.0
                .as_ptr()
                .cast::<u8>()
                .add(offset)
                .cast::<i32>()
                .read()
        }
    }

    fn set_int(&mut self, offset: usize, value: i32) {
        debug_assert!(offset.is_multiple_of(4) && offset + 4 <= 8 * self.0.len());
        // SAFETY: as `int`
        unsafe {
            self.0
                .as_mut_ptr()
                .cast::<u8>()
                .add(offset)
                .cast::<i32>()
                .write(value);
        }
    }

    fn float(&self, offset: usize) -> f32 {
        f32::from_bits(self.int(offset) as u32)
    }

    fn address(&self, offset: usize) -> usize {
        debug_assert!(offset.is_multiple_of(8));
        self.0[offset / 8] as usize
    }
}

/// `superlu_options_t`'s fields (slu_util.h, the same in 5.3.0, 6.0.1 and 7.0.1): enums and
/// ints of 4 bytes, doubles on 8.
mod options {
    pub const FACT: usize = 0;
    pub const EQUIL: usize = 4;
    pub const COL_PERM: usize = 8;
    pub const TRANS: usize = 12;
    pub const ITER_REFINE: usize = 16;
    pub const PIVOT_GROWTH: usize = 36;
    pub const CONDITION_NUMBER: usize = 40;
    pub const PRINT_STAT: usize = 120;
    /// The structure's size; the buffer is larger.
    pub const SIZE: usize = 144;
}

/// `SuperMatrix`'s fields with 32-bit indices: three enums, the rows and columns, the store.
mod supermatrix {
    pub const NROW: usize = 12;
    pub const NCOL: usize = 16;
    pub const STORE: usize = 24;
    /// Where the rows would be with 64-bit indices.
    pub const NROW_LONG: usize = 16;
}

/// `SuperLUStat_t`: three pointers, then the tiny pivots, the refinement's steps, the
/// memory's expansions.
const REFINE_STEPS: usize = 28;
/// `mem_usage_t`: two floats, for the factors and in all.
const TOTAL_NEEDED: usize = 4;

// superlu_enum_consts.h and supermatrix.h, the same in every release
const NO: i32 = 0;
const YES: i32 = 1;
const DOFACT: i32 = 0;
const FACTORED: i32 = 3;
const COLAMD: c_int = 3;
const NOTRANS: i32 = 0;
const TRANS: i32 = 1;
/// `SLU_DOUBLE`: refinement with the residual in double precision.
const REFINE_DOUBLE: i32 = 2;
const SLU_NC: c_int = 0;
const SLU_DN: c_int = 6;
const SLU_Z: c_int = 3;
const SLU_GE: c_int = 0;

/// `MY_PERMC`, the column ordering given: after `ZOLTAN`, before which release 6 added
/// `METIS_ATA`.
fn my_permc(major: u32) -> i32 {
    if major == 5 { 7 } else { 8 }
}

type SetDefaultOptions = unsafe extern "C" fn(*mut c_void);
type StatFn = unsafe extern "C" fn(*mut c_void);
type DestroyFn = unsafe extern "C" fn(*mut c_void);
/// `void get_perm_c(int ispec, SuperMatrix *A, int *perm_c)`.
type GetPermC = unsafe extern "C" fn(c_int, *mut c_void, *mut c_int);
/// `void zCreate_CompCol_Matrix(SuperMatrix *, int m, int n, int_t nnz, doublecomplex *nzval,
/// int_t *rowind, int_t *colptr, Stype_t, Dtype_t, Mtype_t)`.
type CreateCompCol = unsafe extern "C" fn(
    *mut c_void,
    c_int,
    c_int,
    c_int,
    *mut c64,
    *mut c_int,
    *mut c_int,
    c_int,
    c_int,
    c_int,
);
/// `void zCreate_Dense_Matrix(SuperMatrix *, int m, int n, doublecomplex *x, int ldx, Stype_t,
/// Dtype_t, Mtype_t)`.
type CreateDense =
    unsafe extern "C" fn(*mut c_void, c_int, c_int, *mut c64, c_int, c_int, c_int, c_int);
/// `void zgssvx(superlu_options_t *, SuperMatrix *A, int *perm_c, int *perm_r, int *etree,
/// char *equed, double *R, double *C, SuperMatrix *L, SuperMatrix *U, void *work, int_t lwork,
/// SuperMatrix *B, SuperMatrix *X, double *recip_pivot_growth, double *rcond, double *ferr,
/// double *berr, GlobalLU_t *, mem_usage_t *, SuperLUStat_t *, int_t *info)`.
type Zgssvx = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut c_int,
    *mut c_int,
    *mut c_int,
    *mut c_char,
    *mut f64,
    *mut f64,
    *mut c_void,
    *mut c_void,
    *mut c_void,
    c_int,
    *mut c_void,
    *mut c_void,
    *mut f64,
    *mut f64,
    *mut f64,
    *mut f64,
    *mut c_void,
    *mut c_void,
    *mut c_void,
    *mut c_int,
);

struct Api {
    set_default_options: SetDefaultOptions,
    stat_init: StatFn,
    stat_free: StatFn,
    get_perm_c: GetPermC,
    create_compcol: CreateCompCol,
    create_dense: CreateDense,
    zgssvx: Zgssvx,
    destroy_store: DestroyFn,
    destroy_supernode: DestroyFn,
    destroy_compcol: DestroyFn,
    version: String,
    major: u32,
    // last: the functions above must not outlive it
    _library: Library,
}

/// One call into SuperLU at a time.
static CALLS: Mutex<()> = Mutex::new(());

/// The release in a library's file name, its links followed: `libsuperlu.so.7.0.0` is
/// ("7.0.0", 7), `libsuperlu.6.dylib` ("6", 6).
fn release(path: &Path) -> Option<(String, u32)> {
    let name = std::fs::canonicalize(path)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .or_else(|| path.file_name().map(|n| n.to_string_lossy().into_owned()))?;
    let numbers: Vec<&str> = name
        .split('.')
        .filter(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        .collect();
    let major: u32 = numbers.first()?.parse().ok()?;
    Some((numbers.join("."), major))
}

/// The release of the library at `path`, if it is one whose `zgssvx` this is.
fn supported(path: &Path) -> Result<(String, u32)> {
    match release(path) {
        Some((version, major)) if (5..=7).contains(&major) => Ok((version, major)),
        Some((version, _)) => Err(error(format!(
            "SuperLU {version}: photonoxide knows releases 5, 6 and 7"
        ))),
        None => Err(error(
            "its file's name doesn't say which release it is, and their drivers differ",
        )),
    }
}

/// Whether the library's indices are 32-bit: a dense matrix it makes of 3 rows has them where
/// a 32-bit `int_t` puts them.
fn narrow(create_dense: CreateDense, destroy_store: DestroyFn) -> Result<bool> {
    let mut x = Buffer::new(64);
    let mut values = [c64::new(0.0, 0.0); 3];
    let _one = CALLS.lock().unwrap_or_else(|p| p.into_inner());
    // SAFETY: zCreate_Dense_Matrix's arguments, all in registers or 8-byte slots whatever the
    // index's width; it writes the matrix's header and allocates its store
    unsafe {
        create_dense(
            x.pointer(),
            3,
            1,
            values.as_mut_ptr(),
            3,
            SLU_DN,
            SLU_Z,
            SLU_GE,
        )
    };
    let narrow = x.int(supermatrix::NROW) == 3 && x.int(supermatrix::NCOL) == 1;
    let wide = x.int(supermatrix::NROW_LONG) == 3 && !narrow;
    // SAFETY: the matrix made above; only its store is freed, the values are ours
    unsafe { destroy_store(x.pointer()) };
    if narrow {
        Ok(true)
    } else if wide {
        Ok(false)
    } else {
        Err(error(
            "a matrix it makes isn't laid out as SuperLU 5 to 7 lay it out",
        ))
    }
}

fn open(library: Library) -> Result<Api> {
    let (version, major) = supported(library.path())?;
    // SAFETY: each type is the function's declaration in slu_util.h or slu_zdefs.h (5.3.0,
    // 6.0.1 and 7.0.1 alike, with `int_t` an `int`, checked below)
    let api = unsafe {
        Api {
            set_default_options: library.function("set_default_options")?,
            stat_init: library.function("StatInit")?,
            stat_free: library.function("StatFree")?,
            get_perm_c: library.function("get_perm_c")?,
            create_compcol: library.function("zCreate_CompCol_Matrix")?,
            create_dense: library.function("zCreate_Dense_Matrix")?,
            zgssvx: library.function("zgssvx")?,
            destroy_store: library.function("Destroy_SuperMatrix_Store")?,
            destroy_supernode: library.function("Destroy_SuperNode_Matrix")?,
            destroy_compcol: library.function("Destroy_CompCol_Matrix")?,
            version,
            major,
            _library: library,
        }
    };
    if !narrow(api.create_dense, api.destroy_store)? {
        return Err(error(
            "this SuperLU was built with 64-bit indices, which photonoxide doesn't call it with",
        ));
    }
    Ok(api)
}

/// A matrix's columns as SuperLU takes them: zero-based, 32-bit.
struct Columns {
    n: c_int,
    starts: Vec<c_int>,
    rows: Vec<c_int>,
    /// The column ordering, kept for every matrix of the structure.
    perm_c: Vec<c_int>,
}

fn narrowed(indices: &[usize], what: &str) -> Result<Vec<c_int>> {
    indices
        .iter()
        .map(|&i| {
            c_int::try_from(i).map_err(|_| {
                error(format!(
                    "SuperLU's indices are 32-bit: {what} {i} is too large"
                ))
            })
        })
        .collect()
}

/// What `zgssvx`'s `info` says, for a matrix of n columns.
fn check(info: c_int, n: c_int, what: &str) -> Result<()> {
    if info == 0 {
        Ok(())
    } else if info < 0 {
        Err(error(format!(
            "SuperLU's {what}: its argument {} is invalid",
            -info
        )))
    } else if info <= n {
        Err(error(format!(
            "SuperLU's {what}: the pivot of column {info} is exactly zero, the matrix is singular"
        )))
    } else {
        Err(error(format!(
            "SuperLU's {what} couldn't have its memory ({} bytes were allocated when it failed)",
            info - n
        )))
    }
}

/// A matrix's factors as `zgssvx` leaves them, with what its solves need.
struct Engine {
    api: Arc<Api>,
    columns: Arc<Columns>,
    /// The matrix as factorized: equilibrated in place, which a solve reads.
    values: Vec<c64>,
    starts: Vec<c_int>,
    rows: Vec<c_int>,
    a: Buffer,
    l: Buffer,
    u: Buffer,
    options: Buffer,
    perm_c: Vec<c_int>,
    perm_r: Vec<c_int>,
    etree: Vec<c_int>,
    equed: [c_char; 2],
    r: Vec<f64>,
    c: Vec<f64>,
    glu: Buffer,
    /// Whether L and U were made, and so must be destroyed.
    factorized: bool,
    report: Report,
}

// SAFETY: an engine is used by one thread at a time (behind a Mutex), every call into SuperLU
// is under one lock, and its arrays are owned here
unsafe impl Send for Engine {}

impl Engine {
    fn new(api: &Arc<Api>, columns: &Arc<Columns>, values: Vec<c64>) -> Result<Engine> {
        let n = columns.n as usize;
        let mut engine = Engine {
            api: api.clone(),
            columns: columns.clone(),
            values,
            starts: columns.starts.clone(),
            rows: columns.rows.clone(),
            a: Buffer::new(64),
            l: Buffer::new(64),
            u: Buffer::new(64),
            options: Buffer::new(2 * options::SIZE),
            perm_c: columns.perm_c.clone(),
            perm_r: vec![0; n],
            etree: vec![0; n],
            equed: [b'N' as c_char, 0],
            r: vec![0.0; n],
            c: vec![0.0; n],
            glu: Buffer::new(2048),
            factorized: false,
            report: Report::default(),
        };
        let start = Instant::now();
        let nnz = engine.rows.len() as c_int;
        let mut stat = Buffer::new(64);
        let mut memory = Buffer::new(16);
        // no right-hand side: the factorization alone
        let (mut b, mut x) = (Buffer::new(64), Buffer::new(64));
        let mut none = [c64::new(0.0, 0.0); 1];
        let (mut growth, mut rcond, mut ferr, mut berr) = (0.0, 0.0, [0.0], [0.0]);
        let mut info: c_int = 0;
        {
            let _one = CALLS.lock().unwrap_or_else(|p| p.into_inner());
            let api = &engine.api;
            // SAFETY: the buffers are zeroed and larger than SuperLU's structures; the arrays
            // have the lengths its documentation asks (n for the permutations, the tree and
            // the scalings, nnz for the entries), and live in the engine as long as A does
            unsafe {
                (api.set_default_options)(engine.options.pointer());
                (api.create_compcol)(
                    engine.a.pointer(),
                    columns.n,
                    columns.n,
                    nnz,
                    engine.values.as_mut_ptr(),
                    engine.rows.as_mut_ptr(),
                    engine.starts.as_mut_ptr(),
                    SLU_NC,
                    SLU_Z,
                    SLU_GE,
                );
                (api.create_dense)(
                    b.pointer(),
                    columns.n,
                    0,
                    none.as_mut_ptr(),
                    columns.n,
                    SLU_DN,
                    SLU_Z,
                    SLU_GE,
                );
                (api.create_dense)(
                    x.pointer(),
                    columns.n,
                    0,
                    none.as_mut_ptr(),
                    columns.n,
                    SLU_DN,
                    SLU_Z,
                    SLU_GE,
                );
            }
            engine.options.set_int(options::FACT, DOFACT);
            engine.options.set_int(options::EQUIL, YES);
            engine
                .options
                .set_int(options::COL_PERM, my_permc(api.major));
            engine.options.set_int(options::TRANS, NOTRANS);
            engine.options.set_int(options::ITER_REFINE, REFINE_DOUBLE);
            engine.options.set_int(options::PIVOT_GROWTH, NO);
            engine.options.set_int(options::CONDITION_NUMBER, NO);
            engine.options.set_int(options::PRINT_STAT, NO);
            // SAFETY: as above; zgssvx with no right-hand side factorizes, and leaves L and U
            unsafe {
                (api.stat_init)(stat.pointer());
                (api.zgssvx)(
                    engine.options.pointer(),
                    engine.a.pointer(),
                    engine.perm_c.as_mut_ptr(),
                    engine.perm_r.as_mut_ptr(),
                    engine.etree.as_mut_ptr(),
                    engine.equed.as_mut_ptr(),
                    engine.r.as_mut_ptr(),
                    engine.c.as_mut_ptr(),
                    engine.l.pointer(),
                    engine.u.pointer(),
                    std::ptr::null_mut(),
                    0,
                    b.pointer(),
                    x.pointer(),
                    &mut growth,
                    &mut rcond,
                    ferr.as_mut_ptr(),
                    berr.as_mut_ptr(),
                    engine.glu.pointer(),
                    memory.pointer(),
                    stat.pointer(),
                    &mut info,
                );
                (api.stat_free)(stat.pointer());
                (api.destroy_store)(b.pointer());
                (api.destroy_store)(x.pointer());
            }
        }
        // (SuperLU allocates L and U before it can find a zero pivot)
        engine.factorized = engine.l.address(supermatrix::STORE) != 0;
        check(info, columns.n, "factorization")?;
        // nnz is each store's first field, an int_t
        let entries = |m: &Buffer| -> usize {
            let store = m.address(supermatrix::STORE) as *const c_int;
            // SAFETY: SCformat's and NCformat's first field, of the store SuperLU allocated
            (unsafe { store.read() }).max(0) as usize
        };
        engine.report.factor_entries = Some(entries(&engine.l) + entries(&engine.u));
        engine.report.peak_memory_bytes = Some(memory.float(TOTAL_NEEDED).max(0.0) as u64);
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
        // zgssvx scales the right-hand side in place and writes the solution beside it
        let mut b = rhs.to_vec();
        let mut x = vec![c64::new(0.0, 0.0); rhs.len()];
        let (mut bm, mut xm) = (Buffer::new(64), Buffer::new(64));
        let mut stat = Buffer::new(64);
        let mut memory = Buffer::new(16);
        let (mut growth, mut rcond, mut ferr, mut berr) = (0.0, 0.0, [0.0], [0.0]);
        let mut info: c_int = 0;
        self.options.set_int(options::FACT, FACTORED);
        self.options
            .set_int(options::TRANS, if transpose { TRANS } else { NOTRANS });
        let steps;
        {
            let _one = CALLS.lock().unwrap_or_else(|p| p.into_inner());
            let api = &self.api;
            // SAFETY: the factors, permutations and scalings are the ones the factorization
            // left, A the matrix it equilibrated; B and X are n by 1 over arrays of n
            unsafe {
                (api.create_dense)(bm.pointer(), n, 1, b.as_mut_ptr(), n, SLU_DN, SLU_Z, SLU_GE);
                (api.create_dense)(xm.pointer(), n, 1, x.as_mut_ptr(), n, SLU_DN, SLU_Z, SLU_GE);
                (api.stat_init)(stat.pointer());
                (api.zgssvx)(
                    self.options.pointer(),
                    self.a.pointer(),
                    self.perm_c.as_mut_ptr(),
                    self.perm_r.as_mut_ptr(),
                    self.etree.as_mut_ptr(),
                    self.equed.as_mut_ptr(),
                    self.r.as_mut_ptr(),
                    self.c.as_mut_ptr(),
                    self.l.pointer(),
                    self.u.pointer(),
                    std::ptr::null_mut(),
                    0,
                    bm.pointer(),
                    xm.pointer(),
                    &mut growth,
                    &mut rcond,
                    ferr.as_mut_ptr(),
                    berr.as_mut_ptr(),
                    self.glu.pointer(),
                    memory.pointer(),
                    stat.pointer(),
                    &mut info,
                );
                steps = stat.int(REFINE_STEPS);
                (api.stat_free)(stat.pointer());
                (api.destroy_store)(bm.pointer());
                (api.destroy_store)(xm.pointer());
            }
        }
        let _ = steps;
        check(info, n, "solve")?;
        Ok(x)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _one = CALLS.lock().unwrap_or_else(|p| p.into_inner());
        // SAFETY: L and U are SuperLU's to free, A's store too (its arrays are ours)
        unsafe {
            if self.factorized {
                (self.api.destroy_supernode)(self.l.pointer());
                (self.api.destroy_compcol)(self.u.pointer());
            }
            if self.a.address(supermatrix::STORE) != 0 {
                (self.api.destroy_store)(self.a.pointer());
            }
        }
    }
}

/// SuperLU, as photonoxide's registry has it.
pub struct SuperLu {
    api: Arc<Api>,
}

impl SuperLu {
    /// SuperLU's sequential library found, loaded, of a release and an index width
    /// photonoxide calls.
    ///
    /// # Errors
    ///
    /// Why not: not found, not loadable, another release, 64-bit indices.
    pub fn load() -> std::result::Result<SuperLu, String> {
        let (discovery, library) = load(&SUPERLU, None, |l| supported(l.path()).map(drop));
        let library = library.ok_or_else(|| discovery.reason())?;
        let api = open(library).map_err(|e| e.to_string())?;
        Ok(SuperLu { api: Arc::new(api) })
    }

    /// Its release, as its file's name gives it.
    pub fn version(&self) -> &str {
        &self.api.version
    }
}

/// SuperLU: where it was found and its release.
pub fn probe() -> Probe {
    let (discovery, library) = load(&SUPERLU, None, |l| supported(l.path()).map(drop));
    let version = library
        .as_ref()
        .and_then(|l| supported(l.path()).ok())
        .map(|(version, _)| version);
    Probe {
        discovery,
        version,
        details: Vec::new(),
    }
}

impl DirectSolver for SuperLu {
    fn capabilities(&self) -> Capabilities {
        let mut c = Capabilities::new(
            "superlu",
            self.api.version.clone(),
            "BSD-3-Clause (installed by the user)",
        );
        c.symmetric = false;
        c.transpose = true;
        c.threads = Threads::One;
        c.deterministic = false;
        c
    }

    fn analyse(&self, matrix: &Matrix<'_>) -> Result<Option<Arc<dyn Analysis>>> {
        if matrix.form() != Form::General {
            return Err(error("SuperLU takes general matrices only"));
        }
        let start = Instant::now();
        let n = c_int::try_from(matrix.n())
            .map_err(|_| error("SuperLU's indices are 32-bit: too many unknowns"))?;
        let mut starts = narrowed(matrix.column_starts(), "the entry")?;
        let mut rows = narrowed(matrix.row_indices(), "the row")?;
        let mut perm_c = vec![0; matrix.n()];
        let mut a = Buffer::new(64);
        // the ordering reads the structure alone
        let mut values = matrix.values().to_vec();
        {
            let _one = CALLS.lock().unwrap_or_else(|p| p.into_inner());
            // SAFETY: A over arrays of the lengths given; get_perm_c writes n entries of
            // perm_c; A's store is freed and its arrays are ours
            unsafe {
                (self.api.create_compcol)(
                    a.pointer(),
                    n,
                    n,
                    rows.len() as c_int,
                    values.as_mut_ptr(),
                    rows.as_mut_ptr(),
                    starts.as_mut_ptr(),
                    SLU_NC,
                    SLU_Z,
                    SLU_GE,
                );
                (self.api.get_perm_c)(COLAMD, a.pointer(), perm_c.as_mut_ptr());
                (self.api.destroy_store)(a.pointer());
            }
        }
        Ok(Some(Arc::new(Analysed {
            api: self.api.clone(),
            columns: Arc::new(Columns {
                n,
                starts,
                rows,
                perm_c,
            }),
            seconds: start.elapsed().as_secs_f64(),
        })))
    }
}

struct Analysed {
    api: Arc<Api>,
    columns: Arc<Columns>,
    seconds: f64,
}

impl Analysis for Analysed {
    fn form(&self) -> Form {
        Form::General
    }

    fn factorize(&self, matrix: &Matrix<'_>) -> Result<Box<dyn Factorization>> {
        let same = matrix.n() == self.columns.n as usize
            && matrix.values().len() == self.columns.rows.len()
            && matrix
                .column_starts()
                .iter()
                .zip(&self.columns.starts)
                .all(|(&a, &b)| a == b as usize);
        if !same || matrix.form() != Form::General {
            return Err(error("a matrix of another structure than the one analysed"));
        }
        let mut engine = Engine::new(&self.api, &self.columns, matrix.values().to_vec())?;
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
    error("SuperLU's factors' lock was poisoned by a panic")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_files_name_gives_its_release() {
        for (name, version, major) in [
            ("libsuperlu.so.7.0.0", "7.0.0", 7),
            ("libsuperlu.so.6.0.1", "6.0.1", 6),
            ("libsuperlu.so.5", "5", 5),
            ("libsuperlu.7.dylib", "7", 7),
            ("libsuperlu.7.0.0.dylib", "7.0.0", 7),
        ] {
            assert_eq!(
                release(Path::new(name)),
                Some((version.to_owned(), major)),
                "{name}"
            );
            assert!(supported(Path::new(name)).is_ok());
        }
        // no release in the name, and one photonoxide doesn't call
        assert_eq!(release(Path::new("libsuperlu.so")), None);
        assert_eq!(release(Path::new("superlu.dll")), None);
        assert!(supported(Path::new("libsuperlu.so")).is_err());
        let old = supported(Path::new("libsuperlu.so.4.3.0"))
            .unwrap_err()
            .to_string();
        assert!(old.contains("4.3.0") && old.contains("5, 6 and 7"), "{old}");
    }

    #[test]
    fn the_given_ordering_is_the_releases_constant() {
        // colperm_t ends ..., PARMETIS, ZOLTAN, MY_PERMC in 5.3.0, and ..., PARMETIS,
        // METIS_ATA, ZOLTAN, MY_PERMC from 6.0.0
        assert_eq!(my_permc(5), 7);
        assert_eq!(my_permc(6), 8);
        assert_eq!(my_permc(7), 8);
    }

    #[test]
    fn the_options_fields_are_where_the_header_puts_them() {
        // Fact, Equil, ColPerm, Trans, IterRefine: five enums; DiagPivotThresh on 8 bytes at
        // 24; SymmetricMode, PivotGrowth, ConditionNumber, RowPerm, ILU_DropRule: five ints
        // from 32; ILU_DropTol and ILU_FillFactor at 56 and 64; ILU_Norm at 72; ILU_FillTol
        // at 80; ILU_MILU at 88; ILU_MILU_Dim at 96; then nine ints from 104
        assert_eq!(
            (
                options::FACT,
                options::EQUIL,
                options::COL_PERM,
                options::TRANS,
                options::ITER_REFINE
            ),
            (0, 4, 8, 12, 16)
        );
        assert_eq!(
            (options::PIVOT_GROWTH, options::CONDITION_NUMBER),
            (32 + 4, 32 + 8)
        );
        // ParSymbFact, ReplaceTinyPivot, SolveInitialized, RefineInitialized, PrintStat
        assert_eq!(options::PRINT_STAT, 104 + 4 * 4);
        // nnzL, nnzU, num_lookaheads, lookahead_etree, SymPattern after it
        assert_eq!(options::SIZE, options::PRINT_STAT + 4 * 6);
        // SuperMatrix: three enums, two ints, a pointer on 8 bytes
        assert_eq!(
            (supermatrix::NROW, supermatrix::NCOL, supermatrix::STORE),
            (12, 16, 24)
        );
        let mut b = Buffer::new(64);
        b.set_int(options::TRANS, TRANS);
        assert_eq!(b.int(options::TRANS), 1);
        b.set_int(TOTAL_NEEDED, 1.5f32.to_bits() as i32);
        assert_eq!(b.float(TOTAL_NEEDED), 1.5);
    }

    #[test]
    fn an_info_has_its_meaning() {
        assert!(check(0, 10, "solve").is_ok());
        assert!(
            check(-3, 10, "solve")
                .unwrap_err()
                .to_string()
                .contains("argument 3")
        );
        let singular = check(7, 10, "factorization").unwrap_err().to_string();
        assert!(
            singular.contains("column 7") && singular.contains("singular"),
            "{singular}"
        );
        assert!(
            check(4096, 10, "factorization")
                .unwrap_err()
                .to_string()
                .contains("memory")
        );
    }
}
