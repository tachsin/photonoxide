//! photonoxide-mixed on 3D systems: the refinement's history where it converges and where it
//! can't, and the measurements of docs/methods/mixed-precision.md.

use std::sync::Arc;
use std::time::Instant;

use faer::sparse::{SparseColMat, Triplet};
use num_complex::Complex64 as c64;

use super::{ACCURATE, Axis, Boundaries3d, Edges, Grid3d, Lattice, Solver3d, positions};
use crate::backend::{Choice, MIXED};
use crate::fdfd::direct::Plan;
use crate::fdfd::krylov::Sparse;
use crate::sparse::refine::{INNER_ITERATIONS, Refinement, System, refine_observed};
use crate::sparse::{Analysis, Multifrontal};
use crate::units::Wavelength;

type Outcome = std::result::Result<(Vec<c64>, Refinement), Refinement>;
type Entries = [Triplet<usize, usize, c64>];

/// A closed box of `n`³ cells of 50 nm (walls all round), a silicon cube 240 nm across in its
/// middle, in oxide.
fn closed_box(n: usize) -> (Grid3d, Boundaries3d, impl Fn(f64, f64, f64) -> c64 + Copy) {
    let h = 0.05;
    let grid = Grid3d {
        nx: n,
        ny: n,
        nz: n,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
    };
    let wall = Edges::Pml { low: 0, high: 0 };
    let boundaries = Boundaries3d {
        x: wall,
        y: wall,
        z: wall,
        ..Boundaries3d::pml(0)
    };
    let middle = 0.5 * n as f64 * h;
    let eps = move |x: f64, y: f64, z: f64| {
        let inside = [x, y, z].iter().all(|v| (v - middle).abs() < 0.12);
        c64::new(if inside { 12.1 } else { 2.1 }, 0.0)
    };
    (grid, boundaries, eps)
}

/// The guide of `photonoxide bench`'s fdfd3d/guide-direct on an `n`³ grid of 10 nm: a silicon
/// guide n/4 cells square along x in vacuum, PMLs n/4 cells thick (n = 40 is that problem).
fn guide(n: usize) -> (Grid3d, Boundaries3d, impl Fn(f64, f64, f64) -> c64 + Copy) {
    let h = 0.01;
    let half = (n / 4) as f64 * h / 2.0;
    let grid = Grid3d {
        nx: n,
        ny: n,
        nz: n,
        dx: h,
        dy: h,
        dz: h,
        x0: -(n as f64) * h / 2.0,
        y0: -(n as f64) * h / 2.0,
        z0: -(n as f64) * h / 2.0,
    };
    let eps = move |_: f64, y: f64, z: f64| {
        c64::new(
            if y.abs() < half && z.abs() < half {
                12.09
            } else {
                1.0
            },
            0.0,
        )
    };
    (grid, Boundaries3d::pml(n / 4), eps)
}

/// A dipole along x three cells from the grid's middle, as the guide's benchmark has it, and
/// the right-hand side it makes, −i k₀ J (no value of it on a wall).
fn dipole(lattice: &Lattice) -> (Vec<c64>, Vec<c64>) {
    let g = lattice.grid;
    let mut current = vec![c64::new(0.0, 0.0); g.unknowns()];
    current[g.index(Axis::X, (g.nx / 2 + 3, g.ny / 2 + 3, g.nz / 2 + 3))] = c64::new(1.0, 0.0);
    let i_k0 = c64::new(0.0, -lattice.k0);
    let rhs = current.iter().map(|j| i_k0 * j).collect();
    (current, rhs)
}

/// A x, A the matrix of `entries`.
fn apply(entries: &Entries, x: &[c64]) -> Vec<c64> {
    let mut out = vec![c64::new(0.0, 0.0); x.len()];
    for t in entries {
        out[t.row] += t.val * x[t.col];
    }
    out
}

fn relative(x: &[c64], y: &[c64]) -> f64 {
    let d: f64 = x.iter().zip(y).map(|(a, b)| (a - b).norm_sqr()).sum();
    let s: f64 = y.iter().map(|b| b.norm_sqr()).sum();
    (d / s).sqrt()
}

/// ‖rhs − A x‖₂ / ‖rhs‖₂.
fn residual(entries: &Entries, x: &[c64], rhs: &[c64]) -> f64 {
    relative(&apply(entries, x), rhs)
}

