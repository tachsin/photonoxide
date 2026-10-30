//! Ring resonators, all-pass and add-drop, by the closed forms of W. Bogaerts et al., "Silicon
//! microring resonators", Laser Photonics Rev. 6, 47 (published online 13 September 2011; the
//! January 2012 issue), doi:10.1002/lpor.201100017.
//!
//! A ring of round-trip length L carries a mode of [`Dispersion`]; a round trip multiplies its
//! field by a e^(iφ) = e^(iγL), so φ = βL is the single-pass phase and a = e^(−αL/2) the
//! single-pass amplitude (Bogaerts's a² = exp(−αL)), times 10^(−A₀/20) for a fixed loss A₀ per
//! round trip, in dB (the bends' and couplers' of his Eq. 19). The bus couples to it through a point
//! coupler with self-coupling r and cross-coupling k, r² + k² = 1 (lossless, Bogaerts's
//! assumption), in the convention of [`super::Coupler`]: through r, across ik.

use std::f64::consts::{PI, TAU};

use num_complex::Complex64 as c64;

use super::waveguide::Dispersion;
use super::{Dual, Entries, both, check, closed_form, closed_form_derivatives};
use crate::circuit::{Component, Fidelity, Parameter, Port, Provenance, SMatrix};
use crate::units::Wavelength;
use crate::{Error, Result};

/// The ring's parameters, shared by both kinds: `length`, then `coupling` (κ² of the input
/// bus).
fn ring_parameters(drop: bool) -> Vec<Parameter> {
    let mut p = vec![
        Parameter::new("length", "µm", TAU * 10.0, 1.0, 1e5),
        Parameter::new("coupling", "", 0.1, 0.0, 1.0),
    ];
    if drop {
        p.push(Parameter::new("coupling_drop", "", 0.1, 0.0, 1.0));
    }
    p
}

/// The ports, their modes the guide's.
fn ring_ports(guide: &Dispersion, names: &[&str]) -> Vec<Port> {
    let mode = guide
        .polarization
        .map(|polarization| crate::circuit::PortMode {
            polarization,
            order: 0,
            effective_index: guide.effective_index,
            group_index: Some(guide.group_index),
            wavelength: guide.wavelength,
        });
    names
        .iter()
        .map(|&n| match &mode {
            Some(m) => Port::new(n).with_mode(m.clone()),
            None => Port::new(n),
        })
        .collect()
}

/// Validates a ring's model: its guide's index and group index positive.
fn check_guide(guide: &Dispersion) -> Result<()> {
    if !(guide.effective_index > 0.0 && guide.group_index > 0.0 && guide.loss_db_per_cm >= 0.0) {
        return Err(Error::invalid(
            "ring",
            format!(
                "needs a positive effective and group index and a loss of at least 0, got {}, {} \
                 and {} dB/cm",
                guide.effective_index, guide.group_index, guide.loss_db_per_cm
            ),
        ));
    }
    Ok(())
}

/// Refuses a round trip that isn't a positive, finite length.
fn check_length(length: f64) -> Result<()> {
    if !(length.is_finite() && length > 0.0) {
        return Err(Error::invalid(
            "ring",
            format!("the round trip must be a positive length, got {length} um"),
        ));
    }
    Ok(())
}

/// Refuses values that aren't a ring's for its closed forms: `count` of them, the round trip
/// a positive length and each power coupling from 0 to 1.
fn check_values(values: &[f64], count: usize) -> Result<()> {
    if values.len() != count {
        let names = if count == 2 {
            "length and coupling"
        } else {
            "length, coupling and coupling_drop"
        };
        return Err(Error::invalid(
            "ring",
            format!("it takes {count} values, {names}, got {}", values.len()),
        ));
    }
    check_length(values[0])?;
    for &kappa2 in &values[1..] {
        if !(0.0..=1.0).contains(&kappa2) {
            return Err(Error::invalid(
                "ring",
                format!("a power coupling must be from 0 to 1, got {kappa2}"),
            ));
        }
    }
    Ok(())
}

/// `value` if it is finite: a closed form that divided by zero is an error, not an infinity.
fn finite(what: &str, value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::invalid(
            "ring",
            format!("{what} isn't finite for these values ({value})"),
        ))
    }
}

