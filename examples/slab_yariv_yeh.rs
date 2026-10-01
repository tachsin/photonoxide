//! The guided TE modes of two slabs, against Yariv and Yeh's worked examples.
//!
//! A. Yariv, P. Yeh, *Photonics: Optical Electronics in Modern Communications*, 6th ed.,
//! Oxford University Press (2007), a textbook without a DOI. Effective indices printed to 4
//! decimals:
//!
//! - Section 3.2, p. 123 and Figs. 3.8–3.9: an asymmetric slab, n₁ = 1.0, n₂ = 2.0, n₃ = 1.7,
//!   t/λ = 1: two TE modes, 1.9594 and 1.8375, and two TM modes;
//! - Section 3.1, pp. 117–118: a symmetric slab, n = 1.5 / 1.6 / 1.5, d = 5 µm, λ = 1.55 µm
//!   (V = 5.6425): four TE modes, 1.5946, 1.5785, 1.5521 and 1.5175; TE_m and TM_m share a
//!   cutoff, V = mπ/2 (p. 117), so four TM modes.
//!
//! ```sh
//! cargo run --release --example slab_yariv_yeh
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::mode::Polarization;
use photonoxide::mode::slab::Slab;
use photonoxide::units::{Length, Wavelength};

/// Checks a slab's TE effective indices, and its number of guided modes of each polarization.
fn check(
    checks: &mut common::Checks,
    slab: Slab,
    wavelength: Wavelength,
    te: &[f64],
    tm_count: usize,
) {
    let modes = slab.modes(Polarization::Te, wavelength);
    checks.count("TE modes guided", modes.len(), te.len());
    for (m, printed) in modes.iter().zip(te) {
        checks.compare(
            &format!("TE{} effective index", m.order()),
            m.effective_index(),
            *printed,
            5e-5,
        );
    }
    let tm = slab.modes(Polarization::Tm, wavelength).len();
    checks.count("TM modes guided", tm, tm_count);
}

fn main() -> ExitCode {
    let mut checks = common::Checks::default();

    println!("asymmetric slab: n = 1.0 / 2.0 / 1.7, t = λ = 1 µm (Section 3.2), exact: no grid");
    let slab = Slab::new(1.0, 2.0, 1.7, Length::um(1.0)).expect("a valid slab");
    check(
        &mut checks,
        slab,
        Wavelength::um(1.0).expect("a valid wavelength"),
        &[1.9594, 1.8375],
        2,
    );

    println!(
        "symmetric slab: n = 1.5 / 1.6 / 1.5, d = 5 µm, λ = 1.55 µm (Section 3.1), exact: no grid"
    );
    let slab = Slab::new(1.5, 1.6, 1.5, Length::um(5.0)).expect("a valid slab");
    // TE_m and TM_m share the cutoff V = mπ/2: four of each at V = 5.6425 < 2π
    check(
        &mut checks,
        slab,
        Wavelength::um(1.55).expect("a valid wavelength"),
        &[1.5946, 1.5785, 1.5521, 1.5175],
        4,
    );

    checks.finish()
}
