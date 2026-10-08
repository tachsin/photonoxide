//! photonoxide's GMRES preconditioned by its multigrid cycle, on an NVIDIA GPU (#190): the run
//! of `fdfd::krylov::gmres_preconditioned`, step for step, on cuSPARSE, with the cycle
//! photonoxide builds ([`MultigridCycle`]) run on the GPU too.
//!
//! - **GMRES:** Saad's Algorithm 9.5 restarted as 6.11, as photonoxide's own: the Arnoldi basis
//!   kept on the GPU, orthogonalized by modified Gram–Schmidt (`cusparseSpVV`, then
//!   `cusparseAxpby`), the Hessenberg matrix's columns, the Givens rotations and the triangular
//!   solve for y on the host, where only the scalars come.
//! - **The cycle,** on every level: the residual by the level's operator (`cusparseSpMV`), the
//!   smoother's two triangular solves with photonoxide's ILU(0) factors of the level's slabs
//!   (`cusparseSpSV`: one pair of block-diagonal factors, whose independent blocks its analysis
//!   finds), and the restriction Pᵀ and prolongation P as rectangular products.
//! - **The coarsest solve:** on the CPU, by photonoxide's factorization of the coarsest level
//!   (at most 2 000 unknowns by default, 32 kB): its right-hand side comes back from the GPU and
//!   its solution goes up again. Measured on the 68³ guide of `fdfd3d-iterative/guide-multigrid-68`
//!   (943 k unknowns, the coarsest level 5³ cells, 375 unknowns), the 23 coarsest solves of a
//!   1.6 s solve took 5 ms with their copies, so neither cuDSS nor a dense inverse on the GPU
//!   has anything to win; and they keep the bits of photonoxide's own coarsest solve.
//! - **What it costs to set up,** each solve: the hierarchy copied to the GPU and the smoothers'
//!   triangular factors analysed, 0.4 s of that 1.6 s.
//! - **Determinism:** cuSPARSE's sums, not photonoxide's chunked ones, so the iterations agree
//!   with the CPU's to rounding, and a repeated run on one GPU gives the same bits, which the
//!   tests check.

use std::sync::Arc;

use num_complex::Complex64 as c64;
use photonoxide::Result;
use photonoxide::backend::{IluFactors, MultigridCycle, RowMatrix};
use photonoxide::fdfd::{Convergence, CycleShape, Stopping};

use crate::cuda::serial;
use crate::cudss::{Rows, index};
use crate::gpu_qmr::{Api, DeviceMatrix, DeviceVector, ONE, Session, Triangular, ZERO};
use crate::library::error;

/// 32-bit indices, as cuSPARSE takes them here, on rayon's threads.
fn indices(values: &[usize]) -> Result<Vec<i32>> {
    use rayon::prelude::*;
    if let Some(&largest) = values.iter().max() {
        index(largest)?;
    }
    Ok(values
        .par_iter()
        .with_min_len(1 << 16)
        .map(|&v| v as i32)
        .collect())
}

/// A matrix by rows on the GPU, each row's columns ascending (a transfer's prolongation comes
/// in the order photonoxide sums it in, and is sorted here).
fn upload(api: &Arc<Api>, m: RowMatrix<'_>) -> Result<DeviceMatrix> {
    use rayon::prelude::*;
    let (starts, columns, values) = (m.starts(), m.indices(), m.values());
    index(values.len())?;
    index(m.columns())?;
    let ascending = (0..m.rows()).into_par_iter().all(|r| {
        columns[starts[r]..starts[r + 1]]
            .windows(2)
            .all(|p| p[0] < p[1])
    });
    let rows = Rows {
        starts: indices(starts)?,
        columns: indices(columns)?,
    };
    if ascending {
        return DeviceMatrix::with_columns(api, &rows, m.columns(), values);
    }
    let mut rows = rows;
    let mut sorted = values.to_vec();
    let mut row = Vec::new();
    for r in 0..m.rows() {
        let range = starts[r]..starts[r + 1];
        row.clear();
        row.extend(range.clone().map(|k| (rows.columns[k], values[k])));
        row.sort_by_key(|e| e.0);
        for (k, &(c, v)) in range.zip(&row) {
            rows.columns[k] = c;
            sorted[k] = v;
        }
    }
    DeviceMatrix::with_columns(api, &rows, m.columns(), &sorted)
}

