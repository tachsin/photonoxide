//! Silicon on insulator made by Smart Cut, step by step: the physics each step needs, from the
//! letter that introduced the process (M. Bruel, Electron. Lett. 31, 1201 (1995),
//! doi:10.1049/el:19950805) and the tables it rests on.
//!
//! 1. **The donor's oxide.** A bare silicon wafer, the donor, is oxidized to the buried oxide's
//!    thickness by Deal and Grove's law ([`super::oxidation`]). The oxide grows from the
//!    silicon, consuming about 44 % of its own thickness of it ([`CONSUMED`]).
//! 2. **The hydrogen implant**, through that oxide, as Bruel prefers. The protons slow down
//!    crossing the oxide and come to rest in the silicon around their projected range. Both
//!    come from the proton ranges of ICRU Report 49 as NIST's PSTAR tabulates them ([`SILICON`],
//!    [`OXIDE`]): the oxide's CSDA range gives the energy left on entering the silicon (the
//!    residual range, R(E′) = R(E) − the oxide's mass thickness, the path through it taken as
//!    straight), and silicon's projected range at E′ the depth of the peak below the oxide
//!    ([`depth`]). The straggle is Campbell's estimate (Eq. 5.19), ΔRp ≈ (2/3) Rp √(Mᵢ Mₜ)/(Mᵢ +
//!    Mₜ), over the whole depth ([`straggle`]). The dose sets no depth: Bruel found the
//!    thickness set by the energy only ([`DOSE`]).
//! 3. **Bonding** to a second, bare silicon wafer, the handle, oxide down, at room temperature.
//! 4. **Splitting** at 400–600 °C along the implanted plane: the donor's silicon between the
//!    oxide and the peak stays on the handle, the film.
//! 5. **The bond strengthened** above 1000 °C.
//! 6. **A touch-up polish** of the split surface, which is micro-rough.
//! 7. **Thinning** to the design's film: a sacrificial thermal oxide grown from the film and
//!    stripped in hydrofluoric acid, which etches oxide far faster than silicon.
//!
//! How much the polish and the thinning take off, no source in hand states. The route takes the
//! damaged silicon as the implanted profile's near half out to two straggles from the peak (a
//! Gaussian, Campbell Eq. 5.14, holds 95 % of its dose within two): the polish takes the first
//! straggle, the sacrificial oxide the second. The split film is therefore the design's film
//! and two straggles ([`route`]).

use crate::{Error, Result};

use super::oxidation::{self, Ambient};

/// A row of PSTAR's table, as printed: the proton's energy (MeV), its CSDA range and its
/// projected range (g/cm²).
pub type Row = (f64, f64, f64);

/// Protons in silicon (PSTAR's material 14, 2.33 g/cm³), from 10 keV to 1 MeV: PSTAR's default
/// energies. S. Seltzer, NIST Standard Reference Database 124 (1993), doi:10.18434/T4NC7P,
/// from ICRU Report 49.
pub const SILICON: [Row; 45] = [
    (1.000e-02, 5.294e-05, 2.924e-05),
    (1.250e-02, 6.002e-05, 3.508e-05),
    (1.500e-02, 6.656e-05, 4.065e-05),
    (1.750e-02, 7.268e-05, 4.598e-05),
    (2.000e-02, 7.849e-05, 5.113e-05),
    (2.250e-02, 8.404e-05, 5.611e-05),
    (2.500e-02, 8.939e-05, 6.097e-05),
    (2.750e-02, 9.458e-05, 6.572e-05),
    (3.000e-02, 9.963e-05, 7.037e-05),
    (3.500e-02, 1.094e-04, 7.947e-05),
    (4.000e-02, 1.189e-04, 8.836e-05),
    (4.500e-02, 1.282e-04, 9.714e-05),
    (5.000e-02, 1.373e-04, 1.058e-04),
    (5.500e-02, 1.464e-04, 1.145e-04),
    (6.000e-02, 1.555e-04, 1.232e-04),
    (6.500e-02, 1.646e-04, 1.320e-04),
    (7.000e-02, 1.737e-04, 1.408e-04),
    (7.500e-02, 1.830e-04, 1.497e-04),
    (8.000e-02, 1.923e-04, 1.588e-04),
    (8.500e-02, 2.017e-04, 1.679e-04),
    (9.000e-02, 2.113e-04, 1.772e-04),
    (9.500e-02, 2.210e-04, 1.866e-04),
    (1.000e-01, 2.308e-04, 1.962e-04),
    (1.250e-01, 2.821e-04, 2.463e-04),
    (1.500e-01, 3.373e-04, 3.004e-04),
    (1.750e-01, 3.963e-04, 3.583e-04),
    (2.000e-01, 4.588e-04, 4.198e-04),
    (2.250e-01, 5.247e-04, 4.848e-04),
    (2.500e-01, 5.939e-04, 5.529e-04),
    (2.750e-01, 6.661e-04, 6.242e-04),
    (3.000e-01, 7.413e-04, 6.984e-04),
    (3.500e-01, 9.001e-04, 8.553e-04),
    (4.000e-01, 1.070e-03, 1.023e-03),
    (4.500e-01, 1.249e-03, 1.201e-03),
    (5.000e-01, 1.439e-03, 1.389e-03),
    (5.500e-01, 1.638e-03, 1.586e-03),
    (6.000e-01, 1.846e-03, 1.792e-03),
    (6.500e-01, 2.063e-03, 2.007e-03),
    (7.000e-01, 2.289e-03, 2.231e-03),
    (7.500e-01, 2.523e-03, 2.463e-03),
    (8.000e-01, 2.767e-03, 2.705e-03),
    (8.500e-01, 3.020e-03, 2.956e-03),
    (9.000e-01, 3.283e-03, 3.217e-03),
    (9.500e-01, 3.555e-03, 3.487e-03),
    (1.000e+00, 3.835e-03, 3.765e-03),
];

