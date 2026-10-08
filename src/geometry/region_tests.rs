//! The geometry kernel's checks: every primitive's area, perimeter, distance and normal against
//! closed forms, polygons within their tolerance and their areas converging, transforms keeping
//! areas and distances, fills summing to the area, and the index answering as a search of every
//! shape does, on any number of threads.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use super::checks::{Numbers, shapes, um};
use super::primitives::gamma;
use super::*;
use crate::units::Length;

#[test]
fn the_primitives_areas_and_perimeters_are_their_closed_forms() {
    let (a, b) = (1.2, 0.5);
    // the ellipse: πab, and its perimeter by the AGM against quadrature of its speed
    let e = Shape::ellipse(Point::um(0.0, 0.0), [um(a), um(b)], 0.4).unwrap();
    assert!((e.area() - PI * a * b).abs() < 1e-14);
    let p = Region::perimeter(&e).unwrap().to_um();
    let quad = boundary::integrate(|t| (a * t.sin()).hypot(b * t.cos()), 0.0, TAU);
    assert!((p - quad).abs() < 1e-12, "{p} {quad}");
    // a circle's: 2πr
    let circle = Shape::ellipse(Point::um(1.0, 0.0), [um(0.7), um(0.7)], 1.0).unwrap();
    let p = Region::perimeter(&circle).unwrap().to_um();
    assert!((p - TAU * 0.7).abs() < 1e-14, "{p}");
    // Γ and the superellipse: n = 2 is the ellipse, n = 1 the rhombus
    assert!((gamma(0.5) - PI.sqrt()).abs() < 1e-14);
    assert!((gamma(5.0) - 24.0).abs() < 1e-12);
    assert!((gamma(1.25) - 0.906_402_477_055_477).abs() < 1e-14);
    let s2 = Shape::superellipse(Point::um(0.0, 0.0), [um(a), um(b)], 2.0, 0.0).unwrap();
    assert!((s2.area() - PI * a * b).abs() < 1e-13);
    let p2 = Region::perimeter(&s2).unwrap().to_um();
    assert!((p2 - quad).abs() < 1e-11, "{p2} {quad}");
    let s1 = Shape::superellipse(Point::um(0.0, 0.0), [um(a), um(b)], 1.0, 0.3).unwrap();
    assert!((s1.area() - 2.0 * a * b).abs() < 1e-14);
    let p1 = Region::perimeter(&s1).unwrap().to_um();
    assert!((p1 - 4.0 * a.hypot(b)).abs() < 1e-12, "{p1}");
    // the sector, the rounded rectangle and the regular polygon: their formulas against
    // Green's theorem on their boundaries, and their perimeters against the boundaries' lengths
    for (name, shape) in shapes() {
        let loops = match &shape {
            Shape::Sector(s) => s.loops(),
            Shape::RoundedRect(r) => r.loops(),
            Shape::Ellipse(e) => e.loops(),
            Shape::RegularPolygon(r) => r.polygon().loops(),
            Shape::Superellipse(_) => continue,
            other => other.loops(),
        };
        let green = boundary::area_of(&loops);
        assert!(
            (green - shape.area()).abs() < 1e-13 * shape.area().max(1.0),
            "{name}: {green} {}",
            shape.area()
        );
        let length = boundary::length(&loops);
        let p = Region::perimeter(&shape).unwrap().to_um();
        assert!((length - p).abs() < 1e-12, "{name}: {length} {p}");
    }
}

