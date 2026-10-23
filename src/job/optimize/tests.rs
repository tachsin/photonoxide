//! The optimize task's tests: what it refuses, its derivatives against differences, the
//! reductions, the hypervolume, the same bits on any number of threads, and genoxide's
//! L-BFGS-B (a dev-dependency) taking a coupler to its 3 dB length in closed form.

use genoxide::prelude::{Differentiable, Engine, Lbfgsb, Real, Reals, Stop as GStop};

use super::*;
use crate::device::library;
use crate::run::Stop;

fn job_of(kind: &str, fidelity: &str, task: &str) -> Job {
    job_with(kind, fidelity, "", task)
}

fn job_with(kind: &str, fidelity: &str, parameters: &str, task: &str) -> Job {
    Job::parse(&format!(
        "name = \"t\"\ntimeout_minutes = 1\n\n[device]\nkind = \"{kind}\"\nfidelity = \"{fidelity}\"\n{parameters}\n[task]\nkind = \"optimize\"\n{task}"
    ))
    .expect("a job")
}

const COUPLER: &str = "method = \"lbfgsb\"\nseed = 1\nevaluations = 100\n\n[[task.variable]]\nparameter = \"length\"\nfrom = 2.0\nto = 30.0\n\n[[task.objective]]\nname = \"split\"\n";

/// Runs genoxide's L-BFGS-B on `problem` in the variables scaled to their ranges, recording
/// nothing: the best point and the evaluations.
fn lbfgsb(problem: &Problem) -> (Vec<f64>, u64) {
    let vars = problem.variables();
    let n = vars.len();
    let span: Vec<f64> = vars.iter().map(|v| v.to - v.from).collect();
    let x_of = |u: &Reals| -> Vec<f64> {
        (0..n)
            .map(|j| (vars[j].from + u[j] * span[j]).clamp(vars[j].from, vars[j].to))
            .collect()
    };
    let u0: Vec<f64> = (0..n)
        .map(|j| (problem.start()[j] - vars[j].from) / span[j])
        .collect();
    let fitness = |u: &Reals, g: &mut [f64]| {
        let e = problem.evaluate(&x_of(u), true);
        for j in 0..n {
            g[j] = e.gradients[0][j] * span[j];
        }
        e.costs[0]
    };
    let algorithm = Lbfgsb::builder(Real::new(vec![0.0..=1.0; n]).unwrap())
        .initial_genome(Reals::from(u0))
        .gradient_tolerance(1e-12)
        .function_tolerance(0.0)
        .minimize()
        .build()
        .unwrap();
    let outcome = Engine::new(algorithm, Differentiable(&fitness))
        .stop_when(GStop::target(1e-24).or(GStop::evaluations(problem.evaluations() as u64)))
        .run()
        .unwrap();
    (x_of(outcome.best_genome()), outcome.evaluations())
}

#[test]
fn lbfgsb_takes_a_coupler_to_its_3db_length_in_closed_form() {
    let job = job_with(
        "directional coupler",
        "analytic",
        "[device.parameters]\nlength = \"8.0\"\n",
        COUPLER,
    );
    let mut problem = Problem::of(&job).unwrap();
    let checks = problem.check_gradients().unwrap();
    assert_eq!(problem.derivatives(), ["exact"]);
    assert!(checks.iter().all(|c| c.agrees), "{checks:?}");
    let (x, evaluations) = lbfgsb(&problem);
    // L_3dB = L_x/2 = λ/(4Δn): the closed form of the coupled-mode theory the model is
    let spec = problem.design_at(&x).unwrap();
    let (device, design) = spec.build().unwrap();
    let built = device
        .component(design.values(), crate::circuit::Fidelity::Analytic)
        .unwrap();
    let l3 = built.figure("L_3dB").unwrap().value;
    assert!((x[0] - l3).abs() < 1e-6, "{} against {l3}", x[0]);
    assert!(evaluations < 40, "{evaluations} evaluations");
}

