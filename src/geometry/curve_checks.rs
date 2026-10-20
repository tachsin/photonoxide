//! What the validation report and the tests check the shapes defined by functions on: a circle
//! written four ways, closed forms of areas, a linear taper against the path it also is,
//! distances, polygons, fills and their convergence, the implicit region's first-order distance,
//! masks, and the published Y-junction of Zhang et al. 2013.

use std::f64::consts::{PI, TAU};

use super::checks::Numbers;
use super::*;
use crate::expr::{Expr, Parameters};
use crate::units::Length;
use crate::validation::{Case, Outcome, Tier};

fn um(v: f64) -> Length {
    Length::um(v)
}

/// `text` parsed with no parameters.
///
/// # Panics
///
/// If it doesn't parse: the checks' expressions do.
pub(crate) fn expr(text: &str) -> Expr {
    Expr::parse(text, Parameters::none().scope()).expect("the checks' expressions parse")
}

/// The circle's centre and radius the checks write four ways.
pub(crate) const CENTRE: [f64; 2] = [0.31, -0.17];
pub(crate) const RADIUS: f64 = 1.1;

/// The circle of [`CENTRE`] and [`RADIUS`] as the primitive, and written as a polar region, a
/// closed curve and an implicit region.
pub(crate) fn circles() -> Vec<(&'static str, Shape)> {
    let none = Parameters::none();
    let [cx, cy] = CENTRE;
    let c = Point::um(cx, cy);
    let mut out = vec![("circle", Shape::circle(c, um(RADIUS)).expect("a circle"))];
    if let Ok(p) = Polar::new(c, &expr("1.1 um"), &none) {
        out.push(("polar", Shape::Polar(p)));
    }
    if let Ok(p) = ClosedCurve::new(
        &expr("0.31 um + 1.1 um*cos(2*pi*t)"),
        &expr("-0.17 um + 1.1 um*sin(2*pi*t)"),
        &none,
    ) {
        out.push(("parametric", Shape::ClosedCurve(p)));
    }
    let window = Bounds::new(Point::um(cx - 1.5, cy - 1.5), Point::um(cx + 1.5, cy + 1.5));
    if let Ok(p) = Implicit::new(
        &expr("(x - 0.31 um)^2 + (y + 0.17 um)^2 - (1.1 um)^2"),
        window,
        &none,
    ) {
        out.push(("implicit", Shape::Implicit(p)));
    }
    out
}

