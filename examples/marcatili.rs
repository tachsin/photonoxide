//! Marcatili's approximation for a rectangular guide, against the full-vector solver.
//!
//! E. A. J. Marcatili, "Dielectric rectangular waveguide and directional coupler for integrated
//! optics", Bell Syst. Tech. J. 48, 2071 (1969),
//! [doi:10.1002/j.1538-7305.1969.tb01166.x](https://doi.org/10.1002/j.1538-7305.1969.tb01166.x):
//! his Fig. 6b guide, a = 2b, n₁ = 1.5 in n₁/1.05, the fundamental E^x and E^y modes against the
//! normalized height B = 2b/λ (n₁² − n₄²)^½. The ordinate is his normalized constant
//! (k_z² − k₄²)/(k₁² − k₄²). He compares his solutions with Goell's computer solutions there and
//! states they "coincide even for moderately large values of b", and that his closed form is
//! "within a few percent" of his exact solution where the constant is at least 0.5 (p. 2083).
//!
//! Here the transcendental equations are solved exactly, and the full-vector solver plays
//! Goell's part (a quarter of the guide behind the mode's walls, b/40 in the core). Far from
//! cutoff they agree to 1e-4; near it the field reaches into the corners Marcatili ignores.
//! The checks below are his closed form's stated accuracy; the comparison with the full-vector
//! solver is printed, and checked in the validation report.
//!
//! ```sh
//! cargo run --release --example marcatili
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::Complex64 as c64;
use photonoxide::mode::marcatili::{Family, Rectangle, normalized};
use photonoxide::mode::vector::{
    self, Boundaries, Boundary, CrossSection, Permittivity, graded_nodes,
};
use photonoxide::units::{Length, Wavelength};

pub fn main() -> photonoxide::Result<ExitCode> {
    let w = Wavelength::um(1.0)?;
    let (core, clad) = (1.5f64, 1.5f64 / 1.05);
    let mut checks = common::Checks::default();
    println!("Marcatili's guide a = 2b, n1 = 1.5, n1/n4 = 1.05, at 1 um; normalized constants");
    println!("    B   mode   full-vector  Marcatili  closed form");
    for big_b in [1.0f64, 1.5, 2.0, 3.0, 4.0] {
        let b = big_b / (2.0 * (core * core - clad * clad).sqrt());
        let a = 2.0 * b;
        let r = Rectangle::uniform(core, clad, Length::um(a), Length::um(b))?;
        for (family, west, south) in [
            (Family::Ex, Boundary::ElectricWall, Boundary::MagneticWall),
            (Family::Ey, Boundary::MagneticWall, Boundary::ElectricWall),
        ] {
            let h = b / 40.0;
            let x = graded_nodes(
                0.0,
                a / 2.0 + 4.0,
                (0.0, a / 2.0 + 0.2),
                h,
                0.05,
                0.1,
                &[a / 2.0],
            );
            let y = graded_nodes(
                0.0,
                b / 2.0 + 4.0,
                (0.0, b / 2.0 + 0.2),
                h,
                0.05,
                0.1,
                &[b / 2.0],
            );
            let mut cells = Vec::new();
            for i in 0..x.len() - 1 {
                for j in 0..y.len() - 1 {
                    let inside =
                        0.5 * (x[i] + x[i + 1]) < a / 2.0 && 0.5 * (y[j] + y[j + 1]) < b / 2.0;
                    let n = if inside { core } else { clad };
                    cells.push(Permittivity::isotropic(c64::new(n * n, 0.0)));
                }
            }
            let cs = CrossSection::new(x, y, cells)?.with_boundaries(Boundaries {
                west,
                south,
                ..Boundaries::default()
            })?;
            let full = normalized(
                vector::modes(&cs, w, 1, None)?[0].effective_index().re,
                core,
                clad,
            );
            let exact = r
                .mode(family, 1, 1, w)
                .map_or(f64::NAN, |n| normalized(n, core, clad));
            let closed = r
                .closed_form(family, 1, 1, w)
                .map_or(f64::NAN, |n| normalized(n, core, clad));
            println!("  {big_b:>3}   {family:?}_11    {full:.4}      {exact:.4}     {closed:.4}");
            if exact >= 0.5 {
                checks.compare(
                    &format!("B = {big_b}: closed form {family:?}_11, relative"),
                    (closed - exact) / exact,
                    0.0,
                    0.05,
                );
            }
        }
    }
    Ok(checks.finish())
}
