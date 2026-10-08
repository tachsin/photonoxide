//! The FDTD solver's checks: measurements the tests assert and the validation report reports.

use super::*;
use crate::fdfd::{Boundaries3d, Solver3d};
use crate::units::Wavelength;

pub(crate) fn grid(n: [usize; 3], h: f64) -> Grid3d {
    Grid3d {
        nx: n[0],
        ny: n[1],
        nz: n[2],
        dx: h,
        dy: h,
        dz: h,
        x0: -(n[0] as f64) * h / 2.0,
        y0: -(n[1] as f64) * h / 2.0,
        z0: -(n[2] as f64) * h / 2.0,
    }
}

pub(crate) fn periodic() -> Boundaries {
    let p = Edges::Bloch { k: 0.0 };
    Boundaries {
        x: p,
        y: p,
        z: p,
        cpml: Cpml::default(),
    }
}

/// E_z of a plane wave cos(kx x + ky y) on a periodic box, H̃ zero: an eigenvector of the
/// discrete curl-curl, so E_z(t) = cos(nθ) E_z(0) exactly. The measured θ against Taflove and
/// Brodwin's relation sin²(θ/2) = (Δt²/4) Σ (4/Δ_i²) sin²(k_i Δ_i/2).
pub(crate) fn dispersion(m: (usize, usize), courant: f64) -> (f64, f64) {
    let g = Grid3d {
        nx: 24,
        ny: 18,
        nz: 1,
        dx: 0.05,
        dy: 0.04,
        dz: 0.05,
        x0: 0.0,
        y0: 0.0,
        z0: 0.0,
    };
    let (kx, ky) = (
        std::f64::consts::TAU * m.0 as f64 / (g.nx as f64 * g.dx),
        std::f64::consts::TAU * m.1 as f64 / (g.ny as f64 * g.dy),
    );
    let mut s = Simulation::new(g, |_, _, _| 1.0, periodic(), courant).unwrap();
    for j in 0..g.ny {
        for i in 0..g.nx {
            let [x, y, _] = g.e_position(Axis::Z, (i, j, 0));
            s.e_mut(Axis::Z)[j * g.nx + i] = (kx * x + ky * y).cos();
        }
    }
    // a point where the wave is large
    let at = (0..g.nx * g.ny)
        .max_by(|&a, &b| s.e(Axis::Z)[a].abs().total_cmp(&s.e(Axis::Z)[b].abs()))
        .unwrap();
    let p = s
        .add_probe(Field::E, Axis::Z, (at % g.nx, at / g.nx, 0))
        .unwrap();
    let e0 = s.e(Axis::Z)[at];
    s.run(400);
    let mut series = vec![e0];
    series.extend_from_slice(s.probe(p));
    // cos θ from E^(n+1) + E^(n−1) = 2 cos θ E^n, by least squares
    let (mut num, mut den) = (0.0, 0.0);
    for n in 1..series.len() - 1 {
        num += series[n] * (series[n + 1] + series[n - 1]);
        den += 2.0 * series[n] * series[n];
    }
    let measured = (num / den).acos();
    let dt = s.dt();
    let lambda = 4.0 / (g.dx * g.dx) * (kx * g.dx / 2.0).sin().powi(2)
        + 4.0 / (g.dy * g.dy) * (ky * g.dy / 2.0).sin().powi(2);
    let theory = 2.0 * (dt * lambda.sqrt() / 2.0).asin();
    (measured, theory)
}

