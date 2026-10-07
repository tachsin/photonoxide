//! The quality factor of silicon ring resonators against their length, at critical coupling.
//!
//! W. Bogaerts et al., "Silicon microring resonators", Laser Photonics Rev. 6, 47 (2012),
//! [doi:10.1002/lpor.201100017](https://doi.org/10.1002/lpor.201100017), Section 2.4 and
//! Fig. 5: rings losing 2.7 dB/cm in their waveguide, plus 0.04 dB in their bends and 0.035 dB
//! in their coupler (all-pass) or 0.07 dB in their two couplers (add-drop) per round trip (his
//! Eq. 19), at critical coupling (r = a for the all-pass ring, r₁ = r₂a with r₂ = 0.99 for the
//! add-drop ring). A longer ring stores the light longer, but loses more of it per round trip:
//! the loaded Q (his Eqs. 20 and 22) peaks. "The highest Q-factor that can be obtained under
//! the given conditions is about 1.42·10⁵ with an APF resonator of approximately 10 mm
//! roundtrip length. The highest Q-factor for an add-drop resonator would be 1.36·10⁵ for almost
//! 13 mm in length."
//!
//! The lengths of the peaks, and the ratio of the two peaks (1.42/1.36, each printed to three
//! digits), depend only on the losses: Q is n_g/λ times a function of the length. The peaks
//! themselves need the group index Fig. 5 was drawn with, which the paper doesn't give; with
//! the 4.30 it measured (Section 3.3.2, Fig. 13) they come out 6 % below the text's. Each Q is
//! photonoxide's [`AllPassRing::q_factor`] and [`AddDropRing::q_factor`], the paper's formulas;
//! the last lines measure the FWHM on the ring's own spectrum instead, to show what those
//! formulas (a Lorentzian line, ra near 1) leave out at a finesse of 5.
//!
//! ```sh
//! cargo run --release --example ring_q_factor
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::circuit::Component;
use photonoxide::circuit::components::{AddDropRing, AllPassRing, Dispersion};
use photonoxide::units::Wavelength;

/// The single-pass amplitude of a ring `length` µm round losing 2.7 dB/cm and `fixed` dB.
fn single_pass(length: f64, fixed: f64) -> f64 {
    10f64.powf(-(2.7 * length * 1e-4 + fixed) / 20.0)
}

/// The length (µm) in [lo, hi] where `f` peaks, by golden-section search, and the peak.
fn peak(f: impl Fn(f64) -> f64, mut lo: f64, mut hi: f64) -> (f64, f64) {
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let (mut a, mut b) = (hi - g * (hi - lo), lo + g * (hi - lo));
    let (mut fa, mut fb) = (f(a), f(b));
    while hi - lo > 1e-6 * hi {
        if fa > fb {
            hi = b;
            (b, fb) = (a, fa);
            a = hi - g * (hi - lo);
            fa = f(a);
        } else {
            lo = a;
            (a, fa) = (b, fb);
            b = lo + g * (hi - lo);
            fb = f(b);
        }
    }
    let x = (lo + hi) / 2.0;
    (x, f(x))
}

/// The FWHM (µm) of the dip of `through` power around the resonance `centre`, at half the
/// depth between it and `top`, by bisection on each side within a quarter of `fsr`.
fn measured_fwhm(
    through: impl Fn(f64) -> photonoxide::Result<f64>,
    centre: f64,
    top: f64,
    fsr: f64,
) -> photonoxide::Result<f64> {
    let bottom = through(centre)?;
    let half = (top + bottom) / 2.0;
    let mut edges = [0.0; 2];
    for (k, sign) in [-1.0, 1.0].into_iter().enumerate() {
        let (mut near, mut far) = (0.0, fsr / 4.0);
        for _ in 0..60 {
            let mid = (near + far) / 2.0;
            if through(centre + sign * mid)? < half {
                near = mid;
            } else {
                far = mid;
            }
        }
        edges[k] = (near + far) / 2.0;
    }
    Ok(edges[0] + edges[1])
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let l0 = Wavelength::um(1.55)?;
    let guide = Dispersion::new(l0, 2.4, 4.30).with_loss(2.7);
    let all_pass = AllPassRing::new(guide)?.with_round_trip_loss(0.075)?;
    let add_drop = AddDropRing::new(guide)?.with_round_trip_loss(0.11)?;
    let r2: f64 = 0.99;
    // critical coupling: r = a, so k² = 1 − a²; r₁ = r₂a, so k₁² = 1 − r₂²a²
    let q_all_pass = |l: f64| {
        let a = single_pass(l, 0.075);
        all_pass.q_factor(l0, &[l, 1.0 - a * a]).unwrap_or(f64::NAN)
    };
    let q_add_drop = |l: f64| {
        let a = single_pass(l, 0.11);
        add_drop
            .q_factor(l0, &[l, 1.0 - (r2 * a).powi(2), 1.0 - r2 * r2])
            .unwrap_or(f64::NAN)
    };
    let (la, qa) = peak(q_all_pass, 1e3, 3e4);
    let (ld, qd) = peak(q_add_drop, 1e3, 3e4);
    println!("Bogaerts's Eqs. 19-22 at critical coupling, 2.7 dB/cm, exact: no grid");
    println!(
        "  all-pass: Q peaks at {:.3} mm, {qa:.4e} with n_g 4.30",
        la / 1e3
    );
    println!(
        "  add-drop: Q peaks at {:.3} mm, {qd:.4e} with n_g 4.30",
        ld / 1e3
    );
    let mut checks = common::Checks::default();
    // "approximately 10 mm" and "almost 13 mm"
    checks.compare("all-pass, best length (mm)", la / 1e3, 10.0, 1.0);
    checks.compare("add-drop, best length (mm)", ld / 1e3, 12.5, 0.5);
    // 1.42/1.36, each to three digits: ±0.005 in each
    checks.compare("peak Q, all-pass over add-drop", qa / qd, 1.0441, 0.0075);

    // the all-pass ring at its best length, from its spectrum: the FWHM measured, not Eq. 7
    let kappa2 = 1.0 - single_pass(la, 0.075).powi(2);
    let values = [la, kappa2];
    let centre = all_pass.resonance(l0, la)?;
    let fsr = all_pass.fsr(centre, la)?;
    let through = |w: f64| -> photonoxide::Result<f64> {
        Ok(all_pass.s_matrix(Wavelength::um(w)?, &values)?.power(1, 0))
    };
    let (top, _) = all_pass.extremes(centre, &values)?;
    let fwhm = measured_fwhm(through, centre.to_um(), top, fsr)?;
    println!(
        "  all-pass at {:.3} mm, its spectrum: FSR {:.2} pm, FWHM {:.3} pm (Eq. 7: {:.3}), Q {:.4e} (Eq. 20: {:.4e}), finesse {:.2}",
        la / 1e3,
        fsr * 1e6,
        fwhm * 1e6,
        all_pass.fwhm(centre, &values)? * 1e6,
        centre.to_um() / fwhm,
        all_pass.q_factor(centre, &values)?,
        fsr / fwhm
    );
    Ok(checks.finish())
}
