//! Universal multiport interferometers: a unitary decomposed into a mesh of beam splitters
//! with phases, in Reck et al.'s triangle (M. Reck et al., Phys. Rev. Lett. 73, 58 (1994),
//! doi:10.1103/PhysRevLett.73.58) or Clements et al.'s rectangle (W. R. Clements et al.,
//! Optica 3, 1460 (2016), doi:10.1364/OPTICA.3.001460), and Haar-random unitaries to program
//! them with.
//!
//! A cell is Clements et al.'s T_m,n(θ, φ) (their Eq. 1) on neighbouring modes m and n = m + 1:
//! a phase φ on mode m, then a beam splitter of reflectivity cos θ,
//!
//! ```text
//! T = | e^(iφ) cos θ   −sin θ |
//!     | e^(iφ) sin θ    cos θ |,
//! ```
//!
//! and a mesh is U = D ∏ T (their Eq. 2), the cells in the order light meets them, D a phase on
//! each output. Both decompositions null U's entries one by one: Reck's with T⁻¹ from the right
//! only, row by row from the bottom; Clements's alternating T⁻¹ from the right and T from the
//! left along successive diagonals (their Fig. 2), the left ones then moved through D, as
//! T⁻¹ D = D′ T′ (their Eq. 5).

use std::f64::consts::PI;

use num_complex::Complex64 as c64;

use super::matrix::{Matrix, qr};
use super::rng::Rng;
use crate::portable;
use crate::{Error, Result};

/// A Haar-random n × n unitary, drawn with `seed`: the Q of the QR factorization of a matrix of
/// independent complex Gaussians, with R's diagonal positive, which makes the distribution the
/// Haar measure (F. Mezzadri, Notices Amer. Math. Soc. 54, 592 (2007), ref:mezzadri-2007).
///
/// ```
/// use photonoxide::quantum::haar_unitary;
/// let u = haar_unitary(6, 1);
/// assert!(u.isometry_error() < 1e-13);
/// ```
pub fn haar_unitary(n: usize, seed: u64) -> Matrix {
    let mut rng = Rng::new(seed, 0);
    let values: Vec<c64> = (0..n * n)
        .map(|_| {
            let re = rng.normal();
            c64::new(re, rng.normal()) / 2f64.sqrt()
        })
        .collect();
    let z = Matrix::from_fn(n, n, |i, j| values[i * n + j]);
    // Mezzadri's fix multiplies Q's column j by R_jj/|R_jj|, making R's diagonal positive:
    // Gram–Schmidt's R has a positive diagonal already, so its Q is Haar-distributed as it is
    qr(&z).0
}

/// One cell of a mesh: Clements et al.'s T_m,m+1(θ, φ).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    /// The upper of its two neighbouring modes, m; the other is m + 1.
    pub mode: usize,
    /// θ: the beam splitter's reflectivity is cos θ, its power transmission across sin² θ.
    pub theta: f64,
    /// φ: the phase on mode m before it.
    pub phi: f64,
}

impl Cell {
    /// The cell's 2 × 2 matrix.
    pub fn block(&self) -> [[c64; 2]; 2] {
        let e = portable::complex::from_polar(1.0, self.phi);
        let (s, c) = portable::sin_cos(self.theta);
        [[e * c, c64::new(-s, 0.0)], [e * s, c64::new(c, 0.0)]]
    }
}

/// Which arrangement a mesh has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrangement {
    /// Reck et al.'s triangle: optical depth 2N − 3.
    Reck,
    /// Clements et al.'s rectangle: optical depth N.
    Clements,
}

/// A unitary as a mesh of N(N − 1)/2 cells and N output phases: U = D ∏ T.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    arrangement: Arrangement,
    modes: usize,
    cells: Vec<Cell>,
    phases: Vec<f64>,
}

