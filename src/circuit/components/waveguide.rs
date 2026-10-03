//! Straight and bent waveguides: a phase and a loss, from a mode's dispersion.

use std::f64::consts::{LN_10, PI, TAU};

use num_complex::Complex64 as c64;

use super::{Entries, both, check, check_wavelength, closed_form, closed_form_derivatives};
use crate::circuit::{Component, Fidelity, Parameter, Port, PortMode, Provenance, SMatrix};
use crate::mode::Polarization;
use crate::mode::bend::SlabBend;
use crate::mode::dispersion::{loss_db_per_cm, track};
use crate::mode::vector::CrossSection;
use crate::units::{SPEED_OF_LIGHT, Wavelength};
use crate::{Error, Result};

/// A guided mode's dispersion about a wavelength λ₀: its effective index n₀, group index n_g and
/// dispersion parameter D there, and its loss.
///
/// The effective index is second order in λ about λ₀ (Chrostowski & Hochberg 2015, Eqs. 3.5
/// and 3.6, doi:10.1017/CBO9781316084168, solved for the derivatives):
///
/// n(λ) = n₀ + n′ (λ − λ₀) + n″ (λ − λ₀)²/2, with n′ = (n₀ − n_g)/λ₀ and n″ = −c D/λ₀,
///
/// and a mode travelling a length L picks up e^(iγL), γ = 2π n(λ)/λ + iα/2, α the power loss
/// per unit length (loss in dB/cm times 10⁻⁴ ln 10/10 per µm), constant in λ.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dispersion {
    /// λ₀.
    pub wavelength: Wavelength,
    /// n₀, the effective index at λ₀.
    pub effective_index: f64,
    /// n_g at λ₀.
    pub group_index: f64,
    /// D at λ₀, ps/(nm·km), as [`crate::mode::dispersion::dispersion`] gives it; zero for a
    /// first-order model.
    pub dispersion: f64,
    /// The power loss, dB/cm.
    pub loss_db_per_cm: f64,
    /// The mode's polarization (TE-like or TM-like), if known; it is then taken to be the
    /// fundamental mode of that polarization, and the ports say so.
    pub polarization: Option<Polarization>,
}

impl Dispersion {
    /// A lossless mode with effective index `effective_index` and group index `group_index` at
    /// `wavelength`, first order in λ, its polarization unstated.
    pub fn new(wavelength: Wavelength, effective_index: f64, group_index: f64) -> Dispersion {
        Dispersion {
            wavelength,
            effective_index,
            group_index,
            dispersion: 0.0,
            loss_db_per_cm: 0.0,
            polarization: None,
        }
    }

    /// The same, with dispersion parameter `d`, ps/(nm·km).
    #[must_use]
    pub fn with_dispersion(mut self, d: f64) -> Dispersion {
        self.dispersion = d;
        self
    }

    /// The same, losing `db_per_cm`.
    #[must_use]
    pub fn with_loss(mut self, db_per_cm: f64) -> Dispersion {
        self.loss_db_per_cm = db_per_cm;
        self
    }

    /// The same, the fundamental mode of `polarization`.
    #[must_use]
    pub fn with_polarization(mut self, polarization: Polarization) -> Dispersion {
        self.polarization = Some(polarization);
        self
    }

    /// n′ = dn/dλ and n″ = d²n/dλ² at λ₀, per µm and per µm².
    fn derivatives(&self) -> (f64, f64) {
        let l0 = self.wavelength.to_um();
        // D (ps/(nm·km)) = −λ n″ / c × 1e12 with λ in µm and c in m/s (mode::dispersion)
        (
            (self.effective_index - self.group_index) / l0,
            -self.dispersion * SPEED_OF_LIGHT / (l0 * 1e12),
        )
    }

    /// n(λ).
    pub fn effective_index_at(&self, wavelength: Wavelength) -> f64 {
        let (d1, d2) = self.derivatives();
        let x = wavelength.to_um() - self.wavelength.to_um();
        self.effective_index + d1 * x + 0.5 * d2 * x * x
    }

