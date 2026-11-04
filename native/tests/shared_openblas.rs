//! OpenBLAS called by other libraries while photonoxide's calls into it thread: MUMPS and
//! SuperLU, installed from one conda environment with the OpenBLAS, link the same one and call
//! it outside photonoxide's lock (#557, #595). A threaded call of ours sets OpenBLAS's count of
//! threads around itself, and OpenBLAS built with OpenMP frees the buffers of the threads above
//! a lower count; a MUMPS or SuperLU call running meanwhile may be using them. Every product of
//! ours and every solve of theirs must stay right.
//!
//! Skipped, and saying so, where OpenBLAS, MUMPS or SuperLU isn't found; with
//! `PHOTONOXIDE_REQUIRE_BLAS` (`openblas`), `PHOTONOXIDE_REQUIRE_MUMPS` and
//! `PHOTONOXIDE_REQUIRE_SUPERLU` set, as the Libraries workflow's rows that install the three
//! together set them, not finding one is a failure. `PHOTONOXIDE_BLAS_ROUNDS` sets the rounds
//! of products (default 10).

use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use num_complex::Complex64 as c64;
use photonoxide::backend::dense::{Block, BlockMut, DenseKernels, Faer, Product};
use photonoxide::backend::{self, Choice, DirectSolver, Form, Matrix};
use photonoxide_native::{Blas, Mumps, SuperLu};
use rayon::prelude::*;

/// Entries in the unit square, the same on every run.
struct Random(u64);

impl Random {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64 - 0.5
    }

    fn matrix(&mut self, rows: usize, columns: usize) -> Vec<c64> {
        (0..rows * columns)
            .map(|_| c64::new(self.next(), self.next()))
            .collect()
    }
}

/// The largest difference, relative to `b`'s largest entry.
fn relative(a: &[c64], b: &[c64]) -> f64 {
    let scale = b.iter().map(|x| x.norm()).fold(0.0, f64::max);
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b).norm())
        .fold(0.0, f64::max)
        / scale
}

/// A Helmholtz operator with loss on a grid of these cells, general with an asymmetric
/// coupling along x.
fn helmholtz(cells: [usize; 3]) -> (Vec<usize>, Vec<usize>, Vec<c64>) {
    let [mx, my, mz] = cells;
    let n = mx * my * mz;
    let strides = [1, mx, mx * my];
    let dimensions = cells.iter().filter(|&&m| m > 1).count() as f64;
    let (mut starts, mut rows, mut values) = (vec![0], Vec::new(), Vec::new());
    for j in 0..n {
        let at = [j % mx, j / mx % my, j / (mx * my)];
        let mut column: Vec<(usize, c64)> = Vec::new();
        for axis in 0..3 {
            if at[axis] > 0 {
                column.push((j - strides[axis], c64::new(1.0, 0.0)));
            }
            if at[axis] + 1 < cells[axis] {
                let v = if axis == 0 {
                    c64::new(1.0, 0.1)
                } else {
                    c64::new(1.0, 0.0)
                };
                column.push((j + strides[axis], v));
            }
        }
        let diagonal = c64::new(-2.0 * dimensions + 0.3, 0.02 + 0.001 * (j % 11) as f64);
        column.push((j, diagonal));
        column.sort_by_key(|&(r, _)| r);
        for (r, v) in column {
            rows.push(r);
            values.push(v);
        }
        starts.push(rows.len());
    }
    (starts, rows, values)
}

fn rhs(n: usize) -> Vec<c64> {
    (0..n)
        .map(|k| c64::new(1.0, k as f64 * 1e-3).powi((k % 3) as i32))
        .collect()
}

/// Whether `name` is required here (the variable is set), and why it isn't found if it isn't.
fn found<T>(name: &str, variable: &str, library: Result<T, String>) -> Option<T> {
    match library {
        Ok(library) => Some(library),
        Err(reason) => {
            assert!(
                std::env::var_os(variable).is_none(),
                "{name} is required here and isn't found: {reason}"
            );
            println!("skipped: {name} isn't here: {reason}");
            None
        }
    }
}

