//! photonoxide's QMR on an NVIDIA GPU (#190): the run of `fdfd::krylov`, step for step, on
//! cuSPARSE, the vectors kept on the GPU and only the scalars coming back each iteration.
//!
//! - **The products:** `cusparseSpMV` on the matrix by rows, and for the general QMR on its
//!   transpose by rows (the matrix's columns, as given) rather than with a transpose operation.
//! - **The vector work:** also cuSPARSE's, each vector seen as a sparse vector with every entry
//!   (one shared index array): `cusparseAxpby` for y ← αx + βy, `cusparseSpVV` for the dot
//!   products and norms. cuBLAS would read no index array; a first try with it crashed in
//!   `cusparseSpMV`, most likely from the misaligned scalars [`Scalar`] now prevents, and
//!   trying it again is a follow-up. `cusparseSpVV` is deprecated since CUDA 12.8: a CUDA without it
//!   makes this backend unavailable, saying so.
//! - **Determinism:** the sums are cuSPARSE's, not photonoxide's chunked ones, so the iterations
//!   agree with the CPU's to rounding's effect on the residual's history, not bit for bit; a
//!   repeated run on one GPU gives the same bits, which the tests check.
//! - **What stays with photonoxide:** the restarts after a breakdown and the tightened tolerance
//!   on a symmetric similar matrix ([`photonoxide::backend::IterativeSolver`]).
//! - **Threads:** one call at a time in the process, as cuDSS's ([`crate::cuda::serial`]).

use std::ffi::c_void;
use std::os::raw::c_int;
use std::sync::Arc;

use num_complex::Complex64 as c64;
use photonoxide::Result;
use photonoxide::backend::{
    Capabilities, Form, IluFactors, IterativeSolver, Matrix, QmrRun, Threads,
};
use photonoxide::fdfd::{Convergence, Stopping};

use crate::cuda::{Buffer, C_64F, Opaque, Runtime, check, serial};
use crate::cudss::{Rows, columns_as_rows, index, transposed};
use crate::library::{Library, error, load};
use crate::nvidia::{CUSPARSE, property_version};

// cuSPARSE's enums, from CUDA 13.1's cusparse.h
const NON_TRANSPOSE: c_int = 0;
const CONJUGATE_TRANSPOSE: c_int = 2;
const INDEX_32I: c_int = 2;
const BASE_ZERO: c_int = 0;
/// `CUSPARSE_SPMV_CSR_ALG2`: the same bits on every run.
const SPMV_ALGORITHM: c_int = 3;
const SPSV_ALGORITHM: c_int = 0;
const ATTRIBUTE_FILL_MODE: c_int = 0;
const ATTRIBUTE_DIAGONAL: c_int = 1;
const FILL_LOWER: c_int = 0;
const FILL_UPPER: c_int = 1;
const DIAGONAL_NON_UNIT: c_int = 0;
const DIAGONAL_UNIT: c_int = 1;

const ONE: c64 = c64::new(1.0, 0.0);
const ZERO: c64 = c64::new(0.0, 0.0);

/// A complex scalar as cuSPARSE reads and writes it on the host: `cuDoubleComplex` is CUDA's
/// `double2`, aligned to 16 bytes, and cuSPARSE loads it with aligned instructions. A `c64`,
/// aligned to 8, faults when it lands off a 16-byte boundary (an access violation that came
/// and went with the build and the libraries loaded).
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Scalar(c64);

impl Scalar {
    fn ptr(&self) -> *const c_void {
        (self as *const Scalar).cast()
    }
}

type Spmv = unsafe extern "C" fn(
    Opaque,
    c_int,
    *const c_void,
    Opaque,
    Opaque,
    *const c_void,
    Opaque,
    c_int,
    c_int,
    Opaque,
) -> c_int;
type SpmvBufferSize = unsafe extern "C" fn(
    Opaque,
    c_int,
    *const c_void,
    Opaque,
    Opaque,
    *const c_void,
    Opaque,
    c_int,
    c_int,
    *mut usize,
) -> c_int;
type CreateCsr = unsafe extern "C" fn(
    *mut Opaque,
    i64,
    i64,
    i64,
    Opaque,
    Opaque,
    Opaque,
    c_int,
    c_int,
    c_int,
    c_int,
) -> c_int;
type SpsvBufferSize = unsafe extern "C" fn(
    Opaque,
    c_int,
    *const c_void,
    Opaque,
    Opaque,
    Opaque,
    c_int,
    c_int,
    Opaque,
    *mut usize,
) -> c_int;
type SpsvAnalysis = unsafe extern "C" fn(
    Opaque,
    c_int,
    *const c_void,
    Opaque,
    Opaque,
    Opaque,
    c_int,
    c_int,
    Opaque,
    Opaque,
) -> c_int;
type SpsvSolve = unsafe extern "C" fn(
    Opaque,
    c_int,
    *const c_void,
    Opaque,
    Opaque,
    Opaque,
    c_int,
    c_int,
    Opaque,
) -> c_int;
type CreateSpVec =
    unsafe extern "C" fn(*mut Opaque, i64, i64, Opaque, Opaque, c_int, c_int, c_int) -> c_int;