#[test]
fn distances_and_normals_are_the_closed_forms() {
    // a circle: |p − c| − r along (p − c)
    let c = Shape::circle(Point::um(1.0, 2.0), um(0.5)).unwrap();
    let d = c.distance(Point::um(1.0 + 0.3, 2.0 + 0.4));
    assert!((d.signed.to_um() - 0.0).abs() < 1e-15);
    let d = c.distance(Point::um(1.0 + 0.6, 2.0 + 0.8));
    assert!((d.signed.to_um() - 0.5).abs() < 1e-15);
    assert!((d.normal[0] - 0.6).abs() < 1e-15 && (d.normal[1] - 0.8).abs() < 1e-15);
    let d = c.distance(Point::um(1.0, 2.1));
    assert!((d.signed.to_um() + 0.4).abs() < 1e-15 && d.normal[1] > 0.999);
    // a ring: the inner edge's normal points into the hole
    let r = Shape::ring(Point::um(0.0, 0.0), um(1.0), um(0.4)).unwrap();
    let d = r.distance(Point::um(0.85, 0.0));
    assert!((d.signed.to_um() + 0.05).abs() < 1e-15 && (d.normal[0] + 1.0).abs() < 1e-15);
    let d = r.distance(Point::um(0.0, 0.1));
    assert!((d.signed.to_um() - 0.7).abs() < 1e-15 && (d.normal[1] + 1.0).abs() < 1e-15);
    // a rectangle's corner: the distance to it, along the diagonal
    let b = Shape::rect(Point::um(0.0, 0.0), um(2.0), um(1.0)).unwrap();
    let d = b.distance(Point::um(1.3, 0.9));
    assert!((d.signed.to_um() - 0.5).abs() < 1e-15, "{d:?}");
    assert!((d.normal[0] - 0.6).abs() < 1e-15 && (d.normal[1] - 0.8).abs() < 1e-15);
    // a rounded corner: the distance to the corner's circle
    let rr = Shape::rounded_rect(Point::um(0.0, 0.0), um(2.0), um(1.0), um(0.3), 0.0).unwrap();
    let corner = [0.7, 0.2];
    let p = Point::um(corner[0] + 0.3, corner[1] + 0.4);
    let d = rr.distance(p);
    assert!((d.signed.to_um() - 0.2).abs() < 1e-15, "{d:?}");
    // a pie slice's apex, from beyond it
    let pie = Shape::sector(Point::um(0.0, 0.0), Length::ZERO, um(1.0), 0.0, FRAC_PI_2).unwrap();
    let d = pie.distance(Point::um(-0.3, -0.4));
    assert!((d.signed.to_um() - 0.5).abs() < 1e-15, "{d:?}");
    // a superellipse of exponent 2 is the ellipse
    let e = Shape::ellipse(Point::um(0.1, 0.2), [um(1.0), um(0.4)], 0.5).unwrap();
    let s = Shape::superellipse(Point::um(0.1, 0.2), [um(1.0), um(0.4)], 2.0, 0.5).unwrap();
    let mut n = Numbers(7);
    for _ in 0..200 {
        let p = n.point(&e.bounds(), 0.5);
        let (de, ds) = (e.distance(p), s.distance(p));
        assert!(
            (de.signed - ds.signed).to_um().abs() < 1e-10,
            "{p:?}: {de:?} {ds:?}"
        );
    }
}

/// For every shape, at points off the boundary and its medial axis: the distance's gradient
/// (by central differences) is the normal, and stepping back along the normal by the distance
/// lands on the boundary.
#[test]
fn the_normal_is_the_distances_gradient_and_points_at_the_boundary() {
    let h = 1e-6;
    for (name, shape) in shapes() {
        let mut n = Numbers(11);
        let mut checked = 0;
        for _ in 0..400 {
            let p = n.point(&shape.bounds(), 0.4);
            let d = shape.distance(p);
            let at = |dx: f64, dy: f64| {
                shape
                    .distance(Point {
                        x: p.x + um(dx),
                        y: p.y + um(dy),
                    })
                    .signed
                    .to_um()
            };
            let g = [
                (at(h, 0.0) - at(-h, 0.0)) / (2.0 * h),
                (at(0.0, h) - at(0.0, -h)) / (2.0 * h),
            ];
            // off the medial axis, where the distance is smooth and its gradient a unit vector
            if (g[0].hypot(g[1]) - 1.0).abs() > 1e-6 {
                continue;
            }
            let tol = if name == "superellipse" { 1e-5 } else { 1e-7 };
            assert!(
                (g[0] - d.normal[0]).abs() < tol && (g[1] - d.normal[1]).abs() < tol,
                "{name} at {p:?}: {g:?} against {:?}",
                d.normal
            );
            let back = Point {
                x: p.x - d.signed * d.normal[0],
                y: p.y - d.signed * d.normal[1],
            };
            let on = shape.distance(back).signed.to_um().abs();
            assert!(on < 1e-9, "{name}: {on}");
            checked += 1;
        }
        assert!(checked > 200, "{name}: {checked}");
    }
}