/// The wavelength nearest `near` where the round trip's phase is a whole number of turns,
/// n(λ)L = mλ (Bogaerts Eq. 3), by Newton's method on n(λ)L/λ − m.
fn resonance(guide: &Dispersion, near: Wavelength, length: f64) -> Result<Wavelength> {
    check_length(length)?;
    let order = (guide.effective_index_at(near) * length / near.to_um()).round();
    let mut w = near.to_um();
    for _ in 0..50 {
        let at = Wavelength::um(w)?;
        let f = guide.effective_index_at(at) * length / w - order;
        // d(nL/λ)/dλ = (L/λ²)(λ n′ − n) = −n_g L/λ²
        let df = -guide.group_index_at(at) * length / (w * w);
        let step = f / df;
        w -= step;
        if step.abs() < 1e-15 * w {
            return Wavelength::um(w);
        }
    }
    Err(Error::invalid(
        "ring",
        format!("no resonance found near {} um", near.to_um()),
    ))
}

/// The single-pass amplitude a at λ for a round trip of `length` with a fixed loss of `extra`
/// dB.
fn single_pass(guide: &Dispersion, wavelength: Wavelength, length: f64, extra: f64) -> f64 {
    (-guide.propagation(wavelength).im * length).exp() * 10f64.powf(-extra / 20.0)
}

/// Refuses a fixed loss that isn't finite and at least zero.
fn check_loss(db: f64) -> Result<()> {
    if !(db.is_finite() && db >= 0.0) {
        return Err(Error::invalid(
            "ring",
            format!("the loss per round trip must be at least 0 dB, got {db}"),
        ));
    }
    Ok(())
}

/// An all-pass ring: a ring coupled to one bus, ports `in` and `through`, its through field
///
/// t = e^(i(π + φ)) (a − r e^(−iφ)) / (1 − r a e^(iφ)) = (r − a e^(iφ)) / (1 − r a e^(iφ))
///
/// (Bogaerts Eq. 1), the same both ways (the ring is reciprocal: light from `through` circles
/// it the other way); no reflection, the counter-directional coupling of Bogaerts's Section 2.6
/// left out. Parameters `length` (the round trip, µm) and `coupling` (k² = 1 − r²).
///
/// Lossless (a = 1), |t| = 1 at every wavelength: it is then unitary.
#[derive(Clone, Debug, PartialEq)]
pub struct AllPassRing {
    guide: Dispersion,
    round_trip_loss: f64,
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
}

impl AllPassRing {
    /// A ring whose waveguide's mode has dispersion `guide`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless the guide's indices are positive and its loss at least 0.
    pub fn new(guide: Dispersion) -> Result<AllPassRing> {
        check_guide(&guide)?;
        Ok(AllPassRing {
            ports: ring_ports(&guide, &["in", "through"]),
            parameters: ring_parameters(false),
            guide,
            round_trip_loss: 0.0,
        })
    }

    /// The same ring, losing `db` more per round trip, whatever its length: its bends' and
    /// coupler's loss (Bogaerts Eq. 19).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a loss that isn't finite and at least zero.
    pub fn with_round_trip_loss(mut self, db: f64) -> Result<AllPassRing> {
        check_loss(db)?;
        self.round_trip_loss = db;
        Ok(self)
    }

    /// Its waveguide's dispersion.
    pub fn guide(&self) -> &Dispersion {
        &self.guide
    }

    fn entries(&self, wavelength: Wavelength, values: &[Dual]) -> Entries {
        let (length, kappa2) = (values[0], values[1]);
        let i = c64::new(0.0, 1.0);
        let round = length.scale(i * self.guide.propagation(wavelength)).exp()
            * super::amplitude(Dual::real(self.round_trip_loss));
        let r = (Dual::real(1.0) - kappa2).sqrt();
        let t = (r - round) / (Dual::real(1.0) - r * round);
        both(1, 0, t).to_vec()
    }

    /// The resonance nearest `near` for a ring of `length` µm: n(λ)L = mλ (Bogaerts Eq. 3).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a length that isn't positive and finite, or if Newton's
    /// method doesn't converge.
    pub fn resonance(&self, near: Wavelength, length: f64) -> Result<Wavelength> {
        resonance(&self.guide, near, length)
    }

    /// The free spectral range at λ, µm: λ²/(n_g L) (Bogaerts Eq. 9).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a length that isn't positive and finite.
    pub fn fsr(&self, wavelength: Wavelength, length: f64) -> Result<f64> {
        check_length(length)?;
        finite(
            "the free spectral range",
            wavelength.to_um().powi(2) / (self.guide.group_index_at(wavelength) * length),
        )
    }

