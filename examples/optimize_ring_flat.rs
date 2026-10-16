//! Maximally flat filters of coupled rings found by optimization: N = 2 to 6 rings in series
//! between two buses, their couplings fitted by genoxide's L-BFGS-B on the circuit adjoint's
//! gradient (from CMA-ES's best, a global check) until the drop port's response is the
//! Butterworth shape 1/(1 + (δ/δ_c)^2N), and the ring-to-ring couplings compared with Little's
//! synthesis.
//!
//! B. E. Little, S. T. Chu, H. A. Haus, J. Foresi, J.-P. Laine, J. Lightwave Technol. 15, 998
//! (1997), [doi:10.1109/50.588673](https://doi.org/10.1109/50.588673), Table I: the coupling
//! coefficients of maximally flat filters as μᵢ²/μ⁴, μ the bus's coupling (μ² = 2/τ_e, its
//! energy decay rate) and μᵢ the i-th ring-to-ring one (by symmetry the first half): 0.250
//! (N = 2); 0.125 (3); 0.100, 0.040 (4); 0.0955, 0.0295 (5); 0.0915, 0.0245, 0.0179 (6).
//!
//! **In the circuit.** Each ring's round trip T and power couplings κ²: the bus couples the
//! energy out at μ² = κ_b²/T and ring i to ring i + 1 at μᵢ = κᵢ/T, so μᵢ²/μ⁴ = κᵢ²/κ_b⁴,
//! independent of T. Coupled-mode theory, which Little's table is, is the limit of small κ²:
//! the circuit's exact couplers differ from it at O(κ_b²), which this example shows at
//! κ_b² = 1e-3, 1e-4 and 1e-5, falling tenfold with it, and checks at 1e-5, where it is below
//! the printed digits (N = 6's third, 0.017949 by the prototype, is printed 0.0179).
//!
//! **The prototype.** The relation μᵢ²/μ⁴ = g₁²/(4 gᵢ gᵢ₊₁), with gₖ = 2 sin((2k − 1)π/2N) the
//! Butterworth low-pass prototype's elements (G. L. Matthaei, L. Young, E. M. T. Jones,
//! Microwave Filters, Impedance-Matching Networks, and Coupling Structures, Artech House (1980),
//! Section 4.05, no DOI), was derived for #244's plan, not taken from the paper: coupled
//! resonators of external decay γ_e = Δω/g₁ and couplings kᵢ = Δω/√(gᵢgᵢ₊₁) give
//! kᵢ²/(2γ_e)² = g₁²/(4gᵢgᵢ₊₁). It reproduces every row of Table I but N = 4, whose printed
//! 0.100 and 0.040 the prototype makes 0.1036 and 0.0429: N = 4 is checked against the
//! prototype and the paper's values printed beside (#244's decision 4).
//!
//! Lossless rings, the response against the round-trip phase detuning δ (each ring's phase the
//! same: a frequency's), 61 points over ±3κ_b². The circuit solve: exact, no grid.
//!
//! ```sh
//! cargo run --release --example optimize_ring_flat
//! ```

mod common;

use std::cell::Cell;
use std::f64::consts::PI;
use std::process::ExitCode;
use std::sync::Arc;

use genoxide::prelude::*;
use photonoxide::circuit::components::{Coupler, PhaseShifter};
use photonoxide::circuit::{Circuit, Netlist, SMatrix};
use photonoxide::units::Wavelength;
use rayon::prelude::*;

/// The wavelength: the rings' phases don't depend on it.
const LAMBDA: f64 = 1.55;

