//! Eigenmode expansion: a structure cut along its length into uniform sections, the field in
//! each written as that section's modes, and the sections joined by their S-matrices.
//!
//! P. Bienstman, R. Baets, "Optical modelling of photonic crystals and VCSELs using eigenmode
//! expansion and perfectly matched layers", Opt. Quantum Electron. 33, 327 (2001),
//! [doi:10.1023/A:1010882531238](https://doi.org/10.1023/A:1010882531238).
//!
//! This is the 2D method, for slab stacks: the structure is uniform along y, light travels
//! along z, and each section is a stack of layers across x ([`Slice`]), between two walls
//! ([`Wall`]), with a perfectly matched layer inside each wall if asked ([`Eme::with_pml`]).
//! TE has E along y, TM has H along y; the time convention is photonoxide's, e^(−iωt), so a
//! mode goes as e^(i(βz − ωt)) and a lossy one has Im n_eff > 0.
//!
//! - **The modes of a section** ([`Eme::modes`]) are the closed stack's: the field U (E_y for TE,
//!   H_y for TM) and P = (w/k) dU/dx, with w = 1 for TE and 1/n² for TM, are continuous, and a
//!   layer carries (U, P) by [[cos Φ, sin Φ/γ], [−γ sin Φ, cos Φ]], Φ = kαd, γ = wα,
//!   α² = n² − n_eff² (Chilwell and Hodgkinson's Eq. 10 in the form [`crate::mode::multilayer`]
//!   uses). Every entry is a function of α² alone, so the walls' condition (U = 0 or P = 0) is
//!   an entire function of n_eff²: no branch cuts. Without a PML the problem is a regular
//!   Sturm–Liouville one, whose eigenvalues n_eff² are real and simple, and the m-th has m
//!   zeros: the first N are found by bisection on the count of zeros (Prüfer's angle), with
//!   none missed. A **PML** is Bienstman and Baets's Section 2.2: a layer of complex thickness
//!   d(1 + iσ) inside a wall (Chew, Jin and Michielssen 1997), the same formulas with a
//!   complex d. With a PML (or a lossy layer) the modes start from those of the window filled
//!   with one medium, PML and all, whose kαW̃ are multiples of π/2, and are followed as each
//!   layer's permittivity goes from that medium's to its own (Newton's method on the
//!   Wronskian of the states swept from the two walls, met where both have grown most, with
//!   steps that halve where a correction strays): the high modes hardly move, a guided mode
//!   rises out of the window's, and the radiation modes become the PML's absorbing ones, whose
//!   Im n_eff > 0 (their Fig. 2, in their e^(+jωt)'s conjugate). A PML's modes are
//!   ill-conditioned (the operator is far from normal), the more so the stronger the PML and
//!   the higher the mode: a thick PML of small σ keeps them tractable; where one can't be
//!   followed, [`Eme::modes`] says so rather than return a wrong one. Half of a symmetric
//!   structure, its plane of symmetry a wall ([`Eme::with_upper_pml`]), needs half the modes.
//! - **The field** in each section is Σ_k (A⁺_k e^(iβ_k z) + A⁻_k e^(−iβ_k z)) of its modes, the
//!   tangential E the same going either way and the tangential H reversed (their Eq. 1). Each
//!   mode is normalized by the unconjugated product of their Eq. 4, ⟨E_k, H_k⟩ = ∫ (E_k × H_k)·ẑ
//!   dx = 1, taken along the complex coordinate in a PML, where the modes of one section are
//!   orthogonal in it even when lossy. E is in units of Z₀ times H's: H_x = −n_eff E_y for TE,
//!   E_x = (n_eff/n²) H_y for TM.
//! - **At an interface** between sections I and II the tangential E and H are continuous (their
//!   Eq. 2). Here E's continuity is tested with section II's H_j and H's with section I's E_i;
//!   with O_ij = ⟨E_i^I, H_j^II⟩ that gives Oᵀ(A⁺ + A⁻)^I = (A⁺ + A⁻)^II and
//!   (A⁺ − A⁻)^I = O (A⁺ − A⁻)^II, an ideal transformer between the two sets of modes. Light
//!   from section I: R = (1 + OOᵀ)⁻¹(1 − OOᵀ), T = 2Oᵀ(1 + OOᵀ)⁻¹; from section II:
//!   R' = (1 + OᵀO)⁻¹(OᵀO − 1), T' = 2O(1 + OᵀO)⁻¹. Their Eq. 3 tests both with section I's
//!   modes instead; the two agree as the number of modes grows, but this one keeps S symmetric
//!   (T' = Tᵀ, R and R' symmetric) for any number of modes, and, in a lossless guide without a
//!   PML, conserves power exactly: the complex power through the interface is the same on both
//!   sides. The overlaps are Gauss–Legendre sums on each piece between the two sections' layer
//!   boundaries, to round-off.
//! - **The structure's S-matrix** combines the interfaces' and the sections' propagation,
//!   diag(e^(iβ_k L)), by the scattering-matrix recursion of their Eqs. 6–7 (Redheffer's star
//!   product; L. Li 1996), stable for evanescent modes since only decaying exponentials appear.
//!
//! **S-parameters** are in the modes' amplitudes: |S_qp|² is the share of power from mode p into
//! mode q for a real mode, and S's reflections are the tangential E's, as the FDFD ports'
//! ([`crate::fdfd::PortMode`]): the conventions are the same, so the two compare directly.
//!
//! Not here yet: 3D cross-sections, sections drawn from a device's geometry or a job's shapes,
//! adaptive steps along a taper, and S over wavelength (#432).