/// A pseudo-random value in [−1, 1) from an integer.
pub(crate) fn noise(r: usize) -> f64 {
    let mut x = (r as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x5DEE_CE66;
    x ^= x >> 31;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 29;
    (x >> 11) as f64 / (1u64 << 52) as f64 - 1.0
}

/// A closed box with a dielectric block and random fields, stepped 300 times: the largest
/// change of the leapfrog's invariant ½ Σ ε E·E + ½ Σ H̃(t − Δt/2)·H̃(t + Δt/2), relative to it.
pub(crate) fn energy_drift() -> f64 {
    let g = grid([10, 9, 8], 0.05);
    let mut s = Simulation::new(
        g,
        |x, y, _| {
            if x.abs() < 0.12 && y.abs() < 0.1 {
                12.0
            } else {
                2.0
            }
        },
        Boundaries::walls(),
        0.95,
    )
    .unwrap();
    for c in Axis::ALL {
        for r in 0..g.cells() {
            s.e_mut(c)[r] = noise(r + 7 * c.index());
            s.h_mut(c)[r] = noise(r + 13 * c.index() + 100_000);
        }
    }
    // the walls' own values are held at zero from the first step
    s.run(1);
    let volume = g.dx * g.dy * g.dz;
    let energy = |s: &mut Simulation| {
        let before: Vec<Vec<f64>> = Axis::ALL.iter().map(|&c| s.h(c).to_vec()).collect();
        let electric = s.electric_energy();
        s.step();
        let magnetic: f64 = Axis::ALL
            .iter()
            .zip(&before)
            .map(|(&c, b)| b.iter().zip(s.h(c)).map(|(p, q)| p * q).sum::<f64>())
            .sum();
        electric + 0.5 * magnetic * volume
    };
    let first = energy(&mut s);
    (0..300)
        .map(|_| (energy(&mut s) - first).abs() / first)
        .fold(0.0, f64::max)
}

/// A 2D pulse from a dipole in vacuum, recorded `gap` cells from a CPML `thickness` cells thick,
/// against the same interior inside a much larger grid: the largest difference over the run,
/// relative to the largest field.
pub(crate) fn cpml_error(thickness: usize) -> f64 {
    let (h, interior, gap) = (0.05, 40usize, 2usize);
    let pulse = Waveform::Gaussian {
        frequency: Frequency::natural(1.0).unwrap(),
        width: 0.5,
        delay: 1.6,
    };
    let record = |pad: usize, cells: usize| -> Vec<f64> {
        let n = interior + 2 * pad;
        let g = Grid3d {
            nz: 1,
            ..grid([n, n, 1], h)
        };
        let mut s = Simulation::new(g, |_, _, _| 1.0, Boundaries::cpml_2d(cells), 0.7).unwrap();
        let centre = n / 2;
        s.add_source(Source {
            field: Field::E,
            component: Axis::Z,
            at: (centre, centre, 0),
            waveform: pulse,
        })
        .unwrap();
        // near the small grid's CPML, beside the source and towards a corner
        let near = pad + gap;
        let p = s.add_probe(Field::E, Axis::Z, (near, centre, 0)).unwrap();
        let q = s.add_probe(Field::E, Axis::Z, (near, near, 0)).unwrap();
        s.run_until(9.0);
        s.probe(p).iter().chain(s.probe(q)).copied().collect()
    };
    let small = record(thickness, thickness);
    // a reference whose own boundary's reflections can't come back within the run
    let reference = record(thickness + 120, 20);
    let largest = reference.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    small
        .iter()
        .zip(&reference)
        .fold(0.0f64, |m, (a, b)| m.max((a - b).abs()))
        / largest
}

/// The 3D field of a continuous current against FDFD's for the same current: the largest
/// difference (outside the PMLs, if any), relative to the largest field, with FDFD at the
/// leapfrog's frequency ω̃ = (2/Δt) sin(ωΔt/2) and at ω itself.
///
/// With `conductivity` σ, the box is closed by walls and filled with a lossy medium, ε = 2.1:
/// the leapfrog's steady state then solves FDFD's equations exactly at ω̃ with
/// ε + iσ cos(ωΔt/2)/ω̃ (the conductivity averaged over the step), with no PML to differ.
/// Without, a silicon block in a CPML of 4 cells (FDFD's PML in the time domain).
pub(crate) fn against_fdfd(steps_per_period: usize, conductivity: Option<f64>) -> (f64, f64) {
    let (h, n) = (0.05, [16usize, 14, 12]);
    let pml = if conductivity.is_some() { 0 } else { 4usize };
    let g = grid(n, h);
    let eps = move |x: f64, y: f64, _: f64| {
        if conductivity.is_none() && x.abs() < 0.2 && y.abs() < 0.11 {
            12.0
        } else {
            2.1
        }
    };
    let frequency = Frequency::natural(1.0 / 1.55).unwrap();
    let omega = frequency.angular();
    let period = 1.55;
    let dt = period / steps_per_period as f64;
    let courant = dt * (3.0 / (h * h)).sqrt();
    let mut s = Simulation::new(g, eps, Boundaries::cpml(pml), courant).unwrap();
    if let Some(sigma) = conductivity {
        s = s.with_conductivity(|_, _, _| sigma).unwrap();
    }
    assert!((s.dt() - dt).abs() < 1e-15 * dt);
    let at = (n[0] / 2 + 1, n[1] / 2, n[2] / 2);
    s.add_source(Source {
        field: Field::E,
        component: Axis::Y,
        at,
        waveform: Waveform::Continuous {
            frequency,
            amplitude: c64::new(1.0, 0.0),
            ramp: 6.0 * period,
        },
    })
    .unwrap();
    s.run(60 * steps_per_period);
    // the amplitude at ω over whole periods: (2/T) Σ E(t) e^(iωt) Δt
    let periods = 4;
    let cells = g.cells();
    let mut amplitude: Vec<c64> = vec![c64::new(0.0, 0.0); 3 * cells];
    for _ in 0..periods * steps_per_period {
        s.step();
        let w = c64::new(0.0, omega * s.time()).exp();
        for c in Axis::ALL {
            for (r, &v) in s.e(c).iter().enumerate() {
                amplitude[c.index() * cells + r] += v * w;
            }
        }
    }
    let scale = 2.0 / (periods * steps_per_period) as f64;
    for a in &mut amplitude {
        *a *= scale;
    }
    let mut current = vec![c64::new(0.0, 0.0); g.unknowns()];
    current[g.index(Axis::Y, at)] = c64::new(1.0, 0.0);
    let compare = |at_omega: f64| -> f64 {
        let loss = conductivity.map_or(0.0, |sigma| sigma * (omega * dt / 2.0).cos() / at_omega);
        let fdfd_eps = move |x: f64, y: f64, z: f64| c64::new(eps(x, y, z), loss);
        let lam = Wavelength::um(std::f64::consts::TAU / at_omega).unwrap();
        let fdfd = Solver3d::new(g, lam, fdfd_eps, Boundaries3d::pml(pml))
            .unwrap()
            .solve(&current)
            .unwrap();
        let inside = |r: usize| {
            let rest = r % cells;
            let at = [rest % g.nx, (rest / g.nx) % g.ny, rest / (g.nx * g.ny)];
            (0..3).all(|a| at[a] > pml && at[a] + pml + 1 < n[a])
        };
        let largest = fdfd
            .values()
            .iter()
            .enumerate()
            .filter(|(r, _)| inside(*r))
            .fold(0.0f64, |m, (_, v)| m.max(v.norm()));
        fdfd.values()
            .iter()
            .zip(&amplitude)
            .enumerate()
            .filter(|(r, _)| inside(*r))
            .fold(0.0f64, |m, (_, (a, b))| m.max((a - b).norm()))
            / largest
    };
    let tilde = 2.0 / dt * (omega * dt / 2.0).sin();
    (compare(tilde), compare(omega))
}

/// The DFT of every E (Σ E(nΔt) e^(iωnΔt) Δt) at `frequencies`, accumulated while `s` runs
/// `steps` more steps; E is zero at the start.
pub(crate) fn dft_e(s: &mut Simulation, steps: usize, frequencies: &[f64]) -> Vec<Vec<c64>> {
    let cells = s.grid().cells();
    let mut out = vec![vec![c64::new(0.0, 0.0); 3 * cells]; frequencies.len()];
    for _ in 0..steps {
        s.step();
        let t = s.time();
        for (f, sum) in frequencies.iter().zip(out.iter_mut()) {
            let w = c64::new(0.0, std::f64::consts::TAU * f * t).exp() * s.dt();
            for c in Axis::ALL {
                for (r, &v) in s.e(c).iter().enumerate() {
                    sum[c.index() * cells + r] += v * w;
                }
            }
        }
    }
    out
}

/// A Gaussian pulse from an electric point current and from a magnetic dipole between values
/// (restricted to the grid), in a closed box of a lossy medium (ε = 2.1, σ = 2/µm) where the
/// fields die away: E's DFT at the carrier and 0.12 c/µm off it, against `Solver3d`'s field
/// for the sources' own transforms ([`Waveform::spectrum`]) at the leapfrog's frequency. The
/// largest difference relative to the largest field.
pub(crate) fn spectrum_against_fdfd() -> f64 {
    let (h, n) = (0.05, [16usize, 14, 12]);
    let g = grid(n, h);
    let (eps, sigma) = (2.1, 2.0);
    let carrier = 1.0 / 1.55;
    let pulse = Waveform::pulse(Frequency::natural(carrier).unwrap(), 0.3).unwrap();
    let mut s = Simulation::new(g, |_, _, _| eps, Boundaries::walls(), 0.9)
        .unwrap()
        .with_conductivity(|_, _, _| sigma)
        .unwrap();
    let at = (n[0] / 2 + 1, n[1] / 2, n[2] / 2);
    s.add_source(Source {
        field: Field::E,
        component: Axis::Y,
        at,
        waveform: pulse,
    })
    .unwrap();
    s.add_dipole(Dipole {
        field: Field::H,
        component: Axis::Z,
        position: [-0.113, 0.071, 0.052],
        amplitude: c64::new(0.004, 0.0),
        waveform: pulse,
    })
    .unwrap();
    let magnetic = s.currents[0].values.clone();
    // the fields fall as e^(−σt/ε): to 1e-17 of their peak well within the run
    let steps = (60.0 / s.dt()) as usize;
    let frequencies = [carrier, carrier + 0.12];
    let dfts = dft_e(&mut s, steps, &frequencies);
    let dt = s.dt();
    let mut worst = 0.0f64;
    for (f, dft) in frequencies.iter().zip(&dfts) {
        let frequency = Frequency::natural(*f).unwrap();
        let omega = frequency.angular();
        let tilde = s.leapfrog_wavenumber(frequency);
        let loss = sigma * (omega * dt / 2.0).cos() / tilde;
        let lam = Wavelength::um(std::f64::consts::TAU / tilde).unwrap();
        let solver = Solver3d::new(
            g,
            lam,
            move |_, _, _| c64::new(eps, loss),
            Boundaries3d::pml(0),
        )
        .unwrap();
        let mut j = vec![c64::new(0.0, 0.0); g.unknowns()];
        j[g.index(Axis::Y, at)] = pulse.spectrum(Field::E, dt, steps, frequency);
        let mut m = vec![c64::new(0.0, 0.0); g.unknowns()];
        let spectrum_m = pulse.spectrum(Field::H, dt, steps, frequency);
        for &(c, r, a) in &magnetic {
            m[c * g.cells() + r] = a * spectrum_m;
        }
        let electric = solver.solve(&j).unwrap();
        let from_m = solver.solve_magnetic(&m).unwrap();
        let fdfd: Vec<c64> = electric
            .values()
            .iter()
            .zip(from_m.values())
            .map(|(a, b)| a + b)
            .collect();
        let largest = fdfd.iter().fold(0.0f64, |w, v| w.max(v.norm()));
        let error = fdfd
            .iter()
            .zip(dft)
            .fold(0.0f64, |w, (a, b)| w.max((a - b).norm()));
        worst = worst.max(error / largest);
    }
    worst
}

/// Whether `field`'s `component` at `at` is inside the box from nodes `low` to `high` (an axis
/// of one cell is whole).
fn in_box(
    field: Field,
    component: Axis,
    at: [usize; 3],
    low: [usize; 3],
    high: [usize; 3],
    n: [usize; 3],
) -> bool {
    Axis::ALL.into_iter().all(|axis| {
        let a = axis.index();
        if n[a] == 1 {
            return true;
        }
        let t = super::sources::twice(field, component, axis, at[a]);
        t >= 2 * low[a] as i64 && t <= 2 * high[a] as i64
    })
}

/// A plane-wave pulse on a total-field/scattered-field box in vacuum with nothing inside: the
/// largest field anywhere in the scattered region over the run (E and H̃), relative to the
/// largest incident field. In 2D (`three_d` false) on 80 × 80 cells of 50 nm, the box 30 cells
/// a side; in 3D on 36³ cells, the box 12 a side; CPMLs of 10 and 8.
pub(crate) fn tfsf_leakage(
    direction: (i64, i64, i64),
    polarization: [f64; 3],
    three_d: bool,
) -> f64 {
    let h = 0.05;
    let (n, low, high, cpml) = if three_d {
        ([36usize, 36, 36], [12usize, 12, 12], [24usize, 24, 24], 8)
    } else {
        ([80usize, 80, 1], [25usize, 25, 0], [55usize, 55, 1], 10)
    };
    let g = grid(n, h);
    let boundaries = if three_d {
        Boundaries::cpml(cpml)
    } else {
        Boundaries::cpml_2d(cpml)
    };
    let mut s = Simulation::new(g, |_, _, _| 1.0, boundaries, 0.9).unwrap();
    s.add_plane_wave(PlaneWave {
        low: (low[0], low[1], low[2]),
        high: (high[0], high[1], high[2]),
        direction,
        polarization,
        eps: 1.0,
        waveform: Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.6).unwrap(),
    })
    .unwrap();
    let (mut leak, mut largest) = (0.0f64, 0.0f64);
    let steps = (12.0 / s.dt()) as usize;
    for _ in 0..steps {
        s.step();
        for field in [Field::E, Field::H] {
            for c in Axis::ALL {
                let values = match field {
                    Field::E => s.e(c),
                    Field::H => s.h(c),
                };
                for (r, &v) in values.iter().enumerate() {
                    let at = [r % n[0], (r / n[0]) % n[1], r / (n[0] * n[1])];
                    if in_box(field, c, at, low, high, n) {
                        largest = largest.max(v.abs());
                    } else {
                        leak = leak.max(v.abs());
                    }
                }
            }
        }
    }
    leak / largest
}

