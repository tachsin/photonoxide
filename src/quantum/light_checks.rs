//! What the validation report checks the quantum light on (Q2): hafnians against their
//! definition and the perfect matchings of the complete graph, squeezed and two-mode squeezed
//! vacuum against their closed forms, loss, coherent and displaced squeezed states, clicks;
//! pair sources (their pairs, g⁽²⁾, two sources' interference, heralding through a coupler);
//! rings' pairs against Helt et al. and Vernon et al.; squeezers from 0.6.1's amplifiers; the
//! squeezing papers of Vaidya et al. and Nehra et al.; broadband squeezing at high gain against
//! its closed forms. Paesani et al.'s inter-modal source, from
//! the mode solver, is computed by an ignored test here: it doesn't agree with their paper yet
//! (docs/methods/quantum.md says by how much), so it isn't a case.

use std::f64::consts::{PI, TAU};
use std::sync::Arc;

use num_complex::Complex64 as c64;

use super::hafnian::by_definition;
use super::ring::{RingGrid, ring_spectrum};
use super::source::{Source, heralding_through, hom_visibility};
use super::squeezer::{degenerate, four_wave, non_degenerate};
use super::*;
use crate::circuit::components::{AllPassRing, Coupler, Dispersion, Waveguide};
use crate::circuit::{Component, Netlist};
use crate::nonlinear::coupled::{SecondHarmonic, ThreeWave};
use crate::nonlinear::fwm::{FourWave, signal_gain};
use crate::nonlinear::pairs::{JointSpectrum, Pump, gaussian_purity};
use crate::units::{SPEED_OF_LIGHT, Wavelength};
use crate::validation::{Case, Outcome, Tier, note_numerical};

const OK: &str = "a valid quantum-light check";

fn um(x: f64) -> Wavelength {
    Wavelength::um(x).expect(OK)
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

fn worst(error: f64, tolerance: f64) -> Outcome {
    outcome(error, 0.0, tolerance)
}

fn relative(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance: tolerance * expected.abs(),
        error: (measured - expected).abs(),
    }
}

fn factorial(n: usize) -> f64 {
    (1..=n).map(|j| j as f64).product()
}

fn random_symmetric(n: usize, seed: u64) -> Matrix {
    let mut rng = super::rng::Rng::new(seed, 13);
    let mut a = Matrix::zeros(n, n);
    for i in 0..n {
        for j in 0..=i {
            let v = c64::new(rng.uniform() - 0.5, rng.uniform() - 0.5);
            a[(i, j)] = v;
            a[(j, i)] = v;
        }
    }
    a
}

