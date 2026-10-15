//! Zhang et al.'s Y-junction through the Y-branch device: its outline against their Table 1,
//! and its 2D loss against their 2D design.
//!
//! Y. Zhang, S. Yang, A. E.-J. Lim, G.-Q. Lo, C. Galland, T. Baehr-Jones, M. Hochberg, "A
//! compact and low loss Y-junction for submicron silicon waveguide", Opt. Express 21, 1310
//! (2013), [doi:10.1364/OE.21.001310](https://doi.org/10.1364/OE.21.001310), Section 2.1, Fig.
//! 1(a) and Table 1: a taper 2 µm long from a 500 × 220 nm silicon strip in oxide to two 500 nm
//! arms 200 nm apart, its width w₁ … w₁₃ at 13 points of equal spacing, "defined by spline
//! interpolation" of them (the spline's ends unstated: not-a-knot here), optimized at 1550 nm in
//! 2D FDTD until "one solution with sub-0.2 dB insertion loss emerged"; 0.13 dB in 3D FDTD at
//! λ/34; measured 0.28 ± 0.02 dB.
//!
//! Here photonoxide's Y-branch device (`device::StripYBranch`) at its defaults, which are
//! Zhang et al.'s widths, its outline the expression language's `spline` through them, with
//! sine S-bend arms 5 µm long to ports 2 µm apart (sharper arms shed more: 3 µm long to ports 1.5 µm
//! apart, 0.29 dB):
//!
//! - the drawn outline's width at the 13 points, measured through its inside test, against
//!   Table 1 (exactly, to round-off);
//! - its 2D model, 2D FDFD by the effective index method on 20 and 10 nm grids at 1550 nm: the
//!   insertion loss, −10 log₁₀ of the power in both arms' fundamental modes, against their 2D
//!   design's "sub-0.2 dB". A 2D estimate, labelled so: not the device's performance, which is
//!   the 3D model's.
//!
//! ```sh
//! cargo run --release --example y_junction_zhang
//! ```

mod common;

use std::process::ExitCode;

use photonoxide::circuit::Fidelity;
use photonoxide::device::{Device, FdfdGrid, StripYBranch, Values};
use photonoxide::geometry::Point;

/// Zhang et al.'s Table 1, µm.
const TABLE: [f64; 13] = [
    0.5, 0.5, 0.6, 0.7, 0.9, 1.26, 1.4, 1.4, 1.4, 1.4, 1.31, 1.2, 1.2,
];

/// Twice the y at which `inside` stops holding at x, by bisection to the last bit.
fn width(inside: &dyn Fn(Point) -> bool, x: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..200 {
        let m = 0.5 * (lo + hi);
        if m == lo || m == hi {
            break;
        }
        if inside(Point::um(x, m)) {
            lo = m;
        } else {
            hi = m;
        }
    }
    lo + hi
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let device = StripYBranch::new();
    let values = Values::defaults(&device);
    let geometry = device.geometry(&values)?;
    let stem = geometry.structure.shapes("Si")[0].clone();
    let inside = |p: Point| stem.contains(p);
    println!(
        "Zhang et al.'s Y-junction through the Y-branch device: 220 nm SOI, a 2 um taper, arms 5 \
         um long to ports 2 um apart"
    );
    let mut checks = common::Checks::default();
    let mut worst: f64 = 0.0;
    for (k, &w) in TABLE.iter().enumerate() {
        // at the ends, just inside
        let x = (2.0 * k as f64 / 12.0).clamp(1e-12, 2.0 - 1e-12);
        worst = worst.max((width(&inside, x) - w).abs());
    }
    println!("the drawn outline at its 13 points against Table 1 (largest difference):");
    checks.compare("outline against Table 1 (um)", worst, 0.0, 1e-12);
    println!("its 2D model, 2D FDFD by the effective index method at 1.55 um (2D estimates):");
    for step in [0.02, 0.01] {
        let d = device
            .clone()
            .with_grid_2d(FdfdGrid::new(step).with_samples(1));
        let b = d.component(&values, Fidelity::TwoD)?;
        let figure = |s: &str| b.figure(s).map_or(f64::NAN, |f| f.value);
        let (upper, lower) = (figure("P_o2"), figure("P_o3"));
        let loss = -10.0 * (upper + lower).log10();
        println!(
            "  {:.0} nm ({}): arms {upper:.5} and {lower:.5}, insertion loss {loss:.3} dB",
            step * 1e3,
            b.figures[0].grid
        );
        checks.at_most(
            &format!("insertion loss, 2D, {:.0} nm (dB)", step * 1e3),
            loss,
            0.2,
        );
    }
    Ok(checks.finish())
}
