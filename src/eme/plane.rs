//! A 3D section's cross-section on Yee's grid, and its modes (the module's 3D points).
//!
//! The unknowns are the transverse E on a plane of Yee's grid: E_x at (i + ½, j) and E_y at
//! (i, j + ½), between perfectly conducting walls on the window's four edges (the tangential E
//! is zero on them). A mode going as e^(iβz) has E_z = i ∇_t · (ε E_t) / (β ε_z) from the
//! continuity equation, and what is left of the curl-curl equation is an eigenproblem for E_t,
//!
//! A E_t = (C + D) E_t = β² E_t,  C = −∇_t × ∇_t × + k₀² ε,  D = ∇_t ε_z⁻¹ ∇_t · ε,
//!
//! every difference Yee's (a stretched coordinate's inside a PML): the 3D FDFD's ports' own
//! operator ([`crate::fdfd::PortMode3d`]) with β in place of the grid's β_d, the guide
//! continuous along z. Faraday's law then gives the transverse H from E_t alone,
//! (H_y, −H_x) = C E_t / (k₀β), at E_x's and E_y's own places.
//!
//! **Orthogonality, exactly.** With V the product of the stretches at each value of E, V C is
//! symmetric (the curl-curl's), and since the curl of a gradient is zero on Yee's grid, C D =
//! k₀² ε G ε_z⁻¹ (∇ ·) ε, with V G = −(W ∇ ·)ᵀ (W the stretches at the nodes): V C A is
//! symmetric too. So for two modes, (β_i² − β_j²) E_iᵀ V C E_j = 0: the unconjugated product
//! ⟨E_i, H_j⟩ = Σ V E_i · C E_j / (k₀β_j) Δx Δy, the discrete ∫ (E_i × H_j) · ẑ dx dy, is zero
//! between two modes of a section to round-off, lossy or not. Each mode is normalized to
//! ⟨E_i, H_i⟩ = 1 (Bienstman and Baets's Eq. 4 on the grid). Modes of the same β (a symmetric
//! guide's pairs, or a uniform medium's TE and TM of the same order) are given the basis
//! orthogonal in both that product and Σ V E_i · E_j: unique when the two forms' ratios differ
//! within the group, so the basis doesn't depend on the eigensolver's choice.

use std::sync::Arc;

use faer::diag::Diag;
use faer::dyn_stack::{MemBuffer, MemStack};
use faer::linalg::evd::{self, ComputeEigenvectors};
use faer::{Mat, Par};
use num_complex::Complex64 as c64;

use super::Wall;
use crate::portable;
use crate::{Error, Result};

/// The walls on a plane's four edges: perfect electric conductors (the tangential E zero on
/// them) or perfect magnetic ones (the tangential H zero), each a plane of symmetry when the
/// guide is mirror-symmetric about it, so that half or a quarter of the cross-section holds the
/// modes of one symmetry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Walls {
    /// At the smallest x.
    pub west: Wall,
    /// At the largest x.
    pub east: Wall,
    /// At the smallest y.
    pub south: Wall,
    /// At the largest y.
    pub north: Wall,
}

impl Default for Walls {
    /// Electric walls on all four edges.
    fn default() -> Walls {
        Walls {
            west: Wall::Electric,
            east: Wall::Electric,
            south: Wall::Electric,
            north: Wall::Electric,
        }
    }
}

/// A uniform grid over a cross-section: `nx` × `ny` cells of `dx` × `dy` µm, its lowest corner
/// at (`x0`, `y0`), and its four `walls`.
///
/// E_x of index (i, j) sits at (x0 + (i + ½) dx, y0 + j dy) and E_y of index (i, j) at
/// (x0 + i dx, y0 + (j + ½) dy), Yee's places on a plane of nodes (as [`crate::fdfd::Grid3d`]
/// places them). On an electric wall the tangential E is zero, so the unknowns are E_x for
/// 0 < j < ny and E_y for 0 < i < nx between four of them; a magnetic wall's tangential E is
/// an unknown too, and the field beyond it its mirror image (the tangential E even, the normal
/// E and H_z odd), the values on it weighted by half in the product.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaneGrid {
    /// Cells along x.
    pub nx: usize,
    /// Cells along y.
    pub ny: usize,
    /// Cell width along x, µm.
    pub dx: f64,
    /// Along y, µm.
    pub dy: f64,
    /// The window's low x, µm.
    pub x0: f64,
    /// Its low y, µm.
    pub y0: f64,
    /// Its walls.
    pub walls: Walls,
}

impl PlaneGrid {
    /// `nx` × `ny` cells of `dx` × `dy` µm from (`x0`, `y0`), between electric walls.
    pub fn new(nx: usize, ny: usize, (dx, dy): (f64, f64), (x0, y0): (f64, f64)) -> PlaneGrid {
        PlaneGrid {
            nx,
            ny,
            dx,
            dy,
            x0,
            y0,
            walls: Walls::default(),
        }
    }

