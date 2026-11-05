//! Eigenmode expansion in 3D: its validation cases, shared by the tests and the report.

use num_complex::Complex64 as c64;

use super::{
    Eme, Eme3d, Plane, PlaneGrid, Pml3d, Scattering, Section, Section3d, Slice, Wall, Walls,
};
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

/// Silicon in oxide at 1.55 µm, as the 2D cases.
const CORE: f64 = 3.473;
const CLAD: f64 = 1.444;

fn lam() -> Wavelength {
    Wavelength::from_um_unchecked(1.55)
}

fn re(v: f64) -> c64 {
    c64::new(v, 0.0)
}

/// A grid of `nx` × `ny` cells of `dx` × `dy` µm centred on the origin.
pub(crate) fn centred(nx: usize, ny: usize, dx: f64, dy: f64) -> PlaneGrid {
    PlaneGrid::new(
        nx,
        ny,
        (dx, dy),
        (-0.5 * nx as f64 * dx, -0.5 * ny as f64 * dy),
    )
}

/// A silicon strip `w` × `t` µm centred in oxide on `grid`.
pub(crate) fn strip(grid: PlaneGrid, w: f64, t: f64) -> Plane {
    Plane::new(grid, |x, y| {
        if x.abs() < 0.5 * w && y.abs() < 0.5 * t {
            re(CORE * CORE)
        } else {
            re(CLAD * CLAD)
        }
    })
    .expect("a strip")
}

/// The largest |⟨E_i, H_j⟩ − δ_ij| over a section's modes.
fn orthonormality(modes: &[super::Mode3d]) -> f64 {
    let mut worst: f64 = 0.0;
    for (i, a) in modes.iter().enumerate() {
        for (j, b) in modes.iter().enumerate() {
            let expected = if i == j { 1.0 } else { 0.0 };
            let p = a.product(b).expect("the same grid");
            worst = worst.max((p - expected).norm());
        }
    }
    worst
}

/// A 500 × 220 nm silicon strip in oxide, in a window 2.4 × 1.6 µm on a 40 nm grid with a PML
/// 0.4 µm thick of strength 1 inside every wall: its 60 modes' largest departure from
/// orthonormality in the unconjugated product, guided, radiation and PML modes alike.
pub(crate) fn strip_orthonormality() -> f64 {
    let grid = centred(60, 40, 0.04, 0.04);
    let pml = Pml3d::new(0.4, 0.4, 0.4, 0.4, 1.0).expect("a PML");
    let eme = Eme3d::new(lam(), 60).expect("modes").with_pml(pml);
    orthonormality(&eme.modes(&strip(grid, 0.5, 0.22)).expect("the modes"))
}

/// A step between two uniform media, 1.444 to 3.473, in a closed box 13 × 10 cells of
/// 0.1 × 0.11 µm, every one of its 236 modes: each TE mode (∇ · E = 0) against Fresnel's
/// (β₁ − β₂)/(β₁ + β₂), each TM one (∇ × E = 0) against (ε₁β₂ − ε₂β₁)/(ε₁β₂ + ε₂β₁) for the
/// tangential E, β₂² = β₁² + k₀²(ε₂ − ε₁) from its own β₁ (the box's modes are the same
/// fields in both media); and the rest of S₁₁ against 0. The largest difference.
pub(crate) fn fresnel() -> f64 {
    let grid = PlaneGrid::new(13, 10, (0.1, 0.11), (0.0, 0.0));
    let (e1, e2) = (CLAD * CLAD, CORE * CORE);
    let uniform = |e: f64| Plane::new(grid, |_, _| re(e)).expect("a uniform plane");
    let n = grid.unknowns();
    let eme = Eme3d::new(lam(), n).expect("modes");
    let modes = eme.modes(&uniform(e1)).expect("the modes");
    let s = eme
        .s_matrix(&[
            Section3d::new(uniform(e1), Length::ZERO),
            Section3d::new(uniform(e2), Length::ZERO),
        ])
        .expect("the step's S-matrix");
    let k = lam().wavenumber();
    let mut worst: f64 = 0.0;
    for (i, m) in modes.iter().enumerate() {
        let b1 = m.beta();
        let b2 = {
            let b = (b1 * b1 + k * k * (e2 - e1)).sqrt();
            if b.im.abs() < b.re.abs() && b.re < 0.0 || b.im.abs() >= b.re.abs() && b.im < 0.0 {
                -b
            } else {
                b
            }
        };
        let r = if transverse_electric(m) {
            (b1 - b2) / (b1 + b2)
        } else {
            (e1 * b2 - e2 * b1) / (e1 * b2 + e2 * b1)
        };
        for j in 0..n {
            let expected = if i == j { r } else { c64::new(0.0, 0.0) };
            worst = worst.max((s.s11[i][j] - expected).norm());
        }
    }
    worst
}

