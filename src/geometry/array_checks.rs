//! What the validation report and the tests check arrays, lattices and references on: element
//! counts and positions against the lattices' closed forms, hole areas and fill fractions
//! against N π r² and the unit cell, elements made from their index, defects, references
//! against the transformed shapes to the bit, a slab less a hole lattice by inclusion–exclusion,
//! the polygons' convergence, and the L3 cavity of Akahane et al. 2003, its geometry.

use std::f64::consts::PI;

use super::boolean_checks::{circle, deviation, rect};
use super::*;
use crate::material;
use crate::stack::{Layer, LayerStack, Structure};
use crate::units::Length;
use crate::validation::{Case, Outcome, Tier};

fn um(v: f64) -> Length {
    Length::um(v)
}

/// Akahane et al.'s lattice constant, µm: a = 0.42 µm.
pub(crate) const AKAHANE_A: f64 = 0.42;
/// Their air holes' radius over a: R = 0.29 a.
pub(crate) const AKAHANE_R: f64 = 0.29;
/// Their slab's thickness over a: T = 0.6 a.
pub(crate) const AKAHANE_T: f64 = 0.6;
/// The shift of the holes at both ends of the cavity, over a: 0.15 a.
pub(crate) const AKAHANE_SHIFT: f64 = 0.15;

/// The relative difference of `got` from `want`.
fn rel(got: f64, want: f64) -> f64 {
    ((got - want) / want).abs()
}

/// The hexagonal lattice of pitch `a` with an L3 cavity on row 0: sites (−1, 0), (0, 0) and
/// (1, 0) removed, and the holes at its ends, (∓2, 0), moved out along the row by `shift`
/// (µm); i from −`n[0]` to `n[0]`, j from −`n[1]` to `n[1]`.
pub(crate) fn l3_lattice(a: f64, shift: f64, n: [i64; 2]) -> Result<Lattice> {
    Lattice::hexagonal(um(a), [-n[0], n[0]], [-n[1], n[1]])?
        .remove([-1, 0])?
        .remove([0, 0])?
        .remove([1, 0])?
        .shift([-2, 0], [um(-shift), um(0.0)])?
        .shift([2, 0], [um(shift), um(0.0)])
}

/// The lattices' sites against their closed forms: the counts, every position (relative to
/// the pitch), the six nearest neighbours of every interior hexagonal site a apart and its
/// next six √3 a apart, and the polar rings' radii and angles. The largest difference, relative
/// to the pitch; infinite when a count is wrong.
pub(crate) fn lattice_error() -> f64 {
    let mut worst = 0.0f64;
    let (ax, ay) = (0.37, 0.53);
    let Ok(r) = Lattice::rectangular([um(ax), um(ay)], [-3, 5], [2, 6]) else {
        return f64::INFINITY;
    };
    if r.len() != 9 * 5 {
        return f64::INFINITY;
    }
    for s in r.sites() {
        let [i, j] = s.index;
        worst = worst
            .max((s.at.x.to_um() - i as f64 * ax).abs() / ax)
            .max((s.at.y.to_um() - j as f64 * ay).abs() / ay);
    }
    let a = 0.42;
    let Ok(h) = Lattice::hexagonal(um(a), [-6, 6], [-5, 5]) else {
        return f64::INFINITY;
    };
    if h.len() != 13 * 11 {
        return f64::INFINITY;
    }
    let row = a * 3f64.sqrt() / 2.0;
    for s in h.sites() {
        let [i, j] = s.index;
        let x = (i as f64 + if j % 2 == 0 { 0.0 } else { 0.5 }) * a;
        worst = worst
            .max((s.at.x.to_um() - x).abs() / a)
            .max((s.at.y.to_um() - j as f64 * row).abs() / a);
    }
    // the neighbours of the interior sites: six at a, six at √3 a, none nearer
    for s in h.sites() {
        let [i, j] = s.index;
        if i.abs() > 4 || j.abs() > 3 {
            continue;
        }
        let mut d: Vec<f64> = h
            .sites()
            .iter()
            .filter(|t| t.index != s.index)
            .map(|t| (t.at.x - s.at.x).to_um().hypot((t.at.y - s.at.y).to_um()))
            .collect();
        d.sort_by(f64::total_cmp);
        for k in 0..6 {
            worst = worst.max(rel(d[k], a));
            worst = worst.max(rel(d[6 + k], 3f64.sqrt() * a));
        }
        if d[12] < 2.0 * a * (1.0 - 1e-12) {
            return f64::INFINITY;
        }
    }
    let (r0, dr, start) = (0.0, 0.5, 0.3);
    let counts = [1, 6, 12, 18, 24];
    let Ok(p) = Lattice::polar(um(r0), um(dr), &counts, start) else {
        return f64::INFINITY;
    };
    if p.len() != counts.iter().sum::<usize>() {
        return f64::INFINITY;
    }
    for s in p.sites() {
        let [i, j] = s.index;
        let r = r0 + j as f64 * dr;
        let theta = start + 2.0 * PI * i as f64 / counts[j as usize] as f64;
        worst = worst
            .max((s.at.x.to_um() - r * theta.cos()).abs() / dr)
            .max((s.at.y.to_um() - r * theta.sin()).abs() / dr);
    }
    worst
}

