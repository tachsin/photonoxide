//! Paths against closed forms, quadrature and brute force.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use super::checks::Numbers;
use super::path::{Kind, Part};
use super::*;
use crate::expr::{Expr, Parameters};

fn um(v: f64) -> Length {
    Length::um(v)
}

/// A dense polygon of the path's outline, its own polygons within 1e-6 µm.
fn dense(path: &Path) -> Vec<Point> {
    let poly = path.polygons(Tolerance::unsnapped(Length::um(1e-6)).unwrap());
    poly.contours[0].outer.clone()
}

/// The distance from `p` to the polygon `ring`, every edge tried: a brute-force reference.
fn brute(ring: &[Point], p: Point) -> f64 {
    let q = [p.x.to_um(), p.y.to_um()];
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            boundary::Piece::Line([a.x.to_um(), a.y.to_um()], [b.x.to_um(), b.y.to_um()])
                .nearest(q)
                .0
        })
        .fold(f64::INFINITY, f64::min)
}

fn euler(p: f64) -> Path {
    Path::builder(Point::um(0.0, 0.0), 0.0, um(0.5))
        .straight(um(1.0))
        .euler(um(5.0), FRAC_PI_2, p)
        .straight(um(1.0))
        .build()
        .unwrap()
}

#[test]
fn an_arc_is_the_sector_of_the_bend() {
    // the strip bend's geometry: from (0, 0) heading +x, turning left about (0, R)
    let (w, r, angle) = (0.5, 10.0, 90.0f64);
    let sweep = angle.to_radians();
    let path = Path::builder(Point::um(0.0, 0.0), 0.0, um(w))
        .arc(um(r), sweep)
        .build()
        .unwrap();
    let sector = Shape::sector(
        Point::um(0.0, r),
        Length::um(r - w / 2.0),
        Length::um(r + w / 2.0),
        -PI / 2.0,
        sweep,
    )
    .unwrap();
    assert_eq!(path.shape(), sector);
    // its end where the bend puts its port, to the bit
    let end = path.end();
    assert_eq!(
        end.position,
        Point::um(r * sweep.sin(), r - r * sweep.cos())
    );
    assert_eq!(path.length(), um(r * sweep));
    assert!((path.curvature(um(3.0)) - 1.0 / r).abs() < 1e-15);
    // the path itself answers as the sector does
    let mut n = Numbers(11);
    let b = sector.bounds();
    for _ in 0..400 {
        let p = n.point(&b, 0.6);
        assert_eq!(path.contains(p), sector.contains(p), "{p:?}");
        let (a, c) = (path.distance(p), sector.distance(p));
        assert!(
            (a.signed.to_um() - c.signed.to_um()).abs() < 1e-13,
            "{p:?}: {a:?} {c:?}"
        );
        assert!(
            (a.normal[0] - c.normal[0]).abs() < 1e-9 && (a.normal[1] - c.normal[1]).abs() < 1e-9
        );
    }
    let shape = Shape::Path(path.clone());
    assert!((shape.area() - sector.area()).abs() < 1e-13);
    let pb = Region::bounds(&path);
    let sb = sector.bounds();
    assert!(
        (pb.min.x - sb.min.x).to_um().abs() < 1e-13 && (pb.max.y - sb.max.y).to_um().abs() < 1e-13
    );
    // a right turn too
    let right = Path::builder(Point::um(1.0, 2.0), 0.3, um(w))
        .arc(um(r), -1.0)
        .build()
        .unwrap();
    let s = right.shape();
    assert!(matches!(s, Shape::Sector(_)));
    for _ in 0..200 {
        let p = n.point(&Region::bounds(&right), 0.6);
        assert_eq!(right.contains(p), s.contains(p), "{p:?}");
    }
}

#[test]
fn straights_are_rectangles() {
    let p = Path::builder(Point::um(1.0, 2.0), FRAC_PI_2, um(0.4))
        .straight(um(3.0))
        .build()
        .unwrap();
    assert_eq!(
        p.shape(),
        Shape::rect(Point::um(1.0, 3.5), um(0.4), um(3.0)).unwrap()
    );
    let q = Path::builder(Point::um(0.0, 0.0), 0.5, um(0.4))
        .straight(um(3.0))
        .build()
        .unwrap();
    assert!(matches!(q.shape(), Shape::RoundedRect(_)));
    assert!((q.area() - 1.2).abs() < 1e-15);
    assert!((q.perimeter().unwrap().to_um() - 6.8).abs() < 1e-14);
}

