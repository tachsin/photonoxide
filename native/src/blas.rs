//! A BLAS and LAPACK library as the dense kernels of photonoxide's multifrontal fronts (#186):
//! [`DenseKernels`] by `zgemm`, `ztrsm` and `zgetrf`, the standard interface every vendor's
//! library has.
//!
//! - **Providers:** OpenBLAS ([`Blas::openblas`]), Intel oneMKL through `mkl_rt`
//!   ([`Blas::mkl`]) and Apple Accelerate ([`Blas::accelerate`]). AMD's AOCL and Arm's
//!   Performance Libraries have the same interface and aren't loaded yet.
//! - **The interface** is Fortran's, with 32-bit integers (LP64): `zgemm_` and the others,
//!   every argument by address, each character argument's length after the last one as
//!   gfortran passes it (a library that doesn't expect the lengths doesn't read them). A
//!   build with 64-bit integers has other names (`zgemm_64_`) or says so (oneMKL's interface
//!   layer, asked), and isn't used. Nothing here returns a complex number, so that convention
//!   doesn't arise. What was loaded is checked before it is offered: photonoxide's solver
//!   with these kernels must pass the smoke test against its own ([`crate::offer`]).
//! - **`zgemmt`,** the product into one triangle that an L D Lᵀ's updates are, isn't standard:
//!   it is used where the library has it (oneMKL, OpenBLAS) and `zgemm` computes both
//!   triangles where it doesn't. OpenBLAS has it from 0.3.22, and its is used from 0.3.27:
//!   with 0.3.26's (Ubuntu 24.04's package) the solver crashed.
//! - **Threads.** A call says whether it may thread. Fronts factorized side by side on rayon's
//!   threads must each stay on their own thread, while a front near the root has them all, so
//!   the count is set for the calling thread around each call: `mkl_set_num_threads_local`,
//!   `openblas_set_num_threads_local` (OpenBLAS 0.3.27 and later). An OpenBLAS without it is
//!   held to one thread for good; Accelerate has no such setting here and is left to itself.
//! - **Not declared deterministic.**

use std::ffi::{CStr, c_char, c_int, c_void};
use std::path::Path;
use std::sync::Arc;

use num_complex::Complex64 as c64;
use photonoxide::backend::Threads;
use photonoxide::backend::dense::{Block, BlockMut, DenseKernels, Kernels, Product};

use crate::Probe;
use crate::discovery::Spec;
use crate::intel::{Mkl, mkl};
use crate::library::{Library, load};

/// OpenBLAS, with its LAPACK.
pub const OPENBLAS: Spec = Spec {
    name: "OpenBLAS",
    files: if cfg!(windows) {
        &["libopenblas.dll", "openblas.dll"]
    } else if cfg!(target_os = "macos") {
        &["libopenblas.0.dylib", "libopenblas.dylib"]
    } else {
        &["libopenblas.so.0", "libopenblas.so"]
    },
    variables: &["PHOTONOXIDE_OPENBLAS"],
    install: Vec::new,
    wheel: None,
};

const FRAMEWORK: &str = "/System/Library/Frameworks/Accelerate.framework/Accelerate";

type Int = *const c_int;
type Char = *const c_char;
/// `zgemm(transa, transb, m, n, k, alpha, a, lda, b, ldb, beta, c, ldc)` and two lengths.
type Zgemm = unsafe extern "C" fn(
    Char,
    Char,
    Int,
    Int,
    Int,
    *const c64,
    *const c64,
    Int,
    *const c64,
    Int,
    *const c64,
    *mut c64,
    Int,
    usize,
    usize,
);
/// `zgemmt(uplo, transa, transb, n, k, alpha, a, lda, b, ldb, beta, c, ldc)` and three lengths.
type Zgemmt = unsafe extern "C" fn(
    Char,
    Char,
    Char,
    Int,
    Int,
    *const c64,
    *const c64,
    Int,
    *const c64,
    Int,
    *const c64,
    *mut c64,
    Int,
    usize,
    usize,
    usize,
);
/// `ztrsm(side, uplo, transa, diag, m, n, alpha, a, lda, b, ldb)` and four lengths.
type Ztrsm = unsafe extern "C" fn(
    Char,
    Char,
    Char,
    Char,
    Int,
    Int,
    *const c64,
    *const c64,
    Int,
    *mut c64,
    Int,
    usize,
    usize,
    usize,
    usize,
);
/// `zgetrf(m, n, a, lda, ipiv, info)`.
type Zgetrf = unsafe extern "C" fn(Int, Int, *mut c64, Int, *mut c_int, *mut c_int);
type SetInt = unsafe extern "C" fn(c_int) -> c_int;
type SetIntOnly = unsafe extern "C" fn(c_int);

