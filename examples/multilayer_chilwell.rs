//! The bound modes and leaky waves of a four-layer planar guide, by transfer matrices.
//!
//! J. Chilwell, I. Hodgkinson, "Thin-films field-transfer matrix theory of planar multilayer
//! waveguides and reflection from prism-loaded waveguides", J. Opt. Soc. Am. A 1, 742 (1984),
//! [doi:10.1364/JOSAA.1.000742](https://doi.org/10.1364/JOSAA.1.000742): their Fig. 2 guide,
//! cover 1.0, films 1.66, 1.53, 1.60 and 1.66 (500 nm each), substrate 1.50, at 632.8 nm.
//!
//! - Table 3: the four TE and four TM bound modes, to 6 decimals, and the share of each one's
//!   power in every layer, to 0.1 %;
//! - Table 2: five TE leaky waves below the substrate's index, complex, to 5 decimals. One
//!   printed value, m = 5's real part 1.38250, is one unit in the last place above ours
//!   (1.3824892); the other nine are ours rounded.
//!
//! ```sh
//! cargo run --release --example multilayer_chilwell
//! ```

mod common;

use std::process::ExitCode;

use num_complex::Complex64 as c64;
use photonoxide::mode::Polarization;
use photonoxide::mode::multilayer::{Multilayer, Region};
use photonoxide::units::{Length, Wavelength};

fn main() -> photonoxide::Result<ExitCode> {
    let films: Vec<(c64, Length)> = [1.66, 1.53, 1.60, 1.66]
        .iter()
        .map(|&n| (c64::new(n, 0.0), Length::nm(500.0)))
        .collect();
    let stack = Multilayer::new(c64::new(1.0, 0.0), &films, c64::new(1.5, 0.0))?;
    let wavelength = Wavelength::nm(632.8)?;
    let mut checks = common::Checks::default();

    println!("bound modes (Table 3), exact: no grid; power in cover, films 1-4, substrate (%)");
    for (polarization, name, printed) in [
        (
            Polarization::Te,
            "TE",
            [
                (1.622729, [0.0, 0.1, 0.4, 19.3, 76.3, 3.9]),
                (1.605276, [1.0, 87.1, 11.4, 0.2, 0.2, 0.0]),
                (1.557136, [0.0, 1.4, 12.3, 59.0, 20.7, 6.6]),
                (1.503587, [0.3, 6.3, 28.0, 14.7, 18.6, 32.1]),
            ],
        ),
        (
            Polarization::Tm,
            "TM",
            [
                (1.620031, [0.0, 0.0, 0.5, 21.4, 74.3, 3.8]),
                (1.594788, [0.5, 83.4, 15.4, 0.4, 0.3, 0.0]),
                (1.554981, [0.0, 2.2, 12.0, 57.3, 21.3, 7.3]),
                (1.501818, [0.1, 4.2, 22.7, 13.0, 15.1, 45.0]),
            ],
        ),
    ] {
        let modes = stack.bound_modes(polarization, wavelength)?;
        checks.count(&format!("{name} bound modes"), modes.len(), 4);
        for (order, (m, (index, shares))) in modes.iter().zip(printed).enumerate() {
            checks.compare(
                &format!("{name}{order} effective index"),
                m.effective_index().re,
                index,
                5e-7,
            );
            let got: Vec<String> = m
                .power_fractions()
                .iter()
                .map(|p| format!("{:.1}", 100.0 * p))
                .collect();
            println!("    power: {}  (paper {shares:?})", got.join(" "));
            let worst = m
                .power_fractions()
                .iter()
                .zip(shares)
                .map(|(g, p)| (100.0 * g - p).abs())
                .fold(0.0, f64::max);
            checks.compare(
                &format!("{name}{order} power: worst error (% points)"),
                worst,
                0.0,
                0.06,
            );
        }
    }

    println!("TE leaky waves (Table 2), exact: no grid");
    let region = Region {
        re_min: 1.0,
        re_max: 1.499,
        im_min: 0.0,
        im_max: 0.1,
    };
    let leaky = stack.modes_in(Polarization::Te, wavelength, region, 120);
    checks.count("TE leaky waves below 1.5", leaky.len(), 5);
    for (m, (order, re, im)) in leaky.iter().zip([
        (4, 1.46186, 0.00716),
        (5, 1.38250, 0.01817),
        (6, 1.28136, 0.03588),
        (7, 1.14231, 0.05288),
        (8, 1.00304, 0.07077),
    ]) {
        let n = m.effective_index();
        checks.compare(&format!("TE{order} Re n_eff"), n.re, re, 1.5e-5);
        checks.compare(&format!("TE{order} Im n_eff"), n.im, im, 5e-6);
    }
    Ok(checks.finish())
}
