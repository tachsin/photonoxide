//! Fixed problems to time the solvers on: the benchmark harness of the performance plan
//! (docs/plans/performance.md, Phase A0), which `photonoxide bench` runs.
//!
//! Each [`Problem`] builds one structure on one grid, always the same, solves it and returns a
//! [`Measurement`]: the time of each phase, QMR's iterations, and the error of what it computed
//! against an exact answer or a converged reference, so that a faster solve counts only at equal
//! accuracy. [`triad`] measures the memory bandwidth the bandwidth-bound solvers are judged
//! against: the roof of S. Williams, A. Waterman, D. Patterson, "Roofline: an insightful visual
//! performance model for multicore architectures", Commun. ACM 52(4), 65 (2009),
//! doi:10.1145/1498765.1498785, measured by the triad of J. D. McCalpin's STREAM ("Memory
//! bandwidth and machine balance in current high performance computers", IEEE TCCA Newsletter,
//! December 1995, which has no DOI).
//!
//! Times depend on the machine and on what else runs on it, so they are never part of the
//! validation report; the errors don't, and are what a change to a solver must keep.

use std::time::Instant;

use num_complex::Complex64 as c64;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::fdfd::{
    Axis, Boundaries, Boundaries3d, Formulation, Grid, Grid3d, IterativeSolver3d, Polarization,
    Port, Port3d, Side, Solver2d, Stopping,
};
use crate::units::Wavelength;

/// One fixed problem.
#[derive(Clone, Copy, Debug)]
pub struct Problem {
    /// A short, stable identifier, e.g. `"fdfd3d/guide-ilu"`.
    pub id: &'static str,
    /// What is solved, and how, in a sentence.
    pub title: &'static str,
    /// Whether it takes minutes rather than seconds: `photonoxide bench` runs it only when asked.
    pub heavy: bool,
    run: fn(&mut dyn FnMut()) -> Result<Measurement>,
}

impl Problem {
    /// Builds and solves the problem, timing each phase.
    ///
    /// # Errors
    ///
    /// Whatever the solver returns: none is expected, so an error is a regression.
    pub fn run(&self) -> Result<Measurement> {
        (self.run)(&mut || {})
    }

    /// As [`Problem::run`], calling `timed` once the timed phases are over, before what checks
    /// them: a reference solve can take more memory than the solve it checks, so the peak memory
    /// to report is the one `timed` reads.
    ///
    /// # Errors
    ///
    /// As [`Problem::run`].
    pub fn run_with(&self, timed: &mut dyn FnMut()) -> Result<Measurement> {
        (self.run)(timed)
    }
}

/// What a problem measured.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    /// The grid, e.g. `"40 × 40 × 40 cells of 10 nm, PMLs of 10"`: no time without its grid.
    pub grid: String,
    /// The unknowns of the system solved.
    pub unknowns: usize,
    /// The timed phases, in order.
    pub phases: Vec<Phase>,
    /// What was computed, against what it should be.
    pub accuracy: Option<Accuracy>,
}

impl Measurement {
    /// The time of all the phases, in seconds.
    pub fn seconds(&self) -> f64 {
        self.phases.iter().map(|p| p.seconds).sum()
    }

    /// QMR's iterations over all the phases, if any phase iterated.
    pub fn iterations(&self) -> Option<usize> {
        self.phases
            .iter()
            .filter_map(|p| p.iterations)
            .reduce(|a, b| a + b)
    }

    /// The iterating phases' time per unknown per iteration, in nanoseconds: what a kernel
    /// change moves.
    pub fn nanoseconds_per_unknown_iteration(&self) -> Option<f64> {
        let (seconds, iterations) = self
            .phases
            .iter()
            .filter_map(|p| p.iterations.map(|i| (p.seconds, i)))
            .fold((0.0, 0), |(s, n), (t, i)| (s + t, n + i));
        (iterations > 0).then(|| seconds * 1e9 / (self.unknowns as f64 * iterations as f64))
    }
}

/// One timed phase of a problem.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Phase {
    /// What it does, e.g. `"assembly and ILU(0)"`.
    pub name: String,
    /// Its wall-clock time.
    pub seconds: f64,
    /// QMR's iterations in it, if it iterated.
    pub iterations: Option<usize>,
}

