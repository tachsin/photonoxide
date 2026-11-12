//! photonoxide's GMRES preconditioned by its multigrid cycle, on an NVIDIA GPU (#190, #337): the
//! run of `fdfd::krylov::gmres_preconditioned`, step for step, on cuSPARSE, with the cycle
//! photonoxide builds ([`MultigridCycle`]) run on the GPU too, and kept there between solves
//! ([`Prepared`]).
//!
//! - **GMRES:** Saad's Algorithm 9.5 restarted as 6.11, as photonoxide's own: the Arnoldi basis
//!   kept on the GPU, orthogonalized by modified Gram–Schmidt (`cusparseSpVV`, then
//!   `cusparseAxpby`), the Hessenberg matrix's columns, the Givens rotations and the triangular
//!   solve for y on the host, where only the scalars come.
//! - **The cycle,** on every level: the residual by the level's operator (`cusparseSpMV`), the
//!   smoother's two triangular solves with photonoxide's ILU(0) factors of the level's slabs
//!   (`cusparseSpSV`: one pair of block-diagonal factors, whose independent blocks its analysis
//!   finds), and the restriction Pᵀ and prolongation P as rectangular products. A restriction
//!   weighted for a coarse level discretized again, W⁻¹ Pᵀ D_f, is one matrix, its rows and
//!   columns scaled on the host.
//! - **Without the finest matrix** (#337): the finest level's operator, and GMRES's, is Shin and
//!   Fan's stencil, a compute shader of photonoxide's own on Vulkan on buffers cuSPARSE shares
//!   ([`crate::interop`]); its damped Jacobi is a product with the diagonal ω D⁻¹.
//! - **The coarsest solve:** on the CPU, by photonoxide's factorization of the coarsest level
//!   (at most 2 000 unknowns by default, 32 kB): its right-hand side comes back from the GPU and
//!   its solution goes up again. Measured on the 68³ guide of `fdfd3d-iterative/guide-multigrid-68`
//!   (943 k unknowns, the coarsest level 5³ cells, 375 unknowns), the 23 coarsest solves of a
//!   1.6 s solve took 5 ms with their copies, so neither cuDSS nor a dense inverse on the GPU
//!   has anything to win; and they keep the bits of photonoxide's own coarsest solve.
//! - **What it costs to set up:** the hierarchy copied to the GPU and the smoothers' triangular
//!   factors analysed, 0.4 s of that 1.6 s: done once per hierarchy ([`Prepared`]), which an
//!   S-matrix's ports and a recycled sweep's solves on one solver then share.
//! - **Determinism:** cuSPARSE's sums, not photonoxide's chunked ones, so the iterations agree
//!   with the CPU's to rounding, and a repeated run on one GPU gives the same bits, which the
//!   tests check.

use std::sync::{Arc, Mutex};

use num_complex::Complex64 as c64;
use photonoxide::Result;
use photonoxide::backend::{Fine, IluFactors, MultigridCycle, PreparedMultigrid, RowMatrix};
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
/// in the order photonoxide sums it in, and is sorted here); its rows scaled by `left` and its
/// columns by `right`, if given.
fn upload(
    api: &Arc<Api>,
    m: RowMatrix<'_>,
    scaling: Option<(&[c64], &[c64])>,
) -> Result<DeviceMatrix> {
    use rayon::prelude::*;
    let (starts, columns, values) = (m.starts(), m.indices(), m.values());
    index(values.len())?;
    index(m.columns())?;
    let ascending = (0..m.rows()).into_par_iter().all(|r| {
        columns[starts[r]..starts[r + 1]]
            .windows(2)
            .all(|p| p[0] < p[1])
    });
    let mut rows = Rows {
        starts: indices(starts)?,
        columns: indices(columns)?,
    };
    if ascending && scaling.is_none() {
        return DeviceMatrix::with_columns(api, &rows, m.columns(), values);
    }
    let mut sorted = values.to_vec();
    let mut row = Vec::new();
    for r in 0..m.rows() {
        let range = starts[r]..starts[r + 1];
        row.clear();
        row.extend(range.clone().map(|k| (rows.columns[k], values[k])));
        row.sort_by_key(|e| e.0);
        for (k, &(c, v)) in range.zip(&row) {
            rows.columns[k] = c;
            sorted[k] = match scaling {
                // W⁻¹ Pᵀ D_f: the row's W⁻¹, the column's D_f
                Some((right, left)) => left[r] * v * right[c as usize],
                None => v,
            };
        }
    }
    DeviceMatrix::with_columns(api, &rows, m.columns(), &sorted)
}

/// A diagonal matrix on the GPU.
fn diagonal(api: &Arc<Api>, values: &[c64]) -> Result<DeviceMatrix> {
    let n = values.len();
    index(n)?;
    let rows = Rows {
        starts: (0..=n as i32).collect(),
        columns: (0..n as i32).collect(),
    };
    DeviceMatrix::with_columns(api, &rows, n, values)
}

/// A factor's rows, for [`Triangular`].
fn factor((starts, columns, _): (&[usize], &[usize], &[c64])) -> Result<Rows> {
    Ok(Rows {
        starts: indices(starts)?,
        columns: indices(columns)?,
    })
}

/// A level's operator: its matrix, or the stencil's (the shifted one, 0 in [`Stencil`]'s list).
enum Operator {
    Matrix(DeviceMatrix),
    Stencil,
}

/// A level's smoother: ILU(0)'s two triangles, or damped Jacobi's ω D⁻¹.
enum Smoother {
    Ilu { l: Triangular, u: Triangular },
    Jacobi(DeviceMatrix),
}

