//! The monitors' checks: measurements the tests assert and the validation report reports.

use num_complex::Complex64 as c64;

use super::checks::{courant_for, dft_e, grid, guide};
use super::*;
use crate::fdfd::{Boundaries3d, Direction, Solver3d};
use crate::units::Wavelength;

/// The closed lossy box of `checks::spectrum_against_fdfd` (ε = 2.1, σ = 2/µm, 16 × 14 × 12
/// cells of 50 nm, walls) with a Gaussian pulse on one value of E_y, where the fields die away
/// within the run.
fn lossy_box() -> (Simulation, Waveform, (usize, usize, usize)) {
    let g = grid([16, 14, 12], 0.05);
    let pulse = Waveform::pulse(Frequency::natural(1.0 / 1.55).unwrap(), 0.3).unwrap();
    let mut s = Simulation::new(g, |_, _, _| 2.1, Boundaries::walls(), 0.9)
        .unwrap()
        .with_conductivity(|_, _, _| 2.0)
        .unwrap();
    let at = (9, 7, 6);
    s.add_source(Source {
        field: Field::E,
        component: Axis::Y,
        at,
        waveform: pulse,
    })
    .unwrap();
    (s, pulse, at)
}

/// The lossy box's E transformed by a monitor on a box of values and by hand
/// ([`dft_e`]) over the same run, at two frequencies: the largest difference relative to the
/// largest transform.
pub(crate) fn transforms_against_hand() -> f64 {
    let (mut s, _, _) = lossy_box();
    let mut by_hand = s.clone();
    let frequencies = [1.0 / 1.55, 1.0 / 1.55 + 0.12];
    let natural: Vec<Frequency> = frequencies
        .iter()
        .map(|&f| Frequency::natural(f).unwrap())
        .collect();
    let (low, high) = ((2, 3, 1), (13, 9, 10));
    let n = s.add_dft(low, high, &natural).unwrap();
    let steps = 600;
    s.run(steps);
    let hand = dft_e(&mut by_hand, steps, &frequencies);
    let g = s.grid();
    let (mut worst, mut largest) = (0.0f64, 0.0f64);
    for (f, values) in hand.iter().enumerate() {
        for c in Axis::ALL {
            for k in low.2..=high.2 {
                for j in low.1..=high.1 {
                    for i in low.0..=high.0 {
                        let a = s.dft(n).value(Field::E, c, (i, j, k), f).unwrap();
                        let b = values[g.index(c, (i, j, k))];
                        worst = worst.max((a - b).norm());
                        largest = largest.max(b.norm());
                    }
                }
            }
        }
    }
    worst / largest
}

/// The lossy box's flux through the plane halfway between nodes 10 and 11 along x (the whole
/// plane), and out of a box around the source, from the monitors' transforms divided by the
/// pulse's spectrum, against `Solver3d`'s field at the leapfrog's frequency for the source's
/// spectrum (ε + iσ cos(ωΔt/2)/ω̃): FDFD's [`crate::fdfd::Field3d::flux`] through the plane,
/// and what the box's faces give from FDFD's field. The largest difference relative to the
/// plane's flux.
pub(crate) fn flux_against_fdfd() -> f64 {
    let (mut s, pulse, at) = lossy_box();
    let frequency = Frequency::natural(1.0 / 1.55).unwrap();
    let plane = s
        .add_flux(FluxPlane::new(Axis::X, 10), &[frequency])
        .unwrap();
    let planes = [(6, 11), (4, 9), (3, 8)];
    let around = s.add_flux_box(planes, &[frequency]).unwrap();
    // the fields fall as e^(−σt/ε): to 1e-17 of their peak well within the run
    let steps = (60.0 / s.dt()) as usize;
    s.run(steps);
    let dt = s.dt();
    let spectrum = pulse.spectrum(Field::E, dt, steps, frequency);
    let g = s.grid();
    let tilde = s.leapfrog_wavenumber(frequency);
    let loss = 2.0 * (frequency.angular() * dt / 2.0).cos() / tilde;
    let solver = Solver3d::new(
        g,
        Wavelength::um(std::f64::consts::TAU / tilde).unwrap(),
        move |_, _, _| c64::new(2.1, loss),
        Boundaries3d::pml(0),
    )
    .unwrap();
    let mut j = vec![c64::new(0.0, 0.0); g.unknowns()];
    // FDFD for a unit current, FDTD divided by the pulse's spectrum
    j[g.index(Axis::Y, at)] = c64::new(1.0, 0.0);
    let fdfd = solver.solve(&j).unwrap();
    let scale = spectrum.norm_sqr();
    let through = s.flux(plane)[0] / scale;
    let expected = fdfd.flux(Axis::X, 10);
    // the box's faces from FDFD's field: its full planes' fluxes over the faces' windows equal
    // FDFD's fields' through a box only when the box is the whole grid, so compare the box
    // with the same faces built from FDFD's transforms instead
    let out = s.flux(around)[0] / scale;
    let from_fdfd = flux_box_of(&fdfd, planes, tilde);
    ((through - expected).abs()).max((out - from_fdfd).abs()) / expected.abs()
}