#[test]
fn the_clothoid_by_fresnel_against_quadrature() {
    // a clothoid from straight, and one to straight, against Gauss–Legendre of exp(iθ)
    for (k0, k1) in [(0.0, 0.3), (0.0, -0.7), (0.45, 0.0), (-0.2, 0.0)] {
        let len = 4.0;
        let part = Part {
            kind: Kind::Clothoid { k0, k1 },
            o: [0.0, 0.0],
            cos: 1.0,
            sin: 0.0,
            heading: 0.0,
            s0: 0.0,
            len,
            umax: len,
            w: [0.5, 0.5],
            knots: vec![0.0, len],
            arc: vec![0.0, len],
        };
        let a = (k1 - k0) / len;
        for k in 0..=20 {
            let u = len * f64::from(k) / 20.0;
            let p = part.local(u).0;
            let theta = |v: f64| k0 * v + 0.5 * a * v * v;
            let n = 64;
            let (mut x, mut y) = (0.0, 0.0);
            for j in 0..n {
                let (lo, hi) = (
                    u * f64::from(j) / f64::from(n),
                    u * f64::from(j + 1) / f64::from(n),
                );
                x += boundary::gauss(|v| theta(v).cos(), lo, hi);
                y += boundary::gauss(|v| theta(v).sin(), lo, hi);
            }
            assert!(
                (p[0] - x).abs() < 1e-14 && (p[1] - y).abs() < 1e-14,
                "{k0} {k1} {u}: {p:?} {x} {y}"
            );
        }
    }
}

#[test]
fn the_euler_bend_ends_where_the_circular_bend_does() {
    for p in [0.0, 0.05, 0.2, 0.5, 1.0] {
        for (r, angle) in [(5.0, FRAC_PI_2), (3.0, PI), (40.0, PI / 4.0), (5.0, -2.0)] {
            let path = Path::builder(Point::um(0.0, 0.0), 0.0, um(0.5))
                .euler(um(r), angle, p)
                .build()
                .unwrap();
            let e = path.end();
            let (s, c) = angle.abs().sin_cos();
            let want = [r * s, angle.signum() * r * (1.0 - c)];
            let got = [e.position.x.to_um(), e.position.y.to_um()];
            assert!(
                (got[0] - want[0]).abs() < 1e-12 && (got[1] - want[1]).abs() < 1e-12,
                "p = {p}, {angle}: {got:?} against {want:?}"
            );
            let d = e.direction - angle;
            assert!((d - TAU * (d / TAU).round()).abs() < 1e-13);
            // its length and its smallest radius as constructed
            let bend = partial_euler(um(r), angle, p).unwrap();
            assert!((path.length() - bend.length).to_um().abs() < 1e-12);
        }
    }
}

#[test]
fn the_euler_curvature_is_linear_in_arc_length_and_continuous() {
    let path = euler(0.4);
    let bend = partial_euler(um(5.0), FRAC_PI_2, 0.4).unwrap();
    let (lc, rmin) = (bend.clothoid.to_um(), bend.min_radius.to_um());
    let total = path.length().to_um();
    let mut worst_linear = 0.0f64;
    for k in 0..=200 {
        let s = total * f64::from(k) / 200.0;
        let kappa = path.curvature(um(s));
        let along = s - 1.0;
        let want = if !(0.0..=bend.length.to_um()).contains(&along) {
            0.0
        } else if along <= lc {
            along / (lc * rmin)
        } else if along >= bend.length.to_um() - lc {
            (bend.length.to_um() - along) / (lc * rmin)
        } else {
            1.0 / rmin
        };
        worst_linear = worst_linear.max((kappa - want).abs());
    }
    assert!(worst_linear < 1e-12, "{worst_linear}");
    // the parameter A: R L = A² along the clothoid
    assert!((rmin * lc - bend.parameter.to_um().powi(2)).abs() < 1e-12);
}

