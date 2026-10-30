//! Arrays, lattices and references: the validation cases' checks, what a query costs on a
//! large lattice, the union where elements touch or overlap, the same bits on any number of
//! threads, and what is refused.

use std::f64::consts::PI;

use super::array::TESTED;
use super::array_checks as checks;
use super::boolean_checks::{circle, rect};
use super::*;
use crate::units::Length;

fn um(v: f64) -> Length {
    Length::um(v)
}

/// How many elements `f` tests.
fn tests_of(f: impl FnOnce()) -> usize {
    let before = TESTED.with(std::cell::Cell::get);
    f();
    TESTED.with(std::cell::Cell::get) - before
}

#[test]
fn lattices_against_their_closed_forms() {
    let e = checks::lattice_error();
    assert!(e < 1e-14, "{e}");
}

#[test]
fn hole_areas_and_fill_fractions() {
    let e = checks::area_error();
    assert!(e < 1e-12, "{e}");
}

#[test]
fn elements_from_their_index_and_place() {
    let e = checks::index_error();
    assert!(e < 1e-12, "{e}");
}

#[test]
fn defects_remove_shift_and_resize_what_they_say() {
    assert_eq!(checks::defect_errors(), 0);
}

#[test]
fn references_are_their_shapes_transformed_to_the_bit() {
    assert_eq!(checks::reference_errors(), 0);
}

#[test]
fn a_slab_less_holes_by_inclusion_exclusion() {
    let e = checks::slab_error();
    assert!(e < 1e-9, "{e}");
}

#[test]
fn the_polygons_converge_at_second_order() {
    let o = checks::order();
    assert!((o - 2.0).abs() < 0.15, "{o}");
}

#[test]
fn akahanes_l3_cavity() {
    let e = checks::akahane_error();
    assert!(e < 1e-12, "{e}");
}

/// An n × n hexagonal lattice of holes of radius 0.12 µm, 0.42 µm apart: rows along x, the
/// holes' centres from 0 to 0.42 (n − ½) µm along x and to 0.42 (n − 1) √3/2 µm along y.
fn holes(n: i64) -> Array {
    let l = Lattice::hexagonal(um(0.42), [0, n - 1], [0, n - 1]).unwrap();
    Array::of(&l, &circle(0.0, 0.0, 0.12)).unwrap()
}

/// A 100 × 100 hexagonal lattice of holes: 10⁴ elements.
fn big() -> Array {
    holes(100)
}

#[test]
fn a_point_or_a_cell_tests_only_the_elements_near_it() {
    // 50 × 50 holes in 21 × 18 µm: the bounds below are the index's, the same for any number of
    // elements, and 2500 is still far more than any of them, so a query that scanned the
    // lattice fails them here as it would on 10⁴; the time went on the slab less 10⁴ disks
    let a = holes(50);
    assert_eq!(a.len(), 2500);
    assert!(a.is_separate());
    // a hole's centre, a point between holes, a point outside the lattice
    for p in [
        Point::um(0.42 * 25.0, 0.0),
        Point::um(0.42 * 25.25, 0.42 * 3f64.sqrt() * 10.25),
        Point::um(-3.0, 5.0),
    ] {
        let n = tests_of(|| {
            a.contains(p);
        });
        // (a leaf of the index holds up to four elements, each tested)
        assert!(n <= 8, "contains tested {n}");
        let n = tests_of(|| {
            a.distance(p);
        });
        assert!(n <= 40, "distance tested {n}");
    }
    // a cell of 0.1 µm meets at most a few holes; a cell of 2 µm a few dozen
    for (side, most) in [(0.1, 4), (2.0, 60)] {
        let cell = Bounds::new(Point::um(10.0, 10.0), Point::um(10.0 + side, 10.0 + side));
        let n = tests_of(|| {
            a.fill(cell);
        });
        assert!(n <= most, "fill of {side} um tested {n}");
    }
    // a slab less the lattice: its polygons' rings are found through their own index
    let slab = rect(-1.0, -1.0, 22.0, 19.0)
        .difference(&Shape::Array(a.clone()))
        .unwrap();
    let p = Point::um(10.0, 10.0);
    let n = tests_of(|| {
        slab.contains(p);
    });
    assert!(n <= 4, "the slab's inside test tested {n}");
    let cell = Bounds::new(Point::um(10.0, 10.0), Point::um(10.2, 10.2));
    let n = tests_of(|| {
        slab.fill(cell);
    });
    assert!(n <= 8, "the slab's fill tested {n}");
    let holes = 2500.0 * PI * 0.12 * 0.12;
    assert!(((slab.area() - (23.0 * 20.0 - holes)) / slab.area()).abs() < 1e-9);
}