/// The cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "quantum/hafnian-ones",
            title: r"Quantum: the hafnian of the $2n \times 2n$ all-ones matrix is the number of perfect matchings of $2n$ vertices, $(2n - 1)!!$, $n$ = 1 to 12 (largest relative error shown)",
            tier: Tier::Analytic,
            source: r"$\mathop{\mathrm{Haf}}(J_{2n}) = (2n-1)!!$; A. Björklund, B. Gupt, N. Quesada, ACM J. Exp. Algorithmics 24, 1.11 (2019), doi:10.1145/3325111, Eq. (24)",
            run: hafnian_ones,
        },
        Case {
            id: "quantum/hafnian-definition",
            title: r"Quantum: hafnians and loop hafnians by Björklund et al.'s power traces against the sums over perfect matchings (with loops), random complex symmetric matrices of 2 to 12 rows, and loop hafnians of odd sizes 3 to 11 (largest relative difference shown)",
            tier: Tier::Analytic,
            source: "A. Björklund, B. Gupt, N. Quesada, ACM J. Exp. Algorithmics 24, 1.11 (2019), doi:10.1145/3325111, Eqs. (24)–(26) against their Eqs. (1) and (13)",
            run: hafnian_definition,
        },
        Case {
            id: "quantum/hafnian-threads",
            title: r"Quantum: a hafnian and a loop hafnian of a random $28 \times 28$ matrix and the torontonian of a two-mode squeezed state's 14 modes, on 1 and 7 threads (largest difference shown)",
            tier: Tier::Analytic,
            source: "the same bits on any number of threads (AGENTS.md): the 2^14 terms are cut into 256 parts fixed by the size, added in order",
            run: hafnian_threads,
        },
        Case {
            id: "quantum/squeezed-vacuum",
            title: r"Quantum: single-mode squeezed vacuum ($r$ = 0.8, $\varphi$ = 0.7) by the hafnian: $P(2k) = \frac{(2k)!}{4^k (k!)^2} \frac{\tanh^{2k} r}{\cosh r}$ for $k$ = 0 to 15, no odd numbers, the distribution's mean against the closed form's and the covariance's $\sinh^2 r$, and a threshold detector dark with probability $1/\cosh r$ (largest difference shown)",
            tier: Tier::Analytic,
            source: "the squeezed vacuum in the number basis (C. Weedbrook et al., Rev. Mod. Phys. 84, 621 (2012), doi:10.1103/RevModPhys.84.621); C. S. Hamilton et al., Phys. Rev. Lett. 119, 170501 (2017), doi:10.1103/PhysRevLett.119.170501",
            run: squeezed_vacuum,
        },
        Case {
            id: "quantum/two-mode-thermal",
            title: r"Quantum: two-mode squeezed vacuum ($r$ = 0.6): $P(n, m) = \delta_{nm} \tanh^{2n} r / \cosh^2 r$ by the hafnian for $n, m \le 7$, each half a thermal state of $\sinh^2 r$ photons, and both modes clicking as often as the sum of $P(n, n)$, $n \ge 1$, by the torontonian (largest difference shown)",
            tier: Tier::Analytic,
            source: "each half of a two-mode squeezed vacuum is thermal (C. Weedbrook et al., doi:10.1103/RevModPhys.84.621); N. Quesada, J. M. Arrazola, N. Killoran, Phys. Rev. A 98, 062322 (2018), doi:10.1103/PhysRevA.98.062322",
            run: two_mode_thermal,
        },
        Case {
            id: "quantum/squeezed-loss",
            title: r"Quantum: squeezed vacuum ($r$ = 1.2) on one port of a 2 mm guide losing 4.5 dB/cm, through the circuit's $S$ with its loss: the squeezed quadrature's variance $\eta e^{-2r} + 1 - \eta$, $\eta = \lvert S_{21} \rvert^2$, and the anti-squeezed one's $\eta e^{2r} + 1 - \eta$ (largest difference shown)",
            tier: Tier::Analytic,
            source: r"a loss $\eta$ mixes in the vacuum: $\hat b = \sqrt{\eta}\thinspace \hat a + \sqrt{1 - \eta}\thinspace \hat c$",
            run: squeezed_loss,
        },
        Case {
            id: "quantum/displaced-states",
            title: r"Quantum: a coherent state ($\alpha$ = 0.7 − 0.4i) by the loop hafnian is Poissonian, $n$ = 0 to 15; and a displaced squeezed state ($r$ = 0.3, $\varphi$ = 1.1, $\alpha$ = 0.4 + 0.5i): its distribution's mean and variance, to 30 photons, against the covariance's $\frac{1}{4}(\mathop{\mathrm{Tr}} V + \lvert d \rvert^2) - \frac{1}{2}$ and $\frac{1}{8}(\mathop{\mathrm{Tr}} V^2 - 2) + \frac{1}{4} d^T V d$ (largest difference shown)",
            tier: Tier::Analytic,
            source: "a coherent state's number distribution is Poisson's; a Gaussian state's photon-number moments by Wick's theorem (C. Weedbrook et al., doi:10.1103/RevModPhys.84.621, with ħ = 2)",
            run: displaced_states,
        },
        Case {
            id: "quantum/clicks-coherent",
            title: r"Quantum: coherent states ($\alpha$ = 0.6, 0.3i, 0.8 + 0.2i, −0.4 + 0.5i) through a Haar-random $4 \times 4$ interferometer losing 30% in every path: all 16 threshold click patterns against $\prod_j (1 - e^{-\lvert \beta_j \rvert^2})$ over the clicking modes times $e^{-\lvert \beta_j \rvert^2}$ over the dark, $\beta = T\alpha$, and their sum against 1 (largest difference shown)",
            tier: Tier::Analytic,
            source: r"a passive map keeps coherent states coherent, $\beta = T\alpha$, and a coherent state is dark with probability $e^{-\lvert \beta \rvert^2}$, independently in each mode",
            run: clicks_coherent,
        },
        Case {
            id: "quantum/clicks-displaced",
            title: r"Quantum: a displaced two-mode squeezed vacuum ($r$ = 0.1, $\varphi$ = 0.4, $\alpha$ = 0.3 and 0.2i): its four threshold click patterns by the loop form against its photon-number probabilities by the loop hafnian, summed to 15 photons in each mode (their tail below $10^{-14}$; largest difference shown)",
            tier: Tier::Analytic,
            source: "a threshold detector clicks for one photon or more: each pattern is the sum of the number patterns it holds",
            run: clicks_displaced,
        },
        Case {
            id: "quantum/source-pairs",
            title: r"Quantum: a pair source of $K$ = 4 equal Schmidt modes at 0.5 pairs per pulse: the number of pairs is negative binomial, $\binom{n + K - 1}{n} t^n (1 - t)^K$ with $t = \tanh^2 r$, for $n \le 10$; its unheralded $g^{(2)}$ is $1 + 1/K$, and a separable source's 2 (largest difference shown)",
            tier: Tier::Analytic,
            source: "a sum of K independent geometric numbers is negative binomial; each mode's state is thermal, whose g⁽²⁾ is 2",
            run: source_pairs,
        },
        Case {
            id: "quantum/source-hom-gaussian",
            title: r"Quantum: the Hong–Ou–Mandel visibility of heralded photons from two sources with Gaussian joint spectra ($M$ = [[1, 0.6], [0.6, 1.3]] and [[1.4, −0.3], [−0.3, 0.9]]), $\mathop{\mathrm{Tr}}(\rho_A \rho_B)$ on a 161-point grid, against its closed form, and a source with itself against its purity $\sqrt{1 - c^2/ab}$ (largest difference shown)",
            tier: Tier::Analytic,
            source: r"Gaussian integrals: $\mathop{\mathrm{Tr}}(\rho_A \rho_B) = 2\sqrt{\det M_A \det M_B} / \sqrt{b_A b_B \det(Q_A + Q_B)}$, $Q = \frac{1}{2}$[[$2a - c^2/b$, $-c^2/b$], [$-c^2/b$, $2a - c^2/b$]]; W. P. Grice, I. A. Walmsley, Phys. Rev. A 56, 1627 (1997), doi:10.1103/PhysRevA.56.1627",
            run: source_hom_gaussian,
        },
        Case {
            id: "quantum/heralding-coupler",
            title: r"Quantum: a pair source's heralding through a circuit: the signal from one input of a coupler of crossing power 0.3 with 0.5 dB of excess loss, the idler from its other input to one output: the signal's heralding efficiencies at the two outputs sum to the coupler's transmission, $10^{-0.05}$, and the idler's is its path's power transmission (largest difference shown)",
            tier: Tier::Analytic,
            source: "a flat filter passes every pair's photon alike: its power transmission is the heralding efficiency",
            run: heralding_coupler,
        },
        Case {
            id: "quantum/ring-helt-short",
            title: r"Quantum: pairs from Helt et al.'s ring (47.5 µm radius, $n_g$ = 1.753, resonances 1.48 GHz wide, lossless; pump at order 310, pairs at 311 and 309) pumped by a 5 ps pulse: the Schmidt number from the 0.4 ring components' field enhancement (shown)",
            tier: Tier::Published,
            source: "L. G. Helt, Z. Yang, M. Liscidini, J. E. Sipe, Opt. Lett. 35, 3006 (2010), doi:10.1364/OL.35.003006, p. 3007: K = 1.09 for the 5 ps pulse; printed to two decimals",
            run: ring_helt_short,
        },
        Case {
            id: "quantum/ring-helt-long",
            title: r"Quantum: the same ring pumped by a 5 ns pulse: the Schmidt number (shown)",
            tier: Tier::Published,
            source: "L. G. Helt et al., doi:10.1364/OL.35.003006, p. 3007: K = 10.00 for the 5 ns pulse; the linewidth they state, 2π(0.74 GHz), to two figures",
            run: ring_helt_long,
        },
        Case {
            id: "quantum/ring-vernon-equal",
            title: r"Quantum: a ring with equal pump, signal and idler linewidths and a pump far broader than them: the Schmidt number (shown)",
            tier: Tier::Published,
            source: "Z. Vernon et al., Opt. Lett. 42, 3638 (2017), doi:10.1364/OL.42.003638, Fig. 2(a): K = 1.09; printed to two decimals",
            run: ring_vernon_equal,
        },
        Case {
            id: "quantum/squeezer-degenerate",
            title: r"Quantum: the squeezing of a lossless, phase-matched degenerate parametric amplifier (κ = 0.01 per √W per µm, 1 W of pump, 1 mm) from 0.6.1's coupled equations, two seeded runs: $r = \kappa \sqrt{P} L$ and $\lvert \mu \rvert^2 - \lvert \nu \rvert^2 = 1$ (relative to $\lvert \mu \rvert^2$); and a non-degenerate one's map in photon units, signal and idler at 0.6 and 0.5 µm (largest difference shown)",
            tier: Tier::Analytic,
            source: r"undepleted, the fundamental's quadratures grow and shrink as $e^{\pm \kappa \sqrt{P} z}$ (J. A. Armstrong et al., Phys. Rev. 127, 1918 (1962), doi:10.1103/PhysRev.127.1918)",
            run: squeezer_degenerate,
        },
        Case {
            id: "quantum/squeezer-four-wave",
            title: r"Quantum: the two-mode squeezing of four-wave mixing with one pump ($\gamma$ = 1 per W per m, 2 W, $\Delta k$ = −3 per m, 1 m, self- and cross-phase modulation included): $\cosh^2 r$ against 0.6.1's closed-form gain $1 + (\gamma P \sinh(gL)/g)^2$ (relative difference shown)",
            tier: Tier::Analytic,
            source: "G. P. Agrawal, Nonlinear Fiber Optics, 6th ed. (2019), doi:10.1016/C2018-0-01168-8, Eq. (10.4.7)",
            run: squeezer_four_wave,
        },
        Case {
            id: "quantum/squeezer-rk4-order",
            title: r"Quantum: the degenerate amplifier's squeezing $r$ on 80, 160 and 320 RK4 steps: its order of convergence in the step (shown, to two decimals)",
            tier: Tier::Analytic,
            source: "the classical fourth-order Runge–Kutta method's error falls as the step's fourth power",
            run: squeezer_rk4_order,
        },
        Case {
            id: "quantum/vaidya-number-difference",
            title: r"Quantum: a two-mode squeezed vacuum through a loss $\eta$ in each arm: the photon-number difference's variance over the mean number, $1 - \eta$ at every gain, here at $\eta$ = 0.8, the ring's escape efficiency, in dB (shown)",
            tier: Tier::Published,
            source: "V. D. Vaidya et al., Sci. Adv. 6, eaba9186 (2020), doi:10.1126/sciadv.aba9186, Eq. 3 and p. 5: ~7 dB of number-difference squeezing inferred on chip from an escape efficiency of 80%; to the dB",
            run: vaidya_number_difference,
        },
        Case {
            id: "quantum/vaidya-outcoupling",
            title: r"Quantum: the most quadrature squeezing a perfect ring delivers off chip through 1.5 dB and 0.5 dB of coupling loss (squeezed vacuum of $r$ = 10 through the loss; the largest of the two differences shown, dB)",
            tier: Tier::Published,
            source: "V. D. Vaidya et al., doi:10.1126/sciadv.aba9186, p. 6: ~1.5 dB of coupling loss limits the squeezing to \"about 5 dB\", 0.5 dB to \"nearly 10 dB\"; to the dB",
            run: vaidya_outcoupling,
        },
        Case {
            id: "quantum/nehra-inferred",
            title: r"Quantum: Nehra et al.'s measured squeezing and anti-squeezing (4.2 and 9.7 dB at 0.8 pJ) through the loss model of a squeezed vacuum and a loss $1 - \eta$: the squeezing before the loss, both unknowns solved (dB shown)",
            tier: Tier::Published,
            source: "R. Nehra et al., Science 377, 1333 (2022), doi:10.1126/science.abo6213, p. 1335: 10.48 ± 0.87 dB inferred after correcting for losses (L ≈ 0.3)",
            run: nehra_inferred,
        },
        Case {
            id: "quantum/nehra-all-optical",
            title: r"Quantum: Nehra et al.'s all-optical measurement: 11 dB of squeezing amplified by a second squeezer of 33 dB phase-sensitive gain, the mean photon numbers with and without it (their Eq. 1) read as squeezing (dB shown)",
            tier: Tier::Published,
            source: "R. Nehra et al., doi:10.1126/science.abo6213, Eq. 1 and p. 1334: a measurement gain of more than 33 dB \"for ~11 dB\" of squeezing; to the dB",
            run: nehra_all_optical,
        },
        Case {
            id: "quantum/nehra-homodyne-bound",
            title: r"Quantum: the most squeezing a homodyne detector sees behind Nehra et al.'s overall detection efficiency of 0.20 ($r$ = 10, dB shown)",
            tier: Tier::Published,
            source: "R. Nehra et al., doi:10.1126/science.abo6213, p. 1335: η ≈ 0.20 \"puts an upper limit of\" under 1 dB \"to the measurable squeezing for a standard balanced homodyne detection\"",
            run: nehra_homodyne_bound,
        },
        Case {
            id: "quantum/broadband-symplectic",
            title: r"Quantum: a broadband source at high gain ($g$ = 3; 48 signal and 48 idler frequencies, a Gaussian pump, a phase mismatch linear in both, 40 slices): its Bogoliubov map $M$ keeps the commutators, $M \Sigma M^\dagger = \Sigma$, $\Sigma = \mathop{\mathrm{diag}}(I, -I)$ (largest entry of the difference over $M$'s largest squared, shown)",
            tier: Tier::Analytic,
            source: "an ordered product of exponentials of Hermitian generators [[0, G], [G†, 0]] keeps Σ exactly",
            run: broadband_symplectic,
        },
        Case {
            id: "quantum/broadband-exponential",
            title: r"Quantum: the same source's joint spectrum as one exponential at $g$ = 2: its squeezing against the Schmidt model's $r_k = g\sqrt{\lambda_k}$ (largest difference over the first 10 modes shown)",
            tier: Tier::Analytic,
            source: r"exp [[0, G], [G†, 0]] = [[U cosh S U†, U sinh S V†], …] for G = U S V†, so sinh r_k are G's singular values, g√λ_k",
            run: broadband_exponential,
        },
        Case {
            id: "quantum/broadband-low-gain",
            title: r"Quantum: the sliced source's first squeezing against the single exponential's at $g$ = 0.02 and 0.01 (32 frequencies each, 20 slices): the order in $g$ of their relative difference (shown)",
            tier: Tier::Analytic,
            source: "at low gain the ordered product is the exponential of the sum; the first correction, Magnus's second term, is a passive rotation of order g², which moves the squeezing relatively as g²",
            run: broadband_low_gain,
        },
        Case {
            id: "quantum/broadband-single-mode",
            title: r"Quantum: one signal and one idler frequency, phase-matched, coupling 2 over 10 slices: $r$, $\lvert A \rvert = \cosh r$, $\lvert B/A \rvert = \tanh r$ and the mean photon number $\sinh^2 r$ against $r$ = 2 (largest relative difference shown)",
            tier: Tier::Analytic,
            source: "a two-mode squeezer: a_out = cosh r a + sinh r b†; slices that commute add their couplings",
            run: broadband_single_mode,
        },
        Case {
            id: "quantum/broadband-mismatched",
            title: r"Quantum: one frequency each with a phase mismatch $\Delta k L/2$ = 1 and coupling $\kappa L$ = 3, on 1000 slices: $\lvert B \rvert$ against the phase-mismatched amplifier's $\kappa L \sinh(gL)/gL$, $gL = \sqrt{(\kappa L)^2 - (\Delta k L/2)^2}$ (relative, shown)",
            tier: Tier::Analytic,
            source: "d a/dz = κ e^{iΔkz} b†, d b†/dz = κ e^{−iΔkz} a: B(L) has the modulus (κ/g) sinh gL, g² = κ² − (Δk/2)²",
            run: broadband_mismatched,
        },
        Case {
            id: "quantum/broadband-mismatched-order",
            title: r"Quantum: the same on 100, 200 and 400 slices: the order of convergence of $\lvert B \rvert$ in the slice (shown)",
            tier: Tier::Analytic,
            source: "the exponential midpoint rule, a symmetric product, is second order in the slice",
            run: broadband_mismatched_order,
        },
        Case {
            id: "quantum/broadband-photons",
            title: r"Quantum: the high-gain source's mean photon number from its map, $\mathop{\mathrm{Tr}} BB^\dagger$, against $\sum_k \sinh^2 r_k$ and against its 96-mode Gaussian state's photons (largest relative difference shown)",
            tier: Tier::Analytic,
            source: "B = U sinh R W† (Bloch–Messiah), so Tr BB† = Σ sinh² r_k; each pair of squeezed modes a two-mode squeezed vacuum of r_k",
            run: broadband_photons,
        },
    ]
}

