//! Typed quantities and the conventions every solver follows.
//!
//! # Units
//!
//! Lengths are in micrometres. A frequency is in units of c/µm, the reciprocal of the vacuum
//! wavelength in micrometres: 1.55 µm is the frequency 1/1.55. This is the natural unit system
//! of electromagnetic solvers (Oskooi et al., Comput. Phys. Commun. 181, 687 (2010),
//! [doi:10.1016/j.cpc.2009.11.008](https://doi.org/10.1016/j.cpc.2009.11.008)): with c = 1,
//! a time step and a grid step have the same unit. [`Frequency::thz`] and
//! [`Frequency::to_thz`] convert to SI with the exact speed of light, [`SPEED_OF_LIGHT`].
//!
//! [`Length`] (a coordinate or a size), [`Wavelength`] (in vacuum) and [`Frequency`] are
//! different types, so one can't be passed for another.
//!
//! # The time convention
//!
//! A time-harmonic field is the real part of a complex amplitude times **e^(−iωt)**, the
//! physicists' convention. It fixes every sign:
//!
//! - a wave travelling towards +x is `e^(i(kx − ωt))`: at a fixed time its phase grows with x;
//! - in a passive, lossy medium the permittivity has a **positive** imaginary part, and so does
//!   the refractive index, n = n′ + iκ with κ ≥ 0 ([`refractive_index`]);
//! - the amplitude of a real signal s(t) at angular frequency ω is recovered with the kernel
//!   e^(+iωt): `A = (2/T) ∫ s(t) e^(iωt) dt` over whole periods T.

use std::f64::consts::TAU;
use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

use num_complex::Complex64;

use crate::{Error, Result};

/// The speed of light in vacuum, m/s: exact, by the definition of the metre (SI, 1983).
pub const SPEED_OF_LIGHT: f64 = 299_792_458.0;

/// A length or a coordinate, in micrometres. Any finite value, negative too.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Length(f64);

impl Length {
    /// Zero.
    pub const ZERO: Length = Length(0.0);

    /// A length of `value` micrometres.
    pub const fn um(value: f64) -> Length {
        Length(value)
    }

    /// A length of `value` nanometres.
    pub const fn nm(value: f64) -> Length {
        Length(value / 1000.0)
    }

    /// The length in micrometres.
    pub const fn to_um(self) -> f64 {
        self.0
    }

    /// The length in nanometres.
    pub const fn to_nm(self) -> f64 {
        self.0 * 1000.0
    }

    /// The absolute value.
    pub fn abs(self) -> Length {
        Length(self.0.abs())
    }
}

impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} um", self.0)
    }
}

impl Add for Length {
    type Output = Length;
    fn add(self, rhs: Length) -> Length {
        Length(self.0 + rhs.0)
    }
}

impl AddAssign for Length {
    fn add_assign(&mut self, rhs: Length) {
        self.0 += rhs.0;
    }
}

impl Sub for Length {
    type Output = Length;
    fn sub(self, rhs: Length) -> Length {
        Length(self.0 - rhs.0)
    }
}

impl SubAssign for Length {
    fn sub_assign(&mut self, rhs: Length) {
        self.0 -= rhs.0;
    }
}

impl Neg for Length {
    type Output = Length;
    fn neg(self) -> Length {
        Length(-self.0)
    }
}

impl Mul<f64> for Length {
    type Output = Length;
    fn mul(self, rhs: f64) -> Length {
        Length(self.0 * rhs)
    }
}

impl Mul<Length> for f64 {
    type Output = Length;
    fn mul(self, rhs: Length) -> Length {
        Length(self * rhs.0)
    }
}

impl Div<f64> for Length {
    type Output = Length;
    fn div(self, rhs: f64) -> Length {
        Length(self.0 / rhs)
    }
}

/// The ratio of two lengths.
impl Div for Length {
    type Output = f64;
    fn div(self, rhs: Length) -> f64 {
        self.0 / rhs.0
    }
}

/// A wavelength in vacuum: positive and finite.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Wavelength(f64);

impl Wavelength {
    /// A wavelength of `value` micrometres.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `value` is positive and finite.
    pub fn um(value: f64) -> Result<Wavelength> {
        if value.is_finite() && value > 0.0 {
            Ok(Wavelength(value))
        } else {
            Err(Error::invalid(
                "wavelength",
                format!("must be positive and finite, got {value} um"),
            ))
        }
    }

