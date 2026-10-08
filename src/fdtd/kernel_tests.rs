use num_complex::Complex64 as c64;

use super::checks::{grid, noise};
use super::smoothing::checks::rotation;
use super::*;

/// The bits of every value of E and H̃ (and their imaginary parts in a complex run).
fn bits(s: &Simulation) -> Vec<u64> {
    let mut out = Vec::new();
    for c in Axis::ALL {
        out.extend(s.e(c).iter().chain(s.h(c)).map(|v| v.to_bits()));
        if let (Some(e), Some(h)) = (s.e_imaginary(c), s.h_imaginary(c)) {
            out.extend(e.iter().chain(h).map(|v| v.to_bits()));
        }
    }
    out
}

/// `s` stepped `steps` times by the kernel and by the plain loops: the same bits.
fn same_bits(mut s: Simulation, steps: usize, what: &str) {
    let mut reference = s.clone();
    reference.use_reference_kernel();
    s.run(steps);
    reference.run(steps);
    let (a, b) = (bits(&s), bits(&reference));
    assert!(
        a.iter().any(|&v| f64::from_bits(v) != 0.0),
        "{what}: nothing moved"
    );
    let differ = a.iter().zip(&b).filter(|(x, y)| x != y).count();
    assert_eq!(
        differ, 0,
        "{what}: {differ} values differ from the plain loops"
    );
}

fn pulse() -> Waveform {
    Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.6).unwrap()
}

/// Random fields to start from, everywhere.
fn stir(s: &mut Simulation) {
    let n = s.grid().cells();
    for c in Axis::ALL {
        for (r, v) in s.e_mut(c).iter_mut().enumerate().take(n) {
            *v = noise(r + 3 * c.index());
        }
        for (r, v) in s.h_mut(c).iter_mut().enumerate().take(n) {
            *v = noise(r + 7 * c.index() + 50_000);
        }
    }
}

#[test]
fn the_kernel_is_the_plain_loops_bits_with_cpmls_walls_and_periodic_sides() {
    let g = grid([13, 11, 9], 0.05);
    let eps = |x: f64, y: f64, _: f64| if x > 0.05 && y < 0.1 { 4.0 } else { 1.0 };
    // CPMLs everywhere, with κ and α
    let mut boundaries = Boundaries::cpml(3);
    boundaries.cpml.kappa = 3.0;
    boundaries.cpml.alpha = 0.2;
    let mut s = Simulation::new(g, eps, boundaries, 0.9)
        .unwrap()
        .with_conductivity(|x, _, _| if x < -0.1 { 0.7 } else { 0.0 })
        .unwrap();
    stir(&mut s);
    s.add_source(Source {
        field: Field::E,
        component: Axis::Z,
        at: (6, 5, 4),
        waveform: pulse(),
    })
    .unwrap();
    same_bits(s, 40, "CPMLs");
    // walls along x, periodic along y, a CPML along z
    let boundaries = Boundaries {
        x: Edges::Pml { low: 0, high: 0 },
        y: Edges::Bloch { k: 0.0 },
        z: Edges::Pml { low: 2, high: 3 },
        cpml: Cpml::default(),
    };
    let mut s = Simulation::new(g, eps, boundaries, 0.9).unwrap();
    stir(&mut s);
    same_bits(s, 40, "walls, periodic and a CPML");
    // a 2D problem, one cell along z
    let g2 = Grid3d {
        nz: 1,
        ..grid([17, 15, 1], 0.05)
    };
    let mut s = Simulation::new(g2, eps, Boundaries::cpml_2d(4), 0.9).unwrap();
    stir(&mut s);
    same_bits(s, 40, "2D");
    // one cell along x (the rows one value long)
    let g1 = Grid3d {
        nx: 1,
        ..grid([1, 9, 12], 0.05)
    };
    let boundaries = Boundaries {
        x: Edges::Bloch { k: 0.0 },
        ..Boundaries::cpml(2)
    };
    let mut s = Simulation::new(g1, eps, boundaries, 0.9).unwrap();
    stir(&mut s);
    same_bits(s, 40, "rows of one value");
}