/// N rings in series: couplers c0 … cN (c0 the input bus's, cN the drop bus's), each ring two
/// phase shifters, a{k} and b{k}; ports in, through, add, drop.
fn chain(n: usize) -> photonoxide::Result<Circuit> {
    let coupler = Arc::new(Coupler::new());
    let shifter = Arc::new(PhaseShifter::new());
    let mut net = Netlist::new();
    for k in 0..=n {
        net.add(&format!("c{k}"), coupler.clone())?;
    }
    for k in 1..=n {
        net.add(&format!("a{k}"), shifter.clone())?;
        net.add(&format!("b{k}"), shifter.clone())?;
        // ring k between c{k-1} (its o3/o2 side for the first, o4/o1 for the others) and c{k}
        let (out, back) = if k == 1 { ("o3", "o2") } else { ("o4", "o1") };
        let below = format!("c{}", k - 1);
        let above = format!("c{k}");
        net.connect(&format!("{below}.{out}"), &format!("a{k}.o1"))?;
        net.connect(&format!("a{k}.o2"), &format!("{above}.o2"))?;
        net.connect(&format!("{above}.o3"), &format!("b{k}.o1"))?;
        net.connect(&format!("b{k}.o2"), &format!("{below}.{back}"))?;
    }
    net.expose("in", "c0.o1")?;
    net.expose("through", "c0.o4")?;
    net.expose("add", &format!("c{n}.o1"))?;
    net.expose("drop", &format!("c{n}.o4"))?;
    net.compile()
}

/// The Butterworth prototype's elements g₁ … g_N.
fn prototype(n: usize) -> Vec<f64> {
    (1..=n)
        .map(|k| 2.0 * ((2 * k - 1) as f64 * PI / (2 * n) as f64).sin())
        .collect()
}

/// Little's Table I, the first half of μᵢ²/μ⁴ for N = 2 … 6 (N = 4 as printed).
fn table(n: usize) -> Vec<f64> {
    match n {
        2 => vec![0.250],
        3 => vec![0.125],
        4 => vec![0.100, 0.040],
        5 => vec![0.0955, 0.0295],
        _ => vec![0.0915, 0.0245, 0.0179],
    }
}

/// The detunings, in units of κ_b².
fn detunings() -> Vec<f64> {
    (0..61).map(|k| -3.0 + 0.1 * f64::from(k)).collect()
}

/// The problem for N rings at bus coupling `bus` (κ_b²): the circuit, the slots of its inner
/// couplings and of its shifters.
struct Filter {
    n: usize,
    bus: f64,
    circuit: Circuit,
    couplings: Vec<usize>,
    shifters: Vec<usize>,
}

impl Filter {
    fn new(n: usize, bus: f64) -> photonoxide::Result<Filter> {
        let circuit = chain(n)?;
        let at = |name: &str| circuit.parameter(name).expect("a parameter");
        let couplings = (1..n).map(|k| at(&format!("c{k}.coupling"))).collect();
        let shifters = (1..=n)
            .flat_map(|k| [format!("a{k}.phase"), format!("b{k}.phase")])
            .map(|name| at(&name))
            .collect();
        Ok(Filter {
            n,
            bus,
            circuit,
            couplings,
            shifters,
        })
    }

    /// The circuit's values: the bus couplers at κ_b², ring i to i + 1 at rᵢ κ_b⁴, and every
    /// half ring's phase δ/2.
    fn values(&self, ratios: &[f64], delta: f64) -> Vec<f64> {
        let mut v = self.circuit.values().to_vec();
        let at = |name: &str| self.circuit.parameter(name).expect("a parameter");
        v[at("c0.coupling")] = self.bus;
        v[at(&format!("c{}.coupling", self.n))] = self.bus;
        for (&slot, r) in self.couplings.iter().zip(ratios) {
            v[slot] = r * self.bus * self.bus;
        }
        for &slot in &self.shifters {
            v[slot] = delta / 2.0;
        }
        v
    }

