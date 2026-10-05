//! FDFD in 3D: the electric field's curl-curl equation on Yee's grid, as one sparse system.
//!
//! A. Christ, H. L. Hartnagel, IEEE Trans. Microw. Theory Tech. 35, 688 (1987),
//! doi:10.1109/TMTT.1987.1133733, Section III: Maxwell's equations in integral form (their
//! Eqs. 6a–6b) on Yee's cell (K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966),
//! doi:10.1109/TAP.1966.1138693, Fig. 1; Christ and Hartnagel's Fig. 2), E at the middle of the
//! cells' edges and H at the centres of their faces. H, expressed by the E around each face, is
//! eliminated, which leaves every E component tied to itself and 12 neighbours (their Eq. 7):
//! one sparse system M e = b for E alone (their Eq. 9a). With e^(−iωt) and H̃ = η₀ H,
//!
//! ∇ × E = i k₀ H̃, ∇ × H̃ = −i k₀ ε E + J, so −∇ × ∇ × E + k₀² ε E = −i k₀ J,
//!
//! J = η₀ J the electric current; Christ and Hartnagel's e^(+jωt) makes their j our −i. A magnetic
//! current M (∇ × E = i k₀ H̃ − M) adds ∇ × M to the right-hand side.
//!
//! Open boundaries are the same stretched-coordinate PMLs as in 2D (Chew and Weedon's ∂_w →
//! s_w⁻¹ ∂_w, graded as W. Shin, S. Fan, J. Comput. Phys. 231, 3406 (2012),
//! doi:10.1016/j.jcp.2012.01.013, Eqs. 2.5–2.9 describe), backed by a perfectly conducting wall;
//! each axis can instead be Bloch-periodic. Each component of E sees the permittivity averaged
//! over its own cell of the dual grid, harmonically along the component and arithmetically
//! across it, as in 2D.

use crate::sparse::{OrderedLu, OrderedSymbolic, adjacency, nested_dissection};
use faer::sparse::Triplet;
use num_complex::Complex64 as c64;

use super::{Direction, Edges, graded};
use crate::units::Wavelength;
use crate::{Error, Result};

/// An axis of a 3D grid, and the component of a field along it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    /// x.
    X,
    /// y.
    Y,
    /// z.
    Z,
}

impl Axis {
    const ALL: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    fn index(self) -> usize {
        self as usize
    }

    /// The next two axes in cyclic order: (y, z) for x, (z, x) for y, (x, y) for z.
    fn others(self) -> (Axis, Axis) {
        match self {
            Axis::X => (Axis::Y, Axis::Z),
            Axis::Y => (Axis::Z, Axis::X),
            Axis::Z => (Axis::X, Axis::Y),
        }
    }
}

/// A uniform grid of `nx` × `ny` × `nz` cells of `dx` × `dy` × `dz` µm, its lowest corner at
/// (`x0`, `y0`, `z0`).
///
/// The cells' corners are the grid's nodes, x0 + i dx and so on. E's components sit at the middle
/// of the cells' edges and H's at the centres of their faces (Yee's Fig. 1): E_x of index
/// (i, j, k) at (x0 + (i + ½) dx, y0 + j dy, z0 + k dz), half a step along its own axis, and
/// H_x at (x0 + i dx, y0 + (j + ½) dy, z0 + (k + ½) dz), half a step along the other two; the
/// same for y and z. Each component has one value per cell, `nx` × `ny` × `nz`, i fastest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid3d {
    /// Cells along x.
    pub nx: usize,
    /// Cells along y.
    pub ny: usize,
    /// Cells along z.
    pub nz: usize,
    /// Cell width along x, µm.
    pub dx: f64,
    /// Along y, µm.
    pub dy: f64,
    /// Along z, µm.
    pub dz: f64,
    /// The grid's low x, µm.
    pub x0: f64,
    /// Its low y, µm.
    pub y0: f64,
    /// Its low z, µm.
    pub z0: f64,
}

impl Grid3d {
    /// The number of cells.
    pub fn cells(&self) -> usize {
        self.nx * self.ny * self.nz
    }

    /// The number of unknowns, three per cell: the size of the linear system.
    pub fn unknowns(&self) -> usize {
        3 * self.cells()
    }

    /// The cells along `axis`.
    pub fn n(&self, axis: Axis) -> usize {
        [self.nx, self.ny, self.nz][axis.index()]
    }

    /// The step along `axis`, µm.
    pub fn step(&self, axis: Axis) -> f64 {
        [self.dx, self.dy, self.dz][axis.index()]
    }

    fn origin(&self, axis: Axis) -> f64 {
        [self.x0, self.y0, self.z0][axis.index()]
    }

    /// Node `m` along `axis`, µm: the low face of the cells of index `m`.
    pub fn node(&self, axis: Axis, m: usize) -> f64 {
        self.origin(axis) + m as f64 * self.step(axis)
    }

    /// The index of `component`'s value (i, j, k) in a field's values: component by component
    /// (x, y, z), then k, j, i, with i fastest.
    pub fn index(&self, component: Axis, (i, j, k): (usize, usize, usize)) -> usize {
        component.index() * self.cells() + (k * self.ny + j) * self.nx + i
    }

