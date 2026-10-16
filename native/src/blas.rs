//! Any BLAS and LAPACK library as the dense kernels of photonoxide's multifrontal fronts (#186):
//! [`DenseKernels`] by `zgemm`, `ztrsm` and `zgetrf`, the standard interface every vendor's
//! library has.
//!
//! - **Providers** ([`Provider`]), each found by its own rules: Intel oneMKL through `mkl_rt`
//!   ([`crate::intel`]), AMD AOCL (AOCL-BLAS, which is BLIS, and AOCL-LAPACK, which is
//!   libFLAME, or the two in one `aocl` library), Apple Accelerate (macOS), Arm Performance
//!   Libraries (Arm processors) and OpenBLAS (anywhere, and pip's `scipy-openblas32` wheel).
//! - **The interface is found, never assumed** ([`Interface`]). A library's routines are looked
//!   for under every spelling photonoxide knows: Fortran's (`zgemm_`, `zgemm`, `ZGEMM`, with a
//!   prefix such as the wheels' `scipy_`, or a 64-bit suffix: `zgemm_64_`, `zgemm_64`,
//!   `zgemm64_`), then the C interfaces (`cblas_zgemm` and `LAPACKE_zgetrf_work`, with those
//!   suffixes). For the first spelling a library has, the size of its integers is measured:
//!   `zgetrf` factorizes a 1 × 1 matrix whose dimensions are 1 read either way, and the bytes
//!   it writes in its pivot and its `info` say 32 bits (LP64) or 64 (ILP64). The name is no
//!   proof: pip's `scipy-openblas64` exports `zgemm_` taking 64-bit integers. How a Fortran
//!   function returns a complex number (by value, as gfortran does, or through a first
//!   argument, as Intel's compilers and f2c do) is measured the same way with `zdotc`, its
//!   arguments chosen so that either reading is safe; on Windows on x86-64 both are one
//!   convention. No kernel here returns a complex number; the convention is recorded for what
//!   the library is. Then photonoxide's solver with these kernels must pass the smoke test
//!   against its own before it is offered ([`crate::offer`]).
//! - **`zgemmt`,** the product into one triangle that an L D Lᵀ's updates are, isn't standard:
//!   it is used where the library has it and `zgemm` computes both triangles where it doesn't.
//!   OpenBLAS has it from 0.3.22, and its is used from 0.3.27: with 0.3.26's (Ubuntu 24.04's
//!   package) the solver crashed.
//! - **Threads,** set from rayon's thread count, which is photonoxide's: a call says whether it
//!   may thread. Fronts factorized side by side on rayon's threads must each stay on their own
//!   thread, while a front near the root has them all, so the count is set for the calling
//!   thread around each call, by each library's own control: `mkl_set_num_threads_local`, and
//!   for AOCL and Arm PL, which thread by OpenMP, `omp_set_num_threads` on the calling thread,
//!   which AMD's tuning guide names as AOCL-BLAS's control. OpenBLAS has no count per thread:
//!   its `openblas_set_num_threads_local` sets the process's, as `openblas_set_num_threads`
//!   does. With rayon's threads setting it to 1 and 20 at once, the solver ran 80 times slower
//!   and didn't repeat its bits. So OpenBLAS stays on one thread, and a threaded call sets the
//!   count around itself while no other call into OpenBLAS runs (a lock, shared by the calls
//!   on one thread). OpenBLAS's LU runs on one thread always: its threaded `zgetrf` keeps
//!   arrays sized by its build's largest thread count on the stack at each level of its
//!   recursion, and overflowed a 2 MiB thread's stack (pip's and Ubuntu's builds, on Linux).
//!   Without an OpenMP runtime, AOCL-BLAS is held to one thread by
//!   `bli_thread_set_num_threads`; Accelerate has no control photonoxide sets, and is left to
//!   itself.
//! - **Determinism** ([`Kernels::reproducibility`]): oneMKL's conditional numerical
//!   reproducibility (`mkl_cbwr_set`, set up by [`crate::intel`]); the others promise nothing,
//!   and what was measured is recorded with each. No library's kernels are declared
//!   deterministic: none promises the same bits on another number of threads, and faer's
//!   stay photonoxide's deterministic default.
//! - **Accelerate's names** are the ones it has always had (`zgemm_`), not those of its newer
//!   interface (`zgemm$NEWLAPACK`, from macOS 13.3). On macOS 26.6.2 (an M1) the newer `zgemm`
//!   crashed on a 1365 × 1024 product of 512 or 513 terms and on a 5461 × 4096 one of 2049,
//!   called as Fortran's and as `cblas_zgemm$NEWLAPACK` alike, where `zgemm_` and
//!   `cblas_zgemm` computed them; with 1364 rows it didn't, and macOS 14.8 and 15.7 didn't.

use std::cell::RefCell;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use num_complex::Complex64 as c64;
use photonoxide::backend::Threads;
use photonoxide::backend::dense::{Block, BlockMut, DenseKernels, Kernels, Product};

use crate::Probe;
use crate::discovery::{Discovery, Spec, versioned};
use crate::intel::{Mkl, mkl};
use crate::library::{Library, error, load};

// ---------------------------------------------------------------------------------------------
// The providers and where each is found

/// A library with BLAS and LAPACK that photonoxide knows how to find.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Provider {
    /// Intel oneMKL, through `mkl_rt`.
    Mkl,
    /// AMD AOCL: AOCL-BLAS (BLIS) and AOCL-LAPACK (libFLAME).
    Aocl,
    /// Apple Accelerate, part of macOS.
    Accelerate,
    /// Arm Performance Libraries.
    ArmPl,
    /// OpenBLAS.
    OpenBlas,
}

impl Provider {
    /// Every provider, in the order they are registered.
    pub const ALL: [Provider; 5] = [
        Provider::OpenBlas,
        Provider::Mkl,
        Provider::Aocl,
        Provider::ArmPl,
        Provider::Accelerate,
    ];

    /// The kernels' name: the backend is `photonoxide-` and it.
    pub fn name(self) -> &'static str {
        match self {
            Provider::Mkl => "mkl",
            Provider::Aocl => "aocl",
            Provider::Accelerate => "accelerate",
            Provider::ArmPl => "armpl",
            Provider::OpenBlas => "openblas",
        }
    }

    /// The library's name, as discovery reports it.
    pub fn library(self) -> &'static str {
        match self {
            Provider::Mkl => crate::intel::MKL.name,
            Provider::Aocl => AOCL.name,
            Provider::Accelerate => crate::accelerate::ACCELERATE.name,
            Provider::ArmPl => ARMPL.name,
            Provider::OpenBlas => OPENBLAS.name,
        }
    }

    /// Whether it can exist on this system at all: Accelerate is macOS's, Arm PL is built
    /// for Arm processors and AOCL for x86-64 under Windows and Linux. A provider that can't
    /// isn't listed: there is nothing a user could do about it.
    pub fn here(self) -> bool {
        match self {
            Provider::Accelerate => cfg!(target_os = "macos"),
            Provider::ArmPl => cfg!(target_arch = "aarch64"),
            Provider::Aocl => {
                cfg!(target_arch = "x86_64") && (cfg!(windows) || cfg!(target_os = "linux"))
            }
            Provider::Mkl | Provider::OpenBlas => true,
        }
    }

    /// The prefixes its routines' names may have.
    fn prefixes(self) -> &'static [&'static str] {
        match self {
            // pip's scipy-openblas32 and scipy-openblas64 wheels
            Provider::OpenBlas => &["", "scipy_"],
            _ => &[""],
        }
    }

    /// The spellings to try for its routines, the preferred first.
    fn forms(self) -> &'static [Form] {
        match self {
            // its classic names only (the module's documentation says why)
            Provider::Accelerate => &[Form::Under, Form::C("")],
            _ => &FORMS,
        }
    }
}

/// OpenBLAS, with its LAPACK: its own build, conda-forge's and the distributions', and pip's
/// `scipy-openblas32` wheel (whose routines are named `scipy_zgemm_`).
pub const OPENBLAS: Spec = Spec {
    name: "OpenBLAS",
    files: if cfg!(windows) {
        &[
            "libopenblas.dll",
            "openblas.dll",
            "libscipy_openblas.dll",
            "libscipy_openblas64_.dll",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "libopenblas.0.dylib",
            "libopenblas.dylib",
            "libscipy_openblas.dylib",
            "libscipy_openblas64_.dylib",
        ]
    } else {
        &[
            "libopenblas.so.0",
            "libopenblas.so",
            "libscipy_openblas.so",
            "libscipy_openblas64_.so",
        ]
    },
    variables: &["PHOTONOXIDE_OPENBLAS"],
    install: Vec::new,
    wheel: Some("scipy_openblas32"),
};

/// AMD AOCL's BLAS (AOCL-BLAS, which is BLIS), or AOCL's single library with its LAPACK too.
pub const AOCL: Spec = Spec {
    name: "AMD AOCL",
    files: if cfg!(windows) {
        &[
            "aocl.dll",
            "AOCL-LibBlis-Win-MT-dll.dll",
            "AOCL-LibBlis-Win-dll.dll",
        ]
    } else if cfg!(target_os = "linux") {
        &[
            "libaocl.so",
            "libblis-mt.so.5",
            "libblis-mt.so",
            "libblis.so.5",
            "libblis.so",
        ]
    } else {
        &[]
    },
    variables: &["PHOTONOXIDE_AOCL", "AOCL_ROOT"],
    install: aocl_folders,
    wheel: None,
};

