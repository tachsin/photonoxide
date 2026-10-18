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
//! - **The result:** a resonance in 1540 to 1560 nm, its full width at half maximum from the
//!   through port's spectrum, and Q = λ₀/FWHM (Eq. 2), against the resolution (Fig. 26(a) to
//!   (c)); the free spectral range, about 7.4 nm (Lumerical) and 7.6 nm (Tidy3D).
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
//! **How the paper reads them** (its code's repository, github.com/JPPhotonics/fdtd-pipeline:
//! `initiate_fdtd.py` and the ring's `find_FWHM.py`):
//!
//! - **The record:** Lumerical's simulation time is 30 crossings of the cell's 32 µm, 1920 µm/c,
//!   a fixed time; both codes stop at 6.40 ps (Table 13).
//! - **The spectrum:** the through port's TE₀ at 101 frequencies evenly spaced from c/1560 nm to
//!   c/1540 nm (0.2 nm apart), over its maximum, through a cubic spline on 1000 points.
//! - **The dip:** the level halfway from the spline's minimum to 1; the first fall through it
//!   beyond 1541 nm and the first rise through it between 1545 and 1555 nm; FWHM the width
//!   between them, λ₀ the point midway, Q = λ₀/FWHM.
//!
//! The ring loses almost nothing but what it couples out: its Euler bends' radiation is below
//! what the mode solver resolves (Im n_eff < 1e-14 at their 7.06 µm minimum radius). Its loaded Q
//! is about 8500 (the straight pair's supermodes, 5 nm grid, give κ² = 0.14 through the coupler's
//! gap profile; Bogaerts et al. 2011's equations, doi:10.1002/lpor.201100017), so its energy falls
//! by 1/e every 2100 µm/c and the record ends with nearly half of it still in the ring. A lossless
//! ring's through port has no dip in steady state; the dip in a spectrum taken over 1920 µm/c is
//! the record's, and its width is set mostly by the record's length (issue #256 has a model of
//! it: 0.93 nm over 1920 µm/c, 0.46 nm over 4000, for κ² of 0.12 to 0.14 alike).
//!
//! Here, with the materials at 1.545 µm:
//!
//! - **The free spectral range** (what CI runs, seconds): λ²/(n_g L) at 1.545 µm, the ring's
//!   centre line L = 75.747 µm long (its four Euler bends drawn here) and n_g the 500 × 220 nm
//!   strip's group index with silicon's (Li) and silica's (Malitson) dispersion, from the
//!   full-vector mode solver (6.25 × 5 nm grid, quarter domain), against the paper's "around 7.4
//!   nm" and "7.6 nm", within half their difference and the text's last digit. The bends' own
//!   group index, a little above the straight's, is left out.
//! - **The 3D run** (`--full`: 15 and 20 cells a wavelength; hours on 20 threads, so not in CI) to
//!   the paper's 1920 µm/c. E_y on the bus's centre line 0.3 µm out of the input port and out of
//!   the through port (where the mode monitors are) gives the through port's spectrum at the
//!   paper's 101 frequencies over the same record: the same guide and the same point of its
//!   cross-section, so the mode's profile divides out (checked against the mode monitor at 1550
//!   nm). It is read as the paper reads it (a natural spline where SciPy's default is
//!   not-a-knot, which differs only near the band's ends), and over records of 1000 and 1500
//!   µm/c too, to show what the record does.
//! - **The ring's own Q**, beside it: the field in the ring after the pulse, from 200 to 1920
//!   µm/c, by harmonic inversion (`fdtd::harmonic_inversion`) between 1525 and 1575 nm: each
//!   resonance's wavelength and decay, Q = ω/2γ, and λ₀/Q. The material dispersion the run
//!   leaves out lengthens the round trip's group delay, so this Q is scaled by the ratio of the
//!   strip's group index with and without it (4.1792 and 4.0421, 1.0339); the wavelengths need
//!   no correction. The paper gives no such number, so it isn't checked.
//! - **The check** (`--full`): λ₀, the width and Q read the paper's way against the span of both
//!   codes' values at 20 and 25 cells, where the paper finds the ring settled, its middle within
//!   half its width plus the reading. Until #256 these checks took the ring's own Q and the
//!   lowest resonance at or above 1540 nm; they now take what the paper measures, with the same
//!   tolerances. Our pulse (0.04 c/µm wide) peaks at 56 µm/c, Tidy3D's (20 nm of bandwidth) at
//!   about 96 µm/c: in a model of the record the dip's width moves by under 0.01 nm for that.
//!
//! The resonance: the full-vector mode solver (Hadley's equations, 2.5 nm grid, extrapolated:
//! n_eff 2.450691 at 1.545 µm) and the bends' and the coupler's shifts put the ring's order 120
//! at 1546.44 nm with the materials at 1.545 µm (1546.40 nm with their dispersion; 1545.65 nm
//! with Tidy3D's silicon, 1546.68 nm with Lumerical's), order 121 at 1538.69 nm. The codes' line
//! is order 120, 0.6 to 1.1 nm short of that. The uniform grid here makes the guide's n_eff
//! long: the bus alone in FDTD gives 2.45669 at 15 cells and 2.45441 at 20, against 2.45069,
//! about +2.3 nm and +1.4 nm on a resonance.
//!
//! Measured at 15 cells (`--resolution 15`, 2026-10-18; 1101 × 833 × 99 cells of 29.7 nm, the
//! gap 6.7 cells; 112 988 steps to 1920 µm/c):
//!
//! | Read | Resonance | FWHM | Q | Down to |
//! |---|---|---|---|---|
//! | the paper's way, over 1000 µm/c | 1548.15 nm | 2.122 nm | 730 | 0.314 |
//! | the paper's way, over 1500 µm/c | 1548.15 nm | 1.141 nm | 1357 | 0.099 |
//! | **the paper's way, over 1920 µm/c** | **1548.15 nm** | **0.921 nm** | **1681** | 0.045 |
//! | the ring's own (harmonic inversion, dispersive) | 1548.14 nm | 0.19 to 0.21 nm | 7380 to 7960 | |
//! | the codes at 20 and 25 cells | 1544.96 to 1545.67 nm | 0.88 to 0.90 nm | 1715 to 1755 | |
//!
//! The width and Q read over the paper's record come within 2 to 5 % of the codes' (the codes
//! differ from each other by 2 %), where the ring's own Q is 4.5 times theirs; the width follows
//! the record's length, 2.1, 1.1 and 0.92 nm over 1000, 1500 and 1920 µm/c. Away from the dips
//! the spectrum is 0.79 of its maximum on average and ripples about 1.4 nm apart, as the paper's
//! Figs. 26(d) and 27 do. All three checks still fail at 15 cells: the width by 0.018 nm, Q by
//! 31 and the resonance by 2.5 nm, the grid's (above). The other resonances by harmonic
//! inversion: 1532.82, 1540.45, 1555.93 and 1563.78 nm, Q 6740 to 9060; the free spectral range
//! 7.44 nm. At 1550 nm the probes' ratio, 0.8732, is 0.85 % above the mode monitor's 0.8658.
//!
//! The run: 90.8 million cells, 6 h 26 min of stepping on 20 threads of a Core Ultra 7 265K (443
//! million cell-updates/s, other work on the machine at times), 54 s of smoothing, 71 s of mode
//! solves.
//!
//! Silicon's index sets λ₀: the paper's Palik silicon is 3.4738 at 1550 nm in Tidy3D's fit and
//! about 3.4764 in Lumerical's data, ours 3.4757. A difference of 0.0026 moves λ₀ by about 1.0
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
use photonoxide::fdtd::{Field, Simulation, harmonic_inversion};
use photonoxide::material;
use photonoxide::mode::dispersion::{group_index, track};
use photonoxide::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity};
use photonoxide::units::{Frequency, Wavelength};