/// A slab of ε = 4, 0.3 µm thick, inside a total-field/scattered-field box at normal incidence
/// (a column one cell across, periodic across, cells of `h` µm along z), lit by a pulse: the
/// power reflection from the scattered field in front, |DFT(E_s)|²/|DFT(E_i)|², at 0.8, 1 and
/// 1.2 c/µm, against Airy's formula for the slab. The largest difference.
pub(crate) fn tfsf_slab(h: f64) -> f64 {
    let (thickness, eps_slab) = (0.3, 4.0);
    let cells = |x: f64| (x / h).round() as usize;
    let cpml = cells(0.5);
    let nz = 2 * cpml + cells(2.4);
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
    let (low, high) = (cpml + cells(0.4), nz - cpml - cells(0.4));
    let za = g.node(Axis::Z, low + cells(0.5));
    let zb = za + thickness;
    let boundaries = Boundaries {
        x: Edges::Bloch { k: 0.0 },
        y: Edges::Bloch { k: 0.0 },
        z: Edges::Pml {
            low: cpml,
            high: cpml,
        },
        cpml: Cpml::default(),
    };
    let eps = move |_: f64, _: f64, z: f64| if z >= za && z < zb { eps_slab } else { 1.0 };
    let mut s = Simulation::new(g, eps, boundaries, 0.9).unwrap();
    let wave = s
        .add_plane_wave(PlaneWave {
            low: (0, 0, low),
            high: (1, 1, high),
            direction: (0, 0, 1),
            polarization: [1.0, 0.0, 0.0],
            eps: 1.0,
            waveform: Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.8).unwrap(),
        })
        .unwrap();
    let front = (0, 0, low - cells(0.2));
    let inside = (0, 0, low + 2);
    let frequencies = [0.8, 1.0, 1.2];
    let mut scattered = [c64::new(0.0, 0.0); 3];
    let mut incident = [c64::new(0.0, 0.0); 3];
    let steps = (30.0 / s.dt()) as usize;
    for _ in 0..steps {
        s.step();
        let t = s.time();
        let es = s.e(Axis::X)[front.2];
        let ei = s.incident(wave, Field::E, Axis::X, inside);
        for (q, f) in frequencies.iter().enumerate() {
            let w = c64::new(0.0, std::f64::consts::TAU * f * t).exp();
            scattered[q] += es * w;
            incident[q] += ei * w;
        }
    }
    frequencies
        .iter()
        .enumerate()
        .map(|(q, f)| {
            let measured = (scattered[q] / incident[q]).norm_sqr();
            let n2 = eps_slab.sqrt();
            let r12 = (1.0 - n2) / (1.0 + n2);
            let phase = c64::new(0.0, 2.0 * n2 * std::f64::consts::TAU * f * thickness).exp();
            let r = r12 * (1.0 - phase) / (1.0 - r12 * r12 * phase);
            (measured - r.norm_sqr()).abs()
        })
        .fold(0.0, f64::max)
}