#[test]
fn a_sine_bend_and_a_bezier_curve() {
    let (l, h) = (10.0, 2.0);
    let path = Path::builder(Point::um(0.0, 0.0), 0.0, um(0.5))
        .sine(um(l), um(h))
        .build()
        .unwrap();
    let e = path.end();
    assert!((e.position.x.to_um() - l).abs() < 1e-14 && (e.position.y.to_um() - h).abs() < 1e-14);
    assert!(e.direction.abs() < 1e-15);
    assert!(path.curvature(Length::ZERO).abs() < 1e-15);
    assert!(path.curvature(path.length()).abs() < 1e-12);
    // its length against a fine trapezoid sum of √(1 + y′²)
    let n = 200_000;
    let mut len = 0.0;
    for k in 0..n {
        let x = l * (f64::from(k) + 0.5) / f64::from(n);
        let yp = h / l * (1.0 - (TAU * x / l).cos());
        len += (1.0 + yp * yp).sqrt() * l / f64::from(n);
    }
    assert!(
        (path.length().to_um() - len).abs() < 1e-9,
        "{} {len}",
        path.length()
    );
    // the point at half its length is its middle, by symmetry
    let mid = path.point(path.length() / 2.0);
    assert!((mid.x.to_um() - l / 2.0).abs() < 1e-12 && (mid.y.to_um() - h / 2.0).abs() < 1e-12);

    // a cubic Bézier S-bend: (0,0), (3,0), (3,2), (6,2)
    let b = Path::builder(Point::um(1.0, 1.0), 0.0, um(0.4))
        .bezier(vec![
            Point::um(3.0, 0.0),
            Point::um(3.0, 2.0),
            Point::um(6.0, 2.0),
        ])
        .build()
        .unwrap();
    let e = b.end();
    assert!(
        (e.position.x.to_um() - 7.0).abs() < 1e-14 && (e.position.y.to_um() - 3.0).abs() < 1e-14
    );
    assert!(e.direction.abs() < 1e-15);
    let speed = |t: f64| {
        let dx = 3.0 * (1.0 - t) * (1.0 - t) * 3.0 + 6.0 * (1.0 - t) * t * 0.0 + 3.0 * t * t * 3.0;
        let dy = 6.0 * (1.0 - t) * t * 2.0;
        dx.hypot(dy)
    };
    let quad: f64 = (0..256)
        .map(|k| boundary::gauss(speed, f64::from(k) / 256.0, f64::from(k + 1) / 256.0))
        .sum();
    assert!((b.length().to_um() - quad).abs() < 1e-13);
}

#[test]
fn a_parametric_curve_is_the_arc_it_draws() {
    let params =
        Parameters::new(&[("r".into(), "4 um".into()), ("phi".into(), "60 deg".into())]).unwrap();
    let x = Expr::parse("r*sin(phi*t)", params.scope()).unwrap();
    let y = Expr::parse("r*(1 - cos(phi*t))", params.scope()).unwrap();
    let curve = Curve::new(&x, &y, &params).unwrap();
    let path = Path::builder(Point::um(0.5, -1.0), 0.2, um(0.6))
        .parametric(curve)
        .build()
        .unwrap();
    let arc = Path::builder(Point::um(0.5, -1.0), 0.2, um(0.6))
        .arc(um(4.0), PI / 3.0)
        .build()
        .unwrap();
    assert!((path.length().to_um() - 4.0 * PI / 3.0).abs() < 1e-13);
    assert!((path.area() - arc.area()).abs() < 1e-12);
    let mut n = Numbers(5);
    for _ in 0..200 {
        let p = n.point(&Region::bounds(&arc), 0.5);
        let (a, b) = (path.distance(p), arc.distance(p));
        assert!((a.signed.to_um() - b.signed.to_um()).abs() < 1e-11, "{p:?}");
    }
    // a curve that reads x is refused
    let bad = Expr::parse("x + t*1um", params.scope()).unwrap();
    assert!(Curve::new(&bad, &y, &params).is_err());
}

#[test]
fn tapers_and_profiles_keep_the_area_and_the_ports() {
    let t = Path::builder(Point::um(0.0, 0.0), 0.0, um(0.5))
        .straight(um(2.0))
        .straight(um(10.0))
        .taper(um(3.0))
        .straight(um(2.0))
        .build()
        .unwrap();
    assert!((t.area() - (1.0 + 17.5 + 6.0)).abs() < 1e-13);
    assert_eq!(t.end().width, um(3.0));
    assert_eq!(t.start().width, um(0.5));
    assert!((t.width(um(7.0)) - um(1.75)).to_um().abs() < 1e-15);
    assert_eq!(t.start().direction, PI);

    // w(s) = 0.5 + 0.1 sin(2π s/3 µm) over 9 µm of an arc
    let params = Parameters::none();
    let w = Expr::parse("0.5um + 0.1um*sin(2*pi*s/(3um))", params.scope()).unwrap();
    let p = Path::builder(Point::um(0.0, 0.0), 0.0, um(0.5))
        .arc(um(9.0 / FRAC_PI_2), FRAC_PI_2)
        .profile(Profile::new(&w, &params).unwrap())
        .build()
        .unwrap();
    assert!((p.area() - 4.5).abs() < 1e-12, "{}", p.area());
    // the polygons keep it
    let poly = p.polygons(Tolerance::unsnapped(um(1e-3)).unwrap());
    assert!((poly.area() - 4.5).abs() < 1e-10, "{}", poly.area());
}

