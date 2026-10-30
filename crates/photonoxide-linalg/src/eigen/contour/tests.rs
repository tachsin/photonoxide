use super::*;

fn options(subspace: Option<usize>) -> Options {
    Options {
        subspace,
        tolerance: 1e-12,
        iterations: 20,
    }
}

fn in_disc(c: c64, r: f64) -> impl Fn(c64) -> bool + Sync {
    move |z: c64| (z - c).norm() < r
}

fn diagonal(n: usize) -> Vec<(usize, usize, c64)> {
    (0..n)
        .map(|i| (i, i, c64::new(i as f64 + 1.0, 0.0)))
        .collect()
}

#[test]
fn a_diagonal_pencil_gives_its_eigenvalues_to_rounding() {
    // 1, 2, …, 300: the disc around 50.5 of radius 3.2 holds 48 … 53
    let n = 300;
    let a = diagonal(n);
    let pencil = Pencil {
        n,
        a: &a,
        b: None,
        positions: None,
    };
    let (c, r) = (c64::new(50.5, 0.0), 3.2);
    let q = Quadrature::trapezoid(16, true, circle(c, r)).unwrap();
    let found = solve(&pencil, &q, &in_disc(c, r), &options(None)).unwrap();
    let mut values: Vec<f64> = found.pairs.iter().map(|p| p.value.re).collect();
    values.sort_by(f64::total_cmp);
    assert_eq!(values.len(), 6, "{values:?}");
    for (v, e) in values.iter().zip(48..=53) {
        assert!((v - e as f64).abs() < 1e-12, "{v} vs {e}");
    }
    assert!((found.estimate - 6.0).abs() < 2.0, "{}", found.estimate);
}

/// A non-Hermitian sparse matrix of `n` rows: a complex tridiagonal with longer-range entries,
/// and its eigenvalues by a dense solve.
fn non_hermitian(n: usize) -> (Vec<(usize, usize, c64)>, Vec<c64>) {
    let mut a = Vec::new();
    for i in 0..n {
        let x = i as f64;
        a.push((
            i,
            i,
            c64::new(2.0 + 2.0 * (0.37 * x).sin(), 0.3 * (1.3 * x).cos()),
        ));
        if i + 1 < n {
            a.push((i, i + 1, c64::new(-0.6, 0.1)));
            a.push((i + 1, i, c64::new(-0.4, -0.2)));
        }
        if i + 7 < n {
            a.push((i, i + 7, c64::new(0.05, 0.02 * x.cos())));
        }
    }
    let dense = Mat::<c64>::from_fn(n, n, |i, j| {
        a.iter().filter(|e| e.0 == i && e.1 == j).map(|e| e.2).sum()
    });
    (a, dense.eigenvalues().unwrap())
}

#[test]
fn every_eigenvalue_in_a_disc_of_a_non_hermitian_matrix_against_a_dense_solve() {
    let n = 160;
    let (a, all) = non_hermitian(n);
    let pencil = Pencil {
        n,
        a: &a,
        b: None,
        positions: None,
    };
    let (c, r) = (c64::new(2.2, 0.1), 0.45);
    let inside = in_disc(c, r);
    let mut want: Vec<c64> = all.into_iter().filter(|z| inside(*z)).collect();
    assert!(want.len() > 5, "{}", want.len());
    let q = Quadrature::trapezoid(32, false, circle(c, r)).unwrap();
    let found = solve(&pencil, &q, &inside, &options(None)).unwrap();
    let mut got: Vec<c64> = found.pairs.iter().map(|p| p.value).collect();
    assert_eq!(got.len(), want.len(), "{got:?} vs {want:?}");
    let key = |a: &c64, b: &c64| a.re.total_cmp(&b.re);
    got.sort_by(key);
    want.sort_by(key);
    for (g, w) in got.iter().zip(&want) {
        assert!((g - w).norm() < 1e-10, "{g} vs {w}");
    }
}