/// cuSPARSE's functions, the library that holds them, and the CUDA runtime.
struct Api {
    runtime: Arc<Runtime>,
    create: unsafe extern "C" fn(*mut Opaque) -> c_int,
    destroy: unsafe extern "C" fn(Opaque) -> c_int,
    create_csr: CreateCsr,
    destroy_matrix: unsafe extern "C" fn(Opaque) -> c_int,
    create_dense: unsafe extern "C" fn(*mut Opaque, i64, Opaque, c_int) -> c_int,
    destroy_dense: unsafe extern "C" fn(Opaque) -> c_int,
    create_sparse: CreateSpVec,
    destroy_sparse: unsafe extern "C" fn(Opaque) -> c_int,
    spmv_buffer_size: SpmvBufferSize,
    spmv: Spmv,
    axpby: unsafe extern "C" fn(Opaque, *const c_void, Opaque, *const c_void, Opaque) -> c_int,
    spvv_buffer_size: unsafe extern "C" fn(
        Opaque,
        c_int,
        Opaque,
        Opaque,
        *const c_void,
        c_int,
        *mut usize,
    ) -> c_int,
    spvv: unsafe extern "C" fn(Opaque, c_int, Opaque, Opaque, *mut c_void, c_int, Opaque) -> c_int,
    set_attribute: unsafe extern "C" fn(Opaque, c_int, *const c_void, usize) -> c_int,
    spsv_create: unsafe extern "C" fn(*mut Opaque) -> c_int,
    spsv_destroy: unsafe extern "C" fn(Opaque) -> c_int,
    spsv_buffer_size: SpsvBufferSize,
    spsv_analysis: SpsvAnalysis,
    spsv_solve: SpsvSolve,
    version: String,
    // last: the functions above must not outlive it
    _cusparse: Library,
}

impl Api {
    fn load() -> std::result::Result<Api, String> {
        let runtime = Runtime::load()?;
        let (found, cusparse) = load(&CUSPARSE, None, |_| Ok(()));
        let cusparse = cusparse.ok_or_else(|| found.reason())?;
        let version =
            property_version(&cusparse, "cusparseGetProperty").map_err(|e| e.to_string())?;
        let api = || -> Result<Api> {
            // SAFETY: each type is the function's declaration in cusparse.h (handles and
            // descriptors are pointers, enums ints)
            unsafe {
                Ok(Api {
                    create: cusparse.function("cusparseCreate")?,
                    destroy: cusparse.function("cusparseDestroy")?,
                    create_csr: cusparse.function("cusparseCreateCsr")?,
                    destroy_matrix: cusparse.function("cusparseDestroySpMat")?,
                    create_dense: cusparse.function("cusparseCreateDnVec")?,
                    destroy_dense: cusparse.function("cusparseDestroyDnVec")?,
                    create_sparse: cusparse.function("cusparseCreateSpVec")?,
                    destroy_sparse: cusparse.function("cusparseDestroySpVec")?,
                    spmv_buffer_size: cusparse.function("cusparseSpMV_bufferSize")?,
                    spmv: cusparse.function("cusparseSpMV")?,
                    axpby: cusparse.function("cusparseAxpby")?,
                    spvv_buffer_size: cusparse.function("cusparseSpVV_bufferSize")?,
                    spvv: cusparse.function("cusparseSpVV")?,
                    set_attribute: cusparse.function("cusparseSpMatSetAttribute")?,
                    spsv_create: cusparse.function("cusparseSpSV_createDescr")?,
                    spsv_destroy: cusparse.function("cusparseSpSV_destroyDescr")?,
                    spsv_buffer_size: cusparse.function("cusparseSpSV_bufferSize")?,
                    spsv_analysis: cusparse.function("cusparseSpSV_analysis")?,
                    spsv_solve: cusparse.function("cusparseSpSV_solve")?,
                    version: format!("cuSPARSE {version}"),
                    runtime,
                    _cusparse: cusparse,
                })
            }
        };
        api().map_err(|e| e.to_string())
    }
}

