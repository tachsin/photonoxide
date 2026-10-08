use super::super::checks::grid;
use super::super::{Field, Source, Waveform};
use super::checks::*;
use super::*;
use crate::geometry::{Point, Polygon};
use crate::units::{Frequency, Length};

fn smoothing(average: Average, coupling: Coupling) -> Smoothing {
    Smoothing {
        average,
        diameter: 1.0,
        coupling,
    }
}

#[test]
fn the_share_a_plane_cuts_off_a_cell_is_exact() {
    // against counting points of a fine lattice in the cell, and the complement's share
    let size = [1.0, 0.7, 1.3];
    for (n, d) in [
        ([0.3, 0.5, 0.2], 0.1),
        ([0.8, -0.1, 0.05], -0.2),
        ([0.0, 0.6, 0.8], 0.05),
        ([1e-9, 0.6, -0.8], 0.05),
        ([1.0, 1.0, 1.0], 0.3),
        ([0.0, 0.0, -1.0], 0.21),
    ] {
        let n = n.map(|v| v / norm(n));
        let m = 120;
        let at = |s: usize, a: usize| ((s as f64 + 0.5) / m as f64 - 0.5) * size[a];
        let mut count = 0usize;
        for a in 0..m {
            for b in 0..m {
                for c in 0..m {
                    if n[0] * at(a, 0) + n[1] * at(b, 1) + n[2] * at(c, 2) + d < 0.0 {
                        count += 1;
                    }
                }
            }
        }
        let counted = count as f64 / (m * m * m) as f64;
        let f = fill(n, d, size);
        // the lattice's own error: a plane crosses about 3m of its m³ points' cells
        assert!(
            (f - counted).abs() < 3.0 / m as f64 * 0.5,
            "{n:?} {d}: {f} {counted}"
        );
        let flipped = fill(n.map(|v| -v), -d, size);
        assert!((f + flipped - 1.0).abs() < 1e-14, "{f} + {flipped}");
    }
    // a plane through the centre halves the cell; one beyond it leaves nothing or all
    assert!((fill([0.6, 0.8, 0.0], 0.0, size) - 0.5).abs() < 1e-15);
    assert_eq!(fill([1.0, 0.0, 0.0], 0.6, size), 0.0);
    assert_eq!(fill([1.0, 0.0, 0.0], -0.6, size), 1.0);
    // a corner cut off a unit square: a right triangle of legs 0.2/sin and 0.2/cos
    let n = [0.6, 0.8, 0.0];
    let d = 0.5 * (0.6 + 0.8) - 0.2;
    assert!((fill(n, d, [1.0, 1.0, 1.0]) - 0.2 * 0.2 / (2.0 * 0.6 * 0.8)).abs() < 1e-15);
}

#[test]
fn smoothing_isotropic_media_is_farjadpours_eq_1() {
    // ε̃⁻¹ = P⟨ε⁻¹⟩ + (1 − P)⟨ε⟩⁻¹ across an oblique plane
    let n = [0.4f64.cos() * 0.8, 0.4f64.sin() * 0.8, 0.6];
    let body = Body::half_space([0.013, -0.02, 0.0], n).unwrap();
    let s = Structure::new(Permittivity::isotropic(1.0).unwrap())
        .with(body, Permittivity::isotropic(12.0).unwrap());
    let size = [0.1, 0.08, 0.12];
    let m = s.inverse([0.0, 0.0, 0.0], size, Average::Subpixel);
    let f = fill(n, -dot(n, [0.013, -0.02, 0.0]), size);
    let (harmonic, arithmetic) = (f / 12.0 + (1.0 - f), f * 12.0 + (1.0 - f));
    for i in 0..3 {
        for j in 0..3 {
            let p = n[i] * n[j];
            let identity = if i == j { 1.0 } else { 0.0 };
            let expected = p * harmonic + (identity - p) / arithmetic;
            assert!(
                (m[i][j] - expected).abs() < 1e-14,
                "{i}{j}: {} {expected}",
                m[i][j]
            );
        }
    }
    // the mean and the harmonic mean, for every component
    let mean = s.inverse([0.0, 0.0, 0.0], size, Average::Mean);
    let inverse_mean = s.inverse([0.0, 0.0, 0.0], size, Average::InverseMean);
    assert!((mean[0][0] - 1.0 / arithmetic).abs() < 1e-15 && mean[0][1] == 0.0);
    assert!((inverse_mean[2][2] - harmonic).abs() < 1e-15 && inverse_mean[1][2] == 0.0);
}

