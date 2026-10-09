//! 2 × 2 couplers: an ideal one with its coupling as a parameter, and a directional coupler by
//! coupled-mode theory from its two supermodes.
//!
//! Both have ports `o1` (lower left), `o2` (upper left), `o3` (upper right) and `o4` (lower
//! right): the lower guide runs from `o1` to `o4`, the upper from `o2` to `o3`.

use std::f64::consts::PI;

use num_complex::Complex64 as c64;

use super::{
    Dual, Entries, amplitude, both, check, check_wavelength, closed_form, closed_form_derivatives,
};
use crate::circuit::{Component, Fidelity, Parameter, Port, Provenance, SMatrix, ports};
use crate::mode::vector::CrossSection;
use crate::units::Wavelength;
use crate::{Error, Result};

use super::waveguide::Dispersion;

/// The four entries of a symmetric 2 × 2 coupler: `through` along each guide, `across` from
/// one to the other, the same both ways.
fn coupler_entries(through: Dual, across: Dual) -> Entries {
    let mut e = Vec::with_capacity(8);
    e.extend(both(3, 0, through));
    e.extend(both(2, 1, through));
    e.extend(both(2, 0, across));
    e.extend(both(3, 1, across));
    e
}

/// An ideal 2 × 2 coupler: a share κ² of the power crosses to the other guide, the rest goes
/// through, with no phase along the guides: through √(1 − κ²), across iκ, times 10^(−loss/20)
/// for an excess loss in dB. Parameter `coupling`, the power cross-coupling κ².
///
/// The iκ makes it lossless (unitary, without excess loss) and reciprocal: the convention of
/// Bogaerts et al. 2011 (doi:10.1002/lpor.201100017), whose ring responses (their Eq. 1) it
/// reproduces, and of a directional coupler's supermodes ([`DirectionalCoupler`]) up to a
/// common phase.
#[derive(Clone, Debug, PartialEq)]
pub struct Coupler {
    excess_loss: f64,
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
}

impl Coupler {
    /// A lossless coupler.
    pub fn new() -> Coupler {
        Coupler {
            excess_loss: 0.0,
            ports: ports(&["o1", "o2", "o3", "o4"]),
            parameters: vec![Parameter::new("coupling", "", 0.5, 0.0, 1.0)],
        }
    }

    /// The same coupler, losing `db` decibels of power on every path.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a loss that isn't finite and at least zero.
    pub fn with_excess_loss(mut self, db: f64) -> Result<Coupler> {
        if !(db.is_finite() && db >= 0.0) {
            return Err(Error::invalid(
                "coupler",
                format!("the excess loss must be at least 0 dB, got {db}"),
            ));
        }
        self.excess_loss = db;
        Ok(self)
    }

    fn entries(&self, values: &[Dual]) -> Entries {
        let kappa2 = values[0];
        let a = amplitude(Dual::real(self.excess_loss));
        let through = (Dual::real(1.0) - kappa2).sqrt() * a;
        let across = kappa2.sqrt().times_i() * a;
        coupler_entries(through, across)
    }
}

impl Default for Coupler {
    fn default() -> Coupler {
        Coupler::new()
    }
}

impl Component for Coupler {
    fn kind(&self) -> &str {
        "coupler"
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, _: Wavelength, values: &[f64]) -> Result<SMatrix> {
        check(self.kind(), &self.parameters, values)?;
        closed_form(4, values, |v| Ok(self.entries(v)))
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Analytic,
            source: format!(
                "an ideal 2 × 2 coupler, through √(1 − κ²), across iκ (Bogaerts et al. 2011, \
                 doi:10.1002/lpor.201100017, Section 2.1), {} dB excess loss",
                self.excess_loss
            ),
            error: Some(0.0),
            validity: None,
        }
    }

    /// Exact, except at κ² = 0 or 1, where √ has no derivative: `None` there.
    fn derivatives(&self, _: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        check(self.kind(), &self.parameters, values)?;
        if values[0] <= 0.0 || values[0] >= 1.0 {
            return Ok(None);
        }
        closed_form_derivatives(4, values, |v| Ok(self.entries(v)))
    }
}