/// The steady amplitude of every E at `frequency` over `periods` whole periods of
/// `steps_per_period` steps: (2/T) Σ E(t) e^(iωt) Δt, as the run goes on.
pub(crate) fn steady_amplitude(
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
            for (r, &v) in s.e(c).iter().enumerate() {
                amplitude[c.index() * cells + r] += v * w;
            }
        }
    }
    let scale = 2.0 / (periods * steps_per_period) as f64;
    amplitude.iter_mut().for_each(|a| *a *= scale);
    amplitude
}

/// A rectangular guide along z: a core of ε = 12, 0.4 × 0.3 µm, in ε = 2.1.
pub(crate) fn guide(x: f64, y: f64, _: f64) -> f64 {
    if x.abs() < 0.2 && y.abs() < 0.15 {
        12.0
    } else {
        2.1
    }
}

/// The time step for `steps_per_period` steps a period of 1.55 µm on cells of 50 nm in 3D, as
/// a Courant number.
pub(crate) fn courant_for(steps_per_period: usize) -> f64 {
    1.55 / steps_per_period as f64 * (3.0f64 / (0.05 * 0.05)).sqrt()
}

/// The guide's fundamental mode launched forward by FDTD in a closed box of a lossy medium
/// (σ = 4/µm everywhere; 16 × 14 × 24 cells of 50 nm, 1.55 µm, 64 steps a period), its steady
/// amplitude against `Solver3d`'s field for its own mode source of the same mode at the
/// leapfrog's frequency, with ε + iσ cos(ωΔt/2)/ω̃: the largest difference relative to the
/// largest field.
pub(crate) fn mode_source_against_fdfd() -> f64 {
    let (n, sigma, steps_per_period) = ([16usize, 14, 24], 4.0, 64);
    let g = grid(n, 0.05);
    let frequency = Frequency::natural(1.0 / 1.55).unwrap();
    let mut s = Simulation::new(g, guide, Boundaries::walls(), courant_for(steps_per_period))
        .unwrap()
        .with_conductivity(|_, _, _| sigma)
        .unwrap();
    let dt = s.dt();
    let tilde = s.leapfrog_wavenumber(frequency);
    let loss = sigma * (frequency.angular() * dt / 2.0).cos() / tilde;
    let solver = Solver3d::new(
        g,
        s.fdfd_wavelength(frequency).unwrap(),
        move |x, y, z| c64::new(guide(x, y, z), loss),
        Boundaries3d::pml(0),
    )
    .unwrap();
    let mode = solver.port_modes(Axis::Z, 6, 1).unwrap().remove(0);
    s.add_mode_source(
        &mode,
        crate::fdfd::Direction::Forward,
        Waveform::Continuous {
            frequency,
            amplitude: c64::new(1.0, 0.0),
            ramp: 4.0 * 1.55,
        },
    )
    .unwrap();
    // the transients fall as e^(−σt/ε) in the core: to 1e-15 in 90 periods
    s.run(90 * steps_per_period);
    let amplitude = steady_amplitude(&mut s, frequency, steps_per_period, 4);
    let fdfd = solver
        .solve_system(&solver.mode_source(&mode, crate::fdfd::Direction::Forward))
        .unwrap();
    let largest = fdfd.values().iter().fold(0.0f64, |m, v| m.max(v.norm()));
    fdfd.values()
        .iter()
        .zip(&amplitude)
        .fold(0.0f64, |m, (a, b)| m.max((a - b).norm()))
        / largest
}