/// Whether a uniform box's mode is TE (no divergence) rather than TM (no curl): the larger of
/// its discrete curl and divergence, summed over the cells and nodes.
fn transverse_electric(m: &super::Mode3d) -> bool {
    let g = m.grid();
    let (mut curl, mut div) = (0.0, 0.0);
    for i in 0..g.nx {
        for j in 0..g.ny {
            let c = (m.ey(i + 1, j) - m.ey(i, j)) / g.dx - (m.ex(i, j + 1) - m.ex(i, j)) / g.dy;
            curl += c.norm_sqr();
            if i > 0 && j > 0 {
                let d = (m.ex(i, j) - m.ex(i - 1, j)) / g.dx + (m.ey(i, j) - m.ey(i, j - 1)) / g.dy;
                div += d.norm_sqr();
            }
        }
    }
    curl > div
}

/// The whole S, ports 1 then 2, as a function of (row, column).
fn whole(s: &Scattering) -> (usize, impl Fn(usize, usize) -> c64 + '_) {
    let m = s.first.len();
    (2 * m, move |i: usize, j: usize| match (i < m, j < m) {
        (true, true) => s.s11[i][j],
        (true, false) => s.s12[i][j - m],
        (false, true) => s.s21[i - m][j],
        (false, false) => s.s22[i - m][j - m],
    })
}

/// A lossless silicon strip 220 nm thick widening from 400 to 600 nm in two steps 0.1 µm long
/// (500 nm between), in a closed box 1.2 × 0.8 µm on a 50 nm grid, `modes` modes (728 is every
/// one): S's largest departure from symmetry over all its entries, and from unitarity on the
/// propagating modes.
pub(crate) fn lossless(modes: usize) -> (f64, f64) {
    let grid = centred(24, 16, 0.05, 0.05);
    let sections = [
        Section3d::new(strip(grid, 0.4, 0.2), Length::ZERO),
        Section3d::new(strip(grid, 0.5, 0.2), Length::um(0.1)),
        Section3d::new(strip(grid, 0.6, 0.2), Length::um(0.1)),
        Section3d::new(strip(grid, 0.7, 0.2), Length::ZERO),
    ];
    let s = Eme3d::new(lam(), modes)
        .expect("modes")
        .s_matrix(&sections)
        .expect("the S-matrix");
    let (n, at) = whole(&s);
    let mut symmetry: f64 = 0.0;
    for i in 0..n {
        for j in 0..n {
            symmetry = symmetry.max((at(i, j) - at(j, i)).norm());
        }
    }
    let m = s.first.len();
    let real = |v: &[c64]| -> Vec<usize> {
        (0..m)
            .filter(|&i| v[i].im.abs() < 1e-12 * v[i].re.abs() && v[i].re > 0.0)
            .collect()
    };
    let open: Vec<usize> = real(&s.first)
        .into_iter()
        .chain(real(&s.last).into_iter().map(|i| i + m))
        .collect();
    let mut unitarity: f64 = 0.0;
    for &a in &open {
        for &b in &open {
            let sum: c64 = open.iter().map(|&q| at(q, a).conj() * at(q, b)).sum();
            let expected = if a == b { 1.0 } else { 0.0 };
            unitarity = unitarity.max((sum - expected).norm());
        }
    }
    (symmetry, unitarity)
}

/// The lossless strip taper of [`lossless`], every mode of its plane: (|S₁₁|², |S₂₁|²) of the
/// fundamental modes. With every mode the expansion is the exact solution of the equations
/// discrete across the guide and continuous along it.
pub(crate) fn taper_eme(modes: usize) -> (f64, f64) {
    let grid = centred(24, 16, 0.05, 0.05);
    let sections = [
        Section3d::new(strip(grid, 0.4, 0.2), Length::ZERO),
        Section3d::new(strip(grid, 0.5, 0.2), Length::um(0.1)),
        Section3d::new(strip(grid, 0.6, 0.2), Length::um(0.1)),
        Section3d::new(strip(grid, 0.7, 0.2), Length::ZERO),
    ];
    let s = Eme3d::new(lam(), modes)
        .expect("modes")
        .s_matrix(&sections)
        .expect("the S-matrix");
    (s.s11[0][0].norm_sqr(), s.s21[0][0].norm_sqr())
}