#[test]
fn material_at_through_a_structure_reads_the_lattice() {
    use crate::stack::{LayerStack, Structure};
    // 20 × 20 holes in 8.3 × 7 µm: three points are read, and the index's bound on the
    // elements tested doesn't depend on how many there are
    let mut s = Structure::new(LayerStack::soi_220());
    let lattice = Shape::Array(holes(20));
    s.draw(
        "Si",
        rect(-1.0, -1.0, 9.0, 8.0).difference(&lattice).unwrap(),
    )
    .unwrap();
    let z = Length::um(2.11);
    let at = |x: f64, y: f64| s.material_at(Point::um(x, y), z).name().to_owned();
    assert_ne!(at(0.42 * 10.0, 0.0), "Si", "a hole");
    assert_eq!(at(0.42 * 10.5, 0.0), "Si", "between holes");
    assert_eq!(at(-0.5, -0.5), "Si", "the slab's edge");
    let n = tests_of(|| {
        s.material_at(Point::um(0.42 * 10.0, 0.0), z);
    });
    assert!(n <= 4, "{n}");
}

#[test]
fn the_same_bits_on_any_number_of_threads() {
    let a = big();
    let cells: Vec<Bounds> = (0..400)
        .map(|k| {
            let (x, y) = (0.137 * f64::from(k % 20), 0.113 * f64::from(k / 20));
            Bounds::new(Point::um(x, y), Point::um(x + 0.21, y + 0.17))
        })
        .collect();
    let one: Vec<(u64, u64)> = cells
        .iter()
        .map(|c| {
            (
                a.fill(*c).to_bits(),
                a.distance(c.min).signed.to_um().to_bits(),
            )
        })
        .collect();
    for threads in [2, 7, 20] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        let many = pool.install(|| {
            crate::parallel::map_in_order(&cells, |c| {
                (
                    a.fill(*c).to_bits(),
                    a.distance(c.min).signed.to_um().to_bits(),
                )
            })
        });
        assert_eq!(many, one, "{threads} threads");
    }
}

#[test]
fn touching_and_overlapping_elements_are_their_union() {
    // a grating's teeth on a spine: the teeth share edges with the spine
    let teeth = Lattice::rectangular([um(0.5), um(1.0)], [0, 9], [0, 0]).unwrap();
    let tooth = rect(-0.125, 0.0, 0.125, 0.4);
    let mut elements: Vec<Element> = Array::of(&teeth, &tooth).unwrap().elements().to_vec();
    elements.push(Element::new([-1, -1], rect(-0.25, -0.5, 4.75, 0.0)));
    let comb = Array::new(elements).unwrap();
    assert!(!comb.is_separate());
    let exact = 5.0 * 0.5 + 10.0 * 0.25 * 0.4;
    assert!((comb.area() - exact).abs() < 1e-9, "{}", comb.area());
    // the shared edge is inside, a tooth's free edge on the boundary
    assert_eq!(comb.locate(Point::um(0.0, 0.0)), Location::Inside);
    assert_eq!(comb.locate(Point::um(0.0, 0.4)), Location::Boundary);
    assert_eq!(comb.locate(Point::um(0.25, 0.2)), Location::Outside);
    // just inside the spine under a tooth: the distance is to the concave corner where the
    // tooth meets the spine, not to the shared edge
    let d = comb.distance(Point::um(0.0, -0.01));
    assert!(
        (d.signed.to_um() + 0.125f64.hypot(0.01)).abs() < 1e-6,
        "{:?}",
        d
    );
    // outside: the nearest tooth
    let d = comb.distance(Point::um(0.25, 0.3));
    assert!((d.signed.to_um() - 0.125).abs() < 1e-12, "{:?}", d);
    // perimeter: the comb's outline
    let p = comb.perimeter().unwrap().to_um();
    let outline = 2.0 * 5.0 + 2.0 * 0.5 + 10.0 * 2.0 * 0.4;
    assert!((p - outline).abs() < 1e-6, "{p}");
    // overlapping disks: their union's area by inclusion–exclusion
    let pair = Array::new(vec![
        Element::new([0, 0], circle(0.0, 0.0, 1.0)),
        Element::new([1, 0], circle(1.0, 0.0, 1.0)),
    ])
    .unwrap();
    let lens = super::boolean_checks::lens(1.0, 1.0, 1.0);
    assert!((pair.area() - (2.0 * PI - lens)).abs() < 1e-9);
    // polygons: one contour within the tolerance
    let poly = pair.polygons(Tolerance::default());
    assert_eq!(poly.contours.len(), 1);
    assert!(poly.deviation <= Length::nm(1.0));
}

