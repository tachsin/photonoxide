//! Hadley's standard rib through the rib waveguide device, against his Table I.
//!
//! G. R. Hadley, "High-accuracy finite-difference equations for dielectric waveguide analysis
//! II: dielectric corners", J. Lightwave Technol. 20, 1219 (2002),
//! [doi:10.1109/JLT.2002.800371](https://doi.org/10.1109/JLT.2002.800371), Section IV, Fig. 13
//! and Table I: a rib 3 µm wide in a 1 µm film of index 3.44 on a substrate of 3.40 under air,
//! at 1.15 µm, its quasi-TE index for slabs D = 0 to 0.7 µm left beside it, to 6 decimals.
//!
//! Here the rib is photonoxide's rib waveguide device (`device::RibWaveguide`) on that stack,
//! etched 1 − D deep (D = 0, etched through, is the strip waveguide device), its cross-section
//! solved by Hadley's equations on a uniform 50 nm grid: the half behind the rib's vertical
//! mirror plane, 5 µm wide, 4 µm below and above the film's middle (100 × 180 cells). The
//! device's 2D model, the effective index method, is printed beside it with its error: an
//! approximation, a few 1e-3 here.
//!
//! The tolerance is Table I's printed digits and the grid's own error (1.4e-6 on the 50 nm grid,
//! the validation case `stack/rib-hadley-table`).
//!
//! ```sh
//! cargo run --release --example rib_hadley
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::Complex64 as c64;
use photonoxide::device::{Mesh, RibWaveguide, Scheme, StripWaveguide, Symmetry, Values};
use photonoxide::material::{Material, Model, Provenance};
use photonoxide::mode::hadley;
use photonoxide::mode::vector::CrossSection;
use photonoxide::stack::{Layer, LayerStack};
use photonoxide::units::{Length, Wavelength};

/// Hadley's Table I: the TE index at D = 0, 0.1, …, 0.7 µm.
const TABLE: [f64; 8] = [
    3.412022, 3.412126, 3.412279, 3.412492, 3.412774, 3.413132, 3.413571, 3.414100,
];

/// A medium of index `n`, as the paper states it.
fn medium(name: &str, n: f64) -> photonoxide::Result<Material> {
    Material::new(
        name,
        Model::Constant(c64::new(n, 0.0)),
        Wavelength::um(1.0)?,
        Wavelength::um(1.3)?,
        Provenance {
            reference: "G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), Fig. 13".into(),
            doi: "10.1109/JLT.2002.800371".into(),
            data: format!("the paper's index, {n}"),
            temperature: None,
            notes: String::new(),
        },
    )
}

/// The quasi-TE index of `cs` at 1.15 µm by Hadley's equations.
fn te(cs: &CrossSection) -> photonoxide::Result<f64> {
    let lam = Wavelength::um(1.15)?;
    Ok(hadley::modes(cs, lam, 1, None)?[0].effective_index().re)
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let air = medium("air", 1.0)?;
    let stack = LayerStack::new(
        medium("substrate", 3.40)?,
        vec![Layer {
            name: "film".into(),
            thickness: Length::um(1.0),
            material: medium("film", 3.44)?,
            background: air.clone(),
        }],
        air,
    )?;
    let lam = Wavelength::um(1.15)?;
    // the window (µm across and up) and its half's cells; the rib's width and the film's
    // thickness (µm); the tolerance
    let (window, cells) = ((10.0, 9.0), (100, 180));
    let (width, thickness, tolerance) = (3.0, 1.0, 2e-6);
    let mesh = Mesh::window(window.0, window.1, cells.0, cells.1, Scheme::Hadley)
        .with_symmetry(Symmetry::Vertical);
    let rib = RibWaveguide::on(stack.clone(), "film")?.with_mesh(mesh);
    let strip = StripWaveguide::on(stack, "film")?.with_mesh(mesh);
    let design = |d: f64| {
        Values::new(
            &rib,
            &[
                ("width", width),
                ("thickness", thickness),
                ("etch_depth", thickness - d),
                ("wavelength", lam.to_um()),
            ],
        )
    };
    let mut setup = common::Setup::new();
    let slabs: Vec<f64> = (0..TABLE.len()).map(|k| k as f64 / 10.0).collect();
    setup.part(&format!(
        "the rib at D = {} µm (each D alike)",
        common::number(slabs[1])
    ));
    setup.device(&rib, &design(slabs[1])?, lam);
    setup.part("every D");
    setup.item(
        "slab",
        &format!(
            "D = {} µm, the etch {} − D deep; at D = 0 the strip device",
            slabs
                .iter()
                .map(|d| common::number(*d))
                .collect::<Vec<_>>()
                .join(", "),
            common::length(thickness)
        ),
    );
    setup.wavelength(lam);
    setup.item(
        "window",
        &format!(
            "{} across by {} up, its half behind a vertical mirror plane",
            common::length(window.0),
            common::length(window.1)
        ),
    );
    setup.item(
        "grid",
        &format!(
            "the half's {} × {} cells: {} by {}",
            cells.0,
            cells.1,
            common::length(window.0 / 2.0 / cells.0 as f64),
            common::length(window.1 / cells.1 as f64)
        ),
    );
    setup.item(
        "solver",
        "the full-vector mode solver by Hadley's equations, TE; the device's 2D model (the \
         effective index method) beside it",
    );
    setup.item(
        "tolerance",
        &format!("{tolerance:e} on Table I's 6 decimals"),
    );
    setup.print();
    println!(
        "Hadley's standard rib (3 um wide, a 1 um film of 3.44 on 3.40 under air, 1.15 um, TE) \
         through the rib device, by Hadley's equations on a 50 nm grid (the half, 100 x 180 \
         cells); the device's 2D model beside it"
    );
    println!("  D (um)   Table I     the device   difference   2D (EIM)    its error");
    let mut checks = common::Checks::default();
    let mut worst_eim: f64 = 0.0;
    for (k, &printed) in TABLE.iter().enumerate() {
        let d = k as f64 / 10.0;
        let (n, eim) = if k == 0 {
            let v = Values::new(
                &strip,
                &[
                    ("width", width),
                    ("thickness", thickness),
                    ("wavelength", lam.to_um()),
                ],
            )?;
            (te(&strip.cross_section(&v, lam)?)?, None)
        } else {
            let v = design(d)?;
            let (eim, _) = rib.effective_index(&v, lam)?;
            (te(&rib.cross_section(&v, lam)?)?, Some(eim))
        };
        match eim {
            Some(e) => {
                worst_eim = worst_eim.max((e - n).abs());
                println!(
                    "  {d:.1}      {printed:.6}    {n:.6}     {:+.1e}      {e:.6}    {:+.1e}",
                    n - printed,
                    e - n
                );
            }
            None => println!(
                "  {d:.1}      {printed:.6}    {n:.6}     {:+.1e}      (a strip)",
                n - printed
            ),
        }
        checks.compare(&format!("n_eff, D = {d:.1} um"), n, printed, tolerance);
    }
    println!(
        "the effective index method's largest error here: {worst_eim:.1e} (2D, printed, not checked)"
    );
    Ok(checks.finish())
}
