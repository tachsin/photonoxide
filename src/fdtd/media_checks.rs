//! The dispersive media's checks: measurements the tests assert and the validation report
//! reports.

use super::checks::{grid, steady_amplitude};
use super::*;
use crate::fdfd::{Boundaries3d, Solver3d};
use crate::units::Wavelength;

/// A Drude term and a Lorentz term, both well damped: ε∞ = 2, f_p = 1.2 c/µm, f₀ = 0.9 c/µm,
/// Δε = 1.5, dampings 0.3 c/µm.
pub(crate) fn drude_lorentz() -> Dispersive {
    Dispersive {
        eps_inf: 2.0,
        poles: vec![
            Pole::Drude {
                plasma: Frequency::natural(1.2).unwrap(),
                damping: 0.3,
            },
            Pole::Lorentz {
                strength: 1.5,
                resonance: Frequency::natural(0.9).unwrap(),
                damping: 0.3,
            },
        ],
    }
}

/// A continuous current in a closed box filled with [`drude_lorentz`] and a conductivity
/// (16 × 14 × 12 cells of 50 nm, σ = 2/µm, 1.55 µm, 64 steps a period): its steady amplitude
/// against `Solver3d`'s field for the same current at the leapfrog's frequency ω̃, with
/// [`Dispersive::leapfrog_permittivity`] and the conductivity's iσ cos(ωΔt/2)/ω̃. The largest
/// difference relative to the largest field, and the same with the medium's own ε(ω̃).
///
/// The medium fills the box: it is sampled at the values, FDFD's permittivity averaged over
/// their cells, and the two are the same only where the medium is uniform over a cell.
pub(crate) fn against_fdfd() -> (f64, f64) {
    let (h, n, steps_per_period, sigma) = (0.05, [16usize, 14, 12], 64, 2.0);
    let g = grid(n, h);
    let medium = drude_lorentz();
    let frequency = Frequency::natural(1.0 / 1.55).unwrap();
    let dt = 1.55 / steps_per_period as f64;
    let courant = dt * (3.0 / (h * h)).sqrt();
    let mut s = Simulation::new(g, |_, _, _| 1.0, Boundaries::walls(), courant)
        .unwrap()
        .with_conductivity(|_, _, _| sigma)
        .unwrap()
        .with_medium(&medium, |_, _, _| true)
        .unwrap();
    let at = (n[0] / 2 + 1, n[1] / 2, n[2] / 2);
    s.add_source(Source {
        field: Field::E,
        component: Axis::Y,
        at,
        waveform: Waveform::Continuous {
            frequency,
            amplitude: c64::new(1.0, 0.0),
            ramp: 6.0 * 1.55,
        },
    })
    .unwrap();
    s.run(70 * steps_per_period);
    let amplitude = steady_amplitude(&mut s, frequency, steps_per_period, 4);
    let mut current = vec![c64::new(0.0, 0.0); g.unknowns()];
    current[g.index(Axis::Y, at)] = c64::new(1.0, 0.0);
    let tilde = s.leapfrog_wavenumber(frequency);
    let loss = c64::new(0.0, sigma * (frequency.angular() * dt / 2.0).cos() / tilde);
    let compare = |eps: c64| -> f64 {
        let lam = Wavelength::um(std::f64::consts::TAU / tilde).unwrap();
        let fdfd = Solver3d::new(g, lam, move |_, _, _| eps + loss, Boundaries3d::pml(0))
            .unwrap()
            .solve(&current)
            .unwrap();
        let largest = fdfd.values().iter().fold(0.0f64, |m, v| m.max(v.norm()));
        fdfd.values()
            .iter()
            .zip(&amplitude)
            .fold(0.0f64, |m, (a, b)| m.max((a - b).norm()))
            / largest
    };
    (
        compare(medium.leapfrog_permittivity(frequency, dt)),
        compare(medium.permittivity(Frequency::natural(tilde / std::f64::consts::TAU).unwrap())),
    )
}

