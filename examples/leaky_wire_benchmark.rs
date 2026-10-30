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

/// The benchmark's indices: silicon, oxide and air.
const SI: f64 = 3.5;
const OX: f64 = 1.45;
const AIR: f64 = 1.0;
/// The wire's half width and thickness, and the oxide's thickness, µm.
const HALF_WIDTH: f64 = 0.25;
const THICKNESS: f64 = 0.22;
const BOX: f64 = 1.0;
/// The window, µm: x from the wall at the wire's centre, y up from the oxide's top.
const X_WINDOW: (f64, f64) = (0.0, 1.6);
const Y_WINDOW: (f64, f64) = (-2.6, 1.5);
/// The grid: the core grid's box (x, then y, µm), the coarsest spacing away from it (µm).
const FINE_X: (f64, f64) = (0.0, 0.35);
const FINE_Y: (f64, f64) = (-0.1, 0.32);
const COARSE: f64 = 0.0125;
/// The PML below, in the substrate (µm), and its strength.
const PML_SOUTH: f64 = 1.0;
const PML_STRENGTH: f64 = 3.0;

/// The wire's right half with the core's box at spacing `h` (µm): x from the wall at the
/// centre, y up from the oxide's top.
fn wire(h: f64) -> photonoxide::Result<CrossSection> {
    let (si, ox) = (SI, OX);
    let x = graded_nodes(
        X_WINDOW.0,
        X_WINDOW.1,
        FINE_X,
        h,
        COARSE,
        0.01,
        &[HALF_WIDTH],
    );
    let y = graded_nodes(
        Y_WINDOW.0,
        Y_WINDOW.1,
        FINE_Y,
        h,
        COARSE,
        0.01,
        &[0.0, THICKNESS, -BOX, -1.6],
    );
    let mut cells = Vec::new();
    for i in 0..x.len() - 1 {
        let xc = 0.5 * (x[i] + x[i + 1]);
        for j in 0..y.len() - 1 {
            let yc = 0.5 * (y[j] + y[j + 1]);
            let n: f64 = match yc {
                yc if yc > THICKNESS => AIR,
                yc if yc > 0.0 => {
                    if xc < HALF_WIDTH {
                        si
                    } else {
                        AIR
                    }
                }
                yc if yc > -BOX => ox,
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
            south: PML_SOUTH,
            strength: PML_STRENGTH,
            ..Pml::default()
        })
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let w = Wavelength::um(1.55)?;
    let reference = c64::new(2.412372, 2.9135e-8);
    let grids = [0.005, 0.0025, 0.00125];
    // 7 digits printed; ours after extrapolation within 2e-4. The methods' spread is 0.02e-8
    // among the best three; ours within 2 %
    let (re_tolerance, im_tolerance) = (2e-4, 0.06);
    let mut setup = common::Setup::new();
    setup.layer("cladding", None, "air", &common::n(AIR));
    setup.layer(
        "wire",
        Some(THICKNESS),
        "silicon in the wire, air beside it",
        &format!("{}; {}", common::n(SI), common::n(AIR)),
    );
    setup.layer("oxide", Some(BOX), "oxide", &common::n(OX));
    setup.layer("substrate", None, "silicon", &common::n(SI));
    setup.shape(
        "wire",
        &format!(
            "the wire centred at x = 0, {} wide",
            common::length(2.0 * HALF_WIDTH)
        ),
    );
    setup.wavelength(w);
    setup.item(
        "window",
        &format!(
            "the right half, x {} to {} µm from an electric wall at the centre; y {} to {} µm \
             from the oxide's top; a PML of {} in the substrate's bottom, strength {}",
            common::number(X_WINDOW.0),
            common::number(X_WINDOW.1),
            common::number(Y_WINDOW.0),
            common::number(Y_WINDOW.1),
            common::length(PML_SOUTH),
            common::number(PML_STRENGTH)
        ),
    );
    setup.item(
        "grid",
        &format!(
            "graded: {} nm in the core's box (x {} to {} µm, y {} to {} µm), at most {} away \
             from it; Richardson-extrapolated",
            grids.map(|h| common::number(h * 1000.0)).join(", "),
            common::number(FINE_X.0),
            common::number(FINE_X.1),
            common::number(FINE_Y.0),
            common::number(FINE_Y.1),
            common::length(COARSE)
        ),
    );
    setup.item(
        "solver",
        "the full-vector finite-difference mode solver, TE",
    );
    setup.item(
        "tolerance",
        &format!("{re_tolerance:e} in Re n_eff, {im_tolerance} in Im n_eff × 1e8 (2 %)"),
    );
    setup.medium(
        "silicon, oxide, air",
        "constant indices, the benchmark's (Bienstman et al. 2006)",
    );
    setup.print();
    println!(
        "Bienstman et al.'s leaky wire, TE; reference (CAMFR) {:.6} + {:.4e}i",
        reference.re, reference.im
    );
    let mut found = [c64::new(0.0, 0.0); 3];
    for (k, h) in grids.into_iter().enumerate() {
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
    checks.compare(
        "Re n_eff, extrapolated",
        limit.re,
        reference.re,
        re_tolerance,
    );
    checks.compare(
        "Im n_eff x 1e8, extrapolated",
        limit.im * 1e8,
        2.9135,
        im_tolerance,
    );
    Ok(checks.finish())
}
