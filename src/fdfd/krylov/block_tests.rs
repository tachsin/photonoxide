use super::*;
use crate::fdfd::krylov::{Sparse, qmr, qmr_symmetric};

fn stop(tolerance: f64, max_iterations: usize) -> Stopping {
    Stopping {
        tolerance,
        max_iterations,
    }
}

/// A complex, nonsymmetric, diagonally dominant matrix of n rows with a few entries off the
/// diagonal.
fn nonsymmetric(n: usize) -> Sparse {
    let mut entries = Vec::new();
    for r in 0..n {
        entries.push((r, r, c64::new(2.2 + 0.1 * (r % 7) as f64, 0.3)));
        entries.push((r, (r + 1) % n, c64::new(-1.0, 0.3)));
        entries.push((r, (r + n - 1) % n, c64::new(-0.7, -0.2)));
        entries.push((r, (r * 7 + 3) % n, c64::new(0.4, 0.25)));
    }
    Sparse::new(n, entries)
}

/// A complex symmetric matrix (not Hermitian), indefinite: a ring's Laplacian shifted past
/// its lowest eigenvalues, a little loss, and a few long-range couplings.
fn complex_symmetric(n: usize) -> Sparse {
    let mut entries = Vec::new();
    for r in 0..n {
        entries.push((r, r, c64::new(1.2 + 0.1 * (r % 5) as f64, 0.05)));
        let c = (r + 1) % n;
        let v = c64::new(-1.0, 0.02 * (r % 3) as f64);
        entries.push((r, c, v));
        entries.push((c, r, v));
        if r % 4 == 0 {
            let c = (r * 13 + 7) % n;
            if c != r {
                let v = c64::new(0.3, -0.05);
                entries.push((r, c, v));
                entries.push((c, r, v));
            }
        }
    }
    Sparse::new(n, entries)
}

/// The k-th of a set of unrelated right-hand sides of n values, by the portable sine and cosine:
/// the system's differ in the last bit between Linux, Windows and macOS, and a solve that needs
/// nearly n iterations turns that into a different count (docs/methods/conventions.md).
fn rhs(n: usize, k: usize) -> Vec<c64> {
    (0..n)
        .map(|r| {
            let t = (r * (k + 3) + 7 * k) as f64;
            let sin = crate::portable::sin_cos(0.37 * t).0;
            let cos = crate::portable::sin_cos(0.11 * t + k as f64).1;
            c64::new(sin, 0.3 * cos)
        })
        .collect()
}

fn relative_residual(a: &Sparse, b: &[c64], x: &[c64]) -> f64 {
    let ax = a.apply(x);
    let r: Vec<c64> = b.iter().zip(&ax).map(|(p, q)| p - q).collect();
    norm(&r) / norm(b)
}

fn largest_difference(x: &[c64], y: &[c64]) -> f64 {
    let largest = y.iter().map(|z| z.norm()).fold(0.0, f64::max);
    x.iter()
        .zip(y)
        .map(|(p, q)| (p - q).norm())
        .fold(0.0, f64::max)
        / largest
}

fn bits(v: &[c64]) -> Vec<(u64, u64)> {
    v.iter().map(|z| (z.re.to_bits(), z.im.to_bits())).collect()
}

#[test]
fn a_diagonal_system_is_solved_exactly() {
    // A = diag(d): every block Krylov space holds the exact solution xₖ = bₖ / dₖ once it has
    // as many vectors as A has distinct eigenvalues; here 6, so 3 right-hand sides need about
    // 2 block steps
    let n = 60;
    let d = |r: usize| c64::new(1.0 + (r % 6) as f64, 0.3 * (r % 2) as f64);
    let a = Sparse::new(n, (0..n).map(|r| (r, r, d(r))));
    let b: Vec<Vec<c64>> = (0..3).map(|k| rhs(n, k)).collect();
    for symmetric in [false, true] {
        let (x, how) = block_qmr(&a, &b, stop(1e-12, 100), symmetric, None).unwrap();
        for (k, (xk, bk)) in x.iter().zip(&b).enumerate() {
            let exact: Vec<c64> = bk.iter().enumerate().map(|(r, v)| v / d(r)).collect();
            let e = largest_difference(xk, &exact);
            assert!(e < 1e-11, "symmetric {symmetric}, column {k}: {e}");
        }
        // 6 distinct eigenvalues and 3 columns: the space is whole at 18 vectors, its last three
        // products deflated
        assert!(how.iterations <= 18, "{how:?}");
    }
}

