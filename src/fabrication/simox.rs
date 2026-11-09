//! Silicon on insulator made by SIMOX, separation by implanted oxygen (K. Izumi, M. Doken,
//! H. Ariyoshi, Electron. Lett. 14, 593 (1978), doi:10.1049/el:19780397): oxygen ions shot deep
//! into a bare silicon wafer held hot, then a long anneal at a very high temperature, which
//! leaves a buried oxide under a film of the wafer's own silicon.
//!
//! What this module models, and from what:
//!
//! 1. **The buried oxide's thickness, from the dose: a dose balance.** After an anneal above
//!    1300 °C, "only the planar buried oxide remains" (G. K. Celler, S. Cristoloveanu, J. Appl.
//!    Phys. 93, 4955 (2003), doi:10.1063/1.1558223, p. 4959): every implanted oxygen atom is in
//!    it. So its thickness is the dose over the oxygen atoms a cubic centimetre of silicon
//!    dioxide holds ([`OXYGEN`]): twice its molecules, which are the silicon atoms per cm³
//!    (5×10²², S. A. Campbell, *Fabrication Engineering at the Micro- and Nanoscale*, 3rd ed.,
//!    p. 43) times the fraction of its thickness that growing an oxide consumes of silicon
//!    (44 %, p. 74): one silicon atom per molecule, so that fraction is the molecules' density
//!    over silicon's. 2 × 0.44 × 5×10²² = 4.4×10²² cm⁻³ ([`oxide`]).
//! 2. **The film from published points, not from the range.** The route uses the one published
//!    pair of energy and film the process gives: 200 keV and about 200 nm of silicon over the
//!    oxide (Celler, p. 4959), for doses of 1.2–2.2×10¹⁸ cm⁻², over which the film is 100–300 nm
//!    (H. W. Lam, IEEE Circuits Devices Mag. 3, 6 (1987), doi:10.1109/MCD.1987.6323126, p. 6).
//!    That film is the silicon left over the buried oxide after the anneal, not the oxygen's
//!    projected range Rp, which at 200 keV is about 400–470 nm: 398.2 nm measured by J. P. de
//!    Souza, M. Behar, J. F. Dias, J. H. R. dos Santos, Nucl. Instrum. Methods Phys. Res. B
//!    175–177, 46 (2001), doi:10.1016/S0168-583X(00)00525-5 (Table 1, p. 48: ¹⁸O by nuclear
//!    reaction analysis in silicon amorphized to 280 nm, so the 150 and 200 keV ranges lie
//!    beyond the amorphized layer, p. 47), TRIM's (1997) 8–17 % above that table (Table 1);
//!    SRIM2003.26 about 18 % above it (Ö. Kabadayi, H. Gümüş, Phys. Status Solidi B 242, 2391
//!    (2005), doi:10.1002/pssb.200540082, Fig. 3, p. 2397, and p. 2399); and 467.7 nm by our
//!    arithmetic from the LSS projected range (J. Lindhard, M. Scharff, H. E. Schiøtt, Mat. Fys.
//!    Medd. Dan. Vid. Selsk. 33, no. 14 (1963), ref:lindhard-scharff-schiott-1963, Eq. 4.5,
//!    p. 25), within about 6 % of SRIM's from 50 to 200 keV. The depth the oxide forms at isn't
//!    computed from the energy: going from Rp to the film would need how the oxide grows about
//!    the implanted profile, which no source in hand gives as a number. The model is calibrated
//!    on published points, not predictive: it refuses what they don't cover ([`route`]).
//! 3. **Thinning** to the design's film by a sacrificial oxide stripped in hydrofluoric acid, as
//!    Izumi did (600 Å of oxide grown and etched off, taking 300 Å of the surface, p. 593), with
//!    the oxidation of [`super::smart_cut`]'s thinning.
//!
//! Below 1300 °C the dose balance only bounds the oxide from above: part of the oxygen stays
//! in a transition region of oxide precipitates in the silicon, up to 20 % oxygen (Celler,
//! p. 4959). Izumi's 2100 Å after 1150 °C for 2 h (p. 593) and P. L. F. Hemment et al.'s 2800 ±
//! 140 Å as implanted (Vacuum 34, 203 (1984), doi:10.1016/0042-207X(84)90128-3, p. 204) are
//! 77 % and 68 % of the balance's 2727 Å and 4091 Å; [`oxide`] refuses those anneals.

use crate::{Error, Result};

use super::oxidation::{self, Ambient};
use super::smart_cut::{CONSUMED, HF_RATE, OXIDATION};

/// Silicon's atoms per cm³: "the number density of silicon is 5×10²² atoms/cm³" (Campbell,
/// Chapter 3, p. 43).
pub const SILICON_ATOMS: f64 = 5e22;

