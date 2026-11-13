//! The validation cases of rigorous coupled-wave analysis ([`super::rcwa`]): uniform layers
//! against the transfer matrices, energy in lossless gratings, the effective-medium limit, TM's
//! convergence in the harmonics with Li's rules, and Lalanne et al.'s échelette.

use num_complex::Complex64 as c64;

use super::Polarization;
use super::multilayer::Multilayer;
use super::rcwa::{Factorization, Grating, Layer};
use crate::portable;
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid grating";

fn wl(um: f64) -> Wavelength {
    Wavelength::um(um).expect(OK)
}

fn re(n: f64) -> c64 {
    c64::new(n, 0.0)
}

const POLARIZATIONS: [Polarization; 2] = [Polarization::Te, Polarization::Tm];

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "mode/rcwa-uniform-multilayer",
            title: r"Three uniform films (2.0, 300 nm; 1.5 + 0.1i, 200 nm; 3.5, 150 nm) between air and 1.45 at 1 µm, TE and TM at 0 and 0.5 rad, as an RCWA stack with 11 harmonics: $r$ and $t$ against the transfer matrices (largest difference shown)",
            tier: Tier::Analytic,
            source: "the same stack by mode::multilayer (J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742): a uniform layer's modes are its plane waves, exact at any truncation",
            run: uniform_multilayer,
        },
        Case {
            id: "mode/rcwa-energy-lossless",
            title: r"Lossless lamellar gratings (silicon in air, 700 nm period, one layer and an off-centre two-layer staircase, on 1.45), TE and TM at 0 and 0.3 rad, 41 harmonics: $\lvert 1 - R - T \rvert$ over every order (largest shown)",
            tier: Tier::Analytic,
            source: "Poynting's theorem: a lossless structure's reflected and transmitted powers sum to the incident; the truncated system conserves it too, its Toeplitz matrices Hermitian",
            run: energy_lossless,
        },
        Case {
            id: "mode/rcwa-effective-medium-order",
            title: r"A silicon-in-air grating (fill 0.5, 300 nm thick, air over 1.45) at 1.55 µm with periods of 10 and 5 nm: its zeroth-order $t$ against a uniform film of the static limit's index, TE and TM, the order of the difference in the period, TE's second and TM's first (larger departure shown)",
            tier: Tier::Analytic,
            source: r"form birefringence, the grating's static limit: $\varepsilon_\parallel = f\varepsilon_1 + (1 - f)\varepsilon_2$ for E along the stripes, $1/\varepsilon_\perp = f/\varepsilon_1 + (1 - f)/\varepsilon_2$ across them; inside, the correction is even in $\Lambda/\lambda$ (a mirror-symmetric cell), TE's order; across them the normal E at the layer's two faces excites evanescent orders reaching about $\Lambda/2\pi$ in, a skin first order in $\Lambda$, TM's",
            run: effective_medium_order,
        },
        Case {
            id: "mode/rcwa-effective-medium-faces",
            title: r"The same grating (fill 0.5) set in its own static medium ($\varepsilon_\parallel$ for TE, $\varepsilon_\perp$ for TM), one and two half-waves of it thick, at 1.55 µm with periods of 10 and 5 nm: the phase of its zeroth-order $t$ beyond the medium's, split into the part proportional to the thickness (the bulk's) and the rest (the faces'); the orders in the period of TE's and TM's bulk phase (second) and TM's faces' (first), the largest distance from those shown",
            tier: Tier::Analytic,
            source: r"the static limit's correction inside the grating is even in $\Lambda/\lambda$ (a mirror-symmetric cell), second order for both polarizations; at a face the TM field $E_x = D_x/\varepsilon$ inside differs stripe by stripe from the uniform one outside, and the evanescent orders that make up the difference, reaching about $\Lambda/2\pi$ to either side, carry a polarization of their own: a sheet at each face whose strength is first order in $\Lambda$. TE's $E_y$ is continuous everywhere and has no such sheet",
            run: effective_medium_faces,
        },
        Case {
            id: "mode/rcwa-tm-convergence",
            title: r"A deep silicon-in-air grating (500 nm period, fill 0.5, 600 nm thick, air over 1.45) at 1 µm, TM zeroth-order transmittance with Laurent's rule where Li's inverse rule belongs: its order of convergence in the harmonics from 201, 401 and 801 (shown); extrapolated at that order it meets Li's rules at 801 harmonics to 7e-8, which are within 2e-5 at 51",
            tier: Tier::Analytic,
            source: "L. Li, J. Opt. Soc. Am. A 13, 1870 (1996), doi:10.1364/JOSAA.13.001870, Theorem 2 and Eq. 5: Laurent's rule on a product with complementary jumps leaves at each wall a term that doesn't vanish as M grows, so the eigenproblem's error falls only as 1/M; Theorem 3 and Eq. 4, the inverse rule, remove it",
            run: tm_convergence,
        },
    ]
}

