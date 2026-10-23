//! A device's optimization run: genoxide's methods on the library's problem (#244's decision 1:
//! the library supplies the objectives, their derivatives and the record; the program, which
//! builds genoxide in, runs the optimizer).
//!
//! [`execute`] runs any job: an `"optimize"` one here, every other by `photonoxide::job::execute`.
//! The optimizer works in the variables scaled to [0, 1] by their bounds (a gradient scaled by
//! each variable's span), so that every method's steps and populations mean the same for a
//! length in µm and a loss in dB/cm. Each generation's points are evaluated side by side on
//! rayon's threads and recorded in the optimizer's order, so the record is the same, to the bit,
//! on any number of threads. The run stops at the budget, the optimizer's own end, the job's hard
//! limit or a Stop, whichever comes first, and its record says which.
//!
//! A task with a continuation (`[task.continuation]`) runs its method inside genoxide's
//! `Continuation`, which keeps the method's state from one stage to the next (L-BFGS-B built to
//! keep its curvature pairs) unless the task asks for `keep = "point"`. genoxide's stage closure
//! names the stage; the fitness sets it on the problem, and records it, before the stage's first
//! evaluation.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use genoxide::algorithm::continuation::{self, Continuation, Continue};
use genoxide::engine::{BatchExtras, FitnessFunction, Provided};
use genoxide::multi::MultiFitnessFunction;
use genoxide::prelude::{
    Algorithm, Cmaes, De, FirstOrder, Lbfgsb, Mma, MultiEngine, NelderMead, Nsga2,
    PolynomialMutation, Pso, Real, Reals, SimulatedBinaryCrossover, SmsEmoa, first_order,
};
use genoxide::{Objective, StopReason};
use photonoxide::job::{self, Evaluation, Problem, Recorder};
use photonoxide::run::{Job, Run, Stop};
use rayon::prelude::*;

/// Offers the exported files' compressor, the user's own libzstd when it is found, before a job
/// is checked or run: once a process ([`photonoxide_native::register_exporters`]).
pub fn exporters() {
    static EXPORTERS: std::sync::Once = std::sync::Once::new();
    EXPORTERS.call_once(photonoxide_native::register_exporters);
}

/// Runs `job` into `run`: an optimization by genoxide's method here, any other job as the
/// library runs it.
///
/// # Errors
///
/// What the job refuses, or the run's errors.
pub fn execute(job: &Job, run: &mut Run, stop: &Stop) -> photonoxide::Result<()> {
    exporters();
    if job::optimizes(job) {
        // its failure recorded and its results written, as job::execute does for the rest
        let outcome = optimize(job, run, stop);
        job::conclude(run, outcome)
    } else {
        job::execute(job, run, stop)
    }
}

/// The run's state the evaluations record into, in the optimizer's order.
struct State<'r> {
    run: &'r mut Run,
    recorder: Recorder,
    /// The first error writing the record, which ends the run.
    failed: Option<photonoxide::Error>,
}

/// genoxide's fitness function for a problem: a generation at a time, its points evaluated side
/// by side and recorded in order.
struct Fitness<'a, 'r> {
    problem: &'a Problem,
    state: &'a Mutex<State<'r>>,
    stop: &'a Stop,
    /// Each variable's lower bound and span: x = from + u span.
    from: Vec<f64>,
    span: Vec<f64>,
    /// The continuation's stage genoxide has gone on to, set on the problem before the next
    /// evaluation.
    stage: Arc<AtomicUsize>,
}

impl Fitness<'_, '_> {
    /// The point of the variables at `u`, in their units.
    fn x(&self, u: &Reals) -> Vec<f64> {
        self.from
            .iter()
            .zip(&self.span)
            .zip(u.iter())
            .map(|((f, s), u)| (f + u * s).clamp(*f, f + s))
            .collect()
    }