/// A hole lattice's area and fill fraction: N π r² for the holes of a hexagonal and a
/// rectangular lattice; their fills over a grid summed back to that; and the fill of a unit
/// cell, 2 π r² / (√3 a²) for the hexagonal lattice (a rectangle a by √3 a holds two holes'
/// worth, in pieces) and π r² / (aₓ a_y) for the rectangular one, at several places. The
/// largest relative difference.
pub(crate) fn area_error() -> f64 {
    let (a, r) = (AKAHANE_A, AKAHANE_R * AKAHANE_A);
    let hole = circle(0.0, 0.0, r);
    let Ok(h) = Lattice::hexagonal(um(a), [-10, 10], [-8, 8]).and_then(|l| Array::of(&l, &hole))
    else {
        return f64::INFINITY;
    };
    let n = (21 * 17) as f64;
    let mut worst = rel(h.area(), n * PI * r * r);
    // a grid of cells over the whole lattice, its fills summed
    let b = h.bounds();
    let (x0, y0, x1, y1) = (
        b.min.x.to_um() - 0.05,
        b.min.y.to_um() - 0.05,
        b.max.x.to_um() + 0.05,
        b.max.y.to_um() + 0.05,
    );
    let cells = 97;
    let (dx, dy) = ((x1 - x0) / cells as f64, (y1 - y0) / cells as f64);
    let mut sum = 0.0;
    for p in 0..cells {
        for q in 0..cells {
            let x = x0 + p as f64 * dx;
            let y = y0 + q as f64 * dy;
            sum += h.fill(Bounds::new(Point::um(x, y), Point::um(x + dx, y + dy)));
        }
    }
    worst = worst.max(rel(sum, h.area()));
    // unit cells, a by √3 a, at places that cut holes
    let fraction = 2.0 * PI * r * r / (3f64.sqrt() * a * a);
    for (x, y) in [(0.0, 0.0), (0.137, 0.211), (-1.3, 0.9), (0.5 * a, -0.31)] {
        let cell = Bounds::new(Point::um(x, y), Point::um(x + a, y + 3f64.sqrt() * a));
        worst = worst.max(rel(h.fill(cell) / cell.area(), fraction));
    }
    let (ax, ay) = (0.5, 0.4);
    let Ok(rl) = Lattice::rectangular([um(ax), um(ay)], [-12, 12], [-9, 9])
        .and_then(|l| Array::of(&l, &circle(0.0, 0.0, 0.15)))
    else {
        return f64::INFINITY;
    };
    worst = worst.max(rel(rl.area(), (25 * 19) as f64 * PI * 0.15 * 0.15));
    for (x, y) in [(0.0, 0.0), (0.13, -0.07), (-2.21, 1.3)] {
        let cell = Bounds::new(Point::um(x, y), Point::um(x + ax, y + ay));
        worst = worst.max(rel(
            rl.fill(cell) / cell.area(),
            PI * 0.15 * 0.15 / (ax * ay),
        ));
    }
    worst
}