#[test]
fn every_exact_derivative_agrees_with_fourth_order_differences() {
    let mut checked = 0;
    for device in library() {
        let values = crate::device::Values::defaults(device.as_ref());
        let cheapest = device.fidelities()[0].fidelity;
        let Ok(built) = device.component(&values, cheapest) else {
            continue;
        };
        let fidelity = crate::job::device::fidelity_name(cheapest);
        // the parameters the component has exact derivatives in, 10 % about their defaults
        let mut variables = String::new();
        for (k, p) in device.parameters().iter().enumerate() {
            if built.derivatives[k].is_none() {
                continue;
            }
            let (lo, hi) = if p.default == 0.0 {
                (p.min.max(-1.0), p.max.min(1.0))
            } else {
                ((p.default * 0.9).max(p.min), (p.default * 1.1).min(p.max))
            };
            variables += &format!(
                "\n[[task.variable]]\nparameter = \"{}\"\nfrom = {lo:?}\nto = {hi:?}\n",
                p.name
            );
        }
        if variables.is_empty() {
            continue;
        }
        for o in device.objectives() {
            let mut task = format!(
                "method = \"mma\"\nseed = 1\nevaluations = 10\nfrom_um = 1.53\nto_um = 1.57\npoints = 3\n{variables}\n[[task.objective]]\nname = \"{}\"\n",
                o.symbol
            );
            for c in device.constraints() {
                task += &format!("\n[[task.constraint]]\nname = \"{}\"\n", c.symbol);
            }
            let job = job_of(device.kind(), fidelity, &task);
            let mut problem =
                Problem::of(&job).unwrap_or_else(|e| panic!("{} {}: {e}", device.kind(), o.symbol));
            let checks = problem
                .check_gradients()
                .unwrap_or_else(|e| panic!("{} {}: {e}", device.kind(), o.symbol));
            for c in &checks {
                assert!(c.agrees, "{}: {c:?}", device.kind());
            }
            checked += checks.len();
        }
    }
    assert!(checked > 20, "{checked} checks");
}

#[test]
fn the_reductions_weights_are_their_derivatives() {
    let names = {
        let d = crate::device::StripWaveguide::new();
        let v = crate::device::Values::defaults(&d);
        Names::of(&d, &v, None).unwrap()
    };
    let samples = [0.3, 0.7, 0.2, 0.55];
    for (goal, limit) in [
        (Some((Goal::Minimize, 0.0)), None),
        (Some((Goal::Maximize, 0.0)), None),
        (Some((Goal::Target, 0.4)), None),
        (None, Some((Limit::AtMost, 0.5))),
        (None, Some((Limit::AtLeast, 0.25))),
    ] {
        for r in [Reduce::Worst, Reduce::Average, Reduce::Spread] {
            let q = Quantity {
                name: String::new(),
                symbol: String::new(),
                unit: String::new(),
                text: "lambda".into(),
                expression: names.parse("q", "lambda").unwrap(),
                goal,
                limit,
                reduce: r,
                figures: Vec::new(),
                instance: None,
            };
            let base = reduce(&q, &samples);
            for k in 0..samples.len() {
                let h = 1e-7;
                let mut up = samples;
                up[k] += h;
                let mut down = samples;
                down[k] -= h;
                let fd = (reduce(&q, &up).result - reduce(&q, &down).result) / (2.0 * h);
                assert!(
                    (fd - base.weights[k]).abs() < 1e-6,
                    "{goal:?} {limit:?} {r:?} at {k}: {fd} against {}",
                    base.weights[k]
                );
            }
        }
    }
}