/// The flux out of the box between the half-planes after `planes[a].0` and `planes[a].1` of
/// FDFD's field `e` at k₀ = `k0`: each face's ½ Re(E × H̃*) with H̃ = ∇ × E/(ik₀) on the face,
/// the tangential E the mean of the nodes either side, the values on the faces' edges counting
/// half.
fn flux_box_of(e: &crate::fdfd::Field3d, planes: [(usize, usize); 3], k0: f64) -> f64 {
    let g = e.grid();
    let value = |c: Axis, at: [usize; 3]| e.values()[g.index(c, (at[0], at[1], at[2]))];
    // H̃_c at index `at` (half a step along the two axes other than c): (∇ × E)_c/(ik₀)
    let h = |c: Axis, at: [usize; 3]| {
        let (a, b) = c.others();
        let step = |axis: Axis, at: [usize; 3]| {
            let mut next = at;
            next[axis.index()] += 1;
            next
        };
        // (∇ × E)_c = ∂E_b/∂a − ∂E_a/∂b, with (a, b, c) cyclic
        let curl = (value(b, step(a, at)) - value(b, at)) / g.step(a)
            - (value(a, step(b, at)) - value(a, at)) / g.step(b);
        curl / c64::new(0.0, k0)
    };
    let weight = |(lo, hi): (usize, usize), m: usize, half: bool| -> f64 {
        if half {
            if lo < m && m < hi {
                1.0
            } else if m == lo || m == hi {
                0.5
            } else {
                0.0
            }
        } else if lo < m && m <= hi {
            1.0
        } else {
            0.0
        }
    };
    let mut total = 0.0;
    for axis in Axis::ALL {
        let (b, c) = axis.others();
        let (wb, wc) = (planes[b.index()], planes[c.index()]);
        for (plane, outward) in [
            (planes[axis.index()].0, -1.0),
            (planes[axis.index()].1, 1.0),
        ] {
            for (component, hc, sign) in [(b, c, 0.5), (c, b, -0.5)] {
                for v in wc.0..=wc.1 {
                    for u in wb.0..=wb.1 {
                        let w = weight(wb, u, component == b) * weight(wc, v, component == c);
                        if w == 0.0 {
                            continue;
                        }
                        let mut at = [0; 3];
                        at[axis.index()] = plane;
                        at[b.index()] = u;
                        at[c.index()] = v;
                        let low = value(component, at);
                        let hv = h(hc, at);
                        at[axis.index()] = plane + 1;
                        let high = value(component, at);
                        total += outward
                            * w
                            * sign
                            * (0.5 * (low + high) * hv.conj()).re
                            * g.step(b)
                            * g.step(c);
                    }
                }
            }
        }
    }
    total
}

