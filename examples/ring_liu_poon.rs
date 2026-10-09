//! A ring resonator in 3D FDTD against Lumerical FDTD and Tidy3D.
//!
//! Z. Liu, J. K. S. Poon, "Comparison of Lumerical FDTD and Tidy3D for three-dimensional FDTD
//! simulations of passive silicon photonic components", Opt. Continuum 4, 2427 (2025),
//! [doi:10.1364/OPTCON.572107](https://doi.org/10.1364/OPTCON.572107) (open, arXiv:2506.16665),
//! Section 3.6 and Fig. 26:
//!
//! - **The device:** gdsfactory's generic `ring_single`: a 500 × 220 nm silicon bus and, 200 nm
//!   from it along a 4 µm straight, a ring of the same guide: four Euler bends of radius 10 µm
//!   (p = 0.5, the arc's footprint) joined by straights of 4 µm and 0.6 µm, 75.75 µm round; in
//!   silica. TE₀ enters the bus.
//! - **The result:** the lowest resonance in 1540 to 1560 nm, its full width at half maximum
//!   from the through port's spectrum, and Q = λ₀/FWHM (Eq. 2), against the resolution (Fig.
//!   26(a) to (c)); the free spectral range, about 7.4 nm (Lumerical) and 7.6 nm (Tidy3D).
//!
//! The paper's numbers, read off Fig. 26 (arXiv's PNG), good to ±0.02 nm in λ₀, ±0.002 nm in the
//! width and ±3 in Q:
//!
//! | Cells a wavelength | Lumerical λ₀, FWHM, Q | Tidy3D λ₀, FWHM, Q |
//! |---|---|---|
//! | 6 | 1546.65 nm, 0.84 nm, 1839 | 1547.28 nm, 0.88 nm, 1757 |
//! | 15 | 1547.73, 0.86, 1798 | 1545.77, 0.90, 1716 |
//! | 20 | 1545.57, 0.90, 1716 | 1545.07, 0.88, 1754 |
//! | 25 | 1545.67, 0.88, 1755 | 1544.96, 0.90, 1715 |
//!
//! The paper needs 20 cells a wavelength for the ring (Table 14), where the codes differ by 2 %
//! in the width and Q. Their spectra are sampled every 0.2 nm and interpolated to 0.02 nm; their
//! runs stop at 6.4 ps (1920 µm/c), when about 1 % of the ring's energy is left.
//!
//! Here, with the materials at 1.545 µm:
//!
//! - **The free spectral range** (what CI runs, seconds): λ²/(n_g L) at 1.545 µm, the ring's
//!   centre line L = 75.747 µm long (its four Euler bends drawn here) and n_g the 500 × 220 nm
//!   strip's group index with silicon's (Li) and silica's (Malitson) dispersion, from the
//!   full-vector mode solver (6.25 × 5 nm grid, quarter domain), against the paper's "around 7.4
//!   nm" and "7.6 nm", within half their difference and the text's last digit. The bends' own
//!   group index, a little above the straight's, is left out.
//! - **The 3D run** (`--full`: 15 and 20 cells a wavelength; hours on 20 threads, so not in CI):
//!   the field in the ring after the pulse, from 200 to 1000 µm/c, by harmonic inversion
//!   (`fdtd::harmonic_inversion`) between 1525 and 1575 nm: each resonance's wavelength and
//!   decay, Q = ω/2γ the loaded Q, and FWHM = λ₀/Q, the width of the through port's Lorentzian
//!   dip. The paper's broadband pulse sets a pulse going round the ring (306 µm/c a turn at
//!   n_g = 4.04), so the record holds two and a half turns; at 6 cells a wavelength, where the
//!   resonances are narrower, a record to 1000 µm/c was the shortest that found them cleanly.
//! - **The material dispersion** the runs leave out lengthens the round trip's group delay: Q
//!   grows, and the width and the free spectral range shrink, by the ratio of the strip's group
//!   index with and without dispersion (4.1792 and 4.0421, 1.0339). The wavelength needs no
//!   correction: it is where the indices at 1.545 µm put it.
//! - **The check** (`--full`): λ₀, the width and Q against the span of both codes' values at 20
//!   and 25 cells, where the paper finds the ring settled, its middle within half its width
//!   plus the reading.
//!
//! The ring isn't checked on a coarse grid: at 6 cells a wavelength (74 nm cells, the 200 nm gap
//! less than three) its coupling is a fifth of the settled one (Q about 8000, its resonances 4
//! nm short), where the paper's codes, on grids finer near the silicon, still give 1839 and 1757.
//!
//! Silicon's index sets λ₀: the paper's Palik silicon is 3.4738 at 1550 nm in Tidy3D's fit and
//! about 3.4764 in Lumerical's data, ours 3.4757. A difference of 0.0026 moves λ₀ by about 0.85
//! nm, the codes' own difference.
//!
//! ```sh
//! cargo run --release --example ring_liu_poon              # the free spectral range
//! cargo run --release --example ring_liu_poon -- --full    # 3D, 15 and 20 cells a wavelength
//! ```