/// Elements made from their index and place: an apodized grating's teeth (width and place
/// from i) and a metalens's pillars (radius from the site's distance to the centre, through a
/// hyperbolic phase), each an expression of `i`, `j`, `x`, `y` evaluated at every element, its
/// shape against the expression's value. The largest difference, µm.
pub(crate) fn index_error() -> f64 {
    use crate::expr::{Expr, Parameters};
    let p = Parameters::none();
    let width = Expr::parse("0.2 um + 0.01 um*i", p.scope()).expect("an expression");
    let offset = Expr::parse("2 nm*i^2", p.scope()).expect("an expression");
    let Ok(grating) = Lattice::rectangular([um(0.5), um(1.0)], [0, 20], [0, 0]).and_then(|l| {
        Array::generate(&l, |s| {
            let mut v = p.values().to_vec();
            v[5] = s.index[0] as f64;
            v[6] = s.index[1] as f64;
            let (w, dx) = (width.eval(&v), offset.eval(&v));
            Shape::rect(Point::um(dx, 0.0), um(w), um(0.6)).map(Some)
        })
    }) else {
        return f64::INFINITY;
    };
    let mut worst = 0.0f64;
    for e in grating.elements() {
        let i = e.index[0] as f64;
        let b = e.shape.bounds();
        let (w, c) = (
            (b.max.x - b.min.x).to_um(),
            0.5 * (b.max.x + b.min.x).to_um(),
        );
        worst = worst
            .max((w - (0.2 + 0.01 * i)).abs())
            .max((c - (0.5 * i + 0.002 * i * i)).abs());
    }
    // a metalens: pillars on a hexagonal lattice, r = r0 + Δr φ/2π, φ the hyperbolic phase
    // mod 2π at the pillar's place
    let lens = Expr::parse(
        "60 nm + 50 nm*(2*pi*(sqrt(x^2 + y^2 + 10 um^2) - 10 um)/1.55 um - 2*pi*floor((sqrt(x^2 + \
         y^2 + 10 um^2) - 10 um)/1.55 um))/(2*pi)",
        p.scope(),
    )
    .expect("an expression");
    let Ok(pillars) = Lattice::hexagonal(um(0.4), [-12, 12], [-12, 12]).and_then(|l| {
        Array::generate(&l, |s| {
            let mut v = p.values().to_vec();
            v[0] = s.at.x.to_um();
            v[1] = s.at.y.to_um();
            Shape::circle(Point::default(), um(lens.eval(&v))).map(Some)
        })
    }) else {
        return f64::INFINITY;
    };
    for e in pillars.elements() {
        let Shape::Circle { center, radius } = &e.shape else {
            return f64::INFINITY;
        };
        let (x, y) = (center.x.to_um(), center.y.to_um());
        let l = (x * x + y * y + 10.0).sqrt() - 10.0;
        let phase = l / 1.55 - (l / 1.55).floor();
        worst = worst.max((radius.to_um() - (0.06 + 0.05 * phase)).abs());
    }
    worst
}

