//! Tests of bodies in space: distances and normals against closed forms, volumes, and fills of
//! cells summed against the volumes.

use std::f64::consts::PI;

use super::body_checks::{grid_sum, lens, rotation};
use super::*;
use crate::units::Length;

fn um(v: f64) -> Length {
    Length::um(v)
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1.0)
}

/// Points scattered through a box, the same on every run.
fn points(lo: [f64; 3], hi: [f64; 3], n: usize) -> Vec<[f64; 3]> {
    let mut s: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    (0..n)
        .map(|_| std::array::from_fn(|k| lo[k] + (hi[k] - lo[k]) * next()))
        .collect()
}

#[test]
fn a_spheres_distance_and_normal_are_exact() {
    let c = [0.3, -0.2, 0.1];
    let s = Body::sphere(c, 0.7).unwrap();
    for p in points([-1.0; 3], [1.0; 3], 500) {
        let q = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
        let r = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt();
        let d = s.distance(p);
        assert!((d.signed.to_um() - (r - 0.7)).abs() < 1e-14, "{p:?}");
        for (n, qk) in d.normal.iter().zip(q) {
            assert!((n - qk / r).abs() < 1e-13);
        }
        assert_eq!(s.contains(p), r <= 0.7);
    }
    assert!(close(s.volume(), 4.0 / 3.0 * PI * 0.343, 1e-15));
}

#[test]
fn an_ellipsoids_nearest_point_is_on_it_and_along_its_normal() {
    let axes = rotation(0.3, 1.2, -0.7);
    let c = [0.1, 0.2, -0.3];
    let e = [0.9, 0.5, 0.3];
    let body = Body::ellipsoid(c, e, axes).unwrap();
    // the surface sampled densely: no point of it is nearer than the distance found
    let sample: Vec<[f64; 3]> = (0..200)
        .flat_map(|i| {
            (0..400).map(move |j| {
                let th = PI * (i as f64 + 0.5) / 200.0;
                let ph = 2.0 * PI * j as f64 / 400.0;
                let u = [
                    e[0] * th.sin() * ph.cos(),
                    e[1] * th.sin() * ph.sin(),
                    e[2] * th.cos(),
                ];
                std::array::from_fn(|k| c[k] + (0..3).map(|i| u[i] * axes[i][k]).sum::<f64>())
            })
        })
        .collect();
    for p in points([-1.2; 3], [1.2; 3], 60) {
        let d = body.distance(p);
        let ds = d.signed.to_um();
        let foot: [f64; 3] = std::array::from_fn(|k| p[k] - ds * d.normal[k]);
        // the foot is on the surface
        let u: [f64; 3] = std::array::from_fn(|i| {
            (0..3).map(|k| (foot[k] - c[k]) * axes[i][k]).sum::<f64>() / e[i]
        });
        let level = u[0] * u[0] + u[1] * u[1] + u[2] * u[2];
        assert!((level - 1.0).abs() < 1e-12, "{p:?}: {level}");
        // and the normal is the gradient's direction there
        let g: [f64; 3] =
            std::array::from_fn(|k| (0..3).map(|i| u[i] / e[i] * axes[i][k]).sum::<f64>());
        let gl = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt();
        for (gk, n) in g.iter().zip(d.normal) {
            assert!((gk / gl - n).abs() < 1e-9, "{p:?}");
        }
        let nearest = sample
            .iter()
            .map(|s| ((s[0] - p[0]).powi(2) + (s[1] - p[1]).powi(2) + (s[2] - p[2]).powi(2)).sqrt())
            .fold(f64::INFINITY, f64::min);
        assert!(ds.abs() <= nearest + 1e-12, "{p:?}: {ds} against {nearest}");
        assert!(nearest - ds.abs() < 0.02, "{p:?}: {ds} against {nearest}");
        assert_eq!(body.contains(p), ds <= 0.0);
    }
}

