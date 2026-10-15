//! 3D FDFD from a [`Structure`]: each component of E given the permittivity of its own cell of
//! the dual grid by an [`Averaging`], from samples or from the fill fractions of the box
//! ([`crate::stack`]'s slices).
//!
//! With the fill, the mean is ⟨ε⟩ for every component, and the diagonal is Farjadpour et al.'s
//! rule (Opt. Lett. 31, 2972 (2006), doi:10.1364/OL.31.002972, Eq. 1) taken slice by slice: in
//! each slice of the box between the stack's interfaces, the walls are vertical, so E_z sees
//! their mean and E_x, E_y the diagonal of the in-plane tensor with the walls' normal; across
//! the slices, layers normal to z, E_z sees the harmonic mean and E_x, E_y the mean. That is
//! the tensor exactly where every interface is a layer's or a wall along a grid axis; for a wall
//! oblique to the grid it is the tensor's diagonal, without the off-diagonal coupling that only
//! the 2D problem has ([`super::super::Solver2d::from_plane`]). A slice under sloped walls is
//! sampled, as [`Averaging::Samples`] samples a whole cell.

use num_complex::Complex64 as c64;
use rayon::prelude::*;

use super::{Axis, Boundaries3d, Edges, Grid3d, Lattice, Solver3d, averaged, factorize};
use crate::averaging::Averaging;
use crate::backend::Choice;
use crate::fdfd::direct::Plan;
use crate::geometry::Point;
use crate::material::Material;
use crate::stack::Structure;
use crate::stack::fill::{Fills, combine};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// The permittivity of each value of E on `grid` (as [`Grid3d::index`] numbers them) for
/// `structure` at `wavelength`, by `averaging`.
///
/// # Errors
///
/// [`Error::OutsideValidity`] if a material has no data at `wavelength`, and
/// [`Error::InvalidValue`] for [`Averaging::Tensor`], whose off-diagonal coupling the 3D problem
/// doesn't have.
pub(crate) fn structure_values(
    grid: Grid3d,
    wavelength: Wavelength,
    structure: &Structure,
    averaging: Averaging,
) -> Result<Vec<c64>> {
    if averaging == Averaging::Tensor {
        return Err(Error::invalid(
            "fdfd averaging",
            "in 3D the fill's average is \"diagonal\" (or \"mean\"): the tensor's off-diagonal \
             coupling is the 2D problem's only",
        ));
    }
    // bodies over the stack (`Structure::place`): sampled, each point's permittivity theirs
    // where one holds it; the fill's slices are the stack's alone
    let bodies = !structure.bodies().is_empty();
    if bodies {
        if averaging.reads_fill() {
            return Err(Error::invalid(
                "fdfd averaging",
                "a structure with bodies over its stack takes \"samples\" or \"point\": the \
                 fill's slices are the stack's",
            ));
        }
        for (_, medium) in structure.bodies() {
            if let crate::stack::Medium::Material(m) = medium {
                m.permittivity(wavelength)?;
            }
        }
    }
    let fills = Fills::new(structure);
    let materials = fills.materials();
    let eps: Vec<c64> = materials
        .iter()
        .map(|m| m.permittivity(wavelength))
        .collect::<Result<_>>()?;
    let index_of = |m: &Material| {
        materials
            .iter()
            .position(|n| std::ptr::eq(*n, m))
            .expect("every material of the structure is listed")
    };
    let at = |x: f64, y: f64, z: f64| {
        if bodies {
            // every medium's data at the wavelength was checked above
            return structure
                .permittivity_at(Point::um(x, y), Length::um(z), wavelength)
                .unwrap_or(c64::new(f64::NAN, 0.0));
        }
        eps[index_of(structure.material_at(Point::um(x, y), Length::um(z)))]
    };
    let h = [grid.dx, grid.dy, grid.dz];
    let value = |r: usize| -> c64 {
        let (c, [i, j, k]) = grid.at(r);
        let p = grid.e_position(c, (i, j, k));
        match averaging {
            Averaging::Point => at(p[0], p[1], p[2]),
            a if a.reads_fill() => {
                let lo = [p[0] - h[0] / 2.0, p[1] - h[1] / 2.0, p[2] - h[2] / 2.0];
                let hi = [p[0] + h[0] / 2.0, p[1] + h[1] / 2.0, p[2] + h[2] / 2.0];
                let slices = fills.slices(lo, hi);
                combine(&slices, &eps, c.index(), a, |(z0, z1), along| {
                    let centre = [p[0], p[1], 0.5 * (z0 + z1)];
                    let size = [h[0], h[1], z1 - z0];
                    if a == Averaging::Mean {
                        sampled_mean(&at, centre, size)
                    } else {
                        averaged(&at, centre, size, Axis::ALL[along], 8)
                    }
                })
            }
            _ => averaged(&at, p, h, c, 8),
        }
    };
    // collected in order: the same bits on any number of threads
    Ok((0..grid.unknowns()).into_par_iter().map(value).collect())
}