    /// The same grid with `walls`.
    pub fn with_walls(self, walls: Walls) -> PlaneGrid {
        PlaneGrid { walls, ..self }
    }

    /// The range of j where E_x is unknown, and of i where E_y is: inside the electric walls,
    /// on the magnetic ones.
    fn spans(&self) -> ((usize, usize), (usize, usize)) {
        let magnetic = |w: Wall| w == Wall::Magnetic;
        let w = self.walls;
        let j = (
            usize::from(!magnetic(w.south)),
            if magnetic(w.north) {
                self.ny
            } else {
                self.ny.saturating_sub(1)
            },
        );
        let i = (
            usize::from(!magnetic(w.west)),
            if magnetic(w.east) {
                self.nx
            } else {
                self.nx.saturating_sub(1)
            },
        );
        (j, i)
    }

    /// The number of E_x unknowns: they come first.
    pub(crate) fn ex_count(&self) -> usize {
        let ((j0, j1), _) = self.spans();
        if j1 < j0 { 0 } else { self.nx * (j1 + 1 - j0) }
    }

    /// The number of unknowns: E_x and E_y, but for those on electric walls.
    pub fn unknowns(&self) -> usize {
        let (_, (i0, i1)) = self.spans();
        let ey = if i1 < i0 { 0 } else { self.ny * (i1 + 1 - i0) };
        self.ex_count() + ey
    }

    /// Whether node (i, j)'s E_z (and the potential the continuity equation gives) is free:
    /// not on an electric wall.
    pub(crate) fn node_free(&self, i: usize, j: usize) -> bool {
        let w = self.walls;
        let off = |at: usize, end: usize, low: Wall, high: Wall| {
            (at > 0 || low == Wall::Magnetic) && (at < end || high == Wall::Magnetic)
        };
        i <= self.nx
            && j <= self.ny
            && off(i, self.nx, w.west, w.east)
            && off(j, self.ny, w.south, w.north)
    }

    /// The share of E_x's (i, j) inside the window: ½ on a magnetic wall, else 1.
    pub(crate) fn ex_share(&self, j: usize) -> f64 {
        if j == 0 || j == self.ny { 0.5 } else { 1.0 }
    }

    /// The share of E_y's (i, j) inside the window.
    pub(crate) fn ey_share(&self, i: usize) -> f64 {
        if i == 0 || i == self.nx { 0.5 } else { 1.0 }
    }

    /// Node `i` along x, µm.
    pub fn x(&self, i: usize) -> f64 {
        self.x0 + i as f64 * self.dx
    }

    /// Node `j` along y, µm.
    pub fn y(&self, j: usize) -> f64 {
        self.y0 + j as f64 * self.dy
    }

    fn check(&self) -> Result<()> {
        let ok = |v: f64| v.is_finite() && v > 0.0;
        if self.nx == 0 || self.ny == 0 || !ok(self.dx) || !ok(self.dy) {
            return Err(Error::invalid(
                "eme plane",
                format!(
                    "needs cells and positive steps, got {} x {} cells of {} x {} um",
                    self.nx, self.ny, self.dx, self.dy
                ),
            ));
        }
        if !(self.x0.is_finite() && self.y0.is_finite()) {
            return Err(Error::invalid("eme plane", "its corner must be finite"));
        }
        if self.unknowns() == 0 {
            return Err(Error::invalid(
                "eme plane",
                "a window of one cell holds no field between its walls",
            ));
        }
        Ok(())
    }

    /// E_x's unknown at (i, j), `None` on an electric wall or outside.
    pub(crate) fn ex(&self, i: usize, j: usize) -> Option<usize> {
        let ((j0, j1), _) = self.spans();
        (i < self.nx && j >= j0 && j <= j1).then(|| (j - j0) * self.nx + i)
    }

    /// E_y's unknown at (i, j), `None` on an electric wall or outside.
    pub(crate) fn ey(&self, i: usize, j: usize) -> Option<usize> {
        let (_, (i0, i1)) = self.spans();
        let width = (i1 + 1).saturating_sub(i0);
        (j < self.ny && i >= i0 && i <= i1).then(|| self.ex_count() + j * width + (i - i0))
    }
}

/// The permittivity of a cell of the dual grid centred on `centre`, `h` wide, for a field along
/// x (`along_x`) or y, from `samples`² points: harmonic along the field, arithmetic across it,
/// as [`crate::fdfd::Solver3d`] averages it. `None` for E_z's, arithmetic both ways.
fn averaged(
    eps: &impl Fn(f64, f64) -> c64,
    centre: [f64; 2],
    h: [f64; 2],
    along_x: Option<bool>,
    samples: usize,
) -> c64 {
    let n = samples as f64;
    let offset = |s: usize| (s as f64 + 0.5) / n - 0.5;
    let point = |a: usize, b: usize| [centre[0] + offset(a) * h[0], centre[1] + offset(b) * h[1]];
    let mut sum = c64::new(0.0, 0.0);
    for across in 0..samples {
        match along_x {
            Some(x) => {
                let mut inverse = c64::new(0.0, 0.0);
                for along in 0..samples {
                    let p = if x {
                        point(along, across)
                    } else {
                        point(across, along)
                    };
                    inverse += 1.0 / eps(p[0], p[1]);
                }
                sum += n / inverse;
            }
            None => {
                for b in 0..samples {
                    let p = point(across, b);
                    sum += eps(p[0], p[1]) / n;
                }
            }
        }
    }
    sum / n
}

