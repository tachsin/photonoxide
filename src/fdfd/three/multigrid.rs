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

use super::{Axis, Grid3d, Lattice};
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
    /// Each level's cells along x, y and z, the coarsest's included.
    shapes: Vec<[usize; 3]>,
    lu: Lu<usize, c64>,
    options: Multigrid,
}

/// The operator the cycle is built on: `matrix` with k₀² i β ε added on the diagonal of every
/// value not fixed at zero, Erlangga, Oosterlee and Vuik's shift (their Eq. 8 with
/// (β₁, β₂) = (1, β), in our e^(−iωt) on the side of the medium's loss and the PMLs').
pub(crate) fn shifted(lattice: &Lattice, eps: &[c64], matrix: &Sparse, shift: f64) -> Sparse {
    let k2 = lattice.k0 * lattice.k0;
    let add = c64::new(0.0, shift * k2);
    Sparse::new(
        matrix.size(),
        (0..matrix.size())
            .flat_map(|r| {
                matrix
                    .row(r)
                    .map(move |(c, v)| (r, c, v))
                    .collect::<Vec<_>>()
            })
            .chain(
                (0..eps.len())
                    .filter(|&r| shift != 0.0 && !lattice.fixed(r))
                    .map(|r| (r, r, add * eps[r])),
            ),
    )
}

/// A level's nodes along one axis: where each lies, in half-steps of the finest grid, the far
/// end (a wall, or the next period's first node) included. Its values of E lie at the nodes
/// (the components across the axis) or halfway between them (the component along it).
#[derive(Clone, Debug)]
struct Line {
    ends: Vec<usize>,
}

impl Line {
    /// The finest grid's `n` cells.
    fn finest(n: usize) -> Line {
        Line {
            ends: (0..=n).map(|m| 2 * m).collect(),
        }
    }

    /// The cells.
    fn cells(&self) -> usize {
        self.ends.len() - 1
    }

    /// The next level's: neighbouring cells merged in pairs where the pair is no longer, in
    /// the stretched coordinate `x` (in steps of the finest grid), than `target`: the next
    /// level's cells outside the PMLs. Inside a PML a cell is |s| times longer in that
    /// coordinate, so it waits for the levels around it to catch up. A line of fewer than 4
    /// cells isn't coarsened.
    fn coarser(&self, x: &[c64], target: f64) -> Line {
        let n = self.cells();
        if n < 4 {
            return self.clone();
        }
        let mut ends = vec![self.ends[0]];
        let mut m = 0;
        while m < n {
            if m + 2 <= n && (x[self.ends[m + 2]] - x[self.ends[m]]).norm() <= target {
                m += 2;
            } else {
                m += 1;
            }
            ends.push(self.ends[m]);
        }
        Line { ends }
    }

    /// For each of `coarse`'s nodes, the far end included, the index of the same node here.
    fn nodes_of(&self, coarse: &Line) -> Vec<usize> {
        coarse
            .ends
            .iter()
            .map(|e| self.ends.partition_point(|f| f < e))
            .collect()
    }

    /// Where value `m` lies: node m, or halfway between nodes m and m + 1 along the axis.
    fn place(&self, m: usize, along: bool) -> usize {
        if along {
            (self.ends[m] + self.ends[m + 1]) / 2
        } else {
            self.ends[m]
        }
    }
}

/// How much longer, in the stretched coordinate, a coarse cell may be than the level's cells
/// outside the PMLs: a PML's cell is |s| times longer than outside, and its neighbours couple
/// |s|² times more weakly along its axis than across it, an anisotropy no point smoother
/// reduces (U. Trottenberg, C. W. Oosterlee, A. Schüller, Multigrid, Academic Press (2001),
/// Section 5.1: semicoarsening). So a PML is coarsened along its axis only once the cells
/// across it are as long.
const ANISOTROPY: f64 = 1.5;

/// The complex coordinate x̃ = ∫ s dx of each axis on the finest grid, at each of its
/// half-steps, in units of the step: the coordinate in which the PML's waves are as smooth as
/// outside it, and the cycle interpolates. Two of them: the stretch of the cells (s at their
/// centres), which the components across the axis see between neighbours, and the stretch of
/// the nodes (s at the nodes), which the component along it sees between neighbours.
struct Stretched {
    across: [Vec<c64>; 3],
    along: [Vec<c64>; 3],
}

