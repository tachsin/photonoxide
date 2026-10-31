//! Coen et al.'s single soliton in a MgF₂ resonator, by the Lugiato–Lefever equation.
//!
//! S. Coen, H. G. Randle, T. Sylvestre, M. Erkintalo, "Modeling of octave-spanning Kerr
//! frequency combs using a generalized mean-field Lugiato–Lefever model", Opt. Lett. 38, 37
//! (2012), [doi:10.1364/OL.38.000037](https://doi.org/10.1364/OL.38.000037): their Fig. 2, a
//! critically coupled MgF₂ disc, FSR 18.2 GHz, γ = 0.032 /(W·km), β₂ = −13 ps²/km,
//! α = θ = 1.75 × 10⁻⁵, P_in = 55.6 mW, L = 11.9 mm and δ₀ = 0.0012 (p. 38), whose single
//! soliton is "∼400 fs" long (the inset of Fig. 2(a)).
//!
//! The ring's equation (`nonlinear::resonator::Ring`) on 512 modes, its approximate soliton
//! (`Resonator::soliton`) settled by split-step until |ψ|² changes by less than 10⁻¹⁰ of its peak
//! per unit time, at two steps, the finer one the estimate of our own error. The tolerance is
//! half a unit of the one significant digit of '∼400 fs', 50 fs: the paper states no other.
//!
//! ```sh
//! cargo run --release --example soliton_coen
//! ```

mod common;

use std::process::ExitCode;

use common::{Checks, Setup, number};
use photonoxide::nonlinear::pulse::Dispersion;
use photonoxide::nonlinear::resonator::{Ring, peak_width};

/// The steps of the split-step (normalized time) and the settling.
const STEPS: [f64; 2] = [4e-4, 2e-4];
const MODES: usize = 512;
const TOLERANCE_FS: f64 = 50.0;

/// The soliton's FWHM (fs) and peak power inside (W) on steps of `step`.
fn soliton(ring: &Ring, step: f64) -> photonoxide::Result<(f64, f64)> {
    let r = ring.resonator(MODES)?;
    let s = r
        .soliton(ring.d2())
        .ok_or_else(|| photonoxide::Error::InvalidValue {
            what: "soliton",
            reason: "none at this detuning and drive".into(),
        })?;
    let (end, _) = r.settle(&r.soliton_field(&s), step, 0.5, 1e-10, 200.0)?;
    let peak = end.iter().map(|v| v.norm_sqr()).fold(0.0, f64::max);
    Ok((
        ring.time_of(peak_width(&end)?) * 1e3,
        peak * ring.power_scale(),
    ))
}

/// `x` to `decimals` decimals: what the output prints the same on every system (a value's last
/// bits differ between their maths libraries).
fn shown(x: f64, decimals: i32) -> f64 {
    let s = 10f64.powi(decimals);
    (x * s).round() / s
}

pub fn main() -> ExitCode {
    let ring = Ring {
        length: 0.0119,
        roundtrip: 1e12 / 18.2e9,
        loss: 1.75e-5,
        coupling: 1.75e-5,
        detuning: 0.0012,
        gamma: 0.032e-3,
        dispersion: Dispersion::new(1.55, &[-13e-3]).expect("a dispersion"),
        pump: 0.0556,
    };
    let mut setup = Setup::new();
    setup.item(
        "resonator",
        "Coen et al.'s MgF2 disc, critically coupled (their Fig. 2)",
    );
    setup.item(
        "round trip",
        &format!("{} mm, FSR 18.2 GHz", number(ring.length * 1e3)),
    );
    setup.item(
        "loss and coupling",
        &format!("alpha = theta = {}", number(ring.loss)),
    );
    setup.item(
        "waveguide",
        &format!(
            "gamma {} /(W km), beta2 {} ps^2/km",
            number(ring.gamma * 1e3),
            number(ring.dispersion.beta(2) * 1e3)
        ),
    );
    setup.item(
        "pump",
        &format!(
            "{} mW, detuning {} rad per round trip",
            number(ring.pump * 1e3),
            number(ring.detuning)
        ),
    );
    setup.item(
        "normalized",
        &format!(
            "zeta0 {:.4}, f {:.4}, d2 {:.4e}",
            ring.normalized_detuning(),
            ring.normalized_drive(),
            ring.d2()
        ),
    );
    setup.wavelength(photonoxide::units::Wavelength::um(1.55).expect("a wavelength"));
    setup.item(
        "grid",
        &format!(
            "{MODES} modes; split-step of {} and {} in normalized time",
            number(STEPS[0]),
            number(STEPS[1])
        ),
    );
    setup.item(
        "solver",
        "the Lugiato-Lefever equation by split-step from the approximate soliton, settled to 1e-10 of the peak per unit time",
    );
    setup.item(
        "tolerance",
        &format!("{TOLERANCE_FS} fs: half a unit of the one digit of '~400 fs'"),
    );
    setup.medium(
        "MgF2",
        "not in the model: the paper's gamma, beta2 and losses are the whole resonator",
    );
    setup.print();

    println!("Coen et al. 2012's single soliton (Fig. 2) by the Lugiato-Lefever equation\n");
    let (coarse, fine) = match (soliton(&ring, STEPS[0]), soliton(&ring, STEPS[1])) {
        (Ok(c), Ok(f)) => (c, f),
        (Err(e), _) | (_, Err(e)) => {
            println!("the soliton didn't settle: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "  FWHM {:.1} fs on the coarser step, {:.1} fs on the finer; peak inside {:.2} kW",
        coarse.0,
        fine.0,
        fine.1 * 1e-3
    );
    println!(
        "  our own error, the two steps' difference: {:.2} fs\n",
        (coarse.0 - fine.0).abs()
    );
    let mut checks = Checks::default();
    checks.compare("soliton FWHM (fs)", shown(coarse.0, 1), 400.0, TOLERANCE_FS);
    println!(
        "\n  The paper's '~400 fs' is read as one significant digit; 444 fs passes only so (#595)."
    );
    checks.finish()
}