/// The error of what a problem computed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Accuracy {
    /// The error.
    pub error: f64,
    /// What it is measured against, e.g. `"S21 = exp(iβL), S11 = 0"`.
    pub against: String,
}

/// Every problem, in the order `photonoxide bench` runs them.
pub fn problems() -> Vec<Problem> {
    vec![
        Problem {
            id: "fdfd2d/slab-lu",
            title: "2D FDFD by sparse LU: a straight 220 nm silicon slab in oxide, E along z, \
                    its two ports' S-matrix",
            heavy: false,
            run: |timed| slab_2d(0.01, 4.0, timed),
        },
        Problem {
            id: "fdfd3d/guide-qmr",
            title: "3D FDFD by QMR on the curl-curl operator, plain PMLs: a 100 nm silicon guide \
                    through a 40³ grid, a dipole beside it, to a residual of 1e-6",
            heavy: false,
            run: |timed| guide_3d(Guide::Cube { core: 10 }, 10, Solve::Qmr, 1e-6, timed),
        },
        Problem {
            id: "fdfd3d/guide-ilu",
            title: "3D FDFD by QMR + ILU(0) on Shin and Fan's operator, stretched PMLs: the same \
                    guide, to a residual of 1e-8",
            heavy: false,
            run: |timed| guide_3d(Guide::Cube { core: 10 }, 10, Solve::Ilu, 1e-8, timed),
        },
        Problem {
            id: "fdfd3d/diel-ilu",
            title: "3D FDFD by QMR + ILU(0), stretched PMLs: Shin and Fan's Diel, smaller (a \
                    400 × 300 nm silicon guide in vacuum, a current across it), to 1e-8",
            heavy: true,
            run: |timed| guide_3d(Guide::Diel { length: 40 }, 10, Solve::Ilu, 1e-8, timed),
        },
        Problem {
            id: "fdfd3d/strip-ports-ilu",
            title: "3D FDFD by QMR + ILU(0), stretched PMLs: a straight 500 × 220 nm silicon \
                    strip in oxide, its two ports' S-matrix, to 1e-8",
            heavy: true,
            run: |timed| strip_ports_3d(0.02, 16, 1e-8, timed),
        },
    ]
}

/// The STREAM triad, a = b + s c over `len` doubles, on rayon's threads: the best of `repeats`
/// runs, in GB/s, counting 24 bytes an element as STREAM does (two reads and a write).
pub fn triad(len: usize, repeats: usize) -> f64 {
    const CHUNK: usize = 1 << 14;
    let b = vec![1.0_f64; len];
    let c = vec![2.0_f64; len];
    let mut a = vec![0.0_f64; len];
    let s = std::hint::black_box(3.0);
    let mut best = f64::INFINITY;
    for _ in 0..repeats.max(1) {
        let t = Instant::now();
        a.par_chunks_mut(CHUNK)
            .zip(b.par_chunks(CHUNK).zip(c.par_chunks(CHUNK)))
            .for_each(|(a, (b, c))| {
                for ((a, b), c) in a.iter_mut().zip(b).zip(c) {
                    *a = b + s * c;
                }
            });
        std::hint::black_box(&a);
        best = best.min(t.elapsed().as_secs_f64());
    }
    24.0 * len as f64 / best / 1e9
}

/// Times one phase.
fn timed<T>(name: &str, phases: &mut Vec<Phase>, f: impl FnOnce() -> Result<T>) -> Result<T> {
    let t = Instant::now();
    let value = f()?;
    phases.push(Phase {
        name: name.into(),
        seconds: t.elapsed().as_secs_f64(),
        iterations: None,
    });
    Ok(value)
}

fn nm(h: f64) -> f64 {
    (h * 1e3).round()
}

