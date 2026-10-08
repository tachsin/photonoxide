//! oneMKL's PARDISO as a [`DirectSolver`] (#175): O. Schenk, K. Gärtner, "Solving unsymmetric
//! sparse systems of linear equations with PARDISO", Future Gener. Comput. Syst. 20, 475
//! (2004), doi:10.1016/j.future.2003.07.011.
//!
//! - **The forms.** [`Form::General`] is PARDISO's complex unsymmetric matrix (`mtype` 13), by
//!   rows: the columns are turned into rows once per structure. [`Form::Symmetric`] is its
//!   complex symmetric matrix (`mtype` 6, Bunch–Kaufman pivots inside its supernodes), given
//!   its upper triangle by rows: a symmetric matrix's columns are its rows, so each column's
//!   entries on and below the diagonal are the row's on and right of it. PARDISO wants every
//!   diagonal entry of a symmetric matrix in the structure; one missing is given as a zero.
//! - **Settings:** METIS's nested dissection (`iparm[1]` = 2); zero-based indices
//!   (`iparm[34]` = 1); no iterative refinement unless pivots were perturbed (`iparm[7]` = 0:
//!   then two steps); PARDISO's defaults otherwise, as docs/baselines.md ran it: scaling and
//!   weighted matching for `mtype` 13, neither for `mtype` 6; pivots perturbed below 10⁻¹³
//!   (`mtype` 13) or 10⁻⁸ (`mtype` 6) of the largest, and counted (`iparm[13]`).
//! - **Integers** are 64-bit: `pardiso_64`, whatever oneMKL's interface layer (`MKL_INT`), so a
//!   matrix's or its factors' entries may pass 2³¹.
//! - **Phases:** analysis (11) once per structure. Each factorization (22) takes an analysed
//!   PARDISO handle from a pool, and gives it back when dropped, so a sweep analyses once. The
//!   solve (33) with the transpose is `iparm[11]` = 2; the symmetric form is its own. The
//!   handle's memory is released (−1) when it is dropped.
//! - **Report:** the factors' entries (`iparm[17]`), the peak memory, the larger of the
//!   analysis's (`iparm[14]`) and the factorization's and solve's with the analysis's kept
//!   (`iparm[15]` + `iparm[16]`), in kB, and the perturbed pivots (`iparm[13]`).
//! - **Threads:** oneMKL's own, through the threading layer [`crate::intel`] chose; each call
//!   sets them on its thread (`mkl_set_num_threads_local`) to rayon's count where it is called,
//!   photonoxide's setting (`RAYON_NUM_THREADS`), and restores them after (the TBB layer
//!   ignores it and runs on all the processors; see [`crate::intel`]). The factorization
//!   is PARDISO's classic parallel one (`iparm[23]` = 0): its two-level one (1) didn't finish a
//!   complex symmetric 32³ Helmholtz matrix in minutes on 20 threads (oneMKL 2026.1, Windows,
//!   Intel's OpenMP), and was no faster as LU.
//! - **Determinism:** declared when oneMKL's conditional numerical reproducibility is on
//!   (`mkl_cbwr_set(MKL_CBWR_AUTO)`, set by [`crate::intel`]), with PARDISO's own laid out for
//!   a fixed number of threads (`iparm[33]`). Measured with oneMKL 2026.1 on Windows (Intel's
//!   OpenMP, 20 logical processors): the same bits on every run and on 1, 2, 3, 4, 8 and 20
//!   threads, for both forms of a 32³ Helmholtz matrix, at the same speed as without either
//!   setting (which gave the same bits there too); the same on the TBB and sequential layers.
//!   The tests check it wherever they run.
//! - **Errors:** PARDISO's `error` codes, with Intel's meanings.

use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::Result;
use photonoxide::backend::{
    Analysis, Capabilities, DirectSolver, Factorization, Form, Matrix, Report, Threads,
};

use crate::intel::{Mkl, mkl};
use crate::library::error;

/// `void pardiso_64(void *pt, const long long *maxfct, const long long *mnum,
/// const long long *mtype, const long long *phase, const long long *n, const void *a,
/// const long long *ia, const long long *ja, long long *perm, const long long *nrhs,
/// long long *iparm, const long long *msglvl, void *b, void *x, long long *error)`
type Pardiso64 = unsafe extern "C" fn(
    *mut c_void,
    *const i64,
    *const i64,
    *const i64,
    *const i64,
    *const i64,
    *const c_void,
    *const i64,
    *const i64,
    *mut i64,
    *const i64,
    *mut i64,
    *const i64,
    *mut c_void,
    *mut c_void,
    *mut i64,
);

