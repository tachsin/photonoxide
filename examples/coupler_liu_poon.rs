//! A directional coupler in 3D FDTD against Lumerical FDTD and Tidy3D.
//!
//! Z. Liu, J. K. S. Poon, "Comparison of Lumerical FDTD and Tidy3D for three-dimensional FDTD
//! simulations of passive silicon photonic components", Opt. Continuum 4, 2427 (2025),
//! [doi:10.1364/OPTCON.572107](https://doi.org/10.1364/OPTCON.572107) (open, arXiv:2506.16665),
//! Section 3.1, Figs. 4 and 28:
//!
//! - **The device:** gdsfactory's generic `coupler`: two 500 × 220 nm silicon guides 236 nm
//!   apart for 20 µm, each brought in and out by an S-bend (`bend_s`, a cubic Bézier 10 µm long
//!   and 1.632 µm across), in silica. TE₀ enters the lower left port.
//! - **The result:** the transmission into the cross port's TE₀ at 1550 nm against the
//!   resolution (Fig. 4(a)) and over the band at 15 cells a wavelength (Fig. 4(b)); the excess
//!   loss, the power in neither output's TE₀, in dB (Fig. 4(c), (d)).
//!
//! The paper's numbers, read off its figures (arXiv's PNGs, 3000 × 1800) where the text doesn't
//! print them, good to ±0.003 in the transmission and ±0.001 dB:
//!
//! | Cells a wavelength | Lumerical | Tidy3D |
//! |---|---|---|
//! | 6 | 0.493 (text: 49.3 %) | 0.697 (69.7 %) |
//! | 15 | 0.411 (41.1 %); 0.441 with a 50 nm pulse | 0.448 (44.8 %); 0.448 |
//! | 20 | 0.446 | 0.492 |
//! | 25 | 0.430 | 0.456 |
//! | excess loss at 15, 20, 25 | 0.000, 0.001, 0.000 dB | −0.013, −0.002, −0.007 dB |
//!
//! Tidy3D 2.8.5 with subpixel averaging (Fig. 28) gives about 0.47 from 10 cells on. The two
//! codes differ by up to 4.6 % (the text); Lumerical's 20 nm pulse is 3 % below its own 50 nm one
//! (Fig. 4(b)), which the paper traces to its non-uniform mesh changing with the band.
//!
//! Here (`pdk`'s run):
//!
//! - **At 15 and 20 cells a wavelength** (`--full`): the cross port at 1550 nm against the
//!   span of both codes' values from 15 to 25 cells, both pulses and Fig. 28's 0.47 (0.411 to
//!   0.492), its middle within half its width plus the reading; the excess loss the same.
//! - **At 6 cells a wavelength** (what CI runs, 74 nm cells): against the same settled values,
//!   within how far the paper's own codes stray from them at 6 cells (0.493 and 0.697), half
//!   their span and the reading: a coarse grid's error, bounded by the codes' own there.
//!
//! The coupling depends on the guides' supermodes, which a uniform grid of 30 nm resolves in a
//! gap of 236 nm with eight cells; the S-bends' share of the coupling depends on their curved
//! faces, which the smoothing's diagonal takes to first order.
//!
//! Measured (`--full`, 2026-10-09), the cross port's TE₀ and the excess loss at 1550 nm:
//!
//! | Cells a wavelength | Grid | Cross TE₀ | Excess loss | 1540 to 1560 nm |
//! |---|---|---|---|---|
//! | 15 | 1437 × 260 × 99 cells of 29.7 nm | 0.42771 | −0.0014 dB | 0.39639 to 0.46043 |
//! | 20 | 1908 × 338 × 124 cells of 22.3 nm | 0.41352 | −0.0017 dB | 0.38273 to 0.44576 |
//!
//! Both within the codes' 0.411 to 0.492, between Lumerical's 0.411 and Tidy3D's 0.448 at 15
//! cells; the band rises with the wavelength as both codes' does (Fig. 4(b), about 0.07 over 20
//! nm). Run times on 20 threads of a Core Ultra 7 265K, each while other work shared the machine
//! (so slower than it could be): at 15 cells (37.0 million cells, 16 478 steps) 63 minutes of
//! stepping and 2 of mode solves; at 20 (80.0 million, 21 980 steps) 111 and 3. The paper's Table
//! 2: at 15 cells 63 s for Tidy3D, 1293 s for Lumerical locally and 42 s on its cloud GPUs; at 20,
//! 150 s, 2606 s and 77 s.
//!
//! ```sh
//! cargo run --release --example coupler_liu_poon              # 6 cells a wavelength
//! cargo run --release --example coupler_liu_poon -- --full    # 15 and 20
//! ```

mod common;
mod pdk;

use std::process::ExitCode;

use pdk::{Kind, Output, Setup};

pub fn main() -> photonoxide::Result<ExitCode> {
    let device = pdk::coupler();
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
        "Liu and Poon's directional coupler (gdsfactory's generic PDK), 3D FDTD, TE0 into the \
         lower left port; materials at 1.55 um"
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
        pdk::report_cost("coupler_liu_poon", &m);
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
        // both codes from 15 to 25 cells a wavelength, both pulses, and Fig. 28's
        let settled = [0.411, 0.441, 0.446, 0.430, 0.448, 0.492, 0.456, 0.47];
        if coarse {
            pdk::coarse(
                &mut checks,
                "cross TE0, 1550 nm, 6 cells",
                cross,
                &settled,
                &[0.493, 0.697],
                0.003,
            );
        } else {
            let tag = format!("{res} cells");
            pdk::within(
                &mut checks,
                &format!("cross TE0, 1550 nm, {tag}"),
                cross,
                &settled,
                0.003,
            );
            pdk::within(
                &mut checks,
                &format!("excess loss (dB), 1550 nm, {tag}"),
                loss,
                &[0.0, 0.001, 0.0, -0.013, -0.002, -0.007],
                0.001,
            );
        }
    }
    Ok(checks.finish())
}
