//! The strip every first device is made of: its stack and core layer, the mesh its solves use,
//! and its mode's dispersion at the analytic fidelity (the fit, or a stated one) and from the
//! full-vector solver.

use crate::circuit::components::Dispersion;
use crate::material;
use crate::mode::vector::{Boundary, CrossSection};
use crate::stack::{LayerStack, Structure};
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

use super::fit;
use super::section::{self, Cut, Mesh, Scheme};

/// A strip guide's platform: the stack, its core layer, and the mesh solves use; and, if
/// given, the dispersion its analytic fidelity states instead of the fit's.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Strip {
    pub(super) stack: LayerStack,
    pub(super) core: String,
    pub(super) mesh: Mesh,
    pub(super) stated: Option<Dispersion>,
}

/// The default mesh: cells fitted to the core about 10 nm apart, 1 µm of cladding, on the
/// mirrored domain, by Hadley's equations.
pub(super) fn default_mesh() -> Mesh {
    Mesh::fitted(0.01, 1.0, Scheme::Hadley)
}

impl Strip {
    /// 220 nm SOI ([`LayerStack::soi_220`]), its silicon layer the core, on the default mesh.
    pub(super) fn soi() -> Strip {
        Strip {
            stack: LayerStack::soi_220(),
            core: "Si".into(),
            mesh: default_mesh(),
            stated: None,
        }
    }

    /// `stack`, its layer `core` the strip's.
    pub(super) fn on(stack: LayerStack, core: &str) -> Result<Strip> {
        section::core_span(&stack, core)?;
        Ok(Strip {
            stack,
            core: core.into(),
            mesh: default_mesh(),
            stated: None,
        })
    }

    /// The stack with the core `thickness` µm thick.
    pub(super) fn stack(&self, thickness: f64) -> Result<LayerStack> {
        section::with_thickness(&self.stack, &self.core, thickness)
    }

    /// The guides seen in a cross-section: straight along x, `width` wide, at y = ±`centres`
    /// (or one at 0), on the stack with the core `thickness` thick; and the core's mid-plane.
    fn section_structure(
        &self,
        width: f64,
        thickness: f64,
        centres: &[f64],
    ) -> Result<(Structure, f64)> {
        let stack = self.stack(thickness)?;
        let (bottom, top) = section::core_span(&stack, &self.core)?;
        let mut s = Structure::new(stack);
        for &y in centres {
            s.draw(&self.core, section::straight(width, y, -0.5, 0.5)?)?;
        }
        Ok((s, 0.5 * (bottom + top)))
    }

    /// The strip's cross-section at `wavelength`, `width` × `thickness` µm, on the mesh.
    pub(super) fn cross_section(
        &self,
        width: f64,
        thickness: f64,
        wavelength: Wavelength,
    ) -> Result<CrossSection> {
        let (structure, mid) = self.section_structure(width, thickness, &[0.0])?;
        Cut {
            structure: &structure,
            at: 0.0,
            mid,
        }
        .strip(width, thickness, &self.mesh, wavelength)
    }

    /// Two strips `gap` µm apart: their cross-section at `wavelength`, `between` on the plane
    /// between them on the mirrored domain.
    pub(super) fn pair_section(
        &self,
        width: f64,
        thickness: f64,
        gap: f64,
        between: Boundary,
        wavelength: Wavelength,
    ) -> Result<CrossSection> {
        let c = (gap + width) / 2.0;
        let (structure, mid) = self.section_structure(width, thickness, &[-c, c])?;
        Cut {
            structure: &structure,
            at: 0.0,
            mid,
        }
        .pair(width, thickness, gap, between, &self.mesh, wavelength)
    }

    /// The strip bent at `radius` µm: its cross-section at `wavelength`, by the conformal map,
    /// its window reaching past `caustic` µm; unless `mapped`, the same window straight.
    pub(super) fn bent_section(
        &self,
        radius: f64,
        width: f64,
        thickness: f64,
        caustic: f64,
        wavelength: Wavelength,
        mapped: bool,
    ) -> Result<CrossSection> {
        let (structure, mid) = self.section_structure(width, thickness, &[0.0])?;
        Cut {
            structure: &structure,
            at: 0.0,
            mid,
        }
        .bent(
            radius, width, thickness, caustic, &self.mesh, wavelength, mapped,
        )
    }

