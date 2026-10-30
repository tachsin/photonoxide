use num_complex::Complex64 as c64;

use super::crystal_checks::{slab_errors, slab_index, turned_2d, turned_3d};
use super::{Boundaries, Grid, Polarization, Solver2d};
use crate::material::CrystalAxis;
use crate::mode::Polarization as Kind;
use crate::units::Wavelength;

#[test]
fn the_same_three_values_are_the_scalar_problem_to_the_bit() {
    let grid = Grid {
        nx: 30,
        ny: 24,
        dx: 0.05,
        dy: 0.05,
        x0: -0.75,
        y0: -0.6,
    };
    let eps = |x: f64, y: f64| c64::new(if x * x + y * y < 0.1 { 12.0 } else { 2.1 }, 0.0);
    let mut source = vec![c64::new(0.0, 0.0); 30 * 24];
    source[12 * 30 + 9] = c64::new(1.0, 0.0);
    for p in [Polarization::Ez, Polarization::Hz] {
        let w = Wavelength::um(1.55).unwrap();
        let a = Solver2d::new(grid, p, w, eps, Boundaries::pml(6)).unwrap();
        let b = Solver2d::diagonal(grid, p, w, |x, y| [eps(x, y); 3], Boundaries::pml(6)).unwrap();
        assert_eq!(
            a.solve(&source).unwrap().values,
            b.solve(&source).unwrap().values
        );
    }
}

#[test]
fn a_turned_crystal_on_the_mirrored_problem_gives_the_mirrored_field() {
    let d2 = turned_2d();
    assert!(d2 < 1e-10, "{d2:e}");
    let d3 = turned_3d();
    assert!(d3 < 1e-9, "{d3:e}");
}

#[test]
fn a_cut_slabs_port_mode_sees_its_tensor_and_converges_at_second_order() {
    // x-cut: TE sees n_e, TM n_o; z-cut: TE n_o, TM the uniaxial slab's
    for (cut, kind) in [
        (CrystalAxis::X, Kind::Te),
        (CrystalAxis::X, Kind::Tm),
        (CrystalAxis::Z, Kind::Te),
        (CrystalAxis::Z, Kind::Tm),
    ] {
        let (found, exact) = slab_index(cut, kind, 0.01);
        assert!(
            (found - exact).abs() < 1e-3,
            "{cut:?} {kind:?}: {found} {exact}"
        );
    }
    let (errors, orders) = slab_errors(CrystalAxis::Z, Kind::Tm, &[0.02, 0.01]);
    assert!(errors[1] < errors[0], "{errors:?}");
    assert!(orders[0] > 1.7, "{orders:?} {errors:?}");
}
