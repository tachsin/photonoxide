//! Subpixel smoothing's checks: measurements the tests assert and the validation report reports.

use faer::Mat;
use faer::linalg::solvers::Solve;
use num_complex::Complex64 as c64;

use super::super::checks::{grid, noise};
use super::super::{Cpml, Current, Field, PlaneWave, Simulation, Source, Waveform};
use super::*;
use crate::fdfd::Edges;
use crate::units::Frequency;

/// A harmonic of a signal: Re(c e^(−iωt)), decaying at the rate γ.
#[derive(Clone, Copy, Debug)]
// the decay and amplitude are read by the tests
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct Harmonic {
    /// ω, in rad per unit of time.
    pub omega: f64,
    /// γ: e^(−γt).
    pub decay: f64,
    /// |c|, the largest over the signals.
    pub amplitude: f64,
}

/// The harmonics Σ Re(cₖ zₖⁿ) shared by real signals sampled every `dt`, zₖ = e^(−(iωₖ + γₖ)dt):
/// the matrix pencil (Y. Hua, T. K. Sarkar, IEEE Trans. Acoust. Speech Signal Process. 38, 814
/// (1990)), each signal's Hankel matrix of L + 1 columns, L a third of its length, stacked; their
/// right singular vectors V above 1e-10 of the largest, and the zₖ the eigenvalues of
/// V₁⁺V₂, V₁ and V₂ those vectors without their last and first rows. The cₖ by least squares.
pub(crate) fn harmonics(signals: &[Vec<f64>], dt: f64) -> Vec<Harmonic> {
    let n = signals[0].len();
    let l = n / 3;
    let rows = n - l;
    let y = Mat::<f64>::from_fn(rows * signals.len(), l + 1, |i, j| {
        signals[i / rows][i % rows + j]
    });
    let svd = y.thin_svd().expect("the SVD of a small dense matrix");
    let s = svd.S().column_vector();
    let m = (0..s.nrows()).filter(|&k| s[k] > 1e-10 * s[0]).count();
    let v = svd.V();
    let v1 = Mat::<f64>::from_fn(l, m, |i, k| v[(i, k)]);
    let v2 = Mat::<f64>::from_fn(l, m, |i, k| v[(i + 1, k)]);
    let z = (v1.transpose() * &v1)
        .partial_piv_lu()
        .solve(v1.transpose() * &v2);
    let poles = z
        .eigenvalues()
        .expect("the eigenvalues of a small dense matrix");
    // the amplitudes, by the normal equations of each signal's fit
    let k = poles.len();
    let gram = Mat::<c64>::from_fn(k, k, |a, b| {
        (0..n)
            .map(|t| (poles[a].conj() * poles[b]).powu(t as u32))
            .sum()
    });
    let lu = gram.partial_piv_lu();
    let mut amplitude = vec![0.0f64; k];
    for signal in signals {
        let rhs = Mat::<c64>::from_fn(k, 1, |a, _| {
            (0..n)
                .map(|t| poles[a].conj().powu(t as u32) * signal[t])
                .sum()
        });
        let c = lu.solve(rhs);
        for a in 0..k {
            amplitude[a] = amplitude[a].max(c[(a, 0)].norm());
        }
    }
    poles
        .iter()
        .zip(amplitude)
        .filter(|(z, _)| z.im < 0.0 || z.im == 0.0 && z.re > 0.0)
        .map(|(z, amplitude)| Harmonic {
            omega: -z.arg() / dt,
            decay: -z.norm().ln() / dt,
            // a real signal's pair: 2|c|
            amplitude: 2.0 * amplitude,
        })
        .collect()
}

