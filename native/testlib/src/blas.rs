//! A BLAS and LAPACK small enough to read, for the tests of how photonoxide-native finds a
//! library's interface: Fortran's names (`zgemm_`, `ztrsm_`, `zgetrf_`) taking 64-bit integers,
//! as pip's `scipy-openblas64` has them, so that a name is no proof of the integers; Fortran's
//! `zdotc_` returning its result through a first argument, as f2c and Intel's compilers do; and
//! the C interfaces (`cblas_zgemm`, `cblas_ztrsm`, `LAPACKE_zgetrf_work`) with 32-bit integers.
//! No `zgemmt`: `zgemm` computes both triangles.
//!
//! Column-major only, the operations photonoxide asks for (A not transposed in a product), one
//! entry at a time.

use std::os::raw::{c_char, c_int};

use num_complex::Complex64 as c64;

/// A matrix by columns behind a pointer, with its leading dimension.
#[derive(Clone, Copy)]
struct View {
    p: *mut c64,
    ld: usize,
}

impl View {
    /// # Safety
    ///
    /// (i, j) is inside the matrix the pointer and leading dimension describe.
    unsafe fn at(self, i: usize, j: usize) -> c64 {
        // SAFETY: the caller's
        unsafe { *self.p.add(i + j * self.ld) }
    }

    /// # Safety
    ///
    /// As [`View::at`], and the matrix is writable.
    unsafe fn set(self, i: usize, j: usize, v: c64) {
        // SAFETY: the caller's
        unsafe { *self.p.add(i + j * self.ld) = v }
    }
}

fn size(n: i64) -> usize {
    usize::try_from(n).expect("a dimension of at least 0")
}

/// C ← α A op(B) + β C, A m × k.
///
/// # Safety
///
/// The matrices are as large as their dimensions say.
#[allow(clippy::too_many_arguments)]
unsafe fn gemm(
    transposed: bool,
    m: usize,
    n: usize,
    k: usize,
    alpha: c64,
    a: View,
    b: View,
    beta: c64,
    c: View,
) {
    for j in 0..n {
        for i in 0..m {
            let mut sum = c64::new(0.0, 0.0);
            for r in 0..k {
                // SAFETY: inside A and op(B), by the caller's promise
                let (x, y) =
                    unsafe { (a.at(i, r), if transposed { b.at(j, r) } else { b.at(r, j) }) };
                sum += x * y;
            }
            // SAFETY: inside C
            unsafe { c.set(i, j, alpha * sum + beta * c.at(i, j)) };
        }
    }
}

/// B ← α op(T)⁻¹ B (from the left) or α B op(T)⁻¹ (from the right), T triangular.
///
/// # Safety
///
/// T is of the order of B's rows (left) or columns (right); the matrices are as large as
/// their dimensions say.
#[allow(clippy::too_many_arguments)]
unsafe fn trsm(
    left: bool,
    lower: bool,
    transposed: bool,
    unit: bool,
    m: usize,
    n: usize,
    alpha: c64,
    t: View,
    b: View,
) {
    let order = if left { m } else { n };
    // op(T) whole, the other triangle zero
    let mut full = vec![c64::new(0.0, 0.0); order * order];
    for j in 0..order {
        for i in 0..order {
            let inside = if lower { i >= j } else { i <= j };
            let v = if i == j && unit {
                c64::new(1.0, 0.0)
            } else if inside {
                // SAFETY: inside T
                unsafe { t.at(i, j) }
            } else {
                c64::new(0.0, 0.0)
            };
            let (r, c) = if transposed { (j, i) } else { (i, j) };
            full[r + c * order] = v;
        }
    }
    let op_lower = lower != transposed;
    let at = |i: usize, j: usize| full[i + j * order];
    // a vector x with M x = α y, M = op(T) (from the left) or op(T)ᵀ (from the right: X M = B
    // row by row is Mᵀ xᵀ = bᵀ), solved in place
    let solve = |x: &mut [c64], transpose: bool| {
        let m_lower = op_lower != transpose;
        let entry = |i: usize, j: usize| if transpose { at(j, i) } else { at(i, j) };
        let order_of: Vec<usize> = if m_lower {
            (0..order).collect()
        } else {
            (0..order).rev().collect()
        };
        for &i in &order_of {
            let mut s = alpha * x[i];
            for &j in &order_of {
                if j == i {
                    break;
                }
                s -= entry(i, j) * x[j];
            }
            x[i] = s / entry(i, i);
        }
    };
    if left {
        for j in 0..n {
            // SAFETY: inside B
            let mut x: Vec<c64> = (0..m).map(|i| unsafe { b.at(i, j) }).collect();
            solve(&mut x, false);
            for (i, v) in x.into_iter().enumerate() {
                // SAFETY: inside B
                unsafe { b.set(i, j, v) };
            }
        }
    } else {
        for i in 0..m {
            // SAFETY: inside B
            let mut x: Vec<c64> = (0..n).map(|j| unsafe { b.at(i, j) }).collect();
            solve(&mut x, true);
            for (j, v) in x.into_iter().enumerate() {
                // SAFETY: inside B
                unsafe { b.set(i, j, v) };
            }
        }
    }
}

