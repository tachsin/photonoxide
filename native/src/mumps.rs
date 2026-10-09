//! MUMPS as a [`DirectSolver`] (#176): P. R. Amestoy, I. S. Duff, J.-Y. L'Excellent, J. Koster,
//! "A fully asynchronous multifrontal solver using distributed dynamic scheduling", SIAM J.
//! Matrix Anal. Appl. 23, 15 (2001), doi:10.1137/S0895479899358194; P. R. Amestoy, A. Buttari,
//! J.-Y. L'Excellent, T. Mary, "Performance and scalability of the block low-rank multifrontal
//! factorization on multicore architectures", ACM Trans. Math. Softw. 45, 2 (2019),
//! doi:10.1145/3242094. MUMPS asks that work using it cite these.
//!
//! The sequential build (`libzmumps_seq`; `zmumps.dll` in conda-forge's `mumps-seq` on
//! Windows), through its C interface, `zmumps_c`, installed by the user under the CeCILL-C
//! licence and never redistributed.
//!
//! - **The structure.** `zmumps_c` takes one structure, `ZMUMPS_STRUC_C`, whose layout changes
//!   between releases. It holds only 32-bit integers, 64-bit integers, doubles, pointers and
//!   characters, so it is the same on every 64-bit platform, and the fields' offsets here were
//!   computed from each release's own `zmumps_c.h` by C's layout rules: one layout for 5.4.1,
//!   5.5.1 and 5.6.0 to 5.6.2 (8 360 bytes), another for 5.7.0 to 5.8.2 (10 928). The fields up
//!   to the matrix are at the same places in both. MUMPS writes its version into the structure
//!   when an instance is made; it is read at each layout's place for it, and a library whose
//!   version isn't one of those isn't used.
//! - **The forms.** [`Form::General`] is `SYM` = 0, an LU; [`Form::Symmetric`] is `SYM` = 2, an
//!   L D Lᵀ of a complex symmetric matrix, given its lower triangle. The matrix is given
//!   assembled, by entries, with one-based 32-bit indices: a matrix of 2³¹ unknowns or more is
//!   refused.
//! - **Phases:** an instance is made (`JOB` = −1) and analysed (1) once per structure, with
//!   the values, which MUMPS's scaling and matching read. Each factorization (2) takes an
//!   analysed instance from a pool and gives it back when dropped, so a sweep analyses once.
//!   The solve (3) with the transpose is `ICNTL(9)` = 2. The instance is ended (−2) when
//!   dropped.
//! - **Settings:** MUMPS's defaults, its choice of ordering among them (`ICNTL(7)` = 7), and
//!   no output (`ICNTL(1..4)`). A factorization that runs out of its estimated workspace
//!   (errors −8 and −9) is tried again with twice the margin (`ICNTL(14)`), up to three times.
//! - **Report:** the factors' entries (`INFOG(9)`), the memory used during the factorization
//!   (`INFOG(22)`, in millions of bytes), and the pivots static pivoting modified
//!   (`INFOG(25)`).
//! - **One call at a time, on a stack of its own.** The sequential build keeps state of its
//!   own; every call into it is made under one lock, on a thread started for it with 256 MB of
//!   stack: a build with OpenMP (conda-forge's on Linux) keeps its local arrays there, and
//!   crashed on the 2 MB of a thread Rust starts.
//! - **Threads and determinism:** the sequential build's only threads are its BLAS's, set by
//!   the BLAS (`OMP_NUM_THREADS`, `OPENBLAS_NUM_THREADS`). Not declared deterministic.
//! - **Errors:** `INFOG(1)` with `INFOG(2)`, with the user guide's meanings.
//!
//! Block low-rank factorization (`ICNTL(35)`) isn't offered yet.

use std::ffi::c_void;
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

/// MUMPS's sequential complex library.
pub const MUMPS: Spec = Spec {
    name: "MUMPS",
    files: if cfg!(windows) {
        &["zmumps.dll", "libzmumps.dll"]
    } else if cfg!(target_os = "macos") {
        &["libzmumps_seq.dylib"]
    } else {
        // the development package's name, then the runtime packages' (Debian's, by version)
        &[
            "libzmumps_seq.so",
            "libzmumps_seq-5.8.so",
            "libzmumps_seq-5.7.so",
            "libzmumps_seq-5.6.so",
            "libzmumps_seq-5.5.so",
            "libzmumps_seq-5.4.so",
        ]
    },
    variables: &["PHOTONOXIDE_MUMPS"],
    install: Vec::new,
    wheel: None,
};

