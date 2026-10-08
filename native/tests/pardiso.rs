//! oneMKL's PARDISO against photonoxide's own solver, on this machine. Skipped, and saying so,
//! where oneMKL isn't found (CI has none).

use std::sync::Arc;

use num_complex::Complex64 as c64;
use photonoxide::Result;
use photonoxide::backend::{self, Analysis, Capabilities, Choice, DirectSolver, Form, Matrix};
use photonoxide::bench::Measurement;
use photonoxide::bench::catalogue::entry;
use photonoxide_native::{Pardiso, intel, offer, smoke_test};

fn pardiso() -> Option<Arc<Pardiso>> {
    match Pardiso::load() {
        Ok(p) => Some(Arc::new(p)),
        Err(reason) => {
            println!("skipped: oneMKL's PARDISO isn't here: {reason}");
            None
        }
    }
}

#[test]
fn it_finds_onemkl_and_sets_its_threading_layer_up() {
    let probe = intel::probe();
    println!("{probe}");
    let Some(pardiso) = pardiso() else { return };
    let c = pardiso.capabilities();
    println!("{c:?}");
    assert!(probe.version.is_some());
    assert_eq!(c.version, pardiso.mkl().version);
    assert!(c.symmetric && c.transpose);
}

#[test]
fn it_passes_the_smoke_test() {
    let Some(pardiso) = pardiso() else { return };
    let difference = smoke_test(pardiso.as_ref()).unwrap();
    println!("largest difference from photonoxide's: {difference:.1e}");
    assert!(difference < 1e-13);
}

/// PARDISO taking only the general form: the LU of a system photonoxide would give it as
/// complex symmetric.
struct General(Arc<Pardiso>);

impl DirectSolver for General {
    fn capabilities(&self) -> Capabilities {
        let mut c = self.0.capabilities();
        c.name = "pardiso-lu".into();
        c.symmetric = false;
        c
    }

    fn analyse(&self, matrix: &Matrix<'_>) -> Result<Option<Arc<dyn Analysis>>> {
        assert_eq!(matrix.form(), Form::General);
        self.0.analyse(matrix)
    }
}

/// The seconds of a direct solve's analysis and factorization.
fn factorization(m: &Measurement) -> f64 {
    m.phases
        .iter()
        .filter(|p| p.name.contains("factorization"))
        .map(|p| p.seconds)
        .sum()
}

/// The catalogue's entries solved by PARDISO, as L D Lᵀ where the entry's system is symmetric
/// (`pardiso`) and as LU (`pardiso-lu`), each checked by the entry's own accuracy check, and
/// photonoxide's own beside them: the analysis and factorization's seconds, and the error.
fn against_photonoxide(ids: &[&str]) {
    let Some(pardiso) = pardiso() else { return };
    assert!(offer(pardiso.clone()).unwrap());
    backend::register(Arc::new(General(pardiso))).unwrap();
    println!("on {} threads", rayon::current_num_threads());
    for id in ids {
        let e = entry(id).unwrap();
        let ours = e.run(&Choice::Photonoxide).unwrap();
        let mut line = format!(
            "{id} ({}, {} unknowns), analysis and factorization: photonoxide {:.3} s ({:.1e})",
            ours.grid,
            ours.unknowns,
            factorization(&ours),
            ours.accuracy.as_ref().unwrap().error
        );
        for name in ["pardiso", "pardiso-lu"] {
            let theirs = e.run(&Choice::Named(name.into())).unwrap();
            let error = theirs.accuracy.as_ref().unwrap().error;
            line += &format!(", {name} {:.3} s ({error:.1e})", factorization(&theirs));
            assert!(error <= e.tolerance(), "{id} with {name}: {error:e}");
        }
        println!("{line}");
    }
}

/// FDFD's systems from the benchmark's catalogue.
#[test]
fn it_solves_fdfd_systems_as_photonoxide_does() {
    against_photonoxide(&[
        "fdfd2d/slab-ez-100",
        "fdfd2d/ring-hz-100",
        "fdfd2d/slab-ez-bloch-100",
        "fdfd2d/slab-ez-400",
        "fdfd3d/strip-16",
        "fdfd3d/strip-28",
    ]);
}

/// The systems docs/baselines.md ran PARDISO on as an external program (`photonoxide bench
/// --export`), now through the backend: minutes, and up to 17 GB for `strip-40`.
#[test]
#[ignore = "heavy: run with --ignored"]
fn it_solves_the_exported_systems_as_photonoxide_does() {
    against_photonoxide(&["slab-2d", "strip-24", "strip-32", "strip-40"]);
}