/// The LU of the n × n A with partial pivoting by |re| + |im|, as LAPACK's: the pivots from 1,
/// and info (0, or the first zero pivot from 1).
///
/// # Safety
///
/// A is n × n.
unsafe fn getrf(a: View, pivots: &mut [i64]) -> i64 {
    let n = pivots.len();
    let mut info = 0;
    let magnitude = |z: c64| z.re.abs() + z.im.abs();
    for (j, pivot) in pivots.iter_mut().enumerate() {
        // SAFETY: inside A, here and below
        let p = (j..n)
            .max_by(|&x, &y| unsafe { magnitude(a.at(x, j)).total_cmp(&magnitude(a.at(y, j))) })
            .unwrap_or(j);
        *pivot = p as i64 + 1;
        if p != j {
            for c in 0..n {
                // SAFETY: as above
                unsafe {
                    let t = a.at(j, c);
                    a.set(j, c, a.at(p, c));
                    a.set(p, c, t);
                }
            }
        }
        // SAFETY: as above
        let pivot = unsafe { a.at(j, j) };
        if pivot == c64::new(0.0, 0.0) {
            if info == 0 {
                info = j as i64 + 1;
            }
            continue;
        }
        for i in j + 1..n {
            // SAFETY: as above
            unsafe {
                let l = a.at(i, j) / pivot;
                a.set(i, j, l);
                for c in j + 1..n {
                    a.set(i, c, a.at(i, c) - l * a.at(j, c));
                }
            }
        }
    }
    info
}

/// Fortran's `zgemm`, with 64-bit integers; the characters' lengths after the last argument
/// aren't read.
///
/// # Safety
///
/// BLAS's contract for `zgemm`, with `transa` 'N'.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zgemm_(
    transa: *const c_char,
    transb: *const c_char,
    m: *const i64,
    n: *const i64,
    k: *const i64,
    alpha: *const c64,
    a: *mut c64,
    lda: *const i64,
    b: *mut c64,
    ldb: *const i64,
    beta: *const c64,
    c: *mut c64,
    ldc: *const i64,
) {
    // SAFETY: BLAS's contract: every argument is readable, the matrices as large as they say
    unsafe {
        assert_eq!(*transa as u8, b'N', "A not transposed");
        gemm(
            *transb as u8 == b'T',
            size(*m),
            size(*n),
            size(*k),
            *alpha,
            View {
                p: a,
                ld: size(*lda),
            },
            View {
                p: b,
                ld: size(*ldb),
            },
            *beta,
            View {
                p: c,
                ld: size(*ldc),
            },
        );
    }
}

/// Fortran's `ztrsm`, with 64-bit integers.
///
/// # Safety
///
/// BLAS's contract for `ztrsm`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ztrsm_(
    side: *const c_char,
    uplo: *const c_char,
    transa: *const c_char,
    diag: *const c_char,
    m: *const i64,
    n: *const i64,
    alpha: *const c64,
    a: *mut c64,
    lda: *const i64,
    b: *mut c64,
    ldb: *const i64,
) {
    // SAFETY: BLAS's contract
    unsafe {
        trsm(
            *side as u8 == b'L',
            *uplo as u8 == b'L',
            *transa as u8 == b'T',
            *diag as u8 == b'U',
            size(*m),
            size(*n),
            *alpha,
            View {
                p: a,
                ld: size(*lda),
            },
            View {
                p: b,
                ld: size(*ldb),
            },
        );
    }
}