#[test]
fn the_hypervolume_of_a_hand_computed_front() {
    // three points against (4, 4): 3×1 + 2×1 + 1×1... by hand: (1,3) gives 3×1, (2,2) adds 2×1,
    // (3,1) adds 1×1: 6
    let front = [[2.0, 2.0], [1.0, 3.0], [3.0, 1.0], [3.5, 3.5]];
    assert_eq!(hypervolume(&front, [4.0, 4.0]), 6.0);
    // a point beyond the reference adds nothing; a dominated one nothing
    assert_eq!(hypervolume(&[[5.0, 0.0], [1.0, 3.0]], [4.0, 4.0]), 3.0);
    assert_eq!(hypervolume(&[], [4.0, 4.0]), 0.0);
}

#[test]
fn an_evaluation_is_the_same_bits_on_any_number_of_threads() {
    // the width builds another component: its derivative is differenced, on rayon's threads
    let task = "method = \"mma\"\nseed = 1\nevaluations = 10\nfrom_um = 1.53\nto_um = 1.57\npoints = 5\n\n[[task.variable]]\nparameter = \"length\"\nfrom = 2.0\nto = 30.0\n\n[[task.variable]]\nparameter = \"gap\"\nfrom = 0.15\nto = 0.3\n\n[[task.variable]]\nparameter = \"width\"\nfrom = 0.45\nto = 0.55\n\n[[task.objective]]\nname = \"flatness\"\n\n[[task.constraint]]\nname = \"excess_loss\"\nat_most = 0.05\n";
    let job = job_of("directional coupler", "analytic", task);
    let mut problem = Problem::of(&job).unwrap();
    problem.check_gradients().unwrap();
    assert_eq!(
        problem.derivatives(),
        ["exact", "differences", "differences"]
    );
    let on = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| problem.evaluate(&[7.0, 0.2, 0.5], true))
    };
    let (one, many) = (on(1), on(20));
    let bits = |e: &Evaluation| -> Vec<u64> {
        e.costs
            .iter()
            .chain(&e.constraints)
            .chain(e.gradients.iter().flatten())
            .chain(e.jacobian.iter().flatten())
            .map(|v| v.to_bits())
            .collect()
    };
    assert_eq!(bits(&one), bits(&many));
    assert_eq!(one.gradients[0].len(), 3);
    assert!(one.gradients[0].iter().all(|g| g.is_finite()));
}

#[test]
fn what_an_optimize_task_refuses() {
    let refused = |kind: &str, task: &str, says: &str| {
        let e = crate::job::check(&job_of(kind, "analytic", task)).unwrap_err();
        assert!(e.to_string().contains(says), "{e} lacks {says}");
    };
    let dc = "directional coupler";
    refused(dc, &COUPLER.replace("lbfgsb", "simplex"), "genoxide's");
    refused(dc, &COUPLER.replace("seed = 1\n", ""), "seed");
    refused(dc, &COUPLER.replace("evaluations = 100\n", ""), "budget");
    refused(dc, &COUPLER.replace("to = 30.0", "to = 2.0"), "bounds");
    refused(dc, &COUPLER.replace("from = 2.0", "from = 20.0"), "outside");
    refused(
        dc,
        &COUPLER.replace("\"length\"", "\"lenght\""),
        "no parameter lenght",
    );
    refused(
        dc,
        &COUPLER.replace("\"split\"", "\"splt\""),
        "no objective \"splt\"",
    );
    refused(
        dc,
        &format!("{COUPLER}\n[[task.constraint]]\nname = \"gap\"\n"),
        "takes no constraints",
    );
    refused(
        dc,
        &format!("{COUPLER}\n[[task.objective]]\nname = \"flatness\"\n"),
        "Pareto",
    );
    refused(
        dc,
        &COUPLER.replace("name = \"split\"", "expression = \"T_o9_o1\""),
        "T_o3_o1",
    );
    refused(
        dc,
        &COUPLER.replace("name = \"split\"", "expression = \"gap - 100 nm\""),
        "objective 1",
    );
    refused(
        dc,
        &COUPLER.replace(
            "name = \"split\"",
            "name = \"split\"\ngoal = \"minimize\"\ntarget = 0.5",
        ),
        "takes no target",
    );
    // the hard limit
    let text = format!(
        "name = \"t\"\n\n[device]\nkind = \"{dc}\"\nfidelity = \"analytic\"\n\n[task]\nkind = \"optimize\"\n{COUPLER}"
    );
    let e = crate::job::check(&Job::parse(&text).unwrap()).unwrap_err();
    assert!(e.to_string().contains("hard limit"), "{e}");
    // a job the library checks but leaves to the photonoxide program
    let job = job_of(dc, "analytic", COUPLER);
    crate::job::check(&job).unwrap();
    assert!(optimizes(&job));
    let e = crate::job::execute(&job, &mut Run::in_memory(), &Stop::new(None)).unwrap_err();
    assert!(e.to_string().contains("photonoxide run"), "{e}");
}