/// The threads PARDISO's reproducible mode is laid out for (`iparm[33]`), the same whatever
/// the threads a call runs on: this machine's logical processors, and at least 64.
fn cnr_threads() -> i64 {
    let here = std::thread::available_parallelism().map_or(1, |n| n.get());
    i64::try_from(here).unwrap_or(i64::MAX).max(64)
}

const COMPLEX_UNSYMMETRIC: i64 = 13;
const COMPLEX_SYMMETRIC: i64 = 6;
const ANALYSIS: i64 = 11;
const FACTORIZATION: i64 = 22;
const SOLVE: i64 = 33;
const RELEASE: i64 = -1;
/// An entry of the rows that isn't in the matrix given: a symmetric matrix's missing diagonal.
const ZERO: usize = usize::MAX;

/// Intel's meaning of PARDISO's `error`.
fn meaning(code: i64) -> &'static str {
    match code {
        -1 => "input inconsistent",
        -2 => "not enough memory",
        -3 => "reordering problem",
        -4 => "zero pivot, numerical factorization or iterative refinement problem",
        -5 => "unclassified (internal) error",
        -6 => "reordering failed (matrix types 11 and 13 only)",
        -7 => "diagonal matrix is singular",
        -8 => "32-bit integer overflow problem",
        -9 => "not enough memory for OOC",
        -10 => "error opening OOC files",
        -11 => "read/write error with OOC files",
        -12 => "pardiso_64 called from a 32-bit library",
        -13 => "interrupted by the mkl_progress function",
        -15 => "internal error (iparm[23] = 10 with matching, iparm[12] = 1)",
        _ => "an error Intel's documentation doesn't list",
    }
}

/// PARDISO and the oneMKL it is part of.
struct Api {
    pardiso: Pardiso64,
    // last: the function above must not outlive it
    mkl: Arc<Mkl>,
}

/// A matrix's rows as PARDISO takes them, zero-based, 64-bit.
struct Rows {
    n: i64,
    starts: Vec<i64>,
    columns: Vec<i64>,
    /// Each entry by rows, its place in the matrix's values, or [`ZERO`].
    order: Vec<usize>,
    /// The matrix's entries as given (both triangles of a symmetric one).
    entries: usize,
}

impl Rows {
    /// A general matrix's rows: its columns transposed.
    fn general(cs: &[usize], ri: &[usize]) -> Rows {
        let n = cs.len() - 1;
        let mut starts = vec![0usize; n + 1];
        for &r in ri {
            starts[r + 1] += 1;
        }
        for i in 0..n {
            starts[i + 1] += starts[i];
        }
        let mut next = starts.clone();
        let mut columns = vec![0i64; ri.len()];
        let mut order = vec![0; ri.len()];
        for j in 0..n {
            for k in cs[j]..cs[j + 1] {
                let p = next[ri[k]];
                columns[p] = j as i64;
                order[p] = k;
                next[ri[k]] += 1;
            }
        }
        Rows {
            n: n as i64,
            starts: starts.iter().map(|&s| s as i64).collect(),
            columns,
            order,
            entries: ri.len(),
        }
    }

    /// A symmetric matrix's upper triangle by rows: column j's entries from row j down are row
    /// j's from column j on, its diagonal added as a zero if missing.
    fn upper(cs: &[usize], ri: &[usize]) -> Rows {
        let n = cs.len() - 1;
        let mut starts = Vec::with_capacity(n + 1);
        let (mut columns, mut order) = (Vec::new(), Vec::new());
        starts.push(0i64);
        for j in 0..n {
            let below = (cs[j]..cs[j + 1]).filter(|&k| ri[k] >= j);
            let mut first = true;
            for k in below {
                if first && ri[k] != j {
                    columns.push(j as i64);
                    order.push(ZERO);
                }
                first = false;
                columns.push(ri[k] as i64);
                order.push(k);
            }
            if first {
                columns.push(j as i64);
                order.push(ZERO);
            }
            starts.push(columns.len() as i64);
        }
        Rows {
            n: n as i64,
            starts,
            columns,
            order,
            entries: ri.len(),
        }
    }

