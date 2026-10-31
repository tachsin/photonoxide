//! The Rayleigh criterion for a projection lithography: the smallest line it prints is about
//! W = k₁ λ / NA (Campbell, Eq. 7.18; Franssila, Eq. 10.1), with λ its exposure wavelength, NA its
//! lens's numerical aperture and k₁ a factor of the resist and the process, "typically" 0.8 down to
//! 0.4, with a theoretical limit of 0.25 ([`super::rules`]).
//!
//! A lithography's wavelength and numerical apertures are its cited capabilities; a design's
//! feature asks for the k₁ that prints it at the class's highest numerical aperture, which says
//! whether it lies in the typical range, needs resolution enhancement, or is beyond the limit.

use serde::{Deserialize, Serialize};

use super::library::Process;
use super::rules::{k1_limit, k1_typical};
use super::stated::{Quantity, Source, Unit};

/// The Rayleigh resolution k₁ λ / NA, in λ's unit.
pub fn resolution(k1: f64, wavelength: f64, na: f64) -> f64 {
    k1 * wavelength / na
}

/// The k₁ a feature `width` wide asks for at `wavelength` (same unit) and `na`.
pub fn k1_for(width: f64, wavelength: f64, na: f64) -> f64 {
    width * na / wavelength
}

/// Where a feature lies against a projection lithography's resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Reach {
    /// k₁ at or above the typical range's lower end: printed as a matter of course.
    Typical,
    /// k₁ between the theoretical limit and the typical range: printed only with resolution
    /// enhancement (tilted illumination, phase-shifting masks, proximity correction).
    Enhanced,
    /// k₁ below the theoretical limit: not printed by this class at any cited NA.
    Beyond,
}

/// A projection lithography's optics, as its sources state them, and what they print.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Optics {
    /// The process's id.
    pub process: String,
    /// The exposure wavelength, nm.
    pub wavelength_nm: f64,
    /// The lowest and highest cited numerical aperture.
    pub na: [f64; 2],
    /// The smallest line at k₁'s theoretical limit and the highest NA, nm.
    pub limit_nm: f64,
    /// The smallest line at the typical k₁'s lower end and the highest NA, nm: printed as a
    /// matter of course down to here.
    pub typical_nm: f64,
    /// The line at the typical k₁'s upper end and the lowest NA, nm: printed comfortably by any
    /// tool of the class.
    pub comfortable_nm: f64,
    /// The sources of the wavelength and the numerical apertures.
    pub sources: Vec<Source>,
}

impl Optics {
    /// The optics of `p`, if a source gives its wavelength and a numerical aperture.
    pub fn of(p: &Process) -> Option<Optics> {
        let mut sources = Vec::new();
        let wl = p.capabilities_of(&Quantity::Wavelength).find_map(|c| {
            let v = c.value.low()?;
            let nm = match c.value.unit()? {
                Unit::Nm => v,
                Unit::Um => v * 1000.0,
                _ => return None,
            };
            sources.push(c.value.source()?.clone());
            Some(nm)
        })?;
        let mut na = [f64::INFINITY, f64::NEG_INFINITY];
        for c in p.capabilities_of(&Quantity::NumericalAperture) {
            if let (Some(lo), Some(hi), Some(s)) = (
                c.value.low().or(c.value.high()),
                c.value.high().or(c.value.low()),
                c.value.source(),
            ) {
                na = [na[0].min(lo), na[1].max(hi)];
                sources.push(s.clone());
            }
        }
        if !na[0].is_finite() {
            return None;
        }
        let (k_lim, k_lo, k_hi) = (k1_limit().low()?, k1_typical().low()?, k1_typical().high()?);
        Some(Optics {
            process: p.id.clone(),
            wavelength_nm: wl,
            na,
            limit_nm: resolution(k_lim, wl, na[1]),
            typical_nm: resolution(k_lo, wl, na[1]),
            comfortable_nm: resolution(k_hi, wl, na[0]),
            sources,
        })
    }

    /// The k₁ a feature `width_nm` wide asks for at the highest NA.
    pub fn k1(&self, width_nm: f64) -> f64 {
        k1_for(width_nm, self.wavelength_nm, self.na[1])
    }

    /// Where a feature `width_nm` wide lies.
    pub fn reach(&self, width_nm: f64) -> Reach {
        let k1 = self.k1(width_nm);
        let typical = k1_typical().low().unwrap_or(0.4);
        let limit = k1_limit().low().unwrap_or(0.25);
        if k1 >= typical - 1e-12 {
            Reach::Typical
        } else if k1 >= limit - 1e-12 {
            Reach::Enhanced
        } else {
            Reach::Beyond
        }
    }
}
