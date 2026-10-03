//! Measured Mach-Zehnder interferometers: the effective and group index of silicon wires,
//! predicted from their measured cross-sections and extracted from the circuits' spectra.
//!
//! S. Dwivedi, A. Ruocco, M. Vanslembrouck, T. Spuesens, P. Bienstman, P. Dumon,
//! T. Van Vaerenbergh, W. Bogaerts, "Experimental extraction of effective refractive index and
//! thermo-optic coefficients of silicon-on-insulator waveguides using interferometers",
//! J. Lightwave Technol. 33, 4471 (2015),
//! [doi:10.1109/JLT.2015.2476603](https://doi.org/10.1109/JLT.2015.2476603) (the accepted
//! manuscript is open at biblio.ugent.be/publication/7065396). On imec's 200 mm line they made
//! Mach-Zehnder interferometers of 1 × 2 MMIs and silicon wires designed 450, 600 and 800 nm
//! wide and 215 nm thick, in oxide (2 µm below, 1.25 µm above), three per width: two of low
//! order (m = 15 and 16, designed to resonate at 1550 nm), whose resonance gives the effective
//! index unambiguously (n_eff L = m λ, their Eq. 3), and one of high order (M = 110), whose free
//! spectral range gives the group index (FSR = λ²/(n_g L), Eq. 8). Cross-section SEM puts the
//! wires at 470 ± 4, 602 (Table I; 604 in the text) and 805 nm wide and 211 ± 1 nm thick.
//! Their Table I, at 1550 nm:
//!
//! | Width | n_eff | n_g |
//! |---|---|---|
//! | 470 nm | 2.355 ± 0.002 | 4.2739 ± 0.0042 |
//! | 602 nm | 2.534 ± 0.0035 | 4.0453 ± 0.0045 |
//! | 805 nm | 2.67 ± 0.004 | 3.8902 ± 0.005 |
//!
//! Here each wire is the SEM's rectangle (the trapezoid's sidewall angle isn't printed) in
//! photonoxide's own silicon (Li 1980) and fused silica (Malitson 1965), whose 1.4440 at
//! 1550 nm is the paper's; its TE-like mode by Hadley's high-accuracy equations on a quarter
//! domain, about 10 nm grids, at 1.52 to 1.58 µm, a [`Waveguide`]. The interferometers are
//! designed as the paper's were: their path differences ΔL = m λ/n_eff from the designed
//! 215 nm thick wire's index. Each is a netlist of two ideal splitters and two arms, and its
//! spectrum is read the way the paper reads the measured one: the M = 110 interferometer's
//! peaks give n_g = λ₁λ₂/((λ₂ − λ₁) ΔL) between each pair, fitted by a line in λ (Eq. 10), and
//! the m = 15 one's peak λ_r gives n_eff(λ_r) = 15 λ_r/ΔL, carried to 1550 nm by
//! d(n/λ)/dλ = −n_g/λ².
//!
//! The tolerance is the paper's own estimate of what fabrication moves (their Eq. 5 and
//! Fig. 1): ±20 nm in width and ±5 nm in thickness, through the solver's derivatives, plus
//! Table I's uncertainty. The SEM's own ±4 and ±1 nm would allow the 470 nm wire's n_g only
//! 0.013, and it is 0.041 off: the narrowest wire feels its sidewalls most, and the model's
//! rectangle leaves out their slope and roughness; the paper's own simulations of the SEM
//! geometry miss too (its Fig. 7), which it puts down to local variations and the wires'
//! geometrical non-idealities. The others agree within 0.012.
//!
//! ```sh
//! cargo run --release --example mzi_dwivedi
//! ```

mod common;

use std::process::ExitCode;
use std::sync::Arc;

use photonoxide::circuit::components::{Dispersion, Waveguide, YBranch, mzi_y};
use photonoxide::circuit::{Circuit, Component};
use photonoxide::material::{silica, silicon};
use photonoxide::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity};
use photonoxide::units::Wavelength;

/// The wavelength of Table I, µm.
const CENTRE: f64 = 1.55;

