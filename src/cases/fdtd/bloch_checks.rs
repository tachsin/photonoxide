//! The Bloch boundaries' checks: measurements the tests assert and the validation report
//! reports.

use super::checks::grid;
use super::media_checks::{alone, both, column, drude_lorentz};
use super::*;
use crate::fdfd::{Boundaries3d, Solver3d};
use crate::mode::multilayer::Multilayer;
use crate::units::{Length, Wavelength};

/// The steady complex amplitude of every E at `frequency` in a complex run driven by
/// e^(−iωt): (1/T) Σ E(t) e^(iωt) over `periods` whole periods.
fn complex_amplitude(
    s: &mut Simulation,
    frequency: Frequency,
    steps_per_period: usize,
    periods: usize,
) -> Vec<c64> {
    let cells = s.grid().cells();
    let omega = frequency.angular();
    let mut amplitude = vec![c64::new(0.0, 0.0); 3 * cells];
    for _ in 0..periods * steps_per_period {
        s.step();
        let w = c64::new(0.0, omega * s.time()).exp();
        for c in Axis::ALL {
            let im = s.e_imaginary(c).expect("a complex run");
            for (r, (&a, &b)) in s.e(c).iter().zip(im).enumerate() {
                amplitude[c.index() * cells + r] += c64::new(a, b) * w;
            }
        }
    }
    let scale = 1.0 / (periods * steps_per_period) as f64;
    amplitude.iter_mut().for_each(|a| *a *= scale);
    amplitude
}

/// A continuous current, e^(−iωt) at one value and a dipole restricted across a Bloch side, in
/// a box Bloch-periodic along x (k = 1.3 rad/µm) and y (k = −2.1 rad/µm) between walls along
/// z (16 × 14 × 12 cells of 50 nm, 1.55 µm, 64 steps a period), filled with a lossy medium
/// (ε = 2.1, σ = 2/µm) or, with `dispersive`, with [`drude_lorentz`] and the same σ: the
/// complex run's steady amplitude against `Solver3d`'s Bloch field for the same currents at
/// the leapfrog's frequency. The largest difference relative to the largest field.
///
/// The medium is uniform: FDTD samples σ at the values and FDFD averages ε + iσ over their
/// cells, which differ where a cell holds two media.
pub(crate) fn against_fdfd(dispersive: bool) -> f64 {
    let (h, n, steps_per_period, sigma) = (0.05, [16usize, 14, 12], 64, 2.0);
    let g = grid(n, h);
    let (kx, ky) = (1.3, -2.1);
    let boundaries = Boundaries {
        x: Edges::Bloch { k: kx },
        y: Edges::Bloch { k: ky },
        z: Edges::Pml { low: 0, high: 0 },
        cpml: Cpml::default(),
    };
    let eps = |_: f64, _: f64, _: f64| 2.1;
    let frequency = Frequency::natural(1.0 / 1.55).unwrap();
    let dt = 1.55 / steps_per_period as f64;
    let courant = dt * (3.0 / (h * h)).sqrt();
    let medium = drude_lorentz();
    let mut s = Simulation::new(g, eps, boundaries, courant)
        .unwrap()
        .with_conductivity(|_, _, _| sigma)
        .unwrap();
    if dispersive {
        s = s.with_medium(&medium, |_, _, _| true).unwrap();
    }
    assert!(s.is_complex());
    let waveform = Waveform::Continuous {
        frequency,
        amplitude: c64::new(1.0, 0.0),
        ramp: 6.0 * 1.55,
    };
    let at = (n[0] / 2 + 1, n[1] / 2, n[2] / 2);
    s.add_current(Current {
        field: Field::E,
        values: vec![(Axis::Y, at, c64::new(1.0, 0.0))],
        waveform,
    })
    .unwrap();
    // beside the high x side and the low y side: its weights wrap with the Bloch phases
    let dipole = Dipole {
        field: Field::E,
        component: Axis::Z,
        position: [g.x0 + (n[0] as f64 - 0.3) * h, g.y0 - 0.2 * h, 0.017],
        amplitude: c64::new(0.002, 0.001),
        waveform,
    };
    s.add_dipole(dipole).unwrap();
    let restricted = s.currents[1].values.clone();
    // the transients fall as e^(−σt/2ε): below 1e-20 in 70 periods
    s.run(70 * steps_per_period);
    let amplitude = complex_amplitude(&mut s, frequency, steps_per_period, 4);
    let mut current = vec![c64::new(0.0, 0.0); g.unknowns()];
    current[g.index(Axis::Y, at)] = c64::new(1.0, 0.0);
    for &(c, r, a) in &restricted {
        current[c * g.cells() + r] += a;
    }
    let tilde = s.leapfrog_wavenumber(frequency);
    let loss = c64::new(0.0, sigma * (frequency.angular() * dt / 2.0).cos() / tilde);
    let lam = Wavelength::um(std::f64::consts::TAU / tilde).unwrap();
    let inside = medium.leapfrog_permittivity(frequency, dt);
    let fdfd_eps = move |x: f64, y: f64, z: f64| {
        if dispersive {
            inside + loss
        } else {
            c64::new(eps(x, y, z), 0.0) + loss
        }
    };
    let fdfd = Solver3d::new(
        g,
        lam,
        fdfd_eps,
        Boundaries3d {
            x: Edges::Bloch { k: kx },
            y: Edges::Bloch { k: ky },
            ..Boundaries3d::pml(0)
        },
    )
    .unwrap()
    .solve(&current)
    .unwrap();
    let largest = fdfd.values().iter().fold(0.0f64, |m, v| m.max(v.norm()));
    fdfd.values()
        .iter()
        .zip(&amplitude)
        .fold(0.0f64, |m, (a, b)| m.max((a - b).norm()))
        / largest
}