use pdk::{Kind, Output, Setup};

/// The materials' wavelength, µm.
const MATERIALS: f64 = 1.545;
/// The paper's record: both codes stop at 6.40 ps (Table 13), Lumerical's simulation time set
/// to 30 crossings of the cell's 32 µm, µm/c.
const RECORD: f64 = 1920.0;
/// Harmonic inversion of the ring's field from here (after the pulse) to the end, µm/c.
const SETTLED: f64 = 200.0;
/// The paper's monitors: 101 frequencies evenly spaced from c/1.56 µm to c/1.54 µm, about 0.2
/// nm apart.
const SAMPLES: usize = 101;
const BAND: [f64; 2] = [1.54, 1.56];
/// Shorter records read the same way, to show what the record's length does, µm/c.
const SHORTER: [f64; 2] = [1000.0, 1500.0];

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

/// A probe's transform at `frequency` (c/µm) over its first `steps` values, E at each step's end.
fn transform(values: &[f64], dt: f64, frequency: f64, steps: usize) -> c64 {
    let w = std::f64::consts::TAU * frequency * dt;
    values
        .iter()
        .take(steps)
        .enumerate()
        .map(|(n, &v)| v * c64::from_polar(1.0, w * (n + 1) as f64))
        .sum()
}

/// The through port's power over the incident's at each of the paper's frequencies, from the
/// probes on the bus (the same guide, the same point of its cross-section, so the mode's
/// profile divides out), over the first `record` µm/c: (wavelengths ascending, µm; T).
fn spectrum(
    sim: &Simulation,
    incident: usize,
    through: usize,
    record: f64,
) -> (Vec<f64>, Vec<f64>) {
    let dt = sim.dt();
    let steps = (record / dt).round() as usize;
    let (a, b) = (sim.probe(incident), sim.probe(through));
    let (f0, f1) = (1.0 / BAND[1], 1.0 / BAND[0]);
    let mut points: Vec<(f64, f64)> = (0..SAMPLES)
        .map(|k| {
            let f = f0 + (f1 - f0) * k as f64 / (SAMPLES - 1) as f64;
            let t = (transform(b, dt, f, steps) / transform(a, dt, f, steps)).norm_sqr();
            (1.0 / f, t)
        })
        .collect();
    points.sort_by(|p, q| p.0.total_cmp(&q.0));
    points.into_iter().unzip()
}

