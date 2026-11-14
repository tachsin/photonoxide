//! The validation cases of the mode solvers' and RCWA's derivatives (0.7, #761's I9): a slab's
//! dn_eff/dt by the profile solver's adjoint against the dispersion relation's implicit
//! derivative, the profile's and the full-vector solver's gradients against differences, RCWA's
//! forward mode against the transfer matrices' exact derivatives and against differences, and a
//! weak grating's first-order efficiency against the Born approximation.

use num_complex::Complex64 as c64;

use super::Polarization;
use super::multilayer::Multilayer;
use super::rcwa::{Diffraction, Grating, GratingParameter, Layer};
use super::slab::Slab;
use super::slab_fd::Profile;
use super::vector::{Boundaries, Boundary, CrossSection, Permittivity, Pml, modes};
use crate::portable;
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid problem";

fn wl(um: f64) -> Wavelength {
    Wavelength::um(um).expect(OK)
}

fn re(n: f64) -> c64 {
    c64::new(n, 0.0)
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "mode/slab-index-derivative",
            title: r"The book's slab (220 nm of 3.473 in 1.444 at 1.55 µm), TE and TM: $dn_\mathrm{eff}/dt$ by the 1D profile solver's adjoint (the derivative in the upper interface's node, nodes on both interfaces, a 4.02 µm window at 2.5 nm) against the characteristic equation's implicit derivative, $-(\partial C/\partial t)/(\partial C/\partial n_\mathrm{eff})$ (larger relative difference shown)",
            tier: Tier::Analytic,
            source: r"A. Yariv, P. Yeh, Photonics, 6th ed. (2007), Eqs. 3.2-5 and 3.2-11, as mode::slab solves them, differentiated by the implicit function theorem; the profile's adjoint: the eigenvalue's derivative $y^{\mathsf T} dM x / y^{\mathsf T} x$ with the left eigenvector $y = Wx$ of $M = W^{-1}H$",
            run: slab_index_derivative,
        },
        Case {
            id: "mode/profile-index-gradient",
            title: r"A graded, lossy profile with PMLs (160 uneven nodes, an asymmetric core of $\varepsilon = 11 + 2x + 0.01i$), TE and TM: the fundamental mode's $dn_\mathrm{eff}/d\varepsilon$ at five cells and $dn_\mathrm{eff}/dx$ at five nodes by the adjoint, against fourth-order differences ($\delta = 10^{-3}$ and $10^{-4}$ µm) (largest difference relative to the largest derivative shown)",
            tier: Tier::Analytic,
            source: r"the first-order perturbation of a simple eigenvalue, $d\lambda = y^{\mathsf T} dM x / y^{\mathsf T} x$, with $dM$ the rows' own expressions on dual numbers (mode::slab_fd); Hellmann–Feynman's form in the Hermitian case",
            run: profile_index_gradient,
        },
        Case {
            id: "mode/vector-index-gradient",
            title: r"The full-vector solver on an asymmetric, lossy ridge (12 + 0.02i on 2.085, an electric wall on one side and PMLs on two): the fundamental mode's $dn_\mathrm{eff}/d\varepsilon$ at five cells (core, its edge, cladding, substrate, PML) by the adjoint with the left eigenvector, against fourth-order differences ($\delta = 10^{-3}$) (largest difference relative to the largest derivative shown)",
            tier: Tier::Analytic,
            source: r"A. B. Fallahkhair, K. S. Li, T. E. Murphy, J. Lightwave Technol. 26, 1423 (2008), doi:10.1109/JLT.2008.923643, Eqs. 21–36, differentiated on dual numbers; $d\lambda = y^{\mathsf T} dA x / y^{\mathsf T} x$ with $A^{\mathsf T} y = \lambda y$",
            run: vector_index_gradient,
        },
        Case {
            id: "mode/rcwa-tangent-multilayer",
            title: r"Three lossless films (2.0, 300 nm; 1.5, 200 nm; 3.5, 150 nm) between air and 1.45 at 1 µm, TE and TM at 0 and 0.5 rad, as an RCWA stack with 11 harmonics: the reflectance's and transmittance's derivatives in each film's permittivity and thickness by forward mode, against the transfer matrices' exact derivatives (largest difference shown)",
            tier: Tier::Analytic,
            source: r"mode::multilayer's reflection_derivatives (the transfer matrices' product rule, exact), $dR/d\varepsilon = (dR/dn)/2n$; T. W. Hughes et al., ACS Photonics 6, 3010 (2019), doi:10.1021/acsphotonics.9b01238 (forward-mode differentiation)",
            run: rcwa_tangent_multilayer,
        },
        Case {
            id: "mode/rcwa-tangent-gradient",
            title: r"Two lamellar layers (one lossy, three stripes) and a film on glass, 700 nm period at 1.3 µm and 0.2 rad, 17 harmonics, TE and TM: the reflected and transmitted efficiencies of orders −1, 0 and 1 differentiated in three stripes' permittivities and two thicknesses by forward mode, against fourth-order differences ($\delta = 10^{-4}$) (largest difference shown)",
            tier: Tier::Analytic,
            source: r"forward-mode differentiation (Hughes et al. 2019) of every step of mode::rcwa: the Toeplitz matrices, the eigenpairs ($d\lambda_i = C_{ii}$, $dW = WD$, $D_{ij} = C_{ij}/(\lambda_j - \lambda_i)$, $C = W^{-1} dM W$), the interfaces and the star products",
            run: rcwa_tangent_gradient,
        },
        Case {
            id: "mode/rcwa-tangent-born",
            title: r"A weak grating in its own medium (1.5; stripes of $\varepsilon = 2.25 + \Delta$ filling 0.4 of a 1.5 µm period, 300 nm thick) at 1 µm and 0.2 rad, TE, 17 harmonics: the first transmitted order's $d\eta_1/d\Delta$ by forward mode against the Born approximation's $2\eta_1/\Delta$ at $\Delta = 2.5 \times 10^{-3}$ (relative difference shown)",
            tier: Tier::Analytic,
            source: r"the first Born approximation: $t_1 = (i k_0^2 \varepsilon_1 / 2k_{z1}) \int_0^d e^{i(k_{z0} - k_{z1})z} dz$ with $\lvert\varepsilon_1\rvert = \Delta \sin(\pi f)/\pi$, so $\eta_1 = \lvert t_1\rvert^2 k_{z1}/k_{z0}$ grows as $\Delta^2$ and its derivative is $2\eta_1/\Delta$, to first order in $\Delta$",
            run: rcwa_tangent_born,
        },
    ]
}