#[test]
fn smoothing_anisotropic_media_averages_tau_across_the_interface() {
    // across a plane normal to z, τ's blocks average: ⟨1/ε_zz⟩ = 1/ε̃_zz, ⟨ε_tz/ε_zz⟩ and
    // ⟨ε_t − ε_tz ε_zt/ε_zz⟩ (the transverse tensor E_x, E_y see when D_z is fixed)
    let a = tilted_crystal();
    let b = Permittivity::principal([9.0, 7.5, 11.0], rotation([1.0, 0.3, 0.2])).unwrap();
    let body = Body::half_space([0.0, 0.0, 0.0137], [0.0, 0.0, 1.0]).unwrap();
    let s = Structure::new(b).with(body, a);
    let size = [0.05; 3];
    let eps = inverse(&s.inverse([0.0, 0.0, 0.0], size, Average::Subpixel));
    let f = 0.5 + 0.0137 / 0.05;
    let (ea, eb, e) = (a.matrix(), b.matrix(), eps);
    let average = |g: &dyn Fn(&Matrix) -> f64| f * g(&ea) + (1.0 - f) * g(&eb);
    assert!((1.0 / e[2][2] - average(&|m| 1.0 / m[2][2])).abs() < 1e-13);
    for i in 0..2 {
        assert!((e[i][2] / e[2][2] - average(&|m| m[i][2] / m[2][2])).abs() < 1e-13);
        for j in 0..2 {
            let t = |m: &Matrix| m[i][j] - m[i][2] * m[2][j] / m[2][2];
            assert!((t(&e) - average(&t)).abs() < 1e-12, "{i}{j}");
        }
    }
    // and a uniform medium is itself
    let uniform = inverse(&Structure::new(a).inverse([0.0; 3], size, Average::Subpixel));
    for (row, expected) in uniform.iter().zip(a.matrix()) {
        for (v, e) in row.iter().zip(expected) {
            assert!((v - e).abs() < 1e-14);
        }
    }
}