/// The defects of the photonic-crystal cavities and guide: L3 (three sites removed, the end
/// holes shifted out), H1 (one removed), W1 (a row removed), a resized hole: how many
/// elements are wrong (missing, present where removed, moved by other than the shift, or
/// changed where left alone, to the bit).
pub(crate) fn defect_errors() -> usize {
    let (a, r) = (AKAHANE_A, AKAHANE_R * AKAHANE_A);
    let hole = circle(0.0, 0.0, r);
    let s = AKAHANE_SHIFT * a;
    let Ok(base) = Lattice::hexagonal(um(a), [-7, 7], [-5, 5]) else {
        return usize::MAX;
    };
    let Ok(perfect) = Array::of(&base, &hole) else {
        return usize::MAX;
    };
    let mut wrong = 0;
    // each defect: the lattice, what it removes, what it moves and by how much, what it scales
    let w1 = (-7..=7).try_fold(base.clone(), |l, i| l.remove([i, 0]));
    type Defect = (
        Result<Lattice>,
        Vec<[i64; 2]>,
        Vec<([i64; 2], f64)>,
        Vec<([i64; 2], f64)>,
    );
    let cases: Vec<Defect> = vec![
        (
            l3_lattice(a, s, [7, 5]),
            vec![[-1, 0], [0, 0], [1, 0]],
            vec![([-2, 0], -s), ([2, 0], s)],
            vec![],
        ),
        (base.clone().remove([0, 0]), vec![[0, 0]], vec![], vec![]),
        (w1, (-7..=7).map(|i| [i, 0]).collect(), vec![], vec![]),
        (
            base.clone().resize([1, 1], 0.8),
            vec![],
            vec![],
            vec![([1, 1], 0.8)],
        ),
    ];
    for (lattice, removed, shifted, scaled) in cases {
        let Ok(lattice) = lattice else {
            return usize::MAX;
        };
        let Ok(array) = Array::of(&lattice, &hole) else {
            return usize::MAX;
        };
        if array.len() + removed.len() != perfect.len() {
            wrong += 1;
        }
        for e in perfect.elements() {
            let got = array.element(e.index);
            if removed.contains(&e.index) {
                wrong += usize::from(got.is_some());
                continue;
            }
            let Some(got) = got else {
                wrong += 1;
                continue;
            };
            let (
                Shape::Circle {
                    center: c0,
                    radius: r0,
                },
                Shape::Circle { center, radius },
            ) = (&e.shape, &got.shape)
            else {
                wrong += 1;
                continue;
            };
            let dx = shifted
                .iter()
                .find(|(k, _)| *k == e.index)
                .map_or(0.0, |s| s.1);
            let f = scaled
                .iter()
                .find(|(k, _)| *k == e.index)
                .map_or(1.0, |s| s.1);
            let moved = Point::um(c0.x.to_um() + dx, c0.y.to_um());
            let right = if dx == 0.0 {
                center == c0
            } else {
                *center == moved
            };
            if !right || *radius != *r0 * f {
                wrong += 1;
            }
        }
    }
    // a defect at a site that isn't there names it
    let named = |e: Result<Lattice>, says: &str| match e {
        Err(err) => !err.to_string().contains(says),
        Ok(_) => true,
    };
    wrong += usize::from(named(base.clone().remove([9, 0]), "(9, 0)"));
    wrong += usize::from(named(
        base.clone()
            .remove([0, 0])
            .and_then(|l| l.shift([0, 0], [um(0.1), um(0.0)])),
        "removed",
    ));
    wrong
}