// ---------------------------------------------------------------------------------------------
// Broadband squeezing at any gain
// ---------------------------------------------------------------------------------------------

/// A source with a Gaussian pump of unit bandwidth and a phase mismatch x = 3ν_s − 1.5ν_i, its
/// signal and idler on `n` points from −4 to 4.
fn gvm_grid(n: usize) -> Vec<f64> {
    (0..n)
        .map(|k| -4.0 + 8.0 * k as f64 / (n - 1) as f64)
        .collect()
}

/// The source's pump amplitude at ν_s + ν_i.
fn gvm_pump(nu: f64) -> c64 {
    c64::new((-2.0 * std::f64::consts::LN_2 * nu * nu).exp(), 0.0)
}

/// The source's phase mismatch ΔkL/2.
fn gvm_mismatch(s: f64, i: f64) -> crate::Result<f64> {
    Ok(3.0 * s - 1.5 * i)
}

fn gvm(n: usize, slices: usize, gain: f64) -> Broadband {
    let grid = gvm_grid(n);
    Broadband::sliced(&grid, &grid, gvm_pump, gvm_mismatch, slices, gain).expect(OK)
}

/// The joint spectrum the slices of [`gvm`] add up to: α (1/N) Σ e^{2ixt_n}.
fn gvm_spectrum(n: usize, slices: usize) -> JointSpectrum {
    let grid = gvm_grid(n);
    let (pump, mismatch) = (gvm_pump, gvm_mismatch);
    let mut amplitude = Vec::with_capacity(n * n);
    for &s in &grid {
        for &i in &grid {
            let x = mismatch(s, i).expect(OK);
            let sum: c64 = (0..slices)
                .map(|k| {
                    let t = (k as f64 + 0.5) / slices as f64 - 0.5;
                    c64::from_polar(1.0 / slices as f64, 2.0 * x * t)
                })
                .sum();
            amplitude.push(pump(s + i) * sum);
        }
    }
    JointSpectrum {
        signal: grid.clone(),
        idler: grid,
        amplitude,
    }
}

fn broadband_symplectic() -> Outcome {
    outcome(gvm(48, 40, 3.0).symplectic_error(), 0.0, 1e-12)
}

fn broadband_exponential() -> Outcome {
    let js = gvm_spectrum(48, 40);
    let weights = js.schmidt().expect(OK).weights;
    let b = Broadband::exponential(&js, 2.0).expect(OK);
    let worst = b
        .squeezing()
        .iter()
        .zip(&weights)
        .take(10)
        .map(|(r, l)| (r - 2.0 * l.sqrt()).abs())
        .fold(0.0, f64::max);
    outcome(worst, 0.0, 1e-12)
}

fn broadband_low_gain() -> Outcome {
    let js = gvm_spectrum(32, 20);
    let gap = |g: f64| {
        let sliced = gvm(32, 20, g).squeezing()[0];
        let single = Broadband::exponential(&js, g).expect(OK).squeezing()[0];
        ((sliced - single) / single).abs()
    };
    let order = (gap(0.02) / gap(0.01)).log2();
    outcome(order, 2.0, 0.1)
}

fn one_by_one(coupling: f64, x: f64, slices: usize) -> Broadband {
    let gens: Vec<Matrix> = (0..slices)
        .map(|k| {
            let t = (k as f64 + 0.5) / slices as f64 - 0.5;
            Matrix::from_fn(1, 1, |_, _| {
                c64::from_polar(coupling / slices as f64, 2.0 * x * t)
            })
        })
        .collect();
    Broadband::new(&gens).expect(OK)
}

fn broadband_single_mode() -> Outcome {
    let b = one_by_one(2.0, 0.0, 10);
    let m = b.map();
    let (a, bb) = (m[(0, 0)].norm(), m[(0, 1)].norm());
    let r = 2.0_f64;
    let worst = [
        (b.squeezing()[0] - r).abs() / r,
        (a - r.cosh()).abs() / r.cosh(),
        (bb / a - r.tanh()).abs() / r.tanh(),
        (b.mean_photons() - r.sinh().powi(2)).abs() / r.sinh().powi(2),
    ]
    .into_iter()
    .fold(0.0, f64::max);
    outcome(worst, 0.0, 1e-13)
}

/// The phase-mismatched amplifier's |B| in closed form, κL = 3, ΔkL/2 = 1.
fn mismatched_closed_form() -> f64 {
    let gl = (9.0_f64 - 1.0).sqrt();
    3.0 * gl.sinh() / gl
}

fn mismatched(slices: usize) -> f64 {
    one_by_one(3.0, 1.0, slices).map()[(0, 1)].norm()
}

fn broadband_mismatched() -> Outcome {
    let (fine, coarse) = (mismatched(1000), mismatched(500));
    let exact = mismatched_closed_form();
    // second order: the finer run's error is the difference over 2² − 1
    note_numerical((fine - coarse).abs() / 3.0 / exact);
    relative(fine, exact, 1e-5)
}

fn broadband_mismatched_order() -> Outcome {
    let exact = mismatched_closed_form();
    let e: Vec<f64> = [100, 200, 400]
        .iter()
        .map(|&n| (mismatched(n) - exact).abs())
        .collect();
    let order = 0.5 * ((e[0] / e[1]).log2() + (e[1] / e[2]).log2());
    outcome(order, 2.0, 0.1)
}

fn broadband_photons() -> Outcome {
    let b = gvm(48, 40, 3.0);
    let trace = b.mean_photons();
    let sum: f64 = b.squeezing().iter().map(|r| r.sinh().powi(2)).sum();
    let k = b.squeezing().len();
    let g = b.gaussian(k).expect(OK);
    let gaussian: f64 = (0..k).map(|m| g.mean_photons(m).expect(OK)).sum();
    let worst = ((trace - sum).abs()).max((trace - gaussian).abs()) / trace;
    outcome(worst, 0.0, 1e-10)
}

fn hafnian_ones() -> Outcome {
    let mut e: f64 = 0.0;
    let mut double_factorial = 1.0;
    for n in 1..=12usize {
        double_factorial *= (2 * n - 1) as f64;
        let ones = Matrix::from_fn(2 * n, 2 * n, |_, _| c64::new(1.0, 0.0));
        let h = hafnian(&ones).expect(OK);
        e = e.max((h - double_factorial).norm() / double_factorial);
    }
    worst(e, HAFNIAN_ONES)
}

/// The tolerance of `quantum/hafnian-ones`.
const HAFNIAN_ONES: f64 = 1e-8;

fn hafnian_definition() -> Outcome {
    let mut e: f64 = 0.0;
    for n in (2..=12).step_by(2) {
        let a = random_symmetric(n, n as u64);
        for loops in [false, true] {
            let fast = if loops { loop_hafnian(&a) } else { hafnian(&a) }.expect(OK);
            let exact = by_definition(&a, loops);
            e = e.max((fast - exact).norm() / exact.norm());
        }
    }
    for n in (3..=11).step_by(2) {
        let a = random_symmetric(n, 100 + n as u64);
        let exact = by_definition(&a, true);
        e = e.max((loop_hafnian(&a).expect(OK) - exact).norm() / exact.norm());
    }
    worst(e, 1e-12)
}

