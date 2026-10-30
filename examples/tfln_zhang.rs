//! Zhang et al.'s thin-film lithium niobate modulators through the TFLN Mach–Zehnder modulator
//! device.
//!
//! Y. Zhang, L. Shao, J. Yang, Z. Chen, K. Zhang, K.-M. Shum, D. Zhu, C. H. Chan, M. Loncar, C.
//! Wang, "Systematic investigation of millimeter-wave optic modulation performance in thin-film
//! lithium niobate", Photon. Res. 10, 2380 (2022),
//! [doi:10.1364/PRJ.468518](https://doi.org/10.1364/PRJ.468518): a 600 nm x-cut film etched
//! 300 nm to ribs 1.2 µm wide on top in the 5 µm gaps of a coplanar line (a 20 µm signal of 800
//! nm of copper under 30 nm of gold), 2 µm of thermal oxide on silicon, 0.6 µm of PECVD silica
//! (Section 3.A). Measured (Sections 2 and 3.C): Vπ·L 2.3 V·cm at DC and 2.67 at 1 GHz; the
//! 5.8 mm modulator's 3 dB and 6 dB electro-optic bandwidths 170 and 295 GHz and its RF Vπ 8.3 V
//! at 250 GHz, the 10.8 mm one's 100 and 175 GHz and 7.3 V.
//!
//! The device (`device::TflnModulator`, its defaults the 5.8 mm design) at each of its
//! fidelities: the analytic closed forms (the field at a coplanar gap's centre across the film's
//! slab mode; the line by conformal mapping), the 2D model (the static solver's field and the
//! full-vector mode's perturbation for Vπ·L; the quasi-TEM line of the static solver for the
//! response) with its grids gated against ones twice as fine, and the measured devices. The
//! paper doesn't give the walls' angle or the grounds' width: the device takes 70° and 100 µm,
//! as P6's and P7's cases do. The bandwidths are referred to 1 GHz, as the paper's.
//!
//! Vπ·L is a 2D number, a straight section's per unit length; the response is a uniform line's,
//! the Y-branches ideal. Checked: the 2D Vπ·L against the measured 2.3 V·cm (the tolerance of
//! P6's case, 0.15 V·cm), the response at 250 GHz over 1 GHz against the measured RF Vπ's (20%,
//! P7's), the analytic Vπ·L against the 2D (its stated 15%). The 3 dB bandwidths are printed and
//! not checked: the device's 2D line gives about half the measured one at 5.8 mm and a third at
//! 10.8 mm, P7's open finding (#418), and the full-wave mode doesn't close it.
//!
//! ```sh
//! cargo run --release --example tfln_zhang
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::circuit::Fidelity;
use photonoxide::convergence::Study;
use photonoxide::device::{Built, Device, TflnModulator, Values};

/// The figure `symbol` of `b`.
fn get(b: &Built, symbol: &str) -> f64 {
    b.figure(symbol).map_or(f64::NAN, |g| g.value)
}

/// A tolerance taken from a computed value, to three decimals: what the output prints the same
/// on every system.
fn printed(tolerance: f64) -> f64 {
    (tolerance * 1e3).round() / 1e3
}

/// The tolerances: Vπ·L against the measurement at DC (V·cm); the response at 250 GHz (20 %),
/// the 2D grid's and the analytic model's against the 2D, relative.
const VPI_L: f64 = 0.15;
const RESPONSE: f64 = 0.2;
const GRID: f64 = 0.01;
const ANALYTIC: f64 = 0.15;

