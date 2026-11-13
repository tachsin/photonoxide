//! Platform canaries: the building blocks every result rests on, checked on their own, so that
//! Windows, macOS and ARM can run this small set instead of the whole suite. If the blocks
//! agree with their exact values (or with themselves, computed two ways), the methods built on
//! them follow; a failure here points at a primitive, not at a method.
//!
//! `just canaries` runs this file, the `canary_` tests inside the library (`cargo test --lib
//! canary_`: the blocks the public API doesn't reach) and the existing tests the canaries reuse
//! by name; CONTRIBUTING.md, "Platform canaries", says how to add one.
//!
//! Every tolerance is a rule with a basis, never a number picked by hand. Three kinds of
//! reference:
//!
//! - **exact (mpmath)**: the correctly rounded value, from `tests/data/canary/golden.py`
//!   (mpmath at 256 bits, run once; `golden.txt` holds the arguments and results as bits). No
//!   golden value comes from photonoxide or from one system's run.
//! - **by construction**: a bit hash of a computation that is only `+ − × ÷ √` and the pure-Rust
//!   `libm` crate, which IEEE 754 makes the same on every system. Its value was taken on one
//!   system; another system that differs has a primitive that isn't IEEE (a fused multiply-add,
//!   a flag), which is the failure the canary is for.
//! - **self-consistency**: the same quantity two ways on one machine (1, 2 and 5 threads; a
//!   solve and its residual; reciprocity), which needs no stored value at all.
//!
//! | Canary (this file) | Kind | Reference | Tolerance and its basis |
//! |---|---|---|---|
//! | `canary_ieee_arithmetic_and_no_fused_multiply_add` | IEEE 754 | by construction | bit-exact |
//! | `canary_system_libm_within_its_ulps` | system libm, num-complex | exact (mpmath) | ulps of the correctly rounded value, per function (below) |
//! | `canary_portable_functions_bit_for_bit` | `photonoxide::portable` on the libm table's arguments | by construction and exact (mpmath) | bit-exact, per function; and the ulps each one's doc states |
//! | `canary_dense_lu_and_qr_backward_errors` | faer | self-consistency | backward error ≤ 10·n·ε·‖A‖ |
//! | `canary_dense_eigen_and_svd` | faer | exact (mpmath) and self-consistency | Weyl: ≤ 10·n·ε·‖A‖₂; residuals ≤ 10·n·ε·‖A‖ |
//! | `canary_slab_modes_against_their_exact_roots` | mode::slab (libm's sqrt, atan2) | exact (mpmath) | the condition's rounding over its slope, plus the bisection's last step |
//! | `canary_fdfd_tiny_solve` | FDFD, sparse LU, threads | self-consistency | residual and reciprocity ≤ 1e-8 (round-off rule); bit-exact over 1, 2, 5 threads |
//! | `canary_fdtd_2d_bit_hash` | FDTD, threads | by construction | bit-exact, on 1, 2 and 5 threads |
//! | `canary_rayon_reduction_on_any_threads` | rayon | self-consistency | bit-exact over 1, 2, 5 threads |
//! | `canary_job_file_with_crlf_spaces_and_non_ascii_path` | files | self-consistency | the same job as its LF text |
//!
//! The library's own (`src/canary_tests.rs`, and two beside the private modules they test):
//!
//! | Canary (in `src/`) | Kind | Reference | Tolerance and its basis |
//! |---|---|---|---|
//! | `portable::tests::canary_portable_functions_within_their_stated_ulps` | `photonoxide::portable`, every function | exact (mpmath, `tests/data/canary/portable.py`) | the ulps each one's doc states |
//! | `portable::tests::canary_portable_bits_over_a_grid` | `photonoxide::portable`, every function | by construction | bit-exact, per function |
//! | `canary_tests::canary_bessel_and_hankel` | own series (`fdtd::ring`) | exact (mpmath) | the running error of its series as summed |
//! | `canary_tests::canary_exponential_integral` | own series and fraction (`material::catalogue::conductivity`) | exact (mpmath) | the running error of its series or fraction |
//! | `canary_tests::canary_sparse_lu_against_dense` | own multifrontal LU on faer | self-consistency | backward error ≤ 10·n·ε·‖A‖; forward ≤ κ·(both backward errors) |
//! | `canary_tests::canary_qmr_and_gmres_iterations_are_the_closed_form` | QMR, GMRES | closed form | k iterations for k distinct eigenvalues, within 5 % |
//! | `canary_tests::canary_least_squares_and_its_jacobi_fallback` | `lstsq` (faer QR, SVD; Jacobi) | self-consistency | forward error ≤ 10·n·ε·κ; Weyl between the two SVDs |
//! | `expr::tests::canary_erf_to_round_off` | own erf | exact (mpmath) | the series' rounding |
//! | `geometry::region_tests::canary_gamma_to_fifteen_digits` | own Γ (Lanczos) | exact (mpmath) | its doc's 15 digits for the approximation, plus its arithmetic's running error |
//! | `nonlinear::fft::tests::canary_fft_against_the_exact_dft` | own FFT, libm twiddles | exact (mpmath) | Higham's bound for the FFT |
//!
//! The existing tests the canaries reuse, by name (`just canaries` runs them with `--exact`;
//! `canary_reused_tests_exist` below fails when one is renamed or removed):
//!
//! | Test | Where | What it guards |
//! |---|---|---|
//! | `portable::tests::they_are_within_an_ulp_or_two_of_the_systems` | src/portable/tests.rs | the portable exp, sin, cos within 4 ulps of the system's |
//! | `portable::tests::they_give_the_same_bits_on_every_system` | src/portable/tests.rs | their bits, a golden hash |
//! | `fdtd::ring::tests::bessel_functions_match_tabulated_values` | src/fdtd/ring.rs:209 | Bessel J, Y against tables |
//! | `mode::hadley::tests::bessel_matches_known_values` | src/mode/hadley.rs:1065 | Hadley's Bessel J against known values |
//! | `material::catalogue::conductivity::tests::the_exponential_integral_takes_its_known_values_either_side_of_one` | src/material/catalogue/conductivity.rs:834 | E₁ on both branches |
//! | `sparse::multifrontal::tests::the_threaded_product_of_a_fronts_update_is_the_plain_one` | src/sparse/multifrontal.rs:1227 | faer's threaded product (caught Zen 3) |
//! | `backend::dense::tests::faers_kernels_behind_the_trait_give_the_solvers_own_bits` | src/backend/dense.rs:264 | faer's dense kernels behind the trait |
//! | `eigen::schur::tests::the_schur_form_reconstructs_the_matrix_and_has_its_eigenvalues` | src/eigen/schur.rs:306 | Schur form |
//! | `eigen::contour::tests::a_diagonal_pencil_gives_its_eigenvalues_to_rounding` | src/eigen/contour/tests.rs:22 | contour eigensolver |
//! | `eigen::contour::tests::every_eigenvalue_in_a_disc_of_a_non_hermitian_matrix_against_a_dense_solve` | src/eigen/contour/tests.rs:70 | contour against faer's eig |
//! | `sparse::multifrontal::tests::it_solves_and_transposes_by_nested_dissection_or_amd` | src/sparse/multifrontal.rs:1316 | sparse LU residual ≤ 1e-13 |
//! | `sparse::multifrontal::tests::rows_permuted_away_from_the_diagonal_are_matched_back` | src/sparse/multifrontal.rs:1334 | matching |
//! | `sparse::multifrontal::tests::a_zero_pivot_is_perturbed_and_counted` | src/sparse/multifrontal.rs:1365 | pivot perturbation |
//! | `sparse::multifrontal::tests::the_factors_are_the_same_bits_on_any_number_of_threads` | src/sparse/multifrontal.rs:1468 | sparse LU, bits over threads |
//! | `backend::krylov::tests::krylov_solvers_are_the_same_bit_for_bit_on_any_number_of_threads` | src/backend/krylov.rs:1865 | QMR, GMRES, bits over threads |
//! | `backend::krylov::tests::the_threaded_product_is_the_sequential_one_bit_for_bit` | src/backend/krylov.rs:1970 | sparse product, bits over threads |
//! | `backend::krylov::block::tests::block_qmr_is_the_same_bit_for_bit_on_any_number_of_threads` | src/backend/krylov/block_tests.rs:287 | block QMR, bits over threads |
//! | `fdfd::three::tests::qmr_without_the_matrix_takes_the_iterations_and_gives_the_field_of_qmr_with_it` | src/cases/fdfd/three/tests.rs:1795 | matrix-free QMR's iterations |
//! | `nonlinear::fft::tests::matches_the_direct_sum_and_inverts` | src/nonlinear/fft.rs:96 | FFT against the direct sum |
//! | `mode::slab::tests::the_books_slab_gives_its_effective_indices` | src/mode/slab.rs:213 | slab against the book |
//! | `mode::slab::tests::the_roots_satisfy_the_books_tangent_form` | src/mode/slab.rs:235 | slab roots |
//! | `mode::vector::tests::a_uniform_box_has_its_exact_discrete_modes` | src/mode/vector.rs:1857 | full-vector solve, exact discrete modes |
//! | `mode::vector::tests::a_strip_has_a_te_like_fundamental_mode` | src/mode/vector.rs:1948 | full-vector strip |
//! | `eigen::contour::tests::the_same_bits_on_one_and_many_threads` | src/eigen/contour/tests.rs:161 | contour, bits over threads |
//! | `fdfd::tests::a_slab_reflects_and_transmits_as_the_transfer_matrices_say_at_second_order` | src/fdfd/tests.rs:12 | FDFD, second order |
//! | `fdfd::tests::a_step_is_reciprocal_and_a_te_mode_reflects_as_fresnel_says` | src/fdfd/tests.rs:184 | FDFD, Fresnel |
//! | `fdfd::fill::tests::the_fills_values_are_the_same_on_any_number_of_threads` | src/fdfd/fill/tests.rs:46 | FDFD fill, bits over threads |
//! | `fdtd::kernel_tests::the_kernel_is_the_plain_loops_bits_on_every_boundary_and_medium` | src/fdtd/kernel_tests.rs:56 | FDTD kernel against plain loops |
//! | `fdtd::tests::the_fields_are_the_same_bits_on_any_number_of_threads` | src/fdtd/tests.rs:42 | 3D FDTD, bits over threads |
//! | `parallel::tests::the_results_come_in_the_items_order_on_any_number_of_threads` | src/parallel.rs:27 | `map_in_order`'s order |
//! | `circuit::components::tests::spectra_are_the_same_bit_for_bit_on_any_number_of_threads` | src/circuit/components/tests.rs:512 | circuit spectra, bits over threads |
//! | `geometry::array_tests::the_same_bits_on_any_number_of_threads` | src/geometry/array_tests.rs:156 | geometry arrays, bits over threads |
//! | `device::tests::a_solve_gives_the_same_bits_on_one_thread_and_many` | src/device/tests.rs:564 | a device solve, bits over threads |
//! | `compact::touchstone::tests::lines_end_in_lf_crlf_or_cr` | src/compact/touchstone/tests.rs:222 | Touchstone with CRLF and CR |
//! | `export::tables::tests::tables_read_back_to_the_events_bits` | src/export/tables.rs:638 | exported tables read back |
//! | `job::results::tests::a_large_array_goes_to_a_file_with_the_same_bits` | src/job/results.rs:860 | results files' bits |
//! | `job::short_tests::the_api_docs_job_files_are_yaml_that_reads` | src/job/short_tests.rs:413 | job YAML parsing |
//! | `farm::tests::a_worker_refuses_a_wrong_token` | src/farm/tests.rs:739 | farm sockets (#388) |
//! | `farm::tests::a_worker_refuses_what_isnt_a_hello_and_an_overlong_one` | src/farm/tests.rs:773 | farm sockets |
//! | `farm::tests::a_worker_refuses_a_connection_that_says_nothing_within_the_handshake` | src/farm/tests.rs:807 | farm sockets, timeouts |
//! | `farm::tests::whole_jobs_come_back_in_their_order_whichever_finishes_first` | src/farm/tests.rs:411 | farm, order of results |
//!
//! The libm table's rules (`canary_system_libm_within_its_ulps`), in ulps of the correctly
//! rounded value: 1 for exp, ln, sin, cos, atan2 and hypot, the functions glibc, the Microsoft
//! C runtime and Apple's libm all aim to round within an ulp; 2 for the others (exp_m1, ln_1p,
//! log10, tan, asin, acos, atan, sinh, cosh, tanh, powf, cbrt), the largest error glibc's manual
//! lists for them on x86-64 and AArch64. powi is compiler-builtins' square and multiply on
//! Linux (|n| − 1 products and a division for n < 0, each within half an ulp, ≤ |n| − 1 +
//! [n < 0] ulps) but the C runtime's pow on Windows, which is within the same. Complex norm is hypot (1 ulp); complex sqrt,
//! exp and ln are num-complex's formulas over hypot, atan2, sqrt, exp, ln, cos and sin, and
//! their bound is the formula's own first-order error with each call at its rule, computed for
//! each argument (normwise, since a small component of a large result can't be held to its own
//! ulps by these formulas).