/// Oskooi et al.'s 2D anisotropic lattice (their Fig. 2): ellipses of a tensor of principal
/// values 1.45, 2.81 and 4.98 in one of 8.49, 8.78 and 11.52, the principal axes ours (they
/// give none).
pub(crate) fn anisotropic_lattice(periods: [usize; 2]) -> Structure {
    let a = Permittivity::principal([1.45, 2.81, 4.98], rotation([0.3, 1.1, 0.7])).unwrap();
    let b = Permittivity::principal([8.49, 8.78, 11.52], rotation([1.2, 0.4, 2.1])).unwrap();
    let mut s = Structure::new(b);
    for i in 0..periods[0] {
        for j in 0..periods[1] {
            let centre = [i as f64 + 0.5, j as f64 + 0.5];
            let ellipse = Body::ellipse(centre, [0.355, 0.305], 30f64.to_radians()).unwrap();
            s = s.with(ellipse, a);
        }
    }
    s
}

/// The rotation by Euler angles (z, x, z), its rows the rotated axes.
pub(crate) fn rotation([alpha, beta, gamma]: [f64; 3]) -> [[f64; 3]; 3] {
    let z = |t: f64| {
        let (s, c) = t.sin_cos();
        [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]]
    };
    let x = |t: f64| {
        let (s, c) = t.sin_cos();
        [[1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c]]
    };
    let product = |a: [[f64; 3]; 3], b: [[f64; 3]; 3]| -> [[f64; 3]; 3] {
        std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
    };
    product(z(gamma), product(x(beta), z(alpha)))
}

/// A run for the modes of a lattice of period 1 µm, periodic (k = 0) on a box of `periods`
/// periods along x, y and z (0 along z: a 2D lattice, one cell thick).
pub(crate) struct Lattice<'a> {
    pub structure: &'a Structure,
    pub periods: [usize; 3],
    /// Point currents on these components of E, at three points.
    pub sources: &'a [Axis],
    /// A magnetic current M_z ∝ cos(2π k·r) on every value of H̃_z, k (1/µm): only the modes
    /// of that wave vector and those it folds onto.
    pub wave: Option<[f64; 2]>,
    /// The pulse's carrier and width, c/µm.
    pub carrier: f64,
    pub bandwidth: f64,
    /// How long the field is recorded after the pulse, µm/c.
    pub time: f64,
}

