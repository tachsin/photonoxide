//! The curl-curl operator without its matrix: A v = k₀² ε v − ∇ × (∇ × v) by two passes over
//! the grid's rows, E to H and H to E, as FDTD's kernel takes its curls.
//!
//! [`Lattice::assemble`] stores 13 entries a value of E, each a complex number and an index:
//! about 300 bytes an unknown (twice that with its transpose), read once a product. The
//! operator here keeps the permittivity, one diagonal and the PML's stretch along each axis,
//! and two vectors to work in: 64 bytes an unknown, 96 with the symmetric form's two scalings.
//! Each pass writes its rows on rayon's threads, a row its own task and each value a fixed
//! sum, so a product is the same bits on any number of threads. Measured against the stored
//! matrix in docs/methods/fdfd-3d.md, "Without the matrix".
//!
//! The stretch enters as it does in the matrix: a difference along an axis is divided by the
//! step and by the stretch where the difference sits (halfway between nodes for ∇ × E, at the
//! nodes for ∇ × H). A wall holds the tangential E on it at zero (those values have rows and
//! columns of the identity); a Bloch side wraps with its phase.
//!
//! **The transpose and the symmetric similarity** come from one diagonal. With D the product
//! of the three stretches at a value of E (the one along its component halfway, the two across
//! it at its nodes), D (∇ × ∇ ×) is symmetric when no side is Bloch-periodic, so Aᵀ = D A D⁻¹
//! and B = S A S⁻¹ with S = √D is complex symmetric: what QMR for symmetric matrices takes, one
//! product an iteration ([`super::super::krylov::Sparse::symmetrized`] finds the same D from
//! the stored matrix). A Bloch side's phases turn into their inverses under the transpose:
//! Aᵀ(k) = D A(−k) D⁻¹.

use std::sync::{Arc, Mutex, OnceLock};

use num_complex::Complex64 as c64;
use rayon::prelude::*;

use super::{Axis, Grid3d, Lattice};
use crate::fdfd::Edges;
use crate::fdfd::krylov::Operator;

/// The operator L A(±k) R, with L and R diagonal (1 if none): A itself, its transpose, or its
/// symmetric similarity.
pub(crate) struct MatrixFree {
    grid: Grid3d,
    /// k₀² ε at each value of E.
    diagonal: Arc<Vec<c64>>,
    /// 1/(stretch × step) along each axis: at the nodes, and halfway after each.
    at_nodes: Arc<[Vec<c64>; 3]>,
    at_halves: Arc<[Vec<c64>; 3]>,
    /// Whether an axis ends in walls (a PML, of any thickness) or wraps.
    walls: [bool; 3],
    /// The phase a field takes across each axis' high end, if it wraps; its inverse comes
    /// back across the low end.
    up: [Option<c64>; 3],
    /// D, the product of the stretches at each value of E.
    d: Arc<Vec<c64>>,
    left: Option<Arc<Vec<c64>>>,
    right: Option<Arc<Vec<c64>>>,
    /// The masked, scaled input and H between the passes.
    work: Mutex<(Vec<c64>, Vec<c64>)>,
    /// The transpose, made when first asked for.
    transpose: OnceLock<Box<MatrixFree>>,
}

impl MatrixFree {
    /// A, the matrix [`Lattice::assemble`] gives for `eps` (one value per value of E).
    pub(crate) fn new(lattice: &Lattice, eps: &[c64]) -> MatrixFree {
        let grid = lattice.grid;
        let k2 = lattice.k0 * lattice.k0;
        let over = |stretch: &[Vec<c64>; 3]| -> [Vec<c64>; 3] {
            Axis::ALL.map(|a| {
                stretch[a.index()]
                    .iter()
                    .map(|s| 1.0 / (s * grid.step(a)))
                    .collect()
            })
        };
        let walls = Axis::ALL.map(|a| matches!(lattice.boundaries.edges(a), Edges::Pml { .. }));
        let up = Axis::ALL.map(|a| match lattice.boundaries.edges(a) {
            Edges::Bloch { k } => Some(c64::from_polar(1.0, k * grid.n(a) as f64 * grid.step(a))),
            Edges::Pml { .. } => None,
        });
        let (nx, ny) = (grid.nx, grid.ny);
        let cells = grid.cells();
        let d: Vec<c64> = (0..3 * cells)
            .into_par_iter()
            .map(|r| {
                let (c, rest) = (r / cells, r % cells);
                let at = [rest % nx, (rest / nx) % ny, rest / (nx * ny)];
                (0..3)
                    .map(|a| {
                        if a == c {
                            lattice.halves[a][at[a]]
                        } else {
                            lattice.nodes[a][at[a]]
                        }
                    })
                    .product()
            })
            .collect();
        MatrixFree {
            grid,
            diagonal: Arc::new(eps.par_iter().map(|e| k2 * e).collect()),
            at_nodes: Arc::new(over(&lattice.nodes)),
            at_halves: Arc::new(over(&lattice.halves)),
            walls,
            up,
            d: Arc::new(d),
            left: None,
            right: None,
            work: Mutex::new((Vec::new(), Vec::new())),
            transpose: OnceLock::new(),
        }
    }

