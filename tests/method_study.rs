//! A method study on Bennet et al.'s Bragg mirror (docs/methods/optimization.md, "A method
//! study"): how many evaluations Bayesian optimization, differential evolution, CMA-ES and
//! L-BFGS-B take to reach the quarter-wave stack, over 31 seeds each.
//!
//! P. Bennet et al., J. Opt. Soc. Am. B 41, A126 (2024), doi:10.1364/JOSAB.506389, Section 3 and
//! Figs. 3–6, with the problem as their published code states it (Zenodo,
//! doi:10.5281/zenodo.10246032): layers of n = 1.8 and 1.4 alternating from the top, on 1.8,
//! under air, at 600 nm and normal incidence; each thickness from 0 to 214.29 nm; the cost 1 − R.
//!
//! ```sh
//! cargo test --release --test method_study -- --ignored --nocapture
//! ```
//!
//! prints the table of docs/methods/optimization.md. Every run is seeded, and runs side by side
//! on rayon's threads, each on its own: the table is the same on any thread count, as the quick
//! test below checks on a small study.

use std::sync::Mutex;

use genoxide::prelude::*;
use photonoxide::Complex64 as c64;
use photonoxide::mode::Polarization;
use photonoxide::mode::multilayer::Multilayer;
use photonoxide::units::{Length, Wavelength};
use rayon::prelude::*;

/// The wavelength, µm.
const LAMBDA: f64 = 0.6;
const HIGH: f64 = 1.8;
const LOW: f64 = 1.4;
const SUBSTRATE: f64 = 1.8;
/// The largest thickness, nm: a half-wave of the lower index.
const HALF_WAVE: f64 = 1e3 * LAMBDA / (2.0 * LOW);
/// A film of 0 is left out of the transfer matrices; 1e-6 nm stands for it to 1e-10 in R.
const THINNEST: f64 = 1e-6;
/// How far above the quarter-wave stack's 1 − R a run's best must come to count as reaching it.
const TARGET: f64 = 1e-6;

fn index(j: usize) -> f64 {
    if j.is_multiple_of(2) { HIGH } else { LOW }
}

fn stack(d: &[f64]) -> Multilayer {
    let films: Vec<(c64, Length)> = d
        .iter()
        .enumerate()
        .map(|(j, &t)| (c64::new(index(j), 0.0), Length::nm(t.max(THINNEST))))
        .collect();
    Multilayer::new(c64::new(1.0, 0.0), &films, c64::new(SUBSTRATE, 0.0)).expect("a valid stack")
}

/// 1 − R and its gradient per nm (the same bits on every system: libm's sin and cos).
fn cost(d: &[f64], gradient: &mut [f64]) -> f64 {
    let lambda = Wavelength::um(LAMBDA).expect("a wavelength");
    let r = stack(d)
        .reflection_derivatives(Polarization::Te, lambda, 0.0)
        .expect("derivatives");
    for (g, (dr, _)) in gradient.iter_mut().zip(&r.by_thickness) {
        *g = -dr * 1e-3;
    }
    1.0 - r.reflection.reflectance
}

/// The quarter-wave stack's 1 − R in closed form, R = ((1 − Y)/(1 + Y))², Y = (1.8/1.4)^n 1.8.
fn optimum(n: usize) -> f64 {
    let y = (HIGH / LOW).powi(n as i32) * SUBSTRATE;
    1.0 - ((1.0 - y) / (1.0 + y)).powi(2)
}

/// A run's record: its evaluations so far, the first at which it reached the target, its best.
struct Record {
    target: f64,
    evaluations: u64,
    reached: Option<u64>,
    best: f64,
}

/// The cost, counting: what every method evaluates.
struct Counted {
    n: usize,
    record: Mutex<Record>,
}

impl Counted {
    fn new(n: usize) -> Counted {
        Counted {
            n,
            record: Mutex::new(Record {
                target: optimum(n) + TARGET,
                evaluations: 0,
                reached: None,
                best: f64::INFINITY,
            }),
        }
    }