/// The points per axis [`Plane::new`] averages each value's permittivity from, as 3D FDFD.
pub(crate) const SAMPLES: usize = 8;

/// A cross-section uniform along z on a [`PlaneGrid`]: the permittivity each value of E sees.
#[derive(Clone, Debug, PartialEq)]
pub struct Plane {
    grid: PlaneGrid,
    /// ε at E_x's places, (ny + 1) rows of nx, i fastest.
    ex: Vec<c64>,
    /// At E_y's, ny rows of nx + 1.
    ey: Vec<c64>,
    /// At E_z's, the nodes, ny + 1 rows of nx + 1.
    ez: Vec<c64>,
}

impl Plane {
    /// The cross-section whose relative permittivity at (x, y) (µm) is `eps(x, y)`, on `grid`:
    /// each value of E sees it averaged over its cell of the dual grid, harmonically along the
    /// component and arithmetically across it, from 8 × 8 points, as 3D FDFD averages it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a grid without cells, positive steps or unknowns, or a
    /// permittivity that isn't finite.
    pub fn new(grid: PlaneGrid, eps: impl Fn(f64, f64) -> c64) -> Result<Plane> {
        grid.check()?;
        let (nx, ny, h) = (grid.nx, grid.ny, [grid.dx, grid.dy]);
        // a cell on a wall reaches past it: there the structure is its mirror image
        let reflect = |v: f64, low: f64, high: f64| {
            if v < low {
                2.0 * low - v
            } else if v > high {
                2.0 * high - v
            } else {
                v
            }
        };
        let (x1, y1) = (grid.x(nx), grid.y(ny));
        let eps = |x: f64, y: f64| eps(reflect(x, grid.x0, x1), reflect(y, grid.y0, y1));
        let ex = (0..=ny)
            .flat_map(|j| (0..nx).map(move |i| (i, j)))
            .map(|(i, j)| {
                let c = [grid.x(i) + 0.5 * grid.dx, grid.y(j)];
                averaged(&eps, c, h, Some(true), SAMPLES)
            })
            .collect();
        let ey = (0..ny)
            .flat_map(|j| (0..=nx).map(move |i| (i, j)))
            .map(|(i, j)| {
                let c = [grid.x(i), grid.y(j) + 0.5 * grid.dy];
                averaged(&eps, c, h, Some(false), SAMPLES)
            })
            .collect();
        let ez = (0..=ny)
            .flat_map(|j| (0..=nx).map(move |i| (i, j)))
            .map(|(i, j)| averaged(&eps, [grid.x(i), grid.y(j)], h, None, SAMPLES))
            .collect();
        Plane::from_values(grid, ex, ey, ez)
    }

    /// The cross-section from the permittivity at each place, as [`Plane::new`] lays them out.
    pub(crate) fn from_values(
        grid: PlaneGrid,
        ex: Vec<c64>,
        ey: Vec<c64>,
        ez: Vec<c64>,
    ) -> Result<Plane> {
        grid.check()?;
        let (nx, ny) = (grid.nx, grid.ny);
        if ex.len() != nx * (ny + 1) || ey.len() != (nx + 1) * ny || ez.len() != (nx + 1) * (ny + 1)
        {
            return Err(Error::invalid("eme plane", "a permittivity per value of E"));
        }
        let finite =
            |v: &c64| v.re.is_finite() && v.im.is_finite() && portable::complex::abs(*v) > 0.0;
        if !ex.iter().chain(&ey).chain(&ez).all(finite) {
            return Err(Error::invalid(
                "eme plane",
                "every permittivity must be finite and not zero",
            ));
        }
        Ok(Plane { grid, ex, ey, ez })
    }

    /// The grid.
    pub fn grid(&self) -> PlaneGrid {
        self.grid
    }

    /// ε at E_x's place (i, j).
    pub fn eps_x(&self, i: usize, j: usize) -> c64 {
        self.ex[j * self.grid.nx + i]
    }

    /// ε at E_y's place (i, j).
    pub fn eps_y(&self, i: usize, j: usize) -> c64 {
        self.ey[j * (self.grid.nx + 1) + i]
    }

    /// ε at E_z's place, node (i, j).
    pub fn eps_z(&self, i: usize, j: usize) -> c64 {
        self.ez[j * (self.grid.nx + 1) + i]
    }