    /// r a: the round trip's amplitude, coupling included.
    fn ra(&self, wavelength: Wavelength, values: &[f64]) -> f64 {
        (1.0 - values[1]).sqrt()
            * single_pass(&self.guide, wavelength, values[0], self.round_trip_loss)
    }

    /// The full width at half maximum of the resonance at λ, µm: (1 − ra)λ²/(π n_g L √(ra))
    /// (Bogaerts Eq. 7). `values` are the parameters', `[length, coupling]`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `values` are a positive, finite length and a coupling
    /// from 0 to 1, or if the width isn't finite: a ring that couples all its light out
    /// (ra = 0) has no resonance.
    pub fn fwhm(&self, wavelength: Wavelength, values: &[f64]) -> Result<f64> {
        check_values(values, 2)?;
        let ra = self.ra(wavelength, values);
        finite(
            "the resonance's width",
            (1.0 - ra) * wavelength.to_um().powi(2)
                / (PI * self.guide.group_index_at(wavelength) * values[0] * ra.sqrt()),
        )
    }

    /// The finesse, FSR/FWHM = π√(ra)/(1 − ra) (Bogaerts Eqs. 17 and 21).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for `values` as in [`AllPassRing::fwhm`], or if the finesse
    /// isn't finite: a ring without loss or coupling (ra = 1) has no linewidth.
    pub fn finesse(&self, wavelength: Wavelength, values: &[f64]) -> Result<f64> {
        check_values(values, 2)?;
        let ra = self.ra(wavelength, values);
        finite("the finesse", PI * ra.sqrt() / (1.0 - ra))
    }

    /// The loaded Q, λ/FWHM = π n_g L √(ra)/(λ(1 − ra)) (Bogaerts Eqs. 18 and 20).
    ///
    /// # Errors
    ///
    /// As [`AllPassRing::finesse`].
    pub fn q_factor(&self, wavelength: Wavelength, values: &[f64]) -> Result<f64> {
        check_values(values, 2)?;
        let ra = self.ra(wavelength, values);
        finite(
            "the Q factor",
            PI * self.guide.group_index_at(wavelength) * values[0] * ra.sqrt()
                / (wavelength.to_um() * (1.0 - ra)),
        )
    }

    /// The through power off resonance, T_t = (r + a)²/(1 + ra)², and on it, R_min =
    /// (r − a)²/(1 − ra)² (Bogaerts Eqs. 11 and 12); their ratio is the extinction.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for `values` as in [`AllPassRing::fwhm`], or if a power isn't
    /// finite (ra = 1).
    pub fn extremes(&self, wavelength: Wavelength, values: &[f64]) -> Result<(f64, f64)> {
        check_values(values, 2)?;
        let r = (1.0 - values[1]).sqrt();
        let a = single_pass(&self.guide, wavelength, values[0], self.round_trip_loss);
        Ok((
            finite(
                "the power off resonance",
                (r + a).powi(2) / (1.0 + r * a).powi(2),
            )?,
            finite(
                "the power on resonance",
                (r - a).powi(2) / (1.0 - r * a).powi(2),
            )?,
        ))
    }
}

impl Component for AllPassRing {
    fn kind(&self) -> &str {
        "all-pass ring"
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        check(self.kind(), &self.parameters, values)?;
        closed_form(2, values, |v| Ok(self.entries(wavelength, v)))
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Analytic,
            source: format!(
                "W. Bogaerts et al., Laser Photonics Rev. 6, 47 (published online 13 September \
                 2011; the January 2012 issue), doi:10.1002/lpor.201100017, Eq. 1, a point \
                 coupler and a ring of n_eff {}, n_g {} at {} um, {} dB/cm and {} dB per round \
                 trip",
                self.guide.effective_index,
                self.guide.group_index,
                self.guide.wavelength.to_um(),
                self.guide.loss_db_per_cm,
                self.round_trip_loss
            ),
            error: Some(0.0),
            validity: None,
        }
    }

    /// Exact, except at a coupling of 0 or 1, where r or k has no derivative: `None` there.
    fn derivatives(&self, wavelength: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        check(self.kind(), &self.parameters, values)?;
        if values[1] <= 0.0 || values[1] >= 1.0 {
            return Ok(None);
        }
        closed_form_derivatives(2, values, |v| Ok(self.entries(wavelength, v)))
    }
}