fn hafnian_threads() -> Outcome {
    let a = random_symmetric(28, 28);
    let mut g = Gaussian::vacuum(14);
    for k in 0..7 {
        g = g
            .two_mode_squeezed(k, 7 + k, 0.2 + 0.05 * k as f64, 0.0)
            .expect(OK);
    }
    let clicks = [true; 14];
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect(OK)
            .install(|| {
                [
                    hafnian(&a).expect(OK),
                    loop_hafnian(&a).expect(OK),
                    c64::new(g.click_probability(&clicks).expect(OK), 0.0),
                ]
            })
    };
    let (one, seven) = (run(1), run(7));
    let e = (0..3)
        .map(|k| (one[k] - seven[k]).norm())
        .fold(0.0, f64::max);
    worst(e, 0.0)
}

fn squeezed_vacuum() -> Outcome {
    let r: f64 = 0.8;
    let g = Gaussian::vacuum(1).squeezed(0, r, 0.7).expect(OK);
    let t = r.tanh();
    let mut e: f64 = 0.0;
    let (mut mean, mut closed_mean) = (0.0, 0.0);
    for k in 0..=15usize {
        let closed = factorial(2 * k) / (4f64.powi(k as i32) * factorial(k).powi(2))
            * t.powi(2 * k as i32)
            / r.cosh();
        let p = g.probability(&[2 * k]).expect(OK);
        e = e.max((p - closed).abs());
        if k < 15 {
            e = e.max(g.probability(&[2 * k + 1]).expect(OK).abs());
        }
        mean += 2.0 * k as f64 * p;
        closed_mean += 2.0 * k as f64 * closed;
    }
    // the distribution's mean to 30 photons against the closed form's, and the covariance's
    e = e.max((mean - closed_mean).abs());
    e = e.max((g.mean_photons(0).expect(OK) - r.sinh().powi(2)).abs());
    e = e.max((g.click_probability(&[false]).expect(OK) - 1.0 / r.cosh()).abs());
    worst(e, 1e-12)
}

fn two_mode_thermal() -> Outcome {
    let r: f64 = 0.6;
    let g = Gaussian::vacuum(2)
        .two_mode_squeezed(0, 1, r, 0.3)
        .expect(OK);
    let t2 = r.tanh().powi(2);
    let mut e: f64 = 0.0;
    for n in 0..=7usize {
        for m in 0..=7usize {
            let closed = if n == m {
                t2.powi(n as i32) / r.cosh().powi(2)
            } else {
                0.0
            };
            e = e.max((g.probability(&[n, m]).expect(OK) - closed).abs());
        }
    }
    let half = g.reduced(&[1]).expect(OK);
    let thermal = Gaussian::vacuum(1)
        .with_thermal(0, r.sinh().powi(2))
        .expect(OK);
    for (a, b) in half.covariance().iter().zip(thermal.covariance()) {
        e = e.max((a - b).abs());
    }
    let both = g.click_probability(&[true, true]).expect(OK);
    let summed: f64 = (1..=30).map(|n| t2.powi(n) / r.cosh().powi(2)).sum();
    e = e.max((both - summed).abs());
    worst(e, 1e-13)
}

fn squeezed_loss() -> Outcome {
    let guide = Waveguide::new(Dispersion {
        loss_db_per_cm: 4.5,
        ..Dispersion::new(um(1.55), 2.4, 4.2)
    });
    let mut net = Netlist::new();
    net.add("g", Arc::new(guide)).expect(OK);
    net.set("g", "length", 2000.0).expect(OK);
    net.expose("in", "g.o1").expect(OK);
    net.expose("out", "g.o2").expect(OK);
    let t = Transfer::of(&net.compile().expect(OK), um(1.55)).expect(OK);
    let r: f64 = 1.2;
    let input = Gaussian::vacuum(2).squeezed(0, r, 0.0).expect(OK);
    let out = input.through(&t).expect(OK);
    let eta = t.matrix()[(1, 0)].norm_sqr();
    let (low, high) = out.quadrature_extremes(1).expect(OK);
    let e = (low - (eta * (-2.0 * r).exp() + 1.0 - eta))
        .abs()
        .max((high - (eta * (2.0 * r).exp() + 1.0 - eta)).abs());
    worst(e, 1e-13)
}

fn displaced_states() -> Outcome {
    let alpha = c64::new(0.7, -0.4);
    let g = Gaussian::vacuum(1).displaced(0, alpha).expect(OK);
    let mu = alpha.norm_sqr();
    let mut e: f64 = 0.0;
    for n in 0..=15usize {
        let p = g.probability(&[n]).expect(OK);
        e = e.max((p - (-mu).exp() * mu.powi(n as i32) / factorial(n)).abs());
    }
    let g = Gaussian::vacuum(1)
        .squeezed(0, 0.3, 1.1)
        .expect(OK)
        .displaced(0, c64::new(0.4, 0.5))
        .expect(OK);
    let (mut m1, mut m2) = (0.0, 0.0);
    for n in 0..=30usize {
        let p = g.probability(&[n]).expect(OK);
        m1 += n as f64 * p;
        m2 += (n * n) as f64 * p;
    }
    e = e
        .max((m1 - g.mean_photons(0).expect(OK)).abs())
        .max((m2 - m1 * m1 - g.photon_covariance(0, 0).expect(OK)).abs());
    worst(e, 1e-11)
}

/// Every click pattern of `modes` modes, as the binary digits of 0 to 2^modes − 1.
fn click_patterns(modes: usize) -> impl Iterator<Item = Vec<bool>> {
    (0..1usize << modes).map(move |p| (0..modes).map(|m| p >> m & 1 == 1).collect())
}

fn clicks_coherent() -> Outcome {
    let alpha = [
        c64::new(0.6, 0.0),
        c64::new(0.0, 0.3),
        c64::new(0.8, 0.2),
        c64::new(-0.4, 0.5),
    ];
    // a Haar-random interferometer with 30% loss in every path
    let u = super::mesh::haar_unitary(4, 11);
    let t = Matrix::from_fn(4, 4, |r, c| u[(r, c)] * 0.7f64.sqrt());
    let mut g = Gaussian::vacuum(4);
    for (m, &a) in alpha.iter().enumerate() {
        g = g.displaced(m, a).expect(OK);
    }
    let out = g.transformed(&t).expect(OK);
    // b = T a: each output a coherent state of amplitude (T α)_j, dark with e^(−|(T α)_j|²)
    let dark: Vec<f64> = (0..4)
        .map(|j| {
            let b: c64 = (0..4).map(|k| t[(j, k)] * alpha[k]).sum();
            (-b.norm_sqr()).exp()
        })
        .collect();
    let (mut e, mut total): (f64, f64) = (0.0, 0.0);
    for clicks in click_patterns(4) {
        let p = out.click_probability(&clicks).expect(OK);
        let closed: f64 = (0..4)
            .map(|j| if clicks[j] { 1.0 - dark[j] } else { dark[j] })
            .product();
        e = e.max((p - closed).abs());
        total += p;
    }
    worst(e.max((total - 1.0).abs()), 1e-13)
}

fn clicks_displaced() -> Outcome {
    let g = Gaussian::vacuum(2)
        .two_mode_squeezed(0, 1, 0.1, 0.4)
        .expect(OK)
        .displaced(0, c64::new(0.3, 0.0))
        .expect(OK)
        .displaced(1, c64::new(0.0, 0.2))
        .expect(OK);
    // the photon numbers' probabilities to 15 in each mode, summed into the four click
    // patterns; what lies beyond, 1 − their sum, must be below round-off for the sums to be the
    // clicks' reference
    let mut by_pattern = [0.0f64; 4];
    for n in 0..=15usize {
        for m in 0..=15usize {
            by_pattern[usize::from(n > 0) + 2 * usize::from(m > 0)] +=
                g.probability(&[n, m]).expect(OK);
        }
    }
    let tail = 1.0 - by_pattern.iter().sum::<f64>();
    let mut e: f64 = if tail.abs() < 1e-14 {
        0.0
    } else {
        f64::INFINITY
    };
    for (k, clicks) in click_patterns(2).enumerate() {
        e = e.max((g.click_probability(&clicks).expect(OK) - by_pattern[k]).abs());
    }
    worst(e, 1e-13)
}

fn source_pairs() -> Outcome {
    let k = 4usize;
    let s = Source::new(&[1.0; 4], 0.5).expect(OK);
    let t = s.squeezing()[0].tanh().powi(2);
    let p = s.pairs(10);
    let mut e: f64 = 0.0;
    for (n, &pn) in p.iter().enumerate() {
        let binomial = factorial(n + k - 1) / (factorial(n) * factorial(k - 1));
        e = e.max((pn - binomial * t.powi(n as i32) * (1.0 - t).powi(k as i32)).abs());
    }
    e = e.max((s.mean_pairs() - 0.5).abs());
    e = e.max((s.unheralded_g2() - (1.0 + 1.0 / k as f64)).abs());
    e = e.max((Source::new(&[1.0], 0.3).expect(OK).unheralded_g2() - 2.0).abs());
    worst(e, 1e-14)
}

/// A two-dimensional Gaussian amplitude exp(−½ zᵀMz) on a grid of `n` points over ±`half`.
fn gaussian_spectrum(m: [f64; 3], half: f64, n: usize) -> JointSpectrum {
    let axis: Vec<f64> = (0..n)
        .map(|k| -half + 2.0 * half * k as f64 / (n - 1) as f64)
        .collect();
    let [a, b, c] = m;
    let amplitude = axis
        .iter()
        .flat_map(|&s| {
            axis.iter().map(move |&i| {
                c64::new(
                    (-0.5 * (a * s * s + 2.0 * c * s * i + b * i * i)).exp(),
                    0.0,
                )
            })
        })
        .collect();
    JointSpectrum {
        signal: axis.clone(),
        idler: axis,
        amplitude,
    }
}

