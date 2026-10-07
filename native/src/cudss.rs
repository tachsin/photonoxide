//! NVIDIA cuDSS as a [`DirectSolver`]: sparse LU on the GPU (#188).
//!
//! - **The matrices** go to the GPU by rows: a general matrix's columns are turned into rows
//!   once per structure.
//! - **Symmetric matrices** are factorized as general ones for now: cuDSS 0.8's L D Lᵀ solve
//!   failed on the GPU (an illegal address) in its deterministic mode, from a few thousand
//!   unknowns; without that mode it is to be checked.
//! - **Analysis** (reordering and symbolic factorization) is done once per structure. Each
//!   factorization takes an analysed cuDSS object from a pool, and gives it back when dropped,
//!   so a sweep analyses once.
//! - **The transpose:** cuDSS doesn't solve with it, so a general matrix's transpose is
//!   factorized as a matrix of its own the first time it is asked for (its rows are the
//!   matrix's columns, as given).
//! - **Memory:** a factorization whose estimated peak doesn't fit in the GPU's free memory is
//!   refused, with both numbers. cuDSS's hybrid (host and device) mode is not used yet.
//! - **Determinism:** not declared. cuDSS 0.8's deterministic mode makes its kernels read an
//!   illegal address on the GPU when handles are used from more than one thread, even one call
//!   at a time, so it is off; repeated solves then differ in their last bits (the tests bound
//!   the difference). photonoxide's own solver stays the deterministic default.
//! - **Threads:** one cuDSS or CUDA call at a time in the process ([`SERIAL`]): there is one GPU.
//! - **Indices** are 32-bit: matrices with more than 2³¹ − 1 entries are refused, far beyond a
//!   consumer GPU's memory.

use std::cell::Cell;
use std::ffi::c_void;
use std::os::raw::c_int;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::Result;
use photonoxide::backend::{
    Analysis, Capabilities, DirectSolver, Factorization, Form, Matrix, Report, Threads,
};

use crate::library::{Library, error, load};
use crate::nvidia::{CUDA_RUNTIME, CUDSS};

type Opaque = *mut c_void;

/// One cuDSS call at a time in the process: handles used from several threads at once fail
/// in cuDSS's analysis (status 5), and there is one GPU to share anyway.
static SERIAL: Mutex<()> = Mutex::new(());

thread_local! {
    static HOLDING: Cell<bool> = const { Cell::new(false) };
}

