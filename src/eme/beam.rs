//! A beam launched into a structure: a Gaussian across the first section's plane, tilted by an
//! angle, as amplitudes of that section's forward modes ([`Eme::launch`]).
//!
//! The beam is its field across the plane, E_y for TE and H_y for TM:
//! f(x) = exp(−(x − c)²/w²) exp(i k n sin θ (x − c)), a Gaussian of waist w centred at c whose
//! phase fronts are tilted by θ in a medium of index n. Its angular spectrum is a Gaussian
//! centred at k n sin θ, so in a uniform section, whose modes between two walls are plane waves
//! at discrete angles, the expansion samples that spectrum: the modes are a basis of plane waves,
//! and each one meets an interface between uniform media at its own angle with Fresnel's
//! coefficient.
//!
//! The amplitudes are the projections that make the forward field's transverse component on
//! the plane the beam's: for TE a_m = ⟨E_in, H_m⟩ = ∫ E_in h_m dx̃, for TM a_m = ⟨E_m, H_in⟩ =
//! ∫ e_m H_in dx̃, exact by the modes' unconjugated orthogonality ⟨E_m, H_n⟩ = δ_mn. With every
//! mode the field is the beam's; with fewer, its projection on them.

use num_complex::Complex64 as c64;

use super::scattering::GL20;
use super::{Eme, Mode, Scattering, Slice};
use crate::mode::Polarization;
use crate::portable;
use crate::units::Length;
use crate::{Error, Result};

/// How far from its centre the beam is integrated, in waists: exp(−81) is 6.6e-36 of its peak.
const REACH: f64 = 9.0;

/// The widest phase a 20-point panel integrates, as the interfaces' overlaps do.
const PANEL_PHASE: f64 = 10.0;

/// A Gaussian beam across a section's plane, tilted by an angle: its field there is
/// exp(−(x − c)²/w²) exp(i k n sin θ (x − c)) ([`Eme::launch`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beam {
    /// The waist w, µm: the field falls to 1/e at w from the centre.
    waist: f64,
    /// The centre c, µm from the window's middle (PMLs excluded).
    centre: f64,
    /// The tilt θ of its phase fronts, radians, positive toward +x.
    angle: f64,
    /// The index of the medium the tilt is in: its phase across the plane is k n sin θ.
    index: f64,
}

impl Beam {
    /// A Gaussian of `waist` centred `centre` from the window's middle, its phase fronts across
    /// the plane (θ = 0).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a waist that isn't positive and finite, or a centre that
    /// isn't finite.
    pub fn gaussian(waist: Length, centre: Length) -> Result<Beam> {
        let (w, c) = (waist.to_um(), centre.to_um());
        if !(w.is_finite() && w > 0.0 && c.is_finite()) {
            return Err(Error::invalid(
                "eme beam",
                format!("needs a positive waist and a finite centre, got {w} and {c} um"),
            ));
        }
        Ok(Beam {
            waist: w,
            centre: c,
            angle: 0.0,
            index: 1.0,
        })
    }

    /// The same tilted by `angle` radians (positive toward +x) in a medium of `index`: its phase
    /// across the plane grows as k n sin θ.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an angle that isn't finite or reaches ±π/2, or an index
    /// that isn't positive and finite.
    pub fn tilted(self, angle: f64, index: f64) -> Result<Beam> {
        if !(index.is_finite() && index > 0.0) {
            return Err(Error::invalid(
                "eme beam",
                format!("its medium's index must be positive, got {index}"),
            ));
        }
        if !(angle.is_finite() && angle.abs() < std::f64::consts::FRAC_PI_2) {
            return Err(Error::invalid(
                "eme beam",
                format!(
                    "its angle must be within ±90°, got {} degrees",
                    angle.to_degrees()
                ),
            ));
        }
        Ok(Beam {
            angle,
            index,
            ..self
        })
    }

    /// The waist.
    pub fn waist(&self) -> Length {
        Length::um(self.waist)
    }