/// AMD AOCL's LAPACK (AOCL-LAPACK, which is libFLAME).
pub const AOCL_LAPACK: Spec = Spec {
    name: "AMD AOCL-LAPACK",
    files: if cfg!(windows) {
        &["AOCL-LibFlame-Win-MT-dll.dll", "AOCL-LibFlame-Win-dll.dll"]
    } else if cfg!(target_os = "linux") {
        &["libflame.so.5", "libflame.so"]
    } else {
        &[]
    },
    variables: &["PHOTONOXIDE_AOCL_LAPACK", "AOCL_ROOT"],
    install: aocl_folders,
    wheel: None,
};

/// AMD AOCL's utilities, which AOCL-BLAS and AOCL-LAPACK load: loaded first from where AOCL
/// installs them, since Windows doesn't look there.
const AOCL_UTILS: Spec = Spec {
    name: "AMD AOCL-Utils",
    files: if cfg!(windows) {
        &["libaoclutils.dll", "aoclutils.dll"]
    } else if cfg!(target_os = "linux") {
        &["libaoclutils.so"]
    } else {
        &[]
    },
    variables: &["AOCL_ROOT"],
    install: aocl_folders,
    wheel: None,
};

/// Arm Performance Libraries: their OpenMP build with 32-bit integers first.
pub const ARMPL: Spec = Spec {
    name: "Arm Performance Libraries",
    files: if cfg!(windows) {
        &[
            "armpl_lp64_mp.dll",
            "armpl_lp64.dll",
            "armpl_ilp64_mp.dll",
            "armpl_ilp64.dll",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "libarmpl_lp64_mp.dylib",
            "libarmpl_lp64.dylib",
            "libarmpl_ilp64_mp.dylib",
            "libarmpl_ilp64.dylib",
        ]
    } else {
        &[
            "libarmpl_lp64_mp.so",
            "libarmpl_lp64.so",
            "libarmpl_ilp64_mp.so",
            "libarmpl_ilp64.so",
        ]
    },
    variables: &["PHOTONOXIDE_ARMPL", "ARMPL_DIR"],
    install: armpl_folders,
    wheel: None,
};

const FRAMEWORK: &str = "/System/Library/Frameworks/Accelerate.framework/Accelerate";

/// The OpenMP runtimes a library may thread on, by file name (Windows, where a function isn't
/// looked up through the library that loaded its runtime): LLVM's, Intel's, Arm's, GNU's,
/// Microsoft's.
#[cfg(windows)]
const OPENMP_RUNTIMES: &[&str] = &[
    "libomp.dll",
    "libiomp5md.dll",
    "omp.dll",
    "libgomp-1.dll",
    "vcomp140.dll",
];

/// AOCL's folders: below `AOCL_ROOT`, then its default install folders, those with 32-bit
/// integers before those with 64.
fn aocl_folders() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = std::env::var_os("AOCL_ROOT")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    let mut out = Vec::new();
    if cfg!(windows) {
        for var in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(base) = std::env::var_os(var) {
                roots.push(PathBuf::from(base).join("AMD").join("AOCL-Windows"));
            }
        }
        for integers in ["LP64", "ILP64"] {
            for root in &roots {
                for part in ["amd-blis", "amd-libflame"] {
                    out.push(root.join(part).join("lib").join(integers));
                }
            }
        }
        for root in &roots {
            out.push(root.join("amd-utils").join("lib"));
        }
    } else {
        // AMD's packages: /opt/AMD/aocl/aocl-linux-<compiler>-<version>/<compiler>/lib_LP64;
        // its tar files: <folder>/<version>/<compiler>/lib_LP64
        let mut installs = Vec::new();
        for parent in ["/opt/AMD/aocl", "/opt/aocl"] {
            installs.extend(versioned(Path::new(parent), "aocl-linux"));
            installs.push(PathBuf::from(parent));
        }
        if let Some(home) = std::env::var_os("HOME") {
            installs.extend(versioned(&PathBuf::from(home).join("aocl"), ""));
        }
        for install in installs {
            roots.push(install.clone());
            for compiler in ["aocc", "gcc"] {
                roots.push(install.join(compiler));
            }
        }
        for integers in ["lib_LP64", "lib", "lib_ILP64"] {
            for root in &roots {
                out.push(root.join(integers));
            }
        }
    }
    out
}

/// Arm PL's folders: each version's under its default install folder, the newest first.
fn armpl_folders() -> Vec<PathBuf> {
    let parents: Vec<PathBuf> = if cfg!(windows) {
        ["ProgramFiles", "ProgramFiles(Arm)"]
            .iter()
            .filter_map(std::env::var_os)
            .map(|base| PathBuf::from(base).join("Arm Performance Libraries"))
            .collect()
    } else {
        vec![PathBuf::from("/opt/arm")]
    };
    let mut out = Vec::new();
    for parent in parents {
        for version in versioned(&parent, "armpl") {
            out.push(version.join("lib"));
            out.push(version);
        }
        out.push(parent);
    }
    out
}

// ---------------------------------------------------------------------------------------------
// The interface: spellings, integers, complex results

/// Which of the standard interfaces a library is called through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Style {
    /// Fortran's BLAS and LAPACK: every argument by address.
    Fortran,
    /// The C interfaces: CBLAS and LAPACKE.
    C,
}

/// The size of a library's integers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Integers {
    /// 32 bits.
    Lp64,
    /// 64 bits.
    Ilp64,
}

impl fmt::Display for Integers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Integers::Lp64 => write!(f, "32-bit integers (LP64)"),
            Integers::Ilp64 => write!(f, "64-bit integers (ILP64)"),
        }
    }
}

/// How a Fortran function of the library returns a complex number.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ComplexResults {
    /// By value, as gfortran does.
    Value,
    /// Through a first argument, as Intel's compilers and f2c do.
    Argument,
    /// The two are one convention here (Windows on x86-64: a 16-byte result is returned
    /// through a hidden first argument either way).
    OneConvention,
    /// The C interface: `cblas_zdotc_sub` writes its result through an argument.
    C,
    /// Not measured, and why.
    Unknown(String),
}

impl fmt::Display for ComplexResults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComplexResults::Value => write!(f, "complex results returned by value"),
            ComplexResults::Argument => {
                write!(f, "complex results returned through a first argument")
            }
            ComplexResults::OneConvention => write!(
                f,
                "complex results through a hidden first argument, the one convention here"
            ),
            ComplexResults::C => write!(f, "complex results through an argument (CBLAS)"),
            ComplexResults::Unknown(why) => write!(f, "complex results: {why}"),
        }
    }
}

/// How a library is called, as found and measured when it was loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Interface {
    /// Fortran's or C's.
    pub style: Style,
    /// The names its routines were found by: `zgemm_`, `scipy_zgemm_`, `cblas_zgemm_64`.
    pub zgemm: String,
    /// `zgetrf`'s, from its LAPACK.
    pub zgetrf: String,
    /// The size of its integers, measured.
    pub integers: Integers,
    /// How it returns a complex number, measured.
    pub complex_results: ComplexResults,
}

impl fmt::Display for Interface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let style = match self.style {
            Style::Fortran => "Fortran's",
            Style::C => "the C interfaces",
        };
        write!(
            f,
            "{style} ({} and {}), {}, {}",
            self.zgemm, self.zgetrf, self.integers, self.complex_results
        )
    }
}

/// Which interface to take, where a library has more than one: any (Fortran's, then C's; the
/// first spelling found, whatever its integers) unless said.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Want {
    /// Only this style.
    pub style: Option<Style>,
    /// Only integers of this size.
    pub integers: Option<Integers>,
}

impl Want {
    /// Only this style and this size of integers, either if `None`.
    pub fn new(style: Option<Style>, integers: Option<Integers>) -> Want {
        Want { style, integers }
    }
}

/// A spelling of the routines' names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Form {
    /// `zgemm_`
    Under,
    /// `zgemm`
    Bare,
    /// `ZGEMM`
    Upper,
    /// `zgemm_64_`
    Under64Under,
    /// `zgemm_64`
    Under64,
    /// `zgemm64_`
    Plain64Under,
    /// `cblas_zgemm` and `LAPACKE_zgetrf_work`, with this suffix.
    C(&'static str),
}

/// Every spelling, Fortran's first and 32-bit names before 64-bit ones.
const FORMS: [Form; 9] = [
    Form::Under,
    Form::Bare,
    Form::Upper,
    Form::Under64Under,
    Form::Under64,
    Form::Plain64Under,
    Form::C(""),
    Form::C("_64"),
    Form::C("64_"),
];

impl Form {
    fn style(self) -> Style {
        match self {
            Form::C(_) => Style::C,
            _ => Style::Fortran,
        }
    }