/// A cell of two shapes (a disk and an L-shaped polygon) placed five times, translated,
/// rotated by quarter turns and otherwise, mirrored and scaled, against the shapes transformed
/// one by one: at 2000 points, how many inside tests and signed distances differ in any bit,
/// and whether the areas and bounds do. Also an array under a transform against its elements
/// transformed.
pub(crate) fn reference_errors() -> usize {
    let l = super::boolean_checks::polygon(&[
        [0.6, 0.0],
        [1.2, 0.0],
        [1.2, 0.2],
        [0.8, 0.2],
        [0.8, 0.6],
        [0.6, 0.6],
    ]);
    let Ok(cell) = Cell::new("pair", vec![circle(0.0, 0.0, 0.25), l]) else {
        return usize::MAX;
    };
    let Ok(scale) = Transform::scale(0.7, Point::um(0.1, 0.1)) else {
        return usize::MAX;
    };
    let placements = vec![
        Transform::translate(um(-4.0), um(0.0)),
        Transform::rotate(PI / 2.0, Point::um(0.0, 0.0))
            .then(&Transform::translate(um(-1.5), um(0.3))),
        Transform::rotate(0.4, Point::um(0.2, -0.1)).then(&Transform::translate(um(1.0), um(-1.0))),
        Transform::mirror(Point::um(0.0, 0.0), 0.0).then(&Transform::translate(um(3.5), um(1.0))),
        scale.then(&Transform::translate(um(0.5), um(2.5))),
    ];
    let Ok(reference) = Reference::new(cell.clone(), placements.clone()) else {
        return usize::MAX;
    };
    let shapes: Vec<Shape> = placements
        .iter()
        .flat_map(|t| cell.shapes().iter().map(move |s| s.transformed(t)))
        .collect();
    let mut wrong = 0;
    for k in 0..2000 {
        let (u, v) = (
            (k as f64 * 0.618_033_988_75).fract(),
            (k as f64 * 0.754_877_666_25).fract(),
        );
        let p = Point::um(-5.0 + 10.0 * u, -2.5 + 6.0 * v);
        let inside = shapes.iter().any(|s| s.contains(p));
        wrong += usize::from(reference.contains(p) != inside);
        let nearest = shapes
            .iter()
            .map(|s| s.distance(p))
            .reduce(|a, b| if b.signed < a.signed { b } else { a })
            .expect("shapes");
        let d = reference.distance(p);
        wrong += usize::from(d.signed.to_um().to_bits() != nearest.signed.to_um().to_bits());
        wrong += usize::from(d.normal != nearest.normal);
    }
    let area: f64 = shapes.iter().map(Shape::area).sum();
    wrong += usize::from(reference.area().to_bits() != area.to_bits());
    let hull = shapes.iter().map(Shape::bounds).reduce(|a, b| {
        Bounds::new(
            Point::um(
                a.min.x.to_um().min(b.min.x.to_um()),
                a.min.y.to_um().min(b.min.y.to_um()),
            ),
            Point::um(
                a.max.x.to_um().max(b.max.x.to_um()),
                a.max.y.to_um().max(b.max.y.to_um()),
            ),
        )
    });
    wrong += usize::from(Some(reference.bounds()) != hull);
    // a whole array moved and turned is its elements moved and turned
    let t =
        Transform::rotate(0.3, Point::um(1.0, 1.0)).then(&Transform::translate(um(2.0), um(-1.0)));
    let Ok(a) = Lattice::hexagonal(um(0.42), [-3, 3], [-2, 2])
        .and_then(|l| Array::of(&l, &circle(0.0, 0.0, 0.12)))
    else {
        return usize::MAX;
    };
    let moved = Shape::Array(a.clone()).transformed(&t);
    let Shape::Array(moved) = moved else {
        return usize::MAX;
    };
    for (e, m) in a.elements().iter().zip(moved.elements()) {
        wrong += usize::from(e.shape.transformed(&t) != m.shape || e.index != m.index);
    }
    wrong
}

/// A slab less a hole lattice, by inclusion–exclusion: a rectangle A over part of a hexagonal
/// lattice of holes H, cutting through some; |A \ H| + |A ∩ H| = |A|, |H \ A| + |A ∩ H| = |H|,
/// and with every hole inside a larger slab, |A \ H| = |A| − N π r². The largest relative
/// difference.
pub(crate) fn slab_error() -> f64 {
    let (a, r) = (AKAHANE_A, AKAHANE_R * AKAHANE_A);
    let Ok(holes) = Lattice::hexagonal(um(a), [-8, 8], [-6, 6])
        .and_then(|l| Array::of(&l, &circle(0.0, 0.0, r)))
    else {
        return f64::INFINITY;
    };
    let h = Shape::Array(holes);
    let slab = rect(-2.03, -1.71, 2.6, 1.33);
    let (Ok(d), Ok(i), Ok(rest)) = (
        slab.difference(&h),
        slab.intersection(&h),
        h.difference(&slab),
    ) else {
        return f64::INFINITY;
    };
    let mut worst = rel(d.area() + i.area(), slab.area());
    worst = worst.max(rel(rest.area() + i.area(), h.area()));
    let big = rect(-5.0, -4.0, 5.0, 4.0);
    let Ok(all) = big.difference(&h) else {
        return f64::INFINITY;
    };
    worst.max(rel(all.area(), big.area() - (17 * 13) as f64 * PI * r * r))
}

