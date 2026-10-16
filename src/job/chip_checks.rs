//! What the validation report checks chips of devices on ([`super::chip`]): an MZI of devices
//! against its closed form, the chip's exact derivatives against fourth-order differences and
//! against the circuit adjoint of the same objective, a study's points against their own jobs, and
//! an evaluation's bits on any number of threads.

use super::chip::{ChipDesign, ChipInstance, spectrum, study_from_chip};
use super::optimize::{Aim, Bound, Optimization, Problem, Variable, optimize_from_chip};
use super::{Event, SweepAxis};
use crate::circuit::components::mzi_closed_form;
use crate::circuit::{SMatrix, objective};
use crate::device::{Goal, Limit, Spec};
use crate::run::{Run, Stop, replay};
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

/// The cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "chip/mzi-closed-form",
            title: r"Chips of devices: an MZI of two directional-coupler devices (500 x 220 nm strips 200 nm apart at their 3 dB length, analytic) and two strip waveguide devices 150 and 100 µm long, solved as a netlist at 21 wavelengths from 1.53 to 1.57 µm, against the product of its devices' transfer matrices, the largest $\lvert S - S_\mathrm{closed} \rvert$ (shown)",
            tier: Tier::Analytic,
            source: r"the MZI's closed form $M = C_c \thinspace \mathrm{diag}(t_l, t_u) \thinspace C_s$, $C = ((t, x), (x, t))$, each coupler's through and across fields and each arm's transmission from its own device; exact: no grid",
            run: closed_form,
        },
        Case {
            id: "chip/derivatives",
            title: r"Chips' optimizations: every exact derivative of the MZI of devices' objective and constraints (its coupler's length and its arm's, through the devices' derivatives and the circuit adjoint, at 3 wavelengths from 1.54 to 1.56 µm) against fourth-order central differences, the largest relative difference (shown)",
            tier: Tier::Analytic,
            source: r"the derivatives' definition: $(f(x-2h) - 8f(x-h) + 8f(x+h) - f(x+2h))/12h$ with $h = 5\times10^{-6} \max(\lvert x \rvert, 1)$, its error $O(h^4)$",
            run: derivatives,
        },
        Case {
            id: "chip/adjoint",
            title: r"Chips' optimizations: the gradient of the MZI of devices' worst transmission in dB over 5 wavelengths from 1.54 to 1.56 µm in its coupler's length, against the circuit adjoint's of the same objective (Circuit::gradient of objective::worst of objective::power_db), the relative difference (shown)",
            tier: Tier::Analytic,
            source: r"the circuit adjoint (Veronis et al. 2004, doi:10.1364/OL.29.002288): one transposed solve per wavelength; the job's by the Jacobian's columns through the expression in forward mode, the same derivative",
            run: adjoint,
        },
        Case {
            id: "chip/study-points",
            title: r"Chips' studies: a sweep of the MZI of devices' coupler length over 3 points at 5 wavelengths, each point's S-parameters against its own chip's spectrum: the numbers whose bits differ (count shown)",
            tier: Tier::Analytic,
            source: "each point is the chip whose parameter is that value, computed by the same netlist solve: the same bits",
            run: study_points,
        },
        Case {
            id: "chip/threads",
            title: r"Chips' optimizations: the MZI of devices' evaluation with its gradient in three variables (two exact, a coupler's gap by central differences side by side on rayon's threads) on 1 and on 20 threads: the numbers whose bits differ (count shown)",
            tier: Tier::Analytic,
            source: "the same bits on any number of threads (AGENTS.md): each difference is computed on its own and gathered in the variables' order",
            run: threads,
        },
    ]
}

/// A strip waveguide device `length` µm long.
fn guide(length: f64) -> Spec {
    let mut s = Spec::new("waveguide");
    s.parameters.push(("length".into(), format!("{length:?}")));
    s
}