#[test]
fn points_on_the_boundary_are_found_there() {
    let r = Shape::rect(Point::um(0.0, 0.0), um(2.0), um(1.0)).unwrap();
    assert_eq!(r.locate(Point::um(1.0, 0.2)), Location::Boundary);
    assert_eq!(r.locate(Point::um(1.0, 0.5)), Location::Boundary);
    assert_eq!(r.locate(Point::um(0.9, 0.2)), Location::Inside);
    assert_eq!(r.locate(Point::um(1.1, 0.2)), Location::Outside);
    let c = Shape::circle(Point::um(0.0, 0.0), um(1.0)).unwrap();
    assert_eq!(c.locate(Point::um(0.0, -1.0)), Location::Boundary);
    let p = Polygon::new(vec![
        Point::um(0.0, 0.0),
        Point::um(2.0, 0.0),
        Point::um(0.0, 2.0),
    ])
    .unwrap();
    assert_eq!(p.locate(Point::um(1.0, 1.0)), Location::Boundary);
    assert_eq!(p.locate(Point::um(0.5, 0.5)), Location::Inside);
    let s = Shape::sector(Point::um(0.0, 0.0), um(0.5), um(1.0), 0.0, FRAC_PI_2).unwrap();
    assert_eq!(s.locate(Point::um(0.7, 0.0)), Location::Boundary);
    assert_eq!(s.locate(Point::um(0.0, 0.75)), Location::Boundary);
    assert_eq!(s.locate(Point::um(0.6, 0.1)), Location::Inside);
    assert_eq!(s.locate(Point::um(0.6, -0.1)), Location::Outside);
    assert_eq!(s.locate(Point::um(0.3, 0.1)), Location::Outside);
    // a sector of more than half a turn
    let big = Shape::sector(Point::um(0.0, 0.0), Length::ZERO, um(1.0), 0.0, 1.5 * PI).unwrap();
    assert!(big.contains(Point::um(-0.5, 0.1)));
    assert!(big.contains(Point::um(-0.1, -0.5)));
    assert!(!big.contains(Point::um(0.3, -0.3)));
    let e = Shape::ellipse(Point::um(0.0, 0.0), [um(2.0), um(1.0)], 0.0).unwrap();
    assert_eq!(e.locate(Point::um(2.0, 0.0)), Location::Boundary);
    let rr = Shape::rounded_rect(Point::um(0.0, 0.0), um(2.0), um(1.0), um(0.25), 0.0).unwrap();
    assert_eq!(rr.locate(Point::um(0.0, 0.5)), Location::Boundary);
    assert_eq!(rr.locate(Point::um(0.99, 0.49)), Location::Outside);
    assert_eq!(rr.locate(Point::um(0.95, 0.0)), Location::Inside);
}

/// The boundary's points, finely: a polygon within 10⁻⁹ µm of it, unsnapped.
fn fine_boundary(shape: &Shape) -> Vec<Point> {
    let p = shape.polygons(Tolerance::unsnapped(um(1e-9)).unwrap());
    p.contours
        .iter()
        .flat_map(|c| c.outer.iter().chain(c.holes.iter().flatten()).copied())
        .collect()
}