    /// The misfit to the Butterworth shape and its gradient in x = (r₁ … r_{N−1}, s), δ_c =
    /// s κ_b²: by the circuit adjoint at each detuning, and the target's own derivative in s.
    fn misfit(&self, x: &[f64], gradient: &mut [f64]) -> f64 {
        let m = self.n - 1;
        let s = x[m];
        let lambda = [Wavelength::um(LAMBDA).expect("a wavelength")];
        let two_n = 2 * self.n as i32;
        gradient.fill(0.0);
        let mut total = 0.0;
        for t in detunings() {
            let u = (t / s).powi(two_n);
            let target = 1.0 / (1.0 + u);
            let values = self.values(&x[..m], t * self.bus);
            let drop = Cell::new(0.0);
            let objective = |s: &[SMatrix]| {
                let p = s[0].power(3, 0);
                drop.set(p);
                let mut g = SMatrix::zeros(4);
                g[(3, 0)] = s[0][(3, 0)].conj() * (2.0 * (p - target));
                ((p - target) * (p - target), vec![g])
            };
            let Ok((f, g)) = self.circuit.gradient(&lambda, &values, objective) else {
                return f64::NAN;
            };
            total += f;
            for (i, &slot) in self.couplings.iter().enumerate() {
                gradient[i] += g[slot] * self.bus * self.bus;
            }
            // d(target)/ds = (2N/s) u/(1 + u)², and the misfit's: −2 (p − target) d(target)/ds
            let dtarget = (f64::from(two_n) / s) * u / ((1.0 + u) * (1.0 + u));
            gradient[m] += -2.0 * (drop.get() - target) * dtarget;
        }
        total
    }
}

/// CMA-ES, then L-BFGS-B on the adjoint gradient: the ratios rᵢ, s and the misfit.
/// The number of starts of L-BFGS-B.
const STARTS: u64 = 16;

/// The full ratios r₁ … r_{N−1} and s from the half y = (r₁ … r_⌈(N−1)/2⌉, s): a maximally flat
/// filter is symmetric, μᵢ = μ_{N−i}, as Little's table has it.
fn full(n: usize, y: &[f64]) -> Vec<f64> {
    let half = y.len() - 1;
    let mut x: Vec<f64> = (0..n - 1).map(|i| y[i.min(n - 2 - i)]).collect();
    x.push(y[half]);
    x
}

/// The misfit in the half and its gradient: each half coupling's the sum of its two.
fn misfit_half(filter: &Filter, y: &[f64], gradient: &mut [f64]) -> f64 {
    let n = filter.n;
    let x = full(n, y);
    let mut g = vec![0.0; n];
    let f = filter.misfit(&x, &mut g);
    gradient.fill(0.0);
    for i in 0..n - 1 {
        gradient[i.min(n - 2 - i)] += g[i];
    }
    gradient[y.len() - 1] = g[n - 1];
    f
}

/// What a fit found: the ratios and s, the misfit, and CMA-ES's misfit after the same polish.
struct Fit {
    x: Vec<f64>,
    misfit: f64,
    cmaes: f64,
}