/// The open guide (40 × 36 × 40 cells of 50 nm, CPMLs of 8, 64 steps a period of 1.55 µm)
/// with its fundamental mode at 1.55 µm launched forward on plane 13, by a continuous wave at
/// 1.55 µm (`pulse` false) or a pulse 0.1 c/µm wide (`pulse` true). At 1.55 µm, and for the
/// pulse also half its width either side, each from the run's DFT: the frequency, the power
/// going backward behind the source (the mode's backward amplitude on plane 10, squared, times
/// its power) relative to the forward power ahead (the flux through plane 26), and that flux
/// relative to the mode's own power at the source's amplitude there.
pub(crate) fn mode_source_open(pulse: bool) -> Vec<(f64, f64, f64)> {
    use crate::fdfd::{Direction, Formulation, IterativeSolver3d};
    let (n, cpml, steps_per_period) = ([40usize, 36, 40], 8, 64);
    let g = grid(n, 0.05);
    let carrier = 1.0 / 1.55;
    let frequency = Frequency::natural(carrier).unwrap();
    let mut s = Simulation::new(
        g,
        guide,
        Boundaries::cpml(cpml),
        courant_for(steps_per_period),
    )
    .unwrap();
    let dt = s.dt();
    let lattice = |f: f64| {
        // FDFD at the leapfrog's frequency, (2/Δt) sin(ωΔt/2)
        let tilde = 2.0 / dt * (std::f64::consts::PI * f * dt).sin();
        let lam = Wavelength::um(std::f64::consts::TAU / tilde).unwrap();
        IterativeSolver3d::new(
            g,
            lam,
            |x, y, z| c64::new(guide(x, y, z), 0.0),
            Boundaries3d::pml(cpml),
            Formulation::CurlCurl,
        )
        .unwrap()
    };
    let at_carrier = lattice(carrier);
    let mode = at_carrier.port_modes(Axis::Z, 13, 1).unwrap().remove(0);
    let waveform = if pulse {
        Waveform::pulse(frequency, 0.1).unwrap()
    } else {
        Waveform::Continuous {
            frequency,
            amplitude: c64::new(1.0, 0.0),
            ramp: 4.0 * 1.55,
        }
    };
    s.add_mode_source(&mode, Direction::Forward, waveform)
        .unwrap();
    // each frequency's field, and the source's amplitude in it
    let (frequencies, fields, amplitudes): (Vec<f64>, Vec<Vec<c64>>, Vec<c64>) = if pulse {
        let frequencies = vec![carrier - 0.05, carrier, carrier + 0.05];
        // the pulse's peak at 6 widths (22 µm/c), then 40 µm/c to leave the guide
        let steps = (70.0 / dt) as usize;
        let fields = dft_e(&mut s, steps, &frequencies);
        let amplitudes = frequencies
            .iter()
            .map(|&f| {
                0.5 * waveform.analytic_spectrum(
                    Field::E,
                    dt,
                    steps,
                    Frequency::natural(f).unwrap(),
                )
            })
            .collect();
        (frequencies, fields, amplitudes)
    } else {
        s.run(40 * steps_per_period);
        let field = steady_amplitude(&mut s, frequency, steps_per_period, 4);
        (vec![carrier], vec![field], vec![c64::new(1.0, 0.0)])
    };
    frequencies
        .iter()
        .zip(fields)
        .zip(amplitudes)
        .map(|((&f, values), amplitude)| {
            let solver = if f == carrier {
                &at_carrier
            } else {
                &lattice(f)
            };
            let behind = solver.port_modes(Axis::Z, 10, 1).unwrap().remove(0);
            let ahead = solver.port_modes(Axis::Z, 26, 1).unwrap().remove(0);
            let field = solver.field(values);
            let (_, backward) = field.mode_amplitudes(&behind);
            let forward = field.flux(Axis::Z, 26);
            (
                f,
                backward.norm_sqr() * behind.power() / forward,
                forward / (amplitude.norm_sqr() * ahead.power()),
            )
        })
        .collect()
}