    /// n_g(λ) = n(λ) − λ dn/dλ.
    pub fn group_index_at(&self, wavelength: Wavelength) -> f64 {
        let (d1, d2) = self.derivatives();
        let x = wavelength.to_um() - self.wavelength.to_um();
        self.effective_index_at(wavelength) - wavelength.to_um() * (d1 + d2 * x)
    }

    /// γ = 2π n(λ)/λ + iα/2, per µm: the mode picks up e^(iγL) over a length L.
    pub fn propagation(&self, wavelength: Wavelength) -> c64 {
        let alpha = self.loss_db_per_cm * 1e-4 * LN_10 / 10.0;
        c64::new(
            TAU * self.effective_index_at(wavelength) / wavelength.to_um(),
            alpha / 2.0,
        )
    }

    /// The mode a port carries, if the polarization is known.
    fn port_mode(&self) -> Option<PortMode> {
        self.polarization.map(|polarization| PortMode {
            polarization,
            order: 0,
            effective_index: self.effective_index,
            group_index: Some(self.group_index),
            wavelength: self.wavelength,
        })
    }

    /// The dispersion of the effective indices `n` (complex) at λ₀ − 2h, λ₀ − h, λ₀, λ₀ + h,
    /// λ₀ + 2h: n₀, n_g and D from the three in the middle (central differences), the loss
    /// from Im n₀, and how far the model is from the outer two, largest |Δ Re n|.
    fn from_samples(wavelength: Wavelength, step: f64, n: [c64; 5]) -> (Dispersion, f64) {
        let l0 = wavelength.to_um();
        let d1 = (n[3].re - n[1].re) / (2.0 * step);
        let d2 = (n[3].re - 2.0 * n[2].re + n[1].re) / (step * step);
        let model = Dispersion {
            wavelength,
            effective_index: n[2].re,
            group_index: n[2].re - l0 * d1,
            dispersion: -l0 * d2 / SPEED_OF_LIGHT * 1e12,
            loss_db_per_cm: loss_db_per_cm(n[2], wavelength),
            polarization: None,
        };
        let residual = [(0, -2.0), (4, 2.0)]
            .iter()
            .map(|&(k, m)| {
                // (the step is checked positive, so these wavelengths are valid)
                let w = Wavelength::um(l0 + m * step).expect("a positive wavelength");
                (model.effective_index_at(w) - n[k].re).abs()
            })
            .fold(0.0, f64::max);
        (model, residual)
    }

    /// The five wavelengths λ₀ − 2h … λ₀ + 2h.
    fn five(wavelength: Wavelength, step: f64) -> Result<[Wavelength; 5]> {
        let l0 = wavelength.to_um();
        if !(step.is_finite() && step > 0.0 && 2.0 * step < l0) {
            return Err(Error::invalid(
                "dispersion",
                format!("the step must be positive and below λ₀/2, got {step} um"),
            ));
        }
        let w = |m: f64| Wavelength::um(l0 + m * step);
        Ok([w(-2.0)?, w(-1.0)?, wavelength, w(1.0)?, w(2.0)?])
    }

