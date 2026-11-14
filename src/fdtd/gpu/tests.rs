//! The GPU against the CPU: the same problems stepped on both. Each test skips, saying so,
//! where there is no GPU (and on CI, whose runners have none).

use super::checks::{Agreement, agreement, busy, closed, kernel_against_cpu, repeats, run_on};
use super::*;
use crate::fdfd::{Axis, Edges};
use crate::fdtd::checks::grid;
use crate::fdtd::{Boundaries, Device, Dispersive, Pole, Simulation};
use crate::units::Frequency;

fn within(a: &Agreement, tolerance: f64, what: &str) {
    for (name, v) in [
        ("fields", a.fields),
        ("probes", a.probes),
        ("transforms", a.transforms),
        ("flux", a.flux),
    ] {
        assert!(
            v <= tolerance,
            "{what}: {name} {v:e} > {tolerance:e} ({a:?})"
        );
    }
}

/// In f64 the GPU's arithmetic is the CPU's but for the order and fusion of a few operations:
/// fields, probes, transforms and fluxes within 1e-13 of the CPU's after 800 steps (measured up
/// to 1.2e-14), fused or in two passes, in one run or in several.
#[test]
fn in_f64_a_run_on_the_gpu_is_the_cpus_to_round_off() {
    let Some(gpu) = for_tests(Precision::Double) else {
        return;
    };
    for two_d in [false, true] {
        let s = busy(two_d);
        for fused in [true, false] {
            for piece in [800, 130] {
                let a = agreement(&s, &gpu, 800, piece, fused);
                within(
                    &a,
                    1e-13,
                    &format!("2D {two_d}, fused {fused}, runs of {piece}"),
                );
            }
        }
    }
}

/// In f32 a run is the CPU's f64 one to f32's rounding: within 2e-5 after 800 steps with
/// CPMLs, sources, probes and monitors (measured up to 4.4e-6).
#[test]
fn in_f32_a_run_on_the_gpu_is_the_cpus_to_f32s_rounding() {
    let Some(gpu) = for_tests(Precision::Single) else {
        return;
    };
    for two_d in [false, true] {
        let s = busy(two_d);
        for fused in [true, false] {
            let a = agreement(&s, &gpu, 800, 800, fused);
            within(&a, 2e-5, &format!("2D {two_d}, fused {fused}"));
        }
    }
}

/// How rounding grows in a closed box over 25 600 steps. The GPU's f32 kernel against the
/// CPU's f32 kernel: a random walk, within 2e-7 √N (measured 1.1e-6 to 1.6e-5, about 1e-7 √N).
/// Both against f64: a drift from the coefficients rounded to f32, linear, within 1e-7 N
/// (measured 5.6e-8 N for both). In f64, the GPU within 1e-15 √N of the CPU (measured 2e-16
/// √N).
#[test]
fn rounding_grows_on_the_gpu_as_on_the_cpu() {
    let checkpoints = [100, 400, 1600, 6400, 25600];
    if let Some(gpu) = for_tests(Precision::Single) {
        for fused in [true, false] {
            let growth = kernel_against_cpu(&closed(), &gpu, &checkpoints, fused);
            for (&n, [same, double, cpu]) in checkpoints.iter().zip(&growth) {
                let n = n as f64;
                assert!(*same <= 2e-7 * n.sqrt(), "{n}: {growth:?}");
                assert!(*double <= 1e-7 * n && *cpu <= 1e-7 * n, "{n}: {growth:?}");
            }
        }
    }
    if let Some(gpu) = for_tests(Precision::Double) {
        let growth = kernel_against_cpu(&closed(), &gpu, &checkpoints, true);
        for (&n, [same, ..]) in checkpoints.iter().zip(&growth) {
            assert!(*same <= 1e-15 * (n as f64).sqrt(), "{n}: {growth:?}");
        }
    }
}