    /// The highest real part of the permittivity anywhere.
    pub(crate) fn highest(&self) -> f64 {
        self.ex
            .iter()
            .chain(&self.ey)
            .map(|e| e.re)
            .fold(1.0, f64::max)
    }
}

/// Perfectly matched layers inside a 3D section's walls: the last `west`, `east`, `south` and
/// `north` µm of the window (0 for none) stretch the coordinate across them by
/// s = 1 + iσ (u/d)^m at depth u into a layer d thick, σ its `strength` and m its `order`
/// (Chew and Weedon's stretched coordinates, as the 2D expansion's and FDFD's PMLs). m = 0 is
/// a layer of complex thickness d(1 + iσ), the 2D expansion's; FDFD's graded layers are
/// m = 3 with σ = (m + 1) ln(1/R) / (2k₀d).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Pml3d {
    /// The layer at the smallest x, µm.
    pub west: f64,
    /// At the largest x.
    pub east: f64,
    /// At the smallest y.
    pub south: f64,
    /// At the largest y.
    pub north: f64,
    /// σ at the wall.
    pub strength: f64,
    /// The grading's order m.
    pub order: f64,
}

impl Pml3d {
    /// Layers of complex thickness d(1 + iσ): uniform, `strength` σ, the 2D expansion's PML.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a thickness that is negative or not finite, none at all, or
    /// a strength that isn't positive and finite.
    pub fn new(west: f64, east: f64, south: f64, north: f64, strength: f64) -> Result<Pml3d> {
        let bad = |v: f64| !(v.is_finite() && v >= 0.0);
        if [west, east, south, north].into_iter().any(bad) || west + east + south + north == 0.0 {
            return Err(Error::invalid(
                "eme pml",
                "thicknesses must be finite, not negative and not all zero",
            ));
        }
        if !(strength.is_finite() && strength > 0.0) {
            return Err(Error::invalid(
                "eme pml",
                format!("needs a positive strength, got {strength}"),
            ));
        }
        Ok(Pml3d {
            west,
            east,
            south,
            north,
            strength,
            order: 0.0,
        })
    }

    /// The same graded as (u/d)^`order` from 0 at its inner face to σ at the wall.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an order that is negative or not finite.
    pub fn graded(self, order: f64) -> Result<Pml3d> {
        if !(order.is_finite() && order >= 0.0) {
            return Err(Error::invalid(
                "eme pml",
                format!("needs an order >= 0, got {order}"),
            ));
        }
        Ok(Pml3d { order, ..self })
    }

    /// The stretch at `pos` along an axis spanning `span`, with layers `low` and `high` thick.
    fn stretch(&self, pos: f64, span: (f64, f64), (low, high): (f64, f64)) -> c64 {
        let profile = |depth: f64, d: f64| {
            c64::new(
                1.0,
                self.strength * portable::powf((depth / d).clamp(0.0, 1.0), self.order),
            )
        };
        if low > 0.0 && pos < span.0 + low {
            profile(span.0 + low - pos, low)
        } else if high > 0.0 && pos > span.1 - high {
            profile(pos - (span.1 - high), high)
        } else {
            c64::new(1.0, 0.0)
        }
    }
}

/// The stretches along each axis at its nodes and halfway after each.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Stretch {
    pub(crate) x_node: Vec<c64>,
    pub(crate) x_half: Vec<c64>,
    pub(crate) y_node: Vec<c64>,
    pub(crate) y_half: Vec<c64>,
}

impl Stretch {
    /// The stretches of `pml` (none: 1 everywhere) on `grid`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for layers that meet or fill an axis.
    pub(crate) fn new(grid: &PlaneGrid, pml: Option<&Pml3d>) -> Result<Stretch> {
        let span_x = (grid.x(0), grid.x(grid.nx));
        let span_y = (grid.y(0), grid.y(grid.ny));
        let along = |n: usize, h: f64, span: (f64, f64), layers: (f64, f64), half: bool| {
            let count = if half { n } else { n + 1 };
            (0..count)
                .map(|m| {
                    let pos = span.0 + (m as f64 + if half { 0.5 } else { 0.0 }) * h;
                    pml.map_or(c64::new(1.0, 0.0), |p| p.stretch(pos, span, layers))
                })
                .collect::<Vec<c64>>()
        };
        let (lx, ly) = pml.map_or(((0.0, 0.0), (0.0, 0.0)), |p| {
            ((p.west, p.east), (p.south, p.north))
        });
        if lx.0 + lx.1 >= span_x.1 - span_x.0 || ly.0 + ly.1 >= span_y.1 - span_y.0 {
            return Err(Error::invalid(
                "eme pml",
                "the layers are thicker than the window",
            ));
        }
        let w = grid.walls;
        for (name, thickness, wall) in [
            ("west", lx.0, w.west),
            ("east", lx.1, w.east),
            ("south", ly.0, w.south),
            ("north", ly.1, w.north),
        ] {
            if thickness > 0.0 && wall == Wall::Magnetic {
                return Err(Error::invalid(
                    "eme pml",
                    format!("the {name} wall is a plane of symmetry: a PML there would break it"),
                ));
            }
        }
        Ok(Stretch {
            x_node: along(grid.nx, grid.dx, span_x, lx, false),
            x_half: along(grid.nx, grid.dx, span_x, lx, true),
            y_node: along(grid.ny, grid.dy, span_y, ly, false),
            y_half: along(grid.ny, grid.dy, span_y, ly, true),
        })
    }