impl Lattice<'_> {
    /// The harmonics of E_z, H̃_z and E_x at three points on `n` cells a period, by
    /// `smoothing`, the Courant number 0.99, sampled 16 times a period of the carrier.
    pub(crate) fn modes(&self, smoothing: Smoothing, n: usize) -> Vec<Harmonic> {
        let h = 1.0 / n as f64;
        let [px, py, pz] = self.periods;
        let size = [n * px, n * py, (n * pz).max(1)];
        let grid = Grid3d {
            nx: size[0],
            ny: size[1],
            nz: size[2],
            dx: h,
            dy: h,
            dz: h,
            x0: 0.0,
            y0: 0.0,
            z0: 0.0,
        };
        let p = Edges::Bloch { k: 0.0 };
        let boundaries = Boundaries {
            x: p,
            y: p,
            z: p,
            cpml: Cpml::default(),
        };
        let mut s =
            Simulation::smoothed(grid, self.structure, smoothing, boundaries, 0.99).unwrap();
        // points at fixed fractions of the box, whatever n
        let at = |f: [f64; 3]| {
            let c = |a: usize| ((f[a] * size[a] as f64) as usize).min(size[a] - 1);
            (c(0), c(1), c(2))
        };
        let waveform =
            Waveform::pulse(Frequency::natural(self.carrier).unwrap(), self.bandwidth).unwrap();
        for (component, f) in [
            (Axis::X, [0.13, 0.71, 0.37]),
            (Axis::Y, [0.62, 0.27, 0.81]),
            (Axis::Z, [0.83, 0.58, 0.22]),
        ] {
            if self.sources.contains(&component) {
                s.add_source(Source {
                    field: Field::E,
                    component,
                    at: at(f),
                    waveform,
                })
                .unwrap();
            }
        }
        if let Some(k) = self.wave {
            let values = (0..grid.cells())
                .map(|r| {
                    let ijk = (
                        r % size[0],
                        (r / size[0]) % size[1],
                        r / (size[0] * size[1]),
                    );
                    let p = grid.h_position(Axis::Z, ijk);
                    let phase = std::f64::consts::TAU * (k[0] * p[0] + k[1] * p[1]);
                    (Axis::Z, ijk, c64::new(phase.cos(), 0.0))
                })
                .collect();
            s.add_current(Current {
                field: Field::H,
                values,
                waveform,
            })
            .unwrap();
        }
        let probes: Vec<usize> = [
            (Field::E, Axis::Z, [0.31, 0.12, 0.64]),
            (Field::H, Axis::Z, [0.77, 0.43, 0.09]),
            (Field::E, Axis::X, [0.21, 0.89, 0.47]),
        ]
        .into_iter()
        .map(|(field, component, f)| s.add_probe(field, component, at(f)).unwrap())
        .collect();
        let Waveform::Gaussian { delay, .. } = waveform else {
            unreachable!("a pulse is a Gaussian")
        };
        let start = (2.0 * delay / s.dt()).ceil() as usize;
        let steps = ((2.0 * delay + self.time) / s.dt()).ceil() as usize;
        s.run(steps);
        let every = ((1.0 / self.carrier / 16.0 / s.dt()).floor() as usize).max(1);
        let signals: Vec<Vec<f64>> = probes
            .iter()
            .map(|&q| s.probe(q)[start..].iter().step_by(every).copied().collect())
            .collect();
        harmonics(&signals, every as f64 * s.dt())
    }

    /// The frequency (c/µm) of the harmonic nearest `target` on `n` cells a period.
    pub(crate) fn frequency(&self, smoothing: Smoothing, n: usize, target: f64) -> f64 {
        nearest(&self.modes(smoothing, n), target)
    }
}

/// The frequency of the harmonic nearest `target`, c/µm.
pub(crate) fn nearest(modes: &[Harmonic], target: f64) -> f64 {
    modes
        .iter()
        .map(|m| m.omega / std::f64::consts::TAU)
        .min_by(|a, b| (a - target).abs().total_cmp(&(b - target).abs()))
        .unwrap_or(f64::NAN)
}

/// A slab of `eps` in vacuum from z = 1.0137 µm, 0.3 µm thick, its faces off the grid, lit at
/// normal incidence along z by an x-polarized pulse in a column one cell across (periodic),
/// cells of `h` µm along z; its reflection read from the scattered field in front of the
/// total-field/scattered-field box: r_xx and r_yx at 0.8, 1 and 1.2 c/µm, the reflected field
/// over the incident one at the slab's face. Returns each frequency's (r_xx, r_yx).
pub(crate) fn slab_reflection(h: f64, eps: Permittivity, smoothing: Smoothing) -> Vec<[c64; 2]> {
    let (za, thickness) = (1.0137, 0.3);
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
    let (low, high) = (cpml + cells(0.2), nz - cpml - cells(0.2));
    let vacuum = Permittivity::isotropic(1.0).unwrap();
    let slab = Body::ellipsoid(
        [0.0, 0.0, za + thickness / 2.0],
        [f64::INFINITY, f64::INFINITY, thickness / 2.0],
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    )
    .unwrap();
    let structure = Structure::new(vacuum).with(slab, eps);
    let boundaries = Boundaries {
        x: Edges::Bloch { k: 0.0 },
        y: Edges::Bloch { k: 0.0 },
        z: Edges::Pml {
            low: cpml,
            high: cpml,
        },
        cpml: Cpml::default(),
    };
    let courant = 0.9;
    let mut s = Simulation::smoothed(g, &structure, smoothing, boundaries, courant).unwrap();
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
    let front = low - cells(0.1);
    let inside = low + 2;
    let frequencies = [0.8, 1.0, 1.2];
    let mut scattered = [[c64::new(0.0, 0.0); 2]; 3];
    let mut incident = [c64::new(0.0, 0.0); 3];
    let steps = (40.0 / s.dt()) as usize;
    for _ in 0..steps {
        s.step();
        let t = s.time();
        let ei = s.incident(wave, Field::E, Axis::X, (0, 0, inside));
        for (q, f) in frequencies.iter().enumerate() {
            let w = c64::new(0.0, std::f64::consts::TAU * f * t).exp();
            scattered[q][0] += s.e(Axis::X)[front] * w;
            scattered[q][1] += s.e(Axis::Y)[front] * w;
            incident[q] += ei * w;
        }
    }
    let dt = s.dt();
    frequencies
        .iter()
        .enumerate()
        .map(|(q, f)| {
            // the grid's own wavenumber in vacuum: sin(kh/2)/h = sin(ωΔt/2)/Δt
            let omega = std::f64::consts::TAU * f;
            let k = 2.0 / h * ((h / dt) * (omega * dt / 2.0).sin()).asin();
            let (zp, zi) = (front as f64 * h, inside as f64 * h);
            let shift =
                c64::new(0.0, k * (zi - zp)).exp() * c64::new(0.0, -2.0 * k * (za - zp)).exp();
            [0, 1].map(|c| scattered[q][c] / incident[q] * shift)
        })
        .collect()
}