    /// The suffix a library's other functions have beside these routines (OpenBLAS's
    /// `openblas_set_num_threads64_`).
    fn suffix(self) -> &'static str {
        match self {
            Form::Under64Under | Form::Plain64Under | Form::C("64_") => "64_",
            Form::Under64 | Form::C("_64") => "_64",
            _ => "",
        }
    }

    /// The routine's name in this spelling: `routine` is BLAS's (`zgemm`); the C interfaces
    /// have no `zdotc` of the kind measured here.
    fn name(self, prefix: &str, routine: &str) -> Option<String> {
        Some(match self {
            Form::Under => format!("{prefix}{routine}_"),
            Form::Bare => format!("{prefix}{routine}"),
            Form::Upper => format!("{prefix}{}", routine.to_ascii_uppercase()),
            Form::Under64Under => format!("{prefix}{routine}_64_"),
            Form::Under64 => format!("{prefix}{routine}_64"),
            Form::Plain64Under => format!("{prefix}{routine}64_"),
            Form::C(suffix) => match routine {
                "zgetrf" => format!("{prefix}LAPACKE_zgetrf_work{suffix}"),
                "zdotc" => return None,
                _ => format!("{prefix}cblas_{routine}{suffix}"),
            },
        })
    }
}

/// An integer as a library takes it.
trait Int: Copy + Send + Sync + 'static {
    fn of(n: usize) -> Self;
    fn get(self) -> i64;
}

impl Int for i32 {
    fn of(n: usize) -> i32 {
        i32::try_from(n).expect("a front's dimension is below 2³¹")
    }
    fn get(self) -> i64 {
        self.into()
    }
}

impl Int for i64 {
    fn of(n: usize) -> i64 {
        i64::try_from(n).expect("a dimension below 2⁶³")
    }
    fn get(self) -> i64 {
        self
    }
}

type Char = *const c_char;
/// `zgemm(transa, transb, m, n, k, alpha, a, lda, b, ldb, beta, c, ldc)` and two lengths.
type FZgemm<I> = unsafe extern "C" fn(
    Char,
    Char,
    *const I,
    *const I,
    *const I,
    *const c64,
    *const c64,
    *const I,
    *const c64,
    *const I,
    *const c64,
    *mut c64,
    *const I,
    usize,
    usize,
);
/// `zgemmt(uplo, transa, transb, n, k, alpha, a, lda, b, ldb, beta, c, ldc)` and three lengths.
type FZgemmt<I> = unsafe extern "C" fn(
    Char,
    Char,
    Char,
    *const I,
    *const I,
    *const c64,
    *const c64,
    *const I,
    *const c64,
    *const I,
    *const c64,
    *mut c64,
    *const I,
    usize,
    usize,
    usize,
);
/// `ztrsm(side, uplo, transa, diag, m, n, alpha, a, lda, b, ldb)` and four lengths.
type FZtrsm<I> = unsafe extern "C" fn(
    Char,
    Char,
    Char,
    Char,
    *const I,
    *const I,
    *const c64,
    *const c64,
    *const I,
    *mut c64,
    *const I,
    usize,
    usize,
    usize,
    usize,
);
/// `zgetrf(m, n, a, lda, ipiv, info)`.
type FZgetrf<I> = unsafe extern "C" fn(*const I, *const I, *mut c64, *const I, *mut I, *mut I);
/// `cblas_zgemm(layout, transa, transb, m, n, k, alpha, a, lda, b, ldb, beta, c, ldc)`.
type CZgemm<I> = unsafe extern "C" fn(
    c_int,
    c_int,
    c_int,
    I,
    I,
    I,
    *const c64,
    *const c64,
    I,
    *const c64,
    I,
    *const c64,
    *mut c64,
    I,
);
/// `cblas_zgemmt(layout, uplo, transa, transb, n, k, alpha, a, lda, b, ldb, beta, c, ldc)`.
type CZgemmt<I> = unsafe extern "C" fn(
    c_int,
    c_int,
    c_int,
    c_int,
    I,
    I,
    *const c64,
    *const c64,
    I,
    *const c64,
    I,
    *const c64,
    *mut c64,
    I,
);
/// `cblas_ztrsm(layout, side, uplo, transa, diag, m, n, alpha, a, lda, b, ldb)`.
type CZtrsm<I> = unsafe extern "C" fn(
    c_int,
    c_int,
    c_int,
    c_int,
    c_int,
    I,
    I,
    *const c64,
    *const c64,
    I,
    *mut c64,
    I,
);
/// `LAPACKE_zgetrf_work(layout, m, n, a, lda, ipiv)`, returning `info`.
type CZgetrf<I> = unsafe extern "C" fn(c_int, I, I, *mut c64, I, *mut I) -> I;

/// The 16 bytes each probe argument is: an integer read as 1 at either size, or a complex
/// number (its real part the smallest subnormal, its imaginary part 2).
type Cell = [u64; 2];
const UNTOUCHED: u64 = 0xAAAA_AAAA_AAAA_AAAA;
/// zgetrf of a 1 × 1 matrix, every integer given as a [`Cell`].
type ProbeFZgetrf =
    unsafe extern "C" fn(*const Cell, *const Cell, *mut c64, *const Cell, *mut Cell, *mut Cell);
/// LAPACKE_zgetrf_work with its dimensions passed as 64-bit values: a library of 32-bit ones
/// reads their lower halves, which are the same numbers.
type ProbeCZgetrf = unsafe extern "C" fn(c_int, i64, i64, *mut c64, i64, *mut Cell) -> c_int;

/// CBLAS's and LAPACKE's constants.
const COLUMN_MAJOR: c_int = 102;

/// How many bytes an integer the library wrote took: 4 or 8, from what it left of a [`Cell`]
/// filled with [`UNTOUCHED`].
fn bytes_written(cell: &Cell, value: i64) -> Option<usize> {
    let bytes: Vec<u8> = cell.iter().flat_map(|w| w.to_ne_bytes()).collect();
    let untouched = |range: std::ops::Range<usize>| bytes[range].iter().all(|&b| b == 0xAA);
    let four = i32::try_from(value).ok()?.to_ne_bytes();
    if bytes[..4] == four && untouched(4..16) {
        return Some(4);
    }
    (bytes[..8] == value.to_ne_bytes() && untouched(8..16)).then_some(8)
}

/// The size of the integers of a `zgetrf` resolved as `F`: 4 or 8 bytes, or why not known.
///
/// # Safety
///
/// `zgetrf` is LAPACK's `zgetrf` in Fortran's form, with integers of either size.
unsafe fn measure_fortran(zgetrf: ProbeFZgetrf) -> Result<usize, String> {
    let one: Cell = [1, 0];
    let mut a = c64::new(2.0, 0.0);
    let (mut pivot, mut info) = ([UNTOUCHED; 2], [UNTOUCHED; 2]);
    // SAFETY: m, n and lda point to 16 bytes that read 1 as a 32-bit or a 64-bit integer; A
    // is the one entry of a 1 × 1 matrix; the pivot and info have room for either size. A
    // factorization of 1 × 1 writes one pivot and info, nothing else
    unsafe { zgetrf(&one, &one, &mut a, &one, &mut pivot, &mut info) };
    match (bytes_written(&info, 0), bytes_written(&pivot, 1)) {
        (Some(i), Some(p)) if i == p && a == c64::new(2.0, 0.0) => Ok(i),
        _ => Err(format!(
            "its zgetrf of a 1 × 1 matrix left info {info:x?} and pivot {pivot:x?}, which are \
             neither 32-bit nor 64-bit integers"
        )),
    }
}

/// As [`measure_fortran`], through LAPACKE.
///
/// # Safety
///
/// `zgetrf` is LAPACKE's `LAPACKE_zgetrf_work`, with integers of either size.
unsafe fn measure_c(zgetrf: ProbeCZgetrf) -> Result<usize, String> {
    let mut a = c64::new(2.0, 0.0);
    let mut pivot = [UNTOUCHED; 2];
    // SAFETY: the dimensions are 1 at either size (passed in 64-bit registers or stack slots,
    // whose lower halves are a 32-bit reading); A is one entry; the pivot has room for either
    // size
    let info = unsafe { zgetrf(COLUMN_MAJOR, 1, 1, &mut a, 1, &mut pivot) };
    match bytes_written(&pivot, 1) {
        Some(p) if info == 0 && a == c64::new(2.0, 0.0) => Ok(p),
        _ => Err(format!(
            "its LAPACKE_zgetrf_work of a 1 × 1 matrix returned {info} and left the pivot \
             {pivot:x?}, which is neither a 32-bit nor a 64-bit integer"
        )),
    }
}

