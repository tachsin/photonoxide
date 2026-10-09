//! A polarization splitter-rotator in 3D FDTD against Lumerical FDTD and Tidy3D.
//!
//! Z. Liu, J. K. S. Poon, "Comparison of Lumerical FDTD and Tidy3D for three-dimensional FDTD
//! simulations of passive silicon photonic components", Opt. Continuum 4, 2427 (2025),
//! [doi:10.1364/OPTCON.572107](https://doi.org/10.1364/OPTCON.572107) (open, arXiv:2506.16665),
//! Section 3.5 and Fig. 21:
//!
//! - **The device:** gdsfactory's generic `polarization_splitter_rotator` (after D. Dai and J. E.
//!   Bowers): the input tapered from 540 to 690 nm over 4 µm, to 830 nm over 44 µm and to 900 nm
//!   over 1.867 µm; 7 µm of it beside a 405 nm guide 150 nm away, then tapered to 540 nm over
//!   14.33 µm while the narrow guide leaves by an S-bend (`bend_s`, 14.33 µm long, 5 µm across)
//!   widening to 540 nm; 220 nm of silicon on silica under silicon nitride (n = 2.0, the paper's
//!   approximation). TM₀ enters; the taper turns it into TE₁, which the coupler takes to the
//!   upper guide's TE₀.
//! - **The result:** the transmission into the upper port's TE₀ at 1550 nm against the
//!   resolution (Fig. 21(a)), and the crosstalk, the transmission into that port's TM₀, in dB
//!   (Fig. 21(c)).
//!
//! The paper's numbers, read off its figures (arXiv's PNGs) where the text doesn't print them,
//! good to ±0.003 in the transmission and ±0.5 dB:
//!
//! | Cells a wavelength | Lumerical | Tidy3D |
//! |---|---|---|
//! | 6 | 0.147 (text: 14.7 %), −51.0 dB | 0.051 (5.1 %), −32.4 dB |
//! | 15 | 0.945 (about 94 %), −49.1 dB | 0.901 (90.1 %), −32.4 dB |
//! | 20 | 0.942; 0.9425 and 0.946 with 20 and 50 nm pulses (Fig. 21(b)); −36.8 dB | 0.948; 0.9475; −35.4 dB |
//! | 25 | 0.955, −41.1 dB | 0.950, −42.6 dB |
//!
//! Both codes settle near 95 %, Lumerical from 15 cells and Tidy3D from 20, within 0.5 % of each
//! other (the text); their crosstalk agrees only from 20 cells.
//!
//! Here (`pdk`'s run):
//!
//! - **At 15 and 20 cells a wavelength** (`--full`): the upper port's TE₀ at 1550 nm against the
//!   span of both codes' settled values (20 and 25 cells, both pulses: 0.942 to 0.955), its middle
//!   within half its width plus the reading; the crosstalk against the span at 20 and 25 cells.
//! - **At 5 cells a wavelength** (what CI runs, 89 nm cells, coarser than the paper's coarsest to
//!   keep CI short): against the same settled values,
//!   within how far the paper's own codes stray from them at 6 cells (0.147 and 0.051: the
//!   device doesn't work there in either code), half their span and the reading.
//!
//! ```sh
//! cargo run --release --example splitter_rotator_liu_poon              # 5 cells a wavelength
//! cargo run --release --example splitter_rotator_liu_poon -- --full    # 15 and 20
//! ```

mod common;
mod pdk;

use std::process::ExitCode;

use pdk::{Kind, Output, Setup};

pub fn main() -> photonoxide::Result<ExitCode> {
    let device = pdk::splitter_rotator();
    // the upper port (TE0, then TM0), then the lower
    let outputs = [
        Output {
            port: 1,
            kinds: vec![Kind::Te0, Kind::Tm0],
        },
        Output {
            port: 2,
            kinds: vec![Kind::Te0, Kind::Tm0],
        },
    ];
    println!(
        "Liu and Poon's polarization splitter-rotator (gdsfactory's generic PDK), 3D FDTD, TM0 \
         into the input; silicon nitride (n = 2.0) above, materials at 1.55 um"
    );
    let mut checks = common::Checks::default();
    for res in pdk::resolutions(&[15.0, 20.0], 5.0) {
        let coarse = res < 10.0;
        let wavelengths: &[f64] = if coarse { &[1.55] } else { &pdk::WAVELENGTHS };
        let m = pdk::run(
            &Setup {
                device: &device,
                h: pdk::step(res),
                wavelength: 1.55,
                input: Kind::Tm0,
                outputs: &outputs,
                wavelengths,
                limit: 3000.0,
                decay: true,
            },
            &mut |_| Ok(()),
        )?
        .measured;
        pdk::report_cost("splitter_rotator_liu_poon", &m);
        println!("{res} cells a wavelength: {}", pdk::describe(&m.grid));
        println!(
            "  wavelength  upper TE0  upper TM0 (crosstalk)  lower TE0  lower TM0  reflection"
        );
        for (k, l) in m.wavelengths.iter().enumerate() {
            let t = &m.transmission[k];
            println!(
                "  {l:.3} um    {:.5}    {:.2} dB             {:.2e}   {:.2e}   {:.2e}",
                t[0][0],
                pdk::db(t[0][1]),
                t[1][0],
                t[1][1],
                m.reflection[k],
            );
        }
        let (te0, crosstalk) = (m.at_1550(0, 0), pdk::db(m.at_1550(0, 1)));
        // both codes at 20 and 25 cells a wavelength, both pulses
        let settled = [0.942, 0.9425, 0.946, 0.955, 0.948, 0.9475, 0.950];
        if coarse {
            pdk::coarse(
                &mut checks,
                &format!("upper TE0, 1550 nm, {res} cells"),
                te0,
                &settled,
                &[0.147, 0.051],
                0.003,
            );
        } else {
            let tag = format!("{res} cells");
            pdk::within(
                &mut checks,
                &format!("upper TE0, 1550 nm, {tag}"),
                te0,
                &settled,
                0.003,
            );
            pdk::within(
                &mut checks,
                &format!("crosstalk (dB), 1550 nm, {tag}"),
                crosstalk,
                &[-36.8, -41.1, -35.4, -42.6],
                0.5,
            );
        }
    }
    Ok(checks.finish())
}