/// Rounds of products done, and MUMPS and SuperLU solves, for the watchdog to report.
static ROUNDS_DONE: AtomicUsize = AtomicUsize::new(0);
static MUMPS_SOLVES: AtomicUsize = AtomicUsize::new(0);
static SUPERLU_SOLVES: AtomicUsize = AtomicUsize::new(0);

/// A line on the process's stderr, which the test harness doesn't capture: a run that stalls
/// still says how far it got.
fn say(started: Instant, what: &str) {
    let _ = writeln!(
        std::io::stderr(),
        "[{:6.1} s] {what}",
        started.elapsed().as_secs_f64()
    );
}

/// Ends the process, saying how far the test got, if it isn't `finished` within `seconds`
/// (`PHOTONOXIDE_SHARED_SECONDS`, default 300: a run takes seconds): calls that block each
/// other would hold a CI row for its whole timeout, and say nothing.
fn watchdog(started: Instant, finished: &'static AtomicBool, rounds: usize) {
    let seconds: u64 = std::env::var("PHOTONOXIDE_SHARED_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);
    std::thread::spawn(move || {
        while started.elapsed() < Duration::from_secs(seconds) {
            if finished.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        say(
            started,
            &format!(
                "STALLED after {seconds} s: {} of {rounds} rounds of products, {} MUMPS and {}                  SuperLU solves",
                ROUNDS_DONE.load(Ordering::Relaxed),
                MUMPS_SOLVES.load(Ordering::Relaxed),
                SUPERLU_SOLVES.load(Ordering::Relaxed)
            ),
        );
        std::process::exit(2);
    });
}

/// The OpenBLAS files this process has mapped, each once (Linux; empty elsewhere).
fn mapped_openblas() -> Vec<String> {
    let Ok(maps) = std::fs::read_to_string("/proc/self/maps") else {
        return Vec::new();
    };
    let mut files: Vec<String> = maps
        .lines()
        .filter_map(|line| line.split_whitespace().nth(5))
        .filter(|path| path.contains("openblas"))
        .map(str::to_owned)
        .collect();
    files.sort();
    files.dedup();
    files
}

#[test]
fn openblas_called_by_mumps_and_superlu_meanwhile_leaves_every_result_right() {
    let Some(openblas) = found("OpenBLAS", "PHOTONOXIDE_REQUIRE_BLAS", Blas::openblas()) else {
        return;
    };
    let Some(mumps) = found("MUMPS", "PHOTONOXIDE_REQUIRE_MUMPS", Mumps::load()) else {
        return;
    };
    let Some(superlu) = found("SuperLU", "PHOTONOXIDE_REQUIRE_SUPERLU", SuperLu::load()) else {
        return;
    };
    static FINISHED: AtomicBool = AtomicBool::new(false);
    let started = Instant::now();
    let rounds: usize = std::env::var("PHOTONOXIDE_BLAS_ROUNDS")
        .ok()
        .and_then(|r| r.parse().ok())
        .unwrap_or(10);

    watchdog(started, &FINISHED, rounds);
    // the same OpenBLAS: on Linux, one file of the name is mapped for all three
    let files = mapped_openblas();
    println!("OpenBLAS files mapped: {files:?}");
    if cfg!(target_os = "linux") {
        assert_eq!(
            files.len(),
            1,
            "MUMPS, SuperLU and photonoxide should share one OpenBLAS: {files:?}"
        );
    }

    // photonoxide's own solution of a system with fronts large enough that MUMPS and SuperLU
    // call the BLAS for dense blocks
    let (s, r, v) = helmholtz([20, 20, 10]);
    let m = Matrix::new(s.len() - 1, &s, &r, &v, Form::General).unwrap();
    let b = rhs(m.n());
    let own = backend::direct(&Choice::Photonoxide).unwrap();
    let want = own
        .analyse(&m)
        .unwrap()
        .unwrap()
        .factorize(&m)
        .unwrap()
        .solve(&b)
        .unwrap();

    say(started, "photonoxide's own solution done");

    // photonoxide's products, as the again-loaded test makes them
    let (rows, columns, terms) = (432, 320, 320);
    let mut random = Random(595);
    let (a, bm, c0) = (
        random.matrix(rows, terms),
        random.matrix(terms, columns),
        random.matrix(rows, columns),
    );
    let alpha = c64::new(-0.7, 0.3);
    let multiply = |kernels: &dyn DenseKernels, threaded: bool| -> Vec<c64> {
        let mut c = c0.clone();
        kernels.multiply(
            BlockMut::of(&mut c, rows, columns),
            Block::of(&a, rows, terms),
            Block::of(&bm, terms, columns),
            Product::default(),
            alpha,
            threaded,
        );
        c
    };
    let want_product = multiply(&Faer, true);
    let check_product = |threaded: bool| -> Option<String> {
        let d = relative(&multiply(&openblas, threaded), &want_product);
        // a NaN difference is wrong too
        let right = d < 1e-12;
        (!right).then(|| format!("{d:.1e}"))
    };

    say(started, "the products' reference done; the threads start");

    let done = AtomicBool::new(false);
    let solve_until_done = |solver: &dyn DirectSolver, solves: &AtomicUsize| -> Vec<String> {
        let analysis = solver.analyse(&m).unwrap().unwrap();
        say(started, "a solver's analysis done");
        let mut wrong = Vec::new();
        // at least three, so that the run overlaps the products even if they are quick
        while !done.load(Ordering::Relaxed) || solves.load(Ordering::Relaxed) < 3 {
            let x = analysis.factorize(&m).unwrap().solve(&b).unwrap();
            let d = relative(&x, &want);
            let right = d < 1e-10;
            if !right {
                wrong.push(format!("{d:.1e}"));
            }
            solves.fetch_add(1, Ordering::Relaxed);
        }
        wrong
    };
    let (wrong_mumps, wrong_superlu, wrong_products, products) = std::thread::scope(|scope| {
        let mumps_thread = scope.spawn(|| solve_until_done(&mumps, &MUMPS_SOLVES));
        let superlu_thread = scope.spawn(|| solve_until_done(&superlu, &SUPERLU_SOLVES));
        let (mut wrong, mut products) = (Vec::new(), 0usize);
        for round in 0..rounds {
            for _ in 0..4 {
                products += 1;
                if let Some(d) = check_product(true) {
                    wrong.push(format!("round {round}, threaded: {d}"));
                }
            }
            let side_by_side: Vec<Option<String>> = (0..2 * rayon::current_num_threads())
                .into_par_iter()
                .map(|_| check_product(false))
                .collect();
            products += side_by_side.len();
            wrong.extend(
                side_by_side
                    .into_iter()
                    .flatten()
                    .map(|d| format!("round {round}, on one thread: {d}")),
            );
        }
        done.store(true, Ordering::Relaxed);
        (
            mumps_thread.join().unwrap(),
            superlu_thread.join().unwrap(),
            wrong,
            products,
        )
    });
    println!(
        "OpenBLAS {}: {products} products of {rounds} rounds, {} MUMPS and {} SuperLU solves \
         meanwhile; wrong: {} products, {} MUMPS, {} SuperLU",
        openblas.kernels().version,
        MUMPS_SOLVES.load(Ordering::Relaxed),
        SUPERLU_SOLVES.load(Ordering::Relaxed),
        wrong_products.len(),
        wrong_mumps.len(),
        wrong_superlu.len(),
    );
    FINISHED.store(true, Ordering::Relaxed);
    assert!(
        wrong_products.is_empty() && wrong_mumps.is_empty() && wrong_superlu.is_empty(),
        "products: {}; MUMPS: {}; SuperLU: {}",
        wrong_products.join("; "),
        wrong_mumps.join("; "),
        wrong_superlu.join("; ")
    );
}