    /// A matrix's values in these rows' order.
    fn values(&self, values: &[c64]) -> Vec<c64> {
        self.order
            .iter()
            .map(|&k| {
                if k == ZERO {
                    c64::new(0.0, 0.0)
                } else {
                    values[k]
                }
            })
            .collect()
    }
}

/// One PARDISO handle, analysed for one structure: its internal pointers, its parameters and
/// the structure it holds on to.
struct Engine {
    api: Arc<Api>,
    rows: Arc<Rows>,
    mtype: i64,
    /// PARDISO's 64 internal pointers, zero before its first call; boxed so they don't move.
    pt: Box<[*mut c_void; 64]>,
    iparm: [i64; 64],
}

// SAFETY: a handle is used by one thread at a time (behind a Mutex, or owned), and PARDISO's
// handles are independent of the thread that made them
unsafe impl Send for Engine {}

impl Engine {
    /// A handle for this structure, analysed with these values (which PARDISO's matching and
    /// scaling read), in the seconds returned.
    fn new(api: &Arc<Api>, rows: &Arc<Rows>, form: Form, values: &[c64]) -> Result<(Engine, f64)> {
        let mut iparm = [0i64; 64];
        let symmetric = form == Form::Symmetric;
        iparm[0] = 1; // these settings, not PARDISO's defaults
        iparm[1] = 2; // METIS's nested dissection
        iparm[7] = 0; // iterative refinement only after perturbed pivots, two steps
        iparm[9] = if symmetric { 8 } else { 13 }; // pivots perturbed below 10^-iparm[9]
        iparm[10] = if symmetric { 0 } else { 1 }; // scaling
        iparm[12] = if symmetric { 0 } else { 1 }; // weighted matching
        iparm[17] = -1; // report the factors' entries
        iparm[20] = 1; // Bunch–Kaufman pivots for mtype 6
        iparm[23] = 0; // the classic parallel factorization (see the module's docs)
        iparm[33] = cnr_threads(); // the reproducible mode's layout
        iparm[34] = 1; // zero-based indices
        let mut e = Engine {
            api: api.clone(),
            rows: rows.clone(),
            mtype: if symmetric {
                COMPLEX_SYMMETRIC
            } else {
                COMPLEX_UNSYMMETRIC
            },
            pt: Box::new([std::ptr::null_mut(); 64]),
            iparm,
        };
        let start = Instant::now();
        e.call(ANALYSIS, values, None, "analysis")?;
        Ok((e, start.elapsed().as_secs_f64()))
    }

    /// One phase of PARDISO's on `a` (in the rows' order), with a right-hand side `b` for the
    /// solve, which returns x.
    fn call(&mut self, phase: i64, a: &[c64], b: Option<&[c64]>, what: &str) -> Result<Vec<c64>> {
        let n = self.rows.n as usize;
        assert_eq!(a.len(), self.rows.columns.len());
        let mut b = b.map(<[c64]>::to_vec);
        if let Some(b) = &b
            && b.len() != n
        {
            return Err(error(format!(
                "a right-hand side of {} for {n} unknowns",
                b.len()
            )));
        }
        let mut x = vec![c64::new(0.0, 0.0); if b.is_some() { n } else { 0 }];
        let (maxfct, mnum, nrhs, msglvl) = (1i64, 1i64, 1i64, 0i64);
        let mut perm = 0i64;
        let mut code = 0i64;
        let mut dummy = c64::new(0.0, 0.0);
        let (b_ptr, x_ptr): (*mut c_void, *mut c_void) = match &mut b {
            Some(b) => (b.as_mut_ptr().cast(), x.as_mut_ptr().cast()),
            None => ((&raw mut dummy).cast(), (&raw mut dummy).cast()),
        };
        let previous = self.api.mkl.set_threads_here(rayon::current_num_threads());
        // SAFETY: pt is PARDISO's 64 pointers, zero before the first call and PARDISO's after;
        // the integers are 64-bit as pardiso_64 takes them; a holds the rows' nnz complex
        // doubles and ia, ja their n + 1 starts and nnz columns, zero-based as iparm[34] says,
        // unchanged between the phases (the rows are shared and immutable); perm isn't read
        // (iparm[4] = 0); b and x hold n complex doubles for the solve (nrhs = 1), and point
        // at a dummy otherwise, which the other phases don't read
        unsafe {
            (self.api.pardiso)(
                self.pt.as_mut_ptr().cast(),
                &maxfct,
                &mnum,
                &self.mtype,
                &phase,
                &self.rows.n,
                a.as_ptr().cast(),
                self.rows.starts.as_ptr(),
                self.rows.columns.as_ptr(),
                &mut perm,
                &nrhs,
                self.iparm.as_mut_ptr(),
                &msglvl,
                b_ptr,
                x_ptr,
                &mut code,
            );
        }
        self.api.mkl.set_threads_here(previous.max(0) as usize);
        if code != 0 {
            return Err(error(format!(
                "PARDISO's {what} failed with error {code}: {}",
                meaning(code)
            )));
        }
        Ok(x)
    }

