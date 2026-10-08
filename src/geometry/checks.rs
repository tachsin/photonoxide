//! What the validation report and the tests check the geometry kernel on: one shape of every
//! kind, and the largest errors of their areas, polygons, fills and transforms.

use std::f64::consts::{PI, TAU};

use super::*;
use crate::units::Length;
use crate::validation::{Case, Outcome, Tier};

pub(crate) fn um(v: f64) -> Length {
    Length::um(v)
}

/// A fixed sequence of numbers in [0, 1): the same on every run.
pub(crate) struct Numbers(pub(crate) u64);

impl Numbers {
    pub(crate) fn next(&mut self) -> f64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
    }

    pub(crate) fn point(&mut self, b: &Bounds, margin: f64) -> Point {
        let (lo, hi) = (b.corners()[0], b.corners()[1]);
        Point::um(
            lo[0] - margin + (hi[0] - lo[0] + 2.0 * margin) * self.next(),
            lo[1] - margin + (hi[1] - lo[1] + 2.0 * margin) * self.next(),
        )
    }
}

/// One of every kind, at angles and off the origin.
///
/// # Panics
///
/// Never: the shapes are valid.
pub(crate) fn shapes() -> Vec<(&'static str, Shape)> {
    let c = Point::um(0.31, -0.17);
    vec![
        ("rect", Shape::rect(c, um(1.3), um(0.45)).unwrap()),
        ("circle", Shape::circle(c, um(0.8)).unwrap()),
        ("ring", Shape::ring(c, um(1.1), um(0.4)).unwrap()),
        (
            "polygon",
            Shape::Polygon(
                Polygon::new(vec![
                    Point::um(0.0, 0.0),
                    Point::um(2.0, 0.0),
                    Point::um(2.0, 1.0),
                    Point::um(1.0, 1.0),
                    Point::um(1.0, 2.0),
                    Point::um(0.0, 2.0),
                ])
                .unwrap(),
            ),
        ),
        (
            "ellipse",
            Shape::ellipse(c, [um(1.2), um(0.5)], 0.4).unwrap(),
        ),
        (
            "sector",
            Shape::sector(c, um(0.4), um(1.0), 0.3, 2.0).unwrap(),
        ),
        (
            "pie",
            Shape::sector(c, Length::ZERO, um(1.0), -0.5, 4.0).unwrap(),
        ),
        (
            "hexagon",
            Shape::regular_polygon(c, um(0.9), 6, 0.2).unwrap(),
        ),
        (
            "superellipse",
            Shape::superellipse(c, [um(1.0), um(0.6)], 4.0, 0.25).unwrap(),
        ),
        (
            "rounded",
            Shape::rounded_rect(c, um(1.6), um(0.7), um(0.2), 0.6).unwrap(),
        ),
    ]
}

/// The largest relative difference between a primitive's area and perimeter in closed form and
/// what its boundary gives independently: Green's theorem and the boundary's length for every
/// shape bounded by segments and circles, quadrature of the ellipse's speed for its perimeter
/// (the closed form by the arithmetic-geometric mean), and the superellipses of exponent 2 and
/// 1 against the ellipse and the rhombus.
pub(crate) fn area_error() -> f64 {
    let mut worst = 0.0f64;
    let mut rel = |a: f64, b: f64| worst = worst.max(((a - b) / b).abs());
    for (_, shape) in shapes() {
        let loops = match &shape {
            Shape::Sector(s) => s.loops(),
            Shape::RoundedRect(r) => r.loops(),
            Shape::Ellipse(e) => e.loops(),
            Shape::RegularPolygon(r) => r.polygon().loops(),
            Shape::Superellipse(_) => continue,
            other => other.loops(),
        };
        rel(boundary::area_of(&loops), shape.area());
        if !matches!(shape, Shape::Ellipse(_)) {
            let p = Region::perimeter(&shape).map_or(f64::NAN, |p| p.to_um());
            rel(boundary::length(&loops), p);
        }
    }
    let (a, b) = (1.2, 0.5);
    let quad = boundary::integrate(|t| (a * t.sin()).hypot(b * t.cos()), 0.0, TAU);
    let origin = Point::um(0.0, 0.0);
    let perimeter = |s: &Shape| Region::perimeter(s).map_or(f64::NAN, |p| p.to_um());
    if let (Ok(e), Ok(s2), Ok(s1)) = (
        Shape::ellipse(origin, [um(a), um(b)], 0.4),
        Shape::superellipse(origin, [um(a), um(b)], 2.0, 0.0),
        Shape::superellipse(origin, [um(a), um(b)], 1.0, 0.3),
    ) {
        rel(e.area(), PI * a * b);
        rel(perimeter(&e), quad);
        rel(s2.area(), PI * a * b);
        rel(perimeter(&s2), quad);
        rel(s1.area(), 2.0 * a * b);
        rel(perimeter(&s1), 4.0 * a.hypot(b));
    } else {
        worst = f64::NAN;
    }
    worst
}