/// The exact reflection of the same slab at normal incidence, r_xx and r_yx at each of the three
/// frequencies: at normal incidence D_z = 0 in the slab, so E_x and E_y see the 2 × 2 tensor
/// ε_t = ε_tt − ε_tz ε_zt/ε_zz (τ's transverse block), and along each of its principal axes the
/// slab is Airy's, of index √λ.
pub(crate) fn slab_exact(eps: Permittivity) -> Vec<[c64; 2]> {
    let e = eps.matrix();
    let t = |i: usize, j: usize| e[i][j] - e[i][2] * e[2][j] / e[2][2];
    let (a, b, c) = (t(0, 0), t(0, 1), t(1, 1));
    // the principal axes of [[a, b], [b, c]]
    let mean = 0.5 * (a + c);
    let radius = (0.25 * (a - c) * (a - c) + b * b).sqrt();
    let angle = 0.5 * (2.0 * b).atan2(a - c);
    let (sn, cs) = angle.sin_cos();
    let thickness = 0.3;
    [0.8, 1.0, 1.2]
        .iter()
        .map(|f| {
            let airy = |lambda: f64| {
                let n = lambda.sqrt();
                let r12 = (1.0 - n) / (1.0 + n);
                let phase = c64::new(0.0, 2.0 * n * std::f64::consts::TAU * f * thickness).exp();
                r12 * (1.0 - phase) / (1.0 - r12 * r12 * phase)
            };
            let (r1, r2) = (airy(mean + radius), airy(mean - radius));
            // R = r1 u uᵀ + r2 v vᵀ, u = (cos, sin), v = (−sin, cos)
            [r1 * cs * cs + r2 * sn * sn, (r1 - r2) * cs * sn]
        })
        .collect()
}

/// The largest difference between the measured and the exact reflection.
pub(crate) fn slab_error(h: f64, eps: Permittivity, smoothing: Smoothing) -> f64 {
    let measured = slab_reflection(h, eps, smoothing);
    let exact = slab_exact(eps);
    measured
        .iter()
        .zip(&exact)
        .flat_map(|(m, e)| [(m[0] - e[0]).norm(), (m[1] - e[1]).norm()])
        .fold(0.0, f64::max)
}

/// The anisotropic slab's tensor: principal values 2, 3 and 4.5 along axes turned from the
/// grid's, so that every entry is non-zero.
pub(crate) fn tilted_crystal() -> Permittivity {
    Permittivity::principal([2.0, 3.0, 4.5], rotation([0.4, 0.9, 1.3])).unwrap()
}