#[test]
fn distances_normals_and_the_inside_against_brute_force() {
    let params = Parameters::none();
    let w = Expr::parse("0.5um + 0.002*s", params.scope()).unwrap();
    let paths = [
        euler(1.0),
        euler(0.2),
        Path::builder(Point::um(-1.0, 0.5), 0.4, um(0.45))
            .sine(um(8.0), um(-2.5))
            .bezier(vec![Point::um(2.0, 0.0), Point::um(3.0, 3.0)])
            .clothoid(um(4.0), [0.1, 0.3])
            .taper(um(0.8))
            .build()
            .unwrap(),
        Path::builder(Point::um(0.0, 0.0), 0.0, um(0.5))
            .euler(um(6.0), PI, 0.3)
            .profile(Profile::new(&w, &params).unwrap())
            .build()
            .unwrap(),
    ];
    for (i, path) in paths.iter().enumerate() {
        let b = Region::bounds(path);
        let ring = dense(path);
        let mut n = Numbers(17 + i as u64);
        let mut worst_d = 0.0f64;
        for _ in 0..300 {
            let p = n.point(&b, 0.7);
            let d = path.distance(p);
            let reference = brute(&ring, p);
            worst_d = worst_d.max((d.signed.to_um().abs() - reference).abs());
            // the sign is the inside test's
            assert_eq!(
                d.signed.to_um() < 0.0,
                path.locate(p) == Location::Inside,
                "{p:?}"
            );
            // stepping back along the normal by the distance lands on the outline
            let q = Point::um(
                p.x.to_um() - d.signed.to_um() * d.normal[0],
                p.y.to_um() - d.signed.to_um() * d.normal[1],
            );
            assert!(path.distance(q).signed.to_um().abs() < 1e-10, "{i}: {p:?}");
        }
        // the brute force's own polygon is within 1e-6 µm
        assert!(worst_d < 2e-6, "{i}: {worst_d}");
    }
}

#[test]
fn polygons_within_their_tolerance_keep_the_area() {
    for path in [euler(1.0), euler(0.3)] {
        for tol in [10.0, 1.0, 0.1] {
            let t = Tolerance::unsnapped(Length::nm(tol)).unwrap();
            let poly = path.polygons(t);
            assert!(poly.deviation <= t.geometric(), "{tol}: {}", poly.deviation);
            let ring = &poly.contours[0].outer;
            let n = ring.len();
            let mut far = 0.0f64;
            for k in 0..n {
                let (a, b) = (ring[k], ring[(k + 1) % n]);
                for s in [0.0, 0.25, 0.5, 0.75] {
                    let q = Point {
                        x: a.x + (b.x - a.x) * s,
                        y: a.y + (b.y - a.y) * s,
                    };
                    far = far.max(path.distance(q).signed.to_um().abs());
                }
            }
            assert!(far <= tol * 1e-3 * 1.0001, "{tol} nm: {far}");
            assert!(
                ((poly.area() - path.area()) / path.area()).abs() < 1e-13,
                "{tol}: {} {}",
                poly.area(),
                path.area()
            );
        }
        // and on the default grid
        let poly = path.polygons(Tolerance::default());
        assert!(poly.deviation <= Length::nm(1.0));
    }
}

#[test]
fn fills_sum_to_the_area_and_match_counting() {
    let path = Path::builder(Point::um(0.13, -0.07), 0.2, um(0.5))
        .straight(um(0.6))
        .euler(um(2.0), FRAC_PI_2, 0.5)
        .sine(um(3.0), um(1.0))
        .build()
        .unwrap();
    let [lo, hi] = Region::bounds(&path).corners();
    let h = 0.137;
    let (x0, y0) = (lo[0] - 0.05, lo[1] - 0.08);
    let nx = ((hi[0] - x0) / h).ceil() as usize + 1;
    let ny = ((hi[1] - y0) / h).ceil() as usize + 1;
    let mut sum = 0.0;
    let mut n = Numbers(3);
    let mut checked = 0;
    for j in 0..ny {
        for i in 0..nx {
            let cell = Bounds::new(
                Point::um(x0 + i as f64 * h, y0 + j as f64 * h),
                Point::um(x0 + (i + 1) as f64 * h, y0 + (j + 1) as f64 * h),
            );
            let f = path.fill(cell);
            sum += f;
            // some cells on the edge against counting a lattice of 200 × 200 points
            if f > 0.0 && f < h * h && checked < 6 && n.next() < 0.2 {
                checked += 1;
                let m = 200;
                let mut count = 0;
                for a in 0..m {
                    for b in 0..m {
                        let q = Point::um(
                            x0 + (i as f64 + (f64::from(a) + 0.5) / f64::from(m)) * h,
                            y0 + (j as f64 + (f64::from(b) + 0.5) / f64::from(m)) * h,
                        );
                        if path.contains(q) {
                            count += 1;
                        }
                    }
                }
                let counted = f64::from(count) / f64::from(m * m) * h * h;
                assert!((counted - f).abs() < 3e-4 * h * h * 10.0, "{counted} {f}");
            }
        }
    }
    assert!(checked > 0);
    assert!(
        ((sum - path.area()) / path.area()).abs() < 1e-12,
        "{sum} {}",
        path.area()
    );
}