#[test]
fn a_boxs_distance_is_exact_and_its_fills_are_its_overlaps() {
    let b = Body::cuboid([0.0, -0.2, 0.1], [0.5, 0.3, 0.4]).unwrap();
    assert!((b.distance([0.25, 0.05, 0.25]).signed.to_um() + 0.15).abs() < 1e-15);
    let d = b.distance([0.8, 0.7, 0.1]);
    assert!((d.signed.to_um() - 0.3f64.hypot(0.4)).abs() < 1e-15);
    assert!((d.normal[0] - 0.6).abs() < 1e-15 && (d.normal[1] - 0.8).abs() < 1e-15);
    // a 7 × 6 × 5 grid of cells cutting it every which way: each cell's fill its overlap
    let h = [0.11, 0.13, 0.09];
    for i in 0..7 {
        for j in 0..6 {
            for k in 0..5 {
                let lo = [
                    -0.05 + i as f64 * h[0],
                    -0.31 + j as f64 * h[1],
                    0.03 + k as f64 * h[2],
                ];
                let cell = Bounds3d::new(lo, [lo[0] + h[0], lo[1] + h[1], lo[2] + h[2]]);
                let want = (0..3)
                    .map(|a| {
                        let [l0, l1] = [[0.0, 0.5], [-0.2, 0.3], [0.1, 0.4]][a];
                        (cell.max[a].min(l1) - cell.min[a].max(l0)).max(0.0)
                    })
                    .product::<f64>();
                assert_eq!(b.fill(cell), want);
            }
        }
    }
}

#[test]
fn cylinders_and_cones_have_exact_distances() {
    let c = Body::cylinder([0.0, 0.0, -0.5], [0.0, 0.0, 0.5], 0.3).unwrap();
    let d = c.distance([0.5, 0.0, 0.1]);
    assert!((d.signed.to_um() - 0.2).abs() < 1e-15 && (d.normal[0] - 1.0).abs() < 1e-15);
    let d = c.distance([0.0, 0.1, 0.9]);
    assert!((d.signed.to_um() - 0.4).abs() < 1e-15 && (d.normal[2] - 1.0).abs() < 1e-15);
    let d = c.distance([0.6, 0.0, 0.9]);
    assert!((d.signed.to_um() - 0.5).abs() < 1e-15);
    assert!((c.distance([0.0, 0.0, 0.0]).signed.to_um() + 0.3).abs() < 1e-15);
    assert!(close(c.volume(), PI * 0.09, 1e-15));
    // a cone, apex up: on its side the distance falls along the side's normal
    let k = Body::cone([0.0; 3], [0.0, 0.0, 1.0], [0.5, 0.0]).unwrap();
    let side = [0.25, 0.0, 0.5];
    let n = [1.0 / 1.25f64.sqrt(), 0.0, 0.5 / 1.25f64.sqrt()];
    let p = [side[0] + 0.1 * n[0], 0.0, side[2] + 0.1 * n[2]];
    let d = k.distance(p);
    assert!((d.signed.to_um() - 0.1).abs() < 1e-14);
    assert!((d.normal[0] - n[0]).abs() < 1e-14 && (d.normal[2] - n[2]).abs() < 1e-14);
    assert!(k.contains([0.0, 0.0, 0.99]) && !k.contains([0.1, 0.0, 0.9]));
    assert!(close(k.volume(), PI * 0.25 / 3.0, 1e-15));
}

#[test]
fn the_primitives_fills_sum_to_their_volumes() {
    let axes = rotation(0.4, -0.9, 1.3);
    let cases: Vec<(Body, f64)> = vec![
        (
            Body::sphere([0.013, -0.021, 0.034], 0.37).unwrap(),
            4.0 / 3.0 * PI * 0.37f64.powi(3),
        ),
        (
            Body::ellipsoid([0.02, 0.01, -0.03], [0.41, 0.27, 0.19], axes).unwrap(),
            4.0 / 3.0 * PI * 0.41 * 0.27 * 0.19,
        ),
        (
            Body::cylinder([0.01, -0.02, -0.33], [0.01, -0.02, 0.29], 0.21).unwrap(),
            PI * 0.21 * 0.21 * 0.62,
        ),
        (
            Body::cone([-0.31, 0.02, 0.01], [0.33, 0.02, 0.01], [0.27, 0.09]).unwrap(),
            PI * 0.64 * (0.27 * 0.27 + 0.27 * 0.09 + 0.09 * 0.09) / 3.0,
        ),
    ];
    for (body, volume) in cases {
        assert!(close(body.volume(), volume, 1e-14));
        let sum = grid_sum(&body, [-0.5; 3], [0.5; 3], 9);
        assert!(
            (sum - volume).abs() < 1e-12 * volume,
            "{sum} against {volume}"
        );
    }
}

