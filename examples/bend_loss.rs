//! The radiation loss of bent slab waveguides: exact, against Marcuse's formula, and from the
//! full-vector solver on a conformally mapped grid.
//!
//! D. Marcuse, "Bending losses of the asymmetric slab waveguide", Bell Syst. Tech. J. 50, 2551
//! (1971), [doi:10.1002/j.1538-7305.1971.tb02620.x](https://doi.org/10.1002/j.1538-7305.1971.tb02620.x),
//! Eqs. 32–33: the loss of a slab bent at a large radius, from the straight slab's field. Here a
//! slab of 1.6 in 1.5, 1 µm thick, at 1 µm, TE (E normal to the bend plane). The exact loss
//! (`mode::bend`, Marcuse's Eq. 10 with nothing approximated) and his formula meet as the radius
//! grows, their difference falling as 1/R; the check is at 120 µm, within 10 %. (Beyond, the loss
//! nears round-off: 8e-14 at 160 µm.)
//!
//! Then the book's strip as a slab (2.845, 500 nm, in 1.444, at 1.55 µm) bent at 1 and 2 µm, by
//! the full-vector solver with M. Heiblum and J. H. Harris's conformal map (IEEE J. Quantum
//! Electron. 11, 75 (1975), [doi:10.1109/JQE.1975.1068563](https://doi.org/10.1109/JQE.1975.1068563))
//! and a PML outside the bend, against the exact bend: E normal to the bend plane, where the
//! map is exact, and E in it, where it isn't (printed; the validation report checks them).
//!
//! Last, the whole 500 × 220 nm strip on 220 nm SOI bent at 2 µm through photonoxide's bend
//! device ([`StripBend`]), printed: its 2D model, this exact bent slab with the effective index
//! method's slab index, and its 3D one, the full-vector solver's straight strip plus the
//! conformal map's shift. The 2D model's index is the effective index method's, a few per cent
//! high, and isn't the bend's.
//!
//! ```sh
//! cargo run --release --example bend_loss
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::Complex64 as c64;
use photonoxide::circuit::Fidelity;
use photonoxide::device::{Built, Device, StripBend, Values};
use photonoxide::mode::Polarization;
use photonoxide::mode::bend::{SlabBend, marcuse_loss};
use photonoxide::mode::dispersion::bend_loss_db;
use photonoxide::mode::slab::Slab;
use photonoxide::mode::vector::{self, Boundaries, Boundary, CrossSection, Permittivity, Pml};
use photonoxide::units::{Length, Wavelength};

/// The bend device, a 500 × 220 nm strip on 220 nm SOI bent at 2 µm and lossless, at
/// `fidelity`. jobs/bend-2um-2d.yaml and jobs/bend-2um-3d.yaml compute the same, to the bit.
pub fn bend_device(fidelity: Fidelity) -> photonoxide::Result<Built> {
    let bend = StripBend::new();
    let values = bend_values(&bend)?;
    bend.component(&values, fidelity)
}

/// The bend device's design: bent at 2 µm, lossless.
fn bend_values(bend: &StripBend) -> photonoxide::Result<Values> {
    Values::new(bend, &[("radius", 2.0), ("loss", 0.0)])
}

/// The bent slab's window: from the strip's centre, `INNER` µm inward (at most 0.9 R) to
/// `OUTER` µm outward, a PML from `PML_FROM` µm on, of strength `PML_STRENGTH`.
const INNER: f64 = 0.75;
const OUTER: f64 = 6.0;
const PML_FROM: f64 = 2.5;
const PML_STRENGTH: f64 = 3.0;