/// The stack of [`multilayer`]: 4 pairs of n = 2 (0.12 µm) and n = 1.5 (0.16 µm), in vacuum.
const STACK: [(f64, f64); 2] = [(2.0, 0.12), (1.5, 0.16)];
/// Its pairs.
const PAIRS: usize = 4;
/// The Bloch wavenumber along the layers, rad/µm: 2π × 0.2.
const KX: f64 = std::f64::consts::TAU * 0.2;
/// The frequencies of [`multilayer`], c/µm.
pub(crate) const MULTILAYER_BAND: [f64; 7] = [0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3];

/// A pulse at oblique incidence on [`STACK`]'s pairs, at a fixed wavenumber along the layers,
/// k_x = 2π × 0.2 rad/µm: a column one cell across, Bloch-periodic along x (cells of `h` µm,
/// Courant number 0.9, CPMLs of 0.5 µm), lit by a sheet of current e^(ik_x x) (E_y, TE; or J_x,
/// TM, with `tm`). The transmittance |DFT(E)|² behind the stack over the same without it, at
/// [`MULTILAYER_BAND`], each at its own angle sin θ = k_x/ω, against the transfer matrices'.
/// The largest difference, and each frequency's angle (degrees), transmittance and the
/// matrices'.
pub(crate) fn multilayer(h: f64, tm: bool) -> (f64, Vec<(f64, f64, f64, f64)>) {
    let cells = |x: f64| (x / h).round() as usize;
    let cpml = cells(0.5);
    let thickness = PAIRS as f64 * (STACK[0].1 + STACK[1].1);
    let nz = 2 * cpml + cells(thickness + 2.0);
    let (g, boundaries) = column(h, nz, cpml, KX);
    let front = g.node(Axis::Z, cpml + cells(1.0));
    let layered = move |_: f64, _: f64, z: f64| {
        let into = z - front;
        if into < 0.0 || into >= thickness {
            return 1.0;
        }
        let period = STACK[0].1 + STACK[1].1;
        let n = if into % period < STACK[0].1 {
            STACK[0].0
        } else {
            STACK[1].0
        };
        n * n
    };
    let component = if tm { Axis::X } else { Axis::Y };
    let (source, probe) = (cpml + cells(0.4), nz - cpml - cells(0.4));
    let pulse = Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.3).unwrap();
    let run = |with_stack: bool| -> (Vec<c64>, f64) {
        let eps = move |x: f64, y: f64, z: f64| if with_stack { layered(x, y, z) } else { 1.0 };
        let mut s = Simulation::new(g, eps, boundaries, 0.9).unwrap();
        s.add_current(Current {
            field: Field::E,
            values: vec![(component, (0, 0, source), c64::new(1.0, 0.0))],
            waveform: pulse,
        })
        .unwrap();
        let p = s.add_probe(Field::E, component, (0, 0, probe)).unwrap();
        s.run_until(300.0);
        (s.probe_complex(p), s.dt())
    };
    let ((full, dt), (empty, _)) = both(|| run(true), || run(false));
    let dft = |series: &[c64], f: f64| -> c64 {
        let omega = std::f64::consts::TAU * f;
        series
            .iter()
            .enumerate()
            .map(|(n, v)| v * c64::new(0.0, omega * (n + 1) as f64 * dt).exp())
            .sum()
    };
    let films: Vec<(c64, Length)> = (0..PAIRS)
        .flat_map(|_| STACK)
        .map(|(n, d)| (c64::new(n, 0.0), Length::um(d)))
        .collect();
    let stack = Multilayer::new(c64::new(1.0, 0.0), &films, c64::new(1.0, 0.0)).unwrap();
    let polarization = if tm {
        crate::mode::Polarization::Tm
    } else {
        crate::mode::Polarization::Te
    };
    let rows: Vec<(f64, f64, f64, f64)> = MULTILAYER_BAND
        .iter()
        .map(|&f| {
            let measured = (dft(&full, f) / dft(&empty, f)).norm_sqr();
            let angle = (KX / (std::f64::consts::TAU * f)).asin();
            let exact = stack
                .reflection(polarization, Wavelength::um(1.0 / f).unwrap(), angle)
                .unwrap()
                .transmittance;
            (f, angle.to_degrees(), measured, exact)
        })
        .collect();
    let worst = rows
        .iter()
        .fold(0.0f64, |m, (_, _, a, b)| m.max((a - b).abs()));
    (worst, rows)
}