use num_complex::Complex64 as c64;

use crate::mode::Polarization;
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

#[path = "cases/eme/checks.rs"]
pub(crate) mod checks;
mod modes;
mod scattering;

pub use modes::Mode;

/// A wall at an edge of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Wall {
    /// A perfect electric conductor: the tangential E is zero (E_y for TE, E_z for TM).
    Electric,
    /// A perfect magnetic conductor: the tangential H is zero (H_z for TE, H_y for TM).
    Magnetic,
}

/// A cross-section uniform along z: its layers across x, from the lower wall to the upper,
/// each an index (n + iκ, κ ≥ 0 for loss) and a thickness.
#[derive(Clone, Debug, PartialEq)]
pub struct Slice {
    layers: Vec<(c64, f64)>,
}

impl Slice {
    /// The layers (index, thickness), from the lower wall.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no layers, a thickness that isn't positive and finite, or an
    /// index that isn't finite or has a real part that isn't positive.
    pub fn new(layers: &[(c64, Length)]) -> Result<Slice> {
        if layers.is_empty() {
            return Err(Error::invalid("eme slice", "needs at least one layer"));
        }
        let mut out = Vec::with_capacity(layers.len());
        for &(n, d) in layers {
            let d = d.to_um();
            if !(d.is_finite() && d > 0.0) {
                return Err(Error::invalid(
                    "eme slice",
                    format!("a layer's thickness must be positive, got {d} um"),
                ));
            }
            if !(n.re.is_finite() && n.im.is_finite() && n.re > 0.0) {
                return Err(Error::invalid(
                    "eme slice",
                    format!("an index must be finite with a positive real part, got {n}"),
                ));
            }
            // neighbours of the same index are one layer
            match out.last_mut() {
                Some((m, t)) if *m == n => *t += d,
                _ => out.push((n, d)),
            }
        }
        Ok(Slice { layers: out })
    }

    /// A slab: `thickness` of `core` centred in `cladding`, the window `width` wide in all.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] as [`Slice::new`], and for a core no thinner than the window.
    pub fn slab(core: c64, thickness: Length, cladding: c64, width: Length) -> Result<Slice> {
        let (t, w) = (thickness.to_um(), width.to_um());
        if t.partial_cmp(&w) != Some(std::cmp::Ordering::Less) {
            return Err(Error::invalid(
                "eme slice",
                format!("the core ({t} um) must be thinner than the window ({w} um)"),
            ));
        }
        let side = Length::um(0.5 * (w - t));
        Slice::new(&[(cladding, side), (core, thickness), (cladding, side)])
    }

    /// The layers, (index, thickness in µm), from the lower wall.
    pub fn layers(&self) -> &[(c64, f64)] {
        &self.layers
    }

    /// The window's width, wall to wall (inside any PML).
    pub fn width(&self) -> Length {
        Length::um(self.layers.iter().map(|&(_, d)| d).sum())
    }
}

/// A section of a structure: a slice and its length along z.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    /// The cross-section.
    pub slice: Slice,
    /// Its length along z; 0 for a port's reference plane at an interface.
    pub length: Length,
}

impl Section {
    /// A section of `slice`, `length` long.
    pub fn new(slice: Slice, length: Length) -> Section {
        Section { slice, length }
    }

    /// A structure `length` long whose cross-section `slice_at(z)` changes along z (a taper),
    /// cut into `steps` uniform sections of equal length, each the cross-section at its middle.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no steps or a length that isn't positive, and `slice_at`'s.
    pub fn sampled(
        length: Length,
        steps: usize,
        slice_at: impl Fn(Length) -> Result<Slice>,
    ) -> Result<Vec<Section>> {
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
                slice_at(Length::um((s as f64 + 0.5) * h)).map(|slice| Section {
                    slice,
                    length: Length::um(h),
                })
            })
            .collect()
    }
}

