//! What the validation report checks paths on: the Fresnel integrals, the Euler bend's ends and
//! curvature, the outline's perimeter, distances, polygons, fills and their convergence, and the
//! published bends' geometry (Cherchi et al. 2013, Fujisawa et al. 2017, Vogelbacher et al.
//! 2019).

use std::f64::consts::{FRAC_PI_2, PI};

use super::checks::Numbers;
use super::path::euler_half;
use super::*;
use crate::units::Length;
use crate::validation::{Case, Outcome, Tier};

fn um(v: f64) -> Length {
    Length::um(v)
}

/// The paths the checks use: Euler bends of p = 1 and 0.2 between straights, a sine S-bend, a
/// Bézier curve and a clothoid with a taper, and a U-bend with a width profile.
pub(crate) fn paths() -> Vec<Path> {
    let start = Point::um(0.0, 0.0);
    let euler = |p: f64| {
        Path::builder(start, 0.0, um(0.5))
            .straight(um(1.0))
            .euler(um(5.0), FRAC_PI_2, p)
            .straight(um(1.0))
            .build()
    };
    let params = crate::expr::Parameters::none();
    let profile = crate::expr::Expr::parse("0.5um + 0.002*s", params.scope())
        .ok()
        .and_then(|w| Profile::new(&w, &params).ok());
    let mut out = vec![
        euler(1.0),
        euler(0.2),
        Path::builder(Point::um(-1.0, 0.5), 0.4, um(0.45))
            .sine(um(8.0), um(-2.5))
            .bezier(vec![Point::um(2.0, 0.0), Point::um(3.0, 3.0)])
            .clothoid(um(4.0), [0.1, 0.3])
            .taper(um(0.8))
            .build(),
    ];
    if let Some(profile) = profile {
        out.push(
            Path::builder(start, 0.0, um(0.5))
                .euler(um(6.0), PI, 0.3)
                .profile(profile)
                .build(),
        );
    }
    out.into_iter().filter_map(Result::ok).collect()
}

/// The largest error of the Fresnel integrals against A&S Table 7.7 (and DLMF 7.3) at
/// t = 0.5, 1 and 2, and of their limit 1/2 at t = 10⁸.
pub(crate) fn fresnel_error() -> f64 {
    let table = [
        (0.5, 0.492_344_225_871_446_4, 0.064_732_432_859_999_29),
        (1.0, 0.779_893_400_376_822_8, 0.438_259_147_390_354_8),
        (2.0, 0.488_253_406_075_340_75, 0.343_415_678_363_698_24),
    ];
    let mut worst = 0.0f64;
    for (t, c, s) in table {
        let (cc, ss) = super::fresnel::fresnel(t);
        worst = worst.max((cc - c).abs()).max((ss - s).abs());
    }
    // C, S − 1/2 ~ 1/(π t) beyond: within 4e-9 at 10⁸
    let (c, s) = super::fresnel::fresnel(1e8);
    worst
        .max(((c - 0.5).abs() - 3.2e-9).max(0.0))
        .max(((s - 0.5).abs() - 3.2e-9).max(0.0))
}

/// Over partial Euler bends of p = 0, 0.05, 0.2, 0.5 and 1, effective radii 3 to 40 µm and
/// turns of 45°, 90°, 180° and −115°: the largest distance of the end from the circular bend's,
/// µm, and of the end's direction from the turn, radians.
pub(crate) fn euler_end_error() -> f64 {
    let mut worst = 0.0f64;
    for p in [0.0, 0.05, 0.2, 0.5, 1.0] {
        for (r, angle) in [
            (5.0, FRAC_PI_2),
            (3.0, PI),
            (40.0, PI / 4.0),
            (5.0, -115f64.to_radians()),
        ] {
            let Ok(path) = Path::builder(Point::um(0.0, 0.0), 0.0, um(0.5))
                .euler(um(r), angle, p)
                .build()
            else {
                return f64::NAN;
            };
            let e = path.end();
            let (s, c) = angle.abs().sin_cos();
            let want = [r * s, angle.signum() * r * (1.0 - c)];
            worst = worst
                .max((e.position.x.to_um() - want[0]).abs())
                .max((e.position.y.to_um() - want[1]).abs());
            let d = e.direction - angle;
            worst =
                worst.max((d - std::f64::consts::TAU * (d / std::f64::consts::TAU).round()).abs());
        }
    }
    worst
}