#[test]
fn polygons_are_within_their_tolerance() {
    for (name, shape) in shapes() {
        let fine = fine_boundary(&shape);
        for tol in [1e-2, 1e-3, 1e-4] {
            for grid in [0.0, tol / 4.0] {
                let t = Tolerance::new(um(tol), um(grid)).unwrap();
                let p = shape.polygons(t);
                assert!(
                    p.deviation.to_um() <= tol,
                    "{name}: {} > {tol}",
                    p.deviation
                );
                assert_eq!(p.contours.len(), 1, "{name}");
                let c = &p.contours[0];
                // orientation: the outer counterclockwise, holes clockwise
                let rings: Vec<&Vec<Point>> = std::iter::once(&c.outer).chain(&c.holes).collect();
                // every vertex and every edge's points are near the boundary
                for ring in &rings {
                    let n = ring.len();
                    for k in 0..n {
                        let (a, b) = (ring[k], ring[(k + 1) % n]);
                        for s in [0.0, 0.25, 0.5, 0.75] {
                            let q = Point {
                                x: a.x + (b.x - a.x) * s,
                                y: a.y + (b.y - a.y) * s,
                            };
                            let d = shape.distance(q).signed.to_um().abs();
                            assert!(d <= tol * (1.0 + 1e-9), "{name} {tol}: {d}");
                        }
                    }
                }
                // and every point of the boundary is near the polygons
                let outer = Polygon::new(c.outer.clone()).unwrap();
                let holes: Vec<Polygon> = c
                    .holes
                    .iter()
                    .map(|h| Polygon::new(h.clone()).unwrap())
                    .collect();
                for q in fine.iter().step_by(7) {
                    let d = holes
                        .iter()
                        .map(|h| h.distance(*q).signed.to_um().abs())
                        .fold(outer.distance(*q).signed.to_um().abs(), f64::min);
                    // (the fine polygon's own 1e-9 µm aside)
                    assert!(d <= tol + 2e-9, "{name} {tol}: boundary {d} away");
                }
                // the orientation
                assert!(Polygon::new(c.outer.clone()).is_ok());
                let signed: f64 = {
                    let v = &c.outer;
                    (0..v.len())
                        .map(|i| {
                            let (a, b) = (v[i], v[(i + 1) % v.len()]);
                            a.x.to_um() * b.y.to_um() - b.x.to_um() * a.y.to_um()
                        })
                        .sum()
                };
                assert!(signed > 0.0, "{name}: the outer boundary runs clockwise");
                assert!(c.holes.iter().all(|h| {
                    (0..h.len())
                        .map(|i| {
                            let (a, b) = (h[i], h[(i + 1) % h.len()]);
                            a.x.to_um() * b.y.to_um() - b.x.to_um() * a.y.to_um()
                        })
                        .sum::<f64>()
                        < 0.0
                }));
                // snapped vertices are on the grid
                if grid > 0.0 {
                    for v in &c.outer {
                        let k = v.x.to_um() / grid;
                        assert!((k - k.round()).abs() < 1e-6, "{name}: {v:?} off the grid");
                    }
                }
            }
        }
    }
}

/// The polygons' distance from a curve is second order in their chords, so a hundredth of the
/// tolerance takes ten times the vertices; and they keep the region's area exactly, beyond
/// the second order in the tolerance an inscribed polygon's would converge at.
#[test]
fn polygons_converge_at_second_order_and_keep_the_area() {
    for (name, shape) in shapes() {
        let mut errors = Vec::new();
        let mut counts = Vec::new();
        for tol in [1e-4, 1e-6] {
            let p = shape.polygons(Tolerance::unsnapped(um(tol)).unwrap());
            errors.push(((p.area() - shape.area()) / shape.area()).abs());
            counts.push(
                p.contours
                    .iter()
                    .map(|c| c.outer.len() + c.holes.iter().map(Vec::len).sum::<usize>())
                    .sum::<usize>() as f64,
            );
        }
        assert!(errors.iter().all(|e| *e < 1e-13), "{name}: {errors:?}");
        if !matches!(name, "rect" | "polygon" | "hexagon") {
            let ratio = counts[1] / counts[0];
            assert!((7.0..14.0).contains(&ratio), "{name}: {counts:?}");
        }
    }
}

