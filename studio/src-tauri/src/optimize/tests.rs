//! The optimize adapter's tests: genoxide's methods through the job, against closed forms; a
//! seeded run's record the same on any number of threads; the hard limit; the Pareto front; and
//! the built-in chip jobs that reach #244's circuit examples' closed forms.

use photonoxide::circuit::Fidelity;
use photonoxide::job::Event;

use super::*;

fn job(parameters: &str, task: &str) -> Job {
    job_of("directional coupler", parameters, task, 1.0)
}

fn job_of(kind: &str, parameters: &str, task: &str, minutes: f64) -> Job {
    Job::parse(&format!(
        "name = \"t\"\ntimeout_minutes = {minutes:?}\n\n[device]\nkind = \"{kind}\"\nfidelity = \"analytic\"\n{parameters}\n[task]\nkind = \"optimize\"\n{task}"
    ))
    .expect("a job")
}

/// The run's events.
fn events(job: &Job) -> Vec<Event> {
    lines(job, None)
        .iter()
        .map(|l| serde_json::from_str(l).expect("an event"))
        .collect()
}

/// The run's lines, on `threads` threads (rayon's pool otherwise), but its last (Finished, with
/// its time).
fn lines(job: &Job, threads: Option<usize>) -> Vec<String> {
    let mut run = Run::in_memory();
    let go = |run: &mut Run| execute(job, run, &Stop::new(job.timeout())).expect("a run");
    match threads {
        Some(t) => rayon::ThreadPoolBuilder::new()
            .num_threads(t)
            .build()
            .expect("a pool")
            .install(|| go(&mut run)),
        None => go(&mut run),
    }
    let mut l = run.lines().to_vec();
    assert!(l.last().is_some_and(|f| f.contains("\"finished\"")));
    l.pop();
    l
}

/// The optimum's variables and why it stopped.
fn optimum(events: &[Event]) -> (Vec<f64>, String) {
    events
        .iter()
        .find_map(|e| match e {
            Event::Optimum { x, stop, .. } => Some((x.clone(), stop.clone())),
            _ => None,
        })
        .expect("an optimum")
}

/// The coupler's 3 dB length, L_x/2, the closed form of its model.
fn three_db() -> f64 {
    use photonoxide::device::Device;
    let d = photonoxide::device::StripCoupler::new();
    let v = photonoxide::device::Values::defaults(&d);
    d.component(&v, Fidelity::Analytic)
        .unwrap()
        .figure("L_3dB")
        .unwrap()
        .value
}

const SPLIT: &str = "seed = 3\n\n[[task.variable]]\nparameter = \"length\"\nfrom = 2.0\nto = 30.0\n\n[[task.objective]]\nname = \"split\"\n";

#[test]
fn every_gradient_method_takes_a_coupler_to_its_3db_length() {
    for method in ["lbfgsb", "gcmma", "adam"] {
        let budget = match method {
            "lbfgsb" => 60,
            _ => 400,
        };
        let task = format!("method = \"{method}\"\nevaluations = {budget}\n{SPLIT}");
        let e = events(&job("[device.parameters]\nlength = \"8.0\"\n", &task));
        let (x, stop) = optimum(&e);
        let tolerance = match method {
            "lbfgsb" => 1e-6,
            "gcmma" => 1e-6,
            _ => 1e-3,
        };
        assert!(
            (x[0] - three_db()).abs() < tolerance,
            "{method}: {} against {} ({stop})",
            x[0],
            three_db()
        );
        let Some(Event::Optimization {
            variables, checks, ..
        }) = e.iter().find(|e| matches!(e, Event::Optimization { .. }))
        else {
            panic!("an optimization");
        };
        assert_eq!(variables[0].derivatives, "exact");
        assert!(checks.iter().all(|c| c.agrees));
    }
}

#[test]
fn every_value_method_comes_near_it_and_closer_with_a_larger_budget() {
    let l3 = three_db();
    for method in ["nelder-mead", "cmaes", "de", "pso"] {
        let miss = |budget: usize| {
            let task = format!("method = \"{method}\"\nevaluations = {budget}\n{SPLIT}");
            let (x, _) = optimum(&events(&job(
                "[device.parameters]\nlength = \"8.0\"\n",
                &task,
            )));
            (x[0] - l3).abs()
        };
        let (small, large) = (miss(30), miss(300));
        assert!(
            large < 1e-2,
            "{method}: {large} µm off with 300 evaluations"
        );
        assert!(
            large <= small,
            "{method}: {small} with 30, {large} with 300"
        );
    }
}

