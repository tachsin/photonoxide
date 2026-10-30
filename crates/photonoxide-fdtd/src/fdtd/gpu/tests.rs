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