/// A column one cell across (periodic across, `kx` the Bloch wavenumber along x) of `nz` cells
/// of `h` µm along z, with CPMLs of `cpml` cells at its ends.
pub(crate) fn column(h: f64, nz: usize, cpml: usize, kx: f64) -> (Grid3d, Boundaries) {
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
        x: Edges::Bloch { k: kx },
        y: Edges::Bloch { k: 0.0 },
        z: Edges::Pml {
            low: cpml,
            high: cpml,
        },
        cpml: Cpml::default(),
    };
    (g, boundaries)
}

/// `a` and `b` side by side, each on one thread of its own: a column's updates are too small to
/// share among threads, and two runs on two threads take half the time.
pub(crate) fn both<A: Send, B: Send>(
    a: impl FnOnce() -> A + Send,
    b: impl FnOnce() -> B + Send,
) -> (A, B) {
    std::thread::scope(|s| {
        let first = s.spawn(|| alone(a));
        let second = alone(b);
        (first.join().expect("a run"), second)
    })
}

/// `f` on one thread.
pub(crate) fn alone<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("a thread pool")
        .install(f)
}

/// The DFTs Σ v(nΔt) e^(iωnΔt) of a series at `frequencies` (c/µm), and of t v(t) (for the
/// phase's derivative, d/dω of the first is i times the second), over its first `count` steps.
fn transforms(series: &[c64], dt: f64, frequencies: &[f64], count: usize) -> Vec<(c64, c64)> {
    frequencies
        .iter()
        .map(|f| {
            let omega = std::f64::consts::TAU * f;
            series.iter().take(count).enumerate().fold(
                (c64::new(0.0, 0.0), c64::new(0.0, 0.0)),
                |(d, td), (n, &v)| {
                    let t = (n + 1) as f64 * dt;
                    let w = c64::new(0.0, omega * t).exp() * v;
                    (d + w, td + w * t)
                },
            )
        })
        .collect()
}

/// The Drude metal of the half-space: ε∞ = 1, f_p = 1 c/µm, collisions 0.05 c/µm.
pub(crate) fn drude_metal() -> Dispersive {
    Dispersive {
        eps_inf: 1.0,
        poles: vec![Pole::Drude {
            plasma: Frequency::natural(1.0).unwrap(),
            damping: 0.05,
        }],
    }
}

/// The frequencies of [`drude_reflection`], c/µm: 0.4 to 1.6, below and above the plasma
/// frequency.
pub(crate) const DRUDE_BAND: [f64; 7] = [0.4, 0.6, 0.8, 1.0, 1.2, 1.4, 1.6];