/// [`SERIAL`] held by this thread, taken again freely by it (an engine dropped on an early
/// return of a call that holds it).
struct Serial(Option<MutexGuard<'static, ()>>);

fn serial() -> Serial {
    if HOLDING.get() {
        return Serial(None);
    }
    let guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    HOLDING.set(true);
    Serial(Some(guard))
}

impl Drop for Serial {
    fn drop(&mut self) {
        if self.0.is_some() {
            HOLDING.set(false);
        }
    }
}

// cudaMemcpyKind, cudaDataType and cuDSS's enums, from their headers (CUDA 13.1, cuDSS 0.8)
const HOST_TO_DEVICE: c_int = 1;
const DEVICE_TO_HOST: c_int = 2;
const C_64F: c_int = 5;
const R_32I: c_int = 10;
const MTYPE_GENERAL: c_int = 0;
const MVIEW_FULL: c_int = 0;
const BASE_ZERO: c_int = 0;
const LAYOUT_COL_MAJOR: c_int = 0;
const PHASE_ANALYSIS: c_int = 0b11;
const PHASE_FACTORIZATION: c_int = 1 << 2;
const PHASE_SOLVE: c_int = 0b11_1111 << 4;
const DATA_INFO: c_int = 0;
const DATA_LU_NNZ: c_int = 1;
const DATA_NPIVOTS: c_int = 2;
const DATA_MEMORY_ESTIMATES: c_int = 13;

/// cuDSS's and the CUDA runtime's functions, and the libraries that hold them.
struct Api {
    malloc: unsafe extern "C" fn(*mut Opaque, usize) -> c_int,
    free: unsafe extern "C" fn(Opaque) -> c_int,
    memcpy: unsafe extern "C" fn(Opaque, *const c_void, usize, c_int) -> c_int,
    synchronize: unsafe extern "C" fn() -> c_int,
    mem_get_info: unsafe extern "C" fn(*mut usize, *mut usize) -> c_int,
    create: unsafe extern "C" fn(*mut Opaque) -> c_int,
    destroy: unsafe extern "C" fn(Opaque) -> c_int,
    config_create: unsafe extern "C" fn(*mut Opaque) -> c_int,
    config_destroy: unsafe extern "C" fn(Opaque) -> c_int,
    data_create: unsafe extern "C" fn(Opaque, *mut Opaque) -> c_int,
    data_destroy: unsafe extern "C" fn(Opaque, Opaque) -> c_int,
    data_get: unsafe extern "C" fn(Opaque, Opaque, c_int, *mut c_void, usize, *mut usize) -> c_int,
    #[allow(clippy::type_complexity)]
    csr: unsafe extern "C" fn(
        *mut Opaque,
        i64,
        i64,
        i64,
        *const c_void,
        *const c_void,
        *const c_void,
        *const c_void,
        c_int,
        c_int,
        c_int,
        c_int,
        c_int,
        c_int,
    ) -> c_int,
    dense: unsafe extern "C" fn(*mut Opaque, i64, i64, i64, *const c_void, c_int, c_int) -> c_int,
    matrix_destroy: unsafe extern "C" fn(Opaque) -> c_int,
    execute: unsafe extern "C" fn(Opaque, c_int, Opaque, Opaque, Opaque, Opaque, Opaque) -> c_int,
    version: String,
    // last: the functions above must not outlive them
    _cudss: Library,
    _runtime: Library,
}

fn check(status: c_int, call: &str) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(error(format!("{call} failed with status {status}")))
    }
}

impl Api {
    /// The CUDA runtime first, so that cuDSS's own dependency on it is the same library.
    fn load() -> std::result::Result<Api, String> {
        let (found, runtime) = load(&CUDA_RUNTIME, None, |_| Ok(()));
        let runtime = runtime.ok_or_else(|| found.reason())?;
        let (found, cudss) = load(&CUDSS, None, |_| Ok(()));
        let cudss = cudss.ok_or_else(|| found.reason())?;
        let version = crate::nvidia::property_version(&cudss, "cudssGetProperty")
            .map_err(|e| e.to_string())?;
        let api = || -> Result<Api> {
            // SAFETY: each type is the function's declaration in cuda_runtime_api.h or cudss.h
            // (handles, configs, data and matrices are pointers; enums are ints)
            unsafe {
                Ok(Api {
                    malloc: runtime.function("cudaMalloc")?,
                    free: runtime.function("cudaFree")?,
                    memcpy: runtime.function("cudaMemcpy")?,
                    synchronize: runtime.function("cudaDeviceSynchronize")?,
                    mem_get_info: runtime.function("cudaMemGetInfo")?,
                    create: cudss.function("cudssCreate")?,
                    destroy: cudss.function("cudssDestroy")?,
                    config_create: cudss.function("cudssConfigCreate")?,
                    config_destroy: cudss.function("cudssConfigDestroy")?,
                    data_create: cudss.function("cudssDataCreate")?,
                    data_destroy: cudss.function("cudssDataDestroy")?,
                    data_get: cudss.function("cudssDataGet")?,
                    csr: cudss.function("cudssMatrixCreateCsr")?,
                    dense: cudss.function("cudssMatrixCreateDn")?,
                    matrix_destroy: cudss.function("cudssMatrixDestroy")?,
                    execute: cudss.function("cudssExecute")?,
                    version: version.clone(),
                    _cudss: cudss,
                    _runtime: runtime,
                })
            }
        };
        api().map_err(|e| e.to_string())
    }

