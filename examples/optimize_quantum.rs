//! Quantum objectives optimized on their own gradients (0.7, #761's I10; quantum.md's Q4): four
//! optima whose answers are known, each recovered by genoxide's L-BFGS-B, or by Newton's method,
//! on photonoxide's exact gradients.
//!
//! - **A Hong–Ou–Mandel coupler.** Two photons on a directional coupler of power coupling R: the
//!   visibility V = 1 − (1 − 2R)²/(R² + (1 − R)²) of their coincidence dip is 1 at R = ½ (Hong,
//!   Ou & Mandel 1987). The coupler is a netlist; V's sensitivity to its S-matrix
//!   (`quantum::visibility_gradient`) goes through the circuit adjoint to dV/dR. From R = 0.15.
//! - **A Clements mesh programmed to a Haar-random unitary** (Clements et al. 2016): a 4-mode
//!   mesh of MZIs (ideal 50:50 couplers and phase shifters) as a netlist, its 16 phases moved to
//!   maximize Clements et al.'s fidelity (their Eq. 6, `quantum::fidelity_gradient`) through the
//!   circuit adjoint, from random phases: F = 1, as their decomposition proves possible.
//! - **A ring's purity with equal linewidths** (Vernon et al. 2017): Helt et al.'s ring (1.48 GHz
//!   resonances) pumped by a transform-limited Gaussian pulse, the pulse's bandwidth moved to
//!   maximize the heralded photon's purity γ (`quantum::purity_gradient` on the joint amplitude,
//!   its derivative in the bandwidth from `quantum::ring_spectrum_derivative`). The purity rises
//!   with the bandwidth to the broad-pump limit, so L-BFGS-B goes to its bound, 400 linewidths:
//!   K = 1/γ there against their Fig. 2(a)'s K = 1.09 (their text's bound reads 1.08), checked
//!   at its printed digits, the joint spectrum on 81 and 121 points beside.
//! - **Vernon et al.'s dual coupling.** A second channel at the pump widens its resonance
//!   alone; with a pump broad enough to fill it, Newton's method on the extra round-trip loss
//!   finds where γ reaches 1/1.0009 (their Fig. 2(b), K = 1.0009, at a pump linewidth 6.6 times
//!   the signal's). Ours needs a wider pump resonance, and more so as the window grows (the
//!   Lorentzians' tails): 9.6, 11.7 and 12.8 times across ±10, 20 and 40 linewidths, about 14
//!   extrapolated; at their 6.6, K = 1.003. Not reproduced: printed, not checked, and kept open
//!   in #595 (their Fig. 2(b) is from their full device's simulation, Fig. 3's markers, whose
//!   model the paper doesn't give).

mod common;

use std::f64::consts::{LN_2, PI, TAU};
use std::process::ExitCode;
use std::sync::Arc;

use genoxide::prelude::*;
use photonoxide::Complex64 as c64;
use photonoxide::circuit::components::{AllPassRing, Coupler, Dispersion, PhaseShifter};
use photonoxide::circuit::{Circuit, Component, Netlist, SMatrix};
use photonoxide::nonlinear::pairs::{JointSpectrum, Pump};
use photonoxide::quantum::{
    Matrix, RingGrid, fidelity_gradient, haar_unitary, purity_gradient, ring_spectrum_derivative,
    visibility_gradient,
};
use photonoxide::units::{SPEED_OF_LIGHT, Wavelength};

const OK: &str = "a valid example";

fn lambda() -> Wavelength {
    Wavelength::um(1.55).expect(OK)
}

/// A circuit's transfer block, rows `outputs` and columns `inputs` of its S-matrix.
fn block(s: &SMatrix, outputs: &[usize], inputs: &[usize]) -> Matrix {
    Matrix::from_fn(outputs.len(), inputs.len(), |q, p| {
        s[(outputs[q], inputs[p])]
    })
}

/// A block's sensitivity `g` placed back in the S-matrix, scaled by `sign`.
fn embed(size: usize, g: &Matrix, outputs: &[usize], inputs: &[usize], sign: f64) -> SMatrix {
    let mut whole = SMatrix::zeros(size);
    for (q, &o) in outputs.iter().enumerate() {
        for (p, &i) in inputs.iter().enumerate() {
            whole[(o, i)] = g[(q, p)] * sign;
        }
    }
    whole
}

