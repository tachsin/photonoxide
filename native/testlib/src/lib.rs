//! A C library for photonoxide-native's tests, so that discovery, loading, version reading and
//! errors are tested in CI without any real library installed. It solves small dense complex
//! systems by Gaussian elimination with partial pivoting.
//!
//! Every function returns 0 on success, 1 for an invalid argument and 2 for a singular matrix.

use std::os::raw::c_int;

use num_complex::Complex64 as c64;

/// The library's version, 1.2.3.
///
/// # Safety
///
/// Each pointer is null or points to a writable `int`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pxtest_version(
    major: *mut c_int,
    minor: *mut c_int,
    patch: *mut c_int,
) -> c_int {
    if major.is_null() || minor.is_null() || patch.is_null() {
        return 1;
    }
    // SAFETY: the three pointers are non-null and, by the contract, writable ints
    unsafe {
        *major = 1;
        *minor = 2;
        *patch = 3;
    }
    0
}

/// The bytes of the library's integers: 8.
#[unsafe(no_mangle)]
pub extern "C" fn pxtest_index_bytes() -> c_int {
    size_of::<i64>() as c_int
}

/// Solves A x = b, x written over b. `a` is n × n by columns and `b` has n entries, each
/// complex number two doubles (real, imaginary).
///
/// # Safety
///
/// `a` points to 2n² readable doubles and `b` to 2n writable ones, or either is null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pxtest_dense_solve(n: i64, a: *const f64, b: *mut f64) -> c_int {
    let Ok(n) = usize::try_from(n) else {
        return 1;
    };
    if n == 0 || a.is_null() || b.is_null() {
        return 1;
    }
    // SAFETY: by the contract, a holds 2n² doubles and b 2n, and they don't overlap
    let (a, b) = unsafe {
        (
            std::slice::from_raw_parts(a, 2 * n * n),
            std::slice::from_raw_parts_mut(b, 2 * n),
        )
    };
    let mut m: Vec<c64> = a.chunks(2).map(|p| c64::new(p[0], p[1])).collect();
    let mut x: Vec<c64> = b.chunks(2).map(|p| c64::new(p[0], p[1])).collect();
    for k in 0..n {
        let p = (k..n)
            .max_by(|&i, &j| m[k * n + i].norm().total_cmp(&m[k * n + j].norm()))
            .unwrap_or(k);
        if m[k * n + p].norm() == 0.0 {
            return 2;
        }
        for j in 0..n {
            m.swap(j * n + k, j * n + p);
        }
        x.swap(k, p);
        for i in k + 1..n {
            let f = m[k * n + i] / m[k * n + k];
            for j in k..n {
                let v = m[j * n + k];
                m[j * n + i] -= f * v;
            }
            let v = x[k];
            x[i] -= f * v;
        }
    }
    for k in (0..n).rev() {
        let s: c64 = (k + 1..n).map(|j| m[j * n + k] * x[j]).sum();
        x[k] = (x[k] - s) / m[k * n + k];
    }
    for (pair, v) in b.chunks_mut(2).zip(x) {
        pair[0] = v.re;
        pair[1] = v.im;
    }
    0
}
