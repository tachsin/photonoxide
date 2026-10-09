//! The FDTD kernel's problems: a box of vacuum and a silicon guide, in f64 and f32.

use std::time::Instant;

use super::{Accuracy, Measurement, Phase};
use crate::Result;
use crate::fdfd::{Axis, Grid3d};
use crate::fdtd::{Blocking, Boundaries, Simulation, Yee};

/// What runs: the grid's cells along each axis, its cell (µm), whether a silicon guide runs
/// along x through it (else vacuum), the steps timed, and whether by tiles where they pay
/// ([`Blocking::auto`]) or the whole grid every step.
#[derive(Clone, Copy, Debug)]
pub(super) struct Run {
    pub(super) n: usize,
    pub(super) h: f64,
    pub(super) guide: bool,
    pub(super) steps: usize,
    pub(super) tiles: bool,
}

/// CPMLs of this many cells.
const CPML: usize = 8;

/// The problem, its fields a smooth bump of E_y in the middle and H̃ zero.
fn start(run: Run) -> Result<Simulation> {
    let Run { n, h, guide, .. } = run;
    let half = n as f64 * h / 2.0;
    let grid = Grid3d {
        nx: n,
        ny: n,
        nz: n,
        dx: h,
        dy: h,
        dz: h,
        x0: -half,
        y0: -half,
        z0: -half,
    };
    let eps = move |_: f64, y: f64, z: f64| {
        if guide && y.abs() < 0.25 && z.abs() < 0.11 {
            12.09
        } else {
            1.0
        }
    };
    let mut s = Simulation::new(grid, eps, Boundaries::cpml(CPML), 0.9)?;
    let width = 0.1 * n as f64 * h;
    let ey = s.e_mut(Axis::Y);
    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                let p = grid.e_position(Axis::Y, (i, j, k));
                let r2 = p.iter().map(|x| x * x).sum::<f64>();
                ey[(k * n + j) * n + i] = (-r2 / (width * width)).exp();
            }
        }
    }
    Ok(s)
}

/// The kernel alone stepping `run`'s problem in precision `T`, timed, then checked: f64
/// against the plain loops it replaced (the same bits: the largest difference is 0), f32
/// against f64 (the largest difference of E relative to its largest value).
pub(super) fn kernel<T: crate::fdtd::Real>(
    run: Run,
    timed_done: &mut dyn FnMut(),
) -> Result<Measurement> {
    let s = start(run)?;
    let mut yee = Yee::<T>::from_simulation(&s)?;
    let bytes = yee.bytes_per_step() as f64 * run.steps as f64;
    let blocking = run
        .tiles
        .then(|| {
            Blocking::auto(
                s.grid(),
                std::mem::size_of::<T>(),
                true,
                rayon::current_num_threads(),
            )
        })
        .flatten();
    let t = Instant::now();
    yee.run(run.steps, blocking);
    let how = match blocking {
        Some(b) => format!("by diamonds of {} rows, {} steps a half", b.rows, b.steps),
        None => "the whole grid each".into(),
    };
    let phases = vec![Phase {
        name: format!("{} steps, {how}", run.steps),
        seconds: t.elapsed().as_secs_f64(),
        iterations: Some(run.steps),
        bytes: Some(bytes),
    }];
    timed_done();
    let largest = |v: &[f64]| v.iter().fold(0.0f64, |m, x| m.max(x.abs()));
    let accuracy = if std::mem::size_of::<T>() == 8 {
        let mut reference = s;
        reference.use_reference_kernel();
        reference.run(run.steps);
        let worst = Axis::ALL
            .iter()
            .flat_map(|&c| {
                yee.e(c)
                    .iter()
                    .zip(reference.e(c))
                    .map(|(a, b)| (a.to_f64() - b).abs())
            })
            .fold(0.0f64, f64::max);
        Accuracy {
            error: worst,
            against: "the plain loops: the same bits".into(),
        }
    } else {
        let mut double = Yee::<f64>::from_simulation(&s)?;
        for _ in 0..run.steps {
            double.step();
        }
        let scale = Axis::ALL
            .iter()
            .map(|&c| largest(double.e(c)))
            .fold(0.0f64, f64::max);
        let worst = Axis::ALL
            .iter()
            .flat_map(|&c| {
                yee.e(c)
                    .iter()
                    .zip(double.e(c))
                    .map(|(a, b)| (a.to_f64() - b).abs())
            })
            .fold(0.0f64, f64::max);
        Accuracy {
            error: worst / scale,
            against: "f64, relative to the largest E".into(),
        }
    };
    Ok(Measurement {
        grid: format!(
            "{n} × {n} × {n} cells of {} nm, CPMLs of {CPML}, {}",
            super::nm(run.h),
            if run.guide {
                "a 500 × 220 nm silicon guide"
            } else {
                "vacuum"
            },
            n = run.n
        ),
        unknowns: s_cells(run),
        phases,
        accuracy: Some(accuracy),
        factor_entries: None,
    })
}

fn s_cells(run: Run) -> usize {
    run.n * run.n * run.n
}
