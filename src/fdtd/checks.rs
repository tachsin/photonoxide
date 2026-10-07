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
