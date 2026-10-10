//! Each BLAS and LAPACK library found here as the multifrontal fronts' dense kernels: its
//! kernels against faer's, and photonoxide's solver with them against its own. A library that
//! isn't here is skipped, and said so; `PHOTONOXIDE_REQUIRE_BLAS` names those that must be
//! (`openblas,mkl`, as a workflow that installs them sets it), and `PHOTONOXIDE_BLAS_LARGE`
//! adds the largest size.

use std::sync::Arc;
use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::backend::dense::{Block, BlockMut, DenseKernels, Faer, Product, with_kernels};
use photonoxide::backend::{self, Choice, DirectSolver, Form, Matrix};
use photonoxide::bench::catalogue::entry;
use photonoxide_native::{Blas, blas, offer, smoke_test};

/// The libraries found, those required among them.
fn libraries() -> Vec<Arc<Blas>> {
    let required = std::env::var("PHOTONOXIDE_REQUIRE_BLAS").unwrap_or_default();
    let mut found = Vec::new();
    for (name, kernels) in blas::all() {
        match kernels {
            Ok(kernels) => found.push(Arc::new(kernels)),
            Err(reason) => {
                assert!(
                    !required.split(',').any(|r| r.trim() == name),
                    "{name} is required here and isn't found: {reason}"
                );
                println!("skipped: {name} isn't here: {reason}");
            }
        }
    }
    found
}

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

    /// A square matrix whose triangles are well conditioned: a large diagonal.
    fn triangle(&mut self, n: usize) -> Vec<c64> {
        let mut t = self.matrix(n, n);
        for i in 0..n {
            t[i + i * n] += c64::new(n as f64, 0.0);
        }
        t
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

fn sizes() -> Vec<usize> {
    let mut sizes = vec![32, 100, 257, 1024];
    if std::env::var_os("PHOTONOXIDE_BLAS_LARGE").is_some() {
        sizes.push(4096);
    }
    sizes
}

#[test]
fn the_kernels_are_faers_to_round_off() {
    for library in libraries() {
        let what = library.kernels();
        println!(
            "{} {} ({:?}), zgemmt: {}",
            what.name,
            what.version,
            what.threads,
            library.has_triangular_product()
        );
        let mut worst: f64 = 0.0;
        for n in sizes() {
            // not square, so that a dimension taken for another shows
            let (m, r) = (n + n / 3, n / 2 + 1);
            let mut random = Random(n as u64);
            for threaded in [false, true] {
                // C + α A B, and with Bᵀ
                let a = random.matrix(m, r);
                let alpha = c64::new(-0.7, 0.3);
                for transposed in [false, true] {
                    let b = if transposed {
                        random.matrix(n, r)
                    } else {
                        random.matrix(r, n)
                    };
                    let (br, bc) = if transposed { (n, r) } else { (r, n) };
                    let product = Product {
                        transposed,
                        lower: false,
                    };
                    let c0 = random.matrix(m, n);
                    let (mut ours, mut theirs) = (c0.clone(), c0);
                    Faer.multiply(
                        BlockMut::of(&mut ours, m, n),
                        Block::of(&a, m, r),
                        Block::of(&b, br, bc),
                        product,
                        alpha,
                        threaded,
                    );
                    library.multiply(
                        BlockMut::of(&mut theirs, m, n),
                        Block::of(&a, m, r),
                        Block::of(&b, br, bc),
                        product,
                        alpha,
                        threaded,
                    );
                    let d = relative(&theirs, &ours);
                    assert!(d < 1e-12, "{} multiply n {n} {product:?}: {d:e}", what.name);
                    worst = worst.max(d);
                }
                // a square C's lower triangle, C + α A Bᵀ
                let (a, b) = (random.matrix(n, r), random.matrix(n, r));
                let c0 = random.matrix(n, n);
                let (mut ours, mut theirs) = (c0.clone(), c0);
                let product = Product {
                    transposed: true,
                    lower: true,
                };
                Faer.multiply(
                    BlockMut::of(&mut ours, n, n),
                    Block::of(&a, n, r),
                    Block::of(&b, n, r),
                    product,
                    alpha,
                    threaded,
                );
                library.multiply(
                    BlockMut::of(&mut theirs, n, n),
                    Block::of(&a, n, r),
                    Block::of(&b, n, r),
                    product,
                    alpha,
                    threaded,
                );
                let lower = |c: &[c64]| -> Vec<c64> {
                    (0..n)
                        .flat_map(|j| (j..n).map(move |i| (i, j)))
                        .map(|(i, j)| c[i + j * n])
                        .collect()
                };
                let d = relative(&lower(&theirs), &lower(&ours));
                assert!(d < 1e-12, "{} lower product n {n}: {d:e}", what.name);
                worst = worst.max(d);
                // the three triangular solves, on right-hand sides that aren't square
                let t = random.triangle(n);
                let solves: [(&str, usize, usize); 3] =
                    [("L⁻¹ B", n, r), ("B U⁻¹", r, n), ("B L⁻ᵀ", r, n)];
                for (k, (name, rows, columns)) in solves.into_iter().enumerate() {
                    let b0 = random.matrix(rows, columns);
                    let (mut ours, mut theirs) = (b0.clone(), b0);
                    let run = |kernels: &dyn DenseKernels, b: &mut [c64]| {
                        let (l, b) = (Block::of(&t, n, n), BlockMut::of(b, rows, columns));
                        match k {
                            0 => kernels.solve_unit_lower(l, b, threaded),
                            1 => kernels.solve_upper_from_right(l, b, threaded),
                            _ => kernels.solve_unit_lower_transposed_from_right(l, b, threaded),
                        }
                    };
                    run(&Faer, &mut ours);
                    run(library.as_ref(), &mut theirs);
                    let d = relative(&theirs, &ours);
                    assert!(d < 1e-11, "{} {name} n {n}: {d:e}", what.name);
                    worst = worst.max(d);
                }
                // the LU, by what it factorizes: rows of A in the order returned are L U
                let a = random.matrix(n, n);
                let mut factors = a.clone();
                let rows = library.lu(BlockMut::of(&mut factors, n, n), threaded);
                let mut sorted = rows.clone();
                sorted.sort_unstable();
                assert!(
                    sorted.iter().copied().eq(0..n),
                    "{}: not an order",
                    what.name
                );
                let (mut l, mut u) = (
                    vec![c64::new(0.0, 0.0); n * n],
                    vec![c64::new(0.0, 0.0); n * n],
                );
                for j in 0..n {
                    for i in 0..n {
                        let v = factors[i + j * n];
                        if i > j {
                            l[i + j * n] = v;
                        } else {
                            u[i + j * n] = v;
                        }
                    }
                    l[j + j * n] = c64::new(1.0, 0.0);
                }
                let mut product = vec![c64::new(0.0, 0.0); n * n];
                Faer.multiply(
                    BlockMut::of(&mut product, n, n),
                    Block::of(&l, n, n),
                    Block::of(&u, n, n),
                    Product::default(),
                    c64::new(1.0, 0.0),
                    true,
                );
                let permuted: Vec<c64> = (0..n * n).map(|e| a[rows[e % n] + (e / n) * n]).collect();
                let d = relative(&product, &permuted);
                assert!(d < 1e-11, "{} LU n {n}: {d:e}", what.name);
                // partial pivoting: LAPACK takes the entry largest in |re| + |im|, which
                // leaves no entry of L larger than that of 1, and so none beyond √2 in modulus
                let largest = l.iter().map(|v| v.norm()).fold(0.0, f64::max);
                assert!(
                    largest <= 2f64.sqrt() + 1e-12,
                    "{} LU n {n}: |L| to {largest}",
                    what.name
                );
                worst = worst.max(d);
            }
        }
        println!(
            "{}: the largest difference from faer's, {worst:.1e}",
            what.name
        );
    }
}

/// A Helmholtz operator with loss on a grid: complex symmetric, or general with an asymmetric
/// coupling along x.
fn helmholtz(cells: [usize; 3], form: Form) -> (Vec<usize>, Vec<usize>, Vec<c64>) {
    let [mx, my, mz] = cells;
    let n = mx * my * mz;
    let strides = [1, mx, mx * my];
    let (mut starts, mut rows, mut values) = (vec![0], Vec::new(), Vec::new());
    for j in 0..n {
        let at = [j % mx, j / mx % my, j / (mx * my)];
        let mut column: Vec<(usize, c64)> = Vec::new();
        for axis in 0..3 {
            if at[axis] > 0 {
                column.push((j - strides[axis], c64::new(1.0, 0.0)));
            }
            if at[axis] + 1 < cells[axis] {
                let v = if form == Form::General && axis == 0 {
                    c64::new(1.0, 0.1)
                } else {
                    c64::new(1.0, 0.0)
                };
                column.push((j + strides[axis], v));
            }
        }
        column.push((j, c64::new(-5.7, 0.02 + 0.001 * (j % 11) as f64)));
        column.sort_by_key(|&(r, _)| r);
        for (r, v) in column {
            rows.push(r);
            values.push(v);
        }
        starts.push(rows.len());
    }
    (starts, rows, values)
}

/// A solve, its transposed one, the pivots perturbed and the factorization's seconds.
fn solve(
    solver: &dyn DirectSolver,
    cells: [usize; 3],
    form: Form,
) -> (Vec<c64>, Vec<c64>, usize, f64) {
    let (s, r, v) = helmholtz(cells, form);
    let m = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
    let b: Vec<c64> = (0..m.n())
        .map(|k| c64::new(1.0, k as f64 * 1e-3).powi((k % 3) as i32))
        .collect();
    let f = solver.analyse(&m).unwrap().unwrap().factorize(&m).unwrap();
    let report = f.report();
    (
        f.solve(&b).unwrap(),
        f.solve_transpose(&b).unwrap(),
        report.perturbed_pivots,
        report.factorization_seconds,
    )
}

#[test]
fn photonoxides_solver_with_the_kernels_is_its_own_to_round_off() {
    let own = backend::direct(&Choice::Photonoxide).unwrap();
    for library in libraries() {
        let solver = with_kernels(library.clone());
        let c = solver.capabilities();
        println!("{c:?}");
        assert_eq!(c.name, format!("photonoxide-{}", library.kernels().name));
        assert!(c.symmetric && c.transpose && !c.deterministic);
        let difference = smoke_test(solver.as_ref()).unwrap();
        println!(
            "{}: the smoke test's largest difference, {difference:.1e}",
            c.name
        );
        assert!(difference < 1e-12);
        // the last has fronts large enough to thread (a separator of 576 unknowns)
        for cells in [[60, 60, 1], [14, 12, 10], [24, 24, 24]] {
            for form in [Form::General, Form::Symmetric] {
                let ours = solve(own.as_ref(), cells, form);
                let theirs = solve(solver.as_ref(), cells, form);
                let (plain, transposed) =
                    (relative(&theirs.0, &ours.0), relative(&theirs.1, &ours.1));
                println!(
                    "{} {cells:?} {form:?}: {plain:.1e} and {transposed:.1e} (transposed) from \
                     photonoxide's own; factorized in {:.3} s, its own in {:.3} s",
                    c.name, theirs.3, ours.3
                );
                assert!(
                    plain < 1e-10 && transposed < 1e-10,
                    "{} {cells:?} {form:?}",
                    c.name
                );
                assert_eq!(theirs.2, ours.2, "the pivots perturbed");
            }
        }
    }
}

/// FDFD's systems from the benchmark's catalogue, each checked by its own accuracy check, with
/// each library's kernels chosen by the solver's name.
#[test]
fn it_solves_fdfd_systems_to_their_checks() {
    for library in libraries() {
        let name = format!("photonoxide-{}", library.kernels().name);
        assert!(offer(with_kernels(library)).unwrap());
        for id in [
            "fdfd2d/slab-ez-100",
            "fdfd2d/ring-hz-100",
            "fdfd2d/slab-ez-bloch-100",
            "fdfd3d/strip-16",
            "fdfd3d/grating-16",
            "fdfd3d/strip-stretched-16",
        ] {
            let e = entry(id).unwrap();
            let ours = e.run(&Choice::Photonoxide).unwrap();
            let theirs = e.run(&Choice::Named(name.clone())).unwrap();
            let (a, b) = (
                ours.accuracy.as_ref().unwrap().error,
                theirs.accuracy.as_ref().unwrap().error,
            );
            println!(
                "{id} ({} unknowns): photonoxide {a:.1e} in {:.3} s, {name} {b:.1e} in {:.3} s",
                ours.unknowns,
                ours.seconds(),
                theirs.seconds()
            );
            assert!(b <= e.tolerance(), "{id}: {b:e}");
        }
    }
}

/// The product's and the LU's rates, each library's beside faer's, on one thread and on
/// rayon's: printed, not judged (a shared machine's rates say little).
#[test]
fn the_kernels_rates_are_printed() {
    let n = 1024;
    let mut random = Random(7);
    let (a, b) = (random.matrix(n, n), random.matrix(n, n));
    let mut all: Vec<(String, Arc<dyn DenseKernels>)> = vec![("faer".into(), Arc::new(Faer))];
    for library in libraries() {
        all.push((library.kernels().name, library));
    }
    for (name, kernels) in all {
        for threaded in [false, true] {
            let mut c = vec![c64::new(0.0, 0.0); n * n];
            let clock = Instant::now();
            kernels.multiply(
                BlockMut::of(&mut c, n, n),
                Block::of(&a, n, n),
                Block::of(&b, n, n),
                Product::default(),
                c64::new(1.0, 0.0),
                threaded,
            );
            let product = clock.elapsed().as_secs_f64();
            let mut f = a.clone();
            let clock = Instant::now();
            let _ = kernels.lu(BlockMut::of(&mut f, n, n), threaded);
            let lu = clock.elapsed().as_secs_f64();
            // a complex multiply-add is 8 real operations: n³ of them, and n³/3 for the LU
            let flops = 8.0 * (n as f64).powi(3);
            println!(
                "{name}, n {n}, {} ({} of rayon's): product {:.1} Gflop/s, LU {:.1} Gflop/s",
                if threaded { "threaded" } else { "one thread" },
                rayon::current_num_threads(),
                flops / product / 1e9,
                flops / 3.0 / lu / 1e9
            );
        }
    }
}
