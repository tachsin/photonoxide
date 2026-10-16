//! Discovery, loading, versions, errors and the smoke test, against the workspace's test
//! library (`native/testlib`), so that CI tests them with no real library installed.

use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};

use num_complex::Complex64 as c64;
use photonoxide::Result;
use photonoxide::backend::dense::{Block, BlockMut, DenseKernels, Faer, Product, with_kernels};
use photonoxide::backend::{
    self, Analysis, Capabilities, Choice, DirectSolver, Factorization, Form, Matrix, Report,
};
use photonoxide_native::blas::{ComplexResults, Integers, Provider, Style, Want};
use photonoxide_native::{Blas, Library, Source, Spec, Status, discover, load, offer, smoke_test};

type Version = unsafe extern "C" fn(*mut c_int, *mut c_int, *mut c_int) -> c_int;
type Solve = unsafe extern "C" fn(i64, *const f64, *mut f64) -> c_int;

const FILE: &str = if cfg!(windows) {
    "photonoxide_native_testlib.dll"
} else if cfg!(target_os = "macos") {
    "libphotonoxide_native_testlib.dylib"
} else {
    "libphotonoxide_native_testlib.so"
};

/// The test library, built once into this profile's folder.
fn folder() -> &'static Path {
    static FOLDER: OnceLock<PathBuf> = OnceLock::new();
    FOLDER.get_or_init(|| {
        // target/<profile>/deps/this-test → target/<profile>
        let exe = std::env::current_exe().unwrap();
        let profile = exe.parent().unwrap().parent().unwrap().to_path_buf();
        let mut cargo = Command::new(env!("CARGO"));
        cargo.args(["build", "-q", "-p", "photonoxide-native-testlib"]);
        if profile.file_name().unwrap() == "release" {
            cargo.arg("--release");
        }
        let target = profile.parent().unwrap();
        assert!(
            cargo
                .env("CARGO_TARGET_DIR", target)
                .status()
                .unwrap()
                .success()
        );
        assert!(profile.join(FILE).is_file(), "{}", profile.display());
        profile
    })
}

fn install() -> Vec<PathBuf> {
    vec![folder().to_path_buf()]
}

const SPEC: Spec = Spec {
    name: "test library",
    files: &[FILE],
    variables: &[],
    install,
    wheel: None,
};

const MISSING: Spec = Spec {
    name: "missing library",
    files: &["no-such-library-anywhere.dll"],
    variables: &["PHOTONOXIDE_NO_SUCH_LIBRARY"],
    install: Vec::new,
    wheel: None,
};

fn version(library: &Library) -> Result<(c_int, c_int, c_int)> {
    // SAFETY: `int pxtest_version(int *, int *, int *)`
    let get: Version = unsafe { library.function("pxtest_version")? };
    let (mut a, mut b, mut c) = (0, 0, 0);
    // SAFETY: three writable ints
    assert_eq!(unsafe { get(&mut a, &mut b, &mut c) }, 0);
    Ok((a, b, c))
}

#[test]
fn it_is_found_in_its_install_folder_and_by_the_setting() {
    // cargo test also puts target/<profile>/deps, with a copy, on the library path
    let found = discover(&SPEC, None);
    assert_eq!(
        found.candidates[0].source,
        Source::InstallFolder,
        "{found:?}"
    );
    // the setting first, and the same file only once
    let file = folder().join(FILE);
    let found = discover(&SPEC, Some(&file));
    assert_eq!(
        found.candidates.iter().filter(|c| c.path == file).count(),
        1
    );
    assert_eq!(found.candidates[0].source, Source::Setting);
}

#[test]
fn a_missing_library_says_so() {
    let (found, library) = load(&MISSING, None, |_| Ok(()));
    assert!(library.is_none() && found.candidates.is_empty());
    assert_eq!(found.reason(), "missing library wasn't found");
}

