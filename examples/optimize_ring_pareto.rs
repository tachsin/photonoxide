//! An add-drop ring's Pareto front, found by genoxide's NSGA-II and SMS-EMOA: the power into its
//! drop port on resonance against the width of its resonance, over its two couplings, with its
//! loss fixed; and the front in closed form.
//!
//! W. Bogaerts et al., Laser Photonics Rev. 6, 47 (published online 13 September 2011; the
//! January 2012 issue), [doi:10.1002/lpor.201100017](https://doi.org/10.1002/lpor.201100017),
//! Eqs. 5–8: on resonance the drop port carries D = (1 − r₁²)(1 − r₂²) a / (1 − r₁r₂a)², and the
//! resonance's full width at half maximum is FWHM = (1 − r₁r₂a) λ² / (π n_g L √(r₁r₂a)), r₁ and
//! r₂ the couplers' self-couplings, a the round trip's amplitude.
//!
//! **The front, derived here.** The width depends on the product p = r₁r₂ alone, and at a fixed
//! p, (1 − r₁²)(1 − r₂²) = 1 + p² − r₁² − p²/r₁² is largest at r₁² = p (where r₁² + p²/r₁² is
//! least): r₁ = r₂ = √p. So the designs that no other beats on both, the most drop at the least
//! width, are the symmetric rings, D(p) = (1 − p)² a / (1 − pa)² against FWHM(p): narrowing the
//! resonance costs drop. The optimizers' fronts are checked against that curve by their
//! hypervolume (within 1 % of the curve's). Point by point they come within a few 1e-3 of it:
//! off the symmetric ridge the drop falls only quadratically in the asymmetry, so a population
//! spread along the front settles onto it slowly (printed). Each of their points is then
//! polished by genoxide's GCMMA, the most drop at no more than its width (an ε-constraint), on
//! the circuit adjoint's gradient, and checked within 1e-3 of the curve: the global search's front
//! and a local method's polish, as #244's plan has it (Section 1.6).
//!
//! The drop is the circuit solve's at resonance (photonoxide's circuit, Bogaerts's couplers),
//! the width Eq. 8's: exact, no grid. The hypervolume is photonoxide's
//! (`job::optimize::hypervolume`), of the costs −D and FWHM.
//!
//! ```sh
//! cargo run --release --example optimize_ring_pareto
//! ```

mod common;

use std::f64::consts::PI;
use std::process::ExitCode;
use std::sync::Arc;

use genoxide::math;
use genoxide::prelude::*;
use photonoxide::circuit::components::{Coupler, Dispersion, Waveguide};
use photonoxide::circuit::{Circuit, Netlist, objective};
use photonoxide::job::optimize::hypervolume;
use photonoxide::units::Wavelength;
use rayon::prelude::*;

/// The wire: n_eff and n_g at λ₀ (µm).
const INDEX: f64 = 2.4;
const GROUP_INDEX: f64 = 4.2;
const LAMBDA: f64 = 1.55;
/// The ring: its radius (µm) and its loss (dB/cm).
const RADIUS: f64 = 10.0;
const LOSS: f64 = 20.0;
/// The couplings' range.
const KAPPA2: (f64, f64) = (0.005, 0.3);
/// The reference point of the hypervolume, in the costs (−D, FWHM in nm).
const REFERENCE: [f64; 2] = [0.0, 3.0];

fn guide() -> photonoxide::Result<Waveguide> {
    Ok(Waveguide::new(Dispersion::new(
        Wavelength::um(LAMBDA)?,
        INDEX,
        GROUP_INDEX,
    )))
}

