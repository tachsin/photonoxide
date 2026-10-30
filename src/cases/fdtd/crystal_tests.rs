use super::crystal_checks::turned;
use super::{Boundaries, Field, Simulation, Source, Waveform};
use crate::fdfd::{Axis, Grid3d};
use crate::units::Frequency;

#[test]
fn the_same_three_values_are_the_scalar_problem_to_the_bit() {
    let g = Grid3d {
        nx: 14,
        ny: 12,
        nz: 10,
        dx: 0.05,
        dy: 0.05,
        dz: 0.05,
        x0: -0.35,
        y0: -0.3,
        z0: -0.25,
    };
    let eps = |x: f64, y: f64, z: f64| {
        if x * x + y * y + z * z < 0.04 {
            6.0
        } else {
            2.1
        }
    };
    let pulse = Waveform::pulse(Frequency::natural(1.0 / 1.55).unwrap(), 0.3).unwrap();
    let mut runs = [
        Simulation::new(g, eps, Boundaries::cpml(4), 0.9).unwrap(),
        Simulation::diagonal(g, |x, y, z| [eps(x, y, z); 3], Boundaries::cpml(4), 0.9).unwrap(),
    ];
    for s in &mut runs {
        s.add_source(Source {
            field: Field::E,
            component: Axis::Y,
            at: (6, 5, 4),
            waveform: pulse,
        })
        .unwrap();
        s.run(60);
    }
    for c in Axis::ALL {
        assert_eq!(runs[0].e(c), runs[1].e(c));
    }
}

#[test]
fn a_turned_crystal_on_the_mirrored_problem_gives_the_mirrored_field() {
    let d = turned(200);
    assert!(d < 1e-12, "{d:e}");
}