impl Stretched {
    fn new(lattice: &Lattice) -> Stretched {
        let map = |a: usize, along: bool| -> Vec<c64> {
            let n = lattice.grid.n(Axis::ALL[a]);
            let mut x = vec![c64::new(0.0, 0.0); 2 * n + 1];
            for p in 0..2 * n {
                let s = if along {
                    lattice.nodes[a][((p + 1) / 2).min(n - 1)]
                } else {
                    lattice.halves[a][p / 2]
                };
                x[p + 1] = x[p] + 0.5 * s;
            }
            x
        };
        Stretched {
            across: [0, 1, 2].map(|a| map(a, false)),
            along: [0, 1, 2].map(|a| map(a, true)),
        }
    }
}

/// Linear interpolation along one axis from the coarse line's values to the fine one's, in the
/// stretched coordinate `x`: for each fine value, the coarse values and their weights. `along`
/// for the component along the axis (between the nodes), else across it (at the nodes);
/// `phase` for a Bloch-periodic axis, e^(ikL) one period on, `None` for walls.
fn interpolate_line(
    fine: &Line,
    coarse: &Line,
    x: &[c64],
    along: bool,
    phase: Option<c64>,
) -> Vec<Vec<(usize, c64)>> {
    let one = c64::new(1.0, 0.0);
    let nf = fine.cells();
    let nc = coarse.cells();
    let period = *coarse.ends.last().unwrap_or(&0);
    // the coarse values' places, with the one before the first and after the last: across a
    // period's end (Bloch) or beyond a wall
    let place = |m: i64| -> i64 {
        if m < 0 {
            coarse.place(nc - 1, along) as i64 - period as i64
        } else if m as usize >= nc {
            coarse.place(0, along) as i64 + period as i64
        } else {
            coarse.place(m as usize, along) as i64
        }
    };
    // the stretched coordinate at a place (a Bloch axis has no stretch: there it is the place)
    let at = |p: i64| -> c64 {
        if phase.is_some() || p < 0 || p as usize >= x.len() {
            c64::new(p as f64, 0.0)
        } else {
            x[p as usize]
        }
    };
    (0..nf)
        .map(|i| {
            let p = fine.place(i, along) as i64;
            // the coarse value at or before p: m in −1..nc
            let mut m = -1i64;
            while m + 1 < nc as i64 && place(m + 1) <= p {
                m += 1;
            }
            if m >= 0 && place(m) == p {
                return vec![(m as usize, one)];
            }
            let (low, high) = (m, m + 1);
            // past the first or last value: across a period's end, or beyond a wall, where
            // the tangential E is zero (odd about it) and the normal E's derivative is
            // (∇ · E = 0 with the tangential E zero), so it extends evenly
            if phase.is_none() && along && (low < 0 || high >= nc as i64) {
                return vec![(low.clamp(0, nc as i64 - 1) as usize, one)];
            }
            let index = |m: i64| -> Option<(usize, c64)> {
                if (0..nc as i64).contains(&m) {
                    return Some((m as usize, one));
                }
                match phase {
                    Some(ph) if m < 0 => Some((nc - 1, ph.inv())),
                    Some(ph) => Some((0, ph)),
                    None => None,
                }
            };
            let t = (at(p) - at(place(low))) / (at(place(high)) - at(place(low)));
            let mut out = Vec::with_capacity(2);
            if let Some((c, ph)) = index(low) {
                out.push((c, (1.0 - t) * ph));
            }
            if let Some((c, ph)) = index(high) {
                out.push((c, t * ph));
            }
            out
        })
        .collect()
}

/// The permittivity each coarse value of E stands for: along its component, the harmonic
/// mean of the fine values in its coarse cell (as each value's own average is harmonic along
/// it), across it the fine values at the same nodes. `nodes` says which fine node each coarse
/// node is, axis by axis.
fn coarse_eps(gf: &Grid3d, gc: &Grid3d, eps: &[c64], nodes: &[Vec<usize>; 3]) -> Vec<c64> {
    (0..gc.unknowns())
        .map(|r| {
            let (component, at) = gc.at(r);
            let a = component.index();
            let mut fine = [0; 3];
            for (b, f) in fine.iter_mut().enumerate() {
                *f = nodes[b][at[b]];
            }
            let cells = nodes[a][at[a]]..nodes[a][at[a] + 1];
            let count = cells.len() as f64;
            let inverse: c64 = cells
                .map(|m| {
                    fine[a] = m;
                    1.0 / eps[gf.index(component, (fine[0], fine[1], fine[2]))]
                })
                .sum();
            count / inverse
        })
        .collect()
}