/// `void zmumps_c(ZMUMPS_STRUC_C *)`.
type ZmumpsC = unsafe extern "C" fn(*mut c_void);

/// The fields at the same place in every supported release (`zmumps_c.h`).
const SYM: usize = 0;
const PAR: usize = 4;
const JOB: usize = 8;
const COMM_FORTRAN: usize = 12;
const ICNTL: usize = 16;
const N: usize = 5416;
const NNZ: usize = 5432;
const IRN: usize = 5440;
const JCN: usize = 5448;
const A: usize = 5456;

/// The fields that moved, and the structure's size.
#[derive(Debug, PartialEq, Eq)]
struct Layout {
    size: usize,
    rhs: usize,
    nrhs: usize,
    lrhs: usize,
    infog: usize,
    version_number: usize,
    /// The releases whose header gives these offsets.
    versions: &'static [&'static str],
}

const OLD: Layout = Layout {
    size: 8360,
    rhs: 5600,
    nrhs: 5672,
    lrhs: 5676,
    infog: 6048,
    version_number: 7072,
    versions: &["5.4.1", "5.5.1", "5.6.0", "5.6.1", "5.6.2"],
};

const NEW: Layout = Layout {
    size: 10928,
    rhs: 5640,
    nrhs: 5736,
    lrhs: 5740,
    infog: 6120,
    version_number: 7144,
    versions: &[
        "5.7.0", "5.7.1", "5.7.2", "5.7.3", "5.8.0", "5.8.1", "5.8.2",
    ],
};

/// The structure is given this much room whatever its release: a release photonoxide doesn't
/// know must not write past it while its version is read.
const ROOM: usize = 4 * NEW.size;

const INIT: i32 = -1;
const END: i32 = -2;
const ANALYSIS: i32 = 1;
const FACTORIZATION: i32 = 2;
const SOLVE: i32 = 3;
/// The communicator the sequential build takes (`USE_COMM_WORLD` in MUMPS's examples).
const USE_COMM_WORLD: i32 = -987_654;

/// One call into MUMPS at a time.
static CALLS: Mutex<()> = Mutex::new(());

/// The stack each call into MUMPS is given, in bytes. A build with OpenMP (conda-forge's on
/// Linux) keeps its routines' local arrays on the stack, more than the 2 MB of a thread Rust
/// starts: it crashed there on a 2 × 2 matrix. Reserved, not used, until MUMPS takes it.
const STACK: usize = 256 << 20;

/// A pointer that may cross to the thread a call is made on.
struct Crossing(*mut c_void);

// SAFETY: the caller waits for the thread, which alone uses the pointer meanwhile
unsafe impl Send for Crossing {}

/// `zmumps(structure)`, alone among the calls into MUMPS and on a stack of [`STACK`].
///
/// # Safety
///
/// As `zmumps_c`'s: `structure` is a `ZMUMPS_STRUC_C` with room for its release's, and the
/// arrays it points at live through the call.
unsafe fn run(zmumps: ZmumpsC, structure: *mut c_void) -> Result<()> {
    let _one = CALLS.lock().unwrap_or_else(|p| p.into_inner());
    let crossing = Crossing(structure);
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("mumps".into())
            .stack_size(STACK)
            .spawn_scoped(scope, move || {
                let crossing = crossing;
                // SAFETY: the caller's
                unsafe { zmumps(crossing.0) }
            })
            .map_err(|e| error(format!("a thread for MUMPS's call couldn't start: {e}")))?
            .join()
            .map_err(|_| error("MUMPS's call panicked"))
    })
}

/// What `INFOG(1)` means, from the user guide's list of errors.
fn meaning(code: i32) -> &'static str {
    match code {
        -2 => "the number of entries is out of range",
        -3 => "an invalid job, or one out of order",
        -5 => "it couldn't allocate its workspace for the analysis",
        -6 => "the matrix is singular in structure",
        -7 => "it couldn't allocate its integer workspace for the analysis",
        -8 => "its integer workspace is too small for the factorization",
        -9 => "its workspace is too small for the factorization",
        -10 => "the matrix is numerically singular, or a pivot is zero",
        -13 => "it couldn't allocate its workspace",
        -16 => "the number of unknowns is out of range",
        -22 => "an array it was given isn't allocated or is too small",
        -51 | -52 => "a count passes the 32-bit integers it was built with",
        -53 => "the matrix isn't the one analysed",
        _ => "see MUMPS's user guide, \"Error and warning diagnostics\"",
    }
}

