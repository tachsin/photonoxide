//! Tests of the shapes defined by functions (between two curves, polar, closed curves,
//! implicit) and of masks.

use std::f64::consts::PI;

use super::checks::Numbers;
use super::curve_checks::{self, expr};
use super::*;
use crate::expr::{Expr, Parameters};
use crate::units::Length;

fn um(v: f64) -> Length {
    Length::um(v)
}

fn none() -> Parameters {
    Parameters::none()
}

#[test]
fn a_circle_written_four_ways_is_the_circle() {
    let e = curve_checks::circle_error();
    assert!(e < 1e-12, "{e}");
}

#[test]
fn a_linear_taper_between_two_curves_is_the_path_taper() {
    let e = curve_checks::taper_error();
    assert!(e < 1e-12, "{e}");
}

#[test]
fn the_published_y_junction_through_its_widths() {
    let e = curve_checks::zhang_error();
    assert!(e < 1e-12, "{e}");
    // its widest part, 1.4 µm, from 1 µm to 1.5 µm along it
    let t = curve_checks::zhang_taper().unwrap();
    let b = Region::bounds(&t);
    assert!(
        b.max.y.to_um() >= 0.7 - 1e-12 && b.max.y.to_um() < 0.75,
        "{b:?}"
    );
    assert_eq!((b.min.x.to_um(), b.max.x.to_um()), (0.0, 2.0));
}

#[test]
fn the_implicit_regions_holes_and_parts() {
    let shapes = curve_checks::closed_forms();
    let find = |n: &str| {
        shapes
            .iter()
            .find(|s| s.0 == n)
            .map(|s| s.1.clone())
            .unwrap()
    };
    let annulus = find("implicit annulus").polygons(Tolerance::default());
    assert_eq!(annulus.contours.len(), 1);
    assert_eq!(annulus.contours[0].holes.len(), 1);
    let disks = find("two disks").polygons(Tolerance::default());
    assert_eq!(disks.contours.len(), 2);
    assert!(disks.contours.iter().all(|c| c.holes.is_empty()));
    let annulus = find("implicit annulus");
    assert!(annulus.contains(Point::um(0.75, 0.0)));
    assert!(!annulus.contains(Point::um(0.2, 0.1)));
    let d = annulus.distance(Point::um(0.1, 0.0));
    assert!((d.signed.to_um() - 0.4).abs() < 1e-12, "{d:?}");
    // the outward normal of the hole points into it
    assert!((d.normal[0] + 1.0).abs() < 1e-9, "{d:?}");
}

#[test]
fn distances_polygons_and_fills() {
    let d = curve_checks::distance_error();
    assert!(d < 2e-3, "{d} nm");
    let (p, a) = curve_checks::polygon_errors();
    assert!(p <= 1.0 && a < 1e-12, "{p} nm, {a}");
    let (sum, cell) = curve_checks::fill_errors();
    assert!(sum < 1e-12 && cell < 5e-6, "{sum} {cell}");
}

#[test]
fn polygons_converge_at_second_order() {
    let order = curve_checks::polygon_order();
    assert!((order - 2.0).abs() < 0.1, "{order}");
}

#[test]
fn the_implicit_estimate_is_first_order_and_its_error_second() {
    let order = curve_checks::estimate_order();
    assert!((order - 2.0).abs() < 0.05, "{order}");
    // exact on the boundary
    let Some((_, Shape::Implicit(c))) = curve_checks::circles()
        .into_iter()
        .find(|s| s.0 == "implicit")
    else {
        panic!("the implicit circle");
    };
    let [x, y] = curve_checks::CENTRE;
    let e = c.estimate(Point::um(x + curve_checks::RADIUS, y)).to_um();
    assert!(e.abs() < 1e-15, "{e}");
}

