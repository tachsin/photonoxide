//! Fused silica's refractive index, against the paper its Sellmeier formula comes from.
//!
//! I. H. Malitson, "Interspecimen comparison of the refractive index of fused silica",
//! J. Opt. Soc. Am. 55, 1205 (1965), [doi:10.1364/JOSA.55.001205](https://doi.org/10.1364/JOSA.55.001205),
//! Table I: the index his Eq. (1) gives at each wavelength, printed to 6 decimals (20 °C).
//!
//! ```sh
//! cargo run --release --example silica_index
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::material;
use photonoxide::units::Wavelength;

pub fn main() -> ExitCode {
    let sio2 = material::silica();
    // (wavelength in µm, Malitson's computed index), visible to mid-infrared
    let rows = [
        (0.546074, 1.460078),
        (0.852111, 1.452465),
        (1.01398, 1.450242),
        (1.52952, 1.444268),
        (2.0581, 1.437224),
        (3.2439, 1.413118),
    ];
    let tolerance = 1e-6;
    let mut setup = common::Setup::new();
    setup.item(
        "wavelengths",
        &format!("{} µm", rows.map(|r| common::number(r.0)).join(", ")),
    );
    setup.item("grid", "none: the material's model, evaluated");
    setup.item(
        "tolerance",
        &format!("{tolerance:e} absolute (Malitson prints 6 decimals)"),
    );
    setup.material(
        &sio2,
        Wavelength::um(rows[3].0).expect("a valid wavelength"),
    );
    setup.print();
    println!("{} (Malitson 1965, Table I), no grid", sio2.name());
    let mut checks = common::Checks::default();
    for (um, printed) in rows {
        let n = sio2
            .refractive_index(Wavelength::um(um).expect("a valid wavelength"))
            .expect("inside Malitson's range")
            .re;
        // 6 printed decimals, at wavelengths some printed to only 5 or 6 digits
        checks.compare(&format!("n at {um} µm"), n, printed, tolerance);
    }
    checks.finish()
}
