//! Leaky waves, found by the full-vector mode solver with a perfectly matched layer.
//!
//! J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984),
//! [doi:10.1364/JOSAA.1.000742](https://doi.org/10.1364/JOSAA.1.000742), Table 2: the five TE
//! leaky waves of their four-layer guide (cover 1.0; films 1.66, 1.53, 1.60, 1.66, 500 nm each;
//! substrate 1.50; 632.8 nm), complex effective indices to 5 decimals. A leaky wave's field
//! grows into the substrate; the PML (W. C. Chew et al., Microw. Opt. Technol. Lett. 15, 363
//! (1997)) absorbs it, and the loss, Im n_eff, comes out of the eigenvalue.
//!
//! The guide lies flat, uniform in x between two electric walls (TE), on a 2.5 nm grid: 1 µm of
//! cover under a zero wall, the films, 1 µm of substrate and a 2 µm PML of strength 5 below it.
//! m = 8 sits just above the cover's index, so its field decays slowly into the cover, with its
//! phase running towards the guide, which a PML would not absorb: the cover is closed by a zero
//! wall, and m = 8 is within 2e-4 rather than 5e-5.
//!
//! ```sh
//! cargo run --release --example leaky_waves
//! ```

mod common;

use std::process::ExitCode;

use num_complex::Complex64 as c64;
use photonoxide::mode::vector::{self, Boundaries, Boundary, CrossSection, Permittivity, Pml};
use photonoxide::units::Wavelength;

pub fn main() -> photonoxide::Result<ExitCode> {
    let h = 0.0025;
    // y = 0 is the cover's interface; the films and the substrate lie below it
    let index = |y: f64| match y {
        y if y > 0.0 => 1.0,
        y if y > -0.5 => 1.66,
        y if y > -1.0 => 1.53,
        y if y > -1.5 => 1.60,
        y if y > -2.0 => 1.66,
        _ => 1.5,
    };
    let (bottom, top) = (-5.0, 1.0);
    let guide = CrossSection::uniform(
        (0.0, 4.0 * h, 4),
        (bottom, top, ((top - bottom) / h).round() as usize),
        |_, y| Permittivity::isotropic(c64::new(index(y) * index(y), 0.0)),
    )?
    .with_boundaries(Boundaries {
        west: Boundary::ElectricWall,
        east: Boundary::ElectricWall,
        ..Boundaries::default()
    })?
    .with_pml(Pml {
        south: 2.0,
        strength: 5.0,
        ..Pml::default()
    })?;
    let wavelength = Wavelength::nm(632.8)?;
    println!("Chilwell & Hodgkinson's guide, flat, 2.5 nm grid, PML below the substrate");
    let mut checks = common::Checks::default();
    for (order, re, im, tolerance) in [
        (4, 1.46186, 0.00716, 5e-5),
        (5, 1.38250, 0.01817, 5e-5),
        (6, 1.28136, 0.03588, 5e-5),
        (7, 1.14231, 0.05288, 5e-5),
        (8, 1.00304, 0.07077, 2e-4),
    ] {
        let printed = c64::new(re, im);
        // of the three modes nearest the printed real part, the one nearest the printed index
        let found = vector::modes(&guide, wavelength, 3, Some(re))?
            .iter()
            .map(|m| m.effective_index())
            .min_by(|a, b| (a - printed).norm().total_cmp(&(b - printed).norm()))
            .unwrap_or(c64::new(f64::NAN, f64::NAN));
        checks.compare(&format!("TE{order} Re n_eff"), found.re, re, tolerance);
        checks.compare(&format!("TE{order} Im n_eff"), found.im, im, tolerance);
    }
    Ok(checks.finish())
}
