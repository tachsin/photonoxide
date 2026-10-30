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

/// The window's quarter reaches `MARGIN` µm beyond the core; the grid is `CELLS` cells to the
/// core's half height, fine to `FINE` µm beyond the core and at most `COARSE` µm beyond.
const MARGIN: f64 = 4.0;
const CELLS: f64 = 40.0;
const FINE: f64 = 0.2;
const COARSE: f64 = 0.05;

pub fn main() -> photonoxide::Result<ExitCode> {
    let w = Wavelength::um(1.0)?;
    let (core, clad) = (1.5f64, 1.5f64 / 1.05);
    let bigs = [1.0f64, 1.5, 2.0, 3.0, 4.0];
    // the core's height b (µm) at Marcatili's normalized B = (2b/λ) √(n1² − n4²), λ = 1 µm
    let height = |big_b: f64| big_b / (2.0 * (core * core - clad * clad).sqrt());
    // the closed form within 5 % of the exact solution, where B is at least 0.5
    let (tolerance, guided) = (0.05, 0.5);
    let mut setup = common::Setup::new();
    setup.item(
        "media",
        &format!(
            "the core, {}, in the cladding all around it, {}",
            common::n(core),
            common::n(clad)
        ),
    );
    setup.shape(
        "the cladding",
        &format!(
            "the core, a rectangle a = 2b wide, b high, centred; b = {} µm at B = {}",
            bigs.map(|bb| format!("{:.4}", height(bb))).join(", "),
            bigs.map(common::number).join(", ")
        ),
    );
    setup.wavelength(w);
    setup.item(
        "window",
        &format!(
            "the quarter x, y ≥ 0 to {} beyond the core, mirror walls through its centre (by \
             family)",
            common::length(MARGIN)
        ),
    );
    setup.item(
        "grid",
        &format!(
            "graded, b/{} in the core and to {} beyond it, at most {} farther",
            common::number(CELLS),
            common::length(FINE),
            common::length(COARSE)
        ),
    );
    setup.item(
        "solver",
        "the full-vector mode solver; Marcatili's exact and closed-form solutions",
    );
    setup.item(
        "tolerance",
        &format!("the closed form within {tolerance} of the exact, relative, where B ≥ {guided}"),
    );
    setup.medium(
        "core, cladding",
        "constant indices, Marcatili's n1 = 1.5 and n1/n4 = 1.05",
    );
    setup.print();
    let mut checks = common::Checks::default();
    println!("Marcatili's guide a = 2b, n1 = 1.5, n1/n4 = 1.05, at 1 um; normalized constants");
    println!("    B   mode   full-vector  Marcatili  closed form");
    for big_b in bigs {
        let b = height(big_b);
        let a = 2.0 * b;
        let r = Rectangle::uniform(core, clad, Length::um(a), Length::um(b))?;
        for (family, west, south) in [
            (Family::Ex, Boundary::ElectricWall, Boundary::MagneticWall),
            (Family::Ey, Boundary::MagneticWall, Boundary::ElectricWall),
        ] {
            let h = b / CELLS;
            let x = graded_nodes(
                0.0,
                a / 2.0 + MARGIN,
                (0.0, a / 2.0 + FINE),
                h,
                COARSE,
                0.1,
                &[a / 2.0],
            );
            let y = graded_nodes(
                0.0,
                b / 2.0 + MARGIN,
                (0.0, b / 2.0 + FINE),
                h,
                COARSE,
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
            )?;
            // no mode there (below cutoff) prints as NaN
            let b = |n: Option<f64>| -> photonoxide::Result<f64> {
                n.map_or(Ok(f64::NAN), |n| normalized(n, core, clad))
            };
            let exact = b(r.mode(family, 1, 1, w))?;
            let closed = b(r.closed_form(family, 1, 1, w))?;
            println!("  {big_b:>3}   {family:?}_11    {full:.4}      {exact:.4}     {closed:.4}");
            if exact >= guided {
                checks.compare(
                    &format!("B = {big_b}: closed form {family:?}_11, relative"),
                    (closed - exact) / exact,
                    0.0,
                    tolerance,
                );
            }
        }
    }
    Ok(checks.finish())
}