#[test]
fn polygons_follow_the_default_tolerance_and_its_grid() {
    let t = Tolerance::default();
    assert_eq!(t.geometric(), Length::nm(1.0));
    assert_eq!(t.grid(), Length::nm(1.0));
    let ring = Shape::ring(Point::um(0.0, 0.0), um(5.0), um(0.5)).unwrap();
    let p = ring.polygons(t);
    assert!(p.deviation <= Length::nm(1.0));
    assert_eq!(p.contours[0].holes.len(), 1);
    assert!(Tolerance::new(um(1e-3), um(2e-3)).is_err());
    assert!(Tolerance::unsnapped(Length::ZERO).is_err());
    assert!(Tolerance::unsnapped(um(f64::NAN)).is_err());
}

#[test]
fn transforms_keep_areas_and_distances() {
    let t = Transform::rotate(0.7, Point::um(0.5, -0.2))
        .then(&Transform::scale(1.7, Point::um(-1.0, 0.3)).unwrap())
        .then(&Transform::mirror(Point::um(0.2, 0.4), 0.3))
        .then(&Transform::translate(um(2.0), um(-1.5)));
    assert!(t.mirrored());
    assert!((t.factor() - 1.7).abs() < 1e-15);
    for (name, shape) in shapes() {
        let image = shape.transformed(&t);
        let s2 = t.factor() * t.factor();
        assert!(
            (image.area() - s2 * shape.area()).abs() < 1e-12 * image.area(),
            "{name}"
        );
        let mut n = Numbers(3);
        for _ in 0..50 {
            let p = n.point(&shape.bounds(), 0.5);
            let (d, di) = (shape.distance(p), image.distance(t.apply(p)));
            assert!(
                (di.signed.to_um() - t.factor() * d.signed.to_um()).abs() < 1e-9,
                "{name} at {p:?}: {d:?} {di:?}"
            );
            assert_eq!(shape.contains(p), image.contains(t.apply(p)), "{name}");
        }
        // and back: the inverse's image is the shape again
        let p = Region::perimeter(&image).unwrap().to_um();
        let q = Region::perimeter(&shape).unwrap().to_um();
        assert!((p - t.factor() * q).abs() < 1e-9, "{name}");
    }
    // a rectangle stays one under a quarter turn, and turns into a rounded one otherwise
    let r = Shape::rect(Point::um(1.0, 0.0), um(2.0), um(0.5)).unwrap();
    match r.transformed(&Transform::rotate(FRAC_PI_2, Point::um(0.0, 0.0))) {
        Shape::Rect {
            center,
            width,
            height,
        } => {
            assert_eq!(center, Point::um(0.0, 1.0));
            assert_eq!((width, height), (um(0.5), um(2.0)));
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        r.transformed(&Transform::rotate(0.3, Point::um(0.0, 0.0))),
        Shape::RoundedRect(_)
    ));
    // a translation moves a rectangle's centre and nothing else, to the bit
    let moved = r.transformed(&Transform::translate(um(0.25), um(-0.5)));
    assert_eq!(
        moved,
        Shape::rect(Point::um(1.25, -0.5), um(2.0), um(0.5)).unwrap()
    );
    // a mirror twice is nothing
    let m = Transform::mirror(Point::um(0.3, 0.1), 0.9);
    let p = Point::um(1.7, -0.4);
    let q = m.then(&m).apply(p);
    assert!((q.x - p.x).to_um().abs() < 1e-15 && (q.y - p.y).to_um().abs() < 1e-15);
    assert!(Transform::scale(0.0, p).is_err());
}

