//! The convolutional PML's reflection error next to a thin conducting plate in soil.
//!
//! J. A. Roden, S. D. Gedney, "Convolution PML (CPML): An efficient FDTD implementation of the
//! CFS-PML for arbitrary media", Microw. Opt. Technol. Lett. 27, 334 (2000),
//! [doi:10.1002/1098-2760(20001205)27:5<334::AID-MOP14>3.0.CO;2-A](https://doi.org/10.1002/1098-2760(20001205)27:5<334::AID-MOP14>3.0.CO;2-A),
//! Section V and Figs. 2–4:
//!
//! - **The problem:** a 100 × 25 mm perfectly conducting plate, of no thickness, in soil
//!   (ε_r = 7.73, σ = 0.273 S/m), on a grid of 1 mm cells. A CPML of 10 cells sits only 3 cells
//!   from the plate on every side, a 126 × 51 × 26 cell lattice.
//! - **The source:** a vertical current element just above one corner, a differentiated
//!   Gaussian "with a 6 GHz bandwidth".
//! - **The measurement:** the error of the electric field near the opposite corner, against the
//!   same lattice extended 75 cells out on every side (276 × 201 × 176), as
//!   20 log₁₀(|E − E_ref| / max |E_ref|), its largest value over 2000 steps.
//! - **The results:**
//!   - with the PML's grading of order m = 4, σ_max = 0.7 σ_opt and κ_max = 11 but no frequency
//!     shift (α = 0, the traditional PML), "the maximum error ... is on the order of −48 dB";
//!   - with α = 0.05 S/m, σ_max = 1.1 σ_opt and κ_max = 7, "−67 dB", where
//!     σ_opt = (m + 1)/(150π √ε_r Δ) (their Eq. 15).
//!
//! Settings the paper leaves open:
//!
//! - **The time step:** the Courant number is 0.99.
//! - **The field compared:** E_z, the plate's normal component, one cell above the opposite
//!   corner.
//! - **The pulse's width,** which the errors depend on strongly: a differentiated Gaussian of
//!   width τ = 1/(π · 6 GHz) gives −51.5 and −76.1 dB, and half as wide −38.2 and −55.1 dB. It is
//!   set here, as τ = 0.72/(π · 6 GHz) = 38 ps, so that the traditional PML's error is the
//!   paper's −48 dB. The CFS-PML's error is then a prediction.
//! - **The reference:** padded by 40 cells instead of 75. It gives the same errors to 0.12 dB
//!   (measured against the 75-cell one) at a third of the cost.
//!
//! The paper's two numbers are read off contour plots ("on the order of"), so they are compared
//! within 5 dB, and the CFS-PML's gain over the traditional one within 5 dB of the paper's 19 dB
//! (−48 against −67, "almost a 20 dB improvement").
//!
//! ```sh
//! cargo run --release --example cpml_roden_gedney
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::fdfd::{Axis, Edges, Grid3d};
use photonoxide::fdtd::{Boundaries, Cpml, Field, Simulation, Source, Waveform};

/// One millimetre, in µm.
const MM: f64 = 1000.0;
/// η₀ in ohms: a conductivity in S/m times η₀ times 1e-6 m/µm is photonoxide's 1/µm.
const ETA0: f64 = 376.730_313_668;
/// The speed of light, m/s.
const C: f64 = 299_792_458.0;

/// The soil's relative permittivity and conductivity (1/µm).
const EPS: f64 = 7.73;
const SIGMA: f64 = 0.273 * ETA0 * 1e-6;

/// E_z one cell above the plate's corner opposite the source, every step for 2000 steps, on a
/// lattice padded by `pad` cells beyond Roden and Gedney's on every side, terminated by a CPML of
/// 10 cells with `cpml`.
fn run(pad: usize, cpml: Cpml) -> Vec<f64> {
    let (pml, gap) = (10, 3);
    let edge = pml + gap + pad;
    let n = [100 + 2 * edge, 25 + 2 * edge, 2 * edge];
    let grid = Grid3d {
        nx: n[0],
        ny: n[1],
        nz: n[2],
        dx: MM,
        dy: MM,
        dz: MM,
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
    };
    let walls = Edges::Pml {
        low: pml,
        high: pml,
    };
    let boundaries = Boundaries {
        x: walls,
        y: walls,
        z: walls,
        cpml,
    };
    let mut s = Simulation::new(grid, |_, _, _| EPS, boundaries, 0.99)
        .expect("the lattice")
        .with_conductivity(|_, _, _| SIGMA)
        .expect("the soil");
    // the plate on the middle node along z, its 100 x 25 cells
    let plate = n[2] / 2;
    s.plate(Axis::Z, plate, edge..edge + 100, edge..edge + 25);
    // the differentiated Gaussian: τ = 1/(π · 6 GHz), in µm/c
    let tau = 0.72 / (std::f64::consts::PI * 6e9) * C * 1e6;
    s.add_source(Source {
        field: Field::E,
        component: Axis::Z,
        at: (edge, edge, plate),
        waveform: Waveform::DifferentiatedGaussian {
            width: tau,
            delay: 4.0 * tau,
        },
    })
    .expect("the source");
    let probe = s
        .add_probe(Field::E, Axis::Z, (edge + 100, edge + 25, plate))
        .expect("the probe");
    s.run(2000);
    s.probe(probe).to_vec()
}

/// The largest error over the run, in dB of the reference's largest value (their Eq. 14).
fn error_db(field: &[f64], reference: &[f64]) -> f64 {
    let largest = reference.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let worst = field
        .iter()
        .zip(reference)
        .fold(0.0f64, |m, (a, b)| m.max((a - b).abs()));
    20.0 * (worst / largest).log10()
}

pub fn main() -> ExitCode {
    let m = 4.0;
    let optimal = Cpml::sigma_optimal(m, EPS, MM);
    let traditional = Cpml {
        sigma: Some(0.7 * optimal),
        order: m,
        kappa: 11.0,
        alpha: 0.0,
        ..Cpml::default()
    };
    let shifted = Cpml {
        sigma: Some(1.1 * optimal),
        order: m,
        kappa: 7.0,
        alpha: 0.05 * ETA0 * 1e-6,
        ..Cpml::default()
    };
    println!("Roden and Gedney's plate in soil: 1 mm cells, CPML of 10 cells 3 cells from it");
    println!("126 x 51 x 26 cells; reference 206 x 131 x 106 (40 more on every side), 2000 steps");
    println!(
        "sigma_opt = {:.4} S/m (their Eq. 15)",
        optimal / (ETA0 * 1e-6)
    );
    let pad = 40;
    let reference = run(pad, shifted);
    let a = error_db(&run(0, traditional), &reference);
    let b = error_db(&run(0, shifted), &reference);
    let mut checks = common::Checks::default();
    checks.compare("alpha = 0: largest error, dB", a, -48.0, 5.0);
    checks.compare("alpha = 0.05 S/m: largest error, dB", b, -67.0, 5.0);
    checks.compare("the CFS-PML's gain, dB", a - b, 19.0, 5.0);
    checks.finish()
}
