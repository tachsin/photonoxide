//! The GPU's validation report, docs/validation-gpu.md: the GPU against the CPU, its runs'
//! repeatability, a published result on it, every FDTD case of the main report run again on it,
//! and its speed. GitHub's runners have no GPU, so this report is written on tachsin's machine
//! before each release (`cargo test --release --features gpu --test gpu_report -- --ignored`),
//! not checked by CI; docs/validation.md is the same with or without a GPU.

use std::fmt::Write as _;
use std::time::Instant;

use super::{Outcome, Tier, cases};
use crate::Result;
use crate::fdtd::gpu::checks::{
    Agreement, agreement, busy, closed, kernel_against_cpu, repeats, roden_gedney,
};
use crate::fdtd::gpu::everything_on;
use crate::fdtd::{Device, Gpu, Precision};

/// One of the GPU's own cases: as a [`super::Case`], run on a GPU of `precision`.
struct GpuCase {
    id: &'static str,
    title: &'static str,
    tier: Tier,
    source: &'static str,
    precision: Precision,
    run: fn(&Gpu) -> Outcome,
}

/// The CPU's errors on Roden and Gedney's plate, the `cpml_roden_gedney` example's output.
const RODEN_GEDNEY_CPU: [f64; 2] = [-48.620087, -70.474859];

fn largest(a: &Agreement) -> f64 {
    a.fields.max(a.probes).max(a.transforms).max(a.flux)
}