/// Fortran's `zgetrf`, with 64-bit integers (m = n only).
///
/// # Safety
///
/// LAPACK's contract for `zgetrf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zgetrf_(
    m: *const i64,
    n: *const i64,
    a: *mut c64,
    lda: *const i64,
    ipiv: *mut i64,
    info: *mut i64,
) {
    // SAFETY: LAPACK's contract: n pivots to write, A n × n
    unsafe {
        let n = size(*n);
        assert_eq!(size(*m), n, "a square matrix");
        let pivots = std::slice::from_raw_parts_mut(ipiv, n);
        *info = getrf(
            View {
                p: a,
                ld: size(*lda),
            },
            pivots,
        );
    }
}

/// Fortran's `zdotc`, with 64-bit integers, its result through a first argument (f2c's and
/// Intel's convention). Positive increments only.
///
/// # Safety
///
/// BLAS's contract for `zdotc`, the result's place first.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zdotc_(
    result: *mut c64,
    n: *const i64,
    x: *const c64,
    incx: *const i64,
    y: *const c64,
    incy: *const i64,
) {
    // SAFETY: BLAS's contract
    unsafe {
        let (n, incx, incy) = (size(*n), size(*incx), size(*incy));
        let mut sum = c64::new(0.0, 0.0);
        for k in 0..n {
            sum += (*x.add(k * incx)).conj() * *y.add(k * incy);
        }
        *result = sum;
    }
}

const COLUMN_MAJOR: c_int = 102;

fn small(n: c_int) -> usize {
    usize::try_from(n).expect("a dimension of at least 0")
}

/// CBLAS's `cblas_zgemm`, with 32-bit integers, column-major.
///
/// # Safety
///
/// CBLAS's contract, A not transposed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cblas_zgemm(
    layout: c_int,
    transa: c_int,
    transb: c_int,
    m: c_int,
    n: c_int,
    k: c_int,
    alpha: *const c64,
    a: *mut c64,
    lda: c_int,
    b: *mut c64,
    ldb: c_int,
    beta: *const c64,
    c: *mut c64,
    ldc: c_int,
) {
    assert_eq!(
        (layout, transa),
        (COLUMN_MAJOR, 111),
        "column-major, A not transposed"
    );
    // SAFETY: CBLAS's contract
    unsafe {
        gemm(
            transb == 112,
            small(m),
            small(n),
            small(k),
            *alpha,
            View {
                p: a,
                ld: small(lda),
            },
            View {
                p: b,
                ld: small(ldb),
            },
            *beta,
            View {
                p: c,
                ld: small(ldc),
            },
        );
    }
}

/// CBLAS's `cblas_ztrsm`, with 32-bit integers, column-major.
///
/// # Safety
///
/// CBLAS's contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cblas_ztrsm(
    layout: c_int,
    side: c_int,
    uplo: c_int,
    transa: c_int,
    diag: c_int,
    m: c_int,
    n: c_int,
    alpha: *const c64,
    a: *mut c64,
    lda: c_int,
    b: *mut c64,
    ldb: c_int,
) {
    assert_eq!(layout, COLUMN_MAJOR, "column-major");
    // SAFETY: CBLAS's contract
    unsafe {
        trsm(
            side == 141,
            uplo == 122,
            transa == 112,
            diag == 132,
            small(m),
            small(n),
            *alpha,
            View {
                p: a,
                ld: small(lda),
            },
            View {
                p: b,
                ld: small(ldb),
            },
        );
    }
}

/// LAPACKE's `LAPACKE_zgetrf_work`, with 32-bit integers, column-major (m = n only).
///
/// # Safety
///
/// LAPACKE's contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LAPACKE_zgetrf_work(
    layout: c_int,
    m: c_int,
    n: c_int,
    a: *mut c64,
    lda: c_int,
    ipiv: *mut c_int,
) -> c_int {
    assert_eq!((layout, m), (COLUMN_MAJOR, n), "column-major, square");
    let n = small(n);
    let mut pivots = vec![0i64; n];
    // SAFETY: LAPACKE's contract: A n × n, n pivots to write
    unsafe {
        let info = getrf(
            View {
                p: a,
                ld: small(lda),
            },
            &mut pivots,
        );
        for (k, p) in pivots.into_iter().enumerate() {
            *ipiv.add(k) = p as c_int;
        }
        info as c_int
    }
}