/// How a call's threads are set.
enum Threading {
    /// `mkl_set_num_threads_local`.
    Mkl(Arc<Mkl>),
    /// `openblas_set_num_threads_local`.
    OpenBlas(SetInt),
    /// Not set per call: the library was told one thread, or decides itself.
    Fixed,
}

/// A BLAS and LAPACK library's kernels.
pub struct Blas {
    what: Kernels,
    zgemm: Zgemm,
    zgemmt: Option<Zgemmt>,
    ztrsm: Ztrsm,
    zgetrf: Zgetrf,
    threading: Threading,
    // last: the functions above must not outlive it (oneMKL's is kept by `threading`)
    _library: Option<Library>,
}

/// A symbol's first name that the library has.
///
/// # Safety
///
/// As [`Library::function`]: `F` is each name's C signature.
unsafe fn first<F: Copy>(library: &Library, names: &[String]) -> photonoxide::Result<F> {
    let mut last = None;
    for name in names {
        // SAFETY: the caller's
        match unsafe { library.function::<F>(name) } {
            Ok(f) => return Ok(f),
            Err(e) => last = Some(e),
        }
    }
    Err(last.expect("at least one name"))
}

/// The four routines of a library, by these names for `zgemm` and the like.
///
/// # Safety
///
/// The library's routines of these names must be BLAS's and LAPACK's with 32-bit integers.
unsafe fn routines(
    library: &Library,
    name: impl Fn(&str) -> Vec<String>,
) -> photonoxide::Result<(Zgemm, Option<Zgemmt>, Ztrsm, Zgetrf)> {
    // SAFETY: the caller's: each type is the routine's Fortran interface, by address, with
    // the character arguments' lengths after the last argument
    unsafe {
        Ok((
            first(library, &name("zgemm"))?,
            first(library, &name("zgemmt")).ok(),
            first(library, &name("ztrsm"))?,
            first(library, &name("zgetrf"))?,
        ))
    }
}

fn reason(e: photonoxide::Error) -> String {
    match e {
        photonoxide::Error::InvalidValue { reason, .. } => reason,
        other => other.to_string(),
    }
}

impl Blas {
    /// OpenBLAS found and loaded.
    ///
    /// # Errors
    ///
    /// Why not: not found, not loadable, or without the routines.
    pub fn openblas() -> std::result::Result<Blas, String> {
        let (discovery, library) = load(&OPENBLAS, None, |l| {
            // SAFETY: only resolved; the type is zgetrf's
            unsafe { l.function::<Zgetrf>("zgetrf_") }.map(drop)
        });
        let library = library.ok_or_else(|| discovery.reason())?;
        // SAFETY: OpenBLAS's Fortran names; a build with 64-bit integers has other names
        // (the suffix 64_) unless it was built to stand in for this one, which the smoke test
        // then refuses
        let (zgemm, zgemmt, ztrsm, zgetrf) =
            unsafe { routines(&library, |r| vec![format!("{r}_")]) }.map_err(reason)?;
        // SAFETY: `char *openblas_get_config(void)`, a static string
        let config = unsafe {
            library.function::<unsafe extern "C" fn() -> *const c_char>("openblas_get_config")
        }
        .ok()
        .map(|f| {
            // SAFETY: as above
            let text = unsafe { f() };
            if text.is_null() {
                String::new()
            } else {
                // SAFETY: a C string that lives as long as the library
                unsafe { CStr::from_ptr(text) }
                    .to_string_lossy()
                    .into_owned()
            }
        })
        .unwrap_or_default();
        let version = openblas_version(&config);
        let zgemmt = zgemmt.filter(|_| at_least(&version, [0, 3, 27]));
        // SAFETY: `int openblas_set_num_threads_local(int)`, from OpenBLAS 0.3.27
        let local = unsafe { library.function::<SetInt>("openblas_set_num_threads_local") }.ok();
        let (threading, threads) = match local {
            Some(set) => (
                Threading::OpenBlas(set),
                Threads::Library(
                    "OpenBLAS's own, on rayon's thread count (RAYON_NUM_THREADS) by \
                     openblas_set_num_threads_local"
                        .into(),
                ),
            ),
            None => {
                // SAFETY: `void openblas_set_num_threads(int)`
                let global = unsafe { library.function::<SetIntOnly>("openblas_set_num_threads") };
                if let Ok(set) = global {
                    // SAFETY: as above: any positive count
                    unsafe { set(1) };
                }
                (Threading::Fixed, Threads::One)
            }
        };
        let mut what = Kernels::new("openblas", version, "BSD-3-Clause (installed by the user)");
        what.threads = threads;
        Ok(Blas {
            what,
            zgemm,
            zgemmt,
            ztrsm,
            zgetrf,
            threading,
            _library: Some(library),
        })
    }

