//! The TE and TM modes of a 220 nm silicon slab in oxide at 1550 nm, found exactly.
//!
//! L. Chrostowski, M. Hochberg, *Silicon Photonics Design*, Cambridge University Press (2015),
//! [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168), Section 3.2.2:
//! silicon 3.473, oxide 1.444, effective indices TE 2.845 and TM 2.051, printed to 3 decimals.
//!
//! ```sh
//! cargo run --release --example slab_soi
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::mode::Polarization;
use photonoxide::mode::slab::Slab;
use photonoxide::units::{Length, Wavelength};

pub fn main() -> ExitCode {
    let slab = Slab::new(1.444, 3.473, 1.444, Length::nm(220.0)).expect("a valid slab");
    let wavelength = Wavelength::um(1.55).expect("a valid wavelength");
    println!(
        "220 nm of 3.473 in 1.444 at 1550 nm (Chrostowski & Hochberg, Section 3.2.2), exact: no grid"
    );
    let mut checks = common::Checks::default();
    for (polarization, printed) in [(Polarization::Te, 2.845), (Polarization::Tm, 2.051)] {
        let n = slab
            .modes(polarization, wavelength)
            .first()
            .map_or(f64::NAN, |m| m.effective_index());
        let label = match polarization {
            Polarization::Te => "TE",
            Polarization::Tm => "TM",
        };
        checks.compare(&format!("{label}0 effective index"), n, printed, 5e-4);
    }
    checks.finish()
}