/// A pulse at normal incidence on a half-space of [`drude_metal`] in a column (cells of `h`
/// µm, Courant number 0.9, CPMLs of 0.5 µm), its face halfway between two values of E: the
/// reflection coefficient from the run with the metal and the run without, referred to the
/// face with the grid's own wavenumber, against Fresnel's (1 − n)/(1 + n) with the metal's
/// ε(ω) at [`DRUDE_BAND`]. The largest |Δr|, and each frequency's r and Fresnel's.
pub(crate) fn drude_reflection(h: f64) -> (f64, Vec<(f64, c64, c64)>) {
    let cells = |x: f64| (x / h).round() as usize;
    let cpml = cells(0.5);
    let nz = 2 * cpml + cells(4.0);
    let (g, boundaries) = column(h, nz, cpml, 0.0);
    let face_node = cpml + cells(2.0);
    // the face midway between E_x's values at nodes face_node − 1 and face_node
    let face = g.node(Axis::Z, face_node) - h / 2.0;
    let (source, probe) = (cpml + cells(0.5), cpml + cells(1.5));
    let metal = drude_metal();
    let pulse = Waveform::pulse(Frequency::natural(1.0).unwrap(), 1.2).unwrap();
    let run = |with_metal: bool| -> (Vec<c64>, f64) {
        let mut s = Simulation::new(g, |_, _, _| 1.0, boundaries, 0.9).unwrap();
        if with_metal {
            s = s.with_medium(&metal, |_, _, z| z > face).unwrap();
        }
        s.add_source(Source {
            field: Field::E,
            component: Axis::X,
            at: (0, 0, source),
            waveform: pulse,
        })
        .unwrap();
        let p = s.add_probe(Field::E, Axis::X, (0, 0, probe)).unwrap();
        // the slowest light, near the plasma frequency, has died away by then
        s.run_until(150.0);
        (s.probe_complex(p), s.dt())
    };
    let ((full, dt), (empty, _)) = both(|| run(true), || run(false));
    let scattered: Vec<c64> = full.iter().zip(&empty).map(|(a, b)| a - b).collect();
    let count = full.len();
    let s = transforms(&scattered, dt, &DRUDE_BAND, count);
    let i = transforms(&empty, dt, &DRUDE_BAND, count);
    let distance = face - g.node(Axis::Z, probe);
    let rows: Vec<(f64, c64, c64)> = DRUDE_BAND
        .iter()
        .zip(s.iter().zip(&i))
        .map(|(&f, ((s, _), (i, _)))| {
            let frequency = Frequency::natural(f).unwrap();
            // the grid's wavenumber in vacuum: sin(kh/2)/h = sin(ωΔt/2)/Δt
            let w = frequency.angular();
            let k = 2.0 / h * ((w * dt / 2.0).sin() * h / dt).asin();
            let r = s / i * c64::new(0.0, -2.0 * k * distance).exp();
            let n = metal.permittivity(frequency).sqrt();
            let fresnel = (1.0 - n) / (1.0 + n);
            (f, r, fresnel)
        })
        .collect();
    let worst = rows
        .iter()
        .fold(0.0f64, |m, (_, r, fresnel)| m.max((r - fresnel).norm()));
    (worst, rows)
}

/// The Lorentz medium of the slab: ε∞ = 2.25, Δε = 1, f₀ = 2 c/µm, damping 0.02 c/µm.
pub(crate) fn lorentz_glass() -> Dispersive {
    Dispersive {
        eps_inf: 2.25,
        poles: vec![Pole::Lorentz {
            strength: 1.0,
            resonance: Frequency::natural(2.0).unwrap(),
            damping: 0.02,
        }],
    }
}

/// The frequencies of [`lorentz_delay`], c/µm.
pub(crate) const LORENTZ_BAND: [f64; 5] = [0.8, 0.9, 1.0, 1.1, 1.2];

/// The group index of [`lorentz_glass`], Re d(nω)/dω, at `f` (c/µm).
pub(crate) fn group_index(medium: &Dispersive, f: f64) -> f64 {
    let n = |f: f64| medium.permittivity(Frequency::natural(f).unwrap()).sqrt();
    let df = 1e-5 * f;
    (n(f) + f * (n(f + df) - n(f - df)) / (2.0 * df)).re
}

