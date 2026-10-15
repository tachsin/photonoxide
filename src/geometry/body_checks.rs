//! What the validation report and the tests check bodies on: the primitives' distances and
//! normals against closed forms, their volumes from their cells' fills summed over grids, an
//! axis-aligned box's fills against the boxes' overlaps, extrusions against area × height and
//! a frustum's volume, booleans of two spheres against inclusion–exclusion, the tangent planes'
//! fill converging at second order, FDTD's smoothing on these bodies to the bit, and the L3
//! cavity of Akahane et al. 2003 as a body.

use std::f64::consts::PI;

use super::array_checks::{AKAHANE_A, AKAHANE_R, AKAHANE_SHIFT, AKAHANE_T, l3_lattice};
use super::*;
use crate::units::Length;
use crate::validation::{Case, Outcome, Tier};

fn um(v: f64) -> Length {
    Length::um(v)
}

/// Orthonormal axes (one per row): a rotation by `a` about z, then `b` about x, then `c` about
/// z.
pub(crate) fn rotation(a: f64, b: f64, c: f64) -> [[f64; 3]; 3] {
    let rz = |t: f64| {
        let (s, c) = t.sin_cos();
        [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
    };
    let rx = |t: f64| {
        let (s, c) = t.sin_cos();
        [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]]
    };
    let mul = |p: [[f64; 3]; 3], q: [[f64; 3]; 3]| -> [[f64; 3]; 3] {
        std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| p[i][k] * q[k][j]).sum()))
    };
    let m = mul(rz(c), mul(rx(b), rz(a)));
    // its columns as rows
    std::array::from_fn(|i| std::array::from_fn(|j| m[j][i]))
}

/// The sum of a body's fills over a grid of n × n × n equal cells from `lo` to `hi`, in a fixed
/// order.
pub(crate) fn grid_sum(body: &Body, lo: [f64; 3], hi: [f64; 3], n: usize) -> f64 {
    let h: [f64; 3] = std::array::from_fn(|k| (hi[k] - lo[k]) / n as f64);
    let mut sum = 0.0;
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                let a = [
                    lo[0] + i as f64 * h[0],
                    lo[1] + j as f64 * h[1],
                    lo[2] + k as f64 * h[2],
                ];
                let b = [a[0] + h[0], a[1] + h[1], a[2] + h[2]];
                sum += body.fill(Bounds3d::new(a, b));
            }
        }
    }
    sum
}

/// The volume two spheres of radii `ra` and `rb`, centres `d` apart, share: two caps.
pub(crate) fn lens(ra: f64, rb: f64, d: f64) -> f64 {
    if d >= ra + rb {
        return 0.0;
    }
    if d <= (ra - rb).abs() {
        return 4.0 / 3.0 * PI * ra.min(rb).powi(3);
    }
    PI * (ra + rb - d).powi(2)
        * (d * d + 2.0 * d * rb - 3.0 * rb * rb + 2.0 * d * ra + 6.0 * ra * rb - 3.0 * ra * ra)
        / (12.0 * d)
}

/// The primitives against their closed-form volumes: a sphere, an ellipsoid at an angle to the
/// grid, a cylinder along z and a cone's frustum along x, their cells' fills over a 9 × 9 × 9
/// grid that cuts them anywhere summed. The largest relative difference.
pub(crate) fn volume_error() -> f64 {
    let axes = rotation(0.4, -0.9, 1.3);
    let Ok(cases) = (|| -> crate::Result<Vec<(Body, f64)>> {
        Ok(vec![
            (
                Body::sphere([0.013, -0.021, 0.034], 0.37)?,
                4.0 / 3.0 * PI * 0.37f64.powi(3),
            ),
            (
                Body::ellipsoid([0.02, 0.01, -0.03], [0.41, 0.27, 0.19], axes)?,
                4.0 / 3.0 * PI * 0.41 * 0.27 * 0.19,
            ),
            (
                Body::cylinder([0.01, -0.02, -0.33], [0.01, -0.02, 0.29], 0.21)?,
                PI * 0.21 * 0.21 * 0.62,
            ),
            (
                Body::cone([-0.31, 0.02, 0.01], [0.33, 0.02, 0.01], [0.27, 0.09])?,
                PI * 0.64 * (0.27 * 0.27 + 0.27 * 0.09 + 0.09 * 0.09) / 3.0,
            ),
        ])
    })() else {
        return f64::INFINITY;
    };
    cases
        .iter()
        .map(|(body, v)| {
            let sum = grid_sum(body, [-0.5; 3], [0.5; 3], 9);
            ((sum - v) / v).abs().max(((body.volume() - v) / v).abs())
        })
        .fold(0.0, f64::max)
}