/// Layers of ε = 12 and 1, half a period each, the period Λ = 1/√5 µm along n = (1, 2)/√5, offset
/// so that no interface meets a node: periodic on a square of 1 µm, and at an angle to the grid.
pub(crate) fn tilted_layers([p, q]: [f64; 2]) -> Structure {
    let l = p.hypot(q);
    let period = 1.0 / l;
    let n = [p / l, q / l, 0.0];
    let t = [q / l, -p / l, 0.0];
    let mut s = Structure::new(Permittivity::isotropic(1.0).unwrap());
    for j in -2..=4 {
        let centre = (0.137 + j as f64 + 0.25) * period;
        let layer = Body::ellipsoid(
            n.map(|v| v * centre),
            [period / 4.0, f64::INFINITY, f64::INFINITY],
            [n, t, [0.0, 0.0, 1.0]],
        )
        .unwrap();
        s = s.with(layer, Permittivity::isotropic(12.0).unwrap());
    }
    s
}

/// The exact frequency (c/µm) near `guess` of the layers' mode with H̃ along z, travelling along
/// them at β = 2π/Λ (k = (2, −1) 2π on the square) and in phase from period to period: the root
/// of ½ tr(M₁M₂) = 1, Mᵢ each layer's transfer matrix of (H̃_z, ε⁻¹ ∂H̃_z/∂s), by secant steps.
pub(crate) fn tilted_layers_exact([p, q]: [f64; 2], guess: f64) -> f64 {
    let period = 1.0 / p.hypot(q);
    let beta = std::f64::consts::TAU / period;
    let trace = |f: f64| {
        let omega = std::f64::consts::TAU * f;
        let layer = |eps: f64, d: f64| {
            let k = c64::new(eps * omega * omega - beta * beta, 0.0).sqrt();
            let (c, s) = ((k * d).cos(), (k * d).sin());
            [[c, s / k * eps], [-k / eps * s, c]]
        };
        let (a, b) = (layer(12.0, period / 2.0), layer(1.0, period / 2.0));
        let t = a[0][0] * b[0][0] + a[0][1] * b[1][0] + a[1][0] * b[0][1] + a[1][1] * b[1][1];
        0.5 * t.re - 1.0
    };
    // the sign change nearest the guess, within 5 %, then bisection
    let steps = 2000;
    let at = |s: usize| guess * (0.95 + 0.1 * s as f64 / steps as f64);
    let (mut low, mut high) = (0..steps)
        .filter(|&s| trace(at(s)).signum() != trace(at(s + 1)).signum())
        .map(|s| (at(s), at(s + 1)))
        .min_by(|a, b| (a.0 - guess).abs().total_cmp(&(b.0 - guess).abs()))
        .unwrap_or((f64::NAN, f64::NAN));
    for _ in 0..100 {
        let mid = 0.5 * (low + high);
        if trace(mid).signum() == trace(low).signum() {
            low = mid;
        } else {
            high = mid;
        }
    }
    0.5 * (low + high)
}

/// The relative error of the tilted layers' (n = (1, 2)/√5) lowest mode along them, on `n`
/// cells a micrometre with `smoothing`, against [`tilted_layers_exact`].
pub(crate) fn oblique(smoothing: Smoothing, n: usize) -> f64 {
    let structure = tilted_layers([1.0, 2.0]);
    let lattice = Lattice {
        structure: &structure,
        periods: [1, 1, 0],
        sources: &[],
        wave: Some([2.0, -1.0]),
        carrier: 0.88,
        bandwidth: 0.2,
        time: 30.0,
    };
    let exact = tilted_layers_exact([1.0, 2.0], 0.8857);
    (lattice.frequency(smoothing, n, exact) - exact) / exact
}

