//! A Bragg mirror found by optimization: the thicknesses of 20 (and 10) alternating layers
//! searched by genoxide's differential evolution and CMA-ES on the stack's reflectance by
//! transfer matrices, then polished by L-BFGS-B on its exact derivative in each thickness, until
//! they are the quarter-wave stack.
//!
//! P. Bennet et al., J. Opt. Soc. Am. B 41, A126 (2024),
//! [doi:10.1364/JOSAB.506389](https://doi.org/10.1364/JOSAB.506389) (open: arXiv:2309.09760),
//! Section 3 and Figs. 6–7, with the problem as their published code states it (Zenodo,
//! doi:10.5281/zenodo.10246032): layers of n = 1.8 and 1.4 alternating from the top (the first
//! 1.8), on a substrate of 1.8, under air, at 600 nm and normal incidence; each thickness between
//! 0 and a half-wave of the lower index, 214.29 nm; the cost 1 − R. Their DE is
//! "current-to-best" with F₁ = F₂ = 0.8 and CR = 1/2, 30 individuals (their Table 1), genoxide's
//! current-to-pbest/1 with p at the best alone (M. A. Barry et al., Sci. Rep. 10, 12024 (2020),
//! doi:10.1038/s41598-020-68719-3). The best structures their Fig. 7c shows are the quarter-wave
//! stack, 83 and 107 nm (from a plot): exactly λ/4n, 83.333 and 107.143 nm. Its reflectance, in
//! closed form R = ((1 − Y)/(1 + Y))² with Y = (1.8/1.4)^(2N) · 1.8 for N pairs, is 0.985521 for
//! 20 layers and 0.835144 for 10, the lowest costs of their Fig. 6 (from a plot); the plan
//! (docs/plans/optimization.md) prints them to 6 digits, the tolerance here.
//!
//! The derivative is `Multilayer::reflection_derivatives`, checked here against fourth-order
//! differences before L-BFGS-B uses it. Transfer matrices are exact: no grid. CMA-ES, started
//! from the middle of the bounds, is printed beside DE and not checked: on 20 layers it settles
//! in another minimum, as their Fig. 6 has CMA-ES behind DE (from a plot).
//!
//! ```sh
//! cargo run --release --example optimize_bragg
//! ```

mod common;

use std::process::ExitCode;

use genoxide::prelude::*;
use photonoxide::Complex64 as c64;
use photonoxide::mode::Polarization;
use photonoxide::mode::multilayer::Multilayer;
use photonoxide::units::{Length, Wavelength};
use rayon::prelude::*;

/// The wavelength, µm.
const LAMBDA: f64 = 0.6;
/// The indices, from the top.
const HIGH: f64 = 1.8;
const LOW: f64 = 1.4;
/// The substrate's.
const SUBSTRATE: f64 = 1.8;
/// The largest thickness, nm: a half-wave of the lower index.
const HALF_WAVE: f64 = 1e3 * LAMBDA / (2.0 * LOW);
/// The thinnest film the transfer matrices take, nm: a film of 0 is left out, which a film of
/// 1e-6 nm stands for to within 1e-10 in R.
const THINNEST: f64 = 1e-6;
/// The seed of every random method.
const SEED: u64 = 1;

/// Layer j's index: 1.8 from the top, alternating.
fn index(j: usize) -> f64 {
    if j.is_multiple_of(2) { HIGH } else { LOW }
}

/// The stack of thicknesses `d` (nm).
fn stack(d: &[f64]) -> Multilayer {
    let films: Vec<(c64, Length)> = d
        .iter()
        .enumerate()
        .map(|(j, &t)| (c64::new(index(j), 0.0), Length::nm(t.max(THINNEST))))
        .collect();
    Multilayer::new(c64::new(1.0, 0.0), &films, c64::new(SUBSTRATE, 0.0)).expect("a valid stack")
}

fn lambda() -> Wavelength {
    Wavelength::um(LAMBDA).expect("a wavelength")
}

/// 1 − R, by `reflection_derivatives`: with libm's sin and cos, the same bits on every system,
/// so that the optimizers' paths and counts are too.
fn cost(d: &[f64]) -> f64 {
    let r = stack(d)
        .reflection_derivatives(Polarization::Te, lambda(), 0.0)
        .expect("a reflection");
    1.0 - r.reflection.reflectance
}

