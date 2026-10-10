//! Recycling's 3D setups, shared by the tests, the validation report and the measurements: a
//! directional coupler's S-matrix over a wavelength sweep, and a forward solve with its adjoint.

use num_complex::Complex64 as c64;

use super::port_checks::{OXIDE, SILICON};
use super::{
    Axis, Boundaries3d, Formulation, Grid3d, IterativeSolver3d, Multigrid, Port3d, Recycler,
};
use crate::Result;
use crate::fdfd::{Convergence, Side, Stopping};
use crate::units::Wavelength;

/// How the coupler is solved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Solve {
    /// GMRES on Shin and Fan's operator with stretched PMLs, preconditioned by the multigrid.
    Multigrid(Multigrid),
    /// QMR on the curl-curl operator's symmetric similarity, the matrix assembled, plain PMLs.
    Symmetric,
}

/// A directional coupler's straight section: two silicon strips (350 × 220 nm, 150 nm apart)
/// along x in oxide, `length` × 1.2 × 0.8 µm on an `h` grid (µm) inside PMLs of `pml` cells all
/// round, at `wavelength` (µm), solved as `solve` says; `modes` modes on each of two port planes
/// `pml` + 2 cells in from each end. The gap between the strips over the middle third of the
/// length, a design region, has the permittivity of oxide plus `design`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Coupler {
    pub(crate) wavelength: f64,
    pub(crate) design: f64,
    pub(crate) h: f64,
    pub(crate) length: f64,
    pub(crate) pml: usize,
    pub(crate) modes: usize,
    pub(crate) solve: Solve,
}

/// [`Coupler`] at `wavelength` with the rest as given, no design.
#[cfg(test)]
pub(crate) fn coupler(
    wavelength: f64,
    h: f64,
    length: f64,
    pml: usize,
    modes: usize,
    solve: Solve,
) -> Result<(IterativeSolver3d, Vec<Port3d>)> {
    Coupler {
        wavelength,
        design: 0.0,
        h,
        length,
        pml,
        modes,
        solve,
    }
    .build()
}

impl Coupler {
    /// The solver and its ports, the left plane's first.
    pub(crate) fn build(&self) -> Result<(IterativeSolver3d, Vec<Port3d>)> {
        let Coupler {
            wavelength,
            design,
            h,
            length,
            pml,
            modes,
            solve,
        } = *self;
        let start = pml as f64 * h;
        let coupler_eps = move |x: f64, y: f64, z: f64| {
            let core = (0.075..0.425).contains(&y.abs()) && z.abs() < 0.11;
            let gap = y.abs() < 0.075
                && z.abs() < 0.11
                && (start + length / 3.0..start + 2.0 * length / 3.0).contains(&x);
            let n = if core { SILICON } else { OXIDE };
            c64::new(n * n + if gap { design } else { 0.0 }, 0.0)
        };
        build(wavelength, h, length, pml, modes, solve, coupler_eps)
    }
}

fn build(
    wavelength: f64,
    h: f64,
    length: f64,
    pml: usize,
    modes: usize,
    solve: Solve,
    coupler_eps: impl Fn(f64, f64, f64) -> c64 + Copy,
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
    let lam = Wavelength::um(wavelength)?;
    let solver = match solve {
        Solve::Multigrid(options) => IterativeSolver3d::new(
            grid,
            lam,
            coupler_eps,
            Boundaries3d::stretched_pml(pml),
            Formulation::ShinFan,
        )?
        .with_multigrid(options)?,
        Solve::Symmetric => IterativeSolver3d::new(
            grid,
            lam,
            coupler_eps,
            Boundaries3d::pml(pml),
            Formulation::CurlCurl,
        )?,
    };
    let mut ports = Vec::new();
    for (plane, side) in [(pml + 2, Side::Left), (nx - pml - 3, Side::Right)] {
        for mode in solver.port_modes(Axis::X, plane, modes)? {
            ports.push(Port3d { mode, side });
        }
    }
    Ok((solver, ports))
}