/// A matrix by rows on the GPU, with cuSPARSE's descriptor.
struct DeviceMatrix {
    api: Arc<Api>,
    descriptor: Opaque,
    _starts: Buffer,
    _columns: Buffer,
    _values: Buffer,
}

impl DeviceMatrix {
    fn new(api: &Arc<Api>, rows: &Rows, values: &[c64]) -> Result<DeviceMatrix> {
        let _serial = serial();
        let n = (rows.starts.len() - 1) as i64;
        let starts = Buffer::new(&api.runtime, &rows.starts)?;
        let columns = Buffer::new(&api.runtime, &rows.columns)?;
        let device_values = Buffer::new(&api.runtime, values)?;
        let mut descriptor = std::ptr::null_mut();
        check(
            // SAFETY: the buffers hold n + 1 starts, nnz columns and nnz values, as declared
            unsafe {
                (api.create_csr)(
                    &mut descriptor,
                    n,
                    n,
                    values.len() as i64,
                    starts.ptr,
                    columns.ptr,
                    device_values.ptr,
                    INDEX_32I,
                    INDEX_32I,
                    BASE_ZERO,
                    C_64F,
                )
            },
            "cusparseCreateCsr",
        )?;
        Ok(DeviceMatrix {
            api: api.clone(),
            descriptor,
            _starts: starts,
            _columns: columns,
            _values: device_values,
        })
    }
}

impl Drop for DeviceMatrix {
    fn drop(&mut self) {
        let _serial = serial();
        // SAFETY: created once in new(), destroyed once, before its buffers
        unsafe { (self.api.destroy_matrix)(self.descriptor) };
    }
}

/// A vector on the GPU, seen by cuSPARSE both as a dense vector and as a sparse one with every
/// entry.
struct DeviceVector {
    api: Arc<Api>,
    dense: Opaque,
    sparse: Opaque,
    values: Buffer,
}

impl DeviceVector {
    /// A vector of these values, its sparse view over `indices` (0, 1, …, n − 1).
    fn new(api: &Arc<Api>, values: &[c64], indices: &Buffer) -> Result<DeviceVector> {
        let _serial = serial();
        let n = values.len() as i64;
        let mut v = DeviceVector {
            api: api.clone(),
            dense: std::ptr::null_mut(),
            sparse: std::ptr::null_mut(),
            values: Buffer::new(&api.runtime, values)?,
        };
        // SAFETY: the buffer holds n values and `indices` n 32-bit indices, both alive as long
        // as the descriptors (the session holds the indices)
        unsafe {
            check(
                (api.create_dense)(&mut v.dense, n, v.values.ptr, C_64F),
                "cusparseCreateDnVec",
            )?;
            check(
                (api.create_sparse)(
                    &mut v.sparse,
                    n,
                    n,
                    indices.ptr,
                    v.values.ptr,
                    INDEX_32I,
                    BASE_ZERO,
                    C_64F,
                ),
                "cusparseCreateSpVec",
            )?;
        }
        Ok(v)
    }

    fn read(&self) -> Result<Vec<c64>> {
        let mut out = vec![ZERO; (self.values.bytes() / size_of::<c64>()).max(1)];
        self.values.read(&mut out)?;
        Ok(out)
    }
}

impl Drop for DeviceVector {
    fn drop(&mut self) {
        let _serial = serial();
        // SAFETY: each descriptor was created once (or is null and skipped), destroyed once
        unsafe {
            if !self.dense.is_null() {
                (self.api.destroy_dense)(self.dense);
            }
            if !self.sparse.is_null() {
                (self.api.destroy_sparse)(self.sparse);
            }
        }
    }
}

/// A run's handle, its index array and the products' workspace.
struct Session {
    api: Arc<Api>,
    handle: Opaque,
    indices: Buffer,
    workspace: Option<Buffer>,
    /// A vector of zeros, the x of y ← 0·x + βy.
    zero: Option<DeviceVector>,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _serial = serial();
        // the zero vector before the handle
        self.zero = None;
        if !self.handle.is_null() {
            // SAFETY: created once in new(), destroyed once
            unsafe { (self.api.destroy)(self.handle) };
        }
    }
}