/// A perfectly matched layer inside each wall: a layer `thickness` thick of the outermost
/// layer's index, its thickness made complex, d(1 + iσ) with σ its strength.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pml {
    /// Its thickness, µm.
    pub thickness: f64,
    /// σ: the imaginary part of its thickness over the real part.
    pub strength: f64,
    /// Whether it lines the lower wall (not where that wall is a plane of symmetry).
    pub lower: bool,
}

/// An eigenmode expansion's settings: the polarization, the wavelength, how many modes each
/// section keeps, its walls and its PML.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Eme {
    polarization: Polarization,
    wavelength: Wavelength,
    modes: usize,
    walls: [Wall; 2],
    pml: Option<Pml>,
}

/// A structure's S-matrix between the modes of its first section (port 1) and its last (port
/// 2), at the structure's two ends: `s21[q][p]` is mode q's amplitude leaving port 2 for mode p
/// entering port 1 at unit amplitude.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Scattering {
    /// Reflection at port 1.
    pub s11: Vec<Vec<c64>>,
    /// Transmission from port 2 to port 1.
    pub s12: Vec<Vec<c64>>,
    /// Transmission from port 1 to port 2.
    pub s21: Vec<Vec<c64>>,
    /// Reflection at port 2.
    pub s22: Vec<Vec<c64>>,
    /// The effective indices of the first section's modes, in their order.
    pub first: Vec<c64>,
    /// The effective indices of the last section's modes.
    pub last: Vec<c64>,
}

impl Eme {
    /// `modes` modes of `polarization` in each section at `wavelength`, between electric walls
    /// and without a PML.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no modes.
    pub fn new(polarization: Polarization, wavelength: Wavelength, modes: usize) -> Result<Eme> {
        if modes == 0 {
            return Err(Error::invalid("eme", "needs at least one mode"));
        }
        Ok(Eme {
            polarization,
            wavelength,
            modes,
            walls: [Wall::Electric; 2],
            pml: None,
        })
    }

    /// The same with the lower and upper walls given.
    pub fn with_walls(self, lower: Wall, upper: Wall) -> Eme {
        Eme {
            walls: [lower, upper],
            ..self
        }
    }

    /// The same with a PML `thickness` thick and of `strength` σ inside each wall.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a thickness or strength that isn't positive and finite.
    pub fn with_pml(self, thickness: Length, strength: f64) -> Result<Eme> {
        let t = thickness.to_um();
        if !(t.is_finite() && t > 0.0 && strength.is_finite() && strength > 0.0) {
            return Err(Error::invalid(
                "eme pml",
                format!("needs a positive thickness and strength, got {t} um and {strength}"),
            ));
        }
        Ok(Eme {
            pml: Some(Pml {
                thickness: t,
                strength,
                lower: true,
            }),
            ..self
        })
    }

    /// The same with a PML inside the upper wall only: for half of a symmetric structure, its
    /// lower wall the plane of symmetry (magnetic for TE's even modes, electric for TM's).
    ///
    /// # Errors
    ///
    /// As [`Eme::with_pml`].
    pub fn with_upper_pml(self, thickness: Length, strength: f64) -> Result<Eme> {
        let mut eme = self.with_pml(thickness, strength)?;
        if let Some(p) = eme.pml.as_mut() {
            p.lower = false;
        }
        Ok(eme)
    }

    /// The polarization.
    pub fn polarization(&self) -> Polarization {
        self.polarization
    }

    /// The wavelength.
    pub fn wavelength(&self) -> Wavelength {
        self.wavelength
    }

    /// The number of modes each section keeps.
    pub fn mode_count(&self) -> usize {
        self.modes
    }

    /// The first [`Eme::mode_count`] modes of `slice`, highest Re n_eff² first.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] where a mode can't be followed into the PML (or the loss), or
    /// two end as one: fewer modes, or a thicker and weaker PML.
    pub fn modes(&self, slice: &Slice) -> Result<Vec<Mode>> {
        modes::solve(self, slice)
    }

    /// The S-matrix of `sections`, in order along z, between the first's modes and the last's.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no sections, sections whose windows differ in width, or a
    /// length that isn't finite and at least 0; and [`Eme::modes`]'s.
    pub fn s_matrix(&self, sections: &[Section]) -> Result<Scattering> {
        scattering::structure(self, sections)
    }
}

#[cfg(test)]
#[path = "cases/eme/tests.rs"]
mod tests;