#[test]
fn oblique_cylinders_and_cones_fill_to_their_tolerance() {
    let a: [f64; 3] = [0.3, -0.2, 0.25];
    let b: [f64; 3] = [-0.25, 0.3, -0.2];
    let cyl = Body::cylinder(a, b, 0.17).unwrap();
    let l = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
    let want = PI * 0.17 * 0.17 * l;
    let sum = grid_sum(&cyl, [-0.5; 3], [0.5; 3], 7);
    assert!((sum - want).abs() < 1e-7 * want, "{sum} against {want}");
    let cone = Body::cone(a, b, [0.2, 0.05]).unwrap();
    let want = PI * l * (0.04 + 0.01 + 0.0025) / 3.0;
    let sum = grid_sum(&cone, [-0.5; 3], [0.5; 3], 7);
    assert!((sum - want).abs() < 1e-7 * want, "{sum} against {want}");
}

#[test]
fn an_extrusion_with_vertical_walls_is_its_area_times_its_height() {
    let poly = Polygon::new(vec![
        Point::um(-0.3, -0.2),
        Point::um(0.35, -0.25),
        Point::um(0.2, 0.3),
        Point::um(-0.1, 0.15),
        Point::um(-0.25, 0.28),
    ])
    .unwrap();
    let area = poly.area();
    let prism = Body::prism(Shape::Polygon(poly.clone()), [-0.11, 0.23]).unwrap();
    assert!(close(prism.volume(), area * 0.34, 1e-15));
    let sum = grid_sum(&prism, [-0.5; 3], [0.5; 3], 8);
    assert!((sum - area * 0.34).abs() < 1e-14, "{sum}");
    // each cell: the polygon's exact area in its footprint times the heights shared
    let cell = Bounds3d::new([-0.1, -0.1, 0.2], [0.1, 0.05, 0.3]);
    let plan = Shape::Polygon(poly).fill(cell.plan());
    assert_eq!(prism.fill(cell), plan * (0.23 - 0.2));
}

#[test]
fn a_sloped_extrusion_is_its_frustum() {
    let (h, run) = (0.3, 1.0 / 70f64.to_radians().tan());
    // a disk: a cone's frustum, exactly
    let r = 0.2;
    let disk = Body::etched(
        Shape::circle(Point::um(0.01, -0.02), um(r)).unwrap(),
        [-0.1, -0.1 + h],
        70f64.to_radians(),
    )
    .unwrap();
    let r0 = r + h * run;
    let want = PI * h * (r0 * r0 + r0 * r + r * r) / 3.0;
    assert!(close(disk.volume(), want, 1e-14));
    let sum = grid_sum(&disk, [-0.5; 3], [0.5; 3], 9);
    assert!((sum - want).abs() < 1e-12 * want, "{sum} against {want}");
    // a rectangle: its top face grown by (top − z) cot θ with round corners at each height,
    // A h + P run h²/2 + π run² h³/3 (Steiner's formula, integrated)
    let (w, d) = (0.4, 0.25);
    let rect = Body::etched(
        Shape::rect(Point::um(0.0, 0.0), um(w), um(d)).unwrap(),
        [-0.1, -0.1 + h],
        70f64.to_radians(),
    )
    .unwrap();
    let want = w * d * h + 2.0 * (w + d) * run * h * h / 2.0 + PI * run * run * h.powi(3) / 3.0;
    let sum = grid_sum(&rect, [-0.5; 3], [0.5; 3], 6);
    assert!((sum - want).abs() < 1e-6 * want, "{sum} against {want}");
}