/// A sweep's S-matrices and each port's run, wavelength by wavelength.
pub(crate) struct Sweep {
    pub(crate) s: Vec<Vec<Vec<c64>>>,
    pub(crate) runs: Vec<Vec<Convergence>>,
    /// The products spent carrying the recycled space to each new wavelength, summed.
    pub(crate) carried: usize,
    /// The wall time of the solves, the solvers' builds left out (s).
    pub(crate) seconds: f64,
    /// The wall time of the builds: the operators, the multigrids, the port modes (s).
    pub(crate) building: f64,
}

/// The coupler's S-matrix at each of `wavelengths`, by `build`'s solver and ports, to
/// `tolerance`: one solve each when `recycler` is none, else recycled through it.
pub(crate) fn sweep(
    wavelengths: &[f64],
    build: &dyn Fn(f64) -> Result<(IterativeSolver3d, Vec<Port3d>)>,
    tolerance: f64,
    mut recycler: Option<&mut Recycler>,
) -> Result<Sweep> {
    let stopping = Stopping {
        tolerance,
        max_iterations: 100_000,
    };
    let mut out = Sweep {
        s: Vec::new(),
        runs: Vec::new(),
        carried: 0,
        seconds: 0.0,
        building: 0.0,
    };
    for &lam in wavelengths {
        let started = std::time::Instant::now();
        let (solver, ports) = build(lam)?;
        out.building += started.elapsed().as_secs_f64();
        let start = std::time::Instant::now();
        let (s, runs) = match recycler.as_deref_mut() {
            Some(r) => solver.s_matrix_recycled_with_convergence(&ports, stopping, r)?,
            None => solver.s_matrix_with_convergence(&ports, stopping)?,
        };
        out.seconds += start.elapsed().as_secs_f64();
        out.s.push(s);
        out.runs.push(runs);
    }
    if let Some(r) = recycler {
        out.carried = r.products();
    }
    Ok(out)
}

/// The largest difference between two sweeps' S-matrices.
pub(crate) fn largest_difference(a: &Sweep, b: &Sweep) -> f64 {
    a.s.iter()
        .flatten()
        .flatten()
        .zip(b.s.iter().flatten().flatten())
        .map(|(p, q)| (p - q).norm())
        .fold(0.0, f64::max)
}

/// Each wavelength's iterations, its ports' summed.
pub(crate) fn iterations(sweep: &Sweep) -> Vec<usize> {
    sweep
        .runs
        .iter()
        .map(|runs| runs.iter().map(|c| c.iterations).sum())
        .collect()
}

/// The validation's coupler: 0.5 µm long on a 50 nm grid inside PMLs of 4 (18 × 32 × 24 cells,
/// 41 472 unknowns), one mode on each port plane, at `wavelength`.
pub(crate) fn small(wavelength: f64, solve: Solve) -> Result<(IterativeSolver3d, Vec<Port3d>)> {
    Coupler {
        wavelength,
        design: 0.0,
        h: 0.05,
        length: 0.5,
        pml: 4,
        modes: 1,
        solve,
    }
    .build()
}

/// The two ways the validation solves: the multigrid with GCRO-DR keeping 10 vectors and the
/// last 4 solutions, and symmetric QMR with the last 4 solutions.
fn recycled(solve: Solve) -> Recycler {
    match solve {
        Solve::Multigrid(_) => Recycler::new().with_krylov(10).with_solutions(4),
        Solve::Symmetric => Recycler::new().with_solutions(4),
    }
}

/// The small coupler's S-matrix at 1.53, 1.54, 1.55 and 1.56 µm, solved as `solve` says (the
/// multigrid to a relative residual of 1e-11, symmetric QMR to 1e-10, the lowest it reaches
/// here: its rounding stalls it at 1.3e-11), one plain solve per port against recycled through
/// [`recycled`]'s recycler: the largest difference between the two sweeps' entries, NaN if a
/// solve fails.
pub(crate) fn s_matrix_against_plain(solve: Solve) -> f64 {
    let wavelengths = [1.53, 1.54, 1.55, 1.56];
    let build = |lam: f64| small(lam, solve);
    let mut recycler = recycled(solve);
    let tolerance = match solve {
        Solve::Multigrid(_) => 1e-11,
        Solve::Symmetric => 1e-10,
    };
    match (
        sweep(&wavelengths, &build, tolerance, None),
        sweep(&wavelengths, &build, tolerance, Some(&mut recycler)),
    ) {
        (Ok(plain), Ok(again)) => largest_difference(&plain, &again),
        _ => f64::NAN,
    }
}

