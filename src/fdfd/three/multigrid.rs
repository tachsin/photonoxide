//! A multigrid preconditioner for the 3D system: cycles on Shin and Fan's operator, with Galerkin
//! coarse operators, ILU(0) smoothing and a direct solve on the coarsest grid.
//!
//! B. Reps, W. Vanroose, H. bin Zubair, J. Comput. Phys. 229, 8384 (2010),
//! doi:10.1016/j.jcp.2010.07.022, Section 6.1 and Fig. 14: for Helmholtz problems with
//! complex-stretched absorbing layers, ILU(0) smoothing with Galerkin coarse operators, full
//! weighting and (bi)linear prolongation. Y. A. Erlangga, C. W. Oosterlee, C. Vuik, SIAM J. Sci.
//! Comput. 27, 1471 (2006), doi:10.1137/040615195: the complex shift of the preconditioner's
//! operator (their Eq. 8, (β₁, β₂)) and F-cycles (Section 4.3).
//!
//! On Yee's grid each component of E is interpolated from a coarser grid over the same box on
//! its own staggered points, trilinearly (P), restricted by Pᵀ, and each coarse operator is
//! Pᵀ A P.

use faer::sparse::linalg::solvers::Lu;
use num_complex::Complex64 as c64;

use super::{Axis, Boundaries3d, Grid3d, Lattice};
use crate::fdfd::Edges;
use crate::fdfd::krylov::{Ilu0, Operator, Preconditioner, Sparse};
use crate::{Error, Result};

/// The shape of a multigrid cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CycleShape {
    /// One coarse correction per level.
    V,
    /// An F-cycle: an F-cycle then a V-cycle on the coarser level.
    F,
    /// Two coarse corrections per level.
    W,
}

/// How the multigrid preconditioner is built and cycled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Multigrid {
    /// The complex shift β of the preconditioner's operator: k₀²(1 + iβ)ε on its diagonal
    /// (Erlangga et al.'s (β₁, β₂) = (1, β), on the side of the medium's loss). 0 for none.
    pub shift: f64,
    /// The cycle.
    pub shape: CycleShape,
    /// ILU(0) smoothing steps before each coarse correction.
    pub pre: usize,
    /// And after it.
    pub post: usize,
    /// The coarsest grid's largest number of unknowns, solved directly.
    pub coarsest: usize,
}

impl Default for Multigrid {
    /// Reps et al.'s V(0, 1) with no shift, the coarsest grid at most 20 000 unknowns.
    fn default() -> Multigrid {
        Multigrid {
            shift: 0.0,
            shape: CycleShape::V,
            pre: 0,
            post: 1,
            coarsest: 20_000,
        }
    }
}

/// One level: its operator, its smoother, and the prolongation from the next.
struct Level {
    matrix: Sparse,
    smoother: Ilu0,
    /// P by rows: for each value here, the next level's values and their weights.
    prolongation: Vec<Vec<(usize, c64)>>,
    /// The next level's size.
    coarse: usize,
}

/// The cycle's levels, finest first, and the coarsest's factorization.
pub(crate) struct Hierarchy {
    levels: Vec<Level>,
    lu: Lu<usize, c64>,
    options: Multigrid,
}

/// The grid of the next level: half the cells along each axis (rounded up, at least 2), the
/// same box, and PMLs of the same thickness.
fn coarser(grid: &Grid3d, boundaries: &Boundaries3d) -> (Grid3d, Boundaries3d) {
    let half = |n: usize| n.div_ceil(2).max(2);
    let (nx, ny, nz) = (half(grid.nx), half(grid.ny), half(grid.nz));
    let g = Grid3d {
        nx,
        ny,
        nz,
        dx: grid.dx * grid.nx as f64 / nx as f64,
        dy: grid.dy * grid.ny as f64 / ny as f64,
        dz: grid.dz * grid.nz as f64 / nz as f64,
        ..*grid
    };
    let scale = |e: Edges, n: usize, nc: usize| match e {
        Edges::Pml { low, high } => {
            let f = |l: usize| ((l * nc) as f64 / n as f64).round() as usize;
            Edges::Pml {
                low: f(low),
                high: f(high),
            }
        }
        bloch => bloch,
    };
    let b = Boundaries3d {
        x: scale(boundaries.x, grid.nx, nx),
        y: scale(boundaries.y, grid.ny, ny),
        z: scale(boundaries.z, grid.nz, nz),
        ..*boundaries
    };
    (g, b)
}

