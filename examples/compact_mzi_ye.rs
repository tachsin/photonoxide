//! A Mach–Zehnder interferometer's S-parameters as a compact model: how many poles vector
//! fitting needs over the whole C and L bands, and over a narrow band about a carrier, for an
//! error below −60 dB at frequencies it wasn't fitted to.
//!
//! Y. Ye, D. Spina, Y. Xing, W. Bogaerts, T. Dhaene, "Numerical modeling of a linear photonic
//! system for accurate and efficient time-domain simulations", Photonics Res. 6, 560 (2018),
//! [doi:10.1364/PRJ.6.000560](https://doi.org/10.1364/PRJ.6.000560), Section 6.A: an MZI of two
//! ideal 50/50 couplers and arms of 150 and 100 µm, both of n_eff 2.35 and n_g 4.3 at 1.55 µm
//! and 200 dB/m. Its scattering parameters are fitted by vector fitting (Gustavsen and
//! Semlyen's, real poles and conjugate pairs), "aiming at a maximum absolute error of less than
//! −60 dB", checked on frequency samples kept apart from the fit: Model A over 187.5 to 200 THz
//! takes 67 poles, Model B over f_c ± Δ (f_c = 193.72 THz, Δ = 150 GHz) 8.
//!
//! Here the same MZI is photonoxide's netlist of two ideal couplers and two waveguides (n_eff
//! first order in λ about 1.55 µm, as their analytic model's dispersion), its 4 × 4 S fitted by
//! [`CompactModel::fit`] with real poles and conjugate pairs, as theirs, at the even of 2001
//! frequencies (Model A) or 201 (Model B), evenly spaced, and checked at the odd ones. A real
//! model's poles come in pairs here (an even count): the fewest that reach −60 dB, and the
//! next count down, which doesn't, are found and compared with the paper's.
//!
//! ```sh
//! cargo run --release --example compact_mzi_ye
//! ```

mod common;

use std::process::ExitCode;
use std::sync::Arc;

use photonoxide::circuit::components::{Coupler, Dispersion, Waveguide, mzi};
use photonoxide::circuit::{Circuit, Component, Spectrum};
use photonoxide::compact::{CompactModel, Options, Symmetry};
use photonoxide::units::Wavelength;

/// The speed of light, µm THz.
const C_UM_THZ: f64 = 299.792_458;

/// The paper's target, |ΔS| in dB.
const TARGET_DB: f64 = -60.0;

/// The MZI of Section 6.A: arms of 150 and 100 µm, n_eff 2.35 and n_g 4.3 at 1.55 µm, 2 dB/cm.
fn interferometer() -> photonoxide::Result<Circuit> {
    let wire = Dispersion::new(Wavelength::um(1.55)?, 2.35, 4.3);
    let guide: Arc<dyn Component> = Arc::new(Waveguide::new(wire));
    let coupler: Arc<dyn Component> = Arc::new(Coupler::new());
    let mut n = mzi(coupler.clone(), coupler, guide.clone(), guide)?;
    n.set("upper", "length", 150.0)?;
    n.set("lower", "length", 100.0)?;
    n.set("upper", "loss", 2.0)?;
    n.set("lower", "loss", 2.0)?;
    n.compile()
}

/// The circuit's spectrum at `count` frequencies evenly spaced from `from` to `to` THz, split
/// into the even-numbered (to fit) and the odd-numbered (to check).
fn spectra(c: &Circuit, from: f64, to: f64, count: usize) -> photonoxide::Result<[Spectrum; 2]> {
    let wavelengths = (0..count)
        .map(|k| Wavelength::um(C_UM_THZ / (from + (to - from) * k as f64 / (count - 1) as f64)))
        .collect::<photonoxide::Result<Vec<_>>>()?;
    let all = c.spectrum(&wavelengths)?;
    let part = |odd: bool| {
        let keep = |k: &usize| (k % 2 == 1) == odd;
        let w = (0..count)
            .filter(keep)
            .map(|k| all.wavelengths()[k])
            .collect();
        let m = (0..count)
            .filter(keep)
            .map(|k| all.matrices()[k].clone())
            .collect();
        Spectrum::new(all.ports().to_vec(), w, m)
    };
    Ok([part(false)?, part(true)?])
}

/// A real model of `poles` poles fitted to `fit`: its largest |ΔS_qp| over `check`, in dB.
fn held_out(fit: &Spectrum, check: &Spectrum, poles: usize) -> photonoxide::Result<f64> {
    let options = Options {
        symmetry: Symmetry::Real,
        ..Options::new(poles)
    };
    let model = CompactModel::fit(fit, &options)?;
    let mut worst = 0.0f64;
    for (w, s) in check.wavelengths().iter().zip(check.matrices()) {
        let m = model.s_matrix(*w, &[])?;
        for q in 0..4 {
            for p in 0..4 {
                worst = worst.max((m[(q, p)] - s[(q, p)]).norm());
            }
        }
    }
    Ok(20.0 * worst.log10())
}

/// The fewest poles (pairs) whose model is below the target at the frequencies left out, and the
/// errors with them and with two fewer.
fn fewest(c: &Circuit, from: f64, to: f64, count: usize) -> photonoxide::Result<(usize, f64, f64)> {
    let [fit, check] = spectra(c, from, to, count)?;
    let mut below = held_out(&fit, &check, 2)?;
    for poles in (4..=200).step_by(2) {
        let error = held_out(&fit, &check, poles)?;
        if error < TARGET_DB {
            return Ok((poles, error, below));
        }
        below = error;
    }
    Err(photonoxide::Error::InvalidValue {
        what: "example",
        reason: "no model of up to 200 poles reaches the target".into(),
    })
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let c = interferometer()?;
    println!(
        "Ye et al.'s MZI (Photonics Res. 6, 560, 2018): ideal 50/50 couplers, arms of 150 and 100 µm, \
         n_eff 2.35 and n_g 4.3 at 1.55 µm, 200 dB/m; a closed form, exact: no grid"
    );
    println!(
        "  real poles and conjugate pairs, each fit checked at the frequencies between those it was \
         fitted at; the target {TARGET_DB} dB in |ΔS|"
    );
    let mut checks = common::Checks::default();
    // Model A: 187.5 to 200 THz
    let (poles, error, fewer) = fewest(&c, 187.5, 200.0, 2001)?;
    println!(
        "  Model A, 187.5 to 200 THz, fitted at 1001 frequencies: {poles} poles {error:.0} dB, {} poles {fewer:.0} dB",
        poles - 2
    );
    checks.compare("Model A, poles for −60 dB (pairs)", poles as f64, 67.0, 1.0);
    // Model B: f_c ± 150 GHz
    let (fc, delta) = (193.72, 0.15);
    let (poles, error, fewer) = fewest(&c, fc - delta, fc + delta, 201)?;
    println!(
        "  Model B, {fc} THz ± 150 GHz, fitted at 101 frequencies: {poles} poles {error:.0} dB, {} poles {fewer:.0} dB",
        poles - 2
    );
    checks.count("Model B, poles for −60 dB", poles, 8);
    Ok(checks.finish())
}