/// An add-drop ring: a ring between two buses, ports `in`, `through`, `add` and `drop`. With
/// r₁, k₁ the input bus's coupler and r₂, k₂ the drop bus's, A = a e^(iφ) the round trip and
/// D = 1 − r₁r₂A,
///
/// - in → through: (r₁ − r₂A)/D, whose power is Bogaerts's Eq. 5;
/// - in → drop: −k₁k₂ √A / D (√A the half round trip, e^(iγL/2)), whose power is his Eq. 6;
/// - add → drop: (r₂ − r₁A)/D; add → through: as in → drop;
///
/// and the same backwards (reciprocal); in and add, through and drop, don't couple. The two
/// halves of the ring are each L/2. Parameters `length`, `coupling` (k₁²) and `coupling_drop`
/// (k₂²).
///
/// Lossless (a = 1), it is unitary.
#[derive(Clone, Debug, PartialEq)]
pub struct AddDropRing {
    guide: Dispersion,
    round_trip_loss: f64,
    ports: Vec<Port>,
    parameters: Vec<Parameter>,
}

impl AddDropRing {
    /// A ring whose waveguide's mode has dispersion `guide`.
    ///
    /// # Errors
    ///
    /// As [`AllPassRing::new`].
    pub fn new(guide: Dispersion) -> Result<AddDropRing> {
        check_guide(&guide)?;
        Ok(AddDropRing {
            ports: ring_ports(&guide, &["in", "through", "add", "drop"]),
            parameters: ring_parameters(true),
            guide,
            round_trip_loss: 0.0,
        })
    }

    /// The same ring, losing `db` more per round trip, half in each half (Bogaerts Eq. 19).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a loss that isn't finite and at least zero.
    pub fn with_round_trip_loss(mut self, db: f64) -> Result<AddDropRing> {
        check_loss(db)?;
        self.round_trip_loss = db;
        Ok(self)
    }

    /// Its waveguide's dispersion.
    pub fn guide(&self) -> &Dispersion {
        &self.guide
    }

    fn entries(&self, wavelength: Wavelength, values: &[Dual]) -> Entries {
        let (length, k1, k2) = (values[0], values[1], values[2]);
        let i = c64::new(0.0, 1.0);
        let gamma = self.guide.propagation(wavelength);
        let round =
            length.scale(i * gamma).exp() * super::amplitude(Dual::real(self.round_trip_loss));
        let half = length.scale(i * gamma / 2.0).exp()
            * super::amplitude(Dual::real(self.round_trip_loss / 2.0));
        let one = Dual::real(1.0);
        let (r1, r2) = ((one - k1).sqrt(), (one - k2).sqrt());
        let d = one - r1 * r2 * round;
        let through = (r1 - r2 * round) / d;
        let back = (r2 - r1 * round) / d;
        let dropped = -(k1.sqrt() * k2.sqrt() * half) / d;
        let mut e = Vec::with_capacity(8);
        e.extend(both(1, 0, through));
        e.extend(both(3, 2, back));
        e.extend(both(3, 0, dropped));
        e.extend(both(1, 2, dropped));
        e
    }

    /// The resonance nearest `near` for a ring of `length` µm (Bogaerts Eq. 3).
    ///
    /// # Errors
    ///
    /// As [`AllPassRing::resonance`].
    pub fn resonance(&self, near: Wavelength, length: f64) -> Result<Wavelength> {
        resonance(&self.guide, near, length)
    }

    /// The free spectral range at λ, µm (Bogaerts Eq. 9).
    ///
    /// # Errors
    ///
    /// As [`AllPassRing::fsr`].
    pub fn fsr(&self, wavelength: Wavelength, length: f64) -> Result<f64> {
        check_length(length)?;
        finite(
            "the free spectral range",
            wavelength.to_um().powi(2) / (self.guide.group_index_at(wavelength) * length),
        )
    }

    /// r₁r₂a.
    fn rra(&self, wavelength: Wavelength, values: &[f64]) -> f64 {
        (1.0 - values[1]).sqrt()
            * (1.0 - values[2]).sqrt()
            * single_pass(&self.guide, wavelength, values[0], self.round_trip_loss)
    }

