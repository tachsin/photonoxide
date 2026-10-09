//! A waveguide crossing in 3D FDTD against Lumerical FDTD and Tidy3D.
//!
//! Z. Liu, J. K. S. Poon, "Comparison of Lumerical FDTD and Tidy3D for three-dimensional FDTD
//! simulations of passive silicon photonic components", Opt. Continuum 4, 2427 (2025),
//! [doi:10.1364/OPTCON.572107](https://doi.org/10.1364/OPTCON.572107) (open, arXiv:2506.16665),
//! Section 3.2 and Fig. 8:
//!
//! - **The device:** gdsfactory's generic `crossing`: in 220 nm of silicon, two arms of 500 nm
//!   tapered to 1.2 µm over 3.4 µm either side of a 1.2 µm square; in the 150 nm slab, two
//!   ellipses of semi-axes 3 and 1.1 µm across each other; silica above and below. The fundamental
//!   TE mode enters at the left arm.
//! - **The result:** the transmission into the through port's TE₀ at 1550 nm against the
//!   resolution (Fig. 8(a)) and over the band at 15 cells a wavelength (Fig. 8(b)), and the
//!   excess loss, the power in no output's TE₀, in dB (Fig. 8(c), (d)).
//!
//! The paper's numbers, read off its figures (arXiv's PNGs, 3000 × 1800) where the text doesn't
//! print them, the reading good to ±0.0003 in the transmission and ±0.001 dB:
//!
//! | Cells a wavelength | Lumerical | Tidy3D |
//! |---|---|---|
//! | 6 | 0.954 (text: 95.4 %) | 0.939 (93.9 %) |
//! | 15 | 0.957 (95.7 %) | 0.959 (95.9 %) |
//! | 20 | 0.957 | 0.9558 |
//! | 25 | 0.957 (95.7 %) | 0.9567 (95.7 %) |
//! | excess loss at 15, 20, 25 | −0.1925, −0.191, −0.1915 dB | −0.183, −0.1965, −0.1925 dB |
//! | at 1.54 and 1.56 µm, 15 cells, 20 nm | 0.9525, 0.9596 | 0.9542, 0.9613 |
//!
//! Here (`pdk`'s run, on photonoxide's grid at N cells a wavelength in silicon):
//!
//! - **At 15 and 20 cells a wavelength** (`--full`), the through port at 1550 nm is checked
//!   against the span of both codes' values from 15 to 25 cells, its middle within half its
//!   width plus the reading; the excess loss the same; the band's ends against both codes' at
//!   15 cells.
//! - **At 6 cells a wavelength** (what CI runs, 74 nm cells): against the same settled values,
//!   within how far the paper's own codes stray from them at 6 cells (0.954 and 0.939), half
//!   their span and the reading: a coarse grid's error, bounded by the codes' own there.
//!
//! Measured on 20 threads of a Core Ultra 7 265K: at 15 cells (14.1 million cells, 14 124
//! steps), 8 minutes of stepping and 3 of solving modes; the paper's Table 4 has, at 15 cells,
//! 19 s for Tidy3D (GPUs in the cloud), 85 s for Lumerical locally (AMD 3960X) and 10 s on its
//! cloud GPUs, on its non-uniform grids, which are coarser in the cladding.
//!
//! ```sh
//! cargo run --release --example crossing_liu_poon              # 6 cells a wavelength
//! cargo run --release --example crossing_liu_poon -- --full    # 15 and 20
//! ```

mod common;
mod pdk;

use std::process::ExitCode;

use pdk::{Kind, Output, Setup};

pub fn main() -> photonoxide::Result<ExitCode> {
    let device = pdk::crossing();
    // the through port, then the two side ports
    let outputs = [
        Output {
            port: 2,
            kinds: vec![Kind::Te0],
        },
        Output {
            port: 1,
            kinds: vec![Kind::Te0],
        },
        Output {
            port: 3,
            kinds: vec![Kind::Te0],
        },
    ];
    println!(
        "Liu and Poon's crossing (gdsfactory's generic PDK), 3D FDTD, TE0 into the left arm; \
         materials at 1.55 um"
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
                limit: 2000.0,
                decay: true,
            },
            &mut |_| Ok(()),
        )?
        .measured;
        pdk::report_cost("crossing_liu_poon", &m);
        println!("{res} cells a wavelength: {}", pdk::describe(&m.grid));
        println!("  wavelength  through TE0  side TE0s            reflection  excess loss");
        for (k, l) in m.wavelengths.iter().enumerate() {
            let t = &m.transmission[k];
            println!(
                "  {l:.3} um    {:.5}      {:.2e}, {:.2e}   {:.2e}    {:.4} dB",
                t[0][0],
                t[1][0],
                t[2][0],
                m.reflection[k],
                pdk::db(t[0][0] + t[1][0] + t[2][0])
            );
        }
        let through = m.at_1550(0, 0);
        let loss = pdk::db(through + m.at_1550(1, 0) + m.at_1550(2, 0));
        // both codes from 15 to 25 cells a wavelength
        let settled = [0.957, 0.959, 0.957, 0.9558, 0.957, 0.9567];
        if coarse {
            pdk::coarse(
                &mut checks,
                "through TE0, 1550 nm, 6 cells",
                through,
                &settled,
                &[0.954, 0.939],
                0.0003,
            );
        } else {
            let tag = format!("{res} cells");
            pdk::within(
                &mut checks,
                &format!("through TE0, 1550 nm, {tag}"),
                through,
                &settled,
                0.0003,
            );
            pdk::within(
                &mut checks,
                &format!("excess loss (dB), 1550 nm, {tag}"),
                loss,
                &[-0.1925, -0.191, -0.1915, -0.183, -0.1965, -0.1925],
                0.001,
            );
        }
        // the band's ends, which the paper gives at 15 cells (Fig. 8(b), 20 nm pulses)
        if res == 15.0 {
            pdk::within(
                &mut checks,
                "through TE0, 1540 nm, 15 cells",
                m.transmission[0][0][0],
                &[0.9525, 0.9542],
                0.0003,
            );
            pdk::within(
                &mut checks,
                "through TE0, 1560 nm, 15 cells",
                m.transmission[4][0][0],
                &[0.9596, 0.9613],
                0.0003,
            );
        }
    }
    Ok(checks.finish())
}