/// The polygons of a slab less a hexagonal hole lattice with an L3 cavity, at unsnapped
/// tolerances from 1 to 0.001 nm: the slope of −log of the distance they reach from the exact
/// boundary against log of their number of vertices.
pub(crate) fn order() -> f64 {
    let a = AKAHANE_A;
    let Ok(holes) = l3_lattice(a, AKAHANE_SHIFT * a, [4, 3])
        .and_then(|l| Array::of(&l, &circle(0.0, 0.0, AKAHANE_R * a)))
    else {
        return f64::NAN;
    };
    let Ok(slab) = rect(-2.0, -1.5, 2.0, 1.5).difference(&Shape::Array(holes)) else {
        return f64::NAN;
    };
    let mut points = Vec::new();
    for nm in [1.0, 0.1, 0.01, 0.001] {
        let p = slab.polygons(Tolerance::unsnapped(Length::nm(nm)).expect("a tolerance"));
        let (d, n) = deviation(&slab, &p);
        points.push(((n as f64).ln(), d.ln()));
    }
    let m = points.len() as f64;
    let (sx, sy) = points
        .iter()
        .fold((0.0, 0.0), |(a, b), p| (a + p.0, b + p.1));
    let (mx, my) = (sx / m, sy / m);
    let (num, den) = points.iter().fold((0.0, 0.0), |(a, b), p| {
        (a + (p.0 - mx) * (p.1 - my), b + (p.0 - mx) * (p.0 - mx))
    });
    -num / den
}

/// Akahane et al.'s L3 cavity drawn: a silicon slab 0.6 a thick in air (their 0.25 µm), less a
/// hexagonal lattice of air holes of radius 0.29 a (their 0.12 µm), a = 0.42 µm, with three
/// holes missing in a row and the two at its ends moved out by 0.15 a. Checked: the paper's
/// rounded values against a's multiples; 15 × 11 sites less three; every hole's radius and
/// centre against the lattice's closed form, the end holes at ±2.15 a; the cavity's length
/// between the end holes' centres, 4.3 a; its inside (the three sites, and the band the
/// shift uncovered) silicon and the holes air through the slab; and the unshifted cavity the
/// same but at the two end holes. The largest difference, µm (infinite if a count is wrong).
pub(crate) fn akahane_error() -> f64 {
    let a = AKAHANE_A;
    let (r, t, s) = (AKAHANE_R * a, AKAHANE_T * a, AKAHANE_SHIFT * a);
    // their rounded values: T = 0.6a (0.25 µm), R = 0.29a (0.12 µm), to the printed digit
    let mut worst = ((t - 0.25).abs() - 0.005).max(0.0) + ((r - 0.12).abs() - 0.005).max(0.0);
    let hole = circle(0.0, 0.0, r);
    let (Ok(shifted), Ok(start)) = (l3_lattice(a, s, [7, 5]), l3_lattice(a, 0.0, [7, 5])) else {
        return f64::INFINITY;
    };
    let (Ok(design), Ok(initial)) = (Array::of(&shifted, &hole), Array::of(&start, &hole)) else {
        return f64::INFINITY;
    };
    if design.len() != 15 * 11 - 3 || initial.len() != design.len() {
        return f64::INFINITY;
    }
    let row = a * 3f64.sqrt() / 2.0;
    for e in design.elements() {
        let [i, j] = e.index;
        let Shape::Circle { center, radius } = &e.shape else {
            return f64::INFINITY;
        };
        let mut x = (i as f64 + if j % 2 == 0 { 0.0 } else { 0.5 }) * a;
        if j == 0 && i.abs() == 2 {
            x += s * i.signum() as f64;
        }
        worst = worst
            .max((center.x.to_um() - x).abs())
            .max((center.y.to_um() - j as f64 * row).abs())
            .max((radius.to_um() - r).abs());
        // the starting structure differs only at the end holes
        let same = initial.element(e.index).is_some_and(|f| f.shape == e.shape);
        if same == (j == 0 && i.abs() == 2) {
            return f64::INFINITY;
        }
    }
    let ends = [[-2, 0], [2, 0]].map(|k| design.element(k).map(|e| e.shape.bounds()));
    let [Some(left), Some(right)] = ends else {
        return f64::INFINITY;
    };
    let length = 0.5 * ((right.min.x + right.max.x) - (left.min.x + left.max.x)).to_um();
    worst = worst.max((length - 4.3 * a).abs());
    // the slab in air, less the holes
    let air = material::vacuum();
    let Ok(stack) = LayerStack::new(
        air.clone(),
        vec![Layer {
            name: "Si".into(),
            thickness: um(t),
            material: material::silicon(),
            background: air.clone(),
        }],
        air,
    ) else {
        return f64::INFINITY;
    };
    let Ok(slab) = rect(-8.0 * a, -6.0 * row, 8.0 * a, 6.0 * row).difference(&Shape::Array(design))
    else {
        return f64::INFINITY;
    };
    let mut structure = Structure::new(stack);
    if structure.draw("Si", slab).is_err() {
        return f64::INFINITY;
    }
    let si = material::silicon();
    let wrong = |p: Point, z: f64, want_si: bool| {
        (structure.material_at(p, um(z)).name() == si.name()) != want_si
    };
    let mut misses = 0;
    for z in [0.01 * t, 0.5 * t, 0.99 * t] {
        // the cavity's sites, and the band the shift uncovered, 2a − r to 2.15a − r either side
        for x in [-a, 0.0, a, 2.0 * a - r + 0.5 * s, -(2.0 * a - r + 0.5 * s)] {
            misses += usize::from(wrong(Point::um(x, 0.0), z, true));
        }
        // every hole's centre and a point just inside its edge is air
        for e in shifted.sites() {
            let c = Point::um(e.at.x.to_um() + 0.99 * r, e.at.y.to_um());
            misses += usize::from(wrong(e.at, z, false)) + usize::from(wrong(c, z, false));
        }
    }
    if misses > 0 {
        return f64::INFINITY;
    }
    worst
}