// the system's maths functions are what this file measures: an independent reference
#![allow(
    clippy::disallowed_methods,
    reason = "the system's maths library is what the canaries measure"
)]

#[path = "../src/canary_tests/golden.rs"]
mod golden;

use std::hint::black_box;

use golden::{golden, ulp, ulps};
use photonoxide::Complex64 as c64;

const EPS: f64 = f64::EPSILON;

/// Folds a value's bits into a hash, order-sensitive.
fn fold(h: &mut u64, v: f64) {
    *h = h.rotate_left(7) ^ v.to_bits();
}

/// Runs `f` on a rayon pool of `threads` threads.
fn on_threads<R: Send>(threads: usize, f: impl FnOnce() -> R + Send) -> R {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("a thread pool")
        .install(f)
}

// ---------------------------------------------------------------------------------------------
// (a) IEEE 754 arithmetic and no fused multiply-add
// ---------------------------------------------------------------------------------------------

/// `+ − × ÷ √` are exact roundings everywhere, so their bits are the same on every system: the
/// hash below holds wherever no FMA, no x87 precision and no flush-to-zero has leaked in (the
/// same on Windows x86-64 and Linux x86-64). a·b + c with a·b = 1 − 2⁻⁶⁰ is 0 when the product
/// is rounded first and −2⁻⁶⁰ when fused, so a `-C target-cpu=native` or `fp-contract` build
/// fails here, not deep inside a solver.
///
/// powi is not among them. On Linux, LLVM lowers `f64::powi` to compiler-builtins'
/// `__powidf2`, square and multiply; on Windows (the MSVC target) to the C runtime's `pow`,
/// which rounds better and differently: a third of the powers below differ in their last bit
/// between the two. The libm table holds powi to its rule; here, how many of them aren't square
/// and multiply's bits is printed, information that says which one this system has.
#[test]
fn canary_ieee_arithmetic_and_no_fused_multiply_add() {
    // 2^e, exactly, by its bits (powi is not exact everywhere)
    let two_to = |e: i32| f64::from_bits(((1023 + e) as u64) << 52);
    let (a, b, c) = black_box((1.0 + two_to(-30), 1.0 - two_to(-30), -1.0));
    assert_eq!(
        (a * b + c).to_bits(),
        0.0f64.to_bits(),
        "a·b + c was fused (exact −2⁻⁶⁰): an FMA reached the build"
    );
    // subnormals are kept, not flushed to zero
    let tiny = black_box(f64::MIN_POSITIVE);
    assert_eq!(
        (tiny / 4.0).to_bits(),
        1u64 << 50,
        "a subnormal was flushed"
    );

    let mut s: u64 = 0x2026_1113_CA9A_0001;
    let mut next = || {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // 53 random bits, exactly a double in [0, 1)
        (s >> 11) as f64 / 9_007_199_254_740_992.0
    };
    let mut h = 0u64;
    let mut not_square_and_multiply = 0;
    for k in 0..20_000 {
        let u = black_box((next() - 0.5) * two_to((k % 41) - 20));
        let v = black_box((next() + 0.25) * two_to((k % 17) - 8));
        for w in [
            u + v,
            u - v,
            u * v,
            u / v,
            v.sqrt(),
            (u * v + u) / (v - u * 0.5),
        ] {
            fold(&mut h, w);
        }
        let n = black_box((k % 13) - 6);
        if u.powi(n).to_bits() != square_and_multiply(u, n).to_bits() {
            not_square_and_multiply += 1;
        }
    }
    println!(
        "powi: {not_square_and_multiply} of 20000 powers aren't square and multiply's bits \
         (0: compiler-builtins' __powidf2; more: the system's pow)"
    );
    assert_eq!(h, 0xab74_13d0_f5aa_4b5e, "{h:#018x}");
}

