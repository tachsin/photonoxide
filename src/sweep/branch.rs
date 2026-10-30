//! One mode followed across wavelength by its fields' overlaps, at wavelengths in any order: a
//! sweep's samples come where its estimate asks, not one after another.

use num_complex::Complex64 as c64;

use crate::mode::vector::{self, CrossSection, VectorMode};
use crate::units::Wavelength;
use crate::{Error, Result};

/// One mode of a cross-section that changes with wavelength, followed by overlap: the mode at
/// each new wavelength is, of the `candidates` modes with effective indices nearest the mode
/// found at the nearest wavelength so far, the one whose transverse magnetic field overlaps
/// that mode's most ([`VectorMode::overlap`]). It carries on through a crossing with another
/// mode, where sorting by effective index would jump to the other. The rule is
/// [`crate::mode::dispersion::track`]'s, with the reference the nearest wavelength found rather
/// than the last in a list. Give a [`crate::sweep::Sweep`] its
/// [`Branch::effective_index`]: the sweep's first samples are in order from the start of its
/// range, so a branch started there steps along the range before the sweep places samples
/// where it chooses.
///
/// A crossing is followed when the candidates include the mode followed: as many as there are
/// modes within the change of its effective index between neighbouring samples.
pub struct Branch<F> {
    cross_section: F,
    candidates: usize,
    found: Vec<(f64, VectorMode)>,
}

impl<F> Branch<F>
where
    F: FnMut(Wavelength) -> Result<CrossSection>,
{
    /// A branch starting at `start` with the mode whose effective index is nearest `near` (the
    /// fundamental when `None`), followed among `candidates` modes at each next wavelength. The
    /// cross-section at each wavelength is `cross_section`'s, on the same grid at every one.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no candidates or no mode at `start`; and the errors of
    /// `cross_section` and of [`vector::modes`].
    pub fn new(
        mut cross_section: F,
        start: Wavelength,
        near: Option<f64>,
        candidates: usize,
    ) -> Result<Branch<F>> {
        if candidates == 0 {
            return Err(Error::invalid(
                "branch",
                "needs at least one candidate mode",
            ));
        }
        let first = vector::modes(&cross_section(start)?, start, 1, near)?
            .into_iter()
            .next()
            .ok_or_else(|| Error::invalid("branch", "the solver returned no mode"))?;
        Ok(Branch {
            cross_section,
            candidates,
            found: vec![(start.to_um(), first)],
        })
    }

    /// The branch's mode at `wavelength`: found before, or followed from the mode at the
    /// nearest wavelength found.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the solver returns no mode or the grid changes between
    /// wavelengths; and the errors of the cross-section and of [`vector::modes`].
    pub fn mode(&mut self, wavelength: Wavelength) -> Result<VectorMode> {
        let um = wavelength.to_um();
        let i = self.found.partition_point(|f| f.0 < um);
        if let Some(f) = self.found.get(i).filter(|f| f.0 == um) {
            return Ok(f.1.clone());
        }
        // the nearest wavelength found: the one either side of where this one goes
        let reference = [i.wrapping_sub(1), i]
            .iter()
            .filter_map(|&j| self.found.get(j))
            .min_by(|a, b| (a.0 - um).abs().total_cmp(&(b.0 - um).abs()))
            .map(|f| &f.1)
            .ok_or_else(|| Error::invalid("branch", "no mode found yet"))?;
        let found = vector::modes(
            &(self.cross_section)(wavelength)?,
            wavelength,
            self.candidates,
            Some(reference.effective_index().re),
        )?;
        let mut best: Option<(f64, VectorMode)> = None;
        for mode in found {
            let o = reference.overlap(&mode).ok_or_else(|| {
                Error::invalid("branch", "the grid must be the same at every wavelength")
            })?;
            if best.as_ref().is_none_or(|(b, _)| o > *b) {
                best = Some((o, mode));
            }
        }
        let next = best
            .ok_or_else(|| Error::invalid("branch", "the solver returned no mode"))?
            .1;
        self.found.insert(i, (um, next.clone()));
        Ok(next)
    }

    /// The effective index of [`Branch::mode`] at `wavelength`, for a sweep.
    ///
    /// # Errors
    ///
    /// As [`Branch::mode`].
    pub fn effective_index(&mut self, wavelength: Wavelength) -> Result<c64> {
        Ok(self.mode(wavelength)?.effective_index())
    }

    /// The wavelengths (µm) and modes found so far, in order of wavelength.
    pub fn found(&self) -> &[(f64, VectorMode)] {
        &self.found
    }
}