/// A straight slab (220 nm of 3.476 in 1.444) along x, `length` µm and ±1.5 µm across on an
/// h grid, PMLs of 20 cells, E along z, at 1.55 µm; its ports 0.3 µm in from each end. The exact
/// S-matrix is S21 = S12 = exp(iβL), L the ports' distance, and S11 = S22 = 0.
fn slab_2d(h: f64, length: f64, timed_done: &mut dyn FnMut()) -> Result<Measurement> {
    let pml = 20;
    let grid = Grid {
        nx: (length / h).round() as usize + 2 * pml,
        ny: (3.0 / h).round() as usize + 2 * pml,
        dx: h,
        dy: h,
        x0: -(pml as f64) * h,
        y0: -1.5 - pml as f64 * h,
    };
    let eps = |_: f64, y: f64| {
        let n: f64 = if y.abs() < 0.11 { 3.476 } else { 1.444 };
        c64::new(n * n, 0.0)
    };
    let lam = Wavelength::um(1.55)?;
    let mut phases = Vec::new();
    let solver = timed("assembly and LU", &mut phases, || {
        Solver2d::new(grid, Polarization::Ez, lam, eps, Boundaries::pml(pml))
    })?;
    let column = |x: f64| ((x - grid.x0) / h).floor() as usize;
    let (left, right) = (column(0.3), column(length - 0.3));
    let ports = timed("port modes", &mut phases, || {
        let mode = |c: usize| -> Result<_> { Ok(solver.port_modes(c, 1)?.remove(0)) };
        Ok([
            Port {
                mode: mode(left)?,
                side: Side::Left,
            },
            Port {
                mode: mode(right)?,
                side: Side::Right,
            },
        ])
    })?;
    let s = timed("S-matrix, 2 runs", &mut phases, || solver.s_matrix(&ports))?;
    timed_done();
    let through = (c64::new(0.0, 1.0) * ports[0].mode.beta() * ((right - left) as f64 * h)).exp();
    let error = [
        s[0][0].norm(),
        s[1][1].norm(),
        (s[1][0] - through).norm(),
        (s[0][1] - through).norm(),
    ]
    .into_iter()
    .fold(0.0, f64::max);
    Ok(Measurement {
        grid: format!(
            "{} × {} cells of {} nm, PMLs of {pml}",
            grid.nx,
            grid.ny,
            nm(h)
        ),
        unknowns: grid.nx * grid.ny,
        phases,
        accuracy: Some(Accuracy {
            error,
            against: "S21 = exp(iβL), S11 = 0".into(),
        }),
    })
}

/// The silicon guides of the 3D iterative problems, in vacuum on a 10 nm grid.
#[derive(Clone, Copy, Debug)]
enum Guide {
    /// A square guide `core` cells across along x through a cube of 2 `core` cells inside its
    /// PMLs, and an x-directed dipole 3 cells off the centre on each axis (the iterative solver's
    /// tests' 40³ guide for `core` 10 and PMLs of 10).
    Cube { core: usize },
    /// Shin and Fan's Diel (Opt. Express 21, 22578 (2013), Fig. 9b), smaller: a 400 × 300 nm
    /// guide along x, 15 cells of vacuum beside it inside the PMLs, `length` cells long, and a
    /// y-polarized current across it 15 cells in.
    Diel { length: usize },
}

/// How the 3D problem is solved.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Solve {
    /// QMR on the curl-curl operator, plain PMLs.
    Qmr,
    /// QMR + ILU(0) on Shin and Fan's operator, stretched PMLs.
    Ilu,
}