/// A coupled pair of waveguides' two supermodes, the even (symmetric) and the odd
/// (antisymmetric) one: their dispersions, each n_eff, n_g, D and loss at λ₀.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Supermodes {
    /// The even supermode, the higher effective index for TE-like modes.
    pub even: Dispersion,
    /// The odd supermode.
    pub odd: Dispersion,
}

impl Supermodes {
    /// Two supermodes from their dispersions.
    pub fn new(even: Dispersion, odd: Dispersion) -> Supermodes {
        Supermodes { even, odd }
    }

    /// The supermodes of two identical guides of mode `guide` with coupling coefficient C (1/µm)
    /// at λ₀: effective indices n ± Δn/2, Δn = Cλ₀/π (Chrostowski & Hochberg 2015, Eq. 4.3),
    /// the guide's group index, dispersion and loss each. Δn is then constant in λ, and
    /// C = πΔn/λ.
    pub fn from_coupling(guide: Dispersion, coupling: f64) -> Supermodes {
        let dn = coupling * guide.wavelength.to_um() / PI;
        let shifted = |sign: f64| Dispersion {
            effective_index: guide.effective_index + sign * dn / 2.0,
            group_index: guide.group_index + sign * dn / 2.0,
            ..guide
        };
        Supermodes {
            even: shifted(1.0),
            odd: shifted(-1.0),
        }
    }

    /// The supermodes of the full-vector solver ([`Dispersion::from_modes`]): `even` and `odd`
    /// give the coupled guides' cross-sections with a wall on their symmetry plane (an electric
    /// wall there keeps the even TE-like supermode, a magnetic wall the odd one), each's
    /// fundamental mode tracked over λ₀ ± 2h. Returns them, their largest |Δn_eff| against the
    /// solver at λ₀ ± 2h, and the grid.
    ///
    /// # Errors
    ///
    /// As [`Dispersion::from_modes`].
    pub fn from_modes(
        even: impl FnMut(Wavelength) -> Result<CrossSection>,
        odd: impl FnMut(Wavelength) -> Result<CrossSection>,
        wavelength: Wavelength,
        step: f64,
    ) -> Result<(Supermodes, f64, String)> {
        let (e, re, grid) = Dispersion::from_modes(even, wavelength, step, None)?;
        let (o, ro, _) = Dispersion::from_modes(odd, wavelength, step, None)?;
        Ok((Supermodes::new(e, o), re.max(ro), grid))
    }

    /// Δn = n_even − n_odd at λ.
    pub fn splitting(&self, wavelength: Wavelength) -> f64 {
        self.even.effective_index_at(wavelength) - self.odd.effective_index_at(wavelength)
    }

    /// The coupling coefficient C = πΔn/λ, 1/µm (Chrostowski & Hochberg 2015, Eq. 4.3).
    pub fn coupling(&self, wavelength: Wavelength) -> f64 {
        PI * self.splitting(wavelength) / wavelength.to_um()
    }

    /// The cross-over length L_x = λ/(2Δn), µm, over which the power crosses over completely
    /// (Chrostowski & Hochberg 2015, Eq. 4.5).
    pub fn cross_over_length(&self, wavelength: Wavelength) -> f64 {
        wavelength.to_um() / (2.0 * self.splitting(wavelength))
    }
}

/// A directional coupler by coupled-mode theory from its supermodes (Chrostowski & Hochberg
/// 2015, Section 4.1, Eqs. 4.1–4.11, doi:10.1017/CBO9781316084168): the light in one guide is
/// the sum of the two supermodes, which beat along the coupled length L, so
///
/// through = e^(iγ̄L) cos(ΔγL/2), across = i e^(iγ̄L) sin(ΔγL/2),
///
/// γ̄ and Δγ the mean and the difference of the supermodes' propagation constants (complex with
/// their losses), and the power crossing over is sin²(CL), C = πΔn/λ. Parameter `length`, µm,
/// the coupled section's.
///
/// The coupling in the bends that bring the guides together and apart isn't included (it adds
/// a length of order of a micrometre: Chrostowski & Hochberg, Section 4.1.4), nor any
/// reflection. Lossless supermodes make it unitary; it is reciprocal.
#[derive(Clone, Debug, PartialEq)]
pub struct DirectionalCoupler {
    modes: Supermodes,
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
    provenance: Provenance,
}

