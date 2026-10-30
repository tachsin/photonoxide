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
//! The coupler is photonoxide's directional coupler device ([`StripCoupler`]) on the book's
//! materials (Listing 3.1: its Lorentz fit to silicon, and oxide of index 1.444), its 3D model
//! the [`DirectionalCoupler`] built from the supermodes over 1.53–1.57 µm. The supermodes come
//! from the full-vector solver on a quarter of the book's window, its walls the book's metal
//! (zero field), with mirror walls on the two symmetry planes: an electric wall between the
//! guides keeps the symmetric TE-like supermode, a magnetic one the antisymmetric.
//!
//! ```sh
//! cargo run --release --example directional_coupler
//! ```

mod common;

use std::f64::consts::PI;
use std::process::ExitCode;

use faer::c64;
use photonoxide::circuit::components::DirectionalCoupler;
use photonoxide::circuit::{Component, Fidelity};
use photonoxide::device::{Mesh, Scheme, StripCoupler, Values};
use photonoxide::material::{LorentzPole, Material, Model, Provenance};
use photonoxide::mode::vector::modes;
use photonoxide::stack::{Layer, LayerStack};
use photonoxide::units::{Frequency, Length, Wavelength};

/// The book's provenance for its materials.
fn book(data: &str) -> Provenance {
    Provenance {
        reference: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), Eq. 3.2".into(),
        doi: "10.1017/CBO9781316084168".into(),
        data: data.into(),
        temperature: Some(300.0),
        notes: String::new(),
    }
}

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
        book("Listing 3.1, a fit to Palik's handbook"),
    )
}

/// The book's stack: 220 nm of its silicon on 2 µm of oxide (1.444) under oxide.
fn stack() -> photonoxide::Result<LayerStack> {
    let ox = Material::new(
        "oxide (1.444)",
        Model::Constant(c64::new(1.444, 0.0)),
        Wavelength::um(1.15)?,
        Wavelength::um(1.8)?,
        book("Listing 3.1"),
    )?;
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
            layer("Si", Length::nm(220.0), silicon()?),
        ],
        ox.clone(),
    )
}

/// The coupler device on the book's stack, a quarter of its 5.5 × 1.22 µm window in cells of
/// `step` µm.
fn quarter(step: f64) -> photonoxide::Result<StripCoupler> {
    let (nx, ny) = (
        (WINDOW.0 / 2.0 / step).round() as usize,
        (WINDOW.1 / 2.0 / step).round() as usize,
    );
    Ok(StripCoupler::on(stack()?, "Si")?.with_mesh(Mesh::window(
        WINDOW.0,
        WINDOW.1,
        nx,
        ny,
        Scheme::Standard,
    )))
}

/// The cross-section's window, µm across and up, centred on the pair.
const WINDOW: (f64, f64) = (5.5, 1.22);
/// The grids, µm: the 3D model's, and the finer one of the check.
const GRIDS: (f64, f64) = (0.01, 0.005);
/// The book's L_x (µm) and the tolerance: their fit to a 10 nm mesh, and our corners.
const BOOK: f64 = 37.5;
const TOLERANCE: f64 = 1.0;

pub fn main() -> photonoxide::Result<ExitCode> {
    let l0 = Wavelength::um(1.55)?;
    let band = [1.53, 1.57];
    let device = quarter(GRIDS.0)?;
    let values = Values::new(&device, &[("span", 0.04), ("loss", 0.0)])?;
    let mut setup = common::Setup::new();
    setup.device(&device, &values, l0);
    setup.item(
        "wavelengths",
        &format!(
            "{} µm; the supermodes tracked over {} to {} µm",
            common::number(l0.to_um()),
            band[0],
            band[1]
        ),
    );
    setup.item(
        "window",
        &format!(
            "{} across by {} up, centred on the pair; its quarter",
            common::length(WINDOW.0),
            common::length(WINDOW.1)
        ),
    );
    setup.item(
        "grid",
        &format!(
            "uniform, {} (the 3D model) and {} (L_x checked)",
            common::length(GRIDS.0),
            common::length(GRIDS.1)
        ),
    );
    setup.item(
        "solver",
        "the full-vector mode solver, the standard scheme: the even and odd supermodes, \
         L_x = λ/(2(n_even − n_odd))",
    );
    setup.item(
        "tolerance",
        &format!(
            "{} on the book's L_x = {}",
            common::length(TOLERANCE),
            common::length(BOOK)
        ),
    );
    setup.print();
    let mut checks = common::Checks::default();
    println!(
        "two 500 x 220 nm strips of the book's Si in 1.444 oxide, 200 nm apart, quarter domain"
    );
    // the coupler's 3D model, the supermodes tracked over 1.53-1.57 um on a 10 nm grid, lossless
    let coupler: DirectionalCoupler = device.coupler(&values, Fidelity::ThreeD)?;
    let lx = coupler.supermodes().cross_over_length(l0);
    let s = coupler.s_matrix(l0, &[lx / 2.0])?;
    println!(
        "  10 nm grid: L_x {lx:.3} um; at L_x/2 {:.6} of the power crosses, {:.6} goes through",
        s.power(2, 0),
        s.power(3, 0)
    );
    for w in band {
        let w = Wavelength::um(w)?;
        println!(
            "  10 nm grid: L_x at {} um {:.3} um",
            w.to_um(),
            coupler.supermodes().cross_over_length(w)
        );
    }
    // L_x at 1.55 um on a finer grid: the device's two cross-sections, solved
    let fine = quarter(GRIDS.1)?;
    let (even, odd) = fine.cross_sections(&Values::defaults(&fine), l0)?;
    let index =
        |cs| -> photonoxide::Result<f64> { Ok(modes(cs, l0, 1, None)?[0].effective_index().re) };
    let (even, odd) = (index(&even)?, index(&odd)?);
    let fine = l0.to_um() / (2.0 * (even - odd));
    println!("  5 nm grid: n_eff {even:.5} (even), {odd:.5} (odd), L_x {fine:.3} um");
    // the book's Eq. 4.14 at 200 nm, 37.5 um as printed: a fit to their 10 nm mesh's points;
    // ours moves 0.4 um from 10 to 5 nm, the corners converging slowly
    checks.compare("L_x at a 200 nm gap (um), 5 nm grid", fine, BOOK, TOLERANCE);
    Ok(checks.finish())
}
