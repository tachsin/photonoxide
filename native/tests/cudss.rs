//! cuDSS against photonoxide's own solver, on this machine's GPU. Skipped, and saying so, where
//! cuDSS or a GPU isn't found (CI has neither).

use std::sync::{Arc, Mutex};

use num_complex::Complex64 as c64;
use photonoxide::Result;
use photonoxide::backend::{
    self, Analysis, Capabilities, Choice, DirectSolver, Factorization, Form, Matrix, Report,
};
use photonoxide::bench::Measurement;
use photonoxide::bench::catalogue::{Task, entry};
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
fn it_passes_the_smoke_test_in_both_forms() {
    let Some(cudss) = cudss() else { return };
    assert!(cudss.capabilities().symmetric);
    let difference = smoke_test(cudss.as_ref()).unwrap();
    println!("largest difference from photonoxide's: {difference:.1e}");
    assert!(difference < 1e-13);
}

/// cuDSS under another name, taking only the general form (`symmetric` false) or both, keeping
/// each factorization's report.
struct Recorded {
    cudss: Arc<Cudss>,
    name: &'static str,
    symmetric: bool,
    reports: Arc<Mutex<Vec<(Form, Report)>>>,
}

impl DirectSolver for Recorded {
    fn capabilities(&self) -> Capabilities {
        let mut c = self.cudss.capabilities();
        c.name = self.name.into();
        c.symmetric = self.symmetric;
        c
    }

    fn analyse(&self, matrix: &Matrix<'_>) -> Result<Option<Arc<dyn Analysis>>> {
        assert!(self.symmetric || matrix.form() == Form::General);
        Ok(self.cudss.analyse(matrix)?.map(|inner| {
            Arc::new(RecordedAnalysis {
                inner,
                reports: self.reports.clone(),
            }) as Arc<dyn Analysis>
        }))
    }
}

struct RecordedAnalysis {
    inner: Arc<dyn Analysis>,
    reports: Arc<Mutex<Vec<(Form, Report)>>>,
}

impl Analysis for RecordedAnalysis {
    fn form(&self) -> Form {
        self.inner.form()
    }

