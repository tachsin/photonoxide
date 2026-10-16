//! The TE-like mode of a 500 × 220 nm silicon strip in oxide at 1550 nm, with the full-vector
//! mode solver, at three grids.
//!
//! L. Chrostowski, M. Hochberg, *Silicon Photonics Design*, Cambridge University Press (2015),
//! [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168), Fig. 3.14:
//! silicon 3.473, oxide 1.444, effective index 2.443 (Lumerical MODE on a 20 nm conformal mesh,
//! accurate to about 1e-3 by the book's Fig. 3.9).
//!
//! The strip is photonoxide's waveguide device ([`StripWaveguide`]) on the book's two media,
//! and the cross-section the solver sees is the device's, cut from its stack and its strip in
//! a 2.1 × 1.5 µm window. The strip's convex corners slow photonoxide's convergence to about
//! first order (2.4435 at a 2.5 nm grid; `hadley_corners` measures it on exact problems), so the
//! check is at 5 nm, within 3e-3; ROADMAP.md has the fix (Hadley 2002), which the device's own
//! default mesh uses.
//!
//! ```sh
//! cargo run --release --example strip_waveguide
//! ```

mod common;

use std::process::ExitCode;

use faer::c64;
use photonoxide::device::{Mesh, Scheme, StripWaveguide, Symmetry, Values};
use photonoxide::material::{Material, Model, Provenance};
use photonoxide::mode::vector;
use photonoxide::stack::{Layer, LayerStack};
use photonoxide::units::{Length, Wavelength};

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

/// The waveguide device on the book's stack, solved in a 2.1 × 1.5 µm window centred on the
/// strip, whole, on a uniform grid of spacing `h` (µm): the core's edges (±0.25, ±0.11 µm) fall
/// on nodes for the grids used here.
fn strip(h: f64) -> photonoxide::Result<StripWaveguide> {
    let nx = (2.1 / h).round() as usize;
    let ny = (1.5 / h).round() as usize;
    Ok(StripWaveguide::on(stack()?, "Si")?
        .with_mesh(Mesh::window(2.1, 1.5, nx, ny, Scheme::Standard).with_symmetry(Symmetry::Full)))
}

pub fn main() -> ExitCode {
    let wavelength = Wavelength::um(1.55).expect("a valid wavelength");
    println!("500 x 220 nm strip of 3.473 in 1.444 at 1550 nm (Chrostowski & Hochberg, Fig. 3.14)");
    let mut checks = common::Checks::default();
    // the three grids solved side by side, then printed in order
    let grids = [20.0, 10.0, 5.0];
    let solved = photonoxide::parallel::map_in_order(&grids, |&h_nm| {
        let device = strip(h_nm / 1000.0).expect("the book's stack");
        let cs = device
            .cross_section(&Values::defaults(&device), wavelength)
            .expect("a valid grid");
        let modes = vector::modes(&cs, wavelength, 1, None).expect("the solver converges");
        let mode = &modes[0];
        let nodes = (cs.x().len(), cs.y().len());
        (nodes, mode.effective_index().re, mode.te_fraction())
    });
    for (&h_nm, &((nx, ny), n_eff, te)) in grids.iter().zip(&solved) {
        println!(
            "  grid {h_nm} nm ({nx} x {ny} nodes, 2.1 x 1.5 um window): n_eff {n_eff:.6}, TE fraction {te:.3}",
        );
        if h_nm == 5.0 {
            // a TM-like mode (index near 1.8) would fail this too
            checks.compare("TE-like n_eff at a 5 nm grid", n_eff, 2.443, 3e-3);
        }
    }
    checks.finish()
}
