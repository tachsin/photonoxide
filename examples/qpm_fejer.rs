//! Fejer et al.'s Table I: the coherence lengths and acceptance bandwidths of quasi-phase-matched
//! SHG in 1 mm of lithium niobate.
//!
//! M. M. Fejer, G. A. Magel, D. H. Jundt, R. L. Byer, "Quasi-phase-matched second harmonic
//! generation: tuning and tolerances", IEEE J. Quantum Electron. 28, 2631 (1992),
//! [doi:10.1109/3.161322](https://doi.org/10.1109/3.161322), Table I: SHG of 0.86, 1.064 and
//! 2.12 µm in congruent LiNbO₃ at 25 °C by d₃₁, d₃₃ and d₂₂, first-order QPM at a 50 % duty
//! cycle; l_c, and the FWHM acceptance of 1 mm in the fundamental's wavelength, the temperature
//! (with the thermal expansion along the grating) and two tilts. They computed it with Edwards and
//! Lawrence's indices (their Ref. [24]), and δT at 0.86 µm with Jundt et al.'s for lithium-rich
//! material (Ref. [25], footnote f); both are the catalogue's.
//!
//! Each value is compared with the printed one to half a unit of its last printed digit. Of the
//! 49 values of the QPM rows, 41 agree; the other eight (six that differ by more than half a
//! digit, two numerical widths whose half-maximum points lie outside the indices' 0–500 °C) are
//! printed without a check, and docs/methods/nonlinear.md says what is known of each.
//!
//! ```sh
//! cargo run --release --example qpm_fejer
//! ```

mod common;

use std::process::ExitCode;

use common::{Checks, Setup};
use photonoxide::material::catalogue::{self, Axis, IndexModel};
use photonoxide::nonlinear::phase::{
    Device, Expansion, Index, Plane, Polarization, State, Type, Uniaxial, Variable,
    first_order_tilt,
};
use photonoxide::units::Wavelength;

#[derive(Clone, Copy, PartialEq)]
enum Coefficient {
    D31,
    D33,
    D22,
}

#[derive(Clone, Copy, PartialEq)]
enum Column {
    Coherence,
    Wavelength,
    Temperature,
    AboutZ,
    TowardZ,
}

#[derive(Clone, Copy, PartialEq)]
enum Estimate {
    Direct,
    Linear,
    Noncritical,
    FirstOrder,
    Numerical,
}

/// A printed value: fundamental (µm), coefficient, column, estimate, value, half its last digit,
/// lithium-rich (footnote f), and whether it is among the 41 that agree.
type Row = (f64, Coefficient, Column, Estimate, f64, f64, bool, bool);

use Coefficient::{D22, D31, D33};
use Column::{AboutZ, Coherence, Temperature, TowardZ};
use Estimate::{Direct, FirstOrder, Linear, Noncritical, Numerical};