/// How a Fortran `zdotc` returns its complex result.
///
/// # Safety
///
/// `zdotc` is BLAS's `zdotc` in Fortran's form, with integers of either size and either
/// convention for its result.
unsafe fn complex_results(library: &Library, name: &str) -> ComplexResults {
    if cfg!(all(windows, target_arch = "x86_64")) {
        return ComplexResults::OneConvention;
    }
    /// Six cells: through a first argument, (result, n, x, incx, y, incy); by value,
    /// (n, x, incx, y, incy) and the result in registers.
    type Zdotc = unsafe extern "C" fn(
        *mut Cell,
        *mut Cell,
        *mut Cell,
        *mut Cell,
        *mut Cell,
        *mut Cell,
    ) -> c64;
    // SAFETY: only resolved, under the caller's promise of what it is
    let Ok(zdotc) = (unsafe { library.function::<Zdotc>(name) }) else {
        return ComplexResults::Unknown(format!("no {name} to tell by"));
    };
    // each cell reads 1 as an integer of either size, and as a complex number d + 2i, d the
    // smallest subnormal: conj(x) y = |d|² + 4 = 4 whichever cells are x and y
    let cell = || -> Cell { [1, 2.0f64.to_bits()] };
    let mut c = [cell(), cell(), cell(), cell(), cell(), cell()];
    let [c0, c1, c2, c3, c4, c5] = &mut c;
    // SAFETY: either reading takes its n, incx and incy (1) and x and y (one entry each) from
    // these cells, and a result through an argument goes to the first, 16 bytes; a result by
    // value comes back in registers, the declared return
    let value = unsafe { zdotc(c0, c1, c2, c3, c4, c5) };
    let four = [4.0f64.to_bits(), 0.0f64.to_bits()];
    if c[0] == four {
        ComplexResults::Argument
    } else if value == c64::new(4.0, 0.0) && c[0] == cell() {
        ComplexResults::Value
    } else {
        ComplexResults::Unknown(format!(
            "{name} returned neither 4 by value nor through its first argument"
        ))
    }
}

/// The calls a library's routines are made with.
trait Calls: Send + Sync {
    /// Whether it has a product into one triangle (`zgemmt`).
    fn triangular_product(&self) -> bool;

    /// C (m × n) ← C + α A op(B), A m × k; only C's lower triangle if `lower` and the library
    /// has [`Calls::triangular_product`].
    ///
    /// # Safety
    ///
    /// Each block is as large as its dimensions and leading dimension say, and none overlaps
    /// another.
    unsafe fn gemm(&self, g: &Gemm);

    /// `ztrsm` with these letters (side, uplo, trans, diag), B (m × n) solved in place.
    ///
    /// # Safety
    ///
    /// The triangle is of the order of B's rows (from the left) or columns (from the right);
    /// each block is as large as its dimensions say; they don't overlap.
    unsafe fn trsm(&self, t: &Trsm);

    /// `zgetrf` of the n × n matrix at `a`: LAPACK's pivots, from 1, and `info`.
    ///
    /// # Safety
    ///
    /// `a` is n × n with leading dimension `lda`.
    unsafe fn getrf(&self, n: usize, a: *mut c64, lda: usize) -> (Vec<i64>, i64);
}

/// A product's arguments.
struct Gemm {
    transposed: bool,
    lower: bool,
    m: usize,
    n: usize,
    k: usize,
    alpha: c64,
    a: *const c64,
    lda: usize,
    b: *const c64,
    ldb: usize,
    c: *mut c64,
    ldc: usize,
}

/// A triangular solve's arguments.
struct Trsm {
    letters: [u8; 4],
    m: usize,
    n: usize,
    t: *const c64,
    ldt: usize,
    b: *mut c64,
    ldb: usize,
}

const ONE: c64 = c64::new(1.0, 0.0);

fn letter(c: u8) -> c_char {
    c as c_char
}

/// Fortran's routines with integers `I`.
struct Fortran<I> {
    zgemm: FZgemm<I>,
    zgemmt: Option<FZgemmt<I>>,
    ztrsm: FZtrsm<I>,
    zgetrf: FZgetrf<I>,
}

impl<I: Int> Calls for Fortran<I> {
    fn triangular_product(&self) -> bool {
        self.zgemmt.is_some()
    }

    unsafe fn gemm(&self, g: &Gemm) {
        let (no, transb) = (letter(b'N'), letter(if g.transposed { b'T' } else { b'N' }));
        let [m, n, k, lda, ldb, ldc] = [g.m, g.n, g.k, g.lda, g.ldb, g.ldc].map(I::of);
        match self.zgemmt {
            Some(zgemmt) if g.lower && g.m == g.n => {
                let lower = letter(b'L');
                // SAFETY: zgemmt's arguments, by the caller's promise; only C's lower triangle
                // is written; each character's length is 1
                unsafe {
                    zgemmt(
                        &lower, &no, &transb, &n, &k, &g.alpha, g.a, &lda, g.b, &ldb, &ONE, g.c,
                        &ldc, 1, 1, 1,
                    );
                }
            }
            // SAFETY: zgemm's arguments, by the caller's promise
            _ => unsafe {
                (self.zgemm)(
                    &no, &transb, &m, &n, &k, &g.alpha, g.a, &lda, g.b, &ldb, &ONE, g.c, &ldc, 1, 1,
                );
            },
        }
    }

    unsafe fn trsm(&self, t: &Trsm) {
        let [side, uplo, trans, diag] = t.letters.map(letter);
        let [m, n, ldt, ldb] = [t.m, t.n, t.ldt, t.ldb].map(I::of);
        // SAFETY: ztrsm's arguments, by the caller's promise
        unsafe {
            (self.ztrsm)(
                &side, &uplo, &trans, &diag, &m, &n, &ONE, t.t, &ldt, t.b, &ldb, 1, 1, 1, 1,
            );
        }
    }

    unsafe fn getrf(&self, n: usize, a: *mut c64, lda: usize) -> (Vec<i64>, i64) {
        let (ni, ldai) = (I::of(n), I::of(lda));
        let mut pivots = vec![I::of(0); n];
        let mut info = I::of(0);
        // SAFETY: zgetrf's arguments: A by the caller's promise, n pivots
        unsafe { (self.zgetrf)(&ni, &ni, a, &ldai, pivots.as_mut_ptr(), &mut info) };
        (pivots.into_iter().map(I::get).collect(), info.get())
    }
}

/// The C interfaces' routines with integers `I`.
struct Cblas<I> {
    zgemm: CZgemm<I>,
    zgemmt: Option<CZgemmt<I>>,
    ztrsm: CZtrsm<I>,
    zgetrf: CZgetrf<I>,
}

/// CBLAS's letters as its enumerations.
fn cblas(letter: u8) -> c_int {
    match letter {
        b'N' => 111,
        b'T' => 112,
        b'U' => 121,
        // L is lower as uplo, left as side: the caller passes those by position
        b'L' => 122,
        b'R' => 142,
        _ => unreachable!("a letter photonoxide passes"),
    }
}

impl<I: Int> Calls for Cblas<I> {
    fn triangular_product(&self) -> bool {
        self.zgemmt.is_some()
    }

    unsafe fn gemm(&self, g: &Gemm) {
        let (no, transb) = (cblas(b'N'), cblas(if g.transposed { b'T' } else { b'N' }));
        let [m, n, k, lda, ldb, ldc] = [g.m, g.n, g.k, g.lda, g.ldb, g.ldc].map(I::of);
        match self.zgemmt {
            Some(zgemmt) if g.lower && g.m == g.n => {
                // SAFETY: cblas_zgemmt's arguments, by the caller's promise
                unsafe {
                    zgemmt(
                        COLUMN_MAJOR,
                        cblas(b'L'),
                        no,
                        transb,
                        n,
                        k,
                        &g.alpha,
                        g.a,
                        lda,
                        g.b,
                        ldb,
                        &ONE,
                        g.c,
                        ldc,
                    );
                }
            }
            // SAFETY: cblas_zgemm's arguments, by the caller's promise
            _ => unsafe {
                (self.zgemm)(
                    COLUMN_MAJOR,
                    no,
                    transb,
                    m,
                    n,
                    k,
                    &g.alpha,
                    g.a,
                    lda,
                    g.b,
                    ldb,
                    &ONE,
                    g.c,
                    ldc,
                );
            },
        }
    }

    unsafe fn trsm(&self, t: &Trsm) {
        let [side, uplo, trans, diag] = t.letters;
        let side = if side == b'L' { 141 } else { 142 };
        let uplo = if uplo == b'L' { 122 } else { 121 };
        let trans = cblas(trans);
        let diag = if diag == b'U' { 132 } else { 131 };
        let [m, n, ldt, ldb] = [t.m, t.n, t.ldt, t.ldb].map(I::of);
        // SAFETY: cblas_ztrsm's arguments, by the caller's promise
        unsafe {
            (self.ztrsm)(
                COLUMN_MAJOR,
                side,
                uplo,
                trans,
                diag,
                m,
                n,
                &ONE,
                t.t,
                ldt,
                t.b,
                ldb,
            );
        }
    }

    unsafe fn getrf(&self, n: usize, a: *mut c64, lda: usize) -> (Vec<i64>, i64) {
        let mut pivots = vec![I::of(0); n];
        // SAFETY: LAPACKE_zgetrf_work's arguments: A by the caller's promise, n pivots; the
        // column-major layout needs no copy
        let info = unsafe {
            (self.zgetrf)(
                COLUMN_MAJOR,
                I::of(n),
                I::of(n),
                a,
                I::of(lda),
                pivots.as_mut_ptr(),
            )
        };
        (pivots.into_iter().map(I::get).collect(), info.get())
    }
}

/// What [`detect`] found.
struct Detected {
    interface: Interface,
    calls: Box<dyn Calls>,
    prefix: &'static str,
    form: Form,
}

/// The first function of `names` the library has.
///
/// # Safety
///
/// As [`Library::function`]: `F` is each name's C signature.
unsafe fn first<F: Copy>(library: &Library, names: &[String]) -> Option<F> {
    names.iter().find_map(|name| {
        // SAFETY: the caller's
        unsafe { library.function::<F>(name) }.ok()
    })
}

