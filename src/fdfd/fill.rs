//! 2D FDFD from a [`Plane`]: each field component's permittivity averaged over its own cell of
//! the grid by an [`Averaging`], from samples or from the plane's fill fractions, and with
//! [`Averaging::Tensor`] the off-diagonal coupling of A. Farjadpour et al.'s tensor (Opt. Lett.
//! 31, 2972 (2006), doi:10.1364/OL.31.002972, Eq. 1).
//!
//! **Where each component is averaged.** With E along z, E_z sits at the cells' centres and sees
//! the cell around it; every interface of a plane is parallel to E_z, so the mean ⟨ε⟩ is the
//! right average whatever the averaging (diagonal and tensor give the mean). With H along z,
//! E_y sits on the x-faces and E_x on the y-faces, each seeing the cell of the grid's size
//! centred on it: 1/ε_y = (ε̃⁻¹)_yy there and 1/ε_x = (ε̃⁻¹)_xx.
//!
//! **The off-diagonal coupling** (H along z, [`Averaging::Tensor`]). With E = ε̃⁻¹ D and
//! D = (i/k₀)(∂_y H, −∂_x H), the operator becomes
//! ∂_x(η_yy ∂_x H) + ∂_y(η_xx ∂_y H) − ∂_x(η_xy ∂_y H) − ∂_y(η_xy ∂_x H) + k₀² H, η = ε̃⁻¹.
//! The new terms need ∂_y H on the x-faces and ∂_x H on the y-faces: the mean of the four
//! nearest, as Farjadpour et al. take D_y at an E_x point from its four neighbours. Written with
//! G_x and G_y, the differences from the cells to the x- and y-faces (Bloch phases included),
//! and S, the mean of a y-face's four neighbours onto an x-face,
//!
//! T = ½ [G_xᴴ η⁽ʸ⁾ S G_y + G_yᴴ Sᴴ η⁽ʸ⁾ G_x + G_yᴴ η⁽ˣ⁾ Sᴴ G_x + G_xᴴ S η⁽ˣ⁾ G_y],
//!
//! η⁽ʸ⁾ the xy entry at the E_y points (x-faces) and η⁽ˣ⁾ at the E_x points (y-faces): each
//! placement taken half, each with its transpose, so that T is symmetric (Hermitian with Bloch
//! phases), as the rest of the operator is, and a lossless periodic problem keeps real
//! eigenvalues. Inside a PML the row is divided by the cell's stretches s_x s_y, which keeps
//! the system's complex symmetric similarity; the PML's own stretching of these terms is not
//! modelled, which matters only where an oblique interface runs into a PML.

use std::collections::BTreeMap;

use faer::sparse::Triplet;
use num_complex::Complex64 as c64;
use rayon::prelude::*;

use super::direct::Plan;
use super::{
    Boundaries, Direction, Edges, Field2d, Grid, Polarization, PortMode, Solver2d, averaged, check,
    problem_2d, stretch,
};
use crate::averaging::{Averaging, Plane, Shares};
use crate::backend::Choice;
use crate::units::Wavelength;
use crate::{Error, Result};

/// The permittivities of every component on `grid`: (ε_z at the centres, ε_y on the x-faces,
/// ε_x on the y-faces), and with the tensor the off-diagonal η_xy (on the y-faces, on the
/// x-faces). As [`super::Solver2d::new`] lays them out.
pub(crate) type Values = (Vec<c64>, Vec<c64>, Vec<c64>, (Vec<c64>, Vec<c64>));