    /// The centre, from the window's middle.
    pub fn centre(&self) -> Length {
        Length::um(self.centre)
    }

    /// The tilt, radians.
    pub fn angle(&self) -> f64 {
        self.angle
    }

    /// The index of the medium the tilt is in (1 for a beam that isn't tilted).
    pub fn index(&self) -> f64 {
        self.index
    }

    /// Its field at `x` µm from the window's middle at a free-space wavenumber `k` (1/µm):
    /// E_y for TE, H_y for TM, 1 at its centre.
    pub fn field(&self, x: f64, k: f64) -> c64 {
        let t = (x - self.centre) / self.waist;
        let phase = k * self.index * portable::sin(self.angle) * (x - self.centre);
        portable::complex::from_polar(portable::exp(-t * t), phase)
    }

    /// ∫ |f|² dx across an unbounded plane, µm: w √(π/2).
    pub fn norm(&self) -> f64 {
        self.waist * (0.5 * std::f64::consts::PI).sqrt()
    }
}

/// A beam launched on a section's plane: its forward modes' amplitudes.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Launched {
    /// The amplitude of each mode, in the section's order ([`Eme::modes`]): a structure's
    /// S-matrix from that section takes them as they are ([`Scattering::forward`]).
    pub amplitudes: Vec<c64>,
    /// The modes' effective indices, in the same order.
    pub effective_indices: Vec<c64>,
    /// ∫ w(x)|f|² dx across the window (w = 1 for TE, 1/n² for TM): the beam's own norm, which
    /// [`Launched::couplings`] divides by.
    pub norm: f64,
}

/// Whether a mode carries power along z in a lossless window: its index real and positive.
fn propagating(n: c64) -> bool {
    n.re > 0.0 && n.im.abs() <= 1e-9 * n.re
}

/// The power the modes of effective indices `indices` carry with amplitudes `amplitudes`, in
/// units of one mode's: Σ |a_m|² over the propagating modes (a real positive index). Exact in a
/// window without loss or PML, where those modes are orthonormal in power and the evanescent
/// ones carry none.
pub fn power(amplitudes: &[c64], indices: &[c64]) -> f64 {
    amplitudes
        .iter()
        .zip(indices)
        .filter(|(_, n)| propagating(**n))
        .map(|(a, _)| a.norm_sqr())
        .sum()
}

impl Launched {
    /// The amplitudes of a beam that arrives from afar: its propagating modes' alone, the
    /// evanescent ones 0 (they decay away from the plane they are launched on, and would put
    /// into a structure power the beam doesn't carry).
    pub fn arriving(&self) -> Vec<c64> {
        self.amplitudes
            .iter()
            .zip(&self.effective_indices)
            .map(|(&a, &n)| {
                if propagating(n) {
                    a
                } else {
                    c64::new(0.0, 0.0)
                }
            })
            .collect()
    }

    /// The power its propagating modes carry, in units of one mode's ([`power`]).
    pub fn power(&self) -> f64 {
        power(&self.amplitudes, &self.effective_indices)
    }

    /// Each mode's overlap with the beam, the share of the beam's power it takes where the
    /// impedances match: |∫ f U_m w dx|² / (∫ |f|² w dx ∫ U_m² w dx) for a real mode
    /// (w = 1 for TE, 1/n² for TM), that is |a_m|² / (Re n_eff ∫ |f|² w dx); 0 for a mode with
    /// a complex index.
    pub fn couplings(&self) -> Vec<f64> {
        self.amplitudes
            .iter()
            .zip(&self.effective_indices)
            .map(|(a, n)| {
                if propagating(*n) {
                    a.norm_sqr() / (n.re * self.norm)
                } else {
                    0.0
                }
            })
            .collect()
    }
}

