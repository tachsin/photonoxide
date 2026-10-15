//! The MMI devices' self-imaging lengths against Soldano and Pennings's closed forms.
//!
//! L. B. Soldano, E. C. M. Pennings, "Optical multi-mode interference devices based on
//! self-imaging: principles and applications", J. Lightwave Technol. 13, 615 (1995),
//! [doi:10.1109/50.372474](https://doi.org/10.1109/50.372474), Section III:
//!
//! - the beat length of the two lowest modes, L_π ≈ 4 n_r W_e² / (3 λ₀) (Eq. 6), with the
//!   effective width W_e = W_M + (λ₀/π)(n_c/n_r)^{2σ}(n_r² − n_c²)^{−1/2} (Eq. 4), σ = 1 for the
//!   lateral TM field of a TE-like guide;
//! - a 1 × 2 splitter fed in its middle forms its two-fold image at 3L_π/8 (symmetric
//!   interference, Eq. 37 with N = 2), and a 2 × 2 coupler fed at ±W_e/6 at L_π/2 (paired
//!   interference, Eq. 33 with N = 2).
//!
//! Here photonoxide's MMI devices (`device::StripMmi`) on 220 nm SOI, their 2D model: the plane
//! seen from above by the effective index method (n_r the 220 nm film's TE index in silica, n_c
//! silica's, Li's silicon and Malitson's silica at 1.55 µm), the section's exact slab modes. The
//! device's first two-fold image (its `L_image`, from the exact β₀ − β₁) against the paper's,
//! for sections 3 to 12 µm wide: the paraxial approximation of Eq. 6 is the paper's error, about
//! 1 % at 3 µm and less as the section widens. At each device's own length, the guided-mode
//! analysis's power into each output and its excess loss are printed (2D, an estimate, not the
//! device's performance: exact slab modes, no grid).
//!
//! ```sh
//! cargo run --release --example mmi_soldano
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::circuit::Fidelity;
use photonoxide::circuit::components::MmiKind;
use photonoxide::device::{Device, StripMmi, Values};
use photonoxide::generator::{Mmi, SelfImaging};
use photonoxide::material;
use photonoxide::mode::Polarization;
use photonoxide::mode::slab::Slab;
use photonoxide::units::{Length, Wavelength};

pub fn main() -> photonoxide::Result<ExitCode> {
    let lam = Wavelength::um(1.55)?;
    let n =
        |m: &material::Material| -> photonoxide::Result<f64> { Ok(m.refractive_index(lam)?.re) };
    let (nf, nc) = (n(&material::silicon())?, n(&material::silica())?);
    let nr =
        Slab::new(nc, nf, nc, Length::nm(220.0))?.modes(Polarization::Te, lam)[0].effective_index();
    println!(
        "Soldano and Pennings's self-imaging lengths against the MMI devices' on 220 nm SOI at \
         1.55 um (2D: the effective index method, n_r {nr:.5}, n_c {nc:.5}; exact slab modes, no \
         grid)"
    );
    let mut checks = common::Checks::default();
    for (device, kind) in [
        (StripMmi::one_by_two(), MmiKind::OneByTwo),
        (StripMmi::two_by_two(), MmiKind::TwoByTwo),
    ] {
        println!("{}:", device.kind());
        println!(
            "  width (um)  L_pi, Eq. 6   the device's   image, Eq. {}   the device's   outputs   excess loss (dB)",
            if kind == MmiKind::OneByTwo {
                "37"
            } else {
                "33"
            }
        );
        for (width, access) in [(3.0, 0.8), (6.0, 1.5), (12.0, 1.5)] {
            let v = Values::new(
                &device,
                &[
                    ("width", width),
                    ("access_width", access),
                    ("port_width", access),
                ],
            )?;
            let figure =
                |b: &photonoxide::device::Built, s: &str| b.figure(s).map_or(f64::NAN, |f| f.value);
            let b = device.component(&v, Fidelity::TwoD)?;
            let beat = figure(&b, "L_pi");
            let image = figure(&b, "L_image");
            let paper_beat = Mmi::beat_length(Length::um(width), nr, nc, Length::um(1.55), true);
            let mechanism = match kind {
                MmiKind::OneByTwo => SelfImaging::Symmetric,
                MmiKind::TwoByTwo => SelfImaging::Paired,
            };
            let paper = mechanism.length(2, paper_beat).to_um();
            // at its own image
            let at = device.component(&v.with(&device, "length", image)?, Fidelity::TwoD)?;
            let (a, c) = match kind {
                MmiKind::OneByTwo => (figure(&at, "P_o2"), figure(&at, "P_o3")),
                MmiKind::TwoByTwo => (figure(&at, "P_o3"), figure(&at, "P_o4")),
            };
            println!(
                "  {width:>5.1}       {:>9.3}     {beat:>9.3}      {paper:>9.3}       {image:>9.3}    {a:.4}, {c:.4}   {:.3}",
                paper_beat.to_um(),
                figure(&at, "excess_loss")
            );
            checks.against(
                &format!("{} image, {width} um wide (um)", device.kind()),
                image,
                "Soldano & Pennings",
                paper,
                (12.0 * paper).round() / 1000.0,
            );
        }
    }
    Ok(checks.finish())
}
