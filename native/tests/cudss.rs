//! cuDSS against photonoxide's own solver, on this machine's GPU. Skipped, and saying so, where
//! cuDSS or a GPU isn't found (CI has neither).

use std::sync::Arc;

use num_complex::Complex64 as c64;
use photonoxide::backend::{Choice, DirectSolver, Form, Matrix};
use photonoxide::bench::catalogue::entry;
use photonoxide_native::{Cudss, offer, smoke_test};

fn cudss() -> Option<Arc<Cudss>> {
    match Cudss::load() {
        Ok(c) => Some(Arc::new(c)),
        Err(reason) => {
            println!("skipped: cuDSS isn't here: {reason}");
            None
        }
    }
}

#[test]
fn it_passes_the_smoke_test() {
    let Some(cudss) = cudss() else { return };
    let difference = smoke_test(cudss.as_ref()).unwrap();
    println!("largest difference from photonoxide's: {difference:.1e}");
    assert!(difference < 1e-13);
}

/// FDFD's systems from the benchmark's catalogue, solved by cuDSS and checked by each one's own
/// accuracy check, beside photonoxide's time.
#[test]
fn it_solves_fdfd_systems_as_photonoxide_does() {
    let Some(cudss) = cudss() else { return };
    assert!(offer(cudss).unwrap());
    for id in [
        "fdfd2d/slab-ez-100",
        "fdfd2d/ring-hz-100",
        "fdfd2d/slab-ez-bloch-100",
        "fdfd2d/slab-ez-400",
        "fdfd3d/strip-16",
        "fdfd3d/strip-28",
    ] {
        let e = entry(id).unwrap();
        let theirs = e.run(&Choice::Named("cudss".into())).unwrap();
        let ours = e.run(&Choice::Photonoxide).unwrap();
        let error = theirs.accuracy.as_ref().unwrap().error;
        println!(
            "{id} ({}): cuDSS {:.3} s, error {error:.1e}; photonoxide {:.3} s, error {:.1e}",
            theirs.grid,
            theirs.seconds(),
            ours.seconds(),
            ours.accuracy.as_ref().unwrap().error
        );
        assert!(error <= e.tolerance(), "{id}: {error:e}");
    }
}

fn system(form: Form) -> (Vec<usize>, Vec<usize>, Vec<c64>) {
    // a 1D Helmholtz operator with loss, 2000 unknowns, a far coupling to make it general
    let n: usize = 2000;
    let (mut starts, mut rows, mut values) = (vec![0], Vec::new(), Vec::new());
    for j in 0..n {
        for i in j.saturating_sub(1)..(j + 2).min(n) {
            rows.push(i);
            values.push(if i == j {
                c64::new(-2.0 + 0.04, 0.01 * (j % 7) as f64)
            } else if i > j && form == Form::General {
                c64::new(1.0, 0.1)
            } else {
                c64::new(1.0, 0.0)
            });
        }
        starts.push(rows.len());
    }
    (starts, rows, values)
}

/// The largest difference, relative to `b`'s largest entry.
fn relative(a: &[c64], b: &[c64]) -> f64 {
    let scale = b.iter().map(|x| x.norm()).fold(0.0, f64::max);
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b).norm())
        .fold(0.0, f64::max)
        / scale
}

#[test]
fn it_repeats_closely_and_solves_the_transpose() {
    let Some(cudss) = cudss() else { return };
    let own = photonoxide::backend::direct(&Choice::Photonoxide).unwrap();
    for form in [Form::General] {
        let (s, r, v) = system(form);
        let m = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
        let b: Vec<c64> = (0..m.n()).map(|k| c64::new(1.0, k as f64 * 1e-3)).collect();
        let solve = |solver: &dyn DirectSolver, transpose: bool| {
            let f = solver.analyse(&m).unwrap().unwrap().factorize(&m).unwrap();
            if transpose {
                f.solve_transpose(&b).unwrap()
            } else {
                f.solve(&b).unwrap()
            }
        };
        for transpose in [false, true] {
            let first = solve(cudss.as_ref(), transpose);
            // not the same bits: cuDSS's deterministic mode is off (see its module)
            let again = solve(cudss.as_ref(), transpose);
            let repeat = relative(&first, &again);
            println!("{form:?}, transpose {transpose}: repeats differ by {repeat:.1e}");
            assert!(repeat < 1e-12, "{repeat:e}");
            let ours = solve(own.as_ref(), transpose);
            let difference = relative(&first, &ours);
            assert!(difference < 1e-10, "{form:?} {transpose}: {difference:e}");
        }
    }
}

#[test]
fn a_singular_matrix_reports_its_replaced_pivot_and_a_symmetric_one_goes_as_general() {
    let Some(cudss) = cudss() else { return };
    let (s, r) = (vec![0, 1, 2], vec![0, 1]);
    let v = [c64::new(1.0, 0.0), c64::new(0.0, 0.0)];
    let m = Matrix::new(2, &s, &r, &v, Form::General).unwrap();
    let factors = cudss.analyse(&m).unwrap().unwrap().factorize(&m).unwrap();
    assert_eq!(factors.report().perturbed_pivots, 1);
    let (s, r, v) = system(Form::General);
    let m = Matrix::new(s.len() - 1, &s, &r, &v, Form::General).unwrap();
    let factors = cudss.analyse(&m).unwrap().unwrap().factorize(&m).unwrap();
    assert_eq!(factors.report().perturbed_pivots, 0);
    let (s, r, v) = system(Form::Symmetric);
    let m = Matrix::new(s.len() - 1, &s, &r, &v, Form::Symmetric).unwrap();
    assert!(cudss.analyse(&m).unwrap().is_none());
}

#[test]
fn a_sweep_reuses_its_analysis() {
    let Some(cudss) = cudss() else { return };
    let (s, r, mut v) = system(Form::General);
    let m = Matrix::new(s.len() - 1, &s, &r, &v, Form::General).unwrap();
    let analysis = cudss.analyse(&m).unwrap().unwrap();
    for step in 0..3 {
        v[0] += c64::new(0.1, 0.0);
        let m = Matrix::new(s.len() - 1, &s, &r, &v, Form::General).unwrap();
        let f = analysis.factorize(&m).unwrap();
        let report = f.report();
        assert!(report.factor_entries.unwrap() >= v.len(), "{step}");
        assert!(report.peak_memory_bytes.unwrap() > 0);
        let x = f.solve(&vec![c64::new(1.0, 0.0); m.n()]).unwrap();
        assert!(x.iter().all(|x| x.is_finite()));
    }
}