    fn evaluate(&self, d: &[f64], gradient: &mut [f64]) -> f64 {
        let value = cost(d, gradient);
        let mut r = self.record.lock().expect("a record");
        r.evaluations += 1;
        r.best = r.best.min(value);
        if r.reached.is_none() && value <= r.target {
            r.reached = Some(r.evaluations);
        }
        value
    }

    fn value(&self, d: &[f64]) -> f64 {
        self.evaluate(d, &mut vec![0.0; self.n])
    }

    fn evaluations(&self) -> u64 {
        self.record.lock().expect("a record").evaluations
    }

    /// The first evaluation at the target, and the best 1 − R.
    fn outcome(self) -> (Option<u64>, f64) {
        let r = self.record.into_inner().expect("a record");
        (r.reached, r.best)
    }
}

fn space(n: usize) -> Real {
    Real::new(vec![0.0..=HALF_WAVE; n]).expect("bounds")
}

/// The methods compared.
#[derive(Clone, Copy, Debug)]
enum Method {
    /// genoxide's Bayesian optimization, its defaults (a Gaussian process, log expected
    /// improvement, a Latin hypercube of 2(n + 1) points first).
    Bo,
    /// Bennet's DE: current-to-best/1, F 0.8, CR 0.5, 30 individuals.
    De,
    /// CMA-ES, genoxide's defaults, from the middle of the box.
    Cmaes,
    /// L-BFGS-B on the exact gradient, from seeded random starts, each run to its own stop,
    /// restarted until the budget is spent.
    Lbfgsb,
}

impl Method {
    fn name(self) -> &'static str {
        match self {
            Method::Bo => "Bayesian optimization",
            Method::De => "DE (current-to-best/1, F 0.8, CR 0.5, 30)",
            Method::Cmaes => "CMA-ES",
            Method::Lbfgsb => "L-BFGS-B on dR/dd, restarted",
        }
    }

    /// One run on `n` layers with seed `seed` and `budget` evaluations: the first evaluation at
    /// the target, and the best 1 − R.
    fn run(self, n: usize, seed: u64, budget: u64) -> (Option<u64>, f64) {
        let counted = Counted::new(n);
        let value = |x: &Reals| counted.value(x);
        let stop = || Stop::evaluations(budget);
        match self {
            Method::Bo => {
                let bo = Bo::builder(space(n))
                    .minimize()
                    .seed(seed)
                    .build()
                    .expect("a valid BO");
                Engine::new(bo, &value)
                    .stop_when(stop())
                    .run()
                    .expect("a run");
            }
            Method::De => {
                let de = De::builder(space(n))
                    .population_size(30)
                    .strategy(de::Strategy::CurrentToPBest {
                        p: 1.0 / 30.0,
                        archive: 0.0,
                    })
                    .control(de::Control::Fixed { f: 0.8, cr: 0.5 })
                    .minimize()
                    .seed(seed)
                    .build()
                    .expect("a valid DE");
                Engine::new(de, &value)
                    .stop_when(stop())
                    .run()
                    .expect("a run");
            }
            Method::Cmaes => {
                let cmaes = Cmaes::builder(space(n))
                    .minimize()
                    .seed(seed)
                    .build()
                    .expect("a valid CMA-ES");
                Engine::new(cmaes, &value)
                    .stop_when(stop())
                    .run()
                    .expect("a run");
            }
            Method::Lbfgsb => {
                let gradient = |x: &Reals, g: &mut [f64]| counted.evaluate(x, g);
                let mut rng = StreamRng::seed_from_u64(seed);
                while counted.evaluations() < budget {
                    let left = budget - counted.evaluations();
                    let lbfgsb = Lbfgsb::builder(space(n))
                        .initial_genome(space(n).random_genome(&mut rng))
                        .gradient_tolerance(1e-10)
                        .minimize()
                        .build()
                        .expect("a valid L-BFGS-B");
                    Engine::new(lbfgsb, Differentiable(&gradient))
                        .stop_when(Stop::evaluations(left))
                        .run()
                        .expect("a run");
                }
            }
        }
        counted.outcome()
    }
}