/// The quarter x, y ≥ 0 of a `width` × `thickness` µm silicon wire in silica, centred at the
/// origin, on a uniform grid of about 10 nm whose lines fall on the core's faces, 1 µm of oxide
/// beyond them; an electric wall at x = 0 and a magnetic wall at y = 0 keep the TE-like modes.
fn quarter(
    width: f64,
    thickness: f64,
    wavelength: Wavelength,
) -> photonoxide::Result<CrossSection> {
    let (si, ox) = (
        silicon().permittivity(wavelength)?,
        silica().permittivity(wavelength)?,
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

/// The wire's mode about 1550 nm, a model over 1.52 to 1.58 µm, and its grid.
fn wire(width: f64, thickness: f64) -> photonoxide::Result<(Dispersion, String)> {
    let (d, _, grid) = Dispersion::from_hadley(
        |w| quarter(width, thickness, w),
        Wavelength::um(CENTRE)?,
        0.015,
        None,
    )?;
    Ok((d, grid))
}

/// A Mach-Zehnder interferometer of two ideal splitters whose arms of `wire` differ by
/// `delta` µm.
fn interferometer(wire: Dispersion, delta: f64) -> photonoxide::Result<Circuit> {
    let guide: Arc<dyn Component> = Arc::new(Waveguide::new(wire));
    let split: Arc<dyn Component> = Arc::new(YBranch::new());
    let mut n = mzi_y(split.clone(), split, guide.clone(), guide)?;
    n.set("lower", "length", 50.0)?;
    n.set("upper", "length", 50.0 + delta)?;
    n.compile()
}

/// The transmitted power at `l` µm.
fn power(c: &Circuit, l: f64) -> photonoxide::Result<f64> {
    Ok(c.s_matrix(Wavelength::um(l)?)?.power(1, 0))
}

/// The transmission peaks between `a` and `b` µm: sampled every 0.05 nm, each refined by golden
/// section to 1e-10 µm.
fn peaks(c: &Circuit, a: f64, b: f64) -> photonoxide::Result<Vec<f64>> {
    let step = 5e-5;
    let n = ((b - a) / step).round() as usize;
    let t: Vec<f64> = (0..=n)
        .map(|i| power(c, a + step * i as f64))
        .collect::<photonoxide::Result<_>>()?;
    let mut found = Vec::new();
    for i in 1..n {
        if t[i] > t[i - 1] && t[i] >= t[i + 1] {
            let (mut lo, mut hi) = (a + step * (i - 1) as f64, a + step * (i + 1) as f64);
            let g = (5f64.sqrt() - 1.0) / 2.0;
            while hi - lo > 1e-10 {
                let (x1, x2) = (hi - g * (hi - lo), lo + g * (hi - lo));
                if power(c, x1)? > power(c, x2)? {
                    hi = x2;
                } else {
                    lo = x1;
                }
            }
            found.push((lo + hi) / 2.0);
        }
    }
    Ok(found)
}

/// What the paper extracts, from the spectra of the high-order (`high`, path difference
/// `big` µm) and low-order (`low`, `small` µm, order 15) interferometers: n_eff and n_g at
/// 1550 nm.
fn extract(high: &Circuit, big: f64, low: &Circuit, small: f64) -> photonoxide::Result<(f64, f64)> {
    // n_g between neighbouring peaks of the high-order interferometer, fitted by a line in λ
    let p = peaks(high, 1.53, 1.57)?;
    let pairs: Vec<(f64, f64)> = p
        .windows(2)
        .map(|w| ((w[0] + w[1]) / 2.0, w[0] * w[1] / ((w[1] - w[0]) * big)))
        .collect();
    let k = pairs.len() as f64;
    let (mx, my) = (
        pairs.iter().map(|q| q.0).sum::<f64>() / k,
        pairs.iter().map(|q| q.1).sum::<f64>() / k,
    );
    let slope = pairs.iter().map(|q| (q.0 - mx) * (q.1 - my)).sum::<f64>()
        / pairs.iter().map(|q| (q.0 - mx).powi(2)).sum::<f64>();
    let ng = |l: f64| my + slope * (l - mx);
    // the low-order interferometer's peak, order 15: n(λ_r)/λ_r = 15/ΔL, and
    // n(λ₀)/λ₀ = n(λ_r)/λ_r + ∫ n_g/λ² dλ from λ₀ to λ_r
    let r = peaks(low, 1.52, 1.58)?;
    let lr = *r
        .iter()
        .min_by(|a, b| (*a - CENTRE).abs().total_cmp(&(*b - CENTRE).abs()))
        .expect("a peak in the band");
    let (intercept, s) = (my - slope * mx, slope);
    let integral = intercept * (1.0 / CENTRE - 1.0 / lr) + s * (lr / CENTRE).ln();
    Ok((CENTRE * (15.0 / small + integral), ng(CENTRE)))
}

/// One wire's numbers: its model at the SEM's cross-section and its grid, the designed wire's
/// index and the two path differences, what the interferometers give (n_eff, n_g), and what
/// ±20 nm of width and ±5 nm of thickness move (n_eff, n_g) by Eq. 5.
struct Wire {
    fab: Dispersion,
    grid: String,
    drawn: f64,
    delta: (f64, f64),
    extracted: (f64, f64),
    by_width: (f64, f64),
    by_thickness: (f64, f64),
}

/// The wire `width` µm wide, its interferometers designed for the drawn wire `design` µm wide.
fn study(design: f64, width: f64) -> photonoxide::Result<Wire> {
    // the interferometers, designed for the drawn 215 nm wire to resonate at 1550 nm
    let (drawn, _) = wire(design, 0.215)?;
    let (small, big) = (
        15.0 * CENTRE / drawn.effective_index,
        110.0 * CENTRE / drawn.effective_index,
    );
    let (fab, grid) = wire(width, 0.211)?;
    let extracted = extract(
        &interferometer(fab, big)?,
        big,
        &interferometer(fab, small)?,
        small,
    )?;
    // their Eq. 5: half of what the wire's n_eff and n_g change between the extremes
    let moved = |w: f64, t: f64| -> photonoxide::Result<(f64, f64)> {
        let (a, _) = wire(width + w, 0.211 + t)?;
        let (b, _) = wire(width - w, 0.211 - t)?;
        Ok((
            (a.effective_index - b.effective_index).abs() / 2.0,
            (a.group_index - b.group_index).abs() / 2.0,
        ))
    };
    Ok(Wire {
        fab,
        grid,
        drawn: drawn.effective_index,
        delta: (small, big),
        extracted,
        by_width: moved(0.02, 0.0)?,
        by_thickness: moved(0.0, 0.005)?,
    })
}

pub fn main() -> photonoxide::Result<ExitCode> {
    // designed width, SEM width, Table I's n_eff and n_g with their uncertainties
    let table = [
        (0.45, 0.470, (2.355, 0.002), (4.2739, 0.0042)),
        (0.60, 0.602, (2.534, 0.0035), (4.0453, 0.0045)),
        (0.80, 0.805, (2.67, 0.004), (3.8902, 0.005)),
    ];
    // the three wires at once
    let wires: Vec<photonoxide::Result<Wire>> = std::thread::scope(|scope| {
        let running: Vec<_> = table
            .iter()
            .map(|&(design, width, _, _)| scope.spawn(move || study(design, width)))
            .collect();
        running
            .into_iter()
            .map(|t| t.join().expect("the solve doesn't panic"))
            .collect()
    });
    let mut checks = common::Checks::default();
    println!(
        "silicon wires (Li 1980) in silica (Malitson 1965), Hadley's equations, quarter domain"
    );
    for ((design, width, (neff, dn), (ng, dg)), w) in table.into_iter().zip(wires) {
        let w = w?;
        let nm = width * 1e3;
        println!(
            "  {nm:.0} x 211 nm ({}): n_eff {:.5}, n_g {:.5}",
            w.grid, w.fab.effective_index, w.fab.group_index
        );
        println!(
            "    designed {:.0} x 215 nm: n_eff {:.5}, path differences {:.3} um (m = 15) and {:.3} um (M = 110)",
            design * 1e3,
            w.drawn,
            w.delta.0,
            w.delta.1
        );
        println!(
            "    Eq. 5: +-20 nm of width moves n_eff {:.4}, n_g {:.4}; +-5 nm of thickness {:.4}, {:.4}",
            w.by_width.0, w.by_width.1, w.by_thickness.0, w.by_thickness.1
        );
        // rounded up to the third decimal
        let up = |x: f64| (x * 1e3).ceil() / 1e3;
        checks.compare(
            &format!("n_eff at 1550 nm, {nm:.0} nm wide"),
            w.extracted.0,
            neff,
            up(w.by_width.0 + w.by_thickness.0 + dn),
        );
        checks.compare(
            &format!("n_g at 1550 nm, {nm:.0} nm wide"),
            w.extracted.1,
            ng,
            up(w.by_width.1 + w.by_thickness.1 + dg),
        );
    }
    Ok(checks.finish())
}