#[test]
fn transforms_scale_distances_and_areas() {
    let path = euler(0.6);
    let t = Transform::rotate(0.7, Point::um(0.5, -0.2))
        .then(&Transform::scale(1.7, Point::um(-1.0, 0.3)).unwrap())
        .then(&Transform::mirror(Point::um(0.2, 0.4), 0.3))
        .then(&Transform::translate(um(2.0), um(-1.5)));
    let image = Shape::Path(path.clone()).transformed(&t);
    let s = t.factor();
    assert!(((image.area() - s * s * path.area()) / image.area()).abs() < 1e-13);
    let mut n = Numbers(9);
    for _ in 0..100 {
        let p = n.point(&Region::bounds(&path), 0.5);
        let (d, di) = (path.distance(p), image.distance(t.apply(p)));
        assert!(
            (di.signed.to_um() - s * d.signed.to_um()).abs() < 1e-11,
            "{p:?}"
        );
    }
    // the ends move with it
    if let Shape::Path(q) = &image {
        let a = t.apply(path.end().position);
        let b = q.end().position;
        assert!((a.x - b.x).to_um().abs() < 1e-12 && (a.y - b.y).to_um().abs() < 1e-12);
    } else {
        panic!("a path stays a path");
    }
}

#[test]
fn what_is_refused() {
    let start = Point::um(0.0, 0.0);
    // no segments
    assert!(Path::builder(start, 0.0, um(0.5)).build().is_err());
    // wider than twice the radius
    assert!(
        Path::builder(start, 0.0, um(2.0))
            .arc(um(0.9), 1.0)
            .build()
            .is_err()
    );
    // an Euler bend of p = 1 through 270° doesn't exist; through 263° it does
    assert!(partial_euler(um(5.0), 270f64.to_radians(), 1.0).is_err());
    assert!(partial_euler(um(5.0), 263f64.to_radians(), 1.0).is_ok());
    // a loop that crosses itself
    // (along x to (4, 0), three quarters of a turn about (4, 1) to (3, 1) heading down, then
    // down through the first straight)
    let crossing = Path::builder(start, 0.0, um(0.5))
        .straight(um(4.0))
        .arc(um(1.0), 1.5 * PI)
        .straight(um(3.0))
        .build();
    assert!(crossing.is_err(), "{crossing:?}");
    // a negative length, a zero angle
    assert!(
        Path::builder(start, 0.0, um(0.5))
            .straight(um(-1.0))
            .build()
            .is_err()
    );
    assert!(
        Path::builder(start, 0.0, um(0.5))
            .arc(um(1.0), 0.0)
            .build()
            .is_err()
    );
}

#[test]
fn the_same_bits_on_any_thread() {
    let path = euler(0.3);
    let probe = |path: &Path| -> Vec<u64> {
        let mut n = Numbers(1);
        (0..200)
            .flat_map(|_| {
                let p = n.point(&Region::bounds(path), 0.3);
                let d = path.distance(p);
                let cell = Bounds::new(p, Point::um(p.x.to_um() + 0.1, p.y.to_um() + 0.1));
                [d.signed.to_um().to_bits(), path.fill(cell).to_bits()]
            })
            .collect()
    };
    let here = probe(&path);
    let threads: Vec<Vec<u64>> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..4)
            .map(|_| {
                let p = path.clone();
                s.spawn(move || probe(&euler(0.3)).into_iter().chain(probe(&p)).collect())
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for t in threads {
        assert_eq!(t[..here.len()], here[..]);
        assert_eq!(t[here.len()..], here[..]);
    }
}