/// L-BFGS-B on the circuit adjoint from 16 seeded random starts in the symmetric half; and
/// CMA-ES (seed 1, 4000 evaluations) then the same polish, as a global check.
fn fit(filter: &Filter) -> Fit {
    let half = filter.n / 2;
    let mut bounds = vec![1e-3..=0.3; half];
    bounds.push(0.2..=3.0);
    let real = Real::new(bounds).expect("bounds");
    let gradient = |y: &Reals, g: &mut [f64]| misfit_half(filter, y, g);
    let polish = |start: Reals| -> (Vec<f64>, f64) {
        let lbfgsb = Lbfgsb::builder(real.clone())
            .initial_genome(start)
            .gradient_tolerance(1e-15)
            .function_tolerance(0.0)
            .minimize()
            .build()
            .expect("a valid L-BFGS-B");
        let outcome = Engine::new(lbfgsb, Differentiable(&gradient))
            .stop_when(Stop::evaluations(3_000))
            .run()
            .expect("a run");
        let y = outcome.best_genome().to_vec();
        let mut g = vec![0.0; y.len()];
        let f = misfit_half(filter, &y, &mut g);
        (y, f)
    };
    // the starts side by side on rayon's threads, gathered in their order
    let found: Vec<(Vec<f64>, f64)> = (1..=STARTS)
        .into_par_iter()
        .map(|seed| polish(real.random_genome(&mut StreamRng::seed_from_u64(seed))))
        .collect();
    let (best, misfit) = found
        .iter()
        .fold(None::<&(Vec<f64>, f64)>, |b, c| match b {
            Some(b) if b.1 <= c.1 => Some(b),
            _ => Some(c),
        })
        .cloned()
        .expect("a start");
    let value = |y: &Reals| {
        let mut g = vec![0.0; y.len()];
        misfit_half(filter, y, &mut g)
    };
    let cmaes = Cmaes::builder(real.clone())
        .minimize()
        .seed(1)
        .build()
        .expect("a valid CMA-ES");
    let outcome = Engine::new(cmaes, &value)
        .stop_when(Stop::evaluations(4_000))
        .run()
        .expect("a run");
    let (_, cmaes) = polish(outcome.best_genome().clone());
    Fit {
        x: full(filter.n, &best),
        misfit,
        cmaes,
    }
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let mut checks = common::Checks::default();
    println!(
        "Maximally flat filters of N series-coupled lossless rings (Little et al. 1997, Table I), the circuit solve, exact: no grid"
    );
    println!(
        "  each fitted to 1/(1 + (δ/δ_c)^2N) at 61 detunings over ±3κ_b², by L-BFGS-B on the circuit adjoint from 16 seeded random starts, the couplings symmetric; CMA-ES then L-BFGS-B as a global check"
    );
    // the circuit adjoint's gradient of the misfit against fourth-order differences, before
    // any optimizer uses it: four rings, away from the optimum
    {
        let filter = Filter::new(4, 1e-3)?;
        let x = [0.08, 0.05, 0.12, 0.6];
        let mut g = vec![0.0; 4];
        filter.misfit(&x, &mut g);
        let scale = g.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let h = 1e-5;
        let worst = (0..4)
            .map(|k| {
                let at = |s: f64| {
                    let mut y = x;
                    y[k] += s * h;
                    filter.misfit(&y, &mut [0.0; 4])
                };
                let fd = (at(-2.0) - 8.0 * at(-1.0) + 8.0 * at(1.0) - at(2.0)) / (12.0 * h);
                (fd - g[k]).abs() / scale
            })
            .fold(0.0, f64::max);
        checks.against(
            "N = 4: adjoint within 1e-8 of differences",
            f64::from(u8::from(worst < 1e-8)),
            "yes",
            1.0,
            0.0,
        );
    }
    for n in 2..=6 {
        let g = prototype(n);
        let printed = table(n);
        let half = printed.len();
        println!("\nN = {n}: μᵢ²/μ⁴ = κᵢ²/κ_b⁴, i = 1 … {half}");
        for bus in [1e-3, 1e-4, 1e-5] {
            let filter = Filter::new(n, bus)?;
            let Fit { x, misfit, cmaes } = fit(&filter);
            let ratios: Vec<String> = x[..n - 1].iter().map(|r| format!("{r:.5}")).collect();
            println!(
                "  κ_b² = {bus:e}: {} (misfit {}, δ_c = {:.4} κ_b², the best of {STARTS} starts; CMA-ES then L-BFGS-B: misfit {})",
                ratios.join(", "),
                magnitude(misfit),
                x[n - 1],
                magnitude(cmaes)
            );
            if bus > 1e-5 {
                continue;
            }
            for i in 0..half {
                let closed = g[0] * g[0] / (4.0 * g[i] * g[i + 1]);
                let tolerance = if printed[i] >= 0.1 && n != 4 {
                    5e-4
                } else {
                    5e-5
                };
                if n == 4 {
                    checks.against(
                        &format!("N = 4, μ{}²/μ⁴", i + 1),
                        x[i],
                        "prototype",
                        closed,
                        5e-5,
                    );
                    println!("  {:<34} the paper prints {}", "", printed[i]);
                } else {
                    checks.compare(
                        &format!("N = {n}, μ{}²/μ⁴", i + 1),
                        x[i],
                        printed[i],
                        tolerance,
                    );
                }
                // the mirror half: the same coupling
                checks.against(
                    &format!("N = {n}, μ{}²/μ⁴ (mirror)", n - 1 - i),
                    x[n - 2 - i],
                    "μ²/μ⁴ of its mirror",
                    x[i],
                    5e-5,
                );
            }
        }
    }
    Ok(checks.finish())
}

/// A misfit's order of magnitude, as printed: the digits of one at round-off move with the
/// order of a sum.
fn magnitude(f: f64) -> String {
    if f < 1e-18 {
        "below 1e-18".into()
    } else {
        format!("{f:.0e}")
    }
}