#[test]
fn a_singular_b_leaves_only_the_finite_eigenvalues() {
    // Sakurai and Sugiura's Example 5 by FEAST: 0, 0.01, 0.02, 0.03 in their circle
    let (a, b) = sakurai_sugiura_example_5();
    let pencil = Pencil {
        n: 100,
        a: &a,
        b: Some(&b),
        positions: None,
    };
    let (c, r) = (c64::new(0.015, 0.0), 0.02);
    let q = Quadrature::trapezoid(32, true, circle(c, r)).unwrap();
    let found = solve(&pencil, &q, &in_disc(c, r), &options(None)).unwrap();
    let mut values: Vec<f64> = found.pairs.iter().map(|p| p.value.re).collect();
    values.sort_by(f64::total_cmp);
    assert_eq!(values.len(), 4, "{values:?}");
    for (v, e) in values.iter().zip([0.0, 0.01, 0.02, 0.03]) {
        assert!((v - e).abs() < 1e-14, "{v} vs {e}");
    }
}

#[test]
fn sakurai_and_sugiura_example_5_converges_as_they_print() {
    // their errors: 2.1e-6 at N = 64 and 1.3e-12 at 128; the bound δ^(2m − N), δ = 1.25
    let mut e64: Vec<f64> = (0..33).map(|d| sakurai_sugiura_error(64, d)).collect();
    let mut ratios: Vec<f64> = (0..33)
        .map(|d| e64[d as usize] / sakurai_sugiura_error(128, d))
        .collect();
    e64.sort_by(f64::total_cmp);
    ratios.sort_by(f64::total_cmp);
    println!("{e64:?}\n{ratios:?}");
    // the error's size depends on u and v, its fall from 64 to 128 points doesn't
    let median = e64[16];
    assert!(median > 2.1e-7 && median < 2.1e-5, "{median}");
    assert!(
        ratios[0] > 1.4e6 && ratios[32] < 1.8e6,
        "{} {}",
        ratios[0],
        ratios[32]
    );
}

#[test]
fn too_small_a_subspace_is_an_error_that_says_so() {
    let n = 300;
    let a = diagonal(n);
    let pencil = Pencil {
        n,
        a: &a,
        b: None,
        positions: None,
    };
    // 20 eigenvalues inside, a subspace of 6
    let (c, r) = (c64::new(100.5, 0.0), 10.2);
    let q = Quadrature::trapezoid(16, true, circle(c, r)).unwrap();
    let e = solve(&pencil, &q, &in_disc(c, r), &options(Some(6)))
        .err()
        .expect("an error")
        .to_string();
    assert!(e.contains("subspace"), "{e}");
    // with none given, it grows to fit
    let found = solve(&pencil, &q, &in_disc(c, r), &options(None)).unwrap();
    assert_eq!(found.pairs.len(), 20);
}

#[test]
fn the_same_bits_on_one_and_many_threads() {
    let n = 160;
    let (a, _) = non_hermitian(n);
    let pencil = Pencil {
        n,
        a: &a,
        b: None,
        positions: None,
    };
    let (c, r) = (c64::new(2.2, 0.1), 0.45);
    let q = Quadrature::trapezoid(32, false, circle(c, r)).unwrap();
    let run = |threads: usize| {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        pool.install(|| solve(&pencil, &q, &in_disc(c, r), &options(None)).unwrap())
    };
    let one = run(1);
    for threads in [3, 8] {
        let many = run(threads);
        assert_eq!(one.pairs.len(), many.pairs.len());
        for (x, y) in one.pairs.iter().zip(&many.pairs) {
            assert_eq!(x.value, y.value);
            assert_eq!(x.vector, y.vector);
        }
        assert_eq!(one.estimate.to_bits(), many.estimate.to_bits());
    }
}

#[test]
fn the_symmetric_rule_is_its_own_mirror_and_filters() {
    let (c, r) = (c64::new(1.0, 0.0), 0.5);
    let q = Quadrature::trapezoid(16, true, circle(c, r)).unwrap();
    let k = q.nodes().len();
    for j in 0..k / 2 {
        assert_eq!(q.nodes()[k - 1 - j], q.nodes()[j].conj());
    }
    assert!((q.filter(c64::new(1.1, 0.05)) - 1.0).norm() < 1e-10);
    // outside, damped by about (distance / r)^−N
    assert!(q.filter(c64::new(2.5, 0.0)).norm() < 3f64.powi(-15));
    assert!(Quadrature::trapezoid(15, true, circle(c, r)).is_err());
}
