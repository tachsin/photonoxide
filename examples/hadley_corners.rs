//! Four waveguides with dielectric corners, against their exact modal indices.
//!
//! G. R. Hadley, "High-accuracy finite-difference equations for dielectric waveguide analysis
//! II: dielectric corners", J. Lightwave Technol. 20, 1219 (2002),
//! [doi:10.1109/JLT.2002.800371](https://doi.org/10.1109/JLT.2002.800371), Figs. 4–7: a quarter
//! of each waveguide, 1 × 1 µm, at λ = 1.5 µm, with indices from series expansions (up to 4000
//! terms) good to 1e-8 or better.
//!
//! - Figs. 4–5, boxes: ε = 2.25 or 8 for x, y < 0.5 µm, 1 elsewhere; on all four edges H_x is
//!   zero and H_y has zero normal derivative.
//! - Figs. 6–7, impinged corners: ε = 1 for x, y > 0.5 µm, 2.25 or 8 elsewhere; H_y has zero
//!   derivative on the west and south edges and is zero on the others, H_x the opposite.
//!
//! Those edges are mirror walls: H_x zero on an edge normal to x is an electric wall, on an
//! edge normal to y a magnetic one. photonoxide's corners converge at about first order (convex)
//! to 1.4–1.8 (concave), as standard finite differences do; Hadley's corner equations are on
//! the roadmap.
//!
//! ```sh
//! cargo run --release --example hadley_corners
//! ```

mod common;

use std::process::ExitCode;

use faer::c64;
use photonoxide::mode::vector::{self, Boundaries, Boundary, CrossSection, Permittivity};
use photonoxide::units::Wavelength;

/// Hadley's problem on an `n` × `n` grid: the core's ε, and whether it is a box.
fn problem(eps: f64, boxed: bool, n: usize) -> CrossSection {
    use Boundary::{ElectricWall, MagneticWall};
    let boundaries = if boxed {
        Boundaries {
            west: ElectricWall,
            east: ElectricWall,
            south: MagneticWall,
            north: MagneticWall,
        }
    } else {
        Boundaries {
            west: ElectricWall,
            east: MagneticWall,
            south: MagneticWall,
            north: ElectricWall,
        }
    };
    CrossSection::uniform((0.0, 1.0, n), (0.0, 1.0, n), |x, y| {
        let high = if boxed {
            x < 0.5 && y < 0.5
        } else {
            !(x > 0.5 && y > 0.5)
        };
        Permittivity::isotropic(c64::new(if high { eps } else { 1.0 }, 0.0))
    })
    .and_then(|cs| cs.with_boundaries(boundaries))
    .expect("a valid problem")
}

fn main() -> ExitCode {
    let wavelength = Wavelength::um(1.5).expect("a valid wavelength");
    let mut checks = common::Checks::default();
    for (fig, eps, boxed, exact) in [
        (4, 2.25, true, 1.276_274_04),
        (5, 8.0, true, 2.656_796_92),
        (6, 2.25, false, 1.387_926_425),
        (7, 8.0, false, 2.761_465_320),
    ] {
        let kind = if boxed { "box" } else { "impinged corner" };
        println!("Fig. {fig}: {kind}, eps {eps} and 1, 1 x 1 um quarter domain");
        let mut n_eff = f64::NAN;
        for n in [40, 80, 160] {
            let modes = vector::modes(&problem(eps, boxed, n), wavelength, 1, None)
                .expect("the solver converges");
            n_eff = modes[0].effective_index().re;
            println!(
                "  grid {n} x {n} ({:.2} nm): n_eff {n_eff:.9}, error {:+.2e}",
                1000.0 / n as f64,
                n_eff - exact
            );
        }
        checks.compare(
            &format!("Fig. {fig} n_eff at a 6.25 nm grid"),
            n_eff,
            exact,
            5e-5,
        );
    }
    checks.finish()
}