/// No atomics and a fixed order: two runs on the same device are the same bits, fields,
/// probes and transforms, in f32 and f64, fused and in two passes.
#[test]
fn a_run_on_the_gpu_repeats_bit_for_bit() {
    for precision in [Precision::Single, Precision::Double] {
        let Some(gpu) = for_tests(precision) else {
            continue;
        };
        for two_d in [false, true] {
            for fused in [true, false] {
                assert!(
                    repeats(&busy(two_d), &gpu, 300, fused),
                    "{precision:?}, 2D {two_d}, fused {fused}"
                );
            }
        }
    }
}

/// The repeat above many times over, `GPU_REPEATS` runs of each (200 by default), in f32 and
/// f64: a run that differs once in many shows here, not in one repeat. In f64 the CPML's
/// derivatives, divided by Δ in the kernels, gave one of two results from run to run (the 3D
/// problem differed in most of 1200 runs; why: the test of a quotient under load, below); they
/// multiply by 1/Δ now, and no run differs. By
/// hand, on a computer with a GPU:
/// `cargo test --release --features gpu --lib repeats_bit_for_bit_many_times -- --ignored`.
#[test]
#[ignore = "many runs on a GPU: by hand, before a release"]
fn a_run_on_the_gpu_repeats_bit_for_bit_many_times() {
    let runs = std::env::var("GPU_REPEATS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200usize);
    for precision in [Precision::Single, Precision::Double] {
        let Some(gpu) = for_tests(precision) else {
            continue;
        };
        for two_d in [false, true] {
            let s = busy(two_d);
            for fused in [true, false] {
                let differ = (0..runs).filter(|_| !repeats(&s, &gpu, 300, fused)).count();
                println!("{precision:?}, 2D {two_d}, fused {fused}: {differ} of {runs} differ");
                assert_eq!(differ, 0, "{precision:?}, 2D {two_d}, fused {fused}");
            }
        }
    }
}

fn field_bits(s: &Simulation) -> Vec<u64> {
    Axis::ALL
        .iter()
        .flat_map(|&c| s.e(c).iter().chain(s.h(c)))
        .map(|v| v.to_bits())
        .collect()
}

/// A run in pieces is one run's fields and probes bit for bit (the fields go to the CPU and
/// back between pieces exactly); the transforms' sums are added in f64 a piece at a time.
#[test]
fn a_run_in_pieces_is_one_runs_fields() {
    let Some(gpu) = for_tests(Precision::Single) else {
        return;
    };
    let s = busy(false);
    let whole = run_on(&s, &gpu, 400, 400, true);
    let pieces = run_on(&s, &gpu, 400, 77, true);
    assert_eq!(field_bits(&whole), field_bits(&pieces));
    for p in 0..s.probes.len() {
        assert_eq!(whole.probe(p), pieces.probe(p));
    }
    assert_eq!(whole.steps(), 400);
    assert_eq!(pieces.steps(), 400);
}

/// The fused step and the two passes compute every value by the same operations: the kernel
/// alone, with CPMLs, walls, a conductor and conductivity, within f32's rounding of each other
/// after 200 steps (measured: the same values). With sources they differ by the order M is
/// added in, within f32's rounding too.
#[test]
fn the_fused_step_is_the_two_passes() {
    let Some(gpu) = for_tests(Precision::Single) else {
        return;
    };
    let s = busy(false);
    let e =
        |s: &Simulation| -> Vec<f64> { Axis::ALL.iter().flat_map(|&c| s.e(c).to_vec()).collect() };
    let kernel = |fused: bool| {
        let mut on = s.clone();
        let mut r = Resident::new(&gpu, &on).unwrap();
        r.fused &= fused;
        assert_eq!(r.fused, fused);
        r.upload(&on).unwrap();
        r.step(&mut on, 200, false).unwrap();
        r.download(&mut on).unwrap();
        e(&on)
    };
    let r = super::checks::relative(kernel(true).into_iter(), kernel(false).into_iter());
    assert!(r < 1e-6, "the kernel alone: {r:e}");
    let fused = run_on(&s, &gpu, 200, 200, true);
    let passes = run_on(&s, &gpu, 200, 200, false);
    let r = super::checks::relative(e(&fused).into_iter(), e(&passes).into_iter());
    assert!(r < 1e-5, "with sources: {r:e}");
}

/// What the GPU doesn't step yet is refused when the device is set, and the simulation stays
/// on the CPU; a simulation reports its device, and a clone keeps it.
#[test]
fn what_the_gpu_doesnt_step_is_refused() {
    let Some(gpu) = for_tests(Precision::Single) else {
        return;
    };
    let g = grid([12, 10, 8], 0.05);
    let periodic = Edges::Bloch { k: 0.0 };
    let bloch = Boundaries {
        x: Edges::Bloch { k: 1.3 },
        y: periodic,
        z: periodic,
        ..Boundaries::cpml(2)
    };
    let mut s = Simulation::new(g, |_, _, _| 2.0, bloch, 0.9).unwrap();
    let e = s.set_device(Device::Gpu(gpu.clone())).unwrap_err();
    assert!(e.to_string().contains("Bloch"), "{e}");
    assert_eq!(s.device(), Device::Cpu);
    let drude = Dispersive {
        eps_inf: 1.0,
        poles: vec![Pole::Drude {
            plasma: Frequency::natural(1.0).unwrap(),
            damping: 0.1,
        }],
    };
    let mut s = Simulation::new(g, |_, _, _| 1.0, Boundaries::cpml(2), 0.9)
        .unwrap()
        .with_medium(&drude, |x, _, _| x > 0.0)
        .unwrap();
    let e = s.set_device(Device::Gpu(gpu.clone())).unwrap_err();
    assert!(e.to_string().contains("dispersive"), "{e}");
    let mut s = Simulation::new(g, |_, _, _| 1.0, Boundaries::cpml(2), 0.9).unwrap();
    s.set_device(Device::Gpu(gpu.clone())).unwrap();
    assert_eq!(s.device(), Device::Gpu(gpu.clone()));
    assert_eq!(s.clone().device(), Device::Gpu(gpu));
    s.set_device(Device::Cpu).unwrap();
    assert_eq!(s.device(), Device::Cpu);
}

/// Roden and Gedney's plate in soil on the GPU in f32: their errors, −48 and −67 dB "on the
/// order of", within the example's 5 dB, and within 0.05 dB of the CPU's −48.62 and −70.47
/// (the example's output).
#[test]
fn roden_and_gedneys_plate_on_the_gpu() {
    let Some(gpu) = for_tests(Precision::Single) else {
        return;
    };
    let [a, b] = super::checks::roden_gedney(&Device::Gpu(gpu));
    println!("{a} {b}");
    assert!(
        (a - -48.0).abs() < 5.0 && (b - -67.0).abs() < 5.0,
        "{a} {b}"
    );
    assert!(
        (a - -48.620087).abs() < 0.05 && (b - -70.474859).abs() < 0.05,
        "{a} {b}"
    );
}

/// The CPML's recursion in f64, ψ ← b ψ + a d with d a difference times 1/Δ, as the minimal
/// kernel of #818: `QUOTIENT` is how d is taken, and each invocation writes d − x·fl(1/Δ) to
/// `record` where it isn't zero.
const RECURSION: &str = "
@group(0) @binding(0) var<storage, read> x: array<f64>;
@group(0) @binding(1) var<storage, read> coefs: array<f64>;
@group(0) @binding(2) var<storage, read_write> psi: array<f64>;
// Δ, then 1/Δ rounded, from the host
@group(0) @binding(3) var<storage, read> reals: array<f64>;
@group(0) @binding(4) var<storage, read_write> record: array<f64>;

@compute @workgroup_size(64)
fn step(@builtin(global_invocation_id) g: vec3<u32>) {
    let i = g.x;
    let n = arrayLength(&psi);
    if (i >= n) {
        return;
    }
    let h = reals[0];
    let num = x[i + 1u] - x[i];
    let d = QUOTIENT;
    psi[i] = coefs[i] * psi[i] + coefs[n + i] * d;
    // 1/Δ rounded is 20 exactly
    if (d != num * 20.0) {
        record[i] = d - num * 20.0;
    }
}
";

/// What [`division_under_load`] saw: the runs, those whose ψ differs from the first run's,
/// the invocations whose d isn't x·fl(1/Δ) (the most in a run), and whether every such d
/// was the correctly rounded x/Δ.
#[derive(Debug)]
struct UnderLoad {
    runs: usize,
    differ: usize,
    off: usize,
    only_the_quotient: bool,
    windows: Vec<(f64, f64)>,
}

/// The recursion with d taken as `quotient`, over 65 536 values and 50 dispatches a run, run
/// after run for `seconds` on `gpu` (f64).
fn division_under_load(gpu: &Gpu, quotient: &str, seconds: f64) -> UnderLoad {
    use wgpu::util::DeviceExt;
    let c = gpu.context();
    let (device, queue) = (&c.device, &c.queue);
    let n = 65_536usize;
    let h = 0.05f64;
    assert_eq!(1.0 / h, 20.0, "the kernel's x · fl(1/Δ)");
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut unit = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let x: Vec<f64> = (0..=n).map(|_| (unit() - 0.5) * 1e-2).collect();
    let coefs: Vec<f64> = (0..2 * n)
        .map(|k| {
            if k < n {
                0.3 + 0.6 * unit()
            } else {
                -0.4 * unit()
            }
        })
        .collect();
    let bytes = |v: &[f64]| -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() };
    let init = |v: &[f64], usage: wgpu::BufferUsages| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("division under load"),
            contents: &bytes(v),
            usage,
        })
    };
    use wgpu::BufferUsages as U;
    let zeros = vec![0.0; n];
    let buffers = [
        init(&x, U::STORAGE),
        init(&coefs, U::STORAGE),
        init(&zeros, U::STORAGE | U::COPY_SRC | U::COPY_DST),
        init(&[h, 1.0 / h], U::STORAGE),
        init(&zeros, U::STORAGE | U::COPY_SRC | U::COPY_DST),
    ];
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("division under load, read back"),
        size: (16 * n) as u64,
        usage: U::MAP_READ | U::COPY_DST,
        mapped_at_creation: false,
    });
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("division under load"),
        source: wgpu::ShaderSource::Wgsl(RECURSION.replace("QUOTIENT", quotient).into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("division under load"),
        layout: None,
        module: &module,
        entry_point: Some("step"),
        compilation_options: Default::default(),
        cache: None,
    });
    let entries: Vec<wgpu::BindGroupEntry> = buffers
        .iter()
        .zip(0u32..)
        .map(|(b, binding)| wgpu::BindGroupEntry {
            binding,
            resource: b.as_entire_binding(),
        })
        .collect();
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("division under load"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries,
    });
    let run = || -> Vec<u64> {
        queue.write_buffer(&buffers[2], 0, &bytes(&zeros));
        queue.write_buffer(&buffers[4], 0, &bytes(&zeros));
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            for _ in 0..50 {
                pass.dispatch_workgroups(n.div_ceil(64) as u32, 1, 1);
            }
        }
        let size = (8 * n) as u64;
        encoder.copy_buffer_to_buffer(&buffers[2], 0, &read, 0, size);
        encoder.copy_buffer_to_buffer(&buffers[4], 0, &read, size, size);
        queue.submit([encoder.finish()]);
        read.slice(..)
            .map_async(wgpu::MapMode::Read, |r| r.unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let out = read
            .slice(..)
            .get_mapped_range()
            .unwrap()
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| u64::from_le_bytes(*b))
            .collect();
        read.unmap();
        out
    };
    // the correctly rounded x/Δ less x·fl(1/Δ), exact (they are within an ulp)
    let second: Vec<u64> = (0..n)
        .map(|i| {
            let num = x[i + 1] - x[i];
            (num / h - num * (1.0 / h)).to_bits()
        })
        .collect();
    let start = std::time::Instant::now();
    let first = run();
    let mut seen = UnderLoad {
        runs: 1,
        differ: 0,
        off: 0,
        only_the_quotient: true,
        windows: Vec::new(),
    };
    let mut open: Option<f64> = None;
    while start.elapsed().as_secs_f64() < seconds {
        let v = run();
        let t = start.elapsed().as_secs_f64();
        seen.runs += 1;
        let differs = v[..n] != first[..n];
        seen.differ += usize::from(differs);
        let off: Vec<usize> = (0..n).filter(|&i| v[n + i] != 0).collect();
        seen.off = seen.off.max(off.len());
        seen.only_the_quotient &= off.iter().all(|&i| v[n + i] == second[i]);
        match (differs, open) {
            (true, None) => open = Some(t),
            (false, Some(from)) => {
                seen.windows.push((from, t));
                open = None;
            }
            _ => {}
        }
    }
    if let Some(from) = open {
        seen.windows.push((from, start.elapsed().as_secs_f64()));
    }
    seen
}