    fn report(&self) -> (Option<usize>, Option<u64>, usize) {
        let p = &self.iparm;
        let entries = (p[17] >= 0).then_some(p[17] as usize);
        let kb = p[14].max(p[15] + p[16]);
        let peak = (kb > 0).then_some(kb as u64 * 1024);
        (entries, peak, p[13].max(0) as usize)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if self.pt.iter().all(|p| p.is_null()) {
            return;
        }
        // the release reads no values
        let a = vec![c64::new(0.0, 0.0); self.rows.columns.len()];
        let _ = self.call(RELEASE, &a, None, "release");
    }
}

/// oneMKL's PARDISO, as photonoxide's registry has it.
pub struct Pardiso {
    api: Arc<Api>,
}

impl Pardiso {
    /// oneMKL found, loaded and set up ([`crate::intel::mkl`]), and PARDISO in it.
    ///
    /// # Errors
    ///
    /// Why not: oneMKL not found, no threading layer to be had, PARDISO missing.
    pub fn load() -> std::result::Result<Pardiso, String> {
        let (_, mkl) = mkl();
        let mkl = mkl?;
        // SAFETY: the type is pardiso_64's declaration in mkl_pardiso.h
        let pardiso = unsafe { mkl.library().function::<Pardiso64>("pardiso_64") }
            .map_err(|e| e.to_string())?;
        Ok(Pardiso {
            api: Arc::new(Api { pardiso, mkl }),
        })
    }

    /// The oneMKL it is part of.
    pub fn mkl(&self) -> &Mkl {
        &self.api.mkl
    }
}

impl DirectSolver for Pardiso {
    fn capabilities(&self) -> Capabilities {
        let mkl = &self.api.mkl;
        let mut c = Capabilities::new(
            "pardiso",
            mkl.version.clone(),
            "Intel Simplified Software License (installed by the user)",
        );
        c.symmetric = true;
        c.transpose = true;
        c.threads = Threads::Library(mkl.threads());
        c.deterministic = mkl.reproducibility != 0;
        c
    }

    fn analyse(&self, matrix: &Matrix<'_>) -> Result<Option<Arc<dyn Analysis>>> {
        let (cs, ri) = (matrix.column_starts(), matrix.row_indices());
        let rows = Arc::new(match matrix.form() {
            Form::General => Rows::general(cs, ri),
            Form::Symmetric => Rows::upper(cs, ri),
        });
        let values = rows.values(matrix.values());
        let (engine, seconds) = Engine::new(&self.api, &rows, matrix.form(), &values)?;
        Ok(Some(Arc::new(Analysed {
            api: self.api.clone(),
            form: matrix.form(),
            rows,
            pool: Arc::new(Mutex::new(vec![engine])),
            seconds,
        })))
    }
}

struct Analysed {
    api: Arc<Api>,
    form: Form,
    rows: Arc<Rows>,
    pool: Arc<Mutex<Vec<Engine>>>,
    seconds: f64,
}

fn poisoned<T>(_: T) -> photonoxide::Error {
    error("a PARDISO handle's lock was poisoned by a panic")
}

impl Analysis for Analysed {
    fn form(&self) -> Form {
        self.form
    }