/// The oxygen atoms per cm³ of silicon dioxide: two per molecule, and the molecules
/// [`CONSUMED`] times silicon's atoms ([`SILICON_ATOMS`]), since each molecule takes one
/// silicon atom and growing the oxide consumes 44 % of its thickness of silicon. 4.4×10²²
/// cm⁻³.
pub const OXYGEN: f64 = 2.0 * CONSUMED * SILICON_ATOMS;

/// The implant's energy, keV: "for the 200 keV ion energy that was necessary to get a
/// reasonable thickness of about 200 nm of Si above the buried oxide, ∼2×10¹⁸ ions cm⁻² were
/// required" (Celler, p. 4959). The only energy whose film a source in hand gives.
pub const ENERGY: f64 = 200.0;

/// The energies published for SIMOX, keV: "150–200 keV" (Lam, p. 6); Campbell's "150 to
/// 300 keV" (Section 5.7, p. 128) and Celler's 65 keV for a thin oxide (p. 4960) lie beyond.
pub const ENERGIES: (f64, f64) = (150.0, 200.0);

/// The film over the oxide after the anneal, µm, at [`ENERGY`]: "about 200 nm" (Celler,
/// p. 4959).
pub const MADE: f64 = 0.2;

/// The films published for the doses of [`DOSES`], µm: "typically 100–300 nm thick" after
/// implantation and anneal (Lam, p. 6).
pub const MADE_RANGE: (f64, f64) = (0.1, 0.3);

/// The doses the route uses, cm⁻²: "typically 1.2–2.2×10¹⁸/cm²" at 150–200 keV (Lam, p. 6),
/// the doses whose film Lam gives ([`MADE_RANGE`]).
pub const DOSES: (f64, f64) = (1.2e18, 2.2e18);

/// The doses the dose balance takes, cm⁻²: from Celler's thinnest oxide, 2×10¹⁷ cm⁻² (p. 4960),
/// to Lam's highest dose ([`DOSES`]).
pub const BALANCE_DOSES: (f64, f64) = (2e17, 2.2e18);

/// The wafer's temperature during the implant, °C: "at T > 500 °C, and typically close to
/// 600 °C" (Celler, p. 4959).
pub const IMPLANT_TEMPERATURE: f64 = 600.0;

/// The temperatures published for the implant, °C: "400 °C or higher" (Lam, p. 6) to
/// Celler's "close to 600 °C" (p. 4959).
pub const IMPLANT_TEMPERATURES: (f64, f64) = (400.0, 600.0);

/// The anneal's temperature, °C: "currently, all SIMOX wafers are annealed … at temperatures
/// ∼1350 °C" (Celler, p. 4959).
pub const ANNEAL: f64 = 1350.0;

/// The least anneal temperature the dose balance holds for, °C: "at temperatures above 1300 °C
/// the critical radius approaches infinity — only the planar buried oxide remains" (Celler,
/// p. 4959); "the anneal conditions are at least 1300 °C for several hours" (Campbell,
/// Section 5.7, p. 129).
pub const ANNEAL_LEAST: f64 = 1300.0;

/// The anneal's time, hours: Celler's one stated time at 1350 °C, "4 h anneal at 1350 °C"
/// (p. 4960); Campbell's "several hours" (p. 129).
pub const ANNEAL_HOURS: f64 = 4.0;

/// The doses balance and route allow beyond their bounds, relative: the arithmetic's
/// round-off, so that a bound's own oxide is inside.
const ROUND_OFF: f64 = 1e-12;

fn invalid(message: String) -> Error {
    Error::invalid("simox", message)
}

/// The dose balance, µm: the oxide `dose` cm⁻² of oxygen atoms makes, all of them in it,
/// `dose` / [`OXYGEN`]. Unchecked: [`oxide`] refuses the conditions it doesn't hold for.
pub fn balance(dose: f64) -> f64 {
    dose / OXYGEN * 1e4
}

/// The buried oxide a dose of oxygen ions makes, µm, annealed at `anneal` °C: the dose balance,
/// `dose` / [`OXYGEN`].
///
/// # Errors
///
/// [`Error::InvalidValue`] for an anneal below [`ANNEAL_LEAST`], where oxide precipitates hold
/// part of the dose, and a dose outside [`BALANCE_DOSES`].
pub fn oxide(dose: f64, anneal: f64) -> Result<f64> {
    if !(anneal.is_finite() && anneal >= ANNEAL_LEAST) {
        return Err(invalid(format!(
            "below {ANNEAL_LEAST} °C part of the oxygen stays in oxide precipitates in the \
             silicon (Celler and Cristoloveanu 2003, p. 4959), which the dose balance doesn't \
             model: not {anneal} °C"
        )));
    }
    let (lo, hi) = BALANCE_DOSES;
    if !(dose.is_finite() && dose >= lo * (1.0 - ROUND_OFF) && dose <= hi * (1.0 + ROUND_OFF)) {
        return Err(invalid(format!(
            "the published doses run from {lo:e} to {hi:e} cm⁻², not {dose:e}"
        )));
    }
    Ok(balance(dose))
}