#[test]
fn masks() {
    let e = curve_checks::mask_error();
    assert!(e < 1e-12, "{e}");
    let rows = curve_checks::mask_rows();
    let m = Mask::from_rows(&rows, Point::um(-0.3, 0.2), [um(0.2), um(0.15)]).unwrap();
    assert_eq!(m.size(), [7, 5]);
    // the hole (two pixels below ½ in the second row) and the pixels meeting at a corner
    let p = m.polygons(Tolerance::unsnapped(um(1e-9)).unwrap());
    let holes: usize = p.contours.iter().map(|c| c.holes.len()).sum();
    assert_eq!((p.contours.len(), holes), (3, 1), "{p:?}");
    // a pixel of 0.6 is in, one of 0.3 out; an edge between in and out is the boundary
    let pixel = |i: f64, j: f64| Point::um(-0.3 + 0.2 * i, 0.2 + 0.15 * j);
    assert_eq!(m.locate(pixel(4.5, 2.5)), Location::Inside);
    assert_eq!(m.locate(pixel(2.5, 1.5)), Location::Outside);
    assert_eq!(m.locate(pixel(2.0, 3.5)), Location::Boundary);
    assert_eq!(m.locate(pixel(1.5, 2.0)), Location::Boundary);
    assert_eq!(m.locate(pixel(2.0, 2.5)), Location::Inside);
    // the distance from the hole's middle to its edges
    let d = m.distance(pixel(3.0, 3.5));
    assert!((d.signed.to_um() - 0.075).abs() < 1e-15, "{d:?}");
    // bounds: the pixels with ρ > 0
    let b = Region::bounds(&m);
    assert!((b.min.x.to_um() + 0.1).abs() < 1e-15 && (b.min.y.to_um() - 0.35).abs() < 1e-15);
    // from text, the top row first
    let text = "# a mask\n0, 1\n1 0.25\n";
    let t = Mask::parse(text, Point::um(0.0, 0.0), [um(1.0), um(1.0)]).unwrap();
    assert_eq!(t.values(), &[1.0, 0.25, 0.0, 1.0]);
    assert!((t.area() - 2.25).abs() < 1e-15);
    assert!((t.binarised_area() - 2.0).abs() < 1e-15);
    // two pixels at a corner: two squares
    assert_eq!(t.polygons(Tolerance::default()).contours.len(), 2);
}

#[test]
fn what_is_refused() {
    let p = none();
    let err = |r: crate::Result<Shape>| r.map(|_| ()).unwrap_err().to_string();
    // between: crossing curves, a coordinate it has no value of, an empty range
    let e = err(Between::new(
        &expr("-0.5 um + 0.3*x"),
        &expr("0.5 um"),
        [um(0.0), um(5.0)],
        &p,
    )
    .map(Shape::Between));
    assert!(e.contains("meets or crosses the upper at x = 3.33"), "{e}");
    let e =
        err(Between::new(&expr("y"), &expr("1 um"), [um(0.0), um(1.0)], &p).map(Shape::Between));
    assert!(e.contains("reads y"), "{e}");
    let e = err(Between::new(&expr("0"), &expr("1"), [um(0.0), um(1.0)], &p).map(Shape::Between));
    assert!(e.contains("needs a length"), "{e}");
    assert!(Between::new(&expr("0 um"), &expr("1 um"), [um(1.0), um(1.0)], &p).is_err());
    // a pointed end is a region
    assert!(Between::new(&expr("-0.1*x"), &expr("0.1*x"), [um(0.0), um(1.0)], &p).is_ok());
    // polar: a radius that isn't positive, or isn't periodic
    let e =
        err(Polar::new(Point::um(0.0, 0.0), &expr("0.5 um + 1 um*cos(t)"), &p).map(Shape::Polar));
    assert!(e.contains("must be positive"), "{e}");
    let e = err(Polar::new(Point::um(0.0, 0.0), &expr("1 um + 0.1 um*t"), &p).map(Shape::Polar));
    assert!(e.contains("isn't periodic"), "{e}");
    // closed curves: one that doesn't close, a figure eight, one that stops
    let e = err(
        ClosedCurve::new(&expr("cos(pi*t)*1um"), &expr("sin(pi*t)*1um"), &p)
            .map(Shape::ClosedCurve),
    );
    assert!(e.contains("doesn't close"), "{e}");
    let e = err(
        ClosedCurve::new(&expr("sin(2*pi*t)*1um"), &expr("sin(4*pi*t)*0.5um"), &p)
            .map(Shape::ClosedCurve),
    );
    assert!(e.contains("crosses or touches itself"), "{e}");
    let e = err(
        ClosedCurve::new(&expr("cos(2*pi*t)^3*1um"), &expr("sin(2*pi*t)^3*1um"), &p)
            .map(Shape::ClosedCurve),
    );
    assert!(e.contains("stops"), "{e}");
    // implicit: no zero set in the window, a region reaching its edge, a singular point
    let window = Bounds::new(Point::um(-2.0, -2.0), Point::um(2.0, 2.0));
    let e = err(Implicit::new(&expr("x^2 + y^2 + 1 um^2"), window, &p).map(Shape::Implicit));
    assert!(e.contains("no zero set"), "{e}");
    let e = err(Implicit::new(&expr("x^2 + y^2 - 9 um^2"), window, &p).map(Shape::Implicit));
    assert!(e.contains("window's edge"), "{e}");
    let e = err(
        Implicit::new(&expr("(x^2 + y^2)^2 - 2 um^2*(x^2 - y^2)"), window, &p).map(Shape::Implicit),
    );
    assert!(e.contains("gradient"), "{e}");
    let e = err(Implicit::new(&expr("t - 1"), window, &p).map(Shape::Implicit));
    assert!(e.contains("reads t"), "{e}");
    // masks
    let corner = Point::um(0.0, 0.0);
    let px = [um(0.1), um(0.1)];
    assert!(Mask::new(corner, px, 2, vec![0.0, 1.5]).is_err());
    assert!(Mask::new(corner, px, 2, vec![0.0, 0.4]).is_err());
    assert!(Mask::new(corner, px, 2, vec![1.0, 1.0, 1.0]).is_err());
    assert!(Mask::new(corner, [um(0.0), um(0.1)], 1, vec![1.0]).is_err());
    assert!(Mask::parse("1 0\n1\n", corner, px).is_err());
    assert!(Mask::parse("1 x\n", corner, px).is_err());
}

