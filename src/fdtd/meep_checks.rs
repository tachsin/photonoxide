//! Oskooi et al. 2010's published cases (A. F. Oskooi et al., Comput. Phys. Commun. 181, 687
//! (2010), doi:10.1016/j.cpc.2009.11.008), reproduced from the paper alone: Meep's code is GPL
//! and isn't read.

use num_complex::Complex64 as c64;

use super::checks::grid;
use super::*;

/// Fig. 8 (right): a point source E_z in 2D vacuum at a resolution of `resolution` cells a
/// wavelength, in a square of `interior` wavelengths inside CPMLs `thickness` wavelengths thick
/// on every side, σ graded as (x/L)^`order` to a round-trip reflection `reflection` (κ = 1,
/// α = 0, a PML in discrete time): E_z a wavelength from the source along x, at the source's
/// frequency, per unit source spectrum (the transform of a pulse 0.2 c/µm wide, nothing of it
/// at zero frequency, divided by the pulse's).
pub(crate) fn point_source_field(
    resolution: usize,
    interior: usize,
    thickness: usize,
    order: f64,
    reflection: f64,
) -> c64 {
    let h = 1.0 / resolution as f64;
    let cpml = thickness * resolution;
    let n = interior * resolution + 2 * cpml;
    let g = Grid3d {
        nz: 1,
        ..grid([n, n, 1], h)
    };
    let mut boundaries = Boundaries::cpml_2d(cpml);
    boundaries.cpml = Cpml {
        reflection,
        order,
        ..Cpml::default()
    };
    let mut s = Simulation::new(
        g,
        |_, _, _| 1.0,
        boundaries,
        std::f64::consts::FRAC_1_SQRT_2,
    )
    .unwrap();
    let frequency = Frequency::natural(1.0).unwrap();
    let pulse = Waveform::pulse(frequency, 0.2).unwrap();
    let centre = n / 2;
    s.add_source(Source {
        field: Field::E,
        component: Axis::Z,
        at: (centre, centre, 0),
        waveform: pulse,
    })
    .unwrap();
    let at = (centre + resolution, centre, 0);
    let dft = s.add_dft(at, at, &[frequency]).unwrap();
    let probe = s.add_probe(Field::E, Axis::Z, at).unwrap();
    let decayed = s.run_until_decayed(&[probe], 1e-26, 5.0, 400.0).unwrap();
    assert!(decayed, "the field didn't decay");
    let spectrum = pulse.spectrum(Field::E, s.dt(), s.steps(), frequency);
    s.dft(dft).value(Field::E, Axis::Z, at, 0).unwrap() / spectrum
}

/// Fig. 8's rates: for σ graded as (x/L)^d, d = 1, 2 and 3, the slope of log |E^(L+1) − E^L|²
/// against log L at `thickness` wavelengths, from the differences either side taken at their
/// midpoints (20 cells a wavelength, a cell of 4 wavelengths inside the PMLs, R = 1e-15).
pub(crate) fn pml_rates(thickness: usize) -> [f64; 3] {
    [1.0, 2.0, 3.0].map(|order| {
        let fields: Vec<c64> = (thickness - 1..=thickness + 1)
            .map(|l| point_source_field(20, 4, l, order, 1e-15))
            .collect();
        let before = (fields[1] - fields[0]).norm_sqr();
        let after = (fields[2] - fields[1]).norm_sqr();
        let l = thickness as f64;
        (before / after).ln() / ((l + 0.5) / (l - 0.5)).ln()
    })
}

/// A resonance of the ring of [`ring_resonances`]: its azimuthal order, FDTD's complex
/// frequency f − iγ/2π (c/µm) and the exact one.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RingMode {
    pub(crate) order: u32,
    pub(crate) fdtd: c64,
    pub(crate) exact: c64,
}

/// Fig. 11's ring (ε = 11.56) in 2D, between radii 1 and 2 µm in vacuum (inside the inset's
/// proportions; the paper gives no size), at `resolution` cells a µm (1 µm of vacuum around
/// it, then CPMLs of 2 µm), smoothed: E_z kicked by a pulse at 0.15 c/µm on one point of the
/// ring and recorded on another for 300 µm/c after it, its resonances from 0.1 to 0.2 c/µm by
/// harmonic inversion, each against the exact resonance of the order whose root is nearest.
pub(crate) fn ring_resonances(resolution: usize) -> Vec<RingMode> {
    // the report's cases share each resolution's run
    static RUNS: std::sync::Mutex<Vec<(usize, Vec<RingMode>)>> = std::sync::Mutex::new(Vec::new());
    if let Some((_, modes)) = RUNS.lock().unwrap().iter().find(|(r, _)| *r == resolution) {
        return modes.clone();
    }
    let modes = ring_run(resolution);
    RUNS.lock().unwrap().push((resolution, modes.clone()));
    modes
}