/// Fused and in two passes, in one run and in runs of 130 steps: the largest relative
/// difference from the CPU's f64 run after 800 steps.
fn agreement_all(gpu: &Gpu, two_d: bool) -> f64 {
    let s = busy(two_d);
    let mut worst: f64 = 0.0;
    for fused in [true, false] {
        for piece in [800, 130] {
            worst = worst.max(largest(&agreement(&s, gpu, 800, piece, fused)));
        }
    }
    worst
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// The GPU's own cases.
fn gpu_cases() -> Vec<GpuCase> {
    vec![
        GpuCase {
            id: "gpu/f64-3d",
            title: "In f64, a 3D run (37 × 21 × 19 cells of 50 nm: CPMLs with κ and α, a wall, conductivity, a conductor, sources on E and H̃, a current, a dipole, probes, a transform box, a flux box; 800 steps, fused and in two passes, in one run and in runs of 130): fields, probes, transforms and fluxes against the CPU's, relative (largest shown)",
            tier: Tier::CrossCode,
            source: "photonoxide's CPU kernel in f64, the same problem",
            precision: Precision::Double,
            run: |gpu| outcome(agreement_all(gpu, false), 0.0, 1e-13),
        },
        GpuCase {
            id: "gpu/f64-2d",
            title: "In f64, the same in 2D (70 × 45 cells of 50 nm, periodic along x, CPMLs along y, a flux plane)",
            tier: Tier::CrossCode,
            source: "photonoxide's CPU kernel in f64, the same problem",
            precision: Precision::Double,
            run: |gpu| outcome(agreement_all(gpu, true), 0.0, 1e-13),
        },
        GpuCase {
            id: "gpu/f32-3d",
            title: "In f32, the 3D run against the CPU's in f64, relative (largest shown)",
            tier: Tier::CrossCode,
            source: "photonoxide's CPU kernel in f64, the same problem",
            precision: Precision::Single,
            run: |gpu| outcome(agreement_all(gpu, false), 0.0, 2e-5),
        },
        GpuCase {
            id: "gpu/f32-2d",
            title: "In f32, the 2D run against the CPU's in f64, relative (largest shown)",
            tier: Tier::CrossCode,
            source: "photonoxide's CPU kernel in f64, the same problem",
            precision: Precision::Single,
            run: |gpu| outcome(agreement_all(gpu, true), 0.0, 2e-5),
        },
        GpuCase {
            id: "gpu/f32-rounding",
            title: "In f32, a closed box (40 × 36 × 32 cells of 50 nm, a block of ε = 12, random fields) after $N$ = 25 600 steps: E against the CPU's f32 kernel, relative, a random walk of rounding (tolerance $2 \\times 10^{-7} \\sqrt{N}$)",
            tier: Tier::CrossCode,
            source: "photonoxide's CPU kernel in f32, the same problem",
            precision: Precision::Single,
            run: |gpu| {
                let g = kernel_against_cpu(&closed(), gpu, &[25_600], true);
                outcome(g[0][0], 0.0, 3.2e-5)
            },
        },
        GpuCase {
            id: "gpu/f32-drift",
            title: "In f32, the closed box's drift from f64 after 25 600 steps (the coefficients rounded to f32), over the CPU's f32 kernel's drift",
            tier: Tier::CrossCode,
            source: "photonoxide's CPU kernels in f32 and f64, the same problem",
            precision: Precision::Single,
            run: |gpu| {
                let g = kernel_against_cpu(&closed(), gpu, &[25_600], true);
                outcome((g[0][1] / g[0][2] * 1e4).round() / 1e4, 1.0, 0.05)
            },
        },
        GpuCase {
            id: "gpu/f64-rounding",
            title: "In f64, the closed box after $N$ = 25 600 steps: E against the CPU's, relative (tolerance $10^{-15} \\sqrt{N}$)",
            tier: Tier::CrossCode,
            source: "photonoxide's CPU kernel in f64, the same problem",
            precision: Precision::Double,
            run: |gpu| {
                let g = kernel_against_cpu(&closed(), gpu, &[25_600], true);
                outcome(g[0][0], 0.0, 1.6e-13)
            },
        },
        GpuCase {
            id: "gpu/repeat-f32",
            title: "In f32, two runs of the 3D and 2D problems, 300 steps, fused and in two passes: values that differ (fields, probes, transforms)",
            tier: Tier::Analytic,
            source: "no atomics, fixed workgroups, every sum by one invocation in step order: the same bits on the same device and driver",
            precision: Precision::Single,
            run: |gpu| repeat_all(gpu),
        },
        GpuCase {
            id: "gpu/repeat-f64",
            title: "In f64, the same: values that differ",
            tier: Tier::Analytic,
            source: "as above",
            precision: Precision::Double,
            run: |gpu| repeat_all(gpu),
        },
        GpuCase {
            id: "gpu/roden-gedney",
            title: "In f32, Roden and Gedney's plate in soil (the `cpml_roden_gedney` example: 126 × 51 × 26 cells of 1 mm, its reference 206 × 131 × 106, 2000 steps), the traditional PML's largest error, dB",
            tier: Tier::Published,
            source: "J. A. Roden, S. D. Gedney, Microw. Opt. Technol. Lett. 27, 334 (2000), doi:10.1002/1098-2760(20001205)27:5<334::AID-MOP14>3.0.CO;2-A, \"on the order of −48 dB\"",
            precision: Precision::Single,
            run: |gpu| {
                let [a, _] = roden_gedney_on(gpu);
                outcome(a, -48.0, 5.0)
            },
        },
        GpuCase {
            id: "gpu/roden-gedney-cfs",
            title: "The same with the CFS-PML (α = 0.05 S/m), dB",
            tier: Tier::Published,
            source: "Roden and Gedney, as above: \"−67 dB\"",
            precision: Precision::Single,
            run: |gpu| {
                let [_, b] = roden_gedney_on(gpu);
                outcome(b, -67.0, 5.0)
            },
        },
        GpuCase {
            id: "gpu/roden-gedney-cpu",
            title: "Both errors against the CPU's in f64 (−48.6201 and −70.4749 dB, the example's output), largest difference, dB",
            tier: Tier::CrossCode,
            source: "photonoxide's CPU kernel in f64, the `cpml_roden_gedney` example",
            precision: Precision::Single,
            run: |gpu| {
                let e = roden_gedney_on(gpu);
                let d = e
                    .iter()
                    .zip(RODEN_GEDNEY_CPU)
                    .map(|(a, b)| (a - b).abs())
                    .fold(0.0, f64::max);
                outcome(d, 0.0, 0.05)
            },
        },
    ]
}

/// Roden and Gedney's errors on `gpu`, computed once.
fn roden_gedney_on(gpu: &Gpu) -> [f64; 2] {
    static RUN: std::sync::Mutex<Option<[f64; 2]>> = std::sync::Mutex::new(None);
    let mut run = RUN.lock().unwrap_or_else(|e| e.into_inner());
    *run.get_or_insert_with(|| roden_gedney(&Device::Gpu(gpu.clone())))
}

fn repeat_all(gpu: &Gpu) -> Outcome {
    let mut differ = 0.0;
    for two_d in [false, true] {
        for fused in [true, false] {
            if !repeats(&busy(two_d), gpu, 300, fused) {
                differ += 1.0;
            }
        }
    }
    outcome(differ, 0.0, 0.0)
}

fn e(v: f64) -> String {
    if v == 0.0 {
        "0".into()
    } else if (1e-3..1e4).contains(&v.abs()) {
        super::sig(v)
    } else {
        format!("{v:.2e}")
    }
}

/// One of the main report's FDTD cases run with every simulation on `gpu` (the CPU where the
/// GPU doesn't step a problem): its outcome, or why it stopped short (a check of its own that
/// panicked), and the steps it took on the GPU and on the CPU.
fn on_gpu(case: &super::Case, gpu: &Gpu) -> (std::result::Result<Outcome, String>, [usize; 2]) {
    let run = || {
        std::panic::catch_unwind(case.run).map_err(|p| {
            p.downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| p.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "a check panicked".into())
        })
    };
    everything_on(gpu, run)
}

