//! Four waveguides with dielectric corners, against their exact modal indices, by the standard
//! scheme and by Hadley's high-accuracy equations.
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
//! edge normal to y a magnetic one. Each problem is solved twice on each grid:
//!
//! - by the standard full-vector scheme (`mode::vector`, Fallahkhair et al. 2008), which
//!   converges at about first order at the convex corners (Figs. 4–5), with the error changing
//!   sign on the way, and at 1.3–1.8 at the concave ones (Figs. 6–7);
//! - by Hadley's equations (`mode::hadley`: his part I at uniform and interface nodes, part
//!   II's Eqs. 50 and 52 at the corner), about second order, as his Figs. 8–11 show for his
//!   "Full Model". The errors follow his curves: read off the log plots, about 5e-6 at 16 grid
//!   points per axis in Figs. 9 and 11, and about 1e-7 at 64 in Fig. 11 (5.1e-6, 4.9e-6 and
//!   1.2e-7 here).
//!
//! ```sh
//! cargo run --release --example hadley_corners
//! ```

mod common;

use std::process::ExitCode;

use faer::c64;
use photonoxide::mode::hadley;
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

pub fn main() -> ExitCode {
    let wavelength = Wavelength::um(1.5).expect("a valid wavelength");
    let mut checks = common::Checks::default();
    let figures = [
        (4, 2.25, true, 1.276_274_04),
        (5, 8.0, true, 2.656_796_92),
        (6, 2.25, false, 1.387_926_425),
        (7, 8.0, false, 2.761_465_320),
    ];
    let grids = [8, 16, 32, 64, 128];
    // every figure's grids solved side by side, both ways, then printed in order
    let problems: Vec<(f64, bool, usize)> = figures
        .iter()
        .flat_map(|&(_, eps, boxed, _)| grids.map(|n| (eps, boxed, n)))
        .collect();
    let solved = photonoxide::parallel::map_in_order(&problems, |&(eps, boxed, n)| {
        let cs = problem(eps, boxed, n);
        let standard = vector::modes(&cs, wavelength, 1, None).expect("the solver converges")[0]
            .effective_index()
            .re;
        let high = hadley::modes(&cs, wavelength, 1, None).expect("the solver converges")[0]
            .effective_index()
            .re;
        (standard, high)
    });
    for (f, &(fig, eps, boxed, exact)) in figures.iter().enumerate() {
        let kind = if boxed { "box" } else { "impinged corner" };
        println!("Fig. {fig}: {kind}, eps {eps} and 1, 1 x 1 um quarter domain; relative errors");
        println!("  grid                    standard scheme            Hadley's equations");
        let (mut standard, mut high) = (f64::NAN, f64::NAN);
        for (g, &n) in grids.iter().enumerate() {
            (standard, high) = solved[f * grids.len() + g];
            println!(
                "  {n:>3} x {n:<3} ({:>6.2} nm):  {standard:.9} {:+.2e}   {high:.9} {:+.2e}",
                1000.0 / n as f64,
                (standard - exact) / exact,
                (high - exact) / exact,
            );
        }
        checks.compare(
            &format!("Fig. {fig}, standard, 7.8 nm grid"),
            standard,
            exact,
            5e-5,
        );
        checks.compare(
            &format!("Fig. {fig}, Hadley's, 7.8 nm grid"),
            high,
            exact,
            1e-6,
        );
    }
    checks.finish()
}
