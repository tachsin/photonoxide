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
) -> (
    Grid3d,
    impl Fn(f64, f64, f64) -> c64 + Copy + use<>,
    Vec<c64>,
) {
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
        ..Multigrid::default()
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
        ..Multigrid::default()
    }
}

#[test]
fn the_transfers_carry_a_bloch_sides_phase_on_every_level() {
    // a Bloch wave e^(ikx) on each level, k far from 0, interpolated from the next: the period
    // a level's phase takes is the box's, not its cells times the finest step
    let n = 16;
    let k = 7.0;
    let bloch = |k: f64| crate::fdfd::Edges::Bloch { k };
    let boundaries = Boundaries3d {
        x: bloch(k),
        y: bloch(0.0),
        z: bloch(0.0),
        ..Boundaries3d::pml(0)
    };
    let (grid, eps, current) = small(n, false);
    let (lattice, eps, matrix, _) = system(grid, eps, boundaries, &current);
    for kind in [Coarse::Galerkin, Coarse::Rediscretized] {
        let options = Multigrid {
            coarse: kind,
            ..options(CycleShape::V, 0, 1)
        };
        let operator = shifted(&lattice, &eps, &matrix, 0.5);
        let h = Hierarchy::new(&lattice, &eps, operator, options).unwrap();
        assert!(h.levels.len() >= 2, "{:?}", h.shapes());
        for (l, level) in h.levels.iter().enumerate() {
            let [fine, coarse] = [h.shapes()[l], h.shapes()[l + 1]];
            // E_y at the nodes along x: node i of a level of m cells at x0 + i L/m
            let wave = |s: [usize; 3]| -> Vec<c64> {
                let cells = s[0] * s[1] * s[2];
                let mut v = vec![c64::new(0.0, 0.0); 3 * cells];
                for q in 0..cells {
                    let i = q % s[0];
                    let x = grid.x0 + i as f64 * grid.dx * n as f64 / s[0] as f64;
                    v[cells + q] = c64::from_polar(1.0, k * x);
                }
                v
            };
            let through = level.prolong(&wave(coarse), false);
            let exact = wave(fine);
            let worst = through
                .iter()
                .zip(&exact)
                .map(|(p, q)| (p - q).norm())
                .fold(0.0, f64::max);
            // linear interpolation's error, (k Δ)²/8 for Δ the coarse step
            let spacing = grid.dx * n as f64 / coarse[0] as f64;
            assert!(
                worst < (k * spacing).powi(2) / 8.0 * 1.01,
                "{kind:?} level {l}: {worst}"
            );
        }
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
        let v = sequence(coarse.stage.matrix().unwrap().size(), 7);
        let direct = coarse.stage.matrix().unwrap().apply(&v);
        let through = fine.restriction.apply(
            &fine
                .stage
                .matrix()
                .unwrap()
                .apply(&fine.prolongation.apply(&v)),
        );
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

/// The hierarchy without the finest matrix, its coarsest level by photonoxide's own solver.
fn from_stencil(lattice: &Lattice, eps: &[c64], options: Multigrid) -> Hierarchy {
    let solver = crate::backend::direct(&crate::backend::Choice::Auto).unwrap();
    Hierarchy::from_stencil_on(lattice, eps, options, &solver).unwrap()
}

#[test]
fn without_the_finest_matrix_the_galerkin_levels_are_the_stored_matrixs_to_the_bit() {
    // the stencil's rows are the assembled matrix's, so every coarse operator, every transfer
    // and every coarse smoother is the same
    let (grid, eps, current) = small(14, true);
    let (lattice, eps, matrix, _) = system(grid, eps, Boundaries3d::stretched_pml(3), &current);
    let options = options(CycleShape::V, 0, 1);
    let stored = Hierarchy::new(
        &lattice,
        &eps,
        shifted(&lattice, &eps, &matrix, 0.5),
        options,
    )
    .unwrap();
    let free = from_stencil(&lattice, &eps, options);
    assert_eq!(stored.shapes(), free.shapes());
    assert!(stored.levels.len() >= 2);
    assert!(matches!(free.levels[0].stage, Stage::Stencil { .. }));
    let bits = |a: &Sparse, b: &Sparse| {
        a.size() == b.size()
            && (0..a.size()).all(|r| {
                a.row(r).zip(b.row(r)).all(|((c, v), (d, w))| {
                    c == d && v.re.to_bits() == w.re.to_bits() && v.im.to_bits() == w.im.to_bits()
                }) && a.row(r).count() == b.row(r).count()
            })
    };
    for (s, f) in stored.levels[1..].iter().zip(&free.levels[1..]) {
        assert!(bits(s.stage.matrix().unwrap(), f.stage.matrix().unwrap()));
    }
    assert!(bits(&stored.coarsest, &free.coarsest));
    for (s, f) in stored.levels.iter().zip(&free.levels) {
        assert_eq!(s.prolongation.values, f.prolongation.values);
        assert_eq!(s.prolongation.columns, f.prolongation.columns);
    }
    // and the finest level's product is the stored one's to rounding
    let v = sequence(grid.unknowns(), 11);
    let (a, b) = (
        stored.levels[0].stage.apply(&v, false),
        free.levels[0].stage.apply(&v, false),
    );
    let d: Vec<c64> = a.iter().zip(&b).map(|(p, q)| p - q).collect();
    assert!(norm(&d) < 1e-13 * norm(&a));
}

#[test]
fn without_the_finest_matrix_the_transposed_cycle_is_the_cycles_transpose() {
    let (grid, eps, current) = small(12, true);
    let (lattice, eps, _, _) = system(grid, eps, Boundaries3d::stretched_pml(3), &current);
    let (u, v) = (sequence(grid.unknowns(), 1), sequence(grid.unknowns(), 2));
    for coarse in [Coarse::Galerkin, Coarse::Rediscretized] {
        for shape in [CycleShape::V, CycleShape::F] {
            let h = from_stencil(
                &lattice,
                &eps,
                Multigrid {
                    coarse,
                    jacobi: Jacobi {
                        omega: 0.8,
                        pre: 2,
                        post: 1,
                    },
                    ..options(shape, 1, 1)
                },
            );
            let forward = bilinear(&u, &h.solve(&v));
            let backward = bilinear(&v, &h.solve_transpose(&u));
            assert!(
                (forward - backward).norm() < 1e-10 * forward.norm(),
                "{coarse:?} {shape:?}: {forward} against {backward}"
            );
        }
    }
}

#[test]
fn without_the_finest_matrix_the_cycle_is_the_same_bit_for_bit_on_any_number_of_threads() {
    let (grid, eps, current) = small(14, true);
    for coarse in [Coarse::Galerkin, Coarse::Rediscretized] {
        let run = |threads: usize| -> Vec<c64> {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();
            pool.install(|| {
                let (lattice, eps, _, b) =
                    system(grid, eps, Boundaries3d::stretched_pml(3), &current);
                let h = from_stencil(
                    &lattice,
                    &eps,
                    Multigrid {
                        coarse,
                        ..options(CycleShape::F, 1, 1)
                    },
                );
                let mut out = h.solve(&b);
                out.extend(h.solve_transpose(&b));
                out
            })
        };
        let one = run(1);
        for threads in [2, 5, 20] {
            assert!(
                one.iter()
                    .zip(&run(threads))
                    .all(|(p, q)| p.re.to_bits() == q.re.to_bits()
                        && p.im.to_bits() == q.im.to_bits()),
                "{coarse:?}: {threads} threads differ from one"
            );
        }
    }
}

#[test]
fn plain_qmr_on_shin_and_fans_operator_without_its_matrix_gives_the_stored_ones_field() {
    // the general QMR, with the transposed product from the stencil
    use crate::fdfd::{Formulation, IterativeSolver3d, Product};
    let (grid, eps, current) = small(10, true);
    let stop = Stopping {
        tolerance: 1e-10,
        max_iterations: 5000,
    };
    let solve = |product| {
        let solver = IterativeSolver3d::new_with(
            grid,
            Wavelength::um(1.55).unwrap(),
            eps,
            Boundaries3d::stretched_pml(3),
            Formulation::ShinFan,
            product,
        )
        .unwrap();
        assert_eq!(solver.product(), product);
        solver.solve(&current, stop).unwrap()
    };
    let ((free, a), (stored, b)) = (solve(Product::MatrixFree), solve(Product::Matrix));
    let d: Vec<c64> = (free.values().iter())
        .zip(stored.values())
        .map(|(p, q)| p - q)
        .collect();
    assert!(norm(&d) < 1e-7 * norm(stored.values()));
    // the products differ in their last bits, which the Lanczos process carries along
    assert!(
        a.iterations.abs_diff(b.iterations) * 4 <= b.iterations,
        "{} against {}",
        a.iterations,
        b.iterations
    );
}

#[test]
fn gmres_without_the_finest_matrix_gives_the_stored_multigrids_field() {
    use crate::fdfd::{Formulation, IterativeSolver3d, Product};
    let (grid, eps, current) = small(16, true);
    let stop = Stopping {
        tolerance: 1e-10,
        max_iterations: 500,
    };
    let solve = |product, coarse| {
        IterativeSolver3d::new_with(
            grid,
            Wavelength::um(1.55).unwrap(),
            eps,
            Boundaries3d::stretched_pml(4),
            Formulation::ShinFan,
            product,
        )
        .unwrap()
        .with_multigrid(Multigrid {
            coarse,
            ..options(CycleShape::V, 0, 1)
        })
        .unwrap()
        .solve(&current, stop)
        .unwrap()
    };
    let (stored, with) = solve(Product::Matrix, Coarse::Galerkin);
    for coarse in [Coarse::Galerkin, Coarse::Rediscretized] {
        let (free, how) = solve(Product::MatrixFree, coarse);
        let d: Vec<c64> = (free.values().iter())
            .zip(stored.values())
            .map(|(p, q)| p - q)
            .collect();
        assert!(
            norm(&d) < 1e-7 * norm(stored.values()),
            "{coarse:?}: {:e}",
            norm(&d) / norm(stored.values())
        );
        // no more than twice the stored matrix's ILU(0)-smoothed iterations
        assert!(
            how.iterations <= 2 * with.iterations,
            "{coarse:?}: {} against {}",
            how.iterations,
            with.iterations
        );
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

#[test]
fn the_cycle_a_backend_takes_is_photonoxides_cycle() {
    // the neutral form's cycle, run as its docs say, against the hierarchy's own, for each shape
    let (grid, eps, current) = small(12, true);
    let (lattice, eps, matrix, b) = system(grid, eps, Boundaries3d::stretched_pml(3), &current);
    for (shape, pre, post) in [
        (CycleShape::V, 0, 1),
        (CycleShape::F, 1, 1),
        (CycleShape::W, 1, 0),
    ] {
        let operator = shifted(&lattice, &eps, &matrix, 0.5);
        let h = Hierarchy::new(&lattice, &eps, operator, options(shape, pre, post)).unwrap();
        let cycle = h.cycle_for_backend().unwrap();
        assert_eq!(cycle.levels().len() + 1, h.shapes().len());
        assert_eq!(
            (cycle.shape(), cycle.pre(), cycle.post()),
            (shape, pre, post)
        );
        assert_eq!(cycle.restart(), 40);
        let first = cycle.levels()[0];
        assert_eq!(first.operator().rows(), grid.unknowns());
        assert_eq!(first.smoother().n(), grid.unknowns());
        let coarse = cycle
            .levels()
            .get(1)
            .map_or(cycle.coarsest().rows(), |l| l.operator().rows());
        assert_eq!(
            (first.restriction().rows(), first.restriction().columns()),
            (coarse, grid.unknowns())
        );
        assert_eq!(
            (first.prolongation().rows(), first.prolongation().columns()),
            (grid.unknowns(), coarse)
        );
        for v in [b.clone(), sequence(grid.unknowns(), 5)] {
            let (theirs, ours) = (cycle.cycle(&v), h.solve(&v));
            let d: Vec<c64> = theirs.iter().zip(&ours).map(|(p, q)| p - q).collect();
            assert!(
                norm(&d) < 1e-12 * norm(&ours),
                "{shape:?}({pre}, {post}): {:e}",
                norm(&d) / norm(&ours)
            );
        }
        // the coarsest solve is photonoxide's, bit for bit
        let c = sequence(cycle.coarsest().rows(), 9);
        assert_eq!(cycle.coarsest_solve(&c).unwrap(), h.lu.solve(&c).unwrap());
        // the smoother's factors are its blocks', in one numbering
        let r = sequence(grid.unknowns(), 4);
        let ours = h.levels[0].stage.smooth(&r, false);
        let theirs = first.smoother().solve(&r);
        let d: Vec<c64> = theirs.iter().zip(&ours).map(|(p, q)| p - q).collect();
        assert!(norm(&d) < 1e-12 * norm(&ours));
    }
}

/// An iterative backend that runs GMRES with the cycle it is given, on the CPU and through the
/// neutral form only, as a GPU's does; or, `qmr_only`, one that doesn't run it.
struct CycleOnTheCpu {
    qmr_only: bool,
}

struct Neutral<'a, 'b>(&'a crate::backend::MultigridCycle<'b>);

impl Preconditioner for Neutral<'_, '_> {
    fn solve(&self, v: &[c64]) -> Vec<c64> {
        self.0.cycle(v)
    }

    fn solve_transpose(&self, _: &[c64]) -> Vec<c64> {
        unreachable!("GMRES takes no transposes")
    }
}

impl crate::backend::IterativeSolver for CycleOnTheCpu {
    fn capabilities(&self) -> crate::backend::Capabilities {
        let name = if self.qmr_only {
            "qmr-only-for-tests"
        } else {
            "cycle-on-the-cpu"
        };
        crate::backend::Capabilities::new(name, "1", "MIT OR Apache-2.0")
    }

    fn qmr_run(
        &self,
        _: &crate::backend::Matrix<'_>,
        _: &[c64],
        _: Stopping,
    ) -> Result<crate::backend::QmrRun> {
        Err(Error::invalid("tests", "runs no QMR"))
    }

    fn runs_multigrid(&self) -> bool {
        !self.qmr_only
    }

    fn gmres_multigrid(
        &self,
        matrix: crate::backend::RowMatrix<'_>,
        cycle: &crate::backend::MultigridCycle<'_>,
        b: &[c64],
        stopping: Stopping,
    ) -> Result<(Vec<c64>, crate::fdfd::Convergence)> {
        let a = Sparse::from_rows(
            (0..matrix.rows())
                .map(|r| {
                    (matrix.starts()[r]..matrix.starts()[r + 1])
                        .map(|k| (matrix.indices()[k], matrix.values()[k]))
                        .collect()
                })
                .collect(),
        );
        gmres_preconditioned(&a, &Neutral(cycle), b, stopping, cycle.restart())
    }
}

#[test]
fn a_backend_runs_gmres_with_the_multigrid_and_one_that_doesnt_is_refused() {
    use crate::backend::{Choice, register_iterative};
    use crate::fdfd::{Formulation, IterativeSolver3d};
    for qmr_only in [false, true] {
        register_iterative(std::sync::Arc::new(CycleOnTheCpu { qmr_only })).unwrap();
    }
    let (grid, eps, current) = small(14, true);
    let new = || {
        IterativeSolver3d::new(
            grid,
            Wavelength::um(1.55).unwrap(),
            eps,
            Boundaries3d::stretched_pml(3),
            Formulation::ShinFan,
        )
        .unwrap()
        .with_multigrid(options(CycleShape::V, 0, 1))
        .unwrap()
    };
    let stop = Stopping {
        tolerance: 1e-9,
        max_iterations: 500,
    };
    let (ours, own) = new().solve(&current, stop).unwrap();
    let solver = new()
        .with_iterative_backend(&Choice::Named("cycle-on-the-cpu".into()))
        .unwrap();
    let (theirs, how) = solver.solve(&current, stop).unwrap();
    // the same recurrences, with a cycle that differs from photonoxide's only in rounding
    assert!(
        how.iterations.abs_diff(own.iterations) <= 1,
        "{} against {}",
        how.iterations,
        own.iterations
    );
    assert!(how.residual <= stop.tolerance, "{:e}", how.residual);
    let d: Vec<c64> = (theirs.values().iter())
        .zip(ours.values())
        .map(|(p, q)| p - q)
        .collect();
    assert!(norm(&d) < 1e-6 * norm(ours.values()));
    // a backend that doesn't run it is refused for the multigrid, saying why
    let e = new()
        .with_iterative_backend(&Choice::Named("qmr-only-for-tests".into()))
        .err()
        .unwrap();
    assert!(e.to_string().contains("not GMRES with multigrid"), "{e}");
}

/// `photonoxide bench`'s strip with ports on an `h` grid inside stretched PMLs of `pml`: a
/// 500 × 220 nm silicon strip in oxide along x, 0.8 × 1.4 × 1.0 µm inside the PMLs, and a
/// point current on it.
pub(super) fn strip(
    h: f64,
    pml: usize,
) -> (Grid3d, impl Fn(f64, f64, f64) -> c64 + Copy, Vec<c64>) {
    let inside = |um: f64| (um / h).round() as usize;
    let (nx, ny, nz) = (
        inside(0.8) + 2 * pml,
        inside(1.4) + 2 * pml,
        inside(1.0) + 2 * pml,
    );
    let grid = Grid3d {
        nx,
        ny,
        nz,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: -(ny as f64) * h / 2.0,
        z0: -(nz as f64) * h / 2.0,
    };
    let eps = |_: f64, y: f64, z: f64| {
        let n: f64 = if y.abs() < 0.25 && z.abs() < 0.11 {
            3.476
        } else {
            1.444
        };
        c64::new(n * n, 0.0)
    };
    let mut current = vec![c64::new(0.0, 0.0); grid.unknowns()];
    current[grid.index(Axis::Y, (nx / 3, ny / 2, nz / 2))] = c64::new(1.0, 0.0);
    (grid, eps, current)
}

#[test]
#[ignore = "a measurement: what the hierarchy keeps, level by level; MG_CASE=guide|diel|strip, MG_H (strip, 0.02), MG_PML; cargo test --release fdfd::three::multigrid::tests::sizes -- --ignored --nocapture"]
fn sizes() {
    let var = |name: &str, default: &str| std::env::var(name).unwrap_or_else(|_| default.into());
    let case = var("MG_CASE", "strip");
    let pml: usize = var("MG_PML", if case == "strip" { "16" } else { "10" })
        .parse()
        .unwrap();
    let h: f64 = var("MG_H", "0.02").parse().unwrap();
    let (lattice, eps, matrix, _) = if case == "strip" {
        let (grid, eps, current) = strip(h, pml);
        system(grid, eps, Boundaries3d::stretched_pml(pml), &current)
    } else {
        let (grid, eps, current) = benchmark(&case, pml);
        system(grid, eps, Boundaries3d::stretched_pml(pml), &current)
    };
    let n = matrix.size();
    let sparse = |rows: usize, nnz: usize| (48 * nnz + 16 * rows) as f64;
    let mb = |b: f64| b / 1e6;
    println!(
        "{case}: {n} unknowns, outer matrix {} nonzeros, {:.0} MB",
        matrix.nonzeros(),
        mb(sparse(n, matrix.nonzeros()))
    );
    let operator = shifted(&lattice, &eps, &matrix, 0.5);
    let hierarchy = Hierarchy::new(&lattice, &eps, operator, Multigrid::default()).unwrap();
    let mut total = sparse(n, matrix.nonzeros());
    for (l, (level, shape)) in hierarchy.levels.iter().zip(hierarchy.shapes()).enumerate() {
        let rows = level.stage.matrix().map_or(0, Sparse::size);
        let nnz = level.stage.matrix().map_or(0, Sparse::nonzeros);
        let p = level.prolongation.values.len();
        let a = sparse(rows, nnz);
        let ilu = a;
        let transfers = (48 * p + 8 * rows + 8 * level.restriction.starts.len()) as f64;
        total += a + ilu + transfers;
        println!(
            "level {l} {shape:?}: {rows} rows, {nnz} nonzeros ({:.1} a row), P {p} entries; matrix {:.0} MB, ILU(0) about {:.0} MB, P and its transpose {:.0} MB",
            nnz as f64 / rows as f64,
            mb(a),
            mb(ilu),
            mb(transfers)
        );
    }
    let c = &hierarchy.coarsest;
    println!(
        "coarsest {:?}: {} rows, {} nonzeros",
        hierarchy.shapes().last().unwrap(),
        c.size(),
        c.nonzeros()
    );
    println!(
        "total about {:.0} MB, {:.0} B an unknown",
        mb(total),
        total / n as f64
    );
}

#[test]
#[ignore = "an experiment: GMRES with the multigrid with and without the finest matrix; MG_CASE=guide|diel|strip, MG_H, MG_PML, MG_WITH=both|stored|stencil, MG_COARSE=galerkin|rediscretized, MG_SHAPE, MG_OMEGA, MG_JPRE, MG_JPOST, MG_TOL; cargo test --release fdfd::three::multigrid::tests::stencil -- --ignored --nocapture"]
fn stencil() {
    use crate::fdfd::{Formulation, IterativeSolver3d, Product};
    let var = |name: &str, default: &str| std::env::var(name).unwrap_or_else(|_| default.into());
    let case = var("MG_CASE", "guide");
    let pml: usize = var("MG_PML", if case == "strip" { "16" } else { "10" })
        .parse()
        .unwrap();
    let h: f64 = var("MG_H", "0.02").parse().unwrap();
    let tolerance: f64 = var("MG_TOL", "1e-8").parse().unwrap();
    let with = var("MG_WITH", "both");
    type Eps = Box<dyn Fn(f64, f64, f64) -> c64 + Sync>;
    let (grid, eps, current): (Grid3d, Eps, Vec<c64>) = if case == "strip" {
        let (g, e, c) = strip(h, pml);
        (g, Box::new(e), c)
    } else {
        let (g, e, c) = benchmark(&case, pml);
        (g, Box::new(e), c)
    };
    let options = Multigrid {
        shape: match var("MG_SHAPE", "V").as_str() {
            "F" => CycleShape::F,
            "W" => CycleShape::W,
            _ => CycleShape::V,
        },
        coarse: if var("MG_COARSE", "galerkin") == "rediscretized" {
            Coarse::Rediscretized
        } else {
            Coarse::Galerkin
        },
        ..Multigrid::default()
    };
    println!(
        "{case}: {} unknowns, {options:?}, threads {}",
        grid.unknowns(),
        rayon::current_num_threads()
    );
    let stop = Stopping {
        tolerance,
        max_iterations: 2000,
    };
    let lam = Wavelength::um(1.55).unwrap();
    let mut fields = Vec::new();
    for (label, product) in [
        ("stored", Product::Matrix),
        ("stencil", Product::MatrixFree),
    ] {
        if with != "both" && with != label {
            continue;
        }
        let t = std::time::Instant::now();
        let options = if label == "stored" && var("MG_STORED_COARSE", "galerkin") == "galerkin" {
            Multigrid {
                coarse: Coarse::Galerkin,
                shape: CycleShape::V,
                ..options
            }
        } else {
            options
        };
        let solver = IterativeSolver3d::new_with(
            grid,
            lam,
            &eps,
            Boundaries3d::stretched_pml(pml),
            crate::fdfd::Formulation::ShinFan,
            product,
        )
        .unwrap()
        .with_multigrid(options)
        .unwrap();
        let built = t.elapsed();
        let t = std::time::Instant::now();
        let _ = Formulation::ShinFan;
        match solver.solve(&current, stop) {
            Ok((field, how)) => {
                println!(
                    "{case} {label}: levels {:?}, built in {built:.2?}, {} iterations in {:.2?} ({:.1} ms each), residual {:.1e}",
                    solver.multigrid_grids(),
                    how.iterations,
                    t.elapsed(),
                    1e3 * t.elapsed().as_secs_f64() / how.iterations.max(1) as f64,
                    how.residual
                );
                fields.push(field);
            }
            Err(e) => println!(
                "{case} {label}: built in {built:.2?}, failed after {:.2?}: {e}",
                t.elapsed()
            ),
        }
    }
    if let [a, b] = &fields[..] {
        let d: Vec<c64> = a
            .values()
            .iter()
            .zip(b.values())
            .map(|(p, q)| p - q)
            .collect();
        println!(
            "{case}: the fields differ by {:.1e} (relative)",
            norm(&d) / norm(a.values())
        );
    }
}

#[test]
#[ignore = "a diagnostic: the rediscretized coarse operators against W⁻¹ Pᵀ D_f A P, by region; MG_N, MG_PML, MG_SILICON"]
fn rediscretized_against_galerkin() {
    let var = |name: &str, default: &str| std::env::var(name).unwrap_or_else(|_| default.into());
    let n: usize = var("MG_N", "24").parse().unwrap();
    let pml: usize = var("MG_PML", "4").parse().unwrap();
    let (grid, eps, current) = small(n, var("MG_SILICON", "0") == "1");
    let periodic = crate::fdfd::Edges::Bloch { k: 0.0 };
    let boundaries = if var("MG_BC", "pml") == "periodic" {
        Boundaries3d {
            x: periodic,
            y: periodic,
            z: periodic,
            ..Boundaries3d::pml(0)
        }
    } else {
        Boundaries3d::stretched_pml(pml)
    };
    let (lattice, eps, matrix, _) = system(grid, eps, boundaries, &current);
    let options = |coarse| Multigrid {
        coarsest: 300,
        coarse,
        ..Multigrid::default()
    };
    let shifted_matrix = shifted(&lattice, &eps, &matrix, 0.5);
    let again = shifted(&lattice, &eps, &matrix, 0.5);
    let g = Hierarchy::new(&lattice, &eps, shifted_matrix, options(Coarse::Galerkin)).unwrap();
    let r = Hierarchy::new(&lattice, &eps, again, options(Coarse::Rediscretized)).unwrap();
    println!("levels {:?} and {:?}", g.shapes(), r.shapes());
    for level in 0..g.levels.len().min(r.levels.len()) {
        let rl = &r.levels[level];
        let (fine, coarse) = rl.scaling.as_ref().unwrap();
        let shape = r.shapes()[level + 1];
        let next_rows = coarse.len();
        // a smooth coarse field: a slow sine along each axis, every component
        let cells = shape[0] * shape[1] * shape[2];
        let v: Vec<c64> = (0..next_rows)
            .map(|q| {
                let rest = q % cells;
                let at = [
                    rest % shape[0],
                    (rest / shape[0]) % shape[1],
                    rest / (shape[0] * shape[1]),
                ];
                let s: f64 = (0..3)
                    .map(|a| (std::f64::consts::PI * (at[a] as f64 + 0.5) / shape[a] as f64).sin())
                    .product();
                c64::new(s, 0.3 * s)
            })
            .collect();
        // R 1 against 1
        let ones: Vec<c64> = fine.iter().map(|_| c64::new(1.0, 0.0)).collect();
        let sums = rl.restrict(&ones, false);
        let mut worst_volume = 0.0f64;
        for (q, s) in sums.iter().enumerate() {
            let rest = q % cells;
            let at = [
                rest % shape[0],
                (rest / shape[0]) % shape[1],
                rest / (shape[0] * shape[1]),
            ];
            let away = (0..3).all(|a| at[a] >= 1 && at[a] + 1 < shape[a]);
            if s.norm() > 0.0 && away && (s - 1.0).norm() > worst_volume {
                worst_volume = (s - 1.0).norm();
                if std::env::var("MG_WHERE").is_ok() {
                    println!("  R 1 at component {} {at:?}: {s}", q / cells);
                }
            }
        }
        // the coarse operator on v, against R A_f P v, both Galerkin's (with the same R)
        let a_c = if level + 1 < r.levels.len() {
            r.levels[level + 1].stage.matrix().unwrap().apply(&v)
        } else {
            r.coarsest.apply(&v)
        };
        let through = rl.restrict(
            &rl.stage.matrix().unwrap().apply(&rl.prolongation.apply(&v)),
            false,
        );
        // by depth into the PMLs of the coarse grid: inside vs. not
        let (mut inside, mut outside, mut size_in, mut size_out) = (0.0, 0.0, 0.0, 0.0);
        for q in 0..next_rows {
            let rest = q % cells;
            let at = [
                rest % shape[0],
                (rest / shape[0]) % shape[1],
                rest / (shape[0] * shape[1]),
            ];
            let deep = (0..3).any(|a| at[a] < 2 || at[a] + 2 >= shape[a]);
            let d = (a_c[q] - through[q]).norm_sqr();
            if deep {
                inside += d;
                size_in += through[q].norm_sqr();
            } else {
                outside += d;
                size_out += through[q].norm_sqr();
            }
        }
        println!(
            "level {level} -> {shape:?}: R 1 off 1 by {worst_volume:.2e}; A_c v against R A P v: {:.2e} near the sides, {:.2e} inside",
            (inside / size_in).sqrt(),
            (outside / size_out).sqrt()
        );
    }
}
