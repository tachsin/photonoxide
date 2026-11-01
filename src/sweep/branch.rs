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
///
/// # Anticrossings
///
/// Two modes of the same symmetry don't cross: they couple and anticross, and through the
/// anticrossing each one's field turns into the other's. There the branch may follow either
/// one, and says nothing. A step much shorter than the anticrossing is wide finds the same
/// mode's field turning a little, and stays on the mode by index (*along* the anticrossing,
/// the field changing character); a step that jumps it finds the other mode's field still like
/// the one it left, and changes mode (*across* the gap, the field keeping its character). A
/// step that lands inside it finds both about equally alike and takes either. So which mode is
/// followed depends on where the samples fall: other step sizes, another order, another
/// tolerance can follow another. A sweep whose samples follow both fits a jump of the gap's
/// size and spends its samples there.
///
/// [`Branch::steps`] and [`Branch::least_margin`] show it: each step's overlap with the mode
/// followed from and the largest overlap of any other candidate. Away from an anticrossing the
/// first is near 1 and the second near 0; a step from outside two modes' anticrossing to its
/// middle finds both near 1/√2 ≈ 0.71. A step with a small [`BranchStep::margin`] chose between
/// two modes: sample more finely there (or follow both modes and say which one is meant).
/// `docs/methods/sweep.md` (Anticrossings) gives a measured case.
pub struct Branch<F> {
    cross_section: F,
    candidates: usize,
    found: Vec<(f64, VectorMode)>,
    steps: Vec<(f64, BranchStep)>,
}

/// How [`Branch`] chose its mode at one wavelength: the overlaps ([`VectorMode::overlap`], 0 to
/// 1) of the candidates with the mode followed from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BranchStep {
    /// The wavelength (µm) of the mode followed from: the nearest found before.
    pub from: f64,
    /// The overlap of the mode chosen, the largest.
    pub overlap: f64,
    /// The largest overlap of the other candidates; `None` when there was no other.
    pub runner_up: Option<f64>,
}

impl BranchStep {
    /// [`BranchStep::overlap`] less [`BranchStep::runner_up`] (taken as 0 when there is none):
    /// near 1 for a clear choice, near 0 where two modes were equally alike.
    pub fn margin(&self) -> f64 {
        self.overlap - self.runner_up.unwrap_or(0.0)
    }
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
            steps: Vec::new(),
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
        let (from, reference) = [i.wrapping_sub(1), i]
            .iter()
            .filter_map(|&j| self.found.get(j))
            .min_by(|a, b| (a.0 - um).abs().total_cmp(&(b.0 - um).abs()))
            .map(|f| (f.0, &f.1))
            .ok_or_else(|| Error::invalid("branch", "no mode found yet"))?;
        let found = vector::modes(
            &(self.cross_section)(wavelength)?,
            wavelength,
            self.candidates,
            Some(reference.effective_index().re),
        )?;
        let mut scored = Vec::with_capacity(found.len());
        for mode in found {
            let o = reference.overlap(&mode).ok_or_else(|| {
                Error::invalid("branch", "the grid must be the same at every wavelength")
            })?;
            scored.push((o, mode));
        }
        if scored.is_empty() {
            return Err(Error::invalid("branch", "the solver returned no mode"));
        }
        // the first of the largest, as before the steps were kept
        let mut best = 0;
        for k in 1..scored.len() {
            if scored[k].0 > scored[best].0 {
                best = k;
            }
        }
        let runner_up = scored
            .iter()
            .enumerate()
            .filter(|&(k, _)| k != best)
            .map(|(_, s)| s.0)
            .reduce(f64::max);
        let (overlap, next) = scored.swap_remove(best);
        self.found.insert(i, (um, next.clone()));
        let s = self.steps.partition_point(|s| s.0 < um);
        self.steps.insert(
            s,
            (
                um,
                BranchStep {
                    from,
                    overlap,
                    runner_up,
                },
            ),
        );
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

    /// How each mode after the first was chosen, with its wavelength (µm), in order of
    /// wavelength: where to look for an anticrossing (see [`Branch`]).
    pub fn steps(&self) -> &[(f64, BranchStep)] {
        &self.steps
    }

    /// The BranchStep chosen with the least [`BranchStep::margin`], with its wavelength (µm); `None` before
    /// the first BranchStep. A margin well below 1 is a choice between two modes.
    pub fn least_margin(&self) -> Option<(f64, BranchStep)> {
        self.steps
            .iter()
            .copied()
            .min_by(|a, b| a.1.margin().total_cmp(&b.1.margin()))
    }
}