/// A pulse through a slab of [`lorentz_glass`] 10 µm thick, at normal incidence in a column
/// (cells of `h` µm, Courant number 0.9), the slab's faces halfway between values of E: the
/// group delay of the first pass (the transmitted series cut before the first echo arrives),
/// d arg(DFT)/dω of the run with the slab less that of the run without, as a group index
/// 1 + τ/d, at [`LORENTZ_BAND`], against the first pass's exact delay through the continuum,
/// d/dω arg(t₁₂ t₂₁ e^(i(n − 1)ωd)), as the same. The largest difference, and each frequency's
/// measured index, the exact first pass's and the medium's Re d(nω)/dω.
pub(crate) fn lorentz_delay(h: f64) -> (f64, Vec<(f64, f64, f64, f64)>) {
    let thickness = 10.0;
    let cells = |x: f64| (x / h).round() as usize;
    let cpml = cells(0.5);
    let nz = 2 * cpml + cells(thickness + 3.0);
    let (g, boundaries) = column(h, nz, cpml, 0.0);
    let front_node = cpml + cells(1.5);
    let front = g.node(Axis::Z, front_node) - h / 2.0;
    let back = front + thickness;
    let (source, probe) = (cpml + cells(0.5), nz - cpml - cells(0.5));
    let glass = lorentz_glass();
    let pulse = Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.3).unwrap();
    let ng = group_index(&glass, 1.0);
    // the first pass reaches the probe about (n_g − 1)d after the pulse would in vacuum, its
    // first echo 2 n_g d after that: cut halfway
    let vacuum = pulse_peak(&pulse) + g.node(Axis::Z, probe) - g.node(Axis::Z, source);
    let cut = vacuum + (ng - 1.0) * thickness + ng * thickness;
    let run = |with_slab: bool| -> (Vec<c64>, f64) {
        let mut s = Simulation::new(g, |_, _, _| 1.0, boundaries, 0.9).unwrap();
        if with_slab {
            s = s
                .with_medium(&glass, |_, _, z| z > front && z < back)
                .unwrap();
        }
        s.add_source(Source {
            field: Field::E,
            component: Axis::X,
            at: (0, 0, source),
            waveform: pulse,
        })
        .unwrap();
        let p = s.add_probe(Field::E, Axis::X, (0, 0, probe)).unwrap();
        s.run_until(cut);
        (s.probe_complex(p), s.dt())
    };
    let ((full, dt), (empty, _)) = both(|| run(true), || run(false));
    let a = transforms(&full, dt, &LORENTZ_BAND, full.len());
    let b = transforms(&empty, dt, &LORENTZ_BAND, empty.len());
    let rows: Vec<(f64, f64, f64, f64)> = LORENTZ_BAND
        .iter()
        .zip(a.iter().zip(&b))
        .map(|(&f, ((d, td), (d0, td0)))| {
            // d arg D/dω = Im(D'/D), D' = i Σ t v e^(iωt)
            let slope = |d: &c64, td: &c64| (c64::new(0.0, 1.0) * td / d).im;
            let tau = slope(d, td) - slope(d0, td0);
            let measured = 1.0 + tau / thickness;
            let first_pass = |f: f64| -> c64 {
                let n = glass.permittivity(Frequency::natural(f).unwrap()).sqrt();
                let t = 4.0 * n / ((1.0 + n) * (1.0 + n));
                t * (c64::new(0.0, std::f64::consts::TAU * f * thickness) * (n - 1.0)).exp()
            };
            let df = 1e-5 * f;
            let phase = (first_pass(f + df) / first_pass(f - df)).arg();
            let exact = 1.0 + phase / (std::f64::consts::TAU * 2.0 * df) / thickness;
            (f, measured, exact, group_index(&glass, f))
        })
        .collect();
    let worst = rows.iter().fold(0.0f64, |m, (_, measured, exact, _)| {
        m.max((measured - exact).abs())
    });
    (worst, rows)
}

/// A Gaussian waveform's peak, µm/c.
fn pulse_peak(w: &Waveform) -> f64 {
    match *w {
        Waveform::Gaussian { delay, .. } | Waveform::DifferentiatedGaussian { delay, .. } => delay,
        Waveform::Continuous { .. } => 0.0,
    }
}

/// The catalogue's material `id`, from its model `model` (its default if `None`), at its
/// default conditions: the first of its axes.
pub(crate) fn catalogue_material(id: &str, model: Option<&str>) -> crate::material::Material {
    let entry = crate::material::catalogue::entry(id).expect("a catalogue entry");
    let model = match model {
        Some(m) => entry.index.iter().find(|x| x.id == m).expect("a model"),
        None => entry.default_model().expect("a default model"),
    };
    model
        .materials(crate::material::catalogue::Conditions::default())
        .unwrap()
        .remove(0)
}

/// The catalogue's fits of [`fits`]: its id, model, band (µm).
pub(crate) const FITS: [(&str, Option<&str>, (f64, f64)); 3] = [
    ("si", None, (1.2, 1.7)),
    ("sio2", Some("malitson-1965"), (0.4, 1.6)),
    ("ingap", Some("ferrini-2002-table"), (0.4, 0.6)),
];

