//! Validation against measured circuits ([`crate::validation`]): Dwivedi et al. 2015's
//! Mach-Zehnder interferometers of silicon wires, predicted from the wires' measured
//! cross-sections and read the way the paper reads the measured spectra.
//!
//! S. Dwivedi et al., J. Lightwave Technol. 33, 4471 (2015),
//! [doi:10.1109/JLT.2015.2476603](https://doi.org/10.1109/JLT.2015.2476603): interferometers of
//! order m = 15 (n_eff from n_eff L = m λ, Eq. 3) and M = 110 (n_g from the free spectral range,
//! Eq. 8), designed for 450, 600 and 800 nm × 215 nm wires to resonate at 1550 nm; the wires
//! measured by cross-section SEM at 470, 602 and 805 nm × 211 nm; n_eff and n_g at 1550 nm in
//! their Table I. The `mzi_dwivedi` example prints the same with the tolerances' derivation.

use std::sync::{Arc, OnceLock};

use rayon::prelude::*;

use super::{Dispersion, Waveguide, YBranch, mzi_y};
use crate::circuit::{Circuit, Component};
use crate::material;
use crate::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity};
use crate::units::Wavelength;
use crate::validation::Outcome;

const OK: &str = "a valid model";

/// Table I's wavelength, µm.
const CENTRE: f64 = 1.55;

/// The quarter x, y ≥ 0 of a `width` × `thickness` µm silicon wire (Li 1980) in silica
/// (Malitson 1965), on a uniform grid of about 10 nm whose lines fall on the core's faces, with
/// 1 µm of oxide beyond them, mirror walls keeping the TE-like modes.
fn quarter(width: f64, thickness: f64, wavelength: Wavelength) -> crate::Result<CrossSection> {
    let (si, ox) = (
        material::silicon().permittivity(wavelength)?,
        material::silica().permittivity(wavelength)?,
    );
    let (a, b) = (width / 2.0, thickness / 2.0);
    let (dx, dy) = (a / (a / 0.01).round(), b / (b / 0.01).round());
    let (nx, ny) = (
        ((a + 1.0) / dx).round() as usize,
        ((b + 1.0) / dy).round() as usize,
    );
    CrossSection::uniform(
        (0.0, nx as f64 * dx, nx),
        (0.0, ny as f64 * dy, ny),
        |x, y| Permittivity::isotropic(if x < a && y < b { si } else { ox }),
    )?
    .with_boundaries(Boundaries {
        west: Boundary::ElectricWall,
        south: Boundary::MagneticWall,
        ..Boundaries::default()
    })
}

/// The wire's TE-like mode about 1550 nm by Hadley's equations, a model over 1.52 to 1.58 µm.
fn wire(width: f64, thickness: f64) -> Dispersion {
    Dispersion::from_hadley(
        |w| quarter(width, thickness, w),
        Wavelength::um(CENTRE).expect(OK),
        0.015,
        None,
    )
    .expect("the wire's modes")
    .0
}

/// A Mach-Zehnder interferometer of two ideal splitters whose arms of `wire` differ by
/// `delta` µm.
fn interferometer(wire: Dispersion, delta: f64) -> Circuit {
    let guide: Arc<dyn Component> = Arc::new(Waveguide::new(wire));
    let split: Arc<dyn Component> = Arc::new(YBranch::new());
    let mut n = mzi_y(split.clone(), split, guide.clone(), guide).expect(OK);
    n.set("lower", "length", 50.0).expect(OK);
    n.set("upper", "length", 50.0 + delta).expect(OK);
    n.compile().expect(OK)
}

fn power(c: &Circuit, l: f64) -> f64 {
    c.s_matrix(Wavelength::um(l).expect(OK))
        .expect(OK)
        .power(1, 0)
}

/// The transmission peaks between `a` and `b` µm: sampled every 0.05 nm, each refined by golden
/// section to 1e-10 µm.
fn peaks(c: &Circuit, a: f64, b: f64) -> Vec<f64> {
    let step = 5e-5;
    let n = ((b - a) / step).round() as usize;
    let t: Vec<f64> = (0..=n).map(|i| power(c, a + step * i as f64)).collect();
    let g = (5f64.sqrt() - 1.0) / 2.0;
    (1..n)
        .filter(|&i| t[i] > t[i - 1] && t[i] >= t[i + 1])
        .map(|i| {
            let (mut lo, mut hi) = (a + step * (i - 1) as f64, a + step * (i + 1) as f64);
            while hi - lo > 1e-10 {
                let (x1, x2) = (hi - g * (hi - lo), lo + g * (hi - lo));
                if power(c, x1) > power(c, x2) {
                    hi = x2;
                } else {
                    lo = x1;
                }
            }
            (lo + hi) / 2.0
        })
        .collect()
}

