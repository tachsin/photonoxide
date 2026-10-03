//! The first components: waveguide, bend, couplers, MMI, Y-branch, rings and the
//! Mach–Zehnder interferometer, from closed forms and from the mode solvers and 2D FDFD.
//!
//! Each is a [`Component`](super::Component): a model shared by every instance of it in a
//! netlist, its parameters' values passed in. They follow the [module](super)'s conventions
//! (e^(−iωt), power-normalized amplitudes, `s[(q, p)]` from port p into port q, phases at each
//! port's reference plane), and state where their S-matrix comes from
//! ([`provenance`](super::Component::provenance)). The closed forms give their exact
//! derivatives with respect to every parameter
//! ([`derivatives`](super::Component::derivatives)), for the circuit adjoint.
//!
//! | Component | Model | Parameters |
//! |---|---|---|
//! | [`Waveguide`] | e^(i γ L), γ from a [`Dispersion`]: n_eff, n_g and D at λ₀, and the loss; by hand or from the full-vector mode solver | `length`, `loss` |
//! | [`Bend`] | the same along an arc, the bent mode's index and radiation loss; from the full-vector solver's conformal map or the exact bent slab | `angle` |
//! | [`PhaseShifter`] | e^(iφ) | `phase` |
//! | [`Coupler`] | an ideal 2 × 2 coupler: through √(1 − κ²), across iκ | `coupling` |
//! | [`DirectionalCoupler`] | coupled-mode theory from the two supermodes: through cos(πΔn L/λ), across i sin(πΔn L/λ) | `length` |
//! | [`Mmi`] | 1 × 2 and 2 × 2 multimode interference, by guided-mode propagation analysis (Soldano & Pennings 1995) | `length`, `width` |
//! | [`YBranch`] | an ideal 50/50 splitter with an excess loss | `excess_loss` |
//! | [`AllPassRing`], [`AddDropRing`] | Bogaerts et al. 2012's closed forms, Eqs. 1, 5 and 6 as fields | `length`, `coupling`, `coupling_drop` |
//! | [`mzi`], [`mzi_y`] | a Mach–Zehnder interferometer as a netlist of the above | its instances' |
//! | [`Sampled`] | a sampled spectrum (a 2D FDFD run), interpolated in wavelength | none |
//!
//! docs/methods/components.md derives each model, with its references and its validity.

pub(crate) mod checks;
mod coupler;
mod dual;
mod mmi;
mod mzi;
mod ring;
mod sampled;
mod splitter;
mod waveguide;

#[cfg(test)]
mod tests;

pub use coupler::{Coupler, DirectionalCoupler, Supermodes};
pub use mmi::{Mmi, MmiKind, Planar};
pub use mzi::{mzi, mzi_closed_form, mzi_y};
pub use ring::{AddDropRing, AllPassRing};
pub use sampled::Sampled;
pub use splitter::YBranch;
pub use waveguide::{Bend, Dispersion, PhaseShifter, Waveguide};

use super::{Parameter, SMatrix};
use crate::mode::Polarization;
use crate::units::Wavelength;
use crate::{Error, Result};
use dual::Dual;

/// The nonzero entries of a closed-form S-matrix, `(q, p, S_qp)`, each with its derivative with
/// respect to one parameter.
type Entries = Vec<(usize, usize, Dual)>;

/// Both (q, p) and (p, q): a reciprocal pair of entries.
fn both(q: usize, p: usize, s: Dual) -> [(usize, usize, Dual); 2] {
    [(q, p, s), (p, q, s)]
}

/// The n-port S-matrix of `entries`' values.
fn values_of(n: usize, entries: &Entries) -> SMatrix {
    let mut s = SMatrix::zeros(n);
    for &(q, p, e) in entries {
        s[(q, p)] = e.v;
    }
    s
}

/// The n-port matrix of `entries`' derivatives.
fn derivatives_of(n: usize, entries: &Entries) -> SMatrix {
    let mut s = SMatrix::zeros(n);
    for &(q, p, e) in entries {
        s[(q, p)] = e.d;
    }
    s
}

/// A closed-form component's S-matrix, `entries` evaluated at `values`.
fn closed_form(
    n: usize,
    values: &[f64],
    entries: impl Fn(&[Dual]) -> Result<Entries>,
) -> Result<SMatrix> {
    Ok(values_of(n, &entries(&Dual::seeded(values, None))?))
}

/// A closed-form component's exact derivatives, one matrix per parameter.
fn closed_form_derivatives(
    n: usize,
    values: &[f64],
    entries: impl Fn(&[Dual]) -> Result<Entries>,
) -> Result<Option<Vec<SMatrix>>> {
    (0..values.len())
        .map(|k| Ok(derivatives_of(n, &entries(&Dual::seeded(values, Some(k)))?)))
        .collect::<Result<Vec<_>>>()
        .map(Some)
}

/// Refuses `values` unless there is one per parameter and each is allowed.
fn check(kind: &str, parameters: &[Parameter], values: &[f64]) -> Result<()> {
    if values.len() != parameters.len() {
        return Err(Error::invalid(
            "component",
            format!(
                "a {kind} takes {} parameter values, got {}",
                parameters.len(),
                values.len()
            ),
        ));
    }
    for (p, &v) in parameters.iter().zip(values) {
        if !p.allows(v) {
            return Err(Error::invalid(
                "component",
                format!(
                    "a {kind}'s {} must be from {} to {} {}, got {v}",
                    p.name, p.min, p.max, p.unit
                ),
            ));
        }
    }
    Ok(())
}

/// Refuses a wavelength outside `validity` (µm), if the model has one.
fn check_wavelength(
    kind: &str,
    validity: Option<(f64, f64)>,
    wavelength: Wavelength,
) -> Result<()> {
    let w = wavelength.to_um();
    match validity {
        Some((lo, hi)) if !(lo..=hi).contains(&w) => Err(Error::invalid(
            "component",
            format!("a {kind}'s model holds from {lo} to {hi} um, not at {w} um"),
        )),
        _ => Ok(()),
    }
}

/// The amplitude attenuation of `db` decibels of power loss: 10^(−db/20).
fn amplitude(db: Dual) -> Dual {
    (db * (-std::f64::consts::LN_10 / 20.0)).exp()
}

/// The polarization a port states for a guide modelled by its plane seen from above (the
/// effective index method's lateral slab, an MMI's section, a bent slab), whose 2D field has
/// `lateral` polarization. Ports follow the waveguides' convention, the full-vector solver's:
/// TE-like when E lies mainly in the chip's plane. A lateral TM mode (H normal to the plane, E
/// in it) is therefore the TE-like mode, and a lateral TE mode (E normal to the plane) the
/// TM-like one.
fn seen_from_above(lateral: Polarization) -> Polarization {
    match lateral {
        Polarization::Te => Polarization::Tm,
        Polarization::Tm => Polarization::Te,
    }
}

/// `x` to `decimals` places, for a provenance: a value that rounds to zero is printed as zero,
/// without the sign of a tiny negative one (a solver's round-off loss would read "-0.000").
fn fixed(x: f64, decimals: usize) -> String {
    let s = format!("{x:.decimals$}");
    match s.strip_prefix('-') {
        Some(magnitude) if magnitude.bytes().all(|b| b == b'0' || b == b'.') => {
            magnitude.to_owned()
        }
        _ => s,
    }
}