/// 1 − R and its gradient, per nm.
fn cost_and_gradient(d: &[f64], gradient: &mut [f64]) -> f64 {
    let r = stack(d)
        .reflection_derivatives(Polarization::Te, lambda(), 0.0)
        .expect("derivatives");
    for (g, (dr, _)) in gradient.iter_mut().zip(&r.by_thickness) {
        // dR/dd per µm, d in nm
        *g = -dr * 1e-3;
    }
    1.0 - r.reflection.reflectance
}

/// The quarter-wave stack of `n` layers, nm.
fn quarter_wave(n: usize) -> Vec<f64> {
    (0..n).map(|j| 1e3 * LAMBDA / (4.0 * index(j))).collect()
}

/// Its reflectance in closed form.
fn closed_form(n: usize) -> f64 {
    let y = (HIGH / LOW).powi(n as i32) * SUBSTRATE;
    ((1.0 - y) / (1.0 + y)).powi(2)
}

/// The largest relative difference between the exact gradient at `d` and fourth-order central
/// differences of the cost, step 1e-3 nm.
fn gradient_check(d: &[f64]) -> f64 {
    let mut g = vec![0.0; d.len()];
    cost_and_gradient(d, &mut g);
    let h = 1e-3;
    let scale = g.iter().fold(0.0f64, |a, v| a.max(v.abs()));
    (0..d.len())
        .map(|j| {
            let at = |k: f64| {
                let mut e = d.to_vec();
                e[j] += k * h;
                cost(&e)
            };
            let fd = (at(-2.0) - 8.0 * at(-1.0) + 8.0 * at(1.0) - at(2.0)) / (12.0 * h);
            (fd - g[j]).abs() / scale
        })
        .fold(0.0, f64::max)
}

/// Bennet's DE on `n` layers with a budget of `budget` evaluations, on a pool of `threads`
/// threads (the generation evaluated side by side): its best thicknesses and cost.
fn de(n: usize, budget: u64, threads: usize) -> (Vec<f64>, f64) {
    let real = Real::new(vec![0.0..=HALF_WAVE; n]).expect("bounds");
    let algorithm = De::builder(real)
        .population_size(30)
        .strategy(de::Strategy::CurrentToPBest {
            p: 1.0 / 30.0,
            archive: 0.0,
        })
        .control(de::Control::Fixed { f: 0.8, cr: 0.5 })
        .minimize()
        .seed(SEED)
        .build()
        .expect("a valid DE");
    // a generation's genomes side by side on rayon's threads, their costs in their order
    let fitness = Batch(|ds: &[&Reals]| ds.par_iter().map(|d| cost(d)).collect::<Vec<f64>>());
    let outcome = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("a pool")
        .install(|| {
            Engine::new(algorithm, fitness)
                .stop_when(Stop::evaluations(budget))
                .run()
                .expect("a run")
        });
    let best = outcome.best_genome().to_vec();
    let c = cost(&best);
    (best, c)
}

/// CMA-ES on `n` layers from the middle of the bounds, with a budget of `budget`.
fn cmaes(n: usize, budget: u64) -> (Vec<f64>, f64) {
    let real = Real::new(vec![0.0..=HALF_WAVE; n]).expect("bounds");
    let algorithm = Cmaes::builder(real)
        .initial_mean(Reals::from(vec![HALF_WAVE / 2.0; n]))
        .minimize()
        .seed(SEED)
        .build()
        .expect("a valid CMA-ES");
    let fitness = |d: &Reals| cost(d);
    let outcome = Engine::new(algorithm, &fitness)
        .stop_when(Stop::evaluations(budget))
        .run()
        .expect("a run");
    let best = outcome.best_genome().to_vec();
    let c = cost(&best);
    (best, c)
}