pub fn main() -> photonoxide::Result<ExitCode> {
    let d = TflnModulator::new();
    // each design's length (mm), its measured RF V_π at 250 GHz (V) and 3 dB bandwidth (GHz)
    let designs = [(5.8, 8.3, 170.0), (10.8, 7.3, 100.0)];
    let lam = photonoxide::units::Wavelength::um(1.55)?;
    let mut setup = common::Setup::new();
    for (mm, _, _) in designs {
        setup.part(&format!("the {mm} mm modulator"));
        setup.device(&d, &Values::new(&d, &[("length", mm * 1e3)])?, lam);
    }
    setup.part("both");
    setup.wavelength(lam);
    setup.item(
        "grid",
        "the 2D model's own grids, and grids twice as fine (the gate); the analytic model's \
         closed forms",
    );
    setup.item(
        "solver",
        "the static solver's field and the full-vector mode's perturbation, the quasi-TEM line \
         of the static solver (2D); the field at a coplanar gap's centre across the film's slab \
         mode, the line by conformal mapping (analytic)",
    );
    setup.item(
        "tolerance",
        &format!(
            "{VPI_L} V·cm on Vπ·L; {} % on the response at 250 GHz; the 2D grid within {} %; the \
             analytic within {} % of the 2D",
            common::number(100.0 * RESPONSE),
            common::number(100.0 * GRID),
            common::number(100.0 * ANALYTIC)
        ),
    );
    setup.print();
    let mut checks = common::Checks::default();
    println!(
        "Zhang et al. 2022's thin-film lithium niobate modulators through photonoxide's TFLN \
         Mach-Zehnder modulator device, at 1550 nm"
    );
    let mut analytic_vpi_l = 0.0;
    let mut own_vpi_l = 0.0;
    for (mm, vpi_250, f3_measured) in designs {
        println!();
        println!("{mm} mm long");
        let v = Values::new(&d, &[("length", mm * 1e3)])?;
        for f in [Fidelity::Analytic, Fidelity::TwoD, Fidelity::Measured] {
            let b = d.component(&v, f)?;
            let line = if f == Fidelity::Measured {
                String::new()
            } else {
                format!(
                    "   n_rf {:.3}  Z0 {:5.2} ohm (1 GHz)",
                    get(&b, "n_rf"),
                    get(&b, "Z0")
                )
            };
            println!(
                "  {:<9} VpiL {:.3} V cm  Vpi {:.3} V  f_3dB {:6.1} GHz  f_6dB {:6.1} GHz{line}",
                f.to_string(),
                get(&b, "VpiL"),
                get(&b, "Vpi"),
                get(&b, "f_3dB"),
                get(&b, "f_6dB"),
            );
            println!(
                "            ({})",
                b.figure("VpiL").expect("every model gives VpiL").grid
            );
            if mm == 5.8 {
                match f {
                    Fidelity::Analytic => analytic_vpi_l = get(&b, "VpiL"),
                    Fidelity::TwoD => own_vpi_l = get(&b, "VpiL"),
                    _ => {}
                }
            }
        }
        // the 2D model on its own grids and twice as fine, at 1 and 250 GHz
        let at = |r: f64, ghz: f64| -> photonoxide::Result<Built> {
            d.clone()
                .with_refinement(r)?
                .component(&v.with(&d, "frequency", ghz)?, Fidelity::TwoD)
        };
        let (c1, c250, f1, f250) = (
            at(1.0, 1.0)?,
            at(1.0, 250.0)?,
            at(2.0, 1.0)?,
            at(2.0, 250.0)?,
        );
        let ratio = |a: &Built, b: &Built| 10f64.powf((get(b, "m_dB") - get(a, "m_dB")) / 20.0);
        let (coarse, fine) = (ratio(&c1, &c250), ratio(&f1, &f250));
        println!(
            "  2D at 250 GHz: response {:.2} dB from 1 GHz, RF Vpi {:.2} V (measured {vpi_250} V)",
            20.0 * fine.log10(),
            get(&f250, "Vpi_f")
        );
        println!(
            "  2D 3 dB bandwidth {:.1} GHz against the measured {f3_measured}: {:.2} of it",
            get(&f1, "f_3dB"),
            get(&f1, "f_3dB") / f3_measured
        );
        let c = Study::new("m(250 GHz)/m(1 GHz)")
            .on(0.02, coarse)?
            .on(0.01, fine)?
            .check(printed(GRID * fine))?;
        checks.converged(
            &format!("{mm} mm: 2D response at 250 GHz, grids 1 and 1/2"),
            &c,
        );
        let theirs = 2.67 / (0.1 * mm) / vpi_250;
        checks.compare(
            &format!("{mm} mm: 2D m(250)/m(1) (measured)"),
            fine,
            (theirs * 1e4).round() / 1e4,
            printed(RESPONSE * theirs),
        );
        if mm == 5.8 {
            let (a, b) = (get(&c1, "VpiL"), get(&f1, "VpiL"));
            let c = Study::new("VpiL (V cm)")
                .on(0.02, a)?
                .on(0.01, b)?
                .check(printed(GRID * b))?;
            checks.converged("2D VpiL on grids 1 and 1/2, V cm", &c);
            checks.compare("2D VpiL, V cm (measured at DC)", b, 2.3, VPI_L);
        }
    }
    checks.against(
        "analytic VpiL, V cm",
        analytic_vpi_l,
        "2D",
        own_vpi_l,
        printed(ANALYTIC * own_vpi_l),
    );
    Ok(checks.finish())
}