    /// A wavelength of `value` nanometres.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `value` is positive and finite.
    pub fn nm(value: f64) -> Result<Wavelength> {
        Wavelength::um(value / 1000.0).map_err(|_| {
            Error::invalid(
                "wavelength",
                format!("must be positive and finite, got {value} nm"),
            )
        })
    }

    /// The wavelength in micrometres.
    pub const fn to_um(self) -> f64 {
        self.0
    }

    /// The wavelength in nanometres.
    pub const fn to_nm(self) -> f64 {
        self.0 * 1000.0
    }

    /// The wavelength as a [`Length`].
    pub const fn length(self) -> Length {
        Length(self.0)
    }

    /// The frequency of light of this wavelength in vacuum.
    pub fn frequency(self) -> Frequency {
        Frequency(1.0 / self.0)
    }

    /// The vacuum wavenumber k₀ = 2π/λ, in 1/µm.
    pub fn wavenumber(self) -> f64 {
        TAU / self.0
    }
}

impl fmt::Display for Wavelength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} um", self.0)
    }
}

/// A frequency in units of c/µm (see the [module docs](self)): positive and finite.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Frequency(f64);

impl Frequency {
    /// A frequency of `value` c/µm, the reciprocal of the vacuum wavelength in micrometres.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `value` is positive and finite.
    pub fn natural(value: f64) -> Result<Frequency> {
        if value.is_finite() && value > 0.0 {
            Ok(Frequency(value))
        } else {
            Err(Error::invalid(
                "frequency",
                format!("must be positive and finite, got {value} c/um"),
            ))
        }
    }

    /// A frequency of `value` terahertz.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless `value` is positive and finite.
    pub fn thz(value: f64) -> Result<Frequency> {
        // f [c/um] = f [Hz] * 1 um / c
        Frequency::natural(value * 1e12 * 1e-6 / SPEED_OF_LIGHT).map_err(|_| {
            Error::invalid(
                "frequency",
                format!("must be positive and finite, got {value} THz"),
            )
        })
    }

    /// The frequency in c/µm.
    pub const fn to_natural(self) -> f64 {
        self.0
    }

    /// The frequency in terahertz.
    pub fn to_thz(self) -> f64 {
        self.0 * SPEED_OF_LIGHT / 1e-6 / 1e12
    }

    /// The angular frequency ω = 2πf, in units of c/µm.
    pub fn angular(self) -> f64 {
        TAU * self.0
    }

    /// The vacuum wavelength of light at this frequency.
    pub fn wavelength(self) -> Wavelength {
        Wavelength(1.0 / self.0)
    }
}

impl fmt::Display for Frequency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} c/um", self.0)
    }
}

/// The refractive index n = n′ + iκ of a relative permittivity ε: the square root with κ ≥ 0,
/// which is the physical branch under the e^(−iωt) convention (a passive medium attenuates).
///
/// For a real, negative ε (an ideal metal) n is purely imaginary, n = i√|ε|.
pub fn refractive_index(permittivity: Complex64) -> Complex64 {
    let n = permittivity.sqrt();
    if n.im < 0.0 { -n } else { n }
}

