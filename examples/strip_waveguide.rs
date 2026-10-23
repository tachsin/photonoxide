//! The TE-like mode of a 500 × 220 nm silicon strip in oxide at 1550 nm, with the full-vector
//! mode solver, on halving grids, and its grid study.
//!
//! L. Chrostowski, M. Hochberg, *Silicon Photonics Design*, Cambridge University Press (2015),
//! [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168), Fig. 3.14:
//! silicon 3.473, oxide 1.444, effective index 2.443 (Lumerical MODE on a 20 nm conformal mesh,
//! accurate to about 1e-3 by the book's Fig. 3.9).
//!
//! The strip is photonoxide's waveguide device ([`StripWaveguide`]) on the book's two media,
//! and the cross-section the solver sees is the device's, cut from its stack and its strip: the
//! quarter of a 2.1 × 1.5 µm window behind the strip's two mirror planes, with zero walls
//! outside, as the device's own mesh solves it. Each grid is uniform and nested in the next, with
//! the core's edges on nodes, and the window and its walls stay put.
//!
//! The quoted number is from Hadley's equations, the device's default scheme (G. R. Hadley,
//! J. Lightwave Technol. 20, 1219 (2002), [doi:10.1109/JLT.2002.800371](https://doi.org/10.1109/JLT.2002.800371),
//! the corners), and its grid study gates it (`Checks::converged`, at 1e-4, a tenth of the
//! book's accuracy). The standard scheme is printed beside it, not gated. Its convex corners hold
//! it well under second order: from 20 to 2.5 nm its changes are 1.35e-3, 1.32e-3 and 0.89e-3,
//! orders 0.04 and 0.57, so the same study doesn't pass it at 1e-4 (`hadley_corners` measures
//! the corners on exact problems).
//!
//! - **Default**, 10 and 5 nm (106 × 76 and 211 × 151 nodes on the quarter, about 5 s): the
//!   two-grid estimate, which assumes first order.
//! - **`--full`** adds 2.5 nm (421 × 301 nodes, about a minute): Richardson's estimate on three.
//!
//! Measured (`--full`, 2026-10-23): Hadley's 2.442212, 2.442178 and 2.442168 at 10, 5 and
//! 2.5 nm, order 1.84, error 3.7e-6, limit 2.442165. The standard scheme gives 2.445713, 2.444396
//! and 2.443506 there. The converged index is 8.4e-4 under the book's, within its 1e-3. The
//! window's walls move it by 1.3e-5 (a 3.1 × 2.5 µm window at 10 nm).
//!
//! ```sh
//! cargo run --release --example strip_waveguide             # 10 and 5 nm
//! cargo run --release --example strip_waveguide -- --full   # and 2.5 nm
//! ```

mod common;

use std::process::ExitCode;

use faer::c64;
use photonoxide::convergence::Study;
use photonoxide::device::{Mesh, Scheme, StripWaveguide, Symmetry, Values};
use photonoxide::material::{Material, Model, Provenance};
use photonoxide::mode::{hadley, vector};
use photonoxide::stack::{Layer, LayerStack};
use photonoxide::units::{Length, Wavelength};

/// The tolerance on the quoted index's grid error: a tenth of the book's 1e-3.
const GRID_TOLERANCE: f64 = 1e-4;

/// One of the book's media, of constant index `n`.
fn medium(name: &str, n: f64) -> photonoxide::Result<Material> {
    Material::new(
        name,
        Model::Constant(c64::new(n, 0.0)),
        Wavelength::um(1.0)?,
        Wavelength::um(2.0)?,
        Provenance {
            reference: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), Fig. 3.14"
                .into(),
            doi: "10.1017/CBO9781316084168".into(),
            data: String::new(),
            temperature: None,
            notes: String::new(),
        },
    )
}

/// The book's strip: 220 nm of silicon (3.473) on 2 µm of oxide (1.444) under oxide.
fn stack() -> photonoxide::Result<LayerStack> {
    let (si, ox) = (
        medium("Si (3.473)", 3.473)?,
        medium("oxide (1.444)", 1.444)?,
    );
    let layer = |name: &str, thickness, material| Layer {
        name: name.into(),
        thickness,
        material,
        background: ox.clone(),
    };
    LayerStack::new(
        ox.clone(),
        vec![
            layer("BOX", Length::um(2.0), ox.clone()),
            layer("Si", Length::nm(220.0), si),
        ],
        ox.clone(),
    )
}