/// An MZI of two directional-coupler devices at their 3 dB length and two waveguide devices,
/// ports in1 (the splitter's o1), in2 (o2), out1 (the combiner's o4), out2 (o3): the splitter's
/// o3 feeds the upper arm, its o4 the lower; the upper arm enters the combiner at o2, the lower
/// at o1.
pub(crate) fn mzi() -> ChipDesign {
    ChipDesign::new()
        .instance(ChipInstance::device(
            "split",
            Spec::new("directional coupler"),
        ))
        .instance(ChipInstance::device("upper", guide(150.0)))
        .instance(ChipInstance::device("lower", guide(100.0)))
        .instance(ChipInstance::device(
            "combine",
            Spec::new("directional coupler"),
        ))
        .connect("split.o3", "upper.o1")
        .connect("upper.o2", "combine.o2")
        .connect("split.o4", "lower.o1")
        .connect("lower.o2", "combine.o1")
        .port("in1", "split.o1")
        .port("in2", "split.o2")
        .port("out1", "combine.o4")
        .port("out2", "combine.o3")
}

/// The largest difference of the MZI of devices from its closed form.
pub(crate) fn closed_form_error() -> crate::Result<f64> {
    let a = mzi().assemble()?;
    let mut worst: f64 = 0.0;
    for k in 0..21 {
        let w = 1.53 + 0.002 * f64::from(k);
        let at = Wavelength::um(w)?;
        let s = a.s_matrix(w)?;
        let own = |i: usize| -> crate::Result<SMatrix> {
            let part = &a.parts[i];
            let n = part.component.parameters().len();
            part.component
                .s_matrix(at, &a.values[part.offset..part.offset + n])
        };
        let (split, upper, lower, combine) = (own(0)?, own(1)?, own(2)?, own(3)?);
        // a coupler's through o1 → o4 and across o1 → o3
        let tx = |c: &SMatrix| (c[(3, 0)], c[(2, 0)]);
        let m = mzi_closed_form(tx(&split), tx(&combine), upper[(1, 0)], lower[(1, 0)]);
        // the chip's ports in1, in2, out1, out2 against the closed form's o1, o2 and o4, o3
        for (q, p, e) in [
            (2, 0, m[0][0]),
            (2, 1, m[0][1]),
            (3, 0, m[1][0]),
            (3, 1, m[1][1]),
        ] {
            worst = worst.max((s[(q, p)] - e).norm());
        }
    }
    Ok(worst)
}

/// The MZI of devices' optimization: its coupler's length and its upper arm's (exact, through
/// the devices' derivatives and the circuit adjoint), the combiner's gap (differences); the
/// cross power maximized, the splitter's excess loss and the bar power constrained.
pub(crate) fn mzi_problem() -> crate::Result<Problem> {
    let optimization = Optimization::new("mma", 1, 10)
        .variable(Variable::new("split.length", 10.0, 30.0))
        .variable(Variable::new("upper.length", 140.0, 160.0))
        .variable(Variable::new("combine.gap", 0.15, 0.3))
        .objective(Aim::expression("T_out2_in1", Goal::Maximize))
        .constraint(Bound::declared("excess_loss").of_instance("split"))
        .constraint(Bound::expression("T_out1_in1", Limit::AtMost, 0.9));
    let job = optimize_from_chip("check", &mzi(), (1.54, 1.56, 3), &optimization, 1.0)?;
    Problem::of(&job)
}

fn closed_form() -> Outcome {
    let Ok(e) = closed_form_error() else {
        return failed();
    };
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance: 1e-13,
        error: e,
    }
}

fn derivatives() -> Outcome {
    let Ok(mut p) = mzi_problem() else {
        return failed();
    };
    let Ok(checks) = p.check_gradients() else {
        return failed();
    };
    if checks.len() < 4 || p.derivatives() != ["exact", "exact", "differences"] {
        return failed();
    }
    let largest = checks.iter().fold(0.0f64, |m, c| {
        let size = c.exact.abs().max(c.differences.abs());
        if size > 1e-9 {
            m.max((c.exact - c.differences).abs() / size)
        } else {
            m
        }
    });
    Outcome {
        measured: largest,
        expected: 0.0,
        tolerance: 1e-6,
        error: largest,
    }
}

