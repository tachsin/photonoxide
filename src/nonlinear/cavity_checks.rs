//! The validation cases of resonant processes: SHG and four-wave mixing enhanced in a ring
//! ([`super::cavity`]) against the ring run round trip by round trip, the heralding efficiency
//! of photon pairs through filters ([`super::pairs`]) against Gaussian integrals, and optical
//! parametric oscillators ([`super::opo`]) against Boyd's threshold and the photons' balance.

use num_complex::Complex64 as c64;

use crate::circuit::components::{AllPassRing, Dispersion};
use crate::nonlinear::cavity::{RingWave, four_wave, second_harmonic};
use crate::nonlinear::coupled::{SecondHarmonic, ThreeWave};
use crate::nonlinear::fwm::{FourWave, Waves};
use crate::nonlinear::opo::{Feedback, Oscillator, uniform_depletion};
use crate::nonlinear::pairs::{JointSpectrum, gaussian_heralding, gaussian_purity};
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

fn relative(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance: tolerance * expected.abs(),
        error: (measured - expected).abs(),
    }
}

fn absolute(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

// ---------------------------------------------------------------------------------------------
// Rings
// ---------------------------------------------------------------------------------------------

/// An all-pass ring of 100 µm round trip and power coupling `coupling` on a guide of n_eff
/// `effective` and n_g `group` at `near` µm, losing 3 dB/cm.
fn ring(near: f64, effective: f64, group: f64) -> crate::Result<AllPassRing> {
    AllPassRing::new(Dispersion::new(Wavelength::um(near)?, effective, group).with_loss(3.0))
}

/// A wave's round trip in the iteration: the phase at the coupler, then the coupler.
fn couple(wave: &RingWave, coupling: f64, inside: c64, bus: c64) -> (c64, c64) {
    let r = (1.0 - coupling).sqrt();
    let k = c64::new(0.0, coupling.sqrt());
    let back = inside * c64::from_polar(1.0, wave.detuning());
    (k * bus + r * back, r * bus + k * back)
}

/// SHG in a doubly resonant ring: the fundamental near 1.55 µm (coupling 0.05), its harmonic in
/// a ring of n_eff 2.25 and n_g 2.4 at the fundamental's half wavelength (coupling 0.08, off its
/// own resonance by what the indices give), κ = 10⁻⁴ /(√W·µm), Δk = 0.002 rad/µm, 1 nW in:
/// (the harmonic out by running the round trips with the depleted coupled equations, `steps`
/// RK4 steps a pass, until it changes by less than 10⁻¹⁴; the closed form).
pub(crate) fn ring_shg(steps: usize) -> (f64, f64) {
    let run = || -> crate::Result<(f64, f64)> {
        let fundamental = ring(1.55, 2.2, 2.3)?;
        let at = fundamental.resonance(Wavelength::um(1.55)?, 100.0)?;
        let f = fundamental.ring_wave(at, &[100.0, 0.05])?;
        // the harmonic of the fundamental's resonance, wherever its own ring puts it
        let h =
            ring(0.775, 2.25, 2.4)?.ring_wave(Wavelength::um(at.to_um() / 2.0)?, &[100.0, 0.08])?;
        let (kappa, dk, p) = (1e-4, 0.002, 1e-9);
        let closed = second_harmonic(&f, &h, kappa, dk, p)?;
        let shg = SecondHarmonic {
            coupling: kappa,
            mismatch: dk,
            loss: [f.loss(), h.loss()],
        };
        let bus = c64::new(p.sqrt(), 0.0);
        let zero = c64::new(0.0, 0.0);
        let (mut a1, mut a2) = (zero, zero);
        let mut last = f64::NAN;
        let mut out = f64::NAN;
        for _ in 0..200_000 {
            let [e1, e2] = shg.propagate([a1, a2], f.length, steps)?;
            let (n1, _) = couple(&f, 0.05, e1, bus);
            let (n2, o2) = couple(&h, 0.08, e2, zero);
            a1 = n1;
            a2 = n2;
            out = o2.norm_sqr();
            if ((out - last) / out).abs() < 1e-14 {
                break;
            }
            last = out;
        }
        Ok((out, closed))
    };
    run().unwrap_or((f64::NAN, f64::NAN))
}

fn ring_shg_case() -> Outcome {
    let (ours, closed) = ring_shg(RING_STEPS);
    relative(ours, closed, RING_SHG_TOLERANCE)
}

const RING_STEPS: usize = 20;
const RING_SHG_TOLERANCE: f64 = 2e-10;

/// Four-wave mixing in one ring resonant at the pump (near 1.55 µm) and the resonances either
/// side of it, the signal's and the idler's: γ = 100 /(W·m), Δk = 0 (the azimuthal orders
/// matched), 1 nW of pump and of signal in: (the idler out over the signal in by running the
/// round trips with the depleted coupled equations and the losses, `steps` RK4 steps a pass;
/// the closed form).
pub(crate) fn ring_fwm(steps: usize) -> (f64, f64) {
    let run = || -> crate::Result<(f64, f64)> {
        let ring = ring(1.55, 2.2, 2.3)?;
        let values = [100.0, 0.05];
        let lp = ring.resonance(Wavelength::um(1.55)?, values[0])?;
        let fsr = ring.fsr(lp, values[0])?;
        let ls = ring.resonance(Wavelength::um(lp.to_um() - fsr)?, values[0])?;
        let li = ring.resonance(Wavelength::um(lp.to_um() + fsr)?, values[0])?;
        let [wp, ws, wi] = [lp, ls, li].map(|l| ring.ring_wave(l, &values));
        let (wp, ws, wi) = (wp?, ws?, wi?);
        let (gamma, p) = (100.0, 1e-9);
        let closed = four_wave(&wp, &ws, &wi, gamma, 0.0, p)?;
        let fw = FourWave {
            gamma,
            mismatch: 0.0,
        };
        let loss = [wp.loss(), ws.loss(), wi.loss()].map(|a| a * 1e6);
        let bus = c64::new(p.sqrt(), 0.0);
        let zero = c64::new(0.0, 0.0);
        let mut w = Waves {
            pump: zero,
            signal: zero,
            idler: zero,
        };
        let (mut last, mut out) = (f64::NAN, f64::NAN);
        for _ in 0..200_000 {
            let e = fw.propagate_with_loss(w, wp.length * 1e-6, steps, loss)?;
            let (np, _) = couple(&wp, values[1], e.pump, bus);
            let (ns, _) = couple(&ws, values[1], e.signal, bus);
            let (ni, oi) = couple(&wi, values[1], e.idler, zero);
            w = Waves {
                pump: np,
                signal: ns,
                idler: ni,
            };
            out = oi.norm_sqr() / p;
            if ((out - last) / out).abs() < 1e-14 {
                break;
            }
            last = out;
        }
        Ok((out, closed))
    };
    run().unwrap_or((f64::NAN, f64::NAN))
}

fn ring_fwm_case() -> Outcome {
    let (ours, closed) = ring_fwm(RING_STEPS);
    relative(ours, closed, RING_FWM_TOLERANCE)
}

const RING_FWM_TOLERANCE: f64 = 1e-12;

// ---------------------------------------------------------------------------------------------
// Heralding
// ---------------------------------------------------------------------------------------------

/// The heralding cases' amplitude: |f|² = exp(−zᵀMz) with M = [[1, −0.9], [−0.9, 1]] × 10⁶ µm²
/// (detunings in 1/µm, a pair 1 × 10⁻³ /µm wide in each photon and anticorrelated), on `n` × `n`
/// points 10⁻² /µm either side of 0.65 and 0.64 /µm; Gaussian filters
/// |t|² = exp(−ν²/w²), w_s = 0.8 × 10⁻³ and w_i = 1.2 × 10⁻³ /µm, centred on the photons'.
const HERALD_M: [f64; 3] = [1e6, 1e6, -0.9e6];
const HERALD_WIDTHS: [f64; 2] = [0.8e-3, 1.2e-3];

fn herald_spectrum(n: usize) -> (JointSpectrum, Vec<c64>, Vec<c64>) {
    let half = 1e-2;
    let grid = |c: f64| -> Vec<f64> {
        (0..n)
            .map(|k| c - half + 2.0 * half * k as f64 / (n - 1) as f64)
            .collect()
    };
    let (signal, idler) = (grid(0.65), grid(0.64));
    let [a, b, c] = HERALD_M;
    let mut amplitude = Vec::with_capacity(n * n);
    for s in &signal {
        for i in &idler {
            let (x, y) = (s - 0.65, i - 0.64);
            amplitude.push(c64::new(
                (-0.5 * (a * x * x + b * y * y + 2.0 * c * x * y)).exp(),
                0.0,
            ));
        }
    }
    let filter = |grid: &[f64], centre: f64, w: f64| -> Vec<c64> {
        grid.iter()
            .map(|nu| c64::new((-0.5 * ((nu - centre) / w).powi(2)).exp(), 0.0))
            .collect()
    };
    let ts = filter(&signal, 0.65, HERALD_WIDTHS[0]);
    let ti = filter(&idler, 0.64, HERALD_WIDTHS[1]);
    (
        JointSpectrum {
            signal,
            idler,
            amplitude,
        },
        ts,
        ti,
    )
}

/// (ours, closed) for the signal's heralding efficiency on `n` points a side.
pub(crate) fn heralding(n: usize) -> (f64, f64) {
    let (js, ts, ti) = herald_spectrum(n);
    let [a, b, c] = HERALD_M;
    (
        js.heralding(&ts, &ti).map_or(f64::NAN, |h| h.signal),
        gaussian_heralding(a, b, c, HERALD_WIDTHS).signal,
    )
}

/// (ours, closed) for the filtered pairs' purity on `n` points a side.
pub(crate) fn filtered_purity(n: usize) -> (f64, f64) {
    let (js, ts, ti) = herald_spectrum(n);
    let [a, b, c] = HERALD_M;
    let [ws, wi] = HERALD_WIDTHS;
    (
        js.filtered(&ts, &ti)
            .and_then(|f| f.schmidt())
            .map_or(f64::NAN, |s| s.purity()),
        gaussian_purity(a + 1.0 / (ws * ws), b + 1.0 / (wi * wi), c),
    )
}

fn heralding_case() -> Outcome {
    let (ours, closed) = heralding(201);
    relative(ours, closed, HERALDING_TOLERANCE)
}

fn filtered_purity_case() -> Outcome {
    let (ours, closed) = filtered_purity(201);
    relative(ours, closed, FILTERED_PURITY_TOLERANCE)
}

const HERALDING_TOLERANCE: f64 = 1e-12;
const FILTERED_PURITY_TOLERANCE: f64 = 1e-11;

// ---------------------------------------------------------------------------------------------
// Parametric oscillators
// ---------------------------------------------------------------------------------------------

/// The oscillators' guide: a 1.5 µm signal and a 3.0 µm idler from a 1.0 µm pump,
/// κ = 10⁻⁴ /(√W·µm), phase matched and lossless, 1 cm.
fn opo_process() -> ThreeWave {
    ThreeWave {
        frequencies: [1.0 / 1.5, 1.0 / 3.0],
        coupling: 1e-4,
        mismatch: 0.0,
        loss: [0.0; 3],
    }
}

const OPO_LENGTH: f64 = 1e4;

/// g²/P = κ²ν₁ν₂/ν₃², 1/(W·µm²).
fn gain_per_watt() -> f64 {
    let [n1, n2] = opo_process().frequencies;
    let n3 = n1 + n2;
    1e-8 * n1 * n2 / (n3 * n3)
}

/// The threshold of a doubly resonant oscillator (√R₁ = 0.95, √R₂ = 0.9) or, with `singly`, of
/// a singly resonant one (√R₁ = 0.9), on `steps` RK4 steps: (ours, Boyd's Eq. (2.9.8)), W.
pub(crate) fn opo_threshold(singly: bool, steps: usize) -> (f64, f64) {
    let (r1, r2): (f64, f64) = if singly { (0.9, 0.0) } else { (0.95, 0.9) };
    let osc = Oscillator {
        process: opo_process(),
        length: OPO_LENGTH,
        signal: Feedback::resonant(r1 * r1),
        idler: (!singly).then(|| Feedback::resonant(r2 * r2)),
    };
    let (l1, l2) = (1.0 - r1, 1.0 - r2);
    let gl = (1.0 + l1 * l2 / (2.0 - l1 - l2)).acosh();
    (
        osc.threshold(steps).unwrap_or(f64::NAN),
        (gl / OPO_LENGTH).powi(2) / gain_per_watt(),
    )
}

const OPO_STEPS: usize = 200;

fn opo_doubly_case() -> Outcome {
    let (ours, boyd) = opo_threshold(false, OPO_STEPS);
    relative(ours, boyd, OPO_DOUBLY_TOLERANCE)
}

fn opo_singly_case() -> Outcome {
    let (ours, boyd) = opo_threshold(true, OPO_STEPS);
    relative(ours, boyd, OPO_SINGLY_TOLERANCE)
}

const OPO_DOUBLY_TOLERANCE: f64 = 1e-13;
const OPO_SINGLY_TOLERANCE: f64 = 8e-12;

/// A singly resonant oscillator, R₁ = 1 − `loss`, pumped at `times` its threshold, settled on
/// `steps` RK4 steps a pass to 10⁻¹³: its steady state and threshold.
fn settled(
    loss: f64,
    times: f64,
    steps: usize,
) -> crate::Result<(crate::nonlinear::opo::SteadyState, f64)> {
    let osc = Oscillator {
        process: opo_process(),
        length: OPO_LENGTH,
        signal: Feedback::resonant(1.0 - loss),
        idler: None,
    };
    let p = osc.threshold(steps)?;
    Ok((osc.steady_state(times * p, steps, 1e-13, 2_000_000)?, p))
}

/// The steady state's photon balance at twice threshold, R₁ = 0.9: the signal's photons out per
/// round trip, (1 − R₁)P₁(L)/ν₁, against the pump's lost, (P₃(0) − P₃(L))/ν₃, relative.
pub(crate) fn opo_photons(steps: usize) -> f64 {
    let run = || -> crate::Result<f64> {
        let (s, p) = settled(0.1, 2.0, steps)?;
        let [n1, n2] = opo_process().frequencies;
        let n3 = n1 + n2;
        let signal = s.signal / n1;
        let pump = (2.0 * p - s.pump) / n3;
        Ok((signal / pump - 1.0).abs())
    };
    run().unwrap_or(f64::NAN)
}

fn opo_photons_case() -> Outcome {
    absolute(opo_photons(OPO_STEPS), 0.0, OPO_PHOTON_TOLERANCE)
}

const OPO_PHOTON_TOLERANCE: f64 = 9e-10;

/// The pump's depletion at twice threshold of a singly resonant oscillator losing `loss` of the
/// signal per round trip, against the uniform-signal limit's sin²x, x²/sin²x = 2: the gap.
pub(crate) fn opo_uniform_gap(loss: f64) -> f64 {
    settled(loss, 2.0, OPO_STEPS).map_or(f64::NAN, |(s, p)| {
        (1.0 - s.pump / (2.0 * p) - uniform_depletion(2.0)).abs()
    })
}

fn opo_uniform_order() -> Outcome {
    let order = (opo_uniform_gap(0.02) / opo_uniform_gap(0.01)).log2();
    absolute((order * 100.0).round() / 100.0, 1.0, 0.1)
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "nonlinear/ring-shg-enhancement",
            title: r"Second-harmonic generation in a doubly resonant all-pass ring (100 µm, 3 dB/cm; the fundamental's ring component on resonance near 1.55 µm, coupling 0.05; the harmonic's, n_eff 2.25 and n_g 2.4, coupling 0.08, off resonance by what its indices give), $\kappa = 10^{-4}$ /(√W·µm), $\Delta k = 0.002$ rad/µm, 1 nW in: the harmonic out from the round trips run with the depleted coupled equations (20 RK4 steps a pass) against the closed form (W shown)",
            tier: Tier::Analytic,
            source: r"the undepleted pass $G = -i\kappa A_1(0)^2\int_0^L e^{-\alpha_1 z - i\Delta k z}e^{-\alpha_2(L-z)/2}dz$ built up over the round trips: $P_{2\omega} = \lvert F_{2\omega}\rvert^2\lvert G\rvert^2$, $A_1(0) = F_\omega\sqrt{P}$, $F = ik/(1 - ra e^{i\phi})$ (L. G. Helt et al., Opt. Lett. 35, 3006 (2010), doi:10.1364/OL.35.003006, after Eq. (5))",
            run: ring_shg_case,
        },
        Case {
            id: "nonlinear/ring-fwm-enhancement",
            title: r"Four-wave mixing in one all-pass ring (100 µm, coupling 0.05, 3 dB/cm) resonant at the pump near 1.55 µm and the resonances either side, $\gamma = 100$ /(W·m), 1 nW of pump and of signal in: the conversion efficiency from the round trips run with the depleted coupled equations and the losses (20 RK4 steps a pass) against the closed form (shown)",
            tier: Tier::Analytic,
            source: r"Helt et al. (2010), doi:10.1364/OL.35.003006, Eq. (5), $P_i/P_s = (\gamma P_pL)^2\lvert F_s\rvert^2\lvert F_i\rvert^2\lvert F_p\rvert^4$, with the loss along the pass: $L \to \lvert e^{-\alpha_iL/2}(e^{xL} - 1)/x\rvert$, $x = -(2\alpha_p + \alpha_s - \alpha_i)/2$",
            run: ring_fwm_case,
        },
        Case {
            id: "nonlinear/pairs-heralding-gaussian",
            title: r"The heralding efficiency of photon pairs through filters: a Gaussian joint spectrum $\lvert f\rvert^2 = e^{-z^\mathsf{T}Mz}$, $M = [[1, -0.9], [-0.9, 1]] \times 10^6$ µm², through Gaussian filters 0.8 and 1.2 × 10⁻³ /µm wide, on 201 × 201 points: the signal's, heralded by the idler (shown)",
            tier: Tier::Analytic,
            source: r"each sum a Gaussian integral, $\pi/\sqrt{\det}$: $\eta_s = (\det(M + D_i)/\det(M + D_s + D_i))^{1/2}$, $D_s = \operatorname{diag}(1/w_s^2, 0)$, $D_i = \operatorname{diag}(0, 1/w_i^2)$",
            run: heralding_case,
        },
        Case {
            id: "nonlinear/pairs-filtered-purity",
            title: r"The same pairs' purity after the filters, by the Schmidt decomposition of the filtered amplitude (shown)",
            tier: Tier::Analytic,
            source: r"the filtered amplitude is the Gaussian $e^{-\frac{1}{2}z^\mathsf{T}(M + D)z}$, $D = \operatorname{diag}(1/w_s^2, 1/w_i^2)$: $P = (1 - c^2/(a'b'))^{1/2}$ with $a' = a + 1/w_s^2$, $b' = b + 1/w_i^2$",
            run: filtered_purity_case,
        },
        Case {
            id: "nonlinear/opo-threshold-doubly",
            title: r"A doubly resonant parametric oscillator (1.5 and 3.0 µm from 1.0 µm, $\kappa = 10^{-4}$ /(√W·µm), 1 cm, matched and lossless; $\sqrt{R_1} = 0.95$, $\sqrt{R_2} = 0.9$): the pump power where the undepleted round trip's spectral radius reaches 1 (200 RK4 steps; W shown)",
            tier: Tier::Analytic,
            source: r"R. W. Boyd, Nonlinear Optics, 3rd ed. (Academic Press, 2008), Eq. (2.9.8): $\cosh gL = 1 + l_1l_2/(2 - l_1 - l_2)$, $l_j = 1 - \sqrt{R_j}$, with $g^2 = \kappa^2\nu_1\nu_2P/\nu_3^2$",
            run: opo_doubly_case,
        },
        Case {
            id: "nonlinear/opo-threshold-singly",
            title: r"The same guide singly resonant, $\sqrt{R_1} = 0.9$ and the idler not fed back: its threshold (W shown)",
            tier: Tier::Analytic,
            source: r"Boyd (2008), Eq. (2.9.8) with $l_2 = 1$: $\cosh gL = 1/\sqrt{R_1}$",
            run: opo_singly_case,
        },
        Case {
            id: "nonlinear/opo-steady-photons",
            title: r"The singly resonant oscillator ($R_1 = 0.9$) settled at twice its threshold by running the round trips with the depleted coupled equations (200 RK4 steps a pass): the signal's photons out per round trip against the pump's photons lost, their relative difference (shown)",
            tier: Tier::Analytic,
            source: r"in the steady state the signal's photons gained per pass leave through the coupler, and by the Manley–Rowe relations (J. A. Armstrong et al., Phys. Rev. 127, 1918 (1962), doi:10.1103/PhysRev.127.1918, Eq. (6.6)) each one is a pump photon lost",
            run: opo_photons_case,
        },
        Case {
            id: "nonlinear/opo-uniform-limit-order",
            title: r"The singly resonant oscillator's pump depletion at twice threshold approaches the uniform-signal limit $\sin^2x$, $x^2/\sin^2x = 2$, as the signal's loss per round trip falls: the order of the gap in that loss, from 2 % and 1 % (order shown)",
            tier: Tier::Analytic,
            source: r"the coupled equations with the signal held along the guide (the limit of no loss per pass): the pump $\propto\cos gz$, the signal's gain per pass $\propto\sin^2 gL/g^2$; the held signal's error is first order in the loss",
            run: opo_uniform_order,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The convergence the rules quote: run with `--ignored --nocapture` in release.
    #[test]
    #[ignore = "prints the convergence the rules quote"]
    fn convergence() {
        for s in [5, 10, 20, 40] {
            let (o, c) = ring_shg(s);
            let (fo, fc) = ring_fwm(s);
            eprintln!(
                "ring {s}: shg {:.2e} ({o:e} {c:e}) fwm {:.2e} ({fo:e} {fc:e})",
                (o / c - 1.0).abs(),
                (fo / fc - 1.0).abs()
            );
        }
        for n in [101, 151, 201, 301] {
            let (h, hc) = heralding(n);
            let (p, pc) = filtered_purity(n);
            eprintln!(
                "herald {n}: {:.2e} ({h}) purity {:.2e} ({p})",
                (h / hc - 1.0).abs(),
                (p / pc - 1.0).abs()
            );
        }
        for s in [50, 100, 200, 400] {
            let (d, db) = opo_threshold(false, s);
            let (si, sb) = opo_threshold(true, s);
            eprintln!(
                "opo {s}: doubly {:.2e} ({d}) singly {:.2e} ({si})",
                (d / db - 1.0).abs(),
                (si / sb - 1.0).abs()
            );
        }
        for s in [100, 200, 400] {
            eprintln!("photons {s}: {:.2e}", opo_photons(s));
        }
        for l in [0.04, 0.02, 0.01, 0.005] {
            eprintln!("uniform gap {l}: {:.3e}", opo_uniform_gap(l));
        }
    }
}