    /// The weight of each unknown in the product: V Δx Δy, V the stretches' product at its place.
    pub(crate) fn weights(&self, grid: &PlaneGrid) -> Vec<c64> {
        let area = grid.dx * grid.dy;
        let mut w = vec![c64::new(0.0, 0.0); grid.unknowns()];
        for j in 0..=grid.ny {
            for i in 0..grid.nx {
                if let Some(r) = grid.ex(i, j) {
                    w[r] = self.x_half[i] * self.y_node[j] * area * grid.ex_share(j);
                }
            }
        }
        for j in 0..grid.ny {
            for i in 0..=grid.nx {
                if let Some(r) = grid.ey(i, j) {
                    w[r] = self.x_node[i] * self.y_half[j] * area * grid.ey_share(i);
                }
            }
        }
        w
    }
}

/// A sparse matrix over the unknowns as rows of (column, value), each column once.
pub(crate) type Rows = Vec<Vec<(usize, c64)>>;

/// Adds `value` × `terms` to `row`.
fn add(row: &mut Vec<(usize, c64)>, value: c64, terms: &[(Option<usize>, c64)]) {
    for &(col, w) in terms {
        if let Some(col) = col {
            row.push((col, value * w));
        }
    }
}

/// Each row's entries sorted by column, each column's summed.
fn merged(mut rows: Rows) -> Rows {
    for row in &mut rows {
        row.sort_by_key(|&(c, _)| c);
        let mut out: Vec<(usize, c64)> = Vec::with_capacity(row.len());
        for &(c, v) in row.iter() {
            match out.last_mut() {
                Some((last, acc)) if *last == c => *acc += v,
                _ => out.push((c, v)),
            }
        }
        *row = out;
    }
    rows
}

/// The operators C = −∇_t × ∇_t × + k₀² ε and D = ∇_t ε_z⁻¹ ∇_t · ε of `plane` (see the
/// module's docs), at wavenumber `k`.
pub(crate) fn operators(plane: &Plane, stretch: &Stretch, k: f64) -> (Rows, Rows) {
    let g = plane.grid;
    let (nx, ny, dx, dy) = (g.nx, g.ny, g.dx, g.dy);
    let n = g.unknowns();
    let s = stretch;
    // H_z of cell (i, j), (i + ½, j + ½): ∂_x E_y − ∂_y E_x, in the unknowns; a cell beyond a
    // wall is the mirror image of the one inside it, H_z odd across a magnetic wall (only a
    // magnetic wall's rows reach beyond it)
    let hz = |i: isize, j: isize| -> Vec<(Option<usize>, c64)> {
        let (mut sign, mut a, mut b) = (1.0, i, j);
        if a < 0 || a >= nx as isize {
            sign = -sign;
            a = if a < 0 { 0 } else { nx as isize - 1 };
        }
        if b < 0 || b >= ny as isize {
            sign = -sign;
            b = if b < 0 { 0 } else { ny as isize - 1 };
        }
        let (i, j) = (a as usize, b as usize);
        let wx = sign / (s.x_half[i] * dx);
        let wy = sign / (s.y_half[j] * dy);
        vec![
            (g.ey(i + 1, j), wx),
            (g.ey(i, j), -wx),
            (g.ex(i, j + 1), -wy),
            (g.ex(i, j), wy),
        ]
    };
    // ε_z⁻¹ ∇_t · (ε E_t) at node (i, j), off the electric walls; on a magnetic wall the normal
    // E beyond it is minus its image, so its difference doubles
    let potential = |i: usize, j: usize| -> Vec<(Option<usize>, c64)> {
        if !g.node_free(i, j) {
            return Vec::new();
        }
        let inverse = 1.0 / plane.eps_z(i, j);
        let wx = inverse / (s.x_node[i] * dx);
        let wy = inverse / (s.y_node[j] * dy);
        let mut out = Vec::with_capacity(4);
        if i < nx {
            let f = if i == 0 { 2.0 } else { 1.0 };
            out.push((g.ex(i, j), f * wx * plane.eps_x(i, j)));
        }
        if i > 0 {
            let f = if i == nx { 2.0 } else { 1.0 };
            out.push((g.ex(i - 1, j), -f * wx * plane.eps_x(i - 1, j)));
        }
        if j < ny {
            let f = if j == 0 { 2.0 } else { 1.0 };
            out.push((g.ey(i, j), f * wy * plane.eps_y(i, j)));
        }
        if j > 0 {
            let f = if j == ny { 2.0 } else { 1.0 };
            out.push((g.ey(i, j - 1), -f * wy * plane.eps_y(i, j - 1)));
        }
        out
    };
    let k2 = k * k;
    let mut c: Rows = vec![Vec::new(); n];
    let mut d: Rows = vec![Vec::new(); n];
    for j in 0..=ny {
        for i in 0..nx {
            let Some(r) = g.ex(i, j) else { continue };
            c[r].push((r, k2 * plane.eps_x(i, j)));
            // −(∇ × ẑH_z)_x = −∂_y H_z
            let w = 1.0 / (s.y_node[j] * dy);
            let (a, b) = (i as isize, j as isize);
            add(&mut c[r], -w, &hz(a, b));
            add(&mut c[r], w, &hz(a, b - 1));
            // ∂_x of the potential
            let w = 1.0 / (s.x_half[i] * dx);
            add(&mut d[r], w, &potential(i + 1, j));
            add(&mut d[r], -w, &potential(i, j));
        }
    }
    for j in 0..ny {
        for i in 0..=nx {
            let Some(r) = g.ey(i, j) else { continue };
            c[r].push((r, k2 * plane.eps_y(i, j)));
            // −(∇ × ẑH_z)_y = ∂_x H_z
            let w = 1.0 / (s.x_node[i] * dx);
            let (a, b) = (i as isize, j as isize);
            add(&mut c[r], w, &hz(a, b));
            add(&mut c[r], -w, &hz(a - 1, b));
            let w = 1.0 / (s.y_half[j] * dy);
            add(&mut d[r], w, &potential(i, j + 1));
            add(&mut d[r], -w, &potential(i, j));
        }
    }
    (merged(c), merged(d))
}

