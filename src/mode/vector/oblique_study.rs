//! A study of the index at an interface oblique to the grid (issue #815), run by hand: the
//! tables of #815 and of docs/methods/vector.md and eme.md come from it.

use super::*;
use crate::averaging::{Averaging, Plane, Shares};
use crate::geometry::{Point, Shape};
use crate::units::Length;

/// How a cell the interface crosses takes its permittivity.
#[derive(Clone, Copy, Debug, PartialEq)]
enum How {
    /// The material at the centre.
    Stair,
    /// ⟨ε⟩ for every component.
    Mean,
    /// The diagonal of Farjadpour et al.'s tensor.
    Diagonal,
    /// Their whole tensor.
    Tensor,
}

fn cell(plane: &Plane, c: (f64, f64), size: (f64, f64), how: How) -> Permittivity {
    if how == How::Stair {
        return Permittivity::isotropic(plane.at(c.0, c.1));
    }
    let s: Shares<c64> = plane.shares_at(c, size);
    if s.is_uniform() || how == How::Mean {
        return Permittivity::isotropic(s.mean());
    }
    let [a, b, x] = s.inverse_tensor();
    let zero = c64::new(0.0, 0.0);
    // a normal along an axis to rounding has no off-diagonal part (a wall beside it refuses one)
    let x = if portable::complex::abs(x) < 1e-12 * portable::complex::abs(a) {
        zero
    } else {
        x
    };
    if how == How::Diagonal {
        return Permittivity {
            xx: 1.0 / a,
            xy: zero,
            yx: zero,
            yy: 1.0 / b,
            zz: s.mean(),
        };
    }
    let det = a * b - x * x;
    Permittivity {
        xx: b / det,
        xy: -x / det,
        yx: -x / det,
        yy: a / det,
        zz: s.mean(),
    }
}

fn section(plane: &Plane, x: &[f64], y: &[f64], how: How) -> Vec<Permittivity> {
    let mut cells = Vec::new();
    for i in 0..x.len() - 1 {
        for j in 0..y.len() - 1 {
            cells.push(cell(
                plane,
                (0.5 * (x[i] + x[i + 1]), 0.5 * (y[j] + y[j + 1])),
                (x[i + 1] - x[i], y[j + 1] - y[j]),
                how,
            ));
        }
    }
    cells
}

fn line(a: f64, b: f64, n: usize) -> Vec<f64> {
    (0..=n).map(|i| a + (b - a) * i as f64 / n as f64).collect()
}

const SI: f64 = 3.45;
const SIO2: f64 = 1.45;

fn lam() -> Wavelength {
    Wavelength::from_um_unchecked(0.8)
}

/// The rod's (D µm) HE₁₁ index on `n` cells a side over a window `side` µm from the centre:
/// the whole window, or its quarter behind walls.
fn rod(d: f64, n: usize, side: f64, how: How, quarter: bool, offset: [f64; 2]) -> f64 {
    let mut plane = Plane::new(c64::new(SIO2 * SIO2, 0.0));
    plane.draw(
        Shape::circle(Point::um(offset[0], offset[1]), Length::um(d / 2.0)).unwrap(),
        c64::new(SI * SI, 0.0),
    );
    let x = if quarter {
        line(0.0, side, n)
    } else {
        line(-side, side, 2 * n)
    };
    let cells = section(&plane, &x, &x, how);
    let mut cs = CrossSection::new(x.clone(), x, cells).unwrap();
    if quarter {
        cs = cs
            .with_boundaries(Boundaries {
                west: Boundary::ElectricWall,
                south: Boundary::MagneticWall,
                ..Boundaries::default()
            })
            .unwrap();
    }
    modes(&cs, lam(), 1, Some(1.75)).unwrap()[0]
        .effective_index()
        .re
}

/// A stadium `w` × `t` µm (its ends half-circles) turned by `angle` degrees, its quasi-TE index
/// on `2n` cells a side over ±`side` µm.
fn stadium(w: f64, t: f64, angle: f64, n: usize, side: f64, how: How) -> f64 {
    let mut plane = Plane::new(c64::new(SIO2 * SIO2, 0.0));
    let s = Shape::rounded_rect(
        Point::um(0.0, 0.0),
        Length::um(w),
        Length::um(t),
        Length::um(t / 2.0),
        angle.to_radians(),
    )
    .unwrap();
    plane.draw(s, c64::new(SI * SI, 0.0));
    let x = line(-side, side, 2 * n);
    let cells = section(&plane, &x, &x, how);
    let cs = CrossSection::new(x.clone(), x, cells).unwrap();
    modes(&cs, lam(), 1, None).unwrap()[0].effective_index().re
}