/// A 2D Helmholtz operator with loss on an m × m grid, complex symmetric, or general with an
/// asymmetric coupling along x.
fn system(m: usize, form: Form) -> (Vec<usize>, Vec<usize>, Vec<c64>) {
    helmholtz([m, m, 1], form)
}

/// A Helmholtz operator with loss on a grid of these cells, 2D or 3D, as [`system`].
fn helmholtz(cells: [usize; 3], form: Form) -> (Vec<usize>, Vec<usize>, Vec<c64>) {
    let [mx, my, mz] = cells;
    let n = mx * my * mz;
    let strides = [1, mx, mx * my];
    let dimensions = cells.iter().filter(|&&m| m > 1).count() as f64;
    let (mut starts, mut rows, mut values) = (vec![0], Vec::new(), Vec::new());
    for j in 0..n {
        let at = [j % mx, j / mx % my, j / (mx * my)];
        let mut column: Vec<(usize, c64)> = Vec::new();
        for axis in 0..3 {
            if at[axis] > 0 {
                column.push((j - strides[axis], c64::new(1.0, 0.0)));
            }
            if at[axis] + 1 < cells[axis] {
                let v = if form == Form::General && axis == 0 {
                    c64::new(1.0, 0.1)
                } else {
                    c64::new(1.0, 0.0)
                };
                column.push((j + strides[axis], v));
            }
        }
        let diagonal = c64::new(-2.0 * dimensions + 0.3, 0.02 + 0.001 * (j % 11) as f64);
        column.push((j, diagonal));
        column.sort_by_key(|&(r, _)| r);
        for (r, v) in column {
            rows.push(r);
            values.push(v);
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

fn rhs(n: usize) -> Vec<c64> {
    (0..n)
        .map(|k| c64::new(1.0, k as f64 * 1e-3).powi((k % 3) as i32))
        .collect()
}

fn solve(solver: &dyn DirectSolver, m: &Matrix<'_>, b: &[c64], transpose: bool) -> Vec<c64> {
    let f = solver.analyse(m).unwrap().unwrap().factorize(m).unwrap();
    if transpose {
        f.solve_transpose(b).unwrap()
    } else {
        f.solve(b).unwrap()
    }
}

fn on_threads<T: Send>(threads: usize, f: impl FnOnce() -> T + Send) -> T {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap()
        .install(f)
}

#[test]
fn it_solves_both_forms_and_their_transposes_as_photonoxide_does() {
    let Some(pardiso) = pardiso() else { return };
    let own = backend::direct(&Choice::Photonoxide).unwrap();
    for form in [Form::General, Form::Symmetric] {
        let (s, r, v) = system(60, form);
        let m = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
        let b = rhs(m.n());
        for transpose in [false, true] {
            let theirs = solve(pardiso.as_ref(), &m, &b, transpose);
            let ours = solve(own.as_ref(), &m, &b, transpose);
            let difference = relative(&theirs, &ours);
            println!("{form:?}, transpose {transpose}: {difference:.1e} from photonoxide's");
            assert!(difference < 1e-10, "{form:?} {transpose}: {difference:e}");
        }
        // the transpose is the transpose: a general matrix's differs from its solve
        let (x, xt) = (
            solve(pardiso.as_ref(), &m, &b, false),
            solve(pardiso.as_ref(), &m, &b, true),
        );
        let apart = relative(&x, &xt);
        match form {
            Form::General => assert!(apart > 1e-3, "{apart:e}"),
            _ => assert_eq!(x, xt),
        }
    }
}

/// The same bits on every run and on every number of threads, as the capabilities declare.
#[test]
fn it_repeats_bit_for_bit_on_any_number_of_threads() {
    let Some(pardiso) = pardiso() else { return };
    let deterministic = pardiso.capabilities().deterministic;
    let most = std::thread::available_parallelism().map_or(4, |n| n.get());
    let mut counts = vec![1, 2, 3, 4, 8, most];
    counts.retain(|&t| t <= most);
    counts.dedup();
    for form in [Form::General, Form::Symmetric] {
        // 3D: fronts large enough for oneMKL's dense kernels to split among threads
        let (s, r, v) = helmholtz([32, 32, 32], form);
        let m = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
        let b = rhs(m.n());
        let first = on_threads(most, || solve(pardiso.as_ref(), &m, &b, false));
        let mut same = true;
        for &threads in &counts {
            for _ in 0..2 {
                let clock = std::time::Instant::now();
                let again = on_threads(threads, || solve(pardiso.as_ref(), &m, &b, false));
                let seconds = clock.elapsed().as_secs_f64();
                let apart = relative(&again, &first);
                println!(
                    "{form:?} on {threads} threads ({seconds:.2} s): {}",
                    if again == first {
                        "the same bits".to_owned()
                    } else {
                        format!("differs by {apart:.1e}")
                    }
                );
                same &= again == first;
                assert!(apart < 1e-12, "{apart:e}");
            }
        }
        if deterministic {
            assert!(same, "{form:?}: declared deterministic, and it isn't");
        }
    }
}

#[test]
fn a_singular_matrix_is_reported() {
    let Some(pardiso) = pardiso() else { return };
    let (s, r) = (vec![0, 1, 2], vec![0, 1]);
    let v = [c64::new(1.0, 0.0), c64::new(0.0, 0.0)];
    for form in [Form::General, Form::Symmetric] {
        let m = Matrix::new(2, &s, &r, &v, form).unwrap();
        match pardiso.analyse(&m).unwrap().unwrap().factorize(&m) {
            Ok(factors) => {
                let perturbed = factors.report().perturbed_pivots;
                println!("{form:?}: {perturbed} pivot perturbed");
                assert!(perturbed >= 1);
            }
            Err(e) => println!("{form:?}: {e}"),
        }
    }
    // and a regular one perturbs none
    let (s, r, v) = system(30, Form::General);
    let m = Matrix::new(s.len() - 1, &s, &r, &v, Form::General).unwrap();
    let factors = pardiso.analyse(&m).unwrap().unwrap().factorize(&m).unwrap();
    assert_eq!(factors.report().perturbed_pivots, 0);
    // a right-hand side of another length is refused, an empty one too
    for len in [0, m.n() - 1, m.n() + 1] {
        let b = vec![c64::new(1.0, 0.0); len];
        assert!(factors.solve(&b).is_err() && factors.solve_transpose(&b).is_err());
    }
}

#[test]
fn a_sweep_reuses_its_analysis_and_reports_its_factors() {
    let Some(pardiso) = pardiso() else { return };
    let own = backend::direct(&Choice::Photonoxide).unwrap();
    for form in [Form::General, Form::Symmetric] {
        let (s, r, mut v) = system(40, form);
        let m = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
        let analysis = pardiso.analyse(&m).unwrap().unwrap();
        let b = rhs(m.n());
        // two at once: the second takes a handle of its own, analysed anew
        let held = analysis.factorize(&m).unwrap();
        for step in 0..3 {
            v[0] += c64::new(0.1, 0.0);
            let m = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
            let f = analysis.factorize(&m).unwrap();
            let report = f.report();
            let entries = report.factor_entries.unwrap();
            let stored = if form == Form::General {
                v.len()
            } else {
                v.len().div_ceil(2)
            };
            assert!(entries >= stored, "{step}: {entries}");
            assert!(report.peak_memory_bytes.unwrap() > 0);
            assert!(report.analysis_seconds > 0.0 && report.factorization_seconds > 0.0);
            let x = f.solve(&b).unwrap();
            let ours = solve(own.as_ref(), &m, &b, false);
            assert!(relative(&x, &ours) < 1e-10, "{form:?} {step}");
        }
        assert!(held.solve(&b).unwrap().iter().all(|x| x.is_finite()));
        // another structure is refused
        let (s, r, v) = system(41, form);
        let other = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
        assert!(analysis.factorize(&other).is_err());
    }
}

#[test]
fn factorizations_on_several_threads_at_once_agree() {
    let Some(pardiso) = pardiso() else { return };
    let (s, r, v) = system(80, Form::General);
    let m = Matrix::new(s.len() - 1, &s, &r, &v, Form::General).unwrap();
    let b = rhs(m.n());
    let analysis = pardiso.analyse(&m).unwrap().unwrap();
    let alone = analysis.factorize(&m).unwrap().solve(&b).unwrap();
    let solutions: Vec<Vec<c64>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| scope.spawn(|| analysis.factorize(&m).unwrap().solve(&b).unwrap()))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for x in &solutions {
        assert!(relative(x, &alone) < 1e-12);
    }
}