/// compiler-builtins' `__powidf2`: x^n by square and multiply, then 1/x^|n| for n < 0.
fn square_and_multiply(x: f64, n: i32) -> f64 {
    let (mut x, mut p, mut acc) = (x, n.unsigned_abs(), 1.0);
    loop {
        if p & 1 == 1 {
            acc *= x;
        }
        p >>= 1;
        if p == 0 {
            break;
        }
        x *= x;
    }
    if n < 0 { 1.0 / acc } else { acc }
}

// ---------------------------------------------------------------------------------------------
// (b) the system's maths library against the correctly rounded values
// ---------------------------------------------------------------------------------------------

/// One function's result over its golden arguments: the largest distance in ulps, how many
/// results aren't the correctly rounded one, and the failures.
struct Measured {
    name: &'static str,
    inputs: usize,
    worst: f64,
    not_rounded: usize,
    rule: String,
    failures: Vec<String>,
}

/// A real function of one or two arguments, held to `rule(a, b)` ulps of the correctly rounded
/// value.
fn real(
    name: &'static str,
    f: impl Fn(f64, f64) -> f64,
    rule: impl Fn(f64, f64) -> u64,
    rule_text: &str,
) -> Measured {
    let rows = golden(name);
    let mut m = Measured {
        name,
        inputs: rows.len(),
        worst: 0.0,
        not_rounded: 0,
        rule: rule_text.to_owned(),
        failures: Vec::new(),
    };
    for g in &rows {
        let ours = f(g.a, g.b);
        let d = ulps(ours, g.r);
        m.worst = m.worst.max(d as f64);
        m.not_rounded += usize::from(d != 0);
        if d > rule(g.a, g.b) {
            m.failures.push(format!(
                "{name}({:e}, {:e}) = {ours:e}, correctly rounded {:e}: {d} ulps",
                g.a, g.b, g.r
            ));
        }
    }
    m
}

/// A complex function held, for each argument, to `bound(z)` (absolute, normwise) beyond the
/// golden value's own rounding (half an ulp of each part); its worst error reported as the
/// fraction of that bound it used (ulps of |w| say little where |w| is small and a part's
/// rounding is not).
fn complex(
    name: &'static str,
    f: impl Fn(c64) -> c64,
    bound: impl Fn(c64) -> f64,
    rule_text: &str,
) -> Measured {
    let rows = golden(name);
    let mut m = Measured {
        name,
        inputs: rows.len(),
        worst: 0.0,
        not_rounded: 0,
        rule: rule_text.to_owned(),
        failures: Vec::new(),
    };
    for g in &rows {
        let z = c64::new(g.a, g.b);
        let want = c64::new(g.r, g.i);
        let ours = f(z);
        let off = (ours - want).norm();
        let allowed = bound(z) + 0.5 * (ulp(want.re) + ulp(want.im));
        m.worst = m.worst.max(off / allowed);
        m.not_rounded += usize::from(ulps(ours.re, want.re) + ulps(ours.im, want.im) != 0);
        if off > allowed {
            m.failures.push(format!(
                "{name}({z}) = {ours}, correctly rounded {want}: off by {off:e}, allowed {allowed:e}"
            ));
        }
    }
    m
}