#[test]
fn booleans_of_two_spheres_follow_inclusion_and_exclusion() {
    let (ra, rb, d) = (0.3, 0.22, 0.31);
    let a = Body::sphere([-0.1, 0.02, 0.01], ra).unwrap();
    let b = Body::sphere([-0.1 + d, 0.02, 0.01], rb).unwrap();
    let (va, vb) = (4.0 / 3.0 * PI * ra.powi(3), 4.0 / 3.0 * PI * rb.powi(3));
    let both = lens(ra, rb, d);
    for (op, want) in [
        (Operation::Union, va + vb - both),
        (Operation::Intersection, both),
        (Operation::Difference, va - both),
        (Operation::Xor, va + vb - 2.0 * both),
    ] {
        let body = Body::boolean(op, a.clone(), b.clone());
        let sum = grid_sum(&body, [-0.5; 3], [0.5; 3], 6);
        assert!(
            (sum - want).abs() < 1e-6 * want,
            "{op:?}: {sum} against {want}"
        );
        // inside tests are the operands' combined
        for p in points([-0.5; 3], [0.5; 3], 200) {
            let (x, y) = (a.contains(p), b.contains(p));
            let want = match op {
                Operation::Union => x || y,
                Operation::Intersection => x && y,
                Operation::Difference => x && !y,
                Operation::Xor => x != y,
            };
            assert_eq!(body.contains(p), want);
        }
    }
}

#[test]
fn a_half_space_and_a_slab_fill_by_their_planes() {
    let n: [f64; 3] = [0.3, -0.5, 0.8];
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    let h = Body::half_space([0.1, 0.0, 0.0], n).unwrap();
    let cell = Bounds3d::new([0.0, -0.1, -0.05], [0.2, 0.1, 0.07]);
    let c = cell.centre();
    let d = (n[0] * (c[0] - 0.1) + n[1] * c[1] + n[2] * c[2]) / l;
    let want = plane_share(n.map(|v| v / l), d, cell.size()) * cell.volume();
    assert_eq!(h.fill(cell), want);
    // a slab: two semi-axes infinite
    let axes = rotation(0.2, 0.5, 0.9);
    let slab = Body::ellipsoid([0.0; 3], [f64::INFINITY, 0.13, f64::INFINITY], axes).unwrap();
    let sum = grid_sum(&slab, [-0.5; 3], [0.5; 3], 8);
    // its volume in the box: the box's slices between the two planes, by fine sampling
    let m = 160;
    let mut inside = 0usize;
    for i in 0..m {
        for j in 0..m {
            for k in 0..m {
                let p = [i, j, k].map(|v| -0.5 + (v as f64 + 0.5) / m as f64);
                inside += usize::from(slab.contains(p));
            }
        }
    }
    let sampled = inside as f64 / (m * m * m) as f64;
    assert!((sum - sampled).abs() < 2e-3, "{sum} against {sampled}");
}

#[test]
fn bounds_hold_their_bodies() {
    let axes = rotation(0.7, 0.1, -0.4);
    let bodies = [
        Body::ellipsoid([0.1, 0.2, 0.3], [0.5, 0.2, 0.1], axes).unwrap(),
        Body::cone([0.0; 3], [0.3, -0.2, 0.4], [0.2, 0.05]).unwrap(),
        Body::etched(
            Shape::circle(Point::um(0.0, 0.0), um(0.2)).unwrap(),
            [0.0, 0.2],
            1.2,
        )
        .unwrap(),
    ];
    for body in bodies {
        let b = body.bounds();
        for p in points([-1.0; 3], [1.0; 3], 4000) {
            if body.contains(p) {
                assert!(b.contains(p), "{p:?} outside {b:?}");
            }
        }
    }
}

