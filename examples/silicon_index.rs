//! Silicon's refractive index, against the table it comes from.
//!
//! H. H. Li, "Refractive index of silicon and germanium and its wavelength and temperature
//! derivatives", J. Phys. Chem. Ref. Data 9, 561 (1980),
//! [doi:10.1063/1.555624](https://doi.org/10.1063/1.555624), Table 1 (293 K), printed to 4
//! decimals; Li gives its uncertainty as ±2 × 10⁻⁴.
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
    println!("{} (Li 1980, Table 1, 293 K), no grid", si.name());
    let mut checks = common::Checks::default();
    // (wavelength in µm, Li's printed index)
    for (um, printed) in [
        (1.30, 3.5016),
        (1.55, 3.4757),
        (2.00, 3.4510),
        (5.00, 3.4195),
        (10.0, 3.4150),
    ] {
        let n = si
            .refractive_index(Wavelength::um(um).expect("a valid wavelength"))
            .expect("inside Li's range")
            .re;
        checks.compare(&format!("n at {um} µm"), n, printed, 5e-5);
    }
    checks.finish()
}