impl Session {
    fn new(api: &Arc<Api>, n: usize) -> Result<Session> {
        let _serial = serial();
        let indices: Vec<i32> = (0..n).map(index).collect::<Result<_>>()?;
        let mut s = Session {
            api: api.clone(),
            handle: std::ptr::null_mut(),
            indices: Buffer::new(&api.runtime, &indices)?,
            workspace: None,
            zero: None,
        };
        // SAFETY: the out-pointer is writable
        check(unsafe { (api.create)(&mut s.handle) }, "cusparseCreate")?;
        s.zero = Some(s.vector(&vec![ZERO; n])?);
        Ok(s)
    }

    fn vector(&self, values: &[c64]) -> Result<DeviceVector> {
        DeviceVector::new(&self.api, values, &self.indices)
    }

    fn reserve(&mut self, bytes: usize) -> Result<()> {
        if self.workspace.as_ref().is_none_or(|w| w.bytes() < bytes) {
            self.workspace = Some(Buffer::with_bytes(&self.api.runtime, bytes)?);
        }
        Ok(())
    }

    fn workspace(&self) -> Opaque {
        self.workspace
            .as_ref()
            .map_or(std::ptr::null_mut(), |w| w.ptr)
    }

    /// Makes the workspace large enough for products with `m` and for the dot products.
    fn prepare(&mut self, m: &DeviceMatrix, x: &DeviceVector, y: &DeviceVector) -> Result<()> {
        let (mut spmv, mut spvv) = (0usize, 0usize);
        let (one, zero) = (Scalar(ONE), Scalar(ZERO));
        let mut result = Scalar(ZERO);
        // SAFETY: the handle, the matrix and the vectors are live; the sizes are writable
        unsafe {
            check(
                (self.api.spmv_buffer_size)(
                    self.handle,
                    NON_TRANSPOSE,
                    one.ptr(),
                    m.descriptor,
                    x.dense,
                    zero.ptr(),
                    y.dense,
                    C_64F,
                    SPMV_ALGORITHM,
                    &mut spmv,
                ),
                "cusparseSpMV_bufferSize",
            )?;
            check(
                (self.api.spvv_buffer_size)(
                    self.handle,
                    NON_TRANSPOSE,
                    x.sparse,
                    y.dense,
                    (&mut result as *mut Scalar).cast(),
                    C_64F,
                    &mut spvv,
                ),
                "cusparseSpVV_bufferSize",
            )?;
        }
        self.reserve(spmv.max(spvv))
    }

    /// y = M x.
    fn product(&self, m: &DeviceMatrix, x: &DeviceVector, y: &DeviceVector) -> Result<()> {
        let (one, zero) = (Scalar(ONE), Scalar(ZERO));
        check(
            // SAFETY: x and y are n long, M n × n; the workspace is at least what cuSPARSE
            // asked for
            unsafe {
                (self.api.spmv)(
                    self.handle,
                    NON_TRANSPOSE,
                    one.ptr(),
                    m.descriptor,
                    x.dense,
                    zero.ptr(),
                    y.dense,
                    C_64F,
                    SPMV_ALGORITHM,
                    self.workspace(),
                )
            },
            "cusparseSpMV",
        )
    }

    /// Σ a_k b_k, unconjugated, or Σ conj(a_k) b_k.
    fn dot(&self, a: &DeviceVector, b: &DeviceVector, conjugate: bool) -> Result<c64> {
        let mut out = Scalar(ZERO);
        check(
            // SAFETY: a and b are n long; out is writable (host pointer mode)
            unsafe {
                (self.api.spvv)(
                    self.handle,
                    if conjugate {
                        CONJUGATE_TRANSPOSE
                    } else {
                        NON_TRANSPOSE
                    },
                    a.sparse,
                    b.dense,
                    (&mut out as *mut Scalar).cast(),
                    C_64F,
                    self.workspace(),
                )
            },
            "cusparseSpVV",
        )?;
        Ok(out.0)
    }

    /// Σ a_k b_k, unconjugated.
    fn dotu(&self, a: &DeviceVector, b: &DeviceVector) -> Result<c64> {
        self.dot(a, b, false)
    }

    /// ‖a‖².
    fn norm2(&self, a: &DeviceVector) -> Result<f64> {
        Ok(self.dot(a, a, true)?.re)
    }

    /// y ← α x + β y.
    fn axpby(&self, alpha: c64, x: &DeviceVector, beta: c64, y: &DeviceVector) -> Result<()> {
        let (alpha, beta) = (Scalar(alpha), Scalar(beta));
        check(
            // SAFETY: x and y are n long; alpha and beta are read on the host
            unsafe { (self.api.axpby)(self.handle, alpha.ptr(), x.sparse, beta.ptr(), y.dense) },
            "cusparseAxpby",
        )
    }