/// A method's runs over `seeds` seeds: how many reached the target, the evaluations they took
/// (sorted), and every run's best 1 − R (sorted).
struct Summary {
    reached: Vec<u64>,
    best: Vec<f64>,
}

fn study(method: Method, n: usize, seeds: u64, budget: u64) -> Summary {
    let runs: Vec<(Option<u64>, f64)> = (1..=seeds)
        .into_par_iter()
        .map(|seed| method.run(n, seed, budget))
        .collect();
    let mut reached: Vec<u64> = runs.iter().filter_map(|r| r.0).collect();
    reached.sort_unstable();
    let mut best: Vec<f64> = runs.iter().map(|r| r.1).collect();
    best.sort_by(f64::total_cmp);
    Summary { reached, best }
}

/// The value at quantile q of sorted `v` (the nearest rank).
fn quantile<T: Copy>(v: &[T], q: f64) -> T {
    let k = ((q * v.len() as f64).ceil() as usize).clamp(1, v.len());
    v[k - 1]
}

/// The study's table, in Markdown.
fn table(cases: &[(usize, &[(Method, u64)])], seeds: u64) -> String {
    let mut out = String::new();
    for &(n, methods) in cases {
        let best = optimum(n);
        out.push_str(&format!(
            "\n{n} layers: the quarter-wave stack's 1 − R = {best:.6}, the target within {TARGET:e} of it\n\n"
        ));
        out.push_str("| method | budget | runs reaching the target | evaluations to it: median (quartiles) | best 1 − R − the optimum: best, median, worst |\n");
        out.push_str("|---|---|---|---|---|\n");
        for &(method, budget) in methods {
            let s = study(method, n, seeds, budget);
            let evaluations = if s.reached.is_empty() {
                "–".to_string()
            } else {
                format!(
                    "{} ({}–{})",
                    quantile(&s.reached, 0.5),
                    quantile(&s.reached, 0.25),
                    quantile(&s.reached, 0.75)
                )
            };
            // no run beats the quarter-wave stack: it is the optimum, to round-off
            assert!(
                s.best[0] >= best - 1e-12,
                "{} found 1 − R = {} below the quarter-wave stack's {best}",
                method.name(),
                s.best[0]
            );
            let gap = |v: f64| {
                let g = v - best;
                if g < 1e-9 {
                    "below 1e-9".to_string()
                } else {
                    format!("{g:.1e}")
                }
            };
            out.push_str(&format!(
                "| {} | {budget} | {} of {seeds} | {evaluations} | {}, {}, {} |\n",
                method.name(),
                s.reached.len(),
                gap(s.best[0]),
                gap(quantile(&s.best, 0.5)),
                gap(s.best[s.best.len() - 1]),
            ));
        }
    }
    out
}

#[test]
#[ignore = "the full study: minutes; `cargo test --release --test method_study -- --ignored --nocapture`"]
fn bragg_method_study() {
    let ten: &[(Method, u64)] = &[
        (Method::Bo, 300),
        (Method::De, 5_000),
        (Method::Cmaes, 5_000),
        (Method::Lbfgsb, 5_000),
    ];
    let twenty: &[(Method, u64)] = &[
        (Method::Bo, 300),
        (Method::De, 20_000),
        (Method::Cmaes, 20_000),
        (Method::Lbfgsb, 20_000),
    ];
    print!("{}", table(&[(10, ten), (20, twenty)], 31));
}

#[test]
fn a_small_study_is_the_same_on_one_thread_and_on_many() {
    let methods: &[(Method, u64)] = &[
        (Method::Bo, 40),
        (Method::De, 600),
        (Method::Cmaes, 600),
        (Method::Lbfgsb, 600),
    ];
    let small = || table(&[(6, methods)], 3);
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("a pool")
        .install(small);
    let many = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("a pool")
        .install(small);
    assert_eq!(one, many);
}
