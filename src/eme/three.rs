//! Eigenmode expansion in 3D: sections whose cross-sections are planes of Yee's grid
//! ([`super::plane`]), joined as the 2D sections are.

use std::sync::Arc;

use faer::Mat;
use num_complex::Complex64 as c64;
use rayon::prelude::*;

use super::Scattering;
use super::plane::{self, Mode3d, Plane, Pml3d, Stretch};
use super::scattering::{Blocks, interface, product, rows, star, then_propagate};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// A section of a 3D structure: a cross-section and its length along z.
#[derive(Clone, Debug, PartialEq)]
pub struct Section3d {
    /// The cross-section.
    pub plane: Plane,
    /// Its length along z; 0 for a port's reference plane at an interface.
    pub length: Length,
}

impl Section3d {
    /// A section of `plane`, `length` long.
    pub fn new(plane: Plane, length: Length) -> Section3d {
        Section3d { plane, length }
    }

    /// A structure `length` long whose cross-section `plane_at(z)` changes along z (a taper),
    /// cut into `steps` uniform sections of equal length, each the cross-section at its middle
    /// (as [`super::Section::sampled`] in 2D).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no steps or a length that isn't positive, and `plane_at`'s.
    pub fn sampled(
        length: Length,
        steps: usize,
        plane_at: impl Fn(Length) -> Result<Plane>,
    ) -> Result<Vec<Section3d>> {
        let l = length.to_um();
        if steps == 0 || !(l.is_finite() && l > 0.0) {
            return Err(Error::invalid(
                "eme sections",
                format!("needs a positive length and steps, got {l} um in {steps}"),
            ));
        }
        let h = l / steps as f64;
        (0..steps)
            .map(|s| {
                plane_at(Length::um((s as f64 + 0.5) * h)).map(|plane| Section3d {
                    plane,
                    length: Length::um(h),
                })
            })
            .collect()
    }
}

/// A 3D eigenmode expansion's settings: the wavelength, how many modes each section keeps, and
/// the PML inside the window's walls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Eme3d {
    wavelength: Wavelength,
    modes: usize,
    pml: Option<Pml3d>,
}

impl Eme3d {
    /// `modes` modes in each section at `wavelength`, between perfectly conducting walls.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no modes.
    pub fn new(wavelength: Wavelength, modes: usize) -> Result<Eme3d> {
        if modes == 0 {
            return Err(Error::invalid("eme", "needs at least one mode"));
        }
        Ok(Eme3d {
            wavelength,
            modes,
            pml: None,
        })
    }

    /// The same with `pml` inside the walls.
    pub fn with_pml(self, pml: Pml3d) -> Eme3d {
        Eme3d {
            pml: Some(pml),
            ..self
        }
    }

    /// The wavelength.
    pub fn wavelength(&self) -> Wavelength {
        self.wavelength
    }

    /// The number of modes each section keeps.
    pub fn mode_count(&self) -> usize {
        self.modes
    }

    /// The PML, if any.
    pub fn pml(&self) -> Option<Pml3d> {
        self.pml
    }

    /// The first [`Eme3d::mode_count`] modes of `plane`, largest Re β² first, each normalized
    /// to ⟨E, H⟩ = 1 ([`Mode3d::product`]). A plane of at most 1500 unknowns, or one asked for
    /// a third of its modes or more, is solved for all its modes densely; a larger one for
    /// the modes nearest k₀² times its highest permittivity by shift-and-invert Arnoldi
    /// ([`crate::mode::vector::modes`]' eigensolver). Either way on one thread: the same bits on
    /// any number of them.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for more modes than the plane has unknowns, a PML thicker than
    /// the window, or an eigensolve that fails.
    pub fn modes(&self, plane: &Plane) -> Result<Vec<Mode3d>> {
        let stretch = Stretch::new(&plane.grid(), self.pml.as_ref())?;
        plane::solve(plane, &stretch, self.wavelength.wavenumber(), self.modes)
    }