mod common;
mod pdk;

use std::process::ExitCode;

use num_complex::Complex64 as c64;
use photonoxide::fdfd::Axis;
use photonoxide::fdtd::{Field, harmonic_inversion};
use photonoxide::material;
use photonoxide::mode::dispersion::{group_index, track};
use photonoxide::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity};
use photonoxide::units::{Frequency, Wavelength};

use pdk::{Kind, Output, Setup};

/// The materials' wavelength, µm.
const MATERIALS: f64 = 1.545;
/// The span of the run recorded for harmonic inversion, µm/c.
const RECORD: [f64; 2] = [200.0, 1000.0];

/// The 500 × 220 nm strip's group index at 1.545 µm, with silicon's and silica's dispersion
/// (`dispersive`) or with their indices at 1.545 µm: the quarter x, y ≥ 0 on a 6.25 × 5 nm grid
/// in a 1.05 × 0.75 µm window, an electric wall at x = 0 and a magnetic wall at y = 0 keeping
/// the TE-like modes.
fn strip_group_index(dispersive: bool) -> photonoxide::Result<f64> {
    let wavelengths = [1.535, MATERIALS, 1.555]
        .iter()
        .map(|&l| Wavelength::um(l))
        .collect::<photonoxide::Result<Vec<_>>>()?;
    let section = |l: Wavelength| {
        let at = if dispersive {
            l
        } else {
            Wavelength::um(MATERIALS)?
        };
        let si = material::silicon().permittivity(at)?;
        let ox = material::silica().permittivity(at)?;
        CrossSection::uniform((0.0, 1.05, 168), (0.0, 0.75, 150), |x, y| {
            let core = x < 0.25 && y < 0.11;
            Permittivity::isotropic(if core { si } else { ox })
        })?
        .with_boundaries(Boundaries {
            west: Boundary::ElectricWall,
            south: Boundary::MagneticWall,
            ..Boundaries::default()
        })
    };
    let modes = track(section, &wavelengths, None, 3)?;
    let n: Vec<f64> = modes.iter().map(|m| m.effective_index().re).collect();
    Ok(group_index(&wavelengths, &n)?[1])
}