/// The waveguide device on the book's stack, solved on the quarter of a 2.1 × 1.5 µm window
/// centred on the strip, on a uniform grid of spacing `h` (µm): the quarter's 1.05 × 0.75 µm and
/// the core's edges (0.25 and 0.11 µm from the mirror planes) fall on nodes for 10 nm and its
/// halvings.
fn strip(h: f64) -> photonoxide::Result<StripWaveguide> {
    let columns = (1.05 / h).round() as usize;
    let rows = (0.75 / h).round() as usize;
    Ok(StripWaveguide::on(stack()?, "Si")?.with_mesh(
        Mesh::window(2.1, 1.5, columns, rows, Scheme::Hadley).with_symmetry(Symmetry::Mirrored),
    ))
}

pub fn main() -> ExitCode {
    let wavelength = Wavelength::um(1.55).expect("a valid wavelength");
    let full = std::env::args().any(|a| a == "--full");
    println!("500 x 220 nm strip of 3.473 in 1.444 at 1550 nm (Chrostowski & Hochberg, Fig. 3.14)");
    println!("  the quarter of a 2.1 x 1.5 um window, zero walls; TE-like n_eff by scheme");
    println!("  grid     nodes       standard scheme   Hadley's equations   TE fraction");
    let mut checks = common::Checks::default();
    let grids: &[f64] = if full {
        &[10.0, 5.0, 2.5]
    } else {
        &[10.0, 5.0]
    };
    // every grid and scheme solved side by side, then printed in order
    let problems: Vec<(f64, bool)> = grids
        .iter()
        .flat_map(|&h| [(h, false), (h, true)])
        .collect();
    let solved = photonoxide::parallel::map_in_order(&problems, |&(h_nm, high)| {
        let device = strip(h_nm / 1000.0).expect("the book's stack");
        let cs = device
            .cross_section(&Values::defaults(&device), wavelength)
            .expect("a valid grid");
        let modes = if high {
            hadley::modes(&cs, wavelength, 1, None)
        } else {
            vector::modes(&cs, wavelength, 1, None)
        }
        .expect("the solver converges");
        let nodes = (cs.x().len(), cs.y().len());
        (nodes, modes[0].effective_index().re, modes[0].te_fraction())
    });
    let mut standard = Study::new("TE-like n_eff, standard scheme");
    let mut high = Study::new("TE-like n_eff, Hadley's equations");
    let mut quoted = (f64::NAN, f64::NAN);
    for (&h_nm, pair) in grids.iter().zip(solved.chunks(2)) {
        let (((nx, ny), n_standard, _), (_, n_high, te)) = (pair[0], pair[1]);
        println!(
            "  {:<8} {:<11} {n_standard:<17.6} {n_high:<20.6} {te:.3}",
            format!("{h_nm} nm"),
            format!("{nx} x {ny}"),
        );
        standard = standard
            .on(h_nm / 1000.0, n_standard)
            .expect("a finer grid");
        high = high.on(h_nm / 1000.0, n_high).expect("a finer grid");
        quoted = (h_nm, n_high);
    }
    let corners = standard.check(GRID_TOLERANCE).expect("a tolerance");
    println!(
        "  the standard scheme's grids: error {:.1e}, {} at {GRID_TOLERANCE:e} (its corners)",
        corners.error.unwrap_or(f64::NAN),
        corners.verdict.name(),
    );
    let (h_nm, n_eff) = quoted;
    // a TM-like mode (index near 1.8) would fail this too
    checks.compare(
        &format!("TE-like n_eff, Hadley's, {h_nm} nm"),
        n_eff,
        2.443,
        3e-3,
    );
    checks.converged(
        "TE-like n_eff, Hadley's, its grids",
        &high.check(GRID_TOLERANCE).expect("a tolerance"),
    );
    checks.finish()
}