    /// Evaluates `us` side by side and records them in order; `None` for each once the run is
    /// stopped (they aren't evaluated or recorded).
    fn evaluate_all(&self, us: &[&Reals], gradient: bool) -> Vec<Option<Evaluation>> {
        if self.stop.reason().is_some() {
            return vec![None; us.len()];
        }
        let stage = self.stage.load(Ordering::Acquire);
        if self.problem.continuation().is_some() && stage != self.problem.stage() {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let State {
                run,
                recorder,
                failed,
            } = &mut *state;
            if let Err(err) = self
                .problem
                .set_stage(stage)
                .and_then(|()| recorder.stage(self.problem, run))
                && failed.is_none()
            {
                *failed = Some(err);
            }
        }
        let points: Vec<Vec<f64>> = us.iter().map(|u| self.x(u)).collect();
        let evaluations: Vec<Evaluation> = points
            .par_iter()
            .map(|x| self.problem.evaluate(x, gradient))
            .collect();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let State {
            run,
            recorder,
            failed,
        } = &mut *state;
        for (x, e) in points.iter().zip(&evaluations) {
            if failed.is_none()
                && let Err(err) = recorder.record(run, x, e)
            {
                *failed = Some(err);
            }
        }
        if failed.is_none()
            && let Err(err) = recorder.front(run)
        {
            *failed = Some(err);
        }
        evaluations.into_iter().map(Some).collect()
    }

    /// The score and violation genoxide takes: an invalid point's score is NaN.
    fn scored(e: Option<&Evaluation>, k: usize) -> (f64, f64) {
        match e {
            Some(e) if e.costs.iter().all(|c| c.is_finite()) && e.violation.is_finite() => {
                (e.costs[k], e.violation)
            }
            _ => (f64::NAN, 0.0),
        }
    }
}

/// The fitness of a method of one objective.
struct Single<'f, 'a, 'r>(&'f Fitness<'a, 'r>);

/// The fitness of a method of two.
struct Multi<'f, 'a, 'r>(&'f Fitness<'a, 'r>);

impl FitnessFunction<Reals> for Single<'_, '_, '_> {
    type Output = (f64, f64);

    fn evaluate(&self, u: &Reals) -> (f64, f64) {
        self.evaluate_batch(&[u])[0]
    }

    fn is_batch(&self) -> bool {
        true
    }

    fn evaluate_batch(&self, us: &[&Reals]) -> Vec<(f64, f64)> {
        self.0
            .evaluate_all(us, false)
            .iter()
            .map(|e| Fitness::scored(e.as_ref(), 0))
            .collect()
    }

    fn provides(&self) -> Provided {
        let m = self.0.problem.constraints();
        match (self.0.problem.method().gradient, m) {
            (false, _) => Provided::NOTHING,
            (true, 0) => Provided::GRADIENT,
            (true, m) => Provided::GRADIENT
                .with_inequalities(m)
                .with_constraint_jacobian(),
        }
    }

    fn evaluate_batch_with(&self, us: &[&Reals], extras: &mut BatchExtras<'_>) -> Vec<(f64, f64)> {
        let wanted = extras.wanted();
        let evaluations = self
            .0
            .evaluate_all(us, wanted.gradient || wanted.constraint_jacobian);
        let (n, m) = (self.0.span.len(), self.0.problem.constraints());
        for (p, e) in evaluations.iter().enumerate() {
            let usable = e.as_ref().filter(|e| e.costs.iter().all(|c| c.is_finite()));
            if let Some(g) = extras.gradients() {
                for j in 0..n {
                    g[p * n + j] = usable
                        .and_then(|e| e.gradients.first())
                        .map_or(f64::NAN, |d| d[j] * self.0.span[j]);
                }
            }
            if let Some(c) = extras.inequalities() {
                for i in 0..m {
                    c[p * m + i] = usable.map_or(f64::NAN, |e| e.constraints[i]);
                }
            }
            if let Some(jac) = extras.constraint_jacobians() {
                for i in 0..m {
                    for j in 0..n {
                        jac[p * m * n + i * n + j] = usable
                            .and_then(|e| e.jacobian.get(i))
                            .map_or(f64::NAN, |row| row[j] * self.0.span[j]);
                    }
                }
            }
        }
        evaluations
            .iter()
            .map(|e| Fitness::scored(e.as_ref(), 0))
            .collect()
    }
}

impl MultiFitnessFunction<Reals, 2> for Multi<'_, '_, '_> {
    type Output = ([f64; 2], f64);

    fn evaluate(&self, u: &Reals) -> ([f64; 2], f64) {
        MultiFitnessFunction::evaluate_batch(self, &[u])[0]
    }

    fn is_batch(&self) -> bool {
        true
    }

    fn evaluate_batch(&self, us: &[&Reals]) -> Vec<([f64; 2], f64)> {
        self.0
            .evaluate_all(us, false)
            .iter()
            .map(|e| {
                let (a, v) = Fitness::scored(e.as_ref(), 0);
                let (b, _) = Fitness::scored(e.as_ref(), 1);
                if a.is_finite() && b.is_finite() {
                    ([a, b], v)
                } else {
                    // an invalid point: the worst of anything
                    ([f64::MAX, f64::MAX], f64::MAX)
                }
            })
            .collect()
    }
}