fn ring_run(resolution: usize) -> Vec<RingMode> {
    use super::ring::Ring;
    use crate::geometry::{Point, Shape};
    let (inner, outer, eps) = (1.0, 2.0, 11.56);
    let h = 1.0 / resolution as f64;
    let half = outer + 1.0 + 2.0;
    let n = (2.0 * half * resolution as f64).round() as usize;
    let g = Grid3d {
        nz: 1,
        ..grid([n, n, 1], h)
    };
    let shape = Shape::ring(
        Point::um(0.0, 0.0),
        crate::units::Length::um(0.5 * (inner + outer)),
        crate::units::Length::um(outer - inner),
    )
    .unwrap();
    let structure = Structure::new(Permittivity::isotropic(1.0).unwrap())
        .with(Body::extruded(shape), Permittivity::isotropic(eps).unwrap());
    let boundaries = Boundaries::cpml_2d(2 * resolution);
    let mut s = Simulation::smoothed(g, &structure, Smoothing::default(), boundaries, 0.9).unwrap();
    let node = |x: f64| ((x - g.x0) / h).round() as usize;
    let pulse = Waveform::pulse(Frequency::natural(0.15).unwrap(), 0.15).unwrap();
    s.add_source(Source {
        field: Field::E,
        component: Axis::Z,
        at: (node(1.45 * 0.3f64.cos()), node(1.45 * 0.3f64.sin()), 0),
        waveform: pulse,
    })
    .unwrap();
    let probe = s
        .add_probe(
            Field::E,
            Axis::Z,
            (node(1.55 * 2.0f64.cos()), node(1.55 * 2.0f64.sin()), 0),
        )
        .unwrap();
    let Waveform::Gaussian { width, delay, .. } = pulse else {
        unreachable!()
    };
    s.run_until(delay + 6.0 * width);
    let start = s.probe(probe).len();
    s.run_until(s.time() + 300.0);
    // every few steps, about 0.25 µm/c apart: well below the window's Nyquist limit
    let every = (0.25 / s.dt()).floor().max(1.0) as usize;
    let signal: Vec<c64> = s.probe(probe)[start..]
        .iter()
        .step_by(every)
        .map(|&v| c64::new(v, 0.0))
        .collect();
    let found = harmonic_inversion(
        &signal,
        every as f64 * s.dt(),
        Frequency::natural(0.1).unwrap(),
        Frequency::natural(0.2).unwrap(),
    )
    .unwrap();
    let ring = Ring::new(inner, outer, eps).unwrap();
    found
        .iter()
        .filter(|r| r.error < 1e-6 && r.decay > 0.0)
        .filter_map(|r| {
            let fdtd = c64::new(r.frequency, -r.decay / std::f64::consts::TAU);
            (0..16)
                .filter_map(|m| ring.resonance(m, fdtd).ok().map(|e| (m, e)))
                .filter(|(_, e)| (e - fdtd).norm() < 0.02 * fdtd.re && e.im < 0.0)
                .min_by(|a, b| (a.1 - fdtd).norm().total_cmp(&(b.1 - fdtd).norm()))
                .map(|(order, exact)| RingMode { order, fdtd, exact })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ring's three resonances between 0.1 and 0.2 c/µm, m = 3, 4 and 5, on 10 cells a µm:
    /// frequencies within 3.1e-3 and Q within 2.9 % of the exact ones.
    #[test]
    fn the_rings_resonances_are_exact_ones() {
        let modes = ring_resonances(10);
        assert_eq!(modes.iter().map(|m| m.order).collect::<Vec<_>>(), [3, 4, 5]);
        for m in &modes {
            let q = |f| super::super::ring::ring_q(f);
            assert!((m.fdtd.re - m.exact.re).abs() < 4e-3 * m.exact.re, "{m:?}");
            assert!((q(m.fdtd) - q(m.exact)).abs() < 0.04 * q(m.exact), "{m:?}");
        }
    }

    /// The ring at 10, 20 and 40 cells a µm, printed; about two minutes, so on demand.
    #[test]
    #[ignore]
    fn the_rings_resonances_converge() {
        for r in [10, 20, 40] {
            let modes = ring_resonances(r);
            println!("{r} cells a µm:");
            for m in &modes {
                println!(
                    "  m {:>2}: fdtd {:.8} exact {:.8}  df/f {:.3e}  Q {:.2} against {:.2}",
                    m.order,
                    m.fdtd,
                    m.exact,
                    (m.fdtd.re - m.exact.re).abs() / m.exact.re,
                    super::super::ring::ring_q(m.fdtd),
                    super::super::ring::ring_q(m.exact)
                );
            }
        }
    }

    /// The rates at 3 wavelengths, still above Fig. 8's: 6.26, 8.40 and 10.59.
    #[test]
    fn pml_rates_approach_the_papers() {
        let rates = pml_rates(3);
        for (rate, expected) in rates.iter().zip([6.0, 8.0, 10.0]) {
            assert!(rate - expected > 0.0 && rate - expected < 0.7, "{rates:?}");
        }
    }
}