/// Tr(ρ_A ρ_B) of two Gaussian joint spectra in closed form.
fn gaussian_overlap(ma: [f64; 3], mb: [f64; 3]) -> f64 {
    let q = |m: [f64; 3]| {
        let [a, b, c] = m;
        let d = c * c / (2.0 * b);
        [a - d, -d]
    };
    let (qa, qb) = (q(ma), q(mb));
    let (diag, off) = (qa[0] + qb[0], qa[1] + qb[1]);
    let det_q = diag * diag - off * off;
    let det = |m: [f64; 3]| m[0] * m[1] - m[2] * m[2];
    2.0 * (det(ma) * det(mb)).sqrt() / (ma[1] * mb[1] * det_q).sqrt()
}

fn source_hom_gaussian() -> Outcome {
    let (ma, mb) = ([1.0, 1.3, 0.6], [1.4, 0.9, -0.3]);
    let (a, b) = (
        gaussian_spectrum(ma, 9.0, 161),
        gaussian_spectrum(mb, 9.0, 161),
    );
    let e = (hom_visibility(&a, &b).expect(OK) - gaussian_overlap(ma, mb))
        .abs()
        .max((hom_visibility(&a, &a).expect(OK) - gaussian_purity(ma[0], ma[1], ma[2])).abs());
    worst(e, 1e-12)
}

fn heralding_coupler() -> Outcome {
    let spectrum = gaussian_spectrum([3e6, 2e6, 1e6], 3e-3, 41);
    // move the grid to 1.55 µm
    let spectrum = JointSpectrum {
        signal: spectrum.signal.iter().map(|v| v + 1.0 / 1.55).collect(),
        idler: spectrum.idler.iter().map(|v| v + 1.0 / 1.56).collect(),
        amplitude: spectrum.amplitude,
    };
    let lossy = Coupler::new().with_excess_loss(0.5).expect(OK);
    let values = [0.3];
    let to = |port: &str| {
        heralding_through(&spectrum, &lossy, &values, ("o1", port), ("o2", "o4")).expect(OK)
    };
    let (a, b) = (to("o3"), to("o4"));
    // the idler's path, o2 to o4, passes every idler alike: its heralding is that power
    let s = lossy.s_matrix(um(1.56), &values).expect(OK);
    let ports: Vec<&str> = lossy.ports().iter().map(|p| p.name.as_str()).collect();
    let at = |name: &str| ports.iter().position(|p| *p == name).expect(OK);
    let idler = s[(at("o4"), at("o2"))].norm_sqr();
    let e = (a.signal + b.signal - 10f64.powf(-0.05))
        .abs()
        .max((a.idler - idler).abs());
    worst(e, 1e-14)
}

// ---------------------------------------------------------------------------------------------
// Rings
// ---------------------------------------------------------------------------------------------

/// Hertz to 1/µm.
fn per_um(hz: f64) -> f64 {
    hz / (SPEED_OF_LIGHT * 1e6)
}

/// Helt et al.'s ring: R = 47.5 µm, v = 171 µm/ps (n_g = 1.7532), resonances 1.48 GHz wide.
pub(crate) struct HeltRing {
    ring: AllPassRing,
    length: f64,
    /// The power coupling that makes a resonance 1.48 GHz wide.
    coupling: f64,
    /// The pump's, signal's and idler's resonances, 1/µm.
    resonances: [f64; 3],
}

pub(crate) fn helt_ring(fwhm_hz: f64) -> HeltRing {
    let ng = SPEED_OF_LIGHT * 1e-6 / 171.0;
    let length = TAU * 47.5;
    // the effective index puts order 310 at 1.55 µm: n_eff L = 310 λ
    let ring = AllPassRing::new(Dispersion::new(um(1.55), 310.0 * 1.55 / length, ng)).expect(OK);
    let fsr = 1.0 / (ng * length);
    let resonance = |nu: f64| -> f64 {
        1.0 / ring
            .resonance(Wavelength::um(1.0 / nu).expect(OK), length)
            .expect(OK)
            .to_um()
    };
    let pump = resonance(1.0 / 1.55);
    let resonances = [pump, resonance(pump + fsr), resonance(pump - fsr)];
    // the coupling whose resonance at the pump is fwhm_hz wide: bisection on the component's
    // own width (µm, Bogaerts's Eq. 7)
    let target = per_um(fwhm_hz);
    let lam = 1.0 / resonances[0];
    let width = |k2: f64| -> f64 {
        let w = ring
            .fwhm(Wavelength::um(lam).expect(OK), &[length, k2])
            .expect(OK);
        w / (lam * lam)
    };
    let (mut lo, mut hi): (f64, f64) = (1e-6, 0.5);
    for _ in 0..100 {
        let mid = (lo * hi).sqrt();
        if width(mid) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    HeltRing {
        ring,
        length,
        coupling: (lo * hi).sqrt(),
        resonances,
    }
}

impl HeltRing {
    /// F(ν) of a wave in this ring whose round trip loses `extra` of its power besides the
    /// coupling (a second channel's coupling, for Vernon et al.'s pump).
    fn enhancement(&self, extra: f64) -> impl Fn(f64) -> crate::Result<c64> + Sync + '_ {
        move |nu: f64| {
            let w = Wavelength::um(1.0 / nu)?;
            let round =
                self.ring.round_trip(w, &[self.length, self.coupling])? * (1.0 - extra).sqrt();
            let r = (1.0 - self.coupling).sqrt();
            Ok(c64::new(0.0, self.coupling.sqrt()) / (1.0 - r * round))
        }
    }
}