/// A guide's field from one source, solved to `tolerance`; its error is against the same
/// problem (the same PMLs) solved by QMR + ILU(0) on Shin and Fan's operator to 1e-12, which
/// isn't timed. Both operators have the same solution.
fn guide_3d(
    guide: Guide,
    pml: usize,
    solve: Solve,
    tolerance: f64,
    timed_done: &mut dyn FnMut(),
) -> Result<Measurement> {
    let h = 0.01;
    let (n, (wy, wz)) = match guide {
        Guide::Cube { core } => {
            let half = core as f64 * h / 2.0;
            ([2 * core + 2 * pml; 3], (half, half))
        }
        Guide::Diel { length } => ([length, 70 + 2 * pml, 60 + 2 * pml], (0.2, 0.15)),
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
    let eps = move |_: f64, y: f64, z: f64| {
        c64::new(
            if y.abs() < wy && z.abs() < wz {
                12.09
            } else {
                1.0
            },
            0.0,
        )
    };
    let lam = Wavelength::um(1.55)?;
    let mut source = vec![c64::new(0.0, 0.0); grid.unknowns()];
    match guide {
        Guide::Cube { .. } => {
            source[grid.index(Axis::X, (n[0] / 2 + 3, n[1] / 2 + 3, n[2] / 2 + 3))] =
                c64::new(1.0, 0.0);
        }
        Guide::Diel { .. } => {
            for k in 0..grid.nz {
                for j in 0..grid.ny {
                    let [_, y, z] = grid.e_position(Axis::Y, (15, j, k));
                    if y.abs() < wy && z.abs() < wz {
                        source[grid.index(Axis::Y, (15, j, k))] = c64::new(1.0, 0.0);
                    }
                }
            }
        }
    }
    let boundaries = match solve {
        Solve::Qmr => Boundaries3d::pml(pml),
        Solve::Ilu => Boundaries3d::stretched_pml(pml),
    };
    let stopping = |tolerance: f64| Stopping {
        tolerance,
        max_iterations: 100_000,
    };
    let mut phases = Vec::new();
    let field = {
        let solver = match solve {
            Solve::Qmr => timed("assembly", &mut phases, || {
                IterativeSolver3d::new(grid, lam, eps, boundaries, Formulation::CurlCurl)
            })?,
            Solve::Ilu => timed("assembly and ILU(0)", &mut phases, || {
                IterativeSolver3d::new(grid, lam, eps, boundaries, Formulation::ShinFan)?.with_ilu()
            })?,
        };
        let t = Instant::now();
        let (field, convergence) = solver.solve(&source, stopping(tolerance))?;
        phases.push(Phase {
            name: format!("QMR to {tolerance:e}"),
            seconds: t.elapsed().as_secs_f64(),
            iterations: Some(convergence.iterations),
        });
        field
    };
    timed_done();
    let reference =
        IterativeSolver3d::new(grid, lam, eps, boundaries, Formulation::ShinFan)?.with_ilu()?;
    let (reference, _) = reference.solve(&source, stopping(1e-12))?;
    let difference: f64 = (field.values().iter())
        .zip(reference.values())
        .map(|(a, b)| (a - b).norm_sqr())
        .sum();
    let size: f64 = reference.values().iter().map(|z| z.norm_sqr()).sum();
    let error = (difference / size).sqrt();
    Ok(Measurement {
        grid: format!(
            "{} × {} × {} cells of {} nm, {} of {pml}",
            n[0],
            n[1],
            n[2],
            nm(h),
            match solve {
                Solve::Qmr => "PMLs",
                Solve::Ilu => "stretched PMLs",
            }
        ),
        unknowns: grid.unknowns(),
        phases,
        accuracy: Some(Accuracy {
            error,
            against: "QMR + ILU(0) to 1e-12, relative".into(),
        }),
    })
}

/// A straight strip (500 × 220 nm of 3.476 in 1.444) along x on an h grid, 40 × 70 × 50 cells
/// inside stretched PMLs of `pml` cells, at 1.55 µm; ports 5 cells in from the left PML and 35.
/// Its exact S-matrix is S21 = S12 = exp(iβL), L the ports' distance, and S11 = S22 = 0.
fn strip_ports_3d(
    h: f64,
    pml: usize,
    tolerance: f64,
    timed_done: &mut dyn FnMut(),
) -> Result<Measurement> {
    let (nx, ny, nz) = (40 + 2 * pml, 70 + 2 * pml, 50 + 2 * pml);
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
    let strip = |_: f64, y: f64, z: f64| {
        let n: f64 = if y.abs() < 0.25 && z.abs() < 0.11 {
            3.476
        } else {
            1.444
        };
        c64::new(n * n, 0.0)
    };
    let lam = Wavelength::um(1.55)?;
    let mut phases = Vec::new();
    let solver = timed("assembly and ILU(0)", &mut phases, || {
        IterativeSolver3d::new(
            grid,
            lam,
            strip,
            Boundaries3d::stretched_pml(pml),
            Formulation::ShinFan,
        )?
        .with_ilu()
    })?;
    let (left, right) = (pml + 5, pml + 35);
    let ports = timed("port modes", &mut phases, || {
        let mode = |p: usize| -> Result<_> { Ok(solver.port_modes(Axis::X, p, 1)?.remove(0)) };
        Ok([
            Port3d {
                mode: mode(left)?,
                side: Side::Left,
            },
            Port3d {
                mode: mode(right)?,
                side: Side::Right,
            },
        ])
    })?;
    let stopping = Stopping {
        tolerance,
        max_iterations: 100_000,
    };
    let t = Instant::now();
    let (s, runs) = solver.s_matrix_with_convergence(&ports, stopping)?;
    phases.push(Phase {
        name: format!("S-matrix, {} runs of QMR to {tolerance:e}", runs.len()),
        seconds: t.elapsed().as_secs_f64(),
        iterations: Some(runs.iter().map(|c| c.iterations).sum()),
    });
    timed_done();
    let through = (c64::new(0.0, 1.0) * ports[0].mode.beta() * ((right - left) as f64 * h)).exp();
    let error = [
        s[0][0].norm(),
        s[1][1].norm(),
        (s[1][0] - through).norm(),
        (s[0][1] - through).norm(),
    ]
    .into_iter()
    .fold(0.0, f64::max);
    Ok(Measurement {
        grid: format!(
            "{nx} × {ny} × {nz} cells of {} nm, stretched PMLs of {pml}",
            nm(h)
        ),
        unknowns: grid.unknowns(),
        phases,
        accuracy: Some(Accuracy {
            error,
            against: "S21 = exp(iβL), S11 = 0".into(),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_titled() {
        let all = problems();
        for (i, p) in all.iter().enumerate() {
            assert!(p.id.contains('/') && !p.title.is_empty(), "{}", p.id);
            assert!(all[..i].iter().all(|q| q.id != p.id), "{} twice", p.id);
        }
    }

    #[test]
    fn a_measurement_adds_its_phases_up() {
        let m = Measurement {
            grid: "1 cell".into(),
            unknowns: 1000,
            phases: vec![
                Phase {
                    name: "assembly".into(),
                    seconds: 1.0,
                    iterations: None,
                },
                Phase {
                    name: "QMR".into(),
                    seconds: 2.0,
                    iterations: Some(100),
                },
                Phase {
                    name: "QMR".into(),
                    seconds: 2.0,
                    iterations: Some(300),
                },
            ],
            accuracy: None,
        };
        assert_eq!(m.seconds(), 5.0);
        assert_eq!(m.iterations(), Some(400));
        // 4 s over 1000 unknowns and 400 iterations
        assert!((m.nanoseconds_per_unknown_iteration().unwrap() - 1e4).abs() < 1e-6);
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(serde_json::from_str::<Measurement>(&json).unwrap(), m);
    }

    #[test]
    fn the_triad_measures_a_bandwidth() {
        let gb_s = triad(1 << 16, 3);
        assert!(gb_s.is_finite() && gb_s > 0.0, "{gb_s}");
    }

    // the problems themselves, on small grids: each solves, and its error is what its grid allows

    #[test]
    fn the_2d_slab_has_its_exact_s_matrix() {
        let m = slab_2d(0.02, 2.0, &mut || {}).unwrap();
        assert_eq!(m.phases.len(), 3);
        assert_eq!(m.unknowns, 140 * 190);
        // the validation report's fdfd straight-guide case, on the same grid: 1e-3 or better
        let error = m.accuracy.unwrap().error;
        assert!(error < 1e-3, "{error}");
    }

    #[test]
    fn the_3d_guides_converge_to_their_reference() {
        for (guide, solve, tolerance) in [
            (Guide::Cube { core: 4 }, Solve::Qmr, 1e-6),
            (Guide::Cube { core: 4 }, Solve::Ilu, 1e-8),
        ] {
            let m = guide_3d(guide, 4, solve, tolerance, &mut || {}).unwrap();
            assert!(m.iterations().unwrap() > 0, "{guide:?}");
            // the field's error follows the residual QMR stopped at
            let error = m.accuracy.unwrap().error;
            assert!(error < tolerance * 1e3, "{guide:?} {solve:?}: {error}");
        }
    }
}