struct Api {
    zmumps: ZmumpsC,
    version: String,
    layout: &'static Layout,
    // last: the function above must not outlive it
    _library: Library,
}

/// A matrix's entries as MUMPS takes them: one-based rows and columns.
#[derive(Debug, PartialEq, Eq)]
struct Entries {
    n: i32,
    rows: Vec<i32>,
    columns: Vec<i32>,
    /// Each entry's place in the matrix's values.
    order: Vec<usize>,
    /// The matrix's entries as given (both triangles of a symmetric one).
    entries: usize,
}

impl Entries {
    /// A general matrix's entries, or a symmetric one's on and below its diagonal.
    fn new(cs: &[usize], ri: &[usize], lower: bool) -> Result<Entries> {
        let n = cs.len() - 1;
        let n32 = i32::try_from(n).map_err(|_| {
            error(format!(
                "MUMPS's indices are 32-bit: {n} unknowns are too many"
            ))
        })?;
        let (mut rows, mut columns, mut order) = (Vec::new(), Vec::new(), Vec::new());
        for j in 0..n {
            for e in cs[j]..cs[j + 1] {
                if lower && ri[e] < j {
                    continue;
                }
                // (both below 2³¹: n is)
                rows.push(ri[e] as i32 + 1);
                columns.push(j as i32 + 1);
                order.push(e);
            }
        }
        Ok(Entries {
            n: n32,
            rows,
            columns,
            order,
            entries: ri.len(),
        })
    }

    fn values(&self, of: &[c64]) -> Vec<c64> {
        self.order.iter().map(|&e| of[e]).collect()
    }
}

/// `ZMUMPS_STRUC_C`, as bytes at the offsets of its release's header.
struct Structure {
    /// 8-byte words: the structure's alignment.
    words: Vec<u64>,
}

impl Structure {
    fn new() -> Structure {
        Structure {
            words: vec![0; ROOM / 8],
        }
    }

    fn pointer(&mut self) -> *mut c_void {
        self.words.as_mut_ptr().cast()
    }

    fn int(&self, offset: usize) -> i32 {
        debug_assert!(offset.is_multiple_of(4) && offset + 4 <= ROOM);
        // SAFETY: inside the buffer, aligned for an i32 (the buffer is of u64)
        unsafe {
            self.words
                .as_ptr()
                .cast::<u8>()
                .add(offset)
                .cast::<i32>()
                .read()
        }
    }

    fn set_int(&mut self, offset: usize, value: i32) {
        debug_assert!(offset.is_multiple_of(4) && offset + 4 <= ROOM);
        // SAFETY: as `int`
        unsafe {
            self.words
                .as_mut_ptr()
                .cast::<u8>()
                .add(offset)
                .cast::<i32>()
                .write(value);
        }
    }

    fn set_long(&mut self, offset: usize, value: i64) {
        debug_assert!(offset.is_multiple_of(8) && offset + 8 <= ROOM);
        self.words[offset / 8] = value as u64;
    }

    fn set_pointer<T>(&mut self, offset: usize, to: *mut T) {
        debug_assert!(offset.is_multiple_of(8) && offset + 8 <= ROOM);
        self.words[offset / 8] = to as usize as u64;
    }

    /// `ICNTL(k)`, as the user guide numbers it.
    fn set_icntl(&mut self, k: usize, value: i32) {
        self.set_int(ICNTL + 4 * (k - 1), value);
    }

    /// `INFOG(k)`.
    fn infog(&self, layout: &Layout, k: usize) -> i32 {
        self.int(layout.infog + 4 * (k - 1))
    }

    /// The text at `offset`, up to its NUL, if it is printable.
    fn text(&self, offset: usize, most: usize) -> String {
        let bytes: Vec<u8> = (0..most)
            .map(|i| (self.words[(offset + i) / 8] >> (8 * ((offset + i) % 8))) as u8)
            .take_while(|&b| b != 0)
            .collect();
        String::from_utf8_lossy(&bytes).trim().to_owned()
    }
}