/// Along a 90° partial Euler bend of p = 0.4 and effective radius 5 µm between straights, at
/// 401 points: the largest difference of the curvature from the piecewise-linear profile of the
/// construction (0, rising as s/A² over the clothoid, 1/R_min over the arc, falling back), 1/µm.
pub(crate) fn euler_curvature_error() -> f64 {
    let Ok(path) = Path::builder(Point::um(0.0, 0.0), 0.0, um(0.5))
        .straight(um(1.0))
        .euler(um(5.0), FRAC_PI_2, 0.4)
        .straight(um(1.0))
        .build()
    else {
        return f64::NAN;
    };
    let Ok(bend) = partial_euler(um(5.0), FRAC_PI_2, 0.4) else {
        return f64::NAN;
    };
    let (lc, rmin, l) = (
        bend.clothoid.to_um(),
        bend.min_radius.to_um(),
        bend.length.to_um(),
    );
    let a2 = bend.parameter.to_um().powi(2);
    let total = path.length().to_um();
    (0..=400)
        .map(|k| {
            let s = total * f64::from(k) / 400.0;
            let along = s - 1.0;
            let want = if !(0.0..=l).contains(&along) {
                0.0
            } else if along <= lc {
                along / a2
            } else if along >= l - lc {
                (l - along) / a2
            } else {
                1.0 / rmin
            };
            (path.curvature(um(s)) - want).abs()
        })
        .fold(0.0, f64::max)
}

/// A path of constant width w along any centre line of length L has a perimeter of exactly
/// 2L + 2w (its edges are L ∓ (w/2)Δθ long) and an area of wL: over the checks' paths of
/// constant width, the largest relative difference of either, measured on the outline.
pub(crate) fn perimeter_error() -> f64 {
    let mut worst = 0.0f64;
    for path in paths().into_iter().take(2) {
        let (l, w) = (path.length().to_um(), path.start().width.to_um());
        let p = Region::perimeter(&path).map_or(f64::NAN, |p| p.to_um());
        worst = worst.max(((p - 2.0 * l - 2.0 * w) / p).abs());
        let polygon = path.polygons(Tolerance::unsnapped(Length::nm(1.0)).unwrap_or_default());
        worst = worst.max(((polygon.area() - w * l) / (w * l)).abs());
    }
    worst
}

/// Over the checks' paths and 200 points about each: the largest difference, nm, between the
/// distance to the outline and the distance to a polygon of it within 10⁻⁶ µm, every edge
/// tried; and whether every point is on the side the distance's sign says.
pub(crate) fn distance_error() -> f64 {
    let mut worst = 0.0f64;
    for (i, path) in paths().iter().enumerate() {
        let Ok(t) = Tolerance::unsnapped(Length::um(1e-6)) else {
            return f64::NAN;
        };
        let ring: Vec<[f64; 2]> = path.polygons(t).contours[0]
            .outer
            .iter()
            .map(|p| [p.x.to_um(), p.y.to_um()])
            .collect();
        let b = Region::bounds(path);
        let mut n = Numbers(17 + i as u64);
        for _ in 0..200 {
            let p = n.point(&b, 0.7);
            let q = [p.x.to_um(), p.y.to_um()];
            let d = path.distance(p);
            let m = ring.len();
            let brute = (0..m)
                .map(|k| {
                    boundary::Piece::Line(ring[k], ring[(k + 1) % m])
                        .nearest(q)
                        .0
                })
                .fold(f64::INFINITY, f64::min);
            if (d.signed.to_um() < 0.0) != (path.locate(p) == Location::Inside) {
                return f64::NAN;
            }
            worst = worst.max((d.signed.to_um().abs() - brute).abs() * 1e3);
        }
    }
    worst
}