/// The add-drop ring of circuit_fit: couplers c1 and c2 joined by two halves; ports in,
/// through, add, drop.
fn ring() -> photonoxide::Result<Circuit> {
    let coupler = Arc::new(Coupler::new());
    let half = Arc::new(guide()?);
    let mut n = Netlist::new();
    n.add("c1", coupler.clone())?;
    n.add("c2", coupler)?;
    n.add("top", half.clone())?;
    n.add("bottom", half)?;
    for (a, b) in [
        ("c1.o3", "top.o1"),
        ("top.o2", "c2.o2"),
        ("c2.o3", "bottom.o1"),
        ("bottom.o2", "c1.o2"),
    ] {
        n.connect(a, b)?;
    }
    for (name, port) in [
        ("in", "c1.o1"),
        ("through", "c1.o4"),
        ("add", "c2.o1"),
        ("drop", "c2.o4"),
    ] {
        n.expose(name, port)?;
    }
    n.compile()
}

/// The ring's round trip, µm.
fn length() -> f64 {
    2.0 * PI * RADIUS
}

/// Its round trip's amplitude, a = 10^(−αL/20).
fn amplitude() -> f64 {
    math::exp(-LOSS * 1e-4 * length() / 20.0 * std::f64::consts::LN_10)
}

/// The resonance nearest λ₀: n_eff(λ) L = m λ with n_eff first order in λ, n_eff(λ) =
/// n₀ + (n₀ − n_g)(λ − λ₀)/λ₀.
fn resonance() -> f64 {
    let d1 = (INDEX - GROUP_INDEX) / LAMBDA;
    let l = length();
    let m = (INDEX * l / LAMBDA).round();
    l * (INDEX - d1 * LAMBDA) / (m - l * d1)
}

/// The problem: the circuit, its slots, the resonance.
struct Problem {
    circuit: Circuit,
    base: Vec<f64>,
    slots: [usize; 2],
    resonance: Wavelength,
}

impl Problem {
    fn new() -> photonoxide::Result<Problem> {
        let circuit = ring()?;
        let at = |name: &str| circuit.parameter(name).expect("a parameter");
        let mut values = circuit.values().to_vec();
        for half in ["top", "bottom"] {
            values[at(&format!("{half}.length"))] = length() / 2.0;
            values[at(&format!("{half}.loss"))] = LOSS;
        }
        let slots = [at("c1.coupling"), at("c2.coupling")];
        Ok(Problem {
            circuit,
            slots,
            resonance: Wavelength::um(resonance())?,
            base: values,
        })
    }

    /// The drop power at resonance, by the circuit solve, for couplings κ₁², κ₂².
    fn drop(&self, k: [f64; 2]) -> f64 {
        let mut v = self.base.clone();
        v[self.slots[0]] = k[0];
        v[self.slots[1]] = k[1];
        self.circuit
            .s_matrix_with(self.resonance, &v)
            .map_or(f64::NAN, |s| s.power(3, 0))
    }
}

/// Eq. 8's width, nm, for self-couplings with product p.
fn fwhm(p: f64) -> f64 {
    let pa = p * amplitude();
    (1.0 - pa) * resonance().powi(2) / (PI * GROUP_INDEX * length() * pa.sqrt()) * 1e3
}

/// The front's drop at the product p: the symmetric ring's.
fn front_drop(p: f64) -> f64 {
    let a = amplitude();
    (1.0 - p).powi(2) * a / (1.0 - p * a).powi(2)
}