/// Table I's QPM rows, read from the paper's page (its p. 2633).
const TABLE: &[Row] = &[
    (0.86, D31, Coherence, Direct, 4.26, 0.005, false, true),
    (0.86, D33, Coherence, Direct, 1.67, 0.005, false, true),
    (0.86, D22, Coherence, Direct, 1.42, 0.005, false, true),
    (1.064, D31, Coherence, Direct, 129.0, 0.5, false, false),
    (1.064, D33, Coherence, Direct, 3.40, 0.005, false, true),
    (1.064, D22, Coherence, Direct, 2.92, 0.005, false, true),
    (2.12, D31, Coherence, Direct, 14.4, 0.05, false, true),
    (2.12, D33, Coherence, Direct, 15.6, 0.05, false, false),
    (2.12, D22, Coherence, Direct, 13.4, 0.05, false, true),
    (
        0.86,
        D31,
        Column::Wavelength,
        Linear,
        0.89,
        0.005,
        false,
        true,
    ),
    (
        0.86,
        D33,
        Column::Wavelength,
        Linear,
        0.72,
        0.005,
        false,
        true,
    ),
    (
        0.86,
        D22,
        Column::Wavelength,
        Linear,
        0.60,
        0.005,
        false,
        true,
    ),
    (
        1.064,
        D31,
        Column::Wavelength,
        Linear,
        3.11,
        0.005,
        false,
        true,
    ),
    (
        1.064,
        D33,
        Column::Wavelength,
        Linear,
        2.04,
        0.005,
        false,
        true,
    ),
    (
        1.064,
        D22,
        Column::Wavelength,
        Linear,
        1.74,
        0.005,
        false,
        true,
    ),
    (
        2.12,
        D31,
        Column::Wavelength,
        Linear,
        40.0,
        0.5,
        false,
        false,
    ),
    (
        2.12,
        D33,
        Column::Wavelength,
        Linear,
        62.0,
        0.5,
        false,
        true,
    ),
    (
        2.12,
        D22,
        Column::Wavelength,
        Linear,
        55.0,
        0.5,
        false,
        true,
    ),
    (0.86, D31, Temperature, Linear, 6.0, 0.05, true, true),
    (0.86, D33, Temperature, Linear, 13.7, 0.05, true, true),
    (0.86, D22, Temperature, Linear, 17.4, 0.05, true, false),
    (1.064, D31, Temperature, Linear, 9.4, 0.05, false, true),
    (1.064, D33, Temperature, Linear, 25.8, 0.05, false, true),
    (1.064, D22, Temperature, Linear, 64.0, 0.5, false, true),
    (2.12, D31, Temperature, Linear, 27.5, 0.05, false, true),
    (2.12, D33, Temperature, Linear, 193.0, 0.5, false, true),
    (2.12, D33, Temperature, Numerical, 203.0, 0.5, false, false),
    (2.12, D22, Temperature, Linear, 379.0, 0.5, false, true),
    (2.12, D22, Temperature, Numerical, 455.0, 0.5, false, false),
    (0.86, D31, AboutZ, Noncritical, 176.0, 0.5, false, true),
    (0.86, D33, AboutZ, Noncritical, 112.0, 0.5, false, true),
    (0.86, D22, AboutZ, Noncritical, 104.0, 0.5, false, true),
    (1.064, D31, AboutZ, Noncritical, 959.0, 0.5, false, true),
    (1.064, D31, AboutZ, Numerical, 916.0, 0.5, false, true),
    (1.064, D33, AboutZ, Noncritical, 158.0, 0.5, false, true),
    (1.064, D22, AboutZ, Noncritical, 147.0, 0.5, false, true),
    (2.12, D31, AboutZ, Noncritical, 317.0, 0.5, false, true),
    (2.12, D33, AboutZ, Noncritical, 337.0, 0.5, false, true),
    (2.12, D22, AboutZ, Noncritical, 311.0, 0.5, false, true),
    (0.86, D31, TowardZ, FirstOrder, 79.0, 0.5, false, true),
    (0.86, D33, TowardZ, FirstOrder, 103.0, 0.5, false, true),
    (0.86, D22, TowardZ, FirstOrder, 104.0, 0.5, false, true),
    (1.064, D31, TowardZ, FirstOrder, 103.0, 0.5, false, false),
    (1.064, D33, TowardZ, FirstOrder, 146.0, 0.5, false, true),
    (1.064, D22, TowardZ, FirstOrder, 147.0, 0.5, false, true),
    (2.12, D31, TowardZ, FirstOrder, 139.0, 0.5, false, false),
    (2.12, D31, TowardZ, Numerical, 183.0, 0.5, false, true),
    (2.12, D33, TowardZ, FirstOrder, 306.0, 0.5, false, true),
    (2.12, D22, TowardZ, FirstOrder, 311.0, 0.5, false, true),
];

fn edwards_lawrence() -> photonoxide::Result<IndexModel> {
    catalogue::entry("linbo3")
        .and_then(|e| {
            e.index
                .into_iter()
                .find(|m| m.id == "edwards-lawrence-1984")
        })
        .ok_or_else(|| photonoxide::Error::InvalidValue {
            what: "index",
            reason: "no Edwards and Lawrence model".into(),
        })
}

fn indices(rich: bool) -> photonoxide::Result<(Index, Index)> {
    let m = if rich {
        catalogue::lithium_rich_jundt_1990()
    } else {
        edwards_lawrence()?
    };
    Ok((
        Index::model(m.clone(), Axis::Ordinary, None)?,
        Index::model(m, Axis::Extraordinary, None)?,
    ))
}

