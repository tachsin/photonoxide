use super::*;
use crate::fdfd::krylov::{Preconditioner, Sparse, Stopping, qmr, qmr_preconditioned};
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
    if var("MG_GMRES", "0") == "1" {
        for tolerance in [1e-6, 1e-8, 1e-10] {
            let t = std::time::Instant::now();
            let (x, iterations) = gmres(&matrix, &h, &b, tolerance, 200);
            println!(
                "MG {case}: GMRES + MG, {tolerance:e}: {iterations} iterations, {:?}, error {:.1e}",
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

/// Right-preconditioned GMRES(m) by modified Gram-Schmidt, for comparison: the iterations (one
/// preconditioner solve each) to a relative residual of `tolerance`, and the solution.
fn gmres(
    a: &Sparse,
    m: &Hierarchy,
    b: &[c64],
    tolerance: f64,
    restart: usize,
) -> (Vec<c64>, usize) {
    let n = b.len();
    let dot = |u: &[c64], v: &[c64]| -> c64 { u.iter().zip(v).map(|(p, q)| p.conj() * q).sum() };
    let mut x = vec![c64::new(0.0, 0.0); n];
    let beta0 = norm(b);
    let mut iterations = 0;
    loop {
        let ax = a.apply(&x);
        let r: Vec<c64> = b.iter().zip(&ax).map(|(p, q)| p - q).collect();
        let beta = norm(&r);
        if beta <= tolerance * beta0 || iterations > 2000 {
            return (x, iterations);
        }
        let mut v: Vec<Vec<c64>> = vec![r.iter().map(|z| z / beta).collect()];
        let mut z: Vec<Vec<c64>> = Vec::new();
        let mut h = vec![vec![c64::new(0.0, 0.0); restart]; restart + 1];
        let mut g = vec![c64::new(0.0, 0.0); restart + 1];
        g[0] = c64::new(beta, 0.0);
        let mut rot: Vec<(c64, c64)> = Vec::new();
        let mut k = 0;
        while k < restart {
            let zk = m.solve(&v[k]);
            let mut w = a.apply(&zk);
            z.push(zk);
            for i in 0..=k {
                h[i][k] = dot(&v[i], &w);
                for (wj, vj) in w.iter_mut().zip(&v[i]) {
                    *wj -= h[i][k] * vj;
                }
            }
            let hn = norm(&w);
            h[k + 1][k] = c64::new(hn, 0.0);
            for (i, &(c, s)) in rot.iter().enumerate() {
                let (p, q) = (h[i][k], h[i + 1][k]);
                h[i][k] = c.conj() * p + s.conj() * q;
                h[i + 1][k] = -s * p + c * q;
            }
            let (p, q) = (h[k][k], h[k + 1][k]);
            let d = (p.norm_sqr() + q.norm_sqr()).sqrt();
            let (c, s) = (p / d, q / d);
            h[k][k] = c64::new(d, 0.0);
            h[k + 1][k] = c64::new(0.0, 0.0);
            g[k + 1] = -s * g[k];
            g[k] = c.conj() * g[k];
            rot.push((c, s));
            v.push(w.iter().map(|q| q / hn).collect());
            k += 1;
            iterations += 1;
            if g[k].norm() <= tolerance * beta0 {
                break;
            }
        }
        // y from the triangle, x += Z y
        let mut y = vec![c64::new(0.0, 0.0); k];
        for i in (0..k).rev() {
            let mut sum = g[i];
            for j in i + 1..k {
                sum -= h[i][j] * y[j];
            }
            y[i] = sum / h[i][i];
        }
        for (j, yj) in y.iter().enumerate() {
            for (xi, zi) in x.iter_mut().zip(&z[j]) {
                *xi += yj * zi;
            }
        }
    }
}
