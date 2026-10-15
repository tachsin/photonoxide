use super::*;
use crate::geometry::Shape;
use crate::stack::LayerStack;

fn strip() -> Structure {
    let mut s = Structure::new(LayerStack::soi_220());
    let wg = Shape::rect(Point::um(0.0, 0.013), Length::um(10.0), Length::um(0.5)).unwrap();
    s.draw("Si", wg).unwrap();
    s
}

fn grid() -> Grid3d {
    Grid3d {
        nx: 4,
        ny: 14,
        nz: 12,
        dx: 0.06,
        dy: 0.06,
        dz: 0.05,
        x0: 0.0,
        y0: -0.42,
        z0: 1.81,
    }
}

#[test]
fn samples_from_a_structure_are_its_permittivity_function_to_the_bit() {
    let s = strip();
    let lam = Wavelength::um(1.55).unwrap();
    let g = grid();
    let b = Boundaries3d::pml(0);
    let a = Solver3d::from_structure(g, lam, &s, b, Averaging::Samples).unwrap();
    let eps = |m: &Material| m.permittivity(lam).unwrap();
    let f = |x: f64, y: f64, z: f64| eps(s.material_at(Point::um(x, y), Length::um(z)));
    let n = Solver3d::new(g, lam, f, b).unwrap();
    assert_eq!(a.values(), n.values());
}

#[test]
fn the_fills_values_are_the_same_on_any_number_of_threads_and_the_tensor_is_refused() {
    let s = strip();
    let lam = Wavelength::um(1.55).unwrap();
    let on = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| structure_values(grid(), lam, &s, Averaging::Diagonal).unwrap())
    };
    assert_eq!(on(1), on(5));
    let e = structure_values(grid(), lam, &s, Averaging::Tensor)
        .unwrap_err()
        .to_string();
    assert!(e.contains("diagonal"), "{e}");
}

/// A value of E whose cell lies inside one material is that material's permittivity, to the
/// bit, by every averaging.
#[test]
fn a_uniform_cell_is_its_material() {
    let s = strip();
    let lam = Wavelength::um(1.55).unwrap();
    let g = grid();
    let si = crate::material::silicon().permittivity(lam).unwrap();
    for a in [Averaging::Mean, Averaging::Diagonal] {
        let v = structure_values(g, lam, &s, a).unwrap();
        // E_x at (i, 7, 6): y = 0, z = 2.11, deep in the strip
        assert_eq!(v[g.index(Axis::X, (1, 7, 6))], si);
    }
}