    fn free_memory(&self) -> Result<usize> {
        let _serial = serial();
        let (mut free, mut total) = (0, 0);
        check(
            // SAFETY: two writable size_t
            unsafe { (self.mem_get_info)(&mut free, &mut total) },
            "cudaMemGetInfo",
        )?;
        Ok(free)
    }
}

/// Memory on the GPU, freed when dropped.
struct Buffer {
    api: Arc<Api>,
    ptr: Opaque,
    bytes: usize,
}

impl Buffer {
    fn new<T: Copy>(api: &Arc<Api>, data: &[T]) -> Result<Buffer> {
        let _serial = serial();
        let bytes = size_of_val(data).max(1);
        let mut ptr = std::ptr::null_mut();
        // SAFETY: ptr is writable; bytes > 0
        check(unsafe { (api.malloc)(&mut ptr, bytes) }, "cudaMalloc")?;
        let buffer = Buffer {
            api: api.clone(),
            ptr,
            bytes,
        };
        buffer.write(data)?;
        Ok(buffer)
    }

    fn write<T: Copy>(&self, data: &[T]) -> Result<()> {
        let _serial = serial();
        let bytes = size_of_val(data);
        assert!(bytes <= self.bytes);
        // SAFETY: the device buffer holds self.bytes ≥ bytes; data is bytes long
        check(
            unsafe { (self.api.memcpy)(self.ptr, data.as_ptr().cast(), bytes, HOST_TO_DEVICE) },
            "cudaMemcpy to the GPU",
        )
    }

    fn read<T: Copy>(&self, out: &mut [T]) -> Result<()> {
        let _serial = serial();
        let bytes = size_of_val(out);
        assert!(bytes <= self.bytes);
        // SAFETY: out is bytes long and writable; the device buffer holds at least that
        check(
            unsafe { (self.api.memcpy)(out.as_mut_ptr().cast(), self.ptr, bytes, DEVICE_TO_HOST) },
            "cudaMemcpy from the GPU",
        )
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        let _serial = serial();
        // SAFETY: ptr came from cudaMalloc and is freed once
        unsafe { (self.api.free)(self.ptr) };
    }
}

/// A matrix's rows: n + 1 starts and each entry's column, 32-bit.
struct Rows {
    starts: Vec<i32>,
    columns: Vec<i32>,
}

fn index(i: usize) -> Result<i32> {
    i32::try_from(i).map_err(|_| error("cuDSS's indices here are 32-bit: the matrix is too large"))
}

/// One cuDSS solver object: a handle, its settings and data, the matrix, the right-hand side
/// and the solution, all on the GPU, analysed for one structure.
struct Engine {
    api: Arc<Api>,
    n: usize,
    handle: Opaque,
    config: Opaque,
    data: Opaque,
    a: Opaque,
    x: Opaque,
    b: Opaque,
    values: Buffer,
    x_values: Buffer,
    b_values: Buffer,
    _starts: Buffer,
    _columns: Buffer,
    peak_bytes: u64,
}

// SAFETY: an Engine's cuDSS objects are used by one thread at a time, behind a Mutex
unsafe impl Send for Engine {}