/// Protons in silicon dioxide (PSTAR's material 245, 2.32 g/cm³), from 10 keV to 1 MeV, as
/// [`SILICON`].
pub const OXIDE: [Row; 45] = [
    (1.000e-02, 6.140e-05, 3.423e-05),
    (1.250e-02, 7.003e-05, 4.133e-05),
    (1.500e-02, 7.798e-05, 4.808e-05),
    (1.750e-02, 8.540e-05, 5.453e-05),
    (2.000e-02, 9.242e-05, 6.074e-05),
    (2.250e-02, 9.910e-05, 6.674e-05),
    (2.500e-02, 1.055e-04, 7.255e-05),
    (2.750e-02, 1.117e-04, 7.820e-05),
    (3.000e-02, 1.177e-04, 8.371e-05),
    (3.500e-02, 1.291e-04, 9.438e-05),
    (4.000e-02, 1.401e-04, 1.047e-04),
    (4.500e-02, 1.506e-04, 1.146e-04),
    (5.000e-02, 1.609e-04, 1.244e-04),
    (5.500e-02, 1.709e-04, 1.340e-04),
    (6.000e-02, 1.807e-04, 1.434e-04),
    (6.500e-02, 1.903e-04, 1.527e-04),
    (7.000e-02, 1.999e-04, 1.619e-04),
    (7.500e-02, 2.094e-04, 1.711e-04),
    (8.000e-02, 2.188e-04, 1.802e-04),
    (8.500e-02, 2.282e-04, 1.893e-04),
    (9.000e-02, 2.375e-04, 1.984e-04),
    (9.500e-02, 2.469e-04, 2.076e-04),
    (1.000e-01, 2.562e-04, 2.167e-04),
    (1.250e-01, 3.035e-04, 2.630e-04),
    (1.500e-01, 3.521e-04, 3.108e-04),
    (1.750e-01, 4.027e-04, 3.606e-04),
    (2.000e-01, 4.554e-04, 4.126e-04),
    (2.250e-01, 5.106e-04, 4.671e-04),
    (2.500e-01, 5.682e-04, 5.241e-04),
    (2.750e-01, 6.284e-04, 5.836e-04),
    (3.000e-01, 6.910e-04, 6.457e-04),
    (3.500e-01, 8.245e-04, 7.780e-04),
    (4.000e-01, 9.696e-04, 9.219e-04),
    (4.500e-01, 1.126e-03, 1.077e-03),
    (5.000e-01, 1.294e-03, 1.244e-03),
    (5.500e-01, 1.472e-03, 1.421e-03),
    (6.000e-01, 1.659e-03, 1.607e-03),
    (6.500e-01, 1.856e-03, 1.802e-03),
    (7.000e-01, 2.063e-03, 2.007e-03),
    (7.500e-01, 2.278e-03, 2.222e-03),
    (8.000e-01, 2.502e-03, 2.445e-03),
    (8.500e-01, 2.736e-03, 2.677e-03),
    (9.000e-01, 2.978e-03, 2.918e-03),
    (9.500e-01, 3.229e-03, 3.167e-03),
    (1.000e+00, 3.488e-03, 3.425e-03),
];

/// The densities PSTAR gives its materials, g/cm³: (silicon, silicon dioxide).
pub const DENSITY: (f64, f64) = (2.33, 2.32);

/// The silicon thermal oxidation consumes, per thickness of oxide grown: "about 44 % of the
/// thickness of the final oxide" (S. A. Campbell, *Fabrication Engineering at the Micro- and
/// Nanoscale*, 3rd ed., Section 4.1, p. 74).
pub const CONSUMED: f64 = 0.44;

