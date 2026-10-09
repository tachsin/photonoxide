//! A 2 × 2 MMI in 3D FDTD against Lumerical FDTD and Tidy3D.
//!
//! Z. Liu, J. K. S. Poon, "Comparison of Lumerical FDTD and Tidy3D for three-dimensional FDTD
//! simulations of passive silicon photonic components", Opt. Continuum 4, 2427 (2025),
//! [doi:10.1364/OPTCON.572107](https://doi.org/10.1364/OPTCON.572107) (open, arXiv:2506.16665),
//! Section 3.3 and Fig. 12:
//!
//! - **The device:** gdsfactory's generic `mmi2x2_with_sbend` (H. Guan et al.'s layout): a
//!   multimode region 8 µm long, 1.6 µm wide at its ends and 1.48 µm from 2 to 6 µm, four 500 nm
//!   guides tapered to 700 nm over 1 µm, their centres 0.45 µm from its axis, and S-bends
//!   (`bend_s`, 11 µm long, 1.8 µm across) to ports 4.5 µm apart; 220 nm of silicon in silica.
//!   TE₀ enters the lower left port.
//! - **The result:** the transmission into the cross port's TE₀ at 1550 nm against the
//!   resolution (Fig. 12(a)) and over the band at 15 cells (Fig. 12(b)); the excess loss, the
//!   power in neither output's TE₀, in dB (Fig. 12(c), (d)).
//!
//! The paper's numbers, read off its figures (arXiv's PNGs) where the text doesn't print them,
//! good to ±0.002 in the transmission and ±0.01 dB in the loss:
//!
//! | Cells a wavelength | Lumerical | Tidy3D |
//! |---|---|---|
//! | 6 | 0.376 (text: 37.6 %) | 0.358 (35.8 %) |
//! | 15 | 0.483 | 0.479 (47.9 %) |
//! | 20 | 0.486 | 0.484 |
//! | 25 | 0.489 | 0.489 |
//! | excess loss at 15, 20, 25 | −0.14, −0.13, −0.12 dB | −0.155, −0.145, −0.125 dB |
//! | at 1.54 and 1.56 µm, 15 cells | 0.488, 0.477 | 0.4815, 0.4724 |
//!
//! Here (`pdk`'s run):
//!
//! - **At 15 and 20 cells a wavelength** (`--full`): the cross port at 1550 nm against the span
//!   of both codes' values from 15 to 25 cells, its middle within half its width plus the
//!   reading; the excess loss the same; the band's ends against both codes' at 15 cells.
//! - **At 6 cells a wavelength** (what CI runs, 74 nm cells): against the same settled values,
//!   within how far the paper's own codes stray from them at 6 cells (0.376 and 0.358), half
//!   their span and the reading: a coarse grid's error, bounded by the codes' own there.
//!
//! ```sh
//! cargo run --release --example mmi_liu_poon              # 6 cells a wavelength
//! cargo run --release --example mmi_liu_poon -- --full    # 15 and 20
//! ```

mod common;
mod pdk;

use std::process::ExitCode;

use pdk::{Kind, Output, Setup};

pub fn main() -> photonoxide::Result<ExitCode> {
    let device = pdk::mmi();
    // the cross port (upper right), then the bar port
    let outputs = [
        Output {
            port: 2,
            kinds: vec![Kind::Te0],
        },
        Output {
            port: 3,
            kinds: vec![Kind::Te0],
        },
    ];
    println!(
        "Liu and Poon's 2 x 2 MMI (gdsfactory's generic PDK), 3D FDTD, TE0 into the lower left \
         port; materials at 1.55 um"
    );
    let mut checks = common::Checks::default();
    for res in pdk::resolutions(&[15.0, 20.0], 6.0) {
        let coarse = res < 10.0;
        let wavelengths: &[f64] = if coarse { &[1.55] } else { &pdk::WAVELENGTHS };
        let m = pdk::run(
            &Setup {
                device: &device,
                h: pdk::step(res),
                wavelength: 1.55,
                input: Kind::Te0,
                outputs: &outputs,
                wavelengths,
                limit: 3000.0,
                decay: true,
            },
            &mut |_| Ok(()),
        )?
        .measured;
        pdk::report_cost("mmi_liu_poon", &m);
        println!("{res} cells a wavelength: {}", pdk::describe(&m.grid));
        println!("  wavelength  cross TE0  bar TE0   reflection  excess loss");
        for (k, l) in m.wavelengths.iter().enumerate() {
            let t = &m.transmission[k];
            println!(
                "  {l:.3} um    {:.5}    {:.5}   {:.2e}    {:.4} dB",
                t[0][0],
                t[1][0],
                m.reflection[k],
                pdk::db(t[0][0] + t[1][0])
            );
        }
        let cross = m.at_1550(0, 0);
        let loss = pdk::db(cross + m.at_1550(1, 0));
        // both codes from 15 to 25 cells a wavelength
        let settled = [0.483, 0.486, 0.489, 0.479, 0.484, 0.489];
        if coarse {
            pdk::coarse(
                &mut checks,
                "cross TE0, 1550 nm, 6 cells",
                cross,
                &settled,
                &[0.376, 0.358],
                0.002,
            );
        } else {
            let tag = format!("{res} cells");
            pdk::within(
                &mut checks,
                &format!("cross TE0, 1550 nm, {tag}"),
                cross,
                &settled,
                0.002,
            );
            pdk::within(
                &mut checks,
                &format!("excess loss (dB), 1550 nm, {tag}"),
                loss,
                &[-0.14, -0.13, -0.12, -0.155, -0.145, -0.125],
                0.01,
            );
        }
        // the band's ends, which the paper gives at 15 cells (Fig. 12(b))
        if res == 15.0 {
            pdk::within(
                &mut checks,
                "cross TE0, 1540 nm, 15 cells",
                m.transmission[0][0][0],
                &[0.488, 0.4815],
                0.002,
            );
            pdk::within(
                &mut checks,
                "cross TE0, 1560 nm, 15 cells",
                m.transmission[4][0][0],
                &[0.477, 0.4724],
                0.002,
            );
        }
    }
    Ok(checks.finish())
}
