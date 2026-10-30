//! The GPU's rates, measured rather than tested: cell-updates per second of the kernel alone on
//! cubes of vacuum inside CPMLs, against the blocked CPU kernel on all the machine's threads.
//! Run alone on the machine:
//!
//! ```sh
//! cargo test --release --features gpu --lib fdtd::gpu::rates -- --ignored --nocapture --test-threads 1
//! ```
//!
//! `GPU_SIZES=128,320 GPU_PLANES=4,8,16` choose the cubes and the planes a workgroup marches
//! through (0: the default); `GPU_CPU=0` leaves the CPU out.

use std::time::Instant;

use super::{Gpu, Resident};
#[cfg(test)]
use super::{Precision, for_tests};
use crate::fdfd::Axis;
use crate::fdtd::checks::{grid, noise};
use crate::fdtd::{Blocking, Boundaries, Real, Simulation, Yee};

/// A cube of `n` cells of 50 nm of vacuum inside CPMLs of 8, random fields.
pub(crate) fn filled(n: usize) -> Simulation {
    let mut s =
        Simulation::in_uniform_medium(grid([n; 3], 0.05), 1.0, Boundaries::cpml(8), 0.9).unwrap();
    for c in Axis::ALL {
        let k = c.index();
        for (r, v) in s.e_mut(c).iter_mut().enumerate() {
            *v = noise(r + 3 * k);
        }
        for (r, v) in s.h_mut(c).iter_mut().enumerate() {
            *v = noise(r + 7 * k + 50_000);
        }
    }
    s
}

/// The GPU's rate on `s`, the kernel alone, the best of three runs of enough steps to take a few
/// tenths of a second, in cell-updates per second.
pub(crate) fn gpu_rate(s: &mut Simulation, gpu: &Gpu, planes: usize, fused: bool) -> f64 {
    let mut resident = Resident::new(gpu, s).unwrap();
    if planes > 0 {
        resident.planes = planes;
    }
    resident.fused &= fused;
    rate_of(s, &mut resident)
}

/// The rate of `resident`, which holds `s`, as [`gpu_rate`].
fn rate_of(s: &mut Simulation, resident: &mut Resident) -> f64 {
    resident.upload(s).unwrap();
    let cells = s.grid().cells() as f64;
    let mut steps = 4;
    resident.step(s, steps, false).unwrap();
    loop {
        let t = Instant::now();
        resident.step(s, steps, false).unwrap();
        if t.elapsed().as_secs_f64() > 0.3 {
            break;
        }
        steps *= 2;
    }
    (0..3)
        .map(|_| {
            let t = Instant::now();
            resident.step(s, steps, false).unwrap();
            cells * steps as f64 / t.elapsed().as_secs_f64()
        })
        .fold(0.0, f64::max)
}

/// The blocked CPU kernel's rate on `s` in precision `T` on all the threads, by the diamonds
/// [`Blocking::auto`] chooses, the best of three runs.
pub(crate) fn cpu_rate<T: Real>(s: &Simulation) -> f64 {
    let mut yee = Yee::<T>::from_simulation(s).unwrap();
    let blocking = Blocking::auto(
        s.grid(),
        std::mem::size_of::<T>(),
        true,
        rayon::current_num_threads(),
    );
    let cells = s.grid().cells() as f64;
    let block = blocking.map_or(1, |b| 2 * b.steps);
    let mut steps = block.max(4);
    yee.run(steps, blocking);
    loop {
        let t = Instant::now();
        yee.run(steps, blocking);
        if t.elapsed().as_secs_f64() > 0.3 {
            break;
        }
        steps *= 2;
    }
    (0..3)
        .map(|_| {
            let t = Instant::now();
            yee.run(steps, blocking);
            cells * steps as f64 / t.elapsed().as_secs_f64()
        })
        .fold(0.0, f64::max)
}

#[cfg(test)]
fn list(name: &str, default: &str) -> Vec<usize> {
    std::env::var(name)
        .unwrap_or(default.into())
        .split(',')
        .map(|s| s.trim().parse().unwrap())
        .collect()
}