/// The largest difference in r and t between the RCWA stack of uniform films and the transfer
/// matrices.
fn uniform_multilayer() -> Outcome {
    let films = [(re(2.0), 0.3), (c64::new(1.5, 0.1), 0.2), (re(3.5), 0.15)];
    let layers: Vec<Layer> = films
        .iter()
        .map(|&(n, d)| Layer::uniform(n, Length::um(d)))
        .collect();
    let g = Grating::new(Length::um(0.6), re(1.0), &layers, re(1.45)).expect(OK);
    let ml = Multilayer::new(re(1.0), &films.map(|(n, d)| (n, Length::um(d))), re(1.45)).expect(OK);
    let mut error: f64 = 0.0;
    for pol in POLARIZATIONS {
        for angle in [0.0, 0.5] {
            let d = g.diffract(pol, wl(1.0), angle, 5).expect(OK);
            let m = ml.reflection(pol, wl(1.0), angle).expect(OK);
            error = error
                .max(portable::complex::abs(d.r[5] - m.r))
                .max(portable::complex::abs(d.t[5] - m.t));
        }
    }
    Outcome {
        measured: error,
        expected: 0.0,
        tolerance: 1e-12,
        error,
    }
}

/// Silicon in air, one layer and a staircase of two, on 1.45.
fn lossless_gratings() -> Vec<Grating> {
    let (si, air) = (re(3.48), re(1.0));
    let one = Layer::lamellar(
        &[(si, Length::um(0.3)), (air, Length::um(0.4))],
        Length::um(0.5),
    );
    let upper = Layer::lamellar(
        &[
            (air, Length::um(0.1)),
            (si, Length::um(0.2)),
            (air, Length::um(0.4)),
        ],
        Length::um(0.2),
    );
    let lower = Layer::lamellar(
        &[(si, Length::um(0.45)), (air, Length::um(0.25))],
        Length::um(0.25),
    );
    vec![
        Grating::new(Length::um(0.7), air, &[one], re(1.45)).expect(OK),
        Grating::new(Length::um(0.7), air, &[upper, lower], re(1.45)).expect(OK),
    ]
}

fn energy_lossless() -> Outcome {
    let mut error: f64 = 0.0;
    for g in lossless_gratings() {
        for pol in POLARIZATIONS {
            for angle in [0.0, 0.3] {
                let d = g.diffract(pol, wl(1.0), angle, 20).expect(OK);
                error = error.max(d.absorption.abs());
            }
        }
    }
    Outcome {
        measured: error,
        expected: 0.0,
        tolerance: 1e-12,
        error,
    }
}

/// The silicon-in-air grating of the effective-medium case at `period` µm: the difference of its
/// zeroth-order t from the static limit's film, TE and TM.
fn effective_medium_error(period: f64, orders: usize) -> [f64; 2] {
    let (e1, e2) = (re(3.48 * 3.48), re(1.0));
    let f = 0.5;
    let layer = Layer::lamellar(
        &[
            (re(3.48), Length::um(f * period)),
            (re(1.0), Length::um((1.0 - f) * period)),
        ],
        Length::um(0.3),
    );
    let g = Grating::new(Length::um(period), re(1.0), &[layer], re(1.45)).expect(OK);
    let along = e1 * f + e2 * (1.0 - f);
    let across = 1.0 / (f / e1 + (1.0 - f) / e2);
    POLARIZATIONS.map(|pol| {
        let eps = if pol == Polarization::Te {
            along
        } else {
            across
        };
        let film = Multilayer::new(
            re(1.0),
            &[(portable::complex::sqrt(eps), Length::um(0.3))],
            re(1.45),
        )
        .expect(OK)
        .reflection(pol, wl(1.55), 0.0)
        .expect(OK);
        let d = g.diffract(pol, wl(1.55), 0.0, orders).expect(OK);
        portable::complex::abs(d.t[orders] - film.t)
    })
}

