//! `auto` for the iterative solves when the process prefers a backend
//! (`backend::prefer_iterative`), in a test binary of its own: the preference is the process's,
//! and the library's own tests, run side by side in theirs, must not see it.

use std::sync::Arc;

use num_complex::Complex64 as c64;
use photonoxide::Result;
use photonoxide::backend::{
    self, Capabilities, Choice, IterativeSolver, Matrix, QmrRun, prefer_iterative,
};
use photonoxide::fdfd::{
    Axis, Boundaries3d, Formulation, Grid3d, IterativeSolver3d, Multigrid, Stopping,
};
use photonoxide::units::Wavelength;

const NAME: &str = "answers-no-for-tests";

/// A backend that refuses every run, naming itself, so a solve that went to it says so.
struct AnswersNo;

impl IterativeSolver for AnswersNo {
    fn capabilities(&self) -> Capabilities {
        Capabilities::new(NAME, "1", "MIT OR Apache-2.0")
    }

    fn qmr_run(&self, _: &Matrix<'_>, _: &[c64], _: Stopping) -> Result<QmrRun> {
        Err(photonoxide::Error::InvalidValue {
            what: "tests",
            reason: format!("{NAME} ran"),
        })
    }
}

/// Relative permittivity at (x, y, z), µm.
type Permittivity = fn(f64, f64, f64) -> c64;

fn problem() -> (Grid3d, Permittivity, Vec<c64>) {
    let (n, h) = (12, 0.04);
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
    let eps = |_: f64, y: f64, z: f64| {
        c64::new(
            if y.abs() < 0.1 && z.abs() < 0.1 {
                12.09
            } else {
                2.085
            },
            0.0,
        )
    };
    let mut current = vec![c64::new(0.0, 0.0); grid.unknowns()];
    current[grid.index(Axis::X, (n / 2, n / 2, n / 2))] = c64::new(1.0, 0.0);
    (grid, eps, current)
}

fn solver(formulation: Formulation) -> IterativeSolver3d {
    let (grid, eps, _) = problem();
    IterativeSolver3d::new(
        grid,
        Wavelength::um(1.55).unwrap(),
        eps,
        Boundaries3d::stretched_pml(3),
        formulation,
    )
    .unwrap()
}

#[test]
fn auto_runs_on_the_preferred_backend_only_where_it_runs_the_solve() {
    backend::register_iterative(Arc::new(AnswersNo)).unwrap();
    let (grid, eps, current) = problem();
    let stop = Stopping {
        tolerance: 1e-6,
        max_iterations: 20_000,
    };
    let went_to_it = |r: Result<_>| match r {
        Err(e) => e.to_string().contains(NAME),
        Ok(_) => false,
    };
    // not preferred: auto is photonoxide's own
    assert!(solver(Formulation::CurlCurl).solve(&current, stop).is_ok());
    prefer_iterative(&Choice::Named(NAME.into())).unwrap();
    assert_eq!(
        backend::preferred_iterative().map(|s| s.capabilities().name),
        Some(NAME.into())
    );
    // plain QMR and QMR with ILU(0), as auto, go to it
    assert!(went_to_it(
        solver(Formulation::CurlCurl).solve(&current, stop)
    ));
    let ilu = solver(Formulation::ShinFan).with_ilu().unwrap();
    assert!(went_to_it(ilu.solve(&current, stop)));
    let auto = solver(Formulation::CurlCurl)
        .with_iterative_backend(&Choice::Auto)
        .unwrap();
    assert!(went_to_it(auto.solve(&current, stop)));
    // photonoxide named is photonoxide's own
    let own = solver(Formulation::CurlCurl)
        .with_iterative_backend(&Choice::Photonoxide)
        .unwrap();
    assert!(own.solve(&current, stop).is_ok());
    // what it doesn't run stays photonoxide's: the multigrid (it runs no GMRES), block QMR,
    // and a solver without the matrix
    let multigrid = solver(Formulation::ShinFan)
        .with_multigrid(Multigrid::default())
        .unwrap();
    assert!(multigrid.solve(&current, stop).is_ok());
    let rhs = vec![current.clone(), current.iter().map(|z| z * 2.0).collect()];
    assert!(
        solver(Formulation::CurlCurl)
            .solve_systems(&rhs, stop)
            .is_ok()
    );
    let free = IterativeSolver3d::matrix_free(
        grid,
        Wavelength::um(1.55).unwrap(),
        eps,
        Boundaries3d::stretched_pml(3),
    )
    .unwrap();
    assert!(free.solve(&current, stop).is_ok());
    // and auto is photonoxide's own again
    prefer_iterative(&Choice::Auto).unwrap();
    assert!(backend::preferred_iterative().is_none());
    assert!(solver(Formulation::CurlCurl).solve(&current, stop).is_ok());
    // a name nothing is registered under is refused, the preference unchanged
    assert!(prefer_iterative(&Choice::Named("nothing-here".into())).is_err());
    assert!(backend::preferred_iterative().is_none());
}
