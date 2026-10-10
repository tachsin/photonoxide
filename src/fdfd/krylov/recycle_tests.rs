use super::*;
use crate::fdfd::krylov::{Ilu0, Sparse, gmres_preconditioned};

/// A complex, nonsymmetric, diagonally dominant matrix of n rows, shifted by `shift` on its
/// diagonal, and a right-hand side: the same on every run.
fn case(n: usize, shift: f64) -> (Sparse, Vec<c64>) {
    let mut entries = Vec::new();
    for r in 0..n {
        entries.push((r, r, c64::new(1.2 + shift + 0.1 * (r % 7) as f64, 0.3)));
        entries.push((r, (r + 1) % n, c64::new(-1.0, 0.3)));
        entries.push((r, (r + n - 1) % n, c64::new(-0.7, -0.2)));
        entries.push((r, (r * 7 + 3) % n, c64::new(0.4, 0.25)));
    }
    let b = (0..n)
        .map(|r| c64::new((r % 5) as f64 - 2.0, (r % 3) as f64))
        .collect();
    (Sparse::new(n, entries), b)
}

fn residual(a: &Sparse, b: &[c64], x: &[c64]) -> f64 {
    let ax = a.apply(x);
    let r: Vec<c64> = b.iter().zip(&ax).map(|(p, q)| p - q).collect();
    norm(&r) / norm(b)
}

fn stop(tolerance: f64) -> Stopping {
    Stopping {
        tolerance,
        max_iterations: 5000,
    }
}

#[test]
fn gcro_dr_solves_and_keeping_nothing_is_restarted_gmres() {
    let (a, b) = case(600, 0.0);
    let mut space = Space::default();
    let (x, how) = gcro_dr(&a, &Identity, &b, stop(1e-10), 20, 0, &mut space, 0).unwrap();
    assert!(residual(&a, &b, &x) <= 1e-10 && how.residual <= 1e-10);
    assert_eq!(space.dimension(), 0);
    let (y, plain) = gmres_preconditioned(&a, &Identity, &b, stop(1e-10), 20).unwrap();
    assert_eq!(how.iterations, plain.iterations);
    let d: Vec<c64> = x.iter().zip(&y).map(|(p, q)| p - q).collect();
    assert!(norm(&d) < 1e-8 * norm(&y));
}

#[test]
fn a_recycled_space_saves_iterations_on_the_same_matrix_and_the_next() {
    let (a, b) = case(600, 0.0);
    let mut space = Space::default();
    let (_, first) = gcro_dr(&a, &Identity, &b, stop(1e-10), 20, 8, &mut space, 0).unwrap();
    let (_, plain) = gmres_preconditioned(&a, &Identity, &b, stop(1e-10), 20).unwrap();
    assert_eq!(space.dimension(), 8);
    // C = A U and Cᴴ C = I
    for (u, c) in space.u.iter().zip(&space.c) {
        let au = a.apply(u);
        let d: Vec<c64> = au.iter().zip(c).map(|(p, q)| p - q).collect();
        assert!(norm(&d) < 1e-10, "{}", norm(&d));
    }
    for (i, ci) in space.c.iter().enumerate() {
        for (j, cj) in space.c.iter().enumerate() {
            let expected = if i == j { 1.0 } else { 0.0 };
            assert!((dot_conj(ci, cj) - expected).norm() < 1e-12);
        }
    }
    // again, on another right-hand side of the same matrix: no products spent on C
    let b2: Vec<c64> = (0..600)
        .map(|r| c64::new(((r * 3) % 7) as f64, 1.0))
        .collect();
    let (x2, second) = gcro_dr(&a, &Identity, &b2, stop(1e-10), 20, 8, &mut space, 0).unwrap();
    assert!(residual(&a, &b2, &x2) <= 1e-10);
    assert_eq!(space.products, 0);
    let (_, plain2) = gmres_preconditioned(&a, &Identity, &b2, stop(1e-10), 20).unwrap();
    assert!(
        second.iterations < plain2.iterations,
        "{} {} {} {}",
        first.iterations,
        plain.iterations,
        second.iterations,
        plain2.iterations
    );
    // a nearby matrix: C rebuilt with k products
    let (a3, _) = case(600, 0.02);
    let (x3, third) = gcro_dr(&a3, &Identity, &b, stop(1e-10), 20, 8, &mut space, 1).unwrap();
    assert!(residual(&a3, &b, &x3) <= 1e-10);
    assert_eq!(space.products, 8);
    let (_, plain3) = gmres_preconditioned(&a3, &Identity, &b, stop(1e-10), 20).unwrap();
    assert!(third.iterations < plain3.iterations);
}

#[test]
fn gcro_dr_preconditioned_from_the_right() {
    let (a, b) = case(400, 2.0);
    let ilu = Ilu0::new(&a).unwrap();
    let mut space = Space::default();
    let (x, _) = gcro_dr(&a, &ilu, &b, stop(1e-11), 10, 4, &mut space, 0).unwrap();
    assert!(residual(&a, &b, &x) <= 1e-11);
    let (x, _) = gcro_dr(&a, &ilu, &b, stop(1e-11), 10, 4, &mut space, 0).unwrap();
    assert!(residual(&a, &b, &x) <= 1e-11);
}

#[test]
fn parks_section_4_4() {
    // their Fig. 4.9 (c = 0): the rerun with the recycled space takes fewer products than full
    // GMRES; the first run (GMRES-DR) a few more
    let (first, second, full, _) = example::twice(0.0);
    assert!(second < full && full < first, "{first} {second} {full}");
    // their Table 4.3 (c = 0): five cosines at 1 and five at 0
    let cosines = example::table_cosines();
    assert_eq!(cosines.len(), 10);
    assert!(cosines[..5].iter().all(|c| 1.0 - c < 1e-6), "{cosines:?}");
    assert!(cosines[5..].iter().all(|&c| c < 1e-6), "{cosines:?}");
}

#[test]
fn an_exact_invariant_subspace_recycled_converges_as_the_deflated_problem() {
    let (worst, recycled, reference, whole) = example::deflated_against_gmres();
    eprintln!("{worst:e} {recycled} {reference} {whole}");
    assert!(worst < 1e-8, "{worst:e}");
    assert_eq!(recycled, reference);
    assert!(recycled < whole);
}