/// The same taper by 3D FDFD on the same plane (24 × 16 cells of 50 nm, its walls perfectly
/// conducting) and `dz` along z: PMLs `pml` µm thick at the two ends, guides `lead` µm long
/// between them and the steps, the ports' fundamental modes 0.1 µm before the first step and
/// 0.1 µm after the last: (|S₁₁|², |S₂₁|²).
pub(crate) fn taper_fdfd(dz: f64, pml: f64, lead: f64) -> (f64, f64) {
    use crate::fdfd::{Axis, Boundaries3d, Edges, Grid3d, Port3d, Side, Solver3d};
    let cells = |l: f64| (l / dz).round() as usize;
    let pml = cells(pml);
    let p1 = pml + cells(lead - 0.1);
    let start = p1 as f64 * dz + 0.1;
    let p2 = p1 + cells(0.4);
    let grid = Grid3d {
        nx: 24,
        ny: 16,
        nz: p2 + cells(lead - 0.1) + pml,
        dx: 0.05,
        dy: 0.05,
        dz,
        x0: -0.6,
        y0: -0.4,
        z0: 0.0,
    };
    let wall = Edges::Pml { low: 0, high: 0 };
    let boundaries = Boundaries3d {
        x: wall,
        y: wall,
        z: Edges::Pml {
            low: pml,
            high: pml,
        },
        ..Boundaries3d::pml(pml)
    };
    let width = |z: f64| {
        if z < start {
            0.4
        } else if z < start + 0.1 {
            0.5
        } else if z < start + 0.2 {
            0.6
        } else {
            0.7
        }
    };
    let solver = Solver3d::new(
        grid,
        lam(),
        |x, y, z| {
            if x.abs() < 0.5 * width(z) && y.abs() < 0.1 {
                re(CORE * CORE)
            } else {
                re(CLAD * CLAD)
            }
        },
        boundaries,
    )
    .expect("the FDFD problem");
    let port = |plane: usize, side: Side| Port3d {
        mode: solver
            .port_modes(Axis::Z, plane, 1)
            .expect("a port mode")
            .remove(0),
        side,
    };
    let ports = [port(p1, Side::Left), port(p2, Side::Right)];
    let s = solver.s_matrix(&ports).expect("the S-matrix");
    (s[0][0].norm_sqr(), s[1][0].norm_sqr())
}

/// The lossless strip taper of [`taper_eme`] on a quarter of its window, x ≥ 0 and y ≥ 0
/// (12 × 8 cells), every mode of the quarter: an electric wall at x = 0 and a magnetic one at
/// y = 0, the planes of symmetry of the TE-like fundamental (E_x even in both). Its S₂₁ of the
/// fundamental against the whole window's (every one of its 728 modes): the difference, and the
/// two fundamentals' β² apart, relative.
pub(crate) fn quarter() -> (f64, f64) {
    let grid = PlaneGrid::new(12, 8, (0.05, 0.05), (0.0, 0.0)).with_walls(Walls {
        west: Wall::Electric,
        east: Wall::Electric,
        south: Wall::Magnetic,
        north: Wall::Electric,
    });
    let n = grid.unknowns();
    let eme = Eme3d::new(lam(), n).expect("modes");
    let sections = [
        Section3d::new(strip(grid, 0.4, 0.2), Length::ZERO),
        Section3d::new(strip(grid, 0.5, 0.2), Length::um(0.1)),
        Section3d::new(strip(grid, 0.6, 0.2), Length::um(0.1)),
        Section3d::new(strip(grid, 0.7, 0.2), Length::ZERO),
    ];
    let quarter = eme.s_matrix(&sections).expect("the quarter's S");
    let whole_grid = centred(24, 16, 0.05, 0.05);
    let whole = Eme3d::new(lam(), whole_grid.unknowns())
        .expect("modes")
        .s_matrix(&[
            Section3d::new(strip(whole_grid, 0.4, 0.2), Length::ZERO),
            Section3d::new(strip(whole_grid, 0.5, 0.2), Length::um(0.1)),
            Section3d::new(strip(whole_grid, 0.6, 0.2), Length::um(0.1)),
            Section3d::new(strip(whole_grid, 0.7, 0.2), Length::ZERO),
        ])
        .expect("the whole window's S");
    let b2 = |n: c64| n * n;
    (
        (quarter.s21[0][0] - whole.s21[0][0]).norm(),
        (b2(quarter.first[0]) - b2(whole.first[0])).norm() / b2(whole.first[0]).norm(),
    )
}