/// The SIMOX route to `film` µm of silicon on `oxide` µm of buried oxide: every number its
/// steps need.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct Route {
    /// The buried oxide, µm.
    pub oxide: f64,
    /// The film at the end, µm.
    pub film: f64,
    /// The oxygen dose, cm⁻²: the oxide's oxygen, `oxide` × [`OXYGEN`].
    pub dose: f64,
    /// The implant's energy, keV.
    pub energy: f64,
    /// The wafer's temperature during the implant, °C.
    pub implant_temperature: f64,
    /// The anneal's temperature, °C.
    pub anneal_temperature: f64,
    /// The anneal's time, hours.
    pub anneal_hours: f64,
    /// The silicon the oxide takes the place of, µm: [`CONSUMED`] of its thickness.
    pub consumed: f64,
    /// How much the surface rises, µm: the oxide beyond the silicon it replaces.
    pub swelling: f64,
    /// The film over the oxide after the anneal, µm.
    pub made: f64,
    /// What the sacrificial oxide takes off it, µm: the film as annealed less the design's.
    pub thinned: f64,
    /// The sacrificial oxide grown, µm (none when nothing is taken off).
    pub sacrificial: f64,
    /// Its oxidation, hours, in [`OXIDATION`]'s ambient and temperature.
    pub sacrificial_hours: f64,
    /// Its strip in 6:1 hydrofluoric acid, minutes.
    pub strip_minutes: f64,
}

/// The buried oxides the route makes, µm: [`DOSES`]' by the dose balance.
pub fn oxides() -> (f64, f64) {
    (balance(DOSES.0), balance(DOSES.1))
}

/// The SIMOX route to `film` µm of silicon on `oxide` µm of buried oxide.
///
/// # Errors
///
/// [`Error::InvalidValue`] unless both are positive and finite; for an oxide whose dose lies
/// outside [`DOSES`], the doses whose film a source gives; and for a film thicker than the
/// [`MADE`] the anneal leaves, which would need epitaxial silicon over it.
pub fn route(film: f64, oxide: f64) -> Result<Route> {
    if !(film.is_finite() && film > 0.0 && oxide.is_finite() && oxide > 0.0) {
        return Err(invalid(format!(
            "the film ({film} µm) and the buried oxide ({oxide} µm) must be positive"
        )));
    }
    let (lo, hi) = oxides();
    if oxide < lo * (1.0 - ROUND_OFF) || oxide > hi * (1.0 + ROUND_OFF) {
        return Err(invalid(format!(
            "SIMOX makes a buried oxide of {} to {} µm, the dose balance of the doses whose film \
             a source gives (1.2–2.2×10¹⁸ cm⁻², Lam 1987, p. 6); not {oxide} µm",
            super::stated::number(lo),
            super::stated::number(hi)
        )));
    }
    if film > MADE {
        return Err(invalid(format!(
            "the anneal leaves about {MADE} µm of silicon over the oxide (Celler and Cristoloveanu \
             2003, p. 4959); a {film} µm film needs epitaxial silicon grown over it (Izumi et al. \
             1978; Lam 1987), which the route doesn't model"
        )));
    }
    let dose = oxide * 1e-4 * OXYGEN;
    let consumed = CONSUMED * oxide;
    let thinned = MADE - film;
    let sacrificial = thinned / CONSUMED;
    let (ambient, temperature): (Ambient, f64) = OXIDATION;
    let (a, b, tau) = oxidation::constants(ambient, temperature)
        .ok_or_else(|| invalid("Deal and Grove's table has no such row".into()))?;
    Ok(Route {
        oxide,
        film,
        dose,
        energy: ENERGY,
        implant_temperature: IMPLANT_TEMPERATURE,
        anneal_temperature: ANNEAL,
        anneal_hours: ANNEAL_HOURS,
        consumed,
        swelling: oxide - consumed,
        made: MADE,
        thinned,
        sacrificial,
        sacrificial_hours: if thinned > 0.0 {
            oxidation::hours(a, b, tau, sacrificial)
        } else {
            0.0
        },
        strip_minutes: sacrificial / HF_RATE,
    })
}