    /// The same operator with other diagonals and phases, sharing what it reads.
    fn with(
        &self,
        left: Option<Arc<Vec<c64>>>,
        right: Option<Arc<Vec<c64>>>,
        up: [Option<c64>; 3],
    ) -> MatrixFree {
        MatrixFree {
            grid: self.grid,
            diagonal: self.diagonal.clone(),
            at_nodes: self.at_nodes.clone(),
            at_halves: self.at_halves.clone(),
            walls: self.walls,
            up,
            d: self.d.clone(),
            left,
            right,
            work: Mutex::new((Vec::new(), Vec::new())),
            transpose: OnceLock::new(),
        }
    }

    /// S and B = S A S⁻¹, complex symmetric: S² = D, scaled to a largest |d| of 1 as
    /// [`crate::fdfd::krylov::Sparse::symmetrized`] scales it. None with a Bloch side, whose
    /// phases no diagonal makes symmetric. For A itself (no diagonals yet).
    pub(crate) fn symmetrized(&self) -> Option<(Vec<c64>, MatrixFree)> {
        if self.up.iter().any(Option::is_some) || self.left.is_some() || self.right.is_some() {
            return None;
        }
        let largest = self.d.iter().map(|z| z.norm()).fold(0.0, f64::max);
        let s: Vec<c64> = self.d.par_iter().map(|z| (z / largest).sqrt()).collect();
        let inverse: Vec<c64> = s.par_iter().map(|z| 1.0 / z).collect();
        let b = self.with(Some(Arc::new(s.clone())), Some(Arc::new(inverse)), self.up);
        Some((s, b))
    }

    /// The transpose, as an operator of its own, kept once made:
    /// (L A(k) R)ᵀ = (R D) A(−k) (D⁻¹ L).
    fn transposed(&self) -> &MatrixFree {
        self.transpose
            .get_or_init(|| Box::new(self.make_transposed()))
    }

    fn make_transposed(&self) -> MatrixFree {
        let times = |a: &Option<Arc<Vec<c64>>>, inverse: bool| -> Arc<Vec<c64>> {
            Arc::new(
                self.d
                    .par_iter()
                    .enumerate()
                    .map(|(r, d)| {
                        let other = a.as_ref().map_or(c64::new(1.0, 0.0), |v| v[r]);
                        if inverse { other / d } else { other * d }
                    })
                    .collect(),
            )
        };
        self.with(
            Some(times(&self.right, false)),
            Some(times(&self.left, true)),
            self.up.map(|p| p.map(|z| 1.0 / z)),
        )
    }

    /// Whether E's `component` in row (j, k) is on a wall along y or z, tangential to it.
    fn row_fixed(&self, component: usize, j: usize, k: usize) -> bool {
        (component != 1 && self.walls[1] && j == 0) || (component != 2 && self.walls[2] && k == 0)
    }

