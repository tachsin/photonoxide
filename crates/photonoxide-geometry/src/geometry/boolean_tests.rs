//! Booleans, offsets and corner rounding: the validation cases' checks, the regularized inside
//! test on shared edges, distances on and off the boundary, what is refused, transforms, every
//! kind of operand, fills, and polygons within their tolerance.

use std::f64::consts::PI;

use super::boolean_checks::{self as checks, circle, rect};
use super::*;
use crate::expr::{Expr, Parameters};
use crate::units::Length;

fn um(v: f64) -> Length {
    Length::um(v)
}

#[test]
fn areas_against_closed_forms_and_inclusion_exclusion() {
    let e = checks::area_error();
    assert!(e < 1e-9, "{e}");
}

#[test]
fn polygons_are_within_the_tolerance_oriented_and_without_slivers() {
    let (dev, area, bad) = checks::polygon_errors();
    assert!(dev <= 1.0, "{dev} nm");
    assert!(area <= 1.0, "{area}");
    assert_eq!(bad, 0);
}

#[test]
fn degenerate_pairs_combine_as_they_should() {
    let e = checks::degenerate_error();
    assert!(e < 1e-12, "{e}");
}

#[test]
fn the_composed_distance_against_the_exact_boundary() {
    let (off, on) = checks::distance_errors();
    assert!(off < 1e-3, "{off} nm");
    assert!(on < 1e-9, "{on} nm");
}

#[test]
fn offsets_against_steiners_formula_and_the_joins() {
    let (region, poly) = checks::offset_errors();
    assert!(region < 1e-6, "{region}");
    assert!(poly <= 1.0, "{poly}");
}

#[test]
fn fillets_against_the_corner_cuts() {
    let (region, poly) = checks::fillet_errors();
    assert!(region < 1e-8, "{region}");
    assert!(poly <= 1.0, "{poly}");
}

#[test]
fn polygons_converge_at_second_order() {
    let order = checks::order();
    assert!((order - 2.0).abs() < 0.15, "{order}");
}

#[test]
fn zhangs_y_junction_as_a_union() {
    let e = checks::zhang_union_error();
    assert!(e < 1e-12, "{e}");
}

#[test]
fn shapes_sharing_an_edge_are_one_region_across_it() {
    let (a, b) = (rect(0.0, 0.0, 1.0, 1.0), rect(1.0, 0.0, 2.0, 1.0));
    let u = a.union(&b).unwrap();
    // the shared edge is inside the union, its ends on the boundary
    assert_eq!(u.locate(Point::um(1.0, 0.5)), Location::Inside);
    assert_eq!(u.locate(Point::um(1.0, 1.0)), Location::Boundary);
    assert_eq!(u.locate(Point::um(2.0, 0.5)), Location::Boundary);
    assert!((u.area() - 2.0).abs() < 1e-15);
    // its distance there is to the union's boundary, not to the edge
    let d = u.distance(Point::um(1.0, 0.5)).signed.to_um();
    assert!((d + 0.5).abs() < 1e-6, "{d}");
    // the xor of the two is their union; they have no intersection
    let x = a.xor(&b).unwrap();
    assert_eq!(x.locate(Point::um(1.0, 0.5)), Location::Inside);
    assert!((x.area() - 2.0).abs() < 1e-15);
    let e = a.intersection(&b).unwrap_err().to_string();
    assert!(e.contains("empty"), "{e}");
    // a shape less itself is empty; less one beside it, itself
    assert!(a.difference(&a).is_err());
    let d = a.difference(&b).unwrap();
    assert_eq!(d.locate(Point::um(1.0, 0.5)), Location::Boundary);
    assert!((d.area() - 1.0).abs() < 1e-15);
    // two squares meeting at a corner: the corner is on the union's boundary
    let c = a.union(&rect(1.0, 1.0, 2.0, 2.0)).unwrap();
    assert_eq!(c.locate(Point::um(1.0, 1.0)), Location::Boundary);
    assert!(!c.contains(Point::um(1.5, 0.5)));
}