/// genoxide's stop reason as the record names it.
fn reason_name(r: StopReason) -> String {
    match r {
        StopReason::Evaluations => "evaluations".into(),
        StopReason::Converged => "converged".into(),
        StopReason::Target => "target".into(),
        StopReason::Stalled => "stalled".into(),
        StopReason::Generations => "generations".into(),
        StopReason::Time => "time".into(),
        StopReason::Stagnation => "stagnation".into(),
        other => format!("{other:?}").to_lowercase(),
    }
}

/// Runs an optimize job: the problem read and its derivatives checked, then genoxide's method,
/// each evaluation recorded as it comes, then the optimum.
fn optimize(job: &Job, run: &mut Run, stop: &Stop) -> photonoxide::Result<()> {
    job::check(job)?;
    let mut problem = Problem::of(job)?;
    let checks = problem.check_gradients()?;
    let recorder = Recorder::begin(&problem, job, run, checks)?;
    let n = problem.variables().len();
    let from: Vec<f64> = problem.variables().iter().map(|v| v.from).collect();
    let span: Vec<f64> = problem.variables().iter().map(|v| v.to - v.from).collect();
    let u0 = Reals::from(
        (0..n)
            .map(|j| ((problem.start()[j] - from[j]) / span[j]).clamp(0.0, 1.0))
            .collect::<Vec<f64>>(),
    );
    let state = Mutex::new(State {
        run,
        recorder,
        failed: None,
    });
    let fitness = Fitness {
        problem: &problem,
        state: &state,
        stop,
        from,
        span,
        stage: Arc::new(AtomicUsize::new(0)),
    };
    let reason = drive(&problem, &fitness, u0, stop);
    let State {
        run,
        mut recorder,
        failed,
    } = state.into_inner().unwrap_or_else(|e| e.into_inner());
    if let Some(e) = failed {
        return Err(e);
    }
    let stopped = stop.reason().map(|r| match r {
        photonoxide::run::StopReason::Timeout => "timeout".to_owned(),
        photonoxide::run::StopReason::Requested => "requested".to_owned(),
    });
    let why = match (&stopped, reason) {
        (Some(s), _) => s.clone(),
        (None, Ok(r)) => reason_name(r),
        (None, Err(e)) => {
            return Err(photonoxide::Error::InvalidValue {
                what: "optimize",
                reason: format!("genoxide: {e}"),
            });
        }
    };
    recorder.front(run)?;
    recorder.finish(&problem, run, &why)?;
    let seconds = run.elapsed().as_secs_f64();
    run.record(&job::Event::Finished { stopped, seconds })
}

/// The stop conditions every run takes: the budget, and the run's Stop (its hard limit, or the
/// window's Stop) checked after each generation.
fn stops(problem: &Problem, stop: &Stop) -> genoxide::Stop {
    let stop = stop.clone();
    genoxide::Stop::evaluations(problem.evaluations() as u64)
        .or(genoxide::Stop::custom(move |_| stop.reason().is_some()))
}