#[test]
fn it_loads_and_reports_its_version_and_integers() {
    let (found, library) = load(&SPEC, None, |l| version(l).map(drop));
    let library = library.unwrap();
    assert_eq!(found.used().unwrap().status, Status::Used);
    assert_eq!(version(&library).unwrap(), (1, 2, 3));
    // SAFETY: `int pxtest_index_bytes(void)`
    let bytes: unsafe extern "C" fn() -> c_int =
        unsafe { library.function("pxtest_index_bytes") }.unwrap();
    // SAFETY: no arguments
    assert_eq!(unsafe { bytes() }, 8);
}

#[test]
fn a_failed_check_or_a_missing_function_is_recorded() {
    let (found, library) = load(&SPEC, None, |l| {
        // SAFETY: never called
        unsafe { l.function::<Version>("pxtest_no_such_function") }.map(drop)
    });
    assert!(library.is_none());
    let reason = found.reason();
    assert!(
        matches!(&found.candidates[0].status, Status::Failed(_))
            && reason.contains("pxtest_no_such_function"),
        "{reason}"
    );
    let broken = folder().join("Cargo.toml");
    assert!(Library::open(&broken).is_err());
}

/// The test library's dense solve as a backend; `lie` corrupts its answers.
struct Dense {
    library: Arc<Library>,
    name: &'static str,
    lie: bool,
}

struct Factors {
    library: Arc<Library>,
    n: usize,
    dense: Vec<f64>,
    lie: bool,
}

impl DirectSolver for Dense {
    fn capabilities(&self) -> Capabilities {
        let mut c = Capabilities::new(self.name, "1.2.3", "MIT OR Apache-2.0");
        c.transpose = true;
        c.deterministic = true;
        c
    }

    fn analyse(&self, matrix: &Matrix<'_>) -> Result<Option<Arc<dyn Analysis>>> {
        let _ = matrix;
        Ok(Some(Arc::new(Dense {
            library: self.library.clone(),
            name: self.name,
            lie: self.lie,
        })))
    }
}

impl Analysis for Dense {
    fn form(&self) -> Form {
        Form::General
    }

    fn factorize(&self, matrix: &Matrix<'_>) -> Result<Box<dyn Factorization>> {
        let n = matrix.n();
        let mut dense = vec![0.0; 2 * n * n];
        for j in 0..n {
            for k in matrix.column_starts()[j]..matrix.column_starts()[j + 1] {
                let i = matrix.row_indices()[k];
                let v = matrix.values()[k];
                dense[2 * (j * n + i)] = v.re;
                dense[2 * (j * n + i) + 1] = v.im;
            }
        }
        Ok(Box::new(Factors {
            library: self.library.clone(),
            n,
            dense,
            lie: self.lie,
        }))
    }
}

impl Factors {
    fn run(&self, dense: &[f64], b: &[c64]) -> Result<Vec<c64>> {
        // SAFETY: `int pxtest_dense_solve(int64_t n, const double *a, double *b)`
        let solve: Solve = unsafe { self.library.function("pxtest_dense_solve")? };
        let mut x: Vec<f64> = b.iter().flat_map(|v| [v.re, v.im]).collect();
        // SAFETY: dense holds 2n² doubles and x 2n
        let status = unsafe { solve(self.n as i64, dense.as_ptr(), x.as_mut_ptr()) };
        if status != 0 {
            return Err(photonoxide::Error::InvalidValue {
                what: "test library",
                reason: format!("pxtest_dense_solve returned {status}"),
            });
        }
        let lie = if self.lie { 1e-6 } else { 0.0 };
        Ok(x.chunks(2).map(|p| c64::new(p[0] + lie, p[1])).collect())
    }
}

impl Factorization for Factors {
    fn solve(&self, b: &[c64]) -> Result<Vec<c64>> {
        self.run(&self.dense, b)
    }

    fn solve_transpose(&self, b: &[c64]) -> Result<Vec<c64>> {
        let n = self.n;
        let mut t = vec![0.0; 2 * n * n];
        for j in 0..n {
            for i in 0..n {
                t[2 * (i * n + j)] = self.dense[2 * (j * n + i)];
                t[2 * (i * n + j) + 1] = self.dense[2 * (j * n + i) + 1];
            }
        }
        self.run(&t, b)
    }

