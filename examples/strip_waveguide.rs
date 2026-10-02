//! The TE-like mode of a 500 × 220 nm silicon strip in oxide at 1550 nm, with the full-vector
//! mode solver, at three grids.
//!
//! L. Chrostowski, M. Hochberg, *Silicon Photonics Design*, Cambridge University Press (2015),
//! [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168), Fig. 3.14:
//! silicon 3.473, oxide 1.444, effective index 2.443 (Lumerical MODE on a 20 nm conformal mesh,
//! accurate to about 1e-3 by the book's Fig. 3.9).
//!
//! The strip's convex corners slow photonoxide's convergence to about first order (2.4435 at a
//! 2.5 nm grid; `hadley_corners` measures it on exact problems), so the check is at 5 nm, within
//! 3e-3; ROADMAP.md has the fix (Hadley 2002).
//!
//! ```sh
//! cargo run --release --example strip_waveguide
//! ```

mod common;

use std::process::ExitCode;

use faer::c64;
use photonoxide::mode::vector::{self, CrossSection, Permittivity};
use photonoxide::units::Wavelength;

/// The strip, centred, in a 2.1 × 1.5 µm window on a uniform grid of spacing `h` (µm): the
/// core's edges (±0.25, ±0.11 µm) fall on nodes for the grids used here.
fn strip(h: f64) -> CrossSection {
    let nx = (2.1 / h).round() as usize;
    let ny = (1.5 / h).round() as usize;
    CrossSection::uniform((-1.05, 1.05, nx), (-0.75, 0.75, ny), |x, y| {
        let n: f64 = if x.abs() < 0.25 && y.abs() < 0.11 {
            3.473
        } else {
            1.444
        };
        Permittivity::isotropic(c64::new(n * n, 0.0))
    })
    .expect("a valid grid")
}

pub fn main() -> ExitCode {
    let wavelength = Wavelength::um(1.55).expect("a valid wavelength");
    println!("500 x 220 nm strip of 3.473 in 1.444 at 1550 nm (Chrostowski & Hochberg, Fig. 3.14)");
    let mut checks = common::Checks::default();
    for h_nm in [20.0, 10.0, 5.0] {
        let cs = strip(h_nm / 1000.0);
        let modes = vector::modes(&cs, wavelength, 1, None).expect("the solver converges");
        let mode = &modes[0];
        println!(
            "  grid {h_nm} nm ({} x {} nodes, 2.1 x 1.5 um window): n_eff {:.6}, TE fraction {:.3}",
            cs.x().len(),
            cs.y().len(),
            mode.effective_index().re,
            mode.te_fraction()
        );
        if h_nm == 5.0 {
            // a TM-like mode (index near 1.8) would fail this too
            checks.compare(
                "TE-like n_eff at a 5 nm grid",
                mode.effective_index().re,
                2.443,
                3e-3,
            );
        }
    }
    checks.finish()
}