fn zero(error: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: error,
        expected: 0.0,
        tolerance,
        error,
    }
}

/// The fourth-order central difference of `f` at 0 with step `h`.
fn differences<T>(f: &dyn Fn(f64) -> T, h: f64, read: &dyn Fn(&T) -> c64) -> c64 {
    let (a, b, c, d) = (f(h), f(-h), f(2.0 * h), f(-2.0 * h));
    (8.0 * (read(&a) - read(&b)) - (read(&c) - read(&d))) / (12.0 * h)
}

/// The book's slab as a profile at spacing `h`, nodes on its interfaces.
fn book(h: f64) -> Profile {
    Profile::uniform(-2.01, 2.01, (4.02 / h).round() as usize, |x| {
        let n: f64 = if x.abs() < 0.11 { 3.473 } else { 1.444 };
        c64::new(n * n, 0.0)
    })
    .expect(OK)
}

/// The relative error of the profile's adjoint dn_eff/dt at spacing `h`, TE and TM.
pub(crate) fn slab_derivative_error(h: f64) -> [f64; 2] {
    let p = book(h);
    let upper = ((0.11 + 2.01) / h).round() as usize;
    let slab = Slab::new(1.444, 3.473, 1.444, Length::nm(220.0)).expect(OK);
    [(Polarization::Te, 2.8), (Polarization::Tm, 2.0)].map(|(pol, near)| {
        let mode = &p.modes(pol, wl(1.55), 1, Some(near)).expect(OK)[0];
        let g = p.index_gradient(pol, wl(1.55), mode).expect(OK);
        let exact = slab.modes(pol, wl(1.55))[0].derivatives().thickness;
        (g.nodes[upper - 1].re - exact).abs() / exact.abs()
    })
}

fn slab_index_derivative() -> Outcome {
    let [te, tm] = slab_derivative_error(0.0025);
    zero(te.max(tm), 2e-5)
}

