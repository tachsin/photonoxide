//! The GPU against the CPU, as numbers: for the tests and the GPU's validation report.

use num_complex::Complex64 as c64;

use super::{Gpu, Resident};
use crate::fdfd::{Axis, Edges};
use crate::fdtd::Yee;
use crate::fdtd::checks::{grid, noise};
use crate::fdtd::{
    Boundaries, Cpml, Current, Device, Dipole, Field, FluxPlane, Simulation, Source, Waveform,
};
use crate::units::Frequency;

/// The largest difference of `a` from `b`, relative to the largest of `b`.
pub(crate) fn relative(a: impl Iterator<Item = f64>, b: impl Iterator<Item = f64>) -> f64 {
    let (mut worst, mut scale) = (0.0f64, 0.0f64);
    for (x, y) in a.zip(b) {
        worst = worst.max((x - y).abs());
        scale = scale.max(y.abs());
    }
    worst / scale
}

/// The pulse the sources share.
fn pulse() -> Waveform {
    Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.6).unwrap()
}

/// A problem with what the GPU steps: a dielectric block and a lossy one, a conductor, CPMLs
/// with κ and α, a wall and (in 2D) periodic sides, random fields to start from, point sources
/// on E and on H̃ and a dipole's current sharing a value with one of them, probes, a transform
/// box and a flux box. 3D: 37 × 21 × 19 cells of 50 nm, sizes that leave workgroups part-filled;
/// CPMLs of 4 and 5 along x, 4 and a wall along y, 3 and 3 along z. 2D: 70 × 45 cells, periodic
/// along x, CPMLs of 6 along y.
pub(crate) fn busy(two_d: bool) -> Simulation {
    let (g, boundaries) = if two_d {
        let p = Edges::Bloch { k: 0.0 };
        let b = Boundaries {
            x: p,
            y: Edges::Pml { low: 6, high: 6 },
            z: p,
            cpml: Cpml {
                kappa: 2.0,
                alpha: 0.3,
                ..Cpml::default()
            },
        };
        (grid([70, 45, 1], 0.05), b)
    } else {
        let b = Boundaries {
            x: Edges::Pml { low: 4, high: 5 },
            y: Edges::Pml { low: 4, high: 0 },
            z: Edges::Pml { low: 3, high: 3 },
            cpml: Cpml {
                kappa: 3.0,
                alpha: 0.2,
                ..Cpml::default()
            },
        };
        (grid([37, 21, 19], 0.05), b)
    };
    let mut s = Simulation::new(
        g,
        |x, y, _| {
            if x.abs() < 0.3 && y.abs() < 0.2 {
                12.0
            } else {
                1.0
            }
        },
        boundaries,
        0.9,
    )
    .unwrap()
    .with_conductivity(|x, y, _| if x > 0.6 && y > 0.3 { 2.0 } else { 0.0 })
    .unwrap();
    let n = g.cells();
    for c in Axis::ALL {
        for (r, v) in s.e_mut(c).iter_mut().enumerate().take(n) {
            *v = 0.01 * noise(r + 3 * c.index());
        }
        for (r, v) in s.h_mut(c).iter_mut().enumerate().take(n) {
            *v = 0.01 * noise(r + 7 * c.index() + 50_000);
        }
    }
    let (nx, ny, nz) = (g.nx, g.ny, g.nz);
    let mid = (nx / 2, ny / 2, nz / 2);
    s.conductor(Axis::X, (nx / 3, ny / 3, nz / 2));
    s.add_source(Source {
        field: Field::E,
        component: Axis::Z,
        at: mid,
        waveform: pulse(),
    })
    .unwrap();
    s.add_source(Source {
        field: Field::H,
        component: Axis::X,
        at: (nx / 4, ny / 2, nz / 2),
        waveform: Waveform::DifferentiatedGaussian {
            width: 0.3,
            delay: 1.2,
        },
    })
    .unwrap();
    // a current on the first source's value too
    s.add_current(Current {
        field: Field::E,
        values: vec![
            (Axis::Z, mid, c64::new(0.5, -0.25)),
            (Axis::Y, (nx / 2 + 3, ny / 2, nz / 2), c64::new(-0.3, 0.7)),
        ],
        waveform: pulse(),
    })
    .unwrap();
    let centre = [g.x0, g.y0, g.z0]
        .iter()
        .zip([nx, ny, nz])
        .map(|(o, n)| o + 0.5 * n as f64 * 0.05)
        .collect::<Vec<_>>();
    s.add_dipole(Dipole {
        field: Field::H,
        component: Axis::Z,
        position: [centre[0] + 0.33, centre[1] - 0.21, centre[2]],
        amplitude: c64::new(1.0, 0.5),
        waveform: pulse(),
    })
    .unwrap();
    for at in [(3, 3, 0), (nx - 8, ny / 3, nz / 3), mid] {
        s.add_probe(Field::E, Axis::Z, at).unwrap();
        s.add_probe(Field::H, Axis::Y, at).unwrap();
    }
    let frequencies = [0.8, 1.0, 1.3].map(|f| Frequency::natural(f).unwrap());
    let low = (nx / 2 - 6, ny / 2 - 4, 0);
    let high = (nx / 2 + 6, ny / 2 + 4, nz - 1);
    s.add_dft(low, high, &frequencies).unwrap();
    if two_d {
        s.add_flux(FluxPlane::new(Axis::Y, ny - 12), &frequencies)
            .unwrap();
    } else {
        let box_ = |n: usize| (n / 2 - 5, n / 2 + 5);
        s.add_flux_box([box_(nx), box_(ny), box_(nz)], &frequencies)
            .unwrap();
    }
    s
}