/// Over the checks' paths' polygons at the default tolerance (1 nm on a grid of 1 nm): the
/// largest distance from the outline of their vertices and of points along their edges, nm.
pub(crate) fn polygon_error() -> f64 {
    let mut worst = 0.0f64;
    for path in paths() {
        let p = path.polygons(Tolerance::default());
        for c in &p.contours {
            let ring = &c.outer;
            let n = ring.len();
            for k in 0..n {
                let (a, b) = (ring[k], ring[(k + 1) % n]);
                for s in [0.0, 0.25, 0.5, 0.75] {
                    let q = Point {
                        x: a.x + (b.x - a.x) * s,
                        y: a.y + (b.y - a.y) * s,
                    };
                    worst = worst.max(path.distance(q).signed.to_um().abs() * 1e3);
                }
            }
        }
    }
    worst
}

/// Over the checks' paths: the largest relative difference between the area and the sum of the
/// fills of the cells of a grid of 0.137 µm laid off its corners.
pub(crate) fn fill_error() -> f64 {
    let mut worst = 0.0f64;
    for path in paths() {
        let [lo, hi] = Region::bounds(&path).corners();
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
                sum += path.fill(cell);
            }
        }
        worst = worst.max(((sum - path.area()) / path.area()).abs());
    }
    worst
}

/// The order of the polygons' convergence: a 90° Euler bend's outline (all of it clothoid but
/// the two caps) as polygons for unsnapped tolerances from 10 to 0.01 nm, and the slope of the
/// log of the distance they reach against the log of their vertices, fitted by least squares:
/// −2 when the distance is second order in the chords (shown as its magnitude).
pub(crate) fn polygon_order() -> f64 {
    let Ok(path) = Path::builder(Point::um(0.0, 0.0), 0.0, um(0.5))
        .euler(um(5.0), FRAC_PI_2, 1.0)
        .build()
    else {
        return f64::NAN;
    };
    let points: Vec<(f64, f64)> = [10.0, 3.0, 1.0, 0.3, 0.1, 0.03, 0.01]
        .iter()
        .filter_map(|&t| {
            let tol = Tolerance::unsnapped(Length::nm(t)).ok()?;
            let p = path.polygons(tol);
            let n = p.contours[0].outer.len() as f64;
            Some((n.ln(), p.deviation.to_um().ln()))
        })
        .collect();
    let m = points.len() as f64;
    let (sx, sy) = points
        .iter()
        .fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
    let (mx, my) = (sx / m, sy / m);
    let (sxy, sxx) = points.iter().fold((0.0, 0.0), |(a, b), (x, y)| {
        (a + (x - mx) * (y - my), b + (x - mx) * (x - mx))
    });
    -sxy / sxx
}

/// y + x/tan(α/2) of the unscaled half bend: where it vanishes no bend of that p turns by α.
fn denominator(alpha: f64, p: f64) -> f64 {
    let (x, y, _, _) = euler_half(alpha, p);
    let (s, c) = (0.5 * alpha).sin_cos();
    y + x * c / s
}

/// The root of `f` between `a` and `b` (opposite signs), by bisection to the last bit.
fn bisect(f: impl Fn(f64) -> f64, mut a: f64, mut b: f64) -> f64 {
    let fa = f(a);
    for _ in 0..200 {
        let m = 0.5 * (a + b);
        if m == a || m == b {
            break;
        }
        if (f(m) > 0.0) == (fa > 0.0) {
            a = m;
        } else {
            b = m;
        }
    }
    0.5 * (a + b)
}

