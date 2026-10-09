//! How a PML's reflection falls as it thickens, for σ graded as (x/L)^d.
//!
//! A. F. Oskooi, D. Roundy, M. Ibanescu, P. Bermel, J. D. Joannopoulos, S. G. Johnson, "Meep: A
//! flexible free-software package for electromagnetic simulations by the FDTD method", Comput.
//! Phys. Commun. 181, 687 (2010),
//! [doi:10.1016/j.cpc.2009.11.008](https://doi.org/10.1016/j.cpc.2009.11.008), Section 4.2 and
//! Fig. 8 (right):
//!
//! - **The problem:** a point source of E_z in 2D vacuum, in a square cell with a Cartesian PML
//!   L wavelengths thick on every side, at 20 pixels a wavelength.
//! - **The measurement:** the "field convergence", |E^(L+1) − E^L|², the change of E_z at a
//!   point when the PML is one wavelength thicker: "a simple proxy for the PML reflections".
//! - **The result:** with σ turned on as (x/L)^d "for a given round-trip reflection", the
//!   reflections "decrease as 1/L^(2d+2) and field-convergence factors ... as 1/L^(2d+4)":
//!   "precisely these decay rates are observed in Fig. 8", lines of 1/L⁶, 1/L⁸ and 1/L¹⁰ for
//!   d = 1, 2 and 3 (the rates are derived in their ref. 45, Oskooi & Johnson 2008).
//!
//! Settings the paper leaves open, set here:
//!
//! - **The cell:** 4 wavelengths across inside the PML, the source at its centre, E_z recorded
//!   a wavelength from it along x (1 µm, so 50 nm cells).
//! - **The round-trip reflection:** R = 1e-15, σ's maximum (d + 1)(−ln R)/(2L) (κ = 1, α = 0:
//!   photonoxide's CPML is then the PML in discrete time).
//! - **The field:** at the source's frequency, from a run's transform divided by the pulse's
//!   (a Gaussian 0.2 c/µm wide, so nothing of it is at zero frequency, where a PML doesn't
//!   absorb), run until |E_z|² at the point has fallen to 1e-26 of its peak, at Meep's time step,
//!   Δt = Δx/2.
//!
//! The rate is the slope of log |ΔE|² against log L between consecutive differences, each
//! taken at its midpoint, L + ½ (ΔE between L and L + 1 is the derivative there). Each
//! order's rate at the thickest PMLs is compared with 2d + 4, within 0.1: Fig. 8's lines are
//! drawn, not fitted, and the rate is reached from above as L grows.
//!
//! ```sh
//! cargo run --release --example pml_oskooi
//! ```

mod common;

use std::process::ExitCode;

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{Axis, Grid3d};
use photonoxide::fdtd::{Boundaries, Cpml, Field, Simulation, Source, Waveform};
use photonoxide::units::Frequency;

/// Cells a wavelength (λ = 1 µm).
const RESOLUTION: usize = 20;
/// The cell's side inside the PMLs, in wavelengths.
const INTERIOR: usize = 4;
/// The PMLs' round-trip reflection.
const REFLECTION: f64 = 1e-15;

/// E_z a wavelength from the source, at the source's frequency, per unit source spectrum, with
/// a PML `thickness` wavelengths thick graded as (x/L)^`order`.
fn field(thickness: usize, order: f64) -> c64 {
    let h = 1.0 / RESOLUTION as f64;
    let pml = thickness * RESOLUTION;
    let n = INTERIOR * RESOLUTION + 2 * pml;
    let grid = Grid3d {
        nx: n,
        ny: n,
        nz: 1,
        dx: h,
        dy: h,
        dz: h,
        x0: -(n as f64) * h / 2.0,
        y0: -(n as f64) * h / 2.0,
        z0: 0.0,
    };
    let mut boundaries = Boundaries::cpml_2d(pml);
    boundaries.cpml = Cpml {
        reflection: REFLECTION,
        order,
        ..Cpml::default()
    };
    // Meep's time step, Δt = Δx/2: a Courant number of 1/√2 in 2D
    let mut s = Simulation::new(
        grid,
        |_, _, _| 1.0,
        boundaries,
        std::f64::consts::FRAC_1_SQRT_2,
    )
    .expect("the cell");
    let frequency = Frequency::natural(1.0).expect("1 c/µm");
    let pulse = Waveform::pulse(frequency, 0.2).expect("the pulse");
    let centre = n / 2;
    s.add_source(Source {
        field: Field::E,
        component: Axis::Z,
        at: (centre, centre, 0),
        waveform: pulse,
    })
    .expect("the source");
    let at = (centre + RESOLUTION, centre, 0);
    let dft = s.add_dft(at, at, &[frequency]).expect("the transform");
    let probe = s.add_probe(Field::E, Axis::Z, at).expect("the probe");
    let decayed = s
        .run_until_decayed(&[probe], 1e-26, 5.0, 400.0)
        .expect("the run");
    assert!(decayed, "the field didn't decay");
    let spectrum = pulse.spectrum(Field::E, s.dt(), s.steps(), frequency);
    s.dft(dft)
        .value(Field::E, Axis::Z, at, 0)
        .expect("the transform's value")
        / spectrum
}

/// A value rounded to `digits` decimals, so that what is printed doesn't change with the last
/// bits of a platform's math library.
fn rounded(v: f64, digits: i32) -> f64 {
    let scale = 10f64.powi(digits);
    (v * scale).round() / scale
}

pub fn main() -> ExitCode {
    println!("Oskooi et al.'s Fig. 8: a point source of E_z in 2D vacuum, 20 cells a wavelength");
    println!(
        "{INTERIOR} x {INTERIOR} wavelengths inside PMLs L wavelengths thick, R = {REFLECTION:e}; E_z a wavelength from the source"
    );
    let thickest = 8;
    let mut checks = common::Checks::default();
    for d in [1u32, 2, 3] {
        let fields: Vec<c64> = (1..=thickest + 1).map(|l| field(l, f64::from(d))).collect();
        let size = fields[0].norm_sqr();
        let changes: Vec<f64> = fields
            .windows(2)
            .map(|w| (w[1] - w[0]).norm_sqr() / size)
            .collect();
        println!("sigma ~ (x/L)^{d}:");
        let mut rate = 0.0;
        for (k, change) in changes.iter().enumerate() {
            let l = k + 1;
            if k == 0 {
                println!("  L = {l:>2}: |E(L+1) - E(L)|^2 / |E|^2 = {change:.2e}");
                continue;
            }
            // the slope between the midpoints L - 1/2 and L + 1/2
            rate = (changes[k - 1] / change).ln() / ((l as f64 + 0.5) / (l as f64 - 0.5)).ln();
            println!(
                "  L = {l:>2}: |E(L+1) - E(L)|^2 / |E|^2 = {change:.2e}, local rate {:.2}",
                rounded(rate, 2)
            );
        }
        checks.compare(
            &format!("rate at L = {thickest}, d = {d}"),
            rounded(rate, 2),
            f64::from(2 * d + 4),
            0.1,
        );
    }
    checks.finish()
}