/// The convergence of a sweep as the recycled space grows: the small coupler by the multigrid
/// at 1.500, 1.505, …, 1.535 µm (8 wavelengths, 2 ports) to 1e-10, recycling the last 0, 2, 4,
/// 6 and 8 solutions; each one's iterations at each wavelength, its ports' summed, the plain
/// sweep's first. Empty if a solve fails.
pub(crate) fn sweep_convergence() -> Vec<Vec<usize>> {
    let wavelengths: Vec<f64> = (0..8).map(|i| 1.5 + 0.005 * i as f64).collect();
    let build = |lam: f64| small(lam, Solve::Multigrid(Multigrid::default()));
    let mut out = Vec::new();
    for count in [0, 2, 4, 6, 8] {
        let mut recycler = Recycler::new().with_solutions(count);
        let Ok(run) = sweep(&wavelengths, &build, 1e-10, Some(&mut recycler)) else {
            return Vec::new();
        };
        out.push(iterations(&run));
    }
    out
}

/// The small coupler's recycled sweep ([`s_matrix_against_plain`]'s, by the multigrid with
/// GCRO-DR, 1.54 to 1.56 µm, 1e-10) on 1 and on 4 threads: its S-matrices' entries whose bits
/// differ, plus one if the iterations differ; NaN if a solve fails.
pub(crate) fn thread_differences() -> f64 {
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .ok()
            .and_then(|pool| {
                pool.install(|| {
                    let solve = Solve::Multigrid(Multigrid::default());
                    let build = |lam: f64| small(lam, solve);
                    let mut recycler = recycled(solve);
                    sweep(&[1.54, 1.55, 1.56], &build, 1e-10, Some(&mut recycler)).ok()
                })
            })
    };
    let (Some(one), Some(four)) = (run(1), run(4)) else {
        return f64::NAN;
    };
    let mut differ = usize::from(iterations(&one) != iterations(&four));
    for (p, q) in one
        .s
        .iter()
        .flatten()
        .flatten()
        .zip(four.s.iter().flatten().flatten())
    {
        differ += usize::from(p.re.to_bits() != q.re.to_bits() || p.im.to_bits() != q.im.to_bits());
    }
    differ as f64
}

