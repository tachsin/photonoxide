use super::*;
use crate::fdfd::krylov::{
    Preconditioner, Sparse, Stopping, gmres_preconditioned, qmr, qmr_preconditioned,
};
use crate::fdfd::three::{Boundaries3d, Grid3d, Solver3d};
use crate::units::Wavelength;

fn norm(v: &[c64]) -> f64 {
    v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt()
}

/// The benchmark problems of docs/methods/fdfd-3d.md: the 40³ silicon guide 100 nm across
/// through the PMLs ("guide"), or Shin and Fan's Diel, smaller ("diel"): the grid, the
/// permittivity and the current.
pub(super) fn benchmark(
    case: &str,
    pml: usize,
) -> (Grid3d, impl Fn(f64, f64, f64) -> c64 + Copy, Vec<c64>) {
    let h = 0.01;
    let n: [usize; 3] = match case {
        "diel" => [40, 70 + 2 * pml, 60 + 2 * pml],
        _ => [20 + 2 * pml; 3],
    };
    let grid = Grid3d {
        nx: n[0],
        ny: n[1],
        nz: n[2],
        dx: h,
        dy: h,
        dz: h,
        x0: -(n[0] as f64) * h / 2.0,
        y0: -(n[1] as f64) * h / 2.0,
        z0: -(n[2] as f64) * h / 2.0,
    };
    let (wy, wz) = if case == "diel" {
        (0.2, 0.15)
    } else {
        (0.05, 0.05)
    };
    let vacuum = case == "vacuum";
    let guide = move |_: f64, y: f64, z: f64| {
        c64::new(
            if y.abs() < wy && z.abs() < wz && !vacuum {
                12.09
            } else {
                1.0
            },
            0.0,
        )
    };
    let mut source = vec![c64::new(0.0, 0.0); grid.unknowns()];
    if case == "diel" {
        for k in 0..grid.nz {
            for j in 0..grid.ny {
                let [_, y, z] = grid.e_position(Axis::Y, (15, j, k));
                if y.abs() < wy && z.abs() < wz {
                    source[grid.index(Axis::Y, (15, j, k))] = c64::new(1.0, 0.0);
                }
            }
        }
    } else {
        source[grid.index(Axis::X, (n[0] / 2 + 3, n[1] / 2 + 3, n[2] / 2 + 3))] =
            c64::new(1.0, 0.0);
    }
    (grid, guide, source)
}

/// The lattice, the permittivity, Shin and Fan's matrix and the right-hand side for `current`.
fn system(
    grid: Grid3d,
    eps: impl Fn(f64, f64, f64) -> c64,
    boundaries: Boundaries3d,
    current: &[c64],
) -> (Lattice, Vec<c64>, Sparse, Vec<c64>) {
    let (lattice, eps) =
        Solver3d::setup(grid, Wavelength::um(1.55).unwrap(), eps, boundaries).unwrap();
    let matrix = Sparse::new(
        grid.unknowns(),
        lattice
            .assemble_with(&eps, -1.0)
            .into_iter()
            .map(|t| (t.row, t.col, t.val)),
    );
    let i_k0 = c64::new(0.0, -lattice.k0);
    let b: Vec<c64> = (0..current.len())
        .map(|r| {
            if lattice.fixed(r) {
                c64::new(0.0, 0.0)
            } else {
                i_k0 * current[r]
            }
        })
        .collect();
    let b = lattice.transformed_rhs(&eps, &b, -1.0);
    (lattice, eps, matrix, b)
}

/// The cycle as a solver of its own operator: x ← x + B (b − A x), the residual's norm after
/// each of `cycles` cycles, relative to b's.
fn as_solver(h: &Hierarchy, a: &Sparse, b: &[c64], cycles: usize, transpose: bool) -> Vec<f64> {
    let mut x = vec![c64::new(0.0, 0.0); b.len()];
    let mut out = Vec::new();
    for _ in 0..cycles {
        let ax = if transpose {
            a.apply_transpose(&x)
        } else {
            a.apply(&x)
        };
        let r: Vec<c64> = b.iter().zip(&ax).map(|(p, q)| p - q).collect();
        let d = if transpose {
            h.solve_transpose(&r)
        } else {
            h.solve(&r)
        };
        for (xk, dk) in x.iter_mut().zip(&d) {
            *xk += dk;
        }
        let ax = if transpose {
            a.apply_transpose(&x)
        } else {
            a.apply(&x)
        };
        let r: Vec<c64> = b.iter().zip(&ax).map(|(p, q)| p - q).collect();
        out.push(norm(&r) / norm(b));
    }
    out
}