    /// The dispersion of a mode of the full-vector solver ([`crate::mode::vector`]): the mode
    /// nearest `near` (the fundamental when `None`) of `cross_section` at λ₀ − 2h, tracked
    /// ([`crate::mode::dispersion::track`]) to λ₀ − h, λ₀, λ₀ + h and λ₀ + 2h, `step` h µm
    /// apart. Returns the model, its largest |Δn_eff| against the solver at λ₀ ± 2h, and the
    /// cross-section's grid at λ₀, `"nx × ny cells, h nm"` (the smallest spacing).
    ///
    /// Its polarization is TE when the mode's TE fraction is above one half, TM otherwise.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a step that isn't positive and below λ₀/2, and those of
    /// `cross_section` and of the solver.
    pub fn from_modes(
        mut cross_section: impl FnMut(Wavelength) -> Result<CrossSection>,
        wavelength: Wavelength,
        step: f64,
        near: Option<f64>,
    ) -> Result<(Dispersion, f64, String)> {
        let wavelengths = Dispersion::five(wavelength, step)?;
        let grid = {
            let cs = cross_section(wavelength)?;
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
        };
        let modes = track(&mut cross_section, &wavelengths, near, 3)?;
        let n: Vec<c64> = modes.iter().map(|m| m.effective_index()).collect();
        let (mut model, residual) =
            Dispersion::from_samples(wavelength, step, [n[0], n[1], n[2], n[3], n[4]]);
        model.polarization = Some(if modes[2].te_fraction() > 0.5 {
            Polarization::Te
        } else {
            Polarization::Tm
        });
        Ok((model, residual, grid))
    }

    /// The dispersion of an exact bent slab's fundamental mode of `polarization`
    /// ([`crate::mode::bend::SlabBend`], its effective index along the arc at the reference
    /// radius): `bend` gives the slab at each of λ₀ − 2h … λ₀ + 2h, `step` h µm apart. Returns
    /// the model and its largest |Δn_eff| against the exact modes at λ₀ ± 2h. The loss is the
    /// radiation loss along the arc.
    ///
    /// The slab is the guide seen from above, bent in the chip's plane, so the model's
    /// polarization, which its ports state, is the device's and not the slab's: a TM slab mode
    /// (E in the plane) is the TE-like guide's, a TE one (E normal to it) the TM-like guide's.
    ///
    /// # Errors
    ///
    /// As [`Dispersion::from_modes`], with those of `bend` and of the bent slab.
    pub fn from_bent_slab(
        bend: impl Fn(Wavelength) -> Result<SlabBend>,
        polarization: Polarization,
        wavelength: Wavelength,
        step: f64,
    ) -> Result<(Dispersion, f64)> {
        let wavelengths = Dispersion::five(wavelength, step)?;
        let mut n = [c64::new(0.0, 0.0); 5];
        for (k, &w) in wavelengths.iter().enumerate() {
            n[k] = bend(w)?.fundamental(polarization, w)?;
        }
        let (mut model, residual) = Dispersion::from_samples(wavelength, step, n);
        model.polarization = Some(super::seen_from_above(polarization));
        Ok((model, residual))
    }
}

/// The ports of a two-port guide carrying `dispersion`'s mode.
fn guide_ports(dispersion: &Dispersion) -> Vec<Port> {
    ["o1", "o2"]
        .iter()
        .map(|&name| match dispersion.port_mode() {
            Some(mode) => Port::new(name).with_mode(mode),
            None => Port::new(name),
        })
        .collect()
}

/// A straight waveguide: ports `o1` and `o2`, S₂₁ = S₁₂ = e^(iγL) with γ from its
/// [`Dispersion`], no reflection. Parameters `length`, µm, and `loss`, dB/cm (starting at the
/// dispersion's), so that a loss can be fitted or swept like the length.
///
/// Lossless it is unitary; it is reciprocal.
#[derive(Clone, Debug, PartialEq)]
pub struct Waveguide {
    dispersion: Dispersion,
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
    provenance: Provenance,
}