fn port(circuit: &Circuit, name: &str) -> usize {
    circuit
        .ports()
        .iter()
        .position(|p| p.name == name)
        .expect("a port")
}

/// L-BFGS-B on `fitness` over `bounds` from `start`, to `evaluations` at most.
fn lbfgsb(
    bounds: Vec<std::ops::RangeInclusive<f64>>,
    start: Vec<f64>,
    evaluations: u64,
    fitness: &(dyn Fn(&Reals, &mut [f64]) -> f64 + Sync),
) -> (Vec<f64>, u64) {
    let real = Real::new(bounds).expect("bounds");
    let run = Lbfgsb::builder(real)
        .initial_genome(Reals::from(start))
        .gradient_tolerance(0.0)
        .function_tolerance(0.0)
        .minimize()
        .build()
        .expect("a valid L-BFGS-B");
    let outcome = Engine::new(run, Differentiable(fitness))
        .stop_when(Stop::evaluations(evaluations))
        .run()
        .expect("a run");
    (outcome.best_genome().to_vec(), outcome.evaluations())
}

/// The HOM coupler: (R found, V there, evaluations).
fn hom_coupler() -> photonoxide::Result<(f64, f64, u64)> {
    let mut net = Netlist::new();
    net.add("c", Arc::new(Coupler::new()))?;
    for (name, p) in [
        ("in0", "c.o1"),
        ("in1", "c.o2"),
        ("out0", "c.o3"),
        ("out1", "c.o4"),
    ] {
        net.expose(name, p)?;
    }
    let circuit = net.compile()?;
    let slot = circuit.parameter("c.coupling").expect("the coupling");
    let base = circuit.values().to_vec();
    let inputs = [port(&circuit, "in0"), port(&circuit, "in1")];
    let outputs = [port(&circuit, "out0"), port(&circuit, "out1")];
    let size = circuit.ports().len();
    let fitness = |x: &Reals, gradient: &mut [f64]| -> f64 {
        let mut values = base.clone();
        values[slot] = x[0];
        let mut v = 0.0;
        let (_, g) = circuit
            .gradient(&[lambda()], &values, |s| {
                let t = block(&s[0], &outputs, &inputs);
                let (value, sensitivity) = visibility_gradient(&t, [0, 1], [0, 1]).expect(OK);
                v = value;
                (
                    -value,
                    vec![embed(size, &sensitivity, &outputs, &inputs, -1.0)],
                )
            })
            .expect(OK);
        gradient[0] = g[slot];
        -v
    };
    let (best, evaluations) = lbfgsb(vec![0.01..=0.99], vec![0.15], 40, &fitness);
    let v = -fitness(&Reals::from(best.clone()), &mut [0.0]);
    Ok((best[0], v, evaluations))
}

/// A 4-mode Clements mesh of ideal MZIs as a netlist (as `optimize_mesh`'s), its phases'
/// parameter slots, and its ports.
struct Mesh {
    circuit: Circuit,
    base: Vec<f64>,
    slots: Vec<usize>,
    inputs: Vec<usize>,
    outputs: Vec<usize>,
}