/// A leaky, lossy, graded profile on uneven nodes, with PMLs.
fn graded() -> Profile {
    let nodes: Vec<f64> = (0..=160)
        .map(|i| {
            let t = f64::from(i) / 160.0;
            -2.0 + 4.0 * t + 0.15 * portable::sin(std::f64::consts::PI * t)
        })
        .collect();
    let cells = nodes
        .windows(2)
        .map(|w| {
            let x = 0.5 * (w[0] + w[1]);
            if (-0.2..0.15).contains(&x) {
                c64::new(11.0 + 2.0 * x, 0.01)
            } else if x > 0.15 {
                re(2.1)
            } else {
                re(4.0)
            }
        })
        .collect();
    Profile::new(nodes, cells)
        .expect(OK)
        .with_pml(0.4, 0.4, 2.0)
        .expect(OK)
}

/// The largest difference between the adjoint and the differences at `delta`, and between the
/// differences at `delta` and at twice it (what they resolve), both relative to the largest
/// derivative.
struct Agreement {
    error: f64,
    resolution: f64,
}

/// The profile's agreement with its differences.
pub(crate) fn profile_agreement() -> (f64, f64) {
    let p = graded();
    let mut a = Agreement {
        error: 0.0,
        resolution: 0.0,
    };
    for pol in [Polarization::Te, Polarization::Tm] {
        let index = |q: &Profile| q.modes(pol, wl(1.55), 1, Some(3.2)).expect(OK)[0].effective_index;
        let mode = &p.modes(pol, wl(1.55), 1, Some(3.2)).expect(OK)[0];
        let g = p.index_gradient(pol, wl(1.55), mode).expect(OK);
        let top = |v: &[c64]| v.iter().map(|x| portable::complex::abs(*x)).fold(0.0, f64::max);
        let (cells_top, nodes_top) = (top(&g.cells), top(&g.nodes));
        for c in [0, 45, 80, 100, 159] {
            let f = |d: f64| index(&p.with_cell_changed(c, d));
            let one = differences(&f, 1e-3, &|v| *v);
            let two = differences(&f, 2e-3, &|v| *v);
            a.error = a.error.max(portable::complex::abs(one - g.cells[c]) / cells_top);
            a.resolution = a.resolution.max(portable::complex::abs(one - two) / cells_top);
        }
        for m in [1, 40, 75, 120, 159] {
            let f = |d: f64| index(&p.with_node_moved(m, d));
            let one = differences(&f, 1e-4, &|v| *v);
            let two = differences(&f, 2e-4, &|v| *v);
            a.error = a.error.max(portable::complex::abs(one - g.nodes[m - 1]) / nodes_top);
            a.resolution = a.resolution.max(portable::complex::abs(one - two) / nodes_top);
        }
    }
    (a.error, a.resolution)
}

fn profile_index_gradient() -> Outcome {
    let (error, resolution) = profile_agreement();
    crate::verdict::note_numerical(resolution);
    zero(error, 1e-6)
}

/// An asymmetric, lossy ridge: an electric wall west, PMLs east and north.
fn ridge() -> CrossSection {
    let x: Vec<f64> = (0..=28).map(|i| -1.0 + 2.0 * f64::from(i) / 28.0).collect();
    let y: Vec<f64> = (0..=22).map(|j| -0.8 + 1.6 * f64::from(j) / 22.0).collect();
    let mut cells = Vec::new();
    for i in 0..28 {
        for j in 0..22 {
            let (xc, yc) = (0.5 * (x[i] + x[i + 1]), 0.5 * (y[j] + y[j + 1]));
            let eps = if (-0.25..0.3).contains(&xc) && (-0.11..0.11).contains(&yc) {
                c64::new(12.0, 0.02)
            } else if yc < -0.11 {
                re(2.085)
            } else {
                re(1.0)
            };
            cells.push(Permittivity::isotropic(eps));
        }
    }
    CrossSection::new(x, y, cells)
        .expect(OK)
        .with_boundaries(Boundaries {
            west: Boundary::ElectricWall,
            east: Boundary::Zero,
            south: Boundary::Zero,
            north: Boundary::Zero,
        })
        .expect(OK)
        .with_pml(Pml {
            west: 0.0,
            east: 0.3,
            south: 0.0,
            north: 0.25,
            strength: 2.0,
        })
        .expect(OK)
}

