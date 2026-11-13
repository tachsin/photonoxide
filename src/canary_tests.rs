//! The platform canaries that need the library's internals: its own special functions, its
//! sparse LU, its Krylov solvers and its least squares. `tests/canary.rs` has the rest, the
//! tables of every canary with its reference and its rule, and the list of existing tests the
//! canaries reuse; `just canaries` runs them all. Two more sit beside the private modules they
//! test (`expr::tests::canary_erf_to_round_off`, `geometry::region_tests::canary_gamma_to_fifteen_digits`,
//! `nonlinear::fft::tests::canary_fft_against_the_exact_dft`): this module can't reach them.

pub(crate) mod golden;

use std::sync::Arc;

use faer::Mat;
use faer::linalg::solvers::{DenseSolveCore, Solve};
use faer::sparse::{SparseColMat, Triplet};
use num_complex::Complex64 as c64;

use golden::{golden, ulp};

const EPS: f64 = f64::EPSILON;

/// J_m and Y_m (m = 0, 1) on the real axis and in the right half plane, and H⁽¹⁾_m = J_m + iY_m,
/// from `fdtd::ring::bessel`'s power series, against mpmath. The rule is its doc's: the series
/// lose about e^|z|/(2π|z|) of their precision to cancellation (the largest term against the
/// sum), so a value is within ε·e^|z|/(2π|z|) of the exact one, at least its own ulp.
#[test]
fn canary_bessel_and_hankel() {
    let mut worst = 0.0f64;
    let mut failures = Vec::new();
    for m in 0..2u32 {
        let check = |name: String, z: c64, ours: c64, want: c64, failures: &mut Vec<String>| {
            let r = z.norm();
            let lost = EPS * r.exp() / (2.0 * std::f64::consts::PI * r);
            let allowed = lost.max(ulp(want.norm())) + 0.5 * (ulp(want.re) + ulp(want.im));
            let off = (ours - want).norm();
            if off > allowed {
                failures.push(format!(
                    "{name}({z}) = {ours}, exact {want}: off {off:e} > {allowed:e}"
                ));
            }
            off / allowed
        };
        for (name, take_y) in [
            (format!("bessel_j{m}"), false),
            (format!("bessel_y{m}"), true),
        ] {
            for g in golden(&name) {
                let z = c64::new(g.a, 0.0);
                let (j, y) = crate::fdtd::ring::bessel(m, z);
                let ours = if take_y { y } else { j };
                worst = worst.max(check(
                    name.clone(),
                    z,
                    ours,
                    c64::new(g.r, 0.0),
                    &mut failures,
                ));
            }
            for g in golden(&format!("{name}c")) {
                let z = c64::new(g.a, g.b);
                let (j, y) = crate::fdtd::ring::bessel(m, z);
                let ours = if take_y { y } else { j };
                let want = c64::new(g.r, g.i);
                worst = worst.max(check(format!("{name}c"), z, ours, want, &mut failures));
            }
        }
        let name = format!("hankel1_{m}");
        for g in golden(&name) {
            let z = c64::new(g.a, g.b);
            let (j, y) = crate::fdtd::ring::bessel(m, z);
            let ours = j + c64::new(0.0, 1.0) * y;
            let want = c64::new(g.r, g.i);
            worst = worst.max(check(name.clone(), z, ours, want, &mut failures));
        }
    }
    println!("bessel: worst error {worst:.3} of its bound");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// E₁(x) for 1e-5 ≤ x ≤ 50 (`material::catalogue::conductivity::e1`) against mpmath. Below 1,
/// −γ − ln x plus a series whose terms fall at once (x^k/(k·k!)): the sum's rounding (an ulp of
/// each of −γ, ln x and the series, at most 60 terms each rounded twice) relative to the
/// result. Above 1, the continued fraction stopped at a relative step of 1e-16, its value then
/// within a few ulps; the same count of roundings bounds it: 2 per level of the fraction it
/// took, at most 200.
#[test]
fn canary_exponential_integral() {
    let u = EPS / 2.0;
    let mut worst = 0.0f64;
    let mut failures = Vec::new();
    for g in golden("e1") {
        let x = g.a;
        let ours = crate::material::catalogue::conductivity::e1(x);
        let allowed = if x < 1.0 {
            // ulps of the three parts, the series' 60 terms rounded twice each
            let series = x * (1.0 + x); // above the series' sum |Σ| ≤ x e^x / 1
            ulp(std::f64::consts::EULER_GAMMA) + ulp(x.ln()) + 120.0 * u * series + 1.5 * ulp(g.r)
        } else {
            // the fraction's levels, measured by its own stopping: those it needs to reach a
            // step of 1e-16 are at most 200, each two roundings
            400.0 * u * g.r.abs() + 0.5 * ulp(g.r)
        };
        let off = (ours - g.r).abs();
        worst = worst.max(off / ulp(g.r));
        if off > allowed {
            failures.push(format!(
                "e1({x:e}) = {ours:e}, exact {:e}: off {off:e} > {allowed:e}",
                g.r
            ));
        }
    }
    println!("e1: worst {worst:.2} ulps");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A non-symmetric complex operator on a 13 × 11 grid (a convection–diffusion stencil with a
/// complex shift, its rows unequal), its entries exact.
fn stencil() -> (usize, Vec<(usize, usize, c64)>) {
    let (nx, ny) = (13, 11);
    let n = nx * ny;
    let mut t = Vec::new();
    for j in 0..ny {
        for i in 0..nx {
            let k = j * nx + i;
            let w = 1.0 + (k % 5) as f64 / 4.0;
            t.push((k, k, c64::new(-4.0 * w, 0.25)));
            if i + 1 < nx {
                t.push((k, k + 1, c64::new(1.25, 0.125)));
                t.push((k + 1, k, c64::new(0.75, -0.25)));
            }
            if j + 1 < ny {
                t.push((k, k + nx, c64::new(1.0, 0.0)));
                t.push((k + nx, k, c64::new(1.0, 0.0625)));
            }
        }
    }
    (n, t)
}

/// ‖·‖∞ of a complex vector or a matrix's rows.
fn inf_norm(v: &[c64]) -> f64 {
    v.iter().map(|z| z.norm()).fold(0.0, f64::max)
}

/// The library's sparse LU (multifrontal, its fronts on faer's kernels) against faer's dense LU
/// on the same matrix. The rules: each solve's normwise backward error
/// η = ‖b − Ax‖∞ / (‖A‖∞‖x‖∞ + ‖b‖∞) ≤ 10·n·ε, as the dense canaries' (backward stable
/// factorizations, N. J. Higham, *Accuracy and Stability of Numerical Algorithms*, 2nd ed.,
/// SIAM (2002), doi:10.1137/1.9780898718027, Theorem 9.4); and the two solutions within what
/// their backward errors allow, κ∞(η₁ + η₂)/(1 − κ∞ max η) (Theorem 7.2).
#[test]
fn canary_sparse_lu_against_dense() {
    let (n, t) = stencil();
    let triplets: Vec<Triplet<usize, usize, c64>> =
        t.iter().map(|&(r, c, v)| Triplet::new(r, c, v)).collect();
    let a = SparseColMat::try_new_from_triplets(n, n, &triplets).unwrap();
    let b: Vec<c64> = (0..n)
        .map(|i| c64::new((i % 7) as f64 - 3.0, (i % 3) as f64))
        .collect();
    let analysis = Arc::new(crate::sparse::Analysis::new(a.as_ref(), None).unwrap());
    let lu = crate::sparse::Multifrontal::new(analysis, a.as_ref()).unwrap();
    let xs = lu.solve(&b);

    let dense = Mat::<c64>::from_fn(n, n, |_, _| c64::new(0.0, 0.0));
    let mut dense = dense;
    for &(r, c, v) in &t {
        dense[(r, c)] += v;
    }
    let dlu = dense.partial_piv_lu();
    let bm = Mat::from_fn(n, 1, |i, _| b[i]);
    let xd_m = dlu.solve(&bm);
    let xd: Vec<c64> = (0..n).map(|i| xd_m[(i, 0)]).collect();

    let a_inf = (0..n)
        .map(|r| (0..n).map(|c| dense[(r, c)].norm()).sum::<f64>())
        .fold(0.0, f64::max);
    let eta = |x: &[c64]| {
        let mut ax = vec![c64::new(0.0, 0.0); n];
        for &(r, c, v) in &t {
            ax[r] += v * x[c];
        }
        let r: Vec<c64> = b.iter().zip(&ax).map(|(b, a)| b - a).collect();
        inf_norm(&r) / (a_inf * inf_norm(x) + inf_norm(&b))
    };
    let (es, ed) = (eta(&xs), eta(&xd));
    let bound = 10.0 * n as f64 * EPS;
    assert!(
        es <= bound,
        "the sparse LU's backward error {es:e} > {bound:e}"
    );
    assert!(
        ed <= bound,
        "faer's dense LU's backward error {ed:e} > {bound:e}"
    );

    let inv = dlu.inverse();
    let inv_inf = (0..n)
        .map(|r| (0..n).map(|c| inv[(r, c)].norm()).sum::<f64>())
        .fold(0.0, f64::max);
    let kappa = a_inf * inv_inf;
    let diff: Vec<c64> = xs.iter().zip(&xd).map(|(s, d)| s - d).collect();
    let forward = inf_norm(&diff) / inf_norm(&xd);
    let allowed = kappa * (es + ed) / (1.0 - kappa * es.max(ed));
    println!(
        "sparse LU: η {es:.2e}, dense η {ed:.2e}, κ∞ {kappa:.2e}, apart {forward:.2e} (allowed {allowed:.2e})"
    );
    assert!(forward <= allowed, "apart {forward:e} > {allowed:e}");
}

/// The identity, as a preconditioner.
struct Identity;

impl crate::backend::krylov::Preconditioner for Identity {
    fn solve(&self, v: &[c64]) -> Vec<c64> {
        v.to_vec()
    }
    fn solve_transpose(&self, v: &[c64]) -> Vec<c64> {
        v.to_vec()
    }
}

/// QMR and GMRES on a matrix whose iteration count is known in closed form: A is block diagonal
/// with 2 × 2 upper triangular blocks [λ_a 1/2; 0 λ_b], λ_a ≠ λ_b taken from k = 20 values on
/// the circle |λ − 2| = 1, so A is diagonalizable, not normal, and has k distinct eigenvalues;
/// b has a part along each eigenvector. In exact arithmetic the Krylov space of (A, b) has
/// dimension k, and both methods find the solution at step k, not before: the best residual
/// polynomial of degree j < k leaves about (1/2)^j of b (the eigenvalues' distance from 0
/// against the circle's radius), far above the tolerance at j = k − 1. The rule: the iteration
/// count within 5 % of k (one iteration either way at k = 20), as the 3D FDFD's count is held
/// against Linux's; and the true residual ‖b − Ax‖/‖b‖ below the tolerance asked.
#[test]
fn canary_qmr_and_gmres_iterations_are_the_closed_form() {
    use crate::backend::krylov::{Sparse, Stopping, gmres_preconditioned, qmr};
    let k = 20usize;
    let lambda = |j: usize| {
        let a = std::f64::consts::TAU * j as f64 / k as f64;
        c64::new(2.0 + a.cos(), a.sin())
    };
    let blocks = 3 * k;
    let n = 2 * blocks;
    let mut e = Vec::new();
    for m in 0..blocks {
        let (p, q) = (2 * m, 2 * m + 1);
        e.push((p, p, lambda((2 * m) % k)));
        e.push((q, q, lambda((2 * m + 1) % k)));
        e.push((p, q, c64::new(0.5, 0.0)));
    }
    let a = Sparse::new(n, e.clone());
    let b: Vec<c64> = (0..n)
        .map(|i| c64::new(1.0 + (i % 4) as f64 / 8.0, (i % 3) as f64 / 4.0))
        .collect();
    let stopping = Stopping {
        tolerance: 1e-10,
        max_iterations: 200,
    };
    let true_residual = |x: &[c64]| {
        let mut ax = vec![c64::new(0.0, 0.0); n];
        for &(r, c, v) in &e {
            ax[r] += v * x[c];
        }
        let num: f64 = b.iter().zip(&ax).map(|(b, a)| (b - a).norm_sqr()).sum();
        let den: f64 = b.iter().map(|b| b.norm_sqr()).sum();
        (num / den).sqrt()
    };
    let (xq, cq) = qmr(&a, &b, stopping).unwrap();
    let (xg, cg) = gmres_preconditioned(&a, &Identity, &b, stopping, 50).unwrap();
    let allowed = (0.05 * k as f64).floor() as usize;
    for (name, x, c) in [("QMR", &xq, &cq), ("GMRES", &xg, &cg)] {
        let r = true_residual(x);
        println!(
            "{name}: {} iterations (closed form {k}), residual {r:.2e}",
            c.iterations
        );
        assert!(
            c.iterations.abs_diff(k) <= allowed,
            "{name}: {} iterations, the closed form {k} ± {allowed}",
            c.iterations
        );
        assert!(r <= stopping.tolerance, "{name}: true residual {r:e}");
    }
}

/// The library's least squares (faer's QR, then faer's SVD of R) on a consistent 30 × 8
/// polynomial fit (Vandermonde on [0, 1], κ ≈ 1e5 once its columns are scaled), against the
/// exact solution; and its fallback, one-sided Jacobi, against faer's SVD on the same R. The
/// rules: a backward stable least squares with a zero residual has forward error
/// ≤ c·n·ε·κ (Higham, doi:10.1137/1.9780898718027, Theorem 20.3, the residual term 0), here
/// 10·n·ε·κ as the other dense canaries; and the two SVDs' singular values within
/// 10·n·ε·σ_max of each other (Weyl, both backward stable).
#[test]
fn canary_least_squares_and_its_jacobi_fallback() {
    let (m, n) = (30usize, 8usize);
    let a = Mat::from_fn(m, n, |i, j| (i as f64 / (m - 1) as f64).powi(j as i32));
    let x_exact: Vec<f64> = (0..n).map(|j| (j as f64) - 3.5).collect();
    let b = Mat::from_fn(m, 1, |i, _| {
        (0..n).map(|j| a[(i, j)] * x_exact[j]).sum::<f64>()
    });
    let x = crate::lstsq::least_squares(a.clone(), &b);

    // κ of A with its columns scaled to unit norm, as least_squares solves it
    let scaled = Mat::from_fn(m, n, |i, j| a[(i, j)] / a.col(j).norm_l2());
    let (_, sigma_jacobi, _) = crate::lstsq::jacobi_svd(&scaled);
    let mut sigma_faer = scaled.singular_values().unwrap();
    let mut sigma_j = sigma_jacobi.clone();
    sigma_faer.sort_by(f64::total_cmp);
    sigma_j.sort_by(f64::total_cmp);
    let (top, bottom) = (sigma_faer[n - 1], sigma_faer[0]);
    let kappa = top / bottom;
    let weyl = 10.0 * n as f64 * EPS * top;
    for (f, j) in sigma_faer.iter().zip(&sigma_j) {
        assert!(
            (f - j).abs() <= weyl,
            "σ: faer {f:e}, Jacobi {j:e}, > {weyl:e} apart"
        );
    }
    let off = x
        .iter()
        .zip(&x_exact)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
        / x_exact.iter().map(|v| v * v).sum::<f64>().sqrt();
    let allowed = 10.0 * n as f64 * EPS * kappa;
    println!("least squares: κ {kappa:.2e}, forward error {off:.2e} (allowed {allowed:.2e})");
    assert!(off <= allowed, "forward error {off:e} > {allowed:e}");
}