/// Whether `text` is a release's number: digits and dots, three parts.
fn is_version(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// The layout of the release `version`, if photonoxide has its header's.
fn layout_of(version: &str) -> Option<&'static Layout> {
    [&OLD, &NEW]
        .into_iter()
        .find(|l| l.versions.contains(&version))
}

/// Calls MUMPS with `job` on `structure`, under the lock: `INFOG(1)` and `INFOG(2)` as an
/// error if negative.
fn call(api: &Api, structure: &mut Structure, job: i32, what: &str) -> Result<()> {
    structure.set_int(JOB, job);
    // SAFETY: the structure is a zeroed buffer larger than any release's, its fields set at
    // this release's offsets, and the arrays its pointers name outlive the call
    unsafe { run(api.zmumps, structure.pointer()) }?;
    let code = structure.infog(api.layout, 1);
    if code < 0 {
        return Err(error(format!(
            "MUMPS's {what} failed with INFOG(1) = {code}, INFOG(2) = {}: {}",
            structure.infog(api.layout, 2),
            meaning(code)
        )));
    }
    Ok(())
}

/// One MUMPS instance, analysed for one structure.
struct Engine {
    api: Arc<Api>,
    entries: Arc<Entries>,
    structure: Structure,
    /// The values MUMPS holds a pointer to between calls.
    values: Vec<c64>,
    symmetric: bool,
    /// Whether the instance was made, and so must be ended.
    made: bool,
}

// SAFETY: an instance is used by one thread at a time (behind a Mutex, or owned), every call
// into MUMPS is under one lock, and its arrays are owned here
unsafe impl Send for Engine {}

impl Engine {
    /// An instance for this structure, analysed with these values, in the seconds returned.
    fn new(
        api: &Arc<Api>,
        entries: &Arc<Entries>,
        form: Form,
        values: Vec<c64>,
    ) -> Result<(Engine, f64)> {
        let start = Instant::now();
        let mut engine = Engine {
            api: api.clone(),
            entries: entries.clone(),
            structure: Structure::new(),
            values,
            symmetric: form == Form::Symmetric,
            made: false,
        };
        let s = &mut engine.structure;
        s.set_int(SYM, if form == Form::Symmetric { 2 } else { 0 });
        s.set_int(PAR, 1);
        s.set_int(COMM_FORTRAN, USE_COMM_WORLD);
        call(api, s, INIT, "initialization")?;
        engine.made = true;
        let s = &mut engine.structure;
        // no output: the three streams off, and no messages
        for k in 1..=3 {
            s.set_icntl(k, -1);
        }
        s.set_icntl(4, 0);
        s.set_int(N, entries.n);
        s.set_long(NNZ, entries.rows.len() as i64);
        // MUMPS reads the indices and doesn't write them
        s.set_pointer(IRN, entries.rows.as_ptr().cast_mut());
        s.set_pointer(JCN, entries.columns.as_ptr().cast_mut());
        s.set_pointer(A, engine.values.as_mut_ptr());
        call(api, &mut engine.structure, ANALYSIS, "analysis")?;
        Ok((engine, start.elapsed().as_secs_f64()))
    }

    /// Factorizes `values`, with more room if MUMPS's estimate falls short.
    fn factorize(&mut self, values: Vec<c64>) -> Result<()> {
        self.values = values;
        let at = self.values.as_mut_ptr();
        self.structure.set_pointer(A, at);
        let mut margin = 20;
        loop {
            match call(
                &self.api,
                &mut self.structure,
                FACTORIZATION,
                "factorization",
            ) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    let code = self.structure.infog(self.api.layout, 1);
                    if !matches!(code, -8 | -9) || margin >= 160 {
                        return Err(e);
                    }
                    // the estimated workspace was too small: twice the margin
                    margin *= 2;
                    self.structure.set_icntl(14, margin);
                }
            }
        }
    }

    fn solve(&mut self, b: &[c64], transpose: bool) -> Result<Vec<c64>> {
        if b.len() != self.entries.n as usize {
            return Err(error(format!(
                "a right-hand side of {} for a matrix of {} unknowns",
                b.len(),
                self.entries.n
            )));
        }
        let mut x = b.to_vec();
        let layout = self.api.layout;
        self.structure.set_pointer(layout.rhs, x.as_mut_ptr());
        self.structure.set_int(layout.nrhs, 1);
        self.structure.set_int(layout.lrhs, self.entries.n);
        // 1: A x = b; anything else: Aᵀ x = b (a symmetric matrix is its own transpose)
        self.structure
            .set_icntl(9, if transpose && !self.symmetric { 2 } else { 1 });
        let solved = call(&self.api, &mut self.structure, SOLVE, "solve");
        self.structure
            .set_pointer::<c64>(layout.rhs, std::ptr::null_mut());
        solved.map(|()| x)
    }

    /// The factors' entries, the memory the factorization used in bytes, and the pivots static
    /// pivoting modified.
    fn report(&self) -> (Option<usize>, Option<u64>, usize) {
        let info = |k: usize| self.structure.infog(self.api.layout, k);
        // negative: millions of entries
        let entries = match info(9) {
            e if e >= 0 => e as usize,
            e => (-(e as i64)) as usize * 1_000_000,
        };
        (
            Some(entries),
            Some(info(22).max(0) as u64 * 1_000_000),
            info(25).max(0) as usize,
        )
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if self.made {
            let _ = call(&self.api, &mut self.structure, END, "end");
        }
    }
}