/// Bruel's film thickness per keV of implant energy in silicon, nm/keV: "∼8 nm/keV", with no
/// effect of the dose (PDF p. 2).
pub const BRUEL_NM_PER_KEV: f64 = 8.0;

/// Bruel's hydrogen doses, cm⁻²: from 2×10¹⁶ to 1×10¹⁷ (Fig. 1, step 1, PDF p. 1).
pub const DOSE: (f64, f64) = (2e16, 1e17);

/// The mass numbers of the proton and of silicon, as Campbell's Eq. 5.19 takes them (Mᵢ and Mₜ,
/// "neutrons plus protons", Eq. 5.13).
const MASSES: (f64, f64) = (1.0, 28.0);

/// The oxidations' ambient and temperature, °C: wet oxygen at 1100 °C, the row of Deal and
/// Grove's Table I the bare-wafer flows grow their oxide at.
pub const OXIDATION: (Ambient, f64) = (Ambient::Wet, 1100.0);

/// The rate at which 6:1 hydrofluoric acid etches thermal oxide, µm/min: "about 1200 Å/min"
/// (Campbell, Section 11.1, p. 285).
pub const HF_RATE: f64 = 0.12;

/// A column of `rows` at `mev`, linear in the logarithms of both between printed rows (the
/// printed values at a printed energy); `None` outside the table.
fn at(rows: &[Row], column: fn(&Row) -> f64, mev: f64) -> Option<f64> {
    let k = rows
        .windows(2)
        .position(|w| w[0].0 <= mev && mev <= w[1].0)?;
    let (a, b) = (&rows[k], &rows[k + 1]);
    if mev == a.0 {
        return Some(column(a));
    }
    if mev == b.0 {
        return Some(column(b));
    }
    let t = (mev.ln() - a.0.ln()) / (b.0.ln() - a.0.ln());
    Some((column(a).ln() + t * (column(b).ln() - column(a).ln())).exp())
}

/// The energy, MeV, at which a column of `rows` is `value`, as [`at`] reads the table; `None`
/// outside it.
fn energy_at(rows: &[Row], column: fn(&Row) -> f64, value: f64) -> Option<f64> {
    let k = rows
        .windows(2)
        .position(|w| column(&w[0]) <= value && value <= column(&w[1]))?;
    let (a, b) = (&rows[k], &rows[k + 1]);
    let t = (value.ln() - column(a).ln()) / (column(b).ln() - column(a).ln());
    Some((a.0.ln() + t * (b.0.ln() - a.0.ln())).exp())
}

fn csda(r: &Row) -> f64 {
    r.1
}

fn projected_column(r: &Row) -> f64 {
    r.2
}

/// Silicon's projected range for a proton of `kev`, µm; `None` outside the table.
pub fn projected(kev: f64) -> Option<f64> {
    at(&SILICON, projected_column, kev / 1000.0).map(|g| g / DENSITY.0 * 1e4)
}

/// The energy left, keV, after a proton of `kev` crosses `oxide` µm of silicon dioxide: the
/// residual range on the oxide's CSDA ranges; `None` if it stops in the oxide or leaves it below
/// the table's 10 keV.
pub fn through(kev: f64, oxide: f64) -> Option<f64> {
    let left = at(&OXIDE, csda, kev / 1000.0)? - oxide * 1e-4 * DENSITY.1;
    if oxide == 0.0 {
        return Some(kev);
    }
    if left <= 0.0 {
        return None;
    }
    energy_at(&OXIDE, csda, left).map(|mev| mev * 1000.0)
}

/// The depth of the implanted peak below `oxide` µm of oxide, µm, for a proton of `kev`.
pub fn depth(kev: f64, oxide: f64) -> Option<f64> {
    projected(through(kev, oxide)?)
}

/// The energy, keV, that puts the implanted peak `target` µm below `oxide` µm of oxide; `None`
/// when no energy the table holds does.
pub fn energy(target: f64, oxide: f64) -> Option<f64> {
    let (mut lo, mut hi) = (SILICON[0].0 * 1000.0, SILICON[SILICON.len() - 1].0 * 1000.0);
    if depth(hi, oxide)? < target || depth(lo, oxide).is_some_and(|d| d > target) {
        return None;
    }
    // the depth rises with the energy: bisect to the last bit of the interval
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            break;
        }
        match depth(mid, oxide) {
            Some(d) if d >= target => hi = mid,
            _ => lo = mid,
        }
    }
    Some(hi)
}