    fn report(&self) -> Report {
        Report::default()
    }
}

fn dense(name: &'static str, lie: bool) -> Arc<Dense> {
    let (_, library) = load(&SPEC, None, |_| Ok(()));
    Arc::new(Dense {
        library: Arc::new(library.unwrap()),
        name,
        lie,
    })
}

#[test]
fn a_backend_that_agrees_is_offered_and_one_that_doesnt_is_not() {
    let good = dense("testlib", false);
    let difference = smoke_test(good.as_ref()).unwrap();
    assert!(difference < 1e-13, "{difference:e}");
    assert!(offer(good).unwrap());
    assert!(backend::direct(&Choice::Named("testlib".into())).is_ok());

    assert!(!offer(dense("testlib-wrong", true)).unwrap());
    let refused = backend::direct(&Choice::Named("testlib-wrong".into()))
        .err()
        .unwrap()
        .to_string();
    assert!(refused.contains("smoke test failed"), "{refused}");
}

#[test]
fn a_library_error_becomes_photonoxides() {
    let good = dense("testlib-singular", false);
    let (starts, rows, values) = (vec![0, 1, 1], vec![0], vec![c64::new(1.0, 0.0)]);
    let matrix = Matrix::new(2, &starts, &rows, &values, Form::General).unwrap();
    let factors = good
        .analyse(&matrix)
        .unwrap()
        .unwrap()
        .factorize(&matrix)
        .unwrap();
    let e = factors.solve(&[c64::new(1.0, 0.0); 2]).err().unwrap();
    assert!(e.to_string().contains("returned 2"), "{e}");
}

/// The test library's BLAS and LAPACK by its interface `want`, loaded as OpenBLAS's loader
/// loads a file its setting names; the test library's, or why not.
fn test_blas(want: Want) -> std::result::Result<Blas, String> {
    let path = folder().join(FILE);
    Blas::find(Provider::OpenBlas, Some(&path), want).and_then(|blas| {
        // (another candidate, a real OpenBLAS installed here, if this one hasn't that interface)
        if blas.files()[0] == path {
            Ok(blas)
        } else {
            Err(format!("another library: {}", blas.files()[0].display()))
        }
    })
}

/// A's kernels against faer's on small blocks, not square: the largest difference.
fn against_faer(kernels: &dyn DenseKernels) -> f64 {
    let entries = |rows: usize, columns: usize, seed: f64| -> Vec<c64> {
        (0..rows * columns)
            .map(|k| {
                c64::new(
                    (k as f64 * 0.37 + seed).sin(),
                    (k as f64 * 0.21 - seed).cos(),
                )
            })
            .collect()
    };
    let (m, n, r) = (13, 9, 6);
    let mut worst: f64 = 0.0;
    let mut differ = |a: &[c64], b: &[c64]| {
        let scale = b.iter().map(|x| x.norm()).fold(0.0, f64::max);
        let d = a
            .iter()
            .zip(b)
            .map(|(a, b)| (a - b).norm())
            .fold(0.0, f64::max)
            / scale;
        worst = worst.max(d);
    };
    for transposed in [false, true] {
        let a = entries(m, r, 1.0);
        let b = if transposed {
            entries(n, r, 2.0)
        } else {
            entries(r, n, 2.0)
        };
        let (br, bc) = if transposed { (n, r) } else { (r, n) };
        let c0 = entries(m, n, 3.0);
        let (mut ours, mut theirs) = (c0.clone(), c0);
        let product = Product {
            transposed,
            lower: false,
        };
        for (k, c) in [
            (&Faer as &dyn DenseKernels, &mut ours),
            (kernels, &mut theirs),
        ] {
            k.multiply(
                BlockMut::of(c, m, n),
                Block::of(&a, m, r),
                Block::of(&b, br, bc),
                product,
                c64::new(-0.5, 0.25),
                false,
            );
        }
        differ(&theirs, &ours);
    }
    // a triangle with a large diagonal, and the three solves
    let mut t = entries(m, m, 4.0);
    for i in 0..m {
        t[i + i * m] += c64::new(m as f64, 0.0);
    }
    for (k, (rows, columns)) in [(m, n), (n, m), (n, m)].into_iter().enumerate() {
        let b0 = entries(rows, columns, 5.0 + k as f64);
        let (mut ours, mut theirs) = (b0.clone(), b0);
        for (kernels, b) in [
            (&Faer as &dyn DenseKernels, &mut ours),
            (kernels, &mut theirs),
        ] {
            let (l, b) = (Block::of(&t, m, m), BlockMut::of(b, rows, columns));
            match k {
                0 => kernels.solve_unit_lower(l, b, false),
                1 => kernels.solve_upper_from_right(l, b, false),
                _ => kernels.solve_unit_lower_transposed_from_right(l, b, false),
            }
        }
        differ(&theirs, &ours);
    }
    // the LU, by what it factorizes: A's rows in the order returned are L U
    let a = entries(m, m, 6.0);
    let mut factors = a.clone();
    let rows = kernels.lu(BlockMut::of(&mut factors, m, m), false);
    let product: Vec<c64> = (0..m * m)
        .map(|e| {
            let (i, j) = (e % m, e / m);
            (0..=i.min(j))
                .map(|k| {
                    let l = if k == i {
                        c64::new(1.0, 0.0)
                    } else {
                        factors[i + k * m]
                    };
                    l * factors[k + j * m]
                })
                .sum()
        })
        .collect();
    let permuted: Vec<c64> = (0..m * m).map(|e| a[rows[e % m] + (e / m) * m]).collect();
    differ(&product, &permuted);
    worst
}