/// The 1D photonic crystal of [`bands`]: n = 2 for 0.2 µm, then vacuum for 0.3 µm.
const CRYSTAL: [(f64, f64); 2] = [(2.0, 0.2), (1.0, 0.3)];

/// The crystal's analytic bands at the Bloch wavenumber `k` (rad/µm) at normal incidence: the
/// frequencies (c/µm) below `top` where cos(kΛ) = cos(k₁d₁) cos(k₂d₂) −
/// ½(n₁/n₂ + n₂/n₁) sin(k₁d₁) sin(k₂d₂), kᵢ = nᵢω: half the trace of one period's transfer
/// matrix, which is cos(kΛ) for a Bloch wave. By bisection.
pub(crate) fn analytic_bands(k: f64, top: f64) -> Vec<f64> {
    let [(n1, d1), (n2, d2)] = CRYSTAL;
    let period = d1 + d2;
    let f = |nu: f64| {
        let w = std::f64::consts::TAU * nu;
        (n1 * w * d1).cos() * (n2 * w * d2).cos()
            - 0.5 * (n1 / n2 + n2 / n1) * (n1 * w * d1).sin() * (n2 * w * d2).sin()
            - (k * period).cos()
    };
    let points = 20_000;
    let mut out = Vec::new();
    for i in 0..points {
        let (mut a, mut b) = (
            top * i as f64 / points as f64,
            top * (i + 1) as f64 / points as f64,
        );
        if a == 0.0 {
            a = 1e-9;
        }
        if f(a) * f(b) > 0.0 {
            continue;
        }
        for _ in 0..200 {
            let m = 0.5 * (a + b);
            if f(a) * f(m) <= 0.0 {
                b = m;
            } else {
                a = m;
            }
        }
        out.push(0.5 * (a + b));
    }
    out
}