/// Hadley's corner problem `problem` (1 to 4) on the plane: the 1 × 1 µm quarter domain on an
/// `n` × `n` grid, its walls the problem's (G. R. Hadley, J. Lightwave Technol. 20, 1219
/// (2002), Figs. 4–7, as [`crate::mode::vector`]'s check sets them), λ = 1.5 µm: the
/// fundamental's effective index, and Hadley's.
pub(crate) fn hadley(problem: usize, n: usize) -> (f64, f64) {
    use Wall::{Electric, Magnetic};
    let (eps, boxed, expected) = match problem {
        1 => (2.25, true, 1.276_274_04),
        2 => (8.0, true, 2.656_796_92),
        3 => (2.25, false, 1.387_926_425),
        _ => (8.0, false, 2.761_465_320),
    };
    let walls = if boxed {
        Walls {
            west: Electric,
            east: Electric,
            south: Magnetic,
            north: Magnetic,
        }
    } else {
        Walls {
            west: Electric,
            east: Magnetic,
            south: Magnetic,
            north: Electric,
        }
    };
    let h = 1.0 / n as f64;
    let grid = PlaneGrid::new(n, n, (h, h), (0.0, 0.0)).with_walls(walls);
    let plane = Plane::new(grid, |x, y| {
        let inside = if boxed {
            x < 0.5 && y < 0.5
        } else {
            !(x > 0.5 && y > 0.5)
        };
        re(if inside { eps } else { 1.0 })
    })
    .expect("Hadley's plane");
    let lam = Wavelength::from_um_unchecked(1.5);
    let modes = Eme3d::new(lam, 1)
        .expect("a mode")
        .modes(&plane)
        .expect("the fundamental");
    (modes[0].effective_index().re, expected)
}

/// Hadley's four problems on 40, 80 and 160 cells a side, Richardson's limit at each one's
/// observed order: (the largest relative difference of a limit from Hadley's index, the
/// largest relative error of the finest grid, its estimate (f₃ − f₂)/(r − 1), r the ratio of
/// the two changes).
pub(crate) fn hadley_limits() -> (f64, f64) {
    let (mut worst, mut own): (f64, f64) = (0.0, 0.0);
    for problem in 1..=4 {
        let f = [40, 80, 160].map(|n| hadley(problem, n));
        let expected = f[0].1;
        let (d1, d2) = (f[1].0 - f[0].0, f[2].0 - f[1].0);
        let estimate = d2 / (d1 / d2 - 1.0);
        let limit = f[2].0 + estimate;
        worst = worst.max(((limit - expected) / expected).abs());
        own = own.max((estimate / expected).abs());
    }
    (worst, own)
}

/// The strip taper of [`taper_eme`] open: in oxide on a quarter of a window 0.8 µm wide and
/// 0.6 µm tall inside its PML (`thickness` µm more at the east and north walls, `strength` σ
/// graded as (u/d)^`order`), on a 50 nm grid, `modes` modes (all of the quarter's for
/// `None`): (|S₁₁|², |S₂₁|²) of the fundamental, and the modes kept.
pub(crate) fn open_taper(
    thickness: f64,
    (strength, order): (f64, f64),
    modes: Option<usize>,
) -> ((f64, f64), usize) {
    let h = 0.05;
    let cells = |l: f64| (l / h).round() as usize;
    let grid = PlaneGrid::new(
        cells(0.8 + thickness),
        cells(0.6 + thickness),
        (h, h),
        (0.0, 0.0),
    )
    .with_walls(Walls {
        west: Wall::Electric,
        east: Wall::Electric,
        south: Wall::Magnetic,
        north: Wall::Electric,
    });
    let pml = Pml3d::new(0.0, thickness, 0.0, thickness, strength)
        .and_then(|p| p.graded(order))
        .expect("a PML");
    let n = modes.unwrap_or_else(|| grid.unknowns());
    let s = Eme3d::new(lam(), n)
        .expect("modes")
        .with_pml(pml)
        .s_matrix(&[
            Section3d::new(strip(grid, 0.4, 0.2), Length::ZERO),
            Section3d::new(strip(grid, 0.5, 0.2), Length::um(0.1)),
            Section3d::new(strip(grid, 0.6, 0.2), Length::um(0.1)),
            Section3d::new(strip(grid, 0.7, 0.2), Length::ZERO),
        ])
        .expect("the S-matrix");
    ((s.s11[0][0].norm_sqr(), s.s21[0][0].norm_sqr()), n)
}