/// The routines of `blas` and `lapack` (one library or two) by the first spelling of
/// `forms` they both have, with each prefix of `prefixes`, its integers measured.
///
/// # Safety
///
/// A function the libraries export under one of BLAS's or LAPACK's names in a spelling
/// photonoxide knows is that routine, in that spelling's interface (Fortran's or C's).
unsafe fn detect(
    blas: &Library,
    lapack: &Library,
    prefixes: &[&'static str],
    forms: &[Form],
    want: Want,
) -> Result<Detected, String> {
    let mut why = Vec::new();
    let mut pinned = false;
    for &form in forms {
        if want.style.is_some_and(|s| s != form.style()) {
            continue;
        }
        for &prefix in prefixes {
            let name = |r: &str| form.name(prefix, r).expect("a routine of every spelling");
            let has = |l: &Library, r: &str| {
                // SAFETY: only resolved; the type is any function's
                unsafe { l.function::<unsafe extern "C" fn()>(&name(r)) }.is_ok()
            };
            if !(has(blas, "zgemm") && has(blas, "ztrsm") && has(lapack, "zgetrf")) {
                continue;
            }
            // a library called once may have started threads that outlive the call: it stays
            // loaded, whether it is used or not
            if !pinned {
                blas.pin();
                lapack.pin();
                pinned = true;
            }
            // SAFETY: the caller's promise: the routine of that name is zgetrf, of the
            // spelling's interface, with integers of either size (which is what is measured)
            let bytes = unsafe {
                match form.style() {
                    Style::Fortran => match lapack.function::<ProbeFZgetrf>(&name("zgetrf")) {
                        Ok(f) => measure_fortran(f),
                        Err(e) => Err(e.to_string()),
                    },
                    Style::C => match lapack.function::<ProbeCZgetrf>(&name("zgetrf")) {
                        Ok(f) => measure_c(f),
                        Err(e) => Err(e.to_string()),
                    },
                }
            };
            let integers = match bytes {
                Ok(4) => Integers::Lp64,
                Ok(_) => Integers::Ilp64,
                Err(e) => {
                    why.push(format!("{}: {e}", name("zgetrf")));
                    continue;
                }
            };
            if want.integers.is_some_and(|w| w != integers) {
                why.push(format!("{} takes {integers}", name("zgemm")));
                continue;
            }
            // SAFETY: the caller's promise, and the integers measured above
            let calls = unsafe { routines(blas, lapack, form, &name, integers) }
                .map_err(|e| e.to_string())?;
            let complex_results = match form.style() {
                Style::C => ComplexResults::C,
                // SAFETY: the caller's promise: zdotc in Fortran's form
                Style::Fortran => unsafe { complex_results(blas, &name("zdotc")) },
            };
            return Ok(Detected {
                interface: Interface {
                    style: form.style(),
                    zgemm: name("zgemm"),
                    zgetrf: name("zgetrf"),
                    integers,
                    complex_results,
                },
                calls,
                prefix,
                form,
            });
        }
    }
    Err(if why.is_empty() {
        "it has no zgemm, ztrsm and zgetrf by any name photonoxide knows".into()
    } else {
        why.join("; ")
    })
}

/// The routines by `name`, typed for `integers`.
///
/// # Safety
///
/// As [`detect`]'s, with integers of that size.
unsafe fn routines(
    blas: &Library,
    lapack: &Library,
    form: Form,
    name: &dyn Fn(&str) -> String,
    integers: Integers,
) -> photonoxide::Result<Box<dyn Calls>> {
    /// # Safety
    ///
    /// As [`routines`]'.
    unsafe fn fortran<I: Int>(
        blas: &Library,
        lapack: &Library,
        name: &dyn Fn(&str) -> String,
    ) -> photonoxide::Result<Box<dyn Calls>> {
        // SAFETY: the caller's: each type is the routine's Fortran interface with integers I
        unsafe {
            Ok(Box::new(Fortran::<I> {
                zgemm: blas.function(&name("zgemm"))?,
                zgemmt: blas.function(&name("zgemmt")).ok(),
                ztrsm: blas.function(&name("ztrsm"))?,
                zgetrf: lapack.function(&name("zgetrf"))?,
            }))
        }
    }
    /// # Safety
    ///
    /// As [`routines`]'.
    unsafe fn c<I: Int>(
        blas: &Library,
        lapack: &Library,
        name: &dyn Fn(&str) -> String,
    ) -> photonoxide::Result<Box<dyn Calls>> {
        // SAFETY: the caller's: each type is the routine's C interface with integers I
        unsafe {
            Ok(Box::new(Cblas::<I> {
                zgemm: blas.function(&name("zgemm"))?,
                zgemmt: blas.function(&name("zgemmt")).ok(),
                ztrsm: blas.function(&name("ztrsm"))?,
                zgetrf: lapack.function(&name("zgetrf"))?,
            }))
        }
    }
    // SAFETY: the caller's
    unsafe {
        match (form.style(), integers) {
            (Style::Fortran, Integers::Lp64) => fortran::<i32>(blas, lapack, name),
            (Style::Fortran, Integers::Ilp64) => fortran::<i64>(blas, lapack, name),
            (Style::C, Integers::Lp64) => c::<i32>(blas, lapack, name),
            (Style::C, Integers::Ilp64) => c::<i64>(blas, lapack, name),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Threads

type SetInt = unsafe extern "C" fn(c_int) -> c_int;
type SetIntOnly = unsafe extern "C" fn(c_int);
type GetInt = unsafe extern "C" fn() -> c_int;

/// The calls into OpenBLAS: shared by the calls on one thread, OpenBLAS's count being one
/// between them; held alone by a threaded call, which sets the process's count around itself.
/// The fronts factorized side by side on rayon's threads then never see another's count. (Every
/// OpenBLAS loaded shares it; it is only a lock.)
static OPENBLAS_CALLS: std::sync::RwLock<()> = std::sync::RwLock::new(());

/// How a call's threads are set.
enum Threading {
    /// `mkl_set_num_threads_local`.
    Mkl(Arc<Mkl>),
    /// `openblas_set_num_threads`, the process's count (OpenBLAS's
    /// `openblas_set_num_threads_local` sets the same, despite its name), around a threaded
    /// call made while no other runs ([`OPENBLAS_CALLS`]); one thread otherwise.
    OpenBlas(SetIntOnly),
    /// The OpenMP runtime's `omp_set_num_threads` on the calling thread, and
    /// `omp_get_max_threads` to restore.
    OpenMp(SetIntOnly, GetInt),
    /// Not set per call: the library was told one thread, or decides itself.
    Fixed,
}

/// The OpenMP runtime `library` threads on: its `omp_set_num_threads` and
/// `omp_get_max_threads`, and the runtime to keep loaded (Windows).
fn openmp(library: &Library) -> Option<(SetIntOnly, GetInt, Option<Library>)> {
    let functions = |l: &Library| -> Option<(SetIntOnly, GetInt)> {
        // SAFETY: OpenMP's `void omp_set_num_threads(int)` and `int omp_get_max_threads(void)`
        unsafe {
            Some((
                l.function("omp_set_num_threads").ok()?,
                l.function("omp_get_max_threads").ok()?,
            ))
        }
    };
    // where a function is looked up through the libraries a library loaded
    #[cfg(not(windows))]
    {
        functions(library).map(|(set, get)| (set, get, None))
    }
    #[cfg(windows)]
    {
        let _ = library;
        OPENMP_RUNTIMES.iter().find_map(|name| {
            let runtime = Library::loaded(name).ok()?;
            let (set, get) = functions(&runtime)?;
            Some((set, get, Some(runtime)))
        })
    }
}

// ---------------------------------------------------------------------------------------------
// The kernels

/// A BLAS and LAPACK library's kernels.
pub struct Blas {
    what: Kernels,
    interface: Interface,
    calls: Box<dyn Calls>,
    triangular: bool,
    /// Whether its LU may thread. OpenBLAS's threaded `zgetrf` (`getrf_parallel`) keeps
    /// arrays sized by its build's largest thread count on the stack at each level of its
    /// recursion: on a thread with rayon's or a test's stack (2 MiB) it overflowed, with pip's
    /// `scipy-openblas32` and Ubuntu's builds on Linux. Its products and solves still thread.
    threaded_lu: bool,
    threading: Threading,
    /// The files, for what the probe says.
    files: Vec<PathBuf>,
    // last: the functions above must not outlive them (oneMKL's is kept by `threading`)
    _libraries: Vec<Library>,
}

fn reason(e: photonoxide::Error) -> String {
    match e {
        photonoxide::Error::InvalidValue { reason, .. } => reason,
        other => other.to_string(),
    }
}

/// A library loaded by `spec`'s discovery whose routines `detect` finds as `want` asks, and
/// what was found.
fn find(
    spec: &Spec,
    setting: Option<&Path>,
    provider: Provider,
    want: Want,
) -> (Discovery, Result<(Library, Detected), String>) {
    let detected = RefCell::new(None);
    let (discovery, library) = load(spec, setting, |l| {
        // SAFETY: a BLAS and LAPACK library's routines by their standard names
        let found =
            unsafe { detect(l, l, provider.prefixes(), provider.forms(), want) }.map_err(error)?;
        *detected.borrow_mut() = Some(found);
        Ok(())
    });
    let found = match library {
        Some(library) => Ok((
            library,
            detected
                .into_inner()
                .expect("the library loaded passed its check"),
        )),
        None => Err(discovery.reason()),
    };
    (discovery, found)
}

impl Blas {
    /// The provider's kernels, found and loaded, by the first interface it has.
    ///
    /// # Errors
    ///
    /// Why not: not found, not loadable, without the routines, or another system's.
    pub fn load(provider: Provider) -> Result<Blas, String> {
        Blas::find(provider, None, Want::default())
    }

    /// The provider's kernels from the file or folder `setting` if given (else wherever it is
    /// found), by the interface `want` asks for.
    ///
    /// # Errors
    ///
    /// As [`Blas::load`], and when the library has no such interface.
    pub fn find(provider: Provider, setting: Option<&Path>, want: Want) -> Result<Blas, String> {
        Blas::found(provider, setting, want).1
    }

    /// As [`Blas::find`], with where it was looked for (none for oneMKL, whose discovery is
    /// [`crate::intel`]'s, and Accelerate, which is where macOS has it).
    fn found(
        provider: Provider,
        setting: Option<&Path>,
        want: Want,
    ) -> (Option<Discovery>, Result<Blas, String>) {
        match provider {
            Provider::OpenBlas => {
                let (d, found) = find(&OPENBLAS, setting, provider, want);
                (Some(d), found.map(|(l, f)| openblas(l, f)))
            }
            Provider::ArmPl => {
                let (d, found) = find(&ARMPL, setting, provider, want);
                (Some(d), found.map(|(l, f)| armpl(l, f)))
            }
            Provider::Aocl => {
                let (d, found) = aocl(setting, want);
                (Some(d), found)
            }
            Provider::Mkl => (None, onemkl(want)),
            Provider::Accelerate => (None, accelerate(want)),
        }
    }

    /// OpenBLAS found and loaded.
    ///
    /// # Errors
    ///
    /// As [`Blas::load`].
    pub fn openblas() -> Result<Blas, String> {
        Blas::load(Provider::OpenBlas)
    }

    /// oneMKL's BLAS and LAPACK, of the `mkl_rt` [`crate::intel::mkl`] set up.
    ///
    /// # Errors
    ///
    /// As [`Blas::load`].
    pub fn mkl() -> Result<Blas, String> {
        Blas::load(Provider::Mkl)
    }

    /// Accelerate's BLAS and LAPACK, on macOS.
    ///
    /// # Errors
    ///
    /// As [`Blas::load`].
    pub fn accelerate() -> Result<Blas, String> {
        Blas::load(Provider::Accelerate)
    }

    /// AMD AOCL's BLAS (BLIS) and LAPACK (libFLAME).
    ///
    /// # Errors
    ///
    /// As [`Blas::load`].
    pub fn aocl() -> Result<Blas, String> {
        Blas::load(Provider::Aocl)
    }

    /// Arm Performance Libraries' BLAS and LAPACK.
    ///
    /// # Errors
    ///
    /// As [`Blas::load`].
    pub fn armpl() -> Result<Blas, String> {
        Blas::load(Provider::ArmPl)
    }

    /// How it is called, as found when it was loaded.
    pub fn interface(&self) -> &Interface {
        &self.interface
    }

    /// Whether products into one triangle are the library's own (`zgemmt`).
    pub fn has_triangular_product(&self) -> bool {
        self.triangular
    }

    /// Whether a threaded call's threads are set to rayon's for it.
    pub fn threads_per_call(&self) -> bool {
        !matches!(self.threading, Threading::Fixed)
    }

    /// The files it was loaded from.
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// What is known of it, in words, one thing a line: its interface, its threads, its
    /// triangular product and its reproducibility.
    pub fn details(&self) -> Vec<String> {
        let mut out = vec![
            format!("its dense kernels: {}", self.interface),
            match &self.what.threads {
                Threads::Library(how) => format!("threads: {how}"),
                Threads::One => "held to one thread: it has no control photonoxide can set \
                                 for each call"
                    .into(),
                Threads::Rayon => "threads: rayon's".into(),
                other => format!("threads: {other:?}"),
            },
            if self.triangular {
                "with zgemmt".into()
            } else {
                "without zgemmt (or not used): zgemm computes both triangles".into()
            },
        ];
        if !self.what.reproducibility.is_empty() {
            out.push(format!("reproducibility: {}", self.what.reproducibility));
        }
        out
    }

    /// Runs `call` with the library's threads set for it on this thread.
    fn threaded<R>(&self, threaded: bool, call: impl FnOnce() -> R) -> R {
        let threads = if threaded {
            rayon::current_num_threads().max(1)
        } else {
            1
        };
        let count = c_int::try_from(threads).unwrap_or(c_int::MAX);
        match &self.threading {
            Threading::Mkl(mkl) => {
                let before = mkl.set_threads_here(threads);
                let result = call();
                mkl.set_threads_here(usize::try_from(before).unwrap_or(0));
                result
            }
            Threading::OpenBlas(set) if threads > 1 => {
                // the count is the process's: no other call into OpenBLAS runs meanwhile
                let _alone = OPENBLAS_CALLS
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                // SAFETY: `void openblas_set_num_threads(int)`: a positive count, and back to
                // one, which every other call takes
                unsafe { set(count) };
                let result = call();
                // SAFETY: as above
                unsafe { set(1) };
                result
            }
            Threading::OpenBlas(_) => {
                let _shared = OPENBLAS_CALLS
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                call()
            }
            Threading::OpenMp(set, get) => {
                // SAFETY: OpenMP's `int omp_get_max_threads(void)` and
                // `void omp_set_num_threads(int)`, which sets this thread's count of the
                // threads its next parallel regions take: any positive count
                let before = unsafe { get() };
                // SAFETY: as above
                unsafe { set(count) };
                let result = call();
                // SAFETY: as above
                unsafe { set(before.max(1)) };
                result
            }
            Threading::Fixed => call(),
        }
    }

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
        let call = Trsm {
            letters,
            m,
            n,
            t: t.pointer(),
            ldt: t.stride(),
            b: b.pointer(),
            ldb: b.stride(),
        };
        // SAFETY: the triangle is of the order of B's rows (from the left) or columns (from
        // the right), checked above; each block is as large as its dimensions and stride say
        // (a Block's promise); a block borrowed mutably overlaps no other
        self.threaded(threaded, || unsafe { self.calls.trsm(&call) });
    }
}

impl fmt::Debug for Blas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Blas")
            .field("kernels", &self.what)
            .field("interface", &self.interface)
            .finish_non_exhaustive()
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

/// A C string a library returns, or empty.
///
/// # Safety
///
/// `f` returns null or a C string that lives as long as the library.
unsafe fn text(f: unsafe extern "C" fn() -> *const c_char) -> String {
    // SAFETY: the caller's
    let p = unsafe { f() };
    if p.is_null() {
        String::new()
    } else {
        // SAFETY: the caller's: a C string
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

fn kernels(
    provider: Provider,
    version: String,
    licence: &str,
    threads: Threads,
    interface: &Interface,
    reproducibility: String,
) -> Kernels {
    let mut what = Kernels::new(provider.name(), version, licence);
    what.threads = threads;
    what.interface = interface.to_string();
    what.reproducibility = reproducibility;
    what
}

fn openblas(library: Library, found: Detected) -> Blas {
    let (p, s) = (found.prefix, found.form.suffix());
    let names = |base: &str| {
        vec![
            format!("{p}{base}{s}"),
            format!("{p}{base}"),
            base.to_owned(),
        ]
    };
    // SAFETY: `char *openblas_get_config(void)`, a static string
    let config = unsafe {
        first::<unsafe extern "C" fn() -> *const c_char>(&library, &names("openblas_get_config"))
    }
    // SAFETY: as above
    .map(|f| unsafe { text(f) })
    .unwrap_or_default();
    let version = openblas_version(&config);
    let triangular = found.calls.triangular_product() && at_least(&version, [0, 3, 27]);
    // SAFETY: `void openblas_set_num_threads(int)`, the process's count of OpenBLAS's threads
    let global = unsafe { first::<SetIntOnly>(&library, &names("openblas_set_num_threads")) };
    let (threading, threads) = match global {
        Some(set) => {
            // one thread but for a threaded call, which sets it around itself
            // SAFETY: as above: any positive count
            unsafe { set(1) };
            (
                Threading::OpenBlas(set),
                Threads::Library(
                    "OpenBLAS's own, on rayon's thread count (RAYON_NUM_THREADS) by \
                     openblas_set_num_threads, the process's count, around each threaded call, \
                     which runs while no other call into OpenBLAS does"
                        .into(),
                ),
            )
        }
        None => (Threading::Fixed, Threads::One),
    };
    let what = kernels(
        Provider::OpenBlas,
        version,
        "BSD-3-Clause (installed by the user)",
        threads,
        &found.interface,
        "OpenBLAS promises no reproducibility. Measured on an Intel Core Ultra 7 265K \
         (OpenBLAS 0.3.34, conda-forge's): the same bits on every run on a given number of \
         threads, not on another number (docs/baselines.md)"
            .into(),
    );
    Blas {
        what,
        interface: found.interface,
        calls: found.calls,
        triangular,
        threaded_lu: false,
        threading,
        files: vec![library.path().to_path_buf()],
        _libraries: vec![library],
    }
}

fn onemkl(want: Want) -> Result<Blas, String> {
    let (_, found) = mkl();
    let mkl = found?;
    let library = mkl.library();
    // SAFETY: `int MKL_Set_Interface_Layer(int code)`: asking for LP64 (0), which it returns
    // unless another layer was already in use (MKL_INTERFACE_LAYER, or an earlier call); the
    // integers of the names found are measured in any case
    unsafe { library.function::<SetInt>("MKL_Set_Interface_Layer") }
        .map(|set| {
            // SAFETY: as above
            unsafe { set(0) }
        })
        .map_err(reason)?;
    // SAFETY: mkl_rt's BLAS and LAPACK by their standard names
    let found = unsafe {
        detect(
            library,
            library,
            Provider::Mkl.prefixes(),
            Provider::Mkl.forms(),
            want,
        )
    }?;
    let reproducibility = if mkl.reproducibility == 0 {
        "oneMKL's conditional numerical reproducibility is off (MKL_CBWR): nothing is promised"
            .to_owned()
    } else {
        format!(
            "oneMKL's conditional numerical reproducibility (mkl_cbwr_get: {}): the same bits \
             on every run on this processor and number of threads, but not on another number \
             (its strict mode covers zgemm and ztrsm, not zgetrf); measured so on an Intel Core \
             Ultra 7 265K (docs/baselines.md)",
            mkl.reproducibility
        )
    };
    let what = kernels(
        Provider::Mkl,
        mkl.version.clone(),
        "Intel Simplified Software License (installed by the user)",
        Threads::Library(mkl.threads()),
        &found.interface,
        reproducibility,
    );
    Ok(Blas {
        what,
        interface: found.interface,
        triangular: found.calls.triangular_product(),
        threaded_lu: true,
        calls: found.calls,
        threading: Threading::Mkl(mkl.clone()),
        files: vec![library.path().to_path_buf()],
        _libraries: Vec::new(),
    })
}

fn accelerate(want: Want) -> Result<Blas, String> {
    if !cfg!(target_os = "macos") {
        return Err("Accelerate is part of macOS: it isn't on this system".into());
    }
    let library = Library::open(Path::new(FRAMEWORK)).map_err(reason)?;
    // SAFETY: Accelerate's BLAS and LAPACK by their classic names
    let found = unsafe {
        detect(
            &library,
            &library,
            Provider::Accelerate.prefixes(),
            Provider::Accelerate.forms(),
            want,
        )
    }?;
    let what = kernels(
        Provider::Accelerate,
        format!("macOS {}", macos_version()),
        "part of macOS, under Apple's licence for it",
        Threads::Library("Accelerate's own, which a call doesn't set".into()),
        &found.interface,
        "Apple documents no reproducibility for Accelerate's BLAS; not measured".into(),
    );
    Ok(Blas {
        what,
        interface: found.interface,
        triangular: found.calls.triangular_product(),
        threaded_lu: true,
        calls: found.calls,
        threading: Threading::Fixed,
        files: vec![library.path().to_path_buf()],
        _libraries: vec![library],
    })
}

/// The threads of a library that threads by OpenMP: set on the calling thread, or held to
/// one by `hold` where no OpenMP runtime is found.
fn by_openmp(
    library: &Library,
    owner: &str,
    hold: impl FnOnce(),
) -> (Threading, Threads, Option<Library>) {
    match openmp(library) {
        Some((set, get, runtime)) => (
            Threading::OpenMp(set, get),
            Threads::Library(format!(
                "{owner}'s OpenMP, on rayon's thread count (RAYON_NUM_THREADS) by \
                 omp_set_num_threads on the calling thread"
            )),
            runtime,
        ),
        None => {
            hold();
            (Threading::Fixed, Threads::One, None)
        }
    }
}

fn armpl(library: Library, found: Detected) -> Blas {
    type Version = unsafe extern "C" fn(*mut c_int, *mut c_int, *mut c_int, *mut *const c_char);
    // SAFETY: `void armplversion(int *major, int *minor, int *patch, const char **tag)`
    let version = match unsafe { library.function::<Version>("armplversion") } {
        Ok(f) => {
            let (mut major, mut minor, mut patch) = (0, 0, 0);
            let mut tag: *const c_char = std::ptr::null();
            // SAFETY: four places to write, as declared
            unsafe { f(&mut major, &mut minor, &mut patch, &mut tag) };
            format!("{major}.{minor}.{patch}")
        }
        Err(_) => "unknown".into(),
    };
    let (threading, threads, runtime) = by_openmp(&library, "Arm PL", || {});
    let what = kernels(
        Provider::ArmPl,
        version,
        "Arm Performance Libraries' end user licence, free of charge (installed by the user)",
        threads,
        &found.interface,
        "Arm documents no reproducibility for Arm PL's BLAS; not measured (no Arm processor \
         was at hand)"
            .into(),
    );
    let mut libraries = vec![library];
    libraries.extend(runtime);
    Blas {
        what,
        interface: found.interface,
        triangular: found.calls.triangular_product(),
        threaded_lu: true,
        calls: found.calls,
        threading,
        files: vec![libraries[0].path().to_path_buf()],
        _libraries: libraries,
    }
}

/// The discovery of a library loaded and then found wanting: the candidate used becomes one
/// that failed, with the reason.
fn failed(mut discovery: Discovery, why: String) -> (Discovery, Result<Blas, String>) {
    for candidate in &mut discovery.candidates {
        if candidate.status == crate::Status::Used {
            candidate.status = crate::Status::Failed(why.clone());
        }
    }
    (discovery, Err(why))
}

/// AOCL: one library with BLAS and LAPACK (`aocl.dll`, `libaocl.so`), or AOCL-BLAS and then
/// AOCL-LAPACK beside it, its utilities loaded first.
fn aocl(setting: Option<&Path>, want: Want) -> (Discovery, Result<Blas, String>) {
    if !Provider::Aocl.here() {
        let discovery = crate::discover(&AOCL, setting);
        return (
            discovery,
            Err("AMD AOCL has builds for x86-64 under Windows and Linux only".into()),
        );
    }
    let mut libraries = Vec::new();
    // its utilities, which BLIS and libFLAME load, from where AOCL installs them
    let (_, utilities) = load(&AOCL_UTILS, None, |_| Ok(()));
    libraries.extend(utilities);
    let has = |l: &Library, names: &[&str]| {
        names.iter().any(|n| {
            // SAFETY: only resolved; the type is any function's
            unsafe { l.function::<unsafe extern "C" fn()>(n) }.is_ok()
        })
    };
    let (discovery, blas) = load(&AOCL, setting, |l| {
        if has(l, &["zgemm_", "zgemm", "cblas_zgemm"]) {
            Ok(())
        } else {
            Err(error("it has no zgemm"))
        }
    });
    let Some(blas) = blas else {
        return (discovery.clone(), Err(discovery.reason()));
    };
    let single = has(&blas, &["zgetrf_", "zgetrf", "LAPACKE_zgetrf_work"]);
    let lapack = if single {
        None
    } else {
        // libFLAME of the same build: beside BLIS (Linux), or in amd-libflame where BLIS is
        // in amd-blis, with the same integers (Windows)
        let folder = blas.path().parent().map(Path::to_path_buf);
        let beside = folder.as_ref().map(|f| {
            let integers = f.file_name().map(|n| n.to_os_string());
            match (f.parent().and_then(Path::parent), integers) {
                (Some(root), Some(integers))
                    if root.file_name().is_some_and(|n| n == "amd-blis") =>
                {
                    root.with_file_name("amd-libflame")
                        .join("lib")
                        .join(integers)
                }
                _ => f.clone(),
            }
        });
        let beside = beside.filter(|b| b.is_dir());
        let (found, lapack) = load(&AOCL_LAPACK, beside.as_deref(), |l| {
            if has(l, &["zgetrf_", "zgetrf", "LAPACKE_zgetrf_work"]) {
                Ok(())
            } else {
                Err(error("it has no zgetrf"))
            }
        });
        match lapack {
            Some(lapack) => Some(lapack),
            None => {
                let why = format!(
                    "{} has AOCL-BLAS, but its LAPACK wasn't found: {}",
                    blas.path().display(),
                    found.reason()
                );
                return failed(discovery, why);
            }
        }
    };
    let lapack_library = lapack.as_ref().unwrap_or(&blas);
    // SAFETY: AOCL's BLAS and LAPACK by their standard names
    let found = unsafe {
        detect(
            &blas,
            lapack_library,
            Provider::Aocl.prefixes(),
            Provider::Aocl.forms(),
            want,
        )
    };
    let found = match found {
        Ok(found) => found,
        Err(why) => return failed(discovery, why),
    };
    // SAFETY: `const char *bli_info_get_version_str(void)`, a static string
    let version = unsafe {
        blas.function::<unsafe extern "C" fn() -> *const c_char>("bli_info_get_version_str")
    }
    // SAFETY: as above
    .map(|f| unsafe { text(f) })
    .unwrap_or_default();
    let version = if version.is_empty() {
        "unknown".into()
    } else {
        version
    };
    let (threading, threads, runtime) = by_openmp(&blas, "AOCL-BLAS", || {
        // SAFETY: `void bli_thread_set_num_threads(dim_t)`, dim_t a 64-bit integer: one
        // thread for every call, where no per-call control was found
        let set =
            unsafe { blas.function::<unsafe extern "C" fn(i64)>("bli_thread_set_num_threads") };
        if let Ok(set) = set {
            // SAFETY: as above
            unsafe { set(1) };
        }
    });
    let what = kernels(
        Provider::Aocl,
        version,
        "BSD-3-Clause: AOCL-BLAS (BLIS) and AOCL-LAPACK (libFLAME) (installed by the user)",
        threads,
        &found.interface,
        "AMD documents no reproducibility for AOCL-BLAS; not measured (not installed on the \
         machine photonoxide's backends were checked on)"
            .into(),
    );
    let mut files = vec![blas.path().to_path_buf()];
    files.extend(lapack.as_ref().map(|l| l.path().to_path_buf()));
    // the order they are dropped in: LAPACK, BLAS, the runtime, the utilities
    let mut kept = Vec::new();
    kept.extend(lapack);
    kept.push(blas);
    kept.extend(runtime);
    kept.extend(libraries);
    (
        discovery,
        Ok(Blas {
            what,
            interface: found.interface,
            triangular: found.calls.triangular_product(),
            threaded_lu: true,
            calls: found.calls,
            threading,
            files,
            _libraries: kept,
        }),
    )
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
        let call = Gemm {
            transposed: product.transposed,
            lower: product.lower && self.triangular,
            m,
            n,
            k: r,
            alpha,
            a: a.pointer(),
            lda: a.stride(),
            b: b.pointer(),
            ldb: b.stride(),
            c: c.pointer(),
            ldc: c.stride(),
        };
        // SAFETY: C is m × n, A is m × r, op(B) is r × n, checked above; each block is as
        // large as its dimensions and stride say (a Block's promise); C, borrowed mutably,
        // overlaps neither
        self.threaded(threaded, || unsafe { self.calls.gemm(&call) });
    }

    fn solve_unit_lower(&self, l: Block<'_>, b: BlockMut<'_>, threaded: bool) {
        self.triangular(l, b, *b"LLNU", threaded);
    }

    fn solve_upper_from_right(&self, u: Block<'_>, b: BlockMut<'_>, threaded: bool) {
        self.triangular(u, b, *b"RUNN", threaded);
    }

    fn solve_unit_lower_transposed_from_right(
        &self,
        l: Block<'_>,
        b: BlockMut<'_>,
        threaded: bool,
    ) {
        self.triangular(l, b, *b"RLTU", threaded);
    }

    fn lu(&self, mut a: BlockMut<'_>, threaded: bool) -> Vec<usize> {
        let n = a.rows();
        assert_eq!(a.columns(), n, "a square block");
        let mut rows: Vec<usize> = (0..n).collect();
        if n == 0 {
            return rows;
        }
        let (lda, pointer) = (a.stride(), a.pointer());
        // SAFETY: A is n × n with leading dimension lda (a Block's promise)
        let (pivots, info) = self.threaded(threaded && self.threaded_lu, || unsafe {
            self.calls.getrf(n, pointer, lda)
        });
        // (a positive info is a zero pivot, which the caller looks for itself)
        assert!(info >= 0, "zgetrf refused its argument {}", -info);
        // LAPACK's pivots are the swaps it made, one after another, from 1
        for (i, &p) in pivots.iter().enumerate() {
            let p = usize::try_from(p - 1).expect("a pivot from 1");
            assert!(p < n, "zgetrf's pivot {} of {n} rows", p + 1);
            rows.swap(i, p);
        }
        rows
    }
}

/// Every library's kernels that can be on this system, each with its name, or why not.
pub fn all() -> Vec<(&'static str, Result<Blas, String>)> {
    Provider::ALL
        .into_iter()
        .filter(|p| p.here())
        .map(|p| (p.name(), Blas::load(p)))
        .collect()
}

/// A provider found or not: where it was looked for, its version and what is known of its
/// kernels. For oneMKL and Accelerate, whose files are found elsewhere, their own probe
/// ([`crate::intel::probe`], [`crate::accelerate::probe`]) with what is known of their kernels.
pub fn probe(provider: Provider) -> Probe {
    let (discovery, blas) = Blas::found(provider, None, Want::default());
    let Some(discovery) = discovery else {
        // oneMKL's and Accelerate's own probes, with their kernels'
        let mut probe = match provider {
            Provider::Mkl => crate::intel::probe(),
            _ => crate::accelerate::probe(),
        };
        probe.details.extend(match blas {
            Ok(blas) => blas.details(),
            Err(why) => vec![format!("its dense kernels: {why}")],
        });
        return probe;
    };
    let (version, details) = match blas {
        Ok(blas) => {
            let mut details = blas.details();
            if blas.files.len() > 1 {
                let files: Vec<String> =
                    blas.files.iter().map(|f| f.display().to_string()).collect();
                details.insert(0, format!("files: {}", files.join(", ")));
            }
            (Some(blas.what.version.clone()), details)
        }
        Err(why) if discovery.used().is_some() => (None, vec![why]),
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
        // each provider can be asked for, and says why not where it can't be
        for provider in Provider::ALL {
            if !provider.here() {
                assert!(Blas::load(provider).is_err(), "{provider:?}");
            }
        }
    }

    #[test]
    fn every_spelling_names_each_routine() {
        let names: Vec<String> = FORMS.iter().map(|f| f.name("", "zgemm").unwrap()).collect();
        assert_eq!(
            names,
            [
                "zgemm_",
                "zgemm",
                "ZGEMM",
                "zgemm_64_",
                "zgemm_64",
                "zgemm64_",
                "cblas_zgemm",
                "cblas_zgemm_64",
                "cblas_zgemm64_"
            ]
        );
        assert_eq!(
            Form::C("64_").name("scipy_", "zgetrf").unwrap(),
            "scipy_LAPACKE_zgetrf_work64_"
        );
        assert_eq!(
            Form::Under64Under.name("scipy_", "zgetrf").unwrap(),
            "scipy_zgetrf_64_"
        );
        assert_eq!(Form::C("").name("", "zdotc"), None);
        // the suffix OpenBLAS's other functions have beside them
        assert_eq!(Form::Plain64Under.suffix(), "64_");
        assert_eq!(Form::C("_64").suffix(), "_64");
        assert_eq!(Form::Upper.suffix(), "");
        // Fortran's before C's, 32-bit names before 64-bit ones
        assert_eq!(FORMS[0].style(), Style::Fortran);
        assert_eq!(FORMS[FORMS.len() - 1].style(), Style::C);
        // Accelerate by its classic names only
        assert!(
            Provider::Accelerate
                .forms()
                .iter()
                .all(|f| matches!(f, Form::Under | Form::C("")))
        );
    }

    #[test]
    fn the_bytes_an_integer_took_are_read_from_what_it_left() {
        let cell = |bytes: &[u8]| -> Cell {
            let mut all = [0xAAu8; 16];
            all[..bytes.len()].copy_from_slice(bytes);
            [
                u64::from_ne_bytes(all[..8].try_into().unwrap()),
                u64::from_ne_bytes(all[8..].try_into().unwrap()),
            ]
        };
        assert_eq!(bytes_written(&cell(&1i32.to_ne_bytes()), 1), Some(4));
        assert_eq!(bytes_written(&cell(&1i64.to_ne_bytes()), 1), Some(8));
        assert_eq!(bytes_written(&cell(&0i32.to_ne_bytes()), 0), Some(4));
        assert_eq!(bytes_written(&cell(&0i64.to_ne_bytes()), 0), Some(8));
        // untouched, or another value, or more than 8 bytes
        assert_eq!(bytes_written(&[UNTOUCHED; 2], 1), None);
        assert_eq!(bytes_written(&cell(&2i32.to_ne_bytes()), 1), None);
        assert_eq!(bytes_written(&[0, 0], 0), None);
    }

    #[test]
    fn the_providers_have_names_and_places() {
        let names: Vec<&str> = Provider::ALL.iter().map(|p| p.name()).collect();
        assert_eq!(names, ["openblas", "mkl", "aocl", "armpl", "accelerate"]);
        for p in Provider::ALL {
            assert!(!p.library().is_empty());
        }
        // AOCL's folders: 32-bit integers before 64, BLIS's and libFLAME's
        let folders = aocl_folders();
        let text: Vec<String> = folders.iter().map(|f| f.display().to_string()).collect();
        let lp64 = text
            .iter()
            .position(|f| f.contains("LP64") && !f.contains("ILP64"));
        let ilp64 = text.iter().position(|f| f.contains("ILP64"));
        assert!(lp64 < ilp64, "{text:?}");
        if cfg!(windows) {
            assert!(text.iter().any(|f| f.contains("amd-libflame")), "{text:?}");
            assert!(text.iter().any(|f| f.contains("amd-utils")), "{text:?}");
        }
        assert!(!armpl_folders().is_empty());
    }
}