impl Mesh {
    /// U's decomposition in Reck et al.'s triangle.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless U is square and unitary to 1e-9.
    pub fn reck(u: &Matrix) -> Result<Mesh> {
        let mut u = checked(u)?;
        let n = u.rows();
        let mut cells = Vec::with_capacity(n * n.saturating_sub(1) / 2);
        for row in (1..n).rev() {
            for col in 0..row {
                let cell = null_from_right(&u, row, col);
                apply_right_inverse(&mut u, &cell);
                cells.push(cell);
            }
        }
        Ok(Mesh {
            arrangement: Arrangement::Reck,
            modes: n,
            cells,
            phases: (0..n).map(|i| portable::complex::arg(u[(i, i)])).collect(),
        })
    }

    /// U's decomposition in Clements et al.'s rectangle, by their algorithm (their Fig. 2 and
    /// Supplement 1).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless U is square and unitary to 1e-9.
    pub fn clements(u: &Matrix) -> Result<Mesh> {
        let mut u = checked(u)?;
        let n = u.rows();
        let mut right: Vec<Cell> = Vec::new();
        let mut left: Vec<Cell> = Vec::new();
        for i in 0..n.saturating_sub(1) {
            for j in 0..=i {
                if i % 2 == 0 {
                    // null (n − 1 − j, i − j) with T⁻¹ on columns i − j, i − j + 1
                    let cell = null_from_right(&u, n - 1 - j, i - j);
                    apply_right_inverse(&mut u, &cell);
                    right.push(cell);
                } else {
                    // null (n + j − i − 1, j) with T on rows n + j − i − 2, n + j − i − 1
                    let cell = null_from_left(&u, n + j - i - 2, j);
                    apply_left(&mut u, &cell);
                    left.push(cell);
                }
            }
        }
        // left_k … left_1 U right_1⁻¹ … right_r⁻¹ = D, so U = left_1⁻¹ … left_k⁻¹ D right_r … right_1;
        // each left⁻¹ next to D moves through it: T⁻¹(θ, φ) D = D′ T(θ, φ′)
        let mut d: Vec<c64> = (0..n)
            .map(|i| u[(i, i)] / portable::complex::abs(u[(i, i)]))
            .collect();
        let mut moved: Vec<Cell> = Vec::with_capacity(left.len());
        for cell in left.iter().rev() {
            let (m, k) = (cell.mode, cell.mode + 1);
            let (dm, dn) = (d[m], d[k]);
            moved.push(Cell {
                mode: m,
                theta: cell.theta,
                phi: portable::complex::arg(-dm / dn),
            });
            d[m] = -portable::complex::from_polar(1.0, -cell.phi) * dn;
            d[k] = dn;
        }
        // U = D T′_1 … T′_k right_r … right_1: light meets right_1 … right_r, then T′_k (moved
        // first) … T′_1
        let mut cells = right;
        cells.extend(moved);
        Ok(Mesh {
            arrangement: Arrangement::Clements,
            modes: n,
            cells,
            phases: d.iter().map(|z| portable::complex::arg(*z)).collect(),
        })
    }

    /// The arrangement.
    pub fn arrangement(&self) -> Arrangement {
        self.arrangement
    }

    /// The number of modes, N.
    pub fn modes(&self) -> usize {
        self.modes
    }

    /// The cells, in the order light meets them.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// The output phases, D's arguments.
    pub fn phases(&self) -> &[f64] {
        &self.phases
    }

    /// The optical depth: the most cells any path crosses.
    pub fn depth(&self) -> usize {
        let mut crossed = vec![0usize; self.modes];
        for c in &self.cells {
            let d = crossed[c.mode].max(crossed[c.mode + 1]) + 1;
            crossed[c.mode] = d;
            crossed[c.mode + 1] = d;
        }
        crossed.into_iter().max().unwrap_or(0)
    }

    /// The mesh's matrix, D ∏ T: U again, to round-off.
    pub fn matrix(&self) -> Matrix {
        self.lossy(0.0)
    }

    /// The matrix of the mesh with every cell losing `db` decibels of power on both outputs,
    /// Clements et al.'s model of unbalanced loss (their Section 5).
    pub fn lossy(&self, db: f64) -> Matrix {
        let n = self.modes;
        let amplitude = portable::powf(10f64, -db / 20.0);
        let mut m = Matrix::identity(n);
        for cell in &self.cells {
            let t = cell.block();
            let (a, b) = (cell.mode, cell.mode + 1);
            for col in 0..n {
                let (x, y) = (m[(a, col)], m[(b, col)]);
                m[(a, col)] = (t[0][0] * x + t[0][1] * y) * amplitude;
                m[(b, col)] = (t[1][0] * x + t[1][1] * y) * amplitude;
            }
        }
        Matrix::from_fn(n, n, |i, j| {
            portable::complex::from_polar(1.0, self.phases[i]) * m[(i, j)]
        })
    }
}