#[test]
fn a_permittivity_must_be_symmetric_and_positive_definite() {
    assert!(Permittivity::new([[2.0, 0.1, 0.0], [0.2, 2.0, 0.0], [0.0, 0.0, 2.0]]).is_err());
    assert!(Permittivity::new([[1.0, 2.0, 0.0], [2.0, 1.0, 0.0], [0.0, 0.0, 1.0]]).is_err());
    assert!(Permittivity::isotropic(0.0).is_err());
    assert!(Permittivity::isotropic(f64::NAN).is_err());
    assert!(
        Permittivity::principal(
            [1.0, 2.0, 3.0],
            [[1.0, 0.1, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        )
        .is_err()
    );
    assert!(Permittivity::uniaxial(4.8, 4.6, [0.0; 3]).is_err());
    // LiNbO₃-like: n_o² across the axis, n_e² along it
    let u = Permittivity::uniaxial(4.8, 4.6, [1.0, 1.0, 0.0])
        .unwrap()
        .matrix();
    assert!((u[0][0] - 4.7).abs() < 1e-15 && (u[0][1] + 0.1).abs() < 1e-15 && u[2][2] == 4.8);
    assert!(Body::half_space([0.0; 3], [0.0; 3]).is_err());
    assert!(Body::ellipsoid([0.0; 3], [1.0, -1.0, 1.0], rotation([0.0; 3])).is_err());
    assert!(Body::ellipsoid([0.0; 3], [f64::INFINITY; 3], rotation([0.0; 3])).is_err());
}

#[test]
fn the_planar_shapes_fill_their_cells_to_their_area() {
    // Σ of each cell's share over a grid is the shape's area: to h² for each corner, and to
    // second order where the boundary curves (the plane through the nearest point is outside it)
    let um = Length::um;
    let polygon = Polygon::new(vec![
        Point::um(-0.4, -0.3),
        Point::um(0.5, -0.2),
        Point::um(0.1, 0.45),
    ])
    .unwrap();
    for (shape, tolerance) in [
        (
            Shape::rect(Point::um(0.03, -0.01), um(0.61), um(0.37)).unwrap(),
            4e-4,
        ),
        (
            Shape::circle(Point::um(0.02, 0.01), um(0.37)).unwrap(),
            1e-4,
        ),
        (
            Shape::ring(Point::um(0.0, 0.0), um(0.3), um(0.13)).unwrap(),
            2e-4,
        ),
        (Shape::Polygon(polygon), 3e-4),
    ] {
        let area = shape.area();
        let body = Body::extruded(shape);
        let h = 0.01;
        let mut sum = 0.0;
        for i in -60..60 {
            for j in -60..60 {
                let p = [(i as f64 + 0.5) * h, (j as f64 + 0.5) * h, 0.0];
                let (d, n) = body.surface(p);
                sum += fill(n, d, [h; 3]) * h * h;
            }
        }
        assert!((sum - area).abs() < tolerance, "{sum} {area}");
    }
}

#[test]
fn an_ellipsoids_distance_is_exact_for_a_sphere_and_second_order_otherwise() {
    let sphere = Body::ellipsoid([0.1, 0.2, 0.3], [0.5; 3], rotation([0.4, 0.5, 0.6])).unwrap();
    let (d, n) = sphere.surface([0.1 + 0.6, 0.2, 0.3]);
    assert!((d - 0.1).abs() < 1e-15 && (n[0] - 1.0).abs() < 1e-15);
    // an ellipse 0.2 × 0.1: at δ off its end along the long axis, the distance is δ(1 + O(δ))
    let ellipse = Body::ellipse([0.0, 0.0], [0.2, 0.1], 0.0).unwrap();
    for delta in [0.01, 0.005] {
        let (d, _) = ellipse.surface([0.2 + delta, 0.0, 7.0]);
        assert!((d - delta).abs() < 2.0 * delta * delta / 0.1, "{d} {delta}");
    }
}

#[test]
fn a_slabs_reflection_at_any_offset_is_second_order_with_subpixel_smoothing() {
    // faces off the grid: Airy's formula, second order; sampled, an error of the order of the
    // cell, erratic
    let eps = Permittivity::isotropic(4.0).unwrap();
    let subpixel = Smoothing::default();
    let (coarse, fine) = (
        slab_error(1.0 / 40.0, eps, subpixel),
        slab_error(1.0 / 80.0, eps, subpixel),
    );
    assert!(coarse / fine > 3.5 && fine < 4e-3, "{coarse} {fine}");
    let sampled = slab_error(
        1.0 / 80.0,
        eps,
        smoothing(Average::Sampled, Coupling::Nodes),
    );
    assert!(sampled > 5.0 * fine, "{sampled}");
}

#[test]
fn an_anisotropic_slabs_reflection_matrix_is_the_exact_one() {
    // every entry of the tensor non-zero: r_xx and r_yx against the slab along the principal
    // axes of its transverse tensor, second order
    let eps = tilted_crystal();
    let exact = slab_exact(eps);
    assert!(exact.iter().all(|r| r[1].norm() > 0.03), "{exact:?}");
    let (coarse, fine) = (
        slab_error(1.0 / 40.0, eps, Smoothing::default()),
        slab_error(1.0 / 80.0, eps, Smoothing::default()),
    );
    assert!(coarse / fine > 3.0 && fine < 2e-3, "{coarse} {fine}");
}

#[test]
fn an_oblique_interface_is_second_order_with_the_off_diagonal_entries_at_the_points() {
    // layers at 26.6° to the grid, the mode's E across them: Farjadpour et al.'s placement
    // converges as h²; Werner and Cary's, at the nodes, as h, like the mean
    let points = smoothing(Average::Subpixel, Coupling::Points);
    let (e32, e64) = (oblique(points, 32), oblique(points, 64));
    assert!(e32 / e64 > 3.0 && e64.abs() < 1e-3, "{e32} {e64}");
    let nodes = smoothing(Average::Subpixel, Coupling::Nodes);
    let (n32, n64) = (oblique(nodes, 32), oblique(nodes, 64));
    assert!(n32 / n64 > 2.0 && n32 / n64 < 3.2, "{n32} {n64}");
}

#[test]
fn a_uniform_anisotropic_medium_carries_its_plane_waves_to_second_order() {
    // E in the plane, k along (1, 1): ω² = k² t·ε⁻¹t, t ⊥ k, with (ε⁻¹)_xy at the nodes
    let eps = Permittivity::principal([2.0, 6.0, 4.0], rotation([0.5, 0.0, 0.0])).unwrap();
    let m = inverse(&eps.matrix());
    let q = 0.5 * (m[0][0] - 2.0 * m[0][1] + m[1][1]);
    let exact = 2f64.sqrt() * q.sqrt();
    let structure = Structure::new(eps);
    let lattice = Lattice {
        structure: &structure,
        periods: [1, 1, 0],
        sources: &[],
        wave: Some([1.0, 1.0]),
        carrier: exact,
        bandwidth: 0.3,
        time: 30.0,
    };
    let error = |n| (lattice.frequency(Smoothing::default(), n, exact) - exact) / exact;
    let (coarse, fine) = (error(16), error(32));
    assert!(
        (coarse / fine - 4.0).abs() < 0.2 && fine.abs() < 3e-3,
        "{coarse} {fine}"
    );
}

#[test]
fn the_leapfrogs_energy_is_conserved_with_the_off_diagonal_entries_at_the_nodes() {
    let drift = tensor_energy_drift(Coupling::Nodes);
    assert!(drift < 1e-12, "{drift}");
    // at the points, ε̃⁻¹ isn't symmetric and nothing is conserved
    let drift = tensor_energy_drift(Coupling::Points);
    assert!(drift > 1e-6, "{drift}");
}

#[test]
fn a_tensor_medium_takes_no_conductivity_and_holds_conductors_at_zero() {
    let g = grid([6, 6, 6], 0.05);
    let ball = Body::ellipsoid([0.0; 3], [0.08; 3], rotation([0.0; 3])).unwrap();
    let structure =
        Structure::new(Permittivity::isotropic(1.0).unwrap()).with(ball, tilted_crystal());
    let mut s = Simulation::smoothed(
        g,
        &structure,
        Smoothing::default(),
        Boundaries::walls(),
        0.9,
    )
    .unwrap();
    assert!(s.anisotropic.is_some());
    s.conductor(Axis::X, (3, 3, 3));
    s.add_source(Source {
        field: Field::E,
        component: Axis::Y,
        at: (2, 3, 3),
        waveform: Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.5).unwrap(),
    })
    .unwrap();
    s.run(50);
    assert_eq!(s.e(Axis::X)[g.index(Axis::X, (3, 3, 3))], 0.0);
    assert!(s.e(Axis::X).iter().any(|v| *v != 0.0));
    assert!(s.clone().with_conductivity(|_, _, _| 1.0).is_err());
    // nor dispersive media, nor a Bloch phase
    let drude = super::super::Dispersive {
        eps_inf: 1.0,
        poles: vec![super::super::Pole::Drude {
            plasma: Frequency::natural(0.5).unwrap(),
            damping: 0.1,
        }],
    };
    let unstarted = Simulation::smoothed(
        g,
        &structure,
        Smoothing::default(),
        Boundaries::walls(),
        0.9,
    )
    .unwrap();
    assert!(unstarted.with_medium(&drude, |_, _, _| true).is_err());
    let bloch = Boundaries {
        x: crate::fdfd::Edges::Bloch { k: 1.0 },
        ..Boundaries::walls()
    };
    assert!(Simulation::smoothed(g, &structure, Smoothing::default(), bloch, 0.9).is_err());
    // an isotropic medium across grid-aligned interfaces has no off-diagonal entries
    let slab = Body::half_space([0.0, 0.0, 0.013], [0.0, 0.0, 1.0]).unwrap();
    let structure = Structure::new(Permittivity::isotropic(1.0).unwrap())
        .with(slab, Permittivity::isotropic(4.0).unwrap());
    let s = Simulation::smoothed(
        g,
        &structure,
        Smoothing::default(),
        Boundaries::walls(),
        0.9,
    )
    .unwrap();
    assert!(s.anisotropic.is_none());
    assert!(
        Simulation::smoothed(
            g,
            &structure,
            Smoothing {
                diameter: 0.0,
                ..Smoothing::default()
            },
            Boundaries::walls(),
            0.9
        )
        .is_err()
    );
}

#[test]
fn a_tensor_medium_is_the_same_bits_on_any_number_of_threads() {
    let g = grid([12, 10, 9], 0.05);
    let ellipsoid = Body::ellipsoid(
        [0.02, 0.0, -0.01],
        [0.2, 0.13, 0.11],
        rotation([0.3, 0.7, 0.2]),
    )
    .unwrap();
    let structure =
        Structure::new(Permittivity::isotropic(2.0).unwrap()).with(ellipsoid, tilted_crystal());
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let mut s = Simulation::smoothed(
                    g,
                    &structure,
                    Smoothing::default(),
                    Boundaries::cpml(3),
                    0.9,
                )
                .unwrap();
                s.add_source(Source {
                    field: Field::E,
                    component: Axis::Y,
                    at: (5, 4, 4),
                    waveform: Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.5).unwrap(),
                })
                .unwrap();
                s.run(80);
                Axis::ALL.map(|c| (s.e(c).to_vec(), s.h(c).to_vec()))
            })
    };
    let one = run(1);
    for threads in [2, 4, 5, 20] {
        assert!(run(threads) == one, "{threads} threads");
    }
}

#[test]
fn harmonic_inversion_finds_a_sums_frequencies_and_decays() {
    let dt = 0.1;
    let signal: Vec<f64> = (0..300)
        .map(|n| {
            let t = n as f64 * dt;
            2.0 * (1.3 * t + 0.4).cos() * (-0.01 * t).exp() + 0.5 * (2.9 * t).sin()
        })
        .collect();
    let mut found = harmonics(&[signal], dt);
    found.sort_by(|a, b| a.omega.total_cmp(&b.omega));
    assert_eq!(found.len(), 2, "{found:?}");
    assert!((found[0].omega - 1.3).abs() < 1e-10 && (found[0].decay - 0.01).abs() < 1e-10);
    assert!((found[0].amplitude - 2.0).abs() < 1e-8 && (found[1].amplitude - 0.5).abs() < 1e-8);
    assert!((found[1].omega - 2.9).abs() < 1e-10 && found[1].decay.abs() < 1e-10);
}