/// How far a GPU's run is from the CPU's, each relative to the CPU's largest value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Agreement {
    /// E and H̃.
    pub(crate) fields: f64,
    /// The probes' values over the run.
    pub(crate) probes: f64,
    /// Every transform's sums.
    pub(crate) transforms: f64,
    /// The flux monitors' fluxes.
    pub(crate) flux: f64,
}

/// Everything a run leaves: the fields, the probes, the transforms and the fluxes, as numbers.
fn outputs(s: &Simulation) -> [Vec<f64>; 4] {
    let fields = Axis::ALL
        .iter()
        .flat_map(|&c| s.e(c).iter().chain(s.h(c)).copied())
        .collect();
    let probes = (0..s.probes.len())
        .flat_map(|p| s.probe(p).to_vec())
        .collect();
    let transforms = s
        .monitors
        .transforms()
        .flatten()
        .flat_map(|v| [v.re, v.im])
        .collect();
    let flux = (0..s.monitors.fluxes.len())
        .flat_map(|n| s.flux(n))
        .collect();
    [fields, probes, transforms, flux]
}

/// `s` run `steps` steps on the CPU (f64) and on `gpu`, in runs of `piece` steps on the GPU,
/// by the fused step if `fused` (where it fits) or by two passes.
pub(crate) fn agreement(
    s: &Simulation,
    gpu: &Gpu,
    steps: usize,
    piece: usize,
    fused: bool,
) -> Agreement {
    let mut cpu = s.clone();
    cpu.run(steps);
    let on = run_on(s, gpu, steps, piece, fused);
    let [a, b] = [outputs(&on), outputs(&cpu)];
    let r = |n: usize| relative(a[n].iter().copied(), b[n].iter().copied());
    Agreement {
        fields: r(0),
        probes: r(1),
        transforms: r(2),
        flux: r(3),
    }
}

/// `s` run `steps` steps on `gpu`, in runs of `piece` steps, fused if `fused` (where it fits).
pub(crate) fn run_on(
    s: &Simulation,
    gpu: &Gpu,
    steps: usize,
    piece: usize,
    fused: bool,
) -> Simulation {
    let mut on = s.clone();
    on.set_device(Device::Gpu(gpu.clone())).unwrap();
    if let Some(r) = on.gpu.as_mut().and_then(|g| g.resident.as_mut()) {
        r.fused &= fused;
    }
    let mut left = steps;
    while left > 0 {
        let n = left.min(piece);
        on.run(n);
        left -= n;
    }
    on
}

/// Whether two runs of `s` on `gpu` leave the same bits: fields, probes and transforms.
pub(crate) fn repeats(s: &Simulation, gpu: &Gpu, steps: usize, fused: bool) -> bool {
    let bits =
        |s: &Simulation| -> Vec<u64> { outputs(s).iter().flatten().map(|v| v.to_bits()).collect() };
    let one = bits(&run_on(s, gpu, steps, steps, fused));
    let two = bits(&run_on(s, gpu, steps, steps, fused));
    one == two
}

/// The kernel alone, E, H̃ and the CPMLs of `s`, stepped on `gpu` and on the CPU in f32 and in
/// f64, compared after each of `checkpoints` steps (counted from the start): E's largest
/// differences relative to its largest value, [GPU against the CPU in the GPU's precision, GPU
/// against the CPU in f64, the CPU's f32 against its f64].
pub(crate) fn kernel_against_cpu(
    s: &Simulation,
    gpu: &Gpu,
    checkpoints: &[usize],
    fused: bool,
) -> Vec<[f64; 3]> {
    let mut on = s.clone();
    let mut resident = Resident::new(gpu, &on).unwrap();
    resident.fused &= fused;
    resident.upload(&on).unwrap();
    let mut double = Yee::<f64>::from_simulation(s).unwrap();
    let mut single = Yee::<f32>::from_simulation(s).unwrap();
    let e = |y: &dyn Fn(Axis) -> Vec<f64>| Axis::ALL.iter().flat_map(|&c| y(c)).collect::<Vec<_>>();
    let mut done = 0;
    let mut out = Vec::new();
    for &steps in checkpoints {
        resident.step(&mut on, steps - done, false).unwrap();
        resident.download(&mut on).unwrap();
        for _ in done..steps {
            double.step();
            single.step();
        }
        done = steps;
        let d = e(&|c| double.e(c).to_vec());
        let f = e(&|c| single.e(c).iter().map(|&v| f64::from(v)).collect());
        let g = e(&|c| on.e(c).to_vec());
        let same = if gpu.precision() == super::Precision::Single {
            &f
        } else {
            &d
        };
        out.push([
            relative(g.iter().copied(), same.iter().copied()),
            relative(g.iter().copied(), d.iter().copied()),
            relative(f.iter().copied(), d.iter().copied()),
        ]);
    }
    out
}

