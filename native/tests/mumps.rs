//! MUMPS against photonoxide's own solver, on this machine. Skipped, and saying so, where
//! MUMPS isn't found; with `PHOTONOXIDE_REQUIRE_MUMPS` set (the Libraries workflow, after it
//! installs MUMPS) not finding it is a failure.

use std::sync::Arc;

use num_complex::Complex64 as c64;
use photonoxide::backend::{self, Choice, DirectSolver, Form, Matrix};
use photonoxide::bench::catalogue::entry;
use photonoxide_native::{Mumps, mumps, offer, smoke_test};

fn mumps() -> Option<Arc<Mumps>> {
    match Mumps::load() {
        Ok(m) => Some(Arc::new(m)),
        Err(reason) => {
            assert!(
                std::env::var_os("PHOTONOXIDE_REQUIRE_MUMPS").is_none(),
                "MUMPS is required here and isn't found: {reason}"
            );
            println!("skipped: MUMPS isn't here: {reason}");
            None
        }
    }
}

#[test]
fn it_finds_mumps_and_reads_its_release() {
    let probe = mumps::probe();
    println!("{probe}");
    let Some(mumps) = mumps() else { return };
    let c = mumps.capabilities();
    println!("{c:?}");
    assert_eq!(probe.version.as_deref(), Some(mumps.version()));
    assert_eq!(c.name, "mumps");
    assert_eq!(c.version, mumps.version());
    assert!(c.version.starts_with("5."));
    assert!(c.complex && c.symmetric && c.transpose && !c.deterministic);
    assert!(c.licence.contains("CeCILL-C") && c.licence.contains("10.1137/S0895479899358194"));
}

#[test]
fn it_passes_the_smoke_test() {
    let Some(mumps) = mumps() else { return };
    let difference = smoke_test(mumps.as_ref()).unwrap();
    println!("largest difference from photonoxide's: {difference:.1e}");
    assert!(difference < 1e-12);
}

/// A Helmholtz operator with loss on a grid of these cells, 2D or 3D: complex symmetric, or
/// general with an asymmetric coupling along x.
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

