//! Block QMR's 3D setups, shared by the tests and the validation report: a coupler's S-matrix,
//! its ports' runs as one block against one QMR solve each.

use num_complex::Complex64 as c64;

use super::port_checks::{OXIDE, SILICON};
use super::{Axis, Boundaries3d, Formulation, Grid3d, IterativeSolver3d, Port3d};
use crate::Result;
use crate::fdfd::{BlockConvergence, Side, Stopping};
use crate::units::Wavelength;

/// How the coupler is solved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Solve {
    /// QMR on the curl-curl operator's symmetric similarity, the matrix assembled.
    Stored,
    /// The same without the matrix.
    Free,
    /// QMR on Shin and Fan's operator with stretched PMLs, preconditioned by ILU(0).
    Ilu,
    /// The same preconditioned by the multigrid cycle (GMRES one at a time).
    Multigrid,
}

/// The coupler's permittivity: two silicon strips (350 × 220 nm, 150 nm apart) along x in
/// oxide.
fn coupler_eps(_: f64, y: f64, z: f64) -> c64 {
    let core = (0.075..0.425).contains(&y.abs()) && z.abs() < 0.11;
    let n = if core { SILICON } else { OXIDE };
    c64::new(n * n, 0.0)
}

/// A directional coupler's straight section on an `h` grid (µm): [`coupler_eps`]'s strips,
/// `length` × 1.2 × 0.8 µm inside PMLs of `pml` cells all round, at 1.55 µm, solved as `solve`
/// says; `modes` modes on each of two port planes `pml` + 2 cells in from each end, the left
/// plane's ports first.
pub(crate) fn coupler(
    h: f64,
    length: f64,
    pml: usize,
    modes: usize,
    solve: Solve,
) -> Result<(IterativeSolver3d, Vec<Port3d>)> {
    let inside = |um: f64| (um / h).round() as usize;
    let (ix, iy, iz) = (inside(length), inside(1.2), inside(0.8));
    let (nx, ny, nz) = (ix + 2 * pml, iy + 2 * pml, iz + 2 * pml);
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
    let lam = Wavelength::um(1.55)?;
    let solver = match solve {
        Solve::Stored => IterativeSolver3d::new(
            grid,
            lam,
            coupler_eps,
            Boundaries3d::pml(pml),
            Formulation::CurlCurl,
        )?,
        Solve::Free => {
            IterativeSolver3d::matrix_free(grid, lam, coupler_eps, Boundaries3d::pml(pml))?
        }
        Solve::Ilu | Solve::Multigrid => {
            let shin = IterativeSolver3d::new(
                grid,
                lam,
                coupler_eps,
                Boundaries3d::stretched_pml(pml),
                Formulation::ShinFan,
            )?;
            if solve == Solve::Ilu {
                shin.with_ilu()?
            } else {
                shin.with_multigrid(super::Multigrid::default())?
            }
        }
    };
    let mut ports = Vec::new();
    for (plane, side) in [(pml + 2, Side::Left), (nx - pml - 3, Side::Right)] {
        for mode in solver.port_modes(Axis::X, plane, modes)? {
            ports.push(Port3d { mode, side });
        }
    }
    Ok((solver, ports))
}

/// The same S-matrix two ways: its ports' runs as one block and one QMR solve each, to
/// `tolerance`. The largest difference between the two, how the block went, and the single
/// solves' iterations summed.
pub(crate) fn block_against_single(
    solver: &IterativeSolver3d,
    ports: &[Port3d],
    tolerance: f64,
) -> Result<(f64, BlockConvergence, usize)> {
    let stopping = Stopping {
        tolerance,
        max_iterations: 100_000,
    };
    let (single, runs) = solver.s_matrix_with_convergence(ports, stopping)?;
    let (block, how) = solver.s_matrix_block(ports, stopping)?;
    let worst = single
        .iter()
        .flatten()
        .zip(block.iter().flatten())
        .map(|(a, b)| (a - b).norm())
        .fold(0.0, f64::max);
    Ok((worst, how, runs.iter().map(|c| c.iterations).sum()))
}

/// The coupler of [`coupler`] at 50 nm with 0.5 µm inside PMLs of 4 cells, 8 ports (each
/// end's four supermodes), solved as `solve` says to a relative residual of 1e-10, its ports one
/// at a time and as a block: the largest difference between the two S-matrices, and the block's
/// iterations over the single solves' summed. NaN if a solve fails.
pub(crate) fn coupler_block(solve: Solve) -> (f64, f64) {
    let Ok((solver, ports)) = coupler(0.05, 0.5, 4, 4, solve) else {
        return (f64::NAN, f64::NAN);
    };
    match block_against_single(&solver, &ports, 1e-10) {
        Ok((worst, how, single)) => (worst, how.iterations as f64 / single as f64),
        Err(_) => (f64::NAN, f64::NAN),
    }
}