/// A resonance: its wavelength (nm) and loaded Q.
struct Line {
    nm: f64,
    q: f64,
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let device = pdk::ring();
    let outputs = [Output {
        port: 1,
        kinds: vec![Kind::Te0],
    }];
    let (dispersive, fixed) = (strip_group_index(true)?, strip_group_index(false)?);
    let ratio = dispersive / fixed;
    let length = pdk::ring_path().length();
    println!(
        "Liu and Poon's ring (gdsfactory's generic PDK), TE0 into the bus; materials at \
         {MATERIALS} um"
    );
    println!(
        "the 500 x 220 nm strip's group index at {MATERIALS} um (full-vector, 6.25 x 5 nm grid): \
         {dispersive:.4} dispersive, {fixed:.4} not ({ratio:.4}); the ring {length:.3} um round"
    );
    let mut checks = common::Checks::default();
    let fsr = 1e3 * MATERIALS * MATERIALS / (dispersive * length);
    pdk::within(
        &mut checks,
        "free spectral range (nm), n_g and L",
        fsr,
        &[7.4, 7.6],
        0.05,
    );
    for res in pdk::requested(&[15.0, 20.0]) {
        let mut probes = Vec::new();
        let run = pdk::run(
            &Setup {
                device: &device,
                h: pdk::step(res),
                wavelength: MATERIALS,
                input: Kind::Te0,
                outputs: &outputs,
                wavelengths: &[1.55],
                limit: RECORD[1],
                decay: false,
            },
            &mut |sim| {
                // E_y on the top straight's centre line and E_x on the right one's, mid-silicon
                let g = sim.grid();
                let at = |x: f64, y: f64| {
                    (
                        ((x - g.x0) / g.dx).round() as usize,
                        ((y - g.y0) / g.dy).round() as usize,
                        ((0.11 - g.z0) / g.dz).round() as usize,
                    )
                };
                probes.push(sim.add_probe(Field::E, Axis::Y, at(-2.0, 21.3))?);
                probes.push(sim.add_probe(Field::E, Axis::X, at(10.0, 11.0))?);
                Ok(())
            },
        )?;
        let m = &run.measured;
        pdk::report_cost("ring_liu_poon", m);
        println!(
            "3D FDTD, {res} cells a wavelength: {}",
            pdk::describe(&m.grid)
        );
        // the two probes' sum after the pulse, about four samples a period at 1.525 µm
        let sim = &run.sim;
        let dt = sim.dt();
        let every = ((0.25 * 1.525 / dt).floor() as usize).max(1);
        let first = (RECORD[0] / dt).ceil() as usize;
        let (a, b) = (sim.probe(probes[0]), sim.probe(probes[1]));
        let signal: Vec<c64> = (first..a.len().min(b.len()))
            .step_by(every)
            .map(|n| c64::new(a[n] + b[n], 0.0))
            .collect();
        let found = harmonic_inversion(
            &signal,
            every as f64 * dt,
            Frequency::natural(1.0 / 1.575)?,
            Frequency::natural(1.0 / 1.525)?,
        )?;
        let largest = found.iter().map(|r| r.amplitude.norm()).fold(0.0, f64::max);
        let mut lines: Vec<Line> = found
            .iter()
            .filter(|r| r.error < 1e-4 && r.decay > 0.0 && r.amplitude.norm() > 1e-2 * largest)
            .map(|r| Line {
                nm: 1e3 / r.frequency,
                q: r.q(),
            })
            .filter(|l| (1527.0..1573.0).contains(&l.nm))
            .collect();
        lines.sort_by(|p, q| p.nm.total_cmp(&q.nm));
        println!("  resonance    Q (run)   Q (dispersive)  FWHM (dispersive)");
        for l in &lines {
            println!(
                "  {:.2} nm   {:.0}      {:.0}            {:.3} nm",
                l.nm,
                l.q,
                l.q * ratio,
                l.nm / (l.q * ratio)
            );
        }
        let k = lines
            .iter()
            .position(|l| l.nm >= 1540.0)
            .expect("a resonance in 1540 to 1560 nm");
        let (line, next) = (&lines[k], lines.get(k + 1));
        let (q, fwhm) = (line.q * ratio, line.nm / (line.q * ratio));
        if let Some(next) = next {
            println!(
                "  free spectral range {:.2} nm (dispersive)",
                (next.nm - line.nm) / ratio
            );
        }
        // both codes at 20 and 25 cells a wavelength
        let tag = format!("{res} cells");
        pdk::within(
            &mut checks,
            &format!("resonance (nm), {tag}"),
            line.nm,
            &[1545.57, 1545.67, 1545.07, 1544.96],
            0.02,
        );
        pdk::within(
            &mut checks,
            &format!("FWHM (nm), {tag}"),
            fwhm,
            &[0.90, 0.88, 0.88, 0.90],
            0.002,
        );
        pdk::within(
            &mut checks,
            &format!("Q, {tag}"),
            q,
            &[1716.0, 1755.0, 1754.0, 1715.0],
            3.0,
        );
    }
    Ok(checks.finish())
}