impl Waveguide {
    /// A waveguide whose mode has `dispersion`, a closed form.
    pub fn new(dispersion: Dispersion) -> Waveguide {
        Waveguide {
            ports: guide_ports(&dispersion),
            parameters: vec![
                Parameter::new("length", "µm", 10.0, 0.0, 1e5),
                Parameter::new(
                    "loss",
                    "dB/cm",
                    dispersion.loss_db_per_cm.max(0.0),
                    0.0,
                    1e3,
                ),
            ],
            dispersion,
            provenance: Provenance {
                fidelity: Fidelity::Analytic,
                source: format!(
                    "a uniform waveguide, e^(iγL): n_eff {}, n_g {}, D {} ps/(nm km) at {} um, {} dB/cm \
                     (n_eff second order in λ: Chrostowski & Hochberg 2015, Eqs. 3.5-3.6, \
                     doi:10.1017/CBO9781316084168)",
                    dispersion.effective_index,
                    dispersion.group_index,
                    dispersion.dispersion,
                    dispersion.wavelength.to_um(),
                    dispersion.loss_db_per_cm
                ),
                error: Some(0.0),
                validity: None,
            },
        }
    }

    /// The same model, with `provenance` saying where its dispersion came from.
    #[must_use]
    pub fn with_provenance(mut self, provenance: Provenance) -> Waveguide {
        self.provenance = provenance;
        self
    }

    /// A waveguide whose mode is the full-vector solver's, as [`Dispersion::from_modes`] finds
    /// it: exact for a waveguide uniform along its length, to the grid's error, so of
    /// [`Fidelity::ThreeD`]. Valid over λ₀ ± 2h, where the model is within its stated
    /// residual of the solver.
    ///
    /// # Errors
    ///
    /// As [`Dispersion::from_modes`].
    pub fn from_modes(
        cross_section: impl FnMut(Wavelength) -> Result<CrossSection>,
        wavelength: Wavelength,
        step: f64,
        near: Option<f64>,
    ) -> Result<Waveguide> {
        let (dispersion, residual, grid) =
            Dispersion::from_modes(cross_section, wavelength, step, near)?;
        let l0 = wavelength.to_um();
        let provenance = Provenance {
            fidelity: Fidelity::ThreeD,
            source: format!(
                "the full-vector mode solver (Fallahkhair et al. 2008, doi:10.1109/JLT.2008.923643) \
                 on {grid}: n_eff {:.6}, n_g {:.4}, D {} ps/(nm km), {} dB/cm at {l0} um; \
                 n_eff second order in λ, within {residual:.1e} of the solver at ±{} um",
                dispersion.effective_index,
                dispersion.group_index,
                super::fixed(dispersion.dispersion, 0),
                super::fixed(dispersion.loss_db_per_cm, 3),
                2.0 * step
            ),
            error: None,
            validity: Some((l0 - 2.0 * step, l0 + 2.0 * step)),
        };
        Ok(Waveguide::new(dispersion).with_provenance(provenance))
    }

    /// Its mode's dispersion.
    pub fn dispersion(&self) -> &Dispersion {
        &self.dispersion
    }

    fn entries(&self, wavelength: Wavelength, values: &[super::Dual]) -> Result<Entries> {
        let (length, loss) = (values[0], values[1]);
        let beta = TAU * self.dispersion.effective_index_at(wavelength) / wavelength.to_um();
        // e^(iβL) 10^(−loss L/20), the loss in dB/cm and L in µm
        let t = (length.scale(c64::new(0.0, beta)) - loss * length * (1e-4 * LN_10 / 20.0)).exp();
        Ok(both(1, 0, t).to_vec())
    }
}

impl Component for Waveguide {
    fn kind(&self) -> &str {
        "waveguide"
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        check(self.kind(), &self.parameters, values)?;
        check_wavelength(self.kind(), self.provenance.validity, wavelength)?;
        closed_form(2, values, |v| self.entries(wavelength, v))
    }

    fn provenance(&self) -> Provenance {
        self.provenance.clone()
    }

    fn derivatives(&self, wavelength: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        check(self.kind(), &self.parameters, values)?;
        check_wavelength(self.kind(), self.provenance.validity, wavelength)?;
        closed_form_derivatives(2, values, |v| self.entries(wavelength, v))
    }
}