#[test]
#[ignore = "a measurement: run alone, in release"]
fn gpu_rates() {
    let sizes = list("GPU_SIZES", "128,192,256,320");
    let planes = list("GPU_PLANES", "0");
    let cpu = std::env::var("GPU_CPU").map_or(true, |v| v != "0");
    let gpus: Vec<Gpu> = [Precision::Single, Precision::Double]
        .into_iter()
        .filter_map(for_tests)
        .collect();
    println!(
        "{} threads; {}",
        rayon::current_num_threads(),
        gpus.iter().map(Gpu::name).collect::<Vec<_>>().join("; ")
    );
    println!(
        "| Grid | Precision | GPU, two passes (planes) | GPU, fused (planes) | CPU, diamonds | GPU / CPU |"
    );
    println!("|---|---|---:|---:|---:|---:|");
    for &n in &sizes {
        let mut s = filled(n);
        for gpu in &gpus {
            let p = gpu.precision();
            let mut rates = |fused: bool| -> Vec<(usize, f64)> {
                planes
                    .iter()
                    .map(|&planes| (planes, gpu_rate(&mut s, gpu, planes, fused)))
                    .collect()
            };
            let (two, one) = (rates(false), rates(true));
            let best = two.iter().chain(&one).map(|r| r.1).fold(0.0, f64::max);
            let show = |rates: &[(usize, f64)]| {
                rates
                    .iter()
                    .map(|(planes, r)| format!("{:.0} ({planes})", r / 1e6))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let c = cpu.then(|| match p {
                Precision::Single => cpu_rate::<f32>(&s),
                Precision::Double => cpu_rate::<f64>(&s),
            });
            println!(
                "| {n}³ | {p:?} | {} | {} | {} | {} |",
                show(&two),
                show(&one),
                c.map_or("—".into(), |c| format!("{:.0}", c / 1e6)),
                c.map_or("—".into(), |c| format!("{:.2}", best / c)),
            );
        }
    }
}

/// The largest cubes of vacuum inside CPMLs of 8 whose fields fit on the GPU, in steps of 32
/// cells: fused (two copies of E and H̃) and in two passes, each run to check it steps at speed.
#[test]
#[ignore = "a measurement: run alone, in release; takes tens of GB of memory on the CPU"]
fn gpu_memory() {
    for precision in [Precision::Single, Precision::Double] {
        let Some(gpu) = for_tests(precision) else {
            continue;
        };
        let mut n = list("GPU_FROM", "256")[0];
        loop {
            let mut s =
                Simulation::in_uniform_medium(grid([n; 3], 0.05), 1.0, Boundaries::cpml(8), 0.9)
                    .unwrap();
            match Resident::new(&gpu, &s) {
                Ok(mut r) => {
                    let fused = r.fused;
                    let rate = rate_of(&mut s, &mut r);
                    println!(
                        "{precision:?} {n}³ ({:.1} M cells): fits, {}, {:.0} M cell-updates/s",
                        (n * n * n) as f64 / 1e6,
                        if fused { "fused" } else { "two passes" },
                        rate / 1e6
                    );
                }
                Err(e) => {
                    println!("{precision:?} {n}³: {e}");
                    break;
                }
            }
            n += 32;
        }
    }
}

/// A simulation's rate on the GPU with a point source, a probe and a flux box (six faces
/// transformed at three frequencies), as `fdtd::kernel_rates::simulation_rates` measures the
/// CPU's: runs of `GPU_STEPS` steps (default 200), the copies there and back included, and the
/// CPU's by diamonds on all threads.
#[test]
#[ignore = "a measurement: run alone, in release"]
fn gpu_simulation_rates() {
    use crate::fdtd::{Device, Field, Source, Waveform};
    use crate::units::Frequency;
    let steps = list("GPU_STEPS", "200")[0];
    let gpus: Vec<Gpu> = [Precision::Single, Precision::Double]
        .into_iter()
        .filter_map(for_tests)
        .collect();
    for n in list("GPU_SIZES", "128,256") {
        let mut s = filled(n);
        s.add_source(Source {
            field: Field::E,
            component: Axis::Z,
            at: (n / 2, n / 2, n / 2),
            waveform: Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.6).unwrap(),
        })
        .unwrap();
        s.add_probe(Field::E, Axis::Z, (n / 3, n / 2, n / 2))
            .unwrap();
        let frequencies = [0.8, 1.0, 1.2].map(|f| Frequency::natural(f).unwrap());
        let (a, b) = (n / 4, 3 * n / 4);
        s.add_flux_box([(a, b), (a, b), (a, b)], &frequencies)
            .unwrap();
        let cells = (n * n * n * steps) as f64;
        for gpu in &gpus {
            let mut x = s.clone();
            x.set_device(Device::Gpu(gpu.clone())).unwrap();
            x.run(4);
            let t = Instant::now();
            x.run(steps);
            let all = t.elapsed().as_secs_f64();
            println!(
                "{n}³ GPU {:?}: {:.0} M cell-updates/s over runs of {steps} steps, copies included",
                gpu.precision(),
                cells / all / 1e6
            );
        }
        let mut x = s.clone();
        x.run(4);
        let t = Instant::now();
        x.run(steps);
        println!(
            "{n}³ CPU f64, {} threads: {:.0} M cell-updates/s",
            rayon::current_num_threads(),
            cells / t.elapsed().as_secs_f64() / 1e6
        );
    }
}