/// The Euler bend (p = 1) of Cherchi et al.'s Fig. 3 through `angle`: its effective radius and
/// its length, both over its smallest radius.
pub(crate) fn cherchi(angle: f64) -> (f64, f64) {
    partial_euler(um(1.0), angle, 1.0).map_or((f64::NAN, f64::NAN), |b| {
        let rmin = b.min_radius.to_um();
        (1.0 / rmin, b.length.to_um() / rmin)
    })
}

/// The turn (degrees) beyond which no Euler bend of p = 1 exists (Vogelbacher et al. 2019,
/// §2.2: about 263.3°).
pub(crate) fn singular_angle() -> f64 {
    bisect(|a| denominator(a, 1.0), PI, 1.9 * PI).to_degrees()
}

/// The largest p of a 270° partial Euler bend (Vogelbacher et al. 2019, §2.2: 0.585).
pub(crate) fn largest_p_270() -> f64 {
    bisect(|p| denominator(1.5 * PI, p), 0.1, 1.0)
}

/// The p of a 90° bend whose clothoids are the fraction `ratio` of its length.
pub(crate) fn p_of_ratio(ratio: f64) -> f64 {
    bisect(
        |p| partial_euler(um(4.0), FRAC_PI_2, p).map_or(f64::NAN, |b| b.clothoid_ratio()) - ratio,
        1e-9,
        1.0,
    )
}

/// Fujisawa et al. 2017's 90° bends of effective radius 4 µm with clothoid parameters A = 1.3,
/// 2.4 and 2.68 µm: the fraction of each that is clothoid, from A (their Eq. (3), R L = A²).
pub(crate) fn fujisawa_ratios() -> [f64; 3] {
    [1.3, 2.4, 2.68].map(|a| {
        let p = bisect(
            |p| partial_euler(um(4.0), FRAC_PI_2, p).map_or(f64::NAN, |b| b.parameter.to_um()) - a,
            1e-9,
            1.0,
        );
        partial_euler(um(4.0), FRAC_PI_2, p).map_or(f64::NAN, |b| b.clothoid_ratio())
    })
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

fn error(e: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance,
        error: e,
    }
}