/// A circular bend of radius R: ports `o1` and `o2`, S₂₁ = S₁₂ = e^(iγRθ), γ from the bent
/// mode's [`Dispersion`], its effective index along the arc at R and its loss the radiation
/// loss (and any other). Parameter `angle`, degrees.
///
/// The transitions to straight guides (their mode mismatch) are not included.
#[derive(Clone, Debug, PartialEq)]
pub struct Bend {
    radius: f64,
    dispersion: Dispersion,
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
    provenance: Provenance,
}

impl Bend {
    /// A bend of `radius` µm whose mode has `dispersion` along the arc at that radius, a closed
    /// form.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a radius that isn't positive and finite.
    pub fn new(radius: f64, dispersion: Dispersion) -> Result<Bend> {
        if !(radius.is_finite() && radius > 0.0) {
            return Err(Error::invalid(
                "bend",
                format!("the radius must be positive, got {radius} um"),
            ));
        }
        Ok(Bend {
            radius,
            ports: guide_ports(&dispersion),
            parameters: vec![Parameter::new("angle", "°", 90.0, 0.0, 360.0)],
            dispersion,
            provenance: Provenance {
                fidelity: Fidelity::Analytic,
                source: format!(
                    "a circular bend of radius {radius} um, e^(iγRθ): n_eff {}, n_g {} along the arc \
                     at {} um, {} dB/cm",
                    dispersion.effective_index,
                    dispersion.group_index,
                    dispersion.wavelength.to_um(),
                    dispersion.loss_db_per_cm
                ),
                error: Some(0.0),
                validity: None,
            },
        })
    }

    /// The same model, with `provenance` saying where its dispersion came from.
    #[must_use]
    pub fn with_provenance(mut self, provenance: Provenance) -> Bend {
        self.provenance = provenance;
        self
    }

    /// A bend whose mode is the full-vector solver's on a bent cross-section
    /// ([`CrossSection::bent`], Heiblum & Harris's conformal map, with a PML outside for the
    /// radiation): `cross_section` must bend it at `radius`. [`Fidelity::ThreeD`]: exact for E
    /// normal to the bend plane, with an error of order 1/R² for E in it (see
    /// [`CrossSection::bent`]). Valid over λ₀ ± 2h.
    ///
    /// # Errors
    ///
    /// As [`Bend::new`] and [`Dispersion::from_modes`].
    pub fn from_modes(
        radius: f64,
        cross_section: impl FnMut(Wavelength) -> Result<CrossSection>,
        wavelength: Wavelength,
        step: f64,
        near: Option<f64>,
    ) -> Result<Bend> {
        let (dispersion, residual, grid) =
            Dispersion::from_modes(cross_section, wavelength, step, near)?;
        let l0 = wavelength.to_um();
        let provenance = Provenance {
            fidelity: Fidelity::ThreeD,
            source: format!(
                "the full-vector mode solver on a bend of radius {radius} um by Heiblum & Harris's \
                 conformal map (doi:10.1109/JQE.1975.1068563), on {grid}: n_eff {:.6}, n_g {:.4}, \
                 {} dB/cm at {l0} um; within {residual:.1e} of the solver at ±{} um",
                dispersion.effective_index,
                dispersion.group_index,
                super::fixed(dispersion.loss_db_per_cm, 3),
                2.0 * step
            ),
            error: None,
            validity: Some((l0 - 2.0 * step, l0 + 2.0 * step)),
        };
        Ok(Bend::new(radius, dispersion)?.with_provenance(provenance))
    }