fn effective_medium_order() -> Outcome {
    let e: Vec<[f64; 2]> = [0.01, 0.005]
        .iter()
        .map(|&p| effective_medium_error(p, 40))
        .collect();
    // the order of the last halving: TE's second, TM's first
    let order = |k: usize| portable::log2(e[0][k] / e[1][k]);
    let error = (order(0) - 2.0).abs().max((order(1) - 1.0).abs());
    Outcome {
        measured: error,
        expected: 0.0,
        tolerance: 0.1,
        error,
    }
}

/// The silicon-in-air grating (fill 0.5) of period `p` µm, `d` µm thick, between two half-spaces
/// of index `outer`: its zeroth-order r and t at 1.55 µm, normal incidence, by `rule`.
fn embedded(
    pol: Polarization,
    p: f64,
    d: f64,
    outer: c64,
    orders: usize,
    rule: Factorization,
) -> (c64, c64) {
    let layer = Layer::lamellar(
        &[
            (re(3.48), Length::um(0.5 * p)),
            (re(1.0), Length::um(0.5 * p)),
        ],
        Length::um(d),
    );
    let g = Grating::new(Length::um(p), outer, &[layer], outer).expect(OK);
    let x = g.diffract_with(pol, wl(1.55), 0.0, orders, rule).expect(OK);
    (x.r[orders], x.t[orders])
}

/// The static limit's permittivity of the silicon-in-air grating (fill 0.5): ε∥ for TE, ε⊥ for
/// TM.
fn static_limit(pol: Polarization) -> f64 {
    let (e1, e2, f) = (3.48f64 * 3.48, 1.0, 0.5);
    if pol == Polarization::Te {
        f * e1 + (1.0 - f) * e2
    } else {
        1.0 / (f / e1 + (1.0 - f) / e2)
    }
}

/// The grating of period `p` µm set in its own static medium, one and two half-waves of that
/// medium thick (where the faces' multiple reflection adds no phase): the phase of its
/// zeroth-order t beyond the medium's, split into the part proportional to the thickness (the
/// bulk's, per µm) and the part that isn't (the two faces').
fn bulk_and_faces(pol: Polarization, p: f64, orders: usize, rule: Factorization) -> (f64, f64) {
    let n0 = static_limit(pol).sqrt();
    let k0 = 2.0 * std::f64::consts::PI / 1.55;
    let phase = |d: f64| {
        let (_, t) = embedded(pol, p, d, re(n0), orders, rule);
        portable::complex::arg(t * portable::complex::exp(c64::new(0.0, -k0 * n0 * d)))
    };
    let half = 1.55 / (2.0 * n0);
    let (a, b) = (phase(half), phase(2.0 * half));
    let bulk = (b - a) / half;
    (bulk, a - half * bulk)
}

/// The orders in the period, from 10 to 5 nm, of TE's and TM's bulk phase (second) and TM's
/// faces' (first): the largest distance from those.
fn effective_medium_faces() -> Outcome {
    let order = |pol: Polarization, face: bool| {
        let [coarse, fine] = [0.01, 0.005].map(|p| {
            let (bulk, faces) = bulk_and_faces(pol, p, 40, Factorization::Li);
            if face { faces } else { bulk }
        });
        portable::log2(coarse / fine)
    };
    let error = (order(Polarization::Te, false) - 2.0)
        .abs()
        .max((order(Polarization::Tm, false) - 2.0).abs())
        .max((order(Polarization::Tm, true) - 1.0).abs());
    Outcome {
        measured: error,
        expected: 0.0,
        tolerance: 0.1,
        error,
    }
}

/// The deep grating of the TM convergence case.
fn deep_grating() -> Grating {
    let layer = Layer::lamellar(
        &[(re(3.48), Length::um(0.25)), (re(1.0), Length::um(0.25))],
        Length::um(0.6),
    );
    Grating::new(Length::um(0.5), re(1.0), &[layer], re(1.45)).expect(OK)
}

