//! A design region's optimization: genoxide's MMA on the library's stages
//! (`photonoxide::job::Topology`), one stage at a time, each a new MMA from the last stage's best
//! design on the epigraph's constraints and the rules'. Every evaluation is recorded as it comes
//! (`TopologyRecorder`); MMA evaluates one point at a time, so the record is in its order, and
//! each evaluation's solves run side by side on rayon's threads with their results taken in
//! order: the record is the same, to the bit, on any number of threads.

use std::sync::Mutex;

use genoxide::prelude::{Constrained, Engine, Mma, Real, Reals};
use photonoxide::job::{self, Topology, TopologyRecorder};
use photonoxide::run::{Job, Run, Stop};

/// The record being written and the first error writing it.
struct State<'r> {
    run: &'r mut Run,
    recorder: TopologyRecorder,
    failed: Option<photonoxide::Error>,
}

fn genoxide_error(e: genoxide::Error) -> photonoxide::Error {
    photonoxide::Error::InvalidValue {
        what: "optimize",
        reason: format!("genoxide: {e}"),
    }
}

/// Runs a design region's job: each stage's MMA, then the last stage's best design, binary, and
/// [`job::Event::Finished`].
pub(super) fn run(job: &Job, run: &mut Run, stop: &Stop) -> photonoxide::Result<()> {
    job::check(job)?;
    let topology = Topology::of(job)?;
    let recorder = TopologyRecorder::begin(&topology, job, run)?;
    let n = topology.len();
    let mut rho = topology.start();
    let mut state = State {
        run,
        recorder,
        failed: None,
    };
    let mut last = None;
    for k in 0..topology.stages() {
        if stop.reason().is_some() {
            break;
        }
        let stage = topology.stage(k)?;
        state.recorder.stage(&stage, state.run)?;
        let (y0, point) = stage.start(&rho)?;
        state.recorder.record(&stage, state.run, &rho, &point)?;
        let shared = Mutex::new(state);
        let fitness = Constrained::differentiable(
            stage.constraints(),
            |x: &Reals, gradient: &mut [f64], g: &mut [f64], jacobian: &mut [f64]| {
                let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                let State {
                    run,
                    recorder,
                    failed,
                } = &mut *s;
                match stage.evaluate(x, gradient, g, jacobian) {
                    Ok(point) => {
                        if let Err(e) = recorder.record(&stage, run, &x[..n], &point) {
                            failed.get_or_insert(e);
                        }
                        point.t
                    }
                    Err(e) => {
                        if let Err(e) = recorder.failed(&stage, run, &e.to_string()) {
                            failed.get_or_insert(e);
                        }
                        f64::NAN
                    }
                }
            },
        );
        let mma = Mma::builder(Real::new(vec![0.0..=1.0; n + 1]).map_err(genoxide_error)?)
            .initial_genome(Reals::from(y0))
            .minimize()
            .build()
            .map_err(genoxide_error)?;
        let halt = stop.clone();
        Engine::new(mma, fitness)
            .stop_when(
                genoxide::Stop::evaluations(stage.evaluations())
                    .or(genoxide::Stop::custom(move |_| halt.reason().is_some())),
            )
            .run()
            .map_err(genoxide_error)?;
        state = shared.into_inner().unwrap_or_else(|e| e.into_inner());
        if let Some(e) = state.failed.take() {
            return Err(e);
        }
        state.recorder.stage_end(&stage, state.run)?;
        rho = stage.best();
        last = Some(stage);
    }
    let stopped = stop.reason().map(|r| match r {
        photonoxide::run::StopReason::Timeout => "timeout".to_owned(),
        photonoxide::run::StopReason::Requested => "requested".to_owned(),
    });
    if let Some(stage) = &last {
        let why = stopped.clone().unwrap_or_else(|| "evaluations".into());
        state.recorder.finish(stage, state.run, &why)?;
    }
    let seconds = state.run.elapsed().as_secs_f64();
    state.run.record(&job::Event::Finished { stopped, seconds })
}