    /// y += c x.
    fn axpy(&self, c: c64, x: &DeviceVector, y: &DeviceVector) -> Result<()> {
        self.axpby(c, x, ONE, y)
    }

    /// y = x.
    fn copy(&self, x: &DeviceVector, y: &DeviceVector) -> Result<()> {
        self.axpby(ONE, x, ZERO, y)
    }

    /// x *= c.
    fn scale(&self, c: c64, x: &DeviceVector) -> Result<()> {
        let zero = self.zero.as_ref().expect("made in new()");
        self.axpby(ZERO, zero, c, x)
    }
}

/// A triangular factor on the GPU, analysed for `cusparseSpSV`: its descriptor and the buffer
/// the analysis fills, which the solves read.
struct Triangular {
    api: Arc<Api>,
    matrix: DeviceMatrix,
    descriptor: Opaque,
    _buffer: Buffer,
}

impl Triangular {
    /// The factor of these rows, lower or upper, its diagonal unit (not stored) or not, analysed
    /// on vectors like `x` and `y`.
    fn new(
        s: &Session,
        (rows, values): (&Rows, &[c64]),
        lower: bool,
        unit: bool,
        x: &DeviceVector,
        y: &DeviceVector,
    ) -> Result<Triangular> {
        let _serial = serial();
        let api = &s.api;
        let matrix = DeviceMatrix::new(api, rows, values)?;
        let fill = if lower { FILL_LOWER } else { FILL_UPPER };
        let diagonal = if unit {
            DIAGONAL_UNIT
        } else {
            DIAGONAL_NON_UNIT
        };
        let one = Scalar(ONE);
        let mut descriptor = std::ptr::null_mut();
        let mut bytes = 0usize;
        // SAFETY: the matrix descriptor is live; the attributes are ints, as cusparse.h
        // declares; the out-pointers are writable; x and y are n long
        unsafe {
            for (attribute, value) in [
                (ATTRIBUTE_FILL_MODE, &fill),
                (ATTRIBUTE_DIAGONAL, &diagonal),
            ] {
                check(
                    (api.set_attribute)(
                        matrix.descriptor,
                        attribute,
                        (value as *const c_int).cast(),
                        size_of::<c_int>(),
                    ),
                    "cusparseSpMatSetAttribute",
                )?;
            }
            check(
                (api.spsv_create)(&mut descriptor),
                "cusparseSpSV_createDescr",
            )?;
        }
        let mut t = Triangular {
            api: api.clone(),
            matrix,
            descriptor,
            _buffer: Buffer::with_bytes(&api.runtime, 1)?,
        };
        // SAFETY: as above; the buffer holds the bytes cuSPARSE asked for, and lives as long as
        // the descriptor
        unsafe {
            check(
                (api.spsv_buffer_size)(
                    s.handle,
                    NON_TRANSPOSE,
                    one.ptr(),
                    t.matrix.descriptor,
                    x.dense,
                    y.dense,
                    C_64F,
                    SPSV_ALGORITHM,
                    t.descriptor,
                    &mut bytes,
                ),
                "cusparseSpSV_bufferSize",
            )?;
            t._buffer = Buffer::with_bytes(&api.runtime, bytes)?;
            check(
                (api.spsv_analysis)(
                    s.handle,
                    NON_TRANSPOSE,
                    one.ptr(),
                    t.matrix.descriptor,
                    x.dense,
                    y.dense,
                    C_64F,
                    SPSV_ALGORITHM,
                    t.descriptor,
                    t._buffer.ptr,
                ),
                "cusparseSpSV_analysis",
            )?;
        }
        Ok(t)
    }

    /// y = T⁻¹ x.
    fn solve(&self, s: &Session, x: &DeviceVector, y: &DeviceVector) -> Result<()> {
        let one = Scalar(ONE);
        check(
            // SAFETY: analysed in new() on vectors of this length; the buffer is alive
            unsafe {
                (self.api.spsv_solve)(
                    s.handle,
                    NON_TRANSPOSE,
                    one.ptr(),
                    self.matrix.descriptor,
                    x.dense,
                    y.dense,
                    C_64F,
                    SPSV_ALGORITHM,
                    self.descriptor,
                )
            },
            "cusparseSpSV_solve",
        )
    }
}

impl Drop for Triangular {
    fn drop(&mut self) {
        let _serial = serial();
        // SAFETY: created once in new(), destroyed once, before its matrix and buffer
        unsafe { (self.api.spsv_destroy)(self.descriptor) };
    }
}