    /// oneMKL's BLAS and LAPACK, of the `mkl_rt` [`crate::intel::mkl`] set up.
    ///
    /// # Errors
    ///
    /// Why not: oneMKL not found or not set up, or its interface layer is the 64-bit one.
    pub fn mkl() -> std::result::Result<Blas, String> {
        let (_, found) = mkl();
        let mkl = found?;
        let library = mkl.library();
        // SAFETY: `int MKL_Set_Interface_Layer(int code)`: asking for LP64 (0), which it
        // returns unless another layer was already in use
        let layer = unsafe { library.function::<SetInt>("MKL_Set_Interface_Layer") }
            .map(|set| {
                // SAFETY: as above
                unsafe { set(0) }
            })
            .map_err(reason)?;
        // MKL_INTERFACE_ILP64 is 1, and 2 marks GNU's convention beside either
        if layer & 1 != 0 {
            return Err(
                "oneMKL's interface layer is ILP64 (MKL_INTERFACE_LAYER): its BLAS takes 64-bit \
                 integers, which photonoxide doesn't call it with"
                    .into(),
            );
        }
        // SAFETY: mkl_rt's Fortran names, in the LP64 layer checked above
        let (zgemm, zgemmt, ztrsm, zgetrf) =
            unsafe { routines(library, |r| vec![format!("{r}_"), r.to_owned()]) }
                .map_err(reason)?;
        let mut what = Kernels::new(
            "mkl",
            mkl.version.clone(),
            "Intel Simplified Software License (installed by the user)",
        );
        what.threads = Threads::Library(mkl.threads());
        Ok(Blas {
            what,
            zgemm,
            zgemmt,
            ztrsm,
            zgetrf,
            threading: Threading::Mkl(mkl),
            _library: None,
        })
    }

    /// Accelerate's BLAS and LAPACK, on macOS.
    ///
    /// # Errors
    ///
    /// Why not: another system.
    pub fn accelerate() -> std::result::Result<Blas, String> {
        if !cfg!(target_os = "macos") {
            return Err("Accelerate is part of macOS: it isn't on this system".into());
        }
        let library = Library::open(Path::new(FRAMEWORK)).map_err(reason)?;
        // SAFETY: Accelerate's Fortran names, 32-bit integers both: its current LAPACK's
        // ($NEWLAPACK, from macOS 13.3) before the one it keeps for older programs
        let (zgemm, zgemmt, ztrsm, zgetrf) = unsafe {
            routines(&library, |r| {
                vec![format!("{r}$NEWLAPACK"), format!("{r}_")]
            })
        }
        .map_err(reason)?;
        let mut what = Kernels::new(
            "accelerate",
            format!("macOS {}", macos_version()),
            "part of macOS, under Apple's licence for it",
        );
        what.threads = Threads::Library("Accelerate's own, which a call doesn't set".into());
        Ok(Blas {
            what,
            zgemm,
            zgemmt,
            ztrsm,
            zgetrf,
            threading: Threading::Fixed,
            _library: Some(library),
        })
    }

    /// Whether products into one triangle are the library's own (`zgemmt`).
    pub fn has_triangular_product(&self) -> bool {
        self.zgemmt.is_some()
    }

    /// Runs `call` with the library's threads set for it on this thread.
    fn threaded<R>(&self, threaded: bool, call: impl FnOnce() -> R) -> R {
        let threads = if threaded {
            rayon::current_num_threads().max(1)
        } else {
            1
        };
        match &self.threading {
            Threading::Mkl(mkl) => {
                let before = mkl.set_threads_here(threads);
                let result = call();
                mkl.set_threads_here(usize::try_from(before).unwrap_or(0));
                result
            }
            Threading::OpenBlas(set) => {
                let count = c_int::try_from(threads).unwrap_or(c_int::MAX);
                // SAFETY: `int openblas_set_num_threads_local(int)`: a positive count for
                // this thread's calls; it returns the one before, restored after
                let before = unsafe { set(count) };
                let result = call();
                // SAFETY: as above
                unsafe { set(before) };
                result
            }
            Threading::Fixed => call(),
        }
    }
}

