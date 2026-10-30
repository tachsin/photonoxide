//! Silicon's refractive index, against the table it comes from.
//!
//! H. H. Li, "Refractive index of silicon and germanium and its wavelength and temperature
//! derivatives", J. Phys. Chem. Ref. Data 9, 561 (1980),
//! [doi:10.1063/1.555624](https://doi.org/10.1063/1.555624), Table 1 (293 K), printed to 4
//! decimals; Li gives its uncertainty as ±2 × 10⁻³.
//!
//! ```sh
//! cargo run --release --example silicon_index
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::material;
use photonoxide::units::Wavelength;

pub fn main() -> ExitCode {
    let si = material::silicon();
    // (wavelength in µm, Li's printed index)
    let rows = [
        (1.30, 3.5016),
        (1.55, 3.4757),
        (2.00, 3.4510),
        (5.00, 3.4195),
        (10.0, 3.4150),
    ];
    let tolerance = 5e-5;
    let mut setup = common::Setup::new();
    setup.item(
        "wavelengths",
        &format!("{} µm", rows.map(|r| common::number(r.0)).join(", ")),
    );
    setup.item("grid", "none: the material's model, evaluated");
    setup.item(
        "tolerance",
        &format!("{tolerance:e} absolute (Li prints 4 decimals)"),
    );
    setup.material(&si, Wavelength::um(rows[1].0).expect("a valid wavelength"));
    setup.print();
    println!("{} (Li 1980, Table 1, 293 K), no grid", si.name());
    let mut checks = common::Checks::default();
    for (um, printed) in rows {
        let n = si
            .refractive_index(Wavelength::um(um).expect("a valid wavelength"))
            .expect("inside Li's range")
            .re;
        checks.compare(&format!("n at {um} µm"), n, printed, tolerance);
    }
    checks.finish()
}
