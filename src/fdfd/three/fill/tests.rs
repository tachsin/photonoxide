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

/// The iterative solver built from a structure has the direct one's permittivity to the bit,
/// with the matrix and without, refuses the tensor as it does, and solves the same system.
#[test]
fn the_iterative_solver_from_a_structure_is_the_direct_ones_system() {
    let s = strip();
    let lam = Wavelength::um(1.55).unwrap();
    let g = grid();
    let b = Boundaries3d::pml(0);
    for a in [Averaging::Samples, Averaging::Diagonal] {
        let direct = Solver3d::from_structure(g, lam, &s, b, a).unwrap();
        for (formulation, product) in [
            (Formulation::CurlCurl, Product::Matrix),
            (Formulation::ShinFan, Product::MatrixFree),
        ] {
            let it =
                IterativeSolver3d::from_structure(g, lam, &s, b, a, formulation, product).unwrap();
            assert_eq!(it.values(), direct.values(), "{a:?} {product:?}");
            assert_eq!(it.product(), product);
        }
    }
    let Err(e) = IterativeSolver3d::from_structure(
        g,
        lam,
        &s,
        b,
        Averaging::Tensor,
        Formulation::ShinFan,
        Product::MatrixFree,
    ) else {
        panic!("the tensor is refused");
    };
    assert!(e.to_string().contains("diagonal"), "{e}");
    // a current in the strip: QMR to 1e-12 gives the direct solver's field
    let mut current = vec![c64::new(0.0, 0.0); g.unknowns()];
    current[g.index(Axis::Y, (2, 7, 6))] = c64::new(1.0, 0.0);
    let want = Solver3d::from_structure(g, lam, &s, b, Averaging::Diagonal)
        .unwrap()
        .solve(&current)
        .unwrap();
    let (got, _) = IterativeSolver3d::from_structure(
        g,
        lam,
        &s,
        b,
        Averaging::Diagonal,
        Formulation::CurlCurl,
        Product::Matrix,
    )
    .unwrap()
    .solve(
        &current,
        crate::fdfd::Stopping {
            tolerance: 1e-12,
            max_iterations: 100_000,
        },
    )
    .unwrap();
    let largest = want.values().iter().map(|v| v.norm()).fold(0.0, f64::max);
    let worst = want
        .values()
        .iter()
        .zip(got.values())
        .map(|(w, v)| (w - v).norm())
        .fold(0.0, f64::max);
    assert!(worst < 1e-9 * largest, "{worst:e} of {largest:e}");
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

/// A strip drawn in pieces, abutting and at angles, gives every value of E what the strip drawn
/// whole gives, to rounding, slice by slice: its seams are no walls.
#[test]
fn a_strip_in_pieces_is_the_strip_whole_in_every_slice() {
    let (whole, cuts) = crate::averaging::checks::strip_pieces();
    let lam = Wavelength::um(1.55).unwrap();
    let g = Grid3d {
        nx: 21,
        ny: 12,
        nz: 10,
        dx: 0.1,
        dy: 0.1,
        dz: 0.055,
        x0: -1.05,
        y0: -0.6,
        z0: 1.87,
    };
    let of = |pieces: &[Shape]| {
        let mut s = Structure::new(crate::stack::LayerStack::soi_220());
        for p in pieces {
            s.draw("Si", p.clone()).unwrap();
        }
        s
    };
    let one = of(std::slice::from_ref(&whole));
    for a in [Averaging::Mean, Averaging::Diagonal] {
        let want = structure_values(g, lam, &one, a).unwrap();
        for pieces in &cuts {
            let got = structure_values(g, lam, &of(pieces), a).unwrap();
            let worst = want
                .iter()
                .zip(&got)
                .map(|(w, v)| (w - v).norm())
                .fold(0.0, f64::max);
            assert!(worst < 1e-12, "{a:?}, {} pieces: {worst:e}", pieces.len());
        }
    }
}