/// A factor's rows, for [`Triangular`].
fn factor((starts, columns, _): (&[usize], &[usize], &[c64])) -> Result<Rows> {
    Ok(Rows {
        starts: indices(starts)?,
        columns: indices(columns)?,
    })
}

/// One level above the coarsest on the GPU: its operator, smoother and transfers, and its
/// vectors: x, the cycle's result here; r and t, work; `input`, the right-hand side the level
/// above restricts to it (unused on the finest, whose comes from GMRES).
struct Level {
    a: DeviceMatrix,
    l: Triangular,
    u: Triangular,
    restriction: DeviceMatrix,
    prolongation: DeviceMatrix,
    input: DeviceVector,
    x: DeviceVector,
    r: DeviceVector,
    t: DeviceVector,
}

impl Level {
    fn new(
        s: &mut Session,
        a: RowMatrix<'_>,
        smoother: &IluFactors,
        restriction: RowMatrix<'_>,
        prolongation: RowMatrix<'_>,
    ) -> Result<Level> {
        let api = &s.api;
        let n = a.rows();
        let zeros = vec![ZERO; n];
        let (input, x, r, t) = (
            s.vector(&zeros)?,
            s.vector(&zeros)?,
            s.vector(&zeros)?,
            s.vector(&zeros)?,
        );
        let (lower, upper) = (factor(smoother.lower())?, factor(smoother.upper())?);
        let (lower_values, upper_values) = (smoother.lower().2, smoother.upper().2);
        let level = Level {
            a: upload(api, a)?,
            l: Triangular::new(s, (&lower, lower_values), true, true, &r, &t)?,
            u: Triangular::new(s, (&upper, upper_values), false, false, &t, &r)?,
            restriction: upload(api, restriction)?,
            prolongation: upload(api, prolongation)?,
            input,
            x,
            r,
            t,
        };
        s.prepare(&level.a, &level.x, &level.r)?;
        Ok(level)
    }
}

/// The cycle on the GPU: the levels above the coarsest, finest first, and the coarsest's
/// right-hand side and solution.
struct Cycle<'c, 'a> {
    levels: Vec<Level>,
    coarse_input: DeviceVector,
    coarse_x: DeviceVector,
    cycle: &'c MultigridCycle<'a>,
}

impl<'c, 'a> Cycle<'c, 'a> {
    fn new(s: &mut Session, cycle: &'c MultigridCycle<'a>) -> Result<Cycle<'c, 'a>> {
        let mut levels = Vec::with_capacity(cycle.levels().len());
        for l in cycle.levels() {
            levels.push(Level::new(
                s,
                l.operator(),
                l.smoother(),
                l.restriction(),
                l.prolongation(),
            )?);
        }
        let zeros = vec![ZERO; cycle.coarsest().rows()];
        let (coarse_input, coarse_x) = (s.vector(&zeros)?, s.vector(&zeros)?);
        // the transfers' workspace, each on vectors of its shape
        for (k, level) in levels.iter().enumerate() {
            let (next_input, next_x) = match levels.get(k + 1) {
                Some(next) => (&next.input, &next.x),
                None => (&coarse_input, &coarse_x),
            };
            s.prepare(&level.restriction, &level.r, next_input)?;
            s.prepare(&level.prolongation, next_x, &level.t)?;
        }
        Ok(Cycle {
            levels,
            coarse_input,
            coarse_x,
            cycle,
        })
    }

    /// The finest level's result, M⁻¹ v.
    fn result(&self) -> &DeviceVector {
        self.levels.first().map_or(&self.coarse_x, |l| &l.x)
    }

    /// M⁻¹ `b`, into [`Cycle::result`].
    fn apply(&self, s: &Session, b: &DeviceVector) -> Result<()> {
        self.run(s, 0, b, self.cycle.shape())
    }

