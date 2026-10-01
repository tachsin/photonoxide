//! The radiation loss of bent slab waveguides: exact, against Marcuse's formula, and from the
//! full-vector solver on a conformally mapped grid.
//!
//! D. Marcuse, "Bending losses of the asymmetric slab waveguide", Bell Syst. Tech. J. 50, 2551
//! (1971), [doi:10.1002/j.1538-7305.1971.tb02620.x](https://doi.org/10.1002/j.1538-7305.1971.tb02620.x),
//! Eqs. 32–33: the loss of a slab bent at a large radius, from the straight slab's field. Here a
//! slab of 1.6 in 1.5, 1 µm thick, at 1 µm, TE (E normal to the bend plane). The exact loss
//! (`mode::bend`, Marcuse's Eq. 10 with nothing approximated) and his formula meet as the radius
//! grows, their difference falling as 1/R; the check is at 160 µm, within 10 %.
//!
//! Then the book's strip as a slab (2.845, 500 nm, in 1.444, at 1.55 µm) bent at 1 and 2 µm, by
//! the full-vector solver with M. Heiblum and J. H. Harris's conformal map (IEEE J. Quantum
//! Electron. 11, 75 (1975), [doi:10.1109/JQE.1975.1068563](https://doi.org/10.1109/JQE.1975.1068563))
//! and a PML outside the bend, against the exact bend: E normal to the bend plane, where the
//! map is exact, and E in it, where it isn't (printed; the validation report checks them).
//!
//! ```sh
//! cargo run --release --example bend_loss
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::Complex64 as c64;
use photonoxide::mode::Polarization;
use photonoxide::mode::bend::{SlabBend, marcuse_loss};
use photonoxide::mode::dispersion::bend_loss_db;
use photonoxide::mode::slab::Slab;
use photonoxide::mode::vector::{self, Boundaries, Boundary, CrossSection, Permittivity, Pml};
use photonoxide::units::{Length, Wavelength};

fn main() -> photonoxide::Result<ExitCode> {
    let mut checks = common::Checks::default();

    let (core, clad, t) = (1.6, 1.5, Length::um(1.0));
    let w = Wavelength::um(1.0)?;
    let straight = Slab::new(clad, core, clad, t)?.modes(Polarization::Te, w)[0].effective_index();
    println!("slab of 1.6 in 1.5, 1 um, at 1 um, TE; straight n_eff {straight:.6}; exact: no grid");
    println!("  radius   exact Im n_eff   Marcuse      ratio   exact loss per 90 deg");
    for r in [20.0, 40.0, 80.0, 160.0] {
        let exact = SlabBend::new(Length::um(r), clad, &[(core, t)], clad, Length::um(-0.5))?
            .fundamental(Polarization::Te, w)?;
        let marcuse = marcuse_loss(core, clad, t, Length::um(r), w, straight);
        println!(
            "  {r:>4} um  {:.4e}       {marcuse:.4e}   {:.3}   {:.3e} dB",
            exact.im,
            marcuse / exact.im,
            bend_loss_db(exact, w, Length::um(r))
        );
        if r == 160.0 {
            checks.compare(
                "Marcuse / exact loss at 160 um",
                marcuse / exact.im,
                1.0,
                0.1,
            );
        }
    }

    let (core, clad, t) = (2.845, 1.444, 0.5);
    let w = Wavelength::um(1.55)?;
    let h = 0.005;
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
        for r in [1.0f64, 2.0] {
            // x from the strip's centre, outward positive; a PML from 2.5 to 6 um
            let pml = r * (6.0 / r).ln_1p() - r * (2.5 / r).ln_1p();
            let bent = CrossSection::bent(
                r,
                (
                    -0.75f64.max(-0.9 * r),
                    6.0,
                    ((6.0 + 0.75f64.min(0.9 * r)) / h).round() as usize,
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
                strength: 3.0,
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
    Ok(checks.finish())
}