/// MUMPS, as photonoxide's registry has it.
pub struct Mumps {
    api: Arc<Api>,
}

/// The version an instance of this library says it is, and that release's layout.
fn identify(library: &Library) -> Result<(ZmumpsC, String, &'static Layout)> {
    // SAFETY: `void zmumps_c(ZMUMPS_STRUC_C *)`, as zmumps_c.h declares it
    let zmumps: ZmumpsC = unsafe { library.function("zmumps_c")? };
    let mut s = Structure::new();
    s.set_int(SYM, 0);
    s.set_int(PAR, 1);
    s.set_int(COMM_FORTRAN, USE_COMM_WORLD);
    s.set_int(JOB, INIT);
    // SAFETY: a zeroed buffer four times the largest known release's structure, with the four
    // fields an initialization reads, which every release has first
    unsafe { run(zmumps, s.pointer()) }?;
    // the version, where each layout has it: 30 characters and a NUL
    let found: Vec<(&'static Layout, String)> = [&OLD, &NEW]
        .into_iter()
        .map(|l| (l, s.text(l.version_number, 31)))
        .filter(|(_, v)| is_version(v))
        .collect();
    let identified = match found.as_slice() {
        [(layout, version)] if layout_of(version) == Some(*layout) => {
            Ok((zmumps, version.clone(), *layout))
        }
        [(_, version)] => Err(error(format!(
            "MUMPS {version}: photonoxide has the structure's layout of {} and {}, not this \
             release's",
            OLD.versions.join(", "),
            NEW.versions.join(", ")
        ))),
        _ => Err(error(
            "its version isn't where MUMPS 5.4 to 5.8 write it: another release, or not the \
             sequential complex library",
        )),
    };
    // the instance is ended, whichever release it is: the job is at the same place in all
    if !found.is_empty() {
        // quietly: the streams are among the fields every release has at the same place
        for k in 1..=3 {
            s.set_icntl(k, -1);
        }
        s.set_icntl(4, 0);
        s.set_int(JOB, END);
        // SAFETY: the instance made above, ended: the job is the structure's third field
        let _ = unsafe { run(zmumps, s.pointer()) };
    }
    identified
}

impl Mumps {
    /// MUMPS's sequential complex library found, loaded and of a release whose structure
    /// photonoxide knows.
    ///
    /// # Errors
    ///
    /// Why not: not found, not loadable, another release.
    pub fn load() -> std::result::Result<Mumps, String> {
        let (discovery, library) = load(&MUMPS, None, |l| identify(l).map(drop));
        let library = library.ok_or_else(|| discovery.reason())?;
        let (zmumps, version, layout) = identify(&library).map_err(|e| e.to_string())?;
        Ok(Mumps {
            api: Arc::new(Api {
                zmumps,
                version,
                layout,
                _library: library,
            }),
        })
    }

    /// Its release.
    pub fn version(&self) -> &str {
        &self.api.version
    }
}

/// MUMPS: where it was found and its release.
pub fn probe() -> Probe {
    let (discovery, library) = load(&MUMPS, None, |l| identify(l).map(drop));
    let version = library
        .as_ref()
        .and_then(|l| identify(l).ok())
        .map(|(_, version, _)| version);
    Probe {
        discovery,
        version,
        details: Vec::new(),
    }
}

impl DirectSolver for Mumps {
    fn capabilities(&self) -> Capabilities {
        let mut c = Capabilities::new(
            "mumps",
            self.api.version.clone(),
            "CeCILL-C (installed by the user; cite Amestoy et al. 2001, \
             doi:10.1137/S0895479899358194, and 2019, doi:10.1145/3242094)",
        );
        c.symmetric = true;
        c.transpose = true;
        c.threads = Threads::Library(
            "its BLAS's own threads (OMP_NUM_THREADS, OPENBLAS_NUM_THREADS): the sequential \
             build has no others"
                .into(),
        );
        c.deterministic = false;
        c
    }

    fn analyse(&self, matrix: &Matrix<'_>) -> Result<Option<Arc<dyn Analysis>>> {
        let symmetric = matrix.form() == Form::Symmetric;
        let entries = Arc::new(Entries::new(
            matrix.column_starts(),
            matrix.row_indices(),
            symmetric,
        )?);
        let values = entries.values(matrix.values());
        let (engine, seconds) = Engine::new(&self.api, &entries, matrix.form(), values)?;
        Ok(Some(Arc::new(Analysed {
            api: self.api.clone(),
            form: matrix.form(),
            entries,
            pool: Arc::new(Mutex::new(vec![engine])),
            seconds,
        })))
    }
}

struct Analysed {
    api: Arc<Api>,
    form: Form,
    entries: Arc<Entries>,
    pool: Arc<Mutex<Vec<Engine>>>,
    seconds: f64,
}

fn poisoned<T>(_: T) -> photonoxide::Error {
    error("a MUMPS instance's lock was poisoned by a panic")
}

impl Analysis for Analysed {
    fn form(&self) -> Form {
        self.form
    }

    fn factorize(&self, matrix: &Matrix<'_>) -> Result<Box<dyn Factorization>> {
        if matrix.values().len() != self.entries.entries || matrix.n() != self.entries.n as usize {
            return Err(error("a matrix of another structure than the one analysed"));
        }
        let values = self.entries.values(matrix.values());
        let pooled = self.pool.lock().map_err(poisoned)?.pop();
        let (mut engine, analysis_seconds) = match pooled {
            Some(engine) => (engine, self.seconds),
            None => Engine::new(&self.api, &self.entries, self.form, values.clone())?,
        };
        let start = Instant::now();
        // a failed factorization's instance goes back analysed
        let factorized = engine.factorize(values);
        let factorization_seconds = start.elapsed().as_secs_f64();
        if let Err(e) = factorized {
            self.pool.lock().map_err(poisoned)?.push(engine);
            return Err(e);
        }
        let (factor_entries, peak, perturbed_pivots) = engine.report();
        let mut report = Report::default();
        report.factor_entries = factor_entries;
        report.peak_memory_bytes = peak;
        report.analysis_seconds = analysis_seconds;
        report.factorization_seconds = factorization_seconds;
        report.perturbed_pivots = perturbed_pivots;
        Ok(Box::new(Factors {
            engine: Mutex::new(Some(engine)),
            pool: self.pool.clone(),
            report,
        }))
    }
}

struct Factors {
    engine: Mutex<Option<Engine>>,
    pool: Arc<Mutex<Vec<Engine>>>,
    report: Report,
}

impl Factors {
    fn solve_with(&self, b: &[c64], transpose: bool) -> Result<Vec<c64>> {
        let mut engine = self.engine.lock().map_err(poisoned)?;
        match engine.as_mut() {
            Some(engine) => engine.solve(b, transpose),
            None => Err(error("the factors were given back")),
        }
    }
}

impl Factorization for Factors {
    fn solve(&self, b: &[c64]) -> Result<Vec<c64>> {
        self.solve_with(b, false)
    }

    fn solve_transpose(&self, b: &[c64]) -> Result<Vec<c64>> {
        self.solve_with(b, true)
    }

    fn report(&self) -> Report {
        self.report.clone()
    }
}

impl Drop for Factors {
    fn drop(&mut self) {
        if let (Ok(mut engine), Ok(mut pool)) = (self.engine.lock(), self.pool.lock())
            && let Some(engine) = engine.take()
        {
            pool.push(engine);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layouts_are_the_headers() {
        // from each release's zmumps_c.h by C's layout rules (4-byte MUMPS_INT, 8-byte
        // MUMPS_INT8, double and pointers): the two layouts, and what they share
        assert_eq!((SYM, PAR, JOB, COMM_FORTRAN, ICNTL), (0, 4, 8, 12, 16));
        // icntl[60], keep[500], then cntl[15], dkeep[230], keep8[150], n
        let cntl = 16 + 4 * (60 + 500);
        assert_eq!(cntl, 2256);
        assert_eq!(N, cntl + 8 * (15 + 230 + 150));
        // n, nblk, nz_alloc, nz, then nnz on 8 bytes, irn, jcn, a
        assert_eq!((NNZ, IRN, JCN, A), (N + 16, N + 24, N + 32, N + 40));
        for layout in [&OLD, &NEW] {
            // nrhs and lrhs follow each other, info[80] and infog[80] too
            assert_eq!(layout.lrhs, layout.nrhs + 4);
            assert!(layout.rhs > A && layout.nrhs > layout.rhs && layout.infog > layout.lrhs);
            // infog[80], rinfo[40], rinfog[40], then the null space's and Schur's fields
            assert!(layout.version_number > layout.infog + 4 * 80 + 8 * 80);
            assert!(layout.size > layout.version_number + 32 && layout.size.is_multiple_of(8));
            assert!(layout.size <= ROOM / 4);
            for version in layout.versions {
                assert!(is_version(version));
                assert_eq!(layout_of(version), Some(layout));
            }
        }
        // 5.7 added fields before the right-hand side and lengthened the paths
        assert_eq!(NEW.rhs - OLD.rhs, 40);
        assert_eq!(NEW.infog - OLD.infog, 72);
        assert_eq!(NEW.version_number - OLD.version_number, 72);
        assert_eq!(layout_of("5.9.0"), None);
        assert_eq!(layout_of("5.3.5"), None);
    }

    #[test]
    fn a_structures_fields_are_read_where_they_are_written() {
        let mut s = Structure::new();
        s.set_int(JOB, -1);
        s.set_icntl(9, 2);
        s.set_long(NNZ, 1 << 40);
        assert_eq!(s.int(JOB), -1);
        assert_eq!(s.int(ICNTL + 4 * 8), 2);
        assert_eq!(s.words[NNZ / 8], 1 << 40);
        // INFOG(1) and INFOG(2), as MUMPS would leave them
        s.set_int(NEW.infog, -10);
        s.set_int(NEW.infog + 4, 7);
        assert_eq!((s.infog(&NEW, 1), s.infog(&NEW, 2)), (-10, 7));
        // a version, at an offset that isn't a multiple of 8, up to its NUL
        for (i, b) in b"5.8.2\0junk".iter().enumerate() {
            let at = OLD.version_number + 3 + i;
            s.words[at / 8] |= u64::from(*b) << (8 * (at % 8));
        }
        assert_eq!(s.text(OLD.version_number + 3, 31), "5.8.2");
        assert!(is_version("5.8.2") && is_version("5.10.0"));
        for not in ["", "5.8", "5.8.2.1", "5.8.x", "MUMPS", "5..2"] {
            assert!(!is_version(not), "{not}");
        }
    }

    #[test]
    fn a_matrixs_entries_are_one_based_and_a_symmetric_ones_its_lower_triangle() {
        // [a . b]
        // [. . c]
        // [b c d]
        let (cs, ri) = (vec![0, 2, 3, 6], vec![0, 2, 2, 0, 1, 2]);
        let all = Entries::new(&cs, &ri, false).unwrap();
        assert_eq!(all.n, 3);
        assert_eq!(all.rows, [1, 3, 3, 1, 2, 3]);
        assert_eq!(all.columns, [1, 1, 2, 3, 3, 3]);
        assert_eq!(all.order, [0, 1, 2, 3, 4, 5]);
        let lower = Entries::new(&cs, &ri, true).unwrap();
        assert_eq!(lower.rows, [1, 3, 3, 3]);
        assert_eq!(lower.columns, [1, 1, 2, 3]);
        assert_eq!(lower.order, [0, 1, 2, 5]);
        assert_eq!(lower.entries, 6);
        let v: Vec<c64> = (1..=6).map(|k| c64::new(f64::from(k), 0.0)).collect();
        assert_eq!(lower.values(&v), [v[0], v[1], v[2], v[5]]);
    }

    #[test]
    fn an_error_code_has_its_meaning() {
        assert!(meaning(-10).contains("singular"));
        assert!(meaning(-6).contains("structure"));
        assert!(meaning(-9).contains("too small"));
        assert!(meaning(-999).contains("user guide"));
    }
}