/// L-BFGS-B on the exact gradient from `start`: the thicknesses, the cost and the evaluations.
fn polish(start: &[f64]) -> (Vec<f64>, f64, u64) {
    let n = start.len();
    let real = Real::new(vec![0.0..=HALF_WAVE; n]).expect("bounds");
    let algorithm = Lbfgsb::builder(real)
        .initial_genome(Reals::from(start.to_vec()))
        .gradient_tolerance(1e-14)
        .function_tolerance(0.0)
        .minimize()
        .build()
        .expect("a valid L-BFGS-B");
    let fitness = |d: &Reals, g: &mut [f64]| cost_and_gradient(d, g);
    let outcome = Engine::new(algorithm, Differentiable(&fitness))
        .stop_when(Stop::evaluations(2_000))
        .run()
        .expect("a run");
    let best = outcome.best_genome().to_vec();
    let c = cost(&best);
    (best, c, outcome.evaluations())
}

/// The largest |d − d_QW|, nm.
fn from_quarter_wave(d: &[f64]) -> f64 {
    d.iter()
        .zip(quarter_wave(d.len()))
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max)
}

fn thicknesses(d: &[f64]) -> String {
    d.iter()
        .map(|t| format!("{t:.3}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let mut checks = common::Checks::default();
    println!(
        "Bragg mirrors of n = 1.8/1.4 on 1.8 under air at 600 nm (Bennet et al. 2024), transfer matrices, exact: no grid"
    );
    for (n, printed, budget) in [(20, 0.985521, 20_000), (10, 0.835144, 5_000)] {
        println!("\n{n} layers, each 0 to {HALF_WAVE:.2} nm; cost 1 − R");
        let qw = quarter_wave(n);
        checks.against(
            &format!("R of the quarter-wave stack, {n} layers"),
            1.0 - cost(&qw),
            "closed form",
            closed_form(n),
            1e-12,
        );
        checks.compare(
            &format!("its closed form, {n} layers"),
            closed_form(n),
            printed,
            5e-7,
        );
        let mut g = vec![0.0; n];
        cost_and_gradient(&qw, &mut g);
        let stationary = g.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        checks.against(
            "|dR/dd| at the quarter-wave stack, per nm",
            stationary,
            "a stationary point",
            0.0,
            1e-12,
        );

        let (d_de, c_de) = de(n, budget, 1);
        println!(
            "  DE (current-to-best/1, F 0.8, CR 0.5, 30 individuals, seed {SEED}), {budget} evaluations: 1 − R = {c_de:.6e}"
        );
        let (d_twenty, _) = de(n, budget, 20);
        let differ = d_de
            .iter()
            .zip(&d_twenty)
            .filter(|(a, b)| a.to_bits() != b.to_bits())
            .count();
        checks.against(
            "DE's best: bits differing, 1 and 20 threads",
            differ as f64,
            "the same bits",
            0.0,
            0.0,
        );
        // the derivative checked before L-BFGS-B uses it, where the gradient is far from 0:
        // every layer at a quarter of the bounds
        let check = gradient_check(&vec![HALF_WAVE / 4.0; n]);
        println!("  exact dR/dd against fourth-order differences: {check:.1e} of the largest");
        checks.against(
            "exact dR/dd − differences, ×1e8",
            check * 1e8,
            "agreement",
            0.0,
            1.0,
        );
        let (d, c, evaluations) = polish(&d_de);
        println!("  then L-BFGS-B on dR/dd: {evaluations} evaluations, 1 − R = {c:.6e}");
        println!("  thicknesses (nm): {}", thicknesses(&d));
        checks.compare(
            &format!("R after the polish, {n} layers"),
            1.0 - c,
            printed,
            5e-7,
        );
        checks.against(
            "largest |d − λ/4n| after it (nm)",
            from_quarter_wave(&d),
            "λ/4n, Fig. 7c",
            0.0,
            0.01,
        );

        let (d_cma, c_cma) = cmaes(n, budget);
        let (d, c, evaluations) = polish(&d_cma);
        println!(
            "  CMA-ES (from the middle of the bounds, seed {SEED}), {budget} evaluations: 1 − R = {c_cma:.6e}; polished in {evaluations}: 1 − R = {c:.6e}, {:.3} nm from the quarter-wave stack at most",
            from_quarter_wave(&d)
        );
    }
    Ok(checks.finish())
}