/// The mean reduction per cycle over the last `last` cycles of a residual history.
fn factor(history: &[f64], last: usize) -> f64 {
    let n = history.len();
    (history[n - 1] / history[n - 1 - last]).powf(1.0 / last as f64)
}

#[test]
#[ignore = "multigrid experiments, for the docs: MG_CASE=guide|diel cargo test --release fdfd::three::multigrid::tests::experiment -- --ignored --nocapture"]
fn experiment() {
    let var = |name: &str, default: &str| std::env::var(name).unwrap_or_else(|_| default.into());
    let case = var("MG_CASE", "guide");
    let pml: usize = var("MG_PML", "10").parse().unwrap();
    let shift: f64 = var("MG_SHIFT", "0.5").parse().unwrap();
    let pre: usize = var("MG_PRE", "0").parse().unwrap();
    let post: usize = var("MG_POST", "1").parse().unwrap();
    let coarsest: usize = var("MG_COARSEST", "2000").parse().unwrap();
    let shape = match var("MG_SHAPE", "V").as_str() {
        "F" => CycleShape::F,
        "W" => CycleShape::W,
        _ => CycleShape::V,
    };
    let stretch: f64 = var("MG_STRETCH", "1").parse().unwrap();
    let qmr_too = var("MG_QMR", "1") == "1";
    let (grid, eps, current) = benchmark(&case, pml);
    let mut boundaries = Boundaries3d {
        real_stretch: stretch,
        ..Boundaries3d::pml(pml)
    };
    boundaries.reflection = var("MG_R", "1e-8").parse().unwrap();
    let k = crate::fdfd::Edges::Bloch { k: 0.0 };
    match var("MG_BC", "pml").as_str() {
        "periodic" => (boundaries.x, boundaries.y, boundaries.z) = (k, k, k),
        "x" => (boundaries.y, boundaries.z) = (k, k),
        _ => {}
    }
    let t = std::time::Instant::now();
    let (lattice, eps, matrix, b) = system(grid, eps, boundaries, &current);
    println!(
        "MG {case}: {} unknowns, setup {:?}",
        grid.unknowns(),
        t.elapsed()
    );
    let options = Multigrid {
        shift,
        shape,
        pre,
        post,
        coarsest,
        restart: var("MG_RESTART", "40").parse().unwrap(),
    };
    let t = std::time::Instant::now();
    let operator = shifted(&lattice, &eps, &matrix, shift);
    let h = Hierarchy::new(&lattice, &eps, operator, options).unwrap();
    println!(
        "MG {case}: {options:?}, levels {:?}, built in {:?}",
        h.shapes(),
        t.elapsed()
    );
    let shifted_matrix = shifted(&lattice, &eps, &matrix, shift);
    let t = std::time::Instant::now();
    let history = as_solver(&h, &shifted_matrix, &b, 12, false);
    println!(
        "MG {case}: as a solver of its own operator, {:?} per cycle: {:?}, factor {:.3}",
        t.elapsed() / 12,
        history
            .iter()
            .map(|r| format!("{r:.1e}"))
            .collect::<Vec<_>>(),
        factor(&history, 6)
    );
    if var("MG_WHERE", "0") == "1" {
        // where the residual left after 30 cycles lies: by depth into the PMLs and component
        let mut x = vec![c64::new(0.0, 0.0); b.len()];
        for _ in 0..30 {
            let ax = shifted_matrix.apply(&x);
            let r: Vec<c64> = b.iter().zip(&ax).map(|(p, q)| p - q).collect();
            let d = h.solve(&r);
            for (xk, dk) in x.iter_mut().zip(&d) {
                *xk += dk;
            }
        }
        let ax = shifted_matrix.apply(&x);
        let r: Vec<c64> = b.iter().zip(&ax).map(|(p, q)| p - q).collect();
        let mut by_depth = vec![[0.0f64; 3]; pml + 2];
        for (q, rq) in r.iter().enumerate() {
            let (c, at) = grid.at(q);
            // the depth into the deepest PML: 0 outside them
            let depth = Axis::ALL
                .iter()
                .map(|&a| {
                    let (m, n) = (at[a.index()], grid.n(a));
                    let low = pml.saturating_sub(m);
                    let high = (m + pml + 1).saturating_sub(n);
                    low.max(high)
                })
                .max()
                .unwrap_or(0)
                .min(pml + 1);
            by_depth[depth][c.index()] += rq.norm_sqr();
        }
        let total: f64 = by_depth.iter().flatten().sum();
        for (d, v) in by_depth.iter().enumerate() {
            println!(
                "MG {case}: residual at depth {d}: x {:.3} y {:.3} z {:.3}",
                v[0] / total,
                v[1] / total,
                v[2] / total
            );
        }
    }
    if !qmr_too {
        return;
    }
    let stop = |tolerance: f64| Stopping {
        tolerance,
        max_iterations: 20_000,
    };
    let t = std::time::Instant::now();
    let (reference, how) = qmr_preconditioned(&matrix, &h, &b, stop(1e-12)).unwrap();
    println!(
        "MG {case}: QMR + MG to 1e-12: {} iterations, {:?}",
        how.iterations,
        t.elapsed()
    );
    let error = |x: &[c64]| {
        let d: Vec<c64> = x.iter().zip(&reference).map(|(p, q)| p - q).collect();
        norm(&d) / norm(&reference)
    };
    for tolerance in [1e-6, 1e-8, 1e-10] {
        let t = std::time::Instant::now();
        let (x, how) = qmr_preconditioned(&matrix, &h, &b, stop(tolerance)).unwrap();
        println!(
            "MG {case}: QMR + MG, {tolerance:e}: {} iterations, {:?}, error {:.1e}",
            how.iterations,
            t.elapsed(),
            error(&x)
        );
    }
    for tolerance in [1e-6, 1e-8, 1e-10] {
        let t = std::time::Instant::now();
        let (x, how) =
            gmres_preconditioned(&matrix, &h, &b, stop(tolerance), options.restart).unwrap();
        println!(
            "MG {case}: GMRES({}) + MG, {tolerance:e}: {} iterations, {:?}, error {:.1e}",
            options.restart,
            how.iterations,
            t.elapsed(),
            error(&x)
        );
    }
    if var("MG_ILU", "0") == "1" {
        let t = std::time::Instant::now();
        let ilu = crate::fdfd::krylov::Ilu0::new(&matrix).unwrap();
        let built = t.elapsed();
        for tolerance in [1e-8, 1e-10] {
            let t = std::time::Instant::now();
            let (x, how) = qmr_preconditioned(&matrix, &ilu, &b, stop(tolerance)).unwrap();
            println!(
                "MG {case}: QMR + ILU(0) (built in {built:?}), {tolerance:e}: {} iterations, {:?}, error {:.1e}",
                how.iterations,
                t.elapsed(),
                error(&x)
            );
        }
    }
    if var("MG_PLAIN", "0") == "1" {
        let t = std::time::Instant::now();
        let (x, how) = qmr(&matrix, &b, stop(1e-8)).unwrap();
        println!(
            "MG {case}: plain QMR, 1e-8: {} iterations, {:?}, error {:.1e}",
            how.iterations,
            t.elapsed(),
            error(&x)
        );
    }
}