#[test]
fn the_kernel_is_the_plain_loops_bits_with_bloch_dispersive_and_tensor_media() {
    let g = grid([12, 10, 8], 0.05);
    // a Bloch phase: complex fields
    let boundaries = Boundaries {
        x: Edges::Bloch { k: 1.3 },
        y: Edges::Bloch { k: -0.7 },
        z: Edges::Pml { low: 2, high: 2 },
        cpml: Cpml::default(),
    };
    let mut s = Simulation::new(
        g,
        |_, y, _| if y > 0.0 { 2.0 } else { 1.0 },
        boundaries,
        0.8,
    )
    .unwrap();
    s.add_current(Current {
        field: Field::E,
        values: vec![(Axis::Y, (5, 4, 3), c64::new(1.0, 0.3))],
        waveform: pulse(),
    })
    .unwrap();
    same_bits(s, 50, "Bloch");
    // a Drude medium
    let drude = Dispersive {
        eps_inf: 1.5,
        poles: vec![Pole::Drude {
            plasma: Frequency::natural(1.2).unwrap(),
            damping: 0.05,
        }],
    };
    let mut s = Simulation::new(g, |_, _, _| 1.0, Boundaries::cpml(2), 0.8)
        .unwrap()
        .with_medium(&drude, |x, _, _| x > 0.0)
        .unwrap();
    stir(&mut s);
    same_bits(s, 40, "Drude");
    // a smoothed anisotropic ellipsoid: D stepped, E from it
    let crystal = Permittivity::principal([2.0, 3.0, 5.0], rotation([0.3, 0.7, 0.2])).unwrap();
    let ellipsoid =
        Body::ellipsoid([0.0; 3], [0.15, 0.12, 0.1], rotation([0.1, 0.4, 0.9])).unwrap();
    let structure = Structure::new(Permittivity::isotropic(1.0).unwrap()).with(ellipsoid, crystal);
    let mut s = Simulation::smoothed(
        g,
        &structure,
        Smoothing::default(),
        Boundaries::cpml(2),
        0.9,
    )
    .unwrap();
    s.add_source(Source {
        field: Field::E,
        component: Axis::X,
        at: (6, 5, 4),
        waveform: pulse(),
    })
    .unwrap();
    same_bits(s, 40, "a smoothed tensor");
}

/// A box with a dielectric block and CPMLs, random E and H̃.
fn stirred() -> Simulation {
    let g = grid([20, 18, 16], 0.05);
    let mut s = Simulation::new(
        g,
        |x, y, _| {
            if x.abs() < 0.2 && y.abs() < 0.15 {
                12.0
            } else {
                1.0
            }
        },
        Boundaries::cpml(4),
        0.9,
    )
    .unwrap();
    stir(&mut s);
    s
}

#[test]
fn the_kernel_alone_in_f64_is_the_simulations_bits() {
    let s = stirred();
    let mut yee = Yee::<f64>::from_simulation(&s).unwrap();
    let mut s = s;
    for _ in 0..60 {
        yee.step();
    }
    s.run(60);
    for c in Axis::ALL {
        assert!(
            yee.e(c)
                .iter()
                .zip(s.e(c))
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
    }
}

#[test]
fn the_kernel_in_f32_rounds_slowly_and_is_the_same_bits_on_any_number_of_threads() {
    let s = stirred();
    let run = |threads: usize, steps: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let mut yee = Yee::<f32>::from_simulation(&s).unwrap();
                for _ in 0..steps {
                    yee.step();
                }
                yee
            })
    };
    let one = run(1, 100);
    for threads in [4, 20] {
        let other = run(threads, 100);
        for c in Axis::ALL {
            assert!(
                one.e(c)
                    .iter()
                    .zip(other.e(c))
                    .all(|(a, b)| a.to_bits() == b.to_bits()),
                "{threads} threads"
            );
        }
    }
    // against f64: the error relative to the largest E, after 100, 400 and 1600 steps
    let mut double = Yee::<f64>::from_simulation(&s).unwrap();
    let mut single = Yee::<f32>::from_simulation(&s).unwrap();
    let mut errors = Vec::new();
    for steps in [100, 300, 1200] {
        for _ in 0..steps {
            double.step();
            single.step();
        }
        let scale = Axis::ALL
            .iter()
            .flat_map(|&c| double.e(c).iter())
            .fold(0.0f64, |m, v| m.max(v.abs()));
        let worst = Axis::ALL
            .iter()
            .flat_map(|&c| single.e(c).iter().zip(double.e(c)))
            .fold(0.0f64, |m, (a, b)| m.max((f64::from(*a) - b).abs()));
        errors.push(worst / scale);
    }
    // measured 1.8e-6, 4.5e-6 and 1.5e-5 on random fields: about 1e-8 a step, linear in the
    // steps
    for (e, steps) in errors.iter().zip([100.0, 400.0, 1600.0]) {
        assert!(*e < 2e-8 * steps, "{errors:?}");
    }
}
