use std::sync::Arc;

use faer::sparse::{SparseColMat, SparseColMatRef, Triplet};
use num_complex::Complex64 as c64;

use super::{INNER_ITERATIONS, Refinement, STEPS, System, refine};
use crate::sparse::{Analysis, Multifrontal};

/// A shifted 7-point Laplacian on an n³ grid, complex symmetric, with its unknowns' positions:
/// −Δ − σ, its shift σ = `shift` + i `loss`.
fn helmholtz(n: usize, shift: f64, loss: f64) -> (SparseColMat<usize, c64>, Vec<[f64; 3]>) {
    let at = |i: usize, j: usize, k: usize| (k * n + j) * n + i;
    let mut t = Vec::new();
    let mut positions = vec![[0.0; 3]; n * n * n];
    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                let r = at(i, j, k);
                positions[r] = [i as f64, j as f64, k as f64];
                t.push(Triplet::new(r, r, c64::new(6.0 - shift, -loss)));
                for (di, dj, dk) in [(1, 0, 0), (0, 1, 0), (0, 0, 1)] {
                    let (i2, j2, k2) = (i + di, j + dj, k + dk);
                    if i2 < n && j2 < n && k2 < n {
                        let c = at(i2, j2, k2);
                        t.push(Triplet::new(r, c, c64::new(-1.0, 0.0)));
                        t.push(Triplet::new(c, r, c64::new(-1.0, 0.0)));
                    }
                }
            }
        }
    }
    (
        SparseColMat::try_new_from_triplets(n * n * n, n * n * n, &t).unwrap(),
        positions,
    )
}

/// GMRES-IR on `a` x = `b` with `a`'s single-precision factors, as photonoxide-mixed runs it.
fn mixed(
    a: SparseColMatRef<'_, usize, c64>,
    positions: &[[f64; 3]],
    symmetric: bool,
    b: &[c64],
) -> std::result::Result<(Vec<c64>, Refinement), Refinement> {
    let analysis = if symmetric {
        Analysis::new_symmetric(a, Some(positions))
            .unwrap()
            .unwrap()
    } else {
        Analysis::new(a, Some(positions)).unwrap()
    };
    let factors = Multifrontal::single(Arc::new(analysis), a).unwrap();
    let n = a.ncols();
    let product = |x: &[c64]| {
        let mut out = vec![c64::new(0.0, 0.0); n];
        for (j, xj) in x.iter().enumerate() {
            for (i, v) in a.row_idx_of_col(j).zip(a.val_of_col(j)) {
                out[i] += v * xj;
            }
        }
        out
    };
    let inverse = |v: &[c64]| factors.solve(v);
    let mut rows = vec![0.0f64; n];
    let mut entries = vec![0usize; n];
    for j in 0..n {
        for (i, v) in a.row_idx_of_col(j).zip(a.val_of_col(j)) {
            rows[i] += v.norm();
            entries[i] += 1;
        }
    }
    let system = System {
        product: &product,
        inverse: &inverse,
        norm: rows.iter().copied().fold(0.0, f64::max),
        row_entries: entries.into_iter().max().unwrap(),
    };
    refine(&system, b)
}

fn source(n: usize) -> Vec<c64> {
    (0..n)
        .map(|i| c64::new(((i * 7) % 11) as f64 - 5.0, ((i * 3) % 5) as f64))
        .collect()
}

fn relative(x: &[c64], y: &[c64]) -> f64 {
    let d: f64 = x.iter().zip(y).map(|(a, b)| (a - b).norm_sqr()).sum();
    let s: f64 = y.iter().map(|b| b.norm_sqr()).sum();
    (d / s).sqrt()
}

#[test]
fn single_precision_factors_refined_give_the_double_precision_solution() {
    // a lossy Helmholtz problem inside its spectrum, in both forms
    let (a, positions) = helmholtz(14, 2.0, 0.3);
    let n = a.ncols();
    let b = source(n);
    for symmetric in [false, true] {
        let analysis = if symmetric {
            Analysis::new_symmetric(a.as_ref(), Some(&positions))
                .unwrap()
                .unwrap()
        } else {
            Analysis::new(a.as_ref(), Some(&positions)).unwrap()
        };
        let double = Multifrontal::new(Arc::new(analysis), a.as_ref()).unwrap();
        let reference = double.solve(&b);
        let (x, history) = mixed(a.as_ref(), &positions, symmetric, &b).unwrap();
        println!("symmetric {symmetric}: {history:?}");
        assert!(history.converged);
        assert!(*history.backward_errors.last().unwrap() <= history.target);
        // each step at least halves the backward error, the first solve's single precision
        // to the target in a few steps
        for pair in history.backward_errors[..history.backward_errors.len() - 1].windows(2) {
            assert!(pair[1] <= 0.5 * pair[0], "{history:?}");
        }
        assert!(history.backward_errors[0] > 1e-12, "{history:?}");
        assert!(history.inner.len() <= 3, "{history:?}");
        assert!(
            relative(&x, &reference) < 1e-12,
            "{}",
            relative(&x, &reference)
        );
    }
}

#[test]
fn the_history_says_what_happened() {
    let history = Refinement {
        backward_errors: vec![1e-7, 3e-12, 1e-16],
        inner: vec![2, 3],
        target: 9e-16,
        converged: true,
    };
    let said = history.history();
    assert!(said.contains("1.0e-7, 3.0e-12, 1.0e-16"), "{said}");
    assert!(
        said.contains("2 refinement steps") && said.contains("2, 3"),
        "{said}"
    );
    let _ = (STEPS, INNER_ITERATIONS);
}