    /// One cycle from `level` for `b`, from x = 0, into the level's x.
    fn run(&self, s: &Session, level: usize, b: &DeviceVector, shape: CycleShape) -> Result<()> {
        let Some(l) = self.levels.get(level) else {
            // a failed solve gives NaNs, as photonoxide's own cycle, which GMRES then reports
            // as a solve that didn't converge
            let rhs = b.read()?;
            let x = self
                .cycle
                .coarsest_solve(&rhs[..self.cycle.coarsest().rows()])
                .unwrap_or_else(|_| vec![c64::new(f64::NAN, f64::NAN); rhs.len()]);
            return self.coarse_x.write(&x);
        };
        let (next_input, next_x) = match self.levels.get(level + 1) {
            Some(next) => (&next.input, &next.x),
            None => (&self.coarse_input, &self.coarse_x),
        };
        let mut started = false;
        for _ in 0..self.cycle.pre() {
            smooth(s, l, b, started)?;
            started = true;
        }
        let corrections: &[CycleShape] = match shape {
            CycleShape::V => &[CycleShape::V],
            CycleShape::W => &[CycleShape::W, CycleShape::W],
            CycleShape::F => &[CycleShape::F, CycleShape::V],
        };
        for &inner in corrections {
            // Pᵀ (b − A x), or Pᵀ b while x is still zero
            if started {
                residual(s, l, b)?;
                s.product(&l.restriction, &l.r, next_input)?;
            } else {
                s.product(&l.restriction, b, next_input)?;
            }
            self.run(s, level + 1, next_input, inner)?;
            if started {
                s.product(&l.prolongation, next_x, &l.t)?;
                s.axpy(ONE, &l.t, &l.x)?;
            } else {
                s.product(&l.prolongation, next_x, &l.x)?;
            }
            started = true;
        }
        for _ in 0..self.cycle.post() {
            smooth(s, l, b, started)?;
        }
        Ok(())
    }
}

/// r = b − A x on level `l`.
fn residual(s: &Session, l: &Level, b: &DeviceVector) -> Result<()> {
    s.product(&l.a, &l.x, &l.r)?;
    s.axpby(ONE, b, -ONE, &l.r)
}

/// One smoothing step on level `l`: x ← x + U⁻¹ L⁻¹ (b − A x), or x = U⁻¹ L⁻¹ b from x = 0.
fn smooth(s: &Session, l: &Level, b: &DeviceVector, started: bool) -> Result<()> {
    if started {
        residual(s, l, b)?;
        l.l.solve(s, &l.r, &l.t)?;
        l.u.solve(s, &l.t, &l.r)?;
        s.axpy(ONE, &l.r, &l.x)
    } else {
        l.l.solve(s, b, &l.t)?;
        l.u.solve(s, &l.t, &l.x)
    }
}

/// The bytes the run keeps on the GPU, nearly: the matrices and factors at 12 bytes an entry
/// and 4 a row, and 16 a value for the vectors.
fn footprint(matrix: RowMatrix<'_>, cycle: &MultigridCycle<'_>) -> usize {
    let m = |a: RowMatrix<'_>| 12 * a.values().len() + 4 * a.rows();
    let n = matrix.rows();
    let mut bytes = m(matrix) + 16 * n * (cycle.restart() + 5) + 4 * n;
    for l in cycle.levels() {
        let f = l.smoother();
        bytes += m(l.operator()) + m(l.restriction()) + m(l.prolongation());
        bytes += 12 * (f.lower().2.len() + f.upper().2.len()) + 8 * f.n();
        bytes += 4 * 16 * l.operator().rows();
    }
    bytes
}