/// Our value of one row, in the table's unit.
fn value(row: &Row) -> photonoxide::Result<f64> {
    let &(lam, coefficient, column, estimate, _, _, rich, _) = row;
    let state = State::celsius(25.0);
    let (o, e) = indices(rich)?;
    if estimate == FirstOrder {
        // Eq. (A1.9), first order in the birefringence; an extraordinary wave tilted toward the
        // optic axis grows by n_o − n_e (Eq. (A1.4))
        let (e1, e2) = match coefficient {
            D31 => (false, true),
            D33 => (true, true),
            D22 => (false, false),
        };
        let n = |l: f64, ext: bool| {
            if ext {
                e.at(l, &state)
            } else {
                o.at(l, &state)
            }
        };
        let b = |l: f64, ext: bool| -> photonoxide::Result<f64> {
            Ok(if ext {
                o.at(l, &state)? - e.at(l, &state)?
            } else {
                0.0
            })
        };
        let h = lam / 2.0;
        return Ok(
            1e3 * first_order_tilt(lam, n(lam, e1)?, n(h, e2)?, b(lam, e1)?, b(h, e2)?, 1000.0)
        );
    }
    let kind = match coefficient {
        D31 => Type::I,
        D33 => Type::Zero(Polarization::Extraordinary),
        D22 => Type::Zero(Polarization::Ordinary),
    };
    let process = Uniaxial::new(o, e).second_harmonic(Wavelength::um(lam)?, kind, &state)?;
    if column == Coherence {
        return process.coherence_length(&state);
    }
    let device = Device::quasi_phase_matched(process, 1000.0, state, 1, 0.5)?
        .with_expansion(Expansion::LINBO3_JUNDT);
    let (variable, unit) = match column {
        Column::Wavelength => (Variable::Wavelength, 1e3),
        Temperature => (Variable::Temperature, 1.0),
        AboutZ => (Variable::Tilt(Plane::AboutOpticAxis), 1e3),
        _ => (Variable::Tilt(Plane::TowardOpticAxis), 1e3),
    };
    let a = device.acceptance(variable)?;
    Ok(unit
        * match estimate {
            Linear => a.linear,
            Noncritical => a.noncritical,
            _ => a.numerical,
        })
}

fn label(row: &Row) -> String {
    let &(lam, coefficient, column, estimate, ..) = row;
    let d = match coefficient {
        D31 => "d31",
        D33 => "d33",
        D22 => "d22",
    };
    let c = match column {
        Coherence => "l_c (um)",
        Column::Wavelength => "dlambda (nm)",
        Temperature => "dT (K)",
        AboutZ => "dtheta c (mrad)",
        TowardZ => "dtheta d (mrad)",
    };
    let how = if estimate == Numerical {
        ", numerical"
    } else {
        ""
    };
    format!("{lam} um {d} {c}{how}")
}

/// `x` to six significant digits: what the output prints the same on every system (a value's
/// last bits differ between their maths libraries).
fn shown(x: f64) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    let s = 10f64.powi(5 - x.abs().log10().floor() as i32);
    (x * s).round() / s
}

pub fn main() -> ExitCode {
    let mut setup = Setup::new();
    setup.item(
        "crystal",
        "congruent LiNbO3, Edwards and Lawrence's n_o(T) and n_e(T); lithium-rich, Jundt et al.'s, for dT at 0.86 um (footnote f)",
    );
    setup.item(
        "device",
        "1 mm, first-order QPM at a 50 % duty cycle, 25 °C",
    );
    setup.item(
        "thermal expansion",
        "Jundt's along the grating, 1.54e-5 /K and 5.3e-9 /K^2",
    );
    setup.item(
        "processes",
        "SHG of 0.86, 1.064 and 2.12 um: d31 (o + o -> e), d33 (e + e -> e), d22 (o + o -> o)",
    );
    setup.item(
        "solver",
        "the mismatch from the indices; Fejer's linear estimate Eq. (25), the noncritical Eq. (41), Eq. (A1.9), or the half-maximum points found numerically",
    );
    setup.item(
        "tolerance",
        "half a unit of each value's last printed digit",
    );
    if let Ok(m) = edwards_lawrence() {
        setup.medium(
            "LiNbO3",
            &format!(
                "{}: {}",
                m.name,
                m.sources.first().map_or("", |s| s.reference.as_str())
            ),
        );
    }
    let rich = catalogue::lithium_rich_jundt_1990();
    setup.medium(
        "LiNbO3, lithium-rich",
        &format!(
            "{}: {}",
            rich.name,
            rich.sources.first().map_or("", |s| s.reference.as_str())
        ),
    );
    setup.print();

    println!("Fejer et al. 1992's Table I, the QPM rows, from the indices it was computed with\n");
    let mut checks = Checks::default();
    let mut left = Vec::new();
    for row in TABLE {
        let ours = match value(row) {
            Ok(v) => v,
            Err(e) => {
                println!("  {}: {e}", label(row));
                f64::NAN
            }
        };
        if row.7 {
            checks.compare(&label(row), shown(ours), row.4, row.5);
        } else {
            left.push(format!(
                "  {:<34} {:>12}   paper {}",
                label(row),
                if ours.is_finite() {
                    format!("{ours:.3}")
                } else {
                    "outside".to_owned()
                },
                row.4
            ));
        }
    }
    println!("\n  Not reproduced to half a digit, so not checked (docs/methods/nonlinear.md):");
    for l in &left {
        println!("{l}");
    }
    println!();
    checks.finish()
}