/// Shapes of every kind with their areas in closed form.
pub(crate) fn closed_forms() -> Vec<(&'static str, Shape, f64)> {
    let none = Parameters::none();
    let mut out = Vec::new();
    let window = |a: f64, b: f64| Bounds::new(Point::um(-a, -b), Point::um(a, b));
    // the limaçon r = b + a cos θ: π b² + π a²/2
    let (a, b) = (0.6, 1.0);
    if let Ok(p) = Polar::new(Point::um(0.2, 0.1), &expr("1 um + 0.6 um*cos(t)"), &none) {
        out.push(("limacon", Shape::Polar(p), PI * b * b + PI * a * a / 2.0));
    }
    // a five-petalled flower r = r0 (1 + ε cos 5θ): π r0² (1 + ε²/2)
    if let Ok(p) = Polar::new(Point::um(0.0, 0.0), &expr("2 um*(1 + 0.2*cos(5*t))"), &none) {
        out.push(("flower", Shape::Polar(p), PI * 4.0 * (1.0 + 0.02)));
    }
    let (ea, eb) = (1.3, 0.6);
    if let Ok(p) = ClosedCurve::new(
        &expr("1.3 um*cos(2*pi*t)"),
        &expr("0.6 um*sin(2*pi*t)"),
        &none,
    ) {
        out.push(("parametric ellipse", Shape::ClosedCurve(p), PI * ea * eb));
    }
    // the same ellipse run clockwise
    if let Ok(p) = ClosedCurve::new(
        &expr("1.3 um*cos(2*pi*t)"),
        &expr("-0.6 um*sin(2*pi*t)"),
        &none,
    ) {
        out.push(("clockwise ellipse", Shape::ClosedCurve(p), PI * ea * eb));
    }
    if let Ok(p) = Implicit::new(
        &expr("(x/1.3 um)^2 + (y/0.6 um)^2 - 1"),
        window(1.6, 0.9),
        &none,
    ) {
        out.push(("implicit ellipse", Shape::Implicit(p), PI * ea * eb));
    }
    // the superellipse of exponent 4: 4 a b Γ(1 + 1/4)² / Γ(1 + 2/4)
    let g = primitives::gamma(1.25);
    if let Ok(p) = Implicit::new(
        &expr("(x/1 um)^4 + (y/0.6 um)^4 - 1"),
        window(1.3, 0.9),
        &none,
    ) {
        out.push((
            "implicit superellipse",
            Shape::Implicit(p),
            4.0 * 0.6 * g * g / primitives::gamma(1.5),
        ));
    }
    // an annulus, its hole a loop of its own: π (1² − 0.5²)
    if let Ok(p) = Implicit::new(
        &expr("(x^2 + y^2 - 1 um^2)*(x^2 + y^2 - 0.25 um^2)"),
        window(1.4, 1.4),
        &none,
    ) {
        out.push(("implicit annulus", Shape::Implicit(p), PI * 0.75));
    }
    // two disks of radius 0.5 apart: 2 π 0.5²
    if let Ok(p) = Implicit::new(
        &expr("min((x - 0.8 um)^2 + y^2, (x + 0.8 um)^2 + y^2) - 0.25 um^2"),
        window(1.6, 0.8),
        &none,
    ) {
        out.push(("two disks", Shape::Implicit(p), PI * 0.5));
    }
    // a linear taper from 0.5 to 0.9 µm over 4 µm: the trapezoid's 2.8 µm²
    if let Ok(t) = taper() {
        out.push(("linear taper", Shape::Between(t), 2.8));
    }
    // between −0.2 µm and 0.2 µm + 0.1 x²/µm over [−1, 2] µm: 0.4·3 + 0.1·(8 + 1)/3
    if let Ok(p) = Between::new(
        &expr("-0.2 um"),
        &expr("0.2 um + 0.1*x^2/um"),
        [um(-1.0), um(2.0)],
        &none,
    ) {
        out.push(("parabolic taper", Shape::Between(p), 1.2 + 0.3));
    }
    out
}

/// A linear taper from 0.5 to 0.9 µm wide over x from 0 to 4 µm, between two curves.
///
/// # Errors
///
/// Never: it is a region.
pub(crate) fn taper() -> crate::Result<Between> {
    Between::new(
        &expr("-(0.25 um + 0.05*x)"),
        &expr("0.25 um + 0.05*x"),
        [um(0.0), um(4.0)],
        &Parameters::none(),
    )
}

/// The same taper as a path: a straight of 4 µm tapering from 0.5 to 0.9 µm.
///
/// # Errors
///
/// Never: it is a path.
pub(crate) fn taper_path() -> crate::Result<Path> {
    Path::builder(Point::um(0.0, 0.0), 0.0, um(0.5))
        .straight(um(4.0))
        .taper(um(0.9))
        .build()
}

/// The largest relative difference between the areas of [`closed_forms`]' shapes and their
/// closed forms.
pub(crate) fn closed_form_error() -> f64 {
    let shapes = closed_forms();
    if shapes.len() < 10 {
        return f64::NAN;
    }
    shapes
        .iter()
        .map(|(_, s, a)| ((s.area() - a) / a).abs())
        .fold(0.0, f64::max)
}

/// The circle written as a polar region, a closed curve and an implicit region against the
/// primitive: the largest relative difference of their areas and perimeters, and of their signed
/// distances (over the radius) at 300 points about it, and whether every point is on the same
/// side.
pub(crate) fn circle_error() -> f64 {
    let shapes = circles();
    if shapes.len() < 4 {
        return f64::NAN;
    }
    let circle = &shapes[0].1;
    let b = circle.bounds();
    let perimeter = |s: &Shape| Region::perimeter(s).map_or(f64::NAN, |p| p.to_um());
    let mut worst = 0.0f64;
    for (_, s) in &shapes[1..] {
        worst = worst.max(((s.area() - circle.area()) / circle.area()).abs());
        worst = worst.max(((perimeter(s) - perimeter(circle)) / perimeter(circle)).abs());
        let mut n = Numbers(29);
        for _ in 0..300 {
            let p = n.point(&b, 0.6);
            let (d, e) = (circle.distance(p), s.distance(p));
            if circle.contains(p) != s.contains(p) {
                return f64::NAN;
            }
            worst = worst.max((d.signed.to_um() - e.signed.to_um()).abs() / RADIUS);
            let dn = (d.normal[0] - e.normal[0]).hypot(d.normal[1] - e.normal[1]);
            worst = worst.max(dn);
        }
    }
    worst
}