#[test]
fn fills_over_a_grid_sum_to_the_area() {
    for (name, shape) in shapes() {
        let b = shape.bounds();
        let [lo, hi] = b.corners();
        // cells of 0.137 µm from a corner off the shape's, so that edges cut them anywhere
        let h = 0.137;
        let (x0, y0) = (lo[0] - 0.05, lo[1] - 0.08);
        let (nx, ny) = (
            ((hi[0] - x0) / h).ceil() as usize + 1,
            ((hi[1] - y0) / h).ceil() as usize + 1,
        );
        let mut sum = 0.0;
        for j in 0..ny {
            for i in 0..nx {
                let cell = Bounds::new(
                    Point::um(x0 + i as f64 * h, y0 + j as f64 * h),
                    Point::um(x0 + (i + 1) as f64 * h, y0 + (j + 1) as f64 * h),
                );
                let f = shape.fill(cell);
                assert!(
                    (-1e-15..=cell.area() * (1.0 + 1e-12)).contains(&f),
                    "{name}: {f} in a cell of {}",
                    cell.area()
                );
                sum += f;
            }
        }
        let tol = if name == "superellipse" { 1e-9 } else { 1e-12 };
        assert!(
            (sum - shape.area()).abs() < tol * shape.area(),
            "{name}: {sum} against {}",
            shape.area()
        );
    }
}

#[test]
fn a_cells_fill_is_the_area_inside_it() {
    // against counting points of a fine lattice in the cell
    for (name, shape) in shapes() {
        let b = shape.bounds();
        let [lo, hi] = b.corners();
        let cell = Bounds::new(
            Point::um(lo[0] + 0.3 * (hi[0] - lo[0]), lo[1] + 0.2 * (hi[1] - lo[1])),
            Point::um(
                lo[0] + 0.8 * (hi[0] - lo[0]),
                lo[1] + 0.65 * (hi[1] - lo[1]),
            ),
        );
        let [c0, c1] = cell.corners();
        let n = 1000;
        let mut inside = 0usize;
        for j in 0..n {
            for i in 0..n {
                let p = Point::um(
                    c0[0] + (i as f64 + 0.5) / n as f64 * (c1[0] - c0[0]),
                    c0[1] + (j as f64 + 0.5) / n as f64 * (c1[1] - c0[1]),
                );
                inside += usize::from(shape.contains(p));
            }
        }
        let counted = inside as f64 / (n * n) as f64 * cell.area();
        let f = shape.fill(cell);
        // the lattice's error: about its spacing times the boundary's length inside
        assert!(
            (f - counted).abs() < 3e-3 * cell.area(),
            "{name}: {f} against {counted}"
        );
    }
}

/// Many shapes on a layer: holes of a photonic crystal, and a few others over them.
fn crystal() -> crate::stack::Structure {
    use crate::stack::{LayerStack, Structure};
    let mut s = Structure::new(LayerStack::soi_220());
    for j in 0..30 {
        for i in 0..30 {
            let x = 0.42 * f64::from(i) + 0.21 * f64::from(j % 2);
            let y = 0.42 * 0.866 * f64::from(j);
            s.draw("Si", Shape::circle(Point::um(x, y), um(0.12)).unwrap())
                .unwrap();
        }
    }
    for (_, shape) in shapes() {
        s.draw("Si", shape).unwrap();
    }
    s
}

#[test]
fn the_index_answers_as_testing_every_shape_does() {
    let s = crystal();
    let shapes = s.shapes("Si");
    let z = Length::um(2.11);
    let mut n = Numbers(5);
    let b = Bounds::new(Point::um(-2.0, -2.0), Point::um(13.0, 12.0));
    for _ in 0..20_000 {
        let p = n.point(&b, 0.0);
        let linear = shapes.iter().any(|sh| sh.contains(p));
        let indexed = s.material_at(p, z).name() == "Si";
        assert_eq!(linear, indexed, "{p:?}");
    }
    // the circles' centres and edges: on the boundaries, where rounding decides
    for sh in shapes.iter().take(50) {
        if let Shape::Circle { center, radius } = sh {
            for p in [
                *center,
                Point {
                    x: center.x + *radius,
                    y: center.y,
                },
            ] {
                let linear = shapes.iter().any(|sh| sh.contains(p));
                assert_eq!(linear, s.material_at(p, z).name() == "Si");
            }
        }
    }
    // a cell meets every shape whose box it meets
    let index = s.index("Si").unwrap();
    assert_eq!(index.len(), shapes.len());
    let cell = Bounds::new(Point::um(2.0, 2.0), Point::um(3.1, 2.7));
    let mut meeting = index.meeting(cell);
    meeting.sort_unstable();
    let expected: Vec<usize> = (0..shapes.len())
        .filter(|&k| shapes[k].bounds().overlaps(&cell))
        .collect();
    assert!(expected.iter().all(|k| meeting.contains(k)));
    assert!(s.index("Metal").is_none());
}