impl Mesh {
    fn new(n: usize) -> photonoxide::Result<Mesh> {
        // the MZIs' upper modes, layer by layer, alternating
        let tops: Vec<usize> = (0..n)
            .flat_map(|layer| (layer % 2..n - 1).step_by(2))
            .collect();
        let coupler = Arc::new(Coupler::new());
        let shifter = Arc::new(PhaseShifter::new());
        let mut net = Netlist::new();
        let mut head: Vec<Option<String>> = vec![None; n];
        let feed =
            |net: &mut Netlist, head: &mut Option<String>, mode: usize, port: &str| match head
                .take()
            {
                Some(out) => net.connect(&out, port),
                None => net.expose(&format!("in{mode}"), port),
            };
        for (m, &top) in tops.iter().enumerate() {
            let [phi, a, theta, b] = ["phi", "a", "theta", "b"].map(|s| format!("{s}{m}"));
            net.add(&phi, shifter.clone())?;
            net.add(&theta, shifter.clone())?;
            for c in [&a, &b] {
                net.add(c, coupler.clone())?;
                net.set(c, "coupling", 0.5)?;
            }
            feed(&mut net, &mut head[top], top, &format!("{phi}.o1"))?;
            feed(&mut net, &mut head[top + 1], top + 1, &format!("{a}.o1"))?;
            for (p, q) in [
                (format!("{phi}.o2"), format!("{a}.o2")),
                (format!("{a}.o3"), format!("{theta}.o1")),
                (format!("{theta}.o2"), format!("{b}.o2")),
                (format!("{a}.o4"), format!("{b}.o1")),
            ] {
                net.connect(&p, &q)?;
            }
            head[top] = Some(format!("{b}.o3"));
            head[top + 1] = Some(format!("{b}.o4"));
        }
        for (mode, h) in head.iter_mut().enumerate() {
            let d = format!("d{mode}");
            net.add(&d, shifter.clone())?;
            feed(&mut net, h, mode, &format!("{d}.o1"))?;
            net.expose(&format!("out{mode}"), &format!("{d}.o2"))?;
        }
        let circuit = net.compile()?;
        let at = |name: String| circuit.parameter(&name).expect("a parameter");
        let mut slots = Vec::new();
        for m in 0..tops.len() {
            slots.push(at(format!("theta{m}.phase")));
            slots.push(at(format!("phi{m}.phase")));
        }
        slots.extend((0..n).map(|q| at(format!("d{q}.phase"))));
        let inputs = (0..n).map(|k| port(&circuit, &format!("in{k}"))).collect();
        let outputs = (0..n).map(|k| port(&circuit, &format!("out{k}"))).collect();
        Ok(Mesh {
            base: circuit.values().to_vec(),
            circuit,
            slots,
            inputs,
            outputs,
        })
    }

    /// 1 − F and its gradient in the phases, F the fidelity to `target`.
    fn misfit(&self, target: &Matrix, x: &[f64], gradient: &mut [f64]) -> f64 {
        let mut values = self.base.clone();
        for (&s, &g) in self.slots.iter().zip(x) {
            values[s] = g - TAU * ((g + PI) / TAU).floor();
        }
        let size = self.circuit.ports().len();
        let mut f = 0.0;
        let (_, g) = self
            .circuit
            .gradient(&[lambda()], &values, |s| {
                let u = block(&s[0], &self.outputs, &self.inputs);
                let (value, sensitivity) = fidelity_gradient(target, &u);
                f = value;
                (
                    1.0 - value,
                    vec![embed(size, &sensitivity, &self.outputs, &self.inputs, -1.0)],
                )
            })
            .expect(OK);
        for (out, &s) in gradient.iter_mut().zip(&self.slots) {
            *out = g[s];
        }
        1.0 - f
    }
}

/// Helt et al.'s ring (R = 47.5 µm, v = 171 µm/ps, order 310 at 1.55 µm), coupled for
/// resonances 1.48 GHz wide: the component, its length, its coupling and its pump, signal and
/// idler resonances (1/µm).
struct Ring {
    ring: AllPassRing,
    length: f64,
    coupling: f64,
    resonances: [f64; 3],
}

fn per_um(hz: f64) -> f64 {
    hz / (SPEED_OF_LIGHT * 1e6)
}