/// Block QMR on the 50 nm coupler's own operator A (curl-curl, PMLs; 41 472 unknowns) with
/// four right-hand sides: two ports' mode sources b₁ and b₂, b₁ + 2 b₂, and A b₁. The third
/// leaves the block at once and is recovered as x₁ + 2 x₂ (Freund and Malhotra's Eq. 4.18);
/// A b₁ makes A v₁ dependent on the vectors before it, a product deflated, and the system that
/// weighs most in the combination with no quasi-residual left leaves the block (their Eqs.
/// 4.16–4.20); A x = A b₁ is solved by b₁. To 1e-10, on A by the general form and on its
/// symmetric similarity by the symmetric one: the largest of |x₄ − b₁| / |b₁| and
/// |x₃ − (x₁ + 2x₂)| / |x₃| (as maxima over the values), NaN unless systems were dropped.
pub(crate) fn dependent_columns() -> f64 {
    use crate::fdfd::Direction;
    use crate::fdfd::krylov::{Sparse, block_qmr, block_qmr_similar};
    let Ok((solver, ports)) = coupler(0.05, 0.5, 4, 1, Solve::Stored) else {
        return f64::NAN;
    };
    let grid = solver.grid();
    let Ok(lam) = Wavelength::um(1.55) else {
        return f64::NAN;
    };
    let Ok((lattice, eps)) = super::Solver3d::setup(grid, lam, coupler_eps, Boundaries3d::pml(4))
    else {
        return f64::NAN;
    };
    let a = Sparse::new(
        grid.unknowns(),
        lattice
            .assemble(&eps)
            .into_iter()
            .map(|t| (t.row, t.col, t.val)),
    );
    let b1 = solver.mode_source(&ports[0].mode, Direction::Forward);
    let b2 = solver.mode_source(&ports[1].mode, Direction::Backward);
    let b3: Vec<c64> = b1.iter().zip(&b2).map(|(p, q)| p + 2.0 * q).collect();
    let ab1 = a.apply(&b1);
    let b = vec![b1.clone(), b2, b3, ab1];
    let stopping = Stopping {
        tolerance: 1e-10,
        max_iterations: 100_000,
    };
    let largest = |v: &[c64]| v.iter().map(|z| z.norm()).fold(0.0, f64::max);
    let off = |x: &[c64], y: &[c64]| {
        x.iter()
            .zip(y)
            .map(|(p, q)| (p - q).norm())
            .fold(0.0, f64::max)
            / largest(y)
    };
    let mut worst = 0.0f64;
    let symmetric = a.symmetrized();
    for general in [true, false] {
        let solved = if general {
            block_qmr(&a, &b, stopping, false, None)
        } else {
            let Some((s, similar)) = &symmetric else {
                return f64::NAN;
            };
            block_qmr_similar(similar, s, &b, stopping)
        };
        let Ok((x, how)) = solved else {
            return f64::NAN;
        };
        if how.dropped < 2 {
            return f64::NAN;
        }
        let combined: Vec<c64> = x[0].iter().zip(&x[1]).map(|(p, q)| p + 2.0 * q).collect();
        worst = worst.max(off(&x[3], &b1)).max(off(&x[2], &combined));
    }
    worst
}

/// The coupler's block S-matrix (50 nm, 8 ports, 1e-10, the matrix stored) on 1 and on 4
/// threads: its entries whose bits differ (count), NaN if a solve fails.
pub(crate) fn thread_differences() -> f64 {
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .ok()
            .and_then(|pool| {
                pool.install(|| {
                    let (solver, ports) = coupler(0.05, 0.5, 4, 4, Solve::Stored).ok()?;
                    let stopping = Stopping {
                        tolerance: 1e-10,
                        max_iterations: 100_000,
                    };
                    solver.s_matrix_block(&ports, stopping).ok()
                })
            })
    };
    let (Some((one, a)), Some((four, b))) = (run(1), run(4)) else {
        return f64::NAN;
    };
    let mut differ = usize::from(a != b);
    for (p, q) in one.iter().flatten().zip(four.iter().flatten()) {
        differ += usize::from(p.re.to_bits() != q.re.to_bits() || p.im.to_bits() != q.im.to_bits());
    }
    differ as f64
}