/// The 400 × 200 nm strip's plane (24 × 16 cells of 50 nm, perfectly conducting walls): its
/// first 6 modes' β² by the expansion against the 3D FDFD ports' β_d² on the same plane (a
/// grid 50 nm along z, β_d = (2/Δz) sin(βΔz/2) from the port's β): the largest relative
/// difference. The two are the same transverse eigenproblem, so to round-off.
pub(crate) fn ports_agree() -> f64 {
    use crate::fdfd::{Axis, Boundaries3d, Edges, Grid3d, Solver3d};
    let grid = centred(24, 16, 0.05, 0.05);
    let modes = Eme3d::new(lam(), 6)
        .expect("modes")
        .modes(&strip(grid, 0.4, 0.2))
        .expect("the modes");
    let dz = 0.05;
    let fdfd = Grid3d {
        nx: 24,
        ny: 16,
        nz: 8,
        dx: 0.05,
        dy: 0.05,
        dz,
        x0: -0.6,
        y0: -0.4,
        z0: 0.0,
    };
    let wall = Edges::Pml { low: 0, high: 0 };
    let boundaries = Boundaries3d {
        x: wall,
        y: wall,
        z: wall,
        ..Boundaries3d::pml(0)
    };
    let solver = Solver3d::new(
        fdfd,
        lam(),
        |x, y, _| {
            if x.abs() < 0.2 && y.abs() < 0.1 {
                re(CORE * CORE)
            } else {
                re(CLAD * CLAD)
            }
        },
        boundaries,
    )
    .expect("the FDFD problem");
    let ports = solver.port_modes(Axis::Z, 3, 6).expect("the ports' modes");
    let mut worst: f64 = 0.0;
    for (m, p) in modes.iter().zip(&ports) {
        let beta_d = 2.0 / dz * (p.beta() * dz / 2.0).sin();
        let (a, b) = (m.beta() * m.beta(), beta_d * beta_d);
        worst = worst.max((a - b).norm() / a.norm());
    }
    worst
}