#[test]
fn the_tangent_planes_fill_converges_at_second_order() {
    // what FDTD's smoothing takes: each cell's share of the plane through its centre's nearest
    // point of the surface, summed over a grid, against the sphere's volume
    let r = 0.37;
    let sphere = Body::sphere([0.013, -0.021, 0.034], r).unwrap();
    let want = 4.0 / 3.0 * PI * r.powi(3);
    let error = |n: usize| {
        let h = 1.0 / n as f64;
        let mut sum = 0.0;
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    let c = [i, j, k].map(|v| -0.5 + (v as f64 + 0.5) * h);
                    let (d, nrm) = sphere.surface(c);
                    sum += plane_share(nrm, d, [h; 3]) * h * h * h;
                }
            }
        }
        (sum - want).abs()
    };
    let (e1, e2) = (error(16), error(32));
    let order = (e1 / e2).log2();
    assert!((order - 2.0).abs() < 0.3, "{e1} {e2}: {order}");
}

#[test]
fn sections_are_the_planar_shapes_bodies_fill() {
    let window = Bounds::new(Point::um(-2.0, -2.0), Point::um(2.0, 2.0));
    let area = |b: &Body, z: f64| {
        b.section(z, window)
            .unwrap()
            .map_or(0.0, |s| Region::area(&s))
    };
    // a sphere's disk, an ellipsoid's ellipse, a cone's disk, a box's rectangle
    let sphere = Body::sphere([0.1, 0.2, 0.3], 0.5).unwrap();
    assert!((area(&sphere, 0.6) - PI * (0.25 - 0.09)).abs() < 1e-12);
    assert_eq!(area(&sphere, 0.9), 0.0);
    let axes = rotation(0.3, 0.8, -0.4);
    let e = [0.6, 0.4, 0.3];
    let ellipsoid = Body::ellipsoid([0.0; 3], e, axes).unwrap();
    // the section through the centre: pi abc over its half-width along z, sqrt(sum (e_i n_i)^2)
    let along_z = (0..3)
        .map(|i| (e[i] * axes[i][2]).powi(2))
        .sum::<f64>()
        .sqrt();
    let v = 4.0 / 3.0 * PI * e[0] * e[1] * e[2];
    assert!((area(&ellipsoid, 0.0) - 0.75 * v / along_z).abs() < 1e-12);
    let cone = Body::cone([0.0, 0.0, -1.0], [0.0, 0.0, 1.0], [0.6, 0.2]).unwrap();
    assert!((area(&cone, 0.0) - PI * 0.16).abs() < 1e-12);
    let cube = Body::cuboid([-0.5, -0.25, 0.0], [0.5, 0.25, 1.0]).unwrap();
    assert!((area(&cube, 0.5) - 0.5).abs() < 1e-15);
    // a difference, and a cylinder lying along x as a mask of the window
    let hole = Body::cylinder([0.0, 0.0, -2.0], [0.0, 0.0, 2.0], 0.1).unwrap();
    let holed = Body::difference(cube.clone(), hole);
    assert!((area(&holed, 0.5) - (0.5 - PI * 0.01)).abs() < 1e-6);
    let lying = Body::cylinder([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.3).unwrap();
    let strip = 2.0 * 2.0 * (0.09f64 - 0.04).sqrt();
    assert!((area(&lying, 0.2) - strip).abs() < 0.03 * strip);
}

#[test]
fn fdfd_and_the_rasters_sample_a_structures_bodies() {
    use crate::averaging::Averaging;
    use crate::fdfd::{Boundaries3d, Grid3d, Solver3d};
    use crate::stack::{LayerStack, Medium, Structure};
    use crate::units::Wavelength;
    let mut s = Structure::new(LayerStack::soi_220());
    s.place(
        Body::sphere([0.0, 0.0, 2.5], 0.2).unwrap(),
        Medium::Material(crate::material::silicon()),
    );
    let grid = Grid3d {
        nx: 6,
        ny: 6,
        nz: 6,
        dx: 0.1,
        dy: 0.1,
        dz: 0.1,
        x0: -0.3,
        y0: -0.3,
        z0: 2.2,
    };
    let lam = Wavelength::um(1.55).unwrap();
    // the fill's slices are the stack's: refused; samples read the body
    let fill = Solver3d::from_structure(grid, lam, &s, Boundaries3d::pml(0), Averaging::Mean);
    assert!(fill.is_err());
    let sampled = Solver3d::from_structure(grid, lam, &s, Boundaries3d::pml(0), Averaging::Samples);
    assert!(sampled.is_ok());
    let um = Length::um;
    let side = |averaging| {
        s.side_view_averaged(
            um(0.0),
            (um(-0.3), um(0.3)),
            (um(2.3), um(2.7)),
            um(0.1),
            lam,
            averaging,
        )
    };
    // the sphere's silicon, above the stack's top at 2.22 µm, in the samples' picture
    let r = side(Averaging::Samples).unwrap();
    assert!(r.range().1 > 11.0, "{:?}", r.range());
    assert!(side(Averaging::Mean).is_err());
}

#[test]
fn a_graded_body_in_fdtds_smoothing_is_checked() {
    use crate::fdfd::Grid3d;
    use crate::fdtd::{Boundaries, Permittivity, Simulation, Smoothing, Structure};
    use crate::material::Graded;
    let grid = Grid3d {
        nx: 8,
        ny: 8,
        nz: 8,
        dx: 0.1,
        dy: 0.1,
        dz: 0.1,
        x0: -0.4,
        y0: -0.4,
        z0: -0.4,
    };
    let structure = |index: &str| {
        Structure::new(Permittivity::isotropic(1.0).unwrap()).with_graded(
            Body::sphere([0.0; 3], 0.3).unwrap(),
            Graded::parse_index(index).unwrap(),
        )
    };
    let ok = Simulation::smoothed(
        grid,
        &structure("1.5 + 0.2*x/um"),
        Smoothing::default(),
        Boundaries::cpml(0),
        0.9,
    );
    assert!(ok.is_ok());
    // the medium at a point: the body's graded value inside, the background outside
    let s = structure("1.5 + 0.2*x/um");
    let inside = s.at([0.1, 0.0, 0.0]).matrix();
    assert!((inside[0][0] - 1.52f64.powi(2)).abs() < 1e-12);
    assert_eq!(s.at([0.35, 0.0, 0.0]).matrix()[0][0], 1.0);
    // an index whose square falls to zero inside the body is refused
    let bad = Simulation::smoothed(
        grid,
        &structure("sqrt(max(0, 1 - 20*(x/um)))"),
        Smoothing::default(),
        Boundaries::cpml(0),
        0.9,
    );
    let Err(e) = bad else {
        panic!("an index that falls to zero is refused")
    };
    let e = e.to_string();
    assert!(e.contains("positive"), "{e}");
}

#[test]
#[ignore = "times fills; cargo test --release --lib fill_timing -- --ignored --nocapture"]
fn fill_timing() {
    let sphere = Body::sphere([0.013, -0.021, 0.034], 0.37).unwrap();
    for n in [4usize, 16, 64, 256] {
        let h = 1.0 / n as f64;
        let start = std::time::Instant::now();
        let mut count = 0usize;
        let mut sum = 0.0;
        for i in 0..n {
            for j in 0..n {
                let lo = [-0.5 + i as f64 * h, -0.5 + j as f64 * h, 0.0];
                let cell = Bounds3d::new(lo, [lo[0] + h, lo[1] + h, h]);
                // only the cells the surface crosses
                if sphere.distance_um(cell.centre()).0.abs() < 0.9 * h {
                    sum += sphere.fill(cell);
                    count += 1;
                }
            }
        }
        let t = start.elapsed().as_secs_f64();
        println!(
            "n {n}: {count} cells crossed, {:.2} us each ({sum})",
            1e6 * t / count.max(1) as f64
        );
    }
}

#[test]
#[ignore = "times the slow fills; cargo test --release --lib slow_fill_timing -- --ignored --nocapture"]
fn slow_fill_timing() {
    use super::body_checks::*;
    for (name, f) in [
        ("sloped", sloped_error as fn() -> f64),
        ("boolean", boolean_error),
        ("oblique", oblique_error),
        ("volumes", volume_error),
        ("extrusions", extrusion_error),
        ("akahane", akahane_body_error),
    ] {
        let start = std::time::Instant::now();
        let e = f();
        println!("{name}: {e:.3e} in {:.2} s", start.elapsed().as_secs_f64());
    }
}