/// The prolongation from the coarse lattice to the fine one, by rows: each component of E
/// interpolated on its own staggered points, linearly along each axis in the stretched
/// coordinate (trilinearly), values fixed at zero left out. Along its own axis a component is
/// interpolated as ε E, the flux density, whose normal component is continuous across an
/// interface where E's jumps; across it, as E, whose tangential components are continuous.
fn prolongation(
    fine: (&Lattice, &[c64]),
    coarse: (&Lattice, &[c64]),
    lines: (&[Line; 3], &[Line; 3]),
    stretched: &Stretched,
) -> Vec<Vec<(usize, c64)>> {
    let ((fine, eps_f), (coarse, eps_c)) = (fine, coarse);
    let (gf, gc) = (fine.grid, coarse.grid);
    let nodes = [0, 1, 2].map(|a| lines.0[a].nodes_of(&lines.1[a]));
    // for each component and axis, the 1D weights
    let tables: Vec<[Vec<Vec<(usize, c64)>>; 3]> = Axis::ALL
        .iter()
        .map(|&component| {
            Axis::ALL.map(|axis| {
                let a = axis.index();
                let phase = match fine.boundaries.edges(axis) {
                    Edges::Bloch { k } => {
                        Some(c64::from_polar(1.0, k * gf.n(axis) as f64 * gf.step(axis)))
                    }
                    Edges::Pml { .. } => None,
                };
                let along = axis == component;
                let x = if along {
                    &stretched.along[a]
                } else {
                    &stretched.across[a]
                };
                interpolate_line(&lines.0[a], &lines.1[a], x, along, phase)
            })
        })
        .collect();
    #[cfg(test)]
    let flux = std::env::var("MG_PLAIN_INTERP").is_err();
    #[cfg(not(test))]
    let flux = true;
    (0..gf.unknowns())
        .map(|r| {
            if fine.fixed(r) {
                return Vec::new();
            }
            let (component, at) = gf.at(r);
            let a = component.index();
            let t = &tables[a];
            let mut out = Vec::with_capacity(8);
            for &(i, wx) in &t[0][at[0]] {
                for &(j, wy) in &t[1][at[1]] {
                    for &(k, wz) in &t[2][at[2]] {
                        let col = gc.index(component, (i, j, k));
                        if coarse.fixed(col) {
                            continue;
                        }
                        let mut w = wx * wy * wz;
                        if flux {
                            // ε E interpolated along the component, on the coarse value's line
                            let c = [i, j, k];
                            let mut f = [0; 3];
                            for b in 0..3 {
                                f[b] = if b == a { at[a] } else { nodes[b][c[b]] };
                            }
                            w *= eps_c[col] / eps_f[gf.index(component, (f[0], f[1], f[2]))];
                        }
                        out.push((col, w));
                    }
                }
            }
            out
        })
        .collect()
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
    pub(crate) fn new(
        lattice: &Lattice,
        eps: &[c64],
        matrix: Sparse,
        options: Multigrid,
    ) -> Result<Hierarchy> {
        if options.pre + options.post == 0 {
            return Err(Error::invalid("multigrid", "needs some smoothing"));
        }
        let mut levels = Vec::new();
        let mut fine = lattice.clone();
        let g = lattice.grid;
        let mut lines = [Line::finest(g.nx), Line::finest(g.ny), Line::finest(g.nz)];
        let stretched = Stretched::new(lattice);
        let mut a = matrix;
        let mut eps = eps.to_vec();
        let mut shapes = Vec::new();
        loop {
            let n = a.size();
            #[cfg(test)]
            let anisotropy = std::env::var("MG_ANISOTROPY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(ANISOTROPY);
            #[cfg(not(test))]
            let anisotropy = ANISOTROPY;
            let target = (1 << (levels.len() + 1)) as f64 * anisotropy;
            let next_lines = [0, 1, 2].map(|a| lines[a].coarser(&stretched.across[a], target));
            let cells = |l: &[Line; 3]| l.iter().map(Line::cells).product::<usize>();
            if n <= options.coarsest || cells(&next_lines) == cells(&lines) {
                let entries: Vec<_> = (0..n)
                    .flat_map(|r| {
                        a.row(r)
                            .map(move |(c, v)| faer::sparse::Triplet::new(r, c, v))
                            .collect::<Vec<_>>()
                    })
                    .collect();
                let (lu, _) =
                    crate::compact::fit::sequential(|| crate::fdfd::factorize(&entries, n, None))?;
                shapes.push([0, 1, 2].map(|a| lines[a].cells()));
                return Ok(Hierarchy {
                    levels,
                    shapes,
                    lu,
                    options,
                });
            }
            let gc = Grid3d {
                nx: next_lines[0].cells(),
                ny: next_lines[1].cells(),
                nz: next_lines[2].cells(),
                ..fine.grid
            };
            let coarse = Lattice::new(gc, fine.boundaries, fine.k0);
            let nodes = [0, 1, 2].map(|a| lines[a].nodes_of(&next_lines[a]));
            let eps_c = coarse_eps(&fine.grid, &gc, &eps, &nodes);
            let prolongation = prolongation(
                (&fine, &eps),
                (&coarse, &eps_c),
                (&lines, &next_lines),
                &stretched,
            );
            let next = galerkin(&a, &prolongation, gc.unknowns());
            let smoother = Ilu0::new(&a)?;
            shapes.push([0, 1, 2].map(|a| lines[a].cells()));
            levels.push(Level {
                matrix: a,
                smoother,
                prolongation,
                coarse: gc.unknowns(),
            });
            a = next;
            fine = coarse;
            lines = next_lines;
            eps = eps_c;
        }
    }

    /// Each level's cells along x, y and z, finest first.
    pub(crate) fn shapes(&self) -> &[[usize; 3]] {
        &self.shapes
    }

    /// The number of levels, the coarsest included.
    pub(crate) fn depth(&self) -> usize {
        self.levels.len() + 1
    }

    /// One cycle from `level` down, for A x = b (or Aᵀ x = b, the transposed cycle), from
    /// x = 0.
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
        // b − A x, or b itself while x is still zero
        let residual = |x: Option<&[c64]>| -> Vec<c64> {
            match x {
                None => b.to_vec(),
                Some(x) => {
                    let ax = if transpose {
                        l.matrix.apply_transpose(x)
                    } else {
                        l.matrix.apply(x)
                    };
                    b.iter().zip(&ax).map(|(p, q)| p - q).collect()
                }
            }
        };
        let smooth = |x: &mut Option<Vec<c64>>| {
            let r = residual(x.as_deref());
            let d = if transpose {
                l.smoother.solve_transpose(&r)
            } else {
                l.smoother.solve(&r)
            };
            match x {
                None => *x = Some(d),
                Some(x) => {
                    for (xk, dk) in x.iter_mut().zip(&d) {
                        *xk += dk;
                    }
                }
            }
        };
        // the transposed cycle is the cycle's transpose: its steps in reverse order, each
        // transposed (post-smoothing first, the coarse corrections reversed)
        let (pre, post) = if transpose {
            (self.options.post, self.options.pre)
        } else {
            (self.options.pre, self.options.post)
        };
        let mut x: Option<Vec<c64>> = None;
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
            let r = residual(x.as_deref());
            let coarse = l.restrict(&r);
            let correction = self.cycle(level + 1, &coarse, transpose, inner);
            let fine = l.prolong(&correction);
            match &mut x {
                None => x = Some(fine),
                Some(x) => {
                    for (xk, dk) in x.iter_mut().zip(&fine) {
                        *xk += dk;
                    }
                }
            }
        }
        for _ in 0..post {
            smooth(&mut x);
        }
        x.unwrap_or_else(|| vec![c64::new(0.0, 0.0); b.len()])
    }
}

impl Level {
    /// Pᵀ r: the residual restricted to the next level.
    fn restrict(&self, r: &[c64]) -> Vec<c64> {
        let mut coarse = vec![c64::new(0.0, 0.0); self.coarse];
        for (rk, weights) in r.iter().zip(&self.prolongation) {
            for &(c, w) in weights {
                coarse[c] += w * rk;
            }
        }
        coarse
    }

    /// P e: the next level's correction interpolated to this one.
    fn prolong(&self, e: &[c64]) -> Vec<c64> {
        self.prolongation
            .iter()
            .map(|weights| weights.iter().map(|&(c, w)| w * e[c]).sum())
            .collect()
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

#[cfg(test)]
#[path = "multigrid_tests.rs"]
mod tests;
