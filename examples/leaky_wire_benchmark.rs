//! The community benchmark for a leaky silicon photonic wire, by the full-vector solver with a
//! PML, extrapolated from three grids.
//!
//! P. Bienstman et al., "Modelling leaky photonic wires: a mode solver comparison", Opt.
//! Quantum Electron. 38, 731 (2006), [doi:10.1007/s11082-006-9025-9](https://doi.org/10.1007/s11082-006-9025-9):
//! 500 × 220 nm of silicon (3.5) on 1 µm of oxide (1.45) on a silicon substrate, air above, at
//! 1.55 µm. The fundamental TE mode leaks through the oxide into the substrate. Ten methods
//! were compared (Table 6); the two that agree best, CAMFR and the aperiodic Fourier modal
//! method, give Re n_eff = 2.412372 (7 digits) and Im n_eff = 2.9135e-8 and 2.91348e-8.
//!
//! The wire's corners slow finite differences to order ~0.67 here (silicon against air), so
//! the raw results converge slowly: +4.0e-3, +2.5e-3, +1.5e-3 at 5, 2.5 and 1.25 nm. Their
//! differences shrink by a steady factor, so Richardson extrapolation with the fitted order
//! gives the limit. The grid is 5, 2.5 or 1.25 nm in a box around the core and grows to 10 nm
//! away from it; the right half is solved, behind an electric wall.
//!
//! ```sh
//! cargo run --release --example leaky_wire_benchmark
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::Complex64 as c64;
use photonoxide::mode::vector::{
    self, Boundaries, Boundary, CrossSection, Permittivity, Pml, graded_nodes, richardson,
};
use photonoxide::units::Wavelength;

/// The wire's right half with the core's box at spacing `h` (µm): x from the wall at the
/// centre, y up from the oxide's top.
fn wire(h: f64) -> photonoxide::Result<CrossSection> {
    let (si, ox) = (3.5, 1.45);
    let x = graded_nodes(0.0, 1.6, (0.0, 0.35), h, 0.0125, 0.01, &[0.25]);
    let y = graded_nodes(
        -2.6,
        1.5,
        (-0.1, 0.32),
        h,
        0.0125,
        0.01,
        &[0.0, 0.22, -1.0, -1.6],
    );
    let mut cells = Vec::new();
    for i in 0..x.len() - 1 {
        let xc = 0.5 * (x[i] + x[i + 1]);
        for j in 0..y.len() - 1 {
            let yc = 0.5 * (y[j] + y[j + 1]);
            let n: f64 = match yc {
                yc if yc > 0.22 => 1.0,
                yc if yc > 0.0 => {
                    if xc < 0.25 {
                        si
                    } else {
                        1.0
                    }
                }
                yc if yc > -1.0 => ox,
                _ => si,
            };
            cells.push(Permittivity::isotropic(c64::new(n * n, 0.0)));
        }
    }
    CrossSection::new(x, y, cells)?
        .with_boundaries(Boundaries {
            west: Boundary::ElectricWall,
            ..Boundaries::default()
        })?
        .with_pml(Pml {
            south: 1.0,
            strength: 3.0,
            ..Pml::default()
        })
}

fn main() -> photonoxide::Result<ExitCode> {
    let w = Wavelength::um(1.55)?;
    let reference = c64::new(2.412372, 2.9135e-8);
    println!(
        "Bienstman et al.'s leaky wire, TE; reference (CAMFR) {:.6} + {:.4e}i",
        reference.re, reference.im
    );
    let mut found = [c64::new(0.0, 0.0); 3];
    for (k, h) in [0.005, 0.0025, 0.00125].into_iter().enumerate() {
        let cs = wire(h)?;
        let n = vector::modes(&cs, w, 1, Some(2.41))?[0].effective_index();
        println!(
            "  core grid {:.2} nm ({} x {} nodes): {:.6} + {:.4e}i, Re error {:+.2e}, Im / reference {:.4}",
            h * 1000.0,
            cs.x().len(),
            cs.y().len(),
            n.re,
            n.im,
            n.re - reference.re,
            n.im / reference.im
        );
        found[k] = n;
    }
    let (limit, order) = richardson(found);
    println!(
        "  extrapolated (order {order:.2}): {:.6} + {:.4e}i",
        limit.re, limit.im
    );
    let mut checks = common::Checks::default();
    // 7 digits printed; ours after extrapolation within 2e-4
    checks.compare("Re n_eff, extrapolated", limit.re, reference.re, 2e-4);
    // the methods' spread is 0.02e-8 among the best three; ours within 2 %
    checks.compare("Im n_eff x 1e8, extrapolated", limit.im * 1e8, 2.9135, 0.06);
    Ok(checks.finish())
}