/// Least squares of ln|e| = ln C + p ln h: p and its standard error.
fn slope(h: &[f64], e: &[f64]) -> (f64, f64) {
    let x: Vec<f64> = h.iter().map(|&v| portable::ln(v)).collect();
    let y: Vec<f64> = e.iter().map(|&v| portable::ln(v.abs())).collect();
    let m = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / m, y.iter().sum::<f64>() / m);
    let sxx: f64 = x.iter().map(|v| (v - mx) * (v - mx)).sum();
    let sxy: f64 = x.iter().zip(&y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let p = sxy / sxx;
    let c = my - p * mx;
    let rss: f64 = x
        .iter()
        .zip(&y)
        .map(|(a, b)| portable::powi(b - c - p * a, 2))
        .sum();
    let se = if x.len() > 2 {
        (rss / (m - 2.0) / sxx).sqrt()
    } else {
        f64::NAN
    };
    (p, se)
}

/// n(h) = n₀ + C h^p by least squares (p by a scan, n₀ and C linear): (n₀, p, σ_p), σ_p from
/// the residual and the Jacobian.
fn fit(h: &[f64], n: &[f64]) -> (f64, f64, f64) {
    let lin = |p: f64| -> (f64, f64, f64) {
        let g: Vec<f64> = h.iter().map(|&v| portable::powf(v, p)).collect();
        let m = h.len() as f64;
        let (mg, mn) = (g.iter().sum::<f64>() / m, n.iter().sum::<f64>() / m);
        let sgg: f64 = g.iter().map(|v| (v - mg) * (v - mg)).sum();
        let sgn: f64 = g.iter().zip(n).map(|(a, b)| (a - mg) * (b - mn)).sum();
        let c = sgn / sgg;
        let n0 = mn - c * mg;
        let rss = g
            .iter()
            .zip(n)
            .map(|(a, b)| portable::powi(b - n0 - c * a, 2))
            .sum();
        (n0, c, rss)
    };
    let mut best = (f64::INFINITY, 0.0);
    for k in 1..4000 {
        let p = k as f64 * 0.001;
        let (_, _, r) = lin(p);
        if r < best.0 {
            best = (r, p);
        }
    }
    let p = best.1;
    let (n0, c, rss) = lin(p);
    // Jacobian columns: 1, h^p, C h^p ln h
    let rows: Vec<[f64; 3]> = h
        .iter()
        .map(|&v| {
            let g = portable::powf(v, p);
            [1.0, g, c * g * portable::ln(v)]
        })
        .collect();
    let mut a = [[0.0; 3]; 3];
    for r in &rows {
        for i in 0..3 {
            for j in 0..3 {
                a[i][j] += r[i] * r[j];
            }
        }
    }
    // (JᵀJ)⁻¹'s [2][2] by cofactors
    let det = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
    let inv22 = (a[0][0] * a[1][1] - a[0][1] * a[1][0]) / det;
    let dof = h.len() as f64 - 3.0;
    let sigma = if dof > 0.0 {
        (rss / dof * inv22).sqrt()
    } else {
        f64::NAN
    };
    (n0, p, sigma)
}

fn list(var: &str, default: &str) -> Vec<String> {
    std::env::var(var)
        .unwrap_or_else(|_| default.to_owned())
        .split(',')
        .map(|s| s.trim().to_owned())
        .collect()
}

fn how(name: &str) -> How {
    match name {
        "stair" => How::Stair,
        "mean" => How::Mean,
        "diagonal" => How::Diagonal,
        _ => How::Tensor,
    }
}

#[test]
#[ignore = "a grid study, run by hand (cargo test --release -- --ignored --nocapture)"]
fn oblique_rod_study() {
    // OBLIQUE_HOWS="mean,tensor", OBLIQUE_CELLS="60,120,240" (cells over 1.2 µm),
    // OBLIQUE_QUARTER=1 (behind walls), OBLIQUE_OFFSET="0.0013,0.0007" (the rod off the grid)
    let start = std::time::Instant::now();
    let d = 0.15;
    let exact = crate::stack::checks::he11(SI, SIO2, d / 2.0, lam());
    println!("exact HE11 D {d}: {exact:.9}");
    let hows: Vec<How> = list("OBLIQUE_HOWS", "mean")
        .iter()
        .map(|s| how(s))
        .collect();
    let cells: Vec<usize> = list("OBLIQUE_CELLS", "60,120,240")
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let quarter = std::env::var("OBLIQUE_QUARTER").is_ok_and(|v| v == "1");
    let offset: Vec<f64> = list("OBLIQUE_OFFSET", "0,0")
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let total = hows.len() * cells.len();
    let mut k = 0;
    for &w in &hows {
        let (mut hs, mut es, mut ns) = (Vec::new(), Vec::new(), Vec::new());
        for &n in &cells {
            k += 1;
            let h = 1.2 / n as f64;
            let got = rod(d, n, 1.2, w, quarter, [offset[0], offset[1]]);
            let el = start.elapsed().as_secs_f64();
            println!(
                "step {k}/{total} ({:.0} %, {el:.0} s, ETA {:.0} s): {w:?} h {:.4} nm: neff {got:.9} \
                 error {:+.3e}",
                100.0 * k as f64 / total as f64,
                el / k as f64 * (total - k) as f64,
                h * 1e3,
                got - exact
            );
            hs.push(h);
            es.push(got - exact);
            ns.push(got);
        }
        let (p, se) = slope(&hs, &es);
        let (n0, q, sq) = fit(&hs, &ns);
        println!(
            "{w:?}: order (log fit against the exact) {p:.3} ± {se:.3}; free fit n0 {n0:.7} \
             (off {:+.2e}) p {q:.3} ± {sq:.3}",
            n0 - exact
        );
    }
}

