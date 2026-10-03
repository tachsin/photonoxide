//! The cross-over length of a strip-waveguide directional coupler, from its two supermodes.
//!
//! L. Chrostowski, M. Hochberg, *Silicon Photonics Design*, Cambridge University Press (2015),
//! [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168), Section 4.1:
//! two 500 × 220 nm silicon strips in oxide, a gap g apart, carry a symmetric and an
//! antisymmetric supermode whose effective indices differ by Δn; the light crosses from one
//! guide to the other over L_x = λ/(2Δn) (Eq. 4.5), and a coupler L long sends sin²(πΔn L/λ)
//! of the power across (Eqs. 4.1 and 4.3). Their Lumerical MODE solutions (Listings 4.5–4.7:
//! a 5.5 µm wide, 1.22 µm tall window with metal walls, 10 nm mesh, oxide 2 µm above and below)
//! fitted against the gap give L_x = 10^(0.0037645 g[nm] + 0.799434) µm (Eq. 4.14), "for
//! example, the coupler with gap g = 200 nm has a cross-over length of L_x = 37.5 µm".
//!
//! Here the supermodes come from the full-vector solver on a quarter of the same window, its
//! walls the book's metal (zero field), with mirror walls on the two symmetry planes: an
//! electric wall between the guides keeps the symmetric TE-like supermode, a magnetic one the
//! antisymmetric. The materials are the book's (Listing 3.1): its Lorentz fit to silicon and
//! oxide of index 1.444. The coupler is photonoxide's [`DirectionalCoupler`], built from the
//! supermodes over 1.53–1.57 µm.
//!
//! ```sh
//! cargo run --release --example directional_coupler
//! ```

mod common;

use std::f64::consts::PI;
use std::process::ExitCode;

use faer::c64;
use photonoxide::circuit::Component;
use photonoxide::circuit::components::DirectionalCoupler;
use photonoxide::material::{LorentzPole, Material, Model, Provenance};
use photonoxide::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity, modes};
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

/// The quarter x, y ≥ 0 of two strips a `gap` apart (µm), x = 0 midway between them and y = 0
/// at the silicon's mid-height, on a grid of `step` µm: 2.75 × 0.61 µm, the book's window.
fn quarter(
    si: &Material,
    gap: f64,
    step: f64,
    between: Boundary,
    wavelength: Wavelength,
) -> photonoxide::Result<CrossSection> {
    let eps_si = si.permittivity(wavelength)?;
    let eps_ox = c64::new(1.444 * 1.444, 0.0);
    let (nx, ny) = (
        (2.75 / step).round() as usize,
        (0.61 / step).round() as usize,
    );
    CrossSection::uniform((0.0, 2.75, nx), (0.0, 0.61, ny), |x, y| {
        let core = x > gap / 2.0 && x < gap / 2.0 + 0.5 && y < 0.11;
        Permittivity::isotropic(if core { eps_si } else { eps_ox })
    })?
    .with_boundaries(Boundaries {
        west: between,
        south: Boundary::MagneticWall,
        ..Boundaries::default()
    })
}

/// The two supermodes' effective indices at `wavelength` on a grid of `step` µm.
fn supermodes(si: &Material, step: f64, wavelength: Wavelength) -> photonoxide::Result<(f64, f64)> {
    let index = |wall| -> photonoxide::Result<f64> {
        let cs = quarter(si, 0.2, step, wall, wavelength)?;
        Ok(modes(&cs, wavelength, 1, None)?[0].effective_index().re)
    };
    Ok((
        index(Boundary::ElectricWall)?,
        index(Boundary::MagneticWall)?,
    ))
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let si = silicon()?;
    let l0 = Wavelength::um(1.55)?;
    let mut checks = common::Checks::default();
    println!(
        "two 500 x 220 nm strips of the book's Si in 1.444 oxide, 200 nm apart, quarter domain"
    );
    // the coupler, from the supermodes tracked over 1.53-1.57 um on a 10 nm grid
    let coupler = DirectionalCoupler::from_modes(
        |w| quarter(&si, 0.2, 0.01, Boundary::ElectricWall, w),
        |w| quarter(&si, 0.2, 0.01, Boundary::MagneticWall, w),
        l0,
        0.01,
    )?;
    let lx = coupler.supermodes().cross_over_length(l0);
    let s = coupler.s_matrix(l0, &[lx / 2.0])?;
    println!(
        "  10 nm grid: L_x {lx:.3} um; at L_x/2 {:.6} of the power crosses, {:.6} goes through",
        s.power(2, 0),
        s.power(3, 0)
    );
    for w in [1.53, 1.57] {
        let w = Wavelength::um(w)?;
        println!(
            "  10 nm grid: L_x at {} um {:.3} um",
            w.to_um(),
            coupler.supermodes().cross_over_length(w)
        );
    }
    // L_x at 1.55 um on a finer grid
    let (even, odd) = supermodes(&si, 0.005, l0)?;
    let fine = l0.to_um() / (2.0 * (even - odd));
    println!("  5 nm grid: n_eff {even:.5} (even), {odd:.5} (odd), L_x {fine:.3} um");
    // the book's Eq. 4.14 at 200 nm, 37.5 um as printed: a fit to their 10 nm mesh's points;
    // ours moves 0.4 um from 10 to 5 nm, the corners converging slowly
    checks.compare("L_x at a 200 nm gap (um), 5 nm grid", fine, 37.5, 1.0);
    Ok(checks.finish())
}