/// The normwise backward error ‖rhs − A x‖∞ / (‖A‖∞ ‖x‖∞ + ‖rhs‖∞), as the refinement
/// measures it.
fn backward_error(entries: &Entries, x: &[c64], rhs: &[c64]) -> f64 {
    let largest = |v: &[c64]| v.iter().map(|z| z.norm()).fold(0.0, f64::max);
    let applied = apply(entries, x);
    let r: Vec<c64> = rhs.iter().zip(&applied).map(|(b, a)| b - a).collect();
    let mut rows = vec![0.0f64; x.len()];
    for t in entries {
        rows[t.row] += t.val.norm();
    }
    let norm = rows.iter().copied().fold(0.0, f64::max);
    largest(&r) / (norm * largest(x) + largest(rhs))
}

/// GMRES-IR as photonoxide-mixed runs it on the system of `entries` (on `lattice`) with
/// right-hand side `rhs`, as photonoxide's direct solve gives it the system (L D Lᵀ of the
/// complex symmetric similarity B = S A S⁻¹ when no pivot is perturbed, the LU of A
/// otherwise), each iterate of A x = rhs shown to `observe`: the outcome, the pivots the
/// single-precision factors perturbed, and the bytes they take.
fn mixed(
    lattice: &Lattice,
    entries: &Entries,
    rhs: &[c64],
    observe: &mut dyn FnMut(&[c64]),
) -> (Outcome, usize, usize) {
    let n = lattice.grid.unknowns();
    let positions = positions(&lattice.grid);
    let a = Sparse::new(n, entries.iter().map(|t| (t.row, t.col, t.val)));
    let symmetric = a.symmetrized().and_then(|(s, b)| {
        let t: Vec<Triplet<usize, usize, c64>> = (0..n)
            .flat_map(|r| b.row(r).map(move |(c, v)| Triplet::new(r, c, v)))
            .collect();
        let b = SparseColMat::try_new_from_triplets(n, n, &t).unwrap();
        let analysis = Analysis::new_symmetric(b.as_ref(), Some(&positions)).unwrap()?;
        let factors = Multifrontal::single(Arc::new(analysis), b.as_ref()).unwrap();
        (factors.perturbed() == 0).then_some((b, Some(s), factors))
    });
    let (matrix, s, factors) = symmetric.unwrap_or_else(|| {
        let a = SparseColMat::try_new_from_triplets(n, n, entries).unwrap();
        let analysis = Analysis::new(a.as_ref(), Some(&positions)).unwrap();
        let factors = Multifrontal::single(Arc::new(analysis), a.as_ref()).unwrap();
        (a, None, factors)
    });
    let product = |x: &[c64]| {
        let mut out = vec![c64::new(0.0, 0.0); n];
        for (j, xj) in x.iter().enumerate() {
            for (i, v) in matrix.row_idx_of_col(j).zip(matrix.val_of_col(j)) {
                out[i] += v * xj;
            }
        }
        out
    };
    let inverse = |v: &[c64]| factors.solve(v);
    let (mut rows, mut counts) = (vec![0.0f64; n], vec![0usize; n]);
    for j in 0..n {
        for (i, v) in matrix.row_idx_of_col(j).zip(matrix.val_of_col(j)) {
            rows[i] += v.norm();
            counts[i] += 1;
        }
    }
    let system = System {
        product: &product,
        inverse: &inverse,
        norm: rows.iter().copied().fold(0.0, f64::max),
        row_entries: counts.into_iter().max().unwrap(),
    };
    let undo = |y: &[c64]| -> Vec<c64> {
        match &s {
            Some(s) => y.iter().zip(s).map(|(y, s)| y / s).collect(),
            None => y.to_vec(),
        }
    };
    let rhs: Vec<c64> = match &s {
        Some(s) => rhs.iter().zip(s).map(|(b, s)| b * s).collect(),
        None => rhs.to_vec(),
    };
    let outcome = refine_observed(&system, &rhs, &mut |y| observe(&undo(y)))
        .map(|(y, history)| (undo(&y), history));
    (outcome, factors.perturbed(), factors.factor_bytes())
}

#[test]
fn single_precision_factors_refined_give_photonoxides_field() {
    let (grid, boundaries, eps) = closed_box(10);
    refined_as_photonoxides_own("closed box", grid, boundaries, eps);
    let (grid, boundaries, eps) = guide(16);
    refined_as_photonoxides_own("guide", grid, boundaries, eps);
}