/// OpenBLAS's version in its configuration's words ("OpenBLAS 0.3.26 DYNAMIC_ARCH ...").
fn openblas_version(config: &str) -> String {
    config
        .split_whitespace()
        .find(|word| word.starts_with(|c: char| c.is_ascii_digit()))
        .unwrap_or("unknown")
        .to_owned()
}

/// Whether a version in numbers and points is this one or a later one.
fn at_least(version: &str, least: [u32; 3]) -> bool {
    let mut numbers = version.split('.').map(|part| {
        let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
        digits.parse::<u32>().unwrap_or(0)
    });
    let got: [u32; 3] = std::array::from_fn(|_| numbers.next().unwrap_or(0));
    got >= least
}

/// macOS's version, from `kern.osproductversion`.
fn macos_version() -> String {
    type Sysctl =
        unsafe extern "C" fn(*const c_char, *mut c_void, *mut usize, *mut c_void, usize) -> c_int;
    let Ok(system) = Library::open(Path::new("/usr/lib/libSystem.B.dylib")) else {
        return "unknown".into();
    };
    // SAFETY: sysctlbyname's declaration (sys/sysctl.h)
    let Ok(sysctl) = (unsafe { system.function::<Sysctl>("sysctlbyname") }) else {
        return "unknown".into();
    };
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

/// A dimension as BLAS takes it.
fn int(n: usize) -> c_int {
    c_int::try_from(n).expect("a front's dimension is below 2³¹")
}

const ONE: c64 = c64::new(1.0, 0.0);

fn letter(c: u8) -> c_char {
    c as c_char
}

impl DenseKernels for Blas {
    fn kernels(&self) -> Kernels {
        self.what.clone()
    }

    fn multiply(
        &self,
        mut c: BlockMut<'_>,
        a: Block<'_>,
        b: Block<'_>,
        product: Product,
        alpha: c64,
        threaded: bool,
    ) {
        let (m, n, r) = (c.rows(), c.columns(), a.columns());
        assert_eq!(a.rows(), m, "A's rows are C's");
        if product.transposed {
            assert_eq!((b.rows(), b.columns()), (n, r), "Bᵀ is r × n");
        } else {
            assert_eq!((b.rows(), b.columns()), (r, n), "B is r × n");
        }
        if m == 0 || n == 0 || r == 0 {
            return;
        }
        let (no, transb) = (
            letter(b'N'),
            letter(if product.transposed { b'T' } else { b'N' }),
        );
        let (mi, ni, ri) = (int(m), int(n), int(r));
        let (lda, ldb, ldc) = (int(a.stride()), int(b.stride()), int(c.stride()));
        self.threaded(threaded, || match self.zgemmt {
            Some(zgemmt) if product.lower && m == n => {
                let lower = letter(b'L');
                // SAFETY: zgemmt's arguments: C is n × n with leading dimension ldc, A is
                // n × r, op(B) is r × n, each block as large as its dimensions say (checked
                // above) and none overlapping; only C's lower triangle is written
                unsafe {
                    zgemmt(
                        &lower,
                        &no,
                        &transb,
                        &ni,
                        &ri,
                        &alpha,
                        a.pointer(),
                        &lda,
                        b.pointer(),
                        &ldb,
                        &ONE,
                        c.pointer(),
                        &ldc,
                        1,
                        1,
                        1,
                    );
                }
            }
            _ => {
                // SAFETY: zgemm's arguments: C is m × n, A is m × r, op(B) is r × n, each
                // block as large as its dimensions say (checked above) and none overlapping
                unsafe {
                    (self.zgemm)(
                        &no,
                        &transb,
                        &mi,
                        &ni,
                        &ri,
                        &alpha,
                        a.pointer(),
                        &lda,
                        b.pointer(),
                        &ldb,
                        &ONE,
                        c.pointer(),
                        &ldc,
                        1,
                        1,
                    );
                }
            }
        });
    }

    fn solve_unit_lower(&self, l: Block<'_>, b: BlockMut<'_>, threaded: bool) {
        self.triangular(l, b, [b'L', b'L', b'N', b'U'], threaded);
    }

    fn solve_upper_from_right(&self, u: Block<'_>, b: BlockMut<'_>, threaded: bool) {
        self.triangular(u, b, [b'R', b'U', b'N', b'N'], threaded);
    }

    fn solve_unit_lower_transposed_from_right(
        &self,
        l: Block<'_>,
        b: BlockMut<'_>,
        threaded: bool,
    ) {
        self.triangular(l, b, [b'R', b'L', b'T', b'U'], threaded);
    }

    fn lu(&self, mut a: BlockMut<'_>, threaded: bool) -> Vec<usize> {
        let n = a.rows();
        assert_eq!(a.columns(), n, "a square block");
        let mut rows: Vec<usize> = (0..n).collect();
        if n == 0 {
            return rows;
        }
        let (ni, lda) = (int(n), int(a.stride()));
        let mut pivots = vec![0 as c_int; n];
        let mut info: c_int = 0;
        self.threaded(threaded, || {
            // SAFETY: zgetrf's arguments: A is n × n with leading dimension lda, the pivots
            // have n entries
            unsafe { (self.zgetrf)(&ni, &ni, a.pointer(), &lda, pivots.as_mut_ptr(), &mut info) };
        });
        // (a positive info is a zero pivot, which the caller looks for itself)
        assert!(info >= 0, "zgetrf refused its argument {}", -info);
        // LAPACK's pivots are the swaps it made, one after another, from 1
        for (i, &p) in pivots.iter().enumerate() {
            rows.swap(i, (p - 1) as usize);
        }
        rows
    }
}

impl Blas {
    /// `ztrsm` with these four letters (side, uplo, trans, diag), B solved in place.
    fn triangular(&self, t: Block<'_>, mut b: BlockMut<'_>, letters: [u8; 4], threaded: bool) {
        let (m, n) = (b.rows(), b.columns());
        let order = if letters[0] == b'L' { m } else { n };
        assert_eq!(
            (t.rows(), t.columns()),
            (order, order),
            "the triangle's order"
        );
        if m == 0 || n == 0 {
            return;
        }
        let [side, uplo, trans, diag] = letters.map(letter);
        let (mi, ni, ldt, ldb) = (int(m), int(n), int(t.stride()), int(b.stride()));
        self.threaded(threaded, || {
            // SAFETY: ztrsm's arguments: the triangle is of the order of B's rows (from the
            // left) or columns (from the right), checked above; B is m × n; they don't overlap
            unsafe {
                (self.ztrsm)(
                    &side,
                    &uplo,
                    &trans,
                    &diag,
                    &mi,
                    &ni,
                    &ONE,
                    t.pointer(),
                    &ldt,
                    b.pointer(),
                    &ldb,
                    1,
                    1,
                    1,
                    1,
                );
            }
        });
    }
}

/// Every library's kernels found here, each with its name, or why not.
pub fn all() -> Vec<(&'static str, std::result::Result<Blas, String>)> {
    let mut found = vec![("openblas", Blas::openblas()), ("mkl", Blas::mkl())];
    if cfg!(target_os = "macos") {
        found.push(("accelerate", Blas::accelerate()));
    }
    found
}

/// OpenBLAS: where it was found and its version.
pub fn probe() -> Probe {
    let (discovery, library) = load(&OPENBLAS, None, |l| {
        // SAFETY: only resolved; the type is zgetrf's
        unsafe { l.function::<Zgetrf>("zgetrf_") }.map(drop)
    });
    drop(library);
    let (version, details) = match Blas::openblas() {
        Ok(blas) => (
            Some(blas.what.version.clone()),
            vec![
                match blas.threading {
                    Threading::OpenBlas(_) => "its threads set for each call".to_owned(),
                    _ => "held to one thread: it has no openblas_set_num_threads_local".to_owned(),
                },
                if blas.zgemmt.is_some() {
                    "with zgemmt".to_owned()
                } else {
                    "without zgemmt: zgemm computes both triangles".to_owned()
                },
            ],
        ),
        Err(_) => (None, Vec::new()),
    };
    Probe {
        discovery,
        version,
        details,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openblas_says_its_version_in_its_configuration() {
        assert_eq!(
            openblas_version("OpenBLAS 0.3.26 DYNAMIC_ARCH NO_AFFINITY Haswell MAX_THREADS=64"),
            "0.3.26"
        );
        assert_eq!(openblas_version("0.3.29 NO_LAPACKE"), "0.3.29");
        assert_eq!(openblas_version(""), "unknown");
        assert!(at_least("0.3.27", [0, 3, 27]) && at_least("0.3.34", [0, 3, 27]));
        assert!(at_least("0.4", [0, 3, 27]) && at_least("0.3.27.dev", [0, 3, 27]));
        assert!(!at_least("0.3.26", [0, 3, 27]) && !at_least("unknown", [0, 3, 27]));
    }

    #[test]
    fn a_library_that_isnt_here_says_why() {
        for (name, kernels) in all() {
            match kernels {
                Ok(blas) => assert_eq!(blas.kernels().name, name),
                Err(reason) => assert!(!reason.is_empty(), "{name}"),
            }
        }
        if !cfg!(target_os = "macos") {
            assert!(Blas::accelerate().is_err());
        }
    }
}