/// Runs the problem's method; genoxide's reason it stopped.
fn drive(
    problem: &Problem,
    fitness: &Fitness<'_, '_>,
    u0: Reals,
    stop: &Stop,
) -> genoxide::Result<StopReason> {
    let n = u0.len();
    let real = Real::new(vec![0.0..=1.0; n])?;
    let seed = problem.seed();
    let population = problem.population();
    match problem.method().name {
        "lbfgsb" => {
            let a = Lbfgsb::builder(real)
                .initial_genome(u0)
                .keep_pairs(true)
                .minimize()
                .seed(seed)
                .build()?;
            run_continued(a, fitness, problem, stop)
        }
        "adam" => {
            let a = FirstOrder::builder(real)
                .step(first_order::Step::adam(problem.learning_rate()))
                .initial_genome(u0)
                .minimize()
                .seed(seed)
                .build()?;
            run_continued(a, fitness, problem, stop)
        }
        "mma" => {
            let a = Mma::builder(real)
                .initial_genome(u0)
                .minimize()
                .seed(seed)
                .build()?;
            run_continued(a, fitness, problem, stop)
        }
        "gcmma" => {
            let a = Mma::builder(real)
                .method(genoxide::prelude::mma::Method::Gcmma)
                .initial_genome(u0)
                .minimize()
                .seed(seed)
                .build()?;
            run_continued(a, fitness, problem, stop)
        }
        "nelder-mead" => {
            let a = NelderMead::builder(real)
                .initial_genome(u0)
                .minimize()
                .seed(seed)
                .build()?;
            run_continued(a, fitness, problem, stop)
        }
        "cmaes" => {
            let mut b = Cmaes::builder(real).initial_mean(u0).minimize().seed(seed);
            if let Some(p) = population {
                b = b.population_size(p);
            }
            run_continued(b.build()?, fitness, problem, stop)
        }
        "de" => {
            let a = De::builder(real)
                .population_size(population.unwrap_or(20))
                .minimize()
                .seed(seed)
                .build()?;
            run_single(a, fitness, problem, stop)
        }
        "pso" => {
            let a = Pso::builder(real)
                .population_size(population.unwrap_or(20))
                .minimize()
                .seed(seed)
                .build()?;
            run_single(a, fitness, problem, stop)
        }
        "nsga2" => {
            let a = Nsga2::builder(real, [Objective::Minimize; 2])
                .population_size(population.unwrap_or(40))
                .crossover(SimulatedBinaryCrossover::new(15.0)?)
                .mutate(PolynomialMutation::per_gene(1.0 / n as f64, 20.0)?)
                .initial_genomes([u0])
                .seed(seed)
                .build()?;
            run_multi(a, fitness, problem, stop)
        }
        "sms-emoa" => {
            let a = SmsEmoa::builder(real, [Objective::Minimize; 2])
                .population_size(population.unwrap_or(40))
                .crossover(SimulatedBinaryCrossover::new(15.0)?)
                .mutate(PolynomialMutation::per_gene(1.0 / n as f64, 20.0)?)
                .initial_genomes([u0])
                .seed(seed)
                .build()?;
            run_multi(a, fitness, problem, stop)
        }
        other => Err(genoxide::Error::InvalidSetting {
            setting: "method",
            reason: format!("no method {other}"),
        }),
    }
}

fn run_single<A: Algorithm<Genome = Reals>>(
    algorithm: A,
    fitness: &Fitness<'_, '_>,
    problem: &Problem,
    stop: &Stop,
) -> genoxide::Result<StopReason> {
    let outcome = genoxide::Engine::new(algorithm, Single(fitness))
        .stop_when(stops(problem, stop))
        .run()?;
    Ok(outcome.stop_reason())
}

/// Runs a method that can go on across stages: inside genoxide's `Continuation` when the task
/// has one, its state kept from one stage to the next as the task says; by itself otherwise.
fn run_continued<A: Continue<Genome = Reals> + 'static>(
    algorithm: A,
    fitness: &Fitness<'_, '_>,
    problem: &Problem,
    stop: &Stop,
) -> genoxide::Result<StopReason> {
    let Some(schedule) = problem.continuation() else {
        return run_single(algorithm, fitness, problem, stop);
    };
    let keep = match schedule.keep {
        job::Keep::Point => continuation::Keep::Point,
        _ => continuation::Keep::State,
    };
    let stage = Arc::clone(&fitness.stage);
    let mut builder = Continuation::builder(algorithm)
        .stages(schedule.values.len())
        .keep(keep)
        .on_stage(move |k, _| {
            stage.store(k, Ordering::Release);
            Ok(())
        });
    if let Some(g) = schedule.generations {
        builder = builder.generations(g);
    }
    run_single(builder.build()?, fitness, problem, stop)
}

fn run_multi<A: genoxide::multi::MultiObjectiveAlgorithm<2, Genome = Reals>>(
    algorithm: A,
    fitness: &Fitness<'_, '_>,
    problem: &Problem,
    stop: &Stop,
) -> genoxide::Result<StopReason> {
    let outcome = MultiEngine::new(algorithm, Multi(fitness))
        .stop_when(stops(problem, stop))
        .run()?;
    Ok(outcome.stop_reason())
}

#[cfg(test)]
mod tests;