impl Engine {
    /// Uploads the matrix and analyses it, in the seconds returned.
    fn new(api: &Arc<Api>, rows: &Rows, values: &[c64]) -> Result<(Engine, f64)> {
        let _serial = serial();
        let n = rows.starts.len() - 1;
        let zero = vec![c64::new(0.0, 0.0); n];
        let starts = Buffer::new(api, &rows.starts)?;
        let columns = Buffer::new(api, &rows.columns)?;
        let device_values = Buffer::new(api, values)?;
        let (x_values, b_values) = (Buffer::new(api, &zero)?, Buffer::new(api, &zero)?);
        let mut e = Engine {
            api: api.clone(),
            n,
            handle: std::ptr::null_mut(),
            config: std::ptr::null_mut(),
            data: std::ptr::null_mut(),
            a: std::ptr::null_mut(),
            x: std::ptr::null_mut(),
            b: std::ptr::null_mut(),
            values: device_values,
            x_values,
            b_values,
            _starts: starts,
            _columns: columns,
            peak_bytes: 0,
        };
        let _serial = serial();
        let n64 = n as i64;
        // SAFETY: the out-pointers are writable; the device pointers hold n + 1 starts, nnz
        // columns and values, and n entries for x and b, as the matrices are declared; the
        // objects are destroyed in Drop, the null ones skipped
        unsafe {
            check((api.create)(&mut e.handle), "cudssCreate")?;
            check((api.config_create)(&mut e.config), "cudssConfigCreate")?;
            check((api.data_create)(e.handle, &mut e.data), "cudssDataCreate")?;
            check(
                (api.csr)(
                    &mut e.a,
                    n64,
                    n64,
                    rows.columns.len() as i64,
                    e._starts.ptr,
                    std::ptr::null(),
                    e._columns.ptr,
                    e.values.ptr,
                    R_32I,
                    R_32I,
                    C_64F,
                    MTYPE_GENERAL,
                    MVIEW_FULL,
                    BASE_ZERO,
                ),
                "cudssMatrixCreateCsr",
            )?;
            for (m, buffer) in [(&mut e.x, e.x_values.ptr), (&mut e.b, e.b_values.ptr)] {
                check(
                    (api.dense)(m, n64, 1, n64, buffer, C_64F, LAYOUT_COL_MAJOR),
                    "cudssMatrixCreateDn",
                )?;
            }
        }
        let start = Instant::now();
        e.run(PHASE_ANALYSIS, "analysis")?;
        let seconds = start.elapsed().as_secs_f64();
        let mut estimates = [0i64; 16];
        e.get(DATA_MEMORY_ESTIMATES, &mut estimates)?;
        // [1]: the peak of device memory the factorization needs
        e.peak_bytes = estimates[1].max(0) as u64;
        let free = api.free_memory()?;
        if e.peak_bytes > free as u64 {
            const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
            return Err(error(format!(
                "cuDSS's factorization needs {:.2} GiB of the GPU and {:.2} GiB is free",
                e.peak_bytes as f64 / GIB,
                free as f64 / GIB
            )));
        }
        Ok((e, seconds))
    }

    fn run(&self, phase: c_int, what: &str) -> Result<()> {
        // SAFETY: the objects were created in new() and live until Drop
        unsafe {
            check(
                (self.api.execute)(
                    self.handle,
                    phase,
                    self.config,
                    self.data,
                    self.a,
                    self.x,
                    self.b,
                ),
                &format!("cuDSS's {what}"),
            )?;
            check(
                (self.api.synchronize)(),
                &format!("cudaDeviceSynchronize after cuDSS's {what}"),
            )
        }
    }

    /// A value of cuDSS's data (`DATA_*`) into `out`.
    fn get<T: Copy>(&self, what: c_int, out: &mut [T]) -> Result<usize> {
        let mut written = 0;
        check(
            // SAFETY: out is writable for its size in bytes
            unsafe {
                (self.api.data_get)(
                    self.handle,
                    self.data,
                    what,
                    out.as_mut_ptr().cast(),
                    size_of_val(out),
                    &mut written,
                )
            },
            &format!("cudssDataGet({what}, {} bytes)", size_of_val(out)),
        )?;
        Ok(written)
    }