    /// The FWHM at λ, µm: (1 − r₁r₂a)λ²/(π n_g L √(r₁r₂a)) (Bogaerts Eq. 8). `values` are the
    /// parameters', `[length, coupling, coupling_drop]`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `values` are a positive, finite length and two couplings
    /// from 0 to 1, or if the width isn't finite: a ring that couples all its light out
    /// (r₁r₂a = 0) has no resonance.
    pub fn fwhm(&self, wavelength: Wavelength, values: &[f64]) -> Result<f64> {
        check_values(values, 3)?;
        let x = self.rra(wavelength, values);
        finite(
            "the resonance's width",
            (1.0 - x) * wavelength.to_um().powi(2)
                / (PI * self.guide.group_index_at(wavelength) * values[0] * x.sqrt()),
        )
    }

    /// The finesse, π√(r₁r₂a)/(1 − r₁r₂a) (Bogaerts Eq. 23).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for `values` as in [`AddDropRing::fwhm`], or if the finesse
    /// isn't finite: a ring without loss or coupling (r₁r₂a = 1) has no linewidth.
    pub fn finesse(&self, wavelength: Wavelength, values: &[f64]) -> Result<f64> {
        check_values(values, 3)?;
        let x = self.rra(wavelength, values);
        finite("the finesse", PI * x.sqrt() / (1.0 - x))
    }

    /// The loaded Q, π n_g L √(r₁r₂a)/(λ(1 − r₁r₂a)) (Bogaerts Eq. 22).
    ///
    /// # Errors
    ///
    /// As [`AddDropRing::finesse`].
    pub fn q_factor(&self, wavelength: Wavelength, values: &[f64]) -> Result<f64> {
        check_values(values, 3)?;
        let x = self.rra(wavelength, values);
        finite(
            "the Q factor",
            PI * self.guide.group_index_at(wavelength) * values[0] * x.sqrt()
                / (wavelength.to_um() * (1.0 - x)),
        )
    }

    /// The through power off and on resonance, T_t and R_min, and the drop power on and off
    /// resonance, T_max and T_d (Bogaerts Eqs. 13–16).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for `values` as in [`AddDropRing::fwhm`], or if a power isn't
    /// finite (r₁r₂a = 1).
    pub fn extremes(&self, wavelength: Wavelength, values: &[f64]) -> Result<[f64; 4]> {
        check_values(values, 3)?;
        let r1 = (1.0 - values[1]).sqrt();
        let r2 = (1.0 - values[2]).sqrt();
        let a = single_pass(&self.guide, wavelength, values[0], self.round_trip_loss);
        let x = r1 * r2 * a;
        let k = (1.0 - r1 * r1) * (1.0 - r2 * r2) * a;
        let powers = [
            (r2 * a + r1).powi(2) / (1.0 + x).powi(2),
            (r2 * r2 * a * a - 2.0 * r1 * r2 * a + r1 * r1) / (1.0 - x).powi(2),
            k / (1.0 - x).powi(2),
            k / (1.0 + x).powi(2),
        ];
        for p in powers {
            finite("a power at the extremes", p)?;
        }
        Ok(powers)
    }
}

impl Component for AddDropRing {
    fn kind(&self) -> &str {
        "add-drop ring"
    }

    fn ports(&self) -> &[Port] {
        &self.ports
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn s_matrix(&self, wavelength: Wavelength, values: &[f64]) -> Result<SMatrix> {
        check(self.kind(), &self.parameters, values)?;
        closed_form(4, values, |v| Ok(self.entries(wavelength, v)))
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            fidelity: Fidelity::Analytic,
            source: format!(
                "W. Bogaerts et al., Laser Photonics Rev. 6, 47 (published online 13 September \
                 2011; the January 2012 issue), doi:10.1002/lpor.201100017, Eqs. 5-6 as \
                 fields, two point couplers and a ring of n_eff {}, n_g {} at {} um, {} dB/cm \
                 and {} dB per round trip",
                self.guide.effective_index,
                self.guide.group_index,
                self.guide.wavelength.to_um(),
                self.guide.loss_db_per_cm,
                self.round_trip_loss
            ),
            error: Some(0.0),
            validity: None,
        }
    }

    /// Exact, except at a coupling of 0 or 1: `None` there.
    fn derivatives(&self, wavelength: Wavelength, values: &[f64]) -> Result<Option<Vec<SMatrix>>> {
        check(self.kind(), &self.parameters, values)?;
        if values[1..].iter().any(|&k| k <= 0.0 || k >= 1.0) {
            return Ok(None);
        }
        closed_form_derivatives(4, values, |v| Ok(self.entries(wavelength, v)))
    }
}
