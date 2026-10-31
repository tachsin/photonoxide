//! Pu et al.'s AlGaAs-on-insulator Kerr comb threshold, by the Lugiato–Lefever equation.
//!
//! M. Pu, L. Ottaviano, E. Semenova, K. Yvind, "Efficient frequency comb generation in
//! AlGaAs-on-insulator", Optica 3, 823 (2016),
//! [doi:10.1364/OPTICA.3.000823](https://doi.org/10.1364/OPTICA.3.000823): a ring 25 µm across
//! with an FSR of 995 GHz and a loaded Q of about 10⁵ (Fig. 4(a), p. 825), its guide 320 × 630 nm
//! with γ ∼ 660 /(W·m) (p. 823), reached the threshold of parametric oscillation at 3.1 mW in the
//! bus (Fig. 4(b)). The paper's own estimate is a band, from critical coupling (Q_C = 2Q_L) to
//! under-coupling (Q_C = 10Q_L), and the measured thresholds lie in it (Fig. 4(d)); the
//! comb and the Q were measured near 1590 nm (p. 824).
//!
//! Here the same band from the ring's mean-field equation (Coen et al.'s, `nonlinear::resonator`):
//! the homogeneous state turns unstable at |ψ|² = 1, which the drive reaches first at ζ₀ = 1 with
//! f² = 1, so the threshold in the bus is P_th = α³/(θγL), α the field's loss per round trip
//! (θ/2 and the intrinsic loss: ω₀t_R/(2Q_L)) and θ the coupler's power transmission
//! (ω₀t_R/Q_C). The library's instability rate checks that the homogeneous state is stable just
//! below and unstable just above that drive. The measured 3.1 mW must lie in the band.
//!
//! Pu et al.'s own formula (from their Ref. [29]), 1.54(π/2)(Q_C/2Q_L)n²LA_eff/(n₂λQ_L²), is
//! 1.54n²/n_g² times ours: their 1.54, which the mean-field equation's minimum doesn't have,
//! and the material's index n = 3.3 where the round trip takes the group index n_g = c/(FSR·L).
//!
//! ```sh
//! cargo run --release --example comb_pu
//! ```

mod common;

use std::f64::consts::PI;
use std::process::ExitCode;

use common::{Checks, Setup, number};
use photonoxide::nonlinear::pulse::{Dispersion, angular_frequency};
use photonoxide::nonlinear::resonator::{Ring, homogeneous_powers, instability_rate};

/// The paper's numbers.
const FSR_GHZ: f64 = 995.0;
const DIAMETER_UM: f64 = 25.0;
const Q_LOADED: f64 = 1e5;
const GAMMA: f64 = 660.0;
const WAVELENGTH_UM: f64 = 1.59;
const MEASURED_MW: f64 = 3.1;
const INDEX: f64 = 3.3;

/// The ring at a coupling quality factor `q_c` and a pump `pump` W, on resonance.
fn ring(q_c: f64, pump: f64) -> Ring {
    let roundtrip = 1e3 / FSR_GHZ; // ps
    let w0 = angular_frequency(WAVELENGTH_UM); // rad/ps
    // the field's loss per round trip and the coupler's power transmission
    let alpha = w0 * roundtrip / (2.0 * Q_LOADED);
    let theta = w0 * roundtrip / q_c;
    Ring {
        length: PI * DIAMETER_UM * 1e-6,
        roundtrip,
        loss: alpha,
        coupling: theta,
        detuning: alpha,
        gamma: GAMMA,
        dispersion: Dispersion::new(WAVELENGTH_UM, &[0.0]).expect("a dispersion"),
        pump,
    }
}

/// The threshold in the bus (W): the pump at which f = 1.
fn threshold(q_c: f64) -> f64 {
    let r = ring(q_c, 1.0);
    1.0 / r.normalized_drive().powi(2)
}

/// Whether the homogeneous state at ζ₀ = 1 and drive `f` is unstable for some mode's
/// dispersion a (the largest growth rate over a fine sweep of a).
fn unstable(f: f64) -> bool {
    let x = homogeneous_powers(1.0, f)[0];
    (0..4001).any(|k| instability_rate(1.0, x, -2.0 + 4e-3 * k as f64) > 0.0)
}

/// `x` to `decimals` decimals: what the output prints the same on every system (a value's last
/// bits differ between their maths libraries).
fn shown(x: f64, decimals: i32) -> f64 {
    let s = 10f64.powi(decimals);
    (x * s).round() / s
}

pub fn main() -> ExitCode {
    let (critical, under) = (2.0 * Q_LOADED, 10.0 * Q_LOADED);
    let r = ring(critical, 1.0);
    let group_index = r.roundtrip * 1e-12 * 299_792_458.0 / r.length;
    let mut setup = Setup::new();
    setup.item(
        "resonator",
        &format!(
            "Pu et al.'s AlGaAs-on-insulator ring, {} µm across: round trip {} µm, FSR {} GHz",
            number(DIAMETER_UM),
            number(r.length * 1e6),
            number(FSR_GHZ)
        ),
    );
    setup.item(
        "waveguide",
        &format!("320 × 630 nm, gamma {} /(W m) (the paper's)", number(GAMMA)),
    );
    setup.item(
        "quality factors",
        &format!(
            "loaded {}; coupling from {} (critical) to {} (under-coupled)",
            number(Q_LOADED),
            number(critical),
            number(under)
        ),
    );
    setup.wavelength(photonoxide::units::Wavelength::um(WAVELENGTH_UM).expect("a wavelength"));
    setup.item(
        "solver",
        "the mean-field (Lugiato-Lefever) equation's homogeneous state and its instability: f^2 = 1 at zeta0 = 1",
    );
    setup.item(
        "tolerance",
        "the measured threshold within the band from critical to under-coupling, as the paper's Fig. 4(d)",
    );
    setup.medium(
        "Al0.17Ga0.83As",
        "not in the model: the paper's gamma and Q are the whole resonator",
    );
    setup.print();

    println!("Pu et al. 2016's comb threshold, 995 GHz ring, by the mean-field equation\n");
    // the instability sets in at f = 1
    let below = unstable(0.999);
    let above = unstable(1.001);
    println!(
        "  the homogeneous state at zeta0 = 1: unstable at f = 0.999: {below}; at f = 1.001: {above}"
    );
    let (lo, hi) = (threshold(critical) * 1e3, threshold(under) * 1e3);
    println!(
        "  threshold in the bus: {lo:.3} mW critically coupled, {hi:.3} mW under-coupled (Q_C = 10 Q_L)"
    );
    let ratio = 1.54 * INDEX * INDEX / (group_index * group_index);
    println!(
        "  Pu et al.'s formula gives {:.3} to {:.3} mW: 1.54 n^2/n_g^2 = {ratio:.4} times ours (n_g = {group_index:.4})\n",
        lo * ratio,
        hi * ratio
    );
    let mut checks = Checks::default();
    checks.count(
        "instability onset at f = 1",
        usize::from(!below && above),
        1,
    );
    checks.against(
        "band's middle (mW)",
        shown(0.5 * (lo + hi), 3),
        "measured",
        MEASURED_MW,
        shown(0.5 * (hi - lo), 3),
    );
    checks.finish()
}