impl Scattering {
    /// The amplitudes leaving the structure for forward amplitudes `a` on port 1's modes (a
    /// [`Launched`] beam's, say): reflected at port 1, S₁₁ a, and transmitted at port 2, S₂₁ a.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for amplitudes other than port 1's modes in number.
    pub fn forward(&self, a: &[c64]) -> Result<(Vec<c64>, Vec<c64>)> {
        if a.len() != self.first.len() {
            return Err(Error::invalid(
                "eme forward",
                format!(
                    "needs {} amplitudes, one for each of port 1's modes, got {}",
                    self.first.len(),
                    a.len()
                ),
            ));
        }
        let apply = |s: &[Vec<c64>]| -> Vec<c64> {
            s.iter()
                .map(|row| row.iter().zip(a).map(|(s, a)| s * a).sum())
                .collect()
        };
        Ok((apply(&self.s11), apply(&self.s21)))
    }
}

impl Eme {
    /// `beam` launched on `slice`'s plane: the amplitudes of the slice's forward modes
    /// ([`Eme::modes`]) whose transverse field there is the beam's (its E_y for TE, its H_y for
    /// TM).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a bent slice, a beam centred outside the window (PMLs
    /// excluded), and [`Eme::modes`]'s.
    pub fn launch(&self, slice: &Slice, beam: &Beam) -> Result<Launched> {
        if slice.radius.is_some() {
            return Err(Error::invalid(
                "eme launch",
                "a beam is launched on a straight section's plane, not a bend's",
            ));
        }
        let width = slice.width().to_um();
        if beam.centre.abs() >= 0.5 * width {
            return Err(Error::invalid(
                "eme launch",
                format!(
                    "the beam's centre, {} um from the middle, is outside the window {width} um \
                     wide",
                    beam.centre
                ),
            ));
        }
        let modes = self.modes(slice)?;
        // the window's middle in the modes' coordinate, from the lower wall's PML
        let lower = self.pml.filter(|p| p.lower).map_or(0.0, |p| p.thickness);
        let middle = lower + 0.5 * width;
        let layers = &modes[0].layers;
        let k = 2.0 * std::f64::consts::PI / self.wavelength.to_um();
        let tm = self.polarization == Polarization::Tm;
        // the beam's reach, in the modes' coordinate
        let (lo, hi) = (
            middle + beam.centre - REACH * beam.waist,
            middle + beam.centre + REACH * beam.waist,
        );
        let tilt = (k * beam.index * portable::sin(beam.angle)).abs();
        let mut amplitudes = vec![c64::new(0.0, 0.0); modes.len()];
        let mut norm = 0.0;
        for (j, layer) in layers.iter().enumerate() {
            let (a, b) = (layer.x0.max(lo), (layer.x0 + layer.width).min(hi));
            if b <= a {
                continue;
            }
            let fastest = modes.iter().map(|m| m.wavenumber(j)).fold(0.0, f64::max);
            let omega = (fastest + tilt) * portable::complex::abs(layer.stretch);
            // panels no wider than the phase allows, nor than half a waist
            let panels = ((omega * (b - a) / PANEL_PHASE)
                .max(2.0 * (b - a) / beam.waist)
                .ceil() as usize)
                .max(1);
            let h = (b - a) / panels as f64;
            let weight_of = |l: &super::modes::Layer| if tm { 1.0 / l.phys.re } else { 1.0 };
            for p in 0..panels {
                let centre = a + (p as f64 + 0.5) * h;
                for &(node, weight) in &GL20 {
                    for x in [centre - 0.5 * h * node, centre + 0.5 * h * node] {
                        let f = beam.field(x - middle, k);
                        let w = layer.stretch * (0.5 * h * weight);
                        norm += (0.5 * h * weight) * weight_of(layer) * f.norm_sqr();
                        for (m, mode) in modes.iter().enumerate() {
                            let (e, hm) = mode.e_h(j, x);
                            amplitudes[m] += w * f * if tm { e } else { hm };
                        }
                    }
                }
            }
        }
        Ok(Launched {
            effective_indices: modes.iter().map(Mode::effective_index).collect(),
            amplitudes,
            norm,
        })
    }
}