    /// L A R `v` into `out`.
    fn product(&self, v: &[c64], out: &mut [c64]) {
        let g = self.grid;
        let (nx, ny, nz) = (g.nx, g.ny, g.nz);
        let rows = ny * nz;
        let cells = g.cells();
        let zero = c64::new(0.0, 0.0);
        let mut work = self.work.lock().unwrap_or_else(|p| p.into_inner());
        let (e, h) = &mut *work;
        e.resize(3 * cells, zero);
        h.resize(3 * cells, zero);
        // E: the input times R, zero where a wall holds it
        e.par_chunks_mut(nx).enumerate().for_each(|(q, row)| {
            let (c, jk) = (q / rows, q % rows);
            let from = q * nx;
            if self.row_fixed(c, jk % ny, jk / ny) {
                row.fill(zero);
                return;
            }
            match &self.right {
                Some(r) => {
                    for (i, value) in row.iter_mut().enumerate() {
                        *value = v[from + i] * r[from + i];
                    }
                }
                None => row.copy_from_slice(&v[from..from + nx]),
            }
            if c != 0 && self.walls[0] {
                row[0] = zero;
            }
        });
        let e: &[c64] = e;
        let row_of = |c: usize, j: usize, k: usize| -> std::ops::Range<usize> {
            let from = c * cells + (k * ny + j) * nx;
            from..from + nx
        };
        // the row one step up or down an axis, with the phase across the end, if there is one
        let step =
            |m: usize, n: usize, phase: Option<c64>, forward: bool| -> Option<(usize, c64)> {
                let one = c64::new(1.0, 0.0);
                if forward {
                    if m + 1 < n {
                        Some((m + 1, one))
                    } else {
                        phase.map(|p| (0, p))
                    }
                } else if m > 0 {
                    Some((m - 1, one))
                } else {
                    phase.map(|p| (n - 1, 1.0 / p))
                }
            };
        let (hy_inv, hz_inv) = (&self.at_halves[1], &self.at_halves[2]);
        let hx_inv = &self.at_halves[0];
        // H = ∇ × E, each component's rows
        {
            let (hx, rest) = h.split_at_mut(cells);
            let (hy, hz) = rest.split_at_mut(cells);
            (
                hx.par_chunks_mut(nx),
                hy.par_chunks_mut(nx),
                hz.par_chunks_mut(nx),
            )
                .into_par_iter()
                .enumerate()
                .for_each(|(q, (hx, hy, hz))| {
                    let (j, k) = (q % ny, q / ny);
                    let (ex, ey, ez) = (
                        &e[row_of(0, j, k)],
                        &e[row_of(1, j, k)],
                        &e[row_of(2, j, k)],
                    );
                    let above = step(j, ny, self.up[1], true);
                    let beyond = step(k, nz, self.up[2], true);
                    let (wy, wz) = (hy_inv[j], hz_inv[k]);
                    // a neighbour's value, zero beyond a wall
                    let at = |c: usize, to: Option<(usize, c64)>, along_y: bool, i: usize| -> c64 {
                        match to {
                            Some((m, phase)) => {
                                let (j, k) = if along_y { (m, k) } else { (j, m) };
                                e[c * cells + (k * ny + j) * nx + i] * phase
                            }
                            None => zero,
                        }
                    };
                    let last = step(nx - 1, nx, self.up[0], true);
                    for i in 0..nx {
                        // the next value along x, across the end if it wraps
                        let next = |row: &[c64]| -> c64 {
                            if i + 1 < nx {
                                row[i + 1]
                            } else {
                                last.map_or(zero, |(m, phase)| row[m] * phase)
                            }
                        };
                        let wx = hx_inv[i];
                        hx[i] = (at(2, above, true, i) - ez[i]) * wy
                            - (at(1, beyond, false, i) - ey[i]) * wz;
                        hy[i] = (at(0, beyond, false, i) - ex[i]) * wz - (next(ez) - ez[i]) * wx;
                        hz[i] = (next(ey) - ey[i]) * wx - (at(0, above, true, i) - ex[i]) * wy;
                    }
                });
        }
        let h: &[c64] = h;
        let (nx_inv, ny_inv, nz_inv) = (&self.at_nodes[0], &self.at_nodes[1], &self.at_nodes[2]);
        // out = L (k₀² ε E − ∇ × H), and the input itself where a wall holds E
        let (ox, rest) = out.split_at_mut(cells);
        let (oy, oz) = rest.split_at_mut(cells);
        (
            ox.par_chunks_mut(nx),
            oy.par_chunks_mut(nx),
            oz.par_chunks_mut(nx),
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(q, (ox, oy, oz))| {
                let (j, k) = (q % ny, q / ny);
                let (hx, hy, hz) = (
                    &h[row_of(0, j, k)],
                    &h[row_of(1, j, k)],
                    &h[row_of(2, j, k)],
                );
                let below = step(j, ny, self.up[1], false);
                let before = step(k, nz, self.up[2], false);
                let (wy, wz) = (ny_inv[j], nz_inv[k]);
                let at = |c: usize, to: Option<(usize, c64)>, along_y: bool, i: usize| -> c64 {
                    match to {
                        Some((m, phase)) => {
                            let (j, k) = if along_y { (m, k) } else { (j, m) };
                            h[c * cells + (k * ny + j) * nx + i] * phase
                        }
                        None => zero,
                    }
                };
                let first = step(0, nx, self.up[0], false);
                let fixed = [0, 1, 2].map(|c| self.row_fixed(c, j, k));
                let base = (k * ny + j) * nx;
                for i in 0..nx {
                    let previous = |row: &[c64]| -> c64 {
                        if i > 0 {
                            row[i - 1]
                        } else {
                            first.map_or(zero, |(m, phase)| row[m] * phase)
                        }
                    };
                    let wx = nx_inv[i];
                    let curl = [
                        (hz[i] - at(2, below, true, i)) * wy
                            - (hy[i] - at(1, before, false, i)) * wz,
                        (hx[i] - at(0, before, false, i)) * wz - (hz[i] - previous(hz)) * wx,
                        (hy[i] - previous(hy)) * wx - (hx[i] - at(0, below, true, i)) * wy,
                    ];
                    let on_wall = i == 0 && self.walls[0];
                    for (c, target) in [&mut *ox, &mut *oy, &mut *oz].into_iter().enumerate() {
                        let r = c * cells + base + i;
                        target[i] = if fixed[c] || (c != 0 && on_wall) {
                            v[r]
                        } else {
                            let value = self.diagonal[r] * e[r] - curl[c];
                            match &self.left {
                                Some(l) => value * l[r],
                                None => value,
                            }
                        };
                    }
                }
            });
    }
}