fn error(e: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance,
        error: e,
    }
}

fn count(n: usize) -> Outcome {
    let n = n as f64;
    error(n, 0.0)
}

/// The validation cases of arrays, lattices and references.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "geometry/arrays/lattices",
            title: r"Rectangular (9 × 5 sites, pitches 0.37 and 0.53 µm), hexagonal (13 × 11, a = 0.42 µm) and polar (rings of 1, 6, 12, 18 and 24 sites, 0.5 µm apart) lattices: their counts, every site's position, and each interior hexagonal site's six nearest neighbours a away and next six $\sqrt3\thinspace a$ (largest difference over the pitch shown)",
            tier: Tier::Analytic,
            source: r"site $(i, j)$ at $(i a_x, j a_y)$; at $((i + \frac12 (j \bmod 2)) a, j a \sqrt3/2)$; at $r_j (\cos\theta, \sin\theta)$, $r_j = r_0 + j\Delta r$, $\theta = \theta_0 + 2\pi i / n_j$; measured 4.0e-16",
            run: || error(lattice_error(), 1e-14),
        },
        Case {
            id: "geometry/arrays/fill",
            title: r"Hole lattices' areas and fill fractions: 21 × 17 holes of radius 0.29a on a hexagonal lattice (a = 0.42 µm) and 25 × 19 of 0.15 µm on a 0.5 × 0.4 µm rectangular one against $N\pi r^2$, the hexagonal one's fills over a 97 × 97 grid summed against it, and unit cells at four places (the holes they cut, in pieces) against $2\pi r^2/(\sqrt3\thinspace a^2)$ and $\pi r^2/(a_x a_y)$ (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"each hole's exact area and its exact fill of a cell (the disk clipped), summed over the holes whose boxes meet the cell, found through the index over the elements; measured 5.3e-15",
            run: || error(area_error(), 1e-12),
        },
        Case {
            id: "geometry/arrays/index-expressions",
            title: r"Elements made from their index and place: an apodized grating's 21 teeth, width $0.2 + 0.01 i$ µm and moved by $2 i^2$ nm, and a metalens's 625 pillars on a 0.4 µm hexagonal lattice, radius $60 + 50\thinspace\varphi/2\pi$ nm for the hyperbolic phase $\varphi$ (f = 10 µm, λ = 1.55 µm) at the pillar's place, each expression evaluated at every element, against its value (largest difference shown, µm)",
            tier: Tier::Analytic,
            source: r"the expressions' values at $i$, $j$, $x$, $y$ of each site, the elements moved there; measured 1.4e-15",
            run: || error(index_error(), 1e-12),
        },
        Case {
            id: "geometry/arrays/defects",
            title: r"Defects on a 15 × 11 hexagonal hole lattice: L3 (three holes removed in a row, the two at its ends moved out by 0.15a), H1 (one removed), W1 (a row removed), one hole resized by 0.8; every other hole the same to the bit, each removed hole gone, each moved one moved by exactly its shift, and a defect at a missing or removed site refused, naming it (elements wrong shown)",
            tier: Tier::Analytic,
            source: r"what each defect says; measured 0",
            run: || count(defect_errors()),
        },
        Case {
            id: "geometry/arrays/references",
            title: r"A cell of a disk and an L placed five times (translated, rotated by a quarter turn and by 0.4 rad, mirrored, scaled by 0.7), and a hole lattice rotated and moved: their inside tests, signed distances and normals at 2000 points, areas and bounds against the shapes transformed one by one (differences in any bit shown)",
            tier: Tier::Analytic,
            source: r"a reference's element $(k, m)$ is the cell's shape $m$ under placement $k$, its own transform; measured 0",
            run: || count(reference_errors()),
        },
        Case {
            id: "geometry/arrays/slab-less-holes",
            title: r"A slab less a hexagonal lattice of 17 × 13 holes (r = 0.29a, a = 0.42 µm), cutting through some: $\lvert A\setminus H\rvert + \lvert A\cap H\rvert = \lvert A\rvert$, $\lvert H\setminus A\rvert + \lvert A\cap H\rvert = \lvert H\rvert$, and a slab over all of them $\lvert A\rvert - N\pi r^2$ (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"inclusion–exclusion; the booleans' areas from polygons within $10^{-7}$ of their size; measured 9.8e-15",
            run: || error(slab_error(), 1e-9),
        },
        Case {
            id: "geometry/arrays/convergence",
            title: r"A slab less a hexagonal hole lattice with an L3 cavity, as polygons at unsnapped tolerances from 1 to 0.001 nm: the slope of $-\log$ of the distance they reach from the exact boundary against $\log$ of their number of vertices",
            tier: Tier::Analytic,
            source: r"each arc's chords of length $h$ stray by $\kappa h^2/8$: second order (the slope shown to two decimals)",
            run: || {
                let o = (order() * 100.0).round() / 100.0;
                Outcome {
                    measured: o,
                    expected: 2.0,
                    tolerance: 0.15,
                    error: (o - 2.0).abs(),
                }
            },
        },
        Case {
            id: "geometry/arrays/akahane-l3",
            title: r"Akahane et al.'s L3 cavity drawn: a silicon slab 0.6a thick in air less a hexagonal lattice (a = 0.42 µm, 15 × 11 sites) of air holes of radius 0.29a, three missing in a row and the two at its ends moved out by 0.15a. Its 162 holes' centres and radii against the lattice, the cavity's length between the moved holes, 4.3a, the cavity and the band the shift uncovers silicon and every hole air through the slab, and the unshifted cavity different only at the end holes (largest difference shown, µm); geometry only",
            tier: Tier::Published,
            source: r"Y. Akahane, T. Asano, B.-S. Song, S. Noda, Nature 425, 944 (2003), doi:10.1038/nature02063, Fig. 1 and p. 945: a = 0.42 µm, slab 0.6a (0.25 µm), radius 0.29a (0.12 µm), three missing air rods in a line, the rods at both edges shifted by 0.15a (Figs. 1c, 3c); measured 2.2e-16",
            run: || error(akahane_error(), 1e-12),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "writes the cases' rows of the report to PHOTONOXIDE_ARRAY_ROWS"]
    fn rows() {
        let rows: String = cases()
            .iter()
            .map(|c| crate::validation::row(c).0)
            .collect();
        if let Some(out) = std::env::var_os("PHOTONOXIDE_ARRAY_ROWS") {
            std::fs::write(out, rows).expect("the rows are written");
        }
    }

    #[test]
    #[ignore = "prints the cases' values; cargo test --release --lib array_checks -- --ignored --nocapture"]
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
    }
}