impl Ring {
    fn new() -> Ring {
        let ng = SPEED_OF_LIGHT * 1e-6 / 171.0;
        let length = TAU * 47.5;
        let ring =
            AllPassRing::new(Dispersion::new(lambda(), 310.0 * 1.55 / length, ng)).expect(OK);
        let fsr = 1.0 / (ng * length);
        let resonance = |nu: f64| {
            1.0 / ring
                .resonance(Wavelength::um(1.0 / nu).expect(OK), length)
                .expect(OK)
                .to_um()
        };
        let pump = resonance(1.0 / 1.55);
        let resonances = [pump, resonance(pump + fsr), resonance(pump - fsr)];
        let lam = 1.0 / pump;
        let width = |k2: f64| {
            ring.fwhm(Wavelength::um(lam).expect(OK), &[length, k2])
                .expect(OK)
                / (lam * lam)
        };
        let target = per_um(1.48e9);
        let (mut lo, mut hi): (f64, f64) = (1e-6, 0.5);
        for _ in 0..100 {
            let mid = (lo * hi).sqrt();
            if width(mid) < target {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        Ring {
            ring,
            length,
            coupling: (lo * hi).sqrt(),
            resonances,
        }
    }

    /// F(ν) of a wave whose round trip loses `extra` of its power besides the coupling, and
    /// dF/d(extra).
    fn enhancement(&self, extra: f64, nu: f64) -> (c64, c64) {
        let w = Wavelength::um(1.0 / nu).expect(OK);
        let round = self
            .ring
            .round_trip(w, &[self.length, self.coupling])
            .expect(OK);
        let (r, s) = ((1.0 - self.coupling).sqrt(), (1.0 - extra).sqrt());
        let d = 1.0 - r * round * s;
        let i = c64::new(0.0, self.coupling.sqrt());
        (i / d, -i * r * round / (2.0 * s * d * d))
    }

    /// The pump's resonance's full width at half maximum with `extra`, 1/µm, by bisection.
    fn pump_width(&self, extra: f64) -> f64 {
        let peak = self.enhancement(extra, self.resonances[0]).0.norm_sqr();
        let (mut lo, mut hi) = (0.0, per_um(200e9));
        for _ in 0..100 {
            let mid = 0.5 * (lo + hi);
            if self
                .enhancement(extra, self.resonances[0] + mid)
                .0
                .norm_sqr()
                > 0.5 * peak
            {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        2.0 * lo
    }

    /// The purity and its derivatives in the extra loss and in the pump's bandwidth (1/µm), on a
    /// grid of `points` across ±10 linewidths.
    fn purity(&self, extra: f64, bandwidth: f64, points: usize, span: f64) -> (f64, f64, f64) {
        let linewidth = per_um(1.48e9);
        let pump = Pump {
            center: self.resonances[0],
            bandwidth,
        };
        let grid = RingGrid {
            half: span * linewidth,
            points,
            pump_half: (8.0 * bandwidth).min(100.0 * linewidth),
            pump_points: 2001,
        };
        let fp = |nu: f64| Ok(self.enhancement(extra, nu).0);
        let dfp = |nu: f64| Ok(self.enhancement(extra, nu).1);
        let fs = |nu: f64| Ok(self.enhancement(0.0, nu).0);
        let zero = |_: f64| Ok(c64::new(0.0, 0.0));
        let centers = [self.resonances[1], self.resonances[2]];
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
        let (gamma, g) = purity_gradient(&matrix(&js)).expect(OK);
        (
            gamma,
            along(&g, &matrix(&by_loss)),
            along(&g, &matrix(&by_bandwidth)),
        )
    }
}

fn matrix(js: &JointSpectrum) -> Matrix {
    let ni = js.idler.len();
    Matrix::from_fn(js.signal.len(), ni, |i, j| js.amplitude[i * ni + j])
}

/// 2 Re Σ G D.
fn along(g: &Matrix, d: &Matrix) -> f64 {
    let mut s = 0.0;
    for i in 0..g.rows() {
        for j in 0..g.cols() {
            s += 2.0 * (g[(i, j)] * d[(i, j)]).re;
        }
    }
    s
}

/// The pulse length (intensity FWHM, ps) of a transform-limited Gaussian of bandwidth `b`
/// (1/µm).
fn pulse_ps(b: f64) -> f64 {
    4.0 * LN_2 / (TAU * b * SPEED_OF_LIGHT * 1e6) * 1e12
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let mut setup = common::Setup::new();
    setup.item(
        "devices",
        "an ideal coupler; a 4-mode Clements mesh of ideal MZIs; Helt et al.'s ring (R = 47.5 µm, 1.48 GHz resonances), with Vernon et al.'s second channel at the pump",
    );
    setup.item(
        "grid",
        "none for the coupler and the mesh; the ring's joint spectrum on 81 and 121 points across ±10 linewidths, 2001 pump points",
    );
    setup.item(
        "solver",
        "genoxide's L-BFGS-B on the exact gradients (the circuit adjoint, the purity's sensitivity and the spectrum's derivative); Newton's method for the dual coupling",
    );
    setup.item(
        "tolerance",
        "the coupler to 1e-6 in R, the mesh's fidelity to 1e-12, the ring's numbers at the paper's printed digits with the finer grid beside",
    );
    setup.print();
    let mut checks = common::Checks::default();

    let (r, v, evaluations) = hom_coupler()?;
    println!(
        "\nA Hong–Ou–Mandel coupler from R = 0.15: R = {r:.9}, V = {v:.12} in {evaluations} evaluations"
    );
    checks.against(
        "the coupler's R",
        r,
        "Hong, Ou & Mandel, R = 1/2",
        0.5,
        1e-6,
    );
    checks.against("its visibility", v, "1 at R = 1/2", 1.0, 1e-10);

    let mesh = Mesh::new(4)?;
    let target = haar_unitary(4, 2026);
    let start: Vec<f64> = (0..mesh.slots.len())
        .map(|k| 2.0 * PI * ((0.6180339887 * (k + 1) as f64) % 1.0) - PI)
        .collect();
    let fitness = |x: &Reals, gradient: &mut [f64]| mesh.misfit(&target, x, gradient);
    let (best, evaluations) = lbfgsb(vec![-1e3..=1e3; start.len()], start, 2000, &fitness);
    let misfit = mesh.misfit(&target, &best, &mut vec![0.0; best.len()]);
    println!(
        "A 4-mode Clements mesh, {} phases, programmed to a Haar-random unitary: 1 − F = {misfit:.1e} in {evaluations} evaluations",
        best.len()
    );
    checks.at_most("the mesh's 1 − F", misfit, 1e-12);

    let ring = Ring::new();
    let linewidth = per_um(1.48e9);
    // equal linewidths: the pump's bandwidth that maximizes the purity, in linewidths
    let best_bandwidth = |points: usize| -> (f64, f64, u64) {
        let fitness = |x: &Reals, gradient: &mut [f64]| {
            let (gamma, _, by_bandwidth) = ring.purity(0.0, x[0] * linewidth, points, 10.0);
            gradient[0] = -by_bandwidth * linewidth;
            -gamma
        };
        let (best, evaluations) = lbfgsb(vec![0.5..=400.0], vec![3.0], 60, &fitness);
        (
            best[0],
            ring.purity(0.0, best[0] * linewidth, points, 10.0).0,
            evaluations,
        )
    };
    let (b, gamma, evaluations) = best_bandwidth(81);
    let (b_fine, gamma_fine, _) = best_bandwidth(121);
    println!(
        "Equal linewidths: the purity is largest at a pump {b:.3} linewidths wide (a {:.0} ps pulse): γ = {gamma:.5}, K = {:.4}, in {evaluations} evaluations; on 121 points {b_fine:.3}, γ = {gamma_fine:.5}",
        pulse_ps(b * linewidth),
        1.0 / gamma
    );
    checks.compare(
        "the largest purity's K (Fig. 2(a))",
        1.0 / gamma,
        1.09,
        0.005,
    );

    // the dual coupling: Newton on the extra loss for γ = 1/1.0009 with a broad pump
    let broad = 400.0 * linewidth;
    let goal = 1.0 / 1.0009;
    let condition = |span: f64| -> (f64, f64, usize) {
        let points = (8.0 * span) as usize + 1;
        let mut x: f64 = 0.05;
        let mut steps = 0;
        for _ in 0..30 {
            let (gamma, by_loss, _) = ring.purity(x, broad, points, span);
            steps += 1;
            let step = (gamma - goal) / by_loss;
            x = (x - step).clamp(1e-6, 0.95);
            if step.abs() < 1e-12 {
                break;
            }
        }
        (x, ring.pump_width(x) / ring.pump_width(0.0), steps)
    };
    let mut ratios = Vec::new();
    for span in [10.0, 20.0, 40.0] {
        let (x, ratio, steps) = condition(span);
        println!(
            "The dual coupling, ±{span} linewidths: γ = 1/1.0009 at an extra round-trip loss of {x:.5}, the pump's resonance {ratio:.3} times the signal's, in {steps} Newton steps"
        );
        ratios.push(ratio);
    }
    // the window's own error falls as 1/span (the Lorentzians' tails): Richardson's first order
    let limit = 2.0 * ratios[2] - ratios[1];
    // and Vernon et al.'s 6.6 itself: the extra loss that widens the pump's resonance 6.6 times
    let (mut lo, mut hi) = (0.0, 0.9);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if ring.pump_width(mid) / ring.pump_width(0.0) < 6.6 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let (at_their_ratio, _, _) = ring.purity(0.5 * (lo + hi), broad, 321, 40.0);
    println!(
        "  extrapolated in 1/span, {limit:.2}; at their ratio of 6.6, K = {:.4} on ±40 linewidths, against their K = 1.0009 (Fig. 2(b)): not reproduced, reported (#595), not checked",
        1.0 / at_their_ratio
    );
    Ok(checks.finish())
}