/// n_eff and n_g at 1550 nm of a `width` µm wire 211 nm thick, as the paper extracts them from
/// its interferometers' spectra, the path differences designed for the drawn `design` µm wire
/// 215 nm thick: n_g between neighbouring peaks of the M = 110 interferometer,
/// λ₁λ₂/((λ₂ − λ₁) ΔL), fitted by a line in λ (Eq. 10); n_eff(λ_r) = 15 λ_r/ΔL at the
/// m = 15 interferometer's peak, carried to 1550 nm by d(n/λ)/dλ = −n_g/λ².
pub(crate) fn dwivedi_extract(design: f64, width: f64) -> (f64, f64) {
    let drawn = wire(design, 0.215);
    let small = 15.0 * CENTRE / drawn.effective_index;
    let big = 110.0 * CENTRE / drawn.effective_index;
    let fab = wire(width, 0.211);
    let p = peaks(&interferometer(fab, big), 1.53, 1.57);
    let pairs: Vec<(f64, f64)> = p
        .windows(2)
        .map(|w| ((w[0] + w[1]) / 2.0, w[0] * w[1] / ((w[1] - w[0]) * big)))
        .collect();
    let k = pairs.len() as f64;
    let mx = pairs.iter().map(|q| q.0).sum::<f64>() / k;
    let my = pairs.iter().map(|q| q.1).sum::<f64>() / k;
    let slope = pairs.iter().map(|q| (q.0 - mx) * (q.1 - my)).sum::<f64>()
        / pairs.iter().map(|q| (q.0 - mx).powi(2)).sum::<f64>();
    let lr = peaks(&interferometer(fab, small), 1.52, 1.58)
        .into_iter()
        .min_by(|a, b| (a - CENTRE).abs().total_cmp(&(b - CENTRE).abs()))
        .expect("a peak in the band");
    let intercept = my - slope * mx;
    let integral = intercept * (1.0 / CENTRE - 1.0 / lr) + slope * (lr / CENTRE).ln();
    (
        CENTRE * (15.0 / small + integral),
        my + slope * (CENTRE - mx),
    )
}

/// The three wires' extracted (n_eff, n_g): 470, 602 and 805 nm.
fn extracted() -> &'static [(f64, f64)] {
    static RESULTS: OnceLock<Vec<(f64, f64)>> = OnceLock::new();
    RESULTS.get_or_init(|| {
        [(0.45, 0.470), (0.60, 0.602), (0.80, 0.805)]
            .par_iter()
            .map(|&(design, width)| dwivedi_extract(design, width))
            .collect()
    })
}

/// Table I: (n_eff, its uncertainty), (n_g, its uncertainty), for 470, 602 and 805 nm.
const TABLE: [((f64, f64), (f64, f64)); 3] = [
    ((2.355, 0.002), (4.2739, 0.0042)),
    ((2.534, 0.0035), (4.0453, 0.0045)),
    ((2.67, 0.004), (3.8902, 0.005)),
];

/// The tolerances, (n_eff, n_g): the paper's Eq. 5 with Fig. 1's ±20 nm of width and ±5 nm of
/// thickness, by the solver's derivatives (the `mzi_dwivedi` example computes them), plus
/// Table I's uncertainty, rounded up to the third decimal.
const TOLERANCE: [(f64, f64); 3] = [(0.061, 0.049), (0.042, 0.032), (0.031, 0.021)];

fn outcome(wire: usize, group: bool) -> Outcome {
    let (n_eff, n_g) = extracted()[wire];
    let (measured, expected, tolerance) = if group {
        (n_g, TABLE[wire].1.0, TOLERANCE[wire].1)
    } else {
        (n_eff, TABLE[wire].0.0, TOLERANCE[wire].0)
    };
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

pub(crate) fn dwivedi_neff_470() -> Outcome {
    outcome(0, false)
}

pub(crate) fn dwivedi_ng_470() -> Outcome {
    outcome(0, true)
}

pub(crate) fn dwivedi_neff_602() -> Outcome {
    outcome(1, false)
}

pub(crate) fn dwivedi_ng_602() -> Outcome {
    outcome(1, true)
}

pub(crate) fn dwivedi_neff_805() -> Outcome {
    outcome(2, false)
}

pub(crate) fn dwivedi_ng_805() -> Outcome {
    outcome(2, true)
}