    /// The component and (i, j, k) of a field's value `r`.
    fn at(&self, r: usize) -> (Axis, [usize; 3]) {
        let n = self.cells();
        let (c, rest) = (r / n, r % n);
        let i = rest % self.nx;
        let j = (rest / self.nx) % self.ny;
        let k = rest / (self.nx * self.ny);
        (Axis::ALL[c], [i, j, k])
    }

    /// Where E's `component` of index (i, j, k) sits, µm.
    pub fn e_position(&self, component: Axis, (i, j, k): (usize, usize, usize)) -> [f64; 3] {
        let mut p = [
            self.node(Axis::X, i),
            self.node(Axis::Y, j),
            self.node(Axis::Z, k),
        ];
        p[component.index()] += 0.5 * self.step(component);
        p
    }

    /// Where H's `component` of index (i, j, k) sits, µm.
    pub fn h_position(&self, component: Axis, (i, j, k): (usize, usize, usize)) -> [f64; 3] {
        let mut p = [
            self.node(Axis::X, i) + 0.5 * self.dx,
            self.node(Axis::Y, j) + 0.5 * self.dy,
            self.node(Axis::Z, k) + 0.5 * self.dz,
        ];
        p[component.index()] -= 0.5 * self.step(component);
        p
    }

    fn check(&self) -> Result<()> {
        let ok = |v: f64| v.is_finite() && v > 0.0;
        if self.cells() == 0 || !ok(self.dx) || !ok(self.dy) || !ok(self.dz) {
            return Err(Error::invalid(
                "fdfd grid",
                format!(
                    "needs cells and positive steps, got {} x {} x {} cells of {} x {} x {} um",
                    self.nx, self.ny, self.nz, self.dx, self.dy, self.dz
                ),
            ));
        }
        if !(self.x0.is_finite() && self.y0.is_finite() && self.z0.is_finite()) {
            return Err(Error::invalid("fdfd grid", "its corner must be finite"));
        }
        Ok(())
    }
}

/// A 3D grid's boundaries, and the PMLs' grading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Boundaries3d {
    /// Along x.
    pub x: Edges,
    /// Along y.
    pub y: Edges,
    /// Along z.
    pub z: Edges,
    /// The PMLs' target reflection at normal incidence, R in (0, 1).
    pub reflection: f64,
    /// The polynomial grading's order m.
    pub order: f64,
    /// The PMLs' real stretching per unit of their absorption, a: s = 1 + (a + i)σ instead of
    /// 1 + iσ. Any s with Im s > 0 is a PML (Chew and Weedon), so the solution outside them
    /// is the same; a = 0 is Shin and Fan's, and a ≥ 1 keeps s within 45° of the real axis,
    /// where Re(1/s²) > 0 and the operator in the PMLs doesn't turn the wrong way along their
    /// normal: what QMR with [`IterativeSolver3d::with_ilu`] needs (see docs/methods/fdfd-3d.md).
    pub real_stretch: f64,
}

impl Boundaries3d {
    /// PMLs `cells` thick on all six sides, graded to R = 1e-8 with m = 3.
    pub fn pml(cells: usize) -> Boundaries3d {
        let edges = Edges::Pml {
            low: cells,
            high: cells,
        };
        Boundaries3d {
            x: edges,
            y: edges,
            z: edges,
            reflection: 1e-8,
            order: 3.0,
            real_stretch: 0.0,
        }
    }

    /// [`Boundaries3d::pml`] with as much real stretching as absorption (a = 1): the PMLs for
    /// QMR with [`IterativeSolver3d::with_ilu`].
    pub fn stretched_pml(cells: usize) -> Boundaries3d {
        Boundaries3d {
            real_stretch: 1.0,
            ..Boundaries3d::pml(cells)
        }
    }

    fn edges(&self, axis: Axis) -> Edges {
        [self.x, self.y, self.z][axis.index()]
    }