/// The refinement's history on a problem single precision is enough for, and its field against
/// photonoxide's own.
fn refined_as_photonoxides_own(
    name: &str,
    grid: Grid3d,
    boundaries: Boundaries3d,
    eps: impl Fn(f64, f64, f64) -> c64 + Copy,
) {
    let wavelength = Wavelength::um(1.55).unwrap();
    let mixed_choice = Choice::Named(MIXED.into());
    {
        let own =
            Solver3d::new_on(grid, wavelength, eps, boundaries, &Choice::Photonoxide).unwrap();
        let (current, rhs) = dipole(&own.lattice);
        let reference = own.solve(&current).unwrap();
        // the refinement: the first solve single precision's, then each step at least halving
        // the backward error, to p u in a few steps, GMRES reaching its tolerance in each
        let (outcome, perturbed, _) = mixed(&own.lattice, &own.entries, &rhs, &mut |_| {});
        let (x, history) = outcome.unwrap();
        println!("{name}: {history:?}");
        assert!(
            history.converged,
            "{name}: {history:?} ({perturbed} pivots perturbed)"
        );
        assert!(history.backward_errors[0] > 1e-10, "{name}: {history:?}");
        for pair in history.backward_errors[..history.backward_errors.len() - 1].windows(2) {
            assert!(pair[1] <= 0.5 * pair[0], "{name}: {history:?}");
        }
        assert!(history.inner.len() <= 3, "{name}: {history:?}");
        assert!(
            history.inner.iter().all(|&i| i < INNER_ITERATIONS),
            "{name}: {history:?}"
        );
        assert!(relative(&x, reference.values()) < 1e-12, "{name}");
        // and through the backend, as a solver that names it has it
        let theirs = Solver3d::new_on(grid, wavelength, eps, boundaries, &mixed_choice).unwrap();
        assert!(theirs.direct_solver().starts_with(MIXED));
        let field = theirs.solve(&current).unwrap();
        let difference = relative(field.values(), reference.values());
        assert!(difference < 1e-12, "{name}: {difference:e}");
        assert!(residual(&own.entries, field.values(), &rhs) <= ACCURATE);
    }
}

#[test]
fn single_precision_factors_say_so_where_they_arent_enough() {
    // the closed box at 1 mm, 20 000 cells a wavelength: the curl-curl operator's gradients
    // see only k₀² ε, some 10⁻⁹ of its largest entries, below what single precision resolves
    // in the factors (u_f ‖B‖, 6 × 10⁻⁸ of it). Photonoxide's own double-precision factors
    // give a field at double precision's backward error
    let (grid, boundaries, eps) = closed_box(10);
    let wavelength = Wavelength::um(1000.0).unwrap();
    let own = Solver3d::new_on(grid, wavelength, eps, boundaries, &Choice::Photonoxide).unwrap();
    let (current, rhs) = dipole(&own.lattice);
    let reference = own.solve(&current).unwrap();
    let own_error = backward_error(&own.entries, reference.values(), &rhs);
    let (outcome, perturbed, _) = mixed(&own.lattice, &own.entries, &rhs, &mut |_| {});
    let history = outcome.expect_err("the refinement can't converge here");
    // photonoxide's own reaches the backward error the refinement aims at, p u
    assert!(own_error <= history.target, "{own_error:e} {history:?}");
    assert!(
        !history.converged,
        "{history:?} ({perturbed} pivots perturbed)"
    );
    let last = *history.backward_errors.last().unwrap();
    assert!(last > history.target, "{history:?}");
    // through the backend: an error with the history, never a field
    let theirs = Solver3d::new_on(
        grid,
        wavelength,
        eps,
        boundaries,
        &Choice::Named(MIXED.into()),
    )
    .unwrap();
    let e = theirs.solve(&current).err().unwrap().to_string();
    assert!(
        e.contains("didn't reach a double-precision solution")
            && e.contains("backward errors")
            && e.contains("GMRES taking")
            && e.contains("`photonoxide`"),
        "{e}"
    );
}

#[test]
#[ignore = "a measurement, for docs/methods/mixed-precision.md: cargo test --release --lib fdfd::three::mixed_tests::histories -- --ignored --nocapture"]
fn histories() {
    let (grid, boundaries, eps) = closed_box(12);
    println!(
        "| Wavelength | Perturbed pivots | Backward errors | GMRES iterations | Field against \
         photonoxide's own |"
    );
    println!("|---|---|---|---|---|");
    for wavelength in [1.55, 15.0, 100.0, 300.0, 1e3, 3e3] {
        let lam = Wavelength::um(wavelength).unwrap();
        let own = Solver3d::new_on(grid, lam, eps, boundaries, &Choice::Photonoxide).unwrap();
        let (current, rhs) = dipole(&own.lattice);
        let reference = own.solve(&current).unwrap();
        let (outcome, perturbed, _) = mixed(&own.lattice, &own.entries, &rhs, &mut |_| {});
        let (history, against) = match outcome {
            Ok((x, h)) => {
                let d = relative(&x, reference.values());
                (h, format!("{d:.1e}"))
            }
            Err(h) => (h, "failed, said so".into()),
        };
        let errors: Vec<String> = history
            .backward_errors
            .iter()
            .map(|e| format!("{e:.1e}"))
            .collect();
        let inner: Vec<String> = history.inner.iter().map(usize::to_string).collect();
        println!(
            "| {wavelength} µm | {perturbed} | {} | {} | {against} |",
            errors.join(", "),
            inner.join(", ")
        );
    }
}

