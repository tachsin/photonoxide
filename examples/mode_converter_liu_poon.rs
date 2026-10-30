//! A mode converter in 3D FDTD against Lumerical FDTD and Tidy3D.
//!
//! Z. Liu, J. K. S. Poon, "Comparison of Lumerical FDTD and Tidy3D for three-dimensional FDTD
//! simulations of passive silicon photonic components", Opt. Continuum 4, 2427 (2025),
//! [doi:10.1364/OPTCON.572107](https://doi.org/10.1364/OPTCON.572107) (open, arXiv:2506.16665),
//! Section 3.4 and Fig. 16:
//!
//! - **The device:** gdsfactory's generic `mode_converter` (gap 0.15 µm, length 20 µm, after H.
//!   Shu et al.): a 500 nm guide beside a 1 µm one for 20 µm, 150 nm apart, the narrow one brought
//!   in and out by `bend_euler_s` (two 45° Euler bends of radius 10 µm), the wide one tapered to
//!   1.2 µm over 25 µm either side; 220 nm of silicon in silica. TE₀ enters the narrow guide.
//! - **The result:** the transmission into TE₁ of the wide guide's right port (the cross port)
//!   at 1550 nm against the resolution (Fig. 16(a)), and the crosstalk, the transmission into
//!   that port's TE₀, in dB (Fig. 16(c)).
//!
//! The paper's numbers, read off its figures (arXiv's PNGs) where the text doesn't print them,
//! good to ±0.005 in the transmission and ±0.2 dB:
//!
//! | Cells a wavelength | Lumerical | Tidy3D |
//! |---|---|---|
//! | 6 | 0.967 (text: 96.7 %), −47.3 dB | 0.357 (35.7 %), −39.6 dB |
//! | 15 | 0.443; 0.362 with a 50 nm pulse; −37.2 dB | 0.492 (49.2 %); 0.522; −35.4 dB |
//! | 20 | 0.482, −38.1 dB | 0.479, −35.3 dB |
//! | 25 | 0.512, −36.9 dB | 0.430, −35.6 dB |
//!
//! The paper finds both codes' spectra varying by ±0.1 within a few nanometres (Fig. 16(b)),
//! their peaks and valleys apart, and gives the two codes' discrepancy as 16 % (Table 14);
//! Lumerical with a fixed mesh of 29 nm still differs from Tidy3D by over 16 % (Fig. 17).
//!
//! Here (`pdk`'s run):
//!
//! - **At 15 cells a wavelength** (`--full`): TE₁ at 1550 nm against the span of both codes'
//!   values from 15 to 25 cells and both pulses (0.362 to 0.522), its middle within half its
//!   width plus the reading; the crosstalk the same.
//! - **At 5 cells a wavelength** (what CI runs, 89 nm cells, coarser than the paper's coarsest to
//!   keep CI short): against the same settled values,
//!   within how far the paper's own codes stray from them at 6 cells (0.967 and 0.357, −47.3 and
//!   −39.6 dB), half their span and the reading: a coarse grid's error, bounded by the codes'
//!   own there.
//!
//! At 5 cells (832 × 142 × 49 cells of 89.2 nm, 11 397 steps): TE₁ 0.457 and the crosstalk −39.2
//! dB.
//!
//! Measured at 15 cells (`--full`, the same as `--resolution 15`, 2026-10-17), the cross port at 1540 to 1560 nm:
//!
//! | Cells a wavelength | Grid | TE₁ at 1540, 1545, 1550, 1555, 1560 nm | TE₀ (crosstalk) at 1550 nm |
//! |---|---|---|---|
//! | 15 | 2446 × 376 × 99 cells of 29.7 nm | 0.46290, 0.46175, 0.45909, 0.45436, 0.44941 | −37.94 dB |
//!
//! Both within the codes' spread, the TE₁ 0.017 from its middle. The spectrum is smooth over the
//! band, where the paper's vary by ±0.1 within a few nanometres.
//!
//! The run: 91.0 million cells, 24 717 steps to 420 µm/c (decayed), 2 h 27 min of stepping on 20
//! threads of a Core Ultra 7 265K shared with other work (254 million cell-updates/s), 40 s of
//! smoothing and 84 s of mode solves.
//!
//! ```sh
//! cargo run --release --example mode_converter_liu_poon              # 5 cells a wavelength
//! cargo run --release --example mode_converter_liu_poon -- --full    # 15
//! ```