#[test]
fn distances_and_normals_on_and_off_the_boundary() {
    let (a, b) = (circle(0.0, 0.0, 1.0), circle(1.0, 0.0, 1.0));
    let u = a.union(&b).unwrap();
    // outside a union its distance is the nearer operand's, exactly
    let p = Point::um(3.0, 0.5);
    let d = u.distance(p);
    let exact = (2.0f64 * 2.0 + 0.25).sqrt() - 1.0;
    assert_eq!(d.signed.to_um(), b.distance(p).signed.to_um());
    assert!((d.signed.to_um() - exact).abs() < 1e-15);
    assert!((d.normal[0] - 2.0 / 4.25f64.sqrt()).abs() < 1e-15);
    // deep inside it the nearest point of either circle is inside the other: the distance is to
    // the union's boundary, the corners where the circles cross, within 1e-7 of its size
    let d = u.distance(Point::um(0.5, 0.0));
    assert!(
        (d.signed.to_um() + 0.75f64.sqrt()).abs() < 3e-7 * 3.0,
        "{d:?}"
    );
    assert!((d.normal[1].abs() - 1.0).abs() < 1e-6, "{d:?}");
    // the difference's boundary inside A is B's circle: its outward normal points into B
    let diff = a.difference(&b).unwrap();
    let p = Point::um(-0.1, 0.0);
    let d = diff.distance(p);
    assert!((d.signed.to_um() - (-0.1)).abs() < 1e-15, "{d:?}");
    assert!((d.normal[0] - 1.0).abs() < 1e-15);
    // the intersection inside: exact, the nearer circle
    let i = a.intersection(&b).unwrap();
    let d = i.distance(Point::um(0.6, 0.0));
    assert!((d.signed.to_um() + 0.4).abs() < 1e-15, "{d:?}");
    // a signed distance's sign is the inside test's everywhere
    for k in 0..400 {
        let t = f64::from(k) * 0.123;
        let p = Point::um(0.5 + 1.8 * t.cos() * (t * 0.37).sin(), 1.4 * t.sin());
        for s in [&u, &diff, &i] {
            let (d, inside) = (s.distance(p).signed.to_um(), s.contains(p));
            assert!(d <= 0.0 && inside || d >= 0.0 && !inside, "{p:?} {d}");
        }
    }
}

#[test]
fn a_round_offset_tests_points_exactly() {
    let c = circle(0.2, -0.1, 1.0);
    let g = c.offset(um(0.25), Join::Round).unwrap();
    // inside within 1.25 of the centre, its distance there exact
    assert!(g.contains(Point::um(0.2 + 1.25 - 1e-12, -0.1)));
    assert!(!g.contains(Point::um(0.2 + 1.25 + 1e-12, -0.1)));
    let d = g.distance(Point::um(0.2, 2.0)).signed.to_um();
    assert!((d - (2.1 - 1.25)).abs() < 1e-15, "{d}");
    let b = g.bounds();
    assert_eq!(b.max.x.to_um(), 0.2 + 1.0 + 0.25);
    // a rectangle shrunk keeps its corners square
    let r = rect(0.0, 0.0, 2.0, 1.0)
        .offset(um(-0.2), Join::Round)
        .unwrap();
    assert!(r.contains(Point::um(0.2 + 1e-9, 0.2 + 1e-9)));
    assert!(!r.contains(Point::um(0.2 - 1e-9, 0.5)));
    let d = r.distance(Point::um(1.0, 0.5)).signed.to_um();
    assert!((d + 0.3).abs() < 1e-15, "{d}");
    // grown with miters, it is a rectangle again
    let m = rect(0.0, 0.0, 2.0, 1.0)
        .offset(um(0.2), Join::miter(2.0))
        .unwrap();
    assert!(m.contains(Point::um(2.2 - 1e-9, 1.2 - 1e-9)));
    assert!((m.area() - 2.4 * 1.4).abs() < 1e-14);
}

#[test]
fn what_is_refused() {
    let r = rect(0.0, 0.0, 2.0, 1.0);
    let e = |s: Result<Shape>| s.unwrap_err().to_string();
    assert!(e(r.offset(um(0.1), Join::miter(0.5))).contains("at least 1"));
    assert!(e(r.offset(um(f64::NAN), Join::Round)).contains("finite"));
    assert!(e(r.offset(um(-0.5), Join::Round)).contains("leaves nothing"));
    assert!(e(r.rounded(um(-0.1), um(0.0))).contains("radii"));
    assert!(e(r.rounded(um(0.6), um(0.0))).contains("leaves nothing"));
    assert!(e(r.intersection(&rect(3.0, 0.0, 4.0, 1.0))).contains("empty"));
    let p = r.polygons(Tolerance::default());
    assert!(
        p.offset(um(0.1), Join::miter(0.9), Tolerance::default())
            .is_err()
    );
    assert!(
        p.rounded(um(f64::INFINITY), um(0.0), Tolerance::default())
            .is_err()
    );
}