/// What [`beam`] measures.
#[derive(Clone, Debug)]
pub(crate) struct BeamMeasured {
    /// At each distance from the plane (µm): the beam's width 2√⟨(y − ȳ)²⟩ of |E_z|² and its
    /// centre ȳ, across the beam's columns.
    pub(crate) columns: Vec<(f64, f64, f64)>,
    /// The largest |E_z| behind the plane relative to the largest ahead.
    // read by the tests
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) backward: f64,
}

/// A 2D Gaussian beam (E_z) of 1 µm in vacuum, w₀ = 1.5 µm, its focus 2 µm ahead of its plane,
/// tilted by `angle` (radians) towards y, launched by a continuous wave on cells of 50 nm
/// (32 steps a period, CPMLs of 10): its steady |E_z| across y at 2, 5, 8, 11 and 14 µm from
/// the plane.
pub(crate) fn beam(angle: f64) -> BeamMeasured {
    let h = 0.05;
    let n = [10 + 2 + 320 + 10, 460, 1];
    let g = Grid3d {
        nz: 1,
        ..grid(n, h)
    };
    let steps_per_period = 32;
    let courant = 1.0 / steps_per_period as f64 * 2.0f64.sqrt() / h;
    let mut s = Simulation::new(g, |_, _, _| 1.0, Boundaries::cpml_2d(10), courant).unwrap();
    let frequency = Frequency::natural(1.0).unwrap();
    let plane = 12;
    s.add_beam(
        &GaussianBeam {
            axis: Axis::X,
            plane,
            direction: crate::fdfd::Direction::Forward,
            centre: (0.0, 0.0),
            waist: 1.5,
            focus: 2.0,
            tilt: Axis::Y,
            angle,
            polarization: BeamPolarization::Perpendicular,
            eps: 1.0,
        },
        Waveform::Continuous {
            frequency,
            amplitude: c64::new(1.0, 0.0),
            ramp: 4.0,
        },
    )
    .unwrap();
    s.run(32 * steps_per_period);
    let amplitude = steady_amplitude(&mut s, frequency, steps_per_period, 2);
    let ez = |i: usize, j: usize| amplitude[g.index(Axis::Z, (i, j, 0))].norm();
    let columns = [2.0, 5.0, 8.0, 11.0, 14.0]
        .into_iter()
        .map(|d| {
            let i = plane + (d / h).round() as usize;
            let (mut total, mut first, mut second) = (0.0, 0.0, 0.0);
            for j in 0..g.ny {
                let y = g.node(Axis::Y, j);
                let p = ez(i, j).powi(2);
                total += p;
                first += p * y;
                second += p * y * y;
            }
            let centre = first / total;
            let variance = second / total - centre * centre;
            (d, 2.0 * variance.sqrt(), centre)
        })
        .collect();
    let (mut behind, mut ahead) = (0.0f64, 0.0f64);
    for j in 10..g.ny - 10 {
        for i in 10..g.nx - 10 {
            if i < plane {
                behind = behind.max(ez(i, j));
            } else {
                ahead = ahead.max(ez(i, j));
            }
        }
    }
    BeamMeasured {
        columns,
        backward: behind / ahead,
    }
}