impl DirectionalCoupler {
    /// A coupler of supermodes `modes`, a closed form; its length starts at the 3 dB length at
    /// λ₀, L_x/2.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the even supermode's index is above the odd one's at
    /// λ₀.
    pub fn new(modes: Supermodes) -> Result<DirectionalCoupler> {
        let l0 = modes.even.wavelength;
        if modes.splitting(l0).partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err(Error::invalid(
                "directional coupler",
                format!(
                    "the even supermode's index must be above the odd one's, got {} and {}",
                    modes.even.effective_index, modes.odd.effective_index
                ),
            ));
        }
        let cross_over = modes.cross_over_length(l0);
        Ok(DirectionalCoupler {
            ports: ports(&["o1", "o2", "o3", "o4"]),
            parameters: vec![Parameter::new(
                "length",
                "µm",
                cross_over / 2.0,
                0.0,
                (20.0 * cross_over).max(1e4),
            )],
            provenance: Provenance {
                fidelity: Fidelity::Analytic,
                source: format!(
                    "coupled-mode theory from two supermodes (Chrostowski & Hochberg 2015, Eqs. \
                     4.1-4.11, doi:10.1017/CBO9781316084168): n_eff {:.6} and {:.6} at {} um, \
                     cross-over length {:.3} um",
                    modes.even.effective_index,
                    modes.odd.effective_index,
                    l0.to_um(),
                    cross_over
                ),
                error: Some(0.0),
                validity: None,
            },
            modes,
        })
    }

    /// The same model, with `provenance` saying where its supermodes came from.
    #[must_use]
    pub fn with_provenance(mut self, provenance: Provenance) -> DirectionalCoupler {
        self.provenance = provenance;
        self
    }

    /// A coupler whose supermodes are the full-vector solver's ([`Supermodes::from_modes`]):
    /// [`Fidelity::ThreeD`] for the coupled section, exact to the grid's error, the bends
    /// excluded. Valid over λ₀ ± 2h.
    ///
    /// # Errors
    ///
    /// As [`Supermodes::from_modes`] and [`DirectionalCoupler::new`].
    pub fn from_modes(
        even: impl FnMut(Wavelength) -> Result<CrossSection>,
        odd: impl FnMut(Wavelength) -> Result<CrossSection>,
        wavelength: Wavelength,
        step: f64,
    ) -> Result<DirectionalCoupler> {
        let (modes, residual, grid) = Supermodes::from_modes(even, odd, wavelength, step)?;
        let l0 = wavelength.to_um();
        let coupler = DirectionalCoupler::new(modes)?;
        let provenance = Provenance {
            fidelity: Fidelity::ThreeD,
            source: format!(
                "coupled-mode theory (Chrostowski & Hochberg 2015, Eqs. 4.1-4.11) from the \
                 full-vector solver's supermodes on {grid}: n_eff {:.6} and {:.6}, cross-over \
                 length {:.3} um at {l0} um, the coupled section only; within {residual:.1e} \
                 of the solver at ±{} um",
                modes.even.effective_index,
                modes.odd.effective_index,
                modes.cross_over_length(wavelength),
                2.0 * step
            ),
            error: None,
            validity: Some((l0 - 2.0 * step, l0 + 2.0 * step)),
        };
        Ok(coupler.with_provenance(provenance))
    }

    /// Its supermodes.
    pub fn supermodes(&self) -> &Supermodes {
        &self.modes
    }

    fn entries(&self, wavelength: Wavelength, values: &[Dual]) -> Entries {
        let even = self.modes.even.propagation(wavelength);
        let odd = self.modes.odd.propagation(wavelength);
        let i = c64::new(0.0, 1.0);
        let length = values[0];
        let common = length.scale(i * (even + odd) / 2.0).exp();
        let half = length.scale((even - odd) / 2.0);
        coupler_entries(common * half.cos(), (common * half.sin()).times_i())
    }
}

impl Component for DirectionalCoupler {
    fn kind(&self) -> &str {
        "directional coupler"
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
        closed_form(4, values, |v| Ok(self.entries(wavelength, v)))
    }

    fn provenance(&self) -> Provenance {
        self.provenance.clone()
    }

    fn derivatives(&self, wavelength: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        check(self.kind(), &self.parameters, values)?;
        check_wavelength(self.kind(), self.provenance.validity, wavelength)?;
        closed_form_derivatives(4, values, |v| Ok(self.entries(wavelength, v)))
    }
}