/// A box's fills over a 7 × 6 × 5 grid of unequal cells cutting it every which way, against the
/// overlap of each cell with the box: the largest difference, µm³.
pub(crate) fn box_error() -> f64 {
    let (lo, hi) = ([0.0, -0.2, 0.1], [0.5, 0.3, 0.4]);
    let Ok(b) = Body::cuboid(lo, hi) else {
        return f64::INFINITY;
    };
    let h = [0.11, 0.13, 0.09];
    let mut worst = 0.0f64;
    for i in 0..7 {
        for j in 0..6 {
            for k in 0..5 {
                let a = [
                    -0.05 + i as f64 * h[0],
                    -0.31 + j as f64 * h[1],
                    0.03 + k as f64 * h[2],
                ];
                let cell = Bounds3d::new(a, [a[0] + h[0], a[1] + h[1], a[2] + h[2]]);
                let want = (0..3)
                    .map(|x| (cell.max[x].min(hi[x]) - cell.min[x].max(lo[x])).max(0.0))
                    .product::<f64>();
                worst = worst.max((b.fill(cell) - want).abs());
            }
        }
    }
    worst
}

/// Extrusions: a pentagon between two heights with vertical walls against its area times the
/// height (its fills over an 8 × 8 × 8 grid summed, and its volume), and a disk with walls at
/// 70° against its cone's frustum, π h (R₀² + R₀R₁ + R₁²)/3. The larger relative difference.
pub(crate) fn extrusion_error() -> f64 {
    let Ok(poly) = Polygon::new(vec![
        Point::um(-0.3, -0.2),
        Point::um(0.35, -0.25),
        Point::um(0.2, 0.3),
        Point::um(-0.1, 0.15),
        Point::um(-0.25, 0.28),
    ]) else {
        return f64::INFINITY;
    };
    let want = poly.area() * 0.34;
    let Ok(prism) = Body::prism(Shape::Polygon(poly), [-0.11, 0.23]) else {
        return f64::INFINITY;
    };
    let sum = grid_sum(&prism, [-0.5; 3], [0.5; 3], 8);
    let mut worst = ((sum - want) / want)
        .abs()
        .max(((prism.volume() - want) / want).abs());
    let (h, r, angle) = (0.3, 0.2, 70f64.to_radians());
    let run = 1.0 / angle.tan();
    let Ok(Ok(disk)) = Shape::circle(Point::um(0.01, -0.02), um(r))
        .map(|c| Body::etched(c, [-0.1, -0.1 + h], angle))
    else {
        return f64::INFINITY;
    };
    let r0 = r + h * run;
    let want = PI * h * (r0 * r0 + r0 * r + r * r) / 3.0;
    let sum = grid_sum(&disk, [-0.5; 3], [0.5; 3], 9);
    worst = worst.max(((sum - want) / want).abs());
    worst
}