#[test]
fn transforms_carry_the_operations() {
    let a = circle(0.0, 0.0, 1.0);
    let s = a
        .difference(&rect(0.0, -0.3, 2.0, 0.3))
        .unwrap()
        .offset(um(0.1), Join::Round)
        .unwrap()
        .rounded(um(0.05), um(0.05))
        .unwrap();
    let t = Transform::rotate(0.7, Point::um(0.3, 0.1))
        .then(&Transform::scale(1.5, Point::um(0.0, 0.0)).unwrap())
        .then(&Transform::mirror(Point::um(1.0, 0.0), 0.2));
    let image = s.transformed(&t);
    assert!(((image.area() - 2.25 * s.area()) / s.area()).abs() < 1e-6);
    let p = Point::um(-0.5, 0.2);
    let (d, di) = (s.distance(p), image.distance(t.apply(p)));
    assert!((di.signed.to_um() - 1.5 * d.signed.to_um()).abs() < 1e-6);
    assert_eq!(s.contains(p), image.contains(t.apply(p)));
}

fn expr(text: &str) -> Expr {
    Expr::parse(text, Parameters::none().scope()).unwrap()
}

#[test]
fn booleans_of_every_kind_of_shape() {
    let none = Parameters::none();
    let path = Path::builder(Point::um(-2.0, 0.0), 0.0, um(0.5))
        .segment(Segment::Straight { length: um(2.0) })
        .segment(Segment::Arc {
            radius: um(2.0),
            angle: PI / 2.0,
        })
        .build()
        .unwrap();
    let path = Shape::Path(path);
    let taper = Shape::Between(
        Between::new(
            &expr("-0.2 um - 0.1*x"),
            &expr("0.2 um + 0.1*x"),
            [um(-1.0), um(1.0)],
            &none,
        )
        .unwrap(),
    );
    let flower = Shape::Polar(
        Polar::new(
            Point::um(0.5, 1.0),
            &expr("0.8 um*(1 + 0.2*cos(5*t))"),
            &none,
        )
        .unwrap(),
    );
    let blob = Shape::Implicit(
        Implicit::new(
            &expr("(x/1um)^4 + (y/0.6um)^4 - 1"),
            Bounds::new(Point::um(-1.5, -1.0), Point::um(1.5, 1.0)),
            &none,
        )
        .unwrap(),
    );
    let mask = Shape::Mask(
        Mask::from_rows(
            &[vec![1.0, 1.0], vec![1.0, 0.0]],
            Point::um(-0.4, -0.4),
            [um(0.4), um(0.4)],
        )
        .unwrap(),
    );
    let shapes = [&path, &taper, &flower, &blob, &mask];
    for (i, a) in shapes.iter().enumerate() {
        for b in &shapes[i + 1..] {
            let (Ok(u), Ok(n)) = (a.union(b), a.intersection(b)) else {
                // shapes that don't overlap have no intersection
                let u = a.union(b).unwrap();
                assert!((u.area() - a.area() - b.area()).abs() < 1e-6 * u.area());
                continue;
            };
            // inclusion–exclusion within the polygons' 1e-7
            let e = (u.area() + n.area() - a.area() - b.area()).abs() / (a.area() + b.area());
            assert!(e < 1e-6, "{e}");
            let x = a.xor(b).unwrap();
            assert!(((x.area() - u.area() + n.area()) / u.area()).abs() < 1e-6);
        }
    }
    // booleans of booleans, and of offsets
    let nested = taper
        .union(&flower)
        .unwrap()
        .difference(&blob.offset(um(-0.2), Join::Round).unwrap())
        .unwrap();
    assert!(nested.area() > 0.0);
    let p = nested.polygons(Tolerance::default());
    assert!(p.deviation.to_um() <= 1e-3, "{}", p.deviation);
    // a path grown by its own half-width more is its outline widened
    let grown = path.offset(um(0.1), Join::Round).unwrap();
    let wider = Path::builder(Point::um(-2.0, 0.0), 0.0, um(0.7))
        .segment(Segment::Straight { length: um(2.0) })
        .segment(Segment::Arc {
            radius: um(2.0),
            angle: PI / 2.0,
        })
        .build()
        .unwrap();
    // the two differ by what the offset adds beyond each end: the end pushed out by 0.1 µm and
    // a quarter disk of 0.1 µm at each of its corners
    let caps = 2.0 * (0.5 * 0.1 + PI * 0.01 / 2.0);
    let e = (grown.area() - wider.area() - caps).abs();
    assert!(e < 1e-6, "{e}");
}