#[test]
fn block_qmr_solves_a_nonsymmetric_block_to_its_tolerance() {
    let n = 400;
    let a = nonsymmetric(n);
    let b: Vec<Vec<c64>> = (0..5).map(|k| rhs(n, k)).collect();
    let (x, how) = block_qmr(&a, &b, stop(1e-10, 2000), false, None).unwrap();
    let mut singles = 0;
    for (k, (xk, bk)) in x.iter().zip(&b).enumerate() {
        let r = relative_residual(&a, bk, xk);
        assert!(r <= 1e-10, "column {k}: {r}");
        assert!((how.columns[k].residual - r).abs() < 1e-12);
        let (single, once) = qmr(&a, bk, stop(1e-10, 2000)).unwrap();
        singles += once.iterations;
        let e = largest_difference(xk, &single);
        assert!(e < 1e-8, "column {k}: {e}");
    }
    // the same Krylov dimension serves all five: fewer products than one solve each
    assert!(
        how.iterations < singles,
        "{} against {singles}",
        how.iterations
    );
    assert_eq!(how.restarts, 0);
}

#[test]
fn one_right_hand_side_is_qmr() {
    // a block of one is QMR: the same Krylov space and the same quasi-minimal residual, so the
    // same iterations and solution to rounding, while the iterations stay well below n. The
    // indefinite complex symmetric matrix needs about n of them: past n, where exact arithmetic
    // would have stopped, both run on rounding (the Lanczos vectors lose their
    // biorthogonality), and their counts drift apart by a few per cent, by a different amount
    // on each system and right-hand side; there only the solutions are compared closely.
    let n = 300;
    for (a, symmetric) in [(nonsymmetric(n), false), (complex_symmetric(n), true)] {
        let b = rhs(n, 1);
        let (x, how) = block_qmr(
            &a,
            std::slice::from_ref(&b),
            stop(1e-10, 5000),
            symmetric,
            None,
        )
        .unwrap();
        let (y, once) = if symmetric {
            qmr_symmetric(&a, &b, stop(1e-10, 5000)).unwrap()
        } else {
            qmr(&a, &b, stop(1e-10, 5000)).unwrap()
        };
        let allowed = if once.iterations < n / 2 {
            1
        } else {
            once.iterations / 20
        };
        assert!(
            how.iterations.abs_diff(once.iterations) <= allowed,
            "symmetric {symmetric}: {} against {}",
            how.iterations,
            once.iterations
        );
        let e = largest_difference(&x[0], &y);
        assert!(e < 1e-8, "{e}");
    }
}

#[test]
fn the_symmetric_form_is_the_general_one_without_its_left_vectors() {
    // on an exactly symmetric A with L = B, the left Lanczos vectors are the right ones bit for
    // bit, so dropping them and Aᵀ's products changes nothing
    let n = 2 * CHUNK + 5;
    let a = complex_symmetric(n);
    let b: Vec<Vec<c64>> = (0..4).map(|k| rhs(n, k)).collect();
    let (x, how) = block_qmr(&a, &b, stop(1e-10, 4000), false, None).unwrap();
    let (y, how_y) = block_qmr(&a, &b, stop(1e-10, 4000), true, None).unwrap();
    assert!(how.iterations > 20);
    for (p, q) in x.iter().zip(&y) {
        assert_eq!(bits(p), bits(q));
    }
    assert_eq!(how, how_y);
    for (k, (xk, bk)) in y.iter().zip(&b).enumerate() {
        let r = relative_residual(&a, bk, xk);
        assert!(r <= 1e-10, "column {k}: {r}");
    }
}