/// A rectangle's top face 0.4 × 0.25 µm with walls at 70° over 0.3 µm: its top face grown by
/// (top − z) cot θ with round corners at each height, so A h + P run h²/2 + π run² h³/3
/// (Steiner's formula integrated), against its fills over a 6 × 6 × 6 grid, which take the
/// corners' cones by halving: the relative difference.
pub(crate) fn sloped_error() -> f64 {
    let (w, d, h, angle) = (0.4, 0.25, 0.3, 70f64.to_radians());
    let run = 1.0 / angle.tan();
    let Ok(Ok(rect)) = Shape::rect(Point::um(0.0, 0.0), um(w), um(d))
        .map(|s| Body::etched(s, [-0.1, -0.1 + h], angle))
    else {
        return f64::INFINITY;
    };
    let want = w * d * h + (w + d) * run * h * h + PI * run * run * h.powi(3) / 3.0;
    ((grid_sum(&rect, [-0.5; 3], [0.5; 3], 6) - want) / want).abs()
}

/// Distances and normals against closed forms at 400 points: a sphere's |p − c| − r, a box's,
/// a cylinder's and a cone's in the plane through their axes, and an ellipsoid's foot on its
/// surface (Σ (uᵢ/eᵢ)² = 1) with the normal its gradient's direction there. The largest
/// difference.
pub(crate) fn distance_error() -> f64 {
    let mut worst = 0.0f64;
    let mut s: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    };
    let pts: Vec<[f64; 3]> = (0..400).map(|_| [next(), next(), next()]).collect();
    let (Ok(sphere), Ok(cube), Ok(cyl), Ok(cone)) = (
        Body::sphere([0.1, -0.2, 0.05], 0.45),
        Body::cuboid([-0.3, -0.2, -0.1], [0.4, 0.25, 0.35]),
        Body::cylinder([0.0, 0.0, -0.4], [0.0, 0.0, 0.3], 0.35),
        Body::cone([0.0, 0.0, -0.4], [0.0, 0.0, 0.3], [0.5, 0.1]),
    ) else {
        return f64::INFINITY;
    };
    let axes = rotation(0.3, 1.2, -0.7);
    let (c, e) = ([0.1, 0.2, -0.3], [0.9, 0.5, 0.3]);
    let Ok(ellipsoid) = Body::ellipsoid(c, e, axes) else {
        return f64::INFINITY;
    };
    // the distance in the plane through the z axis to a profile from (t₀, r₀) to (t₁, r₁)
    let turned = |p: [f64; 3], t: [f64; 2], r: [f64; 2]| -> f64 {
        let (z, rho) = (p[2], p[0].hypot(p[1]));
        let seg = |a: [f64; 2], b: [f64; 2]| {
            let e = [b[0] - a[0], b[1] - a[1]];
            let s = (((z - a[0]) * e[0] + (rho - a[1]) * e[1]) / (e[0] * e[0] + e[1] * e[1]))
                .clamp(0.0, 1.0);
            (z - a[0] - s * e[0]).hypot(rho - a[1] - s * e[1])
        };
        let d = seg([t[0], 0.0], [t[0], r[0]])
            .min(seg([t[0], r[0]], [t[1], r[1]]))
            .min(seg([t[1], r[1]], [t[1], 0.0]));
        let r_at = r[0] + (r[1] - r[0]) * (z - t[0]) / (t[1] - t[0]);
        if z >= t[0] && z <= t[1] && rho <= r_at {
            -d
        } else {
            d
        }
    };
    for &p in &pts {
        let q = [p[0] - 0.1, p[1] + 0.2, p[2] - 0.05];
        let r = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt();
        worst = worst.max((sphere.distance(p).signed.to_um() - (r - 0.45)).abs());
        // the box
        let (lo, hi) = ([-0.3, -0.2, -0.1], [0.4, 0.25, 0.35]);
        let out: [f64; 3] = std::array::from_fn(|k| (lo[k] - p[k]).max(p[k] - hi[k]).max(0.0));
        let inside = (0..3).all(|k| p[k] > lo[k] && p[k] < hi[k]);
        let want = if inside {
            -(0..3)
                .map(|k| (p[k] - lo[k]).min(hi[k] - p[k]))
                .fold(f64::INFINITY, f64::min)
        } else {
            (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt()
        };
        worst = worst.max((cube.distance(p).signed.to_um() - want).abs());
        worst = worst
            .max((cyl.distance(p).signed.to_um() - turned(p, [-0.4, 0.3], [0.35, 0.35])).abs());
        worst =
            worst.max((cone.distance(p).signed.to_um() - turned(p, [-0.4, 0.3], [0.5, 0.1])).abs());
        // the ellipsoid: its foot on the surface, the normal its gradient's direction
        let d = ellipsoid.distance(p);
        let ds = d.signed.to_um();
        let foot: [f64; 3] = std::array::from_fn(|k| p[k] - ds * d.normal[k]);
        let u: [f64; 3] = std::array::from_fn(|i| {
            (0..3).map(|k| (foot[k] - c[k]) * axes[i][k]).sum::<f64>() / e[i]
        });
        worst = worst.max((u[0] * u[0] + u[1] * u[1] + u[2] * u[2] - 1.0).abs());
        let g: [f64; 3] =
            std::array::from_fn(|k| (0..3).map(|i| u[i] / e[i] * axes[i][k]).sum::<f64>());
        let gl = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt();
        worst = worst.max(
            (0..3)
                .map(|k| (g[k] / gl - d.normal[k]).abs())
                .fold(0.0, f64::max),
        );
    }
    worst
}