/// One level above the coarsest on the GPU: its operator, smoother, smoothing steps and
/// transfers, and its vectors: x, the cycle's result here; r and t, work; `input`, the
/// right-hand side the level above restricts to it (unused on the finest, whose comes from
/// GMRES).
struct Level {
    operator: Operator,
    smoother: Smoother,
    steps: (usize, usize),
    restriction: DeviceMatrix,
    prolongation: DeviceMatrix,
    input: DeviceVector,
    x: DeviceVector,
    r: DeviceVector,
    t: DeviceVector,
}

impl Level {
    /// The level's vectors, `n` values each.
    fn vectors(s: &Session, n: usize) -> Result<[DeviceVector; 4]> {
        let zeros = vec![ZERO; n];
        Ok([
            s.vector(&zeros)?,
            s.vector(&zeros)?,
            s.vector(&zeros)?,
            s.vector(&zeros)?,
        ])
    }

    fn stored(
        s: &mut Session,
        l: &photonoxide::backend::MultigridLevel<'_>,
        steps: (usize, usize),
    ) -> Result<Level> {
        let api = s.api.clone();
        let (a, smoother) = (l.operator(), l.smoother());
        let [input, x, r, t] = Level::vectors(s, a.rows())?;
        let (lower, upper) = (factor(smoother.lower())?, factor(smoother.upper())?);
        let (lower_values, upper_values) = (smoother.lower().2, smoother.upper().2);
        let level = Level {
            operator: Operator::Matrix(upload(&api, a, None)?),
            smoother: Smoother::Ilu {
                l: Triangular::new(s, (&lower, lower_values), true, true, &r, &t)?,
                u: Triangular::new(s, (&upper, upper_values), false, false, &t, &r)?,
            },
            steps,
            restriction: upload(&api, l.restriction(), l.scaling())?,
            prolongation: upload(&api, l.prolongation(), None)?,
            input,
            x,
            r,
            t,
        };
        if let Operator::Matrix(m) = &level.operator {
            s.prepare(m, &level.x, &level.r)?;
        }
        Ok(level)
    }

    fn stencil(s: &mut Session, l: &photonoxide::backend::StencilLevel<'_>) -> Result<Level> {
        let api = s.api.clone();
        let n = l.operator().unknowns();
        let [input, x, r, t] = Level::vectors(s, n)?;
        let jacobi = diagonal(&api, l.jacobi())?;
        s.prepare(&jacobi, &x, &r)?;
        Ok(Level {
            operator: Operator::Stencil,
            smoother: Smoother::Jacobi(jacobi),
            steps: (l.pre(), l.post()),
            restriction: upload(&api, l.restriction(), l.scaling())?,
            prolongation: upload(&api, l.prolongation(), None)?,
            input,
            x,
            r,
            t,
        })
    }
}

/// The stencil on the GPU, the cycle's operator (0) and GMRES's (1), and the two vectors it
/// shares with cuSPARSE: its input and output.
#[cfg(any(windows, target_os = "linux"))]
struct Stencil {
    kernel: crate::interop::GpuStencil,
    input: DeviceVector,
    output: DeviceVector,
    // last: the memory the vectors above see, freed after them
    _shared: [crate::interop::Shared; 2],
}

#[cfg(not(any(windows, target_os = "linux")))]
enum Stencil {}