/// The first-order error of num-complex's exp, e^a (cos b + i sin b), each call within an ulp
/// and each product within half an ulp.
fn exp_budget(z: c64) -> f64 {
    let (e, c, s) = (z.re.exp(), z.im.cos(), z.im.sin());
    let re = ulp(e) * c.abs() + e * ulp(c) + 0.5 * ulp(e * c);
    let im = ulp(e) * s.abs() + e * ulp(s) + 0.5 * ulp(e * s);
    re + im
}

/// The same for ln z = ln|z| + i atan2(b, a): |z| = hypot within an ulp, then ln and atan2
/// within an ulp each.
fn ln_budget(z: c64) -> f64 {
    let r = z.norm();
    ulp(r) / r + ulp(r.ln()) + ulp(z.arg())
}

/// The same for √z = √|z| (cos θ/2 + i sin θ/2), θ = atan2(b, a): hypot, atan2, cos and sin
/// within an ulp, √ and the products within half an ulp (θ/2 is exact).
fn sqrt_budget(z: c64) -> f64 {
    let (r, theta) = (z.norm(), z.arg());
    let s = r.sqrt();
    let (c, sn) = ((theta / 2.0).cos(), (theta / 2.0).sin());
    let ds = 0.5 * ulp(s) + 0.5 * s * ulp(r) / r;
    let dc = ulp(c) + sn.abs() * ulp(theta) / 2.0;
    let dsn = ulp(sn) + c.abs() * ulp(theta) / 2.0;
    let re = ds * c.abs() + s * dc + 0.5 * ulp(s * c);
    let im = ds * sn.abs() + s * dsn + 0.5 * ulp(s * sn);
    re + im
}

/// The rule for powi: |n| − 1 products and, for n < 0, a division, each within half an ulp
/// relative: within |n| − 1 + [n < 0] ulps of the exact power, and so as many of the correctly
/// rounded one. That is square and multiply's bound (Linux); the system's pow that Windows
/// lowers powi to is within it too.
fn powi_rule(n: f64) -> u64 {
    let n = n as i64;
    n.unsigned_abs().saturating_sub(1) + u64::from(n < 0)
}