#[test]
fn a_dependent_right_hand_side_is_deflated_and_recovered() {
    // b₃ = b₁ + 2 b₂ leaves nothing after biorthogonalization against the vectors from b₁ and
    // b₂: it leaves the block at once, and x₃ = x₁ + 2 x₂ (their Eq. 4.18)
    let n = 400;
    for (a, symmetric) in [(nonsymmetric(n), false), (complex_symmetric(n), true)] {
        let (b1, b2) = (rhs(n, 0), rhs(n, 1));
        let b3: Vec<c64> = b1.iter().zip(&b2).map(|(p, q)| p + 2.0 * q).collect();
        let b = vec![b1, b2, b3, rhs(n, 2)];
        let (x, how) = block_qmr(&a, &b, stop(1e-10, 5000), symmetric, None).unwrap();
        assert!(how.dropped >= 1 && how.deflations >= 1, "{how:?}");
        for (k, (xk, bk)) in x.iter().zip(&b).enumerate() {
            let r = relative_residual(&a, bk, xk);
            assert!(r <= 1e-10, "symmetric {symmetric}, column {k}: {r}");
        }
        let combined: Vec<c64> = x[0].iter().zip(&x[1]).map(|(p, q)| p + 2.0 * q).collect();
        let e = largest_difference(&x[2], &combined);
        assert!(e < 1e-12, "{e}");
        assert_eq!(how.restarts, 0, "{how:?}");
    }
}

#[test]
fn a_product_deflated_drops_a_system_whose_solution_is_known() {
    // B = [b, A b, c]: A v₁ lies in the span of b and A b, so the first product is deflated,
    // a combination of the systems has no quasi-residual left and one system leaves the block
    // (their Eqs. 4.16–4.20); the solution of A x = A b is b
    let n = 400;
    for (a, symmetric) in [(nonsymmetric(n), false), (complex_symmetric(n), true)] {
        let b0 = rhs(n, 0);
        let ab = a.apply(&b0);
        let b = vec![b0.clone(), ab, rhs(n, 3)];
        let (x, how) = block_qmr(&a, &b, stop(1e-11, 5000), symmetric, None).unwrap();
        assert!(how.dropped >= 1 && how.deflations >= 1, "{how:?}");
        let e = largest_difference(&x[1], &b0);
        assert!(e < 1e-9, "symmetric {symmetric}: {e}");
        for (k, (xk, bk)) in x.iter().zip(&b).enumerate() {
            let r = relative_residual(&a, bk, xk);
            assert!(r <= 1e-11, "symmetric {symmetric}, column {k}: {r}");
        }
    }
}

#[test]
fn weights_set_the_residual_a_system_stops_on() {
    // ‖W(b − A x)‖ ≤ tol ‖W b‖, W uneven: each column meets it in W's metric
    let n = 300;
    let a = complex_symmetric(n);
    let w: Vec<c64> = (0..n)
        .map(|r| c64::new(1.0 + 30.0 * ((r % 11) == 0) as u8 as f64, 0.0))
        .collect();
    let b: Vec<Vec<c64>> = (0..3).map(|k| rhs(n, k)).collect();
    let (x, how) = block_qmr(&a, &b, stop(1e-9, 5000), true, Some(&w)).unwrap();
    for (k, (xk, bk)) in x.iter().zip(&b).enumerate() {
        let ax = a.apply(xk);
        let r: Vec<c64> = (0..n).map(|i| (bk[i] - ax[i]) * w[i]).collect();
        let wb: Vec<c64> = (0..n).map(|i| bk[i] * w[i]).collect();
        let reached = norm(&r) / norm(&wb);
        assert!(reached <= 1e-9, "column {k}: {reached}");
        assert!((how.columns[k].residual - reached).abs() < 1e-14);
    }
}