/// A closed box of 40 × 36 × 32 cells of 50 nm with a block of ε = 12, random fields: nothing
/// leaves it, so rounding has the whole run to grow.
pub(crate) fn closed() -> Simulation {
    let mut s = Simulation::new(
        grid([40, 36, 32], 0.05),
        |x, y, _| {
            if x.abs() < 0.3 && y.abs() < 0.2 {
                12.0
            } else {
                1.0
            }
        },
        Boundaries::walls(),
        0.9,
    )
    .unwrap();
    let n = s.grid().cells();
    for c in Axis::ALL {
        for (r, v) in s.e_mut(c).iter_mut().enumerate().take(n) {
            *v = noise(r + 3 * c.index());
        }
        for (r, v) in s.h_mut(c).iter_mut().enumerate().take(n) {
            *v = noise(r + 7 * c.index() + 50_000);
        }
    }
    s
}

/// Roden and Gedney's plate in soil, as the `cpml_roden_gedney` example sets it up (see there):
/// the largest error of E_z one cell above the plate's far corner over 2000 steps, in dB of the
/// reference's largest value, with their traditional PML and with their CFS-PML, every run on
/// `device`.
pub(crate) fn roden_gedney(device: &Device) -> [f64; 2] {
    use crate::fdfd::Grid3d;
    const MM: f64 = 1000.0;
    const ETA0: f64 = 376.730_313_668;
    const C: f64 = 299_792_458.0;
    const EPS: f64 = 7.73;
    let sigma = 0.273 * ETA0 * 1e-6;
    let run = |pad: usize, cpml: Cpml| -> Vec<f64> {
        let (pml, gap) = (10, 3);
        let edge = pml + gap + pad;
        let n = [100 + 2 * edge, 25 + 2 * edge, 2 * edge];
        let grid = Grid3d {
            nx: n[0],
            ny: n[1],
            nz: n[2],
            dx: MM,
            dy: MM,
            dz: MM,
            x0: 0.0,
            y0: 0.0,
            z0: 0.0,
        };
        let walls = Edges::Pml {
            low: pml,
            high: pml,
        };
        let boundaries = Boundaries {
            x: walls,
            y: walls,
            z: walls,
            cpml,
        };
        let mut s = Simulation::new(grid, |_, _, _| EPS, boundaries, 0.99)
            .unwrap()
            .with_conductivity(|_, _, _| sigma)
            .unwrap();
        let plate = n[2] / 2;
        s.plate(Axis::Z, plate, edge..edge + 100, edge..edge + 25);
        let tau = 0.72 / (std::f64::consts::PI * 6e9) * C * 1e6;
        s.add_source(Source {
            field: Field::E,
            component: Axis::Z,
            at: (edge, edge, plate),
            waveform: Waveform::DifferentiatedGaussian {
                width: tau,
                delay: 4.0 * tau,
            },
        })
        .unwrap();
        let probe = s
            .add_probe(Field::E, Axis::Z, (edge + 100, edge + 25, plate))
            .unwrap();
        s.set_device(device.clone()).unwrap();
        s.run(2000);
        s.probe(probe).to_vec()
    };
    let error_db = |field: &[f64], reference: &[f64]| {
        let largest = reference.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let worst = field
            .iter()
            .zip(reference)
            .fold(0.0f64, |m, (a, b)| m.max((a - b).abs()));
        20.0 * (worst / largest).log10()
    };
    let m = 4.0;
    let optimal = Cpml::sigma_optimal(m, EPS, MM);
    let traditional = Cpml {
        sigma: Some(0.7 * optimal),
        order: m,
        kappa: 11.0,
        alpha: 0.0,
        ..Cpml::default()
    };
    let shifted = Cpml {
        sigma: Some(1.1 * optimal),
        order: m,
        kappa: 7.0,
        alpha: 0.05 * ETA0 * 1e-6,
        ..Cpml::default()
    };
    let reference = run(40, shifted);
    [
        error_db(&run(0, traditional), &reference),
        error_db(&run(0, shifted), &reference),
    ]
}
