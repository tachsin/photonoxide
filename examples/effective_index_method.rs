//! The effective index method on silicon strips, and its error against the full-vector solver.
//!
//! L. Chrostowski, M. Hochberg, *Silicon Photonics Design*, Cambridge University Press (2015),
//! [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168), Section 3.2.5: for a
//! 500 × 220 nm strip of 3.473 in 1.444 at 1550 nm, the TE slab (2.845, to 3 decimals) and then
//! the lateral TM slab give 2.489 (Lumerical's 1D solver on a 10 nm mesh, fed the rounded
//! 2.845; exactly, from our unrounded slab index, the method gives 2.4884). The method itself is
//! G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977).
//!
//! The method assumes the field separates, which a strip's corners break. Its error against
//! photonoxide's full-vector modes (6.25 × 5 nm grid, quarter domain), for n_eff and for the
//! group index (from 1.54, 1.55, 1.56 µm, no material dispersion), is printed for each width;
//! the book states 1.2 % and 2.7 % for 500 nm, against a 2D solver whose 2.443 its own 2.489 is
//! 1.9 % above.
//!
//! ```sh
//! cargo run --release --example effective_index_method
//! ```

mod common;

use std::process::ExitCode;

use num_complex::Complex64 as c64;
use photonoxide::mode::Polarization;
use photonoxide::mode::dispersion::{group_index, track};
use photonoxide::mode::eim::Ridge;
use photonoxide::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity};
use photonoxide::units::{Length, Wavelength};

/// The book's media: silicon and oxide, and the film's thickness.
const CORE: f64 = 3.473;
const CLAD: f64 = 1.444;
const THICKNESS_NM: f64 = 220.0;

/// The quarter of the window, x, y ≥ 0: its width and cells across, its height and cells up.
const QUARTER: (f64, usize, f64, usize) = (1.05, 168, 0.75, 150);

/// The quarter x, y ≥ 0 of a strip `width` µm wide and 220 nm thick, on a 6.25 × 5 nm grid,
/// behind the TE-like mode's two mirror walls.
fn strip(width: f64) -> photonoxide::Result<CrossSection> {
    let (across, nx, up, ny) = QUARTER;
    CrossSection::uniform((0.0, across, nx), (0.0, up, ny), |x, y| {
        let n: f64 = if x < width / 2.0 && y < THICKNESS_NM / 2000.0 {
            CORE
        } else {
            CLAD
        };
        Permittivity::isotropic(c64::new(n * n, 0.0))
    })?
    .with_boundaries(Boundaries {
        west: Boundary::ElectricWall,
        south: Boundary::MagneticWall,
        ..Boundaries::default()
    })
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let wavelengths = [1.54, 1.55, 1.56]
        .iter()
        .map(|&l| Wavelength::um(l))
        .collect::<photonoxide::Result<Vec<_>>>()?;
    let widths = [0.40, 0.45, 0.50, 0.55, 0.60];
    let (book, tolerance) = (2.489, 1e-3);
    let (across, nx, up, ny) = QUARTER;
    let mut setup = common::Setup::new();
    setup.layer("cladding", None, "oxide", &common::n(CLAD));
    setup.layer(
        "core",
        Some(THICKNESS_NM / 1000.0),
        "silicon in the strip, oxide beside it",
        &format!("{}; {}", common::n(CORE), common::n(CLAD)),
    );
    setup.layer("substrate", None, "oxide", &common::n(CLAD));
    setup.shape(
        "core",
        &format!(
            "strip centred at x = 0, {} µm wide",
            widths.map(common::number).join(", ")
        ),
    );
    setup.item(
        "wavelengths",
        &format!(
            "{} µm (n_g by central differences)",
            wavelengths
                .iter()
                .map(|l| common::number(l.to_um()))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    setup.item(
        "window",
        &format!(
            "the quarter x, y ≥ 0, {} by {}, behind the TE-like mode's mirror walls",
            common::length(across),
            common::length(up)
        ),
    );
    setup.item(
        "grid",
        &format!(
            "uniform, {nx} × {ny} cells: {} by {}",
            common::length(across / nx as f64),
            common::length(up / ny as f64)
        ),
    );
    setup.item(
        "solver",
        "the effective index method (Hocker & Burns), against the full-vector mode solver",
    );
    setup.item(
        "tolerance",
        &format!(
            "{tolerance:e} on the book's {book} for the {} strip",
            common::length(widths[2])
        ),
    );
    setup.medium(
        "silicon, oxide",
        "constant indices, the book's (Chrostowski & Hochberg 2015, Section 3.2.5)",
    );
    setup.print();
    let mut checks = common::Checks::default();
    println!("220 nm strips of 3.473 in 1.444, TE-like; 2D: 6.25 x 5 nm grid");
    println!("  width   EIM n_eff  2D n_eff   error    EIM n_g  2D n_g   error");
    for width in widths {
        let ridge = Ridge::strip(
            CLAD,
            CORE,
            CLAD,
            Length::nm(THICKNESS_NM),
            Length::um(width),
        )?;
        let eim: Vec<f64> = wavelengths
            .iter()
            .map(|&l| ridge.mode(Polarization::Te, l).map(|m| m.effective_index))
            .collect::<photonoxide::Result<_>>()?;
        let full: Vec<f64> = track(|_| strip(width), &wavelengths, None, 3)?
            .iter()
            .map(|m| m.effective_index().re)
            .collect();
        let (ng_eim, ng_full) = (
            group_index(&wavelengths, &eim)?[1],
            group_index(&wavelengths, &full)?[1],
        );
        println!(
            "  {:.0} nm  {:.4}     {:.4}    {:+.2} %  {:.3}    {:.3}   {:+.2} %",
            width * 1000.0,
            eim[1],
            full[1],
            100.0 * (eim[1] / full[1] - 1.0),
            ng_eim,
            ng_full,
            100.0 * (ng_eim / ng_full - 1.0)
        );
        if width == widths[2] {
            // printed to 3 decimals, from the rounded slab index and a 10 nm mesh (see above)
            checks.compare("EIM n_eff, 500 nm strip", eim[1], book, tolerance);
        }
    }
    Ok(checks.finish())
}