#[cfg(any(windows, target_os = "linux"))]
impl Stencil {
    fn new(
        s: &Session,
        interop: &Arc<crate::interop::Interop>,
        operators: &[photonoxide::backend::Stencil<'_>],
    ) -> Result<Stencil> {
        let n = operators[0].unknowns();
        let bytes = (n * size_of::<c64>()) as u64;
        let (mut a, mut b) = (interop.shared(bytes)?, interop.shared(bytes)?);
        let kernel = crate::interop::GpuStencil::new(
            interop,
            operators,
            crate::interop::Precision::Double,
            &a.buffer,
            &b.buffer,
        )?;
        let take = |shared: &mut crate::interop::Shared| {
            shared
                .take_cuda()
                .ok_or_else(|| error("a shared buffer's CUDA view, taken twice"))
        };
        Ok(Stencil {
            kernel,
            input: s.vector_over(take(&mut a)?, n)?,
            output: s.vector_over(take(&mut b)?, n)?,
            _shared: [a, b],
        })
    }

    /// y = A x by operator `which`: x copied into the input on cuSPARSE, CUDA's work finished,
    /// the product run on Vulkan and waited for, and the output copied to y.
    fn product(
        &self,
        s: &Session,
        which: usize,
        x: &DeviceVector,
        y: &DeviceVector,
    ) -> Result<()> {
        s.copy(x, &self.input)?;
        s.api.runtime.synchronize("the stencil's input")?;
        self.kernel.apply(which)?;
        s.copy(&self.output, y)
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
impl Stencil {
    fn new(_: &Session, device: &Device, _: &[photonoxide::backend::Stencil<'_>]) -> Result<Stencil> {
        match *device {}
    }

    fn product(&self, _: &Session, _: usize, _: &DeviceVector, _: &DeviceVector) -> Result<()> {
        match *self {}
    }
}

/// GMRES's operator on the GPU: the matrix, or the stencil's (1 in [`Stencil`]'s list).
enum FineOperator {
    Matrix(DeviceMatrix),
    Stencil,
}

/// The Vulkan device the stencil runs on, where there can be one.
#[cfg(any(windows, target_os = "linux"))]
pub(crate) type Device = Arc<crate::interop::Interop>;
#[cfg(not(any(windows, target_os = "linux")))]
pub(crate) type Device = std::convert::Infallible;

/// GMRES and the cycle on the GPU, as a backend keeps them between solves: the session, the
/// levels above the coarsest (finest first), the coarsest's right-hand side and solution,
/// GMRES's operator and vectors.
pub(crate) struct Prepared {
    fine: FineOperator,
    stencil: Option<Stencil>,
    levels: Vec<Level>,
    coarse_input: DeviceVector,
    coarse_x: DeviceVector,
    /// The cycle's shape, its finest unknowns and its coarsest's, as prepared.
    shape: (CycleShape, usize, usize, usize),
    /// GMRES's right-hand side, iterate, residual and work.
    b: DeviceVector,
    x: DeviceVector,
    residual: DeviceVector,
    w: DeviceVector,
    u: DeviceVector,
    /// The Arnoldi basis, made as it grows and kept across restarts and solves.
    basis: Mutex<Vec<DeviceVector>>,
    // last: cuSPARSE's handle and the index array the vectors above share outlive them
    s: Session,
}

// SAFETY: cuSPARSE's handle, descriptors and device memory aren't tied to a thread; every call on
// them is behind serial(), which `run` holds throughout
unsafe impl Send for Prepared {}
// SAFETY: as Send's
unsafe impl Sync for Prepared {}

/// The bytes a run keeps on the GPU, nearly: the matrices and factors at 12 bytes an entry
/// and 4 a row, 16 a value for the vectors; the stencil's 6 values an unknown (ε, H, the
/// shared input and output, and φ and ε⁻¹ at the nodes) and Jacobi's diagonal.
fn footprint(fine: &Fine<'_>, cycle: &MultigridCycle<'_>) -> usize {
    let m = |a: RowMatrix<'_>| 12 * a.values().len() + 4 * a.rows();
    let n = fine.rows();
    let mut bytes = 16 * n * (cycle.restart() + 5) + 4 * n;
    bytes += match fine {
        Fine::Matrix(a) => m(*a),
        _ => 6 * 16 * n,
    };
    if let Some(l) = cycle.stencil() {
        bytes += 24 * n + m(l.restriction()) + m(l.prolongation()) + 4 * 16 * n;
    }
    for l in cycle.levels() {
        let f: &IluFactors = l.smoother();
        bytes += m(l.operator()) + m(l.restriction()) + m(l.prolongation());
        bytes += 12 * (f.lower().2.len() + f.upper().2.len()) + 8 * f.n();
        bytes += 4 * 16 * l.operator().rows();
    }
    bytes
}

impl Prepared {
    /// GMRES's operator `fine` and the `cycle` copied to the GPU and the smoothers analysed;
    /// `device` for a stencil (GMRES's or the finest level's).
    ///
    /// # Errors
    ///
    /// For an operator and a cycle of different sizes, a stencil without a device, what wouldn't
    /// fit in the GPU's free memory, and cuSPARSE's, CUDA's and Vulkan's errors.
    pub(crate) fn new(
        api: &Arc<Api>,
        device: Option<Device>,
        fine: Fine<'_>,
        cycle: &MultigridCycle<'_>,
    ) -> Result<Prepared> {
        let _serial = serial();
        let n = fine.rows();
        if cycle.unknowns() != n {
            return Err(error(format!(
                "a cycle of {} unknowns for an operator of {n}",
                cycle.unknowns()
            )));
        }
        if let Fine::Matrix(m) = fine
            && m.columns() != n
        {
            return Err(error(format!(
                "GMRES needs a square matrix: {n} × {}",
                m.columns()
            )));
        }
        if cycle.restart() == 0 {
            return Err(error("GMRES needs a restart of at least 1"));
        }
        let (needed, free) = (footprint(&fine, cycle), api.runtime.free_memory()?);
        if needed > free {
            return Err(error(format!(
                "GMRES with multigrid needs about {} MB on the GPU, which has {} MB free",
                needed >> 20,
                free >> 20
            )));
        }
        // the handle first: cuSPARSE's descriptors made before it fail later, in SpMV
        let mut s = Session::new(api, n)?;
        // the stencils: the finest level's (shifted), then GMRES's
        let mut stencils = Vec::new();
        if let Some(l) = cycle.stencil() {
            stencils.push(l.operator());
        }
        if let Fine::Stencil(st) = fine {
            if stencils.is_empty() {
                return Err(error("GMRES on a stencil with a cycle whose finest level has a matrix"));
            }
            stencils.push(st);
        }
        let stencil = if stencils.is_empty() {
            None
        } else {
            let device = device
                .as_ref()
                .ok_or_else(|| error("a stencil without a Vulkan device to run it"))?;
            Some(Stencil::new(&s, device, &stencils)?)
        };
        let fine_operator = match fine {
            Fine::Matrix(m) => FineOperator::Matrix(upload(api, m, None)?),
            _ if stencils.len() == 2 => FineOperator::Stencil,
            _ => return Err(error("GMRES's operator neither a matrix nor the stencil")),
        };
        let mut levels = Vec::with_capacity(cycle.levels().len() + 1);
        if let Some(l) = cycle.stencil() {
            levels.push(Level::stencil(&mut s, l)?);
        }
        for l in cycle.levels() {
            levels.push(Level::stored(&mut s, l, (cycle.pre(), cycle.post()))?);
        }
        let coarsest = cycle.coarsest().rows();
        let zeros = vec![ZERO; coarsest];
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
        let zeros = vec![ZERO; n];
        let (b, x, residual, w, u) = (
            s.vector(&zeros)?,
            s.vector(&zeros)?,
            s.vector(&zeros)?,
            s.vector(&zeros)?,
            s.vector(&zeros)?,
        );
        if let FineOperator::Matrix(a) = &fine_operator {
            s.prepare(a, &x, &w)?;
        } else {
            // the dot products' workspace, on vectors as long
            let jacobi = match levels.first().map(|l| &l.smoother) {
                Some(Smoother::Jacobi(d)) => d,
                _ => return Err(error("the stencil's level without its Jacobi")),
            };
            s.prepare(jacobi, &x, &w)?;
        }
        Ok(Prepared {
            fine: fine_operator,
            stencil,
            levels,
            coarse_input,
            coarse_x,
            shape: (cycle.shape(), n, coarsest, cycle.levels().len()),
            b,
            x,
            residual,
            w,
            u,
            basis: Mutex::new(Vec::new()),
            s,
        })
    }

    /// The finest level's result, M⁻¹ v.
    fn result(&self) -> &DeviceVector {
        self.levels.first().map_or(&self.coarse_x, |l| &l.x)
    }

    /// M⁻¹ `b`, into [`Prepared::result`].
    fn apply(&self, cycle: &MultigridCycle<'_>, b: &DeviceVector) -> Result<()> {
        self.cycle(cycle, 0, b, self.shape.0)
    }

    /// y = A x, GMRES's operator.
    fn product(&self, x: &DeviceVector, y: &DeviceVector) -> Result<()> {
        match (&self.fine, &self.stencil) {
            (FineOperator::Matrix(a), _) => self.s.product(a, x, y),
            (FineOperator::Stencil, Some(stencil)) => stencil.product(&self.s, 1, x, y),
            (FineOperator::Stencil, None) => Err(error("GMRES's stencil isn't on the GPU")),
        }
    }

    /// r = b − A x on level `l`.
    fn residual(&self, l: &Level, b: &DeviceVector) -> Result<()> {
        match (&l.operator, &self.stencil) {
            (Operator::Matrix(a), _) => self.s.product(a, &l.x, &l.r)?,
            (Operator::Stencil, Some(stencil)) => stencil.product(&self.s, 0, &l.x, &l.r)?,
            (Operator::Stencil, None) => return Err(error("the cycle's stencil isn't on the GPU")),
        }
        self.s.axpby(ONE, b, -ONE, &l.r)
    }

    /// One smoothing step on level `l`: x ← x + M⁻¹ (b − A x), or x = M⁻¹ b from x = 0.
    fn smooth(&self, l: &Level, b: &DeviceVector, started: bool) -> Result<()> {
        let s = &self.s;
        match &l.smoother {
            Smoother::Ilu { l: lower, u: upper } => {
                if started {
                    self.residual(l, b)?;
                    lower.solve(s, &l.r, &l.t)?;
                    upper.solve(s, &l.t, &l.r)?;
                    s.axpy(ONE, &l.r, &l.x)
                } else {
                    lower.solve(s, b, &l.t)?;
                    upper.solve(s, &l.t, &l.x)
                }
            }
            Smoother::Jacobi(d) => {
                if started {
                    self.residual(l, b)?;
                    s.product(d, &l.r, &l.t)?;
                    s.axpy(ONE, &l.t, &l.x)
                } else {
                    s.product(d, b, &l.x)
                }
            }
        }
    }

    /// One cycle from `level` for `b`, from x = 0, into the level's x.
    fn cycle(
        &self,
        cycle: &MultigridCycle<'_>,
        level: usize,
        b: &DeviceVector,
        shape: CycleShape,
    ) -> Result<()> {
        let s = &self.s;
        let Some(l) = self.levels.get(level) else {
            // a failed solve gives NaNs, as photonoxide's own cycle, which GMRES then reports
            // as a solve that didn't converge
            let rows = self.shape.2;
            let rhs = b.read()?;
            let x = cycle
                .coarsest_solve(&rhs[..rows])
                .unwrap_or_else(|_| vec![c64::new(f64::NAN, f64::NAN); rows]);
            return self.coarse_x.write(&x);
        };
        let (next_input, next_x) = match self.levels.get(level + 1) {
            Some(next) => (&next.input, &next.x),
            None => (&self.coarse_input, &self.coarse_x),
        };
        let mut started = false;
        for _ in 0..l.steps.0 {
            self.smooth(l, b, started)?;
            started = true;
        }
        let corrections: &[CycleShape] = match shape {
            CycleShape::V => &[CycleShape::V],
            CycleShape::W => &[CycleShape::W, CycleShape::W],
            CycleShape::F => &[CycleShape::F, CycleShape::V],
        };
        for &inner in corrections {
            // R (b − A x), or R b while x is still zero
            if started {
                self.residual(l, b)?;
                s.product(&l.restriction, &l.r, next_input)?;
            } else {
                s.product(&l.restriction, b, next_input)?;
            }
            self.cycle(cycle, level + 1, next_input, inner)?;
            if started {
                s.product(&l.prolongation, next_x, &l.t)?;
                s.axpy(ONE, &l.t, &l.x)?;
            } else {
                s.product(&l.prolongation, next_x, &l.x)?;
            }
            started = true;
        }
        for _ in 0..l.steps.1 {
            self.smooth(l, b, started)?;
        }
        Ok(())
    }

    /// Restarted GMRES on A M⁻¹, M⁻¹ the cycle, as `fdfd::krylov::gmres_preconditioned`.
    pub(crate) fn run(
        &self,
        cycle: &MultigridCycle<'_>,
        b: &[c64],
        stopping: Stopping,
    ) -> Result<(Vec<c64>, Convergence)> {
        let _serial = serial();
        let s = &self.s;
        let (_, n, coarsest, depth) = self.shape;
        if b.len() != n
            || cycle.unknowns() != n
            || cycle.coarsest().rows() != coarsest
            || cycle.levels().len() != depth
        {
            return Err(error(format!(
                "GMRES prepared for {n} unknowns, given a right-hand side of {} and a cycle of {}",
                b.len(),
                cycle.unknowns()
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
        let zeros = vec![ZERO; n];
        self.b.write(b)?;
        self.residual.write(b)?;
        self.x.write(&zeros)?;
        let (x, residual, w, u) = (&self.x, &self.residual, &self.w, &self.u);
        let mut v = self.basis.lock().unwrap_or_else(|p| p.into_inner());
        let target = stopping.tolerance * rho0;
        loop {
            // line 1: r₀ = b − A x₀, β = ‖r₀‖, v₁ = r₀ / β
            let beta = s.norm2(residual)?.sqrt();
            if beta <= target || history.len() >= stopping.max_iterations {
                break;
            }
            if v.is_empty() {
                v.push(s.vector(&zeros)?);
            }
            s.axpby(c64::new(1.0 / beta, 0.0), residual, ZERO, &v[0])?;
            let mut columns: Vec<Vec<c64>> = Vec::new();
            let mut rotations: Vec<(c64, f64)> = Vec::new();
            let mut g = vec![c64::new(beta, 0.0)];
            while columns.len() < restart && history.len() < stopping.max_iterations {
                let j = columns.len();
                // lines 3–8: w = A M⁻¹ v_j, orthogonalized against the basis
                self.apply(cycle, &v[j])?;
                self.product(self.result(), w)?;
                let mut h = Vec::with_capacity(j + 2);
                for vi in &v[..=j] {
                    let hij = s.dot(vi, w, true)?;
                    s.axpy(-hij, vi, w)?;
                    h.push(hij);
                }
                let next = s.norm2(w)?.sqrt();
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
                s.axpby(c64::new(1.0 / next, 0.0), w, ZERO, &v[j + 1])?;
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
            s.axpby(y[0], &v[0], ZERO, u)?;
            for (yj, vj) in y.iter().zip(v.iter()).skip(1) {
                s.axpy(*yj, vj, u)?;
            }
            self.apply(cycle, u)?;
            s.axpy(ONE, self.result(), x)?;
            // line 12: the true residual, to stop on or to restart from
            self.product(x, residual)?;
            s.axpby(ONE, &self.b, -ONE, residual)?;
        }
        let reached = s.norm2(residual)?.sqrt() / rho0;
        s.api.runtime.synchronize("GMRES on the GPU")?;
        if reached.is_nan() || reached > stopping.tolerance {
            return Err(error(format!(
                "GMRES: no convergence to {:e} in {} iterations (the residual is at {reached:e})",
                stopping.tolerance, stopping.max_iterations
            )));
        }
        let mut out = x.read()?;
        out.truncate(n);
        Ok((
            out,
            Convergence {
                iterations: history.len(),
                residual: reached,
                history,
            },
        ))
    }
}

impl PreparedMultigrid for Prepared {
    fn gmres(
        &self,
        cycle: &MultigridCycle<'_>,
        b: &[c64],
        stopping: Stopping,
    ) -> Result<(Vec<c64>, Convergence)> {
        self.run(cycle, b, stopping)
    }
}

#[cfg(all(test, any(windows, target_os = "linux")))]
mod tests {
    use std::sync::{Arc, Mutex};

    use num_complex::Complex64 as c64;
    use photonoxide::Result;
    use photonoxide::backend::{
        Capabilities, Choice, Fine, IterativeSolver, Matrix, MultigridCycle, PreparedMultigrid,
        QmrRun, register_iterative,
    };
    use photonoxide::fdfd::{
        Boundaries3d, Edges, Formulation, Grid3d, IterativeSolver3d, Multigrid, Product, Stopping,
    };
    use photonoxide::units::Wavelength;

    use super::{Session, Stencil, ZERO};
    use crate::gpu_qmr::GpuQmr;
    use crate::library::error;

    /// A backend that, given the multigrid without its finest matrix, multiplies a vector by
    /// both its stencils on the GPU, through the buffers CUDA and Vulkan share, and records how
    /// far each product is from photonoxide's (`Stencil::apply`) and whether a repeat gave the
    /// same bits; then refuses the solve.
    struct Probe {
        gpu: GpuQmr,
        found: Mutex<Vec<(f64, f64, bool)>>,
    }

    fn off(a: &[c64], b: &[c64]) -> f64 {
        let scale = b.iter().map(|z| z.norm()).fold(0.0, f64::max);
        a.iter()
            .zip(b)
            .map(|(p, q)| (p - q).norm())
            .fold(0.0, f64::max)
            / scale
    }

    impl IterativeSolver for Probe {
        fn capabilities(&self) -> Capabilities {
            Capabilities::new("stencil-probe-for-tests", "1", "MIT OR Apache-2.0")
        }

        fn qmr_run(&self, _: &Matrix<'_>, _: &[c64], _: Stopping) -> Result<QmrRun> {
            Err(error("runs no QMR"))
        }

        fn runs_multigrid(&self) -> bool {
            true
        }

        fn runs_multigrid_stencil(&self) -> bool {
            true
        }

        fn prepare_multigrid(
            &self,
            fine: Fine<'_>,
            cycle: &MultigridCycle<'_>,
        ) -> Result<Option<Box<dyn PreparedMultigrid>>> {
            let Fine::Stencil(plain) = fine else {
                return Err(error("a stencil"));
            };
            let shifted = cycle.stencil().unwrap().operator();
            let interop = self.gpu.interop().map_err(error)?;
            let n = plain.unknowns();
            let s = Session::new(&self.gpu.api, n)?;
            let stencil = Stencil::new(&s, &interop, &[shifted, plain])?;
            // a vector with no symmetry, nonzero on the walls too
            let v: Vec<c64> = (0..n)
                .map(|r| {
                    let t = r as f64;
                    c64::new((0.37 * t).sin() + 0.2, (0.11 * t * t).cos() - 0.4)
                })
                .collect();
            let (x, y) = (s.vector(&v)?, s.vector(&vec![ZERO; n])?);
            let mut found = Vec::new();
            for (which, operator) in [(0, shifted), (1, plain)] {
                stencil.product(&s, which, &x, &y)?;
                let first = y.read()?;
                stencil.product(&s, which, &x, &y)?;
                let again = y.read()?;
                let bits = |v: &[c64]| {
                    v.iter()
                        .flat_map(|z| [z.re.to_bits(), z.im.to_bits()])
                        .collect::<Vec<_>>()
                };
                found.push((off(&first, &operator.apply(&v)), 0.0, bits(&first) == bits(&again)));
            }
            self.found.lock().unwrap().push((found[0].0, found[1].0, found[0].2 && found[1].2));
            Err(error("probed"))
        }
    }

    /// Shin and Fan's stencil multiplied on the GPU, the cycle's (shifted) and GMRES's, against
    /// photonoxide's on the CPU, for PMLs, walls and Bloch sides: within 1e-13 of the largest
    /// value, and a repeat the same bits.
    #[test]
    fn the_stencil_on_the_gpu_is_photonoxides_to_rounding_and_repeats() {
        if std::env::var_os("CI").is_some() {
            println!("skipped: on CI, which has no GPU");
            return;
        }
        let gpu = match GpuQmr::load() {
            Ok(g) => g,
            Err(reason) => {
                println!("skipped: cuSPARSE isn't here: {reason}");
                return;
            }
        };
        if let Err(reason) = gpu.stencil_device() {
            println!("skipped: no Vulkan device shares memory with CUDA here: {reason}");
            return;
        }
        let probe = Arc::new(Probe {
            gpu,
            found: Mutex::new(Vec::new()),
        });
        register_iterative(probe.clone()).unwrap();
        let grid = Grid3d {
            nx: 12,
            ny: 10,
            nz: 11,
            dx: 0.04,
            dy: 0.05,
            dz: 0.045,
            x0: -0.22,
            y0: -0.24,
            z0: -0.25,
        };
        let eps = |x: f64, y: f64, z: f64| {
            let inside = (x - 0.02).abs() < 0.11 && (y + 0.01).abs() < 0.08 && z.abs() < 0.1;
            c64::new(
                if inside { 12.1 } else { 2.1 },
                if inside { 0.05 } else { 0.0 },
            )
        };
        let bloch = |k: f64| Edges::Bloch { k };
        let cases = [
            ("stretched PMLs", Boundaries3d::stretched_pml(3)),
            ("walls", Boundaries3d::pml(0)),
            (
                "Bloch along x",
                Boundaries3d {
                    x: bloch(0.8),
                    ..Boundaries3d::stretched_pml(3)
                },
            ),
            (
                "Bloch along y and z",
                Boundaries3d {
                    y: bloch(-1.3),
                    z: bloch(2.1),
                    ..Boundaries3d::stretched_pml(3)
                },
            ),
            (
                "Bloch on every side",
                Boundaries3d {
                    x: bloch(0.4),
                    y: bloch(0.0),
                    z: bloch(-0.9),
                    ..Boundaries3d::pml(0)
                },
            ),
        ];
        for (name, boundaries) in cases {
            let solver = IterativeSolver3d::new_with(
                grid,
                Wavelength::um(1.55).unwrap(),
                eps,
                boundaries,
                Formulation::ShinFan,
                Product::MatrixFree,
            )
            .unwrap()
            .with_multigrid(Multigrid::default())
            .unwrap()
            .with_iterative_backend(&Choice::Named("stencil-probe-for-tests".into()))
            .unwrap();
            let current = vec![c64::new(1.0, 0.0); grid.unknowns()];
            let stop = Stopping {
                tolerance: 1e-8,
                max_iterations: 10,
            };
            let e = solver.solve(&current, stop).err().unwrap();
            assert!(e.to_string().contains("probed"), "{name}: {e}");
            let (shifted, plain, repeats) = probe.found.lock().unwrap().pop().unwrap();
            println!("{name}: the cycle's {shifted:.1e} and GMRES's {plain:.1e} from the CPU's");
            assert!(shifted < 1e-13 && plain < 1e-13, "{name}: {shifted:e}, {plain:e}");
            assert!(repeats, "{name}: a repeated product differs");
        }
    }

    /// The fastest of `repeats` runs of `f`, in milliseconds.
    fn fastest(repeats: usize, mut f: impl FnMut()) -> f64 {
        f();
        (0..repeats)
            .map(|_| {
                let t = std::time::Instant::now();
                f();
                t.elapsed().as_secs_f64()
            })
            .fold(f64::INFINITY, f64::min)
            * 1e3
    }

    /// A buffer's values read back from Vulkan.
    fn read_back(interop: &crate::interop::Interop, buffer: &wgpu::Buffer) -> Vec<u8> {
        let size = buffer.size();
        let staging = interop.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = interop
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, size);
        interop.queue.submit([encoder.finish()]);
        staging.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        interop.wait().unwrap();
        let out = staging.slice(..).get_mapped_range().unwrap().to_vec();
        staging.unmap();
        out
    }

    /// The depth of a triangular factor's level schedule: the longest chain of rows each of
    /// which needs the one before (a row's level one more than its dependencies' largest).
    fn depth((starts, columns, _): (&[usize], &[usize], &[c64]), lower: bool) -> usize {
        let n = starts.len() - 1;
        let mut level = vec![0usize; n];
        let rows: Box<dyn Iterator<Item = usize>> = if lower {
            Box::new(0..n)
        } else {
            Box::new((0..n).rev())
        };
        for r in rows {
            let mut l = 0;
            for &c in &columns[starts[r]..starts[r + 1]] {
                if c != r {
                    l = l.max(level[c] + 1);
                }
            }
            level[r] = l;
        }
        level.iter().max().map_or(0, |m| m + 1)
    }

    /// What decides between the finest level's stencil on Vulkan beside cuSPARSE's cycle and the
    /// whole cycle on Vulkan, measured on the 68³ guide of `fdfd3d-iterative/guide-multigrid-
    /// stencil-68` (943 296 unknowns, Galerkin's coarse operators); docs/plans/backends.md has
    /// the results.
    struct Measure {
        gpu: GpuQmr,
        report: Mutex<Vec<String>>,
    }

    impl IterativeSolver for Measure {
        fn capabilities(&self) -> Capabilities {
            Capabilities::new("measure-for-tests", "1", "MIT OR Apache-2.0")
        }

        fn qmr_run(&self, _: &Matrix<'_>, _: &[c64], _: Stopping) -> Result<QmrRun> {
            Err(error("runs no QMR"))
        }

        fn runs_multigrid(&self) -> bool {
            true
        }

        fn runs_multigrid_stencil(&self) -> bool {
            true
        }

        fn prepare_multigrid(
            &self,
            fine: Fine<'_>,
            cycle: &MultigridCycle<'_>,
        ) -> Result<Option<Box<dyn PreparedMultigrid>>> {
            use crate::interop::{GpuStencil, Precision};
            let mut out = Vec::new();
            let interop = self.gpu.interop().map_err(error)?;
            out.push(format!("Vulkan: {}", interop.name()));
            if let Some(ratio) = self.gpu.api.runtime.single_to_double_ratio() {
                out.push(format!("CUDA's single to double precision ratio: {ratio}"));
            }
            let Fine::Stencil(plain) = fine else {
                return Err(error("a stencil"));
            };
            let shifted = cycle.stencil().unwrap().operator();
            let n = plain.unknowns();
            let mb = (n * 16) as f64 / 1e6;
            out.push(format!(
                "{n} unknowns ({mb:.1} MB a vector in f64); the levels below: {:?} rows, the coarsest {}",
                cycle
                    .levels()
                    .iter()
                    .map(|l| l.operator().rows())
                    .collect::<Vec<_>>(),
                cycle.coarsest().rows()
            ));
            // (a) the kernel alone, f64 and f32, 20 products a submission
            for precision in [Precision::Double, Precision::Single] {
                let bytes = n as u64 * precision.complex();
                let (input, output) = (interop.buffer(bytes, None), interop.buffer(bytes, None));
                let k = GpuStencil::new(&interop, &[shifted], precision, &input, &output)?;
                let ms = fastest(3, || {
                    interop.queue.submit([k.encode(0, 20)]);
                    interop.wait().unwrap();
                }) / 20.0;
                out.push(format!("the kernel alone in {precision:?}: {ms:.3} ms a product"));
            }
            // (b) a product as the cycle takes it, on the buffers CUDA shares: copies in and out
            // on cuSPARSE and both sides' waits
            let prepared = super::Prepared::new(
                &self.gpu.api,
                Some(interop.clone()),
                fine,
                cycle,
            )?;
            let s = &prepared.s;
            let stencil = prepared.stencil.as_ref().unwrap();
            let v: Vec<c64> = (0..n)
                .map(|r| c64::new((0.37 * r as f64).sin(), (0.11 * r as f64).cos()))
                .collect();
            let (x, y) = (s.vector(&v)?, s.vector(&vec![ZERO; n])?);
            let shared = fastest(10, || stencil.product(s, 0, &x, &y).unwrap());
            let sync = fastest(10, || {
                s.api.runtime.synchronize("a wait").unwrap();
                interop.queue.submit([]);
                interop.wait().unwrap();
            });
            let copy = fastest(10, || {
                s.copy(&x, &y).unwrap();
                s.api.runtime.synchronize("a copy").unwrap();
            });
            out.push(format!(
                "a product on the shared buffers: {shared:.3} ms (an empty wait on both sides {sync:.3} ms; a copy on cuSPARSE {copy:.3} ms)"
            ));
            // (c) the same vector through the host instead: CUDA to the host and back, and
            // Vulkan's write and read back
            let cuda = fastest(5, || {
                let host = y.read().unwrap();
                x.write(&host).unwrap();
            });
            let bytes = Precision::Double.complex() * n as u64;
            let plain_buffer = interop.buffer(bytes, None);
            let host = vec![0u8; bytes as usize];
            let vulkan = fastest(5, || {
                interop.queue.write_buffer(&plain_buffer, 0, &host);
                let _ = read_back(&interop, &plain_buffer);
            });
            out.push(format!(
                "a vector through the host: CUDA's down and up {cuda:.3} ms, Vulkan's up and down {vulkan:.3} ms"
            ));
            // (d) the cycle, its levels below the finest, and an iteration of GMRES
            let full = fastest(5, || prepared.apply(cycle, &x).unwrap());
            let below = if prepared.levels.len() > 1 {
                let l1 = &prepared.levels[1];
                fastest(5, || {
                    prepared.cycle(cycle, 1, &l1.input, cycle.shape()).unwrap();
                    s.api.runtime.synchronize("a cycle").unwrap();
                })
            } else {
                0.0
            };
            out.push(format!(
                "a cycle {full:.3} ms, of which the levels below the finest {below:.3} ms"
            ));
            // the first coarse level's smoother on cuSPARSE, and its level schedule's depth
            if let (Some(l1), Some(level)) = (prepared.levels.get(1), cycle.levels().first()) {
                if let super::Smoother::Ilu { l, u } = &l1.smoother {
                    let spsv = fastest(5, || {
                        l.solve(s, &l1.r, &l1.t).unwrap();
                        u.solve(s, &l1.t, &l1.r).unwrap();
                        s.api.runtime.synchronize("the triangles").unwrap();
                    });
                    let operator = level.operator();
                    let spmv = fastest(5, || {
                        if let super::Operator::Matrix(a) = &l1.operator {
                            s.product(a, &l1.x, &l1.r).unwrap();
                        }
                        s.api.runtime.synchronize("a product").unwrap();
                    });
                    let f = level.smoother();
                    out.push(format!(
                        "the first coarse level: {} rows, {} entries; a product {spmv:.3} ms, ILU(0)'s two triangular solves {spsv:.3} ms on cuSPARSE; their level schedules {} and {} deep",
                        operator.rows(),
                        operator.values().len(),
                        depth(f.lower(), true),
                        depth(f.upper(), false),
                    ));
                }
            }
            // a dispatch's cost on Vulkan, in a chain each waiting for the one before (as a
            // level schedule's would): one invocation adding to one value
            let chain = 1000;
            let per_dispatch = dispatch_chain(&interop, chain);
            out.push(format!(
                "a chain of {chain} dependent dispatches on Vulkan: {per_dispatch:.4} ms a dispatch"
            ));
            let b: Vec<c64> = v.clone();
            let t = std::time::Instant::now();
            let (_, how) = prepared.run(cycle, &b, Stopping {
                tolerance: 1e-8,
                max_iterations: 1000,
            })?;
            let solve = t.elapsed().as_secs_f64() * 1e3;
            out.push(format!(
                "GMRES to 1e-8: {} iterations in {solve:.0} ms, {:.2} ms an iteration (with the vector up and the field down)",
                how.iterations,
                solve / how.iterations as f64
            ));
            self.report.lock().unwrap().extend(out);
            Err(error("measured"))
        }
    }

    /// The milliseconds a dispatch takes in a chain of `count`, each reading what the one
    /// before wrote, in one pass.
    fn dispatch_chain(interop: &crate::interop::Interop, count: usize) -> f64 {
        let device = &interop.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                "@group(0) @binding(0) var<storage, read_write> x: array<f32>;\n\
                 @compute @workgroup_size(1) fn add() { x[0] = x[0] + 1.0; }"
                    .into(),
            ),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &module,
            entry_point: Some("add"),
            compilation_options: Default::default(),
            cache: None,
        });
        let buffer = interop.buffer(16, None);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        fastest(3, || {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &group, &[]);
                for _ in 0..count {
                    pass.dispatch_workgroups(1, 1, 1);
                }
            }
            interop.queue.submit([encoder.finish()]);
            interop.wait().unwrap();
        }) / count as f64
    }

    #[test]
    #[ignore = "a measurement for docs/plans/backends.md: cargo test -p photonoxide-native --release --lib gpu_multigrid::tests::measure -- --ignored --nocapture (MEASURE_N, the guide's cells a side, 68)"]
    fn measure() {
        let gpu = GpuQmr::load().expect("cuSPARSE");
        gpu.stencil_device().expect("a Vulkan device on CUDA's GPU");
        let probe = Arc::new(Measure {
            gpu,
            report: Mutex::new(Vec::new()),
        });
        register_iterative(probe.clone()).unwrap();
        let n: usize = std::env::var("MEASURE_N")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(68);
        // the catalogue's guide: 10 nm cells, a silicon core of n − 48 cells, stretched PMLs
        // of 14 (at 68)
        let (h, pml) = (0.01, 14);
        let core = n - 2 * pml;
        let half = core as f64 * h / 2.0;
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
        let eps = move |_: f64, y: f64, z: f64| {
            c64::new(
                if y.abs() < half && z.abs() < half {
                    12.09
                } else {
                    1.0
                },
                0.0,
            )
        };
        let t = std::time::Instant::now();
        let solver = IterativeSolver3d::new_with(
            grid,
            Wavelength::um(1.55).unwrap(),
            eps,
            Boundaries3d::stretched_pml(pml),
            Formulation::ShinFan,
            Product::MatrixFree,
        )
        .unwrap()
        .with_multigrid(Multigrid::default())
        .unwrap()
        .with_iterative_backend(&Choice::Named("measure-for-tests".into()))
        .unwrap();
        println!("built in {:.1} s", t.elapsed().as_secs_f64());
        let mut current = vec![c64::new(0.0, 0.0); grid.unknowns()];
        current[n / 2 + 3 + n * (n / 2 + 3 + n * (n / 2 + 3))] = c64::new(1.0, 0.0);
        let e = solver
            .solve(
                &current,
                Stopping {
                    tolerance: 1e-8,
                    max_iterations: 1000,
                },
            )
            .err()
            .unwrap();
        assert!(e.to_string().contains("measured"), "{e}");
        for line in probe.report.lock().unwrap().iter() {
            println!("{line}");
        }
    }
}

impl Drop for Prepared {
    fn drop(&mut self) {
        // its fields then go, one CUDA call at a time, the session last
        let _serial = serial();
        self.basis.lock().unwrap_or_else(|p| p.into_inner()).clear();
        self.levels.clear();
        self.stencil = None;
    }
}