#[test]
fn mma_meets_a_constraint_at_its_bound() {
    // the shortest coupler that splits at least half across: its 3 dB length
    // (the split is sin² of the length's phase: under 1/2 below L_x/2, over it up to L_x)
    let task = "method = \"mma\"\nevaluations = 200\nseed = 1\n\n[[task.variable]]\nparameter = \"length\"\nfrom = 2.0\nto = 30.0\n\n[[task.objective]]\nexpression = \"length\"\n\n[[task.constraint]]\nexpression = \"T_o3_o1/(T_o3_o1 + T_o4_o1)\"\nat_least = 0.5\n";
    let e = events(&job("[device.parameters]\nlength = \"25.0\"\n", task));
    let (x, stop) = optimum(&e);
    assert!((x[0] - three_db()).abs() < 1e-5, "{} ({stop})", x[0]);
}

#[test]
fn a_ring_is_taken_to_critical_coupling() {
    // κ² = 1 − a², a² the round trip's power transmission: the gap that couples what is lost
    let task = "method = \"nelder-mead\"\nevaluations = 300\nseed = 1\n\n[[task.variable]]\nparameter = \"gap\"\nfrom = 0.1\nto = 0.6\n\n[[task.objective]]\nname = \"critical\"\n";
    let job = job_of(
        "all-pass ring",
        "[device.parameters]\nloss = \"20\"\n",
        task,
        1.0,
    );
    let e = events(&job);
    let (x, stop) = optimum(&e);
    let spec = photonoxide::job::design_of(&job, &x).unwrap();
    let (device, design) = spec.build().unwrap();
    let built = device
        .component(design.values(), Fidelity::Analytic)
        .unwrap();
    let kappa2 = built.figure("kappa2").unwrap().value;
    let length = built.figure("L").unwrap().value;
    let a2 = 10f64.powf(-(20.0 * length * 1e-4) / 10.0);
    assert!(
        (kappa2 - (1.0 - a2)).abs() < 1e-6,
        "κ² {kappa2} against {} ({stop})",
        1.0 - a2
    );
    assert!(built.figure("extinction").unwrap().value > 40.0);
}

#[test]
fn a_seeded_run_is_the_same_bits_on_one_and_four_threads() {
    for method in ["cmaes", "de", "lbfgsb"] {
        // the gap differenced side by side, the length exact
        let task = format!(
            "method = \"{method}\"\nevaluations = 120\nseed = 9\nfrom_um = 1.53\nto_um = 1.57\npoints = 5\n\n[[task.variable]]\nparameter = \"length\"\nfrom = 2.0\nto = 30.0\n\n[[task.variable]]\nparameter = \"gap\"\nfrom = 0.15\nto = 0.3\n\n[[task.objective]]\nname = \"split\"\n"
        );
        let j = job("", &task);
        assert_eq!(lines(&j, Some(1)), lines(&j, Some(4)), "{method}");
    }
    let pareto = job(
        "",
        &PARETO.replace("evaluations = 400", "evaluations = 120"),
    );
    assert_eq!(lines(&pareto, Some(1)), lines(&pareto, Some(4)));
}

const PARETO: &str = "method = \"nsga2\"\nevaluations = 400\nseed = 2\npopulation = 20\nfrom_um = 1.5\nto_um = 1.6\npoints = 5\n\n[[task.variable]]\nparameter = \"length\"\nfrom = 2.0\nto = 30.0\n\n[[task.variable]]\nparameter = \"gap\"\nfrom = 0.15\nto = 0.3\n\n[[task.objective]]\nname = \"split\"\n\n[[task.objective]]\nexpression = \"length\"\n";

#[test]
fn two_objectives_grow_a_front_whose_hypervolume_never_falls() {
    let e = events(&job("", PARETO));
    let fronts: Vec<(f64, usize)> = e
        .iter()
        .filter_map(|e| match e {
            Event::Front {
                hypervolume,
                points,
                ..
            } => Some((*hypervolume, points.len())),
            _ => None,
        })
        .collect();
    assert!(fronts.len() > 3, "{} fronts", fronts.len());
    for w in fronts.windows(2) {
        assert!(w[1].0 >= w[0].0, "{fronts:?}");
    }
    assert!(fronts.last().unwrap().1 > 5);
    let Some(Event::Optimum { front, index, .. }) = e.last() else {
        panic!("an optimum last");
    };
    assert!(index.is_none());
    assert_eq!(front.len(), fronts.last().unwrap().1);
}

#[test]
fn the_hard_limit_ends_a_run_cleanly() {
    // 30 ms for a budget no run reaches in that time
    let task = format!("method = \"de\"\nevaluations = 1000000\npopulation = 50\n{SPLIT}").replace(
        "seed = 3\n",
        "seed = 3\nfrom_um = 1.5\nto_um = 1.6\npoints = 101\n",
    );
    let j = job_of("directional coupler", "", &task, 0.0005);
    let mut run = Run::in_memory();
    execute(&j, &mut run, &Stop::new(j.timeout())).expect("a clean end");
    let events: Vec<Event> = run
        .lines()
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let (_, stop) = optimum(&events);
    assert_eq!(stop, "timeout");
    assert!(matches!(
        events.last(),
        Some(Event::Finished { stopped: Some(s), .. }) if s == "timeout"
    ));
}