/// Two spheres (radii 0.3 and 0.22 µm, centres 0.31 µm apart): their union, intersection,
/// difference and xor, each's fills over a 6 × 6 × 6 grid summed, against the volumes from the
/// lens the two share, |A ∪ B| = |A| + |B| − |A ∩ B| and so on. The largest relative
/// difference.
pub(crate) fn boolean_error() -> f64 {
    let (ra, rb, d) = (0.3, 0.22, 0.31);
    let (Ok(a), Ok(b)) = (
        Body::sphere([-0.1, 0.02, 0.01], ra),
        Body::sphere([-0.1 + d, 0.02, 0.01], rb),
    ) else {
        return f64::INFINITY;
    };
    let (va, vb) = (4.0 / 3.0 * PI * ra.powi(3), 4.0 / 3.0 * PI * rb.powi(3));
    let both = lens(ra, rb, d);
    [
        (Operation::Union, va + vb - both),
        (Operation::Intersection, both),
        (Operation::Difference, va - both),
        (Operation::Xor, va + vb - 2.0 * both),
    ]
    .into_iter()
    .map(|(op, want)| {
        let body = Body::boolean(op, a.clone(), b.clone());
        ((grid_sum(&body, [-0.5; 3], [0.5; 3], 6) - want) / want).abs()
    })
    .fold(0.0, f64::max)
}

/// A cylinder (radius 0.17 µm) and a cone's frustum (radii 0.2 and 0.05 µm) along an axis
/// oblique to the grid: their fills over a 7 × 7 × 7 grid summed, the side's slices exact where
/// the caps don't cross a cell and the tangent planes where they do. The larger relative
/// difference from π r² L and π L (r₀² + r₀r₁ + r₁²)/3.
pub(crate) fn oblique_error() -> f64 {
    let (a, b): ([f64; 3], [f64; 3]) = ([0.3, -0.2, 0.25], [-0.25, 0.3, -0.2]);
    let l = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
    let (Ok(cyl), Ok(cone)) = (Body::cylinder(a, b, 0.17), Body::cone(a, b, [0.2, 0.05])) else {
        return f64::INFINITY;
    };
    let want_cyl = PI * 0.17 * 0.17 * l;
    let want_cone = PI * l * (0.04 + 0.01 + 0.0025) / 3.0;
    let e1 = ((grid_sum(&cyl, [-0.5; 3], [0.5; 3], 7) - want_cyl) / want_cyl).abs();
    let e2 = ((grid_sum(&cone, [-0.5; 3], [0.5; 3], 7) - want_cone) / want_cone).abs();
    e1.max(e2)
}