/// ILU(0) on the GPU: L, U and their transposes, and two work vectors.
struct Ilu {
    l: Triangular,
    u: Triangular,
    lt: Triangular,
    ut: Triangular,
    t1: DeviceVector,
    t2: DeviceVector,
}

impl Ilu {
    fn new(s: &Session, factors: &IluFactors) -> Result<Ilu> {
        let n = factors.n();
        let zeros = vec![ZERO; n];
        let (t1, t2) = (s.vector(&zeros)?, s.vector(&zeros)?);
        // a factor by rows, and its transpose by rows: its columns
        let both = |(starts, columns, values): (&[usize], &[usize], &[c64])| -> Result<_> {
            let rows = columns_as_rows(starts, columns)?;
            let ((t_starts, t_columns), order) = transposed(n, starts, columns);
            let t_values: Vec<c64> = order.iter().map(|&k| values[k]).collect();
            Ok((
                (rows, values.to_vec()),
                (columns_as_rows(&t_starts, &t_columns)?, t_values),
            ))
        };
        let ((l, l_values), (lt, lt_values)) = both(factors.lower())?;
        let ((u, u_values), (ut, ut_values)) = both(factors.upper())?;
        Ok(Ilu {
            l: Triangular::new(s, (&l, &l_values), true, true, &t1, &t2)?,
            u: Triangular::new(s, (&u, &u_values), false, false, &t1, &t2)?,
            lt: Triangular::new(s, (&lt, &lt_values), false, true, &t1, &t2)?,
            ut: Triangular::new(s, (&ut, &ut_values), true, false, &t1, &t2)?,
            t1,
            t2,
        })
    }
}

/// What QMR multiplies by: A, or A M⁻¹ with ILU(0)'s M.
struct Operator<'a> {
    a: &'a DeviceMatrix,
    at: Option<&'a DeviceMatrix>,
    ilu: Option<&'a Ilu>,
}

impl Operator<'_> {
    /// out = A v, or A M⁻¹ v = A U⁻¹ L⁻¹ v.
    fn apply(&self, s: &Session, v: &DeviceVector, out: &DeviceVector) -> Result<()> {
        match self.ilu {
            Some(m) => {
                m.l.solve(s, v, &m.t1)?;
                m.u.solve(s, &m.t1, &m.t2)?;
                s.product(self.a, &m.t2, out)
            }
            None => s.product(self.a, v, out),
        }
    }

    /// out = Aᵀ w, or (A M⁻¹)ᵀ w = L⁻ᵀ U⁻ᵀ Aᵀ w.
    fn apply_transpose(&self, s: &Session, w: &DeviceVector, out: &DeviceVector) -> Result<()> {
        let at = self.at.ok_or_else(|| error("the general QMR needs Aᵀ"))?;
        match self.ilu {
            Some(m) => {
                s.product(at, w, &m.t1)?;
                m.ut.solve(s, &m.t1, &m.t2)?;
                m.lt.solve(s, &m.t2, out)
            }
            None => s.product(at, w, out),
        }
    }
}

/// photonoxide's QMR on the GPU, as an iterative backend named `cusparse`.
pub struct GpuQmr {
    api: Arc<Api>,
}

impl GpuQmr {
    /// cuSPARSE and the CUDA runtime, found and loaded.
    ///
    /// # Errors
    ///
    /// Why not: not found, a function missing.
    pub fn load() -> std::result::Result<GpuQmr, String> {
        Api::load().map(|api| GpuQmr { api: Arc::new(api) })
    }
}

/// A Givens rotation (c, s) applied to (a, b), as `fdfd::krylov` has it.
fn rotate((c, s): (f64, c64), a: c64, b: c64) -> (c64, c64) {
    (c * a + s * b, -s.conj() * a + c * b)
}

impl IterativeSolver for GpuQmr {
    fn capabilities(&self) -> Capabilities {
        let mut c = Capabilities::new(
            "cusparse",
            self.api.version.clone(),
            "NVIDIA CUDA Toolkit licence (installed by the user)",
        );
        c.symmetric = true;
        c.transpose = true;
        c.threads = Threads::Library("the GPU".into());
        // CSR_ALG2's products and cuSPARSE's sums repeat bit for bit on one GPU; the tests
        // check it
        c.deterministic = true;
        c
    }

    fn qmr_run(&self, matrix: &Matrix<'_>, b: &[c64], stopping: Stopping) -> Result<QmrRun> {
        self.run(matrix, None, b, stopping)
    }