#[test]
fn canary_system_libm_within_its_ulps() {
    let one = |_: f64, _: f64| 1;
    let two = |_: f64, _: f64| 2;
    let one_text = "1 ulp: all three libraries aim within an ulp";
    let two_text = "2 ulps: glibc's largest listed error";
    let table = vec![
        real("exp", |a, _| a.exp(), one, one_text),
        real("ln", |a, _| a.ln(), one, one_text),
        real("sin", |a, _| a.sin(), one, one_text),
        real("cos", |a, _| a.cos(), one, one_text),
        real("atan2", f64::atan2, one, one_text),
        real("hypot", f64::hypot, one, one_text),
        real("exp_m1", |a, _| a.exp_m1(), two, two_text),
        real("ln_1p", |a, _| a.ln_1p(), two, two_text),
        real("log10", |a, _| a.log10(), two, two_text),
        real("tan", |a, _| a.tan(), two, two_text),
        real("asin", |a, _| a.asin(), two, two_text),
        real("acos", |a, _| a.acos(), two, two_text),
        real("atan", |a, _| a.atan(), two, two_text),
        real("sinh", |a, _| a.sinh(), two, two_text),
        real("cosh", |a, _| a.cosh(), two, two_text),
        real("tanh", |a, _| a.tanh(), two, two_text),
        real("powf", f64::powf, two, two_text),
        real("cbrt", |a, _| a.cbrt(), two, two_text),
        real(
            "powi",
            |a, n| a.powi(n as i32),
            |_, n| powi_rule(n),
            "|n| − 1 + [n < 0] ulps: its products and division",
        ),
        real("c_norm", |a, b| c64::new(a, b).norm(), one, "1 ulp: hypot"),
        complex(
            "c_exp",
            c64::exp,
            exp_budget,
            "its formula's error, each call at its rule (worst: of that)",
        ),
        complex(
            "c_ln",
            c64::ln,
            ln_budget,
            "its formula's error, each call at its rule (worst: of that)",
        ),
        complex(
            "c_sqrt",
            c64::sqrt,
            sqrt_budget,
            "its formula's error, each call at its rule (worst: of that)",
        ),
    ];
    println!(
        "{:<8} {:>6} {:>10} {:>13}  rule",
        "function", "inputs", "worst ulp", "not rounded"
    );
    let mut failures = Vec::new();
    for m in table {
        println!(
            "{:<8} {:>6} {:>10.2} {:>13}  {}",
            m.name, m.inputs, m.worst, m.not_rounded, m.rule
        );
        failures.extend(m.failures);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// (b′) photonoxide::portable on the same arguments, bit for bit
// ---------------------------------------------------------------------------------------------

/// One portable function on its golden arguments: the hash of its results' bits, its worst
/// distance in ulps from the correctly rounded value, and the arguments where it broke its rule.
struct Portable {
    name: &'static str,
    inputs: usize,
    hash: u64,
    worst: u64,
    failures: Vec<String>,
}

/// Runs `f` (a real function's result as the real part) over `name`'s golden rows, holding each
/// result to `rule(row, result)`.
fn portable(
    name: &'static str,
    f: impl Fn(&golden::Golden) -> c64,
    rule: impl Fn(&golden::Golden, c64) -> bool,
) -> Portable {
    let rows = golden(name);
    let mut m = Portable {
        name,
        inputs: rows.len(),
        hash: 0,
        worst: 0,
        failures: Vec::new(),
    };
    for g in &rows {
        let w = f(g);
        fold(&mut m.hash, w.re);
        fold(&mut m.hash, w.im);
        let want_im = if g.i.is_nan() { 0.0 } else { g.i };
        m.worst = m.worst.max(ulps(w.re, g.r).max(ulps(w.im, want_im)));
        if !rule(g, w) {
            m.failures.push(format!(
                "portable {name}({:e}, {:e}) = {w}, correctly rounded {:e}{:+e}i",
                g.a, g.b, g.r, want_im
            ));
        }
    }
    m
}

/// The bound in binary64 numbers from the correctly rounded value of a function within `e` ulps
/// of the true value, as `photonoxide::portable`'s docs state them.
fn within(e: f64) -> impl Fn(&golden::Golden, c64) -> bool {
    move |g, w| ulps(w.re, g.r) <= (e + 0.5).floor() as u64
}

/// `photonoxide::portable` on the arguments of the libm table: per function, the hash of its
/// results' bits (by construction: `+ − × ÷ √` only, the same on every system; taken on Windows
/// x86-64) and its distance from the correctly rounded value, within the ulps its doc states
/// (the module's own canaries hold every function over more arguments). The system's table above
/// shows what the platform does; this one must not move.
#[test]
fn canary_portable_functions_bit_for_bit() {
    use photonoxide::portable::{self as p, complex as pc};
    let r = |f: fn(f64) -> f64| move |g: &golden::Golden| c64::new(f(g.a), 0.0);
    let r2 = |f: fn(f64, f64) -> f64| move |g: &golden::Golden| c64::new(f(g.a, g.b), 0.0);
    let z = |g: &golden::Golden| c64::new(g.a, g.b);
    let u = EPS / 2.0;
    let table = vec![
        portable("exp", r(p::exp), within(0.949)),
        portable("ln", r(p::ln), within(0.946)),
        portable("sin", r(p::sin), within(0.831)),
        portable("cos", r(p::cos), within(0.834)),
        portable("atan2", r2(p::atan2), within(1.55)),
        portable("hypot", r2(p::hypot), within(1.21)),
        portable("exp_m1", r(p::exp_m1), within(0.909)),
        portable("ln_1p", r(p::ln_1p), within(0.900)),
        portable("log10", r(p::log10), within(0.814)),
        portable("tan", r(p::tan), within(1.02)),
        portable("asin", r(p::asin), within(0.981)),
        portable("acos", r(p::acos), within(0.930)),
        portable("atan", r(p::atan), within(0.861)),
        portable("sinh", r(p::sinh), within(1.88)),
        portable("cosh", r(p::cosh), within(1.47)),
        portable("tanh", r(p::tanh), within(2.22)),
        portable("erf", r(p::erf), within(1.02)),
        portable("cbrt", r(p::cbrt), |g, w| ulps(w.re, g.r) == 0),
        portable("powf", r2(p::powf), |g, w| {
            // its doc: an ulp, but 636 within 2⁻¹⁰ of base 1 for exponents above 10⁶
            let near_one = (g.a - 1.0).abs() < 1.0 / 1024.0 && g.b.abs() > 1e6;
            ulps(w.re, g.r) <= if near_one { 636 } else { 1 }
        }),
        portable(
            "powi",
            |g| c64::new(p::powi(g.a, g.b as i32), 0.0),
            |g, w| ulps(w.re, g.r) <= powi_rule(g.b),
        ),
        portable("c_norm", |g| c64::new(pc::abs(z(g)), 0.0), within(1.21)),
        portable(
            "c_exp",
            |g| pc::exp(z(g)),
            |g, w| ulps(w.re, g.r) <= 5 && ulps(w.im, g.i) <= 5,
        ),
        portable(
            "c_sqrt",
            |g| pc::sqrt(z(g)),
            |g, w| {
                let (re, im) = if g.a >= 0.0 { (3, 4) } else { (4, 3) };
                ulps(w.re, g.r) <= re && ulps(w.im, g.i) <= im
            },
        ),
        portable(
            "c_ln",
            |g| pc::ln(z(g)),
            move |g, w| {
                let want = c64::new(g.r, g.i);
                (w - want).norm_sqr().sqrt()
                    <= 15.0 * u * want.norm_sqr().sqrt() + 0.5 * (ulp(g.r) + ulp(g.i))
            },
        ),
    ];
    println!("{:<8} {:>6} {:>6}  bits", "portable", "inputs", "worst");
    let mut failures = Vec::new();
    for m in &table {
        let want = PORTABLE_HASHES
            .iter()
            .find(|(name, _)| *name == m.name)
            .map(|(_, h)| *h);
        let same = want == Some(m.hash);
        println!(
            "{:<8} {:>6} {:>6}  (\"{}\", {:#018x}),{}",
            m.name,
            m.inputs,
            m.worst,
            m.name,
            m.hash,
            if same { "" } else { "  CHANGED" }
        );
        if !same {
            failures.push(format!(
                "portable {}: bits hash {:#018x}, was {want:#x?}",
                m.name, m.hash
            ));
        }
        failures.extend(m.failures.iter().cloned());
    }
    assert_eq!(
        table.len(),
        PORTABLE_HASHES.len(),
        "PORTABLE_HASHES has another number of functions"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The bit hashes of `canary_portable_functions_bit_for_bit`, taken on Windows x86-64.
const PORTABLE_HASHES: &[(&str, u64)] = &[
    ("exp", 0x2d3fe4ad2f39cabf),
    ("ln", 0x68bf82645d0f3d57),
    ("sin", 0xe885ceb0428d91b9),
    ("cos", 0xb67c2d3b3a76636c),
    ("atan2", 0x31dba04580e49bc6),
    ("hypot", 0x063a3fab0338c18b),
    ("exp_m1", 0x68c50c57998fff48),
    ("ln_1p", 0xe12dd426d785c526),
    ("log10", 0x1ab796338ab4f98c),
    ("tan", 0x1b531e34e8bb4582),
    ("asin", 0x1bbe6ac67407fecb),
    ("acos", 0x272e985ff7de78fe),
    ("atan", 0x06271a695d2d2944),
    ("sinh", 0xd476008809ede342),
    ("cosh", 0x003e2f3331beb377),
    ("tanh", 0xb7f81a9ae3d77575),
    ("erf", 0xaa402224c03c10a3),
    ("cbrt", 0x5030daf1f5a30c93),
    ("powf", 0x22d4e0ae36ec2109),
    ("powi", 0xe68468ae531aef2b),
    ("c_norm", 0xe1a43094c368fe85),
    ("c_exp", 0xedaa90f466b26d2d),
    ("c_sqrt", 0xa2b633f40e91fd2f),
    ("c_ln", 0xdf010592f93a6d6b),
];

// ---------------------------------------------------------------------------------------------
// (c) faer's dense kernels on ill-conditioned matrices
// ---------------------------------------------------------------------------------------------

/// The 8 × 8 Hilbert matrix, its entries the binary64 numbers 1/(i + j + 1): κ₂ ≈ 1.5e10.
fn hilbert() -> faer::Mat<f64> {
    faer::Mat::from_fn(8, 8, |i, j| 1.0 / (i + j + 1) as f64)
}

/// A non-symmetric 10 × 10 matrix graded over 2⁻⁹ to 2⁹ by rows and columns, so that partial
/// pivoting and the Hessenberg reduction both have work to do; every entry exact.
fn graded() -> faer::Mat<f64> {
    faer::Mat::from_fn(10, 10, |i, j| {
        let base = ((i * 7 + j * 3) % 11) as f64 - 5.0 + if i == j { 0.5 } else { 0.0 };
        base * 2f64.powi(i as i32 - j as i32)
    })
}

fn frobenius(m: faer::MatRef<'_, f64>) -> f64 {
    m.norm_l2()
}

/// The backward errors of faer's LU with partial pivoting and its Householder QR. The rule:
/// a backward stable factorization has ‖ΔA‖ ≤ c·n·u·‖A‖ with a small c (N. J. Higham,
/// *Accuracy and Stability of Numerical Algorithms*, 2nd ed., SIAM (2002),
/// doi:10.1137/1.9780898718027, Theorems 9.4 and 19.4); 10·n·ε covers c and the growth factor
/// of these matrices with room, and a wrong kernel (the Zen 3 product, #398's SVD) gives O(1).
#[test]
fn canary_dense_lu_and_qr_backward_errors() {
    use faer::linalg::solvers::Solve;
    for (name, a) in [("hilbert", hilbert()), ("graded", graded())] {
        let n = a.nrows();
        let bound = 10.0 * n as f64 * EPS;
        let b = faer::Mat::from_fn(n, 1, |i, _| (i as f64) - 2.5);
        let x = a.partial_piv_lu().solve(&b);
        let r = &b - &a * &x;
        // normwise backward error, ∞-norms
        let inf = |m: faer::MatRef<'_, f64>| {
            (0..m.nrows())
                .map(|i| (0..m.ncols()).map(|j| m[(i, j)].abs()).sum::<f64>())
                .fold(0.0, f64::max)
        };
        let eta = inf(r.as_ref()) / (inf(a.as_ref()) * inf(x.as_ref()) + inf(b.as_ref()));
        assert!(
            eta <= bound,
            "{name}: LU's backward error {eta:e} > {bound:e}"
        );

        let qr = a.qr();
        let q = qr.compute_Q();
        let rr = qr.R().to_owned();
        let back = frobenius((&a - &q * &rr).as_ref()) / frobenius(a.as_ref());
        assert!(back <= bound, "{name}: ‖A − QR‖/‖A‖ = {back:e} > {bound:e}");
        let ortho = frobenius((q.transpose() * &q - faer::Mat::<f64>::identity(n, n)).as_ref());
        assert!(ortho <= bound, "{name}: ‖QᵀQ − I‖ = {ortho:e} > {bound:e}");
        println!("{name}: LU η {eta:.2e}, QR {back:.2e}, QᵀQ {ortho:.2e} (bound {bound:.2e})");
    }
}

/// The Hilbert matrix's eigenvalues and singular values against their exact values (mpmath),
/// and the residuals of faer's SVD and non-symmetric eigensolver. Weyl's inequality bounds a
/// computed eigenvalue or singular value of a backward stable method by ‖ΔA‖₂ ≤ 10·n·ε·‖A‖₂
/// (the rule of [`canary_dense_lu_and_qr_backward_errors`]); the residuals ‖AV − VΛ‖ and
/// ‖A − UΣVᵀ‖ by the same, with ‖V‖ for eigenvectors that aren't orthogonal.
#[test]
fn canary_dense_eigen_and_svd() {
    let h = hilbert();
    let exact: Vec<f64> = golden("hilbert8").iter().map(|g| g.r).collect();
    let norm2 = exact[7];
    let weyl = 10.0 * 8.0 * EPS * norm2;
    let eig = h
        .self_adjoint_eigenvalues(faer::Side::Lower)
        .expect("the Hilbert matrix's eigenvalues");
    let mut sv = h.singular_values().expect("its singular values");
    sv.sort_by(f64::total_cmp);
    for k in 0..8 {
        for (what, v) in [("eigenvalue", eig[k]), ("singular value", sv[k])] {
            assert!(
                (v - exact[k]).abs() <= weyl,
                "hilbert {what} {k}: {v:e} against {:e}, off {:e} > {weyl:e}",
                exact[k],
                (v - exact[k]).abs()
            );
        }
    }

    for (name, a) in [("hilbert", hilbert()), ("graded", graded())] {
        let n = a.nrows();
        let bound = 10.0 * n as f64 * EPS;
        let svd = a.svd().expect("the SVD");
        let s = faer::Mat::from_fn(n, n, |i, j| if i == j { svd.S()[i] } else { 0.0 });
        let back =
            frobenius((&a - svd.U() * &s * svd.V().transpose()).as_ref()) / frobenius(a.as_ref());
        assert!(
            back <= bound,
            "{name}: ‖A − UΣVᵀ‖/‖A‖ = {back:e} > {bound:e}"
        );

        let ac = faer::Mat::from_fn(n, n, |i, j| c64::new(a[(i, j)], 0.0));
        let eig = a.eigen().expect("the eigenvalues");
        let (v, l) = (eig.U(), eig.S());
        let vl = faer::Mat::from_fn(n, n, |i, j| v[(i, j)] * l[j]);
        let res = (&ac * v - &vl).norm_l2();
        let allowed = bound * ac.norm_l2() * v.norm_l2();
        assert!(res <= allowed, "{name}: ‖AV − VΛ‖ = {res:e} > {allowed:e}");
        println!(
            "{name}: SVD {back:.2e}, eig residual {:.2e} (bound {bound:.2e})",
            res / (ac.norm_l2() * v.norm_l2())
        );
    }
}

// ---------------------------------------------------------------------------------------------
// (f) a slab's modes, a tiny FDFD solve
// ---------------------------------------------------------------------------------------------

/// TE0 and TM0 of the book's slab (220 nm of silicon in oxide at 1.55 µm, the case of
/// `mode::slab`'s tests) against the exact roots of their characteristic equations (mpmath, on
/// the same binary64 t and k). The bound: the bisection stops within ε·n₂ of where the computed
/// condition f changes sign, and that is within δf/|f′| of the root, δf the condition's own
/// rounding. f = ht − atan2(q̄, h) − atan2(p̄, h): h t carries the roundings of n², n₂²,
/// their difference (amplified by (n₂² + n²)/(n₂² − n²)), the square root and two products;
/// each atan2 one ulp of itself (2u relative), and half its arguments' relative errors; the
/// two subtractions half an ulp each.
#[test]
fn canary_slab_modes_against_their_exact_roots() {
    use photonoxide::mode::Polarization;
    use photonoxide::mode::slab::Slab;
    use photonoxide::units::{Length, Wavelength};
    let u = EPS / 2.0;
    let (n1, n2) = (1.444f64, 3.473f64);
    let slab = Slab::new(n1, n2, n1, Length::nm(220.0)).unwrap();
    let lambda = Wavelength::um(1.55).unwrap();
    let (t, k) = (Length::nm(220.0).to_um(), lambda.wavenumber());
    for (name, pol) in [
        ("slab_te0", Polarization::Te),
        ("slab_tm0", Polarization::Tm),
    ] {
        let g = golden(name)[0];
        assert_eq!(
            (g.a, g.b),
            (t, k),
            "{name}: golden.py's t and k are not the library's"
        );
        let n = slab.modes(pol, lambda)[0].effective_index();
        let exact = g.r;
        let h = k * (n2 * n2 - exact * exact).sqrt();
        let q = k * (exact * exact - n1 * n1).sqrt();
        let w = match pol {
            Polarization::Te => 1.0,
            Polarization::Tm => (n2 / n1).powi(2),
        };
        let amp = (n2 * n2 + exact * exact) / (n2 * n2 - exact * exact);
        let rel_h = u * (3.0 + amp + 0.5 * (1.0 + amp));
        let rel_q = u * (3.0 + (exact * exact + n1 * n1) / (exact * exact - n1 * n1));
        let atan = (w * q).atan2(h);
        let df = h * t * (rel_h + 2.0 * u)
            + 2.0 * (2.0 * u * atan + 0.5 * (rel_q + rel_h + 2.0 * u))
            + u * (h * t).abs();
        // f′(n) = −k² n t / h − 2 · d atan2(w q, h)/dn, with dq/dn = k² n / q, dh/dn = −k² n / h
        let dq = k * k * exact / q;
        let dh = -k * k * exact / h;
        let datan = (h * w * dq - w * q * dh) / (h * h + w * w * q * q);
        let slope = (dh * t - 2.0 * datan).abs();
        let bound = EPS * n2 + df / slope;
        let off = (n - exact).abs();
        println!("{name}: n {n} against {exact}: off {off:.2e}, bound {bound:.2e}");
        assert!(
            off <= bound,
            "{name}: {n} against {exact}, off {off:e} > {bound:e}"
        );
    }
}

/// A 2D FDFD solve (E along z) of a dielectric disc in a PML'd box, on 1, 2 and 5 threads:
/// the same bits on each (photonoxide's own solvers must, AGENTS.md); its residual and its
/// reciprocity (the field at b from a source at a is the field at a from one at b, both outside
/// the PMLs, where the operator is symmetric) within 1e-8, the round-off rule's ceiling
/// (CONTRIBUTING.md, "Validation cases and their tolerances"): exact identities.
#[test]
fn canary_fdfd_tiny_solve() {
    use photonoxide::fdfd::{Boundaries, Grid, Polarization, Solver2d};
    use photonoxide::units::Wavelength;
    let grid = Grid {
        nx: 40,
        ny: 36,
        dx: 0.05,
        dy: 0.05,
        x0: -1.0,
        y0: -0.9,
    };
    let disc = |x: f64, y: f64| {
        if x * x + y * y < 0.3 * 0.3 {
            c64::new(12.0, 0.0)
        } else {
            c64::new(2.1, 0.0)
        }
    };
    let (a, b) = ((12, 14), (27, 20));
    let point = |(i, j): (usize, usize)| {
        let mut s = vec![c64::new(0.0, 0.0); grid.nx * grid.ny];
        s[j * grid.nx + i] = c64::new(1.0, 0.0);
        s
    };
    let k0 = Wavelength::um(1.55).unwrap().wavenumber();
    let run = || {
        let solver = Solver2d::new(
            grid,
            Polarization::Ez,
            Wavelength::um(1.55).unwrap(),
            disc,
            Boundaries::pml(8),
        )
        .unwrap();
        let (sa, sb) = (point(a), point(b));
        let (fa, fb) = (solver.solve(&sa).unwrap(), solver.solve(&sb).unwrap());
        // solve takes −i k₀ times the source as the system's right-hand side
        let rhs: Vec<c64> = sa.iter().map(|s| c64::new(0.0, -k0) * s).collect();
        let residual = solver.residual(&fa, &rhs).unwrap();
        let bits: Vec<(u64, u64)> = (0..grid.ny)
            .flat_map(|j| (0..grid.nx).map(move |i| (i, j)))
            .map(|(i, j)| fa.at(i, j))
            .map(|v| (v.re.to_bits(), v.im.to_bits()))
            .collect();
        (bits, fa.at(b.0, b.1), fb.at(a.0, a.1), residual)
    };
    let (bits1, gab, gba, residual) = on_threads(1, run);
    for threads in [2, 5] {
        let (bits, ..) = on_threads(threads, run);
        assert!(
            bits == bits1,
            "the field on {threads} threads isn't one thread's"
        );
    }
    let recip = (gab - gba).norm() / gab.norm();
    println!("fdfd: residual {residual:.2e}, reciprocity {recip:.2e}");
    assert!(residual <= 1e-8, "residual {residual:e}");
    assert!(recip <= 1e-8, "reciprocity: {gab} against {gba}, {recip:e}");
}

// ---------------------------------------------------------------------------------------------
// (g) threads, (h) FDTD
// ---------------------------------------------------------------------------------------------

/// A sum over 100 000 values in fixed chunks of 4096, each chunk on rayon's threads through
/// `parallel::map_in_order` and the chunks' sums added in order: the same bits on 1, 2 and 5
/// threads, as the library's sums are taken (`backend::krylov`'s `dot_conj`).
#[test]
fn canary_rayon_reduction_on_any_threads() {
    let values: Vec<f64> = (0..100_000)
        .map(|k| ((k * 7919) % 1000) as f64 / 7.0 - 70.0)
        .collect();
    let chunks: Vec<&[f64]> = values.chunks(4096).collect();
    let sum = || -> f64 {
        photonoxide::parallel::map_in_order(&chunks, |c| c.iter().sum::<f64>())
            .into_iter()
            .sum()
    };
    let one = on_threads(1, sum);
    for threads in [2, 5] {
        let s = on_threads(threads, sum);
        assert_eq!(
            s.to_bits(),
            one.to_bits(),
            "{threads} threads: {s} against {one}"
        );
    }
}

/// 200 steps of a 2D FDTD run (E along z, one cell thick and periodic along z, walls across x
/// and y: no CPML, whose profile calls the system's powf and exp) of a dielectric strip lit by
/// a Gaussian pulse: its update is `+ − × ÷` and its source the `libm` crate's exp and sincos
/// (`portable`), so its fields are the same bits on every system and on 1, 2 and 5 threads.
/// The hash was taken on Windows x86-64; a system that differs has a libm call or a fused
/// operation in the update.
#[test]
fn canary_fdtd_2d_bit_hash() {
    use photonoxide::fdfd::{Axis, Edges, Grid3d};
    use photonoxide::fdtd::{Boundaries, Field, Simulation, Source, Waveform};
    use photonoxide::units::Frequency;
    let grid = Grid3d {
        nx: 48,
        ny: 40,
        nz: 1,
        dx: 0.05,
        dy: 0.05,
        dz: 0.05,
        x0: -1.2,
        y0: -1.0,
        z0: -0.025,
    };
    let boundaries = Boundaries {
        z: Edges::Bloch { k: 0.0 },
        ..Boundaries::walls()
    };
    let run = || {
        let mut s = Simulation::new(
            grid,
            |x, y, _| if y.abs() < 0.2 && x > -0.3 { 6.25 } else { 1.0 },
            boundaries,
            0.9,
        )
        .unwrap();
        s.add_source(Source {
            field: Field::E,
            component: Axis::Z,
            at: (12, 20, 0),
            waveform: Waveform::Gaussian {
                frequency: Frequency::natural(1.0).unwrap(),
                width: 0.6,
                delay: 1.8,
            },
        })
        .unwrap();
        s.run(200);
        let mut h = 0u64;
        for v in s.e(Axis::Z).iter().chain(s.h(Axis::X)).chain(s.h(Axis::Y)) {
            fold(&mut h, *v);
        }
        let peak = s.e(Axis::Z).iter().fold(0.0f64, |m, v| m.max(v.abs()));
        (h, peak)
    };
    let (one, peak) = on_threads(1, run);
    assert!(
        peak > 1e-6 && peak.is_finite(),
        "the run carries a field: {peak}"
    );
    for threads in [2, 5] {
        assert_eq!(on_threads(threads, run).0, one, "{threads} threads");
    }
    assert_eq!(one, 0x7c8d_b7ae_6998_0888, "{one:#018x}");
}

// ---------------------------------------------------------------------------------------------
// (i) files: CRLF, spaces and a non-ASCII path
// ---------------------------------------------------------------------------------------------

/// A job file written with CRLF line endings, a multi-line description, in a folder whose name
/// has spaces and non-ASCII letters, read from disk: the same job as its LF text read from a
/// string, and one `job::check` passes. Git on Windows checks text out with CRLF
/// (`core.autocrlf`), and users' folders have such names.
#[test]
fn canary_job_file_with_crlf_spaces_and_non_ascii_path() {
    use photonoxide::run::{Format, Job};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(root.join("jobs/strip-modes.yaml"))
        .unwrap()
        .replace("\r\n", "\n")
        .replacen(
            "timeout_minutes:",
            "description: |\n  The modes of a strip,\n  read from a CRLF file.\ntimeout_minutes:",
            1,
        );
    let dir =
        std::env::temp_dir().join(format!("photonoxide canary Ωμέγα ü {}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("strip modes é.yaml");
    std::fs::write(&path, text.replace('\n', "\r\n")).unwrap();
    let from_disk = Job::load(&path);
    let _ = std::fs::remove_dir_all(&dir);
    let from_disk = from_disk.unwrap();
    let plain = Job::parse_as(&text, Format::Yaml).unwrap();
    assert_eq!(from_disk.name(), plain.name());
    assert_eq!(
        from_disk.description(),
        Some("The modes of a strip,\nread from a CRLF file.\n"),
        "the description kept a carriage return"
    );
    assert_eq!(from_disk.description(), plain.description());
    assert_eq!(from_disk.timeout(), plain.timeout());
    assert_eq!(from_disk.task(), plain.task());
    assert_eq!(from_disk.dir(), Some(dir.as_path()));
    photonoxide::job::check(&from_disk).unwrap();
}

// ---------------------------------------------------------------------------------------------
// the reused tests are still there
// ---------------------------------------------------------------------------------------------

/// Every test `just canaries` runs by name still exists: `cargo test --exact` with a name that
/// matches nothing runs nothing and passes, so a rename would drop a canary silently. The names
/// are read from the justfile's `canaries` recipe, the one list; each one's last part must be a
/// `fn` under `src/`, and each must be in this file's table.
#[test]
fn canary_reused_tests_exist() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let justfile = std::fs::read_to_string(root.join("justfile")).unwrap();
    let names = reused_names(&justfile);
    assert!(
        names.len() >= 30,
        "the justfile's canaries recipe: {names:?}"
    );
    let mut missing = Vec::new();
    for name in &names {
        let f = name.rsplit("::").next().unwrap();
        if !source_has_fn(&root.join("src"), f) {
            missing.push(name.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "tests `just canaries` names that no longer exist: {missing:?}"
    );
    // and the module docs' table lists the same names
    let me = std::fs::read_to_string(root.join("tests/canary.rs")).unwrap();
    for name in &names {
        assert!(
            me.contains(&format!("`{name}`")),
            "{name} isn't in canary.rs's table"
        );
    }
}

/// The names after `--exact` in the justfile's `canaries` recipe, to its end (its lines are
/// continued with a backslash).
fn reused_names(justfile: &str) -> Vec<String> {
    let mut lines = justfile.lines().skip_while(|l| !l.starts_with("canaries"));
    lines.next();
    lines
        .take_while(|l| l.starts_with(' ') || l.starts_with('\t'))
        .skip_while(|l| !l.contains("--exact"))
        .flat_map(str::split_whitespace)
        .filter(|w| w.contains("::"))
        .map(str::to_owned)
        .collect()
}

fn source_has_fn(dir: &std::path::Path, name: &str) -> bool {
    let needle = format!("fn {name}(");
    std::fs::read_dir(dir).unwrap().any(|e| {
        let p = e.unwrap().path();
        if p.is_dir() {
            source_has_fn(&p, name)
        } else {
            p.extension().is_some_and(|x| x == "rs")
                && std::fs::read_to_string(&p).is_ok_and(|t| t.contains(&needle))
        }
    })
}