/// A slab `t` µm thick (normal y) whose faces cut cells at the share `frac` of a cell beyond
/// the nodes at ±t/2: its TE (E along x, electric walls on the x edges) or TM index, on cells
/// of side 1.2/n µm over ±1.2 µm in y and four cells in x.
fn slab_cut(t: f64, n: usize, frac: f64, te: bool, how: How) -> f64 {
    let h = 1.2 / n as f64;
    let mut plane = Plane::new(c64::new(SIO2 * SIO2, 0.0));
    plane.draw(
        Shape::rect(
            Point::um(0.0, 0.0),
            Length::um(10.0),
            Length::um(t + 2.0 * frac * h),
        )
        .unwrap(),
        c64::new(SI * SI, 0.0),
    );
    let x = line(-2.0 * h, 2.0 * h, 4);
    let y = line(-1.2, 1.2, 2 * n);
    let cells = section(&plane, &x, &y, how);
    let wall = if te {
        Boundary::ElectricWall
    } else {
        Boundary::MagneticWall
    };
    let cs = CrossSection::new(x, y, cells)
        .unwrap()
        .with_boundaries(Boundaries {
            west: wall,
            east: wall,
            ..Boundaries::default()
        })
        .unwrap();
    modes(&cs, lam(), 1, None).unwrap()[0].effective_index().re
}

#[test]
#[ignore = "a grid study, run by hand (cargo test --release -- --ignored --nocapture)"]
fn cut_slab_study() {
    // a straight interface along the grid but between nodes: the 1D limit of the averaging
    let (t, frac) = (0.15, 0.3);
    let hows: Vec<How> = list("OBLIQUE_HOWS", "mean,tensor")
        .iter()
        .map(|s| how(s))
        .collect();
    for te in [true, false] {
        for &w in &hows {
            let (mut hs, mut es) = (Vec::new(), Vec::new());
            for n in [64usize, 128, 256, 512] {
                let h = 1.2 / n as f64;
                let exact =
                    crate::mode::slab::Slab::new(SIO2, SI, SIO2, Length::um(t + 2.0 * frac * h))
                        .unwrap()
                        .modes(
                            if te {
                                crate::mode::Polarization::Te
                            } else {
                                crate::mode::Polarization::Tm
                            },
                            lam(),
                        )[0]
                    .effective_index();
                let got = slab_cut(t, n, frac, te, w);
                println!(
                    "cut slab te {te} {w:?} h {:.4} nm: {got:.9} exact {exact:.9} error {:+.3e}",
                    h * 1e3,
                    got - exact
                );
                hs.push(h);
                es.push(got - exact);
            }
            let (p, se) = slope(&hs, &es);
            println!("cut slab te {te} {w:?}: order {p:.3} ± {se:.3}");
        }
    }
}

/// The rod's index on Yee's plane (`crate::eme::Plane::from_plane`), a quarter of `n` cells a
/// side behind an electric wall at x = 0 and a magnetic one at y = 0, each value of E's
/// permittivity by `how` ([`yee_index`]).
fn rod_yee(d: f64, n: usize, side: f64, how: Averaging) -> f64 {
    use crate::eme::{PlaneGrid, Wall, Walls};
    let h = side / n as f64;
    let grid = PlaneGrid::new(n, n, (h, h), (0.0, 0.0)).with_walls(Walls {
        west: Wall::Electric,
        south: Wall::Magnetic,
        east: Wall::Electric,
        north: Wall::Electric,
    });
    let mut pl = Plane::new(c64::new(SIO2 * SIO2, 0.0));
    pl.draw(
        Shape::circle(Point::um(0.0, 0.0), Length::um(d / 2.0)).unwrap(),
        c64::new(SI * SI, 0.0),
    );
    yee_index(&pl, grid, how)
}