    /// The indices of the effective index method at `wavelength`: below, core, above and
    /// beside the strip.
    pub(super) fn indices(
        &self,
        thickness: f64,
        wavelength: Wavelength,
    ) -> Result<(f64, f64, f64, f64)> {
        section::indices(&self.stack(thickness)?, &self.core, wavelength)
    }

    /// Refuses a stack the fits weren't made on: silicon in silica, each index within 0.01
    /// of Li's and Malitson's at 1.55 µm.
    fn check_platform(&self, thickness: f64) -> Result<()> {
        let l0 = Wavelength::um(fit::WAVELENGTH)?;
        let (below, core, above, beside) = self.indices(thickness, l0)?;
        let si = material::silicon().refractive_index(l0)?.re;
        let ox = material::silica().refractive_index(l0)?.re;
        let near = |n: f64, of: f64| (n - of).abs() <= 0.01;
        if near(core, si) && near(below, ox) && near(above, ox) && near(beside, ox) {
            Ok(())
        } else {
            Err(Error::invalid(
                "device",
                format!(
                    "the analytic fit is for silicon ({si:.3}) in silica ({ox:.3}) at 1.55 um; \
                     this stack's core is {core:.3} with {below:.3} below, {above:.3} above and \
                     {beside:.3} beside: solve the modes (3D)"
                ),
            ))
        }
    }

    /// The analytic fidelity's dispersion: the stated one, or the fit's at `width` ×
    /// `thickness` µm, losing `loss` dB/cm; with the grid its figures state. `wavelength` is
    /// the band's centre, which the fit's first-order model must hold at.
    pub(super) fn analytic(
        &self,
        width: f64,
        thickness: f64,
        wavelength: f64,
        loss: f64,
    ) -> Result<(Dispersion, String)> {
        if let Some(d) = self.stated {
            return Ok((d.with_loss(loss), "stated: no grid".into()));
        }
        self.check_platform(thickness)?;
        let (lo, hi) = fit::BAND;
        if !(lo..=hi).contains(&wavelength) {
            return Err(Error::invalid(
                "device",
                format!(
                    "the analytic fit holds from {lo} to {hi} um, not about {wavelength} um: \
                     solve the modes (3D)"
                ),
            ));
        }
        let (n, ng) = fit::guide(width, thickness)?;
        Ok((
            Dispersion::new(Wavelength::um(fit::WAVELENGTH)?, n, ng).with_loss(loss),
            fit::GUIDE_GRID.into(),
        ))
    }

    /// The full-vector solver's dispersion about `wavelength` (µm), sampled over `span` µm
    /// (five wavelengths span/4 apart): the model, its largest |Δn_eff| against the solver at
    /// the band's edges, and the grid. By Hadley's equations when the mesh says so.
    pub(super) fn solved(
        &self,
        width: f64,
        thickness: f64,
        wavelength: f64,
        span: f64,
    ) -> Result<(Dispersion, f64, String)> {
        let l0 = Wavelength::um(wavelength)?;
        let section = |w: Wavelength| self.cross_section(width, thickness, w);
        match self.mesh.scheme {
            Scheme::Hadley => Dispersion::from_hadley(section, l0, span / 4.0, None),
            Scheme::Standard => Dispersion::from_modes(section, l0, span / 4.0, None),
        }
    }

    /// The grid of a cross-section, `"nx × ny cells, h nm"` with h the smallest spacing, as the
    /// circuit's builders state it.
    pub(super) fn grid(cs: &CrossSection) -> String {
        let smallest = |v: &[f64]| {
            v.windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .fold(f64::INFINITY, f64::min)
        };
        format!(
            "{} × {} cells, {:.3} nm",
            cs.x().len() - 1,
            cs.y().len() - 1,
            1e3 * smallest(cs.x()).min(smallest(cs.y()))
        )
    }
}

/// The length `um` µm.
pub(super) fn um(um: f64) -> Length {
    Length::um(um)
}