/// The waist, the focus's distance from the plane and the divergence of a beam measured by
/// [`beam`]: w² = w₀² + θ² (d − d₀)², fitted by least squares to its columns' widths.
pub(crate) fn beam_fit(measured: &BeamMeasured) -> (f64, f64, f64) {
    // w² = c0 + c1 d + c2 d², the normal equations of the quadratic
    let mut a = [[0.0f64; 3]; 3];
    let mut rhs = [0.0f64; 3];
    for &(d, w, _) in &measured.columns {
        let powers = [1.0, d, d * d];
        for r in 0..3 {
            for c in 0..3 {
                a[r][c] += powers[r] * powers[c];
            }
            rhs[r] += powers[r] * w * w;
        }
    }
    // Cramer's rule
    let det = |m: [[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let whole = det(a);
    let coefficient = |k: usize| {
        let mut m = a;
        for r in 0..3 {
            m[r][k] = rhs[r];
        }
        det(m) / whole
    };
    let (c0, c1, c2) = (coefficient(0), coefficient(1), coefficient(2));
    let focus = -c1 / (2.0 * c2);
    ((c0 - c2 * focus * focus).sqrt(), focus, c2.sqrt())
}

/// The divergence 2√⟨(dk_x/dk_y)²⟩ of a 2D beam whose angular spectrum is |A(k_y)|² ∝
/// e^(−k_y² w₀²/2) (a Gaussian's, at any distance from its focus): its width's growth far from
/// the focus, which for a sum of plane waves is exactly w² = w₀² + θ² d². With `grid` the
/// step (µm) and time step of the grid, k_x(k_y) from its dispersion,
/// (2/Δ)² (sin²(k_xΔ/2) + sin²(k_yΔ/2)) = ω̃², ω̃ = (2/Δt) sin(ωΔt/2); without, the continuum's,
/// k_x = √(k² − k_y²). λ/(π w₀) is its paraxial limit.
pub(crate) fn beam_divergence(waist: f64, wavelength: f64, grid: Option<(f64, f64)>) -> f64 {
    let omega = std::f64::consts::TAU / wavelength;
    let slope = |ky: f64| -> Option<f64> {
        match grid {
            Some((h, dt)) => {
                let tilde = 2.0 / dt * (omega * dt / 2.0).sin();
                let s = (tilde * h / 2.0).powi(2) - (ky * h / 2.0).sin().powi(2);
                (s > 0.0).then(|| {
                    let kx = 2.0 / h * s.sqrt().asin();
                    (ky * h).sin() / (kx * h).sin()
                })
            }
            None => {
                let s = omega * omega - ky * ky;
                (s > 0.0).then(|| ky / s.sqrt())
            }
        }
    };
    // Simpson's rule over ±10/w₀, where the weight falls to e^(−50)
    let points = 20_000;
    let span = 10.0 / waist;
    let step = 2.0 * span / points as f64;
    let (mut weighted, mut total) = (0.0, 0.0);
    for i in 0..=points {
        let ky = -span + i as f64 * step;
        let factor = if i == 0 || i == points {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        let weight = (-ky * ky * waist * waist / 2.0).exp() * factor;
        if let Some(v) = slope(ky) {
            weighted += weight * v * v;
        }
        total += weight;
    }
    2.0 * (weighted / total).sqrt()
}

/// The slope d⟨y⟩/dx of a 2D beam's centre, tilted by `angle` from x and launched on the grid
/// of step `h` and time step `dt`: ⟨−dk_x/dk_y⟩ over its plane waves, the beam's own
/// e^(−κ²w₀²/2) across its axis (κ = k sin(φ − angle), φ a wave's direction, k = ω̃ as the
/// beam's profile has it), weighted on the plane by dκ/dk_y, and k_x(k_y) from the grid's
/// dispersion. tan(angle) is the continuum's paraxial limit.
pub(crate) fn beam_centre_slope(waist: f64, angle: f64, wavelength: f64, h: f64, dt: f64) -> f64 {
    let omega = std::f64::consts::TAU / wavelength;
    let tilde = 2.0 / dt * (omega * dt / 2.0).sin();
    let points = 20_000;
    let span = (10.0 / waist).min(0.999 * tilde);
    let step = 2.0 * span / points as f64;
    let (mut weighted, mut total) = (0.0, 0.0);
    for i in 0..=points {
        let kappa = -span + i as f64 * step;
        let phi = angle + (kappa / tilde).asin();
        let ky = tilde * phi.sin();
        let s = (tilde * h / 2.0).powi(2) - (ky * h / 2.0).sin().powi(2);
        if s <= 0.0 || phi.cos() <= 0.0 {
            continue;
        }
        let kx = 2.0 / h * s.sqrt().asin();
        let factor = if i == 0 || i == points {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        // |A(k_y)|² dk_y = |a(κ)|² (dκ/dk_y) dκ
        let jacobian = (phi - angle).cos() / phi.cos();
        let weight = (-kappa * kappa * waist * waist / 2.0).exp() * jacobian * factor;
        weighted += weight * (ky * h).sin() / (kx * h).sin();
        total += weight;
    }
    weighted / total
}

/// A plane-wave pulse along z in a column of vacuum (cells of `h` µm, periodic across): the
/// magnitude of its incident E's DFT inside the box at 1 c/µm relative to the waveform's own
/// spectrum. The sheet makes E the waveform to second order in the cells.
#[cfg(test)]
pub(crate) fn plane_wave_amplitude(h: f64) -> f64 {
    let cells = |x: f64| (x / h).round() as usize;
    let cpml = cells(0.5);
    let nz = 2 * cpml + cells(2.0);
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
        z: Edges::Pml {
            low: cpml,
            high: cpml,
        },
        cpml: Cpml::default(),
    };
    let mut s = Simulation::new(g, |_, _, _| 1.0, boundaries, 0.9).unwrap();
    let frequency = Frequency::natural(1.0).unwrap();
    let waveform = Waveform::pulse(frequency, 0.5).unwrap();
    let wave = s
        .add_plane_wave(PlaneWave {
            low: (0, 0, cpml + 2),
            high: (1, 1, nz - cpml - 2),
            direction: (0, 0, 1),
            polarization: [1.0, 0.0, 0.0],
            eps: 1.0,
            waveform,
        })
        .unwrap();
    let at = (0, 0, nz / 2);
    let steps = (20.0 / s.dt()) as usize;
    let mut dft = c64::new(0.0, 0.0);
    for _ in 0..steps {
        s.step();
        let w = c64::new(0.0, frequency.angular() * s.time()).exp() * s.dt();
        dft += s.incident(wave, Field::E, Axis::X, at) * w;
    }
    dft.norm() / waveform.spectrum(Field::E, s.dt(), steps, frequency).norm()
}