/// Each of [`FITS`] fitted by [`Dispersive::fit`]: its largest relative error and its number of
/// terms.
pub(crate) fn fits() -> Vec<(f64, usize)> {
    FITS.iter()
        .map(|&(id, model, (a, b))| {
            let material = catalogue_material(id, model);
            let fit = Dispersive::fit(
                &material,
                Wavelength::um(a).unwrap(),
                Wavelength::um(b).unwrap(),
            )
            .unwrap();
            (fit.relative, fit.medium.poles.len())
        })
        .collect()
}

/// The wavelengths of [`fitted_slab`], µm.
pub(crate) const SLAB_WAVELENGTHS: [f64; 3] = [1.3, 1.45, 1.6];

/// A slab of silicon 0.4 µm thick in vacuum, the catalogue's (Li 1980's table) fitted over
/// 1.2 to 1.7 µm, lit by a pulse at normal incidence in a column (cells of `h` µm, Courant
/// number 0.9, its faces halfway between values of E): the power reflection from the run with
/// the slab and the run without, |DFT(E_s)|²/|DFT(E_i)|², at [`SLAB_WAVELENGTHS`], against
/// Airy's formula with the catalogue's own index. The largest difference, and each
/// wavelength's reflectance and Airy's.
pub(crate) fn fitted_slab(h: f64) -> (f64, Vec<(f64, f64, f64)>) {
    let thickness = 0.4;
    let material = catalogue_material("si", None);
    let fit = Dispersive::fit(
        &material,
        Wavelength::um(1.2).unwrap(),
        Wavelength::um(1.7).unwrap(),
    )
    .unwrap();
    let cells = |x: f64| (x / h).round() as usize;
    let cpml = cells(0.5);
    let nz = 2 * cpml + cells(thickness + 2.5);
    let (g, boundaries) = column(h, nz, cpml, 0.0);
    let front = g.node(Axis::Z, cpml + cells(1.5)) - h / 2.0;
    let back = front + thickness;
    let (source, probe) = (cpml + cells(0.5), cpml + cells(1.0));
    let pulse = Waveform::pulse(Frequency::natural(0.7).unwrap(), 0.5).unwrap();
    let run = |with_slab: bool| -> (Vec<c64>, f64) {
        let mut s = Simulation::new(g, |_, _, _| 1.0, boundaries, 0.9).unwrap();
        if with_slab {
            s = s
                .with_medium(&fit.medium, |_, _, z| z > front && z < back)
                .unwrap();
        }
        s.add_source(Source {
            field: Field::E,
            component: Axis::X,
            at: (0, 0, source),
            waveform: pulse,
        })
        .unwrap();
        let p = s.add_probe(Field::E, Axis::X, (0, 0, probe)).unwrap();
        s.run_until(80.0);
        (s.probe_complex(p), s.dt())
    };
    let ((full, dt), (empty, _)) = both(|| run(true), || run(false));
    let scattered: Vec<c64> = full.iter().zip(&empty).map(|(a, b)| a - b).collect();
    let frequencies: Vec<f64> = SLAB_WAVELENGTHS.iter().map(|l| 1.0 / l).collect();
    let s = transforms(&scattered, dt, &frequencies, full.len());
    let i = transforms(&empty, dt, &frequencies, full.len());
    let rows: Vec<(f64, f64, f64)> = SLAB_WAVELENGTHS
        .iter()
        .zip(s.iter().zip(&i))
        .map(|(&lam, ((s, _), (i, _)))| {
            let measured = (s / i).norm_sqr();
            let n = material
                .refractive_index(Wavelength::um(lam).unwrap())
                .unwrap();
            let r12 = (1.0 - n) / (1.0 + n);
            let phase = (c64::new(0.0, 2.0 * std::f64::consts::TAU / lam * thickness) * n).exp();
            let r = r12 * (1.0 - phase) / (1.0 - r12 * r12 * phase);
            (lam, measured, r.norm_sqr())
        })
        .collect();
    let worst = rows
        .iter()
        .fold(0.0f64, |m, (_, a, b)| m.max((a - b).abs()));
    (worst, rows)
}