#[test]
fn errors_and_zeros() {
    let n = 50;
    let a = nonsymmetric(n);
    let zero = vec![c64::new(0.0, 0.0); n];
    // a zero column is solved by zero in no iterations, beside the others
    let (x, how) = block_qmr(
        &a,
        &[zero.clone(), rhs(n, 0)],
        stop(1e-10, 500),
        false,
        None,
    )
    .unwrap();
    assert_eq!(x[0], zero);
    assert_eq!(how.columns[0].iterations, 0);
    assert!(relative_residual(&a, &rhs(n, 0), &x[1]) <= 1e-10);
    // nothing to solve
    let (x, how) = block_qmr(&a, &[], stop(1e-10, 500), false, None).unwrap();
    assert!(x.is_empty() && how.iterations == 0);
    // wrong sizes, a tolerance that isn't positive, too few iterations
    assert!(
        block_qmr(
            &a,
            &[vec![c64::new(1.0, 0.0); 3]],
            stop(1e-10, 9),
            false,
            None
        )
        .is_err()
    );
    assert!(block_qmr(&a, &[rhs(n, 0)], stop(0.0, 9), false, None).is_err());
    assert!(block_qmr(&a, &[rhs(n, 0)], stop(1e-10, 9), false, Some(&[])).is_err());
    let e = block_qmr(&a, &[rhs(n, 0), rhs(n, 1)], stop(1e-14, 3), false, None).unwrap_err();
    assert!(e.to_string().contains("no convergence"), "{e}");
}

#[test]
fn block_qmr_is_the_same_bit_for_bit_on_any_number_of_threads() {
    // more values than one chunk of its sums, so the chunks are in play
    let n = 3 * CHUNK + 17;
    let b: Vec<Vec<c64>> = (0..3).map(|k| rhs(n, k)).collect();
    for (a, symmetric) in [(nonsymmetric(n), false), (complex_symmetric(n), true)] {
        let run = |threads: usize| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| block_qmr(&a, &b, stop(1e-10, 3000), symmetric, None).unwrap())
        };
        let (one, how) = run(1);
        for threads in [2, 5, 20] {
            let (many, how_many) = run(threads);
            assert_eq!(how, how_many);
            for (p, q) in one.iter().zip(&many) {
                assert_eq!(bits(p), bits(q), "{threads} threads differ from one");
            }
        }
    }
}

#[test]
fn a_blocks_products_and_triangular_solves_are_each_vectors_own() {
    // taken together, the matrix's rows and ILU(0)'s factors read once for all: each result
    // the same bits as alone
    use crate::fdfd::krylov::{Ilu0, Preconditioner};
    let n = 2 * super::super::ROWS + 37;
    let a = nonsymmetric(n);
    let ilu = Ilu0::new(&a).unwrap();
    let vs: Vec<Vec<c64>> = (0..3).map(|k| rhs(n, k)).collect();
    let refs: Vec<&[c64]> = vs.iter().map(Vec::as_slice).collect();
    let together = [
        a.apply_block(&refs),
        a.apply_transpose_block(&refs),
        ilu.solve_block(&refs),
        ilu.solve_transpose_block(&refs),
    ];
    for (k, v) in vs.iter().enumerate() {
        let alone = [
            a.apply(v),
            a.apply_transpose(v),
            ilu.solve(v),
            ilu.solve_transpose(v),
        ];
        for (t, s) in together.iter().zip(&alone) {
            assert_eq!(bits(&t[k]), bits(s));
        }
    }
}

#[test]
fn freund_and_malhotras_example_7_1() {
    let (one, five) = super::example::iterations(5);
    // theirs: 19 and 85, a ratio of 4.47 (their stopping test isn't legible in our copy; at
    // 1e-6 this gives 14 and 63)
    let ratio = five as f64 / one as f64;
    assert!((ratio - 85.0 / 19.0).abs() < 0.5, "{one} and {five}");
}

#[test]
fn a_null_vector_of_a_wide_block() {
    let rows = vec![
        vec![c64::new(1.0, 0.5), c64::new(-2.0, 0.0), c64::new(0.3, 1.0)],
        vec![c64::new(0.0, 1.0), c64::new(1.0, 1.0), c64::new(2.0, -0.5)],
    ];
    let g = null_vector(&rows, 3);
    assert!((norm(&g) - 1.0).abs() < 1e-14);
    for row in &rows {
        let r: c64 = row.iter().zip(&g).map(|(p, q)| p * q).sum();
        assert!(r.norm() < 1e-14, "{r}");
    }
}