/// The deep grating's TM zeroth-order transmittance with −`orders` … `orders` kept, by `rule`.
fn deep_tm(orders: usize, rule: Factorization) -> f64 {
    deep_grating()
        .diffract_with(Polarization::Tm, wl(1.0), 0.0, orders, rule)
        .expect(OK)
        .transmitted_order(0)
}

/// Laurent's rule's order of convergence from its steps 100 → 200 → 400 orders.
fn tm_convergence() -> Outcome {
    let t: Vec<f64> = [100, 200, 400]
        .iter()
        .map(|&m| deep_tm(m, Factorization::Laurent))
        .collect();
    let order = portable::log2((t[1] - t[0]) / (t[2] - t[1]));
    Outcome {
        measured: order,
        expected: 1.0,
        tolerance: 0.05,
        error: (order - 1.0).abs(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lalanne et al.'s échelette: a sawtooth of period 1.9 µm at 633 nm (their 3λ) in glass (1.52)
    /// on glass, lit from air at
    /// normal incidence, `depth` × λ/(n − 1) deep, as a staircase of `layers` lamellar layers (each
    /// layer's glass is the sawtooth's at its mid-height): the first transmitted order's efficiency
    /// for unpolarized light, the mean of TE's and TM's, with −`orders` … `orders` kept.
    fn echelette(layers: usize, orders: usize, depth: f64) -> f64 {
        echelette_at(layers, orders, depth, 1.9 / 0.633)
    }

    /// [`echelette`] with a period of `ratio` wavelengths.
    fn echelette_at(layers: usize, orders: usize, depth: f64, ratio: f64) -> f64 {
        let [te, tm] = echelette_pols(layers, orders, depth, ratio);
        (te + tm) / 2.0
    }

    /// [`echelette_at`]'s TE and TM efficiencies.
    fn echelette_pols(layers: usize, orders: usize, depth: f64, ratio: f64) -> [f64; 2] {
        let n = 1.52;
        // the letter's 3λ, 1.9 µm at 633 nm: exactly 3λ would put the third orders at grazing
        let lambda = 1.9 / ratio;
        let period = 1.9;
        let h = depth * lambda / (n - 1.0);
        let stack: Vec<Layer> = (0..layers)
            .map(|k| {
                // glass where the surface, rising along x, is above the layer's mid-height
                let glass = (k as f64 + 0.5) / layers as f64;
                Layer::lamellar(
                    &[
                        (re(1.0), Length::um((1.0 - glass) * period)),
                        (re(n), Length::um(glass * period)),
                    ],
                    Length::um(h / layers as f64),
                )
            })
            .collect();
        let g = Grating::new(Length::um(period), re(1.0), &stack, re(n)).expect(OK);
        POLARIZATIONS.map(|pol| {
            g.diffract(pol, wl(lambda), 0.0, orders)
                .expect(OK)
                .transmitted_order(1)
        })
    }

    /// The échelette's best depth (in λ/(n − 1)) and its efficiency, by golden-section search on
    /// [0.9, 1.4].
    fn echelette_best(layers: usize, orders: usize) -> (f64, f64) {
        best_depth(|depth| echelette(layers, orders, depth))
    }

    /// The depth (in λ/(n − 1)) on [0.9, 1.4] where `efficiency` peaks, and the peak, by
    /// golden-section search.
    fn best_depth(efficiency: impl Fn(f64) -> f64) -> (f64, f64) {
        let golden = (5f64.sqrt() - 1.0) / 2.0;
        let (mut a, mut b) = (0.9, 1.4);
        let mut c = b - golden * (b - a);
        let mut d = a + golden * (b - a);
        let (mut fc, mut fd) = (efficiency(c), efficiency(d));
        while b - a > 1e-5 {
            if fc > fd {
                b = d;
                d = c;
                fd = fc;
                c = b - golden * (b - a);
                fc = efficiency(c);
            } else {
                a = c;
                c = d;
                fc = fd;
                d = a + golden * (b - a);
                fd = efficiency(d);
            }
        }
        let x = (a + b) / 2.0;
        (x, efficiency(x))
    }

    /// Lalanne et al.'s échelette against the period (in wavelengths), the staircase's layers and
    /// the harmonics, the depth optimized for unpolarized light at each.
    #[test]
    #[ignore = "prints the échelette's sensitivity"]
    fn print_echelette_scan() {
        for ratio in [
            2.95,
            2.99,
            2.999,
            2.9999,
            3.0001,
            3.0005,
            3.001,
            1.9 / 0.633,
            1.9 / 0.6328,
            3.005,
            3.01,
            3.02,
            3.05,
            3.1,
        ] {
            let mut line = format!("ratio {ratio:.5}:");
            for layers in [15, 30, 60, 120] {
                let (h, t) = best_depth(|depth| echelette_at(layers, 30, depth, ratio));
                let [te, tm] = echelette_pols(layers, 30, h, ratio);
                line += &format!(
                    " {layers} layers depth {h:.4} T {:.3} (TE {:.3} TM {:.3});",
                    100.0 * t,
                    100.0 * te,
                    100.0 * tm
                );
            }
            eprintln!("{line}");
        }
        for orders in [15, 30, 60] {
            let (h, t) = best_depth(|depth| echelette_at(60, orders, depth, 1.9 / 0.633));
            eprintln!(
                "60 layers, {orders} orders: depth {h:.4} T {:.3}",
                100.0 * t
            );
        }
        // the efficiency against the depth at the letter's period, 120 layers
        for depth in [1.0, 1.05, 1.1, 1.12, 1.15, 1.2] {
            let t = echelette_at(120, 30, depth, 1.9 / 0.633);
            eprintln!("depth {depth}: T {:.3}", 100.0 * t);
        }
    }

    /// A silicon-in-air lamellar grating on glass: TM zeroth-order transmittance against the
    /// harmonics, by Li's rules and by Laurent's.
    #[test]
    #[ignore = "prints the TM convergence study"]
    fn print_tm_convergence() {
        let g = deep_grating();
        for (pol, rule) in [
            (Polarization::Tm, Factorization::Li),
            (Polarization::Tm, Factorization::Laurent),
            (Polarization::Te, Factorization::Li),
        ] {
            let reference = g
                .diffract_with(pol, wl(1.0), 0.0, 320, rule)
                .expect(OK)
                .transmitted_order(0);
            for m in [5, 10, 20, 40, 80, 160] {
                let t = g
                    .diffract_with(pol, wl(1.0), 0.0, m, rule)
                    .expect(OK)
                    .transmitted_order(0);
                eprintln!(
                    "{pol:?} {rule:?} M {m}: T0 {t:.10} error {:.3e}",
                    (t - reference).abs()
                );
            }
            eprintln!("{pol:?} {rule:?} reference {reference:.12}");
        }
        for p in [0.16, 0.08, 0.04, 0.02, 0.01] {
            eprintln!("emt period {p}: {:?}", effective_medium_error(p, 40));
        }
        eprintln!("emt period 0.005: {:?}", effective_medium_error(0.005, 40));
        eprintln!(
            "emt period 0.02 at 80 orders: {:?}",
            effective_medium_error(0.02, 80)
        );
        // TM's departure: a skin at the faces (the same at any thickness) or the bulk (growing
        // with it)
        for d in [0.15, 0.3, 0.6, 1.2] {
            let p = 0.01;
            let (e1, e2, f) = (re(3.48 * 3.48), re(1.0), 0.5);
            let layer = Layer::lamellar(
                &[
                    (re(3.48), Length::um(f * p)),
                    (re(1.0), Length::um((1.0 - f) * p)),
                ],
                Length::um(d),
            );
            let g = Grating::new(Length::um(p), re(1.0), &[layer], re(1.0)).expect(OK);
            let across = 1.0 / (f / e1 + (1.0 - f) / e2);
            let along = e1 * f + e2 * (1.0 - f);
            for (pol, eps) in [(Polarization::Tm, across), (Polarization::Te, along)] {
                let film = Multilayer::new(
                    re(1.0),
                    &[(portable::complex::sqrt(eps), Length::um(d))],
                    re(1.0),
                )
                .expect(OK)
                .reflection(pol, wl(1.55), 0.0)
                .expect(OK);
                let t = g.diffract(pol, wl(1.55), 0.0, 40).expect(OK).t[40];
                eprintln!("{pol:?} thickness {d}: t/t_film {:.6e}", (t / film.t) - 1.0);
            }
        }
    }

    #[test]
    #[ignore = "prints the convergence tails and the échelette"]
    fn print_tails() {
        let g = deep_grating();
        for (pol, rule) in [
            (Polarization::Tm, Factorization::Li),
            (Polarization::Tm, Factorization::Laurent),
            (Polarization::Te, Factorization::Li),
        ] {
            let t: Vec<f64> = [25, 50, 100, 200, 400]
                .iter()
                .map(|&m| {
                    g.diffract_with(pol, wl(1.0), 0.0, m, rule)
                        .expect(OK)
                        .transmitted_order(0)
                })
                .collect();
            let steps: Vec<f64> = t.windows(2).map(|w| w[1] - w[0]).collect();
            eprintln!("{pol:?} {rule:?}: T {t:.12?} steps {steps:?}");
        }
        for (k, m) in [(15, 30), (30, 30), (60, 30), (30, 60), (120, 30)] {
            let (h, t) = echelette_best(k, m);
            eprintln!("echelette {k} layers, {m} orders: depth {h:.5} x lambda/(n-1), T+1 {t:.6}");
        }
        for ratio in [2.99, 2.999, 2.9999, 3.0001, 3.001, 3.0016, 3.01] {
            let t: Vec<String> = [15, 60]
                .iter()
                .map(|&k| format!("{:.5}", echelette_at(k, 30, 1.1197, ratio)))
                .collect();
            eprintln!("echelette period/lambda {ratio}: T+1 (15, 60 layers) {t:?}");
        }
    }

    /// The grating set in its own static medium: the bulk's index beyond the static limit's
    /// against the second-order form birefringence, the faces' phase, and each face's reflection
    /// as a sheet's, r/(1 + e^(2iφ)), at four thicknesses; by Li's rules and by Laurent's.
    #[test]
    #[ignore = "prints the TM static limit study"]
    fn print_static_limit() {
        let (e1, e2, f) = (3.48f64 * 3.48, 1.0f64, 0.5f64);
        let k0 = 2.0 * std::f64::consts::PI / 1.55;
        // the second order in Λ/λ
        let second = |pol: Polarization, p: f64| {
            let s = portable::powi(std::f64::consts::PI * p / 1.55 * f * (1.0 - f), 2) / 3.0
                * portable::powi(e1 - e2, 2);
            let (along, across) = (
                static_limit(Polarization::Te),
                static_limit(Polarization::Tm),
            );
            if pol == Polarization::Te {
                along + s
            } else {
                across + s * along * portable::powi(across, 3) / portable::powi(e1 * e2, 2)
            }
        };
        for pol in POLARIZATIONS {
            let n0 = static_limit(pol).sqrt();
            for rule in [Factorization::Li, Factorization::Laurent] {
                if pol == Polarization::Te && rule == Factorization::Laurent {
                    continue;
                }
                for m in [20, 40, 80] {
                    for p in [0.04, 0.02, 0.01, 0.005] {
                        let (bulk, faces) = bulk_and_faces(pol, p, m, rule);
                        let sheets: Vec<String> = [0.15, 0.3, 0.6, 1.2]
                            .iter()
                            .map(|&d| {
                                let (r, _) = embedded(pol, p, d, re(n0), m, rule);
                                let s = r
                                    / (1.0
                                        + portable::complex::exp(c64::new(0.0, 2.0 * k0 * n0 * d)));
                                format!("{:.4e}{:+.4e}i", s.re, s.im)
                            })
                            .collect();
                        eprintln!(
                            "{pol:?} {rule:?} M {m} period {p}: dn {:.5e} second order {:.5e} faces {faces:.5e} = {:.4} k0 period; sheets {sheets:?}",
                            bulk / k0,
                            second(pol, p).sqrt() - n0,
                            faces / (k0 * p)
                        );
                    }
                }
            }
        }
        let o = effective_medium_faces();
        eprintln!("faces case: {o:?}");
    }

    #[test]
    #[ignore = "801 harmonics: slow outside release builds (the validation report runs it)"]
    fn tm_convergence_passes() {
        let o = tm_convergence();
        assert!(o.passed(), "{o:?}");
    }

    #[test]
    fn cases_pass() {
        for case in cases() {
            if case.id == "mode/rcwa-tm-convergence" {
                continue;
            }
            let o = (case.run)();
            assert!(o.passed(), "{}: {o:?}", case.id);
        }
    }
}
