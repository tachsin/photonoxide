//! Harris et al.'s in-guide heater through the in-guide thermo-optic phase shifter device.
//!
//! N. C. Harris, Y. Ma, J. Mower, T. Baehr-Jones, D. Englund, M. Hochberg, C. Galland,
//! "Efficient, compact and low loss thermo-optic phase shifter in silicon", Opt. Express 22,
//! 10487 (2014), [doi:10.1364/OE.22.010487](https://doi.org/10.1364/OE.22.010487): a 61.6 µm
//! phase shifter on 220 nm SOI, a current across a ridge on a 90 nm slab, its p region 1.0 µm
//! wide across the guide and p++ beyond, heat channels 0.8 µm wide every 2.44 µm to the contacts
//! (Section 2, Fig. 1). Measured in an unbalanced MZI (Section 4.1): P_π 24.77 ± 0.43 mW, the
//! −3 dB frequency 130.0 ± 5.59 kHz, the insertion loss 0.23 ± 0.13 dB.
//!
//! The device (`device::ThermoShifter::in_guide`, its defaults this design) at each of its
//! fidelities: the analytic closed forms (the silicon as a fin to the contacts, the oxide's
//! spreading closed by a slab), the 2D model (the current by current continuity, its Joule heat
//! the static solver's drive in the paper's model's region: the buried oxide's bottom and the
//! oxide's top 10 µm over the silicon held, the channels averaged into the slab, the contacts
//! held) with its grid gated against one twice as fine, and the measured device. The paper
//! doesn't give the ridge's width, the channels' length or the slab between them: the device
//! takes a 500 nm ridge, the slab continuous to 1.0 µm from the guide's centre and the contacts
//! 3.0 µm out (`shifter/harris-*`'s).
//!
//! P_π is a 2D number, a straight section's (any length), and the periodic channels are
//! averaged: the 2D model is a section's, not the made device's 3D. The tolerances: 10% and the
//! measurement's uncertainty for P_π against the measured (what the paper doesn't give, and the
//! 3D), 20% and its uncertainty for the bandwidth, and the analytic model's stated 15% against
//! the 2D.
//!
//! ```sh
//! cargo run --release --example thermo_harris
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::circuit::Fidelity;
use photonoxide::convergence::Study;
use photonoxide::device::{Device, ThermoShifter, Values};

/// The device's 2D model at its defaults on its grids refined by `r`: P_π (mW) and the −3 dB
/// frequency (kHz).
fn two_d(d: &ThermoShifter, r: f64) -> photonoxide::Result<(f64, f64)> {
    let v = Values::defaults(d);
    let s = d.clone().with_refinement(r)?.shifter(&v)?;
    Ok((s.efficiency()?.p_pi(0, 0) * 1e3, s.cutoff(0, 0)? * 1e-3))
}

/// A tolerance taken from a computed value, to three decimals: what the output prints the same
/// on every system (a value's last bits differ between their maths libraries).
fn printed(tolerance: f64) -> f64 {
    (tolerance * 1e3).round() / 1e3
}

/// The tolerances: P_π (mW, 10 % and the measurement's uncertainty) and the −3 dB frequency
/// (kHz, 20 % and its uncertainty) against the measurement; the analytic model against the 2D,
/// and the 2D grid's, relative.
const P_PI: f64 = 2.9;
const F_3DB: f64 = 32.0;
const ANALYTIC: f64 = 0.15;
const GRID: f64 = 0.01;

pub fn main() -> photonoxide::Result<ExitCode> {
    let d = ThermoShifter::in_guide();
    let v = Values::defaults(&d);
    let lam = photonoxide::units::Wavelength::um(1.55)?;
    let mut setup = common::Setup::new();
    setup.device(&d, &v, lam);
    setup.wavelength(lam);
    setup.item(
        "grid",
        "the 2D model's own grids, and grids twice as fine (the gate); the analytic model's \
         closed forms",
    );
    setup.item(
        "solver",
        "current continuity for the heat, the static (heat) solver's rise, the full-vector \
         mode's perturbation (2D); the silicon as a fin and the oxide's spreading closed by a \
         slab (analytic); the paper's measurements (measured)",
    );
    setup.item(
        "tolerance",
        &format!(
            "{P_PI} mW on P_π and {F_3DB} kHz on f_3dB against the measurement; the analytic \
             within {} % of the 2D; the 2D grid within {} %",
            common::number(100.0 * ANALYTIC),
            common::number(100.0 * GRID)
        ),
    );
    setup.print();
    let mut checks = common::Checks::default();
    println!(
        "Harris et al. 2014's in-guide heater through photonoxide's in-guide thermo-optic phase \
         shifter device, at 1550 nm, 61.6 um long"
    );
    println!();
    let mut figures = Vec::new();
    for f in [Fidelity::Analytic, Fidelity::TwoD, Fidelity::Measured] {
        let b = d.component(&v, f)?;
        let get = |s: &str| b.figure(s).map(|g| g.value);
        let p = b.figure("P_pi").expect("every model gives P_pi");
        let tau = get("tau").map_or("    -".into(), |t| format!("{t:5.2}"));
        let f3 = get("f_3dB").expect("every model gives f_3dB");
        let il = get("IL").expect("every model gives IL");
        println!(
            "  {:<9} P_pi {:7.3} mW   tau {tau} us   f_3dB {f3:6.1} kHz   IL {il:.2} dB",
            f.to_string(),
            p.value
        );
        println!("            ({})", p.grid);
        figures.push(p.value);
    }
    // the 2D model gated by its grids
    let ((p1, f1), (p2, f2)) = (two_d(&d, 1.0)?, two_d(&d, 2.0)?);
    let c = Study::new("P_pi (mW)")
        .on(0.02, p1)?
        .on(0.01, p2)?
        .check(printed(GRID * p2))?;
    checks.converged("2D P_pi on grids 1 and 1/2, mW", &c);
    let c = Study::new("f_3dB (kHz)")
        .on(0.02, f1)?
        .on(0.01, f2)?
        .check(printed(GRID * f2))?;
    checks.converged("2D f_3dB on grids 1 and 1/2, kHz", &c);
    // 10% and the uncertainty, 2.9 mW; 20% and the uncertainty, 32 kHz
    checks.compare("2D P_pi, mW (measured)", p2, 24.77, P_PI);
    checks.compare("2D f_3dB, kHz (measured)", f2, 130.0, F_3DB);
    checks.against(
        "analytic P_pi, mW",
        figures[0],
        "2D",
        figures[1],
        printed(ANALYTIC * figures[1]),
    );
    Ok(checks.finish())
}
