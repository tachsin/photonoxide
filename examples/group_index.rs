//! The group index of silicon strip waveguides of five widths, with material dispersion.
//!
//! L. Chrostowski, M. Hochberg, *Silicon Photonics Design*, Cambridge University Press (2015),
//! [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168), Fig. 3.22b: the
//! group index of 220 nm thick strips, 400 to 600 nm wide, against wavelength (Lumerical MODE,
//! 20 nm mesh, Listings 3.12 and 3.15). Read off the plot at 1.55 µm, to about ±0.005: 4.37,
//! 4.27, 4.18, 4.10 and 4.04.
//!
//! The materials are the book's (Listing 3.1): its Lorentz fit to silicon (Eq. 3.2), and oxide
//! of constant index 1.444. The group index is n_g = n_eff − λ dn_eff/dλ (Eq. 3.5), from the
//! TE-like mode tracked over 1.54, 1.55 and 1.56 µm. The mode is symmetric, so only a quarter of
//! the cross-section is solved, behind two mirror walls. The narrowest strip converges slowest:
//! its corners hold the most light.
//!
//! ```sh
//! cargo run --release --example group_index
//! ```

mod common;

use std::f64::consts::PI;
use std::process::ExitCode;

use faer::c64;
use photonoxide::material::{LorentzPole, Material, Model, Provenance};
use photonoxide::mode::dispersion::{group_index, track};
use photonoxide::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity};
use photonoxide::units::{Frequency, Wavelength};

/// The book's silicon: ε = ε∞ + Δε ω₀²/(ω₀² − ω²), fitted to Palik's data from 1.15 to 1.8 µm.
fn silicon() -> photonoxide::Result<Material> {
    let resonance = 3.932_824_66e15 / (2.0 * PI) / 1e12; // ω₀ in rad/s, as THz
    Material::new(
        "Si (Chrostowski & Hochberg's Lorentz fit)",
        Model::Lorentz {
            eps_inf: 7.987_374_92,
            poles: vec![LorentzPole {
                strength: 3.687_991_43,
                resonance: Frequency::thz(resonance)?,
                damping: 0.0,
            }],
        },
        Wavelength::um(1.15)?,
        Wavelength::um(1.8)?,
        Provenance {
            reference: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), Eq. 3.2"
                .into(),
            doi: "10.1017/CBO9781316084168".into(),
            data: "Listing 3.1, a fit to Palik's handbook".into(),
            temperature: Some(300.0),
            notes: String::new(),
        },
    )
}

/// The quarter x, y ≥ 0 of a strip `width` µm wide and 220 nm thick, centred at the origin, on a
/// 6.25 × 5 nm grid in a 1.05 × 0.75 µm window: an electric wall at x = 0 and a magnetic wall at
/// y = 0 keep the TE-like modes.
fn strip(si: &Material, width: f64, wavelength: Wavelength) -> photonoxide::Result<CrossSection> {
    let eps_si = si.permittivity(wavelength)?;
    let eps_ox = c64::new(1.444 * 1.444, 0.0);
    CrossSection::uniform((0.0, 1.05, 168), (0.0, 0.75, 150), |x, y| {
        let core = x < width / 2.0 && y < 0.11;
        Permittivity::isotropic(if core { eps_si } else { eps_ox })
    })?
    .with_boundaries(Boundaries {
        west: Boundary::ElectricWall,
        south: Boundary::MagneticWall,
        ..Boundaries::default()
    })
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let si = silicon()?;
    let wavelengths = [1.54, 1.55, 1.56]
        .iter()
        .map(|&l| Wavelength::um(l))
        .collect::<photonoxide::Result<Vec<_>>>()?;
    println!("220 nm strips of the book's Si in 1.444 oxide, 6.25 x 5 nm grid, quarter domain");
    let mut checks = common::Checks::default();
    let strips = [
        (0.40, 4.37),
        (0.45, 4.27),
        (0.50, 4.18),
        (0.55, 4.10),
        (0.60, 4.04),
    ];
    // each width's mode tracked over the wavelengths, the widths side by side, then printed in
    // order
    let tracked = photonoxide::parallel::map_in_order(&strips, |&(width, _)| {
        let modes = track(|l| strip(&si, width, l), &wavelengths, None, 3)?;
        let n: Vec<f64> = modes.iter().map(|m| m.effective_index().re).collect();
        let ng = group_index(&wavelengths, &n)?[1];
        Ok::<_, photonoxide::Error>((n[1], modes[1].te_fraction(), ng))
    });
    for (&(width, printed), result) in strips.iter().zip(tracked) {
        let (n_eff, te, ng) = result?;
        println!(
            "  {:.0} nm wide: n_eff {n_eff:.5}, TE fraction {te:.3}",
            width * 1000.0,
        );
        // read off a plot to ±0.005, the book's 20 nm mesh, and our corners' convergence
        checks.compare(
            &format!("n_g at 1.55 um, {:.0} nm wide", width * 1000.0),
            ng,
            printed,
            0.02,
        );
    }
    Ok(checks.finish())
}