/// `plane`'s permittivity for each field component of `polarization` on `grid`, by `averaging`.
pub(crate) fn plane_values(
    grid: Grid,
    polarization: Polarization,
    plane: &Plane,
    averaging: Averaging,
) -> Values {
    let h = (grid.dx, grid.dy);
    let samples = match averaging {
        Averaging::Point => Some(1),
        a if a.reads_fill() => None,
        _ => Some(8),
    };
    let eps = |x: f64, y: f64| plane.at(x, y);
    let at = |(x, y): (f64, f64), along: usize| -> c64 {
        match samples {
            Some(s) => averaged(&eps, (x, y), h, along, s),
            None => plane.shares_at((x, y), h).component(along, averaging),
        }
    };
    let tensor = averaging == Averaging::Tensor && polarization == Polarization::Hz;
    let face_x = |i: usize| grid.x0 + i as f64 * grid.dx;
    let face_y = |j: usize| grid.y0 + j as f64 * grid.dy;
    match polarization {
        Polarization::Ez => {
            let eps_z = by_rows(grid.ny, grid.nx, |i, j| at((grid.x(i), grid.y(j)), 2));
            (eps_z, Vec::new(), Vec::new(), (Vec::new(), Vec::new()))
        }
        Polarization::Hz if !tensor => {
            let eps_y = by_rows(grid.ny, grid.nx + 1, |i, j| at((face_x(i), grid.y(j)), 1));
            let eps_x = by_rows(grid.ny + 1, grid.nx, |i, j| at((grid.x(i), face_y(j)), 0));
            (Vec::new(), eps_y, eps_x, (Vec::new(), Vec::new()))
        }
        Polarization::Hz => {
            // the inverse tensor at each E point: 1/ε on the diagonal, η_xy off it
            let on_y: Vec<[c64; 2]> = by_rows(grid.ny, grid.nx + 1, |i, j| {
                tensor_at(&plane.shares_at((face_x(i), grid.y(j)), h), 1)
            });
            let on_x: Vec<[c64; 2]> = by_rows(grid.ny + 1, grid.nx, |i, j| {
                tensor_at(&plane.shares_at((grid.x(i), face_y(j)), h), 0)
            });
            (
                Vec::new(),
                on_y.iter().map(|t| t[0]).collect(),
                on_x.iter().map(|t| t[0]).collect(),
                (
                    on_x.iter().map(|t| t[1]).collect(),
                    on_y.iter().map(|t| t[1]).collect(),
                ),
            )
        }
    }
}

/// The values `per(i, j)` of `rows` rows of `columns` each, row by row, the rows side by side on
/// rayon's threads and collected in order: the same bits on any number of threads.
fn by_rows<T: Send>(rows: usize, columns: usize, per: impl Fn(usize, usize) -> T + Sync) -> Vec<T> {
    (0..rows)
        .into_par_iter()
        .map(|j| (0..columns).map(|i| per(i, j)).collect::<Vec<T>>())
        .collect::<Vec<Vec<T>>>()
        .into_iter()
        .flatten()
        .collect()
}

/// [ε along `along` (1/(ε̃⁻¹)_aa), (ε̃⁻¹)_xy] for a cell's shares.
fn tensor_at(shares: &Shares<c64>, along: usize) -> [c64; 2] {
    if shares.is_uniform() {
        return [shares.mean(), c64::new(0.0, 0.0)];
    }
    let t = shares.inverse_tensor();
    [1.0 / t[along], t[2]]
}

impl Solver2d {
    /// Assembles and factorizes `polarization` at `wavelength` on `grid`, with the relative
    /// permittivity of `plane`, each component's averaged over its cell by `averaging`, inside
    /// `boundaries`. [`Averaging::Samples`] is [`Solver2d::new`] with the plane's permittivity at
    /// each point, to the bit; see [`crate::averaging`] for the others.
    ///
    /// # Errors
    ///
    /// As [`Solver2d::new`].
    pub fn from_plane(
        grid: Grid,
        polarization: Polarization,
        wavelength: Wavelength,
        plane: &Plane,
        boundaries: Boundaries,
        averaging: Averaging,
    ) -> Result<Solver2d> {
        Self::from_plane_on(
            grid,
            polarization,
            wavelength,
            plane,
            boundaries,
            averaging,
            &Choice::Auto,
        )
    }

    /// [`Solver2d::from_plane`] with the direct solver of `direct`, as [`Solver2d::new_on`].
    ///
    /// # Errors
    ///
    /// As [`Solver2d::from_plane`] and [`Solver2d::new_on`].
    pub fn from_plane_on(
        grid: Grid,
        polarization: Polarization,
        wavelength: Wavelength,
        plane: &Plane,
        boundaries: Boundaries,
        averaging: Averaging,
        direct: &Choice,
    ) -> Result<Solver2d> {
        Self::build_plane(
            grid,
            polarization,
            wavelength,
            plane,
            boundaries,
            averaging,
            Plan::for_problem(direct, problem_2d(grid, &boundaries))?,
        )
    }

    /// The same grid, polarization, boundaries and averaging at another `wavelength` or with
    /// another `plane`, reusing the analysis of the matrix's sparsity (except with the tensor,
    /// whose sparsity follows the geometry).
    ///
    /// # Errors
    ///
    /// As [`Solver2d::from_plane`].
    pub fn reuse_plane(&self, wavelength: Wavelength, plane: &Plane) -> Result<Solver2d> {
        Self::build_plane(
            self.grid,
            self.polarization,
            wavelength,
            plane,
            self.boundaries,
            self.averaging,
            self.plan_for_reuse(),
        )
    }

    /// How the cells were averaged: [`Averaging::Samples`] for a problem not given a plane.
    pub fn averaging(&self) -> Averaging {
        self.averaging
    }