/// `rows` times `v`.
pub(crate) fn apply(rows: &Rows, v: &[c64]) -> Vec<c64> {
    rows.iter()
        .map(|row| row.iter().map(|&(c, a)| a * v[c]).sum())
        .collect()
}

/// A mode of a 3D section: its β and its transverse E and H on the plane's grid.
#[derive(Clone, Debug, PartialEq)]
pub struct Mode3d {
    grid: PlaneGrid,
    k: f64,
    beta: c64,
    /// E_x then E_y over the unknowns.
    e: Vec<c64>,
    /// H_y at E_x's places, then −H_x at E_y's: C E / (k₀β).
    h: Vec<c64>,
    /// Each unknown's weight in the product, V Δx Δy, shared by the section's modes.
    weights: Arc<Vec<c64>>,
}

impl Mode3d {
    /// The effective index β/k₀: a propagating mode's Re > 0, an evanescent one's Im > 0.
    pub fn effective_index(&self) -> c64 {
        self.beta / self.k
    }

    /// The propagation constant β, rad/µm.
    pub fn beta(&self) -> c64 {
        self.beta
    }

    /// The grid it was found on.
    pub fn grid(&self) -> PlaneGrid {
        self.grid
    }

    /// E_x at (x0 + (i + ½) dx, y0 + j dy); 0 on a wall.
    pub fn ex(&self, i: usize, j: usize) -> c64 {
        self.grid.ex(i, j).map_or(c64::new(0.0, 0.0), |r| self.e[r])
    }

    /// E_y at (x0 + i dx, y0 + (j + ½) dy); 0 on a wall.
    pub fn ey(&self, i: usize, j: usize) -> c64 {
        self.grid.ey(i, j).map_or(c64::new(0.0, 0.0), |r| self.e[r])
    }

    /// H_y at E_x's place (i, j), in units of E/Z₀ (H̃ = Z₀H).
    pub fn hy(&self, i: usize, j: usize) -> c64 {
        self.grid.ex(i, j).map_or(c64::new(0.0, 0.0), |r| self.h[r])
    }

    /// H_x at E_y's place (i, j).
    pub fn hx(&self, i: usize, j: usize) -> c64 {
        self.grid
            .ey(i, j)
            .map_or(c64::new(0.0, 0.0), |r| -self.h[r])
    }

    /// The share of |E_t|² in E_x: near 1 for a TE-like mode (E mostly along x), near 0 for a
    /// TM-like one.
    pub fn te_fraction(&self) -> f64 {
        let split = self.grid.ex_count();
        let x: f64 = self.e[..split].iter().map(|v| v.norm_sqr()).sum();
        let y: f64 = self.e[split..].iter().map(|v| v.norm_sqr()).sum();
        x / (x + y)
    }

    /// The unconjugated product ⟨E_self, H_other⟩ = Σ V E · (ẑ × H) Δx Δy, the discrete
    /// ∫ (E_self × H_other) · ẑ dx dy: 1 for a mode with itself, 0 (to round-off) with
    /// another mode of the same section. `None` unless the two share a grid and its PML.
    pub fn product(&self, other: &Mode3d) -> Option<c64> {
        (self.grid == other.grid && self.weights == other.weights).then(|| {
            self.weights
                .iter()
                .zip(&self.e)
                .zip(&other.h)
                .map(|((w, e), h)| w * e * h)
                .sum()
        })
    }