    /// Factorizes these values: the seconds, the factors' entries and the pivots replaced.
    fn factorize(&self, values: &[c64]) -> Result<(f64, Option<usize>, usize)> {
        let _serial = serial();
        self.values.write(values)?;
        let start = Instant::now();
        self.run(PHASE_FACTORIZATION, "factorization")?;
        let seconds = start.elapsed().as_secs_f64();
        // INFO is an int, LU_NNZ an int64_t: cuDSS takes only their exact sizes
        let mut info = [0i32; 1];
        self.get(DATA_INFO, &mut info)?;
        let info = info[0];
        if info != 0 {
            return Err(error(format!(
                "cuDSS's factorization reports {info}: a zero pivot, the matrix singular"
            )));
        }
        let mut entries = [0i64; 1];
        let entries = self
            .get(DATA_LU_NNZ, &mut entries)
            .ok()
            .map(|_| entries[0].max(0) as usize);
        // the pivots cuDSS replaced, too small to divide by (an int): 1 for a singular matrix
        let mut pivots = [0i32; 1];
        self.get(DATA_NPIVOTS, &mut pivots)?;
        Ok((seconds, entries, pivots[0].max(0) as usize))
    }

    fn solve(&self, b: &[c64]) -> Result<Vec<c64>> {
        if b.len() != self.n {
            return Err(error(format!(
                "a right-hand side of {} for {} unknowns",
                b.len(),
                self.n
            )));
        }
        let _serial = serial();
        self.b_values.write(b)?;
        self.run(PHASE_SOLVE, "solve")?;
        let mut x = vec![c64::new(0.0, 0.0); self.n];
        self.x_values.read(&mut x)?;
        Ok(x)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let api = &self.api;
        let _serial = serial();
        // SAFETY: each object was created once by the calls in new() (or is null and
        // skipped), and is destroyed once, the matrices and data before the handle
        unsafe {
            for m in [self.a, self.x, self.b] {
                if !m.is_null() {
                    (api.matrix_destroy)(m);
                }
            }
            if !self.data.is_null() {
                (api.data_destroy)(self.handle, self.data);
            }
            if !self.config.is_null() {
                (api.config_destroy)(self.config);
            }
            if !self.handle.is_null() {
                (api.destroy)(self.handle);
            }
        }
    }
}

/// cuDSS, as photonoxide's registry has it.
pub struct Cudss {
    api: Arc<Api>,
}

impl Cudss {
    /// cuDSS and the CUDA runtime, found and loaded.
    ///
    /// # Errors
    ///
    /// Why not: not found, a function missing.
    pub fn load() -> std::result::Result<Cudss, String> {
        Api::load().map(|api| Cudss { api: Arc::new(api) })
    }
}

impl DirectSolver for Cudss {
    fn capabilities(&self) -> Capabilities {
        let mut c = Capabilities::new(
            "cudss",
            self.api.version.clone(),
            "NVIDIA cuDSS licence (installed by the user)",
        );
        // not yet (see the module's docs): symmetric matrices are factorized as general ones
        c.symmetric = false;
        c.transpose = true;
        c.threads = Threads::Library("the GPU".into());
        // its deterministic mode fails on the GPU from several threads (the module's docs)
        c.deterministic = false;
        c
    }

    fn analyse(&self, matrix: &Matrix<'_>) -> Result<Option<Arc<dyn Analysis>>> {
        let n = matrix.n();
        let (cs, ri, v) = (
            matrix.column_starts(),
            matrix.row_indices(),
            matrix.values(),
        );
        if matrix.form() == Form::Symmetric {
            // the caller asks for the general form instead (see capabilities)
            return Ok(None);
        }
        index(v.len())?;
        let ((starts, columns), order) = transposed(n, cs, ri);
        let rows = columns_as_rows(&starts, &columns)?;
        let values: Vec<c64> = order.iter().map(|&k| v[k]).collect();
        let (engine, seconds) = Engine::new(&self.api, &rows, &values)?;
        Ok(Some(Arc::new(Analysed {
            api: self.api.clone(),
            form: matrix.form(),
            rows,
            order,
            entries: v.len(),
            pool: Arc::new(Mutex::new(vec![engine])),
            seconds,
        })))
    }
}

/// A matrix's columns, read as rows.
fn columns_as_rows(cs: &[usize], ri: &[usize]) -> Result<Rows> {
    Ok(Rows {
        starts: cs.iter().map(|&s| index(s)).collect::<Result<_>>()?,
        columns: ri.iter().map(|&r| index(r)).collect::<Result<_>>()?,
    })
}

