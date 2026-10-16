//! What the validation report checks a device's optimization on ([`super::optimize`]): every
//! exact derivative against fourth-order differences, a coupler's split at its 3 dB length in
//! closed form, an evaluation's bits on any number of threads, and the hypervolume of a front
//! computed by hand. Runs of genoxide's optimizers on these problems are checked where genoxide
//! is: the `photonoxide` program's tests and the `optimize_` examples.

use super::optimize::{Problem, hypervolume};
use crate::circuit::Fidelity;
use crate::device::{Device, Values, library};
use crate::run::Job;
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid optimization";

/// The cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "optimize/derivatives",
            title: r"Devices' objectives and constraints: every exact derivative (each device at its cheapest model, each declared objective with every declared constraint, in each parameter its component differentiates, at 3 wavelengths from 1.53 to 1.57 µm) against fourth-order central differences of the same quantity at each wavelength, the largest relative difference (shown)",
            tier: Tier::Analytic,
            source: r"the derivatives' definition: $(f(x-2h) - 8f(x-h) + 8f(x+h) - f(x+2h))/12h$ with $h = 5\times10^{-6} \max(\lvert x \rvert, 1)$, its error $O(h^4)$; the exact ones by forward mode through the expression from the component's own derivatives (the circuit adjoint for an MZI)",
            run: derivatives,
        },
        Case {
            id: "optimize/coupler-3db",
            title: r"Devices' objectives: a directional coupler's split ratio $T_{31}/(T_{31} + T_{41})$ at its 3 dB length $L_x/2$ (500 x 220 nm strips 200 nm apart, analytic), at 1.55 µm, against 1/2 (difference shown) and its cost $(s - 1/2)^2$ there",
            tier: Tier::Analytic,
            source: r"coupled-mode theory's closed form: the power across is $\sin^2(\pi \Delta n L/\lambda)$ and through $\cos^2$, times the same loss, so the split is $\sin^2(\pi/4) = 1/2$ at $L = \lambda/(4\Delta n) = L_x/2$ whatever the loss",
            run: coupler_3db,
        },
        Case {
            id: "optimize/threads",
            title: r"Devices' optimizations: a coupler's evaluation with its gradient in three variables (its length exact, its gap and width by central differences side by side on rayon's threads, 5 wavelengths) on 1 and on 20 threads: the numbers whose bits differ (count shown)",
            tier: Tier::Analytic,
            source: "the same bits on any number of threads (AGENTS.md): each difference is computed on its own and gathered in the variables' order",
            run: threads,
        },
        Case {
            id: "optimize/hypervolume",
            title: r"Pareto fronts: the hypervolume of the costs (1, 3), (2, 2), (3, 1), a dominated (3.5, 3.5) and (5, 0) beyond the reference, up to (4, 4) (shown)",
            tier: Tier::Analytic,
            source: r"computed by hand: the union of the rectangles from each point to the reference, $3 \times 1 + 2 \times 1 + 1 \times 1 = 6$",
            run: front,
        },
    ]
}

fn job(kind: &str, fidelity: &str, parameters: &str, task: &str) -> Job {
    Job::parse(&format!(
        "name = \"check\"\ntimeout_minutes = 1\n\n[device]\nkind = \"{kind}\"\nfidelity = \"{fidelity}\"\n{parameters}\n[task]\nkind = \"optimize\"\n{task}"
    ))
    .expect(OK)
}

fn derivatives() -> Outcome {
    let mut largest: f64 = 0.0;
    let mut checked = 0;
    for device in library() {
        let values = Values::defaults(device.as_ref());
        let cheapest = device.fidelities()[0].fidelity;
        let Ok(built) = device.component(&values, cheapest) else {
            continue;
        };
        let fidelity = super::device::fidelity_name(cheapest);
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
            let Ok(mut problem) = Problem::of(&job(device.kind(), fidelity, "", &task)) else {
                return failed();
            };
            let Ok(checks) = problem.check_gradients() else {
                return failed();
            };
            for c in checks {
                checked += 1;
                let size = c.exact.abs().max(c.differences.abs());
                if size > 1e-9 {
                    largest = largest.max((c.exact - c.differences).abs() / size);
                }
            }
        }
    }
    if checked < 20 {
        return failed();
    }
    Outcome {
        measured: largest,
        expected: 0.0,
        tolerance: 1e-6,
        error: largest,
    }
}

fn coupler_3db() -> Outcome {
    let device = crate::device::StripCoupler::new();
    let values = Values::defaults(&device);
    let built = device.component(&values, Fidelity::Analytic).expect(OK);
    let half = built.figure("L_3dB").expect(OK).value;
    let task = "method = \"lbfgsb\"\nseed = 1\nevaluations = 10\n\n[[task.variable]]\nparameter = \"length\"\nfrom = 1.0\nto = 40.0\n\n[[task.objective]]\nname = \"split\"\n";
    let problem = Problem::of(&job("directional coupler", "analytic", "", task)).expect(OK);
    let e = problem.evaluate(&[half], false);
    let miss = (e.values[0] - 0.5).abs();
    let cost = e.costs[0];
    Outcome {
        measured: miss,
        expected: 0.0,
        tolerance: 1e-14,
        error: miss.max(cost.abs()),
    }
}

fn threads() -> Outcome {
    let task = "method = \"mma\"\nseed = 1\nevaluations = 10\nfrom_um = 1.53\nto_um = 1.57\npoints = 5\n\n[[task.variable]]\nparameter = \"length\"\nfrom = 2.0\nto = 30.0\n\n[[task.variable]]\nparameter = \"gap\"\nfrom = 0.15\nto = 0.3\n\n[[task.variable]]\nparameter = \"width\"\nfrom = 0.45\nto = 0.55\n\n[[task.objective]]\nname = \"flatness\"\n\n[[task.constraint]]\nname = \"excess_loss\"\nat_most = 0.05\n";
    let mut problem = Problem::of(&job("directional coupler", "analytic", "", task)).expect(OK);
    problem.check_gradients().expect(OK);
    let on = |threads: usize| -> Vec<u64> {
        let e = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("a pool")
            .install(|| problem.evaluate(&[7.0, 0.2, 0.5], true));
        e.costs
            .iter()
            .chain(&e.constraints)
            .chain(e.gradients.iter().flatten())
            .chain(e.jacobian.iter().flatten())
            .map(|v| v.to_bits())
            .collect()
    };
    let (one, twenty) = (on(1), on(20));
    if one.len() != 8 {
        return failed();
    }
    let differ =
        one.len().abs_diff(twenty.len()) + one.iter().zip(&twenty).filter(|(a, b)| a != b).count();
    Outcome {
        measured: differ as f64,
        expected: 0.0,
        tolerance: 0.0,
        error: differ as f64,
    }
}

fn front() -> Outcome {
    let points = [[1.0, 3.0], [2.0, 2.0], [3.0, 1.0], [3.5, 3.5], [5.0, 0.0]];
    let v = hypervolume(&points, [4.0, 4.0]);
    Outcome {
        measured: v,
        expected: 6.0,
        tolerance: 0.0,
        error: (v - 6.0).abs(),
    }
}

fn failed() -> Outcome {
    Outcome {
        measured: f64::NAN,
        expected: 0.0,
        tolerance: 0.0,
        error: f64::INFINITY,
    }
}
