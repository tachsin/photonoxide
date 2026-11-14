//! The platform canaries that need the library's internals: its own special functions, its
//! sparse LU, its Krylov solvers and its least squares. `tests/canary.rs` has the rest, the
//! tables of every canary with its reference and its rule, and the list of existing tests the
//! canaries reuse; `just canaries` runs them all. Two more sit beside the private modules they
//! test (`expr::tests::canary_erf_to_round_off`, `geometry::region_tests::canary_gamma_to_fifteen_digits`,
//! `nonlinear::fft::tests::canary_fft_against_the_exact_dft`): this module can't reach them.

mod faer_bits;
pub(crate) mod golden;

use crate::portable;
use std::sync::Arc;

use faer::Mat;
use faer::linalg::solvers::{DenseSolveCore, Solve};
use faer::sparse::{SparseColMat, Triplet};
use num_complex::Complex64 as c64;

use golden::{golden, ulp};

const EPS: f64 = f64::EPSILON;

/// A running error bound for `fdtd::ring::bessel(m, z)`, its own arithmetic to first order in
/// u = 2⁻⁵³, from the magnitudes of the terms it sums. Each J term t_k comes from the lead
/// (z/2)^m/m! (m complex products and divisions, 4u each) by k updates
/// t_k = t_(k−1)·(−z²/4)/(k(m + k)) (the quarter's product 3u, carried into every term, the
/// product 3u and the division u: 7u each), and N terms are summed (Nu of the sum of their
/// magnitudes). The Y sum weights each term by ψ(k + 1) + ψ(m + k + 1), sums of k + m + 2
/// roundings; ln(z/2) is hypot, ln and atan2, 2u(1 + |ln(z/2)|); the finite sum for m > 0 is
/// built as the J terms are; −finite/π + (2/π) ln(z/2) J − weighted/π rounds its parts 8 times.
/// Returns the bounds on J and on Y.
fn bessel_bound(m: u32, z: c64, j: c64) -> (f64, f64) {
    let u = EPS / 2.0;
    let h = portable::complex::abs(z) / 2.0;
    let (m_f, euler) = (f64::from(m), std::f64::consts::EULER_GAMMA);
    let mut t = (1..=m).fold(1.0, |p, i| p * h / f64::from(i));
    let mut psi_k = euler;
    let mut psi_mk = euler + (1..=m).map(|i| 1.0 / f64::from(i)).sum::<f64>();
    // Σ t_k (7k + 4m) and Σ t_k; the same with the ψ weights' magnitudes
    let (mut sum_j, mut mag_j, mut sum_w, mut mag_w) = (0.0, 0.0, 0.0, 0.0);
    let mut k = 0u32;
    loop {
        let k_f = f64::from(k);
        let w = psi_k + psi_mk;
        sum_j += t * (7.0 * k_f + 4.0 * m_f);
        mag_j += t;
        sum_w += t * w * (9.0 * k_f + 5.0 * m_f + 7.0);
        mag_w += t * w;
        k += 1;
        t *= h * h / (f64::from(k) * f64::from(m + k));
        psi_k += 1.0 / f64::from(k);
        psi_mk += 1.0 / f64::from(m + k);
        if t <= 1e-18 * mag_j && f64::from(k) > h {
            break;
        }
    }
    let terms = f64::from(k);
    let err_j = u * (sum_j + terms * mag_j);
    let err_w = u * (sum_w + terms * mag_w);
    // the finite sum: (m − i − 1)!/i! (z/2)^(2i − m), each built from the first by 4u steps
    let (mut finite, mut err_f) = (0.0, 0.0);
    if m > 0 {
        let mut f = (1..m).map(f64::from).product::<f64>() / portable::powi(h, m as i32);
        for i in 0..m {
            finite += f;
            err_f += u * f * (5.0 * m_f + 4.0 + 4.0 * f64::from(i));
            if i + 1 < m {
                f *= h * h / (f64::from(i + 1) * f64::from(m - i - 1));
            }
        }
    }
    let log = portable::complex::abs(portable::complex::ln(z / 2.0));
    let d_log = 2.0 * u * (1.0 + log);
    let parts = finite + 2.0 * log * portable::complex::abs(j) + mag_w;
    let err_y =
        (err_f + 2.0 * (log * err_j + portable::complex::abs(j) * d_log) + err_w + 8.0 * u * parts)
            / std::f64::consts::PI;
    (err_j, err_y)
}