    fn factorize(&self, matrix: &Matrix<'_>) -> Result<Box<dyn Factorization>> {
        if matrix.values().len() != self.rows.entries || matrix.n() as i64 != self.rows.n {
            return Err(error("a matrix of another structure than the one analysed"));
        }
        let values = self.rows.values(matrix.values());
        let pooled = self.pool.lock().map_err(poisoned)?.pop();
        let (mut engine, analysis_seconds) = match pooled {
            Some(engine) => (engine, self.seconds),
            None => Engine::new(&self.api, &self.rows, self.form, &values)?,
        };
        let start = Instant::now();
        // a failed factorization's handle goes back analysed: the next matrix factorizes anew
        let factorized = engine.call(FACTORIZATION, &values, None, "factorization");
        let factorization_seconds = start.elapsed().as_secs_f64();
        if let Err(e) = factorized {
            self.pool.lock().map_err(poisoned)?.push(engine);
            return Err(e);
        }
        let (factor_entries, peak, perturbed_pivots) = engine.report();
        let mut report = Report::default();
        report.factor_entries = factor_entries;
        report.peak_memory_bytes = peak;
        report.analysis_seconds = analysis_seconds;
        report.factorization_seconds = factorization_seconds;
        report.perturbed_pivots = perturbed_pivots;
        Ok(Box::new(Factors {
            engine: Mutex::new(Some(engine)),
            pool: self.pool.clone(),
            values,
            symmetric: self.form == Form::Symmetric,
            report,
        }))
    }
}

struct Factors {
    engine: Mutex<Option<Engine>>,
    pool: Arc<Mutex<Vec<Engine>>>,
    /// The values factorized, in the rows' order: PARDISO's iterative refinement reads them.
    values: Vec<c64>,
    symmetric: bool,
    report: Report,
}

impl Factors {
    fn solve_with(&self, b: &[c64], transpose: bool) -> Result<Vec<c64>> {
        let mut engine = self.engine.lock().map_err(poisoned)?;
        let engine = engine.as_mut().expect("held until dropped");
        // 2: Aᵀ x = b, not conjugated; a symmetric matrix is its own transpose
        engine.iparm[11] = if transpose && !self.symmetric { 2 } else { 0 };
        let x = engine.call(SOLVE, &self.values, Some(b), "solve");
        engine.iparm[11] = 0;
        x
    }
}

impl Factorization for Factors {
    fn solve(&self, b: &[c64]) -> Result<Vec<c64>> {
        self.solve_with(b, false)
    }

    fn solve_transpose(&self, b: &[c64]) -> Result<Vec<c64>> {
        self.solve_with(b, true)
    }

    fn report(&self) -> Report {
        self.report.clone()
    }
}

impl Drop for Factors {
    fn drop(&mut self) {
        if let (Ok(mut engine), Ok(mut pool)) = (self.engine.lock(), self.pool.lock())
            && let Some(engine) = engine.take()
        {
            pool.push(engine);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3 × 3 matrix by columns, its diagonal's middle entry missing.
    fn columns() -> (Vec<usize>, Vec<usize>) {
        // [a . b]
        // [. . c]
        // [b c d]
        (vec![0, 2, 3, 6], vec![0, 2, 2, 0, 1, 2])
    }

    #[test]
    fn the_rows_of_a_general_matrix_are_its_columns_transposed() {
        let (cs, ri) = columns();
        let rows = Rows::general(&cs, &ri);
        assert_eq!(rows.starts, [0, 2, 3, 6]);
        assert_eq!(rows.columns, [0, 2, 2, 0, 1, 2]);
        assert_eq!(rows.order, [0, 3, 4, 1, 2, 5]);
    }

    #[test]
    fn a_symmetric_matrix_gives_its_upper_triangle_with_its_whole_diagonal() {
        let (cs, ri) = columns();
        let rows = Rows::upper(&cs, &ri);
        // row 0: (0, 0), (0, 2); row 1: (1, 1) added, (1, 2); row 2: (2, 2)
        assert_eq!(rows.starts, [0, 2, 4, 5]);
        assert_eq!(rows.columns, [0, 2, 1, 2, 2]);
        assert_eq!(rows.order, [0, 1, ZERO, 2, 5]);
        let v: Vec<c64> = (1..=6).map(|k| c64::new(k as f64, 0.0)).collect();
        let values = rows.values(&v);
        assert_eq!(values[2], c64::new(0.0, 0.0));
        assert_eq!(values[3], c64::new(3.0, 0.0));
    }
}