/// Reads `MP_CELLS` (the guide's n, 24 by default), `MP_MODE` (`double` or `mixed`) and
/// `MP_REFERENCE` (a file: the double run writes its field there, the mixed run reads it).
/// Each mode in a process of its own, whose peak memory is read from outside.
#[test]
#[ignore = "a measurement, one mode per process, for docs/methods/mixed-precision.md: MP_CELLS=40 MP_MODE=double MP_REFERENCE=<file> cargo test --release --lib fdfd::three::mixed_tests::benchmark -- --ignored --nocapture, then MP_MODE=mixed"]
fn benchmark() {
    let var = |name: &str, default: &str| std::env::var(name).unwrap_or(default.into());
    let n: usize = var("MP_CELLS", "24").parse().unwrap();
    let mode = var("MP_MODE", "double");
    let reference_file = var("MP_REFERENCE", "mp-reference.bin");
    let (grid, boundaries, eps) = guide(n);
    let wavelength = Wavelength::um(1.55).unwrap();
    let threads = rayon::current_num_threads();
    let start = Instant::now();
    // the averaged permittivity and the matrix, untimed below
    let (lattice, values) = Solver3d::setup(grid, wavelength, eps, boundaries).unwrap();
    let entries = lattice.assemble(&values);
    println!(
        "{n}³ guide, {} unknowns, {threads} threads, {mode}: assembly {:.2} s",
        grid.unknowns(),
        start.elapsed().as_secs_f64()
    );
    let (current, rhs) = dipole(&lattice);
    if mode == "double" {
        // photonoxide's own: the solver's factorization and its solve, refined once
        let clock = Instant::now();
        let lu =
            super::factorize(&grid, &entries, Plan::new(&Choice::Photonoxide).unwrap()).unwrap();
        let factorized = clock.elapsed().as_secs_f64();
        let solver = Solver3d {
            lattice,
            eps: values,
            entries,
            lu,
        };
        let clock = Instant::now();
        let field = solver.solve(&current).unwrap();
        let solved = clock.elapsed().as_secs_f64();
        let bytes: Vec<u8> = field
            .values()
            .iter()
            .flat_map(|z| [z.re.to_le_bytes(), z.im.to_le_bytes()].concat())
            .collect();
        std::fs::write(&reference_file, bytes).unwrap();
        println!(
            "double: analysis and factorization {factorized:.2} s, solve and its step of \
             refinement {solved:.2} s, total {:.2} s; factor entries {:?}, L D Lᵀ {}; relative \
             residual {:.1e}",
            factorized + solved,
            solver.factor_entries(),
            solver.symmetric_factors(),
            residual(&solver.entries, field.values(), &rhs)
        );
    } else {
        let raw = std::fs::read(&reference_file).expect("run MP_MODE=double first");
        let reference: Vec<c64> = raw
            .as_chunks::<16>()
            .0
            .iter()
            .map(|b| {
                c64::new(
                    f64::from_le_bytes(b[..8].try_into().unwrap()),
                    f64::from_le_bytes(b[8..].try_into().unwrap()),
                )
            })
            .collect();
        // photonoxide-mixed's steps: the analysis, the single-precision factorization, the
        // refinement, each iterate's time and field error against the double solve's
        let clock = Instant::now();
        let mut iterates: Vec<(f64, f64)> = Vec::new();
        let (outcome, perturbed, bytes) = mixed(&lattice, &entries, &rhs, &mut |x| {
            iterates.push((clock.elapsed().as_secs_f64(), relative(x, &reference)));
        });
        let total = clock.elapsed().as_secs_f64();
        let (x, history) = match outcome {
            Ok((x, h)) => (Some(x), h),
            Err(h) => (None, h),
        };
        println!(
            "mixed: factors {:.3} GB, {perturbed} pivots perturbed, total {total:.2} s, relative \
             residual {:?}, {}",
            bytes as f64 / 1e9,
            x.map(|x| residual(&entries, &x, &rhs)),
            history.history()
        );
        for (k, (t, e)) in iterates.iter().enumerate() {
            println!("  iterate {k}: {t:.2} s, field error {e:.1e}");
        }
    }
}