    fn build_plane(
        grid: Grid,
        polarization: Polarization,
        wavelength: Wavelength,
        plane: &Plane,
        boundaries: Boundaries,
        averaging: Averaging,
        symbolic: Plan,
    ) -> Result<Solver2d> {
        check(grid, &boundaries)?;
        let k0 = 2.0 * std::f64::consts::PI / wavelength.to_um();
        let (eps_z, eps_y, eps_x, eta) = plane_values(grid, polarization, plane, averaging);
        Self::finish_averaged(
            grid,
            polarization,
            k0,
            boundaries,
            (eps_z, eps_y, eps_x),
            (symbolic, false),
            (averaging, eta),
        )
    }

    /// The power `mode` carries `direction` in `field` (a field of this problem, made from a
    /// plane), |a|², and its derivative with respect to a parameter p of the geometry, by the
    /// adjoint method: dF/dp = −2 Re(λᵀ (dA/dp) u), Aᵀ λ = conj(a) c ([`super::adjoint`]), with
    /// dA/dp the difference of the matrices assembled from `planes`, the geometry at p − δ and
    /// p + δ, over `step` = 2δ. Assembling costs no factorization, and with the fill's averaging
    /// the matrix is a smooth function of the geometry, so the difference is a derivative; with
    /// samples it moves in steps. The ports' modes must not depend on p.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a field or mode of another problem, or a step that isn't
    /// positive and finite.
    pub fn mode_power_derivative(
        &self,
        field: &Field2d,
        mode: &PortMode,
        direction: Direction,
        planes: [&Plane; 2],
        step: f64,
    ) -> Result<(f64, f64)> {
        if !(step.is_finite() && step > 0.0) {
            return Err(Error::invalid(
                "fdfd derivative",
                format!("the step must be positive and finite, got {step}"),
            ));
        }
        let (a, lambda) = self.mode_adjoint(field, mode, direction)?;
        let matrix = |plane: &Plane| -> BTreeMap<(usize, usize), c64> {
            let (eps_z, eps_y, eps_x, eta) =
                plane_values(self.grid, self.polarization, plane, self.averaging);
            let mut entries = super::assemble(
                self.grid,
                self.polarization,
                self.k0,
                &eps_z,
                &eps_y,
                &eps_x,
                &self.boundaries,
            );
            if !eta.0.is_empty() {
                entries = with_mixed(entries, self.grid, self.k0, &eta, &self.boundaries);
            }
            let mut m = BTreeMap::new();
            for t in entries {
                *m.entry((t.row, t.col)).or_insert(c64::new(0.0, 0.0)) += t.val;
            }
            m
        };
        let (low, high) = (matrix(planes[0]), matrix(planes[1]));
        let mut d = high;
        for (k, v) in low {
            *d.entry(k).or_insert(c64::new(0.0, 0.0)) -= v;
        }
        let u = &field.values;
        let sum: c64 = d
            .into_iter()
            .filter(|(_, v)| *v != c64::new(0.0, 0.0))
            .map(|((r, c), v)| lambda[r] * v * u[c])
            .sum();
        Ok((a.norm_sqr(), -2.0 * (sum / step).re))
    }
}