/// The relative permittivity ε = n² of a refractive index.
pub fn permittivity(refractive_index: Complex64) -> Complex64 {
    refractive_index * refractive_index
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol * b.abs().max(1.0)
    }

    #[test]
    fn lengths_convert_between_micrometres_and_nanometres() {
        assert_eq!(Length::nm(220.0).to_um(), 0.22);
        assert_eq!(Length::um(0.5).to_nm(), 500.0);
        assert_eq!(Length::um(1.0) + Length::nm(500.0), Length::um(1.5));
        assert_eq!(Length::um(3.0) / Length::um(1.5), 2.0);
        assert_eq!(-Length::um(2.0), Length::um(-2.0));
        assert_eq!(2.0 * Length::um(1.25), Length::um(2.5));
    }

    #[test]
    fn wavelengths_must_be_positive_and_finite() {
        assert!(Wavelength::um(1.55).is_ok());
        for bad in [0.0, -1.55, f64::NAN, f64::INFINITY] {
            let e = Wavelength::um(bad).unwrap_err();
            assert!(
                matches!(
                    e,
                    Error::InvalidValue {
                        what: "wavelength",
                        ..
                    }
                ),
                "{bad}"
            );
        }
        let e = Wavelength::nm(-1550.0).unwrap_err();
        assert!(e.to_string().contains("-1550 nm"), "{e}");
    }

    #[test]
    fn frequencies_must_be_positive_and_finite() {
        for bad in [0.0, -1.0, f64::NAN] {
            assert!(Frequency::natural(bad).is_err());
            assert!(Frequency::thz(bad).is_err());
        }
    }

    #[test]
    fn frequency_and_wavelength_are_reciprocal_in_natural_units() {
        let lam = Wavelength::um(1.55).unwrap();
        let f = lam.frequency();
        assert!(close(f.to_natural(), 1.0 / 1.55, 1e-15));
        assert!(close(f.wavelength().to_um(), 1.55, 1e-15));
        assert!(close(f.angular(), TAU / 1.55, 1e-15));
        assert!(close(lam.wavenumber(), TAU / 1.55, 1e-15));
    }

    #[test]
    fn terahertz_use_the_exact_speed_of_light() {
        // 1.55 um is c / 1.55e-6 m = 193.414489... THz
        let f = Wavelength::um(1.55).unwrap().frequency();
        assert!(close(f.to_thz(), 299_792_458.0 / 1.55e-6 / 1e12, 1e-14));
        assert!(close(f.to_thz(), 193.414_489_032_258, 1e-12));
        let back = Frequency::thz(f.to_thz()).unwrap();
        assert!(close(back.wavelength().to_um(), 1.55, 1e-14));
    }

    #[test]
    fn the_refractive_index_of_a_lossy_medium_attenuates() {
        // Im(eps) > 0 is loss under e^(-i w t); the index must have kappa > 0 as well
        let n = refractive_index(Complex64::new(12.0, 0.5));
        assert!(n.re > 0.0 && n.im > 0.0, "{n}");
        assert!(close(permittivity(n).re, 12.0, 1e-14) && close(permittivity(n).im, 0.5, 1e-14));
        // a wave e^(i(n k0 x - w t)) decays towards +x
        let k0 = Wavelength::um(1.55).unwrap().wavenumber();
        let at = |x: f64| (Complex64::i() * n * k0 * x).exp().norm();
        assert!(at(1.0) < at(0.0));
    }

    #[test]
    fn a_lossless_dielectric_has_a_real_index() {
        let n = refractive_index(Complex64::new(2.085, 0.0));
        assert_eq!(n.im, 0.0);
        assert!(close(n.re, 2.085f64.sqrt(), 1e-15));
    }

    #[test]
    fn an_ideal_metal_has_an_imaginary_index() {
        let n = refractive_index(Complex64::new(-16.0, 0.0));
        assert!(n.re.abs() < 1e-15 && close(n.im, 4.0, 1e-15), "{n}");
    }

    #[test]
    fn a_forward_wave_has_a_phase_that_grows_with_x_and_falls_with_t() {
        // e^(i(kx - wt)): the convention every solver and monitor follows
        let (k, w) = (2.0, 3.0);
        let phase = |x: f64, t: f64| Complex64::new(0.0, k * x - w * t).exp().arg();
        assert!(phase(0.1, 0.0) > phase(0.0, 0.0));
        assert!(phase(0.0, 0.1) < phase(0.0, 0.0));
    }

    #[test]
    fn the_amplitude_of_a_real_signal_is_recovered_with_e_plus_i_w_t() {
        // s(t) = Re(A e^(-i w t)) and A = (2/T) sum s(t) e^(i w t) dt over whole periods
        let a = Complex64::new(0.7, -0.4);
        let w = TAU * 0.65;
        let periods = 20.0;
        let n = 20_000;
        let t_end = periods * TAU / w;
        let dt = t_end / n as f64;
        let mut sum = Complex64::new(0.0, 0.0);
        for i in 0..n {
            let t = (i as f64 + 0.5) * dt;
            let s = (a * Complex64::new(0.0, -w * t).exp()).re;
            sum += s * Complex64::new(0.0, w * t).exp() * dt;
        }
        let got = sum * (2.0 / t_end);
        assert!((got - a).norm() < 1e-9, "{got} vs {a}");
    }
}