/// A closed box (16 × 14 × 12 cells of 50 nm, walls) lossless inside a flux box between the
/// half-planes after nodes 5 and 10, 4 and 9, 3 and 8 (a block of ε = 4 in it) and lossy
/// outside it (σ = 3/µm from a cell beyond the box), a Gaussian pulse on one value of E_y
/// outside the box (`inside` false) or inside it, run until the fields have died away: the
/// flux out of the box, and the largest of its faces' fluxes in magnitude.
pub(crate) fn box_balance(inside: bool) -> (f64, f64) {
    let g = grid([16, 14, 12], 0.05);
    let planes = [(5usize, 10usize), (4, 9), (3, 8)];
    // lossless wherever a value the box weighs is: from the node before its first half-plane
    // to the node after its last
    let lossless = move |x: f64, y: f64, z: f64| {
        let p = [x, y, z];
        Axis::ALL.into_iter().all(|a| {
            let (lo, hi) = planes[a.index()];
            let (from, to) = (g.node(a, lo), g.node(a, hi + 1));
            p[a.index()] >= from - 1e-9 && p[a.index()] <= to + 1e-9
        })
    };
    let block = move |x: f64, y: f64, z: f64| {
        if x.abs() < 0.08 && y.abs() < 0.06 && z.abs() < 0.05 {
            4.0
        } else {
            1.0
        }
    };
    let mut s = Simulation::new(g, block, Boundaries::walls(), 0.9)
        .unwrap()
        .with_conductivity(move |x, y, z| if lossless(x, y, z) { 0.0 } else { 3.0 })
        .unwrap();
    let frequency = Frequency::natural(1.0 / 1.55).unwrap();
    let pulse = Waveform::pulse(frequency, 0.3).unwrap();
    let at = if inside { (8, 6, 5) } else { (2, 6, 5) };
    s.add_source(Source {
        field: Field::E,
        component: Axis::Y,
        at,
        waveform: pulse,
    })
    .unwrap();
    let frequencies = [frequency, Frequency::natural(1.0 / 1.55 + 0.15).unwrap()];
    let around = s.add_flux_box(planes, &frequencies).unwrap();
    let faces: Vec<usize> = Axis::ALL
        .into_iter()
        .flat_map(|axis| {
            let (b, c) = axis.others();
            let (lo, hi) = planes[axis.index()];
            [lo, hi].map(|p| (axis, p, planes[b.index()], planes[c.index()]))
        })
        .map(|(axis, p, wb, wc)| {
            s.add_flux(FluxPlane::new(axis, p).within(wb, wc), &frequencies)
                .unwrap()
        })
        .collect();
    s.run_until(200.0);
    let net = s
        .flux(around)
        .into_iter()
        .fold(0.0f64, |m, v| m.max(v.abs()));
    let largest = faces
        .iter()
        .flat_map(|&f| s.flux(f))
        .fold(0.0f64, |m, v| m.max(v.abs()));
    (net, largest)
}

/// The rectangular guide of `checks::guide` in a closed box of a lossy medium (σ = 4/µm, 16 ×
/// 14 × 24 cells of 50 nm, 64 steps a period of 1.55 µm), its fundamental mode launched
/// forward on plane 6 by a pulse 0.1 c/µm wide at 1.55 µm: the forward and backward amplitudes
/// on planes 4 and 16 from mode monitors, divided by half the pulse's analytic spectrum (the
/// source's amplitude at the carrier), against `Solver3d`'s for its own mode source of the
/// same mode, at the leapfrog's frequency with ε + iσ cos(ωΔt/2)/ω̃. The largest difference
/// relative to the largest amplitude.
pub(crate) fn modes_against_fdfd() -> f64 {
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
    let behind = solver.port_modes(Axis::Z, 4, 1).unwrap().remove(0);
    let ahead = solver.port_modes(Axis::Z, 16, 1).unwrap().remove(0);
    let pulse = Waveform::pulse(frequency, 0.1).unwrap();
    s.add_mode_source(&mode, Direction::Forward, pulse).unwrap();
    let monitor = s
        .add_mode_monitor(&[behind.clone(), ahead.clone()])
        .unwrap();
    // the pulse's peak at 6 widths (22 µm/c); after it the fields fall as e^(−σt/ε), in the
    // core (ε = 12) to 1e-15 in 105 µm/c
    let steps = (130.0 / dt) as usize;
    s.run(steps);
    let source = 0.5 * pulse.analytic_spectrum(Field::E, dt, steps, s.mode_frequencies(monitor)[0]);
    let fdfd = solver
        .solve_system(&solver.mode_source(&mode, Direction::Forward))
        .unwrap();
    let expected = [fdfd.mode_amplitudes(&behind), fdfd.mode_amplitudes(&ahead)];
    let measured = s.mode_amplitudes(monitor);
    let largest = expected
        .iter()
        .flat_map(|&(a, b)| [a.norm(), b.norm()])
        .fold(0.0f64, f64::max);
    expected
        .iter()
        .zip(&measured)
        .flat_map(|(&(a, b), &(c, d))| [(a - c / source).norm(), (b - d / source).norm()])
        .fold(0.0f64, f64::max)
        / largest
}