/// The adjoint gradient by the iterative solvers, recycled from their forward solve, against
/// the direct solver's ([`super::Solver3d::mode_power_gradient`]) on the same problem: a
/// rectangular guide along z (ε = 12, 0.4 × 0.3 µm, in 2.1) with a block of ε = 6 beside it, on
/// 14 × 12 × 24 cells of 50 nm with PMLs of 4, at 1.55 µm, the guide's mode launched on plane 7
/// and its power ahead on plane 16 (as `adjoint::gradient_against_differences`). Two ways, each
/// to a relative residual of 1e-11: symmetric QMR on the curl-curl operator with plain PMLs,
/// and GMRES with the multigrid and GCRO-DR (10 vectors) on Shin and Fan's with stretched ones.
/// The largest difference from the direct solver's gradient relative to its largest value, by
/// QMR and by the multigrid; NaN if a solve fails.
pub(crate) fn adjoint_against_direct() -> (f64, f64) {
    use super::Solver3d;
    use crate::fdfd::Direction;
    let (nx, ny, nz, h) = (14usize, 12usize, 24usize, 0.05);
    let grid = Grid3d {
        nx,
        ny,
        nz,
        dx: h,
        dy: h,
        dz: h,
        x0: -(nx as f64) * h / 2.0,
        y0: -(ny as f64) * h / 2.0,
        z0: -(nz as f64) * h / 2.0,
    };
    let eps = |x: f64, y: f64, z: f64| {
        let v = if x.abs() < 0.2 && y.abs() < 0.15 {
            12.0
        } else if (0.2..0.3).contains(&x) && (0.0..0.1).contains(&y) && (0.0..0.15).contains(&z) {
            6.0
        } else {
            2.1
        };
        c64::new(v, 0.0)
    };
    let stopping = Stopping {
        tolerance: 1e-11,
        max_iterations: 100_000,
    };
    let Ok(lam) = Wavelength::um(1.55) else {
        return (f64::NAN, f64::NAN);
    };
    let run = |boundaries: Boundaries3d, iterative: IterativeSolver3d, mut recycler: Recycler| {
        let direct = Solver3d::new(grid, lam, eps, boundaries).ok()?;
        let source = direct.port_modes(Axis::Z, 7, 1).ok()?.remove(0);
        let ahead = direct.port_modes(Axis::Z, 16, 1).ok()?.remove(0);
        let rhs = direct.mode_source(&source, Direction::Forward);
        let field = direct.solve_system(&rhs).ok()?;
        let (_, exact) = direct
            .mode_power_gradient(&field, &ahead, Direction::Forward)
            .ok()?;
        let (field, _) = iterative
            .solve_system_recycled(&rhs, stopping, &mut recycler)
            .ok()?;
        let (_, gradient, how) = iterative
            .mode_power_gradient_recycled(
                &field,
                &ahead,
                Direction::Forward,
                stopping,
                &mut recycler,
            )
            .ok()?;
        let largest = exact.iter().fold(0.0f64, |m, g| m.max(g.abs()));
        let worst = exact
            .iter()
            .zip(&gradient)
            .fold(0.0f64, |m, (p, q)| m.max((p - q).abs()));
        Some((worst / largest, how.iterations))
    };
    let symmetric =
        IterativeSolver3d::new(grid, lam, eps, Boundaries3d::pml(4), Formulation::CurlCurl)
            .ok()
            .and_then(|s| run(Boundaries3d::pml(4), s, Recycler::new().with_solutions(1)));
    let multigrid = IterativeSolver3d::new(
        grid,
        lam,
        eps,
        Boundaries3d::stretched_pml(4),
        Formulation::ShinFan,
    )
    .and_then(|s| s.with_multigrid(Multigrid::default()))
    .ok()
    .and_then(|s| {
        run(
            Boundaries3d::stretched_pml(4),
            s,
            Recycler::new().with_krylov(10),
        )
    });
    match (symmetric, multigrid) {
        (Some((a, _)), Some((b, _))) => (a, b),
        _ => (f64::NAN, f64::NAN),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "four sweeps: the validation report runs it in release (fdfd3d/recycle-s-matrix)"
    )]
    fn recycled_sweeps_give_the_plain_s_matrices() {
        let worst = s_matrix_against_plain(Solve::Multigrid(Multigrid::default()));
        assert!(worst < 1e-10, "{worst:e}");
        let worst = s_matrix_against_plain(Solve::Symmetric);
        assert!(worst < 1e-10, "{worst:e}");
    }

    #[test]
    fn the_recycled_adjoint_gives_the_direct_solvers_gradient() {
        let (symmetric, multigrid) = adjoint_against_direct();
        let worst = symmetric.max(multigrid);
        assert!(worst < 1e-8, "{worst:e}");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "five sweeps: the validation report runs it in release (fdfd3d/recycle-convergence)"
    )]
    fn a_sweeps_iterations_fall_as_the_recycled_space_grows() {
        let runs = sweep_convergence();
        assert_eq!(runs.len(), 5);
        let last: Vec<usize> = runs.iter().map(|r| *r.last().unwrap()).collect();
        assert!(last.windows(2).all(|w| w[1] <= w[0]), "{last:?}");
        assert!(last[4] * 3 < last[0], "{last:?}");
    }

    #[test]
    fn a_recycler_refuses_as_many_vectors_as_the_restart_and_forgets_on_clear() {
        let (solver, ports) = small(1.55, Solve::Multigrid(Multigrid::default())).unwrap();
        let rhs = solver.mode_source(&ports[0].mode, crate::fdfd::Direction::Forward);
        let stopping = Stopping {
            tolerance: 1e-8,
            max_iterations: 10_000,
        };
        let mut too_many = Recycler::new().with_krylov(40);
        assert!(
            solver
                .solve_system_recycled(&rhs, stopping, &mut too_many)
                .is_err()
        );
        let mut r = Recycler::new().with_krylov(10).with_solutions(2);
        let (_, first) = solver
            .solve_system_recycled(&rhs, stopping, &mut r)
            .unwrap();
        assert_eq!(r.dimension(), 10);
        // the same system again: solved by the earlier solution at once
        let (_, again) = solver
            .solve_system_recycled(&rhs, stopping, &mut r)
            .unwrap();
        assert!(first.iterations > 0 && again.iterations == 0);
        assert_eq!(r.products(), 0);
        r.clear();
        assert_eq!(r.dimension(), 0);
        let (_, after) = solver
            .solve_system_recycled(&rhs, stopping, &mut r)
            .unwrap();
        assert_eq!(after.iterations, first.iterations);
    }

    #[test]
    #[ignore = "a measurement"]
    fn ideal() {
        for (restart, keeps) in [
            (40usize, vec![0usize, 10, 20, 30]),
            (80, vec![0, 20, 40]),
            (120, vec![80]),
        ] {
            let mg = Multigrid {
                restart,
                ..Multigrid::default()
            };
            let (solver, ports) = coupler(1.55, 0.05, 1.0, 6, 1, Solve::Multigrid(mg)).unwrap();
            let stopping = Stopping {
                tolerance: 1e-8,
                max_iterations: 100_000,
            };
            let rhs = solver.mode_source(&ports[0].mode, crate::fdfd::Direction::Forward);
            let rhs2 = solver.mode_source(&ports[1].mode, crate::fdfd::Direction::Backward);
            for &k in &keeps {
                let mut r = Recycler::new().with_krylov(k);
                let a = solver
                    .solve_system_recycled(&rhs, stopping, &mut r)
                    .unwrap()
                    .1;
                let b = solver
                    .solve_system_recycled(&rhs, stopping, &mut r)
                    .unwrap()
                    .1;
                let c = solver
                    .solve_system_recycled(&rhs2, stopping, &mut r)
                    .unwrap()
                    .1;
                eprintln!(
                    "restart {restart} k {k}: first {} again {} other port {}",
                    a.iterations, b.iterations, c.iterations
                );
            }
        }
    }

    /// Measures a forward solve and its adjoint: `cargo test --release --lib
    /// recycle_checks::tests::pair -- --ignored --nocapture`, RECYCLE_H and RECYCLE_LENGTH as
    /// for `measure`, RECYCLE_KEEP ("10,20").
    #[test]
    #[ignore = "a measurement"]
    fn pair() {
        let var = |name: &str, default: &str| std::env::var(name).unwrap_or(default.into());
        let h: f64 = var("RECYCLE_H", "0.05").parse().unwrap();
        let length: f64 = var("RECYCLE_LENGTH", "1.0").parse().unwrap();
        let keeps: Vec<usize> = var("RECYCLE_KEEP", "10,20,30")
            .split(',')
            .map(|k| k.parse().unwrap())
            .collect();
        let rounds: usize = var("RECYCLE_ROUNDS", "2").parse().unwrap();
        let (solver, ports) = coupler(
            1.55,
            h,
            length,
            6,
            1,
            Solve::Multigrid(Multigrid::default()),
        )
        .unwrap();
        eprintln!(
            "{:?}, {} unknowns, {} threads",
            solver.grid(),
            solver.grid().unknowns(),
            rayon::current_num_threads()
        );
        let stopping = Stopping {
            tolerance: 1e-8,
            max_iterations: 100_000,
        };
        let rhs = solver.mode_source(&ports[0].mode, crate::fdfd::Direction::Forward);
        let ahead = &ports[1].mode;
        for round in 0..rounds {
            let start = std::time::Instant::now();
            let (field, forward) = solver.solve_system(&rhs, stopping).unwrap();
            let (_, plain_gradient, adjoint) = solver
                .mode_power_gradient(&field, ahead, crate::fdfd::Direction::Forward, stopping)
                .unwrap();
            eprintln!(
                "round {round} plain: forward {} adjoint {} in {:.2} s",
                forward.iterations,
                adjoint.iterations,
                start.elapsed().as_secs_f64()
            );
            for &k in &keeps {
                let mut r = Recycler::new().with_krylov(k);
                let start = std::time::Instant::now();
                let (field, forward) = solver
                    .solve_system_recycled(&rhs, stopping, &mut r)
                    .unwrap();
                let (_, gradient, adjoint) = solver
                    .mode_power_gradient_recycled(
                        &field,
                        ahead,
                        crate::fdfd::Direction::Forward,
                        stopping,
                        &mut r,
                    )
                    .unwrap();
                let largest = plain_gradient.iter().fold(0.0f64, |m, g| m.max(g.abs()));
                let worst = plain_gradient
                    .iter()
                    .zip(&gradient)
                    .fold(0.0f64, |m, (p, q)| m.max((p - q).abs()));
                eprintln!(
                    "round {round} k {k}: forward {} adjoint {} in {:.2} s, gradients within {:.1e}",
                    forward.iterations,
                    adjoint.iterations,
                    start.elapsed().as_secs_f64(),
                    worst / largest
                );
            }
        }
    }

    /// Measures an optimization's sequence of designs, a forward solve and its adjoint each:
    /// `cargo test --release --lib recycle_checks::tests::designs -- --ignored --nocapture`,
    /// RECYCLE_SOLVE, RECYCLE_H, RECYCLE_LENGTH, RECYCLE_STEPS, RECYCLE_DELTA (the design's step
    /// in permittivity) and RECYCLE_CASES as for `measure`.
    #[test]
    #[ignore = "a measurement"]
    fn designs() {
        let var = |name: &str, default: &str| std::env::var(name).unwrap_or(default.into());
        let h: f64 = var("RECYCLE_H", "0.05").parse().unwrap();
        let length: f64 = var("RECYCLE_LENGTH", "1.0").parse().unwrap();
        let steps: usize = var("RECYCLE_STEPS", "8").parse().unwrap();
        let delta: f64 = var("RECYCLE_DELTA", "0.02").parse().unwrap();
        let solve = match var("RECYCLE_SOLVE", "mg").as_str() {
            "sym" => Solve::Symmetric,
            _ => Solve::Multigrid(Multigrid::default()),
        };
        let cases: Vec<(usize, usize)> = var("RECYCLE_CASES", "0,2")
            .split(';')
            .map(|c| {
                let mut p = c.split(',').map(|x| x.parse().unwrap());
                (p.next().unwrap(), p.next().unwrap())
            })
            .collect();
        let stopping = Stopping {
            tolerance: 1e-8,
            max_iterations: 100_000,
        };
        let forward_direction = crate::fdfd::Direction::Forward;
        let run = |recycler: Option<&mut Recycler>| {
            let mut recycler = recycler;
            let mut counts = Vec::new();
            let mut seconds = 0.0;
            for step in 0..steps {
                let (solver, ports) = Coupler {
                    wavelength: 1.55,
                    design: delta * step as f64,
                    h,
                    length,
                    pml: 6,
                    modes: 1,
                    solve,
                }
                .build()
                .unwrap();
                let rhs = solver.mode_source(&ports[0].mode, forward_direction);
                let start = std::time::Instant::now();
                let (f, a) = match recycler.as_deref_mut() {
                    Some(r) => {
                        let (field, f) = solver.solve_system_recycled(&rhs, stopping, r).unwrap();
                        let (_, _, a) = solver
                            .mode_power_gradient_recycled(
                                &field,
                                &ports[1].mode,
                                forward_direction,
                                stopping,
                                r,
                            )
                            .unwrap();
                        (f, a)
                    }
                    None => {
                        let (field, f) = solver.solve_system(&rhs, stopping).unwrap();
                        let (_, _, a) = solver
                            .mode_power_gradient(
                                &field,
                                &ports[1].mode,
                                forward_direction,
                                stopping,
                            )
                            .unwrap();
                        (f, a)
                    }
                };
                seconds += start.elapsed().as_secs_f64();
                counts.push((f.iterations, a.iterations));
            }
            (counts, seconds)
        };
        let (counts, seconds) = run(None);
        eprintln!("plain: {counts:?} in {seconds:.2} s");
        for (k, s) in cases {
            let mut r = Recycler::new().with_krylov(k).with_solutions(s);
            let (counts, seconds) = run(Some(&mut r));
            eprintln!("k {k} solutions {s}: {counts:?} in {seconds:.2} s");
        }
    }

    /// Measures a sweep: `cargo test --release --lib recycle_checks::tests::measure --
    /// --ignored --nocapture`, with RECYCLE_SOLVE (mg or sym), RECYCLE_POINTS, RECYCLE_STEP (µm),
    /// RECYCLE_H (µm), RECYCLE_LENGTH (µm), RECYCLE_TOL and RECYCLE_CASES ("k,s;k,s").
    #[test]
    #[ignore = "a measurement, minutes long"]
    fn measure() {
        let var = |name: &str, default: &str| std::env::var(name).unwrap_or(default.into());
        let points: usize = var("RECYCLE_POINTS", "8").parse().unwrap();
        let step: f64 = var("RECYCLE_STEP", "0.01").parse().unwrap();
        let h: f64 = var("RECYCLE_H", "0.05").parse().unwrap();
        let length: f64 = var("RECYCLE_LENGTH", "1.0").parse().unwrap();
        let tol: f64 = var("RECYCLE_TOL", "1e-8").parse().unwrap();
        let solve = match var("RECYCLE_SOLVE", "mg").as_str() {
            "sym" => Solve::Symmetric,
            _ => Solve::Multigrid(Multigrid::default()),
        };
        let cases: Vec<(usize, usize)> = var("RECYCLE_CASES", "0,2")
            .split(';')
            .map(|c| {
                let mut p = c.split(',').map(|x| x.parse().unwrap());
                (p.next().unwrap(), p.next().unwrap())
            })
            .collect();
        let wavelengths: Vec<f64> = (0..points).map(|i| 1.50 + step * i as f64).collect();
        let build = |lam: f64| coupler(lam, h, length, 6, 1, solve);
        let (first, _) = build(1.5).unwrap();
        eprintln!(
            "{:?}, {} unknowns, {} threads",
            first.grid(),
            first.grid().unknowns(),
            rayon::current_num_threads()
        );
        drop(first);
        let total = |s: &Sweep| iterations(s).iter().sum::<usize>();
        let rounds: usize = var("RECYCLE_ROUNDS", "1").parse().unwrap();
        for round in 0..rounds {
            let plain = sweep(&wavelengths, &build, tol, None).unwrap();
            eprintln!(
                "round {round} plain: {:?} = {} in {:.2} s (builds {:.2} s)",
                iterations(&plain),
                total(&plain),
                plain.seconds,
                plain.building
            );
            for &(k, s) in &cases {
                let mut r = Recycler::new().with_krylov(k).with_solutions(s);
                let rec = sweep(&wavelengths, &build, tol, Some(&mut r)).unwrap();
                eprintln!(
                    "round {round} k {k} solutions {s}: {:?} = {} carried {} in {:.2} s (builds \
                     {:.2} s), S within {:.1e}",
                    iterations(&rec),
                    total(&rec),
                    rec.carried,
                    rec.seconds,
                    rec.building,
                    largest_difference(&plain, &rec)
                );
            }
        }
    }
}