/// Over every shape's polygons within 1 nm on a grid of 1 nm (the default tolerance): the
/// largest distance from the true boundary of the polygons' vertices and of points along their
/// edges, µm, and the largest relative difference of their unsnapped areas from the region's.
pub(crate) fn polygon_errors() -> (f64, f64) {
    let (mut distance, mut area) = (0.0f64, 0.0f64);
    for (_, shape) in shapes() {
        let p = shape.polygons(Tolerance::default());
        for c in &p.contours {
            for ring in std::iter::once(&c.outer).chain(&c.holes) {
                let n = ring.len();
                for k in 0..n {
                    let (a, b) = (ring[k], ring[(k + 1) % n]);
                    for s in [0.0, 0.25, 0.5, 0.75] {
                        let q = Point {
                            x: a.x + (b.x - a.x) * s,
                            y: a.y + (b.y - a.y) * s,
                        };
                        distance = distance.max(shape.distance(q).signed.to_um().abs());
                    }
                }
            }
        }
        if let Ok(t) = Tolerance::unsnapped(Length::nm(1.0)) {
            let exact = shape.area();
            area = area.max(((shape.polygons(t).area() - exact) / exact).abs());
        }
    }
    (distance, area)
}

/// The largest relative difference between a shape's area and the sum of its fills over the
/// cells of a grid of 0.137 µm, off its corners, that covers it.
pub(crate) fn fill_error() -> f64 {
    let mut worst = 0.0f64;
    for (_, shape) in shapes() {
        let [lo, hi] = shape.bounds().corners();
        let h = 0.137;
        let (x0, y0) = (lo[0] - 0.05, lo[1] - 0.08);
        let nx = ((hi[0] - x0) / h).ceil() as usize + 1;
        let ny = ((hi[1] - y0) / h).ceil() as usize + 1;
        let mut sum = 0.0;
        for j in 0..ny {
            for i in 0..nx {
                let cell = Bounds::new(
                    Point::um(x0 + i as f64 * h, y0 + j as f64 * h),
                    Point::um(x0 + (i + 1) as f64 * h, y0 + (j + 1) as f64 * h),
                );
                sum += shape.fill(cell);
            }
        }
        worst = worst.max(((sum - shape.area()) / shape.area()).abs());
    }
    worst
}