/// What FDTD's smoothing takes from a sphere (radius 0.37 µm): each cell's share of the plane
/// through the nearest point of the surface to its centre, summed over grids of 16, 32 and 64
/// cells a µm. The order at which the error in the volume falls, from the two finer grids.
pub(crate) fn tangent_order() -> f64 {
    let r = 0.37;
    let Ok(sphere) = Body::sphere([0.013, -0.021, 0.034], r) else {
        return f64::NAN;
    };
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
    (error(32) / error(64)).log2()
}

/// Akahane et al.'s L3 cavity as a body in space: a silicon slab 0.6a thick (z from −0.3a to
/// 0.3a) less the air holes (radius 0.29a, a = 0.42 µm, 15 × 11 sites, three missing in a row
/// and the two at its ends moved out by 0.15a) through it, the slab a box and the holes the
/// lattice's disks extruded without end. Checked: its volume from its cells' fills over a grid
/// of a/4 cells across the slab, against T (|A| − N π r²) exactly (every hole inside the
/// slab); the inside tests of the cavity's sites, the band the shift uncovers, every hole's
/// centre and a point just inside its edge, above and below the slab, through it; and FDTD's
/// smoothing structure on the body answering the same. The relative difference in volume
/// (infinite if a point is wrong).
pub(crate) fn akahane_body_error() -> f64 {
    let a = AKAHANE_A;
    let (r, t, s) = (AKAHANE_R * a, AKAHANE_T * a, AKAHANE_SHIFT * a);
    let row = a * 3f64.sqrt() / 2.0;
    let (half_x, half_y) = (8.0 * a, 6.0 * row);
    let Ok(lattice) = l3_lattice(a, s, [7, 5]) else {
        return f64::INFINITY;
    };
    let Ok(hole) = Shape::circle(Point::um(0.0, 0.0), um(r)) else {
        return f64::INFINITY;
    };
    let Ok(holes) = Array::of(&lattice, &hole) else {
        return f64::INFINITY;
    };
    let n = holes.len();
    let Ok(slab) = Body::cuboid([-half_x, -half_y, -0.5 * t], [half_x, half_y, 0.5 * t]) else {
        return f64::INFINITY;
    };
    let body = Body::difference(slab, Body::extruded(Shape::Array(holes)));
    let want = t * (4.0 * half_x * half_y - n as f64 * PI * r * r);
    // a/4 cells over the slab and beyond it
    let h = 0.25 * a;
    let (nx, ny, nz) = (
        (2.0 * half_x / h).ceil() as usize + 2,
        (2.0 * half_y / h).ceil() as usize + 2,
        4usize,
    );
    let mut sum = 0.0;
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                let lo = [
                    -half_x - h + i as f64 * h,
                    -half_y - h + j as f64 * h,
                    -2.0 * h + k as f64 * h,
                ];
                sum += body.fill(Bounds3d::new(lo, [lo[0] + h, lo[1] + h, lo[2] + h]));
            }
        }
    }
    let Ok(si) = crate::fdtd::Permittivity::isotropic(12.0) else {
        return f64::INFINITY;
    };
    let Ok(air) = crate::fdtd::Permittivity::isotropic(1.0) else {
        return f64::INFINITY;
    };
    let fdtd = crate::fdtd::Structure::new(air).with(body.clone(), si);
    let mut misses = 0;
    let mut expect = |p: [f64; 3], want_si: bool| {
        misses += usize::from(body.contains(p) != want_si);
        misses += usize::from((fdtd.at(p) == si) != want_si);
    };
    for z in [-0.49 * t, 0.0, 0.49 * t] {
        for x in [-a, 0.0, a, 2.0 * a - r + 0.5 * s, -(2.0 * a - r + 0.5 * s)] {
            expect([x, 0.0, z], true);
        }
        for e in lattice.sites() {
            let (x, y) = (e.at.x.to_um(), e.at.y.to_um());
            expect([x, y, z], false);
            expect([x + 0.99 * r, y, z], false);
        }
    }
    for z in [-0.51 * t, 0.51 * t] {
        expect([0.0, 0.0, z], false);
    }
    if misses > 0 {
        return f64::INFINITY;
    }
    ((sum - want) / want).abs()
}