#[test]
fn shrinking_deeper_than_a_curves_radius() {
    // a three-petalled flower shrunk past its petals' tips' radius of curvature, every join:
    // its polygons' many short edges' pieces overlap one another
    let none = Parameters::none();
    let flower = Shape::Polar(
        Polar::new(Point::um(0.0, 0.0), &expr("1 um*(1 + 0.2*cos(3*t))"), &none).unwrap(),
    );
    let start = std::time::Instant::now();
    for join in [Join::Round, Join::miter(3.0), Join::Square] {
        let s = flower.offset(um(-0.6), join).unwrap();
        assert!(
            s.area() > 0.0 && s.area() < PI * 0.4 * 0.4 * 1.1,
            "{}",
            s.area()
        );
        // a disk about the centre of radius 0.8·1 − 0.6 µm is inside
        assert!(s.contains(Point::um(0.19, 0.0)));
    }
    let disk = circle(0.0, 0.0, 1.0)
        .offset(um(-0.99), Join::Round)
        .unwrap();
    assert!((disk.area() - PI * 1e-4).abs() < 1e-8, "{}", disk.area());
    assert!(
        start.elapsed().as_secs_f64() < 20.0,
        "{:?}",
        start.elapsed()
    );
}

#[test]
fn fills_sum_to_the_area_on_any_thread_count() {
    use rayon::prelude::*;
    let s = circle(0.0, 0.0, 1.0)
        .difference(&circle(0.6, 0.3, 0.5))
        .unwrap()
        .offset(um(0.05), Join::Square)
        .unwrap();
    let b = s.bounds();
    let [lo, hi] = b.corners();
    let n = 40;
    let (hx, hy) = (
        (hi[0] - lo[0]) / f64::from(n),
        (hi[1] - lo[1]) / f64::from(n),
    );
    let cells: Vec<Bounds> = (0..n * n)
        .map(|k| {
            let (i, j) = (f64::from(k % n), f64::from(k / n));
            Bounds::new(
                Point::um(lo[0] + i * hx, lo[1] + j * hy),
                Point::um(lo[0] + (i + 1.0) * hx, lo[1] + (j + 1.0) * hy),
            )
        })
        .collect();
    let fills = |threads: usize| -> Vec<f64> {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| cells.par_iter().map(|c| s.fill(*c)).collect())
    };
    let (one, many) = (fills(1), fills(4));
    assert_eq!(one, many);
    let sum: f64 = one.iter().sum();
    assert!(
        (sum - s.area()).abs() < 1e-12 * s.area(),
        "{sum} {}",
        s.area()
    );
}

#[test]
fn polygons_booleans_offsets_and_rounding() {
    let t = Tolerance::default();
    let a = rect(0.0, 0.0, 2.0, 1.0).polygons(t);
    let b = circle(2.0, 0.5, 0.5).polygons(t);
    let u = a.boolean(&b, Operation::Union, t);
    assert_eq!(u.contours.len(), 1);
    assert!((u.area() - (2.0 + PI * 0.125)).abs() < 1e-3 * 6.0);
    let grown = a.offset(um(0.1), Join::Round, t).unwrap();
    let exact = 2.0 + 6.0 * 0.1 + PI * 0.01;
    assert!(
        (grown.area() - exact).abs() < 1e-3 * 6.7,
        "{}",
        grown.area()
    );
    let rounded = a.rounded(um(0.2), um(0.0), t).unwrap();
    let exact = 2.0 - (4.0 - PI) * 0.04;
    assert!(
        (rounded.area() - exact).abs() < 1e-3 * 6.0,
        "{}",
        rounded.area()
    );
    // every vertex on the grid
    for c in &rounded.contours {
        for v in &c.outer {
            let x = v.x.to_um() * 1e3;
            assert!((x - x.round()).abs() < 1e-9, "{x}");
        }
    }
}