/// Trilinear interpolation of the coarse lattice's values to the fine one's value `r`: the
/// coarse values of the same component around its place, and their weights.
fn interpolation(fine: &Lattice, coarse: &Lattice, r: usize) -> Vec<(usize, c64)> {
    let (component, at) = fine.grid.at(r);
    let (gf, gc) = (fine.grid, coarse.grid);
    let mut along: [Vec<(usize, c64)>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for axis in Axis::ALL {
        let a = axis.index();
        let offset = if axis == component { 0.5 } else { 0.0 };
        let position = (at[a] as f64 + offset) * gf.step(axis);
        let t = position / gc.step(axis) - offset;
        let low = t.floor();
        let frac = t - low;
        let n = gc.n(axis) as i64;
        for (index, weight) in [(low as i64, 1.0 - frac), (low as i64 + 1, frac)] {
            if weight == 0.0 {
                continue;
            }
            let (m, phase) = if (0..n).contains(&index) {
                (index as usize, c64::new(1.0, 0.0))
            } else {
                match coarse.boundaries.edges(axis) {
                    Edges::Bloch { k } => {
                        let turns = if index < 0 { -1.0 } else { 1.0 };
                        let period = n as f64 * gc.step(axis);
                        (
                            index.rem_euclid(n) as usize,
                            c64::from_polar(1.0, turns * k * period),
                        )
                    }
                    Edges::Pml { .. } => continue,
                }
            };
            along[a].push((m, weight * phase));
        }
    }
    let mut out = Vec::with_capacity(8);
    for &(i, wx) in &along[0] {
        for &(j, wy) in &along[1] {
            for &(k, wz) in &along[2] {
                let col = gc.index(component, (i, j, k));
                if !coarse.fixed(col) {
                    out.push((col, wx * wy * wz));
                }
            }
        }
    }
    out
}

/// Sorts `row` by column and sums repeated columns.
fn merge(row: &mut Vec<(usize, c64)>) {
    row.sort_unstable_by_key(|&(c, _)| c);
    let mut out: Vec<(usize, c64)> = Vec::with_capacity(row.len());
    for &(c, v) in row.iter() {
        match out.last_mut() {
            Some((last, acc)) if *last == c => *acc += v,
            _ => out.push((c, v)),
        }
    }
    *row = out;
}

/// Pᵀ A P, coarse row by coarse row, the rows shared among threads; each row is summed in the
/// same order whatever the threads. A coarse value no fine one reaches gets an identity row.
fn galerkin(a: &Sparse, p: &[Vec<(usize, c64)>], coarse: usize) -> Sparse {
    // Pᵀ by rows: for each coarse value, the fine values it reaches
    let mut pt: Vec<Vec<(usize, c64)>> = vec![Vec::new(); coarse];
    for (r, row) in p.iter().enumerate() {
        for &(c, w) in row {
            pt[c].push((r, w));
        }
    }
    let row_of = |c: usize| -> Vec<(usize, c64)> {
        let mut acc: Vec<(usize, c64)> = Vec::new();
        for &(r, w) in &pt[c] {
            for (s, v) in a.row(r) {
                for &(cc, u) in &p[s] {
                    acc.push((cc, w * v * u));
                }
            }
        }
        merge(&mut acc);
        if acc.iter().all(|&(cc, _)| cc != c) {
            acc.push((c, c64::new(1.0, 0.0)));
            merge(&mut acc);
        }
        acc
    };
    let threads = std::thread::available_parallelism().map_or(1, |t| t.get());
    let chunk = coarse.div_ceil(threads).max(1);
    let rows: Vec<Vec<(usize, c64)>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..coarse)
            .step_by(chunk)
            .map(|start| {
                let row_of = &row_of;
                scope.spawn(move || {
                    (start..(start + chunk).min(coarse))
                        .map(row_of)
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    });
    Sparse::new(
        coarse,
        rows.into_iter()
            .enumerate()
            .flat_map(|(c, row)| row.into_iter().map(move |(cc, v)| (c, cc, v))),
    )
}

impl Hierarchy {
    /// The hierarchy for the fine operator `matrix` on `lattice`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if no steps of smoothing are asked for, or if a level's ILU(0) or
    /// the coarsest factorization fails.
    pub(crate) fn new(lattice: &Lattice, matrix: Sparse, options: Multigrid) -> Result<Hierarchy> {
        if options.pre + options.post == 0 {
            return Err(Error::invalid("multigrid", "needs some smoothing"));
        }
        let mut levels = Vec::new();
        let mut fine = lattice.clone();
        let mut a = matrix;
        loop {
            let n = a.size();
            if n <= options.coarsest {
                let entries: Vec<_> = (0..n)
                    .flat_map(|r| {
                        a.row(r)
                            .map(move |(c, v)| faer::sparse::Triplet::new(r, c, v))
                            .collect::<Vec<_>>()
                    })
                    .collect();
                let (lu, _) = crate::compact::fit::sequential(|| {
                    crate::fdfd::factorize(&entries, n, None)
                })?;
                return Ok(Hierarchy {
                    levels,
                    lu,
                    options,
                });
            }
            let (gc, bc) = coarser(&fine.grid, &fine.boundaries);
            let coarse = Lattice::new(gc, bc, fine.k0);
            let prolongation: Vec<Vec<(usize, c64)>> = (0..n)
                .map(|r| {
                    if fine.fixed(r) {
                        Vec::new()
                    } else {
                        interpolation(&fine, &coarse, r)
                    }
                })
                .collect();
            let next = galerkin(&a, &prolongation, gc.unknowns());
            let smoother = Ilu0::new(&a)?;
            levels.push(Level {
                matrix: a,
                smoother,
                prolongation,
                coarse: gc.unknowns(),
            });
            a = next;
            fine = coarse;
        }
    }

    /// The number of levels, the coarsest included.
    pub(crate) fn depth(&self) -> usize {
        self.levels.len() + 1
    }

    /// One cycle from `level` down, for A x = b (or Aᵀ x = b, the transposed cycle).
    fn cycle(&self, level: usize, b: &[c64], transpose: bool, shape: CycleShape) -> Vec<c64> {
        use faer::linalg::solvers::Solve;
        if level == self.levels.len() {
            let rhs = faer::Mat::<c64>::from_fn(b.len(), 1, |r, _| b[r]);
            let x = crate::compact::fit::sequential(|| {
                if transpose {
                    self.lu.solve_transpose(&rhs)
                } else {
                    self.lu.solve(&rhs)
                }
            });
            return (0..b.len()).map(|r| x[(r, 0)]).collect();
        }
        let l = &self.levels[level];
        let apply = |v: &[c64]| {
            if transpose {
                l.matrix.apply_transpose(v)
            } else {
                l.matrix.apply(v)
            }
        };
        let smooth = |x: &mut Vec<c64>| {
            let ax = apply(x);
            let r: Vec<c64> = b.iter().zip(&ax).map(|(p, q)| p - q).collect();
            let d = if transpose {
                l.smoother.solve_transpose(&r)
            } else {
                l.smoother.solve(&r)
            };
            for (xk, dk) in x.iter_mut().zip(&d) {
                *xk += dk;
            }
        };
        // the transposed cycle runs the steps in reverse: post-smoothing first
        let (pre, post) = if transpose {
            (self.options.post, self.options.pre)
        } else {
            (self.options.pre, self.options.post)
        };
        let mut x = vec![c64::new(0.0, 0.0); b.len()];
        for _ in 0..pre {
            smooth(&mut x);
        }
        let mut corrections = match shape {
            CycleShape::V => vec![CycleShape::V],
            CycleShape::W => vec![CycleShape::W, CycleShape::W],
            CycleShape::F => vec![CycleShape::F, CycleShape::V],
        };
        if transpose {
            corrections.reverse();
        }
        for inner in corrections {
            let ax = apply(&x);
            let mut coarse = vec![c64::new(0.0, 0.0); l.coarse];
            for (r, weights) in l.prolongation.iter().enumerate() {
                let residual = b[r] - ax[r];
                for &(c, w) in weights {
                    coarse[c] += w * residual;
                }
            }
            let correction = self.cycle(level + 1, &coarse, transpose, inner);
            for (r, weights) in l.prolongation.iter().enumerate() {
                for &(c, w) in weights {
                    x[r] += w * correction[c];
                }
            }
        }
        for _ in 0..post {
            smooth(&mut x);
        }
        x
    }
}

impl Preconditioner for Hierarchy {
    fn solve(&self, v: &[c64]) -> Vec<c64> {
        self.cycle(0, v, false, self.options.shape)
    }

    fn solve_transpose(&self, v: &[c64]) -> Vec<c64> {
        self.cycle(0, v, true, self.options.shape)
    }
}