/// The limit at dz → 0 of values `v` on steps `dz`: the least-squares fit of
/// v = v₀ + a dz² + b dz⁴ (FDFD's second order along z), and the fit's largest residual.
pub(crate) fn z_limit(dz: &[f64], v: &[f64]) -> (f64, f64) {
    // the normal equations of the 3-parameter fit, solved by Cramer's rule
    let basis = |h: f64| [1.0, h * h, h.powi(4)];
    let mut m = [[0.0; 3]; 3];
    let mut r = [0.0; 3];
    for (&h, &y) in dz.iter().zip(v) {
        let b = basis(h);
        for i in 0..3 {
            r[i] += b[i] * y;
            for j in 0..3 {
                m[i][j] += b[i] * b[j];
            }
        }
    }
    let det = |m: &[[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let solve = |k: usize| {
        let mut mk = m;
        for (row, value) in mk.iter_mut().zip(r) {
            row[k] = value;
        }
        det(&mk) / det(&m)
    };
    let c = [solve(0), solve(1), solve(2)];
    let residual = dz
        .iter()
        .zip(v)
        .map(|(&h, &y)| {
            let b = basis(h);
            (y - (c[0] + c[1] * b[1] + c[2] * b[2])).abs()
        })
        .fold(0.0, f64::max);
    (c[0], residual)
}

/// The 2D cases' lossless stepped slab (eme/lossless-reciprocity: 220 to 300 nm through 240,
/// 260 and 280 nm, each 0.1 µm long, in a window 3 µm wide between electric walls) as 3D
/// sections uniform in y, `ny` cells of `h` across between electric walls, on an `h` grid, 30
/// modes: its S.
pub(crate) fn extruded(h: f64, ny: usize) -> Scattering {
    let nx = (3.0 / h).round() as usize;
    let grid = PlaneGrid::new(nx, ny, (h, h), (-1.5, 0.0));
    let slab = |t: f64| {
        Plane::new(grid, |x, _| {
            if x.abs() < 0.5 * t {
                re(CORE * CORE)
            } else {
                re(CLAD * CLAD)
            }
        })
        .expect("a slab")
    };
    let mut sections = vec![Section3d::new(slab(0.22), Length::ZERO)];
    for t in [0.24, 0.26, 0.28] {
        sections.push(Section3d::new(slab(t), Length::um(0.1)));
    }
    sections.push(Section3d::new(slab(0.30), Length::ZERO));
    Eme3d::new(lam(), 30)
        .expect("modes")
        .s_matrix(&sections)
        .expect("the S-matrix")
}

/// The extruded slab 4 cells across in y against 1 cell, on a 10 nm grid: the largest
/// difference over every entry of S. A field uniform in y solves the plane's equations alone
/// (no E_x, no divergence, no H_z change across y), and the modes that vary across y are far
/// below the 30 kept, so the two expansions are the same to round-off.
pub(crate) fn separable() -> f64 {
    let (a, b) = (extruded(0.01, 4), extruded(0.01, 1));
    let (n, at) = whole(&a);
    let (_, bt) = whole(&b);
    let mut worst: f64 = 0.0;
    for i in 0..n {
        for j in 0..n {
            worst = worst.max((at(i, j) - bt(i, j)).norm());
        }
    }
    worst
}

/// The same stepped slab by the 2D expansion, its exact modes: S₁₁ and S₂₁ of the fundamental.
pub(crate) fn stepped_2d() -> (c64, c64) {
    let slab =
        |t: f64| Slice::slab(re(CORE), Length::um(t), re(CLAD), Length::um(3.0)).expect("a slab");
    let mut sections = vec![Section::new(slab(0.22), Length::ZERO)];
    for t in [0.24, 0.26, 0.28] {
        sections.push(Section::new(slab(t), Length::um(0.1)));
    }
    sections.push(Section::new(slab(0.30), Length::ZERO));
    let s = Eme::new(crate::mode::Polarization::Te, lam(), 30)
        .expect("modes")
        .s_matrix(&sections)
        .expect("the S-matrix");
    (s.s11[0][0], s.s21[0][0])
}

/// The extruded slab's S₁₁ of the fundamental on 10, 5 and 2.5 nm grids: (the three, the order
/// of their changes, Richardson's limit, the 2D expansion's).
pub(crate) fn extruded_order() -> ([c64; 3], f64, c64, c64) {
    let s = [0.01, 0.005, 0.0025].map(|h| extruded(h, 1).s11[0][0]);
    let (d1, d2) = (s[1] - s[0], s[2] - s[1]);
    let order = (d1.norm() / d2.norm()).log2();
    let limit = s[2] + d2 / 3.0;
    (s, order, limit, stepped_2d().0)
}

fn zero(measured: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected: 0.0,
        tolerance,
        error: measured,
    }
}

/// `v` to `digits` significant digits.
fn significant(v: f64, digits: i32) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let scale = 10f64.powi(digits - 1 - v.abs().log10().floor() as i32);
    (v * scale).round() / scale
}

/// The extruded slab's limit against the 2D expansion's S₁₁, relative, with the finest grid's
/// own error noted.
fn extruded_limit() -> Outcome {
    let (s, _, limit, exact) = extruded_order();
    crate::validation::note_numerical(significant((s[2] - s[1]).norm() / 3.0 / exact.norm(), 2));
    let expected = significant(exact.norm(), 6);
    Outcome {
        measured: significant(limit.norm(), 6),
        expected,
        tolerance: 3e-4 * expected,
        error: significant((limit - exact).norm(), 2),
    }
}

/// The z steps the cross-check's FDFD runs on: 0.1 µm over 2 to 7.
pub(crate) const STEPS: [usize; 6] = [2, 3, 4, 5, 6, 7];