/// The share of `steps` (on the GPU, on the CPU) on the GPU, or that it took none.
fn share([g, c]: [usize; 2]) -> String {
    if g + c == 0 {
        "no steps".into()
    } else if g == 0 {
        "CPU only".into()
    } else {
        format!("{:.0} %", 100.0 * g as f64 / (g + c) as f64)
    }
}

/// An outcome's cell: its value and whether it passed (`pass` and `**FAIL**`, or `within` and
/// `beyond` for f32), or why it stopped short.
fn cell(o: &std::result::Result<Outcome, String>, held: bool) -> String {
    match o {
        Err(why) if held => format!("**stopped: {why}**"),
        Err(why) => format!("stopped: {why}"),
        Ok(o) => format!(
            "{} {}",
            e(o.measured),
            match (o.passed(), held) {
                (true, true) => "pass",
                (false, true) => "**FAIL**",
                (true, false) => "within",
                (false, false) => "beyond",
            }
        ),
    }
}

/// Cell-updates per second of the kernel alone, a 256³ grid in vacuum inside CPMLs of 8,
/// fused, and of the CPU's blocked kernel on all threads, in `T`.
fn speeds(gpus: &[&Gpu]) -> String {
    use crate::fdtd::gpu::rates;
    // `GPU_REPORT_CPU=0` leaves the CPU out, when other work on the machine would slow it
    let cpu = std::env::var("GPU_REPORT_CPU").map_or(true, |v| v != "0");
    let mut out = String::new();
    let _ = writeln!(
        out,
        "| Grid | Precision | GPU, two passes | GPU, fused | CPU, diamonds on {} threads | GPU / CPU |\n|---|---|---:|---:|---:|---:|",
        rayon::current_num_threads()
    );
    for n in [128, 256] {
        let mut s = rates::filled(n);
        for gpu in gpus {
            let p = gpu.precision();
            let two = rates::gpu_rate(&mut s, gpu, 0, false);
            let one = rates::gpu_rate(&mut s, gpu, 0, true);
            let c = cpu.then(|| match p {
                Precision::Single => rates::cpu_rate::<f32>(&s),
                Precision::Double => rates::cpu_rate::<f64>(&s),
            });
            let _ = writeln!(
                out,
                "| {n}³ cells of 50 nm, vacuum, CPMLs of 8 | {} | {:.0} | {:.0} | {} | {} |",
                match p {
                    Precision::Single => "f32",
                    Precision::Double => "f64",
                },
                two / 1e6,
                one / 1e6,
                c.map_or("not measured".into(), |c| format!("{:.0}", c / 1e6)),
                c.map_or("—".into(), |c| format!("{:.2}", two.max(one) / c)),
            );
        }
    }
    out
}