/// A coupler's split through a projection sharpened by a continuation over `beta`.
const PROJECTED: &str = "method = \"adam\"\nseed = 1\nevaluations = 100\n\n[task.continuation]\nparameter = \"beta\"\nvalues = [1.0, 8.0, 64.0]\n\n[[task.variable]]\nparameter = \"length\"\nfrom = 2.0\nto = 30.0\n\n[[task.objective]]\nexpression = \"tanh(beta*(T_o3_o1/(T_o3_o1 + T_o4_o1) - 0.5))/beta\"\ngoal = \"target\"\ntarget = 0.0\n";

#[test]
fn a_continuation_sets_its_parameter_stage_by_stage() {
    let dc = "directional coupler";
    let p = Problem::of(&job_of(dc, "analytic", PROJECTED)).unwrap();
    let s = p.continuation().expect("a continuation");
    assert_eq!(
        (s.parameter.as_str(), s.keep, s.generations),
        ("beta", Keep::State, None)
    );
    assert_eq!(p.stage(), 0);
    // the same split behind every stage's value: tanh(β(s − 1/2))/β
    // (near the 3 dB length, where tanh(64(s − 1/2)) is still away from ±1)
    let x = [18.0];
    let mut split = Vec::new();
    for (k, beta) in [1.0, 8.0, 64.0].into_iter().enumerate() {
        p.set_stage(k).unwrap();
        assert_eq!(p.stage_value(), Some(beta));
        let v = p.evaluate(&x, false).values[0];
        split.push((v * beta).atanh() / beta);
    }
    assert!(split[0].abs() > 1e-3, "{split:?}");
    assert!(
        (split[1] - split[0]).abs() < 1e-10 && (split[2] - split[0]).abs() < 1e-10,
        "{split:?}"
    );
    assert!(p.set_stage(3).is_err());
    // a parameter of the device's is written into the design at each stage
    let gap = PROJECTED
        .replace("\"beta\"", "\"gap\"")
        .replace("[1.0, 8.0, 64.0]", "[0.3, 0.25]")
        .replace("beta", "1");
    let p = Problem::of(&job_of(dc, "analytic", &gap)).unwrap();
    let gap_at = |p: &Problem| {
        let spec = p.design_at(&[12.0]).unwrap();
        spec.parameters
            .iter()
            .find(|(n, _)| n == "gap")
            .unwrap()
            .1
            .clone()
    };
    assert_eq!(gap_at(&p), "0.3");
    p.set_stage(1).unwrap();
    assert_eq!(gap_at(&p), "0.25");
    // and what it refuses
    let refused = |task: &str, says: &str| {
        let e = crate::job::check(&job_of(dc, "analytic", task)).unwrap_err();
        assert!(e.to_string().contains(says), "{e} lacks {says}");
    };
    refused(&PROJECTED.replace("\"adam\"", "\"de\""), "doesn't go on");
    refused(
        &PROJECTED.replace("[1.0, 8.0, 64.0]", "[1.0]"),
        "2 to 64 stages",
    );
    refused(
        &PROJECTED.replace("\"beta\"", "\"length\""),
        "is a variable",
    );
    refused(
        &PROJECTED.replace("\"beta\"", "\"lambda\""),
        "an output's name",
    );
    refused(
        &PROJECTED.replace("values =", "keep = \"all\"\nvalues ="),
        "\"state\" (the default) or \"point\"",
    );
    refused(
        &PROJECTED.replace("values =", "generations = 0\nvalues ="),
        "at least 1 generation",
    );
    refused(&gap.replace("[0.3, 0.25]", "[0.3, 1e-6]"), "range");
}