/// A closed guide's S-matrix by block QMR against the direct solver's, as the tolerance
/// tightens: a silicon strip stepping from 300 to 400 nm wide in a closed box of oxide
/// (0.6 × 0.5 µm, 20 × 12 × 10 cells of 50 nm, PMLs of 5 along x), two modes on each of two
/// ports. The largest difference from the direct solver's S at each of `tolerances`, NaN if a
/// solve fails.
pub(crate) fn against_direct(tolerances: &[f64]) -> Vec<f64> {
    use crate::fdfd::Edges;
    let h = 0.05;
    let (nx, ny, nz) = (20, 12, 10);
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
    let wall = Edges::Pml { low: 0, high: 0 };
    let boundaries = Boundaries3d {
        x: Edges::Pml { low: 5, high: 5 },
        y: wall,
        z: wall,
        reflection: 1e-8,
        order: 3.0,
        real_stretch: 0.0,
    };
    let eps = |x: f64, y: f64, z: f64| {
        let half = if x < 0.5 { 0.15 } else { 0.2 };
        let n: f64 = if y.abs() < half && z.abs() < 0.11 {
            SILICON
        } else {
            OXIDE
        };
        c64::new(n * n, 0.0)
    };
    let nan = vec![f64::NAN; tolerances.len()];
    let Ok(lam) = Wavelength::um(1.55) else {
        return nan;
    };
    let ports = |modes: &dyn Fn(usize) -> Result<Vec<super::PortMode3d>>| -> Result<Vec<Port3d>> {
        let mut out = Vec::new();
        for (plane, side) in [(7, Side::Left), (12, Side::Right)] {
            for mode in modes(plane)? {
                out.push(Port3d { mode, side });
            }
        }
        Ok(out)
    };
    let Ok(direct) = super::Solver3d::new(grid, lam, eps, boundaries) else {
        return nan;
    };
    let Ok(exact) = ports(&|p| direct.port_modes(Axis::X, p, 2)).and_then(|p| direct.s_matrix(&p))
    else {
        return nan;
    };
    let Ok(iterative) = IterativeSolver3d::new(grid, lam, eps, boundaries, Formulation::CurlCurl)
    else {
        return nan;
    };
    let Ok(at) = ports(&|p| iterative.port_modes(Axis::X, p, 2)) else {
        return nan;
    };
    tolerances
        .iter()
        .map(|&tolerance| {
            let stopping = Stopping {
                tolerance,
                max_iterations: 100_000,
            };
            match iterative.s_matrix_block(&at, stopping) {
                Ok((s, _)) => s
                    .iter()
                    .flatten()
                    .zip(exact.iter().flatten())
                    .map(|(p, q)| (p - q).norm())
                    .fold(0.0, f64::max),
                Err(_) => f64::NAN,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blocks_s_matrix_is_the_single_solves() {
        // 4 ports: each end's two modes of highest β, 50 nm grid (the validation report has 8)
        let (solver, ports) = coupler(0.05, 0.5, 4, 2, Solve::Stored).unwrap();
        let (worst, _, _) = block_against_single(&solver, &ports, 1e-10).unwrap();
        assert!(worst < 1e-9, "{worst:e}");
    }

    /// Times an S-matrix's runs one at a time and as a block.
    #[test]
    #[ignore = "a measurement: H (0.04), LENGTH (1.0), PML (6), MODES (4), SOLVE (stored, free, ilu, multigrid), TOL (1e-6), RAYON_NUM_THREADS; cargo test --release fdfd::three::block_checks::tests::measure -- --ignored --nocapture"]
    fn measure() {
        use std::time::Instant;
        let var = |name: &str, default: &str| std::env::var(name).unwrap_or(default.into());
        let h: f64 = var("H", "0.04").parse().unwrap();
        let length: f64 = var("LENGTH", "1.0").parse().unwrap();
        let pml: usize = var("PML", "6").parse().unwrap();
        let modes: usize = var("MODES", "4").parse().unwrap();
        let tolerance: f64 = var("TOL", "1e-6").parse().unwrap();
        let solve = match var("SOLVE", "stored").as_str() {
            "stored" => Solve::Stored,
            "free" => Solve::Free,
            "ilu" => Solve::Ilu,
            "multigrid" => Solve::Multigrid,
            other => panic!("SOLVE {other}?"),
        };
        let (solver, ports) = coupler(h, length, pml, modes, solve).unwrap();
        let grid = solver.grid();
        let stopping = Stopping {
            tolerance,
            max_iterations: 100_000,
        };
        // alternately, the fastest of each kept: the machine is shared
        let repeat: usize = var("REPEAT", "1").parse().unwrap();
        let (mut one, mut together) = (f64::INFINITY, f64::INFINITY);
        let mut results = None;
        for _ in 0..repeat {
            let t = Instant::now();
            let (single, runs) = solver.s_matrix_with_convergence(&ports, stopping).unwrap();
            one = one.min(t.elapsed().as_secs_f64());
            let t = Instant::now();
            let (block, how) = solver.s_matrix_block(&ports, stopping).unwrap();
            together = together.min(t.elapsed().as_secs_f64());
            results = Some((single, runs, block, how));
        }
        let (single, runs, block, how) = results.unwrap();
        let worst = single
            .iter()
            .flatten()
            .zip(block.iter().flatten())
            .map(|(a, b)| (a - b).norm())
            .fold(0.0, f64::max);
        println!(
            "{solve:?}: {} x {} x {} cells of {} nm ({} unknowns), {} ports, tolerance {tolerance:e}, {} threads",
            grid.nx,
            grid.ny,
            grid.nz,
            h * 1000.0,
            grid.unknowns(),
            ports.len(),
            rayon::current_num_threads()
        );
        println!(
            "  one at a time: {one:.2} s, {} iterations ({:?})",
            runs.iter().map(|c| c.iterations).sum::<usize>(),
            runs.iter().map(|c| c.iterations).collect::<Vec<_>>()
        );
        println!(
            "  block: {together:.2} s, {} iterations, {} products, {} deflations, {} restarts; each converged at {:?}",
            how.iterations,
            how.products,
            how.deflations,
            how.restarts,
            how.columns.iter().map(|c| c.iterations).collect::<Vec<_>>()
        );
        println!("  largest difference in S: {worst:e}");
    }
}
