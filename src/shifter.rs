//! The thermo-optic phase shifter on a cross-section: what a heater's power does to the phase
//! of the guides near it, steady and in time.
//!
//! The chain is the one of docs/methods/phase-shifter.md: a drive's power per unit length P′
//! (W/m: heaters drawn as shapes, or a current's Joule heat) gives a temperature rise ΔT(x, y)
//! ([`Heat`], linear in P′); each medium's dn/dT turns it into a change of permittivity
//! Δε = 2n (dn/dT) ΔT; first-order perturbation ([`crate::mode::thermo`], Johnson et al. 2002)
//! turns that into the change of a guide's effective index, Δn_eff = Σ_c w_c ΔT(x_c, y_c), each
//! optical cell's weight w_c the share of its own 1 K rise ([`Guide`]); and over a length L the
//! phase moves by 2π L Δn_eff/λ.
//!
//! Since ΔT ∝ P′, Δn_eff = s P′ with s = ∂n_eff/∂P′ in m/W, and a π shift over L takes
//! P′ = λ/(2 s L), so a power P = P′ L of
//!
//! P_π = λ / (2 s),
//!
//! the same for any length: the length cancels, as Jacques et al. (2019) note (their Section
//! 2.1). It is a 2D number, a straight section's: a device's ends, its bends and leads, and the
//! heat that leaves along the guide are not in it.
//!
//! [`PhaseShifter::efficiency`] gives s for every guide and every drive, so P_π and the
//! crosstalk matrix X_ij = s_ij / s_jj (the share of drive j's own phase that guide i takes);
//! [`PhaseShifter::step`] the phase's step response, whose 1/e and 10–90 % times are the time
//! constants ([`History::crossing`]), and [`PhaseShifter::cutoff`] its −3 dB frequency. The
//! phase in time is the weighted sum of the temperature in time, so its time constant is the
//! mode's, not one point's.

use num_complex::Complex64 as c64;

use crate::electro::Flow;
use crate::geometry::Shape;
use crate::mode::thermo::index_change_density;
use crate::mode::vector::{self, CrossSection, Permittivity};
use crate::statics::spread;
use crate::units::Wavelength;
use crate::{Error, Result};

use crate::thermal::{Assembled, Field, Grid, Heat, History, Section, Stepping};

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("phase shifter", reason)
}

/// The optics of a guide's cross-section: the wavelength, the optical grid's step, and each
/// medium's refractive index and thermo-optic coefficient dn/dT (1/K) at the reference
/// temperature.
#[derive(Clone, Debug, PartialEq)]
pub struct Optics {
    wavelength: Wavelength,
    step_um: f64,
    media: Vec<(String, f64, f64)>,
}

impl Optics {
    /// At `wavelength`, on a uniform optical grid of about `step_um` (µm), no medium yet.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a step that isn't positive and finite.
    pub fn new(wavelength: Wavelength, step_um: f64) -> Result<Optics> {
        if !(step_um.is_finite() && step_um > 0.0) {
            return Err(invalid(format!(
                "the optical grid's step must be positive and finite, got {step_um} um"
            )));
        }
        Ok(Optics {
            wavelength,
            step_um,
            media: Vec::new(),
        })
    }

    /// `medium`'s index and dn/dT, 1/K (a later call for the same medium wins).
    pub fn medium(&mut self, medium: &str, index: f64, dn_dt: f64) -> &mut Optics {
        self.media.retain(|(m, _, _)| m != medium);
        self.media.push((medium.to_owned(), index, dn_dt));
        self
    }

    /// The wavelength.
    pub fn wavelength(&self) -> Wavelength {
        self.wavelength
    }