/// The straggle of a hydrogen implant whose peak lies `range` µm deep, µm: Campbell's Eq. 5.19,
/// ΔRp ≈ (2/3) Rp √(Mᵢ Mₜ)/(Mᵢ + Mₜ).
pub fn straggle(range: f64) -> f64 {
    let (mi, mt) = MASSES;
    2.0 / 3.0 * range * (mi * mt).sqrt() / (mi + mt)
}

/// Where silicon's projected range is proportional to the energy, as Bruel's ∼8 nm/keV reads
/// it: the energy at which Rp/E is least, keV, and Rp/E there, nm/keV. There the range's slope
/// equals its ratio, so both readings of "nm per keV" agree. Between printed rows Rp is a power
/// of E, so Rp/E is monotonic and its least value lies on a printed row.
pub fn proportional() -> (f64, f64) {
    SILICON
        .iter()
        .map(|r| {
            let kev = r.0 * 1000.0;
            (kev, r.2 / DENSITY.0 * 1e7 / kev)
        })
        .fold((f64::NAN, f64::INFINITY), |best, (kev, ratio)| {
            if ratio < best.1 { (kev, ratio) } else { best }
        })
}

/// The Smart Cut route to `film` µm of silicon on `oxide` µm of buried oxide: every number its
/// steps need.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Route {
    /// The buried oxide, µm: the donor's oxide.
    pub oxide: f64,
    /// The film at the end, µm.
    pub film: f64,
    /// The donor's oxidation, hours, in [`OXIDATION`]'s ambient and temperature.
    pub oxide_hours: f64,
    /// The silicon the donor's oxidation consumes, µm.
    pub oxide_consumed: f64,
    /// The implant's energy, keV.
    pub energy: f64,
    /// The energy the protons enter the silicon with, keV.
    pub entering: f64,
    /// The implanted peak's depth below the wafer's surface, µm: the oxide and the split film.
    pub range: f64,
    /// The implant's straggle, µm.
    pub straggle: f64,
    /// The film the split leaves, µm: the silicon between the oxide and the peak.
    pub split: f64,
    /// What it has beyond the design's film, µm: two straggles, which the polish and the
    /// sacrificial oxide take off.
    pub excess: f64,
    /// What the touch-up polish takes off, µm.
    pub polish: f64,
    /// The silicon the sacrificial oxidation consumes, µm.
    pub consumed: f64,
    /// The sacrificial oxide grown, µm.
    pub sacrificial: f64,
    /// Its oxidation, hours.
    pub sacrificial_hours: f64,
    /// Its strip in 6:1 hydrofluoric acid, minutes.
    pub strip_minutes: f64,
}

fn invalid(message: String) -> Error {
    Error::invalid("smart cut", message)
}

/// The Smart Cut route to `film` µm of silicon on `oxide` µm of buried oxide.
///
/// # Errors
///
/// [`Error::InvalidValue`] unless both are positive and finite, and when no energy in the range
/// table puts the implanted peak deep enough.
pub fn route(film: f64, oxide: f64) -> Result<Route> {
    if !(film.is_finite() && film > 0.0 && oxide.is_finite() && oxide > 0.0) {
        return Err(invalid(format!(
            "the film ({film} µm) and the buried oxide ({oxide} µm) must be positive"
        )));
    }
    // the split film is the design's and two straggles of the whole depth: s = f + 2k(o + s)
    let k = straggle(1.0);
    let spread = k * (oxide + (film + 2.0 * k * oxide) / (1.0 - 2.0 * k));
    // the polish and the sacrificial oxide take a straggle each off the excess: doubling is
    // exact, and so is taking one off it again, so the steps leave the design's film to the bit
    let excess = spread + spread;
    let split = film + excess;
    let energy = energy(split, oxide).ok_or_else(|| {
        invalid(format!(
            "no proton energy from 10 keV to 1 MeV puts the implanted peak {split} µm below \
             {oxide} µm of oxide"
        ))
    })?;
    let entering = through(energy, oxide).unwrap_or(f64::NAN);
    let (ambient, temperature) = OXIDATION;
    let (a, b, tau) = oxidation::constants(ambient, temperature)
        .ok_or_else(|| invalid("Deal and Grove's table has no such row".into()))?;
    let (polish, consumed) = (spread, excess - spread);
    let sacrificial = consumed / CONSUMED;
    Ok(Route {
        oxide,
        film,
        oxide_hours: oxidation::hours(a, b, tau, oxide),
        oxide_consumed: CONSUMED * oxide,
        energy,
        entering,
        range: oxide + split,
        straggle: spread,
        split,
        excess,
        polish,
        consumed,
        sacrificial,
        sacrificial_hours: oxidation::hours(a, b, tau, sacrificial),
        strip_minutes: sacrificial / HF_RATE,
    })
}