/// The crystal's bands below 2 c/µm at the Bloch wavenumber `k` (rad/µm) on cells of `h`
/// µm (one period, its interfaces on nodes, Courant number 0.9; a column one cell across), by
/// FDTD: a complex run from a current of the waveform's analytic form (positive frequencies
/// only), first broadband, its probe's DFT's peaks the bands to within its resolution; then,
/// for each, a run whose pulse is 0.03 c/µm wide at that peak, and the frequency from the
/// probe's one-mode recurrence E^(n+1) = z Eⁿ by least squares once the pulse is over,
/// ω = −arg(z)/Δt. Each band's frequency, c/µm.
pub(crate) fn bands(h: f64, k: f64) -> Vec<f64> {
    let [(n1, d1), (_, d2)] = CRYSTAL;
    let period = d1 + d2;
    let nz = (period / h).round() as usize;
    let g = Grid3d {
        nx: 1,
        ny: 1,
        nz,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
    };
    let boundaries = Boundaries {
        x: Edges::Bloch { k: 0.0 },
        y: Edges::Bloch { k: 0.0 },
        z: Edges::Bloch { k },
        cpml: Cpml::default(),
    };
    // periodic: the cells at the ends average across z = 0
    let eps = move |_: f64, _: f64, z: f64| {
        if z.rem_euclid(period) < d1 {
            n1 * n1
        } else {
            1.0
        }
    };
    // a source and a probe off the crystal's symmetry planes, so every band shows
    let (source, probe) = ((0.37 * nz as f64) as usize, (0.81 * nz as f64) as usize);
    let run = |waveform: Waveform, time: f64| -> (Vec<c64>, f64) {
        let mut s = Simulation::new(g, eps, boundaries, 0.9).unwrap();
        s.add_current(Current {
            field: Field::E,
            values: vec![(Axis::X, (0, 0, source), c64::new(1.0, 0.0))],
            waveform,
        })
        .unwrap();
        let p = s.add_probe(Field::E, Axis::X, (0, 0, probe)).unwrap();
        s.run_until(time);
        (s.probe_complex(p), s.dt())
    };
    // broadband: the peaks of |DFT| over the second half of a long run, windowed by
    // Blackman and Harris's four terms (sidelobes 92 dB down: F. J. Harris, Proc. IEEE 66, 51
    // (1978)), on frequencies 1/600 c/µm apart
    let top = 2.0;
    let broad = Waveform::pulse(Frequency::natural(1.0).unwrap(), 2.0).unwrap();
    let (series, dt) = alone(|| run(broad, 400.0));
    let tail = &series[series.len() / 2..];
    let m = tail.len() as f64;
    let window: Vec<c64> = tail
        .iter()
        .enumerate()
        .map(|(n, v)| {
            let x = std::f64::consts::TAU * n as f64 / m;
            let w =
                0.35875 - 0.48829 * x.cos() + 0.14128 * (2.0 * x).cos() - 0.01168 * (3.0 * x).cos();
            v * w
        })
        .collect();
    let spectrum: Vec<(f64, f64)> = (1..1200)
        .map(|i| {
            let f = top * i as f64 / 1200.0;
            let w = std::f64::consts::TAU * f;
            let d: c64 = window
                .iter()
                .enumerate()
                .map(|(n, v)| v * c64::new(0.0, w * n as f64 * dt).exp())
                .sum();
            (f, d.norm())
        })
        .collect();
    let largest = spectrum.iter().fold(0.0f64, |m, p| m.max(p.1));
    let peaks: Vec<f64> = spectrum
        .windows(3)
        .filter(|w| w[1].1 > w[0].1 && w[1].1 >= w[2].1 && w[1].1 > 1e-3 * largest)
        .map(|w| w[1].0)
        .collect();
    let band = |peak: f64| {
        let narrow = Waveform::pulse(Frequency::natural(peak).unwrap(), 0.03).unwrap();
        let Waveform::Gaussian { delay, width, .. } = narrow else {
            unreachable!("a pulse is a Gaussian")
        };
        let over = delay + 6.0 * width;
        let (series, dt) = alone(|| run(narrow, over + 200.0));
        let first = (over / dt) as usize;
        let (mut num, mut den) = (c64::new(0.0, 0.0), 0.0);
        for n in first..series.len() - 1 {
            num += series[n + 1] * series[n].conj();
            den += series[n].norm_sqr();
        }
        -(num / den).arg() / dt / std::f64::consts::TAU
    };
    // each band's run on a thread of its own
    std::thread::scope(|s| {
        let runs: Vec<_> = peaks.iter().map(|&p| s.spawn(move || band(p))).collect();
        runs.into_iter()
            .map(|r| r.join().expect("a band's run"))
            .collect()
    })
}