/// A small box for the tests CI runs: `n`³ cells of 20 nm at 1.55 µm, a silicon core 120 nm
/// across along x if `silicon`, and a point current off its centre.
fn small(n: usize, silicon: bool) -> (Grid3d, impl Fn(f64, f64, f64) -> c64 + Copy, Vec<c64>) {
    let h = 0.02;
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
            if silicon && y.abs() < 0.06 && z.abs() < 0.06 {
                12.09
            } else {
                1.0
            },
            0.0,
        )
    };
    let mut current = vec![c64::new(0.0, 0.0); grid.unknowns()];
    current[grid.index(Axis::X, (n / 2 + 1, n / 2 + 1, n / 2))] = c64::new(1.0, 0.0);
    (grid, eps, current)
}

/// Values to multiply by, the same on every run: a linear congruential sequence in the unit
/// square of the complex plane, about its centre.
fn sequence(n: usize, seed: u64) -> Vec<c64> {
    let mut s = seed;
    let mut next = move || {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (s >> 11) as f64 / (1u64 << 53) as f64 - 0.5
    };
    (0..n).map(|_| c64::new(next(), next())).collect()
}

/// uᵀ v, without conjugation: the product a matrix's transpose is defined by.
fn bilinear(u: &[c64], v: &[c64]) -> c64 {
    u.iter().zip(v).map(|(p, q)| p * q).sum()
}