/// The plain mean of 8 × 8 × 8 samples over the box of `size` about `centre`.
fn sampled_mean(eps: &impl Fn(f64, f64, f64) -> c64, centre: [f64; 3], size: [f64; 3]) -> c64 {
    let n = 8;
    let offset = |s: usize| (s as f64 + 0.5) / n as f64 - 0.5;
    let mut sum = c64::new(0.0, 0.0);
    for a in 0..n {
        for b in 0..n {
            for c in 0..n {
                sum += eps(
                    centre[0] + offset(a) * size[0],
                    centre[1] + offset(b) * size[1],
                    centre[2] + offset(c) * size[2],
                );
            }
        }
    }
    sum / (n * n * n) as f64
}

impl Solver3d {
    /// Assembles and factorizes the problem at `wavelength` on `grid` for `structure` (x and y
    /// in its plane, z its height, µm), each component of E averaged over its cell by
    /// `averaging`, inside `boundaries`. [`Averaging::Samples`] is [`Solver3d::new`] with the
    /// structure's permittivity at each point, to the bit; see the module's docs for the fill.
    ///
    /// # Errors
    ///
    /// As [`Solver3d::new`], [`Error::OutsideValidity`] if a material has no data at
    /// `wavelength`, and [`Error::InvalidValue`] for [`Averaging::Tensor`].
    pub fn from_structure(
        grid: Grid3d,
        wavelength: Wavelength,
        structure: &Structure,
        boundaries: Boundaries3d,
        averaging: Averaging,
    ) -> Result<Solver3d> {
        Self::from_structure_on(
            grid,
            wavelength,
            structure,
            boundaries,
            averaging,
            &Choice::Auto,
        )
    }

    /// [`Solver3d::from_structure`] with the direct solver of `direct`, as
    /// [`Solver3d::new_on`].
    ///
    /// # Errors
    ///
    /// As [`Solver3d::from_structure`] and [`Solver3d::new_on`].
    pub fn from_structure_on(
        grid: Grid3d,
        wavelength: Wavelength,
        structure: &Structure,
        boundaries: Boundaries3d,
        averaging: Averaging,
        direct: &Choice,
    ) -> Result<Solver3d> {
        grid.check()?;
        boundaries.check(&grid)?;
        let eps = structure_values(grid, wavelength, structure, averaging)?;
        let bloch = |e: Edges| matches!(e, Edges::Bloch { .. });
        let problem = crate::backend::auto::Problem {
            family: "fdfd3d",
            form: if bloch(boundaries.x) || bloch(boundaries.y) || bloch(boundaries.z) {
                crate::backend::Form::General
            } else {
                crate::backend::Form::Symmetric
            },
            unknowns: grid.unknowns(),
        };
        let k0 = 2.0 * std::f64::consts::PI / wavelength.to_um();
        let lattice = Lattice::new(grid, boundaries, k0);
        let entries = lattice.assemble(&eps);
        let lu = factorize(&grid, &entries, Plan::for_problem(direct, problem)?)?;
        Ok(Solver3d {
            lattice,
            eps,
            entries,
            lu,
        })
    }
}

#[cfg(test)]
mod tests;