/// The linear taper between two curves against the same taper as a path: the relative
/// difference of their areas and perimeters, and over 400 points about them, whether each is
/// inside both or neither, and the largest difference of their signed distances, µm.
pub(crate) fn taper_error() -> f64 {
    let (Ok(t), Ok(p)) = (taper(), taper_path()) else {
        return f64::NAN;
    };
    let perimeter = |r: &dyn Region| r.perimeter().map_or(f64::NAN, |p| p.to_um());
    let mut worst = ((t.area() - p.area()) / p.area()).abs();
    worst = worst.max(((perimeter(&t) - perimeter(&p)) / perimeter(&p)).abs());
    let mut n = Numbers(41);
    let b = Region::bounds(&p);
    for _ in 0..400 {
        let q = n.point(&b, 0.5);
        if t.contains(q) != p.contains(q) {
            return f64::NAN;
        }
        worst = worst.max((t.distance(q).signed.to_um() - p.distance(q).signed.to_um()).abs());
    }
    worst
}

/// Every shape of [`closed_forms`] and [`circles`].
fn all() -> Vec<Shape> {
    all_named().into_iter().map(|s| s.1).collect()
}

fn all_named() -> Vec<(&'static str, Shape)> {
    closed_forms()
        .into_iter()
        .map(|s| (s.0, s.1))
        .chain(circles().into_iter().skip(1))
        .collect()
}

/// Over every shape and 150 points about each: the largest difference, nm, between the
/// distance to the boundary and the distance to a polygon of it within 10⁻⁶ µm, every edge
/// tried; NaN if a point isn't on the side its distance's sign says.
pub(crate) fn distance_error() -> f64 {
    let Ok(t) = Tolerance::unsnapped(Length::um(1e-6)) else {
        return f64::NAN;
    };
    let mut worst = 0.0f64;
    for (i, shape) in all().iter().enumerate() {
        let edges: Vec<([f64; 2], [f64; 2])> = shape
            .polygons(t)
            .contours
            .iter()
            .flat_map(|c| std::iter::once(&c.outer).chain(&c.holes))
            .flat_map(|ring| {
                let n = ring.len();
                (0..n).map(move |k| {
                    let (a, b) = (ring[k], ring[(k + 1) % n]);
                    ([a.x.to_um(), a.y.to_um()], [b.x.to_um(), b.y.to_um()])
                })
            })
            .collect();
        let b = shape.bounds();
        let mut n = Numbers(53 + i as u64);
        for _ in 0..150 {
            let p = n.point(&b, 0.6);
            let q = [p.x.to_um(), p.y.to_um()];
            let d = shape.distance(p);
            let brute = edges
                .iter()
                .map(|(a, b)| boundary::Piece::Line(*a, *b).nearest(q).0)
                .fold(f64::INFINITY, f64::min);
            if (d.signed.to_um() < 0.0) != (shape.locate(p) == Location::Inside) {
                return f64::NAN;
            }
            worst = worst.max((d.signed.to_um().abs() - brute).abs() * 1e3);
        }
    }
    worst
}