/// Runs the GPU's cases and the main report's FDTD cases on the GPU, and returns the markdown
/// report and whether every case passed: the GPU's own, and every FDTD case run in f64 at
/// least partly on the GPU (in f32 they are shown against their tolerances, set for f64, and
/// not held to them).
///
/// # Errors
///
/// [`crate::Error::Gpu`] if there is no GPU for f32.
pub fn gpu_report() -> Result<(String, bool)> {
    let single = Gpu::new(Precision::Single)?;
    let double = Gpu::new(Precision::Double).ok();
    let mut all = true;
    let mut rows = String::new();
    for case in gpu_cases() {
        let gpu = match case.precision {
            Precision::Single => &single,
            Precision::Double => match &double {
                Some(g) => g,
                None => {
                    let _ = writeln!(
                        rows,
                        "| `{}` | {} | {} | {} | — | — | — | no f64 GPU |",
                        case.id,
                        case.tier.label(),
                        case.title,
                        case.source
                    );
                    continue;
                }
            },
        };
        let o = (case.run)(gpu);
        let passed = o.passed();
        all &= passed;
        let _ = writeln!(
            rows,
            "| `{}` | {} | {} | {} | {} | {} | {:e} | {} |",
            case.id,
            case.tier.label(),
            case.title,
            case.source,
            e(o.measured),
            e(o.expected),
            o.tolerance,
            if passed { "pass" } else { "**FAIL**" }
        );
    }
    let mut every = String::new();
    let (mut ran, mut cpu_only) = (0, 0);
    // `GPU_CASES=fdtd/mie` runs only the cases whose id starts so, for a look at a few
    let only = std::env::var("GPU_CASES").unwrap_or_else(|_| "fdtd/".into());
    for case in cases()
        .iter()
        .filter(|c| c.id.starts_with("fdtd/") && c.id.starts_with(&only))
    {
        let t = Instant::now();
        let cpu = (case.run)();
        let cpu_time = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let (f64_cell, steps) = match &double {
            Some(g) => {
                let (o, steps) = on_gpu(case, g);
                if steps[0] > 0 && !o.as_ref().is_ok_and(Outcome::passed) {
                    all = false;
                }
                (cell(&o, true), steps)
            }
            None => ("—".to_string(), [0, 0]),
        };
        let gpu_time = t.elapsed().as_secs_f64();
        let (f32_run, steps32) = on_gpu(case, &single);
        let steps = if double.is_some() { steps } else { steps32 };
        if steps[0] > 0 {
            ran += 1;
        } else {
            cpu_only += 1;
        }
        let _ = writeln!(
            every,
            "| `{}` | {} | {} | {} | {:e} | {} | {:.1} s / {:.1} s |",
            case.id,
            cell(&Ok(cpu.clone()), true),
            f64_cell,
            cell(&f32_run, false),
            cpu.tolerance,
            share(steps),
            cpu_time,
            gpu_time,
        );
        eprintln!(
            "{}: {cpu_time:.1} s on the CPU, {gpu_time:.1} s on the GPU in f64",
            case.id
        );
    }
    let speed = speeds(
        &[Some(&single), double.as_ref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
    );
    let devices = [Some(&single), double.as_ref()]
        .into_iter()
        .flatten()
        .map(|g| format!("{} in {:?}", g.name(), g.precision()))
        .collect::<Vec<_>>()
        .join("; ");
    let text = format!(
        "# Validation report: the GPU\n\
         \n\
         Written by `cargo test --release --features gpu --test gpu_report -- --ignored` on \
         tachsin's machine before each release; don't edit it by hand. GitHub's runners have no \
         GPU, so CI doesn't run this report: [the main report](validation.md) is the same with \
         or without one. A GPU run repeats bit for bit on the same device and driver, and agrees \
         with the CPU to the tolerances below (see [FDTD](methods/fdtd.md#the-gpu)).\n\
         \n\
         Devices: {devices}.\n\
         \n\
         ## The GPU's cases\n\
         \n\
         | Case | Tier | What | Against | Measured | Expected | Tolerance | Result |\n\
         |---|---|---|---|---|---|---|---|\n\
         {rows}\
         \n\
         ## Every FDTD case on the GPU\n\
         \n\
         Each FDTD case of the main report run three times: on the CPU, then with every \
         simulation it makes on the GPU in f64, then in f32. A simulation the GPU doesn't step \
         yet (a Bloch phase, a dispersive medium, a smoothed tensor that couples E's components, \
         a plane wave, the plain loops) falls back to the CPU; the last column but one is the \
         share of the steps taken on the GPU. The tolerances are the main report's, set for \
         f64: in f32 a case is shown within or beyond its tolerance, not held to it, and a case \
         whose checks of its own f32 can't meet (a field decaying far below f32's rounding, say) \
         stops short. {ran} cases ran on the GPU, {cpu_only} on the CPU only or took no steps \
         (analytic cases, and one whose runs are kept from before). The last column is the time on \
         the CPU and on the GPU in f64, including the copies to and from the GPU at each run.\n\
         \n\
         | Case | CPU | GPU, f64 | GPU, f32 | Tolerance | On the GPU | Time |\n\
         |---|---|---|---|---|---|---|\n\
         {every}\
         \n\
         ## Speed\n\
         \n\
         The kernel alone on random fields, million cell-updates a second, the best of three \
         runs of a few tenths of a second each; the CPU's blocked kernel (Malas et al.'s \
         diamonds, `Blocking::auto`) in the same precision on all its threads, unless the \
         machine was busy when this report was written (`GPU_REPORT_CPU=0`): the GPU's rate \
         hardly depends on the CPU's load, the CPU's does. [FDTD](methods/fdtd.md#the-gpu) has \
         both measured on an idle machine.\n\
         \n\
         {speed}"
    );
    Ok((text, all))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_gpu_cases_have_unique_ids_without_pipes() {
        let cases = super::gpu_cases();
        let mut ids: Vec<_> = cases.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), cases.len());
        for c in &cases {
            assert!(
                !c.title.contains('|') && !c.source.contains('|'),
                "{}",
                c.id
            );
        }
    }
}