/// The built-in job `file` (jobs/), as the program reads it.
fn built_in(file: &str) -> Job {
    let text = crate::examples::jobs()
        .into_iter()
        .find(|j| j.file == file)
        .expect("a built-in job")
        .text;
    Job::parse(&text).expect("a job")
}

/// A chip job's optimum: its variables, after checking its record is the same bits on 1 and 4
/// threads and when read back from its run's directory.
fn chip_optimum(file: &str) -> Vec<f64> {
    let job = built_in(file);
    let one = lines(&job, Some(1));
    assert_eq!(one, lines(&job, Some(4)), "{file}: 1 and 4 threads");
    let root = std::env::temp_dir().join(format!(
        "photonoxide-{}-{}",
        file.trim_end_matches(".toml"),
        std::process::id()
    ));
    let mut run = Run::create(&root, &job).expect("a run directory");
    execute(&job, &mut run, &Stop::new(job.timeout())).expect("a run");
    let back: Vec<Event> = photonoxide::run::replay(run.dir()).expect("its record");
    drop(run);
    let _ = std::fs::remove_dir_all(&root);
    let replayed: Vec<String> = back[..back.len() - 1]
        .iter()
        .map(|e| serde_json::to_string(e).expect("an event"))
        .collect();
    assert_eq!(replayed, one, "{file}: its record read back");
    let e: Vec<Event> = one
        .iter()
        .map(|l| serde_json::from_str(l).expect("an event"))
        .collect();
    let Some(Event::Optimum {
        x,
        stop,
        evaluations,
        ..
    }) = e.last()
    else {
        panic!("{file}: an optimum last");
    };
    eprintln!("{file}: {x:?}, {evaluations} evaluations ({stop})");
    x.clone()
}

/// The 500 × 220 nm strip's analytic model's dispersion at 1.55 µm.
fn strip() -> photonoxide::circuit::components::Dispersion {
    let guide = photonoxide::device::StripWaveguide::new();
    let values = photonoxide::device::Values::defaults(&guide);
    guide.dispersion(&values, Fidelity::Analytic).unwrap().0
}

#[test]
fn the_chip_splitter_reaches_clements_closed_form() {
    // the bar port carries sin²(θ/2): 0.3 at θ = 2 arcsin √0.3
    let x = chip_optimum("chip-splitter.toml");
    let theta = 2.0 * 0.3f64.sqrt().asin();
    assert!((x[0] - theta).abs() < 1e-6, "θ {} against {theta}", x[0]);
}

#[test]
fn the_chip_ring_reaches_critical_coupling_on_resonance() {
    let n = strip().effective_index;
    let x = chip_optimum("chip-ring-critical.toml");
    // the resonance nearest the start (R = 9.985 µm), R = m λ/(2π n_eff), and κ² = 1 − a² with
    // a = 10^(−αL/20) (Bogaerts et al. 2011, Eqs. 3 and 2)
    let tau = std::f64::consts::TAU;
    let m = (n * tau * 9.985 / 1.55).round();
    assert_eq!(m, 99.0);
    let radius = m * 1.55 / (tau * n);
    let a = 10f64.powf(-3.0 * 1e-4 * tau * radius / 20.0);
    let kappa2 = 1.0 - a * a;
    assert!((x[0] - kappa2).abs() < 1e-9, "κ² {} against {kappa2}", x[0]);
    assert!((x[1] - radius).abs() < 1e-9, "R {} against {radius}", x[1]);
}

#[test]
fn the_chip_fit_returns_the_ring_its_spectra_were_made_from() {
    // the objective writes the strip's n_eff and n_g out: they must be its model's
    let d = strip();
    assert_eq!((d.effective_index, d.group_index), (2.44506, 4.172901));
    let x = chip_optimum("chip-fit.toml");
    // κ₁², κ₂², the loss (dB/cm), the radius (µm)
    let truth = [0.08, 0.05, 20.0, 10.0];
    let tolerance = [1e-8, 1e-8, 1e-5, 1e-9];
    for k in 0..4 {
        assert!(
            (x[k] - truth[k]).abs() < tolerance[k],
            "variable {k}: {} against {}",
            x[k],
            truth[k]
        );
    }
}

/// `photonoxide new optimize`'s job runs as it is, and splits the coupler evenly.
#[test]
fn the_optimize_template_runs_to_an_even_split() {
    let t = job::template("optimize").expect("the template");
    let j = Job::parse_as(t.text, photonoxide::run::Format::Yaml).expect("a job");
    let e = events(&j);
    let (x, stop) = optimum(&e);
    // even at the band's worst wavelength, not at 1.55 um: near L_x/2 (18.573 against 18.576 um
    // here), not on it
    let off = (x[0] - three_db()).abs() / three_db();
    assert!(off < 1e-3, "{} against {} ({stop})", x[0], three_db());
}