fn options(shape: CycleShape, pre: usize, post: usize) -> Multigrid {
    Multigrid {
        shift: 0.5,
        shape,
        pre,
        post,
        coarsest: 300,
        restart: 40,
    }
}

#[test]
fn each_coarse_operator_is_pt_a_p() {
    // silicon and stretched PMLs: every part of the transfer is in play
    let (grid, eps, current) = small(12, true);
    let (lattice, eps, matrix, _) = system(grid, eps, Boundaries3d::stretched_pml(3), &current);
    let h = Hierarchy::new(&lattice, &eps, matrix, options(CycleShape::V, 0, 1)).unwrap();
    assert!(h.shapes().len() >= 3, "{:?}", h.shapes());
    for (fine, coarse) in h.levels.iter().zip(&h.levels[1..]) {
        let v = sequence(coarse.matrix.size(), 7);
        let direct = coarse.matrix.apply(&v);
        let through = fine
            .restriction
            .apply(&fine.matrix.apply(&fine.prolongation.apply(&v)));
        // a coarse value no fine one reaches has an identity row in place of Pᵀ A P's empty one
        let reached = |c: usize| fine.restriction.row(c).next().is_some();
        let mut off = 0.0f64;
        for c in 0..v.len() {
            let expected = if reached(c) { through[c] } else { v[c] };
            off = off.max((direct[c] - expected).norm());
        }
        let scale = direct.iter().map(|z| z.norm()).fold(0.0, f64::max);
        assert!(off < 1e-12 * scale, "{off:e} of {scale:e}");
    }
}

#[test]
fn the_transposed_cycle_is_the_cycles_transpose() {
    // QMR needs M⁻ᵀ: uᵀ (M⁻¹ v) = vᵀ (M⁻ᵀ u) for any u and v, whatever the cycle
    let (grid, eps, current) = small(12, true);
    let (lattice, eps, matrix, _) = system(grid, eps, Boundaries3d::stretched_pml(3), &current);
    let (u, v) = (sequence(matrix.size(), 1), sequence(matrix.size(), 2));
    for (shape, pre, post) in [
        (CycleShape::V, 0, 1),
        (CycleShape::V, 2, 1),
        (CycleShape::F, 1, 1),
        (CycleShape::W, 1, 0),
    ] {
        let operator = shifted(&lattice, &eps, &matrix, 0.5);
        let h = Hierarchy::new(&lattice, &eps, operator, options(shape, pre, post)).unwrap();
        let forward = bilinear(&u, &h.solve(&v));
        let backward = bilinear(&v, &h.solve_transpose(&u));
        assert!(
            (forward - backward).norm() < 1e-10 * forward.norm(),
            "{shape:?}({pre}, {post}): {forward} against {backward}"
        );
    }
}

#[test]
fn the_cycle_reduces_the_residual_of_its_own_operator() {
    // as a solver of the operator it is built on, per cycle of V(1, 1): vacuum with periodic
    // sides, with walls, and silicon in stretched PMLs
    let periodic = {
        let k = crate::fdfd::Edges::Bloch { k: 0.0 };
        Boundaries3d {
            x: k,
            y: k,
            z: k,
            ..Boundaries3d::pml(0)
        }
    };
    for (what, silicon, boundaries, bound) in [
        ("vacuum, periodic", false, periodic, 0.15),
        ("vacuum, walls", false, Boundaries3d::pml(0), 0.15),
        (
            "silicon, stretched PMLs",
            true,
            Boundaries3d::stretched_pml(4),
            0.65,
        ),
    ] {
        let (grid, eps, current) = small(16, silicon);
        let (lattice, eps, matrix, b) = system(grid, eps, boundaries, &current);
        let operator = shifted(&lattice, &eps, &matrix, 0.5);
        let h = Hierarchy::new(&lattice, &eps, operator, options(CycleShape::V, 1, 1)).unwrap();
        let operator = shifted(&lattice, &eps, &matrix, 0.5);
        for transpose in [false, true] {
            let history = as_solver(&h, &operator, &b, 8, transpose);
            let per_cycle = factor(&history, 5);
            assert!(
                per_cycle < bound,
                "{what}: {per_cycle} per cycle, {history:?}"
            );
        }
    }
}