/// Oskooi et al.'s anisotropic lattice's lowest mode at k = (½, 0) 2π/µm (two periods along x),
/// c/µm, on `n` cells a period with `smoothing`.
pub(crate) fn oskooi(smoothing: Smoothing, n: usize) -> f64 {
    let structure = anisotropic_lattice([2, 1]);
    let lattice = Lattice {
        structure: &structure,
        periods: [2, 1, 0],
        sources: &[Axis::X, Axis::Y, Axis::Z],
        wave: Some([0.5, 0.0]),
        carrier: 0.16,
        bandwidth: 0.03,
        time: 100.0,
    };
    lattice.frequency(smoothing, n, 0.1594)
}

/// A closed box with an anisotropic ellipsoid at an angle to the grid and random D: the
/// leapfrog's ½ Σ E·D + ½ Σ H̃⁻·H̃⁺ over `steps` steps, its largest change relative to it.
pub(crate) fn tensor_energy_drift(coupling: Coupling, steps: usize) -> f64 {
    let g = grid([10, 9, 8], 0.05);
    let ellipsoid = Body::ellipsoid(
        [0.02, -0.01, 0.0],
        [0.17, 0.12, 0.1],
        rotation([0.3, 0.7, 0.2]),
    )
    .unwrap();
    let structure =
        Structure::new(Permittivity::isotropic(2.0).unwrap()).with(ellipsoid, tilted_crystal());
    let mut s = Simulation::smoothed(
        g,
        &structure,
        Smoothing {
            coupling,
            ..Smoothing::default()
        },
        Boundaries::walls(),
        0.95,
    )
    .unwrap();
    for c in 0..3 {
        for r in 0..g.cells() {
            s.anisotropic.as_mut().unwrap().d[c][r] = noise(r + 7 * c);
            s.h[c][r] = noise(r + 13 * c + 100_000);
        }
    }
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
    (0..steps)
        .map(|_| (energy(&mut s) - first).abs() / first)
        .fold(0.0, f64::max)
}

/// An isotropic ellipsoid of ε = `contrast` at an angle to the grid, in vacuum, its smoothed
/// ε̃⁻¹'s off-diagonal entries at the nodes, in a closed box (20 × 18 × 16 cells of 50 nm),
/// stepped `steps` times at Courant number 0.99 from random D and H: the largest ‖E‖ over the
/// run relative to the first, or `None` where [`Simulation::smoothed`] refuses the ellipsoid,
/// its ε̃⁻¹ not positive definite; or without that check when not `checked`.
pub(crate) fn contrast_growth(contrast: f64, steps: usize, checked: bool) -> Option<f64> {
    let g = grid([20, 18, 16], 0.05);
    let ellipsoid = Body::ellipsoid(
        [0.02, -0.01, 0.0],
        [0.31, 0.22, 0.19],
        rotation([0.3, 0.7, 0.2]),
    )
    .unwrap();
    let structure = Structure::new(Permittivity::isotropic(1.0).unwrap())
        .with(ellipsoid, Permittivity::isotropic(contrast).unwrap());
    let build = if checked {
        Simulation::smoothed
    } else {
        Simulation::smoothed_unchecked
    };
    let mut s = build(
        g,
        &structure,
        Smoothing::default(),
        Boundaries::walls(),
        0.99,
    )
    .ok()?;
    for c in 0..3 {
        for r in 0..g.cells() {
            s.anisotropic.as_mut().unwrap().d[c][r] = noise(r + 7 * c);
            s.h[c][r] = noise(r + 13 * c + 100_000);
        }
    }
    let size = |s: &Simulation| {
        Axis::ALL
            .iter()
            .map(|&c| s.e(c).iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>()
            .sqrt()
    };
    s.run(1);
    let first = size(&s);
    let mut largest: f64 = 1.0;
    for _ in 0..steps.div_ceil(100) {
        s.run(100);
        let ratio = size(&s) / first;
        if !ratio.is_finite() {
            return Some(f64::INFINITY);
        }
        largest = largest.max(ratio);
    }
    Some(largest)
}