/// The strip taper's |S₂₁|² by the expansion with every mode against FDFD's limit at dz → 0
/// from [`STEPS`] (PMLs 1 µm thick, leads 1.5 µm long), its fit's largest residual noted.
/// (FDFD's |S₂₁|² on those steps: 0.9962164, 0.9962334, 0.9962365, 0.9962373, 0.9962376 and
/// 0.9962378, second order; with PMLs 0.5 µm thick and leads 1 µm, 1.1e-5 lower, and with 10
/// cells of PML and short leads not monotone at all: the guide's higher and evanescent modes
/// came back from a PML too thin for them.)
fn taper_cross_code() -> Outcome {
    let dz: Vec<f64> = STEPS.iter().map(|&m| 0.1 / m as f64).collect();
    let fdfd: Vec<f64> = dz.iter().map(|&h| taper_fdfd(h, 1.0, 1.5).1).collect();
    let (limit, residual) = z_limit(&dz, &fdfd);
    crate::validation::note_numerical(significant(residual, 2));
    let eme = taper_eme(728).1;
    Outcome {
        measured: significant(eme, 6),
        expected: significant(limit, 6),
        tolerance: TAPER_TOLERANCE,
        error: significant((eme - limit).abs(), 2),
    }
}

/// [`taper_cross_code`]'s tolerance, absolute on |S₂₁|².
pub(crate) const TAPER_TOLERANCE: f64 = 4e-5;