    /// Its E weighted for the product, over the unknowns.
    pub(crate) fn weighted_e(&self) -> impl Iterator<Item = c64> + '_ {
        self.weights.iter().zip(&self.e).map(|(w, e)| w * e)
    }

    /// Its H over the unknowns.
    pub(crate) fn h_values(&self) -> &[c64] {
        &self.h
    }

    /// The weights it carries.
    pub(crate) fn weights(&self) -> &Arc<Vec<c64>> {
        &self.weights
    }
}

/// Planes with at most this many unknowns are solved for every mode, densely.
pub(crate) const DENSE: usize = 1500;

/// Whether β propagates more than it decays.
fn propagates(beta: c64) -> bool {
    beta.im.abs() < beta.re.abs()
}

/// β from β²: a mode that propagates more than it decays goes forward (Re β > 0), the others
/// decay forward (Im β > 0).
fn forward(beta2: c64) -> c64 {
    let b = portable::complex::sqrt(beta2);
    if propagates(b) && b.re < 0.0 || !propagates(b) && b.im < 0.0 {
        -b
    } else {
        b
    }
}

/// Every eigenpair of the dense `a`, on this thread.
fn dense_pairs(a: &Mat<c64>) -> Result<Vec<(c64, Vec<c64>)>> {
    let n = a.nrows();
    let par = Par::Seq;
    let mut s = Diag::<c64>::zeros(n);
    let mut u = Mat::<c64>::zeros(n, n);
    let mut mem = MemBuffer::new(evd::evd_scratch::<c64>(
        n,
        ComputeEigenvectors::No,
        ComputeEigenvectors::Yes,
        par,
        Default::default(),
    ));
    evd::evd_cplx::<f64>(
        a.as_ref(),
        s.as_mut(),
        None,
        Some(u.as_mut()),
        par,
        MemStack::new(&mut mem),
        Default::default(),
    )
    .map_err(|e| Error::invalid("eme modes", format!("the eigensolver failed: {e:?}")))?;
    Ok((0..n)
        .map(|k| (s[k], (0..n).map(|i| u[(i, k)]).collect()))
        .collect())
}

/// Σ wᵢ aᵢ bᵢ.
fn form(w: &[c64], a: &[c64], b: &[c64]) -> c64 {
    w.iter().zip(a).zip(b).map(|((w, a), b)| w * a * b).sum()
}

/// The `count` modes of `plane` with the largest Re β² at wavenumber `k`, inside `stretch`,
/// highest first, each normalized to ⟨E, H⟩ = 1 (see the module's docs).
///
/// # Errors
///
/// [`Error::InvalidValue`] for more modes than unknowns, an eigensolver that fails, or a mode
/// whose product with itself is zero (it can't be normalized).
pub(crate) fn solve(plane: &Plane, stretch: &Stretch, k: f64, count: usize) -> Result<Vec<Mode3d>> {
    let g = plane.grid;
    let n = g.unknowns();
    if count > n {
        return Err(Error::invalid(
            "eme modes",
            format!("the plane holds {n} unknowns, fewer than the {count} modes asked"),
        ));
    }
    let (c, d) = operators(plane, stretch, k);
    let weights = Arc::new(stretch.weights(&g));
    let mut pairs: Vec<(c64, Vec<c64>)> = if n <= DENSE || 3 * count >= n {
        let mut a = Mat::<c64>::zeros(n, n);
        for (r, row) in c.iter().chain(&d).enumerate() {
            for &(col, v) in row {
                a[(r % n, col)] += v;
            }
        }
        dense_pairs(&a)?
    } else {
        let entries: Vec<(usize, usize, c64)> = c
            .iter()
            .chain(&d)
            .enumerate()
            .flat_map(|(r, row)| row.iter().map(move |&(col, v)| (r % n, col, v)))
            .collect();
        let shift = c64::new(k * k * plane.highest(), 0.0);
        let wanted = (count + 4).min(n);
        crate::eigen::nearest(n, &entries, shift, wanted, 1e-10)?
            .into_iter()
            .map(|p| (p.value, p.vector))
            .collect()
    };
    pairs.sort_by(|a, b| b.0.re.total_cmp(&a.0.re).then(b.0.im.total_cmp(&a.0.im)));
    // groups of the same β², each given the basis orthogonal in both forms
    let scale = |v: c64| portable::complex::abs(v).max(k * k);
    let mut start = 0;
    while start < pairs.len() {
        let mut end = start + 1;
        while end < pairs.len()
            && portable::complex::abs(pairs[end].0 - pairs[start].0) <= 1e-8 * scale(pairs[start].0)
        {
            end += 1;
        }
        if end - start > 1 {
            canonical(&mut pairs[start..end], &c, &weights)?;
        }
        start = end;
    }
    pairs.truncate(count);
    pairs
        .into_iter()
        .map(|(beta2, e)| normalized(g, k, beta2, e, &c, &weights))
        .collect()
}