/// Over every shape's polygons at the default tolerance (1 nm on a grid of 1 nm): the largest
/// distance from the boundary of their vertices and of points along their edges, nm; and the
/// largest relative difference of the unsnapped polygons' areas within 1 nm from the region's.
pub(crate) fn polygon_errors() -> (f64, f64) {
    let (mut distance, mut area) = (0.0f64, 0.0f64);
    for shape in all() {
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
                        distance = distance.max(shape.distance(q).signed.to_um().abs() * 1e3);
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

/// The cells of a grid of `h` laid off `b`'s corner and covering it.
fn grid(b: &Bounds, h: f64) -> Vec<Bounds> {
    let [lo, hi] = b.corners();
    let (x0, y0) = (lo[0] - 0.05, lo[1] - 0.08);
    let nx = ((hi[0] - x0) / h).ceil() as usize + 1;
    let ny = ((hi[1] - y0) / h).ceil() as usize + 1;
    let mut cells = Vec::with_capacity(nx * ny);
    for j in 0..ny {
        for i in 0..nx {
            cells.push(Bounds::new(
                Point::um(x0 + i as f64 * h, y0 + j as f64 * h),
                Point::um(x0 + (i + 1) as f64 * h, y0 + (j + 1) as f64 * h),
            ));
        }
    }
    cells
}

/// Over every shape: the largest relative difference between its area and the sum of its fills
/// over the cells of a grid of 0.137 µm; and over the circle written three ways, the largest
/// difference of each cell's fill from the primitive's exact one, over the cell's area.
pub(crate) fn fill_errors() -> (f64, f64) {
    let h = 0.137;
    let mut sum_error = 0.0f64;
    for shape in all() {
        let sum: f64 = grid(&shape.bounds(), h)
            .iter()
            .map(|c| shape.fill(*c))
            .sum();
        sum_error = sum_error.max(((sum - shape.area()) / shape.area()).abs());
    }
    let shapes = circles();
    let circle = &shapes[0].1;
    let mut cell_error = 0.0f64;
    for (_, s) in &shapes[1..] {
        for c in grid(&circle.bounds(), h) {
            cell_error = cell_error.max((s.fill(c) - circle.fill(c)).abs() / (h * h));
        }
    }
    if shapes.len() < 4 {
        cell_error = f64::NAN;
    }
    (sum_error, cell_error)
}

/// The slope of log(y) against log(x), by least squares.
fn slope(points: &[(f64, f64)]) -> f64 {
    let m = points.len() as f64;
    let (sx, sy) = points
        .iter()
        .fold((0.0, 0.0), |(a, b), (x, y)| (a + x.ln(), b + y.ln()));
    let (mx, my) = (sx / m, sy / m);
    let (sxy, sxx) = points.iter().fold((0.0, 0.0), |(a, b), (x, y)| {
        (
            a + (x.ln() - mx) * (y.ln() - my),
            b + (x.ln() - mx) * (x.ln() - mx),
        )
    });
    sxy / sxx
}

/// The order of the polygons' convergence for one shape of each kind (the limaçon, the
/// parametric ellipse, an implicit superellipse sampled on 24 cells, the parabolic taper): each
/// as polygons at unsnapped tolerances from 1 to 0.001 nm, the slope of the log of the distance
/// they reach against the log of their vertices; the smallest magnitude of the four.
pub(crate) fn polygon_order() -> f64 {
    let mut shapes: Vec<Shape> = closed_forms()
        .into_iter()
        .filter(|s| ["limacon", "parametric ellipse", "parabolic taper"].contains(&s.0))
        .map(|s| s.1)
        .collect();
    // sampled coarsely, so that its chords are long enough to be cut
    if let Ok(s) = Implicit::with_cells(
        &expr("(x/1 um)^4 + (y/0.6 um)^4 - 1"),
        Bounds::new(Point::um(-1.3, -0.9), Point::um(1.3, 0.9)),
        24,
        &Parameters::none(),
    ) {
        shapes.push(Shape::Implicit(s));
    }
    if shapes.len() < 4 {
        return f64::NAN;
    }
    let mut order = f64::INFINITY;
    for shape in &shapes {
        let points: Vec<(f64, f64)> = [1.0, 0.3, 0.1, 0.03, 0.01, 0.003, 0.001]
            .iter()
            .filter_map(|&t| {
                let tol = Tolerance::unsnapped(Length::nm(t)).ok()?;
                let p = shape.polygons(tol);
                let n: usize = p
                    .contours
                    .iter()
                    .map(|c| c.outer.len() + c.holes.iter().map(Vec::len).sum::<usize>())
                    .sum();
                Some((n as f64, p.deviation.to_um()))
            })
            .collect();
        order = order.min(-slope(&points));
    }
    order
}

/// The implicit circle f = (x − c)² + (y − c)² − R²: the order in d of the error of its
/// first-order distance f/|∇f| at distances d from 0.05 down to 0.0016 µm off its boundary
/// (outside, at 17 angles), against the exact distance r − R; the error is d²/(2(R + d)).
pub(crate) fn estimate_order() -> f64 {
    let Some((_, Shape::Implicit(implicit))) = circles().into_iter().find(|s| s.0 == "implicit")
    else {
        return f64::NAN;
    };
    let mut points = Vec::new();
    for k in 0..6 {
        let d = 0.05 / 2f64.powi(k);
        let mut worst = 0.0f64;
        for j in 0..17 {
            let a = TAU * f64::from(j) / 17.0;
            let p = Point::um(
                CENTRE[0] + (RADIUS + d) * a.cos(),
                CENTRE[1] + (RADIUS + d) * a.sin(),
            );
            worst = worst.max((implicit.estimate(p).to_um() - d).abs());
        }
        points.push((d, worst));
    }
    slope(&points)
}

/// A mask of 7 × 5 pixels of 0.2 × 0.15 µm with grey values, a hole and two pixels meeting at a
/// corner, at an angle: the largest relative difference of the sum of its fills over a grid of
/// 0.0913 µm from ∫ρ, of its binarised polygons' area from the pixels with ρ ≥ ½, and of a
/// pixel's fill (the unrotated mask's, the cell the pixel) from its value times its area.
pub(crate) fn mask_error() -> f64 {
    let rows = mask_rows();
    let Ok(m) = Mask::from_rows(&rows, Point::um(-0.3, 0.2), [um(0.2), um(0.15)]) else {
        return f64::NAN;
    };
    let rotated = m.transformed(&Transform::rotate(0.4, Point::um(0.1, 0.1)));
    let mut worst = 0.0f64;
    for mask in [&m, &rotated] {
        let sum: f64 = grid(&mask.bounds(), 0.0913)
            .iter()
            .map(|c| mask.fill(*c))
            .sum();
        worst = worst.max(((sum - mask.area()) / mask.area()).abs());
        let binary = mask.binarised_area();
        worst = worst.max(
            ((mask
                .polygons(Tolerance::unsnapped(um(1e-6)).unwrap_or_default())
                .area()
                - binary)
                / binary)
                .abs(),
        );
    }
    let integral: f64 = rows.iter().flatten().sum::<f64>() * 0.2 * 0.15;
    worst = worst.max(((m.area() - integral) / integral).abs());
    // pixel (2, 1) from the bottom: row 3 of the text, value 0.3
    let cell = Bounds::new(
        Point::um(-0.3 + 0.4, 0.2 + 0.15),
        Point::um(-0.3 + 0.6, 0.2 + 0.3),
    );
    let v = rows[rows.len() - 2][2];
    worst.max(((m.fill(cell) - v * 0.03) / (v * 0.03)).abs())
}

/// The checks' mask, its rows from the top.
pub(crate) fn mask_rows() -> Vec<Vec<f64>> {
    vec![
        vec![0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0, 0.2, 1.0, 0.0, 0.0],
        vec![0.0, 1.0, 1.0, 1.0, 0.6, 0.0, 0.9],
        vec![0.0, 0.1, 0.3, 0.0, 0.0, 0.8, 0.0],
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ]
}

/// Zhang et al. 2013's Y-junction taper: 2 µm long, its widths w1 to w13 at 13 points of equal
/// spacing (their Table 1), µm.
pub(crate) const ZHANG_WIDTHS: [f64; 13] = [
    0.5, 0.5, 0.6, 0.7, 0.9, 1.26, 1.4, 1.4, 1.4, 1.4, 1.31, 1.2, 1.2,
];

/// The not-a-knot cubic spline through (x_k, y_k) at equal spacing h: each piece's
/// coefficients (a, b, c, d) of a + b u + c u² + d u³, u = x − x_k.
pub(crate) fn spline(y: &[f64], h: f64) -> Vec<[f64; 4]> {
    let n = y.len() - 1;
    // the second derivatives M: rows 1..n−1 M_{i−1} + 4 M_i + M_{i+1} = 6 Δ²y/h², and the ends
    // not-a-knot, M_0 − 2 M_1 + M_2 = 0 and M_{n−2} − 2 M_{n−1} + M_n = 0
    let size = n + 1;
    let mut a = vec![vec![0.0; size + 1]; size];
    a[0][0] = 1.0;
    a[0][1] = -2.0;
    a[0][2] = 1.0;
    a[n][n - 2] = 1.0;
    a[n][n - 1] = -2.0;
    a[n][n] = 1.0;
    for i in 1..n {
        a[i][i - 1] = 1.0;
        a[i][i] = 4.0;
        a[i][i + 1] = 1.0;
        a[i][size] = 6.0 * (y[i + 1] - 2.0 * y[i] + y[i - 1]) / (h * h);
    }
    // Gaussian elimination with partial pivoting
    for col in 0..size {
        let pivot = (col..size)
            .max_by(|&r, &s| a[r][col].abs().total_cmp(&a[s][col].abs()))
            .unwrap_or(col);
        a.swap(col, pivot);
        for r in 0..size {
            if r != col {
                let f = a[r][col] / a[col][col];
                let pivot_row = a[col].clone();
                for (x, p) in a[r].iter_mut().zip(&pivot_row).skip(col) {
                    *x -= f * p;
                }
            }
        }
    }
    let m: Vec<f64> = (0..size).map(|i| a[i][size] / a[i][i]).collect();
    (0..n)
        .map(|i| {
            [
                y[i],
                (y[i + 1] - y[i]) / h - h * (2.0 * m[i] + m[i + 1]) / 6.0,
                m[i] / 2.0,
                (m[i + 1] - m[i]) / (6.0 * h),
            ]
        })
        .collect()
}

/// A spline's pieces at knots x_k = k h as one expression of x, µm: `piecewise(x < x_1, piece 0,
/// …, piece n−1)`, scaled by `scale`.
pub(crate) fn spline_text(pieces: &[[f64; 4]], h: f64, scale: f64) -> String {
    let piece = |k: usize, c: &[f64; 4]| {
        let xk = k as f64 * h;
        format!(
            "({:e} um + ({:e})*(x - {xk:e} um) + ({:e})*(x - {xk:e} um)^2/um + ({:e})*(x - {xk:e} um)^3/um^2)",
            scale * c[0],
            scale * c[1],
            scale * c[2],
            scale * c[3]
        )
    };
    let mut parts = Vec::new();
    for (k, c) in pieces.iter().enumerate() {
        if k + 1 < pieces.len() {
            parts.push(format!("x < {:e} um", (k + 1) as f64 * h));
        }
        parts.push(piece(k, c));
    }
    format!("piecewise({})", parts.join(", "))
}

/// Zhang et al.'s taper outline between y = −w(x)/2 and w(x)/2, w the spline through their
/// widths, x from 0 to 2 µm.
///
/// # Errors
///
/// Never: it is a region.
pub(crate) fn zhang_taper() -> crate::Result<Between> {
    let h = 2.0 / 12.0;
    let pieces = spline(&ZHANG_WIDTHS, h);
    let none = Parameters::none();
    let lower = Expr::parse(&spline_text(&pieces, h, -0.5), none.scope())
        .map_err(|e| crate::Error::invalid("the Y-junction's outline", e.to_string()))?;
    let upper = Expr::parse(&spline_text(&pieces, h, 0.5), none.scope())
        .map_err(|e| crate::Error::invalid("the Y-junction's outline", e.to_string()))?;
    Between::new(&lower, &upper, [um(0.0), um(2.0)], &none)
}

/// The taper's width at x, measured through its inside test: twice the y where it stops being
/// inside, by bisection to the last bit.
fn width_at(region: &Between, x: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..200 {
        let m = 0.5 * (lo + hi);
        if m == lo || m == hi {
            break;
        }
        if region.contains(Point::um(x, m)) {
            lo = m;
        } else {
            hi = m;
        }
    }
    let up = 0.5 * (lo + hi);
    let (mut lo, mut hi) = (-1.0, 0.0);
    for _ in 0..200 {
        let m = 0.5 * (lo + hi);
        if m == lo || m == hi {
            break;
        }
        if region.contains(Point::um(x, m)) {
            hi = m;
        } else {
            lo = m;
        }
    }
    up - 0.5 * (lo + hi)
}

/// Zhang et al.'s taper drawn between two curves: the largest difference of its width,
/// measured through its inside test at the 13 points of their Table 1, from the table's, µm;
/// with its ends' widths against the input guide's 0.5 µm and the output's two 0.5 µm arms and
/// 0.2 µm gap, and its area against ∫w, Simpson's rule on each cubic piece (exact).
pub(crate) fn zhang_error() -> f64 {
    let Ok(t) = zhang_taper() else {
        return f64::NAN;
    };
    let h = 2.0 / 12.0;
    let mut worst = 0.0f64;
    for (k, w) in ZHANG_WIDTHS.iter().enumerate() {
        worst = worst.max((width_at(&t, k as f64 * h) - w).abs());
    }
    worst = worst
        .max((width_at(&t, 0.0) - 0.5).abs())
        .max((width_at(&t, 2.0) - (0.5 + 0.2 + 0.5)).abs());
    let pieces = spline(&ZHANG_WIDTHS, h);
    let integral: f64 = pieces
        .iter()
        .map(|c| {
            let w = |u: f64| c[0] + c[1] * u + c[2] * u * u + c[3] * u * u * u;
            h / 6.0 * (w(0.0) + 4.0 * w(0.5 * h) + w(h))
        })
        .sum();
    worst.max((t.area() - integral).abs())
}

fn error(e: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance,
        error: e,
    }
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// The validation cases of the shapes defined by functions and masks.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "geometry/curves/circle",
            title: r"A circle (radius 1.1 µm) written as a polar region $r \le 1.1$ µm, a closed curve $(x(t), y(t))$ and an implicit region $f = \lvert p - c\rvert^2 - R^2 \le 0$, against the circle primitive: areas, perimeters, and signed distances and normals at 300 points about it (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"the primitive's closed forms $\pi R^2$, $2\pi R$, $\lvert p - c\rvert - R$; the definitions' areas $\oint x\thinspace dy$ and lengths by Gauss–Legendre between knots, their distances by root finding of $(p - c)\cdot c'$ on the curves; measured 1.3e-13",
            run: || error(circle_error(), 1e-12),
        },
        Case {
            id: "geometry/curves/closed-forms",
            title: r"Areas against closed forms: a limaçon $r = b + a\cos\theta$ ($\pi b^2 + \pi a^2/2$), a flower $r = r_0(1 + \varepsilon\cos 5\theta)$ ($\pi r_0^2(1 + \varepsilon^2/2)$), an ellipse as a closed curve run both ways and as an implicit region ($\pi a b$), the implicit superellipse $\lvert x/a\rvert^4 + \lvert y/b\rvert^4 \le 1$, an implicit annulus and two disks, a linear and a parabolic taper between two curves (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"$\frac12\int \rho^2 d\theta$, $4ab\thinspace\Gamma(1 + 1/n)^2/\Gamma(1 + 2/n)$ (Γ by C. Lanczos, doi:10.1137/0701008), $\int (y_\text{high} - y_\text{low})\thinspace dx$; measured 3.6e-15",
            run: || error(closed_form_error(), 1e-12),
        },
        Case {
            id: "geometry/curves/taper-as-path",
            title: r"A linear taper (0.5 to 0.9 µm over 4 µm) between two curves against the same taper as a path: areas, perimeters, inside tests and signed distances at 400 points (largest difference shown, µm or relative)",
            tier: Tier::Analytic,
            source: r"the trapezoid: the region between two lines is the path's straight of linear width; measured 5.7e-16",
            run: || error(taper_error(), 1e-12),
        },
        Case {
            id: "geometry/curves/distance",
            title: r"Every kind (polar, closed curves, implicit regions with a hole and two parts, regions between two curves): the signed distance at 150 points about each against a polygon of its boundary within $10^{-6}$ µm, every edge tried, and its sign against the inside test (largest difference shown, nm)",
            tier: Tier::Analytic,
            source: r"the nearest point of each interval of the boundary where $(p - c)\cdot c'$ falls through zero, by bracketed root finding; an implicit boundary's points by root finding along its chords' normals; the reference's own error is within $10^{-3}$ nm; measured 4.7e-4 nm",
            run: || error(distance_error(), 2e-3),
        },
        Case {
            id: "geometry/curves/polygons",
            title: r"The same shapes' polygons at the default tolerance (1 nm on a grid of 1 nm): the largest distance of their vertices and of points along their edges from the boundary (nm, shown), and their areas unsnapped within 1 nm against the regions'",
            tier: Tier::Analytic,
            source: r"chords halved until the curve strays by at most 3/4 of $1 - 1/\sqrt2$ nm, vertices moved out along the normal to keep each loop's area exactly; snapping moves a vertex by at most $1/\sqrt2$ nm; measured 0.75 nm",
            run: || {
                let (d, a) = polygon_errors();
                let mut o = error(d, 1.0);
                if a.is_nan() || a >= 1e-12 {
                    o.error = f64::INFINITY;
                }
                o
            },
        },
        Case {
            id: "geometry/curves/fill",
            title: r"The same shapes' area inside each cell of a grid of 0.137 µm, summed, against the area (relative, shown); and the circle written three ways, each cell's fill against the primitive's exact one, over the cell's area",
            tier: Tier::Analytic,
            source: r"$\oint x\thinspace dy$ over the boundary's polygon within $10^{-7}$ of the larger side, keeping each loop's area, clipped to the cell (Sutherland and Hodgman, doi:10.1145/360767.360802): in error by at most that times the boundary's length in the cell, $2 \times 10^{-7} \times 2.2\thinspace\text{µm} \times 0.137\sqrt2\thinspace\text{µm}/(0.137\thinspace\text{µm})^2$; measured 7.6e-15 and 2.8e-9",
            run: || {
                let (sum, cell) = fill_errors();
                let mut o = error(sum, 1e-12);
                if cell.is_nan() || cell >= 5e-6 {
                    o.error = f64::INFINITY;
                }
                o
            },
        },
        Case {
            id: "geometry/curves/convergence",
            title: r"A limaçon, an ellipse as a closed curve, an implicit superellipse and a parabolic taper between two curves, as polygons at unsnapped tolerances from 1 to 0.001 nm: the order of the distance they reach in their number of vertices, the slope of its $\log$ against $\log$ vertices (the smallest magnitude of the four, shown)",
            tier: Tier::Analytic,
            source: r"a chord of length $h$ strays from a curve of curvature $\kappa$ by $\kappa h^2/8$: second order (the slope shown to two decimals)",
            run: || outcome((polygon_order() * 100.0).round() / 100.0, 2.0, 0.1),
        },
        Case {
            id: "geometry/curves/implicit-estimate",
            title: r"An implicit circle's first-order distance $f/\lvert\nabla f\rvert$ at distances $d$ from 50 to 1.6 nm off its boundary: the order in $d$ of its error against the exact distance (shown to two decimals)",
            tier: Tier::Analytic,
            source: r"for $f = r^2 - R^2$, $f/\lvert\nabla f\rvert = d(2R + d)/(2(R + d))$, in error by $d^2/(2(R + d))$: second order; the exact distance, by root finding on the curve, has none",
            run: || outcome((estimate_order() * 100.0).round() / 100.0, 2.0, 0.05),
        },
        Case {
            id: "geometry/curves/mask",
            title: r"A mask of 7 × 5 pixels with grey values, a hole and two pixels meeting at a corner, as drawn and rotated: its fills over a grid of 0.0913 µm summed against $\int\rho$, its binarised polygons' area against its pixels with $\rho \ge \frac12$, and a pixel's fill against its value times its area (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"$\rho$ constant over each pixel: a cell's fill is each pixel's value times the area it shares with the cell (a rotated pixel clipped to the cell); the binarised design's outline is its pixels' edges; measured 4.7e-16",
            run: || error(mask_error(), 1e-12),
        },
        Case {
            id: "geometry/curves/zhang-y-junction",
            title: r"Zhang et al.'s Y-junction taper (2 µm long, its widths $w_1$ to $w_{13}$ at 13 points of equal spacing) drawn between $y = \mp w(x)/2$, $w$ a cubic spline through the widths: its width at the 13 points through its inside test against their Table 1, its ends against the input guide (0.5 µm) and the two 0.5 µm arms 0.2 µm apart (1.2 µm), and its area against $\int w\thinspace dx$ (largest difference shown, µm or µm²)",
            tier: Tier::Published,
            source: r"Y. Zhang et al., Opt. Express 21, 1310 (2013), doi:10.1364/OE.21.001310, Table 1 and Fig. 1(a): the taper is defined by spline interpolation of $w_1$ to $w_{13}$ (the spline's ends here not-a-knot, which the paper doesn't state; the widths at the points don't depend on it); measured 2.2e-15",
            run: || error(zhang_error(), 1e-12),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "prints the cases' values; cargo test --release --lib curve_checks -- --ignored --nocapture"]
    fn print() {
        for case in cases() {
            let start = std::time::Instant::now();
            let o = (case.run)();
            println!(
                "{}: measured {:.6e} expected {} error {:.3e} tolerance {} {} ({:.2} s)",
                case.id,
                o.measured,
                o.expected,
                o.error,
                o.tolerance,
                o.passed(),
                start.elapsed().as_secs_f64()
            );
        }
        println!("polygons {:?}", polygon_errors());
        println!("fills {:?}", fill_errors());
        for (name, s, a) in closed_forms() {
            println!("{name}: {:.3e}", ((s.area() - a) / a).abs());
        }
    }
}