/// Every 3D eigenmode-expansion case, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "eme/3d-orthonormal",
            title: r"The modes of a 3D section: a 500 × 220 nm silicon strip in oxide (3.473 in 1.444, 1.55 µm) in a window 2.4 × 1.6 µm on a 40 nm grid, a PML 0.4 µm thick of strength 1 inside every wall, 60 modes: their largest departure from orthonormality in the unconjugated product, $\lvert \langle E_i, H_j \rangle - \delta_{ij} \rvert$ (shown)",
            tier: Tier::Analytic,
            source: r"the scheme's own Lorentz reciprocity: $VC$ and $VCA$ are symmetric on Yee's plane, so $(\beta_i^2 - \beta_j^2) E_i^T V C E_j = 0$ for any two modes, lossy or not",
            run: || zero(strip_orthonormality(), 1e-10),
        },
        Case {
            id: "eme/3d-ports",
            title: r"The modes of a 3D section against the 3D FDFD ports' on the same plane: a 400 × 200 nm silicon strip in oxide in a closed box 1.2 × 0.8 µm on a 50 nm grid, its 6 propagating modes' $\beta^2$ against the ports' $\beta_d^2$ (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"the same transverse eigenproblem: the ports' operator with $\beta_d^2 = ((2/\Delta z)\sin(\beta\Delta z/2))^2$ in place of $\beta^2$, its $\nabla_t \varepsilon_z^{-1} \nabla_t \cdot \varepsilon$ term included",
            run: || zero(ports_agree(), 1e-10),
        },
        Case {
            id: "eme/3d-hadley",
            title: r"The modes of a 3D section against a published table: Hadley's four corner problems (a dielectric box or an impinged corner of ε = 2.25 or 8 in air, the 1 × 1 µm quarter domain between electric and magnetic walls, λ = 1.5 µm), the fundamental's index on 40, 80 and 160 cells a side, Richardson's limit at its observed order: the largest relative difference from Hadley's (shown)",
            tier: Tier::Published,
            source: r"G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371, Figs. 4–7: 1.27627404, 2.65679692, 1.387926425 and 2.761465320; our observed orders 1.9, 2.0, 1.7 and 1.3 (convex corners slow it)",
            run: || {
                let (worst, own) = hadley_limits();
                crate::validation::note_numerical(significant(own, 2));
                zero(significant(worst, 2), 1.2e-5)
            },
        },
        Case {
            id: "eme/3d-symmetry",
            title: r"Mirror walls: the 3D strip taper of eme/3d-taper-fdfd on a quarter of its window (an electric wall at x = 0, a magnetic one at y = 0, the TE-like fundamental's planes of symmetry), every mode of the quarter, against the whole window with every one of its 728 modes: $\lvert S_{21} \rvert$'s difference (shown)",
            tier: Tier::Analytic,
            source: r"the mirror image: with the field beyond a wall its image (tangential E even across a magnetic wall, odd across an electric one) the quarter's equations are the whole window's restricted to that symmetry, each value on a wall weighted by half",
            run: || zero(quarter().0, 1e-10),
        },
        Case {
            id: "eme/3d-fresnel",
            title: r"A 3D step between two uniform media, 1.444 to 3.473, in a closed box 1.3 × 1.1 µm (13 × 10 cells), every one of its 236 modes: each against Fresnel's coefficient at its own angle (largest difference of $S_{11}$ from Fresnel's diagonal shown)",
            tier: Tier::Analytic,
            source: r"Fresnel's $(\beta_1 - \beta_2)/(\beta_1 + \beta_2)$ for the TE modes ($\nabla \cdot E = 0$) and $(\varepsilon_1\beta_2 - \varepsilon_2\beta_1)/(\varepsilon_1\beta_2 + \varepsilon_2\beta_1)$ for the TM ones ($\nabla \times E = 0$), $\beta_2^2 = \beta_1^2 + k_0^2(\varepsilon_2 - \varepsilon_1)$: the box's modes are the same fields in both media; no coupling between modes",
            run: || zero(fresnel(), 1e-10),
        },
        Case {
            id: "eme/3d-lossless-reciprocity",
            title: r"A lossless 3D taper: a silicon strip 200 nm thick widening from 400 to 700 nm in two steps 0.1 µm long, in a closed box 1.2 × 0.8 µm on a 50 nm grid, every one of its 728 modes: the S-matrix's largest departure from symmetry, $\lvert S - S^T \rvert$ (shown)",
            tier: Tier::Analytic,
            source: r"Lorentz reciprocity: the interfaces' S-matrices are symmetric for any number of modes, and the star product keeps it",
            run: || zero(lossless(728).0, 1e-10),
        },
        Case {
            id: "eme/3d-lossless-power",
            title: r"The same taper: the propagating modes' block of $S$ against unitarity, $\lvert S_p^\dagger S_p - 1 \rvert$, with every mode (largest entry shown)",
            tier: Tier::Analytic,
            source: r"power conservation: with every mode of the plane the continuity of the tangential fields holds exactly at each interface",
            run: || zero(lossless(728).1, 1e-10),
        },
        Case {
            id: "eme/3d-extruded-separable",
            title: r"The 2D cases' lossless stepped slab (220 to 300 nm through 240, 260 and 280 nm, each 0.1 µm long, a window 3 µm wide, TE, 30 modes) as 3D sections uniform in y, 4 cells across against 1, on a 10 nm grid: the largest difference over every entry of $S$ (shown)",
            tier: Tier::Analytic,
            source: r"separation: a field uniform in y solves the plane's equations alone (no $E_x$, no divergence, no change of $H_z$ across y), and the modes that vary across y lie far below the 30 kept",
            run: || zero(separable(), 1e-10),
        },
        Case {
            id: "eme/3d-extruded-order",
            title: r"The same extruded slab on 10, 5 and 2.5 nm grids: the order of convergence of the fundamental's $S_{11}$ (shown)",
            tier: Tier::Analytic,
            source: r"second order: Yee's differences, the slab's faces on nodes",
            run: || {
                let order = (100.0 * extruded_order().1).round() / 100.0;
                Outcome {
                    measured: order,
                    expected: 2.0,
                    tolerance: 0.1,
                    error: (order - 2.0).abs(),
                }
            },
        },
        Case {
            id: "eme/3d-extruded-slab",
            title: r"The same: Richardson's limit of the three grids' $S_{11}$ against the 2D expansion's on the exact slab modes, $\lvert S_{11} \rvert$ (shown)",
            tier: Tier::Analytic,
            source: r"the 2D expansion (eme/lossless-reciprocity's structure, 30 modes), whose modes are the exact slab's: what the 3D expansion of a cross-section uniform in y converges to",
            run: extruded_limit,
        },
        Case {
            id: "eme/3d-taper-fdfd",
            title: r"A 3D strip taper: silicon 200 nm thick widening from 400 to 700 nm in two steps 0.1 µm long (3.473 in 1.444, 1.55 µm), in a closed box 1.2 × 0.8 µm on a 50 nm grid: its transmission $\lvert S_{21} \rvert^2$ by eigenmode expansion with every one of the plane's 728 modes (shown) against 3D FDFD",
            tier: Tier::CrossCode,
            source: r"photonoxide's 3D FDFD on the same plane, its ports' fundamental modes, along z at 0.1 µm over 2 to 7, PMLs 1 µm thick and leads 1.5 µm long; its limit at $\Delta z \to 0$ by a least-squares fit of $v_0 + a\Delta z^2 + b\Delta z^4$",
            run: taper_cross_code,
        },
    ]
}