#[test]
fn the_cycle_is_the_same_bit_for_bit_on_any_number_of_threads() {
    let (grid, eps, current) = small(14, true);
    let run = |threads: usize| -> Vec<c64> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        pool.install(|| {
            let (lattice, eps, matrix, b) =
                system(grid, eps, Boundaries3d::stretched_pml(3), &current);
            let operator = shifted(&lattice, &eps, &matrix, 0.5);
            let h = Hierarchy::new(&lattice, &eps, operator, options(CycleShape::F, 1, 1)).unwrap();
            let mut out = h.solve(&b);
            out.extend(h.solve_transpose(&b));
            out
        })
    };
    let one = run(1);
    for threads in [2, 4, 5, 20] {
        let many = run(threads);
        let same = one
            .iter()
            .zip(&many)
            .all(|(p, q)| p.re.to_bits() == q.re.to_bits() && p.im.to_bits() == q.im.to_bits());
        assert!(same, "{threads} threads differ from one");
    }
}

#[test]
fn gmres_with_the_cycle_takes_fewer_iterations_than_qmr_with_ilu0_and_agrees() {
    let (grid, eps, current) = small(16, true);
    let (lattice, eps, matrix, b) = system(grid, eps, Boundaries3d::stretched_pml(4), &current);
    let stop = Stopping {
        tolerance: 1e-10,
        max_iterations: 5000,
    };
    let operator = shifted(&lattice, &eps, &matrix, 0.5);
    let h = Hierarchy::new(&lattice, &eps, operator, options(CycleShape::V, 0, 1)).unwrap();
    // GMRES with the cycle, as the solver runs it, and QMR with it: the same field, GMRES in
    // no more iterations (it minimizes the residual QMR only nearly does), each with one cycle
    // where QMR's take two
    let (with_cycle, cycle) = gmres_preconditioned(&matrix, &h, &b, stop, 40).unwrap();
    let (by_qmr, qmr_cycle) = qmr_preconditioned(&matrix, &h, &b, stop).unwrap();
    assert!(cycle.iterations <= qmr_cycle.iterations);
    let d: Vec<c64> = with_cycle.iter().zip(&by_qmr).map(|(p, q)| p - q).collect();
    assert!(norm(&d) < 1e-7 * norm(&by_qmr));
    let ilu = crate::fdfd::krylov::Ilu0::new(&matrix).unwrap();
    let (with_ilu, ilu) = qmr_preconditioned(&matrix, &ilu, &b, stop).unwrap();
    let d: Vec<c64> = with_cycle
        .iter()
        .zip(&with_ilu)
        .map(|(p, q)| p - q)
        .collect();
    assert!(
        cycle.iterations < ilu.iterations,
        "{} against {}",
        cycle.iterations,
        ilu.iterations
    );
    assert!(norm(&d) < 1e-7 * norm(&with_ilu));
}

#[test]
fn the_coarsest_level_is_factorized_and_solved_by_its_backend() {
    use crate::backend::tests::Recording;
    use crate::backend::{Choice, direct};
    let recording = Recording::register("recording-multigrid");
    let solver = direct(&Choice::parse("recording-multigrid").unwrap()).unwrap();
    let (grid, eps, current) = small(12, true);
    let pml = Boundaries3d::stretched_pml(3);
    let (lattice, eps, matrix, _) = system(grid, eps, pml, &current);
    let again = system(grid, |x, y, z| small(12, true).1(x, y, z), pml, &current).2;
    let options = options(CycleShape::V, 0, 1);
    let own = Hierarchy::new(&lattice, &eps, again, options).unwrap();
    let routed = Hierarchy::new_on(&lattice, &eps, matrix, options, &solver).unwrap();
    // one analysis and one factorization, of the coarsest level's matrix
    assert_eq!(recording.counts(), [1, 1, 0, 0]);
    // a cycle and a transposed cycle: one coarsest solve each, the bits of photonoxide's own
    let b = sequence(grid.unknowns(), 3);
    assert_eq!(
        Preconditioner::solve(&routed, &b),
        Preconditioner::solve(&own, &b)
    );
    assert_eq!(recording.counts(), [1, 1, 1, 0]);
    assert_eq!(
        Preconditioner::solve_transpose(&routed, &b),
        Preconditioner::solve_transpose(&own, &b)
    );
    assert_eq!(recording.counts(), [1, 1, 1, 1]);
}
