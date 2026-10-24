//! Jacques et al.'s TiN and N++ heaters through the thermo-optic phase shifter devices.
//!
//! M. Jacques, A. Samani, E. El-Fiky, D. Patel, Z. Xing, D. V. Plant, "Optimization of
//! thermo-optic phase-shifter design and mitigation of thermal crosstalk on the SOI platform",
//! Opt. Express 27, 10456 (2019), [doi:10.1364/OE.27.010456](https://doi.org/10.1364/OE.27.010456):
//! two heaters on 220 nm SOI with 2 µm of buried oxide, simulated (Lumerical DEVICE, a 40 × 18 µm
//! region, Table 3's materials) and measured on the same die, 320 µm long.
//!
//! - **The N++ heater:** two N++ strips 1.0 µm wide in the 90 nm slab, 0.8 µm beyond a 500 ×
//!   220 nm rib: simulated P_π 25.5 mW (Section 2.2, Fig. 1(e)), measured 22.8 mW (Section 3.1,
//!   Fig. 8, the MZI's arms 200 µm apart).
//! - **The TiN heater** as fabricated: 7.5 µm wide, 2 µm above a 500 × 220 nm strip: measured
//!   29.2 mW, their simulation 3.0 mW above it (Section 3.1).
//!
//! Each is photonoxide's device (`device::ThermoShifter::doped` and `::metal`, their defaults
//! these designs), at each of its fidelities: the analytic closed forms, the 2D model (the static
//! solver's rise and the full-vector mode's perturbation in the paper's region) with its grid
//! gated against one twice as fine, and the measured device. The paper doesn't give the TiN's
//! thickness or the oxide over the heaters: the devices take 100 nm of TiN under 1 µm of oxide,
//! and the oxide's top 3.32 µm over the buried oxide's for the N++.
//!
//! P_π is a 2D number, a straight section's (any length): the devices' 2D model is the paper's
//! own kind of simulation. The tolerances are 10% against their simulations (what the paper
//! doesn't state) and the devices' stated errors: 15% for the 2D model against the measurement,
//! and for the analytic against the 2D. The time constants and bandwidths are printed: the
//! paper's simulated time constants sit 3.6 µs above its measured ones (Section 3.2), and its
//! measured responses peak (its Fig. 10(a)).
//!
//! ```sh
//! cargo run --release --example thermo_jacques
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::circuit::Fidelity;
use photonoxide::convergence::Study;
use photonoxide::device::{Device, ThermoShifter, Values};

/// P_π (mW) of `d`'s 2D model at its defaults on its own grids and twice as fine.
fn p_pi(d: &ThermoShifter, r: f64) -> photonoxide::Result<f64> {
    let v = Values::defaults(d);
    Ok(d.clone()
        .with_refinement(r)?
        .shifter(&v)?
        .efficiency()?
        .p_pi(0, 0)
        * 1e3)
}

/// A tolerance taken from a computed value, to three decimals: what the output prints the same
/// on every system (a value's last bits differ between their maths libraries).
fn printed(tolerance: f64) -> f64 {
    (tolerance * 1e3).round() / 1e3
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let mut checks = common::Checks::default();
    println!(
        "Jacques et al. 2019's heaters through photonoxide's thermo-optic phase shifter devices, \
         at 1550 nm, 320 um long"
    );
    for (d, simulated, measured) in [
        (ThermoShifter::doped(), 25.5, 22.8),
        (ThermoShifter::metal(), 29.2 + 3.0, 29.2),
    ] {
        let v = Values::defaults(&d);
        println!();
        println!("{}:", d.kind());
        let mut figures = Vec::new();
        for f in [Fidelity::Analytic, Fidelity::TwoD, Fidelity::Measured] {
            let b = d.component(&v, f)?;
            let get = |s: &str| b.figure(s).map(|g| (g.value, g.grid.clone()));
            let (p, grid) = get("P_pi").expect("every model gives P_pi");
            let (tau, _) = get("tau").expect("every model gives tau");
            let (f3, _) = get("f_3dB").expect("every model gives f_3dB");
            println!(
                "  {:<9} P_pi {p:7.3} mW   tau {tau:6.2} us   f_3dB {f3:6.2} kHz",
                f.to_string()
            );
            println!("            ({grid})");
            figures.push(p);
        }
        // the 2D model's P_π gated by its grids
        let (coarse, fine) = (p_pi(&d, 1.0)?, p_pi(&d, 2.0)?);
        let c = Study::new("P_pi (mW)")
            .on(0.02, coarse)?
            .on(0.01, fine)?
            .check(printed(0.01 * fine))?;
        checks.converged("2D P_pi on grids 1 and 1/2, mW", &c);
        checks.compare("2D P_pi, mW (simulated)", fine, simulated, 0.1 * simulated);
        checks.compare("2D P_pi, mW (measured)", fine, measured, 0.15 * measured);
        checks.against(
            "analytic P_pi, mW",
            figures[0],
            "2D",
            figures[1],
            printed(0.15 * figures[1]),
        );
    }
    Ok(checks.finish())
}
