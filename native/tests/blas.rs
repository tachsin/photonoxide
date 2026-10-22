//! Each BLAS and LAPACK library found here as the multifrontal fronts' dense kernels: its
//! kernels against faer's, each interface it has (Fortran's and C's, 32-bit and 64-bit
//! integers), photonoxide's solver with them against its own on Helmholtz matrices and on the
//! exported systems (`photonoxide bench --export`), and whether they repeat their bits. A
//! library that isn't here is skipped, and said so.
//!
//! - `PHOTONOXIDE_REQUIRE_BLAS` names the providers that must be found (`openblas,mkl`, as a
//!   workflow that installs them sets it).
//! - `PHOTONOXIDE_BLAS_LARGE` adds the largest sizes: n = 4096 for the kernels, `strip-32` for
//!   the exported systems.
//! - `PHOTONOXIDE_BLAS_ALSO` names more files to check as OpenBLAS, separated as `PATH` is (a
//!   second build: pip's `scipy-openblas64`, say).

use std::path::PathBuf;
use std::sync::Arc;

use num_complex::Complex64 as c64;
use photonoxide::backend::dense::{Block, BlockMut, DenseKernels, Faer, Product, with_kernels};
use photonoxide::backend::{self, Choice, DirectSolver, Form, Matrix};
use photonoxide::bench::catalogue::entry;
use photonoxide::bench::export;
use photonoxide_native::blas::{Integers, Provider, Style, Want};
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

fn large() -> bool {
    std::env::var_os("PHOTONOXIDE_BLAS_LARGE").is_some()
}

/// Every kernel of `library` against faer's at these sizes, threaded and not: the largest
/// difference.
fn check_kernels(library: &dyn DenseKernels, sizes: &[usize]) -> f64 {
    let name = library.kernels().name;
    let mut worst: f64 = 0.0;
    for &n in sizes {
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
                for (kernels, c) in [
                    (&Faer as &dyn DenseKernels, &mut ours),
                    (library, &mut theirs),
                ] {
                    kernels.multiply(
                        BlockMut::of(c, m, n),
                        Block::of(&a, m, r),
                        Block::of(&b, br, bc),
                        product,
                        alpha,
                        threaded,
                    );
                }
                let d = relative(&theirs, &ours);
                assert!(d < 1e-12, "{name} multiply n {n} {product:?}: {d:e}");
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
            for (kernels, c) in [
                (&Faer as &dyn DenseKernels, &mut ours),
                (library, &mut theirs),
            ] {
                kernels.multiply(
                    BlockMut::of(c, n, n),
                    Block::of(&a, n, r),
                    Block::of(&b, n, r),
                    product,
                    alpha,
                    threaded,
                );
            }
            let lower = |c: &[c64]| -> Vec<c64> {
                (0..n)
                    .flat_map(|j| (j..n).map(move |i| (i, j)))
                    .map(|(i, j)| c[i + j * n])
                    .collect()
            };
            let d = relative(&lower(&theirs), &lower(&ours));
            assert!(d < 1e-12, "{name} lower product n {n}: {d:e}");
            worst = worst.max(d);
            // the three triangular solves, on right-hand sides that aren't square
            let t = random.triangle(n);
            let solves: [(&str, usize, usize); 3] =
                [("L⁻¹ B", n, r), ("B U⁻¹", r, n), ("B L⁻ᵀ", r, n)];
            for (k, (what, rows, columns)) in solves.into_iter().enumerate() {
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
                run(library, &mut theirs);
                let d = relative(&theirs, &ours);
                assert!(d < 1e-11, "{name} {what} n {n}: {d:e}");
                worst = worst.max(d);
            }
            // the LU, by what it factorizes: rows of A in the order returned are L U
            let a = random.matrix(n, n);
            let mut factors = a.clone();
            let rows = library.lu(BlockMut::of(&mut factors, n, n), threaded);
            let mut sorted = rows.clone();
            sorted.sort_unstable();
            assert!(sorted.iter().copied().eq(0..n), "{name}: not an order");
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
            assert!(d < 1e-11, "{name} LU n {n}: {d:e}");
            // partial pivoting: LAPACK takes the entry largest in |re| + |im|, which leaves no
            // entry of L larger than that of 1, and so none beyond √2 in modulus
            let largest = l.iter().map(|v| v.norm()).fold(0.0, f64::max);
            assert!(
                largest <= 2f64.sqrt() + 1e-12,
                "{name} LU n {n}: |L| to {largest}"
            );
            worst = worst.max(d);
        }
    }
    worst
}