fn system(m: usize, form: Form) -> (Vec<usize>, Vec<usize>, Vec<c64>) {
    helmholtz([m, m, 1], form)
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

#[test]
fn it_solves_both_forms_and_their_transposes_as_photonoxide_does() {
    let Some(mumps) = mumps() else { return };
    let own = backend::direct(&Choice::Photonoxide).unwrap();
    for cells in [[60, 60, 1], [14, 12, 10]] {
        for form in [Form::General, Form::Symmetric] {
            let (s, r, v) = helmholtz(cells, form);
            let m = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
            let b = rhs(m.n());
            for transpose in [false, true] {
                let theirs = solve(mumps.as_ref(), &m, &b, transpose);
                let ours = solve(own.as_ref(), &m, &b, transpose);
                let difference = relative(&theirs, &ours);
                println!(
                    "{cells:?} {form:?}, transpose {transpose}: {difference:.1e} from photonoxide's"
                );
                assert!(
                    difference < 1e-10,
                    "{cells:?} {form:?} {transpose}: {difference:e}"
                );
            }
            // the transpose is the transpose: a general matrix's differs from its solve
            let (x, xt) = (
                solve(mumps.as_ref(), &m, &b, false),
                solve(mumps.as_ref(), &m, &b, true),
            );
            let apart = relative(&x, &xt);
            match form {
                Form::General => assert!(apart > 1e-3, "{apart:e}"),
                _ => assert!(apart < 1e-12, "{apart:e}"),
            }
        }
    }
}

#[test]
fn a_singular_matrix_is_an_error_and_a_wrong_right_hand_side_is_refused() {
    let Some(mumps) = mumps() else { return };
    // a zero on the diagonal of a diagonal matrix
    let (s, r) = (vec![0, 1, 2], vec![0, 1]);
    let v = [c64::new(1.0, 0.0), c64::new(0.0, 0.0)];
    for form in [Form::General, Form::Symmetric] {
        let m = Matrix::new(2, &s, &r, &v, form).unwrap();
        let outcome = mumps
            .analyse(&m)
            .and_then(|a| a.expect("an analysis or an error").factorize(&m));
        match outcome {
            Err(e) => {
                let e = e.to_string();
                println!("{form:?}: {e}");
                assert!(e.contains("INFOG(1)"), "{e}");
            }
            Ok(factors) => {
                // or it says it modified a pivot
                let perturbed = factors.report().perturbed_pivots;
                println!("{form:?}: {perturbed} pivot modified");
                assert!(perturbed >= 1);
            }
        }
    }
    let (s, r, v) = system(30, Form::General);
    let m = Matrix::new(s.len() - 1, &s, &r, &v, Form::General).unwrap();
    let factors = mumps.analyse(&m).unwrap().unwrap().factorize(&m).unwrap();
    assert_eq!(factors.report().perturbed_pivots, 0);
    for len in [0, m.n() - 1, m.n() + 1] {
        let b = vec![c64::new(1.0, 0.0); len];
        assert!(factors.solve(&b).is_err() && factors.solve_transpose(&b).is_err());
    }
}

#[test]
fn a_sweep_reuses_its_analysis_and_reports_its_factors() {
    let Some(mumps) = mumps() else { return };
    let own = backend::direct(&Choice::Photonoxide).unwrap();
    for form in [Form::General, Form::Symmetric] {
        let (s, r, mut v) = system(40, form);
        let m = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
        let analysis = mumps.analyse(&m).unwrap().unwrap();
        assert_eq!(analysis.form(), form);
        let b = rhs(m.n());
        // two at once: the second takes an instance of its own, analysed anew
        let held = analysis.factorize(&m).unwrap();
        let held_solution = held.solve(&b).unwrap();
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
            assert!(entries >= stored, "{step}: {entries} for {stored}");
            assert!(report.analysis_seconds > 0.0 && report.factorization_seconds > 0.0);
            let x = f.solve(&b).unwrap();
            let ours = solve(own.as_ref(), &m, &b, false);
            assert!(relative(&x, &ours) < 1e-10, "{form:?} {step}");
            println!(
                "{form:?} step {step}: {entries} factor entries, {:?} bytes",
                report.peak_memory_bytes
            );
        }
        // the factors held meanwhile are still the first matrix's
        assert_eq!(held.solve(&b).unwrap(), held_solution);
        // another structure is refused
        let (s, r, v) = system(41, form);
        let other = Matrix::new(s.len() - 1, &s, &r, &v, form).unwrap();
        assert!(analysis.factorize(&other).is_err());
    }
}

#[test]
fn factorizations_on_several_threads_at_once_agree() {
    let Some(mumps) = mumps() else { return };
    let (s, r, v) = system(80, Form::General);
    let m = Matrix::new(s.len() - 1, &s, &r, &v, Form::General).unwrap();
    let b = rhs(m.n());
    let analysis = mumps.analyse(&m).unwrap().unwrap();
    let alone = analysis.factorize(&m).unwrap().solve(&b).unwrap();
    // MUMPS itself runs one call at a time; the instances are each thread's own
    let solutions: Vec<Vec<c64>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| scope.spawn(|| analysis.factorize(&m).unwrap().solve(&b).unwrap()))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for x in &solutions {
        assert!(relative(x, &alone) < 1e-11, "{:e}", relative(x, &alone));
    }
}

/// FDFD's systems from the benchmark's catalogue, each checked by its own accuracy check, as
/// L D Lᵀ where the system is symmetric and as LU with a Bloch side.
#[test]
fn it_solves_fdfd_systems_to_their_checks() {
    let Some(mumps) = mumps() else { return };
    assert!(offer(mumps).unwrap());
    for id in [
        "fdfd2d/slab-ez-100",
        "fdfd2d/ring-hz-100",
        "fdfd2d/slab-ez-bloch-100",
        "fdfd3d/strip-16",
        "fdfd3d/grating-16",
        "fdfd3d/strip-stretched-16",
    ] {
        let e = entry(id).unwrap();
        let ours = e.run(&Choice::Photonoxide).unwrap();
        let theirs = e.run(&Choice::Named("mumps".into())).unwrap();
        let (a, b) = (
            ours.accuracy.as_ref().unwrap().error,
            theirs.accuracy.as_ref().unwrap().error,
        );
        println!(
            "{id} ({} unknowns): photonoxide {a:.1e} in {:.3} s, mumps {b:.1e} in {:.3} s",
            ours.unknowns,
            ours.seconds(),
            theirs.seconds()
        );
        assert!(b <= e.tolerance(), "{id}: {b:e}");
    }
}