/// The product p whose width is `w` nm: FWHM falls with p, found by bisection.
fn product_at(w: f64) -> f64 {
    let (mut lo, mut hi) = (1e-6, 1.0 - 1e-12);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if fwhm(mid) > w {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// The costs of couplings κ²: −D and FWHM.
fn costs(problem: &Problem, k: &[f64]) -> [f64; 2] {
    let p = ((1.0 - k[0]) * (1.0 - k[1])).sqrt();
    [-problem.drop([k[0], k[1]]), fwhm(p)]
}

/// The closed-form front's hypervolume up to `REFERENCE`, by the trapezoid rule over 20 001
/// products spanning the couplings' range.
fn front_hypervolume() -> f64 {
    let p_of = |k: f64| 1.0 - k;
    let (p_lo, p_hi) = (p_of(KAPPA2.1), p_of(KAPPA2.0));
    let points: Vec<[f64; 2]> = (0..=20_000)
        .map(|i| {
            let p = p_lo + (p_hi - p_lo) * f64::from(i) / 20_000.0;
            [-front_drop(p), fwhm(p)]
        })
        .collect();
    hypervolume(&points, REFERENCE)
}

/// A front: each point's costs and couplings.
type Front = Vec<([f64; 2], Vec<f64>)>;

/// A multi-objective method's front: each point's costs and couplings.
fn run<A>(problem: &Problem, algorithm: A, budget: u64) -> Front
where
    A: genoxide::multi::MultiObjectiveAlgorithm<2, Genome = Reals>,
{
    // a generation side by side on rayon's threads, its costs in its order
    let fitness = Batch(|ks: &[&Reals]| {
        ks.par_iter()
            .map(|k| costs(problem, k))
            .collect::<Vec<[f64; 2]>>()
    });
    let outcome = MultiEngine::new(algorithm, fitness)
        .stop_when(Stop::evaluations(budget))
        .run()
        .expect("a run");
    outcome
        .front()
        .iter()
        .map(|i| {
            let k = i.genome().to_vec();
            (costs(problem, &k), k)
        })
        .collect()
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let problem = Problem::new()?;
    let mut checks = common::Checks::default();
    let a = amplitude();
    println!(
        "An add-drop ring, R = {RADIUS} µm, {LOSS} dB/cm (a = {a:.6}), n_eff {INDEX}, n_g {GROUP_INDEX}, at its resonance {:.6} µm (Bogaerts et al. 2011, Eqs. 5–8); couplings κ² from {} to {}",
        resonance(),
        KAPPA2.0,
        KAPPA2.1
    );
    println!(
        "  the drop by the circuit solve and the width by Eq. 8, exact: no grid; costs −D and FWHM (nm)"
    );
    // the circuit's drop against Eq. 6 at a design
    let k: [f64; 2] = [0.05, 0.12];
    let (r1, r2) = ((1.0 - k[0]).sqrt(), (1.0 - k[1]).sqrt());
    let eq6 = (1.0 - r1 * r1) * (1.0 - r2 * r2) * a / (1.0 - r1 * r2 * a).powi(2);
    checks.against(
        "drop at resonance, κ² = 0.05, 0.12",
        problem.drop(k),
        "Eq. 6",
        eq6,
        1e-12,
    );
    let analytic = front_hypervolume();
    println!("  the closed-form front's hypervolume up to (0, 3 nm): {analytic:.6}");

    let n = 2;
    let space = || Real::new(vec![KAPPA2.0..=KAPPA2.1; n]).expect("bounds");
    let methods: [(&str, Front); 2] = [
        (
            "NSGA-II",
            run(
                &problem,
                Nsga2::builder(space(), [Objective::Minimize; 2])
                    .population_size(60)
                    .crossover(SimulatedBinaryCrossover::new(30.0).expect("valid"))
                    .mutate(PolynomialMutation::per_gene(0.5, 100.0).expect("valid"))
                    .seed(1)
                    .build()
                    .expect("a valid NSGA-II"),
                40_000,
            ),
        ),
        (
            "SMS-EMOA",
            run(
                &problem,
                SmsEmoa::builder(space(), [Objective::Minimize; 2])
                    .population_size(60)
                    .crossover(SimulatedBinaryCrossover::new(30.0).expect("valid"))
                    .mutate(PolynomialMutation::per_gene(0.5, 100.0).expect("valid"))
                    .seed(1)
                    .build()
                    .expect("a valid SMS-EMOA"),
                40_000,
            ),
        ),
    ];
    for (name, front) in &methods {
        let costs: Vec<[f64; 2]> = front.iter().map(|(c, _)| *c).collect();
        let hv = hypervolume(&costs, REFERENCE);
        let widths = costs.iter().map(|c| c[1]);
        let (narrowest, widest) = (
            widths.clone().fold(f64::INFINITY, f64::min),
            widths.fold(0.0, f64::max),
        );
        // the evolutionary front against the closed form at each point's width: near, but the
        // drop falls off the symmetric ridge only quadratically in the asymmetry, so a population
        // spread along the front settles about it slowly
        let raw = front
            .iter()
            .map(|(c, _)| (-c[0] - front_drop(product_at(c[1]))).abs())
            .fold(0.0, f64::max);
        println!(
            "  {name} (population 60, 40000 evaluations, seed 1): {} points, widths {narrowest:.4} to {widest:.4} nm, hypervolume {hv:.6}; largest |D − D_front| {}",
            front.len(),
            order(raw)
        );
        checks.against(
            &format!("{name}: hypervolume / the front's"),
            hv / analytic,
            "the closed form",
            1.0,
            0.01,
        );
        // each point polished by GCMMA: the most drop at no more than its width (an
        // ε-constraint), on the circuit adjoint's gradient
        let polished: Vec<[f64; 2]> = front
            .par_iter()
            .map(|(c, k)| polish(&problem, [k[0], k[1]], c[1]))
            .collect();
        let worst = polished
            .iter()
            .zip(front)
            .map(|(k, (c, _))| {
                let p = ((1.0 - k[0]) * (1.0 - k[1])).sqrt();
                let drop = problem.drop(*k);
                // still at its width, and on the front there
                ((drop - front_drop(p)).abs()).max((fwhm(p) - c[1]).max(0.0))
            })
            .fold(0.0, f64::max);
        let asymmetry = polished
            .iter()
            .map(|k| (k[0] - k[1]).abs())
            .fold(0.0, f64::max);
        println!(
            "    each point then polished by GCMMA at its width: |κ₁² − κ₂²| at most {}",
            order(asymmetry)
        );
        checks.against(
            &format!("{name}, polished: largest |D − D_front|"),
            worst,
            "the closed form",
            0.0,
            1e-3,
        );
    }
    Ok(checks.finish())
}

/// GCMMA from couplings `k`: the largest drop with the width at most `width` nm, the circuit
/// adjoint's gradient of the drop and Eq. 8's of the width.
fn polish(problem: &Problem, k: [f64; 2], width: f64) -> [f64; 2] {
    let a = amplitude();
    let scale = resonance().powi(2) / (PI * GROUP_INDEX * length()) * 1e3;
    let lambda = [problem.resonance];
    let fitness = Constrained::differentiable(
        1,
        |x: &Reals, gradient: &mut [f64], g: &mut [f64], jacobian: &mut [f64]| {
            let mut v = problem.base.clone();
            v[problem.slots[0]] = x[0];
            v[problem.slots[1]] = x[1];
            let Ok((drop, dd)) = problem
                .circuit
                .gradient(&lambda, &v, |s| objective::power(s, 3, 0))
            else {
                return f64::NAN;
            };
            gradient[0] = -dd[problem.slots[0]];
            gradient[1] = -dd[problem.slots[1]];
            let p = ((1.0 - x[0]) * (1.0 - x[1])).sqrt();
            let pa = p * a;
            g[0] = fwhm(p) - width;
            // dFWHM/dp = −scale a (1 + pa)/(2 (pa)^(3/2)); dp/dκ₁² = −(1 − κ₂²)/(2p)
            let dw = -scale * a * (1.0 + pa) / (2.0 * pa * pa.sqrt());
            jacobian[0] = dw * (-(1.0 - x[1]) / (2.0 * p));
            jacobian[1] = dw * (-(1.0 - x[0]) / (2.0 * p));
            -drop
        },
    );
    let mma = Mma::builder(Real::new(vec![KAPPA2.0..=KAPPA2.1; 2]).expect("bounds"))
        .method(mma::Method::Gcmma)
        .initial_genome(Reals::from(k.to_vec()))
        .minimize()
        .build()
        .expect("a valid GCMMA");
    let outcome = Engine::new(mma, fitness)
        .stop_when(Stop::evaluations(400))
        .run()
        .expect("a run");
    let x = outcome.best_genome();
    [x[0], x[1]]
}

/// A small number's order of magnitude, as printed.
fn order(v: f64) -> String {
    if v < 1e-9 {
        "below 1e-9".into()
    } else {
        format!("{v:.0e}")
    }
}