/// `entries` with the tensor's off-diagonal terms added (see the module's docs), duplicates
/// summed, row by row.
pub(super) fn with_mixed(
    entries: Vec<Triplet<usize, usize, c64>>,
    g: Grid,
    k0: f64,
    (eta_x, eta_y): &(Vec<c64>, Vec<c64>),
    b: &Boundaries,
) -> Vec<Triplet<usize, usize, c64>> {
    let (nx, ny) = (g.nx, g.ny);
    let zero = c64::new(0.0, 0.0);
    let phase = |e: Edges, len: f64| match e {
        Edges::Bloch { k } => Some(c64::from_polar(1.0, k * len)),
        Edges::Pml { .. } => None,
    };
    let px = phase(b.x, nx as f64 * g.dx);
    let py = phase(b.y, ny as f64 * g.dy);
    // the cells x-face (i, j) differences, with their coefficients
    let gx = |i: usize, j: usize| -> [Option<(usize, c64)>; 2] {
        let right = if i < nx {
            Some((j * nx + i, c64::new(1.0 / g.dx, 0.0)))
        } else {
            px.map(|p| (j * nx, p / g.dx))
        };
        let left = if i >= 1 {
            Some((j * nx + i - 1, c64::new(-1.0 / g.dx, 0.0)))
        } else {
            px.map(|p| (j * nx + nx - 1, -p.conj() / g.dx))
        };
        [right, left]
    };
    let gy = |c: usize, l: usize| -> [Option<(usize, c64)>; 2] {
        let top = if l < ny {
            Some((l * nx + c, c64::new(1.0 / g.dy, 0.0)))
        } else {
            py.map(|p| (c, p / g.dy))
        };
        let bottom = if l >= 1 {
            Some(((l - 1) * nx + c, c64::new(-1.0 / g.dy, 0.0)))
        } else {
            py.map(|p| ((ny - 1) * nx + c, -p.conj() / g.dy))
        };
        [top, bottom]
    };
    // x-face (i, j)'s four neighbouring y-faces (column, level, weight)
    let s_yx = |i: usize, j: usize| -> Vec<(usize, usize, c64)> {
        let quarter = c64::new(0.25, 0.0);
        let mut out = Vec::with_capacity(4);
        let columns = [
            if i >= 1 {
                Some((i - 1, quarter))
            } else {
                px.map(|p| (nx - 1, quarter * p.conj()))
            },
            if i < nx {
                Some((i, quarter))
            } else {
                px.map(|p| (0, quarter * p))
            },
        ];
        for (c, w) in columns.into_iter().flatten() {
            for l in [j, j + 1] {
                out.push((c, l, w));
            }
        }
        out
    };
    // y-face (c, l)'s four neighbouring x-faces (face, row, weight)
    let s_xy = |c: usize, l: usize| -> Vec<(usize, usize, c64)> {
        let quarter = c64::new(0.25, 0.0);
        let mut out = Vec::with_capacity(4);
        let rows = [
            if l >= 1 {
                Some((l - 1, quarter))
            } else {
                py.map(|p| (ny - 1, quarter * p.conj()))
            },
            if l < ny {
                Some((l, quarter))
            } else {
                py.map(|p| (0, quarter * p))
            },
        ];
        for (j, w) in rows.into_iter().flatten() {
            for i in [c, c + 1] {
                out.push((i, j, w));
            }
        }
        out
    };
    let mut mixed: Vec<(usize, usize, c64)> = Vec::new();
    let x_faces = if px.is_some() { 1..=nx } else { 0..=nx };
    let y_faces = if py.is_some() { 1..=ny } else { 0..=ny };
    // η at the E_y points (x-faces): ½ (G_xᴴ η S G_y + its transpose)
    for j in 0..ny {
        for i in x_faces.clone() {
            let eta = eta_y[j * (nx + 1) + i];
            if eta == zero {
                continue;
            }
            for (r, a) in gx(i, j).into_iter().flatten() {
                for (c, l, s) in s_yx(i, j) {
                    for (col, bb) in gy(c, l).into_iter().flatten() {
                        mixed.push((r, col, 0.5 * a.conj() * eta * s * bb));
                        mixed.push((col, r, 0.5 * bb.conj() * s.conj() * eta * a));
                    }
                }
            }
        }
    }
    // η at the E_x points (y-faces): ½ (G_yᴴ η Sᴴ G_x + its transpose)
    for l in y_faces {
        for c in 0..nx {
            let eta = eta_x[l * nx + c];
            if eta == zero {
                continue;
            }
            for (r, bb) in gy(c, l).into_iter().flatten() {
                for (i, j, s) in s_xy(c, l) {
                    for (col, a) in gx(i, j).into_iter().flatten() {
                        mixed.push((r, col, 0.5 * bb.conj() * eta * s * a));
                        mixed.push((col, r, 0.5 * a.conj() * s.conj() * eta * bb));
                    }
                }
            }
        }
    }
    // inside a PML, each row divided by its cell's stretches, as the rest of the row is
    let span_x = (g.x0, g.x0 + nx as f64 * g.dx);
    let span_y = (g.y0, g.y0 + ny as f64 * g.dy);
    let scale = |r: usize| {
        let (i, j) = (r % nx, r / nx);
        let sx = stretch(g.x(i), span_x, b.x.pml(), g.dx, k0, b);
        let sy = stretch(g.y(j), span_y, b.y.pml(), g.dy, k0, b);
        1.0 / (sx * sy)
    };
    let mut all: Vec<(usize, usize, c64)> = entries
        .into_iter()
        .map(|t| (t.row, t.col, t.val))
        .chain(mixed.into_iter().map(|(r, c, v)| (r, c, v * scale(r))))
        .collect();
    // a stable sort keeps each entry's terms in the order they were made
    all.sort_by_key(|&(r, c, _)| (r, c));
    let mut out: Vec<Triplet<usize, usize, c64>> = Vec::with_capacity(all.len());
    for (r, c, v) in all {
        match out.last_mut() {
            Some(t) if t.row == r && t.col == c => t.val += v,
            _ => out.push(Triplet::new(r, c, v)),
        }
    }
    out
}

#[cfg(test)]
mod tests;
