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
    // the book's indices, oxide below and above the silicon film
    let (below, core, above, thickness) = (1.444, 3.473, 1.444, Length::nm(220.0));
    let slab = Slab::new(below, core, above, thickness).expect("a valid slab");
    let wavelength = Wavelength::um(1.55).expect("a valid wavelength");
    let tolerance = 5e-4;
    let mut setup = common::Setup::new();
    setup.layer("cladding", None, "oxide", &common::n(above));
    setup.layer("film", Some(thickness.to_um()), "silicon", &common::n(core));
    setup.layer("substrate", None, "oxide", &common::n(below));
    setup.wavelength(wavelength);
    setup.item("grid", "none: the slab's modes found exactly");
    setup.item("solver", "the slab's dispersion relation, its roots");
    setup.item(
        "tolerance",
        &format!("{tolerance:e} absolute (the book prints 3 decimals)"),
    );
    setup.medium(
        "silicon, oxide",
        "constant indices, the book's (Chrostowski & Hochberg 2015, Section 3.2.2)",
    );
    setup.print();
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
        checks.compare(&format!("{label}0 effective index"), n, printed, tolerance);
    }
    checks.finish()
}