fn error(e: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance,
        error: e,
    }
}

/// The validation cases of bodies.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "geometry/bodies/volumes",
            title: r"Bodies' volumes from their cells' fills: a sphere (r = 0.37 µm), an ellipsoid at an angle to the grid (0.41, 0.27, 0.19 µm), a cylinder along z (r = 0.21 µm, 0.62 µm long) and a cone's frustum along x (radii 0.27 and 0.09 µm, 0.64 µm long), each cell's fill from its slices across a grid axis, summed over a 9 × 9 × 9 grid of a 1 µm cube that cuts them anywhere (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"$\frac43\pi abc$, $\pi r^2 L$, $\pi L (r_0^2 + r_0 r_1 + r_1^2)/3$; each slice's area in a cell exact by clipping its arc, integrated piecewise between the closed-form roots where it isn't analytic; measured 5.0e-15",
            run: || error(volume_error(), 1e-12),
        },
        Case {
            id: "geometry/bodies/box",
            title: r"An axis-aligned box's fills of a 7 × 6 × 5 grid of cells (0.11 × 0.13 × 0.09 µm) cutting it every which way, against each cell's overlap with the box (largest difference shown, µm³)",
            tier: Tier::Analytic,
            source: r"the product of the overlaps along each axis; measured 0",
            run: || error(box_error(), 1e-18),
        },
        Case {
            id: "geometry/bodies/extrusions",
            title: r"Extrusions: a pentagon between two heights with vertical walls, its fills over an 8 × 8 × 8 grid summed against its area times its height, and a disk (r = 0.2 µm) with walls at 70° over 0.3 µm, its fills over a 9 × 9 × 9 grid against its cone's frustum (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"$A h$, the polygon's area in each cell exact by clipping; $\pi h (R_0^2 + R_0 R_1 + R_1^2)/3$, $R_0 = R_1 + h \cot\theta$; measured 8.9e-16",
            run: || error(extrusion_error(), 1e-12),
        },
        Case {
            id: "geometry/bodies/sloped-walls",
            title: r"A rectangle's top face, 0.4 × 0.25 µm, its walls at 70° over 0.3 µm (phase 8's sidewall: at each height its top face grown by $(z_1 - z)\cot\theta$, with round corners): its fills over a 6 × 6 × 6 grid summed, the corners' cones by halving each cell they cross to 8 levels (relative difference shown)",
            tier: Tier::Analytic,
            source: r"Steiner's formula $A + P\rho + \pi\rho^2$ of a convex shape grown by $\rho$, integrated over the height: $A h + P h^2 \cot\theta/2 + \pi h^3 \cot^2\theta/3$; measured 3.7e-8",
            run: || error(sloped_error(), 1e-7),
        },
        Case {
            id: "geometry/bodies/distances",
            title: r"Bodies' signed distances and normals at 400 points of a 2 µm cube: a sphere's, a box's, a cylinder's and a cone's frustum's (in the plane through their axis, to the profiles they turn), and an ellipsoid at an angle to the grid (semi-axes 0.9, 0.5, 0.3 µm), its point at the distance along the normal on its surface and the normal its gradient's direction there (largest difference shown)",
            tier: Tier::Analytic,
            source: r"$\lvert p - c\rvert - r$, the box's and the profiles' closed forms; the ellipsoid's nearest point on $\sum (u_i/e_i)^2 = 1$, its normal $\nabla$ of it; measured 1.6e-13",
            run: || error(distance_error(), 1e-12),
        },
        Case {
            id: "geometry/bodies/booleans",
            title: r"Booleans of two spheres (radii 0.3 and 0.22 µm, centres 0.31 µm apart): union, intersection, difference and xor, their fills over a 6 × 6 × 6 grid summed, each cell that both surfaces cross halved to 8 levels and its last parts from the two tangent planes (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"$\lvert A\cup B\rvert = \lvert A\rvert + \lvert B\rvert - \lvert A\cap B\rvert$ and the rest, $\lvert A\cap B\rvert$ the lens of two spherical caps, $\pi (r_a + r_b - d)^2 (d^2 + 2d(r_a + r_b) - 3(r_a - r_b)^2 \ldots)/12d$; measured 1.3e-7",
            run: || error(boolean_error(), 1e-6),
        },
        Case {
            id: "geometry/bodies/oblique",
            title: r"A cylinder (r = 0.17 µm) and a cone's frustum (radii 0.2 and 0.05 µm) along an axis oblique to the grid: their fills over a 7 × 7 × 7 grid summed, the side's elliptical slices across the grid axis nearest its own exact, the cells that a cap crosses too halved to 8 levels (larger relative difference shown)",
            tier: Tier::Analytic,
            source: r"$\pi r^2 L$, $\pi L (r_0^2 + r_0 r_1 + r_1^2)/3$; measured 1.5e-8",
            run: || error(oblique_error(), 1e-7),
        },
        Case {
            id: "geometry/bodies/tangent-order",
            title: r"What FDTD's smoothing takes from a body: each cell's share of the plane through the nearest point of a sphere's surface (r = 0.37 µm) to its centre, summed over grids of 32 and 64 cells a µm; the order at which the volume's error falls (shown)",
            tier: Tier::Analytic,
            source: r"a curved surface strays from its tangent plane by $\kappa\rho^2/2$ across a cell: second order",
            run: || {
                let o = (tangent_order() * 100.0).round() / 100.0;
                Outcome {
                    measured: o,
                    expected: 2.0,
                    tolerance: 0.25,
                    error: (o - 2.0).abs(),
                }
            },
        },
        Case {
            id: "geometry/bodies/fdtd-bits",
            title: r"FDTD's smoothing reads its bodies from here: half-spaces, ellipsoids, elliptic cylinders, extruded shapes and etched ones with sloped and vertical walls, eight bodies, against FDTD's own bodies before they moved, kept word for word: each one's surface (distance and normal) and its box's test at 15 000 points, and the share of a cell a plane cuts off for 20 000 planes and cells, edges and faces among them (values differing in any bit shown)",
            tier: Tier::Analytic,
            source: r"the same arithmetic on the same bodies, the smoothing's averages, tensors and triplets the same code on them (FDTD's bodies of commit ff50758); measured 0",
            run: || {
                let e = crate::fdtd::smoothing::checks::bodies_differences() as f64;
                error(e, 0.0)
            },
        },
        Case {
            id: "geometry/bodies/akahane-l3",
            title: r"Akahane et al.'s L3 cavity as a body: a silicon slab 0.6a thick less the air holes through it (radius 0.29a, a = 0.42 µm, 15 × 11 sites, three missing in a row, the two at its ends moved out by 0.15a), a box less the lattice's disks extruded. Its volume from its fills over a grid of a/4 cells, the cavity and the shift's band silicon and every hole air through the slab, air above and below it, and FDTD's structure on the body answering the same (relative difference in volume shown); geometry only",
            tier: Tier::Published,
            source: r"Y. Akahane, T. Asano, B.-S. Song, S. Noda, Nature 425, 944 (2003), doi:10.1038/nature02063, Fig. 1 and p. 945: a = 0.42 µm, slab 0.6a, radius 0.29a, the rods at both edges shifted by 0.15a; the volume $T(\lvert A\rvert - N\pi r^2)$; measured 7.8e-9",
            run: || error(akahane_body_error(), 1e-7),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "writes the cases' rows of the report to PHOTONOXIDE_BODY_ROWS"]
    fn rows() {
        let rows: String = cases()
            .iter()
            .map(|c| crate::validation::row(c).0)
            .collect();
        if let Some(out) = std::env::var_os("PHOTONOXIDE_BODY_ROWS") {
            std::fs::write(out, rows).expect("the rows are written");
        }
    }

    #[test]
    #[ignore = "prints the cases' values; cargo test --release --lib body_checks -- --ignored --nocapture"]
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