/// The cause of f64 division's two results (#818), seen in the CPML's recursion alone: the
/// driver may run two compiled forms of a kernel and switch between them under load, and a
/// division is one thing WGSL lets the two evaluate differently (§15.7; f64 division is
/// accurate to 2.5 ulp, not correctly rounded). On an RTX 4060 (driver 616.56), d = x · (1/Δ)
/// is x · fl(1/Δ) but for 0.2 s at a time about 4 s into the load (3.8 to 4.0 and 4.6 to
/// 4.8 s, or 4.0 to 4.4 s, the same in every process), when it is the correctly rounded x/Δ
/// (19 480 of 65 536 values an ulp apart, ψ after 50 steps in 25 975 by up to 4 ulp). It fails if a quotient is ever anything else, or if the form photonoxide's
/// kernels use, d = x · inv with inv from the host, ever gives another result. By hand, on a
/// computer with a GPU (`GPU_LOAD_SECONDS`, 15 by default, each):
/// `cargo test --release --features gpu --lib under_load_a_quotient -- --ignored --nocapture`.
#[test]
#[ignore = "seconds of load on a GPU: by hand"]
fn under_load_a_quotient_takes_two_forms_a_product_one() {
    let Some(gpu) = for_tests(Precision::Double) else {
        return;
    };
    let seconds = std::env::var("GPU_LOAD_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(15.0);
    let divided = division_under_load(&gpu, "num * (1.0 / h)", seconds);
    println!(
        "{}: d = x · (1/Δ) in the kernel: {} runs, {} differ from the first, in {:.2?} s; at most {} quotients in a run are the correctly rounded x/Δ rather than x · fl(1/Δ)",
        gpu.name(),
        divided.runs,
        divided.differ,
        divided.windows,
        divided.off
    );
    assert!(
        divided.only_the_quotient,
        "a quotient was neither x · fl(1/Δ) nor x/Δ: {divided:?}"
    );
    assert_eq!(
        divided.differ > 0,
        divided.off > 0,
        "runs differ only where a quotient changes form: {divided:?}"
    );
    let multiplied = division_under_load(&gpu, "num * reals[1]", seconds);
    println!(
        "d = x · inv, inv from the host: {} runs, {} differ",
        multiplied.runs, multiplied.differ
    );
    assert_eq!(
        (multiplied.differ, multiplied.off),
        (0, 0),
        "{multiplied:?}"
    );
}