    fn of(&self, medium: &str) -> Result<(f64, f64)> {
        self.media
            .iter()
            .find(|(m, _, _)| m == medium)
            .map(|&(_, n, d)| (n, d))
            .ok_or_else(|| {
                invalid(format!(
                    "the optics have no index for the medium {medium} (they have {})",
                    self.media
                        .iter()
                        .map(|m| m.0.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
    }
}

/// A guide a heat field acts on: its fundamental mode, and each optical cell's centre (µm, in
/// the heat section's frame) with its weight w_c, how much a rise of 1 K in that cell alone moves
/// the effective index (first-order perturbation, [`index_change_density`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Guide {
    name: String,
    wavelength: Wavelength,
    effective_index: f64,
    pub(crate) cells: Vec<([f64; 2], f64)>,
}

impl Guide {
    /// The guide in `section` over `window` (x and y, µm): `optics`' uniform grid over it (the
    /// window's sides divided into whole cells as near the step as they go), each cell the medium
    /// at its centre with `optics`' index, its dε/dT = 2n dn/dT; the fundamental mode (the
    /// highest effective index) at `optics`' wavelength, the field zero beyond the window.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a window that isn't from a smaller to a larger number, a
    /// medium the optics don't give, or no mode.
    pub fn new(
        name: &str,
        section: &Section,
        window: ([f64; 2], [f64; 2]),
        optics: &Optics,
    ) -> Result<Guide> {
        let ([x0, x1], [y0, y1]) = window;
        if !(x0.is_finite() && x1.is_finite() && y0.is_finite() && y1.is_finite())
            || x0 >= x1
            || y0 >= y1
        {
            return Err(invalid(format!(
                "the guide {name}'s window must go from a smaller to a larger number in x and y"
            )));
        }
        let count = |a: f64, b: f64| ((b - a) / optics.step_um).round().max(1.0) as usize;
        let (nx, ny) = (count(x0, x1), count(y0, y1));
        if nx * ny > 2_000_000 {
            return Err(invalid(format!(
                "the guide {name}'s optical grid would have {} cells: take a longer step",
                nx * ny
            )));
        }
        // each cell's medium, checked once
        let mut slope = Vec::with_capacity(nx * ny);
        let mut fill = Vec::with_capacity(nx * ny);
        for i in 0..nx {
            let x = x0 + (x1 - x0) * (i as f64 + 0.5) / nx as f64;
            for j in 0..ny {
                let y = y0 + (y1 - y0) * (j as f64 + 0.5) / ny as f64;
                let (n, d) = optics.of(section.medium_at(x, y))?;
                fill.push(n * n);
                slope.push(Permittivity::isotropic(faer::c64::new(2.0 * n * d, 0.0)));
            }
        }
        let cs = CrossSection::uniform((x0, x1, nx), (y0, y1, ny), |x, y| {
            let i = (((x - x0) / (x1 - x0) * nx as f64) as usize).min(nx - 1);
            let j = (((y - y0) / (y1 - y0) * ny as f64) as usize).min(ny - 1);
            Permittivity::isotropic(faer::c64::new(fill[i * ny + j], 0.0))
        })?;
        Guide::from_cross_section(name, &cs, &slope, optics.wavelength, [0.0, 0.0])
    }

    /// The guide of the cross-section `cs` (µm, its own frame) whose cells' permittivities
    /// change by `slope` per kelvin (one per cell, in the order of [`CrossSection::cells`]): its
    /// fundamental mode at `wavelength`, its cells placed in the heat section's frame by
    /// `offset` (µm, added to their centres).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a slope per cell of another count, or no mode.
    pub fn from_cross_section(
        name: &str,
        cs: &CrossSection,
        slope: &[Permittivity],
        wavelength: Wavelength,
        offset: [f64; 2],
    ) -> Result<Guide> {
        let mode = vector::modes(cs, wavelength, 1, None)?
            .into_iter()
            .next()
            .ok_or_else(|| invalid(format!("the guide {name} has no mode")))?;
        let weights = index_change_density(&mode, cs, slope)?;
        let (x, y) = (cs.x(), cs.y());
        let ny = y.len() - 1;
        let cells = weights
            .iter()
            .enumerate()
            .filter(|(_, w)| **w != 0.0)
            .map(|(n, &w)| {
                let (i, j) = (n / ny, n % ny);
                (
                    [
                        0.5 * (x[i] + x[i + 1]) + offset[0],
                        0.5 * (y[j] + y[j + 1]) + offset[1],
                    ],
                    w,
                )
            })
            .collect();
        Ok(Guide {
            name: name.to_owned(),
            wavelength,
            effective_index: mode.effective_index().re,
            cells,
        })
    }

    /// The same guide moved `dx` µm across, named `name`: a neighbour of the same cross-section
    /// (its mode's weights moved with it), a probe of the temperature there, its own media not
    /// drawn in the heat problem.
    pub fn moved(&self, name: &str, dx: f64) -> Guide {
        Guide {
            name: name.to_owned(),
            cells: self
                .cells
                .iter()
                .map(|&([x, y], w)| ([x + dx, y], w))
                .collect(),
            ..self.clone()
        }
    }

    /// The guide's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The wavelength.
    pub fn wavelength(&self) -> Wavelength {
        self.wavelength
    }

    /// The mode's effective index at the reference temperature.
    pub fn effective_index(&self) -> f64 {
        self.effective_index
    }

    /// Each optical cell that responds: its centre (µm, in the heat section's frame) and its
    /// weight w_c, the change of the effective index a rise of 1 K in that cell alone gives (a
    /// cell of a medium with no dn/dT has none and isn't listed). On the optics' uniform grid,
    /// so a weight is also its cell's density: where the mode takes its phase from the heat.
    pub fn weights(&self) -> &[([f64; 2], f64)] {
        &self.cells
    }

    /// dn_eff/dT for a uniform rise, 1/K: the sum of the weights.
    pub fn thermo_optic(&self) -> f64 {
        self.cells.iter().map(|c| c.1).sum()
    }

    /// The change of the effective index a temperature rise `rise` gives (K above the
    /// reference, as [`Field::at`] interpolates it at each optical cell's centre), to first
    /// order.
    pub fn index_change(&self, rise: &Field) -> f64 {
        self.cells
            .iter()
            .map(|&([x, y], w)| w * rise.at(x, y))
            .sum()
    }

    /// The weights on `grid`'s cells that give [`Guide::index_change`] from a field's values:
    /// each optical cell's weight shared among the cells [`Field::at`] interpolates it from.
    fn on(&self, grid: &Grid) -> Vec<(usize, f64)> {
        let mut dense = vec![0.0; grid.cells()];
        for &([x, y], w) in &self.cells {
            for (c, b) in grid.bilinear(x, y) {
                dense[c] += w * b;
            }
        }
        dense
            .into_iter()
            .enumerate()
            .filter(|(_, w)| *w != 0.0)
            .collect()
    }
}

/// What heats a phase shifter: heaters drawn as shapes, each with its power per unit length,
/// or a current's Joule heat, cell by cell; all of them turned on and off together.
#[derive(Clone, Debug, PartialEq)]
pub struct Drive {
    name: String,
    shapes: Vec<(Shape, f64)>,
    cells: Option<Vec<f64>>,
}

impl Drive {
    /// A heater: `watts_per_metre` spread over `shape` by area.
    pub fn heater(name: &str, shape: Shape, watts_per_metre: f64) -> Drive {
        Drive {
            name: name.to_owned(),
            shapes: vec![(shape, watts_per_metre)],
            cells: None,
        }
    }

    /// Another heater driven with this one (a doped heater's two strips).
    pub fn and(mut self, shape: Shape, watts_per_metre: f64) -> Drive {
        self.shapes.push((shape, watts_per_metre));
        self
    }

    /// The Joule heat of `flow`, a current solved on the heat problem's grid.
    pub fn joule(name: &str, flow: &Flow) -> Drive {
        Drive {
            name: name.to_owned(),
            shapes: Vec::new(),
            cells: Some(flow.joule().to_vec()),
        }
    }

    /// The drive's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Its power per unit length, W/m.
    pub fn power(&self) -> f64 {
        self.shapes.iter().map(|s| s.1).sum::<f64>()
            + self.cells.as_ref().map_or(0.0, |c| c.iter().sum())
    }
}

/// A phase shifter's cross-section: a heat problem, what drives it, and the guides it acts on.
///
/// The heat problem's own heaters are replaced by the drives, one at a time; a current's Joule
/// heat it holds ([`Heat::joule`]) and its fixed and convective sides stay as they are, a
/// steady background the drives' rises are taken above (the problem is linear).
#[derive(Clone, Debug)]
pub struct PhaseShifter {
    heat: Heat,
    drives: Vec<Drive>,
    guides: Vec<Guide>,
    weights: Vec<Vec<(usize, f64)>>,
}

/// Each guide's response to each drive: s_ij = ∂n_eff,i/∂P′_j, in m/W.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Efficiency {
    /// The wavelength, µm (the first guide's).
    pub wavelength_um: f64,
    /// `slopes[i][j]`: guide i's change of effective index per W/m of drive j.
    pub slopes: Vec<Vec<f64>>,
}

impl Efficiency {
    /// The power (W) of drive `drive` that shifts guide `guide`'s phase by π over any length of
    /// a straight section: λ/(2 s). A 2D number. Infinite if the guide doesn't respond.
    pub fn p_pi(&self, guide: usize, drive: usize) -> f64 {
        self.wavelength_um * 1e-6 / (2.0 * self.slopes[guide][drive].abs())
    }

    /// The phase (rad) `watts` of drive `drive` gives guide `guide`, over any length:
    /// 2π s P/λ.
    pub fn phase(&self, guide: usize, drive: usize, watts: f64) -> f64 {
        2.0 * std::f64::consts::PI * self.slopes[guide][drive] * watts / (self.wavelength_um * 1e-6)
    }

    /// The crosstalk of drive `drive` on guide `victim`: the share of the phase it gives its
    /// own guide `own` that `victim` takes, s_victim,drive / s_own,drive.
    pub fn crosstalk(&self, victim: usize, drive: usize, own: usize) -> f64 {
        self.slopes[victim][drive] / self.slopes[own][drive]
    }
}

impl PhaseShifter {
    /// `heat` driven by `drives`, acting on `guides`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no drive or no guide, a drive with no power or a power that
    /// isn't finite, or a current's Joule heat on another grid.
    pub fn new(heat: Heat, drives: Vec<Drive>, guides: Vec<Guide>) -> Result<PhaseShifter> {
        if drives.is_empty() || guides.is_empty() {
            return Err(invalid("a phase shifter needs a drive and a guide"));
        }
        for d in &drives {
            let p = d.power();
            if !(p.is_finite() && p != 0.0) {
                return Err(invalid(format!(
                    "the drive {} must have a finite power, not {p} W/m",
                    d.name
                )));
            }
            if d.cells
                .as_ref()
                .is_some_and(|c| c.len() != heat.grid.cells())
            {
                return Err(invalid(format!(
                    "the drive {}'s Joule heat is on another grid than the heat problem's",
                    d.name
                )));
            }
        }
        let weights = guides.iter().map(|g| g.on(&heat.grid)).collect();
        Ok(PhaseShifter {
            heat,
            drives,
            guides,
            weights,
        })
    }

    /// The heat problem.
    pub fn heat(&self) -> &Heat {
        &self.heat
    }

    /// The drives.
    pub fn drives(&self) -> &[Drive] {
        &self.drives
    }

    /// The guides.
    pub fn guides(&self) -> &[Guide] {
        &self.guides
    }

    /// The heat problem with drive `j` as its heaters.
    fn driven(&self, j: usize) -> Heat {
        let d = &self.drives[j];
        let mut h = self.heat.clone();
        h.heaters = d.shapes.clone();
        h.driven = d.cells.iter().cloned().collect();
        h
    }

    /// Drive `j`'s temperature rise at 1 W/m above the background (K per W/m).
    ///
    /// # Errors
    ///
    /// As [`Heat::solve`].
    pub fn rise(&self, drive: usize) -> Result<Field> {
        Ok(self.rises()?.swap_remove(drive))
    }

    /// Every drive's rise, the problem factorized once.
    fn rises(&self) -> Result<Vec<Field>> {
        let mut h = self.heat.clone();
        h.heaters.clear();
        h.driven.clear();
        let Assembled {
            discrete,
            base,
            values,
            ..
        } = h.discretize()?;
        let background = discrete.solve(&values, &base)?;
        self.drives
            .iter()
            .map(|d| {
                let mut sources = base.clone();
                for (shape, p) in &d.shapes {
                    spread(&self.heat.grid, shape, *p, &mut sources)?;
                }
                if let Some(cells) = &d.cells {
                    for (s, q) in sources.iter_mut().zip(cells) {
                        *s += q;
                    }
                }
                let on = discrete.solve(&values, &sources)?;
                Ok(on.above(&background, d.power()))
            })
            .collect()
    }

    /// Every guide's response to every drive.
    ///
    /// # Errors
    ///
    /// As [`Heat::solve`].
    pub fn efficiency(&self) -> Result<Efficiency> {
        let rises = self.rises()?;
        let slopes = self
            .weights
            .iter()
            .map(|w| {
                rises
                    .iter()
                    .map(|r| w.iter().map(|&(c, v)| v * r.values()[c]).sum())
                    .collect()
            })
            .collect();
        Ok(Efficiency {
            wavelength_um: self.guides[0].wavelength.to_um(),
            slopes,
        })
    }

    /// Drive `drive` turned on (`on`), from the background, or off, from its steady state, at
    /// its own power: each guide's effective index (less the reference's, background included)
    /// at every step of `stepping`, as [`History::watched`]. A guide's 1/e time is
    /// [`History::crossing`] at 1 − 1/e from its first value to its steady end.
    ///
    /// # Errors
    ///
    /// As [`Heat::transient`].
    pub fn step(&self, drive: usize, stepping: &Stepping, on: bool) -> Result<History> {
        let (from, to) = if on { (0.0, 1.0) } else { (1.0, 0.0) };
        self.driven(drive)
            .transient_weighted(stepping, from, &|_| to, &self.weights, &mut |_| Ok(true))
    }

    /// Guide `guide`'s steady index change at the end of [`PhaseShifter::step`], `on` or off,
    /// background included: where its record goes.
    ///
    /// # Errors
    ///
    /// As [`Heat::solve`].
    pub fn steady(&self, drive: usize, guide: usize, on: bool) -> Result<f64> {
        let field = self.driven(drive).steady_at(if on { 1.0 } else { 0.0 })?;
        Ok(self.weights[guide]
            .iter()
            .map(|&(c, v)| v * field.values()[c])
            .sum())
    }

    /// Drive `drive`'s step response of guide `guide`'s phase: its 1/e time and its 10–90 %
    /// time, seconds, turned `on` or off.
    ///
    /// # Errors
    ///
    /// As [`PhaseShifter::step`], and [`Error::InvalidValue`] if the phase doesn't get 90 % of
    /// the way within the steps.
    pub fn times(
        &self,
        drive: usize,
        guide: usize,
        stepping: &Stepping,
        on: bool,
    ) -> Result<Timing> {
        let history = self.step(drive, stepping, on)?;
        self.timing(drive, guide, &history, on)
    }

    /// Guide `guide`'s times in `history`, drive `drive`'s step response turned `on` or off
    /// ([`PhaseShifter::step`], [`PhaseShifter::step_with_fields`]): what
    /// [`PhaseShifter::times`] gives of the same history.
    ///
    /// # Errors
    ///
    /// As [`PhaseShifter::steady`], and [`Error::InvalidValue`] if the phase doesn't get 90 % of
    /// the way within the history.
    pub fn timing(
        &self,
        drive: usize,
        guide: usize,
        history: &History,
        on: bool,
    ) -> Result<Timing> {
        let end = self.steady(drive, guide, on)?;
        let start = history.watched[guide][0];
        let at = |f: f64| {
            history.crossing(guide, start, end, f).ok_or_else(|| {
                invalid(format!(
                    "the guide {}'s phase doesn't get {}% of the way in {} s: take more steps",
                    self.guides[guide].name,
                    f * 100.0,
                    history.times_s.last().copied().unwrap_or(0.0)
                ))
            })
        };
        let e = at(1.0 - (-1.0f64).exp())?;
        let (a, b) = (at(0.1)?, at(0.9)?);
        Ok(Timing {
            one_over_e_s: e,
            ten_ninety_s: b - a,
            ten_s: a,
            ninety_s: b,
        })
    }

    /// Drive `drive` turned on from the background, as [`PhaseShifter::step`] (the same
    /// history, to the bit), with its rise above the background (K per W/m, as
    /// [`PhaseShifter::rise`]) at each step number of `at` (0 the start, where it is zero), in
    /// `at`'s order.
    ///
    /// # Errors
    ///
    /// As [`PhaseShifter::step`], and [`Error::InvalidValue`] for a step number past the
    /// steps.
    pub fn step_with_fields(
        &self,
        drive: usize,
        stepping: &Stepping,
        at: &[usize],
    ) -> Result<(History, Vec<Field>)> {
        if let Some(k) = at.iter().find(|&&k| k > stepping.steps) {
            return Err(invalid(format!(
                "step {k} is past the transient's {} steps",
                stepping.steps
            )));
        }
        let heat = self.driven(drive);
        let background = if at.is_empty() {
            None
        } else {
            Some(heat.steady_at(0.0)?)
        };
        let power = self.drives[drive].power();
        let mut kept: Vec<(usize, Field)> = Vec::new();
        if let Some(b) = &background
            && at.contains(&0)
        {
            kept.push((0, b.above(b, power)));
        }
        let history =
            heat.transient_weighted(stepping, 0.0, &|_| 1.0, &self.weights, &mut |step| {
                if let Some(b) = &background
                    && at.contains(&step.index)
                {
                    kept.push((step.index, step.field().above(b, power)));
                }
                Ok(true)
            })?;
        let fields = at
            .iter()
            .map(|k| {
                kept.iter()
                    .find(|(i, _)| i == k)
                    .map(|(_, f)| f.clone())
                    .ok_or_else(|| invalid(format!("the transient stopped before step {k}")))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok((history, fields))
    }

    /// The −3 dB frequency of guide `guide`'s phase in response to drive `drive`'s power, Hz
    /// (as [`Heat::cutoff`]).
    ///
    /// # Errors
    ///
    /// As [`Heat::cutoff`].
    pub fn cutoff(&self, drive: usize, guide: usize) -> Result<f64> {
        self.driven(drive).cutoff_weighted(&self.weights[guide])
    }

    /// Guide `guide`'s phase's small-signal response to drive `drive`'s power at
    /// `frequency_hz`, over its steady response (1 at 0 Hz): (K + iωM) T̂ = Q̂ on the heat's
    /// grid, read by the guide's weights.
    ///
    /// # Errors
    ///
    /// As [`Heat::response`].
    pub fn response(&self, drive: usize, guide: usize, frequency_hz: f64) -> Result<c64> {
        self.driven(drive).responder(&self.weights[guide])?(frequency_hz)
    }

    /// Every drive's step response, turned on from the background: each guide's phase per W
    /// of the drive's power (rad/W, over any length of a straight section: 2π s P/λ), at every
    /// time of `stepping` from 0, and its steady value. What [`crate::thermal::RcModel::fit`] takes.
    /// The drives step in parallel, the same bits on any number of threads.
    ///
    /// # Errors
    ///
    /// As [`PhaseShifter::step`] and [`PhaseShifter::efficiency`].
    pub fn step_responses(&self, stepping: &Stepping) -> Result<StepResponses> {
        use rayon::prelude::*;
        let histories = (0..self.drives.len())
            .into_par_iter()
            .map(|j| self.step(j, stepping, true))
            .collect::<Result<Vec<_>>>()?;
        let per =
            |i: usize| 2.0 * std::f64::consts::PI / (self.guides[i].wavelength.to_um() * 1e-6);
        let steps = histories
            .iter()
            .zip(&self.drives)
            .map(|(h, d)| {
                h.watched
                    .iter()
                    .enumerate()
                    .map(|(i, w)| w.iter().map(|v| (v - w[0]) * per(i) / d.power()).collect())
                    .collect()
            })
            .collect();
        let slopes = self.efficiency()?.slopes;
        let steady = slopes
            .iter()
            .enumerate()
            .map(|(i, row)| row.iter().map(|s| s * per(i)).collect())
            .collect();
        Ok(StepResponses {
            times_s: histories[0].times_s.clone(),
            steps,
            steady,
        })
    }
}

/// Every drive's step response of every guide's phase ([`PhaseShifter::step_responses`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StepResponses {
    /// The times, seconds, from 0.
    pub times_s: Vec<f64>,
    /// `steps[drive][guide][time]`: the guide's phase per W of the drive, rad/W, 0 at 0.
    pub steps: Vec<Vec<Vec<f64>>>,
    /// `steady[guide][drive]`: its steady value, rad/W.
    pub steady: Vec<Vec<f64>>,
}

/// A step response's times.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Timing {
    /// When the phase is 1 − 1/e of the way, seconds.
    pub one_over_e_s: f64,
    /// From 10 % to 90 % of the way, seconds: `ninety_s − ten_s`.
    pub ten_ninety_s: f64,
    /// When it is 10 % of the way, seconds.
    pub ten_s: f64,
    /// When it is 90 % of the way, seconds.
    pub ninety_s: f64,
}