/// The Schmidt number of pairs from Helt et al.'s ring pumped by a transform-limited Gaussian
/// pulse of intensity FWHM `pulse` ps, the pump's resonance `ratio` times wider than the
/// signal's and idler's, on a grid `span` linewidths either side and `points` across.
pub(crate) fn ring_schmidt(pulse: f64, ratio: f64, span: f64, points: usize) -> f64 {
    let ring = helt_ring(1.48e9);
    // the pump's extra round-trip loss that widens its resonance by `ratio`, by bisection
    let extra = if ratio == 1.0 {
        0.0
    } else {
        let probe = |x: f64| {
            let f = ring.enhancement(x);
            let peak = f(ring.resonances[0]).expect(OK).norm_sqr();
            // the half width: where |F|² falls to half, by bisection in ν
            let (mut lo, mut hi) = (0.0, per_um(200e9));
            for _ in 0..100 {
                let mid = 0.5 * (lo + hi);
                if f(ring.resonances[0] + mid).expect(OK).norm_sqr() > 0.5 * peak {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            2.0 * lo
        };
        let target = ratio * per_um(1.48e9);
        let (mut lo, mut hi) = (0.0, 0.9);
        for _ in 0..80 {
            let mid = 0.5 * (lo + hi);
            if probe(mid) < target {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    };
    let linewidth = per_um(1.48e9);
    // the pump: intensity FWHM 0.4413/T in frequency for a transform-limited Gaussian
    let pump = Pump {
        center: ring.resonances[0],
        bandwidth: per_um(4.0 * std::f64::consts::LN_2 / (TAU * pulse * 1e-12)),
    };
    let fp = ring.enhancement(extra);
    let fs = ring.enhancement(0.0);
    let pump_half = (8.0 * pump.bandwidth).min(100.0 * ratio * linewidth);
    let grid = RingGrid {
        half: span * linewidth,
        points,
        pump_half,
        pump_points: 4001,
    };
    let js = ring_spectrum(
        pump,
        [&fp, &fs, &fs],
        [ring.resonances[1], ring.resonances[2]],
        grid,
    )
    .expect(OK);
    js.schmidt().expect(OK).number()
}

fn ring_helt_short() -> Outcome {
    let k = ring_schmidt(5.0, 1.0, 20.0, 601);
    let coarse = ring_schmidt(5.0, 1.0, 10.0, 301);
    note_numerical((k - coarse).abs());
    outcome(k, 1.09, 0.005)
}

fn ring_helt_long() -> Outcome {
    let k = ring_schmidt(5000.0, 1.0, 10.0, 1201);
    let coarse = ring_schmidt(5000.0, 1.0, 5.0, 601);
    note_numerical((k - coarse).abs() / k);
    relative(k, 10.0, 0.005 / 0.74)
}

fn ring_vernon_equal() -> Outcome {
    let k = ring_schmidt(0.05, 1.0, 20.0, 601);
    let coarse = ring_schmidt(0.05, 1.0, 10.0, 301);
    note_numerical((k - coarse).abs());
    outcome(k, 1.09, 0.005)
}

// ---------------------------------------------------------------------------------------------
// Squeezers
// ---------------------------------------------------------------------------------------------

fn opa(steps: usize) -> Squeezer {
    let shg = SecondHarmonic {
        coupling: 0.01,
        mismatch: 0.0,
        loss: [0.0, 0.0],
    };
    degenerate(&shg, c64::new(1.0, 0.0), 1000.0, steps).expect(OK)
}

fn squeezer_degenerate() -> Outcome {
    let (s, half) = (opa(2000), opa(1000));
    // Richardson: the finer run's error is the difference over 2⁴ − 1
    note_numerical((s.r() - half.r()).abs() / 15.0);
    // and a non-degenerate amplifier's map in photon units (signal 0.6 µm, idler 0.5 µm)
    let three = ThreeWave {
        frequencies: [1.0 / 0.6, 1.0 / 0.5],
        coupling: 0.01,
        mismatch: 0.0,
        loss: [0.0; 3],
    };
    let nd = non_degenerate(&three, c64::new(1.0, 0.0), 1000.0, 2000).expect(OK);
    let e = (s.r() - 0.01 * 1000.0)
        .abs()
        .max(s.bogoliubov_error())
        .max(nd.bogoliubov_error());
    worst(e, 1e-9)
}

fn squeezer_four_wave() -> Outcome {
    let (gamma, power, mismatch, length): (f64, f64, f64, f64) = (1.0, 2.0, -3.0, 1.0);
    let fwm = FourWave { gamma, mismatch };
    let run = |steps: usize| {
        four_wave(&fwm, c64::new(power.sqrt(), 0.0), length, steps)
            .expect(OK)
            .r()
            .cosh()
            .powi(2)
    };
    let (g2000, g1000) = (run(2000), run(1000));
    let g = signal_gain(gamma, power, mismatch, length);
    note_numerical((g2000 - g1000).abs() / g / 15.0);
    relative(g2000, g, 1e-9)
}

fn squeezer_rk4_order() -> Outcome {
    let r: Vec<f64> = [80usize, 160, 320].iter().map(|&n| opa(n).r()).collect();
    let order = ((r[0] - r[1]) / (r[1] - r[2])).abs().log2();
    outcome((order * 100.0).round() / 100.0, 4.0, 0.2)
}

// ---------------------------------------------------------------------------------------------
// Published squeezing
// ---------------------------------------------------------------------------------------------

fn db(x: f64) -> f64 {
    10.0 * x.log10()
}

fn vaidya_number_difference() -> Outcome {
    // the noise reduction factor at several gains, all 1 − η; shown in dB at the largest
    let eta = 0.8;
    let mut worst_ratio: f64 = 0.0;
    let mut shown = 0.0;
    for r in [0.1, 0.5, 1.0, 2.0] {
        let g = Gaussian::vacuum(2)
            .two_mode_squeezed(0, 1, r, 0.0)
            .expect(OK)
            .lossy(0, eta)
            .expect(OK)
            .lossy(1, eta)
            .expect(OK);
        let var = g.photon_covariance(0, 0).expect(OK) + g.photon_covariance(1, 1).expect(OK)
            - 2.0 * g.photon_covariance(0, 1).expect(OK);
        let ratio = var / (g.mean_photons(0).expect(OK) + g.mean_photons(1).expect(OK));
        worst_ratio = worst_ratio.max((ratio - (1.0 - eta)).abs());
        shown = -db(ratio);
    }
    // the ratio is 1 − η at every gain, to round-off; a model that broke it fails the case
    if worst_ratio > 1e-12 {
        return outcome(f64::NAN, 7.0, 0.5);
    }
    outcome(shown, 7.0, 0.5)
}

/// The squeezing (dB, positive) of a squeezed vacuum of r = 10 through a transmission η.
fn bound(eta: f64) -> f64 {
    let g = Gaussian::vacuum(1)
        .squeezed(0, 10.0, 0.0)
        .expect(OK)
        .lossy(0, eta)
        .expect(OK);
    -g.squeezing_db(0).expect(OK)
}

fn vaidya_outcoupling() -> Outcome {
    let a = bound(10f64.powf(-0.15));
    let b = bound(10f64.powf(-0.05));
    let (ea, eb) = ((a - 5.0).abs(), (b - 10.0).abs());
    if ea >= eb {
        outcome(a, 5.0, 0.5)
    } else {
        outcome(b, 10.0, 0.5)
    }
}

/// Nehra et al.'s loss model solved for η and r from their measured squeezing and
/// anti-squeezing: (r in dB, 1 − η).
pub(crate) fn nehra_solved() -> (f64, f64) {
    let minus = 10f64.powf(-0.42);
    let plus = 10f64.powf(0.97);
    // η(1/x − 1) = minus − 1, η(x − 1) = plus − 1, x = e^{2r}: x = (plus − 1)/(1 − minus)
    let x = (plus - 1.0) / (1.0 - minus);
    let eta = (plus - 1.0) / (x - 1.0);
    // through the Gaussian state: squeeze, lose, read both quadratures back
    let r = 0.5 * x.ln();
    let g = Gaussian::vacuum(1)
        .squeezed(0, r, 0.0)
        .expect(OK)
        .lossy(0, eta)
        .expect(OK);
    let (low, high) = g.quadrature_extremes(0).expect(OK);
    debug_assert!((low - minus).abs() < 1e-12 && (high - plus).abs() < 1e-12);
    (db_of_squeezing(r), 1.0 - eta)
}

fn nehra_inferred() -> Outcome {
    let (squeezing, _) = nehra_solved();
    outcome(squeezing, 10.48, 0.87)
}

fn nehra_all_optical() -> Outcome {
    // squeezing r1 = 11 dB, then a second squeezer of 33 dB gain amplifying the squeezed (or
    // the anti-squeezed) quadrature; the photons with and without the first
    let r1 = 11.0 / db_of_squeezing(1.0);
    let r2 = 33.0 / db_of_squeezing(1.0);
    let photons = |first: f64, phase: f64| {
        Gaussian::vacuum(1)
            .squeezed(0, first, 0.0)
            .expect(OK)
            .squeezed(0, r2, phase)
            .expect(OK)
            .mean_photons(0)
            .expect(OK)
    };
    // the second squeezer anti-aligned (φ = π) amplifies the first's squeezed quadrature
    let read = db(photons(r1, PI) / photons(0.0, PI));
    outcome(-read, 11.0, 0.5)
}

fn nehra_homodyne_bound() -> Outcome {
    let b = bound(0.20);
    // the paper's bound: below 1 dB
    outcome(b, 0.5, 0.5)
}

#[cfg(test)]
mod tests {
    use super::super::intermodal::{DualPump, dual_pump_spectrum, shaped_dual_pump_spectrum};
    use super::*;
    use crate::material::Material;
    use crate::material::catalogue::{Conditions, entry};
    use crate::mode::pockels::Along;
    use crate::mode::vector;
    use crate::nonlinear::guide::{Family, Grid, ModeIndex, Ridge};

    // ---------------------------------------------------------------------------------------------
    // Paesani et al.'s inter-modal source
    // ---------------------------------------------------------------------------------------------

    /// Paesani et al.'s multimode guide: 2 µm × 220 nm silicon in silica.
    fn paesani_ridge() -> Ridge {
        let iso = |id: &str| -> Material {
            entry(id)
                .expect(OK)
                .default_model()
                .expect(OK)
                .materials(Conditions::default())
                .expect(OK)
                .remove(0)
        };
        Ridge {
            film: 0.22,
            etch: 0.22,
            width: 2.0,
            angle: 90.0,
            core: iso("si"),
            substrate: iso("sio2"),
            cladding: iso("sio2"),
            along: Along::Y,
        }
    }

    /// The effective index over 1.45 to 1.65 µm of the TM-like mode of `order` (0 or 1) on cells
    /// of `h` µm (half the window behind the wall its symmetry takes).
    fn paesani_index(order: usize, h: f64) -> ModeIndex {
        let ridge = paesani_ridge();
        // TM₀'s E_y is even across the guide (a magnetic wall), TM₁'s odd (an electric wall)
        let family = if order == 0 { Family::Tm } else { Family::Te };
        let grid = Grid::new(h).halved();
        ModeIndex::sample(
            |w| {
                let cs = ridge.cross_section(w, &grid, family)?;
                vector::modes(&cs, w, 6, Some(1.9))?
                    .into_iter()
                    .filter(|m| m.te_fraction() < 0.5)
                    .max_by(|a, b| a.effective_index().re.total_cmp(&b.effective_index().re))
                    .ok_or_else(|| crate::Error::invalid("paesani", "no TM-like mode"))
            },
            1.45,
            1.65,
            1e-6,
        )
        .expect(OK)
    }

    /// The purity of Paesani et al.'s source with the pumps delayed by `delay` ps and a Gaussian
    /// pump of `fwhm` µm at 1.55 µm, from the TM₀ and TM₁ indices `tm0`, `tm1`, on a `points`
    /// grid; and the signal's and idler's wavelengths.
    fn paesani_purity(
        tm0: &ModeIndex,
        tm1: &ModeIndex,
        delay: f64,
        fwhm: f64,
        points: usize,
    ) -> (f64, f64, f64) {
        paesani_shaped_purity(tm0, tm1, delay, fwhm, points, None)
    }

    /// Paesani et al.'s measured pump (their Supplementary Fig. 2b, p. 3, by FROG), read by pixel
    /// from the figure's raster: the intensity (peak 1) and the spectral phase (rad) from 1540 to
    /// 1560 nm in 0.1 nm steps. The curves' axes were placed by the figure's tick marks; where the
    /// two curves cross, or the intensity lies on the axis, the reading is interpolated. It peaks
    /// at 1549.6 nm, is half its peak from 1548.0 to 1552.5 nm and has its centroid at 1550.39 nm.
    #[rustfmt::skip]
    const PAESANI_PUMP: [(f64, f64); 201] = [
        (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550),
        (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550),
        (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550),
        (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550),
        (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550),
        (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550),
        (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550), (0.0000, 0.9550),
        (0.0000, 0.9550), (0.0000, 0.9550), (0.0050, 0.9524), (0.0050, 0.9470), (0.0050, 0.9358),
        (0.0050, 0.9284), (0.0054, 0.9149), (0.0057, 0.9017), (0.0061, 0.8881), (0.0069, 0.8749),
        (0.0078, 0.8622), (0.0087, 0.8518), (0.0098, 0.8443), (0.0110, 0.8409), (0.0122, 0.8505),
        (0.0139, 0.8711), (0.0155, 0.9031), (0.0173, 0.9447), (0.0188, 0.9905), (0.0203, 1.0409),
        (0.0218, 1.0903), (0.0227, 1.1373), (0.0234, 1.1772), (0.0233, 1.2099), (0.0220, 1.2307),
        (0.0208, 1.2338), (0.0195, 1.2316), (0.0187, 1.2225), (0.0198, 1.2051), (0.0230, 1.1863),
        (0.0281, 1.1623), (0.0346, 1.1335), (0.0422, 1.1027), (0.0502, 1.0695), (0.0589, 1.0347),
        (0.0699, 0.9970), (0.0858, 0.9598), (0.1090, 0.9209), (0.1414, 0.8649), (0.1831, 0.7955),
        (0.2335, 0.7171), (0.2910, 0.6277), (0.3534, 0.5366), (0.4291, 0.4245), (0.4838, 0.3560),
        (0.5471, 0.2735), (0.6070, 0.2064), (0.6637, 0.1531), (0.7165, 0.1172), (0.7653, 0.0985),
        (0.8095, 0.0835), (0.8488, 0.0685), (0.8830, 0.0558), (0.9117, 0.0433), (0.9359, 0.0332),
        (0.9555, 0.0239), (0.9714, 0.0147), (0.9834, 0.0076), (0.9921, 0.0047), (0.9972, 0.0022),
        (0.9999, -0.0019), (1.0000, 0.0021), (0.9990, 0.0063), (0.9950, 0.0108), (0.9895, 0.0185),
        (0.9828, 0.0276), (0.9744, 0.0382), (0.9646, 0.0500), (0.9535, 0.0607), (0.9412, 0.0724),
        (0.9272, 0.0840), (0.9124, 0.0938), (0.8964, 0.1045), (0.8794, 0.1151), (0.8617, 0.1269),
        (0.8430, 0.1385), (0.8236, 0.1513), (0.8032, 0.1641), (0.7822, 0.1764), (0.7605, 0.1881),
        (0.7382, 0.1980), (0.7153, 0.2078), (0.6923, 0.2165), (0.6691, 0.2214), (0.6454, 0.2269),
        (0.6218, 0.2306), (0.5979, 0.2339), (0.5742, 0.2367), (0.5502, 0.2387), (0.5266, 0.2390),
        (0.5032, 0.2439), (0.4799, 0.2478), (0.4568, 0.2457), (0.4339, 0.2491), (0.4116, 0.2516),
        (0.3897, 0.2500), (0.3683, 0.2508), (0.3474, 0.2454), (0.3270, 0.2396), (0.3071, 0.2315),
        (0.2877, 0.2209), (0.2687, 0.2080), (0.2533, 0.1919), (0.2299, 0.1849), (0.2136, 0.1632),
        (0.1959, 0.1476), (0.1783, 0.1324), (0.1607, 0.1189), (0.1432, 0.1046), (0.1259, 0.0847),
        (0.1088, 0.0670), (0.0924, 0.0442), (0.0767, 0.0224), (0.0620, -0.0007), (0.0487, -0.0220),
        (0.0366, -0.0429), (0.0266, -0.0583), (0.0183, -0.0722), (0.0118, -0.0821),
        (0.0071, -0.0843), (0.0050, -0.0846), (0.0000, -0.0774), (0.0000, -0.0643),
        (0.0000, -0.0468), (0.0000, -0.0292), (0.0000, -0.0066), (0.0000, 0.0166),
        (0.0050, 0.0412), (0.0050, 0.0638), (0.0050, 0.0853), (0.0050, 0.1036), (0.0050, 0.1218),
        (0.0050, 0.1359), (0.0050, 0.1511), (0.0050, 0.1670), (0.0050, 0.1831), (0.0050, 0.1984),
        (0.0050, 0.2137), (0.0050, 0.2268), (0.0050, 0.2392), (0.0050, 0.2488), (0.0000, 0.2561),
        (0.0000, 0.2622), (0.0000, 0.2611), (0.0000, 0.2611), (0.0000, 0.2611), (0.0000, 0.2611),
        (0.0000, 0.2611), (0.0000, 0.2611), (0.0000, 0.2611), (0.0000, 0.2611), (0.0000, 0.2611),
        (0.0000, 0.2611), (0.0000, 0.2611), (0.0000, 0.2611), (0.0000, 0.2611), (0.0000, 0.2611),
        (0.0000, 0.2611), (0.0000, 0.2611), (0.0000, 0.2611), (0.0000, 0.2611), (0.0000, 0.2611),
        (0.0000, 0.2611), (0.0000, 0.2611), (0.0000, 0.2611), (0.0000, 0.2611),
    ];

    /// The measured pump's amplitude at ν (1/µm): I^p e^{iσφ} (p = ½ takes the plotted intensity
    /// as |α|²), linear between the table's wavelengths, zero outside them; σ = 0 leaves the
    /// phase out.
    fn paesani_pump(sign: f64, power: f64) -> impl Fn(f64) -> c64 + Sync {
        move |nu: f64| {
            let x = (1e3 / nu - 1540.0) / 0.1;
            if !(0.0..200.0).contains(&x) {
                return c64::new(0.0, 0.0);
            }
            let k = x.floor() as usize;
            let t = x - k as f64;
            let (i0, p0) = PAESANI_PUMP[k];
            let (i1, p1) = PAESANI_PUMP[k + 1];
            let intensity = (i0 + t * (i1 - i0)).max(0.0);
            c64::from_polar(intensity.powf(power), sign * (p0 + t * (p1 - p0)))
        }
    }

    /// Other readings of the measured pump, without its phase: the plotted spectrum taken per
    /// unit wavelength (`Jacobian`, times λ² to make it per unit frequency), the curve's
    /// line-width floor of 0.005 taken off (`Floor`), and the amplitude, not the intensity,
    /// interpolated between the read points (`Amplitude`).
    #[derive(Clone, Copy, Debug)]
    enum Reading {
        Jacobian,
        Floor,
        Amplitude,
    }

    fn paesani_pump_read(reading: Reading) -> impl Fn(f64) -> c64 + Sync {
        move |nu: f64| {
            let x = (1e3 / nu - 1540.0) / 0.1;
            if !(0.0..200.0).contains(&x) {
                return c64::new(0.0, 0.0);
            }
            let k = x.floor() as usize;
            let t = x - k as f64;
            let (i0, i1) = (PAESANI_PUMP[k].0.max(0.0), PAESANI_PUMP[k + 1].0.max(0.0));
            let intensity = i0 + t * (i1 - i0);
            let amplitude = match reading {
                Reading::Jacobian => (intensity * (1e3 / nu / 1550.0).powi(2)).sqrt(),
                Reading::Floor => (intensity - 0.005).max(0.0).sqrt(),
                Reading::Amplitude => i0.sqrt() + t * (i1.sqrt() - i0.sqrt()),
            };
            c64::new(amplitude, 0.0)
        }
    }

    /// [`paesani_purity`] with the pump's amplitude `shaped` in place of the Gaussian when given.
    fn paesani_shaped_purity(
        tm0: &ModeIndex,
        tm1: &ModeIndex,
        delay: f64,
        fwhm: f64,
        points: usize,
        shaped: Option<&(dyn Fn(f64) -> c64 + Sync)>,
    ) -> (f64, f64, f64) {
        let beta = |m: &ModeIndex| {
            let m = m.clone();
            move |nu: f64| TAU * nu * m.n_eff(1.0 / nu)
        };
        let (b0, b1) = (beta(tm0), beta(tm1));
        let pump_nu = 1.0 / 1.55;
        // the non-degenerate phase match: β₀(ν_p) + β₁(ν_p) = β₁(ν_s) + β₀(2ν_p − ν_s), the signal
        // near 1.588 µm in TM₁
        let mismatch = |s: f64| b0(pump_nu) + b1(pump_nu) - b1(s) - b0(2.0 * pump_nu - s);
        let (mut lo, mut hi) = (1.0 / 1.62, 1.0 / 1.565);
        let f_lo = mismatch(lo);
        for _ in 0..100 {
            let mid = 0.5 * (lo + hi);
            if (mismatch(mid) > 0.0) == (f_lo > 0.0) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let signal = 0.5 * (lo + hi);
        let idler = 2.0 * pump_nu - signal;
        let guide = DualPump {
            beta: [&b0, &b1, &b1, &b0],
            length: 11_000.0,
            delay,
            pump: Pump {
                center: pump_nu,
                bandwidth: fwhm / (1.55 * 1.55),
            },
        };
        let js = match shaped {
            None => dual_pump_spectrum(
                &guide,
                [signal, idler],
                0.006,
                points,
                0.008,
                2 * points + 1,
            ),
            Some(alpha) => shaped_dual_pump_spectrum(
                &guide,
                alpha,
                [signal, idler],
                0.006,
                points,
                0.008,
                2 * points + 1,
            ),
        }
        .expect(OK);
        (js.schmidt().expect(OK).purity(), 1.0 / signal, 1.0 / idler)
    }

    #[test]
    #[ignore = "prints the quantum-light cases' numbers"]
    fn print_numbers() {
        for case in cases() {
            let start = std::time::Instant::now();
            let o = (case.run)();
            eprintln!(
                "{:40} measured {:.10e} expected {:.6e} error {:.3e} tol {:.3e} ({:.1} s)",
                case.id,
                o.measured,
                o.expected,
                o.error,
                o.tolerance,
                start.elapsed().as_secs_f64()
            );
        }
    }

    /// The broadband source of the cases at rising gain: its first squeezings against the
    /// Schmidt model's, its number of modes, and how far its first signal mode has moved from
    /// the low-gain one (1 − |⟨u₁(g)|u₁(0.01)⟩|²).
    #[test]
    #[ignore = "prints the broadband source's squeezing and modes against the gain"]
    fn print_broadband() {
        let js = gvm_spectrum(48, 40);
        let low = gvm(48, 40, 0.01);
        let u0 = low.signal_mode(0).unwrap();
        for g in [0.5, 1.0, 2.0, 3.0] {
            for slices in [40, 80] {
                let b = gvm(48, slices, g);
                let e = Broadband::exponential(&js, g).unwrap();
                let u = b.signal_mode(0).unwrap();
                let overlap: c64 = u.iter().zip(u0).map(|(a, b)| a.conj() * b).sum();
                eprintln!(
                    "g {g} slices {slices}: r {:.4} {:.4} {:.4} (one exponential {:.4} {:.4} {:.4}), modes {:.4} ({:.4}), photons {:.4} ({:.4}), mode moved {:.2e}",
                    b.squeezing()[0],
                    b.squeezing()[1],
                    b.squeezing()[2],
                    e.squeezing()[0],
                    e.squeezing()[1],
                    e.squeezing()[2],
                    b.mode_number(),
                    e.mode_number(),
                    b.mean_photons(),
                    e.mean_photons(),
                    1.0 - overlap.norm_sqr()
                );
            }
        }
    }

    /// Vernon et al.'s Fig. 3: the purity's line against Q_SI/Q_P, read from the figure's vector
    /// paths, against Eq. 3 here with the signal's and idler's frequencies cut at ±3 pump
    /// linewidths (3 × ratio signal linewidths), and with no such cut (±20 signal linewidths).
    /// The line's first vertex, 0.8396 at a ratio of 0.5, needs a pump resonance narrower than
    /// the signal's, which `ring_schmidt` (it widens the pump's by an extra loss) doesn't make.
    #[test]
    #[ignore = "Vernon et al.'s Fig. 3 against a window of three pump linewidths: seconds"]
    fn print_vernon_fig3() {
        // (Q_SI/Q_P, the line's purity), from the vector path's vertices
        let line = [
            (1.0, 0.9273),
            (1.5, 0.9592),
            (2.0, 0.9740),
            (3.0, 0.9868),
            (4.0, 0.9921),
        ];
        for (ratio, read) in line {
            let span = 3.0 * ratio;
            let per = |n: f64| (2.0 * span * n) as usize + 1;
            let fine = 1.0 / ring_schmidt(0.05, ratio, span, per(40.0));
            let coarse = 1.0 / ring_schmidt(0.05, ratio, span, per(20.0));
            let open = 1.0 / ring_schmidt(0.05, ratio, 20.0, 801);
            eprintln!(
                "ratio {ratio}: line {read:.4}, ±3 pump linewidths {fine:.5} (grid {:.1e}, off {:+.5}), none {open:.5}",
                (fine - coarse).abs(),
                fine - read
            );
        }
    }

    /// Another reading of Vernon et al.'s Fig. 3 ("maximum heralded state purity") and of their
    /// text's K = 1.08 ("the lower bound" for a Gaussian pulse): the purity at the best Gaussian
    /// pulse (by golden-section search on its length, 0.05 to 50 ps), with no cut of the
    /// frequencies (±20 signal linewidths), against the line. The search ends at the shortest
    /// pulse, the broad pump's purity, so this reading doesn't give the line.
    #[test]
    #[ignore = "Vernon et al.'s Fig. 3 at the best Gaussian pulse: a minute"]
    fn print_vernon_fig3_best_pulse() {
        let line = [
            (1.0, 0.9273),
            (1.5, 0.9592),
            (2.0, 0.9740),
            (3.0, 0.9868),
            (4.0, 0.9921),
        ];
        for (ratio, read) in line {
            let k = |pulse: f64, points: usize| ring_schmidt(pulse, ratio, 20.0, points);
            // the pulse (ps) on a log scale
            let g = 0.5 * (5f64.sqrt() - 1.0);
            let (mut a, mut b) = (0.05f64.ln(), 50f64.ln());
            for _ in 0..30 {
                let (c, d) = (b - g * (b - a), a + g * (b - a));
                if k(c.exp(), 401) < k(d.exp(), 401) {
                    b = d;
                } else {
                    a = c;
                }
            }
            let best = (0.5 * (a + b)).exp();
            let (fine, coarse) = (1.0 / k(best, 801), 1.0 / k(best, 401));
            eprintln!(
                "ratio {ratio}: line {read:.4}, best pulse {best:.3} ps, purity {fine:.5} (K {:.5}; grid {:.1e}, off {:+.5}); a broad pump {:.5}",
                1.0 / fine,
                (fine - coarse).abs(),
                fine - read,
                1.0 / k(0.05, 801)
            );
        }
    }

    #[test]
    #[ignore = "Paesani et al.'s source from the mode solver: slow"]
    fn print_paesani() {
        let start = std::time::Instant::now();
        let c = SPEED_OF_LIGHT * 1e-6; // µm/ps
        // their Supplementary Note 3c's pump: a Gaussian of 4.58 nm, no spectral phase
        let fwhm = 0.00458;
        for h in [0.02, 0.01, 0.005] {
            let (tm0, tm1) = (paesani_index(0, h), paesani_index(1, h));
            let dng = tm1.group_index(1.55) - tm0.group_index(1.55);
            // their Supplementary Eq. 2: τ = −(L/2)(1/v₀ − 1/v₁) = L Δn_g / (2c)
            let ours = 11_000.0 * dng / (2.0 * c);
            eprintln!(
                "{h} um cells: {} and {} points, estimates {:.1e} {:.1e}; TM1 - TM0 group \
                 index at 1.55 um {dng:.5}, delay by Supp. Eq. 2 {ours:.4} ps ({:.1} s)",
                tm0.points(),
                tm1.points(),
                tm0.estimate(),
                tm1.estimate(),
                start.elapsed().as_secs_f64()
            );
            for lam in [1.516, 1.55, 1.588] {
                eprintln!(
                    "  {lam}: TM0 n {:.5} ng {:.5}; TM1 n {:.5} ng {:.5}",
                    tm0.n_eff(lam),
                    tm0.group_index(lam),
                    tm1.n_eff(lam),
                    tm1.group_index(lam)
                );
            }
            let purity =
                |delay: f64, points: usize| paesani_purity(&tm0, &tm1, delay, fwhm, points);
            for delay in [0.0, 1.46, ours, 2.06] {
                let (p, s, i) = purity(delay, 81);
                eprintln!(
                    "  delay {delay:.4} ps: purity {p:.6} (161 points: {:.6}), signal {s:.4} \
                     um, idler {i:.4} um",
                    purity(delay, 161).0
                );
            }
            // the best delay, by golden-section search
            let g = 0.5 * (5f64.sqrt() - 1.0);
            let (mut a, mut b) = (1.7, 2.3);
            for _ in 0..30 {
                let (x1, x2) = (b - g * (b - a), a + g * (b - a));
                if purity(x1, 81).0 > purity(x2, 81).0 {
                    b = x2;
                } else {
                    a = x1;
                }
            }
            let best = 0.5 * (a + b);
            eprintln!(
                "  best delay {best:.4} ps: purity {:.6} ({:.1} s)",
                purity(best, 81).0,
                start.elapsed().as_secs_f64()
            );
        }
    }

    #[test]
    #[ignore = "Paesani et al.'s source with their measured pump: slow"]
    fn print_paesani_measured_pump() {
        let start = std::time::Instant::now();
        let c = SPEED_OF_LIGHT * 1e-6; // µm/ps
        let cells: Vec<f64> = std::env::var("PAESANI_CELLS").map_or_else(
            |_| vec![0.02, 0.01, 0.005],
            |v| v.split(',').map(|s| s.parse().expect(OK)).collect(),
        );
        // their Supp. Eq. 3 has e^{−iω_pτ} e^{−iΔkz} and α = A e^{iφ}: in ours, A e^{−iφ}
        let (flat, theirs, flipped) = (
            paesani_pump(0.0, 0.5),
            paesani_pump(-1.0, 0.5),
            paesani_pump(1.0, 0.5),
        );
        // the plotted intensity taken as the field's amplitude A instead of |A|²
        let (flat_a, theirs_a) = (paesani_pump(0.0, 1.0), paesani_pump(-1.0, 1.0));
        for h in cells {
            let (tm0, tm1) = (paesani_index(0, h), paesani_index(1, h));
            let dng = tm1.group_index(1.55) - tm0.group_index(1.55);
            let eq2 = 11_000.0 * dng / (2.0 * c);
            eprintln!(
                "{h} um cells: delay by Supp. Eq. 2 {eq2:.4} ps ({:.1} s)",
                start.elapsed().as_secs_f64()
            );
            for delay in [eq2, 2.06, 0.0] {
                for points in [81, 161] {
                    let p = |a: &(dyn Fn(f64) -> c64 + Sync)| {
                        paesani_shaped_purity(&tm0, &tm1, delay, 0.00458, points, Some(a)).0
                    };
                    eprintln!(
                        "  delay {delay:.4} ps, {points} points: Gaussian {:.6}, measured without \
                         phase {:.6}, with e^-i phi {:.6}, with e^+i phi {:.6} ({:.1} s)",
                        paesani_purity(&tm0, &tm1, delay, 0.00458, points).0,
                        p(&flat),
                        p(&theirs),
                        p(&flipped),
                        start.elapsed().as_secs_f64()
                    );
                    eprintln!(
                        "    the intensity as A: without phase {:.6}, with e^-i phi {:.6}",
                        p(&flat_a),
                        p(&theirs_a)
                    );
                }
            }
            for reading in [Reading::Jacobian, Reading::Floor, Reading::Amplitude] {
                eprintln!(
                    "  delay {eq2:.4} ps, 161 points, {reading:?} reading, without phase {:.6}",
                    paesani_shaped_purity(
                        &tm0,
                        &tm1,
                        eq2,
                        0.00458,
                        161,
                        Some(&paesani_pump_read(reading))
                    )
                    .0
                );
            }
        }
    }
}
