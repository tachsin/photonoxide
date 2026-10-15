//! The averaging's analytic cases: a film off the grid's lines, a disk, and an interface on
//! them.

use std::f64::consts::PI;

use num_complex::Complex64 as c64;

use super::{Averaging, Plane};
use crate::geometry::{Bounds, Point, Shape};
use crate::units::Length;
use crate::validation::{Case, Outcome, Tier};

fn re(v: f64) -> c64 {
    c64::new(v, 0.0)
}

/// The cells of a grid of `n` × `m` cells of `h` µm from (x0, y0).
fn cells(n: usize, m: usize, h: f64, (x0, y0): (f64, f64)) -> Vec<Bounds> {
    let mut out = Vec::with_capacity(n * m);
    for j in 0..m {
        for i in 0..n {
            let (x, y) = (x0 + i as f64 * h, y0 + j as f64 * h);
            out.push(Bounds::new(Point::um(x, y), Point::um(x + h, y + h)));
        }
    }
    out
}

/// A film of ε 12 and 0.2237 µm from y = 0.0413 in ε 2.085, on a column of 50 nm cells (neither
/// face on a grid line): the largest relative difference of Σ ⟨ε⟩ h from ∫ ε dy, of Σ ε_x h
/// (the diagonal along the film) from it, and of Σ h/ε_y (across it) from ∫ dy/ε.
pub(crate) fn film_integrals() -> f64 {
    let (eps_film, eps_bg) = (12.0, 2.085);
    let (a, t) = (0.0413, 0.2237);
    let film =
        Shape::rect(Point::um(0.0, a + t / 2.0), Length::um(10.0), Length::um(t)).expect("a film");
    let mut plane = Plane::new(re(eps_bg));
    plane.draw(film, re(eps_film));
    let h = 0.05;
    let span = 12.0 * h;
    let integral = eps_film * t + eps_bg * (span - t);
    let inverse = t / eps_film + (span - t) / eps_bg;
    let (mut sum, mut along, mut across) = (0.0, 0.0, 0.0);
    for cell in cells(1, 12, h, (-0.025, -0.2)) {
        let s = plane.shares(cell);
        sum += s.mean().re * h;
        along += s.component(0, Averaging::Diagonal).re * h;
        across += h / s.component(1, Averaging::Diagonal).re;
    }
    [
        (sum - integral).abs() / integral,
        (along - integral).abs() / integral,
        (across - inverse).abs() / inverse,
    ]
    .into_iter()
    .fold(0.0, f64::max)
}

/// A disk of ε 12 and radius 0.437 µm in ε 1, off the grid's lines, over 40 nm cells: the
/// relative difference of Σ (⟨ε⟩ − 1) A_cell from (12 − 1) π r².
pub(crate) fn disk_area() -> f64 {
    let (eps_disk, eps_bg, r) = (12.0, 1.0, 0.437);
    let disk = Shape::circle(Point::um(0.0123, -0.0311), Length::um(r)).expect("a disk");
    let mut plane = Plane::new(re(eps_bg));
    plane.draw(disk, re(eps_disk));
    let h = 0.04;
    let excess: f64 = cells(30, 30, h, (-0.6, -0.6))
        .into_iter()
        .map(|c| (plane.shares(c).mean().re - eps_bg) * h * h)
        .sum();
    let exact = (eps_disk - eps_bg) * PI * r * r;
    (excess - exact).abs() / exact
}

/// A 0.4 × 0.3 µm block whose edges are the grid's lines, on 50 nm cells: the largest
/// difference between every component's fill average (mean, diagonal, tensor) and the
/// permittivity at the cell's centre.
pub(crate) fn aligned_difference() -> f64 {
    let block =
        Shape::rect(Point::um(0.2, 0.15), Length::um(0.4), Length::um(0.3)).expect("a block");
    let mut plane = Plane::new(re(2.085));
    plane.draw(block, re(12.1));
    let mut worst = 0.0f64;
    for cell in cells(16, 16, 0.05, (-0.2, -0.2)) {
        let s = plane.shares(cell);
        let c = Point::um(
            0.5 * (cell.min.x.to_um() + cell.max.x.to_um()),
            0.5 * (cell.min.y.to_um() + cell.max.y.to_um()),
        );
        let point = plane.at(c.x.to_um(), c.y.to_um());
        for along in 0..3 {
            for a in [Averaging::Mean, Averaging::Diagonal, Averaging::Tensor] {
                worst = worst.max((s.component(along, a) - point).norm());
            }
        }
    }
    worst
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "averaging/film-integrals",
            title: r"Averaging from the fill: a film ($\varepsilon$ = 12, 223.7 nm) off a 50 nm grid's lines in $\varepsilon$ = 2.085; a column's $\sum \langle\varepsilon\rangle h$ and $\sum \varepsilon_x h$ against $\int \varepsilon\thinspace dy$, and $\sum h/\varepsilon_y$ against $\int dy/\varepsilon$ (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"a cell's shares are the film's exact overlap with it; the mean along the film and the harmonic mean across it (A. Farjadpour et al., Opt. Lett. 31, 2972 (2006), doi:10.1364/OL.31.002972, Eq. 1) add up to the layer's integrals",
            run: || {
                let e = film_integrals();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-13,
                    error: e,
                }
            },
        },
        Case {
            id: "averaging/disk-area",
            title: r"Averaging from the fill: a disk ($\varepsilon$ = 12, $r$ = 0.437 µm) off a 40 nm grid's lines in air; $\sum (\langle\varepsilon\rangle - 1) A$ over the cells against $(12 - 1)\pi r^2$ (relative difference shown)",
            tier: Tier::Analytic,
            source: r"the disk's fill of each cell, its circular segments clipped exactly (geometry::Region::fill), adds up to its area",
            run: || {
                let e = disk_area();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-12,
                    error: e,
                }
            },
        },
        Case {
            id: "averaging/aligned-bits",
            title: r"Averaging from the fill: a 0.4 × 0.3 µm block whose edges are a 50 nm grid's lines; every component's mean, diagonal and tensor average against the permittivity at the cell's centre (largest difference shown)",
            tier: Tier::Analytic,
            source: "an interface on the grid's lines leaves every cell one material, which the averages give to the bit: what point sampling gives",
            run: || {
                let e = aligned_difference();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 0.0,
                    error: e,
                }
            },
        },
    ]
}