#[test]
fn parameters_reach_the_definitions() {
    let p = Parameters::new(&[
        ("w".to_owned(), "500 nm".to_owned()),
        ("r0".to_owned(), "2 um".to_owned()),
    ])
    .unwrap();
    let lower = Expr::parse("-w/2", p.scope()).unwrap();
    let upper = Expr::parse("w/2 + 0.1*x", p.scope()).unwrap();
    let t = Between::new(&lower, &upper, [um(0.0), um(2.0)], &p).unwrap();
    assert!(
        (t.area() - (0.5 * 2.0 + 0.1 * 2.0)).abs() < 1e-13,
        "{}",
        t.area()
    );
    let wider = p.with("w", 0.7).unwrap();
    let t = Between::new(&lower, &upper, [um(0.0), um(2.0)], &wider).unwrap();
    assert!(
        (t.area() - (0.7 * 2.0 + 0.1 * 2.0)).abs() < 1e-13,
        "{}",
        t.area()
    );
    let rho = Expr::parse("r0", p.scope()).unwrap();
    let c = Polar::new(Point::um(1.0, 0.0), &rho, &p).unwrap();
    assert!((c.area() - PI * 4.0).abs() < 1e-12);
    assert_eq!(c.center(), Point::um(1.0, 0.0));
}

#[test]
fn transforms_keep_each_kind() {
    let scale = Transform::scale(1.7, Point::um(-1.0, 0.3)).unwrap();
    let t = Transform::rotate(0.7, Point::um(0.5, -0.2))
        .then(&scale)
        .then(&Transform::mirror(Point::um(0.2, 0.4), 0.3))
        .then(&Transform::translate(um(2.0), um(-1.5)));
    let s = t.factor();
    let mut shapes: Vec<Shape> = curve_checks::closed_forms()
        .into_iter()
        .map(|s| s.1)
        .collect();
    let m = Mask::from_rows(
        &curve_checks::mask_rows(),
        Point::um(-0.3, 0.2),
        [um(0.2), um(0.15)],
    )
    .unwrap();
    shapes.push(Shape::Mask(m));
    for shape in shapes {
        let image = shape.transformed(&t);
        assert_eq!(
            std::mem::discriminant(&image),
            std::mem::discriminant(&shape)
        );
        let e = ((image.area() - s * s * shape.area()) / image.area()).abs();
        assert!(e < 1e-12, "{shape:?}: {e}");
        let mut n = Numbers(7);
        for _ in 0..40 {
            let p = n.point(&shape.bounds(), 0.4);
            let (d, di) = (shape.distance(p), image.distance(t.apply(p)));
            let e = (di.signed.to_um() - s * d.signed.to_um()).abs();
            assert!(e < 1e-9, "{e}");
        }
    }
}

#[test]
fn the_same_bits_on_any_thread() {
    let probe = || -> Vec<u64> {
        let mut out = Vec::new();
        for (_, shape, _) in curve_checks::closed_forms() {
            let mut n = Numbers(5);
            for _ in 0..20 {
                let p = n.point(&shape.bounds(), 0.3);
                let d = shape.distance(p);
                let cell = Bounds::new(p, Point::um(p.x.to_um() + 0.1, p.y.to_um() + 0.1));
                out.extend([d.signed.to_um().to_bits(), shape.fill(cell).to_bits()]);
            }
        }
        out
    };
    let here = probe();
    let threads: Vec<Vec<u64>> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..3).map(|_| s.spawn(probe)).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for t in threads {
        assert_eq!(t, here);
    }
}

#[test]
fn drawn_in_a_structure() {
    use crate::stack::{LayerStack, Structure};
    let mut s = Structure::new(LayerStack::soi_220());
    let t = curve_checks::taper().unwrap();
    s.draw("Si", Shape::Between(t)).unwrap();
    let z = Length::um(s.stack().layer("Si").unwrap().1.to_um() + 0.1);
    let at = |x: f64, y: f64| s.material_at(Point::um(x, y), z).name().to_owned();
    assert_eq!(at(2.0, 0.3), "Si");
    assert_ne!(at(2.0, 0.4), "Si");
}