/// A rotation, a scaling by 1.7, a mirror and a translation, composed: over every shape and
/// 50 points about each, the largest difference between the image's signed distance from the
/// point's image and 1.7 times the shape's from the point, µm, and the largest relative
/// difference of the image's area from 1.7² times the shape's.
pub(crate) fn transform_errors() -> (f64, f64) {
    let Ok(scale) = Transform::scale(1.7, Point::um(-1.0, 0.3)) else {
        return (f64::NAN, f64::NAN);
    };
    let t = Transform::rotate(0.7, Point::um(0.5, -0.2))
        .then(&scale)
        .then(&Transform::mirror(Point::um(0.2, 0.4), 0.3))
        .then(&Transform::translate(um(2.0), um(-1.5)));
    let s = t.factor();
    let (mut distance, mut area) = (0.0f64, 0.0f64);
    for (_, shape) in shapes() {
        let image = shape.transformed(&t);
        area = area.max(((image.area() - s * s * shape.area()) / image.area()).abs());
        let mut n = Numbers(3);
        for _ in 0..50 {
            let p = n.point(&shape.bounds(), 0.5);
            let (d, di) = (shape.distance(p), image.distance(t.apply(p)));
            distance = distance.max((di.signed.to_um() - s * d.signed.to_um()).abs());
        }
    }
    (distance, area)
}

/// The geometry kernel's validation cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "geometry/areas",
            title: r"Every primitive's area and perimeter in closed form (rectangle, circle, ring, polygon, ellipse at an angle, annular sector, pie slice, hexagon, rectangle with rounded corners; the superellipses of exponent 2 and 1) against Green's theorem $A = \oint x\thinspace dy$ on its boundary and the boundary's length (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"the closed forms: $\pi a b$, $\frac12 \beta (r_1^2 - r_0^2)$, $wh - (4 - \pi) r^2$, $\frac n2 R^2 \sin(2\pi/n)$, $4ab\thinspace\Gamma(1 + 1/n)^2/\Gamma(1 + 2/n)$ (Γ by C. Lanczos, J. SIAM Numer. Anal. B 1, 86 (1964), doi:10.1137/0701008), the ellipse's perimeter by the arithmetic-geometric mean (S. Adlaj, Notices AMS 59, 1094 (2012), doi:10.1090/noti879) against quadrature of its speed; measured 7.9e-14",
            run: || {
                let e = area_error();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-12,
                    error: e,
                }
            },
        },
        Case {
            id: "geometry/polygons",
            title: r"Every shape's polygons at the default tolerance (1 nm, snapped to a grid of 1 nm): the largest distance of their vertices and of points along their edges from the true boundary, nm (shown)",
            tier: Tier::Analytic,
            source: r"each arc flattened to within $1 - 1/\sqrt2$ nm (its chords' sagitta, the vertices placed off the arc so that the polygon keeps the area: 6.4e-16 of it, unsnapped), and snapping moves a vertex by at most $1/\sqrt2$ nm; measured 0.94 nm",
            run: || {
                let d = polygon_errors().0 * 1e3;
                Outcome {
                    measured: d,
                    expected: 0.0,
                    tolerance: 1.0,
                    error: d,
                }
            },
        },
        Case {
            id: "geometry/fill",
            title: r"Every shape's area inside each cell of a grid of 0.137 µm laid off its corners, summed over the grid, against its area (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"$\oint x\thinspace dy$ over the boundary clipped to each cell, a side at a time (I. E. Sutherland, G. W. Hodgman, Commun. ACM 17, 32 (1974), doi:10.1145/360767.360802), in closed form for segments and arcs of circles and ellipses; the superellipse's polygon within $10^{-7}$ of its larger semi-axis, which keeps its area; measured 2.2e-15",
            run: || {
                let e = fill_error();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-12,
                    error: e,
                }
            },
        },
        Case {
            id: "geometry/transforms",
            title: r"A rotation, a scaling by 1.7, a mirror and a translation composed, applied to every shape: the image's signed distance from each of 50 points' images against 1.7 times the shape's from the point (largest difference shown, µm)",
            tier: Tier::Analytic,
            source: r"a similarity maps each primitive to one of its kind, exactly, and scales distances by its factor; the areas by its square to 3.1e-16; measured 4.7e-14 µm",
            run: || {
                let d = transform_errors().0;
                Outcome {
                    measured: d,
                    expected: 0.0,
                    tolerance: 1e-12,
                    error: d,
                }
            },
        },
    ]
}