    /// The S-matrix of `sections`, in order along z, between the first's modes and the last's
    /// (as [`super::Eme::s_matrix`]'s). Neighbours of the same cross-section are one section,
    /// and each distinct cross-section's modes are solved once however often it comes back,
    /// as many side by side as rayon has threads; a section's modes are dropped once no later
    /// section needs them.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no sections, sections on different grids, a length that
    /// isn't finite and at least 0; and [`Eme3d::modes`]'.
    pub fn s_matrix(&self, sections: &[Section3d]) -> Result<Scattering> {
        let Some(first) = sections.first() else {
            return Err(Error::invalid("eme", "needs at least one section"));
        };
        let grid = first.plane.grid();
        for s in sections {
            if s.plane.grid() != grid {
                return Err(Error::invalid(
                    "eme",
                    "every 3D section's cross-section is on the same grid",
                ));
            }
            let l = s.length.to_um();
            if !(l.is_finite() && l >= 0.0) {
                return Err(Error::invalid(
                    "eme",
                    format!("a section's length must be at least 0, got {l} um"),
                ));
            }
        }
        // neighbours of the same plane are one section; each distinct plane numbered once
        let mut distinct: Vec<&Plane> = Vec::new();
        let mut merged: Vec<(usize, f64)> = Vec::new();
        for s in sections {
            let id = match distinct.iter().position(|p| **p == s.plane) {
                Some(id) => id,
                None => {
                    distinct.push(&s.plane);
                    distinct.len() - 1
                }
            };
            match merged.last_mut() {
                Some((last, l)) if *last == id => *l += s.length.to_um(),
                _ => merged.push((id, s.length.to_um())),
            }
        }
        let last_use: Vec<usize> = (0..distinct.len())
            .map(|id| merged.iter().rposition(|&(m, _)| m == id).unwrap_or(0))
            .collect();
        let k = self.wavelength.wavenumber();
        let advance = |m: &[Mode3d], l: f64| -> Vec<c64> {
            m.iter()
                .map(|m| (c64::new(0.0, k * l) * m.effective_index()).exp())
                .collect()
        };
        let batch = rayon::current_num_threads().max(1);
        let mut cache: Vec<Option<Arc<Vec<Mode3d>>>> = vec![None; distinct.len()];
        let mut total: Option<Blocks> = None;
        let mut previous: Option<Arc<Vec<Mode3d>>> = None;
        let mut ends: (Vec<c64>, Vec<c64>) = (Vec::new(), Vec::new());
        for (chunk_index, chunk) in merged.chunks(batch).enumerate() {
            let offset = chunk_index * batch;
            let mut need: Vec<usize> = chunk
                .iter()
                .map(|&(id, _)| id)
                .filter(|&id| cache[id].is_none())
                .collect();
            need.sort_unstable();
            need.dedup();
            let solved: Vec<Arc<Vec<Mode3d>>> = need
                .par_iter()
                .map(|&id| self.modes(distinct[id]).map(Arc::new))
                .collect::<Result<_>>()?;
            for (id, modes) in need.into_iter().zip(solved) {
                cache[id] = Some(modes);
            }
            let here: Vec<Arc<Vec<Mode3d>>> = chunk
                .iter()
                .map(|&(id, _)| cache[id].clone().ok_or_else(lost))
                .collect::<Result<_>>()?;
            // the interfaces into each of the chunk's sections, side by side
            let lefts: Vec<Option<Arc<Vec<Mode3d>>>> = std::iter::once(previous.clone())
                .chain(here.iter().map(|m| Some(Arc::clone(m))))
                .take(here.len())
                .collect();
            let faces: Vec<Option<Blocks>> = lefts
                .par_iter()
                .zip(here.par_iter())
                .map(|(left, right)| {
                    left.as_ref()
                        .map(|l| overlaps(l, right).map(|o| interface(&o)))
                        .transpose()
                })
                .collect::<Result<_>>()?;
            for (i, (face, modes)) in faces.into_iter().zip(&here).enumerate() {
                let length = chunk[i].1;
                let p = advance(modes, length);
                total = Some(match (total.take(), face) {
                    (Some(t), Some(face)) => then_propagate(&star(&t, &face), &p),
                    (Some(_), None) => return Err(lost()),
                    (None, _) => {
                        ends.0 = modes.iter().map(Mode3d::effective_index).collect();
                        let n = p.len();
                        let zero = Mat::<c64>::zeros(n, n);
                        let diag =
                            Mat::from_fn(
                                n,
                                n,
                                |a, b| {
                                    if a == b { p[a] } else { c64::new(0.0, 0.0) }
                                },
                            );
                        Blocks {
                            s11: zero.clone(),
                            s12: diag.clone(),
                            s21: diag,
                            s22: zero,
                        }
                    }
                });
                ends.1 = modes.iter().map(Mode3d::effective_index).collect();
            }
            previous = here.last().cloned();
            // drop the modes no later section needs
            let done = offset + chunk.len() - 1;
            for (id, slot) in cache.iter_mut().enumerate() {
                if last_use[id] <= done {
                    *slot = None;
                }
            }
        }
        let total = total.ok_or_else(lost)?;
        Ok(Scattering {
            s11: rows(&total.s11),
            s12: rows(&total.s12),
            s21: rows(&total.s21),
            s22: rows(&total.s22),
            first: ends.0,
            last: ends.1,
        })
    }
}

/// What the cascade can't lose (a section's modes, an interface): an error, never a panic.
fn lost() -> Error {
    Error::invalid("eme", "a section's modes were lost in the cascade")
}

/// O_ij = ⟨E_i^a, H_j^b⟩ between two sections' modes on the same grid and PML: one matrix
/// product, modes × unknowns × modes, on this thread.
///
/// # Errors
///
/// [`Error::InvalidValue`] for modes on different grids or PMLs.
pub(crate) fn overlaps(a: &[Mode3d], b: &[Mode3d]) -> Result<Mat<c64>> {
    let (Some(fa), Some(fb)) = (a.first(), b.first()) else {
        return Err(Error::invalid("eme", "a section without modes"));
    };
    if fa.grid() != fb.grid() || fa.weights() != fb.weights() {
        return Err(Error::invalid(
            "eme",
            "two sections' modes on different grids or PMLs",
        ));
    }
    let n = fa.grid().unknowns();
    let mut left = Mat::<c64>::zeros(a.len(), n);
    for (i, m) in a.iter().enumerate() {
        for (r, v) in m.weighted_e().enumerate() {
            left[(i, r)] = v;
        }
    }
    let right = Mat::from_fn(n, b.len(), |r, j| b[j].h_values()[r]);
    Ok(product(&left, &right))
}