mod common;
mod pdk;

use std::process::ExitCode;

use pdk::{Kind, Output, Setup};

pub fn main() -> photonoxide::Result<ExitCode> {
    let device = pdk::mode_converter();
    // the wide guide's right port: TE1 and TE0
    let outputs = [Output {
        port: 2,
        kinds: vec![Kind::Te1, Kind::Te0],
    }];
    let resolutions = pdk::resolutions(&[15.0], 5.0);
    let settings = |res: f64| Setup {
        device: &device,
        h: pdk::step(res),
        wavelength: 1.55,
        input: Kind::Te0,
        outputs: &outputs,
        wavelengths: if res < 10.0 {
            &[1.55]
        } else {
            &pdk::WAVELENGTHS
        },
        limit: 3000.0,
        decay: true,
    };
    // both codes from 15 to 25 cells a wavelength, both pulses: TE1 and the crosstalk; their
    // values at 6 cells; the margins
    let settled = [0.443, 0.362, 0.482, 0.512, 0.492, 0.522, 0.479, 0.430];
    let settled_crosstalk = [-37.2, -38.1, -36.9, -35.4, -35.3, -35.6];
    let (at_six, crosstalk_at_six) = ([0.967, 0.357], [-47.3, -39.6]);
    let (margin, crosstalk_margin) = (0.005, 0.2);
    let mut setup = pdk::described(&settings(resolutions[0]), &resolutions);
    setup.item(
        "tolerance",
        &[
            pdk::tolerance("the cross port's TE1 at 1550 nm", &settled, margin),
            pdk::tolerance(
                "its TE0 crosstalk (dB)",
                &settled_crosstalk,
                crosstalk_margin,
            ),
            format!(
                "at 5 cells, plus how far the codes stray at 6 ({}; {} dB)",
                at_six.map(common::number).join(", "),
                crosstalk_at_six.map(common::number).join(", ")
            ),
        ]
        .join("; "),
    );
    setup.print();
    println!(
        "Liu and Poon's mode converter (gdsfactory's generic PDK), 3D FDTD, TE0 into the narrow \
         guide; materials at 1.55 um"
    );
    let mut checks = common::Checks::default();
    for res in resolutions {
        let coarse = res < 10.0;
        let m = pdk::run(&settings(res), &mut |_| Ok(()))?.measured;
        pdk::report_cost("mode_converter_liu_poon", &m);
        println!("{res} cells a wavelength: {}", pdk::describe(&m.grid));
        println!("  wavelength  cross TE1  cross TE0 (crosstalk)  reflection");
        for (k, l) in m.wavelengths.iter().enumerate() {
            let t = &m.transmission[k];
            println!(
                "  {l:.3} um    {:.5}    {:.2} dB             {:.2e}",
                t[0][0],
                pdk::db(t[0][1]),
                m.reflection[k],
            );
        }
        let (te1, crosstalk) = (m.at_1550(0, 0), pdk::db(m.at_1550(0, 1)));
        if coarse {
            pdk::coarse(
                &mut checks,
                &format!("cross TE1, 1550 nm, {res} cells"),
                te1,
                &settled,
                &at_six,
                margin,
            );
            pdk::coarse(
                &mut checks,
                &format!("crosstalk (dB), 1550 nm, {res} cells"),
                crosstalk,
                &settled_crosstalk,
                &crosstalk_at_six,
                crosstalk_margin,
            );
        } else {
            let tag = format!("{res} cells");
            pdk::within(
                &mut checks,
                &format!("cross TE1, 1550 nm, {tag}"),
                te1,
                &settled,
                margin,
            );
            pdk::within(
                &mut checks,
                &format!("crosstalk (dB), 1550 nm, {tag}"),
                crosstalk,
                &settled_crosstalk,
                crosstalk_margin,
            );
        }
    }
    Ok(checks.finish())
}