/// A group of modes of one β²: the basis orthogonal in both B(u, v) = Σ V u · C v and
/// M(u, v) = Σ V u · v, the eigenvectors of the pencil (B, M) in the group, in order of their
/// eigenvalues' real parts, highest first; Gram–Schmidt in B where those meet too.
fn canonical(group: &mut [(c64, Vec<c64>)], c: &Rows, w: &[c64]) -> Result<()> {
    let m = group.len();
    let applied: Vec<Vec<c64>> = group.iter().map(|(_, e)| apply(c, e)).collect();
    let b = Mat::from_fn(m, m, |i, j| form(w, &group[i].1, &applied[j]));
    let mm = Mat::from_fn(m, m, |i, j| form(w, &group[i].1, &group[j].1));
    // M⁻¹ B, by the dense solver on this thread
    let pencil = {
        use faer::linalg::solvers::Solve;
        mm.partial_piv_lu().solve(&b)
    };
    let mut found = dense_pairs(&pencil)?;
    found.sort_by(|p, q| q.0.re.total_cmp(&p.0.re).then(q.0.im.total_cmp(&p.0.im)));
    let n = group[0].1.len();
    let mut vectors: Vec<Vec<c64>> = found
        .iter()
        .map(|(_, x)| {
            (0..n)
                .map(|r| (0..m).map(|i| x[i] * group[i].1[r]).sum())
                .collect()
        })
        .collect();
    // where the pencil's eigenvalues meet as well, Gram–Schmidt in B
    for j in 0..m {
        for i in 0..j {
            if portable::complex::abs(found[i].0 - found[j].0)
                > 1e-8 * portable::complex::abs(found[j].0).max(1e-300)
            {
                continue;
            }
            let ci = apply(c, &vectors[i]);
            let bii = form(w, &vectors[i], &ci);
            let bij = form(w, &vectors[j], &ci);
            if portable::complex::abs(bii) == 0.0 {
                return Err(Error::invalid("eme modes", "a mode of zero product"));
            }
            let f = bij / bii;
            let vi = vectors[i].clone();
            for (v, u) in vectors[j].iter_mut().zip(&vi) {
                *v -= f * u;
            }
        }
    }
    for (slot, v) in group.iter_mut().zip(vectors) {
        slot.1 = v;
    }
    Ok(())
}

/// Σ e w(x, y) with w = e^(0.3 u + 0.7 v), u and v the place's fractions of the window across
/// x and y, E_y's counted 1.7 times: a weight that no mirror of the window maps onto itself,
/// so a symmetric guide's odd modes have a sign that doesn't hang on rounding (the largest
/// value would: an odd mode has two, opposite), and the same field on another grid of the
/// window (another `ny` for a field uniform in y) the same sign.
fn sign_weight(grid: &PlaneGrid, e: &[c64]) -> c64 {
    let (lx, ly) = (grid.nx as f64 * grid.dx, grid.ny as f64 * grid.dy);
    let w = |u: f64, v: f64| portable::exp(0.3 * u / lx + 0.7 * v / ly);
    let mut sum = c64::new(0.0, 0.0);
    for j in 0..=grid.ny {
        for i in 0..grid.nx {
            if let Some(r) = grid.ex(i, j) {
                sum += e[r] * w((i as f64 + 0.5) * grid.dx, j as f64 * grid.dy);
            }
        }
    }
    for j in 0..grid.ny {
        for i in 0..=grid.nx {
            if let Some(r) = grid.ey(i, j) {
                sum += 1.7 * e[r] * w(i as f64 * grid.dx, (j as f64 + 0.5) * grid.dy);
            }
        }
    }
    sum
}

/// The mode of eigenvalue `beta2` and field `e`, normalized to ⟨E, H⟩ = 1, its sign by
/// [`sign_weight`].
fn normalized(
    grid: PlaneGrid,
    k: f64,
    beta2: c64,
    mut e: Vec<c64>,
    c: &Rows,
    weights: &Arc<Vec<c64>>,
) -> Result<Mode3d> {
    let beta = forward(beta2);
    let ce = apply(c, &e);
    let norm = form(weights, &e, &ce) / (k * beta);
    if portable::complex::abs(norm) == 0.0 || !norm.is_finite() {
        return Err(Error::invalid(
            "eme modes",
            format!("a mode (n_eff = {}) of zero product with itself", beta / k),
        ));
    }
    let mut scale = 1.0 / portable::complex::sqrt(norm);
    if (sign_weight(&grid, &e) * scale).re < 0.0 {
        scale = -scale;
    }
    e.iter_mut().for_each(|v| *v *= scale);
    let h = ce.iter().map(|v| v * scale / (k * beta)).collect();
    Ok(Mode3d {
        grid,
        k,
        beta,
        e,
        h,
        weights: Arc::clone(weights),
    })
}