#[test]
fn the_kernels_are_faers_to_round_off() {
    let mut sizes = vec![32, 100, 257, 1024];
    if large() {
        sizes.push(4096);
    }
    for library in libraries() {
        let what = library.kernels();
        println!(
            "{} {}: {}; {:?}; zgemmt: {}",
            what.name,
            what.version,
            library.interface(),
            what.threads,
            library.has_triangular_product()
        );
        let worst = check_kernels(library.as_ref(), &sizes);
        println!(
            "{}: the largest difference from faer's, {worst:.1e}",
            what.name
        );
    }
}

/// Every interface a library has, each found by its own names and its integers measured, and
/// each checked as the one found first is.
#[test]
fn each_interface_a_library_has_is_found_and_agrees() {
    let mut settings: Vec<(Provider, Option<PathBuf>)> = Provider::ALL
        .into_iter()
        .filter(|p| p.here())
        .map(|p| (p, None))
        .collect();
    if let Some(also) = std::env::var_os("PHOTONOXIDE_BLAS_ALSO") {
        settings.extend(std::env::split_paths(&also).map(|p| (Provider::OpenBlas, Some(p))));
    }
    for (provider, setting) in settings {
        let mut found = 0;
        for style in [Style::Fortran, Style::C] {
            for integers in [Integers::Lp64, Integers::Ilp64] {
                let want = Want::new(Some(style), Some(integers));
                let blas = match Blas::find(provider, setting.as_deref(), want) {
                    Ok(blas) => blas,
                    Err(why) => {
                        println!("skipped: {provider:?} {style:?} {integers:?}: {why}");
                        continue;
                    }
                };
                let i = blas.interface().clone();
                // (another candidate than the setting's, if that one hasn't this interface)
                let files: Vec<String> = blas
                    .files()
                    .iter()
                    .map(|f| f.display().to_string())
                    .collect();
                assert_eq!((i.style, i.integers), (style, integers));
                let worst = check_kernels(&blas, &[32, 257]);
                let solver = with_kernels(Arc::new(blas));
                let difference = smoke_test(solver.as_ref()).unwrap();
                assert!(difference < 1e-12, "{provider:?} {want:?}: {difference:e}");
                println!(
                    "{provider:?} {}: {i}; kernels within {worst:.1e} of faer's, the smoke \
                     test within {difference:.1e}",
                    files.join(", ")
                );
                found += 1;
            }
        }
        println!("{provider:?}: {found} interfaces");
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

/// A system of `photonoxide bench --export` by columns, with its unknowns' places.
struct Exported {
    n: usize,
    starts: Vec<usize>,
    rows: Vec<usize>,
    values: Vec<c64>,
    rhs: Vec<c64>,
    entries: Vec<(usize, usize, c64)>,
    positions: Option<Vec<[f64; 3]>>,
}

fn exported(id: &str, form: Form) -> Exported {
    let system = export::system(id).unwrap();
    let positions = export::positions(&system);
    let system = match form {
        Form::General => system,
        // its complex symmetric form, B = S A S⁻¹, as the solvers factorize it
        _ => {
            let zero = vec![c64::new(0.0, 0.0); system.n];
            export::symmetric(&system, &zero).unwrap().0
        }
    };
    let n = system.n;
    let mut entries = system.entries.clone();
    entries.sort_by_key(|&(r, c, _)| (c, r));
    let mut starts = vec![0; n + 1];
    for &(_, c, _) in &entries {
        starts[c + 1] += 1;
    }
    for j in 0..n {
        starts[j + 1] += starts[j];
    }
    Exported {
        n,
        starts,
        rows: entries.iter().map(|e| e.0).collect(),
        values: entries.iter().map(|e| e.2).collect(),
        rhs: system.rhs.clone(),
        entries,
        positions,
    }
}

/// x, ‖b − A x‖ / ‖b‖, the factors' entries, the pivots perturbed and the factorization's
/// seconds of `solver` on an exported system.
fn solve_exported(
    solver: &dyn DirectSolver,
    e: &Exported,
    form: Form,
) -> (Vec<c64>, f64, Option<usize>, usize, f64) {
    let mut m = Matrix::new(e.n, &e.starts, &e.rows, &e.values, form).unwrap();
    if let Some(p) = &e.positions {
        m = m.with_positions(p).unwrap();
    }
    let f = solver.analyse(&m).unwrap().unwrap().factorize(&m).unwrap();
    let x = f.solve(&e.rhs).unwrap();
    let mut r = e.rhs.clone();
    for &(i, j, v) in &e.entries {
        r[i] -= v * x[j];
    }
    let norm = |v: &[c64]| v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
    let report = f.report();
    (
        x,
        norm(&r) / norm(&e.rhs),
        report.factor_entries,
        report.perturbed_pivots,
        report.factorization_seconds,
    )
}

/// The exported systems (`photonoxide bench --export`), as LU and as L D Lᵀ: photonoxide's
/// solver with each library's kernels against its own, the same factors to round-off (the same
/// fill, the same pivots perturbed, the solutions within 1e-10 of each other).
#[test]
fn on_the_exported_systems_the_factors_are_the_defaults_to_round_off() {
    let libraries = libraries();
    if libraries.is_empty() {
        println!("skipped: no BLAS and LAPACK library here");
        return;
    }
    let own = backend::direct(&Choice::Photonoxide).unwrap();
    let mut ids = vec!["slab-2d", "strip-24"];
    if large() {
        ids.push("strip-32");
    }
    for id in ids {
        for form in [Form::General, Form::Symmetric] {
            let e = exported(id, form);
            let ours = solve_exported(own.as_ref(), &e, form);
            println!(
                "{id} {form:?} ({} unknowns): photonoxide's own, residual {:.1e}, {:?} entries, \
                 {:.3} s",
                e.n, ours.1, ours.2, ours.4
            );
            for library in &libraries {
                let solver = with_kernels(library.clone());
                let theirs = solve_exported(solver.as_ref(), &e, form);
                let d = relative(&theirs.0, &ours.0);
                println!(
                    "{id} {form:?}: {} within {d:.1e} of it, residual {:.1e}, {:.3} s",
                    solver.capabilities().name,
                    theirs.1,
                    theirs.4
                );
                assert!(d < 1e-10, "{id} {form:?}: {d:e}");
                assert!(theirs.1 < 1e-11, "{id} {form:?}: residual {:e}", theirs.1);
                assert_eq!(theirs.2, ours.2, "the fill");
                assert_eq!(theirs.3, ours.3, "the pivots perturbed");
            }
        }
    }
}

/// Whether each library's kernels repeat their bits: photonoxide's solver with them on the
/// same matrix, twice on all of rayon's threads, and on one and two threads. Printed, not
/// judged, but for faer's, which must repeat everywhere.
#[test]
fn whether_the_kernels_repeat_their_bits() {
    let all_threads = std::thread::available_parallelism().map_or(2, |n| n.get());
    let mut solvers: Vec<Arc<dyn DirectSolver>> = vec![with_kernels(Arc::new(Faer))];
    for library in libraries() {
        solvers.push(with_kernels(library));
    }
    for solver in solvers {
        let name = solver.capabilities().name;
        for form in [Form::General, Form::Symmetric] {
            let run = |threads: usize| -> Vec<c64> {
                let pool = rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .unwrap();
                pool.install(|| solve(solver.as_ref(), [24, 24, 24], form).0)
            };
            let first = run(all_threads);
            let again = run(all_threads);
            let one = run(1);
            let one_again = run(1);
            let two = run(2);
            let same = |a: &[c64], b: &[c64]| a == b;
            println!(
                "{name} {form:?}: the same bits twice on {all_threads} threads: {}, twice on 1: \
                 {}; on 1 thread as on {all_threads}: {} ({:.1e}); on 2: {} ({:.1e})",
                same(&first, &again),
                same(&one, &one_again),
                same(&first, &one),
                relative(&one, &first),
                same(&first, &two),
                relative(&two, &first),
            );
            if name == "photonoxide-faer" {
                assert!(same(&first, &again) && same(&first, &one) && same(&first, &two));
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
        // and the benchmark's dense kernels, by the same name
        for id in ["dense/lu-256", "dense/product-256"] {
            let e = entry(id).unwrap();
            assert!(e.takes_dense_kernels());
            let theirs = e.run(&Choice::Named(name.clone())).unwrap();
            let error = theirs.accuracy.as_ref().unwrap().error;
            assert!(error <= e.tolerance(), "{id} with {name}: {error:e}");
        }
    }
}