#[test]
fn an_array_of_any_shape_kind_and_its_polygons() {
    let l = Lattice::polar(um(0.0), um(1.0), &[1, 6, 12], 0.0).unwrap();
    let flower = Shape::Polar(
        Polar::new(
            Point::default(),
            &crate::expr::Expr::parse(
                "0.3 um*(1 + 0.2*cos(5*t))",
                crate::expr::Parameters::none().scope(),
            )
            .unwrap(),
            &crate::expr::Parameters::none(),
        )
        .unwrap(),
    );
    let a = Array::of(&l, &flower).unwrap();
    assert_eq!(a.len(), 19);
    assert!(a.is_separate());
    let each = flower.area();
    assert!((a.area() - 19.0 * each).abs() < 1e-12 * a.area());
    let p = a.polygons(Tolerance::default());
    assert_eq!(p.contours.len(), 19);
    assert!(p.deviation <= Length::nm(1.0));
    assert!((p.area() - a.area()).abs() < 19.0 * 1e-3 * 2.0);
    // an array of arrays, and an array inside a reference
    let inner = Shape::Array(a.clone());
    let cell = Cell::new("flowers", vec![inner]).unwrap();
    let r = Reference::new(
        cell,
        vec![Transform::IDENTITY, Transform::translate(um(10.0), um(0.0))],
    )
    .unwrap();
    assert_eq!(r.array().len(), 2);
    assert!((r.area() - 2.0 * a.area()).abs() < 1e-12 * r.area());
    assert!(r.contains(Point::um(10.0, 0.0)));
    assert!(!r.contains(Point::um(5.0, 0.0)));
}

#[test]
fn what_lattices_arrays_and_references_refuse() {
    let says = |e: crate::Error, what: &str| {
        let m = e.to_string();
        assert!(m.contains(what), "{m}");
    };
    says(
        Lattice::rectangular([um(0.0), um(1.0)], [0, 1], [0, 1]).unwrap_err(),
        "positive",
    );
    says(
        Lattice::hexagonal(um(1.0), [2, 1], [0, 1]).unwrap_err(),
        "can't be above",
    );
    says(
        Lattice::polar(um(0.0), um(1.0), &[2, 6], 0.0).unwrap_err(),
        "radius zero",
    );
    says(
        Lattice::polar(um(1.0), um(1.0), &[], 0.0).unwrap_err(),
        "needs a ring",
    );
    says(
        Lattice::hexagonal(um(1.0), [0, 10_000], [0, 10_000]).unwrap_err(),
        "more than",
    );
    let l = Lattice::rectangular([um(1.0), um(1.0)], [0, 1], [0, 0]).unwrap();
    says(l.clone().remove([0, 3]).unwrap_err(), "site (0, 3)");
    says(l.clone().resize([0, 0], -1.0).unwrap_err(), "positive");
    let empty = l.remove([0, 0]).unwrap().remove([1, 0]).unwrap();
    assert!(empty.is_empty());
    says(
        Array::of(&empty, &circle(0.0, 0.0, 0.1)).unwrap_err(),
        "no elements",
    );
    says(
        Cell::new("", vec![circle(0.0, 0.0, 1.0)]).unwrap_err(),
        "name",
    );
    says(Cell::new("c", vec![]).unwrap_err(), "no shapes");
    let c = Cell::new("c", vec![circle(0.0, 0.0, 1.0)]).unwrap();
    says(Reference::new(c, vec![]).unwrap_err(), "nowhere");
}

#[test]
fn a_resized_hole_is_scaled_about_its_site() {
    let l = Lattice::hexagonal(um(0.42), [-2, 2], [-2, 2])
        .unwrap()
        .resize([1, 1], 1.25)
        .unwrap();
    let a = Array::of(&l, &circle(0.0, 0.0, 0.1)).unwrap();
    let Shape::Circle { center, radius } = &a.element([1, 1]).unwrap().shape else {
        panic!("a circle")
    };
    let site = l.site([1, 1]).unwrap().at;
    assert_eq!(*center, site);
    assert!((radius.to_um() - 0.125).abs() < 1e-15);
}

#[test]
#[ignore = "prints timings; cargo test --release --lib array_timing -- --ignored --nocapture"]
fn array_timing() {
    use std::time::Instant;
    let start = Instant::now();
    let a = big();
    let built = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let slab = rect(-1.0, -1.0, 43.0, 37.0)
        .difference(&Shape::Array(a.clone()))
        .unwrap();
    let boolean = start.elapsed().as_secs_f64();
    let n = 1000;
    let (mut inside, mut fill) = (0usize, 0.0);
    let start = Instant::now();
    for p in 0..n {
        for q in 0..n {
            let pt = Point::um(
                -1.0 + 44.0 * f64::from(p) / f64::from(n),
                -1.0 + 38.0 * f64::from(q) / f64::from(n),
            );
            inside += usize::from(slab.contains(pt));
        }
    }
    let sample = start.elapsed().as_secs_f64() / f64::from(n * n);
    let start = Instant::now();
    let m = 300;
    let (dx, dy) = (44.0 / f64::from(m), 38.0 / f64::from(m));
    for p in 0..m {
        for q in 0..m {
            let (x, y) = (-1.0 + dx * f64::from(p), -1.0 + dy * f64::from(q));
            fill += slab.fill(Bounds::new(Point::um(x, y), Point::um(x + dx, y + dy)));
        }
    }
    let cell = start.elapsed().as_secs_f64() / f64::from(m * m);
    println!(
        "10^4 holes: array {built:.3} s, slab less holes {boolean:.3} s, a point {:.0} ns, a cell \
         {:.1} us ({inside} inside, fills {fill:.6} against area {:.6})",
        sample * 1e9,
        cell * 1e6,
        slab.area()
    );
}