pub fn main() -> photonoxide::Result<ExitCode> {
    // Marcuse's slab: (core, cladding, thickness µm, wavelength µm), the radii (µm), and the
    // tolerance on Marcuse's loss over the exact one at the largest
    let marcuse_slab = (1.6, 1.5, 1.0, 1.0);
    let radii = [20.0, 40.0, 80.0, 120.0];
    let tolerance = 0.1;
    // the strip seen as a slab: (core, cladding, width µm, wavelength µm), its grid (µm), radii
    let strip_slab = (2.845, 1.444, 0.5, 1.55);
    let h = 0.005;
    let strip_radii = [1.0f64, 2.0];
    let mut setup = common::Setup::new();
    let (core, clad, t, um) = marcuse_slab;
    setup.part("a bent slab against Marcuse's formula");
    setup.layer("outside", None, "cladding", &common::n(clad));
    setup.layer("slab", Some(t), "core", &common::n(core));
    setup.layer("inside", None, "cladding", &common::n(clad));
    setup.item("wavelength", &format!("{} µm, TE", common::number(um)));
    setup.item(
        "radii",
        &format!("{} µm", radii.map(common::number).join(", ")),
    );
    setup.item(
        "solver",
        "the bent slab's exact modes (no grid), and Marcuse's Eqs. 32-33",
    );
    setup.item(
        "tolerance",
        &format!(
            "Marcuse's loss within {tolerance} of the exact one, relative, at {} µm",
            common::number(radii[3])
        ),
    );
    let (core, clad, t, um) = strip_slab;
    setup.part("the strip as a bent slab, full-vector (printed)");
    setup.layer("outside", None, "cladding", &common::n(clad));
    setup.layer("slab", Some(t), "core", &common::n(core));
    setup.layer("inside", None, "cladding", &common::n(clad));
    setup.item(
        "wavelength",
        &format!("{} µm, TE and TM", common::number(um)),
    );
    setup.item(
        "radii",
        &format!("{} µm", strip_radii.map(common::number).join(", ")),
    );
    setup.item(
        "window",
        &format!(
            "from {} inside the strip's centre to {} outside it, a PML from {} \
             out, strength {PML_STRENGTH}",
            common::length(INNER),
            common::length(OUTER),
            common::length(PML_FROM)
        ),
    );
    setup.item("grid", &format!("uniform, {}", common::length(h)));
    setup.item(
        "solver",
        "the full-vector mode solver on the conformally mapped cross-section, beside the exact",
    );
    let bend = StripBend::new();
    let values = bend_values(&bend)?;
    setup.part("the bend device at its 2D and 3D models (printed)");
    setup.device(&bend, &values, Wavelength::um(um)?);
    setup.wavelength(Wavelength::um(um)?);
    setup.medium(
        "core, cladding",
        "constant indices: Marcuse's slab's, and the strip's (the effective index method's \
         2.845 in Chrostowski & Hochberg's 1.444)",
    );
    setup.print();
    let mut checks = common::Checks::default();

    let (core, clad, t, um) = marcuse_slab;
    let (core, clad, t) = (core, clad, Length::um(t));
    let w = Wavelength::um(um)?;
    let straight = Slab::new(clad, core, clad, t)?.modes(Polarization::Te, w)[0].effective_index();
    println!("slab of 1.6 in 1.5, 1 um, at 1 um, TE; straight n_eff {straight:.6}; exact: no grid");
    println!("  radius   exact Im n_eff   Marcuse      ratio   exact loss per 90 deg");
    for r in radii {
        let exact = SlabBend::new(Length::um(r), clad, &[(core, t)], clad, Length::um(-0.5))?
            .fundamental(Polarization::Te, w)?;
        let marcuse = marcuse_loss(core, clad, t, Length::um(r), w, straight)?;
        println!(
            "  {r:>4} um  {:.4e}       {marcuse:.4e}   {:.3}   {:.3e} dB",
            exact.im,
            marcuse / exact.im,
            bend_loss_db(exact, w, Length::um(r))
        );
        if r == radii[3] {
            checks.compare(
                "Marcuse / exact loss at 120 um",
                marcuse / exact.im,
                1.0,
                tolerance,
            );
        }
    }

    let (core, clad, t, um) = strip_slab;
    let w = Wavelength::um(um)?;
    println!("strip as a slab: 2.845, 500 nm, in 1.444, at 1.55 um; full-vector on a 5 nm grid");
    for (polarization, label) in [
        (Polarization::Te, "E normal to the bend plane"),
        (Polarization::Tm, "E in the bend plane"),
    ] {
        let wall = if polarization == Polarization::Te {
            Boundary::ElectricWall
        } else {
            Boundary::MagneticWall
        };
        for r in strip_radii {
            // x from the strip's centre, outward positive; a PML from 2.5 to 6 um
            let pml = r * (OUTER / r).ln_1p() - r * (PML_FROM / r).ln_1p();
            let bent = CrossSection::bent(
                r,
                (
                    -INNER.max(-0.9 * r),
                    OUTER,
                    ((OUTER + INNER.min(0.9 * r)) / h).round() as usize,
                ),
                (0.0, 4.0 * h, 4),
                |x, _| {
                    let n = if x.abs() < t / 2.0 { core } else { clad };
                    Permittivity::isotropic(c64::new(n * n, 0.0))
                },
            )?
            .with_boundaries(Boundaries {
                south: wall,
                north: wall,
                ..Boundaries::default()
            })?
            .with_pml(Pml {
                east: pml,
                strength: PML_STRENGTH,
                ..Pml::default()
            })?;
            let found = vector::modes(&bent, w, 1, Some(core))?[0].effective_index();
            let exact = SlabBend::new(
                Length::um(r),
                clad,
                &[(core, Length::um(t))],
                clad,
                Length::um(-t / 2.0),
            )?
            .mode_near(polarization, w, found)?;
            println!(
                "  {label}, R {r} um: exact {:.6} + {:.3e}i, full-vector {:.6} + {:.3e}i",
                exact.re, exact.im, found.re, found.im
            );
        }
    }
    // the whole strip through photonoxide's bend device: its 2D model, the exact bent slab of
    // the strip seen from above (the effective index method's 2.84 in 1.444), and its 3D one,
    // the straight strip by Hadley's equations plus the conformal map's shift
    println!("the bend device: a 500 x 220 nm strip on 220 nm SOI bent at 2 um, at 1.55 um");
    for fidelity in [Fidelity::TwoD, Fidelity::ThreeD] {
        let built = bend_device(fidelity)?;
        let figure = |s: &str| built.figure(s).map_or(f64::NAN, |f| f.value);
        println!(
            "  {fidelity}: n_eff {:.5} along the arc, radiation {:.2e} dB per 90 deg ({})",
            figure("n_eff"),
            figure("radiation_90"),
            built.figures[0].grid
        );
    }
    Ok(checks.finish())
}