/// The paths' validation cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "geometry/paths/fresnel",
            title: r"The Fresnel integrals $C(t)$ and $S(t)$, by their power series to $t = 1.6$ and Laplace's continued fraction for erfc beyond, at $t$ = 0.5, 1 and 2, and their limit $\frac12$ at $t = 10^8$ (largest error shown)",
            tier: Tier::Analytic,
            source: r"M. Abramowitz, I. A. Stegun, Handbook of Mathematical Functions (1964), Table 7.7 and Eqs. 7.3.11, 7.3.13, 7.3.22 and 7.1.14 (to 17 digits, DLMF 7.3); measured 1.1e-16, and within 1e-14 of Gauss–Legendre quadrature from 0 to 6",
            run: || error(fresnel_error(), 1e-15),
        },
        Case {
            id: "geometry/paths/euler-ends",
            title: r"Partial Euler bends ($p$ = 0, 0.05, 0.2, 0.5, 1; effective radii 3 to 40 µm; 45°, 90°, 180° and −115°) built from clothoids placed by the Fresnel integrals: their ends and directions against the circular bend's (largest difference shown, µm or rad)",
            tier: Tier::Analytic,
            source: r"F. Vogelbacher et al., Opt. Express 27, 31394 (2019), doi:10.1364/OE.27.031394, Eq. (9): the scale $\eta$ puts the ends on the circular bend's, $(R \sin\alpha, R(1 - \cos\alpha))$; measured 7.1e-15",
            run: || error(euler_end_error(), 1e-12),
        },
        Case {
            id: "geometry/paths/euler-curvature",
            title: r"A 90° partial Euler bend ($p$ = 0.4, effective radius 5 µm) between straights: its curvature at 401 points against the construction's, $s/A^2$ along the clothoids and $1/R_\text{min}$ along the arc (largest difference shown, 1/µm)",
            tier: Tier::Analytic,
            source: r"the clothoid's $R L = A^2$ (T. Fujisawa et al., Opt. Express 25, 9150 (2017), doi:10.1364/OE.25.009150, Eq. (3)); measured 2.2e-16",
            run: || error(euler_curvature_error(), 1e-12),
        },
        Case {
            id: "geometry/paths/perimeter",
            title: r"Euler bends of constant width $w$ between straights: the outline's perimeter against $2L + 2w$ and its polygon's area within 1 nm against $wL$ (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"the edges $c \pm \frac w2 n$ are $L \mp \frac w2 \Delta\theta$ long, and the map $(s, v) \mapsto c(s) + v\thinspace n(s)$ has Jacobian $1 - v\kappa$, whose integral over $\lvert v\rvert \le \frac w2$ is $w$; the edges' lengths by Gauss–Legendre between knots; measured 1.0e-15",
            run: || error(perimeter_error(), 1e-12),
        },
        Case {
            id: "geometry/paths/distance",
            title: r"Four paths (Euler bends of $p$ = 1 and 0.2, a sine S-bend, a Bézier curve and a tapered clothoid, a U-bend with a width $w(s)$): the signed distance at 200 points about each against a polygon of the outline within $10^{-6}$ µm, every edge tried, and its sign against the inside test (largest difference shown, nm)",
            tier: Tier::Analytic,
            source: r"the nearest point of each exact piece in closed form, of each curve piece where $(p - E)\cdot E'$ falls through zero, by bracketed root finding; the reference's own error is within $10^{-3}$ nm; measured 7.7e-4 nm",
            run: || error(distance_error(), 2e-3),
        },
        Case {
            id: "geometry/paths/polygons",
            title: r"The same paths' polygons at the default tolerance (1 nm on a grid of 1 nm): the largest distance of their vertices and of points along their edges from the outline (nm, shown)",
            tier: Tier::Analytic,
            source: r"chords halved until the curve strays by at most 3/4 of $1 - 1/\sqrt2$ nm, vertices moved out to keep the area $\int w\thinspace ds$ exactly, arcs flattened as every shape's; snapping moves a vertex by at most $1/\sqrt2$ nm; measured 0.78 nm",
            run: || error(polygon_error(), 1.0),
        },
        Case {
            id: "geometry/paths/fill",
            title: r"The same paths' area inside each cell of a grid of 0.137 µm laid off their corners, summed, against $\int w\thinspace ds$ (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"$\oint x\thinspace dy$ over the outline clipped to each cell (Sutherland and Hodgman, doi:10.1145/360767.360802): exact along straights and arcs, the curves within $10^{-7}$ of the widest width, keeping the area; measured 2.2e-14",
            run: || error(fill_error(), 1e-12),
        },
        Case {
            id: "geometry/paths/convergence",
            title: r"A 90° Euler bend's polygons at unsnapped tolerances from 10 to 0.01 nm: the order of the distance they reach in their number of vertices, the slope of its $\log$ against $\log$ vertices (magnitude shown)",
            tier: Tier::Analytic,
            source: r"a chord of length $h$ strays from a curve of curvature $\kappa$ by $\kappa h^2/8$: second order (the slope shown to two decimals, as the vertices come in whole halvings)",
            run: || outcome((polygon_order() * 100.0).round() / 100.0, 2.0, 0.1),
        },
        Case {
            id: "geometry/paths/cherchi-l-bend",
            title: r"The Euler L-bend (90°, curvature linear in arc length from each end to the middle): its effective radius over its smallest radius (shown) and its length over the smallest radius, $\pi$",
            tier: Tier::Published,
            source: r"M. Cherchi et al., Opt. Express 21, 17814 (2013), doi:10.1364/OE.21.017814, Fig. 3(b): $R_\text{eff} = 1.87\thinspace R_\text{min}$, path length $\pi$; the length here $\pi$ to 4e-16, and the tolerance the paper's last digit",
            run: || {
                let (r, l) = cherchi(FRAC_PI_2);
                let mut o = outcome(r, 1.87, 0.005);
                o.error = o.error.max((l - PI).abs());
                o
            },
        },
        Case {
            id: "geometry/paths/cherchi-u-bend",
            title: r"The Euler U-bend (180°): its effective radius over its smallest radius (shown) and its length over the smallest radius, $2\pi$",
            tier: Tier::Published,
            source: r"Cherchi et al. (2013), Fig. 3(c): $R_\text{eff} = 1.38\thinspace R_\text{min}$, path length $2\pi$; the tolerance the paper's last digit",
            run: || {
                let (r, l) = cherchi(PI);
                let mut o = outcome(r, 1.38, 0.005);
                o.error = o.error.max((l - 2.0 * PI).abs());
                o
            },
        },
        Case {
            id: "geometry/paths/vogelbacher-singular",
            title: r"The turn beyond which no Euler bend of $p = 1$ has the circular bend's ends, where the scale's denominator $y + x/\tan(\alpha/2)$ of their Eq. (9) vanishes (degrees, shown)",
            tier: Tier::Published,
            source: r"F. Vogelbacher et al., Opt. Express 27, 31394 (2019), doi:10.1364/OE.27.031394, §2.2: approximately 263.3°; the tolerance the paper's last digit",
            run: || outcome(singular_angle(), 263.3, 0.05),
        },
        Case {
            id: "geometry/paths/vogelbacher-270",
            title: r"The largest $p$ of a 270° partial Euler bend (shown)",
            tier: Tier::Published,
            source: r"Vogelbacher et al. (2019), §2.2: $p = 0.585$; the tolerance the paper's last digit",
            run: || outcome(largest_p_270(), 0.585, 0.0005),
        },
        Case {
            id: "geometry/paths/vogelbacher-fujisawa",
            title: r"The $p$ of a 90° partial Euler bend whose clothoids are 68 % of its length, Fujisawa et al.'s optimum at $R_\text{eff}$ = 3 µm (shown)",
            tier: Tier::Published,
            source: r"Vogelbacher et al. (2019), §3.2: 68 % corresponds to $p = 0.52$; the tolerance the paper's last digit",
            run: || outcome(p_of_ratio(0.68), 0.52, 0.005),
        },
        Case {
            id: "geometry/paths/fujisawa-ratios",
            title: r"Fujisawa et al.'s fabricated 90° bends of effective radius 4 µm with clothoid parameters $A$ = 1.3, 2.4 and 2.68 µm: the fraction of each that is clothoid, against 14, 58 and 98 % (largest difference shown, percentage points)",
            tier: Tier::Published,
            source: r"T. Fujisawa et al., Opt. Express 25, 9150 (2017), doi:10.1364/OE.25.009150, §3: A = 0, 1.3, 2.4, 2.68 correspond to 0, 14, 58 and 98 %; their Eq. (3), $R L = A^2$, and the bend's ends at $(R_\text{eff}, R_\text{eff})$; the tolerance half their last digit",
            run: || {
                let r = fujisawa_ratios();
                let e = (r[0] * 100.0 - 14.0)
                    .abs()
                    .max((r[1] * 100.0 - 58.0).abs())
                    .max((r[2] * 100.0 - 98.0).abs());
                error(e, 0.5)
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "prints the cases' values; cargo test --release --lib path_checks -- --ignored --nocapture"]
    fn print() {
        for case in cases() {
            let o = (case.run)();
            println!(
                "{}: measured {:.6e} expected {} error {:.3e} tolerance {} {}",
                case.id,
                o.measured,
                o.expected,
                o.error,
                o.tolerance,
                o.passed()
            );
        }
        println!("cherchi L {:?} U {:?}", cherchi(FRAC_PI_2), cherchi(PI));
        println!("fujisawa {:?}", fujisawa_ratios());
    }
}