/// A 2D cavity with walls (24 × 18 cells of 50 nm, one cell along z, periodic there) of
/// ε = 2 and σ = 0.05/µm, E_z kicked by a pulse on one value and recorded on another after
/// the pulse: its resonances from 0.4 to 1.2 c/µm by harmonic inversion against the leapfrog's
/// own. The mode sin(mπi/nx) sin(nπj/ny) of E_z on the nodes has the curl-curl eigenvalue
/// λ = (4/Δ²)(sin²(mπ/2nx) + sin²(nπ/2ny)), and E's update, E ← ca E + cb ∇ × H̃ with
/// H̃ ← H̃ − Δt ∇ × E, makes its amplitude a sum of uⁿ with u² − (1 + ca − cb Δt λ) u + ca = 0.
/// Returns the number of modes found (error below 1e-8) and the largest |u − u_exact|, each
/// against the nearest exact u.
pub(crate) fn cavity_resonances() -> (usize, f64) {
    let (nx, ny, h) = (24usize, 18usize, 0.05);
    let g = Grid3d {
        nz: 1,
        ..grid([nx, ny, 1], h)
    };
    let boundaries = Boundaries {
        z: Edges::Bloch { k: 0.0 },
        ..Boundaries::walls()
    };
    let (eps, sigma) = (2.0, 0.05);
    let mut s = Simulation::new(g, |_, _, _| eps, boundaries, 0.9)
        .unwrap()
        .with_conductivity(|_, _, _| sigma)
        .unwrap();
    let pulse = Waveform::pulse(Frequency::natural(0.8).unwrap(), 0.8).unwrap();
    s.add_source(Source {
        field: Field::E,
        component: Axis::Z,
        at: (7, 5, 0),
        waveform: pulse,
    })
    .unwrap();
    let probe = s.add_probe(Field::E, Axis::Z, (16, 11, 0)).unwrap();
    let Waveform::Gaussian { width, delay, .. } = pulse else {
        unreachable!()
    };
    s.run_until(delay + 6.0 * width);
    let start = s.probe(probe).len();
    s.run(4000);
    let dt = s.dt();
    let signal: Vec<c64> = s.probe(probe)[start..]
        .iter()
        .map(|&v| c64::new(v, 0.0))
        .collect();
    let found = harmonic_inversion(
        &signal,
        dt,
        Frequency::natural(0.4).unwrap(),
        Frequency::natural(1.2).unwrap(),
    )
    .unwrap();
    let cb = dt / eps / (1.0 + sigma * dt / eps / 2.0);
    let ca = (1.0 - sigma * dt / eps / 2.0) / (1.0 + sigma * dt / eps / 2.0);
    let exact: Vec<c64> = (1..nx)
        .flat_map(|m| (1..ny).map(move |n| (m, n)))
        .map(|(m, n)| {
            let sin2 = |q: usize, cells: usize| {
                (std::f64::consts::PI * q as f64 / (2.0 * cells as f64))
                    .sin()
                    .powi(2)
            };
            let lambda = 4.0 / (h * h) * (sin2(m, nx) + sin2(n, ny));
            let b = 1.0 + ca - cb * dt * lambda;
            // the root with Im u < 0: e^(−iωΔt) for ω > 0
            let root = c64::new(b * b - 4.0 * ca, 0.0).sqrt();
            let u = 0.5 * (c64::new(b, 0.0) - root);
            if u.im < 0.0 {
                u
            } else {
                0.5 * (c64::new(b, 0.0) + root)
            }
        })
        .collect();
    let kept: Vec<&Resonance> = found.iter().filter(|r| r.error < 1e-8).collect();
    let worst = kept
        .iter()
        .map(|r| {
            let omega = c64::new(std::f64::consts::TAU * r.frequency, -r.decay);
            let u = (c64::new(0.0, -1.0) * omega * dt).exp();
            exact
                .iter()
                .map(|e| (e - u).norm())
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0f64, f64::max);
    (kept.len(), worst)
}

/// A slab of ε = 4, 0.5 µm thick, in vacuum along z (a 1D problem: one cell across x and y,
/// periodic there; 1 µm of vacuum and a CPML of 30 cells each side), on cells of `h` µm at
/// Courant number 0.5, E_x kicked by a pulse inside it and recorded there until the field has
/// decayed: its second resonance by harmonic inversion, against the continuum's. A slab of
/// index n and thickness L resonates where r² e^(2inkL) = 1, r = (n − 1)/(n + 1): ω =
/// (πm + i ln r)/(nL), Q = πm/(2 |ln r|) (1.0 c/µm and Q = 2.86 for m = 2). Returns the relative
/// errors of Re ω and of the decay rate.
pub(crate) fn slab_resonance(h: f64) -> (f64, f64) {
    let (n_index, thickness, gap, cpml) = (2.0, 0.5, 1.0, 30usize);
    let cells = ((thickness + 2.0 * gap) / h).round() as usize + 2 * cpml;
    let g = Grid3d {
        nx: 1,
        ny: 1,
        dx: h,
        dy: h,
        x0: 0.0,
        y0: 0.0,
        ..grid([1, 1, cells], h)
    };
    // the faces halfway between nodes, where no cell about an E_x straddles them
    let centre = 0.5 * h;
    let slab = move |_: f64, _: f64, z: f64| {
        if (z - centre).abs() < thickness / 2.0 {
            n_index * n_index
        } else {
            1.0
        }
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
    let mut s = Simulation::new(g, slab, boundaries, 0.5).unwrap();
    let pulse = Waveform::pulse(Frequency::natural(1.0).unwrap(), 1.0).unwrap();
    let middle = cells / 2;
    s.add_source(Source {
        field: Field::E,
        component: Axis::X,
        at: (0, 0, middle - (0.1 / h).round() as usize),
        waveform: pulse,
    })
    .unwrap();
    let probe = s
        .add_probe(
            Field::E,
            Axis::X,
            (0, 0, middle + (0.07 / h).round() as usize),
        )
        .unwrap();
    let Waveform::Gaussian { width, delay, .. } = pulse else {
        unreachable!()
    };
    s.run_until(delay + 6.0 * width);
    let start = s.probe(probe).len();
    let decayed = s.run_until_decayed(&[probe], 1e-28, 2.0, 200.0).unwrap();
    assert!(decayed, "the slab's field didn't decay");
    let dt = s.dt();
    let signal: Vec<c64> = s.probe(probe)[start..]
        .iter()
        .map(|&v| c64::new(v, 0.0))
        .collect();
    let found = harmonic_inversion(
        &signal,
        dt,
        Frequency::natural(0.75).unwrap(),
        Frequency::natural(1.25).unwrap(),
    )
    .unwrap();
    let r = (n_index - 1.0) / (n_index + 1.0);
    let omega = c64::new(2.0 * std::f64::consts::PI, r.ln()) / (n_index * thickness);
    let best = found
        .iter()
        .filter(|x| x.error < 1e-6)
        .min_by(|a, b| {
            (a.frequency - 1.0)
                .abs()
                .total_cmp(&(b.frequency - 1.0).abs())
        })
        .expect("the slab's second resonance");
    (
        (std::f64::consts::TAU * best.frequency - omega.re).abs() / omega.re,
        (best.decay + omega.im).abs() / -omega.im,
    )
}

/// What [`guide_2d`] measures: the transmission into the output guide's mode and the
/// reflection into the input guide's, by FDTD's mode monitors (divided by the source's
/// amplitude at the carrier) and by FDFD's `mode_amplitudes` for the same mode source.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Scattering {
    pub(crate) fdtd: [c64; 2],
    pub(crate) fdfd: [c64; 2],
}

/// A 2D guide (one cell along z, periodic there) of ε = 12 and 0.25 µm in air on cells of
/// 50 nm (70 × 70, CPMLs of 10, FDFD's PMLs in FDFD), straight along x or bent by 90° into
/// +y at a sharp corner (`bend`), its fundamental mode launched forward along x at 1.55 µm by
/// a pulse 0.1 c/µm wide: the transmission into the output's fundamental mode (along x on
/// plane 52, or along y on plane 52) and the reflection into the input's (behind the source,
/// on plane 15), at the carrier.
pub(crate) fn guide_2d(bend: bool) -> Scattering {
    use crate::fdfd::Edges as E;
    let (n, h, cpml) = (70usize, 0.05, 10usize);
    let g = Grid3d {
        nz: 1,
        ..grid([n, n, 1], h)
    };
    let (core, width, low, corner) = (12.0, 0.25, -0.8, 0.8);
    let eps = move |x: f64, y: f64, _: f64| {
        let along_x = (y - low).abs() < width / 2.0 && (!bend || x < corner + width / 2.0);
        let along_y = bend && (x - corner).abs() < width / 2.0 && y > low - width / 2.0;
        if along_x || along_y { core } else { 1.0 }
    };
    let pml = E::Pml {
        low: cpml,
        high: cpml,
    };
    let boundaries = Boundaries {
        x: pml,
        y: pml,
        z: E::Bloch { k: 0.0 },
        cpml: Cpml::default(),
    };
    let frequency = Frequency::natural(1.0 / 1.55).unwrap();
    let mut s = Simulation::new(g, eps, boundaries, 0.9).unwrap();
    let solver = Solver3d::new(
        g,
        s.fdfd_wavelength(frequency).unwrap(),
        move |x, y, z| c64::new(eps(x, y, z), 0.0),
        Boundaries3d {
            x: pml,
            y: pml,
            z: E::Bloch { k: 0.0 },
            ..Boundaries3d::pml(cpml)
        },
    )
    .unwrap();
    let (source, behind, out) = (18usize, 15usize, 52usize);
    let input = solver.port_modes(Axis::X, source, 1).unwrap().remove(0);
    let back = solver.port_modes(Axis::X, behind, 1).unwrap().remove(0);
    let output_axis = if bend { Axis::Y } else { Axis::X };
    let output = solver.port_modes(output_axis, out, 1).unwrap().remove(0);
    let pulse = Waveform::pulse(frequency, 0.1).unwrap();
    s.add_mode_source(&input, Direction::Forward, pulse)
        .unwrap();
    let monitor = s.add_mode_monitor(&[output.clone(), back.clone()]).unwrap();
    let mut at = [n / 2; 3];
    at[2] = 0;
    if bend {
        at[0] = ((corner - g.x0) / h) as usize;
    } else {
        at[1] = ((low - g.y0) / h) as usize;
    }
    at[output_axis.index()] = out + 3;
    let component = if input.e(Axis::Z, (n / 2, 0)).norm() > 0.0 {
        Axis::Z
    } else {
        output_axis.others().0
    };
    let probe = s
        .add_probe(Field::E, component, (at[0], at[1], at[2]))
        .unwrap();
    let decayed = s.run_until_decayed(&[probe], 1e-24, 10.0, 600.0).unwrap();
    assert!(decayed, "the guide's field didn't decay");
    let steps = s.steps();
    let carrier = s.mode_frequencies(monitor)[0];
    let amplitude = 0.5 * pulse.analytic_spectrum(Field::E, s.dt(), steps, carrier);
    let measured = s.mode_amplitudes(monitor);
    let fdfd = solver
        .solve_system(&solver.mode_source(&input, Direction::Forward))
        .unwrap();
    Scattering {
        fdtd: [measured[0].0 / amplitude, measured[1].1 / amplitude],
        fdfd: [
            fdfd.mode_amplitudes(&output).0,
            fdfd.mode_amplitudes(&back).1,
        ],
    }
}

/// Harmonic inversion of Σₖ dₖ e^(−iωₖnτ) without noise: 400 samples of 0.1 µm/c (the
/// Fourier transform's resolution 1/T = 0.025 c/µm), terms at 1.0 and 1.01 c/µm decaying at
/// 0.01 and 0.03 per µm/c, and one at 1.6 c/µm outside the window from 0.9 to 1.1: the largest
/// error of the two terms' frequencies and decay rates (c/µm and per µm/c) and of their
/// amplitudes relative to them, or infinity unless exactly two terms are found with an error
/// below 1e-6.
pub(crate) fn harmonic_synthetic() -> f64 {
    let (tau, n) = (0.1, 400);
    let terms = [
        (1.0, 0.01, c64::new(1.0, 0.5)),
        (1.01, 0.03, c64::new(-0.4, 0.8)),
        (1.6, 0.002, c64::new(2.0, 0.0)),
    ];
    let signal: Vec<c64> = (0..n)
        .map(|s| {
            let t = s as f64 * tau;
            terms
                .iter()
                .map(|&(f, decay, d)| {
                    d * c64::new(-decay * t, -std::f64::consts::TAU * f * t).exp()
                })
                .sum()
        })
        .collect();
    let found: Vec<Resonance> = harmonic_inversion(
        &signal,
        tau,
        Frequency::natural(0.9).unwrap(),
        Frequency::natural(1.1).unwrap(),
    )
    .unwrap()
    .into_iter()
    .filter(|r| r.error < 1e-6)
    .collect();
    if found.len() != 2 {
        return f64::INFINITY;
    }
    found
        .iter()
        .zip(&terms)
        .map(|(r, &(f, decay, d))| {
            (r.frequency - f)
                .abs()
                .max((r.decay - decay).abs())
                .max((r.amplitude - d).norm() / d.norm())
        })
        .fold(0.0f64, f64::max)
}