    /// A bend whose mode is an exact bent slab's ([`Dispersion::from_bent_slab`]): a 2D model,
    /// the slab being the guide seen from above by the effective index method, so of
    /// [`Fidelity::TwoD`]. `bend` must give the slab bent at `radius`.
    ///
    /// # Errors
    ///
    /// As [`Bend::new`] and [`Dispersion::from_bent_slab`].
    pub fn from_bent_slab(
        radius: f64,
        bend: impl Fn(Wavelength) -> Result<SlabBend>,
        polarization: Polarization,
        wavelength: Wavelength,
        step: f64,
    ) -> Result<Bend> {
        let (dispersion, residual) =
            Dispersion::from_bent_slab(bend, polarization, wavelength, step)?;
        let l0 = wavelength.to_um();
        let provenance = Provenance {
            fidelity: Fidelity::TwoD,
            source: format!(
                "an exact bent slab of radius {radius} um (radial shooting to the outgoing Hankel \
                 function, Marcuse 1971, doi:10.1002/j.1538-7305.1971.tb02620.x, Eq. 10): n_eff \
                 {:.6}, n_g {:.4}, {} dB/cm at {l0} um; within {residual:.1e} of the exact modes \
                 at ±{} um",
                dispersion.effective_index,
                dispersion.group_index,
                super::fixed(dispersion.loss_db_per_cm, 3),
                2.0 * step
            ),
            error: None,
            validity: Some((l0 - 2.0 * step, l0 + 2.0 * step)),
        };
        Ok(Bend::new(radius, dispersion)?.with_provenance(provenance))
    }

    /// Its radius, µm.
    pub fn radius(&self) -> f64 {
        self.radius
    }

    /// Its mode's dispersion, along the arc at its radius.
    pub fn dispersion(&self) -> &Dispersion {
        &self.dispersion
    }

    fn entries(&self, wavelength: Wavelength, values: &[super::Dual]) -> Result<Entries> {
        let gamma = self.dispersion.propagation(wavelength);
        let length = values[0] * (self.radius * PI / 180.0);
        let t = length.scale(c64::new(0.0, 1.0) * gamma).exp();
        Ok(both(1, 0, t).to_vec())
    }
}

impl Component for Bend {
    fn kind(&self) -> &str {
        "bend"
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        check(self.kind(), &self.parameters, values)?;
        check_wavelength(self.kind(), self.provenance.validity, wavelength)?;
        closed_form(2, values, |v| self.entries(wavelength, v))
    }

    fn provenance(&self) -> Provenance {
        self.provenance.clone()
    }

    fn derivatives(&self, wavelength: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        check(self.kind(), &self.parameters, values)?;
        check_wavelength(self.kind(), self.provenance.validity, wavelength)?;
        closed_form_derivatives(2, values, |v| self.entries(wavelength, v))
    }
}

/// An ideal phase shifter: ports `o1` and `o2`, S₂₁ = S₁₂ = e^(iφ), lossless and the same at
/// every wavelength. Parameter `phase`, rad, from −2π to 2π: a heater's or a modulator's
/// effect before 0.6 models them.
#[derive(Clone, Debug, PartialEq)]
pub struct PhaseShifter {
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
}

impl PhaseShifter {
    /// A phase shifter, starting at no phase.
    pub fn new() -> PhaseShifter {
        PhaseShifter {
            ports: crate::circuit::ports(&["o1", "o2"]),
            parameters: vec![Parameter::new("phase", "rad", 0.0, -TAU, TAU)],
        }
    }

    fn entries(values: &[super::Dual]) -> Entries {
        both(1, 0, values[0].times_i().exp()).to_vec()
    }
}

impl Default for PhaseShifter {
    fn default() -> PhaseShifter {
        PhaseShifter::new()
    }
}

impl Component for PhaseShifter {
    fn kind(&self) -> &str {
        "phase shifter"
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, _: Wavelength, values: &[f64]) -> Result<SMatrix> {
        check(self.kind(), &self.parameters, values)?;
        closed_form(2, values, |v| Ok(PhaseShifter::entries(v)))
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Analytic,
            source: "an ideal phase shifter, e^(iφ)".into(),
            error: Some(0.0),
            validity: None,
        }
    }

    fn derivatives(&self, _: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        check(self.kind(), &self.parameters, values)?;
        closed_form_derivatives(2, values, |v| Ok(PhaseShifter::entries(v)))
    }
}