#[test]
fn a_job_written_from_a_design_reads_back_the_same() {
    let spec = Spec::new("all-pass ring");
    let optimization = Optimization::new("nsga2", 7, 400)
        .population(20)
        .variable(Variable::new("gap", 0.1, 0.4))
        .variable(Variable::new("radius", 5.0, 20.0))
        .objective(Aim::declared("critical"))
        .objective(Aim::expression("radius", Goal::Minimize))
        .constraint(Bound::declared("FSR").limited(Limit::AtLeast, 8.0))
        .corners(vec![Corner::new("wide", &[("width", 0.01)])]);
    let job = optimize_from_device("ring-pareto", &spec, &Compute::model(), &optimization).unwrap();
    let problem = Problem::of(&job).unwrap();
    assert_eq!(problem.objectives(), 2);
    assert_eq!(problem.constraints(), 1);
    assert_eq!(problem.population(), Some(20));
    assert_eq!(problem.start(), [0.2, 10.0]);
    let again = optimize_from_device("ring-pareto", &spec, &Compute::model(), &optimization)
        .unwrap()
        .text()
        .to_owned();
    assert_eq!(job.text(), again);
    // the design at a point, as Apply takes it
    let at = problem.design_at(&[0.25, 12.0]).unwrap();
    let (_, design) = at.build().unwrap();
    assert_eq!(design.values().get("gap"), Some(0.25));
    assert_eq!(design.values().get("radius"), Some(12.0));
    // two corners' worth of solves: the nominal design and the wide one
    let e = problem.evaluate(&[0.25, 12.0], false);
    assert_eq!(e.solves, 2);
    assert!(e.error.is_none(), "{:?}", e.error);
}

#[test]
fn a_record_names_the_best_and_the_front() {
    let job = job_of("directional coupler", "analytic", COUPLER);
    let mut problem = Problem::of(&job).unwrap();
    let checks = problem.check_gradients().unwrap();
    let mut run = Run::in_memory();
    let mut recorder = Recorder::begin(&problem, &job, &mut run, checks).unwrap();
    for x in [10.0, 25.0, 18.0] {
        let e = problem.evaluate(&[x], true);
        recorder.record(&mut run, &[x], &e).unwrap();
    }
    // a point the model refuses (a gap outside the analytic fit) is recorded, and reads back
    let mut wide = Problem::of(&job_of(
        "directional coupler",
        "analytic",
        &COUPLER.replace(
            "\"length\"\nfrom = 2.0\nto = 30.0",
            "\"gap\"\nfrom = 0.1\nto = 1.5",
        ),
    ))
    .unwrap();
    wide.check_gradients().unwrap();
    let refused = wide.evaluate(&[1.2], true);
    assert!(refused.error.is_some());
    recorder.record(&mut run, &[1.2], &refused).unwrap();
    recorder.finish(&problem, &mut run, "evaluations").unwrap();
    let events: Vec<Event> = run
        .lines()
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert!(matches!(events[0], Event::Started { .. }));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Optimization { .. }))
    );
    let Some(Event::Optimum {
        index, parameters, ..
    }) = events.last()
    else {
        panic!("an optimum last");
    };
    // 18 µm is nearest the 3 dB length (about 18.6 µm)
    assert_eq!(*index, Some(3));
    assert!(parameters.iter().any(|(n, e)| n == "length" && e == "18.0"));
}