    fn qmr_run_ilu(
        &self,
        matrix: &Matrix<'_>,
        ilu: &IluFactors,
        b: &[c64],
        stopping: Stopping,
    ) -> Result<QmrRun> {
        if ilu.n() != matrix.n() {
            return Err(error(format!(
                "ILU(0)'s factors of {} unknowns for a matrix of {}",
                ilu.n(),
                matrix.n()
            )));
        }
        self.run(matrix, Some(ilu), b, stopping)
    }
}

impl GpuQmr {
    /// One run of QMR on A, or of the general QMR on A M⁻¹ with ILU(0)'s M.
    fn run(
        &self,
        matrix: &Matrix<'_>,
        ilu: Option<&IluFactors>,
        b: &[c64],
        stopping: Stopping,
    ) -> Result<QmrRun> {
        let _serial = serial();
        let n = matrix.n();
        if b.len() != n {
            return Err(error(format!(
                "the right-hand side needs {n} values, got {}",
                b.len()
            )));
        }
        if stopping.tolerance.is_nan() || stopping.tolerance <= 0.0 {
            return Err(error("the tolerance must be positive"));
        }
        let rho0 = b.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
        if rho0 == 0.0 {
            return Ok(QmrRun::Done(
                vec![ZERO; n],
                Convergence {
                    iterations: 0,
                    residual: 0.0,
                    history: Vec::new(),
                },
            ));
        }
        // ILU(0) preconditions the general QMR, as photonoxide's own
        let symmetric = matrix.form() == Form::Symmetric && ilu.is_none();
        let api = &self.api;
        let (cs, ri, values) = (
            matrix.column_starts(),
            matrix.row_indices(),
            matrix.values(),
        );
        index(values.len())?;
        // the handle first: cuSPARSE's descriptors made before it fail later, in SpMV
        let mut s = Session::new(api, n)?;
        // A by rows; for the general QMR, Aᵀ by rows too: A's columns as given. A symmetric
        // matrix's columns are its rows.
        let a = if symmetric {
            DeviceMatrix::new(api, &columns_as_rows(cs, ri)?, values)?
        } else {
            let ((starts, columns), order) = transposed(n, cs, ri);
            let by_rows: Vec<c64> = order.iter().map(|&k| values[k]).collect();
            DeviceMatrix::new(api, &columns_as_rows(&starts, &columns)?, &by_rows)?
        };
        let at = if symmetric {
            None
        } else {
            Some(DeviceMatrix::new(api, &columns_as_rows(cs, ri)?, values)?)
        };
        let zeros = vec![ZERO; n];
        let device_b = s.vector(b)?;
        let x = s.vector(&zeros)?;
        let r = s.vector(b)?;
        let (av, tmp) = (s.vector(&zeros)?, s.vector(&zeros)?);
        s.prepare(&a, &x, &av)?;
        if let Some(at) = &at {
            s.prepare(at, &x, &av)?;
        }
        let factors = ilu.map(|f| Ilu::new(&s, f)).transpose()?;
        let op = Operator {
            a: &a,
            at: at.as_ref(),
            ilu: factors.as_ref(),
        };
        let (mut v, mut v_old, mut v_next) =
            (s.vector(&zeros)?, s.vector(&zeros)?, s.vector(&zeros)?);
        let (mut p, mut p_old, mut p_older) =
            (s.vector(&zeros)?, s.vector(&zeros)?, s.vector(&zeros)?);
        // the left vectors, for the general QMR only
        let (mut w, mut w_old, mut w_next, atw) = if symmetric {
            (None, None, None, None)
        } else {
            (
                Some(s.vector(&zeros)?),
                Some(s.vector(&zeros)?),
                Some(s.vector(&zeros)?),
                Some(s.vector(&zeros)?),
            )
        };
        // v1 = w1 = b / rho0
        s.copy(&device_b, &v)?;
        s.scale(c64::new(1.0 / rho0, 0.0), &v)?;
        if let Some(w) = &w {
            s.copy(&v, w)?;
        }
        let mut d = s.dotu(w.as_ref().unwrap_or(&v), &v)?;
        let mut d_old = ONE;
        let (mut rho, mut xi) = (1.0, 1.0);
        let mut rotations: [Option<(f64, c64)>; 2] = [None, None];
        let mut tau = c64::new(rho0, 0.0);
        let mut history = Vec::new();
        for iteration in 1..=stopping.max_iterations {
            if d.norm() < 1e-14 {
                return Ok(QmrRun::Broken(x.read()?, history, d.norm()));
            }
            op.apply(&s, &v, &av)?;
            if let (Some(w), Some(atw)) = (&w, &atw) {
                op.apply_transpose(&s, w, atw)?;
            }
            let alpha = s.dotu(w.as_ref().unwrap_or(&v), &av)? / d;
            let beta_v = xi * d / d_old;
            let beta_w = rho * d / d_old;
            // v_{n+1} = A v_n − α v_n − β_v v_{n−1}, and w_{n+1} from Aᵀ w_n likewise
            s.copy(&av, &v_next)?;
            s.axpy(-alpha, &v, &v_next)?;
            s.axpy(-beta_v, &v_old, &v_next)?;
            let rho2 = s.norm2(&v_next)?;
            let xi2 = match (&w, &w_old, &w_next, &atw) {
                (Some(w), Some(w_old), Some(w_next), Some(atw)) => {
                    s.copy(atw, w_next)?;
                    s.axpy(-alpha, w, w_next)?;
                    s.axpy(-beta_w, w_old, w_next)?;
                    s.norm2(w_next)?
                }
                _ => rho2,
            };
            let (rho_next, xi_next) = (rho2.sqrt(), xi2.sqrt());
            let (mut theta, mut epsilon, mut mu) =
                (ZERO, if iteration > 1 { beta_v } else { ZERO }, alpha);
            if let Some(g) = rotations[0] {
                (theta, epsilon) = rotate(g, ZERO, epsilon);
            }
            if let Some(g) = rotations[1] {
                (epsilon, mu) = rotate(g, epsilon, mu);
            }
            let nu = c64::new(rho_next, 0.0);
            let (c, sn) = if mu.norm() > 0.0 {
                let c = mu.norm() / (mu.norm_sqr() + rho_next * rho_next).sqrt();
                (c, (c * nu / mu).conj())
            } else {
                (0.0, ONE)
            };
            let delta = c * mu + sn * nu;
            rotations = [rotations[1], Some((c, sn))];
            let tau_n = c * tau;
            tau = -sn.conj() * tau;
            let ended = rho_next == 0.0 || xi_next == 0.0;
            // p_n = (v_n − ε p_{n−1} − θ p_{n−2}) / δ, x += τ_n p_n
            s.copy(&v, &p)?;
            s.axpy(-epsilon, &p_old, &p)?;
            s.axpy(-theta, &p_older, &p)?;
            s.scale(ONE / delta, &p)?;
            s.axpy(tau_n, &p, &x)?;
            let mut d_next = ZERO;
            if !ended {
                // the next unit vectors, and r_n = |s|² r_{n−1} + c τ̃_{n+1} v_{n+1}
                s.scale(c64::new(1.0 / rho_next, 0.0), &v_next)?;
                s.axpby(c * tau, &v_next, c64::new(sn.norm_sqr(), 0.0), &r)?;
                d_next = match &w_next {
                    Some(w_next) => {
                        s.scale(c64::new(1.0 / xi_next, 0.0), w_next)?;
                        s.dotu(w_next, &v_next)?
                    }
                    None => s.dotu(&v_next, &v_next)?,
                };
            }
            let r2 = s.norm2(&r)?;
            std::mem::swap(&mut p_older, &mut p_old);
            std::mem::swap(&mut p_old, &mut p);
            let updated = r2.sqrt() / rho0;
            history.push(updated);
            if updated <= stopping.tolerance || ended {
                // the true residual b − A x
                op.apply(&s, &x, &tmp)?;
                s.axpby(ONE, &device_b, -ONE, &tmp)?;
                let residual = s.norm2(&tmp)?.sqrt() / rho0;
                if residual <= stopping.tolerance {
                    api.runtime.synchronize("QMR on the GPU")?;
                    return Ok(QmrRun::Done(
                        x.read()?,
                        Convergence {
                            iterations: iteration,
                            residual,
                            history,
                        },
                    ));
                }
                if ended {
                    return Err(error(format!(
                        "the Lanczos process ended at step {iteration} with the residual at {residual:e}"
                    )));
                }
                s.copy(&tmp, &r)?;
            }
            std::mem::swap(&mut v_old, &mut v);
            std::mem::swap(&mut v, &mut v_next);
            if let (Some(w_old), Some(w), Some(w_next)) = (&mut w_old, &mut w, &mut w_next) {
                std::mem::swap(w_old, w);
                std::mem::swap(w, w_next);
            }
            (rho, xi) = (rho_next, xi_next);
            d_old = d;
            d = d_next;
        }
        Err(error(format!(
            "no convergence to {:e} in {} iterations (the residual is at {:e})",
            stopping.tolerance,
            stopping.max_iterations,
            history.last().copied().unwrap_or(f64::NAN)
        )))
    }
}