impl Operator for MatrixFree {
    fn size(&self) -> usize {
        self.grid.unknowns()
    }

    fn apply(&self, v: &[c64]) -> Vec<c64> {
        let mut out = vec![c64::new(0.0, 0.0); v.len()];
        self.product(v, &mut out);
        out
    }

    fn apply_transpose(&self, v: &[c64]) -> Vec<c64> {
        self.transposed().apply(v)
    }

    fn apply_into(&self, v: &[c64], out: &mut [c64]) {
        self.product(v, out);
    }

    fn apply_transpose_into(&self, v: &[c64], out: &mut [c64]) {
        self.transposed().product(v, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fdfd::krylov::Sparse;
    use crate::fdfd::three::{Boundaries3d, Solver3d};
    use crate::units::Wavelength;

    /// A grid of unequal sides and steps, silicon in oxide off its axes, and a vector of
    /// fixed values with no symmetry.
    fn problem(boundaries: Boundaries3d) -> (Lattice, Vec<c64>, Sparse, Vec<c64>) {
        let grid = Grid3d {
            nx: 11,
            ny: 9,
            nz: 10,
            dx: 0.04,
            dy: 0.05,
            dz: 0.045,
            x0: -0.2,
            y0: -0.21,
            z0: -0.25,
        };
        let eps = |x: f64, y: f64, z: f64| {
            let inside = (x - 0.02).abs() < 0.11 && (y + 0.01).abs() < 0.08 && z.abs() < 0.1;
            c64::new(
                if inside { 12.1 } else { 2.1 },
                if inside { 0.05 } else { 0.0 },
            )
        };
        let (lattice, eps) =
            Solver3d::setup(grid, Wavelength::um(1.55).unwrap(), eps, boundaries).unwrap();
        let matrix = Sparse::new(
            grid.unknowns(),
            lattice
                .assemble(&eps)
                .into_iter()
                .map(|t| (t.row, t.col, t.val)),
        );
        let v: Vec<c64> = (0..grid.unknowns())
            .map(|r| {
                let t = r as f64;
                c64::new((0.37 * t).sin() + 0.2, (0.11 * t * t).cos() - 0.4)
            })
            .collect();
        (lattice, eps, matrix, v)
    }

    /// The largest |a − b| over the largest |b|.
    fn off(a: &[c64], b: &[c64]) -> f64 {
        let scale = b.iter().map(|z| z.norm()).fold(0.0, f64::max);
        a.iter()
            .zip(b)
            .map(|(p, q)| (p - q).norm())
            .fold(0.0, f64::max)
            / scale
    }

    fn cases() -> Vec<(&'static str, Boundaries3d)> {
        let bloch = |k: f64| Edges::Bloch { k };
        vec![
            ("PMLs", Boundaries3d::pml(3)),
            ("stretched PMLs", Boundaries3d::stretched_pml(3)),
            ("walls", Boundaries3d::pml(0)),
            (
                "PMLs of unequal sides",
                Boundaries3d {
                    x: Edges::Pml { low: 2, high: 4 },
                    z: Edges::Pml { low: 0, high: 3 },
                    ..Boundaries3d::pml(3)
                },
            ),
            (
                "Bloch along x",
                Boundaries3d {
                    x: bloch(0.8),
                    ..Boundaries3d::pml(3)
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
        ]
    }

    #[test]
    fn the_product_is_the_assembled_matrixs_to_rounding() {
        for (name, boundaries) in cases() {
            let (lattice, eps, matrix, v) = problem(boundaries);
            let free = MatrixFree::new(&lattice, &eps);
            assert_eq!(free.size(), matrix.size());
            let (a, b) = (free.apply(&v), matrix.apply(&v));
            assert!(off(&a, &b) < 1e-13, "{name}: {}", off(&a, &b));
            // into a vector that held something else, twice: the work it keeps is its own
            let mut out = v.clone();
            free.apply_into(&v, &mut out);
            assert_eq!(out, a, "{name}");
            free.apply_into(&v, &mut out);
            assert_eq!(out, a, "{name}");
        }
    }

    #[test]
    fn the_transposed_product_is_the_assembled_matrixs_to_rounding() {
        for (name, boundaries) in cases() {
            let (lattice, eps, matrix, v) = problem(boundaries);
            let free = MatrixFree::new(&lattice, &eps);
            let (a, b) = (free.apply_transpose(&v), matrix.apply_transpose(&v));
            assert!(off(&a, &b) < 1e-12, "{name}: {}", off(&a, &b));
            let mut out = vec![c64::new(0.0, 0.0); v.len()];
            free.apply_transpose_into(&v, &mut out);
            assert_eq!(out, a, "{name}");
            // and vᵀ(A w) = (Aᵀ v)ᵀ w, by the operator alone
            let w: Vec<c64> = v.iter().rev().map(|z| z.conj() * 0.7).collect();
            let dot = |p: &[c64], q: &[c64]| p.iter().zip(q).map(|(x, y)| x * y).sum::<c64>();
            let (left, right) = (dot(&v, &free.apply(&w)), dot(&a, &w));
            assert!((left - right).norm() < 1e-11 * left.norm(), "{name}");
        }
    }

    #[test]
    fn the_symmetric_similarity_is_the_stored_matrixs() {
        for (name, boundaries) in cases() {
            let (lattice, eps, matrix, v) = problem(boundaries);
            let free = MatrixFree::new(&lattice, &eps);
            let bloch = name.starts_with("Bloch");
            let (Some((s, b)), Some((s_stored, b_stored))) =
                (free.symmetrized(), matrix.symmetrized())
            else {
                // a Bloch side: neither has one
                assert!(bloch, "{name}");
                assert!(free.symmetrized().is_none() && matrix.symmetrized().is_none());
                continue;
            };
            assert!(!bloch, "{name}");
            // B is symmetric: its transpose is itself
            let (forward, back) = (b.apply(&v), b.apply_transpose(&v));
            assert!(
                off(&back, &forward) < 1e-12,
                "{name}: {}",
                off(&back, &forward)
            );
            // the stored matrix's S is this one's times a constant (it starts its walk at 1
            // where this one takes the stretches' product) and a sign at each value (a square
            // root's two branches): the same B but for those signs, P B P
            let free_rows: Vec<usize> = (0..v.len()).filter(|&r| !lattice.fixed(r)).collect();
            let ratio = s[free_rows[0]] / s_stored[free_rows[0]];
            let mut sign = vec![1.0; v.len()];
            for &r in &free_rows {
                let p = s[r] / s_stored[r] / ratio;
                assert!(
                    (p.re.abs() - 1.0).abs() < 1e-12 && p.im.abs() < 1e-12,
                    "{name}: {p}"
                );
                sign[r] = p.re.signum();
            }
            let flipped: Vec<c64> = v.iter().zip(&sign).map(|(z, p)| z * p).collect();
            let stored: Vec<c64> = b_stored
                .apply(&flipped)
                .iter()
                .zip(&sign)
                .map(|(z, p)| z * p)
                .collect();
            assert!(
                off(&forward, &stored) < 1e-12,
                "{name}: {}",
                off(&forward, &stored)
            );
            // and it has no similarity of a similarity
            assert!(b.symmetrized().is_none());
        }
    }

    #[test]
    fn a_product_is_the_same_bits_on_any_number_of_threads() {
        let (lattice, eps, _, v) = problem(Boundaries3d::stretched_pml(3));
        let free = MatrixFree::new(&lattice, &eps);
        let (_, symmetric) = free.symmetrized().unwrap();
        let on = |threads: usize| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| {
                    (
                        free.apply(&v),
                        free.apply_transpose(&v),
                        symmetric.apply(&v),
                    )
                })
        };
        let one = on(1);
        for threads in [2, 4, 5, 20] {
            assert_eq!(on(threads), one, "{threads} threads");
        }
    }
}

#[cfg(test)]
mod measure {
    use std::time::Instant;

    use super::*;
    use crate::fdfd::krylov::Sparse;
    use crate::fdfd::three::{Boundaries3d, Solver3d};
    use crate::units::Wavelength;

    #[test]
    #[ignore = "a measurement: STORED=<largest n with the matrix> (80), FREE=<sizes> (40,64,80,96,128), RAYON_NUM_THREADS; cargo test --release fdfd::three::matrix_free::measure -- --ignored --nocapture"]
    fn products() {
        let sizes: Vec<usize> = std::env::var("FREE")
            .unwrap_or_else(|_| "40,64,80,96,128".into())
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        let stored_up_to: usize = std::env::var("STORED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(80);
        println!("threads {}", rayon::current_num_threads());
        for n in sizes {
            let h = 0.01;
            let half = n as f64 * h / 2.0;
            let grid = Grid3d {
                nx: n,
                ny: n,
                nz: n,
                dx: h,
                dy: h,
                dz: h,
                x0: -half,
                y0: -half,
                z0: -half,
            };
            // the benchmark's guide: a square silicon guide a quarter of the grid across
            let w = n as f64 * h / 8.0;
            let eps = move |_: f64, y: f64, z: f64| {
                c64::new(
                    if y.abs() < w && z.abs() < w {
                        12.09
                    } else {
                        1.0
                    },
                    0.0,
                )
            };
            let lam = Wavelength::um(1.55).unwrap();
            let (lattice, eps) = Solver3d::setup(grid, lam, eps, Boundaries3d::pml(10)).unwrap();
            let unknowns = grid.unknowns();
            let v: Vec<c64> = (0..unknowns)
                .map(|r| c64::new((0.37 * r as f64).sin(), (0.11 * r as f64).cos()))
                .collect();
            let mut out = vec![c64::new(0.0, 0.0); unknowns];
            // the best of several products, in nanoseconds an unknown
            let time = |op: &dyn Fn(&[c64], &mut [c64]), out: &mut [c64]| -> f64 {
                op(&v, out);
                let repeats = (40_000_000 / unknowns).clamp(3, 30);
                (0..repeats)
                    .map(|_| {
                        let t = Instant::now();
                        op(&v, out);
                        t.elapsed().as_secs_f64()
                    })
                    .fold(f64::INFINITY, f64::min)
                    * 1e9
                    / unknowns as f64
            };
            let t = Instant::now();
            let free = MatrixFree::new(&lattice, &eps);
            let (_, symmetric) = free.symmetrized().unwrap();
            let built = t.elapsed().as_secs_f64();
            let a = time(&|v, out| free.apply_into(v, out), &mut out);
            let b = time(&|v, out| symmetric.apply_into(v, out), &mut out);
            // ε, the stretches' product, S and S⁻¹, and the two work vectors
            let bytes = 16.0 * 6.0;
            let mut line = format!(
                "{n}^3, {unknowns} unknowns: matrix-free A {a:.1} ns/unknown, B = S A S^-1 {b:.1}; built in {built:.2} s; {bytes:.0} B/unknown"
            );
            if n <= stored_up_to {
                let t = Instant::now();
                let matrix = Sparse::new(
                    unknowns,
                    lattice
                        .assemble(&eps)
                        .into_iter()
                        .map(|t| (t.row, t.col, t.val)),
                );
                let (_, sb) = matrix.symmetrized().unwrap();
                let built = t.elapsed().as_secs_f64();
                let c = time(&|v, out| matrix.apply_into(v, out), &mut out);
                let d = time(&|v, out| sb.apply_into(v, out), &mut out);
                // each entry's value and column, twice (the matrix and its transpose)
                let stored = 2.0 * 24.0 * matrix.nonzeros() as f64 / unknowns as f64;
                line.push_str(&format!(
                    "; stored A {c:.1}, B {d:.1}; assembled in {built:.2} s; {} nonzeros, {stored:.0} B/unknown",
                    matrix.nonzeros()
                ));
            }
            println!("{line}");
        }
    }

    #[test]
    #[ignore = "a measurement: QMR on the guide with and without the matrix; SIZES (40,64), RAYON_NUM_THREADS; cargo test --release fdfd::three::matrix_free::measure::solves -- --ignored --nocapture"]
    fn solves() {
        use crate::fdfd::{Formulation, IterativeSolver3d, Stopping};
        let sizes: Vec<usize> = std::env::var("SIZES")
            .unwrap_or_else(|_| "40,64".into())
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        println!("threads {}", rayon::current_num_threads());
        for n in sizes {
            let h = 0.01;
            let half = n as f64 * h / 2.0;
            let grid = Grid3d {
                nx: n,
                ny: n,
                nz: n,
                dx: h,
                dy: h,
                dz: h,
                x0: -half,
                y0: -half,
                z0: -half,
            };
            // `photonoxide bench`'s guide at 40: a square silicon guide a quarter of the grid
            // across, PMLs of 10, a dipole 3 cells off the centre
            let w = n as f64 * h / 8.0;
            let eps = move |_: f64, y: f64, z: f64| {
                c64::new(
                    if y.abs() < w && z.abs() < w {
                        12.09
                    } else {
                        1.0
                    },
                    0.0,
                )
            };
            let lam = Wavelength::um(1.55).unwrap();
            let mut source = vec![c64::new(0.0, 0.0); grid.unknowns()];
            source[grid.index(Axis::X, (n / 2 + 3, n / 2 + 3, n / 2 + 3))] = c64::new(1.0, 0.0);
            let stopping = Stopping {
                tolerance: 1e-6,
                max_iterations: 100_000,
            };
            let pml = Boundaries3d::pml(10);
            let t = Instant::now();
            let free = IterativeSolver3d::matrix_free(grid, lam, eps, pml).unwrap();
            let built_free = t.elapsed().as_secs_f64();
            let t = Instant::now();
            let (a, how_a) = free.solve(&source, stopping).unwrap();
            let solved_free = t.elapsed().as_secs_f64();
            let t = Instant::now();
            let stored =
                IterativeSolver3d::new(grid, lam, eps, pml, Formulation::CurlCurl).unwrap();
            let built = t.elapsed().as_secs_f64();
            let t = Instant::now();
            let (b, how_b) = stored.solve(&source, stopping).unwrap();
            let solved = t.elapsed().as_secs_f64();
            let largest = b.values().iter().map(|v| v.norm()).fold(0.0, f64::max);
            let d = a
                .values()
                .iter()
                .zip(b.values())
                .map(|(p, q)| (p - q).norm())
                .fold(0.0, f64::max);
            println!(
                "{n}^3: without the matrix, built in {built_free:.2} s, {} iterations in {solved_free:.2} s ({:.2} ms each); with it, built in {built:.2} s, {} iterations in {solved:.2} s ({:.2} ms each); fields differ by {:.1e} of the largest",
                how_a.iterations,
                1e3 * solved_free / how_a.iterations as f64,
                how_b.iterations,
                1e3 * solved / how_b.iterations as f64,
                d / largest
            );
        }
    }
}