/// J_m and Y_m (m = 0, 1) on the real axis and in the right half plane, and H⁽¹⁾_m = J_m + iY_m,
/// from `fdtd::ring::bessel`'s power series, against mpmath: within [`bessel_bound`], the
/// running error of the series as they are summed, beyond the golden value's own half ulps.
/// (Its doc's estimate of the loss, e^|z|/(2π|z|) of the precision, is low by about √(2π|z|):
/// the terms' magnitudes sum to I_m(|z|) ≈ e^|z|/√(2π|z|), 7.5e-11 of ε at |z| = 15.)
#[test]
fn canary_bessel_and_hankel() {
    let u = EPS / 2.0;
    let mut worst = 0.0f64;
    let mut failures = Vec::new();
    for m in 0..2u32 {
        let names = [
            (format!("bessel_j{m}"), false),
            (format!("bessel_j{m}c"), true),
            (format!("bessel_y{m}"), false),
            (format!("bessel_y{m}c"), true),
            (format!("hankel1_{m}"), true),
        ];
        for (name, complex) in names {
            for g in golden(&name) {
                let z = c64::new(g.a, if complex { g.b } else { 0.0 });
                let want = c64::new(g.r, if complex { g.i } else { 0.0 });
                let (j, y) = crate::fdtd::ring::bessel(m, z);
                let (err_j, err_y) = bessel_bound(m, z, j);
                let (ours, bound) = if name.starts_with("hankel") {
                    let hankel = j + c64::new(0.0, 1.0) * y;
                    (
                        hankel,
                        err_j + err_y + u * (portable::complex::abs(j) + portable::complex::abs(y)),
                    )
                } else if name.starts_with("bessel_y") {
                    (y, err_y)
                } else {
                    (j, err_j)
                };
                let allowed = bound + 0.5 * (ulp(want.re) + ulp(want.im));
                let off = portable::complex::abs(ours - want);
                worst = worst.max(off / allowed);
                if off > allowed {
                    failures.push(format!(
                        "{name}({z}) = {ours}, exact {want}: off {off:e} > {allowed:e}"
                    ));
                }
            }
        }
    }
    println!("bessel: worst error {worst:.3} of its bound");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// E₁(x) for 1e-5 ≤ x ≤ 50 (`material::catalogue::conductivity::e1`) against mpmath, within
/// the running error of its own arithmetic (u = 2⁻⁵³). Below 1, −γ − ln x + Σ: the series'
/// k-th term x^k/(k·k!) made by 2k + 1 roundings and N terms summed, ln x within an ulp, and
/// the two additions u each of their parts. From 1 on, Lentz's continued fraction to a step of
/// 1e-16: each level (at most 200) rounds its d, c, step and product 7 times, all carried into
/// the product, and e^(−x) an ulp.
#[test]
fn canary_exponential_integral() {
    let u = EPS / 2.0;
    let euler = std::f64::consts::EULER_GAMMA;
    let mut worst = 0.0f64;
    let mut failures = Vec::new();
    for g in golden("e1") {
        let x = g.a;
        let ours = crate::material::catalogue::conductivity::e1(x);
        let bound = if x < 1.0 {
            let (mut term, mut sum, mut weighted, mut n) = (1.0, 0.0, 0.0, 0.0);
            for k in 1..60 {
                let k = f64::from(k);
                term *= x / k;
                let add = term / k;
                sum += add;
                weighted += add * (2.0 * k + 1.0);
                n = k;
                if add < 1e-17 * sum {
                    break;
                }
            }
            u * (weighted + n * sum)
                + ulp(portable::ln(x))
                + 2.0 * u * (euler + portable::ln(x).abs() + sum)
        } else {
            u * (7.0 * 200.0 + 2.0) * g.r.abs()
        };
        let allowed = bound + 0.5 * ulp(g.r);
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
    v.iter()
        .map(|z| portable::complex::abs(*z))
        .fold(0.0, f64::max)
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
        .map(|r| {
            (0..n)
                .map(|c| portable::complex::abs(dense[(r, c)]))
                .sum::<f64>()
        })
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
        .map(|r| {
            (0..n)
                .map(|c| portable::complex::abs(inv[(r, c)]))
                .sum::<f64>()
        })
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

/// QMR and GMRES on a matrix whose iteration count is known in closed form: A is diagonal with
/// each of k = 20 values λ_j = 1 + j + i/2 on 6 of its entries (exact numbers, the same bits
/// everywhere), so it has k distinct eigenvalues, and b has a part along each eigenspace. In
/// exact arithmetic the Krylov space of (A, b) has dimension k, and both methods find the
/// solution at step k, not before: on a spectrum of condition κ ≈ 20 the best residual
/// polynomial of degree j < k leaves about ((√κ − 1)/(√κ + 1))^j ≈ 0.63^j of b, 1e-4 at
/// j = k − 1, far above the tolerance. The rule: the iteration count within 5 % of k (one
/// iteration either way at k = 20), as the 3D FDFD's count is held against Linux's; and the
/// true residual ‖b − Ax‖/‖b‖ below the tolerance asked. GMRES is held to the same on k values
/// round the circle |λ − 2| = 1 (`portable`'s sin and cos), where its residual halves at each
/// step; QMR isn't, as its non-Hermitian Lanczos process meets near-breakdowns there (wᵀv
/// small against ‖w‖‖v‖) that cost it two steps in finite precision (22 here on Windows).
/// Both solvers' rotations took `Complex64::norm`, the system's hypot, until #822 (they take
/// `portable`'s now): their iterates weren't the same bits on every system, which is why a count
/// is checked and not a hash.
#[test]
fn canary_qmr_and_gmres_iterations_are_the_closed_form() {
    use crate::backend::krylov::{Sparse, Stopping, gmres_preconditioned, qmr};
    let k = 20usize;
    let circle = |j: usize| {
        let (s, c) = crate::portable::sin_cos(std::f64::consts::TAU * j as f64 / k as f64);
        c64::new(2.0 + c, s)
    };
    let line = |j: usize| c64::new(1.0 + j as f64, 0.5);
    let n = 6 * k;
    let b: Vec<c64> = (0..n)
        .map(|i| c64::new(1.0 + (i % 4) as f64 / 8.0, (i % 3) as f64 / 4.0))
        .collect();
    let stopping = Stopping {
        tolerance: 1e-10,
        max_iterations: 200,
    };
    let true_residual = |e: &[(usize, usize, c64)], x: &[c64]| {
        let mut ax = vec![c64::new(0.0, 0.0); n];
        for &(r, c, v) in e {
            ax[r] += v * x[c];
        }
        let num: f64 = b.iter().zip(&ax).map(|(b, a)| (b - a).norm_sqr()).sum();
        let den: f64 = b.iter().map(|b| b.norm_sqr()).sum();
        (num / den).sqrt()
    };
    let entries = |lambda: &dyn Fn(usize) -> c64| -> Vec<(usize, usize, c64)> {
        (0..n).map(|i| (i, i, lambda(i % k))).collect()
    };
    let (on_line, on_circle) = (entries(&line), entries(&circle));
    let (a_line, a_circle) = (
        Sparse::new(n, on_line.clone()),
        Sparse::new(n, on_circle.clone()),
    );
    let runs = [
        ("QMR, line", &on_line, qmr(&a_line, &b, stopping).unwrap()),
        (
            "GMRES, line",
            &on_line,
            gmres_preconditioned(&a_line, &Identity, &b, stopping, 50).unwrap(),
        ),
        (
            "GMRES, circle",
            &on_circle,
            gmres_preconditioned(&a_circle, &Identity, &b, stopping, 50).unwrap(),
        ),
    ];
    let allowed = (0.05 * k as f64).floor() as usize;
    let mut failures = Vec::new();
    for (name, e, (x, c)) in &runs {
        let r = true_residual(e, x);
        println!(
            "{name}: {} iterations (closed form {k}), residual {r:.2e}",
            c.iterations
        );
        if c.iterations.abs_diff(k) > allowed {
            failures.push(format!(
                "{name}: {} iterations, the closed form {k} ± {allowed}",
                c.iterations
            ));
        }
        if r > stopping.tolerance {
            failures.push(format!("{name}: true residual {r:e}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
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
    let a = Mat::from_fn(m, n, |i, j| {
        portable::powi(i as f64 / (m - 1) as f64, j as i32)
    });
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

/// The library's least squares (faer's QR, then faer's thin SVD of R) on exact inputs, bit for
/// bit: the hash of the solution, printed as `FAER r_lstsq <size> <hash>` beside
/// `tests/canary.rs`'s `canary_faer_dense_kernels_bit_for_bit` (faer_bits.rs says how). Asserted:
/// the same bits computed again, and sequential and on 1 to 5 threads (its QR and SVD are faer's,
/// sequential: `crate::dense::fixed`); not asserted: the hashes against another system's (#822).
#[test]
fn canary_faer_least_squares_bit_for_bit() {
    fn items() -> Vec<faer_bits::Item> {
        [(30, 8), (100, 30), (400, 64)]
            .into_iter()
            .map(|(m, n)| {
                let seed = (1000 * m + n) as u64;
                let x = crate::lstsq::least_squares(
                    faer_bits::real(m, n, seed),
                    &faer_bits::real(m, 1, seed + 1),
                );
                faer_bits::Item {
                    name: "r_lstsq".to_owned(),
                    size: format!("{m}x{n}"),
                    hash: faer_bits::Bits::new().values(&x).finish(),
                }
            })
            .collect()
    }
    let Some(ways) =
        faer_bits::every_way("canary_tests::canary_faer_least_squares_bit_for_bit", items)
    else {
        return;
    };
    faer_bits::assert_what_holds_on_one_system(&ways);
}