    fn check(&self, grid: &Grid3d) -> Result<()> {
        if !(self.reflection > 0.0 && self.reflection < 1.0)
            || !(self.order.is_finite() && self.order >= 0.0)
            || !(self.real_stretch.is_finite() && self.real_stretch >= 0.0)
        {
            return Err(Error::invalid(
                "fdfd boundaries",
                format!(
                    "the PML needs a target reflection in (0, 1), an order >= 0 and a real stretch \n                     >= 0, got {}, {} and {}",
                    self.reflection, self.order, self.real_stretch
                ),
            ));
        }
        for axis in Axis::ALL {
            let n = grid.n(axis);
            match self.edges(axis) {
                Edges::Pml { low, high } if low + high >= n => {
                    return Err(Error::invalid(
                        "fdfd boundaries",
                        format!("PMLs of {low} and {high} cells leave nothing of {n} cells"),
                    ));
                }
                Edges::Bloch { k } if !k.is_finite() => {
                    return Err(Error::invalid(
                        "fdfd boundaries",
                        "the Bloch wavenumber must be finite",
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// The grid with its boundaries at one wavelength: the discrete curls, and which values of E
/// are fixed at zero.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Lattice {
    pub(crate) grid: Grid3d,
    pub(crate) boundaries: Boundaries3d,
    pub(crate) k0: f64,
    /// The PML stretch at the nodes of each axis, and halfway after each node.
    nodes: [Vec<c64>; 3],
    halves: [Vec<c64>; 3],
}

impl Lattice {
    pub(crate) fn new(grid: Grid3d, boundaries: Boundaries3d, k0: f64) -> Lattice {
        let along = |axis: Axis, offset: f64| -> Vec<c64> {
            let n = grid.n(axis);
            let h = grid.step(axis);
            let span = (grid.node(axis, 0), grid.node(axis, n));
            let layers = match boundaries.edges(axis) {
                Edges::Pml { low, high } => (low, high),
                Edges::Bloch { .. } => (0, 0),
            };
            (0..n)
                .map(|m| {
                    let pos = grid.node(axis, m) + offset * h;
                    let s = graded(
                        pos,
                        span,
                        layers,
                        h,
                        k0,
                        (boundaries.reflection, boundaries.order),
                    );
                    c64::new(s.re + boundaries.real_stretch * s.im, s.im)
                })
                .collect()
        };
        Lattice {
            grid,
            boundaries,
            k0,
            nodes: Axis::ALL.map(|a| along(a, 0.0)),
            halves: Axis::ALL.map(|a| along(a, 0.5)),
        }
    }

    /// The index `m + step` along `axis` (step −1, 0 or +1) and the field's phase there, across
    /// a Bloch-periodic end; `None` beyond a wall, where the field is zero.
    fn shift(&self, axis: Axis, m: usize, step: i64) -> Option<(usize, c64)> {
        let n = self.grid.n(axis) as i64;
        let to = m as i64 + step;
        if (0..n).contains(&to) {
            return Some((to as usize, c64::new(1.0, 0.0)));
        }
        match self.boundaries.edges(axis) {
            Edges::Bloch { k } => {
                let turns = if to < 0 { -1.0 } else { 1.0 };
                let period = n as f64 * self.grid.step(axis);
                Some((
                    to.rem_euclid(n) as usize,
                    c64::from_polar(1.0, turns * k * period),
                ))
            }
            Edges::Pml { .. } => None,
        }
    }

    /// Whether E's value `r` is on a wall behind a PML, tangential to it: fixed at zero.
    pub(crate) fn fixed(&self, r: usize) -> bool {
        let (c, at) = self.grid.at(r);
        Axis::ALL.into_iter().any(|a| {
            a != c && at[a.index()] == 0 && matches!(self.boundaries.edges(a), Edges::Pml { .. })
        })
    }

    /// The index of E's or H's `component` at `at`, moved by `step` along `axis`, and the phase
    /// there: `None` beyond a wall.
    fn neighbour(
        &self,
        component: Axis,
        at: [usize; 3],
        axis: Axis,
        step: i64,
    ) -> Option<(usize, c64)> {
        let (m, phase) = self.shift(axis, at[axis.index()], step)?;
        let mut to = at;
        to[axis.index()] = m;
        Some((self.grid.index(component, (to[0], to[1], to[2])), phase))
    }

    /// ∇ × E at H's value `r` (component a at a face): ∂_b E_c − ∂_c E_b, (a, b, c) cyclic, each
    /// difference over the step and the stretch at the face. The values of E it takes and their
    /// coefficients; values fixed at zero included.
    pub(crate) fn curl_e(&self, r: usize) -> Vec<(usize, c64)> {
        let (a, at) = self.grid.at(r);
        let (b, c) = a.others();
        let mut out = Vec::with_capacity(4);
        for (along, field, sign) in [(b, c, 1.0), (c, b, -1.0)] {
            let w = sign / (self.halves[along.index()][at[along.index()]] * self.grid.step(along));
            if let Some((col, phase)) = self.neighbour(field, at, along, 1) {
                out.push((col, w * phase));
            }
            out.push((self.grid.index(field, (at[0], at[1], at[2])), -w));
        }
        out
    }

    /// ∇ × H at E's value `r` (component a on an edge): ∂_b H_c − ∂_c H_b, each difference over
    /// the step and the stretch at the edge. The values of H it takes and their coefficients.
    pub(crate) fn curl_h(&self, r: usize) -> Vec<(usize, c64)> {
        let (a, at) = self.grid.at(r);
        let (b, c) = a.others();
        let mut out = Vec::with_capacity(4);
        for (along, field, sign) in [(b, c, 1.0), (c, b, -1.0)] {
            let w = sign / (self.nodes[along.index()][at[along.index()]] * self.grid.step(along));
            out.push((self.grid.index(field, (at[0], at[1], at[2])), w));
            if let Some((col, phase)) = self.neighbour(field, at, along, -1) {
                out.push((col, -w * phase));
            }
        }
        out
    }

    /// The matrix A = −∇ × ∇ × + k₀² ε, row by row, with ε at each value of E; a value fixed at
    /// zero has a row and column of its own, with 1 on the diagonal.
    pub(crate) fn assemble(&self, eps: &[c64]) -> Vec<Triplet<usize, usize, c64>> {
        self.assemble_with(eps, 0.0)
    }

    /// The matrix A − s ∇(ε⁻¹ ∇ · (ε ·)): W. Shin and S. Fan's operator, Opt. Express 21, 22578
    /// (2013), doi:10.1364/OE.21.022578, Eq. 7, times −1 (their A is ∇ × ∇ × − k₀² ε), with ε⁻¹
    /// at the nodes, where the divergence lives, inside the gradient. s = 0 is A itself, and
    /// s = −1 their choice.
    pub(crate) fn assemble_with(&self, eps: &[c64], s: f64) -> Vec<Triplet<usize, usize, c64>> {
        let mut t = Vec::with_capacity(if s == 0.0 { 13 } else { 15 } * eps.len());
        let mut row: Vec<(usize, c64)> = Vec::with_capacity(32);
        for r in 0..eps.len() {
            self.row_into(eps, s, r, &mut row);
            merged(&mut t, r, &mut row);
        }
        t
    }

    /// Row `r` of −∇ × ∇ ×, appended to `row`: the curl-curl part of the matrix, the part that
    /// couples values to their neighbours. Nothing for a value fixed at zero.
    pub(crate) fn curl_curl_into(&self, r: usize, row: &mut Vec<(usize, c64)>) {
        if self.fixed(r) {
            return;
        }
        for (face, wh) in self.curl_h(r) {
            for (col, we) in self.curl_e(face) {
                if !self.fixed(col) {
                    row.push((col, -wh * we));
                }
            }
        }
    }

    /// Row `r` of [`Lattice::assemble_with`]'s matrix, its entries unsorted and possibly with
    /// repeated columns, into `row` (cleared first).
    pub(crate) fn row_into(&self, eps: &[c64], s: f64, r: usize, row: &mut Vec<(usize, c64)>) {
        row.clear();
        if self.fixed(r) {
            row.push((r, c64::new(1.0, 0.0)));
            return;
        }
        row.push((r, self.k0 * self.k0 * eps[r]));
        self.curl_curl_into(r, row);
        if s != 0.0 {
            for (node, wg) in self.gradient(r) {
                let inverse = 1.0 / self.node_eps(eps, node);
                for (col, wd) in self.divergence(node) {
                    if !self.fixed(col) {
                        row.push((col, -s * wg * wd * eps[col] * inverse));
                    }
                }
            }
        }
    }

    /// Whether node `q` (index (k ny + j) nx + i, at the cells' corner) is on a wall behind a
    /// PML, where the potential the gradient takes is zero.
    fn node_fixed(&self, q: usize) -> bool {
        let (_, at) = self.grid.at(q);
        Axis::ALL
            .into_iter()
            .any(|a| at[a.index()] == 0 && matches!(self.boundaries.edges(a), Edges::Pml { .. }))
    }

    /// ∇ · at node `q`: Σ_a (E_a after the node − E_a before it) over the step and the stretch
    /// at the node. The values of E it takes and their coefficients.
    pub(crate) fn divergence(&self, q: usize) -> Vec<(usize, c64)> {
        let (_, at) = self.grid.at(q);
        let mut out = Vec::with_capacity(6);
        for a in Axis::ALL {
            let w = 1.0 / (self.nodes[a.index()][at[a.index()]] * self.grid.step(a));
            out.push((self.grid.index(a, (at[0], at[1], at[2])), w));
            if let Some((col, phase)) = self.neighbour(a, at, a, -1) {
                out.push((col, -w * phase));
            }
        }
        out
    }

    /// ε at node `q`, for Shin and Fan's ε⁻¹ there: the mean of the edges its divergence takes.
    /// Their Eq. 7 needs it at the node, inside the gradient; taking ε⁻¹ at the edge instead
    /// (outside the gradient), the same in a uniform medium, made QMR 1.6 times slower than
    /// the curl-curl operator on a silicon guide in vacuum, where theirs is faster.
    fn node_eps(&self, eps: &[c64], q: usize) -> c64 {
        let (sum, count) = self
            .divergence(q)
            .into_iter()
            .filter(|&(col, _)| !self.fixed(col))
            .fold((c64::new(0.0, 0.0), 0.0), |(s, n), (col, _)| {
                (s + eps[col], n + 1.0)
            });
        if count > 0.0 {
            sum / count
        } else {
            c64::new(1.0, 0.0)
        }
    }

    /// ∇ at E's value `r` (component a, between two nodes along a): the difference of a
    /// potential at the nodes over the step and the stretch at the edge. The nodes it takes
    /// (indexed as [`Lattice::divergence`]'s) and their coefficients; nodes on a wall behind a
    /// PML left out.
    pub(crate) fn gradient(&self, r: usize) -> Vec<(usize, c64)> {
        let (a, at) = self.grid.at(r);
        let w = 1.0 / (self.halves[a.index()][at[a.index()]] * self.grid.step(a));
        let mut out = Vec::with_capacity(2);
        // node indices are component x's E indices
        if let Some((q, phase)) = self.neighbour(Axis::X, at, a, 1)
            && !self.node_fixed(q)
        {
            out.push((q, w * phase));
        }
        let here = self.grid.index(Axis::X, (at[0], at[1], at[2]));
        if !self.node_fixed(here) {
            out.push((here, -w));
        }
        out
    }

    /// The right-hand side of Shin and Fan's Eq. 7 for A x = `b`: b − (s / k₀²) ∇(ε⁻¹ ∇ · b),
    /// which leaves the solution as it is, since ∇ · (ε E) = ∇ · b / k₀² for the solution.
    pub(crate) fn transformed_rhs(&self, eps: &[c64], b: &[c64], s: f64) -> Vec<c64> {
        let k2 = self.k0 * self.k0;
        let cells = self.grid.cells();
        let div: Vec<c64> = (0..cells)
            .map(|q| {
                if self.node_fixed(q) {
                    c64::new(0.0, 0.0)
                } else {
                    self.divergence(q)
                        .into_iter()
                        .filter(|&(col, _)| !self.fixed(col))
                        .map(|(col, w)| w * b[col])
                        .sum()
                }
            })
            .collect();
        (0..b.len())
            .map(|r| {
                if self.fixed(r) || s == 0.0 {
                    return b[r];
                }
                let grad: c64 = self
                    .gradient(r)
                    .into_iter()
                    .map(|(q, w)| w * div[q] / self.node_eps(eps, q))
                    .sum();
                b[r] - s / k2 * grad
            })
            .collect()
    }
}

/// Row `r`'s entries `row`, sorted and with each column's entries summed, appended to `t`.
fn merged(t: &mut Vec<Triplet<usize, usize, c64>>, r: usize, row: &mut [(usize, c64)]) {
    row.sort_unstable_by_key(|&(col, _)| col);
    let mut last: Option<(usize, c64)> = None;
    for &(col, v) in row.iter() {
        match last {
            Some((c, ref mut acc)) if c == col => *acc += v,
            _ => {
                if let Some((c, acc)) = last {
                    t.push(Triplet::new(r, c, acc));
                }
                last = Some((col, v));
            }
        }
    }
    if let Some((c, acc)) = last {
        t.push(Triplet::new(r, c, acc));
    }
}

/// The permittivity of the cell of size `h` centred on `centre`, for a field component along
/// `along`, from `samples`³ points: harmonic along the component, arithmetic across it.
fn averaged(
    eps: &impl Fn(f64, f64, f64) -> c64,
    centre: [f64; 3],
    h: [f64; 3],
    along: Axis,
    samples: usize,
) -> c64 {
    let n = samples as f64;
    let offset = |s: usize| (s as f64 + 0.5) / n - 0.5;
    let (b, c) = along.others();
    let (a, b, c) = (along.index(), b.index(), c.index());
    let mut sum = c64::new(0.0, 0.0);
    let mut p = centre;
    for sb in 0..samples {
        p[b] = centre[b] + offset(sb) * h[b];
        for sc in 0..samples {
            p[c] = centre[c] + offset(sc) * h[c];
            let mut inverse = c64::new(0.0, 0.0);
            for sa in 0..samples {
                p[a] = centre[a] + offset(sa) * h[a];
                inverse += 1.0 / eps(p[0], p[1], p[2]);
            }
            sum += n / inverse;
        }
    }
    sum / (n * n)
}

/// A 3D FDFD problem, assembled and factorized: one structure at one wavelength, ready for any
/// number of sources.
pub struct Solver3d {
    lattice: Lattice,
    /// The permittivity each value of E sees, averaged over its cell of the dual grid.
    eps: Vec<c64>,
    /// The matrix's entries, for its products with fields.
    entries: Vec<Triplet<usize, usize, c64>>,
    /// The factors, the columns in nested-dissection order; their analysis depends only on the
    /// grid and the boundaries, and [`Solver3d::reuse`] reuses it.
    lu: OrderedLu,
}

impl Solver3d {
    /// Assembles and factorizes the problem at `wavelength` on `grid`, with relative
    /// permittivity `eps(x, y, z)` (µm), inside `boundaries`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an empty grid, PMLs that fill an axis, a target reflection
    /// outside (0, 1), a negative order or a non-finite Bloch wavenumber, or a system that can't
    /// be factorized.
    pub fn new(
        grid: Grid3d,
        wavelength: Wavelength,
        eps: impl Fn(f64, f64, f64) -> c64,
        boundaries: Boundaries3d,
    ) -> Result<Solver3d> {
        Self::build(grid, wavelength, eps, boundaries, None)
    }

    /// The same grid and boundaries at another `wavelength` or with another permittivity: the
    /// matrix's sparsity, which depends on neither, is analysed once and reused.
    ///
    /// # Errors
    ///
    /// As [`Solver3d::new`].
    pub fn reuse(
        &self,
        wavelength: Wavelength,
        eps: impl Fn(f64, f64, f64) -> c64,
    ) -> Result<Solver3d> {
        Self::build(
            self.lattice.grid,
            wavelength,
            eps,
            self.lattice.boundaries,
            Some(self.lu.symbolic().clone()),
        )
    }

    fn build(
        grid: Grid3d,
        wavelength: Wavelength,
        eps: impl Fn(f64, f64, f64) -> c64,
        boundaries: Boundaries3d,
        symbolic: Option<OrderedSymbolic>,
    ) -> Result<Solver3d> {
        let (lattice, eps) = Self::setup(grid, wavelength, eps, boundaries)?;
        let entries = lattice.assemble(&eps);
        let lu = factorize_ordered(&lattice.grid, &entries, symbolic)?;
        Ok(Solver3d {
            lattice,
            eps,
            entries,
            lu,
        })
    }

    /// The checked lattice and the averaged permittivity at each value of E.
    pub(crate) fn setup(
        grid: Grid3d,
        wavelength: Wavelength,
        eps: impl Fn(f64, f64, f64) -> c64,
        boundaries: Boundaries3d,
    ) -> Result<(Lattice, Vec<c64>)> {
        grid.check()?;
        boundaries.check(&grid)?;
        let k0 = 2.0 * std::f64::consts::PI / wavelength.to_um();
        let lattice = Lattice::new(grid, boundaries, k0);
        let h = [grid.dx, grid.dy, grid.dz];
        let samples = 8;
        let eps = (0..grid.unknowns())
            .map(|r| {
                let (c, [i, j, k]) = grid.at(r);
                averaged(&eps, grid.e_position(c, (i, j, k)), h, c, samples)
            })
            .collect();
        Ok((lattice, eps))
    }

    /// The grid.
    pub fn grid(&self) -> Grid3d {
        self.lattice.grid
    }

    /// The permittivity E's `component` of index (i, j, k) sees: averaged over its cell.
    pub fn permittivity(&self, component: Axis, at: (usize, usize, usize)) -> c64 {
        self.eps[self.lattice.grid.index(component, at)]
    }

    /// The field for an electric current J = η₀ J on the edges, one value per value of E, laid
    /// out as [`Grid3d::index`] says (in units of E per µm). On a wall behind a PML the current
    /// is ignored.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the source isn't one value per value of E.
    pub fn solve(&self, current: &[c64]) -> Result<Field3d> {
        let i_k0 = c64::new(0.0, -self.lattice.k0);
        let rhs: Vec<c64> = current.iter().map(|j| i_k0 * j).collect();
        self.solve_system(&rhs)
    }

    /// The field for a magnetic current M (∇ × E = i k₀ H̃ − M) on the faces, one value per
    /// value of H, laid out as [`Grid3d::index`] says. [`Field3d::h`] is ∇ × E / (i k₀): where M
    /// flows, H̃ is that plus M / (i k₀).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the source isn't one value per value of H.
    pub fn solve_magnetic(&self, current: &[c64]) -> Result<Field3d> {
        let n = self.lattice.grid.unknowns();
        if current.len() != n {
            return Err(Error::invalid(
                "fdfd source",
                format!(
                    "needs {n} values, one per value of H, got {}",
                    current.len()
                ),
            ));
        }
        let rhs: Vec<c64> = (0..n)
            .map(|r| {
                self.lattice
                    .curl_h(r)
                    .into_iter()
                    .map(|(face, w)| w * current[face])
                    .sum()
            })
            .collect();
        self.solve_system(&rhs)
    }

    /// The field E of A E = `rhs`, A = −∇ × ∇ × + k₀² ε the assembled matrix. Values fixed at
    /// zero on a wall behind a PML stay zero whatever `rhs` holds there.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if `rhs` isn't one value per value of E.
    pub fn solve_system(&self, rhs: &[c64]) -> Result<Field3d> {
        let n = self.lattice.grid.unknowns();
        if rhs.len() != n {
            return Err(Error::invalid(
                "fdfd source",
                format!("needs {n} values, one per value of E, got {}", rhs.len()),
            ));
        }
        let rhs: Vec<c64> = rhs
            .iter()
            .enumerate()
            .map(|(r, &v)| {
                if self.lattice.fixed(r) {
                    c64::new(0.0, 0.0)
                } else {
                    v
                }
            })
            .collect();
        let mut values = self.lu.solve(&rhs);
        // one step of iterative refinement, as in 2D, which leaves round-off where the
        // factorization is accurate; where it isn't (on GitHub's Windows runners faer's sparse LU
        // of the same matrix left relative residuals of 1e-2 to 1e-1 where it leaves 1e-14
        // here, and refinement crawled or stalled), QMR preconditioned by the factorization
        // finishes the solve
        let norm = |v: &[c64]| v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
        let scale = norm(&rhs);
        if scale == 0.0 {
            return Ok(Field3d {
                lattice: self.lattice.clone(),
                values: vec![c64::new(0.0, 0.0); n],
            });
        }
        let residual = |values: &[c64]| -> Vec<c64> {
            let applied = self.apply(values);
            rhs.iter().zip(&applied).map(|(b, a)| b - a).collect()
        };
        let r = residual(&values);
        let correction = super::krylov::Preconditioner::solve(&LuInverse(&self.lu), &r);
        for (v, d) in values.iter_mut().zip(&correction) {
            *v += d;
        }
        let r = residual(&values);
        let left = norm(&r) / scale;
        if left > ACCURATE {
            use super::krylov::{Sparse, Stopping, qmr_preconditioned};
            let matrix = Sparse::new(n, self.entries.iter().map(|t| (t.row, t.col, t.val)));
            let stopping = Stopping {
                tolerance: ACCURATE / left,
                max_iterations: 1000,
            };
            let (step, _) = qmr_preconditioned(&matrix, &LuInverse(&self.lu), &r, stopping)
                .map_err(|e| {
                    Error::invalid(
                        "fdfd",
                        format!(
                            "the factorization left a residual of {left:e}, and QMR on it \
                             failed: {e}"
                        ),
                    )
                })?;
            for (v, d) in values.iter_mut().zip(&step) {
                *v += d;
            }
        }
        Ok(Field3d {
            lattice: self.lattice.clone(),
            values,
        })
    }

    /// The `count` modes of the waveguide crossing the plane of nodes `plane` along `axis`, the
    /// axis it runs along, highest effective index first: the scheme's own modes (see
    /// [`PortMode3d`] and docs/methods/fdfd-3d.md). The plane and the next must be outside the
    /// PMLs along `axis`, with the guide the same on both.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the plane isn't at least two cells clear of a PML or the grid's
    /// end along `axis`, if `axis` is Bloch-periodic, if another axis is Bloch-periodic with
    /// k ≠ 0, or if the modes don't converge.
    pub fn port_modes(&self, axis: Axis, plane: usize, count: usize) -> Result<Vec<PortMode3d>> {
        let (b, c) = axis.others();
        let g = self.lattice.grid;
        self.port_modes_within(axis, plane, (0..g.n(b), 0..g.n(c)), count)
    }

    /// The modes of a port that spans only `window`, the cells along the plane's two axes
    /// (`axis.others()`: y and z for a plane normal to x, z and x for y, x and y for z): one
    /// guide of several side by side, each its own port. The modes are solved with walls on the
    /// window's edges and are zero outside it, and the projection sees only the window, so the
    /// guide's field must have decayed to nothing there. A Bloch-periodic axis must be whole.
    ///
    /// # Errors
    ///
    /// As [`Solver3d::port_modes`], and [`Error::InvalidValue`] for a window of fewer than 3
    /// cells along an axis or past the grid.
    pub fn port_modes_within(
        &self,
        axis: Axis,
        plane: usize,
        window: (std::ops::Range<usize>, std::ops::Range<usize>),
        count: usize,
    ) -> Result<Vec<PortMode3d>> {
        self.lattice
            .port_modes(&self.eps, (axis, plane), [window.0, window.1], count)
    }

    /// The right-hand side for [`Solver3d::solve_system`] that launches `mode` at unit amplitude
    /// going `direction` along its axis, by total-field/scattered-field. Going forward the total
    /// field is from the mode's plane on; going backward, up to the plane after it. Either way
    /// the mode's amplitude is 1 on its plane. The guide must be the same along the axis there.
    pub fn mode_source(&self, mode: &PortMode3d, direction: Direction) -> Vec<c64> {
        self.lattice.mode_source(mode, direction)
    }

    /// The power-normalized S-matrix between `ports`: `s[q][p]`, from mode p going in to mode
    /// q coming out. One solve per port, each launching that port's mode; every run's incoming
    /// and outgoing amplitudes are measured at every port, and S solves S A = B, A the incoming
    /// and B the outgoing ones, so what the PMLs send back is part of A, not an error in S.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if a port's mode doesn't belong to this problem.
    pub fn s_matrix(&self, ports: &[Port3d]) -> Result<Vec<Vec<c64>>> {
        self.lattice.s_matrix(ports, |rhs| self.solve_system(rhs))
    }

    /// A v, the assembled matrix times `v`.
    pub(crate) fn apply(&self, v: &[c64]) -> Vec<c64> {
        let mut out = vec![c64::new(0.0, 0.0); v.len()];
        for t in &self.entries {
            out[t.row] += t.val * v[t.col];
        }
        out
    }
}

/// A 3D FDFD solution: E on the edges of the grid's cells.
#[derive(Clone, Debug, PartialEq)]
pub struct Field3d {
    lattice: Lattice,
    values: Vec<c64>,
}

impl Field3d {
    /// The grid.
    pub fn grid(&self) -> Grid3d {
        self.lattice.grid
    }

    /// E's `component` of index (i, j, k), at [`Grid3d::e_position`].
    pub fn e(&self, component: Axis, at: (usize, usize, usize)) -> c64 {
        self.values[self.lattice.grid.index(component, at)]
    }

    /// All of E, laid out as [`Grid3d::index`] says.
    pub fn values(&self) -> &[c64] {
        &self.values
    }

    /// H̃ = η₀ H's `component` of index (i, j, k), at [`Grid3d::h_position`]: ∇ × E / (i k₀),
    /// the scheme's own (with the PML's stretch inside one).
    pub fn h(&self, component: Axis, at: (usize, usize, usize)) -> c64 {
        let r = self.lattice.grid.index(component, at);
        self.h_at(r)
    }

    fn h_at(&self, r: usize) -> c64 {
        self.lattice.h_with(r, &|col| self.values[col])
    }

    /// The forward and backward amplitudes of `mode` in this field, across the mode's plane and
    /// the next (see docs/methods/fdfd-3d.md): projections with the scheme's unconjugated
    /// Lorentz form, exact for the scheme's own modes.
    ///
    /// # Panics
    ///
    /// If the mode's plane isn't on this field's grid.
    pub fn mode_amplitudes(&self, mode: &PortMode3d) -> (c64, c64) {
        self.lattice.mode_amplitudes(mode, &|r| self.values[r])
    }

    /// This field minus `other`, a field of the same problem: the scattered field, given the
    /// incident one.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if they aren't on the same grid with the same boundaries at the
    /// same wavelength.
    pub fn minus(&self, other: &Field3d) -> Result<Field3d> {
        if self.lattice != other.lattice {
            return Err(Error::invalid(
                "fdfd field",
                "can only subtract a field of the same grid, boundaries and wavelength",
            ));
        }
        Ok(Field3d {
            lattice: self.lattice.clone(),
            values: self
                .values
                .iter()
                .zip(&other.values)
                .map(|(a, b)| a - b)
                .collect(),
        })
    }

    /// The power crossing the plane halfway between nodes `plane` and `plane + 1` along `axis`,
    /// towards +`axis`: the plane's sum of ½ Re(E × H̃*) · n̂ over its cells (so η₀ = 1; in
    /// |field|² µm²), with H̃ on the plane and the tangential E the mean of its values on the
    /// two nodes. This is the scheme's own flux: in a lossless region without sources, between
    /// Bloch-periodic sides, it is exactly the same through every plane.
    ///
    /// # Panics
    ///
    /// If node `plane + 1` isn't on the grid.
    pub fn flux(&self, axis: Axis, plane: usize) -> f64 {
        let n = self.lattice.grid.n(axis);
        assert!(
            plane + 1 < n,
            "flux needs nodes {plane} and {} along {axis:?} on the grid",
            plane + 1
        );
        self.lattice.flux_with(axis, plane, &|col| self.values[col])
    }
}

impl Lattice {
    /// H̃ = ∇ × E / (i k₀) at H's value `r`, for the field E whose value `col` is `e(col)`.
    pub(crate) fn h_with(&self, r: usize, e: &impl Fn(usize) -> c64) -> c64 {
        let curl: c64 = self.curl_e(r).into_iter().map(|(col, w)| w * e(col)).sum();
        curl / c64::new(0.0, self.k0)
    }

    /// [`Field3d::flux`] for the field E whose value `col` is `e(col)`; nodes `plane` and
    /// `plane + 1` must be on the grid.
    pub(crate) fn flux_with(&self, axis: Axis, plane: usize, e: &impl Fn(usize) -> c64) -> f64 {
        let g = self.grid;
        let (b, c) = axis.others();
        let mut total = 0.0;
        // S·n̂ = E_b H̃_c* − E_c H̃_b*: E_b and H̃_c share their place in the plane, and so do
        // E_c and H̃_b
        for (component, h, sign) in [(b, c, 0.5), (c, b, -0.5)] {
            for v in 0..g.n(c) {
                for u in 0..g.n(b) {
                    let mut at = [0; 3];
                    at[axis.index()] = plane;
                    at[b.index()] = u;
                    at[c.index()] = v;
                    let low = e(g.index(component, (at[0], at[1], at[2])));
                    at[axis.index()] = plane + 1;
                    let high = e(g.index(component, (at[0], at[1], at[2])));
                    at[axis.index()] = plane;
                    let hv = self.h_with(g.index(h, (at[0], at[1], at[2])), e);
                    total += sign * (0.5 * (low + high) * hv.conj()).re;
                }
            }
        }
        total * g.step(b) * g.step(c)
    }
}

pub(crate) mod checks;
mod iterative;
mod multigrid;
pub(crate) mod port_checks;
#[cfg(test)]
mod port_tests;
mod ports;
#[cfg(test)]
mod tests;

pub use iterative::{Formulation, IterativeSolver3d};
pub use multigrid::{CycleShape, Multigrid};
pub use ports::{Port3d, PortMode3d};

/// The relative residual a direct solve guarantees: round-off, above the 1e-15 to 1e-14 that an
/// accurate factorization and one step of refinement leave, and within what QMR reaches.
const ACCURATE: f64 = 1e-12;

/// A factorization as a preconditioner: (LU)⁻¹ and its transpose.
struct LuInverse<'a>(&'a OrderedLu);

impl super::krylov::Preconditioner for LuInverse<'_> {
    fn solve(&self, v: &[c64]) -> Vec<c64> {
        self.0.solve(v)
    }

    fn solve_transpose(&self, v: &[c64]) -> Vec<c64> {
        self.0.solve_transpose(v)
    }
}

/// The 3D system's factors, its columns eliminated in nested-dissection order on Yee's grid
/// ([`crate::sparse`]): measured against faer's COLAMD on a silicon strip in oxide (cells of
/// 40 nm, PMLs of 6), 32³ cells in 10.1 s and 8.6 GB against 19.6 s and 10.5 GB, 40³ in 33.0 s
/// and 22.8 GB against 62.1 s and 29.7 GB (docs/methods/fdfd-3d.md, "Cost"). `symbolic`, if
/// given, is the analysis of a matrix of the same structure.
fn factorize_ordered(
    grid: &Grid3d,
    triplets: &[Triplet<usize, usize, c64>],
    symbolic: Option<OrderedSymbolic>,
) -> Result<OrderedLu> {
    let n = grid.unknowns();
    let matrix = faer::sparse::SparseColMat::<usize, c64>::try_new_from_triplets(n, n, triplets)
        .map_err(|e| Error::invalid("fdfd", format!("can't assemble the matrix: {e:?}")))?;
    let symbolic = match symbolic {
        Some(s) => s,
        None => {
            // each value of E at its place on the grid, and the matrix's graph
            let mut positions = vec![[0.0; 3]; n];
            for component in Axis::ALL {
                for k in 0..grid.nz {
                    for j in 0..grid.ny {
                        for i in 0..grid.nx {
                            positions[grid.index(component, (i, j, k))] =
                                grid.e_position(component, (i, j, k));
                        }
                    }
                }
            }
            let pairs: Vec<(usize, usize)> = triplets.iter().map(|t| (t.row, t.col)).collect();
            let (starts, neighbours) = adjacency(n, &pairs);
            let order = nested_dissection(&starts, &neighbours, &positions);
            OrderedSymbolic::new(matrix.as_ref(), &order)?
        }
    };
    OrderedLu::new(matrix.as_ref(), symbolic, faer::get_global_parallelism())
}