/// The natural cubic spline through (x, y), x ascending, evaluated at `at`.
fn spline(x: &[f64], y: &[f64], at: &[f64]) -> Vec<f64> {
    let n = x.len();
    let h: Vec<f64> = x.windows(2).map(|w| w[1] - w[0]).collect();
    // second derivatives m, zero at the ends: tridiagonal by Thomas's algorithm
    let mut m = vec![0.0; n];
    let (mut c, mut d) = (vec![0.0; n], vec![0.0; n]);
    for i in 1..n - 1 {
        let a = h[i - 1];
        let b = 2.0 * (h[i - 1] + h[i]);
        let r = 6.0 * ((y[i + 1] - y[i]) / h[i] - (y[i] - y[i - 1]) / h[i - 1]);
        let denom = b - a * c[i - 1];
        c[i] = h[i] / denom;
        d[i] = (r - a * d[i - 1]) / denom;
    }
    for i in (1..n - 1).rev() {
        m[i] = d[i] - c[i] * m[i + 1];
    }
    at.iter()
        .map(|&t| {
            let k = x.partition_point(|&v| v <= t).clamp(1, n - 1) - 1;
            let (u, v) = (x[k + 1] - t, t - x[k]);
            let hk = h[k];
            m[k] * u.powi(3) / (6.0 * hk)
                + m[k + 1] * v.powi(3) / (6.0 * hk)
                + (y[k] / hk - m[k] * hk / 6.0) * u
                + (y[k + 1] / hk - m[k + 1] * hk / 6.0) * v
        })
        .collect()
}