/// Restarted GMRES on A M⁻¹, M⁻¹ the cycle, as `fdfd::krylov::gmres_preconditioned`.
pub(crate) fn gmres(
    api: &Arc<Api>,
    matrix: RowMatrix<'_>,
    cycle: &MultigridCycle<'_>,
    b: &[c64],
    stopping: Stopping,
) -> Result<(Vec<c64>, Convergence)> {
    let _serial = serial();
    let n = matrix.rows();
    if matrix.columns() != n || b.len() != n {
        return Err(error(format!(
            "GMRES needs a square matrix and a value of b per row: {} × {}, {} values",
            n,
            matrix.columns(),
            b.len()
        )));
    }
    let finest = cycle
        .levels()
        .first()
        .map_or(cycle.coarsest().rows(), |l| l.operator().rows());
    if finest != n {
        return Err(error(format!(
            "a cycle of {finest} unknowns for a matrix of {n}"
        )));
    }
    let restart = cycle.restart();
    if stopping.tolerance.is_nan() || stopping.tolerance <= 0.0 || restart == 0 {
        return Err(error(
            "GMRES needs a positive tolerance and a restart of at least 1",
        ));
    }
    let rho0 = b.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
    let mut history = Vec::new();
    if rho0 == 0.0 {
        return Ok((
            vec![ZERO; n],
            Convergence {
                iterations: 0,
                residual: 0.0,
                history,
            },
        ));
    }
    let (needed, free) = (footprint(matrix, cycle), api.runtime.free_memory()?);
    if needed > free {
        return Err(error(format!(
            "GMRES with multigrid needs about {} MB on the GPU, which has {} MB free",
            needed >> 20,
            free >> 20
        )));
    }
    // the handle first: cuSPARSE's descriptors made before it fail later, in SpMV
    let mut s = Session::new(api, n)?;
    let a = upload(api, matrix)?;
    let zeros = vec![ZERO; n];
    let device_b = s.vector(b)?;
    let x = s.vector(&zeros)?;
    let residual = s.vector(b)?;
    let (w, u) = (s.vector(&zeros)?, s.vector(&zeros)?);
    s.prepare(&a, &x, &w)?;
    let gpu = Cycle::new(&mut s, cycle)?;
    // the Arnoldi basis, made as it grows and kept across restarts
    let mut v: Vec<DeviceVector> = Vec::new();
    let target = stopping.tolerance * rho0;
    loop {
        // line 1: r₀ = b − A x₀, β = ‖r₀‖, v₁ = r₀ / β
        let beta = s.norm2(&residual)?.sqrt();
        if beta <= target || history.len() >= stopping.max_iterations {
            break;
        }
        if v.is_empty() {
            v.push(s.vector(&zeros)?);
        }
        s.axpby(c64::new(1.0 / beta, 0.0), &residual, ZERO, &v[0])?;
        let mut columns: Vec<Vec<c64>> = Vec::new();
        let mut rotations: Vec<(c64, f64)> = Vec::new();
        let mut g = vec![c64::new(beta, 0.0)];
        while columns.len() < restart && history.len() < stopping.max_iterations {
            let j = columns.len();
            // lines 3–8: w = A M⁻¹ v_j, orthogonalized against the basis
            gpu.apply(&s, &v[j])?;
            s.product(&a, gpu.result(), &w)?;
            let mut h = Vec::with_capacity(j + 2);
            for vi in &v[..=j] {
                let hij = s.dot(vi, &w, true)?;
                s.axpy(-hij, vi, &w)?;
                h.push(hij);
            }
            let next = s.norm2(&w)?.sqrt();
            for (i, &(c, sn)) in rotations.iter().enumerate() {
                let (p, q) = (h[i], h[i + 1]);
                h[i] = c.conj() * p + sn * q;
                h[i + 1] = -sn * p + c * q;
            }
            let size = (h[j].norm_sqr() + next * next).sqrt();
            if size == 0.0 {
                break;
            }
            let (c, sn) = (h[j] / size, next / size);
            h[j] = c64::new(size, 0.0);
            g.push(-sn * g[j]);
            g[j] = c.conj() * g[j];
            rotations.push((c, sn));
            h.truncate(j + 1);
            columns.push(h);
            history.push(g[j + 1].norm() / rho0);
            if g[j + 1].norm() <= target || next == 0.0 {
                break;
            }
            if v.len() == j + 1 {
                v.push(s.vector(&zeros)?);
            }
            s.axpby(c64::new(1.0 / next, 0.0), &w, ZERO, &v[j + 1])?;
        }
        // line 11: y from the triangle, x = x₀ + M⁻¹ V y
        let steps = columns.len();
        if steps == 0 {
            break;
        }
        let mut y = vec![ZERO; steps];
        for i in (0..steps).rev() {
            let mut sum = g[i];
            for (jj, column) in columns.iter().enumerate().skip(i + 1) {
                sum -= column[i] * y[jj];
            }
            y[i] = sum / columns[i][i];
        }
        s.axpby(y[0], &v[0], ZERO, &u)?;
        for (yj, vj) in y.iter().zip(&v).skip(1) {
            s.axpy(*yj, vj, &u)?;
        }
        gpu.apply(&s, &u)?;
        s.axpy(ONE, gpu.result(), &x)?;
        // line 12: the true residual, to stop on or to restart from
        s.product(&a, &x, &residual)?;
        s.axpby(ONE, &device_b, -ONE, &residual)?;
    }
    let reached = s.norm2(&residual)?.sqrt() / rho0;
    api.runtime.synchronize("GMRES on the GPU")?;
    if reached.is_nan() || reached > stopping.tolerance {
        return Err(error(format!(
            "GMRES: no convergence to {:e} in {} iterations (the residual is at {reached:e})",
            stopping.tolerance, stopping.max_iterations
        )));
    }
    let x = x.read()?;
    Ok((
        x,
        Convergence {
            iterations: history.len(),
            residual: reached,
            history,
        },
    ))
}