/// The relative difference of the chip problem's gradient from the circuit adjoint's.
pub(crate) fn adjoint_difference() -> crate::Result<f64> {
    let optimization = Optimization::new("lbfgsb", 1, 10)
        .variable(Variable::new("split.length", 10.0, 30.0))
        .objective(Aim::expression("10*log10(T_out1_in1)", Goal::Minimize));
    let job = optimize_from_chip("check", &mzi(), (1.54, 1.56, 5), &optimization, 1.0)?;
    let mut p = Problem::of(&job)?;
    p.check_gradients()?;
    let x = 21.0;
    let e = p.evaluate(&[x], true);
    let a = p.chip_at(&[x])?.assemble()?;
    let wavelengths = (0..5)
        .map(|k| Wavelength::um(1.54 + 0.005 * f64::from(k)))
        .collect::<crate::Result<Vec<_>>>()?;
    let (f, g) = a.circuit.gradient(&wavelengths, &a.values, |s| {
        objective::worst(s, |m| {
            objective::at_one(m, |x| objective::power_db(x, 2, 0))
        })
    })?;
    let slot = a
        .circuit
        .parameter("split.length")
        .ok_or_else(|| crate::Error::invalid("chip", "no split.length"))?;
    if e.gradients.is_empty() || (e.costs[0] - f).abs() > 1e-12 {
        return Ok(f64::INFINITY);
    }
    Ok((e.gradients[0][0] - g[slot]).abs() / g[slot].abs())
}

fn adjoint() -> Outcome {
    let d = adjoint_difference().unwrap_or(f64::INFINITY);
    Outcome {
        measured: d,
        expected: 0.0,
        tolerance: 1e-9,
        error: d,
    }
}

/// How many numbers of a chip's study differ from their points' own spectra.
pub(crate) fn study_differences() -> crate::Result<usize> {
    let axes = [SweepAxis::new("split.length", 10.0, 30.0, 3)];
    let band = super::chip::band(1.54, 1.56, Some(5), 5)?;
    let job = study_from_chip("check", &mzi(), (1.54, 1.56, 5), &axes, &[], 1.0)?;
    let dir = std::env::temp_dir().join(format!("photonoxide-chip-check-{}", std::process::id()));
    let mut run = Run::create(&dir, &job)?;
    super::execute(&job, &mut run, &Stop::new(None))?;
    let events: Vec<Event> = replay(run.dir())?;
    let _ = std::fs::remove_dir_all(&dir);
    let mut differ = 0;
    let mut points = 0;
    for e in &events {
        if let Event::ChipPoint { point, s, .. } = e {
            points += 1;
            let mut chip = mzi();
            let length = [10.0, 20.0, 30.0][*point];
            let super::chip::InstanceModel::Device(spec) = &mut chip.instances[0].model else {
                return Ok(usize::MAX);
            };
            spec.parameters
                .push(("length".into(), format!("{length:?}")));
            let own = spectrum(&chip, &band)?;
            let flat = |x: &Vec<Vec<Vec<[f64; 2]>>>| -> Vec<u64> {
                x.iter()
                    .flatten()
                    .flatten()
                    .flat_map(|z| [z[0].to_bits(), z[1].to_bits()])
                    .collect()
            };
            let (a, b) = (flat(s), flat(&own));
            differ += a.len().abs_diff(b.len()) + a.iter().zip(&b).filter(|(x, y)| x != y).count();
        }
    }
    Ok(if points == 3 { differ } else { usize::MAX })
}

fn study_points() -> Outcome {
    let d = study_differences().map_or(f64::INFINITY, |d| d as f64);
    Outcome {
        measured: d,
        expected: 0.0,
        tolerance: 0.0,
        error: d,
    }
}

fn threads() -> Outcome {
    let Ok(mut p) = mzi_problem() else {
        return failed();
    };
    if p.check_gradients().is_err() {
        return failed();
    }
    let on = |threads: usize| -> Option<Vec<u64>> {
        let e = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .ok()?
            .install(|| p.evaluate(&[22.0, 151.0, 0.21], true));
        Some(
            e.costs
                .iter()
                .chain(&e.constraints)
                .chain(e.gradients.iter().flatten())
                .chain(e.jacobian.iter().flatten())
                .map(|v| v.to_bits())
                .collect(),
        )
    };
    let (Some(one), Some(twenty)) = (on(1), on(20)) else {
        return failed();
    };
    if one.len() != 12 {
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

fn failed() -> Outcome {
    Outcome {
        measured: f64::NAN,
        expected: 0.0,
        tolerance: 0.0,
        error: f64::INFINITY,
    }
}