/// The fundamental's index of `pl` on Yee's plane `grid`, by `how`: the plane's own 8 × 8
/// samples, the mean, the diagonal of Farjadpour et al.'s tensor, or the whole tensor.
fn yee_index(pl: &Plane, grid: crate::eme::PlaneGrid, how: Averaging) -> f64 {
    use crate::eme::{Eme3d, Plane as YeePlane};
    let plane = YeePlane::from_plane(grid, pl, how).unwrap();
    Eme3d::new(lam(), 1).unwrap().modes(&plane).unwrap()[0]
        .effective_index()
        .re
}

#[test]
#[ignore = "a grid study, run by hand (cargo test --release -- --ignored --nocapture)"]
fn yee_rod_study() {
    // OBLIQUE_CELLS (cells over 1.2 µm), YEE_HOWS ("samples,diagonal,mean,tensor")
    let d = 0.15;
    let exact = crate::stack::checks::he11(SI, SIO2, d / 2.0, lam());
    let start = std::time::Instant::now();
    let cells: Vec<usize> = list("OBLIQUE_CELLS", "60,120,240")
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let hows: Vec<Averaging> = list("YEE_HOWS", "samples,diagonal,mean,tensor")
        .iter()
        .map(|s| Averaging::parse(s).unwrap())
        .collect();
    let total = hows.len() * cells.len();
    let mut k = 0;
    for how in hows {
        let (mut hs, mut es) = (Vec::new(), Vec::new());
        for &n in &cells {
            k += 1;
            let h = 1.2 / n as f64;
            let got = rod_yee(d, n, 1.2, how);
            let el = start.elapsed().as_secs_f64();
            println!(
                "step {k}/{total} ({:.0} %, {el:.0} s, ETA {:.0} s): yee {} h {:.4} nm: \
                 {got:.9} error {:+.3e}",
                100.0 * k as f64 / total as f64,
                el / k as f64 * (total - k) as f64,
                how.name(),
                h * 1e3,
                got - exact,
            );
            hs.push(h);
            es.push(got - exact);
        }
        let (p, se) = slope(&hs, &es);
        println!("yee {}: order {p:.3} ± {se:.3}", how.name());
    }
}

#[test]
#[ignore = "a grid study, run by hand (cargo test --release -- --ignored --nocapture)"]
fn oblique_stadium_study() {
    // OBLIQUE_HOWS (mean, tensor, …; yee-samples … yee-tensor on Yee's plane), OBLIQUE_CELLS (cells over
    // 0.6 µm, the window's half), OBLIQUE_ANGLE (degrees)
    let start = std::time::Instant::now();
    let names = list("OBLIQUE_HOWS", "mean");
    let cells: Vec<usize> = list("OBLIQUE_CELLS", "60,120")
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let angle: f64 = list("OBLIQUE_ANGLE", "30")[0].parse().unwrap();
    let side = 0.6;
    let total = names.len() * cells.len();
    let mut k = 0;
    for name in &names {
        let (mut hs, mut ns) = (Vec::new(), Vec::new());
        for &n in &cells {
            k += 1;
            let h = side / n as f64;
            let got = if let Some(y) = name.strip_prefix("yee-") {
                let mut pl = Plane::new(c64::new(SIO2 * SIO2, 0.0));
                pl.draw(
                    Shape::rounded_rect(
                        Point::um(0.0, 0.0),
                        Length::um(0.4),
                        Length::um(0.15),
                        Length::um(0.075),
                        angle.to_radians(),
                    )
                    .unwrap(),
                    c64::new(SI * SI, 0.0),
                );
                let grid = crate::eme::PlaneGrid::new(2 * n, 2 * n, (h, h), (-side, -side));
                yee_index(&pl, grid, Averaging::parse(y).unwrap())
            } else {
                stadium(0.4, 0.15, angle, n, side, how(name))
            };
            let w = name;
            let el = start.elapsed().as_secs_f64();
            println!(
                "step {k}/{total} ({:.0} %, {el:.0} s, ETA {:.0} s): {w} angle {angle} h {:.4} nm: \
                 neff {got:.9}",
                100.0 * k as f64 / total as f64,
                el / k as f64 * (total - k) as f64,
                h * 1e3,
            );
            hs.push(h);
            ns.push(got);
        }
        let (n0, q, sq) = fit(&hs, &ns);
        println!("{name} angle {angle}: free fit n0 {n0:.7} p {q:.3} ± {sq:.3}");
    }
}
