//! The validation cases of the quantum objectives' gradients (0.7, #761's I10; quantum.md's
//! Q4): a pattern's probability, a unitary's fidelity and a ring source's purity against
//! differences, and a coupler's two-photon visibility against its closed form.

use num_complex::Complex64 as c64;

use super::light_checks::{helt_ring, per_um};
use super::ring::RingGrid;
use super::*;
use crate::nonlinear::pairs::{JointSpectrum, Pump};
use crate::portable;
use crate::validation::{Case, Outcome, Tier, note_numerical};

const OK: &str = "a valid quantum objective";

/// A value along a direction, by the step taken.
type Along = Box<dyn Fn(f64) -> f64>;

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "quantum/probability-gradient",
            title: r"Three photons through a lossy 5 × 5 transfer matrix (0.8 times a Haar-random unitary plus a random part), into modes 1, 2, 3 and out in a bunched pattern (0, 2, 0, 1, 0) and a spread one (1, 0, 1, 0, 1), indistinguishable and distinguishable: the probability's derivative along two random complex directions of the matrix from its Wirtinger sensitivity, $dP = 2 \mathop{\mathrm{Re}} \sum G\thinspace dT$, against fourth-order differences ($h = 10^{-3}$) (largest difference relative to the largest derivative shown)",
            tier: Tier::Analytic,
            source: r"S. Aaronson, A. Arkhipov, Theory Comput. 9, 143 (2013), doi:10.4086/toc.2013.v009a004: $P = \lvert \mathrm{Per}\thinspace M\rvert^2/\prod m!$; the permanent is linear in each entry, $\partial\thinspace\mathrm{Per}\thinspace M/\partial M_{ab} = \mathrm{Per}\thinspace M^{(ab)}$ (the minor), summed over the entries of $M$ that are one entry of $T$",
            run: probability_gradient_case,
        },
        Case {
            id: "quantum/fidelity-gradient",
            title: r"Clements et al.'s fidelity of a lossy 4 × 4 matrix to a Haar-random unitary: its derivative along two random complex directions from its Wirtinger sensitivity against fourth-order differences ($h = 10^{-3}$) (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"W. R. Clements et al., Optica 3, 1460 (2016), doi:10.1364/OPTICA.3.001460, Eq. 6: $F = \lvert\mathrm{tr}(U^\dagger U_\mathrm{exp})\rvert^2/(N\thinspace\mathrm{tr}(U_\mathrm{exp}^\dagger U_\mathrm{exp}))$, so $G = \bar o\thinspace\bar U/(Np) - F\thinspace\bar U_\mathrm{exp}/p$",
            run: fidelity_gradient_case,
        },
        Case {
            id: "quantum/visibility-coupler",
            title: r"Two photons on a lossless coupler of power splitting $R$: $dV/dR$ from the visibility's sensitivity to the coupler's matrix carried through $dS/dR$, against the closed form's derivative, at $R$ = 0.05 to 0.95 in steps of 0.05 (largest difference shown)",
            tier: Tier::Analytic,
            source: r"C. K. Hong, Z. Y. Ou, L. Mandel, Phys. Rev. Lett. 59, 2044 (1987), doi:10.1103/PhysRevLett.59.2044: the coincidences $(1 - 2R)^2$ for indistinguishable photons and $R^2 + (1 - R)^2$ for distinguishable ones, so $V = 1 - (1 - 2R)^2/(R^2 + (1 - R)^2)$, one at $R = 1/2$",
            run: visibility_coupler_case,
        },
        Case {
            id: "quantum/purity-gradient",
            title: r"Helt et al.'s ring (resonances 1.48 GHz wide) pumped by a 50 ps pulse, the pump's resonance widened by a second channel at the pump (Vernon et al.'s scheme, an extra round-trip loss of 0.02): the heralded photon's purity's derivatives in that extra loss and in the pump's bandwidth, from the purity's sensitivity to the joint amplitude and the amplitude's derivative by the product rule, against fourth-order differences ($h = 10^{-5}$ in the loss, $10^{-4}$ of the bandwidth) (larger relative difference shown)",
            tier: Tier::Analytic,
            source: r"Z. Vernon et al., Opt. Lett. 42, 3638 (2017), doi:10.1364/OL.42.003638, Eqs. 1, 3 and 4: $\gamma = \mathrm{tr}\thinspace A^2/(\mathrm{tr}\thinspace A)^2$ with $A = FF^\dagger$ on the grid; the amplitude $F_P(\nu_s + \nu_i) F_s(\nu_s) F_i(\nu_i)$ differentiated term by term",
            run: purity_gradient_case,
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

/// A deterministic complex matrix of entries in the unit square, from `seed`.
fn varied(rows: usize, cols: usize, seed: f64) -> Matrix {
    Matrix::from_fn(rows, cols, |i, j| {
        let t = seed + 1.7 * i as f64 + 2.3 * j as f64;
        c64::new(portable::sin(3.1 * t), portable::cos(1.9 * t + 0.4))
    })
}

fn plus(a: &Matrix, d: &Matrix, h: f64) -> Matrix {
    Matrix::from_fn(a.rows(), a.cols(), |i, j| a[(i, j)] + d[(i, j)] * h)
}

/// 2 Re Σ G D: the derivative along D from a Wirtinger sensitivity.
fn along(g: &Matrix, d: &Matrix) -> f64 {
    let mut s = 0.0;
    for i in 0..g.rows() {
        for j in 0..g.cols() {
            s += 2.0 * (g[(i, j)] * d[(i, j)]).re;
        }
    }
    s
}

/// The fourth-order difference of `f` at 0 with step `h`.
fn differences(f: &dyn Fn(f64) -> f64, h: f64) -> f64 {
    (8.0 * (f(h) - f(-h)) - (f(2.0 * h) - f(-2.0 * h))) / (12.0 * h)
}

/// The largest difference between the sensitivities' derivatives and the differences, and
/// between the differences at h and 2h (what they resolve), both relative to the largest
/// derivative: over `checks`, each (the derivative from G, the value along the direction).
fn agreement(checks: &[(f64, &dyn Fn(f64) -> f64)], h: f64) -> (f64, f64) {
    let top = checks.iter().map(|c| c.0.abs()).fold(0.0, f64::max);
    let (mut error, mut resolution) = (0.0f64, 0.0f64);
    for (exact, f) in checks {
        let (one, two) = (differences(*f, h), differences(*f, 2.0 * h));
        error = error.max((one - exact).abs() / top);
        resolution = resolution.max((one - two).abs() / top);
    }
    (error, resolution)
}

/// The probability's gradient against its differences: (error, resolution).
pub(crate) fn probability_agreement() -> (f64, f64) {
    let haar = haar_unitary(5, 7);
    let t = Matrix::from_fn(5, 5, |i, j| {
        0.8 * haar[(i, j)] + 0.05 * varied(5, 5, 0.3)[(i, j)]
    });
    let input = Fock::new(vec![0, 1, 1, 1, 0]);
    let directions = [varied(5, 5, 1.1), varied(5, 5, 2.9)];
    let mut sets: Vec<(f64, Along)> = Vec::new();
    for output in [
        Fock::new(vec![0, 2, 0, 1, 0]),
        Fock::new(vec![1, 0, 1, 0, 1]),
    ] {
        for photons in [Photons::Indistinguishable, Photons::Distinguishable] {
            let (_, g) = probability_gradient(&t, &input, &output, &photons).expect(OK);
            for d in &directions {
                let (t, d, input, output, photons) = (
                    t.clone(),
                    d.clone(),
                    input.clone(),
                    output.clone(),
                    photons.clone(),
                );
                let exact = along(&g, &d);
                sets.push((
                    exact,
                    Box::new(move |h| {
                        probability(&plus(&t, &d, h), &input, &output, &photons).expect(OK)
                    }),
                ));
            }
        }
    }
    let checks: Vec<(f64, &dyn Fn(f64) -> f64)> =
        sets.iter().map(|(e, f)| (*e, f.as_ref())).collect();
    agreement(&checks, 1e-3)
}

fn probability_gradient_case() -> Outcome {
    let (error, resolution) = probability_agreement();
    note_numerical(resolution);
    zero(error, 5e-8)
}

/// The fidelity's gradient against its differences: (error, resolution).
pub(crate) fn fidelity_agreement() -> (f64, f64) {
    let target = haar_unitary(4, 11);
    let implemented = Matrix::from_fn(4, 4, |i, j| {
        0.9 * haar_unitary(4, 12)[(i, j)] + 0.1 * varied(4, 4, 0.7)[(i, j)]
    });
    let (_, g) = fidelity_gradient(&target, &implemented);
    let sets: Vec<(f64, Along)> = [varied(4, 4, 1.3), varied(4, 4, 4.1)]
        .into_iter()
        .map(|d| {
            let (target, implemented) = (target.clone(), implemented.clone());
            let exact = along(&g, &d);
            let f: Box<dyn Fn(f64) -> f64> =
                Box::new(move |h| fidelity(&target, &plus(&implemented, &d, h)));
            (exact, f)
        })
        .collect();
    let checks: Vec<(f64, &dyn Fn(f64) -> f64)> =
        sets.iter().map(|(e, f)| (*e, f.as_ref())).collect();
    agreement(&checks, 1e-3)
}

fn fidelity_gradient_case() -> Outcome {
    let (error, resolution) = fidelity_agreement();
    note_numerical(resolution);
    zero(error, 5e-8)
}

/// dV/dR through the coupler's matrix against the closed form's, largest difference.
pub(crate) fn visibility_coupler_error() -> f64 {
    let mut worst: f64 = 0.0;
    for k in 1..20 {
        let r = 0.05 * f64::from(k);
        let (t, c) = ((1.0 - r).sqrt(), r.sqrt());
        let s = Matrix::from_fn(2, 2, |i, j| {
            if i == j {
                c64::new(t, 0.0)
            } else {
                c64::new(0.0, c)
            }
        });
        let ds = Matrix::from_fn(2, 2, |i, j| {
            if i == j {
                c64::new(-0.5 / t, 0.0)
            } else {
                c64::new(0.0, 0.5 / c)
            }
        });
        let (v, g) = visibility_gradient(&s, [0, 1], [0, 1]).expect(OK);
        let (n, d) = (
            (1.0 - 2.0 * r) * (1.0 - 2.0 * r),
            2.0 * r * r - 2.0 * r + 1.0,
        );
        let closed = -((-4.0 * (1.0 - 2.0 * r)) * d - n * (4.0 * r - 2.0)) / (d * d);
        worst = worst
            .max((along(&g, &ds) - closed).abs())
            .max((v - (1.0 - n / d)).abs());
    }
    worst
}

fn visibility_coupler_case() -> Outcome {
    zero(visibility_coupler_error(), 1e-12)
}

/// A joint spectrum as a matrix, signal by rows.
pub(crate) fn spectrum_matrix(js: &JointSpectrum) -> Matrix {
    let ni = js.idler.len();
    Matrix::from_fn(js.signal.len(), ni, |i, j| js.amplitude[i * ni + j])
}

/// The ring's purity and its derivatives in the pump's extra loss and in the pump's bandwidth
/// (per 1/µm), the extra loss `extra` and the bandwidth `bandwidth` (1/µm), on a grid of
/// `points`.
pub(crate) fn ring_purity(extra: f64, bandwidth: f64, points: usize) -> (f64, f64, f64) {
    let ring = helt_ring(1.48e9);
    let linewidth = per_um(1.48e9);
    let pump = Pump {
        center: ring.resonances[0],
        bandwidth,
    };
    let grid = RingGrid {
        half: 10.0 * linewidth,
        points,
        pump_half: (8.0 * pump.bandwidth).min(100.0 * linewidth),
        pump_points: 1001,
    };
    let (fp, dfp) = (ring.enhancement(extra), ring.enhancement_slope(extra));
    let fs = ring.enhancement(0.0);
    let zero = |_: f64| -> crate::Result<c64> { Ok(c64::new(0.0, 0.0)) };
    let centers = [ring.resonances[1], ring.resonances[2]];
    let (js, by_loss) = ring_spectrum_derivative(
        pump,
        0.0,
        [&fp, &fs, &fs],
        [&dfp, &zero, &zero],
        centers,
        grid,
    )
    .expect(OK);
    let (_, by_bandwidth) = ring_spectrum_derivative(
        pump,
        1.0,
        [&fp, &fs, &fs],
        [&zero, &zero, &zero],
        centers,
        grid,
    )
    .expect(OK);
    let (gamma, g) = purity_gradient(&spectrum_matrix(&js)).expect(OK);
    (
        gamma,
        along(&g, &spectrum_matrix(&by_loss)),
        along(&g, &spectrum_matrix(&by_bandwidth)),
    )
}

/// The pump's bandwidth of a transform-limited Gaussian pulse `ps` long (intensity FWHM), 1/µm.
pub(crate) fn pulse_bandwidth(ps: f64) -> f64 {
    per_um(4.0 * std::f64::consts::LN_2 / (std::f64::consts::TAU * ps * 1e-12))
}

/// The purity's derivative against its differences: (relative error, resolution).
pub(crate) fn purity_agreement() -> (f64, f64) {
    let (x, b, points) = (0.02, pulse_bandwidth(50.0), 81);
    let (_, by_loss, by_bandwidth) = ring_purity(x, b, points);
    let f = |h: f64| ring_purity(x + h, b, points).0;
    // the bandwidth's step 1e-4 of itself, the derivative scaled to match
    let g = |h: f64| ring_purity(x, b * (1.0 + h), points).0;
    let (e1, r1) = agreement(&[(by_loss, &f)], 1e-5);
    let (e2, r2) = agreement(&[(by_bandwidth * b, &g)], 1e-4);
    (e1.max(e2), r1.max(r2))
}

fn purity_gradient_case() -> Outcome {
    let (error, resolution) = purity_agreement();
    note_numerical(resolution);
    zero(error, 1e-8)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The numbers the rules quote.
    #[test]
    #[ignore = "a study, printed: cargo test --release --lib the_objective_study -- --ignored --nocapture"]
    fn the_objective_study() {
        println!(
            "probability (error, resolution) {:?}",
            probability_agreement()
        );
        println!("fidelity (error, resolution) {:?}", fidelity_agreement());
        println!("visibility coupler {:.3e}", visibility_coupler_error());
        println!("purity (error, resolution) {:?}", purity_agreement());
        println!(
            "purity at 0.02, 81 points {:?}",
            ring_purity(0.02, pulse_bandwidth(50.0), 81)
        );
    }

    #[test]
    fn the_gradients_are_the_differences() {
        assert!(probability_agreement().0 < 1e-8);
        assert!(fidelity_agreement().0 < 1e-8);
        assert!(visibility_coupler_error() < 1e-12);
    }
}