    fn factorize(&self, matrix: &Matrix<'_>) -> Result<Box<dyn Factorization>> {
        let f = self.inner.factorize(matrix)?;
        let mut reports = self.reports.lock().unwrap();
        reports.push((self.inner.form(), f.report()));
        Ok(f)
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

/// The catalogue's entries solved by cuDSS, as L D Lᵀ where the entry's system is symmetric
/// (`cudss-ldlt`) and as LU (`cudss-lu`), each checked by the entry's own accuracy check, with
/// the factors' entries, cuDSS's estimate of its peak on the GPU, and the analysis and
/// factorization's seconds; photonoxide's own beside them.
fn against_photonoxide(ids: &[&str]) {
    let Some(cudss) = cudss() else { return };
    assert!(offer(cudss.clone()).unwrap());
    let reports = Arc::new(Mutex::new(Vec::new()));
    for (name, symmetric) in [("cudss-ldlt", true), ("cudss-lu", false)] {
        backend::register(Arc::new(Recorded {
            cudss: cudss.clone(),
            name,
            symmetric,
            reports: reports.clone(),
        }))
        .unwrap();
    }
    const MIB: f64 = 1024.0 * 1024.0;
    for id in ids {
        let e = entry(id).unwrap();
        let ours = e.run(&Choice::Photonoxide).unwrap();
        println!(
            "{id} ({}, {} unknowns): photonoxide {:.3} s ({:.1e})",
            ours.grid,
            ours.unknowns,
            factorization(&ours),
            ours.accuracy.as_ref().unwrap().error
        );
        for name in ["cudss-ldlt", "cudss-lu"] {
            reports.lock().unwrap().clear();
            let theirs = match e.run(&Choice::Named(name.into())) {
                Ok(m) => m,
                // too large for this GPU: said, with both numbers
                Err(err) if err.to_string().contains("free") => {
                    println!("  {name}: {err}");
                    continue;
                }
                Err(err) => panic!("{id} with {name}: {err}"),
            };
            let error = theirs.accuracy.as_ref().unwrap().error;
            let (form, report) = reports.lock().unwrap().pop().unwrap();
            println!(
                "  {name} ({form:?}): {:.3} s (analysis {:.3} s, factorization {:.3} s), {} \
                 factor entries, peak {:.0} MiB, error {error:.1e}",
                factorization(&theirs),
                report.analysis_seconds,
                report.factorization_seconds,
                report.factor_entries.unwrap(),
                report.peak_memory_bytes.unwrap() as f64 / MIB,
            );
            assert!(error <= e.tolerance(), "{id} with {name}: {error:e}");
            assert_eq!(report.perturbed_pivots, 0, "{id} with {name}");
            // the catalogue's symmetric systems go to cuDSS as L D Lᵀ, unless it is refused
            let symmetric = name == "cudss-ldlt" && e.task == Task::DirectSymmetric;
            assert_eq!(form == Form::Symmetric, symmetric, "{id} with {name}");
        }
    }
}

/// FDFD's systems from the benchmark's catalogue: the symmetric ones as L D Lᵀ and as LU, the
/// Bloch-periodic one as LU.
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

/// Larger systems, for the factors' memory and time in both forms: minutes, and as much of the
/// GPU as they take.
#[test]
#[ignore = "heavy: run with --ignored"]
fn it_solves_larger_systems_in_both_forms() {
    against_photonoxide(&[
        "fdfd2d/slab-ez-1000",
        "strip-32",
        "fdfd3d/strip-36",
        "strip-40",
        "fdfd3d/strip-48",
    ]);
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
    let own = backend::direct(&Choice::Photonoxide).unwrap();
    for form in [Form::General, Form::Symmetric] {
        let (s, r, v) = system(form);
        let m = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
        let b: Vec<c64> = (0..m.n()).map(|k| c64::new(1.0, k as f64 * 1e-3)).collect();
        let solve = |solver: &dyn DirectSolver, transpose: bool| {
            let analysis = solver.analyse(&m).unwrap().unwrap();
            assert_eq!(analysis.form(), form);
            let f = analysis.factorize(&m).unwrap();
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
        // the transpose is the transpose: a general matrix's differs from its solve, a
        // symmetric matrix is its own (solved again, so within the repeats' difference)
        let f = cudss.analyse(&m).unwrap().unwrap().factorize(&m).unwrap();
        let apart = relative(&f.solve(&b).unwrap(), &f.solve_transpose(&b).unwrap());
        match form {
            Form::General => assert!(apart > 1e-3, "{apart:e}"),
            Form::Symmetric => assert!(apart < 1e-12, "{apart:e}"),
        }
    }
}

#[test]
fn a_singular_matrix_reports_its_replaced_pivot_in_either_form() {
    let Some(cudss) = cudss() else { return };
    let (s, r) = (vec![0, 1, 2], vec![0, 1]);
    let v = [c64::new(1.0, 0.0), c64::new(0.0, 0.0)];
    for form in [Form::General, Form::Symmetric] {
        let m = Matrix::new(2, &s, &r, &v, form).unwrap();
        let factors = cudss.analyse(&m).unwrap().unwrap().factorize(&m).unwrap();
        assert_eq!(factors.report().perturbed_pivots, 1, "{form:?}");
        let (s, r, v) = system(form);
        let m = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
        let factors = cudss.analyse(&m).unwrap().unwrap().factorize(&m).unwrap();
        assert_eq!(factors.report().perturbed_pivots, 0, "{form:?}");
        // a right-hand side of another length is refused
        let b = vec![c64::new(1.0, 0.0); m.n() + 1];
        assert!(factors.solve(&b).is_err() && factors.solve_transpose(&b).is_err());
    }
}

#[test]
fn a_sweep_reuses_its_analysis() {
    let Some(cudss) = cudss() else { return };
    let own = backend::direct(&Choice::Photonoxide).unwrap();
    for form in [Form::General, Form::Symmetric] {
        let (s, r, mut v) = system(form);
        let m = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
        let analysis = cudss.analyse(&m).unwrap().unwrap();
        let b = vec![c64::new(1.0, 0.0); m.n()];
        // two at once: the second takes an engine of its own, analysed anew
        let held = analysis.factorize(&m).unwrap();
        for step in 0..3 {
            v[0] += c64::new(0.1, 0.0);
            let m = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
            let f = analysis.factorize(&m).unwrap();
            let report = f.report();
            // L D Lᵀ keeps one triangle
            let stored = match form {
                Form::General => v.len(),
                Form::Symmetric => v.len().div_ceil(2),
            };
            assert!(report.factor_entries.unwrap() >= stored, "{form:?} {step}");
            assert!(report.peak_memory_bytes.unwrap() > 0);
            let x = f.solve(&b).unwrap();
            let ours = own.analyse(&m).unwrap().unwrap().factorize(&m).unwrap();
            let difference = relative(&x, &ours.solve(&b).unwrap());
            assert!(difference < 1e-10, "{form:?} {step}: {difference:e}");
        }
        assert!(held.solve(&b).unwrap().iter().all(|x| x.is_finite()));
    }
}