#[test]
fn a_blas_interface_is_measured_not_assumed() {
    // Fortran's names, which take 64-bit integers here: found so by what zgetrf writes
    let fortran = test_blas(Want::new(Some(Style::Fortran), None)).unwrap();
    let i = fortran.interface().clone();
    assert_eq!(
        (i.style, i.integers),
        (Style::Fortran, Integers::Ilp64),
        "{i}"
    );
    assert_eq!((i.zgemm.as_str(), i.zgetrf.as_str()), ("zgemm_", "zgetrf_"));
    let complex = if cfg!(all(windows, target_arch = "x86_64")) {
        ComplexResults::OneConvention
    } else {
        ComplexResults::Argument
    };
    assert_eq!(i.complex_results, complex, "{i}");
    // without zgemmt, and without a thread count to set: held to one
    assert!(!fortran.has_triangular_product() && !fortran.threads_per_call());
    // the C interfaces, with 32-bit integers
    let c = test_blas(Want::new(Some(Style::C), None)).unwrap();
    let i = c.interface().clone();
    assert_eq!((i.style, i.integers), (Style::C, Integers::Lp64), "{i}");
    assert_eq!(i.zgetrf, "LAPACKE_zgetrf_work");
    assert_eq!(i.complex_results, ComplexResults::C);
    // any interface: Fortran's first
    let any = test_blas(Want::default()).unwrap();
    assert_eq!(any.interface().style, Style::Fortran);
    // Fortran's with 32-bit integers isn't this library's: its names would have said so
    let why = test_blas(Want::new(Some(Style::Fortran), Some(Integers::Lp64))).unwrap_err();
    assert!(
        why.contains("64-bit integers") || why.contains("another library"),
        "{why}"
    );
    for blas in [fortran, c] {
        let d = against_faer(&blas);
        assert!(d < 1e-13, "{}: {d:e}", blas.interface());
        // and photonoxide's solver with them is its own
        let solver = with_kernels(Arc::new(blas));
        assert!(smoke_test(solver.as_ref()).unwrap() < 1e-12);
    }
}

#[test]
fn every_known_library_is_reported_found_or_not() {
    for probe in photonoxide_native::register_all() {
        let line = probe.to_string();
        assert!(
            probe.discovery.used().is_some() == probe.version.is_some(),
            "{line}"
        );
        println!("{line}");
    }
}