/// The full-vector solver's agreement with its differences.
pub(crate) fn vector_agreement() -> (f64, f64) {
    let cs = ridge();
    let mode = &modes(&cs, wl(1.55), 1, Some(2.6)).expect(OK)[0];
    let g = mode.index_gradient(&cs, wl(1.55)).expect(OK);
    let top = g
        .iter()
        .map(|v| portable::complex::abs(*v))
        .fold(0.0, f64::max);
    let (mut error, mut resolution) = (0.0f64, 0.0f64);
    for c in [14 * 22 + 11, 10 * 22 + 11, 14 * 22 + 15, 14 * 22 + 4, 26 * 22 + 11] {
        let f = |d: f64| {
            let mut delta = vec![Permittivity::isotropic(re(0.0)); g.len()];
            delta[c] = Permittivity::isotropic(re(d));
            let q = cs.perturbed(&delta).expect(OK);
            modes(&q, wl(1.55), 1, Some(2.6)).expect(OK)[0].effective_index()
        };
        let one = differences(&f, 1e-3, &|v| *v);
        let two = differences(&f, 2e-3, &|v| *v);
        error = error.max(portable::complex::abs(one - g[c]) / top);
        resolution = resolution.max(portable::complex::abs(one - two) / top);
    }
    (error, resolution)
}

fn vector_index_gradient() -> Outcome {
    let (error, resolution) = vector_agreement();
    crate::verdict::note_numerical(resolution);
    zero(error, 1e-6)
}

fn rcwa_tangent_multilayer() -> Outcome {
    let films = [(re(2.0), 0.3), (re(1.5), 0.2), (re(3.5), 0.15)];
    let layers: Vec<Layer> = films
        .iter()
        .map(|&(n, d)| Layer::uniform(n, Length::um(d)))
        .collect();
    let g = Grating::new(Length::um(0.6), re(1.0), &layers, re(1.45)).expect(OK);
    let ml = Multilayer::new(re(1.0), &films.map(|(n, d)| (n, Length::um(d))), re(1.45)).expect(OK);
    let mut parameters = Vec::new();
    for layer in 0..3 {
        parameters.push(GratingParameter::Permittivity { layer, stripe: 0 });
        parameters.push(GratingParameter::Thickness { layer });
    }
    let mut error: f64 = 0.0;
    for pol in [Polarization::Te, Polarization::Tm] {
        for angle in [0.0, 0.5] {
            let (_, t) = g
                .diffract_tangents(pol, wl(1.0), angle, 5, &parameters)
                .expect(OK);
            let m = ml.reflection_derivatives(pol, wl(1.0), angle).expect(OK);
            for layer in 0..3 {
                let n = films[layer].0.re;
                let (dr, dt) = m.by_index[layer];
                let by_eps = &t[2 * layer];
                error = error
                    .max((by_eps.reflectance - dr / (2.0 * n)).abs())
                    .max((by_eps.transmittance - dt / (2.0 * n)).abs());
                let (dr, dt) = m.by_thickness[layer];
                let by_d = &t[2 * layer + 1];
                error = error
                    .max((by_d.reflectance - dr).abs())
                    .max((by_d.transmittance - dt).abs());
            }
        }
    }
    zero(error, 1e-12)
}

/// Two lamellar layers (one lossy) and a film on glass, the first's ridge of permittivity
/// `eps` and thickness `d1`.
fn stack(parameter: Option<(GratingParameter, f64)>) -> Grating {
    let n = |e: f64| re(e.sqrt());
    let (mut e0, mut e1, mut e2) = (11.0, 4.41, 2.25);
    let (mut d0, mut d2) = (0.22, 0.2);
    match parameter {
        Some((GratingParameter::Permittivity { layer: 0, .. }, h)) => e0 += h,
        Some((GratingParameter::Permittivity { layer: 1, .. }, h)) => e1 += h,
        Some((GratingParameter::Permittivity { layer: 2, .. }, h)) => e2 += h,
        Some((GratingParameter::Thickness { layer: 0 }, h)) => d0 += h,
        Some((GratingParameter::Thickness { layer: 2 }, h)) => d2 += h,
        _ => {}
    }
    let one = Layer::lamellar(
        &[(n(e0), Length::um(0.28)), (n(1.0), Length::um(0.42))],
        Length::um(d0),
    );
    let two = Layer::lamellar(
        &[
            (c64::new(2.0, 0.05), Length::um(0.2)),
            (n(e1), Length::um(0.1)),
            (n(1.0), Length::um(0.4)),
        ],
        Length::um(0.15),
    );
    let three = Layer::uniform(n(e2), Length::um(d2));
    Grating::new(Length::um(0.7), re(1.0), &[one, two, three], re(1.45)).expect(OK)
}