#[test]
fn fills_and_samples_are_the_same_bits_on_any_number_of_threads() {
    use rayon::prelude::*;
    let s = crystal();
    let shapes = s.shapes("Si");
    let index = s.index("Si").unwrap();
    let z = Length::um(2.11);
    let h = 0.05;
    let (nx, ny) = (200, 160);
    let picture = |threads: usize| -> Vec<(f64, bool)> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        pool.install(|| {
            (0..nx * ny)
                .into_par_iter()
                .map(|k| {
                    let (i, j) = ((k % nx) as f64, (k / nx) as f64);
                    let cell = Bounds::new(
                        Point::um(-0.5 + i * h, -0.5 + j * h),
                        Point::um(-0.5 + (i + 1.0) * h, -0.5 + (j + 1.0) * h),
                    );
                    let fill: f64 = index
                        .meeting(cell)
                        .into_iter()
                        .map(|k| shapes[k].fill(cell))
                        .sum();
                    let centre = Point::um(-0.5 + (i + 0.5) * h, -0.5 + (j + 0.5) * h);
                    (fill, s.material_at(centre, z).name() == "Si")
                })
                .collect()
        })
    };
    let one = picture(1);
    for threads in [2, 7, 20] {
        let many = picture(threads);
        assert!(
            one.iter()
                .zip(&many)
                .all(|(a, b)| a.0.to_bits() == b.0.to_bits() && a.1 == b.1),
            "{threads} threads"
        );
    }
}

/// How long sampling a large grid takes with the index and by testing every shape:
/// `cargo test --release --lib material_at_timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, not a check"]
fn material_at_timing() {
    use crate::stack::{LayerStack, Structure};
    use std::time::Instant;
    // a photonic crystal of 100 × 100 holes, sampled on a grid of 2000 × 2000 points
    let mut s = Structure::new(LayerStack::soi_220());
    for j in 0..100 {
        for i in 0..100 {
            let x = 0.42 * f64::from(i) + 0.21 * f64::from(j % 2);
            let y = 0.42 * 0.866 * f64::from(j);
            s.draw("Si", Shape::circle(Point::um(x, y), um(0.12)).unwrap())
                .unwrap();
        }
    }
    let shapes = s.shapes("Si").to_vec();
    let z = Length::um(2.11);
    let n = 2000;
    let (w, hgt) = (42.0, 36.4);
    let start = Instant::now();
    let mut count = 0usize;
    for j in 0..n {
        for i in 0..n {
            let p = Point::um(
                w * f64::from(i) / f64::from(n),
                hgt * f64::from(j) / f64::from(n),
            );
            count += usize::from(s.material_at(p, z).name() == "Si");
        }
    }
    let indexed = start.elapsed();
    let start = Instant::now();
    let mut linear = 0usize;
    for j in 0..n / 10 {
        for i in 0..n {
            let p = Point::um(
                w * f64::from(i) / f64::from(n),
                hgt * f64::from(j) / f64::from(n),
            );
            linear += usize::from(shapes.iter().any(|sh| sh.contains(p)));
        }
    }
    let every = start.elapsed() * 10;
    println!(
        "{} points, {} shapes: {:.3} s with the index ({:.1} ns a point), {:.1} s testing every \
         shape (from a tenth of them); {count} and {linear} inside",
        n * n,
        shapes.len(),
        indexed.as_secs_f64(),
        indexed.as_secs_f64() * 1e9 / f64::from(n * n),
        every.as_secs_f64()
    );
}