/// Clements et al.'s fidelity of an implemented transformation to the intended unitary, their
/// Eq. 6: |tr(U† U_exp) / √(N tr(U_exp† U_exp))|², blind to an overall factor, so it measures
/// unbalanced loss only. 1 for U_exp = c U.
pub fn fidelity(target: &Matrix, implemented: &Matrix) -> f64 {
    let n = target.rows();
    let overlap: c64 = (0..n)
        .flat_map(|i| (0..n).map(move |j| (i, j)))
        .map(|(i, j)| target[(i, j)].conj() * implemented[(i, j)])
        .sum();
    let power: f64 = (0..n)
        .flat_map(|i| (0..n).map(move |j| (i, j)))
        .map(|(i, j)| implemented[(i, j)].norm_sqr())
        .sum();
    overlap.norm_sqr() / (n as f64 * power)
}

fn checked(u: &Matrix) -> Result<Matrix> {
    if !u.is_square() || u.isometry_error() > 1e-9 {
        return Err(Error::invalid(
            "mesh",
            "decomposes a square unitary matrix only",
        ));
    }
    Ok(u.clone())
}

/// The cell T on columns (col, col + 1) whose inverse from the right zeroes U's (row, col):
/// tan θ = |U_row,col / U_row,col+1| and φ = arg U_row,col − arg U_row,col+1.
fn null_from_right(u: &Matrix, row: usize, col: usize) -> Cell {
    let (x, y) = (u[(row, col)], u[(row, col + 1)]);
    Cell {
        mode: col,
        theta: portable::atan2(portable::complex::abs(x), portable::complex::abs(y)),
        phi: if portable::complex::abs(x) > 0.0 && portable::complex::abs(y) > 0.0 {
            portable::complex::arg(x) - portable::complex::arg(y)
        } else {
            0.0
        },
    }
}

/// The cell T on rows (row, row + 1) that zeroes U's (row + 1, col) from the left:
/// tan θ = |U_row+1,col / U_row,col| and φ = arg U_row+1,col − arg U_row,col + π.
fn null_from_left(u: &Matrix, row: usize, col: usize) -> Cell {
    let (x, y) = (u[(row, col)], u[(row + 1, col)]);
    Cell {
        mode: row,
        theta: portable::atan2(portable::complex::abs(y), portable::complex::abs(x)),
        phi: if portable::complex::abs(x) > 0.0 && portable::complex::abs(y) > 0.0 {
            portable::complex::arg(y) - portable::complex::arg(x) + PI
        } else {
            0.0
        },
    }
}

/// U ← U T⁻¹ on the cell's columns: T⁻¹ = T†.
fn apply_right_inverse(u: &mut Matrix, cell: &Cell) {
    let t = cell.block();
    let (a, b) = (cell.mode, cell.mode + 1);
    for row in 0..u.rows() {
        let (x, y) = (u[(row, a)], u[(row, b)]);
        // (U T†)_r,a = x conj(T_aa) + y conj(T_ab); (U T†)_r,b = x conj(T_ba) + y conj(T_bb)
        u[(row, a)] = x * t[0][0].conj() + y * t[0][1].conj();
        u[(row, b)] = x * t[1][0].conj() + y * t[1][1].conj();
    }
}

/// U ← T U on the cell's rows.
fn apply_left(u: &mut Matrix, cell: &Cell) {
    let t = cell.block();
    let (a, b) = (cell.mode, cell.mode + 1);
    for col in 0..u.cols() {
        let (x, y) = (u[(a, col)], u[(b, col)]);
        u[(a, col)] = t[0][0] * x + t[0][1] * y;
        u[(b, col)] = t[1][0] * x + t[1][1] * y;
    }
}
