//! The kernel alone, in either precision: a problem's fields, update coefficients and CPMLs
//! stepped with nothing else (no sources, monitors, media or Bloch phase), for the benchmarks
//! and for f32 against f64.

use super::{Real, Slab, Stencil, curl_update, update_slabs};
use crate::Result;
use crate::fdfd::{Axis, Grid3d};
use crate::fdtd::{Field, Simulation, invalid};

/// A Yee grid's state in precision `T`: E and H̃, E's coefficients, the CPMLs.
#[derive(Clone, Debug)]
pub(crate) struct Yee<T> {
    grid: Grid3d,
    dt: T,
    e: [Vec<T>; 3],
    h: [Vec<T>; 3],
    ca: [Vec<T>; 3],
    cb: [Vec<T>; 3],
    slabs: Vec<Slab<T>>,
    stencils: [Stencil<T>; 2],
}

impl<T: Real> Yee<T> {
    /// `s`'s state now, rounded to `T`: to step on as `s` would with no sources.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] if `s` has what the kernel alone doesn't step: a Bloch
    /// phase, a dispersive medium or a tensor permittivity that couples E's components.
    pub(crate) fn from_simulation(s: &Simulation) -> Result<Yee<T>> {
        if s.bloch.is_some() || s.anisotropic.is_some() || !s.media.is_empty() {
            return Err(invalid(
                "the kernel alone steps E, H and the CPMLs: no Bloch phase, dispersive medium \
                 or coupling tensor",
            ));
        }
        let convert = |v: &[Vec<f64>; 3]| -> [Vec<T>; 3] {
            std::array::from_fn(|c| v[c].iter().map(|&x| T::of(x)).collect())
        };
        let stencil = |st: &Stencil<f64>| st.to::<T>();
        Ok(Yee {
            grid: s.grid,
            dt: T::of(s.dt),
            e: convert(&s.e),
            h: convert(&s.h),
            ca: convert(&s.ca),
            cb: convert(&s.cb),
            slabs: s.slabs.iter().map(|x| x.to::<T>()).collect(),
            stencils: [stencil(&s.stencils[0]), stencil(&s.stencils[1])],
        })
    }

    /// One step: H̃ by E's curl and its CPMLs, then E by H̃'s, as [`Simulation::step`] does.
    pub(crate) fn step(&mut self) {
        curl_update(
            self.grid,
            &mut self.h,
            &self.e,
            &self.stencils[1],
            None,
            self.dt,
        );
        update_slabs(
            self.grid,
            &mut self.slabs,
            Field::H,
            &mut self.h,
            &self.e,
            &self.cb,
            &self.stencils[1],
            self.dt,
        );
        curl_update(
            self.grid,
            &mut self.e,
            &self.h,
            &self.stencils[0],
            Some((&self.ca, &self.cb)),
            self.dt,
        );
        update_slabs(
            self.grid,
            &mut self.slabs,
            Field::E,
            &mut self.e,
            &self.h,
            &self.cb,
            &self.stencils[0],
            self.dt,
        );
    }

    /// E's `component`.
    pub(crate) fn e(&self, component: Axis) -> &[T] {
        &self.e[component.index()]
    }

    /// The bytes a step reads and writes at least once: each field's three components read
    /// and the updated field's written, E's two coefficients, and in the CPMLs each ψ read
    /// and written with the value it updates (and cb for E).
    pub(crate) fn bytes_per_step(&self) -> usize {
        let size = std::mem::size_of::<T>();
        let cells = self.grid.cells();
        let slabs: usize = self
            .slabs
            .iter()
            .map(|s| s.psi.len() * if s.field == Field::E { 6 } else { 5 })
            .sum();
        (cells * (9 + 15) + slabs) * size
    }
}
