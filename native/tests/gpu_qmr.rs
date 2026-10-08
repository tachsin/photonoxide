//! photonoxide's QMR on the GPU against its own on the CPU. Skipped, and saying so, where
//! cuSPARSE, cuBLAS or a GPU isn't found (CI has none).

use std::sync::Arc;

use num_complex::Complex64 as c64;
use photonoxide::backend::{Choice, Form, IterativeSolver, Matrix, QmrRun};
use photonoxide::bench::catalogue::entry;
use photonoxide::fdfd::Stopping;
use photonoxide_native::{GpuQmr, offer_iterative, smoke_test_iterative};

fn gpu() -> Option<Arc<GpuQmr>> {
    match GpuQmr::load() {
        Ok(q) => Some(Arc::new(q)),
        Err(reason) => {
            println!("skipped: cuSPARSE and cuBLAS aren't here: {reason}");
            None
        }
    }
}

#[test]
fn it_passes_the_smoke_test() {
    let Some(gpu) = gpu() else { return };
    let difference = smoke_test_iterative(gpu.as_ref()).unwrap();
    println!("largest difference from photonoxide's direct solve: {difference:.1e}");
}

/// The catalogue's 3D guides by plain QMR: the GPU's field against the direct solver's (each
/// entry's own check), its iterations against the CPU's QMR, and both times.
#[test]
fn it_solves_the_3d_guides_as_photonoxide_does() {
    guides(&["fdfd3d-iterative/guide-qmr-20", "fdfd3d-iterative/guide-qmr-30"]);
}

/// The guide of 44 cells a side too: over an hour, most of it its reference fields to 1e-12.
#[test]
#[ignore]
fn it_solves_the_large_3d_guide() {
    guides(&["fdfd3d-iterative/guide-qmr-44"]);
}

fn guides(ids: &[&str]) {
    let Some(gpu) = gpu() else { return };
    assert!(offer_iterative(gpu).unwrap());
    for &id in ids {
        let e = entry(id).unwrap();
        let theirs = e.run(&Choice::Named("cusparse".into())).unwrap();
        let ours = e.run(&Choice::Photonoxide).unwrap();
        let qmr = |m: &photonoxide::bench::Measurement| {
            m.phases
                .iter()
                .find(|p| p.iterations.is_some())
                .map(|p| (p.seconds, p.iterations.unwrap()))
                .unwrap()
        };
        let ((t_gpu, it_gpu), (t_cpu, it_cpu)) = (qmr(&theirs), qmr(&ours));
        let error = theirs.accuracy.as_ref().unwrap().error;
        println!(
            "{id} ({}): GPU {t_gpu:.3} s, {it_gpu} iterations, error {error:.1e}; \
             CPU {t_cpu:.3} s, {it_cpu} iterations, error {:.1e}",
            theirs.grid,
            ours.accuracy.as_ref().unwrap().error
        );
        assert!(error <= e.tolerance(), "{id}: {error:e}");
        // the same recurrences, summed in another order: over a thousand Lanczos steps on an
        // indefinite system, rounding moves the count by a few percent (1394 against 1423)
        let apart = it_gpu.abs_diff(it_cpu);
        assert!(apart <= 3.max(it_cpu / 20), "{id}: {it_gpu} against {it_cpu}");
    }
}

/// A 1D Helmholtz operator with loss, complex symmetric, or general with an unsymmetric
/// coupling.
fn system(n: usize, form: Form) -> (Vec<usize>, Vec<usize>, Vec<c64>) {
    let (mut starts, mut rows, mut values) = (vec![0], Vec::new(), Vec::new());
    for j in 0..n {
        for i in j.saturating_sub(1)..(j + 2).min(n) {
            rows.push(i);
            values.push(if i == j {
                c64::new(-1.9, 0.05 + 0.01 * (j % 7) as f64)
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

#[test]
fn it_repeats_bit_for_bit_in_both_forms() {
    let Some(gpu) = gpu() else { return };
    for form in [Form::General, Form::Symmetric] {
        let (s, r, v) = system(20_000, form);
        let m = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
        let b: Vec<c64> = (0..m.n()).map(|k| c64::new(1.0, (k % 13) as f64)).collect();
        let stopping = Stopping {
            tolerance: 1e-8,
            max_iterations: 20_000,
        };
        let run = || match gpu.qmr_run(&m, &b, stopping).unwrap() {
            QmrRun::Done(x, how) => (x, how.iterations),
            QmrRun::Broken(_, h, d) => panic!("{form:?}: broke down at {} ({d:e})", h.len()),
        };
        let (first, iterations) = run();
        let (second, _) = run();
        println!("{form:?}: {iterations} iterations");
        assert_eq!(first, second, "{form:?}");
    }
}