/// RCWA's forward mode against its differences: (largest difference, what they resolve).
pub(crate) fn rcwa_agreement() -> (f64, f64) {
    let parameters = [
        GratingParameter::Permittivity { layer: 0, stripe: 0 },
        GratingParameter::Permittivity { layer: 1, stripe: 1 },
        GratingParameter::Permittivity { layer: 2, stripe: 0 },
        GratingParameter::Thickness { layer: 0 },
        GratingParameter::Thickness { layer: 2 },
    ];
    let (mut error, mut resolution) = (0.0f64, 0.0f64);
    for pol in [Polarization::Te, Polarization::Tm] {
        let (_, tangents) = stack(None)
            .diffract_tangents(pol, wl(1.3), 0.2, 8, &parameters)
            .expect(OK);
        for (p, t) in parameters.iter().zip(&tangents) {
            let f = |h: f64| stack(Some((*p, h))).diffract(pol, wl(1.3), 0.2, 8).expect(OK);
            for i in [7, 8, 9] {
                let transmitted = |d: &Diffraction| re(d.transmitted[i]);
                let reflected = |d: &Diffraction| re(d.reflected[i]);
                let reads: [(f64, &dyn Fn(&Diffraction) -> c64); 2] =
                    [(t.transmitted[i], &transmitted), (t.reflected[i], &reflected)];
                for (got, read) in reads {
                    let one = differences(&f, 1e-4, read).re;
                    let two = differences(&f, 2e-4, read).re;
                    error = error.max((one - got).abs());
                    resolution = resolution.max((one - two).abs());
                }
            }
        }
    }
    (error, resolution)
}

fn rcwa_tangent_gradient() -> Outcome {
    let (error, resolution) = rcwa_agreement();
    crate::verdict::note_numerical(resolution);
    zero(error, 1e-8)
}

/// The weak grating's first transmitted order: forward mode's dη₁/dΔ against the Born
/// approximation's 2η₁/Δ, relative, at `delta`.
pub(crate) fn born_gap(delta: f64) -> f64 {
    let (nb, lambda, period, d, f, angle) = (1.5, 1.0, 1.5, 0.3, 0.4, 0.2);
    let layer = Layer::lamellar(
        &[
            (re((nb * nb + delta).sqrt()), Length::um(f * period)),
            (re(nb), Length::um((1.0 - f) * period)),
        ],
        Length::um(d),
    );
    let g = Grating::new(Length::um(period), re(nb), &[layer], re(nb)).expect(OK);
    let p = GratingParameter::Permittivity { layer: 0, stripe: 0 };
    let (_, t) = g
        .diffract_tangents(Polarization::Te, wl(lambda), angle, 8, &[p])
        .expect(OK);
    let k0 = 2.0 * std::f64::consts::PI / lambda;
    let kx0 = nb * k0 * portable::sin(angle);
    let kx1 = kx0 + 2.0 * std::f64::consts::PI / period;
    let kz = |kx: f64| (nb * nb * k0 * k0 - kx * kx).sqrt();
    let (kz0, kz1) = (kz(kx0), kz(kx1));
    let eps1 = delta * portable::sin(std::f64::consts::PI * f) / std::f64::consts::PI;
    let q = kz0 - kz1;
    let integral = 2.0 * portable::sin(0.5 * q * d) / q;
    let eta = portable::powi(k0 * k0 * eps1 * integral / (2.0 * kz1), 2) * kz1 / kz0;
    let born = 2.0 * eta / delta;
    (t[0].transmitted[9] - born).abs() / born
}

fn rcwa_tangent_born() -> Outcome {
    zero(born_gap(2.5e-3), 1e-3)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The numbers the rules quote: the slab's error at four spacings, the agreements with the
    /// differences and the Born gap at four Δ.
    #[test]
    #[ignore = "a study, printed: cargo test --release --lib the_mode_adjoint_study -- --ignored --nocapture"]
    fn the_mode_adjoint_study() {
        for h in [0.02, 0.01, 0.005, 0.0025] {
            let [te, tm] = slab_derivative_error(h);
            println!("slab h = {h}: TE {te:.3e}, TM {tm:.3e}");
        }
        println!("profile (error, resolution) {:?}", profile_agreement());
        println!("vector (error, resolution) {:?}", vector_agreement());
        println!("rcwa (error, resolution) {:?}", rcwa_agreement());
        for delta in [2e-2, 1e-2, 5e-3, 2.5e-3] {
            println!("born gap at {delta}: {:.4e}", born_gap(delta));
        }
    }
}