/// The rows of a matrix given by columns: each row's (starts, columns), and for each entry by
/// rows, its place in the columns' order.
fn transposed(n: usize, cs: &[usize], ri: &[usize]) -> ((Vec<usize>, Vec<usize>), Vec<usize>) {
    let mut starts = vec![0usize; n + 1];
    for &r in ri {
        starts[r + 1] += 1;
    }
    for i in 0..n {
        starts[i + 1] += starts[i];
    }
    let mut next = starts.clone();
    let mut columns = vec![0; ri.len()];
    let mut order = vec![0; ri.len()];
    for j in 0..n {
        for k in cs[j]..cs[j + 1] {
            let p = next[ri[k]];
            columns[p] = j;
            order[p] = k;
            next[ri[k]] += 1;
        }
    }
    ((starts, columns), order)
}

struct Analysed {
    api: Arc<Api>,
    form: Form,
    rows: Rows,
    /// Each entry by rows, its place in the matrix's values.
    order: Vec<usize>,
    /// The matrix's entries, both triangles.
    entries: usize,
    pool: Arc<Mutex<Vec<Engine>>>,
    seconds: f64,
}

fn poisoned<T>(_: T) -> photonoxide::Error {
    error("a cuDSS engine's lock was poisoned by a panic")
}

impl Analysis for Analysed {
    fn form(&self) -> Form {
        self.form
    }

    fn factorize(&self, matrix: &Matrix<'_>) -> Result<Box<dyn Factorization>> {
        if matrix.values().len() != self.entries || matrix.n() != self.rows.starts.len() - 1 {
            return Err(error("a matrix of another structure than the one analysed"));
        }
        let values: Vec<c64> = self.order.iter().map(|&k| matrix.values()[k]).collect();
        let pooled = self.pool.lock().map_err(poisoned)?.pop();
        let (engine, analysis_seconds) = match pooled {
            Some(engine) => (engine, self.seconds),
            None => Engine::new(&self.api, &self.rows, &values)?,
        };
        let (factorization_seconds, factor_entries, perturbed_pivots) =
            engine.factorize(&values)?;
        let mut report = Report::default();
        report.factor_entries = factor_entries;
        report.peak_memory_bytes = Some(engine.peak_bytes);
        report.analysis_seconds = analysis_seconds;
        report.factorization_seconds = factorization_seconds;
        report.perturbed_pivots = perturbed_pivots;
        // its columns as rows: the transpose, factorized when first asked for
        let columns = (
            columns_as_rows(matrix.column_starts(), matrix.row_indices())?,
            matrix.values().to_vec(),
        );
        Ok(Box::new(Factors {
            api: self.api.clone(),
            engine: Mutex::new(Some(engine)),
            pool: self.pool.clone(),
            columns,
            transpose: Mutex::new(None),
            report,
        }))
    }
}

struct Factors {
    api: Arc<Api>,
    engine: Mutex<Option<Engine>>,
    pool: Arc<Mutex<Vec<Engine>>>,
    /// The matrix's columns as rows, and its values: its transpose.
    columns: (Rows, Vec<c64>),
    transpose: Mutex<Option<Engine>>,
    report: Report,
}

impl Factorization for Factors {
    fn solve(&self, b: &[c64]) -> Result<Vec<c64>> {
        let engine = self.engine.lock().map_err(poisoned)?;
        engine.as_ref().expect("held until dropped").solve(b)
    }

    fn solve_transpose(&self, b: &[c64]) -> Result<Vec<c64>> {
        let mut transpose = self.transpose.lock().map_err(poisoned)?;
        if transpose.is_none() {
            let (rows, values) = &self.columns;
            let (engine, _) = Engine::new(&self.api, rows, values)?;
            engine.factorize(values)?;
            *transpose = Some(engine);
        }
        transpose.as_ref().expect("factorized above").solve(b)
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