/// The dip as the paper's analysis reads it (its code's `find_FWHM`, in its repository): the
/// spectrum over its maximum, a cubic spline on 1000 points across the band, the level halfway
/// from the spline's minimum to 1; the first fall through it beyond 1 nm into the band and the
/// first rise through it between 5 nm into the band and 5 nm before its end, each at the first
/// or last point below the level; the width between them, and the resonance the point midway.
/// (Its spline is SciPy's default, not-a-knot; this one is natural, which differs only near the
/// band's ends.) Returns (resonance, FWHM, minimum, the off-resonance mean), nm and normalized.
fn paper_dip(wavelengths: &[f64], t: &[f64]) -> Option<(f64, f64, f64, f64)> {
    let max = t.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let t: Vec<f64> = t.iter().map(|v| v / max).collect();
    let (lo, hi) = (wavelengths[0], wavelengths[wavelengths.len() - 1]);
    let x: Vec<f64> = (0..1000)
        .map(|k| lo + (hi - lo) * k as f64 / 999.0)
        .collect();
    let y = spline(wavelengths, &t, &x);
    let min = y.iter().copied().fold(f64::INFINITY, f64::min);
    let half = 0.5 + 0.5 * min;
    let first = (1..x.len())
        .find(|&i| y[i] < half && y[i - 1] > half && x[i] > lo + 0.001 && x[i] < hi - 0.005)?;
    let second = (0..x.len() - 1)
        .find(|&i| y[i] < half && y[i + 1] > half && x[i] > lo + 0.005 && x[i] < hi - 0.005)?;
    let mid = (0.5 * (first + second) as f64).round() as usize;
    // the spectrum's mean away from its dips: the points above the half level
    let above: Vec<f64> = y.iter().copied().filter(|&v| v > half).collect();
    let mean = above.iter().sum::<f64>() / above.len() as f64;
    Some((1e3 * x[mid], 1e3 * (x[second] - x[first]), min, mean))
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
                limit: RECORD,
                decay: false,
            },
            &mut |sim| {
                let g = sim.grid();
                let at = |x: f64, y: f64| {
                    (
                        ((x - g.x0) / g.dx).round() as usize,
                        ((y - g.y0) / g.dy).round() as usize,
                        ((0.11 - g.z0) / g.dz).round() as usize,
                    )
                };
                // E_y on the top straight's centre line and E_x on the right one's, mid-silicon
                probes.push(sim.add_probe(Field::E, Axis::Y, at(-2.0, 21.3))?);
                probes.push(sim.add_probe(Field::E, Axis::X, at(10.0, 11.0))?);
                // E_y on the bus's centre, 0.3 µm out of the input port (the incident pulse)
                // and of the through port, where the mode monitors are
                let (i, o) = (device.ports[0].out(0.3), device.ports[1].out(0.3));
                probes.push(sim.add_probe(Field::E, Axis::Y, at(i[0], i[1]))?);
                probes.push(sim.add_probe(Field::E, Axis::Y, at(o[0], o[1]))?);
                Ok(())
            },
        )?;
        let m = &run.measured;
        pdk::report_cost("ring_liu_poon", m);
        println!(
            "3D FDTD, {res} cells a wavelength: {}, to {:.0} um/c",
            pdk::describe(&m.grid),
            m.time
        );
        let sim = &run.sim;
        let dt = sim.dt();

        // the through port as the paper reads it, over its record
        let (wl, t) = spectrum(sim, probes[2], probes[3], RECORD);
        let steps = (RECORD / dt).round() as usize;
        let at_1550 = |p: usize| transform(sim.probe(probes[p]), dt, 1.0 / 1.55, steps);
        println!(
            "  through port at 1550 nm over the record: {:.5} from the probes, {:.5} from the mode \
             monitor",
            (at_1550(3) / at_1550(2)).norm_sqr(),
            m.at_1550(0, 0)
        );
        let max = t.iter().copied().fold(0.0, f64::max);
        println!("  the through port's spectrum over its maximum, 0.2 nm apart from 1540 nm:");
        for row in t.chunks(10) {
            let cells: Vec<String> = row.iter().map(|v| format!("{:.3}", v / max)).collect();
            println!("    {}", cells.join(" "));
        }
        // no dip where the paper's reading looks for one: the checks fail on NaN
        let (lambda0, fwhm, min, mean) =
            paper_dip(&wl, &t).unwrap_or((f64::NAN, f64::NAN, f64::NAN, f64::NAN));
        let q = lambda0 / fwhm;
        println!(
            "  the paper's reading over {RECORD:.0} um/c: resonance {lambda0:.2} nm, FWHM {fwhm:.3} \
             nm, Q {q:.0}; the dip down to {min:.3} of the maximum, the spectrum away from it \
             {mean:.3} on average"
        );
        for record in SHORTER {
            let (wl, t) = spectrum(sim, probes[2], probes[3], record);
            match paper_dip(&wl, &t) {
                Some((l, w, lo, _)) => println!(
                    "  the same reading over {record:.0} um/c: resonance {l:.2} nm, FWHM {w:.3} \
                     nm, Q {:.0}, down to {lo:.3}",
                    l / w
                ),
                None => println!("  the same reading over {record:.0} um/c: no dip read"),
            }
        }

        // the ring's own Q: its field after the pulse, about four samples a period at 1.525 µm
        let every = ((0.25 * 1.525 / dt).floor() as usize).max(1);
        let first = (SETTLED / dt).ceil() as usize;
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
        println!(
            "  the ring's own resonances, by harmonic inversion of its field from {SETTLED:.0} to \
             {RECORD:.0} um/c:"
        );
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
        if let Some(w) = lines
            .windows(2)
            .find(|w| w[1].nm > lambda0 - 1.0 && w[0].nm < lambda0 + 1.0)
        {
            println!(
                "  free spectral range {:.2} nm (dispersive), from {:.2} to {:.2} nm",
                (w[1].nm - w[0].nm) / ratio,
                w[0].nm,
                w[1].nm
            );
        }
        // both codes at 20 and 25 cells a wavelength, read by the paper from its record
        let tag = format!("{res} cells");
        pdk::within(
            &mut checks,
            &format!("resonance (nm), {tag}"),
            lambda0,
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
